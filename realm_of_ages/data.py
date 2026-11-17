"""Static tables: land, peoples, magic, classes and skill trees, traits, items, monsters, names."""
import numpy as np

# ---------------------------------------------------------------- land
OCEAN, PLAINS, FOREST, HILLS, MOUNTAIN, SWAMP, DESERT = range(7)
TERRAIN_COLORS = [(36, 70, 120), (150, 178, 92), (46, 110, 56), (150, 136, 96), (125, 120, 116),
                  (82, 104, 74), (214, 194, 132)]

# ---------------------------------------------------------------- peoples
HUMANS, ELVES, DWARVES, ORCS = range(4)
PEOPLES = ["Humans", "Elves", "Dwarves", "Orcs"]
ADJ = ["Human", "Elven", "Dwarven", "Orcish"]
SINGULAR = ["Human", "Elf", "Dwarf", "Orc"]
COLORS = [(214, 64, 52), (60, 205, 135), (240, 175, 40), (150, 70, 190)]

#                 ocean plains forest hills mount swamp desert
FOOD = np.array([[0.9, 3.0, 1.6, 1.4, 0.2, 0.8, 0.5],
                 [0.6, 1.8, 3.0, 0.9, 0.2, 1.0, 0.3],
                 [0.4, 1.4, 1.0, 2.6, 1.6, 0.5, 0.6],
                 [0.5, 2.2, 1.2, 1.4, 0.5, 2.0, 1.6]])
WEALTH = np.array([[0.6, 0.8, 0.8, 1.2, 1.5, 0.3, 0.6],
                   [0.4, 0.5, 1.4, 0.6, 0.6, 0.5, 0.3],
                   [0.2, 0.4, 0.6, 2.2, 3.2, 0.2, 0.8],
                   [0.3, 0.6, 0.5, 1.0, 1.2, 0.8, 1.0]])
GROWTH = [0.035, 0.024, 0.026, 0.038]
MARTIAL = [1.0, 1.1, 1.25, 1.4]
MAGIC = [0.6, 1.6, 0.7, 0.4]
DEFENSE = [1.0, 1.2, 1.6, 0.9]
AGGRESSION = [1.0, 0.5, 0.7, 1.8]
SETTLE_AT = [500, 350, 450, 380]
BASE_REL = np.array([[0, 20, 15, -40], [20, 0, -10, -60], [15, -10, 0, -50], [-40, -60, -50, 0]], float)

LIFESPAN = [75, 600, 260, 55]          # natural span of a notable person
ADULT = [16, 50, 30, 13]
FERTILE_END = [44, 400, 180, 38]
FERTILITY = [0.50, 0.07, 0.16, 0.55]   # yearly chance of a birth for a married woman, before decline
TALENT_BASE = [2.6, 3.6, 2.2, 1.8]     # magical talent: the highest tier a person can ever reach
PROWESS_BASE = [8, 7, 10, 13]
SUCCESSION = ["primogeniture", "elective", "seniority", "strongest"]
REALM_STYLE = ["the Kingdom of {}", "the Sylvan Court of {}", "the Under-Throne of {}", "the Horde of {}"]
RULER_TITLE = [("King", "Queen"), ("Elf-King", "Elf-Queen"), ("Under-King", "Under-Queen"),
               ("Warchief", "Warchief")]
HOUSE_STYLE = ["House {}", "the Line of {}", "Clan {}", "the {} Blood"]
HERO_CAP = 8

