//! Heroes (heroes.py): each people's guild learns a doctrine of deeds (Q-learning over nine actions) and of skills
//! (a bandit over skill-tree branches) that outlives every hero; heroes rise from great lines and big towns, train,
//! hunt, delve, fight in wars, guard, study, serve, hunt bounties, forge, ride together against calamities, and
//! the greatest casters work World Rites.

use std::collections::HashMap;

use crate::people::*;
use crate::power::skill_ix;
use crate::state::State;
use crate::tables::*;

const REST: usize = 0; const TRAIN: usize = 1; const HUNT: usize = 2; const DELVE: usize = 3; const WAR: usize = 4;
const GUARD: usize = 5; const STUDY: usize = 6; const SERVE: usize = 7; const BOUNTY: usize = 8;
const LEVEL_BANDS: [u32; 6] = [10, 20, 35, 50, 70, 90];
const HERO_CAP: usize = 8;

pub type Obs = (u8, u8, u8, u8, u8, u8);

pub struct HeroGuild {
    pub people: usize,
    pub q: HashMap<Obs, [f64; 9]>,
    pub decisions: u64,
    pub branch_val: HashMap<(usize, usize), (f64, u32)>,
    pub heroes_total: u32,
    pub deaths_total: u32,
    pub acts: [u32; 9],
    pub hunts: u32,
    pub hunt_wins: u32,
    pub slain: u32,
}

impl HeroGuild {
    pub fn new(people: usize) -> Self {
        HeroGuild { people, q: HashMap::new(), decisions: 0, branch_val: HashMap::new(), heroes_total: 0, deaths_total: 0, acts: [0; 9], hunts: 0, hunt_wins: 0, slain: 0 }
    }
    fn eps(&self) -> f64 { (0.35 * (-(self.decisions as f64) / 4000.0).exp()).max(0.05) }
}

impl State {
    fn obs_key(o: Obs) -> usize { ((((o.0 as usize * 3 + o.1 as usize) * 4 + o.2 as usize) * 2 + o.3 as usize) * 2 + o.4 as usize) * 2 + o.5 as usize }
    fn key_obs(k: usize) -> Obs {
        let (c, k) = ((k % 2) as u8, k / 2); let (r, k) = ((k % 2) as u8, k / 2); let (w, k) = ((k % 2) as u8, k / 2);
        let (t, k) = ((k % 4) as u8, k / 4); let (h, l) = ((k % 3) as u8, (k / 3) as u8); (l, h, t, w, r, c)
    }

    fn guild_act(&mut self, p: usize, s: Obs) -> usize {
        self.hero_guilds[p].decisions += 1;
        if self.rng.random() < self.hero_guilds[p].eps() { return self.rng.integers(0, 9) as usize; }
        let row = *self.hero_guilds[p].q.entry(s).or_insert([0.0; 9]);
        let best = row.iter().cloned().fold(f64::MIN, f64::max);
        let idx: Vec<usize> = (0..9).filter(|&i| row[i] == best).collect();
        idx[self.rng.integers(0, idx.len() as i64) as usize]
    }

    fn guild_learn(&mut self, p: usize, s: Obs, a: usize, r: f64, s2: Option<Obs>) {
        let g = &mut self.hero_guilds[p];
        let target = match s2 { None => r, Some(s2) => r + 0.95 * g.q.entry(s2).or_insert([0.0; 9]).iter().cloned().fold(f64::MIN, f64::max) };
        let row = g.q.entry(s).or_insert([0.0; 9]);
        row[a] += 0.15 * (target - row[a]);
    }

    /// The guild's doctrine picks a hero's next skill branch (Guild.pick_branch).
    pub fn pick_branch(&mut self, p: usize, options: &[(usize, usize)]) -> (usize, usize) {
        if self.rng.random() < 0.2 { return options[self.rng.integers(0, options.len() as i64) as usize]; }
        let vals: Vec<f64> = options.iter().map(|o| self.hero_guilds[p].branch_val.get(o).map(|v| v.0).unwrap_or(1.0)).collect();
        let best = vals.iter().cloned().fold(f64::MIN, f64::max);
        let idx: Vec<usize> = (0..options.len()).filter(|&i| vals[i] == best).collect();
        options[idx[self.rng.integers(0, idx.len() as i64) as usize]]
    }

