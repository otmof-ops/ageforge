"""Kingdoms and royal houses: marriage and alliance, birth, succession under four laws, regency, plots,
succession wars, the council, ennoblement, and princes who leave court to adventure."""
import math

import numpy as np

from .characters import House, gain_xp, make_character, roman
from .data import (COLORS, ADULT, FERTILE_END, FERTILITY, GENETIC, HERO_CAP, HOUSE_STYLE, LIFESPAN, PEOPLES, REALM_STYLE,
                   RULER_TITLE, SUCCESSION, TOWN_SYL)
from .heroes import start_hero


class Kingdom:
    def __init__(self, kid, people, name, short=None):
        self.id, self.people, self.name, self.law = kid, people, name, SUCCESSION[people]
        self.short = short or PEOPLES[people]
        self.rank, self.parent, self.founded_year = "kingdom", None, 0
        self.color = None
        self.vassals = {}                       # duchy id -> duke character id
        self.factions = []
        self.ended = None
        self.ruler = self.house = self.regent = None
        self.council = {"marshal": None, "steward": None, "spymaster": None, "mage": None, "loremaster": None,
                        "admiral": None}
        self.warships, self.merchants, self.ship_tier, self.blockaded = 4.0, 6.0, 0, 0
        self.colonies, self.discoveries = 0, []
        self.corruption, self.corrupt_seats, self.fear_until = 0.0, [], 0
        self.faith, self.market, self.soulstones, self.dragonbone = people, [], [0] * 5, 0
        self.potions, self.arms, self.made, self.persecuting = 0.0, 0.0, [], False
        self.treasury = []
        self.reigns = []
        self.regnal = {}
        self.mods = {}
        self.allies = set()
        self.battle_mult = self.growth_mult = self.aggr = 1.0
        self.realm_tier = 0
        self.stats = {"taken": 0, "lost": 0}
        self.civil_wars = 0

    @property
    def realm(self):
        return self.id

    def mod(self, key, year, default=1.0):
        v = self.mods.get(key)
        return v[0] if v and v[1] >= year else default


def new_house(realm, founder, name=None, track=None, origin=None):
    f = founder.people
    if name is None:
        name = realm.name(TOWN_SYL[f])
    h = House(realm.next_hid, HOUSE_STYLE[f].format(name), f, founder.id, realm.year)
    realm.houses[h.id] = h
    realm.next_hid += 1
    founder.house = h.id
    if track is None:
        track = max(founder.legacy, key=founder.legacy.get) if founder.legacy else \
            str(realm.rng.choice(["warrior", "ruler", "arcane", "guardian"]))
    h.track = track
    h.origin = origin or f"founded in year {realm.year} by {founder.full_name()}"
    h.legacy = sum(founder.legacy.values()) * 0.5
    return h


def marry(a, b):
    a.spouse, b.spouse = b.id, a.id


