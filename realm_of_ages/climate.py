"""Climate and weather, and what they do to the harvest.

THE CLIMATE OF A PLACE. Warmth falls away from the middle of the world towards the poles and with height; rain comes
in off the sea on the prevailing winds (westerlies in the middle latitudes, easterlies near the middle of the world),
falls on the windward side of mountains and leaves a rain shadow behind them. So every region has its own normal
warmth and rain, and its own way of failing: high and northern fields fail in the cold, dry lands in drought, low
clay plains and fens in the wet.

THE WEATHER OF A YEAR is the sum of:
- an age of the world: a slow drift of a degree or so over centuries, a warm age or a cold one;
- a cycle of a few years that moves rain from one side of the world to the other;
- the ash of great eruptions, which can take the summer away for a year or three;
- the year's own weather, which comes in regional patterns and tends to persist (wet years come in runs).

THE HARVEST answers to it: a kind year (rain when wanted, sun to ripen) bears above the common run; cold shortens the season (worst where it is already short); drought withers (worse in a
hot year); too much rain rots the grain in the ear and the fields cannot be worked (worse in a cold year, as in the
great famines of the old world); floods drown the fens and the low coasts. After two hot dry years the locusts may
come; after wet warm years, the blight and the black rye. Then the soil: fields worn out by too many mouths yield
less, and recover with fallow, rotations and manure. And the granaries: grain rots in damp stores and to vermin."""
import math

import numpy as np
from scipy import ndimage

from .data import DESERT, HILLS, MOUNTAIN, OCEAN, PLAINS, SWAMP

B = 6                                                                 # the weather grid's cell, in map cells
ERUPTION_A = ["Ashen", "Grey", "Dark", "Cold", "Dim", "Silent", "Red-Sun"]
ERUPTION_B = ["Summer", "Year", "Sky", "Winter", "Season"]


def setup(realm):
    """The normal climate of every region, computed once."""
    H, W = realm.H, realm.W
    eh, ew = H // B, W // B
    t = realm.terrain[:eh * B, :ew * B].reshape(eh, B, ew, B).transpose(0, 2, 1, 3).reshape(eh, ew, B * B)
    frac = {k: (t == k).mean(-1) for k in (OCEAN, PLAINS, HILLS, MOUNTAIN, SWAMP, DESERT)}
    lat = np.abs((np.arange(eh)[:, None] + 0.5) / eh - 0.5) * 2 * np.ones((1, ew))   # 0 middle, 1 the poles
    temp = 22 - 16 * lat - 7 * frac[MOUNTAIN] - 2.5 * frac[HILLS]
    # rain carried on the winds: off the sea, wrung out over land and mountains
    rain = np.zeros((eh, ew))
    for row in range(eh):
        west = lat[row, 0] > 0.33                                      # westerlies; easterlies near the middle
        cols = range(ew) if west else range(ew - 1, -1, -1)
        m = 1.0
        for c in cols:
            sea = frac[OCEAN][row, c]
            m = min(1.0, m + 0.35 * sea)
            mtn = frac[MOUNTAIN][row, c]
            fall = m * (0.25 + 0.6 * mtn + 0.15 * frac[HILLS][row, c])
            rain[row, c] = fall
            m = max(0.05, m - fall * 0.5 * (1 - sea))
    rain = ndimage.gaussian_filter(rain, 1.5)
    land = frac[OCEAN] < 0.6
    rain = rain / max(1e-6, rain[land].mean())
    rain = np.clip(rain * (1 - 0.5 * frac[DESERT]), 0.15, 3.0)
    realm.climate = {
        "eh": eh, "ew": ew, "lat": lat, "temp": temp, "rain": rain, "land": land,
        "cold_sens": np.clip((lat - 0.35) * 2, 0, 1) + frac[MOUNTAIN] + 0.5 * frac[HILLS] + (temp < 8) * 0.5,
        "dry_sens": frac[DESERT] + (rain < 0.6) * 0.8 + 0.3 * frac[PLAINS],
        "wet_sens": frac[PLAINS] * 0.6 + frac[SWAMP] * 1.2 + (rain > 1.4) * 0.3,
        "flood_sens": frac[SWAMP] + 0.5 * (frac[OCEAN] > 0.2) * (frac[HILLS] + frac[MOUNTAIN] < 0.2),
        "plains": frac[PLAINS], "desert": frac[DESERT],
        "trend": 0.0, "osc": 0.0, "volc": 0.0, "volc_hemi": 0, "tn": np.zeros((eh, ew)), "pn": np.zeros((eh, ew)),
        "dry_run": np.zeros((eh, ew)), "wet_run": np.zeros((eh, ew)), "age": 0,
        "pattern": np.cos(2 * math.pi * (np.arange(ew)[None, :] / ew) + realm.rng.uniform(0, 6.3)) * np.ones((eh, 1)),
        "log": [],
    }


