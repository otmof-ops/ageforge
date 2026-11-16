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
        // as world.Realm: the four first rows take the peoples' table (indexed by realm id, see PORT.md "Python bugs")
        let mut relation = vec![vec![0.0; 4]; 4];
        for i in 0..4 { for j in 0..4 { relation[i][j] = BASE_REL[i][j] + rng.normal(0.0, 8.0); } }
        for i in 0..4 { for j in 0..i { let m = (relation[i][j] + relation[j][i]) / 2.0; relation[i][j] = m; relation[j][i] = m; } }
        let st = State { year: 0, rng, map, res_grid: Grid::new(h, w, -1), chars: vec![], houses: vec![], towns: vec![],
                         realms: vec![], ruins: vec![], used_names: HashSet::new(), blocked: Grid::new(h, w, false),
                         owner: Grid::new(h, w, -1), relation, stats_defaults: vec![], stats_forced: vec![], mined: 0.0,
                         bankruptcies: vec![], risings: vec![], epidemics: vec![], last_pandemic: -999, last_plague: -999, log: vec![],
                         climate: crate::climate::Climate::setup(&Grid::new(1, 1, 0i8), &mut Pcg64::from_state(0, 1)), blight: 0,
                         cultures: (0..4).map(|p| crate::people::Culture { people: p, innovations: Default::default() }).collect(),
                         famines: vec![], migrants: 0.0, grain_moved: 0.0 };
        let mut world = World { st };
        world.st.place_resources();
        world.st.place_ruins();
        world.st.place_capitals(8);
        world.compute_territory();
        world.st.grow_tables();
        world.st.climate = crate::climate::Climate::setup(&world.st.map.terrain, &mut world.st.rng);
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
        d.set_item("cr", self.st.cr(c, false))?; d.set_item("tier", self.st.tier(c))?;
        d.set_item("gate", self.st.power_gate(c, false))?; d.set_item("cause", c.cause.clone())?;
        d.set_item("epithet", c.epithet.clone())?; d.set_item("renown", c.renown)?;
        let sk = PyDict::new(py);
        for (i, s) in SKILLS.iter().enumerate() { sk.set_item(*s, c.skills[i])?; }
        d.set_item("skills", sk)?;
        Ok(d)
    }

    /// Run the world forward. (Slice 2: the land's year; the rest of the year arrives slice by slice.)
    fn step(&mut self, years: i64) {
        for _ in 0..years {
            self.st.year += 1;
            self.compute_territory();
            self.st.weather_year();
            self.st.realm_resources();
            self.st.land_year();
            self.st.expand();
            self.st.dynasty_year();
            self.st.opinion_year();
            self.st.treasury_year();
            self.st.trade_year();
            self.st.money_year();
            self.st.manage_year();
            self.st.moods_year();
            self.st.plague_year();
        }
    }

    /// Totals for checking: (living towns, people, mean poverty, famine years so far, grain moved, migrants).
    fn totals(&self) -> (usize, f64, f64, usize, f64, f64) {
        let live: Vec<&crate::people::Town> = self.st.towns.iter().filter(|t| t.alive).collect();
        let pop: f64 = live.iter().map(|t| t.pop).sum();
        let pov = live.iter().map(|t| t.poverty * t.pop).sum::<f64>() / pop.max(1.0);
        (live.len(), pop, pov, self.st.famines.len(), self.st.grain_moved, self.st.migrants)
    }

    /// Weather only, for checking the climate: runs `years` and returns (year, harvest, warmth, rain) means over land.
    fn weather_years(&mut self, years: i64) -> Vec<(i64, f64, f64, f64)> {
        for _ in 0..years { self.st.year += 1; self.st.weather_year(); }
        self.st.climate.history.clone()
    }

    fn harvest<'py>(&self, py: Python<'py>) -> Bound<'py, PyArray2<f64>> {
        let c = &self.st.climate; arr(py, c.eh, c.ew, &c.harvest.v)
    }

    /// Every person, for the calibration table: (people, female, born, died or -1, cause, children, ruled).
    fn people_rows(&self) -> Vec<(usize, bool, i64, i64, String, usize, bool)> {
        self.st.chars.iter().map(|c| (c.people, c.female, c.born, c.died.unwrap_or(-1), c.cause.clone().unwrap_or_default(),
                                      c.children.len(), c.ruled.is_some())).collect()
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
