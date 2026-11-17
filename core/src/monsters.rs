//! The monster world: monsters.py, the monster parts of world.py, dragons.py, gates.py, arcana.py and dungeons.py.
//! Monsters age, breed and leave hoards; they rise in the wild, raid towns, and are slain for soulstones, bones and
//! purses. Dragons have size, temper and lairs, take tribute and flee when beaten. The Veil frays (and in a
//! Sundering, tears). Magic writes scrolls from monster parts, binds the dead, and in the ancients, burns their life.
//! Every ruin is a dungeon with floors, mini-bosses and a boss.

use crate::grid::{edt, Grid};
use crate::people::*;
use crate::state::State;
use crate::tables::*;

const ADULT_POWER: f64 = 6000.0;
const SOVEREIGN: &str = "Wyrm-Sovereign";
const DEMON_POWER: [(u32, f64); 5] = [(3, 450.0), (4, 1100.0), (5, 2400.0), (6, 4500.0), (7, 9000.0)];
const RIFT_TIERS: [(u32, f64); 3] = [(3, 0.6), (4, 0.3), (5, 0.1)];
const GATE_TIERS: [(u32, f64); 4] = [(3, 0.45), (4, 0.35), (5, 0.15), (6, 0.05)];
const LORD_A: [&str; 7] = ["Vel", "Ashk", "Mor", "Thul", "Zag", "Oru", "Nhel"];
const LORD_B: [&str; 6] = ["goroth", "amaz", "ithrax", "uzeal", "aroth", "emnon"];
const ARTS: [&str; 7] = ["Steel Breath", "Iron Fortress Stance", "Flowing Step", "Twin Fang Strike", "Mind's-Eye Parry", "Thousand Cuts", "Heaven-Cleaver"];
const LOST_ARTS: [&str; 3] = ["The Last Stance", "Sky-Splitting Edge", "The Unbroken Wall"];
const MARTIAL_CLASSES: [&str; 8] = ["Warrior", "Knight", "Ranger", "Rogue", "Warlord", "Dragonslayer", "Spellblade", "Shadowblade"];
const PART_FOR_TIER: [usize; 11] = [0, 0, 0, 0, 0, 1, 1, 2, 2, 3, 3];   // hide hide ash ash horn horn scale scale from tier 3
const WILD_COST: f64 = 60.0;

pub struct Gates {
    pub open: bool,
    pub start: i64,
    pub end: i64,
    pub lord: usize,
    pub lord_name: String,
    pub opener: usize,
    pub count: u32,
}

pub fn mdist(a: (usize, usize), b: (usize, usize)) -> f64 { (a.0 as f64 - b.0 as f64).hypot(a.1 as f64 - b.1 as f64) }
fn mon(kind: &str) -> &'static (&'static str, i32, (f64, f64), f64, f64, f64, usize, f64, f64) { MONSTER.iter().find(|m| m.0 == kind).unwrap_or(&MONSTER[0]) }
fn part_of(kind: &str) -> Option<usize> { match kind { "troll" | "beast" => Some(0), "wraith" => Some(1), "demon" => Some(2), "dragon" | "calamity" => Some(3), _ => None } }
pub fn tier_of(power: f64) -> u32 { if power < 700.0 { 3 } else if power < 1700.0 { 4 } else if power < 3500.0 { 5 } else if power < 7000.0 { 6 } else { 7 } }
fn curve(age: f64) -> f64 {
    if age < 50.0 { 0.05 + 0.15 * age / 50.0 } else if age < 150.0 { 0.2 + 0.4 * (age - 50.0) / 100.0 } else if age < 400.0 { 0.6 + 0.4 * (age - 150.0) / 250.0 }
    else if age < 800.0 { 1.0 + (age - 400.0) / 400.0 } else { (2.0 + (age - 800.0) / 350.0).min(4.0) }
}
pub fn dragon_stage(age: i64) -> &'static str { if age < 50 { "wyrmling" } else if age < 150 { "young" } else if age < 400 { "adult" } else if age < 800 { "elder" } else { "ancient" } }
fn flee_at(t: &str) -> f64 { match t { "reclusive" => 0.5, "territorial" => 0.3, _ => 0.0 } }
fn evasion(t: &str) -> f64 { match t { "reclusive" => 0.9, "territorial" => 0.8, _ => 0.0 } }
pub fn raid_mult(t: &str) -> f64 { match t { "reclusive" => 0.3, "territorial" => 0.8, "aggressive" => 1.6, _ => 1.0 } }

impl State {
    // ------------------------------------------------------------ where things are
    pub fn town_distance(&mut self) {
        let (h, w) = (self.map.h, self.map.w);
        let mut feat = Grid::new(h, w, false);
        let mut any = false;
        for t in self.towns.iter().filter(|t| t.alive) { feat.set(t.pos.0, t.pos.1, true); any = true; }
        self.town_dist = if any { edt(&feat).0 } else { Grid::new(h, w, 1e9) };
    }

    /// Cells of a mask far from any town (world.remote_cells): a sample, kept to the remotest share.
    pub fn remote_cells(&mut self, mask: &dyn Fn(usize, usize) -> bool) -> Vec<(usize, usize)> {
        let (h, w) = (self.map.h, self.map.w);
        let mut cells: Vec<(usize, usize)> = vec![];
        for r in 0..h { for c in 0..w { if mask(r, c) { cells.push((r, c)); } } }
        if cells.len() > 4000 {
            for i in 0..4000 { let j = i + self.rng.integers(0, (cells.len() - i) as i64) as usize; cells.swap(i, j); }
            cells.truncate(4000);
        }
        if cells.is_empty() { return cells; }
        let mx = cells.iter().map(|&(r, c)| self.town_dist.at(r, c)).fold(0.0, f64::max);
        let lim = (0.8 * mx).max(3.0).min(8.0);
        cells.into_iter().filter(|&(r, c)| self.town_dist.at(r, c) >= lim).collect()
    }

    pub fn nearest_town_name(&self, pos: (usize, usize)) -> String {
        self.towns.iter().filter(|t| t.alive).min_by(|a, b| mdist(a.pos, pos).partial_cmp(&mdist(b.pos, pos)).unwrap()).map(|t| t.name.clone()).unwrap_or("nowhere".into())
    }

    fn clip(&self, r: i64, c: i64) -> (usize, usize) { (r.clamp(0, self.map.h as i64 - 1) as usize, c.clamp(0, self.map.w as i64 - 1) as usize) }

    // ------------------------------------------------------------ a monster's life
    #[allow(clippy::too_many_arguments)]
    pub fn new_monster(&mut self, kind: &'static str, pos: (usize, usize), power: f64, name: Option<String>, parents: &[usize], quiet: bool, world: bool) -> usize {
        let near = self.nearest_town_name(pos);
        let mut power = power;
        let name = match name {
            Some(n) => n,
            None => match kind {
                "dragon" => {
                    let n = format!("{} {}", self.name(&DRAGON_SYL_A, &DRAGON_SYL_B), DRAGON_EPITHET[self.rng.integers(0, DRAGON_EPITHET.len() as i64) as usize]);
                    if !quiet { self.say(format!("**A dragon.** {n} comes down out of the peaks and makes a lair near {near}."), false); }
                    n
                }
                "demon" => {
                    power = self.demon_cap(power);
                    let n = format!("{}, a demon of the {} tier", self.name(&DEMON_SYL_A, &DEMON_SYL_B), TIER_WORDS[tier_of(power) as usize]);
                    if !quiet { self.say(format!("A ley-nexus tears near {near} and {n} steps through."), false); }
                    n
                }
                "wraith" => {
                    let b = WILD_B.iter().find(|x| x.0 == SWAMP).unwrap().1;
                    let n = format!("the Wraith of the {} {}", WILD_A[self.rng.integers(0, WILD_A.len() as i64) as usize], b[self.rng.integers(0, b.len() as i64) as usize]);
                    if !quiet { self.say(format!("Something stirs in the marshes near {near}: {n} rises."), true); }
                    n
                }
                "calamity" => {
                    let n = format!("{}, {}", self.name(&CALAMITY_A, &CALAMITY_B), CALAMITY_TITLE[self.rng.integers(0, CALAMITY_TITLE.len() as i64) as usize]);
                    let where_ = self.land_name(self.map.landmass.at(pos.0, pos.1));
                    let t = if world { format!("**A world calamity wakes.** Beneath {where_}, near {near}, {n} opens its eyes, and every shore of the world feels it. No continent is safe while it lives.") }
                            else { format!("**A calamity wakes on {where_}.** Beneath the land near {near}, {n} opens its eyes. Its strength is beyond any single hero living, and the wars of {where_} falter.") };
                    if !quiet { self.say(t, false); }
                    n
                }
                _ => {
                    let t = self.map.terrain.at(pos.0, pos.1);
                    let b: &[&str] = WILD_B.iter().find(|x| x.0 == t).map(|x| x.1).unwrap_or(&["Wilds"]);
                    format!("the trolls of the {} {}", WILD_A[self.rng.integers(0, WILD_A.len() as i64) as usize], b[self.rng.integers(0, b.len() as i64) as usize])
                }
            },
        };
        let life = MONSTER_LIFE.iter().find(|x| x.0 == kind).map(|x| x.1).unwrap_or(1e6);
        let lifespan = life * self.rng.uniform(0.7, 1.2);
        let mut traits: Vec<&'static str> = vec![];
        if kind == "dragon" {
            if !parents.is_empty() {
                for &p in parents { for &t in &self.monsters[p].traits.clone() { if self.rng.random() < 0.6 && !traits.contains(&t) { traits.push(t); } } }
                if self.rng.random() < 0.1 { let t = DRAGON_TRAITS[self.rng.integers(0, 6) as usize]; if !traits.contains(&t) { traits.push(t); } }
            } else {
                traits.push(DRAGON_TRAITS[self.rng.integers(0, 4) as usize]);
                if self.rng.random() < 0.2 { let t = DRAGON_TRAITS[4 + self.rng.integers(0, 2) as usize]; if !traits.contains(&t) { traits.push(t); } }
            }
        }
        let mut lifespan = lifespan;
        if traits.contains(&"elder") { lifespan *= 1.5; }
        let mut name = name;
        if kind == "dragon" && !traits.is_empty() && parents.is_empty() {
            let mut ts = traits.clone(); ts.sort();
            name = format!("{name} ({})", ts.join(", "));
        }
        let generation = 1 + parents.iter().map(|&p| self.monsters[p].generation).max().unwrap_or(0);
        let id = self.monsters.len();
        self.monsters.push(Monster { kind, name, pos, alive: true, base: power, power, hoard: 0.0, born: self.year, sleep_until: 0, last_said: -999,
            items: vec![], landmass: self.map.landmass.at(pos.0, pos.1), world, slain_by: NONE, lifespan, adult_base: power, parents: parents.to_vec(),
            generation, children: vec![], title: None, kills: 0, traits, size: None, temper: "", settled: self.year, game0: None, hungry: 0,
            tributaries: vec![], wrath: vec![], fled: None, fled_told: None, wild_ready: 0, gate: false, crashed: false, sea: false, spilled: false,
            bounties: vec![], species: None, eco_cell: 0, cycle: NONE, sealed: false });
        id
    }

