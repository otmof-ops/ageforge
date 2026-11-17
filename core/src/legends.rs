//! Legends and curses (legends.py): the great may curse as they die (a land, a bloodline, a weapon, or a curse of
//! return); a returning one comes back in every age for a relic, and a Hero's Mark, a Sealing Light and a sworn
//! blade answer it; legendary matter makes legendary weapons; if the hero fails, the Fall, and the land withers.

use crate::monsters::mdist;
use crate::people::*;
use crate::power::skill_ix;
use crate::state::State;
use crate::tables::*;

pub struct Curse { pub kind: &'static str, pub source: String, pub since: i64, pub lifted: Option<i64>, pub pos: (usize, usize), pub radius: f64, pub land: i32,
                   pub house: usize, pub item: usize, pub cycle: usize }

pub struct Cycle {
    pub id: usize, pub villain: String, pub title: &'static str, pub landmass: i32, pub born: i64, pub power: f64, pub period: i64, pub next_return: i64,
    pub relic: usize, pub blade: usize, pub hero: usize, pub seer: usize, pub state: &'static str, pub monster: usize, pub ruler: usize,
    pub ruler_power: f64, pub dark_center: (usize, usize), pub dark_radius: f64, pub ages: Vec<String>, pub kept_at: usize, pub old_name: String,
}

pub struct Material { pub kind: &'static str, pub name: String, pub land: i32, pub year: i64, pub cycle: usize, pub used: bool }

fn ordinal(n: usize) -> String { match n { 1 => "first".into(), 2 => "second".into(), 3 => "third".into(), 4 => "fourth".into(), 5 => "fifth".into(), 6 => "sixth".into(), _ => format!("{n}th") } }
fn mat_name(kind: &str, x: &str) -> String { LEGEND_MATERIALS.iter().find(|m| m.0 == kind).unwrap().1.replace("{}", x) }

impl State {
    pub fn myth(&mut self, title: String, text: String) { self.myths.push((self.year, title, text)); }

    fn new_curse(&mut self, kind: &'static str, source: String) -> Curse {
        Curse { kind, source, since: self.year, lifted: None, pos: (0, 0), radius: 0.0, land: -1, house: NONE, item: NONE, cycle: NONE }
    }

    /// Anyone dies: the great may curse with their last breath (legends.on_death).
    pub fn legends_on_death(&mut self, c: usize, cause: &str, killer_house: usize) {
        let cc = &self.chars[c];
        let spirit = self.tier(cc).max(0) as f64 / 10.0 + (cc.renown / 2000.0).min(1.0) + 0.3 * cc.undead as u8 as f64;
        let k = cc.realm;
        let tyrant = cc.tyranny >= 60.0 && cc.ruled.is_some();
        let struck = cc.ruled.is_some() && cc.renown >= 300.0 && !["old age", "illness", "childhood"].iter().any(|w| cause.contains(w));
        let betrayed = cc.hero && cc.renown >= 400.0 && ["purges", "executed", "murdered"].iter().any(|w| cause.contains(w));
        let lich = cc.undead;
        if !(tyrant || betrayed || lich || struck) || self.rng.random() > 0.15 + 0.35 * spirit { return; }
        let culprit = if betrayed && k < self.realms.len() && self.realms[k].ruler != NONE { self.chars[self.realms[k].ruler].house } else { killer_house };
        let roll = self.rng.random();
        let full = match &cc.regnal { Some(r) => format!("{} {r}", cc.name), None => cc.name.clone() };
        if (tyrant || lich) && roll < 0.35 {
            let home = self.capital_of(k);
            if home != NONE {
                let lid = self.towns[home].landmass;
                self.make_cycle(full.clone(), lid, 9000.0 + 3000.0 * spirit, NONE, &format!("the tyrant {full}, dying, swears to return"));
                return;
            }
        }
        if !self.chars[c].items.is_empty() && roll < 0.55 {
            let it = *self.chars[c].items.iter().max_by_key(|&&i| self.items[i].rarity).unwrap();
            let mut cu = self.new_curse("item", full.clone());
            cu.item = it;
            self.curses.push(cu);
            let iname = self.items[it].name.clone();
            self.myth(format!("The curse on {iname}"), format!("{full} poured their last breath into {iname}. It gives strength, and takes the mind."));
            self.say(format!("**A curse.** As {full} dies, {iname} drinks their last breath. Whoever bears it now will be strong, and not themselves."), false);
            return;
        }
        if culprit != NONE { self.curse_house(culprit, &full, if betrayed { "betrayal" } else { "a dying curse" }); }
        else {
            let home = if self.chars[c].home != NONE { self.chars[c].home } else { self.capital_of(k) };
            if home != NONE { let (p, l) = (self.towns[home].pos, self.towns[home].landmass); self.curse_land(p, l, &full, 10.0 + 10.0 * spirit); }
        }
    }

