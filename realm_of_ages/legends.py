"""Legends and curses, born from what actually happens.

When a great power dies with spiritual force behind it (a calamity, a Wyrm-Sovereign, a lich, a tyrant, a
betrayed hero), it may lay a curse: on a bloodline, on the land, on a weapon, on a hunter's line, or the
greatest of them, a curse of return. A returning one swears to come back in every age on its continent to
claim a relic it was denied. Each time, a hero bearing the Hero's Mark and a royal child with the Sealing Light
are born to answer it, and only the weapon sworn against it can seal it again. If the hero fails, the
returned one takes the relic and the throne, and the land around it withers until a hero comes.

Legendary weapons are forged from legendary matter (the heartstone of a slain calamity, star-iron from a
falling star, the bones of a Wyrm-Sovereign) by the greatest smiths, and a weapon made from a cursed one's
heart is sworn against it."""
import math

import numpy as np

from .characters import Item, make_character
from .data import ADULT, LEGACY, OCEAN, PEOPLES, SWAMP, DESERT
from .data2 import CURSES, CYCLE_TITLES, HERO_MARK, LEGEND_MATERIALS, OATH_A, SEER_GIFT
from .geography import land_name
from .skills import skill


def d(a, b):
    return math.hypot(a[0] - b[0], a[1] - b[1])


class Cycle:
    def __init__(self, cid, villain, title, landmass, year, power):
        self.id, self.villain, self.title, self.landmass = cid, villain, title, landmass
        self.born, self.power = year, power
        self.period = 0
        self.next_return = 0
        self.relic = None
        self.blade = None
        self.hero = self.seer = None
        self.state = "sealed"
        self.monster = None
        self.ruler = None
        self.ages = []
        self.age = {}


def myth(realm, title, text):
    realm.myths.append((realm.year, title, text))


# ---------------------------------------------------------------- forming curses
def on_death(realm, c, cause, killer):
    """Called when anyone dies. The great may curse with their last breath."""
    rng = realm.rng
    spirit = max(0, c.tier()) / 10 + min(1.0, c.renown / 2000) + 0.3 * c.undead
    k = realm.kingdoms[c.realm]
    tyrant = c.tyranny >= 60 and c.ruled is not None
    # a great ruler struck down by violence: the fate of a land, decided by a blade
    struck = c.ruled is not None and c.renown >= 300 and not any(w in cause for w in ("old age", "illness",
                                                                                      "childhood"))
    betrayed = c.hero and c.renown >= 400 and ("purges" in cause or "executed" in cause or "murdered" in cause)
    lich = c.undead
    if not (tyrant or betrayed or lich or struck) or rng.random() > 0.15 + 0.35 * spirit:
        return
    # who did it, and whose line pays
    culprit_house = None
    if betrayed and k.ruler is not None:
        culprit_house = realm.chars[k.ruler].house
    elif killer is not None and hasattr(killer, "house"):
        culprit_house = killer.house
    roll = rng.random()
    if (tyrant or lich) and roll < 0.35:
        home = realm.capital_of(c.realm)
        if home is not None:
            make_cycle(realm, f"{c.name} {c.regnal or ''}".strip(), home.landmass, 9000 + 3000 * spirit, None,
                       f"the tyrant {c.full_name()}, dying, swears to return")
            return
    if c.items and roll < 0.55:
        it = max(c.items, key=lambda i: i.rarity)
        it.cursed = True
        it.curse_of = c.full_name()
        realm.curses.append({"kind": "item", "item": it.name, "source": c.full_name(), "since": realm.year,
                             "lifted": None})
        myth(realm, f"The curse on {it.name}", f"{c.full_name()} poured their last breath into {it.name}. It "
             f"gives strength, and takes the mind.")
        realm.say(f"**A curse.** As {c.full_name()} dies, {it.name} drinks their last breath. Whoever bears it now "
                  f"will be strong, and not themselves.")
        return
    if culprit_house is not None and culprit_house in realm.houses:
        curse_house(realm, culprit_house, c.full_name(), "betrayal" if betrayed else "a dying curse")
    else:
        home = realm.by_id.get(c.home) or realm.capital_of(c.realm)
        if home is not None:
            curse_land(realm, home.pos, home.landmass, c.full_name(), 10 + 10 * spirit)