def found_kingdoms(realm, seats):
    """Every people begins as small tribes: a chief, their kin, a seat, and nothing yet above them."""
    rng = realm.rng
    from .civics import style_for
    for pid, (f, pos) in enumerate(seats):
        k = Kingdom(pid, f, "")
        k.rank, k.gov = "tribe", "tribal"
        k.law = "strongest" if f == 3 else "elective"
        base = np.array(COLORS[f], float)
        k.color = COLORS[f] if pid == min(i for i, (g, _) in enumerate(seats) if g == f) else \
            tuple(int(v) for v in np.clip(base * rng.uniform(0.6, 1.3, 3), 30, 255))
        realm.kingdoms.append(k)
        realm.grow_tables()
        cap = realm.add_settlement(pid, pos, int(rng.integers(120, 260)), capital=True, quiet=True)
        k.place = cap.name
        k.name = style_for(k, "tribe", cap.name)
        k.short = f"{PEOPLES[f]} of {cap.name}"
        realm.home_land[pid] = cap.landmass
        realm.known[pid] = {cap.landmass}
        age = ADULT[k.people] + int(rng.integers(5, max(6, ADULT[k.people])))
        ruler = make_character(realm, pid, born=-age)
        house = new_house(realm, ruler)
        house.prestige = 30.0
        gain_xp(ruler, 100 + 20 * min(age, 60), realm)
        sp = make_character(realm, pid, born=-(age + int(rng.integers(-5, 6))), female=not ruler.female)
        marry(ruler, sp)
        dad, mum = (sp, ruler) if ruler.female else (ruler, sp)
        for _ in range(int(rng.integers(1, 3))):
            kid_age = int(rng.integers(0, max(1, age - ADULT[k.people])))
            kid = make_character(realm, pid, born=-kid_age, father=dad, mother=mum, house=house.id)
            gain_xp(kid, 20 * kid_age, realm)
        house.track, house.origin = "ruler", f"chiefs of {k.name}, from year 0"
        crown(realm, k, ruler, "the first of the line")
        n = make_character(realm, pid, born=-(ADULT[k.people] + int(rng.integers(0, 15))))
        new_house(realm, n)
        gain_xp(n, 300, realm)
    # power bloodlines run in some houses of the world: two shown, two carried unseen
    from .lineage import seed_gene
    founders = [realm.chars[h.founder] for h in realm.houses.values()]
    picks = rng.permutation(len(founders))
    for i, (trait, shown) in enumerate((("dragonblood", True), ("starblood", True), ("titanblood", False),
                                        ("elderblood", False))):
        c = founders[int(picks[i % len(founders)])]
        if shown:
            seed_gene(realm, c, trait)
            realm.say(f"It is whispered that {trait.replace('blood', ' blood')} runs in {realm.houses[c.house].name}.")
        else:
            c.latent.add(trait)


def court(realm, k):
    """The ruler and those close to the throne: the ones the crown marries off."""
    r = realm.chars.get(k.ruler)
    if r is None:
        return []
    ids = {r.id} | set(r.children)
    for kid in r.children:
        ids |= set(realm.chars[kid].children)
    for p in (r.father, r.mother):
        if p is not None:
            ids |= set(realm.chars[p].children)
    return [realm.chars[i] for i in ids if realm.chars[i].alive]


# ---------------------------------------------------------------- succession
def primo_heir(realm, old):
    f = old.people

    def line(c, skip):
        for kid in sorted((realm.chars[i] for i in c.children), key=lambda x: x.born):
            if kid.id == skip:
                continue
            if kid.alive and kid.people == f:
                return kid
            got = line(kid, skip)
            if got is not None:
                return got
        return None
    heir = line(old, None)
    up, skip = old, old.id
    for _ in range(4):
        if heir is not None:
            break
        parent = realm.chars.get(up.father) if up.father is not None else None
        if parent is None or (old.house is not None and parent.house != old.house):
            parent = realm.chars.get(up.mother) if up.mother is not None else None
        if parent is None:
            break
        heir = line(parent, skip)
        up, skip = parent, parent.id
    return heir


def adults_of(realm, f, house=None):
    y = realm.year
    return [c for c in realm.members(f) if c.adult(y) and (house is None or c.house == house)]


def claim_score(realm, c):
    hp = realm.houses[c.house].prestige / 100 if c.house is not None else 0
    k = realm.kingdoms[c.realm]
    blood = 8 if (k.house is not None and c.house == k.house) else 0
    return (c.attr("lrn") + c.attr("dip") + 0.5 * c.attr("mar") + c.level / 6 + min(10, c.renown / 80) + hp
            + blood + realm.rng.normal(0, 3))


def might_score(realm, c):
    return c.cr(False) / 40 + c.attr("mar") + c.prowess / 2 + realm.rng.normal(0, 4)


def pool(realm, k):
    f = k.id
    cands = adults_of(realm, f, k.house) if k.house is not None else []
    if getattr(k, "gov", None) in ("republic", "theocracy"):
        from .civics import reform_candidates
        cands = reform_candidates(realm, k) + cands
    heroes = sorted((h for h in realm.heroes() if h.realm == f and h.adult(realm.year)), key=lambda h: -h.renown)[:4]
    nobles = sorted((c for c in adults_of(realm, f) if c.house is not None and c.house != k.house and not c.hero),
                    key=lambda c: -realm.houses[c.house].prestige)[:4]
    seen, out = set(), []
    for c in cands + heroes + nobles:
        if c.id not in seen:
            seen.add(c.id)
            out.append(c)
    return out


