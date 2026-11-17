//! The living world (ecology.py): a food web over 6x6-league regions. Grazers share the forage, predators eat with
//! a saturating appetite, every species breeds, starves, migrates and can go extinct; people clear and hunt the land
//! they hold; new mythic breeds split off where the ley lines run strong; great alphas rise as overworld bosses.

use crate::grid::Grid;
use crate::people::NONE;
use crate::state::State;
use crate::tables::*;

pub const B: usize = 6;

#[derive(Clone)]
pub struct Species {
    pub name: String,
    pub troph: u8,
    pub suit: [f64; 9],
    pub r: f64,
    pub size: f64,
    pub mythic: bool,
    pub prey: Vec<String>,
    pub tame: Option<&'static str>,
    pub danger: f64,
    pub extinct: Option<i64>,
    pub origin: String,
    pub peak: f64,
}

pub struct Ecology {
    pub eh: usize,
    pub ew: usize,
    pub biome: Vec<usize>,
    pub mana: Vec<f64>,
    pub species: Vec<Species>,
    pub pop: Vec<Vec<f64>>,                 // [species][region]
    pub alphas: Vec<(usize, usize)>,        // (monster, species)
    pub region_names: Vec<((usize, usize), String)>,
    pub events: Vec<(i64, String)>,
}

impl Ecology {
    pub fn empty() -> Self { Ecology { eh: 0, ew: 0, biome: vec![], mana: vec![], species: vec![], pop: vec![], alphas: vec![], region_names: vec![], events: vec![] } }

    pub fn capacity(&self, i: usize, human: Option<&[f64]>) -> Vec<f64> {
        let sp = &self.species[i];
        (0..self.eh * self.ew).map(|j| {
            let b = self.biome[j];
            let mut k = sp.suit[b] * PRODUCTIVITY[b] * 100.0 / sp.size.powf(0.75) / (sp.troph as f64).powf(2.2);
            if sp.mythic { k *= (self.mana[j] * 4.0 + 0.1).clamp(0.0, 1.0) * if sp.troph == 1 { 0.25 } else { 1.0 }; }
            if let Some(h) = human { k *= 1.0 - 0.8 * h[j]; }
            k
        }).collect()
    }

    pub fn idx(&self, name: &str) -> Option<usize> { self.species.iter().position(|s| s.name == name) }

    pub fn cell(&self, pos: (usize, usize)) -> usize { (pos.0 / B).min(self.eh - 1) * self.ew + (pos.1 / B).min(self.ew - 1) }
}

impl State {
    pub fn setup_ecology(&mut self) {
        let (eh, ew) = (self.map.h / B, self.map.w / B);
        let mut biome = vec![0usize; eh * ew];
        let mut mana = vec![0.0; eh * ew];
        for by in 0..eh {
            for bx in 0..ew {
                let mut counts = [0usize; 7];
                let mut sd = 0.0;
                let mut mn = 0.0;
                for r in by * B..(by + 1) * B {
                    for c in bx * B..(bx + 1) * B {
                        counts[self.map.terrain.at(r, c) as usize] += 1;
                        sd += self.map.sea_dist.at(r, c);
                        mn += self.map.mana.at(r, c);
                    }
                }
                let n = (B * B) as f64;
                let sea_frac = counts[0] as f64 / n;
                // plains forest hills mountain swamp desert, as biomes 0-5
                let order = [PLAINS, FOREST, HILLS, MOUNTAIN, SWAMP, DESERT];
                let mut b = 0;
                for (i, &t) in order.iter().enumerate() { if counts[t as usize] > counts[order[b] as usize] { b = i; } }
                let lat = (((by as f64 + 0.5) / eh as f64) - 0.5).abs() * 2.0;
                if lat > 0.72 && b != 3 { b = 6; }
                if sea_frac > 0.6 { b = if sd / n > 5.0 { 8 } else { 7 }; }
                biome[by * ew + bx] = b;
                mana[by * ew + bx] = mn / n;
            }
        }
        let species: Vec<Species> = SPECIES.iter().map(|s| Species { name: s.0.into(), troph: s.1, suit: s.2, r: s.3, size: s.4, mythic: s.5,
            prey: s.6.iter().map(|x| x.to_string()).collect(), tame: s.7, danger: s.8, extinct: None, origin: "the first age".into(), peak: 0.0 }).collect();
        let mut eco = Ecology { eh, ew, biome, mana, species, pop: vec![], alphas: vec![], region_names: vec![], events: vec![] };
        for i in 0..eco.species.len() {
            let k = eco.capacity(i, None);
            let p: Vec<f64> = k.iter().map(|&x| x * self.rng.uniform(0.3, 0.8)).collect();
            eco.pop.push(p);
        }
        self.eco = eco;
    }

