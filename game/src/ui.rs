//! The game's screens, laid out as the Textual interface was: the home screen, the new-world dialog, the world with
//! its control buttons and tabs, god mode, the character editor and the confirm dialog.
//!
//! Screens read the world only between years (`try_lock`) and keep what they drew, so switching tabs while years
//! run never waits on the world and never sees it mid-step.

use std::path::PathBuf;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::time::{Duration, Instant};

use crossterm::event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use crossterm::execute;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::{DefaultTerminal, Frame};
use serde_json::{json, Value};
use world::state::State;
use world::tables::*;

use crate::events::CATEGORIES;
use crate::god;
use crate::kit::*;
use crate::session::{list_worlds, Session, PRESETS, SIZES};

const TABS: [&str; 10] = ["Realms", "Chronicle", "Transactions", "Economy", "Guild", "Wars", "People", "Tracked", "Map", "Books"];
const BOOKS: [&str; 15] = ["summary.md", "realms.md", "dynasties.md", "legends.md", "myths.md", "world.md", "magic.md", "seas.md", "pirates.md",
                           "society.md", "works.md", "dungeons.md", "bestiary.md", "calibration.md", "tracked.md"];

fn cat_color(c: &str) -> Color {
    match c {
        "god" => Color::Magenta, "war" => Color::Red, "politics" => Color::Yellow, "economy" => Color::Green, "people" => Color::Cyan,
        "lives" => Color::White, "culture" => Color::LightBlue, "heart" => Color::LightMagenta, "magic" => Color::Rgb(190, 120, 255),
        "monsters" => Color::LightRed, "heroes" => Color::LightYellow, "sea" => Color::Blue, "faith" => Color::LightCyan,
        "crime" => Color::Rgb(180, 180, 180), "nature" => Color::LightGreen, _ => Color::Rgb(128, 128, 128),
    }
}

/// Buttons in a row, each as wide as its label needs, squeezed to fit the row.
fn button_row(r: Rect, labels: &[&str]) -> Vec<Rect> {
    let want: Vec<u16> = labels.iter().map(|l| (l.chars().count() as u16 + 4).max(10)).collect();
    let total: u16 = want.iter().sum::<u16>() + labels.len() as u16;
    let scale = if total > r.width { r.width as f64 / total as f64 } else { 1.0 };
    let mut x = r.x;
    want.iter().map(|&w| { let w = ((w as f64 * scale) as u16).max(4); let rr = Rect { x, y: r.y, width: w, height: 3.min(r.height) }; x += w + 1; rr }).collect()
}

/// Equal parts across a row.
fn parts(r: Rect, n: usize) -> Vec<Rect> {
    let w = (r.width.saturating_sub(n as u16 - 1)) / n as u16;
    (0..n).map(|i| Rect { x: r.x + i as u16 * (w + 1), y: r.y, width: w, height: r.height }).collect()
}

fn notify(t: &mut Vec<Toast>, text: impl Into<String>, error: bool) { t.push(Toast { text: text.into(), error, at: Instant::now() }); }

enum Modal { None, NewWorld, Confirm, God, Edit(usize) }

struct App {
    ui: Ui,
    toasts: Vec<Toast>,
    modal: Modal,
    mui: Ui,
    worlds: Vec<(PathBuf, Value)>,
    opening: Option<std::thread::JoinHandle<Result<Arc<Session>, String>>>,
    s: Option<Arc<Session>>,
    tab: usize,
    acts: u64,
    views: Views,
    run_started: Option<Instant>,
    god_scroll: u16,
    dice: god::Dice,
    quit: bool,
}

#[derive(Default)]
struct Views {
    built: [i64; TABS.len()],
    status: String,
    realms: Vec<Vec<String>>,
    ledger: Vec<Vec<String>>,
    tx: Vec<(i64, f64, String)>,
    econ: Vec<Vec<String>>,
    classes: Vec<Vec<String>>,
    rich: Vec<Vec<String>>,
    parties: Vec<Vec<String>>,
    quests: Vec<Vec<String>>,
    hall: Vec<String>,
    wars: Vec<String>,
    people: Vec<(usize, String)>,
    tracked: Vec<Line<'static>>,
    sheet: (usize, String),
    map: Vec<Line<'static>>,
    map_size: (u16, u16),
    book: (usize, String),
    last: Option<Instant>,
}

pub fn run() -> std::io::Result<()> {
    let mut term = ratatui::init();
    execute!(std::io::stdout(), EnableMouseCapture)?;
    let res = main_loop(&mut term);
    let _ = execute!(std::io::stdout(), DisableMouseCapture);
    ratatui::restore();
    res
}

fn main_loop(term: &mut DefaultTerminal) -> std::io::Result<()> {
    let mut app = App { ui: Ui::default(), toasts: vec![], modal: Modal::None, mui: Ui::default(), worlds: list_worlds(), opening: None, s: None, tab: 0,
                        acts: 0, views: Views { built: [-1; TABS.len()], ..Default::default() }, run_started: None, god_scroll: 0, dice: god::Dice::new(), quit: false };
    app.ui.cursor.insert("worlds".into(), 0);
    while !app.quit {
        if app.opening.as_ref().is_some_and(|j| j.is_finished()) {
            match app.opening.take().unwrap().join() {
                Ok(Ok(s)) => app.enter_world(s),
                Ok(Err(e)) => notify(&mut app.toasts, format!("Cannot open it: {e}"), true),
                Err(_) => notify(&mut app.toasts, "Opening the world failed.", true),
            }
        }
        term.draw(|f| app.draw(f))?;
        if !event::poll(Duration::from_millis(100))? { continue; }
        match event::read()? {
            Event::Key(k) if k.kind == KeyEventKind::Press => app.key(k),
            Event::Mouse(m) => app.mouse(m),
            _ => {}
        }
    }
    Ok(())
}

impl App {
    fn enter_world(&mut self, s: Arc<Session>) {
        self.s = Some(s);
        self.tab = 0;
        self.views = Views { built: [-1; TABS.len()], ..Default::default() };
        self.ui = Ui::default();
        for id in ["realms", "people-list", "books"] { self.ui.cursor.insert(id.into(), 0); }
        for id in ["chron<", "tx<", "econ", "classes", "rich", "parties", "quests", "hall<", "wars", "sheet", "tracked<", "book", "ledger"] { self.ui.scroll.insert(id.into(), 0); }
        self.ui.selects.insert("chron-cat".into(), Select::new("all kinds", std::iter::once(("all kinds".to_string(), json!("all"), String::new())).chain(CATEGORIES.iter().map(|c| (c.to_string(), json!(c), String::new()))).collect()).first());
        self.ui.selects.insert("chron-level".into(), Select::new("major only", vec![("major only".into(), json!("major"), String::new()), ("everything".into(), json!("all"), String::new())]).first());
        self.ui.focus = "fwd100".into();
    }

    fn leave_world(&mut self) {
        if let Some(s) = &self.s { s.save(); }
        self.s = None;
        self.ui = Ui::default();
        self.ui.cursor.insert("worlds".into(), 0);
        self.worlds = list_worlds();
    }

    fn running(&self) -> bool { self.s.as_ref().is_some_and(|s| s.running.load(Ordering::SeqCst)) }

    // ---------------------------------------------------------------- input
    fn key(&mut self, k: KeyEvent) {
        if k.code == KeyCode::Char('q') && k.modifiers.contains(KeyModifiers::CONTROL) {
            if self.running() { notify(&mut self.toasts, "Wait for the years to finish, or press Stop.", false); } else { if self.s.is_some() { self.leave_world(); } self.quit = true; }
            return;
        }
        if !matches!(self.modal, Modal::None) {
            if k.code == KeyCode::Esc && self.mui.open.is_none() { self.modal = Modal::None; return; }
            if let Modal::God = self.modal {
                if self.mui.open.is_none() && matches!(k.code, KeyCode::PageUp | KeyCode::PageDown) {
                    self.god_scroll = if k.code == KeyCode::PageUp { self.god_scroll.saturating_sub(10) } else { self.god_scroll + 10 };
                    return;
                }
            }
            let act = self.mui.key(k);
            self.modal_act(act);
            return;
        }
        if self.s.is_none() {
            let act = self.ui.key(k);
            self.home_act(act);
            return;
        }
        // the world: bindings first, unless typing into a box or choosing from a list
        let typing = self.ui.focus.starts_with("in:") || self.ui.open.is_some();
        if !typing {
            match k.code {
                KeyCode::Char(c @ '1'..='9') => { self.press(&format!("fwd{}", PRESETS[(c as u8 - b'1') as usize])); return; }
                KeyCode::Char('g') => { self.press("god"); return; }
                KeyCode::Left => { self.set_tab((self.tab + TABS.len() - 1) % TABS.len()); return; }
                KeyCode::Right => { self.set_tab((self.tab + 1) % TABS.len()); return; }
                _ => {}
            }
        }
        let act = self.ui.key(k);
        self.world_act(act);
    }

