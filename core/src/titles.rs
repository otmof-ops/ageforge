//! Titles, vassals, factions and the birth of new realms (titles.py), with partition of tribal inheritances and
//! the union of two crowns on one head (dynasty.partition, dynasty.unite_crowns, civics.absorb).

use crate::people::*;
use crate::state::State;
use crate::tables::*;

const DUCHY: i32 = 24;
const KTITLE: i32 = 72;
const PARTITION: [f64; 4] = [0.35, 0.35, 0.35, 1.0];

pub fn style(people: usize, rank: &str, place: &str) -> String {
    let f = match rank {
        "empire" => ["the Empire of {}", "the High Court of {}", "the Deep Empire of {}", "the Great Horde of {}"][people],
        "kingdom" => REALM_STYLE[people],
        _ => ["the Duchy of {}", "the Elven March of {}", "the Hold of {}", "the Warband of {}"][people],
    };
    f.replace("{}", place)
}

pub fn style_for(people: usize, gov: &str, rank: &str, place: &str) -> String {
    match (gov, rank) {
        ("republic", _) => format!("the Republic of {place}"),
        ("theocracy", _) => format!("the Holy See of {place}"),
        (_, "tribe") => ["the {} Tribe", "the Kin of {}", "the {} Clans", "the {} Warband"][people].replace("{}", place),
        (_, "chiefdom") => format!("the Chiefdom of {place}"),
        _ => style(people, rank, place),
    }
}

impl State {
    pub fn capital_of(&self, k: usize) -> usize {
        self.towns.iter().find(|t| t.alive && t.realm == k && t.capital).map(|t| t.id).unwrap_or(NONE)
    }

    pub fn land_name(&self, l: i32) -> String {
        if l >= 0 { self.map.lands[l as usize].name.clone() } else { "the open sea".into() }
    }

    /// Who gets what a person leaves (governance.heir_of): the eldest child of their people, the spouse, the
    /// eldest of the house.
    pub fn heir_of(&self, c: usize) -> usize {
        let cc = &self.chars[c];
        let mut kids: Vec<usize> = cc.children.iter().copied().filter(|&i| self.chars[i].alive && self.chars[i].people == cc.people).collect();
        kids.sort_by_key(|&i| self.chars[i].born);
        if let Some(&k) = kids.first() { return k; }
        if cc.spouse != NONE && self.chars[cc.spouse].alive { return cc.spouse; }
        if cc.house != NONE {
            return self.chars.iter().filter(|x| x.alive && x.house == cc.house).min_by_key(|x| x.born).map(|x| x.id).unwrap_or(NONE);
        }
        NONE
    }

    pub fn ensure_titles(&mut self) {
        for t in 0..self.towns.len() {
            if !self.towns[t].alive { continue; }
            let (l, r, c) = (self.towns[t].landmass, self.towns[t].pos.0 as i32, self.towns[t].pos.1 as i32);
            let dk = (l, r / DUCHY, c / DUCHY);
            let kt = (l, r / KTITLE, c / KTITLE);
            if !self.duchies.contains_key(&dk) {
                let name = format!("the Duchy of {}", self.towns[t].name);
                self.duchies.insert(dk, Duchy { name, holder: NONE, kt, since: self.year, land: l, ctrl: NONE });
            }
            if !self.ktitles.contains_key(&kt) {
                let small = l >= 0 && self.map.lands[l as usize].size < 3000;
                let name = if small { format!("the Kingdom of {}", self.land_name(l)) } else { format!("the Kingdom of {}", self.towns[t].name) };
                self.ktitles.insert(kt, KTitle { name, ctrl: NONE });
            }
            self.towns[t].duchy = Some(dk);
        }
    }

    /// Who controls each duchy (most of its towns) and each kingdom title (most of its duchies), this year.
    pub fn control(&mut self) {
        let mut counts: std::collections::BTreeMap<DKey, Vec<(usize, u32)>> = Default::default();
        for t in self.towns.iter().filter(|t| t.alive) {
            let v = counts.entry(t.duchy.unwrap()).or_default();
            match v.iter_mut().find(|x| x.0 == t.realm) { Some(x) => x.1 += 1, None => v.push((t.realm, 1)) }
        }
        let mut kc: std::collections::BTreeMap<DKey, Vec<(usize, u32)>> = Default::default();
        for (dk, d) in self.duchies.iter_mut() {
            d.ctrl = counts.get(dk).and_then(|c| c.iter().max_by_key(|x| x.1).map(|x| x.0)).unwrap_or(NONE);
            if d.ctrl != NONE {
                let v = kc.entry(d.kt).or_default();
                match v.iter_mut().find(|x| x.0 == d.ctrl) { Some(x) => x.1 += 1, None => v.push((d.ctrl, 1)) }
            }
        }
        for (kt, kd) in self.ktitles.iter_mut() {
            kd.ctrl = kc.get(kt).and_then(|c| c.iter().max_by_key(|x| x.1).map(|x| x.0)).unwrap_or(NONE);
        }
    }

