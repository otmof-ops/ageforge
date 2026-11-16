"""Statecraft, after the feudal model: reasons for war, wars fought for goals, tributaries and puppets, rivals,
the limits a ruler's own skill puts on what they can hold, and the great decisions that build empires.

A war needs a casus belli: a de jure claim (a duchy that belongs to a kingdom title the attacker holds), a
claim through marriage, a holy war against another faith, conquest (for rulers whose agenda is conquest),
a resource war (a strategic resource the attacker lacks), or subjugation (a great chief over a small tribe).
Battles, captured towns and occupation build warscore; at +100 the attacker takes the goal, at -100 the
defender has won; long stalemates end in white peace. A subjugated or puppeted realm becomes a tributary:
it keeps its ruler, pays tribute, and fights its overlord's wars until it is strong enough to break free.

Every ruler has a vassal limit and a domain limit set by rank and stewardship; holding more than their skill
can manage costs taxes and the dukes' loyalty. Rulers who take their rivals down, unite a people or a
continent, or reach hegemony, earn renown, fame and a legacy."""
import math

from .data import PEOPLES
from .geography import land_name

RANK_VASSALS = {"tribe": 0, "chiefdom": 2, "duchy": 5, "kingdom": 10, "empire": 20}
RANK_DOMAIN = {"tribe": 2, "chiefdom": 3, "duchy": 4, "kingdom": 5, "empire": 6}


def honour(realm, r, renown, why, legacy="ruler"):
    """Renown, fame and legacy for a ruler's great deed; prestige for their house."""
    r.renown += renown
    from .lineage import gain_legacy
    gain_legacy(realm, r, legacy, renown * 0.5)
    if r.house is not None:
        realm.houses[r.house].prestige += renown * 0.5
    r.deeds[why] = r.deeds.get(why, 0) + 1


# ---------------------------------------------------------------- limits from skill
def limits_year(realm):
    for k in realm.kingdoms:
        if realm.extinct[k.id] or k.ruler is None:
            continue
        r = realm.chars[k.ruler]
        from .court import perk_value
        k.vassal_limit = RANK_VASSALS.get(k.rank, 5) + 5 + r.attr("ste") // 2 + perk_value(r, "vassal")
        k.domain_limit = RANK_DOMAIN.get(k.rank, 4) + r.attr("ste") // 4 + perk_value(r, "domain")
        vassals = sum(1 for d in realm.duchies.values() if d["ctrl"] == k.id and d["holder"] not in (None, r.id))
        demesne = sum(1 for t in realm.alive_towns(k.id) if realm.duchies.get(getattr(t, "duchy", None), {}).get(
            "holder") in (None, r.id))
        k.vassals, k.demesne = vassals, demesne
        k.over_vassals = max(0, vassals - k.vassal_limit)
        k.over_domain = max(0, demesne - k.domain_limit * 4)
        k.tax_mult = max(0.4, 1 - 0.02 * k.over_vassals - 0.01 * k.over_domain)


# ---------------------------------------------------------------- casus belli and war
def cb_against(realm, k, o):
    """The best reason this realm has for war on another, or None."""
    from .resources import lacks
    r = realm.chars.get(k.ruler)
    if r is None:
        return None
    # vengeance: someone in that realm killed one this ruler loved
    from .heart import vengeance_target
    v, whom = vengeance_target(realm, r)
    if v is not None and v.realm == o.id:
        dk = nearest_duchy(realm, k, o)
        return (f"vengeance for {whom}", {"duchy": dk, "avenge": v.id})
    # a de jure claim: a duchy the other holds that lies in a kingdom title this realm controls
    mine = {kt for kt, kd in realm.ktitles.items() if kd["ctrl"] == k.id}
    for dk, d in realm.duchies.items():
        if d["ctrl"] == o.id and d["kt"] in mine:
            return ("de jure claim", {"duchy": dk})
    # subjugation: a great chief over a small tribe of the same people
    if k.gov in ("tribal", "clan") and o.rank in ("tribe", "chiefdom") and o.people == k.people and \
            getattr(k, "army_power", 0) > 3 * max(1.0, getattr(o, "army_power", 0)):
        return ("subjugation", {"subjugate": True})
    # a claim through blood: a kinsman of this ruler has a claim on the other's throne
    if o.house is not None and r.house is not None:
        kin = [realm.chars[i] for i in realm.living if realm.chars[i].house == r.house and realm.chars[i].adult(realm.year)
               and realm.chars[i].spouse is not None and realm.chars[realm.chars[i].spouse].house == o.house
               and realm.chars[i].id != r.id]
        if kin and "ambitious" in r.personality:
            return ("claim", {"claimant": kin[0].id})
    target = nearest_duchy(realm, k, o)
    if target is None:
        return None
    if k.faith != o.faith and (getattr(k, "laws", {}).get("tolerance") == "intolerant"
                               or "holy war" in realm.religions[k.faith].tenets):
        return ("holy war", {"duchy": target})
    want = lacks(k, o)
    if want:
        return (f"resource war for {want[0]}", {"duchy": resource_duchy(realm, o, want[0]) or target})
    if getattr(k, "agenda", None) == "conquest" or "ambitious" in r.personality and "wrathful" in r.personality:
        return ("conquest", {"duchy": target})
    return None