    fn mouse(&mut self, m: crossterm::event::MouseEvent) {
        if !matches!(self.modal, Modal::None) {
            if let Modal::God = self.modal {
                if matches!(m.kind, event::MouseEventKind::ScrollUp | event::MouseEventKind::ScrollDown) && self.mui.open.is_none() {
                    self.god_scroll = if matches!(m.kind, event::MouseEventKind::ScrollUp) { self.god_scroll.saturating_sub(3) } else { self.god_scroll + 3 };
                    return;
                }
            }
            let act = self.mui.mouse(m);
            self.modal_act(act);
            return;
        }
        let act = self.ui.mouse(m);
        if self.s.is_none() { self.home_act(act) } else { self.world_act(act) }
    }

    fn set_tab(&mut self, t: usize) {
        self.tab = t;
        self.ui.focus = match t { 1 => "chron<", 6 => "people-list", 9 => "books", 0 => "realms", 5 => "wars", 7 => "tracked<", _ => "fwd100" }.to_string();
    }

    fn home_act(&mut self, act: Act) {
        if let Act::Pressed(id) = act {
            match id.as_str() {
                "new" => {
                    self.mui = Ui::default();
                    self.mui.set_input("in:name", "");
                    self.mui.set_input("in:seed", "1");
                    self.mui.selects.insert("size".into(), Select::new("size", SIZES.iter().enumerate().map(|(i, s)| (s.0.to_string(), json!(i), String::new())).collect()).with(json!(1)));
                    self.mui.focus = "in:name".into();
                    self.modal = Modal::NewWorld;
                }
                "load" | "worlds" => {
                    if self.opening.is_some() { return; }
                    let i = self.ui.cursor_of("worlds");
                    let Some((dir, meta)) = self.worlds.get(i).cloned() else { notify(&mut self.toasts, "Pick a world first.", false); return };
                    notify(&mut self.toasts, format!("Opening the world: replaying {} years of its history...", meta["year"].as_i64().unwrap_or(0)), false);
                    self.opening = Some(std::thread::spawn(move || Session::load(&dir)));
                }
                "quit" => self.quit = true,
                _ => {}
            }
        }
    }

    fn press(&mut self, id: &str) { self.world_act(Act::Pressed(id.to_string())); }

    fn world_act(&mut self, act: Act) {
        let s = self.s.clone().unwrap();
        match act {
            Act::Pressed(id) => {
                if let Some(t) = id.strip_prefix("tab:") { self.set_tab(t.parse().unwrap_or(0)); return; }
                if let Some(n) = id.strip_prefix("fwd") {
                    let n: i64 = n.parse().unwrap_or(100);
                    if self.running() { notify(&mut self.toasts, "Already running.", false); return; }
                    if s.ended() { notify(&mut self.toasts, "This world has ended.", false); return; }
                    s.advance(n);
                    self.run_started = Some(Instant::now());
                    notify(&mut self.toasts, format!("Moving the world forward {n} years..."), false);
                    return;
                }
                let sel_char = self.views.people.get(self.ui.cursor_of("people-list")).map(|x| x.0);
                match id.as_str() {
                    "stop" => { s.stop.store(true, Ordering::SeqCst); notify(&mut self.toasts, "Stopping at the end of this year...", false); }
                    "god" => { if self.running() { notify(&mut self.toasts, "Wait for the years to finish first.", false); } else { self.open_god(); } }
                    "end" => { if !self.running() { self.mui = Ui::default(); self.mui.focus = "no".into(); self.modal = Modal::Confirm; } }
                    "close" => { if self.running() { notify(&mut self.toasts, "Wait for the years to finish, or press Stop.", false); } else { self.leave_world(); } }
                    "track" | "untrack" => {
                        if self.running() { return; }
                        let Some(c) = sel_char else { notify(&mut self.toasts, "Pick someone in the list first.", false); return };
                        self.god_act("track", serde_json::Map::from_iter([("c".into(), json!(c)), ("on".into(), json!(id == "track"))]));
                    }
                    "edit" => {
                        if self.running() { notify(&mut self.toasts, "Wait for the years to finish first.", false); return; }
                        let Some(c) = sel_char else { notify(&mut self.toasts, "Pick someone in the list first.", false); return };
                        self.open_edit(c);
                    }
                    _ => {}
                }
            }
            Act::Changed(id) => {
                if id == "in:people" { self.ui.cursor.insert("people-list".into(), 0); self.views.built[6] = -1; }
                if id == "people-list" || id == "books" { self.views.built[self.tab] = -1; }
                if id.starts_with("chron") || id == "in:chron" { self.ui.scroll.insert("chron<".into(), 0); }
            }
            Act::Scrolled => { if matches!(self.ui.focus.as_str(), "people-list" | "books") { self.views.built[self.tab] = -1; } }
            Act::None => {}
        }
    }

    fn god_act(&mut self, act: &str, args: serde_json::Map<String, Value>) -> bool {
        let s = self.s.clone().unwrap();
        match s.god(act, args) {
            Ok(t) => { notify(&mut self.toasts, t, false); self.acts += 1; self.views.built = [-1; TABS.len()]; true }
            Err(e) => { notify(&mut self.toasts, e, true); false }
        }
    }

    // ---------------------------------------------------------------- god mode and the editor
    fn open_god(&mut self) {
        let s = self.s.clone().unwrap();
        let Ok(st) = s.world.try_lock() else { return };
        let realms = god::realm_opts(&st);
        let mut u = Ui::default();
        u.selects.insert("ra".into(), Select::new("Realm A", realms.clone()));
        u.selects.insert("rb".into(), Select::new("Realm B", realms.clone()));
        u.selects.insert("land".into(), Select::new("Continent", god::land_opts(&st)));
        u.selects.insert("cul".into(), Select::new("People", god::people_opts()));
        u.selects.insert("gov".into(), Select::new("Government", ["tribal", "clan", "feudal", "republic", "theocracy", "administrative"].iter().map(|g| (g.to_string(), json!(g), String::new())).collect()));
        u.selects.insert("mon".into(), Select::new("Monster", ["troll", "wraith", "demon", "dragon"].iter().map(|g| (g.to_string(), json!(g), String::new())).collect()));
        u.set_input("in:amt", "");
        u.set_input("in:cname", "");
        u.selects.insert("cpeople".into(), Select::new("People", god::people_opts()));
        u.selects.insert("csex".into(), Select::new("Sex", vec![("woman".into(), json!(true), String::new()), ("man".into(), json!(false), String::new())]));
        u.set_input("in:cage", "20");
        u.selects.insert("crealm".into(), Select::new("Realm", realms));
        u.selects.insert("cclass".into(), Select::new("Class", god::class_opts(true)));
        u.set_input("in:clevel", "1");
        u.set_input("in:cprow", "");
        u.set_input("in:ctal", "");
        u.set_input("in:cgold", "0");
        u.set_input("in:cattrs", "");
        u.selects.insert("cpers".into(), Select::checklist("personality (up to four)", god::personality_opts(), &[]));
        u.selects.insert("cgen".into(), Select::checklist("genetic / bloodlines", god::gene_opts(false), &[]));
        u.selects.insert("clat".into(), Select::checklist("hidden (latent) bloodlines", god::gene_opts(true), &[]));
        u.selects.insert("chero".into(), Select::new("hero?", vec![("hero".into(), json!(true), String::new()), ("not a hero".into(), json!(false), String::new())]).first());
        u.selects.insert("corigin".into(), Select::new("Origin", god::origin_opts()));
        u.selects.insert("cambition".into(), Select::new("Ambition", god::ambition_opts()));
        u.selects.insert("cpreset".into(), Select::new("Start from a preset...", god::ARCHETYPES.iter().map(|a| (a.0.to_string(), json!(a.0), format!("{} level {}", a.1, a.2))).collect()));
        u.set_input("in:cstory", "");
        u.focus = "ra".into();
        self.mui = u;
        self.god_scroll = 0;
        self.modal = Modal::God;
    }

