//! The world, owned by Rust; Python reads it for the screen and the books.

use std::collections::HashSet;

use numpy::{IntoPyArray, PyArray2};
use pyo3::prelude::*;
use std::time::Instant;
use pyo3::types::PyDict;

use crate::geography::make_world;
use crate::grid::Grid;
use crate::people::NONE;
use crate::rng::Pcg64;
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

/// Add the time since `t` to a phase's running total.
fn tick(times: &mut Vec<(&'static str, f64)>, name: &'static str, t: Instant) {
    let d = t.elapsed().as_secs_f64();
    match times.iter_mut().find(|x| x.0 == name) { Some(x) => x.1 += d, None => times.push((name, d)) }
}

impl World {
    /// Run the world forward by whole years, in the Python engine's order.
    fn run_years(&mut self, years: i64) {
        for _ in 0..years {

            let st = &mut self.st;
            st.rebuild_living();
            st.year += 1;
            { let t = Instant::now(); st.town_distance(); tick(&mut st.phase_times, "town_distance", t); }
            { let t = Instant::now(); self.compute_territory(); tick(&mut self.st.phase_times, "territory", t); }
            let st = &mut self.st;
            { let t = Instant::now(); st.realm_resources(); tick(&mut st.phase_times, "realm_resources", t); }
            { let t = Instant::now(); st.weather_year(); tick(&mut st.phase_times, "weather_year", t); }
            { let t = Instant::now(); st.land_year(); tick(&mut st.phase_times, "land_year", t); }
            { let t = Instant::now(); st.expand(); tick(&mut st.phase_times, "expand", t); }
            { let t = Instant::now(); st.refresh_stats(); tick(&mut st.phase_times, "refresh_stats", t); }
            { let t = Instant::now(); st.dynasty_year(); tick(&mut st.phase_times, "dynasty_year", t); }
            { let t = Instant::now(); st.govern_year(); tick(&mut st.phase_times, "govern_year", t); }
            { let t = Instant::now(); st.youth_year(); tick(&mut st.phase_times, "youth_year", t); }
            { let t = Instant::now(); st.titles_year(); tick(&mut st.phase_times, "titles_year", t); }
            { let t = Instant::now(); st.civics_year(); tick(&mut st.phase_times, "civics_year", t); }
            { let t = Instant::now(); st.courts_and_people_year(); tick(&mut st.phase_times, "courts_and_people_year", t); }
            { let t = Instant::now(); st.opinion_year(); tick(&mut st.phase_times, "opinion_year", t); }
            { let t = Instant::now(); st.meters_year(); tick(&mut st.phase_times, "meters_year", t); }
            { let t = Instant::now(); st.hearts_year(); tick(&mut st.phase_times, "hearts_year", t); }
            { let t = Instant::now(); st.statecraft_year(); tick(&mut st.phase_times, "statecraft_year", t); }
            { let t = Instant::now(); st.trade_year(); tick(&mut st.phase_times, "trade_year", t); }
            { let t = Instant::now(); st.money_year(); tick(&mut st.phase_times, "money_year", t); }
            { let t = Instant::now(); st.manage_year(); tick(&mut st.phase_times, "manage_year", t); }
            { let t = Instant::now(); st.arcana_year(); tick(&mut st.phase_times, "arcana_year", t); }
            { let t = Instant::now(); st.moods_year(); tick(&mut st.phase_times, "moods_year", t); }
            { let t = Instant::now(); st.plague_year(); tick(&mut st.phase_times, "plague_year", t); }
            { let t = Instant::now(); st.armies_year(); tick(&mut st.phase_times, "armies_year", t); }
            { let t = Instant::now(); st.spawn_specialists(); tick(&mut st.phase_times, "spawn_specialists", t); }
            { let t = Instant::now(); st.specialists_year(); tick(&mut st.phase_times, "specialists_year", t); }
            { let t = Instant::now(); st.guilds_year(); tick(&mut st.phase_times, "guilds_year", t); }
            { let t = Instant::now(); st.crime_year(); tick(&mut st.phase_times, "crime_year", t); }
            { let t = Instant::now(); st.religion_year(); tick(&mut st.phase_times, "religion_year", t); }
            { let t = Instant::now(); st.shinobi_year(); tick(&mut st.phase_times, "shinobi_year", t); }
            { let t = Instant::now(); st.bending_year(); tick(&mut st.phase_times, "bending_year", t); }
            { let t = Instant::now(); st.sea_year(); tick(&mut st.phase_times, "sea_year", t); }
            { let t = Instant::now(); st.beasts_year(); tick(&mut st.phase_times, "beasts_year", t); }
            { let t = Instant::now(); st.legends_year(); tick(&mut st.phase_times, "legends_year", t); }
            { let t = Instant::now(); st.ecology_year(); tick(&mut st.phase_times, "ecology_year", t); }
            { let t = Instant::now(); st.dungeons_year(); tick(&mut st.phase_times, "dungeons_year", t); }
            { let t = Instant::now(); st.monster_year(); tick(&mut st.phase_times, "monster_year", t); }
            { let t = Instant::now(); st.gates_year(); tick(&mut st.phase_times, "gates_year", t); }
            let t = Instant::now(); let war_power = st.borders_and_diplomacy_with(true); tick(&mut st.phase_times, "borders_and_diplomacy_with", t);
            { let t = Instant::now(); st.events_and_blight(); tick(&mut st.phase_times, "events_and_blight", t); }
            let t = Instant::now(); let hero_power = st.heroes_act(); tick(&mut st.phase_times, "heroes_act", t);
            let t = Instant::now(); let won = st.frontier_year(war_power, hero_power); tick(&mut st.phase_times, "frontier_year", t);
            { let t = Instant::now(); st.spawn_monsters(); tick(&mut st.phase_times, "spawn_monsters", t); }
            { let t = Instant::now(); st.raids(); tick(&mut st.phase_times, "raids", t); }
            { let t = Instant::now(); st.bounties_year(); tick(&mut st.phase_times, "bounties_year", t); }
            { let t = Instant::now(); st.guild_year(); tick(&mut st.phase_times, "guild_year", t); }
            { let t = Instant::now(); st.heroes_resolve(&won); tick(&mut st.phase_times, "heroes_resolve", t); }
            { let t = Instant::now(); st.cleanup(); tick(&mut st.phase_times, "cleanup", t); }
            { let t = Instant::now(); st.spawn_heroes(); tick(&mut st.phase_times, "spawn_heroes", t); }
            { let t = Instant::now(); st.settle_books(); tick(&mut st.phase_times, "settle_books", t); }
        }
    }

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
        let st = State { year: 0, phase_times: vec![], living: vec![], deep_sea: vec![], rng, map, res_grid: Grid::new(h, w, -1), chars: vec![], houses: vec![], towns: vec![],
                         realms: vec![], ruins: vec![], used_names: HashSet::new(), blocked: Grid::new(h, w, false),
                         owner: Grid::new(h, w, -1), relation, war: vec![], truce: vec![], duchies: Default::default(),
                         ktitles: Default::default(), breakaways: vec![], wars: vec![], war_since: vec![], weariness: vec![], armies: vec![],
                         next_army: 0, war_log: vec![], battles_fought: vec![], sieges_won: 0, people_peak: [0; 4], recent_decl: vec![], avenge_goal: (NONE, NONE, NONE), items: vec![], monsters: vec![], guilds_soc: vec![], religions: vec![], eco: crate::ecology::Ecology::empty(), town_dist: Grid::new(h, w, 1e9),
                         sovereign_line: vec![], calamity_log: vec![], last_cal: vec![], gates: None, gate_history: vec![], dungeon_log: vec![], slain: 0, god_harvest: vec![],
                         hero_guilds: (0..4).map(crate::heroes::HeroGuild::new).collect(), acted: vec![], people_tier: [0; 4], world_tier: 3,
                         tier_events: vec![], rites: vec![], hall: Default::default(), myths: vec![], curses: vec![], cycles: vec![], materials: vec![],
                         watch: vec![], corrupted: Grid::new(h, w, false), books: vec![], ledger: Default::default(), tx: vec![], tribes: vec![], concord: NONE, concord_next: 0, concord_waiting_child: false,
                         concord_line: vec![], clans: vec![], clan_rivalry: vec![], clan_war: vec![], clan_truce: vec![], first_clan: NONE, havens: vec![],
                         pirates: vec![], pirate_king: NONE, pirate_kings: vec![], piracy: vec![], sea_terror: vec![], naval_battles: vec![], dragon_riders: vec![], merges: vec![], item_names: Default::default(), greatest: vec![], stats_defaults: vec![], stats_forced: vec![], mined: 0.0,
                         bankruptcies: vec![], risings: vec![], epidemics: vec![], last_pandemic: -999, last_plague: -999, log: vec![],
                         climate: crate::climate::Climate::setup(&Grid::new(1, 1, 0i8), &mut Pcg64::from_state(0, 1)), blight: 0,
                         cultures: (0..4).map(crate::people::Culture::new).collect(),
                         famines: vec![], migrants: 0.0, grain_moved: 0.0 };
        let mut world = World { st };
        world.st.place_resources();
        world.st.setup_ecology();
        world.st.place_ruins();
        world.st.place_capitals(8);
        world.compute_territory();
        world.st.grow_tables();
        world.st.found_guilds();
        world.st.found_faiths();
        world.st.found_clans();
        world.st.found_tribes();
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
        py.allow_threads(|| self.run_years(years))
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
    fn god(&mut self, act: &str, args: &Bound<'_, PyDict>) -> PyResult<String> { self.st.god_act(act, args) }

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
