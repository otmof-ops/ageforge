//! The sea (seafaring.py): shipwrights, fleets and sea trade; explorers who find the rest of the world; colonies;
//! naval battles, blockades and landings; sea monsters; and pirates, each with a past and a résumé.

use crate::monsters::mdist;
use crate::people::*;
use crate::power::skill_ix;
use crate::state::State;
use crate::tables::*;

const PIRATE_CAP_BASE: usize = 6;

pub struct Haven { pub name: String, pub pos: (usize, usize), pub land: i32, pub founded: i64 }

impl State {
    fn ports_of(&self, k: usize) -> Vec<usize> { self.alive_towns_of(k).into_iter().filter(|&t| self.towns[t].port).collect() }

    pub fn navy_power(&self, k: usize) -> f64 {
        let adm = self.realms[k].council[5];
        let a = if adm != NONE && self.chars[adm].alive { self.skill_val(&self.chars[adm], skill_ix("Seamanship")) } else { 10.0 };
        self.realms[k].warships * (1.0 + self.realms[k].ship_tier as f64) * (1.0 + a / 100.0)
    }

    pub fn sea_name_at(&self, pos: (usize, usize)) -> String {
        let s = if self.map.terrain.at(pos.0, pos.1) == OCEAN { self.map.sea_of.at(pos.0, pos.1) } else { -1 };
        self.map.sea_names.iter().find(|x| x.0 as i32 == s).map(|x| x.1.clone()).unwrap_or("the open sea".into())
    }

    fn sea_cell(&self, pos: (usize, usize)) -> (usize, usize) {
        let (r0, c0) = (pos.0.saturating_sub(3), pos.1.saturating_sub(3));
        for r in r0..(pos.0 + 4).min(self.map.h) { for c in c0..(pos.1 + 4).min(self.map.w) { if self.map.terrain.at(r, c) == OCEAN { return (r, c); } } }
        pos
    }

    pub fn sea_year(&mut self) {
        let n = self.realms.len();
        if self.piracy.len() < n { self.piracy.resize(n, 0.0); self.sea_terror.resize(n, 0.0); }
        self.fleets_year();
        self.explore_year();
        self.colonize_year();
        self.naval_war_year();
        self.sea_monsters_year();
        self.pirates_year();
    }

    fn fleets_year(&mut self) {
        let y = self.year;
        for k in 0..self.realms.len() {
            if self.realms[k].extinct { continue; }
            let ps = self.ports_of(k);
            if ps.is_empty() { self.realms[k].warships *= 0.9; self.realms[k].merchants *= 0.9; continue; }
            let wrights: Vec<usize> = self.chars.iter().filter(|c| c.alive && c.realm == k && c.profession == Some("shipwright")).map(|c| c.id).collect();
            let best = wrights.iter().map(|&w| self.skill_val(&self.chars[w], skill_ix("Seamanship"))).fold(10.0, f64::max);
            let tier = (0..SHIPS.len()).filter(|&i| best >= SHIPS[i].1).max().unwrap_or(0);
            let cap = (self.cultures[self.realms[k].people].best("ships", 0.0) as i64 - 1).max(0) as usize;
            let tier = tier.min(cap) as u32;
            if tier > self.realms[k].ship_tier {
                self.realms[k].ship_tier = tier;
                let who = wrights.iter().copied().max_by(|&a, &b| self.skill_val(&self.chars[a], skill_ix("Seamanship")).partial_cmp(&self.skill_val(&self.chars[b], skill_ix("Seamanship"))).unwrap());
                if let (true, Some(w)) = (tier >= 2, who) {
                    let t = format!("{} lays down the first {} of {}.", self.who(w), SHIPS[tier as usize].0, self.realms[k].name);
                    self.say(t, tier < 3);
                    self.gain_fame(w, "shipwright", 80.0 * tier as f64);
                }
            }
            for &w in &wrights { self.use_skill(w, skill_ix("Seamanship"), 1.2); self.use_skill(w, skill_ix("Smithing"), 0.5); self.chars[w].gold += 30.0 + 10.0 * tier as f64; }
            let pw: f64 = ps.iter().map(|&t| self.towns[t].wealth).sum();
            let st = self.realms[k].ship_tier as f64;
            let cost = 40.0 * (1.0 + st);
            let held = &self.realms[k].held;
            let timber = if held.iter().any(|&r| matches!(RESOURCES[r].name, "timber" | "ancient wood")) { 1.0 } else { 0.4 };
            let at_war = self.war[k].iter().any(|&w| w);
            let realm = &mut self.realms[k];
            realm.warships = 0.97 * realm.warships + 0.004 * pw / cost * if at_war { 1.5 } else { 1.0 } * timber;
            realm.merchants = (4.0 * ps.len() as f64).min(0.97 * realm.merchants + 0.006 * pw / cost + 0.3);
            let partners = (0..self.realms.len()).filter(|&o| o != k && !self.realms[o].extinct && !self.war[k][o] && !self.ports_of(o).is_empty()).count() as f64;
            let lost = self.piracy[k] + self.sea_terror[k];
            let (merch, block) = (self.realms[k].merchants, self.realms[k].blockaded > y);
            for &t in &ps {
                if block { self.towns[t].wealth *= 0.985; continue; }
                self.towns[t].wealth += (2.0 + 1.5 * partners) * (merch / ps.len() as f64).min(3.0) * (1.0 - lost).max(0.1) * if self.towns[t].colony { 1.3 } else { 1.0 };
            }
            let adm = self.realms[k].council[5];
            if adm != NONE && self.chars[adm].alive { self.use_skill(adm, skill_ix("Seamanship"), 1.0); self.use_skill(adm, skill_ix("Navigation"), 0.5); }
        }
        for v in self.piracy.iter_mut() { *v = 0.0; }
        for v in self.sea_terror.iter_mut() { *v = 0.0; }
    }

