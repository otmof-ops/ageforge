"""The deep layer of the fantastic, and what it does to the world's work and wealth.

GIFTS. One person in about a hundred and twenty is born with a gift: an innate power no training gives and no
training takes away. A Mana Well casts a tier above their training; a Greenhand's fields yield more; an Appraiser
sees the worth of anything at a glance; a Spellweaver can reach past the tenth tier. Gifts are not inherited.

BATTLE ARTS. Those who fight without magic learn named arts as they rise: stances, breaths and strikes that make a
swordsman's body a weapon on its own terms. The greatest learn a lost art.

TIERS. As the old accounts tell it, most casters never pass the third tier; the fourth to sixth belong to the rare
masters of an age, and no ordinary mortal goes beyond the sixth (a Mana Well's gift, a power bloodline or a relic may
lift one to the seventh, and no further). The eighth tier and above, and BEYOND THE TENTH TIER, belong only to the
characters the player makes. One of the tenth tier (or the ninth, with the Spellweaver's gift) may work a working of
the beyond-tier once in a long while: fire from heaven on a host, a harvest of the dead, a spring that
lasts ten years, a dawn that burns out plague, the unmaking of walls, a tide of spirits. Each school has its own.

WILD MAGIC. Older than the tiers, and wielded only by dragons of three centuries and more. It strikes as hard as the
seventh tier, and every casting burns sixty years of the dragon's life; a dragon that spends too much dies of it.

SCROLLS AND MONSTER STUFF. Casters write their spells into scrolls; anyone may read one. Below the third tier a
scroll needs only parchment. Higher tiers need hides, ash, horn and scale that only monsters give: a beast's hide for
the third and fourth tiers, a wraith's ash for the fifth and sixth, a demon's horn for the seventh and eighth, a
dragon's scale for the ninth and tenth. So hunting monsters is a trade, and a realm without hunters cannot write
high magic. Scrolls arm hosts and walls, and sell abroad.

THE WARDENS' HALL. Monster hunters are licensed by seal, from the Ash Seal to the Sun Seal, by their deeds. A town
threatened by a monster posts a bounty; the slayer collects it, and a dragon's hoard goes back into the world.

MONSTERS AND THE ECONOMY. Every monster casts a shadow: fields near a lair go untended and roads go unused. A raid
drives refugees into the nearest towns, empties the merchants' stores, and frightens trade away for years; a dragon
carries off the gold. A calamity collapses the markets of a whole continent.

THE BOUND DEAD. A realm whose ruler or court mage is a necromancer of the fourth tier or above (or a lich) may bind
its dead to work the fields and mines: tireless, unpaid, owned by the lords. The living lose that work, the people
live in dread, and the neighbours remember."""
import math

import numpy as np

from .data import NECRO, PROWESS_CAP, SCHOOLS

GIFTS = {  # name: what it does
    "Mana Well": "casts one tier above their training, and past their people's limit",
    "Spellweaver": "can reach beyond the tenth tier from the ninth",
    "Greenhand": "the fields of their town yield more",
    "Appraiser": "sees the worth of anything; earns double at trade, lifts their town's markets",
    "Forgeheart": "teaches a trade twice as well",
    "Mender's Touch": "the sick of their town recover",
    "Iron Skin": "a body like armour",
    "Unfailing Memory": "learns everything faster",
    "Silver Tongue": "persuades anyone",
    "Born Captain": "men follow them",
    "Seer's Eye": "sees through lies and plots",
    "Beastspeech": "speaks with beasts",
}
GIFT_CHANCE = 1 / 120
ARTS = ["Steel Breath", "Iron Fortress Stance", "Flowing Step", "Twin Fang Strike", "Mind's-Eye Parry",
        "Thousand Cuts", "Heaven-Cleaver"]
