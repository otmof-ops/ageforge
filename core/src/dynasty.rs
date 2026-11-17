//! The year at court (dynasty.py): deaths and births, marriages and alliances, regencies, plots, and succession
//! by each realm's law, with epithets for the reigns that end and wars for contested thrones.
//! (Heroes join the pools with slice 6; partition of tribal lands with titles, slice 4.)

use crate::people::*;
use crate::state::State;
use crate::tables::*;

impl State {
    /// A person dies (world.kill): legacy passes to the children, the realm hears of a ruler's death.
    pub fn kill(&mut self, cid: usize, cause: String) {
        self.kill_by(cid, cause, false, NONE);
    }

    pub fn kill_by(&mut self, cid: usize, cause: String, quiet: bool, by: usize) {
        if !self.chars[cid].alive { return; }
        let y = self.year;
        let c = &mut self.chars[cid];
        c.alive = false;
        c.died = Some(y);
        c.cause = Some(cause.clone());
        // deeds done after the children were born still reach them (lineage.pass_legacy_on_death)
        let legacy = self.chars[cid].legacy.clone();
        for kid in self.chars[cid].children.clone() {
            if !self.chars[kid].alive { continue; }
            for &(kind, pts) in &legacy {
                let kl = &mut self.chars[kid].legacy;
                match kl.iter_mut().find(|x| x.0 == kind) { Some(x) => if pts * 0.5 > x.1 { x.1 = pts * 0.5 }, None => kl.push((kind, pts * 0.5)) }
            }
        }
        self.heart_on_death(cid, by);
        if self.chars[cid].hero { let p = self.chars[cid].people; self.hero_guilds[p].deaths_total += 1; }
        let kh = if by != NONE { self.chars[by].house } else { NONE };
        self.legends_on_death(cid, &cause, kh);
        let heir = self.heir_of(cid);
        self.pass_beast(cid, heir);
        self.inherit(cid, None);
        let k = self.chars[cid].realm;
        if k < self.realms.len() && self.realms[k].ruler == cid && !self.chars[cid].hero && !quiet {
            let text = format!("{} {}, aged {}.", self.who(cid), cause, self.chars[cid].age(y));
            self.say(text, false);
        }
    }

    /// A year's chance of death, calibrated on the medieval elite and scaled to a lifespan.
    fn mortality(&self, c: &Character, a: i64) -> f64 {
        let h = a as f64 * 75.0 / c.life.max(1.0);
        let mut q = if h < 5.0 { 0.03 } else if h < 15.0 { 0.008 } else { 0.004 + 0.0025 * (0.085 * (h - 30.0)).exp() };
        q *= 1.0 + (5.0 - c.health).max(0.0) / 3.0;
        let k = &self.realms[c.realm.min(self.realms.len() - 1)];
        if k.physician && c.house != NONE && c.house == k.house { q *= 0.7; }
        q
    }

