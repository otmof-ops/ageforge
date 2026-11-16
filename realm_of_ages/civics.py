"""Civilisation: eras and innovations, culture research, governments, laws and agendas, armies and garrisons,
political ideologies, reform movements, decisions, and culture events.

Every people begins as scattered tribes. Each culture researches innovations era by era (Tribal, Early
Medieval, High Medieval, Late Medieval, and no further): walls and castles, crop rotation, levy rolls and
men-at-arms, written law, coinage, sea charts, hereditary titles, guild charters, universities, banking.
A culture's leading realm sets its research focus. Innovations open new governments (feudal, clan,
republic, theocracy, administrative), stronger crown authority, better ships and metals, and colonisation.

Realms tax their towns, raise levies, keep regiments of men-at-arms and garrison their walls. Rulers set
laws and pursue an agenda. Every notable person has views on how they should be governed (authority,
succession, faith, economy, tradition) from their culture, their nature, their trade and the trend of the
times; when the great of a realm disagree with its government, a reform movement grows. Decisions turn
tribes into chiefdoms, duchies, kingdoms and empires, and unite tribes into confederations."""
import math

import numpy as np

from .data import ADULT, MARTIAL, PEOPLES

ERAS = ["Tribal", "Early Medieval", "High Medieval", "Late Medieval"]
# name, category, effects
INNOVATIONS = [
    [("Palisades", "military", {"walls_cap": 2, "garrison": 0.05}),
     ("Hunting Grounds", "civic", {"food": 0.05}),
     ("War Bands", "military", {"levy": 0.02, "maa": ["Axemen", "Skirmishers"]}),
     ("Oral Law", "civic", {"legit": 3}),
     ("Kinship Councils", "civic", {"corruption": -0.05}),
     ("Longships", "maritime", {"ships": 1}),
     ("Ironworking", "craft", {"material": 2}),
     ("Sacred Fires", "cultural", {"research": 0.1})],
    [("Motte and Bailey", "military", {"walls_cap": 3, "garrison": 0.08}),
     ("Two-Field Rotation", "civic", {"food": 0.08}),
     ("Levy Rolls", "military", {"levy": 0.03, "maa": ["Spearmen", "Archers", "Light Horse"]}),
     ("Written Law", "civic", {"corruption": -0.1, "legit": 3, "crown": 2}),
     ("Coinage", "civic", {"wealth": 0.1}),
     ("Sea Charts", "maritime", {"ships": 2, "colonize": True}),
     ("Hereditary Titles", "civic", {"feudal": True}),
     ("Monastery Schools", "cultural", {"research": 0.15, "theocracy": True}),
     ("Steel Forging", "craft", {"material": 3})],
    [("Stone Keeps", "military", {"walls_cap": 4, "garrison": 0.12}),
     ("Three-Field Rotation", "civic", {"food": 0.1}),
     ("Knightly Orders", "military", {"maa": ["Knights", "Heavy Infantry", "Crossbowmen", "culture"]}),
     ("Royal Chancery", "civic", {"crown": 3, "corruption": -0.1}),
     ("Guild Charters", "civic", {"wealth": 0.1, "republic": True}),
     ("Carrack Hulls", "maritime", {"ships": 3}),
     ("Universities", "cultural", {"research": 0.2}),
     ("Silversteel Craft", "craft", {"material": 5})],
    [("Concentric Castles", "military", {"walls_cap": 5, "garrison": 0.18}),
     ("Heavy Plough", "civic", {"food": 0.12}),
     ("Professional Armies", "military", {"levy": 0.02, "maa": ["Pikemen", "Plate Knights", "War Mages"]}),
     ("Bureaucracy", "civic", {"crown": 4, "corruption": -0.15, "admin": True}),
     ("Banking Houses", "civic", {"wealth": 0.15}),
     ("Galleon Yards", "maritime", {"ships": 4}),
     ("Printed Sagas", "cultural", {"research": 0.2, "legacy": 0.25}),
     ("Adamant Forging", "craft", {"material": 6})],
]
INNOV = {n: (era, cat, fx) for era, row in enumerate(INNOVATIONS) for n, cat, fx in row}
COST = [300, 700, 1300, 2100]
MAA = {  # regiment: (power, upkeep)
    "Axemen": (1.0, 1.0), "Skirmishers": (0.9, 0.8), "Spearmen": (1.2, 1.0), "Archers": (1.3, 1.1),
    "Light Horse": (1.6, 1.6), "Knights": (3.0, 3.0), "Heavy Infantry": (1.9, 1.8), "Crossbowmen": (1.7, 1.4),
    "Pikemen": (2.2, 1.8), "Plate Knights": (4.5, 4.0), "War Mages": (3.8, 3.5),
    "Shieldwall Wardens": (2.6, 2.0), "Bladesingers": (3.2, 2.8), "Ironbreakers": (3.4, 2.8),
    "Wolf Riders": (3.0, 2.4),
}
CULTURE_UNIT = ["Shieldwall Wardens", "Bladesingers", "Ironbreakers", "Wolf Riders"]
ETHOS = ["Courtly", "Spiritual", "Stoic", "Bellicose"]
TRADITIONS = {  # culture traditions, adopted with culture points; (effect, ideology nudge)
    "Warrior Culture": ({"levy": 0.02}, {"auth": 0.1}), "Seafarers": ({"ships": 0}, {"econ": 0.1}),
    "Mountain Homes": ({"garrison": 0.05}, {"trad": 0.1}), "Sacred Groves": ({"research": 0.05}, {"faith": 0.15}),
    "Ancestor Sagas": ({"legacy": 0.15}, {"trad": 0.15}), "Mercantile Spirit": ({"wealth": 0.08}, {"econ": 0.2}),
    "Horse Lords": ({"levy": 0.01}, {"auth": 0.05}), "Isolationist": ({"garrison": 0.05}, {"trad": 0.2}),
    "Republican Ideals": ({"wealth": 0.05}, {"auth": -0.2, "succ": -0.2}),
    "Philosopher Kings": ({"research": 0.1}, {"trad": -0.15}), "Blood Feuds": ({"levy": 0.02}, {"succ": -0.1}),
    "Hospitality": ({"legit": 2}, {"auth": -0.05}), "Stone Masons": ({"walls": 1}, {"trad": 0.05}),
}
AXES = ["auth", "succ", "faith", "econ", "trad"]
AXIS_NAMES = {"auth": ("absolutist", "councilist"), "succ": ("hereditary", "elective"),
              "faith": ("theocratic", "secular"), "econ": ("mercantile", "agrarian"),
              "trad": ("traditionalist", "reformist")}