def curse_house(realm, hid, source, why):
    h = realm.houses[hid]
    if getattr(h, "curse", None):
        return
    h.curse = {"source": source, "since": realm.year, "why": why}
    realm.curses.append({"kind": "blood", "house": h.name, "source": source, "since": realm.year, "lifted": None})
    myth(realm, f"The curse of {h.name}", f"{source} cursed {h.name} for {why}: its children sicken, its lords go "
         f"mad, its heirs die young.")
    realm.say(f"**A blood curse.** {source} curses {h.name} for {why}. Its children will sicken and its lords go mad, "
              f"until the curse is lifted.")


def curse_land(realm, pos, lid, source, radius):
    realm.curses.append({"kind": "land", "pos": pos, "radius": radius, "land": lid, "source": source,
                         "since": realm.year, "lifted": None})
    myth(realm, f"The withering of {land_name(realm, lid)}", f"Where {source} fell, the land itself was cursed.")
    realm.say(f"**The land is cursed.** Where {source} fell on {land_name(realm, lid)}, the soil withers for "
              f"{int(radius)} leagues around, and the dead do not lie easy.")


def make_cycle(realm, villain, lid, power, sworn_item, how):
    rng = realm.rng
    cy = Cycle(len(realm.cycles), villain, str(rng.choice(CYCLE_TITLES)), lid, realm.year, power)
    cy.period = int(rng.integers(150, 350))
    cy.next_return = realm.year + cy.period
    cy.relic = make_relic(realm, lid)
    if sworn_item is not None:
        swear(realm, sworn_item, cy)
    realm.cycles.append(cy)
    realm.curses.append({"kind": "cycle", "cycle": cy.id, "source": villain, "since": realm.year, "lifted": None})
    myth(realm, f"The curse of {villain}, {cy.title}",
         f"{how}: in every age it will return to {land_name(realm, lid)} to take {cy.relic.name}. "
         + (f"Only {cy.blade.name} can seal it." if cy.blade else "No blade is yet sworn against it."))
    realm.say(f"**A curse of return.** {how}: in every age they will come back to {land_name(realm, lid)} to take "
              f"{cy.relic.name}, and only a sworn blade can seal them again. The sages reckon the next age "
              f"begins in year {cy.next_return}.")
    return cy


def on_slumber(realm, m):
    """A calamity that goes back into the deep unbeaten may swear to return in every age."""
    rng = realm.rng
    if rng.random() > 0.5 or getattr(m, "cycle", None) is not None:
        return
    name = m.name.split(",")[0]
    cy = make_cycle(realm, name, m.landmass, m.base * 0.6, None,
                    f"{m.name}, sinking unbeaten into the deep, swears to come back for what it was denied")
    realm.legend_materials.append({"kind": "heartstone",
                                   "name": f"a shard of {name}'s heart, left in the earth where it sank",
                                   "land": m.landmass, "year": realm.year, "cycle": cy.id, "used": False})


def make_relic(realm, lid):
    name = f"the Heart of {land_name(realm, lid)}"
    it = Item(name, 6, realm.year, f"the soul of {land_name(realm, lid)}", special="tier",
              note="the relic the returning one seeks")
    it.relic_of = lid
    keep = keeper_town(realm, lid)
    if keep is not None:
        realm.kingdoms[keep.faction].treasury.append(it)
        it.kept_at = keep.id
    return it


def swear(realm, it, cy):
    if not it.name.startswith(tuple(OATH_A)):
        it.old_name = it.name
    it.name = f"{realm.rng.choice(OATH_A)}, Sworn Against {cy.villain}"
    it.rarity = max(it.rarity, 5)
    it._bonus = max(it.bonus, 1.0)
    it.oath = cy.id
    cy.blade = it
    realm.say(f"{getattr(it, 'old_name', 'A blade')} is sworn against {cy.villain}; it is called {it.name} now.")


def keeper_town(realm, lid):
    towns = [t for t in realm.alive_towns() if t.landmass == lid]
    if not towns:
        return None
    counts = {}
    for t in towns:
        counts[t.faction] = counts.get(t.faction, 0) + 1
    f = max(counts, key=counts.get)
    cap = realm.capital_of(f)
    if cap is not None and cap.landmass == lid:
        return cap
    return max((t for t in towns if t.faction == f), key=lambda t: t.pop)