    fn guild_credit(&mut self, h: usize, reward: f64) {
        let Some(&(key, _)) = self.chars[h].branch.iter().max_by_key(|b| b.1) else { return };
        let p = self.chars[h].people;
        let e = self.hero_guilds[p].branch_val.entry(key).or_insert((0.0, 0));
        e.1 += 1;
        e.0 += (reward - e.0) / e.1 as f64;
    }

    // ------------------------------------------------------------ deeds, renown, wounds
    pub fn deed(&mut self, h: usize, k: &'static str) {
        let d = &mut self.chars[h].deeds;
        match d.iter_mut().find(|x| x.0 == k) { Some(x) => x.1 += 1.0, None => d.push((k, 1.0)) }
        self.chars[h].last_deed = Some(k);
    }

    pub fn renown(&mut self, h: usize, amount: f64) {
        self.chars[h].renown += amount;
        if let Some(d) = self.chars[h].last_deed { if let Some(&(_, kind)) = DEED_LEGACY.iter().find(|x| x.0 == d) { self.gain_legacy(h, kind, amount); } }
        let house = self.chars[h].house;
        if house != NONE { self.houses[house].prestige += amount * 0.5; }
        let c = &self.chars[h];
        if c.epithet.is_none() && c.renown >= 80.0 && c.ruled.is_none() {
            let weight = |d: &str| match d { "calamity" => 1000.0, "dragon" => 150.0, "rite" => 50.0, "demon" => 60.0, "wraith" => 30.0, "troll" => 10.0, "study" => 12.0, "guard" => 5.0, "delve" => 3.0, "war" => 1.5, _ => 1.0 };
            let best = c.deeds.iter().max_by(|a, b| (a.1 * weight(a.0)).partial_cmp(&(b.1 * weight(b.0))).unwrap()).map(|x| x.0).unwrap_or("guard");
            let ep = if best == "guard" {
                if c.home != NONE { format!("the Shield of {}", self.towns[c.home].name) } else { "the Shield".into() }
            } else { EPITHET_HERO.iter().find(|x| x.0 == best).map(|x| x.1).unwrap_or("the Bold").to_string() };
            let (name, people) = (c.name.clone(), c.people);
            self.chars[h].epithet = Some(ep.clone());
            self.say(format!("{name} of the {} is called {name} {ep} now.", PEOPLES[people]), false);
        }
    }

    pub fn hurt(&mut self, h: usize, dmg: f64, cause: String, monster: usize, by: usize) {
        if !self.chars[h].alive { return; }
        let hm = self.hp_mult(&self.chars[h]).max(0.3);
        self.chars[h].hp -= dmg / hm;
        if self.chars[h].hp <= 0.0 && !self.chars[h].latent.is_empty() && self.rng.random() < 0.2 {
            self.awaken(h, "at the edge of death");
            self.chars[h].hp = 20.0;
        }
        if self.chars[h].hp <= 0.0 {
            if monster != NONE { self.kill_by_monster(h, cause, monster); } else { self.kill_by(h, cause, false, by); }
        }
    }

    // ------------------------------------------------------------ heroes
    pub fn start_hero(&mut self, c: usize, why: Option<String>) {
        self.chars[c].hero = true;
        if self.chars[c].home == NONE || !self.towns[self.chars[c].home].alive {
            let cap = self.capital_of(self.chars[c].realm);
            self.chars[c].home = cap;
        }
        let p = self.chars[c].people;
        self.hero_guilds[p].heroes_total += 1;
        let k = self.chars[c].realm;
        let mut tr = std::mem::take(&mut self.realms[k].treasury);
        tr.sort_by_key(|&i| std::cmp::Reverse(self.items[i].rarity));
        let mut taken = 0;
        while !tr.is_empty() && self.items[tr[0]].rarity >= 2 && taken < 2 {
            let it = tr.remove(0);
            let prev = self.items[it].bearers.last().map(|&b| self.chars[b].name.clone());
            self.give_item(c, it, true);
            taken += 1;
            if self.items[it].rarity >= 4 {
                let t = format!("{} takes up {}{}.", self.who(c), self.items[it].label(), prev.map(|p| format!(", once borne by {p}")).unwrap_or_default());
                self.say(t, false);
            }
        }
        self.realms[k].treasury.extend(tr);
        if let Some(w) = why { let minor = w.contains("calling"); let t = format!("{} {w}.", self.who(c)); self.say(t, minor); }
    }

