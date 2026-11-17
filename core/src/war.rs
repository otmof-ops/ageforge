//! War and the limits of rule (statecraft.py, warfare.py and world.py's borders, diplomacy, battles, events and
//! cleanup): domain and vassal limits, overstretch, the warbands' strong eating the weak, last stands, casus belli,
//! declarations and war scores, peace terms, tributaries, rivals, great decisions; hosts that march four times a
//! year, field battles and sieges, risings and free communes; frontier battles; blight; abandoned towns.
//! (Vengeance and love join with the heart; faiths with slice 5; heroes and dragons in battle with slice 6.)

use crate::people::*;
use crate::state::State;
use crate::tables::*;

const TICKS: usize = 4;
const SPEED: usize = 9;
const COHESION: [(&str, f64); 6] = [("tribal", 12.0), ("clan", 20.0), ("feudal", 45.0), ("theocracy", 40.0), ("republic", 30.0), ("administrative", 90.0)];
const DOMAIN_BASE: [f64; 4] = [6.0, 5.0, 6.0, 3.0];

fn rank_vassals(r: &str) -> f64 { match r { "tribe" => 0.0, "chiefdom" => 2.0, "duchy" => 5.0, "kingdom" => 10.0, "empire" => 20.0, _ => 5.0 } }
fn rank_domain_limit(r: &str) -> f64 { match r { "tribe" => 2.0, "chiefdom" => 3.0, "duchy" => 4.0, "kingdom" => 5.0, "empire" => 6.0, _ => 4.0 } }
fn rank_domain(r: &str) -> f64 { match r { "tribe" => 0.0, "chiefdom" => 1.0, "duchy" => 2.0, "kingdom" => 3.0, "empire" => 4.0, _ => 1.0 } }
fn dist(a: (usize, usize), b: (usize, usize)) -> f64 { (a.0 as f64 - b.0 as f64).hypot(a.1 as f64 - b.1 as f64) }
fn cap1(s: &str) -> String { let mut c = s.chars(); match c.next() { Some(f) => f.to_uppercase().collect::<String>() + c.as_str(), None => String::new() } }

impl State {
    pub fn pname(&self, f: usize) -> String {
        let k = &self.realms[f];
        if self.realms.iter().filter(|o| o.people == k.people).count() <= 1 { PEOPLES[k.people].to_string() } else { k.short.clone() }
    }

    fn at_war(&self, k: usize) -> bool { self.war[k].iter().any(|&w| w) }

    fn naval_contact(&self, i: usize, j: usize) -> bool {
        let (a, b) = (&self.realms[i], &self.realms[j]);
        a.warships >= 3.0 && b.warships >= 3.0 && (a.known.contains(&b.home_land) || b.known.contains(&a.home_land))
    }

    fn contact(&self, k: usize, o: usize) -> bool {
        let lk: Vec<i32> = self.alive_towns_of(k).iter().map(|&t| self.towns[t].landmass).collect();
        if self.alive_towns_of(o).iter().any(|&t| lk.contains(&self.towns[t].landmass)) { return true; }
        self.naval_contact(k, o)
    }

    // ------------------------------------------------------------ limits
    fn limits_year(&mut self) {
        let y = self.year;
        for k in 0..self.realms.len() {
            if !self.active(k) { continue; }
            let r = self.realms[k].ruler;
            let ste = self.chars[r].attr(2, y);
            let vassals = self.duchies.values().filter(|d| d.ctrl == k && d.holder != NONE && d.holder != r).count() as f64;
            let demesne = self.alive_towns_of(k).iter().filter(|&&t| self.towns[t].duchy.and_then(|d| self.duchies.get(&d)).is_none_or(|d| d.holder == NONE || d.holder == r)).count() as f64;
            let realm = &mut self.realms[k];
            realm.vassal_limit = rank_vassals(realm.rank) + 5.0 + (ste / 2.0).floor();
            realm.domain_limit = rank_domain_limit(realm.rank) + (ste / 4.0).floor();
            realm.over_vassals = (vassals - realm.vassal_limit).max(0.0);
            let over_domain = (demesne - realm.domain_limit * 4.0).max(0.0);
            realm.tax_mult = (1.0 - 0.02 * realm.over_vassals - 0.01 * over_domain).max(0.4);
        }
    }

    fn domain_year(&mut self) {
        let y = self.year;
        for k in 0..self.realms.len() {
            if !self.active(k) { continue; }
            let r = self.realms[k].ruler;
            let inn = &self.cultures[self.realms[k].people].innovations;
            let limit = DOMAIN_BASE[self.realms[k].people] + (self.chars[r].attr(2, y) / 5.0).floor() + rank_domain(self.realms[k].rank)
                + inn.contains("Written Law") as u8 as f64 + 2.0 * inn.contains("Royal Chancery") as u8 as f64 + 3.0 * inn.contains("Bureaucracy") as u8 as f64;
            let mine: Vec<DKey> = self.duchies.iter().filter(|(_, d)| d.ctrl == k && (d.holder == NONE || d.holder == r)).map(|(dk, _)| *dk).collect();
            let held = self.alive_towns_of(k).iter().filter(|&&t| self.towns[t].duchy.is_some_and(|d| mine.contains(&d))).count() as f64;
            self.realms[k].domain_limit = limit;
            let over = (held - limit).max(0.0);
            self.realms[k].domain_penalty = (over / limit.max(1.0)).min(1.0);
            if over == 0.0 { continue; }
            let seat = self.capital_of(k);
            let sd = if seat != NONE { self.towns[seat].duchy } else { None };
            let vacant: Vec<DKey> = mine.into_iter().filter(|dk| self.duchies[dk].holder == NONE && Some(*dk) != sd).collect();
            let mut cands: Vec<usize> = self.chars.iter().filter(|c| c.alive && c.realm == k && c.adult(y) && c.id != r && !c.undead && c.duchies.is_empty()).map(|c| c.id).collect();
            for dk in vacant {
                if cands.is_empty() { break; }
                let i = (0..cands.len()).max_by(|&a, &b| {
                    let s = |c: usize| self.chars[c].attr(2, y) + self.chars[c].attr(1, y);
                    s(cands[a]).partial_cmp(&s(cands[b])).unwrap()
                }).unwrap();
                let c = cands.remove(i);
                self.give_duchy_of(c, dk, k);
            }
        }
    }

    /// Among the warbands the strong rule the weak.
    fn strong_eat_weak(&mut self) {
        let orcs: Vec<usize> = (0..self.realms.len()).filter(|&k| self.active(k) && self.realms[k].people == 3).collect();
        let mut size: Vec<usize> = vec![0; self.realms.len()];
        let mut lands: Vec<Vec<i32>> = vec![vec![]; self.realms.len()];
        for &k in &orcs { let ts = self.alive_towns_of(k); size[k] = ts.len(); lands[k] = ts.iter().map(|&t| self.towns[t].landmass).collect(); }
        for &small in &orcs {
            if self.realms[small].extinct || !(1..=3).contains(&size[small]) || self.rng.random() > 0.15 { continue; }
            let bigger: Vec<usize> = orcs.iter().copied().filter(|&b| b != small && !self.realms[b].extinct && size[b] >= 3 * size[small]
                && lands[b].iter().any(|l| lands[small].contains(l)) && (self.war[b][small] || self.relation[b][small] < 0.0)).collect();
            let Some(&b) = bigger.iter().max_by_key(|&&b| size[b]) else { continue };
            let loser = self.realms[small].ruler;
            let lname = if loser != NONE { format!("; {} kneels or dies.", self.chars[loser].name) } else { ".".into() };
            let sname = self.realms[small].name.clone();
            self.absorb(b, small);
            size[b] += size[small];
            let text = format!("{} beats {sname} into the warband{lname}", self.who(self.realms[b].ruler));
            self.say(text, size[b] < 20);
        }
    }

