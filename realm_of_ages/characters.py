"""People: heroes, rulers and their kin. Levels, classes, skill trees, tiered magic, items and character sheets."""
import math

from .lineage import inherit_legacy, inherit_power_genes, legacy_lines
from .data import (BLOOD_LEVEL_CAP, BLOODLINES, LEVEL_CAP, OPPOSED, PROWESS_CAP, TIER_CAP, ADJ, ADULT, ATTR_NAMES, ATTRS, BLOODLINES, CASTERS, CLASS_WEIGHTS, CLASSES, FIRST, GENETIC,
                   GENETIC_FREQ, LIFESPAN, PEOPLES, PERSONALITY, PERSONALITY_FX, PRESTIGE, PROWESS_BASE, RARITY,
                   RARITY_BONUS, ROMAN, SCHOOLS, SINGULAR, SPELLS, TALENT_BASE, TIER_WORDS, spell_power)

MULT_KEYS = ("phys", "magic", "hp", "war", "guard", "vs_monster", "delve", "equip")
ADD_KEYS = ("tier", "lead", "intrigue")
MAX_LEVEL = 100
CTX = {}                     # the running realm, so a person's power can see their house and legacies


def xp_needed(level):
    """Experience for the next level: past the twenties every level is a long road, so most heroes of the world
    settle in their twenties and thirties, and the forties and beyond belong to legends."""
    return 10 * level ** 1.2 * (1 + max(0, level - 25) / 5) ** 2


def power_gate(c, martial=False):
    """The highest tier of power a mortal can reach, magic or martial, by the world's own measure: three for anyone
    capable; four for rare masters of talent and long training; five and six for the near-mythic, who need a gift of
    power besides. Characters the player makes have no ordinary limit."""
    d = c.__dict__
    if d.get("custom"):
        return 10
    lv = c.level
    if martial:
        cap = PROWESS_CAP[c.people]
        skill = c.prowess / max(1.0, cap)
        gift = getattr(c, "gift", None) in ("Iron Skin", "Born Captain") or bool({"titanblood", "dragonblood"} & c.genetic)
    else:
        skill = c.eff_talent() / 10
        gift = getattr(c, "gift", None) == "Mana Well" or bool({"starblood", "abyssblood"} & c.genetic) or \
            any(it.special == "tier" for it in c.items)
    g = 3
    if skill >= 0.6 and lv >= 22:
        g = 4
        if skill >= 0.85 and lv >= 40 and gift:
            g = 5
            if skill >= 0.95 and lv >= 55:
                g = 6
    if d.get("concord"):
        g = max(g, 5)
    return g


class House:
    def __init__(self, hid, name, people, founder, year):
        self.id, self.name, self.people, self.founder, self.founded = hid, name, people, founder, year
        self.prestige, self.rulers, self.reign_years = 0.0, 0, 0
        self.track, self.legacy, self.origin = "ruler", 0.0, ""
        self.parent, self.dynasty = None, hid


class Item:
    def __init__(self, name, rarity, year, origin, special=None, note=""):
        self.name, self.rarity, self.year, self.origin = name, rarity, year, origin
        self.special, self.note = special, note
        self.bearers = []

    def label(self):
        desc = getattr(self, "desc", None)
        ench = getattr(self, "enchants", None)
        extra = f", {desc}" if desc else ""
        if ench:
            extra += ", " + " and ".join(f"{t} {e}" for e, t in ench)
        return f"{self.name} ({RARITY[self.rarity]}{extra})"

    @property
    def bonus(self):
        b = getattr(self, "_bonus", None)
        return b if b is not None else RARITY_BONUS[self.rarity]