    pub fn dynasty_year(&mut self) {
        let y = self.year;
        let t_mort = std::time::Instant::now();
        let living: Vec<usize> = self.living.iter().copied().filter(|&i| self.chars[i].alive).collect();
        for cid in living {
            let c = &self.chars[cid];
            let a = c.age(y);
            if !c.latent.is_empty() && c.adult(y) && a < 2 * ADULT[c.people] as i64 + 20 && self.rng.random() < 0.004 {
                self.awaken(cid, "in the prime of life");
            }
            let c = &self.chars[cid];
            if c.undead { continue; }
            if a as f64 > c.life * 1.3 {
                self.kill(cid, "died of old age".into());
            } else if self.rng.random() < self.mortality(c, a) {
                let frac = a as f64 / c.life.max(1.0);
                let why = if a < 1 { "died in infancy" } else if !c.adult(y) { "died in childhood" } else if frac < 0.75 { "died of illness" } else { "died of old age" };
                self.kill(cid, why.into());
            }
        }
        self.sub_tick("dynasty: deaths", t_mort);
        let t = std::time::Instant::now(); self.births(); self.sub_tick("dynasty: births", t);
        let (mut t_succ, mut t_plot, mut t_marry, mut t_court) = (0.0, 0.0, 0.0, 0.0);
        for k in 0..self.realms.len() {
            if self.realms[k].extinct { continue; }
            let t = std::time::Instant::now();
            let r = self.realms[k].ruler;
            if r == NONE || !self.chars[r].alive {
                let how = if r != NONE { self.chars[r].cause.clone().unwrap_or_default() } else { "the throne lay empty".into() };
                self.succeed(k, &how);
                if self.realms[k].extinct || self.realms[k].ruler == NONE { continue; }
            }
            t_succ += t.elapsed().as_secs_f64();
            let t = std::time::Instant::now();
            self.regency(k);
            self.plots(k);
            t_plot += t.elapsed().as_secs_f64();
            let r = self.realms[k].ruler;
            if r != NONE && !self.chars[r].alive {
                let how = self.chars[r].cause.clone().unwrap_or_default();
                self.succeed(k, &how);
                if self.realms[k].extinct || self.realms[k].ruler == NONE { continue; }
            }
            let t = std::time::Instant::now();
            self.marriages(k);
            t_marry += t.elapsed().as_secs_f64();
            let t = std::time::Instant::now();
            for c in self.court(k) {
                if !self.chars[c].hero {
                    let xp = if self.chars[c].age(y) < ADULT[self.realms[k].people] as i64 + 20 { 40.0 } else { 8.0 };
                    self.gain_xp(c, xp);
                }
            }
            t_court += t.elapsed().as_secs_f64();
            let h = self.realms[k].house;
            if h != NONE { self.houses[h].prestige += 1.0; self.houses[h].reign_years += 1; }
        }
        for (n, v) in [("dynasty: succession", t_succ), ("dynasty: regency and plots", t_plot), ("dynasty: marriages", t_marry), ("dynasty: court xp", t_court)] {
            match self.phase_times.iter_mut().find(|x| x.0 == n) { Some(x) => x.1 += v, None => self.phase_times.push((n, v)) }
        }
        let t = std::time::Instant::now(); self.councils(); self.council_effects(); self.sub_tick("dynasty: councils", t);
    }

    /// A power gene carried unseen wakes (lineage.awaken).
    pub fn awaken(&mut self, cid: usize, why: &str) {
        let mut lat = self.chars[cid].latent.clone();
        if lat.is_empty() { return; }
        lat.sort_by_key(|&g| GENETIC[g]);
        let g = lat[self.rng.integers(0, lat.len() as i64) as usize];
        let fx = GENETIC_FX[g];
        let c = &mut self.chars[cid];
        c.latent.retain(|&x| x != g);
        c.genetic.push(g);
        for i in 0..6 { c.attrs[i] += fx[i]; }
        c.prowess += fx[6];
        c.talent = (c.talent + fx[7]).min(10.0);
        c.life *= fx[8];
        let what = POWER_GENES.iter().find(|p| p.0 == GENETIC[g]).map(|p| p.1).unwrap_or("");
        let text = format!("**The {} wakes in {}** {why}. It {what}.", GENETIC[g].replace("blood", " blood"), self.who(cid));
        self.say(text, false);
    }

    fn births(&mut self) {
        let y = self.year;
        let mothers: Vec<usize> = self.living.iter().copied().filter(|&i| {
            let c = &self.chars[i];
            c.alive && c.female && c.spouse != NONE && !c.undead
        }).collect();
        for m in mothers {
            let dad = self.chars[m].spouse;
            if !self.chars[dad].alive { continue; }
            let (mum, d) = (&self.chars[m], &self.chars[dad]);
            let a = mum.age(y);
            let (adult, end) = (ADULT[mum.people] as i64, FERTILE_END[mum.people] as i64);
            if a < adult || a > end { continue; }
            let mut fec = if mum.has(gene("fecund")) || d.has(gene("fecund")) { 1.6 } else { 1.0 };
            if d.house == NONE && mum.house == NONE {
                if mum.children.len() >= 5 { continue; }
                fec *= 0.85;
            }
            let older = if a as f64 > adult as f64 + (end - adult) as f64 * 0.7 { 0.5 } else { 1.0 };
            let p = FERTILITY[mum.people] * fec * older * 0.9f64.powi(mum.children.len() as i32);
            if self.rng.random() >= p { continue; }
            let hs: Vec<(f64, usize)> = [dad, m].iter().filter(|&&p| self.chars[p].house != NONE)
                .map(|&p| (self.houses[self.chars[p].house].prestige, self.chars[p].house)).collect();
            let house = hs.iter().max_by(|a, b| a.partial_cmp(b).unwrap()).map(|x| x.1).unwrap_or(NONE);
            let pid = if house != NONE && self.chars[dad].house == house { self.chars[dad].realm } else { self.chars[m].realm };
            let kid = self.make_character(pid, y, None, dad, m, house);
            self.heart_on_birth(kid);
            self.born_bender(kid);
            self.legends_on_birth(kid);
            if !self.chars[m].undead && self.rng.random() < 0.012 { self.kill(m, "died in childbirth".into()); }
            if self.rng.random() < 0.16 { self.kill(kid, "died in infancy".into()); }
            if pid < self.realms.len() {
                let r = self.realms[pid].ruler;
                if r != NONE && (dad == r || m == r) {
                    let rare: Vec<&str> = self.chars[kid].genetic.iter().map(|&g| GENETIC[g])
                        .filter(|g| ["dragonblood", "starblood", "genius", "spellborn"].contains(g)).collect();
                    if !rare.is_empty() {
                        let text = format!("A child, {}, is born to {}; the midwives speak of {}.", self.chars[kid].name, self.who(r), rare.join(", "));
                        self.say(text, false);
                    }
                }
            }
        }
    }