def succeed(realm, k, how):
    rng = realm.rng
    f = k.id
    old = realm.chars.get(k.ruler)
    if old is not None:
        end_reign(realm, k, how)
    heir, rival, reason = None, None, ""
    if k.law == "primogeniture" and old is not None:
        heir = primo_heir(realm, old)
        reason = "by right of birth"
    elif k.law == "seniority" and k.house is not None:
        elders = adults_of(realm, f, k.house)
        if elders:
            heir = min(elders, key=lambda c: c.born)
            reason = f"as eldest of {realm.houses[k.house].name}"
    elif k.law == "strongest":
        cands = pool(realm, k)
        if cands:
            scored = sorted(((might_score(realm, c), c) for c in cands), key=lambda t: -t[0])
            heir = scored[0][1]
            reason = "by strength of arm"
            if len(scored) > 1 and scored[1][0] > 0.85 * scored[0][0] and "ambitious" in scored[1][1].personality:
                rival = scored[1][1]
    if heir is None:
        cands = pool(realm, k)
        if cands:
            heir = max(cands, key=lambda c: claim_score(realm, c))
            reason = "chosen by the great of the realm" if k.law == "elective" else "raised up when the line failed"
    if heir is None:
        heir = make_character(realm, f, born=realm.year - ADULT[k.people] - int(rng.integers(0, 20)))
        gain_xp(heir, 800, realm)
        reason = "a nobody raised up out of the ruin of the realm"
    if rival is None:
        others = [c for c in pool(realm, k) if c is not heir and "ambitious" in c.personality
                  and (c.renown >= 300 or c.house == k.house)]
        if others and rng.random() < 0.25:
            rival = max(others, key=lambda c: claim_score(realm, c))
    if rival is not None:
        heir = civil_war(realm, k, heir, rival)
        reason = "after a war for the throne"
    crown(realm, k, heir, reason)
    if old is not None:
        partition(realm, k, old, heir)


PARTITION = [0.35, 0.35, 0.35, 1.0]                                    # orc custom always divides the inheritance


def partition(realm, k, old, heir):
    """Tribal and clan custom divides a dead ruler's lands among their grown children: the heir keeps the seat, and
    each other child takes a duchy as a realm of their own."""
    if getattr(k, "gov", "tribal") not in ("tribal", "clan") or realm.rng.random() > PARTITION[k.people]:
        return
    kids = [realm.chars[i] for i in old.children if i in realm.chars]
    kids = [c for c in kids if c.alive and c is not heir and c.adult(realm.year) and c.realm == k.id and not c.ruled]
    if not kids:
        return
    seat = realm.capital_of(k.id)
    towns = realm.alive_towns(k.id)
    dks = sorted({t.duchy for t in towns if getattr(t, "duchy", None) and (seat is None or t.duchy != seat.duchy)},
                 key=lambda dk: -sum(getattr(t, "duchy", None) == dk for t in towns))
    from .titles import secede
    shares = list(zip(kids, dks))
    if not shares:
        return
    realm.say(f"**The inheritance is divided.** By the custom of {k.name}, the lands of {old.full_name()} are "
              f"shared among their children: {', '.join(c.name for c, _ in shares)} each take a land of their own.")
    for c, dk in shares:
        secede(realm, k, {dk}, c, 0)


def civil_war(realm, k, a, b):
    rng = realm.rng
    yrs = int(rng.integers(1, 7))
    sa = (claim_score(realm, a) + might_score(realm, a)) * rng.uniform(0.6, 1.4)
    sb = (claim_score(realm, b) + might_score(realm, b)) * rng.uniform(0.6, 1.4)
    win, lose = (a, b) if sa >= sb else (b, a)
    for t in realm.alive_towns(k.realm):
        t.pop *= 1 - rng.uniform(0.05, 0.15)
        t.wealth *= 0.8
    k.civil_wars += 1
    fate = "is slain" if rng.random() < 0.6 else "flees into exile"
    realm.say(f"**Succession war.** {realm.who(a)} and {realm.who(b)} both claim {k.name}. "
              f"After {yrs} year{'s' if yrs > 1 else ''} of war {realm.who(win)} holds the throne, and {lose.name} {fate}.")
    if fate == "is slain":
        realm.kill(lose, f"was slain in the war for the throne of {k.name}", by=win)
    elif not lose.hero:
        lose.home = None
        start_hero(realm, lose, "takes up the sword in exile")
    return win