    pub fn region_name(&mut self, ey: usize, ex: usize) -> String {
        let key = (ey / 4, ex / 4);
        if let Some(n) = self.eco.region_names.iter().find(|r| r.0 == key) { return n.1.clone(); }
        let b = self.eco.biome[ey * self.eco.ew + ex];
        let n = format!("the {} {}", REGION_A[self.rng.integers(0, REGION_A.len() as i64) as usize], REGION_B[b]);
        self.eco.region_names.push((key, n.clone()));
        n
    }

    pub fn ecology_year(&mut self) {
        let (eh, ew) = (self.eco.eh, self.eco.ew);
        let nreg = eh * ew;
        let mut owned = vec![0.0; nreg];
        for by in 0..eh { for bx in 0..ew {
            let mut o = 0;
            for r in by * B..(by + 1) * B { for c in bx * B..(bx + 1) * B { if self.owner.at(r, c) >= 0 { o += 1; } } }
            owned[by * ew + bx] = o as f64 / (B * B) as f64;
        } }
        let eco = &self.eco;
        let n = eco.species.len();
        let p = &eco.pop;
        let mut new = p.clone();
        let mut graze = vec![0.0; nreg];
        let mut share = vec![0.0; nreg];
        for i in 0..n {
            let sp = &eco.species[i];
            if sp.troph != 1 { continue; }
            for j in 0..nreg {
                graze[j] += p[i][j] * sp.size.powf(0.75);
                if sp.extinct.is_none() { share[j] += sp.suit[eco.biome[j]]; }
            }
        }
        let forage: Vec<f64> = (0..nreg).map(|j| PRODUCTIVITY[eco.biome[j]] * 100.0 * (1.0 - 0.7 * owned[j]) * (share[j] + 0.5) + 1e-6).collect();
        // predation: demand, then supply (no prey loses more than two-fifths of itself)
        let mut demand: Vec<(usize, usize, Vec<f64>)> = vec![];
        for i in 0..n {
            let sp = &eco.species[i];
            if sp.extinct.is_some() || sp.troph == 1 || sp.prey.is_empty() { continue; }
            let prey: Vec<usize> = sp.prey.iter().filter_map(|x| eco.idx(x)).filter(|&q| eco.species[q].extinct.is_none()).collect();
            if prey.is_empty() { continue; }
            let a = 0.3 / sp.size.powf(0.25);
            let h = 0.4;
            // prey available in each region and what the predator eats there, once per predator (not once per prey)
            let avail: Vec<f64> = (0..nreg).map(|j| prey.iter().map(|&x| p[x][j] * eco.species[x].size).sum()).collect();
            let intake: Vec<f64> = (0..nreg).map(|j| a * p[i][j] * avail[j] / (1.0 + a * h * avail[j])).collect();
            for &q in &prey {
                let d: Vec<f64> = (0..nreg).map(|j| intake[j] * (p[q][j] * eco.species[q].size / (avail[j] + 1e-9)) / eco.species[q].size).collect();
                demand.push((i, q, d));
            }
        }
        let mut total = vec![vec![0.0; nreg]; n];
        for (_, q, d) in &demand { for j in 0..nreg { total[*q][j] += d[j]; } }
        let mut gain = vec![vec![0.0; nreg]; n];
        for (i, q, d) in &demand {
            for j in 0..nreg {
                let scale = (0.4 * p[*q][j] / (total[*q][j] + 1e-9)).min(1.0);
                let got = d[j] * scale;
                new[*q][j] -= got;
                gain[*i][j] += got * eco.species[*q].size;
            }
        }
        for i in 0..n {
            let sp = &eco.species[i];
            if sp.extinct.is_some() { continue; }
            let k = eco.capacity(i, Some(&owned));
            for j in 0..nreg {
                let kk = k[j] + 1e-6;
                if sp.troph == 1 { new[i][j] += sp.r * p[i][j] * (1.0 - 0.5 * p[i][j] / kk - 0.5 * graze[j] / forage[j]); }
                else if !sp.prey.is_empty() { new[i][j] += 0.3 * gain[i][j] / sp.size - 0.15 * sp.r * p[i][j] - 0.3 * sp.r * p[i][j] * p[i][j] / kk; }
                else { new[i][j] += sp.r * p[i][j] * (1.0 - p[i][j] / kk); }
                new[i][j] -= p[i][j] * owned[j] * if sp.troph == 1 { 0.12 } else { 0.15 };
                if new[i][j] < 0.0 { new[i][j] = 0.0; }
            }
        }
        // migration into neighbouring regions a species can live in (the map wraps, as np.roll)
        for i in 0..n {
            let sp = &eco.species[i];
            if sp.extinct.is_some() { continue; }
            let q = new[i].clone();
            let mig = if sp.troph > 1 { 0.08 } else { 0.05 };
            for y in 0..eh {
                for x in 0..ew {
                    let j = y * ew + x;
                    if sp.suit[eco.biome[j]] <= 0.0 { new[i][j] = 0.0; continue; }
                    let nb = (q[((y + eh - 1) % eh) * ew + x] + q[((y + 1) % eh) * ew + x] + q[y * ew + (x + ew - 1) % ew] + q[y * ew + (x + 1) % ew]) / 4.0;
                    new[i][j] = (1.0 - mig) * q[j] + mig * nb;
                }
            }
        }
        self.eco.pop = new;
        let y = self.year;
        for i in 0..n {
            let total: f64 = self.eco.pop[i].iter().sum();
            let sp = &mut self.eco.species[i];
            sp.peak = sp.peak.max(total);
            if sp.extinct.is_none() && sp.peak > 0.0 && total < 0.5 {
                sp.extinct = Some(y);
                let (name, minor) = (sp.name.clone(), !sp.mythic && sp.troph == 1);
                for v in self.eco.pop[i].iter_mut() { *v = 0.0; }
                self.say(format!("**An extinction.** The last {name} dies; there will be no more in the world."), minor);
                self.eco.events.push((y, format!("the {name} goes extinct")));
            }
        }
        self.speciate();
        self.towns_and_wild();
        self.alphas();
    }