def weather_year(realm):
    """This year's warmth and rain everywhere, and the harvest they make."""
    if getattr(realm, "climate", None) is None:
        setup(realm)
    cl = realm.climate
    rng = realm.rng
    eh, ew = cl["eh"], cl["ew"]
    # an age of the world: a slow drift of warmth over centuries
    cl["trend"] = 0.995 * cl["trend"] + rng.normal(0, 0.08)
    age = cl["age"]                                                    # an age, once begun, lasts until it has truly turned
    if cl["trend"] > 0.7:
        age = 1
    elif cl["trend"] < -0.7:
        age = -1
    elif abs(cl["trend"]) < 0.2:
        age = 0
    if age != cl["age"]:
        if age:
            realm.say(f"**{'A warm age' if age > 0 else 'A cold age'} begins.** "
                      + ("The summers lengthen; vines grow where none grew, and the high fields bear."
                         if age > 0 else "The winters lengthen and the high fields are given up; "
                                         "the glaciers come down the valleys."))
        cl["age"] = age
    # a cycle of a few years that moves the rain about
    cl["osc"] = 0.6 * cl["osc"] + rng.normal(0, 0.8)
    # great eruptions
    cl["volc"] *= 0.5
    if rng.random() < 0.02:
        strength = rng.uniform(0.8, 2.5)
        cl["volc"] += strength
        cl["volc_hemi"] = int(rng.integers(-1, 2)) if strength < 2 else 0
        name = f"the {rng.choice(ERUPTION_A)} {rng.choice(ERUPTION_B)}"
        where = {-1: "the north", 1: "the south", 0: "the whole world"}[cl["volc_hemi"]]
        realm.say(f"**{name[0].upper() + name[1:]}.** Far away a mountain tears itself open; ash veils the sun over "
                  f"{where}, and summer does not come.")
        cl["log"].append((realm.year, name, round(strength, 1), where))
    rows = (np.arange(eh) + 0.5) / eh
    hemi = {-1: rows < 0.5, 1: rows >= 0.5, 0: np.ones(eh, bool)}[cl["volc_hemi"]][:, None] * np.ones((1, ew))
    volc = cl["volc"] * hemi
    # the year's own weather: regional patterns, and runs of wet or dry years
    cl["tn"] = 0.3 * cl["tn"] + ndimage.gaussian_filter(rng.normal(0, 1, (eh, ew)), 3) * 9 * 0.9
    cl["pn"] = 0.35 * cl["pn"] + ndimage.gaussian_filter(rng.normal(0, 1, (eh, ew)), 3) * 9 * 0.22
    T = cl["trend"] + 0.3 * cl["osc"] * cl["pattern"] - volc + cl["tn"]
    P = np.exp(cl["pn"] + 0.15 * cl["osc"] * cl["pattern"] + 0.06 * volc)
    cl["T"], cl["P"] = T, P
    # the harvest
    Y = np.ones((eh, ew))
    cold = np.minimum(T, 0)
    Y *= np.clip(1 + 0.10 * cold * (1 + 2 * cl["cold_sens"]), 0.25, 1.0)
    Y *= 1 + np.clip(0.03 * np.maximum(T, 0) * cl["cold_sens"], 0, 0.1)            # warmth helps the marginal north
    dry_at = np.where(T > 0.8, 0.9, 0.8)
    Y *= np.where(P < dry_at, np.clip((P / dry_at) ** (1 + cl["dry_sens"]), 0.2, 1), 1)
    wet = np.maximum(P - 1.25, 0) * (1.2 + cl["wet_sens"]) * np.where(T < 0, 1.3, 1.0)
    Y *= np.clip(1 - wet, 0.3, 1)
    Y *= np.clip(1 - 0.6 * cl["flood_sens"] * np.maximum(P - 1.5, 0), 0.4, 1)
    # a kind year: rain when it is wanted and sun when it ripens, and the fields bear above the common run
    Y *= 1 + 0.18 * np.exp(-((P - 1.0) / 0.18) ** 2) * np.clip(1 + 0.5 * T, 0, 1.3)
    # pests and blight, after runs of bad years
    cl["dry_run"] = np.where((P < 0.85) & (T > 0.3), cl["dry_run"] + 1, 0)
    cl["wet_run"] = np.where((P > 1.2) & (T > -0.3), cl["wet_run"] + 1, 0)
    plague_of(realm, cl, Y, cl["dry_run"] >= 2, cl["plains"] + cl["desert"] > 0.4, 0.12, 6, 0.45, "locusts")
    plague_of(realm, cl, Y, cl["wet_run"] >= 2, cl["plains"] > 0.3, 0.08, 5, 0.6, "blight")
    if realm.blight > 0:
        Y *= 0.7
    realm.harvest = np.clip(Y, 0.15, 1.35)
    cl.setdefault("history", []).append((realm.year, float(np.mean(realm.harvest[cl["land"]])),
                                         float(np.mean(T[cl["land"]])), float(np.mean(P[cl["land"]]))))


