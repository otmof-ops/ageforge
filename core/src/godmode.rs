//! God mode (godmode.py): every act the player can take on the world, each written into the chronicle as an act of
//! the gods. One dispatcher, so a world's history of acts can be replayed exactly.

use serde::de::DeserializeOwned;

use crate::people::*;

use crate::state::State;
use crate::tables::*;

/// The arguments of an act, as JSON: what the record of a world stores, and what any front end can build.
pub type Args = serde_json::Map<String, serde_json::Value>;

fn get<T: DeserializeOwned>(a: &Args, k: &str) -> Result<T, String> {
    let v = a.get(k).ok_or_else(|| format!("missing argument {k}"))?;
    serde_json::from_value(v.clone()).map_err(|e| format!("argument {k}: {e}"))
}
fn opt<T: DeserializeOwned>(a: &Args, k: &str) -> Result<Option<T>, String> {
    match a.get(k) { Some(v) if !v.is_null() => serde_json::from_value(v.clone()).map(Some).map_err(|e| format!("argument {k}: {e}")), _ => Ok(None) }
}
fn ptrait(name: &str) -> Option<usize> { PERSONALITY.iter().position(|p| *p == name) }

impl State {
    fn god_say(&mut self, text: String) -> String { self.say(format!("**God.** {text}"), false); text }

    fn full(&self, c: usize) -> String {
        let cc = &self.chars[c];
        let mut n = cc.name.clone();
        if let Some(r) = &cc.regnal { n += &format!(" {r}"); }
        if let Some(e) = &cc.epithet { n += &format!(" {e}"); }
        n
    }

    fn set_trait(&mut self, c: usize, trait_: &str, on: bool) -> Result<String, String> {
        if let Some(t) = ptrait(trait_) {
            let opp = t ^ 1;
            let p = &mut self.chars[c].personality;
            if on { p.retain(|&x| x != opp); if !p.contains(&t) { p.push(t); } } else { p.retain(|&x| x != t); }
        } else if let Some(g) = GENETIC.iter().position(|x| *x == trait_) {
            let fx = GENETIC_FX[g];
            let ch = &mut self.chars[c];
            if on && !ch.has(g) {
                ch.genetic.push(g); ch.latent.retain(|&x| x != g);
                for i in 0..6 { ch.attrs[i] += fx[i]; }
                ch.prowess += fx[6]; ch.talent = (ch.talent + fx[7]).min(10.0); ch.life *= fx[8];
            } else if !on && ch.has(g) {
                ch.genetic.retain(|&x| x != g);
                for i in 0..6 { ch.attrs[i] -= fx[i]; }
                ch.prowess -= fx[6]; ch.talent = (ch.talent - fx[7]).max(0.0); ch.life /= fx[8];
            }
        } else { return Err(format!("unknown trait {trait_}")); }
        let t = format!("{} {} the trait {trait_}.", self.full(c), if on { "gains" } else { "loses" });
        Ok(self.god_say(t))
    }

    fn raise_level(&mut self, c: usize, target: u32) {
        while self.chars[c].level < target.min(self.chars[c].level_cap()) {
            let need = crate::people::xp_needed(self.chars[c].level) + 1.0;
            let before = self.chars[c].level;
            self.gain_xp(c, need);
            if self.chars[c].level == before { break; }
        }
    }

