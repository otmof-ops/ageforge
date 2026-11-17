//! God mode: the acts, the character creator and the character editor, each a form whose every choice is a list.

use serde_json::{json, Value};
use world::lives::{origin_words, AMBITIONS, ORIGINS};
use world::people::NONE;
use world::state::State;
use world::tables::*;

use crate::forms::{Field, Form};

type Opt = (String, Value, String);

// ---------------------------------------------------------------- lists drawn from the world
pub fn realm_opts(st: &State) -> Vec<Opt> {
    let mut ks: Vec<usize> = (0..st.realms.len()).filter(|&k| !st.realms[k].extinct).collect();
    let towns = |k: usize| st.towns.iter().filter(|t| t.alive && t.realm == k).count();
    ks.sort_by_key(|&k| std::cmp::Reverse(towns(k)));
    ks.into_iter().map(|k| {
        let r = st.realms[k].ruler;
        let ruler = if r != NONE { crate::sheet::full_name(st, r) } else { "no ruler".into() };
        (format!("{} ({}, {} towns)", st.realms[k].name, PEOPLES[st.realms[k].people], towns(k)), json!(k), format!("Ruled by {ruler}. Gold {:.0}.", st.realms[k].gold))
    }).collect()
}
pub fn land_opts(st: &State, continents_only: bool) -> Vec<Opt> {
    st.map.lands.iter().filter(|l| !continents_only || l.kind == "continent").map(|l| (format!("{} ({}, {} cells)", l.name, l.kind, l.size), json!(l.id), String::new())).collect()
}
pub fn people_opts() -> Vec<Opt> {
    (0..4).map(|p| (PEOPLES[p].to_string(), json!(p), format!("Comes of age at {}, lives about {:.0} years; magic tier cap {}.", ADULT[p], LIFESPAN[p], TIER_CAP[p]))).collect()
}
pub fn town_opts(st: &State) -> Vec<Opt> {
    let mut ts: Vec<usize> = (0..st.towns.len()).filter(|&t| st.towns[t].alive).collect();
    ts.sort_by(|&a, &b| st.towns[b].pop.partial_cmp(&st.towns[a].pop).unwrap());
    ts.into_iter().map(|t| (format!("{} ({}, {:.0} people)", st.towns[t].name, st.realms[st.towns[t].realm].short, st.towns[t].pop), json!(t), String::new())).collect()
}
pub fn house_opts(st: &State) -> Vec<Opt> {
    let mut hs: Vec<usize> = (0..st.houses.len()).collect();
    hs.sort_by(|&a, &b| st.houses[b].prestige.partial_cmp(&st.houses[a].prestige).unwrap());
    hs.into_iter().take(300).map(|h| (format!("{} (prestige {:.0})", st.houses[h].name, st.houses[h].prestige), json!(h), String::new())).collect()
}
pub fn char_opts(st: &State, living: bool) -> Vec<Opt> {
    let mut cs: Vec<usize> = if living { st.living.iter().copied().filter(|&c| st.chars[c].alive).collect() } else { (0..st.chars.len()).collect() };
    cs.sort_by(|&a, &b| (st.chars[b].tracked, st.chars[b].renown).partial_cmp(&(st.chars[a].tracked, st.chars[a].renown)).unwrap());
    cs.into_iter().take(4000).map(|c| (char_label(st, c), json!(c), String::new())).collect()
}
pub fn char_label(st: &State, c: usize) -> String {
    let ch = &st.chars[c];
    let ruler = if ch.ruled.is_some_and(|r| r.2.is_none()) { " ♛" } else { "" };
    let tr = if ch.tracked { " ★" } else { "" };
    let life = if ch.alive { format!("{}", ch.age(st.year)) } else { format!("died {}", ch.died.unwrap_or(0)) };
    format!("{}{ruler}{tr} ({}, {life}, lv {}, #{c})", crate::sheet::full_name(st, c), PEOPLES[ch.people], ch.level)
}
fn class_opts(prestige: bool) -> Vec<Opt> {
    CLASSES.iter().filter(|c| prestige || !c.prestige).map(|c| {
        let b = c.branches.iter().filter(|b| !b.0.is_empty()).map(|b| b.0).collect::<Vec<_>>().join(", ");
        (c.name.to_string(), json!(c.name), format!("Body {:.1}, magic {:.1}, toughness {:.1}.{}", c.phys, c.magic, c.hp, if b.is_empty() { " A prestige class.".into() } else { format!(" Paths: {b}.") }))
    }).collect()
}
const STAT_NAMES: [&str; 6] = ["diplomacy", "martial", "stewardship", "intrigue", "learning", "lore"];
fn fx_words(fx: &[f64], names: &[&str]) -> String {
    let parts: Vec<String> = fx.iter().zip(names).filter(|(v, _)| **v != 0.0).map(|(v, n)| format!("{}{v} {n}", if *v > 0.0 { "+" } else { "" })).collect();
    if parts.is_empty() { "No effect on the stats; it shapes choices and moods.".into() } else { parts.join(", ") }
}
pub fn personality_opts() -> Vec<Opt> {
    PERSONALITY.iter().enumerate().map(|(i, p)| {
        let opp = PERSONALITY[i ^ 1];
        (format!("{p} (not {opp})"), json!(p), format!("{} Opposite of {opp}.", fx_words(&PERSONALITY_FX[i], &STAT_NAMES)))
    }).collect()
}
const BLOODLINE_WORDS: [(&str, &str); 11] = [
    ("starblood", "lifts the tier cap of their people by one, as far as the seventh tier and no further"),
    ("dragonblood", "breaks the level cap: past the hundredth level, to the hundred-and-thirtieth"),
    ("elderblood", "breaks the span of their people: they live three times as long"),
    ("titanblood", "breaks the prowess cap of their people, and strikes a third harder"),
    ("abyssblood", "breaks the talent cap: their magic is limited by training alone"),
    ("wolf-merged", "a wolf in the blood: keen, swift and fierce"), ("bear-merged", "a great bear in the blood: vast strength"),
    ("eagle-merged", "an eagle in the blood: far sight"), ("griffin-merged", "a griffin in the blood: strength and the high air"),
    ("wyvern-merged", "a wyvern in the blood: venom and wings"), ("dragon-merged", "a dragon in the blood: fire, long life, the bond dragons answer"),
];
const GENE_NAMES: [&str; 9] = ["diplomacy", "martial", "stewardship", "intrigue", "learning", "lore", "prowess", "talent", "lifespan"];
pub fn gene_opts(bloodlines: bool) -> Vec<Opt> {
    GENETIC.iter().enumerate().filter(|(i, g)| BLOODLINES.contains(i) == bloodlines && !g.starts_with("half-") && !matches!(**g, "tidebound" | "stoneborn" | "flameheart" | "skyborn" | "hollow-eye" | "deepwell"))
        .map(|(i, g)| {
            let mut fx = GENETIC_FX[i];
            fx[8] = if fx[8] != 1.0 { (fx[8] - 1.0) * 100.0 } else { 0.0 };
            let words = BLOODLINE_WORDS.iter().find(|b| b.0 == *g).map(|b| b.1.to_string()).unwrap_or_else(|| {
                if *g == "artistic" { "The gift to make: only the artistic make art, songs and books.".into() } else { fx_words(&fx, &GENE_NAMES).replace("lifespan", "% lifespan") }
            });
            (g.to_string(), json!(g), words)
        }).collect()
}
fn rarity_opts() -> Vec<Opt> {
    RARITY.iter().enumerate().skip(1).map(|(i, r)| (r.to_string(), json!(i), String::new())).collect()
}
fn yes(v: &Value) -> bool { v.as_bool().unwrap_or(false) }

