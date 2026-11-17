//! Rule and its costs (governance.py, and the realm's multipliers from dynasty.council): the ruler's legitimacy
//! and tyranny, the council's corruption, purges, risings and overthrows, lords of towns, and legacy from deeds.

use crate::people::*;
use crate::state::State;
use crate::tables::*;

const SEAT_NAMES: [&str; 6] = ["marshal", "steward", "spymaster", "mage", "loremaster", "admiral"];

fn clamp(v: f64) -> f64 { v.clamp(0.0, 100.0) }

impl State {
    /// How far deeds are told (lineage.storytelling).
    fn storytelling(&self, c: usize) -> f64 {
        let cc = &self.chars[c];
        let y = self.year;
        let keeper = if cc.realm < self.realms.len() { self.realms[cc.realm].council[4] } else { NONE };
        let lore = cc.attr(5, y) + if keeper != NONE && self.chars[keeper].alive { self.chars[keeper].attr(5, y) } else { 5.0 };
        (lore / 20.0 * STORY[cc.people]).max(0.2)
    }

    /// Deeds become legacy only as far as they are told (lineage.gain_legacy).
    pub fn gain_legacy(&mut self, c: usize, kind: &str, renown: f64) {
        if renown <= 0.0 { return; }
        let kix = LEGACY_KINDS.iter().position(|x| *x == kind).unwrap();
        let pts = renown * self.storytelling(c);
        let lv = |p: f64| crate::power::level_of(p, &LEGACY_LEVELS);
        let cur = self.chars[c].legacy.iter().find(|x| x.0 == kix).map(|x| x.1).unwrap_or(0.0);
        let (before, after) = (lv(cur), lv(cur + pts));
        match self.chars[c].legacy.iter_mut().find(|x| x.0 == kix) { Some(x) => x.1 += pts, None => self.chars[c].legacy.push((kix, pts)) }
        if after > before && after >= 2 {
            let name = LEGACY_NAMES[kix];
            let text = format!("The songs of {} spread; theirs is now a legacy of {name} {}.", self.who(c), ROMAN[after as usize]);
            self.say(text, after < 5);
        }
        let h = self.chars[c].house;
        if h != NONE {
            let share = if self.houses[h].track == kind { 0.5 } else { 0.2 };
            self.houses[h].legacy += pts * share;
        }
    }

    /// Renown, legacy and prestige for a ruler's great deed (statecraft.honour).
    pub fn honour(&mut self, r: usize, renown: f64, why: &'static str) {
        self.chars[r].renown += renown;
        self.gain_legacy(r, "ruler", renown * 0.5);
        let h = self.chars[r].house;
        if h != NONE { self.houses[h].prestige += renown * 0.5; }
        let d = &mut self.chars[r].deeds;
        match d.iter_mut().find(|x| x.0 == why) { Some(x) => x.1 += 1.0, None => d.push((why, 1.0)) }
    }

    pub fn add_tyranny(&mut self, c: usize, amount: f64, why: &str) {
        if c == NONE { return; }
        let before = self.chars[c].tyranny;
        self.chars[c].tyranny = clamp(before + amount);
        if before < 60.0 && self.chars[c].tyranny >= 60.0 {
            let w = if why.is_empty() { String::new() } else { format!(", after {why}") };
            let text = format!("{} is called a tyrant now{w}. Men lower their eyes when the crown rides by.", self.who(c));
            self.say(text, false);
        }
    }

    fn official_target(&self, c: usize, skill: f64, ruler: usize) -> f64 {
        let cc = &self.chars[c];
        let p = |t: &str| cc.has_trait(t) as u8 as f64;
        let mut t = 45.0 + skill * 1.2 + 15.0 * p("honest") - 15.0 * p("greedy") - 10.0 * p("deceitful") + 5.0 * p("diligent") - 5.0 * p("lazy");
        if ruler != NONE { if let Some(l) = self.chars[ruler].legitimacy { t += (l - 50.0) / 5.0 - self.chars[ruler].tyranny / 10.0; } }
        t
    }

