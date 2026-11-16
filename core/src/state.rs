//! The whole world in one struct, and the acts that set it up: resources, ruins, the first tribes.

use std::collections::HashSet;

use crate::climate::Climate;
use crate::geography::Map;
use crate::grid::Grid;
use crate::people::*;
use crate::rng::Pcg64;
use crate::tables::*;

pub struct State {
    pub year: i64,
    pub rng: Pcg64,
    pub map: Map,
    pub res_grid: Grid<i16>,
    pub chars: Vec<Character>,
    pub houses: Vec<House>,
    pub towns: Vec<Town>,
    pub realms: Vec<Realm>,
    pub ruins: Vec<Ruin>,
    pub used_names: HashSet<String>,
    pub blocked: Grid<bool>,
    pub owner: Grid<i64>,
    pub relation: Vec<Vec<f64>>,     // between realms (grown as realms appear)
    pub stats_defaults: Vec<(i64, usize)>,
    pub stats_forced: Vec<(i64, usize)>,
    pub mined: f64,
    pub bankruptcies: Vec<(i64, usize)>,
    pub log: Vec<(i64, bool, String)>, // (year, minor, text)
    pub climate: Climate,
    pub blight: i64,
    pub cultures: Vec<crate::people::Culture>,
    pub famines: Vec<(i64, usize, f64)>,
    pub migrants: f64,
    pub grain_moved: f64,
}

impl State {
    /// This year's weather and harvest.
    pub fn weather_year(&mut self) {
        let towns: Vec<((usize, usize), f64, String)> =
            self.towns.iter().filter(|t| t.alive).map(|t| (t.pos, t.pop, t.name.clone())).collect();
        let said = self.climate.year(self.year, &mut self.rng, &towns, self.blight > 0);
        for (text, minor) in said { self.say(text, minor); }
    }

    pub fn say(&mut self, text: String, minor: bool) {
        self.log.push((self.year, minor, text));
    }

    /// A fresh name from syllables, never used before in this world (world.Realm.name).
    pub fn name(&mut self, a: &[&str], b: &[&str]) -> String {
        for tries in 0..400 {
            let mut n = rng_pick(&mut self.rng, a).to_string();
            if tries >= 100 {
                n.push_str(&rng_pick(&mut self.rng, a).to_lowercase());
            }
            n.push_str(rng_pick(&mut self.rng, b));
            if self.used_names.insert(n.clone()) {
                return n;
            }
        }
        let n = format!("{}{} {}", rng_pick(&mut self.rng, a), rng_pick(&mut self.rng, b), self.used_names.len());
        self.used_names.insert(n.clone());
        n
    }

    pub fn place_resources(&mut self) {
        let (h, w) = (self.map.h, self.map.w);
        let mut grid = Grid::new(h, w, -1i16);
        for (i, r) in RESOURCES.iter().enumerate() {
            for cell in 0..h * w {
                let t = self.map.terrain.v[cell];
                let land = t != OCEAN;
                let mana = self.map.mana.v[cell];
                let ok = r.terrains.iter().any(|&code| match code {
                    -1 => self.map.coast.v[cell],
                    -2 => land && mana > 0.6,
                    -3 => t == FOREST && mana > 0.4,
                    k => t as i32 == k,
                });
                let draw = self.rng.random();
                if ok && draw < r.chance && grid.v[cell] < 0 {
                    grid.v[cell] = i as i16;
                }
            }
        }
        self.res_grid = grid;
    }

    pub fn place_ruins(&mut self) {
        let land: Vec<usize> = (0..self.map.h * self.map.w)
            .filter(|&i| { let t = self.map.terrain.v[i]; t != OCEAN && t != MOUNTAIN }).collect();
        let big = self.map.lands.iter().filter(|l| l.kind != "isle").count().max(1);
        for _ in 0..12 * big / 2 {
            let cell = land[self.rng.integers(0, land.len() as i64) as usize];
            let name = format!("the ruins of {}", self.name(&["Ny", "Aza", "Quel", "Oro", "Tes", "Vael", "Hy", "Esk"],
                                                            &["thar", "marath", "lorn", "essa", "vane", "dor", "ul", "ith"]));
            let riches = self.rng.integers(3, 9);
            let depth = self.rng.uniform(1.0, 4.0);
            self.ruins.push(Ruin { name, pos: (cell / self.map.w, cell % self.map.w), riches, depth });
        }
    }