    /// A realm can hold only so much; far chiefs drift away when it grows past it.
    fn overstretch(&mut self) {
        let y = self.year;
        for k in 0..self.realms.len() {
            if !self.active(k) { continue; }
            let r = self.realms[k].ruler;
            let towns = self.alive_towns_of(k);
            let inn = &self.cultures[self.realms[k].people].innovations;
            let admin = 15.0 * inn.contains("Written Law") as u8 as f64 + 25.0 * inn.contains("Royal Chancery") as u8 as f64 + 40.0 * inn.contains("Bureaucracy") as u8 as f64;
            let orc = self.realms[k].people == 3;
            let coh = COHESION.iter().find(|c| c.0 == self.realms[k].gov).map(|c| c.1).unwrap_or(30.0);
            let rc = &self.chars[r];
            let cap = (coh + admin) * (1.0 + (rc.attr(0, y) + rc.attr(2, y)) / 40.0) * (rc.legitimacy.unwrap_or(50.0) / 60.0).max(0.4) * if orc { 0.45 } else { 1.0 };
            let n = towns.len() as f64;
            if n <= cap || self.rng.random() > 0.05 * (n / cap - 1.0) * if orc { 3.0 } else { 1.0 } { continue; }
            let seat = self.capital_of(k);
            if seat == NONE { continue; }
            let held: Vec<DKey> = self.duchies.iter().filter(|(dk, d)| d.ctrl == k && d.holder != NONE && d.holder != r && self.chars[d.holder].alive
                && towns.iter().filter(|&&t| self.towns[t].duchy == Some(**dk)).count() >= 3
                && (orc || ["ambitious", "greedy", "arrogant", "vengeful"].iter().any(|t| self.chars[d.holder].has_trait(t)))).map(|(dk, _)| *dk).collect();
            if held.is_empty() { continue; }
            let sp = self.towns[seat].pos;
            let far = |dk: &DKey| towns.iter().filter(|&&t| self.towns[t].duchy == Some(*dk)).map(|&t| dist(self.towns[t].pos, sp)).fold(f64::MAX, f64::min);
            let dk = *held.iter().max_by(|a, b| far(a).partial_cmp(&far(b)).unwrap()).unwrap();
            let leader = self.duchies[&dk].holder;
            let text = format!("**The realm frays.** {} has grown past what {} can hold; {} takes {} and goes their own way.",
                               self.realms[k].name, self.who(r), self.who(leader), self.duchies[&dk].name);
            self.say(text, false);
            self.secede(k, &[dk], leader, 0);
        }
    }

    fn last_stands(&mut self) {
        let mut held = [0usize; 4];
        for t in self.towns.iter().filter(|t| t.alive) { held[t.culture] += 1; }
        for p in 0..4 { self.people_peak[p] = self.people_peak[p].max(held[p]); }
        for k in 0..self.realms.len() {
            if self.realms[k].extinct { continue; }
            let r = self.realms[k].ruler;
            let floor = if r != NONE && self.chars[r].has_trait("paranoid") { 30.0 } else { 0.0 };
            self.realms[k].security = (self.realms[k].security * 0.97).max(floor);
            let p = self.realms[k].people;
            let (n, top) = (held[p], self.people_peak[p]);
            let stand = top >= 10 && (n as f64) <= (8.0f64).max(0.2 * top as f64);
            if stand && !self.realms[k].last_stand {
                let text = format!("**A last stand.** The {} hold only {n} towns of the {top} their people once held; in {} every hand takes up a spear.", self.pname(k), self.realms[k].name);
                self.say(text, false);
            }
            self.realms[k].last_stand = stand;
            if stand { self.realms[k].battle_mult *= 1.6; }
        }
    }

    // ------------------------------------------------------------ casus belli and war
    fn nearest_duchy(&self, k: usize, o: usize) -> Option<DKey> {
        let cap = self.capital_of(k);
        if cap == NONE { return None; }
        let cp = self.towns[cap].pos;
        let mut best: Option<(f64, DKey)> = None;
        for (dk, d) in &self.duchies {
            if d.ctrl != o { continue; }
            let dmin = self.alive_towns_of(o).iter().filter(|&&t| self.towns[t].duchy == Some(*dk)).map(|&t| dist(self.towns[t].pos, cp)).fold(f64::MAX, f64::min);
            if dmin < f64::MAX && best.is_none_or(|b| dmin < b.0) { best = Some((dmin, *dk)); }
        }
        best.map(|b| b.1)
    }

    /// The best reason this realm has for war on another, or none.
    fn cb_against(&mut self, k: usize, o: usize) -> Option<(String, Option<DKey>, bool, usize)> {
        let r = self.realms[k].ruler;
        if r == NONE { return None; }
        if let Some((v, whom)) = self.vengeance_target(r) {
            if self.chars[v].realm == o {
                let dk = self.nearest_duchy(k, o);
                self.avenge_goal = (k, o, v);
                return Some((format!("vengeance for {whom}"), dk, false, NONE));
            }
        }
        let mine: Vec<DKey> = self.ktitles.iter().filter(|(_, kd)| kd.ctrl == k).map(|(kt, _)| *kt).collect();
        for (dk, d) in &self.duchies { if d.ctrl == o && mine.contains(&d.kt) { return Some(("de jure claim".into(), Some(*dk), false, NONE)); } }
        let (rk, ro) = (&self.realms[k], &self.realms[o]);
        if matches!(rk.gov, "tribal" | "clan") && matches!(ro.rank, "tribe" | "chiefdom") && ro.people == rk.people && rk.army_power > 3.0 * ro.army_power.max(1.0) {
            return Some(("subjugation".into(), None, true, NONE));
        }
        let y = self.year;
        let rh = self.chars[r].house;
        if ro.house != NONE && rh != NONE && self.chars[r].has_trait("ambitious") {
            if let Some(kin) = self.chars.iter().find(|c| c.alive && c.house == rh && c.adult(y) && c.spouse != NONE && self.chars[c.spouse].house == ro.house && c.id != r) {
                return Some(("claim".into(), None, false, kin.id));
            }
        }
        let target = self.nearest_duchy(k, o)?;
        if rk.faith != ro.faith && (!rk.tolerant || self.religions[rk.faith].has("holy war")) { return Some(("holy war".into(), Some(target), false, NONE)); }
        let lacks: Vec<usize> = ro.held.iter().copied().filter(|&x| RESOURCES[x].kind == "strategic" && !rk.held.contains(&x)).collect();
        if let Some(&want) = lacks.first() {
            let rd = self.alive_towns_of(o).into_iter().find(|&t| {
                let p = self.towns[t].pos;
                // a duchy of the other realm whose land holds the deposit
                (p.0.saturating_sub(8)..(p.0 + 9).min(self.map.h)).any(|rr| (p.1.saturating_sub(8)..(p.1 + 9).min(self.map.w)).any(|cc| self.res_grid.at(rr, cc) == want as i16 && self.owner.at(rr, cc) == t as i64))
            }).and_then(|t| self.towns[t].duchy);
            return Some((format!("resource war for {}", RESOURCES[want].name), rd.or(Some(target)), false, NONE));
        }
        let rc = &self.chars[r];
        if rk.agenda == "conquest" || (rc.has_trait("ambitious") && rc.has_trait("wrathful")) { return Some(("conquest".into(), Some(target), false, NONE)); }
        None
    }

