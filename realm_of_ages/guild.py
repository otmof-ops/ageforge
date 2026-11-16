"""The Wardens' Hall: the guild of adventurers, its seals, its parties and its quest board.

SEALS. Every hero holds a seal, from the Ash Seal to the Sun Seal, earned by merit (quests done, weighted by their
rank) and by level. A Sun Seal is a living legend; there are seldom more than a handful in the world.

PARTIES. Heroes band together, three to six of a like seal on the same land, under a party name; the loners (the
shy and the arrogant) more often work alone. A party's seal is its members' middle seal. Its strength is the sum of
its members, with more for a balanced party (blade, shield, bow, spell and prayer), and nothing from a member too
weak to matter against what they face.

THE QUEST BOARD. Quests are posted from what the world needs, and ranked by their danger:
- monster slaying, from the towns' purses (and dragon raids, Star Seal and above, four strong at the least);
- outlaw hunts, from the towns' rewards on their most wanted;
- dungeon delves into the ruins, for what lies at the bottom;
- treasure hunts, for a named relic lost in a ruin;
- exploration of the wild, paid by a crown, which may find a deposit or a forgotten ruin;
- caravan escorts through dangerous country, paid by the merchants;
- wolf culls, for the Ash and Oak seals, paid by the villages.
Everyone stays in their lane: a party takes nothing above its seal, and nothing more than two seals below it.

THE FIGHT. A monster quest is fought in rounds; a dragon fights in its lair, where it is strongest, and takes wing
when the fight turns unless it is too proud or too pinned to fly. Driving a dragon off earns merit; slaying one is
the deed of an age. Every quest can wound, and the dangerous ones can kill."""
import math

from .data import MONSTER

SEALS = ["Ash Seal", "Oak Seal", "Iron Seal", "Silver Seal", "Gold Seal", "Star Seal", "Sun Seal"]
SEAL_LEVEL = [0, 6, 12, 18, 25, 33, 42]
SEAL_MERIT = [0, 20, 60, 150, 350, 800, 1800]
RANK_DANGER = [0, 120, 300, 700, 1500, 3500, 7000]
PARTY_A = ["Crimson", "Silver", "Iron", "Ashen", "Golden", "Black", "Wandering", "Seventh", "Last", "Laughing",
           "Broken", "Bright", "Grey", "Storm", "Thorn", "Lantern", "Oaken", "Hollow", "Bitter", "Lucky"]
PARTY_B = ["Lanterns", "Blades", "Wolves", "Ravens", "Oath", "Company", "Shields", "Hounds", "Spears", "Crows",
           "Sworn", "Wardens", "Lions", "Kestrels", "Vow", "Brothers", "Sisters", "Hand", "Banner", "Road"]
ROLE = {"Warrior": "blade", "Knight": "shield", "Ranger": "bow", "Rogue": "blade", "Wizard": "spell",
        "Necromancer": "spell", "Runesmith": "spell", "Priest": "prayer", "Druid": "prayer", "Shaman": "prayer"}


def dist(a, b):
    return math.hypot(a[0] - b[0], a[1] - b[1])


def rank_of(danger):
    return max(i for i, d in enumerate(RANK_DANGER) if danger >= d)


def hall(realm):
    g = getattr(realm, "guild", None)
    if g is None:
        g = realm.guild = {"parties": {}, "quests": {}, "next_q": 0, "next_p": 0, "log": []}
    return g


def home_of(realm, h):
    return realm.by_id.get(h.home)


def free(realm, h):
    return h.alive and h.hero and h.hp >= 40 and not (h.ruled and h.ruled[2] is None) and home_of(realm, h) is not None


# ---------------------------------------------------------------- the year
def guild_year(realm):
    g = hall(realm)
    seals(realm)
    parties(realm, g)
    post(realm, g)
    work(realm, g)
    for qid in [q for q, v in g["quests"].items() if v["expires"] < realm.year]:
        del g["quests"][qid]
    if len(g["log"]) > 3000:
        del g["log"][:1000]


