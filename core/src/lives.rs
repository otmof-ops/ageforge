//! Lives: where a person comes from, what they want, and the story of their life.
//!
//! Every child born gets an origin from the real circumstances of their birth: a famine winter, a plague year, a
//! war, a ruling house, a hero for a parent, a temple, a forge, a merchant's house, a harbour, the poorest street,
//! or an ordinary hearth; losing a parent while young is added when it happens. At coming of age the origin and
//! the character's nature settle an ambition (glory, wealth, knowledge, faith, power, love, home, or vengeance on
//! whoever orphaned them), and the ambition steers real choices: taking up the hero's calling, the trade they
//! follow, plotting, writing, founding faiths. Fulfilled or not, it shows in their happiness.
//!
//! A tracked character keeps a life log, written from their own life rather than from any line that happens to
//! share their name. A child's inheritance is held in trust by a guardian until they come of age.

use crate::people::*;
use crate::state::State;
use crate::tables::*;

pub const AMBITIONS: [(&str, &str); 8] = [
    ("glory", "glory, a name the songs will keep"), ("wealth", "wealth, never to be hungry again"),
    ("knowledge", "knowledge, to understand how the world works"), ("faith", "the faith, and a life given to it"),
    ("power", "power, a seat among those who rule"), ("love", "love, and a family of their own"),
    ("home", "a quiet life, home and hearth"), ("vengeance", "vengeance on the one who took their parent"),
];

/// What each origin means, for menus and for stories.
pub fn origin_words(o: &str) -> &'static str {
    match o {
        "hearth" => "raised at an ordinary hearth", "hunger" => "born in a hungry year", "plague" => "born while sickness walked the streets",
        "war" => "born into a war", "highborn" => "born into a ruling house", "heroblood" => "a hero's child", "temple" => "raised in a temple",
        "forge" => "raised at a craftsman's workbench", "trade" => "raised in a merchant's house", "sea" => "raised within sound of the harbour",
        "poor" => "raised among the poorest", "founder" => "of the first tribes", _ => "come from beyond the settled lands",
    }
}

pub const ORIGINS: [&str; 13] = ["hearth", "hunger", "plague", "war", "highborn", "heroblood", "temple", "forge", "trade", "sea", "poor", "founder", "wanderer"];

const LIFE_CAP: usize = 600;

impl State {
    /// A line in a tracked character's life story.
    pub fn life(&mut self, c: usize, cat: &'static str, text: String) {
        if c >= self.chars.len() || !self.chars[c].tracked { return; }
        let y = self.year;
        let l = &mut self.chars[c].journal;
        if l.last().is_some_and(|x| x.0 == y && x.2 == text) { return; }
        l.push((y, cat, text));
        if l.len() > LIFE_CAP { l.remove(1); } // keep the origin story at the head
    }

    fn town_name(&self, t: usize) -> String { if t != NONE && t < self.towns.len() { self.towns[t].name.clone() } else { "the wilds".into() } }

