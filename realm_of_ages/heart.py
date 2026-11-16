"""The heart: love, grief, vengeance, and the three meters every person carries.

Love. Parents love their children (unless they are callous), children their parents, spouses grow to love each
other if they get on, and people fall in love, whether or not they are free to: a married person in love with
someone else has a lover and a secret. Pets and tamed beasts are loved too. Each bond has a strength, adds to
happiness while it lasts, and gives the loved one a hold over the lover (a hook of the heart).

Grief and vengeance. When someone dies, everyone who loved them grieves (stress, lost happiness). If someone
is to blame (an executioner, a murderer, a plotter, the ruler whose war killed them, the client of an
assassin), the bereaved remember it for life, and the vengeful never forgive. A ruler with an unavenged
grievance has a casus belli of vengeance against the one who did it, and that is how many wars begin.

Stress, happiness and health. Stress (0 to 400) rises when people act against their nature (the compassionate
who execute, the craven who go to war, the shy who hold court, the lazy who rule too much), and with grief; at
100, 200 and 300 they break, and take up a coping habit, or at the last level go mad. Happiness (-100 to 100)
comes from love, status, a lover, a pet, and the trait one was born with; it lowers stress, and its lack
raises it. Health (0 to 10) falls with age, stress and wounds and rises with happiness and a court physician,
and a body below 3 is close to death."""
import math

COPING = {  # coping habit: the natures that fall into it
    "drunkard": ("gregarious", "impatient", "arbitrary"), "reclusive": ("shy", "patient"),
    "flagellant": ("zealous", "just", "humble"), "irritable": ("wrathful", "impatient", "arrogant"),
    "comfort eater": ("gluttonous", "content"), "profligate": ("greedy", "lustful"),
    "journaller": ("diligent", "patient"), "confider": ("trusting", "gregarious"),
    "hunter": ("brave", "callous"), "inappetetic": ("temperate", "chaste"),
}
PETS = [("dog", 14), ("cat", 16), ("hawk", 12), ("horse", 25), ("wolfhound", 12), ("raven", 18)]
PET_NAMES = ["Ash", "Biscuit", "Thorn", "Mira", "Rook", "Pip", "Bram", "Sable", "Tansy", "Brindle", "Moth",
             "Cinder", "Wren", "Bracken", "Juniper", "Hob"]


# ---------------------------------------------------------------- meters
def init(c, rng):
    c.stress = 0.0
    c.happy = 0.0
    c.health = 5.0 + 1.0 * ("hale" in c.genetic) - 2.0 * ("sickly" in c.genetic) + rng.normal(0, 0.5)
    c.loves = {}            # id -> [kind, strength]
    c.vendettas = {}        # id -> (year, whom they lost)
    c.grief = 0.0
    c.coping = []
    c.pet = None
    c.orientation = rng.choice(["straight", "gay", "bi"], p=[0.86, 0.05, 0.09])


def ensure(c, rng):
    if not hasattr(c, "stress"):
        init(c, rng)