# ---------------------------------------------------------------- magic
ARCANE, DIVINE, NATURE, RUNE, NECRO, SPIRIT = range(6)
SCHOOLS = ["Arcane", "Divine", "Nature", "Rune", "Necromancy", "Spirit"]
SPELLS = [
    ["Spark", "Force Dart", "Arcane Ward", "Lightning Lance", "Prismatic Burst", "Shattering Gale",
     "Starfall Lance", "Time's Stillness", "Ninefold Ruin", "Sky-Breaker Meteor", "The Last Equation"],
    ["Candle Blessing", "Mend Flesh", "Holy Ward", "Sun Censure", "Purging Flame", "Restoration",
     "Wings of Judgement", "Resurrection", "Heaven's Gate", "Choir of Swords", "Dawn Unending"],
    ["Seed Call", "Thornlash", "Barkskin", "Call the Pack", "Thunderwood", "Earth's Wrath", "Wild Hunt",
     "Ancient Grove", "Heartquake", "World-Tree's Fury", "Verdant Rebirth"],
    ["Spark-Rune", "Rune of Iron", "Rune of Warding", "Rune of Fire", "Rune of Shattering",
     "Rune of the Mountain", "Rune of Unmaking", "Rune of the Deep King", "Rune of Ten Thousand Hammers",
     "Rune of the Forge-Heart", "The Final Rune"],
    ["Chill Touch", "Bone Servant", "Dread", "Drain Life", "Raise the Fallen", "Plague Wind", "Soul Harvest",
     "Legion of Ash", "Grave Tide", "Black Sun", "Death of Ages"],
    ["Ancestor's Whisper", "Blood Rage", "Spirit Wolf", "Totem of War", "Storm Howl", "Bloodstorm",
     "Ancestral Host", "Rite of Skulls", "Thunder of the Fathers", "Wrath of the Ancestors", "The Red Moon"],
]
TIER_WORDS = ["cantrip", "first", "second", "third", "fourth", "fifth", "sixth", "seventh", "eighth", "ninth",
              "tenth"]
ROMAN = ["0", "I", "II", "III", "IV", "V", "VI", "VII", "VIII", "IX", "X"]
WORLD_RITES = ["the Rite of Ashfall", "the Rite of Dawn", "the Rite of the Greening", "the Rite of the Iron Dawn",
               "the Rite of the Long Night", "the Rite of the Blood Moon"]


def spell_power(tier):
    return 12 * 1.9 ** tier