    fn open_edit(&mut self, c: usize) {
        let s = self.s.clone().unwrap();
        let Ok(st) = s.world.try_lock() else { return };
        let ch = &st.chars[c];
        let mut u = Ui::default();
        let mut stats: Vec<god::Opt> = ATTRS.iter().enumerate().map(|(i, a)| (format!("{a} (now {:.0})", ch.attrs[i]), json!(a), String::new())).collect();
        for (k, v) in [("prowess", ch.prowess), ("talent", ch.talent), ("level", ch.level as f64), ("renown", ch.renown), ("gold", ch.gold), ("stress", ch.stress), ("happy", ch.happy), ("health", ch.health)] {
            stats.push((format!("{k} (now {v:.0})"), json!(k), String::new()));
        }
        for (i, sk) in SKILLS.iter().enumerate() { stats.push((format!("skill: {sk} (now {:.0})", ch.skills[i]), json!(sk), String::new())); }
        u.selects.insert("stat".into(), Select::new("Stat", stats));
        u.set_input("in:sval", "");
        let mut traits = god::personality_opts();
        traits.extend(god::gene_opts(false));
        let have: Vec<String> = ch.personality.iter().map(|&p| PERSONALITY[p].to_string()).chain(ch.genetic.iter().map(|&g| GENETIC[g].to_string())).collect();
        for t in traits.iter_mut() { if have.iter().any(|h| Value::from(h.as_str()) == t.1) { t.0 = format!("✓ {}", t.0); } }
        u.selects.insert("trait".into(), Select::new("Trait (✓ = has it)", traits));
        u.selects.insert("cls".into(), Select::new("Class", god::class_opts(true)));
        u.set_input("in:relic", "");
        u.selects.insert("rar".into(), Select::new("rarity", RARITY.iter().enumerate().map(|(i, r)| (r.to_string(), json!(i), String::new())).collect()).with(json!(5)));
        u.selects.insert("other".into(), Select::new("another person (type to search)", god::char_opts(&st)));
        u.focus = "stat".into();
        self.mui = u;
        self.modal = Modal::Edit(c);
    }

    fn modal_act(&mut self, act: Act) {
        match self.modal {
            Modal::NewWorld => {
                if let Act::Pressed(id) = act {
                    if id == "cancel" { self.modal = Modal::None; return; }
                    if id == "ok" || id.starts_with("in:") {
                        let name = { let n = self.mui.input("in:name"); if n.trim().is_empty() { "Untitled World".to_string() } else { n.trim().to_string() } };
                        let seed: u64 = self.mui.input("in:seed").trim().parse().unwrap_or(1);
                        let (_, w, h) = SIZES[self.mui.sel("size").and_then(|v| v.as_u64()).unwrap_or(1) as usize];
                        notify(&mut self.toasts, "The world wakes...", false);
                        match Session::new(&name, seed, w, h) {
                            Ok(s) => { self.modal = Modal::None; self.enter_world(s); }
                            Err(e) => notify(&mut self.toasts, e.to_string(), true),
                        }
                    }
                }
            }
            Modal::Confirm => {
                if let Act::Pressed(id) = act {
                    if id == "yes" { let s = self.s.clone().unwrap(); s.end(); self.acts += 1; self.views.built = [-1; TABS.len()]; notify(&mut self.toasts, "The world has ended. Its books are written.", false); }
                    self.modal = Modal::None;
                }
            }
            Modal::God => self.god_pressed(act),
            Modal::Edit(c) => self.edit_pressed(c, act),
            Modal::None => {}
        }
    }

    fn god_pressed(&mut self, act: Act) {
        let id = match act {
            Act::Pressed(id) => id,
            Act::Changed(id) => {
                if id == "cpreset" { self.apply_preset(); }
                if id == "cpers" { tidy_personality(self.mui.selects.get_mut("cpers").unwrap()); }
                return;
            }
            _ => return,
        };
        if id == "close" { self.modal = Modal::None; return; }
        if id.starts_with("in:") { return; }
        let u = &self.mui;
        let num = |s: String, d: f64| s.trim().parse::<f64>().unwrap_or(d);
        let (a, b, land, cul) = (u.sel("ra"), u.sel("rb"), u.sel("land"), u.sel("cul"));
        let amt = num(u.input("in:amt"), 0.0);
        let need_a = ["monster", "plague", "famine", "bless", "gold", "govset", "break", "curse", "smite", "war", "peace"];
        if need_a.contains(&id.as_str()) && a.is_none() { notify(&mut self.toasts, "pick Realm A first", true); return; }
        let m = |pairs: Vec<(&str, Value)>| serde_json::Map::from_iter(pairs.into_iter().map(|(k, v)| (k.to_string(), v)));
        let (act, args) = match id.as_str() {
            "calamity" | "wcalamity" => { let Some(l) = land else { notify(&mut self.toasts, "pick a continent", true); return }; ("wake_calamity", m(vec![("lid", l), ("world", json!(id == "wcalamity"))])) }
            "monster" => ("loose_monster", m(vec![("kind", u.sel("mon").unwrap_or(json!("dragon"))), ("kid", a.unwrap())])),
            "plague" => ("send_plague", m(vec![("kid", a.unwrap()), ("great", json!(true))])),
            "famine" => ("set_harvest", m(vec![("kid", a.unwrap()), ("mult", json!(0.35)), ("years", json!(if amt > 0.0 { amt as i64 } else { 3 }))])),
            "bless" => ("set_harvest", m(vec![("kid", a.unwrap()), ("mult", json!(1.6)), ("years", json!(if amt > 0.0 { amt as i64 } else { 10 }))])),
            "gold" => ("grant_gold", m(vec![("kid", a.unwrap()), ("amount", json!(if amt > 0.0 { amt } else { 1000.0 }))])),
            "war" | "peace" => {
                if b.is_none() || b == a { notify(&mut self.toasts, "pick two different realms", true); return; }
                (if id == "war" { "declare_war" } else { "impose_peace" }, m(vec![("a", a.unwrap()), ("b", b.unwrap())]))
            }
            "govset" => ("change_government", m(vec![("kid", a.unwrap()), ("gov", u.sel("gov").unwrap_or(json!("feudal")))])),
            "era" | "golden" => { let Some(c) = cul else { notify(&mut self.toasts, "pick a people", true); return }; (if id == "era" { "raise_era" } else { "golden_age" }, m(vec![("culture", c)])) }
            "break" => { let Some(l) = land else { notify(&mut self.toasts, "pick a continent", true); return }; ("break_realm", m(vec![("kid", a.unwrap()), ("lid", l)])) }
            "curse" => {
                let s = self.s.clone().unwrap();
                let h = s.world.lock().unwrap().realms[a.clone().unwrap().as_u64().unwrap_or(0) as usize].house;
                if h == world::people::NONE { notify(&mut self.toasts, "that realm has no ruling house", true); return; }
                ("curse_house", m(vec![("hid", json!(h))]))
            }
            "smite" => {
                let s = self.s.clone().unwrap();
                let cap = { let st = s.world.lock().unwrap(); st.capital_of(a.clone().unwrap().as_u64().unwrap_or(0) as usize) };
                if cap == world::people::NONE { notify(&mut self.toasts, "no seat", true); return; }
                ("smite_town", m(vec![("town", json!(cap))]))
            }
            "create" => match self.person_args() { Ok(args) => ("create_character", args), Err(e) => { notify(&mut self.toasts, e, true); return } },
            _ => return,
        };
        if self.god_act(act, args) && act == "create_character" {
            notify(&mut self.toasts, "They walk into the world, tracked.", false);
        }
    }

