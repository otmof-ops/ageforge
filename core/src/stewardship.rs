//! The crown's purse and who runs it: the council, taxes and the levy (civics.armies_year), trade between realms
//! (markets.trade_year), gold and credit (markets.money_year), and the ruler's stewardship of the economy
//! (stewardship.py): buildings, relief, and the realm's judgement of its ruler.

use crate::economy::thousands;
use crate::people::*;
use crate::state::State;
use crate::tables::*;

pub const BUILDINGS: [(&str, f64, &str, Option<&str>); 8] = [
    ("Mill and Granary", 6.0, "farm", None), ("Mine Works", 8.0, "mine", None), ("Workshops", 7.0, "craft", None),
    ("Market Hall", 7.0, "trade", Some("Coinage")), ("Harbour", 10.0, "trade", Some("Longships")),
    ("School", 9.0, "skill", Some("Monastery Schools")), ("Housing", 4.0, "housing", None),
    ("Arcane College", 9.0, "arcane", Some("Monastery Schools")),
];
pub const B_MILL: usize = 0;
pub const B_MINE: usize = 1;
pub const B_WORKSHOPS: usize = 2;
pub const B_MARKET: usize = 3;
pub const B_HARBOUR: usize = 4;
pub const B_SCHOOL: usize = 5;
pub const B_HOUSING: usize = 6;
pub const B_COLLEGE: usize = 7;
const MAX_LEVEL: u8 = 5;
const SEAT_TITLES: [&str; 6] = ["Marshal", "Steward", "Spymaster", "Court Mage", "Loremaster", "Lord Admiral"];
const SECTOR_OF: [&str; 7] = ["farm", "mine", "craft", "trade", "clergy", "service", "arcane"];

pub fn stage(inn: &std::collections::HashSet<&'static str>) -> u32 {
    if inn.contains("Banking Houses") { 2 } else if inn.contains("Coinage") { 1 } else { 0 }
}

impl State {
    pub fn alive_towns_of(&self, k: usize) -> Vec<usize> {
        (0..self.towns.len()).filter(|&t| self.towns[t].alive && self.towns[t].realm == k).collect()
    }

    pub fn active(&self, k: usize) -> bool {
        !self.realms[k].extinct && self.realms[k].ruler != NONE
    }

    /// A new realm needs a row and a column in the tables of war and peace (world.grow_tables).
    pub fn grow_tables(&mut self) {
        let n = self.realms.len();
        for row in self.relation.iter_mut() { row.resize(n, 0.0); }
        while self.relation.len() < n { self.relation.push(vec![0.0; n]); }
        for row in self.war.iter_mut() { row.resize(n, false); }
        while self.war.len() < n { self.war.push(vec![false; n]); }
        for row in self.truce.iter_mut() { row.resize(n, 0); }
        while self.truce.len() < n { self.truce.push(vec![0; n]); }
    }

    /// Which luxury and strategic deposits each realm holds this year (resources.realm_resources).
    pub fn realm_resources(&mut self) {
        let mut held: Vec<Vec<usize>> = vec![vec![]; self.realms.len()];
        for i in 0..self.map.h * self.map.w {
            let (g, o) = (self.res_grid.v[i], self.owner.v[i]);
            if g < 0 || o < 0 { continue; }
            let k = self.towns[o as usize].realm;
            let r = g as usize;
            if !held[k].contains(&r) { held[k].push(r); }
        }
        for (k, h) in held.into_iter().enumerate() {
            self.realms[k].goods = h.iter().copied().filter(|&r| matches!(RESOURCES[r].kind, "luxury" | "strategic")).collect();
            self.realms[k].held = h;
        }
    }

