//! The shinobi clans (shinobi.py): hidden villages in the wild; ki, natures and arts in ladders of five; ranks,
//! eye bloodlines, forbidden arts; missions, trump cards, contracts bid for between clans, feuds and clan wars.

use crate::monsters::mdist;
use crate::people::*;
use crate::power::skill_ix;
use crate::state::State;
use crate::tables::*;

const CLAN_CAP: usize = 7;
const TRUMP_CAP: usize = 2;

pub struct Clan { pub id: usize, pub name: &'static str, pub pos: (usize, usize), pub members: Vec<usize>, pub head: usize, pub heads: Vec<(i64, usize)>,
                  pub gold: f64, pub won: u32, pub lost: u32, pub pact: &'static str, pub kill_year: i64, pub standing: f64, pub holds: Vec<(i32, u32)> }

/// Their strongest technique and its tier.
pub fn technique(c: &Character) -> (u8, &'static str) {
    let mut best = (0u8, "");
    for &(n, lv) in &c.natures { if lv > best.0 { best = (lv, NATURES[n].1[lv as usize - 1]); } }
    for &(a, lv) in &c.sarts { if lv > best.0 && lv > 0 { best = (lv, SHINOBI_ARTS[a].1[lv as usize - 1]); } }
    best
}

pub fn shinobi_power(c: &Character) -> f64 {
    let lv: f64 = c.natures.iter().map(|x| x.1 as f64).sum::<f64>() + c.sarts.iter().map(|x| x.1 as f64).sum::<f64>();
    let top = technique(c).0 as i32;
    let mut p = c.ki.powf(0.6) * (1.0 + lv * 0.25) * 1.8f64.powi(top) * (1.0 + c.level as f64 / 60.0);
    if c.has(gene("hollow-eye")) { p *= 1.4; }
    p
}

fn has_trump(c: &Character) -> bool { c.natures.iter().any(|x| x.1 == 5) || c.sarts.iter().any(|x| x.1 == 5) }

impl State {
    pub fn found_clans(&mut self) {
        let land: Vec<(usize, usize)> = (0..self.map.h * self.map.w).filter(|&i| matches!(self.map.terrain.v[i], FOREST | MOUNTAIN | HILLS) && self.owner.v[i] == -1)
            .map(|i| (i / self.map.w, i % self.map.w)).collect();
        if land.is_empty() { return; }
        for (n, &name) in SHINOBI_CLANS.iter().enumerate() {
            let pos = land[self.rng.integers(0, land.len() as i64) as usize];
            self.clans.push(Clan { id: n, name, pos, members: vec![], head: NONE, heads: vec![], gold: 0.0, won: 0, lost: 0, pact: SUMMON_PACTS[n % SUMMON_PACTS.len()],
                                   kill_year: -1, standing: 10.0, holds: vec![] });
            for _ in 0..4 { self.recruit(n, true); }
        }
        let names = self.clans.iter().map(|c| c.name).collect::<Vec<_>>().join(", ");
        self.say(format!("Three hidden clans keep their villages in the wild places: {names}. No king rules them; every king hires them."), false);
    }

    fn recruit(&mut self, cl: usize, founding: bool) -> usize {
        let y = self.year;
        let parents: Vec<usize> = self.clans[cl].members.iter().copied().filter(|&m| self.chars[m].alive && self.chars[m].adult(y)).collect();
        let f = self.rng.weighted(&[0.45, 0.2, 0.15, 0.2]);
        let age = ADULT[f] as i64 + self.rng.integers(0, 10);
        let c = self.make_character(f, y - age, None, NONE, NONE, NONE);
        self.initiate(cl, c, founding, &parents);
        c
    }