    pub fn add_town(&mut self, realm: usize, pos: (usize, usize), pop: f64, capital: bool) -> usize {
        let cu = self.realms[realm].people;
        let [a, b] = TOWN_SYL[cu];
        let name = self.name(a, b);
        let (r, c) = pos;
        let (h, w) = (self.map.h, self.map.w);
        let mut port = false;
        for rr in r.saturating_sub(1)..(r + 2).min(h) {
            for cc in c.saturating_sub(1)..(c + 2).min(w) {
                port |= self.map.coast.at(rr, cc);
            }
        }
        for rr in r.saturating_sub(3)..(r + 4).min(h) {
            for cc in c.saturating_sub(3)..(c + 4).min(w) {
                if ((rr as f64 - r as f64).hypot(cc as f64 - c as f64)) <= 3.5 {
                    self.blocked.set(rr, cc, true);
                }
            }
        }
        let id = self.towns.len();
        self.towns.push(Town { id, name, realm, culture: cu, pos, pop, wealth: 30.0, walls: capital as u32,
                               alive: true, founded: self.year, capital, landmass: self.map.landmass.at(r, c), port,
                               cells: 0, magic: 0.0, soil: 1.0, granary: None, food: 0.0, food_prod: pop, famine: 0.0,
                               hunger_dead: 0.0, lord_mod: 1.0, res_food: 0.0, res_wealth: 0.0, jobs: [0.0; 7],
                               jobless: 0.0, farm_fill: 1.0, labour_short: 0.0, skill: None, arcane: 0.0,
                               labour_value: 0.0, income: 0.0, housing: None, crowded: 0.0, stock: [0.0; 3],
                               cls: [(0.0, 0.0, 0.0); 3], poverty: 0.0, econ_mood: 0.0, bld: [0; 8], relief: 0,
                               coin: None, garrison: 0.0, trade_gain: 0.0, real_income: 0.0, bullion: 0.0 });
        id
    }

