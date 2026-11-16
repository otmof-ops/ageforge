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
        if c.nature_lv[n] < 5 and c.ki >= 40 * c.nature_lv[n] ** 1.6:
            c.nature_lv[n] += 1
            if c.nature_lv[n] >= 4:
                realm.say(f"{realm.who(c)} of {cl.name} masters {NATURES[n][c.nature_lv[n] - 1]}, a "
                          f"tier-{ROMAN[c.nature_lv[n]]} {n} technique.", minor=c.nature_lv[n] < 5)
    if rng.random() < 0.1 * speed:
        a = str(rng.choice(list(c.arts)))
        if c.arts[a] < 5 and c.ki >= 30 * (c.arts[a] + 1) ** 1.5:
            c.arts[a] += 1
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
    rank = min(4, int(shinobi_power(c) ** 0.5 / 9))
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
        defence = t.cr() * (2 if t.undead else 1)
        if rng.random() < pw / (pw + 3 * defence + 500):
            c.gold += 400
            gain_fame(realm, c, "shinobi", 40)
            realm.say(f"{realm.who(t)} dies in the night with no wound but a needle-mark: {cl.name} has done "
                      f"{realm.pname(client)} work.", minor=tp != t.realm or t.id != tk.ruler)
            realm.kill(t, f"was killed by a shinobi of {cl.name}", quiet=True, by=realm.chars.get(realm.kingdoms[client].ruler))
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
    ca = max((realm.chars[m] for m in A.members), key=shinobi_power)
    cb = max((realm.chars[m] for m in B.members), key=shinobi_power)
    pa, pb = shinobi_power(ca), shinobi_power(cb)
    win, lose = (ca, cb) if rng.random() < pa / (pa + pb) else (cb, ca)
    wc = realm.clans[win.clan]
    realm.say(f"**Clan feud.** {realm.who(win)} of {wc.name} meets {realm.who(lose)} in the mountain passes; "
              f"{technique(win)[1]} answers {technique(lose)[1]}, and {lose.name} does not come home.")
    from .crafts import gain_fame
    gain_fame(realm, win, "shinobi", 60)
    realm.kill(lose, f"fell in a clan feud to {win.full_name()}")