    /// Every blow lands here: a dragon about to die takes wing if it can (Monster.power, dragons.flight).
    pub fn set_power(&mut self, m: usize, value: f64) {
        let mo = &self.monsters[m];
        let old = mo.power;
        let mut value = value;
        if mo.kind == "dragon" && value < old && mo.alive && !mo.temper.is_empty() && value <= flee_at(mo.temper) * mo.base {
            let pinned = (old - value) > 0.6 * mo.base;
            let ev = evasion(mo.temper) * if pinned { 0.4 } else { 1.0 };
            if self.rng.random() < ev {
                self.monsters[m].fled = Some(self.year);
                value = value.max(0.15 * self.monsters[m].base);
            }
        }
        self.monsters[m].power = value;
    }

    pub fn hit(&mut self, m: usize, dmg: f64) { let v = self.monsters[m].power - dmg; self.set_power(m, v); }

    fn demon_cap(&self, power: f64) -> f64 { if self.gates.as_ref().is_some_and(|g| g.open) { power } else { power.min(3499.0) } }

    fn roll_tier(&mut self, table: &[(u32, f64)]) -> u32 {
        let r = self.rng.random();
        let mut acc = 0.0;
        for &(t, p) in table { acc += p; if r < acc { return t; } }
        table.last().unwrap().0
    }

    fn demon_power(t: u32) -> f64 { DEMON_POWER.iter().find(|x| x.0 == t).unwrap().1 }

    pub fn spawn_monsters(&mut self) {
        let land_n = self.map.terrain.v.iter().filter(|&&t| t != OCEAN).count().max(1);
        let wild_n = (0..self.map.h * self.map.w).filter(|&i| self.map.terrain.v[i] != OCEAN && self.owner.v[i] == -1).count();
        let wild = wild_n as f64 / land_n as f64;
        let scale = self.map.lands.iter().filter(|l| l.kind != "isle").count().max(1);
        for &(kind, terr, prange, _, _, _, cap, _, p0) in MONSTER.iter() {
            let mut n = self.monsters.iter().filter(|m| m.alive && m.kind == kind).count();
            for _ in 0..scale {
                if n >= cap * scale || self.rng.random() > p0 * (wild * 2.0 + 0.3) { continue; }
                let terrain = self.map.terrain.clone();
                let owner = self.owner.clone();
                let mana = self.map.mana.clone();
                let cells = if kind == "troll" { self.remote_cells(&|r, c| matches!(terrain.at(r, c), HILLS | FOREST) && owner.at(r, c) == -1) }
                            else if terr == -2 { self.remote_cells(&|r, c| mana.at(r, c) > 0.6) }
                            else { self.remote_cells(&|r, c| terrain.at(r, c) as i32 == terr) };
                if cells.is_empty() { continue; }
                let w: Vec<f64> = cells.iter().map(|&(r, c)| self.map.mana.at(r, c) + 0.05).collect();
                let (r, c) = cells[self.rng.weighted(&w)];
                let mut power = self.rng.uniform(prange.0, prange.1);
                if kind == "demon" {
                    let loose = self.monsters.iter().filter(|m| m.alive && m.kind == "demon" && !m.gate).count();
                    if loose >= 2 || self.rng.random() > 0.15 { continue; }
                    let t = self.roll_tier(&RIFT_TIERS);
                    power = Self::demon_power(t) * self.rng.uniform(0.85, 1.15);
                }
                if kind == "dragon" && self.monsters.iter().any(|o| o.alive && o.kind == "dragon" && mdist(o.pos, (r, c)) < 45.0) { continue; }
                self.new_monster(kind, (r, c), power, None, &[], false, false);
                n += 1;
            }
        }
        let mut settled: Vec<i32> = self.towns.iter().filter(|t| t.alive).map(|t| t.landmass).collect();
        settled.sort(); settled.dedup();
        let busy: Vec<i32> = self.monsters.iter().filter(|m| m.alive && m.kind == "calamity").map(|m| m.landmass).collect();
        for lid in settled {
            if busy.contains(&lid) || lid < 0 || self.map.lands[lid as usize].kind == "isle" { continue; }
            let last = self.last_cal.iter().find(|x| x.0 == lid).map(|x| x.1).unwrap_or(0);
            if self.year > 150 && self.year - last > 200 && self.rng.random() < 0.004 {
                let lm = self.map.landmass.clone();
                let terrain = self.map.terrain.clone();
                let cells = self.remote_cells(&|r, c| terrain.at(r, c) != OCEAN && lm.at(r, c) == lid);
                if cells.is_empty() { continue; }
                let (r, c) = cells[self.rng.integers(0, cells.len() as i64) as usize];
                let world = self.rng.random() < 0.12 && !self.monsters.iter().any(|m| m.alive && m.kind == "calamity" && m.world);
                let p = self.rng.uniform(6000.0, 20000.0) * if world { 2.0 } else { 1.0 };
                self.new_monster("calamity", (r, c), p, None, &[], false, world);
                self.last_cal.retain(|x| x.0 != lid);
                self.last_cal.push((lid, self.year));
            }
        }
    }

    pub fn monster_year(&mut self) {
        let y = self.year;
        for m in 0..self.monsters.len() {
            let mo = &self.monsters[m];
            if !mo.alive || mo.kind == "calamity" { continue; }
            let age = y - mo.born;
            if mo.kind != "dragon" && mo.base < mo.adult_base {
                let grow = 0.05 * (mo.adult_base - mo.base);
                self.monsters[m].base += grow;
                self.monsters[m].power += grow;
            }
            let mo = &self.monsters[m];
            if age as f64 > mo.lifespan { self.die_of_age(m); continue; }
            match mo.kind {
                "dragon" => self.breed_dragon(m, age),
                "troll" => if age > 15 && mo.power > 1.5 * mo.base && self.rng.random() < 0.05 { self.split_band(m) },
                "demon" => if self.rng.random() < 0.0004 { self.demon_child(m) },
                _ => {}
            }
        }
        self.crown_sovereign();
        self.dragons_year();
    }

    fn breed_dragon(&mut self, m: usize, age: i64) {
        let mo = &self.monsters[m];
        if age < 150 || mo.base < 0.8 * mo.adult_base { return; }
        let mates: Vec<usize> = (0..self.monsters.len()).filter(|&o| o != m && self.monsters[o].alive && self.monsters[o].kind == "dragon"
            && self.year - self.monsters[o].born >= 150 && mdist(self.monsters[o].pos, mo.pos) <= 40.0).collect();
        let mut p = if mates.is_empty() { 0.003 } else { 0.02 };
        if mo.traits.contains(&"brood-mother") { p *= 2.0; }
        let n = self.monsters.iter().filter(|o| o.alive && o.kind == "dragon").count();
        if n >= 7 || self.rng.random() > p { return; }
        let mate = if mates.is_empty() { NONE } else { mates[self.rng.integers(0, mates.len() as i64) as usize] };
        let parents: Vec<usize> = if mate == NONE { vec![m] } else { vec![m, mate] };
        let clutch = self.rng.integers(1, 3);
        let mut names = vec![];
        for _ in 0..clutch {
            let (pr, pc) = self.monsters[m].pos;
            let (dr, dc) = (self.rng.integers(-5, 6), self.rng.integers(-5, 6));
            let pos = self.clip(pr as i64 + dr, pc as i64 + dc);
            let adult = parents.iter().map(|&q| self.monsters[q].adult_base).sum::<f64>() / parents.len() as f64 * self.rng.uniform(0.85, 1.25);
            let name = format!("{} {}", self.name(&DRAGON_SYL_A, &DRAGON_SYL_B), DRAGON_EPITHET[self.rng.integers(0, DRAGON_EPITHET.len() as i64) as usize]);
            let kids = self.monsters[m].children.clone();
            if (0..self.monsters.len()).any(|o| self.monsters[o].alive && self.monsters[o].kind == "dragon" && o != m && !kids.contains(&o) && (mate == NONE || o != mate) && mdist(self.monsters[o].pos, pos) < 30.0) { continue; }
            let young = self.new_monster("dragon", pos, adult * 0.25, Some(name), &parents, true, false);
            self.monsters[young].adult_base = adult * if self.monsters[young].traits.contains(&"elder") { 1.3 } else { 1.0 };
            for &q in &parents { self.monsters[q].children.push(young); }
            names.push(self.monsters[young].name.clone());
        }
        if names.is_empty() { return; }
        let who = if mate != NONE { format!("{} and {}", self.monsters[m].name, self.monsters[mate].name) } else { self.monsters[m].name.clone() };
        self.say(format!("A clutch hatches in the lair of {who}: {}.", names.join(", ")), false);
    }