    /// A person is born (characters.make_character): genes from the parents and the world, personality, the
    /// eight attributes, prowess, talent, span, first class, skills, legacy and perhaps a gift.
    pub fn make_character(&mut self, realm: usize, born: i64, female: Option<bool>, father: usize, mother: usize,
                          house: usize) -> usize {
        let female = female.unwrap_or_else(|| self.rng.random() < 0.5);
        let people = self.realms.get(realm).map(|k| k.people).unwrap_or(realm);
        let name = first_name(&mut self.rng, people, female);
        let parents: Vec<usize> = [father, mother].into_iter().filter(|&p| p != NONE).collect();
        let rng = &mut self.rng;
        let mut genetic: Vec<usize> = Vec::new();
        let mut pool: Vec<usize> = Vec::new();
        for &p in &parents {
            for &g in &self.chars[p].genetic {
                if !pool.contains(&g) { pool.push(g); }
            }
        }
        for &t in &pool {
            if GENETIC[t].starts_with("half-") || BLOODLINES.contains(&t) { continue; }
            let both = parents.len() == 2 && parents.iter().all(|&p| self.chars[p].has(t));
            if rng.random() < if both { 0.8 } else { 0.45 } { genetic.push(t); }
        }
        for &(t, f) in GENETIC_FREQ.iter() {
            if rng.random() < f && !genetic.contains(&t) { genetic.push(t); }
        }
        for &(a, b) in OPPOSED.iter() {
            if genetic.contains(&a) && genetic.contains(&b) { genetic.retain(|&x| x != b); }
        }
        // power genes: carried unseen, shown sometimes (lineage.inherit_power_genes)
        let mut latent = Vec::new();
        for &g in BLOODLINES.iter() {
            let shown = parents.iter().filter(|&&p| self.chars[p].has(g)).count();
            let carried = parents.iter().filter(|&&p| self.chars[p].latent.contains(&g)).count();
            if shown == 0 && carried == 0 { continue; }
            let p_carry = 1.0 - (1.0f64 - if shown > 0 { 0.75 } else { 0.0 }).powi(shown as i32) * 0.5f64.powi(carried as i32);
            if rng.random() >= p_carry { continue; }
            let p_show = if shown == 2 { 0.45 } else if shown > 0 { 0.3 } else { 0.12 };
            if rng.random() < p_show { genetic.push(g) } else { latent.push(g) }
        }
        if parents.len() == 2 && self.chars[father].people != self.chars[mother].people {
            let other = if self.chars[father].people != people { self.chars[father].people } else { self.chars[mother].people };
            genetic.push(gene(&format!("half-{}", ADJ[other])));
        }
        // personality: four of the axes, sometimes echoing a parent
        let n_axes = PERSONALITY.len() / 2;
        let mut axes: Vec<usize> = (0..n_axes).collect();
        for i in 0..4 {
            let j = i + rng.integers(0, (n_axes - i) as i64) as usize;
            axes.swap(i, j);
        }
        let mut personality = Vec::new();
        for &ax in &axes[..4] {
            let (a, b) = (2 * ax, 2 * ax + 1);
            let inherited: Vec<usize> = parents.iter().flat_map(|&p| self.chars[p].personality.iter().copied())
                .filter(|&t| t == a || t == b).collect();
            if !inherited.is_empty() && rng.random() < 0.35 {
                personality.push(inherited[rng.integers(0, inherited.len() as i64) as usize]);
            } else {
                personality.push(if rng.random() < 0.5 { a } else { b });
            }
        }
        let mut attrs = [0.0f64; 6];
        for (i, a) in attrs.iter_mut().enumerate() {
            let mut base = rng.normal(5.0, 2.0);
            if !parents.is_empty() {
                base = 0.5 * base + 0.5 * parents.iter().map(|&p| self.chars[p].attrs[i]).sum::<f64>() / parents.len() as f64;
            }
            *a = base;
        }
        let mut prowess = PROWESS_BASE[people] + rng.normal(0.0, 3.0);
        let mut talent = TALENT_BASE[people] + rng.normal(0.0, 1.1);
        if !parents.is_empty() {
            let inh: f64 = parents.iter().map(|&p| {
                let c = &self.chars[p];
                c.talent - c.genetic.iter().map(|&g| GENETIC_FX[g][7]).sum::<f64>()
            }).sum::<f64>() / parents.len() as f64;
            talent = 0.5 * talent + 0.5 * inh;
        }
        let mut life = LIFESPAN[people] * rng.uniform(0.85, 1.15);
        if genetic.iter().any(|&g| GENETIC[g].starts_with("half-")) {
            let other = if self.chars[father].people != people { self.chars[father].people } else { self.chars[mother].people };
            life = 0.5 * life + 0.5 * LIFESPAN[other];
        }
        for &g in &genetic {
            let fx = GENETIC_FX[g];
            for i in 0..6 { attrs[i] += fx[i]; }
            prowess += fx[6];
            talent += fx[7];
            life *= fx[8];
        }
        for &t in &personality {
            for i in 0..6 { attrs[i] += PERSONALITY_FX[t][i]; }
        }
        for a in attrs.iter_mut() { *a = a.max(0.0).round(); }
        talent = talent.clamp(0.0, 10.0);
        let cls = pick_class(rng, people, talent, &[]);
        // skills and the eight attributes (skills.init_skills)
        let mut attributes = [0.0f64; 8];
        for (i, a) in attributes.iter_mut().enumerate() {
            *a = rng.integers(30, 46) as f64 + RACE_ATTR[people][i];
        }
        if genetic.contains(&gene("strong")) || genetic.contains(&gene("titanblood")) { attributes[0] += 10.0; }
        if genetic.contains(&gene("genius")) { attributes[1] += 10.0; }
        if genetic.contains(&gene("comely")) || genetic.contains(&gene("silver-tongued")) { attributes[6] += 8.0; }
        let mut skills: Vec<f64> = (0..SKILLS.len()).map(|i| rng.integers(3, 13) as f64 + RACE_SKILL[people][i]).collect();
        for &s in CLASS_SKILLS[cls] { skills[s] += 12.0; }
        // legacy: half of each parent's (lineage.inherit_legacy)
        let mut legacy: Vec<(usize, f64)> = Vec::new();
        for &p in &parents {
            for &(kind, pts) in &self.chars[p].legacy {
                match legacy.iter_mut().find(|x| x.0 == kind) {
                    Some(x) => if pts * 0.5 > x.1 { x.1 = pts * 0.5 },
                    None => legacy.push((kind, pts * 0.5)),
                }
            }
        }
        let religion = if mother != NONE { self.chars[mother].religion } else if father != NONE { self.chars[father].religion } else { people };
        let id = self.chars.len();
        let mut c = Character {
            id, name, people, realm, female, born, house, father, mother, spouse: NONE, children: vec![],
            genetic, latent, personality, attrs, prowess, talent, life, classes: vec![(cls, 1)], level: 1, xp: 0.0,
            hp: 100.0, renown: 0.0, branch: vec![], capstones: vec![], attributes, skills, legacy, legitimacy: None,
            tyranny: 0.0, regnal: None, ruled: None, gift: None, religion, hero: false, undead: false, built: 0,
            disgraced_until: 0, titles: vec![], memories: vec![], alive: true, died: None,
            gold: 0.0,
        };
        // a gift (arcana.maybe_gift)
        if self.rng.random() < GIFT_CHANCE[0] {
            let g = self.rng.integers(0, GIFTS.len() as i64) as usize;
            c.gift = Some(g);
            match GIFTS[g] {
                "Iron Skin" => c.prowess += 8.0,
                "Unfailing Memory" => c.talent = (c.talent + 1.5).min(10.0),
                "Silver Tongue" => c.attrs[0] += 4.0,
                "Born Captain" => c.attrs[1] += 4.0,
                "Seer's Eye" => c.attrs[3] += 4.0,
                "Beastspeech" => if !c.has(gene("beastspeaker")) { c.genetic.push(gene("beastspeaker")) },
                _ => {}
            }
        }
        self.chars.push(c);
        if father != NONE { self.chars[father].children.push(id); }
        if mother != NONE { self.chars[mother].children.push(id); }
        id
    }

