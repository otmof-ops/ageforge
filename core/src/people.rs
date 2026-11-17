//! People, houses, towns and realms: the world's records, kept in vectors indexed by id.

use crate::rng::Pcg64;
use crate::tables::*;

pub const NONE: usize = usize::MAX;

#[derive(Clone)]
pub struct Character {
    pub id: usize,
    pub name: String,
    pub people: usize,
    pub realm: usize,
    pub female: bool,
    pub born: i64,
    pub house: usize,
    pub father: usize,
    pub mother: usize,
    pub spouse: usize,
    pub children: Vec<usize>,
    pub genetic: Vec<usize>,     // indexes into GENETIC
    pub latent: Vec<usize>,
    pub personality: Vec<usize>, // indexes into PERSONALITY
    pub attrs: [f64; 6],         // dip mar ste intr lrn lore
    pub prowess: f64,
    pub talent: f64,
    pub life: f64,
    pub classes: Vec<(usize, u32)>, // (index into CLASSES, levels)
    pub level: u32,
    pub xp: f64,
    pub hp: f64,
    pub renown: f64,
    pub branch: Vec<((usize, usize), u8)>,
    pub capstones: Vec<usize>,
    pub attributes: [f64; 8],
    pub skills: Vec<f64>,
    pub legacy: Vec<(usize, f64)>,
    pub legitimacy: Option<f64>,
    pub tyranny: f64,
    pub regnal: Option<String>,
    pub ruled: Option<(usize, i64, Option<i64>)>,
    pub gift: Option<usize>,
    pub religion: usize,
    pub hero: bool,
    pub undead: bool,
    pub built: u32,
    pub disgraced_until: i64,
    pub titles: Vec<String>,
    pub memories: Vec<(usize, &'static str, f64, f64, i64)>, // (about whom, why, value, decay, year), for opinion
    pub deeds: Vec<(&'static str, f64)>,
    pub perks: Vec<(usize, usize)>,        // (skill, perk index)
    pub skill_progress: [f64; 8],
    pub battle_arts: u32,
    pub lost_art: bool,
    pub custom: bool,
    pub concord: bool,
    pub health: f64,
    pub epithet: Option<String>,
    pub duchies: Vec<DKey>,
    pub op_bias: Option<f64>,
    pub view_noise: Option<[f64; 5]>,
    pub profession: Option<&'static str>,
    pub lordships: Vec<usize>,
    pub happy: f64,
    pub relations: Vec<(usize, bool)>,     // (whom, friend: true / rival: false)
    pub rival_ruler: usize,
    pub hooks: Vec<(usize, &'static str)>,
    pub rival: usize,
    // the heart
    pub stress: f64,
    pub loves: Vec<(usize, &'static str, f64)>,     // (whom, kind, strength)
    pub loved_by: Vec<usize>,
    pub vendettas: Vec<(usize, i64, String)>,       // (whom, year, whom they lost)
    pub grief: f64,
    pub coping: Vec<&'static str>,
    pub pet: Option<(&'static str, &'static str, i64, i64)>,  // kind, name, since, life
    pub orientation: u8,                            // 0 straight, 1 gay, 2 bi
    pub affair: usize,
    // crafts, the court and youth
    pub items: Vec<usize>,
    pub fame: Vec<(&'static str, f64)>,
    pub bounty: f64,
    pub jailed_until: i64,
    pub education: Option<(usize, usize)>,  // (focus: dip mar ste intr lrn, tier)
    pub ls_xp: f64,
    pub perk_list: Vec<(&'static str, &'static str, &'static str)>,
    pub practice: u32,
    pub tutored: f64,
    pub trained_in: Option<usize>,
    pub marks: Vec<&'static str>,
    pub clan: usize,
    pub element: Option<&'static str>,
    pub last_deed: Option<&'static str>,
    pub rite_ready: i64,
    pub peak_cr: f64,
    pub seal: Option<usize>,
    pub merit: f64,
    pub party: usize,
    pub pending: Option<(usize, usize, f64)>,   // (state, action, renown before)
    pub kills: u32,
    pub beyond_ready: i64,
    pub posted: f64,
    pub beast: Option<(&'static str, String, f64, i64, f64)>,  // kind, name, power, since, life
    pub merged: Option<&'static str>,
    pub bend: Vec<(usize, f64)>,                               // (element, level)
    pub high_art: Option<&'static str>,
    pub tribe: usize,
    pub last_peace: i64,
    pub ki: f64,
    pub natures: Vec<(usize, u8)>,                             // (nature, level)
    pub sarts: Vec<(usize, u8)>,                               // (art, level)
    pub srank: usize,
    pub missions: u32,
    pub pirate: bool,
    pub ship: (String, usize, i64),                            // name, tier, crew
    pub haven: usize,
    pub infamy: f64,
    pub marque: usize,
    pub resume: Vec<String>,
    pub guilds: Vec<usize>,
    pub guild_rank: Option<&'static str>,
    pub extra: [f64; 6],                  // legacy, class effects and perks on the six stats, refreshed yearly
    pub alive: bool,
    pub died: Option<i64>,
    pub cause: Option<String>,
    pub home: usize,
    pub gold: f64,
}

impl Character {
    pub fn deed(&self, kind: &str) -> f64 {
        self.deeds.iter().find(|d| d.0 == kind).map(|d| d.1).unwrap_or(0.0)
    }
    pub fn has_trait(&self, name: &str) -> bool {
        self.personality.iter().any(|&p| PERSONALITY[p] == name)
    }

    pub fn age(&self, year: i64) -> i64 { year - self.born }
    pub fn adult(&self, year: i64) -> bool { self.age(year) >= ADULT[self.people] as i64 }

    /// A base stat as used (Character.attr): trained value, a point per fifteen levels, less the toll of age.
    /// Legacy, education, perks and class effects join here with slices 3 and 4.
    pub fn attr(&self, a: usize, year: i64) -> f64 {
        let mut v = self.attrs[a] + (self.level / 15) as f64 + self.extra[a];
        if a != 4 && a != 5 && !self.undead {
            let frac = self.age(year) as f64 / self.life.max(1.0);
            v -= ((frac - 0.66) * 25.0).max(0.0).floor();
        }
        v
    }

    pub fn has(&self, g: usize) -> bool {
        self.genetic.contains(&g)
    }
    pub fn level_cap(&self) -> u32 {
        if self.has(gene("dragonblood")) { LEVEL_CAP_T[1] as u32 } else { LEVEL_CAP_T[0] as u32 }
    }
}

pub struct House {
    pub id: usize,
    pub name: String,
    pub people: usize,
    pub founder: usize,
    pub founded: i64,
    pub prestige: f64,
    pub rulers: u32,
    pub track: String,
    pub legacy: f64,
    pub origin: String,
    pub cursed: bool,
}

pub struct Town {
    pub id: usize,
    pub name: String,
    pub realm: usize,
    pub culture: usize,
    pub pos: (usize, usize),
    pub pop: f64,
    pub wealth: f64,
    pub walls: u32,
    pub alive: bool,
    pub founded: i64,
    pub capital: bool,
    pub landmass: i32,
    pub port: bool,
    // the land's year (economy and markets)
    pub cells: usize,
    pub magic: f64,
    pub soil: f64,
    pub granary: Option<f64>,
    pub food: f64,
    pub food_prod: f64,
    pub famine: f64,
    pub hunger_dead: f64,
    pub lord_mod: f64,
    pub res_food: f64,
    pub res_wealth: f64,
    pub jobs: [f64; 7],
    pub jobless: f64,
    pub farm_fill: f64,
    pub labour_short: f64,
    pub skill: Option<[f64; 7]>,
    pub arcane: f64,
    pub labour_value: f64,
    pub income: f64,
    pub housing: Option<f64>,
    pub crowded: f64,
    pub stock: [f64; 3],
    pub cls: [(f64, f64, f64); 3],     // (people, income each, poor share) for lower, middle, upper
    pub poverty: f64,
    pub econ_mood: f64,
    pub bld: [u8; 8],                 // levels of BUILDINGS (stewardship)
    pub relief: i64,
    pub coin: Option<f64>,
    pub garrison: f64,
    pub trade_gain: f64,
    pub real_income: f64,
    pub bullion: f64,
    // the common people (populace)
    pub burghers: f64,
    pub mood_p: Option<f64>,
    pub mood_b: f64,
    pub rising: Option<i64>,
    pub rallied: f64,
    pub immune: f64,
    pub sick_now: f64,
    pub crime: f64,
    pub duchy: Option<DKey>,
    pub faith: Vec<(usize, f64)>,
    pub lord: usize,
    pub occupied_by: usize,
    pub trade: f64,
    pub guard: f64,
    pub threat: f64,
    pub raided: Option<&'static str>,
    pub raided_by: usize,
    pub raided_year: i64,
    pub game: f64,
    pub curse_mult: f64,
    pub colony: bool,
    pub metropole: usize,
}

pub struct Epidemic {
    pub name: String,
    pub start: i64,
    pub fatality: f64,
    pub spread: f64,
    pub great: bool,
    pub deaths: f64,
    pub towns: std::collections::HashSet<usize>,
    pub front: Vec<(usize, f64)>,
    pub end: Option<i64>,
}

pub struct Realm {
    pub id: usize,
    pub people: usize,
    pub name: String,
    pub short: String,
    pub rank: &'static str,
    pub gov: &'static str,
    pub law: &'static str,
    pub color: [u8; 3],
    pub ruler: usize,
    pub house: usize,
    pub regnal: Vec<(String, u32)>,
    pub reigns: Vec<Reign>,
    pub place: String,
    pub extinct: bool,
    pub growth_mult: f64,
    pub tax_now: f64,
    pub gold: f64,
    pub crown_law: u32,
    pub tolerant: bool,
    pub expansionist: bool,
    pub agenda: &'static str,
    pub agenda_for: usize,
    pub maa: Vec<(usize, u32)>,             // (index into MAA, regiments)
    pub army_power: f64,
    pub garrison_bonus: f64,
    pub aggr: f64,
    pub reform: f64,
    pub agreement: f64,
    pub great_works: Vec<String>,
    pub held: Vec<usize>,                   // every resource held
    pub home_land: i32,
    pub known: Vec<i32>,
    pub overlord: usize,
    pub vassal_limit: f64,
    pub domain_limit: f64,
    pub over_vassals: f64,
    pub domain_penalty: f64,
    pub last_stand: bool,
    pub security: f64,
    pub realm_tier: i64,
    pub fear_until: i64,
    pub corrupt_seats: Vec<usize>,
    pub done: Vec<(&'static str, i32)>,
    pub great: Vec<String>,
    pub tax_relief: i64,
    pub regent: usize,
    pub taken: u32,
    pub lost: u32,
    pub civil_wars: u32,
    pub physician: bool,
    pub parent: usize,
    pub founded_year: i64,
    pub ended: Option<i64>,
    pub battle_mult: f64,
    pub warships: f64,
    pub ship_tier: u32,
    pub allies: Vec<usize>,
    pub factions: Vec<(&'static str, i32, Vec<usize>, usize, f64)>,   // (kind, land, members, leader, power)
    pub faction_since: Vec<((&'static str, i32), i64)>,
    pub ultimatum_cooldown: Vec<(&'static str, i64)>,
    pub council: [usize; 6],          // marshal steward spymaster mage loremaster admiral
    pub council_for: usize,
    pub econ_distress: f64,
    pub corruption: f64,
    pub collect: f64,
    pub tax_mult: f64,
    pub levy: f64,
    pub income: f64,
    pub real_gdp: f64,
    pub credit: Option<f64>,
    pub defaults: u32,
    pub rate: f64,
    pub money: f64,
    pub trade_val: f64,
    pub trade_net: f64,
    pub partners: u32,
    pub trade_protectionist: bool,
    pub last_forced: i64,
    pub last_default: i64,
    pub mint_skill: f64,
    pub sector_boost: [f64; 7],
    pub econ_prev_gdp: Option<f64>,
    pub goods: Vec<usize>,            // luxury and strategic resources held
    pub market: Vec<usize>,
    pub treasury: Vec<usize>,
    pub soulstones: [u32; 5],
    pub dragonbone: u32,
    pub potions: f64,
    pub arms: f64,
    pub made: Vec<usize>,
    pub court: Option<Court>,
    pub court_mod: f64,
    pub bodyguard: bool,
    pub faith: usize,
    pub persecuting: bool,
    pub mods: Vec<(&'static str, f64, i64)>,      // (key, value, until year)
    pub parts: [f64; 4],                          // hide ash horn scale
    pub scrolls: [f64; 11],
    pub scroll_power: f64,
    pub top_scroll: usize,
    pub bound_share: f64,
    pub trade_shock: i64,
    pub dragons: u32,
    pub merchants: f64,
    pub blockaded: i64,
    pub colonies: u32,
    pub discoveries: Vec<(i64, i32, usize)>,
}

impl Realm {
    pub fn modv(&self, key: &str, year: i64) -> f64 {
        self.mods.iter().find(|m| m.0 == key && m.2 >= year).map(|m| m.1).unwrap_or(1.0)
    }
}

pub struct Court {
    pub since: i64,
    pub grandeur: f64,
    pub amen: [u32; 4],
    pub pos: Vec<(&'static str, usize)>,
    pub kind: &'static str,
    pub events: Vec<(i64, String)>,
}

pub struct Culture {
    pub people: usize,
    pub era: usize,
    pub innovations: std::collections::HashSet<&'static str>,
    pub learned: Vec<(&'static str, i64)>,
    pub progress: f64,
    pub focus: Option<usize>,              // index into INNOVATIONS
    pub traditions: Vec<usize>,            // indexes into TRADITIONS
    pub culture_points: f64,
    pub trend: [f64; 5],                   // auth succ faith econ trad
    pub golden: i64,
    pub dark: i64,
    pub history: Vec<(i64, String)>,
}

impl Culture {
    pub fn new(people: usize) -> Self {
        Culture { people, era: 0, innovations: Default::default(), learned: vec![], progress: 0.0, focus: None,
                  traditions: vec![], culture_points: 0.0, trend: CULTURE_VIEWS[people], golden: 0, dark: 0, history: vec![] }
    }
    fn innov(&self) -> impl Iterator<Item = &'static crate::tables::Innov> + '_ {
        INNOVATIONS.iter().filter(|i| self.innovations.contains(i.name))
    }
    pub fn has(&self, flag: &str) -> bool { self.innov().any(|i| i.flags.contains(&flag)) }
    /// A culture's summed bonus for one effect, innovations and traditions together.
    pub fn total(&self, key: &str) -> f64 {
        let j = INNOV_NUM.iter().position(|k| *k == key);
        let a: f64 = j.map(|j| self.innov().map(|i| i.num[j]).sum()).unwrap_or(0.0);
        a + self.traditions.iter().map(|&t| TRADITIONS[t].1.iter().filter(|x| x.0 == key).map(|x| x.1).sum::<f64>()).sum::<f64>()
    }
    pub fn best(&self, key: &str, default: f64) -> f64 {
        let j = INNOV_NUM.iter().position(|k| *k == key).unwrap();
        self.innov().map(|i| i.num[j]).filter(|&v| v != 0.0).fold(None, |m: Option<f64>, v| Some(m.map_or(v, |m| m.max(v)))).unwrap_or(default)
    }
}

/// A duchy's key: (landmass, row / 24, column / 24); a kingdom title's: the same at 72.
pub type DKey = (i32, i32, i32);

pub struct Duchy {
    pub name: String,
    pub holder: usize,
    pub kt: DKey,
    pub since: i64,
    pub land: i32,
    pub ctrl: usize,
}

pub struct KTitle {
    pub name: String,
    pub ctrl: usize,
}

pub struct War {
    pub attacker: usize,
    pub defender: usize,
    pub cb: String,
    pub goal_duchy: Option<DKey>,
    pub subjugate: bool,
    pub freedom: bool,
    pub claimant: usize,
    pub start: i64,
    pub score: f64,
    pub avenge: usize,
}

pub struct Army {
    pub id: usize,
    pub realm: usize,            // NONE for a rising
    pub pos: (usize, usize),
    pub men: f64,
    pub quality: f64,
    pub commander: usize,
    pub morale: f64,
    pub siege: f64,
    pub rebel: bool,
    pub rebel_of: usize,
    pub home: usize,
    pub name: String,
    pub why: &'static str,
    pub done: bool,
}

pub struct Item {
    pub name: String,
    pub rarity: usize,
    pub year: i64,
    pub origin: String,
    pub special: Option<&'static str>,
    pub note: String,
    pub bonus: f64,
    pub desc: Option<String>,
    pub enchants: Vec<(&'static str, &'static str)>,
    pub kind: &'static str,          // weapon, armor, relic
    pub maker: usize,
    pub enchanter: usize,
    pub value: f64,
    pub mat_index: usize,
    pub bearers: Vec<usize>,
}

impl Item {
    pub fn new(name: String, rarity: usize, year: i64, origin: String) -> Self {
        Item { name, rarity, year, origin, special: None, note: String::new(), bonus: RARITY_BONUS[rarity.min(6)], desc: None,
               enchants: vec![], kind: "relic", maker: usize::MAX, enchanter: usize::MAX, value: 0.0, mat_index: 0, bearers: vec![] }
    }
    pub fn label(&self) -> String {
        let mut extra = match &self.desc { Some(d) => format!(", {d}"), None => String::new() };
        if !self.enchants.is_empty() {
            extra += ", ";
            extra += &self.enchants.iter().map(|(e, t)| format!("{t} {e}")).collect::<Vec<_>>().join(" and ");
        }
        format!("{} ({}{extra})", self.name, RARITY[self.rarity.min(6)])
    }
}

pub struct Monster {
    pub kind: &'static str,
    pub name: String,
    pub pos: (usize, usize),
    pub alive: bool,
    pub base: f64,
    pub power: f64,
    pub hoard: f64,
    pub born: i64,
    pub sleep_until: i64,
    pub last_said: i64,
    pub items: Vec<usize>,
    pub landmass: i32,
    pub world: bool,
    pub slain_by: usize,
    // lineage (monsters.new_lineage_fields)
    pub lifespan: f64,
    pub adult_base: f64,
    pub parents: Vec<usize>,
    pub generation: u32,
    pub children: Vec<usize>,
    pub title: Option<&'static str>,
    pub kills: u32,
    pub traits: Vec<&'static str>,
    // dragons
    pub size: Option<f64>,
    pub temper: &'static str,
    pub settled: i64,
    pub game0: Option<f64>,
    pub hungry: u32,
    pub tributaries: Vec<(usize, i64, usize)>,   // (realm, since, payer)
    pub wrath: Vec<(usize, i64)>,
    pub fled: Option<i64>,
    pub fled_told: Option<i64>,
    pub wild_ready: i64,
    // the rest of the world's dealings with it
    pub gate: bool,
    pub crashed: bool,
    pub sea: bool,
    pub spilled: bool,
    pub bounties: Vec<(usize, f64)>,
    pub species: Option<String>,
    pub eco_cell: usize,
    pub cycle: usize,
    pub sealed: bool,
}

pub struct SocGuild {
    pub id: usize,
    pub name: String,
    pub kind: &'static str,
    pub people: usize,            // NONE for the Quiet Hand, which crosses every border
    pub members: Vec<usize>,
    pub master: usize,
    pub gold: f64,
    pub masters: Vec<(i64, usize)>,
}

pub struct Religion {
    pub id: usize,
    pub name: String,
    pub deity: String,
    pub tenets: Vec<&'static str>,
    pub founder: usize,
    pub founded: i64,
    pub people: usize,
    pub parent: usize,
    pub holy_site: usize,
    pub peak: f64,
    pub followers: f64,
    pub dead: bool,
}

impl Religion {
    pub fn has(&self, t: &str) -> bool { self.tenets.contains(&t) }
}

pub struct Reign {
    pub id: usize,
    pub start: i64,
    pub end: Option<i64>,
    pub how: Option<String>,
    pub epithet: Option<String>,
    pub legit: Option<f64>,
    pub tyranny: f64,
}

pub struct Ruin {
    pub name: String,
    pub pos: (usize, usize),
    pub riches: i64,
    pub depth: f64,
    pub loot: Vec<usize>,
    // the dungeon (dungeons.ensure)
    pub kind: &'static str,
    pub floors: usize,
    pub grand: bool,
    pub floor: usize,
    pub conquered: Option<i64>,
    pub conquests: Vec<(i64, usize, String)>,
    pub minis: Vec<(usize, String, f64)>,
    pub boss: (String, f64, bool),
    pub reinfest: i64,
    pub stocked: bool,
}

impl Ruin {
    pub fn new(name: String, pos: (usize, usize), riches: i64, depth: f64) -> Self {
        Ruin { name, pos, riches, depth, loot: vec![], kind: "", floors: 0, grand: false, floor: 0, conquered: None, conquests: vec![],
               minis: vec![], boss: (String::new(), 0.0, true), reinfest: i64::MAX, stocked: false }
    }
}

pub fn gene(name: &str) -> usize {
    GENETIC.iter().position(|g| *g == name).expect("unknown gene")
}


pub fn roman(mut n: u32) -> String {
    let vals = [(1000, "M"), (900, "CM"), (500, "D"), (400, "CD"), (100, "C"), (90, "XC"), (50, "L"), (40, "XL"),
                (10, "X"), (9, "IX"), (5, "V"), (4, "IV"), (1, "I")];
    let mut out = String::new();
    for (v, s) in vals {
        while n >= v {
            out.push_str(s);
            n -= v;
        }
    }
    out
}

pub fn first_name(rng: &mut Pcg64, people: usize, female: bool) -> String {
    let [pre_m, pre_f, suf_m, suf_f] = FIRST[people];
    let mut name = String::new();
    for _ in 0..20 {
        name = format!("{}{}", rng.choose(if female { pre_f } else { pre_m }), rng.choose(if female { suf_f } else { suf_m }));
        if !BORROWED.contains(&name.to_lowercase().as_str()) {
            return name;
        }
    }
    name + "a"
}

pub fn pick_class(rng: &mut Pcg64, people: usize, talent: f64, exclude: &[usize]) -> usize {
    let opts: Vec<(usize, f64)> = CLASS_WEIGHTS[people].iter()
        .filter(|(c, _)| !exclude.contains(c))
        .map(|&(c, v)| (c, v * if CLASSES[c].magic >= 1.0 { (talent / 3.0).max(0.3) } else { 1.0 }))
        .collect();
    let w: Vec<f64> = opts.iter().map(|x| x.1).collect();
    opts[rng.weighted(&w)].0
}

pub fn xp_needed(level: u32) -> f64 {
    let l = level as f64;
    10.0 * l.powf(1.2) * (1.0 + (l - 25.0).max(0.0) / 5.0).powi(2)
}
