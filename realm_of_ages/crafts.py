"""Specialists, their work, gold and fame: smiths forge weapons and armour in seven material tiers, enchanters
bind effects in five tiers (the high tiers need soulstones from slain monsters), alchemists brew, merchants
trade. Everything made goes to a people's market, where heroes and nobles buy with gold. Fame in any
specialty becomes renown, and legacy, so anyone can become famous for being very good at something."""
from .characters import Item, first_name, make_character
from .data import ADULT, ITEM_A, ITEM_B, PEOPLES
from .data2 import (ARMORS, ENCHANT_ARMOR, ENCHANT_TIERS, ENCHANT_WEAPON, MATERIALS, PROFESSIONS, QUALITY,
                    WEAPONS)
from .skills import perk_count, skill, use

SKILL_LEGACY = {"Smithing": "smith", "Enchanting": "enchanter", "Alchemy": "alchemist", "Mercantile": "merchant",
                "Speech": "prophet", "Sneak": "thief", "Security": "thief", "Pickpocket": "thief",
                "Blade": "warrior", "Blunt": "warrior", "Archery": "warrior", "Block": "guardian",
                "Heavy Armor": "guardian", "Light Armor": "warrior", "Destruction": "arcane",
                "Restoration": "arcane", "Conjuration": "arcane", "Illusion": "arcane", "Alteration": "arcane"}
FAME_WORD = {"Smithing": "smith", "Enchanting": "enchanter", "Alchemy": "alchemist", "Mercantile": "merchant",
             "Speech": "speaker", "Sneak": "thief", "Security": "thief", "Pickpocket": "thief", "assassin": "assassin",
             "shinobi": "shinobi", "bender": "bender", "prophet": "prophet", "artist": "artist"}
FAME_STEPS = [(500, "known in their own country as a fine {}"), (2000, "spoken of across the land as a great {}")]
GREATEST_AT = 5000


def rarity_of(bonus):
    """Made things top out at Relic; Worldforged is kept for the eight relics of the world's making."""
    for i, t in enumerate((0.08, 0.15, 0.28, 0.45, 0.75)):
        if bonus < t:
            return i
    return 5


def gain_fame(realm, c, specialty, amount):
    """Fame in a specialty: renown for the person, and legacy for their line, as far as the tale is told."""
    if amount <= 0 or not c.alive:
        return
    before = c.fame.get(specialty, 0.0)
    c.fame[specialty] = before + amount
    c.renown += amount * 0.5
    kind = SKILL_LEGACY.get(specialty, specialty if specialty in ("assassin", "shinobi", "bender", "prophet",
                                                                  "concord") else None)
    if kind is not None:
        from .lineage import gain_legacy
        gain_legacy(realm, c, kind, amount * 0.5)
    word = FAME_WORD.get(specialty)
    if word:
        for at, text in FAME_STEPS:
            if before < at <= c.fame[specialty]:
                realm.say(f"{realm.who(c)} is {text.format(word)}.", minor=True)
        # "the greatest of the age" is one title per craft, and must be taken from whoever holds it
        holder = realm.chars.get(realm.greatest.get(word))
        bar = max(GREATEST_AT, holder.fame.get(specialty, 0) if holder is not None and holder.alive else 0)
        if c.fame[specialty] >= bar and (holder is None or holder.id != c.id):
            realm.greatest[word] = c.id
            was = f", surpassing {holder.full_name()}" if holder is not None and holder.alive else ""
            realm.say(f"**{realm.who(c)} is called the greatest {word} of the age**{was}.")