def crown(realm, k, c, reason):
    f = k.id
    new_dyn = c.house is None or c.house != k.house
    if c.house is None:
        new_house(realm, c, c.name)
    old = realm.chars.get(k.ruler)
    if new_dyn and old is not None and old.house is not None and len(k.reigns) > 0:
        from .opinion import remember
        for i in list(realm.living):
            m = realm.chars[i]
            if m.house == old.house and m.id != c.id:
                remember(realm, m, c, "took my family's throne", -50, decay=0.5)
    k.ruler, k.house = c.id, c.house
    n = k.regnal.get(c.name, 0) + 1
    k.regnal[c.name] = n
    c.regnal = roman(n)
    c.ruled = (f, realm.year, None)
    k.reigns.append({"id": c.id, "start": realm.year, "end": None, "how": None})
    realm.houses[c.house].rulers += 1
    k.stats = {"taken": 0, "lost": 0}
    if c.hero:
        c.hero = False
        realm.hero_ids.discard(c.id)
        reason += ", and gives up the adventurer's road for the crown"
    title = RULER_TITLE[c.people][int(c.female)]                     # a ruler is styled as their own people style them
    lead = "**A new dynasty.** " if new_dyn and len(k.reigns) > 1 else ""
    from .governance import on_crown
    on_crown(realm, k, c, reason)
    realm.say(f"{lead}{c.name} {c.regnal} of {realm.houses[c.house].name} is crowned {title} of {k.name}, {reason}"
              f" (legitimacy {c.legitimacy:.0f}).")


def end_reign(realm, k, how):
    c = realm.chars[k.ruler]
    r = k.reigns[-1]
    r["end"], r["how"] = realm.year, how
    c.ruled = (k.id, c.ruled[1], realm.year)
    yrs = realm.year - r["start"]
    if c.epithet is None:
        p = c.personality
        ep = None
        if c.undead:
            ep = "the Undying"
        elif c.tyranny >= 60:
            ep = "the Tyrant"
        elif c.legitimacy is not None and c.legitimacy < 25 and yrs > 3:
            ep = "the Corrupt"
        elif yrs <= 1:
            ep = "the Brief"
        elif k.stats["taken"] >= 5 and k.stats["taken"] > k.stats["lost"]:
            ep = "the Conqueror"
        elif k.stats["lost"] >= 6:
            ep = "the Ruinous"
        elif how == "murdered" and yrs < 6:
            ep = "the Unlucky"
        elif "cruel" in p and yrs > 10:
            ep = "the Cruel"
        elif c.renown >= 400:
            ep = "the Great"
        elif yrs >= LIFESPAN[k.people] * 0.5:
            ep = "the Long-Reigning"
        elif "kind" in p and yrs > 15:
            ep = "the Good"
        elif c.attr("lrn") >= 14:
            ep = "the Wise"
        elif "wrathful" in p:
            ep = "the Wrathful"
        elif "paranoid" in p:
            ep = "the Suspicious"
        elif "greedy" in p:
            ep = "the Grasping"
        elif "generous" in p:
            ep = "the Bountiful"
        c.epithet = ep
    r["epithet"] = c.epithet
    r["legit"], r["tyranny"] = c.legitimacy, c.tyranny
    k.ruler = None