    fn explore_year(&mut self) {
        for k in 0..self.realms.len() {
            if self.realms[k].extinct { continue; }
            let ps = self.ports_of(k);
            if ps.is_empty() || self.cultures[self.realms[k].people].best("ships", 0.0) < 1.0 { continue; }
            let mut unknown: Vec<usize> = (0..self.map.lands.len()).filter(|&l| !self.realms[k].known.contains(&(l as i32))).collect();
            if unknown.is_empty() { continue; }
            let mut tries: Vec<(usize, f64)> = self.chars.iter().filter(|c| c.alive && c.realm == k && c.profession == Some("captain")).map(|c| (c.id, self.skill_val(c, skill_ix("Navigation")))).collect();
            tries.push((NONE, 15.0));
            for (c, nav) in tries {
                let src = ps[self.rng.integers(0, ps.len() as i64) as usize];
                let sp = self.towns[src].pos;
                let mut best = (f64::MAX, 0);
                for &l in &unknown { let s = mdist(self.map.lands[l].center, sp) / (self.map.lands[l].size as f64).sqrt() + self.rng.random() * 20.0; if s < best.0 { best = (s, l); } }
                let tgt = best.1;
                let dist = mdist(self.map.lands[tgt].center, sp);
                let p = (0.25 + nav / 200.0) * (-dist / (80.0 + nav * 3.0)).exp() * if c == NONE { 0.3 } else { 1.0 };
                if c != NONE {
                    self.use_skill(c, skill_ix("Navigation"), 1.5);
                    self.use_skill(c, skill_ix("Seamanship"), 1.0);
                    if self.rng.random() < 0.02 + dist / 8000.0 {
                        let why = if self.rng.random() < 0.5 { "was lost at sea seeking a new land" } else { "was lost at sea seeking the edge of the map" };
                        self.kill(c, why.into());
                        continue;
                    }
                }
                if self.rng.random() < p {
                    self.discover(k, tgt, c);
                    unknown.retain(|&l| l != tgt);
                    if unknown.is_empty() { break; }
                }
            }
        }
    }

    fn discover(&mut self, f: usize, land: usize, by: usize) {
        self.realms[f].known.push(land as i32);
        let first = self.map.lands[land].found_by.is_empty() && !self.towns.iter().any(|t| t.alive && t.landmass == land as i32);
        self.map.lands[land].found_by.push((f as i64, self.year));
        self.realms[f].discoveries.push((self.year, land as i32, by));
        let who = if by != NONE { self.who(by) } else { format!("sailors of {}", self.realms[f].name) };
        let (kind, name, size) = (self.map.lands[land].kind, self.map.lands[land].name.clone(), self.map.lands[land].size);
        if by != NONE { self.gain_fame(by, "explorer", if first { 150.0 } else { 40.0 } * if kind == "continent" { 2.0 } else { 1.0 }); }
        if kind == "continent" || (first && kind == "great isle") {
            let tail = if first { ", where no one of any people has ever set foot.".to_string() } else {
                let others: Vec<String> = self.map.lands[land].found_by.iter().filter(|x| x.0 != f as i64).map(|x| if x.0 >= 0 { self.pname(x.0 as usize) } else { "pirates".into() }).collect();
                format!(", already known to {}.", others.join(", "))
            };
            let lead = if first && kind == "continent" { "**A new world.** " } else { "" };
            self.say(format!("{lead}{who} sights {name}, a {kind} of {} leagues{tail}", crate::economy::thousands(size as f64)), !first);
        } else { self.say(format!("{who} charts {name}."), true); }
    }