    fn person_args(&self) -> Result<serde_json::Map<String, Value>, String> {
        let u = &self.mui;
        let (Some(people), Some(realm)) = (u.sel("cpeople"), u.sel("crealm")) else { return Err("pick a people and a realm".into()) };
        let num = |k: &str| u.input(k).trim().parse::<f64>().ok();
        let mut a = serde_json::Map::new();
        a.insert("name".into(), Value::String({ let n = u.input("in:cname"); if n.trim().is_empty() { "Nameless".to_string() } else { n.trim().to_string() } }));
        a.insert("people".into(), people);
        a.insert("female".into(), u.sel("csex").unwrap_or(json!(false)));
        a.insert("age".into(), json!(num("in:cage").unwrap_or(20.0) as i64));
        a.insert("realm".into(), realm);
        if let Some(c) = u.sel("cclass") { a.insert("cls".into(), c); }
        a.insert("level".into(), json!(num("in:clevel").unwrap_or(1.0) as i64));
        if let Some(p) = num("in:cprow") { a.insert("prowess".into(), json!(p)); }
        if let Some(t) = num("in:ctal") { a.insert("talent".into(), json!(t)); }
        a.insert("gold".into(), json!(num("in:cgold").unwrap_or(0.0)));
        let at = u.input("in:cattrs");
        if !at.trim().is_empty() {
            let vals: Vec<f64> = at.split(',').map(|x| x.trim().parse().unwrap_or(5.0)).collect();
            a.insert("attrs".into(), Value::Object(ATTRS.iter().zip(vals).map(|(k, v)| (k.to_string(), json!(v))).collect()));
        }
        let pers = u.checked("cpers");
        if !pers.is_empty() { a.insert("personality".into(), Value::Array(pers)); }
        a.insert("genetic".into(), Value::Array(u.checked("cgen")));
        a.insert("latent".into(), Value::Array(u.checked("clat")));
        a.insert("hero".into(), u.sel("chero").unwrap_or(json!(true)));
        if let Some(o) = u.sel("corigin") { a.insert("origin".into(), o); }
        if let Some(am) = u.sel("cambition") { if !am.is_null() { a.insert("ambition".into(), am); } }
        let story = u.input("in:cstory");
        if !story.trim().is_empty() { a.insert("origin_story".into(), json!(story)); }
        a.insert("track".into(), json!(true));
        Ok(a)
    }

    /// A preset fills in the person's calling, stats, traits, origin and ambition (and a name if none is set).
    fn apply_preset(&mut self) {
        let Some(p) = self.mui.sel("cpreset").and_then(|v| v.as_str().map(String::from)) else { return };
        let Some(&(_, cls, lvl, stats, pers, genes, origin, amb, hero)) = god::ARCHETYPES.iter().find(|a| a.0 == p) else { return };
        let u = &mut self.mui;
        if let Some(s) = u.selects.get_mut("cclass") { *s = s.clone().with(json!(cls)); }
        u.set_input("in:clevel", &lvl.to_string());
        u.set_input("in:cattrs", &stats.iter().map(|x| x.to_string()).collect::<Vec<_>>().join(","));
        if let Some(s) = u.selects.get_mut("cpers") { for i in 0..s.opts.len() { let v = s.opts[i].1.clone(); s.checked[i] = pers.iter().any(|p| Value::from(*p) == v); } }
        if let Some(s) = u.selects.get_mut("cgen") { for i in 0..s.opts.len() { let v = s.opts[i].1.clone(); s.checked[i] = genes.iter().any(|g| Value::from(*g) == v); } }
        if let Some(s) = u.selects.get_mut("corigin") { *s = s.clone().with(json!(origin)); }
        if let Some(s) = u.selects.get_mut("cambition") { *s = s.clone().with(json!(amb)); }
        if let Some(s) = u.selects.get_mut("chero") { *s = s.clone().with(json!(hero)); }
        if u.input("in:cname").trim().is_empty() {
            let people = u.sel("cpeople").and_then(|v| v.as_u64()).unwrap_or(0) as usize;
            let fe = u.sel("csex").and_then(|v| v.as_bool()).unwrap_or(false);
            let n = god::random_name(&mut self.dice, people, fe);
            self.mui.set_input("in:cname", &n);
        }
        notify(&mut self.toasts, format!("{p}: filled in. Change anything before you create them."), false);
    }

    fn edit_pressed(&mut self, c: usize, act: Act) {
        let Act::Pressed(id) = act else { return };
        if id == "close" { self.modal = Modal::None; return; }
        if id.starts_with("in:") { return; }
        let u = &self.mui;
        let m = |pairs: Vec<(&str, Value)>| { let mut x = serde_json::Map::from_iter(pairs.into_iter().map(|(k, v)| (k.to_string(), v))); x.insert("c".into(), json!(c)); x };
        let (act, args) = match id.as_str() {
            "setstat" => {
                let Some(st) = u.sel("stat") else { notify(&mut self.toasts, "pick a stat", true); return };
                let Ok(v) = u.input("in:sval").trim().parse::<f64>() else { notify(&mut self.toasts, "type a value", true); return };
                ("set_stat", m(vec![("key", st), ("value", json!(v))]))
            }
            "addtrait" | "deltrait" => { let Some(t) = u.sel("trait") else { notify(&mut self.toasts, "pick a trait", true); return }; ("set_trait", m(vec![("trait", t), ("on", json!(id == "addtrait"))])) }
            "addclass" => { let Some(cl) = u.sel("cls") else { notify(&mut self.toasts, "pick a class", true); return }; ("set_class", m(vec![("cls", cl)])) }
            "give" => ("give_item", m(vec![("name", Value::String({ let n = u.input("in:relic"); if n.trim().is_empty() { "a nameless relic".to_string() } else { n } })), ("rarity", u.sel("rar").unwrap_or(json!(5)))])),
            "marry" | "love" => {
                let Some(o) = u.sel("other") else { notify(&mut self.toasts, "pick another person", true); return };
                let mut x = serde_json::Map::new(); x.insert("a".into(), json!(c)); x.insert("b".into(), o);
                (if id == "marry" { "marry" } else { "make_love" }, x)
            }
            "track" | "untrack" => ("track", m(vec![("on", json!(id == "track"))])),
            "hero" => ("make_hero", m(vec![])),
            "crown" => ("crown_character", m(vec![])),
            "immortal" => ("make_immortal", m(vec![("on", json!(true))])),
            "concord" => ("name_concord", m(vec![])),
            "custom" => { let on = self.s.as_ref().unwrap().world.lock().map(|st| !st.chars[c].custom).unwrap_or(true); ("make_custom", m(vec![("on", json!(on))])) }
            "kill" => ("strike_down", m(vec![])),
            "raise" => ("raise_dead", m(vec![])),
            _ => return,
        };
        self.god_act(act, args);
        if matches!(id.as_str(), "addtrait" | "deltrait" | "setstat") { let f = self.mui.focus.clone(); self.open_edit(c); self.mui.focus = f; }
    }

    // ---------------------------------------------------------------- drawing
    fn draw(&mut self, f: &mut Frame) {
        let area = f.area();
        fill(f.buffer_mut(), area, BG);
        self.ui.begin();
        if self.s.is_none() { self.draw_home(f, area); } else { self.draw_world(f, area); }
        if matches!(self.modal, Modal::None) { self.ui.dropdown(f); } else { self.draw_modal(f, area); }
        toasts(f, &mut self.toasts);
    }

    fn draw_home(&mut self, f: &mut Frame, area: Rect) {
        header(f, Rect { height: 1, ..area }, "Realm of Ages", "");
        let w = 80.min(area.width);
        let x = area.x + (area.width - w) / 2;
        let col = |y: u16, h: u16| Rect { x, y, width: w, height: h };
        let top = area.y + 1 + (area.height.saturating_sub(31)) / 2;
        let buf = f.buffer_mut();
        center(buf, col(top, 1), top + 1, "REALM OF AGES", Style::default().fg(TEXT).add_modifier(Modifier::BOLD));
        center(buf, col(top, 1), top + 2, "A world that runs a century at a time, for as long as you let it.", Style::default().fg(TEXT));
        self.ui.button(f, Rect { x, y: top + 4, width: 19, height: 3 }, "new", "Start new world", Variant::Success);
        self.ui.label(f, col(top + 8, 1), "Saved worlds:", Style::default().fg(TEXT));
        let items: Vec<String> = self.worlds.iter().map(|(_, m)| {
            let state = if m["ended"].as_bool().unwrap_or(false) { "ended".to_string() } else { format!("year {}", m["year"].as_i64().unwrap_or(0)) };
            let old = if m["engine"].as_str() != Some("rust") { ", old engine: read its books on disk" }
                      else if m["rules"].as_i64().unwrap_or(1) < crate::session::RULES_VERSION { ", older rules: regrows differently" } else { "" };
            format!("{}  ({state}, seed {}{old})", m["name"].as_str().unwrap_or("?"), m["seed"])
        }).collect();
        self.ui.option_list(f, col(top + 9, 12), "worlds", &items);
        self.ui.button(f, Rect { x, y: top + 22, width: 23, height: 3 }, "load", "Load selected world", Variant::Primary);
        self.ui.button(f, Rect { x, y: top + 26, width: 10, height: 3 }, "quit", "Quit", Variant::Error);
        footer(f, Rect { y: area.y + area.height - 1, height: 1, ..area }, &[("^q", "Quit"), ("tab", "next"), ("enter", "press")]);
    }

