"""Rulers and their courts, after the feudal model.

Base stats: Diplomacy, Martial, Stewardship, Intrigue, Learning and Prowess. Each comes from inborn traits,
personality, and three things that change across a life:
  - education: at coming of age everyone gains an education in one stat, at one of four tiers, decided by
    their own gifts, the years they practised in youth, their diligence and, for royal children, the court
    tutor (+1, +3, +5 or +7 to that stat);
  - a lifestyle: adults pursue one stat's lifestyle and earn its perks, three trees of three perks each;
  - age: past two-thirds of a lifespan the stats begin to fail.

A realm that becomes a kingdom or an empire (and is not tribal, a republic or a theocracy) holds a royal court.
The court has a grandeur: its amenities (food, lodging, fashion, servants), its filled positions, the
artifacts on display and the events it holds. A kingdom is expected to keep a grand enough court, an empire
grander; falling short costs legitimacy and the dukes' regard, exceeding it earns both."""
from .data import ADULT, ATTRS, PEOPLES

FOCI = ["dip", "mar", "ste", "intr", "lrn"]
EDUCATION = {
    "dip": ["Hesitant Envoy", "Able Envoy", "Gifted Diplomat", "Voice of Nations"],
    "mar": ["Green Soldier", "Seasoned Captain", "Field Tactician", "Master of War"],
    "ste": ["Thrifty Clerk", "Careful Steward", "Shrewd Treasurer", "Architect of Realms"],
    "intr": ["Whisperer", "Schemer", "Master of Secrets", "Shadow of the Throne"],
    "lrn": ["Novice Scholar", "Learned Reader", "Sage", "Keeper of All Knowledge"],
}
EDU_BONUS = [1, 3, 5, 7]
LIFESTYLES = {  # stat: [(tree, effect, [three perks])]
    "dip": [("Statecraft", "dip", ["Gracious Host", "Silver Envoy", "Peacemaker"]),
            ("Family", "fertility", ["Matchmaker", "Kin-Binder", "Elder's Voice"]),
            ("Majesty", "legit", ["Regal Bearing", "Sovereign Presence", "Crown of Hearts"])],
    "mar": [("Strategy", "battle", ["Ambusher", "Flanker", "Grand Strategist"]),
            ("Chivalry", "prowess", ["Sworn Sword", "Knight-Commander", "Paragon of Arms"]),
            ("Authority", "opinion", ["Iron Discipline", "Warden of Borders", "Conqueror's Will"])],
    "ste": [("Administration", "vassal", ["Census-Taker", "Tax Assessor", "Grand Administrator"]),
            ("Domain", "domain", ["Land Steward", "Many Halls", "Realm-Builder"]),
            ("Commerce", "tax", ["Market Rights", "Guild Patron", "Merchant Prince"])],
    "intr": [("Shadows", "guard", ["Eavesdropper", "Poison Lore", "Unseen Hand"]),
             ("Schemes", "intr", ["Blackmailer", "Kingmaker", "Puppet Master"]),
             ("Fear", "fear", ["Hard Gaze", "Terrifying", "Dread Lord"])],
    "lrn": [("Lore", "research", ["Archivist", "Natural Philosopher", "Master of Lore"]),
            ("Faith", "faith", ["Devout", "Pilgrim", "Living Scripture"]),
            ("Medicine", "health", ["Herbalist", "Surgeon", "Healer of Kings"])],
}
PERK_VALUE = {"dip": 2, "fertility": 0.15, "legit": 4, "battle": 0.05, "prowess": 3, "opinion": 5, "vassal": 3,
              "domain": 1, "tax": 0.05, "guard": 4, "intr": 2, "fear": 0.2, "research": 0.05, "faith": 0.05,
              "health": 0.2}
POSITIONS = {  # position: how to choose the best candidate
    "Champion": lambda r, c: c.eff_prowess() + c.level / 3,
    "Court Physician": lambda r, c: c.skills.get("Restoration", 0) + c.attr("lrn"),
    "Master of the Hunt": lambda r, c: c.skills.get("Archery", 0) + c.skills.get("Beast Taming", 0),
    "Court Poet": lambda r, c: c.attr("lore") * 5,
    "Royal Architect": lambda r, c: c.attr("ste") * 5,
    "Antiquarian": lambda r, c: c.attr("lrn") * 4,
    "Court Tutor": lambda r, c: c.attr("lrn") * 3 + c.attr("dip") * 2,
    "Bodyguard": lambda r, c: c.eff_prowess() * 2,
}
COURT_TYPES = {"mar": "Warlike", "dip": "Diplomatic", "ste": "Administrative", "intr": "Intrigue", "lrn": "Scholarly"}
AMENITIES = ["food", "lodging", "fashion", "servants"]
EXPECTED = {"kingdom": 30, "empire": 50, "hegemony": 70}


