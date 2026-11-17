//! The game in the terminal: the worlds list, the world's tabs, god mode, the creator and the editor.
//!
//! Screens read the world only between years (`try_lock`) and keep what they drew, so switching tabs while years
//! run never waits on the world and never sees it mid-step.

use std::path::PathBuf;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::time::{Duration, Instant};

use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind};
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Block, Borders, Cell, Clear, Gauge, List, ListItem, ListState, Paragraph, Row, Table, Tabs, Wrap};
use ratatui::{DefaultTerminal, Frame};
use serde_json::{json, Value};
use world::state::State;

use crate::events::CATEGORIES;
use crate::forms::{centered, Field, Form, FormResult, PickResult, Picker, ACCENT, DIM};
use crate::god;
use crate::session::{list_worlds, Session, PRESETS, SIZES};

const TABS: [&str; 10] = ["Realms", "Chronicle", "Money", "Economy", "Guild", "Wars", "People", "Tracked", "Map", "Books"];

enum Overlay {
    Help,
    Confirm(String, &'static str),
    Acts(Picker),
    ActForm(String, Form),
    Creator(Form, god::Dice),
    Presets(Picker),
    Edits(usize, Picker),
    EditForm(usize, String, Form),
    PickChar(Picker),
}

/// What a tab last drew, with the world's year and act count it was drawn from.
#[derive(Default)]
struct Views {
    stamp: (i64, u64),
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
    tracked: Vec<(usize, String)>,
    map: Vec<Line<'static>>,
    map_size: (u16, u16),
    built: [i64; TABS.len()],
}

struct WorldScreen {
    s: Arc<Session>,
    tab: usize,
    views: Views,
    acts: u64,
    overlay: Option<Overlay>,
    // chronicle
    cat: usize,      // 0 = all
    major_only: bool,
    query: String,
    typing: bool,
    scroll: usize,
    // people / tracked / books
    list: ListState,
    include_dead: bool,
    sheet: (usize, String),
    sheet_scroll: u16,
    books: Vec<PathBuf>,
    book_text: String,
    note: String,
    last_view_build: Instant,
    pending_creator: Option<Overlay>,
}

enum Screen { Home(HomeScreen), World(Box<WorldScreen>) }

struct HomeScreen { worlds: Vec<(PathBuf, Value)>, list: ListState, new_world: Option<Form>, note: String, opening: Option<std::thread::JoinHandle<Result<Arc<Session>, String>>> }

impl HomeScreen {
    fn new() -> HomeScreen { let mut list = ListState::default(); list.select(Some(0)); HomeScreen { worlds: list_worlds(), list, new_world: None, note: String::new(), opening: None } }
}

pub fn run() -> std::io::Result<()> {
    let mut term = ratatui::init();
    let res = main_loop(&mut term);
    ratatui::restore();
    res
}

fn main_loop(term: &mut DefaultTerminal) -> std::io::Result<()> {
    let mut screen = Screen::Home(HomeScreen::new());
    loop {
        // a world being opened on its own thread (a replay takes a moment for an old world)
        if let Screen::Home(h) = &mut screen {
            if h.opening.as_ref().is_some_and(|j| j.is_finished()) {
                match h.opening.take().unwrap().join() {
                    Ok(Ok(s)) => { screen = Screen::World(Box::new(WorldScreen::new(s))); continue; }
                    Ok(Err(e)) => h.note = format!("Cannot open it: {e}"),
                    Err(_) => h.note = "Opening the world failed.".into(),
                }
            }
        }
        term.draw(|f| match &mut screen { Screen::Home(h) => draw_home(f, h), Screen::World(w) => w.draw(f) })?;
        if !event::poll(Duration::from_millis(120))? { continue; }
        let Event::Key(k) = event::read()? else { continue };
        if k.kind != KeyEventKind::Press { continue; }
        match &mut screen {
            Screen::Home(h) => {
                if let Some(f) = h.new_world.as_mut() {
                    match f.key(k) {
                        FormResult::Submit => {
                            let name = f.get("name").as_str().unwrap_or("").trim().to_string();
                            let seed = f.get("seed").as_f64().unwrap_or(1.0) as u64;
                            let (_, w, hh) = SIZES[f.get("size").as_u64().unwrap_or(0) as usize];
                            if name.is_empty() { f.note = "Give the world a name first.".into(); continue; }
                            match Session::new(&name, seed, w, hh) {
                                Ok(s) => { screen = Screen::World(Box::new(WorldScreen::new(s))); }
                                Err(e) => { h.note = e.to_string(); h.new_world = None; }
                            }
                        }
                        FormResult::Cancel => h.new_world = None,
                        FormResult::Open => {}
                    }
                    continue;
                }
                if h.opening.is_some() { continue; }
                let n = h.worlds.len();
                let sel = h.list.selected().unwrap_or(0);
                match k.code {
                    KeyCode::Char('q') | KeyCode::Esc => return Ok(()),
                    KeyCode::Char('n') => {
                        let mut d = god::Dice::new();
                        h.new_world = Some(Form::new("A new world", vec![
                            Field::text("name", "Name", "", "What the world is called."),
                            Field::num("seed", "Seed", (d.next() % 100_000) as f64, 0.0, 1e12, 1.0, "The same seed grows the same world."),
                            Field::choice("size", "Size", SIZES.iter().enumerate().map(|(i, s)| (s.0.to_string(), json!(i), String::new())).collect(), "Vast worlds hold several continents and take longer to run."),
                            Field::action("Begin", "F10 or Enter: found the world."),
                        ]));
                    }
                    KeyCode::Up => h.list.select(Some(sel.saturating_sub(1))),
                    KeyCode::Down => h.list.select(Some((sel + 1).min(n.saturating_sub(1)))),
                    KeyCode::Enter if n > 0 => {
                        let (dir, meta) = h.worlds[sel].clone();
                        h.note = format!("Opening {}: replaying {} years of its history...", meta["name"].as_str().unwrap_or("it"), meta["year"].as_i64().unwrap_or(0));
                        h.opening = Some(std::thread::spawn(move || Session::load(&dir)));
                    }
                    _ => {}
                }
            }
            Screen::World(w) => {
                if w.key(k) { w.s.save(); screen = Screen::Home(HomeScreen::new()); }
            }
        }
    }
}

fn draw_home(f: &mut Frame, h: &mut HomeScreen) {
    let area = f.area();
    let [title, body, foot] = Layout::vertical([Constraint::Length(5), Constraint::Min(5), Constraint::Length(3)]).areas(area);
    let t = Text::from(vec![
        Line::from(Span::styled("AGEFORGE", Style::default().fg(ACCENT).add_modifier(Modifier::BOLD))),
        Line::from(Span::styled("Realm of Ages: a world that forges its own history", Style::default().fg(DIM))),
    ]);
    f.render_widget(Paragraph::new(t).alignment(ratatui::layout::Alignment::Center).block(Block::default().borders(Borders::BOTTOM)), title);
    let items: Vec<ListItem> = h.worlds.iter().map(|(_, m)| {
        let state = if m["ended"].as_bool().unwrap_or(false) { "ended".to_string() } else { format!("year {}", m["year"].as_i64().unwrap_or(0)) };
        let old = if m["engine"].as_str() != Some("rust") { "   (old engine: read its books on disk)" }
                  else if m["rules"].as_i64().unwrap_or(1) < crate::session::RULES_VERSION { "   (older rules: it will regrow differently from its chronicle)" } else { "" };
        ListItem::new(format!("{}   {state}, seed {}{old}", m["name"].as_str().unwrap_or("?"), m["seed"]))
    }).collect();
    let empty = items.is_empty();
    let list = List::new(items).block(Block::default().borders(Borders::ALL).title(" Worlds ")).highlight_style(Style::default().bg(Color::Rgb(60, 60, 90)).add_modifier(Modifier::BOLD)).highlight_symbol("▶ ");
    f.render_stateful_widget(list, body, &mut h.list);
    if empty { f.render_widget(Paragraph::new("No worlds yet. Press n to found one.").style(Style::default().fg(DIM)), body.inner(ratatui::layout::Margin { horizontal: 3, vertical: 2 })); }
    let keys = if h.opening.is_some() { h.note.clone() } else { format!("n new world · ↑↓ choose · Enter open · q quit   {}", h.note) };
    f.render_widget(Paragraph::new(keys).style(Style::default().fg(DIM)).block(Block::default().borders(Borders::TOP)), foot);
    if let Some(form) = h.new_world.as_mut() { form.draw(f, area); }
}

fn table<'a>(header: &[&'a str], rows: &'a [Vec<String>], widths: &[u16]) -> Table<'a> {
    let w: Vec<Constraint> = widths.iter().map(|&x| Constraint::Length(x)).collect();
    Table::new(rows.iter().map(|r| Row::new(r.iter().map(|c| Cell::from(c.as_str())))), w)
        .header(Row::new(header.iter().copied()).style(Style::default().fg(ACCENT).add_modifier(Modifier::BOLD)))
        .column_spacing(1)
}