    pub fn curse_house(&mut self, hid: usize, source: &str, why: &str) {
        if self.houses[hid].cursed { return; }
        self.houses[hid].cursed = true;
        let hn = self.houses[hid].name.clone();
        let mut cu = self.new_curse("blood", source.to_string());
        cu.house = hid;
        self.curses.push(cu);
        self.myth(format!("The curse of {hn}"), format!("{source} cursed {hn} for {why}: its children sicken, its lords go mad, its heirs die young."));
        self.say(format!("**A blood curse.** {source} curses {hn} for {why}. Its children will sicken and its lords go mad, until the curse is lifted."), false);
    }

    pub fn curse_land(&mut self, pos: (usize, usize), lid: i32, source: &str, radius: f64) {
        let mut cu = self.new_curse("land", source.to_string());
        cu.pos = pos; cu.radius = radius; cu.land = lid;
        self.curses.push(cu);
        let ln = self.land_name(lid);
        self.myth(format!("The withering of {ln}"), format!("Where {source} fell, the land itself was cursed."));
        self.say(format!("**The land is cursed.** Where {source} fell on {ln}, the soil withers for {} leagues around, and the dead do not lie easy.", radius as i64), false);
    }

    fn keeper_town(&self, lid: i32) -> usize {
        let towns: Vec<usize> = (0..self.towns.len()).filter(|&t| self.towns[t].alive && self.towns[t].landmass == lid).collect();
        if towns.is_empty() { return NONE; }
        let mut counts: Vec<(usize, usize)> = vec![];
        for &t in &towns { let f = self.towns[t].realm; match counts.iter_mut().find(|x| x.0 == f) { Some(x) => x.1 += 1, None => counts.push((f, 1)) } }
        let f = counts.iter().max_by_key(|x| x.1).unwrap().0;
        let cap = self.capital_of(f);
        if cap != NONE && self.towns[cap].landmass == lid { return cap; }
        towns.into_iter().filter(|&t| self.towns[t].realm == f).max_by(|&a, &b| self.towns[a].pop.partial_cmp(&self.towns[b].pop).unwrap()).unwrap()
    }

    fn make_relic(&mut self, lid: i32) -> (usize, usize) {
        let ln = self.land_name(lid);
        let mut it = Item::new(format!("the Heart of {ln}"), 6, self.year, format!("the soul of {ln}"));
        it.special = Some("tier");
        it.note = "the relic the returning one seeks".into();
        let id = self.new_item(it);
        let keep = self.keeper_town(lid);
        if keep != NONE { let f = self.towns[keep].realm; self.realms[f].treasury.push(id); }
        (id, keep)
    }

    fn swear(&mut self, it: usize, cy: usize) {
        let old = self.items[it].name.clone();
        let villain = self.cycles[cy].villain.clone();
        let name = format!("{}, Sworn Against {villain}", OATH_A[self.rng.integers(0, OATH_A.len() as i64) as usize]);
        let item = &mut self.items[it];
        item.name = name.clone();
        item.rarity = item.rarity.max(5);
        item.bonus = item.bonus.max(1.0);
        self.cycles[cy].blade = it;
        self.say(format!("{old} is sworn against {villain}; it is called {name} now."), false);
    }

    pub fn make_cycle(&mut self, villain: String, lid: i32, power: f64, sworn: usize, how: &str) -> usize {
        let title = CYCLE_TITLES[self.rng.integers(0, CYCLE_TITLES.len() as i64) as usize];
        let period = self.rng.integers(150, 350);
        let (relic, kept) = self.make_relic(lid);
        let id = self.cycles.len();
        self.cycles.push(Cycle { id, villain: villain.clone(), title, landmass: lid, born: self.year, power, period, next_return: self.year + period, relic, blade: NONE,
            hero: NONE, seer: NONE, state: "sealed", monster: NONE, ruler: NONE, ruler_power: 0.0, dark_center: (0, 0), dark_radius: 0.0, ages: vec![], kept_at: kept, old_name: String::new() });
        if sworn != NONE { self.swear(sworn, id); }
        let mut cu = self.new_curse("cycle", villain.clone());
        cu.cycle = id;
        self.curses.push(cu);
        let (ln, rn) = (self.land_name(lid), self.items[relic].name.clone());
        let blade = if self.cycles[id].blade != NONE { format!("Only {} can seal it.", self.items[self.cycles[id].blade].name) } else { "No blade is yet sworn against it.".into() };
        self.myth(format!("The curse of {villain}, {title}"), format!("{how}: in every age it will return to {ln} to take {rn}. {blade}"));
        let nr = self.cycles[id].next_return;
        self.say(format!("**A curse of return.** {how}: in every age they will come back to {ln} to take {rn}, and only a sworn blade can seal them again. The sages reckon the next age begins in year {nr}."), false);
        id
    }

