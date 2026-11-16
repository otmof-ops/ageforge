"""Youth: what a child does with what was handed down to them.

From the age of six until they come of age, a child of notable parents may practise, year by year, whatever
their parents were best at: their best skills, their strongest attributes, their gifts for war or rule or
lore. Diligent children practise nearly every year, lazy ones seldom. Each year of practice closes part of the
gap between child and parent. A child marked by fate (the Hero's Mark, the Sealing Light) is drilled twice as
hard by old masters who know what is coming, and comes of age already spoken of: renown and fame in proportion
to the years they put in."""
from .characters import gain_xp
from .data import ADULT, ATTRS
from .data2 import ATTRIBUTES, HERO_MARK, SEER_GIFT

START = 6


def marked(c):
    return bool(set(getattr(c, "marks", [])) & {HERO_MARK, SEER_GIFT})


def youth_year(realm):
    rng = realm.rng
    y = realm.year
    for i in list(realm.living):
        c = realm.chars[i]
        age = c.age(y)
        adult = ADULT[c.people]
        if age < START or age > adult:
            continue
        parents = [realm.chars[p] for p in (c.father, c.mother) if p is not None]
        if not parents and not marked(c):
            continue
        if age == adult:
            come_of_age(realm, c, parents)
            continue
        p = 0.6 + 0.25 * ("diligent" in c.personality) - 0.35 * ("lazy" in c.personality) + 0.1 * ("ambitious" in c.personality)
        mult = 2.0 if marked(c) else 1.0
        if marked(c):
            p += 0.3
        if rng.random() > p:
            continue
        c.practice = getattr(c, "practice", 0) + 1
        # each year the young one closes part of the gap to the best of their parents in what they were best at
        rate = 0.05 * mult * (20 / max(10, adult - START))
        for par in parents:
            if not getattr(par, "skills", None):
                continue
            for val, s in sorted(((v, s) for s, v in par.skills.items()), reverse=True)[:4]:
                if val > c.skills.get(s, 0):
                    c.skills[s] += (val - c.skills[s]) * rate
            for a in sorted(ATTRIBUTES, key=lambda a: -par.attributes.get(a, 0))[:2]:
                if par.attributes[a] > c.attributes[a]:
                    c.attributes[a] += max(1, int((par.attributes[a] - c.attributes[a]) * rate))
            for a in ATTRS:
                if par.attrs.get(a, 0) > c.attrs.get(a, 0) and rng.random() < rate * 2:
                    c.attrs[a] += 1
        if marked(c):                                               # the old masters' drills: a fighter's body
            from .skills import fight_skills
            fight_skills(realm, c, 1.0)
        gain_xp(c, (30 + 4 * age) * mult, realm)
        c.invalidate()


def come_of_age(realm, c, parents):
    from .court import educate
    educate(realm, c, parents)
    years = getattr(c, "practice", 0)
    if years and parents:
        best = max(((v, s) for s, v in c.skills.items()), default=(0, None))
        c.trained_in = best[1]
    if not marked(c):
        return
    from .crafts import gain_fame
    renown = 80 + 25 * years
    c.renown += renown
    spec = getattr(c, "trained_in", None) or "Blade"
    gain_fame(realm, c, spec, 30 * years)
    who = " and ".join(p.full_name() for p in parents) if parents else "no one living"
    mark = HERO_MARK if HERO_MARK in getattr(c, "marks", []) else SEER_GIFT
    realm.say(f"{realm.who(c)}, who bears {mark}, comes of age after {years} years of practice in what {who} "
              f"passed down to them; the songs already speak of them (renown {c.renown:,.0f}).",
              minor=years < 5)