    fn stamp(&self) -> i64 { self.s.as_ref().unwrap().year_now.load(Ordering::SeqCst) * 1_000_000 + self.acts as i64 }

    /// Rebuild what the open tab shows, when the world moved and it is free (never waits on a year being run).
    fn refresh(&mut self, body: Rect) {
        let s = self.s.clone().unwrap();
        let stamp = self.stamp();
        let map_resized = self.tab == 8 && self.views.map_size != (body.width, body.height);
        if self.views.built[self.tab] == stamp && !map_resized { return; }
        if self.running() && self.views.last.is_some_and(|t| t.elapsed() < Duration::from_millis(900)) { return; }
        let Ok(st) = s.world.try_lock() else { return };
        self.views.last = Some(Instant::now());
        self.views.status = status_line(&st);
        match self.tab {
            0 => self.views.realms = crate::sheet::realm_rows(&st),
            2 => { self.views.ledger = crate::sheet::ledger_rows(&st, st.year - 100); self.views.tx = crate::sheet::tx_lines(&st, 1500); }
            3 => { self.views.econ = crate::sheet::econ_rows(&st); self.views.classes = crate::sheet::class_rows(&st); self.views.rich = crate::sheet::rich_rows(&st, 40); }
            4 => { self.views.parties = crate::sheet::party_rows(&st); self.views.quests = crate::sheet::quest_rows(&st); self.views.hall = crate::sheet::hall_log(&st, 400); }
            5 => self.views.wars = crate::sheet::war_lines(&st),
            6 => {
                self.views.people = people_list(&st, &self.ui.input("in:people"));
                let i = self.ui.cursor_of("people-list").min(self.views.people.len().saturating_sub(1));
                self.ui.cursor.insert("people-list".into(), i);
                if let Some(&(c, _)) = self.views.people.get(i) {
                    if self.views.sheet.0 != c { self.ui.scroll.insert("sheet".into(), 0); }
                    self.views.sheet = (c, crate::sheet::sheet(&st, c));
                }
            }
            7 => self.views.tracked = tracked_lines(&st),
            8 => { self.views.map = map_lines(&st, body.width as usize - 2, body.height as usize); self.views.map_size = (body.width, body.height); }
            9 => {
                let i = self.ui.cursor_of("books").min(BOOKS.len() - 1);
                self.ui.cursor.insert("books".into(), i);
                let t = std::fs::read_to_string(s.dir.join(BOOKS[i])).unwrap_or_else(|_| "_Not written yet: move the world forward first._".into());
                if self.views.book.0 != i { self.ui.scroll.insert("book".into(), 0); }
                self.views.book = (i, t.chars().take(300_000).collect());
            }
            _ => {}
        }
        self.views.built[self.tab] = stamp;
    }

    fn draw_world(&mut self, f: &mut Frame, area: Rect) {
        let s = self.s.clone().unwrap();
        let y0 = area.y;
        let body = Rect { x: area.x, y: y0 + 8, width: area.width, height: area.height.saturating_sub(9) };
        self.refresh(body);
        let msg = s.message.lock().unwrap().take();
        if let Some(msg) = msg { notify(&mut self.toasts, msg, false); self.acts += 1; self.views.built = [-1; TABS.len()]; self.run_started = None; }
        header(f, Rect { y: y0, height: 1, ..area }, "Realm of Ages", &s.name());
        let year = s.year_now.load(Ordering::SeqCst);
        let ended = if s.ended() { " ENDED" } else { "" };
        let status = Rect { y: y0 + 1, height: 1, ..area };
        fill(f.buffer_mut(), status, BOOST);
        let status_text = if self.running() { format!("{}   running... year {year}", s.name()) } else { String::new() };
        f.buffer_mut().set_line(status.x + 1, status.y, &Line::from(if self.running() {
            vec![Span::styled(status_text, Style::default().fg(TEXT).bg(BOOST).add_modifier(Modifier::BOLD))]
        } else { vec![
            Span::styled(format!("{}{ended}", s.name()), Style::default().fg(TEXT).bg(BOOST).add_modifier(Modifier::BOLD)),
            Span::styled(format!("   Year {year}   "), Style::default().fg(TEXT).bg(BOOST).add_modifier(Modifier::BOLD)),
            Span::styled(self.views.status.clone(), Style::default().fg(TEXT).bg(BOOST))] }), area.width.saturating_sub(2));
        // the controls
        let labels: Vec<String> = PRESETS.iter().map(|n| format!("+{n}y")).collect();
        let mut all: Vec<&str> = labels.iter().map(|x| x.as_str()).collect();
        all.extend(["Stop", "God mode", "End world", "Close"]);
        let rects = button_row(Rect { x: area.x, y: y0 + 2, width: area.width, height: 3 }, &all);
        for (i, n) in PRESETS.iter().enumerate() { self.ui.button(f, rects[i], &format!("fwd{n}"), &labels[i], Variant::Success); }
        let n = PRESETS.len();
        self.ui.button(f, rects[n], "stop", "Stop", Variant::Warning);
        self.ui.button(f, rects[n + 1], "god", "God mode", Variant::Primary);
        self.ui.button(f, rects[n + 2], "end", "End world", Variant::Error);
        self.ui.button(f, rects[n + 3], "close", "Close", Variant::Default);
        let running = self.running();
        let (d, t) = (s.done.load(Ordering::SeqCst), s.total.load(Ordering::SeqCst));
        let ratio = if t > 0 { Some(d as f64 / t as f64) } else { None };
        let eta = self.run_started.filter(|_| running && d > 0).map(|st| st.elapsed().mul_f64((t - d) as f64 / d as f64));
        progress(f, Rect { y: y0 + 5, height: 1, ..area }, ratio, eta);
        tabs(f, Rect { y: y0 + 6, height: 2, ..area }, &TABS, self.tab, &mut self.ui);
        self.draw_tab(f, body);
        footer(f, Rect { y: area.y + area.height - 1, height: 1, ..area }, &[("1", "+1y"), ("2", "+5y"), ("3", "+10y"), ("4", "+50y"), ("5", "+100y"), ("6", "+200y"), ("7", "+300y"), ("8", "+500y"), ("9", "+1000y"), ("g", "God mode"), ("^q", "Quit")]);
    }

