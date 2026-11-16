//! The land's year (economy.py and the labour half of markets.py): the harvest, grain moved within a realm, eating
//! and storing, famine and flight, then work, skill, income, the classes and their poverty, and new towns.
//!
//! Systems that later slices bring (culture innovations, buildings, masters, trade, garrisons, monsters' threat,
//! the bound dead, occupation) enter here at their year-0 values: none yet.

use crate::state::State;
use crate::tables::*;

const VITAL: [(f64, f64); 4] = [(0.040, 0.035), (0.014, 0.010), (0.022, 0.018), (0.050, 0.043)];
const GRANARY_YEARS: f64 = 1.0;
// work: farm mine craft trade clergy service arcane
const VALUE: [f64; 7] = [1.0, 2.0, 2.2, 2.6, 0.6, 0.9, 3.2];
const IDLE_VALUE: f64 = 0.25;
const UNIT: f64 = 0.0075;
const WORKING: f64 = 0.55;
const PEOPLE_SKILL: [[f64; 7]; 4] = [
    [0.45, 0.35, 0.40, 0.50, 0.45, 0.45, 0.40],
    [0.40, 0.25, 0.55, 0.40, 0.55, 0.40, 0.70],
    [0.30, 0.60, 0.60, 0.45, 0.40, 0.35, 0.45],
    [0.35, 0.40, 0.30, 0.25, 0.30, 0.30, 0.30],
];
const MINERAL: [&str; 11] = ["stone", "copper", "iron", "gold", "silver", "gems", "mithril", "adamantine ore", "salt",
                             "dragonbone fossils", "mana crystals"];
const MATERIAL: [&str; 10] = ["iron", "copper", "timber", "mithril", "adamantine ore", "ancient wood", "furs", "silk",
                              "ivory", "amber"];
const SUBSIST: f64 = 0.0030;
// farm mine craft trade clergy service arcane
const INNOV_SKILL: [(&str, [f64; 7]); 17] = [
    ("Hunting Grounds", [0.03, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0]), ("Ironworking", [0.0, 0.05, 0.05, 0.0, 0.0, 0.0, 0.0]),
    ("Two-Field Rotation", [0.06, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0]), ("Coinage", [0.0, 0.0, 0.0, 0.06, 0.0, 0.05, 0.0]),
    ("Monastery Schools", [0.03; 7]), ("Steel Forging", [0.0, 0.0, 0.07, 0.0, 0.0, 0.0, 0.0]),
    ("Sea Charts", [0.0, 0.0, 0.0, 0.04, 0.0, 0.0, 0.0]), ("Three-Field Rotation", [0.07, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0]),
    ("Guild Charters", [0.0, 0.0, 0.10, 0.06, 0.0, 0.0, 0.0]), ("Universities", [0.05; 7]),
    ("Silversteel Craft", [0.0, 0.05, 0.05, 0.0, 0.0, 0.0, 0.0]), ("Heavy Plough", [0.08, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0]),
    ("Banking Houses", [0.0, 0.0, 0.0, 0.10, 0.0, 0.0, 0.0]), ("Printed Sagas", [0.0, 0.0, 0.0, 0.03, 0.04, 0.0, 0.0]),
    ("Adamant Forging", [0.0, 0.0, 0.05, 0.0, 0.0, 0.0, 0.0]), ("Royal Chancery", [0.0, 0.0, 0.0, 0.0, 0.0, 0.05, 0.0]),
    ("Bureaucracy", [0.0, 0.0, 0.0, 0.0, 0.0, 0.05, 0.0]),
];
const SAVE: [f64; 3] = [0.02, 0.12, 0.2];
const SIGMA: [f64; 3] = [0.5, 0.6, 0.8];
const TAX_SHARE: [f64; 3] = [0.8, 1.0, 0.4];