    /// The council (dynasty.council): weighed afresh every five years, on a new ruler, or when a seat falls empty.
    pub fn councils(&mut self) {
        let y = self.year;
        let mut by_realm: Vec<Vec<usize>> = vec![vec![]; self.realms.len()];
        for c in &self.chars {
            if c.alive && c.adult(y) && c.realm < self.realms.len() { by_realm[c.realm].push(c.id); }
        }
        for k in 0..self.realms.len() {
            if !self.active(k) { continue; }
            let r = self.realms[k].ruler;
            let cands: Vec<usize> = by_realm[k].iter().copied()
                .filter(|&c| c != r && self.chars[c].disgraced_until <= y).collect();
            let fresh = (y + k as i64) % 5 == 0 || self.realms[k].council_for != r;
            self.realms[k].council_for = r;
            let prev = self.realms[k].council;
            let mut taken: Vec<usize> = vec![];
            for seat in 0..6 {
                let cur = self.realms[k].council[seat];
                if !fresh && cur != NONE && self.chars[cur].alive && self.chars[cur].realm == k && cur != r && !taken.contains(&cur) {
                    taken.push(cur);
                    continue;
                }
                // (powerful vassals expecting a seat come with titles, slice 4)
                let score = |c: &Character| -> f64 {
                    match seat {
                        0 => c.attr(1, y),
                        1 => c.attr(2, y) + c.skills[skill("Mercantile")] / 25.0,
                        2 => c.attr(3, y),
                        3 => -1.0, // the court mage needs tiers of magic (slice 6)
                        4 => c.attr(5, y),
                        _ => 0.0,  // seamanship (slice 6)
                    }
                };
                let mut order: Vec<usize> = cands.iter().copied().filter(|c| !taken.contains(c)).collect();
                order.sort_by(|&a, &b| score(&self.chars[b]).partial_cmp(&score(&self.chars[a])).unwrap());
                let pick = order.first().copied();
                match pick {
                    Some(p) if seat != 3 => {
                        taken.push(p);
                        if self.realms[k].council[seat] != p {
                            self.realms[k].council[seat] = p;
                            let t = format!("{} of {} from year {}", SEAT_TITLES[seat], self.realms[k].name, y);
                            self.chars[p].titles.push(t);
                            self.chars[p].memories.push((r, "appointed me to the council", 15.0, 0.5, y));
                            let old = prev[seat];
                            if old != NONE && self.chars[old].alive {
                                self.chars[old].memories.push((r, "removed me from the council", -20.0, 1.0, y));
                            }
                        }
                    }
                    _ => self.realms[k].council[seat] = NONE,
                }
            }
        }
    }