    /// A realm's rank follows the kingdom titles it holds.
    pub fn rank_names(&mut self) {
        let mut held = vec![0u32; self.realms.len()];
        for kd in self.ktitles.values() { if kd.ctrl != NONE { held[kd.ctrl] += 1; } }
        for k in 0..self.realms.len() {
            if self.realms[k].extinct { continue; }
            let n = held[k];
            let rank = if n >= 3 { "empire" } else if n >= 1 { "kingdom" } else { "duchy" };
            if rank == self.realms[k].rank { continue; }
            let cap = self.capital_of(k);
            if self.realms[k].place.is_empty() { self.realms[k].place = if cap != NONE { self.towns[cap].name.clone() } else { self.realms[k].short.clone() }; }
            let old = self.realms[k].name.clone();
            let new = style(self.realms[k].people, rank, &self.realms[k].place);
            let was = self.realms[k].rank;
            self.realms[k].name = new.clone();
            if rank == "empire" {
                self.say(format!("**An empire.** Holding {n} kingdom titles, {old} proclaims itself {new}."), false);
            } else if was == "empire" {
                self.say(format!("{old} has lost its kingdoms; it is {new} now."), false);
            } else if rank == "kingdom" && was == "duchy" {
                self.say(format!("{old} raises itself to a kingdom: {new}."), true);
            }
            self.realms[k].rank = rank;
        }
    }

    fn grant_duchies(&mut self) {
        let y = self.year;
        let keys: Vec<DKey> = self.duchies.keys().copied().collect();
        for k in 0..self.realms.len() {
            if !self.active(k) { continue; }
            let r = self.realms[k].ruler;
            let cap = self.capital_of(k);
            let cap_duchy = if cap != NONE { self.towns[cap].duchy } else { None };
            for &dk in &keys {
                if self.duchies[&dk].ctrl != k { continue; }
                let h = self.duchies[&dk].holder;
                if Some(dk) == cap_duchy { self.duchies.get_mut(&dk).unwrap().holder = r; continue; }
                if h != NONE && self.chars[h].alive && self.chars[h].realm == k && h != r { continue; }
                if h != NONE && !self.chars[h].alive {
                    let heir = self.heir_of(h);
                    if heir != NONE && self.chars[heir].alive && self.chars[heir].realm == k && heir != r {
                        self.give_duchy(heir, dk, k, true);
                        continue;
                    }
                }
                let cands: Vec<usize> = self.chars.iter().filter(|c| c.alive && c.realm == k && c.adult(y) && c.id != r && !c.undead
                    && c.disgraced_until <= y && c.duchies.len() < 2 && (c.house != NONE || c.renown >= 200.0)).map(|c| c.id).collect();
                if cands.is_empty() { self.duchies.get_mut(&dk).unwrap().holder = NONE; continue; }
                let near: Vec<usize> = cands.iter().copied().filter(|&c| self.chars[c].lordships.iter()
                    .any(|&s| self.towns[s].alive && self.towns[s].duchy == Some(dk))).collect();
                let pool = if near.is_empty() { cands } else { near };
                let rh = self.chars[r].house;
                let mut best = (f64::MIN, NONE);
                for c in pool {
                    let cc = &self.chars[c];
                    let s = cc.renown / 50.0 + if cc.house != NONE { self.houses[cc.house].prestige / 200.0 } else { 0.0 }
                        + if cc.house == rh { 8.0 } else { 0.0 } + self.rng.random() * 5.0;
                    if s > best.0 { best = (s, c); }
                }
                self.give_duchy(best.1, dk, k, false);
            }
        }
    }

    pub fn give_duchy_of(&mut self, c: usize, dk: DKey, k: usize) { self.give_duchy(c, dk, k, false); }

