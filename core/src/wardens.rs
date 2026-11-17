//! Village bounties and the Wardens' Hall (bounties.py, guild.py): towns post purses on the monsters and outlaws
//! that plague them; heroes hold seals from Ash to Sun, band into parties, and take quests in their own lane from a
//! board posted from what the world needs: slayings, dragon raids, outlaw hunts, delves, treasure, exploration,
//! escorts and culls.

use crate::monsters::{dragon_stage, mdist};
use crate::people::*;
use crate::state::State;
use crate::tables::*;

pub const SEALS: [&str; 7] = ["Ash Seal", "Oak Seal", "Iron Seal", "Silver Seal", "Gold Seal", "Star Seal", "Sun Seal"];
const SEAL_LEVEL: [u32; 7] = [0, 6, 12, 18, 25, 33, 42];
const SEAL_MERIT: [f64; 7] = [0.0, 20.0, 60.0, 150.0, 350.0, 800.0, 1800.0];
const RANK_DANGER: [f64; 7] = [0.0, 120.0, 300.0, 700.0, 1500.0, 3500.0, 7000.0];
const PARTY_A: [&str; 20] = ["Crimson", "Silver", "Iron", "Ashen", "Golden", "Black", "Wandering", "Seventh", "Last", "Laughing", "Broken", "Bright", "Grey", "Storm", "Thorn", "Lantern", "Oaken", "Hollow", "Bitter", "Lucky"];
const PARTY_B: [&str; 20] = ["Lanterns", "Blades", "Wolves", "Ravens", "Oath", "Company", "Shields", "Hounds", "Spears", "Crows", "Sworn", "Wardens", "Lions", "Kestrels", "Vow", "Brothers", "Sisters", "Hand", "Banner", "Road"];

fn role(cls: &str) -> Option<&'static str> {
    match cls { "Warrior" | "Rogue" => Some("blade"), "Knight" => Some("shield"), "Ranger" => Some("bow"), "Wizard" | "Necromancer" | "Runesmith" => Some("spell"),
                "Priest" | "Druid" | "Shaman" => Some("prayer"), _ => None }
}
fn rank_of(danger: f64) -> usize { (0..7).filter(|&i| danger >= RANK_DANGER[i]).max().unwrap_or(0) }

pub struct Party { pub id: usize, pub name: String, pub members: Vec<usize>, pub rank: usize, pub landmass: i32, pub formed: i64, pub done: u32, pub lost: u32 }

#[derive(Clone)]
pub struct Quest { pub id: usize, pub kind: &'static str, pub title: String, pub danger: f64, pub rank: usize, pub reward: f64, pub landmass: i32,
                   pub pos: (usize, usize), pub target: (char, usize), pub posted: i64, pub expires: i64, pub taken: i64 }

#[derive(Default)]
pub struct Hall { pub parties: Vec<Party>, pub quests: Vec<Quest>, pub next_q: usize, pub next_p: usize, pub log: Vec<(i64, String, &'static str, String, usize, &'static str, usize)> }

