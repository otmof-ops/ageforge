"""Tables for the deeper layer: skills, attributes and perks; smithing and enchanting; professions and
guilds; crime; religion; the shinobi clans; the bending tribes and the Concord."""

# ---------------------------------------------------------------- attributes and skills
ATTRIBUTES = ["str", "int", "wil", "agi", "spd", "end", "per", "lck"]
ATTRIBUTE_NAMES = {"str": "Strength", "int": "Intelligence", "wil": "Willpower", "agi": "Agility", "spd": "Speed",
                   "end": "Endurance", "per": "Personality", "lck": "Luck"}
RACE_ATTR = [  # starting bonuses by people
    {"per": 10, "lck": 5, "str": 5},
    {"int": 10, "agi": 10, "spd": 5},
    {"end": 15, "str": 10, "wil": 5},
    {"str": 15, "end": 10, "spd": 5},
]
SKILLS = {  # skill: (governing attribute, group)
    "Blade": ("str", "combat"), "Blunt": ("str", "combat"), "Archery": ("agi", "combat"),
    "Block": ("end", "combat"), "Heavy Armor": ("end", "combat"), "Light Armor": ("spd", "combat"),
    "Destruction": ("wil", "magic"), "Restoration": ("wil", "magic"), "Conjuration": ("int", "magic"),
    "Illusion": ("per", "magic"), "Alteration": ("wil", "magic"),
    "Sneak": ("agi", "stealth"), "Security": ("agi", "stealth"), "Pickpocket": ("agi", "stealth"),
    "Speech": ("per", "social"), "Mercantile": ("per", "social"),
    "Smithing": ("str", "craft"), "Enchanting": ("int", "craft"), "Alchemy": ("int", "craft"),
}
RACE_SKILL = [
    {"Speech": 10, "Mercantile": 10, "Blade": 10, "Restoration": 5},
    {"Archery": 10, "Alteration": 10, "Illusion": 10, "Sneak": 5},
    {"Smithing": 15, "Heavy Armor": 10, "Blunt": 10, "Enchanting": 5},
    {"Blunt": 10, "Heavy Armor": 10, "Block": 10, "Destruction": 5},
]
PERK_AT = [20, 40, 60, 80, 100]
PERKS = {
    "Blade": ["Edge Discipline", "Riposte", "Bladestorm Footwork", "Hamstring Cut", "Thousand Cuts"],
    "Blunt": ["Bonebreaker", "Crushing Weight", "Shieldsplitter", "Earthshaker Swing", "Hammer of the Mountain"],
    "Archery": ["Steady Draw", "Eagle Eye", "Quick Nock", "Windreader", "Arrow That Never Misses"],
    "Block": ["Braced Stance", "Shield Bash", "Turn the Blade", "Wall of One", "Unbreachable"],
    "Heavy Armor": ["Plated", "Iron Gait", "Fortress Body", "Juggernaut's March", "Walking Bastion"],
    "Light Armor": ["Supple Leathers", "Fleet", "Wind-Dancer", "Slip the Blow", "Untouchable"],
    "Destruction": ["Kindling", "Searing Focus", "Overload", "Cascade", "Annihilator"],
    "Restoration": ["Gentle Hands", "Ward of Mercy", "Renewal", "Avert Death", "Lifewell"],
    "Conjuration": ["Bound Blade", "Twin Summons", "Binding Pact", "Lord of Thralls", "Gatecaller"],
    "Illusion": ["Whisper", "Veil", "Mass Glamour", "Mind-Thief", "Dreamlord"],
    "Alteration": ["Stoneflesh", "Featherfall", "Transmute", "Reshape the World", "Masterwright of Matter"],
    "Sneak": ["Soft Step", "Shadow Cloak", "Silent Kill", "Vanish", "Ghost Among Men"],
    "Security": ["Pickfingers", "Tumbler Sense", "Vault-Breaker", "No Door Closed", "Master Key"],
    "Pickpocket": ["Light Fingers", "Purse-Cutter", "Strip the Mark", "Plant Evidence", "Pocket Kingdom"],
    "Speech": ["Silver Words", "Haggler", "Persuasion", "Spellbinding Orator", "Voice That Moves Nations"],
    "Mercantile": ["Shrewd Eye", "Caravan Master", "Trade Network", "Merchant Prince", "Golden Empire"],
    "Smithing": ["Iron Hand", "Steelwright", "Silversteel Secrets", "Starmetal Forging", "Dragonbone Mastery"],
    "Enchanting": ["Rune-Scribe", "Soul Siphon", "Double Weave", "Arcane Smith", "Mythic Inscription"],
    "Alchemy": ["Herbalist", "Physician", "Poisoner", "Concentrated Draughts", "Elixir of Ages"],
}
CLASS_SKILLS = {
    "Warrior": ["Blade", "Blunt", "Heavy Armor", "Block"],
    "Knight": ["Blade", "Block", "Heavy Armor", "Restoration"],
    "Ranger": ["Archery", "Light Armor", "Sneak", "Alteration"],
    "Rogue": ["Blade", "Sneak", "Security", "Pickpocket", "Light Armor"],
    "Wizard": ["Destruction", "Alteration", "Conjuration", "Illusion"],
    "Priest": ["Restoration", "Illusion", "Speech"],
    "Druid": ["Alteration", "Restoration", "Light Armor"],
    "Runesmith": ["Smithing", "Enchanting", "Blunt", "Heavy Armor"],
    "Shaman": ["Conjuration", "Destruction", "Blunt"],
    "Necromancer": ["Conjuration", "Destruction", "Illusion"],
}
SCHOOL_SKILL = ["Destruction", "Restoration", "Alteration", "Alteration", "Conjuration", "Conjuration"]
WEAPON_SKILL = {"Warrior": "Blade", "Knight": "Blade", "Ranger": "Archery", "Rogue": "Blade",
                "Runesmith": "Blunt", "Shaman": "Blunt", "Druid": "Blunt", "Priest": "Blunt", "Wizard": "Blade",
                "Necromancer": "Blade"}

