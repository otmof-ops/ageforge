"""Armies on the map.

When a war begins each side raises a host at its seat: levies from its towns and its regiments of men-at-arms,
under a commander (the marshal, or a martial ruler), with the realm's heroes, dragon riders and war mages riding
with it. Four times a year every host marches (slower through hills and mountains, never across the sea),
loses men to attrition (more in enemy land, more where the harvest failed, more when it is too big for the land
to feed), and looks for its target: the attacker the towns of its war goal, the defender the enemy host in its
own lands or a town under siege. Hosts that meet fight a field battle: men, quality, the commander's skill,
morale, the defender's ground (hills, forest, mountains), heroes, dragons. A host at an enemy town besieges it;
walls and the garrison hold it off, war mages bring it down faster. A taken town is occupied, sacked a little,
and counts towards the warscore; at peace it goes back, unless it was the prize.

The same hosts put down peasant risings, and march on calamities that walk their lands."""
import math

import numpy as np

from .data import MOUNTAIN, OCEAN, PEOPLES, FOREST, HILLS
from .geography import land_name

TICKS = 4
SPEED = 9


class Army:
    def __init__(self, aid, realm_id, pos, men, quality, commander, year, side="realm"):
        self.id, self.realm, self.pos, self.men, self.quality = aid, realm_id, pos, float(men), quality
        self.commander, self.year, self.side = commander, year, side
        self.morale, self.siege, self.target, self.heroes = 1.0, 0.0, None, []
        self.start_men = float(men)
        self.name = ""
        self.rebel_of = None
        self.monster = None


def power(realm, a):
    c = realm.chars.get(a.commander)
    lead = 1 + (c.attr("mar") / 40 if c is not None and c.alive else 0)
    heroes = sum(realm.chars[h].cr() for h in a.heroes if h in realm.chars and realm.chars[h].alive) / 50
    k = realm.kingdoms[a.realm] if a.realm is not None else None
    dragons = 1 + 0.25 * getattr(k, "dragons", 0) if k is not None else 1
    return (a.men * a.quality * lead * a.morale + heroes) * dragons


def raise_army(realm, k, why=""):
    """Muster a realm's host at its seat, if it has none in the field."""
    if any(a.realm == k.id and a.side == "realm" and a.men > 0 for a in realm.armies):
        return None
    cap = realm.capital_of(k.id)
    if cap is None:
        return None
    from .civics import MAA
    maa_men = sum(n * 300 for n in getattr(k, "maa", {}).values())
    maa_q = sum(MAA[m][0] * n * 300 for m, n in getattr(k, "maa", {}).items())
    levy = getattr(k, "levy", 0.0) * 0.6
    men = levy + maa_men
    if men < 50:
        return None
    quality = (levy + maa_q) / max(1.0, men)
    r = realm.chars.get(k.ruler)
    mar = realm.chars.get(k.council.get("marshal"))
    cmd = r if r is not None and "brave" in r.personality and r.adult(realm.year) else (mar or r)
    a = Army(realm.next_army, k.id, cap.pos, men, quality, cmd.id if cmd else None, realm.year)
    realm.next_army += 1
    heroes = sorted((h for h in realm.heroes() if h.realm == k.id), key=lambda h: -h.cr())[:3]
    a.heroes = [h.id for h in heroes]
    a.name = f"the host of {k.name}"
    realm.armies.append(a)
    k.gold -= men * 0.004                 # muster costs
    from .events import book
    book(realm, k.id, "muster", -men * 0.004, f"musters {int(men):,} men")
    return a


def raise_for_war(realm, a_id, b_id):
    for f in (a_id, b_id):
        raise_army(realm, realm.kingdoms[f])
        for t in realm.kingdoms:
            if getattr(t, "overlord", None) == f and not realm.extinct[t.id]:
                raise_army(realm, t)


def raise_rebels(realm, k, town, leader, why):
    a = Army(realm.next_army, None, town.pos, town.pop * 0.12 + getattr(town, "rallied", 0.0), 0.8, leader.id,
             realm.year, side="rebel")
    town.rallied = 0.0
    realm.next_army += 1
    a.rebel_of = k.id
    a.home = town.id
    a.name = f"the {town.name} rising"
    a.why = why
    realm.armies.append(a)
    raise_army(realm, k)


def enemies(realm, a, b):
    if a.side == "rebel":
        return b.realm == a.rebel_of
    if b.side == "rebel":
        return a.realm == b.rebel_of
    return a.realm is not None and b.realm is not None and a.realm != b.realm and bool(realm.war[a.realm, b.realm])