    fn start_war(&mut self, a: usize, b: usize, cb: (String, Option<DKey>, bool, usize), freedom: bool) {
        let y = self.year;
        self.war[a][b] = true; self.war[b][a] = true;
        self.war_since[a][b] = y; self.war_since[b][a] = y;
        self.weariness[a][b] = 0.0; self.weariness[b][a] = 0.0;
        let (name, duchy, subj, claimant) = cb;
        let goal_txt = if let Some(dk) = duchy.filter(|d| self.duchies.contains_key(d)) { format!(" for {}", self.duchies[&dk].name) }
            else if subj { " to make it a tributary".into() }
            else if claimant != NONE { format!(" to put {} on its throne", self.chars[claimant].name) } else { String::new() };
        let txt = format!(" ({name}{goal_txt})");
        self.wars.retain(|w| !((w.attacker == a && w.defender == b) || (w.attacker == b && w.defender == a)));
        self.wars.push(War { attacker: a, defender: b, cb: name, goal_duchy: duchy, subjugate: subj, freedom, claimant, start: y, score: 0.0, avenge: NONE });
        let r = self.realms[a].ruler;
        let avenge = if self.avenge_goal.0 == a && self.avenge_goal.1 == b { self.avenge_goal.2 } else { NONE };
        self.avenge_goal = (NONE, NONE, NONE);
        if let Some(w) = self.wars.last_mut() { w.avenge = avenge; }
        self.reluct(r, "war", "going to war");
        let key = (b, txt.clone());
        let joined = self.recent_decl.iter().any(|d| d.0 == key && y - d.1 <= 15);
        self.recent_decl.retain(|d| d.0 != key && y - d.1 <= 15);
        self.recent_decl.push((key, y));
        let text = if joined { format!("{} of {} joins the fight against {}{txt}.", self.who(r), self.realms[a].name, self.realms[b].name) }
                   else { format!("**War.** {} of {} declares war on {}{txt}.", self.who(r), self.realms[a].name, self.realms[b].name) };
        self.say(text, joined);
        self.raise_for_war(a, b);
        for t in 0..self.realms.len() {
            if self.realms[t].overlord == a && !self.realms[t].extinct && t != b {
                self.war[t][b] = true; self.war[b][t] = true;
                self.war_since[t][b] = y; self.war_since[b][t] = y;
            }
        }
    }

    fn indemnity(&self, k: usize) -> f64 {
        let r = &self.realms[k];
        (0.5 * r.gold).max(1.5 * r.income * r.tax_now * r.collect).max(0.0)
    }

    fn battle_result(&mut self, winner: usize, loser: usize, captured: usize) {
        let Some(w) = self.wars.iter_mut().find(|w| (w.attacker == winner && w.defender == loser) || (w.attacker == loser && w.defender == winner)) else { return };
        let sign = if winner == w.attacker { 1.0 } else { -1.0 };
        let mut pts = 8.0;
        if captured != NONE { pts += if self.towns[captured].duchy == w.goal_duchy && w.goal_duchy.is_some() { 25.0 } else { 10.0 }; }
        w.score += sign * pts;
    }

    fn wars_year(&mut self) {
        let y = self.year;
        let mut i = 0;
        while i < self.wars.len() {
            let (a, b) = (self.wars[i].attacker, self.wars[i].defender);
            if !self.war[a][b] { self.wars.remove(i); continue; }
            if self.realms[a].extinct || self.realms[b].extinct { self.end_war(i, "dissolved"); continue; }
            let dk = self.wars[i].goal_duchy;
            let mut score = self.wars[i].score;
            if dk.is_some_and(|d| self.duchies.get(&d).is_some_and(|x| x.ctrl == a)) { score += 10.0; }
            for t in self.alive_towns_of(b) { if self.towns[t].occupied_by == a { score += if self.towns[t].duchy == dk && dk.is_some() { 15.0 } else { 5.0 }; } }
            score -= 5.0 * self.alive_towns_of(a).iter().filter(|&&t| self.towns[t].occupied_by == b).count() as f64;
            let (pa, pb) = (self.realms[a].army_power, self.realms[b].army_power);
            score += 15.0 * (pa - pb) / (pa + pb).max(1.0) + self.rng.normal(0.0, 4.0);
            let start = self.wars[i].start;
            if y - start > 10 { score -= 2.0; }
            self.wars[i].score = score;
            if score >= 100.0 { self.end_war(i, "attacker"); }
            else if score <= -100.0 { self.end_war(i, "defender"); }
            else if y - start > 25 && score.abs() < 40.0 { self.end_war(i, "white"); }
            else { i += 1; }
        }
    }

    fn end_war(&mut self, i: usize, outcome: &'static str) {
        let w = self.wars.remove(i);
        let (a, b) = (w.attacker, w.defender);
        let y = self.year;
        self.war[a][b] = false; self.war[b][a] = false;
        self.truce[a][b] = y + 15; self.truce[b][a] = y + 15;
        for t in 0..self.towns.len() {
            let s = &mut self.towns[t];
            if s.alive && (s.occupied_by == a || s.occupied_by == b) && (s.realm == a || s.realm == b) { s.occupied_by = NONE; }
        }
        self.relation[a][b] = -20.0; self.relation[b][a] = -20.0;
        let (ra, rb) = (self.realms[a].ruler, self.realms[b].ruler);
        if ra != NONE && rb != NONE {
            let lw = match outcome { "attacker" => Some((rb, ra)), "defender" => Some((ra, rb)), _ => None };
            if let Some((l, wn)) = lw { self.remember(l, wn, "defeated me in war", -40.0, 0.5); self.reluct(l, "defeat", "a lost war"); }
        }
        let yrs = y - w.start;
        let (an, bn) = (self.realms[a].name.clone(), self.realms[b].name.clone());
        let result: String;
        if outcome == "attacker" {
            let mut what: String = if w.goal_duchy.is_none() && w.avenge != NONE { "forces its terms".into() } else if let Some(dk) = w.goal_duchy.filter(|d| self.duchies.contains_key(d)) {
                let towns: Vec<usize> = self.alive_towns_of(b).into_iter().filter(|&t| self.towns[t].duchy == Some(dk)).collect();
                if !towns.is_empty() {
                    for &t in &towns { self.lose_lordship(t); self.towns[t].realm = a; self.towns[t].capital = false; }
                    format!("takes {} ({} towns)", self.duchies[&dk].name, towns.len())
                } else {
                    let pay = self.indemnity(b);
                    self.realms[b].gold -= pay; self.realms[a].gold += pay;
                    format!("finds {} already lost to another, and takes {} gold in tribute instead", self.duchies[&dk].name, crate::economy::thousands(pay))
                }
            } else if w.subjugate {
                self.realms[b].overlord = a;
                format!("makes {bn} its tributary")
            } else if w.freedom {
                self.realms[a].overlord = NONE;
                "wins its freedom".into()
            } else {
                let mut claimant = w.claimant;
                if claimant != NONE && self.chars[claimant].ruled.is_some_and(|r| r.2.is_none()) {
                    let kids: Vec<usize> = self.chars[claimant].children.iter().copied().filter(|&x| self.chars[x].alive && self.chars[x].adult(y)
                        && !self.chars[x].ruled.is_some_and(|r| r.2.is_none())).collect();
                    claimant = kids.into_iter().min_by_key(|&x| self.chars[x].born).unwrap_or(NONE);
                }
                if claimant != NONE && self.chars[claimant].alive && rb != NONE {
                    self.kill_by(rb, format!("was deposed by {an} and did not survive it"), true, ra);
                    self.end_reign(b, &format!("deposed by {an}"));
                    self.chars[claimant].realm = b;
                    self.crown(b, claimant, "raised up when the line failed");
                    self.realms[b].overlord = a;
                    format!("puts {} on the throne of {bn}, a puppet now", self.chars[claimant].name)
                } else { "wins, but has no one to crown".into() }
            };
            let av = w.avenge;
            if av != NONE && self.chars[av].alive && ra != NONE && self.chars[ra].alive {
                let whom = self.chars[ra].vendettas.iter().find(|x| x.0 == av).map(|x| x.2.clone()).unwrap_or("their beloved".into());
                let why = format!("was executed in vengeance by {}", self.chars[ra].name);
                self.kill_by(av, why, false, ra);
                self.avenged(ra, av, &whom);
                what += &format!(", and {} pays with their life", self.chars[av].name);
            }
            self.say(format!("**{an} wins its {}** against {bn} after {yrs} years and {what}.", w.cb), false);
            if ra != NONE && self.chars[ra].alive {
                self.honour(ra, 100.0 + 10.0 * yrs as f64, "war");
                if self.chars[ra].rival == rb && rb != NONE {
                    self.honour(ra, 300.0, "rival");
                    let text = format!("{} has humbled their rival, {}.", self.who(ra), self.chars[rb].name);
                    self.say(text, false);
                }
            }
            result = what;
        } else if outcome == "defender" {
            if ra != NONE { let l = self.chars[ra].legitimacy.unwrap_or(50.0); self.chars[ra].legitimacy = Some((l - 10.0).max(0.0)); }
            let pay = self.indemnity(a);
            self.realms[a].gold -= pay; self.realms[b].gold += pay;
            if rb != NONE && self.chars[rb].alive { self.honour(rb, 80.0 + 5.0 * yrs as f64, "war"); }
            if w.freedom { self.realms[a].overlord = b; }
            self.say(format!("{bn} throws back the {} of {an} after {yrs} years; {an} pays {} gold.", w.cb, crate::economy::thousands(pay)), false);
            result = format!("defender won; {} gold paid", crate::economy::thousands(pay));
        } else if outcome == "white" {
            self.say(format!("The {} between {an} and {bn} ends in a white peace after {yrs} years.", w.cb), true);
            result = "white peace".into();
        } else { result = "dissolved".into(); }
        self.war_log.push((y, an, bn, result, yrs, outcome));
    }

