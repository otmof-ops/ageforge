"""The living world: a food web simulated across the whole map.

The map is cut into regions of 6 x 6 leagues. Each region has a biome (from its land, its latitude and the
sea) and a forage it grows. Every species has a population in every region. Grazers and browsers eat the
forage and crowd each other; predators eat prey with a saturating appetite (Holling type II); every species
breeds, starves, dies and migrates into neighbouring regions it can live in. People clear land, hunt game for
food and drive out predators; predators raid herds and, where they are thick, a great alpha rises as an
overworld boss. Species go extinct. Where the ley lines run strong, new mythic breeds arise from old ones."""
import math

import numpy as np

from .data import DESERT, FOREST, HILLS, MOUNTAIN, OCEAN, PLAINS, SWAMP

B = 6
BIOMES = ["plains", "forest", "hills", "mountain", "swamp", "desert", "tundra", "shallows", "deep sea"]
PRODUCTIVITY = np.array([1.0, 0.9, 0.6, 0.3, 0.8, 0.25, 0.3, 0.9, 0.5])
#            plains forest hills mount swamp desert tundra shallows deep
SPECIES = [  # name, trophic (1 grazer, 2 predator, 3 apex), habitat suitability, breeding rate, size, mythic, prey,
    # tame-as, danger to herds
    ("red deer", 1, [0.8, 1, 0.5, 0.1, 0.2, 0, 0.3, 0, 0], 0.35, 1.0, False, [], None, 0),
    ("aurochs", 1, [1, 0.4, 0.4, 0, 0.3, 0.1, 0.2, 0, 0], 0.25, 2.5, False, [], None, 0),
    ("wild boar", 1, [0.4, 1, 0.3, 0, 0.8, 0, 0, 0, 0], 0.45, 1.2, False, [], None, 0),
    ("mountain goat", 1, [0, 0, 0.7, 1, 0, 0.1, 0.2, 0, 0], 0.35, 0.8, False, [], None, 0),
    ("woolly mammoth", 1, [0.3, 0.1, 0.2, 0, 0, 0, 1, 0, 0], 0.12, 6.0, False, [], None, 0),
    ("sand antelope", 1, [0.4, 0, 0.1, 0, 0, 1, 0, 0, 0], 0.4, 0.9, False, [], None, 0),
    ("giant elk", 1, [0.2, 0.8, 0.3, 0, 0, 0, 0.8, 0, 0], 0.2, 2.2, False, [], None, 0),
    ("hare", 1, [1, 0.7, 0.6, 0.2, 0.3, 0.3, 0.5, 0, 0], 0.9, 0.2, False, [], None, 0),
    ("moss tortoise", 1, [0, 0.2, 0, 0, 1, 0, 0, 0, 0], 0.1, 1.5, False, [], None, 0),
    ("sunback camel", 1, [0.1, 0, 0, 0, 0, 1, 0, 0, 0], 0.15, 2.0, False, [], None, 0),
    ("unicorn", 1, [0.3, 1, 0.2, 0, 0, 0, 0, 0, 0], 0.08, 1.8, True, [], None, 0),
    ("silverhorn ibex", 1, [0, 0, 0.5, 1, 0, 0, 0.3, 0, 0], 0.1, 1.2, True, [], None, 0),
    ("grey wolf", 2, [0.7, 1, 0.8, 0.3, 0.2, 0.1, 0.8, 0, 0], 0.3, 1.0, False,
     ["red deer", "wild boar", "hare", "mountain goat", "giant elk"], "wolf", 0.5),
    ("great bear", 2, [0.2, 1, 0.6, 0.7, 0.2, 0, 0.7, 0, 0], 0.12, 3.0, False,
     ["red deer", "wild boar", "hare", "giant elk"], "bear", 0.6),
    ("sabrecat", 2, [1, 0.4, 0.8, 0.2, 0, 0.3, 0.2, 0, 0], 0.18, 1.6, False,
     ["aurochs", "red deer", "sand antelope", "wild boar"], None, 0.7),
    ("mire lurker", 2, [0, 0.1, 0, 0, 1, 0, 0, 0.2, 0], 0.15, 2.0, False, ["wild boar", "moss tortoise"], None, 0.5),
    ("giant eagle", 2, [0.3, 0.2, 0.8, 1, 0, 0.2, 0.3, 0, 0], 0.2, 0.8, False,
     ["hare", "mountain goat", "sand antelope"], "eagle", 0.2),
    ("dire wolf", 2, [0.2, 0.4, 0.4, 0.2, 0, 0, 1, 0, 0], 0.2, 1.6, True,
     ["giant elk", "woolly mammoth", "red deer"], "wolf", 0.9),
    ("basilisk", 2, [0, 0, 0, 0, 0.8, 0.8, 0, 0, 0], 0.06, 2.5, True, ["moss tortoise", "sunback camel",
                                                                       "sand antelope"], None, 0.8),
    ("manticore", 2, [0.2, 0, 0.8, 0.3, 0, 0.9, 0, 0, 0], 0.07, 3.0, True, ["sand antelope", "aurochs",
                                                                         "sunback camel"], None, 1.2),
    ("sand wyrm", 2, [0, 0, 0, 0, 0, 1, 0, 0, 0], 0.05, 5.0, True, ["sunback camel", "sand antelope"], None, 1.0),
    ("griffin", 3, [0.2, 0.1, 1, 1, 0, 0.1, 0, 0, 0], 0.07, 3.5, True,
     ["mountain goat", "silverhorn ibex", "red deer", "aurochs"], "griffin", 1.0),
    ("wyvern", 3, [0, 0.2, 0.6, 0.9, 0.7, 0.2, 0, 0, 0], 0.06, 4.0, True,
     ["wild boar", "aurochs", "mountain goat", "moss tortoise"], "wyvern", 1.3),
    ("drake", 3, [0.1, 0, 0.6, 1, 0, 0.6, 0, 0, 0], 0.04, 6.0, True,
     ["aurochs", "woolly mammoth", "sunback camel", "griffin"], None, 1.6),
    ("roc", 3, [0, 0, 0.3, 1, 0, 0.2, 0.5, 0, 0], 0.03, 7.0, True, ["woolly mammoth", "giant elk", "aurochs"],
     None, 1.4),
    ("phoenix", 3, [0, 0, 0, 0.3, 0, 1, 0, 0, 0], 0.02, 3.0, True, [], None, 0),
    ("carrion crow", 2, [1, 1, 1, 0.5, 1, 0.5, 0.5, 0.2, 0], 0.6, 0.1, False, ["hare"], None, 0),
    ("ghoul", 2, [0, 0, 0, 0, 1, 0.3, 0, 0, 0], 0.08, 1.5, True, ["moss tortoise"], None, 0.6),
    ("herring shoal", 1, [0, 0, 0, 0, 0, 0, 0, 1, 0.6], 0.9, 0.3, False, [], None, 0),
    ("grey seal", 2, [0, 0, 0, 0, 0, 0, 0.2, 1, 0.2], 0.2, 1.2, False, ["herring shoal"], None, 0),
    ("giant squid", 2, [0, 0, 0, 0, 0, 0, 0, 0.2, 1], 0.2, 3.0, False, ["herring shoal"], None, 0),
    ("great whale", 1, [0, 0, 0, 0, 0, 0, 0, 0.4, 1], 0.05, 10.0, False, [], None, 0),
    ("sea serpent", 3, [0, 0, 0, 0, 0, 0, 0, 0.3, 1], 0.05, 6.0, True, ["grey seal", "giant squid", "herring shoal"],
     None, 0),
    ("kraken", 3, [0, 0, 0, 0, 0, 0, 0, 0, 1], 0.02, 12.0, True, ["great whale", "giant squid"], None, 0),
]
VARIANT_PREFIX = ["Ember", "Frost", "Storm", "Shadow", "Star", "Glass", "Bone", "Thorn", "Spirit", "Gold", "Ash",
                  "Moon", "Blood", "Ghost", "Iron"]
