//! A 2-D grid of values and the image operations the world needs: smoothed value noise, quantiles, dilation,
//! 4-connected labels, and the exact Euclidean distance transform (Felzenszwalb-Huttenlocher) with the nearest
//! feature's index, as in SciPy.

use crate::rng::Pcg64;

#[derive(Clone)]
pub struct Grid<T> {
    pub h: usize,
    pub w: usize,
    pub v: Vec<T>,
}

impl<T: Copy> Grid<T> {
    pub fn new(h: usize, w: usize, fill: T) -> Self {
        Grid { h, w, v: vec![fill; h * w] }
    }
    #[inline]
    pub fn at(&self, r: usize, c: usize) -> T {
        self.v[r * self.w + c]
    }
    #[inline]
    pub fn set(&mut self, r: usize, c: usize, x: T) {
        self.v[r * self.w + c] = x;
    }
}

/// Smooth value noise over `octaves`, normalised to [0, 1] (the same construction as geography.value_noise).
pub fn value_noise(h: usize, w: usize, rng: &mut Pcg64, octaves: u32, base: u32) -> Grid<f64> {
    let mut out = Grid::new(h, w, 0.0f64);
    for o in 0..octaves {
        let cy = 2usize.pow(o + base);
        let cx = ((cy as f64 * w as f64 / h as f64).round() as usize).max(2);
        let lat: Vec<f64> = (0..(cy + 1) * (cx + 1)).map(|_| rng.random()).collect();
        let at = |y: usize, x: usize| lat[y * (cx + 1) + x];
        let amp = 1.0 / 2f64.powi(o as i32);
        for r in 0..h {
            let ys = r as f64 * cy as f64 / h as f64;
            let iy = ys as usize;
            let mut fy = ys - iy as f64;
            fy = fy * fy * (3.0 - 2.0 * fy);
            for c in 0..w {
                let xs = c as f64 * cx as f64 / w as f64;
                let ix = xs as usize;
                let mut fx = xs - ix as f64;
                fx = fx * fx * (3.0 - 2.0 * fx);
                let (a, b, cc, d) = (at(iy, ix), at(iy, ix + 1), at(iy + 1, ix), at(iy + 1, ix + 1));
                let top = a + (b - a) * fx;
                let bot = cc + (d - cc) * fx;
                out.v[r * w + c] += (top + (bot - top) * fy) * amp;
            }
        }
    }
    let (mn, mx) = out.v.iter().fold((f64::MAX, f64::MIN), |(a, b), &x| (a.min(x), b.max(x)));
    for x in out.v.iter_mut() {
        *x = (*x - mn) / (mx - mn);
    }
    out
}

/// The q-quantile of a set of values (linear interpolation, as NumPy's default).
pub fn quantile(vals: &[f64], q: f64) -> f64 {
    let mut s: Vec<f64> = vals.to_vec();
    s.sort_by(|a, b| a.partial_cmp(b).unwrap());
    if s.is_empty() {
        return 0.0;
    }
    let pos = q * (s.len() - 1) as f64;
    let lo = pos.floor() as usize;
    let hi = pos.ceil() as usize;
    s[lo] + (s[hi] - s[lo]) * (pos - lo as f64)
}

/// Cells of `a` or touching one (4-neighbourhood).
pub fn dilate(a: &Grid<bool>) -> Grid<bool> {
    let mut out = a.clone();
    for r in 0..a.h {
        for c in 0..a.w {
            if a.at(r, c) {
                if r > 0 { out.set(r - 1, c, true); }
                if r + 1 < a.h { out.set(r + 1, c, true); }
                if c > 0 { out.set(r, c - 1, true); }
                if c + 1 < a.w { out.set(r, c + 1, true); }
            }
        }
    }
    out
}