# ---------------------------------------------------------------- each year
def legends_year(realm):
    rng = realm.rng
    y = realm.year
    # the deaths of great powers this year
    for mid, m in list(realm.watch.items()):
        if m.alive:
            continue
        del realm.watch[mid]
        if getattr(m, "cycle", None) is not None:
            continue
        slain = not any(e[0] == m.name and "slumber" in e[3] for e in realm.calamity_log)
        if m.kind == "calamity" and slain:
            slayer = realm.chars.get(getattr(m, "slain_by", None))
            realm.legend_materials.append({"kind": "heartstone", "name": LEGEND_MATERIALS["heartstone"].format(m.name),
                                           "land": m.landmass, "year": y, "cycle": None, "used": False})
            if rng.random() < 0.75:
                roll = rng.random()
                if roll < 0.5:
                    sworn = max(slayer.items, key=lambda i: i.rarity) if slayer is not None and slayer.items else None
                    cy = make_cycle(realm, m.name.split(",")[0], m.landmass, m.base * 0.8, sworn,
                                    f"{m.name}, dying, swears an oath on its own heart")
                    realm.legend_materials[-1]["cycle"] = cy.id
                elif roll < 0.7 or slayer is None or slayer.house is None:
                    curse_land(realm, m.pos, m.landmass, m.name, 14)
                else:
                    curse_house(realm, slayer.house, m.name, f"the slaying of {m.name}")
        elif m.kind == "dragon" and getattr(m, "title", None) == "Wyrm-Sovereign" and slain:
            realm.legend_materials.append({"kind": "sovereign-bone",
                                           "name": LEGEND_MATERIALS["sovereign-bone"].format(m.name.split(" (")[0]),
                                           "land": m.landmass, "year": y, "cycle": None, "used": False})
            slayer = realm.chars.get(getattr(m, "slain_by", None))
            if slayer is not None and slayer.house is not None and rng.random() < 0.3:
                curse_house(realm, slayer.house, m.name.split(" (")[0], "the death of the Wyrm-Sovereign")
    for m in realm.monsters:
        if m.alive and (m.kind == "calamity" or getattr(m, "title", None)):
            realm.watch[m.mid] = m
    # a star falls
    conts = [L for L in realm.lands if L["kind"] == "continent"]
    if conts and rng.random() < 0.004:
        L = conts[int(rng.integers(len(conts)))]
        realm.legend_materials.append({"kind": "star-iron", "name": LEGEND_MATERIALS["star-iron"].format(L["name"]),
                                       "land": L["id"], "year": y, "cycle": None, "used": False})
        realm.say(f"A star falls burning over {L['name']}; the smiths of the world want what it left behind.")
    forge_legends(realm)
    curse_effects(realm)
    for cy in realm.cycles:
        cycle_turn(realm, cy)


def forge_legends(realm):
    rng = realm.rng
    for mat in realm.legend_materials:
        if mat["used"]:
            continue
        peoples = {t.faction for t in realm.alive_towns() if t.landmass == mat["land"]}
        smiths = [realm.chars[i] for i in realm.living if realm.chars[i].profession == "smith"
                  and realm.chars[i].realm in peoples and skill(realm, realm.chars[i], "Smithing") >= 85]
        if not smiths or rng.random() > 0.2:
            continue
        sm = max(smiths, key=lambda s: skill(realm, s, "Smithing"))
        mat["used"] = True
        name = f"{rng.choice(OATH_A)}"
        while name in realm.item_names:
            name = f"{rng.choice(OATH_A)} of {realm.name((['Ar', 'Bel', 'Cor', 'Dra', 'Eth'], ['iel', 'on', 'ath', 'ys']))}"
        realm.item_names.add(name)
        it = Item(name, 5, realm.year, f"forged by {sm.full_name()} from "
                  f"{mat['name']}", special="vs_monster", note=f"forged from {mat['name']}")
        it._bonus = rng.uniform(1.2, 2.0)
        it.maker = sm.id
        it.legendary = True
        realm.legend_items.append(it)
        from .crafts import gain_fame
        gain_fame(realm, sm, "Smithing", 600)
        text = f"{realm.who(sm)} forges {it.name} from {mat['name']}"
        if mat["cycle"] is not None:
            cy = realm.cycles[mat["cycle"]]
            if cy.blade is None:
                swear(realm, it, cy)
                text += f", the one blade that can seal {cy.villain}"
        myth(realm, f"The forging of {it.name}", text + ".")
        realm.say(f"**A legendary forging.** {text}.")
        k = realm.kingdoms[sm.realm]
        heroes = [h for h in realm.heroes() if h.people == sm.people]
        if heroes:
            from .heroes import give_item
            h = max(heroes, key=lambda h: h.renown)
            give_item(realm, h, it, quiet=True)
            realm.say(f"{k.name} gives {it.name} to {realm.who(h)}.", minor=True)
        else:
            k.treasury.append(it)


