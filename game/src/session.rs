//! A world in play: its folder, its record, its chronicle, and the thread its years run on.
//!
//! A world is saved as a record, not an image: `meta.json` keeps the seed, every run of years and every act of the
//! gods, in order. The world is deterministic, so loading replays the record and arrives at exactly the same world.
//! While years run on the worker thread it holds the world one year at a time; the screens take it only between
//! years (`try_lock`), and show what they last drew in the meantime, so they never see a world mid-step.

use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicI64, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Instant;

use serde_json::{json, Value};
use world::state::State;

use crate::events::classify;

/// The version of the world's rules. A world saved under older rules replays under the new ones, so it grows into a
/// different world from the one its chronicle describes; the worlds list says so.
pub const RULES_VERSION: i64 = 2;

pub const PRESETS: [i64; 9] = [1, 5, 10, 50, 100, 200, 300, 500, 1000];
pub const SIZES: [(&str, usize, usize); 3] = [("Small (240 x 135)", 240, 135), ("Standard (480 x 270)", 480, 270), ("Huge (640 x 360)", 640, 360)];

#[derive(Clone)]
pub struct Event {
    pub year: i64,
    pub cat: &'static str,
    pub major: bool,
    pub text: String,
}

pub struct Session {
    pub dir: PathBuf,
    pub meta: Mutex<Value>,
    pub world: Mutex<State>,
    pub events: Mutex<Vec<Event>>,
    ev_index: Mutex<usize>,
    pub running: AtomicBool,
    pub stop: AtomicBool,
    pub done: AtomicI64,
    pub total: AtomicI64,
    pub year_now: AtomicI64,
    pub message: Mutex<Option<String>>,
}

/// Where worlds are kept: $AGEFORGE_WORLDS, else the repository's worlds/ (next to target/), else ./worlds.
pub fn worlds_root() -> PathBuf {
    if let Ok(p) = std::env::var("AGEFORGE_WORLDS").or_else(|_| std::env::var("REALM_WORLDS")) { return PathBuf::from(p); }
    if let Ok(exe) = std::env::current_exe() {
        if let Some(repo) = exe.ancestors().find(|a| a.join("Cargo.toml").exists() && a.join("core").exists()) { return repo.join("worlds"); }
    }
    PathBuf::from("worlds")
}

pub fn slug(name: &str) -> String {
    let s: String = name.to_lowercase().chars().map(|c| if c.is_ascii_alphanumeric() { c } else { '-' }).collect();
    let s = s.split('-').filter(|x| !x.is_empty()).collect::<Vec<_>>().join("-");
    if s.is_empty() { "world".into() } else { s }
}

/// Every world on disk: (folder, its meta.json).
pub fn list_worlds() -> Vec<(PathBuf, Value)> {
    let root = worlds_root();
    let _ = fs::create_dir_all(&root);
    let mut out = vec![];
    if let Ok(rd) = fs::read_dir(&root) {
        for e in rd.flatten() {
            let m = e.path().join("meta.json");
            if let Ok(t) = fs::read_to_string(&m) { if let Ok(v) = serde_json::from_str::<Value>(&t) { out.push((e.path(), v)); } }
        }
    }
    out.sort_by(|a, b| a.0.cmp(&b.0));
    out
}

fn found(meta: &Value) -> State {
    let seed = meta["seed"].as_u64().unwrap_or(1);
    let (s, i) = world::engine::seed_state(seed);
    State::new_world(meta["width"].as_u64().unwrap_or(480) as usize, meta["height"].as_u64().unwrap_or(270) as usize, s, i)
}

impl Session {
    fn wrap(dir: PathBuf, meta: Value, st: State) -> Arc<Session> {
        let year = st.year;
        Arc::new(Session { dir, meta: Mutex::new(meta), world: Mutex::new(st), events: Mutex::new(vec![]), ev_index: Mutex::new(0),
                           running: AtomicBool::new(false), stop: AtomicBool::new(false), done: AtomicI64::new(0), total: AtomicI64::new(0),
                           year_now: AtomicI64::new(year), message: Mutex::new(None) })
    }

    /// A new world in the worlds folder, named after `name`.
    pub fn new(name: &str, seed: u64, width: usize, height: usize) -> std::io::Result<Arc<Session>> {
        let root = worlds_root();
        let mut dir = root.join(slug(name));
        let mut n = 2;
        while dir.exists() { dir = root.join(format!("{}-{n}", slug(name))); n += 1; }
        Self::create(&dir, name, seed, width, height)
    }