LOST_ARTS = ["The Last Stance", "Sky-Splitting Edge", "The Unbroken Wall"]
MARTIAL_CLASSES = {"Warrior", "Knight", "Ranger", "Rogue", "Warlord", "Dragonslayer", "Spellblade", "Shadowblade"}
BEYOND = {  # school: (name, what it does)
    "Arcane": "the Fall of Heaven's Fire",
    "Divine": "the Dawn That Burns the Sickness",
    "Nature": "the Long Spring",
    "Rune": "the Rune of Unmaking",
    "Necromancy": "the Black Harvest",
    "Spirit": "the Spirit Tide",
}
PARTS = {"troll": "hide", "beast": "hide", "wraith": "ash", "demon": "horn", "dragon": "scale", "calamity": "scale"}
PART_FOR_TIER = {3: "hide", 4: "hide", 5: "ash", 6: "ash", 7: "horn", 8: "horn", 9: "scale", 10: "scale"}
SEALS = ["Ash Seal", "Oak Seal", "Iron Seal", "Silver Seal", "Gold Seal", "Star Seal", "Sun Seal"]
SEAL_AT = [0, 40, 150, 400, 900, 2000, 5000]                        # renown for each seal


# ---------------------------------------------------------------- gifts
def maybe_gift(realm, c):
    if realm.rng.random() < GIFT_CHANCE:
        c.gift = str(realm.rng.choice(list(GIFTS)))
        if c.gift == "Iron Skin":
            c.prowess += 8
        elif c.gift == "Unfailing Memory":
            c.talent = min(10.0, c.talent + 1.5)
        elif c.gift == "Silver Tongue":
            c.attrs["dip"] += 4
        elif c.gift == "Born Captain":
            c.attrs["mar"] += 4
        elif c.gift == "Seer's Eye":
            c.attrs["intr"] += 4
        elif c.gift == "Beastspeech":
            c.genetic.add("beastspeaker")
        c.invalidate()


# ---------------------------------------------------------------- the year
def arcana_year(realm):
    threat_year(realm)
    for k in realm.kingdoms:
        if realm.extinct[k.id] or k.ruler is None:
            continue
        if not hasattr(k, "parts"):
            k.parts = {"hide": 0.0, "ash": 0.0, "horn": 0.0, "scale": 0.0}
            k.scrolls = [0.0] * 11
        bound_dead(realm, k)
    scribes_year(realm)
    arts_year(realm)
    beyond_year(realm)
    wild_magic(realm)


# ---------------------------------------------------------------- monsters on the land
def threat_year(realm):
    """How much each town lives in a monster's shadow (vectorised by landmass)."""
    towns = realm.alive_towns()
    if not towns:
        return
    tp = np.array([t.pos for t in towns], float)
    tl = np.array([t.landmass for t in towns])
    threat = np.zeros(len(towns))
    for m in realm.monsters:
        if not m.alive or getattr(m, "sea", False):
            continue
        sel = tl == m.landmass
        if not sel.any():
            continue
        d = np.hypot(tp[sel, 0] - m.pos[0], tp[sel, 1] - m.pos[1])
        reach = 40 if m.kind == "calamity" else 15
        w = min(m.power, 4000) / 4000
        if m.kind == "dragon":
            from .dragons import RAID_MULT, hidden
            w *= 0 if hidden(m, realm.year) else RAID_MULT.get(getattr(m, "temper", "territorial"), 1.0) * 0.6
        threat[sel] += w * np.exp(-d / reach)
    for t, v in zip(towns, threat):
        t.threat = float(min(3.0, v))
    # a calamity on a continent: its markets collapse while it lives
    for m in realm.monsters:
        if m.alive and m.kind == "calamity" and not getattr(m, "crashed", False):
            m.crashed = True
            lands = realm.lands[m.landmass]["name"] if m.landmass < len(realm.lands) else "the land"
            realm.say(f"**The markets of {lands} collapse.** With {m.name} abroad no caravan moves, no lender lends, "
                      f"and the merchants bury their gold.")
            for k in realm.kingdoms:
                if not realm.extinct[k.id] and any(t.landmass == m.landmass for t in realm.alive_towns(k.id)):
                    k.credit = max(0.0, getattr(k, "credit", 50.0) - 15)
                    k.trade_shock = realm.year + 10