fn rent(gov: &str) -> f64 {
    match gov { "tribal" => 0.15, "clan" => 0.2, "feudal" => 0.4, "republic" => 0.3, "theocracy" => 0.35, _ => 0.3 }
}

/// The error function (Numerical Recipes' erfc approximation, accurate to about 1e-7).
fn erf(x: f64) -> f64 {
    let t = 1.0 / (1.0 + 0.5 * x.abs());
    let y = 1.0 - t * (-x * x - 1.26551223 + t * (1.00002368 + t * (0.37409196 + t * (0.09678418 + t * (-0.18628806
        + t * (0.27886807 + t * (-1.13520398 + t * (1.48851587 + t * (-0.82215223 + t * 0.17087277))))))))).exp();
    if x >= 0.0 { y } else { -y }
}

pub fn poor_share(line: f64, mean: f64, sigma: f64) -> f64 {
    if mean <= 0.0 { return 1.0; }
    let z = ((line / mean).ln() + sigma * sigma / 2.0) / sigma;
    0.5 * (1.0 + erf(z / std::f64::consts::SQRT_2))
}

impl State {
    /// The year for every town's land: harvest, grain, people, then work and its fruits (world.economy).
    pub fn land_year(&mut self) {
        let n = self.towns.len();
        let (mut food, mut wealth, mut mana, mut cells) = (vec![0.0; n], vec![0.0; n], vec![0.0; n], vec![0usize; n]);
        let mut res_count: Vec<Vec<(usize, u32)>> = vec![vec![]; n];
        for i in 0..self.map.h * self.map.w {
            let o = self.owner.v[i];
            if o < 0 { continue; }
            let o = o as usize;
            let cu = self.towns[o].culture;
            let t = self.map.terrain.v[i] as usize;
            food[o] += FOOD[cu][t];
            wealth[o] += WEALTH[cu][t];
            mana[o] += self.map.mana.v[i];
            cells[o] += 1;
            let r = self.res_grid.v[i];
            if r >= 0 {
                match res_count[o].iter_mut().find(|x| x.0 == r as usize) { Some(x) => x.1 += 1, None => res_count[o].push((r as usize, 1)) }
            }
        }
        self.town_year(&food, &mana, &cells, &res_count);
        self.labour_year(&wealth, &res_count);
    }