    /// A calamity that goes back into the deep unbeaten may swear to return in every age.
    pub fn on_slumber(&mut self, m: usize) {
        if self.rng.random() > 0.5 || self.monsters[m].cycle != NONE { return; }
        let name = self.monsters[m].name.split(',').next().unwrap().to_string();
        let how = format!("{}, sinking unbeaten into the deep, swears to come back for what it was denied", self.monsters[m].name);
        let (lid, base) = (self.monsters[m].landmass, self.monsters[m].base);
        let cy = self.make_cycle(name.clone(), lid, base * 0.6, NONE, &how);
        self.materials.push(Material { kind: "heartstone", name: format!("a shard of {name}'s heart, left in the earth where it sank"), land: lid, year: self.year, cycle: cy, used: false });
    }

    pub fn legends_year(&mut self) {
        let y = self.year;
        let watch = std::mem::take(&mut self.watch);
        for m in watch {
            if self.monsters[m].alive { continue; }
            if self.monsters[m].cycle != NONE { continue; }
            let mname = self.monsters[m].name.clone();
            let slain = !self.calamity_log.iter().any(|e| e.0 == mname && e.3.contains("slumber"));
            let (kind, lid, pos, base) = (self.monsters[m].kind, self.monsters[m].landmass, self.monsters[m].pos, self.monsters[m].base);
            let slayer = self.monsters[m].slain_by;
            if kind == "calamity" && slain {
                self.materials.push(Material { kind: "heartstone", name: mat_name("heartstone", &mname), land: lid, year: y, cycle: NONE, used: false });
                if self.rng.random() < 0.75 {
                    let roll = self.rng.random();
                    if roll < 0.5 {
                        let sworn = if slayer != NONE && !self.chars[slayer].items.is_empty() { *self.chars[slayer].items.iter().max_by_key(|&&i| self.items[i].rarity).unwrap() } else { NONE };
                        let cy = self.make_cycle(mname.split(',').next().unwrap().to_string(), lid, base * 0.8, sworn, &format!("{mname}, dying, swears an oath on its own heart"));
                        self.materials.last_mut().unwrap().cycle = cy;
                    } else if roll < 0.7 || slayer == NONE || self.chars[slayer].house == NONE { self.curse_land(pos, lid, &mname, 14.0); }
                    else { let h = self.chars[slayer].house; self.curse_house(h, &mname, &format!("the slaying of {mname}")); }
                }
            } else if kind == "dragon" && self.monsters[m].title == Some("Wyrm-Sovereign") && slain {
                let short = mname.split(" (").next().unwrap().to_string();
                self.materials.push(Material { kind: "sovereign-bone", name: mat_name("sovereign-bone", &short), land: lid, year: y, cycle: NONE, used: false });
                if slayer != NONE && self.chars[slayer].house != NONE && self.rng.random() < 0.3 { let h = self.chars[slayer].house; self.curse_house(h, &short, "the death of the Wyrm-Sovereign"); }
            }
        }
        self.watch = (0..self.monsters.len()).filter(|&m| self.monsters[m].alive && (self.monsters[m].kind == "calamity" || self.monsters[m].title.is_some())).collect();
        let conts: Vec<usize> = (0..self.map.lands.len()).filter(|&l| self.map.lands[l].kind == "continent").collect();
        if !conts.is_empty() && self.rng.random() < 0.004 {
            let l = conts[self.rng.integers(0, conts.len() as i64) as usize];
            let ln = self.map.lands[l].name.clone();
            self.materials.push(Material { kind: "star-iron", name: mat_name("star-iron", &ln), land: l as i32, year: y, cycle: NONE, used: false });
            self.say(format!("A star falls burning over {ln}; the smiths of the world want what it left behind."), false);
        }
        self.forge_legends();
        self.curse_effects();
        for cy in 0..self.cycles.len() { self.cycle_turn(cy); }
    }

