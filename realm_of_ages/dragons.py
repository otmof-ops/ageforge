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
years. A slain dragon frees its tributaries."""
import math

import numpy as np

from .data import MONSTER

REACH = MONSTER["dragon"][2]


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
            price = revenue * min(0.5, 0.1 + m.power / 15000) * len(towns) / max(1, len(realm.alive_towns(kid)))
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
            danger = exposure * (m.power / 2000)
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
