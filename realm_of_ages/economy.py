"""The material world: weather and harvests, granaries and famine, births and deaths, goods and wealth that wear
away, taxes on income, debt and bankruptcy, and people moving away from hunger and war.

Each region has its own weather every year (spatially smooth, so a bad year hits a whole province). A town feeds
people from its land: yields by terrain and people, times the harvest, times tilling innovations, magic, game,
curses and legendary seeds. Surplus goes to the granary (up to a year's need); a shortfall is drawn from it, and
what is still short is famine: half of the shortfall die, and the hungry leave. Births and deaths follow crude
rates calibrated on the medieval record (see calibration.md), with frontier towns on open land breeding faster.
Goods and wealth depreciate, so wealth is a flow, not a pile; crowns tax income, and pay interest on what they
borrow."""
import numpy as np
from . import nd as ndimage

from .data import FOOD

# crude birth and death rates a year, by people: humans are calibrated on medieval Europe, the others scaled
VITAL = [(0.040, 0.035), (0.014, 0.010), (0.022, 0.018), (0.050, 0.043)]
GRANARY_YEARS = 1.0
DEPRECIATION = 0.04


def weather_year(realm):
    """This year's weather and harvest: see climate.py."""
    from .climate import weather_year as climate_year
    climate_year(realm)