    fn initiate(&mut self, cl: usize, c: usize, founding: bool, parents: &[usize]) {
        self.chars[c].clan = cl;
        self.clans[cl].members.push(c);
        self.chars[c].ki = self.rng.uniform(40.0, 80.0);
        let n1 = self.rng.integers(0, NATURES.len() as i64) as usize;
        let mut natures = vec![(n1, 1u8)];
        if self.rng.random() < 0.25 {
            let others: Vec<usize> = (0..NATURES.len()).filter(|&n| n != n1).collect();
            natures.push((others[self.rng.integers(0, others.len() as i64) as usize], 1));
        }
        self.chars[c].natures = natures;
        self.chars[c].sarts = (0..SHINOBI_ARTS.len()).filter(|&a| SHINOBI_ARTS[a].0 != "Forbidden").map(|a| (a, 0)).collect();
        self.chars[c].srank = 0;
        self.chars[c].missions = 0;
        for &(eye, _) in SHINOBI_EYES.iter() {
            let g = gene(eye);
            let carriers = parents.iter().any(|&p| self.chars[p].has(g));
            if self.rng.random() < if carriers { 0.3 } else { 0.0 } + 0.01 {
                if !self.chars[c].has(g) { self.chars[c].genetic.push(g); }
                if eye == "deepwell" { self.chars[c].ki *= 3.0; }
            }
        }
        let (sn, bl) = (skill_ix("Sneak"), skill_ix("Blade"));
        self.chars[c].skills[sn] = (self.chars[c].skills[sn] + 25.0).min(100.0);
        self.chars[c].skills[bl] = (self.chars[c].skills[bl] + 15.0).min(100.0);
        if !founding {
            if let Some(&(eye, what)) = SHINOBI_EYES.iter().find(|e| self.chars[c].has(gene(e.0))) {
                let t = format!("A child of {} opens the {}: {}. It {what}.", self.clans[cl].name, title_case(&eye.replace('-', " ")), self.chars[c].name);
                self.say(t, true);
            }
        }
    }

    fn trumps(&self, cl: usize) -> usize { self.clans[cl].members.iter().filter(|&&m| self.chars[m].alive && has_trump(&self.chars[m])).count() }

    fn may_trump(&self, c: usize, cl: usize) -> bool {
        let cc = &self.chars[c];
        if has_trump(cc) { return false; }
        (cc.srank >= 3 || (cc.srank >= 2 && self.clans[cl].head == c)) && self.trumps(cl) < TRUMP_CAP
    }

    fn announce_trump(&mut self, c: usize, cl: usize, name: &str) {
        let t = format!("**A trump card.** {} of {} masters {name}, a tier-V technique the clan will guard with their lives.", self.who(c), self.clans[cl].name);
        self.say(t, false);
    }

    fn target_in(&self, cl: usize) -> usize {
        let ms: Vec<usize> = self.clans[cl].members.iter().copied().filter(|&m| self.chars[m].alive).collect();
        let tr: Vec<usize> = ms.iter().copied().filter(|&m| has_trump(&self.chars[m])).collect();
        let pool = if tr.is_empty() { ms } else { tr };
        pool.into_iter().max_by(|&a, &b| shinobi_power(&self.chars[a]).partial_cmp(&shinobi_power(&self.chars[b])).unwrap()).unwrap_or(NONE)
    }