    fn colonize_year(&mut self) {
        for k in 0..self.realms.len() {
            if self.realms[k].extinct || !self.cultures[self.realms[k].people].has("colonize") { continue; }
            let ps: Vec<usize> = self.ports_of(k).into_iter().filter(|&t| self.towns[t].pop > 250.0).collect();
            let home = self.realms[k].home_land;
            let foreign: Vec<i32> = self.realms[k].known.iter().copied().filter(|&l| l != home).collect();
            if ps.is_empty() || foreign.is_empty() || self.realms[k].merchants < 2.0 { continue; }
            let towns = self.alive_towns_of(k);
            let crowd = towns.iter().map(|&t| self.towns[t].pop).sum::<f64>() / (towns.len() * 600).max(1) as f64;
            if self.rng.random() > 0.06 * (crowd + 0.3).min(2.0) * (self.realms[k].merchants / 5.0).min(2.0) { continue; }
            let lid = foreign[self.rng.integers(0, foreign.len() as i64) as usize];
            let mut cells: Vec<(usize, usize)> = (0..self.map.h * self.map.w).filter(|&i| self.map.coast.v[i] && self.map.landmass.v[i] == lid && self.owner.v[i] == -1 && !self.blocked.v[i])
                .map(|i| (i / self.map.w, i % self.map.w)).collect();
            if cells.is_empty() { continue; }
            for i in 0..cells.len().min(300) { let j = i + self.rng.integers(0, (cells.len() - i) as i64) as usize; cells.swap(i, j); }
            cells.truncate(300);
            let p = self.realms[k].people;
            let score = |&(r, c): &(usize, usize)| -> f64 {
                let mut s = 0.0;
                for rr in r.saturating_sub(2)..(r + 3).min(self.map.h) { for cc in c.saturating_sub(2)..(c + 3).min(self.map.w) { s += FOOD[p][self.map.terrain.at(rr, cc) as usize]; } }
                s
            };
            let best = *cells.iter().max_by(|a, b| score(a).partial_cmp(&score(b)).unwrap()).unwrap();
            if score(&best) < 12.0 { continue; }
            let src = *ps.iter().max_by(|&&a, &&b| self.towns[a].pop.partial_cmp(&self.towns[b].pop).unwrap()).unwrap();
            let settlers = 0.25 * self.towns[src].pop;
            self.towns[src].pop -= settlers;
            self.towns[src].wealth -= 40.0;
            let t = self.add_town(k, best, settlers, false);
            self.towns[t].colony = true;
            self.towns[t].metropole = src;
            self.realms[k].colonies += 1;
            let first_here = self.alive_towns_of(k).iter().filter(|&&s| self.towns[s].landmass == lid).count() == 1;
            let tx = format!("{}Settlers from {} sail for {} and found {}, a colony of {}.", if first_here { "**A colony.** " } else { "" }, self.towns[src].name, self.land_name(lid), self.towns[t].name, self.realms[k].name);
            self.say(tx, !first_here);
        }
    }

    fn naval_war_year(&mut self) {
        let n = self.realms.len();
        for i in 0..n {
            for j in i + 1..n {
                if !self.war[i][j] || self.rng.random() > 0.5 { continue; }
                if self.realms[i].warships < 1.0 || self.realms[j].warships < 1.0 { continue; }
                let pa = self.navy_power(i) * self.rng.uniform(0.6, 1.4);
                let pb = self.navy_power(j) * self.rng.uniform(0.6, 1.4);
                let (win, lose) = if pa > pb { (i, j) } else { (j, i) };
                let frac = pa.max(pb) / (pa + pb);
                let (a, b) = (self.rng.uniform(0.3, 0.6), self.rng.uniform(0.1, 0.3));
                self.realms[lose].warships *= 1.0 - a * frac;
                self.realms[win].warships *= 1.0 - b * (1.0 - frac);
                self.realms[lose].blockaded = self.year + 3;
                self.battle_result_pub(win, lose);
                let (pw, pl) = (self.ports_of(win), self.ports_of(lose));
                let where_ = if let Some(&p) = pl.first() { let c = self.sea_cell(self.towns[p].pos); self.sea_name_at(c) } else { "the open sea".into() };
                let big = self.realms[i].warships + self.realms[j].warships > 40.0;
                let (ws, ls) = (SHIPS[self.realms[win].ship_tier as usize].0, SHIPS[self.realms[lose].ship_tier as usize].0);
                self.naval_battles.push((self.year, where_.clone(), self.realms[win].short.clone(), self.realms[lose].short.clone()));
                let title = crate::bending_title(&where_.replacen("the ", "", 1));
                let t = format!("**The Battle of {title}.** The {}s of {} break the fleet of {}; its ports are blockaded.", ws.to_lowercase(), self.realms[win].name, self.realms[lose].name);
                self.say(t, !big);
                let _ = ls;
                for (k, won) in [(win, true), (lose, false)] {
                    let adm = self.realms[k].council[5];
                    if adm != NONE && self.chars[adm].alive {
                        self.use_skill(adm, skill_ix("Seamanship"), 2.0);
                        self.gain_fame(adm, "admiral", if won { 40.0 } else { 5.0 });
                        if !won && self.rng.random() < 0.15 { let why = format!("went down with the flagship of {}", self.realms[k].name); self.kill(adm, why); }
                    }
                }
                if !pl.is_empty() && self.rng.random() < 0.5 {
                    let tgt = pl[self.rng.integers(0, pl.len() as i64) as usize];
                    let force = pw.iter().map(|&t| self.towns[t].pop).sum::<f64>() * 0.05 * (1.0 + self.realms[win].ship_tier as f64 * 0.2);
                    if force > self.towns[tgt].pop * 0.5 * (1.0 + 0.3 * self.towns[tgt].walls as f64) {
                        self.lose_lordship(tgt);
                        let t = format!("A host lands from the ships of {} and takes {} on {}.", self.realms[win].name, self.towns[tgt].name, self.land_name(self.towns[tgt].landmass));
                        self.say(t, true);
                        self.towns[tgt].realm = win; self.towns[tgt].capital = false; self.towns[tgt].pop *= 0.7;
                        self.realms[win].taken += 1; self.realms[lose].lost += 1;
                    } else { self.towns[tgt].pop *= 0.92; self.towns[tgt].wealth *= 0.8; }
                }
            }
        }
    }

