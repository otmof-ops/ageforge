//! Beast taming (beasts.py): a rare gift. Beastspeakers tame wolves, bears, eagles, griffins, wyverns and at the
//! summit a young dragon; a master may merge blood with their beast for good; a dragon-merged line bonds hatchlings,
//! and a house of dragonriders on a throne is dragon royalty.

use crate::people::*;
use crate::power::skill_ix;
use crate::state::State;
use crate::tables::*;

fn order(kind: &str) -> usize { BEASTS.iter().position(|b| b.0 == kind).unwrap_or(0) }

impl State {
    fn gifted(c: &Character) -> bool { c.has(gene("beastspeaker")) || MERGED.iter().any(|m| c.has(gene(m.1))) }
    fn dragonbound(c: &Character) -> bool { c.has(gene("dragon-merged")) }

    fn land_of_char(&self, c: usize) -> i32 {
        let h = if self.chars[c].home != NONE { self.chars[c].home } else { self.capital_of(self.chars[c].realm) };
        if h != NONE { self.towns[h].landmass } else { -1 }
    }

    pub fn beasts_year(&mut self) {
        let y = self.year;
        let mut riders = vec![0u32; self.realms.len()];
        for c in self.living.clone() {
            if !self.chars[c].alive { continue; }
            if let Some(b) = self.chars[c].beast.clone() {
                if (y - b.3) as f64 > b.4 {
                    let t = format!("{}'s {}, {}, dies of age.", self.who(c), b.0, b.1);
                    self.say(t, b.0 != "dragon");
                    self.chars[c].beast = None;
                } else if b.0 == "dragon" { riders[self.chars[c].realm] += 1; }
            }
            if !self.chars[c].adult(y) || !Self::gifted(&self.chars[c]) { continue; }
            if self.chars[c].merged.is_none() {
                let amt = if self.chars[c].profession == Some("tamer") { 0.9 } else { 0.35 };
                self.use_skill(c, skill_ix("Beast Taming"), amt);
                self.tame(c);
                self.maybe_merge(c);
            }
            if Self::dragonbound(&self.chars[c]) && self.chars[c].beast.is_none() { self.bond_hatchling(c); }
        }
        for k in 0..self.realms.len() {
            if riders[k] > 0 { self.realms[k].battle_mult *= 1.0 + 0.15 * riders[k] as f64; }
            self.realms[k].dragons = riders[k];
        }
    }

    fn tame(&mut self, c: usize) {
        let sk = self.skill_val(&self.chars[c], skill_ix("Beast Taming"));
        let have = self.chars[c].beast.as_ref().map(|b| order(b.0));
        let mut best = None;
        for (i, b) in BEASTS.iter().enumerate() { if sk >= b.1 && have.is_none_or(|h| i > h) { best = Some(b.0); } }
        let Some(mut best) = best else { return };
        if self.rng.random() > 0.15 { return; }
        if best != "dragon" {
            let near = self.tameable_near(c);
            let opts: Vec<&str> = BEASTS.iter().map(|b| b.0).filter(|k| near.contains(k) && sk >= BEASTS[order(k)].1 && have.is_none_or(|h| order(k) > h)).collect();
            let Some(&o) = opts.last() else { return };
            best = o;
        }
        if best == "dragon" {
            let lid = self.land_of_char(c);
            let young: Vec<usize> = (0..self.monsters.len()).filter(|&m| self.monsters[m].alive && self.monsters[m].kind == "dragon" && self.monsters[m].landmass == lid && self.year - self.monsters[m].born < 120).collect();
            if young.is_empty() { return; }
            let m = young[self.rng.integers(0, young.len() as i64) as usize];
            if self.rng.random() > (sk - 80.0) / 60.0 {
                if self.rng.random() < 0.3 { let why = format!("was burned to ash trying to tame {}", self.monsters[m].name); self.kill_by_monster(c, why, m); }
                return;
            }
            self.monsters[m].alive = false;
            let name = self.monsters[m].name.split(" (").next().unwrap().to_string();
            let life = BEASTS[order("dragon")].3;
            self.chars[c].beast = Some(("dragon", name.clone(), self.monsters[m].adult_base, self.year, life));
            self.dragon_riders.push((self.year, c, name.clone(), "tamed"));
            let t = format!("**{} tames the young dragon {name}** and rides it over {}.", self.who(c), self.land_name(lid));
            self.say(t, false);
        } else {
            let (_, _, power, life) = BEASTS[order(best)];
            let name = self.name(&["Ash", "Storm", "Grey", "Night", "Ember", "Frost", "Swift", "Iron", "Moon", "Blood"], &["fang", "claw", "wing", "mane", "heart", "eye", "talon", "hide"]);
            self.chars[c].beast = Some((BEASTS[order(best)].0, name.clone(), power * (0.8 + sk / 250.0), self.year, life));
            let t = format!("{} tames a {best}, {name}.", self.who(c));
            self.say(t, !matches!(best, "griffin" | "wyvern"));
            self.gain_fame(c, "tamer", power / 20.0);
        }
    }

