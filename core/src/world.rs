//! The world, owned by Rust; Python reads it for the screen and the books.

use std::collections::HashSet;

use numpy::{IntoPyArray, PyArray2};
use pyo3::prelude::*;
use pyo3::types::PyDict;

use crate::geography::make_world;
use crate::grid::Grid;
use crate::people::NONE;
use crate::rng::Pcg64;
use crate::state::State;
use crate::tables::*;

#[pyclass(module = "realm_of_ages._core", unsendable)]
pub struct World {
    pub st: State,
}

fn arr<'py, T: numpy::Element + Copy>(py: Python<'py>, h: usize, w: usize, v: &[T]) -> Bound<'py, PyArray2<T>> {
    numpy::ndarray::Array2::from_shape_vec((h, w), v.to_vec()).unwrap().into_pyarray(py)
}

fn opt(x: usize) -> Option<usize> {
    if x == NONE { None } else { Some(x) }
}

impl World {
    pub fn compute_territory(&mut self) {
        let st = &mut self.st;
        let towns: Vec<(usize, usize, f64, bool)> =
            st.towns.iter().filter(|t| t.alive).map(|t| (t.pos.0, t.pos.1, t.pop, t.capital)).collect();
        let ids: Vec<usize> = st.towns.iter().filter(|t| t.alive).map(|t| t.id).collect();
        let terr = &st.map.terrain;
        let own = crate::territory_of(st.map.h, st.map.w, |r, c| terr.at(r, c) == OCEAN, &towns);
        st.owner = Grid { h: st.map.h, w: st.map.w, v: own.into_iter().map(|k| if k < 0 { -1 } else { ids[k as usize] as i64 }).collect() };
    }
}

#[pymethods]
impl World {
    /// A new world. `state` and `inc` are NumPy's PCG64 state for the seed (Python passes
    /// default_rng(seed).bit_generator.state), so seeds mean the same thing in both engines.
    #[new]
    fn new(width: usize, height: usize, state: u128, inc: u128) -> Self {
        let mut rng = Pcg64::from_state(state, inc);
        let map = make_world(height, width, &mut rng, 0.33);
        let (h, w) = (height, width);
        let mut relation = [[0.0; 4]; 4];
        for i in 0..4 { for j in 0..4 { relation[i][j] = BASE_REL[i][j] + rng.normal(0.0, 8.0); } }
        for i in 0..4 { for j in 0..i { let m = (relation[i][j] + relation[j][i]) / 2.0; relation[i][j] = m; relation[j][i] = m; } }
        let st = State { year: 0, rng, map, res_grid: Grid::new(h, w, -1), chars: vec![], houses: vec![], towns: vec![],
                         realms: vec![], ruins: vec![], used_names: HashSet::new(), blocked: Grid::new(h, w, false),
                         owner: Grid::new(h, w, -1), relation, log: vec![] };
        let mut world = World { st };
        world.st.place_resources();
        world.st.place_ruins();
        world.st.place_capitals(8);
        world.compute_territory();
        world
    }

    #[getter]
    fn year(&self) -> i64 { self.st.year }

