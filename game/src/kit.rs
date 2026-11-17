//! The look and feel of the game, after the Textual interface it began as: a header, a footer of key bindings,
//! buttons in rows (green to go, blue for god mode, amber to stop, red for what cannot be undone), input boxes,
//! drop-down selects, striped tables, tabs underlined in amber, a progress bar, and notices that slide in at the top
//! right. Everything can be clicked; Tab and Shift+Tab move between controls; Enter or Space presses a button.

use std::collections::HashMap;
use std::time::{Duration, Instant};

use crossterm::event::{KeyCode, KeyEvent, MouseButton, MouseEvent, MouseEventKind};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::Frame;
use serde_json::Value;

// ---------------------------------------------------------------- the theme (Textual's dark theme)
pub const BG: Color = Color::Rgb(18, 18, 18);
pub const SURFACE: Color = Color::Rgb(30, 30, 30);
pub const PANEL: Color = Color::Rgb(36, 41, 47);
pub const BOOST: Color = Color::Rgb(44, 44, 48);
pub const TEXT: Color = Color::Rgb(224, 224, 224);
pub const MUTED: Color = Color::Rgb(138, 138, 138);
pub const PRIMARY: Color = Color::Rgb(1, 120, 212);
pub const ACCENT: Color = Color::Rgb(255, 166, 43);
pub const SUCCESS: Color = Color::Rgb(78, 191, 113);
pub const WARNING: Color = Color::Rgb(255, 166, 43);
pub const ERROR: Color = Color::Rgb(186, 60, 91);
pub const MAGENTA: Color = Color::Rgb(200, 80, 200);

#[derive(Clone, Copy, PartialEq)]
pub enum Variant { Default, Success, Primary, Warning, Error }

fn shade(c: Color, f: f64) -> Color {
    if let Color::Rgb(r, g, b) = c { let s = |x: u8| ((x as f64 * f).clamp(0.0, 255.0)) as u8; Color::Rgb(s(r), s(g), s(b)) } else { c }
}

fn variant_colors(v: Variant) -> (Color, Color) {
    match v {
        Variant::Default => (Color::Rgb(58, 58, 64), TEXT),
        Variant::Success => (SUCCESS, Color::Rgb(16, 16, 16)),
        Variant::Primary => (PRIMARY, Color::White),
        Variant::Warning => (WARNING, Color::Rgb(16, 16, 16)),
        Variant::Error => (ERROR, Color::White),
    }
}

/// A drop-down: one choice (or several, as a checklist), with a search line when it is long.
#[derive(Clone)]
pub struct Select {
    pub prompt: String,
    pub opts: Vec<(String, Value, String)>, // label, value, help
    pub sel: Option<usize>,
    pub multi: bool,
    pub checked: Vec<bool>,
    pub query: String,
    pub hl: usize,
}

impl Select {
    pub fn new(prompt: &str, opts: Vec<(String, Value, String)>) -> Select {
        let n = opts.len();
        Select { prompt: prompt.into(), opts, sel: None, multi: false, checked: vec![false; n], query: String::new(), hl: 0 }
    }
    pub fn with(mut self, v: Value) -> Select { self.sel = self.opts.iter().position(|o| o.1 == v); self }
    pub fn first(mut self) -> Select { if !self.opts.is_empty() { self.sel = Some(0); } self }
    pub fn checklist(prompt: &str, opts: Vec<(String, Value, String)>, on: &[Value]) -> Select {
        let mut s = Select::new(prompt, opts);
        s.multi = true;
        s.checked = s.opts.iter().map(|o| on.contains(&o.1)).collect();
        s
    }
    pub fn value(&self) -> Option<Value> { self.sel.map(|i| self.opts[i].1.clone()) }
    pub fn values(&self) -> Vec<Value> { (0..self.opts.len()).filter(|&i| self.checked[i]).map(|i| self.opts[i].1.clone()).collect() }
    pub fn shown(&self) -> String {
        if self.multi {
            let names: Vec<&str> = (0..self.opts.len()).filter(|&i| self.checked[i]).map(|i| self.opts[i].0.split(" (").next().unwrap_or("")).collect();
            if names.is_empty() { String::new() } else { names.join(", ") }
        } else { self.sel.map(|i| self.opts[i].0.clone()).unwrap_or_default() }
    }
    fn visible(&self) -> Vec<usize> {
        let q = self.query.to_lowercase();
        (0..self.opts.len()).filter(|&i| q.is_empty() || self.opts[i].0.to_lowercase().contains(&q)).collect()
    }
}