fn cat_color(c: &str) -> Color {
    match c {
        "god" => Color::Magenta, "war" => Color::Red, "politics" => Color::Yellow, "economy" => Color::Green, "people" => Color::Cyan,
        "lives" => Color::White, "culture" => Color::LightBlue, "heart" => Color::LightMagenta, "magic" => Color::Rgb(190, 120, 255),
        "monsters" => Color::LightRed, "heroes" => Color::LightYellow, "sea" => Color::Blue, "faith" => Color::LightCyan, _ => Color::Gray,
    }
}

impl WorldScreen {
    fn new(s: Arc<Session>) -> WorldScreen {
        let mut list = ListState::default();
        list.select(Some(0));
        WorldScreen { s, tab: 0, views: Views { stamp: (-1, 0), built: [-1; TABS.len()], ..Default::default() }, acts: 0, overlay: None, cat: 0, major_only: false,
                      query: String::new(), typing: false, scroll: 0, list, include_dead: false, sheet: (usize::MAX, String::new()), sheet_scroll: 0,
                      books: vec![], book_text: String::new(), note: String::new(), last_view_build: Instant::now() - Duration::from_secs(5), pending_creator: None }
    }

    fn stamp(&self) -> (i64, u64) { (self.s.year_now.load(Ordering::SeqCst), self.acts) }

