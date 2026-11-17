//! Works that outlast their makers: art, books, heirlooms and monuments.
//!
//! Only the gifted make them. A painting, a carving, a song or a book needs a maker born `artistic` (about one birth
//! in seventy, and it runs in families); a monument needs a realm rich enough to pay for it and an artistic
//! master-builder among its people. Heirlooms are what the great leave behind. Every work records a real event of
//! the world, and keeps its own history as it changes hands, is copied, carried off as plunder, lost in a burning
//! town, or found again in a ruin.

use crate::people::*;
use crate::state::State;
use crate::tables::*;

pub struct Work {
    pub id: usize,
    pub kind: &'static str, // painting, carving, tapestry, song, epic, chronicle, treatise, monument, heirloom
    pub title: String,
    pub maker: usize,
    pub year: i64,
    pub subject: String,
    pub quality: f64,  // 0-100: what it is worth to the eye and the ear
    pub realm: usize,  // the realm that holds it (NONE when lost)
    pub town: usize,   // where it stands or is kept (NONE: at court, or carried by its holder)
    pub house: usize,  // for heirlooms, the house it belongs to
    pub holder: usize, // for heirlooms, who carries it now
    pub hero: usize,   // for epics, the hero sung
    pub sector: usize, // for treatises, the trade it teaches (index of the town's sectors)
    pub faith: usize,  // for holy books, the faith they carry
    pub field: &'static str, // for natural philosophy: medicine, agronomy, astronomy, mathematics, engineering, alchemy
    pub copies: Vec<usize>, // for books: the towns holding a copy
    pub lost: Option<i64>,
    pub ruin: usize,   // where a lost work lies, if anywhere
    pub history: Vec<(i64, String)>,
}

/// The arts each people favours, most favoured first.
const ARTS: [[&str; 3]; 4] = [["painting", "tapestry", "song"], ["song", "painting", "carving"], ["carving", "song", "tapestry"], ["song", "carving", "tapestry"]];
const SECTORS: [&str; 7] = ["farming", "mining", "craft", "trade", "worship", "service", "the arcane"];

impl State {
    fn quality_of(&mut self, c: usize) -> f64 {
        let y = self.year;
        let ch = &self.chars[c];
        let practice = (ch.age(y) - ADULT[ch.people] as i64).max(0) as f64;
        let base = 2.0 * ch.attr(4, y) + 2.5 * ch.attr(5, y) + 8.0 * (practice / 15.0).min(3.0)
            + if ch.has(gene("genius")) { 12.0 } else { 0.0 } + if ch.has_trait("diligent") { 6.0 } else { 0.0 }
            - if ch.has_trait("lazy") { 8.0 } else { 0.0 };
        (base + self.rng.normal(0.0, 10.0)).clamp(1.0, 100.0)
    }

