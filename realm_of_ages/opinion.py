"""Opinion, after the feudal model: what every person thinks of every other, and why.

Opinion of A towards B is a sum of named terms:
  - base terms: shared and opposed personality traits, B's diplomacy, beauty, culture and faith (a hostile
    faith counts double), kinship, marriage, a shared dynasty, friendship and rivalry;
  - liege terms, when B rules A's realm: legitimacy, tyranny, a short reign, crown authority, the court's
    grandeur, too many vassals for B to manage, how far A's politics are from the realm's, and, for a powerful
    vassal, whether B has given them the council seat or court office they expect;
  - memories: events that happened between them or to their kin (a title granted or revoked, a seat given or
    taken, a marriage, ennoblement, a kinsman executed or murdered, a lost war, a usurped throne), each with a
    value that fades by a set amount a year.

Dread is fear of a ruler: tyranny, cruelty and the Fear lifestyle. A vassal whose boldness is below their
liege's dread is intimidated: they will not rise against them, however they feel. A hook is leverage: a
secret or a debt. A ruler with a hook on a vassal can keep them out of factions."""
from .data import PERSONALITY

OPPOSED = {}
for a, b in PERSONALITY:
    OPPOSED[a], OPPOSED[b] = b, a


# ---------------------------------------------------------------- memories
def remember(realm, who, about, name, value, decay=2.0):
    """Who remembers something about another person: name, opinion value, how fast it fades a year."""
    if who is None or about is None or who.id == about.id:
        return
    key = (who.id, about.id)
    if getattr(who, "tracked", False):
        from .events import journal
        journal(realm, who, f"Will remember that {about.full_name()} {name} ({value:+.0f}).", "heart")
    mods = realm.op_mods.setdefault(key, [])
    for m in mods:
        if m[0] == name:
            m[1] = value if abs(value) > abs(m[1]) else m[1]
            return
    mods.append([name, float(value), float(decay)])


def remember_kin(realm, victim, culprit, name, value):
    """Everyone close to the victim remembers what the culprit did."""
    if culprit is None:
        return
    kin = set(victim.children) | {p for p in (victim.father, victim.mother, victim.spouse) if p is not None}
    for i in kin:
        c = realm.chars.get(i)
        if c is not None and c.alive:
            remember(realm, c, culprit, name, value, decay=1.0)


def fade(realm):
    for key in list(realm.op_mods):
        a, b = key
        if not realm.chars[a].alive or not realm.chars[b].alive:
            del realm.op_mods[key]
            continue
        mods = realm.op_mods[key]
        for m in mods:
            if m[1] > 0:
                m[1] = max(0.0, m[1] - m[2])
            else:
                m[1] = min(0.0, m[1] + m[2])
        realm.op_mods[key] = [m for m in mods if abs(m[1]) >= 1]
        if not realm.op_mods[key]:
            del realm.op_mods[key]


# ---------------------------------------------------------------- the sum
def kin(a, b):
    if b.id in (a.father, a.mother) or a.id in (b.father, b.mother):
        return "parent or child"
    if a.father is not None and a.father == b.father or a.mother is not None and a.mother == b.mother:
        return "sibling"
    return None


def opinion(realm, a, b, breakdown=False):
    """What a thinks of b. With breakdown=True, also the list of (term, value)."""
    terms = []
    for t in a.personality:
        if t in b.personality:
            terms.append((f"shares their {t} nature", 10))
        elif OPPOSED.get(t) in b.personality:
            terms.append((f"{t}, and they are {OPPOSED[t]}", -10))
    terms.append(("their diplomacy", round((b.attr("dip") - 6) * 1.5)))
    if "comely" in b.genetic:
        terms.append(("beautiful", 5))
    terms.append(("same people" if a.people == b.people else "a foreigner", 5 if a.people == b.people else -10))
    if a.religion == b.religion:
        terms.append(("same faith", 10))
    else:
        hostile = "holy war" in realm.religions[a.religion].tenets
        terms.append(("a heathen" if hostile else "another faith", -30 if hostile else -15))
    k = kin(a, b)
    if k:
        terms.append((k, 20))
    if a.spouse == b.id:
        terms.append(("spouse", 25))
    if a.house is not None and b.house is not None:
        ha, hb = realm.houses[a.house], realm.houses[b.house]
        if getattr(ha, "dynasty", ha.id) == getattr(hb, "dynasty", hb.id):
            terms.append(("same dynasty", 10))
    rel = getattr(a, "relations", {}).get(b.id)
    if rel == "friend":
        terms.append(("friend", 60))
    elif rel == "rival":
        terms.append(("rival", -60))
    if getattr(a, "rival", None) == b.id:
        terms.append(("rival ruler", -40))
    kd = realm.kingdoms[a.realm] if a.realm is not None and a.realm < len(realm.kingdoms) else None
    if kd is not None and kd.ruler == b.id and a.id != b.id:
        terms += liege_terms(realm, a, b, kd)
    for name, val, _ in realm.op_mods.get((a.id, b.id), ()):
        terms.append((name, round(val)))
    total = int(sum(v for _, v in terms if v))
    return (total, [t for t in terms if t[1]]) if breakdown else total


