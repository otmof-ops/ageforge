//! Civilisation (civics.py): research by era, traditions, the trend of the times and each person's views on how
//! to be ruled, laws and agendas, the treasury with its levy, garrisons and regiments, rank decisions, government
//! reforms, confederations of tribes, reform movements and wars, and culture events.
//! (Merchants and priests as reform candidates join with professions, slice 5; sagas of heroes with slice 6.)

use crate::people::*;
use crate::state::State;
use crate::tables::*;

const RANKS: [&str; 5] = ["tribe", "chiefdom", "duchy", "kingdom", "empire"];

fn gov_ix(g: &str) -> usize { GOVS.iter().position(|x| *x == g).unwrap_or(0) }
fn gov_static(g: &str) -> &'static str { GOVS[gov_ix(g)] }

impl State {
    pub fn walls_cap(&self, k: usize) -> u32 {
        let cu = &self.cultures[self.realms[k].people];
        (cu.best("walls_cap", 1.0) + cu.traditions.iter().map(|&t| TRADITIONS[t].1.iter().filter(|x| x.0 == "walls").map(|x| x.1).sum::<f64>()).sum::<f64>()) as u32
    }

    fn research_year(&mut self) {
        let y = self.year;
        let mut pts = [0.0; 4];
        let mut lead = [NONE; 4];
        for k in 0..self.realms.len() {
            if !self.active(k) { continue; }
            let r = self.realms[k].ruler;
            let n = self.alive_towns_of(k).len();
            let (lore, mage) = (self.realms[k].council[4], self.realms[k].council[3]);
            let mut p = (n as f64).sqrt() + self.chars[r].attr(4, y) / 3.0
                + if lore != NONE { self.chars[lore].attr(5, y) / 4.0 } else { 0.0 }
                + if mage != NONE { self.tier(&self.chars[mage]).max(0) as f64 } else { 0.0 };
            if self.realms[k].agenda == "learning" { p *= 1.4; }
            let pe = self.realms[k].people;
            pts[pe] += p;
            if lead[pe] == NONE || n > self.alive_towns_of(lead[pe]).len() { lead[pe] = k; }
        }
        let eras: Vec<usize> = self.cultures.iter().map(|c| c.era).collect();
        for ci in 0..4 {
            let cu = &self.cultures[ci];
            let mult = 1.0 + cu.total("research") + if cu.golden > y { 0.5 } else { 0.0 } - if cu.dark > y { 0.4 } else { 0.0 };
            let diffusion = 1.0 + 0.15 * eras.iter().filter(|&&e| e > cu.era).count() as f64;
            let cu = &mut self.cultures[ci];
            cu.progress += pts[ci] * mult * diffusion * 0.25;
            cu.culture_points += pts[ci] * 0.05;
            if cu.focus.is_none_or(|f| cu.innovations.contains(INNOVATIONS[f].name)) {
                let open: Vec<usize> = (0..INNOVATIONS.len()).filter(|&i| INNOVATIONS[i].era <= cu.era && !cu.innovations.contains(INNOVATIONS[i].name)).collect();
                if open.is_empty() { continue; }
                let agenda = if lead[ci] != NONE { self.realms[lead[ci]].agenda } else { "unity" };
                let want = match agenda { "conquest" => "military", "learning" | "faith" => "cultural", _ => "civic" };
                let pref: Vec<usize> = open.iter().copied().filter(|&i| INNOVATIONS[i].cat == want).collect();
                let pick = if pref.is_empty() { &open } else { &pref };
                let f = pick[self.rng.integers(0, pick.len() as i64) as usize];
                self.cultures[ci].focus = Some(f);
            }
            let cu = &mut self.cultures[ci];
            let f = cu.focus.unwrap();
            let cost = ERA_COST[INNOVATIONS[f].era];
            if cu.progress >= cost {
                cu.progress -= cost;
                let name = INNOVATIONS[f].name;
                cu.innovations.insert(name);
                cu.learned.push((name, y));
                cu.history.push((y, format!("discovers {name}")));
                cu.focus = None;
                let row: Vec<&str> = INNOVATIONS.iter().filter(|i| i.era == cu.era).map(|i| i.name).collect();
                let mut new_era = false;
                if cu.era < ERAS.len() - 1 && row.iter().filter(|n| cu.innovations.contains(*n)).count() as f64 >= 0.7 * row.len() as f64 {
                    cu.era += 1;
                    cu.history.push((y, format!("enters the {} era", ERAS[cu.era])));
                    cu.trend[4] -= 0.1;
                    new_era = true;
                }
                let era = cu.era;
                self.say(format!("The {} learn {name}.", PEOPLES[ci]), true);
                if new_era { self.say(format!("**A new era.** The {} enter the {} age.", PEOPLES[ci], ERAS[era]), false); }
            }
            let cu = &self.cultures[ci];
            if cu.culture_points >= 60.0 * (cu.traditions.len() + 1) as f64 && cu.traditions.len() < 5 {
                let avail: Vec<usize> = (0..TRADITIONS.len()).filter(|t| !cu.traditions.contains(t)).collect();
                let t = avail[self.rng.integers(0, avail.len() as i64) as usize];
                let cu = &mut self.cultures[ci];
                cu.culture_points = 0.0;
                cu.traditions.push(t);
                for a in 0..5 { cu.trend[a] += TRADITIONS[t].2[a]; }
                cu.history.push((y, format!("adopts the tradition {}", TRADITIONS[t].0)));
                self.say(format!("The {} take up a new tradition: {}.", PEOPLES[ci], TRADITIONS[t].0), true);
            }
        }
    }