// ---------------------------------------------------------------- the acts
pub struct Act { pub key: &'static str, pub label: &'static str, pub help: &'static str }

pub const ACTS: [Act; 17] = [
    Act { key: "create", label: "Create a character", help: "Design a person, CK3 or Oblivion style: people, calling, stats, traits, bloodlines, skills, origin and ambition." },
    Act { key: "edit", label: "Change a character", help: "Pick anyone, living or dead, and change their stats, traits, class, items, loves and fate." },
    Act { key: "wake_calamity", label: "Wake a calamity", help: "A continent-shaking horror rises on a continent of your choosing; or one that threatens the whole world." },
    Act { key: "loose_monster", label: "Loose monsters on a realm", help: "Trolls, wraiths, demons or a dragon fall on a realm's seat." },
    Act { key: "smite_town", label: "Smite a town", help: "Fire from the sky: the town is gone, a ruin left in its place." },
    Act { key: "send_plague", label: "Send a plague", help: "A sickness starts in a realm's seat and spreads along the roads." },
    Act { key: "famine", label: "Send famine", help: "The harvests of a realm fail for some years." },
    Act { key: "plenty", label: "Send plenty", help: "The harvests of a realm are blessed for some years." },
    Act { key: "grant_gold", label: "Grant gold", help: "Gold appears in a realm's treasury." },
    Act { key: "declare_war", label: "Start a war", help: "Set one realm against another." },
    Act { key: "impose_peace", label: "Impose peace", help: "End a war between two realms." },
    Act { key: "change_government", label: "Change a government", help: "Remake how a realm is ruled." },
    Act { key: "raise_era", label: "Raise a people's era", help: "Teach a people the next age's learning." },
    Act { key: "golden_age", label: "Grant a golden age", help: "Forty years in which a people's learning and arts flower." },
    Act { key: "break_realm", label: "Break a realm apart", help: "Tear a realm's holdings on one landmass away into a realm of their own." },
    Act { key: "curse_house", label: "Curse a house", help: "A blood curse on a house, passed to its children." },
    Act { key: "god_note", label: "Write in the chronicle", help: "Set down a line of your own in the chronicle." },
];

/// The form for an act, with its choices filled from the world as it is now. None for create and edit.
pub fn act_form(st: &State, key: &str) -> Option<Form> {
    let realm = |k: &str, l: &str| Field::choice(k, l, realm_opts(st), "Type to search the realms.");
    let go = Field::action("Do it", "Enter or F10 to do this.");
    let fields = match key {
        "wake_calamity" => vec![Field::choice("lid", "Continent", land_opts(st, false), "Where it wakes."), Field::toggle("world", "A world-ender", false, "Only a few calamities can threaten the whole world."), go],
        "loose_monster" => vec![Field::choice("kind", "Monster", ["troll", "wraith", "demon", "dragon"].iter().map(|m| (m.to_string(), json!(m), String::new())).collect(), ""), realm("kid", "Realm"), go],
        "smite_town" => vec![Field::choice("town", "Town", town_opts(st), "Largest first; type to search."), go],
        "send_plague" => vec![realm("kid", "Realm"), Field::toggle("great", "A great pandemic", true, "A great one can kill a third or more where it reaches."), go],
        "famine" => vec![realm("kid", "Realm"), Field::num("years", "Years", 3.0, 1.0, 30.0, 1.0, ""), go],
        "plenty" => vec![realm("kid", "Realm"), Field::num("years", "Years", 10.0, 1.0, 50.0, 1.0, ""), go],
        "grant_gold" => vec![realm("kid", "Realm"), Field::num("amount", "Gold", 1000.0, 1.0, 1e7, 100.0, ""), go],
        "declare_war" | "impose_peace" => vec![realm("a", "Realm"), realm("b", "Against"), go],
        "change_government" => vec![realm("kid", "Realm"), Field::choice("gov", "Government", ["tribal", "clan", "feudal", "republic", "theocracy", "administrative"].iter().map(|g| (g.to_string(), json!(g), String::new())).collect(), ""), go],
        "raise_era" | "golden_age" => vec![Field::choice("culture", "People", people_opts(), ""), go],
        "break_realm" => vec![realm("kid", "Realm"), Field::choice("lid", "Landmass", land_opts(st, false), "The realm's holdings here break away."), go],
        "curse_house" => vec![Field::choice("hid", "House", house_opts(st), "Most prestigious first."), go],
        "god_note" => vec![Field::text("text", "Words", "", "What the chronicle should say."), go],
        _ => return None,
    };
    let act = ACTS.iter().find(|a| a.key == key)?;
    let mut f = Form::new(act.label, fields);
    f.note = act.help.to_string();
    Some(f)
}

/// The act and its arguments as the world takes them.
pub fn act_args(key: &str, f: &Form) -> (String, serde_json::Map<String, Value>) {
    let mut a = f.args();
    let act = match key {
        "famine" => { a.insert("mult".into(), json!(0.35)); "set_harvest" }
        "plenty" => { a.insert("mult".into(), json!(1.6)); "set_harvest" }
        "god_note" => { let t = a.get("text").and_then(|v| v.as_str()).unwrap_or("").to_string(); a.insert("text".into(), json!(format!("**God.** {t}"))); "god_note" }
        k => k,
    };
    (act.to_string(), a)
}

// ---------------------------------------------------------------- the character creator
pub const ARCHETYPES: [(&str, &str); 12] = [
    ("Warrior", "A born fighter: strong, brave, trained in blade and armor."),
    ("Knight", "A sworn sword of a house: martial, just, at home in heavy armor."),
    ("Ranger", "A hunter of the wilds: keen-eyed, patient, a bow and a tracker's eye."),
    ("Thief", "A quiet hand: sneak, locks and pockets, deceitful and quick."),
    ("Wizard", "A student of the arcane: learned, spellborn, destruction and alteration."),
    ("Priest", "A servant of the gods: zealous, compassionate, speech and restoration."),
    ("Ruler-in-waiting", "Born to rule: diplomacy and stewardship, ambitious and just."),
    ("Scholar", "A mind for how the world works: a genius who writes natural philosophy."),
    ("Artist", "The gift to make: songs, paintings and books that outlast them."),
    ("Merchant", "A trader: greedy, gregarious, mercantile and silver-tongued."),
    ("Runesmith", "A master of the forge: smithing and enchanting, diligent and stubborn."),
    ("Hero of legend", "A marked hero: high level, brave, starblood in the veins. Overpowered by design."),
];

/// A small generator of its own, so designing a character never touches the world's dice.
pub struct Dice(u64);
impl Dice {
    pub fn new() -> Dice { Dice(std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_nanos() as u64).unwrap_or(7) | 1) }
    pub fn next(&mut self) -> u64 { self.0 ^= self.0 << 13; self.0 ^= self.0 >> 7; self.0 ^= self.0 << 17; self.0 }
    pub fn below(&mut self, n: usize) -> usize { (self.next() % n.max(1) as u64) as usize }
    pub fn range(&mut self, a: f64, b: f64) -> f64 { a + (b - a) * (self.next() % 10_000) as f64 / 10_000.0 }
}