# ---------------------------------------------------------------- base stats
def stat_bonus(realm, c, a):
    v = 0
    edu = getattr(c, "education", None)
    if edu is not None and edu[0] == a:
        v += EDU_BONUS[edu[1]]
    if a in ("dip", "intr"):
        v += perk_value(c, a)
    return v                                   # (the decline of age is applied live, in Character.attr)


def age_penalty(realm, c):
    if c.undead:
        return 0
    frac = c.age(realm.year) / max(1.0, c.life)
    return max(0, int((frac - 0.66) * 25))


def perk_value(c, effect):
    return sum(PERK_VALUE[effect] for (_, eff, perk) in getattr(c, "perk_list", []) if eff == effect)


def educate(realm, c, parents=()):
    """Coming of age: an education in one stat, at a tier earned by gift, practice, diligence and tutoring."""
    if getattr(c, "education", None) is not None:
        return
    rng = realm.rng
    pool = {a: c.attrs.get(a, 5) for a in FOCI}
    for p in parents:
        best = max(FOCI, key=lambda a: p.attrs.get(a, 5))
        pool[best] += 2
    focus = max(pool, key=lambda a: pool[a] + rng.random() * 3)
    roll = rng.random() * 3 + getattr(c, "practice", 0) * 0.25 + 1.0 * ("diligent" in c.personality) \
        - 1.0 * ("lazy" in c.personality) + 0.5 * ("genius" in c.genetic) + getattr(c, "tutored", 0)
    tier = 0 if roll < 1.6 else 1 if roll < 3.0 else 2 if roll < 4.3 else 3
    c.education = (focus, tier)
    c.lifestyle = focus
    c.ls_xp, c.perk_list = 0.0, []
    c.invalidate()


def lifestyle_year(realm, c):
    if getattr(c, "lifestyle", None) is None or len(c.perk_list) >= 9:
        return
    c.ls_xp += 12 + c.attrs.get(c.lifestyle, 5) + 4 * c.education[1]
    need = 120 * (len(c.perk_list) + 1)
    if c.ls_xp >= need:
        c.ls_xp -= need
        trees = LIFESTYLES[c.lifestyle]
        taken = [p for _, _, p in c.perk_list]
        for tree, eff, perks in sorted(trees, key=lambda t: -sum(1 for p in t[2] if p in taken)):
            nxt = [p for p in perks if p not in taken]
            if nxt:
                c.perk_list.append((tree, eff, nxt[0]))
                c.invalidate()
                if c.ruled is not None and len(nxt) == 1:
                    realm.say(f"{realm.who(c)} completes the {tree} path of the {lifestyle_name(c.lifestyle)} "
                              f"lifestyle: {nxt[0]}.", minor=True)
                break


def lifestyle_name(a):
    return {"dip": "Diplomacy", "mar": "Martial", "ste": "Stewardship", "intr": "Intrigue", "lrn": "Learning"}[a]


def education_name(c):
    e = getattr(c, "education", None)
    return f"{EDUCATION[e[0]][e[1]]} ({lifestyle_name(e[0])} education, tier {e[1] + 1})" if e else "none yet"


def people_year(realm):
    y = realm.year
    for i in list(realm.living):
        c = realm.chars[i]
        if not c.adult(y):
            continue
        if getattr(c, "education", None) is None:
            educate(realm, c, [realm.chars[p] for p in (c.father, c.mother) if p is not None])
        if c.ruled is not None or c.hero or getattr(c, "duchies", None) or c.lordships or c.profession:
            lifestyle_year(realm, c)


