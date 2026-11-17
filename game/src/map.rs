//! The world drawn: a PNG for the world's folder and colored half-blocks for the terminal.

use std::fs::File;
use std::io::BufWriter;
use std::path::Path;

use world::people::NONE;
use world::state::State;
use world::tables::{COLORS, TIER_WORDS};

pub const TERRAIN_COLORS: [[u8; 3]; 7] = [[36, 70, 120], [150, 178, 92], [46, 110, 56], [150, 136, 96], [125, 120, 116], [82, 104, 74], [214, 194, 132]];

pub fn realm_color(st: &State, k: usize) -> [u8; 3] {
    if k >= st.realms.len() { return [255, 255, 255]; }
    let c = st.realms[k].color;
    if c == [0, 0, 0] { COLORS[st.realms[k].people.min(3)] } else { c }
}

/// The realm holding each cell (-1 for none).
pub fn owner_realm(st: &State) -> Vec<i64> {
    st.owner.v.iter().map(|&o| if o < 0 { -1 } else { st.towns[o as usize].realm as i64 }).collect()
}

/// One color per map cell: terrain, mana's violet, corrupted land, the realm's tint, and its borders darkened.
pub fn cell_colors(st: &State) -> Vec<[u8; 3]> {
    let (h, w) = (st.map.h, st.map.w);
    let f = owner_realm(st);
    let mut out = Vec::with_capacity(h * w);
    for r in 0..h {
        for c in 0..w {
            let i = r * w + c;
            let t = st.map.terrain.v[i].max(0) as usize;
            let mut col = TERRAIN_COLORS[t.min(6)].map(|x| x as f64);
            let mm = ((st.map.mana.v[i] - 0.45) * 2.0).clamp(0.0, 1.0);
            for (k, v) in [170.0, 90.0, 230.0].iter().enumerate() { col[k] = col[k] * (1.0 - 0.55 * mm) + v * 0.55 * mm; }
            if st.corrupted.v[i] { for (k, v) in [90.0, 0.0, 60.0].iter().enumerate() { col[k] = col[k] * 0.4 + v * 0.6; } }
            if f[i] >= 0 {
                let rc = realm_color(st, f[i] as usize);
                for k in 0..3 { col[k] = col[k] * 0.55 + rc[k] as f64 * 0.45; }
                let edge = (c + 1 < w && f[i + 1] != f[i]) || (r + 1 < h && f[i + w] != f[i]);
                if edge { for k in 0..3 { col[k] *= 0.55; } }
            }
            out.push(col.map(|x| x.clamp(0.0, 255.0) as u8));
        }
    }
    out
}

// ---------------------------------------------------------------- a 3 by 5 pixel font for the PNG's labels
fn glyph(ch: char) -> [u8; 5] {
    match ch.to_ascii_uppercase() {
        'A' => [2, 5, 7, 5, 5], 'B' => [6, 5, 6, 5, 6], 'C' => [3, 4, 4, 4, 3], 'D' => [6, 5, 5, 5, 6], 'E' => [7, 4, 6, 4, 7],
        'F' => [7, 4, 6, 4, 4], 'G' => [3, 4, 5, 5, 3], 'H' => [5, 5, 7, 5, 5], 'I' => [7, 2, 2, 2, 7], 'J' => [1, 1, 1, 5, 2],
        'K' => [5, 5, 6, 5, 5], 'L' => [4, 4, 4, 4, 7], 'M' => [5, 7, 7, 5, 5], 'N' => [6, 5, 5, 5, 5], 'O' => [2, 5, 5, 5, 2],
        'P' => [6, 5, 6, 4, 4], 'Q' => [2, 5, 5, 6, 3], 'R' => [6, 5, 6, 5, 5], 'S' => [3, 4, 2, 1, 6], 'T' => [7, 2, 2, 2, 2],
        'U' => [5, 5, 5, 5, 7], 'V' => [5, 5, 5, 5, 2], 'W' => [5, 5, 7, 7, 5], 'X' => [5, 5, 2, 5, 5], 'Y' => [5, 5, 2, 2, 2],
        'Z' => [7, 1, 2, 4, 7], '0' => [7, 5, 5, 5, 7], '1' => [2, 6, 2, 2, 7], '2' => [6, 1, 2, 4, 7], '3' => [6, 1, 2, 1, 6],
        '4' => [5, 5, 7, 1, 1], '5' => [7, 4, 6, 1, 6], '6' => [3, 4, 6, 5, 2], '7' => [7, 1, 2, 2, 2], '8' => [2, 5, 2, 5, 2],
        '9' => [2, 5, 3, 1, 6], '-' => [0, 0, 7, 0, 0], ',' => [0, 0, 0, 2, 4], '.' => [0, 0, 0, 0, 2], ':' => [0, 2, 0, 2, 0],
        '(' => [1, 2, 2, 2, 1], ')' => [4, 2, 2, 2, 4], '\'' => [2, 2, 0, 0, 0], '/' => [1, 1, 2, 4, 4], '+' => [0, 2, 7, 2, 0],
        _ => [0, 0, 0, 0, 0],
    }
}

