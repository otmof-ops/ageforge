//! Education, lifestyles and perks, the royal court, and youth (court.py, youth.py): what a child practises of
//! what their parents were best at, the education they gain at coming of age, the lifestyle they follow after,
//! and the court a kingdom or empire keeps, with its amenities, positions, grandeur and events.

use crate::people::*;
use crate::power::skill_ix;
use crate::state::State;
use crate::tables::*;

const POSITIONS: [&str; 8] = ["Champion", "Court Physician", "Master of the Hunt", "Court Poet", "Royal Architect", "Antiquarian", "Court Tutor", "Bodyguard"];
const EDU_BONUS: [f64; 4] = [1.0, 3.0, 5.0, 7.0];

fn expected(rank: &str) -> Option<f64> { match rank { "kingdom" => Some(30.0), "empire" => Some(50.0), "hegemony" => Some(70.0), _ => None } }

pub fn perk_value(c: &Character, effect: &str) -> f64 {
    let v = PERK_VALUE.iter().find(|p| p.0 == effect).map(|p| p.1).unwrap_or(0.0);
    c.perk_list.iter().filter(|p| p.1 == effect).count() as f64 * v
}

impl State {
    /// The parts of the six stats that come from legacy, class effects, education and perks (Character._attr),
    /// refreshed once a year for the living.
    pub fn refresh_stats(&mut self) {
        for i in 0..self.chars.len() {
            if !self.chars[i].alive { continue; }
            let c = &self.chars[i];
            let m = self.mods(c);
            let mut e = [0.0; 6];
            e[0] += self.legacy_bonus(c, "dip").floor();
            e[1] += self.legacy_bonus(c, "mar").floor() + m[9];
            e[3] += m[10];
            if let Some((f, t)) = c.education { e[f] += EDU_BONUS[t]; }
            e[0] += perk_value(c, "dip");
            e[3] += perk_value(c, "intr");
            self.chars[i].extra = e;
        }
    }

    fn educate(&mut self, c: usize) {
        if self.chars[c].education.is_some() { return; }
        let mut pool = [0.0; 5];
        for (a, p) in pool.iter_mut().enumerate() { *p = self.chars[c].attrs[a]; }
        for p in [self.chars[c].father, self.chars[c].mother] {
            if p == NONE { continue; }
            let best = (0..5).max_by(|&a, &b| self.chars[p].attrs[a].partial_cmp(&self.chars[p].attrs[b]).unwrap()).unwrap();
            pool[best] += 2.0;
        }
        let mut best = (f64::MIN, 0);
        for (a, &v) in pool.iter().enumerate() { let s = v + self.rng.random() * 3.0; if s > best.0 { best = (s, a); } }
        let cc = &self.chars[c];
        let roll = self.rng.random() * 3.0 + cc.practice as f64 * 0.25 + if cc.has_trait("diligent") { 1.0 } else { 0.0 } - if cc.has_trait("lazy") { 1.0 } else { 0.0 }
            + if cc.has(gene("genius")) { 0.5 } else { 0.0 } + cc.tutored;
        let tier = if roll < 1.6 { 0 } else if roll < 3.0 { 1 } else if roll < 4.3 { 2 } else { 3 };
        let cc = &mut self.chars[c];
        cc.education = Some((best.1, tier));
        cc.ls_xp = 0.0;
        cc.perk_list.clear();
    }

    fn lifestyle_year(&mut self, c: usize) {
        let Some((focus, tier)) = self.chars[c].education else { return };
        if self.chars[c].perk_list.len() >= 9 { return; }
        let cc = &mut self.chars[c];
        cc.ls_xp += 12.0 + cc.attrs[focus] + 4.0 * tier as f64;
        let need = 120.0 * (cc.perk_list.len() + 1) as f64;
        if cc.ls_xp < need { return; }
        cc.ls_xp -= need;
        let taken: Vec<&str> = cc.perk_list.iter().map(|p| p.2).collect();
        let mut trees: Vec<&(&str, &str, [&str; 3])> = LIFESTYLES[focus].iter().collect();
        trees.sort_by_key(|t| std::cmp::Reverse(t.2.iter().filter(|p| taken.contains(p)).count()));
        for &(tree, eff, perks) in trees {
            let nxt: Vec<&str> = perks.iter().copied().filter(|p| !taken.contains(p)).collect();
            if let Some(&p) = nxt.first() {
                self.chars[c].perk_list.push((tree, eff, p));
                if self.chars[c].ruled.is_some() && nxt.len() == 1 {
                    let name = ["Diplomacy", "Martial", "Stewardship", "Intrigue", "Learning"][focus];
                    let t = format!("{} completes the {tree} path of the {name} lifestyle: {p}.", self.who(c));
                    self.say(t, true);
                }
                break;
            }
        }
    }