# ---------------------------------------------------------------- making things
def craft(realm, smith):
    rng = realm.rng
    k = realm.kingdoms[smith.realm]
    sk = skill(realm, smith, "Smithing")
    from .civics import material_cap
    from .resources import material_ok
    avail = [m for i, m in enumerate(MATERIALS) if sk >= m[1] and i <= material_cap(realm, smith.people)
             and material_ok(k, i)]
    if avail[-1][0] == "Dragonbone" and k.dragonbone <= 0:
        avail = avail[:-1]
    mat_i = len(avail) - 1 if rng.random() < 0.7 else max(0, len(avail) - 2)
    mat = avail[mat_i]
    if mat[0] == "Dragonbone":
        k.dragonbone -= 1
    roll = sk / 20 + perk_count(smith, "Smithing") * 0.3 + rng.normal(0, 0.8) + smith.attributes["lck"] / 100
    q = int(max(0, min(len(QUALITY) - 1, roll - 1)))
    weapon = rng.random() < 0.6
    wname, wskill = (WEAPONS if weapon else ARMORS)[int(rng.integers(len(WEAPONS if weapon else ARMORS)))]
    bonus = mat[2] * QUALITY[q][1]
    it = Item(f"{QUALITY[q][0]} {mat[0]} {wname}", rarity_of(bonus), realm.year, f"the forge of {smith.name}")
    it.crafted, it.material, it.quality, it.kind = True, mat[0], QUALITY[q][0], ("weapon" if weapon else "armor")
    it.wskill, it.maker, it.enchants, it._bonus = wskill, smith.id, [], bonus
    it.mat_index = MATERIALS.index(mat)
    it.value = int(20 * (bonus * 100) ** 1.3)
    if bonus >= 0.75:
        it.name = master_name(realm)
        it.desc = f"{QUALITY[q][0]} {mat[0]} {wname}"
        realm.say(f"{realm.who(smith)} forges {it.name}, a {it.desc}.", minor=bonus < 1.2)
        gain_fame(realm, smith, "Smithing", 40 + 100 * bonus)
    else:
        gain_fame(realm, smith, "Smithing", 1 + 8 * bonus)
    use(realm, smith, "Smithing", 1.5)
    k.made.append(it.mat_index)
    return it


def master_name(realm):
    rng = realm.rng
    for _ in range(60):
        n = f"the {rng.choice(ITEM_A)} of {rng.choice(ITEM_B)}"
        if n not in realm.item_names:
            realm.item_names.add(n)
            return n
    return f"the {rng.choice(ITEM_A)} of {first_name(rng, 0, False)}"


def enchant(realm, ench, it):
    rng = realm.rng
    k = realm.kingdoms[ench.realm]
    sk = skill(realm, ench, "Enchanting")
    tiers = [i for i, (_, need, _) in enumerate(ENCHANT_TIERS) if sk >= need]
    t = tiers[-1]
    while t >= 2 and k.soulstones[t] <= 0:          # Greater and above need a soulstone of that strength
        t -= 1
    if t >= 2:
        k.soulstones[t] -= 1
    double = perk_count(ench, "Enchanting") >= 3 and rng.random() < 0.5
    pool = ENCHANT_WEAPON if getattr(it, "kind", "weapon") == "weapon" else ENCHANT_ARMOR
    effects = [str(rng.choice(pool))] + ([str(rng.choice(pool))] if double else [])
    add = ENCHANT_TIERS[t][2] * (1 + 0.1 * perk_count(ench, "Enchanting")) * len(effects)
    it.enchants = getattr(it, "enchants", []) + [(e, ENCHANT_TIERS[t][0]) for e in effects]
    it._bonus = it.bonus + add
    it.rarity = rarity_of(it._bonus)
    it.enchanter = ench.id
    it.value = int(getattr(it, "value", 100) + 25 * (add * 100) ** 1.3)
    use(realm, ench, "Enchanting", 1.5)
    if t >= 3:
        realm.say(f"{realm.who(ench)} lays a {ENCHANT_TIERS[t][0]} {' and '.join(effects)} enchantment on "
                  f"{it.name}.", minor=t < 4)
        gain_fame(realm, ench, "Enchanting", 30 * t)
    else:
        gain_fame(realm, ench, "Enchanting", 4 * (t + 1))