    fn ambitions_year(&mut self) {
        let y = self.year;
        for k in 0..self.realms.len() {
            if !self.active(k) || self.realms[k].overlord != NONE { continue; }
            let r = self.realms[k].ruler;
            let v = self.vengeance_target(r);
            let urge = if v.is_some() { 0.45 } else { 0.08 * if self.chars[r].happy < -30.0 && self.chars[r].has_trait("wrathful") { 1.3 } else { 1.0 } };
            if self.war[k].iter().filter(|&&w| w).count() >= 2 || self.rng.random() > urge { continue; }
            if !self.chars[r].adult(y) { continue; }
            for o in 0..self.realms.len() {
                if o == k || !self.active(o) || self.war[k][o] || self.realms[k].allies.contains(&o) { continue; }
                if y < self.truce[k][o] || self.realms[o].overlord == k { continue; }
                let avenging = v.as_ref().is_some_and(|x| self.chars[x.0].realm == o);
                if self.realms[k].gold < 0.0 && !avenging { continue; }
                if !self.contact(k, o) { continue; }
                let or = self.realms[o].ruler;
                if self.opinion(r, or) > 30.0 && self.rng.random() < 0.7 { continue; }
                if !avenging && self.realms[k].army_power < 0.8 * self.realms[o].army_power { continue; }
                if !avenging && self.chars[r].loves.iter().any(|l| self.chars[l.0].alive && self.chars[l.0].realm == o) { continue; }
                let cb = self.cb_against(k, o);
                let Some(cb) = cb else { continue };
                self.start_war(k, o, cb, false);
                break;
            }
        }
    }

    fn tributaries_year(&mut self) {
        for k in 0..self.realms.len() {
            let ov = self.realms[k].overlord;
            if ov == NONE { continue; }
            if self.realms[k].extinct || self.realms[ov].extinct { self.realms[k].overlord = NONE; continue; }
            let tribute = self.realms[k].gold.max(0.0) * 0.15;
            self.realms[k].gold -= tribute;
            self.realms[ov].gold += tribute;
            if !self.realms[k].allies.contains(&ov) { self.realms[k].allies.push(ov); }
            if !self.realms[ov].allies.contains(&k) { self.realms[ov].allies.push(k); }
            if self.realms[k].army_power > 0.9 * self.realms[ov].army_power && self.rng.random() < 0.05 && self.active(k) {
                self.realms[k].overlord = NONE;
                self.start_war(k, ov, ("independence".into(), None, false, NONE), true);
                let text = format!("{} refuses tribute and fights to be free of {}.", self.realms[k].name, self.realms[ov].name);
                self.say(text, false);
            }
        }
    }

    fn rivals_year(&mut self) {
        for k in 0..self.realms.len() {
            if !self.active(k) { continue; }
            let r = self.realms[k].ruler;
            let riv = self.chars[r].rival;
            if riv != NONE && self.chars[riv].alive && self.chars[riv].ruled.is_some_and(|x| x.2.is_none()) { continue; }
            let cands: Vec<usize> = (0..self.realms.len()).filter(|&o| o != k && self.active(o) && self.realms[o].rank == self.realms[k].rank && self.contact(k, o)).collect();
            if let Some(&o) = cands.iter().max_by_key(|&&o| self.alive_towns_of(o).len()) {
                let orr = self.realms[o].ruler;
                self.chars[r].rival = orr;
                if self.chars[orr].rival == NONE { self.chars[orr].rival = r; }
            }
        }
    }

    fn great_decisions(&mut self) {
        let y = self.year;
        let total = self.towns.iter().filter(|t| t.alive).count();
        let mut by_land: std::collections::BTreeMap<i32, Vec<usize>> = Default::default();
        for t in self.towns.iter().filter(|t| t.alive) { by_land.entry(t.landmass).or_default().push(t.id); }
        for k in 0..self.realms.len() {
            if !self.active(k) { continue; }
            let r = self.realms[k].ruler;
            let subs: Vec<usize> = (0..self.realms.len()).filter(|&o| self.realms[o].overlord == k && !self.realms[o].extinct).collect();
            for (&lid, ts) in &by_land {
                if lid < 0 || self.map.lands[lid as usize].kind != "continent" || ts.len() < 40 || self.realms[k].done.contains(&("unite", lid)) { continue; }
                if self.realms[k].people == 3 && self.chars[r].renown < 800.0 { continue; }
                let held = ts.iter().filter(|&&t| self.towns[t].realm == k || subs.contains(&self.towns[t].realm)).count();
                if held as f64 >= 0.75 * ts.len() as f64 {
                    self.realms[k].done.push(("unite", lid));
                    let place = self.land_name(lid);
                    self.realms[k].great.push(place.clone());
                    self.realms[k].rank = "empire";
                    self.realms[k].name = format!("the Great Empire of {place}");
                    self.honour(r, 2000.0, "unification");
                    let text = format!("**{} unites {place}.** From shore to shore it answers to one throne: {}.", self.who(r), self.realms[k].name);
                    self.say(text, false);
                }
            }
            let p = self.realms[k].people;
            let ever = self.realms.iter().filter(|o| o.people == p).count();
            if !self.realms[k].done.contains(&("people", -1)) && ever >= 4
                && (0..self.realms.len()).filter(|&o| self.realms[o].people == p && !self.realms[o].extinct).all(|o| o == k || subs.contains(&o)) {
                self.realms[k].done.push(("people", -1));
                self.honour(r, 800.0, "unification");
                self.chars[r].titles.push(format!("High Sovereign of the {} from year {y}", PEOPLES[p]));
                let text = format!("**{} is High Sovereign of all the {}**: every realm of their people kneels or pays tribute.", self.who(r), PEOPLES[p]);
                self.say(text, false);
            }
            let mine = self.alive_towns_of(k).len() + subs.iter().map(|&o| self.alive_towns_of(o).len()).sum::<usize>();
            if !self.realms[k].done.contains(&("hegemony", -1)) && total > 200 && mine as f64 >= 0.35 * total as f64 {
                self.realms[k].done.push(("hegemony", -1));
                self.realms[k].rank = "hegemony";
                let place = if self.realms[k].place.is_empty() { self.realms[k].short.clone() } else { self.realms[k].place.clone() };
                self.realms[k].name = format!("the Hegemony of {place}");
                self.honour(r, 4000.0, "hegemony");
                let text = format!("**A hegemony.** {}: a third of the world's towns answer to {}.", self.realms[k].name, self.who(r));
                self.say(text, false);
            }
        }
    }