def liege_terms(realm, a, b, k):
    y = realm.year
    out = []
    leg = b.legitimacy if b.legitimacy is not None else 50
    out.append(("legitimacy", round((leg - 50) / 2.5)))
    if b.tyranny > 5:
        out.append(("tyranny", -round(b.tyranny / 2)))
    if b.ruled is not None:
        reign = y - b.ruled[1]
        if reign < 10:
            out.append(("short reign", -(10 - reign) * 2))
    crown = getattr(k, "laws", {}).get("crown", 1)
    if crown > 1 and (getattr(a, "duchies", None) or a.lordships):
        out.append(("crown authority", -5 * (crown - 1)))
    if getattr(k, "court_mod", 0):
        out.append(("court grandeur", round(k.court_mod)))
    if getattr(k, "over_vassals", 0):
        out.append(("too many vassals", -3 * k.over_vassals))
    if getattr(realm, "cultures", None):
        from .civics import agreement
        out.append(("how the realm is ruled", round(30 * (agreement(realm, a, k) - 0.55))))
    court_pos = set((getattr(k, "court", None) or {}).get("pos", {}).values())
    if a.id in getattr(k, "powerful", ()):
        seated = a.id in k.council.values() or a.id in court_pos
        out.append(("given office, as a powerful vassal deserves" if seated else
                    "a powerful vassal kept from the council", 15 if seated else -25))
    elif a.id in k.council.values():
        out.append(("on the council", 10))
    from .court import perk_value
    if perk_value(b, "opinion"):
        out.append(("their authority", perk_value(b, "opinion")))
    return out


# ---------------------------------------------------------------- dread, boldness, hooks
def dread(realm, r):
    from .court import perk_value
    return r.tyranny * 0.6 + 15 * ("cruel" in r.personality) + 100 * perk_value(r, "fear") + r.attr("intr")


def boldness(c):
    return 20 + 25 * ("brave" in c.personality) - 20 * ("craven" in c.personality) + 10 * ("ambitious" in c.personality) \
        + c.attr("mar")


def intimidated(realm, c, r):
    return dread(realm, r) > boldness(c)


def hooked(realm, r, c):
    return getattr(r, "hooks", {}).get(c.id) is not None


def gain_hook(realm, r, c, strength="weak", why=""):
    r.hooks = getattr(r, "hooks", {})
    r.hooks[c.id] = strength
    realm.say(f"{realm.who(r)} gains a {strength} hook on {realm.who(c)}{why}.", minor=True)


# ---------------------------------------------------------------- the year: friends, rivals, hooks, powerful vassals
def opinion_year(realm):
    rng = realm.rng
    fade(realm)
    for k in realm.kingdoms:
        if realm.extinct[k.id] or k.ruler is None:
            continue
        r = realm.chars[k.ruler]
        dukes = [realm.chars[d["holder"]] for d in realm.duchies.values()
                 if d["ctrl"] == k.id and d["holder"] not in (None, r.id) and realm.chars[d["holder"]].alive]
        uniq = {d.id: d for d in dukes}
        # the powerful vassals: the strongest few, who expect to be heard
        power = {}
        for d in uniq.values():
            power[d.id] = sum(t.pop for t in realm.alive_towns(k.id) if getattr(t, "duchy", None) in getattr(d, "duchies", []))
        n = 3 if len(uniq) < 10 else 5 if len(uniq) < 25 else 8
        k.powerful = set(sorted(power, key=power.get, reverse=True)[:n])
        # the court: friendships and rivalries form among those who see each other
        court = [r] + list(uniq.values()) + [realm.chars[i] for i in k.council.values() if i in realm.chars]
        court = [c for c in {c.id: c for c in court}.values() if c.alive]
        if len(court) >= 2 and rng.random() < 0.3:
            a, b = rng.choice(len(court), 2, replace=False)
            a, b = court[int(a)], court[int(b)]
            o = opinion(realm, a, b) + opinion(realm, b, a)
            a.relations, b.relations = getattr(a, "relations", {}), getattr(b, "relations", {})
            if o > 60 and b.id not in a.relations:
                a.relations[b.id] = b.relations[a.id] = "friend"
                realm.say(f"{realm.who(a)} and {realm.who(b)} become friends.", minor=True)
            elif o < -40 and b.id not in a.relations:
                a.relations[b.id] = b.relations[a.id] = "rival"
                realm.say(f"{realm.who(a)} and {realm.who(b)} become rivals.", minor=True)
        # a ruler with a gift for intrigue finds leverage over their vassals
        spy = realm.chars.get(k.council.get("spymaster"))
        sk = r.attr("intr") + (spy.attr("intr") if spy is not None else 0)
        if uniq and rng.random() < sk / 400:
            target = list(uniq.values())[int(rng.integers(len(uniq)))]
            if not hooked(realm, r, target):
                gain_hook(realm, r, target, "weak", ", from a secret the spymaster dug up")