def nearest_duchy(realm, k, o):
    cap = realm.capital_of(k.id)
    best, bd = None, None
    for dk, d in realm.duchies.items():
        if d["ctrl"] != o.id:
            continue
        towns = [t for t in realm.alive_towns(o.id) if getattr(t, "duchy", None) == dk]
        if not towns or cap is None:
            continue
        dist = min(math.hypot(t.pos[0] - cap.pos[0], t.pos[1] - cap.pos[1]) for t in towns)
        if bd is None or dist < bd:
            best, bd = dk, dist
    return best


def resource_duchy(realm, o, res):
    for t in realm.alive_towns(o.id):
        if getattr(t, "res", {}).get(res):
            return getattr(t, "duchy", None)
    return None


def declare(realm, a, b, cb):
    name, goal = cb
    key = (min(a, b), max(a, b))
    realm.wars[key] = {"attacker": a, "defender": b, "cb": name, "goal": goal, "start": realm.year, "score": 0.0}
    goal_txt = ""
    if "duchy" in goal and goal["duchy"] in realm.duchies:
        goal_txt = f" for {realm.duchies[goal['duchy']]['name']}"
    elif goal.get("subjugate"):
        goal_txt = " to make it a tributary"
    elif "claimant" in goal:
        goal_txt = f" to put {realm.chars[goal['claimant']].full_name()} on its throne"
    return f" ({name}{goal_txt})"


def war_of(realm, a, b):
    return realm.wars.get((min(a, b), max(a, b)))


def battle_result(realm, winner, loser, captured=None):
    w = war_of(realm, winner, loser)
    if w is None:
        return
    sign = 1 if winner == w["attacker"] else -1
    pts = 8.0
    if captured is not None:
        pts += 25 if getattr(captured, "duchy", None) == w["goal"].get("duchy") else 10
    w["score"] += sign * pts


def wars_year(realm):
    rng = realm.rng
    y = realm.year
    for key, w in list(realm.wars.items()):
        a, b = w["attacker"], w["defender"]
        if not realm.war[a, b]:                                        # ended elsewhere (a calamity, the Concord)
            del realm.wars[key]
            continue
        if realm.extinct[a] or realm.extinct[b]:
            end_war(realm, key, w, "dissolved")
            continue
        dk = w["goal"].get("duchy")
        if dk is not None and realm.duchies.get(dk, {}).get("ctrl") == a:
            w["score"] += 10                                          # holding the goal
        occ_a = [t for t in realm.alive_towns(b) if getattr(t, "occupied_by", None) == a]
        occ_b = [t for t in realm.alive_towns(a) if getattr(t, "occupied_by", None) == b]
        w["score"] += sum(15 if getattr(t, "duchy", None) == dk else 5 for t in occ_a) - 5 * len(occ_b)
        pa = getattr(realm.kingdoms[a], "army_power", 1.0)
        pb = getattr(realm.kingdoms[b], "army_power", 1.0)
        w["score"] += 15 * (pa - pb) / max(1.0, pa + pb) + rng.normal(0, 4)   # sieges and the grind of armies
        if y - w["start"] > 10:
            w["score"] -= 2                                           # the defender's ticking warscore
        if w["score"] >= 100:
            end_war(realm, key, w, "attacker")
        elif w["score"] <= -100:
            end_war(realm, key, w, "defender")
        elif y - w["start"] > 25 and abs(w["score"]) < 40:
            end_war(realm, key, w, "white")