    /// Something worth remembering, from what this world has actually lived through: (title stem, the event told).
    fn subject_for(&mut self, k: usize, maker: usize) -> (String, String, usize) {
        let y = self.year;
        let rn = self.realms[k].short.clone();
        let mut opts: Vec<(String, String, usize)> = vec![];
        for b in self.battles_fought.iter().rev().take(60) {
            if y - b.0 > 60 { break; }
            if b.2 == rn || b.3 == rn {
                opts.push((format!("the Battle of {}", b.1), format!("the battle of {} in year {}, where {} beat {} and {} fell", b.1, b.0, b.2, b.3, b.6), NONE));
            }
        }
        for r in self.realms[k].reigns.iter().rev().take(3) {
            if let (Some(e), Some(how)) = (r.end, r.how.as_ref()) {
                if y - e <= 80 { opts.push((format!("the passing of {}", self.chars[r.id].name), format!("the death of {} in year {} ({})", self.chars[r.id].name, e, how), NONE)); }
            }
        }
        for f in self.famines.iter().rev().take(5) {
            if y - f.0 <= 50 && f.2 > 3000.0 { opts.push((format!("the hunger of {}", f.0), format!("the famine of year {}, when some {} starved", f.0, f.2 as i64), NONE)); }
        }
        for c in self.calamity_log.iter().rev().take(4) {
            let word = if c.3.contains("unbeaten") || c.3.contains("slumber") { "slumber" } else { "fall" };
            if y - c.2 <= 100 { opts.push((format!("the {word} of {}", c.0), format!("the end of {} in year {}: {}", c.0, c.2, c.3), NONE)); }
        }
        for d in self.dungeon_log.iter().rev().take(10) {
            if y - d.0 <= 60 && self.chars[d.1].realm == k { opts.push((format!("the descent into {}", d.2), format!("the clearing of {} by {} in year {}, who slew {}", d.2, self.chars[d.1].name, d.0, d.3), d.1)); }
        }
        let heroes: Vec<usize> = self.living.iter().copied().filter(|&h| self.chars[h].alive && self.chars[h].hero && self.chars[h].realm == k && self.chars[h].renown > 600.0).collect();
        for h in heroes.into_iter().take(3) { opts.push((format!("the deeds of {}", self.chars[h].name), format!("the deeds of {}, renowned across {}", self.chars[h].name, self.realms[k].name), h)); }
        if opts.is_empty() {
            let r = self.realms[k].ruler;
            if r != NONE && r != maker { return (format!("the likeness of {}", self.chars[r].name), format!("{} as they were in year {}", self.chars[r].name, y), NONE); }
            let home = self.realms[k].home_land;
            let ln = self.land_name(home);
            return (format!("the hills of {}", ln), format!("the land of {} as the maker saw it", ln), NONE);
        }
        opts.swap_remove(self.rng.integers(0, opts.len() as i64) as usize)
    }

    fn add_work(&mut self, kind: &'static str, title: String, maker: usize, subject: String, quality: f64, realm: usize, town: usize) -> usize {
        let id = self.works.len();
        let y = self.year;
        self.works.push(Work { id, kind, title, maker, year: y, subject, quality, realm, town, house: NONE, holder: NONE, hero: NONE,
                               sector: usize::MAX, faith: NONE, field: "", copies: vec![], lost: None, ruin: NONE, history: vec![] });
        id
    }

    fn note(&mut self, w: usize, text: String) { let y = self.year; self.works[w].history.push((y, text)); }

    pub fn works_year(&mut self) {
        let y = self.year;
        self.art_year(y);
        self.holy_books_year();
        self.monuments_year(y);
        self.heirlooms_year(y);
        self.fates_year(y);
    }

