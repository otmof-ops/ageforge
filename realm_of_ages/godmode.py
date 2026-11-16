"""God mode: reach into the world. Every act is written in the chronicle as an act of the gods.

World acts: wake a calamity, loose a monster, smite a town, send plague, famine or a blessed harvest, grant
gold, declare war or impose peace, change a realm's government, raise a culture's era, start a golden age,
break a realm apart, curse or lift a curse from a house, name the Concord.

Characters: create one to your own design (name, people, sex, age, realm, class, level, every stat, every
trait, power bloodlines, skills, gold, hero or not), and change anything about any character afterwards. Any
character can be tracked: a tracked character keeps a journal of everything that happens to them."""
from .data import (ADJ, ADULT, ATTRS, BLOODLINES, CLASSES, GENETIC, PEOPLES, PERSONALITY, POWER_GENES, PRESTIGE)
from .data2 import SKILLS

OPPOSITE = {}
for _a, _b in PERSONALITY:
    OPPOSITE[_a], OPPOSITE[_b] = _b, _a
ALL_PERSONALITY = sorted(OPPOSITE)
ALL_GENETIC = sorted(GENETIC)
GOVERNMENTS = ["tribal", "clan", "feudal", "republic", "theocracy", "administrative"]
MONSTERS = ["troll", "wraith", "demon", "dragon"]


def god(realm, text):
    realm.say(f"**God.** {text}", cat="god")
    return text


def living_realms(realm):
    return [k for k in realm.kingdoms if not realm.extinct[k.id]]


# ---------------------------------------------------------------- tracking
def track(realm, c, on=True):
    realm.tracked = getattr(realm, "tracked", set())
    c.tracked = on
    if on:
        realm.tracked.add(c.id)
        c.journal = getattr(c, "journal", [])
        c.journal.append((realm.year, "god", f"Tracking begins: {c.full_name()}, aged {c.age(realm.year)}."))
    else:
        realm.tracked.discard(c.id)
    return f"{'Now tracking' if on else 'No longer tracking'} {c.full_name()}."


def snapshot(realm):
    """Every ten years, a tracked character's journal records where their life stands."""
    from .events import journal
    for cid in list(getattr(realm, "tracked", ())):
        c = realm.chars.get(cid)
        if c is None or not c.alive or realm.year % 10:
            continue
        k = realm.kingdoms[c.realm]
        journal(realm, c, f"Aged {c.age(realm.year)}, level {c.level}, renown {c.renown:,.0f}, gold {c.gold:,.0f}; "
                          f"stress {getattr(c, 'stress', 0):.0f}, happiness {getattr(c, 'happy', 0):+.0f}, "
                          f"health {getattr(c, 'health', 5):.1f}; of {k.name}"
                          + (f"; {c.titles[-1]}" if c.titles else ""), "lives")