    fn people_year(&mut self) {
        let y = self.year;
        for i in 0..self.chars.len() {
            let c = &self.chars[i];
            if !c.alive || !c.adult(y) { continue; }
            if c.education.is_none() { self.educate(i); }
            let c = &self.chars[i];
            if c.ruled.is_some() || c.hero || !c.duchies.is_empty() || !c.lordships.is_empty() || c.profession.is_some() { self.lifestyle_year(i); }
        }
    }

    fn court_year(&mut self) {
        let y = self.year;
        for k in 0..self.realms.len() {
            if !self.active(k) { continue; }
            let Some(exp) = expected(self.realms[k].rank) else { continue };
            if matches!(self.realms[k].gov, "tribal" | "republic" | "theocracy") { continue; }
            let r = self.realms[k].ruler;
            if self.realms[k].court.is_none() {
                self.realms[k].court = Some(Court { since: y, grandeur: 20.0, amen: [1; 4], pos: vec![], kind: "", events: vec![] });
                let cap = self.capital_of(k);
                let place = if cap != NONE { self.towns[cap].name.clone() } else { self.realms[k].name.clone() };
                let t = format!("**A royal court.** {} gathers a royal court at {place}.", self.who(r));
                self.say(t, false);
            }
            let focus = (0..5).max_by(|&a, &b| self.chars[r].attr(a, y).partial_cmp(&self.chars[r].attr(b, y)).unwrap()).unwrap();
            let realm = &mut self.realms[k];
            let revenue = realm.income * realm.tax_now * realm.collect;
            let ct = realm.court.as_mut().unwrap();
            ct.kind = ["Diplomatic", "Warlike", "Administrative", "Intrigue", "Scholarly"][focus];
            let cost = 1.0 + ct.amen.iter().map(|&v| (v as f64).powf(1.5)).sum::<f64>() / 4.0 * 0.08 * revenue;
            realm.gold -= cost;
            self.books.push((k, "court", -cost, String::new(), NONE));
            if realm.gold > cost * 6.0 { let i = (0..4).min_by_key(|&a| ct.amen[a]).unwrap(); ct.amen[i] = (ct.amen[i] + 1).min(5); }
            else if realm.gold < 0.0 { let i = (0..4).max_by_key(|&a| ct.amen[a]).unwrap(); ct.amen[i] = (ct.amen[i].max(2)) - 1; }
            // positions
            let pool: Vec<usize> = self.chars.iter().filter(|c| c.alive && c.realm == k && c.adult(y) && c.id != r).map(|c| c.id).collect();
            let mut used: Vec<usize> = vec![];
            for pos in POSITIONS {
                let cur = self.realms[k].court.as_ref().unwrap().pos.iter().find(|p| p.0 == pos).map(|p| p.1);
                if let Some(cur) = cur { if self.chars[cur].alive && self.chars[cur].realm == k { used.push(cur); continue; } }
                let cands: Vec<usize> = pool.iter().copied().filter(|c| !used.contains(c)).collect();
                let ct = self.realms[k].court.as_mut().unwrap();
                ct.pos.retain(|p| p.0 != pos);
                if cands.is_empty() { continue; }
                let score = |c: &Character| -> f64 { match pos {
                    "Champion" => self.eff_prowess(c) + c.level as f64 / 3.0,
                    "Court Physician" => c.skills[skill_ix("Restoration")] + c.attr(4, y),
                    "Master of the Hunt" => c.skills[skill_ix("Archery")] + c.skills[skill_ix("Beast Taming")],
                    "Court Poet" => c.attr(5, y) * 5.0, "Royal Architect" => c.attr(2, y) * 5.0, "Antiquarian" => c.attr(4, y) * 4.0,
                    "Court Tutor" => c.attr(4, y) * 3.0 + c.attr(0, y) * 2.0, _ => self.eff_prowess(c) * 2.0 } };
                let best = *cands.iter().max_by(|&&a, &&b| score(&self.chars[a]).partial_cmp(&score(&self.chars[b])).unwrap()).unwrap();
                self.realms[k].court.as_mut().unwrap().pos.push((pos, best));
                used.push(best);
                let t = format!("{pos} of {} from year {y}", self.realms[k].name);
                self.chars[best].titles.push(t);
                self.remember(best, r, "made me a court officer", 12.0, 0.5);
                self.realms[k].gold -= 5.0;
            }
            let arts: Vec<usize> = self.realms[k].treasury.iter().chain(self.chars[r].items.iter()).copied().filter(|&i| self.items[i].rarity >= 3).collect();
            let art_g: f64 = arts.iter().map(|&i| (self.items[i].rarity as f64).powf(1.4).min(8.0)).sum();
            let ct = self.realms[k].court.as_mut().unwrap();
            let g = ct.amen.iter().sum::<u32>() as f64 * 3.5 + 2.5 * ct.pos.len() as f64 + art_g + 0.05 * (y - ct.since) as f64;
            ct.grandeur = 0.8 * ct.grandeur + 0.2 * g.min(100.0);
            let diff = ct.grandeur - exp;
            let l = self.chars[r].legitimacy.unwrap_or(50.0);
            self.chars[r].legitimacy = Some((l + diff / 40.0).clamp(0.0, 100.0));
            self.realms[k].court_mod = diff / 4.0;
            self.court_events(k, r);
            let pos = self.realms[k].court.as_ref().unwrap().pos.clone();
            let get = |n: &str| pos.iter().find(|p| p.0 == n).map(|p| p.1);
            self.realms[k].physician = get("Court Physician").is_some_and(|c| self.chars[c].alive);
            self.realms[k].bodyguard = get("Bodyguard").is_some();
            if let Some(tutor) = get("Court Tutor") {
                let t = 1.0 + self.chars[tutor].attr(4, y) / 20.0;
                for kid in self.chars[r].children.clone() { if self.chars[kid].alive && !self.chars[kid].adult(y) { self.chars[kid].tutored = t; } }
            }
        }
    }