# ---------------------------------------------------------------- the royal court
def court_year(realm):
    rng = realm.rng
    y = realm.year
    for k in realm.kingdoms:
        if realm.extinct[k.id] or k.ruler is None:
            continue
        eligible = k.rank in EXPECTED and getattr(k, "gov", "feudal") not in ("tribal", "republic", "theocracy")
        if not eligible:
            continue
        r = realm.chars[k.ruler]
        cap = realm.capital_of(k.id)
        if getattr(k, "court", None) is None:
            k.court = {"since": y, "grandeur": 20.0, "amen": {a: 1 for a in AMENITIES}, "pos": {},
                       "type": None, "events": [], "artifacts": []}
            realm.say(f"**A royal court.** {realm.who(r)} gathers a royal court at {cap.name if cap else k.name}.")
        ct = k.court
        ct["type"] = COURT_TYPES[max(FOCI, key=lambda a: r.attr(a))]
        # amenities: raised when the treasury allows, cut when it does not
        revenue = getattr(k, "income", 0) * getattr(k, "tax_now", 0.2) * getattr(k, "collect", 1.0)
        cost = 1 + sum(v ** 1.5 for v in ct["amen"].values()) / 4 * 0.08 * revenue   # a court lives as the realm can afford
        k.gold -= cost
        from .events import book
        book(realm, k.id, "court", -cost)
        if k.gold > cost * 6:
            low = min(AMENITIES, key=lambda a: ct["amen"][a])
            ct["amen"][low] = min(5, ct["amen"][low] + 1)
        elif k.gold < 0:
            high = max(AMENITIES, key=lambda a: ct["amen"][a])
            ct["amen"][high] = max(1, ct["amen"][high] - 1)
        # positions
        pool = [realm.chars[i] for i in realm.member_ids(k.id) if True and realm.chars[i].adult(y)
                and realm.chars[i].id != r.id and getattr(realm.chars[i], "skills", None)]
        used = set()
        for pos, score in POSITIONS.items():
            cur = realm.chars.get(ct["pos"].get(pos))
            if cur is not None and cur.alive and cur.realm == k.id:
                used.add(cur.id)
                continue
            cands = [c for c in pool if c.id not in used]
            if not cands:
                ct["pos"].pop(pos, None)
                continue
            best = max(cands, key=lambda c: score(r, c))
            ct["pos"][pos] = best.id
            used.add(best.id)
            best.titles.append(f"{pos} of {k.name} from year {y}")
            from .opinion import remember
            remember(realm, best, r, f"made me {pos}", 12, decay=0.5)
            k.gold -= 5
        # what the court has to show
        arts = [it for it in k.treasury if it.rarity >= 3] + [it for it in r.items if it.rarity >= 3]
        ct["artifacts"] = [it.name for it in arts]
        g = (sum(ct["amen"].values()) * 3.5 + 2.5 * len(ct["pos"]) + sum(min(8, it.rarity ** 1.4) for it in arts)
             + 0.05 * (y - ct["since"]))
        ct["grandeur"] = 0.8 * ct["grandeur"] + 0.2 * min(100.0, g)
        exp = EXPECTED.get(k.rank, 30)
        diff = ct["grandeur"] - exp
        r.legitimacy = max(0.0, min(100.0, (r.legitimacy or 50) + diff / 40))
        k.court_mod = diff / 4
        court_events(realm, k, r, ct)
        physician = realm.chars.get(ct["pos"].get("Court Physician"))
        k.physician = physician is not None and physician.alive
        k.bodyguard = realm.chars.get(ct["pos"].get("Bodyguard")) is not None
        tutor = realm.chars.get(ct["pos"].get("Court Tutor"))
        if tutor is not None:
            for kid in r.children:
                kc = realm.chars[kid]
                if kc.alive and not kc.adult(y):
                    kc.tutored = 1 + tutor.attr("lrn") / 20


def court_events(realm, k, r, ct):
    rng = realm.rng
    y = realm.year
    roll = rng.random()
    if roll < 0.04 and ct["amen"]["food"] >= 3:
        r.legitimacy = min(100.0, (r.legitimacy or 50) + 5)
        ct["events"].append((y, "a grand feast"))
        realm.say(f"{realm.who(r)} holds a grand feast at court; the great of {k.name} eat and remember.", minor=True)
        from .heart import add_stress, reluctance
        add_stress(realm, r, reluctance(r, "feast"), "a feast")
    elif roll < 0.08 and "Champion" in ct["pos"]:
        champ = realm.chars[ct["pos"]["Champion"]]
        champ.renown += 40
        r.renown += 20
        ct["events"].append((y, f"a tournament won by {champ.full_name()}"))
        realm.say(f"A great tournament at the court of {k.name}: {realm.who(champ)} carries the day.", minor=True)
    elif roll < 0.11 and "Master of the Hunt" in ct["pos"]:
        r.renown += 25
        ct["events"].append((y, "a royal hunt"))
    elif roll < 0.14 and "Court Poet" in ct["pos"]:
        from .lineage import gain_legacy
        gain_legacy(realm, r, "ruler", 60)
        ct["events"].append((y, "a court poet sings of the ruler"))
    elif roll < 0.17:
        smiths = [realm.chars[i] for i in realm.member_ids(k.id) if True
                  and realm.chars[i].profession in ("smith", "enchanter")]
        if smiths and k.gold > 150:
            from .characters import Item
            from .crafts import master_name
            sm = max(smiths, key=lambda s: s.skills.get("Smithing", 0) + s.skills.get("Enchanting", 0))
            it = Item(master_name(realm), 4 if sm.skills.get("Smithing", 0) < 80 else 5, y,
                      f"made by {sm.full_name()} for the court of {k.name}")
            it._bonus = 0.5 + sm.skills.get("Smithing", 0) / 200
            k.gold -= 150
            k.treasury.append(it)
            from .crafts import gain_fame
            gain_fame(realm, sm, "Smithing", 80)
            ct["events"].append((y, f"{sm.full_name()} is inspired: {it.name}"))
            realm.say(f"{realm.who(sm)}, inspired at the court of {k.name}, makes {it.name} for the throne.")


def courts_and_people_year(realm):
    people_year(realm)
    court_year(realm)