    pub fn spawn_heroes(&mut self) {
        let y = self.year;
        for f in 0..self.realms.len() {
            if self.realms[f].extinct { continue; }
            let mut n = self.living.iter().map(|&i| &self.chars[i]).filter(|h| h.alive && h.hero && h.realm == f).count();
            let towns = self.alive_towns_of(f);
            let cap = (HERO_CAP + 16).min(1 + towns.len() / 25);
            if n >= cap { continue; }
            let adult = ADULT[self.realms[f].people] as i64;
            let heirs: Vec<usize> = self.living.iter().map(|&i| &self.chars[i]).filter(|c| c.alive && c.realm == f && !c.hero && c.ruled.is_none() && c.adult(y) && c.age(y) < adult + 25
                && c.father != NONE && (self.chars[c.father].hero || self.chars[c.father].renown > 100.0)).map(|c| c.id).collect();
            for c in heirs {
                if n >= cap { break; }
                if self.rng.random() < 0.15 {
                    let why = format!("takes up the calling of {}", self.chars[self.chars[c].father].name);
                    self.start_hero(c, Some(why));
                    n += 1;
                }
            }
            for s in towns {
                if n >= cap { break; }
                if self.towns[s].pop > 250.0 && self.rng.random() < 0.004 * self.towns[s].pop / 100.0 {
                    let age = adult + self.rng.integers(0, 6);
                    let c = self.make_character(f, y - age, None, NONE, NONE, NONE);
                    self.chars[c].home = s;
                    let xp = self.rng.uniform(150.0, 900.0);
                    self.gain_xp(c, xp);
                    self.start_hero(c, None);
                    n += 1;
                }
            }
        }
    }

    fn hero_home(&mut self, h: usize) -> usize {
        let s = self.chars[h].home;
        if s != NONE && self.towns[s].alive && self.towns[s].realm == self.chars[h].realm { return s; }
        let towns = self.alive_towns_of(self.chars[h].realm);
        let Some(&best) = towns.iter().max_by(|&&a, &&b| self.towns[a].pop.partial_cmp(&self.towns[b].pop).unwrap()) else { return NONE };
        self.chars[h].home = best;
        best
    }

    fn nearest_monster(&self, pos: (usize, usize), reach: f64) -> usize {
        let lid = self.map.landmass.at(pos.0, pos.1);
        let mut best = NONE;
        let mut bd = 0.0;
        for (i, m) in self.monsters.iter().enumerate() {
            if !m.alive || m.sea || (m.landmass != lid && !m.world) || m.kind == "dragon" { continue; }
            let lim = if m.kind == "calamity" { 45.0 } else { reach };
            let d = crate::monsters::mdist(m.pos, pos);
            if d > lim { continue; }
            let better = best == NONE || {
                let (mc, bc) = (m.kind == "calamity", self.monsters[best].kind == "calamity");
                mc > bc || (mc == bc && d < bd)
            };
            if better { best = i; bd = d; }
        }
        best
    }

    fn nearest_ruin(&self, pos: (usize, usize), reach: f64) -> usize {
        let lid = self.map.landmass.at(pos.0, pos.1);
        let mut best = NONE;
        let mut bd = reach + 1.0;
        for (i, r) in self.ruins.iter().enumerate() {
            if self.map.landmass.at(r.pos.0, r.pos.1) != lid || !(!r.loot.is_empty() || r.conquered.is_none()) { continue; }
            let d = crate::monsters::mdist(r.pos, pos);
            if d < bd { best = i; bd = d; }
        }
        best
    }