    /// Trade between realms by land and, with ships, by sea; gains to the trading towns (markets.trade_year).
    pub fn trade_year(&mut self) {
        let ks: Vec<usize> = (0..self.realms.len()).filter(|&k| self.active(k)).collect();
        let n = self.realms.len();
        let mut gdp = vec![0.0; n];
        let mut craft = vec![0.3; n];
        let mut ports: Vec<Option<(f64, f64)>> = vec![None; n];
        let mut reach = vec![0u32; n];
        for &k in &ks {
            let towns = self.alive_towns_of(k);
            gdp[k] = towns.iter().map(|&t| self.towns[t].real_income - self.towns[t].trade_gain).sum();
            let sk: Vec<f64> = towns.iter().filter_map(|&t| self.towns[t].skill.map(|s| s[2])).collect();
            if !sk.is_empty() { craft[k] = sk.iter().sum::<f64>() / sk.len() as f64; }
            let pt: Vec<usize> = towns.iter().copied().filter(|&t| self.towns[t].port).collect();
            if !pt.is_empty() {
                let m = pt.len() as f64;
                ports[k] = Some((pt.iter().map(|&t| self.towns[t].pos.0 as f64).sum::<f64>() / m,
                                 pt.iter().map(|&t| self.towns[t].pos.1 as f64).sum::<f64>() / m));
            }
            let inn = &self.cultures[self.realms[k].people].innovations;
            reach[k] = if inn.contains("Galleon Yards") { 3 } else if inn.contains("Carrack Hulls") || inn.contains("Sea Charts") { 2 }
                       else if inn.contains("Longships") { 1 } else { 0 };
            for &t in &towns { self.towns[t].trade_gain = 0.0; }
        }
        // neighbours by land: realms whose territories touch
        let mut land = std::collections::HashSet::new();
        let (h, w) = (self.map.h, self.map.w);
        let realm_at = |i: usize| -> i64 { let o = self.owner.v[i]; if o < 0 { -1 } else { self.towns[o as usize].realm as i64 } };
        for r in 0..h {
            for c in 0..w {
                let a = realm_at(r * w + c);
                if a < 0 { continue; }
                for j in [if c + 1 < w { Some(r * w + c + 1) } else { None }, if r + 1 < h { Some((r + 1) * w + c) } else { None }].into_iter().flatten() {
                    let b = realm_at(j);
                    if b >= 0 && b != a { land.insert((a.min(b) as usize, a.max(b) as usize)); }
                }
            }
        }
        let diag = (h as f64).hypot(w as f64);
        let mut routes: Vec<((usize, usize), f64, f64)> = vec![];
        for (i, &a) in ks.iter().enumerate() {
            for &b in &ks[i + 1..] {
                if gdp[a] <= 0.0 || gdp[b] <= 0.0 { continue; }
                let key = (a.min(b), a.max(b));
                let carry;
                if land.contains(&key) {
                    carry = 1.0;
                } else if let (Some(pa), Some(pb)) = (ports[a], ports[b]) {
                    let d = (pa.0 - pb.0).hypot(pa.1 - pb.1) / diag;
                    let limit = [0.0, 0.12, 0.4, 2.0][reach[a].min(reach[b]) as usize];
                    if d > limit { continue; }
                    carry = 0.8 - 0.6 * d;
                } else {
                    continue;
                }
                let (ga, gb) = (&self.realms[a].goods, &self.realms[b].goods);
                let a_has = ga.iter().filter(|x| !gb.contains(x)).count() as f64;
                let b_has = gb.iter().filter(|x| !ga.contains(x)).count() as f64;
                let comp = (1.0 + 0.4 * (a_has + b_has) + (craft[a] - craft[b]).abs()).min(4.0);
                let prot = self.realms[a].trade_protectionist as i32 + self.realms[b].trade_protectionist as i32;
                let friendly = if self.relation[a][b] < -50.0 { 0.5 } else { 1.0 };
                let t = 0.012 * (gdp[a] * gdp[b]).sqrt() * comp * carry * friendly * 0.5f64.powi(prot);
                if t < 0.5 { continue; }
                let offer_a = 1.0 + a_has + 3.0 * craft[a];
                let offer_b = 1.0 + b_has + 3.0 * craft[b];
                routes.push((key, t, offer_a / (offer_a + offer_b)));
            }
        }
        for &k in &ks { let r = &mut self.realms[k]; r.trade_val = 0.0; r.trade_net = 0.0; r.partners = 0; }
        for &((a, b), t, share_a) in &routes {
            for (k, sh) in [(a, share_a), (b, 1.0 - share_a)] {
                let r = &mut self.realms[k];
                r.trade_val += t;
                r.trade_net += t * (2.0 * sh - 1.0);
                r.partners += 1;
            }
        }
        for &k in &ks {
            if self.realms[k].trade_val == 0.0 { continue; }
            let towns: Vec<usize> = self.alive_towns_of(k).into_iter().filter(|&t| self.towns[t].port).collect();
            let weight = { let w: f64 = towns.iter().map(|&t| 1.0 + 2.0 * self.towns[t].port as u8 as f64).sum(); if w == 0.0 { 1.0 } else { w } };
            let tv = self.realms[k].trade_val;
            for &t in &towns { self.towns[t].trade_gain = 0.25 * tv * (1.0 + 2.0 * self.towns[t].port as u8 as f64) / weight; }
            if stage(&self.cultures[self.realms[k].people].innovations) >= 1 {
                let tariff = if self.realms[k].trade_protectionist { 0.10 } else { 0.03 };
                self.realms[k].gold += tariff * tv;
            }
        }
    }