pub struct Toast { pub text: String, pub error: bool, pub at: Instant }

/// The controls of one screen: their states, where they were drawn, and which has the focus.
#[derive(Default)]
pub struct Ui {
    pub focus: String,
    pub order: Vec<String>,
    pub hits: Vec<(Rect, String)>,
    pub inputs: HashMap<String, String>,
    pub selects: HashMap<String, Select>,
    pub open: Option<String>,
    drop_rect: Option<Rect>,
    pub scroll: HashMap<String, usize>,
    pub cursor: HashMap<String, usize>,
}

pub enum Act { None, Pressed(String), Changed(String), Scrolled }

impl Ui {
    pub fn begin(&mut self) { self.order.clear(); self.hits.clear(); }
    fn reg(&mut self, id: &str, r: Rect, focusable: bool) {
        self.hits.push((r, id.to_string()));
        if focusable { self.order.push(id.to_string()); }
        if self.focus.is_empty() && focusable { self.focus = id.to_string(); }
    }
    pub fn focused(&self, id: &str) -> bool { self.focus == id }
    pub fn input(&self, id: &str) -> String { self.inputs.get(id).cloned().unwrap_or_default() }
    pub fn set_input(&mut self, id: &str, v: &str) { self.inputs.insert(id.into(), v.into()); }
    pub fn sel(&self, id: &str) -> Option<Value> { self.selects.get(id).and_then(|s| s.value()) }
    pub fn checked(&self, id: &str) -> Vec<Value> { self.selects.get(id).map(|s| s.values()).unwrap_or_default() }
    pub fn scroll_of(&self, id: &str) -> usize { self.scroll.get(id).copied().unwrap_or(0) }
    pub fn cursor_of(&self, id: &str) -> usize { self.cursor.get(id).copied().unwrap_or(0) }

    // ---------------------------------------------------------------- drawing
    pub fn button(&mut self, f: &mut Frame, r: Rect, id: &str, label: &str, v: Variant) {
        self.reg(id, r, true);
        let (bg, fg) = variant_colors(v);
        let focus = self.focused(id);
        let bg = if focus { shade(bg, 1.25) } else { bg };
        let buf = f.buffer_mut();
        fill(buf, r, bg);
        if r.height >= 3 {
            for x in r.x..r.x + r.width {
                buf[(x, r.y)].set_symbol("▔").set_style(Style::default().fg(shade(bg, 1.45)).bg(bg));
                buf[(x, r.y + r.height - 1)].set_symbol("▁").set_style(Style::default().fg(shade(bg, 0.55)).bg(bg));
            }
        }
        let style = Style::default().fg(fg).bg(bg).add_modifier(Modifier::BOLD).add_modifier(if focus { Modifier::UNDERLINED } else { Modifier::empty() });
        center(buf, r, r.y + r.height / 2, label, style);
    }

    pub fn text_input(&mut self, f: &mut Frame, r: Rect, id: &str, placeholder: &str) {
        self.reg(id, r, true);
        let focus = self.focused(id);
        boxed(f.buffer_mut(), r, if focus { ACCENT } else { Color::Rgb(70, 70, 76) }, SURFACE);
        let v = self.input(id);
        let inner = inner1(r);
        let buf = f.buffer_mut();
        if v.is_empty() {
            buf.set_stringn(inner.x, inner.y, placeholder, inner.width as usize, Style::default().fg(MUTED).bg(SURFACE));
            if focus { buf.set_string(inner.x, inner.y, "▏", Style::default().fg(ACCENT).bg(SURFACE)); }
        } else {
            let w = inner.width as usize;
            let shown: String = if v.chars().count() + 1 > w { v.chars().skip(v.chars().count() + 1 - w).collect() } else { v.clone() };
            buf.set_stringn(inner.x, inner.y, &shown, w, Style::default().fg(TEXT).bg(SURFACE));
            if focus { let x = inner.x + shown.chars().count() as u16; if x < inner.x + inner.width { buf.set_string(x, inner.y, "▏", Style::default().fg(ACCENT).bg(SURFACE)); } }
        }
    }

