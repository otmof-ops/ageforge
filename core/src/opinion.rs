//! Opinion after the feudal model (opinion.py): what each person thinks of another, as a sum of named terms and
//! remembered events that fade; dread and boldness; hooks; friendships and rivalries at court.
//! (Faith's holy wars join with slice 5; how the realm is ruled, court grandeur and perks with slice 4.)

use crate::people::*;
use crate::state::State;

impl State {
    /// `who` remembers something `about` did: the stronger memory of the same name wins.
    pub fn remember(&mut self, who: usize, about: usize, name: &'static str, value: f64, decay: f64) {
        if who == NONE || about == NONE || who == about { return; }
        let y = self.year;
        let mem = &mut self.chars[who].memories;
        if let Some(m) = mem.iter_mut().find(|m| m.0 == about && m.1 == name) {
            if value.abs() > m.2.abs() { m.2 = value; }
            return;
        }
        mem.push((about, name, value, decay, y));
    }

    /// Everyone close to the victim remembers what the culprit did.
    pub fn remember_kin(&mut self, victim: usize, culprit: usize, name: &'static str, value: f64) {
        if culprit == NONE { return; }
        let v = &self.chars[victim];
        let mut kin: Vec<usize> = v.children.clone();
        for p in [v.father, v.mother, v.spouse] { if p != NONE { kin.push(p); } }
        for i in kin { if self.chars[i].alive { self.remember(i, culprit, name, value, 1.0); } }
    }

    pub fn fade(&mut self) {
        let alive: Vec<bool> = self.chars.iter().map(|c| c.alive).collect();
        for c in self.chars.iter_mut().filter(|c| c.alive && !c.memories.is_empty()) {
            for m in c.memories.iter_mut() {
                if m.2 > 0.0 { m.2 = (m.2 - m.3).max(0.0) } else { m.2 = (m.2 + m.3).min(0.0) }
            }
            c.memories.retain(|m| m.2.abs() >= 1.0 && alive[m.0]);
        }
    }

    pub fn opinion(&self, a: usize, b: usize) -> f64 {
        let (ca, cb) = (&self.chars[a], &self.chars[b]);
        let y = self.year;
        let mut t = 0.0;
        for &p in &ca.personality {
            let opp = p ^ 1;   // traits come in pairs: 2k and 2k+1
            if cb.personality.contains(&p) { t += 10.0 } else if cb.personality.contains(&opp) { t -= 10.0 }
        }
        t += ((cb.attr(0, y) - 6.0) * 1.5).round();
        if cb.has(gene("comely")) { t += 5.0; }
        t += if ca.people == cb.people { 5.0 } else { -10.0 };
        t += if ca.religion == cb.religion { 10.0 } else { -15.0 };
        let kin = cb.id == ca.father || cb.id == ca.mother || ca.id == cb.father || ca.id == cb.mother
            || (ca.father != NONE && ca.father == cb.father) || (ca.mother != NONE && ca.mother == cb.mother);
        if kin { t += 20.0; }
        if ca.spouse == b { t += 25.0; }
        if ca.house != NONE && ca.house == cb.house { t += 10.0; }
        if let Some(&(_, friend)) = ca.relations.iter().find(|r| r.0 == b) { t += if friend { 60.0 } else { -60.0 }; }
        if ca.rival_ruler == b { t -= 40.0; }
        if ca.realm < self.realms.len() && self.realms[ca.realm].ruler == b && a != b {
            let leg = cb.legitimacy.unwrap_or(50.0);
            t += ((leg - 50.0) / 2.5).round();
            if cb.tyranny > 5.0 { t -= (cb.tyranny / 2.0).round(); }
            if let Some((_, start, _)) = cb.ruled { let reign = y - start; if reign < 10 { t -= ((10 - reign) * 2) as f64; } }
            if self.realms[ca.realm].council.contains(&a) { t += 10.0; }
        }
        for m in &ca.memories { if m.0 == b { t += m.2.round(); } }
        t.trunc()
    }

    pub fn dread(&self, r: usize) -> f64 {
        let c = &self.chars[r];
        c.tyranny * 0.6 + if c.has_trait("cruel") { 15.0 } else { 0.0 } + c.attr(3, self.year)
    }

    pub fn boldness(&self, c: usize) -> f64 {
        let c = &self.chars[c];
        20.0 + if c.has_trait("brave") { 25.0 } else { 0.0 } - if c.has_trait("craven") { 20.0 } else { 0.0 }
            + if c.has_trait("ambitious") { 10.0 } else { 0.0 } + c.attr(1, self.year)
    }

    pub fn gain_hook(&mut self, r: usize, c: usize, strength: &'static str, why: &str) {
        let hooks = &mut self.chars[r].hooks;
        match hooks.iter_mut().find(|h| h.0 == c) { Some(h) => h.1 = strength, None => hooks.push((c, strength)) }
        let text = format!("{} gains a {strength} hook on {}{why}.", self.who(r), self.who(c));
        self.say(text, true);
    }

    /// Friends and rivals at court, and leverage over vassals (opinion.opinion_year). Vassals arrive with titles.
    pub fn opinion_year(&mut self) {
        self.fade();
        for k in 0..self.realms.len() {
            if !self.active(k) { continue; }
            let r = self.realms[k].ruler;
            let mut court: Vec<usize> = vec![r];
            for &c in &self.realms[k].council { if c != NONE && self.chars[c].alive && !court.contains(&c) { court.push(c); } }
            if court.len() >= 2 && self.rng.random() < 0.3 {
                let i = self.rng.integers(0, court.len() as i64) as usize;
                let mut j = self.rng.integers(0, court.len() as i64 - 1) as usize;
                if j >= i { j += 1; }
                let (a, b) = (court[i], court[j]);
                let o = self.opinion(a, b) + self.opinion(b, a);
                let known = self.chars[a].relations.iter().any(|x| x.0 == b);
                if !known && (o > 60.0 || o < -40.0) {
                    let friend = o > 60.0;
                    self.chars[a].relations.push((b, friend));
                    self.chars[b].relations.push((a, friend));
                    let text = format!("{} and {} become {}.", self.who(a), self.who(b), if friend { "friends" } else { "rivals" });
                    self.say(text, true);
                }
            }
        }
    }
}
