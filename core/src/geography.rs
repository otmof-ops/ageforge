//! The map: continents and island chains, terrain, mana, landmasses and seas (after geography.make_world).

use crate::grid::{dilate, edt, label, quantile, value_noise, Grid};
use crate::rng::Pcg64;
use crate::tables::*;

pub struct Land {
    pub id: usize,
    pub name: String,
    pub size: usize,
    pub kind: &'static str, // "continent", "great isle", "isle"
    pub center: (usize, usize),
}

pub struct Map {
    pub h: usize,
    pub w: usize,
    pub terrain: Grid<i8>,
    pub mana: Grid<f64>,
    pub landmass: Grid<i32>,
    pub lands: Vec<Land>,
    pub coast: Grid<bool>,
    pub sea_dist: Grid<f64>,
    pub sea_of: Grid<i32>,
    pub sea_names: Vec<(usize, String)>,
}

pub fn make_world(h: usize, w: usize, rng: &mut Pcg64, land_frac: f64) -> Map {
    let n = rng.integers(6, 9) as usize;
    let scale = h.min(w) as f64;
    let wy = value_noise(h, w, rng, 5, 2);
    let wx = value_noise(h, w, rng, 5, 2);
    let mut blobs: Vec<(f64, f64, f64, f64, f64)> = Vec::new();
    for i in 0..n {
        let cy = rng.uniform(0.15, 0.85) * h as f64;
        let cx = rng.uniform(0.08, 0.92) * w as f64;
        let ry = rng.uniform(0.13, 0.26) * scale * if i < 2 { 1.4 } else { 1.0 };
        let rx = ry * rng.uniform(0.9, 1.9);
        let ang = rng.uniform(0.0, std::f64::consts::PI);
        blobs.push((cy, cx, ry, rx, ang));
    }
    // the nearest and second-nearest blob distance at every cell
    let mut d0 = Grid::new(h, w, f64::MAX);
    let mut d1 = Grid::new(h, w, f64::MAX);
    for r in 0..h {
        for c in 0..w {
            let i = r * w + c;
            let (dy0, dx0) = (r as f64 + (wy.v[i] - 0.5) * scale * 0.35, c as f64 + (wx.v[i] - 0.5) * scale * 0.35);
            for &(cy, cx, ry, rx, ang) in &blobs {
                let (dy, dx) = (dy0 - cy, dx0 - cx);
                let u = (dx * ang.cos() + dy * ang.sin()) / rx;
                let v = (-dx * ang.sin() + dy * ang.cos()) / ry;
                let d = u.hypot(v);
                if d < d0.v[i] {
                    d1.v[i] = d0.v[i];
                    d0.v[i] = d;
                } else if d < d1.v[i] {
                    d1.v[i] = d;
                }
            }
        }
    }
    let mut bump = Grid::new(h, w, 0.0f64);
    for i in 0..h * w {
        bump.v[i] = (1.0 - d0.v[i]).clamp(0.0, 1.0) * ((d1.v[i] - d0.v[i]) / 0.25).clamp(0.0, 1.0);
    }
    for _ in 0..rng.integers(5, 10) {
        let (mut y0, mut x0) = (rng.uniform(0.0, h as f64), rng.uniform(0.0, w as f64));
        let mut ang = rng.uniform(0.0, 2.0 * std::f64::consts::PI);
        for _ in 0..rng.integers(4, 12) {
            ang += rng.normal(0.0, 0.4);
            y0 += ang.sin() * scale * 0.04;
            x0 += ang.cos() * scale * 0.04;
            let rr = rng.uniform(0.008, 0.025) * scale;
            let (r_lo, r_hi) = (((y0 - rr).floor().max(0.0)) as usize, ((y0 + rr).ceil().min(h as f64 - 1.0)).max(0.0) as usize);
            let (c_lo, c_hi) = (((x0 - rr).floor().max(0.0)) as usize, ((x0 + rr).ceil().min(w as f64 - 1.0)).max(0.0) as usize);
            for r in r_lo..=r_hi.min(h - 1) {
                for c in c_lo..=c_hi.min(w - 1) {
                    let v = (1.0 - (r as f64 - y0).hypot(c as f64 - x0) / rr).clamp(0.0, 1.0) * 0.9;
                    let i = r * w + c;
                    if v > bump.v[i] {
                        bump.v[i] = v;
                    }
                }
            }
        }
    }
    let ns = value_noise(h, w, rng, 5, 3);
    let elev: Vec<f64> = (0..h * w).map(|i| bump.v[i] * 0.8 + ns.v[i] * 0.25).collect();
    let cut = quantile(&elev, 1.0 - land_frac);
    let mut land = Grid::new(h, w, false);
    for r in 0..h {
        for c in 0..w {
            let i = r * w + c;
            let edge = r < 2 || r >= h - 2 || c < 2 || c >= w - 2;
            land.v[i] = !edge && elev[i] >= cut && (d1.v[i] - d0.v[i]) > 0.06;
        }
    }
    let moist = value_noise(h, w, rng, 5, 3);
    let land_idx: Vec<usize> = (0..h * w).filter(|&i| land.v[i]).collect();
    let lm: Vec<f64> = land_idx.iter().map(|&i| moist.v[i]).collect();
    let le: Vec<f64> = land_idx.iter().map(|&i| elev[i]).collect();
    let (mq0, mq1, mq2) = (quantile(&lm, 0.18), quantile(&lm, 0.55), quantile(&lm, 0.86));
    let (e45, e80, e93) = (quantile(&le, 0.45), quantile(&le, 0.80), quantile(&le, 0.93));
    let mut terrain = Grid::new(h, w, OCEAN);
    for r in 0..h {
        let lat = ((r as f64 / h as f64) - 0.5).abs() * 2.0;
        for c in 0..w {
            let i = r * w + c;
            if !land.v[i] {
                continue;
            }
            let (m, e) = (moist.v[i], elev[i]);
            let mut t = PLAINS;
            if m > mq1 { t = FOREST; }
            if m < mq0 && lat < 0.6 { t = DESERT; }
            if m > mq2 && e < e45 { t = SWAMP; }
            if e >= e80 { t = HILLS; }
            if e >= e93 { t = MOUNTAIN; }
            terrain.v[i] = t;
        }
    }
    let ley = value_noise(h, w, rng, 3, 3);
    let mut mana = Grid::new(h, w, 0.0f64);
    for i in 0..h * w {
        mana.v[i] = (-((ley.v[i] - 0.5) / 0.025).powi(2)).exp();
    }
    for _ in 0..4 * n {
        let (pr, pc) = (rng.integers(0, h as i64) as f64, rng.integers(0, w as i64) as f64);
        let (r_lo, r_hi) = ((pr - 16.0).max(0.0) as usize, ((pr + 16.0) as usize).min(h - 1));
        let (c_lo, c_hi) = ((pc - 16.0).max(0.0) as usize, ((pc + 16.0) as usize).min(w - 1));
        for r in r_lo..=r_hi {
            for c in c_lo..=c_hi {
                mana.v[r * w + c] += 0.9 * (-((r as f64 - pr).powi(2) + (c as f64 - pc).powi(2)) / 32.0).exp();
            }
        }
    }
    for i in 0..h * w {
        mana.v[i] = if land.v[i] { mana.v[i].clamp(0.0, 1.0) } else { 0.0 };
    }
    // landmasses, named by size
    let (lab, count) = label(&land);
    let mut sizes = vec![0usize; count];
    let mut sy = vec![0.0f64; count];
    let mut sx = vec![0.0f64; count];
    for r in 0..h {
        for c in 0..w {
            let l = lab.at(r, c);
            if l >= 0 {
                sizes[l as usize] += 1;
                sy[l as usize] += r as f64;
                sx[l as usize] += c as f64;
            }
        }
    }
    let mut order: Vec<usize> = (0..count).collect();
    order.sort_by_key(|&i| std::cmp::Reverse(sizes[i]));
    let mut used = std::collections::HashSet::new();
    let mut lands: Vec<Land> = Vec::new();
    for &i in &order {
        let size = sizes[i];
        let kind = if size >= 1500 { "continent" } else if size >= 300 { "great isle" } else { "isle" };
        let mut nm = String::new();
        for _ in 0..100 {
            nm = match kind {
                "continent" => format!("{}{}", rng.choose(&LAND_A), rng.choose(&LAND_B)),
                "great isle" => format!("{}{} Isle", rng.choose(&LAND_A), rng.choose(&LAND_B)),
                _ => format!("{} Isle", rng.choose(&ISLE_A)),
            };
            if used.insert(nm.clone()) {
                break;
            }
        }
        lands.push(Land { id: i, name: nm, size, kind,
                          center: ((sy[i] / size as f64) as usize, (sx[i] / size as f64) as usize) });
    }
    lands.sort_by_key(|l| l.id);
    let coast = {
        let sea = Grid { h, w, v: land.v.iter().map(|&x| !x).collect() };
        let near_sea = dilate(&sea);
        Grid { h, w, v: (0..h * w).map(|i| land.v[i] && near_sea.v[i]).collect() }
    };
    let (sea_dist, _) = edt(&land);
    // each sea cell takes the name of the nearest landmass that is not a small isle
    let big: Vec<bool> = lands.iter().map(|l| l.kind != "isle").collect();
    let feature = Grid { h, w, v: (0..h * w).map(|i| lab.v[i] >= 0 && big[lab.v[i] as usize]).collect() };
    let (_, idx) = edt(&feature);
    let mut sea_of = Grid::new(h, w, -1i32);
    for i in 0..h * w {
        if !land.v[i] {
            let (r, c) = idx.v[i];
            sea_of.v[i] = lab.at(r, c);
        }
    }
    let sea_names = lands.iter().filter(|l| l.kind != "isle")
        .map(|l| (l.id, format!("{} {}", rng.choose(&SEA_A), l.name))).collect();
    Map { h, w, terrain, mana, landmass: lab, lands, coast, sea_dist, sea_of, sea_names }
}