CULTURE_VIEWS = [  # where each people starts on the five axes
    {"auth": 0.2, "succ": 0.4, "faith": 0.1, "econ": 0.3, "trad": 0.0},
    {"auth": -0.2, "succ": -0.5, "faith": 0.3, "econ": -0.2, "trad": 0.6},
    {"auth": 0.3, "succ": 0.5, "faith": 0.0, "econ": 0.4, "trad": 0.7},
    {"auth": 0.5, "succ": -0.6, "faith": 0.2, "econ": -0.5, "trad": 0.2},
]
GOV_VIEWS = {
    "tribal": {"auth": -0.2, "succ": -0.4, "faith": 0.0, "econ": -0.5, "trad": 0.5},
    "clan": {"auth": 0.1, "succ": 0.2, "faith": 0.0, "econ": -0.2, "trad": 0.5},
    "feudal": {"auth": 0.3, "succ": 0.6, "faith": 0.1, "econ": 0.0, "trad": 0.3},
    "republic": {"auth": -0.5, "succ": -0.8, "faith": -0.2, "econ": 0.8, "trad": -0.3},
    "theocracy": {"auth": 0.3, "succ": -0.5, "faith": 0.9, "econ": -0.2, "trad": 0.4},
    "administrative": {"auth": 0.7, "succ": -0.2, "faith": -0.1, "econ": 0.3, "trad": -0.4},
}
GOV_LAW = {"tribal": None, "clan": "seniority", "feudal": "primogeniture", "republic": "elective",
           "theocracy": "elective", "administrative": "elective"}
TAX = {"tribal": 0.015, "clan": 0.025, "feudal": 0.03, "republic": 0.05, "theocracy": 0.035, "administrative": 0.05}
LEVY = {"tribal": 0.12, "clan": 0.1, "feudal": 0.08, "republic": 0.05, "theocracy": 0.07, "administrative": 0.06}
AGENDAS = ["conquest", "building", "faith", "learning", "trade", "unity"]
GREAT_WORKS = ["the Great Library of {}", "the Cathedral of {}", "the Colossus of {}", "the Hall of Kings at {}",
               "the Sky Bridge of {}", "the Tower of Stars at {}", "the Hanging Gardens of {}", "the Grand Forge of {}"]
RANKS = ["tribe", "chiefdom", "duchy", "kingdom", "empire"]


