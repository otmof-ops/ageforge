//! Specialists, their work, gold and fame (crafts.py): smiths forge in seven materials and six qualities,
//! enchanters bind effects (the high tiers need soulstones), alchemists brew, merchants trade; everything made
//! goes to a people's market where heroes and the great buy it; fame in any specialty becomes renown and legacy.

use crate::people::*;
use crate::power::skill_ix;
use crate::state::State;
use crate::tables::*;

const SKILL_LEGACY: [(&str, &str); 19] = [("Smithing", "smith"), ("Enchanting", "enchanter"), ("Alchemy", "alchemist"), ("Mercantile", "merchant"),
    ("Speech", "prophet"), ("Sneak", "thief"), ("Security", "thief"), ("Pickpocket", "thief"), ("Blade", "warrior"), ("Blunt", "warrior"),
    ("Archery", "warrior"), ("Block", "guardian"), ("Heavy Armor", "guardian"), ("Light Armor", "warrior"), ("Destruction", "arcane"),
    ("Restoration", "arcane"), ("Conjuration", "arcane"), ("Illusion", "arcane"), ("Alteration", "arcane")];
const FAME_WORD: [(&str, &str); 14] = [("artist", "artist"), ("scholar", "scholar"), ("Smithing", "smith"), ("Enchanting", "enchanter"), ("Alchemy", "alchemist"), ("Mercantile", "merchant"),
    ("Speech", "speaker"), ("Sneak", "thief"), ("Security", "thief"), ("Pickpocket", "thief"), ("assassin", "assassin"),
    ("shinobi", "shinobi"), ("bender", "bender"), ("prophet", "prophet")];
const GREATEST_AT: f64 = 5000.0;

pub fn rarity_of(bonus: f64) -> usize {
    [0.08, 0.15, 0.28, 0.45, 0.75].iter().position(|&t| bonus < t).unwrap_or(5)
}

impl State {
    /// Fame in a specialty: renown for the person, legacy for their line, as far as the tale is told.
    pub fn gain_fame(&mut self, c: usize, specialty: &'static str, amount: f64) {
        if amount <= 0.0 || !self.chars[c].alive { return; }
        let before = self.chars[c].fame.iter().find(|x| x.0 == specialty).map(|x| x.1).unwrap_or(0.0);
        let after = before + amount;
        match self.chars[c].fame.iter_mut().find(|x| x.0 == specialty) { Some(x) => x.1 = after, None => self.chars[c].fame.push((specialty, after)) }
        self.chars[c].renown += amount * 0.5;
        let kind = SKILL_LEGACY.iter().find(|x| x.0 == specialty).map(|x| x.1)
            .or(if ["assassin", "shinobi", "bender", "prophet", "concord"].contains(&specialty) { Some(specialty) } else { None });
        if let Some(kind) = kind { self.gain_legacy(c, kind, amount * 0.5); }
        let Some(&(_, word)) = FAME_WORD.iter().find(|x| x.0 == specialty) else { return };
        for (at, text) in [(500.0, "known in their own country as a fine"), (2000.0, "spoken of across the land as a great")] {
            if before < at && at <= after { let t = format!("{} is {text} {word}.", self.who(c)); self.say(t, true); }
        }
        let holder = self.greatest.iter().find(|g| g.0 == word).map(|g| g.1).unwrap_or(NONE);
        let hf = if holder != NONE && self.chars[holder].alive { self.chars[holder].fame.iter().find(|x| x.0 == specialty).map(|x| x.1).unwrap_or(0.0) } else { 0.0 };
        if after >= GREATEST_AT.max(hf) && holder != c {
            let was = if holder != NONE && self.chars[holder].alive { format!(", surpassing {}", self.chars[holder].name) } else { String::new() };
            self.greatest.retain(|g| g.0 != word);
            self.greatest.push((word, c));
            let t = format!("**{} is called the greatest {word} of the age**{was}.", self.who(c));
            self.say(t, false);
        }
    }

    pub fn master_name(&mut self) -> String {
        for _ in 0..60 {
            let n = format!("the {} of {}", ITEM_A[self.rng.integers(0, ITEM_A.len() as i64) as usize], ITEM_B[self.rng.integers(0, ITEM_B.len() as i64) as usize]);
            if self.item_names.insert(n.clone()) { return n; }
        }
        let f = crate::people::first_name(&mut self.rng, 0, false);
        format!("the {} of {f}", ITEM_A[self.rng.integers(0, ITEM_A.len() as i64) as usize])
    }