    /// Rebuild what the open tab shows, if the world moved and it is free (never waits on a year being run).
    fn refresh(&mut self, area: Rect) {
        let running = self.s.running.load(Ordering::SeqCst);
        let stamp = self.stamp();
        let tab_stale = self.views.built[self.tab] != stamp.0 * 1_000_000 + stamp.1 as i64;
        let map_resized = self.tab == 8 && self.views.map_size != (area.width, area.height);
        if !(tab_stale || map_resized || self.views.stamp != stamp) { return; }
        if running && self.last_view_build.elapsed() < Duration::from_millis(900) { return; }
        let Ok(st) = self.s.world.try_lock() else { return };
        self.last_view_build = Instant::now();
        self.views.stamp = stamp;
        self.views.status = status_line(&st, &self.s);
        match self.tab {
            0 => self.views.realms = crate::sheet::realm_rows(&st),
            2 => { self.views.ledger = crate::sheet::ledger_rows(&st, st.year - 100); self.views.tx = crate::sheet::tx_lines(&st, 1500); }
            3 => { self.views.econ = crate::sheet::econ_rows(&st); self.views.classes = crate::sheet::class_rows(&st); self.views.rich = crate::sheet::rich_rows(&st, 40); }
            4 => { self.views.parties = crate::sheet::party_rows(&st); self.views.quests = crate::sheet::quest_rows(&st); self.views.hall = crate::sheet::hall_log(&st, 400); }
            5 => self.views.wars = crate::sheet::war_lines(&st),
            6 => self.views.people = people_list(&st, &self.query, self.include_dead),
            7 => self.views.tracked = st.chars.iter().filter(|c| c.tracked).map(|c| (c.id, god::char_label(&st, c.id))).collect(),
            8 => { self.views.map = map_lines(&st, area.width.saturating_sub(2) as usize, area.height.saturating_sub(4) as usize); self.views.map_size = (area.width, area.height); }
            9 => {
                let mut b: Vec<PathBuf> = std::fs::read_dir(&self.s.dir).map(|rd| rd.flatten().map(|e| e.path()).filter(|p| p.extension().is_some_and(|x| x == "md")).collect()).unwrap_or_default();
                b.sort();
                self.books = b;
            }
            _ => {}
        }
        // the selected character's sheet or life
        if matches!(self.tab, 6 | 7) {
            let ids: Vec<usize> = if self.tab == 6 { self.views.people.iter().map(|x| x.0).collect() } else { self.views.tracked.iter().map(|x| x.0).collect() };
            let sel = self.list.selected().unwrap_or(0).min(ids.len().saturating_sub(1));
            self.list.select(Some(sel));
            if let Some(&c) = ids.get(sel) {
                let text = if self.tab == 6 { crate::sheet::sheet(&st, c) } else {
                    let ch = &st.chars[c];
                    let mut t = format!("{}\n\n", god::char_label(&st, c));
                    for (y, cat, line) in &ch.journal { t += &format!("{y:>5} [{cat}] {line}\n"); }
                    t
                };
                self.sheet = (c, text);
            }
        }
        self.views.built[self.tab] = stamp.0 * 1_000_000 + stamp.1 as i64;
    }

    fn mark_changed(&mut self) { self.acts += 1; }

    fn selected_char(&self) -> Option<usize> {
        match self.tab {
            6 => self.views.people.get(self.list.selected().unwrap_or(0)).map(|x| x.0),
            7 => self.views.tracked.get(self.list.selected().unwrap_or(0)).map(|x| x.0),
            _ => None,
        }
    }

    fn do_act(&mut self, act: &str, args: serde_json::Map<String, Value>) {
        match self.s.god(act, args) {
            Ok(t) => { self.note = t; self.mark_changed(); }
            Err(e) => self.note = format!("The gods cannot: {e}"),
        }
    }

