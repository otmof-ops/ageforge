//! What a person can do (characters.py, skills.py and lineage.py): legacies and their bonuses, the effects of
//! classes, branches and capstones, magic tiers within the world's measure of a mortal, and the combat rating.
//! (Items join with crafts, slice 5; bending, beasts and shinobi power with slice 6.)

use crate::people::*;
use crate::state::State;
use crate::tables::*;

// indexes into FX_KEYS
pub const PHYS: usize = 0;
pub const MAGIC_K: usize = 1;
pub const HP: usize = 2;
pub const EQUIP: usize = 7;
pub const TIER: usize = 8;
const ADD_KEYS: [usize; 3] = [8, 9, 10];

pub fn level_of(points: f64, table: &[f64]) -> u32 {
    table.iter().filter(|&&t| t <= points).count() as u32
}

impl State {
    pub fn legacy_level(&self, c: &Character, kind: usize) -> u32 {
        let own = level_of(c.legacy.iter().find(|x| x.0 == kind).map(|x| x.1).unwrap_or(0.0), &LEGACY_LEVELS);
        if c.house != NONE {
            let h = &self.houses[c.house];
            if h.track == LEGACY_KINDS[kind] { return own.max(level_of(h.legacy, &HOUSE_LEGACY_LEVELS)); }
        }
        own
    }

    pub fn legacy_bonus(&self, c: &Character, key: &str) -> f64 {
        let mut total = 0.0;
        for (kind, fx) in LEGACY_FX.iter().enumerate() {
            if let Some(&(_, v)) = fx.iter().find(|x| x.0 == key) {
                let lv = self.legacy_level(c, kind);
                if lv > 0 { total += v * lv as f64; }
            }
        }
        total
    }

    /// The multipliers and additions a person's training gives (Character.mods).
    pub fn mods(&self, c: &Character) -> [f64; 11] {
        let mut m = [1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 0.0, 0.0, 0.0];
        let apply = |m: &mut [f64; 11], k: usize, v: f64| if ADD_KEYS.contains(&k) { m[k] += v } else { m[k] *= v };
        for &((cls, b), n) in &c.branch {
            if n == 0 { continue; }
            let key = CLASSES[cls].branches[b].1;
            let n = n as f64;
            match FX_KEYS[key] {
                "tier" => { apply(&mut m, MAGIC_K, 1.0 + 0.05 * n); if n >= 4.0 { apply(&mut m, TIER, 1.0); } }
                "lead" => apply(&mut m, 9, 2.0 * n),
                "intrigue" => apply(&mut m, 10, 3.0 * n),
                _ => apply(&mut m, key, (1.0 + 0.08 * n) * if n >= 4.0 { 1.1 } else { 1.0 }),
            }
        }
        for &cls in &c.capstones { for &(k, v) in CLASSES[cls].fx { apply(&mut m, k, v); } }
        for &(cls, _) in &c.classes { if CLASSES[cls].prestige { for &(k, v) in CLASSES[cls].fx { apply(&mut m, k, v); } } }
        for (key, idx) in [("vs_monster", 5), ("delve", 6), ("guard", 4)] {
            let b = self.legacy_bonus(c, key);
            if b != 0.0 { apply(&mut m, idx, 1.0 + b); }
        }
        if c.has(gene("titanblood")) { apply(&mut m, PHYS, 1.3); }
        if c.has(gene("abyssblood")) { apply(&mut m, MAGIC_K, 1.3); }
        m
    }

    pub fn school(&self, c: &Character) -> Option<usize> {
        let (mut best, mut bl) = (None, 0);
        for &(cls, lv) in &c.classes {
            if !CLASSES[cls].prestige && CLASSES[cls].school >= 0 && lv > bl { best = Some(CLASSES[cls].school as usize); bl = lv; }
        }
        best
    }

    pub fn eff_talent(&self, c: &Character) -> f64 {
        (c.talent + self.legacy_bonus(c, "talent")).min(10.0)
    }