def raid_shock(realm, m, s):
    """A town falls to a monster: refugees, ruined stores, gold carried off, and trade frightened away."""
    k = realm.kingdoms[s.faction]
    s.raided_year = realm.year
    near = sorted((t for t in realm.alive_towns(s.faction) if t is not s and t.landmass == s.landmass),
                  key=lambda t: (t.pos[0] - s.pos[0]) ** 2 + (t.pos[1] - s.pos[1]) ** 2)[:3]
    fled = s.pop * 0.15
    if near:
        s.pop -= fled
        for t in near:
            t.pop += fled / len(near)                                   # crowding, and poverty, follow the refugees
    if hasattr(s, "stock"):
        s.stock["middle"] *= 0.6
    gold = getattr(s, "coin", 0.0) * (0.6 if m.kind == "dragon" else 0.2)
    if gold:
        s.coin -= gold
        m.hoard += gold
    k.trade_shock = max(getattr(k, "trade_shock", 0), realm.year + 4)
    if getattr(k, "credit", None) is not None:
        k.credit = max(0.0, k.credit - 2)


def on_slain(realm, m, slayer, people):
    """Monster stuff for the scribes, the bounty for the slayer, and the hoard back into the world."""
    from .events import book, private
    from .bounties import pay_out
    pay_out(realm, m, slayer)
    if people is None:
        return
    k = realm.kingdoms[people]
    if not hasattr(k, "parts"):
        k.parts = {"hide": 0.0, "ash": 0.0, "horn": 0.0, "scale": 0.0}
        k.scrolls = [0.0] * 11
    part = PARTS.get(m.kind)
    if part:
        k.parts[part] += max(1.0, m.base / 300)
    if slayer is None:
        return
    bounty = min(k.gold * 0.5, m.base * 0.02) if k.gold > 0 else 0.0
    if bounty > 0:
        k.gold -= bounty
        slayer.gold += bounty
        book(realm, k.id, "bounties", -bounty, f"pays {slayer.full_name()} for {m.name}")
    hoard = getattr(m, "hoard", 0.0)
    if hoard > 1:
        slayer.gold += hoard * 0.3
        k.gold += hoard * 0.7
        private(realm, slayer, "hoard", hoard * 0.3, f"{slayer.full_name()} carries off the hoard of {m.name}")
        book(realm, k.id, "hoards", hoard * 0.7, f"the hoard of {m.name} comes home")
        m.hoard = 0.0
        if hoard > 50:
            realm.say(f"The hoard of {m.name}, {hoard:,.0f} gold, comes back into the world; {realm.who(slayer)} "
                      f"keeps a third.")


# ---------------------------------------------------------------- scrolls
def scroll_value(t):
    return 0.15 * 1.8 ** t


def max_scroll_tier(k, t):
    """The highest tier a scribe of tier t can write in realm k, given the monster stuff in its stores."""
    while t >= 3 and k.parts.get(PART_FOR_TIER[t], 0) < 0.2:
        t -= 1
    return t


def scribes_year(realm):
    """Casters write scrolls, limited by their tier and by the realm's monster stuff; the realm's stock arms its
    hosts and walls; the best are sold abroad."""
    from .events import private
    out = {}
    for i in realm.living:
        c = realm.chars[i]
        t = c.tier()
        if t < 1 or c.hero and realm.rng.random() < 0.5:
            continue
        out.setdefault(c.realm, []).append((c, t))
    for k in realm.kingdoms:
        if realm.extinct[k.id] or not hasattr(k, "scrolls"):
            continue
        k.scrolls = [v * 0.85 for v in k.scrolls]                       # read in the field, lost, crumbled
        for c, t in out.get(k.id, ()):
            tt = max_scroll_tier(k, t)
            n = 1 + t // 3
            if tt >= 3:
                k.parts[PART_FOR_TIER[tt]] -= 0.2 * n
            k.scrolls[tt] += n
            earn = scroll_value(tt) * n * 0.5
            c.gold += earn
            if tt >= 7 and realm.rng.random() < 0.05:
                realm.say(f"{realm.who(c)} writes a scroll of the {['cantrip', 'first', 'second', 'third', 'fourth', 'fifth', 'sixth', 'seventh', 'eighth', 'ninth', 'tenth'][tt]} "
                          f"tier on {PART_FOR_TIER[tt]} from a slain monster.", minor=True)
        power = sum(v * (t + 1) for t, v in enumerate(k.scrolls))
        k.scroll_power = power
        k.battle_mult = getattr(k, "battle_mult", 1.0) * (1 + min(0.4, power / 2000))
        k.top_scroll = max((t for t, v in enumerate(k.scrolls) if v >= 1), default=0)