    fn forge_legends(&mut self) {
        for mi in 0..self.materials.len() {
            if self.materials[mi].used { continue; }
            let land = self.materials[mi].land;
            let peoples: Vec<usize> = self.towns.iter().filter(|t| t.alive && t.landmass == land).map(|t| t.realm).collect();
            let smiths: Vec<usize> = self.living.iter().map(|&i| &self.chars[i]).filter(|c| c.alive && c.profession == Some("smith") && peoples.contains(&c.realm)).map(|c| c.id)
                .filter(|&c| self.skill_val(&self.chars[c], skill_ix("Smithing")) >= 85.0).collect();
            if smiths.is_empty() || self.rng.random() > 0.2 { continue; }
            let sm = *smiths.iter().max_by(|&&a, &&b| self.skill_val(&self.chars[a], skill_ix("Smithing")).partial_cmp(&self.skill_val(&self.chars[b], skill_ix("Smithing"))).unwrap()).unwrap();
            self.materials[mi].used = true;
            let mut name = OATH_A[self.rng.integers(0, OATH_A.len() as i64) as usize].to_string();
            while self.item_names.contains(&name) { name = format!("{} of {}", OATH_A[self.rng.integers(0, OATH_A.len() as i64) as usize], self.name(&["Ar", "Bel", "Cor", "Dra", "Eth"], &["iel", "on", "ath", "ys"])); }
            self.item_names.insert(name.clone());
            let mname = self.materials[mi].name.clone();
            let mut it = Item::new(name.clone(), 5, self.year, format!("forged by {} from {mname}", self.chars[sm].name));
            it.special = Some("vs_monster");
            it.note = format!("forged from {mname}");
            it.bonus = self.rng.uniform(1.2, 2.0);
            it.maker = sm;
            let id = self.new_item(it);
            self.gain_fame(sm, "Smithing", 600.0);
            let mut text = format!("{} forges {name} from {mname}", self.who(sm));
            let cy = self.materials[mi].cycle;
            if cy != NONE && self.cycles[cy].blade == NONE { self.swear(id, cy); text += &format!(", the one blade that can seal {}", self.cycles[cy].villain); }
            let iname = self.items[id].name.clone();
            self.myth(format!("The forging of {iname}"), format!("{text}."));
            self.say(format!("**A legendary forging.** {text}."), false);
            let k = self.chars[sm].realm;
            let p = self.chars[sm].people;
            let hero = self.living.iter().map(|&i| &self.chars[i]).filter(|h| h.alive && h.hero && h.people == p).max_by(|a, b| a.renown.partial_cmp(&b.renown).unwrap()).map(|h| h.id);
            match hero {
                Some(h) => { self.give_item(h, id, true); let t = format!("{} gives {iname} to {}.", self.realms[k].name, self.who(h)); self.say(t, true); }
                None => self.realms[k].treasury.push(id),
            }
        }
    }

    fn curse_effects(&mut self) {
        let y = self.year;
        for t in self.towns.iter_mut() { t.curse_mult = 1.0; }
        for ci in 0..self.curses.len() {
            if self.curses[ci].lifted.is_some() { continue; }
            match self.curses[ci].kind {
                "land" => {
                    let (lid, pos, rad) = (self.curses[ci].land, self.curses[ci].pos, self.curses[ci].radius);
                    for t in self.towns.iter_mut() { if t.alive && t.landmass == lid && mdist(t.pos, pos) <= rad { t.curse_mult = t.curse_mult.min(0.55); } }
                    if self.rng.random() < 0.05 {
                        let r = rad as i64;
                        let rr = (pos.0 as i64 + self.rng.integers(-r, r + 1)).clamp(0, self.map.h as i64 - 1) as usize;
                        let cc = (pos.1 as i64 + self.rng.integers(-r, r + 1)).clamp(0, self.map.w as i64 - 1) as usize;
                        if self.map.terrain.at(rr, cc) != OCEAN { self.corrupted.set(rr, cc, true); }
                    }
                    let lifter = self.lifter_for();
                    if lifter != NONE || y - self.curses[ci].since > 300 { self.lift(ci, lifter); }
                }
                "blood" => {
                    let h = self.curses[ci].house;
                    let members: Vec<usize> = self.living.iter().map(|&i| &self.chars[i]).filter(|c| c.alive && c.house == h).map(|c| c.id).collect();
                    let hn = self.houses[h].name.clone();
                    let (wr, calm) = (PERSONALITY.iter().position(|p| *p == "wrathful").unwrap(), PERSONALITY.iter().position(|p| *p == "calm").unwrap());
                    for c in members {
                        if self.rng.random() < 0.015 { self.kill(c, format!("died of the curse of {hn}")); }
                        else if self.rng.random() < 0.01 && !self.chars[c].personality.contains(&wr) { self.chars[c].personality.push(wr); self.chars[c].personality.retain(|&p| p != calm); }
                        if let Some(l) = self.chars[c].legitimacy { self.chars[c].legitimacy = Some((l - 0.3).max(0.0)); }
                    }
                    let lifter = self.lifter_for();
                    if lifter != NONE || y - self.curses[ci].since > 200 { self.lift(ci, lifter); self.houses[h].cursed = false; }
                }
                "item" => {
                    let it = self.curses[ci].item;
                    let wr = PERSONALITY.iter().position(|p| *p == "wrathful").unwrap();
                    let bearers: Vec<usize> = self.living.iter().map(|&i| &self.chars[i]).filter(|c| c.alive && c.items.contains(&it)).map(|c| c.id).collect();
                    for c in bearers {
                        self.chars[c].tyranny = (self.chars[c].tyranny + 1.0).min(100.0);
                        if !self.chars[c].personality.contains(&wr) && self.rng.random() < 0.1 {
                            self.chars[c].personality.push(wr);
                            let t = format!("{} grows wrathful; {} is in their dreams.", self.who(c), self.items[it].name);
                            self.say(t, true);
                        }
                    }
                }
                _ => {}
            }
        }
    }