def curse_effects(realm):
    rng = realm.rng
    y = realm.year
    for s in realm.alive_towns():
        s.curse_mult = 1.0
    for cu in realm.curses:
        if cu["lifted"] is not None:
            continue
        if cu["kind"] == "land":
            for s in realm.alive_towns():
                if s.landmass == cu["land"] and d(s.pos, cu["pos"]) <= cu["radius"]:
                    s.curse_mult = min(s.curse_mult, 0.55)
            if rng.random() < 0.05:
                r, c = cu["pos"]
                rr = int(np.clip(r + rng.integers(-int(cu["radius"]), int(cu["radius"]) + 1), 0, realm.H - 1))
                cc = int(np.clip(c + rng.integers(-int(cu["radius"]), int(cu["radius"]) + 1), 0, realm.W - 1))
                if realm.terrain[rr, cc] != OCEAN:
                    realm.corrupted[rr, cc] = True
            lifter = lifter_for(realm, cu)
            if lifter is not None or y - cu["since"] > 300:
                lift(realm, cu, lifter)
        elif cu["kind"] == "blood":
            h = next((h for h in realm.houses.values() if h.name == cu["house"]), None)
            if h is None:
                continue
            for i in list(realm.living):
                c = realm.chars[i]
                if c.house == h.id:
                    if rng.random() < 0.015:
                        realm.kill(c, f"died of the curse of {h.name}")
                    elif rng.random() < 0.01 and "wrathful" not in c.personality:
                        c.personality.add("wrathful")
                        c.personality.discard("calm")
                    if c.legitimacy is not None:
                        c.legitimacy = max(0.0, c.legitimacy - 0.3)
            lifter = lifter_for(realm, cu)
            if lifter is not None or y - cu["since"] > 200:
                lift(realm, cu, lifter)
                h.curse = None
        elif cu["kind"] == "item":
            for i in realm.living:
                c = realm.chars[i]
                if any(it.name == cu["item"] for it in c.items):
                    c.tyranny = min(100.0, c.tyranny + 1.0)
                    if "wrathful" not in c.personality and rng.random() < 0.1:
                        c.personality.add("wrathful")
                        realm.say(f"{realm.who(c)} grows wrathful; {cu['item']} is in their dreams.", minor=True)


def lifter_for(realm, cu):
    """A saint, a master healer, an archdruid or the Concord can lift a curse, if they come near it."""
    rng = realm.rng
    if rng.random() > 0.02:
        return None
    cands = []
    for i in realm.living:
        c = realm.chars[i]
        if c.concord or any(cls in ("Saint", "Archdruid") for cls, _ in c.classes) or \
                skill(realm, c, "Restoration") >= 95:
            cands.append(c)
    return cands[int(rng.integers(len(cands)))] if cands else None


def lift(realm, cu, by):
    cu["lifted"] = realm.year
    what = {"land": f"the curse on the land where {cu['source']} fell", "blood": f"the curse on {cu.get('house')}",
            "item": f"the curse on {cu.get('item')}"}.get(cu["kind"], "the curse")
    if by is not None:
        realm.say(f"**A curse is lifted.** {realm.who(by)} lifts {what}.")
        from .crafts import gain_fame
        gain_fame(realm, by, "prophet", 200)
        myth(realm, f"The lifting of {what}", f"{by.full_name()} lifted {what} in year {realm.year}.")
    else:
        realm.say(f"{what.capitalize()} fades at last, after {realm.year - cu['since']} years.", minor=True)


