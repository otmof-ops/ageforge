//! The heart (heart.py): love, grief and vengeance, and the three meters every person carries (stress,
//! happiness, health), with coping habits, madness, pets and the stress of acting against one's nature.

use crate::people::*;
use crate::state::State;

const COPING: [(&str, &[&str]); 10] = [
    ("drunkard", &["gregarious", "impatient", "arbitrary"]), ("reclusive", &["shy", "patient"]),
    ("flagellant", &["zealous", "just", "humble"]), ("irritable", &["wrathful", "impatient", "arrogant"]),
    ("comfort eater", &["gluttonous", "content"]), ("profligate", &["greedy", "lustful"]),
    ("journaller", &["diligent", "patient"]), ("confider", &["trusting", "gregarious"]),
    ("hunter", &["brave", "callous"]), ("inappetetic", &["temperate", "chaste"]),
];
const PETS: [(&str, i64); 6] = [("dog", 14), ("cat", 16), ("hawk", 12), ("horse", 25), ("wolfhound", 12), ("raven", 18)];
const PET_NAMES: [&str; 16] = ["Ash", "Biscuit", "Thorn", "Mira", "Rook", "Pip", "Bram", "Sable", "Tansy", "Brindle", "Moth",
                               "Cinder", "Wren", "Bracken", "Juniper", "Hob"];

fn stress_level(c: &Character) -> i64 { ((c.stress / 100.0) as i64).min(3) }

/// Stress a person takes from an act against their nature (negative is relief).
pub fn reluctance(c: &Character, act: &str) -> f64 {
    let p = |t: &str| c.has_trait(t) as u8 as f64;
    match act {
        "execute" => 40.0 * p("compassionate") + 30.0 * p("kind") + 20.0 * p("forgiving") + 15.0 * p("just") - 15.0 * p("cruel") - 10.0 * p("callous"),
        "war" => 30.0 * p("craven") + 20.0 * p("compassionate") + 10.0 * p("content") - 10.0 * p("wrathful") - 10.0 * p("vengeful") - 5.0 * p("brave"),
        "feast" => 15.0 * p("shy") - 10.0 * p("gregarious") - 10.0 * p("gluttonous"),
        "revoke" => 25.0 * p("just") + 15.0 * p("kind") - 10.0 * p("arbitrary"),
        "defeat" => 40.0 + 30.0 * p("arrogant") - 15.0 * p("humble"),
        _ => 0.0,
    }
}

impl State {
    pub fn add_stress(&mut self, c: usize, amount: f64, why: &str) {
        if c == NONE || !self.chars[c].alive { return; }
        let before = stress_level(&self.chars[c]);
        self.chars[c].stress = (self.chars[c].stress + amount).clamp(0.0, 400.0);
        let after = stress_level(&self.chars[c]);
        if after > before { self.mental_break(c, after, why); }
    }

    pub fn reluct(&mut self, c: usize, act: &str, why: &str) {
        if c == NONE { return; }
        let r = reluctance(&self.chars[c], act);
        self.add_stress(c, r, why);
    }

    fn mental_break(&mut self, c: usize, level: i64, why: &str) {
        let after = if why.is_empty() { String::new() } else { format!(", after {why}") };
        if level >= 3 && self.rng.random() < 0.4 && !self.chars[c].coping.contains(&"mad") {
            self.chars[c].coping.push("mad");
            self.chars[c].stress -= 80.0;
            let text = format!("**{} breaks under it all and goes mad**{after}.", self.who(c));
            let minor = self.chars[c].ruled.is_none();
            self.say(text, minor);
            return;
        }
        let cc = &self.chars[c];
        let mut fits: Vec<&str> = COPING.iter().filter(|(h, n)| n.iter().any(|t| cc.has_trait(t)) && !cc.coping.contains(h)).map(|x| x.0).collect();
        if fits.is_empty() { fits = COPING.iter().map(|x| x.0).filter(|h| !cc.coping.contains(h)).collect(); }
        if fits.is_empty() { return; }
        let habit = fits[self.rng.integers(0, fits.len() as i64) as usize];
        self.chars[c].coping.push(habit);
        self.chars[c].stress -= 50.0;
        let text = format!("{} cracks under the strain{after}, and becomes a {habit}.", self.who(c));
        self.say(text, true);
    }

    pub fn love(&mut self, c: usize, other: usize, kind: &'static str, strength: f64) {
        if !self.chars[other].loved_by.contains(&c) { self.chars[other].loved_by.push(c); }
        let strong = matches!(kind, "lover" | "soulmate" | "spouse");
        let l = &mut self.chars[c].loves;
        match l.iter_mut().find(|x| x.0 == other) {
            Some(x) => if x.2 < strength || strong { x.1 = kind; x.2 = strength; },
            None => l.push((other, kind, strength)),
        }
    }

