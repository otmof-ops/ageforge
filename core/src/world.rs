//! The world, owned by Rust; Python reads it for the screen and the books.


use numpy::{IntoPyArray, PyArray2};
use pyo3::prelude::*;
use pyo3::types::PyDict;

use crate::people::NONE;
use crate::state::State;
use crate::tables::*;

#[pyclass(module = "realm_of_ages._core")]
pub struct World {
    pub st: State,
}

fn arr<'py, T: numpy::Element + Copy>(py: Python<'py>, h: usize, w: usize, v: &[T]) -> Bound<'py, PyArray2<T>> {
    numpy::ndarray::Array2::from_shape_vec((h, w), v.to_vec()).unwrap().into_pyarray(py)
}

fn opt(x: usize) -> Option<usize> {
    if x == NONE { None } else { Some(x) }
}

#[pymethods]
impl World {
    /// A new world. `state` and `inc` are NumPy's PCG64 state for the seed (Python passes
    /// default_rng(seed).bit_generator.state), so seeds mean the same thing in both engines.
    #[new]
    fn new(width: usize, height: usize, state: u128, inc: u128) -> Self {
        World { st: State::new_world(width, height, state, inc) }
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
            d.set_item("house", opt(k.house))?; d.set_item("extinct", k.extinct)?;
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

    /// Run the world forward. The interpreter lock is released while it runs, so a screen stays live.
    fn step(&mut self, py: Python<'_>, years: i64) {
        py.allow_threads(|| self.st.run_years(years))
    }

    /// Seconds spent in each phase of the year since the world began, slowest first.
    fn phase_times(&self) -> Vec<(&'static str, f64)> {
        let mut v = self.st.phase_times.clone();
        v.sort_by(|a, b| b.1.total_cmp(&a.1));
        v
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

    /// Per people: (era, innovations learned).
    fn cultures(&self) -> Vec<(usize, usize)> { self.st.cultures.iter().map(|c| (c.era, c.innovations.len())).collect() }

    /// Everything the screens and books show, in the Python engine's names (ledger rows from `since` on).
    fn snapshot<'py>(&self, py: Python<'py>, since: i64) -> PyResult<Bound<'py, PyDict>> { self.st.snapshot(py, since) }

    /// One character's full record (the snapshot gives the forgotten dead only as shades).
    fn character_full<'py>(&self, py: Python<'py>, cid: usize) -> PyResult<Bound<'py, PyDict>> {
        if cid >= self.st.chars.len() { return Err(pyo3::exceptions::PyKeyError::new_err(cid)); }
        self.st.character_full(py, cid)
    }

    /// Grids for the map: terrain, the realm owning each cell (-1 none), landmass, mana, corrupted land.
    fn grids<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyDict>> {
        let (h, w) = (self.st.map.h, self.st.map.w);
        let owner_fac: Vec<i64> = self.st.owner.v.iter().map(|&o| if o < 0 { -1 } else { self.st.towns[o as usize].realm as i64 }).collect();
        let d = PyDict::new(py);
        d.set_item("terrain", arr(py, h, w, &self.st.map.terrain.v))?;
        d.set_item("owner_fac", arr(py, h, w, &owner_fac))?;
        d.set_item("owner", arr(py, h, w, &self.st.owner.v))?;
        d.set_item("landmass", arr(py, h, w, &self.st.map.landmass.v))?;
        d.set_item("mana", arr(py, h, w, &self.st.map.mana.v))?;
        d.set_item("corrupted", arr(py, h, w, &self.st.corrupted.v))?;
        d.set_item("res_grid", arr(py, h, w, &self.st.res_grid.v))?;
        Ok(d)
    }

    /// An act of the gods (see godmode.rs); returns what the chronicle says, or a new character's id.
    #[pyo3(signature = (act, args))]
    fn god(&mut self, py: Python<'_>, act: &str, args: &Bound<'_, PyDict>) -> PyResult<String> {
        let text: String = py.import("json")?.call_method1("dumps", (args,))?.extract()?;
        let a: crate::godmode::Args = serde_json::from_str(&text).map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))?;
        self.st.god_act(act, &a).map_err(pyo3::exceptions::PyValueError::new_err)
    }

    /// Each people's hero guild: (hunts, hunt wins, deeds chosen so far by action).
    fn hero_guilds(&self) -> Vec<(u32, u32, Vec<u32>)> { self.st.hero_guilds.iter().map(|g| (g.hunts, g.hunt_wins, g.acts.to_vec())).collect() }

    fn n_characters(&self) -> usize { self.st.chars.len() }
    fn n_houses(&self) -> usize { self.st.houses.len() }
    fn n_ruins(&self) -> usize { self.st.ruins.len() }

    fn house_name(&self, hid: usize) -> String { self.st.houses[hid].name.clone() }

    /// The chronicle from an index on: (entries, the next index).
    fn events_from(&self, index: usize) -> (Vec<(i64, bool, String)>, usize) {
        let n = self.st.log.len();
        (self.st.log[index.min(n)..].to_vec(), n)
    }

    /// The chronicle since a year: (year, minor, text).
    fn events(&self, since: i64) -> Vec<(i64, bool, String)> {
        self.st.log.iter().filter(|e| e.0 >= since).cloned().collect()
    }
}