# ---------------------------------------------------------------- classes and skill trees
# Each base class: physical and magical weight, school, toughness, three branches of four skills (each
# branch improves one effect key) and a capstone taken once all twelve are learned.
# Effect keys: phys magic hp war guard vs_monster delve equip (multipliers); tier lead intrigue (additive).
CLASSES = {
    "Warrior": dict(phys=1.4, magic=0.0, school=None, hp=1.3, branches=[
        ("Weapon Master", "phys", ["Blade Discipline", "Heavy Strikes", "Cleave", "Whirlwind"]),
        ("Juggernaut", "hp", ["Thick Skin", "Iron Will", "Second Wind", "Unbreakable"]),
        ("Warmonger", "war", ["Battle Cry", "Shield Wall", "Vanguard", "Field Commander"])],
        capstone=("Avatar of Battle", {"phys": 1.5, "war": 1.3})),
    "Knight": dict(phys=1.25, magic=0.3, school=DIVINE, hp=1.4, branches=[
        ("Guardian", "guard", ["Bulwark", "Oathkeeper", "Aegis", "Last Stand"]),
        ("Cavalier", "war", ["Lance Charge", "Mounted Mastery", "Breakthrough", "Thunder of Hooves"]),
        ("Paragon", "lead", ["Chivalry", "Inspiring Presence", "Banner of Kings", "Living Legend"])],
        capstone=("Paladin's Vow", {"guard": 1.4, "tier": 1})),
    "Ranger": dict(phys=1.2, magic=0.4, school=NATURE, hp=1.1, branches=[
        ("Marksman", "phys", ["Steady Aim", "Piercing Shot", "Volley", "Heartseeker"]),
        ("Hunter", "vs_monster", ["Tracker", "Beast Lore", "Monster Bane", "Apex Predator"]),
        ("Pathfinder", "delve", ["Trailcraft", "Survivalist", "Shadowstep", "Ghost of the Wilds"])],
        capstone=("Lord of the Hunt", {"vs_monster": 1.5})),
    "Rogue": dict(phys=1.15, magic=0.0, school=None, hp=1.0, branches=[
        ("Assassin", "phys", ["Backstab", "Poison Lore", "Vital Strike", "Death Mark"]),
        ("Thief", "delve", ["Lockbreaker", "Trapfinder", "Treasure Sense", "Vault Ghost"]),
        ("Spymaster", "intrigue", ["Whisper Network", "Blackmail", "Ciphers", "Puppet Master"])],
        capstone=("Shadow Sovereign", {"delve": 1.5, "intrigue": 10})),
    "Wizard": dict(phys=0.5, magic=1.5, school=ARCANE, hp=0.8, branches=[
        ("Evoker", "magic", ["Focused Power", "Overchannel", "Spell Echo", "Maximize Magic"]),
        ("Scholar", "tier", ["Arcane Theory", "Deep Study", "Forbidden Library", "Transcendence"]),
        ("Abjurer", "hp", ["Mana Shield", "Counterspell", "Spell Mirror", "Absolute Ward"])],
        capstone=("Archon of the Arcane", {"magic": 1.5, "tier": 1})),
    "Priest": dict(phys=0.6, magic=1.2, school=DIVINE, hp=1.0, branches=[
        ("Healer", "hp", ["Laying Hands", "Mass Mend", "Life Ward", "Miracle"]),
        ("Inquisitor", "magic", ["Smite", "Judgement", "Purge", "Wrath of Heaven"]),
        ("Hierophant", "tier", ["Scripture", "Revelation", "Divine Communion", "Voice of the Gods"])],
        capstone=("Living Saint", {"hp": 1.5, "guard": 1.3})),
    "Druid": dict(phys=0.7, magic=1.3, school=NATURE, hp=1.0, branches=[
        ("Shapeshifter", "phys", ["Wolf Form", "Bear Form", "Wyrm Form", "Primal Avatar"]),
        ("Stormcaller", "magic", ["Gale", "Lightning Call", "Tempest", "Eye of the Storm"]),
        ("Warden", "guard", ["Rootbind", "Living Wall", "Grove Keeper", "Heart of the Forest"])],
        capstone=("Voice of the World-Tree", {"magic": 1.3, "tier": 1})),
    "Runesmith": dict(phys=1.0, magic=1.0, school=RUNE, hp=1.2, branches=[
        ("Forgemaster", "equip", ["Tempering", "Masterwork", "Rune-Forging", "Relic Craft"]),
        ("Runecaster", "magic", ["Rune Lore", "Chain Runes", "Grand Glyph", "Rune of Power"]),
        ("Stonewarden", "guard", ["Stone Skin", "Hold the Gate", "Mountain's Patience", "Deep Bastion"])],
        capstone=("Rune-Father", {"equip": 1.4, "tier": 1})),
    "Shaman": dict(phys=0.9, magic=1.2, school=SPIRIT, hp=1.1, branches=[
        ("Bloodcaller", "war", ["Blood Frenzy", "War Drums", "Red Tide", "Bloodstorm Chief"]),
        ("Spiritwalker", "tier", ["Ancestor Sight", "Spirit Bargain", "Walk Between", "Spirit King's Favour"]),
        ("Totemist", "guard", ["Totem of Earth", "Totem of Fury", "Totem Circle", "Great Totem"])],
        capstone=("Speaker of the Ancestors", {"war": 1.4, "magic": 1.3})),
    "Necromancer": dict(phys=0.5, magic=1.4, school=NECRO, hp=0.9, branches=[
        ("Reanimator", "war", ["Bone Legion", "Corpse Craft", "Grave Champion", "Undying Host"]),
        ("Reaper", "magic", ["Soul Siphon", "Wither", "Doom", "Death's Embrace"]),
        ("Lichcraft", "tier", ["Phylactery Theory", "Black Rites", "Bone Throne", "Undeath"])],
        capstone=("Master of the Grave", {"magic": 1.4, "tier": 1})),
}
CLASS_WEIGHTS = [
    {"Warrior": 3, "Knight": 3, "Ranger": 2, "Rogue": 2, "Wizard": 2, "Priest": 2, "Necromancer": 0.3},
    {"Ranger": 3, "Wizard": 3, "Druid": 3, "Rogue": 1, "Knight": 1, "Priest": 1, "Necromancer": 0.1},
    {"Warrior": 3, "Knight": 2, "Runesmith": 4, "Priest": 1.5, "Rogue": 0.5},
    {"Warrior": 4, "Shaman": 3, "Rogue": 1.5, "Ranger": 1.5, "Necromancer": 0.7},
]
CASTERS = {k for k, v in CLASSES.items() if v["magic"] >= 1.0}