def seals(realm):
    for h in realm.heroes():
        if not h.alive:
            continue
        merit = getattr(h, "merit", 0.0) + h.renown * 0.02 * (getattr(h, "seal", None) is None)
        h.merit = merit
        s = max(i for i in range(len(SEALS)) if h.level >= SEAL_LEVEL[i] and merit >= SEAL_MERIT[i])
        if getattr(h, "seal", -1) < s:
            if s >= 5:
                peers = sum(1 for o in realm.heroes() if o.alive and getattr(o, "seal", 0) >= s)
                realm.say(f"**The Wardens' Hall raises {realm.who(h)} to the {SEALS[s]}**; there are {peers + 1} "
                          f"of that seal in the world.")
            elif s >= 3:
                realm.say(f"The Wardens' Hall raises {realm.who(h)} to the {SEALS[s]}.", minor=True)
            h.seal = s


# ---------------------------------------------------------------- parties
def party_of(g, h):
    pid = getattr(h, "party", None)
    return g["parties"].get(pid) if pid is not None else None


def parties(realm, g):
    rng = realm.rng
    for pid, p in list(g["parties"].items()):
        p["members"] = [i for i in p["members"] if i in realm.chars and realm.chars[i].alive
                        and realm.chars[i].hero and not (realm.chars[i].ruled and realm.chars[i].ruled[2] is None)]
        if len(p["members"]) < 2:
            for i in p["members"]:
                realm.chars[i].party = None
            realm.say(f"{p['name']} is no more; too few of them are left.", minor=p["rank"] < 4)
            del g["parties"][pid]
            continue
        ranks = sorted(getattr(realm.chars[i], "seal", 0) for i in p["members"])
        p["rank"] = ranks[len(ranks) // 2]
    loners = [h for h in realm.heroes() if free(realm, h) and party_of(g, h) is None]
    rng.shuffle(loners)
    for h in loners:
        if party_of(g, h) is not None or rng.random() > 0.35:
            continue
        if ("shy" in h.personality or "arrogant" in h.personality) and rng.random() < 0.6:
            continue
        land = home_of(realm, h).landmass
        seal = getattr(h, "seal", 0)
        fits = [p for p in g["parties"].values() if p["landmass"] == land and len(p["members"]) < 6
                and abs(p["rank"] - seal) <= 1]
        if fits:
            p = max(fits, key=lambda p: -abs(p["rank"] - seal))
            p["members"].append(h.id)
            h.party = p["id"]
            continue
        mates = [o for o in loners if o is not h and party_of(g, o) is None and free(realm, o)
                 and home_of(realm, o).landmass == land and abs(getattr(o, "seal", 0) - seal) <= 1]
        if len(mates) < 2:
            continue
        crew = [h] + mates[:int(rng.integers(2, 6))]
        pid = g["next_p"]
        g["next_p"] += 1
        name = f"the {rng.choice(PARTY_A)} {rng.choice(PARTY_B)}"
        g["parties"][pid] = {"id": pid, "name": name, "members": [c.id for c in crew], "rank": seal,
                             "landmass": land, "formed": realm.year, "done": 0, "lost": 0, "slain": []}
        for c in crew:
            c.party = pid
        realm.say(f"A new party, {name}: {', '.join(realm.who(c) for c in crew)}.", minor=seal < 4)


def strength(realm, members, danger):
    """What a band brings against a danger: the sum of those who matter, more for a balanced band."""
    total = 0.0
    roles = set()
    for h in members:
        cr = h.cr() * h.mods()["vs_monster"]
        total += cr if cr >= 0.1 * danger else cr * 0.05                 # too weak to scratch it
        cls = h.classes[0][0] if h.classes else None
        if cls in ROLE:
            roles.add(ROLE[cls])
    return total * (1 + 0.08 * len(roles))


# ---------------------------------------------------------------- the board
def add(realm, g, kind, title, danger, reward, land, pos, target=None, poster=None, years=6):
    if any(q["kind"] == kind and q["target"] == target and target is not None for q in g["quests"].values()):
        return
    qid = g["next_q"]
    g["next_q"] += 1
    g["quests"][qid] = {"id": qid, "kind": kind, "title": title, "danger": float(danger), "rank": rank_of(danger),
                        "reward": float(reward), "landmass": land, "pos": pos, "target": target, "poster": poster,
                        "posted": realm.year, "expires": realm.year + years, "taken": None}


def post(realm, g):
    rng = realm.rng
    from .dragons import hidden, stage
    # monsters with purses on them, and grown dragons
    for m in realm.monsters:
        if not m.alive or getattr(m, "sea", False) or m.kind == "calamity" or hidden(m, realm.year):
            continue
        purse = sum(getattr(m, "bounties", {}).values())
        if m.kind == "dragon":
            st = stage(m, realm.year)
            if purse <= 0 and getattr(m, "temper", "") == "reclusive":
                continue
            add(realm, g, "dragon", f"Bring down {m.name}, the {st} wyrm", m.base, purse + 40, m.landmass, m.pos,
                target=("m", m.mid), years=12)
        elif purse > 0:
            add(realm, g, "slay", f"Slay {m.name}", m.power, purse, m.landmass, m.pos, target=("m", m.mid))
    # outlaws with towns' rewards on their heads
    for i in realm.living:
        c = realm.chars[i]
        if getattr(c, "posted", 0) > 0 and getattr(c, "jailed_until", 0) <= realm.year:
            t = realm.by_id.get(c.home)
            if t is not None:
                add(realm, g, "outlaw", f"Bring in {c.full_name()}, wanted in {t.name}", 3 * c.cr() + 60, c.bounty,
                    t.landmass, t.pos, target=("c", c.id))
    # ruins: delves and treasure
    from .dungeons import ensure
    for r in realm.ruins:
        if getattr(r, "conquered", None) or rng.random() > 0.05:
            continue
        ensure(realm, r)
        land = int(realm.landmass[r.pos])
        loot = [it for it in getattr(r, "loot", []) if it.rarity >= 3]
        if loot and rng.random() < 0.5:
            it = max(loot, key=lambda it: it.rarity)
            add(realm, g, "treasure", f"Recover {it.label()} from {r.name}", r.boss["power"] * 0.8,
                10 + 10 * it.rarity, land, r.pos, target=("r", r.name), years=10)
        else:
            add(realm, g, "delve", f"Delve {r.name} to the bottom", r.boss["power"], 4 * r.riches, land, r.pos,
                target=("r", r.name), years=10)
    for k in realm.kingdoms:
        if realm.extinct[k.id] or k.ruler is None or rng.random() > 0.15:
            continue
        towns = realm.alive_towns(k.id)
        if not towns:
            continue
        revenue = max(1.0, getattr(k, "income", 0.0) * getattr(k, "tax_now", 0.3))
        # exploration of the wild beyond the realm's edge
        t = towns[int(rng.integers(len(towns)))]
        add(realm, g, "explore", f"Chart the wild country beyond {t.name} for {k.name}",
            60 + 600 * getattr(t, "threat", 0.0), max(2.0, 0.3 * revenue), t.landmass, t.pos,
            target=("x", t.id), poster=("k", k.id), years=5)
        # escorts along trade roads through dangerous country
        traders = [x for x in towns if getattr(x, "trade", 0) > 0 and getattr(x, "threat", 0) > 0.15]
        if len(traders) >= 1 and len(towns) >= 2:
            a = traders[int(rng.integers(len(traders)))]
            b = min((x for x in towns if x is not a and x.landmass == a.landmass),
                    key=lambda x: dist(x.pos, a.pos), default=None)
            if b is not None:
                add(realm, g, "escort", f"Escort the merchants of {a.name} to {b.name}",
                    100 + 900 * (a.threat + getattr(b, "threat", 0)), 0.05 * a.wealth, a.landmass, a.pos,
                    target=("e", a.id), poster=("t", a.id), years=3)
        # wolf culls for the youngest seals
        v = towns[int(rng.integers(len(towns)))]
        add(realm, g, "cull", f"Cull the wolves preying on the flocks of {v.name}", rng.uniform(30, 110),
            max(1.0, 0.02 * v.wealth), v.landmass, v.pos, target=("w", v.id), poster=("t", v.id), years=3)


# ---------------------------------------------------------------- taking and doing
def work(realm, g):
    rng = realm.rng
    bands = [("p", p) for p in g["parties"].values()]
    bands += [("s", h) for h in realm.heroes() if free(realm, h) and party_of(g, h) is None]
    rng.shuffle(bands)
    for kind, b in bands:
        if kind == "p":
            members = [realm.chars[i] for i in b["members"] if free(realm, realm.chars[i])]
            if len(members) < 2:
                continue
            rank, land, size = b["rank"], home_of(realm, members[0]).landmass, len(members)
        else:
            members, rank = [b], getattr(b, "seal", 0)
            land, size = home_of(realm, b).landmass, 1
        lane = [q for q in g["quests"].values() if q["taken"] != realm.year and q["landmass"] == land
                and rank - 2 <= q["rank"] <= rank]
        lane = [q for q in lane if q["kind"] != "dragon" or (size >= 4 and rank >= 5 and (q["rank"] <= 5 or rank >= 6))]
        if size == 1:
            lane = [q for q in lane if q["kind"] not in ("dragon",) and q["rank"] <= max(0, rank - 1) + 1]
        if not lane or rng.random() > 0.6:
            continue
        q = max(lane, key=lambda q: q["reward"] * (1 + q["rank"]) * rng.uniform(0.7, 1.3))
        q["taken"] = realm.year
        go(realm, g, q, members, b if kind == "p" else None)


def go(realm, g, q, members, party):
    rng = realm.rng
    from .heroes import hurt, renown
    name = party["name"] if party else realm.who(members[0])
    leader = max(members, key=lambda h: h.level)
    big = q["rank"] >= 4 or q["kind"] == "dragon"
    if q["kind"] in ("slay", "dragon"):
        m = next((x for x in realm.monsters if x.alive and x.mid == q["target"][1]), None)
        if m is None:
            del g["quests"][q["id"]]
            return
        outcome = fight(realm, m, members, q)
    else:
        P = strength(realm, members, q["danger"])
        win = rng.random() < P / (P + 0.8 * q["danger"])
        outcome = "done" if win else "failed"
        for h in members:
            if rng.random() < (0.3 if win else 0.7):
                hurt(realm, h, rng.uniform(5, 40) * q["danger"] / max(1.0, P), f"died on the quest: {q['title']}")
    alive = [h for h in members if h.alive]
    lost = len(members) - len(alive)
    if party:
        party["lost"] += lost
    if outcome in ("done", "slain"):
        effect(realm, q, leader if leader.alive else (alive[0] if alive else None))
        share = q["reward"] / max(1, len(alive))
        merit = 10 * (q["rank"] + 1) ** 1.5 * (3 if outcome == "slain" and q["kind"] == "dragon" else 1)
        for h in alive:
            h.gold += share
            h.merit = getattr(h, "merit", 0.0) + merit
            renown(realm, h, merit / 4)
        if party:
            party["done"] += 1
        if q["id"] in g["quests"]:
            del g["quests"][q["id"]]
    elif outcome == "driven off":
        for h in alive:
            h.merit = getattr(h, "merit", 0.0) + 10 * (q["rank"] + 1) ** 1.5 * 0.5
    g["log"].append((realm.year, name, q["kind"], q["title"], q["rank"], outcome, lost))
    words = {"done": "completes", "slain": "completes", "failed": "fails at", "driven off": "drives off the wyrm in",
             "fled": "is beaten back from"}[outcome]
    tail = f"; {lost} of them do not come home" if lost else ""
    realm.say(f"{'**' if big and outcome != 'failed' else ''}{name}{'**' if big and outcome != 'failed' else ''} "
              f"({SEALS[q['rank']]}) {words} the quest: {q['title']}{tail}.",
              minor=not (big or lost >= 2))


def fight(realm, m, members, q):
    """Rounds against a monster; a dragon fights in its lair and takes wing if it can."""
    rng = realm.rng
    from .heroes import hurt
    lair = 0.7 if m.kind == "dragon" else 1.0
    for rnd in range(3):
        P = strength(realm, [h for h in members if h.alive], m.power)
        if P <= 0:
            return "fled"
        m.power -= P * rng.uniform(0.4, 0.9) * lair
        if m.power <= 0 and m.alive:
            m.alive = False
            slayer = max((h for h in members if h.alive), key=lambda h: h.cr(), default=None)
            realm.win["slain"] += 1
            from .monsters import on_monster_slain
            from .heroes import deed, renown
            purse = sum(getattr(m, "bounties", {}).values())
            on_monster_slain(realm, m, slayer)
            if slayer is not None and purse > 0:                          # the leader shares the purse out
                rest = [h for h in members if h.alive and h is not slayer]
                cut = purse / (len(rest) + 1)
                slayer.gold -= cut * len(rest)
                for h in rest:
                    h.gold += cut
            for h in members:
                if h.alive:
                    deed(h, m.kind)
                    renown(realm, h, MONSTER[m.kind][6] / max(1, len(members)) * 2)
            if m.kind == "dragon":
                realm.say(f"**A dragon falls.** {m.name} dies in its lair at the hands of "
                          f"{', '.join(realm.who(h) for h in members if h.alive)}.")
            return "slain"
        if getattr(m, "fled", None) == realm.year:
            return "driven off"
        for h in members:                                                  # it hits back
            if h.alive and rng.random() < 0.6:
                hurt(realm, h, rng.uniform(15, 60) * m.power / max(1.0, P + m.power) * 2,
                     f"was slain by {m.name}", killer=m)
    return "fled"


def effect(realm, q, who):
    """What a finished quest changes in the world."""
    rng = realm.rng
    kind, tgt = q["kind"], q["target"]
    if kind == "outlaw":
        c = realm.chars.get(tgt[1])
        if c is not None and c.alive:
            if rng.random() < 0.5:
                realm.kill(c, "was brought in dead by bounty hunters", quiet=True)
            else:
                c.jailed_until = realm.year + int(rng.integers(5, 20))
            c.bounty, c.posted = 0.0, 0.0
    elif kind in ("delve", "treasure"):
        r = next((x for x in realm.ruins if x.name == tgt[1]), None)
        if r is not None:
            r.conquered = realm.year
            loot = sorted(getattr(r, "loot", []), key=lambda it: -it.rarity)
            if loot and who is not None:
                from .heroes import give_item
                it = loot[0]
                r.loot.remove(it)
                give_item(realm, who, it)
    elif kind == "explore":
        t = realm.by_id.get(tgt[1])
        if t is not None and rng.random() < 0.4:
            cells = realm.remote_cells((realm.terrain != 0) & (realm.landmass == t.landmass))
            if len(cells):
                r, c = cells[int(rng.integers(len(cells)))]
                if rng.random() < 0.5:
                    from .world import Ruin
                    ruin = Ruin(realm.elder_name(), (int(r), int(c)), int(rng.integers(3, 9)), rng.uniform(1.0, 2.5))
                    realm.ruins.append(ruin)
                    found = ruin.name
                else:
                    i = int(rng.integers(len(realm.res_names)))
                    realm.res_grid[r, c] = i
                    found = f"a deposit of {realm.res_names[i]}"
                realm.say(f"Wardens charting the wild beyond {t.name} find {found}.", minor=True)
    elif kind == "escort":
        t = realm.by_id.get(tgt[1])
        if t is not None:
            t.wealth += q["reward"] * 2
    elif kind == "cull":
        t = realm.by_id.get(tgt[1])
        if t is not None:
            t.threat = max(0.0, getattr(t, "threat", 0.0) - 0.05)


def party_rows(realm):
    g = hall(realm)
    out = []
    for p in sorted(g["parties"].values(), key=lambda p: (-p["rank"], -p["done"])):
        names = [realm.chars[i].full_name() for i in p["members"] if i in realm.chars]
        out.append((p["name"], SEALS[p["rank"]], len(names), p["done"], p["lost"], p["formed"], ", ".join(names)[:90]))
    return out


def quest_rows(realm):
    g = hall(realm)
    return [(SEALS[q["rank"]], q["kind"], q["title"][:70], f"{q['reward']:,.0f}", q["posted"], q["expires"])
            for q in sorted(g["quests"].values(), key=lambda q: (-q["rank"], -q["reward"]))]
