//! NumPy's PCG64 (XSL-RR 128/64), stepped exactly as numpy.random.default_rng steps it, so Rust and Python can
//! hand one generator back and forth. `random()` is `(next_u64 >> 11) * 2^-53`, as in NumPy.

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
#[pyfunction]
pub fn pcg64_doubles(state: u128, inc: u128, n: usize) -> (Vec<f64>, u128) {
    let mut g = Pcg64 { state, inc };
    let out = (0..n).map(|_| g.random()).collect();
    (out, g.state)
}