    /// A new world in a folder of your choosing.
    pub fn create(dir: &Path, name: &str, seed: u64, width: usize, height: usize) -> std::io::Result<Arc<Session>> {
        fs::create_dir_all(dir.join("maps"))?;
        let meta = json!({"name": name, "seed": seed, "width": width, "height": height, "created": now_text(), "year": 0, "ended": false,
                          "advances": [], "ops": [], "engine": "rust", "rules": RULES_VERSION, "tracked": []});
        let st = found(&meta);
        fs::write(dir.join("chronicle.md"), format!("# The Chronicle of {name} (seed {seed})\n\n"))?;
        fs::write(dir.join("details.md"), "")?;
        let s = Self::wrap(dir.to_path_buf(), meta, st);
        { let st = s.world.lock().unwrap(); s.flush(&st); }
        s.save();
        Ok(s)
    }

    /// Open a saved world: found it from its seed and replay its record.
    pub fn load(dir: &Path) -> Result<Arc<Session>, String> {
        let text = fs::read_to_string(dir.join("meta.json")).map_err(|e| e.to_string())?;
        let meta: Value = serde_json::from_str(&text).map_err(|e| e.to_string())?;
        if meta["engine"].as_str() != Some("rust") {
            return Err("it was grown by the old engine, which the game no longer runs; its books and maps are still in its folder".into());
        }
        let mut st = found(&meta);
        for op in meta["ops"].as_array().cloned().unwrap_or_default() {
            match op[0].as_str() {
                Some("step") => st.run_years(op[1].as_i64().unwrap_or(0)),
                Some("god") => { if let (Some(act), Some(args)) = (op[1].as_str(), op[2].as_object()) { let _ = st.god_act(act, args); } }
                _ => {}
            }
        }
        let s = Self::wrap(dir.to_path_buf(), meta, st);
        {
            // the chronicle already on disk stays as it is; only the in-memory log is rebuilt
            let st = s.world.lock().unwrap();
            let mut ev = s.events.lock().unwrap();
            for (y, minor, t) in &st.log { ev.push(Event { year: *y, cat: cat_of(t), major: !minor, text: t.clone() }); }
            *s.ev_index.lock().unwrap() = st.log.len();
        }
        Ok(s)
    }

    pub fn name(&self) -> String { self.meta.lock().unwrap()["name"].as_str().unwrap_or("").to_string() }
    pub fn seed(&self) -> u64 { self.meta.lock().unwrap()["seed"].as_u64().unwrap_or(0) }
    pub fn ended(&self) -> bool { self.meta.lock().unwrap()["ended"].as_bool().unwrap_or(false) }

    pub fn save(&self) {
        let year = self.year_now.load(Ordering::SeqCst);
        let text = {
            let mut m = self.meta.lock().unwrap();
            m["year"] = json!(year);
            serde_json::to_string_pretty(&*m).unwrap_or_default()
        };
        let tmp = self.dir.join("meta.json.tmp");
        if fs::write(&tmp, text).is_ok() { let _ = fs::rename(&tmp, self.dir.join("meta.json")); }
    }

    fn record(&self, op: Value) {
        let mut m = self.meta.lock().unwrap();
        let ops = m["ops"].as_array_mut().expect("ops");
        if op[0] == "step" {
            if let Some(last) = ops.last_mut() {
                if last[0] == "step" { let n = last[1].as_i64().unwrap_or(0) + op[1].as_i64().unwrap_or(0); last[1] = json!(n); return; }
            }
        }
        ops.push(op);
    }

    /// New chronicle entries into the files and the event log.
    fn flush(&self, st: &State) {
        let mut idx = self.ev_index.lock().unwrap();
        if *idx >= st.log.len() { return; }
        let open = |n: &str| OpenOptions::new().create(true).append(true).open(self.dir.join(n));
        let (mut cf, mut df) = (open("chronicle.md").ok(), open("details.md").ok());
        let mut ev = self.events.lock().unwrap();
        for (y, minor, t) in &st.log[*idx..] {
            let line = format!("- **Year {y}.** {t}\n");
            if *minor { if let Some(f) = df.as_mut() { let _ = f.write_all(line.as_bytes()); } }
            else if let Some(f) = cf.as_mut() { let _ = f.write_all(line.as_bytes()); }
            ev.push(Event { year: *y, cat: cat_of(t), major: !minor, text: t.clone() });
        }
        *idx = st.log.len();
    }