    fn town_year(&mut self, food: &[f64], mana: &[f64], cells: &[usize], res: &[Vec<(usize, u32)>]) {
        let live: Vec<usize> = (0..self.towns.len()).filter(|&i| self.towns[i].alive).collect();
        // 1. the harvest
        let food_tech: Vec<f64> = self.cultures.iter().map(|c| c.total("food")).collect();
        let rest_bonus: Vec<f64> = self.cultures.iter().map(|c| 0.006 * c.innovations.contains("Two-Field Rotation") as u8 as f64
            + 0.01 * c.innovations.contains("Three-Field Rotation") as u8 as f64 + 0.004 * c.innovations.contains("Hunting Grounds") as u8 as f64).collect();
        for &i in &live {
            let realm_tier = 0.0;
            let s = &mut self.towns[i];
            s.cells = cells[i];
            s.magic = MAGIC[s.culture] * (mana[i] / 5.0).min(1.0) * (1.0 + 0.1 * realm_tier);
            let rf = (0.05 * res[i].iter().filter(|x| RESOURCES[x.0].effect == "food").map(|x| x.1 as f64).sum::<f64>()).min(0.4);
            s.res_food = rf;
            s.bullion = res[i].iter().map(|x| x.1 as f64 * match RESOURCES[x.0].name { "gold" => 0.6, "silver" => 0.25, _ => 0.0 }).sum();
            s.res_wealth = 1.5 * res[i].iter().filter(|x| matches!(RESOURCES[x.0].effect, "wealth" | "treasury")).map(|x| x.1 as f64).sum::<f64>();
            let tech = 1.0 + food_tech[s.culture];
            let magic = 1.0 + 0.15 * s.magic + 0.01 * realm_tier;
            let base = food[i] * 25.0;
            // soil: worn by too many mouths, rested by fallow (climate.soil_year)
            let press = s.pop / (base * tech).max(1.0);
            let wear = 0.008 * (press - 0.6).max(0.0);
            let rest = 0.01 + rest_bonus[s.culture];
            s.soil = (s.soil - wear + rest * (1.05 - s.soil)).clamp(0.5, 1.1);
            let harvest = self.climate.at(&self.climate.harvest, s.pos);
            let mut prod = base * s.soil * harvest * tech * magic * (1.0 + rf);
            prod *= 0.55 + 0.45 * s.farm_fill.min(1.0);
            s.food_prod = prod;
            s.food = prod;
        }
        // 2. grain moves from plenty to want, inside a realm and over land, as far as its carts and clerks reach
        let mut groups: Vec<((usize, i32), Vec<usize>)> = Vec::new();
        for &i in &live {
            let key = (self.towns[i].realm, self.towns[i].landmass);
            match groups.iter_mut().find(|g| g.0 == key) { Some(g) => g.1.push(i), None => groups.push((key, vec![i])) }
        }
        for (key, towns) in &groups {
            if towns.len() < 2 { continue; }
            let inn = &self.cultures[self.realms[key.0].people].innovations;
            let reach = (0.15 + 0.2 * inn.contains("Coinage") as u8 as f64 + 0.15 * inn.contains("Royal Chancery") as u8 as f64
                         + 0.1 * inn.contains("Bureaucracy") as u8 as f64).min(0.6);
            let spare: Vec<(usize, f64)> = towns.iter().filter(|&&t| self.towns[t].food > self.towns[t].pop)
                .map(|&t| (t, self.towns[t].food - self.towns[t].pop)).collect();
            let want: Vec<(usize, f64)> = towns.iter().map(|&t| {
                let s = &self.towns[t];
                (t, s.pop - s.food - s.granary.unwrap_or(0.0))
            }).filter(|x| x.1 > 0.0).collect();
            if spare.is_empty() || want.is_empty() { continue; }
            let have = spare.iter().map(|x| x.1).sum::<f64>() * reach;
            let need: f64 = want.iter().map(|x| x.1).sum();
            let moved = have.min(need);
            if moved <= 0.0 { continue; }
            for &(t, v) in &spare { self.towns[t].food -= moved * v / (have / reach); }
            for &(t, w) in &want { self.towns[t].food += moved * w / need; }
            self.grain_moved += moved;
        }
        // 3. eat, store, and count the cost
        let mut famine_towns = Vec::new();
        for &i in &live {
            let rain = self.climate.at(&self.climate.p, self.towns[i].pos);
            let growth = self.realms[self.towns[i].realm].growth_mult;
            let noise = self.rng.normal(0.0, 0.005);
            let s = &mut self.towns[i];
            let (eat, need) = (s.food, s.pop);
            let mut g = s.granary.unwrap_or(need * 0.3);
            g *= 0.95 * (1.0 - 0.08 * (rain - 1.1).max(0.0));
            let mut short;
            if eat >= need {
                g = (need * GRANARY_YEARS).min(g + (eat - need));
                short = 0.0;
            } else {
                let draw = g.min(need - eat);
                g -= draw;
                short = (need - eat - draw) / need.max(1.0);
            }
            short *= 1.0 - (15.0 * s.arcane).min(0.25);
            s.granary = Some(g);
            s.famine = short;
            let (mut b, d) = VITAL[s.culture];
            let ratio = eat / need.max(1.0);
            b *= (1.0 + 0.3 * ((ratio - 1.0) / 2.0).clamp(0.0, 1.0)) * ((ratio - 0.95) / 0.45).clamp(0.15, 1.2);
            s.pop += s.pop * (b - d) * growth * s.lord_mod;
            s.hunger_dead = 0.0;
            if short > 0.0 {
                let dead = s.pop * short * 0.5;
                s.pop -= dead;
                s.hunger_dead = dead;
                famine_towns.push(i);
            }
            s.pop *= 1.0 + noise;
            s.pop = s.pop.max(1.0);
        }
        self.famine_news(&famine_towns);
        self.migrate(&famine_towns);
    }

