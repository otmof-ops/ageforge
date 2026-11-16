"""The real economy: work, skill, money, credit and trade, growing from barter to a world market.

WORK. Every town needs hands: farmers for its fields (fewer as ploughs and rotations improve, which frees people for
the towns), miners for its deposits, craftsmen (more with guilds, coin and materials to work), merchants for its
roads and harbours, clergy, and servants once there is coin to pay them. Working people beyond the jobs are idle or
take casual work for a pittance; jobs beyond the people (after a plague) are a labour shortage, and wages rise.

SKILL. Each town's workforce has a skill in each trade, drifting towards what its people are good at, what their
culture has learned, and what the masters living there teach: a smith of Smithing 90 lifts every forge in town. The
masters themselves earn by their skill. The steward's stewardship decides how much of the taxes reach the crown.

MONEY. Tribes barter: no coin, no lenders, no bankruptcy, only unpaid warriors. With Coinage a realm strikes gold:
coin is gold, so it cannot be printed and prices hold. Gold comes from the mines (the crown takes a royalty) and from
a surplus of trade, and wears and leaks away. A crown that spends more than it taxes borrows; when lenders will lend
no more it must raise taxes, cut its court and regiments, seize the merchants' gold in a forced loan, or default.
With Banking Houses credit is cheaper and deeper, but a default is remembered for a generation.

TRADE. Realms trade with their neighbours by land, along the coasts with Longships, across seas with Carrack Hulls
and anywhere with Galleon Yards: more between large economies and between realms that each have what the other lacks;
none in war, half under protectionist law. The exporter's surplus is settled in bullion. When one web of trade spans
three continents, the world has a market."""
import math

import numpy as np

from .data import MAGIC, PEOPLES

SECTORS = ["farm", "mine", "craft", "trade", "clergy", "service", "arcane"]
VALUE = {"farm": 1.0, "mine": 2.0, "craft": 2.2, "trade": 2.6, "clergy": 0.6, "service": 0.9, "arcane": 3.2}
IDLE_VALUE = 0.25
UNIT = 0.0075                                 # converts a worker-year of value into the sim's gold
WORKING = 0.55                                # share of people of working age and able
# what each people is good at, 0..1: humans, elves, dwarves, orcs
PEOPLE_SKILL = [
    {"farm": 0.45, "mine": 0.35, "craft": 0.40, "trade": 0.50, "clergy": 0.45, "service": 0.45, "arcane": 0.40},
    {"farm": 0.40, "mine": 0.25, "craft": 0.55, "trade": 0.40, "clergy": 0.55, "service": 0.40, "arcane": 0.70},
    {"farm": 0.30, "mine": 0.60, "craft": 0.60, "trade": 0.45, "clergy": 0.40, "service": 0.35, "arcane": 0.45},
    {"farm": 0.35, "mine": 0.40, "craft": 0.30, "trade": 0.25, "clergy": 0.30, "service": 0.30, "arcane": 0.30},
]
INNOV_SKILL = {
    "Hunting Grounds": {"farm": 0.03}, "Ironworking": {"craft": 0.05, "mine": 0.05},
    "Two-Field Rotation": {"farm": 0.06}, "Coinage": {"trade": 0.06, "service": 0.05},
    "Monastery Schools": {s: 0.03 for s in SECTORS}, "Steel Forging": {"craft": 0.07},
    "Sea Charts": {"trade": 0.04}, "Three-Field Rotation": {"farm": 0.07}, "Guild Charters": {"craft": 0.10, "trade": 0.06},
    "Universities": {s: 0.05 for s in SECTORS}, "Silversteel Craft": {"craft": 0.05, "mine": 0.05},
    "Heavy Plough": {"farm": 0.08}, "Banking Houses": {"trade": 0.10}, "Printed Sagas": {"clergy": 0.04, "trade": 0.03},
    "Adamant Forging": {"craft": 0.05}, "Royal Chancery": {"service": 0.05}, "Bureaucracy": {"service": 0.05},
}
LABOUR_SAVING = {"Two-Field Rotation": 0.04, "Three-Field Rotation": 0.05, "Heavy Plough": 0.06}
PROF_SECTOR = {"smith": "craft", "enchanter": "craft", "alchemist": "craft", "shipwright": "craft",
               "merchant": "trade", "captain": "trade", "priest": "clergy", "tamer": "farm"}