    /// The realm's multipliers from its marshal, steward and mage, and its ruler's temper.
    pub fn council_effects(&mut self) {
        let y = self.year;
        for k in 0..self.realms.len() {
            if !self.active(k) { continue; }
            let r = self.realms[k].ruler;
            let [m, s, _, mg, _, _] = self.realms[k].council;
            let rc = &self.chars[r];
            let battle = 1.0 + ((if m != NONE { self.chars[m].attr(1, y) } else { 0.0 }) + rc.attr(1, y) * 0.5) / 60.0;
            let growth = 1.0 + ((if s != NONE { self.chars[s].attr(2, y) } else { 0.0 }) + rc.attr(2, y) * 0.5) / 80.0;
            let tier = if mg != NONE { self.tier(&self.chars[mg]).max(0) } else { 0 };
            let p = |t: &str| rc.has_trait(t) as u8 as f64;
            let aggr = (1.0 + 0.5 * p("ambitious") + 0.5 * p("wrathful") + 0.3 * p("cruel") - 0.4 * p("content") - 0.3 * p("calm") - 0.3 * p("kind")).max(0.2);
            let realm = &mut self.realms[k];
            realm.battle_mult = battle;
            realm.growth_mult = growth;
            realm.realm_tier = tier;
            realm.aggr = aggr;
        }
    }

    pub fn govern_year(&mut self) {
        let y = self.year;
        for k in 0..self.realms.len() {
            if !self.active(k) { continue; }
            let r = self.realms[k].ruler;
            if self.chars[r].legitimacy.is_none() { self.chars[r].legitimacy = Some(50.0); }
            let rc = &self.chars[r];
            let reign = y - rc.ruled.map(|x| x.1).unwrap_or(y);
            let ruler_kind = LEGACY_KINDS.iter().position(|x| *x == "ruler").unwrap();
            let lvl = self.legacy_level(rc, ruler_kind) as f64;
            let p = |t: &str| rc.has_trait(t) as u8 as f64;
            let mut target = 50.0 + (rc.attr(0, y) + rc.attr(2, y) - 10.0) * 1.5 + lvl * 5.0 + (rc.renown / 80.0).min(15.0)
                + (reign as f64 / 5.0).min(15.0) - rc.tyranny / 2.0
                + (2.0 * (self.realms[k].taken as f64 - 2.0 * self.realms[k].lost as f64)).clamp(-20.0, 15.0)
                + 5.0 * p("honest") - 5.0 * p("deceitful") - 5.0 * p("greedy");
            if rc.house != NONE && self.houses[rc.house].track == "ruler" { target += 3.0 * lvl; }
            target += self.ruler_faith_legitimacy(k, r);
            let leg = rc.legitimacy.unwrap();
            let tyr = rc.tyranny * 0.985 + 0.5 * p("cruel") + 0.3 * p("paranoid") + 0.2 * p("wrathful") - 0.4 * p("kind") - 0.2 * p("calm");
            let (honest_or_diligent, cruel_paranoid) = (rc.has_trait("honest") || rc.has_trait("diligent"), rc.has_trait("cruel") && rc.has_trait("paranoid"));
            let undead = rc.undead;
            let rc = &mut self.chars[r];
            rc.legitimacy = Some(clamp(leg + 0.1 * (target - leg)));
            rc.tyranny = clamp(tyr);
            if !undead { let l = self.chars[r].legitimacy.unwrap(); self.gain_legacy(r, "ruler", 2.0 + l / 25.0); }
            // the council: low legitimacy is corruption
            let mut corrupt = vec![];
            for seat in 0..6 {
                let c = self.realms[k].council[seat];
                if c == NONE || !self.chars[c].alive { continue; }
                let cc = &self.chars[c];
                let skill = match seat { 0 => cc.attr(1, y), 1 => cc.attr(2, y), 2 => cc.attr(3, y), 3 => self.tier(cc).max(0) as f64 * 2.0,
                                         4 => cc.attr(5, y), _ => cc.skills[crate::power::skill_ix("Seamanship")] / 6.0 };
                let tgt = self.official_target(c, skill, r);
                if self.chars[c].legitimacy.is_none() { let n = self.rng.normal(0.0, 8.0); self.chars[c].legitimacy = Some(clamp(tgt + n)); }
                let was = self.chars[c].legitimacy.unwrap() < 30.0;
                let n = self.rng.normal(0.0, 2.0);
                let l = self.chars[c].legitimacy.unwrap();
                self.chars[c].legitimacy = Some(clamp(l + 0.1 * (tgt - l) + n));
                if self.chars[c].legitimacy.unwrap() < 30.0 {
                    corrupt.push(seat);
                    if !was {
                        let text = format!("{}, {} of {}, has grown corrupt; coin goes missing from the treasury.", self.who(c), SEAT_NAMES[seat], self.realms[k].name);
                        self.say(text, true);
                    }
                    if self.chars[r].legitimacy.unwrap() >= 60.0 && honest_or_diligent && self.rng.random() < 0.3 {
                        self.chars[c].disgraced_until = y + 25;
                        self.realms[k].council[seat] = NONE;
                        let text = format!("{} strips {} of the office of {} for corruption.", self.who(r), self.chars[c].name, SEAT_NAMES[seat]);
                        self.say(text, false);
                    }
                }
            }
            let rl = self.chars[r].legitimacy.unwrap();
            let corruption = ((50.0 - rl) / 50.0).max(0.0) + 0.1 * corrupt.len() as f64;
            let realm = &mut self.realms[k];
            realm.corruption = corruption.min(1.0);
            realm.growth_mult *= 1.0 - 0.15 * realm.corruption;
            if corrupt.contains(&0) { realm.battle_mult *= 0.9; }
            if corrupt.contains(&3) { realm.realm_tier = (realm.realm_tier - 1).max(0); }
            realm.corrupt_seats = corrupt;
            let cor = realm.corruption;
            for t in self.alive_towns_of(k) { self.towns[t].wealth *= 1.0 - 0.03 * cor; }
            if cruel_paranoid && self.rng.random() < 0.04 { self.purge(k, r); }
            if !self.chars[r].alive { continue; }
            if self.rng.random() < 0.004 * self.chars[r].tyranny / 10.0 { self.rising_against(k, r); }
            let rl = self.chars[r].legitimacy.unwrap();
            if rl < 25.0 && self.rng.random() < 0.03 * (30.0 - rl) / 30.0 { self.overthrow(k, r); }
        }
        self.lords_year();
    }

