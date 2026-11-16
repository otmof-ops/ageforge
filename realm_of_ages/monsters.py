"""Monster life: monsters age and die, breed, and pass their hoards, relics and titles to their young.

Dragons live up to fifteen centuries, pair or (rarely) lay alone, and a clutch inherits its parents' traits.
The greatest dragon holds the title Wyrm-Sovereign, which passes to its eldest child. Troll bands that grow
strong split into daughter bands. Demons do not breed with their own kind, but sometimes with people, and that
is where abyss blood comes from. A monster that dies of age leaves its hoard to its eldest child, or its lair
becomes a ruin full of everything it ever took from the heroes it killed."""
import math

import numpy as np

from .data import ADULT, DRAGON_EPITHET, DRAGON_SYL, DRAGON_TRAITS, MONSTER, MONSTER_LIFE, PEOPLES, SINGULAR

SOVEREIGN = "Wyrm-Sovereign"


def mdist(a, b):
    return math.hypot(a[0] - b[0], a[1] - b[1])


def monster_year(realm):
    rng = realm.rng
    y = realm.year
    for m in list(realm.monsters):
        if not m.alive or m.kind == "calamity":
            continue
        age = y - m.born
        if m.kind != "dragon" and m.base < m.adult_base:             # the young grow into their strength (dragons: dragons.py)
            grow = 0.05 * (m.adult_base - m.base)
            m.base += grow
            m.power += grow
        if age > m.lifespan:
            die_of_age(realm, m)
            continue
        if m.kind == "dragon":
            breed_dragon(realm, m, age)
        elif m.kind == "troll" and age > 15 and m.power > 1.5 * m.base and rng.random() < 0.05:
            split_band(realm, m)
        elif m.kind == "demon" and rng.random() < 0.0004:
            demon_child(realm, m)
    crown_sovereign(realm)
    from .dragons import dragons_year
    dragons_year(realm)


def new_lineage_fields(m, rng, parents=()):
    life = MONSTER_LIFE.get(m.kind, 10 ** 6)
    m.lifespan = life * rng.uniform(0.7, 1.2)
    m.adult_base = m.base
    m.parents = [p.mid for p in parents]
    m.generation = 1 + max((p.generation for p in parents), default=0)
    m.children = []
    m.title = None
    m.kills = 0
    m.traits = set()
    if m.kind == "dragon":
        if parents:
            for p in parents:
                for t in p.traits:
                    if rng.random() < 0.6:
                        m.traits.add(t)
            if rng.random() < 0.1:
                m.traits.add(str(rng.choice(DRAGON_TRAITS)))
        else:
            m.traits.add(str(rng.choice(DRAGON_TRAITS[:4])))
            if rng.random() < 0.2:
                m.traits.add(str(rng.choice(DRAGON_TRAITS[4:])))
        if "elder" in m.traits:
            m.lifespan *= 1.5


def breed_dragon(realm, m, age):
    rng = realm.rng
    if age < 150 or m.base < 0.8 * m.adult_base:
        return
    mates = [o for o in realm.monsters if o.alive and o.kind == "dragon" and o is not m
             and (realm.year - o.born) >= 150 and mdist(o.pos, m.pos) <= 40]
    p = 0.02 if mates else 0.003
    if "brood-mother" in m.traits:
        p *= 2
    n_dragons = sum(1 for o in realm.monsters if o.alive and o.kind == "dragon")
    if n_dragons >= 7 or rng.random() > p:
        return
    mate = mates[int(rng.integers(len(mates)))] if mates else None
    parents = [m] + ([mate] if mate else [])
    clutch = int(rng.integers(1, 3))
    names = []
    for _ in range(clutch):
        from .geography import clip
        pos = clip(realm, (m.pos[0] + rng.integers(-5, 6), m.pos[1] + rng.integers(-5, 6)))
        adult = sum(p.adult_base for p in parents) / len(parents) * rng.uniform(0.85, 1.25)
        name = f"{realm.name(DRAGON_SYL)} {rng.choice(DRAGON_EPITHET)}"
        if any(o.alive and o.kind == "dragon" and o is not m and o.mid not in m.children
               and mdist(o.pos, pos) < 30 for o in realm.monsters if mate is None or o is not mate):
            continue                                                    # no range for another; the hatchling starves
        young = realm.new_monster("dragon", pos, adult * 0.25, name=name, parents=parents, quiet=True)
        young.adult_base = adult * (1.3 if "elder" in young.traits else 1.0)
        for p in parents:
            p.children.append(young.mid)
        names.append(young.name)
    if not names:
        return
    who = f"{m.name} and {mate.name}" if mate else m.name
    realm.say(f"A clutch hatches in the lair of {who}: {', '.join(names)}.")