    /// Where a newborn comes from, read from the world as it is the year they are born.
    pub fn assign_origin(&mut self, kid: usize) {
        let y = self.year;
        let (dad, mum) = (self.chars[kid].father, self.chars[kid].mother);
        let home = [mum, dad].iter().copied().filter(|&p| p != NONE).map(|p| self.chars[p].home).find(|&h| h != NONE && self.towns[h].alive)
            .unwrap_or_else(|| { let k = self.chars[kid].realm; if k < self.realms.len() { self.capital_of(k) } else { NONE } });
        self.chars[kid].home = home;
        self.chars[kid].birthplace = home;
        let parents: Vec<usize> = [dad, mum].into_iter().filter(|&p| p != NONE).collect();
        let k = self.chars[kid].realm;
        let prof = |s: &State, want: &[&str]| parents.iter().copied().find(|&p| s.chars[p].profession.is_some_and(|x| want.contains(&x)));
        let (origin, circ): (&'static str, String) = if parents.iter().any(|&p| self.chars[p].ruled.is_some()) {
            ("highborn", format!(", into the ruling house of {}", if k < self.realms.len() { self.realms[k].name.clone() } else { "a fallen realm".into() }))
        } else if let Some(p) = parents.iter().copied().find(|&p| self.chars[p].hero && self.chars[p].renown > 300.0) {
            ("heroblood", format!(", to {}, a hero of renown", self.chars[p].name))
        } else if home != NONE && self.towns[home].famine > 0.05 {
            ("hunger", " in a hungry year, when the granaries ran dry".to_string())
        } else if home != NONE && self.towns[home].sick_now > 0.05 {
            ("plague", " while sickness walked the streets".to_string())
        } else if k < self.realms.len() && home != NONE && (self.towns[home].occupied_by != NONE || self.war[k].iter().any(|&w| w) && self.towns[home].threat > 0.3) {
            ("war", format!(" while {} was at war", self.realms[k].name))
        } else if let Some(p) = prof(self, &["priest"]) {
            ("temple", format!(", and raised in the temple where {} served", self.chars[p].name))
        } else if let Some(p) = prof(self, &["smith", "enchanter", "alchemist", "shipwright"]) {
            ("forge", format!(", and raised at the workbench of {} the {}", self.chars[p].name, self.chars[p].profession.unwrap_or("smith")))
        } else if let Some(p) = prof(self, &["merchant"]) {
            ("trade", format!(", in the house of {} the merchant", self.chars[p].name))
        } else if home != NONE && self.towns[home].port {
            ("sea", ", within sound of the harbour".to_string())
        } else if home != NONE && self.towns[home].poverty > 0.45 {
            ("poor", ", among the poorest of the town".to_string())
        } else { ("hearth", String::new()) };
        let nth = |n: usize| ["first", "second", "third", "fourth", "fifth", "sixth", "seventh"].get(n).copied().unwrap_or("youngest");
        let order = if mum != NONE { self.chars[mum].children.iter().position(|&x| x == kid).unwrap_or(0) } else { 0 };
        let of = match (dad, mum) {
            (NONE, NONE) => String::new(),
            (d, NONE) => format!(", the {} child of {}", nth(order), self.chars[d].name),
            (NONE, m) => format!(", the {} child of {}", nth(order), self.chars[m].name),
            (d, m) => format!(", the {} child of {} and {}", nth(order), self.chars[d].name, self.chars[m].name),
        };
        self.chars[kid].origin = origin;
        self.chars[kid].origin_story = format!("Born in {} in year {y}{circ}{of}.", self.town_name(home));
    }

    /// Those who come into the world grown (founders, wanderers, the newly arrived): an origin from what is known.
    pub fn ensure_origin(&mut self, c: usize) {
        if !self.chars[c].origin.is_empty() { return; }
        let home = self.chars[c].home;
        let (origin, story) = if self.chars[c].born <= 0 {
            ("founder", format!("Of the first tribes, from the days before the realms; born about year {}.", self.chars[c].born))
        } else {
            ("wanderer", format!("Came to {} from beyond the settled lands; born about year {}, of parents no one remembers.", self.town_name(home), self.chars[c].born))
        };
        self.chars[c].origin = origin;
        self.chars[c].origin_story = story;
    }

    /// A child whose parent dies young: orphaned, and if the parent was killed, it is not forgotten.
    pub fn on_parent_death(&mut self, parent: usize, by: usize) {
        let y = self.year;
        let cause = self.chars[parent].cause.clone().unwrap_or_default();
        let pn = self.chars[parent].name.clone();
        for kid in self.chars[parent].children.clone() {
            if !self.chars[kid].alive { continue; }
            let rel = if self.chars[parent].female { "mother" } else { "father" };
            self.life(kid, "heart", format!("Their {rel}, {pn}, {cause}."));
            if self.chars[kid].adult(y) { continue; }
            let age = self.chars[kid].age(y);
            if by != NONE && by != kid { self.chars[kid].orphaned_by = by; }
            let other = if self.chars[parent].female { self.chars[kid].father } else { self.chars[kid].mother };
            let alone = other == NONE || !self.chars[other].alive;
            let line = if alone { format!(" Orphaned at {age}, when {pn} {cause}.") } else { format!(" Lost their {rel} at {age}: {pn} {cause}.") };
            if self.chars[kid].origin_story.len() < 600 { self.chars[kid].origin_story.push_str(&line); }
            if alone && self.chars[kid].origin != "highborn" { self.chars[kid].orphan = true; }
        }
    }