    /// What a person thinks governance should be: their culture's trend, their nature, their trade.
    pub fn views(&mut self, c: usize) -> [f64; 5] {
        if self.chars[c].view_noise.is_none() {
            let n = [0; 5].map(|_| self.rng.normal(0.0, 0.2));
            self.chars[c].view_noise = Some(n);
        }
        let cc = &self.chars[c];
        let mut v = self.cultures[cc.people].trend;
        let p = |t: &str| cc.has_trait(t) as u8 as f64;
        v[0] += 0.3 * p("ambitious") + 0.2 * p("cruel") - 0.3 * p("kind") - 0.2 * p("trusting");
        v[2] += 0.4 * p("zealous") - 0.4 * p("cynical");
        v[3] += 0.3 * p("greedy") - 0.2 * p("generous");
        v[4] += 0.2 * p("content") - 0.3 * p("ambitious") - 0.1 * p("diligent");
        v[1] += if cc.house != NONE && self.houses[cc.house].rulers > 0 { 0.3 } else { -0.1 };
        match cc.profession {
            Some("merchant") => { v[3] += 0.6; v[1] -= 0.4; }
            Some("priest") => v[2] += 0.6,
            _ => if cc.hero { v[4] -= 0.2 },
        }
        let n = cc.view_noise.unwrap();
        for a in 0..5 { v[a] = (v[a] + n[a]).clamp(-1.0, 1.0); }
        v
    }

    /// -1 (hates how they are ruled) to +1 (exactly as it should be).
    pub fn agreement(&mut self, c: usize, k: usize) -> f64 {
        let v = self.views(c);
        let mut g = GOV_VIEWS[gov_ix(self.realms[k].gov)];
        g[0] += 0.15 * (self.realms[k].crown_law as f64 - 1.0);
        1.0 - (0..5).map(|a| (v[a] - g[a]).abs()).sum::<f64>() / 5.0
    }

    fn trends_year(&mut self) {
        for ci in 0..4 {
            let ks: Vec<usize> = (0..self.realms.len()).filter(|&k| self.active(k) && self.realms[k].people == ci).collect();
            if ks.is_empty() { continue; }
            let tyr = ks.iter().map(|&k| self.chars[self.realms[k].ruler].tyranny).sum::<f64>() / ks.len() as f64;
            let wars = ks.iter().filter(|&&k| self.war[k].iter().any(|&w| w)).count() as f64 / ks.len() as f64;
            let merch = self.chars.iter().filter(|c| c.alive && c.people == ci && c.profession == Some("merchant")).count() as f64 / ks.len() as f64;
            let cu = &mut self.cultures[ci];
            cu.trend[0] += 0.006 * (wars - 0.4) - 0.003 * (tyr - 20.0) / 10.0;
            cu.trend[3] += 0.004 * (merch - 1.5);
            cu.trend[2] -= 0.002;               // (a calamity on their lands turns them to faith: slice 6)
            for a in 0..5 {
                cu.trend[a] += 0.04 * (CULTURE_VIEWS[ci][a] - cu.trend[a]);
                cu.trend[a] = cu.trend[a].clamp(-1.0, 1.0);
            }
        }
    }