    /// Gold coin, the mines and the balance of trade, lending, and the crown's choices when the lenders say no.
    pub fn money_year(&mut self) {
        for k in 0..self.realms.len() {
            if !self.active(k) { continue; }
            let towns = self.alive_towns_of(k);
            if towns.is_empty() { continue; }
            let y = self.year;
            let r = self.realms[k].ruler;
            let st = stage(&self.cultures[self.realms[k].people].innovations);
            self.realms[k].income = towns.iter().map(|&t| self.towns[t].income).sum();
            self.realms[k].real_gdp = towns.iter().map(|&t| self.towns[t].real_income).sum();
            let steward = self.realms[k].council[1];
            let ste = if steward != NONE { self.chars[steward].attr(2, y) } else { self.chars[r].attr(2, y) * 0.7 };
            let mint = self.realms[k].mint_skill;
            self.realms[k].collect = (0.75 + 0.02 * ste + 0.05 * mint.min(1.0)).min(1.2);
            if st == 0 {
                let floor = -(20.0f64).max(0.5 * self.realms[k].income);
                if self.realms[k].gold < floor {
                    self.realms[k].gold = floor;
                    let l = self.chars[r].legitimacy.unwrap_or(50.0);
                    self.chars[r].legitimacy = Some((l - 2.0).max(0.0));
                    if self.rng.random() < 0.1 {
                        let t = format!("The warriors of {} go unpaid; {} has nothing left to give them.", self.realms[k].name, self.who(r));
                        self.say(t, true);
                    }
                }
                continue;
            }
            if self.realms[k].credit.is_none() {
                self.realms[k].credit = Some(45.0 + 10.0 * st as f64);
                let t = format!("**Coin.** The mint of {} strikes its first gold.", self.realms[k].name);
                self.say(t, true);
            }
            let wear = 0.996 + 0.002 * mint.min(1.0);
            let mut mined = 0.0;
            for &t in &towns {
                // gold and silver in the town's own land (counted in land_year as res_wealth's companions)
                let bullion = self.towns[t].bullion;
                let s = &mut self.towns[t];
                let coin = s.coin.unwrap_or(s.real_income / 4.0);
                let m = bullion * (0.6 + 0.8 * s.skill.map(|x| x[1]).unwrap_or(0.35));
                s.coin = Some(coin * wear + m * 0.9);
                mined += m;
            }
            let settle = 0.1 * self.realms[k].trade_net;
            let ports: Vec<usize> = { let p: Vec<usize> = towns.iter().copied().filter(|&t| self.towns[t].port).collect(); if p.is_empty() { towns.clone() } else { p } };
            for &t in &ports { let s = &mut self.towns[t]; s.coin = Some((s.coin.unwrap() + settle / ports.len() as f64).max(0.0)); }
            let money: f64 = towns.iter().map(|&t| self.towns[t].coin.unwrap_or(0.0)).sum();
            let realm = &mut self.realms[k];
            realm.money = money;
            realm.gold += 0.1 * mined;
            self.mined += mined;
            let debt = (-realm.gold).max(0.0);
            let revenue = (realm.income * realm.tax_now * realm.collect).max(1.0);
            let credit = realm.credit.unwrap();
            let credit = (credit + 1.0 - 0.8 * (debt / revenue - 3.0).max(0.0)).clamp(0.0, 100.0);
            realm.credit = Some(credit);
            let base = if st == 2 { 0.06 } else { 0.12 };
            realm.rate = (base + 0.02 * debt / revenue + (50.0 - credit).max(0.0) / 250.0).min(0.25);
            let capacity = (0.5 * revenue).max(revenue * if st == 2 { 4.0 } else { 2.0 } * credit / 60.0);
            if debt > 0.0 { realm.gold -= debt.min(capacity) * realm.rate; }
            if debt > capacity { self.crisis(k, debt, capacity, revenue, ste); }
        }
    }