    /// The gifted at work: art, songs and books.
    fn art_year(&mut self, y: i64) {
        let g = gene("artistic");
        let genius = gene("genius");
        let makers: Vec<usize> = self.living.iter().copied().filter(|&i| {
            let c = &self.chars[i];
            c.alive && (c.has(g) || c.has(genius) && c.attr(4, y) >= 18.0) && c.adult(y) && c.jailed_until <= y && !c.undead && c.realm < self.realms.len()
        }).collect();
        for c in makers {
            // a working life: the gifted finish something worth keeping once in forty or fifty years, a master more often
            if self.rng.random() > (0.015 + 0.0008 * self.chars[c].attr(5, y)) * self.ambition_bias(c, "works") { continue; }
            let k = self.chars[c].realm;
            if self.realms[k].extinct { continue; }
            let p = self.chars[c].people;
            let learned = self.chars[c].attr(4, y);
            let writing = self.cultures[p].era >= 1;
            let gifted = self.chars[c].has(g);
            let devout = self.chars[c].profession == Some("priest") || self.chars[c].has_trait("zealous");
            let kind = if !gifted {
                // a learned genius without the artist's gift writes only what can be reasoned out
                if !writing || self.rng.random() > 0.4 { continue; }
                "philosophy"
            } else if writing && devout && self.rng.random() < 0.5 {
                "theology"
            } else if writing && learned >= 16.0 && self.rng.random() < 0.15 {
                "philosophy"
            } else if writing && learned >= 14.0 && self.rng.random() < 0.35 {
                if self.rng.random() < 0.4 { "treatise" } else if self.rng.random() < 0.5 { "chronicle" } else { "epic" }
            } else {
                let arts = ARTS[p];
                arts[self.rng.weighted(&[0.55, 0.3, 0.15])]
            };
            // natural philosophy turns on what its author knows best
            let field = if kind == "philosophy" {
                let sk = &self.chars[c].skills;
                let top = (0..sk.len()).max_by(|&a, &b| sk[a].partial_cmp(&sk[b]).unwrap()).unwrap_or(0);
                match SKILLS[top] {
                    "Restoration" => "medicine", "Alchemy" | "Enchanting" => "alchemy", "Navigation" | "Seamanship" => "astronomy",
                    "Smithing" | "Heavy Armor" | "Block" => "engineering", "Mercantile" => "mathematics", "Beast Taming" => "agronomy",
                    _ => ["medicine", "agronomy", "astronomy", "mathematics", "engineering", "alchemy"][self.rng.integers(0, 6) as usize],
                }
            } else { "" };
            let mut q = self.quality_of(c);
            let (stem, subject, hero) = self.subject_for(k, c);
            // a patron: the crown pays for the best, and keeps them at court
            let r = self.realms[k].ruler;
            let court = self.realms[k].court.is_some();
            let fee = 6.0 + q * 0.6;
            let patron = r != NONE && r != c && self.realms[k].gold > 10.0 * fee && (court || q > 60.0) && self.rng.random() < 0.5;
            if patron { q = (q + 6.0).min(100.0); }
            if q < 50.0 { continue; } // lesser pieces are made, sold and forgotten; only what is remembered is kept
            let home = self.chars[c].home;
            let town = if patron { NONE } else if home != NONE && self.towns[home].alive { home } else { self.capital_of(k) };
            let mn = self.chars[c].name.clone();
            let st = title_case(&stem);
            let faith = self.chars[c].religion;
            let title = match kind {
                "theology" => {
                    let rl = &self.religions[faith.min(self.religions.len() - 1)];
                    let tenet = if rl.tenets.is_empty() { "faith" } else { rl.tenets[self.rng.integers(0, rl.tenets.len() as i64) as usize] };
                    match self.rng.integers(0, 3) {
                        0 => format!("Sermons on {}", title_case(tenet)),
                        1 => format!("The Teachings of {}", rl.deity),
                        _ => format!("The Life of {}", if rl.founder != NONE { self.chars[rl.founder].name.clone() } else { format!("the First Servant of {}", rl.deity) }),
                    }
                }
                "philosophy" => {
                    let opts: [&str; 3] = match field {
                        "medicine" => ["The Canon of Healing", "On Fevers and Their Cure", "Of the Humours of the Body"],
                        "agronomy" => ["On the Tilling of Fields", "Of Seed, Soil and Season", "On the Keeping of Herds"],
                        "astronomy" => ["On the Motions of the Stars", "Tables of the Wandering Lights", "Of the Calendar and the Moon"],
                        "mathematics" => ["The Book of Numbers and Measures", "On Proportion", "Of Reckoning and Figures"],
                        "engineering" => ["Of Arches, Engines and Walls", "On Mills and Waterwheels", "Of Bridges and Vaults"],
                        _ => ["On the Transmutation of Matter", "Of the Essences of Things", "On the Fire and the Vessel"],
                    };
                    opts[self.rng.integers(0, 3) as usize].to_string()
                }
                "treatise" => {
                    let best = (0..self.chars[c].skills.len()).max_by(|&a, &b| self.chars[c].skills[a].partial_cmp(&self.chars[c].skills[b]).unwrap()).unwrap_or(14);
                    format!("On {}", SKILLS[best])
                }
                "chronicle" => format!("The Chronicle of {}", title_case(&self.realms[k].name)),
                "epic" => format!("The Lay of {st}"),
                "song" => format!("A Song of {st}"),
                "carving" => format!("{} in Stone", cap_first(&st)),
                "tapestry" => format!("The Tapestry of {st}"),
                _ => cap_first(&st),
            };
            // a title already taken is known by its author's name, as such books were
            let title = if self.works.iter().any(|w| w.title == title) { format!("{title} of {mn}") } else { title };
            let subject = if kind == "theology" { format!("the teachings of {}", self.religions[faith.min(self.religions.len() - 1)].name) }
                else if kind == "philosophy" { format!("{field}, as its author reasoned it out") }
                else if kind == "treatise" { format!("the craft its author mastered") } else if kind == "chronicle" { format!("the history of {} up to year {y}", self.realms[k].name) } else { subject };
            let w = self.add_work(kind, title.clone(), c, subject, q, k, town);
            self.works[w].hero = if kind == "epic" { hero } else { NONE };
            self.works[w].faith = if kind == "theology" { faith } else { NONE };
            self.works[w].field = field;
            if kind == "philosophy" { self.works[w].sector = match field { "alchemy" => 6, "engineering" => 2, "mathematics" => 3, "medicine" => 4, _ => usize::MAX }; }
            if matches!(kind, "treatise" | "chronicle" | "epic" | "scripture" | "theology" | "philosophy") {
                let at = if town != NONE { town } else { self.capital_of(k) };
                if at != NONE { self.works[w].copies.push(at); }
                if kind == "treatise" {
                    let best = (0..self.chars[c].skills.len()).max_by(|&a, &b| self.chars[c].skills[a].partial_cmp(&self.chars[c].skills[b]).unwrap()).unwrap_or(14);
                    self.works[w].sector = match SKILLS[best] { "Smithing" | "Enchanting" | "Alchemy" => 2, "Mercantile" | "Speech" => 3, "Restoration" => 4,
                        "Destruction" | "Conjuration" | "Illusion" | "Alteration" => 6, "Seamanship" | "Navigation" => 3, "Beast Taming" => 0, _ => 5 };
                }
            }
            let how = if patron { format!("for {}", self.who(r)) } else { "of their own will".into() };
            self.note(w, format!("made by {mn} {how}"));
            if patron {
                self.realms[k].gold -= fee;
                self.chars[c].gold += fee;
                self.books.push((k, "patronage", -fee, title.clone(), c));
            }
            let fame = q * if patron { 1.5 } else { 1.0 };
            self.gain_fame(c, if kind == "philosophy" { "scholar" } else if kind == "theology" { "prophet" } else { "artist" }, fame);
            self.life(c, "culture", format!("Finishes {title}, a {kind}."));
            if kind == "epic" && hero != NONE && self.chars[hero].alive {
                // a hero sung is a hero remembered: the song carries their deeds into their line's legacy
                let top = self.chars[hero].legacy.iter().max_by(|a, b| a.1.partial_cmp(&b.1).unwrap()).map(|x| x.0).unwrap_or(0);
                self.gain_legacy(hero, LEGACY_KINDS[top], q * 3.0);
                self.chars[hero].renown += q;
            }
            if q >= 85.0 {
                let t = format!("**A masterwork.** {} finishes {}, a {} of {}.", self.who(c), title, kind, self.works[w].subject);
                self.say(t, false);
            } else if q >= 60.0 {
                let t = format!("{} finishes {} ({}).", self.who(c), title, kind);
                self.say(t, true);
            }
        }
        // books are copied by scribes where learning is kept, and travel along the roads of trade
        for w in 0..self.works.len() {
            if self.works[w].lost.is_some() || !matches!(self.works[w].kind, "treatise" | "chronicle" | "epic" | "scripture" | "theology" | "philosophy") { continue; }
            let holy = matches!(self.works[w].kind, "scripture" | "theology");
            let n = self.works[w].copies.len();
            if n == 0 || n >= if holy { 120 } else { 40 } { continue; }
            if self.rng.random() > 0.06 * (self.works[w].quality / 50.0) * if holy { 2.0 } else { 1.0 } { continue; }
            let from = self.works[w].copies[self.rng.integers(0, n as i64) as usize];
            if !self.towns[from].alive { continue; }
            let fp = self.towns[from].pos;
            let to = self.alive_towns_of(self.towns[from].realm).into_iter()
                .filter(|t| !self.works[w].copies.contains(t))
                .min_by_key(|&t| (self.towns[t].pos.0 as i64 - fp.0 as i64).abs() + (self.towns[t].pos.1 as i64 - fp.1 as i64).abs());
            if let Some(t) = to { self.works[w].copies.push(t); }
        }
    }