pub fn random_name(d: &mut Dice, people: usize, female: bool) -> String {
    let [pm, pf, sm, sf] = FIRST[people.min(3)];
    let (p, s) = if female { (pf, sf) } else { (pm, sm) };
    format!("{}{}", p[d.below(p.len())], s[d.below(s.len())])
}

pub fn creator_form(st: &State) -> Form {
    let mut d = Dice::new();
    let realms = realm_opts(st);
    let first_realm = realms.first().map(|r| r.1.clone()).unwrap_or(json!(0));
    let people = first_realm.as_u64().map(|k| st.realms[k as usize].people).unwrap_or(0);
    let mut fields = vec![
        Field::header("Who they are"),
        Field::text("name", "Name", &random_name(&mut d, people, false), "Enter to type a name, or press r to roll one."),
        Field::choice("people", "People", people_opts(), "Humans, Elves, Dwarves or Orcs: each comes of age, ages and wields magic differently."),
        Field::choice("female", "Sex", vec![("man".into(), json!(false), String::new()), ("woman".into(), json!(true), String::new())], ""),
        Field::num("age", "Age", 20.0, 1.0, 900.0, 1.0, "Younger than their people's coming of age makes a child."),
        Field::choice("realm", "Realm", realms, "Where they are set down. Type to search."),
        Field::choice("origin", "Origin", ORIGINS.iter().map(|o| (o.to_string(), json!(o), origin_words(o).to_string())).collect(), "Where they come from: it shapes the ambition they are likely to form, and their story."),
        Field::choice("ambition", "Ambition", std::iter::once(("(let their life decide)".to_string(), Value::Null, "Chosen at coming of age from their origin and nature.".to_string()))
            .chain(AMBITIONS.iter().map(|a| (a.0.to_string(), json!(a.0), format!("They set their heart on {}. It steers their choices, and fulfilling it makes them content.", a.1)))).collect(), ""),
        Field::text("origin_story", "Their story", "", "Optional: a line of your own for where they come from. Left empty, one is written for them."),
        Field::header("Their calling"),
        Field::choice("cls", "Class", class_opts(false), "Their first class. Prestige classes are earned in play."),
        Field::num("level", "Level", 5.0, 1.0, 130.0, 1.0, "Most people never pass level 20; past 100 needs dragonblood."),
        Field::toggle("hero", "A hero", false, "Heroes take quests, delve dungeons and hunt monsters."),
        Field::num("gold", "Gold", 50.0, 0.0, 1e6, 10.0, ""),
        Field::header("Mind and body"),
    ];
    for (i, a) in ATTRS.iter().enumerate() {
        fields.push(Field::num(a, STAT_NAMES[i], 8.0, 0.0, 40.0, 1.0, &format!("{}: CK3-style stat; 5 is ordinary, 10 good, 15 excellent, 20 legendary.", STAT_NAMES[i])));
    }
    fields.push(Field::num("prowess", "Prowess", PROWESS_BASE[people], 0.0, 80.0, 1.0, "Bodily might in a fight."));
    fields.push(Field::num("talent", "Magical talent", TALENT_BASE[people], 0.0, 10.0, 0.5, "How deep their magic can go (0-10)."));
    fields.push(Field::header("Nature"));
    fields.push(Field::multi("personality", "Personality", personality_opts(), vec![json!("brave"), json!("diligent")], "Up to four traits. Opposites cancel: picking one removes its opposite."));
    fields.push(Field::multi("genetic", "Inborn traits", gene_opts(false), vec![], "Traits of the body and mind, inherited by their children."));
    fields.push(Field::multi("bloodlines", "Bloodlines (shown)", gene_opts(true), vec![], "Power bloodlines that break the limits of their people. Shown in them, passed on to children."));
    fields.push(Field::multi("latent", "Bloodlines (carried)", gene_opts(true), vec![], "Carried unseen: they may wake in them, or show in their descendants."));
    fields.push(Field::header("Skills (0-100)"));
    for s in SKILLS.iter() { fields.push(Field::num(&format!("skill:{s}"), s, 10.0, 0.0, 100.0, 5.0, "")); }
    fields.push(Field::header("Watching them"));
    fields.push(Field::toggle("track", "Track their life", true, "Keep their life story in the Tracked tab."));
    fields.push(Field::action("Create this character", "F10 or Enter: set them down in the world."));
    let mut f = Form::new("Create a character   ·   p presets · r roll name · R randomize all", fields);
    f.set("people", json!(people));
    f.note = "Design a person. Every list opens with Enter; ←→ change values.".into();
    f
}