def end_war(realm, key, w, outcome):
    from .governance import add_tyranny
    a, b = w["attacker"], w["defender"]
    ka, kb = realm.kingdoms[a], realm.kingdoms[b]
    realm.war[a, b] = realm.war[b, a] = False
    realm.truce[a, b] = realm.truce[b, a] = realm.year + 15
    from .warfare import end_occupations
    end_occupations(realm, a, b)
    realm.relation[a, b] = realm.relation[b, a] = -20
    del realm.wars[key]
    ra, rb = realm.chars.get(ka.ruler), realm.chars.get(kb.ruler)
    if ra is not None and rb is not None:
        from .opinion import remember
        loser, winner = (rb, ra) if outcome == "attacker" else (ra, rb) if outcome == "defender" else (None, None)
        if loser is not None:
            remember(realm, loser, winner, "defeated me in war", -40, decay=0.5)
            from .heart import add_stress, reluctance
            add_stress(realm, loser, reluctance(loser, "defeat"), "a lost war")
    yrs = realm.year - w["start"]
    record = {"year": realm.year, "attacker": ka.name, "defender": kb.name, "cb": w["cb"], "years": yrs,
              "outcome": outcome}
    if outcome == "attacker":
        goal = w["goal"]
        if goal.get("duchy") is None and "avenge" in goal:
            what = "forces its terms"
        elif "duchy" in goal and goal["duchy"] in realm.duchies:
            towns = [t for t in realm.alive_towns(b) if getattr(t, "duchy", None) == goal["duchy"]]
            if towns:
                from .governance import lose_lordship
                for t in towns:
                    lose_lordship(realm, t)
                    t.faction, t.capital = a, False
                    realm.town_moved(t)
                what = f"takes {realm.duchies[goal['duchy']]['name']} ({len(towns)} towns)"
            else:                                                       # another got there first: tribute instead
                pay = indemnity(realm, kb)
                kb.gold -= pay
                ka.gold += pay
                from .events import book
                book(realm, b, "indemnity out", -pay, f"pays {ka.name} for a lost war")
                book(realm, a, "indemnity in", pay, f"paid by {kb.name}")
                what = (f"finds {realm.duchies[goal['duchy']]['name']} already lost to another, and takes "
                        f"{pay:,.0f} gold in tribute instead")
        elif goal.get("subjugate"):
            kb.overlord = a
            what = f"makes {kb.name} its tributary"
        elif goal.get("freedom"):
            ka.overlord = None
            what = "wins its freedom"
        else:
            claimant = realm.chars.get(goal.get("claimant"))
            from .dynasty import crown, end_reign
            if claimant is not None and claimant.alive and rb is not None:
                realm.kill(rb, f"was deposed by {ka.name} and did not survive it", quiet=True, by=ra)
                end_reign(realm, kb, f"deposed by {ka.name}")
                claimant.realm = b
                crown(realm, kb, claimant, "raised up when the line failed")
                kb.overlord = a
                what = f"puts {claimant.full_name()} on the throne of {kb.name}, a puppet now"
            else:
                what = "wins, but has no one to crown"
        av = realm.chars.get(goal.get("avenge"))
        if av is not None and av.alive and ra is not None:
            from .heart import avenged
            whom = ra.vendettas.get(av.id, (0, "their beloved"))[1] if hasattr(ra, "vendettas") else "their beloved"
            realm.kill(av, f"was executed in vengeance by {ra.full_name()}", by=ra)
            avenged(realm, ra, av, whom)
            what += f", and {av.full_name()} pays with their life"
        realm.say(f"**{ka.name} wins its {w['cb']}** against {kb.name} after {yrs} years and {what}.")
        if ra is not None:
            honour(realm, ra, 100 + 10 * yrs, "war")
            if getattr(ra, "rival", None) == kb.ruler:
                honour(realm, ra, 300, "rival")
                realm.say(f"{realm.who(ra)} has humbled their rival, {rb.full_name() if rb else kb.name}.")
        record["result"] = what
    elif outcome == "defender":
        if ra is not None:
            ra.legitimacy = max(0.0, (ra.legitimacy or 50) - 10)
        pay = indemnity(realm, ka)
        ka.gold -= pay
        kb.gold += pay
        from .events import book
        book(realm, a, "indemnity out", -pay, f"pays {kb.name} for a lost war")
        book(realm, b, "indemnity in", pay, f"paid by {ka.name}")
        if rb is not None:
            honour(realm, rb, 80 + 5 * yrs, "war")
        if w["goal"].get("freedom"):
            ka.overlord = b
        realm.say(f"{kb.name} throws back the {w['cb']} of {ka.name} after {yrs} years; {ka.name} pays "
                  f"{pay:,.0f} gold.")
        record["result"] = f"defender won; {pay:,.0f} gold paid"
    elif outcome == "white":
        realm.say(f"The {w['cb']} between {ka.name} and {kb.name} ends in a white peace after {yrs} years.",
                  minor=True)
        record["result"] = "white peace"
    realm.war_log.append(record)


