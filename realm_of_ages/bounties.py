"""Village bounties: towns that want a monster or a criminal dealt with put up a purse, and the purse draws hunters.

A town in a monster's shadow, or one that was raided lately, puts up a purse from its own wealth on the monster
that threatens it most; neighbours add to it, so a monster that menaces a whole valley carries a great bounty. A town
plagued by crime posts a reward for its most wanted. Hunters come from anywhere on the landmass and weigh the purse
against the danger; the greedy take longer odds. Whoever brings the monster down, on contract or not, collects every
purse on it."""
import math

from .data import MONSTER


def dist(a, b):
    return math.hypot(a[0] - b[0], a[1] - b[1])


def bounties_year(realm):
    post_monsters(realm)
    post_criminals(realm)
    # the purses are answered on the Wardens' Hall quest board (guild.py)


# ---------------------------------------------------------------- posting
def post_monsters(realm):
    rng = realm.rng
    by_land = {}
    for m in realm.monsters:
        if m.alive and not getattr(m, "sea", False) and m.kind != "calamity":
            by_land.setdefault(m.landmass, []).append(m)
    for t in realm.alive_towns():
        raided = realm.year - getattr(t, "raided_year", -99) <= 8
        if (getattr(t, "threat", 0.0) < 0.25 and not raided) or t.wealth < 30 or rng.random() > 0.3:
            continue
        near = [m for m in by_land.get(t.landmass, ()) if dist(m.pos, t.pos) <= MONSTER[m.kind][2] + 10]
        if not near:
            continue
        m = max(near, key=lambda m: m.power * math.exp(-dist(m.pos, t.pos) / 15))
        m.bounties = getattr(m, "bounties", {})
        if t.id in m.bounties and not raided:
            continue
        purse = min(0.2 * t.wealth, 3 + 0.01 * m.base) * (1.5 if raided else 1.0)
        t.wealth -= purse
        before = sum(m.bounties.values())
        m.bounties[t.id] = m.bounties.get(t.id, 0.0) + purse
        total = before + purse
        if before < 60 <= total:
            realm.say(f"**A great bounty.** The towns around {t.name} have put up {total:,.0f} gold for the head of "
                      f"{m.name}; hunters are on the roads.")
        else:
            realm.say(f"The elders of {t.name} put up {purse:,.0f} gold for whoever rids them of {m.name}.",
                      minor=True)


def post_criminals(realm):
    rng = realm.rng
    wanted = {}
    for i in realm.living:
        c = realm.chars[i]
        if c.profession in ("thief", "assassin", "bandit") and getattr(c, "jailed_until", 0) <= realm.year:
            wanted.setdefault(c.home, []).append(c)
    for t in realm.alive_towns():
        if getattr(t, "crime", 0.0) < 0.2 or t.wealth < 30 or rng.random() > 0.2:
            continue
        here = wanted.get(t.id)
        if not here:
            continue
        c = max(here, key=lambda c: c.bounty)
        purse = 0.1 * t.wealth
        t.wealth -= purse
        c.bounty += purse
        c.posted = getattr(c, "posted", 0.0) + purse
        realm.say(f"The town of {t.name} posts {purse:,.0f} gold for {c.full_name()}, dead or alive.", minor=True)


# ---------------------------------------------------------------- answering
def hunters_on(realm, landmass):
    out = []
    for h in realm.heroes():
        if not h.alive or h.hp < 50 or (h.ruled and h.ruled[2] is None):
            continue
        home = realm.by_id.get(h.home)
        if home is not None and home.landmass == landmass:
            out.append((h, home))
    return out


def answer_monsters(realm):
    from .heroes import hunt
    rng = realm.rng
    hunters = {}
    for m in list(realm.monsters):
        purse = sum(getattr(m, "bounties", {}).values()) if m.alive else 0.0
        if purse <= 0 or rng.random() > min(0.7, purse / (purse + 30)):
            continue
        if m.landmass not in hunters:
            hunters[m.landmass] = hunters_on(realm, m.landmass)
        best = None
        for h, home in hunters[m.landmass]:
            cr = h.cr() * h.mods()["vs_monster"]
            odds = cr / (cr + 0.6 * m.power)
            need = 0.25 if "greedy" in h.personality else 0.35
            if odds >= need and (best is None or odds > best[2]):
                best = (h, home, odds)
        if best is None:
            continue
        h, home, odds = best
        hunters[m.landmass] = [x for x in hunters[m.landmass] if x[0] is not h]
        realm.say(f"{realm.who(h)} takes the bounty on {m.name} ({purse:,.0f} gold).", minor=purse < 60)
        hunt(realm, h, m, home, realm.guilds[h.people])


def pay_out(realm, m, slayer):
    """Whoever brings a monster down collects every purse on it."""
    purse = sum(getattr(m, "bounties", {}).values())
    m.bounties = {}
    if purse <= 0 or slayer is None:
        return
    slayer.gold += purse
    from .events import private
    private(realm, slayer, "bounty", purse, f"{slayer.full_name()} collects the towns' bounty on {m.name}")
    realm.say(f"{realm.who(slayer)} collects {purse:,.0f} gold in bounties for {m.name}.", minor=purse < 60)


def answer_criminals(realm):
    from .society import hunt_bounty
    rng = realm.rng
    for i in list(realm.living):
        c = realm.chars[i]
        posted = getattr(c, "posted", 0.0)
        if posted <= 0 or not c.alive or getattr(c, "jailed_until", 0) > realm.year or rng.random() > min(0.6, posted / (posted + 40)):
            continue
        home = realm.by_id.get(c.home)
        if home is None:
            continue
        cands = [h for h, _ in hunters_on(realm, home.landmass) if h.realm == c.realm]
        if not cands:
            continue
        h = max(cands, key=lambda h: h.cr())
        hunt_bounty(realm, h)