    /// A realm with something to remember, the revenue to pay for it and a master to design it raises a monument: a
    /// project of a decade or more, paid for year by year out of revenue (and loans), stalled when the crown is in
    /// trouble, and abandoned if it stalls too long.
    fn monuments_year(&mut self, y: i64) {
        let g = gene("artistic");
        // work on what is already building
        for p in (0..self.monument_works.len()).rev() {
            let (k, at, _, _, _, _, ref mut left, ref mut stalled) = self.monument_works[p];
            let pay = (self.realms[k].income * 0.15).min(*left);
            let alive = !self.realms[k].extinct && self.towns[at].alive && self.towns[at].realm == k;
            if alive && self.realms[k].gold > -0.5 * self.realms[k].income {
                *left -= pay;
                self.realms[k].gold -= pay;
                self.books.push((k, "monuments", -pay, String::new(), NONE));
            } else { *stalled += 1; }
            let (left, stalled) = (*left, *stalled);
            if !alive || stalled > 15 {
                let (k, at, _, title, _, _, _, _) = self.monument_works.swap_remove(p);
                if !self.realms[k].extinct { let t = format!("Work on {title} in {} is given up, half-built.", self.towns[at].name); self.say(t, true); }
                continue;
            }
            if left <= 0.5 {
                let (k, at, b, title, subject, q, _, _) = self.monument_works.swap_remove(p);
                let r = self.realms[k].ruler;
                let w = self.add_work("monument", title.clone(), b, subject.clone(), q, k, at);
                let bw = self.chars[b].name.clone();
                let rw = if r != NONE { self.who(r) } else { self.realms[k].name.clone() };
                self.note(w, format!("raised in {} to the design of {bw}, finished for {rw}", self.towns[at].name));
                if r != NONE { if let Some(l) = self.chars[r].legitimacy.as_mut() { *l = (*l + 3.0 + q / 25.0).min(100.0); } }
                let h = self.realms[k].house;
                if h != NONE { self.houses[h].prestige += q; }
                if self.chars[b].alive { self.gain_fame(b, "artist", q * 2.0); }
                let t = format!("**A monument.** {title} is finished in {}, to remember {subject}.", self.towns[at].name);
                self.say(t, q < 50.0);
            }
        }
        // new projects
        for k in 0..self.realms.len() {
            if self.realms[k].extinct || self.realms[k].ruler == NONE || self.monument_works.iter().any(|m| m.0 == k) { continue; }
            let cost = 120.0 + 15.0 * self.alive_towns_of(k).len() as f64;
            if self.realms[k].income < cost / 8.0 || self.realms[k].gold < 0.0 || self.rng.random() > 0.03 { continue; }
            let builder = self.living.iter().copied().filter(|&i| { let c = &self.chars[i]; c.alive && c.realm == k && c.has(g) && c.adult(y) })
                .max_by(|&a, &b| self.chars[a].attr(4, y).partial_cmp(&self.chars[b].attr(4, y)).unwrap());
            let Some(b) = builder else { continue };
            let (stem, subject, _) = self.subject_for(k, NONE);
            if stem.starts_with("the likeness") || stem.starts_with("the hills") { continue; } // nothing yet worth the stone
            let at = self.capital_of(k);
            if at == NONE { continue; }
            let q = self.quality_of(b);
            let shape = ["Pillar", "Arch", "Obelisk", "Statue", "Cairn", "Gate"][self.rng.integers(0, 6) as usize];
            let title = format!("The {} of {}", shape, title_case(&stem));
            let r = self.realms[k].ruler;
            let t = format!("{} orders {title} raised in {}, to the design of {}.", self.who(r), self.towns[at].name, self.chars[b].name);
            self.say(t, true);
            self.monument_works.push((k, at, b, title, subject, q, cost, 0));
        }
    }

