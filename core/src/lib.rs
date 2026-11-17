//! The Realm of Ages world. `engine` founds and runs it; the game (`game/`) plays it; with the `python` feature it is
//! also the Python extension `realm_of_ages._core`, which the reference engine's checks use.
//!
//! Every function here has a Python twin it must agree with exactly; the differential tests run both from the
//! same world and compare. Random numbers come from `rng::Pcg64`, the same generator NumPy's `default_rng` uses,
//! with its state passed in and handed back, so a system can move across without changing the world's history.

#[cfg(feature = "python")]
use numpy::{PyArray2, PyReadonlyArray1, PyReadonlyArray2, IntoPyArray};
#[cfg(feature = "python")]
use pyo3::prelude::*;

pub mod beasts;
pub mod bending;
pub mod civics;
pub mod climate;
pub mod court;
pub mod crafts;
pub mod economy;
pub mod legends;
pub mod wardens;
pub mod heroes;
pub mod monsters;
pub mod ecology;
pub mod dynasty;
#[cfg(feature = "python")]
pub mod export;
pub mod geography;
pub mod godmode;
pub mod governance;
pub mod heart;
pub mod grid;
pub mod opinion;
pub mod people;
pub mod populace;
pub mod power;
pub mod sea;
pub mod shinobi;
pub mod society;
pub mod state;
pub mod stewardship;
pub mod titles;
pub mod war;
pub mod rng;
pub mod tables;
pub mod works;
#[cfg(feature = "python")]
pub mod world;
pub mod engine;

/// Which town's territory each map cell falls in: every town casts influence over a disc whose radius grows with
/// its population, stronger near the town and for a seat; the strongest influence takes the cell (the earlier town
/// on a tie). Sea cells are taken only within two leagues of a town. Returns the owner grid (-1 for no one).
#[cfg(feature = "python")]
#[pyfunction]
fn territory<'py>(
    py: Python<'py>,
    terrain: PyReadonlyArray2<'py, i64>,
    ocean: i64,
    rows: PyReadonlyArray1<'py, i64>,
    cols: PyReadonlyArray1<'py, i64>,
    pops: PyReadonlyArray1<'py, f64>,
    capital: PyReadonlyArray1<'py, bool>,
) -> PyResult<Bound<'py, PyArray2<i64>>> {
    let terrain = terrain.as_array();
    let (h, w) = (terrain.shape()[0], terrain.shape()[1]);
    let (rows, cols, pops, capital) = (rows.as_slice()?, cols.as_slice()?, pops.as_slice()?, capital.as_slice()?);
    let towns: Vec<(usize, usize, f64, bool)> =
        (0..rows.len()).map(|k| (rows[k] as usize, cols[k] as usize, pops[k], capital[k])).collect();
    let owner = territory_of(h, w, |r, c| terrain[[r, c]] == ocean, &towns);
    let (h, w) = (h as i64, w as i64);
    let arr = numpy::ndarray::Array2::from_shape_vec((h as usize, w as usize), owner)
        .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))?;
    Ok(arr.into_pyarray(py))
}

/// The territory rule itself, for Python's call above and for the Rust world.
pub fn territory_of(h: usize, w: usize, is_ocean: impl Fn(usize, usize) -> bool,
                    towns: &[(usize, usize, f64, bool)]) -> Vec<i64> {
    let (h, w) = (h as i64, w as i64);
    let mut owner = vec![-1i64; (h * w) as usize];
    let mut best = vec![0.0f64; (h * w) as usize];
    for (k, &(r0, c0, pop, cap)) in towns.iter().enumerate() {
        let pop = pop.max(1.0);
        let rad = (2.0 + (pop.sqrt() / 8.0).min(6.0)) as i64;
        let strength = pop.powf(0.35) * if cap { 1.6 } else { 1.0 };
        let (r0, c0) = (r0 as i64, c0 as i64);
        for r in (r0 - rad).max(0)..(r0 + rad + 1).min(h) {
            for c in (c0 - rad).max(0)..(c0 + rad + 1).min(w) {
                let d = ((r - r0) as f64).hypot((c - c0) as f64);
                if d > rad as f64 || (is_ocean(r as usize, c as usize) && d > 2.0) {
                    continue;
                }
                let p = strength * (1.0 / (1.0 + d));
                let i = (r * w + c) as usize;
                if p > best[i] {
                    best[i] = p;
                    owner[i] = k as i64;
                }
            }
        }
    }
    owner
}

/// "the open sea" -> "Open Sea": a title for a place.
pub fn bending_title(s: &str) -> String {
    s.split(' ').map(|w| { let mut c = w.chars(); match c.next() { Some(f) => f.to_uppercase().collect::<String>() + c.as_str(), None => String::new() } }).collect::<Vec<_>>().join(" ")
}

#[cfg(feature = "python")]
#[pymodule]
fn _core(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(territory, m)?)?;
    m.add_function(wrap_pyfunction!(rng::pcg64_doubles, m)?)?;
    m.add_class::<world::World>()?;
    Ok(())
}