    fn lifter_for(&mut self) -> usize {
        if self.rng.random() > 0.02 { return NONE; }
        let cands: Vec<usize> = self.living.iter().map(|&i| &self.chars[i]).filter(|c| c.alive && (c.concord || c.classes.iter().any(|x| matches!(CLASSES[x.0].name, "Saint" | "Archdruid"))
            || self.skill_val(c, skill_ix("Restoration")) >= 95.0)).map(|c| c.id).collect();
        if cands.is_empty() { NONE } else { cands[self.rng.integers(0, cands.len() as i64) as usize] }
    }

    fn lift(&mut self, ci: usize, by: usize) {
        self.curses[ci].lifted = Some(self.year);
        let cu = &self.curses[ci];
        let what = match cu.kind { "land" => format!("the curse on the land where {} fell", cu.source), "blood" => format!("the curse on {}", self.houses[cu.house].name),
                                   "item" => format!("the curse on {}", self.items[cu.item].name), _ => "the curse".into() };
        let since = cu.since;
        if by != NONE {
            let t = format!("**A curse is lifted.** {} lifts {what}.", self.who(by));
            self.say(t, false);
            self.gain_fame(by, "prophet", 200.0);
            let n = self.chars[by].name.clone();
            self.myth(format!("The lifting of {what}"), format!("{n} lifted {what} in year {}.", self.year));
        } else {
            let mut w = what.clone(); w.replace_range(0..1, &what[0..1].to_uppercase());
            self.say(format!("{w} fades at last, after {} years.", self.year - since), true);
        }
    }

    /// In the years before a return, the hero and the seer are born again.
    pub fn legends_on_birth(&mut self, kid: usize) {
        for cy in 0..self.cycles.len() {
            if self.year < self.cycles[cy].next_return - 45 && self.cycles[cy].state == "sealed" { continue; }
            let k = self.chars[kid].realm;
            if !self.alive_towns_of(k).iter().any(|&t| self.towns[t].landmass == self.cycles[cy].landmass) { continue; }
            let ln = self.land_name(self.cycles[cy].landmass);
            if self.cycles[cy].hero == NONE {
                self.cycles[cy].hero = kid;
                self.chars[kid].marks.push(HERO_MARK[0]);
                let t = format!("A child, {}, is born on {ln} with {} on their hand. The old songs of {} are sung again.", self.chars[kid].name, HERO_MARK[0], self.cycles[cy].villain);
                self.say(t, true);
            } else if self.cycles[cy].seer == NONE && self.chars[kid].house != NONE && self.chars[kid].house == self.realms[k].house {
                self.cycles[cy].seer = kid;
                self.chars[kid].marks.push(SEER_GIFT[0]);
                let t = format!("A royal child, {} of {}, is born with {}.", self.chars[kid].name, self.houses[self.chars[kid].house].name, SEER_GIFT[0]);
                self.say(t, true);
            }
        }
    }