    fn attracted(&self, a: usize, b: usize) -> bool {
        let ok = |x: &Character, y: &Character| x.orientation == 2 || (x.orientation == 0 && x.female != y.female) || (x.orientation == 1 && x.female == y.female);
        ok(&self.chars[a], &self.chars[b]) && ok(&self.chars[b], &self.chars[a])
    }

    pub fn heart_on_birth(&mut self, kid: usize) {
        for p in [self.chars[kid].father, self.chars[kid].mother] {
            if p == NONE || !self.chars[p].alive { continue; }
            if !self.chars[p].has_trait("callous") { let s = self.rng.uniform(55.0, 95.0); self.love(p, kid, "child", s); }
            let s = self.rng.uniform(50.0, 90.0);
            self.love(kid, p, "parent", s);
        }
    }

    pub fn heart_on_marriage(&mut self, a: usize, b: usize) {
        let lover = self.chars[a].loves.iter().any(|x| x.0 == b && x.1 == "lover");
        if self.opinion(a, b) > 20.0 || lover {
            let (s1, s2) = (self.rng.uniform(40.0, 80.0), self.rng.uniform(40.0, 80.0));
            self.love(a, b, "spouse", s1);
            self.love(b, a, "spouse", s2);
        }
    }

    pub fn meters_year(&mut self) {
        let y = self.year;
        for i in 0..self.chars.len() {
            let c = &self.chars[i];
            if !c.alive || c.age(y) < 6 { continue; }
            let loved: f64 = c.loves.iter().map(|x| (x.2 / 5.0).min(20.0)).sum();
            let lover = if c.loves.iter().any(|x| x.1 == "lover") { 15.0 } else { 0.0 };
            let ruling = c.ruled.is_some_and(|r| r.2.is_none());
            let status = if ruling { 15.0 } else if !c.duchies.is_empty() { 8.0 } else if c.hero || !c.lordships.is_empty() { 5.0 } else { 0.0 };
            let p = |t: &str| c.has_trait(t) as u8 as f64;
            let natured = 10.0 * p("content") - if c.has_trait("ambitious") && status < 10.0 { 10.0 } else { 0.0 } + 5.0 * p("gregarious")
                - if c.has_trait("arrogant") && status < 10.0 { 5.0 } else { 0.0 };
            let mad = c.coping.contains(&"mad");
            let target = loved.min(45.0) + lover + status + natured + if c.pet.is_some() { 10.0 } else { 0.0 } - c.grief
                - 15.0 * stress_level(c) as f64 - if mad { 20.0 } else { 0.0 };
            let ease = 15.0 + if c.happy > 20.0 { 10.0 } else { 0.0 } + 6.0 * c.coping.len() as f64;
            let c = &mut self.chars[i];
            c.happy += 0.3 * (target.clamp(-100.0, 100.0) - c.happy);
            c.grief *= 0.85;
            c.stress = (c.stress - ease).max(0.0);
            if c.happy < -40.0 { self.add_stress(i, 5.0, "a long unhappiness"); }
            let c = &self.chars[i];
            if ruling {
                let k = &self.realms[c.realm];
                if c.has_trait("lazy") { let a = 3.0 * k.over_vassals + 2.0; self.add_stress(i, a, "the weight of a realm"); }
                let c = &self.chars[i];
                if c.has_trait("ambitious") && matches!(self.realms[c.realm].rank, "tribe" | "chiefdom") { self.add_stress(i, 4.0, "a small throne"); }
            }
            let c = &self.chars[i];
            if c.has_trait("ambitious") && status == 0.0 && c.age(y) > 40 { self.add_stress(i, 4.0, "a life of no account"); }
            let c = &self.chars[i];
            if c.has_trait("vengeful") && !c.vendettas.is_empty() { self.add_stress(i, 8.0, "an unavenged wrong"); }
            let c = &self.chars[i];
            if c.has_trait("lustful") && lover == 0.0 && c.spouse == NONE { self.add_stress(i, 3.0, "an empty bed"); }
            let c = &self.chars[i];
            let frac = c.age(y) as f64 / c.life.max(1.0);
            let mut th = 5.5 + if c.has(gene("hale")) { 1.0 } else { 0.0 } - if c.has(gene("sickly")) { 2.0 } else { 0.0 } - ((frac - 0.6) * 12.0).max(0.0)
                - 0.8 * stress_level(c) as f64 + c.happy / 60.0 - if c.coping.contains(&"drunkard") { 0.8 } else { 0.0 } - if c.coping.contains(&"comfort eater") { 0.5 } else { 0.0 };
            if c.ruled.is_some() && c.realm < self.realms.len() && self.realms[c.realm].physician { th += 0.6; }
            if c.undead { th = 8.0; }
            let hero_wound = c.hero && c.hp < 30.0;
            let c = &mut self.chars[i];
            c.health += 0.2 * (th - c.health);
            if hero_wound { c.health -= 0.3; }
            let (h, undead, grief) = (c.health, c.undead, c.grief);
            if !undead && h < 2.0 && self.rng.random() < (2.0 - h) * 0.03 {
                let why = format!("died of failing health{}", if grief > 20.0 { ", worn down by grief" } else { "" });
                self.kill(i, why);
                continue;
            }
            if let Some((kind, name, since, life)) = self.chars[i].pet {
                if y - since > life {
                    self.chars[i].pet = None;
                    self.grieve(i, NONE, 15.0, &format!("their {kind} {name}"));
                }
            }
        }
    }