    fn give_duchy(&mut self, c: usize, dk: DKey, k: usize, inherited: bool) {
        let old = self.duchies[&dk].holder;
        if old != NONE { self.chars[old].duchies.retain(|&x| x != dk); }
        self.duchies.get_mut(&dk).unwrap().holder = c;
        self.chars[c].duchies.push(dk);
        let r = self.realms[k].ruler;
        if !inherited && r != NONE { self.remember(c, r, "granted me a title", 25.0, 0.5); }
        let place = self.duchies[&dk].name.replace("the Duchy of ", "");
        let t = format!("{} of {place} from year {}", if self.chars[c].female { "Duchess" } else { "Duke" }, self.year);
        self.chars[c].titles.push(t);
        if self.chars[c].legitimacy.is_none() { self.chars[c].legitimacy = Some(55.0); }
        let cc = &self.chars[c];
        if !inherited && cc.house != NONE && r != NONE && self.rng.random() < 0.3 && self.houses[cc.house].founder != c
            && cc.father != NONE && self.houses[cc.house].rulers > 0 {
            let parent = cc.house;
            let name = HOUSE_STYLE[cc.people].replace("{}", &place);
            if self.houses.iter().any(|h| h.name == name) { return; }
            let track = self.houses[parent].track.clone();
            let h = self.new_house(c, Some(place.clone()));
            let pname = self.houses[parent].name.clone();
            let full = self.chars[c].name.clone();
            let ttl = if self.chars[c].female { "Duchess" } else { "Duke" };
            let pp = self.houses[parent].prestige;
            let ph = &mut self.houses[h];
            ph.track = track;
            ph.origin = format!("a cadet branch of {pname}, founded in year {} by {full}, {ttl} of {place}", self.year);
            ph.prestige = pp * 0.1;
            let text = format!("{} founds {}, a cadet branch of {pname}.", self.who(c), self.houses[h].name);
            self.say(text, true);
        }
    }

    /// A duke's opinion of their liege: the full opinion, the pull of distance and of their own power.
    fn duke_opinion(&mut self, duke: usize, k: usize, r: usize, dk: DKey) -> f64 {
        if self.chars[duke].op_bias.is_none() { self.chars[duke].op_bias = Some(self.rng.normal(0.0, 10.0)); }
        let d = &self.duchies[&dk];
        let cap = self.capital_of(k);
        let mut op = self.opinion(duke, r) + 15.0;
        if cap != NONE && d.land != self.towns[cap].landmass { op -= 35.0 + 10.0 * ((self.year - d.since) / 100).min(5) as f64; }
        op -= self.duchies.values().filter(|x| x.ctrl == k).count() as f64 / 8.0;
        op -= 2.0 * self.towns.iter().filter(|t| t.alive && t.realm == k && t.duchy == Some(dk)).count() as f64;
        let dc = &self.chars[duke];
        if dc.has_trait("ambitious") { op -= 10.0; }
        if dc.has_trait("content") { op += 10.0; }
        op
    }