class Character:
    def __init__(self, cid, name, people, female, born):
        self.id, self.name, self.people, self.female, self.born = cid, name, people, female, born
        self.house = self.father = self.mother = self.spouse = None
        self.children = []
        self.genetic, self.personality = set(), set()
        self.attrs = {a: 5 for a in ATTRS}
        self.prowess, self.talent, self.life = 8.0, 2.0, float(LIFESPAN[people])
        self.classes = []                     # [[class name, levels]]
        self.level, self.xp, self.hp, self.renown = 1, 0.0, 100.0, 0.0
        self.branch = {}                      # (class, branch index) -> skills learned (0-4)
        self.capstones = []
        self.items = []
        self.deeds = {}
        self.titles = []
        self.epithet = None
        self.hero = self.undead = False
        self.alive, self.died, self.cause = True, None, None
        self.home = None
        self.pending = None
        self.rite_ready = 0
        self.peak_cr, self.peak_tier = 0.0, -1
        self.regnal = None
        self.ruled = None                     # (people, start, end)
        self.latent = set()                   # power genes carried but not shown
        self.legacy, self.legacy_origin = {}, {}
        self.legitimacy, self.tyranny = None, 0.0
        self.lordships = []
        self.last_deed, self.disgraced_until = None, 0
        self.gold, self.fame, self.profession, self.bounty = 0.0, {}, None, 0.0
        self.religion, self.guilds, self.guild_rank = people, [], None
        self.clan = self.element = self.tribe = None
        self.concord = False
        self.realm = people
        self._mods = None

    # ---------------- identity
    def age(self, year):
        return year - self.born

    def full_name(self, houses=None):
        n = self.name + (f" {self.regnal}" if self.regnal else "")
        if self.epithet:
            n += f" {self.epithet}"
        return n

    def house_name(self, houses):
        return houses[self.house].name if self.house is not None else "no house"

    def adult(self, year):
        return self.age(year) >= ADULT[self.people]

    # ---------------- skills and power
    def invalidate(self):
        """Something about this person changed: drop every cached figure derived from them."""
        self._mods = None
        self._attr_c = None
        self._lb = None
        self._cr = None
        self._tier = None

    def mods(self):
        if self._mods is not None:
            return self._mods
        m = {k: 1.0 for k in MULT_KEYS}
        m.update({k: 0 for k in ADD_KEYS})

        def apply(key, val):
            if key in ADD_KEYS:
                m[key] += val
            else:
                m[key] *= val
        for (cls, b), n in self.branch.items():
            key = CLASSES[cls]["branches"][b][1]
            if n <= 0:
                continue
            if key == "tier":
                apply("magic", 1 + 0.05 * n)
                if n >= 4:
                    apply("tier", 1)
            elif key == "lead":
                apply("lead", 2 * n)
            elif key == "intrigue":
                apply("intrigue", 3 * n)
            else:
                apply(key, (1 + 0.08 * n) * (1.1 if n >= 4 else 1.0))
        for cls in self.capstones:
            for k, v in CLASSES[cls]["capstone"][1].items():
                apply(k, v)
        for cls, _ in self.classes:
            if cls in PRESTIGE:
                for k, v in PRESTIGE[cls]["fx"].items():
                    apply(k, v)
        realm = CTX.get("realm")
        if realm is not None:
            from .lineage import legacy_bonus
            for key in ("vs_monster", "delve", "guard"):
                b = legacy_bonus(realm, self, key)
                if b:
                    apply(key, 1 + b)
        if "titanblood" in self.genetic:
            apply("phys", 1.3)
        if "abyssblood" in self.genetic:
            apply("magic", 1.3)
        for it in self.items:
            if it.special == "tier":
                apply("tier", 1)
            elif it.special == "guard":
                apply("guard", 2.0)
            elif it.special == "magic":
                apply("magic", 1.5)
            elif it.special == "vs_monster":
                apply("vs_monster", 2.0)
        self._mods = m
        return m

    def class_def(self, cls):
        return CLASSES.get(cls) or PRESTIGE[cls]

    def school(self):
        best, bl = None, 0
        for cls, lv in self.classes:
            if cls in CLASSES and CLASSES[cls]["school"] is not None and lv > bl:
                best, bl = CLASSES[cls]["school"], lv
        return best

    def tier(self):
        """Highest tier of magic this person can cast (cached until this person changes)."""
        v = self.__dict__.get("_tier")
        if v is None:
            v = self._tier = self._tier_calc()
        return v

    def _tier_calc(self):
        """Highest tier of magic this person can cast: training (caster levels) capped by talent, plus gifts."""
        if self.school() is None:
            return -1
        eff = sum(lv * min(1.0, self.class_def(cls)["magic"]) for cls, lv in self.classes)
        base = int(eff / 9) if "abyssblood" in self.genetic else min(int(eff / 9), int(self.eff_talent()))
        well = getattr(self, "gift", None) == "Mana Well"
        cap = self.tier_cap() + sum(1 for it in self.items if it.special == "tier") + well
        limit = power_gate(self)                                        # the world's measure of a mortal
        return max(0, min(limit, cap, base + self.mods()["tier"] + well))

    def tier_cap(self):
        if self.__dict__.get("custom"):                                # only characters the player makes go further
            return 10
        return TIER_CAP[self.people] + ("starblood" in self.genetic)

    def level_cap(self):
        return BLOOD_LEVEL_CAP if "dragonblood" in self.genetic else LEVEL_CAP

    def eff_talent(self):
        realm = CTX.get("realm")
        bonus = 0.0
        if realm is not None:
            from .lineage import legacy_bonus
            bonus = legacy_bonus(realm, self, "talent")
        return min(10.0, self.talent + bonus)

    def eff_prowess(self):
        realm = CTX.get("realm")
        v = self.prowess
        if realm is not None:
            from .lineage import legacy_bonus
            v += legacy_bonus(realm, self, "prowess")
            from .court import age_penalty, perk_value
            v += perk_value(self, "prowess") - age_penalty(realm, self)
        from .arcana import arts_bonus
        return (v if "titanblood" in self.genetic else min(v, PROWESS_CAP[self.people])) * arts_bonus(self)

    def hp_mult(self):
        lv = max(1, sum(l for _, l in self.classes))
        v = sum(self.class_def(c)["hp"] * l for c, l in self.classes) / lv * self.mods()["hp"]
        realm = CTX.get("realm")
        if realm is not None and getattr(self, "skills", None):
            from . import skills as sk
            v *= sk.hp_mult(realm, self)
        return v

    def cr(self, hp_matters=True):
        """Combat rating; the base figure is cached for the year, and wounds scale it."""
        realm = CTX.get("realm")
        yr = realm.year if realm is not None else None
        cache = self.__dict__.get("_cr")
        if cache is not None and cache[0] == yr:
            v = cache[1]
        else:
            v = self._cr_base()
            self._cr = (yr, v)
        return v * (0.5 + 0.5 * self.hp / 100) if hp_matters else v

    def _cr_base(self):
        """Combat rating: training and prowess, the best spell they can cast, and what they carry."""
        m = self.mods()
        phys_lv = sum(lv * self.class_def(cls)["phys"] for cls, lv in self.classes)
        phys = 2 * (phys_lv + max(0.0, self.eff_prowess())) ** 1.2 * m["phys"]
        realm = CTX.get("realm")
        if realm is not None and getattr(self, "skills", None):
            from . import skills as sk
            phys *= sk.phys_mult(realm, self)
        t = self.tier()
        mag = 0.0
        if t >= 0:
            mw = max(self.class_def(cls)["magic"] for cls, _ in self.classes)
            mag = spell_power(t) * mw * m["magic"]
            if realm is not None and getattr(self, "skills", None):
                mag *= sk.magic_mult(realm, self)
        best = sorted((it.bonus for it in self.items), reverse=True)[:4]
        eq = 1 + sum(best) * m["equip"]
        v = (phys + mag) * eq
        if self.element is not None and realm is not None:
            from .bending import bend_power
            v += bend_power(realm, self)
        if getattr(self, "beast", None) and realm is not None:
            from .beasts import companion_power
            v += companion_power(self, realm.year)
        if self.clan is not None and getattr(self, "ki", None):
            from .shinobi import shinobi_power
            v += shinobi_power(self)
        return v

    def attr(self, a):
        """A base stat. Everything but age is cached until this person changes (invalidate); age is applied live."""
        realm = CTX.get("realm")
        cache = self.__dict__.get("_attr_c")
        if cache is None:
            cache = self._attr_c = {}
        v = cache.get(a)
        if v is None:
            v = cache[a] = self._attr(a, realm)
        if realm is not None and a not in ("lore", "lrn") and getattr(realm, "kingdoms", None):
            from .court import age_penalty
            v -= age_penalty(realm, self)
        return v

    def _attr(self, a, realm):
        v = self.attrs.get(a, 5) + self.level // 15
        if realm is not None and a in ("mar", "dip"):
            from .lineage import legacy_bonus
            v += int(legacy_bonus(realm, self, a))
        if a == "mar":
            v += self.mods()["lead"]
        if a == "intr":
            v += self.mods()["intrigue"]
        if realm is not None and getattr(realm, "kingdoms", None):
            from .court import stat_bonus
            v += stat_bonus(realm, self, a)
        return v

    def known_spells(self, n=3):
        t, s = self.tier(), self.school()
        if t < 0:
            return []
        return [f"{ROMAN[k]} {SPELLS[s][k]}" for k in range(t, max(-1, t - n), -1)]