    fn split_band(&mut self, m: usize) {
        let (pr, pc) = self.monsters[m].pos;
        let (dr, dc) = (self.rng.integers(-6, 7), self.rng.integers(-6, 7));
        let pos = self.clip(pr as i64 + dr, pc as i64 + dc);
        self.monsters[m].power /= 2.0;
        let p = self.monsters[m].power;
        let child = self.new_monster("troll", pos, p, None, &[m], true, false);
        self.monsters[child].adult_base = self.monsters[m].adult_base;
        self.monsters[m].children.push(child);
    }

    fn demon_child(&mut self, m: usize) {
        let mp = self.monsters[m].pos;
        let Some(t) = (0..self.towns.len()).filter(|&t| self.towns[t].alive).min_by(|&a, &b| mdist(self.towns[a].pos, mp).partial_cmp(&mdist(self.towns[b].pos, mp)).unwrap()) else { return };
        if mdist(self.towns[t].pos, mp) > 25.0 { return; }
        let f = self.towns[t].realm;
        let kid = self.make_character(f, self.year, None, NONE, NONE, NONE);
        self.chars[kid].home = t;
        let g = gene("abyssblood");
        if !self.chars[kid].has(g) {
            let fx = GENETIC_FX[g];
            let c = &mut self.chars[kid];
            c.genetic.push(g);
            for i in 0..6 { c.attrs[i] += fx[i]; }
            c.prowess += fx[6]; c.talent = (c.talent + fx[7]).min(10.0); c.life *= fx[8];
        }
        let kind = PERSONALITY.iter().position(|p| *p == "kind").unwrap();
        self.chars[kid].personality.retain(|&p| p != kind);
        let who = self.monsters[m].name.split(',').next().unwrap().to_string();
        let t = format!("A child is born in {} with eyes like coals. The midwives will not say whose it is; the {} whisper the name of {who}.", self.towns[t].name, self.pname(f));
        self.say(t, false);
    }

    fn die_of_age(&mut self, m: usize) {
        self.monsters[m].alive = false;
        let kids: Vec<usize> = self.monsters[m].children.iter().copied().filter(|&o| self.monsters[o].alive).collect();
        let heir = kids.iter().copied().max_by(|&a, &b| self.monsters[a].power.partial_cmp(&self.monsters[b].power).unwrap());
        let mo = &self.monsters[m];
        if matches!(mo.kind, "dragon" | "demon") || mo.hoard > 100.0 || !mo.items.is_empty() {
            let mut tale = format!("{} {} of great age, {} years old", mo.name, if mo.kind == "troll" { "die" } else { "dies" }, self.year - mo.born);
            let (hoard, items, title, name, pos) = (mo.hoard, mo.items.clone(), mo.title, mo.name.clone(), mo.pos);
            if let Some(h) = heir {
                self.monsters[h].hoard += hoard;
                self.monsters[h].items.extend(items);
                tale += &format!("; its hoard passes to {}", self.monsters[h].name);
                if let Some(t) = title {
                    self.monsters[h].title = Some(t);
                    tale += &format!(", and with it the title of {t}");
                    self.sovereign_line.push((self.year, self.monsters[h].name.clone(), format!("inherited from {name}")));
                }
            } else {
                let mut r = Ruin::new(format!("the lair of {}", name.split(',').next().unwrap()), pos, 2 + ((hoard / 300.0) as i64).min(6), 2.5);
                let n = items.len();
                r.loot = items;
                tale += &format!("; its lair lies open now, {}{}", r.name, if n > 0 { format!(", with {n} relics of the heroes it slew") } else { String::new() });
                self.ruins.push(r);
            }
            let minor = self.monsters[m].kind != "dragon";
            self.say(format!("{tale}."), minor);
        }
        self.monsters[m].items.clear();
        self.monsters[m].hoard = 0.0;
    }

    fn crown_sovereign(&mut self) {
        let dragons: Vec<usize> = (0..self.monsters.len()).filter(|&m| self.monsters[m].alive && self.monsters[m].kind == "dragon").collect();
        if dragons.is_empty() || dragons.iter().any(|&d| self.monsters[d].title == Some(SOVEREIGN)) { return; }
        let best = *dragons.iter().max_by(|&&a, &&b| self.monsters[a].power.partial_cmp(&self.monsters[b].power).unwrap()).unwrap();
        if self.monsters[best].power < 2500.0 { return; }
        self.monsters[best].title = Some(SOVEREIGN);
        let n = self.monsters[best].name.clone();
        self.sovereign_line.push((self.year, n.clone(), "the greatest of the dragons".into()));
        self.say(format!("**{n} is the {SOVEREIGN}** now, eldest power among the dragons of the world."), false);
    }

    /// A slain monster: purses, parts and soulstones, its title to its eldest child, its relics to the slayer.
    pub fn on_monster_slain(&mut self, m: usize, slayer: usize) {
        if slayer != NONE { self.monsters[m].slain_by = slayer; }
        self.slain += 1;
        let mut people = if slayer != NONE { self.chars[slayer].realm } else { NONE };
        if people == NONE {
            let mp = self.monsters[m].pos;
            people = self.towns.iter().filter(|t| t.alive && mdist(t.pos, mp) < 25.0).min_by(|a, b| mdist(a.pos, mp).partial_cmp(&mdist(b.pos, mp)).unwrap()).map(|t| t.realm).unwrap_or(NONE);
        }
        self.on_slain(m, slayer, people);
        if self.monsters[m].kind == "dragon" { self.freed(m); }
        let kind = self.monsters[m].kind;
        if people != NONE {
            if let Some(&(_, t)) = SOULSTONE.iter().find(|s| s.0 == kind) { self.realms[people].soulstones[t] += 1; }
            if kind == "dragon" { self.realms[people].dragonbone += 2; }
        }
        let kids: Vec<usize> = self.monsters[m].children.iter().copied().filter(|&o| self.monsters[o].alive).collect();
        if let (Some(t), false) = (self.monsters[m].title, kids.is_empty()) {
            let h = *kids.iter().max_by(|&&a, &&b| self.monsters[a].power.partial_cmp(&self.monsters[b].power).unwrap()).unwrap();
            self.monsters[h].title = Some(t);
            let (hn, mn) = (self.monsters[h].name.clone(), self.monsters[m].name.clone());
            self.sovereign_line.push((self.year, hn.clone(), format!("inherited when {mn} was slain")));
            self.say(format!("{hn} takes up the title of {t} from its slain parent, and remembers who killed it."), false);
        }
        if slayer != NONE && self.chars[slayer].alive && !self.monsters[m].items.is_empty() {
            let items = std::mem::take(&mut self.monsters[m].items);
            let best = *items.iter().max_by_key(|&&i| self.items[i].rarity).unwrap();
            for it in items { self.give_item(slayer, it, true); }
            if self.items[best].rarity >= 3 {
                let t = format!("{} takes back {} from the hoard of {}.", self.who(slayer), self.items[best].label(), self.monsters[m].name);
                self.say(t, false);
            }
        }
    }

    // ------------------------------------------------------------ raids
    pub fn raids(&mut self) {
        for m in 0..self.monsters.len() {
            if !self.monsters[m].alive { continue; }
            if self.monsters[m].kind == "calamity" { self.calamity_turn(m); continue; }
            if self.monsters[m].sea { continue; }
            let (_, _, _, reach, chance, loss, _, _, _) = *mon(self.monsters[m].kind);
            let mo = &mut self.monsters[m];
            if mo.power < mo.base { mo.power += 0.05 * (mo.base - mo.power); }
            if mo.kind == "troll" && self.year - mo.born > 80 && self.rng.random() < 0.05 { self.monsters[m].alive = false; continue; }
            let mo = &self.monsters[m];
            let y = self.year;
            let angry: Vec<usize> = if mo.kind == "dragon" {
                (0..self.towns.len()).filter(|&t| self.towns[t].alive && self.towns[t].landmass == mo.landmass && mdist(self.towns[t].pos, mo.pos) <= reach
                    && mo.wrath.iter().any(|w| w.0 == self.towns[t].realm && w.1 > y)).collect()
            } else { vec![] };
            let mut mult = if angry.is_empty() { 1.0 } else { 2.0 };
            if mo.kind == "dragon" {
                if y - mo.born < 50 { continue; }
                mult *= raid_mult(mo.temper) * if mo.hungry > 0 { 2.0 } else { 1.0 };
            }
            if y < mo.sleep_until || self.rng.random() > chance * mult { continue; }
            let near: Vec<usize> = if !angry.is_empty() { angry } else {
                (0..self.towns.len()).filter(|&t| self.towns[t].alive && self.towns[t].landmass == mo.landmass && mdist(self.towns[t].pos, mo.pos) <= reach
                    && !(mo.kind == "dragon" && mo.tributaries.iter().any(|x| x.0 == self.towns[t].realm))).collect()
            };
            if near.is_empty() { continue; }
            let w: Vec<f64> = near.iter().map(|&t| self.towns[t].wealth + 10.0).collect();
            let s = near[self.rng.weighted(&w)];
            self.assault(m, s, loss);
        }
    }

