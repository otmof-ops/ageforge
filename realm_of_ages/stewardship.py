"""Running a realm's economy: what rulers see, what they decide, and what it costs them to get it wrong.

Every year a ruler reads the realm: debt against revenue, poverty, the idle, the shortage of hands, the skill
of the trades, the crowding of the towns. Whether they act at all, and how well, is their own stewardship and their
steward's, and their nature: the diligent act, the lazy let it drift, the greedy build for show, the compassionate
feed the poor. What they can do:

- build in the towns: mills and granaries, mine works, workshops, market halls, harbours, schools, and housing;
- hire to the court: a Master of the Mint, a Harbourmaster, a Master of Works, an Almoner (the Royal Architect,
  already at court, makes building cheaper);
- relieve the poor, or lift the taxes.

None of it is free, and none of it happens by itself. A realm whose debt runs away, whose trades sit idle and whose
poor starve is badly run, and the ruler pays for it: legitimacy, and the lords' opinion, which is where factions and
risings start. A realm that prospers lifts its ruler: legitimacy, and renown as a builder."""
import math

from .markets import CLASSES, stage

BUILDINGS = {  # name: (base cost, what it is for, needs)
    "Mill and Granary": (6, "farm", None),
    "Mine Works": (8, "mine", None),
    "Workshops": (7, "craft", None),
    "Market Hall": (7, "trade", "Coinage"),
    "Harbour": (10, "trade", "Longships"),
    "School": (9, "skill", "Monastery Schools"),
    "Housing": (4, "housing", None),
    "Arcane College": (9, "arcane", "Monastery Schools"),
}
MAX_LEVEL = 5
ECON_POSITIONS = {  # position: (how to judge a candidate, salary a year, what drives the need)
    "Master of the Mint": (lambda c: c.attr("ste") * 4 + c.skills.get("Mercantile", 0) / 3, 6, "money"),
    "Harbourmaster": (lambda c: c.skills.get("Navigation", 0) + c.skills.get("Seamanship", 0), 4, "trade"),
    "Master of Works": (lambda c: c.skills.get("Smithing", 0) + c.attr("ste") * 3, 4, "craft"),
    "Almoner": (lambda c: c.skills.get("Speech", 0) + c.skills.get("Restoration", 0)
                + 30 * ("compassionate" in c.personality), 3, "poverty"),
}


def realm_economy(realm, k):
    """The numbers a ruler is judged by, gathered from the towns."""
    towns = realm.alive_towns(k.id)
    pop = sum(t.pop for t in towns) or 1.0
    cls = {c: [0.0, 0.0, 0.0, {"land": 0.0, "housing": 0.0, "stock": 0.0, "coin": 0.0}] for c in CLASSES}
    for t in towns:
        for c, (n, per, pov, assets) in getattr(t, "cls", {}).items():
            row = cls[c]
            row[0] += n
            row[1] += n * per
            row[2] += n * pov
            for a, v in assets.items():
                row[3][a] += v
    out = {}
    total_assets = sum(sum(r[3].values()) for r in cls.values()) or 1.0
    for c, (n, inc, pov, assets) in cls.items():
        out[c] = {"people": n, "income": inc / max(1.0, n), "poverty": pov / max(1.0, n),
                  "assets": assets, "per_head": sum(assets.values()) / max(1.0, n),
                  "wealth_share": sum(assets.values()) / total_assets}
    work = sum(0.55 * t.pop for t in towns) or 1.0
    debt = max(0.0, -k.gold)
    revenue = max(1.0, getattr(k, "income", 0.0) * getattr(k, "tax_now", 0.3) * getattr(k, "collect", 1.0))
    e = {"classes": out, "poverty": sum(out[c]["poverty"] * out[c]["people"] for c in CLASSES) / pop,
         "jobless": sum(getattr(t, "jobless", 0.0) * 0.55 * t.pop for t in towns) / work,
         "short": sum(getattr(t, "labour_short", 0.0) * t.pop for t in towns) / pop,
         "crowded": sum(getattr(t, "crowded", 0.0) * t.pop for t in towns) / pop,
         "debt_ratio": debt / revenue, "revenue": revenue,
         "gdp": getattr(k, "real_gdp", 0.0)}
    e["distress"] = min(1.0, max(0.0, (e["debt_ratio"] - 1.5) / 4)
                        + max(0.0, e["poverty"] - 0.45) * 2 + e["jobless"] * 3)
    k.econ = e
    k.econ_distress = e["distress"]
    return e