    pub fn select(&mut self, f: &mut Frame, r: Rect, id: &str) {
        self.reg(id, r, true);
        let focus = self.focused(id);
        boxed(f.buffer_mut(), r, if focus { ACCENT } else { Color::Rgb(70, 70, 76) }, SURFACE);
        let Some(s) = self.selects.get(id) else { return };
        let inner = inner1(r);
        let shown = s.shown();
        let (t, st) = if shown.is_empty() { (s.prompt.clone(), Style::default().fg(MUTED).bg(SURFACE)) } else { (shown, Style::default().fg(TEXT).bg(SURFACE)) };
        let buf = f.buffer_mut();
        buf.set_stringn(inner.x, inner.y, &t, inner.width.saturating_sub(2) as usize, st);
        buf.set_string(inner.x + inner.width.saturating_sub(1), inner.y, if self.open.as_deref() == Some(id) { "▲" } else { "▼" }, Style::default().fg(if focus { ACCENT } else { MUTED }).bg(SURFACE));
        if self.open.as_deref() == Some(id) { self.drop_rect = Some(r); }
    }

    /// The open drop-down, drawn over everything else.
    pub fn dropdown(&mut self, f: &mut Frame) {
        let (Some(id), Some(anchor)) = (self.open.clone(), self.drop_rect.take()) else { return };
        let Some(s) = self.selects.get(&id) else { return };
        let area = f.area();
        let vis = s.visible();
        let want = (vis.len() as u16 + 2 + if s.opts.len() > 8 { 1 } else { 0 } + if s.multi { 1 } else { 0 }).min(18);
        let below = area.height.saturating_sub(anchor.y + anchor.height);
        let y = if below >= want.min(8) { anchor.y + anchor.height } else { anchor.y.saturating_sub(want) };
        let h = want.min(if below >= want.min(8) { below } else { anchor.y }).max(3);
        let w = anchor.width.max(36).min(area.width.saturating_sub(anchor.x));
        let r = Rect { x: anchor.x, y, width: w, height: h };
        let buf = f.buffer_mut();
        boxed(buf, r, ACCENT, PANEL);
        let inner = inner1(r);
        let mut row = inner.y;
        if s.opts.len() > 8 {
            buf.set_stringn(inner.x, row, &format!("search: {}▏", s.query), inner.width as usize, Style::default().fg(ACCENT).bg(PANEL));
            row += 1;
        }
        let rows = (inner.y + inner.height).saturating_sub(row) as usize - if s.multi { 1 } else { 0 };
        let start = s.hl.saturating_sub(rows.saturating_sub(1));
        for (n, &i) in vis.iter().enumerate().skip(start).take(rows) {
            let on = n == s.hl;
            let mark = if s.multi { if s.checked[i] { "[x] " } else { "[ ] " } } else if Some(i) == s.sel { "● " } else { "  " };
            let st = if on { Style::default().fg(Color::White).bg(PRIMARY).add_modifier(Modifier::BOLD) } else { Style::default().fg(TEXT).bg(PANEL) };
            buf.set_stringn(inner.x, row, &format!("{mark}{:<w$}", s.opts[i].0, w = inner.width as usize), inner.width as usize, st);
            self.hits.push((Rect { x: inner.x, y: row, width: inner.width, height: 1 }, format!("{id}#{n}")));
            row += 1;
        }
        if s.multi { buf.set_stringn(inner.x, inner.y + inner.height - 1, "Space ticks · Enter or Esc closes", inner.width as usize, Style::default().fg(MUTED).bg(PANEL)); }
        if let Some(&i) = vis.get(s.hl) {
            let help = &s.opts[i].2;
            if !help.is_empty() && r.y + r.height < area.height {
                let hr = Rect { x: r.x, y: r.y + r.height, width: r.width, height: 1 };
                buf.set_stringn(hr.x, hr.y, &format!(" {help}"), hr.width as usize, Style::default().fg(Color::Black).bg(ACCENT));
            }
        }
    }

    /// A list you can pick from (an OptionList).
    pub fn option_list(&mut self, f: &mut Frame, r: Rect, id: &str, items: &[String]) {
        self.reg(id, r, true);
        let focus = self.focused(id);
        boxed(f.buffer_mut(), r, if focus { ACCENT } else { Color::Rgb(70, 70, 76) }, SURFACE);
        let inner = inner1(r);
        let cur = self.cursor_of(id).min(items.len().saturating_sub(1));
        let h = inner.height as usize;
        let start = cur.saturating_sub(h.saturating_sub(1));
        let buf = f.buffer_mut();
        for (n, it) in items.iter().enumerate().skip(start).take(h) {
            let y = inner.y + (n - start) as u16;
            let st = if n == cur { Style::default().fg(Color::White).bg(if focus { PRIMARY } else { Color::Rgb(50, 60, 80) }).add_modifier(Modifier::BOLD) } else { Style::default().fg(TEXT).bg(SURFACE) };
            buf.set_stringn(inner.x, y, &format!("{:<w$}", it, w = inner.width as usize), inner.width as usize, st);
        }
    }