    /// The ruler and those close to the throne: the ones the crown marries off.
    pub fn court(&self, k: usize) -> Vec<usize> {
        let r = self.realms[k].ruler;
        if r == NONE { return vec![]; }
        let rc = &self.chars[r];
        let mut ids = vec![r];
        ids.extend(&rc.children);
        for &kid in &rc.children { ids.extend(&self.chars[kid].children); }
        for p in [rc.father, rc.mother] { if p != NONE { ids.extend(&self.chars[p].children); } }
        ids.sort();
        ids.dedup();
        ids.into_iter().filter(|&i| self.chars[i].alive).collect()
    }

    pub fn adults_of(&self, k: usize, house: usize) -> Vec<usize> {
        let y = self.year;
        self.living.iter().map(|&i| &self.chars[i]).filter(|c| c.alive && c.realm == k && c.adult(y) && (house == NONE || c.house == house)).map(|c| c.id).collect()
    }

    fn find_match(&mut self, c: usize, pool: &[usize]) -> usize {
        let y = self.year;
        let mut best = NONE;
        let mut best_r = f64::MIN;
        for &o in pool {
            let (oc, cc) = (&self.chars[o], &self.chars[c]);
            if oc.alive && oc.spouse == NONE && oc.female != cc.female && oc.adult(y) && !oc.undead && o != c
                && oc.age(y) <= FERTILE_END[oc.people] as i64 && oc.house != cc.house {
                let score = oc.renown + self.rng.random();
                if best == NONE || score > best_r { best = o; best_r = self.chars[o].renown.max(score); }
            }
        }
        best
    }

    fn marry(&mut self, a: usize, b: usize) {
        self.chars[a].spouse = b;
        self.chars[b].spouse = a;
        self.heart_on_marriage(a, b);
    }

    fn marriages(&mut self, k: usize) {
        let y = self.year;
        for c in self.court(k) {
            let cc = &self.chars[c];
            if cc.spouse != NONE || !cc.adult(y) || cc.undead || cc.age(y) > FERTILE_END[cc.people] as i64 { continue; }
            if self.rng.random() > 0.45 { continue; }
            let r = self.rng.random();
            if r < 0.15 {
                let others: Vec<usize> = (0..self.realms.len()).filter(|&o| o != k && !self.realms[o].extinct && self.relation[k][o] > -30.0).collect();
                if !others.is_empty() {
                    let o = others[self.rng.integers(0, others.len() as i64) as usize];
                    let pool = self.court(o);
                    let mate = self.find_match(c, &pool);
                    if mate != NONE {
                        self.marry(c, mate);
                        let text = format!("**Alliance.** {} of {} weds {} of {}.", self.who(c), self.realms[k].name, self.who(mate), self.realms[o].name);
                        self.say(text, false);
                        self.relation[k][o] += 25.0;
                        self.relation[o][k] += 25.0;
                        continue;
                    }
                }
            }
            // (r < 0.40: a renowned hero of the realm, with slice 6)
            let house = self.realms[k].house;
            let nobles: Vec<usize> = self.adults_of(k, NONE).into_iter()
                .filter(|&o| self.chars[o].house != NONE && self.chars[o].house != house && !self.chars[o].hero).collect();
            let mut mate = self.find_match(c, &nobles);
            if mate == NONE {
                let born = self.chars[c].born + self.rng.integers(-5, 6);
                let female = !self.chars[c].female;
                mate = self.make_character(k, born, Some(female), NONE, NONE, NONE);
                self.new_house(mate, None);
                self.gain_xp(mate, 500.0);
            }
            self.marry(c, mate);
        }
    }

