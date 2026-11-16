"""Dungeons: every ruin, lair and buried hoard is a dungeon with floors. Mini-dungeons have a floor or two and
a mini-boss; grand dungeons (the elder ruins, the fallen capitals, caves that open in the deep wild) run five
to twelve floors, with mini-bosses guarding the way and a boss at the bottom. Heroes delve floor by floor; the
deeper the floor, the better the loot. A conquered dungeon lies quiet for a while, then something new moves in.
An unconquered grand dungeon spills monsters onto the overworld."""
import math

from .data import MONSTER, PEOPLES
from .geography import land_name

KINDS = {
    "crypt": (["the Bone King", "the Weeping Abbess", "the Lich of the Lower Vault", "the Hollow Saint"],
              ["the Ossuary Knight", "the Twin Ghouls", "the Grave Warden", "the Shroud-Spinner"]),
    "cave": (["the Blind Wyrm", "the Broodmother", "the Pale Thing Below", "the Eyeless Titan"],
             ["the Rat King", "the Cave Troll Chief", "the Glowworm Hive", "the Slime Lord"]),
    "fortress": (["the Iron Warden", "the Headless Marshal", "the Last Garrison", "the Black Castellan"],
                 ["the Jailor", "the Gatekeeper", "the Siege Golem", "the Turncoat Captain"]),
    "temple": (["the Drowned Prophet", "the Faceless Idol", "the Choir That Never Ends", "the Fallen Seraph"],
               ["the High Acolyte", "the Mirror Priest", "the Serpent Altar", "the Silent Bell"]),
    "mine": (["the Deep Golem", "the Gem-Eater", "the Mad Foreman", "the Living Vein"],
             ["the Rust Crawler", "the Collapsed Shift", "the Lantern Ghost", "the Ore Mimic"]),
    "tower": (["the Mad Archmage", "the Clockwork Sentinel", "the Bound Elemental", "the Mirror Self"],
              ["the Apprentice Gone Wrong", "the Animated Library", "the Gargoyle Pair", "the Arcane Lock"]),
    "lair": (["the Brood of the Dead Wyrm", "the Hoard-Wraith", "the Last Egg"],
             ["the Wyrmling Pack", "the Scale-Mites"]),
}


def kind_for(r):
    n = r.name
    if "lair of" in n:
        return "lair"
    if "hoard" in n:
        return "cave"
    if n.startswith("the ruins of") and r.depth >= 2:
        return ["crypt", "temple", "tower", "fortress"][hash(n) % 4]
    return ["crypt", "fortress", "mine", "cave"][hash(n) % 4]


def ensure(realm, r):
    """Give a ruin its dungeon: floors, mini-bosses on the way down, a boss at the bottom."""
    if getattr(r, "floors", None) is not None:
        return
    rng = realm.rng
    r.kind = kind_for(r)
    grand = r.depth >= 2.4 or r.name.startswith(("the Deep", "the Dark", "the Howling", "the Sunken", "the Black",
                                                  "the Echo")) or not r.name.startswith(("the ruins of", "the lair",
                                                                                         "the buried"))
    r.floors = int(rng.integers(5, 13)) if grand else int(rng.integers(1, 3))
    r.grand = grand
    r.floor = 0
    r.conquered = None
    r.conquests = []
    stock(realm, r)


def stock(realm, r):
    rng = realm.rng
    bosses, minis = KINDS[r.kind]
    r.minis = {}
    for f in range(1, r.floors):
        if rng.random() < (0.35 if r.grand else 0.0):
            r.minis[f] = {"name": str(rng.choice(minis)), "power": 120 * r.depth ** 1.5 * (1 + f * 0.4)}
    name = str(rng.choice(bosses if r.grand else minis))
    if name == "the Mad Archmage":
        name = f"the Mad Archmage {realm.name((['Vel', 'Mor', 'Qui', 'Zan'], ['ithar', 'oc', 'emus', 'arro']))}"
    r.boss = {"name": name, "power": 200 * r.depth ** 1.6 * (1 + r.floors * 0.35) * (2.5 if r.grand else 1.0),
              "mini": not r.grand}
    r.riches = max(r.riches, r.floors)


def danger(r, f):
    return 80 * r.depth ** 1.5 * (1 + f * 0.3)