    pub fn shinobi_year(&mut self) {
        let y = self.year;
        for cl in 0..self.clans.len() {
            let alive: Vec<usize> = self.clans[cl].members.iter().copied().filter(|&m| self.chars[m].alive).collect();
            self.clans[cl].members = alive;
            let kids: Vec<usize> = self.living.iter().map(|&i| &self.chars[i]).filter(|c| c.alive && c.clan == NONE && c.age(y) >= 8 && !c.hero && c.ruled.is_none() && c.profession.is_none()
                && [c.father, c.mother].iter().any(|&p| p != NONE && self.chars[p].clan == cl)).map(|c| c.id).collect();
            for k in kids { if self.clans[cl].members.len() < CLAN_CAP { let ps: Vec<usize> = vec![]; self.initiate(cl, k, false, &ps); } }
            for m in self.clans[cl].members.clone() {
                if self.chars[m].spouse == NONE && self.chars[m].adult(y) && self.rng.random() < 0.05 {
                    let (realm, born, fem) = (self.chars[m].realm, self.chars[m].born, self.chars[m].female);
                    let b = born + self.rng.integers(-5, 6);
                    let mate = self.make_character(realm, b, Some(!fem), NONE, NONE, NONE);
                    self.chars[m].spouse = mate; self.chars[mate].spouse = m;
                    self.heart_on_marriage(m, mate);
                }
            }
            if self.clans[cl].members.len() < CLAN_CAP && self.rng.random() < 0.3 { self.recruit(cl, false); }
            for m in self.clans[cl].members.clone() { if self.chars[m].alive { self.train_shinobi(m, cl); } if self.chars[m].alive { self.mission(m, cl); } }
            let alive: Vec<usize> = self.clans[cl].members.iter().copied().filter(|&m| self.chars[m].alive).collect();
            if let Some(&head) = alive.iter().max_by(|&&a, &&b| shinobi_power(&self.chars[a]).partial_cmp(&shinobi_power(&self.chars[b])).unwrap()) {
                if self.clans[cl].head != head {
                    self.clans[cl].head = head;
                    self.clans[cl].heads.push((y, head));
                    let cn = self.clans[cl].name;
                    self.chars[head].titles.push(format!("{} of {cn} from year {y}", SHINOBI_HEAD[0]));
                    let t = format!("{} takes the veil as {} of {cn}, master of {}.", self.who(head), SHINOBI_HEAD[0], technique(&self.chars[head]).1);
                    self.say(t, true);
                }
            }
        }
        self.contracts_year();
        self.clan_wars();
        self.feuds();
    }

    fn train_shinobi(&mut self, c: usize, cl: usize) {
        let speed = if self.chars[c].has(gene("hollow-eye")) { 1.5 } else { 1.0 };
        self.chars[c].ki = (self.chars[c].ki * 1.01 + 1.5 * speed).min(1000.0);
        let l = self.chars[c].level as f64;
        self.gain_xp(c, 60.0 + 3.0 * l);
        self.use_skill(c, skill_ix("Sneak"), 1.0);
        self.use_skill(c, skill_ix("Blade"), 0.8);
        if self.rng.random() < 0.12 * speed {
            let i = self.rng.integers(0, self.chars[c].natures.len() as i64) as usize;
            let (n, lv) = self.chars[c].natures[i];
            if lv < 5 && self.chars[c].ki >= 40.0 * (lv as f64).powf(1.6) && (lv < 4 || self.may_trump(c, cl)) {
                self.chars[c].natures[i].1 += 1;
                let nl = lv + 1;
                if nl == 5 { self.announce_trump(c, cl, NATURES[n].1[4]); }
                if nl == 4 { let t = format!("{} of {} masters {}, a tier-IV {} technique.", self.who(c), self.clans[cl].name, NATURES[n].1[3], NATURES[n].0); self.say(t, true); }
            }
        }
        if self.rng.random() < 0.1 * speed && !self.chars[c].sarts.is_empty() {
            let i = self.rng.integers(0, self.chars[c].sarts.len() as i64) as usize;
            let (a, lv) = self.chars[c].sarts[i];
            if lv < 5 && self.chars[c].ki >= 30.0 * ((lv + 1) as f64).powf(1.5) && (lv < 4 || self.may_trump(c, cl)) {
                self.chars[c].sarts[i].1 += 1;
                if lv + 1 == 5 { self.announce_trump(c, cl, SHINOBI_ARTS[a].1[4]); }
                if SHINOBI_ARTS[a].0 == "Summoning" && lv + 1 >= 3 { let t = format!("{} signs the pact of {} in blood.", self.who(c), self.clans[cl].pact); self.say(t, true); }
            }
        }
        let forb = SHINOBI_ARTS.iter().position(|a| a.0 == "Forbidden").unwrap();
        let has_f = self.chars[c].sarts.iter().position(|x| x.0 == forb);
        match has_f {
            None if self.chars[c].has_trait("ambitious") && self.chars[c].ki > 300.0 && self.rng.random() < 0.02 => {
                self.chars[c].sarts.push((forb, 1));
                self.chars[c].life -= 10.0;
                let t = format!("{} of {} learns {}, a forbidden art. It costs years of life to use.", self.who(c), self.clans[cl].name, SHINOBI_ARTS[forb].1[0]);
                self.say(t, false);
            }
            Some(i) if self.chars[c].sarts[i].1 < 5 && self.rng.random() < 0.05 => { self.chars[c].sarts[i].1 += 1; self.chars[c].life -= 8.0; }
            _ => {}
        }
        let gate = self.power_gate(&self.chars[c], true);
        let rank = ((shinobi_power(&self.chars[c]).sqrt() / 9.0) as usize).min(match gate { 3 => 2, 4 => 3, _ => 4 });
        if rank > self.chars[c].srank {
            self.chars[c].srank = rank;
            if rank >= 3 { let t = format!("{} is named a {} of {}.", self.who(c), SHINOBI_RANKS[rank], self.clans[cl].name); self.say(t, rank < 4); }
        }
        let p = shinobi_power(&self.chars[c]);
        self.chars[c].peak_cr = self.chars[c].peak_cr.max(p);
    }