PROF_SKILL = {"smith": "Smithing", "enchanter": "Enchanting", "alchemist": "Alchemy", "shipwright": "Seamanship",
              "merchant": "Mercantile", "captain": "Navigation", "priest": "Speech", "tamer": "Beast Taming"}
SECTOR_BUILDING = {"farm": "Mill and Granary", "mine": "Mine Works", "craft": "Workshops", "trade": "Market Hall",
                   "clergy": "School", "arcane": "Arcane College"}
MINERAL = {"stone", "copper", "iron", "gold", "silver", "gems", "mithril", "adamantine ore", "salt",
           "dragonbone fossils", "mana crystals"}
MATERIAL = {"iron", "copper", "timber", "mithril", "adamantine ore", "ancient wood", "furs", "silk", "ivory", "amber"}
BULLION = {"gold": 0.6, "silver": 0.25}
VELOCITY = 4.0
STAGES = ["barter", "coin", "credit"]


def stage(cu):
    inn = cu.innovations
    return 2 if "Banking Houses" in inn else 1 if "Coinage" in inn else 0


def sea_reach(cu):
    inn = cu.innovations
    return 3 if "Galleon Yards" in inn else 2 if "Carrack Hulls" in inn or "Sea Charts" in inn else \
        1 if "Longships" in inn else 0


# ---------------------------------------------------------------- work and skill
_TARGETS = {}


def culture_targets(cu, people):
    """What a people's craft and learning make of each trade (cached until the culture learns something new)."""
    key = (id(cu), len(cu.innovations), people)
    v = _TARGETS.get(key)
    if v is None:
        inn = cu.innovations
        v = _TARGETS[key] = {sct: PEOPLE_SKILL[people][sct] + sum(fx.get(sct, 0.0) for n, fx in INNOV_SKILL.items()
                                                                     if n in inn) for sct in SECTORS}
    return v
def master_skill(c):
    p = c.profession
    return c.skills.get(PROF_SKILL[p], 0) if p in PROF_SKILL else max(0, c.tier()) * 10


def master_sector(c):
    return PROF_SECTOR.get(c.profession) or "arcane"


def masters_by_town(realm):
    """The masters of each town (tradesmen by profession, and casters of the third tier and above), and the gifted."""
    out, gifts = {}, {}
    for i in realm.living:
        c = realm.chars[i]
        if c.home is None:
            continue
        if c.profession in PROF_SECTOR or (not c.profession and c.tier() >= 3):
            out.setdefault(c.home, []).append(c)
        g = getattr(c, "gift", None)
        if g:
            gifts.setdefault(c.home, set()).add(g)
    realm.town_gifts = gifts
    return out