# ---------------------------------------------------------------- smithing and enchanting
MATERIALS = [  # name, smithing needed, bonus
    ("Bronze", 0, 0.04), ("Iron", 15, 0.08), ("Steel", 30, 0.14), ("Silversteel", 50, 0.24),
    ("Starmetal", 70, 0.38), ("Adamant", 85, 0.55), ("Dragonbone", 100, 0.8)]
QUALITY = [("Crude", 0.8), ("Fine", 1.0), ("Superior", 1.15), ("Exquisite", 1.3), ("Flawless", 1.5),
           ("Legendary", 1.8)]
WEAPONS = [("Longsword", "Blade"), ("Dagger", "Blade"), ("Greatsword", "Blade"), ("War Axe", "Blunt"),
           ("Mace", "Blunt"), ("Warhammer", "Blunt"), ("Longbow", "Archery"), ("Spear", "Blade")]
ARMORS = [("Plate", "Heavy Armor"), ("Mail", "Heavy Armor"), ("Scale", "Heavy Armor"), ("Leathers", "Light Armor"),
          ("Brigandine", "Light Armor"), ("Shield", "Block")]
ENCHANT_TIERS = [("Minor", 0, 0.05), ("Lesser", 20, 0.1), ("Greater", 45, 0.2), ("Grand", 70, 0.35),
                 ("Mythic", 90, 0.6)]
ENCHANT_WEAPON = ["Flame", "Frost", "Storm", "Draining", "Banishing", "Soulbinding"]
ENCHANT_ARMOR = ["Warding", "Vigor", "Swiftness", "Spellbreaking", "Fortune", "Thorns"]
SOULSTONE = {"troll": 1, "wraith": 2, "demon": 3, "dragon": 4, "calamity": 4, "sea serpent": 2, "kraken": 3,
             "leviathan": 4, "beast": 2}  # best enchant tier it opens