def competence(realm, k, r):
    steward = realm.chars.get(k.council.get("steward")) if getattr(k, "council", None) else None
    s = steward.attr("ste") if steward is not None else 0
    p = r.personality
    attention = 0.35 + 0.025 * r.attr("ste") + 0.02 * s + 0.3 * ("diligent" in p) - 0.3 * ("lazy" in p) \
        - 0.1 * ("content" in p) + 0.1 * ("ambitious" in p)
    judgement = (r.attr("ste") + 1.3 * s) / 2                         # how well the choice is made, when made
    return max(0.05, min(0.95, attention)), judgement


def build_cost(realm, k, name):
    ct = getattr(k, "court", None) or {}
    arch = realm.chars.get(ct.get("pos", {}).get("Royal Architect"))
    cut = min(0.3, arch.attr("ste") / 60) if arch is not None else 0.0
    return BUILDINGS[name][0] * (1 - cut)


def options(realm, k, e, towns):
    """Every building that would help, scored by how much the realm needs it."""
    inn = realm.cultures[k.people].innovations
    out = []
    for t in towns:
        b = getattr(t, "bld", {})
        sk = getattr(t, "skill", {})
        weight = math.sqrt(t.pop)
        for name, (cost, aim, need) in BUILDINGS.items():
            if b.get(name, 0) >= MAX_LEVEL or (need and need not in inn):
                continue
            if name == "Harbour" and not t.port:
                continue
            if name == "Mine Works" and not getattr(t, "jobs", {}).get("mine"):
                continue
            if aim == "housing":
                gain = 4 * getattr(t, "crowded", 0.0) + 0.5 * max(0.0, getattr(t, "poverty", 0) - 0.4)
            elif aim == "skill":
                gain = 0.6 * (1 - sum(sk.values()) / max(1, len(sk))) + 0.3 * max(0.0, e["poverty"] - 0.4)
            elif aim == "farm":
                gain = 0.5 * getattr(t, "labour_short", 0.0) + 0.3 * max(0.0, 1 - getattr(t, "food_prod", t.pop)
                                                                        / max(1.0, t.pop)) + 0.1
            else:                                                      # new trades for the idle
                gain = 3 * getattr(t, "jobless", 0.0) + 0.4 * (1 - sk.get(aim, 0.5)) + 0.05
            out.append((gain * weight / (1 + b.get(name, 0)), t, name))
    return out


def manage_year(realm):
    from .events import book
    from .opinion import remember
    rng = realm.rng
    for k in realm.kingdoms:
        if realm.extinct[k.id] or k.ruler is None:
            continue
        towns = realm.alive_towns(k.id)
        if not towns:
            continue
        r = realm.chars[k.ruler]
        e = realm_economy(realm, k)
        attention, judgement = competence(realm, k, r)
        p = r.personality
        # upkeep of what has been built
        upkeep = sum(BUILDINGS[n][0] * lv for t in towns for n, lv in getattr(t, "bld", {}).items()) * 0.01
        if upkeep:
            k.gold -= upkeep
            book(realm, k.id, "buildings upkeep", -upkeep)
        hire(realm, k, r, e)
        if rng.random() > attention:
            reckoning(realm, k, r, e)                                  # the year goes by unattended
            continue
        # relief for the poor, if the ruler cares and can pay
        if e["poverty"] > 0.5 and ("compassionate" in p or "just" in p or judgement > 12) and k.gold > 50:
            cost = 0.04 * e["gdp"]
            k.gold -= cost
            book(realm, k.id, "poor relief", -cost)
            for t in towns:
                if getattr(t, "poverty", 0) > 0.45:
                    t.relief = realm.year + 3
            if rng.random() < 0.2:
                realm.say(f"{realm.who(r)} opens the granaries and funds the almshouses of {k.name}.", minor=True)
        if e["poverty"] > 0.6 and getattr(k, "tax_relief", 0) <= realm.year and e["debt_ratio"] < 2 and \
                ("compassionate" in p or "generous" in p or judgement > 14):
            k.tax_relief = realm.year + 5
            realm.say(f"{realm.who(r)} lifts part of the taxes of {k.name} while the poor recover.", minor=True)
        # building: a sound judge builds what is needed; a poor one, what catches the eye
        prudent = judgement >= 9 and "impatient" not in p
        budget = k.gold - (3 if prudent else 0.5) * max(10.0, e["revenue"]) * 0.3
        opts = options(realm, k, e, towns)
        if not opts:
            reckoning(realm, k, r, e)
            continue
        for _ in range(1 + int(judgement > 12)):
            if "greedy" in p and rng.random() < 0.4:
                pick = max(opts, key=lambda o: o[1].pop)               # the grand works in the capital's eye
                pick = (0, realm.capital_of(k.id) or pick[1], "Market Hall" if "Coinage" in
                        realm.cultures[k.people].innovations else "Workshops")
            elif rng.random() < min(0.9, judgement / 18):
                pick = max(opts, key=lambda o: o[0])
            else:
                pick = opts[int(rng.integers(len(opts)))]
            _, t, name = pick
            cost = build_cost(realm, k, name)
            if cost > budget:
                break
            t.bld = getattr(t, "bld", {})
            t.bld[name] = t.bld.get(name, 0) + 1
            if name == "Housing":
                t.housing = getattr(t, "housing", t.pop) + 300
            k.gold -= cost
            budget -= cost
            book(realm, k.id, "buildings", -cost, f"builds {name} in {t.name}")
            r.built = getattr(r, "built", 0) + 1
            if t.bld[name] >= 3:
                realm.say(f"{realm.who(r)} builds {'a ' if name not in ('Housing', 'Workshops') else ''}{name} at "
                          f"{t.name} (level {t.bld[name]}).", minor=True)
            opts = [o for o in opts if not (o[1] is t and o[2] == name)]
            if not opts:
                break
        reckoning(realm, k, r, e)