# ---------------------------------------------------------------- making people
def first_name(rng, people, female):
    pre_m, pre_f, suf_m, suf_f = FIRST[people]
    return str(rng.choice(pre_f if female else pre_m)) + str(rng.choice(suf_f if female else suf_m))


def pick_class(rng, people, talent, exclude=()):
    w = {c: v * (max(0.3, talent / 3) if c in CASTERS else 1.0) for c, v in CLASS_WEIGHTS[people].items()
         if c not in exclude}
    names = list(w)
    p = [w[c] for c in names]
    tot = sum(p)
    return names[int(rng.choice(len(names), p=[x / tot for x in p]))]


def make_character(realm, people, born, female=None, father=None, mother=None, house=None, hero=False):
    rng = realm.rng
    if female is None:
        female = bool(rng.random() < 0.5)
    pid = people                                        # the realm they are born into...
    if realm.kingdoms and 0 <= pid < len(realm.kingdoms):
        people = realm.kingdoms[pid].people             # ...and the people they are born of
    c = Character(realm.next_cid, first_name(rng, people, female), people, female, born)
    c.realm = pid
    realm.next_cid += 1
    c.house, c.father, c.mother = house, (father.id if father else None), (mother.id if mother else None)
    parents = [p for p in (father, mother) if p is not None]
    # inherited and new genetic traits
    pool = set().union(*(p.genetic for p in parents)) if parents else set()
    for t in pool:
        if t.startswith("half-") or t in BLOODLINES:
            continue
        both = all(t in p.genetic for p in parents) and len(parents) == 2
        p_in = (0.9 if both else 0.5) if t in BLOODLINES else (0.8 if both else 0.45)
        if rng.random() < p_in:
            c.genetic.add(t)
    for t, f in GENETIC_FREQ.items():
        if rng.random() < f:
            c.genetic.add(t)
    for a, b in OPPOSED:
        if a in c.genetic and b in c.genetic:
            c.genetic.discard(b)
    inherit_power_genes(c, parents, rng)
    if len(parents) == 2 and father.people != mother.people:
        other = father.people if father.people != people else mother.people
        c.genetic.add(f"half-{ADJ[other]}")
    # personality: three of the nine axes, sometimes echoing a parent
    axes = rng.choice(len(PERSONALITY), 4, replace=False)
    for ax in axes:
        a, b = PERSONALITY[ax]
        inherited = [t for p in parents for t in (a, b) if t in p.personality]
        if inherited and rng.random() < 0.35:
            c.personality.add(inherited[int(rng.integers(len(inherited)))])
        else:
            c.personality.add(a if rng.random() < 0.5 else b)
    # attributes, prowess, talent
    for a in ATTRS:
        base = rng.normal(5, 2)
        if parents:
            base = 0.5 * base + 0.5 * sum(p.attrs[a] for p in parents) / len(parents)
        c.attrs[a] = base
    c.prowess = PROWESS_BASE[people] + rng.normal(0, 3)
    c.talent = TALENT_BASE[people] + rng.normal(0, 1.1)
    if parents:
        c.talent = 0.5 * c.talent + 0.5 * (sum(p.talent - sum(GENETIC.get(t, {}).get("talent", 0)
                                                               for t in p.genetic) for p in parents) / len(parents))
    life = LIFESPAN[people] * rng.uniform(0.85, 1.15)
    if any(t.startswith("half-") for t in c.genetic):
        life = 0.5 * life + 0.5 * LIFESPAN[father.people if father.people != people else mother.people]
    for t in c.genetic:
        fx = GENETIC.get(t, {})
        for a in ATTRS:
            c.attrs[a] += fx.get(a, 0)
        c.prowess += fx.get("prowess", 0)
        c.talent += fx.get("talent", 0)
        life *= fx.get("life", 1.0)
    for t in c.personality:
        fx = PERSONALITY_FX.get(t, {})
        for a in ATTRS:
            c.attrs[a] += fx.get(a, 0) + fx.get("all", 0)
    c.attrs = {a: int(round(max(0, v))) for a, v in c.attrs.items()}
    c.talent = float(min(10.0, max(0.0, c.talent)))
    c.life = life
    c.classes = [[pick_class(rng, people, c.talent), 1]]
    c.hero = hero
    inherit_legacy(c, parents)
    from .skills import init_skills
    init_skills(c, rng)
    if parents:
        c.religion = (mother or father).religion
    realm.chars[c.id] = c
    realm.living.add(c.id)
    if father is not None:
        father.children.append(c.id)
    if mother is not None:
        mother.children.append(c.id)
    from .arcana import maybe_gift
    maybe_gift(realm, c)
    return c


