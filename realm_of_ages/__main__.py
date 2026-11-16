"""Realm of Ages: a persistent fantasy world of peoples, royal houses, tiered magic, skill trees and heroes.

    python3 -m realm_of_ages --years 1000              # run (resumes from the checkpoint if present)
    python3 -m realm_of_ages --years 1000 --fresh      # grow a new world
Outputs in --dir: chronicle.md, summary.md, dynasties.md, legends.md, magic.md, log.csv, guilds.csv, maps/.
"""
import argparse
import csv
import os
import pickle
import signal
import time

import numpy as np

from .data import PEOPLES
from .heroes import ACTIONS
from .output import render, write_bestiary, write_dynasties, write_legends, write_magic, write_summary
from .world import Realm


def save(path, realm, guild_rows):
    with open(path + ".tmp", "wb") as fh:
        pickle.dump((realm, guild_rows), fh)
    os.replace(path + ".tmp", path)


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--years", type=int, default=1000)
    ap.add_argument("--width", type=int, default=480)
    ap.add_argument("--height", type=int, default=270)
    ap.add_argument("--seed", type=int, default=1)
    ap.add_argument("--dir", default="realm_run")
    ap.add_argument("--fresh", action="store_true")
    ap.add_argument("--map-every", type=int, default=100)
    args = ap.parse_args()
    os.makedirs(os.path.join(args.dir, "maps"), exist_ok=True)
    ckpt = os.path.join(args.dir, "checkpoint.pkl")
    paths = {k: os.path.join(args.dir, k) for k in ("log.csv", "guilds.csv", "chronicle.md")}

    if os.path.exists(ckpt) and not args.fresh:
        with open(ckpt, "rb") as fh:
            realm, guild_rows = pickle.load(fh)
        from . import characters
        characters.CTX["realm"] = realm
        print(f"resumed at year {realm.year}")
    else:
        for p in paths.values():
            if os.path.exists(p):
                os.remove(p)
        realm, guild_rows = Realm(args.width, args.height, args.seed), []
        render(realm, os.path.join(args.dir, "maps", "year-0000.png"))

    stop = {"now": False}
    signal.signal(signal.SIGINT, lambda *_: stop.update(now=True))
    new = not os.path.exists(paths["log.csv"])
    lf = open(paths["log.csv"], "a", newline="")
    gf = open(paths["guilds.csv"], "a", newline="")
    cf = open(paths["chronicle.md"], "a")
    lw, gw = csv.writer(lf), csv.writer(gf)
    if new:
        lw.writerow(["year"] + [f"pop_{p}" for p in PEOPLES] + [f"towns_{p}" for p in PEOPLES]
                    + [f"tier_{p}" for p in PEOPLES]
                    + ["wars", "battles", "captures", "towns_destroyed", "raids", "monsters", "calamity", "slain",
                       "heroes", "living_people", "ruins"])
        gw.writerow(["year", "people", "heroes", "mean_level", "max_level", "hunts", "hunt_wins", "slain", "deaths",
                     "mean_renown_at_death", "eps", "states"] + [f"act_{a}" for a in ACTIONS])
        cf.write(f"# The Chronicle of the Realm (seed {realm.seed})\n\n")

    df = open(os.path.join(args.dir, "details.md"), "a")

    def flush_chronicle():
        for y, text, minor in realm.chron:
            (df if minor else cf).write(f"- **Year {y}.** {text}\n")
        realm.chron.clear()

    flush_chronicle()
    t0, y0, end = time.time(), realm.year, realm.year + args.years
    while realm.year < end and not stop["now"]:
        realm.step()
        flush_chronicle()
        if realm.year % 10 == 0:
            towns = [[t for t in realm.alive_towns() if t.culture == f] for f in range(4)]
            w = realm.win
            lw.writerow([realm.year] + [round(sum(s.pop for s in t)) for t in towns] + [len(t) for t in towns]
                        + list(realm.people_tier)
                        + [int(realm.war.sum() // 2), w["battles"], w["captures"], w["destroyed"], w["raids"],
                           len(realm.monsters), int(bool(realm.calamity_alive())), w["slain"],
                           len(realm.hero_ids), len(realm.living), len(realm.ruins)])
            realm.win = realm.fresh_window()
            for f in range(4):
                g = realm.guilds[f]
                hs = [h for h in realm.heroes() if h.people == f]
                gw.writerow([realm.year, PEOPLES[f], len(hs),
                             round(float(np.mean([h.level for h in hs])), 1) if hs else "",
                             max((h.level for h in hs), default=""),
                             g.w["hunts"], g.w["hunt_wins"], g.w["slain"], g.w["deaths"],
                             round(float(np.mean(g.w["renown_at_death"])), 1) if g.w["renown_at_death"] else "",
                             round(g.eps, 3), len(g.q)] + g.w["acts"].tolist())
                guild_rows.append({"year": realm.year, "faction": f, "hunts": g.w["hunts"],
                                   "hunt_wins": g.w["hunt_wins"], "_ren": list(g.w["renown_at_death"]),
                                   "_lev": list(g.w["level_at_death"]), "_acts": g.w["acts"].copy()})
                g.reset_window()
            lf.flush()
            gf.flush()
            cf.flush()
        if realm.year % args.map_every == 0:
            render(realm, os.path.join(args.dir, "maps", f"year-{realm.year:04d}.png"))
        if realm.year % 100 == 0:
            pops = " ".join(f"{PEOPLES[f][0]}={sum(s.pop for s in realm.alive_towns() if s.culture == f):,.0f}"
                            f"/{sum(1 for s in realm.alive_towns() if s.culture == f)}" for f in range(4))
            pops += f"  realms={sum(1 for k in realm.kingdoms if not realm.extinct[k.id])}"
            print(f"year {realm.year:5d}  {pops}  wars={int(realm.war.sum() // 2)}  monsters={len(realm.monsters)}"
                  f"  heroes={len(realm.hero_ids)}  people={len(realm.living)}  tier={realm.world_tier}"
                  f"  ({(realm.year - y0) / max(1e-9, time.time() - t0):.0f} yr/s)", flush=True)
        if realm.year % 250 == 0:
            save(ckpt, realm, guild_rows)
    save(ckpt, realm, guild_rows)
    render(realm, os.path.join(args.dir, "maps", f"year-{realm.year:04d}.png"))
    write_summary(realm, os.path.join(args.dir, "summary.md"), guild_rows)
    write_dynasties(realm, os.path.join(args.dir, "dynasties.md"))
    write_legends(realm, os.path.join(args.dir, "legends.md"))
    write_magic(realm, os.path.join(args.dir, "magic.md"))
    write_bestiary(realm, os.path.join(args.dir, "bestiary.md"))
    from .output import write_books
    write_books(realm, args.dir)
    from .output import write_realms
    write_realms(realm, os.path.join(args.dir, "realms.md"))
    from .output import write_calibration
    write_calibration(realm, os.path.join(args.dir, "calibration.md"))
    for fh in (lf, gf, cf):
        fh.close()
    print(f"stopped at year {realm.year}; outputs in {args.dir}")


if __name__ == "__main__":
    main()