    fn hero_obs(&self, h: usize, home: usize) -> Obs {
        let c = &self.chars[h];
        let lb = LEVEL_BANDS.iter().filter(|&&t| c.level >= t).count() as u8;
        let hb = ((c.hp / 34.0) as i64).clamp(0, 2) as u8;
        let m = self.nearest_monster(self.towns[home].pos, 25.0);
        let threat = if m == NONE { 0 } else { let ratio = self.monsters[m].power * 0.6 / self.cr(c, true).max(1.0); if ratio < 0.6 { 1 } else if ratio < 1.6 { 2 } else { 3 } };
        let war = self.war[c.realm].iter().any(|&w| w) as u8;
        let ruin = (self.nearest_ruin(self.towns[home].pos, 20.0) != NONE) as u8;
        let cal = self.monsters.iter().any(|m| m.alive && m.kind == "calamity") as u8;
        (lb, hb, threat, war, ruin, cal)
    }

    /// The heroes' year (heroes.heroes_act); returns what each realm's heroes add to its war.
    pub fn heroes_act(&mut self) -> Vec<f64> {
        let mut war_power = vec![0.0; self.realms.len()];
        let mut hunters: Vec<(usize, usize)> = vec![];
        self.acted.clear();
        for t in self.towns.iter_mut() { t.guard = 0.0; t.raided = None; t.raided_by = NONE; }
        let heroes: Vec<usize> = self.living.iter().map(|&i| &self.chars[i]).filter(|c| c.alive && c.hero).map(|c| c.id).collect();
        for h in heroes {
            if !self.chars[h].alive { continue; }
            let mut home = self.hero_home(h);
            if home == NONE {
                let p = self.chars[h].people;
                let kin = (0..self.realms.len()).filter(|&k| !self.realms[k].extinct && self.realms[k].people == p && !self.alive_towns_of(k).is_empty()).max_by_key(|&k| self.alive_towns_of(k).len());
                if let Some(k) = kin { self.chars[h].realm = k; self.chars[h].home = NONE; home = self.hero_home(h); }
            }
            if home == NONE {
                let why = format!("faded from the world with the fall of {}", self.realms[self.chars[h].realm].name);
                self.kill(h, why);
                continue;
            }
            self.chars[h].hp = (self.chars[h].hp + 15.0).min(100.0);
            self.try_rite(h);
            let p = self.chars[h].people;
            let s = self.hero_obs(h, home);
            let a = self.guild_act(p, s);
            self.hero_guilds[p].acts[a] += 1;
            self.chars[h].pending = Some((Self::obs_key(s), a, self.chars[h].renown));
            self.acted.push(h);
            let l = self.chars[h].level as f64;
            let k = self.chars[h].realm;
            match a {
                REST => { let pot = self.realms[k].potions; self.chars[h].hp = (self.chars[h].hp + 40.0 + pot.min(30.0)).min(100.0); self.realms[k].potions = (pot - 1.0).max(0.0); }
                BOUNTY => { let got = self.hunt_bounty(h); if got > 0.0 { self.deed(h, "guard"); self.renown(h, got); } }
                TRAIN => { self.gain_xp(h, 80.0 + 5.0 * l); let cls = self.chars[h].classes[0].0; for &s in CLASS_SKILLS[cls] { self.use_skill(h, s, 1.0); } self.forge(h); }
                STUDY => { self.gain_xp(h, 60.0 + 4.0 * l); self.fight_skills(h, 0.3); self.use_skill(h, skill_ix("Enchanting"), 0.5); self.use_skill(h, skill_ix("Alchemy"), 0.5); self.study(h); self.forge(h); }
                SERVE => {
                    let on = self.realms[k].council.contains(&h);
                    self.gain_xp(h, 30.0);
                    self.use_skill(h, skill_ix("Speech"), 1.2);
                    self.chars[h].gold += 20.0 + if on { 40.0 } else { 0.0 };
                    self.renown(h, if on { 5.0 } else { 2.0 });
                }
                HUNT => {
                    let m = self.nearest_monster(self.towns[home].pos, 25.0);
                    if m == NONE { continue; }
                    if self.monsters[m].kind == "calamity" { hunters.push((h, m)); continue; }
                    self.hunt(h, m, home);
                }
                DELVE => { let r = self.nearest_ruin(self.towns[home].pos, 20.0); if r != NONE { self.delve(h, r); } }
                WAR => { if self.war[k].iter().any(|&w| w) { war_power[k] += self.cr(&self.chars[h], true) * self.mods(&self.chars[h])[3]; self.fight_skills(h, 1.0); self.chars[h].gold += 15.0; } }
                GUARD => { let g = self.cr(&self.chars[h], true) * self.mods(&self.chars[h])[4]; self.towns[home].guard += g; self.use_skill(h, skill_ix("Block"), 0.8); self.gain_xp(h, 20.0); }
                _ => {}
            }
        }
        self.great_hunt(hunters);
        war_power
    }

