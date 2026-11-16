"""The sea: ports and shipwrights, fleets and sea trade, explorers who find the rest of the world, colonies
planted across the water, naval battles and blockades, sea monsters, and pirates.

A pirate is a person with a past and a résumé. Each year they look for the best opportunity their nature
allows (raid shipping, raid a port, hunt treasure, bury it, take a letter of marque from a king at war,
hunt a sea monster, duel a rival, buy a bigger ship, retire on a pardon) and every choice, mutiny and
narrow escape is written into their record."""
import math

import numpy as np

from .characters import make_character
from .data import ADULT, OCEAN, PEOPLES, SINGULAR, TOWN_SYL
from .data2 import (HAVEN_STYLE, PIRATE_EPITHET, PIRATE_ORIGINS, PIRATE_SHIP_A, PIRATE_SHIP_B, SEA_MONSTER_NAMES,
                    SEA_MONSTERS, SHIPS)
from .geography import land_name, sea_name
from .skills import skill, use

PIRATE_CAP_BASE = 6


def d(a, b):
    return math.hypot(a[0] - b[0], a[1] - b[1])


def ports(realm, f):
    return [t for t in realm.alive_towns(f) if t.port]


# ---------------------------------------------------------------- fleets and trade
def fleets_year(realm):
    rng = realm.rng
    for k in realm.kingdoms:
        f = k.id
        if realm.extinct[f]:
            continue
        ps = ports(realm, f)
        if not ps:
            k.warships *= 0.9
            k.merchants *= 0.9
            continue
        wrights = [realm.chars[i] for i in realm.member_ids(f) if True
                   and realm.chars[i].profession == "shipwright"]
        best = max((skill(realm, w, "Seamanship") for w in wrights), default=10.0)
        tier = max(i for i, (_, need) in enumerate(SHIPS) if best >= need)
        from .civics import ship_cap
        tier = min(tier, max(0, ship_cap(realm, f) - 1))
        if tier > k.ship_tier:
            k.ship_tier = tier
            who = max(wrights, key=lambda w: skill(realm, w, "Seamanship")) if wrights else None
            if tier >= 2 and who is not None:
                realm.say(f"{realm.who(who)} lays down the first {SHIPS[tier][0]} of {k.name}.", minor=tier < 3)
                from .crafts import gain_fame
                gain_fame(realm, who, "shipwright", 80 * tier)
        for w in wrights:
            use(realm, w, "Seamanship", 1.2)
            use(realm, w, "Smithing", 0.5)
            w.gold += 30 + 10 * tier
        port_wealth = sum(t.wealth for t in ps)
        cost = 40 * (1 + k.ship_tier)
        from .resources import has as _has
        timber = 1.0 if (_has(k, "timber") or _has(k, "ancient wood")) else 0.4
        k.warships = 0.97 * k.warships + 0.004 * port_wealth / cost * (1.5 if realm.war[f].any() else 1.0) * timber
        k.merchants = min(4.0 * len(ps), 0.97 * k.merchants + 0.006 * port_wealth / cost + 0.3)
        # trade: ports of peoples at peace grow rich on one another, unless pirates and monsters feed on it
        partners = sum(1 for o in range(realm.npol) if o != f and not realm.extinct[o] and not realm.war[f, o]
                       and ports(realm, o))
        lost = realm.piracy.get(f, 0.0) + realm.sea_terror.get(f, 0.0)
        for t in ps:
            if k.blockaded > realm.year:
                t.wealth *= 0.985
                continue
            t.wealth += (2 + 1.5 * partners) * min(3.0, k.merchants / max(1, len(ps))) * max(0.1, 1 - lost) \
                * (1.3 if getattr(t, "colony", False) else 1.0)
        adm = realm.chars.get(k.council.get("admiral"))
        if adm is not None and adm.alive:
            use(realm, adm, "Seamanship", 1.0)
            use(realm, adm, "Navigation", 0.5)
    realm.piracy, realm.sea_terror = {}, {}


