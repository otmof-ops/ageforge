"""The common people: estates and their moods, risings, and the plague.

Every town is peasants, burghers, clergy and nobles. The peasants' mood follows bread (famine), sickness, taxes
(the crown's authority and the government's appetite), war, occupation, tyranny, a ruler of another faith, and
good lords; the burghers' follows trade, tolls and crime. Peasants pushed far enough rise: a rebel host marches
on the crown, and a rising that holds its town for three years becomes a free commune, a realm of its own.

Plague moves from town to town along the roads and the sea lanes. Every town has a share of its people who are
immune from the last outbreak, and it fades as the survivors die. Small outbreaks are common; a great pandemic
comes perhaps once in two or three lifetimes and kills a third or more of every town it reaches. Healers
(a court physician, priests and masters of restoration) save some."""
import math

import numpy as np

from .data import PLAGUES
from .geography import land_name

ESTATES = {"peasants": 0.84, "burghers": 0.08, "clergy": 0.04, "nobles": 0.04}
DISEASE_A = ["Grey", "Red", "Black", "Sweating", "Pale", "Weeping", "Burning", "Spotted", "Bone", "Blue"]
DISEASE_B = ["Death", "Fever", "Cough", "Pox", "Rot", "Flux", "Wasting", "Sickness", "Plague"]


class Epidemic:
    def __init__(self, name, start, fatality, spread, great):
        self.name, self.start, self.fatality, self.spread, self.great = name, start, fatality, spread, great
        self.deaths, self.towns, self.end = 0.0, set(), None


# ---------------------------------------------------------------- moods
def moods_year(realm):
    rng = realm.rng
    for k in realm.kingdoms:
        if realm.extinct[k.id] or k.ruler is None:
            continue
        r = realm.chars[k.ruler]
        tax = getattr(k, "tax_now", 0.3)
        at_war = realm.war[k.id].any()
        relief = getattr(k, "tax_relief", 0) > realm.year
        for t in realm.alive_towns(k.id):
            if not hasattr(t, "estates"):
                t.estates = dict(ESTATES)
                t.mood = {"peasants": 10.0, "burghers": 10.0}
            lord = realm.chars.get(t.lord) if t.lord is not None else None
            faith_ok = getattr(t, "faith", None) and max(t.faith, key=t.faith.get) == r.religion
            p = (12 - 120 * getattr(t, "famine", 0) - 60 * getattr(t, "sick_now", 0) - 60 * (tax - 0.25)
                 - 20 * (getattr(t, "occupied_by", None) is not None) - 6 * at_war - r.tyranny / 3
                 - (0 if faith_ok else 10) + (5 if lord is not None and (lord.legitimacy or 0) >= 60 else 0)
                 + (15 if relief else 0) + getattr(t, "econ_mood", 0.0))
            b = 10 + 3 * min(10.0, getattr(t, "trade", 0.0)) - 40 * getattr(t, "crime", 0.1) - 40 * (tax - 0.25) \
                - (15 if getattr(k, "laws", {}).get("trade") == "protectionist" else 0)
            t.mood["peasants"] += 0.3 * (p - t.mood["peasants"])
            t.mood["burghers"] += 0.3 * (b - t.mood["burghers"])
            # burghers grow with trade and the guilds
            share = 0.06 + 0.04 * t.port + 0.04 * ("Guild Charters" in realm.cultures[t.culture].innovations)
            t.estates["burghers"] += 0.05 * (share - t.estates["burghers"])
            if getattr(t, "rising", None) and realm.year - t.rising > 30:
                t.rising = None                                       # an old rising is long over
            if t.mood["peasants"] < -15 and not getattr(t, "rising", None) and t.pop > 200 and \
                    rng.random() < 0.05 * (-t.mood["peasants"] - 15) / 20:
                rise(realm, k, t)


def rise(realm, k, t):
    from .characters import make_character
    from .warfare import raise_rebels
    leader = make_character(realm, k.id, born=realm.year - 25 - int(realm.rng.integers(0, 20)))
    leader.home = t.id
    leader.personality.add("brave")
    why = "bread" if getattr(t, "famine", 0) > 0.05 else "the plague" if getattr(t, "sick_now", 0) > 0.05 else \
        "the taxes" if getattr(k, "tax_now", 0.3) > 0.3 else "the tyrant" if realm.chars[k.ruler].tyranny >= 50 else \
        "their lord"
    t.rising = realm.year
    # the discontented of the towns around rally to it
    for o in realm.alive_towns(k.id):
        if o is not t and o.landmass == t.landmass and getattr(o, "mood", {}).get("peasants", 0) < -10 and \
                math.hypot(o.pos[0] - t.pos[0], o.pos[1] - t.pos[1]) < 18:
            o.rising = realm.year
            t.rallied = getattr(t, "rallied", 0.0) + o.pop * 0.08
    realm.risings.append((realm.year, t.id, k.id, why))
    raise_rebels(realm, k, t, leader, why)
    realm.say(f"**A rising.** The peasants of {t.name} rise against {k.name} over {why}, led by "
              f"{leader.full_name()}.", minor=t.pop < 1500)