# ---------------------------------------------------------------- characters
def create_character(realm, name, people, female, age, realm_id, cls=None, level=1, attrs=None, prowess=None,
                     talent=None, personality=None, genetic=None, latent=None, skills=None, gold=0, hero=False,
                     tracked=True, home=None):
    """A character to your own design, placed in the world and (by default) tracked."""
    from .characters import gain_xp, make_character, xp_needed
    from .lineage import apply_gene
    c = make_character(realm, realm_id, born=realm.year - int(age), female=female)
    if c.people != people:                                            # born of the chosen people, not the realm's
        from .data import LIFESPAN, PROWESS_BASE, TALENT_BASE
        c.people = people
        c.life = LIFESPAN[people] * realm.rng.uniform(0.9, 1.1)
        c.prowess = PROWESS_BASE[people] + realm.rng.normal(0, 3)
        c.talent = min(10.0, max(0.0, TALENT_BASE[people] + realm.rng.normal(0, 1.1)))
        for t in c.genetic:
            fx = GENETIC.get(t, {})
            c.life *= fx.get("life", 1.0)
            c.prowess += fx.get("prowess", 0)
    c.name = name
    c.custom = True
    if personality is not None:
        c.personality = set(personality)
    for g in (genetic or []):
        set_trait(realm, c, g, True)
    for g in (latent or []):
        if g in BLOODLINES:
            c.latent.add(g)
    if attrs:
        for a, v in attrs.items():
            c.attrs[a] = int(v)
    if prowess is not None:
        c.prowess = float(prowess)
    if talent is not None:
        c.talent = float(talent)
    if cls is not None and cls in CLASSES:
        c.classes = [[cls, 1]]
    from .skills import init_skills
    init_skills(c, realm.rng)
    if skills:
        for s, v in skills.items():
            c.skills[s] = float(v)
    while c.level < min(int(level), c.level_cap()):
        gain_xp(c, xp_needed(c.level) + 1, realm)
    c.gold = float(gold)
    towns = realm.alive_towns(realm_id)
    if home is not None:
        c.home = home
    elif towns:
        c.home = max(towns, key=lambda t: t.pop).id
    from .heart import init
    init(c, realm.rng)
    c.invalidate()
    if hero:
        from .heroes import start_hero
        start_hero(realm, c, None)
    if tracked:
        track(realm, c, True)
    god(realm, f"{c.full_name()}, a {ADJ[people]} {'woman' if female else 'man'} of {age}, is set down in "
               f"{realm.kingdoms[realm_id].name} by the gods' hand.")
    return c


def set_stat(realm, c, key, value):
    """Set any number about a person: a base stat, prowess, talent, level, a skill, the three meters, gold, renown."""
    from .characters import gain_xp, xp_needed
    v = float(value)
    if key in ATTRS:
        c.attrs[key] = int(v)
    elif key in ("prowess", "talent", "renown", "gold", "stress", "happy", "health", "life"):
        if key == "talent":
            v = min(10.0, max(0.0, v))
        setattr(c, key, v)
    elif key == "level":
        target = int(v)
        while c.level < min(target, c.level_cap()):
            gain_xp(c, xp_needed(c.level) + 1, realm)
        if target < c.level:
            c.level = max(1, target)
            c.classes[-1][1] = max(1, c.classes[-1][1] - (c.level - target))
    elif key in SKILLS:
        c.skills[key] = min(120.0, max(0.0, v))
    else:
        raise ValueError(f"unknown stat {key}")
    c.invalidate()
    god(realm, f"{c.full_name()}'s {key} is now {value}.")


def set_trait(realm, c, trait, on=True):
    """Give or take any trait: personality (its opposite goes), genetic, or a power bloodline."""
    from .lineage import apply_gene
    if trait in OPPOSITE:
        if on:
            c.personality.discard(OPPOSITE[trait])
            c.personality.add(trait)
        else:
            c.personality.discard(trait)
    elif trait in GENETIC:
        if on and trait not in c.genetic:
            c.genetic.add(trait)
            c.latent.discard(trait)
            apply_gene(c, trait)
        elif not on and trait in c.genetic:
            c.genetic.discard(trait)
            fx = GENETIC.get(trait, {})
            for a in ATTRS:
                c.attrs[a] -= fx.get(a, 0)
            c.prowess -= fx.get("prowess", 0)
            c.talent = max(0.0, c.talent - fx.get("talent", 0))
            c.life /= fx.get("life", 1.0)
    else:
        raise ValueError(f"unknown trait {trait}")
    c.invalidate()
    return god(realm, f"{c.full_name()} {'gains' if on else 'loses'} the trait {trait}.")


def set_class(realm, c, cls):
    if cls not in CLASSES and cls not in PRESTIGE:
        raise ValueError(f"unknown class {cls}")
    c.classes.append([cls, 1])
    c.invalidate()
    return god(realm, f"{c.full_name()} takes up the class {cls}.")


def make_hero(realm, c):
    from .heroes import start_hero
    if not c.hero:
        start_hero(realm, c, "is called to the road by a dream sent from above")
    return f"{c.full_name()} is a hero now."


def crown_character(realm, c):
    from .dynasty import crown, end_reign
    k = realm.kingdoms[c.realm]
    old = realm.chars.get(k.ruler)
    if old is not None and old.id != c.id:
        end_reign(realm, k, "set aside by the gods")
    crown(realm, k, c, "raised up when the line failed")
    return god(realm, f"The gods set {c.full_name()} on the throne of {k.name}.")