def navy_power(realm, k):
    adm = realm.chars.get(k.council.get("admiral"))
    a = skill(realm, adm, "Seamanship") if adm is not None and adm.alive else 10
    return k.warships * (1 + k.ship_tier) * (1 + a / 100)


# ---------------------------------------------------------------- exploration and colonies
def explore_year(realm):
    rng = realm.rng
    for k in realm.kingdoms:
        f = k.id
        if realm.extinct[f]:
            continue
        ps = ports(realm, f)
        if not ps:
            continue
        from .civics import ship_cap
        if ship_cap(realm, f) < 1:
            continue                                                  # no keels yet to carry them
        unknown = [L for L in realm.lands if L["id"] not in realm.known[f]]
        if not unknown:
            continue
        captains = [realm.chars[i] for i in realm.member_ids(f) if True
                    and realm.chars[i].profession == "captain"]
        tries = [(c, skill(realm, c, "Navigation")) for c in captains] + [(None, 15.0)]
        for c, nav in tries:
            src = ps[int(rng.integers(len(ps)))]
            tgt = min(unknown, key=lambda L: d(L["center"], src.pos) / math.sqrt(L["size"]) + rng.random() * 20)
            dist = d(tgt["center"], src.pos)
            p = (0.25 + nav / 200) * math.exp(-dist / (80 + nav * 3)) * (0.3 if c is None else 1.0)
            if c is not None:
                use(realm, c, "Navigation", 1.5)
                use(realm, c, "Seamanship", 1.0)
                if rng.random() < 0.02 + dist / 8000:
                    realm.kill(c, f"was lost at sea seeking {'a new land' if rng.random() < 0.5 else 'the edge of the map'}")
                    continue
            if rng.random() < p:
                discover(realm, f, tgt, c)
                unknown.remove(tgt)
                if not unknown:
                    break


def discover(realm, f, land, by):
    realm.known[f].add(land["id"])
    k = realm.kingdoms[f]
    first = not land["found_by"]
    land["found_by"][f] = realm.year
    k.discoveries.append((realm.year, land["id"], by.id if by is not None else None))
    who = realm.who(by) if by is not None else f"sailors of {k.name}"
    if by is not None:
        from .crafts import gain_fame
        gain_fame(realm, by, "explorer", (150 if first else 40) * (2 if land["kind"] == "continent" else 1))
    if land["kind"] == "continent" or first and land["kind"] == "great isle":
        realm.say(f"{'**A new world.** ' if first and land['kind'] == 'continent' else ''}{who} sights "
                  f"{land['name']}, a {land['kind']} of {land['size']:,} leagues"
                  + (", where no one of any people has ever set foot." if first else
                     f", already known to {', '.join(realm.pname(o) if o >= 0 else 'pirates' for o in land['found_by'] if o != f)}."),
                  minor=not first)
    else:
        realm.say(f"{who} charts {land['name']}.", minor=True)


def colonize_year(realm):
    rng = realm.rng
    for k in realm.kingdoms:
        f = k.id
        if realm.extinct[f]:
            continue
        ps = [t for t in ports(realm, f) if t.pop > 250]
        from .civics import can_colonize
        if not can_colonize(realm, f):
            continue
        foreign = [lid for lid in realm.known[f] if lid != realm.home_land.get(f)]
        if not ps or not foreign or k.merchants < 2:
            continue
        towns = realm.alive_towns(f)
        crowd = sum(t.pop for t in towns) / max(1, len(towns) * 600)
        if rng.random() > 0.06 * min(2.0, crowd + 0.3) * min(2.0, k.merchants / 5):
            continue
        lid = foreign[int(rng.integers(len(foreign)))]
        cells = np.argwhere(realm.coast & (realm.landmass == lid) & (realm.owner == -1) & ~realm.blocked)
        if not len(cells):
            continue
        cells = cells[rng.choice(len(cells), min(300, len(cells)), replace=False)]
        from .data import FOOD
        score = np.array([FOOD[k.people][realm.terrain[max(0, r - 2):r + 3, max(0, c - 2):c + 3]].sum() for r, c in cells])
        i = int(score.argmax())
        if score[i] < 12:
            continue
        src = max(ps, key=lambda t: t.pop)
        settlers = 0.25 * src.pop
        src.pop -= settlers
        src.wealth -= 40
        t = realm.add_settlement(f, (int(cells[i][0]), int(cells[i][1])), settlers, quiet=True)
        t.colony = True
        t.metropole = src.id
        k.colonies += 1
        first_here = sum(1 for s in realm.alive_towns(f) if s.landmass == lid) == 1
        realm.say(f"{'**A colony.** ' if first_here else ''}Settlers from {src.name} sail for "
                  f"{land_name(realm, lid)} and found {t.name}, a colony of {k.name}.", minor=not first_here)


