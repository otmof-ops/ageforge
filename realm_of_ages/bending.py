"""The four bending tribes, and the Concord.

Bending is a bloodline: tidebound, stoneborn, flameheart and skyborn children can move water, stone, fire
and air. It is common among the four remote tribes and rare everywhere else. A bender climbs four tiers of
forms and, at mastery, opens one of two high arts of their element.

The Concord is one soul, reborn again and again as a bender of each element in turn. It comes only when the
world needs a hero: a calamity walks, a tyrant rules, the peoples are at war on every side, or the
Wyrm-Sovereign grows too strong. When the need comes and no Concord lives, the next child born with the next
element's gift, anywhere in the world, carries the soul. At adulthood it is revealed, learns all four
elements, and turns on whatever the world needs it to face. When it dies, the soul waits for the next need."""
import math

import numpy as np

from .characters import make_character
from .data import ADULT, PEOPLES, ROMAN
from .data2 import BEND_FORMS, BEND_TIER_NAMES, CONCORD, ELEMENT_GENE, ELEMENTS, GENE_ELEMENT, TRIBES
from .skills import use

TRIBE_CAP = 7


class Tribe:
    def __init__(self, element, name, pos):
        self.element, self.name, self.pos = element, name, pos
        self.members, self.chief, self.chiefs = set(), None, []


def found_tribes(realm):
    from .data import DESERT, MOUNTAIN, OCEAN
    rng = realm.rng
    land = realm.terrain != OCEAN
    coast = land & (np.roll(realm.terrain, 1, 0) == OCEAN)
    spots = {"tide": coast, "stone": realm.terrain == MOUNTAIN, "flame": realm.terrain == DESERT,
             "sky": realm.terrain == MOUNTAIN}
    for e in ELEMENTS:
        cells = realm.remote_cells(spots[e] & (realm.owner == -1)) if (spots[e] & (realm.owner == -1)).any() \
            else np.argwhere(spots[e])
        if not len(cells):
            cells = np.argwhere(land)
        r, c = cells[int(rng.integers(len(cells)))]
        t = Tribe(e, TRIBES[e], (int(r), int(c)))
        realm.tribes[e] = t
        for _ in range(4):
            join_tribe(realm, t, new_bender(realm, e, None))
    realm.say("Far from the kingdoms live the four bending tribes: " + "; ".join(t.name for t in realm.tribes.values())
              + ". It is told that once in every age of need, one soul is born who can bend all four.")


def new_bender(realm, e, parent):
    rng = realm.rng
    y = realm.year
    f = int(rng.choice(4, p=[0.55, 0.15, 0.15, 0.15]))
    c = make_character(realm, f, born=y - ADULT[f] - int(rng.integers(0, 12)))
    c.genetic.add(ELEMENT_GENE[e])
    setup_bender(c, e, rng)
    return c


def setup_bender(c, e, rng):
    c.element = e
    c.bend = {e: float(rng.uniform(15, 35))}
    c.high_art = None


def join_tribe(realm, t, c):
    t.members.add(c.id)
    c.tribe = t.element