    fn maybe_merge(&mut self, c: usize) {
        let Some(b) = self.chars[c].beast.clone() else { return };
        if self.chars[c].skills[skill_ix("Beast Taming")] < 100.0 { return; }
        if matches!(b.0, "wolf" | "eagle") && self.rng.random() < 0.7 { return; }
        let cc = &self.chars[c];
        let royal = cc.ruled.is_some() || (cc.house != NONE && self.realms[cc.realm].house == cc.house);
        let will = 0.04 + 0.06 * cc.has_trait("ambitious") as u8 as f64 + 0.04 * cc.has_trait("brave") as u8 as f64 + 0.05 * royal as u8 as f64;
        if self.rng.random() > will { return; }
        let Some(&(_, gname)) = MERGED.iter().find(|m| m.0 == b.0) else { return };
        let g = gene(gname);
        let fx = GENETIC_FX[g];
        let ch = &mut self.chars[c];
        if !ch.has(g) { ch.genetic.push(g); }
        ch.latent.retain(|&x| x != g);
        for i in 0..6 { ch.attrs[i] += fx[i]; }
        ch.prowess += fx[6]; ch.talent = (ch.talent + fx[7]).min(10.0); ch.life *= fx[8];
        ch.merged = Some(b.0);
        ch.beast = None;
        self.merges.push((self.year, c, b.0, b.1.clone()));
        let house = if self.chars[c].house != NONE { format!(" {} will carry it.", self.houses[self.chars[c].house].name) } else { String::new() };
        let what = POWER_GENES.iter().find(|p| p.0 == gname).map(|p| p.1.replace("carries", "carry")).unwrap_or_default();
        let t = format!("**{} merges blood with {} the {}.** The beast is gone into them for good; they will never tame another. They {what}.{house}", self.who(c), b.1, b.0);
        self.say(t, false);
        self.gain_fame(c, "tamer", 300.0);
    }

    fn bond_hatchling(&mut self, c: usize) {
        if self.rng.random() > 0.08 { return; }
        let lid = self.land_of_char(c);
        let young: Vec<usize> = (0..self.monsters.len()).filter(|&m| self.monsters[m].alive && self.monsters[m].kind == "dragon" && self.year - self.monsters[m].born < 60).collect();
        let young: Vec<usize> = young.into_iter().filter(|&m| self.monsters[m].landmass == lid || self.rng.random() < 0.2).collect();
        if young.is_empty() { return; }
        let m = young[self.rng.integers(0, young.len() as i64) as usize];
        self.monsters[m].alive = false;
        let name = self.monsters[m].name.split(" (").next().unwrap().to_string();
        self.chars[c].beast = Some(("dragon", name.clone(), self.monsters[m].adult_base, self.year, BEASTS[order("dragon")].3));
        self.dragon_riders.push((self.year, c, name.clone(), "bonded"));
        let cc = &self.chars[c];
        let royal = cc.house != NONE && self.realms[cc.realm].house == cc.house;
        let t = format!("{}The hatchling {name} bonds to {}, who has dragon in the blood.", if royal { "**Dragon royalty.** " } else { "" }, self.who(c));
        self.say(t, !royal);
    }

    /// A dragon outlives its rider: to a dragon-blooded heir, or back to the wild.
    pub fn pass_beast(&mut self, c: usize, heir: usize) {
        let Some(b) = self.chars[c].beast.clone() else { return };
        if b.0 != "dragon" { return; }
        self.chars[c].beast = None;
        if heir != NONE && self.chars[heir].alive && (Self::dragonbound(&self.chars[heir]) || self.skill_val(&self.chars[heir], skill_ix("Beast Taming")) >= 90.0) && self.chars[heir].beast.is_none() {
            self.chars[heir].beast = Some(b.clone());
            let t = format!("The dragon {} accepts {} after {} dies.", b.1, self.who(heir), self.chars[c].name);
            self.say(t, false);
            self.dragon_riders.push((self.year, heir, b.1, "inherited"));
        } else {
            let home = if self.chars[c].home != NONE { self.chars[c].home } else { self.capital_of(self.chars[c].realm) };
            if home != NONE {
                let m = self.new_monster("dragon", self.towns[home].pos, b.2, Some(b.1.clone()), &[], true, false);
                self.monsters[m].adult_base = b.2;
                let t = format!("With {} dead, the dragon {} goes wild over {}.", self.chars[c].name, b.1, self.towns[home].name);
                self.say(t, false);
            }
        }
    }
}