struct Canvas { w: usize, h: usize, px: Vec<[u8; 3]> }

impl Canvas {
    fn set(&mut self, x: i64, y: i64, c: [u8; 3]) {
        if x >= 0 && y >= 0 && (x as usize) < self.w && (y as usize) < self.h { self.px[y as usize * self.w + x as usize] = c; }
    }
    fn rect(&mut self, x0: i64, y0: i64, x1: i64, y1: i64, fill: [u8; 3], edge: Option<[u8; 3]>) {
        for y in y0..=y1 { for x in x0..=x1 {
            let on_edge = x == x0 || x == x1 || y == y0 || y == y1;
            self.set(x, y, if on_edge { edge.unwrap_or(fill) } else { fill });
        } }
    }
    fn disc(&mut self, cx: i64, cy: i64, r: f64, fill: [u8; 3], edge: Option<[u8; 3]>) {
        let ri = r.ceil() as i64;
        for y in -ri..=ri { for x in -ri..=ri {
            let d = ((x * x + y * y) as f64).sqrt();
            if d <= r { self.set(cx + x, cy + y, if d > r - 1.0 { edge.unwrap_or(fill) } else { fill }); }
        } }
    }
    fn text(&mut self, x: i64, y: i64, s: &str, c: [u8; 3], scale: i64) {
        let mut cx = x;
        for ch in s.chars() {
            let g = glyph(ch);
            for (row, bits) in g.iter().enumerate() {
                for col in 0..3 {
                    if bits & (4 >> col) != 0 {
                        for dy in 0..scale { for dx in 0..scale { self.set(cx + col as i64 * scale + dx, y + row as i64 * scale + dy, c); } }
                    }
                }
            }
            cx += 4 * scale;
        }
    }
}

