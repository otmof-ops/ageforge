"""Game mode: a world you start, advance a hundred to a thousand years at a time, and end only when you choose.

Each world lives in its own folder under worlds/: the saved state (world.pkl), a small meta.json, the chronicle
and details logs (appended as the years pass), and, after every advance, a fresh set of books and a map. A world
can be closed and opened again later; ending it writes the last books and marks it finished."""
import json
import os
import re
import time

import numpy as np

from .bridge import Mirror, to_log
from .data import PEOPLES

PRESETS = [1, 5, 10, 50, 100, 200, 300, 500, 1000]
ROOT = os.environ.get("REALM_WORLDS", os.path.join(os.path.dirname(os.path.dirname(os.path.abspath(__file__))),
                                                    "worlds"))


def slug(name):
    return re.sub(r"[^a-z0-9]+", "-", name.lower()).strip("-") or "world"


def list_worlds():
    os.makedirs(ROOT, exist_ok=True)
    out = []
    for d in sorted(os.listdir(ROOT)):
        meta = os.path.join(ROOT, d, "meta.json")
        if os.path.exists(meta):
            with open(meta) as fh:
                m = json.load(fh)
            m["dir"] = os.path.join(ROOT, d)
            out.append(m)
    return out


def _world(seed, width, height):
    """A Rust world for a seed: the generator starts where NumPy's default_rng(seed) starts."""
    from . import _core
    st = np.random.default_rng(seed).bit_generator.state["state"]
    return _core.World(width, height, st["state"], st["inc"])


class Progress:
    """What the progress callback is handed each year: the year, and nothing to change."""

    def __init__(self, year):
        self.year = year