    pub fn assault(&mut self, m: usize, s: usize, loss: f64) {
        let f = self.towns[s].realm;
        let kd = &self.realms[f];
        let t = &self.towns[s];
        let d = (t.garrison + t.pop * 0.15 * MARTIAL[t.culture] * (1.0 + 0.3 * t.walls as f64) * DEFENSE[t.culture] * (1.0 + 0.5 * t.magic)
                 * (1.0 + 0.12 * kd.realm_tier as f64) + t.guard) * (1.0 + (kd.scroll_power / 3000.0).min(0.5));
        let rm = self.monsters[m].power * self.rng.uniform(0.5, 1.5);
        let rd = d * self.rng.uniform(0.5, 1.5);
        self.towns[s].raided_by = m;
        let mo = &self.monsters[m];
        if mo.kind == "dragon" && mo.temper == "reclusive" && !mo.wrath.iter().any(|w| w.0 == f && w.1 > self.year) {
            let s_ = &mut self.towns[s];
            s_.granary = Some(s_.granary.unwrap_or(0.0) * 0.7);
            let x = self.rng.random();
            self.towns[s].pop *= 1.0 - 0.01 * x;
            self.monsters[m].hungry = 0;
            if self.rng.random() < 0.15 { let t = format!("{} takes the cattle of {} and is gone before the bells stop.", self.monsters[m].name, self.towns[s].name); self.say(t, true); }
            return;
        }
        if rm > rd {
            self.towns[s].raided = Some("lost");
            let g = self.towns[s].granary.unwrap_or(0.0) * 0.5;
            self.towns[s].granary = Some(g);
            let x = self.rng.uniform(0.5, 1.2);
            self.towns[s].pop *= 1.0 - loss * x;
            let stolen = self.towns[s].wealth * if self.monsters[m].kind == "dragon" { 0.5 } else { 0.2 };
            self.towns[s].wealth -= stolen;
            self.monsters[m].hoard += stolen;
            self.monsters[m].power *= 1.03;
            self.raid_shock(m, s);
            if self.monsters[m].kind == "dragon" { self.monsters[m].sleep_until = self.year + self.rng.integers(4, 15); }
            let (kind, mname) = (self.monsters[m].kind, self.monsters[m].name.clone());
            if self.towns[s].pop < 15.0 {
                let t = format!("{} of the {} is laid waste by {mname}.", self.towns[s].name, self.pname(f));
                self.say(t, false);
                self.destroy(s);
            } else if self.towns[s].capital && matches!(kind, "dragon" | "demon" | "calamity") && self.year - self.monsters[m].last_said >= 30 {
                self.monsters[m].last_said = self.year;
                let t = format!("{mname} falls upon {}, seat of {}.", self.towns[s].name, self.realms[f].name);
                self.say(t, false);
            }
            if self.towns[s].capital && matches!(kind, "dragon" | "calamity") && self.rng.random() < 0.1 {
                let r = self.realms[f].ruler;
                if r != NONE && self.chars[r].alive && !self.chars[r].undead {
                    let why = format!("was killed when {mname} fell upon {}", self.towns[s].name);
                    self.kill_by_monster(r, why, m);
                }
            }
        } else {
            self.towns[s].raided = Some("repelled");
            self.hit(m, rd * 0.25);
            if self.monsters[m].power <= 0.0 && self.monsters[m].alive {
                self.monsters[m].alive = false;
                self.on_monster_slain(m, NONE);
                let hoard = self.monsters[m].hoard;
                self.towns[s].wealth += hoard;
                let items = std::mem::take(&mut self.monsters[m].items);
                self.realms[f].treasury.extend(items);
                let (kind, mname) = (self.monsters[m].kind, self.monsters[m].name.clone());
                if kind != "troll" {
                    let t = format!("The defenders of {} ({}) bring down {mname}.", self.towns[s].name, self.pname(f));
                    self.say(t, !matches!(kind, "dragon" | "calamity"));
                }
                if kind == "calamity" { let b = self.monsters[m].born; self.calamity_log.push((mname, b, self.year, format!("brought down by the defenders of {}", self.towns[s].name))); }
            }
        }
    }

    /// A person killed by a monster: their items go into its hoard.
    pub fn kill_by_monster(&mut self, c: usize, cause: String, m: usize) {
        if !self.chars[c].alive { return; }
        let items = std::mem::take(&mut self.chars[c].items);
        if !items.is_empty() {
            let best = *items.iter().max_by_key(|&&i| self.items[i].rarity).unwrap();
            if self.items[best].rarity >= 4 { let t = format!("{} is dragged into the hoard of {}.", self.items[best].label(), self.monsters[m].name); self.say(t, true); }
            self.monsters[m].items.extend(items);
        }
        self.monsters[m].kills += 1;
        self.kill_by(c, cause, false, NONE);
    }

    fn calamity_turn(&mut self, m: usize) {
        if self.year - self.monsters[m].born > 40 && self.rng.random() < 0.03 {
            self.monsters[m].alive = false;
            let (lid, name, born) = (self.monsters[m].landmass, self.monsters[m].name.clone(), self.monsters[m].born);
            self.last_cal.retain(|x| x.0 != lid);
            self.last_cal.push((lid, self.year));
            self.calamity_log.push((name.clone(), born, self.year, "sank back into slumber, unbeaten".into()));
            self.say(format!("**{name} sinks back into the deep**, unbeaten, after {} years. It will dream, and one day it will wake.", self.year - born), false);
            self.on_slumber(m);
            return;
        }
        let mo = &mut self.monsters[m];
        mo.base += 100.0;
        if mo.power < mo.base { mo.power += 0.03 * (mo.base - mo.power); }
        let (world, lid, pos) = (mo.world, mo.landmass, mo.pos);
        let near: Vec<usize> = (0..self.towns.len()).filter(|&t| self.towns[t].alive && (world || self.towns[t].landmass == lid) && mdist(self.towns[t].pos, pos) <= 25.0).collect();
        if !near.is_empty() && self.rng.random() < 0.6 { let s = near[self.rng.integers(0, near.len() as i64) as usize]; self.assault(m, s, 0.4); }
        if self.rng.random() < 0.15 {
            let kind = ["troll", "wraith", "demon"][self.rng.integers(0, 3) as usize];
            let (dr, dc) = (self.rng.integers(-6, 7), self.rng.integers(-6, 7));
            let p = self.clip(pos.0 as i64 + dr, pos.1 as i64 + dc);
            if self.map.terrain.at(p.0, p.1) != OCEAN { let pr = mon(kind).2; let pw = self.rng.uniform(pr.0, pr.1); self.new_monster(kind, p, pw, None, &[], false, false); }
        }
        let step = if world { 40 } else { 8 };
        if self.rng.random() < if world { 0.15 } else { 0.08 } {
            let (dr, dc) = (self.rng.integers(-step, step + 1), self.rng.integers(-step, step + 1));
            let p = self.clip(pos.0 as i64 + dr, pos.1 as i64 + dc);
            let nl = self.map.landmass.at(p.0, p.1);
            if self.map.terrain.at(p.0, p.1) != OCEAN && (world || nl == lid) {
                if world && nl != lid { let t = format!("{} crosses the sea to {}.", self.monsters[m].name, self.land_name(nl)); self.say(t, false); }
                self.monsters[m].pos = p;
                self.monsters[m].landmass = nl;
            }
        }
    }

    // ------------------------------------------------------------ dragons
    fn target_power(&self, m: usize) -> f64 {
        let mo = &self.monsters[m];
        ADULT_POWER * mo.size.unwrap_or(1.0) * curve((self.year - mo.born) as f64) * if mo.traits.contains(&"elder") { 1.3 } else { 1.0 }
    }

    fn init_dragon(&mut self, m: usize) {
        let size = self.rng.uniform(0.75, 1.35);
        let r = self.rng.random();
        let temper = if r < 0.6 { "reclusive" } else if r < 0.85 { "territorial" } else { "aggressive" };
        if self.monsters[m].parents.is_empty() && self.year - self.monsters[m].born < 5 { let a = self.rng.integers(150, 450); self.monsters[m].born = self.year - a; }
        let mo = &mut self.monsters[m];
        mo.size = Some(size);
        mo.temper = temper;
        mo.adult_base = ADULT_POWER * size;
        let tp = self.target_power(m);
        self.monsters[m].base = tp;
        self.monsters[m].power = tp;
    }

    pub fn dragon_hidden(&self, m: usize) -> bool { let mo = &self.monsters[m]; mo.kind == "dragon" && mo.size.is_some() && self.year - mo.born < 50 }

