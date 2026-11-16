//! Climate and weather, and what they do to the harvest (climate.py): the normal warmth and rain of every region,
//! then each year an age of the world, a few-year cycle, eruptions and the year's own regional weather, and the
//! harvest that answers to them, with locusts and blight after runs of bad years.

use crate::grid::Grid;
use crate::rng::Pcg64;
use crate::tables::*;

pub const B: usize = 6;
const ERUPTION_A: [&str; 7] = ["Ashen", "Grey", "Dark", "Cold", "Dim", "Silent", "Red-Sun"];
const ERUPTION_B: [&str; 5] = ["Summer", "Year", "Sky", "Winter", "Season"];

/// SciPy's gaussian_filter (truncate 4, mode 'reflect') on an eh x ew grid.
pub fn gaussian(a: &Grid<f64>, sigma: f64) -> Grid<f64> {
    let r = (4.0 * sigma + 0.5) as i64;
    let k: Vec<f64> = (-r..=r).map(|x| (-0.5 * (x as f64 / sigma).powi(2)).exp()).collect();
    let s: f64 = k.iter().sum();
    let k: Vec<f64> = k.iter().map(|x| x / s).collect();
    let refl = |i: i64, n: i64| -> usize {
        let mut i = i;
        loop {
            if i < 0 { i = -i - 1; } else if i >= n { i = 2 * n - i - 1; } else { return i as usize; }
        }
    };
    let (h, w) = (a.h as i64, a.w as i64);
    let mut tmp = Grid::new(a.h, a.w, 0.0);
    for y in 0..h {
        for x in 0..w {
            let mut acc = 0.0;
            for (j, kv) in k.iter().enumerate() {
                acc += kv * a.at(refl(y + j as i64 - r, h), x as usize);
            }
            tmp.set(y as usize, x as usize, acc);
        }
    }
    let mut out = Grid::new(a.h, a.w, 0.0);
    for y in 0..h {
        for x in 0..w {
            let mut acc = 0.0;
            for (j, kv) in k.iter().enumerate() {
                acc += kv * tmp.at(y as usize, refl(x + j as i64 - r, w));
            }
            out.set(y as usize, x as usize, acc);
        }
    }
    out
}

pub struct Climate {
    pub eh: usize,
    pub ew: usize,
    pub lat: Vec<f64>,
    pub temp: Grid<f64>,
    pub rain: Grid<f64>,
    pub land: Grid<bool>,
    pub cold_sens: Grid<f64>,
    pub dry_sens: Grid<f64>,
    pub wet_sens: Grid<f64>,
    pub flood_sens: Grid<f64>,
    pub plains: Grid<f64>,
    pub desert: Grid<f64>,
    pub trend: f64,
    pub osc: f64,
    pub volc: f64,
    pub volc_hemi: i64,
    pub tn: Grid<f64>,
    pub pn: Grid<f64>,
    pub dry_run: Grid<f64>,
    pub wet_run: Grid<f64>,
    pub age: i64,
    pub pattern: Grid<f64>,
    pub t: Grid<f64>,
    pub p: Grid<f64>,
    pub harvest: Grid<f64>,
    pub eruptions: Vec<(i64, String, f64, &'static str)>,
    pub history: Vec<(i64, f64, f64, f64)>,
}

/// What the year's weather did, for the chronicle: (text, minor).
pub type Said = Vec<(String, bool)>;

impl Climate {
    pub fn setup(terrain: &Grid<i8>, rng: &mut Pcg64) -> Climate {
        let (eh, ew) = (terrain.h / B, terrain.w / B);
        let mut frac = vec![Grid::new(eh, ew, 0.0f64); 7];
        for by in 0..eh {
            for bx in 0..ew {
                for r in by * B..(by + 1) * B {
                    for c in bx * B..(bx + 1) * B {
                        let t = terrain.at(r, c) as usize;
                        frac[t].v[by * ew + bx] += 1.0 / (B * B) as f64;
                    }
                }
            }
        }
        let f = |k: i8, i: usize| frac[k as usize].v[i];
        let lat: Vec<f64> = (0..eh).map(|r| (((r as f64 + 0.5) / eh as f64) - 0.5).abs() * 2.0).collect();
        let mut temp = Grid::new(eh, ew, 0.0);
        let mut rain = Grid::new(eh, ew, 0.0);
        for row in 0..eh {
            let west = lat[row] > 0.33;
            let cols: Vec<usize> = if west { (0..ew).collect() } else { (0..ew).rev().collect() };
            let mut m = 1.0f64;
            for c in cols {
                let i = row * ew + c;
                temp.v[i] = 22.0 - 16.0 * lat[row] - 7.0 * f(MOUNTAIN, i) - 2.5 * f(HILLS, i);
                let sea = f(OCEAN, i);
                m = (m + 0.35 * sea).min(1.0);
                let fall = m * (0.25 + 0.6 * f(MOUNTAIN, i) + 0.15 * f(HILLS, i));
                rain.v[i] = fall;
                m = (m - fall * 0.5 * (1.0 - sea)).max(0.05);
            }
        }
        let mut rain = gaussian(&rain, 1.5);
        let land = Grid { h: eh, w: ew, v: (0..eh * ew).map(|i| f(OCEAN, i) < 0.6).collect() };
        let (mut s, mut n) = (0.0f64, 0.0f64);
        for i in 0..eh * ew { if land.v[i] { s += rain.v[i]; n += 1.0; } }
        let mean = (s / n.max(1.0)).max(1e-6);
        for i in 0..eh * ew { rain.v[i] = (rain.v[i] / mean * (1.0 - 0.5 * f(DESERT, i))).clamp(0.15, 3.0); }
        let mk = |g: &dyn Fn(usize) -> f64| Grid { h: eh, w: ew, v: (0..eh * ew).map(g).collect() };
        let cold_sens = mk(&|i| ((lat[i / ew] - 0.35) * 2.0).clamp(0.0, 1.0) + f(MOUNTAIN, i) + 0.5 * f(HILLS, i)
                                 + if temp.v[i] < 8.0 { 0.5 } else { 0.0 });
        let dry_sens = mk(&|i| f(DESERT, i) + if rain.v[i] < 0.6 { 0.8 } else { 0.0 } + 0.3 * f(PLAINS, i));
        let wet_sens = mk(&|i| f(PLAINS, i) * 0.6 + f(SWAMP, i) * 1.2 + if rain.v[i] > 1.4 { 0.3 } else { 0.0 });
        let flood_sens = mk(&|i| f(SWAMP, i) + if f(OCEAN, i) > 0.2 && f(HILLS, i) + f(MOUNTAIN, i) < 0.2 { 0.5 } else { 0.0 });
        let plains = mk(&|i| f(PLAINS, i));
        let desert = mk(&|i| f(DESERT, i));
        let phase = rng.uniform(0.0, 6.3);
        let pattern = mk(&|i| (2.0 * std::f64::consts::PI * ((i % ew) as f64 / ew as f64) + phase).cos());
        let z = Grid::new(eh, ew, 0.0);
        Climate { eh, ew, lat, temp, rain, land, cold_sens, dry_sens, wet_sens, flood_sens, plains, desert,
                  trend: 0.0, osc: 0.0, volc: 0.0, volc_hemi: 0, tn: z.clone(), pn: z.clone(), dry_run: z.clone(),
                  wet_run: z.clone(), age: 0, pattern, t: z.clone(), p: z.clone(), harvest: Grid::new(eh, ew, 1.0),
                  eruptions: vec![], history: vec![] }
    }