    /// Coming of age: the ambition their life will turn on.
    pub fn choose_ambition(&mut self, c: usize) {
        self.ensure_origin(c);
        let ch = &self.chars[c];
        let mut w = [1.0f64; 8]; // glory wealth knowledge faith power love home vengeance
        let add = |w: &mut [f64; 8], i: usize, x: f64| w[i] += x;
        match ch.origin {
            "hunger" => { add(&mut w, 1, 3.0); add(&mut w, 6, 2.0) }
            "plague" => { add(&mut w, 3, 2.0); add(&mut w, 2, 2.0) }
            "war" => { add(&mut w, 0, 3.0); add(&mut w, 6, 1.0) }
            "highborn" => { add(&mut w, 4, 4.0); add(&mut w, 0, 1.0) }
            "heroblood" => add(&mut w, 0, 4.0),
            "temple" => add(&mut w, 3, 5.0),
            "forge" => { add(&mut w, 1, 2.0); add(&mut w, 2, 2.0) }
            "trade" => add(&mut w, 1, 4.0),
            "sea" => { add(&mut w, 0, 1.0); add(&mut w, 1, 2.0) }
            "poor" => { add(&mut w, 1, 3.0); add(&mut w, 4, 1.0) }
            _ => { add(&mut w, 6, 3.0); add(&mut w, 5, 2.0) }
        }
        if ch.orphan { add(&mut w, 6, 1.5); add(&mut w, 4, 1.0); }
        for (t, i, x) in [("ambitious", 4, 3.0), ("ambitious", 0, 2.0), ("greedy", 1, 3.0), ("zealous", 3, 4.0), ("lustful", 5, 2.0), ("gregarious", 5, 1.5),
                          ("content", 6, 3.0), ("humble", 6, 2.0), ("brave", 0, 2.0), ("wrathful", 0, 1.5), ("diligent", 2, 2.0), ("lazy", 6, 2.0),
                          ("compassionate", 5, 1.5), ("cynical", 1, 1.0)] {
            if ch.has_trait(t) { add(&mut w, i, x); }
        }
        if ch.has(gene("genius")) || ch.has(gene("artistic")) { add(&mut w, 2, 3.0); }
        let killer = ch.orphaned_by;
        if killer != NONE && self.chars[killer].alive {
            w[7] = 6.0 + if ch.has_trait("vengeful") { 8.0 } else { 0.0 } - if ch.has_trait("forgiving") { 5.0 } else { 0.0 };
        } else { w[7] = 0.0; }
        let pick = self.rng.weighted(&w);
        let (amb, phrase) = AMBITIONS[pick];
        self.chars[c].ambition = amb;
        if amb == "vengeance" {
            let lost = self.chars[c].origin_story.split(" Orphaned at ").nth(1).and_then(|s| s.split(", when ").nth(1)).and_then(|s| s.split(' ').next()).unwrap_or("their parent").to_string();
            let y = self.year;
            if !self.chars[c].vendettas.iter().any(|v| v.0 == killer) { self.chars[c].vendettas.push((killer, y, lost)); }
        }
        let trained = self.chars[c].trained_in.map(|s| format!(", trained in {}", SKILLS[s])).unwrap_or_default();
        self.life(c, "lives", format!("Comes of age{trained}, and sets their heart on {phrase}."));
    }