    fn mission(&mut self, c: usize, cl: usize) {
        if !self.chars[c].adult(self.year) || self.rng.random() > 0.5 { return; }
        let mut kind = ["spy", "guard", "kill", "beast"][self.rng.weighted(&[0.45, 0.37, 0.06, 0.12])];
        if kind == "kill" { if self.clans[cl].kill_year == self.year { kind = "spy"; } self.clans[cl].kill_year = self.year; }
        let pw = shinobi_power(&self.chars[c]);
        self.chars[c].missions += 1;
        match kind {
            "spy" | "guard" => {
                let pay = 30.0 + 10.0 * self.chars[c].srank as f64;
                if self.rng.random() < 0.9 {
                    self.chars[c].gold += pay; self.clans[cl].gold += pay * 0.3; self.clans[cl].won += 1;
                    let r = self.chars[c].srank as f64;
                    self.gain_fame(c, "shinobi", 2.0 + r);
                } else {
                    self.clans[cl].lost += 1;
                    if self.rng.random() < 0.2 { self.kill(c, "died on a mission no one will speak of".into()); }
                }
            }
            "beast" => {
                let cp = self.clans[cl].pos;
                let Some(m) = (0..self.monsters.len()).filter(|&m| self.monsters[m].alive && self.monsters[m].kind != "calamity" && !self.monsters[m].sea && mdist(self.monsters[m].pos, cp) < 30.0)
                    .min_by(|&a, &b| mdist(self.monsters[a].pos, cp).partial_cmp(&mdist(self.monsters[b].pos, cp)).unwrap()) else { return };
                if self.rng.random() < pw / (pw + 0.6 * self.monsters[m].power) {
                    self.hit(m, pw * 1.5);
                    if self.monsters[m].power <= 0.0 && self.monsters[m].alive {
                        self.monsters[m].alive = false;
                        self.on_monster_slain(m, c);
                        self.chars[c].gold += 200.0;
                        let b = self.monsters[m].base;
                        self.gain_fame(c, "shinobi", 20.0 + b / 50.0);
                        if matches!(self.monsters[m].kind, "dragon" | "demon") {
                            let t = format!("{} of {} kills {} with {}.", self.who(c), self.clans[cl].name, self.monsters[m].name, technique(&self.chars[c]).1);
                            self.say(t, false);
                        }
                    }
                } else {
                    self.chars[c].hp -= self.rng.uniform(20.0, 80.0);
                    if self.chars[c].hp <= 0.0 { let why = format!("was slain by {}", self.monsters[m].name); self.kill_by_monster(c, why, m); }
                    else { self.chars[c].hp = self.chars[c].hp.max(30.0); }
                }
            }
            _ => {
                let n = self.realms.len();
                let wars: Vec<(usize, usize)> = (0..n).flat_map(|i| (0..n).map(move |j| (i, j))).filter(|&(i, j)| i != j && self.war[i][j]).collect();
                if wars.is_empty() { return; }
                let (client, tp) = wars[self.rng.integers(0, wars.len() as i64) as usize];
                let mut tgts: Vec<usize> = self.living.iter().map(|&i| &self.chars[i]).filter(|h| h.alive && h.hero && h.realm == tp).map(|h| h.id).collect();
                let tr = self.realms[tp].ruler;
                if tr != NONE && self.rng.random() < 0.3 { tgts = vec![tr]; }
                if tgts.is_empty() { return; }
                let t = tgts[self.rng.integers(0, tgts.len() as i64) as usize];
                let defence = self.cr(&self.chars[t], true) * if self.chars[t].undead { 2.0 } else { 1.0 } + 10.0 * self.court_guard(tp, t);
                if self.rng.random() < pw / (pw + 3.0 * defence + 500.0) {
                    self.chars[c].gold += 400.0;
                    self.gain_fame(c, "shinobi", 40.0);
                    let tx = format!("{} dies in the night with no wound but a needle-mark: {} has done {} work.", self.who(t), self.clans[cl].name, self.pname(client));
                    self.say(tx, t != tr);
                    let by = self.realms[client].ruler;
                    let why = format!("was killed by a shinobi of {}", self.clans[cl].name);
                    self.kill_by(t, why, true, by);
                    self.realms[tp].security += 40.0;
                } else {
                    self.chars[c].hp -= self.rng.uniform(30.0, 100.0);
                    if self.chars[c].hp <= 0.0 { let why = format!("was cut down trying to kill {}", self.chars[t].name); self.kill(c, why); }
                    else { self.chars[c].hp = self.chars[c].hp.max(30.0); }
                }
            }
        }
    }

