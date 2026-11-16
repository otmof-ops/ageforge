"""What runs in the blood and what is told: power genetics (latent and awakened) and legacies."""
from .data import (ATTRS, BLOODLINES, DEED_LEGACY, GENETIC, HOUSE_LEGACY_LEVELS, LEGACY, LEGACY_LEVELS, POWER_GENES,
                   ROMAN, STORY)


# ---------------------------------------------------------------- power genetics
def inherit_power_genes(c, parents, rng):
    """A child can carry a power gene without showing it. Carriers pass it on; it shows at birth only sometimes,
    and can wake later in life, most often when its carrier is close to death."""
    for g in BLOODLINES:
        shown = [p for p in parents if g in p.genetic]
        carried = [p for p in parents if g in p.latent]
        if not shown and not carried:
            continue
        p_carry = 1 - (1 - (0.75 if shown else 0)) ** len(shown) * (1 - 0.5) ** len(carried)
        if rng.random() >= p_carry:
            continue
        p_show = 0.45 if len(shown) == 2 else 0.3 if shown else 0.12
        if rng.random() < p_show:
            c.genetic.add(g)
        else:
            c.latent.add(g)


def apply_gene(c, g):
    fx = GENETIC.get(g, {})
    for a in ATTRS:
        c.attrs[a] = c.attrs.get(a, 0) + fx.get(a, 0)
    c.prowess += fx.get("prowess", 0)
    c.talent = min(10.0, c.talent + fx.get("talent", 0))
    c.life *= fx.get("life", 1.0)
    c.invalidate()


def awaken(realm, c, why):
    if not c.latent:
        return None
    g = sorted(c.latent)[int(realm.rng.integers(len(c.latent)))]
    c.latent.discard(g)
    c.genetic.add(g)
    apply_gene(c, g)
    realm.say(f"**The {g.replace('blood', ' blood')} wakes in {realm.who(c)}** {why}. It {POWER_GENES[g]}.")
    realm.awakenings.append((realm.year, c.id, g))
    return g


def seed_gene(realm, c, g):
    """Bring a power gene into the world: a bloodline's first bearer."""
    c.genetic.add(g)
    apply_gene(c, g)


# ---------------------------------------------------------------- legacies
def level_of(points, table=LEGACY_LEVELS):
    return sum(points >= t for t in table)


def legacy_level(realm, c, kind):
    """A person's standing in a legacy: their own, or their house's if the house carries that track."""
    own = level_of(c.legacy.get(kind, 0.0))
    if c.house is not None:
        h = realm.houses[c.house]
        if h.track == kind:
            own = max(own, level_of(h.legacy, HOUSE_LEGACY_LEVELS))
    return own


def storytelling(realm, c):
    k = realm.kingdoms[c.realm] if realm.kingdoms else None
    keeper = realm.chars.get(k.council.get("loremaster")) if k is not None else None
    lore = c.attr("lore") + (keeper.attr("lore") if keeper is not None and keeper.alive else 5)
    return max(0.2, lore / 20 * STORY[c.people])


def gain_legacy(realm, c, kind, renown):
    """Deeds become legacy only as far as they are told."""
    if kind is None or renown <= 0:
        return
    pts = renown * storytelling(realm, c)
    before = level_of(c.legacy.get(kind, 0.0))
    c.legacy[kind] = c.legacy.get(kind, 0.0) + pts
    if kind not in c.legacy_origin:
        c.legacy_origin[kind] = (c.full_name(), c.born)
    after = level_of(c.legacy[kind])
    if after > before and after >= 2:
        realm.say(f"The songs of {realm.who(c)} spread; theirs is now a legacy of {LEGACY[kind][0]} "
                  f"{ROMAN[after]}.", minor=after < 5)
    c.invalidate()
    if c.house is not None:
        h = realm.houses[c.house]
        hb = level_of(h.legacy, HOUSE_LEGACY_LEVELS)
        h.legacy += pts * (0.5 if h.track == kind else 0.2)
        ha = level_of(h.legacy, HOUSE_LEGACY_LEVELS)
        if ha > hb:
            for i in realm.living:
                if realm.chars[i].house == h.id:
                    realm.chars[i].invalidate()
            realm.say(f"{h.name}'s legacy of {LEGACY[h.track][0]} reaches its {ROMAN[ha]} rank; every child of the "
                      f"house is born to it.", minor=ha < 3)


def inherit_legacy(c, parents):
    for p in parents:
        for kind, pts in p.legacy.items():
            if pts * 0.5 > c.legacy.get(kind, 0.0):
                c.legacy[kind] = pts * 0.5
                c.legacy_origin[kind] = p.legacy_origin.get(kind, (p.full_name(), p.born))


def pass_legacy_on_death(realm, c):
    """Deeds done after the children were born still reach them when the parent dies."""
    for kid_id in c.children:
        kid = realm.chars[kid_id]
        if kid.alive:
            inherit_legacy(kid, [c])
            kid.invalidate()


def legacy_bonus(realm, c, key):
    cache = c.__dict__.get("_lb")                 # kept until the person changes (invalidate)
    if cache is None:
        cache = c._lb = {}
    v = cache.get(key)
    if v is None:
        v = cache[key] = _legacy_bonus(realm, c, key)
    return v


def _legacy_bonus(realm, c, key):
    total = 0.0
    for kind, (_, fx) in LEGACY.items():
        if key in fx:
            lv = legacy_level(realm, c, kind)
            if lv:
                total += fx[key] * lv
    return total


def legacy_lines(realm, c):
    out = []
    kinds = set(c.legacy) | ({realm.houses[c.house].track} if c.house is not None else set())
    for kind in sorted(kinds):
        lv = legacy_level(realm, c, kind)
        if lv <= 0:
            continue
        origin = c.legacy_origin.get(kind)
        src = ""
        if origin and origin[0] != c.full_name():
            src = f", from {origin[0]} (born year {origin[1]})"
        elif c.house is not None and realm.houses[c.house].track == kind and level_of(c.legacy.get(kind, 0)) < lv:
            src = f", from {realm.houses[c.house].name}"
        out.append(f"{LEGACY[kind][0]} {ROMAN[lv]}{src}")
    return out