impl State {
    // ------------------------------------------------------------ purses
    pub fn bounties_year(&mut self) {
        // monsters
        for t in 0..self.towns.len() {
            let tt = &self.towns[t];
            if !tt.alive { continue; }
            let raided = self.year - tt.raided_year <= 8;
            if (tt.threat < 0.25 && !raided) || tt.wealth < 30.0 || self.rng.random() > 0.3 { continue; }
            let near: Vec<usize> = (0..self.monsters.len()).filter(|&m| { let mo = &self.monsters[m];
                mo.alive && !mo.sea && mo.kind != "calamity" && mo.landmass == tt.landmass && mdist(mo.pos, tt.pos) <= MONSTER.iter().find(|x| x.0 == mo.kind).unwrap().3 + 10.0 }).collect();
            let Some(&m) = near.iter().max_by(|&&a, &&b| { let f = |m: usize| self.monsters[m].power * (-mdist(self.monsters[m].pos, tt.pos) / 15.0).exp(); f(a).partial_cmp(&f(b)).unwrap() }) else { continue };
            if self.monsters[m].bounties.iter().any(|b| b.0 == t) && !raided { continue; }
            let purse = (0.2 * tt.wealth).min(3.0 + 0.01 * self.monsters[m].base) * if raided { 1.5 } else { 1.0 };
            self.towns[t].wealth -= purse;
            let before: f64 = self.monsters[m].bounties.iter().map(|b| b.1).sum();
            match self.monsters[m].bounties.iter_mut().find(|b| b.0 == t) { Some(b) => b.1 += purse, None => self.monsters[m].bounties.push((t, purse)) }
            let total = before + purse;
            let (tn, mn) = (self.towns[t].name.clone(), self.monsters[m].name.clone());
            if before < 60.0 && 60.0 <= total { self.say(format!("**A great bounty.** The towns around {tn} have put up {} gold for the head of {mn}; hunters are on the roads.", crate::economy::thousands(total)), false); }
            else { self.say(format!("The elders of {tn} put up {} gold for whoever rids them of {mn}.", crate::economy::thousands(purse)), true); }
        }
        // criminals
        let y = self.year;
        for t in 0..self.towns.len() {
            if !self.towns[t].alive || self.towns[t].crime < 0.2 || self.towns[t].wealth < 30.0 || self.rng.random() > 0.2 { continue; }
            let here = self.chars.iter().filter(|c| c.alive && c.home == t && c.jailed_until <= y && matches!(c.profession, Some("thief") | Some("assassin") | Some("bandit")))
                .max_by(|a, b| a.bounty.partial_cmp(&b.bounty).unwrap()).map(|c| c.id);
            let Some(c) = here else { continue };
            let purse = 0.1 * self.towns[t].wealth;
            self.towns[t].wealth -= purse;
            self.chars[c].bounty += purse;
            self.chars[c].posted += purse;
            let tx = format!("The town of {} posts {} gold for {}, dead or alive.", self.towns[t].name, crate::economy::thousands(purse), self.chars[c].name);
            self.say(tx, true);
        }
    }

    /// Whoever brings a monster down collects every purse on it.
    pub fn pay_out(&mut self, m: usize, slayer: usize) {
        let purse: f64 = self.monsters[m].bounties.iter().map(|b| b.1).sum();
        self.monsters[m].bounties.clear();
        if purse <= 0.0 || slayer == NONE { return; }
        self.chars[slayer].gold += purse;
        let t = format!("{} collects {} gold in bounties for {}.", self.who(slayer), crate::economy::thousands(purse), self.monsters[m].name);
        self.say(t, purse < 60.0);
    }

    // ------------------------------------------------------------ the hall
    fn free(&self, h: usize) -> bool {
        let c = &self.chars[h];
        c.alive && c.hero && c.hp >= 40.0 && !c.ruled.is_some_and(|r| r.2.is_none()) && c.home != NONE && self.towns[c.home].alive
    }

    pub fn guild_year(&mut self) {
        self.seals();
        self.parties();
        self.post();
        self.work();
        let y = self.year;
        self.hall.quests.retain(|q| q.expires >= y);
        if self.hall.log.len() > 3000 { self.hall.log.drain(..1000); }
    }

    fn seals(&mut self) {
        let heroes: Vec<usize> = self.chars.iter().filter(|c| c.alive && c.hero).map(|c| c.id).collect();
        for h in heroes {
            let c = &self.chars[h];
            let merit = c.merit + if c.seal.is_none() { c.renown * 0.02 } else { 0.0 };
            self.chars[h].merit = merit;
            let s = (0..7).filter(|&i| self.chars[h].level >= SEAL_LEVEL[i] && merit >= SEAL_MERIT[i]).max().unwrap_or(0);
            if self.chars[h].seal.is_none_or(|x| x < s) {
                if s >= 5 {
                    let peers = self.chars.iter().filter(|o| o.alive && o.hero && o.seal.unwrap_or(0) >= s).count();
                    let t = format!("**The Wardens' Hall raises {} to the {}**; there are {} of that seal in the world.", self.who(h), SEALS[s], peers + 1);
                    self.say(t, false);
                } else if s >= 3 { let t = format!("The Wardens' Hall raises {} to the {}.", self.who(h), SEALS[s]); self.say(t, true); }
                self.chars[h].seal = Some(s);
            }
        }
    }


