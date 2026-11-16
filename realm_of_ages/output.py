"""What a run leaves behind: maps, the summary, the royal houses, the legends' character sheets, the magic."""
import math

import numpy as np

from .characters import sheet
from .data import (SINGULAR, COLORS, PEOPLES, RARITY, RULER_TITLE, SCHOOLS, SPELLS, TERRAIN_COLORS, TIER_WORDS, WORLDFORGED)
from .heroes import ACTIONS


def render(realm, path, px=None):
    from PIL import Image, ImageDraw
    H, W = realm.H, realm.W
    px = px or max(2, min(7, 1600 // W))
    img = np.array(TERRAIN_COLORS, float)[realm.terrain]
    mm = np.clip((realm.mana - 0.45) * 2, 0, 1)[..., None]
    img = img * (1 - 0.55 * mm) + np.array([170, 90, 230]) * 0.55 * mm
    if getattr(realm, "corrupted", None) is not None:
        cm = realm.corrupted[..., None]
        img = np.where(cm, img * 0.4 + np.array([90, 0, 60]) * 0.6, img)
    F = realm.owner_fac
    pal = [k.color or COLORS[k.people] for k in realm.kingdoms] + [(0, 0, 0)]
    cols = np.array(pal, float)[F]
    img = np.where((F >= 0)[..., None], img * 0.55 + cols * 0.45, img)
    edge = np.zeros((H, W), bool)
    edge[:, :-1] |= F[:, :-1] != F[:, 1:]
    edge[:-1] |= F[:-1] != F[1:]
    img[edge & (F >= 0)] *= 0.55
    im = Image.fromarray(img.clip(0, 255).astype(np.uint8)).resize((W * px, H * px), Image.NEAREST)
    top = 64
    canvas = Image.new("RGB", (W * px, H * px + top), (18, 18, 24))
    canvas.paste(im, (0, top))
    d = ImageDraw.Draw(canvas)
    cal = realm.calamity_alive()
    d.text((8, 4), f"Realm of Ages - year {realm.year} (seed {realm.seed})   highest tier ever cast: "
                   f"{TIER_WORDS[realm.world_tier]}" + (f"   CALAMITY: {cal[0].name}" if cal else ""),
           fill=(235, 235, 235))
    w4 = W * px // 6
    big = sorted((k for k in realm.kingdoms if not realm.extinct[k.id]), key=lambda k: -len(realm.alive_towns(k.id)))
    for n, k in enumerate(big[:6]):
        towns = realm.alive_towns(k.id)
        r = realm.chars.get(k.ruler)
        col = tuple(k.color or COLORS[k.people])
        d.text((8 + n * w4, 24), f"{k.name}"[:34], fill=col)
        d.text((8 + n * w4, 42), f"{len(towns)} towns, {sum(s.pop for s in towns):,.0f}"
               + (f", {r.name} {r.regnal or ''}" if r is not None else ""), fill=col)
    if len(big) > 6:
        d.text((W * px - 150, 4), f"+{len(big) - 6} more realms", fill=(200, 200, 200))
    for r in realm.ruins:
        y, x = r.pos[0] * px + px // 2 + top, r.pos[1] * px + px // 2
        col = (255, 215, 0) if r.loot else (200, 200, 200)
        d.line([(x - 3, y - 3), (x + 3, y + 3)], fill=col)
        d.line([(x - 3, y + 3), (x + 3, y - 3)], fill=col)
    for L in realm.lands:
        if L["kind"] == "continent":
            y, x = L["center"][0] * px + top, L["center"][1] * px
            d.text((x - 20, y), L["name"].upper(), fill=(250, 250, 250))
    for hv in getattr(realm, "havens", []):
        y, x = hv["pos"][0] * px + px // 2 + top, hv["pos"][1] * px + px // 2
        d.rectangle([x - 4, y - 4, x + 4, y + 4], fill=(0, 0, 0), outline=(255, 255, 255))
    for cl in getattr(realm, "clans", []):
        y, x = cl.pos[0] * px + px // 2 + top, cl.pos[1] * px + px // 2
        d.polygon([(x, y - 5), (x + 4, y + 4), (x - 4, y + 4)], fill=(30, 30, 30), outline=(200, 200, 200))
    for t in getattr(realm, "tribes", {}).values():
        y, x = t.pos[0] * px + px // 2 + top, t.pos[1] * px + px // 2
        col = {"tide": (80, 160, 255), "stone": (160, 120, 60), "flame": (255, 90, 30), "sky": (230, 230, 160)}[t.element]
        d.ellipse([x - 5, y - 5, x + 5, y + 5], outline=col, width=2)
    for s in realm.alive_towns():
        y, x = s.pos[0] * px + px // 2 + top, s.pos[1] * px + px // 2
        rad = 1.5 + math.sqrt(s.pop) / 20
        if s.capital:
            d.rectangle([x - rad - 1, y - rad - 1, x + rad + 1, y + rad + 1], fill=tuple(realm.kingdoms[s.faction].color or COLORS[s.culture]),
                        outline=(255, 255, 255))
            d.text((x + rad + 3, y - 6), s.name, fill=(255, 255, 255))
        else:
            d.ellipse([x - rad, y - rad, x + rad, y + rad], fill=tuple(realm.kingdoms[s.faction].color or COLORS[s.culture]), outline=(0, 0, 0))
    for a in getattr(realm, "armies", []):
        y, x = a.pos[0] * px + px // 2 + top, a.pos[1] * px + px // 2
        k = realm.kingdoms[a.realm] if a.realm is not None else None
        col = tuple(k.color or COLORS[k.people]) if k is not None else (255, 255, 255)
        rad = 3 + min(8, (a.men / 1000) ** 0.5 * 2)
        d.rectangle([x - rad, y - rad / 2, x + rad, y + rad / 2], fill=col, outline=(0, 0, 0))
    for m in realm.monsters:
        if not m.alive:
            continue
        y, x = m.pos[0] * px + px // 2 + top, m.pos[1] * px + px // 2
        if m.kind == "dragon":
            d.polygon([(x, y - 7), (x - 6, y + 5), (x + 6, y + 5)], fill=(230, 30, 20), outline=(0, 0, 0))
        elif m.kind == "wraith":
            d.polygon([(x, y - 5), (x + 5, y), (x, y + 5), (x - 5, y)], fill=(235, 235, 255), outline=(0, 0, 0))
        elif getattr(m, "sea", False):
            d.ellipse([x - 4, y - 4, x + 4, y + 4], fill=(40, 230, 230), outline=(0, 0, 0))
        elif m.kind == "demon":
            d.rectangle([x - 4, y - 4, x + 4, y + 4], fill=(255, 120, 0), outline=(0, 0, 0))
        elif m.kind == "calamity":
            pts = []
            for i in range(10):
                a = math.pi / 2 + i * math.pi / 5
                rr = 14 if i % 2 == 0 else 6
                pts.append((x + rr * math.cos(a), y - rr * math.sin(a)))
            d.polygon(pts, fill=(10, 10, 10), outline=(255, 40, 40))
            d.text((x + 16, y - 6), m.name.split(",")[0], fill=(255, 80, 80))
        else:
            d.ellipse([x - 3, y - 3, x + 3, y + 3], fill=(110, 70, 30), outline=(0, 0, 0))
    canvas.save(path)


def write_summary(realm, path, guild_rows):
    L = [f"# Realm of Ages, year {realm.year}", "", f"Seed {realm.seed}, a {realm.W} by {realm.H} world. "
         f"Highest tier of magic ever cast: **{TIER_WORDS[realm.world_tier]}**.", "", "## The realms", "",
         "| People | Realm | Ruler | House | Law | Legitimacy | Tyranny | Corruption | Towns | Population | Allies |",
         "|---|---|---|---|---|---|---|---|---|---|---|"]
    for k in sorted(realm.kingdoms, key=lambda k: (realm.extinct[k.id], -len(realm.alive_towns(k.id)))):
        f = k.id
        if realm.extinct[f]:
            continue
        towns = realm.alive_towns(f)
        r = realm.chars.get(k.ruler)
        if r is None:
            continue
        ruler = f"{RULER_TITLE[k.people][int(r.female)]} {r.full_name()}, age {r.age(realm.year)}, level {r.level}"
        L.append(f"| {PEOPLES[k.people]} | {k.name} ({k.rank}{', broke from ' + realm.kingdoms[k.parent].name + ' in ' + str(k.founded_year) if k.parent is not None else ''}) | {ruler} | "
                 f"{realm.houses[k.house].name if k.house is not None else '-'} | {k.law} | "
                 f"{(r.legitimacy or 0):.0f} | {r.tyranny:.0f} | {100 * k.corruption:.0f}%"
                 f"{' (' + ', '.join(k.corrupt_seats) + ' corrupt)' if k.corrupt_seats else ''} | {len(towns)} | "
                 f"{sum(s.pop for s in towns):,.0f} | {', '.join(realm.kingdoms[a].short for a in sorted(k.allies)) or '-'} |")
    wars = [f"{realm.kingdoms[i].name} against {realm.kingdoms[j].name} (since {realm.war_since[i, j]})"
            for i in range(realm.npol) for j in range(i + 1, realm.npol) if realm.war[i, j]]
    L += ["", "## Realms born of breakaway", ""]
    L += [f"- Year {y}: {realm.kingdoms[n].name} broke from {realm.kingdoms[o].name} under "
          f"{realm.chars[c].full_name()}" for y, n, o, c in realm.breakaways] or ["- none yet"]
    L += ["", "## Factions now", ""]
    for k in realm.kingdoms:
        for fa in getattr(k, "factions", []):
            L.append(f"- In {k.name}: a {fa['kind']} faction of {len(fa['members'])} dukes, led by "
                     f"{realm.chars[fa['leader']].full_name()}, at {100 * fa['power']:.0f}% of the crown's strength")
    L += ["", f"**Wars now:** {', '.join(wars) if wars else 'none'}.", "", "## Monsters abroad", ""]
    L += [f"- {m.name} ({m.kind}, strength {m.power:,.0f}, since year {m.born}"
          f"{f', hoard {m.hoard:,.0f}' if m.hoard > 1 else ''})" for m in realm.monsters if m.alive] or ["- none"]
    L += ["", "## Calamities", ""]
    L += [f"- {n}: woke in year {a}, ended in year {b}, {how}" for n, a, b, how in realm.calamity_log] or \
        ["- none have ended"]
    L += ["", "## Worldforged relics", ""]
    for name, _, note in WORLDFORGED:
        holder = "never found"
        if name in realm.worldforged_found:
            holder = "lost"
            for c in realm.chars.values():
                if c.alive and any(it.name == name for it in c.items):
                    holder = f"borne by {realm.who(c)}"
            for k in realm.kingdoms:
                if any(it.name == name for it in k.treasury):
                    holder = f"in the treasury of {k.name}"
            for r in realm.ruins:
                if any(it.name == name for it in r.loot):
                    holder = f"lost in {r.name}"
        L.append(f"- **{name}** ({note}): {holder}")
    everyone = [c for c in realm.chars.values() if c.hero or c.renown > 0 or c.ruled]
    top = sorted(everyone, key=lambda h: -h.renown)[:12]
    L += ["", "## Hall of legends (top 12 by renown)", "",
          "| Name | People | Level | Classes | Tier | Renown | Fate |", "|---|---|---|---|---|---|---|"]
    for h in top:
        fate = "living" if h.alive else h.cause
        L.append(f"| {h.full_name()} | {PEOPLES[h.people]} | {h.level} | "
                 f"{', '.join(c for c, lv in h.classes if lv > 0)} | {max(h.tier(), h.peak_tier) if max(h.tier(), h.peak_tier) >= 0 else '-'} | "
                 f"{h.renown:,.0f} | {fate} |")
    L += ["", "## What the guilds learned", "",
          "First century against the last full century. Renown at death is the mean over heroes who died in "
          "that century.", "",
          "| Guild | Century | Hunt win rate | Renown at death | Level at death | Deaths | Deeds (" +
          "/".join(ACTIONS) + ") |", "|---|---|---|---|---|---|---|"]
    for f in range(4):
        rows = [r for r in guild_rows if r["faction"] == f]
        if not rows:
            continue
        last_c = (rows[-1]["year"] - 1) // 100
        for label, sel in (("first", [r for r in rows if r["year"] <= 100]),
                           ("last", [r for r in rows if (r["year"] - 1) // 100 == last_c])):
            if not sel:
                continue
            hunts = sum(r["hunts"] for r in sel)
            wins = sum(r["hunt_wins"] for r in sel)
            ren = [x for r in sel for x in r["_ren"]]
            lev = [x for r in sel for x in r["_lev"]]
            acts = np.sum([r["_acts"] for r in sel], 0)
            mix = "/".join(f"{100 * a / max(1, acts.sum()):.0f}" for a in acts)
            L.append(f"| {PEOPLES[f]} | {label} | {wins}/{hunts} ({100 * wins / max(1, hunts):.0f}%) | "
                     f"{np.mean(ren) if ren else float('nan'):.1f} | {np.mean(lev) if lev else float('nan'):.1f} | "
                     f"{len(ren)} | {mix} |")
    L += ["", "**Guild doctrines** (the skill branches each guild has learned to favour, by mean reward "
              "per hero-year):", ""]
    for f in range(4):
        doc = realm.guilds[f].doctrine()
        L.append(f"- {PEOPLES[f]}: " + (", ".join(f"{n} ({v:.2f})" for n, v in doc) if doc else "not settled yet"))
    with open(path, "w") as fh:
        fh.write("\n".join(L) + "\n")


def write_dynasties(realm, path):
    chars, houses = realm.chars, realm.houses
    L = ["# The royal houses", ""]
    L += ["## Dynasties and their branches", ""]
    roots = {}
    for h in houses.values():
        roots.setdefault(getattr(h, "dynasty", h.id), []).append(h)
    for root, hs_ in sorted(roots.items(), key=lambda kv: -sum(h.reign_years for h in kv[1]))[:12]:
        if sum(h.rulers for h in hs_) == 0:
            continue
        r0 = houses[root]
        L.append(f"- **The dynasty of {r0.name}** ({sum(h.rulers for h in hs_)} rulers, "
                 f"{sum(h.reign_years for h in hs_)} years on thrones, prestige {sum(h.prestige for h in hs_):,.0f})"
                 + (": cadet branches " + ", ".join(h.name for h in hs_ if h.id != root) if len(hs_) > 1 else ""))
    L.append("")
    for k in realm.kingdoms:
        f = k.people
        L += [f"## {k.name}" + (" (no more)" if realm.extinct[k.id] else ""), "",
              f"People: {PEOPLES[f]}. Rank: {k.rank}."
              + (f" Broke from {realm.kingdoms[k.parent].name} in year {k.founded_year}." if k.parent is not None else "")
              + f" Succession law: **{k.law}**. Succession wars: {k.civil_wars}.", "",
              "| # | Ruler | House | Reign | Years | Legitimacy | Tyranny | How it ended |",
              "|---|---|---|---|---|---|---|---|"]
        for i, r in enumerate(k.reigns, 1):
            c = chars[r["id"]]
            end = r["end"] if r["end"] is not None else "..."
            yrs = (r["end"] if r["end"] is not None else realm.year) - r["start"]
            L.append(f"| {i} | {RULER_TITLE[f][int(c.female)]} {c.full_name()} | {houses[c.house].name} | "
                     f"{r['start']}-{end} | {yrs} | {(r.get('legit', c.legitimacy) or 0):.0f} | "
                     f"{r.get('tyranny', c.tyranny):.0f} | {r['how'] or 'reigning'} |")
        ruled = {chars[r["id"]].house for r in k.reigns}
        hs = sorted((houses[h] for h in ruled if h is not None), key=lambda h: -h.reign_years)
        L += ["", "| House | Origin | Legacy | Rulers | Years on the throne | Prestige |", "|---|---|---|---|---|---|"]
        for h in hs:
            from .data import LEGACY, ROMAN
            from .lineage import level_of
            from .data import HOUSE_LEGACY_LEVELS
            lv = level_of(h.legacy, HOUSE_LEGACY_LEVELS)
            L.append(f"| {h.name} | {h.origin} | {LEGACY[h.track][0]} {ROMAN[lv] if lv else '(unranked)'} | "
                     f"{h.rulers} | {h.reign_years} | {h.prestige:,.0f} |")
        if hs:
            L += ["", f"### Family tree of {hs[0].name} (rulers in bold, crowned years in brackets)", ""]
            L += tree(realm, chars[hs[0].founder], k)
        L.append("")
    with open(path, "w") as fh:
        fh.write("\n".join(L) + "\n")


def tree(realm, root, k, max_depth=8):
    chars = realm.chars
    rulers = {r["id"]: r for r in k.reigns}
    memo = {}

    def has_ruler(c):
        if c.id in memo:
            return memo[c.id]
        v = c.id in rulers or any(has_ruler(chars[x]) for x in c.children)
        memo[c.id] = v
        return v
    out = []

    def walk(c, depth):
        if depth > max_depth:
            return
        tag = c.full_name()
        if c.id in rulers:
            r = rulers[c.id]
            tag = f"**{tag}** [{r['start']}-{r['end'] if r['end'] is not None else ''}]"
        life = f"{c.born}-{c.died if not c.alive else ''}"
        sp = chars[c.spouse].full_name() if c.spouse is not None else None
        extra = []
        if c.hero or c.renown >= 100:
            extra.append(f"renown {c.renown:,.0f}")
        rare = sorted(t for t in c.genetic if t in ("dragonblood", "starblood", "genius", "spellborn")
                      or t.startswith("half-"))
        if rare:
            extra.append(", ".join(rare))
        out.append("  " * depth + f"- {tag} ({life})" + (f" m. {sp}" if sp else "")
                   + (f"; {'; '.join(extra)}" if extra else ""))
        kids = sorted((chars[x] for x in c.children), key=lambda x: x.born)
        for kid in kids:
            if has_ruler(kid) or depth < 1:
                walk(kid, depth + 1)
    walk(root, 0)
    return out


def write_legends(realm, path, n=16):
    everyone = [c for c in realm.chars.values() if c.hero or c.renown > 0 or c.ruled]
    top = sorted(everyone, key=lambda h: -(h.renown + h.peak_cr / 50))[:n]
    L = ["# Legends of the realm", "", f"The {n} greatest names of {realm.year} years, with their character sheets.",
         ""]
    for c in top:
        L.append(sheet(c, realm))
    with open(path, "w") as fh:
        fh.write("\n".join(L) + "\n")


def write_magic(realm, path):
    chars = realm.chars
    L = ["# Magic", "", "Tiers run from cantrips to the tenth. Talent caps how high anyone can climb; most never pass "
         "the third. The World Rites lie beyond the tiers.", "", "## The tier records", "",
         "| Year | Caster | Tier | Spell |", "|---|---|---|---|"]
    for y, cid, t, spell in realm.tier_events:
        L.append(f"| {y} | {realm.who(chars[cid])} | {TIER_WORDS[t]} | {spell} |")
    L += ["", "## World Rites spoken", ""]
    L += [f"- Year {y}: {realm.who(chars[cid])}, {rite}" for y, cid, rite in realm.rites] or ["- none"]
    L += ["", "## Power genetics awakened", ""]
    L += [f"- Year {y}: {g} wakes in {realm.who(chars[cid])}" for y, cid, g in realm.awakenings] or ["- none"]
    L += ["", "## Highest tier reached among each people, by century", "", "| Year | " +
          " | ".join(PEOPLES) + " |", "|---|---|---|---|---|"]
    for y, tiers in realm.tier_hist:
        if y % 100 == 0:
            L.append(f"| {y} | " + " | ".join(str(t) for t in tiers) + " |")
    L += ["", "## The spell ladders", ""]
    for s, name in enumerate(SCHOOLS):
        L.append(f"- **{name}:** " + "; ".join(f"{TIER_WORDS[t]}: {SPELLS[s][t]}" for t in range(11)))
    living_casters = sorted((c for c in realm.chars.values() if c.alive and c.tier() >= 0),
                            key=lambda c: -c.tier())[:10]
    L += ["", "## The strongest casters living", ""]
    L += [f"- {realm.who(c)}: {SCHOOLS[c.school()]}, {TIER_WORDS[c.tier()]} tier, talent {c.talent:.1f}"
          for c in living_casters]
    with open(path, "w") as fh:
        fh.write("\n".join(L) + "\n")


def write_bestiary(realm, path):
    from .monsters import SOVEREIGN
    L = ["# Bestiary", "", "Monsters age, breed and die. Hoards, relics and titles pass to their young.", "",
         f"## The line of the {SOVEREIGN}", ""]
    L += [f"- Year {y}: {name}, {how}" for y, name, how in realm.sovereign_line] or ["- never held"]
    L += ["", "## Living monsters", "", "| Name | Kind | Age | Generation | Strength | Traits | Title | Heroes slain | "
          "Hoard | Relics |", "|---|---|---|---|---|---|---|---|---|---|"]
    for m in sorted((m for m in realm.monsters if m.alive), key=lambda m: -m.power):
        L.append(f"| {m.name} | {m.kind} | {realm.year - m.born} | {getattr(m, 'generation', 1)} | {m.power:,.0f} | "
                 f"{', '.join(sorted(getattr(m, 'traits', []))) or '-'} | {getattr(m, 'title', None) or '-'} | "
                 f"{getattr(m, 'kills', 0)} | {m.hoard:,.0f} | "
                 f"{', '.join(i.label() for i in m.items) or '-'} |")
    with open(path, "w") as fh:
        fh.write("\n".join(L) + "\n")


def _w(path, L):
    with open(path, "w") as fh:
        fh.write("\n".join(L) + "\n")


def write_books(realm, out):
    """The deeper books of the world: lands and life, myths, dungeons, seas, pirates, society."""
    import os
    from .data2 import SHIPS, CONCORD
    chars = realm.chars
    # ---- the world and its life
    L = ["# The world and its life", "", "## Lands", "", "| Land | Kind | Leagues | Towns by people | First found by |",
         "|---|---|---|---|---|"]
    for d in sorted(realm.lands, key=lambda d: -d["size"]):
        if d["kind"] == "isle" and d["size"] < 150:
            continue
        held = {}
        for t in realm.alive_towns():
            if t.landmass == d["id"]:
                nm = realm.kingdoms[t.faction].short
                held[nm] = held.get(nm, 0) + 1
        fb = ", ".join(f"{realm.kingdoms[f].short if f >= 0 else 'pirates'} ({y})" for f, y in d["found_by"].items()) or "-"
        L.append(f"| {d['name']} | {d['kind']} | {d['size']:,} | "
                 f"{', '.join(f'{k} {v}' for k, v in held.items()) or 'wild'} | {fb} |")
    eco = realm.eco
    from .ecology import BIOMES
    L += ["", "## The living world", "", f"{sum(1 for s in eco.species if s['extinct'] is None)} species living, "
          f"{sum(1 for s in eco.species if s['extinct'])} extinct.", "",
          "| Species | Kind | Mythic | Population | Peak | Where it thrives | Origin | Extinct |",
          "|---|---|---|---|---|---|---|---|"]
    for i, sp in sorted(enumerate(eco.species), key=lambda t: -t[1]["peak"]):
        tot = float(eco.pop[i].sum())
        where = "-"
        if tot > 0:
            ey, ex = np.unravel_index(int(eco.pop[i].argmax()), eco.pop[i].shape)
            where = eco.region(realm.rng, ey, ex)
        kind = {1: "grazer", 2: "predator", 3: "apex"}[sp["troph"]]
        L.append(f"| {sp['name']} | {kind} | {'yes' if sp['mythic'] else ''} | {tot:,.0f} | {sp['peak']:,.0f} | "
                 f"{where} | {sp['origin']} | {sp['extinct'] or ''} |")
    L += ["", "## What happened to the wild", ""] + [f"- Year {y}: {t}" for y, t in eco.events]
    _w(os.path.join(out, "world.md"), L)
    # ---- myths, curses and legends
    L = ["# Myths, curses and legends", "", "## The curses of return", ""]
    for cy in realm.cycles:
        L += [f"### {cy.villain}, {cy.title}", "",
              f"Bound to {realm.lands[cy.landmass]['name']} since year {cy.born}; it seeks {cy.relic.name}. "
              f"Sworn blade: {cy.blade.name if cy.blade else 'none yet'}. State: **{cy.state}**"
              + (f", next return year {cy.next_return}" if cy.state == "sealed" else "") + ".", ""]
        for n, a in enumerate(cy.ages, 1):
            h = chars.get(a.get("hero"))
            s = chars.get(a.get("seer"))
            L.append(f"- Age {n} (year {a.get('start', '?')} to {a.get('end', '?')}): hero "
                     f"{h.full_name() if h else 'none'}, seer {s.full_name() if s else 'none'}; {a.get('outcome', '')}"
                     + (f"; heroes who fell: {', '.join(a['fallen'])}" if a.get("fallen") else "")
                     + (f"; the land fell at {a['fell'][1]} in year {a['fell'][0]}" if a.get("fell") else ""))
        if cy.state != "sealed" and cy.age:
            h = chars.get(cy.hero)
            s = chars.get(cy.seer)
            L.append(f"- Age {len(cy.ages) + 1} (from year {cy.age.get('start', '?')}, still unfolding): "
                     + ("it rules as " + chars[cy.ruler].full_name() + " from the stolen throne"
                        if cy.state == "triumphant" and cy.ruler in chars else "it walks the land")
                     + f"; marked hero {h.full_name() if h else 'not yet born'}; seer {s.full_name() if s else 'none'}")
        L.append("")
    L += ["## Curses", "", "| Kind | On | Laid by | Since | Lifted |", "|---|---|---|---|---|"]
    for cu in realm.curses:
        on = cu.get("house") or cu.get("item") or (realm.lands[cu["land"]]["name"] if "land" in cu else "") \
            or (realm.cycles[cu["cycle"]].villain if "cycle" in cu else "")
        L.append(f"| {cu['kind']} | {on} | {cu['source']} | {cu['since']} | {cu['lifted'] or 'no'} |")
    L += ["", "## Legendary weapons", ""]
    for it in realm.legend_items:
        bearers = ", ".join(chars[b].full_name() for b in it.bearers if b in chars) or "none yet"
        L.append(f"- **{it.name}** ({it.origin}, year {it.year}): borne by {bearers}")
    L += ["", "## Dragonriders and merged blood", ""]
    L += [f"- Year {y}: {chars[c].full_name()} ({PEOPLES[chars[c].people]}) {how} the dragon {n}"
          for y, c, n, how in realm.dragon_riders]
    L += [f"- Year {y}: {chars[c].full_name()} merged blood with {n} the {k}" for y, c, k, n in realm.merges]
    L += ["", "## The tales", ""] + [f"- **{t}** (year {y}): {x}" for y, t, x in realm.myths]
    _w(os.path.join(out, "myths.md"), L)
    # ---- dungeons
    L = ["# Dungeons", "", "| Dungeon | Land | Kind | Floors | Deepest cleared | Boss | State | Conquests |",
         "|---|---|---|---|---|---|---|---|"]
    for r in sorted(realm.ruins, key=lambda r: -(getattr(r, "floors", 0) or 0)):
        if getattr(r, "floors", None) is None:
            continue
        land = realm.lands[int(realm.landmass[r.pos])]["name"] if realm.landmass[r.pos] >= 0 else "?"
        L.append(f"| {r.name} | {land} | {r.kind}{' (grand)' if r.grand else ''} | {r.floors} | {r.floor} | "
                 f"{r.boss['name']} | {'conquered ' + str(r.conquered) if r.conquered else 'unconquered'} | "
                 f"{len(r.conquests)} |")
    L += ["", "## Conquests", ""] + [f"- Year {y}: {chars[h].full_name()} cleared {n} ({f} floors), killing {b}"
                                    for y, h, n, b, f in realm.dungeon_log]
    _w(os.path.join(out, "dungeons.md"), L)
    # ---- seas
    L = ["# The seas", "", "| Realm | Warships | Merchantmen | Best ship | Lord Admiral | Colonies | Lands known |",
         "|---|---|---|---|---|---|---|"]
    for k in realm.kingdoms:
        adm = chars.get(k.council.get("admiral"))
        L.append(f"| {k.name} | {k.warships:.0f} | {k.merchants:.0f} | {SHIPS[k.ship_tier][0]} | "
                 f"{adm.full_name() if adm else '-'} | {k.colonies} | {len(realm.known[k.id])} |")
    L += ["", "## Discoveries", ""]
    for k in realm.kingdoms:
        for y, lid, by in k.discoveries:
            L.append(f"- Year {y}: {realm.lands[lid]['name']} ({realm.lands[lid]['kind']}), by "
                     f"{chars[by].full_name() if by is not None else 'sailors'} of {k.name}")
    L += ["", f"## Naval battles ({len(realm.naval_battles)})", ""]
    L += [f"- Year {y}: {w} over {l} at {where} ({ws} against {ls})" for y, where, w, l, ws, ls in realm.naval_battles[-40:]]
    _w(os.path.join(out, "seas.md"), L)
    # ---- pirates
    pir = sorted((chars[i] for i in realm.pirates), key=lambda p: -getattr(p, "infamy", 0))
    L = ["# Pirates", "", "## Pirate Kings", ""]
    L += [f"- Year {y}: {chars[i].full_name()}" for y, i in realm.pirate_kings] or ["- none yet"]
    L += ["", "## The most infamous, and their records", ""]
    for p in pir[:15]:
        fate = "living" if p.alive else p.cause
        L += [f"### {p.full_name()} ({SINGULAR[p.people]}), infamy {getattr(p, 'infamy', 0):,.0f}", "",
              f"Ship: {p.ship['name']} ({SHIPS[min(4, p.ship['tier'])][0]}, {p.ship['crew']} hands). Fate: {fate}.", ""]
        L += [f"- {line}" for line in p.resume]
        L.append("")
    _w(os.path.join(out, "pirates.md"), L)
    # ---- society: guilds, faiths, the greatest, shinobi, tribes, the Concord
    L = ["# Society", "", "## Guilds", "", "| Guild | Guildmaster | Members | Gold |", "|---|---|---|---|"]
    for g in realm.guilds_soc:
        m = chars.get(g.master)
        L.append(f"| {g.name} | {m.full_name() if m else '-'} | {len(g.members)} | {g.gold:,.0f} |")
    L += ["", "## Faiths", "", "| Faith | Deity | Founded | By | Followers | Tenets |", "|---|---|---|---|---|---|"]
    for r in sorted(realm.religions, key=lambda r: -getattr(r, "followers", 0)):
        f = chars.get(r.founder)
        L.append(f"| {r.name} | {r.deity} | {r.founded} | {f.full_name() if f else 'of old'} | "
                 f"{getattr(r, 'followers', 0):,.0f} | {', '.join(r.tenets)} |")
    L += ["", "## The greatest of the age", ""]
    L += [f"- The greatest {w}: {chars[i].full_name()}" for w, i in realm.greatest.items()]
    L += ["", "## Crime", ""]
    for k in realm.kingdoms:
        towns = realm.alive_towns(k.realm)
        if towns:
            L.append(f"- {k.name}: mean crime {100 * sum(getattr(t, 'crime', 0) for t in towns) / len(towns):.0f}%")
    L += ["", "## The shinobi clans", ""]
    for cl in realm.clans:
        h = chars.get(cl.head)
        L.append(f"- **{cl.name}**: {len(cl.members)} shinobi, led by {h.full_name() if h else 'no one'}; missions "
                 f"won {cl.missions['won']}, lost {cl.missions['lost']}; pact with {cl.pact}")
    L += ["", "## The bending tribes", ""]
    for t in realm.tribes.values():
        ch = chars.get(t.chief)
        L.append(f"- **{t.name}** ({t.element}): {len(t.members)} benders, chief {ch.full_name() if ch else '-'}")
    L += ["", f"## {CONCORD}", ""]
    for rec in realm.concord_line:
        c = chars[rec["id"]]
        L.append(f"- {c.full_name()} ({rec['element']}), born year {rec['born']} in a time of need ({rec['need']}); "
                 f"revealed {rec['revealed'] or 'never'}; " + ("; ".join(rec["deeds"]) or "no great deeds")
                 + (f"; died year {rec['died'][0]}: {rec['died'][1]}" if rec["died"] else ""))
    _w(os.path.join(out, "society.md"), L)


def write_realms(realm, path):
    """Civilisation and statecraft: eras, governments, laws, armies, resources, courts, wars and decisions."""
    from .civics import ERAS, INNOV, AXES, ideology_label
    from .court import EXPECTED
    from .resources import RESOURCES
    chars = realm.chars
    L = ["# Realms, civilisation and statecraft", "", "## The peoples' learning", "",
         "| People | Era | Innovations | Traditions | Ethos | Trend of the times |", "|---|---|---|---|---|---|"]
    for cu in realm.cultures:
        L.append(f"| {PEOPLES[cu.people]} | {ERAS[cu.era]} | {len(cu.innovations)} | {', '.join(cu.traditions) or '-'} | "
                 f"{cu.ethos} | {ideology_label(cu.trend)} |")
    L += ["", "## Living realms", "",
          "| Realm | People | Rank | Government | Crown | Agenda | Towns | Treasury | Levy | Men-at-arms | "
          "Strategic resources | Vassals / limit | Overlord | Court (grandeur / expected) | Thinks |",
          "|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|"]
    live = sorted((k for k in realm.kingdoms if not realm.extinct[k.id]), key=lambda k: -len(realm.alive_towns(k.id)))
    for k in live:
        strat = [r for r in getattr(k, "resources", {}) if RESOURCES[r][0] == "strategic"]
        maa = ", ".join(f"{n} {m}" for m, n in getattr(k, "maa", {}).items()) or "-"
        ov = realm.kingdoms[k.overlord].name if getattr(k, "overlord", None) is not None else "-"
        ct = getattr(k, "court", None)
        court = f"{ct['grandeur']:.0f} / {EXPECTED.get(k.rank, 30)} ({ct['type']})" if ct else "-"
        thinks = ideology_label(k.sentiment) if getattr(k, "sentiment", None) else "-"
        L.append(f"| {k.name} | {PEOPLES[k.people]} | {k.rank} | {getattr(k, 'gov', '-')} | "
                 f"{getattr(k, 'laws', {}).get('crown', '-')} | {getattr(k, 'agenda', '-')} | "
                 f"{len(realm.alive_towns(k.id))} | {getattr(k, 'gold', 0):,.0f} | {getattr(k, 'levy', 0):,.0f} | {maa} | "
                 f"{', '.join(strat) or '-'} | {getattr(k, 'vassals', 0)} / {getattr(k, 'vassal_limit', 0)} | {ov} | "
                 f"{court} | {thinks} |")
    L += ["", "## The great vassals, and what they think of their rulers", ""]
    from .opinion import opinion as _op, intimidated, hooked
    for k in live[:12]:
        r = chars.get(k.ruler)
        if r is None:
            continue
        dukes = {realm.chars[d["holder"]].id: realm.chars[d["holder"]] for d in realm.duchies.values()
                 if d["ctrl"] == k.id and d["holder"] not in (None, r.id) and realm.chars[d["holder"]].alive}
        if not dukes:
            continue
        L.append(f"**{k.name}**, under {r.full_name()}:")
        for d in sorted(dukes.values(), key=lambda d: d.id not in getattr(k, "powerful", ()))[:6]:
            tot, terms = _op(realm, d, r, breakdown=True)
            top = sorted(terms, key=lambda t: -abs(t[1]))[:5]
            tags = [t for t, on in (("powerful", d.id in getattr(k, "powerful", ())), ("intimidated", intimidated(realm, d, r)),
                                    ("hooked", hooked(realm, r, d))) if on]
            L.append(f"- {d.full_name()}{' (' + ', '.join(tags) + ')' if tags else ''}: {tot:+d}: "
                     + ", ".join(f"{n} {v:+d}" for n, v in top))
        L.append("")
    L += ["", "## Royal courts", ""]
    for k in live:
        ct = getattr(k, "court", None)
        if not ct:
            continue
        pos = ", ".join(f"{p}: {chars[c].full_name()}" for p, c in ct["pos"].items() if c in chars)
        L.append(f"- **{k.name}** ({ct['type']} court since year {ct['since']}): amenities "
                 + ", ".join(f"{a} {v}" for a, v in ct["amen"].items()) + f"; {pos}"
                 + (f"; on display: {', '.join(ct['artifacts'][:5])}" if ct["artifacts"] else "")
                 + (f"; lately {ct['events'][-1][1]}" if ct["events"] else ""))
    L += ["", "## Great decisions and the forming of realms", ""]
    L += [f"- Year {y}: {realm.kingdoms[k].name}: {t}" for y, k, t in realm.decisions] or ["- none"]
    L += ["", "## Wars", "", "| Ended | Attacker | Defender | Casus belli | Years | Outcome |", "|---|---|---|---|---|---|"]
    for w in realm.war_log[-80:]:
        L.append(f"| {w['year']} | {w['attacker']} | {w['defender']} | {w['cb']} | {w['years']} | "
                 f"{w.get('result', w['outcome'])} |")
    L += ["", "## Wars now", ""]
    L += [f"- {realm.kingdoms[w['attacker']].name} against {realm.kingdoms[w['defender']].name} ({w['cb']}, since "
          f"{w['start']}, warscore {w['score']:+.0f})" for w in realm.wars.values()] or ["- none"]
    L += ["", "## Hosts in the field", ""]
    L += [f"- {a.name}: {int(a.men):,} men"
          + (f", besieging (progress {a.siege:.0f}%)" if a.siege > 0 else "") for a in realm.armies] or ["- none"]
    L += ["", "## Great battles", ""]
    L += [f"- Year {y}: the Battle of {w}: {wn} ({wm:,}) against {ln} ({lm:,}); {d:,} fell"
          for y, w, wn, ln, wm, lm, d in sorted(realm.battles_fought, key=lambda b: -b[6])[:20]]
    L += ["", "## Famines, plagues, risings, bankruptcies", ""]
    L += [f"- Famine, year {y}: {n} towns, {d:,.0f} dead" for y, n, d in realm.famines if d > 1000][-15:]
    L += [f"- {e.name}{' (a great pandemic)' if e.great else ''}, {e.start}-{e.end or '...'}: {len(e.towns)} towns, "
          f"{e.deaths:,.0f} dead" for e in realm.epidemics if e.deaths > 2000 or e.great][-15:]
    L += [f"- Rising, year {y}: {realm.by_id[t].name} against {realm.kingdoms[k].name} over {w}"
          for y, t, k, w in realm.risings[-15:]]
    L += [f"- Bankruptcy, year {y}: {realm.kingdoms[k].name}" for y, k in realm.bankruptcies]
    L += ["", "## Great works", ""] + [f"- Year {y}: {n} ({realm.kingdoms[k].name})" for y, k, n in realm.wonders]
    L += ["", "## Culture history", ""]
    for cu in realm.cultures:
        L.append(f"- **{PEOPLES[cu.people]}:** " + "; ".join(f"{y} {t}" for y, t in cu.history[-25:]))
    _w(path, L)


def write_calibration(realm, path):
    """What the world's numbers were set from, and what the simulation actually produced."""
    import math as _m
    chars = list(realm.chars.values())
    human = [c for c in chars if c.people == 0 and c.born > 50 and c.born < realm.year - 80
             and (c.father is not None or c.mother is not None)]            # born in the world, not made as adults
    born = len(human)
    dead_inf = sum(1 for c in human if not c.alive and c.died - c.born < 1)
    dead15 = sum(1 for c in human if not c.alive and c.died - c.born < 15)
    adults = [c for c in human if (c.alive and c.age(realm.year) >= 15) or (not c.alive and c.died - c.born >= 15)]
    ages = [(c.died - c.born) for c in adults if not c.alive and "old age" in (c.cause or "") or
            (not c.alive and c.died - c.born >= 15)]
    mothers = [c for c in human if c.female and c.spouse is not None and not c.alive and c.died - c.born > 44]
    births = [len(c.children) for c in mothers]
    childbed = sum(1 for c in human if c.cause == "died in childbirth")
    towns = realm.alive_towns()
    L = ["# Calibration", "",
         "The rates below were set from commonly cited figures for medieval Europe (approximate, and varying a",
         "great deal by place and century). Humans are calibrated directly; the other peoples are scaled by their",
         "lifespan. The measured column is what this run actually produced, from the human notables born to parents",
         "in the world after year 50 and at least 80 years ago (so their lives are complete or nearly).", "",
         "| Measure | Reference (medieval Europe, approximate) | This run |", "|---|---|---|"]
    if born:
        L.append(f"| Infant deaths (first year) | about 15-30% (lower for elites) | {100 * dead_inf / born:.0f}% of {born} |")
        L.append(f"| Deaths before 15 | about 30-50% | {100 * dead15 / born:.0f}% |")
    if ages:
        L.append(f"| Mean age at death, for those reaching 15 | about 50-65 for elites | {sum(ages) / len(ages):.0f} |")
    if births:
        L.append(f"| Children per married woman (completed) | about 5-7 | {sum(births) / len(births):.1f} |")
    if born:
        L.append(f"| Deaths in childbirth | about 1-1.5% per birth | {childbed} of {born} births |")
    great = [e for e in realm.epidemics if e.great]
    L.append(f"| Great pandemics (Black Death scale) | about one every 150-300 years, 30-60% mortality where it reaches | "
             f"{len(great)} in {realm.year} years; " + ", ".join(f"{e.name} ({e.start}, {e.deaths:,.0f} dead)" for e in great[:4]) + " |")
    L.append(f"| Small epidemics | every few years | {len(realm.epidemics) - len(great)} |")
    L.append(f"| Famines (killing over 1,000) | regional bad harvests about one year in seven; great famines a few per century | "
             f"{len(realm.famines)} |")
    L.append(f"| Peasant risings | dozens per century across Europe in the Late Middle Ages | {len(realm.risings)} |")
    L.append(f"| Bankrupt crowns | several (Edward III, Philip II...) | {len(realm.bankruptcies)} |")
    st = getattr(realm, "econ_stats", {})
    ks = [k for k in realm.kingdoms if not realm.extinct[k.id] and getattr(k, "econ", None)]
    if ks:
        pop = sum(sum(t.pop for t in realm.alive_towns(k.id)) for k in ks) or 1
        pov = sum(k.econ["poverty"] * sum(t.pop for t in realm.alive_towns(k.id)) for k in ks) / pop
        cls = {c: sum(k.econ["classes"][c]["people"] for k in ks) / pop for c in ("lower", "middle", "upper")}
        top = sum(k.econ["classes"]["upper"]["wealth_share"] * sum(t.pop for t in realm.alive_towns(k.id))
                  for k in ks) / pop
        towns = [t for t in realm.alive_towns() if getattr(t, "jobs", None)]
        farm = sum(t.jobs["farm"] for t in towns) / max(1.0, sum(sum(t.jobs.values()) for t in towns))
        L.append(f"| Forced loans (seizing merchants' gold) | common in crises (Edward III's wool, Philip IV) | "
                 f"{len(st.get('forced', []))} |")
        L.append(f"| Share of workers on the land | about 75-90% | {farm:.0%} |")
        L.append(f"| Poor (below subsistence) | about 30-50% near or below the line in hard times | {pov:.0%} now |")
        L.append(f"| Classes (lower / middle / upper) | about 80 / 15 / 5 | {cls['lower']:.0%} / {cls['middle']:.0%} / "
                 f"{cls['upper']:.0%} |")
        L.append(f"| Wealth held by the upper class | the richest 5-10% held about 50-70% | {top:.0%} |")
    L += ["", "## The parameters", "",
          "- Crude birth and death rates (humans): 40 and 35 per thousand a year in a fed town, higher births on open land.",
          "- Child mortality: 16% in the first year, 3% a year to five, 0.8% a year to fifteen.",
          "- Adult mortality: 0.4% a year plus 0.25% growing about 8.5% a year past thirty (doubling every eight years).",
          "- Marital fertility: 32% a year for a married woman, halving in her last fertile years and falling with each child.",
          "- Childbirth: 1.2% of births kill the mother.",
          "- Harvests: a smooth regional field with 13% spread; a year without a summer about one year in thirty somewhere.",
          "- Famine: half of any food shortfall left after the granary dies; the hungry migrate.",
          "- Plague: small outbreaks about one year in forty, great pandemics at most one in 120 years (35-60% fatality).",
          "- Technology: four eras (Tribal to Late Medieval), each taking two to four centuries, like 500-1500 in Europe.",
          "", f"Population now: {sum(t.pop for t in towns):,.0f} in {len(towns)} towns; {realm.migrants:,.0f} have migrated "
          f"from hunger and war since the beginning."]
    _w(path, L)