def bend_tier(lv):
    return min(3, int(lv // 25))


def bend_power(realm, c):
    from .lineage import legacy_bonus
    tot = 0.0
    for e, lv in c.bend.items():
        lv = min(120.0, lv + legacy_bonus(realm, c, "bend"))
        tot += 25 * 1.9 ** (lv / 15)
    if c.high_art:
        tot *= 1.3
    if getattr(c, "concord", False) and c.adult(realm.year):
        tot *= 2.0 + 0.5 * len(c.bend)
    return tot * (1 + c.level / 80)


def born_bender(realm, c):
    """Called on every birth: does the child carry a bending gift, and is it the Concord?"""
    rng = realm.rng
    parents = [realm.chars[p] for p in (c.father, c.mother) if p is not None]
    gift = None
    for p in parents:
        el = getattr(p, "element", None)
        if el and rng.random() < 0.5:
            gift = el
    if gift is None and rng.random() < 0.003:
        gift = ELEMENTS[int(rng.integers(4))]
    if gift is None:
        return
    c.genetic.add(ELEMENT_GENE[gift])
    setup_bender(c, gift, rng)
    tribe_parent = next((p for p in parents if getattr(p, "tribe", None)), None)
    if tribe_parent is not None:
        t = realm.tribes[tribe_parent.tribe]
        if len(t.members) < TRIBE_CAP + 3:
            join_tribe(realm, t, c)
    # the Concord's soul is reborn only in a time of need, into the next element of the cycle
    need = world_need(realm)
    if need and realm.concord is None and not realm.concord_waiting_child and gift == ELEMENTS[realm.concord_next]:
        c.concord = True
        realm.concord = c.id
        realm.concord_waiting_child = True
        realm.concord_need = need
        realm.concord_line.append({"id": c.id, "born": realm.year, "element": gift, "need": need,
                                   "revealed": None, "died": None, "deeds": []})


def world_need(realm):
    cal = [m for m in realm.monsters if m.alive and m.kind == "calamity"]
    if cal:
        return f"{cal[0].name} walks the world"
    for k in realm.kingdoms:
        r = realm.chars.get(k.ruler)
        if r is not None and r.alive and r.tyranny >= 60:
            return f"the tyranny of {r.name} {r.regnal}"
    if int(realm.war.sum() // 2) >= 4:
        return "war on every side"
    sov = [m for m in realm.monsters if m.alive and getattr(m, "title", None) == "Wyrm-Sovereign"]
    if sov and sov[0].power > 18000:
        return f"the shadow of {sov[0].name.split(' (')[0]}"
    return None


def bending_year(realm):
    rng = realm.rng
    y = realm.year
    for t in realm.tribes.values():
        t.members = {m for m in t.members if realm.chars[m].alive}
        if len(t.members) < TRIBE_CAP and rng.random() < 0.2:
            join_tribe(realm, t, new_bender(realm, t.element, None))
        if t.members:
            chief = max((realm.chars[m] for m in t.members), key=lambda c: bend_power(realm, c))
            if t.chief != chief.id:
                t.chief = chief.id
                t.chiefs.append((y, chief.id))
                chief.titles.append(f"Chief of {t.name} from year {y}")
        for m in list(t.members):
            c = realm.chars[m]
            if c.spouse is None and c.adult(y) and len(t.members) < TRIBE_CAP + 3 and rng.random() < 0.05:
                from .dynasty import marry
                mate = make_character(realm, c.realm, born=c.born + int(rng.integers(-5, 6)), female=not c.female)
                if rng.random() < 0.5:
                    mate.genetic.add(ELEMENT_GENE[t.element])
                    setup_bender(mate, t.element, rng)
                    join_tribe(realm, t, mate)
                marry(c, mate)
    for i in list(realm.living):
        c = realm.chars[i]
        if getattr(c, "element", None) is None:
            continue
        train(realm, c)
    concord_year(realm)


def train(realm, c):
    rng = realm.rng
    if not c.adult(realm.year) and c.age(realm.year) < 8:
        return
    for e in list(c.bend):
        before = c.bend[e]
        from .characters import power_gate
        top = {3: 49.9, 4: 74.9}.get(max(power_gate(c), power_gate(c, martial=True)), 100.0)   # Master and Grandmaster are rare
        c.bend[e] = min(top, before + rng.uniform(0.5, 2.5) * (1 - before / 110) * (1.5 if getattr(c, "concord", False)
                                                                                     else 1.0))
        tb, ta = bend_tier(before), bend_tier(c.bend[e])
        if ta > tb and ta >= 2:
            realm.say(f"{realm.who(c)} becomes a {BEND_TIER_NAMES[ta]} of {e}-bending, master of "
                      f"{BEND_FORMS[e][0][ta]}.", minor=not getattr(c, "concord", False))
            from .crafts import gain_fame
            gain_fame(realm, c, "bender", 30 * ta)
        if before < 70 <= c.bend[e] and c.high_art is None:
            c.high_art = str(rng.choice(BEND_FORMS[e][1]))
            realm.say(f"{realm.who(c)} opens the high art of {c.high_art}.", minor=True)
    from .characters import gain_xp
    gain_xp(c, 40 + 2 * c.level, realm)
    use(realm, c, "Alteration", 0.5)
    c.peak_cr = max(c.peak_cr, bend_power(realm, c))
    # benders in the tribes guard their homes from monsters
    if getattr(c, "tribe", None) and c.adult(realm.year) and rng.random() < 0.15:
        t = realm.tribes[c.tribe]
        ms = [m for m in realm.monsters if m.alive and m.kind not in ("calamity", "dragon") and not getattr(m, "sea", False)
              and math.hypot(m.pos[0] - t.pos[0], m.pos[1] - t.pos[1]) < 20]       # no one hunts a dragon alone
        if ms:
            m = ms[0]
            pw = bend_power(realm, c)
            if rng.random() < pw / (pw + 0.6 * m.power):
                m.power -= pw * 1.5
                if m.power <= 0 and m.alive:
                    m.alive = False
                    from .monsters import on_monster_slain
                    on_monster_slain(realm, m, c)
                    from .crafts import gain_fame
                    gain_fame(realm, c, "bender", 20 + m.base / 40)
                    if m.kind in ("dragon", "demon"):
                        realm.say(f"{realm.who(c)} of {t.name} brings down {m.name} with {BEND_FORMS[c.element][0][bend_tier(c.bend[c.element])]}.")
            else:
                c.hp -= rng.uniform(20, 70)
                if c.hp <= 0:
                    realm.kill(c, f"was slain by {m.name} defending {t.name}", killer=m)
                c.hp = max(c.hp, 20) if c.alive else c.hp


def concord_year(realm):
    rng = realm.rng
    y = realm.year
    if realm.concord is None:
        return
    c = realm.chars[realm.concord]
    rec = realm.concord_line[-1]
    if not c.alive:
        rec["died"] = (y, c.cause)
        realm.say(f"**{CONCORD} is dead.** {c.full_name()} {c.cause}. The soul passes on, to wait for the next "
                  f"age of need.")
        realm.concord = None
        realm.concord_waiting_child = False
        realm.concord_next = (ELEMENTS.index(c.element) + 1) % 4
        return
    if rec["revealed"] is None:
        if c.adult(y) or c.age(y) >= 16:
            rec["revealed"] = y
            c.epithet = c.epithet or "the Concord"
            realm.say(f"**{CONCORD} returns.** In a time of need ({rec['need']}), {realm.who(c)}, born a "
                      f"{c.element}-bender of the {PEOPLES[c.people]}, is revealed as {CONCORD}: the soul who can bend "
                      f"all four elements.")
            c.hero = True
            realm.hero_ids.add(c.id)
            realm.guilds[c.people].heroes_total += 1
        return
    # the Concord learns the next element each few years, in the cycle's order
    if len(c.bend) < 4 and rng.random() < 0.15:
        nxt = ELEMENTS[(ELEMENTS.index(c.element) + len(c.bend)) % 4]
        c.bend[nxt] = 20.0
        realm.say(f"{CONCORD}, {c.name}, learns {nxt}-bending from {realm.tribes[nxt].name}.", minor=True)
        if len(c.bend) == 4:
            realm.say(f"**{CONCORD} masters the fourth element.** {c.name} can bend tide, stone, flame and sky.")
    from .crafts import gain_fame
    pw = bend_power(realm, c)
    need = world_need(realm)
    if need is None:
        gain_fame(realm, c, "concord", 5)
        return
    cal = [m for m in realm.monsters if m.alive and m.kind == "calamity"]
    if cal and rng.random() < 0.4:
        m = cal[0]
        dmg = pw * rng.uniform(0.8, 1.6)
        m.power -= dmg
        rec["deeds"].append(f"year {y}: struck {m.name}")
        if m.power <= 0:
            m.alive = False
            realm.calamity_log.append((m.name, m.born, y, f"ended by {CONCORD}, {c.name}"))
            m.slain_by = c.id
            realm.say(f"**{CONCORD} ends {m.name}.** {c.name} enters the Concord State, all four elements at once, "
                      f"and the calamity is unmade.")
            gain_fame(realm, c, "concord", 800)
            c.renown += 1000
        else:
            c.hp -= rng.uniform(10, 60) * min(3.0, m.power / max(1.0, pw))
            if c.hp <= 0:
                realm.kill(c, f"fell in the Concord State against {m.name}", killer=m)
        return
    for k in realm.kingdoms:
        r = realm.chars.get(k.ruler)
        if r is not None and r.alive and r.tyranny >= 60 and rng.random() < 0.3:
            if rng.random() < pw / (pw + r.cr() * 3):
                realm.say(f"**{CONCORD} confronts {realm.who(r)}** and breaks the tyrant's power. The great of "
                          f"{k.name} turn on their ruler.")
                rec["deeds"].append(f"year {y}: brought down {r.name} {r.regnal}")
                realm.kill(r, f"was brought down by {CONCORD}", quiet=True)
                gain_fame(realm, c, "concord", 300)
            else:
                c.hp -= rng.uniform(20, 80)
                if c.hp <= 0:
                    realm.kill(c, f"fell confronting the tyrant {r.name} {r.regnal}")
            return
    if int(realm.war.sum() // 2) >= 3 and rng.random() < 0.3 and y - getattr(c, "last_peace", -99) >= 15:
        c.last_peace = y
        pairs = [(i, j) for i in range(realm.npol) for j in range(i + 1, realm.npol) if realm.war[i, j]]
        i, j = pairs[int(rng.integers(len(pairs)))]
        realm.war[i, j] = realm.war[j, i] = False
        realm.relation[i, j] = realm.relation[j, i] = 0
        realm.truce[i, j] = realm.truce[j, i] = y + 30
        realm.say(f"{CONCORD}, {c.name}, walks between the armies of the {realm.pname(i)} and the {realm.pname(j)}, "
                  f"and the war ends.")
        rec["deeds"].append(f"year {y}: ended the war of the {realm.pname(i)} and the {realm.pname(j)}")
        gain_fame(realm, c, "concord", 150)
    sov = []                                    # the Concord no longer hunts dragons alone: dragons are raids for a party
    if sov and rng.random() < 0.3:
        m = sov[0]
        if rng.random() < pw / (pw + 0.6 * m.power):
            m.power -= pw * 1.5
            if m.power <= 0:
                m.alive = False
                from .monsters import on_monster_slain
                on_monster_slain(realm, m, c)
                realm.say(f"**{CONCORD} slays {m.name}, the Wyrm-Sovereign.**")
                rec["deeds"].append(f"year {y}: slew the Wyrm-Sovereign {m.name}")
                gain_fame(realm, c, "concord", 400)
        else:
            c.hp -= rng.uniform(20, 70)
            if c.hp <= 0:
                realm.kill(c, f"was slain by the Wyrm-Sovereign {m.name}", killer=m)