    /// A table with a bold header, zebra stripes and (optionally) a row cursor.
    pub fn table(&mut self, f: &mut Frame, r: Rect, id: &str, header: &[&str], rows: &[Vec<String>], widths: &[u16], cursor: bool) {
        self.reg(id, r, true);
        let buf = f.buffer_mut();
        fill(buf, r, BG);
        let mut x = r.x;
        let cols: Vec<(u16, u16)> = widths.iter().map(|&w| { let c = (x, w); x += w + 1; c }).collect();
        let line = |buf: &mut Buffer, y: u16, cells: &[String], st: Style| {
            fill(buf, Rect { x: r.x, y, width: r.width, height: 1 }, st.bg.unwrap_or(BG));
            for (i, c) in cells.iter().enumerate() {
                if let Some(&(cx, w)) = cols.get(i) { if cx < r.x + r.width { buf.set_stringn(cx, y, c, (w as usize).min((r.x + r.width - cx) as usize), st); } }
            }
        };
        let head: Vec<String> = header.iter().map(|h| h.to_string()).collect();
        line(buf, r.y, &head, Style::default().fg(ACCENT).bg(PANEL).add_modifier(Modifier::BOLD));
        let h = r.height.saturating_sub(1) as usize;
        let cur = self.cursor_of(id).min(rows.len().saturating_sub(1));
        let start = if cursor { cur.saturating_sub(h.saturating_sub(1)) } else { self.scroll_of(id).min(rows.len().saturating_sub(h.min(rows.len()))) };
        let focus = self.focused(id);
        for (n, row) in rows.iter().enumerate().skip(start).take(h) {
            let y = r.y + 1 + (n - start) as u16;
            let st = if cursor && n == cur && focus { Style::default().fg(Color::White).bg(PRIMARY) }
                     else if n % 2 == 1 { Style::default().fg(TEXT).bg(Color::Rgb(28, 28, 32)) } else { Style::default().fg(TEXT).bg(BG) };
            line(buf, y, row, st);
        }
    }

    /// Lines of text that scroll (a RichLog); `from_bottom` keeps the newest in view.
    pub fn log(&mut self, f: &mut Frame, r: Rect, id: &str, lines: Vec<Line<'static>>, from_bottom: bool) {
        self.reg(id, r, true);
        let buf = f.buffer_mut();
        fill(buf, r, BG);
        let wrapped = wrap_lines(lines, r.width.saturating_sub(1));
        let h = r.height as usize;
        let max_scroll = wrapped.len().saturating_sub(h);
        let s = self.scroll_of(id).min(max_scroll);
        self.scroll.insert(id.into(), s);
        let start = if from_bottom { max_scroll - s } else { s };
        for (n, l) in wrapped.iter().skip(start).take(h).enumerate() { buf.set_line(r.x, r.y + n as u16, l, r.width.saturating_sub(1)); }
        if wrapped.len() > h {
            let pos = (start as f64 / max_scroll.max(1) as f64 * (h - 1) as f64) as u16;
            for y in 0..h as u16 { buf[(r.x + r.width - 1, r.y + y)].set_symbol(if y == pos { "█" } else { "│" }).set_style(Style::default().fg(if y == pos { ACCENT } else { Color::Rgb(60, 60, 60) })); }
        }
    }

    pub fn label(&self, f: &mut Frame, r: Rect, text: &str, st: Style) { f.buffer_mut().set_stringn(r.x, r.y, text, r.width as usize, st); }