def ambitions_year(realm):
    """Rulers with a reason and the strength go to war for it, without waiting for hatred."""
    rng = realm.rng
    for k in realm.kingdoms:
        if realm.extinct[k.id] or k.ruler is None or getattr(k, "overlord", None) is not None:
            continue
        r = realm.chars[k.ruler]
        from .heart import vengeance_target
        v, _ = vengeance_target(realm, r)
        urge = 0.45 if v is not None else 0.08 * (1.3 if getattr(r, "happy", 0) < -30 and "wrathful" in r.personality
                                                   else 1.0)
        if sum(1 for o in range(realm.npol) if realm.war[k.id, o]) >= 2 or rng.random() > urge:
            continue
        if not r.adult(realm.year):
            continue
        for o in realm.kingdoms:
            if o is k or realm.extinct[o.id] or o.ruler is None or realm.war[k.id, o.id] or o.id in k.allies:
                continue
            if realm.year < realm.truce[k.id, o.id] or getattr(o, "overlord", None) == k.id:
                continue
            if k.gold < 0 and not avenging_pre(realm, r, o):
                continue                                              # an empty treasury does not start wars
            if not contact(realm, k, o):
                continue
            from .opinion import opinion as _op
            if realm.chars.get(o.ruler) is not None and _op(realm, r, realm.chars[o.ruler]) > 30 and \
                    realm.rng.random() < 0.7:
                continue                                              # one does not lightly attack a friend
            avenging = v is not None and v.realm == o.id
            if not avenging and getattr(k, "army_power", 0) < 0.8 * getattr(o, "army_power", 1):
                continue
            # one does not make war on the realm where one's child, spouse or lover lives
            if not avenging and any(realm.chars[i].realm == o.id and realm.chars[i].alive
                                    for i in getattr(r, "loves", {}) if i in realm.chars):
                continue
            cb = cb_against(realm, k, o)
            if cb is None:
                continue
            start_war(realm, k.id, o.id, cb)
            break


def avenging_pre(realm, r, o):
    from .heart import vengeance_target
    v, _ = vengeance_target(realm, r)
    return v is not None and v.realm == o.id


def contact(realm, k, o):
    lk = {t.landmass for t in realm.alive_towns(k.id)}
    lo = {t.landmass for t in realm.alive_towns(o.id)}
    if lk & lo:
        return True
    from .seafaring import naval_contact
    return naval_contact(realm, k.id, o.id)


def indemnity(realm, k):
    """What a beaten realm pays: half its treasury, or a year and a half of its revenue if that is more, borrowed if
    need be."""
    revenue = getattr(k, "income", 0.0) * getattr(k, "tax_now", 0.3) * getattr(k, "collect", 1.0)
    return max(0.0, 0.5 * k.gold, 1.5 * revenue)


def last_stands(realm):
    """A people near its end fights like one: when a people holds a small part of what it once held, its last realms
    fight on home ground with everything they have."""
    held = {}
    for t in realm.alive_towns():
        held[t.culture] = held.get(t.culture, 0) + 1
    peak = getattr(realm, "people_peak", None)
    if peak is None:
        peak = realm.people_peak = {}
    for p, n in held.items():
        peak[p] = max(peak.get(p, 0), n)
    for k in realm.kingdoms:
        if realm.extinct[k.id]:
            continue
        n, top = held.get(k.people, 0), peak.get(k.people, 0)
        stand = top >= 10 and n <= max(8, 0.2 * top)
        if stand and not getattr(k, "last_stand", False):
            realm.say(f"**A last stand.** The {realm.pname(k.id)} hold only {n} towns of the {top} their people once "
                      f"held; in {k.name} every hand takes up a spear.")
        k.last_stand = stand
        if stand:
            k.battle_mult = getattr(k, "battle_mult", 1.0) * 1.6