    fn dragons_year(&mut self) {
        let dragons: Vec<usize> = (0..self.monsters.len()).filter(|&m| self.monsters[m].alive && self.monsters[m].kind == "dragon").collect();
        for &m in &dragons {
            if self.monsters[m].size.is_none() { self.init_dragon(m); }
            let tp = self.target_power(m);
            let mo = &mut self.monsters[m];
            mo.base = tp;
            let heal = if mo.power < mo.base { 0.12 * (mo.base - mo.power) } else { mo.base - mo.power };
            mo.power += heal;
            if let Some(f) = mo.fled {
                if (f == self.year - 1 || f == self.year) && mo.fled_told != Some(f) {
                    mo.fled_told = Some(f);
                    let st = dragon_stage(self.year - mo.born);
                    let t = format!("{}, torn and bleeding, breaks off the fight and takes wing; it will remember who hurt it.", mo.name);
                    self.say(t, !matches!(st, "adult" | "elder" | "ancient"));
                }
            }
        }
        for &m in &dragons { if !self.dragon_hidden(m) { self.settle(m, &dragons); } }
        self.tribute_year(&dragons);
    }

    fn settle(&mut self, m: usize, dragons: &[usize]) {
        let game = self.game_near(self.monsters[m].pos, 3);
        if self.monsters[m].game0.is_none() { self.monsters[m].game0 = Some(game.max(1.0)); }
        let mo = &mut self.monsters[m];
        mo.hungry = if game < 0.35 * mo.game0.unwrap() { mo.hungry + 1 } else { mo.hungry.saturating_sub(1) };
        let mo = &self.monsters[m];
        let reason = if mo.power < 0.4 * mo.base { "wounded" }
            else if self.year - mo.settled < 20 { return; }
            else if dragons.iter().any(|&o| o != m && self.monsters[o].power > 1.3 * mo.power && mdist(self.monsters[o].pos, mo.pos) < 20.0 && self.monsters[o].settled > mo.settled) { "rival" }
            else if mo.hungry >= 4 && mo.hoard < 300.0 { "hunger" } else { return };
        let lid = mo.landmass;
        let lm = self.map.landmass.clone();
        let terrain = self.map.terrain.clone();
        let cells = self.remote_cells(&|r, c| terrain.at(r, c) != OCEAN && lm.at(r, c) == lid);
        if cells.is_empty() { return; }
        let mut pick = cells;
        for i in 0..pick.len().min(40) { let j = i + self.rng.integers(0, (pick.len() - i) as i64) as usize; pick.swap(i, j); }
        pick.truncate(40);
        let score: Vec<f64> = pick.iter().map(|&p| if reason == "hunger" { self.game_near(p, 3) } else {
            dragons.iter().filter(|&&o| o != m).map(|&o| mdist(p, self.monsters[o].pos)).fold(999.0, f64::min) + self.town_dist.at(p.0, p.1)
        }).collect();
        let i = (0..pick.len()).max_by(|&a, &b| score[a].partial_cmp(&score[b]).unwrap()).unwrap();
        let old = self.nearest_town_name(self.monsters[m].pos);
        let np = pick[i];
        let g0 = self.game_near(np, 3).max(1.0);
        let mo = &mut self.monsters[m];
        mo.pos = np;
        mo.landmass = self.map.landmass.at(np.0, np.1);
        mo.settled = self.year;
        mo.game0 = Some(g0);
        mo.hungry = 0;
        mo.tributaries.clear();
        let why = match reason { "hunger" => "the herds are gone, and it follows them", "wounded" => "wounded near to death, it flies to the far peaks to heal", _ => "a greater dragon has come, and it gives way" };
        let minor = reason == "hunger" && self.monsters[m].power < 2500.0;
        let t = format!("{} leaves its lair near {old}: {why}. It settles near {}.", self.monsters[m].name, self.nearest_town_name(np));
        self.say(t, minor);
    }

    fn tribute_year(&mut self, dragons: &[usize]) {
        let y = self.year;
        let reach = mon("dragon").3;
        for &m in dragons {
            if self.dragon_hidden(m) || !self.monsters[m].alive { continue; }
            self.monsters[m].wrath.retain(|w| w.1 > y);
            let mo = &self.monsters[m];
            let mut by: Vec<(usize, Vec<usize>)> = vec![];
            for t in 0..self.towns.len() {
                let tt = &self.towns[t];
                if tt.alive && tt.landmass == mo.landmass && mdist(tt.pos, mo.pos) <= reach {
                    match by.iter_mut().find(|x| x.0 == tt.realm) { Some(x) => x.1.push(t), None => by.push((tt.realm, vec![t])) }
                }
            }
            let keep: Vec<usize> = by.iter().map(|x| x.0).collect();
            let ext: Vec<bool> = self.realms.iter().map(|k| k.extinct).collect();
            self.monsters[m].tributaries.retain(|x| keep.contains(&x.0) && !ext[x.0]);
            for (kid, towns) in by {
                let r = self.realms[kid].ruler;
                if r == NONE || self.monsters[m].wrath.iter().any(|w| w.0 == kid) { continue; }
                let realm = &self.realms[kid];
                let revenue = (realm.income * realm.tax_now).max(1.0);
                let all = self.alive_towns_of(kid).len().max(1);
                let price = (revenue * (0.1 + self.monsters[m].base / 40000.0).min(0.5) * towns.len() as f64 / all as f64).max(1.0);
                let rc = &self.chars[r];
                let p = |t: &str| rc.has_trait(t);
                if let Some(&(_, since, payer)) = self.monsters[m].tributaries.iter().find(|x| x.0 == kid) {
                    let refuse = payer != r && p("brave") && (p("wrathful") || p("arrogant")) && self.rng.random() < 0.5;
                    if self.realms[kid].gold < -revenue * 2.0 || refuse {
                        self.monsters[m].tributaries.retain(|x| x.0 != kid);
                        self.monsters[m].wrath.push((kid, y + 8));
                        let why = if refuse { format!("{} will not bow to a wyrm", self.who(r)) } else { "The treasury is empty".into() };
                        let t = format!("**{} stops paying {}.** {why}; the dragon remembers {} years of gold, and comes for it in fire.", self.realms[kid].name, self.monsters[m].name, y - since);
                        self.say(t, false);
                        continue;
                    }
                    self.realms[kid].gold -= price;
                    self.monsters[m].hoard += price;
                    let note = format!("tribute to {}", self.monsters[m].name);
                    self.books.push((kid, "dragon tribute", -price, note, NONE));
                    continue;
                }
                let tp: f64 = towns.iter().map(|&t| self.towns[t].pop).sum();
                let ap: f64 = self.alive_towns_of(kid).iter().map(|&t| self.towns[t].pop).sum();
                let danger = tp / ap.max(1.0) * (self.monsters[m].base / 2000.0) * raid_mult(self.monsters[m].temper);
                let proud = p("brave") as u8 + p("wrathful") as u8 + p("arrogant") as u8 + p("zealous") as u8;
                let wary = p("craven") as u8 + p("content") as u8 + p("calm") as u8 + p("patient") as u8 + (rc.attr(2, y) >= 10.0) as u8;
                let hit = towns.iter().any(|&t| self.towns[t].raided_by == m);
                if danger > 0.3 && wary + hit as u8 > proud && self.realms[kid].gold > price && self.rng.random() < 0.3 {
                    self.monsters[m].tributaries.push((kid, y, r));
                    self.realms[kid].gold -= price;
                    self.monsters[m].hoard += price;
                    let t = format!("**A bargain with a dragon.** {} sends a tithe of cattle and gold up the mountain to {} every year, and {} is spared its fire.",
                                    self.who(r), self.monsters[m].name, self.realms[kid].name);
                    self.say(t, false);
                    let l = self.chars[r].legitimacy.unwrap_or(50.0);
                    self.chars[r].legitimacy = Some((l - if self.realms[kid].people == 3 { 6.0 } else { 2.0 }).max(0.0));
                    let holders: Vec<usize> = self.duchies.values().filter(|d| d.ctrl == kid && d.holder != NONE && d.holder != r).map(|d| d.holder).collect();
                    for hd in holders { if self.chars[hd].has_trait("brave") || self.chars[hd].has_trait("arrogant") { self.remember(hd, r, "bowed to a wyrm", -10.0, 1.0); } }
                }
            }
        }
    }

    fn freed(&mut self, m: usize) {
        for (kid, _, _) in std::mem::take(&mut self.monsters[m].tributaries) {
            if !self.realms[kid].extinct { let t = format!("With {} dead, {} pays no more tribute to the mountain.", self.monsters[m].name, self.realms[kid].name); self.say(t, true); }
        }
    }

    // ------------------------------------------------------------ the Veil
    pub fn gates_year(&mut self) {
        let open = self.gates.as_ref().is_some_and(|g| g.open);
        if !open {
            if let Some((k, r, why)) = self.torn_by() { self.open_gates(k, r, &why); }
            return;
        }
        let (lord, end, lname) = { let g = self.gates.as_ref().unwrap(); (g.lord, g.end, g.lord_name.clone()) };
        if !self.monsters[lord].alive { self.close_gates(&format!("with {lname} slain, the gates fall silent one by one")); return; }
        if self.year >= end { self.close_gates("the Veil heals of itself, and the last gates close"); return; }
        for _ in 0..self.rng.integers(1, 4) { self.gate(); }
    }

