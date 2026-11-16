"""Resources in the land: regular goods that feed and enrich, and strategic ones that unlock what a realm can
build, field and forge. A realm holds a resource when one of its towns' territory covers a deposit. Missing a
strategic resource that a neighbour has is a reason to expand, and a reason for war."""
import numpy as np

from .data import DESERT, FOREST, HILLS, MOUNTAIN, OCEAN, PLAINS, SWAMP

# name: (kind, terrains, chance per suitable cell, colour, effect)
RESOURCES = {
    "grain": ("regular", (PLAINS,), 0.010, (230, 210, 90), "food"),
    "timber": ("strategic", (FOREST,), 0.010, (90, 60, 30), "ships"),
    "furs": ("luxury", (FOREST,), 0.004, (160, 110, 70), "wealth"),
    "salt": ("regular", (DESERT, SWAMP), 0.006, (240, 240, 240), "garrison"),
    "fish": ("regular", ("coast",), 0.012, (120, 190, 230), "food"),
    "stone": ("regular", (HILLS,), 0.008, (170, 170, 170), "walls"),
    "copper": ("regular", (HILLS,), 0.006, (200, 120, 60), "wealth"),
    "wine": ("luxury", (PLAINS, HILLS), 0.004, (140, 30, 70), "wealth"),
    "spices": ("luxury", (DESERT, SWAMP), 0.004, (230, 120, 30), "wealth"),
    "silk": ("luxury", (PLAINS, FOREST), 0.002, (240, 200, 230), "wealth"),
    "amber": ("luxury", ("coast", FOREST), 0.002, (250, 170, 40), "wealth"),
    "ivory": ("luxury", (DESERT, PLAINS), 0.002, (250, 245, 220), "wealth"),
    "iron": ("strategic", (HILLS, MOUNTAIN), 0.008, (90, 90, 110), "material"),
    "horses": ("strategic", (PLAINS,), 0.006, (150, 100, 50), "cavalry"),
    "gold": ("strategic", (MOUNTAIN, HILLS), 0.003, (255, 215, 0), "treasury"),
    "silver": ("strategic", (HILLS, MOUNTAIN), 0.004, (200, 200, 215), "treasury"),
    "gems": ("luxury", (MOUNTAIN,), 0.003, (80, 220, 220), "wealth"),
    "mithril": ("strategic", (MOUNTAIN,), 0.0012, (200, 220, 255), "material"),
    "adamantine ore": ("strategic", (MOUNTAIN,), 0.0006, (40, 40, 60), "material"),
    "mana crystals": ("strategic", ("mana",), 0.004, (190, 90, 255), "magic"),
    "dragonbone fossils": ("strategic", (DESERT, MOUNTAIN), 0.0004, (240, 230, 200), "material"),
    "sacred grove": ("strategic", ("grove",), 0.002, (60, 200, 90), "faith"),
    "ancient wood": ("strategic", (FOREST,), 0.0008, (40, 90, 40), "ships"),
    "war beasts": ("strategic", (HILLS, PLAINS, FOREST), 0.0015, (120, 40, 20), "cavalry"),
}
# what each strategic resource opens
NEEDS_MATERIAL = {2: "iron", 3: "iron", 4: "mithril", 5: "adamantine ore", 6: "dragonbone fossils"}
CAVALRY = {"Light Horse", "Knights", "Plate Knights"}


def place_resources(realm):
    rng = realm.rng
    H, W = realm.H, realm.W
    grid = np.full((H, W), -1, np.int16)
    names = list(RESOURCES)
    land = realm.terrain != OCEAN
    for i, (name, (kind, terr, p, col, fx)) in enumerate(RESOURCES.items()):
        mask = np.zeros((H, W), bool)
        for t in terr:
            if t == "coast":
                mask |= realm.coast
            elif t == "mana":
                mask |= land & (realm.mana > 0.6)
            elif t == "grove":
                mask |= (realm.terrain == FOREST) & (realm.mana > 0.4)
            else:
                mask |= realm.terrain == t
        hit = mask & (rng.random((H, W)) < p) & (grid < 0)
        grid[hit] = i
    realm.res_grid = grid
    realm.res_names = names


def realm_resources(realm):
    """Which resources each realm holds this year, and how many deposits of each."""
    g, o = realm.res_grid, realm.owner_fac
    m = (g >= 0) & (o >= 0)
    held = {}
    for f, r in zip(o[m].tolist(), g[m].tolist()):
        d = held.setdefault(f, {})
        name = realm.res_names[r]
        d[name] = d.get(name, 0) + 1
    for k in realm.kingdoms:
        k.resources = held.get(k.id, {})
    # towns benefit from what lies in their own land
    own = realm.owner
    mm = (g >= 0) & (own >= 0)
    for s in realm.cache:
        s.res = {}
    for t_idx, r in zip(own[mm].tolist(), g[mm].tolist()):
        s = realm.cache[t_idx]
        nm = realm.res_names[r]
        s.res[nm] = s.res.get(nm, 0) + 1


def has(k, name):
    return getattr(k, "resources", {}).get(name, 0) > 0


def town_bonus(s):
    """Food and wealth from a town's own deposits."""
    res = getattr(s, "res", {})
    food = 0.05 * sum(n for r, n in res.items() if RESOURCES[r][4] == "food")
    wealth = 1.5 * sum(n for r, n in res.items() if RESOURCES[r][4] in ("wealth", "treasury"))
    return min(0.4, food), wealth


def material_ok(k, idx):
    need = NEEDS_MATERIAL.get(idx)
    return need is None or has(k, need) or (need == "dragonbone fossils" and k.dragonbone > 0)


def lacks(k, other):
    """Strategic resources the other realm holds and this one wants."""
    return [r for r in getattr(other, "resources", {}) if RESOURCES[r][0] == "strategic" and not has(k, r)]