def marry_characters(realm, a, b):
    from .dynasty import marry
    from .heart import love, on_marriage
    for x in (a, b):
        if x.spouse is not None and realm.chars[x.spouse].alive:
            realm.chars[x.spouse].spouse = None
    marry(a, b)
    on_marriage(realm, a, b)
    return god(realm, f"{a.full_name()} and {b.full_name()} are wed by divine decree.")


def make_love(realm, a, b):
    from .heart import love
    love(a, b, "soulmate", 100)
    love(b, a, "soulmate", 100)
    return god(realm, f"{a.full_name()} and {b.full_name()} fall in love, as the gods intend.")


def give_item(realm, c, name, rarity=5):
    from .characters import Item
    from .heroes import give_item as give
    it = Item(name, int(rarity), realm.year, "the hands of the gods")
    it._bonus = [0.05, 0.1, 0.2, 0.35, 0.6, 1.0, 2.0][int(rarity)]
    give(realm, c, it, quiet=True)
    return god(realm, f"{c.full_name()} finds {it.label()} at their bedside one morning.")


def make_immortal(realm, c, on=True):
    c.undying_by_gods = on
    c.life = 10 ** 6 if on else c.life
    c.undead = on or c.undead
    return god(realm, f"{c.full_name()} {'will not die of age' if on else 'is mortal again'}.")


def make_custom(realm, c, on=True):
    """Make an existing character custom: the only kind of character who can go past the seventh tier of magic."""
    c.custom = on
    c.invalidate()
    c.__dict__.pop("_tier", None)
    return god(realm, f"{c.full_name()} is {'set apart by the gods' if on else 'an ordinary mortal again'}.")


def strike_down(realm, c):
    realm.kill(c, "was struck down by the gods")
    return god(realm, f"{c.full_name()} is struck down from the sky.")


def raise_dead(realm, c):
    if c.alive:
        return f"{c.full_name()} is not dead."
    c.alive, c.died, c.cause = True, None, None
    c.hp, c.health = 100.0, 6.0
    realm.living.add(c.id)
    return god(realm, f"{c.full_name()} walks out of the grave.")


# ---------------------------------------------------------------- the world
def wake_calamity(realm, lid, world=False):
    land = realm.terrain != 0
    cells = realm.remote_cells(land & (realm.landmass == lid))
    if not len(cells):
        return "No wild land there to wake anything in."
    r, c = cells[int(realm.rng.integers(len(cells)))]
    m = realm.new_monster("calamity", (int(r), int(c)), realm.rng.uniform(8000, 22000) * (2 if world else 1),
                          world=world)
    return god(realm, f"The gods wake {m.name} beneath {realm.lands[lid]['name']}.")


def loose_monster(realm, kind, kid):
    from .data import MONSTER
    cap = realm.capital_of(kid)
    if cap is None:
        return "That realm has no seat."
    pos = (cap.pos[0] + int(realm.rng.integers(-8, 9)), cap.pos[1] + int(realm.rng.integers(-8, 9)))
    from .geography import clip
    pos = clip(realm, pos)
    if realm.terrain[pos] == 0:
        pos = cap.pos
    m = realm.new_monster(kind, pos, realm.rng.uniform(*MONSTER[kind][1]) * 1.5)
    return god(realm, f"The gods loose {m.name} upon {realm.kingdoms[kid].name}.")


def smite_town(realm, town):
    realm.destroy(town)
    return god(realm, f"Fire falls on {town.name}; nothing is left of it.")


def send_plague(realm, kid, great=True):
    from .populace import Epidemic
    towns = realm.alive_towns(kid)
    if not towns:
        return "No towns there."
    src = max(towns, key=lambda t: t.pop)
    e = Epidemic(f"the Gods' {'Scourge' if great else 'Fever'}", realm.year, 0.5 if great else 0.15,
                 0.75 if great else 0.4, great)
    e.front = {src.id: 0.3}
    realm.epidemics.append(e)
    return god(realm, f"A plague is sent upon {realm.kingdoms[kid].name}, starting in {src.name}.")