    fn famine_news(&mut self, towns: &[usize]) {
        let dead: f64 = towns.iter().map(|&t| self.towns[t].hunger_dead).sum();
        if towns.is_empty() || dead < 1000.0 { return; }
        let worst = *towns.iter().max_by(|&&a, &&b| self.towns[a].famine.partial_cmp(&self.towns[b].famine).unwrap()).unwrap();
        let mut lands: Vec<i32> = towns.iter().map(|&t| self.towns[t].landmass).collect();
        lands.sort();
        lands.dedup();
        let names: Vec<String> = lands.iter().map(|&l| if l >= 0 { self.map.lands[l as usize].name.clone() } else { "the open sea".into() }).collect();
        let lead = if dead > 3000.0 { "**Famine.** " } else { "" };
        let text = format!("{lead}The harvest fails across {}; {} towns go hungry and some {} starve, worst in {}.",
                           names.join(", "), towns.len(), thousands(dead), self.towns[worst].name);
        self.say(text, dead < 3000.0);
        self.famines.push((self.year, towns.len(), dead));
    }

    /// The hungry leave for the nearest towns that can feed them.
    fn migrate(&mut self, hungry: &[usize]) {
        if hungry.is_empty() { return; }
        let fed: Vec<usize> = (0..self.towns.len()).filter(|&t| self.towns[t].alive && self.towns[t].famine == 0.0).collect();
        if fed.is_empty() { return; }
        for &s in hungry {
            let (pos, land) = (self.towns[s].pos, self.towns[s].landmass);
            let n = self.towns[s].pop * (0.04 + 0.3 * self.towns[s].famine);
            let mut by: Vec<(f64, usize)> = fed.iter().map(|&t| {
                let p = self.towns[t].pos;
                ((p.0 as f64 - pos.0 as f64).hypot(p.1 as f64 - pos.1 as f64), t)
            }).collect();
            by.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
            let near: Vec<usize> = by.iter().take(3).filter(|x| x.0 < 25.0 && self.towns[x.1].landmass == land).map(|x| x.1).collect();
            if near.is_empty() { continue; }
            self.towns[s].pop -= n;
            for &t in &near { self.towns[t].pop += n / near.len() as f64; }
            self.migrants += n;
        }
    }