    // ---------------------------------------------------------------- input
    /// A key for the focused control. Returns what happened; Tab and Shift+Tab are handled here.
    pub fn key(&mut self, k: KeyEvent) -> Act {
        if let Some(id) = self.open.clone() {
            let s = self.selects.get_mut(&id).unwrap();
            let vis = s.visible();
            match k.code {
                KeyCode::Esc => { self.open = None; return if s.multi { Act::Changed(id) } else { Act::None }; }
                KeyCode::Up => s.hl = s.hl.saturating_sub(1),
                KeyCode::Down => s.hl = (s.hl + 1).min(vis.len().saturating_sub(1)),
                KeyCode::PageUp => s.hl = s.hl.saturating_sub(12),
                KeyCode::PageDown => s.hl = (s.hl + 12).min(vis.len().saturating_sub(1)),
                KeyCode::Char(' ') if s.multi => { if let Some(&i) = vis.get(s.hl) { s.checked[i] = !s.checked[i]; } }
                KeyCode::Enter => {
                    if s.multi { self.open = None; return Act::Changed(id); }
                    if let Some(&i) = vis.get(s.hl) { s.sel = Some(i); }
                    self.open = None;
                    return Act::Changed(id);
                }
                KeyCode::Backspace => { s.query.pop(); s.hl = 0; }
                KeyCode::Char(c) => { s.query.push(c); s.hl = 0; }
                _ => {}
            }
            return Act::None;
        }
        match k.code {
            KeyCode::Tab => { self.move_focus(1); return Act::None; }
            KeyCode::BackTab => { self.move_focus(-1); return Act::None; }
            _ => {}
        }
        let id = self.focus.clone();
        if self.inputs.contains_key(&id) || id.starts_with("in:") {
            let v = self.inputs.entry(id.clone()).or_default();
            match k.code {
                KeyCode::Backspace => { v.pop(); return Act::Changed(id); }
                KeyCode::Char(c) => { v.push(c); return Act::Changed(id); }
                KeyCode::Enter => return Act::Pressed(id),
                _ => return Act::None,
            }
        }
        if self.selects.contains_key(&id) {
            if matches!(k.code, KeyCode::Enter | KeyCode::Char(' ') | KeyCode::Down) {
                let s = self.selects.get_mut(&id).unwrap();
                s.query.clear();
                s.hl = s.sel.unwrap_or(0);
                self.open = Some(id);
            }
            return Act::None;
        }
        match k.code {
            KeyCode::Enter | KeyCode::Char(' ') if !self.cursor.contains_key(&id) && !self.scroll.contains_key(&id) => Act::Pressed(id),
            KeyCode::Enter => Act::Pressed(id),
            KeyCode::Up => { self.nudge(&id, -1); Act::Scrolled }
            KeyCode::Down => { self.nudge(&id, 1); Act::Scrolled }
            KeyCode::PageUp => { self.nudge(&id, -15); Act::Scrolled }
            KeyCode::PageDown => { self.nudge(&id, 15); Act::Scrolled }
            _ => Act::None,
        }
    }

    /// Lists move their cursor; logs and tables move their scroll (logs that read from the bottom scroll back).
    fn nudge(&mut self, id: &str, d: i64) {
        if let Some(c) = self.cursor.get_mut(id) { *c = (*c as i64 + d).max(0) as usize; return; }
        let s = self.scroll.entry(id.to_string()).or_insert(0);
        *s = (*s as i64 + if id.ends_with("<") { -d } else { d }).max(0) as usize;
    }

    pub fn move_focus(&mut self, d: i64) {
        if self.order.is_empty() { return; }
        let i = self.order.iter().position(|x| *x == self.focus).map(|i| i as i64).unwrap_or(-1);
        let n = self.order.len() as i64;
        self.focus = self.order[(i + d).rem_euclid(n) as usize].clone();
    }

    pub fn mouse(&mut self, m: MouseEvent) -> Act {
        let hit = self.hits.iter().rev().find(|(r, _)| m.column >= r.x && m.column < r.x + r.width && m.row >= r.y && m.row < r.y + r.height).map(|(r, id)| (*r, id.clone()));
        match m.kind {
            MouseEventKind::ScrollUp | MouseEventKind::ScrollDown => {
                let d = if matches!(m.kind, MouseEventKind::ScrollUp) { -3 } else { 3 };
                if let Some(id) = self.open.clone() { let s = self.selects.get_mut(&id).unwrap(); s.hl = (s.hl as i64 + d).max(0) as usize; return Act::None; }
                if let Some((_, id)) = hit { self.nudge(&id, d); return Act::Scrolled; }
                Act::None
            }
            MouseEventKind::Down(MouseButton::Left) => {
                let Some((r, id)) = hit else { self.open = None; return Act::None };
                if let Some((sid, n)) = id.split_once('#') {
                    let s = self.selects.get_mut(sid).unwrap();
                    let vis = s.visible();
                    if let Some(&i) = vis.get(n.parse::<usize>().unwrap_or(0)) {
                        s.hl = n.parse().unwrap_or(0);
                        if s.multi { s.checked[i] = !s.checked[i]; return Act::None; }
                        s.sel = Some(i);
                    }
                    self.open = None;
                    return Act::Changed(sid.to_string());
                }
                if self.open.is_some() { self.open = None; }
                self.focus = id.clone();
                if self.selects.contains_key(&id) {
                    let s = self.selects.get_mut(&id).unwrap();
                    s.query.clear();
                    s.hl = s.sel.unwrap_or(0);
                    self.open = Some(id);
                    return Act::None;
                }
                if self.cursor.contains_key(&id) {
                    let start = self.cursor_of(&id).saturating_sub((r.height as usize).saturating_sub(3));
                    let row = (m.row.saturating_sub(r.y + 1)) as usize + start;
                    self.cursor.insert(id.clone(), row);
                    return Act::Changed(id);
                }
                if self.inputs.contains_key(&id) || self.scroll.contains_key(&id) { return Act::None; }
                Act::Pressed(id)
            }
            _ => Act::None,
        }
    }
}