# ---------------------------------------------------------------- culture state
class Culture:
    def __init__(self, people):
        self.people = people
        self.era = 0
        self.innovations = {}
        self.progress = 0.0
        self.focus = None
        self.ethos = ETHOS[people]
        self.traditions = []
        self.culture_points = 0.0
        self.trend = dict(CULTURE_VIEWS[people])
        self.golden = 0
        self.dark = 0
        self.history = []

    def has(self, fx_key):
        return any(fx_key in INNOV[n][2] for n in self.innovations)

    def total(self, key):
        """A culture's summed bonus for one effect (cached until it learns an innovation or a tradition)."""
        stamp = (len(self.innovations), len(self.traditions))
        cache = self.__dict__.get("_tot")
        if cache is None or cache[0] != stamp:
            cache = self._tot = (stamp, {})
        v = cache[1].get(key)
        if v is None:
            v = sum(INNOV[n][2].get(key, 0) for n in self.innovations
                    if isinstance(INNOV[n][2].get(key, 0), (int, float)))
            v += sum(TRADITIONS[t][0].get(key, 0) for t in self.traditions)
            cache[1][key] = v
        return v

    def best(self, key, default):
        vals = [INNOV[n][2][key] for n in self.innovations if key in INNOV[n][2]]
        return max(vals) if vals else default


def found_cultures(realm):
    realm.cultures = [Culture(p) for p in range(4)]
    for k in realm.kingdoms:
        setup_realm(realm, k)


def setup_realm(realm, k, gov="tribal"):
    k.gov = getattr(k, "gov", gov)
    k.laws = getattr(k, "laws", {"crown": 1, "tolerance": "tolerant", "trade": "open", "stance": "defensive"})
    k.agenda = getattr(k, "agenda", "unity")
    k.gold = getattr(k, "gold", 50.0)
    k.maa = getattr(k, "maa", {})
    k.levy, k.army_power, k.garrison_bonus = 0.0, 0.0, 0.0
    k.reform = getattr(k, "reform", 0.0)
    k.great_works = getattr(k, "great_works", [])
    if GOV_LAW.get(k.gov):
        k.law = GOV_LAW[k.gov]


def _perk(realm, k, eff):
    r = realm.chars.get(k.ruler)
    from .court import perk_value
    return perk_value(r, eff) if r is not None else 0


def culture_of(realm, f):
    return realm.cultures[realm.kingdoms[f].people]


# ---------------------------------------------------------------- research
def research_year(realm):
    rng = realm.rng
    pts = [0.0] * 4
    lead = [None] * 4
    for k in realm.kingdoms:
        if realm.extinct[k.id] or k.ruler is None:
            continue
        r = realm.chars[k.ruler]
        towns = realm.alive_towns(k.id)
        lore = realm.chars.get(k.council.get("loremaster"))
        mage = realm.chars.get(k.council.get("mage"))
        p = (len(towns) ** 0.5 + r.attr("lrn") / 3 + (lore.attr("lore") / 4 if lore else 0)
             + (max(0, mage.tier()) if mage else 0)) * (1.4 if k.agenda == "learning" else 1.0)
        pts[k.people] += p
        if lead[k.people] is None or len(towns) > len(realm.alive_towns(lead[k.people].id)):
            lead[k.people] = k
    eras = [c.era for c in realm.cultures]
    for cu in realm.cultures:
        mult = 1 + cu.total("research") + (0.5 if cu.golden > realm.year else 0) - (0.4 if cu.dark > realm.year else 0)
        diffusion = 1 + 0.15 * sum(1 for e in eras if e > cu.era)             # ideas spread from those ahead
        cu.progress += pts[cu.people] * mult * diffusion * 0.25
        cu.culture_points += pts[cu.people] * 0.05
        # the leading realm of a culture chooses what it is fascinated by
        if cu.focus is None or cu.focus in cu.innovations:
            open_ = [n for n, (era, cat, fx) in INNOV.items() if era <= cu.era and n not in cu.innovations]
            if not open_:
                continue
            k = lead[cu.people]
            want = {"conquest": "military", "building": "civic", "trade": "civic", "learning": "cultural",
                    "faith": "cultural", "unity": "civic"}.get(getattr(k, "agenda", "unity") if k else "unity")
            pref = [n for n in open_ if INNOV[n][1] == want] or open_
            cu.focus = pref[int(rng.integers(len(pref)))]
        cost = COST[INNOV[cu.focus][0]]
        if cu.progress >= cost:
            cu.progress -= cost
            cu.innovations[cu.focus] = realm.year
            cu.history.append((realm.year, f"discovers {cu.focus}"))
            realm.say(f"The {PEOPLES[cu.people]} learn {cu.focus}.", minor=True)
            cu.focus = None
            row = [n for n, _, _ in INNOVATIONS[cu.era]]
            if cu.era < len(ERAS) - 1 and sum(1 for n in row if n in cu.innovations) >= 0.7 * len(row):
                cu.era += 1
                cu.history.append((realm.year, f"enters the {ERAS[cu.era]} era"))
                cu.trend["trad"] -= 0.1
                realm.say(f"**A new era.** The {PEOPLES[cu.people]} enter the {ERAS[cu.era]} age.")
        if cu.culture_points >= 60 * (len(cu.traditions) + 1) and len(cu.traditions) < 5:
            cu.culture_points = 0
            avail = [t for t in TRADITIONS if t not in cu.traditions]
            t = avail[int(rng.integers(len(avail)))]
            cu.traditions.append(t)
            for ax, v in TRADITIONS[t][1].items():
                cu.trend[ax] += v
            cu.history.append((realm.year, f"adopts the tradition {t}"))
            realm.say(f"The {PEOPLES[cu.people]} take up a new tradition: {t}.", minor=True)


