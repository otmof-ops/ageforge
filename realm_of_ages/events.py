"""Every event the world records, sorted into categories for browsing; and the ledger of money moving."""
import re

CATEGORIES = ["god", "war", "politics", "economy", "people", "lives", "culture", "heart", "magic", "monsters", "heroes",
              "sea", "faith", "crime", "nature", "misc"]
RULES = [
    ("god", r"\*\*god\b|by the gods' hand|divine"),
    ("politics", r"is crowned|a new dynasty|new seat|succession war|swears fealty|is born\b.*realm|regent"),
    ("crime", r"heist|quiet hand|assassin|bounty|hanged|thieves|shinobi|needle-mark|black feather"),
    ("sea", r"pirate|colony|sights |charts |naval|fleet|privateer|mutiny|the battle of (the )?(sea|gulf|straits|deeps|shallows)|kraken|serpent|leviathan|haven"),
    ("people", r"rising|commune|pestilence|burns out|plague|sickness|fever|cough|flux|pox|wasting|migrat"),
    ("war", r"\*\*war\.\*\*|battle of|siege|host of|white peace|wins its|throws back|warscore|musters|"
            r"declares war|peace\.\*\*|vengeance for"),
    ("economy", r"famine|harvest|bankrupt|buys |forges|great work|market|treasury|gold"),
    ("heart", r"love|lovers|marry|married|scandal|swears vengeance|vengeance on|grief|goes mad|cracks under|"
              r"friends|rivals|affair|a child is born"),
    ("magic", r"tier|rite|spell|blood wakes|curse|legendary forging|returns\*\*|seals|casts down|concord|lich|"
              r"the fall\.|bending|bender|sworn against"),
    ("nature", r"extinction|new breed|alpha of|species"),
    ("monsters", r"dragon|troll|wraith|demon|calamity|clutch|wyrm|beast|sinks back"),
    ("heroes", r"slays|\bdelves?\b|the delver|bottom of|floors? |\bhero|artifact|relic|dungeon|masters |level"),
    ("faith", r"faith|proclaims|prophet|religion|holy war|temple|heresy"),
    ("culture", r" learn |tradition|lifestyle|golden age|dark age|new era|saga|thinkers|old ways"),
    ("lives", r"died of|died in|is born|midwives|comes of age|aged \d+"),
    ("politics", r"crown|dynasty|faction|ultimatum|new realm|duke|duchess|council|regent|succession|throne|is no more|"
                 r" found |guildmaster|cadet branch|hook on|"
                 r"nobility|royal court|empire|unites|tributary|new order|reform|charter|tyrant|purge|rises|"
                 r"chiefdom|realm|kingdom|alliance|abdicat|deposed|usurp"),
]
COMPILED = [(c, re.compile(p, re.I)) for c, p in RULES]


def classify(text):
    for cat, rx in COMPILED:
        if rx.search(text):
            return cat
    return "misc"


def record(realm, text, minor, cat=None):
    if not hasattr(realm, "event_log"):
        realm.event_log = []
    cat = cat or classify(text)
    realm.event_log.append((realm.year, cat, not minor, text))
    # tracked characters keep a journal of everything that names them
    for cid in getattr(realm, "tracked", ()):
        c = realm.chars.get(cid)
        if c is not None and c.name in text and (c.full_name() in text or c.name + " " in text):
            journal(realm, c, text, cat)


def journal(realm, c, text, cat="misc"):
    if not getattr(c, "tracked", False):
        return
    if not hasattr(c, "journal"):
        c.journal = []
    if c.journal and c.journal[-1][0] == realm.year and c.journal[-1][2] == text:
        return
    c.journal.append((realm.year, cat, text))


# ---------------------------------------------------------------- the ledger
LEDGER_CATS = ["taxes", "tribute in", "loot", "indemnity in", "tribute out", "indemnity out", "army upkeep",
               "regiments", "field armies", "muster", "court", "interest", "decisions", "great works", "default"]


def book(realm, kid, cat, amount, note=None, who=None):
    """Money moving into (+) or out of (-) a realm's treasury, kept by year; notable movements kept by name."""
    if kid is None or amount == 0:
        return
    if not hasattr(realm, "ledger"):
        realm.ledger, realm.tx = {}, []
    row = realm.ledger.setdefault((realm.year, kid), {})
    row[cat] = row.get(cat, 0.0) + amount
    if note is not None or abs(amount) >= 1000:
        realm.tx.append((realm.year, kid, cat, float(amount), note or "", who))
        if len(realm.tx) > 20000:
            del realm.tx[:5000]


def private(realm, who, cat, amount, note):
    """A transaction between people rather than crowns: a sale, a bounty, a prize."""
    if not hasattr(realm, "ledger"):
        realm.ledger, realm.tx = {}, []
    realm.tx.append((realm.year, getattr(who, "realm", None), cat, float(amount), note, getattr(who, "id", None)))
    if len(realm.tx) > 20000:
        del realm.tx[:5000]