    fn cycle_turn(&mut self, cy: usize) {
        let y = self.year;
        let (h, s) = (self.cycles[cy].hero, self.cycles[cy].seer);
        if h != NONE && !self.chars[h].alive { self.cycles[cy].hero = NONE; }
        if s != NONE && !self.chars[s].alive { self.cycles[cy].seer = NONE; }
        match self.cycles[cy].state {
            "sealed" => { if y >= self.cycles[cy].next_return { self.ret(cy); } }
            "returned" => {
                let m = self.cycles[cy].monster;
                if m == NONE || !self.monsters[m].alive {
                    if m != NONE && !self.monsters[m].sealed {
                        let half = self.cycles[cy].period / 2;
                        self.cycles[cy].state = "sealed";
                        self.cycles[cy].next_return = y + half;
                        self.cycles[cy].ages.push(format!("ended {y}: scattered by other hands, not sealed; it will be back sooner"));
                        let v = self.cycles[cy].villain.clone();
                        self.say(format!("{v} is struck down, but not by the sworn blade. It is only scattered; the sages say it will be back within {half} years."), false);
                    }
                    return;
                }
                let mut rt = self.cycles[cy].kept_at;
                if rt == NONE || !self.towns[rt].alive { rt = self.keeper_town(self.cycles[cy].landmass); self.cycles[cy].kept_at = rt; }
                if rt != NONE {
                    let (tp, mp) = (self.towns[rt].pos, self.monsters[m].pos);
                    let (dy, dx) = (tp.0 as f64 - mp.0 as f64, tp.1 as f64 - mp.1 as f64);
                    let dist = dy.hypot(dx);
                    if dist > 3.0 {
                        let step = dist.min(6.0);
                        let np = ((mp.0 as f64 + dy / dist * step) as usize, (mp.1 as f64 + dx / dist * step) as usize);
                        if self.map.terrain.at(np.0, np.1) != OCEAN { self.monsters[m].pos = np; }
                    } else {
                        self.assault(m, rt, 0.3);
                        if self.towns[rt].pop < 200.0 || self.rng.random() < 0.15 { self.fall(cy, rt); return; }
                    }
                }
                self.hero_strikes(cy);
            }
            "triumphant" => { self.spread_dark(cy); self.hero_strikes(cy); }
            _ => {}
        }
    }

    fn ret(&mut self, cy: usize) {
        let lid = self.cycles[cy].landmass;
        let terrain = self.map.terrain.clone();
        let lm = self.map.landmass.clone();
        let cells = self.remote_cells(&|r, c| terrain.at(r, c) != OCEAN && lm.at(r, c) == lid);
        if cells.is_empty() { self.cycles[cy].next_return = self.year + 50; return; }
        let pos = cells[self.rng.integers(0, cells.len() as i64) as usize];
        let n = self.cycles[cy].ages.len() + 1;
        self.cycles[cy].power *= 1.25;
        let (v, t, p) = (self.cycles[cy].villain.clone(), self.cycles[cy].title, self.cycles[cy].power);
        let m = self.new_monster("calamity", pos, p, Some(format!("{v}, {t}")), &[], true, false);
        self.monsters[m].cycle = cy;
        self.cycles[cy].monster = m;
        self.cycles[cy].state = "returned";
        let (h, s) = (self.cycles[cy].hero, self.cycles[cy].seer);
        let mut text = format!("**{v} returns** to {}, {t}, in the {} age of the curse. It seeks {}.", self.land_name(lid), ordinal(n), self.items[self.cycles[cy].relic].name);
        text += &if h != NONE { format!(" {} bears {}.", self.who(h), HERO_MARK[0]) } else { " No one bearing the Hero's Mark has been born.".into() };
        if s != NONE { text += &format!(" {} carries {}.", self.who(s), SEER_GIFT[0]); }
        self.say(text, false);
        let ln = self.land_name(lid);
        self.myth(format!("The {} return of {v}", ordinal(n)), format!("In year {} {v} came back to {ln}.", self.year));
    }