# ---------------------------------------------------------------- the bound dead
def bound_dead(realm, k):
    r = realm.chars[k.ruler]
    mage = realm.chars.get(k.council.get("mage")) if getattr(k, "council", None) else None
    necro = any(x is not None and x.school() == NECRO and x.tier() >= 4 for x in (r, mage)) or r.undead
    share = getattr(k, "bound_share", 0.0)
    p = r.personality
    willing = necro and "compassionate" not in p and "zealous" not in p
    if willing:
        if share == 0.0:
            realm.say(f"**The dead rise to work.** {realm.who(r)} binds the dead of {k.name} to the plough and the "
                      f"mine; they need no bread and take no wage, and the living watch them with dread.")
        k.bound_share = min(0.15, share + 0.01)
        for t in realm.alive_towns(k.id):
            if hasattr(t, "mood"):
                t.mood["peasants"] -= 3
        for o in range(realm.npol):
            if o != k.id and not realm.extinct[o] and realm.relation[k.id, o] > -60:
                realm.relation[k.id, o] -= 0.5
                realm.relation[o, k.id] -= 0.5
    elif share > 0:
        k.bound_share = max(0.0, share - 0.03)                            # laid back to rest
        if k.bound_share == 0.0:
            realm.say(f"The bound dead of {k.name} are laid to rest at last.", minor=True)


