//! Forms and pickers: every input in the game is a list of fields you move through, with a help line for each, and
//! any choice longer than a handful opens a searchable list. Nothing has to be known by heart.

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph, Wrap};
use ratatui::Frame;
use serde_json::{json, Value};

pub const ACCENT: Color = Color::Rgb(240, 175, 40);
pub const DIM: Color = Color::Rgb(130, 130, 140);

/// A searchable list: type to narrow it, arrows to move, Enter to choose, Esc to back out.
pub struct Picker {
    pub title: String,
    pub items: Vec<(String, Value, String)>, // label, value, help
    pub query: String,
    pub state: ListState,
    pub multi: Option<Vec<bool>>,             // a checklist when Some
}

pub enum PickResult { Open, Chosen(Value), Done(Vec<Value>), Cancel }

impl Picker {
    pub fn new(title: &str, items: Vec<(String, Value, String)>) -> Picker {
        let mut state = ListState::default();
        state.select(Some(0));
        Picker { title: title.into(), items, query: String::new(), state, multi: None }
    }
    pub fn checklist(title: &str, items: Vec<(String, Value, String)>, on: Vec<bool>) -> Picker {
        let mut p = Picker::new(title, items);
        p.multi = Some(on);
        p
    }
    fn visible(&self) -> Vec<usize> {
        let q = self.query.to_lowercase();
        (0..self.items.len()).filter(|&i| q.is_empty() || self.items[i].0.to_lowercase().contains(&q)).collect()
    }
    pub fn key(&mut self, k: KeyEvent) -> PickResult {
        let vis = self.visible();
        let sel = self.state.selected().unwrap_or(0).min(vis.len().saturating_sub(1));
        match k.code {
            KeyCode::Esc => return if self.multi.is_some() { self.done() } else { PickResult::Cancel },
            KeyCode::Up => self.state.select(Some(sel.saturating_sub(1))),
            KeyCode::Down => self.state.select(Some((sel + 1).min(vis.len().saturating_sub(1)))),
            KeyCode::PageUp => self.state.select(Some(sel.saturating_sub(15))),
            KeyCode::PageDown => self.state.select(Some((sel + 15).min(vis.len().saturating_sub(1)))),
            KeyCode::Enter | KeyCode::Char(' ') if self.multi.is_some() && (k.code == KeyCode::Char(' ') || !vis.is_empty()) => {
                if k.code == KeyCode::Enter && k.modifiers.contains(KeyModifiers::CONTROL) { return self.done(); }
                if let Some(&i) = vis.get(sel) { if let Some(m) = self.multi.as_mut() { m[i] = !m[i]; } }
            }
            KeyCode::Enter => return vis.get(sel).map(|&i| PickResult::Chosen(self.items[i].1.clone())).unwrap_or(PickResult::Cancel),
            KeyCode::Backspace => { self.query.pop(); self.state.select(Some(0)); }
            KeyCode::Char(c) => { self.query.push(c); self.state.select(Some(0)); }
            _ => {}
        }
        PickResult::Open
    }
    fn done(&self) -> PickResult {
        let m = self.multi.as_ref().unwrap();
        PickResult::Done((0..self.items.len()).filter(|&i| m[i]).map(|i| self.items[i].1.clone()).collect())
    }
    pub fn draw(&mut self, f: &mut Frame, area: Rect) {
        let r = centered(area, 70, 80);
        f.render_widget(Clear, r);
        let [list_r, help_r] = Layout::vertical([Constraint::Min(5), Constraint::Length(4)]).areas(r);
        let vis = self.visible();
        let items: Vec<ListItem> = vis.iter().map(|&i| {
            let mark = match &self.multi { Some(m) => if m[i] { "[x] " } else { "[ ] " }, None => "" };
            ListItem::new(format!("{mark}{}", self.items[i].0))
        }).collect();
        let hint = if self.multi.is_some() { "type to search · Space/Enter toggle · Esc done" } else { "type to search · ↑↓ move · Enter choose · Esc back" };
        let title = format!(" {} {} ", self.title, if self.query.is_empty() { String::new() } else { format!("· \"{}\"", self.query) });
        let list = List::new(items).block(Block::default().borders(Borders::ALL).title(title).title_bottom(Line::from(format!(" {hint} ")).style(Style::default().fg(DIM))))
            .highlight_style(Style::default().bg(Color::Rgb(60, 60, 90)).add_modifier(Modifier::BOLD)).highlight_symbol("▶ ");
        f.render_stateful_widget(list, list_r, &mut self.state);
        let help = vis.get(self.state.selected().unwrap_or(0)).map(|&i| self.items[i].2.clone()).unwrap_or_default();
        f.render_widget(Paragraph::new(help).wrap(Wrap { trim: true }).block(Block::default().borders(Borders::ALL)).style(Style::default().fg(DIM)), help_r);
    }
}

