fn fnv(events: &[(i64, bool, String)]) -> u64 {
    let mut h: u64 = 0xcbf29ce484222325;
    for (y, m, t) in events {
        for b in format!("{y}|{}|{t}\n", if *m { "True" } else { "False" }).bytes() { h ^= b as u64; h = h.wrapping_mul(0x100000001b3); }
    }
    h
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.get(1).map(|s| s.as_str()) == Some("lives") {
        let (s, i) = world::engine::seed_state(4);
        let mut st = world::state::State::new_world(480, 270, s, i);
        st.run_years(60);
        let y = st.year;
        let mut picks: Vec<usize> = st.living.iter().copied().filter(|&c| st.chars[c].alive && st.chars[c].age(y) <= 3 && st.chars[c].house != usize::MAX).take(3).collect();
        picks.extend(st.living.iter().copied().filter(|&c| st.chars[c].alive && st.chars[c].age(y) >= 20 && st.chars[c].age(y) <= 30).take(2));
        for &c in &picks {
            let mut a = serde_json::Map::new();
            a.insert("c".into(), c.into()); a.insert("on".into(), true.into());
            st.god_act("track", &a).unwrap();
        }
        st.run_years(80);
        for &c in &picks {
            let ch = &st.chars[c];
            println!("\n== {} (origin {}, ambition {}{})", ch.name, ch.origin, ch.ambition, if ch.ambition_met.is_some() { ", fulfilled" } else { "" });
            for (yy, cat, t) in &ch.journal { println!("  {yy} [{cat}] {t}"); }
        }
        let mut o = std::collections::BTreeMap::new(); let mut a = std::collections::BTreeMap::new(); let (mut met, mut n) = (0, 0);
        for &c in &st.living { let ch = &st.chars[c]; if !ch.alive { continue; } *o.entry(ch.origin).or_insert(0) += 1; if !ch.ambition.is_empty() { n += 1; *a.entry(ch.ambition).or_insert(0) += 1; if ch.ambition_met.is_some() { met += 1; } } }
        println!("\norigins {o:?}\nambitions {a:?}\nfulfilled {met} of {n}");
        let minors_bearing = st.living.iter().filter(|&&c| st.chars[c].alive && !st.chars[c].adult(st.year) && st.chars[c].items.iter().any(|&i| st.items[i].rarity >= 3)).count();
        println!("children carrying relics: {minors_bearing}; items held in trust: {}", st.items.iter().filter(|i| i.held_for != usize::MAX).count());
    }
    if args.get(1).map(|s| s.as_str()) == Some("check") {
        let seed: u64 = args[2].parse().unwrap();
        let years: i64 = args[3].parse().unwrap();
        let (s, i) = world::engine::seed_state(seed);
        let mut st = world::state::State::new_world(480, 270, s, i);
        st.run_years(years);
        println!("chars={} events={} fnv={:016x}", st.chars.len(), st.log.len(), fnv(&st.log));
    }
}
