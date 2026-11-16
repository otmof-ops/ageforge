"""Titles, vassals, factions and the birth of new realms, after the feudal model.

Every town is a county. The land is divided de jure into duchies (24 leagues square, never crossing the sea)
and de jure kingdom titles (72 leagues square). Whoever holds most of a duchy's counties controls it; whoever
controls most of a kingdom title's duchies holds that kingdom. A realm's rank follows its titles: a duchy, a
kingdom, or, with three kingdom titles, an empire.

The ruler keeps the capital's duchy and grants the rest to vassal dukes, and a duchy passes to the duke's heir.
Each duke has an opinion of their liege: legitimacy, tyranny, shared blood, culture and faith, and distance
(a duchy across the sea is a colony, and colonies drift). Unhappy dukes form factions: independence (their
lands become a realm of their own), a claimant (put someone else on the throne) or liberty (bind the crown).
A faction strong enough sends an ultimatum; refused, it means war. Cadet branches split from great houses
when a younger child is given a duchy, and stay part of the dynasty."""
import numpy as np

from .data import COLORS, MARTIAL, PEOPLES, REALM_STYLE
from .geography import land_name

DUCHY, KTITLE = 24, 72


def opinion_bias(c, rng):
    if not hasattr(c, "op_bias"):
        c.op_bias = float(rng.normal(0, 10))
    return c.op_bias