    fn sea_monsters_year(&mut self) {
        let deep: Vec<(usize, usize)> = (0..self.map.h * self.map.w).filter(|&i| self.map.sea_dist.v[i] >= 6.0).map(|i| (i / self.map.w, i % self.map.w)).collect();
        let share = (deep.len() / 15000).max(1);
        for &(kind, prange, cap, _, p0, life) in SEA_MONSTERS.iter() {
            let n = self.monsters.iter().filter(|m| m.alive && m.kind == kind).count();
            if n < cap * share && self.rng.random() < p0 * share as f64 && !deep.is_empty() {
                let pos = deep[self.rng.integers(0, deep.len() as i64) as usize];
                let name = format!("{} the {}", self.name(&SEA_MONSTER_A, &SEA_MONSTER_B), crate::bending_title(kind));
                let p = self.rng.uniform(prange.0, prange.1);
                let m = self.new_monster(kind, pos, p, Some(name.clone()), &[], true, false);
                self.monsters[m].sea = true;
                self.monsters[m].lifespan = life * self.rng.uniform(0.7, 1.2);
                if kind != "sea serpent" { let t = format!("Sailors of {} speak of {name} rising from the deep.", self.sea_name_at(pos)); self.say(t, kind == "kraken"); }
            }
        }
        for m in 0..self.monsters.len() {
            if !self.monsters[m].alive || !self.monsters[m].sea { continue; }
            for _ in 0..3 {
                let p = self.monsters[m].pos;
                let (dr, dc) = (self.rng.integers(-4, 5), self.rng.integers(-4, 5));
                let np = ((p.0 as i64 + dr).clamp(0, self.map.h as i64 - 1) as usize, (p.1 as i64 + dc).clamp(0, self.map.w as i64 - 1) as usize);
                if self.map.terrain.at(np.0, np.1) == OCEAN { self.monsters[m].pos = np; break; }
            }
            for k in 0..self.realms.len() {
                let mp = self.monsters[m].pos;
                if !self.ports_of(k).iter().any(|&t| mdist(self.towns[t].pos, mp) < 30.0) { continue; }
                self.sea_terror[k] += (self.monsters[m].power / 20000.0).min(0.4);
                if self.rng.random() < 0.05 { self.realms[k].merchants *= 0.9; }
                if self.realms[k].warships >= 3.0 && self.rng.random() < 0.15 {
                    let np = self.navy_power(k) * 30.0;
                    if self.rng.random() < np / (np + self.monsters[m].power) {
                        self.monsters[m].power -= np;
                        if self.monsters[m].power <= 0.0 {
                            self.monsters[m].alive = false;
                            let adm = self.realms[k].council[5];
                            let slayer = if adm != NONE && self.chars[adm].alive { adm } else { NONE };
                            self.on_monster_slain(m, slayer);
                            let t = format!("The fleet of {} hunts down {}.", self.realms[k].name, self.monsters[m].name);
                            let minor = self.monsters[m].kind == "sea serpent";
                            self.say(t, minor);
                            if slayer != NONE { let ren = SEA_MONSTERS.iter().find(|s| s.0 == self.monsters[m].kind).unwrap().3; self.gain_fame(slayer, "admiral", ren); }
                            break;
                        }
                    } else { self.realms[k].warships *= 0.85; }
                }
            }
        }
    }

    fn ensure_havens(&mut self) {
        if !self.havens.is_empty() { return; }
        let mut isles: Vec<usize> = (0..self.map.lands.len()).filter(|&l| self.map.lands[l].kind != "continent").collect();
        for i in (1..isles.len()).rev() { let j = self.rng.integers(0, i as i64 + 1) as usize; isles.swap(i, j); }
        for &l in isles.iter().take(4) {
            let cells: Vec<usize> = (0..self.map.h * self.map.w).filter(|&i| self.map.coast.v[i] && self.map.landmass.v[i] == l as i32).collect();
            if cells.is_empty() { continue; }
            let i = cells[self.rng.integers(0, cells.len() as i64) as usize];
            let style = HAVEN_STYLE[self.rng.integers(0, HAVEN_STYLE.len() as i64) as usize];
            let n = self.name(&["Rum", "Gallow", "Black", "Salt", "Rat", "Grog"], &["wick", "mouth", "bay", "hole", "tooth"]);
            self.havens.push(Haven { name: style.replace("{}", &n), pos: (i / self.map.w, i % self.map.w), land: l as i32, founded: self.year });
        }
    }

    fn plog(&mut self, p: usize, text: String) { let y = self.year; self.chars[p].resume.push(format!("Year {y}: {text}")); }