def start_war(realm, a, b, cb):
    realm.war[a, b] = realm.war[b, a] = True
    realm.war_since[a, b] = realm.war_since[b, a] = realm.year
    realm.weariness[a, b] = realm.weariness[b, a] = 0
    txt = declare(realm, a, b, cb)
    ka, kb = realm.kingdoms[a], realm.kingdoms[b]
    r = realm.chars[ka.ruler]
    from .heart import add_stress, reluctance
    add_stress(realm, r, reluctance(r, "war"), "going to war")
    pile = getattr(realm, "recent_decl", None)
    if pile is None:
        pile = realm.recent_decl = {}
    key = (b, txt)
    joined = realm.year - pile.get(key, -99) <= 15
    pile[key] = realm.year
    if joined:
        realm.say(f"{realm.who(r)} of {ka.name} joins the fight against {kb.name}{txt}.", minor=True)
    else:
        realm.say(f"**War.** {realm.who(r)} of {ka.name} declares war on {kb.name}{txt}.")
    from .warfare import raise_for_war
    raise_for_war(realm, a, b)
    # tributaries answer their overlord's call to arms
    for t in realm.kingdoms:
        if getattr(t, "overlord", None) == a and not realm.extinct[t.id] and t.id != b:
            realm.war[t.id, b] = realm.war[b, t.id] = True
            realm.war_since[t.id, b] = realm.war_since[b, t.id] = realm.year


# ---------------------------------------------------------------- tributaries, rivals
def tributaries_year(realm):
    rng = realm.rng
    for k in realm.kingdoms:
        ov = getattr(k, "overlord", None)
        if ov is None:
            continue
        if realm.extinct[k.id] or realm.extinct[ov]:
            k.overlord = None
            continue
        o = realm.kingdoms[ov]
        tribute = max(0.0, k.gold) * 0.15
        k.gold -= tribute
        o.gold += tribute
        from .events import book
        book(realm, k.id, "tribute out", -tribute)
        book(realm, ov, "tribute in", tribute)
        k.allies.add(ov)
        o.allies.add(k.id)
        if getattr(k, "army_power", 0) > 0.9 * getattr(o, "army_power", 1) and rng.random() < 0.05:
            k.overlord = None
            start_war(realm, k.id, ov, ("independence", {"freedom": True}))
            realm.say(f"{k.name} refuses tribute and fights to be free of {o.name}.")


def rivals_year(realm):
    for k in realm.kingdoms:
        if realm.extinct[k.id] or k.ruler is None:
            continue
        r = realm.chars[k.ruler]
        riv = realm.chars.get(getattr(r, "rival", None))
        if riv is not None and riv.alive and riv.ruled is not None and riv.ruled[2] is None:
            continue
        cands = [o for o in realm.kingdoms if o is not k and not realm.extinct[o.id] and o.ruler is not None
                 and contact(realm, k, o) and o.rank == k.rank]
        if cands:
            o = max(cands, key=lambda o: len(realm.alive_towns(o.id)))
            r.rival = o.ruler
            realm.chars[o.ruler].rival = getattr(realm.chars[o.ruler], "rival", None) or r.id