    fn court_events(&mut self, k: usize, r: usize) {
        let y = self.year;
        let roll = self.rng.random();
        let pos = self.realms[k].court.as_ref().unwrap().pos.clone();
        let get = |n: &str| pos.iter().find(|p| p.0 == n).map(|p| p.1);
        let food = self.realms[k].court.as_ref().unwrap().amen[0];
        let name = self.realms[k].name.clone();
        let mut event = None;
        if roll < 0.04 && food >= 3 {
            let l = self.chars[r].legitimacy.unwrap_or(50.0);
            self.chars[r].legitimacy = Some((l + 5.0).min(100.0));
            event = Some("a grand feast".to_string());
            let t = format!("{} holds a grand feast at court; the great of {name} eat and remember.", self.who(r));
            self.say(t, true);
            self.reluct(r, "feast", "a feast");
        } else if roll < 0.08 && get("Champion").is_some() {
            let champ = get("Champion").unwrap();
            self.chars[champ].renown += 40.0;
            self.chars[r].renown += 20.0;
            event = Some(format!("a tournament won by {}", self.chars[champ].name));
            let t = format!("A great tournament at the court of {name}: {} carries the day.", self.who(champ));
            self.say(t, true);
        } else if roll < 0.11 && get("Master of the Hunt").is_some() {
            self.chars[r].renown += 25.0;
            event = Some("a royal hunt".into());
        } else if roll < 0.14 && get("Court Poet").is_some() {
            self.gain_legacy(r, "ruler", 60.0);
            event = Some("a court poet sings of the ruler".into());
        } else if roll < 0.17 {
            let smiths: Vec<usize> = self.chars.iter().filter(|c| c.alive && c.realm == k && matches!(c.profession, Some("smith") | Some("enchanter"))).map(|c| c.id).collect();
            if !smiths.is_empty() && self.realms[k].gold > 150.0 {
                let sm = *smiths.iter().max_by(|&&a, &&b| {
                    let s = |c: usize| self.chars[c].skills[skill_ix("Smithing")] + self.chars[c].skills[skill_ix("Enchanting")];
                    s(a).partial_cmp(&s(b)).unwrap()
                }).unwrap();
                let smith = self.chars[sm].skills[skill_ix("Smithing")];
                let nm = self.master_name();
                let mut it = Item::new(nm.clone(), if smith < 80.0 { 4 } else { 5 }, y, format!("made by {} for the court of {name}", self.chars[sm].name));
                it.bonus = 0.5 + smith / 200.0;
                let id = self.new_item(it);
                self.realms[k].gold -= 150.0;
                self.realms[k].treasury.push(id);
                self.gain_fame(sm, "Smithing", 80.0);
                event = Some(format!("{} is inspired: {nm}", self.chars[sm].name));
                let t = format!("{}, inspired at the court of {name}, makes {nm} for the throne.", self.who(sm));
                self.say(t, false);
            }
        }
        if let Some(e) = event { self.realms[k].court.as_mut().unwrap().events.push((y, e)); }
    }

