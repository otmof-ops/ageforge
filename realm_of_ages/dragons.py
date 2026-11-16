"""Dragons at home: where they lair, why they would ever leave, and the bargains made with them.

A dragon stays where it is comfortable. It leaves only for a reason nature would understand:
- hunger: the big game within a day's flight of the lair has collapsed, so it follows the herds;
- wounds: beaten half to death, it flees to the remotest peaks to heal;
- a rival: a greater dragon settles too near, and the lesser gives way;
and never willingly while it sits on a great hoard.

TRIBUTE. A realm whose towns lie within a dragon's reach may pay it: gold every year into the hoard, and the dragon
spares that realm's towns. Whether a ruler pays is their nature and their judgement: the cautious and the clever pay
when the danger is real; the brave, the proud and the wrathful refuse; the lords of a martial people think less of a
ruler who bows to a wyrm. A tribute missed, or refused by a new ruler, brings the dragon's wrath on that realm for
years. A slain dragon frees its tributaries.

THE DRAGON ITSELF. A dragon's strength is its age and its size. Wyrmlings (under fifty) stay hidden in the peaks;
the young grow towards their strength; an adult (from a hundred and fifty years) is a raid boss of the seventh tier,
beyond any single hero; elders grow to twice that and the ancients to four times. Its scales turn aside the blows of
lesser fighters, and it fights best in its own lair. Each has a temper: most are reclusive, and use the towns only
for food when the game runs thin; some are territorial; a few are aggressive, raid for gold and blood, and fight to
the death. The rest, when a fight turns against them, take wing: a dragon's evasion is high, and only a blow great
enough to pin it, or a dragon too proud to fly, ends with a dragon dead. So dragon-slaying is rare.

The year's bookkeeping is in dragons_year: strength from age and size, healing, settling, tribute, and the news of
dragons that fled."""
import math

import numpy as np

from .data import MONSTER

REACH = MONSTER["dragon"][2]
ADULT_POWER = 6000.0                                                   # a full-grown dragon of middling size
TEMPERS = [("reclusive", 0.6), ("territorial", 0.25), ("aggressive", 0.15)]
FLEE_AT = {"reclusive": 0.5, "territorial": 0.3, "aggressive": 0.0}     # share of its strength at which it flies
EVASION = {"reclusive": 0.9, "territorial": 0.8, "aggressive": 0.0}
RAID_MULT = {"reclusive": 0.3, "territorial": 0.8, "aggressive": 1.6}
STAGES = [(50, "wyrmling"), (150, "young"), (400, "adult"), (800, "elder"), (10 ** 6, "ancient")]


def age_of(m, year):
    return year - m.born


def stage(m, year):
    a = age_of(m, year)
    return next(n for lim, n in STAGES if a < lim)


def curve(age):
    """A dragon's strength against its age, as a share of a full-grown adult's."""
    if age < 50:
        return 0.05 + 0.15 * age / 50
    if age < 150:
        return 0.2 + 0.4 * (age - 50) / 100
    if age < 400:
        return 0.6 + 0.4 * (age - 150) / 250
    if age < 800:
        return 1.0 + (age - 400) / 400
    return min(4.0, 2.0 + (age - 800) / 350)


def target_power(m, year):
    return ADULT_POWER * m.size * curve(age_of(m, year)) * (1.3 if "elder" in m.traits else 1.0)


def hidden(m, year):
    return m.kind == "dragon" and getattr(m, "size", None) is not None and age_of(m, year) < 50


def init_dragon(realm, m):
    """Size and temper when first met; a dragon that comes down from the peaks is already grown."""
    rng = realm.rng
    m.size = float(rng.uniform(0.75, 1.35))
    r = rng.random()
    m.temper = next(t for t, p in [("reclusive", 0.6), ("territorial", 0.85), ("aggressive", 1.0)] if r < p)
    if not m.parents and realm.year - m.born < 5:
        m.born = realm.year - int(rng.integers(150, 450))                # it came down out of the peaks full-grown
    m.adult_base = ADULT_POWER * m.size
    m.base = target_power(m, realm.year)
    m.__dict__["_power"] = m.base
    m.__dict__.pop("power", None)