    pub fn statecraft_year(&mut self) {
        self.last_stands();
        self.domain_year();
        self.strong_eat_weak();
        self.overstretch();
        self.limits_year();
        self.rivals_year();
        self.tributaries_year();
        self.ambitions_year();
        self.wars_year();
        self.great_decisions();
    }

    // ------------------------------------------------------------ armies
    fn army_power(&self, a: &Army) -> f64 {
        let lead = if a.commander != NONE && self.chars[a.commander].alive { 1.0 + self.chars[a.commander].attr(1, self.year) / 40.0 } else { 1.0 };
        a.men * a.quality * lead * a.morale
    }

    fn raise_army(&mut self, k: usize) -> Option<usize> {
        if self.armies.iter().any(|a| a.realm == k && !a.rebel && a.men > 0.0) { return None; }
        let cap = self.capital_of(k);
        if cap == NONE { return None; }
        let realm = &self.realms[k];
        let maa_men: f64 = realm.maa.iter().map(|&(_, n)| n as f64 * 300.0).sum();
        let maa_q: f64 = realm.maa.iter().map(|&(m, n)| MAA[m].1 * n as f64 * 300.0).sum();
        let levy = realm.levy * 0.6;
        let men = levy + maa_men;
        if men < 50.0 { return None; }
        let r = realm.ruler;
        let mar = realm.council[0];
        let cmd = if r != NONE && self.chars[r].has_trait("brave") && self.chars[r].adult(self.year) { r } else if mar != NONE { mar } else { r };
        let id = self.next_army;
        self.next_army += 1;
        self.armies.push(Army { id, realm: k, pos: self.towns[cap].pos, men, quality: (levy + maa_q) / men.max(1.0), commander: cmd, morale: 1.0,
                                siege: 0.0, rebel: false, rebel_of: NONE, home: NONE, name: format!("the host of {}", realm.name), why: "", done: false });
        self.realms[k].gold -= men * 0.004;
        Some(id)
    }

    fn raise_for_war(&mut self, a: usize, b: usize) {
        for f in [a, b] {
            self.raise_army(f);
            for t in 0..self.realms.len() { if self.realms[t].overlord == f && !self.realms[t].extinct { self.raise_army(t); } }
        }
    }

    /// A peasant rising takes the field (warfare.raise_rebels).
    pub fn raise_rebels(&mut self, k: usize, town: usize, leader: usize, why: &'static str) {
        let id = self.next_army;
        self.next_army += 1;
        let men = self.towns[town].pop * 0.12 + self.towns[town].rallied;
        self.towns[town].rallied = 0.0;
        self.armies.push(Army { id, realm: NONE, pos: self.towns[town].pos, men, quality: 0.8, commander: leader, morale: 1.0, siege: 0.0,
                                rebel: true, rebel_of: k, home: town, name: format!("the {} rising", self.towns[town].name), why, done: false });
        self.raise_army(k);
    }

    fn enemies(&self, a: &Army, b: &Army) -> bool {
        if a.rebel { return b.realm == a.rebel_of; }
        if b.rebel { return a.realm == b.rebel_of; }
        a.realm != NONE && b.realm != NONE && a.realm != b.realm && self.war[a.realm][b.realm]
    }

    fn step_toward(&self, pos: (usize, usize), target: (usize, usize), speed: usize) -> (usize, usize) {
        let (mut r, mut c) = (pos.0 as i64, pos.1 as i64);
        let mut speed = speed as i64;
        let mut i = 0;
        while i < speed {
            i += 1;
            let (dy, dx) = (target.0 as i64 - r, target.1 as i64 - c);
            if dy.abs() + dx.abs() <= 1 { break; }
            let (sy, sx) = (dy.signum(), dx.signum());
            let mut next = None;
            for (my, mx) in [(sy, sx), (sy, 0), (0, sx), (sy, -sx), (-sy, sx)] {
                let (ny, nx) = (r + my, c + mx);
                if (my != 0 || mx != 0) && ny >= 0 && ny < self.map.h as i64 && nx >= 0 && nx < self.map.w as i64 && self.map.terrain.at(ny as usize, nx as usize) != OCEAN {
                    next = Some((ny, nx));
                    break;
                }
            }
            let Some((ny, nx)) = next else { break };
            r = ny; c = nx;
            if self.map.terrain.at(r as usize, c as usize) == MOUNTAIN { speed -= 1; }
        }
        (r as usize, c as usize)
    }

    /// (kind, index): 0 = an army, 1 = a town.
    fn pick_target(&self, ai: usize) -> Option<(u8, usize)> {
        let a = &self.armies[ai];
        let lid = self.map.landmass.at(a.pos.0, a.pos.1);
        if a.rebel {
            let seats: Vec<usize> = self.alive_towns_of(a.rebel_of).into_iter().filter(|&t| self.towns[t].landmass == lid
                && (self.towns[t].capital || self.towns[t].lord != NONE) && t != a.home).collect();
            return seats.into_iter().min_by(|&x, &y| dist(self.towns[x].pos, a.pos).partial_cmp(&dist(self.towns[y].pos, a.pos)).unwrap()).map(|t| (1, t));
        }
        let near = (0..self.armies.len()).filter(|&j| j != ai && self.armies[j].men > 0.0 && self.enemies(a, &self.armies[j])
            && self.map.landmass.at(self.armies[j].pos.0, self.armies[j].pos.1) == lid && dist(self.armies[j].pos, a.pos) < 30.0)
            .min_by(|&x, &y| dist(self.armies[x].pos, a.pos).partial_cmp(&dist(self.armies[y].pos, a.pos)).unwrap());
        if let Some(j) = near { return Some((0, j)); }
        let goal = self.wars.iter().find(|w| w.attacker == a.realm || w.defender == a.realm).and_then(|w| if w.attacker == a.realm { w.goal_duchy } else { None });
        let mut towns: Vec<usize> = vec![];
        for f in 0..self.realms.len() {
            if a.realm != NONE && self.war[a.realm][f] {
                towns.extend(self.alive_towns_of(f).into_iter().filter(|&t| self.towns[t].landmass == lid && self.towns[t].occupied_by == NONE));
            }
        }
        if towns.is_empty() { return None; }
        if let Some(g) = goal {
            let gt: Vec<usize> = towns.iter().copied().filter(|&t| self.towns[t].duchy == Some(g)).collect();
            if !gt.is_empty() { towns = gt; }
        }
        towns.into_iter().min_by(|&x, &y| dist(self.towns[x].pos, a.pos).partial_cmp(&dist(self.towns[y].pos, a.pos)).unwrap()).map(|t| (1, t))
    }