# ---------------------------------------------------------------- battle arts
def arts_year(realm):
    for i in realm.living:
        c = realm.chars[i]
        if not any(cls in MARTIAL_CLASSES for cls, _ in c.classes):
            continue
        have = getattr(c, "battle_arts", None)
        from .characters import power_gate
        gate = power_gate(c, martial=True)
        n = min(c.level // 8, {3: 3, 4: 4, 5: 5}.get(gate, 7))         # three arts for most; the rest for the few
        if have is None or len(have) < n:
            c.battle_arts = ARTS[:n]
            if n >= 5 and realm.rng.random() < 0.3:
                realm.say(f"{realm.who(c)} masters {ARTS[n - 1]}.", minor=True)
        if gate >= 6 and len(getattr(c, "battle_arts", [])) == len(ARTS) and not getattr(c, "lost_art", None) \
                and c.prowess >= 0.75 * PROWESS_CAP[c.people] and realm.rng.random() < 0.05:
            c.lost_art = str(realm.rng.choice(LOST_ARTS))
            realm.say(f"**A lost art.** {realm.who(c)} rediscovers {c.lost_art}, unseen for an age.")


def arts_bonus(c):
    return 1 + 0.04 * len(getattr(c, "battle_arts", ())) + (0.15 if getattr(c, "lost_art", None) else 0.0)


# ---------------------------------------------------------------- beyond the tenth tier
def can_beyond(c):
    t = c.tier()
    return t >= 10 or (t >= 9 and getattr(c, "gift", None) == "Spellweaver")


def is_special(c):
    """Only characters the player made (or made custom in god mode) reach the high tiers."""
    return getattr(c, "custom", False)


def beyond_year(realm):
    rng = realm.rng
    for i in list(realm.living):
        c = realm.chars[i]
        if not is_special(c) or not can_beyond(c) or realm.year < getattr(c, "beyond_ready", 0):
            continue
        k = realm.kingdoms[c.realm]
        if realm.extinct[c.realm] or rng.random() > 0.15:
            continue
        circle = None
        school = SCHOOLS[c.school()]
        name = BEYOND[school]
        towns = realm.alive_towns(k.id)
        text = None
        foes = [a for a in getattr(realm, "armies", ()) if realm.war[k.id, a.realm]]
        if school == "Arcane" and foes:
            a = max(foes, key=lambda a: a.men)
            lost = int(a.men * 0.7)
            a.men -= lost
            text = f"works {name} upon the host of {realm.kingdoms[a.realm].name}: {lost:,} men burn"
        elif school == "Necromancy" and foes:
            a = max(foes, key=lambda a: a.men)
            lost = int(a.men * 0.5)
            a.men -= lost
            k.bound_share = min(0.25, getattr(k, "bound_share", 0.0) + 0.05)
            text = f"works {name}: {lost:,} of the enemy fall, and rise again in {k.name}'s service"
        elif school == "Divine" and any(getattr(t, "sick_now", 0) > 0.05 for t in towns):
            for e in realm.epidemics:
                if e.front:
                    e.front = {tid: v for tid, v in e.front.items() if realm.by_id.get(tid) is None
                               or realm.by_id[tid].faction != k.id}
            text = f"works {name}: the sickness in {k.name} is burned away"
        elif school == "Nature" and any(getattr(t, "famine", 0) > 0 for t in towns):
            realm.god_harvest = getattr(realm, "god_harvest", {})
            realm.god_harvest[k.id] = (1.4, realm.year + 10)
            text = f"works {name}: ten years of harvests without a failed field"
        elif school == "Rune" and foes:
            foe_towns = [t for t in realm.alive_towns() if realm.war[k.id, t.faction]]
            if foe_towns:
                t = max(foe_towns, key=lambda t: t.walls)
                t.walls = 0
                text = f"works {name}: the walls of {t.name} fall to dust"
        elif school == "Spirit" and (foes or any(m.alive and m.kind == "calamity" for m in realm.monsters)):
            k.battle_mult = getattr(k, "battle_mult", 1.0) * 2
            text = f"works {name}: the spirits of the land march with {k.name}'s hosts"
        if text:
            c.beyond_ready = realm.year + 40
            c.renown += 300
            realm.say(f"**Beyond the tenth tier.** {realm.who(c)} {text}.")


# ---------------------------------------------------------------- wild magic
WILD_COST = 60                                                       # years of life each casting burns


def wild_magic(realm):
    """The ancient dragons' magic, older than the tiers: as strong as the seventh tier, paid for in years of life.
    A dragon casts it seldom, and more readily when it is wounded and fighting for its life."""
    rng = realm.rng
    for m in realm.monsters:
        if not m.alive or m.kind != "dragon" or realm.year - m.born < 300:
            continue
        hurt = m.power < 0.6 * m.base
        if rng.random() > (0.03 if hurt else 0.002) or realm.year < getattr(m, "wild_ready", 0):
            continue
        near = [t for t in realm.alive_towns() if t.landmass == m.landmass]
        if not near:
            continue
        m.wild_ready = realm.year + 50
        m.lifespan -= WILD_COST
        t = min(near, key=lambda t: (t.pos[0] - m.pos[0]) ** 2 + (t.pos[1] - m.pos[1]) ** 2)
        if hurt:
            m.power = max(m.power, 0.9 * m.base)
            what = f"knits its torn body whole"
        elif rng.random() < 0.5:
            from .legends import curse_land
            curse_land(realm, t.pos, m.landmass, m.name, 6)
            what = f"withers the land around {t.name}"
        else:
            t.pop *= 0.7
            t.walls = max(0, t.walls - 1)
            what = f"lays a blight of fire and ash on {t.name}"
        left = int(m.lifespan - (realm.year - m.born))
        realm.say(f"**Wild magic.** {m.name}, ancient beyond the reckoning of the tiers, speaks a word in the old "
                  f"tongue and {what}; it pays with {WILD_COST} years of its life"
                  + (", and has little left to give." if left < 60 else "."))


# ---------------------------------------------------------------- the wardens' hall
def seals_year(realm):
    for i in realm.hero_ids:
        c = realm.chars.get(i)
        if c is None or not c.alive:
            continue
        seal = max(n for n, at in enumerate(SEAL_AT) if c.renown >= at)
        if getattr(c, "seal", -1) < seal:
            c.seal = seal
            if seal >= 4:
                realm.say(f"The Wardens' Hall gives {realm.who(c)} the {SEALS[seal]}.", minor=seal < 6)