    fn hero_strikes(&mut self, cy: usize) {
        let y = self.year;
        let (h, s) = (self.cycles[cy].hero, self.cycles[cy].seer);
        if h == NONE || (!self.chars[h].adult(y) && self.chars[h].age(y) < 14) { return; }
        if !self.chars[h].hero { self.start_hero(h, Some(format!("takes up the road, {} burning on their hand", HERO_MARK[0]))); }
        let blade = self.cycles[cy].blade;
        if blade == NONE { return; }
        if !self.chars[h].items.contains(&blade) {
            if self.rng.random() > 0.35 { return; }
            let where_ = self.take_blade(blade);
            self.give_item(h, blade, true);
            let t = format!("{} draws {}{where_}. It knows its bearer.", self.who(h), self.items[blade].name);
            self.say(t, false);
            return;
        }
        let foe = if self.cycles[cy].state == "returned" { self.monsters[self.cycles[cy].monster].power } else { self.cycles[cy].ruler_power };
        let seer_up = s != NONE && self.chars[s].alive && self.chars[s].age(y) >= 14;
        let hp = self.cr(&self.chars[h], true) * 3.0 * if seer_up { 1.6 } else { 1.0 };
        if self.rng.random() > 0.4 { return; }
        if self.rng.random() < hp / (hp + 0.6 * foe) { self.seal(cy, h, s); }
        else {
            self.chars[h].hp -= self.rng.uniform(40.0, 120.0);
            if self.chars[h].hp <= 0.0 {
                let v = self.cycles[cy].villain.clone();
                let t = format!("{} falls before {v}. {} fades from their hand; it will be born again.", self.who(h), HERO_MARK[0]);
                self.say(t, false);
                self.chars[h].items.retain(|&i| i != blade);
                let title = self.cycles[cy].title;
                self.kill(h, format!("fell before {v}, {title}"));
                let keep = self.keeper_town(self.cycles[cy].landmass);
                if keep != NONE { let f = self.towns[keep].realm; self.realms[f].treasury.push(blade); }
            } else { self.chars[h].hp = self.chars[h].hp.max(10.0); }
        }
    }

    fn take_blade(&mut self, blade: usize) -> String {
        for k in 0..self.realms.len() {
            if self.realms[k].treasury.contains(&blade) { self.realms[k].treasury.retain(|&x| x != blade); return format!(" from the treasury of {}", self.realms[k].name); }
            if self.realms[k].market.contains(&blade) { self.realms[k].market.retain(|&x| x != blade); return format!(" from the market of {}", self.realms[k].name); }
        }
        for c in self.living.clone() { if self.chars[c].alive && self.chars[c].items.contains(&blade) { self.chars[c].items.retain(|&x| x != blade); return format!(", given up by {}", self.chars[c].name); } }
        for r in 0..self.ruins.len() { if self.ruins[r].loot.contains(&blade) { self.ruins[r].loot.retain(|&x| x != blade); return format!(" from {}", self.ruins[r].name); } }
        for m in 0..self.monsters.len() { if self.monsters[m].items.contains(&blade) { self.monsters[m].items.retain(|&x| x != blade); return format!(" out of the hoard of {}", self.monsters[m].name); } }
        String::new()
    }

    fn seal(&mut self, cy: usize, h: usize, s: usize) {
        let y = self.year;
        let was = self.cycles[cy].state;
        if was == "returned" { let m = self.cycles[cy].monster; if m != NONE { self.monsters[m].sealed = true; self.monsters[m].alive = false; } }
        else if was == "triumphant" { self.restore(cy, s); }
        let period = self.cycles[cy].period;
        self.cycles[cy].state = "sealed";
        self.cycles[cy].next_return = y + period;
        let deed = if was == "returned" { "seals" } else { "casts down" };
        let seer_alive = s != NONE && self.chars[s].alive;
        let (v, bn) = (self.cycles[cy].villain.clone(), self.items[self.cycles[cy].blade].name.clone());
        let mut text = format!("**{} {deed} {v}** with {bn}", self.who(h));
        if seer_alive { text += &format!(", {}'s {} holding it still", self.who(s), SEER_GIFT[0]); }
        text += &format!(". The {} age of the curse is over; the sages say it will return in year {}.", ordinal(self.cycles[cy].ages.len() + 1), y + period);
        self.say(text, false);
        self.gain_fame(h, "concord", 400.0);
        self.chars[h].renown += 800.0;
        self.deed(h, "calamity");
        if seer_alive { self.chars[s].renown += 300.0; if let Some(l) = self.chars[s].legitimacy { self.chars[s].legitimacy = Some((l + 20.0).min(100.0)); } }
        let hn = self.chars[h].name.clone();
        self.cycles[cy].ages.push(format!("ended {y}: sealed by {hn}"));
        let n = self.cycles[cy].ages.len();
        self.myth(format!("The sealing of {v}, age {n}"), format!("{hn} {deed} {v} with {bn} in year {y}."));
        self.cycles[cy].hero = NONE;
        self.cycles[cy].seer = NONE;
    }

