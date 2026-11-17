//! Guilds, crime and religion (society.py): the guilds of every people and the Quiet Hand, ranked members and
//! guildmasters; crime by town, thieves, heists, bandits, assassins' contracts and the watch; bounty hunting;
//! faiths spreading town by town, persecution, rulers turning, and prophets proclaiming new faiths.

use crate::people::*;
use crate::power::skill_ix;
use crate::state::State;
use crate::tables::*;

fn tenet(name: &str) -> &'static str { TENETS.iter().find(|t| t.0 == name).map(|t| t.0).unwrap() }

impl State {
    pub fn found_guilds(&mut self) {
        for f in 0..4 {
            let Some(k) = (0..self.realms.len()).find(|&k| self.realms[k].people == f) else { continue };
            let cap = self.capital_of(k);
            let place = if cap != NONE { self.towns[cap].name.clone() } else { PEOPLES[f].to_string() };
            for &(kind, pat) in GUILD_KINDS.iter() {
                let id = self.guilds_soc.len();
                self.guilds_soc.push(SocGuild { id, name: pat.replace("{}", &place), kind, people: f, members: vec![], master: NONE, gold: 0.0, masters: vec![] });
            }
        }
        let id = self.guilds_soc.len();
        self.guilds_soc.push(SocGuild { id, name: ASSASSINS[0].into(), kind: "assassins", people: NONE, members: vec![], master: NONE, gold: 0.0, masters: vec![] });
    }

    pub fn found_faiths(&mut self) {
        let base = [["sacred kingship", "charity"], ["ancestor worship", "pacifism"], ["ancestor worship", "wealth gospel"], ["holy war", "martyrdom"]];
        for f in 0..4 {
            let (name, deity) = BASE_FAITHS[f];
            let site = (0..self.realms.len()).find(|&k| self.realms[k].people == f).map(|k| self.capital_of(k)).unwrap_or(NONE);
            self.religions.push(Religion { id: f, name: name.into(), deity: deity.into(), tenets: base[f].iter().map(|t| tenet(t)).collect(),
                founder: NONE, founded: 0, people: f, parent: NONE, holy_site: site, peak: 0.0, followers: 0.0, dead: false });
        }
    }

