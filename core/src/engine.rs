//! The world itself, with no front end: founding a world from a seed, and running it year by year. The Python
//! extension (`world.rs`) and the game (`game/`) both drive the world through this.

use std::collections::HashSet;
use std::time::Instant;

use crate::geography::make_world;
use crate::grid::Grid;
use crate::people::NONE;
use crate::rng::Pcg64;
use crate::state::State;
use crate::tables::*;

/// Add the time since `t` to a phase's running total.
fn tick(times: &mut Vec<(&'static str, f64)>, name: &'static str, t: Instant) {
    let d = t.elapsed().as_secs_f64();
    match times.iter_mut().find(|x| x.0 == name) { Some(x) => x.1 += d, None => times.push((name, d)) }
}

/// NumPy's SeedSequence and PCG64 seeding, so a seed grows the same world here as `np.random.default_rng(seed)` did:
/// returns the generator's (state, increment) after seeding.
pub fn seed_state(seed: u64) -> (u128, u128) {
    const INIT_A: u32 = 0x43b0d7e5; const MULT_A: u32 = 0x931e8875; const INIT_B: u32 = 0x8b51f9dd; const MULT_B: u32 = 0x58f38ded;
    const MIX_L: u32 = 0xca01f9dd; const MIX_R: u32 = 0x4973f715;
    let entropy: Vec<u32> = if seed >> 32 == 0 { vec![seed as u32] } else { vec![seed as u32, (seed >> 32) as u32] };
    let mut hc = INIT_A;
    let hashmix = |v: u32, hc: &mut u32| -> u32 { let mut v = v ^ *hc; *hc = hc.wrapping_mul(MULT_A); v = v.wrapping_mul(*hc); v ^ (v >> 16) };
    let mix = |x: u32, y: u32| -> u32 { let r = MIX_L.wrapping_mul(x).wrapping_sub(MIX_R.wrapping_mul(y)); r ^ (r >> 16) };
    let mut pool = [0u32; 4];
    for i in 0..4 { pool[i] = hashmix(if i < entropy.len() { entropy[i] } else { 0 }, &mut hc); }
    for s in 0..4 { for d in 0..4 { if s != d { let h = hashmix(pool[s], &mut hc); pool[d] = mix(pool[d], h); } } }
    for &e in entropy.iter().skip(4) { for d in 0..4 { let h = hashmix(e, &mut hc); pool[d] = mix(pool[d], h); } }
    let mut hb = INIT_B;
    let mut words = [0u32; 8];
    for i in 0..8 { let mut v = pool[i % 4] ^ hb; hb = hb.wrapping_mul(MULT_B); v = v.wrapping_mul(hb); words[i] = v ^ (v >> 16); }
    let w64: Vec<u64> = (0..4).map(|i| words[2 * i] as u64 | (words[2 * i + 1] as u64) << 32).collect();
    let initstate = (w64[0] as u128) << 64 | w64[1] as u128;
    let initseq = (w64[2] as u128) << 64 | w64[3] as u128;
    // pcg64_srandom_r
    const MUL: u128 = (2549297995355413924u128 << 64) + 4865540595714422341u128;
    let inc = (initseq << 1) | 1;
    let mut state: u128 = 0;
    state = state.wrapping_mul(MUL).wrapping_add(inc);
    state = state.wrapping_add(initstate);
    state = state.wrapping_mul(MUL).wrapping_add(inc);
    (state, inc)
}

impl State {
    /// A new world. `state` and `inc` are the generator's state (see `seed_state`).
    pub fn new_world(width: usize, height: usize, state: u128, inc: u128) -> State {
        let mut rng = Pcg64::from_state(state, inc);
        let map = make_world(height, width, &mut rng, 0.33);
        let (h, w) = (height, width);
        // as world.Realm: the four first rows take the peoples' table (indexed by realm id, see PORT.md "Python bugs")
        let mut relation = vec![vec![0.0; 4]; 4];
        for i in 0..4 { for j in 0..4 { relation[i][j] = BASE_REL[i][j] + rng.normal(0.0, 8.0); } }
        for i in 0..4 { for j in 0..i { let m = (relation[i][j] + relation[j][i]) / 2.0; relation[i][j] = m; relation[j][i] = m; } }
        let st = State { year: 0, phase_times: vec![], living: vec![], deep_sea: vec![], works: vec![], monument_works: vec![], rng, map, res_grid: Grid::new(h, w, -1), chars: vec![], houses: vec![], towns: vec![],
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
        let mut st = st;
        st.place_resources();
        st.setup_ecology();
        st.place_ruins();
        st.place_capitals(8);
        st.compute_territory();
        st.grow_tables();
        st.found_guilds();
        st.found_faiths();
        st.found_clans();
        st.found_tribes();
        st.climate = crate::climate::Climate::setup(&st.map.terrain, &mut st.rng);
        st
    }

    /// Run the world forward by whole years, in the Python engine's order.
    pub fn run_years(&mut self, years: i64) {
        for _ in 0..years {

            let st = &mut *self;
            st.rebuild_living();
            st.year += 1;
            { let t = Instant::now(); st.town_distance(); tick(&mut st.phase_times, "town_distance", t); }
            { let t = Instant::now(); st.compute_territory(); tick(&mut st.phase_times, "territory", t); }
            let st = &mut *self;
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
            { let t = Instant::now(); st.works_year(); tick(&mut st.phase_times, "works_year", t); }
            { let t = Instant::now(); st.settle_books(); tick(&mut st.phase_times, "settle_books", t); }
        }
    }

    pub fn compute_territory(&mut self) {
        let st = &mut *self;
        let towns: Vec<(usize, usize, f64, bool)> =
            st.towns.iter().filter(|t| t.alive).map(|t| (t.pos.0, t.pos.1, t.pop, t.capital)).collect();
        let ids: Vec<usize> = st.towns.iter().filter(|t| t.alive).map(|t| t.id).collect();
        let terr = &st.map.terrain;
        let own = crate::territory_of(st.map.h, st.map.w, |r, c| terr.at(r, c) == OCEAN, &towns);
        st.owner = Grid { h: st.map.h, w: st.map.w, v: own.into_iter().map(|k| if k < 0 { -1 } else { ids[k as usize] as i64 }).collect() };
    }
}