    /// The highest tier a mortal can reach by the world's own measure (characters._power_gate).
    pub fn power_gate(&self, c: &Character, martial: bool) -> i64 {
        if c.custom { return 10; }
        let lv = c.level;
        let (skill, gift) = if martial {
            (c.prowess / PROWESS_CAP[c.people].max(1.0),
             c.gift.is_some_and(|g| GIFTS[g] == "Iron Skin" || GIFTS[g] == "Born Captain") || c.has(gene("titanblood")) || c.has(gene("dragonblood")))
        } else {
            (self.eff_talent(c) / 10.0,
             c.gift.is_some_and(|g| GIFTS[g] == "Mana Well") || c.has(gene("starblood")) || c.has(gene("abyssblood")))
        };
        let mut g = 3;
        if skill >= 0.6 && lv >= 22 {
            g = 4;
            if skill >= 0.85 && lv >= 40 && gift {
                g = 5;
                if skill >= 0.95 && lv >= 55 { g = 6; }
            }
        }
        if c.concord { g = g.max(5); }
        g
    }

    /// The highest tier of magic this person can cast: training capped by talent, plus gifts, within the gate.
    pub fn tier(&self, c: &Character) -> i64 {
        if self.school(c).is_none() { return -1; }
        let eff: f64 = c.classes.iter().map(|&(k, lv)| lv as f64 * CLASSES[k].magic.min(1.0)).sum();
        let base = if c.has(gene("abyssblood")) { (eff / 9.0) as i64 } else { ((eff / 9.0) as i64).min(self.eff_talent(c) as i64) };
        let well = c.gift.is_some_and(|g| GIFTS[g] == "Mana Well") as i64;
        let cap = if c.custom { 10 } else { TIER_CAP[c.people] as i64 + c.has(gene("starblood")) as i64 } + well;
        let limit = self.power_gate(c, false);
        (base + self.mods(c)[TIER] as i64 + well).min(cap).min(limit).max(0)
    }

    pub fn skill_val(&self, c: &Character, s: usize) -> f64 {
        (c.skills[s] + self.legacy_bonus(c, &format!("skill:{}", SKILLS[s]))).min(120.0)
    }

    fn perk_count(c: &Character, s: usize) -> f64 { c.perks.iter().filter(|p| p.0 == s).count() as f64 }

    pub fn eff_prowess(&self, c: &Character) -> f64 {
        let frac = c.age(self.year) as f64 / c.life.max(1.0);
        let age_pen = if c.undead { 0.0 } else { ((frac - 0.66) * 25.0).max(0.0).floor() };
        let v = c.prowess + self.legacy_bonus(c, "prowess") - age_pen;   // (court perks join with slice 4)
        let v = if c.has(gene("titanblood")) { v } else { v.min(PROWESS_CAP[c.people]) };
        v * (1.0 + 0.04 * c.battle_arts as f64 + if c.lost_art { 0.15 } else { 0.0 })
    }

    pub fn hp_mult(&self, c: &Character) -> f64 {
        let lv = c.classes.iter().map(|x| x.1).sum::<u32>().max(1) as f64;
        let v = c.classes.iter().map(|&(k, l)| CLASSES[k].hp * l as f64).sum::<f64>() / lv * self.mods(c)[HP];
        let armor = self.skill_val(c, skill_ix("Heavy Armor")).max(self.skill_val(c, skill_ix("Light Armor")));
        v * (1.0 + armor / 250.0 + self.skill_val(c, skill_ix("Block")) / 400.0) * (1.0 + (c.attributes[5] - 40.0) / 300.0)
    }