    fn guild_kind_for(&self, c: usize) -> Option<&'static str> {
        let cc = &self.chars[c];
        match cc.profession {
            Some("smith") | Some("enchanter") => Some("smiths"), Some("merchant") => Some("merchants"),
            Some("thief") | Some("bandit") => Some("thieves"), Some("assassin") => Some("assassins"),
            _ if cc.hero => Some(if self.school(cc).is_some() && matches!(CLASSES[cc.classes[0].0].name, "Wizard" | "Necromancer" | "Druid" | "Priest" | "Shaman") { "mages" } else { "fighters" }),
            _ => None,
        }
    }

    fn guild_score(&self, c: usize, kind: &str) -> f64 {
        let cc = &self.chars[c];
        let base = match kind { "smiths" => self.skill_val(cc, skill_ix("Smithing")), "merchants" => self.skill_val(cc, skill_ix("Mercantile")),
                                "thieves" | "assassins" => self.skill_val(cc, skill_ix("Sneak")), _ => cc.level as f64 };
        base + cc.fame.iter().map(|f| f.1).sum::<f64>() / 50.0 + cc.renown / 100.0
    }

    pub fn guilds_year(&mut self) {
        for gi in 0..self.guilds_soc.len() {
            let alive: Vec<usize> = self.guilds_soc[gi].members.iter().copied().filter(|&m| self.chars[m].alive).collect();
            self.guilds_soc[gi].members = alive;
        }
        for i in 0..self.chars.len() {
            if !self.chars[i].alive { continue; }
            let Some(kind) = self.guild_kind_for(i) else { continue };
            let p = self.chars[i].people;
            if let Some(gi) = self.guilds_soc.iter().position(|g| g.kind == kind && (g.people == NONE || g.people == p)) {
                if !self.guilds_soc[gi].members.contains(&i) { self.guilds_soc[gi].members.push(i); self.chars[i].guilds.push(gi); }
            }
        }
        for gi in 0..self.guilds_soc.len() {
            if self.guilds_soc[gi].members.is_empty() { continue; }
            let kind = self.guilds_soc[gi].kind;
            let mut ranked: Vec<(f64, usize)> = self.guilds_soc[gi].members.iter().map(|&m| (self.guild_score(m, kind), m)).collect();
            ranked.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap());
            for &(sc, c) in &ranked { self.chars[c].guild_rank = Some(GUILD_RANKS[((sc / 22.0) as usize).min(4)]); }
            let top = ranked[0].1;
            if self.guilds_soc[gi].master != top {
                let old = self.guilds_soc[gi].master;
                let y = self.year;
                let g = &mut self.guilds_soc[gi];
                g.master = top;
                g.masters.push((y, top));
                let gname = g.name.clone();
                self.chars[top].titles.push(format!("Guildmaster of {gname} from year {y}"));
                let succ = if old != NONE { format!(", succeeding {}", self.chars[old].name) } else { String::new() };
                let t = format!("{} becomes Guildmaster of {gname}{succ}.", self.who(top));
                self.say(t, true);
            }
            for m in self.guilds_soc[gi].members.clone() { let due = self.chars[m].gold * 0.05; self.chars[m].gold -= due; self.guilds_soc[gi].gold += due; }
        }
    }

    pub fn faith_of_town(&self, t: usize) -> usize {
        self.towns[t].faith.iter().max_by(|a, b| a.1.partial_cmp(&b.1).unwrap()).map(|f| f.0).unwrap_or(self.realms[self.towns[t].realm].faith)
    }

    pub fn crime_year(&mut self) {
        let y = self.year;
        let mut gang = [0.0; 4];
        for g in &self.guilds_soc { if g.kind == "thieves" && g.people < 4 { gang[g.people] = 0.05 * g.members.len().min(4) as f64; } }
        for t in 0..self.towns.len() {
            if !self.towns[t].alive { continue; }
            let s = &self.towns[t];
            let k = &self.realms[s.realm];
            let guard = 0.1 * s.walls as f64 + if s.lord != NONE && self.chars[s.lord].legitimacy.unwrap_or(0.0) >= 50.0 { 0.15 } else { 0.0 };
            let poor = if s.wealth < 60.0 { 0.2 } else { 0.0 };
            let charity = if self.religions[self.faith_of_town(t)].has("charity") { 0.1 } else { 0.0 };
            let target = (0.08 + 0.4 * k.corruption + poor + gang[s.culture] - guard - charity + 0.6 * s.jobless).max(0.02);
            let s = &mut self.towns[t];
            s.crime = 0.8 * s.crime + 0.2 * target;
            s.wealth *= 1.0 - 0.03 * s.crime;
        }
        for i in 0..self.chars.len() {
            let c = &self.chars[i];
            if !c.alive || c.jailed_until > y || !matches!(c.profession, Some("thief") | Some("assassin") | Some("bandit")) { continue; }
            let mut home = c.home;
            if home == NONE || !self.towns[home].alive {
                let towns = self.alive_towns_of(c.realm);
                if towns.is_empty() { continue; }
                home = towns[self.rng.integers(0, towns.len() as i64) as usize];
                self.chars[i].home = home;
            }
            match self.chars[i].profession.unwrap() {
                "thief" => {
                    let steal = self.towns[home].wealth * 0.02 * (0.5 + self.skill_val(&self.chars[i], skill_ix("Sneak")) / 100.0);
                    self.towns[home].wealth -= steal;
                    self.chars[i].gold += steal;
                    self.chars[i].bounty += steal * 0.5;
                    self.use_skill(i, skill_ix("Sneak"), 1.0);
                    self.use_skill(i, skill_ix("Pickpocket"), 0.8);
                    if self.skill_val(&self.chars[i], skill_ix("Security")) >= 60.0 && self.rng.random() < 0.05 { self.heist(i); }
                    else { self.use_skill(i, skill_ix("Security"), 0.6); }
                }
                "bandit" => {
                    let take = self.towns[home].wealth * 0.03;
                    self.towns[home].wealth -= take;
                    self.towns[home].pop *= 0.995;
                    self.chars[i].gold += take;
                    self.chars[i].bounty += take;
                    self.use_skill(i, skill_ix("Blade"), 1.0);
                    self.use_skill(i, skill_ix("Archery"), 0.6);
                    self.gain_fame(i, "Sneak", take / 100.0);
                }
                _ => self.contract(i),
            }
            self.caught(i, home);
        }
    }

    /// How well a court keeps its own (society.court_guard).
    pub fn court_guard(&self, k: usize, tgt: usize) -> f64 {
        let g = match self.realms[k].rank { "chiefdom" => 15.0, "duchy" => 30.0, "kingdom" => 60.0, "empire" => 90.0, _ => 0.0 } + self.realms[k].security;
        if tgt != NONE && self.realms[k].ruler == tgt && !self.chars[tgt].adult(self.year) { g + 40.0 } else { g }
    }

    fn heist(&mut self, c: usize) {
        let k = self.chars[c].realm;
        let mut targets: Vec<(usize, bool)> = self.realms[k].treasury.iter().map(|&i| (i, true)).collect();
        targets.extend(self.realms[k].market.iter().take(5).map(|&i| (i, false)));
        if targets.is_empty() { return; }
        let guard = self.court_guard(k, NONE);
        let sec = self.skill_val(&self.chars[c], skill_ix("Security"));
        if self.rng.random() < guard / (guard + sec + 20.0) {
            self.chars[c].jailed_until = self.year + self.rng.integers(5, 15);
            let wh = if targets[0].1 { "treasury" } else { "market" };
            let t = format!("{} is taken in the {wh} of {} and thrown in irons.", self.who(c), self.realms[k].name);
            self.say(t, true);
            return;
        }
        let &(it, treasury) = targets.iter().max_by(|a, b| self.items[a.0].bonus.partial_cmp(&self.items[b.0].bonus).unwrap()).unwrap();
        if treasury { self.realms[k].treasury.retain(|&x| x != it); } else { self.realms[k].market.retain(|&x| x != it); }
        self.give_item(c, it, true);
        let b = self.items[it].bonus;
        self.chars[c].bounty += 10.0 + b * 40.0;
        self.gain_fame(c, "Security", 60.0 + 100.0 * b);
        let wh = format!("{} of {}", if treasury { "treasury" } else { "market" }, self.realms[k].name);
        let t = format!("**A heist.** {} lifts {} from the {wh}. The bounty on their head is now {} gold.", self.who(c), self.items[it].label(), crate::economy::thousands(self.chars[c].bounty));
        let minor = self.items[it].rarity < 4;
        self.say(t, minor);
    }

    /// Not every ruler at war will buy a murder.
    fn willing(&mut self, kid: usize) -> bool {
        let r = self.realms[kid].ruler;
        if r == NONE { return false; }
        let rc = &self.chars[r];
        if ["honest", "just", "compassionate"].iter().any(|t| rc.has_trait(t)) { return false; }
        ["deceitful", "cruel", "paranoid", "vengeful", "ambitious", "arbitrary"].iter().any(|t| rc.has_trait(t)) || self.rng.random() < 0.15
    }

    /// The Quiet Hand takes contracts from kings at war.
    fn contract(&mut self, c: usize) {
        if self.rng.random() > 0.06 { self.use_skill(c, skill_ix("Sneak"), 0.8); return; }
        let n = self.realms.len();
        let mut wars = vec![];
        for i in 0..n { for j in 0..n { if i != j && self.war[i][j] && self.willing(i) { wars.push((i, j)); } } }
        if wars.is_empty() { return; }
        let (client, tp) = wars[self.rng.integers(0, wars.len() as i64) as usize];
        let mut cands: Vec<usize> = self.chars.iter().filter(|h| h.alive && h.hero && h.realm == tp).map(|h| h.id).collect();
        let tr = self.realms[tp].ruler;
        if tr != NONE && self.chars[tr].alive { cands.push(tr); }
        if cands.is_empty() { return; }
        let tgt = cands[self.rng.integers(0, cands.len() as i64) as usize];
        let spy = self.realms[tp].council[2];
        let defence = self.cr(&self.chars[tgt], true) / 50.0 + if spy != NONE { self.chars[spy].attr(3, self.year) * 3.0 } else { 0.0 }
            + if self.chars[tgt].undead { 20.0 } else { 0.0 } + self.court_guard(tp, tgt);
        let cc = &self.chars[c];
        let attack = self.skill_val(cc, skill_ix("Sneak")) + self.skill_val(cc, skill_ix("Blade")) / 2.0 + self.skill_val(cc, skill_ix("Alchemy")) / 3.0;
        self.use_skill(c, skill_ix("Sneak"), 2.0);
        self.use_skill(c, skill_ix("Blade"), 1.0);
        if self.rng.random() < attack / (attack + defence + 60.0) {
            let rev = self.realms[tp].income * self.realms[tp].tax_now;
            let fee = ((if tgt != tr { 0.3 } else { 1.0 }) * rev).max(5.0);
            if self.realms[client].gold < fee { return; }
            self.realms[client].gold -= fee;
            self.books.push((client, "assassins", -fee, "a quiet payment".into(), NONE));
            self.chars[c].gold += fee;
            self.chars[c].bounty += fee;
            self.gain_fame(c, "assassin", if tgt == tr { 80.0 } else { 30.0 });
            let t = format!("{} is found dead, a black feather on the pillow: the work of the Quiet Hand, paid in {} gold.", self.who(tgt), self.pname(client));
            self.say(t, tgt != tr);
            let by = self.realms[client].ruler;
            self.kill_by(tgt, "was murdered by the Quiet Hand".into(), true, by);
            self.realms[tp].security += 40.0;
        } else if self.rng.random() < 0.3 {
            let t = format!("An assassin of the Quiet Hand, {}, is taken in the act and hanged.", self.who(c));
            self.say(t, true);
            self.kill(c, "was hanged after a failed contract".into());
        }
    }

    fn caught(&mut self, c: usize, home: usize) {
        if !self.chars[c].alive { return; }
        let s = &self.towns[home];
        let lord_ok = s.lord != NONE && self.chars[s.lord].legitimacy.unwrap_or(0.0) >= 50.0;
        let mut watch = 0.03 + 0.04 * s.walls as f64 / 5.0 + if lord_ok { 0.05 } else { 0.0 };
        watch *= (1.0 - self.skill_val(&self.chars[c], skill_ix("Sneak")) / 140.0).max(0.2);
        if self.rng.random() >= watch { return; }
        let r = self.realms[self.towns[home].realm].ruler;
        let bounty = self.chars[c].bounty;
        if (r != NONE && self.chars[r].has_trait("cruel")) || bounty > 1500.0 {
            let t = format!("{}, wanted for {} gold, is taken by the watch of {} and hanged.", self.who(c), crate::economy::thousands(bounty), self.towns[home].name);
            self.say(t, bounty < 1000.0);
            let why = format!("was hanged in {}", self.towns[home].name);
            self.kill(c, why);
            if r != NONE { self.add_tyranny(r, 0.5, ""); }
        } else {
            self.chars[c].jailed_until = self.year + self.rng.integers(3, 15);
            self.chars[c].bounty *= 0.3;
        }
    }

    /// A hero hunts the most wanted criminal of their people (society.hunt_bounty).
    pub fn hunt_bounty(&mut self, h: usize) -> f64 {
        let y = self.year;
        let realm = self.chars[h].realm;
        let target = self.chars.iter().filter(|c| c.alive && c.realm == realm && matches!(c.profession, Some("thief") | Some("assassin") | Some("bandit"))
            && c.bounty > 50.0 && c.jailed_until <= y).max_by(|a, b| a.bounty.partial_cmp(&b.bounty).unwrap()).map(|c| c.id);
        let Some(c) = target else { return 0.0 };
        let evade = self.skill_val(&self.chars[c], skill_ix("Sneak")) * 10.0 + self.chars[c].level as f64 * 20.0;
        self.use_skill(h, skill_ix("Sneak"), 0.5);
        let hcr = self.cr(&self.chars[h], true);
        if self.rng.random() < hcr / (hcr + evade) {
            let reward = self.chars[c].bounty;
            self.chars[h].gold += reward;
            let t = format!("{} brings in {} and collects a bounty of {} gold.", self.who(h), self.who(c), crate::economy::thousands(reward));
            self.say(t, reward < 1000.0);
            for it in std::mem::take(&mut self.chars[c].items) { self.give_item(h, it, true); }
            let why = format!("was brought to justice by {}", self.chars[h].name);
            self.kill(c, why);
            return 5.0 + reward / 100.0;
        }
        0.0
    }

    fn shift(&mut self, t: usize, rid: usize, amount: f64) {
        let f = &mut self.towns[t].faith;
        match f.iter_mut().find(|x| x.0 == rid) { Some(x) => x.1 += amount, None => f.push((rid, amount)) }
        let tot: f64 = f.iter().map(|x| x.1).sum();
        for x in f.iter_mut() { x.1 /= tot; }
        f.retain(|x| x.1 >= 0.005);
    }

    pub fn religion_year(&mut self) {
        let y = self.year;
        for i in 0..self.chars.len() {
            let c = &self.chars[i];
            if c.alive && c.adult(y) && (c.has_trait("zealous") || c.profession == Some("priest")) && self.skill_val(c, skill_ix("Speech")) >= 45.0
                && c.attr(5, y) >= 7.0 && self.rng.random() < 0.004 { self.proclaim(i); }
        }
        let towns: Vec<usize> = (0..self.towns.len()).filter(|&t| self.towns[t].alive).collect();
        let priests: Vec<usize> = self.chars.iter().filter(|c| c.alive && c.profession == Some("priest")).map(|c| c.id).collect();
        for p in priests {
            let home = if self.chars[p].home != NONE && self.towns[self.chars[p].home].alive { self.chars[p].home } else { self.capital_of(self.chars[p].realm) };
            if home == NONE || !self.towns[home].alive { continue; }
            let r = self.chars[p].religion;
            let mut power = 0.01 + self.skill_val(&self.chars[p], skill_ix("Speech")) / 2000.0;
            if self.religions[r].has("holy war") || self.religions[r].has("martyrdom") { power *= 1.2; }
            let hp = self.towns[home].pos;
            let near: Vec<usize> = towns.iter().copied().filter(|&t| { let q = self.towns[t].pos; (q.0 as f64 - hp.0 as f64).hypot(q.1 as f64 - hp.1 as f64) <= 10.0 }).collect();
            for t in near { self.shift(t, r, power); }
            self.use_skill(p, skill_ix("Speech"), 1.0);
            self.gain_fame(p, "prophet", power * 50.0);
        }
        for k in 0..self.realms.len() {
            if !self.active(k) { continue; }
            let r = self.realms[k].ruler;
            let rel = self.chars[r].religion;
            let persecutor = self.chars[r].has_trait("zealous") && self.chars[r].has_trait("cruel");
            let mut shares: Vec<(usize, f64)> = vec![];
            for t in self.alive_towns_of(k) {
                self.shift(t, rel, 0.004);
                if persecutor {
                    for x in self.towns[t].faith.iter_mut() { if x.0 != rel { x.1 *= 0.9; } }
                    self.realms[k].persecuting = true;
                }
                let pop = self.towns[t].pop;
                for &(rid, sh) in &self.towns[t].faith { match shares.iter_mut().find(|x| x.0 == rid) { Some(x) => x.1 += sh * pop, None => shares.push((rid, sh * pop)) } }
            }
            let Some(&(major, _)) = shares.iter().max_by(|a, b| a.1.partial_cmp(&b.1).unwrap()) else { continue };
            if major != self.realms[k].faith {
                let t = format!("{} is now the faith of most of {}.", self.religions[major].name, self.realms[k].name);
                self.say(t, false);
                self.realms[k].faith = major;
            }
            if rel != major && !self.chars[r].has_trait("zealous") && self.rng.random() < 0.1 {
                let t = format!("{} turns from {} to {}, the faith of their people.", self.who(r), self.religions[rel].name, self.religions[major].name);
                self.say(t, false);
                self.chars[r].religion = major;
            }
        }
        let mut counts = vec![0.0; self.religions.len()];
        for &t in &towns { for &(rid, sh) in &self.towns[t].faith { counts[rid] += sh * self.towns[t].pop; } }
        for ri in 0..self.religions.len() {
            let total = counts[ri];
            let rel = &mut self.religions[ri];
            rel.followers = total;
            rel.peak = rel.peak.max(total);
            if rel.founder != NONE && rel.peak > 0.0 && total < 1.0 && !rel.dead && y - rel.founded > 10 {
                rel.dead = true;
                let t = format!("The last of {} is gone; its prophet's words are forgotten.", rel.name);
                self.say(t, true);
            }
        }
    }

    fn proclaim(&mut self, c: usize) {
        let home = if self.chars[c].home != NONE && self.towns[self.chars[c].home].alive { self.chars[c].home } else { self.capital_of(self.chars[c].realm) };
        if home == NONE { return; }
        let parent = self.chars[c].religion;
        let n = self.rng.integers(2, 4) as usize;
        let cc = &self.chars[c];
        let mut bias: Vec<&str> = vec![];
        if cc.has_trait("kind") { bias.extend(["charity", "pacifism"]); }
        if cc.has_trait("wrathful") || cc.has_trait("cruel") { bias.extend(["holy war", "martyrdom"]); }
        if cc.has_trait("greedy") { bias.push("wealth gospel"); }
        if self.monsters.iter().any(|m| m.alive && m.kind == "dragon") && self.rng.random() < 0.2 { bias.push("dragon veneration"); }
        let mut pool: Vec<&str> = TENETS.iter().map(|t| t.0).collect();
        for i in (1..pool.len()).rev() { let j = self.rng.integers(0, i as i64 + 1) as usize; pool.swap(i, j); }
        let mut tenets: Vec<&'static str> = vec![];
        for t in bias.into_iter().chain(pool) {
            if let Some(tt) = TENETS.iter().find(|x| x.0 == t) { if !tenets.contains(&tt.0) { tenets.push(tt.0); } }
            if tenets.len() >= n { break; }
        }
        if tenets.contains(&"pacifism") && tenets.contains(&"holy war") { tenets.retain(|t| *t != "holy war"); }
        let deity = format!("{} {}", DEITY_A[self.rng.integers(0, DEITY_A.len() as i64) as usize], DEITY_B[self.rng.integers(0, DEITY_B.len() as i64) as usize]);
        let name = format!("the {} of {deity}", FAITH_A[self.rng.integers(0, FAITH_A.len() as i64) as usize]);
        let id = self.religions.len();
        let teach = tenets.iter().map(|t| TENETS.iter().find(|x| x.0 == *t).unwrap().1).collect::<Vec<_>>().join("; ");
        self.religions.push(Religion { id, name: name.clone(), deity, tenets, founder: c, founded: self.year, people: self.chars[c].people, parent, holy_site: home,
                                       peak: 0.0, followers: 0.0, dead: false });
        self.chars[c].religion = id;
        self.shift(home, id, 0.3);
        if self.chars[c].profession.is_none() { self.chars[c].profession = Some("priest"); }
        let t = format!("**A new faith.** {} proclaims {name} in {}, breaking from {}. Its teachings: {teach}.", self.who(c), self.towns[home].name, self.religions[parent].name);
        self.say(t, false);
        self.gain_fame(c, "prophet", 100.0);
    }

    pub fn ruler_faith_legitimacy(&self, k: usize, r: usize) -> f64 {
        let f = self.realms[k].faith;
        if self.chars[r].religion == f { 8.0 + if self.religions[f].has("sacred kingship") { 6.0 } else { 0.0 } } else { -10.0 }
    }

    pub fn tenet_aggression(&self, k: usize) -> f64 {
        let r = &self.religions[self.realms[k].faith];
        1.0 + 0.4 * r.has("holy war") as u8 as f64 - 0.5 * r.has("pacifism") as u8 as f64
    }
}