# ---------------------------------------------------------------- naval war
def naval_war_year(realm):
    rng = realm.rng
    for i in range(realm.npol):
        for j in range(i + 1, realm.npol):
            if not realm.war[i, j] or rng.random() > 0.5:
                continue
            A, B = realm.kingdoms[i], realm.kingdoms[j]
            if A.warships < 1 or B.warships < 1:
                continue
            pa, pb = navy_power(realm, A) * rng.uniform(0.6, 1.4), navy_power(realm, B) * rng.uniform(0.6, 1.4)
            win, lose = (A, B) if pa > pb else (B, A)
            frac = max(pa, pb) / (pa + pb)
            lose.warships *= 1 - rng.uniform(0.3, 0.6) * frac
            win.warships *= 1 - rng.uniform(0.1, 0.3) * (1 - frac)
            lose.blockaded = realm.year + 3
            from .statecraft import battle_result
            battle_result(realm, win.id, lose.id)
            pw = ports(realm, win.realm)
            pl = ports(realm, lose.realm)
            where = sea_name(realm, sea_cell(realm, pl[0].pos)) if pl else "the open sea"
            big = A.warships + B.warships > 40
            realm.naval_battles.append((realm.year, where, win.short, lose.short,
                                        SHIPS[win.ship_tier][0], SHIPS[lose.ship_tier][0]))
            realm.say(f"**The Battle of {where.replace('the ', '').title()}.** The {SHIPS[win.ship_tier][0].lower()}s of "
                      f"{win.name} break the fleet of {lose.name}; its ports are blockaded.", minor=not big)
            for k_, won in ((win, True), (lose, False)):
                adm = realm.chars.get(k_.council.get("admiral"))
                if adm is not None and adm.alive:
                    use(realm, adm, "Seamanship", 2.0)
                    from .crafts import gain_fame
                    gain_fame(realm, adm, "admiral", 40 if won else 5)
                    if not won and rng.random() < 0.15:
                        realm.kill(adm, f"went down with the flagship of {k_.name}")
            # command of the sea lets the winner land troops on the loser's coasts, wherever they are
            if pl and rng.random() < 0.5:
                tgt = pl[int(rng.integers(len(pl)))]
                force = sum(t.pop for t in pw) * 0.05 * (1 + win.ship_tier * 0.2)
                if force > tgt.pop * 0.5 * (1 + 0.3 * tgt.walls):
                    from . import governance
                    governance.lose_lordship(realm, tgt)
                    realm.say(f"A host lands from the ships of {win.name} and takes {tgt.name} on "
                              f"{land_name(realm, tgt.landmass)}.", minor=True)
                    tgt.faction, tgt.capital = win.id, False
                    realm.town_moved(tgt)
                    tgt.pop *= 0.7
                    win.stats["taken"] += 1
                    lose.stats["lost"] += 1
                else:
                    tgt.pop *= 0.92
                    tgt.wealth *= 0.8


def sea_cell(realm, pos):
    r, c = pos
    win = realm.terrain[max(0, r - 3):r + 4, max(0, c - 3):c + 4]
    cells = np.argwhere(win == OCEAN)
    if not len(cells):
        return pos
    return (max(0, r - 3) + int(cells[0][0]), max(0, c - 3) + int(cells[0][1]))


def naval_contact(realm, i, j):
    """Two peoples can make war across the sea once they know each other's lands and both put fleets out."""
    if realm.kingdoms[i].warships < 3 or realm.kingdoms[j].warships < 3:
        return False
    return bool(realm.home_land.get(j) in realm.known[i] or realm.home_land.get(i) in realm.known[j])


