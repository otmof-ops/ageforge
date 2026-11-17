"""The shinobi clans: hidden villages in the wild places, outside every kingdom.

A shinobi trains ki (inner energy), one or two of five natures (Flame, Gale, Thunder, Stone, Tide) and six
arts (Body, Illusion, Seal, Summoning, Healing, Forbidden), each a ladder of five techniques. They rise through
five ranks, take contracts from the kingdoms (spying, guarding, killing, monster-slaying) for gold, feud with
the rival clans, and pass down two eye bloodlines: the Hollow Eye, which sees ki itself, and the Deepwell,
whose reserves are bottomless. Forbidden arts are strong, and cost years of life."""
import math

import numpy as np

from .characters import make_character
from .data import ADULT, PEOPLES, ROMAN
from .data2 import ARTS, NATURES, SHINOBI_CLANS, SHINOBI_EYES, SHINOBI_HEAD, SHINOBI_RANKS, SUMMON_PACTS
from .skills import use

CLAN_CAP = 7


class Clan:
    def __init__(self, cid, name, pos, year):
        self.id, self.name, self.pos, self.founded = cid, name, pos, year
        self.members, self.head, self.heads, self.gold = set(), None, [], 0.0
        self.feud = None
        self.missions = {"won": 0, "lost": 0}
        self.pact = None
        self.kill_year = -1


def found_clans(realm):
    rng = realm.rng
    from .data import FOREST, MOUNTAIN, HILLS
    land = np.argwhere(np.isin(realm.terrain, (FOREST, MOUNTAIN, HILLS)) & (realm.owner == -1))
    for n, name in enumerate(SHINOBI_CLANS):
        r, c = land[int(rng.integers(len(land)))]
        cl = Clan(n, name, (int(r), int(c)), 0)
        cl.pact = SUMMON_PACTS[n % len(SUMMON_PACTS)]
        realm.clans.append(cl)
        for _ in range(4):
            recruit(realm, cl, founding=True)
    realm.say("Three hidden clans keep their villages in the wild places: " + ", ".join(c.name for c in realm.clans)
              + ". No king rules them; every king hires them.")


def recruit(realm, cl, founding=False):
    rng = realm.rng
    y = realm.year
    parents = [realm.chars[m] for m in cl.members if realm.chars[m].alive and realm.chars[m].adult(y)]
    f = int(rng.choice(4, p=[0.45, 0.2, 0.15, 0.2]))
    c = make_character(realm, f, born=y - ADULT[f] - int(rng.integers(0, 10)))
    return initiate(realm, cl, c, founding, parents)


def initiate(realm, cl, c, founding=False, parents=()):
    rng = realm.rng
    c.clan = cl.id
    cl.members.add(c.id)
    c.ki = float(rng.uniform(40, 80))
    c.natures = [str(rng.choice(list(NATURES)))]
    if rng.random() < 0.25:
        c.natures.append(str(rng.choice([n for n in NATURES if n not in c.natures])))
    c.arts = {a: 0 for a in ARTS if a != "Forbidden"}
    c.nature_lv = {n: 1 for n in c.natures}
    c.srank = 0
    c.missions = 0
    for eye in SHINOBI_EYES:
        carriers = [p for p in parents if eye in p.genetic]
        if rng.random() < (0.3 if carriers else 0.0) + 0.01:
            c.genetic.add(eye)
            if eye == "deepwell":
                c.ki *= 3
    c.skills["Sneak"] = min(100.0, c.skills["Sneak"] + 25)
    c.skills["Blade"] = min(100.0, c.skills["Blade"] + 15)
    if not founding and (set(c.genetic) & set(SHINOBI_EYES)):
        eye = sorted(set(c.genetic) & set(SHINOBI_EYES))[0]
        realm.say(f"A child of {cl.name} opens the {eye.replace('-', ' ').title()}: {c.name}. It "
                  f"{SHINOBI_EYES[eye]}.", minor=True)
    return c


def technique(c):
    """Their strongest technique, and its tier."""
    best = (0, None)
    for n, lv in c.nature_lv.items():
        if lv > best[0]:
            best = (lv, NATURES[n][lv - 1])
    for a, lv in c.arts.items():
        if lv > best[0]:
            best = (lv, ARTS[a][lv - 1])
    return best


def shinobi_power(c):
    lv = sum(c.nature_lv.values()) + sum(c.arts.values())
    top = technique(c)[0]
    p = (c.ki ** 0.6) * (1 + lv * 0.25) * (1.8 ** top) * (1 + c.level / 60)
    if "hollow-eye" in c.genetic:
        p *= 1.4
    return p


