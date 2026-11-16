"""Beast taming: a rare gift. Only those born beastspeakers can learn it. They tame wolves, bears, eagles,
griffins, wyverns and, at the summit, a young dragon of the wild. A master at the hundredth rank may make the
last choice: merge their blood with their beast for good. The beast is gone, its power is theirs and their
children's, and taming becomes a thing they did once. A line merged with a dragon carries the dragonbond: its
children can bond dragon hatchlings without any taming at all, and a house of dragonriders on a throne is a
dragon royalty."""
import math

from .data import MERGED, PEOPLES, POWER_GENES
from .data2 import BEASTS
from .skills import skill, use

ORDER = list(BEASTS)


def gifted(c):
    return "beastspeaker" in c.genetic or any(g in c.genetic for g in MERGED.values())


def dragonbound(c):
    return "dragon-merged" in c.genetic


def companion_power(c, year):
    b = getattr(c, "beast", None)
    if not b:
        return 0.0
    age_f = min(1.0, 0.3 + (year - b["since"]) / 40) if b["kind"] == "dragon" else 1.0
    return b["power"] * age_f


def beasts_year(realm):
    rng = realm.rng
    y = realm.year
    riders = {}
    for i in list(realm.living):
        c = realm.chars[i]
        b = getattr(c, "beast", None)
        if b:
            if y - b["since"] > b["life"]:
                realm.say(f"{realm.who(c)}'s {b['kind']}, {b['name']}, dies of age.", minor=b["kind"] != "dragon")
                c.beast = None
            elif b["kind"] == "dragon":
                riders[c.realm] = riders.get(c.realm, 0) + 1
        if not c.adult(y) or not gifted(c):
            continue
        merged = getattr(c, "merged", None)
        if merged is None:
            use(realm, c, "Beast Taming", 0.9 if c.profession == "tamer" else 0.35)
            tame(realm, c)
            maybe_merge(realm, c)
        if dragonbound(c) and not getattr(c, "beast", None):
            bond_hatchling(realm, c)
    # dragon royalty: every dragon ridden by a people's great is worth a host in the field
    for k in realm.kingdoms:
        if riders.get(k.id):
            k.battle_mult *= 1 + 0.15 * riders[k.id]
            k.dragons = riders[k.id]
        else:
            k.dragons = 0


def tame(realm, c):
    rng = realm.rng
    sk = skill(realm, c, "Beast Taming")
    have = getattr(c, "beast", None)
    best = None
    for kind in ORDER:
        need, power, life = BEASTS[kind]
        if sk >= need and (have is None or ORDER.index(kind) > ORDER.index(have["kind"])):
            best = kind
    if best is None or rng.random() > 0.15:
        return
    if best != "dragon" and getattr(realm, "eco", None) is not None:
        from .ecology import tameable_near
        near = tameable_near(realm, c)
        options = [k for k in ORDER if k in near and sk >= BEASTS[k][0]
                   and (have is None or ORDER.index(k) > ORDER.index(have["kind"]))]
        if not options:
            return
        best = options[-1]
    if best == "dragon":
        young = [m for m in realm.monsters if m.alive and m.kind == "dragon" and m.landmass == land_of(realm, c)
                 and (realm.year - m.born) < 120]
        if not young:
            return
        m = young[int(rng.integers(len(young)))]
        if rng.random() > (sk - 80) / 60:
            if rng.random() < 0.3:
                realm.kill(c, f"was burned to ash trying to tame {m.name}", killer=m)
            return
        m.alive = False
        c.beast = {"kind": "dragon", "name": m.name.split(" (")[0], "power": m.adult_base, "since": realm.year,
                   "life": BEASTS["dragon"][2]}
        realm.dragon_riders.append((realm.year, c.id, c.beast["name"], "tamed"))
        realm.say(f"**{realm.who(c)} tames the young dragon {c.beast['name']}** and rides it over "
                  f"{realm.lands[land_of(realm, c)]['name']}.")
    else:
        need, power, life = BEASTS[best]
        name = realm.name((["Ash", "Storm", "Grey", "Night", "Ember", "Frost", "Swift", "Iron", "Moon", "Blood"],
                           ["fang", "claw", "wing", "mane", "heart", "eye", "talon", "hide"]))
        c.beast = {"kind": best, "name": name, "power": power * (0.8 + sk / 250), "since": realm.year, "life": life}
        realm.say(f"{realm.who(c)} tames a {best}, {name}.", minor=best not in ("griffin", "wyvern"))
        from .crafts import gain_fame
        gain_fame(realm, c, "tamer", power / 20)
    c.invalidate()