# ---------------------------------------------------------------- caps the rest of the world reads
def walls_cap(realm, f):
    memo = getattr(realm, "_walls_memo", None)
    if memo is None or memo[0] != realm.year:
        memo = realm._walls_memo = (realm.year, {})
    v = memo[1].get(f)
    if v is None:
        cu = culture_of(realm, f)
        v = memo[1][f] = cu.best("walls_cap", 1) + sum(TRADITIONS[t][0].get("walls", 0) for t in cu.traditions)
    return v


def material_cap(realm, culture):
    return realm.cultures[culture].best("material", 1)


def ship_cap(realm, f):
    return culture_of(realm, f).best("ships", 0)


def can_colonize(realm, f):
    return culture_of(realm, f).has("colonize")


# ---------------------------------------------------------------- ideologies
def views(realm, c):
    """What this person thinks governance should be: their culture's trend, their nature, their trade."""
    cache = getattr(c, "_views", None)
    if cache is not None and cache[0] == realm.year:
        return cache[1]
    v = dict(realm.cultures[c.people].trend)
    p = c.personality
    v["auth"] += 0.3 * ("ambitious" in p) + 0.2 * ("cruel" in p) - 0.3 * ("kind" in p) - 0.2 * ("trusting" in p)
    v["faith"] += 0.4 * ("zealous" in p) - 0.4 * ("cynical" in p)
    v["econ"] += 0.3 * ("greedy" in p) - 0.2 * ("generous" in p)
    v["trad"] += 0.2 * ("content" in p) - 0.3 * ("ambitious" in p) - 0.1 * ("diligent" in p)
    v["succ"] += 0.3 if c.house is not None and realm.houses[c.house].rulers > 0 else -0.1
    if c.profession == "merchant":
        v["econ"] += 0.6
        v["succ"] -= 0.4
    elif c.profession == "priest":
        v["faith"] += 0.6
    elif c.hero:
        v["trad"] -= 0.2
    if not hasattr(c, "view_noise"):
        c.view_noise = {a: float(realm.rng.normal(0, 0.2)) for a in AXES}
    for a in AXES:
        v[a] = max(-1.0, min(1.0, v[a] + c.view_noise[a]))
    c._views = (realm.year, v)
    return v


def gov_position(k):
    pos = dict(GOV_VIEWS[k.gov])
    pos["auth"] += 0.15 * (k.laws["crown"] - 1)
    return pos


def agreement(realm, c, k):
    """-1 (hates how they are ruled) to +1 (it is exactly how it should be)."""
    v, g = views(realm, c), gov_position(k)
    return 1 - sum(abs(v[a] - g[a]) for a in AXES) / 5


def ideology_label(v):
    out = []
    for a in AXES:
        if abs(v[a]) >= 0.35:
            out.append(AXIS_NAMES[a][0] if v[a] > 0 else AXIS_NAMES[a][1])
    return ", ".join(out) or "moderate"


def trends_year(realm):
    """The trend of the times: what happens to a people moves what they think."""
    for cu in realm.cultures:
        ks = [k for k in realm.kingdoms if not realm.extinct[k.id] and k.people == cu.people and k.ruler is not None]
        if not ks:
            continue
        tyr = np.mean([realm.chars[k.ruler].tyranny for k in ks])
        wars = np.mean([float(realm.war[k.id].any()) for k in ks])
        merch = sum(1 for i in realm.living if realm.chars[i].people == cu.people
                    and realm.chars[i].profession == "merchant") / max(1, len(ks))
        lands = {t.landmass for k in ks for t in realm.alive_towns(k.id)}
        cu.trend["auth"] += 0.006 * (wars - 0.4) - 0.003 * (tyr - 20) / 10
        cu.trend["econ"] += 0.004 * (merch - 1.5)
        cu.trend["faith"] += 0.008 if any(m.alive and m.kind == "calamity" and m.landmass in lands
                                          for m in realm.monsters) else -0.002
        for a in AXES:                                                    # and back towards their roots
            cu.trend[a] += 0.04 * (CULTURE_VIEWS[cu.people][a] - cu.trend[a])
            cu.trend[a] = max(-1.0, min(1.0, cu.trend[a]))