#[derive(Clone)]
pub enum Kind {
    Text,
    Number { min: f64, max: f64, step: f64 },
    Toggle,
    Choice(Vec<(String, Value, String)>), // label, value, help
    Action,                              // a button: Enter runs the form
    Header,                              // a section title
    Multi(Vec<(String, Value, String)>), // a checklist; the value is the array of chosen values
}

#[derive(Clone)]
pub struct Field {
    pub key: String,
    pub label: String,
    pub kind: Kind,
    pub value: Value,
    pub help: String,
}

impl Field {
    pub fn text(key: &str, label: &str, v: &str, help: &str) -> Field { Field { key: key.into(), label: label.into(), kind: Kind::Text, value: json!(v), help: help.into() } }
    pub fn num(key: &str, label: &str, v: f64, min: f64, max: f64, step: f64, help: &str) -> Field {
        Field { key: key.into(), label: label.into(), kind: Kind::Number { min, max, step }, value: json!(v), help: help.into() }
    }
    pub fn toggle(key: &str, label: &str, on: bool, help: &str) -> Field { Field { key: key.into(), label: label.into(), kind: Kind::Toggle, value: json!(on), help: help.into() } }
    pub fn choice(key: &str, label: &str, opts: Vec<(String, Value, String)>, help: &str) -> Field {
        let v = opts.first().map(|o| o.1.clone()).unwrap_or(Value::Null);
        Field { key: key.into(), label: label.into(), kind: Kind::Choice(opts), value: v, help: help.into() }
    }
    pub fn action(label: &str, help: &str) -> Field { Field { key: String::new(), label: label.into(), kind: Kind::Action, value: Value::Null, help: help.into() } }
    pub fn header(label: &str) -> Field { Field { key: String::new(), label: label.into(), kind: Kind::Header, value: Value::Null, help: String::new() } }
    pub fn multi(key: &str, label: &str, opts: Vec<(String, Value, String)>, on: Vec<Value>, help: &str) -> Field {
        Field { key: key.into(), label: label.into(), kind: Kind::Multi(opts), value: Value::Array(on), help: help.into() }
    }

    pub fn shown(&self) -> String {
        match &self.kind {
            Kind::Text => self.value.as_str().unwrap_or("").to_string(),
            Kind::Number { .. } => { let v = self.value.as_f64().unwrap_or(0.0); if v.fract() == 0.0 { format!("{v:.0}") } else { format!("{v:.1}") } }
            Kind::Toggle => if self.value.as_bool().unwrap_or(false) { "yes".into() } else { "no".into() },
            Kind::Choice(o) => o.iter().find(|x| x.1 == self.value).map(|x| x.0.clone()).unwrap_or_else(|| "(none)".into()),
            Kind::Action | Kind::Header => String::new(),
            Kind::Multi(o) => {
                let on = self.value.as_array().cloned().unwrap_or_default();
                let names: Vec<String> = o.iter().filter(|x| on.contains(&x.1)).map(|x| x.0.split(" (").next().unwrap_or("").to_string()).collect();
                if names.is_empty() { "(none)".into() } else { names.join(", ") }
            }
        }
    }
    fn cycle(&mut self, d: i64) {
        match &self.kind {
            Kind::Number { min, max, step } => { let v = self.value.as_f64().unwrap_or(0.0) + d as f64 * step; self.value = json!(v.clamp(*min, *max)); }
            Kind::Toggle => self.value = json!(!self.value.as_bool().unwrap_or(false)),
            Kind::Choice(o) if !o.is_empty() => {
                let i = o.iter().position(|x| x.1 == self.value).unwrap_or(0) as i64;
                self.value = o[(i + d).rem_euclid(o.len() as i64) as usize].1.clone();
            }
            _ => {}
        }
    }
}

