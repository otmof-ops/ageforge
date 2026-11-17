"""Realm of Ages: a persistent fantasy world of peoples, royal houses, tiered magic, skill trees and heroes.

    python3 -m realm_of_ages --years 1000              # run (resumes the world in --dir if there is one)
    python3 -m realm_of_ages --years 1000 --fresh      # grow a new world
Outputs in --dir: chronicle.md, details.md, summary.md, the books, log.csv, meta.json, maps/.
The world runs in Rust; the game itself is `bin/realm-tui`.
"""
import argparse
import csv
import os
import shutil
import signal
import time

from .session import Session


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--years", type=int, default=1000)
    ap.add_argument("--width", type=int, default=480)
    ap.add_argument("--height", type=int, default=270)
    ap.add_argument("--seed", type=int, default=1)
    ap.add_argument("--dir", default="realm_run")
    ap.add_argument("--fresh", action="store_true")
    args = ap.parse_args()

    if os.path.exists(os.path.join(args.dir, "meta.json")) and not args.fresh:
        s = Session.load(args.dir)
        print(f"resumed at year {s.world.year}")
    else:
        shutil.rmtree(args.dir, ignore_errors=True)
        s = Session.create(args.dir, os.path.basename(os.path.abspath(args.dir)), args.seed, args.width, args.height)

    signal.signal(signal.SIGINT, lambda *_: setattr(s, "stop", True))
    log = os.path.join(args.dir, "log.csv")
    new = not os.path.exists(log)
    with open(log, "a", newline="") as fh:
        w = csv.writer(fh)
        if new:
            w.writerow(["year", "towns", "population", "mean_poverty", "famine_years", "notables", "houses", "ruins"])
        t0, y0 = time.time(), s.world.year

        def progress(done, total, at):
            y = at.year
            if y % 10 == 0:
                towns, pop, pov, famine, _, _ = s.world.totals()
                w.writerow([y, towns, round(pop), round(pov, 4), famine, s.world.n_characters(), s.world.n_houses(),
                            s.world.n_ruins()])
            if y % 100 == 0:
                o = s.overview()
                print(f"year {y:5d}  pop={o['pop']:,.0f} in {o['towns']} towns  realms={o['realms']}  wars={o['wars']}"
                      f"  monsters={o['monsters']}  heroes={o['heroes']}  people={o['people']}  tier={o['tier']}"
                      f"  ({(y - y0) / max(1e-9, time.time() - t0):.0f} yr/s)", flush=True)

        s.advance(args.years, progress)
    print(f"stopped at year {s.world.year}; outputs in {args.dir}")


if __name__ == "__main__":
    main()