// ---------------------------------------------------------------- painting helpers
pub fn fill(buf: &mut Buffer, r: Rect, bg: Color) {
    let r = r.intersection(buf.area);
    for y in r.y..r.y + r.height { for x in r.x..r.x + r.width { buf[(x, y)].set_symbol(" ").set_style(Style::default().bg(bg)); } }
}

/// Textual's "tall" border: thin bars above and below, thick on the left.
pub fn boxed(buf: &mut Buffer, r: Rect, border: Color, bg: Color) {
    let r = r.intersection(buf.area);
    if r.width < 2 || r.height < 2 { return; }
    fill(buf, r, bg);
    let st = Style::default().fg(border).bg(bg);
    for x in r.x..r.x + r.width { buf[(x, r.y)].set_symbol("▔").set_style(st); buf[(x, r.y + r.height - 1)].set_symbol("▁").set_style(st); }
    for y in r.y..r.y + r.height { buf[(r.x, y)].set_symbol("▊").set_style(st); buf[(r.x + r.width - 1, y)].set_symbol("▎").set_style(st); }
}

pub fn inner1(r: Rect) -> Rect { Rect { x: r.x + 2, y: r.y + 1, width: r.width.saturating_sub(4), height: r.height.saturating_sub(2) } }

pub fn center(buf: &mut Buffer, r: Rect, y: u16, text: &str, st: Style) {
    let w = text.chars().count() as u16;
    let x = r.x + r.width.saturating_sub(w) / 2;
    buf.set_stringn(x, y, text, r.width as usize, st);
}

/// A modal's frame: a thick border in its color over the dimmed screen.
pub fn modal(f: &mut Frame, r: Rect, color: Color) -> Rect {
    let buf = f.buffer_mut();
    let area = buf.area;
    for y in area.y..area.y + area.height { for x in area.x..area.x + area.width {
        let c = &mut buf[(x, y)];
        let st = c.style();
        c.set_style(st.fg(shade(st.fg.unwrap_or(TEXT), 0.45)).bg(shade(st.bg.unwrap_or(BG), 0.5)));
    } }
    fill(buf, r, SURFACE);
    let st = Style::default().fg(color).bg(SURFACE);
    for x in r.x..r.x + r.width { buf[(x, r.y)].set_symbol("▀").set_style(Style::default().fg(color).bg(shade(BG, 0.5))); buf[(x, r.y + r.height - 1)].set_symbol("▄").set_style(Style::default().fg(color).bg(shade(BG, 0.5))); }
    for y in r.y..r.y + r.height { buf[(r.x, y)].set_symbol("█").set_style(st); buf[(r.x + r.width - 1, y)].set_symbol("█").set_style(st); }
    Rect { x: r.x + 3, y: r.y + 1, width: r.width.saturating_sub(6), height: r.height.saturating_sub(2) }
}

pub fn centered(area: Rect, w: u16, h: u16) -> Rect {
    let w = w.min(area.width);
    let h = h.min(area.height);
    Rect { x: area.x + (area.width - w) / 2, y: area.y + (area.height - h) / 2, width: w, height: h }
}

pub fn header(f: &mut Frame, r: Rect, title: &str, sub: &str) {
    let buf = f.buffer_mut();
    fill(buf, r, PANEL);
    buf.set_string(r.x + 1, r.y, "⭘", Style::default().fg(TEXT).bg(PANEL));
    let t = if sub.is_empty() { title.to_string() } else { format!("{title} — {sub}") };
    center(buf, r, r.y, &t, Style::default().fg(TEXT).bg(PANEL));
}