    fn torn_by(&mut self) -> Option<(usize, usize, String)> {
        if self.year < 150 { return None; }
        for k in 0..self.realms.len() {
            if !self.active(k) { continue; }
            let r = self.realms[k].ruler;
            let mage = self.realms[k].council[3];
            let necro = |c: usize| c != NONE && self.school(&self.chars[c]) == Some(4) && self.tier(&self.chars[c]) >= 4;
            let dark = necro(r) || necro(mage) || self.chars[r].undead;
            let rc = &self.chars[r];
            if !dark || !(rc.has_trait("ambitious") || rc.has_trait("cruel") || rc.has_trait("arbitrary")) { continue; }
            let losing = rc.legitimacy.unwrap_or(50.0) < 25.0 || self.war[k].iter().filter(|&&w| w).count() >= 2;
            if losing && self.rng.random() < 0.01 {
                return Some((k, r, format!("{} of {}, desperate, tears open the Veil to save their throne", self.who(r), self.realms[k].name)));
            }
        }
        if self.monsters.iter().any(|m| m.alive && m.kind == "calamity" && m.world) && self.rng.random() < 0.004 {
            return Some((NONE, NONE, "the strain of a calamity abroad in the world tears the Veil".into()));
        }
        None
    }

    fn open_gates(&mut self, k: usize, r: usize, why: &str) {
        let terrain = self.map.terrain.clone();
        let mana = self.map.mana.clone();
        let mut cells = self.remote_cells(&|y, x| terrain.at(y, x) != OCEAN && mana.at(y, x) > 0.5);
        if cells.is_empty() { return; }
        if k != NONE {
            let cap = self.capital_of(k);
            if cap != NONE {
                let sp = self.towns[cap].pos;
                let near: Vec<(usize, usize)> = cells.iter().copied().filter(|c| (c.0 as i64 - sp.0 as i64).abs() + (c.1 as i64 - sp.1 as i64).abs() < 60).collect();
                if !near.is_empty() { cells = near; }
            }
        }
        let pos = cells[self.rng.integers(0, cells.len() as i64) as usize];
        let name = format!("{}{}, Lord of the Gates", LORD_A[self.rng.integers(0, 7) as usize], LORD_B[self.rng.integers(0, 6) as usize]);
        self.gates = Some(Gates { open: true, start: self.year, end: 0, lord: NONE, lord_name: name.clone(), opener: k, count: 0 });
        let lord = self.new_monster("demon", pos, 9000.0, Some(name.clone()), &[], false, false);
        self.monsters[lord].gate = true;
        let end = self.year + self.rng.integers(20, 41);
        let g = self.gates.as_mut().unwrap();
        g.lord = lord;
        g.end = end;
        self.say(format!("**The Sundering.** {why}. Gates open along the ley lines of every land, and {name}, a demon of the seventh tier, walks out of the first of them."), false);
        if k != NONE {
            for o in 0..self.realms.len() { if o != k && !self.realms[o].extinct { self.relation[k][o] = -100.0; self.relation[o][k] = -100.0; } }
            let l = self.chars[r].legitimacy.unwrap_or(50.0);
            self.chars[r].legitimacy = Some((l - 30.0).max(0.0));
        }
    }

    fn gate(&mut self) {
        let settled: Vec<i32> = self.towns.iter().filter(|t| t.alive).map(|t| t.landmass).collect();
        let terrain = self.map.terrain.clone();
        let mana = self.map.mana.clone();
        let lm = self.map.landmass.clone();
        let cells: Vec<(usize, usize)> = self.remote_cells(&|y, x| terrain.at(y, x) != OCEAN && mana.at(y, x) > 0.4).into_iter().filter(|&(y, x)| settled.contains(&lm.at(y, x))).collect();
        if cells.is_empty() { return; }
        let pos = cells[self.rng.integers(0, cells.len() as i64) as usize];
        let t = self.roll_tier(&GATE_TIERS);
        let p = Self::demon_power(t) * self.rng.uniform(0.85, 1.15);
        let m = self.new_monster("demon", pos, p, None, &[], true, false);
        self.monsters[m].gate = true;
        self.gates.as_mut().unwrap().count += 1;
        if t >= 5 { let tx = format!("A gate tears open near {}, and {} comes through.", self.nearest_town_name(pos), self.monsters[m].name); self.say(tx, false); }
    }

    fn close_gates(&mut self, how: &str) {
        let g = self.gates.as_mut().unwrap();
        g.open = false;
        let (s, n, c) = (g.start, g.lord_name.clone(), g.count);
        self.gate_history.push((s, self.year, n, c));
        self.say(format!("**The gates close.** After {} years and {c} gates, {how}.", self.year - s), false);
    }

    // ------------------------------------------------------------ arcana
    pub fn arcana_year(&mut self) {
        self.threat_year();
        for k in 0..self.realms.len() { if self.active(k) { self.bound_dead(k); } }
        self.scribes_year();
        self.arts_year();
        self.beyond_year();
        self.wild_magic();
    }

    fn threat_year(&mut self) {
        let y = self.year;
        let mut threat = vec![0.0; self.towns.len()];
        for m in 0..self.monsters.len() {
            let mo = &self.monsters[m];
            if !mo.alive || mo.sea { continue; }
            let reach = if mo.kind == "calamity" { 40.0 } else { 15.0 };
            let mut w = mo.power.min(4000.0) / 4000.0;
            if mo.kind == "dragon" { w *= if self.dragon_hidden(m) { 0.0 } else { raid_mult(mo.temper) * 0.6 }; }
            for (t, th) in threat.iter_mut().enumerate() {
                let tt = &self.towns[t];
                if tt.alive && tt.landmass == mo.landmass { *th += w * (-mdist(tt.pos, mo.pos) / reach).exp(); }
            }
        }
        for (t, th) in threat.into_iter().enumerate() { self.towns[t].threat = th.min(3.0); }
        for m in 0..self.monsters.len() {
            if !(self.monsters[m].alive && self.monsters[m].kind == "calamity" && !self.monsters[m].crashed) { continue; }
            self.monsters[m].crashed = true;
            let lid = self.monsters[m].landmass;
            let t = format!("**The markets of {} collapse.** With {} abroad no caravan moves, no lender lends, and the merchants bury their gold.", self.land_name(lid), self.monsters[m].name);
            self.say(t, false);
            for k in 0..self.realms.len() {
                if !self.realms[k].extinct && self.alive_towns_of(k).iter().any(|&t| self.towns[t].landmass == lid) {
                    if let Some(c) = self.realms[k].credit { self.realms[k].credit = Some((c - 15.0).max(0.0)); }
                    self.realms[k].trade_shock = y + 10;
                }
            }
        }
    }

    fn raid_shock(&mut self, m: usize, s: usize) {
        let k = self.towns[s].realm;
        self.towns[s].raided_year = self.year;
        let sp = self.towns[s].pos;
        let mut near: Vec<usize> = self.alive_towns_of(k).into_iter().filter(|&t| t != s && self.towns[t].landmass == self.towns[s].landmass).collect();
        near.sort_by(|&a, &b| mdist(self.towns[a].pos, sp).partial_cmp(&mdist(self.towns[b].pos, sp)).unwrap());
        near.truncate(3);
        let fled = self.towns[s].pop * 0.15;
        if !near.is_empty() { self.towns[s].pop -= fled; for &t in &near { self.towns[t].pop += fled / near.len() as f64; } }
        self.towns[s].stock[1] *= 0.6;
        if let Some(c) = self.towns[s].coin {
            let gold = c * if self.monsters[m].kind == "dragon" { 0.6 } else { 0.2 };
            self.towns[s].coin = Some(c - gold);
            self.monsters[m].hoard += gold;
        }
        self.realms[k].trade_shock = self.realms[k].trade_shock.max(self.year + 4);
        if let Some(c) = self.realms[k].credit { self.realms[k].credit = Some((c - 2.0).max(0.0)); }
    }

    fn on_slain(&mut self, m: usize, slayer: usize, people: usize) {
        self.pay_out(m, slayer);
        if people == NONE { return; }
        if let Some(p) = part_of(self.monsters[m].kind) { self.realms[people].parts[p] += (self.monsters[m].base / 300.0).max(1.0); }
        if slayer == NONE { return; }
        let k = people;
        let bounty = if self.realms[k].gold > 0.0 { (self.realms[k].gold * 0.5).min(self.monsters[m].base * 0.02) } else { 0.0 };
        if bounty > 0.0 { self.realms[k].gold -= bounty; self.chars[slayer].gold += bounty; }
        let hoard = self.monsters[m].hoard;
        if hoard > 1.0 {
            self.chars[slayer].gold += hoard * 0.3;
            self.realms[k].gold += hoard * 0.7;
            self.monsters[m].hoard = 0.0;
            if hoard > 50.0 {
                let t = format!("The hoard of {}, {} gold, comes back into the world; {} keeps a third.", self.monsters[m].name, crate::economy::thousands(hoard), self.who(slayer));
                self.say(t, false);
            }
        }
    }

    fn scribes_year(&mut self) {
        let mut out: Vec<(usize, usize, i64)> = vec![];
        for c in self.living.clone() {
            if !self.chars[c].alive { continue; }
            let t = self.tier(&self.chars[c]);
            if t < 1 || (self.chars[c].hero && self.rng.random() < 0.5) { continue; }
            out.push((self.chars[c].realm, c, t));
        }
        for k in 0..self.realms.len() {
            if self.realms[k].extinct { continue; }
            for v in self.realms[k].scrolls.iter_mut() { *v *= 0.85; }
        }
        for (k, c, t) in out {
            if k >= self.realms.len() || self.realms[k].extinct { continue; }
            let mut tt = t as usize;
            while tt >= 3 && self.realms[k].parts[PART_FOR_TIER[tt]] < 0.2 { tt -= 1; }
            let n = 1 + t as usize / 3;
            if tt >= 3 { self.realms[k].parts[PART_FOR_TIER[tt]] -= 0.2 * n as f64; }
            self.realms[k].scrolls[tt] += n as f64;
            self.chars[c].gold += 0.15 * 1.8f64.powi(tt as i32) * n as f64 * 0.5;
            if tt >= 7 && self.rng.random() < 0.05 {
                let part = ["hide", "ash", "horn", "scale"][PART_FOR_TIER[tt]];
                let tx = format!("{} writes a scroll of the {} tier on {part} from a slain monster.", self.who(c), TIER_WORDS[tt]);
                self.say(tx, true);
            }
        }
        for k in 0..self.realms.len() {
            if self.realms[k].extinct { continue; }
            let r = &mut self.realms[k];
            let power: f64 = r.scrolls.iter().enumerate().map(|(t, v)| v * (t + 1) as f64).sum();
            r.scroll_power = power;
            r.battle_mult *= 1.0 + (power / 2000.0).min(0.4);
            r.top_scroll = r.scrolls.iter().enumerate().filter(|x| *x.1 >= 1.0).map(|x| x.0).max().unwrap_or(0);
        }
    }

