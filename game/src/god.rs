//! The lists god mode chooses from, drawn from the world as it is; names and presets for making people.

use serde_json::{json, Value};
use world::lives::{origin_words, AMBITIONS, ORIGINS};
use world::people::NONE;
use world::state::State;
use world::tables::*;

pub type Opt = (String, Value, String);

pub fn realm_opts(st: &State) -> Vec<Opt> {
    let mut ks: Vec<usize> = (0..st.realms.len()).filter(|&k| !st.realms[k].extinct).collect();
    let towns = |k: usize| st.towns.iter().filter(|t| t.alive && t.realm == k).count();
    ks.sort_by_key(|&k| std::cmp::Reverse(towns(k)));
    ks.into_iter().map(|k| {
        let r = st.realms[k].ruler;
        let ruler = if r != NONE { crate::sheet::full_name(st, r) } else { "no ruler".into() };
        (st.realms[k].name.chars().take(45).collect(), json!(k), format!("{}, {} towns, ruled by {ruler}", PEOPLES[st.realms[k].people], towns(k)))
    }).collect()
}
pub fn land_opts(st: &State) -> Vec<Opt> {
    let mut ls: Vec<usize> = (0..st.map.lands.len()).filter(|&l| st.map.lands[l].kind != "isle").collect();
    ls.sort_by_key(|&l| std::cmp::Reverse(st.map.lands[l].size));
    ls.into_iter().map(|l| (format!("{} ({})", st.map.lands[l].name, st.map.lands[l].kind), json!(l), String::new())).collect()
}
pub fn people_opts() -> Vec<Opt> {
    (0..4).map(|p| (PEOPLES[p].to_string(), json!(p), format!("come of age at {}, live about {:.0} years, magic tier cap {}", ADULT[p], LIFESPAN[p], TIER_CAP[p]))).collect()
}
pub fn char_opts(st: &State) -> Vec<Opt> {
    let mut cs: Vec<usize> = st.living.iter().copied().filter(|&c| st.chars[c].alive).collect();
    cs.sort_by(|&a, &b| st.chars[b].renown.partial_cmp(&st.chars[a].renown).unwrap());
    cs.into_iter().take(3000).map(|c| (char_label(st, c), json!(c), String::new())).collect()
}
pub fn char_label(st: &State, c: usize) -> String {
    let ch = &st.chars[c];
    let ruler = if ch.ruled.is_some_and(|r| r.2.is_none()) { " ♛" } else { "" };
    let tr = if ch.tracked { " ★" } else { "" };
    let life = if ch.alive { format!("{}", ch.age(st.year)) } else { format!("died {}", ch.died.unwrap_or(0)) };
    format!("{}{ruler}{tr}  ({}, {life}, lv {}, #{c})", crate::sheet::full_name(st, c), PEOPLES[ch.people], ch.level)
}
pub fn class_opts(prestige: bool) -> Vec<Opt> {
    CLASSES.iter().filter(|c| prestige || !c.prestige).map(|c| {
        let b = c.branches.iter().filter(|b| !b.0.is_empty()).map(|b| b.0).collect::<Vec<_>>().join(", ");
        (c.name.to_string(), json!(c.name), if b.is_empty() { "a prestige class".into() } else { format!("paths: {b}") })
    }).collect()
}
const STAT_NAMES: [&str; 6] = ["diplomacy", "martial", "stewardship", "intrigue", "learning", "lore"];
fn fx_words(fx: &[f64], names: &[&str]) -> String {
    let parts: Vec<String> = fx.iter().zip(names).filter(|(v, _)| **v != 0.0).map(|(v, n)| format!("{}{v} {n}", if *v > 0.0 { "+" } else { "" })).collect();
    if parts.is_empty() { "shapes choices and moods".into() } else { parts.join(", ") }
}
pub fn personality_opts() -> Vec<Opt> {
    PERSONALITY.iter().enumerate().map(|(i, p)| (format!("{p} (not {})", PERSONALITY[i ^ 1]), json!(p), fx_words(&PERSONALITY_FX[i], &STAT_NAMES))).collect()
}
const BLOODLINE_WORDS: [(&str, &str); 11] = [
    ("starblood", "lifts their people's tier cap by one, to the seventh at most"), ("dragonblood", "past the hundredth level, to the 130th"),
    ("elderblood", "lives three times as long"), ("titanblood", "breaks the prowess cap, strikes a third harder"),
    ("abyssblood", "breaks the talent cap"), ("wolf-merged", "a wolf in the blood"), ("bear-merged", "a great bear in the blood"),
    ("eagle-merged", "an eagle in the blood"), ("griffin-merged", "a griffin in the blood"), ("wyvern-merged", "a wyvern in the blood"),
    ("dragon-merged", "a dragon in the blood"),
];
const GENE_NAMES: [&str; 9] = ["diplomacy", "martial", "stewardship", "intrigue", "learning", "lore", "prowess", "talent", "life"];
/// Inborn traits and bloodlines; `bloodlines_only` for the hidden (carried) list.
pub fn gene_opts(bloodlines_only: bool) -> Vec<Opt> {
    GENETIC.iter().enumerate().filter(|(i, g)| (!bloodlines_only || BLOODLINES.contains(i)) && !g.starts_with("half-")
            && !matches!(**g, "tidebound" | "stoneborn" | "flameheart" | "skyborn" | "hollow-eye" | "deepwell"))
        .map(|(i, g)| {
            let mut fx = GENETIC_FX[i];
            fx[8] = (fx[8] - 1.0) * 100.0;
            let w = BLOODLINE_WORDS.iter().find(|b| b.0 == *g).map(|b| format!("bloodline: {}", b.1))
                .unwrap_or_else(|| if *g == "artistic" { "only the artistic make art, songs and books".into() } else { fx_words(&fx, &GENE_NAMES).replace(" life", "% life") });
            (g.to_string(), json!(g), w)
        }).collect()
}
pub fn origin_opts() -> Vec<Opt> { ORIGINS.iter().map(|o| (format!("{o}: {}", origin_words(o)), json!(o), String::new())).collect() }
pub fn ambition_opts() -> Vec<Opt> {
    std::iter::once(("let their life decide".to_string(), Value::Null, "chosen at coming of age from their origin and nature".to_string()))
        .chain(AMBITIONS.iter().map(|a| (a.0.to_string(), json!(a.0), a.1.to_string()))).collect()
}