# ---------------------------------------------------------------- the cycle of return
def on_birth(realm, kid):
    """In the years before a return, the hero and the seer are born again."""
    for cy in realm.cycles:
        if realm.year < cy.next_return - 45 and cy.state == "sealed":
            continue
        home = realm.capital_of(kid.realm)
        lands = {t.landmass for t in realm.alive_towns(kid.realm)}
        if cy.landmass not in lands:
            continue
        if cy.hero is None:
            cy.hero = kid.id
            kid.marks = getattr(kid, "marks", []) + [HERO_MARK]
            realm.say(f"A child, {kid.name}, is born on {land_name(realm, cy.landmass)} with {HERO_MARK} on their "
                      f"hand. The old songs of {cy.villain} are sung again.", minor=True)
            cy.age["hero"] = kid.id
        elif cy.seer is None:
            k = realm.kingdoms[kid.realm]
            if kid.house is not None and kid.house == k.house:
                cy.seer = kid.id
                kid.marks = getattr(kid, "marks", []) + [SEER_GIFT]
                realm.say(f"A royal child, {kid.name} of {realm.houses[kid.house].name}, is born with "
                          f"{SEER_GIFT}.", minor=True)
                cy.age["seer"] = kid.id


def cycle_turn(realm, cy):
    rng = realm.rng
    y = realm.year
    hero = realm.chars.get(cy.hero)
    seer = realm.chars.get(cy.seer)
    if hero is not None and not hero.alive:
        cy.age.setdefault("fallen", []).append(hero.full_name())
        cy.hero = None
        hero = None
    if seer is not None and not seer.alive:
        cy.seer = None
        seer = None
    if cy.state == "sealed":
        if y >= cy.next_return:
            ret(realm, cy)
        return
    if cy.state == "returned":
        m = cy.monster
        if m is None or not m.alive:
            if m is not None and getattr(m, "sealed", False) is False:
                # struck down without the sworn blade: scattered, not sealed
                cy.state = "sealed"
                cy.next_return = y + cy.period // 2
                cy.ages.append(dict(cy.age, end=y, outcome="scattered by other hands, not sealed; it will be back sooner"))
                cy.age = {}
                realm.say(f"{cy.villain} is struck down, but not by the sworn blade. It is only scattered; the sages "
                          f"say it will be back within {cy.period // 2} years.")
            return
        relic_town = realm.by_id.get(getattr(cy.relic, "kept_at", None))
        if relic_town is None or not relic_town.alive:
            relic_town = keeper_town(realm, cy.landmass)
            if relic_town is not None:
                cy.relic.kept_at = relic_town.id
        if relic_town is not None:                                 # it marches on the relic
            dy, dx = relic_town.pos[0] - m.pos[0], relic_town.pos[1] - m.pos[1]
            dist = math.hypot(dy, dx)
            if dist > 3:
                step = min(6.0, dist)
                np_ = (int(m.pos[0] + dy / dist * step), int(m.pos[1] + dx / dist * step))
                if realm.terrain[np_] != OCEAN:
                    m.pos = np_
            else:
                realm.assault(m, relic_town, 0.3)
                if relic_town.pop < 200 or rng.random() < 0.15:
                    fall(realm, cy, relic_town)
                    return
        hero_strikes(realm, cy, hero, seer)
        return
    if cy.state == "triumphant":
        spread_dark(realm, cy)
        hero_strikes(realm, cy, hero, seer)


def ret(realm, cy):
    rng = realm.rng
    cells = realm.remote_cells((realm.terrain != OCEAN) & (realm.landmass == cy.landmass))
    if not len(cells):
        cy.next_return = realm.year + 50
        return
    r, c = cells[int(rng.integers(len(cells)))]
    n = len(cy.ages) + 1
    cy.power *= 1.25
    m = realm.new_monster("calamity", (int(r), int(c)), cy.power, name=f"{cy.villain}, {cy.title}", quiet=True)
    m.cycle = cy.id
    m.sealed = False
    cy.monster = m
    cy.state = "returned"
    cy.age = dict(cy.age, n=n, start=realm.year)
    hero = realm.chars.get(cy.hero)
    seer = realm.chars.get(cy.seer)
    realm.say(f"**{cy.villain} returns** to {land_name(realm, cy.landmass)}, {cy.title}, in the {ordinal(n)} age of "
              f"the curse. It seeks {cy.relic.name}."
              + (f" {realm.who(hero)} bears {HERO_MARK}." if hero else " No one bearing the Hero's Mark has been born.")
              + (f" {realm.who(seer)} carries {SEER_GIFT}." if seer else ""))
    myth(realm, f"The {ordinal(n)} return of {cy.villain}", f"In year {realm.year} {cy.villain} came back to "
         f"{land_name(realm, cy.landmass)}.")


