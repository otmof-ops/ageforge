//! The four bending tribes and the Concord (bending.py): bending is a bloodline, common in four remote tribes and
//! rare elsewhere; benders climb four tiers and open a high art; the Concord is one soul reborn in a time of need,
//! a bender of each element in turn, who learns all four and turns on what the world needs it to face.

use crate::monsters::mdist;
use crate::people::*;
use crate::power::skill_ix;
use crate::state::State;
use crate::tables::*;

const TRIBE_CAP: usize = 7;

pub struct Tribe { pub element: usize, pub name: &'static str, pub pos: (usize, usize), pub members: Vec<usize>, pub chief: usize, pub chiefs: Vec<(i64, usize)> }
pub struct ConcordLife { pub id: usize, pub born: i64, pub element: usize, pub need: String, pub revealed: Option<i64>, pub died: Option<(i64, String)>, pub deeds: Vec<String> }

fn bend_tier(lv: f64) -> usize { ((lv / 25.0) as usize).min(3) }

impl State {
    pub fn found_tribes(&mut self) {
        let (h, w) = (self.map.h, self.map.w);
        for e in 0..4 {
            let terrain = self.map.terrain.clone();
            let owner = self.owner.clone();
            let spot = move |r: usize, c: usize| -> bool { match e {
                0 => terrain.at(r, c) != OCEAN && terrain.at((r + h - 1) % h, c) == OCEAN,
                1 | 3 => terrain.at(r, c) == MOUNTAIN, _ => terrain.at(r, c) == DESERT } };
            let mut cells = self.remote_cells(&|r, c| spot(r, c) && owner.at(r, c) == -1);
            if cells.is_empty() { for r in 0..h { for c in 0..w { if spot(r, c) { cells.push((r, c)); } } } }
            if cells.is_empty() { for r in 0..h { for c in 0..w { if self.map.terrain.at(r, c) != OCEAN { cells.push((r, c)); } } } }
            let pos = cells[self.rng.integers(0, cells.len() as i64) as usize];
            self.tribes.push(Tribe { element: e, name: TRIBES[e], pos, members: vec![], chief: NONE, chiefs: vec![] });
            for _ in 0..4 { let b = self.new_bender(e); self.join_tribe(e, b); }
        }
        let names = self.tribes.iter().map(|t| t.name).collect::<Vec<_>>().join("; ");
        self.say(format!("Far from the kingdoms live the four bending tribes: {names}. It is told that once in every age of need, one soul is born who can bend all four."), false);
    }

    fn new_bender(&mut self, e: usize) -> usize {
        let f = self.rng.weighted(&[0.55, 0.15, 0.15, 0.15]);
        let age = ADULT[f] as i64 + self.rng.integers(0, 12);
        let c = self.make_character(f, self.year - age, None, NONE, NONE, NONE);
        let g = gene(ELEMENT_GENE[e]);
        if !self.chars[c].has(g) { self.chars[c].genetic.push(g); }
        self.setup_bender(c, e);
        c
    }

    fn setup_bender(&mut self, c: usize, e: usize) {
        self.chars[c].element = Some(ELEMENTS[e]);
        let lv = self.rng.uniform(15.0, 35.0);
        self.chars[c].bend = vec![(e, lv)];
        self.chars[c].high_art = None;
    }

    fn join_tribe(&mut self, e: usize, c: usize) { self.tribes[e].members.push(c); self.chars[c].tribe = e; }

    pub fn bend_power(&self, c: &Character) -> f64 {
        let lb = self.legacy_bonus(c, "bend");
        let mut tot: f64 = c.bend.iter().map(|&(_, lv)| 25.0 * 1.9f64.powf((lv + lb).min(120.0) / 15.0)).sum();
        if c.high_art.is_some() { tot *= 1.3; }
        if c.concord && c.adult(self.year) { tot *= 2.0 + 0.5 * c.bend.len() as f64; }
        tot * (1.0 + c.level as f64 / 80.0)
    }