    fn regency(&mut self, k: usize) {
        let y = self.year;
        let r = self.realms[k].ruler;
        if self.chars[r].adult(y) {
            if self.realms[k].regent != NONE {
                let text = format!("{} comes of age and takes the rule of {} into their own hands.", self.who(r), self.realms[k].name);
                self.say(text, false);
                self.realms[k].regent = NONE;
            }
            return;
        }
        let mut reg = self.realms[k].regent;
        if reg == NONE || !self.chars[reg].alive {
            let cands: Vec<usize> = self.adults_of(k, NONE).into_iter().filter(|&c| c != r).collect();
            if cands.is_empty() { return; }
            let house = self.realms[k].house;
            reg = *cands.iter().max_by(|&&a, &&b| {
                let s = |c: usize| self.chars[c].attr(2, y) + self.chars[c].attr(0, y) + if self.chars[c].house == house { 5.0 } else { 0.0 };
                s(a).partial_cmp(&s(b)).unwrap()
            }).unwrap();
            self.realms[k].regent = reg;
            let text = format!("{} is a child; {} rules {} as regent.", self.who(r), self.who(reg), self.realms[k].name);
            self.say(text, false);
        }
        let rc = &self.chars[reg];
        if rc.has_trait("ambitious") && (rc.has_trait("cruel") || rc.has_trait("deceitful")) && self.rng.random() < 0.04 {
            let text = format!("The child-ruler {} dies in the night. No one asks the regent, {}, any questions.", self.who(r), self.who(reg));
            self.say(text, false);
            self.kill_by(r, "died in mysterious circumstances".into(), false, reg);
            self.end_reign(k, "died in mysterious circumstances");
            self.realms[k].regent = NONE;
            self.crown(k, reg, "seizing the throne from the cradle");
        }
    }

    fn plots(&mut self, k: usize) {
        let y = self.year;
        let r = self.realms[k].ruler;
        let spy = self.realms[k].council[2];
        let mut guard = if spy != NONE { self.chars[spy].attr(3, y) } else { 0.0 } + if self.chars[r].has_trait("paranoid") { 3.0 } else { 0.0 };
        if self.chars[r].undead { guard += 10.0; }
        let (house, regent) = (self.realms[k].house, self.realms[k].regent);
        let plotters: Vec<usize> = self.adults_of(k, NONE).into_iter().filter(|&c| {
            let cc = &self.chars[c];
            let vend = cc.vendettas.iter().any(|v| v.0 == r);
            c != r && ((cc.has_trait("ambitious") && (cc.has_trait("deceitful") || cc.has_trait("cruel"))) || vend)
                && (cc.house == house || cc.renown > 150.0 || c == regent)
        }).filter(|&c| self.chars[c].vendettas.iter().any(|v| v.0 == r) || self.opinion(c, r) < -20.0).collect();
        for p in plotters {
            let chance = (0.001 * (self.chars[p].attr(3, y) - 0.6 * guard - 4.0).max(0.0)).min(0.01);
            if self.rng.random() < chance {
                let text = format!("{} is murdered. The court whispers the name of {}.", self.who(r), self.who(p));
                self.say(text, false);
                self.remember_kin(r, p, "murdered my kin", -80.0);
                self.kill_by(r, "was murdered".into(), true, p);
                return;
            }
            if spy != NONE && self.rng.random() < 0.002 * self.chars[spy].attr(3, y) {
                let text = format!("{}, spymaster of {}, uncovers a plot; {} is put to death.", self.who(spy), self.realms[k].name, self.who(p));
                self.say(text, false);
                let rc = &self.chars[r];
                let spare = if rc.has_trait("cruel") { 0.1 } else if rc.has_trait("compassionate") || rc.has_trait("forgiving") { 0.85 } else { 0.45 };
                if self.rng.random() < spare {
                    self.gain_hook(r, p, "strong", ", sparing a plotter's life in exchange for their loyalty");
                    continue;
                }
                self.remember_kin(p, r, "executed my kin", -60.0);
                self.reluct(r, "execute", "an execution");
                let text = format!("was executed for plotting against {} {}", self.chars[r].name, self.chars[r].regnal.clone().unwrap_or_default());
                self.kill_by(p, text, true, r);
                self.chars[r].tyranny = (self.chars[r].tyranny + 5.0).min(100.0);
            }
        }
        // (assassins sent by realms at war join with warfare, slice 4)
    }

    pub fn claim_of(&mut self, c: usize) -> f64 { self.claim_score(c) }
    pub fn civil_war_of(&mut self, k: usize, a: usize, b: usize) -> usize { self.civil_war(k, a, b) }
    pub fn pool_of(&self, k: usize) -> Vec<usize> { self.pool(k) }

