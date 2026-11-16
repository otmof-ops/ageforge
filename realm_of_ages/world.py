"""The realm: land, towns, territory, economy, diplomacy and war, monsters and calamities, and the year."""
import math

import numpy as np

from . import (beasts, bending, crafts, dungeons, dynasty, ecology, geography, governance, heroes, legends, monsters,
               titles, youth, civics, resources, statecraft, court, opinion, heart, economy, populace,
               warfare, events, godmode, markets, stewardship, arcana,
               seafaring,
               shinobi, society)
from .characters import Item, SINGULAR
from .data import (ADJ, AGGRESSION, BASE_REL, CALAMITY_A, CALAMITY_B, CALAMITY_TITLE, DEFENSE, DEMON_SYL,
                   DRAGON_EPITHET, DRAGON_SYL, FOOD, FOREST, GROWTH, HILLS, MAGIC, MARTIAL, MONSTER, MOUNTAIN, OCEAN,
                   PEOPLES, PLAGUES, PLAINS, SETTLE_AT, SPELLS, SWAMP, TIER_WORDS, TOWN_SYL, WEALTH, WILD_A, WILD_B,
                   DESERT)


def value_noise(size, rng, octaves=5):
    out = np.zeros((size, size))
    for o in range(octaves):
        cells = 2 ** (o + 2)
        lat = rng.random((cells + 1, cells + 1))
        xs = np.linspace(0, cells, size, endpoint=False)
        i = xs.astype(int)
        f = xs - i
        f = f * f * (3 - 2 * f)
        a, b = lat[np.ix_(i, i)], lat[np.ix_(i, i + 1)]
        c, d = lat[np.ix_(i + 1, i)], lat[np.ix_(i + 1, i + 1)]
        top, bot = a + (b - a) * f[None, :], c + (d - c) * f[None, :]
        out += (top + (bot - top) * f[:, None]) / 2 ** o
    return (out - out.min()) / (out.max() - out.min())


def box_sum(grid, r):
    cs = np.pad(np.pad(grid, r).cumsum(0).cumsum(1), ((1, 0), (1, 0)))
    k = 2 * r + 1
    return cs[k:, k:] - cs[:-k, k:] - cs[k:, :-k] + cs[:-k, :-k]


def dist(a, b):
    return math.hypot(a[0] - b[0], a[1] - b[1])


def _kernel(rad):
    yy, xx = np.ogrid[-rad:rad + 1, -rad:rad + 1]
    D = np.hypot(yy, xx)
    return D, 1.0 / (1 + D)


KERNELS = {r: _kernel(r) for r in range(0, 12)}


class Settlement:
    def __init__(self, sid, name, faction, pos, pop, year, capital=False):
        self.id, self.name, self.faction, self.pos = sid, name, faction, pos
        self.pop, self.wealth, self.walls = float(pop), 30.0, 1 if capital else 0
        self.alive, self.founded, self.capital = True, year, capital
        self.magic, self.cells, self.guard = 0.0, 0, 0.0
        self.raided = self.raided_by = None
        self.lord, self.lord_mod = None, 1.0
        self.culture = faction                       # replaced by the founding realm's people in add_settlement


class Monster:
    def __init__(self, kind, name, pos, power, year):
        self.kind, self.name, self.pos = kind, name, pos
        self.power = self.base = float(power)
        self.hoard, self.born, self.alive = 0.0, year, True
        self.sleep_until, self.last_said = 0, -999
        self.mid, self.items = None, []


class Ruin:
    def __init__(self, name, pos, riches, depth):
        self.name, self.pos, self.riches, self.depth = name, pos, riches, depth
        self.loot = []