    fn field_battle(&mut self, ai: usize, bi: usize) {
        let terr = match self.map.terrain.at(self.armies[bi].pos.0, self.armies[bi].pos.1) { MOUNTAIN => 1.3, HILLS => 1.2, FOREST => 1.15, _ => 1.0 };
        let pa = self.army_power(&self.armies[ai]);
        let pb = self.army_power(&self.armies[bi]) * terr;
        let ra = pa * self.rng.uniform(0.7, 1.3);
        let rb = pb * self.rng.uniform(0.7, 1.3);
        let (win, lose) = if ra > rb { (ai, bi) } else { (bi, ai) };
        let frac = ra.max(rb) / (ra + rb);
        let lost_l = self.armies[lose].men * self.rng.uniform(0.25, 0.45) * frac;
        let lost_w = self.armies[win].men * self.rng.uniform(0.06, 0.16) * (1.0 - frac) * 2.0;
        self.armies[lose].men -= lost_l;
        self.armies[win].men -= lost_w;
        self.armies[lose].morale *= 0.7;
        self.armies[win].morale = (self.armies[win].morale * 1.05).min(1.2);
        let lr = self.armies[lose].realm;
        if lr != NONE { let home = self.capital_of(lr); if home != NONE { let p = self.step_toward(self.armies[lose].pos, self.towns[home].pos, SPEED); self.armies[lose].pos = p; } }
        let apos = self.armies[ai].pos;
        let where_ = self.towns.iter().filter(|t| t.alive).min_by(|x, y| dist(x.pos, apos).partial_cmp(&dist(y.pos, apos)).unwrap()).map(|t| t.name.clone()).unwrap_or("the field".into());
        let total = lost_l + lost_w;
        self.battles_fought.push((self.year, where_.clone(), total as i64));
        let big = total > 3000.0;
        let (wn, ln) = (self.armies[win].name.clone(), self.armies[lose].name.clone());
        let text = format!("{}The Battle of {where_}{} {} ({} men) breaks {ln} ({}); {} fall.", if big { "**" } else { "" }, if big { ".**" } else { "." }, cap1(&wn),
                           crate::economy::thousands(self.armies[win].men + lost_w), crate::economy::thousands(self.armies[lose].men + lost_l), crate::economy::thousands(total));
        self.say(text, !big);
        let (cl, cw) = (self.armies[lose].commander, self.armies[win].commander);
        if cl != NONE && self.chars[cl].alive && self.rng.random() < 0.08 { self.kill_by(cl, format!("fell at the Battle of {where_}"), false, cw); }
        if cw != NONE && self.chars[cw].alive { self.honour(cw, 20.0 + total / 400.0, "war"); }
        for (side, lost) in [(lose, lost_l), (win, lost_w)] {
            let r = self.armies[side].realm;
            if r == NONE { continue; }
            let towns = self.alive_towns_of(r);
            let tot = { let t: f64 = towns.iter().map(|&t| self.towns[t].pop).sum(); if t == 0.0 { 1.0 } else { t } };
            for t in towns { let p = self.towns[t].pop; self.towns[t].pop -= lost * p / tot * 0.7; }
        }
        if self.armies[win].rebel && self.armies[lose].men < 0.5 * (self.armies[lose].men + lost_l) {
            let home = self.armies[win].home;
            if home != NONE && self.towns[home].alive { self.rebels_win(win, home); return; }
        }
        if !self.armies[win].rebel && !self.armies[lose].rebel && self.armies[win].realm != NONE {
            let (wr, lr) = (self.armies[win].realm, self.armies[lose].realm);
            self.battle_result(wr, lr, NONE);
            self.battle_result(wr, lr, NONE);
        }
    }

    fn siege(&mut self, ai: usize, t: usize) {
        let k = self.armies[ai].realm;
        let mages = if k != NONE { self.realms[k].maa.iter().filter(|x| MAA[x.0].0 == "War Mages").map(|x| x.1).sum::<u32>() as f64 } else { 0.0 };
        let hold = self.towns[t].garrison * (1.0 + 0.4 * self.towns[t].walls as f64) + 30.0;
        let p = self.army_power(&self.armies[ai]);
        self.armies[ai].siege += 25.0 * p / (hold * 10.0) * (1.0 + 0.5 * mages);
        if self.armies[ai].siege < 100.0 { return; }
        self.armies[ai].siege = 0.0;
        if self.armies[ai].rebel { self.rebels_win(ai, t); return; }
        let old = self.towns[t].realm;
        self.towns[t].occupied_by = k;
        let loot = self.towns[t].wealth * 0.4;
        self.towns[t].wealth -= loot;
        self.towns[t].pop *= 0.92;
        if k != NONE { self.realms[k].gold += loot; }
        self.battle_result(k, old, t);
        self.sieges_won += 1;
        let text = format!("{} takes {} after a siege.", cap1(&self.armies[ai].name), self.towns[t].name);
        let minor = !self.towns[t].capital;
        self.say(text, minor);
    }

    fn rebels_win(&mut self, ai: usize, t: usize) {
        let k = self.armies[ai].rebel_of;
        let r = self.realms[k].ruler;
        self.realms[k].tax_relief = self.year + 20;
        self.armies[ai].men = 0.0;
        let who = if r != NONE { self.who(r) } else { self.realms[k].name.clone() };
        let text = format!("**The rising wins.** {} takes {}; {who} must grant what the people demanded ({}).", cap1(&self.armies[ai].name), self.towns[t].name, self.armies[ai].why);
        self.say(text, false);
        if r != NONE { let l = self.chars[r].legitimacy.unwrap_or(50.0); self.chars[r].legitimacy = Some((l - 10.0).max(0.0)); }
        let home = self.armies[ai].home;
        if home != NONE && self.towns[home].alive && self.towns[home].realm == k && self.towns[home].pop > 800.0 && self.rng.random() < 0.4 {
            let leader = self.armies[ai].commander;
            self.free_commune(k, home, leader);
        }
    }

    fn free_commune(&mut self, k: usize, town: usize, leader: usize) {
        let name = format!("the Free Commune of {}", self.towns[town].name);
        let short = format!("commoners of {}", self.towns[town].name);
        let place = self.towns[town].name.clone();
        let nid = self.new_realm(self.towns[town].culture, name, short, "duchy", "republic", [230, 230, 230], place);
        self.realms[nid].law = "elective";
        self.realms[nid].faith = self.realms[k].faith;
        self.realms[nid].parent = k;
        self.realms[nid].home_land = self.towns[town].landmass;
        self.realms[nid].known = self.realms[k].known.clone();
        self.towns[town].realm = nid;
        self.towns[town].capital = true;
        if leader != NONE && self.chars[leader].alive { self.chars[leader].realm = nid; self.crown(nid, leader, "raised up when the line failed"); }
        self.truce[k][nid] = self.year + 15; self.truce[nid][k] = self.year + 15;
        self.breakaways.push((self.year, nid, k, leader));
        let text = format!("**A free commune.** {} throws off {} and governs itself.", self.towns[town].name, self.realms[k].name);
        self.say(text, false);
    }

