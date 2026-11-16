//! The common people (populace.py): the estates' moods, risings, and plague along the roads and sea lanes.

use crate::economy::thousands;
use crate::people::*;
use crate::state::State;

const DISEASE_A: [&str; 10] = ["Grey", "Red", "Black", "Sweating", "Pale", "Weeping", "Burning", "Spotted", "Bone", "Blue"];
const DISEASE_B: [&str; 9] = ["Death", "Fever", "Cough", "Pox", "Rot", "Flux", "Wasting", "Sickness", "Plague"];

fn cap(s: &str) -> String {
    let mut c = s.chars();
    match c.next() { Some(f) => f.to_uppercase().collect::<String>() + c.as_str(), None => String::new() }
}

impl State {
    pub fn moods_year(&mut self) {
        let y = self.year;
        for k in 0..self.realms.len() {
            if !self.active(k) { continue; }
            let r = self.realms[k].ruler;
            let (tax, relief, people) = (self.realms[k].tax_now, self.realms[k].tax_relief > y, self.realms[k].people);
            let protect = self.realms[k].trade_protectionist;
            let tyranny = self.chars[r].tyranny;
            let at_war = self.war[k].iter().any(|&w| w);
            for t in self.alive_towns_of(k) {
                let guild = self.cultures[self.towns[t].culture].innovations.contains("Guild Charters");
                let s = &mut self.towns[t];
                let mp = s.mood_p.unwrap_or(10.0);
                // (a town's faith and its lord join with slices 4 and 5: until then the ruler's faith is the town's)
                let occupied = s.occupied_by != NONE;
                let p = 12.0 - 120.0 * s.famine - if occupied { 20.0 } else { 0.0 } - 60.0 * s.sick_now - 60.0 * (tax - 0.25) - 6.0 * at_war as u8 as f64
                    - tyranny / 3.0 + if relief { 15.0 } else { 0.0 } + s.econ_mood
                    - if s.culture != people { 12.0 } else { 0.0 };
                let b = 10.0 + 3.0 * s.trade.min(10.0) - 40.0 * s.crime - 40.0 * (tax - 0.25) - if protect { 15.0 } else { 0.0 };
                let mp = mp + 0.3 * (p - mp);
                s.mood_p = Some(mp);
                s.mood_b += 0.3 * (b - s.mood_b);
                let share = 0.06 + 0.04 * s.port as u8 as f64 + 0.04 * guild as u8 as f64;
                s.burghers += 0.05 * (share - s.burghers);
                if s.rising.is_some_and(|ry| y - ry > 30) { s.rising = None; }
                if mp < -15.0 && s.rising.is_none() && s.pop > 200.0 && self.rng.random() < 0.05 * (-mp - 15.0) / 20.0 {
                    self.rise(k, t);
                }
            }
        }
    }

    fn rise(&mut self, k: usize, t: usize) {
        let age = 25 + self.rng.integers(0, 20);
        let leader = self.make_character(k, self.year - age, None, NONE, NONE, NONE);
        self.chars[leader].home = t;
        let brave = crate::tables::PERSONALITY.iter().position(|p| *p == "brave").unwrap();
        if !self.chars[leader].personality.contains(&brave) { self.chars[leader].personality.push(brave); }
        let s = &self.towns[t];
        let why = if s.famine > 0.05 { "bread" } else if s.sick_now > 0.05 { "the plague" }
            else if self.realms[k].tax_now > 0.3 { "the taxes" } else if self.chars[self.realms[k].ruler].tyranny >= 50.0 { "the tyrant" }
            else { "their lord" };
        let (pos, land) = (s.pos, s.landmass);
        self.towns[t].rising = Some(self.year);
        for o in self.alive_towns_of(k) {
            let os = &self.towns[o];
            if o != t && os.landmass == land && os.mood_p.unwrap_or(0.0) < -10.0
                && (os.pos.0 as f64 - pos.0 as f64).hypot(os.pos.1 as f64 - pos.1 as f64) < 18.0 {
                let add = os.pop * 0.08;
                self.towns[o].rising = Some(self.year);
                self.towns[t].rallied += add;
            }
        }
        self.risings.push((self.year, t, k, why));
        self.raise_rebels(k, t, leader, why);
        let text = format!("**A rising.** The peasants of {} rise against {} over {why}, led by {}.",
                           self.towns[t].name, self.realms[k].name, self.chars[leader].name);
        let minor = self.towns[t].pop < 1500.0;
        self.say(text, minor);
    }