    fn steal_scroll(&mut self, win: usize, lose: usize, wc: usize) {
        let best = self.chars[lose].natures.iter().find(|x| x.1 == 5).map(|x| (true, x.0)).or(self.chars[lose].sarts.iter().find(|x| x.1 == 5).map(|x| (false, x.0)));
        let Some((nature, i)) = best else { return };
        if !self.may_trump(win, wc) || self.rng.random() > 0.35 { return; }
        let tech = if nature {
            let Some(x) = self.chars[win].natures.iter_mut().find(|x| x.0 == i) else { return };
            x.1 = 5; NATURES[i].1[4]
        } else {
            match self.chars[win].sarts.iter_mut().find(|x| x.0 == i) { Some(x) => x.1 = 5, None => self.chars[win].sarts.push((i, 5)) }
            SHINOBI_ARTS[i].1[4]
        };
        let t = format!("{} takes the scroll of {tech} from the body of {}; it belongs to {} now.", self.who(win), self.chars[lose].name, self.clans[wc].name);
        self.say(t, false);
    }

    fn duel(&mut self, a: usize, b: usize, war: bool) {
        let (ca, cb) = (self.target_in(a), self.target_in(b));
        if ca == NONE || cb == NONE { return; }
        let (pa, pb) = (shinobi_power(&self.chars[ca]), shinobi_power(&self.chars[cb]));
        let (win, lose) = if self.rng.random() < pa / (pa + pb) { (ca, cb) } else { (cb, ca) };
        let wc = self.chars[win].clan;
        let (tw, tl) = (technique(&self.chars[win]).1, technique(&self.chars[lose]).1);
        if war {
            let t = format!("In the clan war, {} of {} meets {}; {tw} answers {tl}, and {} does not come home.", self.who(win), self.clans[wc].name, self.who(lose), self.chars[lose].name);
            let minor = !has_trump(&self.chars[lose]);
            self.say(t, minor);
        } else {
            let t = format!("**Clan feud.** {} of {} meets {} in the mountain passes; {tw} answers {tl}, and {} does not come home.", self.who(win), self.clans[wc].name, self.who(lose), self.chars[lose].name);
            self.say(t, false);
            self.gain_fame(win, "shinobi", 60.0);
        }
        let why = format!("fell in a clan {} to {}", if war { "war" } else { "feud" }, self.chars[win].name);
        self.kill(lose, why);
        self.clans[wc].standing += if war { 10.0 } else { 15.0 };
        let lc = self.chars[lose].clan;
        if lc != NONE { self.clans[lc].standing *= 0.85; }
        self.steal_scroll(win, lose, wc);
    }