def delve(realm, h, home, r):
    """One year below: clear the next floor (or its guardian), or face the boss at the bottom."""
    from .characters import gain_xp
    from .heroes import deed, give_item, hurt, make_item, renown, roll_rarity
    rng = realm.rng
    ensure(realm, r)
    if r.conquered is not None:
        if r.loot:
            it = r.loot.pop(0)
            give_item(realm, h, it, quiet=True)
            realm.say(f"{realm.who(h)} finds {it.label()} in {r.name}.", minor=it.rarity < 4)
        return
    nxt = r.floor + 1
    cr = h.cr() * h.mods()["delve"]
    from .skills import use
    use(realm, h, "Sneak", 0.8)
    use(realm, h, "Security", 0.8)
    if nxt < r.floors:
        guard = r.minis.get(nxt)
        foe = guard["power"] if guard else danger(r, nxt)
        if rng.random() < cr / (cr + foe):
            r.floor = nxt
            deed(h, "delve")
            gain_xp(h, foe, realm)
            h.gold += 30 * r.depth * nxt
            renown(realm, h, 2 + nxt + (8 if guard else 0))
            if guard:
                realm.say(f"{realm.who(h)} kills {guard['name']} on the {nxt}th floor of {r.name}.", minor=True)
                del r.minis[nxt]
            if rng.random() < 0.12 + 0.03 * nxt + (0.3 if guard else 0):
                it = make_item(realm, roll_rarity(rng, r.depth + nxt * 0.3), f"floor {nxt} of {r.name}")
                give_item(realm, h, it)
        else:
            hurt(realm, h, rng.uniform(20, 70) * (1 + nxt * 0.1),
                 f"died on the {nxt}th floor of {r.name}" + (f", killed by {guard['name']}" if guard else ""))
        return
    boss = r.boss
    if rng.random() < cr / (cr + 0.7 * boss["power"]):
        deed(h, "delve")
        gain_xp(h, boss["power"], realm)
        h.gold += 200 * r.depth * r.floors
        renown(realm, h, (15 if boss["mini"] else 60) + 5 * r.floors)
        it = make_item(realm, min(5, roll_rarity(rng, r.depth + r.floors * 0.4) + (1 if r.grand else 0)),
                       f"the hoard of {boss['name']}")
        give_item(realm, h, it)
        for extra in list(r.loot):
            give_item(realm, h, extra, quiet=True)
        r.loot = []
        r.conquered = realm.year
        r.conquests.append((realm.year, h.id, boss["name"]))
        r.riches = 0
        r.reinfest = realm.year + int(rng.integers(40, 150))
        realm.dungeon_log.append((realm.year, h.id, r.name, boss["name"], r.floors))
        realm.say(f"{'**' if r.grand else ''}{realm.who(h)} reaches the bottom of {r.name}, {r.floors} floors down, "
                  f"and kills {boss['name']}{'**' if r.grand else ''}.", minor=not r.grand)
    else:
        hurt(realm, h, rng.uniform(40, 110), f"was killed by {boss['name']} at the bottom of {r.name}")


def dungeons_year(realm):
    rng = realm.rng
    for r in list(realm.ruins):
        ensure(realm, r)
        if r.conquered is not None and realm.year >= getattr(r, "reinfest", 10 ** 9):
            r.conquered = None
            r.floor = 0
            r.depth = min(6.0, r.depth + 0.3)
            stock(realm, r)
            realm.say(f"Something new has moved into {r.name}: {r.boss['name']} rules its depths now.",
                      minor=not r.grand)
        # an unconquered grand dungeon spills onto the overworld
        if r.grand and r.conquered is None and rng.random() < 0.002 and \
                sum(1 for m in realm.monsters if m.alive and getattr(m, "spilled", False)) < 60:
            kind = str(rng.choice(["troll", "wraith", "demon"]))
            m = realm.new_monster(kind, r.pos, rng.uniform(*MONSTER[kind][1]) * (1 + r.depth / 4), quiet=True)
            m.name = f"{m.name}, out of {r.name}"
            m.spilled = True
    # new caves open in the deep wild
    if rng.random() < 0.02:
        from .world import Ruin
        land = (realm.terrain != 0) & (realm.owner == -1)
        cells = realm.remote_cells(land)
        if len(cells):
            y, x = cells[int(rng.integers(len(cells)))]
            r = Ruin(f"the {realm.name((['Deep', 'Dark', 'Howling', 'Sunken', 'Black', 'Echo'], ['delve', 'maw', 'hollow', 'warren', 'pit', 'gate']))}",
                     (int(y), int(x)), int(rng.integers(4, 9)), float(rng.uniform(2, 4.5)))
            realm.ruins.append(r)
            ensure(realm, r)
            realm.say(f"A way into the earth opens on {land_name(realm, int(realm.landmass[r.pos]))}: {r.name}, "
                      f"{r.floors} floors deep, and {r.boss['name']} waits at the bottom.", minor=True)