# ---------------------------------------------------------------- great decisions
def great_decisions(realm):
    y = realm.year
    towns = realm.alive_towns()
    total = len(towns)
    by_land = {}
    for t in towns:
        by_land.setdefault(t.landmass, []).append(t)
    for k in realm.kingdoms:
        if realm.extinct[k.id] or k.ruler is None:
            continue
        done = getattr(k, "done", set())
        k.done = done
        r = realm.chars[k.ruler]
        mine = realm.alive_towns(k.id)
        subs = [o for o in realm.kingdoms if getattr(o, "overlord", None) == k.id and not realm.extinct[o.id]]
        # unite a continent
        for lid, ts in by_land.items():
            if realm.lands[lid]["kind"] != "continent" or len(ts) < 40 or ("unite", lid) in done:
                continue
            held = sum(1 for t in ts if t.faction == k.id or realm.kingdoms[t.faction] in subs)
            if held >= 0.75 * len(ts):
                done.add(("unite", lid))
                place = land_name(realm, lid)
                k.great = getattr(k, "great", []) + [place]
                if k.rank != "empire":
                    k.rank = "empire"
                k.name = f"the Great Empire of {place}"
                honour(realm, r, 2000, "unification")
                realm.say(f"**{realm.who(r)} unites {place}.** From shore to shore it answers to one throne: "
                          f"{k.name}.")
                realm.decisions.append((y, k.id, f"united {place}"))
                from .legends import myth
                myth(realm, f"The unification of {place}", f"{r.full_name()} united {place} in year {y}.")
        # unite a people
        same = [o for o in realm.kingdoms if o.people == k.people and not realm.extinct[o.id]]
        if ("people",) not in done and len([o for o in realm.kingdoms if o.people == k.people]) >= 4 and \
                all(o is k or o in subs for o in same):
            done.add(("people",))
            honour(realm, r, 800, "unification")
            r.titles.append(f"High Sovereign of the {PEOPLES[k.people]} from year {y}")
            realm.say(f"**{realm.who(r)} is High Sovereign of all the {PEOPLES[k.people]}**: every realm of their "
                      f"people kneels or pays tribute.")
            realm.decisions.append((y, k.id, f"united the {PEOPLES[k.people]}"))
        # hegemony
        if ("hegemony",) not in done and total > 200 and len(mine) + sum(len(realm.alive_towns(o.id)) for o in subs) \
                >= 0.35 * total:
            done.add(("hegemony",))
            k.rank = "hegemony"
            k.name = f"the Hegemony of {getattr(k, 'place', k.short)}"
            honour(realm, r, 4000, "hegemony")
            realm.say(f"**A hegemony.** {k.name}: a third of the world's towns answer to {realm.who(r)}.")
            realm.decisions.append((y, k.id, "proclaimed a hegemony"))


COHESION = {"tribal": 12, "clan": 20, "feudal": 45, "theocracy": 40, "republic": 30, "administrative": 90}


def overstretch(realm):
    """A realm can hold only so much: chiefs and clans far from the seat drift away when it grows past what its form
    of rule, its ruler's gifts and its legitimacy can hold together. Fast-growing tribal peoples feel it first."""
    from .titles import secede
    rng = realm.rng
    for k in realm.kingdoms:
        if realm.extinct[k.id] or k.ruler is None:
            continue
        r = realm.chars.get(k.ruler)
        towns = realm.alive_towns(k.id)
        inn = realm.cultures[k.people].innovations
        admin = 15 * ("Written Law" in inn) + 25 * ("Royal Chancery" in inn) + 40 * ("Bureaucracy" in inn)
        cap = (COHESION.get(getattr(k, "gov", "tribal"), 30) + admin) * (1 + (r.attr("dip") + r.attr("ste")) / 40) \
            * max(0.4, (r.legitimacy or 50) / 60)
        if len(towns) <= cap or rng.random() > 0.05 * (len(towns) / cap - 1):
            continue
        seat = realm.capital_of(k.id)
        if seat is None:
            continue
        held = [(dk, d) for dk, d in realm.duchies.items() if d["ctrl"] == k.id and d["holder"] not in (None, r.id)
                and d["holder"] in realm.chars and realm.chars[d["holder"]].alive]
        held = [(dk, d) for dk, d in held if sum(getattr(t, "duchy", None) == dk for t in towns) >= 3
                and {"ambitious", "greedy", "arrogant", "vengeful"} & realm.chars[d["holder"]].personality]
        if not held:
            continue
        far = lambda dk: min(dist_xy(t.pos, seat.pos) for t in towns if getattr(t, "duchy", None) == dk)
        dk, d = max(held, key=lambda x: far(x[0]))
        leader = realm.chars[d["holder"]]
        realm.say(f"**The realm frays.** {k.name} has grown past what {realm.who(r)} can hold; "
                  f"{realm.who(leader)} takes {d['name']} and goes their own way.")
        secede(realm, k, {dk}, leader, 0)


def dist_xy(a, b):
    return math.hypot(a[0] - b[0], a[1] - b[1])


def statecraft_year(realm):
    last_stands(realm)
    overstretch(realm)
    _statecraft_year(realm)


def _statecraft_year(realm):
    limits_year(realm)
    rivals_year(realm)
    tributaries_year(realm)
    ambitions_year(realm)
    wars_year(realm)
    great_decisions(realm)
