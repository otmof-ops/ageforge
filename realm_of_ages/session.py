"""Game mode: a world you start, advance a hundred to a thousand years at a time, and end only when you choose.

Each world lives in its own folder under worlds/: the saved state (world.pkl), a small meta.json, the chronicle
and details logs (appended as the years pass), and, after every advance, a fresh set of books and a map. A world
can be closed and opened again later; ending it writes the last books and marks it finished."""
import json
import os
import pickle
import re
import time

import numpy as np

from . import characters
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


class Session:
    def __init__(self, path, realm, guild_rows, meta):
        self.path, self.realm, self.guild_rows, self.meta = path, realm, guild_rows, meta
        self.stop = False
        characters.CTX["realm"] = realm

    # ---------------- lifecycle
    @classmethod
    def new(cls, name, seed=1, width=480, height=270):
        from .world import Realm
        path = os.path.join(ROOT, slug(name))
        if os.path.exists(path):
            n = 2
            while os.path.exists(f"{path}-{n}"):
                n += 1
            path = f"{path}-{n}"
        os.makedirs(os.path.join(path, "maps"), exist_ok=True)
        realm = Realm(width, height, seed)
        meta = {"name": name, "seed": seed, "width": width, "height": height, "created": time.strftime("%Y-%m-%d %H:%M"),
                "year": 0, "ended": False, "advances": []}
        s = cls(path, realm, [], meta)
        with open(os.path.join(path, "chronicle.md"), "w") as fh:
            fh.write(f"# The Chronicle of {name} (seed {seed})\n\n")
        open(os.path.join(path, "details.md"), "w").close()
        s.flush()
        s.save()
        return s

    @classmethod
    def load(cls, path):
        with open(os.path.join(path, "world.pkl"), "rb") as fh:
            realm, guild_rows = pickle.load(fh)
        with open(os.path.join(path, "meta.json")) as fh:
            meta = json.load(fh)
        return cls(path, realm, guild_rows, meta)

    def save(self):
        tmp = os.path.join(self.path, "world.pkl.tmp")
        with open(tmp, "wb") as fh:
            pickle.dump((self.realm, self.guild_rows), fh)
        os.replace(tmp, os.path.join(self.path, "world.pkl"))
        self.meta["year"] = self.realm.year
        with open(os.path.join(self.path, "meta.json"), "w") as fh:
            json.dump(self.meta, fh, indent=1)

    def flush(self):
        r = self.realm
        if not r.chron:
            return
        with open(os.path.join(self.path, "chronicle.md"), "a") as cf, \
                open(os.path.join(self.path, "details.md"), "a") as df:
            for y, text, minor in r.chron:
                (df if minor else cf).write(f"- **Year {y}.** {text}\n")
        r.chron.clear()

    # ---------------- the passing of years
    def advance(self, years, progress=None):
        """Run the world forward; progress(done, total, realm) is called every year. Stops early if self.stop."""
        r = self.realm
        if self.meta.get("ended"):
            raise RuntimeError("this world has ended")
        self.stop = False
        start, t0 = r.year, time.time()
        for n in range(int(years)):
            if self.stop:
                break
            r.step()
            self.flush()
            if r.year % 10 == 0:
                self.decade()
            if r.year % 100 == 0:
                from .output import render
                render(r, os.path.join(self.path, "maps", f"year-{r.year:05d}.png"))
            if progress is not None:
                progress(n + 1, int(years), r)
        self.meta["advances"].append({"from": start, "to": r.year, "seconds": round(time.time() - t0, 1)})
        self.save()
        self.write_books()
        return r.year - start

    def decade(self):
        r = self.realm
        for f in range(4):
            g = r.guilds[f]
            self.guild_rows.append({"year": r.year, "faction": f, "hunts": g.w["hunts"], "hunt_wins": g.w["hunt_wins"],
                                    "_ren": list(g.w["renown_at_death"]), "_lev": list(g.w["level_at_death"]),
                                    "_acts": g.w["acts"].copy()})
            g.reset_window()
        r.win = r.fresh_window()

    def write_books(self):
        from . import output
        r, p = self.realm, self.path
        output.render(r, os.path.join(p, "maps", "latest.png"))
        output.write_summary(r, os.path.join(p, "summary.md"), self.guild_rows)
        output.write_dynasties(r, os.path.join(p, "dynasties.md"))
        output.write_legends(r, os.path.join(p, "legends.md"))
        output.write_magic(r, os.path.join(p, "magic.md"))
        output.write_bestiary(r, os.path.join(p, "bestiary.md"))
        output.write_books(r, p)
        output.write_realms(r, os.path.join(p, "realms.md"))
        output.write_calibration(r, os.path.join(p, "calibration.md"))
        self.write_journals()

    def write_journals(self):
        r = self.realm
        tracked = [r.chars[i] for i in getattr(r, "tracked", ()) if i in r.chars]
        if not tracked:
            return
        lines = ["# Tracked characters", ""]
        for c in tracked:
            lines += [f"## {c.full_name()}" + ("" if c.alive else f" (died {c.died})"), ""]
            lines += [f"- **{y}** [{cat}] {t}" for y, cat, t in getattr(c, "journal", [])]
            lines.append("")
        with open(os.path.join(self.path, "tracked.md"), "w") as fh:
            fh.write("\n".join(lines) + "\n")

    def end(self):
        self.meta["ended"] = True
        self.meta["ended_at"] = self.realm.year
        self.realm.say(f"**The end of the world.** The chronicle closes in year {self.realm.year}.", cat="god")
        self.flush()
        self.save()
        self.write_books()

    # ---------------- what the screens show
    def overview(self):
        r = self.realm
        towns = r.alive_towns()
        realms = [k for k in r.kingdoms if not r.extinct[k.id]]
        return {"year": r.year, "pop": sum(t.pop for t in towns), "towns": len(towns), "realms": len(realms),
                "wars": int(r.war.sum() // 2), "people": len(r.living), "heroes": len(r.hero_ids),
                "monsters": sum(1 for m in r.monsters if m.alive),
                "calamities": sum(1 for m in r.monsters if m.alive and m.kind == "calamity"),
                "tier": r.world_tier, "tracked": len(getattr(r, "tracked", ()))}

    def realm_rows(self):
        r = self.realm
        rows = []
        for k in sorted((k for k in r.kingdoms if not r.extinct[k.id]), key=lambda k: -len(r.alive_towns(k.id))):
            towns = r.alive_towns(k.id)
            ru = r.chars.get(k.ruler)
            at_war = [r.kingdoms[o].short for o in range(r.npol) if r.war[k.id, o]]
            rows.append((k.id, k.name, PEOPLES[k.people], k.rank, getattr(k, "gov", "-"),
                         ru.full_name() if ru else "-", len(towns), int(sum(t.pop for t in towns)),
                         int(getattr(k, "gold", 0)), int(getattr(k, "army_power", 0)), ", ".join(at_war[:3]) or "-"))
        return rows

    def find_characters(self, query, limit=60):
        r = self.realm
        q = query.lower().strip()
        out = []
        pool = [r.chars[i] for i in r.living] if not q.startswith("dead:") else list(r.chars.values())
        q = q.replace("dead:", "")
        for c in pool:
            if not q or q in c.full_name().lower() or (q.isdigit() and int(q) == c.id):
                out.append(c)
        out.sort(key=lambda c: -(c.renown + (5000 if getattr(c, "tracked", False) else 0) +
                                 (3000 if c.ruled and c.ruled[2] is None else 0)))
        return out[:limit]