/// Opposite personality traits and opposed genes cannot be held together; four personality traits at most.
pub fn tidy_traits(f: &mut Form) {
    let mut ps: Vec<Value> = f.get("personality").as_array().cloned().unwrap_or_default();
    let mut keep: Vec<Value> = vec![];
    for p in ps.drain(..) {
        let i = PERSONALITY.iter().position(|x| Value::from(*x) == p).unwrap_or(0);
        if keep.iter().any(|k| *k == json!(PERSONALITY[i ^ 1])) { continue; }
        keep.push(p);
    }
    keep.truncate(4);
    f.set("personality", Value::Array(keep));
    let gs: Vec<Value> = f.get("genetic").as_array().cloned().unwrap_or_default();
    let mut gk: Vec<Value> = vec![];
    for g in gs {
        let i = GENETIC.iter().position(|x| Value::from(*x) == g).unwrap_or(0);
        if OPPOSED.iter().any(|&(a, b)| (a == i && gk.contains(&json!(GENETIC[b]))) || (b == i && gk.contains(&json!(GENETIC[a])))) { continue; }
        gk.push(g);
    }
    f.set("genetic", Value::Array(gk));
}

pub fn apply_archetype(f: &mut Form, name: &str, d: &mut Dice) {
    let set_stats = |f: &mut Form, v: [f64; 6]| { for (i, a) in ATTRS.iter().enumerate() { f.set(a, json!(v[i])); } };
    for s in SKILLS.iter() { f.set(&format!("skill:{s}"), json!(10.0 + d.below(10) as f64)); }
    let skills = |f: &mut Form, list: &[(&str, f64)]| { for (s, v) in list { f.set(&format!("skill:{s}"), json!(v)); } };
    let traits = |f: &mut Form, t: &[&str]| f.set("personality", Value::Array(t.iter().map(|x| json!(x)).collect()));
    let genes = |f: &mut Form, g: &[&str]| f.set("genetic", Value::Array(g.iter().map(|x| json!(x)).collect()));
    let (cls, lvl) = match name {
        "Warrior" => { set_stats(f, [5.0, 14.0, 5.0, 5.0, 4.0, 6.0]); skills(f, &[("Blade", 60.0), ("Block", 50.0), ("Heavy Armor", 45.0)]); traits(f, &["brave", "wrathful", "stubborn"]); genes(f, &["strong"]); ("Warrior", 12.0) }
        "Knight" => { set_stats(f, [8.0, 13.0, 7.0, 4.0, 6.0, 6.0]); skills(f, &[("Blade", 55.0), ("Heavy Armor", 55.0), ("Block", 50.0)]); traits(f, &["brave", "just", "honest"]); genes(f, &["iron-boned"]); ("Knight", 12.0) }
        "Ranger" => { set_stats(f, [5.0, 10.0, 6.0, 7.0, 7.0, 8.0]); skills(f, &[("Archery", 60.0), ("Sneak", 40.0), ("Light Armor", 40.0), ("Beast Taming", 25.0)]); traits(f, &["patient", "calm", "brave"]); genes(f, &["keen-eyed"]); ("Ranger", 10.0) }
        "Thief" => { set_stats(f, [6.0, 5.0, 6.0, 14.0, 6.0, 6.0]); skills(f, &[("Sneak", 60.0), ("Security", 55.0), ("Pickpocket", 55.0), ("Light Armor", 35.0)]); traits(f, &["deceitful", "greedy", "impatient"]); genes(f, &["quick"]); ("Rogue", 9.0) }
        "Wizard" => { set_stats(f, [6.0, 4.0, 6.0, 7.0, 15.0, 12.0]); skills(f, &[("Destruction", 60.0), ("Alteration", 50.0), ("Conjuration", 35.0)]); traits(f, &["diligent", "arrogant", "patient"]); genes(f, &["spellborn"]); ("Wizard", 12.0) }
        "Priest" => { set_stats(f, [10.0, 4.0, 6.0, 5.0, 10.0, 11.0]); skills(f, &[("Restoration", 60.0), ("Speech", 55.0)]); traits(f, &["zealous", "compassionate", "humble"]); genes(f, &[]); ("Priest", 9.0) }
        "Ruler-in-waiting" => { set_stats(f, [14.0, 9.0, 13.0, 8.0, 9.0, 9.0]); skills(f, &[("Speech", 55.0), ("Mercantile", 40.0), ("Blade", 35.0)]); traits(f, &["ambitious", "just", "gregarious"]); genes(f, &["comely"]); f.set("origin", json!("highborn")); f.set("ambition", json!("power")); ("Knight", 8.0) }
        "Scholar" => { set_stats(f, [6.0, 3.0, 8.0, 5.0, 18.0, 14.0]); skills(f, &[("Restoration", 40.0), ("Alchemy", 50.0), ("Speech", 35.0)]); traits(f, &["diligent", "patient", "humble"]); genes(f, &["genius"]); f.set("ambition", json!("knowledge")); ("Wizard", 6.0) }
        "Artist" => { set_stats(f, [9.0, 3.0, 5.0, 6.0, 14.0, 15.0]); skills(f, &[("Speech", 50.0), ("Illusion", 30.0)]); traits(f, &["gregarious", "diligent", "fickle"]); genes(f, &["artistic"]); f.set("ambition", json!("glory")); ("Rogue", 4.0) }
        "Merchant" => { set_stats(f, [12.0, 4.0, 14.0, 9.0, 8.0, 7.0]); skills(f, &[("Mercantile", 65.0), ("Speech", 55.0)]); traits(f, &["greedy", "gregarious", "ambitious"]); genes(f, &["silver-tongued"]); f.set("origin", json!("trade")); f.set("ambition", json!("wealth")); ("Rogue", 5.0) }
        "Runesmith" => { set_stats(f, [5.0, 9.0, 9.0, 4.0, 11.0, 10.0]); skills(f, &[("Smithing", 70.0), ("Enchanting", 55.0), ("Heavy Armor", 40.0)]); traits(f, &["diligent", "stubborn", "temperate"]); genes(f, &["iron-boned"]); f.set("origin", json!("forge")); ("Runesmith", 12.0) }
        _ => { set_stats(f, [10.0, 16.0, 8.0, 8.0, 10.0, 12.0]); skills(f, &[("Blade", 85.0), ("Block", 70.0), ("Heavy Armor", 70.0), ("Restoration", 40.0)]); traits(f, &["brave", "just", "diligent"]); genes(f, &["strong", "hale"]);
              f.set("bloodlines", json!(["starblood"])); f.set("hero", json!(true)); f.set("ambition", json!("glory")); f.set("origin", json!("heroblood")); ("Knight", 40.0) }
    };
    f.set("cls", json!(cls));
    f.set("level", json!(lvl));
    if name != "Hero of legend" { f.set("hero", json!(matches!(name, "Warrior" | "Knight" | "Ranger" | "Wizard"))); f.set("bloodlines", json!([])); }
}