    pub fn hunt(&mut self, h: usize, m: usize, home: usize) {
        let p = self.chars[h].people;
        self.hero_guilds[p].hunts += 1;
        self.fight_skills(h, 1.2);
        let cr = self.cr(&self.chars[h], true) * self.mods(&self.chars[h])[5];
        let mp = self.monsters[m].power;
        let mut pr = cr / (cr + 0.6 * mp);
        if cr < 0.12 * mp { pr *= 0.25; }
        let base = self.monsters[m].base;
        let mname = self.monsters[m].name.clone();
        if self.rng.random() < pr {
            self.hero_guilds[p].hunt_wins += 1;
            self.hit(m, cr * 1.5);
            self.gain_xp(h, base * 0.8);
            let d = self.rng.uniform(5.0, 30.0);
            self.hurt(h, d, format!("died of wounds from {mname}"), m, NONE);
        } else {
            self.hit(m, cr * 0.4);
            self.gain_xp(h, base * 0.2);
            let mp = self.monsters[m].power;
            let d = (self.rng.uniform(30.0, 90.0) * 2.0 * mp / (mp + cr)).min(150.0);
            self.hurt(h, d, format!("was slain by {mname}"), m, NONE);
        }
        if self.monsters[m].power <= 0.0 && self.monsters[m].alive {
            self.monsters[m].alive = false;
            self.hero_guilds[p].slain += 1;
            let kind = self.monsters[m].kind;
            if self.chars[h].alive { self.deed(h, kind); }
            let slayer = if self.chars[h].alive { h } else { NONE };
            self.on_monster_slain(m, slayer);
            let hoard = self.monsters[m].hoard;
            self.towns[home].wealth += hoard * 0.7;
            self.chars[h].gold += hoard * 0.3 + base * 0.1;
            let ren = MONSTER.iter().find(|x| x.0 == kind).map(|x| x.7).unwrap_or(10.0);
            if self.chars[h].alive { self.renown(h, ren); }
            let drop = if kind == "dragon" { let r = self.rng.integers(3, 6) as usize; Some(self.make_item(r, format!("the hoard of {mname}"))) }
                else if kind == "demon" && self.rng.random() < 0.6 { let r = self.rng.integers(2, 5) as usize; Some(self.make_item(r, format!("the ashes of {mname}"))) } else { None };
            if kind != "troll" || self.rng.random() < 0.1 {
                let g = if hoard > 50.0 { format!(" and carries home {} gold", crate::economy::thousands(hoard)) } else { String::new() };
                let t = format!("{} of {} slays {mname}{g}.", self.who(h), self.towns[home].name);
                self.say(t, false);
            }
            if let Some(d) = drop { if self.chars[h].alive { self.give_item(h, d, false); } }
        }
    }

    fn study(&mut self, h: usize) {
        let c = &self.chars[h];
        if self.school(c).is_none() { return; }
        let eff: f64 = c.classes.iter().map(|&(k, lv)| lv as f64 * CLASSES[k].magic.min(1.0)).sum();
        if eff / 9.0 > c.talent.floor() && c.talent < 10.0 && self.rng.random() < 0.03 * (1.0 + c.attr(4, self.year) / 20.0) {
            let before = self.tier(&self.chars[h]);
            self.chars[h].talent = (self.chars[h].talent.floor() + 1.0).min(10.0);
            self.deed(h, "study");
            let after = self.tier(&self.chars[h]);
            if after > before {
                self.on_new_tier(h, after);
                if after >= 5 {
                    self.renown(h, 10.0 * after as f64);
                    let t = format!("{} breaks through to the {} tier after long study.", self.who(h), TIER_WORDS[after as usize]);
                    self.say(t, false);
                }
            }
        }
    }