    /// Jobs, workers, skill and output for every town; income, wealth, walls, the classes; the idle move to work.
    fn labour_year(&mut self, terr_wealth: &[f64], res: &[Vec<(usize, u32)>]) {
        let walls_caps: Vec<u32> = (0..self.realms.len()).map(|k| self.walls_cap(k)).collect();
        // what each people's learning makes of each trade (markets.culture_targets)
        let targets: Vec<[f64; 7]> = (0..4).map(|p| {
            let mut t = PEOPLE_SKILL[p];
            for &(n, fx) in INNOV_SKILL.iter() {
                if self.cultures[p].innovations.contains(n) { for j in 0..7 { t[j] += fx[j]; } }
            }
            t
        }).collect();
        for i in 0..self.towns.len() {
            if !self.towns[i].alive { continue; }
            let k = self.towns[i].realm;
            let inn = &self.cultures[self.towns[i].culture].innovations;
            let st = if inn.contains("Banking Houses") { 2.0 } else if inn.contains("Coinage") { 1.0 } else { 0.0 };
            let sea = if inn.contains("Galleon Yards") { 3.0 } else if inn.contains("Carrack Hulls") || inn.contains("Sea Charts") { 2.0 }
                      else if inn.contains("Longships") { 1.0 } else { 0.0 };
            let saving = 0.04 * inn.contains("Two-Field Rotation") as u8 as f64 + 0.05 * inn.contains("Three-Field Rotation") as u8 as f64
                         + 0.06 * inn.contains("Heavy Plough") as u8 as f64;
            let guild = inn.contains("Guild Charters") as u8 as f64;
            let mats: f64 = res[i].iter().filter(|x| MATERIAL.contains(&RESOURCES[x.0].name)).map(|x| x.1 as f64).sum();
            let mins: f64 = res[i].iter().filter(|x| MINERAL.contains(&RESOURCES[x.0].name)).map(|x| x.1 as f64).sum();
            let gov = self.realms[k].gov;
            let tax = self.realms[k].tax_now;
            let boost = self.realms[k].sector_boost;
            let year = self.year;
            let s = &mut self.towns[i];
            let working = WORKING * s.pop;
            let soldiers = (working * 0.08).min(0.25 * s.garrison);
            let b = s.bld;
            let bl = |i: usize| b[i] as f64;
            let free = working - soldiers;
            let culture = s.culture;
            use crate::stewardship::*;
            let jobs = [
                0.78 * WORKING * s.pop * (s.food_prod / s.pop.max(1.0)).clamp(0.9, 1.05) * (1.0 - saving) * (1.0 - 0.02 * bl(B_MILL)),
                25.0 * mins + if mins > 0.0 { 30.0 * bl(B_MINE) } else { 0.0 },
                s.pop * (0.03 + 0.04 * guild + 0.02 * (st >= 1.0) as u8 as f64 + 0.015 * bl(B_WORKSHOPS)) + 25.0 * mats,
                40.0 * s.port as u8 as f64 * sea + 0.02 * s.pop * st + 0.01 * s.pop * bl(B_MARKET) + 40.0 * bl(B_HARBOUR),
                0.035 * s.pop,
                s.pop * 0.05 * st,
                s.pop * 0.004 * MAGIC[culture] * (1.0 + s.magic.min(2.0)) * (1.0 + 0.5 * bl(B_COLLEGE)),
            ];
            let demand: f64 = jobs.iter().sum();
            let (filled, idle, short) = if demand <= free {
                (jobs, free - demand, 0.0)
            } else {
                let mut f = [0.0; 7];
                f[0] = jobs[0].min(free);
                let rest = free - f[0];
                let other = demand - jobs[0];
                for j in 1..7 { f[j] = jobs[j] * if other > 0.0 { rest / other } else { 0.0 }; }
                (f, 0.0, demand / free.max(1.0) - 1.0)
            };
            s.jobs = filled;
            s.jobless = idle / working.max(1.0);
            s.farm_fill = filled[0] / jobs[0].max(1.0);
            s.labour_short = short;
            let mut sk = s.skill.unwrap_or(PEOPLE_SKILL[culture]);
            // (masters teaching their trade join with slice 5; innovations' skill with slice 4)
            let bld_of = [B_MILL, B_MINE, B_WORKSHOPS, B_MARKET, usize::MAX, usize::MAX, B_COLLEGE];
            for j in 0..7 {
                let own = if bld_of[j] == usize::MAX { 0.0 } else { bl(bld_of[j]) };
                let target = (targets[culture][j] + 0.03 * bl(B_SCHOOL) + 0.04 * own + boost[j]).min(1.0);
                sk[j] += 0.06 * (target - sk[j]);
            }
            s.skill = Some(sk);
            let harvest = s.food_prod / s.pop.max(1.0);
            let arc = filled[6] / s.pop.max(1.0) * (0.6 + 0.8 * sk[6]);
            s.arcane = arc;
            let mult = [(0.5 + 0.5 * harvest).min(1.6) * (1.0 + 0.08 * bl(B_MILL)), 1.0 + 0.12 * bl(B_MINE),
                        1.0 + (25.0 * arc).min(0.2), 1.0 + 0.08 * bl(B_MARKET), 1.0, 1.0, 1.0];
            let mut val = [0.0; 7];
            for j in 0..7 { val[j] = filled[j] * VALUE[j] * mult[j] * (0.6 + 0.8 * sk[j]) * UNIT; }
            let value = val.iter().sum::<f64>() / UNIT + idle * IDLE_VALUE;
            s.labour_value = value * UNIT;
            let real = value * UNIT + 0.3 * terr_wealth[i] + s.trade_gain;
            s.real_income = real;
            s.income = real;
            s.wealth = s.wealth * 0.96 + real;
            let walls_cap = walls_caps[k];
            if s.wealth > 80.0 + 50.0 * s.walls as f64 && s.walls < walls_cap && self.rng.random() < 0.1 {
                s.wealth -= 60.0 + 40.0 * s.walls as f64;
                s.walls += 1;
            }
            // the classes: who earns what, who owns what, how many fall below the line (markets.classes)
            let rest = real - s.labour_value;
            let r_ = rent(gov) * val[0];
            let vf = val[0] - r_;
            let inc = [
                vf * 0.8 + val[1] * 0.55 + val[2] * 0.3 + val[3] * 0.1 + val[5] * 0.6 + idle * IDLE_VALUE * UNIT + soldiers * 0.6 * UNIT,
                vf * 0.2 + r_ * 0.1 + val[2] * 0.6 + val[3] * 0.6 + val[4] + val[5] * 0.4 + rest * 0.4 + val[6] * 0.6,
                r_ * 0.9 + val[1] * 0.45 + val[2] * 0.1 + val[3] * 0.3 + rest * 0.6 + val[6] * 0.4,
            ];
            let heads = [
                filled[0] * 0.9 + filled[1] + filled[2] * 0.4 + filled[3] * 0.1 + filled[5] * 0.6 + idle + soldiers,
                filled[0] * 0.08 + filled[2] * 0.55 + filled[3] * 0.7 + filled[4] + filled[5] * 0.4 + filled[6] * 0.8,
                filled[0] * 0.02 + filled[2] * 0.05 + filled[3] * 0.2 + s.pop * 0.01 + filled[6] * 0.2,
            ];
            let th = { let t: f64 = heads.iter().sum(); if t == 0.0 { 1.0 } else { t } };
            let tshare = if gov == "republic" || gov == "administrative" { [0.8, 1.0, 0.8] } else { TAX_SHARE };
            let mut housing = s.housing.unwrap_or(s.pop * 1.1);
            if housing < s.pop * 1.15 { housing += 0.015 * housing; }
            s.housing = Some(housing);
            let crowd = (s.pop / housing.max(1.0) - 1.0).max(0.0);
            s.crowded = crowd;
            let food = (s.food_prod / s.pop.max(1.0)).clamp(0.5, 1.3);
            let mut line = SUBSIST * food.powf(-0.7) * (1.0 + 0.6 * crowd);
            if s.relief > year { line *= 0.9; }
            let mut poverty = 0.0;
            for c in 0..3 {
                let people = heads[c] / th * s.pop;
                let net = inc[c] * (1.0 - tax * tshare[c]);
                let per = net / people.max(1.0);
                s.stock[c] = s.stock[c] * 0.95 + SAVE[c] * net;
                let ps = poor_share(line, per, SIGMA[c]);
                s.cls[c] = (people, per, ps);
                poverty += people * ps;
            }
            s.poverty = poverty / s.pop.max(1.0);
            s.econ_mood = -70.0 * s.jobless + 25.0 * short.min(0.4) - 40.0 * (s.cls[0].2 - 0.45);
        }
        // the idle drift to towns that need hands
        for f in 0..self.realms.len() {
            let towns: Vec<usize> = (0..self.towns.len()).filter(|&t| self.towns[t].alive && self.towns[t].realm == f).collect();
            if towns.len() < 2 { continue; }
            let fed: Vec<usize> = towns.iter().copied().filter(|&t| self.towns[t].food_prod > 1.1 * self.towns[t].pop).collect();
            let Some(&need) = fed.iter().max_by(|&&a, &&b| self.towns[a].labour_short.partial_cmp(&self.towns[b].labour_short).unwrap()) else { continue };
            if self.towns[need].labour_short <= 0.05 { continue; }
            let mut room = 0.5 * (self.towns[need].food_prod - 1.05 * self.towns[need].pop);
            for &t in &towns {
                if room <= 0.0 { break; }
                if t != need && self.towns[t].jobless > 0.08 && self.towns[t].landmass == self.towns[need].landmass {
                    let n = room.min(self.towns[t].pop * self.towns[t].jobless * 0.15);
                    self.towns[t].pop -= n;
                    self.towns[need].pop += n;
                    room -= n;
                }
            }
        }
    }