def ordinal(n):
    return {1: "first", 2: "second", 3: "third", 4: "fourth", 5: "fifth", 6: "sixth"}.get(n, f"{n}th")


def hero_strikes(realm, cy, hero, seer):
    """The marked hero must find the sworn blade, then face the returned one; the seer's light weakens it."""
    rng = realm.rng
    y = realm.year
    if hero is None:
        return
    if not hero.adult(y) and hero.age(y) < 14:
        return
    if not hero.hero:
        from .heroes import start_hero
        start_hero(realm, hero, f"takes up the road, {HERO_MARK} burning on their hand")
    blade = cy.blade
    if blade is None:
        return
    if blade not in hero.items:
        if rng.random() > 0.35:
            return
        where = take_blade(realm, blade)
        from .heroes import give_item
        give_item(realm, hero, blade, quiet=True)
        realm.say(f"{realm.who(hero)} draws {blade.name}{where}. It knows its bearer.")
        cy.age["blade_drawn"] = y
        return
    foe_power = cy.monster.power if cy.state == "returned" else cy.ruler_power
    hp = hero.cr() * 3 * (1.6 if seer is not None and seer.alive and seer.age(y) >= 14 else 1.0)
    if rng.random() > 0.4:
        return
    if rng.random() < hp / (hp + 0.6 * foe_power):
        seal(realm, cy, hero, seer)
    else:
        hero.hp -= rng.uniform(40, 120)
        if hero.hp <= 0:
            realm.say(f"{realm.who(hero)} falls before {cy.villain}. {HERO_MARK} fades from their hand; it will "
                      f"be born again.")
            realm.kill(hero, f"fell before {cy.villain}, {cy.title}")
            if blade in hero.items:
                hero.items.remove(blade)
            keep = keeper_town(realm, cy.landmass)
            if keep is not None:
                realm.kingdoms[keep.faction].treasury.append(blade)
        else:
            hero.hp = max(hero.hp, 10)


def take_blade(realm, blade):
    for k in realm.kingdoms:
        if blade in k.treasury:
            k.treasury.remove(blade)
            return f" from the treasury of {k.name}"
        if blade in k.market:
            k.market.remove(blade)
            return f" from the market of {k.name}"
    for i in realm.living:
        c = realm.chars[i]
        if blade in c.items:
            c.items.remove(blade)
            c.invalidate()
            return f", given up by {c.full_name()}"
    for r in realm.ruins:
        if blade in r.loot:
            r.loot.remove(blade)
            return f" from {r.name}"
    for m in realm.monsters:
        if blade in m.items:
            m.items.remove(blade)
            return f" out of the hoard of {m.name}"
    return ""


def seal(realm, cy, hero, seer):
    y = realm.year
    was = cy.state
    if cy.state == "returned" and cy.monster is not None:
        cy.monster.sealed = True
        cy.monster.alive = False
    elif cy.state == "triumphant":
        restore(realm, cy, seer)
    cy.state = "sealed"
    cy.next_return = y + cy.period
    deed = "seals" if was == "returned" else "casts down"
    realm.say(f"**{realm.who(hero)} {deed} {cy.villain}** with {cy.blade.name}"
              + (f", {realm.who(seer)}'s {SEER_GIFT} holding it still" if seer is not None and seer.alive else "")
              + f". The {ordinal(len(cy.ages) + 1)} age of the curse is over; the sages say it will return in "
              f"year {cy.next_return}.")
    from .crafts import gain_fame
    gain_fame(realm, hero, "concord", 400)
    hero.renown += 800
    hero.deeds["calamity"] = hero.deeds.get("calamity", 0) + 1
    if seer is not None and seer.alive:
        seer.renown += 300
        if seer.legitimacy is not None:
            seer.legitimacy = min(100.0, seer.legitimacy + 20)
    cy.ages.append(dict(cy.age, end=y, outcome=f"sealed by {hero.full_name()}"
                        + (f" with {seer.full_name()}" if seer is not None else "")))
    myth(realm, f"The sealing of {cy.villain}, age {len(cy.ages)}",
         f"{hero.full_name()} {deed} {cy.villain} with {cy.blade.name} in year {y}.")
    cy.age = {}
    cy.hero = cy.seer = None