# Prestige classes: taken in place of a new base class at level 40 or 70 once earned.
PRESTIGE = {
    "Archmage": dict(phys=0.4, magic=1.6, hp=0.9, fx={"magic": 1.5, "tier": 1},
                     need="Wizard, and tier 5 or higher"),
    "Spellblade": dict(phys=1.3, magic=1.0, hp=1.1, fx={"phys": 1.25, "magic": 1.25},
                       need="a martial class and magical talent of 4 or more"),
    "Saint": dict(phys=0.6, magic=1.3, hp=1.3, fx={"hp": 1.4, "guard": 1.4, "tier": 1},
                  need="Priest or Knight, renown 200, kind or zealous"),
    "Lich": dict(phys=0.4, magic=1.6, hp=1.0, fx={"magic": 1.4, "tier": 1},
                 need="Necromancer, tier 6: casts off the flesh and stops ageing"),
    "Dragonslayer": dict(phys=1.5, magic=0.3, hp=1.3, fx={"vs_monster": 1.6, "phys": 1.2},
                         need="has slain a dragon"),
    "Warlord": dict(phys=1.4, magic=0.2, hp=1.2, fx={"war": 1.6, "lead": 6}, need="fifteen years of war"),
    "Shadowblade": dict(phys=1.3, magic=0.3, hp=1.0, fx={"delve": 1.5, "intrigue": 8, "phys": 1.2},
                        need="Rogue, eight ruins delved"),
    "Rune-Lord": dict(phys=1.1, magic=1.2, hp=1.2, fx={"equip": 1.5, "tier": 1, "guard": 1.3},
                      need="Runesmith, tier 4"),
    "Archdruid": dict(phys=0.8, magic=1.5, hp=1.0, fx={"magic": 1.4, "tier": 1}, need="Druid, tier 5"),
    "Spirit-King": dict(phys=1.0, magic=1.4, hp=1.1, fx={"war": 1.4, "magic": 1.3, "tier": 1},
                        need="Shaman, tier 4"),
}

# ---------------------------------------------------------------- traits
GENETIC = {"genius": {"lrn": 4, "talent": 1.5}, "quick": {"lrn": 2, "talent": 0.5},
           "slow": {"lrn": -3, "talent": -1}, "strong": {"prowess": 4, "mar": 1}, "weak": {"prowess": -3},
           "comely": {"dip": 2}, "spellborn": {"talent": 2}, "giant": {"prowess": 5, "life": 0.85},
           "frail": {"prowess": -2, "life": 0.8}, "longlived": {"life": 1.3},
           "hale": {"life": 1.1}, "sickly": {"life": 0.75}, "fecund": {}, "silver-tongued": {"lore": 3, "dip": 1},
           "dull": {"lore": -2, "lrn": -1}, "keen-eyed": {"prowess": 2}, "iron-boned": {"prowess": 2},
           # the gift to make: art, songs and books come only from the artistic, and it runs in families (works.rs)
           "artistic": {"lore": 1},
           # power genetics: rare bloodlines that break the limits a people is born under
           "starblood": {"talent": 3, "lrn": 2}, "dragonblood": {"prowess": 6, "talent": 1, "life": 1.5},
           "elderblood": {"life": 3.0, "lrn": 2, "lore": 2}, "titanblood": {"prowess": 10},
           "abyssblood": {"talent": 2, "intr": 2},
           "beastspeaker": {"lore": 1},
           # merged bloodlines: a master tamer's beast, taken into the blood for good
           "wolf-merged": {"prowess": 5, "life": 1.1}, "bear-merged": {"prowess": 9, "life": 1.15},
           "eagle-merged": {"prowess": 4, "lrn": 1}, "griffin-merged": {"prowess": 7, "talent": 1, "life": 1.2},
           "wyvern-merged": {"prowess": 9, "talent": 1, "life": 1.3},
           "dragon-merged": {"prowess": 12, "talent": 2, "life": 2.0}}