    /// Combat rating: training and prowess, the best spell they can cast (characters._cr_base); wounds scale it.
    pub fn cr(&self, c: &Character, hp_matters: bool) -> f64 {
        let m = self.mods(c);
        let phys_lv: f64 = c.classes.iter().map(|&(k, lv)| lv as f64 * CLASSES[k].phys).sum();
        let w = WEAPON_SKILL[c.classes[0].0];
        let a = &c.attributes;
        let pm = (1.0 + self.skill_val(c, w) / 100.0 * 0.6 + 0.05 * Self::perk_count(c, w)) * (1.0 + (a[0] + a[3] - 80.0) / 300.0);
        let phys = 2.0 * (phys_lv + self.eff_prowess(c).max(0.0)).powf(1.2) * m[PHYS] * pm;
        let t = self.tier(c);
        let mut mag = 0.0;
        if t >= 0 {
            let mw = c.classes.iter().map(|&(k, _)| CLASSES[k].magic).fold(0.0, f64::max);
            let sk = SCHOOL_SKILL[self.school(c).unwrap()];
            let mm = (1.0 + self.skill_val(c, sk) / 100.0 * 0.6 + 0.05 * Self::perk_count(c, sk)) * (1.0 + (a[1] + a[2] - 80.0) / 300.0);
            mag = 12.0 * 1.9f64.powi(t as i32) * mw * m[MAGIC_K] * mm;
        }
        let v = (phys + mag) * (1.0 + 0.0 * m[EQUIP]);
        if hp_matters { v * (0.5 + 0.5 * c.hp / 100.0) } else { v }
    }

    /// Practice (skills.use): the closer to mastery, the slower the climb; perks every twenty points.
    pub fn use_skill(&mut self, cid: usize, s: usize, amount: f64) {
        let c = &self.chars[cid];
        if !c.alive { return; }
        if SKILLS[s] == "Beast Taming" && !c.has(gene("beastspeaker")) { return; }
        let gov = SKILL_GOV[s];
        let before = c.skills[s];
        let rate = amount * 2.2 * (1.0 - before / 110.0) * (0.7 + c.attributes[gov] / 100.0);
        let after = (before + rate).min(100.0);
        let c = &mut self.chars[cid];
        c.skills[s] = after;
        c.skill_progress[gov] += after - before;
        while c.skill_progress[gov] >= 10.0 {
            c.skill_progress[gov] -= 10.0;
            c.attributes[gov] = (c.attributes[gov] + 1.0).min(100.0);
        }
        let mut mastered = None;
        for (i, &at) in PERK_AT.iter().enumerate() {
            if before < at && at <= after {
                c.perks.push((s, i));
                if at == 100.0 { mastered = Some(i); }
            }
        }
        if let Some(i) = mastered {
            let text = format!("{} masters {}, and earns the perk {}.", self.who(cid), SKILLS[s], PERKS[s][i]);
            self.say(text, true);
        }
    }

    pub fn prestige_options(&self, c: &Character) -> Vec<usize> {
        let have: Vec<&str> = c.classes.iter().map(|x| CLASSES[x.0].name).collect();
        let h = |n: &str| have.contains(&n);
        let t = self.tier(c);
        let mut opts = vec![];
        if h("Wizard") && t >= 5 { opts.push("Archmage"); }
        if (h("Warrior") || h("Knight") || h("Ranger")) && c.talent >= 4.0 { opts.push("Spellblade"); }
        if (h("Priest") || h("Knight")) && c.renown >= 200.0 && (c.has_trait("kind") || c.has_trait("zealous")) { opts.push("Saint"); }
        if h("Necromancer") && t >= 6 { opts.push("Lich"); }
        if c.deed("dragon") >= 1.0 { opts.push("Dragonslayer"); }
        if c.deed("war") >= 15.0 { opts.push("Warlord"); }
        if h("Rogue") && c.deed("delve") >= 8.0 { opts.push("Shadowblade"); }
        if h("Runesmith") && t >= 4 { opts.push("Rune-Lord"); }
        if h("Druid") && t >= 5 { opts.push("Archdruid"); }
        if h("Shaman") && t >= 4 { opts.push("Spirit-King"); }
        opts.into_iter().filter(|o| !h(o)).map(|o| CLASSES.iter().position(|k| k.name == o).unwrap()).collect()
    }
}

pub fn skill_ix(name: &str) -> usize {
    SKILLS.iter().position(|s| *s == name).expect("unknown skill")
}