# ---------------------------------------------------------------- the year at court
def dynasty_year(realm):
    rng = realm.rng
    y = realm.year
    for cid in list(realm.living):
        c = realm.chars[cid]
        undying = c.undead or any(it.special == "undying" for it in c.items)
        a = c.age(y)
        if c.latent and c.adult(y) and a < 2 * ADULT[c.people] + 20 and rng.random() < 0.004:
            from .lineage import awaken
            awaken(realm, c, "in the prime of life")
        if undying:
            continue
        if a > c.life * 1.3:
            realm.kill(c, "died of old age")
        elif rng.random() < mortality(realm, c, a):
            frac = a / max(1.0, c.life)
            realm.kill(c, "died in infancy" if a < 1 else "died in childhood" if not c.adult(y) else
                       "died of illness" if frac < 0.75 else "died of old age")
    births(realm)
    for k in realm.kingdoms:
        f = k.id
        if realm.extinct[f]:
            continue
        r = realm.chars.get(k.ruler)
        if r is None or not r.alive:
            succeed(realm, k, r.cause if r is not None else "the throne lay empty")
            r = realm.chars[k.ruler]
        regency(realm, k, r)
        plots(realm, k)
        r = realm.chars[k.ruler]
        if not r.alive:
            succeed(realm, k, r.cause)
        marriages(realm, k)
        council(realm, k)
        princes_errant(realm, k)
        for c in court(realm, k):
            if not c.hero:
                gain_xp(c, 40 if c.age(y) < ADULT[k.people] + 20 else 8, realm)
        realm.houses[k.house].prestige += 1
        realm.houses[k.house].reign_years += 1
    hero_marriages(realm)
    ennoble(realm)
    alliances(realm)


def mortality(realm, c, a):
    """A year's chance of death, calibrated on the medieval elite (see calibration.md), scaled to a lifespan:
    about one in six infants dies in the first year, a quarter to a third of children before fifteen, and adult
    risk doubles roughly every eight years after thirty."""
    h = a * 75 / max(1.0, c.life)                                     # age in human-equivalent years
    if h < 5:                                                         # (the first year is settled at birth)
        q = 0.03
    elif h < 15:
        q = 0.008
    else:
        q = 0.004 + 0.0025 * math.exp(0.085 * (h - 30))
    q *= 1 + max(0.0, 5 - getattr(c, "health", 5)) / 3                # a failing body
    k = realm.kingdoms[c.realm]
    if getattr(k, "physician", False) and c.house is not None and c.house == k.house:
        q *= 0.7
    return q


def births(realm):
    rng = realm.rng
    y = realm.year
    for cid in list(realm.living):
        mum = realm.chars[cid]
        if not mum.female or mum.spouse is None or mum.undead:
            continue
        dad = realm.chars[mum.spouse]
        if not dad.alive:
            continue
        a = mum.age(y)
        if not (ADULT[mum.people] <= a <= FERTILE_END[mum.people]):
            continue
        fec = 1.6 if ("fecund" in mum.genetic or "fecund" in dad.genetic) else 1.0
        if dad.house is None and mum.house is None:          # commoners close to the tale: fewer children followed
            if len(mum.children) >= 5:
                continue
            fec *= 0.85
        older = 0.5 if a > ADULT[mum.people] + (FERTILE_END[mum.people] - ADULT[mum.people]) * 0.7 else 1.0
        if rng.random() < FERTILITY[mum.people] * fec * older * 0.9 ** len(mum.children):
            hs = [(realm.houses[p.house].prestige, p.house) for p in (dad, mum) if p.house is not None]
            house = max(hs)[1] if hs else None
            pid = dad.realm if (house is not None and dad.house == house) else mum.realm
            kid = make_character(realm, pid, born=y, father=dad, mother=mum, house=house)
            people = pid
            from .bending import born_bender
            born_bender(realm, kid)
            from .heart import on_birth as heart_birth
            heart_birth(realm, kid)
            for par in (dad, mum):
                if getattr(par, "tracked", False):
                    from .events import journal
                    journal(realm, par, f"A child is born: {kid.name}.", "lives")
            if not mum.undead and rng.random() < 0.012:              # about one birth in eighty killed the mother
                realm.kill(mum, "died in childbirth")
            if rng.random() < 0.16:                                   # and about one in six died within the year
                realm.kill(kid, "died in infancy")
            from .legends import on_birth
            on_birth(realm, kid)
            r = realm.chars.get(realm.kingdoms[people].ruler)
            if r is not None and (dad.id == r.id or mum.id == r.id):
                rare = [t for t in kid.genetic if t in ("dragonblood", "starblood", "genius", "spellborn")]
                if rare:
                    realm.say(f"A child, {kid.name}, is born to {realm.who(r)}; the midwives speak of "
                              f"{', '.join(rare)}.")