    /// Who falls in love this year, who takes a lover, whose marriage warms, who adopts a dog.
    pub fn hearts_year(&mut self) {
        let y = self.year;
        for k in 0..self.realms.len() {
            if !self.active(k) { continue; }
            let circle: Vec<usize> = self.chars.iter().filter(|c| c.alive && c.realm == k && c.adult(y) && (c.house != NONE || c.hero || !c.duchies.is_empty())).map(|c| c.id).collect();
            if circle.len() < 2 { continue; }
            for _ in 0..(circle.len() / 12).max(1) {
                if self.rng.random() > 0.35 { continue; }
                let i = self.rng.integers(0, circle.len() as i64) as usize;
                let mut j = self.rng.integers(0, circle.len() as i64 - 1) as usize;
                if j >= i { j += 1; }
                let (a, b) = (circle[i], circle[j]);
                if self.chars[a].undead || self.chars[b].undead || !self.attracted(a, b) || self.chars[a].loves.iter().any(|x| x.0 == b) { continue; }
                let n = self.rng.normal(0.0, 25.0);
                let chem = self.opinion(a, b) + self.opinion(b, a) + 20.0 * self.chars[a].has_trait("lustful") as u8 as f64
                    + 20.0 * self.chars[b].has(gene("comely")) as u8 as f64 - 30.0 * self.chars[a].has_trait("chaste") as u8 as f64 + n;
                if chem < 95.0 { continue; }
                if self.chars[a].spouse == b {
                    let (s1, s2) = (self.rng.uniform(60.0, 100.0), self.rng.uniform(60.0, 100.0));
                    self.love(a, b, "spouse", s1); self.love(b, a, "spouse", s2);
                    continue;
                }
                let kind = if chem > 140.0 { "soulmate" } else { "lover" };
                let (s1, s2) = (self.rng.uniform(60.0, 100.0), self.rng.uniform(60.0, 100.0));
                self.love(a, b, kind, s1); self.love(b, a, kind, s2);
                let minor = !(self.chars[a].ruled.is_some() || self.chars[b].ruled.is_some());
                if self.chars[a].spouse == NONE && self.chars[b].spouse == NONE {
                    self.chars[a].spouse = b; self.chars[b].spouse = a;
                    self.heart_on_marriage(a, b);
                    let text = format!("{} and {} marry for love.", self.who(a), self.who(b));
                    self.say(text, minor);
                    continue;
                }
                self.chars[a].affair = b; self.chars[b].affair = a;
                let text = format!("{} and {} become lovers, in secret.", self.who(a), self.who(b));
                self.say(text, true);
                for (x, other) in [(a, b), (b, a)] {
                    if self.chars[x].ruled.is_some() { self.gain_hook(other, x, "weak", ", the hold of a lover"); }
                    if self.chars[x].has_trait("honest") || self.chars[x].has_trait("chaste") { self.add_stress(x, 15.0, "a secret love"); }
                }
                let spy = self.realms[k].council[2];
                if spy != NONE && self.rng.random() < self.chars[spy].attr(3, y) / 40.0 {
                    self.gain_hook(spy, a, "strong", ", knowing of an affair");
                    let sp = self.chars[a].spouse;
                    if self.rng.random() < 0.3 && sp != NONE {
                        self.remember(sp, a, "was unfaithful to me", -60.0, 1.0);
                        self.remember(sp, b, "seduced my spouse", -80.0, 0.5);
                        self.chars[sp].loves.retain(|x| x.0 != a);
                        self.add_stress(sp, 40.0, "a betrayal");
                        let minor = !(self.chars[a].ruled.is_some() || self.chars[sp].ruled.is_some());
                        let text = format!("**A scandal.** The affair of {} and {} comes out; {} will not forgive it.", self.who(a), self.who(b), self.who(sp));
                        self.say(text, minor);
                    }
                }
            }
            for c in circle {
                let sp = self.chars[c].spouse;
                if sp != NONE && self.chars[sp].alive && !self.chars[c].loves.iter().any(|x| x.0 == sp)
                    && self.opinion(c, sp) > 35.0 && self.rng.random() < 0.08 {
                    let s = self.rng.uniform(40.0, 70.0);
                    self.love(c, sp, "spouse", s);
                }
                if self.chars[c].pet.is_none() && self.rng.random() < 0.03 {
                    let (kind, life) = PETS[self.rng.integers(0, 6) as usize];
                    let name = PET_NAMES[self.rng.integers(0, 16) as usize];
                    self.chars[c].pet = Some((kind, name, y, life));
                }
            }
        }
        for i in 0..self.chars.len() {
            if !self.chars[i].alive || !self.chars[i].has_trait("fickle") { continue; }
            let loves = self.chars[i].loves.clone();
            let mut keep = vec![];
            for l in loves { if matches!(l.1, "lover" | "soulmate") && self.rng.random() < 0.08 { continue; } keep.push(l); }
            self.chars[i].loves = keep;
        }
    }

