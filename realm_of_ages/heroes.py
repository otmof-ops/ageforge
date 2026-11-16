"""Heroes' guilds: a learned doctrine of deeds (Q-learning) and of skills (a bandit over the skill-tree
branches), shared by every hero of a people and outliving each of them. Also items, World Rites, and the
great hunts against the calamities."""
import math

import numpy as np

from .characters import gain_xp, make_character, tier_name
from .data import (ADULT, CLASSES, DIVINE, HERO_CAP, ITEM_A, ITEM_B, MONSTER, NATURE, NECRO, ARCANE, PEOPLES,
                   RARITY, RUNE, SCHOOLS, SPELLS, SPIRIT, WORLD_RITES, WORLDFORGED, EPITHET_HERO)

REST, TRAIN, HUNT, DELVE, WAR, GUARD, STUDY, SERVE, BOUNTY = range(9)
ACTIONS = ["rest", "train", "hunt", "delve", "war", "guard", "study", "serve", "bounty"]
LEVEL_BANDS = [10, 20, 35, 50, 70, 90]


class Guild:
    def __init__(self, people):
        self.people, self.q, self.decisions = people, {}, 0
        self.alpha, self.gamma = 0.15, 0.95
        self.branch_val = {}
        self.heroes_total = self.deaths_total = 0
        self.reset_window()

    @property
    def eps(self):
        return max(0.05, 0.35 * math.exp(-self.decisions / 4000))

    def reset_window(self):
        self.w = {"acts": np.zeros(len(ACTIONS), int), "hunts": 0, "hunt_wins": 0, "slain": 0, "deaths": 0,
                  "renown_at_death": [], "level_at_death": [], "reward": 0.0}

    def row(self, s):
        if s not in self.q:
            self.q[s] = np.zeros(len(ACTIONS))
        return self.q[s]

    def act(self, s, rng):
        self.decisions += 1
        if rng.random() < self.eps:
            return int(rng.integers(len(ACTIONS)))
        r = self.row(s)
        return int(rng.choice(np.flatnonzero(r == r.max())))

    def learn(self, s, a, r, s2, done):
        target = r if done else r + self.gamma * self.row(s2).max()
        self.row(s)[a] += self.alpha * (target - self.row(s)[a])

    def pick_branch(self, options, rng):
        if rng.random() < 0.2:
            return options[int(rng.integers(len(options)))]
        vals = [self.branch_val.get(o, (1.0, 0))[0] for o in options]   # untried branches look promising
        best = max(vals)
        idx = [i for i, v in enumerate(vals) if v == best]
        return options[idx[int(rng.integers(len(idx)))]]

    def credit(self, hero, reward):
        if not hero.branch:
            return
        key = max(hero.branch, key=hero.branch.get)
        mean, n = self.branch_val.get(key, (0.0, 0))
        n += 1
        self.branch_val[key] = (mean + (reward - mean) / n, n)

    def doctrine(self, top=3):
        ranked = sorted(((v[0], k) for k, v in self.branch_val.items() if v[1] >= 20), reverse=True)
        return [(f"{cls} / {CLASSES[cls]['branches'][b][0]}", v) for v, (cls, b) in ranked[:top]]


# ---------------------------------------------------------------- items
def make_item(realm, rarity, origin):
    rng = realm.rng
    if rarity >= 6:
        free = [w for w in WORLDFORGED if w[0] not in realm.worldforged_found]
        if not free:
            rarity = 5
        else:
            name, special, note = free[int(rng.integers(len(free)))]
            realm.worldforged_found.add(name)
            it = realm.Item(name, 6, realm.year, origin, special, note)
            return it
    for _ in range(60):
        name = f"the {rng.choice(ITEM_A)} of {rng.choice(ITEM_B)}"
        if name not in realm.item_names:
            break
    else:
        name = f"the {rng.choice(ITEM_A)} of {realm.name((['Ul', 'Ka', 'Mo', 'Ze', 'Ash', 'Vor'], ['rath', 'mon', 'dis', 'gal']))}"
    realm.item_names.add(name)
    return realm.Item(name, int(rarity), realm.year, origin)