class Session:
    """A world in play. The world itself lives in Rust; this keeps its folder, its chronicle files, the tracked
    characters' journals and the record of everything done to it. Saving writes that record: the seed, every advance
    and every act of the gods, in order. A Rust world is deterministic, so loading replays the record."""

    def __init__(self, path, world, meta):
        self.path, self.world, self.meta = path, world, meta
        self.stop = False
        self.event_log = []
        self.journals = {int(k): [tuple(x) for x in v] for k, v in meta.get("journals", {}).items()}
        self.tracked = set(meta.get("tracked", []))
        self.ev_index = 0
        self._mirror = None
        self.guild_rows = []

    # ---------------- lifecycle
    @classmethod
    def new(cls, name, seed=1, width=480, height=270):
        path = os.path.join(ROOT, slug(name))
        if os.path.exists(path):
            n = 2
            while os.path.exists(f"{path}-{n}"):
                n += 1
            path = f"{path}-{n}"
        return cls.create(path, name, seed, width, height)

    @classmethod
    def create(cls, path, name, seed=1, width=480, height=270):
        """A new world in a folder of your choosing."""
        os.makedirs(os.path.join(path, "maps"), exist_ok=True)
        meta = {"name": name, "seed": seed, "width": width, "height": height, "created": time.strftime("%Y-%m-%d %H:%M"),
                "year": 0, "ended": False, "advances": [], "ops": [], "engine": "rust", "tracked": [], "journals": {}}
        s = cls(path, _world(seed, width, height), meta)
        with open(os.path.join(path, "chronicle.md"), "w") as fh:
            fh.write(f"# The Chronicle of {name} (seed {seed})\n\n")
        open(os.path.join(path, "details.md"), "w").close()
        s.flush()
        s.save()
        return s

    @classmethod
    def load(cls, path):
        with open(os.path.join(path, "meta.json")) as fh:
            meta = json.load(fh)
        if meta.get("engine") != "rust":
            raise RuntimeError("it was grown by the old Python engine, which the game no longer runs; its books and maps are still in its folder")
        s = cls(path, _world(meta["seed"], meta["width"], meta["height"]), meta)
        for op in meta.get("ops", []):                                  # the world's history, replayed exactly
            if op[0] == "step":
                s.world.step(int(op[1]))
            else:
                s.world.god(op[1], op[2])
        events, s.ev_index = s.world.events_from(0)
        s.event_log = [to_log(y, m, t) for y, m, t in events]
        return s

    def save(self):
        self.meta["year"] = self.world.year
        self.meta["tracked"] = sorted(self.tracked)
        self.meta["journals"] = {str(k): v[-2000:] for k, v in self.journals.items()}
        tmp = os.path.join(self.path, "meta.json.tmp")
        with open(tmp, "w") as fh:
            json.dump(self.meta, fh, indent=1)
        os.replace(tmp, os.path.join(self.path, "meta.json"))

    @property
    def realm(self):
        """The world as the screens read it, rebuilt after any change."""
        if self._mirror is None or self._mirror.year != self.world.year or self._dirty:
            self._mirror = Mirror(self.world, self.tracked, self.journals, self.event_log, self.meta["seed"])
            self._mirror._session = self
            self._dirty = False
        return self._mirror

    _dirty = True

    def flush(self):
        """New chronicle entries into the files, the event log and the tracked journals."""
        events, self.ev_index = self.world.events_from(self.ev_index)
        if not events:
            return
        names = {cid: self.world.character(cid)["name"] for cid in self.tracked}
        with open(os.path.join(self.path, "chronicle.md"), "a") as cf, \
                open(os.path.join(self.path, "details.md"), "a") as df:
            for y, minor, text in events:
                (df if minor else cf).write(f"- **Year {y}.** {text}\n")
                row = to_log(y, minor, text)
                self.event_log.append(row)
                for cid, nm in names.items():
                    if nm and nm in text:
                        self.journals.setdefault(cid, []).append((y, row[1], text))
        self._dirty = True

    def record(self, op):
        ops = self.meta.setdefault("ops", [])
        if op[0] == "step" and ops and ops[-1][0] == "step":
            ops[-1] = ["step", ops[-1][1] + op[1]]
        else:
            ops.append(list(op))

    # ---------------- the passing of years
    def advance(self, years, progress=None):
        """Run the world forward; progress(done, total, view) is called every year. Stops early if self.stop."""
        if self.meta.get("ended"):
            raise RuntimeError("this world has ended")
        self.stop = False
        start, t0 = self.world.year, time.time()
        for n in range(int(years)):
            if self.stop:
                break
            self.world.step(1)
            self.record(("step", 1))
            self.flush()
            y = self.world.year
            if y % 10 == 0:
                self.decade()
            if y % 100 == 0:
                from .output import render
                render(self.realm, os.path.join(self.path, "maps", f"year-{y:05d}.png"))
            if progress is not None:
                progress(n + 1, int(years), Progress(y))
        self.meta["advances"].append({"from": start, "to": self.world.year, "seconds": round(time.time() - t0, 1)})
        self.save()
        self.write_books()
        return self.world.year - start

    def decade(self):
        """Every ten years: the hero guilds' tally for the decade, and where each tracked life stands."""
        y = self.world.year
        snap = self.world.hero_guilds()
        prev = getattr(self, "_guild_prev", None) or [(0, 0, [0] * 9)] * len(snap)
        for f, ((hunts, wins, acts), (ph, pw, pa)) in enumerate(zip(snap, prev)):
            self.guild_rows.append({"year": y, "faction": f, "hunts": hunts - ph, "hunt_wins": wins - pw, "_ren": [], "_lev": [],
                                    "_acts": np.array(acts) - np.array(pa)})
        self._guild_prev = snap
        for cid in self.tracked:
            c = self.world.character(cid)
            if not c["alive"]:
                continue
            self.journals.setdefault(cid, []).append(
                (y, "lives", f"Aged {y - c['born']}, level {c['level']}, renown {c['renown']:,.0f}."))

    def god(self, act, args):
        """An act of the gods, done to the world and written into its record."""
        out = self.world.god(act, args)
        self.record(("god", act, args))
        self.flush()
        self._dirty = True
        return out

    def write_books(self):
        from . import output
        r, p = self.realm, self.path
        output.render(r, os.path.join(p, "maps", "latest.png"))
        for fn, name in ((output.write_dynasties, "dynasties.md"), (output.write_legends, "legends.md"),
                         (output.write_magic, "magic.md"), (output.write_bestiary, "bestiary.md"),
                         (output.write_realms, "realms.md"), (output.write_calibration, "calibration.md")):
            fn(r, os.path.join(p, name))
        output.write_summary(r, os.path.join(p, "summary.md"), self.guild_rows)
        output.write_books(r, p)
        self.write_journals()

    def write_journals(self):
        if not self.tracked:
            return
        lines = ["# Tracked characters", ""]
        for cid in sorted(self.tracked):
            c = self.realm.chars[cid]
            lines += [f"## {c.full_name()}" + ("" if c.alive else f" (died {c.died})"), ""]
            lines += [f"- **{y}** [{cat}] {t}" for y, cat, t in self.journals.get(cid, [])]
            lines.append("")
        with open(os.path.join(self.path, "tracked.md"), "w") as fh:
            fh.write("\n".join(lines) + "\n")

    def end(self):
        self.meta["ended"] = True
        self.meta["ended_at"] = self.world.year
        self.god("god_note", {"text": f"**The end of the world.** The chronicle closes in year {self.world.year}."})
        self.save()
        self.write_books()

    # ---------------- what the screens show
    def overview(self):
        r = self.realm
        towns = r.alive_towns()
        return {"year": r.year, "pop": sum(t.pop for t in towns), "towns": len(towns),
                "realms": sum(1 for k in r.kingdoms if not k.extinct), "wars": int(r.war.sum() // 2),
                "people": len(r.living), "heroes": len(r.hero_ids), "monsters": sum(1 for m in r.monsters if m.alive),
                "calamities": sum(1 for m in r.monsters if m.alive and m.kind == "calamity"),
                "tier": r.world_tier, "tracked": len(self.tracked)}

    def realm_rows(self):
        r = self.realm
        rows = []
        for k in sorted((k for k in r.kingdoms if not k.extinct), key=lambda k: -len(r.alive_towns(k.id))):
            towns = r.alive_towns(k.id)
            ru = r.chars.get(k.ruler)
            at_war = [r.kingdoms[o].short for o in range(r.npol) if r.war[k.id, o]]
            rows.append((k.id, k.name, PEOPLES[k.people], k.rank, k.gov, ru.full_name() if ru else "-", len(towns),
                         int(sum(t.pop for t in towns)), int(k.gold), int(k.army_power), ", ".join(at_war[:3]) or "-"))
        return rows

    def find_characters(self, query, limit=60):
        r = self.realm
        q = query.lower().strip()
        pool = [r.chars[i] for i in r.living] if not q.startswith("dead:") else list(r.chars.values())
        q = q.replace("dead:", "")
        out = [c for c in pool if not q or q in c.full_name().lower() or (q.isdigit() and int(q) == c.id)]
        out.sort(key=lambda c: -(c.renown + (5000 if c.id in self.tracked else 0) +
                                 (3000 if c.ruled and c.ruled[2] is None else 0)))
        return out[:limit]
