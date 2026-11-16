"""Governance: legitimacy (low legitimacy is corruption), tyranny, lordships, and what passes at death.

Everyone who governs (a ruler, a regent, a councillor, a lord) has a legitimacy from 0 to 100. Below 30 they
are corrupt: they skim, the realm's wealth bleeds and its growth slows, and a corrupt ruler invites revolt.
Rulers also carry tyranny: executions, purges, usurpation, sacked towns and dark rites raise it. Tyranny buys
fear (fewer plots) at the price of legitimacy and peasant risings, and a tyrant with no legitimacy left is
overthrown."""
from .data import ADULT, LEGACY, PEOPLES, RARITY
from .lineage import legacy_level, pass_legacy_on_death

CROWN_BASE = {"by right of birth": 70, "as eldest": 68, "chosen by the great of the realm": 62,
              "by strength of arm": 55, "the first of the line": 75, "raised up when the line failed": 45,
              "after a war for the throne": 40, "seizing the throne from the cradle": 12,
              "a nobody raised up": 30, "overthrowing": 35, "leading the war of independence": 52,
              "restored when the dark was cast down": 70}


def clamp(v, lo=0.0, hi=100.0):
    return max(lo, min(hi, v))


def is_corrupt(c):
    return c.legitimacy is not None and c.legitimacy < 30


# ---------------------------------------------------------------- on taking power
def on_crown(realm, k, c, reason):
    base = 50
    for key, v in CROWN_BASE.items():
        if reason.startswith(key) or key in reason:
            base = v
            break
    base += legacy_level(realm, c, "ruler") * 5 + min(15, c.renown / 60)
    if c.house is not None:
        base += realm.houses[c.house].prestige / 200
    c.legitimacy = clamp(base)
    if "seizing" in reason:
        c.tyranny = clamp(c.tyranny + 30)


def add_tyranny(realm, c, amount, why=None):
    if c is None:
        return
    before = c.tyranny
    c.tyranny = clamp(c.tyranny + amount)
    if before < 60 <= c.tyranny:
        realm.say(f"{realm.who(c)} is called a tyrant now{f', after {why}' if why else ''}. Men lower their eyes "
                  f"when the crown rides by.")


def official_target(realm, c, skill, ruler):
    p = c.personality
    t = 45 + skill * 1.2 + 15 * ("honest" in p) - 15 * ("greedy" in p) - 10 * ("deceitful" in p) \
        + 5 * ("diligent" in p) - 5 * ("lazy" in p)
    if ruler is not None and ruler.legitimacy is not None:
        t += (ruler.legitimacy - 50) / 5 - ruler.tyranny / 10
    return t


# ---------------------------------------------------------------- the year
def govern_year(realm):
    rng = realm.rng
    y = realm.year
    for k in realm.kingdoms:
        f = k.id
        if realm.extinct[f] or k.ruler is None:
            continue
        r = realm.chars[k.ruler]
        if r.legitimacy is None:
            r.legitimacy = 50.0
        # the ruler
        p = r.personality
        reign = y - (r.ruled[1] if r.ruled else y)
        target = (50 + (r.attr("dip") + r.attr("ste") - 10) * 1.5 + legacy_level(realm, r, "ruler") * 5
                  + min(15, r.renown / 80) + min(15, reign / 5) - r.tyranny / 2
                  + max(-20, min(15, 2 * (k.stats["taken"] - 2 * k.stats["lost"])))
                  + 5 * ("honest" in p) - 5 * ("deceitful" in p) - 5 * ("greedy" in p))
        if c_house := (realm.houses[r.house] if r.house is not None else None):
            if c_house.track == "ruler":
                target += 3 * legacy_level(realm, r, "ruler")
        if getattr(realm, "religions", None):
            from .society import ruler_faith_legitimacy
            target += ruler_faith_legitimacy(realm, k, r)
        r.legitimacy = clamp(r.legitimacy + 0.1 * (target - r.legitimacy))
        r.tyranny = clamp(r.tyranny * 0.985 + 0.5 * ("cruel" in p) + 0.3 * ("paranoid" in p)
                          + 0.2 * ("wrathful" in p) - 0.4 * ("kind" in p) - 0.2 * ("calm" in p))
        if not r.undead:
            from .lineage import gain_legacy
            gain_legacy(realm, r, "ruler", 2 + r.legitimacy / 25)
        # the council: low legitimacy is corruption
        corrupt_seats = []
        for seat, cid in k.council.items():
            c = realm.chars.get(cid)
            if c is None or not c.alive:
                continue
            skill = {"marshal": c.attr("mar"), "steward": c.attr("ste"), "spymaster": c.attr("intr"),
                     "mage": max(0, c.tier()) * 2, "loremaster": c.attr("lore"),
                     "admiral": c.skills.get("Seamanship", 0) / 6}[seat]
            if c.legitimacy is None:
                c.legitimacy = clamp(official_target(realm, c, skill, r) + rng.normal(0, 8))
            was = is_corrupt(c)
            c.legitimacy = clamp(c.legitimacy + 0.1 * (official_target(realm, c, skill, r) - c.legitimacy)
                                 + rng.normal(0, 2))
            if is_corrupt(c):
                corrupt_seats.append(seat)
                if not was:
                    realm.say(f"{realm.who(c)}, {seat} of {k.name}, has grown corrupt; coin goes missing from the "
                              f"treasury.", minor=True)
                if r.legitimacy >= 60 and ("honest" in p or "diligent" in p) and rng.random() < 0.3:
                    c.disgraced_until = y + 25
                    k.council[seat] = None
                    realm.say(f"{realm.who(r)} strips {c.name} of the office of {seat} for corruption.")
        k.corrupt_seats = corrupt_seats
        corruption = max(0.0, (50 - r.legitimacy) / 50) + 0.1 * len(corrupt_seats)
        k.corruption = min(1.0, corruption)
        k.growth_mult *= 1 - 0.15 * k.corruption
        if "marshal" in corrupt_seats:
            k.battle_mult *= 0.9
        if "mage" in corrupt_seats:
            k.realm_tier = max(0, k.realm_tier - 1)
        for t in realm.alive_towns(f):
            t.wealth *= 1 - 0.03 * k.corruption
        # tyranny: purges, risings, and the fall of tyrants
        if "cruel" in p and "paranoid" in p and rng.random() < 0.04:
            purge(realm, k, r)
        if rng.random() < 0.004 * r.tyranny / 10:
            rising(realm, k, r)
        if r.legitimacy < 25 and rng.random() < 0.03 * (30 - r.legitimacy) / 30:
            overthrow(realm, k, r)
    lords_year(realm)