def labour_year(realm, alive, terr_wealth):
    """Jobs, workers, skill and output for every town; sets s.income (in the realm's coin) and s.wealth."""
    from . import civics
    from .resources import RESOURCES
    masters = masters_by_town(realm)
    y = realm.year
    realm.econ_jobs = {"jobless": 0.0, "working": 0.0, "short": 0.0}
    for idx, s in enumerate(alive):
        if not s.alive:
            continue
        k = realm.kingdoms[s.faction]
        cu = realm.cultures[s.culture]
        inn = cu.innovations
        st = stage(cu)
        res = getattr(s, "res", {})
        working = WORKING * s.pop
        soldiers = min(working * 0.08, 0.25 * getattr(s, "garrison", 0.0))   # the men under arms in the town
        free = working - soldiers
        saving = sum(v for n, v in LABOUR_SAVING.items() if n in inn)
        guild = "Guild Charters" in inn
        mats = sum(n for r, n in res.items() if r in MATERIAL)
        mins = sum(n for r, n in res.items() if r in MINERAL)
        b = getattr(s, "bld", {})
        jobs = {
            "farm": 0.78 * WORKING * s.pop * min(1.05, max(0.9, getattr(s, "food_prod", s.pop) / max(1.0, s.pop)))
            * (1 - saving) * (1 - 0.02 * b.get("Mill and Granary", 0)),
            "mine": 25.0 * mins + (30.0 * b.get("Mine Works", 0) if mins else 0.0),
            "craft": s.pop * (0.03 + 0.04 * guild + 0.02 * (st >= 1) + 0.015 * b.get("Workshops", 0)) + 25.0 * mats,
            "trade": 12.0 * getattr(s, "trade", 0.0) + 40.0 * s.port * sea_reach(cu) + 0.02 * s.pop * st
            + 0.01 * s.pop * b.get("Market Hall", 0) + 40.0 * b.get("Harbour", 0),
            "clergy": 0.035 * s.pop,
            "service": s.pop * 0.05 * st,
            "arcane": s.pop * 0.004 * MAGIC[s.culture] * (1 + min(2.0, s.magic)) * (1 + 0.5 * b.get("Arcane College", 0)),
        }
        # the bound dead: a necromancer's realm works its fields and mines with the dead, unpaid and tireless
        bound = getattr(k, "bound_share", 0.0) * working
        b_farm = min(bound, 0.6 * jobs["farm"])
        b_mine = min(bound - b_farm, 0.6 * jobs["mine"])
        jobs["farm"] -= b_farm
        jobs["mine"] -= b_mine
        s.bound_work = b_farm + b_mine
        demand = sum(jobs.values())
        if demand <= free:
            filled = dict(jobs)
            idle = free - demand
            short = 0.0
        else:                                                          # the fields are worked first
            filled = {"farm": min(jobs["farm"], free)}
            rest = free - filled["farm"]
            other = demand - jobs["farm"]
            for sct in SECTORS[1:]:
                filled[sct] = jobs[sct] * (rest / other if other > 0 else 0.0)
            idle = 0.0
            short = demand / max(1.0, free) - 1
        s.jobs, s.jobless = filled, idle / max(1.0, working)
        s.farm_fill = filled["farm"] / max(1.0, jobs["farm"])
        s.labour_short = short
        # skill: drifting towards the people's gift, the culture's learning and the masters in town
        sk = getattr(s, "skill", None)
        if sk is None:
            sk = s.skill = dict(PEOPLE_SKILL[s.culture])
        teach = {}
        for c in masters.get(s.id, ()):
            sc = master_sector(c)
            teach[sc] = teach.get(sc, 0.0) + master_skill(c) / 100 * 0.12 * (2 if getattr(c, "gift", None) == "Forgeheart" else 1)
        base = culture_targets(cu, s.culture)
        boost = getattr(k, "sector_boost", {})
        school = 0.03 * b.get("School", 0)
        for sct in SECTORS:
            target = base[sct] + school + 0.04 * b.get(SECTOR_BUILDING.get(sct), 0) + boost.get(sct, 0.0)
            target = min(1.0, target + min(0.3, teach.get(sct, 0.0)))
            sk[sct] += 0.06 * (target - sk[sct])
        # output, in real terms
        harvest = getattr(s, "food_prod", s.pop) / max(1.0, s.pop)
        gifted = realm.town_gifts.get(s.id, ())
        safe = 1 - min(0.4, 0.25 * getattr(s, "threat", 0.0))          # fields and roads near a lair go untended
        arc = filled["arcane"] / max(1.0, s.pop) * (0.6 + 0.8 * sk["arcane"])
        s.arcane = arc
        mult = {"farm": min(1.6, 0.5 + 0.5 * harvest) * (1 + 0.08 * b.get("Mill and Granary", 0)) * safe
                * (1.15 if "Greenhand" in gifted else 1.0),
                "mine": (1 + 0.12 * b.get("Mine Works", 0)) * safe,
                "craft": 1 + min(0.2, 25 * arc),                       # light to work by, tools that mend themselves
                "trade": (1 + 0.1 * min(10.0, getattr(s, "trade", 0.0))) * (1 + 0.08 * b.get("Market Hall", 0)) * safe
                * (1.1 if "Appraiser" in gifted else 1.0)}
        val = {sct: filled[sct] * VALUE[sct] * mult.get(sct, 1.0) * (0.6 + 0.8 * sk[sct]) * UNIT for sct in SECTORS}
        val["bound"] = (b_farm * VALUE["farm"] + b_mine * VALUE["mine"]) * 0.84 * UNIT * safe
        value = sum(val.values()) / UNIT + idle * IDLE_VALUE
        s.labour_value = value * UNIT
        real = value * UNIT + 0.3 * terr_wealth[idx] + getattr(s, "trade_gain", 0.0)
        real *= 0.6 if getattr(s, "occupied_by", None) is not None else 1.0
        s.real_income = real
        s.income = real
        s.wealth = s.wealth * 0.96 + real
        if s.wealth > 80 + 50 * s.walls and s.walls < civics.walls_cap(realm, s.faction) and realm.rng.random() < 0.1:
            s.wealth -= 60 + 40 * s.walls
            s.walls += 1
        # the mood of the poor: work and want
        classes(realm, s, k, filled, idle, soldiers, val, real - s.labour_value)
        s.econ_mood = -70 * s.jobless + 25 * min(0.4, short) \
            - 40 * (s.cls["lower"][2] - 0.45)
        realm.econ_jobs["jobless"] += idle
        realm.econ_jobs["working"] += working
        realm.econ_jobs["short"] += max(0.0, demand - free)
    # the idle drift to towns that need hands
    for f in range(realm.npol):
        towns = realm.alive_towns(f)
        if len(towns) < 2:
            continue
        fed = [t for t in towns if getattr(t, "food_prod", 0) > 1.1 * t.pop]   # only where there is bread to spare
        if not fed:
            continue
        need = max(fed, key=lambda t: getattr(t, "labour_short", 0.0))
        if getattr(need, "labour_short", 0.0) <= 0.05:
            continue
        room = 0.5 * (need.food_prod - 1.05 * need.pop)
        for t in towns:
            if room <= 0:
                break
            if t is not need and getattr(t, "jobless", 0.0) > 0.08 and t.landmass == need.landmass:
                n = min(room, t.pop * t.jobless * 0.15)
                t.pop -= n
                need.pop += n
                room -= n
    pay_masters(realm, masters)