    /// Four seasons of marching, fighting and besieging (warfare.armies_year).
    pub fn armies_year(&mut self) {
        for _ in 0..TICKS {
            for ai in 0..self.armies.len() {
                if self.armies[ai].men < 30.0 { continue; }
                let a = &self.armies[ai];
                if !a.rebel && (a.realm == NONE || !self.at_war(a.realm)) && !self.armies.iter().any(|b| b.rebel && b.rebel_of == a.realm && b.men > 0.0) {
                    self.armies[ai].men = 0.0;
                    continue;
                }
                if a.rebel && self.realms[a.rebel_of].extinct { self.armies[ai].men = 0.0; continue; }
                let Some((kind, obj)) = self.pick_target(ai) else { continue };
                let dest = if kind == 0 { self.armies[obj].pos } else { self.towns[obj].pos };
                if dist(dest, self.armies[ai].pos) > 2.0 { let p = self.step_toward(self.armies[ai].pos, dest, SPEED); self.armies[ai].pos = p; }
                let pos = self.armies[ai].pos;
                let o = self.owner.at(pos.0, pos.1);
                let hostile = o >= 0 && self.armies[ai].realm != NONE && self.war[self.armies[ai].realm][self.towns[o as usize].realm];
                let harvest = self.climate.at(&self.climate.harvest, pos);
                let att = 0.006 + 0.01 * hostile as u8 as f64 + 0.01 * (1.0 - harvest).max(0.0) + 0.004 * (self.armies[ai].men / 20000.0 - 1.0).max(0.0);
                self.armies[ai].men *= 1.0 - att;
                if kind == 0 && self.armies[obj].men > 0.0 && dist(self.armies[obj].pos, pos) <= 3.0 { self.field_battle(ai, obj); }
                else if kind == 1 && dist(dest, pos) <= 2.0 && self.towns[obj].alive { self.siege(ai, obj); }
            }
        }
        for ai in 0..self.armies.len() {
            let a = &self.armies[ai];
            if a.rebel && a.men < 30.0 && !a.done {
                let (k, home, c) = (a.rebel_of, a.home, a.commander);
                self.armies[ai].done = true;
                if home != NONE { self.towns[home].rising = None; }
                let r = self.realms[k].ruler;
                if c != NONE && self.chars[c].alive && r != NONE {
                    let why = format!("was hanged when {} was crushed", self.armies[ai].name);
                    self.kill_by(c, why, false, r);
                }
            }
        }
        self.armies.retain(|a| a.men >= 30.0);
    }

    // ------------------------------------------------------------ borders, diplomacy, frontier battles
    /// Touching realms and town pairs on the frontier; the towns trade across peaceful borders.
    pub fn borders_and_diplomacy_with(&mut self, _trade: bool) -> Vec<(usize, usize)> {
        let n = self.realms.len();
        for t in self.towns.iter_mut() { t.trade = 0.0; }
        let (h, w) = (self.map.h, self.map.w);
        let mut pf = vec![vec![0.0f64; n]; n];
        let mut pairs: std::collections::BTreeSet<(usize, usize)> = Default::default();
        let realm_of = |o: i64| if o < 0 { NONE } else { self.towns[o as usize].realm };
        for r in 0..h {
            for c in 0..w {
                let a = self.owner.at(r, c);
                for (rr, cc) in [(r, c + 1), (r + 1, c)] {
                    if rr >= h || cc >= w { continue; }
                    let b = self.owner.at(rr, cc);
                    let (fa, fb) = (realm_of(a), realm_of(b));
                    if fa != NONE && fb != NONE && fa != fb {
                        pf[fa][fb] += 1.0; pf[fb][fa] += 1.0;
                        pairs.insert(((a.min(b)) as usize, (a.max(b)) as usize));
                    }
                }
            }
        }
        // (as world.diplomacy, which pairs every realm only with realms 0-3: see PORT.md "Python bugs")
        for i in 0..n {
            for j in (i + 1)..4.min(n) {
                if self.realms[i].extinct || self.realms[j].extinct { continue; }
                let (pi, pj) = (self.realms[i].people, self.realms[j].people);
                let (ri, rj) = (self.realms[i].ruler, self.realms[j].ruler);
                let mut rel = self.relation[i][j];
                rel += 0.08 * ((BASE_REL[pi][pj] - if pi == pj { 20.0 } else { 0.0 }) - rel) + self.rng.normal(0.0, 3.0);
                if ri != NONE && rj != NONE { rel += (self.chars[ri].attr(0, self.year) + self.chars[rj].attr(0, self.year) - 16.0) * 0.05; }
                let allied = self.realms[i].allies.contains(&j);
                if allied { rel += 2.0; }
                let contact = pf[i][j] > 0.0 || self.naval_contact(i, j);
                if pf[i][j] > 0.0 { rel -= (0.15 * pf[i][j]).min(8.0) * (AGGRESSION[pi] + AGGRESSION[pj]) / 2.0; } else if contact { rel -= 1.0; }
                if self.war[i][j] { rel -= 1.0; }
                let mut rel = rel.clamp(-100.0, 100.0);
                self.relation[i][j] = rel; self.relation[j][i] = rel;
                if !self.war[i][j] {
                    let tribal = matches!(self.realms[i].rank, "tribe" | "chiefdom") || matches!(self.realms[j].rank, "tribe" | "chiefdom");
                    let p = 0.15 * (AGGRESSION[pi] * self.realms[i].aggr).max(AGGRESSION[pj] * self.realms[j].aggr) * if tribal { 2.0 } else { 1.0 }
                        * self.tenet_aggression(i).max(self.tenet_aggression(j));
                    if self.realms[i].faith != self.realms[j].faith { rel -= 1.0; }
                    if contact && !allied && self.year >= self.truce[i][j] && rel < if tribal { -20.0 } else { -45.0 } && self.rng.random() < p
                        && ri != NONE && rj != NONE {
                        let (x, y) = (self.rng.random(), self.rng.random());
                        let (a, b) = if AGGRESSION[pi] * self.realms[i].aggr * x > AGGRESSION[pj] * self.realms[j].aggr * y { (i, j) } else { (j, i) };
                        let cb = self.cb_against(a, b);
                        let cb = cb.or_else(|| self.nearest_duchy(a, b).map(|dk| ("border war".to_string(), Some(dk), false, NONE)));
                        if let Some(cb) = cb { self.start_war(a, b, cb, false); }
                    }
                } else {
                    let wsum = self.weariness[i][j] + self.weariness[j][i];
                    let long = self.year - self.war_since[i][j];
                    let declared = self.wars.iter().any(|w| (w.attacker == i && w.defender == j) || (w.attacker == j && w.defender == i));
                    let x = self.rng.random();
                    let end = !declared && (wsum > 0.25 + x * 0.3 || long > 40 || !contact) && self.rng.random() < 0.35;
                    if end {
                        self.war[i][j] = false; self.war[j][i] = false;
                        self.relation[i][j] = -15.0; self.relation[j][i] = -15.0;
                        let t = self.year + 15 + self.rng.integers(0, 20);
                        self.truce[i][j] = t; self.truce[j][i] = t;
                        let towns: Vec<usize> = self.alive_towns_of(i).into_iter().chain(self.alive_towns_of(j)).collect();
                        let place = if towns.is_empty() { "the border".into() } else { self.towns[towns[self.rng.integers(0, towns.len() as i64) as usize]].name.clone() };
                        let text = format!("**Peace.** The Peace of {place} ends {long} years of war between the {} and the {}.", self.pname(i), self.pname(j));
                        self.say(text, false);
                    }
                }
            }
        }
        for &(a, b) in &pairs {
            if self.towns[a].alive && self.towns[b].alive && !self.war[self.towns[a].realm][self.towns[b].realm] {
                self.towns[a].trade += 1.0; self.towns[b].trade += 1.0;
                self.towns[a].wealth += 2.0; self.towns[b].wealth += 2.0;
            }
        }
        pairs.into_iter().collect()
    }