def purge(realm, k, r):
    from .dynasty import adults_of
    rng = realm.rng
    victims = [c for c in adults_of(realm, k.id) if c.id != r.id and c.house is not None and not c.undead
               and c.house != r.house]
    if not victims:
        return
    v = victims[int(rng.integers(len(victims)))]
    realm.say(f"{realm.who(r)} purges the court: {realm.who(v)} is dragged out and put to death.")
    from .opinion import remember_kin
    remember_kin(realm, v, r, "purged my kin", -60)
    from .heart import add_stress, reluctance
    add_stress(realm, r, reluctance(r, "execute"), "the purges")
    realm.kill(v, f"was put to death in the purges of {r.name} {r.regnal}", quiet=True, by=r)
    add_tyranny(realm, r, 8, "the purges")
    k.fear_until = realm.year + 10


def rising(realm, k, r):
    rng = realm.rng
    towns = realm.alive_towns(k.realm)
    if not towns:
        return
    t = towns[int(rng.integers(len(towns)))]
    t.pop *= 0.8
    t.walls = max(0, t.walls - 1)
    t.wealth *= 0.6
    r.legitimacy = clamp(r.legitimacy - 4)
    realm.say(f"The common folk of {t.name} rise against {realm.who(r)}; the rising is put down in blood.",
              minor=r.tyranny < 50)
    add_tyranny(realm, r, 3, "the risings")


def overthrow(realm, k, r):
    from .dynasty import civil_war, crown, end_reign, pool, claim_score
    cands = [c for c in pool(realm, k) if c.id != r.id]
    if not cands:
        return
    rival = max(cands, key=lambda c: claim_score(realm, c) + (c.legitimacy or 50) / 10)
    why = "tyranny" if r.tyranny >= 50 else "corruption"
    realm.say(f"**The realm rises.** {k.name} will no longer suffer the {why} of {realm.who(r)}; the great "
              f"lords raise the banner of {realm.who(rival)}.")
    win = civil_war(realm, k, r, rival)
    if win is rival:
        if r.alive:
            realm.kill(r, f"was overthrown for {why}", quiet=True)
        end_reign(realm, k, f"overthrown for {why}")
        crown(realm, k, rival, f"overthrowing a ruler hated for {why}")
    else:
        add_tyranny(realm, r, 10, "crushing the rising")
        r.legitimacy = clamp(r.legitimacy + 10)


# ---------------------------------------------------------------- lordships
def lord_title(c, s):
    return f"{'Lady' if c.female else 'Lord'} of {s.name}"