    fn draw_tab(&mut self, f: &mut Frame, r: Rect) {
        let lbl = |f: &mut Frame, y: u16, t: &str| f.buffer_mut().set_stringn(r.x + 1, y, t, r.width as usize - 2, Style::default().fg(TEXT));
        match self.tab {
            0 => self.ui.table(f, r, "realms", &["id", "Realm", "People", "Rank", "Rule", "Ruler", "Towns", "Population", "Gold", "Army", "At war with"], &self.views.realms, &[4, 32, 8, 10, 14, 30, 6, 11, 9, 8, 40], true),
            1 => {
                let row = Rect { height: 3, ..r };
                let sw = 50.min(row.width.saturating_sub(62));
                self.ui.select(f, Rect { width: 30, ..row }, "chron-cat");
                self.ui.text_input(f, Rect { x: row.x + 31, width: sw, ..row }, "in:chron", "search...");
                self.ui.select(f, Rect { x: row.x + 32 + sw, width: 30, ..row }, "chron-level");
                let s = self.s.clone().unwrap();
                let ev = s.events.lock().unwrap();
                let cat = self.ui.sel("chron-cat").and_then(|v| v.as_str().map(String::from)).unwrap_or_else(|| "all".into());
                let major = self.ui.sel("chron-level").and_then(|v| v.as_str().map(String::from)).unwrap_or_else(|| "major".into()) == "major";
                let q = self.ui.input("in:chron").to_lowercase();
                let picked: Vec<&crate::session::Event> = ev.iter().rev().filter(|e| (cat == "all" || e.cat == cat) && (!major || e.major) && (q.is_empty() || e.text.to_lowercase().contains(&q))).take(1500).collect();
                let mut lines: Vec<Line<'static>> = vec![];
                if picked.len() >= 1500 { lines.push(Line::from(Span::styled("... (older events are in chronicle.md and details.md)", Style::default().fg(MUTED)))); }
                for e in picked.iter().rev() {
                    lines.push(Line::from(vec![Span::styled(format!("{:>5}  ", e.year), Style::default().fg(MUTED)), Span::styled(format!("[{}] ", e.cat), Style::default().fg(cat_color(e.cat))),
                        Span::styled(e.text.replace("**", ""), if e.major { Style::default().fg(TEXT).add_modifier(Modifier::BOLD) } else { Style::default().fg(TEXT) })]));
                }
                drop(ev);
                self.ui.log(f, Rect { y: r.y + 3, height: r.height.saturating_sub(3), ..r }, "chron<", lines, true);
            }
            2 => {
                lbl(f, r.y, "Each crown's treasury over the last 100 years (in, out, by kind), then notable dealings:");
                let th = (self.views.ledger.len() as u16 + 1).min(r.height / 2).max(3);
                self.ui.table(f, Rect { y: r.y + 1, height: th, ..r }, "ledger", &["Realm", "Income", "Spending", "Taxes", "Loot", "Tribute", "Armies", "Court", "Interest", "Treasury now"], &self.views.ledger, &[34, 10, 10, 10, 9, 9, 9, 9, 9, 12], false);
                let lines: Vec<Line<'static>> = self.views.tx.iter().map(|(y, a, t)| Line::from(vec![Span::styled(format!("{y:>5}  "), Style::default().fg(MUTED)),
                    Span::styled(format!("{a:>+10.0}  "), Style::default().fg(if *a > 0.0 { Color::Green } else { Color::Red })), Span::styled(t.clone(), Style::default().fg(TEXT))])).collect();
                self.ui.log(f, Rect { y: r.y + 2 + th, height: r.height.saturating_sub(2 + th), ..r }, "tx<", lines, true);
            }
            3 => {
                let h = r.height.saturating_sub(3) / 3;
                lbl(f, r.y, "Realms: stage, output, gold in circulation, crown debt and credit, work, poverty, trade");
                self.ui.table(f, Rect { y: r.y + 1, height: h, ..r }, "econ", &["Realm", "Stage", "Output", "Gold about", "Crown debt", "Rate", "Credit", "Jobless", "Poverty", "Middle class", "Trade partners", "Top scroll", "Monster stores", "Bound dead"], &self.views.econ, &[28, 9, 9, 10, 10, 6, 7, 8, 8, 12, 14, 14, 14, 10], false);
                lbl(f, r.y + 1 + h, "Classes in each realm: people, income a head, poverty, and what they own (land, houses, stock, coin)");
                self.ui.table(f, Rect { y: r.y + 2 + h, height: h, ..r }, "classes", &["Realm", "Class", "People", "Share", "Income a head", "Poor", "Land", "Houses", "Stock", "Coin", "Owns a head", "Share of wealth"], &self.views.classes, &[28, 7, 9, 6, 14, 6, 9, 9, 9, 9, 12, 15], false);
                lbl(f, r.y + 2 + 2 * h, "The richest people in the world, by everything they own");
                self.ui.table(f, Rect { y: r.y + 3 + 2 * h, height: r.height.saturating_sub(3 + 2 * h), ..r }, "rich", &["Person", "Realm", "Net worth", "Gold in hand", "Work", "#"], &self.views.rich, &[40, 18, 12, 12, 16, 7], false);
            }
            4 => {
                let h = r.height.saturating_sub(3) / 3;
                lbl(f, r.y, "The Wardens' Hall: parties, by seal");
                self.ui.table(f, Rect { y: r.y + 1, height: h, ..r }, "parties", &["Party", "Seal", "Members", "Quests done", "Lost", "Formed", "Who"], &self.views.parties, &[28, 10, 8, 11, 5, 7, 70], false);
                lbl(f, r.y + 1 + h, "The quest board");
                self.ui.table(f, Rect { y: r.y + 2 + h, height: h, ..r }, "quests", &["Seal", "Kind", "Quest", "Reward", "Posted", "Expires"], &self.views.quests, &[10, 12, 64, 8, 7, 8], false);
                lbl(f, r.y + 2 + 2 * h, "Lately done, failed or fled");
                let lines: Vec<Line<'static>> = self.views.hall.iter().map(|l| Line::from(Span::styled(l.clone(), Style::default().fg(if l.contains("failed") { Color::Red } else if l.contains("done") || l.contains("slain") { Color::Green } else { TEXT })))).collect();
                self.ui.log(f, Rect { y: r.y + 3 + 2 * h, height: r.height.saturating_sub(3 + 2 * h), ..r }, "hall<", lines, true);
            }
            5 => {
                let lines: Vec<Line<'static>> = self.views.wars.iter().map(|l| if !l.is_empty() && l.chars().all(|c| !c.is_lowercase()) { Line::from(Span::styled(l.clone(), Style::default().fg(Color::Red).add_modifier(Modifier::BOLD))) } else { Line::from(Span::styled(l.clone(), Style::default().fg(TEXT))) }).collect();
                self.ui.log(f, r, "wars", lines, false);
            }
            6 => {
                let lw = 60.min(r.width / 2);
                let left = Rect { width: lw, ..r };
                self.ui.text_input(f, Rect { height: 3, ..left }, "in:people", "name or id (prefix dead: to include the dead)");
                let items: Vec<String> = self.views.people.iter().map(|x| x.1.clone()).collect();
                self.ui.option_list(f, Rect { y: left.y + 3, height: left.height.saturating_sub(6), ..left }, "people-list", &items);
                let br = button_row(Rect { y: left.y + left.height.saturating_sub(3), height: 3, ..left }, &["Track", "Untrack", "Edit (god)"]);
                self.ui.button(f, br[0], "track", "Track", Variant::Default);
                self.ui.button(f, br[1], "untrack", "Untrack", Variant::Default);
                self.ui.button(f, br[2], "edit", "Edit (god)", Variant::Primary);
                let right = Rect { x: r.x + lw + 1, width: r.width.saturating_sub(lw + 1), ..r };
                let text = if self.views.people.is_empty() { String::new() } else { self.views.sheet.1.clone() };
                self.ui.log(f, right, "sheet", markdown(&text), false);
            }
            7 => {
                let lines = if self.views.tracked.is_empty() { vec![Line::from("No one is tracked. Track people from the People tab, or create one in God mode.")] } else { self.views.tracked.clone() };
                self.ui.log(f, r, "tracked<", lines, true);
            }
            8 => {
                let mut lines = self.views.map.clone();
                if let Some(s) = &self.s { lines.push(Line::from(Span::styled(format!("Full map: {}   ♛ seat  ⚔ host  ▲ dragon  ◆ demon  ✸ calamity", s.dir.join("maps").join("latest.png").display()), Style::default().fg(MUTED)))); }
                for (i, l) in lines.iter().enumerate().take(r.height as usize) { f.buffer_mut().set_line(r.x + 1, r.y + i as u16, l, r.width.saturating_sub(2)); }
            }
            9 => {
                let left = Rect { width: 24, ..r };
                let items: Vec<String> = BOOKS.iter().map(|b| b.to_string()).collect();
                self.ui.option_list(f, left, "books", &items);
                let right = Rect { x: r.x + 25, width: r.width.saturating_sub(25), ..r };
                let text = self.views.book.1.clone();
                self.ui.log(f, right, "book", markdown(&text), false);
            }
            _ => {}
        }
    }

    fn draw_modal(&mut self, f: &mut Frame, area: Rect) {
        self.mui.begin();
        match self.modal {
            Modal::NewWorld => {
                let inner = modal(f, centered(area, 70, 19), ACCENT);
                self.mui.label(f, inner, "A new world", Style::default().fg(TEXT).bg(SURFACE).add_modifier(Modifier::BOLD));
                self.mui.text_input(f, Rect { y: inner.y + 2, height: 3, ..inner }, "in:name", "World name");
                self.mui.text_input(f, Rect { y: inner.y + 5, height: 3, ..inner }, "in:seed", "Seed (a number)");
                self.mui.select(f, Rect { y: inner.y + 8, height: 3, ..inner }, "size");
                let br = button_row(Rect { y: inner.y + 12, height: 3, ..inner }, &["Create", "Cancel"]);
                self.mui.button(f, br[0], "ok", "Create", Variant::Success);
                self.mui.button(f, br[1], "cancel", "Cancel", Variant::Default);
            }
            Modal::Confirm => {
                let inner = modal(f, centered(area, 64, 9), ACCENT);
                self.mui.label(f, inner, "End this world for good? Its books and logs stay on disk.", Style::default().fg(TEXT).bg(SURFACE));
                let br = button_row(Rect { y: inner.y + 3, height: 3, ..inner }, &["Yes", "No"]);
                self.mui.button(f, br[0], "yes", "Yes", Variant::Error);
                self.mui.button(f, br[1], "no", "No", Variant::Default);
            }
            Modal::God => self.draw_god(f, area),
            Modal::Edit(c) => self.draw_edit(f, area, c),
            Modal::None => {}
        }
        self.mui.dropdown(f);
    }

    fn draw_god(&mut self, f: &mut Frame, area: Rect) {
        let outer = Rect { x: area.x + area.width / 40, y: area.y + area.height / 20, width: area.width * 95 / 100, height: area.height * 9 / 10 };
        let inner = modal(f, outer, MAGENTA);
        let inner = Rect { width: inner.width.saturating_sub(2), ..inner };
        let u = &mut self.mui;
        let bottom = (inner.y + inner.height) as i32;
        let label = |f: &mut Frame, r: Rect, t: &str, st: Style| { f.buffer_mut().set_stringn(r.x, r.y, t, r.width as usize, st.bg(SURFACE)); };
        // each row is drawn only when it is wholly in view
        let mut rows: Vec<(u16, Box<dyn FnMut(&mut Frame, Rect, &mut Ui)>)> = vec![];
        rows.push((1, Box::new(move |f, r, _| label(f, r, "GOD MODE  (every act is written into the chronicle)", Style::default().fg(MAGENTA).add_modifier(Modifier::BOLD)))));
        rows.push((1, Box::new(move |f, r, _| label(f, r, "The world", Style::default().fg(TEXT).add_modifier(Modifier::BOLD)))));
        rows.push((3, Box::new(|f, r, u| { let p = parts(r, 3); u.select(f, p[0], "ra"); u.select(f, p[1], "rb"); u.select(f, p[2], "land"); })));
        rows.push((3, Box::new(|f, r, u| { let p = parts(r, 4); u.select(f, p[0], "cul"); u.select(f, p[1], "gov"); u.select(f, p[2], "mon"); u.text_input(f, p[3], "in:amt", "amount / years"); })));
        let button_rows: [Vec<(&'static str, &'static str, Variant)>; 3] = [
            vec![("calamity", "Wake calamity (continent)", Variant::Default), ("wcalamity", "World calamity", Variant::Default), ("monster", "Loose monster on A", Variant::Default), ("plague", "Plague on A", Variant::Default), ("famine", "Famine on A", Variant::Default), ("bless", "Bless harvest of A", Variant::Default)],
            vec![("gold", "Gold to A", Variant::Default), ("war", "A declares war on B", Variant::Default), ("peace", "Peace A and B", Variant::Default), ("govset", "Set A's government", Variant::Default), ("era", "Raise people's era", Variant::Default), ("golden", "Golden age for people", Variant::Default)],
            vec![("break", "Break A on continent", Variant::Error), ("curse", "Curse A's ruling house", Variant::Error), ("smite", "Smite A's seat", Variant::Error)],
        ];
        for bs in button_rows {
            rows.push((3, Box::new(move |f, r, u| { let labels: Vec<&str> = bs.iter().map(|b| b.1).collect(); let rr = button_row(r, &labels); for (i, b) in bs.iter().enumerate() { u.button(f, rr[i], b.0, b.1, b.2); } })));
        }
        rows.push((1, Box::new(move |f, r, _| label(f, r, "Make a person (tracked by default; pick traits and bloodlines from the lists, or start from a preset)", Style::default().fg(TEXT).add_modifier(Modifier::BOLD)))));
        rows.push((3, Box::new(|f, r, u| { let p = parts(r, 5); u.text_input(f, p[0], "in:cname", "Name"); u.select(f, p[1], "cpeople"); u.select(f, p[2], "csex"); u.text_input(f, p[3], "in:cage", "Age"); u.select(f, p[4], "crealm"); })));
        rows.push((3, Box::new(|f, r, u| { let p = parts(r, 5); u.select(f, p[0], "cclass"); u.text_input(f, p[1], "in:clevel", "Level"); u.text_input(f, p[2], "in:cprow", "Prowess"); u.text_input(f, p[3], "in:ctal", "Talent 0-10"); u.text_input(f, p[4], "in:cgold", "Gold"); })));
        rows.push((3, Box::new(|f, r, u| { let p = parts(r, 2); u.text_input(f, p[0], "in:cattrs", "dip,mar,ste,intr,lrn,lore e.g. 12,10,8,6,14,9"); u.select(f, p[1], "cpers"); })));
        rows.push((3, Box::new(|f, r, u| { let p = parts(r, 3); u.select(f, p[0], "cgen"); u.select(f, p[1], "clat"); u.select(f, p[2], "chero"); })));
        rows.push((3, Box::new(|f, r, u| { let p = parts(r, 4); u.select(f, p[0], "corigin"); u.select(f, p[1], "cambition"); u.select(f, p[2], "cpreset"); u.text_input(f, p[3], "in:cstory", "their story (optional)"); })));
        rows.push((1, Box::new(move |f, r, _| label(f, r, "Lists open on click or Enter; type to search them. Checklists: Space ticks, Enter closes. Opposite traits cancel.", Style::default().fg(MUTED)))));
        rows.push((3, Box::new(|f, r, u| { let rr = button_row(r, &["Create person", "Close"]); u.button(f, rr[0], "create", "Create person", Variant::Success); u.button(f, rr[1], "close", "Close", Variant::Default); })));
        // every control, in Tab order, and the row each sits on, so Tab reaches controls scrolled out of view
        const ROW_IDS: [&[&str]; 15] = [&[], &[], &["ra", "rb", "land"], &["cul", "gov", "mon", "in:amt"],
            &["calamity", "wcalamity", "monster", "plague", "famine", "bless"], &["gold", "war", "peace", "govset", "era", "golden"], &["break", "curse", "smite"], &[],
            &["in:cname", "cpeople", "csex", "in:cage", "crealm"], &["cclass", "in:clevel", "in:cprow", "in:ctal", "in:cgold"], &["in:cattrs", "cpers"],
            &["cgen", "clat", "chero"], &["corigin", "cambition", "cpreset", "in:cstory"], &[], &["create", "close"]];
        let heights: Vec<i32> = rows.iter().map(|(h, _)| *h as i32).collect();
        let total: i32 = heights.iter().map(|h| h + 1).sum();
        let max = (total - inner.height as i32).max(0);
        if let Some(ri) = ROW_IDS.iter().position(|ids| ids.contains(&u.focus.as_str())) {
            let top: i32 = heights[..ri].iter().map(|h| h + 1).sum();
            let s = self.god_scroll as i32;
            if top < s { self.god_scroll = top as u16; }
            if top + heights[ri] > s + inner.height as i32 { self.god_scroll = (top + heights[ri] - inner.height as i32).clamp(0, max) as u16; }
        }
        if self.god_scroll as i32 > max { self.god_scroll = max as u16; }
        let mut y = inner.y as i32 - self.god_scroll as i32;
        for (h, mut draw) in rows {
            let h = h as i32;
            if y >= inner.y as i32 && y + h <= bottom { draw(f, Rect { x: inner.x, y: y as u16, width: inner.width, height: h as u16 }, u); }
            y += h + 1;
        }
        u.order = ROW_IDS.iter().flat_map(|ids| ids.iter().map(|x| x.to_string())).collect();
        let max = max as u16;
        if max > 0 {
            let buf = f.buffer_mut();
            let x = outer.x + outer.width - 3;
            for yy in 0..inner.height { buf[(x, inner.y + yy)].set_symbol("│").set_style(Style::default().fg(Color::Rgb(70, 70, 70)).bg(SURFACE)); }
            let pos = inner.y + (self.god_scroll as u32 * (inner.height as u32 - 1) / max as u32) as u16;
            buf[(x, pos)].set_symbol("█").set_style(Style::default().fg(MAGENTA).bg(SURFACE));
        }
    }

    fn draw_edit(&mut self, f: &mut Frame, area: Rect, c: usize) {
        let outer = Rect { x: area.x + area.width / 40, y: area.y + area.height / 20, width: area.width * 95 / 100, height: (area.height * 9 / 10).min(28) };
        let inner = modal(f, outer, MAGENTA);
        let name = self.s.as_ref().and_then(|s| s.world.try_lock().ok().map(|st| crate::sheet::full_name(&st, c))).unwrap_or_default();
        let u = &mut self.mui;
        f.buffer_mut().set_stringn(inner.x, inner.y, format!("GOD MODE: {name} (#{c})"), inner.width as usize, Style::default().fg(MAGENTA).bg(SURFACE).add_modifier(Modifier::BOLD));
        let mut y = inner.y + 2;
        let mut row = |h: u16| { let r = Rect { x: inner.x, y, width: inner.width, height: h }; y += h + 1; r };
        let r = row(3); let p = parts(r, 3);
        u.select(f, p[0], "stat"); u.text_input(f, p[1], "in:sval", "value"); u.button(f, Rect { width: 14.min(p[2].width), ..p[2] }, "setstat", "Set stat", Variant::Success);
        let r = row(3); let p = parts(r, 3);
        u.select(f, p[0], "trait"); u.button(f, Rect { width: 16.min(p[1].width), ..p[1] }, "addtrait", "Give trait", Variant::Success); u.button(f, Rect { width: 16.min(p[2].width), ..p[2] }, "deltrait", "Take trait", Variant::Warning);
        let r = row(3); let p = parts(r, 5);
        u.select(f, p[0], "cls"); u.button(f, p[1], "addclass", "Add class", Variant::Default); u.text_input(f, p[2], "in:relic", "relic name"); u.select(f, p[3], "rar"); u.button(f, p[4], "give", "Give relic", Variant::Default);
        let r = row(3); let p = parts(r, 3);
        u.select(f, p[0], "other"); u.button(f, p[1], "marry", "Marry them", Variant::Default); u.button(f, p[2], "love", "Make them fall in love", Variant::Default);
        let r = row(3);
        let bs = [("track", "Track"), ("untrack", "Untrack"), ("hero", "Make hero"), ("crown", "Crown in their realm"), ("immortal", "Make ageless"), ("concord", "Name the Concord")];
        let rr = button_row(r, &bs.iter().map(|b| b.1).collect::<Vec<_>>());
        for (i, b) in bs.iter().enumerate() { u.button(f, rr[i], b.0, b.1, Variant::Default); }
        let r = row(3);
        let rr = button_row(r, &["Make custom", "Strike down", "Raise from the dead", "Close"]);
        u.button(f, rr[0], "custom", "Make custom", Variant::Primary);
        u.button(f, rr[1], "kill", "Strike down", Variant::Error);
        u.button(f, rr[2], "raise", "Raise from the dead", Variant::Warning);
        u.button(f, rr[3], "close", "Close", Variant::Default);
    }
}