    pub fn new_item(&mut self, it: Item) -> usize { self.items.push(it); self.items.len() - 1 }

    pub fn give_item(&mut self, c: usize, it: usize, quiet: bool) {
        if self.chars[c].items.contains(&it) { return; }
        // an item handed on outright is no longer kept in trust for anyone
        self.items[it].held_for = NONE;
        self.items[it].keeper = NONE;
        if self.items[it].rarity >= 3 { let lb = self.items[it].label(); self.life(c, "heroes", format!("Comes to bear {lb}.")); }
        self.chars[c].items.push(it);
        self.items[it].bearers.push(c);
        let mut items = std::mem::take(&mut self.chars[c].items);
        items.sort_by_key(|&i| std::cmp::Reverse(self.items[i].rarity));
        while items.len() > 4 {
            let spare = items.pop().unwrap();
            let k = self.chars[c].realm;
            if k < self.realms.len() { self.realms[k].treasury.push(spare); }
        }
        self.chars[c].items = items;
        if !quiet && self.items[it].rarity >= 4 {
            let note = if self.items[it].note.is_empty() { String::new() } else { format!(": {}", self.items[it].note) };
            let t = format!("{} claims {}{note}.", self.who(c), self.items[it].label());
            self.say(t, false);
        }
    }

    fn craft(&mut self, smith: usize) -> usize {
        let k = self.chars[smith].realm;
        let sk = self.skill_val(&self.chars[smith], skill_ix("Smithing"));
        let mat_cap = self.cultures[self.chars[smith].people].best("material", 1.0) as usize;
        let held = &self.realms[k].held;
        let ok = |i: usize| match NEEDS_MATERIAL.iter().find(|x| x.0 == i) {
            None => true,
            Some(&(_, need)) => held.iter().any(|&r| RESOURCES[r].name == need) || (need == "dragonbone fossils" && self.realms[k].dragonbone > 0),
        };
        let mut avail: Vec<usize> = (0..MATERIALS.len()).filter(|&i| sk >= MATERIALS[i].1 && i <= mat_cap && ok(i)).collect();
        if avail.is_empty() { avail.push(0); }
        if MATERIALS[*avail.last().unwrap()].0 == "Dragonbone" && self.realms[k].dragonbone == 0 { avail.pop(); }
        let mi = if self.rng.random() < 0.7 { avail.len() - 1 } else { avail.len().saturating_sub(2) };
        let mat = avail[mi];
        if MATERIALS[mat].0 == "Dragonbone" { self.realms[k].dragonbone -= 1; }
        let perks = self.chars[smith].perks.iter().filter(|p| p.0 == skill_ix("Smithing")).count() as f64;
        let roll = sk / 20.0 + perks * 0.3 + self.rng.normal(0.0, 0.8) + self.chars[smith].attributes[7] / 100.0;
        let q = ((roll - 1.0).clamp(0.0, (QUALITY.len() - 1) as f64)) as usize;
        let weapon = self.rng.random() < 0.6;
        let (wname, _) = if weapon { WEAPONS[self.rng.integers(0, WEAPONS.len() as i64) as usize] } else { ARMORS[self.rng.integers(0, ARMORS.len() as i64) as usize] };
        let bonus = MATERIALS[mat].2 * QUALITY[q].1;
        let mut it = Item::new(format!("{} {} {wname}", QUALITY[q].0, MATERIALS[mat].0), rarity_of(bonus), self.year, format!("the forge of {}", self.chars[smith].name));
        it.kind = if weapon { "weapon" } else { "armor" };
        it.maker = smith;
        it.bonus = bonus;
        it.mat_index = mat;
        it.value = (20.0 * (bonus * 100.0).powf(1.3)).floor();
        if bonus >= 0.75 {
            it.desc = Some(it.name.clone());
            it.name = self.master_name();
            let t = format!("{} forges {}, a {}.", self.who(smith), it.name, it.desc.clone().unwrap());
            self.say(t, bonus < 1.2);
            self.gain_fame(smith, "Smithing", 40.0 + 100.0 * bonus);
        } else {
            self.gain_fame(smith, "Smithing", 1.0 + 8.0 * bonus);
        }
        self.use_skill(smith, skill_ix("Smithing"), 1.5);
        self.realms[k].made.push(mat);
        self.new_item(it)
    }