def grant_lordship(realm, c, s, why=""):
    if s.lord is not None:
        return
    s.lord = c.id
    c.lordships.append(s.id)
    c.titles.append(f"{lord_title(c, s)} from year {realm.year}")
    if c.legitimacy is None:
        c.legitimacy = clamp(55 + 10 * ("honest" in c.personality) - 10 * ("greedy" in c.personality))
    if why:
        realm.say(f"{realm.who(c)} is made {lord_title(c, s)}{why}.", minor=True)


def lose_lordship(realm, s):
    if s.lord is None:
        return
    c = realm.chars.get(s.lord)
    if c is not None and s.id in c.lordships:
        c.lordships.remove(s.id)
    s.lord = None


def lords_year(realm):
    rng = realm.rng
    y = realm.year
    for s in realm.alive_towns():
        c = realm.chars.get(s.lord) if s.lord is not None else None
        if c is not None and (not c.alive or c.realm != s.faction):
            lose_lordship(realm, s)
            c = None
        if c is None:
            s.lord_mod = 1.0
            continue
        r = realm.chars.get(realm.kingdoms[s.faction].ruler)
        target = official_target(realm, c, c.attr("ste"), r)
        c.legitimacy = clamp((c.legitimacy if c.legitimacy is not None else target)
                             + 0.1 * (target - (c.legitimacy or target)) + rng.normal(0, 2))
        if is_corrupt(c):
            s.wealth *= 0.95
            s.lord_mod = 0.9
        else:
            s.lord_mod = 1 + (c.attr("ste") - 5) / 60
    # the crown grants vacant lordships to its councillors and to renowned heroes
    for k in realm.kingdoms:
        if realm.extinct[k.realm] or k.ruler is None:
            continue
        vacant = [s for s in realm.alive_towns(k.realm) if s.lord is None and not s.capital]
        if not vacant or rng.random() > 0.3:
            continue
        deserving = [realm.chars[i] for i in k.council.values() if i is not None and realm.chars[i].alive]
        deserving += [h for h in realm.heroes() if h.realm == k.id and h.renown >= 150]
        deserving = [d for d in deserving if len(d.lordships) < 3 and d.disgraced_until <= y]
        if deserving:
            c = max(deserving, key=lambda d: d.renown + 20 * len(d.titles) * 0 + rng.random() * 50)
            s = max(vacant, key=lambda t: t.pop)
            grant_lordship(realm, c, s, f" by grant of the crown")


# ---------------------------------------------------------------- what passes at death
def heir_of(realm, c):
    kids = sorted((realm.chars[i] for i in c.children if realm.chars[i].alive and realm.chars[i].people == c.people),
                  key=lambda x: x.born)
    if kids:
        return kids[0]
    if c.spouse is not None and realm.chars[c.spouse].alive:
        return realm.chars[c.spouse]
    if c.house is not None:
        kin = [realm.chars[i] for i in realm.living if realm.chars[i].house == c.house]
        if kin:
            return min(kin, key=lambda x: x.born)
    return None


def inherit(realm, c, killer=None):
    """Items go to the heir (or into the hoard of the monster that killed them); lordships to the heir;
    the deeds they did after their children were born still pass to the children as legacy."""
    from .heroes import give_item
    pass_legacy_on_death(realm, c)
    heir = heir_of(realm, c)
    from .beasts import pass_beast
    pass_beast(realm, c, heir)
    k = realm.kingdoms[c.realm] if realm.kingdoms else None
    items, c.items = c.items, []
    c.invalidate()
    if items:
        if killer is not None and getattr(killer, "alive", False):
            killer.items.extend(items)
            best = max(items, key=lambda i: i.rarity)
            if best.rarity >= 4:
                realm.say(f"{best.label()} is dragged into the hoard of {killer.name}.", minor=True)
        elif heir is not None:
            best = max(items, key=lambda i: i.rarity)
            for it in items:
                give_item(realm, heir, it, quiet=True)
            if best.rarity >= 4:
                rel = "child" if heir.id in c.children else "spouse" if heir.id == c.spouse else "kin"
                realm.say(f"{best.label()} passes from {c.full_name()} to their {rel}, {realm.who(heir)}.")
        elif k is not None and not realm.extinct[c.realm]:
            k.treasury.extend(items)
        elif realm.ruins:
            realm.ruins[int(realm.rng.integers(len(realm.ruins)))].loot.extend(items)
    for sid in list(c.lordships):
        s = realm.by_id.get(sid)
        c.lordships.remove(sid)
        if s is None or not s.alive:
            continue
        s.lord = None
        if heir is not None and heir.realm == s.faction:
            grant_lordship(realm, heir, s)
            if heir.legitimacy is not None:
                heir.legitimacy = clamp(heir.legitimacy + 5)