pub enum FormResult { Open, Submit, Cancel }

/// A list of fields. ↑↓ move, ←→ change a value, Enter edits text, opens a long list or presses a button.
pub struct Form {
    pub title: String,
    pub fields: Vec<Field>,
    pub sel: usize,
    pub editing: bool,
    pub picker: Option<(usize, Picker)>,
    pub note: String,
    /// The checklist just closed, so the owner can tidy it (opposed traits)
    pub multi_done: Option<usize>,
}

impl Form {
    pub fn new(title: &str, fields: Vec<Field>) -> Form { let mut f = Form { title: title.into(), fields, sel: 0, editing: false, picker: None, note: String::new(), multi_done: None }; f.skip_headers(1); f }
    fn skip_headers(&mut self, d: i64) {
        let n = self.fields.len() as i64;
        for _ in 0..n { if matches!(self.fields[self.sel].kind, Kind::Header) { self.sel = (self.sel as i64 + d).rem_euclid(n) as usize; } else { break; } }
    }
    pub fn get(&self, key: &str) -> Value { self.fields.iter().find(|f| f.key == key).map(|f| f.value.clone()).unwrap_or(Value::Null) }
    pub fn set(&mut self, key: &str, v: Value) { if let Some(f) = self.fields.iter_mut().find(|f| f.key == key) { f.value = v; } }
    /// The form's values as god-act arguments (buttons and blank text left out).
    pub fn args(&self) -> serde_json::Map<String, Value> {
        let mut m = serde_json::Map::new();
        for f in &self.fields {
            if f.key.is_empty() || matches!(f.kind, Kind::Action | Kind::Header) { continue; }
            if matches!(f.kind, Kind::Text) && f.value.as_str().unwrap_or("").is_empty() { continue; }
            m.insert(f.key.clone(), f.value.clone());
        }
        m
    }

    pub fn key(&mut self, k: KeyEvent) -> FormResult {
        if let Some((i, p)) = self.picker.as_mut() {
            match p.key(k) {
                PickResult::Chosen(v) => { let i = *i; self.fields[i].value = v; self.picker = None; }
                PickResult::Done(vs) => { let i = *i; self.fields[i].value = Value::Array(vs); self.picker = None; self.multi_done = Some(i); }
                PickResult::Cancel => self.picker = None,
                PickResult::Open => {}
            }
            return FormResult::Open;
        }
        let n = self.fields.len();
        if self.editing {
            let f = &mut self.fields[self.sel];
            match k.code {
                KeyCode::Enter | KeyCode::Esc | KeyCode::Tab => self.editing = false,
                KeyCode::Backspace => {
                    let mut s = f.shown(); s.pop();
                    f.value = if matches!(f.kind, Kind::Number { .. }) { json!(s.parse::<f64>().unwrap_or(0.0)) } else { json!(s) };
                }
                KeyCode::Char(c) => {
                    if let Kind::Number { min, max, .. } = f.kind {
                        if c.is_ascii_digit() || c == '.' || c == '-' {
                            let s = format!("{}{c}", if f.value.as_f64().unwrap_or(0.0) == 0.0 { String::new() } else { f.shown() });
                            if let Ok(v) = s.parse::<f64>() { f.value = json!(v.clamp(min.min(0.0), max)); }
                        }
                    } else { let s = format!("{}{c}", f.shown()); f.value = json!(s); }
                }
                _ => {}
            }
            return FormResult::Open;
        }
        match k.code {
            KeyCode::Esc => return FormResult::Cancel,
            KeyCode::Up | KeyCode::BackTab => { self.sel = (self.sel + n - 1) % n.max(1); self.skip_headers(-1); }
            KeyCode::Down | KeyCode::Tab => { self.sel = (self.sel + 1) % n.max(1); self.skip_headers(1); }
            KeyCode::PageUp => { self.sel = self.sel.saturating_sub(10); self.skip_headers(1); }
            KeyCode::PageDown => { self.sel = (self.sel + 10).min(n.saturating_sub(1)); self.skip_headers(-1); }
            KeyCode::Left | KeyCode::Char('-') => self.fields[self.sel].cycle(if k.modifiers.contains(KeyModifiers::SHIFT) { -10 } else { -1 }),
            KeyCode::Right | KeyCode::Char('+') | KeyCode::Char('=') => self.fields[self.sel].cycle(if k.modifiers.contains(KeyModifiers::SHIFT) { 10 } else { 1 }),
            KeyCode::F(10) => return FormResult::Submit,
            KeyCode::Enter | KeyCode::Char(' ') => {
                let f = &self.fields[self.sel];
                match &f.kind {
                    Kind::Action => return FormResult::Submit,
                    Kind::Header => {}
                    Kind::Multi(o) => {
                        let on = f.value.as_array().cloned().unwrap_or_default();
                        let flags = o.iter().map(|x| on.contains(&x.1)).collect();
                        self.picker = Some((self.sel, Picker::checklist(&f.label, o.clone(), flags)));
                    }
                    Kind::Text | Kind::Number { .. } => self.editing = true,
                    Kind::Toggle => self.fields[self.sel].cycle(1),
                    Kind::Choice(o) => {
                        if o.len() <= 3 { self.fields[self.sel].cycle(1); } else {
                            let mut p = Picker::new(&f.label, o.clone());
                            let at = o.iter().position(|x| x.1 == f.value).unwrap_or(0);
                            p.state.select(Some(at));
                            self.picker = Some((self.sel, p));
                        }
                    }
                }
            }
            _ => {}
        }
        FormResult::Open
    }