    /// How much an ambition pushes a decision: 1.0 is no push.
    pub fn ambition_bias(&self, c: usize, decision: &str) -> f64 {
        let a = self.chars[c].ambition;
        match (decision, a) {
            ("hero", "glory") => 2.5, ("hero", "vengeance") => 1.5, ("hero", "power") => 1.2, ("hero", "home") => 0.4, ("hero", "love") => 0.6,
            ("merchant", "wealth") => 3.0, ("smith", "wealth") => 1.5, ("priest", "faith") => 3.0, ("alchemist", "knowledge") => 2.5,
            ("enchanter", "knowledge") => 2.5, ("captain", "glory") => 1.5, ("captain", "wealth") => 1.5,
            ("plot", "power") => 2.5, ("plot", "home") => 0.4, ("plot", "love") => 0.5,
            ("faith", "faith") => 3.0, ("works", "knowledge") => 1.6, ("works", "faith") => 1.2, ("marry", "love") => 1.6, ("marry", "home") => 1.3,
            _ => 1.0,
        }
    }

    /// The trade a grown person without one chooses, if their ambition points at one.
    pub fn chosen_trade(&self, members: &[usize], pname: &str, y: i64) -> Option<usize> {
        members.iter().copied().find(|&c| {
            let cc = &self.chars[c];
            cc.profession.is_none() && !cc.hero && cc.adult(y) && cc.ruled.is_none() && self.ambition_bias(c, pname) >= 2.5
        })
    }

    /// Once a year: ambitions fulfilled or soured, and a tracked life's ten-year summaries.
    pub fn lives_year(&mut self) {
        let y = self.year;
        for c in self.living.clone() {
            if self.chars[c].alive && self.chars[c].origin.is_empty() { self.ensure_origin(c); }
            let ch = &self.chars[c];
            if !ch.alive || ch.ambition.is_empty() || ch.ambition_met.is_some() { continue; }
            let met = match ch.ambition {
                "glory" => ch.renown >= 1500.0,
                "wealth" => ch.gold >= 400.0,
                "knowledge" => ch.fame.iter().any(|f| matches!(f.0, "scholar" | "artist") && f.1 > 0.0) || ch.level >= 30,
                "faith" => ch.profession == Some("priest") || ch.fame.iter().any(|f| f.0 == "prophet"),
                "power" => ch.ruled.is_some() || !ch.duchies.is_empty() || (ch.realm < self.realms.len() && self.realms[ch.realm].council.contains(&c)),
                "love" => ch.spouse != NONE && self.chars[ch.spouse].alive && ch.loves.iter().any(|l| l.0 == ch.spouse),
                "home" => ch.age(y) >= ADULT[ch.people] as i64 + 35 && ch.home != NONE && self.towns[ch.home].alive,
                "vengeance" => ch.orphaned_by != NONE && !self.chars[ch.orphaned_by].alive,
                _ => false,
            };
            if met {
                self.chars[c].ambition_met = Some(y);
                self.chars[c].stress = (self.chars[c].stress - 40.0).max(0.0);
                let how = match self.chars[c].ambition {
                    "vengeance" => { let k = self.chars[c].orphaned_by; format!("{} is dead. The debt is paid.", self.chars[k].name) }
                    "glory" => "Their name is in the songs now.".into(),
                    "wealth" => "They will never be hungry again.".into(),
                    "power" => "They sit among those who rule.".into(),
                    "love" => "They have the family they wanted.".into(),
                    "home" => "They have grown old in peace, at home.".into(),
                    "faith" => "Their life belongs to the faith.".into(),
                    _ => "They have learned what they set out to learn.".into(),
                };
                self.life(c, "lives", format!("Ambition fulfilled: {how}"));
            } else {
                // a life running out with the dream unmet sours
                let ch = &self.chars[c];
                if ch.age(y) as f64 > ch.life * 0.75 && self.rng.random() < 0.1 {
                    self.add_stress(c, 10.0, "a dream gone unmet");
                }
            }
        }
        // ten-year summaries for the tracked
        for c in self.living.clone() {
            let ch = &self.chars[c];
            if !ch.alive || !ch.tracked { continue; }
            let age = ch.age(y);
            if age <= 0 || age % 10 != 0 { continue; }
            let cls = ch.classes.last().map(|x| CLASSES[x.0].name).unwrap_or("Commoner");
            let wed = if ch.spouse != NONE && self.chars[ch.spouse].alive { format!("married to {}", self.chars[ch.spouse].name) } else { "unmarried".into() };
            let kids = ch.children.iter().filter(|&&k| self.chars[k].alive).count();
            let line = format!("At {age}: level {} {cls}, renown {:.0}, {wed}, {kids} child{} living, {:.0} gold; health {:.0}, stress {:.0}.",
                               ch.level, ch.renown, if kids == 1 { "" } else { "ren" }, ch.gold, ch.health, ch.stress);
            self.life(c, "lives", line);
        }
    }