# ---------------------------------------------------------------- the treasury, the levy, the regiments
def armies_year(realm):
    rng = realm.rng
    for k in realm.kingdoms:
        if realm.extinct[k.id] or k.ruler is None:
            continue
        cu = culture_of(realm, k.id)
        towns = realm.alive_towns(k.id)
        base = sum(getattr(t, "income", 0.0) for t in towns if getattr(t, "occupied_by", None) is None)
        k.tax_now = (0.12 + (TAX[k.gov] - 0.03) * 4 + 0.05 * (k.laws["crown"] - 1)) * \
            (0.6 if getattr(k, "tax_relief", 0) > realm.year else 1.0)
        from .events import book
        taxes = base * k.tax_now * (1 - k.corruption) * (1 + cu.total("wealth")) * getattr(k, "tax_mult", 1.0) \
            * (1 + _perk(realm, k, "tax")) * getattr(k, "collect", 1.0)
        k.gold += taxes
        book(realm, k.id, "taxes", taxes)
        # hosts in the field must be fed and paid
        field = sum(a.men for a in getattr(realm, "armies", ()) if a.realm == k.id) * 0.008
        k.gold -= field
        book(realm, k.id, "field armies", -field)
        k.levy = sum(t.pop for t in towns) * (LEVY[k.gov] + cu.total("levy"))
        k.garrison_bonus = cu.total("garrison")
        for t in towns:
            t.garrison = t.pop * (0.02 + k.garrison_bonus) * (1 + 0.2 * t.walls)
        # upkeep, and new regiments when there is gold and need
        upkeep = sum(MAA[m][1] * n for m, n in k.maa.items()) * 8
        k.gold -= upkeep
        book(realm, k.id, "army upkeep", -upkeep)
        while k.gold < -2 * max(10.0, base * k.tax_now) and k.maa:     # a crown borrows before it disbands
            m = max(k.maa, key=lambda m: MAA[m][1])
            k.maa[m] -= 1
            if k.maa[m] <= 0:
                del k.maa[m]
            k.gold += MAA[m][1] * 8
        avail = [m for n in cu.innovations for m in INNOV[n][2].get("maa", [])]
        avail = [CULTURE_UNIT[cu.people] if m == "culture" else m for m in avail]
        from .resources import CAVALRY, has
        avail = [m for m in avail if (m not in CAVALRY or has(k, "horses") or has(k, "war beasts"))
                 and (m != "War Mages" or has(k, "mana crystals"))
                 and (m != "Wolf Riders" or has(k, "war beasts") or has(k, "horses"))]
        if "War Mages" in avail and (realm.chars.get(k.council.get("mage")) is None
                                     or realm.chars[k.council["mage"]].tier() < 3):
            avail.remove("War Mages")
        need = realm.war[k.id].any() or k.agenda == "conquest" or k.laws["stance"] == "expansionist"
        if avail and k.gold > 120 + 4 * sum(MAA[m][1] * n for m, n in k.maa.items()) and (need or rng.random() < 0.2):
            m = max(avail, key=lambda m: MAA[m][0] / MAA[m][1] + rng.random() * 0.3)
            k.maa[m] = k.maa.get(m, 0) + 1
            k.gold -= MAA[m][1] * 40
            book(realm, k.id, "regiments", -MAA[m][1] * 40, f"raises a regiment of {m}")
        maa_power = sum(MAA[m][0] * n * 300 for m, n in k.maa.items())
        k.army_power = k.levy + maa_power
        k.battle_mult *= 1 + min(2.0, maa_power / max(1.0, k.levy))
        for t in towns:                                                    # walls are only as high as the craft allows
            t.walls = min(t.walls, walls_cap(realm, k.id))