    pub fn god_act(&mut self, act: &str, a: &Args) -> Result<String, String> {
        let y = self.year;
        Ok(match act {
            "create_character" => {
                let (name, people, female, age, realm): (String, usize, bool, i64, usize) = (get(a, "name")?, get(a, "people")?, get(a, "female")?, get(a, "age")?, get(a, "realm")?);
                let c = self.make_character(realm, y - age, Some(female), NONE, NONE, NONE);
                if self.chars[c].people != people {
                    let (l, p, t) = (LIFESPAN[people] * self.rng.uniform(0.9, 1.1), PROWESS_BASE[people] + self.rng.normal(0.0, 3.0), TALENT_BASE[people] + self.rng.normal(0.0, 1.1));
                    let ch = &mut self.chars[c];
                    ch.people = people; ch.life = l; ch.prowess = p; ch.talent = t.clamp(0.0, 10.0);
                    for &g in &ch.genetic.clone() { ch.life *= GENETIC_FX[g][8]; ch.prowess += GENETIC_FX[g][6]; }
                }
                self.chars[c].name = name;
                self.chars[c].custom = true;
                if let Some(ps) = opt::<Vec<String>>(a, "personality")? { self.chars[c].personality = ps.iter().filter_map(|p| ptrait(p)).collect(); }
                for g in opt::<Vec<String>>(a, "genetic")?.unwrap_or_default() { self.set_trait(c, &g, true)?; }
                for g in opt::<Vec<String>>(a, "latent")?.unwrap_or_default() { if let Some(i) = GENETIC.iter().position(|x| *x == g) { if BLOODLINES.contains(&i) && !self.chars[c].latent.contains(&i) { self.chars[c].latent.push(i); } } }
                if let Some(at) = opt::<std::collections::HashMap<String, f64>>(a, "attrs")? { for (k, v) in at { if let Some(i) = ATTRS.iter().position(|x| *x == k) { self.chars[c].attrs[i] = v.floor(); } } }
                if let Some(p) = opt::<f64>(a, "prowess")? { self.chars[c].prowess = p; }
                if let Some(t) = opt::<f64>(a, "talent")? { self.chars[c].talent = t; }
                if let Some(cls) = opt::<String>(a, "cls")? { if let Some(k) = CLASSES.iter().position(|x| x.name == cls && !x.prestige) { self.chars[c].classes = vec![(k, 1)]; } }
                if let Some(sk) = opt::<std::collections::HashMap<String, f64>>(a, "skills")? { for (s, v) in sk { if let Some(i) = SKILLS.iter().position(|x| *x == s) { self.chars[c].skills[i] = v; } } }
                let level: u32 = opt(a, "level")?.unwrap_or(1);
                self.raise_level(c, level);
                self.chars[c].gold = opt(a, "gold")?.unwrap_or(0.0);
                let home: Option<usize> = opt(a, "home")?;
                self.chars[c].home = home.unwrap_or_else(|| self.alive_towns_of(realm).into_iter().max_by(|&x, &z| self.towns[x].pop.partial_cmp(&self.towns[z].pop).unwrap()).unwrap_or(NONE));
                if opt::<bool>(a, "hero")?.unwrap_or(false) { self.start_hero(c, None); }
                // where they come from and what they want, as the player wrote them (lives.rs)
                if let Some(o) = opt::<String>(a, "origin")? { if let Some(k) = crate::lives::ORIGINS.iter().find(|x| **x == o) { self.chars[c].origin = k; } }
                if self.chars[c].origin.is_empty() { self.chars[c].origin = "wanderer"; }
                let home_name = if self.chars[c].home != NONE { self.towns[self.chars[c].home].name.clone() } else { self.realms[realm].name.clone() };
                self.chars[c].origin_story = match opt::<String>(a, "origin_story")? {
                    Some(t) if !t.trim().is_empty() => t.trim().to_string(),
                    _ => format!("Set down in {home_name} in year {y} by the gods' hand, aged {age}; {}.", crate::lives::origin_words(self.chars[c].origin)),
                };
                if let Some(am) = opt::<String>(a, "ambition")? { if let Some(x) = crate::lives::AMBITIONS.iter().find(|x| x.0 == am) { self.chars[c].ambition = x.0; } }
                let t = format!("{}, a {} {} of {age}, is set down in {} by the gods' hand.", self.full(c), ADJ[people], if female { "woman" } else { "man" }, self.realms[realm].name);
                self.god_say(t);
                if opt::<bool>(a, "track")?.unwrap_or(false) { self.begin_life_log(c); }
                c.to_string()
            }
            "set_stat" => {
                let (c, key, v): (usize, String, f64) = (get(a, "c")?, get(a, "key")?, get(a, "value")?);
                if let Some(i) = ATTRS.iter().position(|x| *x == key) { self.chars[c].attrs[i] = v.floor(); }
                else if let Some(i) = SKILLS.iter().position(|x| *x == key) { self.chars[c].skills[i] = v.clamp(0.0, 120.0); }
                else {
                    let ch = &mut self.chars[c];
                    match key.as_str() {
                        "prowess" => ch.prowess = v, "talent" => ch.talent = v.clamp(0.0, 10.0), "renown" => ch.renown = v, "gold" => ch.gold = v,
                        "stress" => ch.stress = v, "happy" => ch.happy = v, "health" => ch.health = v, "life" => ch.life = v,
                        "level" => { let target = v as u32; if target > ch.level { self.raise_level(c, target); } else { let d = ch.level - target.max(1); ch.level = target.max(1); let last = ch.classes.last_mut().unwrap(); last.1 = last.1.saturating_sub(d).max(1); } }
                        _ => return Err(format!("unknown stat {key}")),
                    }
                }
                let t = format!("{}'s {key} is now {v}.", self.full(c));
                self.god_say(t)
            }
            "set_trait" => { let (c, t, on): (usize, String, bool) = (get(a, "c")?, get(a, "trait")?, get(a, "on")?); self.set_trait(c, &t, on)? }
            "set_class" => {
                let (c, cls): (usize, String) = (get(a, "c")?, get(a, "cls")?);
                let k = CLASSES.iter().position(|x| x.name == cls).ok_or_else(|| format!("unknown class {cls}"))?;
                self.chars[c].classes.push((k, 1));
                let t = format!("{} takes up the class {cls}.", self.full(c));
                self.god_say(t)
            }
            "make_hero" => { let c: usize = get(a, "c")?; if !self.chars[c].hero { self.start_hero(c, Some("is called to the road by a dream sent from above".into())); } format!("{} is a hero now.", self.full(c)) }
            "crown_character" => {
                let c: usize = get(a, "c")?;
                let k = self.chars[c].realm;
                let old = self.realms[k].ruler;
                if old != NONE && old != c { self.end_reign(k, "set aside by the gods"); }
                self.crown(k, c, "raised up when the line failed");
                let t = format!("The gods set {} on the throne of {}.", self.full(c), self.realms[k].name);
                self.god_say(t)
            }
            "marry" => {
                let (x, z): (usize, usize) = (get(a, "a")?, get(a, "b")?);
                for p in [x, z] { let s = self.chars[p].spouse; if s != NONE && self.chars[s].alive { self.chars[s].spouse = NONE; } }
                self.chars[x].spouse = z; self.chars[z].spouse = x;
                self.heart_on_marriage(x, z);
                let t = format!("{} and {} are wed by divine decree.", self.full(x), self.full(z));
                self.god_say(t)
            }
            "make_love" => {
                let (x, z): (usize, usize) = (get(a, "a")?, get(a, "b")?);
                self.love(x, z, "soulmate", 100.0); self.love(z, x, "soulmate", 100.0);
                let t = format!("{} and {} fall in love, as the gods intend.", self.full(x), self.full(z));
                self.god_say(t)
            }
            "give_item" => {
                let (c, name, rarity): (usize, String, usize) = (get(a, "c")?, get(a, "name")?, opt(a, "rarity")?.unwrap_or(5));
                let mut it = Item::new(name, rarity.min(6), y, "the hands of the gods".into());
                it.bonus = RARITY_BONUS[rarity.min(6)];
                let id = self.new_item(it);
                self.give_item(c, id, true);
                let t = format!("{} finds {} at their bedside one morning.", self.full(c), self.items[id].label());
                self.god_say(t)
            }
            "make_immortal" => {
                let (c, on): (usize, bool) = (get(a, "c")?, get(a, "on")?);
                if on { self.chars[c].life = 1e6; self.chars[c].undead = true; }
                let t = format!("{} {}.", self.full(c), if on { "will not die of age" } else { "is mortal again" });
                self.god_say(t)
            }
            "make_custom" => {
                let (c, on): (usize, bool) = (get(a, "c")?, get(a, "on")?);
                self.chars[c].custom = on;
                let t = format!("{} is {}.", self.full(c), if on { "set apart by the gods" } else { "an ordinary mortal again" });
                self.god_say(t)
            }
            "strike_down" => { let c: usize = get(a, "c")?; self.kill(c, "was struck down by the gods".into()); let t = format!("{} is struck down from the sky.", self.full(c)); self.god_say(t) }
            "raise_dead" => {
                let c: usize = get(a, "c")?;
                if self.chars[c].alive { return Ok(format!("{} is not dead.", self.full(c))); }
                let ch = &mut self.chars[c];
                ch.alive = true; ch.died = None; ch.cause = None; ch.hp = 100.0; ch.health = 6.0;
                self.rebuild_living();
                let t = format!("{} walks out of the grave.", self.full(c));
                self.god_say(t)
            }
            "wake_calamity" => {
                let (lid, world): (i32, bool) = (get(a, "lid")?, opt(a, "world")?.unwrap_or(false));
                let terrain = self.map.terrain.clone();
                let lm = self.map.landmass.clone();
                let cells = self.remote_cells(&|r, c| terrain.at(r, c) != OCEAN && lm.at(r, c) == lid);
                if cells.is_empty() { return Ok("No wild land there to wake anything in.".into()); }
                let pos = cells[self.rng.integers(0, cells.len() as i64) as usize];
                let p = self.rng.uniform(8000.0, 22000.0) * if world { 2.0 } else { 1.0 };
                let m = self.new_monster("calamity", pos, p, None, &[], false, world);
                let t = format!("The gods wake {} beneath {}.", self.monsters[m].name, self.map.lands[lid as usize].name);
                self.god_say(t)
            }
            "loose_monster" => {
                let (kind, kid): (String, usize) = (get(a, "kind")?, get(a, "kid")?);
                let kind: &'static str = ["troll", "wraith", "demon", "dragon"].into_iter().find(|k| *k == kind).unwrap_or("troll");
                let cap = self.capital_of(kid);
                if cap == NONE { return Ok("That realm has no seat.".into()); }
                let cp = self.towns[cap].pos;
                let (dr, dc) = (self.rng.integers(-8, 9), self.rng.integers(-8, 9));
                let mut pos = ((cp.0 as i64 + dr).clamp(0, self.map.h as i64 - 1) as usize, (cp.1 as i64 + dc).clamp(0, self.map.w as i64 - 1) as usize);
                if self.map.terrain.at(pos.0, pos.1) == OCEAN { pos = cp; }
                let pr = MONSTER.iter().find(|m| m.0 == kind).unwrap().2;
                let p = self.rng.uniform(pr.0, pr.1) * 1.5;
                let m = self.new_monster(kind, pos, p, None, &[], false, false);
                let t = format!("The gods loose {} upon {}.", self.monsters[m].name, self.realms[kid].name);
                self.god_say(t)
            }
            "smite_town" => { let t: usize = get(a, "town")?; self.destroy(t); let tx = format!("Fire falls on {}; nothing is left of it.", self.towns[t].name); self.god_say(tx) }
            "send_plague" => {
                let (kid, great): (usize, bool) = (get(a, "kid")?, opt(a, "great")?.unwrap_or(true));
                let towns = self.alive_towns_of(kid);
                let Some(&src) = towns.iter().max_by(|&&x, &&z| self.towns[x].pop.partial_cmp(&self.towns[z].pop).unwrap()) else { return Ok("No towns there.".into()) };
                self.epidemics.push(Epidemic { name: format!("the Gods' {}", if great { "Scourge" } else { "Fever" }), start: y, fatality: if great { 0.5 } else { 0.15 },
                    spread: if great { 0.75 } else { 0.4 }, great, deaths: 0.0, towns: Default::default(), front: vec![(src, 0.3)], end: None });
                let t = format!("A plague is sent upon {}, starting in {}.", self.realms[kid].name, self.towns[src].name);
                self.god_say(t)
            }
            "set_harvest" => {
                let (kid, mult, years): (usize, f64, i64) = (get(a, "kid")?, get(a, "mult")?, opt(a, "years")?.unwrap_or(3));
                self.god_harvest.retain(|g| g.0 != kid);
                self.god_harvest.push((kid, mult, y + years));
                let t = format!("The gods send {} upon {} for {years} years.", if mult > 1.0 { "a blessed harvest" } else { "famine" }, self.realms[kid].name);
                self.god_say(t)
            }
            "grant_gold" => {
                let (kid, amount): (usize, f64) = (get(a, "kid")?, get(a, "amount")?);
                self.realms[kid].gold += amount;
                self.books.push((kid, "divine gift", amount, "gold from the gods".into(), NONE));
                let t = format!("{} gold appears in the treasury of {}.", crate::economy::thousands(amount), self.realms[kid].name);
                self.god_say(t)
            }
            "declare_war" => {
                let (x, z): (usize, usize) = (get(a, "a")?, get(a, "b")?);
                self.god_war(x, z);
                let t = format!("The gods set {} against {}.", self.realms[x].name, self.realms[z].name);
                self.god_say(t)
            }
            "impose_peace" => {
                let (x, z): (usize, usize) = (get(a, "a")?, get(a, "b")?);
                self.god_peace(x, z);
                let t = format!("The gods impose peace between {} and {}.", self.realms[x].name, self.realms[z].name);
                self.god_say(t)
            }
            "change_government" => {
                let (kid, gov): (usize, String) = (get(a, "kid")?, get(a, "gov")?);
                self.change_gov(kid, &gov, "By the will of the gods");
                format!("{} is under {gov} rule.", self.realms[kid].name)
            }
            "raise_era" => {
                let cu: usize = get(a, "culture")?;
                let era = self.cultures[cu].era;
                if era >= ERAS.len() - 1 { return Ok("That people already stands at the last era.".into()); }
                for i in INNOVATIONS.iter().filter(|i| i.era == era) { if self.cultures[cu].innovations.insert(i.name) { self.cultures[cu].learned.push((i.name, y)); } }
                self.cultures[cu].era += 1;
                let t = format!("The gods teach the {}; they enter the {} age.", PEOPLES[cu], ERAS[era + 1]);
                self.god_say(t)
            }
            "golden_age" => { let cu: usize = get(a, "culture")?; self.cultures[cu].golden = y + 40; self.god_say(format!("A golden age is granted to the {}.", PEOPLES[cu])) }
            "break_realm" => {
                let (kid, lid): (usize, i32) = (get(a, "kid")?, get(a, "lid")?);
                let cap = self.capital_of(kid);
                let cd = if cap != NONE { self.towns[cap].duchy } else { None };
                let mut dks: Vec<DKey> = self.alive_towns_of(kid).into_iter().filter(|&t| self.towns[t].landmass == lid).filter_map(|t| self.towns[t].duchy).filter(|d| Some(*d) != cd).collect();
                dks.sort(); dks.dedup();
                if dks.is_empty() { return Ok("Nothing there to break away.".into()); }
                let leader = dks.iter().filter_map(|d| self.duchies.get(d)).map(|d| d.holder).filter(|&h| h != NONE).max_by(|&x, &z| self.chars[x].renown.partial_cmp(&self.chars[z].renown).unwrap());
                let leader = match leader { Some(l) => l, None => self.make_character(kid, y - 35, None, NONE, NONE, NONE) };
                self.secede(kid, &dks, leader, 0);
                let t = format!("The gods tear {} from {}.", self.map.lands[lid as usize].name, self.realms[kid].name);
                self.god_say(t)
            }
            "curse_house" => { let h: usize = get(a, "hid")?; self.curse_house(h, "the gods", "divine displeasure"); format!("{} is cursed.", self.houses[h].name) }
            "name_concord" => {
                let c: usize = get(a, "c")?;
                if self.chars[c].element.is_none() { self.chars[c].element = Some(ELEMENTS[0]); self.chars[c].bend = vec![(0, 25.0)]; }
                self.chars[c].concord = true;
                self.concord = c;
                let e = ELEMENTS.iter().position(|x| Some(*x) == self.chars[c].element).unwrap_or(0);
                let born = self.chars[c].born;
                self.concord_line.push(crate::bending::ConcordLife { id: c, born, element: e, need: "the gods' choosing".into(), revealed: None, died: None, deeds: vec![] });
                let t = format!("The gods name {} the Concord.", self.full(c));
                self.god_say(t)
            }
            "god_note" => { let t: String = get(a, "text")?; self.god_say(t) }
            "track" => {
                let (c, on): (usize, bool) = (get(a, "c")?, get(a, "on")?);
                if on { self.begin_life_log(c); } else { self.chars[c].tracked = false; }
                format!("{} {} tracked.", self.full(c), if on { "is now" } else { "is no longer" })
            }
            _ => return Err(format!("unknown act {act}")),
        })
    }
}