def step_toward(realm, pos, target, speed):
    r, c = pos
    for _ in range(speed):
        dy, dx = target[0] - r, target[1] - c
        if abs(dy) + abs(dx) <= 1:
            break
        opts = []
        sy, sx = int(np.sign(dy)), int(np.sign(dx))
        for my, mx in ((sy, sx), (sy, 0), (0, sx), (sy, -sx), (-sy, sx)):
            ny, nx = r + my, c + mx
            if (my or mx) and 0 <= ny < realm.H and 0 <= nx < realm.W and realm.terrain[ny, nx] != OCEAN:
                opts.append((ny, nx))
        if not opts:
            break
        r, c = opts[0]
        if realm.terrain[r, c] in (MOUNTAIN,):
            speed -= 1                                                # mountains are slow going
    return (r, c)


def pick_target(realm, a):
    """A town to take, a host to fight, or a monster to bring down."""
    if a.monster is not None:
        m = a.monster
        return ("monster", m) if m.alive else None
    lid = int(realm.landmass[a.pos])
    foes = [b for b in realm.armies if b is not a and b.men > 0 and enemies(realm, a, b)
            and int(realm.landmass[b.pos]) == lid]
    if a.side == "rebel":
        # a rising marches on the nearest seat of power: a lord's town or the crown's own
        k = realm.kingdoms[a.rebel_of]
        seats = [t for t in realm.alive_towns(k.id) if t.landmass == lid and (t.capital or t.lord is not None)
                 and getattr(t, "id", None) != getattr(a, "home", None)]
        if not seats:
            return None
        return ("town", min(seats, key=lambda t: math.hypot(t.pos[0] - a.pos[0], t.pos[1] - a.pos[1])))
    near = [b for b in foes if math.hypot(b.pos[0] - a.pos[0], b.pos[1] - a.pos[1]) < 30]
    if near:
        return ("army", min(near, key=lambda b: math.hypot(b.pos[0] - a.pos[0], b.pos[1] - a.pos[1])))
    w = None
    for key, rec in realm.wars.items():
        if a.realm in (rec["attacker"], rec["defender"]):
            w = rec
            break
    towns = []
    for f in range(realm.npol):
        if a.realm is not None and realm.war[a.realm, f]:
            towns += [t for t in realm.alive_towns(f) if t.landmass == lid and getattr(t, "occupied_by", None) is None]
    if not towns:
        return None
    goal = w["goal"].get("duchy") if w is not None and w["attacker"] == a.realm else None
    if goal is not None:
        g = [t for t in towns if getattr(t, "duchy", None) == goal]
        towns = g or towns
    return ("town", min(towns, key=lambda t: math.hypot(t.pos[0] - a.pos[0], t.pos[1] - a.pos[1])))


def terrain_bonus(realm, pos):
    t = realm.terrain[pos]
    return 1.3 if t == MOUNTAIN else 1.2 if t == HILLS else 1.15 if t == FOREST else 1.0


def battle(realm, a, b):
    rng = realm.rng
    pa, pb = power(realm, a), power(realm, b) * terrain_bonus(realm, b.pos)
    ra, rb = pa * rng.uniform(0.7, 1.3), pb * rng.uniform(0.7, 1.3)
    win, lose = (a, b) if ra > rb else (b, a)
    frac = max(ra, rb) / (ra + rb)
    lost_l = lose.men * rng.uniform(0.25, 0.45) * frac
    lost_w = win.men * rng.uniform(0.06, 0.16) * (1 - frac) * 2
    lose.men -= lost_l
    win.men -= lost_w
    lose.morale *= 0.7
    win.morale = min(1.2, win.morale * 1.05)
    # the defeated fall back towards home
    home = realm.capital_of(lose.realm) if lose.realm is not None else None
    if home is not None:
        lose.pos = step_toward(realm, lose.pos, home.pos, SPEED)
    near = min(realm.alive_towns(), key=lambda t: math.hypot(t.pos[0] - a.pos[0], t.pos[1] - a.pos[1]), default=None)
    where = near.name if near is not None else "the field"
    total = lost_l + lost_w
    realm.battles_fought.append((realm.year, where, win.name, lose.name, int(win.men + lost_w), int(lose.men + lost_l),
                                 int(total)))
    big = total > 3000
    realm.say(f"{'**' if big else ''}The Battle of {where}{'.**' if big else '.'} {win.name[0].upper() + win.name[1:]} "
              f"({int(win.men + lost_w):,} men) breaks {lose.name} ({int(lose.men + lost_l):,}); "
              f"{int(total):,} fall.", minor=not big)
    # the commander of the beaten host may not come home
    cl = realm.chars.get(lose.commander)
    cw = realm.chars.get(win.commander)
    if cl is not None and cl.alive and rng.random() < 0.08:
        realm.kill(cl, f"fell at the Battle of {where}", by=cw)
    if cw is not None and cw.alive:
        from .statecraft import honour
        honour(realm, cw, 20 + total / 400, "war")
    for hid in lose.heroes:
        h = realm.chars.get(hid)
        if h is not None and h.alive and rng.random() < 0.1:
            realm.kill(h, f"fell at the Battle of {where}", by=cw)
    # the dead were someone's levy: the towns feel it
    for side, lost in ((lose, lost_l), (win, lost_w)):
        if side.realm is not None:
            towns = realm.alive_towns(side.realm)
            tot = sum(t.pop for t in towns) or 1
            for t in towns:
                t.pop -= lost * t.pop / tot * 0.7
    if win.side == "rebel" and lose.men < 0.5 * (lose.men + lost_l):
        # the royal host is broken in the field: the crown must treat with the rising
        home = realm.by_id.get(getattr(win, "home", None))
        if home is not None and home.alive:
            rebels_win(realm, win, home)
            return
    if win.side != "rebel" and lose.side != "rebel" and win.realm is not None:
        from .statecraft import battle_result
        battle_result(realm, win.realm, lose.realm)
        battle_result(realm, win.realm, lose.realm)


