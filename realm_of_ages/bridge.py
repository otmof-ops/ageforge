"""The Rust world, as the screens and the books read it.

The simulation runs in Rust (``_core.World``). After a run of years, ``Mirror`` builds a read-only view from one
snapshot: realms, towns, characters, items, monsters and the rest, with the Python engine's own attribute and method
names, so the TUI, the books and the maps work on it unchanged. Nothing written to a mirror reaches the world; acts
go through ``World.god``."""
import numpy as np

from .data import CLASSES, PEOPLES, PRESTIGE, RULER_TITLE, SINGULAR
from .data2 import SHINOBI_RANKS
from .events import classify


class Obj:
    """A record from the snapshot: its keys are its attributes."""

    def __init__(self, d):
        self.__dict__.update(d)

    def __repr__(self):
        return f"<{type(self).__name__} {self.__dict__.get('name', '')}>"


class MItem(Obj):
    def __init__(self, d):
        d = dict(d)
        d["_label"] = d.pop("label")
        super().__init__(d)

    def label(self):
        return self._label


class MChar(Obj):
    legacy_origin = {}          # where each legacy came from: the Rust world keeps the legacies, not their origins

    def __init__(self, d, realm):
        d = dict(d)
        for k in ("tier", "school", "cr", "known_spells", "items", "eff_talent", "eff_prowess", "hp_mult"):  # the methods return these
            d["_" + k] = d.pop(k)
        super().__init__(d)
        self._realm = realm
        self.genetic = set(d["genetic"])
        self.latent = set(d["latent"])
        self.personality = set(d["personality"])
        self.classes = [[c, n] for c, n in d["classes"]]
        self.ruled = tuple(d["ruled"]) if d["ruled"] is not None else None
        self.tracked = d["id"] in realm.tracked
        self.journal = realm.journals.get(d["id"], [])
        b = d["beast"]
        self.beast = {"kind": b[0], "name": b[1], "power": b[2], "since": b[3], "life": b[4]} if b else None
        sh = d["ship"]
        self.ship = {"name": sh[0], "tier": sh[1], "crew": sh[2]}
        self.pet = {"kind": d["pet"][0], "name": d["pet"][1], "since": d["pet"][2], "life": d["pet"][3]} if d["pet"] else None
        self.loves = {k: list(v) for k, v in d["loves"].items()}
        self.branch = {tuple(k): v for k, v in d["branch"].items()}
        self.peak_tier = d["_tier"]
        self.relations = {}

    @property
    def items(self):
        return [self._realm.items[i] for i in self._items]

    def full_name(self, houses=None):
        n = self.name + (f" {self.regnal}" if self.regnal else "")
        return n + (f" {self.epithet}" if self.epithet else "")

    def house_name(self, houses=None):
        return self._realm.houses[self.house].name if self.house is not None else "no house"

    def age(self, year):
        return year - self.born

    def adult(self, year):
        from .data import ADULT
        return self.age(year) >= ADULT[self.people]

    def tier(self):
        return self._tier

    def school(self):
        return self._school

    def cr(self, hp_matters=True):
        return self.cr_now if hp_matters else self._cr

    def attr(self, a):
        return self.attr_now[a]

    def known_spells(self, n=3):
        return self._known_spells[:n]

    def tier_cap(self):
        from .data import TIER_CAP
        return 10 if self.custom else TIER_CAP[self.people] + ("starblood" in self.genetic)

    def level_cap(self):
        return 130 if "dragonblood" in self.genetic else 100

    def eff_talent(self):
        return self._eff_talent

    def eff_prowess(self):
        return self._eff_prowess

    def hp_mult(self):
        return self._hp_mult

    def class_def(self, cls):
        return CLASSES.get(cls) or PRESTIGE[cls]

    def mods(self):
        return {"phys": 1.0, "magic": 1.0, "hp": 1.0, "war": 1.0, "guard": 1.0, "vs_monster": 1.0, "delve": 1.0,
                "equip": 1.0, "tier": 0, "lead": 0, "intrigue": 0}

    def invalidate(self):
        pass