    fn claim_score(&mut self, c: usize) -> f64 {
        let y = self.year;
        let cc = &self.chars[c];
        let hp = if cc.house != NONE { self.houses[cc.house].prestige / 100.0 } else { 0.0 };
        let blood = if self.realms[cc.realm].house != NONE && cc.house == self.realms[cc.realm].house { 8.0 } else { 0.0 };
        cc.attr(4, y) + cc.attr(0, y) + 0.5 * cc.attr(1, y) + cc.level as f64 / 6.0 + (cc.renown / 80.0).min(10.0) + hp + blood
            + self.rng.normal(0.0, 3.0)
    }

    fn might_score(&mut self, c: usize) -> f64 {
        let cc = &self.chars[c];
        self.cr(cc, false) / 40.0 + cc.attr(1, self.year) + cc.prowess / 2.0 + self.rng.normal(0.0, 4.0)
    }

    fn pool(&self, k: usize) -> Vec<usize> {
        let house = self.realms[k].house;
        let mut out: Vec<usize> = if house != NONE { self.adults_of(k, house) } else { vec![] };
        let mut nobles: Vec<usize> = self.adults_of(k, NONE).into_iter()
            .filter(|&c| self.chars[c].house != NONE && self.chars[c].house != house && !self.chars[c].hero).collect();
        nobles.sort_by(|&a, &b| self.houses[self.chars[b].house].prestige.partial_cmp(&self.houses[self.chars[a].house].prestige).unwrap());
        for c in nobles.into_iter().take(4) { if !out.contains(&c) { out.push(c); } }
        out
    }

    fn primo_heir(&self, old: usize) -> usize {
        let f = self.chars[old].people;
        fn line(st: &State, c: usize, skip: usize, f: usize) -> usize {
            let mut kids = st.chars[c].children.clone();
            kids.sort_by_key(|&k| st.chars[k].born);
            for kid in kids {
                if kid == skip { continue; }
                if st.chars[kid].alive && st.chars[kid].people == f { return kid; }
                let got = line(st, kid, skip, f);
                if got != NONE { return got; }
            }
            NONE
        }
        let mut heir = line(self, old, NONE, f);
        let (mut up, mut skip) = (old, old);
        for _ in 0..4 {
            if heir != NONE { break; }
            let mut parent = self.chars[up].father;
            if parent == NONE || (self.chars[old].house != NONE && self.chars[parent].house != self.chars[old].house) {
                parent = self.chars[up].mother;
            }
            if parent == NONE { break; }
            heir = line(self, parent, skip, f);
            up = parent;
            skip = parent;
        }
        heir
    }

    pub fn end_reign(&mut self, k: usize, how: &str) {
        let y = self.year;
        let c = self.realms[k].ruler;
        if c == NONE { return; }
        let start = self.realms[k].reigns.last().map(|r| r.start).unwrap_or(y);
        let yrs = y - start;
        if let Some((rk, s, _)) = self.chars[c].ruled { self.chars[c].ruled = Some((rk, s, Some(y))); }
        let cc = &self.chars[c];
        if cc.epithet.is_none() {
            let (taken, lost) = (self.realms[k].taken, self.realms[k].lost);
            let p = |t: &str| cc.has_trait(t);
            let ep = if cc.undead { Some("the Undying") } else if cc.tyranny >= 60.0 { Some("the Tyrant") }
                else if cc.legitimacy.is_some_and(|l| l < 25.0) && yrs > 3 { Some("the Corrupt") }
                else if yrs <= 1 { Some("the Brief") }
                else if taken >= 5 && taken > lost { Some("the Conqueror") }
                else if lost >= 6 { Some("the Ruinous") }
                else if how == "murdered" && yrs < 6 { Some("the Unlucky") }
                else if p("cruel") && yrs > 10 { Some("the Cruel") }
                else if cc.renown >= 400.0 { Some("the Great") }
                else if yrs as f64 >= LIFESPAN[self.realms[k].people] * 0.5 { Some("the Long-Reigning") }
                else if p("kind") && yrs > 15 { Some("the Good") }
                else if cc.attr(4, y) >= 14.0 { Some("the Wise") }
                else if p("wrathful") { Some("the Wrathful") }
                else if p("paranoid") { Some("the Suspicious") }
                else if p("greedy") { Some("the Grasping") }
                else if p("generous") { Some("the Bountiful") }
                else { None };
            self.chars[c].epithet = ep.map(String::from);
        }
        let (ep, leg, tyr) = (self.chars[c].epithet.clone(), self.chars[c].legitimacy, self.chars[c].tyranny);
        if let Some(r) = self.realms[k].reigns.last_mut() {
            r.end = Some(y);
            r.how = Some(how.to_string());
            r.epithet = ep;
            r.legit = leg;
            r.tyranny = tyr;
        }
        self.realms[k].ruler = NONE;
    }