def fall(realm, cy, town):
    """The hero failed: the returned one takes the relic and the throne, and the land begins to wither."""
    from .dynasty import crown, end_reign
    y = realm.year
    f = town.faction
    k = realm.kingdoms[f]
    m = cy.monster
    m.alive = False
    m.sealed = True
    if cy.relic in k.treasury:
        k.treasury.remove(cy.relic)
    villain = make_character(realm, f, born=y - 300)
    villain.name = cy.villain.split()[0]
    villain.undead = True
    villain.tyranny = 100.0
    villain.personality = {"cruel", "ambitious", "wrathful"}
    villain.items.append(cy.relic)
    old = realm.chars.get(k.ruler)
    if old is not None and old.alive:
        realm.kill(old, f"was cast down by {cy.villain}", quiet=True)
        end_reign(realm, k, f"cast down by {cy.villain}")
    elif k.ruler is not None:
        end_reign(realm, k, f"cast down by {cy.villain}")
    k.old_name = k.name
    k.name = f"the Dominion of {cy.villain}"
    crown(realm, k, villain, "seizing the throne from the cradle")
    villain.legitimacy = 5.0
    cy.ruler = villain.id
    cy.ruler_power = m.power
    cy.state = "triumphant"
    cy.dark_center = town.pos
    cy.dark_radius = 4.0
    cy.age["fell"] = (y, town.name)
    realm.say(f"**The Fall.** {cy.villain} takes {cy.relic.name} at {town.name} and sits the throne of "
              f"{k.old_name}. It is {k.name} now, and the land around it begins to die.")
    myth(realm, f"The Fall of {k.old_name}", f"In year {y} {cy.villain} took {cy.relic.name} and the throne.")


def spread_dark(realm, cy):
    rng = realm.rng
    cy.dark_radius = min(40.0, cy.dark_radius + 0.7)
    r0, c0 = cy.dark_center
    R = int(cy.dark_radius)
    rlo, rhi, clo, chi = max(0, r0 - R), min(realm.H, r0 + R + 1), max(0, c0 - R), min(realm.W, c0 + R + 1)
    yy, xx = np.ogrid[rlo:rhi, clo:chi]
    disk = np.hypot(yy - r0, xx - c0) <= cy.dark_radius
    land = realm.terrain[rlo:rhi, clo:chi] != OCEAN
    sub = realm.corrupted[rlo:rhi, clo:chi]
    sub |= disk & land & (rng.random(disk.shape) < 0.08)
    t = realm.terrain[rlo:rhi, clo:chi]
    wither = sub & land & (rng.random(disk.shape) < 0.02)
    t[wither] = np.where(rng.random(int(wither.sum())) < 0.5, SWAMP, DESERT)
    for s in realm.alive_towns():
        if d(s.pos, cy.dark_center) <= cy.dark_radius:
            s.curse_mult = min(getattr(s, "curse_mult", 1.0), 0.5)
            s.pop *= 0.985
    if rng.random() < 0.1:
        kind = str(rng.choice(["wraith", "demon", "troll"]))
        from .data import MONSTER
        realm.new_monster(kind, cy.dark_center, rng.uniform(*MONSTER[kind][1]) * 1.5)


def restore(realm, cy, seer):
    from .dynasty import crown, end_reign, primo_heir
    k = next((k for k in realm.kingdoms if k.ruler == cy.ruler), None)
    v = realm.chars.get(cy.ruler)
    if v is not None and v.alive:
        realm.kill(v, f"was cast down by {HERO_MARK}", quiet=True)
    if k is None:
        return
    end_reign(realm, k, "cast down by the hero of the age")
    k.name = getattr(k, "old_name", k.name)
    heir = seer if seer is not None and seer.alive and seer.realm == k.id else None
    if heir is None:
        from .dynasty import pool, claim_score
        cands = pool(realm, k)
        heir = max(cands, key=lambda c: claim_score(realm, c)) if cands else None
    if heir is not None:
        crown(realm, k, heir, "restored when the dark was cast down")
    cy.ruler = None
    realm.say(f"{k.name} is free again.")