def shinobi_year(realm):
    rng = realm.rng
    y = realm.year
    for cl in realm.clans:
        cl.members = {m for m in cl.members if realm.chars[m].alive}
        for i in list(realm.living):                       # children of the clan come home to it
            c = realm.chars[i]
            if c.clan is None and c.age(y) >= 8 and not c.hero and not c.ruled and not c.profession and any(
                    p is not None and realm.chars[p].clan == cl.id for p in (c.father, c.mother)):
                if len(cl.members) < CLAN_CAP:
                    initiate(realm, cl, c)
        for m in list(cl.members):
            c = realm.chars[m]
            if c.spouse is None and c.adult(y) and rng.random() < 0.05:
                from .dynasty import marry
                mate = make_character(realm, c.realm, born=c.born + int(rng.integers(-5, 6)), female=not c.female)
                marry(c, mate)
        if len(cl.members) < CLAN_CAP and rng.random() < 0.3:
            recruit(realm, cl)
        for m in list(cl.members):
            c = realm.chars[m]
            train(realm, c, cl)
            mission(realm, c, cl)
        if cl.members:
            head = max((realm.chars[m] for m in cl.members), key=shinobi_power)
            if cl.head != head.id:
                cl.head = head.id
                cl.heads.append((y, head.id))
                head.titles.append(f"{SHINOBI_HEAD} of {cl.name} from year {y}")
                realm.say(f"{realm.who(head)} takes the veil as {SHINOBI_HEAD} of {cl.name}, master of "
                          f"{technique(head)[1]}.", minor=True)
    contracts_year(realm)
    clan_wars(realm)
    feuds(realm)


def train(realm, c, cl):
    rng = realm.rng
    speed = 1.5 if "hollow-eye" in c.genetic else 1.0
    c.ki = min(1000.0, c.ki * 1.01 + 1.5 * speed)
    from .characters import gain_xp
    gain_xp(c, 60 + 3 * c.level, realm)
    use(realm, c, "Sneak", 1.0)
    use(realm, c, "Blade", 0.8)
    if rng.random() < 0.12 * speed:
        n = c.natures[int(rng.integers(len(c.natures)))]
        if c.nature_lv[n] < 5 and c.ki >= 40 * c.nature_lv[n] ** 1.6 and (c.nature_lv[n] < 4 or may_trump(realm, c, cl)):
            c.nature_lv[n] += 1
            if c.nature_lv[n] == 5:
                announce_trump(realm, c, cl, NATURES[n][4])
            if c.nature_lv[n] == 4:
                realm.say(f"{realm.who(c)} of {cl.name} masters {NATURES[n][3]}, a tier-IV {n} technique.",
                          minor=True)
    if rng.random() < 0.1 * speed:
        a = str(rng.choice(list(c.arts)))
        if c.arts[a] < 5 and c.ki >= 30 * (c.arts[a] + 1) ** 1.5 and (c.arts[a] < 4 or may_trump(realm, c, cl)):
            c.arts[a] += 1
            if c.arts[a] == 5:
                announce_trump(realm, c, cl, ARTS[a][4])
            if a == "Summoning" and c.arts[a] >= 3:
                realm.say(f"{realm.who(c)} signs the pact of {cl.pact} in blood.", minor=True)
    if "Forbidden" not in c.arts and "ambitious" in c.personality and c.ki > 300 and rng.random() < 0.02:
        c.arts["Forbidden"] = 1
        c.life -= 10
        realm.say(f"{realm.who(c)} of {cl.name} learns {ARTS['Forbidden'][0]}, a forbidden art. It costs years of "
                  f"life to use.")
    elif "Forbidden" in c.arts and c.arts["Forbidden"] < 5 and rng.random() < 0.05:
        c.arts["Forbidden"] += 1
        c.life -= 8
    from .characters import power_gate
    rank = min(int(shinobi_power(c) ** 0.5 / 9), {3: 2, 4: 3}.get(power_gate(c, martial=True), 4))
    if rank > c.srank:
        c.srank = rank
        if rank >= 3:
            realm.say(f"{realm.who(c)} is named a {SHINOBI_RANKS[rank]} of {cl.name}.", minor=rank < 4)
    c.peak_cr = max(c.peak_cr, shinobi_power(c))