    pub fn on_new_tier(&mut self, h: usize, t: i64) {
        let p = self.chars[h].people;
        self.chars[h].peak_cr = self.chars[h].peak_cr.max(0.0);
        if t > self.people_tier[p] { self.people_tier[p] = t; }
        if t > self.world_tier {
            self.world_tier = t;
            let s = self.school(&self.chars[h]).unwrap_or(0);
            let spell = SPELLS[s][t as usize];
            self.tier_events.push((self.year, h, t, spell));
            let tx = format!("**{} casts {spell}**, the first {}-tier spell in the history of the world.", self.who(h), TIER_WORDS[t as usize]);
            self.say(tx, false);
        }
    }

    fn forge(&mut self, h: usize) {
        let rs = CLASSES.iter().position(|c| c.name == "Runesmith").unwrap();
        let n = self.chars[h].branch.iter().find(|b| b.0 == (rs, 0)).map(|b| b.1).unwrap_or(0) as f64;
        let lord = self.chars[h].classes.iter().any(|c| CLASSES[c.0].name == "Rune-Lord");
        if n == 0.0 && !lord { return; }
        if self.rng.random() < 0.04 * n + if lord { 0.1 } else { 0.0 } {
            let hi = 3 + (n as i64) / 2 + lord as i64;
            let rar = (self.rng.integers(1, hi.max(2)) as usize).min(5);
            let it = self.make_item(rar, format!("the forge of {}", self.chars[h].name));
            if rar >= 4 { let t = format!("{} forges {}.", self.who(h), self.items[it].label()); self.say(t, false); }
            self.give_item(h, it, true);
        }
    }

    fn great_hunt(&mut self, hunters: Vec<(usize, usize)>) {
        let mut by: Vec<(usize, Vec<usize>)> = vec![];
        for (h, m) in hunters {
            if !self.chars[h].alive || !self.monsters[m].alive { continue; }
            match by.iter_mut().find(|x| x.0 == m) { Some(x) => x.1.push(h), None => by.push((m, vec![h])) }
        }
        for (m, party) in by {
            let total: f64 = party.iter().map(|&h| self.cr(&self.chars[h], true) * self.mods(&self.chars[h])[5]).sum();
            let mp = self.monsters[m].power;
            let mut p = total / (total + 0.6 * mp);
            if total < 0.12 * mp { p *= 0.25; }
            for &h in &party { let pe = self.chars[h].people; self.hero_guilds[pe].hunts += 1; }
            let mut names = party.iter().take(6).map(|&h| self.who(h)).collect::<Vec<_>>().join(", ");
            if party.len() > 6 { names += &format!(" and {} more", party.len() - 6); }
            if self.rng.random() < p {
                self.hit(m, total * 1.2);
                for &h in &party { let pe = self.chars[h].people; self.hero_guilds[pe].hunt_wins += 1; }
            } else { self.hit(m, total * 0.3); }
            let (base, mname) = (self.monsters[m].base, self.monsters[m].name.clone());
            for &h in &party {
                self.gain_xp(h, base * 0.05);
                let mp = self.monsters[m].power.max(0.0);
                let d = self.rng.uniform(20.0, 80.0) * 2.0 * mp / (mp + total);
                self.hurt(h, d, format!("fell in the great hunt against {mname}"), m, NONE);
            }
            if self.monsters[m].power <= 0.0 && self.monsters[m].alive {
                self.monsters[m].alive = false;
                self.slain += 1;
                let living: Vec<usize> = party.iter().copied().filter(|&h| self.chars[h].alive).collect();
                for &h in &living { self.deed(h, "calamity"); self.renown(h, 300.0); }
                let born = self.monsters[m].born;
                self.say(format!("**{mname} is slain** after {} years, by {names}. The peoples will sing of it for an age.", self.year - born), false);
                if let Some(&top) = living.iter().max_by(|&&a, &&b| self.cr(&self.chars[a], true).partial_cmp(&self.cr(&self.chars[b], true)).unwrap()) {
                    self.monsters[m].slain_by = top;
                    self.renown(top, 700.0);
                    let it = self.make_item(6, format!("the corpse of {mname}"));
                    self.give_item(top, it, false);
                }
                self.calamity_log.push((mname, born, self.year, format!("slain by heroes: {names}")));
            } else if party.len() >= 2 {
                self.say(format!("A war-band of heroes ({names}) rides against {mname}; it still stands."), true);
            }
        }
    }

