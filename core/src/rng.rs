//! NumPy's PCG64 (XSL-RR 128/64), stepped exactly as numpy.random.default_rng steps it, so Rust and Python can
//! hand one generator back and forth. `random()` is `(next_u64 >> 11) * 2^-53`, as in NumPy.

#[cfg(feature = "python")]
use pyo3::prelude::*;

const MULT: u128 = (2549297995355413924u128 << 64) | 4865540595714422341u128;

pub struct Pcg64 {
    pub state: u128,
    pub inc: u128,
}

impl Pcg64 {
    #[inline]
    pub fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_mul(MULT).wrapping_add(self.inc);
        let s = self.state;
        let xored = ((s >> 64) as u64) ^ (s as u64);
        xored.rotate_right((s >> 122) as u32)
    }

    #[inline]
    pub fn random(&mut self) -> f64 {
        (self.next_u64() >> 11) as f64 * (1.0 / 9007199254740992.0)
    }
}

/// n draws of `random()` from the given state; returns the draws and the state after them (for testing parity).
#[cfg(feature = "python")]
#[pyfunction]
pub fn pcg64_doubles(state: u128, inc: u128, n: usize) -> (Vec<f64>, u128) {
    let mut g = Pcg64 { state, inc };
    let out = (0..n).map(|_| g.random()).collect();
    (out, g.state)
}

impl Pcg64 {
    /// Seeded like numpy.random.default_rng(seed): the state is taken from NumPy at world creation (see
    /// World::new), so a Rust world and a Python world of one seed begin from the same generator.
    pub fn from_state(state: u128, inc: u128) -> Self {
        Pcg64 { state, inc }
    }

    /// A whole number in [lo, hi) (Lemire's nearly-divisionless method).
    pub fn integers(&mut self, lo: i64, hi: i64) -> i64 {
        let range = (hi - lo) as u64;
        if range == 0 {
            return lo;
        }
        let mut m = (self.next_u64() as u128) * (range as u128);
        let mut l = m as u64;
        if l < range {
            let t = range.wrapping_neg() % range;
            while l < t {
                m = (self.next_u64() as u128) * (range as u128);
                l = m as u64;
            }
        }
        lo + (m >> 64) as i64
    }

    pub fn uniform(&mut self, a: f64, b: f64) -> f64 {
        a + (b - a) * self.random()
    }

    /// Normal draws by the polar method (statistically NumPy's normal, not its exact stream).
    pub fn normal(&mut self, mu: f64, sigma: f64) -> f64 {
        loop {
            let u = 2.0 * self.random() - 1.0;
            let v = 2.0 * self.random() - 1.0;
            let s = u * u + v * v;
            if s > 0.0 && s < 1.0 {
                return mu + sigma * u * (-2.0 * s.ln() / s).sqrt();
            }
        }
    }

    pub fn choose<'a, T>(&mut self, xs: &'a [T]) -> &'a T {
        &xs[self.integers(0, xs.len() as i64) as usize]
    }

    /// An index drawn with the given weights (which need not sum to one).
    pub fn weighted(&mut self, w: &[f64]) -> usize {
        let total: f64 = w.iter().sum();
        let mut x = self.random() * total;
        for (i, v) in w.iter().enumerate() {
            if x < *v {
                return i;
            }
            x -= v;
        }
        w.len() - 1
    }
}