pub fn randomize(f: &mut Form, d: &mut Dice) {
    let people = d.below(4);
    let female = d.below(2) == 1;
    f.set("people", json!(people));
    f.set("female", json!(female));
    f.set("name", json!(random_name(d, people, female)));
    f.set("age", json!((ADULT[people] as f64 * d.range(1.0, 2.2)).round()));
    f.set("cls", json!(CLASS_WEIGHTS[people][d.below(CLASS_WEIGHTS[people].len())].0).as_u64().map(|i| json!(CLASSES[i as usize].name)).unwrap_or(json!("Warrior")));
    f.set("level", json!(1 + d.below(18)));
    for a in ATTRS.iter() { f.set(a, json!(3 + d.below(12))); }
    for s in SKILLS.iter() { f.set(&format!("skill:{s}"), json!(d.below(45))); }
    let mut ps = vec![];
    while ps.len() < 3 { let i = d.below(PERSONALITY.len()); if !ps.contains(&(i ^ 1)) && !ps.contains(&i) { ps.push(i); } }
    f.set("personality", Value::Array(ps.iter().map(|&i| json!(PERSONALITY[i])).collect()));
    let gs: Vec<Value> = GENETIC_FREQ.iter().filter(|(_, p)| d.range(0.0, 1.0) < p * 2.0).map(|(g, _)| json!(GENETIC[*g])).collect();
    f.set("genetic", Value::Array(gs));
    f.set("bloodlines", json!([]));
    f.set("origin", json!(ORIGINS[d.below(11)]));
    f.set("ambition", Value::Null);
    tidy_traits(f);
}