    /// The great leave something behind: a blade, a ring, a banner, a journal, kept by their house from hand to hand.
    pub fn leave_heirloom(&mut self, c: usize) {
        let y = self.year;
        let ch = &self.chars[c];
        let notable = ch.renown >= 500.0 || ch.ruled.is_some_and(|r| r.2.is_none()) || ch.ruled.is_some() && ch.renown >= 150.0;
        if !notable || ch.house == NONE || self.rng.random() > 0.35 { return; }
        let heir = self.heir_of(c);
        if heir == NONE || !self.chars[heir].alive { return; }
        let held: Vec<usize> = self.works.iter().filter(|w| w.kind == "heirloom" && w.lost.is_none() && w.holder == c).map(|w| w.id).collect();
        if !held.is_empty() { return; } // they carried one already; it passes on its own
        let (name, house) = (self.chars[c].name.clone(), self.chars[c].house);
        let best = self.chars[c].items.iter().copied().max_by_key(|&i| self.items[i].rarity);
        let (title, item_note) = match best {
            Some(i) if self.items[i].rarity >= 3 => (format!("{}, heirloom of {}", self.items[i].name, self.houses[house].name), "the weapon they carried"),
            _ => {
                let what = ["signet ring", "war-banner", "journal", "cloak", "drinking horn", "seal", "helm"][self.rng.integers(0, 7) as usize];
                (format!("the {what} of {name}"), "a keepsake")
            }
        };
        let deeds = self.chars[c].titles.last().cloned().unwrap_or_else(|| format!("renown {:.0}", self.chars[c].renown));
        let k = self.chars[heir].realm;
        let w = self.add_work("heirloom", title.clone(), c, format!("{name}, {deeds}"), (self.chars[c].renown / 30.0).clamp(5.0, 100.0), k, NONE);
        self.works[w].house = house;
        self.works[w].holder = heir;
        self.life(heir, "heart", format!("Receives {title}, left to them by {name}."));
        self.works[w].year = y;
        let hw = self.who(heir);
        self.note(w, format!("{} ({item_note}) left by {name} to {hw}", title));
    }