/// 4-connected components of the true cells, numbered 0.. in scan order (-1 elsewhere); returns (labels, count).
pub fn label(a: &Grid<bool>) -> (Grid<i32>, usize) {
    let mut lab = Grid::new(a.h, a.w, -1i32);
    let mut n = 0i32;
    let mut stack = Vec::new();
    for r in 0..a.h {
        for c in 0..a.w {
            if a.at(r, c) && lab.at(r, c) < 0 {
                lab.set(r, c, n);
                stack.push((r, c));
                while let Some((y, x)) = stack.pop() {
                    let push = |yy: usize, xx: usize, lab: &mut Grid<i32>, st: &mut Vec<(usize, usize)>| {
                        if a.at(yy, xx) && lab.at(yy, xx) < 0 {
                            lab.set(yy, xx, n);
                            st.push((yy, xx));
                        }
                    };
                    if y > 0 { push(y - 1, x, &mut lab, &mut stack); }
                    if y + 1 < a.h { push(y + 1, x, &mut lab, &mut stack); }
                    if x > 0 { push(y, x - 1, &mut lab, &mut stack); }
                    if x + 1 < a.w { push(y, x + 1, &mut lab, &mut stack); }
                }
                n += 1;
            }
        }
    }
    (lab, n as usize)
}

fn edt_1d(f: &[f64], d: &mut [f64], arg: &mut [usize]) {
    let n = f.len();
    let inf = 1e20;
    let mut v = vec![0usize; n];
    let mut z = vec![0.0f64; n + 1];
    let mut k = 0usize;
    let mut started = false;
    for q in 0..n {
        if f[q] >= inf {
            continue;
        }
        if !started {
            v[0] = q;
            z[0] = -inf;
            z[1] = inf;
            started = true;
            continue;
        }
        loop {
            let p = v[k];
            let s = ((f[q] + (q * q) as f64) - (f[p] + (p * p) as f64)) / (2.0 * q as f64 - 2.0 * p as f64);
            if s <= z[k] {
                if k == 0 {
                    v[0] = q;
                    z[0] = -inf;
                    z[1] = inf;
                    break;
                }
                k -= 1;
                continue;
            }
            k += 1;
            v[k] = q;
            z[k] = s;
            z[k + 1] = inf;
            break;
        }
    }
    if !started {
        for q in 0..n {
            d[q] = inf;
            arg[q] = usize::MAX;
        }
        return;
    }
    k = 0;
    for q in 0..n {
        while z[k + 1] < q as f64 {
            k += 1;
        }
        let p = v[k];
        d[q] = ((q as f64 - p as f64).powi(2)) + f[p];
        arg[q] = p;
    }
}

/// Distance from every cell where `feature` is false to the nearest cell where it is true, with that cell.
pub fn edt(feature: &Grid<bool>) -> (Grid<f64>, Grid<(usize, usize)>) {
    let (h, w) = (feature.h, feature.w);
    let inf = 1e20;
    let mut col_d = Grid::new(h, w, inf);
    let mut col_i = Grid::new(h, w, 0usize);
    let mut f = vec![0.0; h];
    let mut d = vec![0.0; h];
    let mut a = vec![0usize; h];
    for x in 0..w {
        for y in 0..h {
            f[y] = if feature.at(y, x) { 0.0 } else { inf };
        }
        edt_1d(&f, &mut d, &mut a);
        for y in 0..h {
            col_d.set(y, x, d[y]);
            col_i.set(y, x, a[y]);
        }
    }
    let mut dist = Grid::new(h, w, 0.0f64);
    let mut idx = Grid::new(h, w, (0usize, 0usize));
    let mut f = vec![0.0; w];
    let mut d = vec![0.0; w];
    let mut a = vec![0usize; w];
    for y in 0..h {
        for x in 0..w {
            f[x] = col_d.at(y, x);
        }
        edt_1d(&f, &mut d, &mut a);
        for x in 0..w {
            dist.set(y, x, d[x].sqrt());
            let ax = if a[x] == usize::MAX { x } else { a[x] };
            idx.set(y, x, (col_i.at(y, ax).min(h - 1), ax));
        }
    }
    (dist, idx)
}