/// The world as a PNG: the land and its realms, towns, ruins, monsters and hosts, with a header.
pub fn write_png(st: &State, path: &Path, seed: u64) -> std::io::Result<()> {
    let (h, w) = (st.map.h, st.map.w);
    let px = (1600 / w).clamp(2, 7);
    let top = 64usize;
    let mut cv = Canvas { w: w * px, h: h * px + top, px: vec![[18, 18, 24]; w * px * (h * px + top)] };
    let cells = cell_colors(st);
    for r in 0..h { for c in 0..w {
        let col = cells[r * w + c];
        for dy in 0..px { for dx in 0..px { cv.px[(top + r * px + dy) * cv.w + c * px + dx] = col; } }
    } }
    let at = |p: (usize, usize)| ((p.1 * px + px / 2) as i64, (p.0 * px + px / 2 + top) as i64);
    let cal = st.monsters.iter().find(|m| m.alive && m.kind == "calamity").map(|m| m.name.clone());
    let head = format!("REALM OF AGES - YEAR {} (SEED {seed})   HIGHEST TIER EVER CAST: {}{}", st.year, TIER_WORDS[st.world_tier.clamp(0, 10) as usize],
                       cal.map(|n| format!("   CALAMITY: {n}")).unwrap_or_default());
    cv.text(8, 6, &head, [235, 235, 235], 2);
    let mut big: Vec<usize> = (0..st.realms.len()).filter(|&k| !st.realms[k].extinct).collect();
    let towns_of = |k: usize| st.towns.iter().filter(|t| t.alive && t.realm == k).count();
    big.sort_by_key(|&k| std::cmp::Reverse(towns_of(k)));
    let col_w = (cv.w / 6) as i64;
    for (n, &k) in big.iter().take(6).enumerate() {
        let pop: f64 = st.towns.iter().filter(|t| t.alive && t.realm == k).map(|t| t.pop).sum();
        let c = realm_color(st, k);
        let name: String = st.realms[k].name.chars().take(30).collect();
        cv.text(8 + n as i64 * col_w, 26, &name, c, 2);
        cv.text(8 + n as i64 * col_w, 42, &format!("{} TOWNS, {:.0}K", towns_of(k), pop / 1000.0), c, 2);
    }
    for r in &st.ruins {
        let (x, y) = at(r.pos);
        let c = if r.loot.is_empty() { [200, 200, 200] } else { [255, 215, 0] };
        for d in -3..=3 { cv.set(x + d, y + d, c); cv.set(x + d, y - d, c); }
    }
    for l in st.map.lands.iter().filter(|l| l.kind == "continent") {
        let (x, y) = at(l.center);
        cv.text(x - 20, y, &l.name.to_uppercase(), [250, 250, 250], 2);
    }
    for t in st.towns.iter().filter(|t| t.alive) {
        let (x, y) = at(t.pos);
        let rad = 1.5 + t.pop.sqrt() / 20.0;
        let c = realm_color(st, t.realm);
        if t.capital {
            let r = rad.round() as i64 + 1;
            cv.rect(x - r, y - r, x + r, y + r, c, Some([255, 255, 255]));
            cv.text(x + r + 3, y - 4, &t.name, [255, 255, 255], 1);
        } else { cv.disc(x, y, rad, c, Some([0, 0, 0])); }
    }
    for a in &st.armies {
        let (x, y) = at(a.pos);
        let c = if a.realm != NONE { realm_color(st, a.realm) } else { [255, 255, 255] };
        let r = 3 + ((a.men / 1000.0).sqrt() * 2.0).min(8.0) as i64;
        cv.rect(x - r, y - r / 2, x + r, y + r / 2, c, Some([0, 0, 0]));
    }
    for m in st.monsters.iter().filter(|m| m.alive) {
        let (x, y) = at(m.pos);
        match m.kind {
            "dragon" => { for d in 0..9i64 { for e in -(d / 2)..=(d / 2) { cv.set(x + e, y - 4 + d, [230, 30, 20]); } } }
            "calamity" => { cv.disc(x, y, 9.0, [10, 10, 10], Some([255, 40, 40])); cv.text(x + 12, y - 4, m.name.split(',').next().unwrap_or(""), [255, 80, 80], 1); }
            "demon" => cv.rect(x - 4, y - 4, x + 4, y + 4, [255, 120, 0], Some([0, 0, 0])),
            "wraith" => cv.disc(x, y, 4.0, [235, 235, 255], Some([0, 0, 0])),
            _ if m.sea => cv.disc(x, y, 4.0, [40, 230, 230], Some([0, 0, 0])),
            _ => cv.disc(x, y, 3.0, [110, 70, 30], Some([0, 0, 0])),
        }
    }
    let file = BufWriter::new(File::create(path)?);
    let mut enc = png::Encoder::new(file, cv.w as u32, cv.h as u32);
    enc.set_color(png::ColorType::Rgb);
    enc.set_depth(png::BitDepth::Eight);
    let mut wr = enc.write_header().map_err(std::io::Error::other)?;
    let data: Vec<u8> = cv.px.iter().flat_map(|p| p.iter().copied()).collect();
    wr.write_image_data(&data).map_err(std::io::Error::other)?;
    Ok(())
}

/// One terminal cell of the map: the upper and lower half colors, and a symbol drawn over them if any.
pub struct TermCell { pub top: [u8; 3], pub bottom: [u8; 3], pub symbol: Option<(char, [u8; 3])> }

/// The map fitted to `cols` by `rows` terminal cells, two map rows to a cell (half blocks).
pub fn terminal(st: &State, cols: usize, rows: usize) -> Vec<Vec<TermCell>> {
    let (h, w) = (st.map.h, st.map.w);
    let cells = cell_colors(st);
    let sx = w.div_ceil(cols.max(1)).max(1);
    let sy = h.div_ceil(rows.max(1) * 2).max(1);
    let (nc, nr) = (w / sx, h / (2 * sy));
    let sample = |r: usize, c: usize| cells[(r.min(h - 1)) * w + c.min(w - 1)];
    let mut out: Vec<Vec<TermCell>> = (0..nr).map(|r| (0..nc).map(|c| TermCell { top: sample(2 * r * sy + sy / 2, c * sx + sx / 2), bottom: sample((2 * r + 1) * sy + sy / 2, c * sx + sx / 2), symbol: None }).collect()).collect();
    let mut mark = |p: (usize, usize), ch: char, col: [u8; 3]| {
        let (r, c) = (p.0 / (2 * sy), p.1 / sx);
        if r < nr && c < nc { out[r][c].symbol = Some((ch, col)); }
    };
    for a in &st.armies { mark(a.pos, '⚔', [255, 255, 255]); }
    for m in st.monsters.iter().filter(|m| m.alive && matches!(m.kind, "dragon" | "calamity" | "demon")) {
        mark(m.pos, if m.kind == "calamity" { '✸' } else if m.kind == "dragon" { '▲' } else { '◆' }, if m.kind == "calamity" { [255, 60, 60] } else { [255, 140, 40] });
    }
    for t in st.towns.iter().filter(|t| t.alive && t.capital) { mark(t.pos, '♛', [255, 255, 255]); }
    out
}