    /// Heirlooms passing from hand to hand.
    fn heirlooms_year(&mut self, _y: i64) {
        for w in 0..self.works.len() {
            if self.works[w].kind != "heirloom" || self.works[w].lost.is_some() { continue; }
            let h = self.works[w].holder;
            if h == NONE || self.chars[h].alive { continue; }
            let house = self.works[w].house;
            let mut next = self.heir_of(h);
            if next == NONE || !self.chars[next].alive || (house != NONE && self.chars[next].house != house) {
                next = self.living.iter().copied().filter(|&i| self.chars[i].alive && self.chars[i].house == house).max_by(|&a, &b| self.chars[a].renown.partial_cmp(&self.chars[b].renown).unwrap()).unwrap_or(NONE);
            }
            if next == NONE {
                self.works[w].lost = Some(self.year);
                self.works[w].holder = NONE;
                let hn = if house != NONE { self.houses[house].name.clone() } else { "its house".into() };
                self.note(w, format!("lost when the last of {hn} died"));
                continue;
            }
            self.works[w].holder = next;
            self.works[w].realm = self.chars[next].realm;
            let t = format!("passed to {}", self.who(next));
            self.note(w, t);
            if self.chars[next].ruled.is_some_and(|r| r.2.is_none()) {
                if let Some(l) = self.chars[next].legitimacy.as_mut() { *l = (*l + 2.0).min(100.0); }
            }
        }
    }