def pay_masters(realm, masters):
    """Masters earn by their skill: a great one earns many times a journeyman. (Casters earn by their scrolls.)"""
    for cs in masters.values():
        for c in cs:
            if c.profession not in PROF_SKILL:
                continue
            k = realm.kingdoms[c.realm]
            sk = c.skills.get(PROF_SKILL[c.profession], 0)
            st = stage(realm.cultures[k.people]) if not realm.extinct[c.realm] else 0
            c.gold += (0.5 + st) * (sk / 40) ** 1.6 * (2 if getattr(c, "gift", None) == "Appraiser" else 1)


# ---------------------------------------------------------------- classes, assets and poverty
CLASSES = ["lower", "middle", "upper"]
RENT = {"tribal": 0.15, "clan": 0.2, "feudal": 0.4, "republic": 0.3, "theocracy": 0.35, "administrative": 0.3}
TAX_SHARE = {"lower": 0.8, "middle": 1.0, "upper": 0.4}
SAVE = {"lower": 0.02, "middle": 0.12, "upper": 0.2}
SIGMA = {"lower": 0.5, "middle": 0.6, "upper": 0.8}
SUBSIST = 0.0030                      # a year's bread, roof and fire for one person, in the sim's gold
HOUSE = 0.02                          # the worth of a place to live for one person


def poor_share(line, mean, sigma):
    """The share of a class below the poverty line, its incomes spread log-normally around their mean."""
    if mean <= 0:
        return 1.0
    z = (math.log(line / mean) + sigma * sigma / 2) / sigma
    return 0.5 * (1 + math.erf(z / math.sqrt(2)))