# ---------------------------------------------------------------- sea monsters
def sea_monsters_year(realm):
    rng = realm.rng
    deep = realm.sea_dist >= 6
    share = max(1, int(deep.sum() / 15000))
    for kind, (prange, cap, _, p0, life) in SEA_MONSTERS.items():
        n = sum(1 for m in realm.monsters if m.alive and m.kind == kind)
        if n < cap * share and rng.random() < p0 * share:
            cells = np.argwhere(deep)
            r, c = cells[int(rng.integers(len(cells)))]
            name = f"{realm.name(SEA_MONSTER_NAMES)} the {kind.title()}"
            m = realm.new_monster(kind, (int(r), int(c)), rng.uniform(*prange), name=name, quiet=True)
            m.sea = True
            m.lifespan = life * rng.uniform(0.7, 1.2)
            if kind != "sea serpent":
                realm.say(f"Sailors of {sea_name(realm, m.pos).replace('the ', 'the ')} speak of {name} rising from "
                          f"the deep.", minor=kind == "kraken")
    for m in realm.monsters:
        if not m.alive or not getattr(m, "sea", False):
            continue
        for _ in range(3):                                          # it roams the open water
            pos = (int(np.clip(m.pos[0] + rng.integers(-4, 5), 0, realm.H - 1)),
                   int(np.clip(m.pos[1] + rng.integers(-4, 5), 0, realm.W - 1)))
            if realm.terrain[pos] == OCEAN:
                m.pos = pos
                break
        # it feeds on the shipping of any people whose ports lie near
        for k in realm.kingdoms:
            near = [t for t in ports(realm, k.realm) if d(t.pos, m.pos) < 30]
            if near:
                realm.sea_terror[k.id] = realm.sea_terror.get(k.id, 0.0) + min(0.4, m.power / 20000)
                if rng.random() < 0.05:
                    k.merchants *= 0.9
                # the navy hunts what threatens its ports
                if k.warships >= 3 and rng.random() < 0.15:
                    np_ = navy_power(realm, k) * 30
                    if rng.random() < np_ / (np_ + m.power):
                        m.power -= np_
                        if m.power <= 0:
                            m.alive = False
                            from .monsters import on_monster_slain
                            adm = realm.chars.get(k.council.get("admiral"))
                            on_monster_slain(realm, m, adm if adm is not None and adm.alive else None)
                            realm.say(f"The fleet of {k.name} hunts down {m.name}.", minor=m.kind == "sea serpent")
                            if adm is not None and adm.alive:
                                from .crafts import gain_fame
                                gain_fame(realm, adm, "admiral", SEA_MONSTERS[m.kind][2])
                            break
                    else:
                        k.warships *= 0.85


# ---------------------------------------------------------------- pirates
def ship_name(rng):
    return f"the {rng.choice(PIRATE_SHIP_A)} {rng.choice(PIRATE_SHIP_B)}"


def ensure_havens(realm):
    rng = realm.rng
    if realm.havens:
        return
    isles = [L for L in realm.lands if L["kind"] != "continent"]
    rng.shuffle(isles)
    for L in isles[:4]:
        cells = np.argwhere(realm.coast & (realm.landmass == L["id"]))
        if not len(cells):
            continue
        r, c = cells[int(rng.integers(len(cells)))]
        name = str(rng.choice(HAVEN_STYLE)).format(realm.name((["Rum", "Gallow", "Black", "Salt", "Rat", "Grog"],
                                                                 ["wick", "mouth", "bay", "hole", "tooth"])))
        realm.havens.append({"name": name, "pos": (int(r), int(c)), "land": L["id"], "founded": realm.year,
                             "king": None})


def log(realm, p, text):
    p.resume.append(f"Year {realm.year}: {text}")