def plague_of(realm, cl, Y, run, suits, chance, radius, mult, what):
    """Locusts after hot dry years, blight after warm wet ones: a swarm or a rot over a region."""
    cand = np.argwhere(run & suits & cl["land"])
    if not len(cand) or realm.rng.random() > chance:
        return
    cy, cx = cand[int(realm.rng.integers(len(cand)))]
    yy, xx = np.ogrid[0:cl["eh"], 0:cl["ew"]]
    near = (yy - cy) ** 2 + (xx - cx) ** 2 <= radius ** 2
    Y[near] *= mult
    pos = (int(cy * B + B // 2), int(cx * B + B // 2))
    towns = [t for t in realm.alive_towns() if (t.pos[0] - pos[0]) ** 2 + (t.pos[1] - pos[1]) ** 2 <= (radius * B) ** 2]
    if towns:
        t = max(towns, key=lambda t: t.pop)
        realm.say(("Locusts darken the sky over" if what == "locusts" else "The blight and the black rye spread through")
                  + f" the fields around {t.name}; {len(towns)} towns lose their harvest.", minor=len(towns) < 8)


def at(realm, grid, pos):
    cl = realm.climate
    return float(grid[min(cl["eh"] - 1, pos[0] // B), min(cl["ew"] - 1, pos[1] // B)])


def rain_at(realm, pos):
    return at(realm, realm.climate["P"], pos)


def soil_year(realm, s, base_capacity, cu):
    """Fields wear out under too many mouths and recover with fallow, rotations and manure."""
    inn = cu.innovations if cu is not None else {}
    soil = getattr(s, "soil", 1.0)
    press = s.pop / max(1.0, base_capacity)
    wear = 0.008 * max(0.0, press - 0.6)
    rest = 0.01 + 0.006 * ("Two-Field Rotation" in inn) + 0.01 * ("Three-Field Rotation" in inn) \
        + 0.004 * ("Hunting Grounds" in inn)
    s.soil = min(1.1, max(0.5, soil - wear + rest * (1.05 - soil)))
    return s.soil