    fn terrain<'py>(&self, py: Python<'py>) -> Bound<'py, PyArray2<i8>> { arr(py, self.st.map.h, self.st.map.w, &self.st.map.terrain.v) }
    fn landmass<'py>(&self, py: Python<'py>) -> Bound<'py, PyArray2<i32>> { arr(py, self.st.map.h, self.st.map.w, &self.st.map.landmass.v) }
    fn mana<'py>(&self, py: Python<'py>) -> Bound<'py, PyArray2<f64>> { arr(py, self.st.map.h, self.st.map.w, &self.st.map.mana.v) }
    fn owner<'py>(&self, py: Python<'py>) -> Bound<'py, PyArray2<i64>> { arr(py, self.st.map.h, self.st.map.w, &self.st.owner.v) }
    fn resources<'py>(&self, py: Python<'py>) -> Bound<'py, PyArray2<i16>> { arr(py, self.st.map.h, self.st.map.w, &self.st.res_grid.v) }

    fn lands<'py>(&self, py: Python<'py>) -> PyResult<Vec<Bound<'py, PyDict>>> {
        self.st.map.lands.iter().map(|l| {
            let d = PyDict::new(py);
            d.set_item("id", l.id)?; d.set_item("name", &l.name)?; d.set_item("size", l.size)?;
            d.set_item("kind", l.kind)?; d.set_item("center", l.center)?;
            Ok(d)
        }).collect()
    }

    fn towns<'py>(&self, py: Python<'py>) -> PyResult<Vec<Bound<'py, PyDict>>> {
        self.st.towns.iter().map(|t| {
            let d = PyDict::new(py);
            d.set_item("id", t.id)?; d.set_item("name", &t.name)?; d.set_item("realm", t.realm)?;
            d.set_item("pos", t.pos)?; d.set_item("pop", t.pop)?; d.set_item("capital", t.capital)?;
            d.set_item("port", t.port)?; d.set_item("alive", t.alive)?; d.set_item("landmass", t.landmass)?;
            Ok(d)
        }).collect()
    }

    fn realms<'py>(&self, py: Python<'py>) -> PyResult<Vec<Bound<'py, PyDict>>> {
        self.st.realms.iter().map(|k| {
            let d = PyDict::new(py);
            d.set_item("id", k.id)?; d.set_item("people", k.people)?; d.set_item("name", &k.name)?;
            d.set_item("short", &k.short)?; d.set_item("rank", k.rank)?; d.set_item("gov", k.gov)?;
            d.set_item("law", k.law)?; d.set_item("color", k.color)?; d.set_item("ruler", opt(k.ruler))?;
            d.set_item("house", opt(k.house))?;
            Ok(d)
        }).collect()
    }

    fn character<'py>(&self, py: Python<'py>, cid: usize) -> PyResult<Bound<'py, PyDict>> {
        let c = &self.st.chars[cid];
        let d = PyDict::new(py);
        d.set_item("id", c.id)?; d.set_item("name", &c.name)?; d.set_item("people", c.people)?;
        d.set_item("realm", c.realm)?; d.set_item("female", c.female)?; d.set_item("born", c.born)?;
        d.set_item("house", opt(c.house))?; d.set_item("father", opt(c.father))?; d.set_item("mother", opt(c.mother))?;
        d.set_item("spouse", opt(c.spouse))?; d.set_item("children", c.children.clone())?;
        d.set_item("genetic", c.genetic.iter().map(|&g| GENETIC[g]).collect::<Vec<_>>())?;
        d.set_item("latent", c.latent.iter().map(|&g| GENETIC[g]).collect::<Vec<_>>())?;
        d.set_item("personality", c.personality.iter().map(|&g| PERSONALITY[g]).collect::<Vec<_>>())?;
        let attrs = PyDict::new(py);
        for (i, a) in ATTRS.iter().enumerate() { attrs.set_item(*a, c.attrs[i] as i64)?; }
        d.set_item("attrs", attrs)?;
        d.set_item("prowess", c.prowess)?; d.set_item("talent", c.talent)?; d.set_item("life", c.life)?;
        d.set_item("classes", c.classes.iter().map(|&(k, n)| (CLASSES[k].name, n)).collect::<Vec<_>>())?;
        d.set_item("level", c.level)?; d.set_item("xp", c.xp)?;
        d.set_item("regnal", c.regnal.clone())?; d.set_item("legitimacy", c.legitimacy)?;
        d.set_item("gift", c.gift.map(|g| GIFTS[g]))?; d.set_item("alive", c.alive)?;
        let sk = PyDict::new(py);
        for (i, s) in SKILLS.iter().enumerate() { sk.set_item(*s, c.skills[i])?; }
        d.set_item("skills", sk)?;
        Ok(d)
    }

    fn n_characters(&self) -> usize { self.st.chars.len() }
    fn n_houses(&self) -> usize { self.st.houses.len() }
    fn n_ruins(&self) -> usize { self.st.ruins.len() }

    fn house_name(&self, hid: usize) -> String { self.st.houses[hid].name.clone() }

    /// The chronicle since a year: (year, minor, text).
    fn events(&self, since: i64) -> Vec<(i64, bool, String)> {
        self.st.log.iter().filter(|e| e.0 >= since).cloned().collect()
    }
}