    pub fn draw(&mut self, f: &mut Frame, area: Rect) {
        let r = centered(area, 76, 86);
        f.render_widget(Clear, r);
        let [body, help] = Layout::vertical([Constraint::Min(5), Constraint::Length(5)]).areas(r);
        let lw = self.fields.iter().map(|x| x.label.chars().count()).max().unwrap_or(10).min(28);
        let height = body.height.saturating_sub(2) as usize;
        let start = self.sel.saturating_sub(height.saturating_sub(1));
        let lines: Vec<Line> = self.fields.iter().enumerate().skip(start).take(height).map(|(i, fl)| {
            let on = i == self.sel;
            let st = if on { Style::default().bg(Color::Rgb(60, 60, 90)).add_modifier(Modifier::BOLD) } else { Style::default() };
            if matches!(fl.kind, Kind::Header) {
                return Line::from(Span::styled(format!("── {} ", fl.label), Style::default().fg(ACCENT).add_modifier(Modifier::BOLD)));
            }
            if matches!(fl.kind, Kind::Action) {
                return Line::from(Span::styled(format!("  [ {} ]", fl.label), st.fg(ACCENT)));
            }
            let val = fl.shown();
            let cursor = if on && self.editing { "▏" } else { "" };
            let arrows = match fl.kind { Kind::Choice(ref o) if o.len() > 3 => " ▾", Kind::Multi(_) => " ☰", Kind::Number { .. } | Kind::Choice(_) | Kind::Toggle => " ◂▸", _ => "" };
            Line::from(vec![Span::styled(format!(" {:<lw$}  ", fl.label), st.fg(DIM)), Span::styled(format!("{val}{cursor}"), st), Span::styled(if on { arrows } else { "" }, Style::default().fg(DIM))])
        }).collect();
        let title = format!(" {} ", self.title);
        let keys = " ↑↓ field · ←→ change (Shift ×10) · Enter edit/choose · F10 do it · Esc back ";
        f.render_widget(Paragraph::new(lines).block(Block::default().borders(Borders::ALL).title(Span::styled(title, Style::default().fg(ACCENT).add_modifier(Modifier::BOLD))).title_bottom(Line::from(keys).style(Style::default().fg(DIM)))), body);
        let mut h = self.fields.get(self.sel).map(|x| x.help.clone()).unwrap_or_default();
        if !self.note.is_empty() { h = format!("{}\n{h}", self.note); }
        f.render_widget(Paragraph::new(h).wrap(Wrap { trim: true }).block(Block::default().borders(Borders::ALL)).style(Style::default().fg(Color::Rgb(200, 200, 210))), help);
        if let Some((_, p)) = self.picker.as_mut() { p.draw(f, area); }
    }
}

pub fn centered(area: Rect, pw: u16, ph: u16) -> Rect {
    let w = (area.width as u32 * pw as u32 / 100) as u16;
    let h = (area.height as u32 * ph as u32 / 100) as u16;
    Rect { x: area.x + (area.width - w) / 2, y: area.y + (area.height - h) / 2, width: w, height: h }
}