    pub fn succeed(&mut self, k: usize, how: &str) {
        let old = self.realms[k].ruler;
        if old != NONE { self.end_reign(k, how); }
        let (mut heir, mut rival, mut reason) = (NONE, NONE, String::new());
        let law = self.realms[k].law;
        let house = self.realms[k].house;
        if law == "primogeniture" && old != NONE {
            heir = self.primo_heir(old);
            reason = "by right of birth".into();
        } else if law == "seniority" && house != NONE {
            let elders = self.adults_of(k, house);
            if let Some(&e) = elders.iter().min_by_key(|&&c| self.chars[c].born) {
                heir = e;
                reason = format!("as eldest of {}", self.houses[house].name);
            }
        } else if law == "strongest" {
            let cands = self.pool(k);
            if !cands.is_empty() {
                let mut scored: Vec<(f64, usize)> = cands.iter().map(|&c| (self.might_score(c), c)).collect();
                scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap());
                heir = scored[0].1;
                reason = "by strength of arm".into();
                if scored.len() > 1 && scored[1].0 > 0.85 * scored[0].0 && self.chars[scored[1].1].has_trait("ambitious") { rival = scored[1].1; }
            }
        }
        if heir == NONE {
            let cands = self.pool(k);
            if !cands.is_empty() {
                let mut best = (f64::MIN, NONE);
                for c in cands { let s = self.claim_score(c); if s > best.0 { best = (s, c); } }
                heir = best.1;
                reason = if law == "elective" { "chosen by the great of the realm".into() } else { "raised up when the line failed".into() };
            }
        }
        if heir == NONE {
            let born = self.year - ADULT[self.realms[k].people] as i64 - self.rng.integers(0, 20);
            heir = self.make_character(k, born, None, NONE, NONE, NONE);
            self.gain_xp(heir, 800.0);
            reason = "a nobody raised up out of the ruin of the realm".into();
        }
        if rival == NONE {
            let others: Vec<usize> = self.pool(k).into_iter().filter(|&c| c != heir && self.chars[c].has_trait("ambitious")
                && (self.chars[c].renown >= 300.0 || self.chars[c].house == house)).collect();
            if !others.is_empty() && self.rng.random() < 0.25 {
                let mut best = (f64::MIN, NONE);
                for c in others { let s = self.claim_score(c); if s > best.0 { best = (s, c); } }
                rival = best.1;
            }
        }
        if rival != NONE {
            heir = self.civil_war(k, heir, rival);
            reason = "after a war for the throne".into();
        }
        self.crown(k, heir, &reason);
        if old != NONE { self.partition(k, old, heir); }
    }

    fn civil_war(&mut self, k: usize, a: usize, b: usize) -> usize {
        let yrs = self.rng.integers(1, 7);
        let sa = (self.claim_score(a) + self.might_score(a)) * self.rng.uniform(0.6, 1.4);
        let sb = (self.claim_score(b) + self.might_score(b)) * self.rng.uniform(0.6, 1.4);
        let (win, lose) = if sa >= sb { (a, b) } else { (b, a) };
        for t in self.alive_towns_of(k) {
            let f = 1.0 - self.rng.uniform(0.05, 0.15);
            self.towns[t].pop *= f;
            self.towns[t].wealth *= 0.8;
        }
        self.realms[k].civil_wars += 1;
        let slain = self.rng.random() < 0.6;
        let fate = if slain { "is slain" } else { "flees into exile" };
        let text = format!("**Succession war.** {} and {} both claim {}. After {yrs} year{} of war {} holds the throne, and {} {fate}.",
                           self.who(a), self.who(b), self.realms[k].name, if yrs > 1 { "s" } else { "" }, self.who(win), self.chars[lose].name);
        self.say(text, false);
        if slain {
            let why = format!("was slain in the war for the throne of {}", self.realms[k].name);
            self.kill_by(lose, why, false, win);
        } else {
            self.chars[lose].home = NONE;   // (takes up the sword in exile as a hero, slice 6)
        }
        win
    }
}