/// A small generator of its own, so designing a person never touches the world's dice.
pub struct Dice(u64);
impl Dice {
    pub fn new() -> Dice { Dice(std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_nanos() as u64).unwrap_or(7) | 1) }
    pub fn next(&mut self) -> u64 { self.0 ^= self.0 << 13; self.0 ^= self.0 >> 7; self.0 ^= self.0 << 17; self.0 }
    pub fn below(&mut self, n: usize) -> usize { (self.next() % n.max(1) as u64) as usize }
}
pub fn random_name(d: &mut Dice, people: usize, female: bool) -> String {
    let [pm, pf, sm, sf] = FIRST[people.min(3)];
    let (p, s) = if female { (pf, sf) } else { (pm, sm) };
    format!("{}{}", p[d.below(p.len())], s[d.below(s.len())])
}

/// Archetypes for making a person: (name, class, level, stats dip..lore, personality, inborn, origin, ambition, hero).
pub const ARCHETYPES: [(&str, &str, u32, [i32; 6], &[&str], &[&str], &str, &str, bool); 12] = [
    ("Warrior", "Warrior", 12, [5, 14, 5, 5, 4, 6], &["brave", "wrathful", "stubborn"], &["strong"], "hearth", "glory", true),
    ("Knight", "Knight", 12, [8, 13, 7, 4, 6, 6], &["brave", "just", "honest"], &["iron-boned"], "hearth", "glory", true),
    ("Ranger", "Ranger", 10, [5, 10, 6, 7, 7, 8], &["patient", "calm", "brave"], &["keen-eyed"], "hearth", "glory", true),
    ("Thief", "Rogue", 9, [6, 5, 6, 14, 6, 6], &["deceitful", "greedy", "impatient"], &["quick"], "poor", "wealth", false),
    ("Wizard", "Wizard", 12, [6, 4, 6, 7, 15, 12], &["diligent", "arrogant", "patient"], &["spellborn"], "hearth", "knowledge", true),
    ("Priest", "Priest", 9, [10, 4, 6, 5, 10, 11], &["zealous", "compassionate", "humble"], &[], "temple", "faith", false),
    ("Ruler-in-waiting", "Knight", 8, [14, 9, 13, 8, 9, 9], &["ambitious", "just", "gregarious"], &["comely"], "highborn", "power", false),
    ("Scholar", "Wizard", 6, [6, 3, 8, 5, 18, 14], &["diligent", "patient", "humble"], &["genius"], "hearth", "knowledge", false),
    ("Artist", "Rogue", 4, [9, 3, 5, 6, 14, 15], &["gregarious", "diligent", "fickle"], &["artistic"], "hearth", "glory", false),
    ("Merchant", "Rogue", 5, [12, 4, 14, 9, 8, 7], &["greedy", "gregarious", "ambitious"], &["silver-tongued"], "trade", "wealth", false),
    ("Runesmith", "Runesmith", 12, [5, 9, 9, 4, 11, 10], &["diligent", "stubborn", "temperate"], &["iron-boned"], "forge", "wealth", false),
    ("Hero of legend", "Knight", 40, [10, 16, 8, 8, 10, 12], &["brave", "just", "diligent"], &["strong", "hale", "starblood"], "heroblood", "glory", true),
];