def mission(realm, c, cl):
    rng = realm.rng
    if not c.adult(realm.year) or rng.random() > 0.5:
        return
    kind = rng.choice(["spy", "guard", "kill", "beast"], p=[0.45, 0.37, 0.06, 0.12])
    if kind == "kill":
        if cl.kill_year == realm.year:
            kind = "spy"
        cl.kill_year = realm.year
    pw = shinobi_power(c)
    c.missions += 1
    from .crafts import gain_fame
    if kind in ("spy", "guard"):
        pay = 30 + 10 * c.srank
        if rng.random() < 0.9:
            c.gold += pay
            cl.gold += pay * 0.3
            cl.missions["won"] += 1
            gain_fame(realm, c, "shinobi", 2 + c.srank)
        else:
            cl.missions["lost"] += 1
            if rng.random() < 0.2:
                realm.kill(c, "died on a mission no one will speak of")
    elif kind == "beast":
        ms = [m for m in realm.monsters if m.alive and m.kind != "calamity" and not getattr(m, "sea", False)
              and math.hypot(m.pos[0] - cl.pos[0], m.pos[1] - cl.pos[1]) < 30]
        if not ms:
            return
        m = min(ms, key=lambda m: math.hypot(m.pos[0] - cl.pos[0], m.pos[1] - cl.pos[1]))
        if rng.random() < pw / (pw + 0.6 * m.power):
            m.power -= pw * 1.5
            if m.power <= 0 and m.alive:
                m.alive = False
                from .monsters import on_monster_slain
                on_monster_slain(realm, m, c)
                c.gold += 200
                gain_fame(realm, c, "shinobi", 20 + m.base / 50)
                if m.kind in ("dragon", "demon"):
                    realm.say(f"{realm.who(c)} of {cl.name} kills {m.name} with {technique(c)[1]}.")
        else:
            c.hp -= rng.uniform(20, 80)
            if c.hp <= 0:
                realm.kill(c, f"was slain by {m.name}", killer=m)
            else:
                c.hp = max(c.hp, 30)
    else:  # an assassination contract from a king at war
        wars = [(i, j) for i in range(realm.npol) for j in range(realm.npol) if i != j and realm.war[i, j]]
        if not wars:
            return
        client, tp = wars[int(rng.integers(len(wars)))]
        tk = realm.kingdoms[tp]
        tgts = [h for h in realm.heroes() if h.realm == tp]
        if tk.ruler is not None and rng.random() < 0.3:
            tgts = [realm.chars[tk.ruler]]
        if not tgts:
            return
        t = tgts[int(rng.integers(len(tgts)))]
        from .society import court_guard
        defence = t.cr() * (2 if t.undead else 1) + 10 * court_guard(realm, tk, t)
        if rng.random() < pw / (pw + 3 * defence + 500):
            c.gold += 400
            gain_fame(realm, c, "shinobi", 40)
            realm.say(f"{realm.who(t)} dies in the night with no wound but a needle-mark: {cl.name} has done "
                      f"{realm.pname(client)} work.", minor=tp != t.realm or t.id != tk.ruler)
            realm.kill(t, f"was killed by a shinobi of {cl.name}", quiet=True, by=realm.chars.get(realm.kingdoms[client].ruler))
            tk.security = getattr(tk, "security", 0.0) + 40                 # the court hires guards and tasters
        else:
            dmg = rng.uniform(30, 100)
            c.hp -= dmg
            if c.hp <= 0:
                realm.kill(c, f"was cut down trying to kill {t.full_name()}")
            else:
                c.hp = max(30.0, c.hp)


def feuds(realm):
    rng = realm.rng
    if len(realm.clans) < 2 or rng.random() > 0.03:
        return
    a, b = rng.choice(len(realm.clans), 2, replace=False)
    A, B = realm.clans[int(a)], realm.clans[int(b)]
    if not A.members or not B.members:
        return
    ca = target_in(realm, A)
    cb = target_in(realm, B)
    pa, pb = shinobi_power(ca), shinobi_power(cb)
    win, lose = (ca, cb) if rng.random() < pa / (pa + pb) else (cb, ca)
    wc = realm.clans[win.clan]
    realm.say(f"**Clan feud.** {realm.who(win)} of {wc.name} meets {realm.who(lose)} in the mountain passes; "
              f"{technique(win)[1]} answers {technique(lose)[1]}, and {lose.name} does not come home.")
    from .crafts import gain_fame
    gain_fame(realm, win, "shinobi", 60)
    realm.kill(lose, f"fell in a clan feud to {win.full_name()}")
    wc.standing = getattr(wc, "standing", 10.0) + 15
    lc = realm.clans[lose.clan] if lose.clan is not None else None
    if lc is not None:
        lc.standing = getattr(lc, "standing", 10.0) * 0.85
    steal_scroll(realm, win, lose, wc)


# ---------------------------------------------------------------- trump cards, contracts, clan wars
TRUMP_CAP = 2