    fn bound_dead(&mut self, k: usize) {
        let r = self.realms[k].ruler;
        let mage = self.realms[k].council[3];
        let necro = |c: usize| c != NONE && self.school(&self.chars[c]) == Some(4) && self.tier(&self.chars[c]) >= 4;
        let is_necro = necro(r) || necro(mage) || self.chars[r].undead;
        let share = self.realms[k].bound_share;
        let willing = is_necro && !self.chars[r].has_trait("compassionate") && !self.chars[r].has_trait("zealous");
        if willing {
            if share == 0.0 {
                let t = format!("**The dead rise to work.** {} binds the dead of {} to the plough and the mine; they need no bread and take no wage, and the living watch them with dread.", self.who(r), self.realms[k].name);
                self.say(t, false);
            }
            self.realms[k].bound_share = (share + 0.01).min(0.15);
            for t in self.alive_towns_of(k) { if let Some(m) = self.towns[t].mood_p { self.towns[t].mood_p = Some(m - 3.0); } }
            for o in 0..self.realms.len() { if o != k && !self.realms[o].extinct && self.relation[k][o] > -60.0 { self.relation[k][o] -= 0.5; self.relation[o][k] -= 0.5; } }
        } else if share > 0.0 {
            self.realms[k].bound_share = (share - 0.03).max(0.0);
            if self.realms[k].bound_share == 0.0 { let t = format!("The bound dead of {} are laid to rest at last.", self.realms[k].name); self.say(t, true); }
        }
    }

    fn arts_year(&mut self) {
        for c in self.living.clone() {
            let cc = &self.chars[c];
            if !cc.alive || !cc.classes.iter().any(|x| MARTIAL_CLASSES.contains(&CLASSES[x.0].name)) { continue; }
            let gate = self.power_gate(cc, true);
            let n = ((cc.level / 8) as usize).min(match gate { 3 => 3, 4 => 4, 5 => 5, _ => 7 });
            if (cc.battle_arts as usize) < n {
                self.chars[c].battle_arts = n as u32;
                if n >= 5 && self.rng.random() < 0.3 { let t = format!("{} masters {}.", self.who(c), ARTS[n - 1]); self.say(t, true); }
            }
            let cc = &self.chars[c];
            if gate >= 6 && cc.battle_arts as usize == ARTS.len() && !cc.lost_art && cc.prowess >= 0.75 * PROWESS_CAP[cc.people] && self.rng.random() < 0.05 {
                self.chars[c].lost_art = true;
                let art = LOST_ARTS[self.rng.integers(0, 3) as usize];
                let t = format!("**A lost art.** {} rediscovers {art}, unseen for an age.", self.who(c));
                self.say(t, false);
            }
        }
    }

    fn beyond_year(&mut self) {
        let y = self.year;
        for c in self.living.clone() {
            let cc = &self.chars[c];
            if !cc.alive || !cc.custom || y < cc.beyond_ready { continue; }
            let t = self.tier(cc);
            if !(t >= 10 || (t >= 9 && cc.gift.is_some_and(|g| GIFTS[g] == "Spellweaver"))) { continue; }
            let k = cc.realm;
            if self.realms[k].extinct || self.rng.random() > 0.15 { continue; }
            let school = SCHOOLS[self.school(cc).unwrap()];
            let name = match school { "Arcane" => "the Fall of Heaven's Fire", "Divine" => "the Dawn That Burns the Sickness", "Nature" => "the Long Spring",
                                      "Rune" => "the Rune of Unmaking", "Necromancy" => "the Black Harvest", _ => "the Spirit Tide" };
            let towns = self.alive_towns_of(k);
            let foes: Vec<usize> = (0..self.armies.len()).filter(|&a| self.armies[a].realm != NONE && self.war[k][self.armies[a].realm]).collect();
            let biggest = foes.iter().copied().max_by(|&a, &b| self.armies[a].men.partial_cmp(&self.armies[b].men).unwrap());
            let text = match (school, biggest) {
                ("Arcane", Some(a)) => { let lost = (self.armies[a].men * 0.7).floor(); self.armies[a].men -= lost; Some(format!("works {name} upon the host of {}: {} men burn", self.realms[self.armies[a].realm].name, crate::economy::thousands(lost))) }
                ("Necromancy", Some(a)) => { let lost = (self.armies[a].men * 0.5).floor(); self.armies[a].men -= lost; self.realms[k].bound_share = (self.realms[k].bound_share + 0.05).min(0.25);
                                             Some(format!("works {name}: {} of the enemy fall, and rise again in {}'s service", crate::economy::thousands(lost), self.realms[k].name)) }
                ("Divine", _) if towns.iter().any(|&t| self.towns[t].sick_now > 0.05) => {
                    for e in self.epidemics.iter_mut() { e.front.retain(|&(t, _)| !(self.towns[t].alive && self.towns[t].realm == k)); }
                    Some(format!("works {name}: the sickness in {} is burned away", self.realms[k].name)) }
                ("Nature", _) if towns.iter().any(|&t| self.towns[t].famine > 0.0) => { self.god_harvest.retain(|g| g.0 != k); self.god_harvest.push((k, 1.4, y + 10)); Some(format!("works {name}: ten years of harvests without a failed field")) }
                ("Rune", Some(_)) => {
                    let ft = (0..self.towns.len()).filter(|&t| self.towns[t].alive && self.war[k][self.towns[t].realm]).max_by_key(|&t| self.towns[t].walls);
                    ft.map(|t| { self.towns[t].walls = 0; format!("works {name}: the walls of {} fall to dust", self.towns[t].name) }) }
                ("Spirit", f) if f.is_some() || self.monsters.iter().any(|m| m.alive && m.kind == "calamity") => { self.realms[k].battle_mult *= 2.0; Some(format!("works {name}: the spirits of the land march with {}'s hosts", self.realms[k].name)) }
                _ => None,
            };
            if let Some(tx) = text {
                self.chars[c].beyond_ready = y + 40;
                self.chars[c].renown += 300.0;
                let t = format!("**Beyond the tenth tier.** {} {tx}.", self.who(c));
                self.say(t, false);
            }
        }
    }

    fn wild_magic(&mut self) {
        for m in 0..self.monsters.len() {
            let mo = &self.monsters[m];
            if !mo.alive || mo.kind != "dragon" || self.year - mo.born < 300 { continue; }
            let hurt = mo.power < 0.6 * mo.base;
            if self.rng.random() > if hurt { 0.03 } else { 0.002 } || self.year < mo.wild_ready { continue; }
            let (lid, pos) = (mo.landmass, mo.pos);
            let Some(t) = (0..self.towns.len()).filter(|&t| self.towns[t].alive && self.towns[t].landmass == lid).min_by(|&a, &b| mdist(self.towns[a].pos, pos).partial_cmp(&mdist(self.towns[b].pos, pos)).unwrap()) else { continue };
            self.monsters[m].wild_ready = self.year + 50;
            self.monsters[m].lifespan -= WILD_COST;
            let what = if hurt { let b = self.monsters[m].base; self.monsters[m].power = self.monsters[m].power.max(0.9 * b); "knits its torn body whole".to_string() }
                else if self.rng.random() < 0.5 { let n = self.monsters[m].name.clone(); let tp = self.towns[t].pos; self.curse_land(tp, lid, &n, 6.0); format!("withers the land around {}", self.towns[t].name) }
                else { self.towns[t].pop *= 0.7; self.towns[t].walls = self.towns[t].walls.saturating_sub(1); format!("lays a blight of fire and ash on {}", self.towns[t].name) };
            let left = self.monsters[m].lifespan - (self.year - self.monsters[m].born) as f64;
            let t = format!("**Wild magic.** {}, ancient beyond the reckoning of the tiers, speaks a word in the old tongue and {what}; it pays with {WILD_COST} years of its life{}",
                            self.monsters[m].name, if left < 60.0 { ", and has little left to give." } else { "." });
            self.say(t, false);
        }
    }

    // ------------------------------------------------------------ dungeons
    pub fn make_item(&mut self, rarity: usize, origin: String) -> usize {
        let mut rarity = rarity;
        if rarity >= 6 {
            let free: Vec<usize> = (0..WORLDFORGED.len()).filter(|&i| !self.item_names.contains(WORLDFORGED[i].0)).collect();
            if free.is_empty() { rarity = 5; } else {
                let (name, special, note) = WORLDFORGED[free[self.rng.integers(0, free.len() as i64) as usize]];
                self.item_names.insert(name.to_string());
                let mut it = Item::new(name.into(), 6, self.year, origin);
                it.special = Some(special);
                it.note = note.into();
                return self.new_item(it);
            }
        }
        let name = self.master_name();
        self.new_item(Item::new(name, rarity, self.year, origin))
    }