class Realm:
    Item = Item

    def __init__(self, width=480, height=270, seed=1):
        self.rng = np.random.default_rng(seed)
        self.W, self.H, self.year, self.seed = width, height, 0, seed
        self.used_names, self.item_names, self.worldforged_found = set(), set(), set()
        self.chron = []
        self.make_land()
        self.settlements, self.by_id, self.next_id = [], {}, 0
        self.monsters, self.ruins = [], []
        self.chars, self.living, self.hero_ids, self.next_cid = {}, set(), set(), 0
        self.houses, self.next_hid = {}, 0
        self.kingdoms = []
        self.guilds = [heroes.Guild(f) for f in range(4)]
        self.relation = BASE_REL + self.rng.normal(0, 8, (4, 4))
        self.relation = (self.relation + self.relation.T) / 2
        self.war = np.zeros((4, 4), bool)
        self.war_since = np.zeros((4, 4), int)
        self.weariness = np.zeros((4, 4))
        self.truce = np.zeros((4, 4), int)
        self.extinct = [False] * 4
        self.founded = [0] * 4
        self.blight, self.last_plague, self.last_calamity = 0, -999, 0
        self.world_tier, self.people_tier = 3, [0] * 4
        self.tier_events, self.rites, self.calamity_log, self.tier_hist = [], [], [], []
        self.acted = []
        self.next_mid, self.awakenings, self.sovereign_line = 0, [], []
        from . import characters
        characters.CTX["realm"] = self
        self.win = self.fresh_window()
        land = np.argwhere((self.terrain != OCEAN) & (self.terrain != MOUNTAIN))
        self.blocked = np.zeros((self.H, self.W), bool)
        self.town_dist = np.full((self.H, self.W), 1e9)
        for _ in range(12 * max(1, sum(1 for d in self.lands if d["kind"] != "isle")) // 2):
            r, c = land[self.rng.integers(len(land))]
            self.ruins.append(Ruin(self.elder_name(), (int(r), int(c)), int(self.rng.integers(3, 9)),
                                   float(self.rng.uniform(1, 4))))
        self.place_capitals()
        self.compute_territory()
        conts = [d for d in self.lands if d["kind"] == "continent"]
        isles = [d for d in self.lands if d["kind"] != "continent"]
        self.say(f"The world wakes: {self.W} by {self.H} leagues, {len(conts)} continents ("
                 + ", ".join(d["name"] for d in sorted(conts, key=lambda d: -d["size"]))
                 + f") and {len(isles)} isles, ley lines through {int((self.mana > 0.5).sum())} leagues of it, and "
                 f"{len(self.ruins)} elder ruins from an age no one remembers. "
                 f"It is said that tier three is the limit of ordinary genius, and that what lies above it "
                 f"belongs to legend.")
        self.guilds_soc, self.religions, self.clans, self.tribes = [], [], [], {}
        self.concord, self.concord_next, self.concord_waiting_child = None, 0, False
        self.concord_line, self.concord_need = [], None
        self.greatest = {}
        self.last_cal = {}
        self.duchies, self.ktitles, self.breakaways = {}, {}, []
        self.decisions, self.wonders = [], []
        self.wars, self.war_log = {}, []
        self.op_mods = {}
        self.love_log = []
        self.armies, self.next_army, self.epidemics, self.last_pandemic = [], 0, [], -999
        self.famines, self.risings, self.battles_fought, self.bankruptcies = [], [], [], []
        self.sieges_won, self.migrants = 0, 0.0
        self.piracy, self.sea_terror, self.havens, self.pirates = {}, {}, [], []
        self.pirate_king, self.pirate_kings, self.naval_battles = None, [], []
        self.cycles, self.curses, self.myths, self.legend_materials, self.legend_items = [], [], [], [], []
        self.watch, self.merges, self.dragon_riders = {}, [], []
        self.corrupted = np.zeros((self.H, self.W), bool)
        civics.found_cultures(self)
        society.found_guilds(self)
        society.found_faiths(self)
        shinobi.found_clans(self)
        bending.found_tribes(self)

    # ---------------- helpers
    def say(self, text, minor=False, cat=None):
        self.chron.append((self.year, text, minor))
        events.record(self, text, minor, cat)

    def who(self, c):
        k = self.kingdoms[c.realm] if self.kingdoms else None
        if k is not None and k.ruler == c.id:
            from .data import RULER_TITLE
            return f"{RULER_TITLE[c.people][int(c.female)]} {c.full_name()}"
        if c.concord and c.hero:
            return f"{c.full_name()} ({SINGULAR[c.people]}, the Concord)"
        if c.clan is not None:
            from .data2 import SHINOBI_RANKS
            return f"{c.full_name()} ({SINGULAR[c.people]} shinobi, {SHINOBI_RANKS[getattr(c, 'srank', 0)]})"
        if c.element is not None and not c.hero:
            return f"{c.full_name()} ({SINGULAR[c.people]} {c.element}-bender)"
        if c.profession and not c.hero:
            return f"{c.full_name()} ({SINGULAR[c.people]} {c.profession})"
        cls = c.classes[-1][0] if c.classes[-1][1] > 0 or len(c.classes) == 1 else c.classes[-2][0]
        return f"{c.full_name()} ({SINGULAR[c.people]} {cls} {c.level})"

    def faith_of_town(self, s):
        f = getattr(s, "faith", None)
        if not f:
            return None
        return self.religions[max(f, key=f.get)]

    def heroes(self):
        return [self.chars[i] for i in list(self.hero_ids) if self.chars[i].alive]

    def kill(self, c, cause, quiet=False, killer=None, by=None):
        if not c.alive:
            return
        if getattr(c, "tracked", False) or getattr(c, "undying_by_gods", False):
            if getattr(c, "undying_by_gods", False) and "old age" in cause:
                return                                                # the gods said not yet
            events.journal(self, c, f"Dies, aged {c.age(self.year)}: {cause}.", "lives")
        for kin_id in set(c.children) | {p for p in (c.father, c.mother, c.spouse) if p is not None}:
            kin = self.chars.get(kin_id)
            if kin is not None and getattr(kin, "tracked", False) and kin.alive:
                events.journal(self, kin, f"Loses {c.full_name()} ({cause}).", "lives")
        if killer is not None:
            killer.kills = getattr(killer, "kills", 0) + 1
        c.alive, c.died, c.cause = False, self.year, cause
        self.living.discard(c.id)
        if c.hero:
            self.hero_ids.discard(c.id)
            g = self.guilds[c.people]
            g.deaths_total += 1
            g.w["deaths"] += 1
            g.w["renown_at_death"].append(c.renown)
            g.w["level_at_death"].append(c.level)
            if c.renown >= 60 or c.level >= 40:
                arts = [it.label() for it in c.items if it.rarity >= 4]
                self.say(f"{self.who(c)} {cause}, aged {c.age(self.year)}; renown {c.renown:,.0f}"
                         + (f", bearer of {', '.join(arts)}" if arts else "") + ".")
        k = self.kingdoms[c.realm] if self.kingdoms else None
        legends.on_death(self, c, cause, killer)
        heart.on_death(self, c, by)
        governance.inherit(self, c, killer)
        if k is not None and k.ruler == c.id and not c.hero and not quiet:
            self.say(f"{self.who(c)} {cause}, aged {c.age(self.year)}.")

    def on_new_tier(self, c, t):
        c.peak_tier = max(c.peak_tier, t)
        if t > self.people_tier[c.people]:
            self.people_tier[c.people] = t
        if t > self.world_tier:
            self.world_tier = t
            spell = SPELLS[c.school()][t]
            self.tier_events.append((self.year, c.id, t, spell))
            self.say(f"**{self.who(c)} casts {spell}**, the first {TIER_WORDS[t]}-tier spell in the history "
                     f"of the world.")

    def fresh_window(self):
        return {"battles": 0, "captures": 0, "raids": 0, "slain": 0, "destroyed": 0}

    def name(self, syl):
        for tries in range(400):
            a, b = syl[0], syl[1]
            n = a[int(self.rng.integers(len(a)))]
            if tries >= 100:
                n += a[int(self.rng.integers(len(a)))].lower()
            n += b[int(self.rng.integers(len(b)))]
            if n not in self.used_names:
                self.used_names.add(n)
                return n
        n = f"{self.rng.choice(syl[0])}{self.rng.choice(syl[1])} {len(self.used_names)}"
        self.used_names.add(n)
        return n

    def elder_name(self):
        return "the ruins of " + self.name((["Ny", "Aza", "Quel", "Oro", "Tes", "Vael", "Hy", "Esk"],
                                            ["thar", "marath", "lorn", "essa", "vane", "dor", "ul", "ith"]))

    @property
    def npol(self):
        return len(self.kingdoms) if self.kingdoms else 4

    def nearest_realm(self, culture):
        """The living realm of a people, or any living realm at all, for someone whose own is gone."""
        alive = [k for k in self.kingdoms if not self.extinct[k.id]]
        same = [k for k in alive if k.people == culture]
        pick = max(same or alive, key=lambda k: len(self.alive_towns(k.id)), default=None)
        return pick.id if pick is not None else 0

    def cul(self, f):
        """The people (culture) of a realm."""
        return self.kingdoms[f].people if self.kingdoms else f

    def pname(self, f):
        """What to call a realm's folk in a sentence: 'the Humans', or for a breakaway 'the Humans of Zania'."""
        if not self.kingdoms:
            return self.pname(f)
        k = self.kingdoms[f]
        return k.short

    def grow_tables(self):
        """A new realm needs a row and a column in every table of war and peace."""
        n = len(self.kingdoms)
        for name in ("relation", "war", "war_since", "weariness", "truce"):
            a = getattr(self, name)
            if a.shape[0] < n:
                pad = n - a.shape[0]
                setattr(self, name, np.pad(a, ((0, pad), (0, pad))))
        while len(self.extinct) < n:
            self.extinct.append(False)
        while len(self.known) < n:
            self.known.append(set())
        while len(self.founded) < n:
            self.founded.append(0)

    def alive_towns(self, f=None):
        """Living towns, of one realm or all. Answers are cached until a town is founded, destroyed or changes hands."""
        key = (self.year, getattr(self, "_town_ver", 0))
        memo = getattr(self, "_town_memo", None)
        if memo is None or memo[0] != key:
            memo = (key, {})
            self._town_memo = memo
        hit = memo[1].get(f)
        if hit is not None:
            return list(hit)
        out = self._alive_towns(f)
        memo[1][f] = out
        return list(out)

    def _alive_towns(self, f=None):
        idx = getattr(self, "_town_idx", None)
        if idx is None or idx[0] != self.year:
            allt = [s for s in self.settlements if s.alive]
            by = {}
            for s in allt:
                by.setdefault(s.faction, []).append(s)
            idx = (self.year, allt, by)
            self._town_idx = idx
        if f is None:
            out = [s for s in idx[1] if s.alive]
            if getattr(self, "_moved", None) and getattr(self, "_moved_year", None) == self.year:
                seen = {id(s) for s in out}
                out += [s for s in self._moved if s.alive and id(s) not in seen]
            return out
        out = [s for s in idx[2].get(f, ()) if s.alive and s.faction == f]
        if getattr(self, "_moved", None) and getattr(self, "_moved_year", None) == self.year:
            seen = {id(s) for s in out}
            out += [s for s in self._moved if s.alive and s.faction == f and id(s) not in seen]
        return out

    def member_ids(self, f):
        """Ids of the living who serve realm f. Indexed once a year; anyone who changed realm since is re-checked."""
        idx = getattr(self, "_mem_idx", None)
        if idx is None or idx[0] != self.year:
            by = {}
            for i in self.living:
                by.setdefault(self.chars[i].realm, []).append(i)
            idx = (self.year, by)
            self._mem_idx = idx
        chars = self.chars
        return [i for i in idx[1].get(f, ()) if i in self.living and chars[i].realm == f]

    def members(self, f):
        return [self.chars[i] for i in self.member_ids(f)]

    def town_moved(self, s):
        """A town changed hands or was founded mid-year: keep it visible until the yearly re-index."""
        self._town_ver = getattr(self, "_town_ver", 0) + 1
        if getattr(self, "_moved_year", None) != self.year:
            self._moved, self._moved_year = [], self.year
        self._moved.append(s)

    def capital_of(self, f):
        for s in self.alive_towns(f):
            if s.capital:
                return s
        return None

    # ---------------- land
    def make_land(self):
        geography.make_world(self)
        resources.place_resources(self)
        self.eco = ecology.Ecology(self)
        self.dungeon_log = []

    def place_capitals(self, per_people=8):
        """The peoples begin on three continents (two share the greatest) as scattered tribes."""
        H, W = self.H, self.W
        conts = sorted((d for d in self.lands if d["kind"] == "continent"), key=lambda d: -d["size"])
        homes = [conts[0]["id"], conts[0]["id"], conts[min(1, len(conts) - 1)]["id"], conts[min(2, len(conts) - 1)]["id"]]
        order = [int(f) for f in self.rng.permutation(4)]
        yy, xx = np.ogrid[0:H, 0:W]
        seats, placed = [], []
        self.home_land = {}
        self.known = []
        for f, lid in zip(order, homes):
            base = box_sum(FOOD[f][self.terrain], 3)
            base[(self.terrain == OCEAN) | (self.landmass != lid)] = -1e9
            base[:6], base[-6:], base[:, :6], base[:, -6:] = -1e9, -1e9, -1e9, -1e9
            for p in placed:                                            # peoples sharing a continent keep apart
                base[np.hypot(yy - p[0], xx - p[1]) < np.sqrt(self.lands[lid]["size"]) * 0.35] = -1e9
            mine = []
            for n in range(per_people):
                score = base + self.rng.random((H, W)) * 3
                for p in mine:
                    score[np.hypot(yy - p[0], xx - p[1]) < 10] = -1e9
                if score.max() < -1e8:
                    break
                r, c = np.unravel_index(int(score.argmax()), score.shape)
                mine.append((int(r), int(c)))
                if n == 0:                                               # the rest scatter around the first
                    base[np.hypot(yy - r, xx - c) > np.sqrt(self.lands[lid]["size"]) * 0.45] = -1e9
            placed += mine
            seats += [(f, p) for p in mine]
        from . import dynasty
        dynasty.found_kingdoms(self, seats)

    def add_settlement(self, f, pos, pop, capital=False, quiet=False):
        cu = self.cul(f)
        s = Settlement(self.next_id, self.name(TOWN_SYL[cu]), f, pos, pop, self.year, capital)
        s.culture = cu
        r, c = pos
        s.landmass = int(self.landmass[pos])
        s.port = bool(self.coast[max(0, r - 1):r + 2, max(0, c - 1):c + 2].any())
        r0, r1, c0, c1 = max(0, r - 3), min(self.H, r + 4), max(0, c - 3), min(self.W, c + 4)
        yy, xx = np.ogrid[r0:r1, c0:c1]
        self.blocked[r0:r1, c0:c1] |= np.hypot(yy - r, xx - c) <= 3.5
        self.next_id += 1
        self.settlements.append(s)
        self.by_id[s.id] = s
        self.town_moved(s)
        self.founded[f] += 1
        if not quiet:
            n = sum(1 for t in self.settlements if t.alive and t.faction == f)
            if self.founded[f] <= 3:
                self.say(f"The {self.pname(f)} found {s.name}.", minor=True)
            elif n % 25 == 0:
                self.say(f"With the founding of {s.name}, the {self.pname(f)} now hold {n} towns.")
        return s

    def destroy(self, s):
        governance.lose_lordship(self, s)
        self._town_ver = getattr(self, "_town_ver", 0) + 1
        s.alive = False
        self.win["destroyed"] += 1
        self.ruins.append(Ruin(f"the ruins of {s.name}", s.pos, 1 + min(5, int(s.wealth / 40)), 1 + s.walls * 0.4))

    # ---------------- territory and economy
    def compute_territory(self):
        H, W = self.H, self.W
        owner = np.full((H, W), -1, int)
        best = np.zeros((H, W))
        alive = self.alive_towns()
        kern = KERNELS
        for k, s in enumerate(alive):
            rad = int(2 + min(6, math.sqrt(max(s.pop, 1)) / 8))
            r0, c0 = s.pos
            rlo, rhi, clo, chi = max(0, r0 - rad), min(H, r0 + rad + 1), max(0, c0 - rad), min(W, c0 + rad + 1)
            Dk, inv = kern[rad]
            sl = (slice(rlo - r0 + rad, rhi - r0 + rad), slice(clo - c0 + rad, chi - c0 + rad))
            D = Dk[sl]
            ok = (D <= rad) & ((self.terrain[rlo:rhi, clo:chi] != OCEAN) | (D <= 2))
            p = (max(s.pop, 1) ** 0.35 * (1.6 if s.capital else 1.0)) * inv[sl]
            sb, so = best[rlo:rhi, clo:chi], owner[rlo:rhi, clo:chi]
            upd = ok & (p > sb)
            sb[upd] = p[upd]
            so[upd] = k
        self.owner, self.cache = owner, alive
        fac = np.array([s.faction for s in alive] + [-1])
        self.owner_fac = fac[owner]

    def economy(self):
        alive = self.cache
        n = len(alive)
        if not n:
            return
        y = self.year
        fac = np.array([s.culture for s in alive])
        m = self.owner >= 0
        o, t = self.owner[m], self.terrain[m]
        f = fac[o]
        food = np.bincount(o, FOOD[f, t], n)
        wealth = np.bincount(o, WEALTH[f, t], n)
        mana = np.bincount(o, self.mana[m], n)
        cells = np.bincount(o, minlength=n)
        economy.town_year(self, alive, food, wealth, mana, cells)
        markets.labour_year(self, alive, wealth)

    def expand(self):
        H, W, rng = self.H, self.W, self.rng
        for s in list(self.alive_towns()):
            f = s.faction
            cu = s.culture
            if s.pop < SETTLE_AT[cu] or s.wealth < 30 or rng.random() > 0.25:
                continue
            ang = rng.uniform(0, 2 * np.pi, 60)
            rad = rng.uniform(4, 10, 60)
            rr = np.round(s.pos[0] + rad * np.sin(ang)).astype(int)
            cc = np.round(s.pos[1] + rad * np.cos(ang)).astype(int)
            ok = (rr >= 1) & (rr < H - 1) & (cc >= 1) & (cc < W - 1)
            rr, cc = rr[ok], cc[ok]
            if not len(rr):
                continue
            ter = self.terrain[rr, cc]
            of = self.owner_fac[rr, cc]
            ok = ((ter != OCEAN) & ((ter != MOUNTAIN) | (cu == 2)) & ((of == -1) | (of == f))
                  & ~self.blocked[rr, cc] & (self.landmass[rr, cc] == s.landmass))
            rr, cc = rr[ok], cc[ok]
            if not len(rr):
                continue
            score = np.array([FOOD[cu][self.terrain[max(0, r - 2):r + 3, max(0, c - 2):c + 3]].sum()
                              + 3 * self.mana[r, c] + 6 * (self.res_grid[max(0, r - 2):r + 3, max(0, c - 2):c + 3] >= 0).sum()
                              for r, c in zip(rr, cc)]) + rng.random(len(rr))
            i = int(score.argmax())
            if score[i] < 18:
                continue
            settlers = 0.3 * s.pop
            s.pop -= settlers
            s.wealth -= 30
            self.add_settlement(f, (int(rr[i]), int(cc[i])), settlers)

    def borders(self):
        for t in self.alive_towns():
            t.trade = 0.0
        o, F = self.owner, self.owner_fac
        pf = np.zeros((self.npol, self.npol))
        pairs = set()
        for a, b, fa, fb in ((o[:, :-1], o[:, 1:], F[:, :-1], F[:, 1:]), (o[:-1], o[1:], F[:-1], F[1:])):
            m = (fa >= 0) & (fb >= 0) & (fa != fb)
            np.add.at(pf, (fa[m], fb[m]), 1)
            if m.any():
                sp = np.sort(np.stack([a[m], b[m]], 1), 1)
                pairs.update(map(tuple, np.unique(sp, axis=0).tolist()))
        return pf + pf.T, pairs

    def calamity_alive(self):
        return [m for m in self.monsters if m.alive and m.kind == "calamity"]

    def diplomacy(self, pf, pairs):
        rng = self.rng
        cals = self.calamity_alive()
        lands = [set() for _ in range(self.npol)]
        for t in self.alive_towns():
            lands[t.faction].add(t.landmass)
        for i in range(self.npol):
            for j in range(i + 1, 4):
                if self.extinct[i] or self.extinct[j]:
                    continue
                # a calamity silences the wars only of peoples who both live on its continent (any, if a world one)
                li, lj = lands[i], lands[j]
                cal = [m for m in cals if m.world or (m.landmass in li and m.landmass in lj)]
                ki, kj = self.kingdoms[i], self.kingdoms[j]
                ri, rj = self.chars.get(ki.ruler), self.chars.get(kj.ruler)
                rel = self.relation[i, j]
                rel += 0.08 * ((BASE_REL[self.cul(i), self.cul(j)] - (20 if self.cul(i) == self.cul(j) else 0)) - rel) + rng.normal(0, 3)
                if ri is not None and rj is not None:
                    rel += (ri.attr("dip") + rj.attr("dip") - 16) * 0.05
                allied = j in ki.allies
                if allied:
                    rel += 2
                contact = pf[i, j] > 0 or seafaring.naval_contact(self, i, j)
                if pf[i, j] > 0:
                    rel -= min(8.0, 0.15 * pf[i, j]) * (AGGRESSION[self.cul(i)] + AGGRESSION[self.cul(j)]) / 2
                elif contact:
                    rel -= 1.0
                if self.war[i, j]:
                    rel -= 1.0
                rel = float(np.clip(rel, -100, 100))
                self.relation[i, j] = self.relation[j, i] = rel
                if not self.war[i, j]:
                    p = 0.15 * max(AGGRESSION[self.cul(i)] * ki.aggr, AGGRESSION[self.cul(j)] * kj.aggr)
                    if self.religions:
                        from .society import tenet_aggression
                        p *= max(tenet_aggression(self, ki), tenet_aggression(self, kj))
                        if ki.faith != kj.faith:
                            rel -= 1.0
                    tribal = ki.rank in ("tribe", "chiefdom") or kj.rank in ("tribe", "chiefdom")
                    if tribal:
                        p *= 2.0                                          # chiefs raid and feud
                    if (contact and not allied and not cal and self.year >= self.truce[i, j]
                            and rel < (-20 if tribal else -45)
                            and rng.random() < p):
                        a, b = ((i, j) if AGGRESSION[self.cul(i)] * ki.aggr * rng.random() > AGGRESSION[self.cul(j)] * kj.aggr * rng.random()
                                else (j, i))
                        cb = statecraft.cb_against(self, self.kingdoms[a], self.kingdoms[b])
                        if cb is None:
                            dk = statecraft.nearest_duchy(self, self.kingdoms[a], self.kingdoms[b])
                            cb = ("border war", {"duchy": dk}) if dk is not None else None
                        if cb is not None:
                            statecraft.start_war(self, a, b, cb)
                else:
                    w = self.weariness[i, j] + self.weariness[j, i]
                    long = self.year - self.war_since[i, j]
                    end = (statecraft.war_of(self, i, j) is None
                           and (w > 0.25 + rng.random() * 0.3 or long > 40 or not contact) and rng.random() < 0.35)
                    if cal and rng.random() < 0.3:
                        end = True
                    if end:
                        self.war[i, j] = self.war[j, i] = False
                        self.relation[i, j] = self.relation[j, i] = -15
                        self.truce[i, j] = self.truce[j, i] = self.year + 15 + int(rng.integers(0, 20))
                        if cal:
                            self.say(f"**Peace.** Facing {cal[0].name}, the {self.pname(i)} and the {self.pname(j)} set "
                                     f"aside {long} years of war.")
                        else:
                            towns = self.alive_towns(i) + self.alive_towns(j)
                            place = towns[int(rng.integers(len(towns)))].name if towns else "the border"
                            self.say(f"**Peace.** The Peace of {place} ends {long} years of war between the "
                                     f"{self.pname(i)} and the {self.pname(j)}.")
        for a, b in pairs:
            sa, sb = self.cache[a], self.cache[b]
            if sa.alive and sb.alive and not self.war[sa.faction, sb.faction]:
                sa.trade = getattr(sa, "trade", 0.0) + 1
                sb.trade = getattr(sb, "trade", 0.0) + 1
                sa.wealth += 2
                sb.wealth += 2

    def battles(self, pairs, war_power):
        rng = self.rng
        y = self.year
        won = [0] * self.npol
        fpop = [0.0] * self.npol
        for t in self.alive_towns():
            fpop[t.faction] += t.pop
        hero_mult = [1 + min(1.5, war_power[f] * self.kingdoms[f].mod("war_power", y) / max(1.0, fpop[f] * 0.1))
                     for f in range(self.npol)]
        fought = set()
        pairs = list(pairs)
        rng.shuffle(pairs)
        for a, b in pairs:
            A, D = self.cache[a], self.cache[b]
            fa, fd = A.faction, D.faction
            if (not A.alive or not D.alive or fa == fd or not self.war[fa, fd] or a in fought or b in fought
                    or rng.random() > 0.25):
                continue
            fought.update((a, b))
            ka, kd = self.kingdoms[fa], self.kingdoms[fd]
            if rng.random() > AGGRESSION[self.cul(fa)] * ka.aggr / (AGGRESSION[self.cul(fa)] * ka.aggr + AGGRESSION[self.cul(fd)] * kd.aggr):
                A, D, fa, fd, ka, kd = D, A, fd, fa, kd, ka
            pa = A.pop * 0.3 * MARTIAL[self.cul(fa)] * (1 + 0.5 * A.magic) * hero_mult[fa] * ka.battle_mult
            pd = (D.pop * 0.3 * MARTIAL[self.cul(fd)] * (1 + 0.5 * D.magic) * (1 + 0.3 * D.walls) * DEFENSE[self.cul(fd)]
                  * hero_mult[fd] * kd.battle_mult + D.guard + getattr(D, "garrison", 0.0))
            ra, rd = pa * rng.uniform(0.6, 1.4), pd * rng.uniform(0.6, 1.4)
            frac = ra / (ra + rd + 1e-9)
            la, ld = A.pop * 0.12 * (1 - frac), D.pop * 0.2 * frac
            A.pop -= la
            D.pop -= ld
            self.weariness[fa, fd] += la / max(1.0, fpop[fa])
            self.weariness[fd, fa] += ld / max(1.0, fpop[fd])
            self.win["battles"] += 1
            if ra > rd:
                won[fa] += 1
                statecraft.battle_result(self, fa, fd, D if D.pop < A.pop * 0.7 else None)
                if D.pop < A.pop * 0.7 and rng.random() < 0.08:      # border skirmishes rarely take a town now
                    was_cap = D.capital
                    ruler = self.chars.get(ka.ruler)
                    cruel = ruler is not None and "cruel" in ruler.personality
                    if rng.random() < (0.4 if self.cul(fa) == 3 else 0.05) + (0.2 if cruel else 0):
                        self.say(f"The {self.pname(fa)} of {A.name} sack and burn {D.name}"
                                 f"{', seat of ' + kd.name if was_cap else ''}.", minor=not was_cap)
                        self.destroy(D)
                        kd.stats["lost"] += 1
                        if cruel:
                            governance.add_tyranny(self, ruler, 4, f"the burning of {D.name}")
                    else:
                        self.say(f"{D.name}{', seat of ' + kd.name + ',' if was_cap else ''} falls to the "
                                 f"{self.pname(fa)} of {A.name}.", minor=not was_cap)
                        governance.lose_lordship(self, D)
                        D.faction, D.capital, D.pop, D.walls = fa, False, D.pop * 0.6, max(0, D.walls - 1)
                        self.town_moved(D)
                        kd.stats["lost"] += 1
                        self.win["captures"] += 1
                    ka.stats["taken"] += 1
            else:
                won[fd] += 1
        for f in range(self.npol):                                  # kings who ride with their armies
            if self.war[f].any() and not self.extinct[f]:
                r = self.chars.get(self.kingdoms[f].ruler)
                if r is not None and r.alive and "brave" in r.personality and not r.undead and r.adult(y) \
                        and rng.random() < 0.015:
                    fid = int(np.flatnonzero(self.war[f])[0])
                    foe = self.pname(fid)
                    self.kill(r, f"fell leading the host against the {foe}", by=self.chars.get(self.kingdoms[fid].ruler))
        return won

    # ---------------- monsters and calamities
    def remote_cells(self, mask, sample=4000):
        cells = np.argwhere(mask)
        if len(cells) > sample:
            cells = cells[self.rng.choice(len(cells), sample, replace=False)]
        if not len(cells):
            return cells
        d = self.town_dist[cells[:, 0], cells[:, 1]]
        return cells[d >= min(8.0, max(3.0, 0.8 * d.max()))]

    def spawn_monsters(self):
        """Monsters arise in the wild of every landmass; the more land, the more of them.
        Calamities wake one continent at a time; only a rare few are great enough to threaten the whole world."""
        rng = self.rng
        land = self.terrain != OCEAN
        wild = float(((self.owner == -1) & land).sum() / max(1, land.sum()))
        scale = max(1, sum(1 for d in self.lands if d["kind"] != "isle"))
        for kind, (terr, prange, _, _, _, cap, _, p0) in MONSTER.items():
            n = sum(1 for m in self.monsters if m.alive and m.kind == kind)
            for _ in range(scale):
                if n >= cap * scale or rng.random() > p0 * (wild * 2 + 0.3):
                    continue
                if kind == "troll":
                    mask = np.isin(self.terrain, (HILLS, FOREST)) & (self.owner == -1)
                elif terr == "mana":
                    mask = self.mana > 0.6
                else:
                    mask = self.terrain == terr
                cells = self.remote_cells(mask)
                if not len(cells):
                    continue
                w = self.mana[cells[:, 0], cells[:, 1]] + 0.05
                r, c = cells[rng.choice(len(cells), p=w / w.sum())]
                power = rng.uniform(*prange)
                if kind == "demon" and rng.random() < 0.5:              # the veil holds more often than not
                    continue
                self.new_monster(kind, (int(r), int(c)), power)
                n += 1
        # calamities: at most one per settled continent; a world calamity is rare and twice as strong
        settled = {s.landmass for s in self.alive_towns()}
        busy = {m.landmass for m in self.calamity_alive()}
        for lid in settled - busy:
            if self.lands[lid]["kind"] == "isle":
                continue
            if self.year > 150 and self.year - self.last_cal.get(lid, 0) > 200 and rng.random() < 0.004:
                cells = self.remote_cells(land & (self.landmass == lid))
                if not len(cells):
                    continue
                r, c = cells[int(rng.integers(len(cells)))]
                world = rng.random() < 0.12 and not any(m.world for m in self.calamity_alive())
                m = self.new_monster("calamity", (int(r), int(c)), rng.uniform(6000, 20000) * (2 if world else 1),
                                     world=world)
                self.last_cal[lid] = self.year

    def new_monster(self, kind, pos, power, name=None, parents=(), quiet=False, world=False):
        rng = self.rng
        near = self.nearest_town_name(pos)
        if name is not None:
            pass
        elif kind == "dragon":
            name = f"{self.name(DRAGON_SYL)} {rng.choice(DRAGON_EPITHET)}"
            self.say(f"**A dragon.** {name} comes down out of the peaks and makes a lair near {near}.")
        elif kind == "demon":
            top = max(5, getattr(self, "world_tier", 0) + 2)             # no more than two tiers past the best mortal
            power = min(power, (top - 4) * 600 + 599)
            name = f"{self.name(DEMON_SYL)}, a demon of the {TIER_WORDS[int(min(10, 4 + power // 600))]} tier"
            self.say(f"A ley-nexus tears near {near} and {name} steps through.")
        elif kind == "wraith":
            name = f"the Wraith of the {rng.choice(WILD_A)} {rng.choice(WILD_B[SWAMP])}"
            self.say(f"Something stirs in the marshes near {near}: {name} rises.", minor=True)
        elif kind == "calamity":
            name = f"{self.name((CALAMITY_A, CALAMITY_B))}, {rng.choice(CALAMITY_TITLE)}"
            where = geography.land_name(self, int(self.landmass[pos]))
            if world:
                self.say(f"**A world calamity wakes.** Beneath {where}, near {near}, {name} opens its eyes, and every "
                         f"shore of the world feels it. No continent is safe while it lives.")
            else:
                self.say(f"**A calamity wakes on {where}.** Beneath the land near {near}, {name} opens its eyes. Its "
                         f"strength is beyond any single hero living, and the wars of {where} falter.")
        else:
            t = int(self.terrain[pos])
            name = f"the trolls of the {rng.choice(WILD_A)} {rng.choice(WILD_B.get(t, ['Wilds']))}"
        m = Monster(kind, name, pos, power, self.year)
        m.landmass, m.world = int(self.landmass[pos]), world
        m.mid = self.next_mid
        self.next_mid += 1
        monsters.new_lineage_fields(m, rng, parents)
        if kind == "dragon" and m.traits:
            m.name += f" ({', '.join(sorted(m.traits))})" if not parents else ""
        self.monsters.append(m)
        return m

    def nearest_town_name(self, pos):
        towns = self.alive_towns()
        return min(towns, key=lambda s: dist(s.pos, pos)).name if towns else "nowhere"

    def raids(self):
        rng = self.rng
        for m in list(self.monsters):
            if not m.alive:
                continue
            if m.kind == "calamity":
                self.calamity_turn(m)
                continue
            if getattr(m, "sea", False):
                continue
            _, _, rng_max, chance, loss, _, _, _ = MONSTER[m.kind]
            if m.power < m.base:
                m.power += 0.05 * (m.base - m.power)
            if m.kind == "dragon":
                m.base += 10
            if m.kind == "troll" and self.year - m.born > 80 and rng.random() < 0.05:
                m.alive = False
                continue
            if self.year < m.sleep_until or rng.random() > chance:
                continue
            near = [s for s in self.alive_towns() if s.landmass == m.landmass and dist(s.pos, m.pos) <= rng_max]
            if not near:
                continue
            w = np.array([s.wealth + 10 for s in near])
            s = near[rng.choice(len(near), p=w / w.sum())]
            self.assault(m, s, loss)

    def assault(self, m, s, loss):
        rng = self.rng
        f = s.faction
        kd = self.kingdoms[f]
        d = (getattr(s, "garrison", 0.0) + s.pop * 0.15 * MARTIAL[s.culture] * (1 + 0.3 * s.walls) * DEFENSE[s.culture] * (1 + 0.5 * s.magic)
             * (1 + 0.12 * kd.realm_tier) + s.guard) * (1 + min(0.5, getattr(kd, "scroll_power", 0) / 3000))
        rm, rd = m.power * rng.uniform(0.5, 1.5), d * rng.uniform(0.5, 1.5)
        s.raided_by = m
        self.win["raids"] += 1
        if rm > rd:
            s.raided = "lost"
            s.granary = getattr(s, "granary", 0.0) * 0.5
            s.pop *= 1 - loss * rng.uniform(0.5, 1.2)
            stolen = s.wealth * (0.5 if m.kind == "dragon" else 0.2)
            s.wealth -= stolen
            m.hoard += stolen
            m.power *= 1.03
            arcana.raid_shock(self, m, s)
            if m.kind == "dragon":
                m.sleep_until = self.year + int(rng.integers(4, 15))
            if s.pop < 15:
                self.say(f"{s.name} of the {self.pname(f)} is laid waste by {m.name}.")
                self.destroy(s)
            elif s.capital and m.kind in ("dragon", "demon", "calamity") and self.year - m.last_said >= 30:
                m.last_said = self.year
                self.say(f"{m.name} falls upon {s.name}, seat of {kd.name}.")
            if s.capital and m.kind in ("dragon", "calamity") and rng.random() < 0.1:
                r = self.chars.get(kd.ruler)
                if r is not None and r.alive and not r.undead:
                    self.kill(r, f"was killed when {m.name} fell upon {s.name}", killer=m)
        else:
            s.raided = "repelled"
            m.power -= rd * 0.25
            if m.power <= 0:
                m.alive = False
                monsters.on_monster_slain(self, m)
                self.win["slain"] += 1
                s.wealth += m.hoard
                if m.items:
                    self.kingdoms[f].treasury.extend(m.items)
                    m.items = []
                if m.kind != "troll":
                    self.say(f"The defenders of {s.name} ({self.pname(f)}) bring down {m.name}.",
                             minor=m.kind not in ("dragon", "calamity"))
                if m.kind == "calamity":
                    self.calamity_log.append((m.name, m.born, self.year, f"brought down by the defenders of {s.name}"))

    def calamity_turn(self, m):
        rng = self.rng
        if self.year - m.born > 40 and rng.random() < 0.03:
            m.alive = False
            self.last_cal[m.landmass] = self.year
            self.calamity_log.append((m.name, m.born, self.year, "sank back into slumber, unbeaten"))
            self.say(f"**{m.name} sinks back into the deep**, unbeaten, after {self.year - m.born} years. "
                     f"It will dream, and one day it will wake.")
            legends.on_slumber(self, m)
            return
        m.base += 100
        if m.power < m.base:
            m.power += 0.03 * (m.base - m.power)
        reach = 25
        near = [s for s in self.alive_towns() if (m.world or s.landmass == m.landmass) and dist(s.pos, m.pos) <= reach]
        if near and rng.random() < 0.6:
            self.assault(m, near[int(rng.integers(len(near)))], 0.4)
        if rng.random() < 0.15:
            kind = str(rng.choice(["troll", "wraith", "demon"]))
            pos = geography.clip(self, (m.pos[0] + rng.integers(-6, 7), m.pos[1] + rng.integers(-6, 7)))
            if self.terrain[pos] != OCEAN:
                self.new_monster(kind, pos, rng.uniform(*MONSTER[kind][1]))
        step = 40 if m.world else 8
        if rng.random() < (0.15 if m.world else 0.08):                # it moves; a world calamity crosses seas
            pos = geography.clip(self, (m.pos[0] + rng.integers(-step, step + 1), m.pos[1] + rng.integers(-step, step + 1)))
            if self.terrain[pos] != OCEAN and (m.world or self.landmass[pos] == m.landmass):
                if m.world and self.landmass[pos] != m.landmass:
                    self.say(f"{m.name} crosses the sea to {geography.land_name(self, int(self.landmass[pos]))}.")
                m.pos, m.landmass = pos, int(self.landmass[pos])

    def events(self):
        rng = self.rng
        if self.blight > 0:
            self.blight -= 1
        elif rng.random() < 0.008:
            self.blight = int(rng.integers(1, 4))
            self.say(f"Blight. The harvests fail across the land for {self.blight} year"
                     f"{'s' if self.blight > 1 else ''}.")

    def cleanup(self):
        gone = [s for s in self.alive_towns() if s.pop < 15]
        for s in gone:
            self.destroy(s)
        if gone:
            self.say(f"{len(gone)} town{'s are' if len(gone) > 1 else ' is'} abandoned: "
                     + ", ".join(f"{s.name} ({self.pname(s.faction)})" for s in gone) + ".", minor=True)
        by = {}
        for t in self.alive_towns():
            by.setdefault(t.faction, []).append(t)
        for f in range(self.npol):
            if self.extinct[f]:
                continue
            towns = by.get(f, [])
            if not towns:
                self.extinct[f] = True
                self.war[f, :] = self.war[:, f] = False
                k = self.kingdoms[f]
                folk = PEOPLES[k.people]
                still = any(t.culture == k.people for t in self.alive_towns())
                self.say(f"**{k.name} is no more.** Its last town has fallen."
                         + ("" if still else f" The {folk} are gone from the world."))
                r = self.chars.get(k.ruler)
                for i in list(self.living):                       # its people serve whoever holds their home now
                    c = self.chars[i]
                    if c.realm == f:
                        home = self.by_id.get(c.home) if c.home is not None else None
                        c.realm = home.faction if home is not None and home.alive else self.nearest_realm(c.people)
                continue
            if self.capital_of(f) is None:
                s = max(towns, key=lambda t: t.pop)
                s.capital = True
                self.say(f"{self.kingdoms[f].name} makes {s.name} its new seat.")
        self.monsters = [m for m in self.monsters if m.alive]

    def step(self):
        self.year += 1
        geography.town_distance(self)
        self.compute_territory()
        resources.realm_resources(self)
        economy.weather_year(self)
        self.economy()
        self.expand()
        dynasty.dynasty_year(self)
        governance.govern_year(self)
        youth.youth_year(self)
        titles.titles_year(self)
        civics.civics_year(self)
        court.courts_and_people_year(self)
        opinion.opinion_year(self)
        heart.meters_year(self)
        godmode.snapshot(self)
        heart.hearts_year(self)
        statecraft.statecraft_year(self)
        markets.trade_year(self)
        markets.money_year(self)
        stewardship.manage_year(self)
        arcana.arcana_year(self)
        populace.moods_year(self)
        populace.plague_year(self)
        warfare.armies_year(self)
        crafts.spawn_specialists(self)
        crafts.specialists_year(self)
        society.guilds_year(self)
        society.crime_year(self)
        society.religion_year(self)
        shinobi.shinobi_year(self)
        bending.bending_year(self)
        seafaring.sea_year(self)
        beasts.beasts_year(self)
        legends.legends_year(self)
        ecology.ecology_year(self)
        dungeons.dungeons_year(self)
        monsters.monster_year(self)
        pf, pairs = self.borders()
        self.diplomacy(pf, pairs)
        self.events()
        war_power = heroes.heroes_act(self)
        won = self.battles(pairs, war_power)
        self.spawn_monsters()
        self.raids()
        heroes.heroes_resolve(self, won)
        self.cleanup()
        heroes.spawn_heroes(self)
        if self.year % 10 == 0:
            self.tier_hist.append((self.year, list(self.people_tier)))