def stress_level(c):
    return int(min(3, getattr(c, "stress", 0) // 100))


def add_stress(realm, c, amount, why=""):
    if c is None or not c.alive:
        return
    ensure(c, realm.rng)
    before = stress_level(c)
    c.stress = max(0.0, min(400.0, c.stress + amount))
    after = stress_level(c)
    if after > before:
        mental_break(realm, c, after, why)


def mental_break(realm, c, level, why):
    rng = realm.rng
    if level >= 3 and rng.random() < 0.4 and "mad" not in c.coping:
        c.coping.append("mad")
        c.stress -= 80
        realm.say(f"**{realm.who(c)} breaks under it all and goes mad**{', after ' + why if why else ''}.",
                  minor=c.ruled is None)
        return
    fits = [h for h, natures in COPING.items() if set(natures) & c.personality and h not in c.coping] or \
        [h for h in COPING if h not in c.coping]
    if not fits:
        return
    habit = fits[int(rng.integers(len(fits)))]
    c.coping.append(habit)
    c.stress -= 50
    realm.say(f"{realm.who(c)} cracks under the strain{', after ' + why if why else ''}, and becomes a {habit}.",
              minor=True)


def meters_year(realm):
    rng = realm.rng
    y = realm.year
    for i in list(realm.living):
        c = realm.chars[i]
        ensure(c, rng)
        if c.age(y) < 6:
            continue
        p = c.personality
        # happiness: love, status, comfort, grief and the nature one was born with
        loved = sum(min(20.0, s / 5) for (kind, s) in c.loves.values())
        lover = 15 if any(k == "lover" for k, _ in c.loves.values()) else 0
        status = 15 if c.ruled is not None and c.ruled[2] is None else 8 if getattr(c, "duchies", None) else \
            5 if c.hero or c.lordships else 0
        natured = 10 * ("content" in p) - 10 * ("ambitious" in p and status < 10) + 5 * ("gregarious" in p) \
            - 5 * ("arrogant" in p and status < 10)
        target = min(45.0, loved) + lover + status + natured + (10 if c.pet else 0) - c.grief - 15 * stress_level(c) \
            - 20 * ("mad" in c.coping)
        c.happy += 0.3 * (max(-100, min(100, target)) - c.happy)
        c.grief *= 0.85
        # stress: eases with happiness and coping, grows with unhappiness and with a life against one's nature
        ease = 15 + (10 if c.happy > 20 else 0) + 6 * len(c.coping)
        c.stress = max(0.0, c.stress - ease)
        if c.happy < -40:
            add_stress(realm, c, 5, "a long unhappiness")
        if c.ruled is not None and c.ruled[2] is None:
            k = realm.kingdoms[c.realm]
            if "lazy" in p:
                add_stress(realm, c, 3 * getattr(k, "over_vassals", 0) + 2, "the weight of a realm")
            if "ambitious" in p and k.rank in ("tribe", "chiefdom"):
                add_stress(realm, c, 4, "a small throne")
        if "ambitious" in p and status == 0 and c.age(y) > 40:
            add_stress(realm, c, 4, "a life of no account")
        if "vengeful" in p and c.vendettas:
            add_stress(realm, c, 8, "an unavenged wrong")
        if "lustful" in p and not lover and c.spouse is None:
            add_stress(realm, c, 3, "an empty bed")
        # health: age, stress, happiness, a physician, and the habits one copes with
        frac = c.age(y) / max(1.0, c.life)
        target_h = 5.5 + 1.0 * ("hale" in c.genetic) - 2.0 * ("sickly" in c.genetic) - max(0.0, (frac - 0.6) * 12) \
            - 0.8 * stress_level(c) + c.happy / 60 - 0.8 * ("drunkard" in c.coping) - 0.5 * ("comfort eater" in c.coping)
        if c.ruled is not None and getattr(realm.kingdoms[c.realm], "physician", False):
            target_h += 0.6
        if c.undead:
            target_h = 8.0
        c.health += 0.2 * (target_h - c.health)
        if c.hero and c.hp < 30:
            c.health -= 0.3                                            # old wounds
        if not c.undead and c.health < 2 and rng.random() < (2 - c.health) * 0.03:
            realm.kill(c, "died of failing health" + (", worn down by grief" if c.grief > 20 else ""))
            continue
        if c.pet is not None and y - c.pet["since"] > c.pet["life"]:
            grieve(realm, c, None, 15, f"their {c.pet['kind']} {c.pet['name']}")
            c.pet = None


# ---------------------------------------------------------------- love
def love(c, other, kind, strength):
    c.loves = getattr(c, "loves", {})
    if getattr(c, "tracked", False) and other.id not in c.loves and kind in ("lover", "soulmate", "spouse"):
        c.journal = getattr(c, "journal", [])
        from .characters import CTX
        c.journal.append((CTX["realm"].year if "realm" in CTX else 0, "heart",
                          f"Comes to love {other.full_name()} ({kind})."))
    other.loved_by = getattr(other, "loved_by", set())
    other.loved_by.add(c.id)
    cur = c.loves.get(other.id)
    if cur is None or cur[1] < strength or kind in ("lover", "soulmate", "spouse"):
        c.loves[other.id] = [kind, float(strength)]


def attracted(a, b):
    ao, bo = getattr(a, "orientation", "straight"), getattr(b, "orientation", "straight")

    def ok(x, xo, y):
        return xo == "bi" or (xo == "straight" and x.female != y.female) or (xo == "gay" and x.female == y.female)
    return ok(a, ao, b) and ok(b, bo, a)


def on_birth(realm, kid):
    rng = realm.rng
    init(kid, rng)
    for pid in (kid.father, kid.mother):
        p = realm.chars.get(pid)
        if p is None or not p.alive:
            continue
        ensure(p, rng)
        if "callous" not in p.personality:
            love(p, kid, "child", rng.uniform(55, 95))
        love(kid, p, "parent", rng.uniform(50, 90))


def on_marriage(realm, a, b):
    from .opinion import opinion
    rng = realm.rng
    ensure(a, rng)
    ensure(b, rng)
    if opinion(realm, a, b) > 20 or a.loves.get(b.id, ["", 0])[0] == "lover":
        love(a, b, "spouse", rng.uniform(40, 80))
        love(b, a, "spouse", rng.uniform(40, 80))


def hearts_year(realm):
    """Who falls in love this year, who takes a lover, whose marriage warms or cools, who adopts a dog."""
    from .opinion import gain_hook, opinion, remember
    rng = realm.rng
    y = realm.year
    for k in realm.kingdoms:
        if realm.extinct[k.id] or k.ruler is None:
            continue
        circle = [realm.chars[i] for i in realm.member_ids(k.id) if True and realm.chars[i].adult(y)
                  and (realm.chars[i].house is not None or realm.chars[i].hero or getattr(realm.chars[i], "duchies", None))]
        if len(circle) < 2:
            continue
        for _ in range(max(1, len(circle) // 12)):
            if rng.random() > 0.35:
                continue
            a, b = rng.choice(len(circle), 2, replace=False)
            a, b = circle[int(a)], circle[int(b)]
            ensure(a, rng)
            ensure(b, rng)
            if a.undead or b.undead or not attracted(a, b) or b.id in a.loves:
                continue
            chem = opinion(realm, a, b) + opinion(realm, b, a) + 20 * ("lustful" in a.personality) \
                + 20 * ("comely" in b.genetic) - 30 * ("chaste" in a.personality) + rng.normal(0, 25)
            if chem < 95:
                continue
            if a.spouse == b.id:                                       # falling for one's own spouse is no affair
                love(a, b, "spouse", rng.uniform(60, 100))
                love(b, a, "spouse", rng.uniform(60, 100))
                continue
            soul = chem > 140
            kind = "soulmate" if soul else "lover"
            love(a, b, kind, rng.uniform(60, 100))
            love(b, a, kind, rng.uniform(60, 100))
            realm.love_log.append((y, a.id, b.id, "fell in love"))
            if a.spouse is None and b.spouse is None:
                from .dynasty import marry
                marry(a, b)
                on_marriage(realm, a, b)
                realm.say(f"{realm.who(a)} and {realm.who(b)} marry for love.", minor=not (a.ruled or b.ruled))
                continue
            # an affair: a secret, and leverage for whoever learns it
            a.affair = b.id
            b.affair = a.id
            realm.say(f"{realm.who(a)} and {realm.who(b)} become lovers, in secret.", minor=True)
            for x, other in ((a, b), (b, a)):
                gain_hook(realm, other, x, "weak", ", the hold of a lover") if x.ruled is not None else None
                if "honest" in x.personality or "chaste" in x.personality:
                    add_stress(realm, x, 15, "a secret love")
            spy = realm.chars.get(k.council.get("spymaster"))
            if spy is not None and rng.random() < spy.attr("intr") / 40:
                gain_hook(realm, spy, a, "strong", ", knowing of an affair")
                if rng.random() < 0.3 and a.spouse is not None:
                    sp = realm.chars[a.spouse]
                    remember(realm, sp, a, "was unfaithful to me", -60, decay=1.0)
                    remember(realm, sp, b, "seduced my spouse", -80, decay=0.5)
                    sp.loves.pop(a.id, None) if hasattr(sp, "loves") else None
                    add_stress(realm, sp, 40, "a betrayal")
                    realm.say(f"**A scandal.** The affair of {realm.who(a)} and {realm.who(b)} comes out; "
                              f"{realm.who(sp)} will not forgive it.", minor=not (a.ruled or sp.ruled))
        # marriages warm or cool
        for c in circle:
            ensure(c, rng)
            if c.spouse is not None and realm.chars[c.spouse].alive and c.spouse not in c.loves:
                if opinion(realm, c, realm.chars[c.spouse]) > 35 and rng.random() < 0.08:
                    love(c, realm.chars[c.spouse], "spouse", rng.uniform(40, 70))
            if c.pet is None and rng.random() < 0.03:
                kind, life = PETS[int(rng.integers(len(PETS)))]
                c.pet = {"kind": kind, "name": str(rng.choice(PET_NAMES)), "since": y, "life": life}
    # love wears thin in the fickle
    for i in realm.living:
        c = realm.chars[i]
        if "fickle" in c.personality and getattr(c, "loves", None):
            for oid, rel in list(c.loves.items()):
                if rel[0] in ("lover", "soulmate") and rng.random() < 0.08:
                    del c.loves[oid]


# ---------------------------------------------------------------- grief and vengeance
def grieve(realm, c, by, amount, whom):
    ensure(c, realm.rng)
    c.grief += amount
    add_stress(realm, c, amount, f"losing {whom}")
    if by is not None and getattr(by, "id", None) is not None and by.id != c.id:
        from .opinion import remember
        remember(realm, c, by, f"killed my beloved {whom}", -100, decay=0.0 if "vengeful" in c.personality else 0.3)
        if "forgiving" not in c.personality:
            c.vendettas[by.id] = (realm.year, whom)


def on_death(realm, dead, by=None):
    """Everyone who loved the dead grieves; if someone is to blame, they remember."""
    lovers = getattr(dead, "loved_by", None)
    if lovers is None:                                                # an old save without the index: scan once
        lovers = [i for i in realm.living if dead.id in getattr(realm.chars[i], "loves", {})]
    for i in list(lovers):
        c = realm.chars.get(i)
        if c is None or not c.alive:
            continue
        rel = getattr(c, "loves", {}).get(dead.id)
        if rel is None:
            continue
        kind, strength = rel
        del c.loves[dead.id]
        word = {"child": "child", "parent": "parent", "spouse": "spouse", "lover": "lover",
                "soulmate": "soulmate"}.get(kind, "beloved")
        weight = {"child": 0.4 if dead.age(realm.year) < 5 else 0.9, "parent": 0.35, "soulmate": 1.0,
                  "spouse": 0.7, "lover": 0.6}.get(kind, 0.5)
        if by is not None:
            weight = max(weight, 1.0)                                 # a death someone caused cuts deepest
        grieve(realm, c, by, strength * weight, f"{word} {dead.full_name()}")
        if strength > 70 and kind in ("child", "soulmate", "spouse"):
            c.coping = c.coping if "heartbroken" in c.coping else c.coping + ["heartbroken"]
        if by is not None and c.ruled is not None and kind in ("child", "spouse", "soulmate", "lover") and \
                getattr(by, "realm", None) is not None and by.realm != c.realm:
            realm.say(f"{realm.who(c)} swears vengeance on {realm.who(by)} for the death of their {word} "
                      f"{dead.full_name()}.")
        realm.love_log.append((realm.year, c.id, dead.id, f"lost their {word}" + (f", blaming {by.full_name()}"
                                                                                    if by is not None else "")))


def vengeance_target(realm, r):
    """A living enemy a ruler has sworn to punish, and the realm they belong to."""
    for vid, (year, whom) in list(getattr(r, "vendettas", {}).items()):
        v = realm.chars.get(vid)
        if v is None or not v.alive:
            del r.vendettas[vid]
            continue
        if v.realm is not None and v.realm != r.realm and not realm.extinct[v.realm]:
            return v, whom
    return None, None


def avenged(realm, r, v, whom):
    r.vendettas.pop(v.id, None)
    r.stress = max(0.0, getattr(r, "stress", 0) - 80)
    r.grief = max(0.0, getattr(r, "grief", 0) - 40)
    realm.say(f"{realm.who(r)} has their vengeance on {v.full_name()} for {whom}.")


# ---------------------------------------------------------------- how a nature weighs a choice
def reluctance(c, act):
    """Stress a person takes from an act against their nature (negative is relief)."""
    p = c.personality
    if act == "execute":
        return 40 * ("compassionate" in p) + 30 * ("kind" in p) + 20 * ("forgiving" in p) + 15 * ("just" in p) \
            - 15 * ("cruel" in p) - 10 * ("callous" in p)
    if act == "war":
        return 30 * ("craven" in p) + 20 * ("compassionate" in p) + 10 * ("content" in p) - 10 * ("wrathful" in p) \
            - 10 * ("vengeful" in p) - 5 * ("brave" in p)
    if act == "feast":
        return 15 * ("shy" in p) - 10 * ("gregarious" in p) - 10 * ("gluttonous" in p)
    if act == "revoke":
        return 25 * ("just" in p) + 15 * ("kind" in p) - 10 * ("arbitrary" in p)
    if act == "defeat":
        return 40 + 30 * ("arrogant" in p) - 15 * ("humble" in p)
    return 0
