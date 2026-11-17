"""God mode: reach into the world. Every act is written in the chronicle as an act of the gods.

World acts: wake a calamity, loose a monster, smite a town, send plague, famine or a blessed harvest, grant
gold, declare war or impose peace, change a realm's government, raise a culture's era, start a golden age,
break a realm apart, curse or lift a curse from a house, name the Concord.

Characters: create one to your own design (name, people, sex, age, realm, class, level, every stat, every
trait, power bloodlines, skills, gold, hero or not), and change anything about any character afterwards. Any
character can be tracked: a tracked character keeps a journal of everything that happens to them.

Every act is done by the Rust world (World.god) and written into the world's record, so it is replayed on load."""
from .data import (ADJ, ADULT, ATTRS, BLOODLINES, CLASSES, GENETIC, PEOPLES, PERSONALITY, POWER_GENES, PRESTIGE)
from .data2 import SKILLS

OPPOSITE = {}
for _a, _b in PERSONALITY:
    OPPOSITE[_a], OPPOSITE[_b] = _b, _a
ALL_PERSONALITY = sorted(OPPOSITE)
ALL_GENETIC = sorted(GENETIC)
GOVERNMENTS = ["tribal", "clan", "feudal", "republic", "theocracy", "administrative"]
MONSTERS = ["troll", "wraith", "demon", "dragon"]


def _s(realm):
    return realm._session


def _id(x):
    return x if isinstance(x, int) else x.id


def living_realms(realm):
    return [k for k in realm.kingdoms if not k.extinct]


# ---------------------------------------------------------------- tracking
def track(realm, c, on=True):
    s = _s(realm)
    if on:
        s.tracked.add(c.id)
        s.journals.setdefault(c.id, []).append((realm.year, "god", f"Tracking begins: {c.full_name()}, aged {c.age(realm.year)}."))
    else:
        s.tracked.discard(c.id)
    s._dirty = True
    s.save()
    return f"{'Now tracking' if on else 'No longer tracking'} {c.full_name()}."


# ---------------------------------------------------------------- characters
def create_character(realm, name, people, female, age, realm_id, cls=None, level=1, attrs=None, prowess=None,
                     talent=None, personality=None, genetic=None, latent=None, skills=None, gold=0, hero=False,
                     tracked=True, home=None):
    """A character to your own design, placed in the world and (by default) tracked."""
    s = _s(realm)
    cid = int(s.god("create_character", {"name": name, "people": int(people), "female": bool(female), "age": int(age),
                                         "realm": int(realm_id), "cls": cls, "level": int(level), "attrs": attrs,
                                         "prowess": prowess, "talent": talent, "personality": list(personality) if personality is not None else None,
                                         "genetic": list(genetic or []), "latent": list(latent or []), "skills": skills,
                                         "gold": float(gold), "hero": bool(hero), "home": home}))
    c = s.realm.chars[cid]
    if tracked:
        track(s.realm, c, True)
    return s.realm.chars[cid]


def set_stat(realm, c, key, value):
    return _s(realm).god("set_stat", {"c": c.id, "key": key, "value": float(value)})


def set_trait(realm, c, trait, on=True):
    if trait not in OPPOSITE and trait not in GENETIC and not trait.endswith(("bound", "born", "heart", "-eye", "deepwell")):
        raise ValueError(f"unknown trait {trait}")
    return _s(realm).god("set_trait", {"c": c.id, "trait": trait, "on": bool(on)})


def set_class(realm, c, cls):
    if cls not in CLASSES and cls not in PRESTIGE:
        raise ValueError(f"unknown class {cls}")
    return _s(realm).god("set_class", {"c": c.id, "cls": cls})


def make_hero(realm, c):
    return _s(realm).god("make_hero", {"c": c.id})


def crown_character(realm, c):
    return _s(realm).god("crown_character", {"c": c.id})


def marry_characters(realm, a, b):
    return _s(realm).god("marry", {"a": a.id, "b": b.id})


def make_love(realm, a, b):
    return _s(realm).god("make_love", {"a": a.id, "b": b.id})


def give_item(realm, c, name, rarity=5):
    return _s(realm).god("give_item", {"c": c.id, "name": name, "rarity": int(rarity)})


def make_immortal(realm, c, on=True):
    return _s(realm).god("make_immortal", {"c": c.id, "on": bool(on)})


def make_custom(realm, c, on=True):
    """Make an existing character custom: the only kind of character who can go past the seventh tier of magic."""
    return _s(realm).god("make_custom", {"c": c.id, "on": bool(on)})


def strike_down(realm, c):
    return _s(realm).god("strike_down", {"c": c.id})


def raise_dead(realm, c):
    return _s(realm).god("raise_dead", {"c": c.id})


# ---------------------------------------------------------------- the world
def wake_calamity(realm, lid, world=False):
    return _s(realm).god("wake_calamity", {"lid": int(lid), "world": bool(world)})


def loose_monster(realm, kind, kid):
    return _s(realm).god("loose_monster", {"kind": kind, "kid": int(kid)})


def smite_town(realm, town):
    return _s(realm).god("smite_town", {"town": _id(town)})


def send_plague(realm, kid, great=True):
    return _s(realm).god("send_plague", {"kid": int(kid), "great": bool(great)})


def set_harvest(realm, kid, mult, years=3):
    return _s(realm).god("set_harvest", {"kid": int(kid), "mult": float(mult), "years": int(years)})


def grant_gold(realm, kid, amount):
    return _s(realm).god("grant_gold", {"kid": int(kid), "amount": float(amount)})


def declare_war(realm, a, b):
    return _s(realm).god("declare_war", {"a": int(a), "b": int(b)})


def impose_peace(realm, a, b):
    return _s(realm).god("impose_peace", {"a": int(a), "b": int(b)})


def change_government(realm, kid, gov):
    return _s(realm).god("change_government", {"kid": int(kid), "gov": gov})


def raise_era(realm, culture):
    return _s(realm).god("raise_era", {"culture": int(culture)})


def golden_age(realm, culture):
    return _s(realm).god("golden_age", {"culture": int(culture)})


def break_realm(realm, kid, lid):
    """Tear a realm's holdings on one landmass away into a realm of their own."""
    return _s(realm).god("break_realm", {"kid": int(kid), "lid": int(lid)})


def curse_house(realm, hid):
    return _s(realm).god("curse_house", {"hid": int(hid)})


def name_concord(realm, c):
    return _s(realm).god("name_concord", {"c": c.id})