def duchy_key(realm, t):
    return (t.landmass, t.pos[0] // DUCHY, t.pos[1] // DUCHY)


def ktitle_key(realm, t):
    return (t.landmass, t.pos[0] // KTITLE, t.pos[1] // KTITLE)


def ensure_titles(realm):
    for t in realm.alive_towns():
        dk = duchy_key(realm, t)
        if dk not in realm.duchies:
            realm.duchies[dk] = {"name": f"the Duchy of {t.name}", "holder": None, "kt": ktitle_key(realm, t),
                                 "since": realm.year, "land": t.landmass, "ctrl": None}
        kt = ktitle_key(realm, t)
        if kt not in realm.ktitles:
            small = realm.lands[t.landmass]["size"] < 3000
            nm = (f"the Kingdom of {land_name(realm, t.landmass)}" if small
                  else f"the Kingdom of {t.name}")
            realm.ktitles[kt] = {"name": nm, "ctrl": None}
        t.duchy = dk


def control(realm):
    """Who controls each duchy and each kingdom title, this year."""
    counts = {}
    for t in realm.alive_towns():
        counts.setdefault(t.duchy, {}).setdefault(t.faction, 0)
        counts[t.duchy][t.faction] += 1
    for dk, d in realm.duchies.items():
        c = counts.get(dk)
        d["ctrl"] = max(c, key=c.get) if c else None
    kc = {}
    for dk, d in realm.duchies.items():
        if d["ctrl"] is not None:
            kc.setdefault(d["kt"], {}).setdefault(d["ctrl"], 0)
            kc[d["kt"]][d["ctrl"]] += 1
    for kt, kd in realm.ktitles.items():
        c = kc.get(kt)
        kd["ctrl"] = max(c, key=c.get) if c else None


def rank_names(realm):
    """A realm's rank follows the kingdom titles it holds."""
    held = {}
    for kd in realm.ktitles.values():
        if kd["ctrl"] is not None:
            held[kd["ctrl"]] = held.get(kd["ctrl"], 0) + 1
    for k in realm.kingdoms:
        if realm.extinct[k.id]:
            continue
        n = held.get(k.id, 0)
        rank = "empire" if n >= 3 else "kingdom" if n >= 1 else "duchy"
        if rank != k.rank:
            cap = realm.capital_of(k.id)
            place = getattr(k, "place", None) or (cap.name if cap else k.short)
            k.place = place
            old = k.name
            k.name = style(k.people, rank, place)
            if rank == "empire" and k.rank != "empire":
                realm.say(f"**An empire.** Holding {n} kingdom titles, {old} proclaims itself {k.name}.")
            elif k.rank == "empire":
                realm.say(f"{old} has lost its kingdoms; it is {k.name} now.")
            elif rank == "kingdom" and k.rank == "duchy":
                realm.say(f"{old} raises itself to a kingdom: {k.name}.", minor=True)
            k.rank = rank


def style(people, rank, place):
    if rank == "empire":
        return ["the Empire of {}", "the High Court of {}", "the Deep Empire of {}", "the Great Horde of {}"][people].format(place)
    if rank == "kingdom":
        return REALM_STYLE[people].format(place)
    return ["the Duchy of {}", "the Elven March of {}", "the Hold of {}", "the Warband of {}"][people].format(place)


# ---------------------------------------------------------------- vassals
def grant_duchies(realm):
    rng = realm.rng
    y = realm.year
    for k in realm.kingdoms:
        if realm.extinct[k.id] or k.ruler is None:
            continue
        r = realm.chars[k.ruler]
        cap = realm.capital_of(k.id)
        cap_duchy = cap.duchy if cap is not None else None
        for dk, d in realm.duchies.items():
            if d["ctrl"] != k.id:
                continue
            h = realm.chars.get(d["holder"])
            if dk == cap_duchy:
                d["holder"] = r.id                                    # the crown keeps its own duchy
                continue
            if h is not None and h.alive and h.realm == k.id and h.id != r.id:
                continue
            if h is not None and not h.alive:                        # a duchy passes to the duke's heir
                from .governance import heir_of
                heir = heir_of(realm, h)
                if heir is not None and heir.alive and heir.realm == k.id and heir.id != r.id:
                    give_duchy(realm, heir, dk, d, k, inherited=True)
                    continue
            cands = [realm.chars[i] for i in realm.member_ids(k.id) if True
                     and realm.chars[i].adult(y) and realm.chars[i].id != r.id and not realm.chars[i].undead
                     and realm.chars[i].disgraced_until <= y and len(getattr(realm.chars[i], "duchies", [])) < 2
                     and (realm.chars[i].house is not None or realm.chars[i].renown >= 200)]
            if not cands:
                d["holder"] = None
                continue
            near = [c for c in cands if any(realm.by_id.get(s) is not None and realm.by_id[s].duchy == dk
                                            for s in c.lordships)]
            pick = max(near or cands, key=lambda c: c.renown / 50 + (realm.houses[c.house].prestige / 200
                                                                       if c.house is not None else 0)
                       + (8 if c.house == r.house else 0) + rng.random() * 5)
            give_duchy(realm, pick, dk, d, k)


def give_duchy(realm, c, dk, d, k, inherited=False):
    from .dynasty import new_house
    old = realm.chars.get(d["holder"])
    if old is not None and dk in getattr(old, "duchies", []):
        old.duchies.remove(dk)
    d["holder"] = c.id
    c.duchies = getattr(c, "duchies", []) + [dk]
    if not inherited and k.ruler is not None:
        from .opinion import remember
        remember(realm, c, realm.chars[k.ruler], "granted me a title", 25, decay=0.5)
    c.titles.append(f"{'Duchess' if c.female else 'Duke'} of {d['name'].replace('the Duchy of ', '')} "
                    f"from year {realm.year}")
    if c.legitimacy is None:
        c.legitimacy = 55.0
    r = realm.chars.get(k.ruler)
    # a younger child of a great house, given a duchy, may found a cadet branch of the dynasty
    if (not inherited and c.house is not None and r is not None and realm.rng.random() < 0.3
            and realm.houses[c.house].founder != c.id and c.father is not None
            and realm.houses[c.house].rulers > 0):
        parent = realm.houses[c.house]
        place = d["name"].replace("the Duchy of ", "")
        from .data import HOUSE_STYLE
        if any(h.name == HOUSE_STYLE[c.people].format(place) for h in realm.houses.values()):
            return                                                    # that name is already a house's
        h = new_house(realm, c, place, track=parent.track, origin=f"a cadet branch of {parent.name}, founded in year "
                      f"{realm.year} by {c.full_name()}, {'Duchess' if c.female else 'Duke'} of {place}")
        h.parent = parent.id
        h.dynasty = getattr(parent, "dynasty", parent.id)
        h.prestige = parent.prestige * 0.1
        realm.say(f"{realm.who(c)} founds {h.name}, a cadet branch of {parent.name}.", minor=True)


# ---------------------------------------------------------------- factions
def opinion(realm, duke, k, r, dk):
    """A duke's opinion of their liege: the full opinion, plus the pull of distance and of their own power."""
    from . import opinion as op_mod
    d = realm.duchies[dk]
    cap = realm.capital_of(k.id)
    op = op_mod.opinion(realm, duke, r) + 15
    if cap is not None and d["land"] != cap.landmass:
        op -= 35 + 10 * min(5, (realm.year - d["since"]) // 100)         # colonies drift further every century
    op -= sum(1 for x in realm.duchies.values() if x["ctrl"] == k.id) / 8   # the bigger the realm, the more distant the crown
    op -= 2 * sum(1 for t in realm.alive_towns(k.id) if getattr(t, "duchy", None) == dk)  # a great duchy wants to rule itself
    op -= 10 * ("ambitious" in duke.personality)
    op += 10 * ("content" in duke.personality)
    return op


def towns_of(realm, dks):
    return [t for t in realm.alive_towns() if getattr(t, "duchy", None) in dks]


def factions_year(realm):
    rng = realm.rng
    y = realm.year
    for k in list(realm.kingdoms):
        if realm.extinct[k.id] or k.ruler is None:
            continue
        r = realm.chars[k.ruler]
        if not r.adult(y):
            continue
        cap = realm.capital_of(k.id)
        if cap is None:
            continue
        groups = {}
        for dk, d in realm.duchies.items():
            if d["ctrl"] != k.id or d["holder"] in (None, r.id):
                continue
            duke = realm.chars.get(d["holder"])
            if duke is None or not duke.alive or duke.realm != k.id:
                continue
            op = opinion(realm, duke, k, r, dk)
            duke.opinion = op
            from . import opinion as op_mod
            if op_mod.intimidated(realm, duke, r) or op_mod.hooked(realm, r, duke):
                continue                                              # afraid of them, or in their pocket
            unhappy = getattr(duke, "happy", 0) < -30
            if op >= 0 and not (op < 20 and ("ambitious" in duke.personality or unhappy)) \
                    and r.id not in getattr(duke, "vendettas", {}):
                continue
            if d["land"] != cap.landmass or duke.people != r.people:
                key = ("independence", d["land"])                     # each overseas land breaks away on its own
            elif r.tyranny >= 40:
                key = ("liberty", None)
            else:
                key = ("claimant", None)
            groups.setdefault(key, []).append((duke, dk))
        k.factions = []
        k.faction_since = {key: v for key, v in getattr(k, "faction_since", {}).items() if key in groups}
        for (kind, land), members in groups.items():
            dks = {dk for _, dk in members}
            fp = sum(t.pop * MARTIAL[t.culture] for t in towns_of(realm, dks))
            rest = [t for t in realm.alive_towns(k.id) if t.duchy not in dks]
            if kind == "independence" and land is not None:
                # across the sea the crown fights with what it holds there, and what its ships can carry
                local = sum(t.pop * MARTIAL[t.culture] for t in rest if t.landmass == land)
                carried = sum(t.pop * MARTIAL[t.culture] for t in rest if t.landmass != land) * \
                    min(0.5, 0.05 * (1 + k.ship_tier) * max(1.0, k.warships) ** 0.5)
                lp = (local + carried) * min(2.0, k.battle_mult)
            else:
                lp = sum(t.pop * MARTIAL[t.culture] for t in rest) * min(2.0, k.battle_mult)
            ratio = fp / max(1.0, lp)
            enemies = [o for o in range(realm.npol) if realm.war[k.id, o]]
            if enemies:
                ratio *= 1.25                                          # the crown's enemies arm its rebels
            leader = max((d for d, _ in members), key=lambda c: c.renown + c.attr("mar") * 10)
            fp *= 1 + leader.attr("mar") / 30                          # a war leader worth following
            ages = getattr(k, "faction_since", {})
            since = ages.setdefault((kind, land), y)
            k.faction_since = ages
            fp *= 1 + min(1.0, (y - since) / 30)                      # grievances harden with the years
            k.factions.append({"kind": kind, "land": land, "members": [d.id for d, _ in members], "leader": leader.id,
                               "power": ratio})
            if ratio < 0.7 or y < getattr(k, "ultimatum_cooldown", {}).get(kind, 0):
                continue
            k.ultimatum_cooldown = getattr(k, "ultimatum_cooldown", {})
            k.ultimatum_cooldown[kind] = y + 15
            ultimatum(realm, k, r, kind, land, members, leader, ratio)
            break


def ultimatum(realm, k, r, kind, land, members, leader, ratio):
    rng = realm.rng
    demand = {"independence": f"the freedom of {land_name(realm, land)}" if land is not None else "independence",
              "liberty": "an end to tyranny", "claimant": "a new ruler"}[kind]
    accept = ratio > 1.6 or ("content" in r.personality and kind != "claimant") or \
        (kind == "liberty" and "kind" in r.personality)
    uniq = list({d.id: d for d, _ in members}.values())
    names = ", ".join(realm.who(d) for d in uniq[:4]) + (f" and {len(uniq) - 4} more" if len(uniq) > 4 else "")
    if accept:
        realm.say(f"**An ultimatum.** {names} demand {demand} of {realm.who(r)}, who yields.")
        resolve(realm, k, r, kind, land, members, leader, won=True, war_years=0)
        return
    yrs = int(rng.integers(1, 6))
    fp = ratio * rng.uniform(0.6, 1.4)
    lp = 1.0 * rng.uniform(0.6, 1.4) * (1 + 0.1 * len(getattr(k, "allies", ())))
    won = fp > lp
    for t in realm.alive_towns(k.id):
        t.pop *= 1 - rng.uniform(0.03, 0.12)
    realm.say(f"**A {kind} war.** {names} demand {demand}; {realm.who(r)} refuses. After {yrs} year"
              f"{'s' if yrs > 1 else ''} of war the faction {'wins' if won else 'is crushed'}.")
    resolve(realm, k, r, kind, land, members, leader, won=won, war_years=yrs)


def resolve(realm, k, r, kind, land, members, leader, won, war_years):
    rng = realm.rng
    from .governance import add_tyranny
    if not won:
        for d, dk in members:
            realm.duchies[dk]["holder"] = None
            from .opinion import remember, remember_kin
            remember(realm, d, r, "revoked my title", -40, decay=0.5)
            if d.id == leader.id and rng.random() < 0.6:
                remember_kin(realm, d, r, "executed my kin", -60)
                from .heart import add_stress, reluctance
                add_stress(realm, r, reluctance(r, "execute"), "an execution")
                realm.kill(d, f"was executed for leading a {kind} faction against {r.name} {r.regnal}", quiet=True,
                           by=r)
                add_tyranny(realm, r, 6, "the executions after the rising")
            else:
                d.disgraced_until = realm.year + 30
        r.legitimacy = min(100.0, (r.legitimacy or 50) + 5)
        return
    if kind == "independence":
        secede(realm, k, {dk for _, dk in members}, leader, war_years)
    elif kind == "liberty":
        r.tyranny = max(0.0, r.tyranny - 35)
        r.legitimacy = max(0.0, (r.legitimacy or 50) - 10)
        for d, _ in members:
            d.op_bias = getattr(d, "op_bias", 0) + 15
        realm.say(f"{realm.who(r)} binds the crown: a charter of liberties for the great of {k.name}.")
    else:
        from .dynasty import claim_score, crown, end_reign, pool
        cands = [c for c in pool(realm, k) if c.id != r.id]
        claimant = max(cands, key=lambda c: claim_score(realm, c)) if cands else leader
        realm.kill(r, f"was deposed by a faction of the great and did not survive it", quiet=True)
        end_reign(realm, k, "deposed by a faction")
        crown(realm, k, claimant, "raised up when the line failed")


def secede(realm, k, dks, leader, war_years):
    """Give a faction's duchies a realm of their own, with its leader on the throne."""
    from .dynasty import Kingdom, crown
    rng = realm.rng
    towns = towns_of(realm, dks)
    if not towns:
        return
    cap = max(towns, key=lambda t: t.pop)
    nid = len(realm.kingdoms)
    culture = max({t.culture for t in towns}, key=lambda c: sum(t.pop for t in towns if t.culture == c))
    place = cap.name
    nk = Kingdom(nid, culture, style(culture, "duchy", place), short=f"{PEOPLES[culture]} of {place}")
    nk.place, nk.rank, nk.parent, nk.founded_year = place, "duchy", k.id, realm.year
    base = np.array(COLORS[culture], float)
    nk.color = tuple(int(v) for v in np.clip(base * rng.uniform(0.55, 1.25, 3), 30, 255))
    nk.faith = k.faith
    from .civics import setup_realm, style_for
    nk.gov, nk.laws = k.gov, dict(getattr(k, 'laws', {'crown': 1, 'tolerance': 'tolerant', 'trade': 'open', 'stance': 'defensive'}))
    setup_realm(realm, nk, k.gov)
    nk.rank = 'duchy' if len(towns) >= 12 else 'chiefdom'
    nk.name = style_for(nk, nk.rank, place)
    realm.kingdoms.append(nk)
    realm.grow_tables()
    realm.known[nid] = set(realm.known[k.id])
    realm.home_land[nid] = cap.landmass
    for t in towns:
        if t.capital:
            t.capital = False
        t.faction = nid
        realm.town_moved(t)
    cap.capital = True
    moved = 0
    for i in list(realm.living):
        c = realm.chars[i]
        home = realm.by_id.get(c.home) if c.home is not None else None
        if c.realm == k.id and ((home is not None and home.faction == nid) or c.id == leader.id
                                or set(getattr(c, "duchies", [])) & dks
                                or (c.spouse == leader.id) or c.id in leader.children):
            c.realm = nid
            moved += 1
    rel = -40.0
    realm.relation[k.id, nid] = realm.relation[nid, k.id] = rel
    realm.truce[k.id, nid] = realm.truce[nid, k.id] = realm.year + 20
    realm.say(f"**A new realm.** {land_name(realm, cap.landmass)} breaks from {k.name}"
              + (f" after {war_years} years of war" if war_years else "")
              + f": {nk.name} is born, {len(towns)} towns strong, with {leader.full_name()} on its throne.")
    realm.breakaways.append((realm.year, nid, k.id, leader.id))
    crown(realm, nk, leader, "leading the war of independence")


def titles_year(realm):
    ensure_titles(realm)
    control(realm)
    grant_duchies(realm)
    factions_year(realm)