    /// Run the world forward on a worker thread; `stop` ends it at the end of the current year.
    pub fn advance(self: &Arc<Self>, years: i64) {
        if self.running.swap(true, Ordering::SeqCst) || self.ended() { self.running.store(false, Ordering::SeqCst); return; }
        self.stop.store(false, Ordering::SeqCst);
        self.done.store(0, Ordering::SeqCst);
        self.total.store(years, Ordering::SeqCst);
        let s = Arc::clone(self);
        thread::spawn(move || {
            let t0 = Instant::now();
            let start = s.year_now.load(Ordering::SeqCst);
            for n in 0..years {
                if s.stop.load(Ordering::SeqCst) { break; }
                let mut st = s.world.lock().unwrap();
                st.run_years(1);
                s.record(json!(["step", 1]));
                s.flush(&st);
                let y = st.year;
                s.year_now.store(y, Ordering::SeqCst);
                if y % 100 == 0 { let _ = crate::map::write_png(&st, &s.dir.join("maps").join(format!("year-{y:05}.png")), s.seed()); }
                drop(st);
                s.done.store(n + 1, Ordering::SeqCst);
            }
            let end = s.year_now.load(Ordering::SeqCst);
            {
                let mut m = s.meta.lock().unwrap();
                if let Some(a) = m["advances"].as_array_mut() { a.push(json!({"from": start, "to": end, "seconds": (t0.elapsed().as_secs_f64() * 10.0).round() / 10.0})); }
            }
            s.save();
            s.write_books();
            *s.message.lock().unwrap() = Some(format!("Year {end}. The books and the map are written ({:.1} s).", t0.elapsed().as_secs_f64()));
            s.running.store(false, Ordering::SeqCst);
        });
    }

    /// The books, the latest map and the tracked lives.
    pub fn write_books(&self) {
        let st = self.world.lock().unwrap();
        let _ = crate::map::write_png(&st, &self.dir.join("maps").join("latest.png"), self.seed());
        let _ = crate::books::write_all(&st, &self.dir, self.seed());
        let _ = self.write_tracked(&st);
    }

    fn write_tracked(&self, st: &State) -> std::io::Result<()> {
        let mut out = String::from("# Tracked characters\n\n");
        for c in st.chars.iter().filter(|c| c.tracked) {
            out += &format!("## {}{}\n\n", crate::sheet::full_name(st, c.id), c.died.map(|d| format!(" (died {d})")).unwrap_or_default());
            for (y, cat, t) in &c.journal { out += &format!("- **{y}** [{cat}] {t}\n"); }
            out += "\n";
        }
        fs::write(self.dir.join("tracked.md"), out)
    }

    /// An act of the gods, done to the world and written into its record.
    pub fn god(&self, act: &str, args: serde_json::Map<String, Value>) -> Result<String, String> {
        if self.running.load(Ordering::SeqCst) { return Err("wait for the years to finish".into()); }
        let mut st = self.world.lock().unwrap();
        let out = st.god_act(act, &args)?;
        self.record(json!(["god", act, Value::Object(args)]));
        self.flush(&st);
        drop(st);
        self.save();
        Ok(out)
    }

    pub fn end(&self) {
        let year = self.year_now.load(Ordering::SeqCst);
        let mut a = serde_json::Map::new();
        a.insert("text".into(), json!(format!("**The end of the world.** The chronicle closes in year {year}.")));
        let _ = self.god("god_note", a);
        { let mut m = self.meta.lock().unwrap(); m["ended"] = json!(true); m["ended_at"] = json!(year); }
        self.save();
        self.write_books();
    }
}

fn cat_of(text: &str) -> &'static str { if text.starts_with("**God.**") { "god" } else { classify(text) } }

fn now_text() -> String {
    let secs = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
    let days = secs / 86400;
    // civil date from days since 1970-01-01 (Howard Hinnant's algorithm)
    let z = days as i64 + 719468;
    let era = z.div_euclid(146097);
    let doe = z - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + if m <= 2 { 1 } else { 0 };
    format!("{y:04}-{m:02}-{d:02} {:02}:{:02}", (secs / 3600) % 24, (secs / 60) % 60)
}