/// Opposites cancel, and four personality traits at most.
fn tidy_personality(s: &mut Select) {
    let mut kept: Vec<usize> = vec![];
    for i in 0..s.opts.len() {
        if !s.checked[i] { continue; }
        let opp = PERSONALITY.iter().position(|p| Value::from(*p) == s.opts[i].1).map(|j| PERSONALITY[j ^ 1]);
        if kept.iter().any(|&k| opp.is_some_and(|o| Value::from(o) == s.opts[k].1)) || kept.len() >= 4 { s.checked[i] = false; continue; }
        kept.push(i);
    }
}

fn status_line(st: &State) -> String {
    let towns = st.towns.iter().filter(|t| t.alive).count();
    let pop: f64 = st.towns.iter().filter(|t| t.alive).map(|t| t.pop).sum();
    let realms = st.realms.iter().filter(|k| !k.extinct).count();
    let living = st.living.iter().filter(|&&c| st.chars[c].alive).count();
    let heroes = st.living.iter().filter(|&&c| st.chars[c].alive && st.chars[c].hero).count();
    let monsters = st.monsters.iter().filter(|m| m.alive).count();
    let cal = st.monsters.iter().filter(|m| m.alive && m.kind == "calamity").count();
    let tracked = st.chars.iter().filter(|c| c.tracked).count();
    format!("Population {pop:.0} in {towns} towns   Realms {realms}   Wars {}   Notables {living}   Heroes {heroes}   Monsters {monsters} (calamities {cal})   Tracked {tracked}", st.wars.len())
}