# ---------------------------------------------------------------- professions and guilds
PROFESSIONS = {  # profession: (cap per people, primary skills, legacy kind)
    "smith": (4, ["Smithing", "Blunt"], "smith"),
    "enchanter": (3, ["Enchanting", "Alteration"], "enchanter"),
    "alchemist": (2, ["Alchemy", "Restoration"], "alchemist"),
    "merchant": (3, ["Mercantile", "Speech"], "merchant"),
    "priest": (3, ["Speech", "Restoration"], "prophet"),
    "thief": (3, ["Sneak", "Security", "Pickpocket"], "thief"),
    "assassin": (2, ["Sneak", "Blade", "Alchemy"], "assassin"),
    "bandit": (2, ["Blade", "Archery", "Light Armor"], "thief"),
}
GUILD_KINDS = {  # kind: (name pattern, who joins)
    "fighters": "the {} Fighters' Guild", "mages": "the {} College of Mages", "smiths": "the {} Smiths' Guild",
    "merchants": "the {} Merchants' League", "thieves": "the {} Thieves' Guild",
}
GUILD_RANKS = ["Initiate", "Journeyman", "Adept", "Expert", "Master"]
ASSASSINS = "the Quiet Hand"

# ---------------------------------------------------------------- religion
BASE_FAITHS = [("the Faith of the Dawn", "the Dawnfather"), ("the Old Songs", "the Singer Beneath the Leaves"),
               ("the Ancestor Halls", "the First Smith"), ("the Blood Spirits", "the Red Ancestors")]
TENETS = {
    "pacifism": "the faithful shall not make war first",
    "holy war": "the unbeliever shall be brought to the faith by the sword",
    "charity": "the faithful feed the poor, and crime withers",
    "sacred kingship": "the crown is holy, and the faithful obey it",
    "ancestor worship": "the dead are remembered, and their legacies grow",
    "wealth gospel": "riches are a sign of grace",
    "martyrdom": "to die for the faith is the highest glory",
    "magic is sin": "the tiers of magic are forbidden",
    "dragon veneration": "the dragons are gods, and are not to be hunted",
    "the Concord awaits": "the Concord will come again, and the faithful prepare its way",
}
FAITH_A = ["Path", "Way", "Church", "Covenant", "Order", "Creed", "Light", "Truth", "Circle"]
DEITY_A = ["the Burning", "the Silent", "the Weeping", "the Hollow", "the Golden", "the Nameless", "the Ninefold",
           "the Last", "the Sleeping", "the Iron"]
DEITY_B = ["Mother", "King", "Eye", "Flame", "Star", "Serpent", "Tree", "Wolf", "Moon", "Judge"]