    /// Tracking starts: the life so far, rebuilt from what the world knows.
    pub fn begin_life_log(&mut self, c: usize) {
        self.ensure_origin(c);
        let y = self.year;
        let ch = &self.chars[c];
        // the birth as it was; losses that came later get their own lines below
        let birth = ch.origin_story.split(" Orphaned at ").next().unwrap_or("").split(" Lost their ").next().unwrap_or("").to_string();
        let mut past: Vec<(i64, &'static str, String)> = vec![(ch.born, "lives", birth)];
        for &k in &ch.children { past.push((self.chars[k].born, "heart", format!("A child, {}, is born.", self.chars[k].name))); }
        for p in [ch.father, ch.mother] {
            if p != NONE { if let Some(d) = self.chars[p].died { if d <= y {
                let rel = if self.chars[p].female { "mother" } else { "father" };
                past.push((d, "heart", format!("Their {rel}, {}, {}.", self.chars[p].name, self.chars[p].cause.clone().unwrap_or_else(|| "died".into()))));
            } } }
        }
        for t in &ch.titles { past.push((y, "politics", format!("Holds {t}."))); }
        if !ch.ambition.is_empty() {
            let phrase = AMBITIONS.iter().find(|a| a.0 == ch.ambition).map(|a| a.1).unwrap_or("");
            let adult_at = ch.born + ADULT[ch.people] as i64;
            past.push((adult_at.min(y), "lives", format!("Came of age with their heart set on {phrase}.")));
        }
        past.sort_by_key(|x| x.0);
        let age = ch.age(y);
        past.push((y, "god", format!("The gods turn their eyes on them, aged {age}.")));
        self.chars[c].journal = past;
        self.chars[c].tracked = true;
    }

    /// Who keeps a child's inheritance until they come of age: the living parent, else the eldest of their house.
    pub fn guardian_of(&self, kid: usize, not: usize) -> usize {
        let y = self.year;
        for p in [self.chars[kid].mother, self.chars[kid].father] {
            if p != NONE && p != not && self.chars[p].alive && self.chars[p].adult(y) { return p; }
        }
        let h = self.chars[kid].house;
        if h == NONE { return NONE; }
        self.living.iter().copied().filter(|&i| i != not && self.chars[i].alive && self.chars[i].house == h && self.chars[i].adult(y))
            .min_by_key(|&i| self.chars[i].born).unwrap_or(NONE)
    }

    /// Items held in trust: handed to their heir at coming of age.
    pub fn take_up_trust(&mut self, c: usize) {
        let trust = std::mem::take(&mut self.chars[c].in_trust);
        for it in trust {
            if self.items[it].held_for != c { continue; }
            let keeper = self.items[it].keeper;
            let mut found = false;
            if keeper != NONE && self.chars[keeper].items.contains(&it) {
                self.chars[keeper].items.retain(|&x| x != it);
                found = true;
            } else {
                for k in self.realms.iter_mut() { if let Some(p) = k.treasury.iter().position(|&x| x == it) { k.treasury.remove(p); found = true; break; } }
            }
            self.items[it].held_for = NONE;
            self.items[it].keeper = NONE;
            if !found { continue; }
            self.give_item(c, it, true);
            let (lb, kn) = (self.items[it].label(), if keeper != NONE { self.chars[keeper].name.clone() } else { "the crown".into() });
            self.life(c, "heroes", format!("Comes into their inheritance: {lb}, kept for them by {kn}."));
        }
    }
}