def trumps(realm, cl):
    return [realm.chars[m] for m in cl.members if realm.chars[m].alive and
            (5 in realm.chars[m].nature_lv.values() or 5 in realm.chars[m].arts.values())]


def may_trump(realm, c, cl):
    """A tier-V technique is a clan's trump card: only a Master or above learns one, and a clan keeps two at most."""
    if 5 in c.nature_lv.values() or 5 in c.arts.values():
        return False                                                     # one trump card to a shinobi
    return (c.srank >= 3 or (c.srank >= 2 and cl.head == c.id)) and len(trumps(realm, cl)) < TRUMP_CAP


def announce_trump(realm, c, cl, name):
    realm.say(f"**A trump card.** {realm.who(c)} of {cl.name} masters {name}, a tier-V technique the clan will "
              f"guard with their lives.")


def target_in(realm, cl):
    """Who a rival clan goes after: a trump-card holder if they have one, otherwise the strongest."""
    t = trumps(realm, cl)
    pool = t or [realm.chars[m] for m in cl.members]
    return max(pool, key=shinobi_power)


def steal_scroll(realm, win, lose, wc):
    """From the body of a fallen trump card, the scroll of their technique, if the victor may hold one."""
    best = [(n, 5) for n, lv in lose.nature_lv.items() if lv == 5] + [(a, 5) for a, lv in lose.arts.items() if lv == 5]
    if not best or not may_trump(realm, win, wc) or realm.rng.random() > 0.35:
        return
    n, _ = best[0]
    if n in NATURES and n in win.nature_lv:
        win.nature_lv[n] = 5
        tech = NATURES[n][4]
    elif n in ARTS:
        win.arts[n] = 5
        tech = ARTS[n][4]
    else:
        return
    realm.say(f"{realm.who(win)} takes the scroll of {tech} from the body of {lose.name}; it belongs to {wc.name} now.")


def region_of(realm, pos):
    return int(realm.landmass[pos])


def contracts_year(realm):
    """Crowns and towns post shinobi work; the clans near enough bid for it; the winner does it, the losers resent it."""
    from .events import book
    rng = realm.rng
    clans = [cl for cl in realm.clans if cl.members]
    if not clans:
        return
    for cl in clans:
        cl.standing = getattr(cl, "standing", 10.0) * 0.98 + 3 * len(trumps(realm, cl))
        cl.holds = getattr(cl, "holds", {})
    jobs = []
    for k in realm.kingdoms:
        if realm.extinct[k.id] or k.ruler is None:
            continue
        seat = realm.capital_of(k.id)
        if seat is None:
            continue
        revenue = max(1.0, getattr(k, "income", 0.0) * getattr(k, "tax_now", 0.3))
        foes = [o for o in range(realm.npol) if realm.war[k.id, o]]
        if foes and rng.random() < 0.4:
            jobs.append(("spy", k, seat, 0.3 * revenue, int(rng.choice(foes))))
        r = realm.chars.get(k.ruler)
        if (getattr(k, "security", 0) > 30 or (r is not None and "paranoid" in r.personality)) and rng.random() < 0.3:
            jobs.append(("guard", k, seat, 0.2 * revenue, None))
    g = getattr(realm, "guild", None)
    if g is not None:
        for q in list(g["quests"].values()):
            t = realm.by_id.get(q["target"][1]) if q["kind"] == "escort" else None
            if t is not None and q["taken"] != realm.year and t.wealth > 8 * q["reward"] and rng.random() < 0.4:
                q["taken"] = realm.year
                jobs.append(("escort", realm.kingdoms[t.faction], t, 2 * q["reward"], q["id"]))
    for kind, k, place, fee, extra in jobs:
        if k.gold < fee and kind != "escort":
            continue
        near = [cl for cl in clans if math.hypot(cl.pos[0] - place.pos[0], cl.pos[1] - place.pos[1]) < 80]
        if not near:
            continue
        reg = region_of(realm, place.pos)
        bids = sorted(near, key=lambda cl: -(getattr(cl, "standing", 10.0) + 5 * cl.holds.get(reg, 0)) ** 0.5
                      * shinobi_power(realm.chars[cl.head]) ** 0.3 * rng.uniform(0.6, 1.4) if cl.head in realm.chars
                      else 0)
        cl = bids[0]
        for loser in bids[1:]:                                           # the losers remember who undercut them
            if reg not in getattr(loser, "holds", {}) or reg not in cl.holds:
                continue                                                 # only where both already work the land
            key = (min(cl.id, loser.id), max(cl.id, loser.id))
            realm.clan_rivalry = getattr(realm, "clan_rivalry", {})
            realm.clan_rivalry[key] = realm.clan_rivalry.get(key, 0.0) + 0.5
        agent = target_in(realm, cl) if kind != "guard" else max((realm.chars[m] for m in cl.members), key=shinobi_power)
        ok = rng.random() < (0.85 if kind == "escort" else 0.85 + 0.03 * agent.srank)
        if kind != "escort":
            k.gold -= fee
            book(realm, k.id, "shinobi", -fee, f"{cl.name}: {kind}")
        else:
            place.wealth -= fee
            if g is not None and extra in g["quests"]:
                del g["quests"][extra]
        agent.gold += fee * 0.6
        cl.gold += fee * 0.4
        cl.holds[reg] = cl.holds.get(reg, 0) + 1
        if ok:
            cl.standing = getattr(cl, "standing", 10.0) + 2
            if kind == "spy":
                k.battle_mult = getattr(k, "battle_mult", 1.0) * 1.1        # they know where the enemy will be
            elif kind == "guard":
                k.security = getattr(k, "security", 0.0) + 20
            else:
                place.wealth += fee * 3
        else:
            cl.standing = getattr(cl, "standing", 10.0) * 0.9
            if kind == "spy" and rng.random() < 0.35:                    # most failures are quiet; some are caught
                realm.relation[k.id, extra] -= 15
                realm.relation[extra, k.id] -= 15
                realm.say(f"A spy of {cl.name} in the pay of {k.name} is taken in the court of "
                          f"{realm.kingdoms[extra].name}.", minor=True)
    first = max(clans, key=lambda cl: getattr(cl, "standing", 0.0))
    cur = next((cl for cl in clans if cl.id == getattr(realm, "first_clan", None)), None)
    if getattr(realm, "first_clan", None) != first.id and getattr(first, "standing", 0) > 40 and \
            (cur is None or first.standing > 1.3 * getattr(cur, "standing", 0.0)):     # it must clearly overtake
        realm.first_clan = first.id
        realm.say(f"**{first.name} is first among the shinobi clans** now: the richest contracts go to them.")