def hire(realm, k, r, e):
    """The court's economic offices: filled when the realm needs them and can pay, let go when it cannot."""
    from .events import book
    ct = getattr(k, "court", None)
    if not ct:
        return
    st = stage(realm.cultures[k.people])
    need = {"money": st >= 1, "trade": getattr(k, "partners", 0) >= 3 and any(t.port for t in realm.alive_towns(k.id)),
            "craft": st >= 1, "poverty": e["poverty"] > 0.4}
    pool = None
    for pos, (score, salary, drive) in ECON_POSITIONS.items():
        cur = realm.chars.get(ct["pos"].get(pos))
        broke = k.gold < -e["revenue"] * 1.5
        if cur is not None and (not cur.alive or cur.realm != k.id or broke or not need[drive]):
            ct["pos"].pop(pos, None)
            cur = None
        if cur is None and need[drive] and not broke and k.gold > salary * 10:
            if pool is None:
                used = set(ct["pos"].values()) | {r.id}
                pool = [realm.chars[i] for i in realm.member_ids(k.id) if realm.chars[i].adult(realm.year)
                        and i not in used]
            if not pool:
                continue
            best = max(pool, key=score)
            pool.remove(best)
            ct["pos"][pos] = best.id
            best.titles.append(f"{pos} of {k.name} from year {realm.year}")
            realm.say(f"{realm.who(r)} names {realm.who(best)} {pos}.", minor=True)
            cur = best
        if cur is not None:
            pay = salary
            k.gold -= pay
            cur.gold += pay
            book(realm, k.id, "court offices", -pay)
    # what the officers do
    mint = realm.chars.get(ct["pos"].get("Master of the Mint"))
    k.mint_skill = score_of(ECON_POSITIONS["Master of the Mint"][0], mint) / 60 if mint else 0.0
    works = realm.chars.get(ct["pos"].get("Master of Works"))
    harbour = realm.chars.get(ct["pos"].get("Harbourmaster"))
    k.sector_boost = {"craft": min(0.08, score_of(ECON_POSITIONS["Master of Works"][0], works) / 1500) if works else 0.0,
                      "trade": min(0.08, score_of(ECON_POSITIONS["Harbourmaster"][0], harbour) / 2000) if harbour else 0.0}
    almoner = realm.chars.get(ct["pos"].get("Almoner"))
    if almoner is not None:
        for t in realm.alive_towns(k.id):
            if getattr(t, "poverty", 0) > 0.5:
                t.relief = max(getattr(t, "relief", 0), realm.year + 1)


def score_of(fn, c):
    return fn(c) if c is not None else 0.0


def reckoning(realm, k, r, e):
    """The realm judges its ruler by how it fares: prosperity lifts a ruler, mismanagement wears them down."""
    from .opinion import remember
    prev = getattr(k, "econ_prev", None)
    k.econ_prev = e
    good = e["distress"] < 0.1 and e["poverty"] < 0.45
    bad = e["distress"] > 0.5
    if good:
        r.legitimacy = min(100.0, (r.legitimacy or 50) + 0.4)
        if prev is not None and e["gdp"] > prev["gdp"] * 1.01:
            r.renown += 0.5 + 0.1 * getattr(r, "built", 0) ** 0.5
    elif bad:
        r.legitimacy = max(0.0, (r.legitimacy or 50) - 1.0 * e["distress"])
        if realm.rng.random() < 0.2:
            for d in realm.duchies.values():
                if d["ctrl"] == k.id and d["holder"] not in (None, r.id) and d["holder"] in realm.chars:
                    remember(realm, realm.chars[d["holder"]], r, "has ruined the realm's purse", -8, decay=1.5)
            if realm.rng.random() < 0.3:
                why = "debts" if e["debt_ratio"] > 3 else \
                    "hunger and want" if e["poverty"] > 0.5 else "idle hands"
                realm.say(f"The lords of {k.name} blame {realm.who(r)} for the {why} that grip the realm.")