# ---------------------------------------------------------------- shinobi
SHINOBI_CLANS = ["the Hollow Pine Clan", "the Ninefold Mist Clan", "the Clan of Falling Ash"]
SHINOBI_RANKS = ["Initiate", "Blade", "Shadow", "Master", "Grandmaster"]
SHINOBI_HEAD = "the Veiled Lord"
NATURES = {  # nature: five techniques, tier I to V
    "Flame": ["Ember Spit", "Coiling Fire Serpent", "Phoenix Wing Volley", "Inferno Lotus", "Sun-Eater Pyre"],
    "Gale": ["Razor Breath", "Twin Cyclone Palm", "Thousand Wind Needles", "Vacuum Lance", "Heaven-Splitting Gale"],
    "Thunder": ["Spark Fingers", "Lightning Hound", "Thunderclap Step", "Storm Crown", "Sky-Father's Spear"],
    "Stone": ["Mud Wall", "Stone Spear Field", "Earth-Swallow", "Mountain Fist", "Continent Turtle"],
    "Tide": ["Water Whip", "Rain of Needles", "Drowning Prison", "Leviathan Wave", "Ocean's Judgement"],
}
ARTS = {
    "Body": ["Iron Palm", "Hundred-Strike Flurry", "Gate of Pain", "Dragon-Spine Kick", "Eighth Gate Release"],
    "Illusion": ["Flicker of Fear", "Mirror Path", "Waking Nightmare", "Moon-Prison Dream", "World of the Eye"],
    "Seal": ["Binding Tag", "Chain Seal", "Spirit Lock", "Reaper's Seal", "Eternity Seal"],
    "Summoning": ["Call the Ravens", "Great Serpent Pact", "Moon Hare Host", "Iron Tortoise Elder",
                  "Summon the Sky Whale"],
    "Healing": ["Mending Palm", "Poison Draw", "Cellwork", "Hundred Lives Seal", "Rebirth Rite"],
    "Forbidden": ["Shadow Doubles Unbound", "Life-Burning Gate", "Soul Exchange", "Corpse Puppet Legion",
                  "Death-God's Bargain"],
}
SHINOBI_EYES = {"hollow-eye": "sees the flow of ki itself: learns techniques half again as fast and is never "
                              "deceived by illusion",
                "deepwell": "carries ki reserves ten ninja could not match"}
SUMMON_PACTS = ["the Ravens of the Black Pine", "the Great Serpents", "the Moon Hares", "the Iron Tortoises",
                "the Sky Whales"]

# ---------------------------------------------------------------- bending
ELEMENTS = ["tide", "stone", "flame", "sky"]
ELEMENT_GENE = {"tide": "tidebound", "stone": "stoneborn", "flame": "flameheart", "sky": "skyborn"}
GENE_ELEMENT = {v: k for k, v in ELEMENT_GENE.items()}
TRIBES = {"tide": "the Saltmere Clans of the Frozen Reach", "stone": "the Deepstone Kin of the High Ridges",
          "flame": "the Emberkin of the Burning Waste", "sky": "the Windward Wanderers of the High Peaks"}
TRIBE_TERRAIN = {"tide": "coast", "stone": "mountain", "flame": "desert", "sky": "peak"}
BEND_FORMS = {  # four tiers of forms, then two high arts opened at mastery 70
    "tide": (["Wave-Push", "Ice Lance", "Tidal Coil", "Glacier Fortress"], ["Mending Tide", "Mistweaving"]),
    "stone": (["Stone Toss", "Earthen Wall", "Quake Step", "Mountain Raising"], ["Ore-Calling", "Sand-Singing"]),
    "flame": (["Fire Jab", "Flame Whip", "Blazing Wheel", "Dragon's Breath"], ["Storm-Heart", "Sun-Breath"]),
    "sky": (["Gust", "Air Shield", "Cyclone Dance", "Hurricane Spiral"], ["Sky-Stepping", "Spirit-Walking"]),
}
BEND_TIER_NAMES = ["Novice", "Adept", "Master", "Grandmaster"]
CONCORD = "the Concord"

# ---------------------------------------------------------------- the sea
SEA_SKILLS = {"Seamanship": ("agi", "sea"), "Navigation": ("int", "sea")}
SKILLS.update(SEA_SKILLS)
PERKS["Seamanship"] = ["Sea Legs", "Weather Eye", "Broadside Drill", "Storm Rider", "Master of the Waves"]
PERKS["Navigation"] = ["Star Reckoning", "Chart-Maker", "Current Reader", "Edge of the Map", "Finder of Worlds"]
SHIPS = [("Cog", 0), ("Caravel", 25), ("Carrack", 45), ("Galleon", 65), ("Ship of the Line", 85)]
PROFESSIONS.update({
    "shipwright": (2, ["Seamanship", "Smithing"], "shipwright"),
    "captain": (3, ["Navigation", "Seamanship"], "explorer"),
})
SEA_MONSTERS = {  # kind: (power range, cap per ocean share, renown, spawn chance, lifespan)
    "sea serpent": ((500, 1500), 6, 30, 0.05, 300),
    "kraken": ((1500, 4000), 4, 120, 0.02, 800),
    "leviathan": ((5000, 12000), 2, 400, 0.004, 3000),
}
SEA_MONSTER_NAMES = (["Thal", "Mor", "Ysh", "Kral", "Vor", "Ulth", "Nag", "Sca", "Gol", "Bel"],
                     ["assa", "ugoth", "ira", "ekk", "umos", "ath", "ithra", "orrak", "ean"])