    fn parties(&mut self) {
        let mut i = 0;
        while i < self.hall.parties.len() {
            let alive: Vec<usize> = self.hall.parties[i].members.iter().copied().filter(|&m| self.chars[m].alive && self.chars[m].hero && !self.chars[m].ruled.is_some_and(|r| r.2.is_none())).collect();
            if alive.len() < 2 {
                for &m in &alive { self.chars[m].party = NONE; }
                let p = self.hall.parties.remove(i);
                self.say(format!("{} is no more; too few of them are left.", p.name), p.rank < 4);
                continue;
            }
            let mut ranks: Vec<usize> = alive.iter().map(|&m| self.chars[m].seal.unwrap_or(0)).collect();
            ranks.sort();
            self.hall.parties[i].rank = ranks[ranks.len() / 2];
            self.hall.parties[i].members = alive;
            i += 1;
        }
        let mut loners: Vec<usize> = self.chars.iter().filter(|c| c.hero && c.party == NONE).map(|c| c.id).filter(|&h| self.free(h)).collect();
        for i in (1..loners.len()).rev() { let j = self.rng.integers(0, i as i64 + 1) as usize; loners.swap(i, j); }
        for &h in &loners.clone() {
            if self.chars[h].party != NONE || self.rng.random() > 0.35 { continue; }
            if (self.chars[h].has_trait("shy") || self.chars[h].has_trait("arrogant")) && self.rng.random() < 0.6 { continue; }
            let land = self.towns[self.chars[h].home].landmass;
            let seal = self.chars[h].seal.unwrap_or(0);
            let fits: Vec<usize> = (0..self.hall.parties.len()).filter(|&p| self.hall.parties[p].landmass == land && self.hall.parties[p].members.len() < 6 && (self.hall.parties[p].rank as i64 - seal as i64).abs() <= 1).collect();
            if let Some(&p) = fits.iter().max_by_key(|&&p| -(self.hall.parties[p].rank as i64 - seal as i64).abs()) {
                self.hall.parties[p].members.push(h);
                self.chars[h].party = self.hall.parties[p].id;
                continue;
            }
            let mates: Vec<usize> = loners.iter().copied().filter(|&o| o != h && self.chars[o].party == NONE && self.free(o) && self.towns[self.chars[o].home].landmass == land
                && (self.chars[o].seal.unwrap_or(0) as i64 - seal as i64).abs() <= 1).collect();
            if mates.len() < 2 { continue; }
            let take = self.rng.integers(2, 6) as usize;
            let mut crew = vec![h];
            crew.extend(mates.into_iter().take(take));
            let pid = self.hall.next_p;
            self.hall.next_p += 1;
            let name = format!("the {} {}", PARTY_A[self.rng.integers(0, 20) as usize], PARTY_B[self.rng.integers(0, 20) as usize]);
            for &c in &crew { self.chars[c].party = pid; }
            let who = crew.iter().map(|&c| self.who(c)).collect::<Vec<_>>().join(", ");
            self.hall.parties.push(Party { id: pid, name: name.clone(), members: crew, rank: seal, landmass: land, formed: self.year, done: 0, lost: 0 });
            self.say(format!("A new party, {name}: {who}."), seal < 4);
        }
    }

    fn strength(&self, members: &[usize], danger: f64) -> f64 {
        let mut total = 0.0;
        let mut roles: Vec<&str> = vec![];
        for &h in members {
            let cr = self.cr(&self.chars[h], true) * self.mods(&self.chars[h])[5];
            total += if cr >= 0.1 * danger { cr } else { cr * 0.05 };
            if let Some(r) = role(CLASSES[self.chars[h].classes[0].0].name) { if !roles.contains(&r) { roles.push(r); } }
        }
        total * (1.0 + 0.08 * roles.len() as f64)
    }