    fn new_pirate(&mut self, src: usize) -> usize {
        let y = self.year;
        let hv = self.rng.integers(0, self.havens.len() as i64) as usize;
        let (p, origin) = if src == NONE {
            let f = self.rng.integers(0, 4.min(self.realms.len() as i64)) as usize;
            let age = ADULT[self.realms[f].people] as i64 + self.rng.integers(2, 15);
            let p = self.make_character(f, y - age, None, NONE, NONE, NONE);
            let towns = { let t = self.alive_towns_of(f); if t.is_empty() { (0..self.towns.len()).filter(|&t| self.towns[t].alive).collect() } else { t } };
            let town = if towns.is_empty() { "nowhere".into() } else { self.towns[towns[self.rng.integers(0, towns.len() as i64) as usize]].name.clone() };
            let o = PIRATE_ORIGINS[self.rng.integers(0, PIRATE_ORIGINS.len() as i64) as usize].replace("{town}", &town).replace("{realm}", &self.realms[f].name);
            (p, o)
        } else {
            let c = &self.chars[src];
            let o = match c.profession {
                Some("thief") => format!("a thief of {}, wanted for {} gold, who fled to sea", self.realms[c.realm].name, crate::economy::thousands(c.bounty)),
                Some("bandit") => "a bandit chief who traded the road for the waves".into(),
                _ => format!("once {}, now fallen", c.titles.last().cloned().unwrap_or("a nobody".into())),
            };
            (src, o)
        };
        let ship = format!("the {} {}", PIRATE_SHIP_A[self.rng.integers(0, PIRATE_SHIP_A.len() as i64) as usize], PIRATE_SHIP_B[self.rng.integers(0, PIRATE_SHIP_B.len() as i64) as usize]);
        let crew = self.rng.integers(20, 50);
        let c = &mut self.chars[p];
        c.profession = Some("pirate");
        c.pirate = true;
        c.ship = (ship.clone(), 0, crew);
        c.haven = hv;
        c.infamy = 0.0;
        c.marque = NONE;
        c.resume = vec![format!("Born year {}, {origin}.", c.born)];
        for (s, add) in [("Seamanship", 30.0), ("Navigation", 15.0), ("Blade", 15.0)] { let i = skill_ix(s); c.skills[i] = (c.skills[i] + add).min(100.0); }
        let hn = self.havens[hv].name.clone();
        self.plog(p, format!("takes {ship}, a {} with {crew} hands, and runs up the black flag at {hn}.", SHIPS[0].0.to_lowercase()));
        self.pirates.push(p);
        let t = format!("{} turns pirate, sailing {ship} out of {hn}.", self.who(p));
        self.say(t, true);
        p
    }

    fn pirates_year(&mut self) {
        self.ensure_havens();
        if self.havens.is_empty() { return; }
        let mut alive: Vec<usize> = self.pirates.iter().copied().filter(|&p| self.chars[p].alive && self.chars[p].pirate).collect();
        let cap = PIRATE_CAP_BASE + self.map.lands.len() / 8;
        if alive.len() < cap && self.rng.random() < 0.3 {
            let wanted = self.chars.iter().find(|c| c.alive && matches!(c.profession, Some("thief") | Some("bandit")) && c.bounty > 400.0 && !c.pirate).map(|c| c.id);
            let src = if wanted.is_some() && self.rng.random() < 0.5 { wanted.unwrap() } else { NONE };
            alive.push(self.new_pirate(src));
        }
        for p in alive { if self.chars[p].alive { self.pirate_turn(p); } }
        self.crown_pirate_king();
    }

    fn pirate_power(&self, p: usize) -> f64 {
        let c = &self.chars[p];
        c.ship.2 as f64 * (1.0 + c.ship.1 as f64) * (1.0 + self.skill_val(c, skill_ix("Seamanship")) / 100.0) * (1.0 + self.skill_val(c, skill_ix("Blade")) / 200.0)
    }

    fn infamy(&mut self, p: usize, x: f64) { self.chars[p].infamy += x; self.gain_fame(p, "pirate", x); }