    /// Experience, levels and skill points (characters.gain_xp and spend_point).
    pub fn gain_xp(&mut self, cid: usize, xp: f64) {
        let cap = self.chars[cid].level_cap();
        if self.chars[cid].level >= cap || !self.chars[cid].alive { return; }
        self.chars[cid].xp += xp;
        while self.chars[cid].level < cap && self.chars[cid].xp >= xp_needed(self.chars[cid].level) {
            let need = xp_needed(self.chars[cid].level);
            let c = &mut self.chars[cid];
            c.xp -= need;
            c.level += 1;
            c.classes.last_mut().unwrap().1 += 1;
            if c.level % 2 == 0 {
                self.spend_point(cid);
            }
            if self.chars[cid].level == 40 || self.chars[cid].level == 70 {
                // prestige classes come with slice 3 (they need tiers); until then a new base class
                let have: Vec<usize> = self.chars[cid].classes.iter().map(|x| x.0).collect();
                let (p, t) = (self.chars[cid].people, self.chars[cid].talent);
                let cls = pick_class(&mut self.rng, p, t, &have);
                self.chars[cid].classes.push((cls, 0));
            }
        }
    }

    fn spend_point(&mut self, cid: usize) {
        let c = &self.chars[cid];
        let mut options = Vec::new();
        for &(cls, _) in &c.classes {
            if CLASSES[cls].prestige { continue; }
            for b in 0..3 {
                let n = c.branch.iter().find(|x| x.0 == (cls, b)).map(|x| x.1).unwrap_or(0);
                if n < 4 { options.push((cls, b)); }
            }
        }
        if options.is_empty() {
            let caps: Vec<usize> = c.classes.iter().map(|x| x.0).filter(|&k| !CLASSES[k].prestige && !c.capstones.contains(&k)).collect();
            if let Some(&k) = caps.first() { self.chars[cid].capstones.push(k); }
            return;
        }
        let active = c.classes.last().unwrap().0;
        let act: Vec<(usize, usize)> = options.iter().copied().filter(|o| o.0 == active).collect();
        let act = if act.is_empty() { options } else { act };
        let choice = act[self.rng.integers(0, act.len() as i64) as usize];
        let c = &mut self.chars[cid];
        match c.branch.iter_mut().find(|x| x.0 == choice) {
            Some(x) => x.1 += 1,
            None => c.branch.push((choice, 1)),
        }
        let cls = choice.0;
        let full = (0..3).all(|b| c.branch.iter().find(|x| x.0 == (cls, b)).map(|x| x.1).unwrap_or(0) >= 4);
        if full && !c.capstones.contains(&cls) { c.capstones.push(cls); }
    }