# ---------------------------------------------------------------- plague
def plague_year(realm):
    rng = realm.rng
    y = realm.year
    towns = realm.alive_towns()
    if not towns:
        return
    for t in towns:
        t.immune = getattr(t, "immune", 0.0) * 0.96                   # survivors die, children are born
        t.sick_now = 0.0
    live = [e for e in realm.epidemics if e.end is None]
    # new outbreaks
    great = rng.random() < 0.004 and y - realm.last_pandemic > 120
    if great or rng.random() < 0.025:
        src = towns[int(rng.integers(len(towns)))]
        name = f"the {rng.choice(DISEASE_A)} {rng.choice(DISEASE_B)}"
        e = Epidemic(name, y, rng.uniform(0.35, 0.6) if great else rng.uniform(0.05, 0.2),
                     rng.uniform(0.5, 0.8) if great else rng.uniform(0.2, 0.45), great)
        e.front = {src.id: 0.25 if great else 0.1}
        realm.epidemics.append(e)
        live.append(e)
        if great:
            realm.last_pandemic = y
            realm.say(f"**A great pestilence.** {e.name[0].upper() + e.name[1:]} breaks out in {src.name} on "
                      f"{land_name(realm, src.landmass)}.")
        realm.last_plague = y
    if not live:
        return
    pos = np.array([t.pos for t in towns], float)
    port = np.array([t.port for t in towns])
    by_id = {t.id: i for i, t in enumerate(towns)}
    heal = {}
    for k in realm.kingdoms:
        if realm.extinct[k.id]:
            continue
        healers = sum(1 for c in realm.members(k.id) if c.profession == "priest" or c.skills.get("Restoration", 0) >= 70)
        heal[k.id] = min(0.4, 0.04 * healers + (0.1 if getattr(k, "physician", False) else 0)
                         + 0.01 * getattr(k, "potions", 0))
    for e in live:
        new = {}
        for tid, load in e.front.items():
            i = by_id.get(tid)
            if i is None:
                continue
            t = towns[i]
            attack = min(0.9, load * (1 - t.immune) * 3)
            mend = min(0.3, 20 * getattr(t, "arcane", 0.0)) + (0.15 if "Mender's Touch" in
                                                                getattr(realm, "town_gifts", {}).get(t.id, ()) else 0)
            dead = t.pop * attack * e.fatality * (1 - min(0.7, heal.get(t.faction, 0) + mend))
            t.pop -= dead
            t.immune = min(0.95, t.immune + attack)
            t.sick_now = max(t.sick_now, attack)
            e.deaths += dead
            e.towns.add(t.id)
            if attack < 0.02:
                continue
            # it travels: along the roads to near towns, and along the sea lanes from port to port
            d = np.hypot(pos[:, 0] - t.pos[0], pos[:, 1] - t.pos[1])
            road = (d < 12) & (d > 0)
            sea = port & (d < 45) & t.port
            for j in np.flatnonzero(road | sea):
                if rng.random() < e.spread * attack * (1.6 if sea[j] else 1.0):
                    o = towns[int(j)]
                    if o.id not in e.towns or rng.random() < 0.1:
                        new[o.id] = max(new.get(o.id, 0), load * 0.8)
            # the great of the town die like anyone else, if a little less often
            if dead > 50 and rng.random() < 0.3:
                for c in [c for c in realm.members(t.faction) if c.home == t.id and not c.undead]:
                    if rng.random() < attack * e.fatality * 0.4:
                        realm.kill(c, f"died of {e.name}")
        e.front = new
        if not new:
            e.end = y
            if e.deaths > 2000 or e.great:
                realm.say(f"{'**' if e.great else ''}{e.name[0].upper() + e.name[1:]} burns out after {y - e.start + 1} years: "
                          f"{len(e.towns)} towns, some {e.deaths:,.0f} dead.{'**' if e.great else ''}",
                          minor=not e.great and e.deaths < 20000)