def set_harvest(realm, kid, mult, years=3):
    realm.god_harvest = getattr(realm, "god_harvest", {})
    realm.god_harvest[kid] = (float(mult), realm.year + int(years))
    what = "a blessed harvest" if mult > 1 else "famine"
    return god(realm, f"The gods send {what} upon {realm.kingdoms[kid].name} for {years} years.")


def grant_gold(realm, kid, amount):
    from .events import book
    k = realm.kingdoms[kid]
    k.gold += float(amount)
    book(realm, kid, "divine gift", float(amount), "gold from the gods")
    return god(realm, f"{amount:,.0f} gold appears in the treasury of {k.name}.")


def declare_war(realm, a, b):
    from .statecraft import nearest_duchy, start_war
    ka, kb = realm.kingdoms[a], realm.kingdoms[b]
    dk = nearest_duchy(realm, ka, kb)
    start_war(realm, a, b, ("divine decree", {"duchy": dk}))
    return god(realm, f"The gods set {ka.name} against {kb.name}.")


def impose_peace(realm, a, b):
    from .statecraft import end_war, war_of
    w = war_of(realm, a, b)
    if w is not None:
        end_war(realm, (min(a, b), max(a, b)), w, "white")
    realm.war[a, b] = realm.war[b, a] = False
    realm.truce[a, b] = realm.truce[b, a] = realm.year + 30
    return god(realm, f"The gods impose peace between {realm.kingdoms[a].name} and {realm.kingdoms[b].name}.")


def change_government(realm, kid, gov):
    from .civics import change_gov
    change_gov(realm, realm.kingdoms[kid], gov, "By the will of the gods")
    return f"{realm.kingdoms[kid].name} is under {gov} rule."


def raise_era(realm, culture):
    from .civics import ERAS, INNOVATIONS
    cu = realm.cultures[culture]
    if cu.era >= len(ERAS) - 1:
        return "That people already stands at the last era."
    for n, _, _ in INNOVATIONS[cu.era]:
        cu.innovations.setdefault(n, realm.year)
    cu.era += 1
    return god(realm, f"The gods teach the {PEOPLES[culture]}; they enter the {ERAS[cu.era]} age.")


def golden_age(realm, culture):
    cu = realm.cultures[culture]
    cu.golden = realm.year + 40
    return god(realm, f"A golden age is granted to the {PEOPLES[culture]}.")


def break_realm(realm, kid, lid):
    """Tear a realm's holdings on one landmass away into a realm of their own."""
    from .titles import secede
    k = realm.kingdoms[kid]
    dks = {t.duchy for t in realm.alive_towns(kid) if t.landmass == lid and getattr(t, "duchy", None)}
    cap = realm.capital_of(kid)
    if cap is not None:
        dks.discard(cap.duchy)
    if not dks:
        return "Nothing there to break away."
    cands = [realm.chars[realm.duchies[d]["holder"]] for d in dks if realm.duchies[d]["holder"] in realm.chars]
    leader = max(cands, key=lambda c: c.renown) if cands else None
    if leader is None:
        from .characters import make_character
        leader = make_character(realm, kid, born=realm.year - 35)
    secede(realm, k, dks, leader, 0)
    return god(realm, f"The gods tear {realm.lands[lid]['name']} from {k.name}.")


def curse_house(realm, hid):
    from .legends import curse_house as curse
    curse(realm, hid, "the gods", "divine displeasure")
    return f"{realm.houses[hid].name} is cursed."


def name_concord(realm, c):
    from .bending import setup_bender
    from .data2 import ELEMENTS
    if c.element is None:
        setup_bender(c, ELEMENTS[0], realm.rng)
    c.concord = True
    realm.concord = c.id
    realm.concord_line.append({"id": c.id, "born": c.born, "element": c.element, "need": "the gods' choosing",
                               "revealed": None, "died": None, "deeds": []})
    return god(realm, f"The gods name {c.full_name()} the Concord.")
