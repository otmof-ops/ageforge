//! The world, owned by Rust; Python reads it for the screen and the books.

use numpy::{IntoPyArray, PyArray2};
use pyo3::prelude::*;
use pyo3::types::PyDict;

use crate::geography::{make_world, Map};
use crate::rng::Pcg64;

#[pyclass(module = "realm_of_ages._core")]
pub struct World {
    pub year: i64,
    pub rng: Pcg64,
    pub map: Map,
}

fn arr<'py, T: numpy::Element + Copy>(py: Python<'py>, h: usize, w: usize, v: &[T]) -> Bound<'py, PyArray2<T>> {
    numpy::ndarray::Array2::from_shape_vec((h, w), v.to_vec()).unwrap().into_pyarray(py)
}

#[pymethods]
impl World {
    /// A new world. `state` and `inc` are NumPy's PCG64 state for the seed (Python passes
    /// default_rng(seed).bit_generator.state), so seeds mean the same thing in both engines.
    #[new]
    fn new(width: usize, height: usize, state: u128, inc: u128) -> Self {
        let mut rng = Pcg64::from_state(state, inc);
        let map = make_world(height, width, &mut rng, 0.33);
        World { year: 0, rng, map }
    }

    #[getter]
    fn year(&self) -> i64 {
        self.year
    }

    fn terrain<'py>(&self, py: Python<'py>) -> Bound<'py, PyArray2<i8>> {
        arr(py, self.map.h, self.map.w, &self.map.terrain.v)
    }

    fn landmass<'py>(&self, py: Python<'py>) -> Bound<'py, PyArray2<i32>> {
        arr(py, self.map.h, self.map.w, &self.map.landmass.v)
    }

    fn mana<'py>(&self, py: Python<'py>) -> Bound<'py, PyArray2<f64>> {
        arr(py, self.map.h, self.map.w, &self.map.mana.v)
    }

    fn lands<'py>(&self, py: Python<'py>) -> PyResult<Vec<Bound<'py, PyDict>>> {
        self.map.lands.iter().map(|l| {
            let d = PyDict::new(py);
            d.set_item("id", l.id)?;
            d.set_item("name", &l.name)?;
            d.set_item("size", l.size)?;
            d.set_item("kind", l.kind)?;
            d.set_item("center", l.center)?;
            Ok(d)
        }).collect()
    }
}