    fn pirate_turn(&mut self, p: usize) {
        let c = &self.chars[p];
        let t = |n: &str| c.has_trait(n) as u8 as f64;
        let any_war = self.war.iter().any(|r| r.iter().any(|&w| w));
        let mut ch = vec![("shipping", 3.0), ("port", 1.0 + 2.0 * t("cruel") + t("ambitious")), ("treasure", 1.5 + t("ambitious")),
                          ("bury", 0.5 + t("paranoid") + t("greedy") + if c.gold > 2000.0 { 1.0 } else { 0.0 }), ("marque", 0.0), ("beast", 0.3 + t("brave")),
                          ("duel", 0.3 + t("wrathful")), ("refit", 0.0), ("retire", 0.0)];
        if c.gold > 600.0 * (c.ship.1 + 1) as f64 && c.ship.1 < 4 { ch[7].1 = 2.5; }
        if c.marque == NONE && any_war && !c.has_trait("cruel") { ch[4].1 = 0.8 + t("ambitious"); }
        if c.gold > 4000.0 && (c.age(self.year) > ADULT[c.people] as i64 + 25 || c.has_trait("content")) { ch[8].1 = 2.0; }
        let w: Vec<f64> = ch.iter().map(|x| x.1).collect();
        let act = ch[self.rng.weighted(&w)].0;
        let pw = self.pirate_power(p);
        self.use_skill(p, skill_ix("Seamanship"), 1.0);
        let marque = self.chars[p].marque;
        match act {
            "shipping" => {
                let opts: Vec<usize> = (0..self.realms.len()).filter(|&k| !self.realms[k].extinct && self.realms[k].merchants > 1.0 && (marque == NONE || self.war[marque][k])).collect();
                let mut best = (f64::MIN, NONE);
                for k in opts { let s = self.realms[k].merchants * (1.0 + self.rng.random()); if s > best.0 { best = (s, k); } }
                let k = best.1;
                if k == NONE || self.alive_towns_of(k).is_empty() { return; }
                let guard = self.navy_power(k) * 0.3;
                if self.rng.random() < pw / (pw + guard + 50.0) {
                    let loot = 60.0 + 40.0 * self.realms[k].merchants * self.rng.uniform(0.5, 1.5);
                    self.chars[p].gold += loot;
                    self.chars[p].bounty += loot * if marque != NONE { 0.0 } else { 0.5 };
                    self.realms[k].merchants *= 0.95;
                    self.piracy[k] += 0.05;
                    self.infamy(p, loot / 40.0);
                    let at = { let c = self.capital_of(k); if c != NONE { c } else { self.alive_towns_of(k)[0] } };
                    let sea = self.sea_name_at(self.sea_cell(self.towns[at].pos));
                    self.plog(p, format!("takes a {} merchantman off {sea}{}: {} gold.", SINGULAR[self.realms[k].people].to_lowercase(), if marque != NONE { " under letters of marque" } else { "" }, crate::economy::thousands(loot)));
                } else {
                    self.chars[p].ship.2 = (self.chars[p].ship.2 as f64 * 0.8) as i64;
                    let kn = self.realms[k].name.clone();
                    self.plog(p, format!("is driven off by the navy of {kn}, losing a fifth of the crew."));
                    if self.rng.random() < 0.15 { self.captured(p, k); return; }
                }
            }
            "port" => {
                let targets: Vec<usize> = (0..self.towns.len()).filter(|&t| self.towns[t].alive && self.towns[t].port && (marque == NONE || self.war[marque][self.towns[t].realm])).collect();
                if targets.is_empty() { return; }
                let tt = targets[self.rng.integers(0, targets.len() as i64) as usize];
                let defence = self.towns[tt].pop * 0.1 * (1.0 + 0.3 * self.towns[tt].walls as f64);
                if self.rng.random() < pw / (pw + defence) {
                    let loot = self.towns[tt].wealth * 0.3;
                    self.towns[tt].wealth -= loot; self.towns[tt].pop *= 0.95;
                    self.chars[p].gold += loot; self.chars[p].bounty += loot;
                    let burn = self.chars[p].has_trait("cruel") && self.rng.random() < 0.2 && self.towns[tt].pop < 400.0 && !self.towns[tt].capital;
                    self.infamy(p, loot / 20.0 + if burn { 40.0 } else { 0.0 });
                    let (tn, pn) = (self.towns[tt].name.clone(), self.pname(self.towns[tt].realm));
                    let b = if burn { " and burns it to the waterline" } else { "" };
                    self.plog(p, format!("sacks the port of {tn} ({pn}){b}: {} gold.", crate::economy::thousands(loot)));
                    let tx = format!("{}, captain of {}, sacks {tn}{b}.", self.who(p), self.chars[p].ship.0);
                    self.say(tx, !burn && loot < 400.0);
                    if burn { self.destroy(tt); }
                } else {
                    self.chars[p].ship.2 = (self.chars[p].ship.2 as f64 * 0.7) as i64;
                    let tn = self.towns[tt].name.clone();
                    self.plog(p, format!("is beaten off the walls of {tn}."));
                }
            }
            "treasure" => {
                let ruins: Vec<usize> = (0..self.ruins.len()).filter(|&r| !self.ruins[r].loot.is_empty() || self.ruins[r].riches > 0).collect();
                let unknown: Vec<usize> = (0..self.map.lands.len()).filter(|&l| self.map.lands[l].found_by.is_empty()).collect();
                if !unknown.is_empty() && self.rng.random() < 0.3 {
                    let l = unknown[self.rng.integers(0, unknown.len() as i64) as usize];
                    self.map.lands[l].found_by.push((-1, self.year));
                    self.infamy(p, 30.0);
                    let (ln, lk) = (self.map.lands[l].name.clone(), self.map.lands[l].kind);
                    self.plog(p, format!("finds an unknown shore: {ln}, a {lk} no chart has ever shown."));
                    let tx = format!("The pirate {} is the first to see {ln}, and tells no one but the crew.", self.who(p));
                    self.say(tx, lk != "continent");
                } else if !ruins.is_empty() {
                    let r = ruins[self.rng.integers(0, ruins.len() as i64) as usize];
                    if !self.ruins[r].loot.is_empty() {
                        let it = self.ruins[r].loot.remove(0);
                        self.give_item(p, it, true);
                        let b = self.items[it].bonus;
                        self.infamy(p, 20.0 + 40.0 * b);
                        let (lb, rn) = (self.items[it].label(), self.ruins[r].name.clone());
                        self.plog(p, format!("digs {lb} out of {rn}."));
                        if self.items[it].rarity >= 4 { let tx = format!("{} digs {lb} out of {rn}.", self.who(p)); self.say(tx, false); }
                    } else {
                        self.ruins[r].riches -= 1;
                        let g = 100.0 * self.ruins[r].depth;
                        self.chars[p].gold += g;
                        let rn = self.ruins[r].name.clone();
                        self.plog(p, format!("loots {rn} for {} gold.", crate::economy::thousands(g)));
                    }
                }
            }
            "bury" => {
                let amt = self.chars[p].gold * 0.6;
                let isles: Vec<usize> = (0..self.map.lands.len()).filter(|&l| self.map.lands[l].kind == "isle").collect();
                let l = if isles.is_empty() { 0 } else { isles[self.rng.integers(0, isles.len() as i64) as usize] };
                let cells: Vec<usize> = (0..self.map.h * self.map.w).filter(|&i| self.map.landmass.v[i] == l as i32).collect();
                if cells.is_empty() { return; }
                let i = cells[self.rng.integers(0, cells.len() as i64) as usize];
                let mut ru = Ruin::new(format!("the buried hoard of {}", self.chars[p].name), (i / self.map.w, i % self.map.w), 2 + (amt / 500.0) as i64, 1.5);
                let mine: Vec<usize> = self.chars[p].items.iter().copied().filter(|&it| self.items[it].rarity >= 3).take(2).collect();
                self.chars[p].items.retain(|it| !mine.contains(it));
                ru.loot = mine.clone();
                self.ruins.push(ru);
                self.chars[p].gold -= amt;
                let ln = self.map.lands[l].name.clone();
                let what = if mine.is_empty() { String::new() } else { format!(" and {}", mine.iter().map(|&i| self.items[i].label()).collect::<Vec<_>>().join(", ")) };
                self.plog(p, format!("buries {} gold{what} on {ln}, and draws a map.", crate::economy::thousands(amt)));
            }
            "marque" => {
                let n = self.realms.len();
                let wars: Vec<(usize, usize)> = (0..n).flat_map(|i| (0..n).map(move |j| (i, j))).filter(|&(i, j)| i != j && self.war[i][j]).collect();
                if let Some(&(i, j)) = wars.get(self.rng.integers(0, wars.len().max(1) as i64) as usize) {
                    self.chars[p].marque = i;
                    self.chars[p].bounty *= 0.5;
                    let (kn, pj) = (self.realms[i].name.clone(), self.pname(j));
                    self.plog(p, format!("takes letters of marque from {kn}: a privateer now, licensed to prey on the {pj}."));
                    let tx = format!("{} sails as a privateer for {kn}.", self.who(p));
                    self.say(tx, true);
                }
            }
            "beast" => {
                let hp = self.havens[self.chars[p].haven].pos;
                let Some(m) = (0..self.monsters.len()).filter(|&m| self.monsters[m].alive && self.monsters[m].sea).min_by(|&a, &b| mdist(self.monsters[a].pos, hp).partial_cmp(&mdist(self.monsters[b].pos, hp)).unwrap()) else { return };
                let mn = self.monsters[m].name.clone();
                if self.rng.random() < pw * 3.0 / (pw * 3.0 + self.monsters[m].power) {
                    self.monsters[m].power -= pw * 4.0;
                    if self.monsters[m].power <= 0.0 {
                        self.monsters[m].alive = false;
                        self.on_monster_slain(m, p);
                        let ren = SEA_MONSTERS.iter().find(|s| s.0 == self.monsters[m].kind).map(|s| s.3).unwrap_or(30.0);
                        self.infamy(p, ren);
                        self.plog(p, format!("kills {mn} and hangs its teeth from the yards."));
                        let tx = format!("The pirate {} kills {mn}.", self.who(p));
                        let minor = self.monsters[m].kind == "sea serpent";
                        self.say(tx, minor);
                    } else { self.plog(p, format!("wounds {mn} and lives to tell it.")); }
                } else {
                    self.chars[p].ship.2 /= 2;
                    self.plog(p, format!("is half-drowned by {mn}; half the crew are lost."));
                    if self.rng.random() < 0.25 { self.kill_by_monster(p, format!("was dragged under by {mn}"), m); return; }
                }
            }
            "duel" => {
                let rivals: Vec<usize> = self.pirates.iter().copied().filter(|&o| o != p && self.chars[o].alive && self.chars[o].pirate).collect();
                if rivals.is_empty() { return; }
                let o = rivals[self.rng.integers(0, rivals.len() as i64) as usize];
                let po = self.pirate_power(o);
                let (win, lose) = if self.rng.random() < (pw + 1e-9) / (pw + po + 2e-9) { (p, o) } else { (o, p) };
                let (ln, ls, wn) = (self.chars[lose].name.clone(), self.chars[lose].ship.0.clone(), self.chars[win].name.clone());
                self.plog(win, format!("meets {ln} of {ls} and takes the ship."));
                self.plog(lose, format!("is beaten by {wn} and goes over the side."));
                let g = self.chars[lose].gold;
                self.chars[win].gold += g;
                let crew = self.chars[lose].ship.2 / 2;
                self.chars[win].ship.2 += crew;
                if win == p { self.infamy(p, 30.0); }
                let tx = format!("{} defeats the pirate {ln} and takes {ls}.", self.who(win));
                self.say(tx, true);
                self.kill(lose, format!("was killed by the pirate {wn}"));
                if !self.chars[p].alive { return; }
            }
            "refit" => {
                let cost = 600.0 * (self.chars[p].ship.1 + 1) as f64;
                self.chars[p].gold -= cost;
                self.chars[p].ship.1 += 1;
                self.chars[p].ship.2 += 30;
                let (sn, t) = (self.chars[p].ship.0.clone(), self.chars[p].ship.1);
                self.plog(p, format!("refits {sn} as a {}.", SHIPS[t.min(SHIPS.len() - 1)].0.to_lowercase()));
            }
            "retire" => {
                let k = self.chars[p].realm;
                let towns: Vec<usize> = self.alive_towns_of(k).into_iter().filter(|&t| self.towns[t].port && self.towns[t].lord == NONE).collect();
                if !towns.is_empty() && self.rng.random() < 0.5 {
                    let tt = towns[0];
                    self.chars[p].pirate = false;
                    self.chars[p].profession = Some("merchant");
                    self.chars[p].gold -= 3000.0;
                    self.grant_lordship(p, tt, "");
                    let tn = self.towns[tt].name.clone();
                    let who = if self.chars[p].female { "woman" } else { "man" };
                    self.plog(p, format!("buys a royal pardon and the lordship of {tn}, and goes ashore an honest {who}."));
                    let tx = format!("The pirate {} buys a pardon and becomes {} of {tn}.", self.who(p), if self.chars[p].female { "Lady" } else { "Lord" });
                    self.say(tx, false);
                } else {
                    self.chars[p].pirate = false;
                    self.chars[p].profession = None;
                    self.plog(p, "sails off the edge of the charts with a hold full of gold, and is never seen again.".into());
                    let tx = format!("{} sails away with a fortune and is never seen again.", self.who(p));
                    self.say(tx, true);
                }
                return;
            }
            _ => {}
        }
        if self.chars[p].alive && self.chars[p].pirate {
            if self.chars[p].gold < 50.0 && (self.chars[p].has_trait("cruel") || self.chars[p].has_trait("greedy")) && self.rng.random() < 0.2 { self.mutiny(p); }
            else if self.rng.random() < 0.01 { self.plog(p, "goes down with all hands in a storm.".into()); self.kill(p, "went down with all hands in a storm".into()); }
        }
    }