def classes(realm, s, k, filled, idle, soldiers, val, rest):
    """Who earns what in a town, who owns what, and how many fall below the line."""
    gov = getattr(k, "gov", "tribal")
    R = RENT.get(gov, 0.3) * val["farm"]
    vf = val["farm"] - R
    inc = {
        "lower": vf * 0.8 + val["mine"] * 0.55 + val["craft"] * 0.3 + val["trade"] * 0.1 + val["service"] * 0.6
        + idle * IDLE_VALUE * UNIT + soldiers * 0.6 * UNIT,
        "middle": vf * 0.2 + R * 0.1 + val["craft"] * 0.6 + val["trade"] * 0.6 + val["clergy"] + val["service"] * 0.4
        + rest * 0.4 + val["arcane"] * 0.6,
        "upper": R * 0.9 + val["mine"] * 0.45 + val["craft"] * 0.1 + val["trade"] * 0.3 + rest * 0.6
        + val["arcane"] * 0.4 + val["bound"],
    }
    heads = {
        "lower": filled["farm"] * 0.9 + filled["mine"] + filled["craft"] * 0.4 + filled["trade"] * 0.1
        + filled["service"] * 0.6 + idle + soldiers,
        "middle": filled["farm"] * 0.08 + filled["craft"] * 0.55 + filled["trade"] * 0.7 + filled["clergy"]
        + filled["service"] * 0.4 + filled["arcane"] * 0.8,
        "upper": filled["farm"] * 0.02 + filled["craft"] * 0.05 + filled["trade"] * 0.2 + s.pop * 0.01
        + filled["arcane"] * 0.2,
    }
    th = sum(heads.values()) or 1.0
    tax = getattr(k, "tax_now", 0.25)
    if gov in ("republic", "administrative"):
        tshare = dict(TAX_SHARE, upper=0.8)
    else:
        tshare = TAX_SHARE
    housing = getattr(s, "housing", None)
    if housing is None:
        housing = s.housing = s.pop * 1.1
    if housing < s.pop * 1.15:                                         # private builders put up what they can
        s.housing = housing = housing + 0.015 * housing
    crowd = max(0.0, s.pop / max(1.0, housing) - 1)
    s.crowded = crowd
    food = min(1.3, max(0.5, getattr(s, "food_prod", s.pop) / max(1.0, s.pop)))
    line = SUBSIST * food ** -0.7 * (1 + 0.6 * crowd)
    if getattr(s, "relief", 0) > realm.year:
        line *= 0.9                                                   # alms and poor relief
    stock = getattr(s, "stock", None)
    if stock is None:
        stock = s.stock = {c: 0.0 for c in CLASSES}
    communal = gov in ("tribal", "clan")
    land = R / 0.06
    land_split = {"lower": 0.5 if communal else 0.0, "middle": 0.15, "upper": 0.35 if communal else 0.85}
    coin = getattr(s, "coin", 0.0)
    coin_split = {"lower": 0.10, "middle": 0.35, "upper": 0.55}
    house_split = {"lower": 0.3, "middle": 0.35, "upper": 0.35}
    out = {}
    for c in CLASSES:
        people = heads[c] / th * s.pop
        net = inc[c] * (1 - tax * tshare[c])
        per = net / max(1.0, people)
        stock[c] = stock[c] * 0.95 + SAVE[c] * net
        assets = {"land": land * land_split[c], "housing": housing * HOUSE * house_split[c], "stock": stock[c],
                  "coin": coin * coin_split[c]}
        out[c] = (people, per, poor_share(line, per, SIGMA[c]), assets)
    s.cls = out
    s.poverty = sum(v[0] * v[2] for v in out.values()) / max(1.0, s.pop)