    /// Returns true to leave the world.
    fn key(&mut self, k: KeyEvent) -> bool {
        if let Some(ov) = self.overlay.take() {
            self.overlay = self.overlay_key(ov, k);
            return false;
        }
        if self.typing {
            match k.code {
                KeyCode::Enter | KeyCode::Esc => self.typing = false,
                KeyCode::Backspace => { self.query.pop(); }
                KeyCode::Char(c) => self.query.push(c),
                _ => {}
            }
            self.list.select(Some(0));
            self.scroll = 0;
            self.views.built[self.tab] = -1;
            return false;
        }
        let running = self.s.running.load(Ordering::SeqCst);
        match k.code {
            KeyCode::Char('q') | KeyCode::Esc => { if running { self.note = "Wait for the years to finish, or press s to stop.".into(); } else { return true; } }
            KeyCode::Char(c @ '1'..='9') => {
                if running { self.note = "Already running.".into(); } else if self.s.ended() { self.note = "This world has ended.".into(); }
                else { let n = PRESETS[(c as u8 - b'1') as usize]; self.s.advance(n); self.note = format!("Moving the world forward {n} years..."); }
            }
            KeyCode::Char('s') => { self.s.stop.store(true, Ordering::SeqCst); self.note = "Stopping at the end of this year...".into(); }
            KeyCode::Right | KeyCode::Tab => { self.tab = (self.tab + 1) % TABS.len(); self.on_tab(); }
            KeyCode::Left | KeyCode::BackTab => { self.tab = (self.tab + TABS.len() - 1) % TABS.len(); self.on_tab(); }
            KeyCode::F(n) if (1..=10).contains(&n) => { self.tab = (n - 1) as usize; self.on_tab(); }
            KeyCode::Char('?') | KeyCode::Char('h') => self.overlay = Some(Overlay::Help),
            KeyCode::Char('g') if !running => self.overlay = Some(Overlay::Acts(Picker::new("God mode", god::ACTS.iter().map(|a| (a.label.to_string(), json!(a.key), a.help.to_string())).collect()))),
            KeyCode::Char('c') if !running => self.open_creator(),
            KeyCode::Char('e') if !running => {
                if let Some(c) = self.selected_char() { self.overlay = Some(Overlay::Edits(c, edit_picker())); }
                else if let Ok(st) = self.s.world.try_lock() { self.overlay = Some(Overlay::PickChar(Picker::new("Change whom?", god::char_opts(&st, false)))); }
            }
            KeyCode::Char('t') if !running => {
                if let Some(c) = self.selected_char() {
                    let on = self.s.world.try_lock().map(|st| !st.chars[c].tracked).unwrap_or(true);
                    self.do_act("track", serde_json::Map::from_iter([("c".into(), json!(c)), ("on".into(), json!(on))]));
                }
            }
            KeyCode::Char('x') if !running => self.overlay = Some(Overlay::Confirm("End this world for good? Its books and logs stay on disk. (y/n)".into(), "end")),
            KeyCode::Char('/') if matches!(self.tab, 1 | 6) => { self.typing = true; self.query.clear(); }
            KeyCode::Char('f') if self.tab == 1 => { self.cat = (self.cat + 1) % (CATEGORIES.len() + 1); self.scroll = 0; }
            KeyCode::Char('m') if self.tab == 1 => { self.major_only = !self.major_only; self.scroll = 0; }
            KeyCode::Char('d') if self.tab == 6 => { self.include_dead = !self.include_dead; self.views.built[6] = -1; }
            KeyCode::Up => { if matches!(self.tab, 6 | 7 | 9) { let s = self.list.selected().unwrap_or(0); self.list.select(Some(s.saturating_sub(1))); self.on_select(); } else { self.scroll += 1; } }
            KeyCode::Down => { if matches!(self.tab, 6 | 7 | 9) { let s = self.list.selected().unwrap_or(0); self.list.select(Some(s + 1)); self.on_select(); } else { self.scroll = self.scroll.saturating_sub(1); } }
            KeyCode::PageUp => { if matches!(self.tab, 6 | 7 | 9) { self.sheet_scroll = self.sheet_scroll.saturating_sub(15); } else { self.scroll += 20; } }
            KeyCode::PageDown => { if matches!(self.tab, 6 | 7 | 9) { self.sheet_scroll += 15; } else { self.scroll = self.scroll.saturating_sub(20); } }
            KeyCode::Home => { self.scroll = usize::MAX / 2; self.sheet_scroll = 0; }
            KeyCode::End => { self.scroll = 0; }
            _ => {}
        }
        false
    }

    fn on_tab(&mut self) { self.list.select(Some(0)); self.scroll = 0; self.sheet_scroll = 0; self.typing = false; self.on_select(); }

    fn on_select(&mut self) {
        self.sheet_scroll = 0;
        if self.tab == 9 {
            let i = self.list.selected().unwrap_or(0).min(self.books.len().saturating_sub(1));
            self.list.select(Some(i));
            self.book_text = self.books.get(i).and_then(|p| std::fs::read_to_string(p).ok()).unwrap_or_else(|| "Not written yet: move the world forward first.".into());
        } else {
            let n = if self.tab == 6 { self.views.people.len() } else { self.views.tracked.len() };
            if n > 0 { let i = self.list.selected().unwrap_or(0).min(n - 1); self.list.select(Some(i)); }
            self.views.built[self.tab] = -1;
        }
    }

    fn open_creator(&mut self) {
        let Ok(st) = self.s.world.try_lock() else { self.note = "The world is busy; try again in a moment.".into(); return };
        let f = god::creator_form(&st);
        self.overlay = Some(Overlay::Creator(f, god::Dice::new()));
    }