    /// The lenders will lend no more; what the ruler does depends on who the ruler is (markets.crisis).
    fn crisis(&mut self, k: usize, debt: f64, capacity: f64, revenue: f64, ste: f64) {
        let r = self.realms[k].ruler;
        let has = |t: &str| self.chars[r].personality.contains(&PERSONALITY.iter().position(|p| *p == t).unwrap());
        let gap = debt - capacity;
        let honest = has("honest") as i32 + has("just") as i32 + (ste >= 12.0) as i32;
        let venal = has("greedy") as i32 + has("deceitful") as i32 + has("arbitrary") as i32 + has("cruel") as i32;
        if self.realms[k].tax_mult < 1.4 {
            self.realms[k].tax_mult += 0.1;
            let t = format!("{} raises new taxes to pay the lenders of {}.", self.who(r), self.realms[k].name);
            self.say(t, true);
        }
        if venal > honest && self.rng.random() < 0.6 && self.year - self.realms[k].last_forced >= 5 {
            self.realms[k].last_forced = self.year;
            let towns: Vec<usize> = self.alive_towns_of(k).into_iter().filter(|&t| self.towns[t].coin.unwrap_or(0.0) > 0.0).collect();
            let have: f64 = towns.iter().map(|&t| self.towns[t].coin.unwrap()).sum();
            let take = (gap * 0.8).min(0.3 * have);
            if take > 0.0 {
                for &t in &towns { let c = self.towns[t].coin.unwrap(); self.towns[t].coin = Some(c - take * c / have); }
                self.realms[k].gold += take;
                self.realms[k].credit = Some((self.realms[k].credit.unwrap() - 10.0).max(0.0));
                self.stats_forced.push((self.year, k));
                let t = format!("**A forced loan in {}.** {} seizes {} gold from the merchants and guilds, against promises no one believes.",
                                self.realms[k].name, self.who(r), thousands(take));
                self.say(t, take < revenue);
                return;
            }
        }
        // austerity: regiments come with slice 4; the court is stripped (courts too)
        let l = self.chars[r].legitimacy.unwrap_or(50.0);
        self.chars[r].legitimacy = Some((l - 1.0).max(0.0));
        if (-self.realms[k].gold).max(0.0) > (capacity * 1.5).max(revenue) && self.year - self.realms[k].last_default >= 10 {
            self.realms[k].last_default = self.year;
            self.stats_defaults.push((self.year, k));
            self.realms[k].credit = Some((self.realms[k].credit.unwrap() - 40.0).max(0.0));
            self.realms[k].defaults += 1;
            self.bankrupt(k);
        }
    }

    fn bankrupt(&mut self, k: usize) {
        let r = self.realms[k].ruler;
        self.realms[k].gold = 0.0;
        let l = self.chars[r].legitimacy.unwrap_or(50.0);
        self.chars[r].legitimacy = Some((l - 15.0).max(0.0));
        self.bankruptcies.push((self.year, k));
        let t = format!("**{} is bankrupt.** {} cannot pay the soldiers or the lenders; half the regiments melt away and the court goes threadbare.",
                        self.realms[k].name, self.who(r));
        self.say(t, false);
    }