def new_pirate(realm, src=None):
    rng = realm.rng
    y = realm.year
    hv = realm.havens[int(rng.integers(len(realm.havens)))]
    if src is None:
        f = int(rng.integers(4))
        p = make_character(realm, f, born=y - ADULT[f] - int(rng.integers(2, 15)))
        towns = realm.alive_towns(f) or realm.alive_towns()
        town = towns[int(rng.integers(len(towns)))].name if towns else "nowhere"
        origin = str(rng.choice(PIRATE_ORIGINS)).format(town=town, realm=realm.kingdoms[f].name)
    else:
        p = src
        origin = {"thief": f"a thief of {realm.kingdoms[p.realm].name}, wanted for {p.bounty:,.0f} gold, who "
                           f"fled to sea", "bandit": "a bandit chief who traded the road for the waves"}.get(
            p.profession, f"once {p.titles[-1] if p.titles else 'a nobody'}, now fallen")
    p.profession = "pirate"
    p.pirate = True
    p.ship = {"name": ship_name(rng), "tier": 0, "crew": int(rng.integers(20, 50))}
    p.haven = realm.havens.index(hv)
    p.infamy, p.marque, p.pirate_since = 0.0, None, y
    p.resume = [f"Born year {p.born}, {origin}."]
    p.skills["Seamanship"] = min(100.0, p.skills["Seamanship"] + 30)
    p.skills["Navigation"] = min(100.0, p.skills["Navigation"] + 15)
    p.skills["Blade"] = min(100.0, p.skills["Blade"] + 15)
    log(realm, p, f"takes {p.ship['name']}, a {SHIPS[0][0].lower()} with {p.ship['crew']} hands, and runs up the "
                  f"black flag at {hv['name']}.")
    realm.pirates.append(p.id)
    realm.say(f"{realm.who(p)} turns pirate, sailing {p.ship['name']} out of {hv['name']}.", minor=True)
    return p


def pirates_year(realm):
    rng = realm.rng
    ensure_havens(realm)
    if not realm.havens:
        return
    alive = [realm.chars[i] for i in realm.pirates if realm.chars[i].alive and getattr(realm.chars[i], "pirate", False)]
    cap = PIRATE_CAP_BASE + len(realm.lands) // 8
    if len(alive) < cap and rng.random() < 0.3:
        wanted = [realm.chars[i] for i in realm.living if realm.chars[i].profession in ("thief", "bandit")
                  and realm.chars[i].bounty > 400 and not getattr(realm.chars[i], "pirate", False)]
        alive.append(new_pirate(realm, wanted[0] if wanted and rng.random() < 0.5 else None))
    for p in alive:
        if p.alive:
            pirate_turn(realm, p)
    crown_pirate_king(realm)


def pirate_power(realm, p):
    return (p.ship["crew"] * (1 + p.ship["tier"]) * (1 + skill(realm, p, "Seamanship") / 100)
            * (1 + skill(realm, p, "Blade") / 200))


def richest_shipping(realm, p):
    opts = [k for k in realm.kingdoms if not realm.extinct[k.realm] and k.merchants > 1 and
            (p.marque is None or realm.war[p.marque, k.id])]
    return max(opts, key=lambda k: k.merchants * (1 + realm.rng.random()), default=None)