GENETIC_FREQ = {"genius": 0.02, "quick": 0.06, "slow": 0.04, "strong": 0.06, "weak": 0.04, "comely": 0.06,
                "spellborn": 0.03, "giant": 0.01, "frail": 0.03, "longlived": 0.03, "hale": 0.05, "sickly": 0.04,
                "fecund": 0.04, "silver-tongued": 0.04, "dull": 0.04, "keen-eyed": 0.05, "iron-boned": 0.03,
                "beastspeaker": 0.006, "artistic": 0.015}
POWER_GENES = {
    "starblood": "lifts the tier cap of their people by one, as far as the seventh tier and no further",
    "dragonblood": "breaks the level cap: they can climb past the hundredth level, to the hundred-and-thirtieth",
    "elderblood": "breaks the span of their people: they live three times as long",
    "titanblood": "breaks the prowess cap of their people, and strikes a third harder",
    "abyssblood": "breaks the talent cap: their magic is limited by training alone, and runs a third stronger",
    "wolf-merged": "carries a wolf in the blood: keen, swift and fierce",
    "bear-merged": "carries a great bear in the blood: vast strength",
    "eagle-merged": "carries an eagle in the blood: far sight",
    "griffin-merged": "carries a griffin in the blood: strength and the high air",
    "wyvern-merged": "carries a wyvern in the blood: venom and wings",
    "dragon-merged": "carries a dragon in the blood: fire, long life, and the bond that lets dragons answer them",
}
MERGED = {g.split("-")[0]: g for g in ("wolf-merged", "bear-merged", "eagle-merged", "griffin-merged",
                                       "wyvern-merged", "dragon-merged")}
BLOODLINES = tuple(POWER_GENES)
OPPOSED = (("genius", "slow"), ("strong", "weak"), ("giant", "frail"), ("hale", "sickly"),
           ("silver-tongued", "dull"))

# the limits every people is born under, which only power genetics break
TIER_CAP = [5, 6, 5, 4]
LEVEL_CAP, BLOOD_LEVEL_CAP = 100, 130
PROWESS_CAP = [30, 25, 35, 40]
STORY = [1.0, 1.3, 1.2, 0.9]           # how well each people tells and keeps its stories
PERSONALITY = [("brave", "craven"), ("ambitious", "content"), ("cruel", "kind"), ("zealous", "cynical"),
               ("greedy", "generous"), ("paranoid", "trusting"), ("wrathful", "calm"), ("honest", "deceitful"),
               ("diligent", "lazy"), ("gregarious", "shy"), ("lustful", "chaste"), ("gluttonous", "temperate"),
               ("arrogant", "humble"), ("forgiving", "vengeful"), ("patient", "impatient"), ("just", "arbitrary"),
               ("compassionate", "callous"), ("stubborn", "fickle")]