    fn factions_year(&mut self) {
        let y = self.year;
        for k in 0..self.realms.len() {
            if !self.active(k) { continue; }
            let r = self.realms[k].ruler;
            if !self.chars[r].adult(y) { continue; }
            let cap = self.capital_of(k);
            if cap == NONE { continue; }
            let cap_land = self.towns[cap].landmass;
            let mut groups: Vec<((&'static str, i32), Vec<(usize, DKey)>)> = vec![];
            let keys: Vec<DKey> = self.duchies.iter().filter(|(_, d)| d.ctrl == k && d.holder != NONE && d.holder != r).map(|(k, _)| *k).collect();
            for dk in keys {
                let duke = self.duchies[&dk].holder;
                if !self.chars[duke].alive || self.chars[duke].realm != k { continue; }
                let op = self.duke_opinion(duke, k, r, dk);
                if self.dread(r) > self.boldness(duke) || self.chars[r].hooks.iter().any(|h| h.0 == duke) { continue; }
                let dc = &self.chars[duke];
                let unhappy = dc.happy < -30.0;
                if op >= 0.0 && !(op < 20.0 && (dc.has_trait("ambitious") || unhappy)) { continue; }
                let land = self.duchies[&dk].land;
                let key = if land != cap_land || dc.people != self.chars[r].people { ("independence", land) }
                          else if self.chars[r].tyranny >= 40.0 { ("liberty", -1) } else { ("claimant", -1) };
                match groups.iter_mut().find(|g| g.0 == key) { Some(g) => g.1.push((duke, dk)), None => groups.push((key, vec![(duke, dk)])) }
            }
            self.realms[k].factions.clear();
            let keys: Vec<(&str, i32)> = groups.iter().map(|g| g.0).collect();
            self.realms[k].faction_since.retain(|f| keys.contains(&f.0));
            let at_war = self.war[k].iter().any(|&w| w);
            for ((kind, land), members) in groups {
                let dks: Vec<DKey> = members.iter().map(|m| m.1).collect();
                let strength = |t: &Town| t.pop * MARTIAL[t.culture];
                let fp0: f64 = self.towns.iter().filter(|t| t.alive && t.duchy.is_some_and(|d| dks.contains(&d))).map(strength).sum();
                let rest: Vec<usize> = self.alive_towns_of(k).into_iter().filter(|&t| !self.towns[t].duchy.is_some_and(|d| dks.contains(&d))).collect();
                let bm = self.realms[k].battle_mult.min(2.0);
                let lp = if kind == "independence" {
                    let local: f64 = rest.iter().filter(|&&t| self.towns[t].landmass == land).map(|&t| strength(&self.towns[t])).sum();
                    let carried: f64 = rest.iter().filter(|&&t| self.towns[t].landmass != land).map(|&t| strength(&self.towns[t])).sum::<f64>()
                        * (0.05 * (1.0 + self.realms[k].ship_tier as f64) * self.realms[k].warships.max(1.0).sqrt()).min(0.5);
                    (local + carried) * bm
                } else {
                    rest.iter().map(|&t| strength(&self.towns[t])).sum::<f64>() * bm
                };
                let mut ratio = fp0 / lp.max(1.0);
                if at_war { ratio *= 1.25; }
                let mut uniq: Vec<usize> = members.iter().map(|m| m.0).collect();
                uniq.sort();
                uniq.dedup();
                let leader = *uniq.iter().max_by(|&&a, &&b| {
                    let s = |c: usize| self.chars[c].renown + self.chars[c].attr(1, y) * 10.0;
                    s(a).partial_cmp(&s(b)).unwrap()
                }).unwrap();
                // (titles.py also scales the faction's strength by its leader and its age, but after the ratio is
                // taken, so neither counts: see PORT.md "Python bugs")
                if !self.realms[k].faction_since.iter().any(|f| f.0 == (kind, land)) { self.realms[k].faction_since.push(((kind, land), y)); }
                self.realms[k].factions.push((kind, land, uniq.clone(), leader, ratio));
                let cool = self.realms[k].ultimatum_cooldown.iter().find(|c| c.0 == kind).map(|c| c.1).unwrap_or(0);
                if ratio < 0.7 || y < cool { continue; }
                match self.realms[k].ultimatum_cooldown.iter_mut().find(|c| c.0 == kind) { Some(c) => c.1 = y + 15, None => self.realms[k].ultimatum_cooldown.push((kind, y + 15)) }
                self.ultimatum(k, r, kind, land, &members, &uniq, leader, ratio);
                break;
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn ultimatum(&mut self, k: usize, r: usize, kind: &'static str, land: i32, members: &[(usize, DKey)], uniq: &[usize], leader: usize, ratio: f64) {
        let demand = match kind {
            "independence" => if land >= 0 { format!("the freedom of {}", self.land_name(land)) } else { "independence".into() },
            "liberty" => "an end to tyranny".into(),
            _ => "a new ruler".into(),
        };
        let rc = &self.chars[r];
        let accept = ratio > 1.6 || (rc.has_trait("content") && kind != "claimant") || (kind == "liberty" && rc.has_trait("kind"));
        let mut names = uniq.iter().take(4).map(|&d| self.who(d)).collect::<Vec<_>>().join(", ");
        if uniq.len() > 4 { names += &format!(" and {} more", uniq.len() - 4); }
        if accept {
            let text = format!("**An ultimatum.** {names} demand {demand} of {}, who yields.", self.who(r));
            self.say(text, false);
            self.resolve(k, r, kind, members, leader, true, 0);
            return;
        }
        let yrs = self.rng.integers(1, 6);
        let fp = ratio * self.rng.uniform(0.6, 1.4);
        let lp = self.rng.uniform(0.6, 1.4) * (1.0 + 0.1 * self.realms[k].allies.len() as f64);
        let won = fp > lp;
        for t in self.alive_towns_of(k) { let f = 1.0 - self.rng.uniform(0.03, 0.12); self.towns[t].pop *= f; }
        let text = format!("**A {kind} war.** {names} demand {demand}; {} refuses. After {yrs} year{} of war the faction {}.",
                           self.who(r), if yrs > 1 { "s" } else { "" }, if won { "wins" } else { "is crushed" });
        self.say(text, false);
        self.resolve(k, r, kind, members, leader, won, yrs);
    }

    #[allow(clippy::too_many_arguments)]
    fn resolve(&mut self, k: usize, r: usize, kind: &str, members: &[(usize, DKey)], leader: usize, won: bool, war_years: i64) {
        if !won {
            for &(d, dk) in members {
                self.duchies.get_mut(&dk).unwrap().holder = NONE;
                self.chars[d].duchies.retain(|&x| x != dk);
                self.remember(d, r, "revoked my title", -40.0, 0.5);
                if d == leader && self.chars[d].alive && self.rng.random() < 0.6 {
                    self.remember_kin(d, r, "executed my kin", -60.0);
                    let why = format!("was executed for leading a {kind} faction against {} {}", self.chars[r].name, self.chars[r].regnal.clone().unwrap_or_default());
                    self.kill_by(d, why, true, r);
                    self.chars[r].tyranny = (self.chars[r].tyranny + 6.0).min(100.0);
                } else {
                    self.chars[d].disgraced_until = self.year + 30;
                }
            }
            let l = self.chars[r].legitimacy.unwrap_or(50.0);
            self.chars[r].legitimacy = Some((l + 5.0).min(100.0));
            return;
        }
        match kind {
            "independence" => { let dks: Vec<DKey> = members.iter().map(|m| m.1).collect(); self.secede(k, &dks, leader, war_years); }
            "liberty" => {
                let rc = &mut self.chars[r];
                rc.tyranny = (rc.tyranny - 35.0).max(0.0);
                rc.legitimacy = Some((rc.legitimacy.unwrap_or(50.0) - 10.0).max(0.0));
                for &(d, _) in members { let b = self.chars[d].op_bias.unwrap_or(0.0); self.chars[d].op_bias = Some(b + 15.0); }
                let text = format!("{} binds the crown: a charter of liberties for the great of {}.", self.who(r), self.realms[k].name);
                self.say(text, false);
            }
            _ => {
                let cands: Vec<usize> = self.pool_of(k).into_iter().filter(|&c| c != r).collect();
                let claimant = if cands.is_empty() { leader } else {
                    let mut best = (f64::MIN, NONE);
                    for c in cands { let s = self.claim_of(c); if s > best.0 { best = (s, c); } }
                    best.1
                };
                self.kill_by(r, "was deposed by a faction of the great and did not survive it".into(), true, NONE);
                self.end_reign(k, "deposed by a faction");
                self.crown(k, claimant, "raised up when the line failed");
            }
        }
    }

    /// Give duchies a realm of their own, with a leader on the throne.
    pub fn secede(&mut self, k: usize, dks: &[DKey], leader: usize, war_years: i64) {
        let towns: Vec<usize> = (0..self.towns.len()).filter(|&t| self.towns[t].alive && self.towns[t].duchy.is_some_and(|d| dks.contains(&d))).collect();
        if towns.is_empty() { return; }
        let cap = *towns.iter().max_by(|&&a, &&b| self.towns[a].pop.partial_cmp(&self.towns[b].pop).unwrap()).unwrap();
        let nid = self.realms.len();
        let mut by_culture = [0.0; 4];
        for &t in &towns { by_culture[self.towns[t].culture] += self.towns[t].pop; }
        let culture = (0..4).max_by(|&a, &b| by_culture[a].partial_cmp(&by_culture[b]).unwrap()).unwrap();
        let place = self.towns[cap].name.clone();
        let mut color = COLORS[culture];
        for v in color.iter_mut() { *v = (*v as f64 * self.rng.uniform(0.55, 1.25)).clamp(30.0, 255.0) as u8; }
        let gov = self.realms[k].gov;
        let rank = if towns.len() >= 12 { "duchy" } else { "chiefdom" };
        let name = style_for(culture, gov, rank, &place);
        self.new_realm(culture, name, format!("{} of {place}", PEOPLES[culture]), rank, gov, color, place.clone());
        let law = self.realms[k].law;
        let crown_law = self.realms[k].crown_law;
        let (hl, known) = (self.towns[cap].landmass, self.realms[k].known.clone());
        let nk = &mut self.realms[nid];
        nk.home_land = hl;
        nk.known = known;
        nk.parent = k;
        nk.founded_year = self.year;
        nk.law = law;
        nk.crown_law = crown_law;
        for &t in &towns { self.towns[t].capital = false; self.towns[t].realm = nid; }
        self.towns[cap].capital = true;
        for i in 0..self.chars.len() {
            let c = &self.chars[i];
            if !c.alive || c.realm != k { continue; }
            let home_here = c.home != NONE && self.towns[c.home].realm == nid;
            if home_here || i == leader || c.duchies.iter().any(|d| dks.contains(d)) || c.spouse == leader || self.chars[leader].children.contains(&i) {
                self.chars[i].realm = nid;
            }
        }
        self.relation[k][nid] = -40.0;
        self.relation[nid][k] = -40.0;
        self.truce[k][nid] = self.year + 20;
        self.truce[nid][k] = self.year + 20;
        let lname = self.land_name(self.towns[cap].landmass);
        let text = format!("**A new realm.** {lname} breaks from {}{}: {} is born, {} towns strong, with {} on its throne.",
                           self.realms[k].name, if war_years > 0 { format!(" after {war_years} years of war") } else { String::new() },
                           self.realms[nid].name, towns.len(), self.chars[leader].name);
        self.say(text, false);
        self.breakaways.push((self.year, nid, k, leader));
        self.crown(nid, leader, "leading the war of independence");
    }

    #[allow(clippy::too_many_arguments)]
    pub fn new_realm(&mut self, people: usize, name: String, short: String, rank: &'static str, gov: &'static str, color: [u8; 3], place: String) -> usize {
        let id = self.realms.len();
        self.realms.push(Realm { id, people, name, short, rank, gov, law: if people == 3 { "strongest" } else { "elective" }, color,
            ruler: NONE, house: NONE, regnal: vec![], reigns: vec![], place, extinct: false, growth_mult: 1.0, tax_now: 0.06,
            gold: 50.0, crown_law: 1, tolerant: true, expansionist: false, agenda: "unity", agenda_for: NONE, maa: vec![],
                army_power: 0.0, garrison_bonus: 0.0, aggr: 1.0, reform: 0.0, agreement: 1.0, great_works: vec![], held: vec![], home_land: -1, known: vec![], overlord: NONE,
                vassal_limit: 0.0, domain_limit: 0.0, over_vassals: 0.0, domain_penalty: 0.0, last_stand: false, security: 0.0,
                realm_tier: 0, fear_until: 0, corrupt_seats: vec![], done: vec![], great: vec![], tax_relief: 0, regent: NONE, taken: 0, lost: 0, civil_wars: 0, physician: false,
            parent: NONE, founded_year: self.year, ended: None, battle_mult: 1.0, warships: 4.0, ship_tier: 0, allies: vec![],
            factions: vec![], faction_since: vec![], ultimatum_cooldown: vec![], council: [NONE; 6], council_for: NONE,
            econ_distress: 0.0, corruption: 0.0, collect: 1.0, tax_mult: 1.0, levy: 0.0, income: 0.0, real_gdp: 0.0,
            credit: None, defaults: 0, rate: 0.0, money: 0.0, trade_val: 0.0, trade_net: 0.0, partners: 0,
            trade_protectionist: false, last_forced: -99, last_default: -99, mint_skill: 0.0, sector_boost: [0.0; 7],
            econ_prev_gdp: None, goods: vec![] });
        self.grow_tables();
        id
    }

    /// A realm folds into another: its towns and people change allegiance (civics.absorb).
    pub fn absorb(&mut self, k: usize, o: usize) {
        for t in 0..self.towns.len() { if self.towns[t].alive && self.towns[t].realm == o { self.towns[t].realm = k; self.towns[t].capital = false; } }
        for c in self.chars.iter_mut() { if c.alive && c.realm == o { c.realm = k; } }
        if self.realms[o].ruler != NONE { let n = format!("swore fealty to {}", self.realms[k].name); self.end_reign(o, &n); }
        self.realms[o].extinct = true;
        for i in 0..self.realms.len() { self.war[o][i] = false; self.war[i][o] = false; }
        self.realms[o].ended = Some(self.year);
    }

    /// A ruler who comes into a second crown holds both titles in one realm (dynasty.unite_crowns).
    pub fn unite_crowns(&mut self, mine: usize, other: usize, c: usize, reason: &str) {
        let old = self.realms[other].ruler;
        if old != NONE && old != c && self.chars[old].ruled.is_some_and(|r| r.2.is_none()) { self.end_reign(other, "the crowns were united"); }
        let name = self.realms[other].name.clone();
        self.absorb(mine, other);
        self.ensure_titles();
        self.control();
        self.rank_names();
        let text = format!("**Two crowns, one head.** {}, already ruling {}, comes into the crown of {name} ({reason}); both titles are held now by {}.",
                           self.who(c), self.realms[mine].name, self.realms[mine].name);
        self.say(text, false);
    }

    /// Tribal and clan custom divides a dead ruler's lands among the grown children (dynasty.partition).
    pub fn partition(&mut self, k: usize, old: usize, heir: usize) {
        let y = self.year;
        if self.realms[k].extinct || !matches!(self.realms[k].gov, "tribal" | "clan") || self.rng.random() > PARTITION[self.realms[k].people] { return; }
        let kids: Vec<usize> = self.chars[old].children.iter().copied().filter(|&c| {
            let cc = &self.chars[c];
            cc.alive && c != heir && cc.adult(y) && cc.realm == k && cc.ruled.is_none()
        }).collect();
        if kids.is_empty() { return; }
        self.ensure_titles();
        self.control();
        let seat = self.capital_of(k);
        let seat_duchy = if seat != NONE { self.towns[seat].duchy } else { None };
        let seat_kt = seat_duchy.and_then(|d| self.duchies.get(&d)).map(|d| d.kt);
        let crowns: Vec<DKey> = self.ktitles.iter().filter(|(kt, kd)| kd.ctrl == k && Some(**kt) != seat_kt).map(|(kt, _)| *kt).collect();
        let old_name = self.chars[old].name.clone();
        if !crowns.is_empty() {
            let mut shares: Vec<(usize, Vec<DKey>, DKey)> = vec![];
            for (&c, &kt) in kids.iter().zip(crowns.iter()) {
                let dks: Vec<DKey> = self.duchies.iter().filter(|(_, d)| d.kt == kt && d.ctrl == k).map(|(dk, _)| *dk).collect();
                if !dks.is_empty() { shares.push((c, dks, kt)); }
            }
            if !shares.is_empty() {
                let parts: Vec<String> = shares.iter().map(|(c, _, kt)| format!("{} takes {}", self.chars[*c].name, self.ktitles[kt].name)).collect();
                let text = format!("**The crowns are divided.** By the custom of {}, the kingdoms of {old_name} go to their children: {}.", self.realms[k].name, parts.join(", "));
                self.say(text, false);
                for (c, dks, _) in shares { self.secede(k, &dks, c, 0); }
                return;
            }
        }
        let towns = self.alive_towns_of(k);
        let mut dks: Vec<DKey> = towns.iter().filter_map(|&t| self.towns[t].duchy).filter(|d| Some(*d) != seat_duchy).collect();
        dks.sort();
        dks.dedup();
        dks.sort_by_key(|dk| std::cmp::Reverse(towns.iter().filter(|&&t| self.towns[t].duchy == Some(*dk)).count()));
        let shares: Vec<(usize, DKey)> = kids.into_iter().zip(dks).collect();
        if shares.is_empty() { return; }
        let text = if shares.len() > 1 {
            format!("**The inheritance is divided.** By the custom of {}, the lands of {old_name} are shared among their children: {} each take a land of their own.",
                    self.realms[k].name, shares.iter().map(|s| self.chars[s.0].name.clone()).collect::<Vec<_>>().join(", "))
        } else {
            format!("**The inheritance is divided.** By the custom of {}, the lands of {old_name} are divided: {} takes a land of their own.", self.realms[k].name, self.chars[shares[0].0].name)
        };
        self.say(text, false);
        for (c, dk) in shares { self.secede(k, &[dk], c, 0); }
    }

    pub fn titles_year(&mut self) {
        self.ensure_titles();
        self.control();
        self.grant_duchies();
        self.factions_year();
    }
}