def siege(realm, a, t):
    rng = realm.rng
    from .civics import MAA
    k = realm.kingdoms[a.realm] if a.realm is not None else None
    mages = getattr(k, "maa", {}).get("War Mages", 0) if k is not None else 0
    hold = getattr(t, "garrison", t.pop * 0.02) * (1 + 0.4 * t.walls) + 30
    a.siege += 25 * power(realm, a) / (hold * 10) * (1 + 0.5 * mages)
    if a.siege < 100:
        return
    a.siege = 0.0
    if a.side == "rebel":
        rebels_win(realm, a, t)
        return
    old = t.faction
    t.occupied_by = a.realm
    loot = t.wealth * 0.4
    t.wealth -= loot
    t.pop *= 0.92
    if k is not None:
        k.gold += loot
        from .events import book
        book(realm, k.id, "loot", loot, f"sacks {t.name}")
    from .statecraft import battle_result
    battle_result(realm, a.realm, old, t)
    realm.sieges_won += 1
    realm.say(f"{a.name[0].upper() + a.name[1:]} takes {t.name} after a siege.", minor=not t.capital)


def rebels_win(realm, a, t):
    from .opinion import remember
    k = realm.kingdoms[a.rebel_of]
    r = realm.chars.get(k.ruler)
    k.tax_relief = realm.year + 20
    a.men = 0
    realm.say(f"**The rising wins.** {a.name[0].upper() + a.name[1:]} takes {t.name}; {realm.who(r) if r else k.name} must grant "
              f"what the people demanded ({a.why}).")
    if r is not None:
        r.legitimacy = max(0.0, (r.legitimacy or 50) - 10)
    home = realm.by_id.get(getattr(a, "home", None))
    if home is not None and home.alive and home.faction == k.id and home.pop > 800 and realm.rng.random() < 0.4:
        free_commune(realm, k, home, realm.chars.get(a.commander))


def free_commune(realm, k, town, leader):
    from .dynasty import Kingdom, crown
    from .civics import setup_realm
    nid = len(realm.kingdoms)
    nk = Kingdom(nid, town.culture, f"the Free Commune of {town.name}", short=f"commoners of {town.name}")
    nk.place, nk.rank, nk.parent, nk.founded_year = town.name, "duchy", k.id, realm.year
    nk.color = (230, 230, 230)
    nk.faith = k.faith
    realm.kingdoms.append(nk)
    realm.grow_tables()
    realm.known[nid] = set(realm.known[k.id])
    realm.home_land[nid] = town.landmass
    setup_realm(realm, nk, "republic")
    nk.gov = "republic"
    nk.law = "elective"
    town.faction, town.capital = nid, True
    realm.town_moved(town)
    if leader is not None and leader.alive:
        leader.realm = nid
        crown(realm, nk, leader, "raised up when the line failed")
    realm.truce[k.id, nid] = realm.truce[nid, k.id] = realm.year + 15
    realm.breakaways.append((realm.year, nid, k.id, leader.id if leader else None))
    realm.say(f"**A free commune.** {town.name} throws off {k.name} and governs itself.")