    fn speciate(&mut self) {
        if self.eco.species.len() >= 90 || self.rng.random() > 0.015 { return; }
        let nreg = self.eco.eh * self.eco.ew;
        let hot: Vec<bool> = self.eco.mana.iter().map(|&m| m > 0.35).collect();
        let cands: Vec<usize> = (0..self.eco.species.len()).filter(|&i| self.eco.species[i].extinct.is_none()
            && (0..nreg).filter(|&j| hot[j]).map(|j| self.eco.pop[i][j]).sum::<f64>() > 5.0).collect();
        if cands.is_empty() { return; }
        let i = cands[self.rng.integers(0, cands.len() as i64) as usize];
        let base = self.eco.species[i].clone();
        let last = base.name.split(' ').last().unwrap().to_string();
        let mut name = None;
        for _ in 0..20 {
            let n = format!("{} {last}", VARIANT_PREFIX[self.rng.integers(0, VARIANT_PREFIX.len() as i64) as usize].to_lowercase());
            if self.eco.idx(&n).is_none() { name = Some(n); break; }
        }
        let Some(name) = name else { return };
        let mut sp = base.clone();
        for b in 0..9 { sp.suit[b] = if base.suit[b] > 0.0 { (base.suit[b] + self.rng.normal(0.0, 0.2)).clamp(0.0, 1.0) } else { 0.0 }; }
        sp.name = name.clone();
        sp.r = base.r * self.rng.uniform(0.7, 1.2);
        sp.size = base.size * self.rng.uniform(1.0, 1.6);
        sp.mythic = true;
        sp.extinct = None;
        sp.origin = format!("split from the {} in year {}", base.name, self.year);
        sp.peak = 0.0;
        sp.danger = base.danger * 1.4;
        let j = (0..nreg).max_by(|&a, &b| (self.eco.pop[i][a] * hot[a] as u8 as f64).partial_cmp(&(self.eco.pop[i][b] * hot[b] as u8 as f64)).unwrap()).unwrap();
        let mut seed = vec![0.0; nreg];
        seed[j] = (self.eco.pop[i][j] * 0.3).max(2.0);
        self.eco.species.push(sp);
        self.eco.pop.push(seed);
        let (ey, ex) = (j / self.eco.ew, j % self.eco.ew);
        let where_ = self.region_name(ey, ex);
        let t = format!("**A new breed.** In {where_}, where the ley lines run strong, the first {name}s are seen, kin to the {} and not the same.", base.name);
        self.say(t, base.troph <= 1);
        self.eco.events.push((self.year, format!("the {name} appears in {where_}")));
    }

    /// Game feeds towns; predators take from their herds.
    fn towns_and_wild(&mut self) {
        let nreg = self.eco.eh * self.eco.ew;
        let mut herb = vec![0.0; nreg];
        let mut danger = vec![0.0; nreg];
        for (i, sp) in self.eco.species.iter().enumerate() {
            for j in 0..nreg {
                if sp.troph == 1 { herb[j] += self.eco.pop[i][j] * sp.size; }
                if sp.danger > 0.0 { danger[j] += self.eco.pop[i][j] * sp.danger; }
            }
        }
        for t in 0..self.towns.len() {
            if !self.towns[t].alive { continue; }
            let j = self.eco.cell(self.towns[t].pos);
            self.towns[t].game = (herb[j] / 400.0).min(0.3);
            self.towns[t].pop *= 1.0 - (danger[j] / 20000.0).min(0.01);
        }
    }

