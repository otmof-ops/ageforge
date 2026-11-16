"""Guilds, crime and religion.

Every people has a Fighters' Guild, a College of Mages, a Smiths' Guild, a Merchants' League and a Thieves'
Guild; the Quiet Hand, a brotherhood of assassins, crosses every border. Members rank by skill and fame, a
guildmaster leads, and the guild trains its own. Crime grows where the governors are corrupt and the people
poor; criminals steal, pull heists, raid caravans and take murder contracts, carry bounties, and are caught
by lords' guards or hunted down by heroes. Religions begin as one faith per people, but anyone with the
voice and the zeal can proclaim a new one; faiths spread town by town, are persecuted, and decide whether a
ruler is seen as legitimate."""
import math

import numpy as np

from .data import ADULT, PEOPLES
from .data2 import (ASSASSINS, BASE_FAITHS, DEITY_A, DEITY_B, FAITH_A, GUILD_KINDS, GUILD_RANKS, TENETS)
from .skills import skill, use


# ---------------------------------------------------------------- guilds
class Guild:
    def __init__(self, gid, name, kind, people, year):
        self.id, self.name, self.kind, self.people, self.founded = gid, name, kind, people, year
        self.members, self.master, self.gold, self.masters = set(), None, 0.0, []


def found_guilds(realm):
    for f in range(4):
        cap = realm.capital_of(min(k.id for k in realm.kingdoms if k.people == f))
        for kind, pat in GUILD_KINDS.items():
            g = Guild(len(realm.guilds_soc), pat.format(cap.name), kind, f, 0)
            realm.guilds_soc.append(g)
    realm.guilds_soc.append(Guild(len(realm.guilds_soc), ASSASSINS, "assassins", None, 0))


def guild_kind_for(c):
    p = c.profession
    if p in ("smith", "enchanter"):
        return "smiths"
    if p == "merchant":
        return "merchants"
    if p in ("thief", "bandit"):
        return "thieves"
    if p == "assassin":
        return "assassins"
    if c.hero:
        return "mages" if c.school() is not None and c.classes[0][0] in ("Wizard", "Necromancer", "Druid",
                                                                           "Priest", "Shaman") else "fighters"
    return None


def guild_score(realm, c, kind):
    key = {"smiths": "Smithing", "merchants": "Mercantile", "thieves": "Sneak", "assassins": "Sneak"}.get(kind)
    base = skill(realm, c, key) if key else c.level
    return base + sum(c.fame.values()) / 50 + c.renown / 100


