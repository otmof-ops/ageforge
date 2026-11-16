"""Demons and the Veil.

ORDINARY RIFTS. Now and then the Veil frays at a ley-nexus and a single demon slips through: most of the third
tier, some of the fourth, rarely the fifth, and never more than two loose in the world at once. A fifth-tier demon
is a catastrophe for a region; nothing higher comes through a fray.

THE SUNDERING. The Veil does not fail by itself. It is torn, for a reason: a necromancer ruler (or a ruler with a
necromancer at their council) of the fourth tier or above, ambitious or cruel, losing a war or losing their throne,
may tear it open to save themselves; and a world calamity abroad strains it until it may give way. Then for twenty
to forty years gates open along the ley lines of every settled land, demons of the third to sixth tier pour through,
and a Gate-Lord of the seventh tier walks out to lead them. It ends when the Gate-Lord is slain, or when the Veil
heals of itself. The realm that tore it is the enemy of every other."""
from .data import NECRO

DEMON_POWER = {3: 450, 4: 1100, 5: 2400, 6: 4500, 7: 9000}
RIFT_TIERS = [(3, 0.6), (4, 0.3), (5, 0.1)]
GATE_TIERS = [(3, 0.45), (4, 0.35), (5, 0.15), (6, 0.05)]
LOOSE_CAP = 2
LORD_A = ["Vel", "Ashk", "Mor", "Thul", "Zag", "Oru", "Nhel"]
LORD_B = ["goroth", "amaz", "ithrax", "uzeal", "aroth", "emnon"]


def tier_of(power):
    return 3 if power < 700 else 4 if power < 1700 else 5 if power < 3500 else 6 if power < 7000 else 7


def roll_tier(rng, table):
    r, acc = rng.random(), 0.0
    for t, p in table:
        acc += p
        if r < acc:
            return t
    return table[-1][0]


def crisis(realm):
    g = getattr(realm, "gates", None)
    return g if g is not None and g.get("open") else None


def loose_demons(realm):
    return sum(1 for m in realm.monsters if m.alive and m.kind == "demon" and not getattr(m, "gate", False))


def may_rift(realm):
    """Whether an ordinary fray can let one more demon through."""
    return loose_demons(realm) < LOOSE_CAP


def rift_power(realm):
    return DEMON_POWER[roll_tier(realm.rng, RIFT_TIERS)] * realm.rng.uniform(0.85, 1.15)


def demon_cap(realm, power):
    """No ordinary demon above the fifth tier; in a Sundering, the gates set their own."""
    return power if crisis(realm) else min(power, 3499.0)


def gates_year(realm):
    rng = realm.rng
    g = crisis(realm)
    if g is None:
        cause = torn_by(realm)
        if cause is not None:
            open_gates(realm, *cause)
        return
    lord = next((m for m in realm.monsters if m.mid == g["lord"]), None)
    if lord is None or not lord.alive:
        close(realm, g, f"with {g['lord_name']} slain, the gates fall silent one by one")
        return
    if realm.year >= g["end"]:
        close(realm, g, "the Veil heals of itself, and the last gates close")
        return
    for _ in range(int(rng.integers(1, 4))):
        gate(realm)


def torn_by(realm):
    """Who tears the Veil, and why: a desperate necromancer ruler, or the strain of a world calamity."""
    rng = realm.rng
    if realm.year < 150:
        return None
    for k in realm.kingdoms:
        if realm.extinct[k.id] or k.ruler is None:
            continue
        r = realm.chars[k.ruler]
        mage = realm.chars.get(k.council.get("mage")) if getattr(k, "council", None) else None
        dark = any(x is not None and x.school() == NECRO and x.tier() >= 4 for x in (r, mage)) or r.undead
        if not dark or not ({"ambitious", "cruel", "arbitrary"} & r.personality):
            continue
        losing = (r.legitimacy or 50) < 25 or sum(realm.war[k.id]) >= 2
        if losing and rng.random() < 0.01:
            return k, r, f"{realm.who(r)} of {k.name}, desperate, tears open the Veil to save their throne"
    if any(m.alive and m.kind == "calamity" and getattr(m, "world", False) for m in realm.monsters) and \
            rng.random() < 0.004:
        return None, None, "the strain of a calamity abroad in the world tears the Veil"
    return None


def open_gates(realm, k, r, why):
    rng = realm.rng
    land = realm.terrain != 0
    cells = realm.remote_cells(land & (realm.mana > 0.5))
    if not len(cells):
        return
    if k is not None and realm.capital_of(k.id) is not None:
        seat = realm.capital_of(k.id).pos
        near = [c for c in cells if abs(c[0] - seat[0]) + abs(c[1] - seat[1]) < 60]
        cells = near or cells
    y, x = cells[int(rng.integers(len(cells)))]
    name = f"{rng.choice(LORD_A)}{rng.choice(LORD_B)}, Lord of the Gates"
    lord = realm.new_monster("demon", (int(y), int(x)), DEMON_POWER[7], name=name)
    lord.gate = True
    realm.gates = {"open": True, "start": realm.year, "end": realm.year + int(rng.integers(20, 41)),
                   "lord": lord.mid, "lord_name": name, "opener": k.id if k is not None else None, "count": 0,
                   "history": getattr(realm, "gates", {}).get("history", [])}
    realm.say(f"**The Sundering.** {why}. Gates open along the ley lines of every land, and {name}, a demon of the "
              f"seventh tier, walks out of the first of them.")
    if k is not None:
        for o in range(realm.npol):
            if o != k.id and not realm.extinct[o]:
                realm.relation[k.id, o] = realm.relation[o, k.id] = -100
        r.legitimacy = max(0.0, (r.legitimacy or 50) - 30)


def gate(realm):
    rng = realm.rng
    settled = {t.landmass for t in realm.alive_towns()}
    land = (realm.terrain != 0) & (realm.mana > 0.4)
    cells = realm.remote_cells(land)
    cells = [c for c in cells if int(realm.landmass[c[0], c[1]]) in settled]
    if not cells:
        return
    y, x = cells[int(rng.integers(len(cells)))]
    t = roll_tier(rng, GATE_TIERS)
    m = realm.new_monster("demon", (int(y), int(x)), DEMON_POWER[t] * rng.uniform(0.85, 1.15))
    m.gate = True
    realm.gates["count"] += 1
    if t >= 5:
        realm.say(f"A gate tears open near {realm.nearest_town_name(m.pos)}, and {m.name} comes through.")


def close(realm, g, how):
    g["open"] = False
    g.setdefault("history", []).append((g["start"], realm.year, g["lord_name"], g["count"]))
    realm.say(f"**The gates close.** After {realm.year - g['start']} years and {g['count']} gates, {how}.")