    pub fn plague_year(&mut self) {
        let y = self.year;
        let towns: Vec<usize> = (0..self.towns.len()).filter(|&t| self.towns[t].alive).collect();
        if towns.is_empty() { return; }
        for &t in &towns { let s = &mut self.towns[t]; s.immune *= 0.96; s.sick_now = 0.0; }
        let great = self.rng.random() < 0.004 && y - self.last_pandemic > 120;
        if great || self.rng.random() < 0.025 {
            let src = towns[self.rng.integers(0, towns.len() as i64) as usize];
            let name = format!("the {} {}", DISEASE_A[self.rng.integers(0, 10) as usize], DISEASE_B[self.rng.integers(0, 9) as usize]);
            let fatality = if great { self.rng.uniform(0.35, 0.6) } else { self.rng.uniform(0.05, 0.2) };
            let spread = if great { self.rng.uniform(0.5, 0.8) } else { self.rng.uniform(0.2, 0.45) };
            self.epidemics.push(Epidemic { name: name.clone(), start: y, fatality, spread, great, deaths: 0.0,
                towns: Default::default(), front: vec![(src, if great { 0.25 } else { 0.1 })], end: None });
            if great {
                self.last_pandemic = y;
                let land = self.towns[src].landmass;
                let ln = if land >= 0 { self.map.lands[land as usize].name.clone() } else { "the open sea".into() };
                let text = format!("**A great pestilence.** {} breaks out in {} on {ln}.", cap(&name), self.towns[src].name);
                self.say(text, false);
            }
            self.last_plague = y;
        }
        let live: Vec<usize> = (0..self.epidemics.len()).filter(|&e| self.epidemics[e].end.is_none()).collect();
        if live.is_empty() { return; }
        // healers: priests and masters of restoration (professions and the physician join with slice 5)
        let resto = crate::stewardship::skill("Restoration");
        let mut heal = vec![0.0; self.realms.len()];
        for c in &self.chars { if c.alive && c.realm < heal.len() && c.skills[resto] >= 70.0 { heal[c.realm] += 0.04; } }
        for h in heal.iter_mut() { *h = f64::min(0.4, *h); }
        for e in live {
            let front = std::mem::take(&mut self.epidemics[e].front);
            let (fat, spread) = (self.epidemics[e].fatality, self.epidemics[e].spread);
            let mut new: Vec<(usize, f64)> = vec![];
            for (tid, load) in front {
                if !self.towns[tid].alive { continue; }
                let s = &self.towns[tid];
                let attack = (load * (1.0 - s.immune) * 3.0).min(0.9);
                let mend = (20.0 * s.arcane).min(0.3);
                let dead = s.pop * attack * fat * (1.0 - (heal[s.realm] + mend).min(0.7));
                let (pos, port, realm) = (s.pos, s.port, s.realm);
                let s = &mut self.towns[tid];
                s.pop -= dead;
                s.immune = (s.immune + attack).min(0.95);
                s.sick_now = s.sick_now.max(attack);
                self.epidemics[e].deaths += dead;
                self.epidemics[e].towns.insert(tid);
                if attack < 0.02 { continue; }
                for &o in &towns {
                    let op = self.towns[o].pos;
                    let d = (op.0 as f64 - pos.0 as f64).hypot(op.1 as f64 - pos.1 as f64);
                    let road = d < 12.0 && d > 0.0;
                    let sea = self.towns[o].port && d < 45.0 && port;
                    if !(road || sea) { continue; }
                    if self.rng.random() < spread * attack * if sea { 1.6 } else { 1.0 }
                        && (!self.epidemics[e].towns.contains(&o) || self.rng.random() < 0.1) {
                        match new.iter_mut().find(|x| x.0 == o) { Some(x) => x.1 = x.1.max(load * 0.8), None => new.push((o, load * 0.8)) }
                    }
                }
                if dead > 50.0 && self.rng.random() < 0.3 {
                    let here: Vec<usize> = self.chars.iter().filter(|c| c.alive && c.realm == realm && c.home == tid && !c.undead).map(|c| c.id).collect();
                    let name = self.epidemics[e].name.clone();
                    for c in here { if self.rng.random() < attack * fat * 0.4 { self.kill(c, format!("died of {name}")); } }
                }
            }
            let ended = new.is_empty();
            self.epidemics[e].front = new;
            if ended {
                let ep = &mut self.epidemics[e];
                ep.end = Some(y);
                if ep.deaths > 2000.0 || ep.great {
                    let b = if ep.great { "**" } else { "" };
                    let text = format!("{b}{} burns out after {} years: {} towns, some {} dead.{b}", cap(&ep.name), y - ep.start + 1,
                                       ep.towns.len(), thousands(ep.deaths));
                    let minor = !ep.great && ep.deaths < 20000.0;
                    self.say(text, minor);
                }
            }
        }
    }
}