def flight(m, old, value):
    """Called on every blow to a dragon. Below the line its temper sets, it flies, unless the blow pins it."""
    temper = getattr(m, "temper", None)
    if temper is None or value > FLEE_AT[temper] * m.base:
        return value
    from .characters import CTX
    realm = CTX.get("realm")
    rng = realm.rng if realm is not None else np.random.default_rng()
    pinned = (old - value) > 0.6 * m.base                               # a blow too great to fly from
    evade = EVASION[temper] * (0.4 if pinned else 1.0)
    if rng.random() < evade:
        m.fled = realm.year if realm is not None else 0
        return max(value, 0.15 * m.base)
    return value


def dist(a, b):
    return math.hypot(a[0] - b[0], a[1] - b[1])


def game_near(realm, pos, r=3):
    """The big game (grazers and browsers of fair size) within a day's flight of a place."""
    eco = realm.eco
    big = getattr(realm, "_big_game", None)
    if big is None:
        big = realm._big_game = [i for i, sp in enumerate(eco.species)
                                 if sp["troph"] == 1 and sp["size"] >= 0.8 and sp["size"] < 8]
    cy, cx = min(eco.eh - 1, pos[0] // 6), min(eco.ew - 1, pos[1] // 6)
    y0, y1, x0, x1 = max(0, cy - r), cy + r + 1, max(0, cx - r), cx + r + 1
    return float(sum(eco.pop[i, y0:y1, x0:x1].sum() * eco.species[i]["size"] for i in big))


def dragons_year(realm):
    dragons = [m for m in realm.monsters if m.alive and m.kind == "dragon"]
    for m in dragons:
        if getattr(m, "size", None) is None:
            init_dragon(realm, m)
        m.base = target_power(m, realm.year)
        heal = 0.12 * (m.base - m.power) if m.power < m.base else m.base - m.power
        m.__dict__["_power"] = m.power + heal                            # healing is not a blow
        if getattr(m, "fled", None) == realm.year - 1 or getattr(m, "fled", None) == realm.year:
            if getattr(m, "fled_told", None) != m.fled:
                m.fled_told = m.fled
                realm.say(f"{m.name}, torn and bleeding, breaks off the fight and takes wing; it will remember who "
                          f"hurt it.", minor=stage(m, realm.year) not in ("adult", "elder", "ancient"))
    for m in dragons:
        if not hidden(m, realm.year):
            settle(realm, m, dragons)
    tribute_year(realm, dragons)


# ---------------------------------------------------------------- staying and leaving
def settle(realm, m, dragons):
    rng = realm.rng
    game = game_near(realm, m.pos)
    if getattr(m, "game0", None) is None:
        m.game0 = max(1.0, game)
    h = getattr(m, "hungry", 0)
    m.hungry = h + 1 if game < 0.35 * m.game0 else max(0, h - 1)
    m.settled = getattr(m, "settled", m.born)
    reason = None
    if m.power < 0.4 * m.base:
        reason = "wounded"
    elif realm.year - m.settled < 20:
        return                                                         # a new lair is kept a while
    elif any(o is not m and o.power > 1.3 * m.power and dist(o.pos, m.pos) < 20
             and getattr(o, "settled", o.born) > m.settled for o in dragons):
        reason = "rival"                                               # a greater dragon has come into its land
    elif m.hungry >= 4 and m.hoard < 300:
        reason = "hunger"
    if reason is None:
        return
    land = realm.terrain != 0
    cells = realm.remote_cells(land & (realm.landmass == m.landmass))
    if not len(cells):
        return
    pick = cells[rng.choice(len(cells), size=min(40, len(cells)), replace=False)]
    if reason == "hunger":
        score = [game_near(realm, (int(r), int(c))) for r, c in pick]
    elif reason in ("rival", "wounded"):
        score = [min([dist((r, c), o.pos) for o in dragons if o is not m] or [999]) + realm.town_dist[r, c]
                 for r, c in pick]
    r, c = pick[int(np.argmax(score))]
    old = realm.nearest_town_name(m.pos)
    m.pos = (int(r), int(c))
    m.landmass = int(realm.landmass[m.pos])
    m.settled = realm.year
    m.game0 = max(1.0, game_near(realm, m.pos))
    m.hungry = 0
    m.tributaries = {}
    why = {"hunger": "the herds are gone, and it follows them", "wounded": "wounded near to death, it flies to the "
           "far peaks to heal", "rival": "a greater dragon has come, and it gives way"}[reason]
    realm.say(f"{m.name} leaves its lair near {old}: {why}. It settles near {realm.nearest_town_name(m.pos)}.",
              minor=reason == "hunger" and m.power < 2500)


# ---------------------------------------------------------------- tribute
def tribute_year(realm, dragons):
    from .events import book
    from .opinion import remember
    rng = realm.rng
    for m in dragons:
        if hidden(m, realm.year):
            continue
        m.tributaries = getattr(m, "tributaries", {})
        m.wrath = {k: v for k, v in getattr(m, "wrath", {}).items() if v > realm.year}
        reach = {}
        for t in realm.alive_towns():
            if t.landmass == m.landmass and dist(t.pos, m.pos) <= REACH:
                reach.setdefault(t.faction, []).append(t)
        for kid in list(m.tributaries):
            if kid not in reach or realm.extinct[kid]:
                del m.tributaries[kid]
        for kid, towns in reach.items():
            k = realm.kingdoms[kid]
            r = realm.chars.get(k.ruler)
            if r is None or kid in m.wrath:
                continue
            revenue = max(1.0, getattr(k, "income", 0.0) * getattr(k, "tax_now", 0.3))
            price = revenue * min(0.5, 0.1 + m.base / 40000) * len(towns) / max(1, len(realm.alive_towns(kid)))
            price = max(1.0, price)
            p = r.personality
            if kid in m.tributaries:
                since, payer = m.tributaries[kid]
                refuse = payer != r.id and ("brave" in p and ("wrathful" in p or "arrogant" in p)) and rng.random() < 0.5
                if k.gold < -revenue * 2 or refuse:                       # missed, or a new ruler will not bow
                    del m.tributaries[kid]
                    m.wrath[kid] = realm.year + 8
                    realm.say(f"**{k.name} stops paying {m.name}.** "
                              + (f"{realm.who(r)} will not bow to a wyrm" if refuse else "The treasury is empty")
                              + f"; the dragon remembers {realm.year - since} years of gold, and comes for it in fire.")
                    continue
                k.gold -= price
                m.hoard += price
                book(realm, kid, "dragon tribute", -price, f"tribute to {m.name}")
                continue
            # should we pay? the danger against the price, weighed by who the ruler is
            exposure = sum(t.pop for t in towns) / max(1.0, sum(t.pop for t in realm.alive_towns(kid)))
            danger = exposure * (m.base / 2000) * RAID_MULT[getattr(m, "temper", "territorial")]
            proud = ("brave" in p) + ("wrathful" in p) + ("arrogant" in p) + ("zealous" in p)
            wary = ("craven" in p) + ("content" in p) + ("calm" in p) + ("patient" in p) + (r.attr("ste") >= 10)
            hit = any(getattr(t, "raided_by", None) is m for t in towns)
            if danger > 0.3 and wary + hit > proud and k.gold > price and rng.random() < 0.3:
                m.tributaries[kid] = (realm.year, r.id)
                k.gold -= price
                m.hoard += price
                book(realm, kid, "dragon tribute", -price, f"tribute to {m.name}")
                realm.say(f"**A bargain with a dragon.** {realm.who(r)} sends gold up the mountain to {m.name}, "
                          f"{price:,.1f} a year, and {k.name} is spared its fire.")
                r.legitimacy = max(0.0, (r.legitimacy or 50) - (6 if k.people == 3 else 2))
                for d in realm.duchies.values():
                    if d["ctrl"] == kid and d["holder"] not in (None, r.id) and d["holder"] in realm.chars:
                        holder = realm.chars[d["holder"]]
                        if "brave" in holder.personality or "arrogant" in holder.personality:
                            remember(realm, holder, r, "bowed to a wyrm", -10, decay=1.0)


def spares(m, town):
    return town.faction in getattr(m, "tributaries", {})


def wrath_on(m, town, year):
    return getattr(m, "wrath", {}).get(town.faction, 0) > year


def freed(realm, m):
    """A slain dragon frees its tributaries."""
    for kid in getattr(m, "tributaries", {}):
        if not realm.extinct[kid]:
            realm.say(f"With {m.name} dead, {realm.kingdoms[kid].name} pays no more tribute to the mountain.",
                      minor=True)
    m.tributaries = {}