    fn captured(&mut self, p: usize, k: usize) {
        let kn = self.realms[k].name.clone();
        self.plog(p, format!("is taken by the navy of {kn} and hanged at the harbour mouth."));
        let tx = format!("{}, the pirate, is taken by the navy of {kn} and hanged; the bounty was {} gold.", self.who(p), crate::economy::thousands(self.chars[p].bounty));
        let minor = self.chars[p].infamy < 300.0;
        self.say(tx, minor);
        let adm = self.realms[k].council[5];
        if adm != NONE && self.chars[adm].alive { let f = 20.0 + self.chars[p].infamy / 10.0; self.gain_fame(adm, "admiral", f); }
        self.kill(p, format!("was hanged by {kn}"));
    }

    fn mutiny(&mut self, p: usize) {
        let age = ADULT[self.chars[p].people] as i64 + self.rng.integers(3, 20);
        let realm = self.chars[p].realm;
        let q = self.make_character(realm, self.year - age, None, NONE, NONE, NONE);
        self.plog(p, "is overthrown by mutineers and marooned on a sandbar.".into());
        let survived = self.rng.random() < 0.3;
        let old = self.chars[p].ship.clone();
        self.pirates.push(q);
        let pn = self.chars[p].name.clone();
        let y = self.year;
        let hv = self.chars[p].haven;
        let qc = &mut self.chars[q];
        qc.profession = Some("pirate"); qc.pirate = true; qc.ship = old.clone(); qc.haven = hv; qc.infamy = 0.0; qc.marque = NONE;
        let qb = qc.born;
        qc.resume = vec![format!("Born year {qb}, quartermaster of {} under {pn}.", old.0), format!("Year {y}: leads the mutiny and takes {} for their own.", old.0)];
        let s = skill_ix("Seamanship");
        qc.skills[s] = (qc.skills[s] + 30.0).min(100.0);
        let tx = format!("Mutiny aboard {}: {} turns the crew on {pn}.", old.0, self.who(q));
        self.say(tx, true);
        if survived {
            self.chars[p].ship = ("a stolen fishing boat".into(), 0, 5);
            self.chars[p].gold = 0.0;
            self.plog(p, "survives the sandbar, steals a fishing boat, and swears revenge.".into());
        } else { self.kill(p, "was marooned by mutineers and died of thirst".into()); }
    }

    fn crown_pirate_king(&mut self) {
        let Some(top) = self.pirates.iter().copied().filter(|&p| self.chars[p].alive && self.chars[p].pirate).max_by(|&a, &b| self.chars[a].infamy.partial_cmp(&self.chars[b].infamy).unwrap()) else { return };
        if self.chars[top].infamy < 400.0 || self.pirate_king == top { return; }
        let old = self.pirate_king;
        self.pirate_king = top;
        if self.chars[top].epithet.is_none() { self.chars[top].epithet = Some(PIRATE_EPITHET[self.rng.integers(0, PIRATE_EPITHET.len() as i64) as usize].into()); }
        self.plog(top, "is acclaimed Pirate King by the Brethren of the havens.".into());
        self.pirate_kings.push((self.year, top));
        let in_place = if old != NONE && self.chars[old].alive { format!(", in place of {}", self.chars[old].name) } else { String::new() };
        let tx = format!("**{} is acclaimed Pirate King** by the Brethren of the havens{in_place}.", self.who(top));
        self.say(tx, false);
    }
}