    fn enchant(&mut self, ench: usize, it: usize) {
        let k = self.chars[ench].realm;
        let sk = self.skill_val(&self.chars[ench], skill_ix("Enchanting"));
        let mut t = (0..ENCHANT_TIERS.len()).filter(|&i| sk >= ENCHANT_TIERS[i].1).last().unwrap_or(0);
        while t >= 2 && self.realms[k].soulstones[t] == 0 { t -= 1; }
        if t >= 2 { self.realms[k].soulstones[t] -= 1; }
        let perks = self.chars[ench].perks.iter().filter(|p| p.0 == skill_ix("Enchanting")).count() as f64;
        let double = perks >= 3.0 && self.rng.random() < 0.5;
        let pool: &[&str] = if self.items[it].kind == "weapon" { &ENCHANT_WEAPON } else { &ENCHANT_ARMOR };
        let mut effects = vec![pool[self.rng.integers(0, pool.len() as i64) as usize]];
        if double { effects.push(pool[self.rng.integers(0, pool.len() as i64) as usize]); }
        let add = ENCHANT_TIERS[t].2 * (1.0 + 0.1 * perks) * effects.len() as f64;
        let item = &mut self.items[it];
        for e in &effects { item.enchants.push((e, ENCHANT_TIERS[t].0)); }
        item.bonus += add;
        item.rarity = rarity_of(item.bonus);
        item.enchanter = ench;
        item.value = (item.value + 25.0 * (add * 100.0).powf(1.3)).floor();
        self.use_skill(ench, skill_ix("Enchanting"), 1.5);
        if t >= 3 {
            let text = format!("{} lays a {} {} enchantment on {}.", self.who(ench), ENCHANT_TIERS[t].0, effects.join(" and "), self.items[it].name);
            self.say(text, t < 4);
            self.gain_fame(ench, "Enchanting", 30.0 * t as f64);
        } else {
            self.gain_fame(ench, "Enchanting", 4.0 * (t + 1) as f64);
        }
    }

    pub fn spawn_specialists(&mut self) {
        let y = self.year;
        for f in 0..self.realms.len() {
            if self.realms[f].extinct { continue; }
            let mut towns = self.alive_towns_of(f);
            if towns.is_empty() { continue; }
            towns.sort_by(|&a, &b| self.towns[b].pop.partial_cmp(&self.towns[a].pop).unwrap());
            let members: Vec<usize> = self.living.iter().map(|&i| &self.chars[i]).filter(|c| c.alive && c.realm == f).map(|c| c.id).collect();
            let n = towns.len();
            for &(pname, cap, prim, _) in PROFESSIONS.iter() {
                let have = members.iter().filter(|&&c| self.chars[c].profession == Some(pname)).count();
                let cap = cap.min(n / 6 + if pname == "smith" && n >= 2 { 1 } else { 0 });
                if have >= cap || self.rng.random() > 0.15 { continue; }
                let kin = members.iter().copied().find(|&c| {
                    let cc = &self.chars[c];
                    cc.profession.is_none() && !cc.hero && cc.adult(y) && cc.ruled.is_none() && cc.father != NONE && self.chars[cc.father].profession == Some(pname)
                });
                let kin = kin.map(|c| (c, true)).or_else(|| self.chosen_trade(&members, pname, y).map(|c| (c, false)));
                let (c, why) = match kin {
                    Some((c, true)) => (c, Some(format!("takes up the trade of {}", self.chars[self.chars[c].father].name))),
                    Some((c, false)) => (c, Some(format!("becomes a {pname}, as they always meant to"))),
                    None => {
                        let age = ADULT[self.realms[f].people] as i64 + self.rng.integers(0, 15);
                        let c = self.make_character(f, y - age, None, NONE, NONE, NONE);
                        let t = towns[self.rng.integers(0, n.min(6) as i64) as usize];
                        self.chars[c].home = t;
                        (c, None)
                    }
                };
                self.chars[c].profession = Some(pname);
                self.life(c, "economy", format!("Takes up the trade of {pname}."));
                for &s in prim { let add = self.rng.integers(20, 40) as f64; self.chars[c].skills[s] = (self.chars[c].skills[s] + add).min(100.0); }
                if pname == "tamer" && !self.chars[c].has(gene("beastspeaker")) { self.chars[c].genetic.push(gene("beastspeaker")); }
                if let Some(w) = why { let t = format!("{} {w}.", self.who(c)); self.say(t, true); }
            }
        }
    }

