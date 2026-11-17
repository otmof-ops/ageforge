//! Ageforge: the Realm of Ages, played in the terminal.
//!
//!     ageforge                                   the game
//!     ageforge run --years 1000 --seed 7 --dir D  run without the interface (resumes the world in D if there is one)

mod books;
mod events;
mod kit;
mod god;
mod map;
mod session;
mod sheet;
mod ui;

use std::path::PathBuf;
use std::sync::atomic::Ordering;

fn arg(args: &[String], name: &str) -> Option<String> { args.iter().position(|a| a == name).and_then(|i| args.get(i + 1).cloned()) }

fn main() -> std::io::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    match args.get(1).map(|s| s.as_str()) {
        Some("run") => batch(&args),
        Some("--help") | Some("-h") => { println!("ageforge                  play\nageforge run --years N --seed S --dir D [--fresh] [--width W --height H]"); Ok(()) }
        _ => ui::run(),
    }
}

/// Years without the interface: the chronicle, the books, a map each century and a decade log.
fn batch(args: &[String]) -> std::io::Result<()> {
    let years: i64 = arg(args, "--years").and_then(|v| v.parse().ok()).unwrap_or(1000);
    let seed: u64 = arg(args, "--seed").and_then(|v| v.parse().ok()).unwrap_or(1);
    let width: usize = arg(args, "--width").and_then(|v| v.parse().ok()).unwrap_or(480);
    let height: usize = arg(args, "--height").and_then(|v| v.parse().ok()).unwrap_or(270);
    let dir = PathBuf::from(arg(args, "--dir").unwrap_or_else(|| "realm_run".into()));
    let fresh = args.iter().any(|a| a == "--fresh");
    let s = if dir.join("meta.json").exists() && !fresh {
        let s = session::Session::load(&dir).map_err(std::io::Error::other)?;
        println!("resumed at year {}", s.year_now.load(Ordering::SeqCst));
        s
    } else {
        let _ = std::fs::remove_dir_all(&dir);
        let name = dir.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_else(|| "world".into());
        session::Session::create(&dir, &name, seed, width, height)?
    };
    let t0 = std::time::Instant::now();
    s.advance(years);
    let mut last = -1;
    while s.running.load(Ordering::SeqCst) {
        std::thread::sleep(std::time::Duration::from_millis(200));
        let y = s.year_now.load(Ordering::SeqCst);
        if y / 100 != last / 100 && y % 100 < 5 && y != last {
            if let Ok(st) = s.world.try_lock() {
                let towns = st.towns.iter().filter(|t| t.alive).count();
                let pop: f64 = st.towns.iter().filter(|t| t.alive).map(|t| t.pop).sum();
                let realms = st.realms.iter().filter(|k| !k.extinct).count();
                println!("year {:5}  pop={pop:.0} in {towns} towns  realms={realms}  wars={}  ({:.0} yr/s)", st.year, st.wars.len(), st.year as f64 / t0.elapsed().as_secs_f64().max(1e-9));
                last = y;
            }
        }
    }
    println!("stopped at year {}; outputs in {}", s.year_now.load(Ordering::SeqCst), dir.display());
    Ok(())
}