/// The creator's values as the world's create_character act takes them.
pub fn creator_args(f: &Form) -> serde_json::Map<String, Value> {
    let mut a = serde_json::Map::new();
    for k in ["name", "people", "female", "age", "realm", "origin", "ambition", "origin_story", "cls", "level", "hero", "gold", "prowess", "talent", "personality", "track"] {
        let v = f.get(k);
        if !v.is_null() && !(v.is_string() && v.as_str() == Some("")) { a.insert(k.into(), v); }
    }
    let mut genes: Vec<Value> = f.get("genetic").as_array().cloned().unwrap_or_default();
    genes.extend(f.get("bloodlines").as_array().cloned().unwrap_or_default());
    a.insert("genetic".into(), Value::Array(genes));
    a.insert("latent".into(), f.get("latent"));
    let mut attrs = serde_json::Map::new();
    for at in ATTRS.iter() { attrs.insert(at.to_string(), f.get(at)); }
    a.insert("attrs".into(), Value::Object(attrs));
    let mut sk = serde_json::Map::new();
    for s in SKILLS.iter() { sk.insert(s.to_string(), f.get(&format!("skill:{s}"))); }
    a.insert("skills".into(), Value::Object(sk));
    for k in ["age", "level", "realm", "people"] { if let Some(n) = a.get(k).and_then(|v| v.as_f64()) { a.insert(k.into(), json!(n as i64)); } }
    a
}