    /// Taxes in, field armies fed, levy and garrisons, regiments kept, disbanded and raised (civics.armies_year).
    fn treasury_year(&mut self) {
        for k in 0..self.realms.len() {
            if !self.active(k) { continue; }
            let cu = &self.cultures[self.realms[k].people];
            let (wealth, levy_b, garrison_b) = (cu.total("wealth"), cu.total("levy"), cu.total("garrison"));
            let avail_all: Vec<&'static str> = INNOVATIONS.iter().filter(|i| cu.innovations.contains(i.name)).flat_map(|i| i.maa.iter().copied()).collect();
            let towns = self.alive_towns_of(k);
            let base: f64 = towns.iter().map(|&t| self.towns[t].income).sum();
            let year = self.year;
            let walls_cap = self.walls_cap(k);
            let at_war = self.war[k].iter().any(|&w| w);
            let pop: f64 = towns.iter().map(|&t| self.towns[t].pop).sum();
            let realm = &mut self.realms[k];
            let tax = match realm.gov { "tribal" => 0.015, "clan" => 0.025, "feudal" => 0.03, "republic" => 0.05, "theocracy" => 0.035, _ => 0.05 };
            realm.tax_now = (0.12 + (tax - 0.03) * 4.0 + 0.05 * (realm.crown_law as f64 - 1.0)) * if realm.tax_relief > year { 0.6 } else { 1.0 };
            realm.gold += base * realm.tax_now * (1.0 - realm.corruption) * (1.0 + wealth) * realm.tax_mult * realm.collect;
            // (hosts in the field are fed from here once warfare is ported)
            let levy = match realm.gov { "tribal" => 0.12, "clan" => 0.1, "feudal" => 0.08, "republic" => 0.05, "theocracy" => 0.07, _ => 0.06 };
            realm.levy = pop * (levy + levy_b);
            realm.garrison_bonus = garrison_b;
            let upkeep: f64 = realm.maa.iter().map(|&(m, n)| MAA[m].2 * n as f64).sum::<f64>() * 8.0;
            realm.gold -= upkeep;
            while realm.gold < -2.0 * (10.0f64).max(base * realm.tax_now) && !realm.maa.is_empty() {
                let j = (0..realm.maa.len()).max_by(|&a, &b| MAA[realm.maa[a].0].2.partial_cmp(&MAA[realm.maa[b].0].2).unwrap()).unwrap();
                realm.maa[j].1 -= 1;
                realm.gold += MAA[realm.maa[j].0].2 * 8.0;
                if realm.maa[j].1 == 0 { realm.maa.remove(j); }
            }
            let held = realm.held.clone();
            let has = |name: &str| held.iter().any(|&r| RESOURCES[r].name == name);
            let people = realm.people;
            let mut avail: Vec<&str> = avail_all.iter().map(|&m| if m == "culture" { CULTURE_UNIT[people] } else { m })
                .filter(|&m| (!CAVALRY.contains(&m) || has("horses") || has("war beasts")) && (m != "War Mages" || has("mana crystals"))
                        && (m != "Wolf Riders" || has("war beasts") || has("horses"))).collect();
            let mage = realm.council[3];
            if avail.contains(&"War Mages") && (mage == NONE || self.tier(&self.chars[mage]) < 3) { avail.retain(|&m| m != "War Mages"); }
            let realm = &self.realms[k];
            let kept: f64 = realm.maa.iter().map(|&(m, n)| MAA[m].2 * n as f64).sum();
            let need = at_war || realm.agenda == "conquest" || realm.expansionist;
            if !avail.is_empty() && realm.gold > 120.0 + 4.0 * kept && (need || self.rng.random() < 0.2) {
                let mut best = (f64::MIN, 0usize);
                for &m in &avail {
                    let j = MAA.iter().position(|x| x.0 == m).unwrap();
                    let s = MAA[j].1 / MAA[j].2 + self.rng.random() * 0.3;
                    if s > best.0 { best = (s, j); }
                }
                let realm = &mut self.realms[k];
                match realm.maa.iter_mut().find(|x| x.0 == best.1) { Some(x) => x.1 += 1, None => realm.maa.push((best.1, 1)) }
                realm.gold -= MAA[best.1].2 * 40.0;
            }
            let realm = &mut self.realms[k];
            let maa_power: f64 = realm.maa.iter().map(|&(m, n)| MAA[m].1 * n as f64 * 300.0).sum();
            realm.army_power = realm.levy + maa_power;
            realm.battle_mult *= 1.0 + (maa_power / realm.levy.max(1.0)).min(2.0);
            let gb = realm.garrison_bonus;
            for &t in &towns {
                let s = &mut self.towns[t];
                s.walls = s.walls.min(walls_cap);
                s.garrison = s.pop * (0.02 + gb) * (1.0 + 0.2 * s.walls as f64);
            }
        }
    }