    pub fn specialists_year(&mut self) {
        let y = self.year;
        for k in self.realms.iter_mut() { k.made.clear(); }
        for i in self.living.clone() {
            let c = &self.chars[i];
            if !c.alive || c.jailed_until > y { continue; }
            let Some(p) = c.profession else { continue };
            let k = c.realm;
            if k >= self.realms.len() || self.realms[k].extinct { continue; }
            match p {
                "smith" => {
                    let n = 1 + (self.skill_val(&self.chars[i], skill_ix("Smithing")) > 60.0) as usize;
                    for _ in 0..n { let it = self.craft(i); self.realms[k].market.push(it); }
                }
                "enchanter" => {
                    let plain: Vec<usize> = self.realms[k].market.iter().copied().filter(|&it| self.items[it].enchants.is_empty()).collect();
                    if let Some(&best) = plain.iter().max_by(|&&a, &&b| self.items[a].bonus.partial_cmp(&self.items[b].bonus).unwrap()) { self.enchant(i, best); }
                }
                "alchemist" => {
                    let sk = self.skill_val(&self.chars[i], skill_ix("Alchemy"));
                    self.realms[k].potions += sk / 20.0;
                    self.use_skill(i, skill_ix("Alchemy"), 1.2);
                    self.gain_fame(i, "Alchemy", sk / 15.0);
                }
                "merchant" => {
                    let home = if self.chars[i].home != NONE && self.towns[self.chars[i].home].alive { self.chars[i].home } else { self.capital_of(k) };
                    let income = 20.0 + self.skill_val(&self.chars[i], skill_ix("Mercantile")) * 2.0 + if home != NONE { self.towns[home].wealth * 0.01 } else { 0.0 };
                    self.chars[i].gold += income;
                    if home != NONE { self.towns[home].wealth += income * 0.5; }
                    self.use_skill(i, skill_ix("Mercantile"), 1.2);
                    self.use_skill(i, skill_ix("Speech"), 0.5);
                    self.gain_fame(i, "Mercantile", income / 40.0);
                }
                _ => {}
            }
        }
        for f in 0..self.realms.len() {
            if self.realms[f].extinct { continue; }
            let mut m = std::mem::take(&mut self.realms[f].market);
            m.sort_by(|&a, &b| self.items[b].bonus.partial_cmp(&self.items[a].bonus).unwrap());
            m.truncate(30);
            self.realms[f].market = m;
            if !self.realms[f].made.is_empty() {
                let avg = self.realms[f].made.iter().sum::<usize>() as f64 / self.realms[f].made.len() as f64;
                self.realms[f].arms = 0.9 * self.realms[f].arms + 0.1 * avg;
            }
            let budget = 0.01 * self.alive_towns_of(f).iter().map(|&t| self.towns[t].wealth).sum::<f64>();
            let smiths: Vec<usize> = self.living.iter().map(|&i| &self.chars[i]).filter(|c| c.alive && c.realm == f && c.profession == Some("smith")).map(|c| c.id).collect();
            for &s in &smiths { self.chars[s].gold += budget / smiths.len() as f64; }
            let arms = self.realms[f].arms;
            self.realms[f].battle_mult *= 1.0 + 0.04 * arms;
        }
        self.market();
    }

    /// Heroes, shinobi, benders and the great of each people buy from their own people's market.
    fn market(&mut self) {
        let mut buyers: Vec<usize> = self.living.iter().map(|&i| &self.chars[i]).filter(|c| c.alive && c.gold > 50.0 && (c.hero || c.ruled.is_some() || c.clan != NONE || c.element.is_some())).map(|c| c.id).collect();
        buyers.sort_by(|&a, &b| self.chars[b].gold.partial_cmp(&self.chars[a].gold).unwrap());
        for c in buyers {
            let k = self.chars[c].realm;
            if k >= self.realms.len() { continue; }
            let worst = if self.chars[c].items.len() >= 4 { self.chars[c].items.iter().map(|&i| self.items[i].bonus).fold(f64::MAX, f64::min) } else { 0.0 };
            let gold = self.chars[c].gold;
            let best = self.realms[k].market.iter().copied().filter(|&it| self.items[it].value <= gold && self.items[it].bonus > worst + 0.02)
                .max_by(|&a, &b| self.items[a].bonus.partial_cmp(&self.items[b].bonus).unwrap());
            let Some(it) = best else { continue };
            self.realms[k].market.retain(|&x| x != it);
            let (value, maker, ench) = (self.items[it].value, self.items[it].maker, self.items[it].enchanter);
            self.chars[c].gold -= value;
            if maker != NONE { self.chars[maker].gold += value * if ench != NONE { 0.7 } else { 1.0 }; self.gain_fame(maker, "Smithing", value / 1000.0); }
            if ench != NONE { self.chars[ench].gold += value * 0.3; }
            self.give_item(c, it, true);
            if self.items[it].rarity >= 4 {
                let t = format!("{} buys {} for {} gold.", self.who(c), self.items[it].label(), crate::economy::thousands(value));
                self.say(t, true);
            }
        }
    }