    fn try_rite(&mut self, h: usize) {
        let y = self.year;
        let c = &self.chars[h];
        let t = self.tier(c);
        let can = t >= 9 || (t >= 8 && c.items.iter().any(|&i| self.items[i].special == Some("rite")));
        if y < c.rite_ready || !can || self.rng.random() > 0.25 { return; }
        let s = self.school(c).unwrap();
        let k = c.realm;
        let rite = WORLD_RITES[s];
        let at_war = self.war[k].iter().any(|&w| w);
        let cal = (0..self.monsters.len()).find(|&m| self.monsters[m].alive && self.monsters[m].kind == "calamity");
        let text: Option<String> = if let Some(m) = cal {
            let dmg = self.cr(&self.chars[h], true) * 4.0;
            self.monsters[m].power -= dmg;
            let mname = self.monsters[m].name.clone();
            let mut tx = format!("turns {rite} upon {mname}, burning away {} of its strength", crate::economy::thousands(dmg));
            if self.monsters[m].power <= 0.0 {
                self.monsters[m].alive = false;
                self.deed(h, "calamity");
                self.renown(h, 1000.0);
                let b = self.monsters[m].born;
                let who = self.who(h);
                self.calamity_log.push((mname.clone(), b, y, format!("unmade by {who} with {rite}")));
                self.monsters[m].slain_by = h;
                tx += &format!(", and **{mname} is unmade**");
                let it = self.make_item(6, format!("the corpse of {mname}"));
                self.give_item(h, it, false);
            }
            Some(tx)
        } else if s == 0 && at_war {
            let tgt = (0..self.towns.len()).filter(|&t| self.towns[t].alive && self.war[k][self.towns[t].realm]).max_by(|&a, &b| self.towns[a].pop.partial_cmp(&self.towns[b].pop).unwrap());
            tgt.map(|tg| {
                let tp = self.towns[tg].pos;
                for t in 0..self.towns.len() { if t != tg && self.towns[t].alive && crate::monsters::mdist(self.towns[t].pos, tp) <= 5.0 && self.towns[t].realm != k { self.towns[t].pop *= 0.5; } }
                let tx = format!("speaks {rite}; fire falls from a clear sky and {} of the {} is gone", self.towns[tg].name, self.pname(self.towns[tg].realm));
                self.destroy(tg);
                for o in 0..self.realms.len() { if o != k { self.relation[k][o] -= 25.0; self.relation[o][k] -= 25.0; } }
                tx
            })
        } else if s == 4 && at_war {
            self.realms[k].mods.retain(|m| m.0 != "war_power"); self.realms[k].mods.push(("war_power", 2.0, y + 10));
            let r = self.realms[k].ruler;
            self.add_tyranny(r, 20.0, "the Long Night");
            self.chars[h].tyranny += 20.0;
            for o in 0..self.realms.len() { if o != k { self.relation[k][o] -= 40.0; self.relation[o][k] -= 40.0; } }
            Some(format!("speaks {rite}; the dead of every battlefield rise to march for the {}", self.pname(k)))
        } else if s == 5 && at_war {
            self.realms[k].mods.retain(|m| m.0 != "martial"); self.realms[k].mods.push(("martial", 1.4, y + 10));
            Some(format!("speaks {rite}; the moon goes red and the {} fight like their fathers' ghosts", self.pname(k)))
        } else if s == 3 && self.alive_towns_of(k).iter().any(|&t| self.towns[t].walls < 5) {
            for t in self.alive_towns_of(k) { self.towns[t].walls = (self.towns[t].walls + 2).min(5); }
            Some(format!("speaks {rite}; overnight every town of the {} stands behind new stone", self.pname(k)))
        } else if s == 1 && y - self.last_plague < 10 {
            for t in self.alive_towns_of(k) { self.towns[t].pop *= 1.15; }
            for o in self.living.clone() { if self.chars[o].alive && self.chars[o].hero && self.chars[o].realm == k { self.chars[o].hp = 100.0; } }
            Some(format!("speaks {rite}; the sick rise from their beds across the land of the {}", self.pname(k)))
        } else if s == 2 && self.blight > 0 {
            self.realms[k].mods.retain(|m| m.0 != "food"); self.realms[k].mods.push(("food", 1.4, y + 25));
            Some(format!("speaks {rite}; the blight lifts and the {}'s fields bear twice over for a generation", self.pname(k)))
        } else { None };
        if let Some(tx) = text {
            self.chars[h].rite_ready = y + 60;
            self.deed(h, "rite");
            self.renown(h, 50.0);
            self.rites.push((y, h, rite));
            let tier = self.tier(&self.chars[h]);
            let t = format!("**World Rite.** {}, a {}-tier caster of the {}, {tx}.", self.who(h), TIER_WORDS[tier as usize], SCHOOLS[s]);
            self.say(t, false);
        }
    }