def find_match(realm, c, pool_):
    y = realm.year
    best = None
    for o in pool_:
        if (o.alive and o.spouse is None and o.female != c.female and o.adult(y) and not o.undead
                and o.id != c.id and o.age(y) <= FERTILE_END[o.people] and o.house != c.house):
            if best is None or o.renown + rng_tiebreak(realm) > best.renown:
                best = o
    return best


def rng_tiebreak(realm):
    return float(realm.rng.random())


def marriages(realm, k):
    rng = realm.rng
    y = realm.year
    f = k.id
    for c in court(realm, k):
        if c.spouse is not None or not c.adult(y) or c.undead or c.age(y) > FERTILE_END[c.people]:
            continue
        if rng.random() > 0.45:
            continue
        r = rng.random()
        mate = None
        if r < 0.15:
            others = [o for o in realm.kingdoms if o is not k and not realm.extinct[o.realm]
                      and not realm.war[f, o.realm] and realm.relation[f, o.id] > -30]
            if others:
                o = others[int(rng.integers(len(others)))]
                mate = find_match(realm, c, court(realm, o))
                if mate is not None:
                    marry(c, mate)
                    realm.say(f"**Alliance.** {realm.who(c)} of {k.name} weds {realm.who(mate)} of {o.name}.")
                    realm.relation[f, o.id] += 25
                    realm.relation[o.id, f] += 25
                    continue
        if r < 0.40:
            heroes = [h for h in realm.heroes() if h.realm == f and h.renown >= 100]
            mate = find_match(realm, c, heroes)
            if mate is not None:
                marry(c, mate)
                from .opinion import remember
                remember(realm, mate, realm.chars.get(k.ruler), "married into the royal family", 20, decay=0.5)
                realm.say(f"The crown rewards {realm.who(mate)} with the hand of {realm.who(c)}.")
                continue
        nobles = [o for o in adults_of(realm, f) if o.house is not None and o.house != k.house and not o.hero]
        mate = find_match(realm, c, nobles)
        if mate is None:
            mate = make_character(realm, f, born=c.born + int(rng.integers(-5, 6)), female=not c.female)
            new_house(realm, mate)
            gain_xp(mate, 500, realm)
        marry(c, mate)


def hero_marriages(realm):
    rng = realm.rng
    y = realm.year
    for h in list(realm.heroes()):
        if h.spouse is None and h.adult(y) and not h.undead and h.age(y) <= FERTILE_END[h.people] \
                and rng.random() < 0.08:
            mate = make_character(realm, h.realm, born=h.born + int(rng.integers(-5, 6)), female=not h.female)
            marry(h, mate)


def regency(realm, k, r):
    rng = realm.rng
    y = realm.year
    if r.adult(y):
        if k.regent is not None:
            realm.say(f"{realm.who(r)} comes of age and takes the rule of {k.name} into their own hands.")
            k.regent = None
        return
    reg = realm.chars.get(k.regent) if k.regent is not None else None
    if reg is None or not reg.alive:
        cands = [c for c in adults_of(realm, k.id) if c.id != r.id]
        if not cands:
            return
        reg = max(cands, key=lambda c: c.attr("ste") + c.attr("dip") + (5 if c.house == k.house else 0))
        k.regent = reg.id
        realm.say(f"{realm.who(r)} is a child; {realm.who(reg)} rules {k.name} as regent.")
    if "ambitious" in reg.personality and reg.personality & {"cruel", "deceitful"} and rng.random() < 0.04:
        realm.say(f"The child-ruler {realm.who(r)} dies in the night. No one asks the regent, {realm.who(reg)}, "
                  f"any questions.")
        realm.kill(r, "died in mysterious circumstances", by=reg)
        end_reign(realm, k, "died in mysterious circumstances")
        k.regent = None
        crown(realm, k, reg, "seizing the throne from the cradle")