    /// On every birth: does the child carry a bending gift, and is it the Concord?
    pub fn born_bender(&mut self, c: usize) {
        let parents: Vec<usize> = [self.chars[c].father, self.chars[c].mother].into_iter().filter(|&p| p != NONE).collect();
        let mut gift = None;
        for &p in &parents { if let Some(el) = self.chars[p].element { if self.rng.random() < 0.5 { gift = Some(ELEMENTS.iter().position(|e| *e == el).unwrap()); } } }
        if gift.is_none() && self.rng.random() < 0.003 { gift = Some(self.rng.integers(0, 4) as usize); }
        let Some(e) = gift else { return };
        let g = gene(ELEMENT_GENE[e]);
        if !self.chars[c].has(g) { self.chars[c].genetic.push(g); }
        self.setup_bender(c, e);
        if let Some(&tp) = parents.iter().find(|&&p| self.chars[p].tribe != NONE) {
            let t = self.chars[tp].tribe;
            if self.tribes[t].members.len() < TRIBE_CAP + 3 { self.join_tribe(t, c); }
        }
        if let Some(need) = self.world_need() {
            if self.concord == NONE && !self.concord_waiting_child && e == self.concord_next {
                self.chars[c].concord = true;
                self.concord = c;
                self.concord_waiting_child = true;
                self.concord_line.push(ConcordLife { id: c, born: self.year, element: e, need, revealed: None, died: None, deeds: vec![] });
            }
        }
    }

    fn world_need(&self) -> Option<String> {
        if let Some(m) = self.monsters.iter().find(|m| m.alive && m.kind == "calamity") { return Some(format!("{} walks the world", m.name)); }
        for k in &self.realms { if k.ruler != NONE && !k.extinct { let r = &self.chars[k.ruler]; if r.alive && r.tyranny >= 60.0 { return Some(format!("the tyranny of {} {}", r.name, r.regnal.clone().unwrap_or_default())); } } }
        let wars: usize = self.war.iter().map(|row| row.iter().filter(|&&w| w).count()).sum::<usize>() / 2;
        if wars >= 4 { return Some("war on every side".into()); }
        if let Some(m) = self.monsters.iter().find(|m| m.alive && m.title == Some("Wyrm-Sovereign")) { if m.power > 18000.0 { return Some(format!("the shadow of {}", m.name.split(" (").next().unwrap())); } }
        None
    }

    pub fn bending_year(&mut self) {
        let y = self.year;
        for t in 0..self.tribes.len() {
            let alive: Vec<usize> = self.tribes[t].members.iter().copied().filter(|&m| self.chars[m].alive).collect();
            self.tribes[t].members = alive;
            if self.tribes[t].members.len() < TRIBE_CAP && self.rng.random() < 0.2 { let b = self.new_bender(t); self.join_tribe(t, b); }
            if let Some(&chief) = self.tribes[t].members.iter().max_by(|&&a, &&b| self.bend_power(&self.chars[a]).partial_cmp(&self.bend_power(&self.chars[b])).unwrap()) {
                if self.tribes[t].chief != chief {
                    self.tribes[t].chief = chief;
                    self.tribes[t].chiefs.push((y, chief));
                    let tn = self.tribes[t].name;
                    self.chars[chief].titles.push(format!("Chief of {tn} from year {y}"));
                }
            }
            for m in self.tribes[t].members.clone() {
                let c = &self.chars[m];
                if c.spouse == NONE && c.adult(y) && self.tribes[t].members.len() < TRIBE_CAP + 3 && self.rng.random() < 0.05 {
                    let (realm, born, fem) = (c.realm, c.born, c.female);
                    let b = born + self.rng.integers(-5, 6);
                    let mate = self.make_character(realm, b, Some(!fem), NONE, NONE, NONE);
                    if self.rng.random() < 0.5 {
                        let g = gene(ELEMENT_GENE[t]);
                        if !self.chars[mate].has(g) { self.chars[mate].genetic.push(g); }
                        self.setup_bender(mate, t);
                        self.join_tribe(t, mate);
                    }
                    self.chars[m].spouse = mate; self.chars[mate].spouse = m;
                    self.heart_on_marriage(m, mate);
                }
            }
        }
        for c in 0..self.chars.len() { if self.chars[c].alive && self.chars[c].element.is_some() { self.train_bender(c); } }
        self.concord_year();
    }