    /// The ruler's stewardship: read the realm, build, relieve the poor, and be judged (stewardship.manage_year).
    pub fn manage_year(&mut self) {
        let y = self.year;
        for k in 0..self.realms.len() {
            if !self.active(k) { continue; }
            let towns = self.alive_towns_of(k);
            if towns.is_empty() { continue; }
            let r = self.realms[k].ruler;
            let e = self.realm_economy(k, &towns);
            let steward = self.realms[k].council[1];
            let s = if steward != NONE { self.chars[steward].attr(2, y) } else { 0.0 };
            let rc = &self.chars[r];
            let has = |t: &str| rc.personality.contains(&PERSONALITY.iter().position(|p| *p == t).unwrap());
            let attention = (0.35 + 0.025 * rc.attr(2, y) + 0.02 * s + 0.3 * has("diligent") as u8 as f64 - 0.3 * has("lazy") as u8 as f64
                             - 0.1 * has("content") as u8 as f64 + 0.1 * has("ambitious") as u8 as f64).clamp(0.05, 0.95);
            let judgement = (rc.attr(2, y) + 1.3 * s) / 2.0;
            let (compassionate, just, generous, impatient, greedy) = (has("compassionate"), has("just"), has("generous"), has("impatient"), has("greedy"));
            let upkeep: f64 = towns.iter().map(|&t| (0..8).map(|b| BUILDINGS[b].1 * self.towns[t].bld[b] as f64).sum::<f64>()).sum::<f64>() * 0.01;
            self.realms[k].gold -= upkeep;
            if self.rng.random() > attention {
                self.reckoning(k, &e);
                continue;
            }
            if e.poverty > 0.5 && (compassionate || just || judgement > 12.0) && self.realms[k].gold > 50.0 {
                self.realms[k].gold -= 0.04 * e.gdp;
                for &t in &towns { if self.towns[t].poverty > 0.45 { self.towns[t].relief = y + 3; } }
                if self.rng.random() < 0.2 {
                    let t = format!("{} opens the granaries and funds the almshouses of {}.", self.who(r), self.realms[k].name);
                    self.say(t, true);
                }
            }
            if e.poverty > 0.6 && self.realms[k].tax_relief <= y && e.debt_ratio < 2.0 && (compassionate || generous || judgement > 14.0) {
                self.realms[k].tax_relief = y + 5;
                let t = format!("{} lifts part of the taxes of {} while the poor recover.", self.who(r), self.realms[k].name);
                self.say(t, true);
            }
            let prudent = judgement >= 9.0 && !impatient;
            let mut budget = self.realms[k].gold - if prudent { 3.0 } else { 0.5 } * e.revenue.max(10.0) * 0.3;
            let mut opts = self.build_options(k, &e, &towns);
            if opts.is_empty() { self.reckoning(k, &e); continue; }
            let coinage = self.cultures[self.realms[k].people].innovations.contains("Coinage");
            for _ in 0..1 + (judgement > 12.0) as usize {
                let pick = if greedy && self.rng.random() < 0.4 {
                    let cap = towns.iter().copied().find(|&t| self.towns[t].capital)
                        .unwrap_or_else(|| *towns.iter().max_by(|&&a, &&b| self.towns[a].pop.partial_cmp(&self.towns[b].pop).unwrap()).unwrap());
                    (0.0, cap, if coinage { B_MARKET } else { B_WORKSHOPS })
                } else if self.rng.random() < (judgement / 18.0).min(0.9) {
                    *opts.iter().max_by(|a, b| a.0.partial_cmp(&b.0).unwrap()).unwrap()
                } else {
                    opts[self.rng.integers(0, opts.len() as i64) as usize]
                };
                let (_, t, b) = pick;
                let cost = BUILDINGS[b].1;          // (the Royal Architect's saving comes with courts, slice 4)
                if cost > budget { break; }
                self.towns[t].bld[b] += 1;
                if b == B_HOUSING {
                    let s = &mut self.towns[t];
                    s.housing = Some(s.housing.unwrap_or(s.pop) + 300.0);
                }
                self.realms[k].gold -= cost;
                budget -= cost;
                self.chars[r].built += 1;
                if self.towns[t].bld[b] >= 3 {
                    let name = BUILDINGS[b].0;
                    let a = if b != B_HOUSING && b != B_WORKSHOPS { "a " } else { "" };
                    let text = format!("{} builds {a}{name} at {} (level {}).", self.who(r), self.towns[t].name, self.towns[t].bld[b]);
                    self.say(text, true);
                }
                opts.retain(|o| !(o.1 == t && o.2 == b));
                if opts.is_empty() { break; }
            }
            self.reckoning(k, &e);
        }
    }

    fn build_options(&self, k: usize, e: &Econ, towns: &[usize]) -> Vec<(f64, usize, usize)> {
        let inn = &self.cultures[self.realms[k].people].innovations;
        let mut out = vec![];
        for &t in towns {
            let s = &self.towns[t];
            let sk = s.skill.unwrap_or([0.5; 7]);
            let weight = s.pop.sqrt();
            for (b, &(_, _, aim, need)) in BUILDINGS.iter().enumerate() {
                if s.bld[b] >= MAX_LEVEL || need.is_some_and(|n| !inn.contains(n)) { continue; }
                if b == B_HARBOUR && !s.port { continue; }
                if b == B_MINE && s.jobs[1] <= 0.0 { continue; }
                let gain = match aim {
                    "housing" => 4.0 * s.crowded + 0.5 * (s.poverty - 0.4).max(0.0),
                    "skill" => 0.6 * (1.0 - sk.iter().sum::<f64>() / 7.0) + 0.3 * (e.poverty - 0.4).max(0.0),
                    "farm" => 0.5 * s.labour_short + 0.3 * (1.0 - s.food_prod / s.pop.max(1.0)).max(0.0) + 0.1,
                    _ => {
                        let j = SECTOR_OF.iter().position(|x| *x == aim).unwrap();
                        3.0 * s.jobless + 0.4 * (1.0 - sk[j]) + 0.05
                    }
                };
                out.push((gain * weight / (1.0 + s.bld[b] as f64), t, b));
            }
        }
        out
    }