// ---------------------------------------------------------------- the character editor
pub const EDITS: [(&str, &str, &str); 14] = [
    ("set_stat", "Set a stat or skill", "Any stat, skill, level, renown, gold, stress, happiness, health or lifespan."),
    ("traits", "Change traits", "Personality and inborn traits, as a checklist of what they have now."),
    ("set_class", "Change class", "Take up a new class, prestige classes included."),
    ("give_item", "Give an item", "A relic of your naming, at the rarity you choose."),
    ("marry", "Marry them to someone", "Wed by divine decree."),
    ("make_love", "Make them fall in love", "With anyone you choose."),
    ("track", "Track or stop tracking", "Their life story in the Tracked tab."),
    ("make_hero", "Make them a hero", "They take up the hero's calling."),
    ("crown_character", "Crown them", "Set them on their realm's throne."),
    ("make_immortal", "Make them immortal", "They will not die of age."),
    ("make_custom", "Set them apart", "A custom character: the only kind who can climb past the seventh tier of magic."),
    ("name_concord", "Name them the Concord", "The one bender of every element, when the world needs them."),
    ("strike_down", "Strike them down", "Death from the sky."),
    ("raise_dead", "Raise them from the dead", "They walk out of the grave."),
];

pub fn edit_form(st: &State, key: &str, c: usize) -> Option<Form> {
    let ch = &st.chars[c];
    let go = Field::action("Do it", "Enter or F10 to do this.");
    let name = crate::sheet::full_name(st, c);
    let fields = match key {
        "set_stat" => {
            let mut opts: Vec<Opt> = ATTRS.iter().enumerate().map(|(i, a)| (format!("{} (now {:.0})", STAT_NAMES[i], ch.attrs[i]), json!(a), String::new())).collect();
            for (k, v) in [("prowess", ch.prowess), ("talent", ch.talent), ("level", ch.level as f64), ("renown", ch.renown), ("gold", ch.gold), ("stress", ch.stress), ("happy", ch.happy), ("health", ch.health), ("life", ch.life)] {
                opts.push((format!("{k} (now {v:.0})"), json!(k), String::new()));
            }
            for (i, s) in SKILLS.iter().enumerate() { opts.push((format!("{s} (now {:.0})", ch.skills[i]), json!(s), String::new())); }
            vec![Field::choice("key", "Stat", opts, "Type to search."), Field::num("value", "New value", 10.0, -100.0, 1e6, 1.0, ""), go]
        }
        "traits" => {
            let have_p: Vec<Value> = ch.personality.iter().map(|&p| json!(PERSONALITY[p])).collect();
            let have_g: Vec<Value> = ch.genetic.iter().map(|&g| json!(GENETIC[g])).collect();
            let mut all_g = gene_opts(false);
            all_g.extend(gene_opts(true));
            vec![Field::multi("personality", "Personality", personality_opts(), have_p, "Tick and untick; opposites cancel."),
                 Field::multi("genetic", "Inborn traits and bloodlines", all_g, have_g, "Tick and untick."), go]
        }
        "set_class" => vec![Field::choice("cls", "Class", class_opts(true), ""), go],
        "give_item" => vec![Field::text("name", "Name it", "a nameless relic", ""), Field::choice("rarity", "Rarity", rarity_opts(), ""), go],
        "marry" | "make_love" => vec![Field::choice("b", "With", char_opts(st, true), "Type a name to search."), go],
        "track" => vec![Field::toggle("on", "Track them", !ch.tracked, ""), go],
        "make_immortal" | "make_custom" => vec![Field::toggle("on", "On", true, ""), go],
        _ => vec![go],
    };
    let label = EDITS.iter().find(|e| e.0 == key).map(|e| e.1).unwrap_or(key);
    let mut f = Form::new(&format!("{label}: {name}"), fields);
    f.note = EDITS.iter().find(|e| e.0 == key).map(|e| e.2.to_string()).unwrap_or_default();
    Some(f)
}