    fn grieve(&mut self, c: usize, by: usize, amount: f64, whom: &str) {
        self.chars[c].grief += amount;
        self.add_stress(c, amount, &format!("losing {whom}"));
        if by != NONE && by != c {
            let decay = if self.chars[c].has_trait("vengeful") { 0.0 } else { 0.3 };
            self.remember(c, by, "killed my beloved", -100.0, decay);
            if !self.chars[c].has_trait("forgiving") {
                let y = self.year;
                let v = &mut self.chars[c].vendettas;
                v.retain(|x| x.0 != by);
                v.push((by, y, whom.to_string()));
            }
        }
    }

    /// Everyone who loved the dead grieves; if someone is to blame, they remember.
    pub fn heart_on_death(&mut self, dead: usize, by: usize) {
        let y = self.year;
        for c in self.chars[dead].loved_by.clone() {
            if !self.chars[c].alive { continue; }
            let Some(&(_, kind, strength)) = self.chars[c].loves.iter().find(|x| x.0 == dead) else { continue };
            self.chars[c].loves.retain(|x| x.0 != dead);
            let word = match kind { "child" | "parent" | "spouse" | "lover" | "soulmate" => kind, _ => "beloved" };
            let mut weight = match kind { "child" => if self.chars[dead].age(y) < 5 { 0.4 } else { 0.9 }, "parent" => 0.35, "soulmate" => 1.0, "spouse" => 0.7, "lover" => 0.6, _ => 0.5 };
            if by != NONE { weight = f64::max(weight, 1.0); }
            let name = self.chars[dead].name.clone();
            self.grieve(c, by, strength * weight, &format!("{word} {name}"));
            if strength > 70.0 && matches!(kind, "child" | "soulmate" | "spouse") && !self.chars[c].coping.contains(&"heartbroken") { self.chars[c].coping.push("heartbroken"); }
            if by != NONE && self.chars[c].ruled.is_some() && matches!(kind, "child" | "spouse" | "soulmate" | "lover") && self.chars[by].realm != self.chars[c].realm {
                let text = format!("{} swears vengeance on {} for the death of their {word} {name}.", self.who(c), self.who(by));
                self.say(text, false);
            }
        }
    }

    /// A living enemy a ruler has sworn to punish, in another living realm.
    pub fn vengeance_target(&mut self, r: usize) -> Option<(usize, String)> {
        let vs = self.chars[r].vendettas.clone();
        for (v, _, whom) in vs {
            if !self.chars[v].alive { self.chars[r].vendettas.retain(|x| x.0 != v); continue; }
            let vr = self.chars[v].realm;
            if vr != self.chars[r].realm && vr < self.realms.len() && !self.realms[vr].extinct { return Some((v, whom)); }
        }
        None
    }

    pub fn avenged(&mut self, r: usize, v: usize, whom: &str) {
        self.chars[r].vendettas.retain(|x| x.0 != v);
        self.chars[r].stress = (self.chars[r].stress - 80.0).max(0.0);
        self.chars[r].grief = (self.chars[r].grief - 40.0).max(0.0);
        let text = format!("{} has their vengeance on {} for {whom}.", self.who(r), self.chars[v].name);
        self.say(text, false);
    }
}