    pub fn new_house(&mut self, founder: usize, name: Option<String>) -> usize {
        let f = self.chars[founder].people;
        let name = name.unwrap_or_else(|| { let [a, b] = TOWN_SYL[f]; self.name(a, b) });
        let track = match self.chars[founder].legacy.iter().max_by(|a, b| a.1.partial_cmp(&b.1).unwrap()) {
            Some(&(k, _)) => LEGACY_KINDS[k].to_string(),
            None => rng_pick(&mut self.rng, &["warrior", "ruler", "arcane", "guardian"]).to_string(),
        };
        let id = self.houses.len();
        let legacy = self.chars[founder].legacy.iter().map(|x| x.1).sum::<f64>() * 0.5;
        let origin = format!("founded in year {} by {}", self.year, self.chars[founder].name);
        self.houses.push(House { id, name: HOUSE_STYLE[f].replace("{}", &name), people: f, founder,
                                 founded: self.year, prestige: 0.0, rulers: 0, track, legacy, origin });
        self.chars[founder].house = id;
        id
    }

    /// dynasty.crown for a realm's first ruler; successions come with slice 4.
    pub fn crown(&mut self, k: usize, cid: usize, reason: &str) {
        if self.chars[cid].house == NONE {
            let n = self.chars[cid].name.clone();
            self.new_house(cid, Some(n));
        }
        let name = self.chars[cid].name.clone();
        let realm = &mut self.realms[k];
        realm.ruler = cid;
        realm.house = self.chars[cid].house;
        let n = match realm.regnal.iter_mut().find(|x| x.0 == name) {
            Some(x) => { x.1 += 1; x.1 }
            None => { realm.regnal.push((name.clone(), 1)); 1 }
        };
        realm.reigns.push((cid, self.year, None));
        let c = &mut self.chars[cid];
        c.regnal = Some(roman(n));
        c.ruled = Some((k, self.year, None));
        c.hero = false;
        let h = c.house;
        self.houses[h].rulers += 1;
        // legitimacy at the crowning (governance.on_crown)
        let base = match reason { "the first of the line" => 75.0, "by right of birth" => 70.0, "as eldest" => 68.0, _ => 50.0 };
        let renown = self.chars[cid].renown;
        let leg = base + (renown / 60.0).min(15.0) + self.houses[h].prestige / 200.0;
        self.chars[cid].legitimacy = Some(leg.clamp(0.0, 100.0));
        let c = &self.chars[cid];
        let title = if c.female { RULER_TITLE_F[c.people] } else { RULER_TITLE_M[c.people] };
        let text = format!("{} {} of {} is crowned {} of {}, {} (legitimacy {:.0}).", c.name, c.regnal.as_ref().unwrap(),
                           self.houses[h].name, title, self.realms[k].name, reason, c.legitimacy.unwrap());
        self.say(text, false);
    }