    fn laws_year(&mut self) {
        let y = self.year;
        for k in 0..self.realms.len() {
            if !self.active(k) { continue; }
            let r = self.realms[k].ruler;
            let ci = self.realms[k].people;
            if self.realms[k].agenda_for != r {
                self.realms[k].agenda_for = r;
                let rc = &self.chars[r];
                let p = |t: &str| rc.has_trait(t) as u8 as f64;
                let eth = ETHOS[ci];
                let w = [("conquest", 1.0 + 2.0 * p("ambitious") + 2.0 * p("wrathful") + (eth == "Bellicose") as u8 as f64),
                         ("building", 1.0 + 2.0 * p("diligent") + (eth == "Stoic") as u8 as f64),
                         ("faith", 1.0 + 3.0 * p("zealous") + (eth == "Spiritual") as u8 as f64),
                         ("learning", 1.0 + rc.attr(4, y) / 5.0),
                         ("trade", 1.0 + 2.0 * p("greedy") + (eth == "Courtly") as u8 as f64),
                         ("unity", 1.0 + 2.0 * p("kind") + 2.0 * p("calm"))];
                let (zealous, greedy) = (rc.has_trait("zealous"), rc.has_trait("greedy"));
                let ws: Vec<f64> = w.iter().map(|x| x.1).collect();
                let agenda = w[self.rng.weighted(&ws)].0;
                let realm = &mut self.realms[k];
                realm.agenda = agenda;
                realm.expansionist = agenda == "conquest";
                realm.tolerant = !zealous;
                realm.trade_protectionist = greedy && agenda != "trade";
            }
            let cap = self.cultures[ci].best("crown", 1.0) as u32;
            let v = self.views(r);
            if self.realms[k].crown_law < cap && self.chars[r].legitimacy.unwrap_or(0.0) >= 60.0 && v[0] > 0.2 && self.rng.random() < 0.05 {
                self.realms[k].crown_law += 1;
                let text = format!("{} raises the crown's authority in {} (level {}).", self.who(r), self.realms[k].name, self.realms[k].crown_law);
                self.say(text, true);
            }
            let realm = &mut self.realms[k];
            realm.aggr *= if realm.expansionist { 1.4 } else { 0.8 };
            match realm.agenda {
                "building" => { for t in self.alive_towns_of(k).into_iter().take(5) { self.towns[t].wealth += 3.0; } }
                "trade" => { for t in self.alive_towns_of(k) { if self.towns[t].port { self.towns[t].wealth += 2.0; } } }
                _ => {}
            }
        }
    }