    /// Where a dangerous species is thick, a great alpha rises: an overworld boss with a name.
    fn alphas(&mut self) {
        let gone: Vec<(usize, usize)> = self.eco.alphas.iter().copied().filter(|&(m, _)| !self.monsters[m].alive).collect();
        for (m, i) in gone {
            let j = self.monsters[m].eco_cell;
            self.eco.pop[i][j] *= 0.5;
            self.eco.alphas.retain(|a| a.0 != m);
        }
        if self.rng.random() > 0.3 { return; }
        let i = self.rng.integers(0, self.eco.species.len() as i64) as usize;
        let sp = &self.eco.species[i];
        if sp.extinct.is_some() || sp.troph < 2 || sp.danger <= 0.0 { return; }
        if self.eco.alphas.iter().filter(|a| a.1 == i).count() >= 2 { return; }
        let j = (0..self.eco.pop[i].len()).max_by(|&a, &b| self.eco.pop[i][a].partial_cmp(&self.eco.pop[i][b]).unwrap()).unwrap();
        let cap = self.eco.capacity(i, None);
        if self.eco.pop[i][j] / (cap[j] + 1e-6) < 0.6 { return; }
        let (ey, ex) = (j / self.eco.ew, j % self.eco.ew);
        let pos = (ey * B + B / 2, ex * B + B / 2);
        if self.map.terrain.at(pos.0, pos.1) == OCEAN { return; }
        let where_ = self.region_name(ey, ex);
        let sp = &self.eco.species[i];
        let base = if sp.name.to_lowercase().starts_with("great ") { sp.name[6..].to_string() } else { sp.name.clone() };
        let title = format!("the Great {} of {}", title_case(&base), where_.replacen("the ", "", 1));
        let power = 150.0 * sp.size * (1.0 + sp.troph as f64) * if sp.mythic { 1.5 } else { 1.0 };
        let (sname, troph) = (sp.name.clone(), sp.troph);
        let m = self.new_monster("beast", pos, power, Some(title.clone()), &[], true, false);
        self.monsters[m].species = Some(sname.clone());
        self.monsters[m].eco_cell = j;
        self.eco.alphas.push((m, i));
        self.say(format!("{title} rises, alpha of the {sname}s of {where_}; travellers go round."), troph < 3);
    }

    /// What kinds a tamer could find around their home.
    pub fn tameable_near(&self, c: usize) -> Vec<&'static str> {
        let home = if self.chars[c].home != NONE { self.chars[c].home } else { self.capital_of(self.chars[c].realm) };
        if home == NONE { return vec![]; }
        let (ey, ex) = ((self.towns[home].pos.0 / B).min(self.eco.eh - 1) as i64, (self.towns[home].pos.1 / B).min(self.eco.ew - 1) as i64);
        let mut out = vec![];
        for dy in -1..=1 { for dx in -1..=1 {
            let (y, x) = (ey + dy, ex + dx);
            if y < 0 || x < 0 || y >= self.eco.eh as i64 || x >= self.eco.ew as i64 { continue; }
            let j = y as usize * self.eco.ew + x as usize;
            for (i, sp) in self.eco.species.iter().enumerate() {
                if let Some(t) = sp.tame { if sp.extinct.is_none() && self.eco.pop[i][j] > 0.5 && !out.contains(&t) { out.push(t); } }
            }
        } }
        out
    }

    /// The big game within a day's flight of a place (dragons.game_near).
    pub fn game_near(&self, pos: (usize, usize), r: i64) -> f64 {
        let (cy, cx) = ((pos.0 / B).min(self.eco.eh - 1) as i64, (pos.1 / B).min(self.eco.ew - 1) as i64);
        let mut v = 0.0;
        for (i, sp) in self.eco.species.iter().enumerate().take(SPECIES.len()) {
            if !(sp.troph == 1 && sp.size >= 0.8 && sp.size < 8.0) { continue; }
            for y in (cy - r).max(0)..(cy + r + 1).min(self.eco.eh as i64) {
                for x in (cx - r).max(0)..(cx + r + 1).min(self.eco.ew as i64) { v += self.eco.pop[i][y as usize * self.eco.ew + x as usize] * sp.size; }
            }
        }
        v
    }
}

fn title_case(s: &str) -> String {
    s.split(' ').map(|w| { let mut c = w.chars(); match c.next() { Some(f) => f.to_uppercase().collect::<String>() + c.as_str(), None => String::new() } }).collect::<Vec<_>>().join(" ")
}

#[allow(dead_code)]
fn _grid_unused(_: Grid<f64>) {}