    fn purge(&mut self, k: usize, r: usize) {
        let rh = self.chars[r].house;
        let victims: Vec<usize> = self.adults_of(k, NONE).into_iter().filter(|&c| c != r && self.chars[c].house != NONE && !self.chars[c].undead && self.chars[c].house != rh).collect();
        if victims.is_empty() { return; }
        let v = victims[self.rng.integers(0, victims.len() as i64) as usize];
        let text = format!("{} purges the court: {} is dragged out and put to death.", self.who(r), self.who(v));
        self.say(text, false);
        self.remember_kin(v, r, "purged my kin", -60.0);
        self.reluct(r, "execute", "the purges");
        let why = format!("was put to death in the purges of {} {}", self.chars[r].name, self.chars[r].regnal.clone().unwrap_or_default());
        self.kill_by(v, why, true, r);
        self.add_tyranny(r, 8.0, "the purges");
        self.realms[k].fear_until = self.year + 10;
    }

    fn rising_against(&mut self, k: usize, r: usize) {
        let towns = self.alive_towns_of(k);
        if towns.is_empty() { return; }
        let t = towns[self.rng.integers(0, towns.len() as i64) as usize];
        let s = &mut self.towns[t];
        s.pop *= 0.8;
        s.walls = s.walls.saturating_sub(1);
        s.wealth *= 0.6;
        let l = self.chars[r].legitimacy.unwrap_or(50.0);
        self.chars[r].legitimacy = Some(clamp(l - 4.0));
        let text = format!("The common folk of {} rise against {}; the rising is put down in blood.", self.towns[t].name, self.who(r));
        let minor = self.chars[r].tyranny < 50.0;
        self.say(text, minor);
        self.add_tyranny(r, 3.0, "the risings");
    }