def march_on_calamities(realm):
    """A realm at peace raises its host against a calamity that walks its own land."""
    for m in realm.monsters:
        if not m.alive or m.kind != "calamity":
            continue
        for k in realm.kingdoms:
            if realm.extinct[k.id] or realm.war[k.id].any():
                continue
            towns = [t for t in realm.alive_towns(k.id) if t.landmass == m.landmass
                     and math.hypot(t.pos[0] - m.pos[0], t.pos[1] - m.pos[1]) < 30]
            if len(towns) < 3 or any(a.monster is m and a.realm == k.id for a in realm.armies):
                continue
            a = raise_army(realm, k)
            if a is not None:
                a.monster = m
                a.name = f"the host of {k.name} against {m.name.split(',')[0]}"
                realm.say(f"{k.name} musters {int(a.men):,} against {m.name}.", minor=True)


def fight_monster(realm, a, m):
    rng = realm.rng
    p = power(realm, a)
    dmg = p * rng.uniform(0.05, 0.12)
    m.power -= dmg
    a.men *= 1 - rng.uniform(0.05, 0.2) * min(3.0, m.power / max(1.0, p))
    a.morale *= 0.95
    if m.power <= 0 and m.alive:
        m.alive = False
        m.slain_by = a.commander
        k = realm.kingdoms[a.realm]
        a.name = f"the host of {k.name}"
        realm.calamity_log.append((m.name, m.born, realm.year, f"brought down by {a.name}"))
        realm.say(f"**{a.name[0].upper() + a.name[1:]} brings down {m.name}**, at a terrible cost.")
        a.monster = None
        a.men = 0


def armies_year(realm):
    """Four seasons of marching, fighting and besieging."""
    rng = realm.rng
    march_on_calamities(realm)
    for _ in range(TICKS):
        for a in list(realm.armies):
            if a.men < 30:
                continue
            # a host whose war is over goes home
            if a.side == "realm" and a.monster is None and (a.realm is None or not realm.war[a.realm].any()) and \
                    not any(b.side == "rebel" and b.rebel_of == a.realm and b.men > 0 for b in realm.armies):
                a.men = 0
                continue
            if a.side == "rebel" and realm.extinct[a.rebel_of]:
                a.men = 0
                continue
            if a.monster is not None and not a.monster.alive:          # its calamity is gone: it is a host again
                a.monster = None
                if a.realm is not None:
                    a.name = f"the host of {realm.kingdoms[a.realm].name}"
                if a.realm is None or not realm.war[a.realm].any():
                    a.men = 0
                    continue
            tgt = pick_target(realm, a)
            if tgt is None:
                continue
            kind, obj = tgt
            dest = obj.pos
            if math.hypot(dest[0] - a.pos[0], dest[1] - a.pos[1]) > 2:
                a.pos = step_toward(realm, a.pos, dest, SPEED)
            # attrition: a little always, more in enemy land, more where the harvest failed
            hostile = realm.owner_fac[a.pos] >= 0 and a.realm is not None and \
                bool(realm.war[a.realm, realm.owner_fac[a.pos]])
            from .economy import harvest_at
            att = 0.006 + 0.01 * hostile + 0.01 * max(0.0, 1 - harvest_at(realm, a.pos)) + \
                0.004 * max(0.0, a.men / 20000 - 1)
            a.men *= 1 - att
            if kind == "army" and obj.men > 0 and math.hypot(obj.pos[0] - a.pos[0], obj.pos[1] - a.pos[1]) <= 3:
                battle(realm, a, obj)
            elif kind == "town" and math.hypot(dest[0] - a.pos[0], dest[1] - a.pos[1]) <= 2 and obj.alive:
                siege(realm, a, obj)
            elif kind == "monster" and math.hypot(dest[0] - a.pos[0], dest[1] - a.pos[1]) <= 3:
                fight_monster(realm, a, obj)
    # beaten risings melt away, their leaders hanged
    for a in realm.armies:
        if a.side == "rebel" and a.men < 30 and not getattr(a, "done", False):
            a.done = True
            k = realm.kingdoms[a.rebel_of]
            home = realm.by_id.get(getattr(a, "home", None))
            if home is not None:
                home.rising = None
            c = realm.chars.get(a.commander)
            r = realm.chars.get(k.ruler)
            if c is not None and c.alive and r is not None:
                realm.kill(c, f"was hanged when {a.name} was crushed", by=r)
    realm.armies = [a for a in realm.armies if a.men >= 30]


def end_occupations(realm, a_id, b_id, keep_goal=None):
    """At peace, occupied towns go back, except what the treaty gave away."""
    for t in realm.alive_towns():
        occ = getattr(t, "occupied_by", None)
        if occ in (a_id, b_id) and t.faction in (a_id, b_id):
            t.occupied_by = None