# ---------------------------------------------------------------- trade between realms
def land_neighbours(realm):
    F = realm.owner_fac
    pairs = set()
    n = realm.npol
    for a, b in ((F[:, :-1], F[:, 1:]), (F[:-1], F[1:])):
        m = (a != b) & (a >= 0) & (b >= 0)
        if m.any():
            code = np.unique(np.minimum(a[m], b[m]).astype(np.int64) * n + np.maximum(a[m], b[m]))
            pairs.update((int(c // n), int(c % n)) for c in code)
    return pairs


def trade_year(realm):
    from .events import book
    from .resources import RESOURCES
    ks = [k for k in realm.kingdoms if not realm.extinct[k.id] and k.ruler is not None]
    gdp, goods, craft, ports, reach = {}, {}, {}, {}, {}
    for k in ks:
        towns = realm.alive_towns(k.id)
        gdp[k.id] = sum(getattr(t, "real_income", 0.0) - getattr(t, "trade_gain", 0.0) for t in towns)
        goods[k.id] = {r for r in getattr(k, "resources", {}) if RESOURCES[r][0] in ("luxury", "strategic")}
        craft[k.id] = float(np.mean([t.skill["craft"] for t in towns if hasattr(t, "skill")] or [0.3]))
        pt = [t for t in towns if t.port]
        ports[k.id] = (np.mean([t.pos[0] for t in pt]), np.mean([t.pos[1] for t in pt])) if pt else None
        reach[k.id] = sea_reach(realm.cultures[k.people])
        for t in towns:
            t.trade_gain = 0.0
    land = land_neighbours(realm)
    routes = {}
    diag = math.hypot(realm.H, realm.W)
    for i, a in enumerate(ks):
        for b in ks[i + 1:]:
            ia, ib = a.id, b.id
            if realm.war[ia, ib] or gdp[ia] <= 0 or gdp[ib] <= 0:
                continue
            key = (min(ia, ib), max(ia, ib))
            if key in land:
                way, carry = "land", 1.0
            elif ports[ia] is not None and ports[ib] is not None:
                d = math.hypot(ports[ia][0] - ports[ib][0], ports[ia][1] - ports[ib][1]) / diag
                r = min(reach[ia], reach[ib])
                limit = {0: 0.0, 1: 0.12, 2: 0.4, 3: 2.0}[r]
                if d > limit:
                    continue
                way, carry = ("ocean" if d > 0.25 else "sea"), 0.8 - 0.6 * d
            else:
                continue
            for x in (ia, ib):                                           # raids and calamities frighten trade away
                if getattr(realm.kingdoms[x], "trade_shock", 0) > realm.year:
                    carry *= 0.6
            a_has, b_has = len(goods[ia] - goods[ib]), len(goods[ib] - goods[ia])
            comp = min(4.0, 1 + 0.4 * (a_has + b_has) + abs(craft[ia] - craft[ib]))
            prot = sum(realm.kingdoms[x].laws.get("trade") == "protectionist" for x in (ia, ib))
            friendly = 0.5 if realm.relation[ia, ib] < -50 else 1.0
            T = 0.012 * math.sqrt(gdp[ia] * gdp[ib]) * comp * carry * friendly * 0.5 ** prot
            if T < 0.5:
                continue
            offer_a = 1 + a_has + 3 * craft[ia] + 0.3 * getattr(a, "top_scroll", 0)    # scrolls sell abroad
            offer_b = 1 + b_has + 3 * craft[ib] + 0.3 * getattr(b, "top_scroll", 0)
            share = offer_a / (offer_a + offer_b)
            routes[key] = (T, way, share if key[0] == ia else 1 - share)
    realm.trade_routes = routes
    # gains, customs and the bullion that settles the balance
    for k in ks:
        k.trade_val, k.trade_net, k.partners = 0.0, 0.0, 0
    for (ia, ib), (T, way, share_lo) in routes.items():
        for kid, sh in ((ia, share_lo), (ib, 1 - share_lo)):
            k = realm.kingdoms[kid]
            k.trade_val += T
            k.trade_net += T * (2 * sh - 1)
            k.partners += 1
    for k in ks:
        if not k.trade_val:
            continue
        towns = [t for t in realm.alive_towns(k.id) if t.port or getattr(t, "trade", 0) > 0]
        weight = sum(1 + 2 * t.port + getattr(t, "trade", 0) for t in towns) or 1
        for t in towns:                                               # both sides gain from trade
            t.trade_gain = 0.25 * k.trade_val * (1 + 2 * t.port + getattr(t, "trade", 0)) / weight
        tariff = 0.10 if k.laws.get("trade") == "protectionist" else 0.03
        if stage(realm.cultures[k.people]) >= 1:
            duty = tariff * k.trade_val
            k.gold += duty
            book(realm, k.id, "customs", duty)
    world_market(realm, routes)


def world_market(realm, routes):
    """Watch the webs of trade join up: the first crossing of an ocean, and the first market spanning the world."""
    seen = getattr(realm, "trade_firsts", None)
    if seen is None:
        seen = realm.trade_firsts = {}
    for (ia, ib), (T, way, _) in routes.items():
        if way == "ocean" and "ocean" not in seen:
            seen["ocean"] = realm.year
            realm.say(f"**Across the ocean.** Ships of the {realm.pname(ia)} and the {realm.pname(ib)} open the first "
                      f"trade across open ocean.")
    if "world" in seen:
        return
    parent = {}

    def find(x):
        while parent.get(x, x) != x:
            x = parent[x]
        return x
    for ia, ib in routes:
        parent[find(ia)] = find(ib)
    comps = {}
    for k in realm.kingdoms:
        if realm.extinct[k.id]:
            continue
        comps.setdefault(find(k.id), set()).update(t.landmass for t in realm.alive_towns(k.id))
    big = [c for c in comps.values() if len({m for m in c if realm.lands[m]["kind"] == "continent"}) >= 3]
    if big:
        seen["world"] = realm.year
        realm.say("**A world market.** Goods and gold now pass along one web of trade across three continents; a "
                  "price set in one harbour is felt in another a sea away.")


# ---------------------------------------------------------------- gold and credit
def money_year(realm):
    """Gold coin, the mines and the balance of trade, lending, and the crown's choices when the lenders say no."""
    from .events import book
    rng = realm.rng
    stats = getattr(realm, "econ_stats", None)
    if stats is None:
        stats = realm.econ_stats = {"defaults": [], "forced": [], "mined": 0.0}
    for k in realm.kingdoms:
        if realm.extinct[k.id] or k.ruler is None:
            continue
        r = realm.chars[k.ruler]
        cu = realm.cultures[k.people]
        st = stage(cu)
        towns = realm.alive_towns(k.id)
        if not towns:                                                  # a court in exile: no land to tax, no lenders
            continue
        k.income = sum(getattr(t, "income", 0.0) for t in towns if getattr(t, "occupied_by", None) is None)
        k.real_gdp = sum(getattr(t, "real_income", 0.0) for t in towns)
        steward = realm.chars.get(k.council.get("steward")) if getattr(k, "council", None) else None
        ste = steward.attr("ste") if steward is not None else r.attr("ste") * 0.7
        mint = getattr(k, "mint_skill", 0.0)                           # a good Master of the Mint: honest weights, full royalties
        k.collect = min(1.2, 0.75 + 0.02 * ste + 0.05 * min(1.0, mint))
        if st == 0:                                                     # barter: no coin, no lenders
            floor = -max(20.0, 0.5 * k.income)
            if k.gold < floor:
                k.gold = floor
                r.legitimacy = max(0.0, (r.legitimacy or 50) - 2)
                if rng.random() < 0.1:
                    realm.say(f"The warriors of {k.name} go unpaid; {realm.who(r)} has nothing left to give them.",
                              minor=True)
            continue
        if getattr(k, "defaults", None) is None:                        # coin is first struck
            k.credit, k.defaults = min(getattr(k, "credit", 100.0), 45.0 + 10 * st), 0
            realm.say(f"**Coin.** The mint of {k.name} strikes its first gold.", minor=True)
        # ---- gold in the towns: hoards brought to the mint, the mines, wear and loss, the balance of trade
        mined = 0.0
        wear = 0.996 + 0.002 * min(1.0, mint)
        for t in towns:
            if getattr(t, "coin", None) is None:
                t.coin = getattr(t, "real_income", 0.0) / VELOCITY
            m = sum(BULLION.get(rs, 0) * n for rs, n in getattr(t, "res", {}).items()) \
                * (0.6 + 0.8 * getattr(t, "skill", PEOPLE_SKILL[t.culture])["mine"])
            t.coin = t.coin * wear + m * 0.9
            mined += m
        settle = 0.1 * getattr(k, "trade_net", 0.0)
        ports = [t for t in towns if t.port or getattr(t, "trade", 0) > 0] or towns
        for t in ports:
            t.coin = max(0.0, t.coin + settle / len(ports))
        k.money = sum(t.coin for t in towns)
        royalty = 0.1 * mined                                          # the crown's tenth of what the mines bring up
        k.gold += royalty
        stats["mined"] += mined
        book(realm, k.id, "mine royalties", royalty)
        # ---- credit
        debt = max(0.0, -k.gold)
        revenue = max(1.0, k.income * getattr(k, "tax_now", 0.3) * k.collect)
        k.credit = max(0.0, min(100.0, k.credit + 1.0 - 0.8 * max(0.0, debt / revenue - 3)))
        base = 0.06 if st == 2 else 0.12
        k.rate = min(0.25, base + 0.02 * debt / revenue + max(0.0, 50 - k.credit) / 250)
        # what lenders will carry, and what unpaid soldiers and suppliers will bear when no lender will
        capacity = max(0.5 * revenue, revenue * (4.0 if st == 2 else 2.0) * k.credit / 60)
        if debt:
            interest = min(debt, capacity) * k.rate                      # arrears beyond what was lent bear no interest
            k.gold -= interest
            book(realm, k.id, "interest", -interest)
        if debt > capacity:
            crisis(realm, k, r, debt, capacity, revenue, ste, st, stats)


def crisis(realm, k, r, debt, capacity, revenue, ste, st, stats):
    """The lenders will lend no more. What the ruler does depends on who the ruler is."""
    from .economy import austerity, bankrupt
    from .events import book
    p = r.personality
    gap = debt - capacity
    honest = ("honest" in p) + ("just" in p) + (ste >= 12)
    venal = ("greedy" in p) + ("deceitful" in p) + ("arbitrary" in p) + ("cruel" in p)
    # 1. raise the taxes, if they are not already crushing
    if getattr(k, "tax_mult", 1.0) < 1.4:
        k.tax_mult = getattr(k, "tax_mult", 1.0) + 0.1
        for t in realm.alive_towns(k.id):
            if hasattr(t, "mood"):
                t.mood["peasants"] -= 6
                t.mood["burghers"] -= 4
        realm.say(f"{realm.who(r)} raises new taxes to pay the lenders of {k.name}.", minor=True)
    # 2. a forced loan: take the merchants' gold
    if venal > honest and realm.rng.random() < 0.6 and realm.year - getattr(k, "last_forced", -99) >= 5:
        k.last_forced = realm.year
        towns = [t for t in realm.alive_towns(k.id) if getattr(t, "coin", 0) > 0]
        have = sum(t.coin for t in towns)
        take = min(gap * 0.8, 0.3 * have)
        if take > 0:
            for t in towns:
                t.coin -= take * t.coin / have
                if hasattr(t, "mood"):
                    t.mood["burghers"] -= 20
            k.gold += take
            k.credit = max(0.0, k.credit - 10)
            book(realm, k.id, "forced loans", take)
            stats["forced"].append((realm.year, k.id))
            realm.say(f"**A forced loan in {k.name}.** {realm.who(r)} seizes {take:,.0f} gold from the merchants "
                      f"and guilds, against promises no one believes.", minor=take < revenue)
            return
    austerity(realm, k)
    r.legitimacy = max(0.0, (r.legitimacy or 50) - 1)                 # soldiers and suppliers go unpaid
    if max(0.0, -k.gold) > max(capacity * 1.5, revenue) and realm.year - getattr(k, "last_default", -99) >= 10:
        k.last_default = realm.year
        stats["defaults"].append((realm.year, k.id))
        k.credit = max(0.0, k.credit - 40)
        k.defaults += 1
        bankrupt(realm, k)


# ---------------------------------------------------------------- reporting
def net_worth(realm, c):
    """Everything a person owns: coin, the worth of their gear and relics, and their share of the land and houses of
    the lordships they hold."""
    k = realm.kingdoms[c.realm]
    worth = max(0.0, c.gold) + sum(getattr(it, "value", 0) for it in c.items) * 0.01
    for tid in getattr(c, "lordships", ()):
        t = realm.by_id.get(tid)
        if t is not None and t.alive and hasattr(t, "cls"):
            a = t.cls["upper"][3]
            worth += 0.25 * (a["land"] + a["housing"])
    if c.ruled and c.ruled[2] is None:
        worth += 0.05 * max(0.0, k.gold)
    return worth


def econ_rows(realm):
    rows = []
    for k in sorted((k for k in realm.kingdoms if not realm.extinct[k.id]), key=lambda k: -getattr(k, "real_gdp", 0)):
        cu = realm.cultures[k.people]
        towns = realm.alive_towns(k.id)
        work = sum(WORKING * t.pop for t in towns) or 1
        jobless = sum(getattr(t, "jobless", 0) * WORKING * t.pop for t in towns) / work
        rows.append((k.name[:38], STAGES[stage(cu)], f"{getattr(k, 'real_gdp', 0):,.0f}", f"{getattr(k, 'money', 0):,.0f}" if getattr(k, "money", None) else "-", f"{max(0, -k.gold):,.0f}", f"{getattr(k, 'rate', 0):.0%}",
                     f"{getattr(k, 'credit', 0):.0f}" if getattr(k, "money", None) else "-", f"{jobless:.0%}",
                     f"{getattr(k, 'econ', {}).get('poverty', 0):.0%}",
                     f"{getattr(k, 'econ', {}).get('classes', {}).get('middle', {}).get('people', 0) / max(1, sum(t.pop for t in towns)):.0%}",
                     str(getattr(k, "partners", 0)), str(getattr(k, "top_scroll", 0)),
                     " ".join(f"{n[0]}{v:.0f}" for n, v in getattr(k, "parts", {}).items() if v >= 1) or "-",
                     f"{getattr(k, 'bound_share', 0):.0%}"))
    return rows