    fn overlay_key(&mut self, ov: Overlay, k: KeyEvent) -> Option<Overlay> {
        match ov {
            Overlay::Help => None,
            Overlay::Confirm(text, what) => {
                if matches!(k.code, KeyCode::Char('y') | KeyCode::Char('Y')) && what == "end" { self.s.end(); self.mark_changed(); self.note = "The world has ended. Its books are written.".into(); return None; }
                if matches!(k.code, KeyCode::Char('n') | KeyCode::Esc) { return None; }
                Some(Overlay::Confirm(text, what))
            }
            Overlay::Acts(mut p) => match p.key(k) {
                PickResult::Chosen(v) => {
                    let key = v.as_str().unwrap_or("").to_string();
                    if key == "create" { self.open_creator(); return self.overlay.take(); }
                    let Ok(st) = self.s.world.try_lock() else { return None };
                    if key == "edit" { return Some(Overlay::PickChar(Picker::new("Change whom?", god::char_opts(&st, false)))); }
                    god::act_form(&st, &key).map(|f| Overlay::ActForm(key, f))
                }
                PickResult::Cancel | PickResult::Done(_) => None,
                PickResult::Open => Some(Overlay::Acts(p)),
            },
            Overlay::ActForm(key, mut f) => match f.key(k) {
                FormResult::Submit => { let (act, args) = god::act_args(&key, &f); self.do_act(&act, args); None }
                FormResult::Cancel => None,
                FormResult::Open => Some(Overlay::ActForm(key, f)),
            },
            Overlay::PickChar(mut p) => match p.key(k) {
                PickResult::Chosen(v) => Some(Overlay::Edits(v.as_u64().unwrap_or(0) as usize, edit_picker())),
                PickResult::Cancel | PickResult::Done(_) => None,
                PickResult::Open => Some(Overlay::PickChar(p)),
            },
            Overlay::Edits(c, mut p) => match p.key(k) {
                PickResult::Chosen(v) => {
                    let key = v.as_str().unwrap_or("").to_string();
                    let Ok(st) = self.s.world.try_lock() else { return None };
                    god::edit_form(&st, &key, c).map(|f| Overlay::EditForm(c, key, f))
                }
                PickResult::Cancel | PickResult::Done(_) => None,
                PickResult::Open => Some(Overlay::Edits(c, p)),
            },
            Overlay::EditForm(c, key, mut f) => match f.key(k) {
                FormResult::Submit => {
                    let acts = { let st = self.s.world.lock().unwrap(); god::edit_acts(&st, &key, c, &f) };
                    let mut said = vec![];
                    for (act, args) in acts { match self.s.god(&act, args) { Ok(t) => said.push(t), Err(e) => said.push(format!("cannot: {e}")) } }
                    self.mark_changed();
                    self.note = if said.is_empty() { "Nothing changed.".into() } else { said.join(" ") };
                    Some(Overlay::Edits(c, edit_picker()))
                }
                FormResult::Cancel => Some(Overlay::Edits(c, edit_picker())),
                FormResult::Open => { if f.multi_done.take().is_some() { god::tidy_traits(&mut f); } Some(Overlay::EditForm(c, key, f)) }
            },
            Overlay::Presets(mut p) => match p.key(k) {
                PickResult::Chosen(v) => {
                    if let Some(Overlay::Creator(mut f, mut d)) = self.pending_creator.take() { god::apply_archetype(&mut f, v.as_str().unwrap_or(""), &mut d); return Some(Overlay::Creator(f, d)); }
                    None
                }
                PickResult::Cancel | PickResult::Done(_) => self.pending_creator.take(),
                PickResult::Open => Some(Overlay::Presets(p)),
            },
            Overlay::Creator(mut f, mut d) => {
                if f.picker.is_none() && !f.editing {
                    match k.code {
                        KeyCode::Char('p') => {
                            self.pending_creator = Some(Overlay::Creator(f, d));
                            return Some(Overlay::Presets(Picker::new("Start from an archetype", god::ARCHETYPES.iter().map(|a| (a.0.to_string(), json!(a.0), a.1.to_string())).collect())));
                        }
                        KeyCode::Char('r') => { let p = f.get("people").as_u64().unwrap_or(0) as usize; let fe = f.get("female").as_bool().unwrap_or(false); f.set("name", json!(god::random_name(&mut d, p, fe))); return Some(Overlay::Creator(f, d)); }
                        KeyCode::Char('R') => { god::randomize(&mut f, &mut d); return Some(Overlay::Creator(f, d)); }
                        _ => {}
                    }
                }
                match f.key(k) {
                    FormResult::Submit => {
                        god::tidy_traits(&mut f);
                        let args = god::creator_args(&f);
                        match self.s.god("create_character", args) {
                            Ok(id) => {
                                self.mark_changed();
                                let id: usize = id.trim().parse().unwrap_or(0);
                                self.note = "They walk into the world. Their life is in the Tracked tab.".into();
                                self.tab = 7;
                                self.views.built[7] = -1;
                                let _ = id;
                                None
                            }
                            Err(e) => { f.note = format!("The gods cannot: {e}"); Some(Overlay::Creator(f, d)) }
                        }
                    }
                    FormResult::Cancel => None,
                    FormResult::Open => { if f.multi_done.take().is_some() { god::tidy_traits(&mut f); } Some(Overlay::Creator(f, d)) }
                }
            }
        }
    }