PERSONALITY_FX = {"brave": {"mar": 2}, "craven": {"mar": -2, "intr": 1}, "ambitious": {"all": 1},
                  "cruel": {"intr": 1, "dip": -2}, "kind": {"dip": 2}, "zealous": {"lrn": 1},
                  "cynical": {"intr": 1}, "greedy": {"ste": 1, "dip": -1}, "generous": {"dip": 1},
                  "paranoid": {"intr": 2}, "trusting": {"intr": -1, "dip": 1}, "wrathful": {"mar": 1, "dip": -1},
                  "calm": {"dip": 1, "intr": 1}, "honest": {"dip": 1, "intr": -2}, "deceitful": {"intr": 2},
                  "diligent": {"all": 1}, "lazy": {"all": -1},
                  "gregarious": {"dip": 2}, "shy": {"dip": -2, "lrn": 1}, "lustful": {"intr": 1}, "chaste": {"lrn": 1},
                  "gluttonous": {"ste": -1}, "temperate": {"ste": 1}, "arrogant": {"dip": -1, "mar": 1},
                  "humble": {"dip": 1}, "forgiving": {"dip": 1, "intr": -1}, "vengeful": {"intr": 2, "dip": -1},
                  "patient": {"lrn": 1, "ste": 1}, "impatient": {"mar": 1, "lrn": -1}, "just": {"ste": 2, "intr": -1},
                  "arbitrary": {"intr": 2, "ste": -1}, "compassionate": {"dip": 2, "intr": -2},
                  "callous": {"intr": 2, "dip": -1}, "stubborn": {"ste": 1}, "fickle": {"dip": 1, "ste": -1},
                  }
ATTRS = ("dip", "mar", "ste", "intr", "lrn", "lore")
ATTR_NAMES = {"dip": "Diplomacy", "mar": "Martial", "ste": "Stewardship", "intr": "Intrigue", "lrn": "Learning",
              "lore": "Storytelling"}

# ---------------------------------------------------------------- legacies
# Earned from deeds, but only as far as the deeds are told: points = renown x storytelling. Passed to children
# at half strength; royal and noble houses carry a legacy track from their origin.
LEGACY = {  # kind: (name, effect per level)
    "warrior": ("the Sword", {"prowess": 2, "mar": 1}),
    "slayer": ("the Slayer", {"vs_monster": 0.08}),
    "arcane": ("the Arcane", {"talent": 0.4}),
    "delver": ("the Deep Roads", {"delve": 0.08}),
    "ruler": ("the Crown", {"legit": 5, "dip": 1}),
    "guardian": ("the Shield", {"guard": 0.08}),
    # anyone can be famous for being very good at something, and pass it down
    "smith": ("the Forge", {"skill:Smithing": 5}),
    "enchanter": ("the Rune-Word", {"skill:Enchanting": 5}),
    "alchemist": ("the Alembic", {"skill:Alchemy": 5}),
    "merchant": ("the Golden Hand", {"skill:Mercantile": 5}),
    "prophet": ("the Prophet", {"skill:Speech": 5}),
    "thief": ("the Shadow", {"skill:Sneak": 5, "skill:Security": 3}),
    "assassin": ("the Black Feather", {"skill:Sneak": 4, "skill:Blade": 3}),
    "shinobi": ("the Hidden Blade", {"ki": 15}),
    "bender": ("the Element", {"bend": 5}),
    "concord": ("the Concord", {"bend": 10}),
    "explorer": ("the Compass", {"skill:Navigation": 6}),
    "shipwright": ("the Keel", {"skill:Seamanship": 5}),
    "pirate": ("the Black Flag", {"skill:Seamanship": 4, "skill:Blade": 3}),
    "admiral": ("the Fleet", {"skill:Seamanship": 5}),
}
LEGACY_LEVELS = [100, 300, 700, 1500, 3000]
HOUSE_LEGACY_LEVELS = [200, 600, 1500, 3500, 8000]
DEED_LEGACY = {"war": "warrior", "troll": "slayer", "wraith": "slayer", "demon": "slayer", "dragon": "slayer",
               "calamity": "slayer", "delve": "delver", "guard": "guardian", "study": "arcane", "rite": "arcane"}

# ---------------------------------------------------------------- monster life
MONSTER_LIFE = {"troll": 150, "wraith": 400, "demon": 600, "dragon": 1500, "beast": 60}
DRAGON_TRAITS = ["fire-wyrm", "frost-wyrm", "storm-wyrm", "venom-wyrm", "elder", "brood-mother"]