def land_of(realm, c):
    home = realm.by_id.get(c.home) if c.home is not None else realm.capital_of(c.realm)
    if home is None:
        home = realm.capital_of(c.realm)
    return home.landmass if home is not None else -1


def maybe_merge(realm, c):
    """The final choice: at mastery, a tamer may take their beast into the blood."""
    rng = realm.rng
    b = getattr(c, "beast", None)
    if not b or c.skills.get("Beast Taming", 0) < 100:
        return
    if b["kind"] in ("wolf", "eagle") and rng.random() < 0.7:
        return                                                     # most hold out for a greater beast
    will = 0.04 + 0.06 * ("ambitious" in c.personality) + 0.04 * ("brave" in c.personality) \
        + 0.05 * (c.ruled is not None or (c.house is not None and realm.kingdoms[c.realm].house == c.house))
    if rng.random() > will:
        return
    gene = MERGED[b["kind"]]
    from .lineage import apply_gene
    c.genetic.add(gene)
    c.latent.discard(gene)
    apply_gene(c, gene)
    c.merged = b["kind"]
    c.beast = None
    c.invalidate()
    realm.merges.append((realm.year, c.id, b["kind"], b["name"]))
    house = f" {realm.houses[c.house].name} will carry it." if c.house is not None else ""
    realm.say(f"**{realm.who(c)} merges blood with {b['name']} the {b['kind']}.** The beast is gone into "
              f"them for good; they will never tame another. They {POWER_GENES[gene].replace('carries', 'carry')}.{house}")
    from .crafts import gain_fame
    gain_fame(realm, c, "tamer", 300)


def bond_hatchling(realm, c):
    """The dragonbond: a dragon-merged child can bond a hatchling no tamer could touch."""
    rng = realm.rng
    if rng.random() > 0.08:
        return
    lid = land_of(realm, c)
    young = [m for m in realm.monsters if m.alive and m.kind == "dragon" and (realm.year - m.born) < 60
             and (m.landmass == lid or rng.random() < 0.2)]
    if not young:
        return
    m = young[int(rng.integers(len(young)))]
    m.alive = False
    c.beast = {"kind": "dragon", "name": m.name.split(" (")[0], "power": m.adult_base, "since": realm.year,
               "life": BEASTS["dragon"][2]}
    c.invalidate()
    realm.dragon_riders.append((realm.year, c.id, c.beast["name"], "bonded"))
    royal = c.house is not None and realm.kingdoms[c.realm].house == c.house
    realm.say(f"{'**Dragon royalty.** ' if royal else ''}The hatchling {c.beast['name']} bonds to "
              f"{realm.who(c)}, who has dragon in the blood.", minor=not royal)


def pass_beast(realm, c, heir):
    """A dragon outlives its rider; it goes to a dragon-blooded heir, or back to the wild."""
    b = getattr(c, "beast", None)
    if not b or b["kind"] != "dragon":
        return
    c.beast = None
    if heir is not None and heir.alive and (dragonbound(heir) or skill(realm, heir, "Beast Taming") >= 90) \
            and not getattr(heir, "beast", None):
        heir.beast = b
        heir.invalidate()
        realm.say(f"The dragon {b['name']} accepts {realm.who(heir)} after {c.full_name()} dies.")
        realm.dragon_riders.append((realm.year, heir.id, b["name"], "inherited"))
    else:
        home = realm.by_id.get(c.home) if c.home is not None else realm.capital_of(c.realm)
        if home is not None:
            m = realm.new_monster("dragon", home.pos, b["power"], name=b["name"], quiet=True)
            m.adult_base = b["power"]
            realm.say(f"With {c.full_name()} dead, the dragon {b['name']} goes wild over {home.name}.")