# ---------------------------------------------------------------- laws and agendas
def laws_year(realm):
    rng = realm.rng
    for k in realm.kingdoms:
        if realm.extinct[k.id] or k.ruler is None:
            continue
        r = realm.chars[k.ruler]
        cu = culture_of(realm, k.id)
        if getattr(k, "agenda_for", None) != r.id:
            k.agenda_for = r.id
            p = r.personality
            w = {"conquest": 1 + 2 * ("ambitious" in p) + 2 * ("wrathful" in p) + (cu.ethos == "Bellicose"),
                 "building": 1 + 2 * ("diligent" in p) + (cu.ethos == "Stoic"),
                 "faith": 1 + 3 * ("zealous" in p) + (cu.ethos == "Spiritual"),
                 "learning": 1 + r.attr("lrn") / 5, "trade": 1 + 2 * ("greedy" in p) + (cu.ethos == "Courtly"),
                 "unity": 1 + 2 * ("kind" in p) + 2 * ("calm" in p)}
            names = list(w)
            pr = np.array([w[n] for n in names], float)
            k.agenda = names[int(rng.choice(len(names), p=pr / pr.sum()))]
            k.laws["stance"] = "expansionist" if k.agenda == "conquest" else "defensive"
            k.laws["tolerance"] = "intolerant" if "zealous" in r.personality else "tolerant"
            k.laws["trade"] = "protectionist" if "greedy" in r.personality and k.agenda != "trade" else "open"
        # a ruler who believes in a strong crown raises its authority, as far as the realm's learning allows
        cap = cu.best("crown", 1)
        v = views(realm, r)
        if k.laws["crown"] < cap and (r.legitimacy or 0) >= 60 and v["auth"] > 0.2 and rng.random() < 0.05:
            k.laws["crown"] += 1
            realm.say(f"{realm.who(r)} raises the crown's authority in {k.name} (level {k.laws['crown']}).",
                      minor=True)
        k.aggr *= 1.4 if k.laws["stance"] == "expansionist" else 0.8
        if k.agenda == "building":
            for t in realm.alive_towns(k.id)[:5]:
                t.wealth += 3
        elif k.agenda == "trade":
            for t in realm.alive_towns(k.id):
                if t.port:
                    t.wealth += 2


# ---------------------------------------------------------------- decisions
def decisions_year(realm):
    rng = realm.rng
    from .titles import style
    held = {}
    for kd in realm.ktitles.values():
        if kd["ctrl"] is not None:
            held[kd["ctrl"]] = held.get(kd["ctrl"], 0) + 1
    duch = {}
    for d in realm.duchies.values():
        if d["ctrl"] is not None:
            duch[d["ctrl"]] = duch.get(d["ctrl"], 0) + 1
    for k in list(realm.kingdoms):
        if realm.extinct[k.id] or k.ruler is None:
            continue
        r = realm.chars[k.ruler]
        cu = culture_of(realm, k.id)
        towns = realm.alive_towns(k.id)
        n = len(towns)
        cap = realm.capital_of(k.id)
        place = getattr(k, "place", None) or (cap.name if cap else k.short)
        rank = RANKS.index(k.rank) if k.rank in RANKS else 2
        want = None
        if rank == 0 and n >= 5:
            want = ("chiefdom", f"unites the clans around {place}", 0)
        elif rank == 1 and duch.get(k.id, 0) >= 2 and n >= 12 and cu.era >= 1:
            want = ("duchy", f"takes the title of {'Duchess' if r.female else 'Duke'}", 80)
        elif rank == 2 and held.get(k.id, 0) >= 1 and cu.era >= 1 and n >= 25:
            want = ("kingdom", "is crowned and anointed", 200)
        elif rank == 3 and held.get(k.id, 0) >= 3 and cu.era >= 2 and n >= 70:
            want = ("empire", "proclaims an empire", 500)
        if want and k.gold >= want[2] and rng.random() < 0.3:
            new, what, cost = want
            k.gold -= cost
            from .events import book
            book(realm, k.id, "decisions", -cost, what)
            old = k.name
            k.rank = new
            k.place = place
            k.name = style_for(k, new, place)
            realm.say(f"{'**' if new in ('kingdom', 'empire') else ''}{realm.who(r)} {what}: {old} becomes "
                      f"{k.name}.{'**' if new in ('kingdom', 'empire') else ''}", minor=new == "chiefdom")
            realm.decisions.append((realm.year, k.id, f"{what}: {old} becomes {k.name}"))
            r.renown += 50 * (RANKS.index(new) + 1)
        # government reforms, opened by innovations
        g = k.gov
        nxt = None
        if g == "tribal" and cu.has("feudal") and rank >= 2:
            nxt = "clan" if k.people in (2, 3) else "feudal"
        elif g in ("feudal", "clan") and cu.has("admin") and r.attr("ste") >= 10 and rng.random() < 0.3:
            nxt = "administrative"
        if nxt and rng.random() < 0.1:
            change_gov(realm, k, nxt, f"{realm.who(r)} reforms the rule of {k.name}")
        confederate(realm, k, r)