class MKingdom(Obj):
    def __init__(self, d, realm):
        super().__init__(d)
        self.treasury = [realm.items[i] for i in d["treasury"]]
        self.market = [realm.items[i] for i in d["market"]]
        self.laws = {"crown": d["crown"], "tolerance": "tolerant" if d["tolerant"] else "intolerant",
                     "trade": "protectionist" if d["protectionist"] else "open", "stance": "expansionist" if d["expansionist"] else "defensive"}
        self.stats = {"taken": 0, "lost": 0}
        self.allies = set(d["allies"])
        self.powerful = set()
        self.sentiment = {}
        self.mods = {}
        if not d["coin"]:
            self.money = None
            self.credit = None
        self.court = d["court"]

    @property
    def realm(self):
        return self.id

    def mod(self, key, year, default=1.0):
        return default


class MTown(Obj):
    def __init__(self, d):
        super().__init__(d)
        self.pos = tuple(d["pos"])
        self.faith = dict(d["faith"])
        self.duchy = tuple(d["duchy"]) if d["duchy"] is not None else None


class EcoView:
    """The living world as the books read it: species, their numbers by region, what happened."""

    def __init__(self, s):
        self.species = s["eco"]
        self.eh, self.ew = s["eco_shape"]
        n = len(self.species)
        self.pop = np.array(s["eco_pop"], float).reshape(n, self.eh, self.ew) if n else np.zeros((0, self.eh, self.ew))
        self.events = s["eco_events"]
        self.region_names = {tuple(k): v for k, v in s["eco_regions"]}

    def region(self, rng, ey, ex):
        return self.region_names.get((ey // 4, ex // 4), "the wild")


class HeroGuildView:
    """A people's hero guild as the summary reads it: totals, the deeds chosen, and its doctrine."""

    def __init__(self, g):
        (self.heroes_total, self.deaths_total, self.acts, self.hunts, self.hunt_wins, self.slain, self.states, self._doc) = g

    def doctrine(self, top=3):
        return [(n, v) for n, v, _ in self._doc[:top]]


class Mirror:
    """A read-only view of the Rust world, shaped like the Python engine's Realm."""

    def __init__(self, world, tracked=(), journals=None, event_log=None, seed=None):
        s = world.snapshot(0)
        g = world.grids()
        self.year, self.W, self.H, self.seed = s["year"], s["W"], s["H"], seed
        self.tracked = set(tracked)
        self.journals = journals or {}
        self.event_log = event_log if event_log is not None else []
        self.chron = []
        self.rng = np.random.default_rng(seed)
        self.terrain, self.owner_fac, self.owner, self.landmass = g["terrain"], g["owner_fac"], g["owner"], g["landmass"]
        self.mana, self.corrupted, self.res_grid = g["mana"], g["corrupted"], g["res_grid"]
        self.items = [MItem(d) for d in s["items"]]
        self.houses = {d["id"]: Obj(d) for d in s["houses"]}
        self.kingdoms = [MKingdom(d, self) for d in s["kingdoms"]]
        self.extinct = [k.extinct for k in self.kingdoms]
        self.settlements = [MTown(d) for d in s["towns"]]
        self.by_id = {t.id: t for t in self.settlements}
        self.chars = {d["id"]: MChar(d, self) for d in s["chars"]}
        self.living = {i for i, c in self.chars.items() if c.alive}
        self.hero_ids = {i for i in self.living if self.chars[i].hero}
        self.monsters = [Obj(dict(d, items=[self.items[i] for i in d["items"]])) for d in s["monsters"]]
        self.ruins = [Obj(dict(d, loot=[self.items[i] for i in d["loot"]], boss={"name": d["boss"][0], "power": d["boss"][1],
                                                                             "mini": d["boss"][2]})) for d in s["ruins"]]
        self.lands = [dict(d, found_by={f: y for f, y in d["found_by"]}) for d in s["lands"]]
        self.cultures = [Obj(dict(d, ethos=["Courtly", "Spiritual", "Stoic", "Bellicose"][d["people"]], innovations=dict(d["innovations"]), trend=dict(zip(("auth", "succ", "faith", "econ", "trad"), d["trend"]))))
                         for d in s["cultures"]]
        self.religions = [Obj(d) for d in s["religions"]]
        self.guilds_soc = [Obj({"name": n, "kind": k, "people": p, "members": set(m), "master": ms, "gold": gl})
                           for n, k, p, m, ms, gl in s["guilds_soc"]]
        self.epidemics = [Obj({"name": n, "start": st, "end": e, "fatality": f, "great": gr, "deaths": de,
                               "towns": set(range(nt))}) for n, st, e, f, gr, de, nt in s["epidemics"]]
        self.wars = {(min(w["attacker"], w["defender"]), max(w["attacker"], w["defender"])): w for w in s["wars"]}
        self.war_log = s["war_log"]
        self.battles_fought = s["battles_fought"]
        self.armies = [Obj({"name": n, "men": m, "pos": tuple(p), "siege": sg, "realm": r, "side": "rebel" if rb else "realm"})
                       for n, m, p, sg, r, rb in s["armies"]]
        n = len(self.kingdoms)
        self.war = np.array(s["war"], bool).reshape(n, n) if n else np.zeros((0, 0), bool)
        self.relation = np.array(s["relation"], float).reshape(n, n) if n else np.zeros((0, 0))
        self.war_since = np.array(s["war_since"]).reshape(n, n) if n else np.zeros((0, 0))
        for name in ("famines", "risings", "bankruptcies", "naval_battles", "tier_events", "rites", "myths",
                     "sovereign_line", "calamity_log", "world_tier", "breakaways", "dungeon_log", "dragon_riders",
                     "merges", "pirate_kings", "migrants", "concord_line", "cycles", "gate_history", "blight"):
            setattr(self, name, s[name])
        self.people_tier = list(s["people_tier"])
        self.cycles = [Obj(dict(c, relic=Obj({"name": c["relic"]}), blade=Obj({"name": c["blade"]}) if c["blade"] else None,
                                ages=[{"outcome": a} for a in c["ages"]], age={}, ruler=None)) for c in s["cycles"]]
        self.curses = s["curses"]
        self.legend_items = [it for it in self.items if it.rarity == 5 and it.special == "vs_monster" and it.origin.startswith("forged by")]
        self.known = [set(k.known) for k in self.kingdoms]
        self.home_land = {k.id: k.home_land for k in self.kingdoms}
        self.pirates = list(s["pirates"])
        self.havens = [{"name": nm, "pos": tuple(p), "land": l} for nm, p, l in s["havens"]]
        self.tribes = {e: Obj({"name": nm, "element": e, "pos": tuple(p), "members": set(m), "chief": ch})
                       for nm, e, p, m, ch in s["tribes"]}
        self.clans = [Obj({"id": i, "name": nm, "pos": tuple(p), "members": set(m), "head": h, "gold": g, "standing": st,
                           "pact": pact, "missions": {"won": won, "lost": lost}})
                      for i, (nm, p, m, h, g, st, pact, won, lost) in enumerate(s["clans"])]
        self.concord_line = [{"id": i, "born": b, "element": e, "need": nd, "revealed": rv, "died": dd, "deeds": de}
                             for i, b, e, nd, rv, dd, de in s["concord_line"]]
        self.eco = EcoView(s)
        self.concord = s["concord"]
        self.econ_stats = {"defaults": s["defaults"], "forced": s["forced"], "mined": s["mined"]}
        self.ledger = {tuple(k): v for k, v in s["ledger"].items()}
        self.tx = s["tx"]
        self.greatest = dict(s["greatest"])
        self.duchies = {tuple(k): v for k, v in s["duchies"].items()}
        self.ktitles = {tuple(k): v for k, v in s["ktitles"].items()}
        self.op_mods = {tuple(k): [list(m) for m in v] for k, v in s["op_mods"].items()}
        self.gates = None if s["gates"] is None else dict(zip(("open", "start", "end", "lord_name", "count"), s["gates"]))
        self.worldforged_found = {it.name for it in self.items if it.rarity >= 6}
        self.wonders = [(0, k.id, w) for k in self.kingdoms for w in k.great_works]
        self.decisions, self.tier_hist, self.love_log = [], [], []
        h = s["hall"]
        self.guild = {"parties": {i: {"id": i, "name": nm, "rank": rk, "members": list(m), "done": dn, "lost": lo,
                                      "formed": fm, "landmass": -1, "slain": []}
                                  for i, (nm, rk, m, dn, lo, fm) in enumerate(h["parties"])},
                      "quests": {i: {"id": i, "rank": rk, "kind": kd, "title": t, "reward": rw, "posted": ps,
                                     "expires": ex, "target": None, "taken": None}
                                 for i, (rk, kd, t, rw, ps, ex) in enumerate(h["quests"])},
                      "log": h["log"], "next_q": len(h["quests"]), "next_p": len(h["parties"])}
        self.hero_guilds = s["hero_guilds"]
        self.guilds = [HeroGuildView(g) for g in s["hero_guilds"]]
        self.eco_species = s["eco"]
        self.awakenings = []
        self._alive = None

    # ---------------- the Realm interface the screens and books use
    @property
    def npol(self):
        return len(self.kingdoms)

    def alive_towns(self, f=None):
        if self._alive is None:
            self._alive = [t for t in self.settlements if t.alive]
        return self._alive if f is None else [t for t in self._alive if t.faction == f]

    @property
    def cache(self):
        return self.alive_towns()

    def capital_of(self, f):
        return next((t for t in self.alive_towns(f) if t.capital), None)

    def cul(self, f):
        return self.kingdoms[f].people

    def pname(self, f):
        k = self.kingdoms[f]
        return PEOPLES[k.people] if sum(1 for o in self.kingdoms if o.people == k.people) <= 1 else k.short

    def heroes(self):
        return [self.chars[i] for i in self.hero_ids]

    def member_ids(self, f):
        return [i for i in self.living if self.chars[i].realm == f]

    def members(self, f):
        return [self.chars[i] for i in self.member_ids(f)]

    def outlaw_ids(self, f):
        return [i for i in self.member_ids(f) if self.chars[i].profession in ("thief", "assassin", "bandit")]

    def house_ids(self, h):
        return [i for i in self.living if self.chars[i].house == h]

    def faith_of_town(self, s):
        f = getattr(s, "faith", None)
        return self.religions[max(f, key=f.get)] if f else None

    def nearest_realm(self, culture):
        alive = [k for k in self.kingdoms if not k.extinct]
        same = [k for k in alive if k.people == culture]
        pick = max(same or alive, key=lambda k: len(self.alive_towns(k.id)), default=None)
        return pick.id if pick is not None else 0

    def who(self, c):
        if c.ruled is not None and c.ruled[2] is None:
            return f"{RULER_TITLE[c.people][int(c.female)]} {c.full_name()}"
        if getattr(c, "concord", False):
            return f"{c.full_name()} ({SINGULAR[c.people]}, the Concord)"
        if c.clan is not None:
            return f"{c.full_name()} ({SINGULAR[c.people]} shinobi, {SHINOBI_RANKS[c.srank]})"
        if c.element is not None:
            return f"{c.full_name()} ({SINGULAR[c.people]} {c.element}-bender)"
        if c.profession:
            return f"{c.full_name()} ({SINGULAR[c.people]} {c.profession})"
        cls = c.classes[-1][0] if c.classes[-1][1] > 0 or len(c.classes) == 1 else c.classes[-2][0]
        return f"{c.full_name()} ({SINGULAR[c.people]} {cls} {c.level})"

    def calamity_alive(self):
        return [m for m in self.monsters if m.alive and m.kind == "calamity"]

    def say(self, text, minor=False, cat=None):
        self.chron.append((self.year, text, minor))

    def update(self, *a, **k):
        pass


def to_log(year, minor, text):
    """One chronicle entry as the event log keeps it: (year, category, major, text)."""
    cat = "god" if text.startswith("**God.**") else classify(text)
    return (year, cat, not minor, text)


__all__ = ["Mirror", "to_log"]