    #[allow(clippy::too_many_arguments)]
    fn add_quest(&mut self, kind: &'static str, title: String, danger: f64, reward: f64, land: i32, pos: (usize, usize), target: (char, usize), years: i64) {
        if self.hall.quests.iter().any(|q| q.kind == kind && q.target == target) { return; }
        let id = self.hall.next_q;
        self.hall.next_q += 1;
        self.hall.quests.push(Quest { id, kind, title, danger, rank: rank_of(danger), reward, landmass: land, pos, target, posted: self.year, expires: self.year + years, taken: -1 });
    }

    fn post(&mut self) {
        for m in 0..self.monsters.len() {
            let mo = &self.monsters[m];
            if !mo.alive || mo.sea || mo.kind == "calamity" || self.dragon_hidden(m) { continue; }
            let purse: f64 = mo.bounties.iter().map(|b| b.1).sum();
            if mo.kind == "dragon" {
                if purse <= 0.0 && mo.temper == "reclusive" { continue; }
                let st = dragon_stage(self.year - mo.born);
                let t = format!("Bring down {}, the {st} wyrm", mo.name);
                let (b, l, p) = (mo.base, mo.landmass, mo.pos);
                self.add_quest("dragon", t, b, purse + 40.0, l, p, ('m', m), 12);
            } else if purse > 0.0 {
                let t = format!("Slay {}", mo.name);
                let (pw, l, p) = (mo.power, mo.landmass, mo.pos);
                self.add_quest("slay", t, pw, purse, l, p, ('m', m), 6);
            }
        }
        let y = self.year;
        for c in 0..self.chars.len() {
            if !self.chars[c].alive || self.chars[c].posted <= 0.0 || self.chars[c].jailed_until > y { continue; }
            let t = self.chars[c].home;
            if t == NONE { continue; }
            let title = format!("Bring in {}, wanted in {}", self.chars[c].name, self.towns[t].name);
            let danger = 3.0 * self.cr(&self.chars[c], true) + 60.0;
            let (b, l, p) = (self.chars[c].bounty, self.towns[t].landmass, self.towns[t].pos);
            self.add_quest("outlaw", title, danger, b, l, p, ('c', c), 6);
        }
        for r in 0..self.ruins.len() {
            if self.ruins[r].conquered.is_some() || self.rng.random() > 0.05 { continue; }
            self.ensure_dungeon(r);
            let land = self.map.landmass.at(self.ruins[r].pos.0, self.ruins[r].pos.1);
            let loot: Vec<usize> = self.ruins[r].loot.iter().copied().filter(|&i| self.items[i].rarity >= 3).collect();
            let (bp, pos, rich, name) = (self.ruins[r].boss.1, self.ruins[r].pos, self.ruins[r].riches, self.ruins[r].name.clone());
            if !loot.is_empty() && self.rng.random() < 0.5 {
                let it = *loot.iter().max_by_key(|&&i| self.items[i].rarity).unwrap();
                let t = format!("Recover {} from {name}", self.items[it].label());
                let rw = 10.0 + 10.0 * self.items[it].rarity as f64;
                self.add_quest("treasure", t, bp * 0.8, rw, land, pos, ('r', r), 10);
            } else {
                self.add_quest("delve", format!("Delve {name} to the bottom"), bp, 4.0 * rich as f64, land, pos, ('r', r), 10);
            }
        }
        for k in 0..self.realms.len() {
            if !self.active(k) || self.rng.random() > 0.15 { continue; }
            let towns = self.alive_towns_of(k);
            if towns.is_empty() { continue; }
            let revenue = (self.realms[k].income * self.realms[k].tax_now).max(1.0);
            let t = towns[self.rng.integers(0, towns.len() as i64) as usize];
            let title = format!("Chart the wild country beyond {} for {}", self.towns[t].name, self.realms[k].name);
            let (th, l, p) = (self.towns[t].threat, self.towns[t].landmass, self.towns[t].pos);
            self.add_quest("explore", title, 60.0 + 600.0 * th, (0.3 * revenue).max(2.0), l, p, ('x', t), 5);
            let traders: Vec<usize> = towns.iter().copied().filter(|&x| self.towns[x].trade > 0.0 && self.towns[x].threat > 0.15).collect();
            if !traders.is_empty() && towns.len() >= 2 {
                let a = traders[self.rng.integers(0, traders.len() as i64) as usize];
                let ap = self.towns[a].pos;
                let b = towns.iter().copied().filter(|&x| x != a && self.towns[x].landmass == self.towns[a].landmass).min_by(|&x, &y| mdist(self.towns[x].pos, ap).partial_cmp(&mdist(self.towns[y].pos, ap)).unwrap());
                if let Some(b) = b {
                    let title = format!("Escort the merchants of {} to {}", self.towns[a].name, self.towns[b].name);
                    let danger = 100.0 + 900.0 * (self.towns[a].threat + self.towns[b].threat);
                    let (w, l) = (self.towns[a].wealth, self.towns[a].landmass);
                    self.add_quest("escort", title, danger, 0.05 * w, l, ap, ('e', a), 3);
                }
            }
            let v = towns[self.rng.integers(0, towns.len() as i64) as usize];
            let title = format!("Cull the wolves preying on the flocks of {}", self.towns[v].name);
            let d = self.rng.uniform(30.0, 110.0);
            let (w, l, p) = (self.towns[v].wealth, self.towns[v].landmass, self.towns[v].pos);
            self.add_quest("cull", title, d, (0.02 * w).max(1.0), l, p, ('w', v), 3);
        }
    }