# ---------------------------------------------------------------- items
RARITY = ["Common", "Fine", "Rare", "Epic", "Legendary", "Relic", "Worldforged"]
RARITY_BONUS = [0.05, 0.1, 0.2, 0.35, 0.6, 1.0, 2.0]
ITEM_A = ["Sword", "Crown", "Staff", "Ring", "Shield", "Horn", "Spear", "Amulet", "Helm", "Lantern", "Axe", "Bow",
          "Cloak", "Gauntlet", "Tome", "Mail"]
ITEM_B = ["Dawn", "the Deep", "Ages", "Embers", "Whispers", "Kings", "the Moon", "Thunder", "Winter",
          "the First Tree", "Sorrow", "Ash", "Stars", "the Drowned", "Iron", "Saints", "the Tide", "Hunger"]
WORLDFORGED = [("the Crown of the First Dawn", "tier", "raises its bearer's magic one tier"),
               ("the Hourglass Without Sand", "undying", "its bearer does not age"),
               ("the Ninefold Mirror", "guard", "doubles its bearer's guard"),
               ("the Black Grail", "magic", "magic half again as strong"),
               ("the Spear That Remembers", "vs_monster", "doubles its bearer's strength against monsters"),
               ("the Heartwood Seed", "food", "its bearer's people reap a fifth more"),
               ("the Key of Hollow Gates", "rite", "opens the World Rites at the eighth tier"),
               ("the Unwritten Codex", "rite", "opens the World Rites at the eighth tier")]

# ---------------------------------------------------------------- monsters
# kind: (terrain or "mana", power range, raid range, raid chance, pop lost on a successful raid, cap, renown,
#        spawn chance)
MONSTER = {
    "troll": (None, (80, 250), 8, 0.40, 0.10, 10, 10, 0.10),
    "wraith": (SWAMP, (300, 900), 10, 0.15, 0.12, 4, 30, 0.035),
    "demon": ("mana", (800, 2500), 14, 0.20, 0.15, 3, 60, 0.012),
    "dragon": (MOUNTAIN, (1500, 4000), 18, 0.15, 0.15, 3, 150, 0.012),
    "beast": (None, (300, 3000), 10, 0.25, 0.06, 0, 40, 0.0),     # ecosystem alphas: never spawned by chance
}
CALAMITY_A = ["Azh", "Mol", "Ygg", "Xar", "Uth", "Neph", "Kor", "Zhu", "Vel", "Orm"]
CALAMITY_B = ["mereth", "oloth", "ugoth", "arax", "shul", "emnon", "azzur", "ithra"]
CALAMITY_TITLE = ["the Hunger Below", "the Sleeper in the Deep", "the King of Ash", "the Pale Mother",
                  "the Eater of Stars", "the Worm That Waits", "the Thousand-Eyed", "the Unmaker"]

