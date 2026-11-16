"""A world of many continents and island chains, each landmass named, the seas named after the lands they
wash. Distances and remoteness come from scipy's distance transforms, so the map can be very large."""
import numpy as np
from scipy import ndimage

from .data import DESERT, FOREST, HILLS, MOUNTAIN, OCEAN, PLAINS, SWAMP

LAND_A = ["Var", "Ost", "Kael", "Mor", "Thal", "Ys", "Bran", "Ael", "Dur", "Nor", "Esk", "Vel", "Qar", "Hes",
          "Ul", "Sar", "Ith", "Corv", "Zan", "Fen", "Tor", "Gal", "Rhun", "Ash"]
LAND_B = ["hold", "andor", "essa", "mark", "vale", "oria", "heim", "garde", "thas", "wyn", "mere", "reach", "ia",
          "dun", "mar", "kara", "lund", "oth"]
ISLE_A = ["Gull", "Salt", "Storm", "Black", "Amber", "Coral", "Fog", "Wreck", "Seal", "Whale", "Skull", "Pearl",
          "Ember", "Thorn", "Lantern", "Widow", "Siren", "Bone", "Copper", "Mist"]
SEA_A = ["the Sea of", "the Gulf of", "the Straits of", "the Deeps of", "the Shallows of"]


def value_noise(H, W, rng, octaves=5, base=2):
    out = np.zeros((H, W))
    for o in range(octaves):
        cy = 2 ** (o + base)
        cx = max(2, int(round(cy * W / H)))
        lat = rng.random((cy + 1, cx + 1))
        ys = np.linspace(0, cy, H, endpoint=False)
        xs = np.linspace(0, cx, W, endpoint=False)
        iy, ix = ys.astype(int), xs.astype(int)
        fy, fx = ys - iy, xs - ix
        fy, fx = fy * fy * (3 - 2 * fy), fx * fx * (3 - 2 * fx)
        a, b = lat[np.ix_(iy, ix)], lat[np.ix_(iy, ix + 1)]
        c, d = lat[np.ix_(iy + 1, ix)], lat[np.ix_(iy + 1, ix + 1)]
        top, bot = a + (b - a) * fx[None, :], c + (d - c) * fx[None, :]
        out += (top + (bot - top) * fy[:, None]) / 2 ** o
    return (out - out.min()) / (out.max() - out.min())


def make_world(realm, n_continents=None, land_frac=0.33):
    """Continents are big noisy blobs; island chains are strings of small ones. Then terrain, mana, landmasses."""
    H, W, rng = realm.H, realm.W, realm.rng
    yy, xx = np.mgrid[0:H, 0:W]
    n = n_continents or int(rng.integers(6, 9))
    scale = min(H, W)
    warp_y = (value_noise(H, W, rng, base=2) - 0.5) * scale * 0.35   # ragged coasts
    warp_x = (value_noise(H, W, rng, base=2) - 0.5) * scale * 0.35
    D = []
    for i in range(n):
        cy, cx = rng.uniform(0.15, 0.85) * H, rng.uniform(0.08, 0.92) * W
        ry = rng.uniform(0.13, 0.26) * scale * (1.4 if i < 2 else 1.0)
        rx = ry * rng.uniform(0.9, 1.9)
        ang = rng.uniform(0, np.pi)
        dy, dx = yy + warp_y - cy, xx + warp_x - cx
        u = (dx * np.cos(ang) + dy * np.sin(ang)) / rx
        v = (-dx * np.sin(ang) + dy * np.cos(ang)) / ry
        D.append(np.hypot(u, v))
    D = np.sort(np.array(D), axis=0)
    # each continent is its own blob, and a channel of sea always runs between two neighbours
    bump = np.clip(1 - D[0], 0, 1) * np.clip((D[1] - D[0]) / 0.25, 0, 1)
    for _ in range(int(rng.integers(5, 10))):                       # island chains along arcs
        y0, x0 = rng.uniform(0, H), rng.uniform(0, W)
        ang = rng.uniform(0, 2 * np.pi)
        for k in range(int(rng.integers(4, 12))):
            ang += rng.normal(0, 0.4)
            y0, x0 = y0 + np.sin(ang) * scale * 0.04, x0 + np.cos(ang) * scale * 0.04
            r = rng.uniform(0.008, 0.025) * scale
            bump = np.maximum(bump, np.clip(1 - np.hypot(yy - y0, xx - x0) / r, 0, 1) * 0.9)
    elev = bump * 0.8 + value_noise(H, W, rng, base=3) * 0.25
    land = elev >= np.quantile(elev, 1 - land_frac)
    land &= (D[1] - D[0]) > 0.06
    land[:2], land[-2:], land[:, :2], land[:, -2:] = False, False, False, False
    moist = value_noise(H, W, rng, base=3)
    lat = np.abs(yy / H - 0.5) * 2                                  # 0 at the equator, 1 at the poles
    moist = np.clip(moist + 0.15 * (lat - 0.5) * 0, 0, 1)
    le = elev[land]
    mq = np.quantile(moist[land], [0.18, 0.55, 0.86])
    t = np.full((H, W), PLAINS, np.int8)
    t[moist > mq[1]] = FOREST
    t[(moist < mq[0]) & (lat < 0.6)] = DESERT
    t[(moist > mq[2]) & (elev < np.quantile(le, 0.45))] = SWAMP
    t[elev >= np.quantile(le, 0.80)] = HILLS
    t[elev >= np.quantile(le, 0.93)] = MOUNTAIN
    t[~land] = OCEAN
    realm.terrain = t
    ley = value_noise(H, W, rng, octaves=3, base=3)
    mana = np.exp(-((ley - 0.5) / 0.025) ** 2)
    for _ in range(4 * n):
        r, c = rng.integers(0, H), rng.integers(0, W)
        mana += 0.9 * np.exp(-((yy - r) ** 2 + (xx - c) ** 2) / (2 * 4.0 ** 2))
    realm.mana = np.clip(mana, 0, 1) * land
    label_lands(realm)
    realm.coast = land & ndimage.binary_dilation(~land)              # land cells touching the sea
    realm.sea_dist = ndimage.distance_transform_edt(~land)           # how far out to sea each ocean cell is
    name_seas(realm)