    fn draw(&mut self, f: &mut Frame) {
        let area = f.area();
        let [top, tabs_r, body, foot] = Layout::vertical([Constraint::Length(2), Constraint::Length(1), Constraint::Min(5), Constraint::Length(2)]).areas(area);
        self.refresh(body);
        let msg = self.s.message.lock().unwrap().take();
        if let Some(m) = msg { self.note = m; self.mark_changed(); }
        let running = self.s.running.load(Ordering::SeqCst);
        let year = self.s.year_now.load(Ordering::SeqCst);
        let ended = if self.s.ended() { "  ENDED" } else { "" };
        let head = Line::from(vec![Span::styled(format!(" {}{ended} ", self.s.name()), Style::default().fg(ACCENT).add_modifier(Modifier::BOLD)),
                                   Span::styled(format!(" Year {year}  "), Style::default().add_modifier(Modifier::BOLD)), Span::raw(self.views.status.clone())]);
        if running {
            let [a, b] = Layout::vertical([Constraint::Length(1), Constraint::Length(1)]).areas(top);
            f.render_widget(Paragraph::new(head), a);
            let (d, t) = (self.s.done.load(Ordering::SeqCst), self.s.total.load(Ordering::SeqCst).max(1));
            f.render_widget(Gauge::default().ratio((d as f64 / t as f64).min(1.0)).label(format!("running: {d} of {t} years · s stops")).gauge_style(Style::default().fg(ACCENT)), b);
        } else {
            f.render_widget(Paragraph::new(head), top);
        }
        let titles: Vec<Line> = TABS.iter().enumerate().map(|(i, t)| Line::from(format!("F{} {t}", i + 1))).collect();
        f.render_widget(Tabs::new(titles).select(self.tab).highlight_style(Style::default().fg(Color::Black).bg(ACCENT).add_modifier(Modifier::BOLD)).divider("│"), tabs_r);
        match self.tab {
            0 => f.render_widget(table(&["id", "Realm", "People", "Rank", "Rule", "Ruler", "Towns", "Population", "Gold", "Army", "At war with"], &self.views.realms, &[4, 30, 7, 9, 13, 30, 6, 11, 9, 8, 30]).block(Block::default().borders(Borders::ALL)), body),
            1 => self.draw_chronicle(f, body),
            2 => {
                let [a, b] = Layout::vertical([Constraint::Percentage(45), Constraint::Percentage(55)]).areas(body);
                f.render_widget(table(&["Realm", "Income", "Spending", "Taxes", "Loot", "Tribute", "Armies", "Court", "Interest", "Treasury"], &self.views.ledger, &[30, 10, 10, 10, 9, 9, 9, 9, 9, 10]).block(Block::default().borders(Borders::ALL).title(" The last hundred years ")), a);
                let lines: Vec<Line> = self.views.tx.iter().rev().skip(self.scroll).take(b.height as usize).map(|(y, amt, t)| Line::from(vec![
                    Span::styled(format!("{y:>5} "), Style::default().fg(DIM)), Span::styled(format!("{amt:>+11.0}  "), Style::default().fg(if *amt > 0.0 { Color::Green } else { Color::Red })), Span::raw(t.clone())])).collect();
                f.render_widget(Paragraph::new(lines).block(Block::default().borders(Borders::ALL).title(" Transactions, newest first (↑↓ scroll) ")), b);
            }
            3 => {
                let [a, b, c] = Layout::vertical([Constraint::Percentage(35), Constraint::Percentage(35), Constraint::Percentage(30)]).areas(body);
                f.render_widget(table(&["Realm", "Stage", "Output", "Gold about", "Debt", "Rate", "Credit", "Jobless", "Poverty", "Middle", "Partners", "Top scroll", "Stores", "Bound dead"], &self.views.econ, &[26, 9, 9, 10, 9, 6, 7, 7, 7, 7, 8, 14, 7, 9]).block(Block::default().borders(Borders::ALL).title(" Economies ")), a);
                f.render_widget(table(&["Realm", "Class", "People", "Share", "Income", "Poor", "Land", "Houses", "Stock", "Coin", "Owns", "Wealth"], &self.views.classes, &[26, 7, 9, 6, 8, 6, 8, 8, 8, 8, 8, 7]).block(Block::default().borders(Borders::ALL).title(" Classes and what they own ")), b);
                f.render_widget(table(&["Person", "Realm", "Net worth", "Gold", "Work", "#"], &self.views.rich, &[34, 16, 12, 10, 14, 7]).block(Block::default().borders(Borders::ALL).title(" The richest ")), c);
            }
            4 => {
                let [a, b, c] = Layout::vertical([Constraint::Percentage(30), Constraint::Percentage(35), Constraint::Percentage(35)]).areas(body);
                f.render_widget(table(&["Party", "Seal", "Members", "Done", "Lost", "Formed", "Who"], &self.views.parties, &[26, 10, 8, 6, 5, 7, 60]).block(Block::default().borders(Borders::ALL).title(" Parties of the Wardens' Hall ")), a);
                f.render_widget(table(&["Seal", "Kind", "Quest", "Reward", "Posted", "Expires"], &self.views.quests, &[10, 12, 60, 8, 7, 8]).block(Block::default().borders(Borders::ALL).title(" Quests posted ")), b);
                let lines: Vec<Line> = self.views.hall.iter().rev().take(c.height as usize).map(|l| Line::from(l.clone())).collect();
                f.render_widget(Paragraph::new(lines).block(Block::default().borders(Borders::ALL).title(" The Hall's log, newest first ")), c);
            }
            5 => {
                let lines: Vec<Line> = self.views.wars.iter().skip(self.scroll.min(self.views.wars.len())).map(|l| if l.chars().all(|c| !c.is_lowercase()) && !l.is_empty() { Line::from(Span::styled(l.clone(), Style::default().fg(Color::Red).add_modifier(Modifier::BOLD))) } else { Line::from(l.clone()) }).collect();
                f.render_widget(Paragraph::new(lines).wrap(Wrap { trim: false }).block(Block::default().borders(Borders::ALL).title(" Wars (↑↓ scroll) ")), body);
            }
            6 | 7 => self.draw_people(f, body),
            8 => f.render_widget(Paragraph::new(self.views.map.clone()).block(Block::default().borders(Borders::ALL).title(" The world   ♛ seat · ⚔ host · ▲ dragon · ◆ demon · ✸ calamity   (full map: maps/latest.png) ")), body),
            9 => self.draw_books(f, body),
            _ => {}
        }
        let keys = match self.tab {
            1 => "1-9 run 1·5·10·50·100·200·300·500·1000 yrs · / search · f kind · m major only · ↑↓ PgUp/PgDn scroll · g god · c create · ? help · q back",
            6 => "1-9 run years · / search · d dead too · ↑↓ choose · PgUp/PgDn sheet · e change · t track · c create · g god · ? help · q back",
            7 => "1-9 run years · ↑↓ choose · PgUp/PgDn life · e change · t untrack · c create · g god · ? help · q back",
            _ => "1-9 run 1·5·10·50·100·200·300·500·1000 yrs · ←→/F1-F10 tabs · g god mode · c create character · x end world · ? help · q back",
        };
        f.render_widget(Paragraph::new(vec![Line::from(Span::styled(self.note.clone(), Style::default().fg(Color::Rgb(200, 220, 255)))), Line::from(Span::styled(keys, Style::default().fg(DIM)))]), foot);
        match self.overlay.as_mut() {
            Some(Overlay::Help) => draw_help(f, area),
            Some(Overlay::Confirm(t, _)) => { let r = centered(area, 50, 20); f.render_widget(Clear, r); f.render_widget(Paragraph::new(t.as_str()).wrap(Wrap { trim: true }).block(Block::default().borders(Borders::ALL).title(" Are you sure? ")), r); }
            Some(Overlay::Acts(p)) | Some(Overlay::Presets(p)) | Some(Overlay::PickChar(p)) | Some(Overlay::Edits(_, p)) => p.draw(f, area),
            Some(Overlay::ActForm(_, form)) | Some(Overlay::EditForm(_, _, form)) | Some(Overlay::Creator(form, _)) => form.draw(f, area),
            None => {}
        }
    }