    pub fn courts_and_people_year(&mut self) {
        self.people_year();
        self.court_year();
    }

    fn marked(c: &Character) -> bool { c.marks.iter().any(|m| *m == HERO_MARK[0] || *m == SEER_GIFT[0]) }

    /// What a child does with what was handed down to them (youth.youth_year).
    pub fn youth_year(&mut self) {
        let y = self.year;
        for i in 0..self.chars.len() {
            let c = &self.chars[i];
            if !c.alive { continue; }
            let age = c.age(y);
            let adult = ADULT[c.people] as i64;
            if age < 6 || age > adult { continue; }
            let parents: Vec<usize> = [c.father, c.mother].into_iter().filter(|&p| p != NONE).collect();
            let marked = Self::marked(c);
            if parents.is_empty() && !marked { continue; }
            if age == adult { self.come_of_age(i, &parents); continue; }
            let mut p = 0.6 + 0.25 * c.has_trait("diligent") as u8 as f64 - 0.35 * c.has_trait("lazy") as u8 as f64 + 0.1 * c.has_trait("ambitious") as u8 as f64;
            let mult = if marked { 2.0 } else { 1.0 };
            if marked { p += 0.3; }
            if self.rng.random() > p { continue; }
            self.chars[i].practice += 1;
            let rate = 0.05 * mult * (20.0 / ((adult - 6) as f64).max(10.0));
            for &par in &parents {
                let mut top: Vec<(f64, usize)> = self.chars[par].skills.iter().enumerate().map(|(s, &v)| (v, s)).collect();
                top.sort_by(|a, b| b.partial_cmp(a).unwrap());
                for &(v, s) in top.iter().take(4) { if v > self.chars[i].skills[s] { let d = (v - self.chars[i].skills[s]) * rate; self.chars[i].skills[s] += d; } }
                let mut at: Vec<usize> = (0..8).collect();
                at.sort_by(|&a, &b| self.chars[par].attributes[b].partial_cmp(&self.chars[par].attributes[a]).unwrap());
                for &a in at.iter().take(2) {
                    let (pv, cv) = (self.chars[par].attributes[a], self.chars[i].attributes[a]);
                    if pv > cv { self.chars[i].attributes[a] += ((pv - cv) * rate).floor().max(1.0); }
                }
                for a in 0..6 {
                    if self.chars[par].attrs[a] > self.chars[i].attrs[a] && self.rng.random() < rate * 2.0 { self.chars[i].attrs[a] += 1.0; }
                }
            }
            if marked { self.fight_skills(i, 1.0); }
            self.gain_xp(i, (30.0 + 4.0 * age as f64) * mult);
        }
    }

    pub fn fight_skills(&mut self, c: usize, amount: f64) {
        let cls = self.chars[c].classes[0].0;
        self.use_skill(c, WEAPON_SKILL[cls], amount);
        let armor = if matches!(CLASSES[cls].name, "Warrior" | "Knight" | "Runesmith") { "Heavy Armor" } else { "Light Armor" };
        self.use_skill(c, skill_ix(armor), amount * 0.7);
        self.use_skill(c, skill_ix("Block"), amount * 0.4);
        if let Some(s) = self.school(&self.chars[c]) { self.use_skill(c, SCHOOL_SKILL[s], amount); }
    }

    fn come_of_age(&mut self, c: usize, parents: &[usize]) {
        self.educate(c);
        let years = self.chars[c].practice;
        if years > 0 && !parents.is_empty() {
            let best = (0..self.chars[c].skills.len()).max_by(|&a, &b| self.chars[c].skills[a].partial_cmp(&self.chars[c].skills[b]).unwrap());
            self.chars[c].trained_in = best;
        }
        if !Self::marked(&self.chars[c]) { return; }
        self.chars[c].renown += 80.0 + 25.0 * years as f64;
        let spec = self.chars[c].trained_in.map(|s| SKILLS[s]).unwrap_or("Blade");
        self.gain_fame(c, spec, 30.0 * years as f64);
        let who = if parents.is_empty() { "no one living".to_string() } else { parents.iter().map(|&p| self.chars[p].name.clone()).collect::<Vec<_>>().join(" and ") };
        let mark = if self.chars[c].marks.contains(&HERO_MARK[0]) { HERO_MARK[0] } else { SEER_GIFT[0] };
        let t = format!("{}, who bears {mark}, comes of age after {years} years of practice in what {who} passed down to them; the songs already speak of them (renown {}).",
                        self.who(c), crate::economy::thousands(self.chars[c].renown));
        self.say(t, years < 5);
    }
}