def pirate_turn(realm, p):
    rng = realm.rng
    per = p.personality
    choices = {"shipping": 3.0, "port": 1.0 + 2 * ("cruel" in per) + ("ambitious" in per),
               "treasure": 1.5 + ("ambitious" in per), "bury": 0.5 + ("paranoid" in per) + ("greedy" in per),
               "marque": 0.0, "beast": 0.3 + ("brave" in per), "duel": 0.3 + ("wrathful" in per),
               "refit": 0.0, "retire": 0.0}
    if p.gold > 2000:
        choices["bury"] += 1.0
    if p.gold > 600 * (p.ship["tier"] + 1) and p.ship["tier"] < 4:
        choices["refit"] = 2.5
    if p.marque is None and realm.war.any() and not ("cruel" in per):
        choices["marque"] = 0.8 + ("ambitious" in per)
    if p.gold > 4000 and (p.age(realm.year) > ADULT[p.people] + 25 or "content" in per):
        choices["retire"] = 2.0
    keys = list(choices)
    w = np.array([choices[k] for k in keys], float)
    act = keys[int(rng.choice(len(keys), p=w / w.sum()))]
    pw = pirate_power(realm, p)
    use(realm, p, "Seamanship", 1.0)
    from .crafts import gain_fame

    def infamy(x):
        p.infamy += x
        gain_fame(realm, p, "pirate", x)

    if act == "shipping":
        k = richest_shipping(realm, p)
        if k is None or not realm.alive_towns(k.id):
            return
        guard = navy_power(realm, k) * 0.3
        if rng.random() < pw / (pw + guard + 50):
            loot = 60 + 40 * k.merchants * rng.uniform(0.5, 1.5)
            p.gold += loot
            p.bounty += loot * (0 if p.marque is not None else 0.5)
            from .events import private
            private(realm, p, "piracy", loot, f"{p.full_name()} takes a merchantman of {k.name}")
            k.merchants *= 0.95
            realm.piracy[k.id] = realm.piracy.get(k.id, 0.0) + 0.05
            infamy(loot / 40)
            log(realm, p, f"takes a {SINGULAR[k.people].lower()} merchantman off {sea_name(realm, sea_cell(realm, (realm.capital_of(k.realm) or realm.alive_towns(k.realm)[0]).pos))}"
                          f"{' under letters of marque' if p.marque is not None else ''}: {loot:,.0f} gold.")
        else:
            p.ship["crew"] = int(p.ship["crew"] * 0.8)
            log(realm, p, f"is driven off by the navy of {k.name}, losing a fifth of the crew.")
            if rng.random() < 0.15:
                captured(realm, p, k)
                return
    elif act == "port":
        targets = [t for t in realm.alive_towns() if t.port and (p.marque is None or realm.war[p.marque, t.faction])]
        if not targets:
            return
        t = targets[int(rng.integers(len(targets)))]
        defence = t.pop * 0.1 * (1 + 0.3 * t.walls)
        if rng.random() < pw / (pw + defence):
            loot = t.wealth * 0.3
            t.wealth -= loot
            t.pop *= 0.95
            p.gold += loot
            p.bounty += loot
            burn = "cruel" in per and rng.random() < 0.2 and t.pop < 400 and not t.capital
            infamy(loot / 20 + (40 if burn else 0))
            log(realm, p, f"sacks the port of {t.name} ({realm.pname(t.faction)}){' and burns it to the waterline' if burn else ''}: {loot:,.0f} gold.")
            realm.say(f"{realm.who(p)}, captain of {p.ship['name']}, sacks {t.name}"
                      f"{' and burns it to the waterline' if burn else ''}.", minor=not burn and loot < 400)
            if burn:
                realm.destroy(t)
        else:
            p.ship["crew"] = int(p.ship["crew"] * 0.7)
            log(realm, p, f"is beaten off the walls of {t.name}.")
    elif act == "treasure":
        ruins = [r for r in realm.ruins if (r.loot or r.riches > 0)]
        unknown = [L for L in realm.lands if not L["found_by"]]
        if unknown and rng.random() < 0.3:
            L = unknown[int(rng.integers(len(unknown)))]
            L["found_by"][-1] = realm.year
            infamy(30)
            log(realm, p, f"finds an unknown shore: {L['name']}, a {L['kind']} no chart has ever shown.")
            realm.say(f"The pirate {realm.who(p)} is the first to see {L['name']}, and tells no one but the crew.",
                      minor=L["kind"] != "continent")
        elif ruins:
            r = ruins[int(rng.integers(len(ruins)))]
            if r.loot:
                it = r.loot.pop(0)
                from .heroes import give_item
                give_item(realm, p, it, quiet=True)
                infamy(20 + 40 * it.bonus)
                log(realm, p, f"digs {it.label()} out of {r.name}.")
                if it.rarity >= 4:
                    realm.say(f"{realm.who(p)} digs {it.label()} out of {r.name}.")
            else:
                r.riches -= 1
                p.gold += 100 * r.depth
                log(realm, p, f"loots {r.name} for {100 * r.depth:,.0f} gold.")
    elif act == "bury":
        amt = p.gold * 0.6
        isles = [L for L in realm.lands if L["kind"] == "isle"]
        L = isles[int(rng.integers(len(isles)))] if isles else realm.lands[0]
        cells = np.argwhere(realm.landmass == L["id"])
        r, c = cells[int(rng.integers(len(cells)))]
        from .world import Ruin
        ru = Ruin(f"the buried hoard of {p.name}", (int(r), int(c)), int(2 + amt // 500), 1.5)
        ru.loot = [it for it in p.items if it.rarity >= 3][:2]
        for it in ru.loot:
            p.items.remove(it)
        p.invalidate()
        realm.ruins.append(ru)
        p.gold -= amt
        log(realm, p, f"buries {amt:,.0f} gold{' and ' + ', '.join(i.label() for i in ru.loot) if ru.loot else ''} "
                      f"on {L['name']}, and draws a map.")
    elif act == "marque":
        wars = [(i, j) for i in range(realm.npol) for j in range(realm.npol) if i != j and realm.war[i, j]]
        if wars:
            i, j = wars[int(rng.integers(len(wars)))]
            p.marque = i
            p.bounty *= 0.5
            log(realm, p, f"takes letters of marque from {realm.kingdoms[i].name}: a privateer now, licensed to "
                          f"prey on the {realm.pname(j)}.")
            realm.say(f"{realm.who(p)} sails as a privateer for {realm.kingdoms[i].name}.", minor=True)
    elif act == "beast":
        ms = [m for m in realm.monsters if m.alive and getattr(m, "sea", False)]
        if not ms:
            return
        m = min(ms, key=lambda m: d(m.pos, realm.havens[p.haven]["pos"]))
        if rng.random() < pw * 3 / (pw * 3 + m.power):
            m.power -= pw * 4
            if m.power <= 0:
                m.alive = False
                from .monsters import on_monster_slain
                on_monster_slain(realm, m, p)
                infamy(SEA_MONSTERS[m.kind][2])
                log(realm, p, f"kills {m.name} and hangs its teeth from the yards.")
                realm.say(f"The pirate {realm.who(p)} kills {m.name}.", minor=m.kind == "sea serpent")
            else:
                log(realm, p, f"wounds {m.name} and lives to tell it.")
        else:
            p.ship["crew"] = int(p.ship["crew"] * 0.5)
            log(realm, p, f"is half-drowned by {m.name}; half the crew are lost.")
            if rng.random() < 0.25:
                realm.kill(p, f"was dragged under by {m.name}", killer=m)
                return
    elif act == "duel":
        rivals = [realm.chars[i] for i in realm.pirates if realm.chars[i].alive and i != p.id
                  and getattr(realm.chars[i], "pirate", False)]
        if rivals:
            o = rivals[int(rng.integers(len(rivals)))]
            po = pirate_power(realm, o)
            win, lose = (p, o) if rng.random() < (pw + 1e-9) / (pw + po + 2e-9) else (o, p)
            log(realm, win, f"meets {lose.full_name()} of {lose.ship['name']} and takes the ship.")
            log(realm, lose, f"is beaten by {win.full_name()} and goes over the side.")
            win.gold += lose.gold
            win.ship["crew"] += lose.ship["crew"] // 2
            infamy(30) if win is p else None
            realm.say(f"{realm.who(win)} defeats the pirate {lose.full_name()} and takes {lose.ship['name']}.",
                      minor=True)
            realm.kill(lose, f"was killed by the pirate {win.full_name()}")
            if not p.alive:
                return
    elif act == "refit":
        cost = 600 * (p.ship["tier"] + 1)
        p.gold -= cost
        p.ship["tier"] += 1
        p.ship["crew"] += 30
        log(realm, p, f"refits {p.ship['name']} as a {SHIPS[p.ship['tier']][0].lower()}.")
    elif act == "retire":
        towns = [t for t in realm.alive_towns(p.realm) if t.port and t.lord is None]
        if towns and rng.random() < 0.5:
            t = towns[0]
            from .governance import grant_lordship
            p.pirate = False
            p.profession = "merchant"
            p.gold -= 3000
            grant_lordship(realm, p, t)
            log(realm, p, f"buys a royal pardon and the lordship of {t.name}, and goes ashore an honest "
                          f"{'woman' if p.female else 'man'}.")
            realm.say(f"The pirate {realm.who(p)} buys a pardon and becomes {'Lady' if p.female else 'Lord'} of "
                      f"{t.name}.")
        else:
            p.pirate = False
            p.profession = None
            log(realm, p, f"sails off the edge of the charts with a hold full of gold, and is never seen again.")
            realm.say(f"{realm.who(p)} sails away with a fortune and is never seen again.", minor=True)
        return
    # every year at sea: mutiny, the navy, the weather
    if p.alive and getattr(p, "pirate", False):
        if p.gold < 50 and ("cruel" in per or "greedy" in per) and rng.random() < 0.2:
            mutiny(realm, p)
        elif rng.random() < 0.01:
            log(realm, p, "goes down with all hands in a storm.")
            realm.kill(p, "went down with all hands in a storm")


def captured(realm, p, k):
    log(realm, p, f"is taken by the navy of {k.name} and hanged at the harbour mouth.")
    realm.say(f"{realm.who(p)}, the pirate, is taken by the navy of {k.name} and hanged; the bounty was "
              f"{p.bounty:,.0f} gold.", minor=p.infamy < 300)
    adm = realm.chars.get(k.council.get("admiral"))
    if adm is not None and adm.alive:
        from .crafts import gain_fame
        gain_fame(realm, adm, "admiral", 20 + p.infamy / 10)
    realm.kill(p, f"was hanged by {k.name}")


def mutiny(realm, p):
    rng = realm.rng
    q = make_character(realm, p.realm, born=realm.year - ADULT[p.people] - int(rng.integers(3, 20)))
    q.home = None
    log(realm, p, f"is overthrown by mutineers and marooned on a sandbar.")
    survived = rng.random() < 0.3
    old_ship = p.ship
    realm.pirates.append(q.id)
    q.profession, q.pirate = "pirate", True
    q.ship = dict(old_ship)
    q.haven, q.infamy, q.marque, q.pirate_since = p.haven, 0.0, None, realm.year
    q.resume = [f"Born year {q.born}, quartermaster of {old_ship['name']} under {p.full_name()}.",
                f"Year {realm.year}: leads the mutiny and takes {old_ship['name']} for their own."]
    q.skills["Seamanship"] = min(100.0, q.skills["Seamanship"] + 30)
    realm.say(f"Mutiny aboard {old_ship['name']}: {realm.who(q)} turns the crew on {p.full_name()}.", minor=True)
    if survived:
        p.ship = {"name": "a stolen fishing boat", "tier": 0, "crew": 5}
        p.gold = 0
        log(realm, p, "survives the sandbar, steals a fishing boat, and swears revenge.")
    else:
        realm.kill(p, "was marooned by mutineers and died of thirst")


def crown_pirate_king(realm):
    alive = [realm.chars[i] for i in realm.pirates if realm.chars[i].alive and getattr(realm.chars[i], "pirate", False)]
    if not alive:
        return
    top = max(alive, key=lambda p: p.infamy)
    if top.infamy < 400:
        return
    if realm.pirate_king != top.id:
        old = realm.chars.get(realm.pirate_king)
        realm.pirate_king = top.id
        if top.epithet is None:
            top.epithet = str(realm.rng.choice(PIRATE_EPITHET))
        log(realm, top, "is acclaimed Pirate King by the Brethren of the havens.")
        realm.pirate_kings.append((realm.year, top.id))
        realm.say(f"**{realm.who(top)} is acclaimed Pirate King** by the Brethren of the havens"
                  + (f", in place of {old.full_name()}" if old is not None and old.alive else "") + ".")


def sea_year(realm):
    fleets_year(realm)
    explore_year(realm)
    colonize_year(realm)
    naval_war_year(realm)
    sea_monsters_year(realm)
    pirates_year(realm)