    fn draw_chronicle(&mut self, f: &mut Frame, area: Rect) {
        let ev = self.s.events.lock().unwrap();
        let cat = if self.cat == 0 { None } else { Some(CATEGORIES[self.cat - 1]) };
        let q = self.query.to_lowercase();
        let h = area.height.saturating_sub(2) as usize;
        let picked: Vec<&crate::session::Event> = ev.iter().rev().filter(|e| cat.is_none_or(|c| e.cat == c) && (!self.major_only || e.major) && (q.is_empty() || e.text.to_lowercase().contains(&q)))
            .skip(self.scroll.min(ev.len())).take(h).collect();
        let lines: Vec<Line> = picked.iter().rev().map(|e| Line::from(vec![
            Span::styled(format!("{:>5} ", e.year), Style::default().fg(DIM)),
            Span::styled(format!("[{}] ", e.cat), Style::default().fg(cat_color(e.cat))),
            Span::styled(e.text.replace("**", ""), if e.major { Style::default().add_modifier(Modifier::BOLD) } else { Style::default() }),
        ])).collect();
        let title = format!(" Chronicle · kind: {} · {} · search: {}{} ", cat.unwrap_or("all"), if self.major_only { "major only" } else { "all events" },
                            if self.query.is_empty() { "-".into() } else { format!("\"{}\"", self.query) }, if self.typing { "▏" } else { "" });
        f.render_widget(Paragraph::new(lines).wrap(Wrap { trim: false }).block(Block::default().borders(Borders::ALL).title(title)), area);
    }

    fn draw_people(&mut self, f: &mut Frame, area: Rect) {
        let [l, r] = Layout::horizontal([Constraint::Percentage(38), Constraint::Percentage(62)]).areas(area);
        let rows = if self.tab == 6 { &self.views.people } else { &self.views.tracked };
        let items: Vec<ListItem> = rows.iter().map(|x| ListItem::new(x.1.clone())).collect();
        let title = if self.tab == 6 {
            format!(" People{} · search: {}{} ", if self.include_dead { " (and the dead)" } else { "" }, if self.query.is_empty() { "-".into() } else { format!("\"{}\"", self.query) }, if self.typing { "▏" } else { "" })
        } else { " Tracked (t on People to add, c to create) ".to_string() };
        f.render_stateful_widget(List::new(items).block(Block::default().borders(Borders::ALL).title(title)).highlight_style(Style::default().bg(Color::Rgb(60, 60, 90)).add_modifier(Modifier::BOLD)).highlight_symbol("▶ "), l, &mut self.list);
        let text = if rows.is_empty() { if self.tab == 7 { "No one is tracked yet. Pick someone on the People tab and press t, or press c to create a character.".to_string() } else { String::new() } } else { self.sheet.1.clone() };
        f.render_widget(Paragraph::new(md_lines(&text)).wrap(Wrap { trim: false }).scroll((self.sheet_scroll, 0)).block(Block::default().borders(Borders::ALL).title(if self.tab == 6 { " Sheet (PgUp/PgDn) " } else { " Their life (PgUp/PgDn) " })), r);
    }