    fn realm_economy(&mut self, k: usize, towns: &[usize]) -> Econ {
        let pop = { let p: f64 = towns.iter().map(|&t| self.towns[t].pop).sum(); if p == 0.0 { 1.0 } else { p } };
        let (mut poor, mut people) = (0.0, 0.0);
        for &t in towns { for c in 0..3 { let (n, _, pv) = self.towns[t].cls[c]; poor += n * pv; people += n; } }
        let poverty = if people > 0.0 { poor / pop } else { 0.0 };
        let work = { let w: f64 = towns.iter().map(|&t| 0.55 * self.towns[t].pop).sum(); if w == 0.0 { 1.0 } else { w } };
        let jobless = towns.iter().map(|&t| self.towns[t].jobless * 0.55 * self.towns[t].pop).sum::<f64>() / work;
        let realm = &self.realms[k];
        let debt = (-realm.gold).max(0.0);
        let revenue = (realm.income * realm.tax_now * realm.collect).max(1.0);
        let debt_ratio = debt / revenue;
        let distress = (((debt_ratio - 1.5) / 4.0).max(0.0) + (poverty - 0.45).max(0.0) * 2.0 + jobless * 3.0).clamp(0.0, 1.0);
        self.realms[k].econ_distress = distress;
        Econ { poverty, debt_ratio, revenue, gdp: self.realms[k].real_gdp, distress }
    }

    /// The realm judges its ruler by how it fares.
    fn reckoning(&mut self, k: usize, e: &Econ) {
        let r = self.realms[k].ruler;
        let prev = self.realms[k].econ_prev_gdp;
        self.realms[k].econ_prev_gdp = Some(e.gdp);
        let l = self.chars[r].legitimacy.unwrap_or(50.0);
        if e.distress < 0.1 && e.poverty < 0.45 {
            self.chars[r].legitimacy = Some((l + 0.4).min(100.0));
            if let Some(p) = prev { if e.gdp > p * 1.01 {
                let b = self.chars[r].built as f64;
                self.chars[r].renown += 0.5 + 0.1 * b.sqrt();
            } }
        } else if e.distress > 0.5 {
            self.chars[r].legitimacy = Some((l - e.distress).max(0.0));
            // (the lords' blame lands on duchy holders: titles, slice 4)
            if self.rng.random() < 0.2 && self.rng.random() < 0.3 {
                let why = if e.debt_ratio > 3.0 { "debts" } else if e.poverty > 0.5 { "hunger and want" } else { "idle hands" };
                let t = format!("The lords of {} blame {} for the {why} that grip the realm.", self.realms[k].name, self.who(r));
                self.say(t, false);
            }
        }
    }

    /// How the chronicle names a person (world.who, for rulers and the rest).
    pub fn who(&self, cid: usize) -> String {
        let c = &self.chars[cid];
        let full = match &c.regnal { Some(n) => format!("{} {}", c.name, n), None => c.name.clone() };
        if let Some((_, _, None)) = c.ruled {
            let t = if c.female { RULER_TITLE_F[c.people] } else { RULER_TITLE_M[c.people] };
            return format!("{t} {full}");
        }
        let cls = CLASSES[c.classes.last().unwrap().0].name;
        format!("{full} ({} {cls} {})", SINGULAR[c.people], c.level)
    }
}

pub struct Econ {
    pub poverty: f64,
    pub debt_ratio: f64,
    pub revenue: f64,
    pub gdp: f64,
    pub distress: f64,
}

pub fn skill(name: &str) -> usize {
    SKILLS.iter().position(|s| *s == name).expect("unknown skill")
}