def style_for(k, rank, place):
    from .titles import style
    if k.gov == "republic":
        return f"the Republic of {place}"
    if k.gov == "theocracy":
        return f"the Holy See of {place}"
    if rank == "tribe":
        return ["the {} Tribe", "the Kin of {}", "the {} Clans", "the {} Warband"][k.people].format(place)
    if rank == "chiefdom":
        return f"the Chiefdom of {place}"
    return style(k.people, rank, place)


def change_gov(realm, k, gov, why):
    old = k.gov
    k.gov = gov
    if GOV_LAW.get(gov):
        k.law = GOV_LAW[gov]
    place = getattr(k, "place", k.short)
    if gov in ("republic", "theocracy"):
        k.name = style_for(k, k.rank, place)
    realm.say(f"**A new order.** {why}: {old} rule gives way to {gov} rule"
              + (f", and the realm is {k.name} now" if gov in ("republic", "theocracy") else "") + ".")
    realm.decisions.append((realm.year, k.id, f"{old} to {gov}"))


def confederate(realm, k, r):
    """Tribes of one people at peace, bound by marriage or friendship, may join under the stronger chief."""
    rng = realm.rng
    if k.rank not in ("tribe", "chiefdom") or rng.random() > 0.05:
        return
    mine = len(realm.alive_towns(k.id))
    for o in realm.kingdoms:
        if o is k or realm.extinct[o.id] or o.people != k.people or o.rank not in ("tribe", "chiefdom"):
            continue
        if realm.war[k.id, o.id] or o.ruler is None:
            continue
        theirs = len(realm.alive_towns(o.id))
        if theirs >= mine or (realm.relation[k.id, o.id] < 20 and o.id not in k.allies):
            continue
        orul = realm.chars[o.ruler]
        if "ambitious" in orul.personality and rng.random() < 0.7:
            continue
        absorb(realm, k, o)
        realm.say(f"The chief of {o.name}, {orul.full_name()}, swears to {realm.who(r)}; the two peoples are "
                  f"one now under {k.name}.")
        realm.decisions.append((realm.year, k.id, f"absorbs {o.name}"))
        return


def absorb(realm, k, o):
    for t in realm.alive_towns(o.id):
        t.faction = k.id
        t.capital = False
        realm.town_moved(t)
    for i in realm.living:
        c = realm.chars[i]
        if c.realm == o.id:
            c.realm = k.id
    if o.ruler is not None:
        from .dynasty import end_reign
        end_reign(realm, o, f"swore fealty to {k.name}")
    realm.extinct[o.id] = True
    realm.war[o.id, :] = realm.war[:, o.id] = False
    o.ended = realm.year


# ---------------------------------------------------------------- reform movements
def reform_year(realm):
    """When the great of a realm disagree with how it is ruled, the pressure for reform builds."""
    rng = realm.rng
    for k in realm.kingdoms:
        if realm.extinct[k.id] or k.ruler is None:
            continue
        r = realm.chars[k.ruler]
        great = [realm.chars[i] for i in realm.member_ids(k.id) if True and realm.chars[i].adult(realm.year)
                 and (getattr(realm.chars[i], "duchies", None) or realm.chars[i].lordships
                      or realm.chars[i].profession in ("merchant", "priest") or realm.chars[i].id in k.council.values())]
        if len(great) < 3:
            continue
        agree = np.mean([agreement(realm, c, k) for c in great])
        k.sentiment = {a: float(np.mean([views(realm, c)[a] for c in great])) for a in AXES}
        k.agreement = float(agree)
        k.reform = max(0.0, k.reform + (0.45 - agree) * 0.5)
        if k.reform < 3 or rng.random() > 0.2:
            continue
        cu = culture_of(realm, k.id)
        s = k.sentiment
        target = None
        ports_share = sum(1 for t in realm.alive_towns(k.id) if t.port) / max(1, len(realm.alive_towns(k.id)))
        if (s["econ"] > 0.45 and s["succ"] < -0.1 and cu.has("republic") and k.gov != "republic"
                and ports_share >= 0.3 and k.rank in ("duchy", "kingdom")):
            target = "republic"
        elif s["faith"] > 0.45 and cu.has("theocracy") and k.gov != "theocracy":
            target = "theocracy"
        elif s["auth"] < -0.2 and k.laws["crown"] > 1:
            target = "crown"
        if target is None:
            k.reform *= 0.5
            continue
        k.reform = 0.0
        want = "a charter for the crown" if target == "crown" else f"{target} rule"
        yields = (r.legitimacy or 50) < 40 or "content" in r.personality or "kind" in r.personality
        if not yields:
            win = rng.random() < 0.45
            for t in realm.alive_towns(k.id):
                t.pop *= 1 - rng.uniform(0.02, 0.08)
            realm.say(f"**A reform war.** The great of {k.name} rise for {want}; {realm.who(r)} "
                      f"{'falls' if win else 'crushes them'}.")
            if not win:
                from .governance import add_tyranny
                add_tyranny(realm, r, 5, "crushing the reformers")
                continue
            from .dynasty import claim_score, crown, end_reign, pool
            cands = [c for c in pool(realm, k) if c.id != r.id]
            realm.kill(r, "was overthrown by the reformers", quiet=True)
            end_reign(realm, k, "overthrown by reformers")
            if target in ("republic", "theocracy"):
                change_gov(realm, k, target, f"The reformers of {k.name} remake the realm")
                cands = reform_candidates(realm, k) or cands
            elif cands:
                k.laws["crown"] = max(1, k.laws["crown"] - 2)
            if cands:
                crown(realm, k, max(cands, key=lambda c: claim_score(realm, c)), "raised up when the line failed")
            continue
        if target == "crown":
            k.laws["crown"] = max(1, k.laws["crown"] - 1)
            realm.say(f"{realm.who(r)} yields to the reformers of {k.name} and lowers the crown's authority.",
                      minor=True)
        else:
            change_gov(realm, k, target, f"{realm.who(r)} yields to the reformers of {k.name}")