    fn fall(&mut self, cy: usize, town: usize) {
        let y = self.year;
        let f = self.towns[town].realm;
        let m = self.cycles[cy].monster;
        self.monsters[m].alive = false;
        self.monsters[m].sealed = true;
        let relic = self.cycles[cy].relic;
        self.realms[f].treasury.retain(|&x| x != relic);
        let villain = self.make_character(f, y - 300, None, NONE, NONE, NONE);
        let vname = self.cycles[cy].villain.split(' ').next().unwrap().to_string();
        let ptraits: Vec<usize> = ["cruel", "ambitious", "wrathful"].iter().map(|t| PERSONALITY.iter().position(|p| p == t).unwrap()).collect();
        let vc = &mut self.chars[villain];
        vc.name = vname; vc.undead = true; vc.tyranny = 100.0; vc.personality = ptraits; vc.items.push(relic);
        let old = self.realms[f].ruler;
        let v = self.cycles[cy].villain.clone();
        if old != NONE && self.chars[old].alive { self.kill_by(old, format!("was cast down by {v}"), true, NONE); }
        if self.realms[f].ruler != NONE { self.end_reign(f, &format!("cast down by {v}")); }
        let old_name = self.realms[f].name.clone();
        self.cycles[cy].old_name = old_name.clone();
        self.realms[f].name = format!("the Dominion of {v}");
        self.crown(f, villain, "seizing the throne from the cradle");
        self.chars[villain].legitimacy = Some(5.0);
        self.cycles[cy].ruler = villain;
        self.cycles[cy].ruler_power = self.monsters[m].power;
        self.cycles[cy].state = "triumphant";
        self.cycles[cy].dark_center = self.towns[town].pos;
        self.cycles[cy].dark_radius = 4.0;
        let (rn, tn, kn) = (self.items[relic].name.clone(), self.towns[town].name.clone(), self.realms[f].name.clone());
        self.say(format!("**The Fall.** {v} takes {rn} at {tn} and sits the throne of {old_name}. It is {kn} now, and the land around it begins to die."), false);
        self.myth(format!("The Fall of {old_name}"), format!("In year {y} {v} took {rn} and the throne."));
    }

    fn spread_dark(&mut self, cy: usize) {
        self.cycles[cy].dark_radius = (self.cycles[cy].dark_radius + 0.7).min(40.0);
        let (c0, rad) = (self.cycles[cy].dark_center, self.cycles[cy].dark_radius);
        let r = rad as i64;
        for y in (c0.0 as i64 - r).max(0)..(c0.0 as i64 + r + 1).min(self.map.h as i64) {
            for x in (c0.1 as i64 - r).max(0)..(c0.1 as i64 + r + 1).min(self.map.w as i64) {
                let (yy, xx) = (y as usize, x as usize);
                if mdist((yy, xx), c0) > rad || self.map.terrain.at(yy, xx) == OCEAN { continue; }
                if self.rng.random() < 0.08 { self.corrupted.set(yy, xx, true); }
                if self.corrupted.at(yy, xx) && self.rng.random() < 0.02 { let t = if self.rng.random() < 0.5 { SWAMP } else { DESERT }; self.map.terrain.set(yy, xx, t); }
            }
        }
        for t in self.towns.iter_mut() { if t.alive && mdist(t.pos, c0) <= rad { t.curse_mult = t.curse_mult.min(0.5); t.pop *= 0.985; } }
        if self.rng.random() < 0.1 {
            let mut kind = ["wraith", "demon", "troll"][self.rng.integers(0, 3) as usize];
            if kind == "demon" && self.monsters.iter().filter(|m| m.alive && m.kind == "demon" && !m.gate).count() >= 2 { kind = "wraith"; }
            let pr = MONSTER.iter().find(|x| x.0 == kind).unwrap().2;
            let p = self.rng.uniform(pr.0, pr.1) * 1.5;
            self.new_monster(kind, c0, p, None, &[], false, false);
        }
    }

    fn restore(&mut self, cy: usize, seer: usize) {
        let ruler = self.cycles[cy].ruler;
        let k = (0..self.realms.len()).find(|&k| self.realms[k].ruler == ruler && ruler != NONE);
        if ruler != NONE && self.chars[ruler].alive { self.kill_by(ruler, format!("was cast down by {}", HERO_MARK[0]), true, NONE); }
        let Some(k) = k else { return };
        self.end_reign(k, "cast down by the hero of the age");
        self.realms[k].name = self.cycles[cy].old_name.clone();
        let heir = if seer != NONE && self.chars[seer].alive && self.chars[seer].realm == k { seer } else {
            let cands = self.pool_of(k);
            let mut best = (f64::MIN, NONE);
            for c in cands { let s = self.claim_of(c); if s > best.0 { best = (s, c); } }
            best.1
        };
        if heir != NONE { self.crown(k, heir, "restored when the dark was cast down"); }
        self.cycles[cy].ruler = NONE;
        let t = format!("{} is free again.", self.realms[k].name);
        self.say(t, false);
    }
}