def plots(realm, k):
    rng = realm.rng
    r = realm.chars[k.ruler]
    spy = realm.chars.get(k.council["spymaster"])
    guard = (spy.attr("intr") if spy is not None else 0) + (3 if "paranoid" in r.personality else 0) \
        + (5 if getattr(k, "bodyguard", False) else 0)
    if r.undead:
        guard += 10
    from .opinion import opinion as _op
    plotters = [c for c in adults_of(realm, k.id)
                if c.id != r.id and (("ambitious" in c.personality and c.personality & {"deceitful", "cruel"}
                                      and _op(realm, c, r) < -20) or r.id in getattr(c, "vendettas", {}))
                and (c.house == k.house or c.renown > 150 or c.id == k.regent)]
    for p in plotters:
        chance = min(0.01, 0.001 * max(0.0, p.attr("intr") - 0.6 * guard - 4))
        if rng.random() < chance:
            realm.say(f"{realm.who(r)} is murdered. The court whispers the name of {realm.who(p)}.")
            from .opinion import remember_kin
            remember_kin(realm, r, p, "murdered my kin", -80)
            realm.kill(r, "was murdered", quiet=True, by=p)
            return
        if spy is not None and rng.random() < 0.002 * spy.attr("intr"):
            realm.say(f"{realm.who(spy)}, spymaster of {k.name}, uncovers a plot; {realm.who(p)} is put to death.")
            from .opinion import gain_hook, remember_kin
            from .heart import add_stress, reluctance
            spare = 0.1 if "cruel" in r.personality else 0.85 if r.personality & {"compassionate", "forgiving"} else 0.45
            if realm.rng.random() < spare:
                gain_hook(realm, r, p, "strong", ", sparing a plotter's life in exchange for their loyalty")
                continue
            remember_kin(realm, p, r, "executed my kin", -60)
            add_stress(realm, r, reluctance(r, "execute"), "an execution")
            realm.kill(p, f"was executed for plotting against {r.name} {r.regnal}", quiet=True, by=r)
            from .governance import add_tyranny
            add_tyranny(realm, r, 5, "the executions")
    for o in realm.kingdoms:
        if o is k or not realm.war[k.realm, o.id] or realm.extinct[o.realm]:
            continue
        ospy = realm.chars.get(o.council["spymaster"])
        shield = 0.8 * guard + getattr(k, "security", 0.0) / 4             # guards and tasters the court has learned to keep
        if ospy is not None and rng.random() < min(0.01, 0.0008 * max(0.0, ospy.attr("intr") - shield)):
            k.security = getattr(k, "security", 0.0) + 40
            realm.say(f"{realm.who(r)} is assassinated by agents of {o.name}, at the word of {realm.who(ospy)}.")
            realm.kill(r, f"was assassinated by agents of {o.name}", quiet=True, by=realm.chars.get(o.ruler))
            return


SEATS = {"marshal": ("Marshal", lambda c: c.attr("mar")),
         "steward": ("Steward", lambda c: c.attr("ste") + c.skills.get("Mercantile", 0) / 25),
         "spymaster": ("Spymaster", lambda c: c.attr("intr")), "mage": ("Court Mage", lambda c: c.tier()),
         "loremaster": ("Loremaster", lambda c: c.attr("lore")),
         "admiral": ("Lord Admiral", lambda c: c.skills.get("Seamanship", 0) / 5)}