def guilds_year(realm):
    for g in realm.guilds_soc:
        g.members = {m for m in g.members if realm.chars[m].alive}
    for i in realm.living:
        c = realm.chars[i]
        kind = guild_kind_for(c)
        if kind is None:
            continue
        for g in realm.guilds_soc:
            if g.kind == kind and (g.people is None or g.people == c.people):
                if c.id not in g.members:
                    g.members.add(c.id)
                    c.guilds = getattr(c, "guilds", []) + [g.id]
                break
    for g in realm.guilds_soc:
        if not g.members:
            continue
        ranked = sorted((realm.chars[m] for m in g.members), key=lambda c: -guild_score(realm, c, g.kind))
        for n, c in enumerate(ranked):
            sc = guild_score(realm, c, g.kind)
            c.guild_rank = GUILD_RANKS[min(4, int(sc // 22))]
        top = ranked[0]
        if g.master != top.id:
            old = realm.chars.get(g.master)
            g.master = top.id
            g.masters.append((realm.year, top.id))
            top.titles.append(f"Guildmaster of {g.name} from year {realm.year}")
            realm.say(f"{realm.who(top)} becomes Guildmaster of {g.name}"
                      + (f", succeeding {old.full_name()}" if old is not None else "") + ".", minor=True)
        for m in g.members:                                         # dues
            c = realm.chars[m]
            due = c.gold * 0.05
            c.gold -= due
            g.gold += due


# ---------------------------------------------------------------- crime
def crime_year(realm):
    rng = realm.rng
    y = realm.year
    thieves_guild = {g.people: g for g in realm.guilds_soc if g.kind == "thieves"}
    for s in realm.alive_towns():
        k = realm.kingdoms[s.faction]
        lord = realm.chars.get(s.lord) if s.lord is not None else None
        guard = 0.1 * s.walls + (0.15 if lord is not None and (lord.legitimacy or 0) >= 50 else 0)
        poor = 0.2 if s.wealth < 60 else 0.0
        faith = realm.faith_of_town(s)
        charity = 0.1 if faith is not None and "charity" in faith.tenets else 0.0
        g = thieves_guild.get(s.culture)
        gang = 0.05 * min(4, len(g.members)) if g else 0.0
        target = max(0.02, 0.08 + 0.4 * k.corruption + poor + gang - guard - charity + 0.6 * getattr(s, "jobless", 0.0))
        s.crime = 0.8 * getattr(s, "crime", 0.1) + 0.2 * target
        s.wealth *= 1 - 0.03 * s.crime
    for i in list(realm.living):
        c = realm.chars[i]
        if c.profession not in ("thief", "assassin", "bandit") or getattr(c, "jailed_until", 0) > y:
            continue
        home = realm.by_id.get(c.home) if c.home is not None else None
        if home is None or not home.alive:
            towns = realm.alive_towns(c.realm)
            if not towns:
                continue
            home = towns[int(rng.integers(len(towns)))]
            c.home = home.id
        if c.profession == "thief":
            steal = home.wealth * 0.02 * (0.5 + skill(realm, c, "Sneak") / 100)
            home.wealth -= steal
            c.gold += steal
            c.bounty += steal * 0.5
            use(realm, c, "Sneak", 1.0)
            use(realm, c, "Pickpocket", 0.8)
            if skill(realm, c, "Security") >= 60 and rng.random() < 0.05:
                heist(realm, c)
            else:
                use(realm, c, "Security", 0.6)
        elif c.profession == "bandit":
            take = home.wealth * 0.03
            home.wealth -= take
            home.pop *= 0.995
            c.gold += take
            c.bounty += take
            use(realm, c, "Blade", 1.0)
            use(realm, c, "Archery", 0.6)
            from .crafts import gain_fame
            gain_fame(realm, c, "Sneak", take / 100)
        elif c.profession == "assassin":
            contract(realm, c)
        caught(realm, c, home)


def heist(realm, c):
    rng = realm.rng
    k = realm.kingdoms[c.realm]
    targets = [(it, "treasury of " + k.name, k.treasury) for it in k.treasury] + \
              [(it, "market of " + k.name, k.market) for it in k.market[:5]]
    if not targets:
        return
    it, where, src = max(targets, key=lambda t: t[0].bonus)
    src.remove(it)
    from .heroes import give_item
    give_item(realm, c, it, quiet=True)
    c.bounty += 10 + it.bonus * 40
    from .crafts import gain_fame
    gain_fame(realm, c, "Security", 60 + 100 * it.bonus)
    realm.say(f"**A heist.** {realm.who(c)} lifts {it.label()} from the {where}. "
              f"The bounty on their head is now {c.bounty:,.0f} gold.", minor=it.rarity < 4)


def willing(realm, kid):
    """Not every ruler at war will buy a murder: the honest, the just and the compassionate refuse."""
    r = realm.chars.get(realm.kingdoms[kid].ruler)
    if r is None:
        return False
    p = r.personality
    if p & {"honest", "just", "compassionate"}:
        return False
    return bool(p & {"deceitful", "cruel", "paranoid", "vengeful", "ambitious", "arbitrary"}) or realm.rng.random() < 0.15


def contract(realm, c):
    """The Quiet Hand takes contracts from kings at war and from plotters at court."""
    rng = realm.rng
    if rng.random() > 0.06:
        use(realm, c, "Sneak", 0.8)
        return
    wars = [(i, j) for i in range(realm.npol) for j in range(realm.npol) if i != j and realm.war[i, j]
            and willing(realm, i)]
    if not wars:
        return
    client, target_people = wars[int(rng.integers(len(wars)))]
    tk = realm.kingdoms[target_people]
    ck = realm.kingdoms[client]
    cands = [h for h in realm.heroes() if h.realm == target_people and h.alive] + \
            ([realm.chars[tk.ruler]] if tk.ruler is not None and realm.chars[tk.ruler].alive else [])
    if not cands:
        return
    tgt = cands[int(rng.integers(len(cands)))]
    spy = realm.chars.get(tk.council.get("spymaster"))
    defence = tgt.cr() / 50 + (spy.attr("intr") * 3 if spy else 0) + (20 if tgt.undead else 0) \
        + getattr(tk, "security", 0.0)                                  # guards, tasters, locked doors
    attack = skill(realm, c, "Sneak") + skill(realm, c, "Blade") / 2 + skill(realm, c, "Alchemy") / 3
    use(realm, c, "Sneak", 2.0)
    use(realm, c, "Blade", 1.0)
    if rng.random() < attack / (attack + defence + 60):
        rev = getattr(tk, "income", 0.0) * getattr(tk, "tax_now", 0.3)
        fee = max(5.0, (0.3 if tgt.id != tk.ruler else 1.0) * rev)       # a ruler's life costs a year of a realm's taxes
        if ck.gold < fee:
            return                                                       # the client cannot pay; no one moves
        ck.gold -= fee
        from .events import book
        book(realm, client, "assassins", -fee, "a quiet payment")
        c.gold += fee
        c.bounty += fee
        from .crafts import gain_fame
        gain_fame(realm, c, "assassin", 80 if tgt.id == tk.ruler else 30)
        realm.say(f"{realm.who(tgt)} is found dead, a black feather on the pillow: the work of the Quiet Hand, paid in "
                  f"{realm.pname(client)} gold.", minor=tgt.id != tk.ruler)
        realm.kill(tgt, "was murdered by the Quiet Hand", quiet=True, by=realm.chars.get(realm.kingdoms[client].ruler))
        tk.security = getattr(tk, "security", 0.0) + 40                     # the court hires guards and tasters
    elif rng.random() < 0.3:
        realm.say(f"An assassin of the Quiet Hand, {realm.who(c)}, is taken in the act and hanged.", minor=True)
        realm.kill(c, "was hanged after a failed contract")


def caught(realm, c, home):
    rng = realm.rng
    if not c.alive:
        return
    lord = realm.chars.get(home.lord) if home.lord is not None else None
    watch = 0.03 + 0.04 * home.walls / 5 + (0.05 if lord is not None and (lord.legitimacy or 0) >= 50 else 0)
    watch *= max(0.2, 1 - skill(realm, c, "Sneak") / 140)
    if rng.random() < watch:
        r = realm.chars.get(realm.kingdoms[home.faction].ruler)
        if r is not None and "cruel" in r.personality or c.bounty > 1500:
            realm.say(f"{realm.who(c)}, wanted for {c.bounty:,.0f} gold, is taken by the watch of {home.name} "
                      f"and hanged.", minor=c.bounty < 1000)
            realm.kill(c, f"was hanged in {home.name}")
            if r is not None:
                from .governance import add_tyranny
                add_tyranny(realm, r, 0.5)
        else:
            c.jailed_until = realm.year + int(rng.integers(3, 15))
            c.bounty *= 0.3


def bounty_target(realm, h):
    best, bv = None, 50.0
    for i in realm.living:
        c = realm.chars[i]
        if c.profession in ("thief", "assassin", "bandit") and c.bounty > bv and \
                getattr(c, "jailed_until", 0) <= realm.year and c.realm == h.realm:
            best, bv = c, c.bounty
    return best


def hunt_bounty(realm, h):
    """A hero hunts the most wanted criminal of their people."""
    rng = realm.rng
    c = bounty_target(realm, h)
    if c is None:
        return 0.0
    evade = skill(realm, c, "Sneak") * 10 + c.level * 20
    use(realm, h, "Sneak", 0.5)
    if rng.random() < h.cr() / (h.cr() + evade):
        reward = c.bounty
        h.gold += reward
        from .events import private
        private(realm, h, "bounty", reward, f"{h.full_name()} collects {reward:,.0f} for {c.full_name()}")
        realm.say(f"{realm.who(h)} brings in {realm.who(c)} and collects a bounty of {reward:,.0f} gold.",
                  minor=reward < 1000)
        if c.items:
            from .heroes import give_item
            for it in list(c.items):
                give_item(realm, h, it, quiet=True)
            c.items = []
        realm.kill(c, f"was brought to justice by {h.full_name()}")
        return 5 + reward / 100
    return 0.0


# ---------------------------------------------------------------- religion
class Religion:
    def __init__(self, rid, name, deity, tenets, founder, year, people, parent=None):
        self.id, self.name, self.deity, self.tenets = rid, name, deity, tenets
        self.founder, self.founded, self.people, self.parent = founder, year, people, parent
        self.holy_site = None
        self.peak = 0.0


def found_faiths(realm):
    rng = realm.rng
    base_tenets = [["sacred kingship", "charity"], ["ancestor worship", "pacifism"],
                   ["ancestor worship", "wealth gospel"], ["holy war", "martyrdom"]]
    for f in range(4):
        name, deity = BASE_FAITHS[f]
        r = Religion(len(realm.religions), name, deity, base_tenets[f], None, 0, f)
        realm.religions.append(r)
        cap = realm.capital_of(min(k.id for k in realm.kingdoms if k.people == f))
        r.holy_site = cap.id
    for s in realm.settlements:
        s.faith = {s.culture: 1.0}
    for c in realm.chars.values():
        c.religion = c.people


def religion_year(realm):
    rng = realm.rng
    y = realm.year
    for s in realm.alive_towns():
        if not getattr(s, "faith", None):
            s.faith = {realm.kingdoms[s.faction].faith: 1.0}
    # prophets
    for i in list(realm.living):
        c = realm.chars[i]
        if (c.adult(y) and ("zealous" in c.personality or c.profession == "priest")
                and skill(realm, c, "Speech") >= 45 and c.attr("lore") >= 7 and rng.random() < 0.004):
            proclaim(realm, c)
    # missionaries, the pull of the crown, persecution
    priests = [realm.chars[i] for i in realm.living if realm.chars[i].profession == "priest"]
    towns = realm.alive_towns()
    tpos = np.array([t.pos for t in towns], float) if towns else np.zeros((0, 2))
    for p in priests:
        home = realm.by_id.get(p.home) or realm.capital_of(p.realm)
        if home is None or not home.alive:
            continue
        r = p.religion
        power = 0.01 + skill(realm, p, "Speech") / 2000
        if "holy war" in realm.religions[r].tenets or "martyrdom" in realm.religions[r].tenets:
            power *= 1.2
        near = np.flatnonzero(np.hypot(tpos[:, 0] - home.pos[0], tpos[:, 1] - home.pos[1]) <= 10)
        for n in near:
            shift(towns[int(n)], r, power)
        use(realm, p, "Speech", 1.0)
        from .crafts import gain_fame
        gain_fame(realm, p, "prophet", power * 50)
    for k in realm.kingdoms:
        if realm.extinct[k.realm] or k.ruler is None:
            continue
        r = realm.chars[k.ruler]
        my = realm.religions[r.religion]
        for t in realm.alive_towns(k.realm):
            shift(t, r.religion, 0.004)
            if "zealous" in r.personality and "cruel" in r.personality:
                for other in list(t.faith):
                    if other != r.religion:
                        t.faith[other] *= 0.9
                k.persecuting = True
        # the realm's majority faith, and a ruler who does not share it
        shares = {}
        for t in realm.alive_towns(k.realm):
            for rid, sh in t.faith.items():
                shares[rid] = shares.get(rid, 0.0) + sh * t.pop
        if shares:
            major = max(shares, key=shares.get)
            if major != k.faith:
                realm.say(f"{realm.religions[major].name} is now the faith of most of {k.name}.")
                k.faith = major
            if r.religion != major and "zealous" not in r.personality and rng.random() < 0.1:
                realm.say(f"{realm.who(r)} turns from {my.name} to {realm.religions[major].name}, the faith of "
                          f"their people.")
                r.religion = major
    counts = {}
    for t in towns:                                                  # one pass over the towns, not one per faith
        for rid, sh in t.faith.items():
            counts[rid] = counts.get(rid, 0.0) + sh * t.pop
    for rel in realm.religions:
        total = counts.get(rel.id, 0.0)
        rel.followers = total
        rel.peak = max(rel.peak, total)
        if rel.founder is not None and 0 < rel.peak and total < 1 and not getattr(rel, "dead", False) \
                and y - rel.founded > 10:
            rel.dead = True
            realm.say(f"The last of {rel.name} is gone; its prophet's words are forgotten.", minor=True)


def shift(t, rid, amount):
    t.faith[rid] = t.faith.get(rid, 0.0) + amount
    tot = sum(t.faith.values())
    for k in list(t.faith):
        t.faith[k] /= tot
        if t.faith[k] < 0.005:
            del t.faith[k]


def proclaim(realm, c):
    rng = realm.rng
    home = realm.by_id.get(c.home) or realm.capital_of(c.realm)
    if home is None:
        return
    parent = realm.religions[c.religion]
    n = int(rng.integers(2, 4))
    pool = list(TENETS)
    bias = []
    if "kind" in c.personality:
        bias += ["charity", "pacifism"]
    if "wrathful" in c.personality or "cruel" in c.personality:
        bias += ["holy war", "martyrdom"]
    if "greedy" in c.personality:
        bias.append("wealth gospel")
    if any(m.kind == "dragon" and m.alive for m in realm.monsters) and rng.random() < 0.2:
        bias.append("dragon veneration")
    if getattr(realm, "concord", None) is not None and rng.random() < 0.3:
        bias.append("the Concord awaits")
    tenets = []
    for t in bias + list(rng.permutation(pool)):
        if t not in tenets:
            tenets.append(str(t))
        if len(tenets) >= n:
            break
    if "pacifism" in tenets and "holy war" in tenets:
        tenets.remove("holy war")
    deity = f"{rng.choice(DEITY_A)} {rng.choice(DEITY_B)}"
    name = f"the {rng.choice(FAITH_A)} of {deity}"
    r = Religion(len(realm.religions), name, deity, tenets, c.id, realm.year, c.people, parent.id)
    r.holy_site = home.id
    realm.religions.append(r)
    c.religion = r.id
    shift(home, r.id, 0.3)
    c.profession = c.profession or "priest"
    realm.say(f"**A new faith.** {realm.who(c)} proclaims {name} in {home.name}, breaking from {parent.name}. "
              f"Its teachings: {'; '.join(TENETS[t] for t in tenets)}.")
    from .crafts import gain_fame
    gain_fame(realm, c, "prophet", 100)


def ruler_faith_legitimacy(realm, k, r):
    """A ruler who shares the realm's faith is accepted more readily; sacred kingship adds to it."""
    if r.religion == k.faith:
        bonus = 8 + (6 if "sacred kingship" in realm.religions[k.faith].tenets else 0)
    else:
        bonus = -10
    return bonus


def tenet_aggression(realm, k):
    tenets = realm.religions[k.faith].tenets
    return 1 + 0.4 * ("holy war" in tenets) - 0.5 * ("pacifism" in tenets)