def split_band(realm, m):
    rng = realm.rng
    from .geography import clip
    pos = clip(realm, (m.pos[0] + rng.integers(-6, 7), m.pos[1] + rng.integers(-6, 7)))
    m.power /= 2
    child = realm.new_monster("troll", pos, m.power, parents=[m], quiet=True)
    child.adult_base = m.adult_base
    m.children.append(child.mid)


def demon_child(realm, m):
    """A demon's child, born to a person of the nearest people. It is where abyss blood comes from."""
    from .characters import make_character
    rng = realm.rng
    towns = realm.alive_towns()
    if not towns:
        return
    t = min(towns, key=lambda s: mdist(s.pos, m.pos))
    if mdist(t.pos, m.pos) > 25:
        return
    f = t.faction
    kid = make_character(realm, f, born=realm.year)
    kid.home = t.id
    from .lineage import seed_gene
    seed_gene(realm, kid, "abyssblood")
    kid.personality.discard("kind")
    realm.say(f"A child is born in {t.name} with eyes like coals. The midwives will not say whose it is; "
              f"the {realm.pname(f)} whisper the name of {m.name.split(',')[0]}.")


def die_of_age(realm, m):
    rng = realm.rng
    m.alive = False
    kids = [o for o in realm.monsters if o.alive and o.mid in m.children]
    heir = max(kids, key=lambda o: o.power) if kids else None
    if m.kind in ("dragon", "demon") or m.hoard > 100 or m.items:
        tale = f"{m.name} {'die' if m.kind == 'troll' else 'dies'} of great age, {realm.year - m.born} years old"
        if heir is not None:
            heir.hoard += m.hoard
            heir.items.extend(m.items)
            tale += f"; its hoard passes to {heir.name}"
            if m.title:
                heir.title = m.title
                tale += f", and with it the title of {m.title}"
                realm.sovereign_line.append((realm.year, heir.name, f"inherited from {m.name}"))
        else:
            from .world import Ruin
            r = Ruin(f"the lair of {m.name.split(',')[0]}", m.pos, 2 + min(6, int(m.hoard / 300)), 2.5)
            r.loot = list(m.items)
            realm.ruins.append(r)
            tale += f"; its lair lies open now, {r.name}" + (f", with {len(m.items)} relics of the heroes it slew"
                                                            if m.items else "")
        realm.say(tale + ".", minor=m.kind not in ("dragon",))
    m.items, m.hoard = [], 0.0


def crown_sovereign(realm):
    dragons = [m for m in realm.monsters if m.alive and m.kind == "dragon"]
    if not dragons:
        return
    if any(d.title == SOVEREIGN for d in dragons):
        return
    best = max(dragons, key=lambda d: d.power)
    if best.power < 2500:
        return
    best.title = SOVEREIGN
    realm.sovereign_line.append((realm.year, best.name, "the greatest of the dragons"))
    realm.say(f"**{best.name} is the {SOVEREIGN}** now, eldest power among the dragons of the world.")


def on_monster_slain(realm, m, slayer=None):
    """A slain monster's title passes to its eldest living child; its relics go to whoever killed it.
    Its soul is caught in a soulstone for the enchanters, and a dragon's bones go to the smiths."""
    from .data2 import SOULSTONE
    if slayer is not None:
        m.slain_by = slayer.id
    people = slayer.realm if slayer is not None else None
    if people is None:
        near = [t for t in realm.alive_towns() if mdist(t.pos, m.pos) < 25]
        people = min(near, key=lambda t: mdist(t.pos, m.pos)).faction if near else None
    from .arcana import on_slain
    on_slain(realm, m, slayer, people)
    if m.kind == "dragon":
        from .dragons import freed
        freed(realm, m)
    if people is not None and m.kind in SOULSTONE:
        realm.kingdoms[people].soulstones[SOULSTONE[m.kind]] += 1
        if m.kind == "dragon":
            realm.kingdoms[people].dragonbone += 2
    kids = [o for o in realm.monsters if o.alive and o.mid in m.children]
    if m.title and kids:
        heir = max(kids, key=lambda o: o.power)
        heir.title = m.title
        realm.sovereign_line.append((realm.year, heir.name, f"inherited when {m.name} was slain"))
        realm.say(f"{heir.name} takes up the title of {m.title} from its slain parent, and remembers who killed it.")
    if slayer is not None and m.items:
        from .heroes import give_item
        best = max(m.items, key=lambda i: i.rarity)
        for it in m.items:
            give_item(realm, slayer, it, quiet=True)
        if best.rarity >= 3:
            realm.say(f"{realm.who(slayer)} takes back {best.label()} from the hoard of {m.name}.")
        m.items = []