def label_lands(realm):
    rng = realm.rng
    land = realm.terrain != OCEAN
    lab, n = ndimage.label(land)
    sizes = ndimage.sum(land, lab, index=np.arange(1, n + 1))
    coms = ndimage.center_of_mass(land, lab, index=np.arange(1, n + 1))
    realm.landmass = lab - 1                                        # -1 at sea
    realm.lands = []
    used = set()
    big = sorted(range(n), key=lambda i: -sizes[i])
    for rank, i in enumerate(big):
        size = int(sizes[i])
        kind = "continent" if size >= 1500 else "great isle" if size >= 300 else "isle"
        for _ in range(100):
            if kind == "continent":
                nm = f"{rng.choice(LAND_A)}{rng.choice(LAND_B)}"
            elif kind == "great isle":
                nm = f"{rng.choice(LAND_A)}{rng.choice(LAND_B)} Isle"
            else:
                nm = f"{rng.choice(ISLE_A)} Isle"
            if nm not in used:
                used.add(nm)
                break
        realm.lands.append(None)
        realm.lands[-1] = {"id": i, "name": nm, "size": size, "kind": kind,
                           "center": (int(coms[i][0]), int(coms[i][1])), "found_by": {}, "rank": rank}
    realm.lands.sort(key=lambda d: d["id"])


def name_seas(realm):
    """Each sea cell takes the name of the nearest great landmass: the Sea of X."""
    rng = realm.rng
    land = realm.terrain != OCEAN
    big = np.isin(realm.landmass, [d["id"] for d in realm.lands if d["kind"] != "isle"])
    _, idx = ndimage.distance_transform_edt(~big, return_indices=True)
    near = realm.landmass[idx[0], idx[1]]
    realm.sea_of = np.where(land, -1, near)
    realm.sea_names = {d["id"]: f"{rng.choice(SEA_A)} {d['name']}" for d in realm.lands if d["kind"] != "isle"}


def land_of(realm, pos):
    return int(realm.landmass[pos])


def land_name(realm, lid):
    return realm.lands[lid]["name"] if lid >= 0 else "the open sea"


def sea_name(realm, pos):
    s = int(realm.sea_of[pos]) if realm.terrain[pos] == OCEAN else -1
    return realm.sea_names.get(s, "the open sea")


def town_distance(realm):
    """Distance from every cell to the nearest town, for 'remote' placement. Once a year."""
    mask = np.ones((realm.H, realm.W), bool)
    for s in realm.alive_towns():
        mask[s.pos] = False
    if mask.all():
        realm.town_dist = np.full((realm.H, realm.W), 1e9)
    else:
        realm.town_dist = ndimage.distance_transform_edt(mask)


def clip(realm, pos):
    return (int(np.clip(pos[0], 0, realm.H - 1)), int(np.clip(pos[1], 0, realm.W - 1)))
