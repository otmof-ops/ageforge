"""Skills that rise by use, eight attributes that rise with them, and perks every twenty points."""
from .data import ADULT
from .data2 import (ATTRIBUTES, CLASS_SKILLS, PERK_AT, PERKS, RACE_ATTR, RACE_SKILL, SCHOOL_SKILL, SKILLS,
                    WEAPON_SKILL)


def init_skills(c, rng):
    c.attributes = {a: int(rng.integers(30, 46)) + RACE_ATTR[c.people].get(a, 0) for a in ATTRIBUTES}
    if "strong" in c.genetic or "titanblood" in c.genetic:
        c.attributes["str"] += 10
    if "genius" in c.genetic:
        c.attributes["int"] += 10
    if "comely" in c.genetic or "silver-tongued" in c.genetic:
        c.attributes["per"] += 8
    c.skills = {s: float(rng.integers(3, 13)) + RACE_SKILL[c.people].get(s, 0) for s in SKILLS}
    for s in CLASS_SKILLS.get(c.classes[0][0], []):
        c.skills[s] += 12
    c.perks = []
    c.skill_progress = {a: 0.0 for a in ATTRIBUTES}


def skill(realm, c, s):
    """A skill as it is used: trained level plus whatever legacy the family carries in it."""
    from .lineage import legacy_bonus
    return min(120.0, c.skills.get(s, 0.0) + legacy_bonus(realm, c, f"skill:{s}"))


def perk_count(c, s):
    return sum(1 for p in c.perks if p[0] == s)


def use(realm, c, s, amount=1.0):
    """Practice: the closer to mastery, the slower the climb. Guild training and a governing attribute help."""
    if not c.alive or s not in c.skills:
        return
    if s == "Beast Taming" and ("beastspeaker" not in c.genetic or getattr(c, "merged", None)):
        return                                   # a rare gift; and once merged, taming is a thing of the past
    gov = SKILLS[s][0]
    before = c.skills[s]
    rate = amount * 2.2 * (1 - before / 110) * (0.7 + c.attributes[gov] / 100)
    if getattr(c, "guilds", None):
        rate *= 1.2
    after = min(100.0, before + rate)
    c.skills[s] = after
    c.skill_progress[gov] += after - before
    while c.skill_progress[gov] >= 10:
        c.skill_progress[gov] -= 10
        c.attributes[gov] = min(100, c.attributes[gov] + 1)
    for i, at in enumerate(PERK_AT):
        if before < at <= after:
            c.perks.append((s, PERKS[s][i]))
            c.invalidate()
            if at == 100:
                realm.say(f"{realm.who(c)} masters {s}, and earns the perk {PERKS[s][i]}.",
                          minor=True)
                from .crafts import gain_fame
                gain_fame(realm, c, s, 60)


def train_class(realm, c, amount=1.0):
    for s in CLASS_SKILLS.get(c.classes[0][0], []):
        use(realm, c, s, amount)


def fight_skills(realm, c, amount=1.0):
    cls = c.classes[0][0]
    use(realm, c, WEAPON_SKILL.get(cls, "Blade"), amount)
    use(realm, c, "Heavy Armor" if cls in ("Warrior", "Knight", "Runesmith") else "Light Armor", amount * 0.7)
    use(realm, c, "Block", amount * 0.4)
    s = c.school()
    if s is not None:
        use(realm, c, SCHOOL_SKILL[s], amount)


def phys_mult(realm, c):
    cls = c.classes[0][0]
    w = skill(realm, c, WEAPON_SKILL.get(cls, "Blade"))
    a = c.attributes
    return (1 + w / 100 * 0.6 + 0.05 * perk_count(c, WEAPON_SKILL.get(cls, "Blade"))) * \
        (1 + (a["str"] + a["agi"] - 80) / 300)


def magic_mult(realm, c):
    s = c.school()
    if s is None:
        return 1.0
    sk = SCHOOL_SKILL[s]
    a = c.attributes
    return (1 + skill(realm, c, sk) / 100 * 0.6 + 0.05 * perk_count(c, sk)) * (1 + (a["int"] + a["wil"] - 80) / 300)


def hp_mult(realm, c):
    armor = max(skill(realm, c, "Heavy Armor"), skill(realm, c, "Light Armor"))
    return (1 + armor / 250 + skill(realm, c, "Block") / 400) * (1 + (c.attributes["end"] - 40) / 300)


def best_skills(realm, c, n=6):
    return sorted(((skill(realm, c, s), s) for s in c.skills), reverse=True)[:n]