    /// The peoples begin on three continents (two share the greatest) as scattered tribes (world.place_capitals).
    pub fn place_capitals(&mut self, per_people: usize) {
        let (h, w) = (self.map.h, self.map.w);
        let mut conts: Vec<(usize, usize)> = self.map.lands.iter().filter(|l| l.kind == "continent").map(|l| (l.id, l.size)).collect();
        conts.sort_by_key(|x| std::cmp::Reverse(x.1));
        if conts.is_empty() {
            let mut all: Vec<(usize, usize)> = self.map.lands.iter().map(|l| (l.id, l.size)).collect();
            all.sort_by_key(|x| std::cmp::Reverse(x.1));
            conts = all;
        }
        let pick = |i: usize| conts[i.min(conts.len() - 1)].0;
        let homes = [pick(0), pick(0), pick(1), pick(2)];
        let mut order = [0usize, 1, 2, 3];
        for i in (1..4).rev() {
            let j = self.rng.integers(0, i as i64 + 1) as usize;
            order.swap(i, j);
        }
        let mut placed: Vec<(usize, usize)> = Vec::new();
        let mut seats: Vec<(usize, (usize, usize))> = Vec::new();
        for (f, lid) in order.into_iter().zip(homes) {
            // food within three cells (box sum), on the home continent, away from the edge and from the other people
            let food = |r: usize, c: usize, t: &Grid<i8>| FOOD[f][t.at(r, c) as usize];
            let mut base = Grid::new(h, w, -1e9f64);
            let size = self.map.lands[lid].size as f64;
            for r in 6..h.saturating_sub(6) {
                for c in 6..w.saturating_sub(6) {
                    if self.map.terrain.at(r, c) == OCEAN || self.map.landmass.at(r, c) != lid as i32 { continue; }
                    if placed.iter().any(|p| ((r as f64 - p.0 as f64).hypot(c as f64 - p.1 as f64)) < size.sqrt() * 0.35) { continue; }
                    let mut s = 0.0;
                    for rr in r - 3..=r + 3 { for cc in c - 3..=c + 3 { s += food(rr, cc, &self.map.terrain); } }
                    base.set(r, c, s);
                }
            }
            let mut mine: Vec<(usize, usize)> = Vec::new();
            for n in 0..per_people {
                let mut best = (-1e8f64, 0usize);
                for i in 0..h * w {
                    let v = base.v[i] + self.rng.random() * 3.0;
                    if v > best.0 {
                        let (r, c) = (i / w, i % w);
                        if mine.iter().any(|p| ((r as f64 - p.0 as f64).hypot(c as f64 - p.1 as f64)) < 10.0) { continue; }
                        best = (v, i);
                    }
                }
                if best.0 <= -1e8 { break; }
                let (r, c) = (best.1 / w, best.1 % w);
                mine.push((r, c));
                if n == 0 {
                    for i in 0..h * w {
                        if ((i / w) as f64 - r as f64).hypot((i % w) as f64 - c as f64) > size.sqrt() * 0.45 { base.v[i] = -1e9; }
                    }
                }
            }
            placed.extend(&mine);
            seats.extend(mine.into_iter().map(|p| (f, p)));
        }
        self.found_kingdoms(&seats);
    }