def harvest_at(realm, pos):
    eco = realm.eco
    return float(realm.harvest[min(eco.eh - 1, pos[0] // 6), min(eco.ew - 1, pos[1] // 6)])


def town_year(realm, alive, food, wealth, mana, cells):
    """Food, people and goods for every town: grow the harvest (weather, soil, hands), move grain to the hungry as
    far as the realm can, then eat, store and count the cost."""
    from . import resources
    from .climate import rain_at, soil_year
    from .data import MAGIC
    rng = realm.rng
    y = realm.year
    seed_bonus = {}
    for h in realm.heroes():
        if any(it.special == "food" for it in h.items):
            seed_bonus[h.realm] = 1.2
    live = []
    # 1. the harvest
    for k_, s in enumerate(alive):
        if not s.alive:
            continue
        kd = realm.kingdoms[s.faction]
        cu = realm.cultures[s.culture] if getattr(realm, "cultures", None) else None
        s.cells = int(cells[k_])
        s.magic = MAGIC[s.culture] * min(1.0, mana[k_] / 5) * (1 + 0.1 * kd.realm_tier)
        rf, rw = resources.town_bonus(s)
        tech = 1 + (cu.total("food") if cu else 0)
        magic = 1 + 0.15 * s.magic + 0.01 * kd.realm_tier                # druids and hedge-wizards in the fields
        base = food[k_] * 25
        soil = soil_year(realm, s, base * tech, cu)
        prod = (base * soil * harvest_at(realm, s.pos) * tech * magic * kd.mod("food", y)
                * seed_bonus.get(s.faction, 1.0) * getattr(s, "curse_mult", 1.0) * (1 + getattr(s, "game", 0.0))
                * (1 + rf) * (0.6 if getattr(s, "occupied_by", None) is not None else 1.0))
        gh = getattr(realm, "god_harvest", {}).get(s.faction)
        if gh is not None and gh[1] >= y:
            prod *= gh[0]                                             # the gods' hand on the fields
        prod *= 0.55 + 0.45 * min(1.0, getattr(s, "farm_fill", 1.0))   # fields need hands to work them
        s.food_prod = prod
        s.food = prod
        live.append(s)
    # 2. grain moves from plenty to want, inside a realm and over land, as far as its carts, clerks and markets reach
    by = {}
    for s in live:
        by.setdefault((s.faction, s.landmass), []).append(s)
    moved_all = 0.0
    for (f, _), towns in by.items():
        if len(towns) < 2:
            continue
        cu = realm.cultures[realm.kingdoms[f].people]
        inn = cu.innovations
        reach = min(0.6, 0.15 + 0.2 * ("Coinage" in inn) + 0.15 * ("Royal Chancery" in inn)
                    + 0.1 * ("Bureaucracy" in inn))
        spare = [(s, s.food - s.pop) for s in towns if s.food > s.pop]
        want = [(s, s.pop - s.food - getattr(s, "granary", 0.0)) for s in towns]
        want = [(s, w) for s, w in want if w > 0]
        if not spare or not want:
            continue
        have = sum(v for _, v in spare) * reach
        need = sum(w for _, w in want)
        moved = min(have, need)
        if moved <= 0:
            continue
        for s, v in spare:
            s.food -= moved * v / (have / reach)
        for s, w in want:
            s.food += moved * w / need
            s.grain_in = moved * w / need
        moved_all += moved
    realm.grain_moved = getattr(realm, "grain_moved", 0.0) + moved_all
    # 3. eat, store, and count the cost
    famine_towns = []
    for s in live:
        kd = realm.kingdoms[s.faction]
        eat = s.food
        need = s.pop
        g = getattr(s, "granary", need * 0.3)
        g *= 0.95 * (1 - 0.08 * max(0.0, rain_at(realm, s.pos) - 1.1))   # vermin, and rot in damp stores
        if eat >= need:
            g = min(need * GRANARY_YEARS, g + (eat - need))
            short = 0.0
        else:
            draw = min(g, need - eat)
            g -= draw
            short = (need - eat - draw) / max(1.0, need)
        short *= 1 - min(0.25, 15 * getattr(s, "arcane", 0.0))       # preserving spells, and spells that call rain
        s.granary = g
        s.famine = short
        b, d = VITAL[s.culture]
        ratio = eat / max(1.0, need)
        # open land breeds faster; as the land fills, people marry later and have fewer children
        b *= (1 + 0.3 * min(1.0, max(0.0, (ratio - 1) / 2))) * min(1.2, max(0.15, (ratio - 0.95) / 0.45))
        s.pop += s.pop * (b - d) * kd.growth_mult * s.lord_mod
        if short > 0:
            dead = s.pop * short * 0.5
            s.pop -= dead
            s.hunger_dead = dead
            famine_towns.append(s)
        s.pop *= 1 + rng.normal(0, 0.005)
        s.pop = max(1.0, s.pop)
    famine_news(realm, famine_towns)
    migrate(realm, famine_towns)


def famine_news(realm, towns):
    dead = sum(getattr(s, "hunger_dead", 0) for s in towns)
    if not towns or dead < 1000:
        return
    worst = max(towns, key=lambda s: s.famine)
    lands = {s.landmass for s in towns}
    from .geography import land_name
    realm.say(f"{'**Famine.** ' if dead > 3000 else ''}The harvest fails across {', '.join(land_name(realm, l) for l in lands)}"
              f"; {len(towns)} towns go hungry and some {dead:,.0f} starve, worst in {worst.name}.", minor=dead < 3000)
    realm.famines.append((realm.year, len(towns), dead))


def migrate(realm, hungry):
    """The hungry and the war-torn leave for the nearest towns that can feed them."""
    rng = realm.rng
    moving = list(hungry) + [s for s in realm.alive_towns() if getattr(s, "occupied_by", None) is not None]
    if not moving:
        return
    towns = [t for t in realm.alive_towns() if getattr(t, "famine", 0) == 0 and getattr(t, "occupied_by", None) is None]
    if not towns:
        return
    pos = np.array([t.pos for t in towns], float)
    for s in moving:
        n = s.pop * (0.04 + 0.3 * getattr(s, "famine", 0))
        d = np.hypot(pos[:, 0] - s.pos[0], pos[:, 1] - s.pos[1])
        near = [towns[int(i)] for i in np.argsort(d)[:3] if d[int(i)] < 25 and towns[int(i)].landmass == s.landmass]
        if not near:
            continue
        s.pop -= n
        for t in near:
            t.pop += n / len(near)
        realm.migrants += n


# ---------------------------------------------------------------- the crown's purse
def treasury_year(realm):
    """Taxes on income, interest on debt, and bankruptcy for a crown that cannot pay."""
    from .opinion import remember
    for k in realm.kingdoms:
        if realm.extinct[k.id] or k.ruler is None:
            continue
        towns = realm.alive_towns(k.id)
        k.income = sum(getattr(t, "income", 0.0) for t in towns if getattr(t, "occupied_by", None) is None)
        if k.gold < 0:
            cu = realm.cultures[k.people]
            rate = 0.05 if "Banking Houses" in cu.innovations else 0.08
            from .events import book
            book(realm, k.id, "interest", k.gold * rate)
            k.gold *= 1 + rate                                        # (superseded by markets.money_year)
            limit = -max(300.0, 6 * k.income * getattr(k, "tax_now", 0.3))
            if k.gold < limit / 2:
                austerity(realm, k)                                   # cut the court and the regiments first
            if k.gold < limit:
                bankrupt(realm, k)


def austerity(realm, k):
    """A crown deep in debt sells off regiments and strips the court before it will default."""
    from .civics import MAA
    from .events import book
    ct = getattr(k, "court", None)
    if ct:
        for a in ct["amen"]:
            ct["amen"][a] = 1
    sold = 0
    while k.maa and k.gold < 0:
        m = max(k.maa, key=lambda m: MAA[m][1])
        k.maa[m] -= 1
        if k.maa[m] <= 0:
            del k.maa[m]
        k.gold += MAA[m][1] * 20                                       # arms and horses sold back
        book(realm, k.id, "austerity", MAA[m][1] * 20)
        sold += 1
    if sold:
        realm.say(f"{k.name} is deep in debt: {sold} regiments are paid off and the court is stripped bare.",
                  minor=True)


def bankrupt(realm, k):
    from .opinion import remember
    r = realm.chars[k.ruler]
    k.gold = 0.0
    for m in list(k.maa):
        k.maa[m] = k.maa[m] // 2
        if k.maa[m] <= 0:
            del k.maa[m]
    r.legitimacy = max(0.0, (r.legitimacy or 50) - 15)
    for d in realm.duchies.values():
        if d["ctrl"] == k.id and d["holder"] not in (None, r.id):
            remember(realm, realm.chars[d["holder"]], r, "the crown defaulted on its debts", -15, decay=1.0)
    if getattr(k, "court", None):
        for a in k.court["amen"]:
            k.court["amen"][a] = 1
    realm.bankruptcies.append((realm.year, k.id))
    realm.say(f"**{k.name} is bankrupt.** {realm.who(r)} cannot pay the soldiers or the lenders; half the regiments "
              f"melt away and the court goes threadbare.")