fn people_list(st: &State, query: &str) -> Vec<(usize, String)> {
    let q = query.to_lowercase();
    let dead = q.starts_with("dead:");
    let q = q.trim_start_matches("dead:").trim().to_string();
    let mut ids: Vec<usize> = if dead { (0..st.chars.len()).collect() } else { st.living.iter().copied().filter(|&c| st.chars[c].alive).collect() };
    if !q.is_empty() { ids.retain(|&c| crate::sheet::full_name(st, c).to_lowercase().contains(&q) || q.parse::<usize>().ok() == Some(c)); }
    let score = |c: usize| { let ch = &st.chars[c]; ch.renown + if ch.tracked { 5000.0 } else { 0.0 } + if ch.ruled.is_some_and(|r| r.2.is_none()) { 3000.0 } else { 0.0 } };
    ids.sort_by(|&a, &b| score(b).partial_cmp(&score(a)).unwrap());
    ids.truncate(60);
    ids.into_iter().map(|c| {
        let ch = &st.chars[c];
        let tag = if ch.tracked { " ★" } else { "" };
        let ruler = if ch.ruled.is_some_and(|r| r.2.is_none()) { " ♛" } else { "" };
        (c, format!("{}{ruler}{tag}  ({}, {}, lv {}, #{c})", crate::sheet::full_name(st, c), PEOPLES[ch.people], ch.age(st.year), ch.level))
    }).collect()
}

fn tracked_lines(st: &State) -> Vec<Line<'static>> {
    let mut out = vec![];
    for c in st.chars.iter().filter(|c| c.tracked) {
        let fate = if c.alive { String::new() } else { format!(" (died {}: {})", c.died.unwrap_or(0), c.cause.clone().unwrap_or_default()) };
        out.push(Line::from(""));
        out.push(Line::from(Span::styled(format!("{}{fate}", crate::sheet::full_name(st, c.id)), Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))));
        for (y, cat, t) in c.journal.iter().rev().take(300).rev() {
            out.push(Line::from(vec![Span::styled(format!("{y:>5}  "), Style::default().fg(MUTED)), Span::styled(format!("[{cat}] "), Style::default().fg(cat_color(cat))), Span::styled(t.replace("**", ""), Style::default().fg(TEXT))]));
        }
    }
    out
}

fn map_lines(st: &State, cols: usize, rows: usize) -> Vec<Line<'static>> {
    let cells = crate::map::terminal(st, cols, rows.saturating_sub(1));
    cells.into_iter().map(|row| Line::from(row.into_iter().map(|c| {
        let bg = Color::Rgb(c.bottom[0], c.bottom[1], c.bottom[2]);
        match c.symbol {
            Some((ch, col)) => Span::styled(ch.to_string(), Style::default().fg(Color::Rgb(col[0], col[1], col[2])).bg(bg).add_modifier(Modifier::BOLD)),
            None => Span::styled("▀", Style::default().fg(Color::Rgb(c.top[0], c.top[1], c.top[2])).bg(bg)),
        }
    }).collect::<Vec<_>>())).collect()
}