    fn train_bender(&mut self, c: usize) {
        let y = self.year;
        if !self.chars[c].adult(y) && self.chars[c].age(y) < 8 { return; }
        let gate = self.power_gate(&self.chars[c], false).max(self.power_gate(&self.chars[c], true));
        let top = match gate { 3 => 49.9, 4 => 74.9, _ => 100.0 };
        let conc = self.chars[c].concord;
        for i in 0..self.chars[c].bend.len() {
            let (e, before) = self.chars[c].bend[i];
            let after = (before + self.rng.uniform(0.5, 2.5) * (1.0 - before / 110.0) * if conc { 1.5 } else { 1.0 }).min(top);
            self.chars[c].bend[i].1 = after;
            let (tb, ta) = (bend_tier(before), bend_tier(after));
            if ta > tb && ta >= 2 {
                let t = format!("{} becomes a {} of {}-bending, master of {}.", self.who(c), BEND_TIER_NAMES[ta], ELEMENTS[e], BEND_FORMS[e].0[ta]);
                self.say(t, !conc);
                self.gain_fame(c, "bender", 30.0 * ta as f64);
            }
            if before < 70.0 && after >= 70.0 && self.chars[c].high_art.is_none() {
                let art = BEND_FORMS[e].1[self.rng.integers(0, 2) as usize];
                self.chars[c].high_art = Some(art);
                let t = format!("{} opens the high art of {art}.", self.who(c));
                self.say(t, true);
            }
        }
        let l = self.chars[c].level as f64;
        self.gain_xp(c, 40.0 + 2.0 * l);
        self.use_skill(c, skill_ix("Alteration"), 0.5);
        let bp = self.bend_power(&self.chars[c]);
        self.chars[c].peak_cr = self.chars[c].peak_cr.max(bp);
        let tribe = self.chars[c].tribe;
        if tribe != NONE && self.chars[c].adult(y) && self.rng.random() < 0.15 {
            let tp = self.tribes[tribe].pos;
            let Some(m) = (0..self.monsters.len()).find(|&m| self.monsters[m].alive && !matches!(self.monsters[m].kind, "calamity" | "dragon") && !self.monsters[m].sea && mdist(self.monsters[m].pos, tp) < 20.0) else { return };
            if self.rng.random() < bp / (bp + 0.6 * self.monsters[m].power) {
                self.hit(m, bp * 1.5);
                if self.monsters[m].power <= 0.0 && self.monsters[m].alive {
                    self.monsters[m].alive = false;
                    self.on_monster_slain(m, c);
                    let b = self.monsters[m].base;
                    self.gain_fame(c, "bender", 20.0 + b / 40.0);
                    if matches!(self.monsters[m].kind, "dragon" | "demon") {
                        let e = ELEMENTS.iter().position(|x| Some(*x) == self.chars[c].element).unwrap();
                        let lv = self.chars[c].bend.iter().find(|x| x.0 == e).map(|x| x.1).unwrap_or(0.0);
                        let t = format!("{} of {} brings down {} with {}.", self.who(c), self.tribes[tribe].name, self.monsters[m].name, BEND_FORMS[e].0[bend_tier(lv)]);
                        self.say(t, false);
                    }
                }
            } else {
                self.chars[c].hp -= self.rng.uniform(20.0, 70.0);
                if self.chars[c].hp <= 0.0 { let why = format!("was slain by {} defending {}", self.monsters[m].name, self.tribes[tribe].name); self.kill_by_monster(c, why, m); }
                else { self.chars[c].hp = self.chars[c].hp.max(20.0); }
            }
        }
    }