    fn work(&mut self) {
        let mut bands: Vec<(bool, usize)> = (0..self.hall.parties.len()).map(|p| (true, self.hall.parties[p].id)).collect();
        bands.extend(self.chars.iter().filter(|c| c.hero && c.party == NONE).map(|c| c.id).filter(|&h| self.free(h)).map(|h| (false, h)));
        for i in (1..bands.len()).rev() { let j = self.rng.integers(0, i as i64 + 1) as usize; bands.swap(i, j); }
        let y = self.year;
        for (is_party, id) in bands {
            let (members, rank, party) = if is_party {
                let Some(pi) = self.hall.parties.iter().position(|p| p.id == id) else { continue };
                let ms: Vec<usize> = self.hall.parties[pi].members.iter().copied().filter(|&m| self.free(m)).collect();
                if ms.len() < 2 { continue; }
                (ms, self.hall.parties[pi].rank, Some(pi))
            } else {
                if !self.free(id) { continue; }
                (vec![id], self.chars[id].seal.unwrap_or(0), None)
            };
            let land = self.towns[self.chars[members[0]].home].landmass;
            let size = members.len();
            let lane: Vec<usize> = (0..self.hall.quests.len()).filter(|&q| { let qq = &self.hall.quests[q];
                qq.taken != y && qq.landmass == land && rank as i64 - 2 <= qq.rank as i64 && qq.rank <= rank
                && (qq.kind != "dragon" || (size >= 4 && rank >= 5 && (qq.rank <= 5 || rank >= 6)))
                && (size > 1 || (qq.kind != "dragon" && qq.rank <= rank.saturating_sub(1) + 1)) }).collect();
            if lane.is_empty() || self.rng.random() > 0.6 { continue; }
            let mut best = (f64::MIN, 0);
            for q in lane { let qq = &self.hall.quests[q]; let s = qq.reward * (1.0 + qq.rank as f64) * self.rng.uniform(0.7, 1.3); if s > best.0 { best = (s, q); } }
            self.hall.quests[best.1].taken = y;
            let q = self.hall.quests[best.1].clone();
            self.go(q, members, party);
        }
    }