    fn draw_books(&mut self, f: &mut Frame, area: Rect) {
        let [l, r] = Layout::horizontal([Constraint::Length(22), Constraint::Min(20)]).areas(area);
        if self.book_text.is_empty() && !self.books.is_empty() { self.on_select(); }
        let items: Vec<ListItem> = self.books.iter().map(|p| ListItem::new(p.file_name().map(|x| x.to_string_lossy().to_string()).unwrap_or_default())).collect();
        f.render_stateful_widget(List::new(items).block(Block::default().borders(Borders::ALL).title(" Books ")).highlight_style(Style::default().bg(Color::Rgb(60, 60, 90))).highlight_symbol("▶ "), l, &mut self.list);
        let text: String = self.book_text.chars().take(400_000).collect();
        f.render_widget(Paragraph::new(md_lines(&text)).wrap(Wrap { trim: false }).scroll((self.sheet_scroll, 0)).block(Block::default().borders(Borders::ALL).title(" PgUp/PgDn to read ")), r);
    }
}

fn edit_picker() -> Picker { Picker::new("Change them", god::EDITS.iter().map(|e| (e.1.to_string(), json!(e.0), e.2.to_string())).collect()) }

fn status_line(st: &State, s: &Session) -> String {
    let towns: Vec<&world::people::Town> = st.towns.iter().filter(|t| t.alive).collect();
    let pop: f64 = towns.iter().map(|t| t.pop).sum();
    let realms = st.realms.iter().filter(|k| !k.extinct).count();
    let wars = st.wars.len();
    let living = st.living.iter().filter(|&&c| st.chars[c].alive).count();
    let heroes = st.living.iter().filter(|&&c| st.chars[c].alive && st.chars[c].hero).count();
    let monsters = st.monsters.iter().filter(|m| m.alive).count();
    let tracked = st.chars.iter().filter(|c| c.tracked).count();
    let _ = s;
    format!("· {pop:.0} people in {} towns · {realms} realms · {wars} wars · {living} notables · {heroes} heroes · {monsters} monsters · {tracked} tracked", towns.len())
}

fn people_list(st: &State, query: &str, dead: bool) -> Vec<(usize, String)> {
    let q = query.to_lowercase();
    let mut ids: Vec<usize> = if dead { (0..st.chars.len()).collect() } else { st.living.iter().copied().filter(|&c| st.chars[c].alive).collect() };
    if !q.is_empty() {
        ids.retain(|&c| crate::sheet::full_name(st, c).to_lowercase().contains(&q) || q.parse::<usize>().ok() == Some(c));
    }
    let score = |c: usize| { let ch = &st.chars[c]; ch.renown + if ch.tracked { 5000.0 } else { 0.0 } + if ch.ruled.is_some_and(|r| r.2.is_none()) { 3000.0 } else { 0.0 } };
    ids.sort_by(|&a, &b| score(b).partial_cmp(&score(a)).unwrap());
    ids.truncate(400);
    ids.into_iter().map(|c| (c, god::char_label(st, c))).collect()
}

fn map_lines(st: &State, cols: usize, rows: usize) -> Vec<Line<'static>> {
    let cells = crate::map::terminal(st, cols, rows);
    cells.into_iter().map(|row| Line::from(row.into_iter().map(|c| {
        let bg = Color::Rgb(c.bottom[0], c.bottom[1], c.bottom[2]);
        match c.symbol {
            Some((ch, col)) => Span::styled(ch.to_string(), Style::default().fg(Color::Rgb(col[0], col[1], col[2])).bg(bg).add_modifier(Modifier::BOLD)),
            None => Span::styled("▀", Style::default().fg(Color::Rgb(c.top[0], c.top[1], c.top[2])).bg(bg)),
        }
    }).collect::<Vec<_>>())).collect()
}

/// Markdown made readable in a terminal: headings bold, emphasis stripped.
fn md_lines(text: &str) -> Vec<Line<'static>> {
    text.lines().map(|l| {
        if let Some(h) = l.strip_prefix("### ").or_else(|| l.strip_prefix("## ")).or_else(|| l.strip_prefix("# ")) {
            Line::from(Span::styled(h.to_string(), Style::default().fg(ACCENT).add_modifier(Modifier::BOLD)))
        } else { Line::from(l.replace("**", "")) }
    }).collect()
}

fn draw_help(f: &mut Frame, area: Rect) {
    let r = centered(area, 70, 80);
    f.render_widget(Clear, r);
    let lines = vec![
        "MOVING TIME", "  1 2 3 4 5 6 7 8 9   run 1, 5, 10, 50, 100, 200, 300, 500 or 1000 years", "  s                   stop at the end of the current year", "",
        "LOOKING AROUND", "  ← → or Tab, F1-F10   switch tabs", "  ↑ ↓                 move in a list, or scroll", "  PgUp PgDn           scroll a sheet, a life or a book",
        "  /                   search (Chronicle, People)", "  f / m               chronicle: kind of event / major only", "  d                   people: include the dead", "",
        "THE GODS", "  g                   god mode: every act on the world", "  c                   create a character (presets: p, roll a name: r, randomize: R)",
        "  e                   change the selected character (or pick one)", "  t                   track or stop tracking the selected character", "  x                   end the world", "",
        "IN FORMS", "  ↑ ↓ field · ← → change (Shift ×10) · Enter edit, choose from a list or tick a checklist · F10 do it · Esc back", "",
        "  q or Esc            back to the worlds list (the world is saved)", "", "Press any key.",
    ];
    f.render_widget(Paragraph::new(lines.into_iter().map(Line::from).collect::<Vec<_>>()).block(Block::default().borders(Borders::ALL).title(Span::styled(" Keys ", Style::default().fg(ACCENT)))), r);
}