def reform_candidates(realm, k):
    prof = "merchant" if k.gov == "republic" else "priest"
    return [realm.chars[i] for i in realm.member_ids(k.id) if True and realm.chars[i].adult(realm.year)
            and realm.chars[i].profession == prof]


# ---------------------------------------------------------------- culture events
def culture_events(realm):
    rng = realm.rng
    y = realm.year
    for cu in realm.cultures:
        ks = [k for k in realm.kingdoms if not realm.extinct[k.id] and k.people == cu.people and k.ruler is not None]
        if not ks:
            continue
        roll = rng.random()
        k = max(ks, key=lambda k: len(realm.alive_towns(k.id)))
        cap = realm.capital_of(k.id)
        if cap is None:
            continue
        at_war = realm.war[k.id].any()
        if roll < 0.006 and cu.golden < y and not at_war:
            cu.golden = y + int(rng.integers(20, 50))
            cu.history.append((y, "a golden age begins"))
            realm.say(f"**A golden age.** The {PEOPLES[cu.people]} flourish: songs, scholars and new ideas pour out "
                      f"of {cap.name}.")
        elif roll < 0.010 and cu.dark < y and (at_war or realm.year - realm.last_plague < 5):
            cu.dark = y + int(rng.integers(15, 40))
            cu.history.append((y, "a dark age begins"))
            realm.say(f"**A dark age.** War and pestilence close the schools of the {PEOPLES[cu.people]}; much is "
                      f"forgotten.")
        elif roll < 0.016 and k.gold > 300:
            name = str(rng.choice(GREAT_WORKS)).format(cap.name)
            if name not in k.great_works:
                k.gold -= 300
                from .events import book
                book(realm, k.id, "great works", -300, f"builds {name}")
                k.great_works.append(name)
                cap.wealth += 100
                r = realm.chars[k.ruler]
                r.legitimacy = min(100.0, (r.legitimacy or 50) + 10)
                realm.wonders.append((y, k.id, name))
                realm.say(f"**A great work.** {realm.who(r)} completes {name}.")
        elif roll < 0.022:
            heroes = [h for h in realm.heroes() if h.people == cu.people and h.renown > 300]
            if heroes:
                h = max(heroes, key=lambda h: h.renown)
                from .lineage import gain_legacy
                gain_legacy(realm, h, "warrior", 300)
                cu.history.append((y, f"the Saga of {h.name} is composed"))
                realm.say(f"The Saga of {h.full_name()} is composed among the {PEOPLES[cu.people]}, and sung in every "
                          f"hall.", minor=True)
        elif roll < 0.026:
            cu.trend["trad"] += float(rng.choice([-0.2, 0.2]))
            what = "a movement of new thinkers" if cu.trend["trad"] < 0 else "a return to the old ways"
            cu.history.append((y, what))
            realm.say(f"Among the {PEOPLES[cu.people]}, {what} takes hold.", minor=True)


def civics_year(realm):
    research_year(realm)
    trends_year(realm)
    laws_year(realm)
    armies_year(realm)
    decisions_year(realm)
    reform_year(realm)
    culture_events(realm)