    pub fn change_gov(&mut self, k: usize, gov: &str, why: &str) {
        let old = self.realms[k].gov;
        let gov = gov_static(gov);
        self.realms[k].gov = gov;
        match gov { "clan" => self.realms[k].law = "seniority", "feudal" => self.realms[k].law = "primogeniture",
                    "republic" | "theocracy" | "administrative" => self.realms[k].law = "elective", _ => {} }
        let place = if self.realms[k].place.is_empty() { self.realms[k].short.clone() } else { self.realms[k].place.clone() };
        if gov == "republic" || gov == "theocracy" {
            self.realms[k].name = crate::titles::style_for(self.realms[k].people, gov, self.realms[k].rank, &place);
        }
        let tail = if gov == "republic" || gov == "theocracy" { format!(", and the realm is {} now", self.realms[k].name) } else { String::new() };
        self.say(format!("**A new order.** {why}: {old} rule gives way to {gov} rule{tail}."), false);
    }

    fn decisions_year(&mut self) {
        let mut held = vec![0u32; self.realms.len()];
        for kd in self.ktitles.values() { if kd.ctrl != NONE { held[kd.ctrl] += 1; } }
        let mut duch = vec![0u32; self.realms.len()];
        for d in self.duchies.values() { if d.ctrl != NONE { duch[d.ctrl] += 1; } }
        let y = self.year;
        for k in 0..self.realms.len() {
            if !self.active(k) { continue; }
            let r = self.realms[k].ruler;
            let ci = self.realms[k].people;
            let era = self.cultures[ci].era;
            let n = self.alive_towns_of(k).len();
            let cap = self.capital_of(k);
            let place = if !self.realms[k].place.is_empty() { self.realms[k].place.clone() } else if cap != NONE { self.towns[cap].name.clone() } else { self.realms[k].short.clone() };
            let rank = RANKS.iter().position(|x| *x == self.realms[k].rank).unwrap_or(2);
            let fem = self.chars[r].female;
            let want: Option<(&'static str, String, f64)> =
                if rank == 0 && n >= 5 { Some(("chiefdom", format!("unites the clans around {place}"), 0.0)) }
                else if rank == 1 && duch[k] >= 2 && n >= 12 && era >= 1 { Some(("duchy", format!("takes the title of {}", if fem { "Duchess" } else { "Duke" }), 80.0)) }
                else if rank == 2 && held[k] >= 1 && era >= 1 && n >= 25 { Some(("kingdom", "is crowned and anointed".into(), 200.0)) }
                else if rank == 3 && held[k] >= 3 && era >= 2 && n >= 70 { Some(("empire", "proclaims an empire".into(), 500.0)) }
                else { None };
            if let Some((new, what, cost)) = want {
                if self.realms[k].gold >= cost && self.rng.random() < 0.3 {
                    self.realms[k].gold -= cost;
                    let old = self.realms[k].name.clone();
                    self.realms[k].rank = new;
                    self.realms[k].place = place.clone();
                    self.realms[k].name = crate::titles::style_for(ci, self.realms[k].gov, new, &place);
                    let b = if new == "kingdom" || new == "empire" { "**" } else { "" };
                    let text = format!("{b}{} {what}: {old} becomes {}.{b}", self.who(r), self.realms[k].name);
                    self.say(text, new == "chiefdom");
                    self.chars[r].renown += 50.0 * (RANKS.iter().position(|x| *x == new).unwrap() + 1) as f64;
                }
            }
            let g = self.realms[k].gov;
            let rank = RANKS.iter().position(|x| *x == self.realms[k].rank).unwrap_or(2);
            let mut nxt = None;
            if g == "tribal" && self.cultures[ci].has("feudal") && rank >= 2 {
                nxt = Some(if ci == 2 || ci == 3 { "clan" } else { "feudal" });
            } else if (g == "feudal" || g == "clan") && self.cultures[ci].has("admin") && self.chars[r].attr(2, y) >= 10.0 && self.rng.random() < 0.3 {
                nxt = Some("administrative");
            }
            if let Some(nx) = nxt {
                if self.rng.random() < 0.1 {
                    let why = format!("{} reforms the rule of {}", self.who(r), self.realms[k].name);
                    self.change_gov(k, nx, &why);
                }
            }
            self.confederate(k, r);
        }
    }

    /// Tribes of one people at peace, bound by marriage or friendship, may join under the stronger chief.
    fn confederate(&mut self, k: usize, r: usize) {
        if !matches!(self.realms[k].rank, "tribe" | "chiefdom") || self.rng.random() > 0.05 { return; }
        let mine = self.alive_towns_of(k).len();
        for o in 0..self.realms.len() {
            if o == k || self.realms[o].extinct || self.realms[o].people != self.realms[k].people || !matches!(self.realms[o].rank, "tribe" | "chiefdom") { continue; }
            if self.war[k][o] || self.realms[o].ruler == NONE { continue; }
            let theirs = self.alive_towns_of(o).len();
            if theirs >= mine || (self.relation[k][o] < 20.0 && !self.realms[k].allies.contains(&o)) { continue; }
            let orul = self.realms[o].ruler;
            if self.chars[orul].has_trait("ambitious") && self.rng.random() < 0.7 { continue; }
            if self.realms[k].people == 3 && (self.relation[k][o] < 60.0 || self.rng.random() > 0.2) { continue; }
            let oname = self.realms[o].name.clone();
            let orn = self.chars[orul].name.clone();
            self.absorb(k, o);
            let text = format!("The chief of {oname}, {orn}, swears to {}; the two peoples are one now under {}.", self.who(r), self.realms[k].name);
            self.say(text, false);
            return;
        }
    }

    fn reform_year(&mut self) {
        let y = self.year;
        for k in 0..self.realms.len() {
            if !self.active(k) { continue; }
            let r = self.realms[k].ruler;
            let council = self.realms[k].council;
            let great: Vec<usize> = self.chars.iter().filter(|c| c.alive && c.realm == k && c.adult(y)
                && (!c.duchies.is_empty() || !c.lordships.is_empty() || matches!(c.profession, Some("merchant") | Some("priest")) || council.contains(&c.id)))
                .map(|c| c.id).collect();
            if great.len() < 3 { continue; }
            let mut agree = 0.0;
            let mut sent = [0.0; 5];
            for &c in &great {
                agree += self.agreement(c, k);
                let v = self.views(c);
                for a in 0..5 { sent[a] += v[a]; }
            }
            let n = great.len() as f64;
            agree /= n;
            for s in sent.iter_mut() { *s /= n; }
            self.realms[k].agreement = agree;
            self.realms[k].reform = (self.realms[k].reform + (0.45 - agree) * 0.5).max(0.0);
            if self.realms[k].reform < 3.0 || self.rng.random() > 0.2 { continue; }
            let cu = &self.cultures[self.realms[k].people];
            let towns = self.alive_towns_of(k);
            let ports = towns.iter().filter(|&&t| self.towns[t].port).count() as f64 / towns.len().max(1) as f64;
            let g = self.realms[k].gov;
            let target = if sent[3] > 0.45 && sent[1] < -0.1 && cu.has("republic") && g != "republic" && ports >= 0.3 && matches!(self.realms[k].rank, "duchy" | "kingdom") { Some("republic") }
                else if sent[2] > 0.45 && cu.has("theocracy") && g != "theocracy" { Some("theocracy") }
                else if sent[0] < -0.2 && self.realms[k].crown_law > 1 { Some("crown") } else { None };
            let Some(target) = target else { self.realms[k].reform *= 0.5; continue };
            self.realms[k].reform = 0.0;
            let want = if target == "crown" { "a charter for the crown".to_string() } else { format!("{target} rule") };
            let rc = &self.chars[r];
            let yields = rc.legitimacy.unwrap_or(50.0) < 40.0 || rc.has_trait("content") || rc.has_trait("kind");
            if !yields {
                let win = self.rng.random() < 0.45;
                for t in self.alive_towns_of(k) { let f = 1.0 - self.rng.uniform(0.02, 0.08); self.towns[t].pop *= f; }
                let text = format!("**A reform war.** The great of {} rise for {want}; {} {}.", self.realms[k].name, self.who(r), if win { "falls" } else { "crushes them" });
                self.say(text, false);
                if !win { self.chars[r].tyranny = (self.chars[r].tyranny + 5.0).min(100.0); continue; }
                let cands: Vec<usize> = self.pool_of(k).into_iter().filter(|&c| c != r).collect();
                self.kill_by(r, "was overthrown by the reformers".into(), true, NONE);
                self.end_reign(k, "overthrown by reformers");
                if target == "republic" || target == "theocracy" {
                    let why = format!("The reformers of {} remake the realm", self.realms[k].name);
                    self.change_gov(k, target, &why);
                } else if !cands.is_empty() {
                    self.realms[k].crown_law = (self.realms[k].crown_law.max(3) - 2).max(1);
                }
                if !cands.is_empty() {
                    let mut best = (f64::MIN, NONE);
                    for c in cands { let s = self.claim_of(c); if s > best.0 { best = (s, c); } }
                    self.crown(k, best.1, "raised up when the line failed");
                }
                continue;
            }
            if target == "crown" {
                self.realms[k].crown_law = (self.realms[k].crown_law - 1).max(1);
                let text = format!("{} yields to the reformers of {} and lowers the crown's authority.", self.who(r), self.realms[k].name);
                self.say(text, true);
            } else {
                let why = format!("{} yields to the reformers of {}", self.who(r), self.realms[k].name);
                self.change_gov(k, target, &why);
            }
        }
    }

    fn culture_events(&mut self) {
        let y = self.year;
        for ci in 0..4 {
            let ks: Vec<usize> = (0..self.realms.len()).filter(|&k| self.active(k) && self.realms[k].people == ci).collect();
            if ks.is_empty() { continue; }
            let roll = self.rng.random();
            let k = *ks.iter().max_by_key(|&&k| self.alive_towns_of(k).len()).unwrap();
            let cap = self.capital_of(k);
            if cap == NONE { continue; }
            let at_war = self.war[k].iter().any(|&w| w);
            let cname = self.towns[cap].name.clone();
            if roll < 0.006 && self.cultures[ci].golden < y && !at_war {
                self.cultures[ci].golden = y + self.rng.integers(20, 50);
                self.cultures[ci].history.push((y, "a golden age begins".into()));
                self.say(format!("**A golden age.** The {} flourish: songs, scholars and new ideas pour out of {cname}.", PEOPLES[ci]), false);
            } else if roll < 0.010 && self.cultures[ci].dark < y && (at_war || y - self.last_plague < 5) {
                self.cultures[ci].dark = y + self.rng.integers(15, 40);
                self.cultures[ci].history.push((y, "a dark age begins".into()));
                self.say(format!("**A dark age.** War and pestilence close the schools of the {}; much is forgotten.", PEOPLES[ci]), false);
            } else if roll < 0.016 && self.realms[k].gold > 300.0 {
                let name = GREAT_WORKS[self.rng.integers(0, GREAT_WORKS.len() as i64) as usize].replace("{}", &cname);
                if !self.realms[k].great_works.contains(&name) {
                    self.realms[k].gold -= 300.0;
                    self.realms[k].great_works.push(name.clone());
                    self.towns[cap].wealth += 100.0;
                    let r = self.realms[k].ruler;
                    let l = self.chars[r].legitimacy.unwrap_or(50.0);
                    self.chars[r].legitimacy = Some((l + 10.0).min(100.0));
                    let text = format!("**A great work.** {} completes {name}.", self.who(r));
                    self.say(text, false);
                }
            } else if roll < 0.022 {
                // (the saga of a great hero: slice 6)
            } else if roll < 0.026 {
                let d = if self.rng.random() < 0.5 { -0.2 } else { 0.2 };
                self.cultures[ci].trend[4] += d;
                let what = if self.cultures[ci].trend[4] < 0.0 { "a movement of new thinkers" } else { "a return to the old ways" };
                self.cultures[ci].history.push((y, what.into()));
                self.say(format!("Among the {}, {what} takes hold.", PEOPLES[ci]), true);
            }
        }
    }

    pub fn civics_year(&mut self) {
        self.research_year();
        self.trends_year();
        self.laws_year();
        self.treasury_year();
        self.decisions_year();
        self.reform_year();
        self.culture_events();
    }
}