    fn go(&mut self, q: Quest, members: Vec<usize>, party: Option<usize>) {
        let name = match party { Some(p) => self.hall.parties[p].name.clone(), None => self.who(members[0]) };
        let leader = *members.iter().max_by_key(|&&h| self.chars[h].level).unwrap();
        let big = q.rank >= 4 || q.kind == "dragon";
        let outcome: &'static str = if q.kind == "slay" || q.kind == "dragon" {
            let m = q.target.1;
            if !self.monsters[m].alive { self.hall.quests.retain(|x| x.id != q.id); return; }
            self.fight(m, &members)
        } else {
            let p = self.strength(&members, q.danger);
            let win = self.rng.random() < p / (p + 0.8 * q.danger);
            for &h in &members {
                if self.rng.random() < if win { 0.3 } else { 0.7 } {
                    let d = self.rng.uniform(5.0, 40.0) * q.danger / p.max(1.0);
                    self.hurt(h, d, format!("died on the quest: {}", q.title), NONE, NONE);
                }
            }
            if win { "done" } else { "failed" }
        };
        let alive: Vec<usize> = members.iter().copied().filter(|&h| self.chars[h].alive).collect();
        let lost = members.len() - alive.len();
        if let Some(p) = party { if p < self.hall.parties.len() { self.hall.parties[p].lost += lost as u32; } }
        if outcome == "done" || outcome == "slain" {
            let who = if self.chars[leader].alive { leader } else { alive.first().copied().unwrap_or(NONE) };
            self.quest_effect(&q, who);
            let share = q.reward / alive.len().max(1) as f64;
            let merit = 10.0 * ((q.rank + 1) as f64).powf(1.5) * if outcome == "slain" && q.kind == "dragon" { 3.0 } else { 1.0 };
            for &h in &alive { self.chars[h].gold += share; self.chars[h].merit += merit; self.renown(h, merit / 4.0); }
            if let Some(p) = party { if p < self.hall.parties.len() { self.hall.parties[p].done += 1; } }
            self.hall.quests.retain(|x| x.id != q.id);
        } else if outcome == "driven off" {
            for &h in &alive { self.chars[h].merit += 10.0 * ((q.rank + 1) as f64).powf(1.5) * 0.5; }
        }
        self.hall.log.push((self.year, name.clone(), q.kind, q.title.clone(), q.rank, outcome, lost));
        let words = match outcome { "done" | "slain" => "completes", "failed" => "fails at", "driven off" => "drives off the wyrm in", _ => "is beaten back from" };
        let tail = if lost > 0 { format!("; {lost} of them do not come home") } else { String::new() };
        let b = if big && outcome != "failed" { "**" } else { "" };
        self.say(format!("{b}{name}{b} ({}) {words} the quest: {}{tail}.", SEALS[q.rank], q.title), !(big || lost >= 2));
    }

    fn fight(&mut self, m: usize, members: &[usize]) -> &'static str {
        let lair = if self.monsters[m].kind == "dragon" { 0.7 } else { 1.0 };
        for _ in 0..3 {
            let alive: Vec<usize> = members.iter().copied().filter(|&h| self.chars[h].alive).collect();
            let p = self.strength(&alive, self.monsters[m].power);
            if p <= 0.0 { return "fled"; }
            let dmg = p * self.rng.uniform(0.4, 0.9) * lair;
            self.hit(m, dmg);
            if self.monsters[m].power <= 0.0 && self.monsters[m].alive {
                self.monsters[m].alive = false;
                let alive: Vec<usize> = members.iter().copied().filter(|&h| self.chars[h].alive).collect();
                let slayer = alive.iter().copied().max_by(|&a, &b| self.cr(&self.chars[a], true).partial_cmp(&self.cr(&self.chars[b], true)).unwrap()).unwrap_or(NONE);
                let purse: f64 = self.monsters[m].bounties.iter().map(|b| b.1).sum();
                self.on_monster_slain(m, slayer);
                if slayer != NONE && purse > 0.0 {
                    let rest: Vec<usize> = alive.iter().copied().filter(|&h| h != slayer).collect();
                    let cut = purse / (rest.len() + 1) as f64;
                    self.chars[slayer].gold -= cut * rest.len() as f64;
                    for &h in &rest { self.chars[h].gold += cut; }
                }
                let kind = self.monsters[m].kind;
                let ren = MONSTER.iter().find(|x| x.0 == kind).unwrap().7;
                for &h in &alive { self.deed(h, kind); self.renown(h, ren / members.len().max(1) as f64 * 2.0); }
                if kind == "dragon" {
                    let who = alive.iter().map(|&h| self.who(h)).collect::<Vec<_>>().join(", ");
                    self.say(format!("**A dragon falls.** {} dies in its lair at the hands of {who}.", self.monsters[m].name), false);
                }
                return "slain";
            }
            if self.monsters[m].fled == Some(self.year) { return "driven off"; }
            let mname = self.monsters[m].name.clone();
            for &h in members {
                if self.chars[h].alive && self.rng.random() < 0.6 {
                    let mp = self.monsters[m].power;
                    let d = self.rng.uniform(15.0, 60.0) * mp / (p + mp).max(1.0) * 2.0;
                    self.hurt(h, d, format!("was slain by {mname}"), m, NONE);
                }
            }
        }
        "fled"
    }

    fn quest_effect(&mut self, q: &Quest, who: usize) {
        match q.kind {
            "outlaw" => {
                let c = q.target.1;
                if self.chars[c].alive {
                    if self.rng.random() < 0.5 { self.kill_by(c, "was brought in dead by bounty hunters".into(), true, NONE); }
                    else { self.chars[c].jailed_until = self.year + self.rng.integers(5, 20); }
                    self.chars[c].bounty = 0.0;
                    self.chars[c].posted = 0.0;
                }
            }
            "delve" | "treasure" => {
                let r = q.target.1;
                self.ruins[r].conquered = Some(self.year);
                if who != NONE && !self.ruins[r].loot.is_empty() {
                    let it = *self.ruins[r].loot.iter().max_by_key(|&&i| self.items[i].rarity).unwrap();
                    self.ruins[r].loot.retain(|&x| x != it);
                    self.give_item(who, it, false);
                }
            }
            "explore" => {
                let t = q.target.1;
                if self.towns[t].alive && self.rng.random() < 0.4 {
                    let lid = self.towns[t].landmass;
                    let terrain = self.map.terrain.clone();
                    let lm = self.map.landmass.clone();
                    let cells = self.remote_cells(&|y, x| terrain.at(y, x) != OCEAN && lm.at(y, x) == lid);
                    if !cells.is_empty() {
                        let pos = cells[self.rng.integers(0, cells.len() as i64) as usize];
                        let found = if self.rng.random() < 0.5 {
                            let name = format!("the ruins of {}", self.name(&["Ny", "Aza", "Quel", "Oro", "Tes", "Vael", "Hy", "Esk"], &["thar", "marath", "lorn", "essa", "vane", "dor", "ul", "ith"]));
                            let (rich, depth) = (self.rng.integers(3, 9), self.rng.uniform(1.0, 2.5));
                            self.ruins.push(Ruin::new(name.clone(), pos, rich, depth));
                            name
                        } else {
                            let i = self.rng.integers(0, RESOURCES.len() as i64) as usize;
                            self.res_grid.set(pos.0, pos.1, i as i16);
                            format!("a deposit of {}", RESOURCES[i].name)
                        };
                        let tx = format!("Wardens charting the wild beyond {} find {found}.", self.towns[t].name);
                        self.say(tx, true);
                    }
                }
            }
            "escort" => { let t = q.target.1; if self.towns[t].alive { self.towns[t].wealth += q.reward * 2.0; } }
            "cull" => { let t = q.target.1; if self.towns[t].alive { self.towns[t].threat = (self.towns[t].threat - 0.05).max(0.0); } }
            _ => {}
        }
    }
}