    /// Items go to the heir, or into the treasury (governance.inherit; a monster's hoard joins with slice 6).
    pub fn inherit(&mut self, c: usize, killer_hoard: Option<usize>) {
        let mut items = std::mem::take(&mut self.chars[c].items);
        // one item, one entry: a second entry would be handed on twice
        let mut seen = std::collections::HashSet::new();
        items.retain(|&i| seen.insert(i));
        if items.is_empty() { return; }
        let best = *items.iter().max_by_key(|&&i| self.items[i].rarity).unwrap();
        if let Some(m) = killer_hoard {
            if self.items[best].rarity >= 4 { let t = format!("{} is dragged into the hoard of {}.", self.items[best].label(), self.monsters[m].name); self.say(t, true); }
            self.monsters[m].items.extend(items);
            return;
        }
        let heir = self.heir_of(c);
        let k = self.chars[c].realm;
        // what was only kept for a child goes on to that child (or their next guardian), not to the keeper's heir
        let (kept, items): (Vec<usize>, Vec<usize>) = items.into_iter().partition(|&i| self.items[i].held_for != NONE && self.chars[self.items[i].held_for].alive);
        for &i in &items { self.items[i].held_for = NONE; self.items[i].keeper = NONE; }
        for it in kept {
            let kid = self.items[it].held_for;
            if kid == NONE || kid >= self.chars.len() || !self.chars[kid].alive { self.items[it].held_for = NONE; self.items[it].keeper = NONE; if k < self.realms.len() && !self.realms[k].extinct { self.realms[k].treasury.push(it); } continue; }
            if self.chars[kid].adult(self.year) { self.items[it].held_for = NONE; self.items[it].keeper = NONE; self.give_item(kid, it, true); continue; }
            let g = self.guardian_of(kid, c);
            self.items[it].keeper = g;
            if g != NONE { if !self.chars[g].items.contains(&it) { self.chars[g].items.push(it); } } else if k < self.realms.len() && !self.realms[k].extinct { self.realms[k].treasury.push(it); }
        }
        if items.is_empty() { return; }
        let best = *items.iter().max_by_key(|&&i| self.items[i].rarity).unwrap();
        // a child heir does not carry their inheritance: a guardian keeps it until they come of age
        if heir != NONE && !self.chars[heir].adult(self.year) {
            let g = self.guardian_of(heir, c);
            for &it in &items {
                self.items[it].held_for = heir;
                self.items[it].keeper = g;
                if !self.chars[heir].in_trust.contains(&it) { self.chars[heir].in_trust.push(it); }
                if g != NONE { if !self.chars[g].items.contains(&it) { self.chars[g].items.push(it); } } else if k < self.realms.len() && !self.realms[k].extinct { self.realms[k].treasury.push(it); }
            }
            if self.items[best].rarity >= 4 {
                let kn = if g != NONE { self.who(g) } else { "the crown".into() };
                let t = format!("{} passes from {} to their heir {}, a child; {kn} keeps it until they come of age.", self.items[best].label(), self.chars[c].name, self.chars[heir].name);
                self.say(t, true);
            }
            return;
        }
        if heir != NONE {
            for &it in &items { self.give_item(heir, it, true); }
            if self.items[best].rarity >= 4 {
                let rel = if self.chars[c].children.contains(&heir) { "child" } else if self.chars[c].spouse == heir { "spouse" } else { "kin" };
                let t = format!("{} passes from {} to their {rel}, {}.", self.items[best].label(), self.chars[c].name, self.who(heir));
                self.say(t, false);
            }
        } else if k < self.realms.len() && !self.realms[k].extinct {
            self.realms[k].treasury.extend(items);
        } else if !self.ruins.is_empty() {
            let r = self.rng.integers(0, self.ruins.len() as i64) as usize;
            self.ruins[r].loot.extend(items);
        }
    }
}