# ---------------------------------------------------------------- names
TOWN_SYL = {
    HUMANS: (["Ash", "Bran", "Cor", "Dun", "Ed", "Hal", "Kings", "Mar", "Ox", "Ral", "Stan", "Wel", "Thorn", "Lyn",
              "Bel", "Ald", "Har", "Cold"],
             ["ford", "ton", "bury", "wick", "field", "haven", "mere", "stead", "gate", "holm", "bridge", "moor"]),
    ELVES: (["Ael", "Cel", "Ela", "Ili", "Lor", "Mel", "Sil", "Tha", "Yse", "Nim", "Ara", "Eri", "Fal", "Ith"],
            ["andor", "ethil", "ion", "wen", "loth", "dell", "riel", "thas", "lind", "athil", "orien", "vanna"]),
    DWARVES: (["Bar", "Dur", "Grim", "Khaz", "Thor", "Bal", "Kar", "Dol", "Gund", "Mor", "Az", "Bur", "Nar"],
              ["dum", "grad", "hold", "forge", "delve", "heim", "kar", "dun", "bad", "zul", "ak"]),
    ORCS: (["Gor", "Mug", "Ruk", "Snag", "Ugr", "Zug", "Grash", "Lug", "Krag", "Bol", "Shag", "Durz", "Ogg"],
           ["mash", "gash", "rok", "thak", "nar", "dug", "skar", "gul", "burz", "lok", "grub"]),
}
FIRST = {  # (male, female, either) first-name syllables per people
    HUMANS: (["Ed", "Al", "Ber", "Gar", "Hen", "Os", "Ro", "Wil", "Ced", "Bri", "Tom", "Ger"],
             ["Isa", "Mae", "El", "Ros", "Gwen", "Mar", "Ad", "Cat", "Lia", "Ann"],
             ["ric", "wyn", "mund", "ard", "win", "bert", "an", "ald", "mer", "ias"],
             ["a", "elle", "ys", "ora", "wyn", "ith", "ine", "eth", "issa", "anne"]),
    ELVES: (["Aer", "Cal", "Fin", "Gal", "Ith", "Thra", "Ys", "Eol", "Lor", "Ruin"],
            ["Ama", "Eil", "Lae", "Nae", "Sae", "Ilia", "Ela", "Mir", "Ara", "Nim"],
            ["ion", "dir", "las", "thil", "nor", "orn", "ael", "andil", "ethor", "ion"],
            ["wen", "riel", "iel", "andra", "ise", "eth", "ara", "ithil", "ielle", "loth"]),
    DWARVES: (["Bof", "Dwa", "Gim", "Thra", "Bal", "Dag", "Kil", "Nor", "Sig", "Bru", "Gun", "Tor"],
              ["Dis", "Hel", "Bry", "Gud", "Sig", "Ast", "Ing", "Thy", "Ger", "Ran"],
              ["in", "ur", "li", "grim", "rek", "rid", "mar", "dain", "ok", "ar"],
              ["hild", "dis", "a", "run", "veig", "rid", "dis", "gerd", "ny", "laug"]),
    ORCS: (["Gash", "Ugl", "Mor", "Shak", "Grub", "Azg", "Bolg", "Narz", "Ruk", "Yaz", "Krom", "Dur"],
           ["Sha", "Ugra", "Mog", "Gar", "Yaz", "Ruka", "Bash", "Ogra", "Zug", "Lug"],
           ["nak", "ob", "ul", "rath", "duk", "mog", "ash", "gor", "uk", "bag"],
           ["ka", "ga", "ra", "sha", "ula", "dra", "gha", "ma", "za", "ba"]),
}
EPITHET_HERO = {"dragon": "Dragonslayer", "troll": "Trollbane", "wraith": "Wraithbane", "demon": "Demonbane",
                "calamity": "Worldsaver", "delve": "the Delver", "war": "the Warlord", "guard": "the Shield",
                "rite": "the World-Speaker", "study": "the Learned"}
PLAGUES = ["the Grey Cough", "the Red Fever", "the Sweating Sickness", "the Blackrot", "the Wasting",
           "the Pale Death"]
WILD_A = ["Grey", "Black", "Howling", "Weeping", "Hollow", "Broken", "Iron", "Mist", "Crow", "Bitter", "Dead"]
WILD_B = {FOREST: ["Wood", "Thicket", "Weald"], HILLS: ["Tors", "Downs", "Crags"], SWAMP: ["Fen", "Mire", "Bog"],
          PLAINS: ["Fields", "Barrows"], DESERT: ["Waste", "Sands"], MOUNTAIN: ["Peaks", "Teeth"]}
DRAGON_SYL = (["Vyr", "Kal", "Ush", "Mor", "Zar", "Ith", "Sca", "Bal", "Nyx", "Tor", "Aur", "Gla"],
              ["mathax", "ganth", "raxis", "ondor", "thyss", "ghul", "vorn", "oryx", "athrax", "elion"])
DRAGON_EPITHET = ["the Ashen", "the Red", "the Devourer", "the Old", "Goldhoarder", "the Pale", "Stormwing",
                  "the Black", "Kingsbane", "the Sleepless"]
DEMON_SYL = (["Bel", "Ash", "Maz", "Orob", "Gor", "Vas", "Ul", "Sab", "Thez"],
             ["azoth", "tarot", "imar", "uzael", "goroth", "phegor", "ithiel", "nox"])