# ---------------------------------------------------------------- levels and the skill trees
def prestige_options(c, realm):
    have = {cls for cls, _ in c.classes}
    t = c.tier()
    opts = []
    if "Wizard" in have and t >= 5:
        opts.append("Archmage")
    if have & {"Warrior", "Knight", "Ranger"} and c.talent >= 4:
        opts.append("Spellblade")
    if have & {"Priest", "Knight"} and c.renown >= 200 and c.personality & {"kind", "zealous"}:
        opts.append("Saint")
    if "Necromancer" in have and t >= 6:
        opts.append("Lich")
    if c.deeds.get("dragon", 0) >= 1:
        opts.append("Dragonslayer")
    if c.deeds.get("war", 0) >= 15:
        opts.append("Warlord")
    if "Rogue" in have and c.deeds.get("delve", 0) >= 8:
        opts.append("Shadowblade")
    if "Runesmith" in have and t >= 4:
        opts.append("Rune-Lord")
    if "Druid" in have and t >= 5:
        opts.append("Archdruid")
    if "Shaman" in have and t >= 4:
        opts.append("Spirit-King")
    return [o for o in opts if o not in have]


def spend_point(c, realm):
    """One skill point: the guild's doctrine picks the branch (heroes); others follow their own bent."""
    rng = realm.rng
    options = []
    for cls, _ in c.classes:
        if cls not in CLASSES:
            continue
        for b in range(3):
            if c.branch.get((cls, b), 0) < 4:
                options.append((cls, b))
    if not options:
        for cls, _ in c.classes:
            if cls in CLASSES and cls not in c.capstones:
                c.capstones.append(cls)
                c.invalidate()
                return f"capstone {CLASSES[cls]['capstone'][0]}"
        return None
    active = c.classes[-1][0]
    act_opts = [o for o in options if o[0] == active] or options
    if c.hero:
        g = realm.guilds[c.people]
        choice = g.pick_branch(act_opts, rng)
    else:
        choice = act_opts[int(rng.integers(len(act_opts)))]
    n = c.branch.get(choice, 0) + 1
    c.branch[choice] = n
    c.invalidate()
    cls, b = choice
    if all(c.branch.get((cls, k), 0) >= 4 for k in range(3)) and cls not in c.capstones:
        c.capstones.append(cls)
        return f"capstone {CLASSES[cls]['capstone'][0]}"
    return None