    fn concord_year(&mut self) {
        let y = self.year;
        let c = self.concord;
        if c == NONE { return; }
        if !self.chars[c].alive {
            let cause = self.chars[c].cause.clone().unwrap_or_default();
            if let Some(rec) = self.concord_line.last_mut() { rec.died = Some((y, cause.clone())); }
            let t = format!("**{} is dead.** {} {cause}. The soul passes on, to wait for the next age of need.", cap(CONCORD[0]), self.chars[c].name);
            self.say(t, false);
            self.concord = NONE;
            self.concord_waiting_child = false;
            let e = ELEMENTS.iter().position(|x| Some(*x) == self.chars[c].element).unwrap_or(0);
            self.concord_next = (e + 1) % 4;
            return;
        }
        if self.concord_line.last().unwrap().revealed.is_none() {
            if self.chars[c].adult(y) || self.chars[c].age(y) >= 16 {
                self.concord_line.last_mut().unwrap().revealed = Some(y);
                if self.chars[c].epithet.is_none() { self.chars[c].epithet = Some("the Concord".into()); }
                let need = self.concord_line.last().unwrap().need.clone();
                let t = format!("**{} returns.** In a time of need ({need}), {}, born a {}-bender of the {}, is revealed as {}: the soul who can bend all four elements.",
                                cap(CONCORD[0]), self.who(c), self.chars[c].element.unwrap(), PEOPLES[self.chars[c].people], CONCORD[0]);
                self.say(t, false);
                self.chars[c].hero = true;
                let p = self.chars[c].people;
                self.hero_guilds[p].heroes_total += 1;
            }
            return;
        }
        if self.chars[c].bend.len() < 4 && self.rng.random() < 0.15 {
            let e0 = ELEMENTS.iter().position(|x| Some(*x) == self.chars[c].element).unwrap();
            let nxt = (e0 + self.chars[c].bend.len()) % 4;
            self.chars[c].bend.push((nxt, 20.0));
            let t = format!("{}, {}, learns {}-bending from {}.", cap(CONCORD[0]), self.chars[c].name, ELEMENTS[nxt], self.tribes[nxt].name);
            self.say(t, true);
            if self.chars[c].bend.len() == 4 { let t = format!("**{} masters the fourth element.** {} can bend tide, stone, flame and sky.", cap(CONCORD[0]), self.chars[c].name); self.say(t, false); }
        }
        let pw = self.bend_power(&self.chars[c]);
        if self.world_need().is_none() { self.gain_fame(c, "concord", 5.0); return; }
        if let Some(m) = (0..self.monsters.len()).find(|&m| self.monsters[m].alive && self.monsters[m].kind == "calamity") {
            if self.rng.random() < 0.4 {
                let dmg = pw * self.rng.uniform(0.8, 1.6);
                self.monsters[m].power -= dmg;
                let mn = self.monsters[m].name.clone();
                self.concord_line.last_mut().unwrap().deeds.push(format!("year {y}: struck {mn}"));
                if self.monsters[m].power <= 0.0 {
                    self.monsters[m].alive = false;
                    let b = self.monsters[m].born;
                    let cn = self.chars[c].name.clone();
                    self.calamity_log.push((mn.clone(), b, y, format!("ended by {}, {cn}", CONCORD[0])));
                    self.monsters[m].slain_by = c;
                    self.say(format!("**{} ends {mn}.** {cn} enters the Concord State, all four elements at once, and the calamity is unmade.", cap(CONCORD[0])), false);
                    self.gain_fame(c, "concord", 800.0);
                    self.chars[c].renown += 1000.0;
                } else {
                    let d = self.rng.uniform(10.0, 60.0) * (self.monsters[m].power / pw.max(1.0)).min(3.0);
                    self.chars[c].hp -= d;
                    if self.chars[c].hp <= 0.0 { self.kill_by_monster(c, format!("fell in the Concord State against {mn}"), m); }
                }
                return;
            }
        }
        for k in 0..self.realms.len() {
            let r = self.realms[k].ruler;
            if r == NONE || self.realms[k].extinct || !self.chars[r].alive || self.chars[r].tyranny < 60.0 || self.rng.random() >= 0.3 { continue; }
            let rcr = self.cr(&self.chars[r], true);
            if self.rng.random() < pw / (pw + rcr * 3.0) {
                let t = format!("**{} confronts {}** and breaks the tyrant's power. The great of {} turn on their ruler.", cap(CONCORD[0]), self.who(r), self.realms[k].name);
                self.say(t, false);
                let deed = format!("year {y}: brought down {} {}", self.chars[r].name, self.chars[r].regnal.clone().unwrap_or_default());
                self.concord_line.last_mut().unwrap().deeds.push(deed);
                self.kill_by(r, format!("was brought down by {}", CONCORD[0]), true, c);
                self.gain_fame(c, "concord", 300.0);
            } else {
                self.chars[c].hp -= self.rng.uniform(20.0, 80.0);
                if self.chars[c].hp <= 0.0 { let why = format!("fell confronting the tyrant {} {}", self.chars[r].name, self.chars[r].regnal.clone().unwrap_or_default()); self.kill(c, why); }
            }
            return;
        }
        let wars: Vec<(usize, usize)> = (0..self.realms.len()).flat_map(|i| (i + 1..self.realms.len()).map(move |j| (i, j))).filter(|&(i, j)| self.war[i][j]).collect();
        if wars.len() >= 3 && self.rng.random() < 0.3 && y - self.chars[c].last_peace >= 15 {
            self.chars[c].last_peace = y;
            let (i, j) = wars[self.rng.integers(0, wars.len() as i64) as usize];
            self.war[i][j] = false; self.war[j][i] = false;
            self.relation[i][j] = 0.0; self.relation[j][i] = 0.0;
            self.truce[i][j] = y + 30; self.truce[j][i] = y + 30;
            let (pi, pj) = (self.pname(i), self.pname(j));
            self.say(format!("{}, {}, walks between the armies of the {pi} and the {pj}, and the war ends.", cap(CONCORD[0]), self.chars[c].name), false);
            self.concord_line.last_mut().unwrap().deeds.push(format!("year {y}: ended the war of the {pi} and the {pj}"));
            self.gain_fame(c, "concord", 150.0);
        }
    }
}

fn cap(s: &str) -> String { let mut c = s.chars(); match c.next() { Some(f) => f.to_uppercase().collect::<String>() + c.as_str(), None => String::new() } }