    /// Every people begins as small tribes: a chief, their kin, a seat, nothing yet above them.
    fn found_kingdoms(&mut self, seats: &[(usize, (usize, usize))]) {
        for (pid, &(f, pos)) in seats.iter().enumerate() {
            let first = seats.iter().position(|s| s.0 == f).unwrap() == pid;
            let color = if first { COLORS[f] } else {
                let mut c = COLORS[f];
                for v in c.iter_mut() { *v = (*v as f64 * self.rng.uniform(0.6, 1.3)).clamp(30.0, 255.0) as u8; }
                c
            };
            self.realms.push(Realm { id: pid, people: f, name: String::new(), short: String::new(), rank: "tribe",
                gov: "tribal", law: if f == 3 { "strongest" } else { "elective" }, color, ruler: NONE, house: NONE,
                regnal: vec![], reigns: vec![], place: String::new(), extinct: false,
                growth_mult: 1.0, tax_now: 0.06, gold: 50.0, crown_law: 1, tax_relief: 0, council: [NONE; 6],
                council_for: NONE, econ_distress: 0.0, corruption: 0.0, collect: 1.0, tax_mult: 1.0, levy: 0.0,
                income: 0.0, real_gdp: 0.0, credit: None, defaults: 0, rate: 0.0, money: 0.0, trade_val: 0.0,
                trade_net: 0.0, partners: 0, trade_protectionist: false, last_forced: -99, last_default: -99,
                mint_skill: 0.0, sector_boost: [0.0; 7], econ_prev_gdp: None, goods: vec![] });
            let pop = self.rng.integers(120, 260) as f64;
            let cap = self.add_town(pid, pos, pop, true);
            let place = self.towns[cap].name.clone();
            let k = &mut self.realms[pid];
            k.name = ["the {} Tribe", "the Kin of {}", "the {} Clans", "the {} Warband"][f].replace("{}", &place);
            k.short = format!("{} of {}", PEOPLES[f], place);
            k.place = place;
            let adult = ADULT[f] as i64;
            let age = adult + self.rng.integers(5, (adult).max(6));
            let ruler = self.make_character(pid, -age, None, NONE, NONE, NONE);
            let house = self.new_house(ruler, None);
            self.houses[house].prestige = 30.0;
            self.gain_xp(ruler, 100.0 + 20.0 * age.min(60) as f64);
            let sp_age = age + self.rng.integers(-5, 6);
            let rf = self.chars[ruler].female;
            let sp = self.make_character(pid, -sp_age, Some(!rf), NONE, NONE, NONE);
            self.chars[ruler].spouse = sp;
            self.chars[sp].spouse = ruler;
            let (dad, mum) = if rf { (sp, ruler) } else { (ruler, sp) };
            for _ in 0..self.rng.integers(1, 3) {
                let kid_age = self.rng.integers(0, (age - adult).max(1));
                let kid = self.make_character(pid, -kid_age, None, dad, mum, house);
                self.gain_xp(kid, 20.0 * kid_age as f64);
            }
            self.houses[house].track = "ruler".into();
            self.houses[house].origin = format!("chiefs of {}, from year 0", self.realms[pid].name);
            self.crown(pid, ruler, "the first of the line");
            let n_age = adult + self.rng.integers(0, 15);
            let n = self.make_character(pid, -n_age, None, NONE, NONE, NONE);
            self.new_house(n, None);
            self.gain_xp(n, 300.0);
        }
        // power bloodlines: two shown, two carried unseen
        let founders: Vec<usize> = self.houses.iter().map(|h| h.founder).collect();
        let mut picks: Vec<usize> = (0..founders.len()).collect();
        for i in (1..picks.len()).rev() {
            let j = self.rng.integers(0, i as i64 + 1) as usize;
            picks.swap(i, j);
        }
        for (i, (trait_, shown)) in [("dragonblood", true), ("starblood", true), ("titanblood", false), ("elderblood", false)].into_iter().enumerate() {
            let cid = founders[picks[i % founders.len()]];
            let g = gene(trait_);
            if shown {
                let fx = GENETIC_FX[g];
                let c = &mut self.chars[cid];
                if !c.has(g) { c.genetic.push(g); }
                for k in 0..6 { c.attrs[k] += fx[k]; }
                c.prowess += fx[6];
                c.talent = (c.talent + fx[7]).min(10.0);
                c.life *= fx[8];
                let hn = self.houses[self.chars[cid].house].name.clone();
                self.say(format!("It is whispered that {} runs in {}.", trait_.replace("blood", " blood"), hn), false);
            } else if !self.chars[cid].latent.contains(&g) {
                self.chars[cid].latent.push(g);
            }
        }
    }
}

pub fn rng_pick<'a>(rng: &mut Pcg64, xs: &[&'a str]) -> &'a str {
    xs[rng.integers(0, xs.len() as i64) as usize]
}