    fn feuds(&mut self) {
        if self.clans.len() < 2 || self.rng.random() > 0.03 { return; }
        let a = self.rng.integers(0, self.clans.len() as i64) as usize;
        let mut b = self.rng.integers(0, self.clans.len() as i64 - 1) as usize;
        if b >= a { b += 1; }
        if self.clans[a].members.is_empty() || self.clans[b].members.is_empty() { return; }
        self.duel(a, b, false);
    }

    /// Crowns and towns post shinobi work; the clans near enough bid; the losers resent it.
    fn contracts_year(&mut self) {
        let live: Vec<usize> = (0..self.clans.len()).filter(|&c| !self.clans[c].members.is_empty()).collect();
        if live.is_empty() { return; }
        for &cl in &live { let t = self.trumps(cl) as f64; self.clans[cl].standing = self.clans[cl].standing * 0.98 + 3.0 * t; }
        let mut jobs: Vec<(&str, usize, usize, f64, usize)> = vec![];   // kind, realm, town, fee, extra (enemy / quest)
        for k in 0..self.realms.len() {
            if !self.active(k) { continue; }
            let seat = self.capital_of(k);
            if seat == NONE { continue; }
            let revenue = (self.realms[k].income * self.realms[k].tax_now).max(1.0);
            let foes: Vec<usize> = (0..self.realms.len()).filter(|&o| self.war[k][o]).collect();
            if !foes.is_empty() && self.rng.random() < 0.4 { let f = foes[self.rng.integers(0, foes.len() as i64) as usize]; jobs.push(("spy", k, seat, 0.3 * revenue, f)); }
            let r = self.realms[k].ruler;
            if (self.realms[k].security > 30.0 || self.chars[r].has_trait("paranoid")) && self.rng.random() < 0.3 { jobs.push(("guard", k, seat, 0.2 * revenue, NONE)); }
        }
        let y = self.year;
        for qi in 0..self.hall.quests.len() {
            let q = &self.hall.quests[qi];
            if q.kind != "escort" { continue; }
            let t = q.target.1;
            if self.towns[t].alive && q.taken != y && self.towns[t].wealth > 8.0 * q.reward && self.rng.random() < 0.4 {
                let (id, rw) = (q.id, q.reward);
                self.hall.quests[qi].taken = y;
                jobs.push(("escort", self.towns[t].realm, t, 2.0 * rw, id));
            }
        }
        for (kind, k, place, fee, extra) in jobs {
            if self.realms[k].gold < fee && kind != "escort" { continue; }
            let pp = self.towns[place].pos;
            let near: Vec<usize> = live.iter().copied().filter(|&c| mdist(self.clans[c].pos, pp) < 80.0).collect();
            if near.is_empty() { continue; }
            let reg = self.map.landmass.at(pp.0, pp.1);
            let mut bids: Vec<(f64, usize)> = near.iter().map(|&c| {
                let hold = self.clans[c].holds.iter().find(|h| h.0 == reg).map(|h| h.1).unwrap_or(0) as f64;
                let head = self.clans[c].head;
                let hp = if head != NONE { shinobi_power(&self.chars[head]).powf(0.3) } else { 0.0 };
                ((self.clans[c].standing + 5.0 * hold).max(0.0).sqrt() * hp * self.rng.uniform(0.6, 1.4), c)
            }).collect();
            bids.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap());
            let cl = bids[0].1;
            for &(_, loser) in &bids[1..] {
                if !self.clans[loser].holds.iter().any(|h| h.0 == reg) || !self.clans[cl].holds.iter().any(|h| h.0 == reg) { continue; }
                let key = (cl.min(loser), cl.max(loser));
                match self.clan_rivalry.iter_mut().find(|r| r.0 == key) { Some(r) => r.1 += 0.5, None => self.clan_rivalry.push((key, 0.5)) }
            }
            let agent = if kind != "guard" { self.target_in(cl) } else {
                self.clans[cl].members.iter().copied().filter(|&m| self.chars[m].alive).max_by(|&a, &b| shinobi_power(&self.chars[a]).partial_cmp(&shinobi_power(&self.chars[b])).unwrap()).unwrap_or(NONE)
            };
            if agent == NONE { continue; }
            let ok = self.rng.random() < if kind == "escort" { 0.85 } else { 0.85 + 0.03 * self.chars[agent].srank as f64 };
            if kind != "escort" { self.realms[k].gold -= fee; } else { self.towns[place].wealth -= fee; self.hall.quests.retain(|q| q.id != extra); }
            self.chars[agent].gold += fee * 0.6;
            self.clans[cl].gold += fee * 0.4;
            match self.clans[cl].holds.iter_mut().find(|h| h.0 == reg) { Some(h) => h.1 += 1, None => self.clans[cl].holds.push((reg, 1)) }
            if ok {
                self.clans[cl].standing += 2.0;
                match kind { "spy" => self.realms[k].battle_mult *= 1.1, "guard" => self.realms[k].security += 20.0, _ => self.towns[place].wealth += fee * 3.0 }
            } else {
                self.clans[cl].standing *= 0.9;
                if kind == "spy" && self.rng.random() < 0.35 {
                    self.relation[k][extra] -= 15.0; self.relation[extra][k] -= 15.0;
                    let t = format!("A spy of {} in the pay of {} is taken in the court of {}.", self.clans[cl].name, self.realms[k].name, self.realms[extra].name);
                    self.say(t, true);
                }
            }
        }
        let first = *live.iter().max_by(|&&a, &&b| self.clans[a].standing.partial_cmp(&self.clans[b].standing).unwrap()).unwrap();
        let cur = self.first_clan;
        if cur != first && self.clans[first].standing > 40.0 && (cur == NONE || self.clans[first].standing > 1.3 * self.clans[cur].standing) {
            self.first_clan = first;
            let t = format!("**{} is first among the shinobi clans** now: the richest contracts go to them.", self.clans[first].name);
            self.say(t, false);
        }
    }