def give_item(realm, c, it, quiet=False):
    c.items.append(it)
    it.bearers.append(c.id)
    c.items.sort(key=lambda i: -i.rarity)
    while len(c.items) > 4:
        spare = c.items.pop()
        k = realm.kingdoms[c.realm]
        k.treasury.append(spare)
    c.invalidate()
    if not quiet and it.rarity >= 4:
        realm.say(f"{realm.who(c)} claims {it.label()}" + (f": {it.note}" if it.note else "") + ".")


def roll_rarity(rng, depth):
    return int(min(5, max(0, math.floor(rng.exponential(0.9) + depth * 0.6))))


# ---------------------------------------------------------------- heroes
def spawn_heroes(realm):
    rng = realm.rng
    y = realm.year
    for f in range(realm.npol):
        if realm.extinct[f]:
            continue
        n = sum(1 for h in realm.heroes() if h.realm == f)
        cap = min(HERO_CAP + 16, 1 + len(realm.alive_towns(f)) // 25)    # a great realm raises more heroes
        if n >= cap:
            continue
        # heroic lineages first: grown children of heroes and nobles
        heirs = [realm.chars[c] for c in realm.living if realm.chars[c].realm == f]
        heirs = [c for c in heirs if not c.hero and not c.ruled and c.adult(y) and c.age(y) < ADULT[realm.cul(f)] + 25
                 and c.father is not None and (realm.chars[c.father].hero or realm.chars[c.father].renown > 100)]
        for c in heirs:
            if n >= cap:
                break
            if rng.random() < 0.15:
                start_hero(realm, c, f"takes up the calling of {realm.chars[c.father].full_name()}")
                n += 1
        for s in realm.alive_towns(f):
            if n >= cap:
                break
            if s.pop > 250 and rng.random() < 0.004 * s.pop / 100:
                c = make_character(realm, f, born=y - ADULT[realm.cul(f)] - int(rng.integers(0, 6)))
                c.home = s.id
                gain_xp(c, float(rng.uniform(150, 900)), realm)
                start_hero(realm, c, None)
                n += 1


def start_hero(realm, c, why):
    c.hero = True
    realm.hero_ids.add(c.id)
    if c.home is None or realm.by_id.get(c.home) is None:
        cap = realm.capital_of(c.realm)
        c.home = cap.id if cap else None
    realm.guilds[c.people].heroes_total += 1
    k = realm.kingdoms[c.realm]
    k.treasury.sort(key=lambda i: -i.rarity)
    taken = 0
    while k.treasury and k.treasury[0].rarity >= 2 and taken < 2:
        it = k.treasury.pop(0)
        prev = realm.chars[it.bearers[-1]].full_name() if it.bearers else None
        give_item(realm, c, it, quiet=True)
        taken += 1
        if it.rarity >= 4:
            realm.say(f"{realm.who(c)} takes up {it.label()}" + (f", once borne by {prev}" if prev else "") + ".")
    if why:
        realm.say(f"{realm.who(c)} {why}.", minor="calling" in why)


def hero_home(realm, h):
    s = realm.by_id.get(h.home)
    if s is None or not s.alive or s.faction != h.realm:
        towns = realm.alive_towns(h.realm)
        if not towns:
            return None
        s = max(towns, key=lambda t: t.pop)
        h.home = s.id
    return s


def nearest_monster(realm, pos, reach=25):
    best, bd = None, None
    lid = int(realm.landmass[pos])
    for m in realm.monsters:
        if not m.alive or getattr(m, "sea", False) or (m.landmass != lid and not getattr(m, "world", False)):
            continue
        lim = 45 if m.kind == "calamity" else reach
        d = math.hypot(m.pos[0] - pos[0], m.pos[1] - pos[1])
        if d <= lim and (bd is None or (m.kind == "calamity") > (best.kind == "calamity") or
                         ((m.kind == "calamity") == (best.kind == "calamity") and d < bd)):
            best, bd = m, d
    return best


def ruins_on(realm, lid):
    """The ruins of one landmass, indexed once a year (new ruins join the index the next year)."""
    idx = getattr(realm, "_ruin_idx", None)
    if idx is None or idx[0] != realm.year:
        by = {}
        for r in realm.ruins:
            rl = r.__dict__.get("lid")
            if rl is None:
                rl = r.lid = int(realm.landmass[r.pos])
            by.setdefault(rl, []).append(r)
        idx = (realm.year, by)
        realm._ruin_idx = idx
    return idx[1].get(lid, ())


def nearest_ruin(realm, pos, reach=20):
    best, bd = None, reach + 1
    lid = int(realm.landmass[pos])
    for r in ruins_on(realm, lid):
        if r.loot or getattr(r, "conquered", None) is None:
            d = math.hypot(r.pos[0] - pos[0], r.pos[1] - pos[1])
            if d < bd:
                best, bd = r, d
    return best


def hero_obs(realm, h, home):
    lb = sum(h.level >= t for t in LEVEL_BANDS)
    hb = min(2, int(h.hp // 34))
    m = nearest_monster(realm, home.pos)
    if m is None:
        threat = 0
    else:
        ratio = m.power * 0.6 / max(1.0, h.cr())
        threat = 1 if ratio < 0.6 else 2 if ratio < 1.6 else 3
    war = int(realm.war[h.realm].any())
    ruin = int(nearest_ruin(realm, home.pos) is not None)
    cal = int(any(m.alive and m.kind == "calamity" for m in realm.monsters))
    return (lb, hb, threat, war, ruin, cal)


def renown(realm, h, amount):
    h.renown += amount
    from .data import DEED_LEGACY
    from .lineage import gain_legacy
    gain_legacy(realm, h, DEED_LEGACY.get(h.last_deed), amount)
    if h.house is not None:
        realm.houses[h.house].prestige += amount * 0.5
    if h.epithet is None and h.renown >= 80 and not h.ruled:
        weight = {"calamity": 1000, "dragon": 150, "rite": 50, "demon": 60, "wraith": 30, "troll": 10,
                  "study": 12, "guard": 5, "delve": 3, "war": 1.5}
        best = max(h.deeds, key=lambda d: h.deeds[d] * weight.get(d, 1)) if h.deeds else "guard"
        ep = EPITHET_HERO.get(best, "the Bold")
        if best == "guard":
            home = realm.by_id.get(h.home)
            ep = f"the Shield of {home.name}" if home else "the Shield"
        h.epithet = ep
        realm.say(f"{h.name} of the {PEOPLES[h.people]} is called {h.full_name()} now.")


def hurt(realm, h, dmg, cause, killer=None, by=None):
    h.hp -= dmg / max(0.3, h.hp_mult())
    if h.hp <= 0 and h.latent and realm.rng.random() < 0.2:
        from .lineage import awaken
        awaken(realm, h, "at the edge of death")
        h.hp = 20.0
    if h.hp <= 0:
        realm.kill(h, cause, killer=killer, by=by)


def deed(h, k):
    h.deeds[k] = h.deeds.get(k, 0) + 1
    h.last_deed = k


def heroes_act(realm):
    rng = realm.rng
    war_power = [0.0] * realm.npol
    hunters = []
    realm.acted = []
    for s in realm.alive_towns():
        s.guard, s.raided, s.raided_by = 0.0, None, None
    for h in list(realm.heroes()):
        home = hero_home(realm, h)
        if home is None:
            realm.kill(h, f"faded from the world with the fall of {realm.kingdoms[h.realm].name}")
            continue
        h.hp = min(100.0, h.hp + 15)
        try_rite(realm, h, home)
        g = realm.guilds[h.people]
        s = hero_obs(realm, h, home)
        a = g.act(s, rng)
        g.w["acts"][a] += 1
        h.pending = [s, a, h.renown]
        realm.acted.append(h)
        L = h.level
        from . import skills as sk
        if a == REST:
            h.hp = min(100.0, h.hp + 40 + min(30.0, realm.kingdoms[h.realm].potions))
            realm.kingdoms[h.realm].potions = max(0.0, realm.kingdoms[h.realm].potions - 1)
        elif a == BOUNTY:
            from .society import hunt_bounty
            got = hunt_bounty(realm, h)
            if got:
                deed(h, "guard")
                renown(realm, h, got)
        elif a == TRAIN:
            gain_xp(h, 80 + 5 * L, realm)
            sk.train_class(realm, h, 1.0)
            forge(realm, h)
        elif a == STUDY:
            gain_xp(h, 60 + 4 * L, realm)
            sk.fight_skills(realm, h, 0.3)
            sk.use(realm, h, "Enchanting", 0.5)
            sk.use(realm, h, "Alchemy", 0.5)
            study(realm, h)
            forge(realm, h)
        elif a == SERVE:
            k = realm.kingdoms[h.realm]
            on_council = h.id in k.council.values()
            gain_xp(h, 30, realm)
            sk.use(realm, h, "Speech", 1.2)
            h.gold += 20 + (40 if on_council else 0)
            renown(realm, h, 5 if on_council else 2)
            h.serving = realm.year
        elif a == HUNT:
            m = nearest_monster(realm, home.pos)
            if m is None:
                continue
            if m.kind == "calamity":
                hunters.append((h, m))
                continue
            hunt(realm, h, m, home, g)
        elif a == DELVE:
            delve(realm, h, home)
        elif a == WAR:
            if realm.war[h.realm].any():
                war_power[h.realm] += h.cr() * h.mods()["war"]
                sk.fight_skills(realm, h, 1.0)
                h.gold += 15
        elif a == GUARD:
            home.guard += h.cr() * h.mods()["guard"]
            sk.use(realm, h, "Block", 0.8)
            gain_xp(h, 20, realm)
    great_hunt(realm, hunters)
    return war_power


def hunt(realm, h, m, home, g):
    rng = realm.rng
    g.w["hunts"] += 1
    from .skills import fight_skills
    fight_skills(realm, h, 1.2)
    cr = h.cr() * h.mods()["vs_monster"]
    p = cr / (cr + 0.6 * m.power)
    if cr < 0.12 * m.power:
        p *= 0.25                                                   # the gap between tiers is not crossed by luck
    if rng.random() < p:
        g.w["hunt_wins"] += 1
        m.power -= cr * 1.5
        gain_xp(h, m.base * 0.8, realm)
        hurt(realm, h, rng.uniform(5, 30), f"died of wounds from {m.name}", killer=m)
    else:
        m.power -= cr * 0.4
        gain_xp(h, m.base * 0.2, realm)
        hurt(realm, h, min(150, rng.uniform(30, 90) * 2 * m.power / (m.power + cr)), f"was slain by {m.name}", killer=m)
    if m.power <= 0 and m.alive:
        m.alive = False
        g.w["slain"] += 1
        realm.win["slain"] += 1
        deed(h, m.kind)
        from .monsters import on_monster_slain
        on_monster_slain(realm, m, h if h.alive else None)
        home.wealth += m.hoard * 0.7
        h.gold += m.hoard * 0.3 + m.base * 0.1
        renown(realm, h, MONSTER[m.kind][6])
        drop = None
        if m.kind == "dragon":
            drop = make_item(realm, int(rng.integers(3, 6)), f"the hoard of {m.name}")
        elif m.kind == "demon" and rng.random() < 0.6:
            drop = make_item(realm, int(rng.integers(2, 5)), f"the ashes of {m.name}")
        if m.kind != "troll" or rng.random() < 0.1:
            realm.say(f"{realm.who(h)} of {home.name} slays {m.name}"
                      f"{f' and carries home {m.hoard:,.0f} gold' if m.hoard > 50 else ''}.")
        if drop and h.alive:
            give_item(realm, h, drop)


def delve(realm, h, home):
    r = nearest_ruin(realm, home.pos)
    if r is None:
        return
    from .dungeons import delve as run
    run(realm, h, home, r)


def study(realm, h):
    """Reaching past the talent one was born with is rare, and it is how the high tiers are opened."""
    rng = realm.rng
    if h.school() is None:
        return
    eff = sum(lv * min(1.0, h.class_def(cls)["magic"]) for cls, lv in h.classes)
    if eff / 9 > int(h.talent) and h.talent < 10:
        p = 0.03 * (1 + h.attr("lrn") / 20)
        if rng.random() < p:
            before = h.tier()
            h.talent = min(10.0, int(h.talent) + 1.0)
            h.invalidate()
            deed(h, "study")
            after = h.tier()
            if after > before:
                realm.on_new_tier(h, after)
                if after >= 5:
                    renown(realm, h, 10 * after)
                    realm.say(f"{realm.who(h)} breaks through to the {tier_name(after)} after long study.")


def forge(realm, h):
    """Runesmiths deep in the Forgemaster tree make their own gear, and sometimes a relic."""
    rng = realm.rng
    n = h.branch.get(("Runesmith", 0), 0)
    lord = any(c == "Rune-Lord" for c, _ in h.classes)
    if n == 0 and not lord:
        return
    if rng.random() < 0.04 * n + (0.1 if lord else 0):
        rar = int(min(5, rng.integers(1, 3 + n // 2 + (1 if lord else 0))))
        it = make_item(realm, rar, f"the forge of {h.name}")
        if rar >= 4:
            realm.say(f"{realm.who(h)} forges {it.label()}.")
        give_item(realm, h, it, quiet=True)


def great_hunt(realm, hunters):
    """Heroes who ride out against a calamity fight it together."""
    rng = realm.rng
    by_m = {}
    for h, m in hunters:
        if h.alive and m.alive:
            by_m.setdefault(id(m), (m, []))[1].append(h)
    for m, party in by_m.values():
        total = sum(h.cr() * h.mods()["vs_monster"] for h in party)
        p = total / (total + 0.6 * m.power)
        if total < 0.12 * m.power:
            p *= 0.25
        for h in party:
            realm.guilds[h.people].w["hunts"] += 1
        names = ", ".join(realm.who(h) for h in party[:6]) + (f" and {len(party) - 6} more" if len(party) > 6 else "")
        if rng.random() < p:
            m.power -= total * 1.2
            for h in party:
                realm.guilds[h.people].w["hunt_wins"] += 1
        else:
            m.power -= total * 0.3
        for h in party:
            gain_xp(h, m.base * 0.05, realm)
            hurt(realm, h, rng.uniform(20, 80) * 2 * max(0.0, m.power) / (max(0.0, m.power) + total),
                 f"fell in the great hunt against {m.name}", killer=m)
        if m.power <= 0 and m.alive:
            m.alive = False
            realm.win["slain"] += 1
            living = [h for h in party if h.alive]
            for h in living:
                deed(h, "calamity")
                renown(realm, h, 300)
            realm.say(f"**{m.name} is slain** after {realm.year - m.born} years, by {names}. "
                      f"The peoples will sing of it for an age.")
            if living:
                top = max(living, key=lambda h: h.cr())
                m.slain_by = top.id
                renown(realm, top, 700)
                it = make_item(realm, 6, f"the corpse of {m.name}")
                give_item(realm, top, it)
            realm.calamity_log.append((m.name, m.born, realm.year, "slain by heroes: " + names))
        elif len(party) >= 2:
            realm.say(f"A war-band of heroes ({names}) rides against {m.name}; it still stands.", minor=True)


def can_rite(h):
    t = h.tier()
    return t >= 9 or (t >= 8 and any(it.special == "rite" for it in h.items))


def try_rite(realm, h, home):
    rng = realm.rng
    y = realm.year
    if y < h.rite_ready or not can_rite(h) or rng.random() > 0.25:
        return
    s = h.school()
    k = realm.kingdoms[h.realm]
    f = h.realm
    cal = [m for m in realm.monsters if m.alive and m.kind == "calamity"]
    rite = WORLD_RITES[s]
    at_war = realm.war[f].any()
    text = None
    if cal:
        m = cal[0]
        dmg = h.cr() * 4
        m.power -= dmg
        text = f"turns {rite} upon {m.name}, burning away {dmg:,.0f} of its strength"
        if m.power <= 0:
            m.alive = False
            deed(h, "calamity")
            renown(realm, h, 1000)
            realm.calamity_log.append((m.name, m.born, y, f"unmade by {realm.who(h)} with {rite}"))
            m.slain_by = h.id
            text += f", and **{m.name} is unmade**"
            give_item(realm, h, make_item(realm, 6, f"the corpse of {m.name}"))
    elif s == ARCANE and at_war:
        foes = [t for t in realm.alive_towns() if realm.war[f, t.faction]]
        if foes:
            tgt = max(foes, key=lambda t: t.pop)
            for t in realm.alive_towns():
                if t is not tgt and math.hypot(t.pos[0] - tgt.pos[0], t.pos[1] - tgt.pos[1]) <= 5 and t.faction != f:
                    t.pop *= 0.5
            text = f"speaks {rite}; fire falls from a clear sky and {tgt.name} of the {realm.pname(tgt.faction)} is gone"
            realm.destroy(tgt)
            for o in range(realm.npol):
                if o != f:
                    realm.relation[f, o] -= 25
                    realm.relation[o, f] -= 25
    elif s == NECRO and at_war:
        k.mods["war_power"] = (2.0, y + 10)
        from .governance import add_tyranny
        add_tyranny(realm, realm.chars.get(k.ruler), 20, "the Long Night")
        h.tyranny += 20
        for o in range(realm.npol):
            if o != f:
                realm.relation[f, o] -= 40
                realm.relation[o, f] -= 40
        text = f"speaks {rite}; the dead of every battlefield rise to march for the {realm.pname(f)}"
    elif s == SPIRIT and at_war:
        k.mods["martial"] = (1.4, y + 10)
        text = f"speaks {rite}; the moon goes red and the {realm.pname(f)} fight like their fathers' ghosts"
    elif s == RUNE and any(t.walls < 5 for t in realm.alive_towns(f)):
        for t in realm.alive_towns(f):
            t.walls = min(5, t.walls + 2)
        text = f"speaks {rite}; overnight every town of the {realm.pname(f)} stands behind new stone"
    elif s == DIVINE and realm.year - realm.last_plague < 10:
        for t in realm.alive_towns(f):
            t.pop *= 1.15
        for o in realm.heroes():
            if o.realm == f:
                o.hp = 100.0
        text = f"speaks {rite}; the sick rise from their beds across the land of the {realm.pname(f)}"
    elif s == NATURE and realm.blight > 0:
        k.mods["food"] = (1.4, y + 25)
        text = f"speaks {rite}; the blight lifts and the {realm.pname(f)}'s fields bear twice over for a generation"
    if text:
        h.rite_ready = y + 60
        deed(h, "rite")
        renown(realm, h, 50)
        realm.rites.append((y, h.id, rite))
        realm.say(f"**World Rite.** {realm.who(h)}, a {tier_name(h.tier())} caster of the {SCHOOLS[s]}, {text}.")


def heroes_resolve(realm, won):
    rng = realm.rng
    for h in realm.acted:
        if h.pending is None:
            continue
        s, a, r0 = h.pending
        if h.alive and a == WAR and realm.war[h.realm].any():
            foes = [e for e in range(realm.npol) if realm.war[h.realm, e]]
            enemies = [realm.pname(e) for e in foes]
            blame = realm.chars.get(realm.kingdoms[foes[0]].ruler)
            if rng.random() < 0.25 / (1 + 0.05 * h.level):
                hurt(realm, h, rng.uniform(40, 110), f"fell in the war against the {enemies[0]}", by=blame)
            else:
                hurt(realm, h, rng.uniform(0, 30), f"died of wounds taken fighting the {enemies[0]}", by=blame)
            if h.alive:
                deed(h, "war")
                gain_xp(h, 150 + 4 * h.level, realm)
                renown(realm, h, min(10, 2 * won[h.realm]))
        if h.alive and a == GUARD:
            home = realm.by_id.get(h.home)
            if home is not None and home.raided == "repelled":
                deed(h, "guard")
                gain_xp(h, 100, realm)
                renown(realm, h, 5)
            elif home is not None and home.raided == "lost":
                hurt(realm, h, rng.uniform(20, 60), f"fell defending {home.name} from {home.raided_by.name}", killer=home.raided_by)
        g = realm.guilds[h.people]
        done = not h.alive
        combat_death = done and h.cause not in ("died of old age", "died of illness")
        reward = (h.renown - r0) / 10 + (-5.0 if combat_death else 0.05)
        g.w["reward"] += reward
        g.credit(h, reward)
        s2 = None
        if not done:
            home = hero_home(realm, h)
            if home is None:
                done = True
            else:
                s2 = hero_obs(realm, h, home)
        g.learn(s, a, reward, s2, done)
        h.pending = None
        if h.alive:
            h.peak_cr = max(h.peak_cr, h.cr(False))