PIRATE_SHIP_A = ["Crimson", "Black", "Laughing", "Drowned", "Wicked", "Silver", "Howling", "Bloody", "Golden",
                 "Restless", "Grinning", "Salt", "Ragged", "Mad", "Faithless", "Lucky"]
PIRATE_SHIP_B = ["Widow", "Gull", "Maiden", "Serpent", "Revenge", "Bride", "Fortune", "Shark", "Wench", "Kraken",
                 "Jest", "Lantern", "Debt", "Saint", "Crow", "Promise"]
PIRATE_EPITHET = ["Redbeard", "the Butcher of the Shallows", "Blacktooth", "the Gentleman", "Saltblood",
                  "the Grey Ghost", "One-Eye", "the Smiling Knife", "Cutlass", "the Widowmaker", "Halfhand",
                  "the Drowned Queen", "Stormcrow", "the Merry", "Ironhook", "Gold-Tooth"]
HAVEN_STYLE = ["Free Port of {}", "{} Cove", "Hangman's {}", "{} Harbour"]
PIRATE_ORIGINS = [
    "a fisher's child of {town}, who wanted more than nets",
    "a deserter from the fleet of {realm}",
    "a sailor pressed into the navy of {realm}, who mutinied on the first voyage",
    "a freed galley slave, chained once on a ship of {realm}",
    "a smuggler of {town} whose luck ran out ashore",
    "a ruined merchant of {town}, owing more gold than the sea could hold",
    "a shipwright's apprentice of {town} who stole the ship she built",
    "the disinherited youngest child of a house of {realm}",
]

# ---------------------------------------------------------------- beast taming
SKILLS["Beast Taming"] = ("per", "wild")
PERKS["Beast Taming"] = ["Soft Voice", "Pack Sense", "Sky-Caller", "Heart of the Wild", "One Blood"]
BEASTS = {  # kind: (taming needed, companion power, lifespan)
    "wolf": (15, 80, 15), "bear": (30, 220, 30), "eagle": (40, 260, 40), "griffin": (55, 650, 80),
    "wyvern": (70, 1300, 150), "dragon": (90, 3000, 1500),
}
PROFESSIONS["tamer"] = (2, ["Beast Taming", "Archery"], "tamer")

# ---------------------------------------------------------------- legends and curses
CURSES = {
    "blood": "a curse on a bloodline: its children sicken, its lords go mad, its heirs die young",
    "land": "a curse on the land: the soil withers, the game flees, the dead do not rest",
    "item": "a curse on a weapon: it grants power and takes the bearer's mind",
    "beast": "a curse of the hunt: the kin of the slain beast hunt the slayer's line",
    "cycle": "a curse of return: the cursed one swears to come back in every age to take what it was denied",
}
CYCLE_TITLES = ["the Returning King", "the Unending", "the Eternal Shadow", "the Thrice-Born", "the Undying Claim",
                "the One Who Comes Again"]
HERO_MARK = "the Hero's Mark"
SEER_GIFT = "the Sealing Light"
LEGEND_MATERIALS = {"heartstone": "the heartstone of {}", "star-iron": "star-iron from a falling star over {}",
                    "sovereign-bone": "the bones of {}, Wyrm-Sovereign"}
OATH_A = ["Oathblade", "Bane", "Doomsworn", "Last Light", "Sealbreaker", "Ever-Return", "Dawnbringer",
          "Spiritsunder", "Wardsword", "Fate's Edge"]