/// The acts an edit comes to (traits may be several).
pub fn edit_acts(st: &State, key: &str, c: usize, f: &Form) -> Vec<(String, serde_json::Map<String, Value>)> {
    let mut base = serde_json::Map::new();
    base.insert("c".into(), json!(c));
    let with = |extra: serde_json::Map<String, Value>| { let mut m = base.clone(); m.extend(extra); m };
    match key {
        "traits" => {
            let ch = &st.chars[c];
            let mut out = vec![];
            let want_p: Vec<String> = f.get("personality").as_array().cloned().unwrap_or_default().iter().filter_map(|v| v.as_str().map(String::from)).collect();
            let want_g: Vec<String> = f.get("genetic").as_array().cloned().unwrap_or_default().iter().filter_map(|v| v.as_str().map(String::from)).collect();
            let have_p: Vec<String> = ch.personality.iter().map(|&p| PERSONALITY[p].to_string()).collect();
            let have_g: Vec<String> = ch.genetic.iter().map(|&g| GENETIC[g].to_string()).collect();
            for (want, have) in [(&want_p, &have_p), (&want_g, &have_g)] {
                for t in have.iter().filter(|t| !want.contains(t)) { out.push(("set_trait".into(), with(serde_json::Map::from_iter([("trait".into(), json!(t)), ("on".into(), json!(false))])))); }
                for t in want.iter().filter(|t| !have.contains(t)) { out.push(("set_trait".into(), with(serde_json::Map::from_iter([("trait".into(), json!(t)), ("on".into(), json!(true))])))); }
            }
            out
        }
        "marry" | "make_love" => { let mut m = serde_json::Map::new(); m.insert("a".into(), json!(c)); m.insert("b".into(), f.get("b")); vec![(key.into(), m)] }
        "make_immortal" | "make_custom" | "track" => vec![(key.into(), with(serde_json::Map::from_iter([("on".into(), json!(yes(&f.get("on"))))])))],
        _ => vec![(key.into(), with(f.args()))],
    }
}