    fn overthrow(&mut self, k: usize, r: usize) {
        let cands: Vec<usize> = self.pool_of(k).into_iter().filter(|&c| c != r).collect();
        if cands.is_empty() { return; }
        let mut best = (f64::MIN, NONE);
        for c in cands { let s = self.claim_of(c) + self.chars[c].legitimacy.unwrap_or(50.0) / 10.0; if s > best.0 { best = (s, c); } }
        let rival = best.1;
        let why = if self.chars[r].tyranny >= 50.0 { "tyranny" } else { "corruption" };
        let text = format!("**The realm rises.** {} will no longer suffer the {why} of {}; the great lords raise the banner of {}.", self.realms[k].name, self.who(r), self.who(rival));
        self.say(text, false);
        let win = self.civil_war_of(k, r, rival);
        if win == rival {
            if self.chars[r].alive { self.kill_by(r, format!("was overthrown for {why}"), true, NONE); }
            self.end_reign(k, &format!("overthrown for {why}"));
            self.crown(k, rival, &format!("overthrowing a ruler hated for {why}"));
        } else {
            self.add_tyranny(r, 10.0, "crushing the rising");
            let l = self.chars[r].legitimacy.unwrap_or(50.0);
            self.chars[r].legitimacy = Some(clamp(l + 10.0));
        }
    }

    pub fn grant_lordship(&mut self, c: usize, s: usize, why: &str) {
        if self.towns[s].lord != NONE { return; }
        self.towns[s].lord = c;
        self.chars[c].lordships.push(s);
        let title = format!("{} of {}", if self.chars[c].female { "Lady" } else { "Lord" }, self.towns[s].name);
        self.chars[c].titles.push(format!("{title} from year {}", self.year));
        if self.chars[c].legitimacy.is_none() {
            let cc = &self.chars[c];
            self.chars[c].legitimacy = Some(clamp(55.0 + 10.0 * cc.has_trait("honest") as u8 as f64 - 10.0 * cc.has_trait("greedy") as u8 as f64));
        }
        if !why.is_empty() { let text = format!("{} is made {title}{why}.", self.who(c)); self.say(text, true); }
    }

    pub fn lose_lordship(&mut self, s: usize) {
        let c = self.towns[s].lord;
        if c == NONE { return; }
        self.chars[c].lordships.retain(|&x| x != s);
        self.towns[s].lord = NONE;
    }

    fn lords_year(&mut self) {
        let y = self.year;
        for s in 0..self.towns.len() {
            if !self.towns[s].alive { continue; }
            let c = self.towns[s].lord;
            if c != NONE && (!self.chars[c].alive || self.chars[c].realm != self.towns[s].realm) { self.lose_lordship(s); }
            let c = self.towns[s].lord;
            if c == NONE { self.towns[s].lord_mod = 1.0; continue; }
            let r = self.realms[self.towns[s].realm].ruler;
            let tgt = self.official_target(c, self.chars[c].attr(2, y), r);
            let n = self.rng.normal(0.0, 2.0);
            let l = self.chars[c].legitimacy.unwrap_or(tgt);
            self.chars[c].legitimacy = Some(clamp(l + 0.1 * (tgt - l) + n));
            if self.chars[c].legitimacy.unwrap() < 30.0 { self.towns[s].wealth *= 0.95; self.towns[s].lord_mod = 0.9; }
            else { self.towns[s].lord_mod = 1.0 + (self.chars[c].attr(2, y) - 5.0) / 60.0; }
        }
        for k in 0..self.realms.len() {
            if !self.active(k) { continue; }
            let vacant: Vec<usize> = self.alive_towns_of(k).into_iter().filter(|&s| self.towns[s].lord == NONE && !self.towns[s].capital).collect();
            if vacant.is_empty() || self.rng.random() > 0.3 { continue; }
            // (renowned heroes join the deserving with slice 6)
            let deserving: Vec<usize> = self.realms[k].council.iter().copied().filter(|&c| c != NONE && self.chars[c].alive
                && self.chars[c].lordships.len() < 3 && self.chars[c].disgraced_until <= y).collect();
            if deserving.is_empty() { continue; }
            let mut best = (f64::MIN, NONE);
            for c in deserving { let sc = self.chars[c].renown + self.rng.random() * 50.0; if sc > best.0 { best = (sc, c); } }
            let s = *vacant.iter().max_by(|&&a, &&b| self.towns[a].pop.partial_cmp(&self.towns[b].pop).unwrap()).unwrap();
            self.grant_lordship(best.1, s, " by grant of the crown");
        }
    }
}