    /// Settlers go out from towns grown big and rich enough, to good open land nearby (world.expand).
    pub fn expand(&mut self) {
        let (h, w) = (self.map.h as i64, self.map.w as i64);
        let ids: Vec<usize> = (0..self.towns.len()).filter(|&t| self.towns[t].alive).collect();
        for s in ids {
            let (f, cu, pos, land) = (self.towns[s].realm, self.towns[s].culture, self.towns[s].pos, self.towns[s].landmass);
            if self.towns[s].pop < SETTLE_AT[cu] || self.towns[s].wealth < 30.0 || self.rng.random() > 0.25 { continue; }
            let angs: Vec<f64> = (0..60).map(|_| self.rng.uniform(0.0, 2.0 * std::f64::consts::PI)).collect();
            let rads: Vec<f64> = (0..60).map(|_| self.rng.uniform(4.0, 10.0)).collect();
            let mut cand = Vec::new();
            for j in 0..60 {
                let r = (pos.0 as f64 + rads[j] * angs[j].sin()).round() as i64;
                let c = (pos.1 as f64 + rads[j] * angs[j].cos()).round() as i64;
                if r < 1 || r >= h - 1 || c < 1 || c >= w - 1 { continue; }
                let (r, c) = (r as usize, c as usize);
                let t = self.map.terrain.at(r, c);
                let o = self.owner.at(r, c);
                let of = if o >= 0 { self.towns[o as usize].realm as i64 } else { -1 };
                if t == OCEAN || (t == MOUNTAIN && cu != 2) || !(of == -1 || of == f as i64) || self.blocked.at(r, c)
                    || self.map.landmass.at(r, c) != land { continue; }
                cand.push((r, c));
            }
            if cand.is_empty() { continue; }
            let mut best = (f64::MIN, 0usize);
            for (j, &(r, c)) in cand.iter().enumerate() {
                let mut score = 3.0 * self.map.mana.at(r, c);
                for rr in r.saturating_sub(2)..(r + 3).min(self.map.h) {
                    for cc in c.saturating_sub(2)..(c + 3).min(self.map.w) {
                        score += FOOD[cu][self.map.terrain.at(rr, cc) as usize];
                        if self.res_grid.at(rr, cc) >= 0 { score += 6.0; }
                    }
                }
                score += self.rng.random();
                if score > best.0 { best = (score, j); }
            }
            if best.0 < 18.0 { continue; }
            let settlers = 0.3 * self.towns[s].pop;
            self.towns[s].pop -= settlers;
            self.towns[s].wealth -= 30.0;
            self.add_town(f, cand[best.1], settlers, false);
        }
    }
}

pub fn thousands(x: f64) -> String {
    let s = format!("{:.0}", x);
    let mut out = String::new();
    for (i, ch) in s.chars().enumerate() {
        if i > 0 && (s.len() - i) % 3 == 0 { out.push(','); }
        out.push(ch);
    }
    out
}
