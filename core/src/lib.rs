//! The heavy yearly work of the Realm of Ages simulation, moved out of Python one system at a time.
//!
//! Every function here has a Python twin it must agree with exactly; the differential tests run both from the
//! same world and compare. Random numbers come from `rng::Pcg64`, the same generator NumPy's `default_rng` uses,
//! with its state passed in and handed back, so a system can move across without changing the world's history.

use numpy::{PyArray2, PyReadonlyArray1, PyReadonlyArray2, IntoPyArray};
use pyo3::prelude::*;

mod rng;

/// Which town's territory each map cell falls in: every town casts influence over a disc whose radius grows with
/// its population, stronger near the town and for a seat; the strongest influence takes the cell (the earlier town
/// on a tie). Sea cells are taken only within two leagues of a town. Returns the owner grid (-1 for no one).
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
    let (h, w) = (terrain.shape()[0] as i64, terrain.shape()[1] as i64);
    let (rows, cols, pops, capital) = (rows.as_slice()?, cols.as_slice()?, pops.as_slice()?, capital.as_slice()?);
    let mut owner = vec![-1i64; (h * w) as usize];
    let mut best = vec![0.0f64; (h * w) as usize];
    for k in 0..rows.len() {
        let pop = pops[k].max(1.0);
        let rad = (2.0 + (pop.sqrt() / 8.0).min(6.0)) as i64;
        let strength = pop.powf(0.35) * if capital[k] { 1.6 } else { 1.0 };
        let (r0, c0) = (rows[k], cols[k]);
        for r in (r0 - rad).max(0)..(r0 + rad + 1).min(h) {
            for c in (c0 - rad).max(0)..(c0 + rad + 1).min(w) {
                let d = ((r - r0) as f64).hypot((c - c0) as f64);
                if d > rad as f64 {
                    continue;
                }
                if terrain[[r as usize, c as usize]] == ocean && d > 2.0 {
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
    let arr = numpy::ndarray::Array2::from_shape_vec((h as usize, w as usize), owner)
        .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))?;
    Ok(arr.into_pyarray(py))
}

#[pymodule]
fn _core(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(territory, m)?)?;
    m.add_function(wrap_pyfunction!(rng::pcg64_doubles, m)?)?;
    Ok(())
}