# ---------------------------------------------------------------- specialists
def spawn_specialists(realm):
    rng = realm.rng
    y = realm.year
    for f in range(realm.npol):
        if realm.extinct[f]:
            continue
        towns = sorted(realm.alive_towns(f), key=lambda t: -t.pop)
        if not towns:
            continue
        have = {}
        for c in realm.members(f):
            if c.profession:
                have[c.profession] = have.get(c.profession, 0) + 1
        ntowns = len(towns)
        for prof, (cap, prim, _) in PROFESSIONS.items():
            cap = min(cap, ntowns // 6 + (1 if prof == "smith" and ntowns >= 2 else 0))
            if have.get(prof, 0) >= cap or rng.random() > 0.15:
                continue
            # a master's child takes up the trade first
            kin = [realm.chars[i] for i in realm.member_ids(f) if True and not realm.chars[i].profession
                   and not realm.chars[i].hero and realm.chars[i].adult(y) and not realm.chars[i].ruled
                   and realm.chars[i].father is not None
                   and realm.chars[realm.chars[i].father].profession == prof]
            if kin:
                c = kin[0]
                why = f"takes up the trade of {realm.chars[c.father].full_name()}"
            else:
                c = make_character(realm, f, born=y - ADULT[realm.cul(f)] - int(rng.integers(0, 15)))
                t = towns[int(rng.integers(min(6, len(towns))))]
                c.home = t.id
                why = None
            c.profession = prof
            for s in prim:
                c.skills[s] = min(100.0, c.skills[s] + float(rng.integers(20, 40)))
            if prof == "tamer":
                c.genetic.add("beastspeaker")
            if prof in ("thief", "assassin", "bandit"):
                c.bounty = 0.0
            if why:
                realm.say(f"{realm.who(c)} {why}.", minor=True)


def specialists_year(realm):
    rng = realm.rng
    for f in range(realm.npol):
        k = realm.kingdoms[f]
        k.made = []
    for i in list(realm.living):
        c = realm.chars[i]
        if not c.profession or getattr(c, "jailed_until", 0) > realm.year:
            continue
        k = realm.kingdoms[c.realm]
        if realm.extinct[c.realm]:
            continue
        p = c.profession
        if p == "smith":
            for _ in range(1 + int(skill(realm, c, "Smithing") > 60)):
                k.market.append(craft(realm, c))
        elif p == "enchanter":
            plain = [it for it in k.market if not getattr(it, "enchants", None)]
            if plain:
                enchant(realm, c, max(plain, key=lambda it: it.bonus))
        elif p == "alchemist":
            k.potions += skill(realm, c, "Alchemy") / 20
            use(realm, c, "Alchemy", 1.2)
            gain_fame(realm, c, "Alchemy", skill(realm, c, "Alchemy") / 15)
        elif p == "merchant":
            home = realm.by_id.get(c.home) or realm.capital_of(c.realm)
            income = 20 + skill(realm, c, "Mercantile") * 2 + (home.wealth * 0.01 if home else 0)
            c.gold += income
            if home is not None:
                home.wealth += income * 0.5
            use(realm, c, "Mercantile", 1.2)
            use(realm, c, "Speech", 0.5)
            gain_fame(realm, c, "Mercantile", income / 40)
    for f in range(realm.npol):
        k = realm.kingdoms[f]
        if realm.extinct[f]:
            continue
        k.market.sort(key=lambda it: -it.bonus)
        del k.market[30:]
        # the crown arms its hosts from its own smiths
        if k.made:
            k.arms = 0.9 * k.arms + 0.1 * (sum(k.made) / len(k.made))
        budget = 0.01 * sum(t.wealth for t in realm.alive_towns(f))
        smiths = [realm.chars[i] for i in realm.member_ids(f) if True and
                  realm.chars[i].profession == "smith"]
        for s in smiths:
            s.gold += budget / len(smiths)
        k.battle_mult *= 1 + 0.04 * k.arms
    market(realm)


def market(realm):
    """Heroes, shinobi, benders and the great of each people buy from their own people's market."""
    buyers = [realm.chars[i] for i in realm.living if realm.chars[i].gold > 50 and
              (realm.chars[i].hero or realm.chars[i].ruled or getattr(realm.chars[i], "clan", None) is not None
               or getattr(realm.chars[i], "element", None) is not None)]
    buyers.sort(key=lambda c: -c.gold)
    for c in buyers:
        k = realm.kingdoms[c.realm]
        worst = min((it.bonus for it in c.items), default=0.0) if len(c.items) >= 4 else 0.0
        options = [it for it in k.market if it.value <= c.gold and it.bonus > worst + 0.02]
        if not options:
            continue
        it = max(options, key=lambda i: i.bonus)
        k.market.remove(it)
        c.gold -= it.value
        maker = realm.chars.get(getattr(it, "maker", None))
        ench = realm.chars.get(getattr(it, "enchanter", None))
        if maker is not None:
            maker.gold += it.value * (0.7 if ench else 1.0)
            gain_fame(realm, maker, "Smithing", it.value / 1000)
        if ench is not None:
            ench.gold += it.value * 0.3
        from .heroes import give_item
        give_item(realm, c, it, quiet=True)
        from .events import private
        if it.value >= 300:
            private(realm, c, "market", -it.value, f"{c.full_name()} buys {it.name} from "
                    f"{maker.full_name() if maker else 'the market'}")
        if it.rarity >= 4:
            realm.say(f"{realm.who(c)} buys {it.label()} for {it.value:,} gold.", minor=True)