def gain_xp(c, xp, realm):
    if c.level >= c.level_cap() or not c.alive:
        return
    c.xp += xp
    while c.level < c.level_cap() and c.xp >= xp_needed(c.level):
        c.xp -= xp_needed(c.level)
        before_tier = c.tier()
        c.level += 1
        c.classes[-1][1] += 1
        c.invalidate()
        if c.level % 2 == 0:
            got = spend_point(c, realm)
            if got and (c.hero or c.ruled):
                realm.say(f"{realm.who(c)} masters the {got.split(' ', 1)[1]}.", minor=True)
        if c.level in (40, 70):
            opts = prestige_options(c, realm)
            if opts:
                new = opts[int(realm.rng.integers(len(opts)))]
                c.classes.append([new, 0])
                realm.say(f"{realm.who(c)} reaches level {c.level} and becomes {an(new)} {new}.")
                if new == "Lich":
                    c.undead = True
                    realm.say(f"**{realm.who(c)} casts off the flesh.** A Lich now walks among the "
                              f"{PEOPLES[c.people]}, and does not age.")
            else:
                have = [cls for cls, _ in c.classes]
                c.classes.append([pick_class(realm.rng, c.people, c.talent, exclude=have), 0])
            c.invalidate()
        if c.level == BLOOD_LEVEL_CAP:
            realm.say(f"**{realm.who(c)} reaches the hundred-and-thirtieth level.** The dragon blood has carried "
                      f"them past the summit of mortals.")
        if getattr(c, "tracked", False) and c.level % 5 == 0:
            from .events import journal
            journal(realm, c, f"Reaches level {c.level}.", "heroes")
        if c.level == MAX_LEVEL:
            realm.say(f"**{realm.who(c)} reaches the hundredth level**, the summit no mortal is meant to stand on.")
        t = c.tier()
        if t > before_tier:
            realm.on_new_tier(c, t)