REGION_A = ["Whispering", "Broken", "Ashen", "Silver", "Hollow", "Old", "Thorned", "Misty", "Red", "Howling"]
REGION_B = {"plains": "Steppe", "forest": "Wood", "hills": "Downs", "mountain": "Peaks", "swamp": "Fen",
            "desert": "Waste", "tundra": "Tundra", "shallows": "Shoals", "deep sea": "Deep"}


class Ecology:
    def __init__(self, realm):
        H, W = realm.H, realm.W
        self.eh, self.ew = H // B, W // B
        t = realm.terrain[:self.eh * B, :self.ew * B].reshape(self.eh, B, self.ew, B).transpose(0, 2, 1, 3)
        t = t.reshape(self.eh, self.ew, B * B)
        sea_frac = (t == OCEAN).mean(-1)
        counts = np.stack([(t == k).sum(-1) for k in (PLAINS, FOREST, HILLS, MOUNTAIN, SWAMP, DESERT)], -1)
        biome = counts.argmax(-1)
        lat = np.abs((np.arange(self.eh)[:, None] + 0.5) / self.eh - 0.5) * 2 * np.ones((1, self.ew))
        biome = np.where((lat > 0.72) & (biome != 3), 6, biome)
        sd = realm.sea_dist[:self.eh * B, :self.ew * B].reshape(self.eh, B, self.ew, B).mean((1, 3))
        biome = np.where(sea_frac > 0.6, np.where(sd > 5, 8, 7), biome)
        self.biome = biome
        self.land = sea_frac <= 0.6
        self.mana = realm.mana[:self.eh * B, :self.ew * B].reshape(self.eh, B, self.ew, B).mean((1, 3))
        self.species = [dict(name=s[0], troph=s[1], suit=np.array(s[2], float), r=s[3], size=s[4], mythic=s[5],
                             prey=list(s[6]), tame=s[7], danger=s[8], extinct=None, origin="the first age",
                             peak=0.0) for s in SPECIES]
        self.pop = np.zeros((len(self.species), self.eh, self.ew))
        for i, sp in enumerate(self.species):
            k = self.capacity(i)
            self.pop[i] = k * realm.rng.uniform(0.3, 0.8, k.shape)
        self.alphas = {}
        self.region_names = {}
        self.events = []

    def capacity(self, i, human=None):
        sp = self.species[i]
        k = sp["suit"][self.biome] * PRODUCTIVITY[self.biome] * 100 / (sp["size"] ** 0.75) / (sp["troph"] ** 2.2)
        if sp["mythic"]:
            k = k * np.clip(self.mana * 4 + 0.1, 0, 1) * (0.25 if sp["troph"] == 1 else 1.0)
        if human is not None:
            k = k * (1 - 0.8 * human)
        return k

    def idx(self, name):
        for i, s in enumerate(self.species):
            if s["name"] == name:
                return i
        return None

    def region(self, rng, ey, ex):
        key = (ey // 4, ex // 4)
        if key not in self.region_names:
            self.region_names[key] = f"the {rng.choice(REGION_A)} {REGION_B[BIOMES[self.biome[ey, ex]]]}"
        return self.region_names[key]


def ecology_year(realm):
    eco = realm.eco
    rng = realm.rng
    H, W = eco.eh * B, eco.ew * B
    owned = (realm.owner[:H, :W] >= 0).reshape(eco.eh, B, eco.ew, B).mean((1, 3))
    n = len(eco.species)
    P = eco.pop
    new = P.copy()
    # grazers: logistic growth against a shared forage
    graze = sum(P[i] * eco.species[i]["size"] ** 0.75 for i in range(n) if eco.species[i]["troph"] == 1)
    share = sum(eco.species[i]["suit"][eco.biome] for i in range(n)
                if eco.species[i]["troph"] == 1 and eco.species[i]["extinct"] is None)
    forage = PRODUCTIVITY[eco.biome] * 100 * (1 - 0.7 * owned) * (share + 0.5) + 1e-6
    # predation, first as demand: what every predator would eat of every prey this year
    demand = {}
    for i, sp in enumerate(eco.species):
        if sp["extinct"] is not None or sp["troph"] == 1 or not sp["prey"]:
            continue
        prey = [q for q in (eco.idx(x) for x in sp["prey"]) if q is not None and eco.species[q]["extinct"] is None]
        if not prey:
            continue
        avail = sum(P[q] * eco.species[q]["size"] for q in prey)
        a, h = 0.3 / sp["size"] ** 0.25, 0.4
        intake = a * P[i] * avail / (1 + a * h * avail)
        for q in prey:
            share = P[q] * eco.species[q]["size"] / (avail + 1e-9)
            demand[(i, q)] = intake * share / eco.species[q]["size"]
    # then supply: no prey loses more than two-fifths of itself to all its hunters together
    total = {}
    for (i, q), v in demand.items():
        total[q] = total.get(q, 0) + v
    scale = {q: np.minimum(1.0, 0.4 * P[q] / (t + 1e-9)) for q, t in total.items()}
    eaten = np.zeros(n)
    gain = [np.zeros_like(P[0]) for _ in range(n)]
    for (i, q), v in demand.items():
        got = v * scale[q]
        new[q] -= got
        gain[i] += got * eco.species[q]["size"]
    for i, sp in enumerate(eco.species):
        if sp["extinct"] is not None:
            continue
        K = eco.capacity(i, owned) + 1e-6
        if sp["troph"] == 1:
            crowd = graze / forage
            new[i] += sp["r"] * P[i] * (1 - 0.5 * P[i] / K - 0.5 * crowd)
        elif sp["prey"]:
            new[i] += 0.3 * gain[i] / sp["size"] - 0.15 * sp["r"] * P[i] - 0.3 * sp["r"] * P[i] * P[i] / K
        else:                                                       # the phoenix lives on the ley lines alone
            new[i] += sp["r"] * P[i] * (1 - P[i] / K)
        # people hunt in the land they hold
        new[i] -= P[i] * owned * (0.12 if sp["troph"] == 1 else 0.15)
    new = np.clip(new, 0, None)
    # migration into neighbouring regions a species can live in
    for i, sp in enumerate(eco.species):
        if sp["extinct"] is not None:
            continue
        hab = sp["suit"][eco.biome] > 0
        Q = new[i]
        nb = (np.roll(Q, 1, 0) + np.roll(Q, -1, 0) + np.roll(Q, 1, 1) + np.roll(Q, -1, 1)) / 4
        mig = 0.08 if sp["troph"] > 1 else 0.05
        new[i] = np.where(hab, (1 - mig) * Q + mig * nb, 0)
    eco.pop = new
    for i, sp in enumerate(eco.species):
        total = float(new[i].sum())
        sp["peak"] = max(sp["peak"], total)
        if sp["extinct"] is None and sp["peak"] > 0 and total < 0.5:
            sp["extinct"] = realm.year
            eco.pop[i] = 0
            realm.say(f"**An extinction.** The last {sp['name']} dies; there will be no more in the world.",
                      minor=not sp["mythic"] and sp["troph"] == 1)
            eco.events.append((realm.year, f"the {sp['name']} goes extinct"))
    speciate(realm)
    towns_and_wild(realm, owned)
    alphas(realm)


def speciate(realm):
    """In strong ley-line country a new mythic breed may split from an old species."""
    eco = realm.eco
    rng = realm.rng
    if len(eco.species) >= 90 or rng.random() > 0.015:
        return
    hot = eco.mana > 0.35
    cands = [i for i, s in enumerate(eco.species) if s["extinct"] is None and (eco.pop[i] * hot).sum() > 5]
    if not cands:
        return
    i = cands[int(rng.integers(len(cands)))]
    base = eco.species[i]
    for _ in range(20):
        name = f"{rng.choice(VARIANT_PREFIX).lower()} {base['name'].split()[-1]}"
        if eco.idx(name) is None:
            break
    else:
        return
    sp = dict(base)
    sp.update(name=name, suit=np.clip(base["suit"] + rng.normal(0, 0.2, len(BIOMES)), 0, 1) * (base["suit"] > 0),
              r=base["r"] * rng.uniform(0.7, 1.2), size=base["size"] * rng.uniform(1.0, 1.6), mythic=True,
              prey=list(base["prey"]), extinct=None, origin=f"split from the {base['name']} in year {realm.year}",
              peak=0.0, danger=base["danger"] * 1.4)
    eco.species.append(sp)
    seed = np.zeros((1, eco.eh, eco.ew))
    ey, ex = np.unravel_index(int((eco.pop[i] * hot).argmax()), hot.shape)
    seed[0, ey, ex] = max(2.0, eco.pop[i, ey, ex] * 0.3)
    eco.pop = np.concatenate([eco.pop, seed], 0)
    where = eco.region(rng, ey, ex)
    realm.say(f"**A new breed.** In {where}, where the ley lines run strong, the first {name}s are seen, kin to the "
              f"{base['name']} and not the same.", minor=not base["troph"] > 1)
    eco.events.append((realm.year, f"the {name} appears in {where}"))


def towns_and_wild(realm, owned):
    """Game feeds towns; predators take from their herds."""
    eco = realm.eco
    herb = sum(eco.pop[i] * eco.species[i]["size"] for i in range(len(eco.species)) if eco.species[i]["troph"] == 1)
    danger = sum(eco.pop[i] * eco.species[i]["danger"] for i in range(len(eco.species)) if eco.species[i]["danger"])
    for s in realm.alive_towns():
        ey, ex = min(eco.eh - 1, s.pos[0] // B), min(eco.ew - 1, s.pos[1] // B)
        s.game = float(min(0.3, herb[ey, ex] / 400))
        s.pop *= 1 - min(0.01, danger[ey, ex] / 20000)


def alphas(realm):
    """Where a dangerous species is thick, a great alpha rises: an overworld boss with a name."""
    eco = realm.eco
    rng = realm.rng
    for a_id, (m, i) in list(eco.alphas.items()):
        if not m.alive:
            ey, ex = m.eco_cell
            eco.pop[i, ey, ex] *= 0.5                              # its kin scatter when it falls
            del eco.alphas[a_id]
    if rng.random() > 0.3:
        return
    i = int(rng.integers(len(eco.species)))
    sp = eco.species[i]
    if sp["extinct"] is not None or sp["troph"] < 2 or sp["danger"] <= 0:
        return
    if sum(1 for m, j in eco.alphas.values() if j == i) >= 2:
        return
    ey, ex = np.unravel_index(int(eco.pop[i].argmax()), eco.pop[i].shape)
    dens = eco.pop[i, ey, ex] / (eco.capacity(i)[ey, ex] + 1e-6)
    if dens < 0.6:
        return
    pos = (int(ey * B + B // 2), int(ex * B + B // 2))
    if realm.terrain[pos] == OCEAN:
        return
    where = eco.region(rng, ey, ex)
    base = sp['name'][6:] if sp['name'].lower().startswith("great ") else sp['name']
    title = f"the Great {base.title()} of {where.replace('the ', '')}"
    power = 150 * sp["size"] * (1 + sp["troph"]) * (1.5 if sp["mythic"] else 1.0)
    m = realm.new_monster("beast", pos, power, name=title, quiet=True)
    m.species, m.eco_cell = sp["name"], (int(ey), int(ex))
    eco.alphas[m.mid] = (m, i)
    realm.say(f"{title} rises, alpha of the {sp['name']}s of {where}; travellers go round.", minor=sp["troph"] < 3)


def tameable_near(realm, c):
    """What kinds a tamer could find in their own region."""
    eco = realm.eco
    home = realm.by_id.get(c.home) if c.home is not None else realm.capital_of(c.realm)
    if home is None:
        return set()
    ey, ex = min(eco.eh - 1, home.pos[0] // B), min(eco.ew - 1, home.pos[1] // B)
    out = set()
    for dy in (-1, 0, 1):
        for dx in (-1, 0, 1):
            y, x = ey + dy, ex + dx
            if 0 <= y < eco.eh and 0 <= x < eco.ew:
                for i, sp in enumerate(eco.species):
                    if sp["tame"] and sp["extinct"] is None and eco.pop[i, y, x] > 0.5:
                        out.add(sp["tame"])
    return out