def clan_wars(realm):
    """Clans that keep undercutting each other in the same lands go to war for them; it ends when one side breaks."""
    rng = realm.rng
    wars = getattr(realm, "clan_war", None)
    if wars is None:
        wars = realm.clan_war = {}
    riv = getattr(realm, "clan_rivalry", {})
    for key, v in list(riv.items()):
        riv[key] = v * 0.9
        if key not in wars and v >= 8 and getattr(realm, "clan_truce", {}).get(key, 0) <= realm.year:
            a, b = (realm.clans[i] for i in key)
            if a.members and b.members:
                wars[key] = realm.year
                realm.say(f"**A clan war.** {a.name} and {b.name} have undercut each other once too often; the "
                          f"passes between them close, and the contracts of the land go to whoever is left.")
    for key, since in list(wars.items()):
        a, b = (realm.clans[i] for i in key)
        if not a.members or not b.members or realm.year - since > 15:
            win = a if getattr(a, "standing", 0) >= getattr(b, "standing", 0) else b
            lose = b if win is a else a
            for reg in list(getattr(lose, "holds", {})):
                win.holds[reg] = win.holds.get(reg, 0) + lose.holds.pop(reg)
            riv[key] = 0.0
            del wars[key]
            realm.clan_truce = getattr(realm, "clan_truce", {})
            realm.clan_truce[key] = realm.year + 20                          # an uneasy truce
            realm.say(f"The clan war ends: {win.name} holds the contracts of the land, and {lose.name} "
                      f"{'is no more' if not lose.members else 'withdraws into the hills'}.")
            continue
        if rng.random() < 0.5:                                            # a duel in the passes, aimed at their best
            ca, cb = target_in(realm, a), target_in(realm, b)
            pa, pb = shinobi_power(ca), shinobi_power(cb)
            win, lose = (ca, cb) if rng.random() < pa / (pa + pb) else (cb, ca)
            wc = realm.clans[win.clan]
            realm.say(f"In the clan war, {realm.who(win)} of {wc.name} meets {realm.who(lose)}; "
                      f"{technique(win)[1]} answers {technique(lose)[1]}, and {lose.name} does not come home.",
                      minor=not (5 in lose.nature_lv.values() or 5 in lose.arts.values()))
            realm.kill(lose, f"fell in the clan war to {win.full_name()}")
            wc.standing = getattr(wc, "standing", 10.0) + 10
            lc = realm.clans[lose.clan] if lose.clan is not None else None
            if lc is not None:
                lc.standing = getattr(lc, "standing", 10.0) * 0.85
            steal_scroll(realm, win, lose, wc)