def council(realm, k):
    r = realm.chars[k.ruler]
    cands = [c for c in adults_of(realm, k.id) if c.id != r.id and c.disgraced_until <= realm.year]
    taken = set()
    prev_holder = dict(k.council)
    powerful = getattr(k, "powerful", set())
    # a ruler weighs skill against politics: powerful vassals expect a seat, and a diplomat seats them
    appease = 4 + r.attr("dip") / 2
    # the council is weighed afresh every five years, on a new ruler, or when a seat falls empty
    fresh = (realm.year + k.id) % 5 == 0 or getattr(k, "council_for", None) != r.id
    k.council_for = r.id
    for seat, (title, score) in SEATS.items():
        cur = realm.chars.get(k.council.get(seat))
        if not fresh and cur is not None and cur.alive and cur.realm == k.id and cur.id != r.id \
                and cur.id not in taken:
            taken.add(cur.id)
            continue
        pick = None
        ap = appease
        if seat == "steward":
            wise = "diligent" in r.personality or "just" in r.personality or r.attr("ste") >= 10
            ap *= 1 - min(1.0, getattr(k, "econ_distress", 0.0)) * (1.0 if wise else 0.3)
        for c in sorted(cands, key=lambda c: score(c) + (ap if c.id in powerful else 0), reverse=True):
            if c.id not in taken:
                pick = c
                break
        if pick is None or (seat == "mage" and pick.tier() < 0):
            k.council[seat] = None
            continue
        taken.add(pick.id)
        if k.council[seat] != pick.id:
            k.council[seat] = pick.id
            pick.titles.append(f"{title} of {k.name} from year {realm.year}")
            from .opinion import remember
            remember(realm, pick, r, "appointed me to the council", 15, decay=0.5)
            old = realm.chars.get(prev_holder.get(seat)) if prev_holder.get(seat) is not None else None
            if old is not None and old.alive:
                remember(realm, old, r, "removed me from the council", -20, decay=1.0)
    y = realm.year
    m = realm.chars.get(k.council["marshal"])
    s = realm.chars.get(k.council["steward"])
    mg = realm.chars.get(k.council["mage"])
    k.battle_mult = (1 + ((m.attr("mar") if m else 0) + r.attr("mar") * 0.5) / 60) * k.mod("martial", y)
    k.growth_mult = 1 + ((s.attr("ste") if s else 0) + r.attr("ste") * 0.5) / 80
    k.realm_tier = max(0, mg.tier()) if mg else 0
    p = r.personality
    k.aggr = max(0.2, 1 + 0.5 * ("ambitious" in p) + 0.5 * ("wrathful" in p) + 0.3 * ("cruel" in p)
                 - 0.4 * ("content" in p) - 0.3 * ("calm" in p) - 0.3 * ("kind" in p))


def princes_errant(realm, k):
    rng = realm.rng
    y = realm.year
    f = k.id
    r = realm.chars[k.ruler]
    heir_id = min(r.children, key=lambda i: realm.chars[i].born) if r.children else None
    n = sum(1 for h in realm.heroes() if h.realm == f)
    for c in court(realm, k):
        if (c.id in (r.id, heir_id) or c.hero or not c.adult(y) or c.age(y) > ADULT[k.people] + 25
                or not (c.personality & {"brave", "ambitious"})):
            continue
        if n < HERO_CAP + 2 and rng.random() < 0.03:
            c.home = None
            start_hero(realm, c, f"leaves the court of {k.name} to seek glory")
            n += 1


def ennoble(realm):
    for h in list(realm.heroes()):
        if h.house is None and h.renown >= 300:
            house = new_house(realm, h, h.name)
            house.prestige = h.renown * 0.5
            from .data import LEGACY
            rk = realm.chars.get(realm.kingdoms[h.realm].ruler)
            from .opinion import remember
            remember(realm, h, rk, "raised me to the nobility", 30, decay=0.5)
            realm.say(f"{realm.who(h)} is raised to the nobility and founds {house.name}, a house of "
                      f"{LEGACY[house.track][0]}.")
            home = realm.by_id.get(h.home)
            if home is not None and home.alive and not home.capital and home.lord is None:
                from .governance import grant_lordship
                grant_lordship(realm, h, home)


def alliances(realm):
    for k in realm.kingdoms:
        k.allies = set()
    by_house = {k.house: k for k in realm.kingdoms if not realm.extinct[k.realm]}
    for i in realm.living:
        c = realm.chars[i]
        if c.house in by_house and c.spouse is not None:
            sp = realm.chars[c.spouse]
            if sp.alive and sp.house in by_house and sp.house != c.house:
                a, b = by_house[c.house], by_house[sp.house]
                a.allies.add(b.id)
                b.allies.add(a.id)