    fn clan_wars(&mut self) {
        let y = self.year;
        for r in self.clan_rivalry.iter_mut() { r.1 *= 0.9; }
        for i in 0..self.clan_rivalry.len() {
            let (key, v) = self.clan_rivalry[i];
            let truce = self.clan_truce.iter().find(|t| t.0 == key).map(|t| t.1).unwrap_or(0);
            if !self.clan_war.iter().any(|w| w.0 == key) && v >= 8.0 && truce <= y && !self.clans[key.0].members.is_empty() && !self.clans[key.1].members.is_empty() {
                self.clan_war.push((key, y));
                let t = format!("**A clan war.** {} and {} have undercut each other once too often; the passes between them close, and the contracts of the land go to whoever is left.", self.clans[key.0].name, self.clans[key.1].name);
                self.say(t, false);
            }
        }
        for (key, since) in self.clan_war.clone() {
            let (a, b) = key;
            if self.clans[a].members.is_empty() || self.clans[b].members.is_empty() || y - since > 15 {
                let (win, lose) = if self.clans[a].standing >= self.clans[b].standing { (a, b) } else { (b, a) };
                for (reg, n) in std::mem::take(&mut self.clans[lose].holds) {
                    match self.clans[win].holds.iter_mut().find(|h| h.0 == reg) { Some(h) => h.1 += n, None => self.clans[win].holds.push((reg, n)) }
                }
                if let Some(r) = self.clan_rivalry.iter_mut().find(|r| r.0 == key) { r.1 = 0.0; }
                self.clan_war.retain(|w| w.0 != key);
                self.clan_truce.retain(|t| t.0 != key);
                self.clan_truce.push((key, y + 20));
                let t = format!("The clan war ends: {} holds the contracts of the land, and {} {}.", self.clans[win].name, self.clans[lose].name,
                                if self.clans[lose].members.is_empty() { "is no more" } else { "withdraws into the hills" });
                self.say(t, false);
                continue;
            }
            if self.rng.random() < 0.5 { self.duel(a, b, true); }
        }
    }
}

fn title_case(s: &str) -> String {
    s.split(' ').map(|w| { let mut c = w.chars(); match c.next() { Some(f) => f.to_uppercase().collect::<String>() + c.as_str(), None => String::new() } }).collect::<Vec<_>>().join(" ")
}