def an(word):
    return "an" if word[0] in "AEIOU" else "a"


# ---------------------------------------------------------------- the character sheet
def sheet(c, realm):
    houses, chars = realm.houses, realm.chars
    y = c.died if not c.alive else realm.year
    lines = [f"### {c.full_name()}" + (f" of {houses[c.house].name}" if c.house is not None else "")
             + f": {SINGULAR[c.people] if not any(t.startswith('half-') for t in c.genetic) else 'Half-' + SINGULAR[c.people]}"
             f"{' (undead)' if c.undead else ''}, level {c.level}", ""]
    life = f"Born in year {c.born}"
    if c.alive:
        life += f"; living, aged {c.age(realm.year)}."
    else:
        life += f"; {c.cause}, year {c.died}, aged {c.age(c.died)}."
    lines.append(life)
    if c.ruled:
        p, a, b = c.ruled
        lines.append(f"Reigned over {realm.kingdoms[p].name} from year {a} to "
                     f"{b if b is not None else 'the present'}.")
    lines.append("")
    cls_txt = ", ".join(f"{cls} {lv}" for cls, lv in c.classes if lv > 0)
    lines.append(f"- **Classes:** {cls_txt} (total {c.level})")
    if getattr(c, "gift", None):
        from .arcana import GIFTS
        lines.append(f"- **Gift:** {c.gift} ({GIFTS[c.gift]})")
    if getattr(c, "battle_arts", None):
        lines.append(f"- **Battle arts:** {', '.join(c.battle_arts)}" + (f"; and the lost art {c.lost_art}"
                                                                   if getattr(c, "lost_art", None) else ""))
    if getattr(c, "seal", -1) >= 0 and c.hero:
        from .arcana import SEALS
        lines.append(f"- **Wardens' Hall:** {SEALS[c.seal]}")
    if c.alive:
        from .markets import net_worth
        nw = net_worth(realm, c)
        rank = "upper" if nw > 30 or c.ruled or getattr(c, "lordships", None) else "middle" if nw > 3 or c.profession \
            else "lower"
        lines.append(f"- **Wealth:** {c.gold:,.0f} gold in hand; net worth {nw:,.1f} (coin, gear, land and houses); "
                     f"{rank} class" + (f"; works as a {c.profession}" if c.profession else ""))
    t, s = c.tier(), c.school()
    if t >= 0:
        lines.append(f"- **Magic:** {SCHOOLS[s]}, {TIER_WORDS[t]} tier (peak {TIER_WORDS[max(t, c.peak_tier)]}). "
                     f"Spells: {', '.join(c.known_spells())}")
    else:
        lines.append("- **Magic:** none")
    lines.append(f"- **Combat rating:** {c.cr(hp_matters=False):,.0f} (peak {max(c.peak_cr, c.cr(False)):,.0f}); "
                 f"prowess {c.prowess:.0f}, magical talent {c.talent:.1f}")
    lines.append("- **Attributes:** " + ", ".join(f"{ATTR_NAMES[a]} {c.attr(a)}" for a in ATTRS))
    if getattr(c, "attributes", None):
        from .data2 import ATTRIBUTE_NAMES
        from .skills import best_skills
        lines.append("- **Body and mind:** " + ", ".join(f"{ATTRIBUTE_NAMES[k]} {v}" for k, v in c.attributes.items()))
        lines.append("- **Best skills:** " + ", ".join(f"{s} {v:.0f}" for v, s in best_skills(realm, c)))
        if c.perks:
            lines.append("- **Perks:** " + ", ".join(f"{p} ({s})" for s, p in c.perks[-10:])
                         + (f", and {len(c.perks) - 10} more" if len(c.perks) > 10 else ""))
    life_bits = []
    if c.profession:
        life_bits.append(f"profession {c.profession}")
    if c.guilds:
        gs = [realm.guilds_soc[g].name for g in c.guilds]
        life_bits.append(f"{c.guild_rank or 'member'} of {', '.join(gs)}")
    if getattr(realm, "religions", None):
        life_bits.append(f"follows {realm.religions[c.religion].name}")
    life_bits.append(f"{c.gold:,.0f} gold")
    if c.bounty > 0 and c.profession in ("thief", "assassin", "bandit"):
        life_bits.append(f"bounty {c.bounty:,.0f}")
    lines.append("- **Life:** " + "; ".join(life_bits))
    if c.fame:
        lines.append("- **Fame:** " + ", ".join(f"{k} {v:,.0f}" for k, v in sorted(c.fame.items(), key=lambda kv: -kv[1])[:4]))
    if c.clan is not None:
        from .data2 import SHINOBI_RANKS
        from .shinobi import technique
        cl = realm.clans[c.clan]
        lines.append(f"- **Shinobi:** {SHINOBI_RANKS[c.srank]} of {cl.name}; ki {c.ki:,.0f}; natures "
                     + ", ".join(f"{n} {ROMAN[v]}" for n, v in c.nature_lv.items()) + "; arts "
                     + ", ".join(f"{a} {ROMAN[v]}" for a, v in c.arts.items() if v) + f"; signature {technique(c)[1]}"
                     + f"; {c.missions} missions")
    if getattr(c, "beast", None) or getattr(c, "merged", None) or getattr(c, "marks", None):
        bits = []
        if getattr(c, "beast", None):
            b = c.beast
            bits.append(f"rides/keeps the {b['kind']} {b['name']} since year {b['since']}")
        if getattr(c, "merged", None):
            bits.append(f"merged blood with a {c.merged}; tames no more")
        if getattr(c, "marks", None):
            bits.append("bears " + " and ".join(c.marks))
        lines.append("- **Beasts and marks:** " + "; ".join(bits))
    if c.element is not None:
        from .data2 import BEND_FORMS, BEND_TIER_NAMES
        from .bending import bend_tier
        txt = ", ".join(f"{e} {BEND_TIER_NAMES[bend_tier(v)]} ({v:.0f})" for e, v in c.bend.items())
        lines.append(f"- **Bending:** {txt}" + (f"; high art {c.high_art}" if c.high_art else "")
                     + ("; **the Concord**" if c.concord else "")
                     + (f"; of {realm.tribes[c.tribe].name}" if c.tribe else ""))
    if hasattr(c, "stress"):
        from .heart import stress_level
        bits = [f"stress {c.stress:.0f} (level {stress_level(c)})", f"happiness {c.happy:+.0f}", f"health {c.health:.1f}/10"]
        if c.coping:
            bits.append("copes as " + ", ".join(c.coping))
        loves = [(realm.chars[i], kind, s) for i, (kind, s) in c.loves.items() if i in realm.chars]
        if loves:
            bits.append("loves " + ", ".join(f"{o.full_name()} ({kind})" for o, kind, s in sorted(loves, key=lambda t: -t[2])[:4]))
        if c.pet:
            bits.append(f"a {c.pet['kind']} called {c.pet['name']}")
        if c.vendettas:
            bits.append("sworn vengeance on " + ", ".join(realm.chars[v].full_name() for v in c.vendettas if v in realm.chars))
        lines.append("- **Heart:** " + "; ".join(bits) + f"; orientation {c.orientation}")
    from .court import education_name
    lines.append(f"- **Education:** {education_name(c)}"
                 + (f"; lifestyle perks: {', '.join(p for _, _, p in c.perk_list)}" if getattr(c, "perk_list", None) else ""))
    k = realm.kingdoms[c.realm] if c.realm is not None and c.realm < len(realm.kingdoms) else None
    if c.alive and k is not None and k.ruler is not None and k.ruler != c.id and getattr(realm, "op_mods", None) is not None:
        from .opinion import opinion as _op
        tot, terms = _op(realm, c, realm.chars[k.ruler], breakdown=True)
        top = sorted(terms, key=lambda t: -abs(t[1]))[:6]
        lines.append(f"- **Opinion of their ruler:** {tot:+d} (" + ", ".join(f"{n} {v:+d}" for n, v in top) + ")")
    if getattr(realm, "cultures", None):
        from .civics import ideology_label, views
        lines.append(f"- **Views on governance:** {ideology_label(views(realm, c))}")
    tr = sorted(c.genetic) + sorted(c.personality)
    lines.append(f"- **Traits:** {', '.join(tr) if tr else 'none'}"
                 + (f"; carries latent {', '.join(sorted(c.latent))}" if c.latent else ""))
    lim = []
    if "starblood" in c.genetic:
        lim.append("tier cap broken (starblood)")
    if "dragonblood" in c.genetic:
        lim.append("level cap 130 (dragonblood)")
    if "titanblood" in c.genetic:
        lim.append("prowess cap broken (titanblood)")
    if "abyssblood" in c.genetic:
        lim.append("talent cap broken (abyssblood)")
    if "elderblood" in c.genetic:
        lim.append("lifespan tripled (elderblood)")
    lines.append(f"- **Limits:** tier cap {c.tier_cap()}, level cap {c.level_cap()}"
                 + (f"; {', '.join(lim)}" if lim else ""))
    leg = legacy_lines(realm, c)
    if leg:
        lines.append("- **Legacies:** " + "; ".join(leg))
    if c.legitimacy is not None:
        state = "corrupt" if c.legitimacy < 30 else "trusted" if c.legitimacy >= 70 else "accepted"
        lines.append(f"- **Governance:** legitimacy {c.legitimacy:.0f} ({state}), tyranny {c.tyranny:.0f}")
    if c.lordships:
        lines.append("- **Lordships held:** " + ", ".join(realm.by_id[s].name for s in c.lordships
                                                          if s in realm.by_id))
    sk = []
    for (cls, b), n in sorted(c.branch.items()):
        if n > 0:
            bname, _, nodes = CLASSES[cls]["branches"][b]
            sk.append(f"{cls} / {bname} {n}/4 ({', '.join(nodes[:n])})")
    if sk:
        lines.append("- **Skill trees:** " + "; ".join(sk))
    if c.capstones:
        lines.append("- **Capstones:** " + ", ".join(CLASSES[x]["capstone"][0] for x in c.capstones))
    if c.items:
        lines.append("- **Equipment:** " + ", ".join(it.label() for it in sorted(c.items, key=lambda i: -i.rarity)))
    if c.deeds:
        lines.append("- **Deeds:** " + ", ".join(f"{k} {v}" for k, v in sorted(c.deeds.items(), key=lambda kv: -kv[1]))
                     + f"; renown {c.renown:,.0f}")
    if c.titles:
        lines.append("- **Offices:** " + "; ".join(c.titles[:6]))
    fam = []
    if c.father is not None or c.mother is not None:
        fam.append("child of " + " and ".join(chars[p].full_name() for p in (c.father, c.mother) if p is not None))
    if c.spouse is not None:
        sp = chars[c.spouse]
        fam.append(f"married {sp.full_name()}" + (f" of {houses[sp.house].name}" if sp.house is not None else ""))
    if c.children:
        kids = [chars[k] for k in c.children]
        fam.append(f"{len(kids)} children: " + ", ".join(k.full_name() for k in kids[:8]))
    if fam:
        lines.append("- **Family:** " + "; ".join(fam))
    return "\n".join(lines) + "\n"


def tier_name(t):
    return f"{TIER_WORDS[t]}-tier" if t > 0 else "cantrip"


def roman(n):
    vals = [(1000, "M"), (900, "CM"), (500, "D"), (400, "CD"), (100, "C"), (90, "XC"), (50, "L"), (40, "XL"),
            (10, "X"), (9, "IX"), (5, "V"), (4, "IV"), (1, "I")]
    out = ""
    for v, s in vals:
        while n >= v:
            out += s
            n -= v
    return out


def sqrt_safe(x):
    return math.sqrt(max(0.0, x))