    /// The year's reckoning for each hero: war wounds, guard duty, and what the guild learns from it.
    pub fn heroes_resolve(&mut self, won: &[u32]) {
        for h in std::mem::take(&mut self.acted) {
            let Some((s, a, r0)) = self.chars[h].pending.take() else { continue };
            let k = self.chars[h].realm;
            if self.chars[h].alive && a == WAR && self.war[k].iter().any(|&w| w) {
                let foes: Vec<usize> = (0..self.realms.len()).filter(|&e| self.war[k][e]).collect();
                let enemy = self.pname(foes[0]);
                let blame = self.realms[foes[0]].ruler;
                if self.rng.random() < 0.25 / (1.0 + 0.05 * self.chars[h].level as f64) {
                    let d = self.rng.uniform(40.0, 110.0); self.hurt(h, d, format!("fell in the war against the {enemy}"), NONE, blame);
                } else {
                    let d = self.rng.uniform(0.0, 30.0); self.hurt(h, d, format!("died of wounds taken fighting the {enemy}"), NONE, blame);
                }
                if self.chars[h].alive {
                    self.deed(h, "war");
                    let l = self.chars[h].level as f64;
                    self.gain_xp(h, 150.0 + 4.0 * l);
                    self.renown(h, (2.0 * won.get(k).copied().unwrap_or(0) as f64).min(10.0));
                }
            }
            if self.chars[h].alive && a == GUARD {
                let home = self.chars[h].home;
                if home != NONE {
                    match self.towns[home].raided {
                        Some("repelled") => { self.deed(h, "guard"); self.gain_xp(h, 100.0); self.renown(h, 5.0); }
                        Some("lost") => {
                            let m = self.towns[home].raided_by;
                            let d = self.rng.uniform(20.0, 60.0);
                            let why = format!("fell defending {} from {}", self.towns[home].name, if m != NONE { self.monsters[m].name.clone() } else { "raiders".into() });
                            self.hurt(h, d, why, m, NONE);
                        }
                        _ => {}
                    }
                }
            }
            let p = self.chars[h].people;
            let mut done = !self.chars[h].alive;
            let combat_death = done && !matches!(self.chars[h].cause.as_deref(), Some("died of old age") | Some("died of illness"));
            let reward = (self.chars[h].renown - r0) / 10.0 + if combat_death { -5.0 } else { 0.05 };
            self.guild_credit(h, reward);
            let mut s2 = None;
            if !done {
                let home = self.hero_home(h);
                if home == NONE { done = true; } else { s2 = Some(self.hero_obs(h, home)); }
            }
            let _ = done;
            self.guild_learn(p, Self::key_obs(s), a, reward, s2);
            if self.chars[h].alive { let c = self.cr(&self.chars[h], false); self.chars[h].peak_cr = self.chars[h].peak_cr.max(c); }
        }
    }
}