    /// Frontier battles between towns of realms at war, with the heroes' strength behind each side (world.battles).
    pub fn frontier_year(&mut self, mut pairs: Vec<(usize, usize)>, war_power: Vec<f64>) -> Vec<u32> {
        let mut won = vec![0u32; self.realms.len()];
        let n = self.realms.len();
        let mut fpop = vec![0.0; n];
        for t in self.towns.iter().filter(|t| t.alive) { fpop[t.realm] += t.pop; }
        let y = self.year;
        let hero_mult: Vec<f64> = (0..n).map(|f| 1.0 + (war_power.get(f).copied().unwrap_or(0.0) * self.realms[f].modv("war_power", y) / (fpop[f] * 0.1).max(1.0)).min(1.5)).collect();
        for i in (1..pairs.len()).rev() { let j = self.rng.integers(0, i as i64 + 1) as usize; pairs.swap(i, j); }
        let mut fought = std::collections::HashSet::new();
        for (a, b) in pairs {
            let (fa0, fd0) = (self.towns[a].realm, self.towns[b].realm);
            if !self.towns[a].alive || !self.towns[b].alive || fa0 == fd0 || !self.war[fa0][fd0] || fought.contains(&a) || fought.contains(&b) || self.rng.random() > 0.25 { continue; }
            fought.insert(a); fought.insert(b);
            let (pa_, pd_) = (self.realms[fa0].people, self.realms[fd0].people);
            let ea = AGGRESSION[pa_] * self.realms[fa0].aggr;
            let ed = AGGRESSION[pd_] * self.realms[fd0].aggr;
            let (at, de) = if self.rng.random() > ea / (ea + ed) { (b, a) } else { (a, b) };
            let (fa, fd) = (self.towns[at].realm, self.towns[de].realm);
            let (ca, cd) = (self.realms[fa].people, self.realms[fd].people);
            let (sa, sd) = (&self.towns[at], &self.towns[de]);
            let pa = sa.pop * 0.3 * MARTIAL[ca] * (1.0 + 0.5 * sa.magic) * hero_mult[fa] * self.realms[fa].battle_mult;
            let pd = sd.pop * 0.3 * MARTIAL[cd] * (1.0 + 0.5 * sd.magic) * (1.0 + 0.3 * sd.walls as f64) * DEFENSE[cd] * hero_mult[fd] * self.realms[fd].battle_mult + sd.guard + sd.garrison;
            let ra = pa * self.rng.uniform(0.6, 1.4);
            let rd = pd * self.rng.uniform(0.6, 1.4);
            let frac = ra / (ra + rd + 1e-9);
            let (la, ld) = (self.towns[at].pop * 0.12 * (1.0 - frac), self.towns[de].pop * 0.2 * frac);
            self.towns[at].pop -= la;
            self.towns[de].pop -= ld;
            self.weariness[fa][fd] += la / fpop[fa].max(1.0);
            self.weariness[fd][fa] += ld / fpop[fd].max(1.0);
            if ra <= rd { won[fd] += 1; }
            if ra > rd {
                won[fa] += 1;
                let weak = self.towns[de].pop < self.towns[at].pop * 0.7;
                self.battle_result(fa, fd, if weak { de } else { NONE });
                if weak && self.rng.random() < 0.08 {
                    let was_cap = self.towns[de].capital;
                    let ruler = self.realms[fa].ruler;
                    let cruel = ruler != NONE && self.chars[ruler].has_trait("cruel");
                    let (an, dn, kn) = (self.towns[at].name.clone(), self.towns[de].name.clone(), self.realms[fd].name.clone());
                    let seat = if was_cap { format!(", seat of {kn}") } else { String::new() };
                    if self.rng.random() < if ca == 3 { 0.4 } else { 0.05 } + if cruel { 0.2 } else { 0.0 } {
                        let text = format!("The {} of {an} sack and burn {dn}{seat}.", self.pname(fa));
                        self.say(text, !was_cap);
                        self.destroy(de);
                        self.realms[fd].lost += 1;
                        if cruel { self.add_tyranny(ruler, 4.0, &format!("the burning of {dn}")); }
                    } else {
                        let seat = if was_cap { format!(", seat of {kn},") } else { String::new() };
                        let text = format!("{dn}{seat} falls to the {} of {an}.", self.pname(fa));
                        self.say(text, !was_cap);
                        self.lose_lordship(de);
                        let s = &mut self.towns[de];
                        s.realm = fa; s.capital = false; s.pop *= 0.6; s.walls = s.walls.saturating_sub(1);
                        self.realms[fd].lost += 1;
                    }
                    self.realms[fa].taken += 1;
                }
            }
        }
        for f in 0..n {
            if self.realms[f].extinct || !self.at_war(f) { continue; }
            let r = self.realms[f].ruler;
            if r != NONE && self.chars[r].alive && self.chars[r].has_trait("brave") && !self.chars[r].undead && self.chars[r].adult(self.year) && self.rng.random() < 0.015 {
                let fid = (0..n).find(|&o| self.war[f][o]).unwrap();
                let foe = self.pname(fid);
                let by = self.realms[fid].ruler;
                self.kill_by(r, format!("fell leading the host against the {foe}"), false, by);
            }
        }
        won
    }

    pub fn destroy(&mut self, s: usize) {
        self.lose_lordship(s);
        self.towns[s].alive = false;
        let name = format!("the ruins of {}", self.towns[s].name);
        let riches = 1 + ((self.towns[s].wealth / 40.0) as i64).min(5);
        let depth = 1.0 + self.towns[s].walls as f64 * 0.4;
        self.ruins.push(Ruin::new(name, self.towns[s].pos, riches, depth));
    }

    /// Blight, abandoned towns, realms whose last town has fallen, seats lost (world.events, world.cleanup).
    pub fn events_and_blight(&mut self) {
        if self.blight > 0 { self.blight -= 1; }
        else if self.rng.random() < 0.008 {
            self.blight = self.rng.integers(1, 4);
            let text = format!("Blight. The harvests fail across the land for {} year{}.", self.blight, if self.blight > 1 { "s" } else { "" });
            self.say(text, false);
        }
    }

    pub fn cleanup(&mut self) {
        let gone: Vec<usize> = (0..self.towns.len()).filter(|&t| self.towns[t].alive && self.towns[t].pop < 15.0).collect();
        for &s in &gone { self.destroy(s); }
        if !gone.is_empty() {
            let names: Vec<String> = gone.iter().map(|&s| format!("{} ({})", self.towns[s].name, self.pname(self.towns[s].realm))).collect();
            let text = format!("{} town{} abandoned: {}.", gone.len(), if gone.len() > 1 { "s are" } else { " is" }, names.join(", "));
            self.say(text, true);
        }
        for f in 0..self.realms.len() {
            if self.realms[f].extinct { continue; }
            let towns = self.alive_towns_of(f);
            if towns.is_empty() {
                self.realms[f].extinct = true;
                for i in 0..self.realms.len() { self.war[f][i] = false; self.war[i][f] = false; }
                let p = self.realms[f].people;
                let still = self.towns.iter().any(|t| t.alive && t.culture == p);
                let text = format!("**{} is no more.** Its last town has fallen.{}", self.realms[f].name, if still { String::new() } else { format!(" The {} are gone from the world.", PEOPLES[p]) });
                self.say(text, false);
                let alive: Vec<usize> = (0..self.realms.len()).filter(|&k| !self.realms[k].extinct).collect();
                let same: Vec<usize> = alive.iter().copied().filter(|&k| self.realms[k].people == p).collect();
                let pool = if same.is_empty() { &alive } else { &same };
                let dest = pool.iter().copied().max_by_key(|&k| self.alive_towns_of(k).len()).unwrap_or(0);
                for c in 0..self.chars.len() {
                    if self.chars[c].alive && self.chars[c].realm == f {
                        let home = self.chars[c].home;
                        self.chars[c].realm = if home != NONE && self.towns[home].alive { self.towns[home].realm } else { dest };
                    }
                }
                continue;
            }
            if self.capital_of(f) == NONE {
                let s = *towns.iter().max_by(|&&a, &&b| self.towns[a].pop.partial_cmp(&self.towns[b].pop).unwrap()).unwrap();
                self.towns[s].capital = true;
                let text = format!("{} makes {} its new seat.", self.realms[f].name, self.towns[s].name);
                self.say(text, false);
            }
        }
    }
}