/// The footer: the bindings that work here, key in amber.
pub fn footer(f: &mut Frame, r: Rect, keys: &[(&str, &str)]) {
    let buf = f.buffer_mut();
    fill(buf, r, PANEL);
    let mut spans: Vec<Span> = vec![];
    for (k, d) in keys {
        spans.push(Span::styled(format!(" {k} "), Style::default().fg(Color::Black).bg(ACCENT).add_modifier(Modifier::BOLD)));
        spans.push(Span::styled(format!(" {d} "), Style::default().fg(TEXT).bg(PANEL)));
    }
    buf.set_line(r.x, r.y, &Line::from(spans), r.width);
}

/// Tabs, the active one underlined in amber. Returns where each tab was drawn, for clicks.
pub fn tabs(f: &mut Frame, r: Rect, names: &[&str], active: usize, ui: &mut Ui) {
    let buf = f.buffer_mut();
    fill(buf, r, BG);
    let mut x = r.x + 1;
    for (i, n) in names.iter().enumerate() {
        let w = n.chars().count() as u16 + 2;
        if x + w > r.x + r.width { break; }
        let on = i == active;
        buf.set_string(x, r.y, format!(" {n} "), Style::default().fg(if on { TEXT } else { MUTED }).bg(BG).add_modifier(if on { Modifier::BOLD } else { Modifier::empty() }));
        for xx in x..x + w { buf[(xx, r.y + 1)].set_symbol(if on { "━" } else { "─" }).set_style(Style::default().fg(if on { ACCENT } else { Color::Rgb(60, 60, 60) }).bg(BG)); }
        ui.hits.push((Rect { x, y: r.y, width: w, height: 2 }, format!("tab:{i}")));
        x += w + 1;
    }
    for xx in x..r.x + r.width { buf[(xx, r.y + 1)].set_symbol("─").set_style(Style::default().fg(Color::Rgb(60, 60, 60)).bg(BG)); }
}

pub fn progress(f: &mut Frame, r: Rect, ratio: Option<f64>, eta: Option<Duration>) {
    let buf = f.buffer_mut();
    fill(buf, r, BG);
    let bw = r.width.saturating_sub(22).min(60);
    match ratio {
        Some(p) => {
            let done = (p.clamp(0.0, 1.0) * bw as f64) as u16;
            for x in 0..bw { buf[(r.x + 1 + x, r.y)].set_symbol("━").set_style(Style::default().fg(if x < done { ACCENT } else { Color::Rgb(60, 60, 60) }).bg(BG)); }
            let e = eta.map(|d| { let s = d.as_secs(); format!("{:02}:{:02}:{:02}", s / 3600, (s / 60) % 60, s % 60) }).unwrap_or_else(|| "--:--:--".into());
            buf.set_string(r.x + bw + 3, r.y, format!("{:>3.0}%  {e}", p * 100.0), Style::default().fg(TEXT).bg(BG));
        }
        None => {
            for x in 0..bw { buf[(r.x + 1 + x, r.y)].set_symbol("━").set_style(Style::default().fg(Color::Rgb(60, 60, 60)).bg(BG)); }
            buf.set_string(r.x + bw + 3, r.y, "  --%  --:--:--", Style::default().fg(MUTED).bg(BG));
        }
    }
}

pub fn toasts(f: &mut Frame, list: &mut Vec<Toast>) {
    list.retain(|t| t.at.elapsed() < Duration::from_secs(6));
    let area = f.area();
    // stacked up from the bottom right, above the footer, as Textual shows them
    let mut bottom = area.y + area.height.saturating_sub(2);
    for t in list.iter().rev().take(5) {
        let w = 52u16.min(area.width);
        let lines = wrap_lines(vec![Line::from(t.text.clone())], w - 4);
        let h = lines.len() as u16 + 2;
        if bottom < area.y + h { break; }
        let r = Rect { x: area.x + area.width - w - 1, y: bottom - h, width: w, height: h };
        let buf = f.buffer_mut();
        fill(buf, r, PANEL);
        let edge = if t.error { ERROR } else { PRIMARY };
        for yy in r.y..r.y + r.height { buf[(r.x, yy)].set_symbol("▌").set_style(Style::default().fg(edge).bg(PANEL)); }
        for (n, l) in lines.iter().enumerate() { buf.set_line(r.x + 2, r.y + 1 + n as u16, l, w - 3); }
        bottom = r.y.saturating_sub(1);
    }
}