    /// What becomes of works when towns change hands or burn, when realms fall, and when ruins are cleared.
    fn fates_year(&mut self, y: i64) {
        for w in 0..self.works.len() {
            let kind = self.works[w].kind;
            if kind == "heirloom" { continue; }
            if let Some(_) = self.works[w].lost {
                // lost works lie in the ruins of the town they burned with, until someone clears it
                let r = self.works[w].ruin;
                if r != NONE && self.ruins[r].conquered == Some(y) {
                    if let Some((_, hero, _)) = self.ruins[r].conquests.last().cloned() {
                        if hero < self.chars.len() && self.chars[hero].alive {
                            let k = self.chars[hero].realm;
                            self.works[w].lost = None;
                            self.works[w].ruin = NONE;
                            self.works[w].realm = k;
                            self.works[w].town = NONE;
                            if matches!(kind, "treatise" | "chronicle" | "epic" | "scripture" | "theology" | "philosophy") { let c = self.capital_of(k); if c != NONE { self.works[w].copies = vec![c]; } }
                            let (hw, rn, ti) = (self.who(hero), self.ruins[r].name.clone(), self.works[w].title.clone());
                            self.note(w, format!("found again in {rn} by {hw}"));
                            let t = format!("{hw} brings {ti} out of {rn}, lost since year {}.", self.works[w].history.iter().rev().find(|h| h.1.starts_with("lost")).map(|h| h.0).unwrap_or(y));
                            self.say(t, self.works[w].quality < 70.0);
                        }
                    }
                }
                continue;
            }
            if matches!(kind, "treatise" | "chronicle" | "epic" | "scripture" | "theology" | "philosophy") {
                let before = self.works[w].copies.len();
                let burned: Vec<usize> = self.works[w].copies.iter().copied().filter(|&t| !self.towns[t].alive).collect();
                self.works[w].copies.retain(|&t| self.towns[t].alive);
                if before > 0 && self.works[w].copies.is_empty() {
                    let t = burned[0];
                    let tn = self.towns[t].name.clone();
                    self.lose_in(w, t, format!("the last copy burned with {tn}"));
                }
                continue;
            }
            let t = self.works[w].town;
            let k = self.works[w].realm;
            if t == NONE {
                // kept at court: when the realm falls, the court's treasures scatter
                if k != NONE && self.realms[k].extinct {
                    self.works[w].lost = Some(y);
                    self.works[w].realm = NONE;
                    let rn = self.realms[k].name.clone();
                    self.note(w, format!("scattered when {rn} fell"));
                }
                continue;
            }
            if !self.towns[t].alive {
                let tn = self.towns[t].name.clone();
                if kind == "monument" {
                    if self.works[w].history.last().map(|h| !h.1.starts_with("stands among")).unwrap_or(true) {
                        self.note(w, format!("stands among the ruins of {tn}"));
                    }
                } else {
                    self.lose_in(w, t, format!("burned with {tn}"));
                }
                continue;
            }
            let now = self.towns[t].realm;
            if now != k {
                let (nn, tn) = (self.realms[now].name.clone(), self.towns[t].name.clone());
                let insult = k != NONE && self.works[w].subject.contains(&format!("beat {}", self.realms[now].short));
                if kind == "monument" && insult && self.rng.random() < 0.6 {
                    // a monument to their own defeat: the new masters pull it down
                    self.works[w].lost = Some(y);
                    self.note(w, format!("pulled down by {nn} when they took {tn}"));
                    let t2 = format!("{nn} pull down {} in {tn}, raised to their old defeat.", self.works[w].title);
                    self.say(t2, true);
                    continue;
                }
                if kind != "monument" && self.rng.random() < 0.4 {
                    self.works[w].town = NONE;
                    self.note(w, format!("carried off as plunder by {nn} from {tn}"));
                } else {
                    self.note(w, format!("passed to {nn} with {tn}"));
                }
                self.works[w].realm = now;
            }
        }
    }

    fn lose_in(&mut self, w: usize, town: usize, why: String) {
        let pos = self.towns[town].pos;
        self.works[w].lost = Some(self.year);
        self.works[w].realm = NONE;
        self.works[w].ruin = self.ruins.iter().position(|r| r.pos == pos).unwrap_or(NONE);
        self.note(w, why);
    }

