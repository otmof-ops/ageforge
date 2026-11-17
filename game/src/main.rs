fn fnv(events: &[(i64, bool, String)]) -> u64 {
    let mut h: u64 = 0xcbf29ce484222325;
    for (y, m, t) in events {
        for b in format!("{y}|{}|{t}\n", if *m { "True" } else { "False" }).bytes() { h ^= b as u64; h = h.wrapping_mul(0x100000001b3); }
    }
    h
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.get(1).map(|s| s.as_str()) == Some("check") {
        let seed: u64 = args[2].parse().unwrap();
        let years: i64 = args[3].parse().unwrap();
        let (s, i) = world::engine::seed_state(seed);
        let mut st = world::state::State::new_world(480, 270, s, i);
        st.run_years(years);
        println!("chars={} events={} fnv={:016x}", st.chars.len(), st.log.len(), fnv(&st.log));
    }
}