    pub fn roll_rarity(&mut self, depth: f64) -> usize {
        let e = -self.rng.random().max(1e-12).ln() * 0.9;
        ((e + depth * 0.6).floor().clamp(0.0, 5.0)) as usize
    }

    pub fn ensure_dungeon(&mut self, r: usize) {
        if self.ruins[r].stocked { return; }
        let n = self.ruins[r].name.clone();
        let h = n.bytes().fold(0u64, |a, b| a.wrapping_mul(31).wrapping_add(b as u64)) as usize;
        let kind = if n.contains("lair of") { "lair" } else if n.contains("hoard") { "cave" }
            else if n.starts_with("the ruins of") && self.ruins[r].depth >= 2.0 { ["crypt", "temple", "tower", "fortress"][h % 4] }
            else { ["crypt", "fortress", "mine", "cave"][h % 4] };
        let grand = self.ruins[r].depth >= 2.4 || ["the Deep", "the Dark", "the Howling", "the Sunken", "the Black", "the Echo"].iter().any(|p| n.starts_with(p))
            || !["the ruins of", "the lair", "the buried"].iter().any(|p| n.starts_with(p));
        let floors = if grand { self.rng.integers(5, 13) } else { self.rng.integers(1, 3) } as usize;
        let ru = &mut self.ruins[r];
        ru.kind = kind; ru.grand = grand; ru.floors = floors; ru.floor = 0; ru.conquered = None; ru.stocked = true;
        self.stock(r);
    }

    fn stock(&mut self, r: usize) {
        let (bosses, minis) = DUNGEON_KINDS.iter().find(|k| k.0 == self.ruins[r].kind).map(|k| (k.1, k.2)).unwrap();
        let (floors, grand, depth) = (self.ruins[r].floors, self.ruins[r].grand, self.ruins[r].depth);
        let mut ms = vec![];
        for f in 1..floors {
            if self.rng.random() < if grand { 0.35 } else { 0.0 } { ms.push((f, minis[self.rng.integers(0, minis.len() as i64) as usize].to_string(), 120.0 * depth.powf(1.5) * (1.0 + f as f64 * 0.4))); }
        }
        let pool = if grand { bosses } else { minis };
        let mut name = pool[self.rng.integers(0, pool.len() as i64) as usize].to_string();
        if name == "the Mad Archmage" { name = format!("the Mad Archmage {}", self.name(&["Vel", "Mor", "Qui", "Zan"], &["ithar", "oc", "emus", "arro"])); }
        let ru = &mut self.ruins[r];
        ru.minis = ms;
        ru.boss = (name, 200.0 * depth.powf(1.6) * (1.0 + floors as f64 * 0.35) * if grand { 2.5 } else { 1.0 }, !grand);
        ru.riches = ru.riches.max(floors as i64);
    }

    /// One year below: the next floor (or its guardian), or the boss at the bottom (dungeons.delve).
    pub fn delve(&mut self, h: usize, r: usize) {
        self.ensure_dungeon(r);
        if self.ruins[r].conquered.is_some() {
            if !self.ruins[r].loot.is_empty() {
                let it = self.ruins[r].loot.remove(0);
                self.give_item(h, it, true);
                let t = format!("{} finds {} in {}.", self.who(h), self.items[it].label(), self.ruins[r].name);
                let minor = self.items[it].rarity < 4;
                self.say(t, minor);
            }
            return;
        }
        let nxt = self.ruins[r].floor + 1;
        let cr = self.cr(&self.chars[h], true) * self.mods(&self.chars[h])[6];
        self.use_skill(h, crate::power::skill_ix("Sneak"), 0.8);
        self.use_skill(h, crate::power::skill_ix("Security"), 0.8);
        let depth = self.ruins[r].depth;
        if nxt < self.ruins[r].floors {
            let guard = self.ruins[r].minis.iter().find(|m| m.0 == nxt).cloned();
            let foe = guard.as_ref().map(|g| g.2).unwrap_or(80.0 * depth.powf(1.5) * (1.0 + nxt as f64 * 0.3));
            if self.rng.random() < cr / (cr + foe) {
                self.ruins[r].floor = nxt;
                self.deed(h, "delve");
                self.gain_xp(h, foe);
                self.chars[h].gold += 30.0 * depth * nxt as f64;
                self.renown(h, 2.0 + nxt as f64 + if guard.is_some() { 8.0 } else { 0.0 });
                if let Some(g) = &guard {
                    let t = format!("{} kills {} on the {nxt}th floor of {}.", self.who(h), g.1, self.ruins[r].name);
                    self.say(t, true);
                    self.ruins[r].minis.retain(|m| m.0 != nxt);
                }
                if self.rng.random() < 0.12 + 0.03 * nxt as f64 + if guard.is_some() { 0.3 } else { 0.0 } {
                    let rar = self.roll_rarity(depth + nxt as f64 * 0.3);
                    let it = self.make_item(rar, format!("floor {nxt} of {}", self.ruins[r].name));
                    self.give_item(h, it, false);
                }
            } else {
                let dmg = self.rng.uniform(20.0, 70.0) * (1.0 + nxt as f64 * 0.1);
                let why = format!("died on the {nxt}th floor of {}{}", self.ruins[r].name, guard.map(|g| format!(", killed by {}", g.1)).unwrap_or_default());
                self.hurt(h, dmg, why, NONE, NONE);
            }
            return;
        }
        let (bname, bpow, bmini) = self.ruins[r].boss.clone();
        if self.rng.random() < cr / (cr + 0.7 * bpow) {
            let floors = self.ruins[r].floors;
            self.deed(h, "delve");
            self.gain_xp(h, bpow);
            self.chars[h].gold += 200.0 * depth * floors as f64;
            self.renown(h, if bmini { 15.0 } else { 60.0 } + 5.0 * floors as f64);
            let grand = self.ruins[r].grand;
            let rar = (self.roll_rarity(depth + floors as f64 * 0.4) + grand as usize).min(5);
            let it = self.make_item(rar, format!("the hoard of {bname}"));
            self.give_item(h, it, false);
            for extra in std::mem::take(&mut self.ruins[r].loot) { self.give_item(h, extra, true); }
            let y = self.year;
            let re = y + self.rng.integers(40, 150);
            let ru = &mut self.ruins[r];
            ru.conquered = Some(y);
            ru.conquests.push((y, h, bname.clone()));
            ru.riches = 0;
            ru.reinfest = re;
            let rn = ru.name.clone();
            self.dungeon_log.push((y, h, rn.clone(), bname.clone(), floors));
            let b = if grand { "**" } else { "" };
            let t = format!("{b}{} reaches the bottom of {rn}, {floors} floors down, and kills {bname}{b}.", self.who(h));
            self.say(t, !grand);
        } else {
            let dmg = self.rng.uniform(40.0, 110.0);
            let why = format!("was killed by {bname} at the bottom of {}", self.ruins[r].name);
            self.hurt(h, dmg, why, NONE, NONE);
        }
    }

    pub fn dungeons_year(&mut self) {
        for r in 0..self.ruins.len() {
            self.ensure_dungeon(r);
            if self.ruins[r].conquered.is_some() && self.year >= self.ruins[r].reinfest {
                let ru = &mut self.ruins[r];
                ru.conquered = None; ru.floor = 0; ru.depth = (ru.depth + 0.3).min(6.0);
                self.stock(r);
                let t = format!("Something new has moved into {}: {} rules its depths now.", self.ruins[r].name, self.ruins[r].boss.0);
                let minor = !self.ruins[r].grand;
                self.say(t, minor);
            }
            if self.ruins[r].grand && self.ruins[r].conquered.is_none() && self.rng.random() < 0.002 && self.monsters.iter().filter(|m| m.alive && m.spilled).count() < 60 {
                let kind = ["troll", "wraith", "demon"][self.rng.integers(0, 3) as usize];
                let pr = mon(kind).2;
                let p = self.rng.uniform(pr.0, pr.1) * (1.0 + self.ruins[r].depth / 4.0);
                let m = self.new_monster(kind, self.ruins[r].pos, p, None, &[], true, false);
                self.monsters[m].name = format!("{}, out of {}", self.monsters[m].name, self.ruins[r].name);
                self.monsters[m].spilled = true;
            }
        }
        if self.rng.random() < 0.02 {
            let terrain = self.map.terrain.clone();
            let owner = self.owner.clone();
            let cells = self.remote_cells(&|y, x| terrain.at(y, x) != OCEAN && owner.at(y, x) == -1);
            if !cells.is_empty() {
                let pos = cells[self.rng.integers(0, cells.len() as i64) as usize];
                let name = format!("the {}", self.name(&["Deep", "Dark", "Howling", "Sunken", "Black", "Echo"], &["delve", "maw", "hollow", "warren", "pit", "gate"]));
                let riches = self.rng.integers(4, 9);
                let depth = self.rng.uniform(2.0, 4.5);
                self.ruins.push(Ruin::new(name, pos, riches, depth));
                let r = self.ruins.len() - 1;
                self.ensure_dungeon(r);
                let t = format!("A way into the earth opens on {}: {}, {} floors deep, and {} waits at the bottom.", self.land_name(self.map.landmass.at(pos.0, pos.1)),
                                self.ruins[r].name, self.ruins[r].floors, self.ruins[r].boss.0);
                self.say(t, true);
            }
        }
    }
}
