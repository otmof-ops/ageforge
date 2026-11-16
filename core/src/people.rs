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
    pub alive: bool,
    pub died: Option<i64>,
    pub gold: f64,
}

impl Character {
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
    pub reigns: Vec<(usize, i64, Option<i64>)>,
    pub place: String,
    pub extinct: bool,
}

pub struct Ruin {
    pub name: String,
    pub pos: (usize, usize),
    pub riches: i64,
    pub depth: f64,
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