/// Lines folded to a width, keeping their styles.
pub fn wrap_lines(lines: Vec<Line<'static>>, width: u16) -> Vec<Line<'static>> {
    let w = width.max(10) as usize;
    let mut out = vec![];
    for l in lines {
        let total: usize = l.spans.iter().map(|s| s.content.chars().count()).sum();
        if total <= w { out.push(l); continue; }
        let mut cur: Vec<Span<'static>> = vec![];
        let mut n = 0;
        for sp in l.spans {
            let st = sp.style;
            let mut word_buf = String::new();
            for ch in sp.content.chars() {
                word_buf.push(ch);
                n += 1;
                if n >= w {
                    // break at the last space if there is one in this chunk
                    if let Some(p) = word_buf.rfind(' ') {
                        let rest = word_buf.split_off(p + 1);
                        cur.push(Span::styled(word_buf.clone(), st));
                        out.push(Line::from(std::mem::take(&mut cur)));
                        word_buf = rest;
                        n = word_buf.chars().count();
                    } else {
                        cur.push(Span::styled(word_buf.clone(), st));
                        out.push(Line::from(std::mem::take(&mut cur)));
                        word_buf.clear();
                        n = 0;
                    }
                }
            }
            if !word_buf.is_empty() { cur.push(Span::styled(word_buf, st)); }
        }
        if !cur.is_empty() { out.push(Line::from(cur)); }
    }
    out
}

/// Markdown as Textual's Markdown widget shows it: headings bold and colored, bullets, and tables lined up.
pub fn markdown(text: &str) -> Vec<Line<'static>> {
    let mut out: Vec<Line<'static>> = vec![];
    let lines: Vec<&str> = text.lines().collect();
    let mut i = 0;
    let inline = |s: &str| -> Line<'static> {
        let mut spans = vec![];
        for (n, part) in s.split("**").enumerate() {
            if part.is_empty() { continue; }
            let part = part.replace('*', "");
            spans.push(if n % 2 == 1 { Span::styled(part, Style::default().fg(TEXT).add_modifier(Modifier::BOLD)) } else { Span::styled(part, Style::default().fg(TEXT)) });
        }
        Line::from(spans)
    };
    while i < lines.len() {
        let l = lines[i];
        if let Some(h) = l.strip_prefix("# ") {
            out.push(Line::from(Span::styled(h.replace("**", ""), Style::default().fg(ACCENT).add_modifier(Modifier::BOLD | Modifier::UNDERLINED))));
        } else if let Some(h) = l.strip_prefix("## ") {
            out.push(Line::from(Span::styled(h.replace("**", ""), Style::default().fg(ACCENT).add_modifier(Modifier::BOLD))));
        } else if let Some(h) = l.strip_prefix("### ") {
            out.push(Line::from(Span::styled(h.replace("**", ""), Style::default().fg(Color::Rgb(255, 200, 120)).add_modifier(Modifier::BOLD))));
        } else if l.starts_with('|') {
            // a table: gather it, size its columns, draw it lined up
            let mut rows: Vec<Vec<String>> = vec![];
            while i < lines.len() && lines[i].starts_with('|') {
                let cells: Vec<String> = lines[i].trim().trim_matches('|').split('|').map(|c| c.trim().replace("**", "")).collect();
                if !cells.iter().all(|c| c.chars().all(|ch| ch == '-' || ch == ':')) { rows.push(cells); }
                i += 1;
            }
            let ncol = rows.iter().map(|r| r.len()).max().unwrap_or(0);
            let widths: Vec<usize> = (0..ncol).map(|c| rows.iter().map(|r| r.get(c).map(|x| x.chars().count()).unwrap_or(0)).max().unwrap_or(0).min(40)).collect();
            for (n, r) in rows.iter().enumerate() {
                let cells: Vec<String> = (0..ncol).map(|c| { let v: String = r.get(c).cloned().unwrap_or_default().chars().take(40).collect(); format!("{v:<w$}", w = widths[c]) }).collect();
                let st = if n == 0 { Style::default().fg(ACCENT).bg(PANEL).add_modifier(Modifier::BOLD) } else if n % 2 == 0 { Style::default().fg(TEXT).bg(Color::Rgb(28, 28, 32)) } else { Style::default().fg(TEXT) };
                out.push(Line::from(Span::styled(cells.join("  "), st)));
            }
            continue;
        } else if let Some(b) = l.strip_prefix("- ") {
            let mut line = inline(b);
            line.spans.insert(0, Span::styled(" • ", Style::default().fg(ACCENT)));
            out.push(line);
        } else if let Some(b) = l.strip_prefix("  - ") {
            let mut line = inline(b);
            line.spans.insert(0, Span::styled("     ◦ ", Style::default().fg(ACCENT)));
            out.push(line);
        } else {
            out.push(inline(l));
        }
        i += 1;
    }
    out
}