    /// A new faith's words written down: the scripture its followers copy and carry.
    pub fn write_scripture(&mut self, founder: usize, faith: usize, town: usize) {
        let k = self.chars[founder].realm;
        let deity = self.religions[faith].deity.clone();
        let q = self.quality_of(founder).max(55.0);
        let w = self.add_work("scripture", format!("The Book of {deity}"), founder, format!("the revelation of {}", self.religions[faith].name), q, k, town);
        self.works[w].faith = faith;
        self.works[w].copies.push(town);
        let fname = self.chars[founder].name.clone();
        self.note(w, format!("set down from the words of {fname} in {}", self.towns[town].name));
    }

    /// Holy books draw their towns toward the faith they carry, a little each year, as they are read aloud.
    pub fn holy_books_year(&mut self) {
        let mut pulls: Vec<(usize, usize, f64)> = vec![];
        for w in &self.works {
            if w.lost.is_some() || w.faith == NONE || w.faith >= self.religions.len() || self.religions[w.faith].dead { continue; }
            for &t in &w.copies { if self.towns[t].alive { pulls.push((t, w.faith, 0.004 * w.quality / 100.0)); } }
        }
        for (t, f, a) in pulls { self.shift(t, f, a); }
    }

    /// Natural philosophy kept in each town, by field: (medicine, agronomy, astronomy, engineering), each capped at 2.
    pub fn learning_in(&self) -> Vec<[f64; 4]> {
        let mut out = vec![[0.0; 4]; self.towns.len()];
        for w in &self.works {
            if w.kind != "philosophy" || w.lost.is_some() { continue; }
            let i = match w.field { "medicine" => 0, "agronomy" => 1, "astronomy" => 2, "engineering" | "mathematics" => 3, _ => continue };
            for &t in &w.copies { out[t][i] = (out[t][i] + w.quality / 100.0).min(2.0); }
        }
        out
    }

    /// What a people's surviving natural philosophy adds to its research each year.
    pub fn science_points(&self, people: usize) -> f64 {
        self.works.iter().filter(|w| w.kind == "philosophy" && w.lost.is_none() && w.maker < self.chars.len() && self.chars[w.maker].people == people)
            .map(|w| w.quality / 100.0 * (1.0 + (w.copies.len() as f64).ln_1p())).sum::<f64>().min(12.0)
    }

    /// What the works add to a court: the eye of visitors (court grandeur).
    pub fn court_art(&self, k: usize) -> f64 {
        self.works.iter().filter(|w| w.realm == k && w.town == NONE && w.lost.is_none() && !matches!(w.kind, "heirloom" | "monument"))
            .map(|w| (w.quality / 12.0).min(8.0)).sum::<f64>().min(30.0)
    }

    /// Treatises kept in a town: how many teach each of its trades.
    pub fn treatises_in(&self) -> Vec<[f64; 7]> {
        let mut out = vec![[0.0; 7]; self.towns.len()];
        for w in &self.works {
            if !matches!(w.kind, "treatise" | "philosophy") || w.lost.is_some() || w.sector >= 7 { continue; }
            for &t in &w.copies { out[t][w.sector] += w.quality / 100.0; }
        }
        out
    }
}

/// "the passing of Eleth" -> "the Passing of Eleth": capitals on the words that carry the meaning.
fn title_case(s: &str) -> String {
    const SMALL: [&str; 9] = ["the", "of", "in", "and", "a", "an", "at", "by", "to"];
    s.split(' ').enumerate().map(|(i, w)| if i > 0 && SMALL.contains(&w) || i == 0 && w == "the" { w.to_string() } else { cap_first(w) }).collect::<Vec<_>>().join(" ")
}

fn cap_first(w: &str) -> String { let mut c = w.chars(); match c.next() { Some(f) => f.to_uppercase().collect::<String>() + c.as_str(), None => String::new() } }

pub fn sector_name(i: usize) -> &'static str { SECTORS.get(i).copied().unwrap_or("") }