    fn noise(&self, rng: &mut Pcg64) -> Grid<f64> {
        let g = Grid { h: self.eh, w: self.ew, v: (0..self.eh * self.ew).map(|_| rng.normal(0.0, 1.0)).collect() };
        gaussian(&g, 3.0)
    }

    /// One year's weather and harvest. `towns` is (pos, pop, name) of the living towns, for the news of pests;
    /// `blight` is the world's lingering blight. Returns what the chronicle should say.
    pub fn year(&mut self, year: i64, rng: &mut Pcg64, towns: &[((usize, usize), f64, String)], blight: bool) -> Said {
        let mut said = Said::new();
        let (eh, ew) = (self.eh, self.ew);
        self.trend = 0.995 * self.trend + rng.normal(0.0, 0.08);
        let mut age = self.age;
        if self.trend > 0.7 { age = 1 } else if self.trend < -0.7 { age = -1 } else if self.trend.abs() < 0.2 { age = 0 }
        if age != self.age {
            if age != 0 {
                said.push((if age > 0 {
                    "**A warm age begins.** The summers lengthen; vines grow where none grew, and the high fields bear.".into()
                } else {
                    "**A cold age begins.** The winters lengthen and the high fields are given up; the glaciers come down the valleys.".into()
                }, false));
            }
            self.age = age;
        }
        self.osc = 0.6 * self.osc + rng.normal(0.0, 0.8);
        self.volc *= 0.5;
        if rng.random() < 0.02 {
            let strength = rng.uniform(0.8, 2.5);
            self.volc += strength;
            self.volc_hemi = if strength < 2.0 { rng.integers(-1, 2) } else { 0 };
            let name = format!("the {} {}", ERUPTION_A[rng.integers(0, 7) as usize], ERUPTION_B[rng.integers(0, 5) as usize]);
            let r#where = match self.volc_hemi { -1 => "the north", 1 => "the south", _ => "the whole world" };
            let mut cap = name.clone();
            cap.replace_range(0..1, "T");
            said.push((format!("**{cap}.** Far away a mountain tears itself open; ash veils the sun over {where}, and summer does not come."), false));
            self.eruptions.push((year, name, (strength * 10.0).round() / 10.0, r#where));
        }
        let tn = self.noise(rng);
        let pn = self.noise(rng);
        let mut y = Grid::new(eh, ew, 1.0f64);
        for i in 0..eh * ew {
            let row = i / ew;
            let row_frac = (row as f64 + 0.5) / eh as f64;
            let hemi = match self.volc_hemi { -1 => row_frac < 0.5, 1 => row_frac >= 0.5, _ => true };
            let volc = if hemi { self.volc } else { 0.0 };
            self.tn.v[i] = 0.3 * self.tn.v[i] + tn.v[i] * 9.0 * 0.9;
            self.pn.v[i] = 0.35 * self.pn.v[i] + pn.v[i] * 9.0 * 0.22;
            let pat = self.pattern.v[i];
            let t = self.trend + 0.3 * self.osc * pat - volc + self.tn.v[i];
            let p = (self.pn.v[i] + 0.15 * self.osc * pat + 0.06 * volc).exp();
            self.t.v[i] = t;
            self.p.v[i] = p;
            let mut yy = 1.0;
            let cs = self.cold_sens.v[i];
            yy *= (1.0 + 0.10 * t.min(0.0) * (1.0 + 2.0 * cs)).clamp(0.25, 1.0);
            yy *= 1.0 + (0.03 * t.max(0.0) * cs).clamp(0.0, 0.1);
            let dry_at = if t > 0.8 { 0.9 } else { 0.8 };
            if p < dry_at { yy *= (p / dry_at).powf(1.0 + self.dry_sens.v[i]).clamp(0.2, 1.0); }
            let wet = (p - 1.25).max(0.0) * (1.2 + self.wet_sens.v[i]) * if t < 0.0 { 1.3 } else { 1.0 };
            yy *= (1.0 - wet).clamp(0.3, 1.0);
            yy *= (1.0 - 0.6 * self.flood_sens.v[i] * (p - 1.5).max(0.0)).clamp(0.4, 1.0);
            yy *= 1.0 + 0.18 * (-((p - 1.0) / 0.18).powi(2)).exp() * (1.0 + 0.5 * t).clamp(0.0, 1.3);
            self.dry_run.v[i] = if p < 0.85 && t > 0.3 { self.dry_run.v[i] + 1.0 } else { 0.0 };
            self.wet_run.v[i] = if p > 1.2 && t > -0.3 { self.wet_run.v[i] + 1.0 } else { 0.0 };
            y.v[i] = yy;
        }
        let dry: Vec<usize> = (0..eh * ew).filter(|&i| self.dry_run.v[i] >= 2.0 && self.plains.v[i] + self.desert.v[i] > 0.4 && self.land.v[i]).collect();
        let wet: Vec<usize> = (0..eh * ew).filter(|&i| self.wet_run.v[i] >= 2.0 && self.plains.v[i] > 0.3 && self.land.v[i]).collect();
        self.pest(&mut y, &dry, rng, 0.12, 6, 0.45, true, towns, &mut said);
        self.pest(&mut y, &wet, rng, 0.08, 5, 0.6, false, towns, &mut said);
        let (mut hs, mut ts, mut ps, mut n) = (0.0f64, 0.0f64, 0.0f64, 0.0f64);
        for i in 0..eh * ew {
            if blight { y.v[i] *= 0.7; }
            y.v[i] = y.v[i].clamp(0.15, 1.35);
            if self.land.v[i] { hs += y.v[i]; ts += self.t.v[i]; ps += self.p.v[i]; n += 1.0; }
        }
        self.harvest = y;
        let n = n.max(1.0);
        self.history.push((year, hs / n, ts / n, ps / n));
        said
    }

    #[allow(clippy::too_many_arguments)]
    fn pest(&self, y: &mut Grid<f64>, cand: &[usize], rng: &mut Pcg64, chance: f64, radius: i64, mult: f64,
            locusts: bool, towns: &[((usize, usize), f64, String)], said: &mut Said) {
        if cand.is_empty() || rng.random() > chance { return; }
        let i = cand[rng.integers(0, cand.len() as i64) as usize];
        let (cy, cx) = ((i / self.ew) as i64, (i % self.ew) as i64);
        for j in 0..self.eh * self.ew {
            let (yy, xx) = ((j / self.ew) as i64, (j % self.ew) as i64);
            if (yy - cy).pow(2) + (xx - cx).pow(2) <= radius * radius { y.v[j] *= mult; }
        }
        let pos = (cy * B as i64 + B as i64 / 2, cx * B as i64 + B as i64 / 2);
        let r2 = (radius * B as i64).pow(2);
        let hit: Vec<&((usize, usize), f64, String)> = towns.iter()
            .filter(|t| (t.0 .0 as i64 - pos.0).pow(2) + (t.0 .1 as i64 - pos.1).pow(2) <= r2).collect();
        if let Some(t) = hit.iter().max_by(|a, b| a.1.partial_cmp(&b.1).unwrap()) {
            let lead = if locusts { "Locusts darken the sky over" } else { "The blight and the black rye spread through" };
            said.push((format!("{lead} the fields around {}; {} towns lose their harvest.", t.2, hit.len()), hit.len() < 8));
        }
    }

    pub fn at(&self, g: &Grid<f64>, pos: (usize, usize)) -> f64 {
        g.at((pos.0 / B).min(self.eh - 1), (pos.1 / B).min(self.ew - 1))
    }
}
