//! The search in the title bar, which is also the command palette
//! (Ctrl+K, or Ctrl+F for notes): every command, sheet and note, fuzzy
//! filtered as you type. Closed it's a quiet field; open, you type in it and
//! the results drop below, under tabs for each scope with how many each
//! finds. A first `>` keeps to commands, `/` to sheets, `@` to notes (where
//! `#tag` and `paper:sage` work too), and Tab moves between them. Arrows
//! move, Enter runs, Esc closes; with nothing found, Enter makes a note of
//! what you typed.

use gpui::component::input::Input;
use gpui::component::Sizable as _;
use gpui::{div, font, prelude::*, px, AnimationExt as _, Context, CursorStyle, FontWeight, MouseButton, SharedString, StyledText, TextRun, Window};

use super::app::{BinTab, Folio, Screen};
use super::home::first_line;
use super::*;
use crate::model::Id;
use crate::theme::{theme, Theme};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Cmd {
    NewNote,
    NewSheet,
    QuickNote,
    Home,
    Drawer,
    Settings,
    Trash,
    Archive,
    Backups,
    BackUpNow,
    Theme,
    Fit,
    Tidy,
    Map,
    Profile,
    Intro,
    Back,
    Forward,
    Drive,
    Transfer,
    Import,
    Rename,
}

const CMDS: [(Cmd, &str, &str, &str); 22] = [
    (Cmd::NewNote, "New note", "Ctrl N", "icons/sticky-note.svg"),
    (Cmd::NewSheet, "New sheet", "Ctrl Shift N", "icons/plus.svg"),
    (Cmd::QuickNote, "Quick note to Inbox", "Ctrl Alt N", "icons/inbox.svg"),
    (Cmd::Rename, "Rename this sheet", "", "icons/pencil.svg"),
    (Cmd::Home, "Go to all sheets", "Ctrl Shift H", "icons/layout-grid.svg"),
    (Cmd::Drawer, "Show the sheets drawer", "Ctrl B", "icons/panel-left.svg"),
    (Cmd::Transfer, "Sync & export: Google Drive, files", "", "icons/cloud.svg"),
    (Cmd::Drive, "Sync with Google Drive", "", "icons/refresh-cw.svg"),
    (Cmd::Import, "Import a Folio file…", "", "icons/upload.svg"),
    (Cmd::Settings, "Settings", "Ctrl ,", "icons/settings.svg"),
    (Cmd::Trash, "Open the Trash", "", "icons/trash.svg"),
    (Cmd::Archive, "Open the Archive", "", "icons/archive.svg"),
    (Cmd::Backups, "Restore a backup", "", "icons/rotate-ccw.svg"),
    (Cmd::BackUpNow, "Back up now", "", "icons/hard-drive-download.svg"),
    (Cmd::Theme, "Switch light / dark", "Ctrl Shift L", "icons/moon.svg"),
    (Cmd::Fit, "Zoom to fit", "Ctrl 0", "icons/scan.svg"),
    (Cmd::Tidy, "Tidy up the sheet", "", "icons/wand-sparkles.svg"),
    (Cmd::Map, "Show / hide the map", "", "icons/map.svg"),
    (Cmd::Profile, "Change your name and avatar", "", "icons/user-round-pen.svg"),
    (Cmd::Intro, "Play the opening again", "", "icons/play.svg"),
    (Cmd::Back, "Go back", "Alt Left", "icons/arrow-left.svg"),
    (Cmd::Forward, "Go forward", "Alt Right", "icons/arrow-right.svg"),
];

/// What a query looks through, from its first character: the tabs.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Scope {
    All,
    Commands,
    Sheets,
    Notes,
}

impl Scope {
    const TABS: [Scope; 4] = [Scope::All, Scope::Commands, Scope::Sheets, Scope::Notes];

    fn prefix(self) -> &'static str {
        match self {
            Scope::All => "",
            Scope::Commands => ">",
            Scope::Sheets => "/",
            Scope::Notes => "@",
        }
    }

    fn label(self) -> &'static str {
        match self {
            Scope::All => "All",
            Scope::Commands => "Commands",
            Scope::Sheets => "Sheets",
            Scope::Notes => "Notes",
        }
    }

    fn icon(self) -> Option<&'static str> {
        match self {
            Scope::All => None,
            Scope::Commands => Some("icons/zap.svg"),
            Scope::Sheets => Some("icons/layers.svg"),
            Scope::Notes => Some("icons/sticky-note.svg"),
        }
    }

    fn of(group: &str) -> Scope {
        match group {
            "Commands" => Scope::Commands,
            "Sheets" => Scope::Sheets,
            "Notes" => Scope::Notes,
            _ => Scope::All,
        }
    }

    fn index(self) -> usize {
        Scope::TABS.iter().position(|s| *s == self).unwrap_or(0)
    }
}

/// The scope a query asks for, and its words.
fn scope(query: &str) -> (Scope, &str) {
    match query.chars().next() {
        Some('>') => (Scope::Commands, &query[1..]),
        Some('/') => (Scope::Sheets, &query[1..]),
        Some('@') => (Scope::Notes, &query[1..]),
        _ => (Scope::All, query),
    }
}

/// In All, each group shows this many rows; its tab shows the rest.
const PER_GROUP: usize = 4;
const MAX_RECENT: usize = 5;
const MAX_NOTES: usize = 40;

#[derive(Clone, Debug, PartialEq)]
enum Target {
    Cmd(Cmd),
    Sheet(Id),
    Note(Id),
    /// Nothing matched: a note holding the query.
    NoteOf(String),
    /// Nothing matched: a sheet named by it.
    SheetOf(String),
}

#[derive(Clone)]
struct Row {
    label: SharedString,
    group: &'static str,
    icon: &'static str,
    hint: Option<SharedString>,
    /// A quieter second part: the note's sheet, a snippet.
    detail: Option<SharedString>,
    /// Shown in the icon's place: a sheet's emoji.
    emoji: Option<SharedString>,
    target: Target,
}

struct Found {
    scope: Scope,
    typed: bool,
    rows: Vec<(Row, Vec<usize>)>,
    counts: [usize; 4],
    /// Groups All shows cut short, with how many each has.
    cut: Vec<(&'static str, usize)>,
}

// How well a label matches, best first. Each tier beats every one below
// it, whatever the label's length; within one, shorter labels win.
const EXACT: i32 = 50_000;
const PREFIX: i32 = 40_000;
const WORD: i32 = 30_000;
const WORDS: i32 = 20_000;
const INSIDE: i32 = 10_000;
/// A note whose text (not its first line) holds the query.
const IN_TEXT: i32 = 9_000;

fn fold(c: char) -> char {
    c.to_lowercase().next().unwrap_or(c)
}

/// Where `run` sits in `chars`, best place first: at a word's start, else
/// the first place it's anywhere.
fn find_run(chars: &[(usize, char)], run: &[char], starts: &dyn Fn(usize) -> bool) -> Option<usize> {
    if run.is_empty() || run.len() > chars.len() {
        return None;
    }
    let at = |k: usize| chars[k..k + run.len()].iter().map(|c| c.1).eq(run.iter().copied());
    let all = || (0..=chars.len() - run.len()).filter(|&k| at(k));
    all().find(|&k| starts(k)).or_else(|| all().next())
}

/// Matches `query` against `text`. Higher is better: the whole label, then
/// its start, then a word in it starting with what you typed, then every
/// word you typed starting one of its words, then the query anywhere in a
/// row, then its letters in order but scattered (where word starts and
/// runs score more). Returns the matched byte offsets.
pub fn fuzzy(query: &str, text: &str) -> Option<(i32, Vec<usize>)> {
    let words: Vec<Vec<char>> = query.split_whitespace().map(|w| w.chars().map(fold).collect()).collect();
    if words.is_empty() {
        return Some((0, vec![]));
    }
    let chars: Vec<(usize, char)> = text.char_indices().map(|(i, c)| (i, fold(c))).collect();
    let starts = |k: usize| k == 0 || !chars[k - 1].1.is_alphanumeric();
    let shorter = -(text.len().min(4_000) as i32) / 4;
    let span = |k: usize, n: usize| chars[k..k + n].iter().map(|c| c.0).collect::<Vec<_>>();

    // What you typed, in a row.
    let phrase: Vec<char> = words.join(&' ');
    if let Some(k) = find_run(&chars, &phrase, &starts) {
        let tier = match k {
            0 if phrase.len() == chars.len() => EXACT,
            0 => PREFIX,
            _ if starts(k) => WORD,
            _ => INSIDE,
        };
        return Some((tier + shorter, span(k, phrase.len())));
    }
    // Every word you typed starts a word of the label, in any order.
    if words.len() > 1 {
        let mut hits = vec![];
        for w in &words {
            let k = (0..chars.len()).find(|&k| starts(k) && k + w.len() <= chars.len() && chars[k..k + w.len()].iter().map(|c| c.1).eq(w.iter().copied()))?;
            hits.extend(span(k, w.len()));
        }
        hits.sort_unstable();
        hits.dedup();
        return Some((WORDS + shorter, hits));
    }
    // The letters in order, scattered.
    let mut positions = vec![];
    let mut score = 0;
    let mut at = 0;
    let mut prev: Option<usize> = None;
    for q in phrase.iter().copied().filter(|c| !c.is_whitespace()) {
        let found = (at..chars.len()).find(|&k| chars[k].1 == q)?;
        score += 1 + if starts(found) { 8 } else { 0 } + if prev == Some(found.wrapping_sub(1)) { 5 } else { 0 };
        positions.push(chars[found].0);
        prev = Some(found);
        at = found + 1;
    }
    Some(((score * 10).min(INSIDE / 2) + shorter, positions))
}

/// A note's first line as it reads: no Markdown marks, link targets or
/// highlight tags.
fn plain(line: &str) -> String {
    let mut s = line.to_string();
    // <mark color="…">word</mark> → word
    while let (Some(a), Some(b)) = (s.find("<mark"), s.find('>')) {
        if b <= a {
            break;
        }
        s.replace_range(a..=b, "");
    }
    s = s.replace("</mark>", "");
    // [text](url) → text
    while let Some(a) = s.find("](") {
        let Some(close) = s[a..].find(')') else { break };
        s.replace_range(a..a + close + 1, "");
        if let Some(open) = s[..a].rfind('[') {
            s.remove(open);
        }
    }
    let s = s.trim_start_matches(|c| c == '#' || c == '>' || c == ' ');
    let s = s.strip_prefix("- [ ] ").or_else(|| s.strip_prefix("- [x] ")).or_else(|| s.strip_prefix("- ")).unwrap_or(s);
    let out: String = s.replace("**", "").replace("__", "").replace('`', "").replace("~~", "");
    let out = out.split_whitespace().collect::<Vec<_>>().join(" ");
    if out.is_empty() {
        "Untitled note".into()
    } else {
        out
    }
}

/// Where `needle` sits in `text` (any case), as byte offsets to highlight.
fn substring_hits(text: &str, needle: &str) -> Vec<usize> {
    let lower = text.to_lowercase();
    if needle.is_empty() || lower.len() != text.len() {
        return vec![];
    }
    lower.find(needle).map(|at| text[at..at + needle.len()].char_indices().map(|(i, _)| at + i).collect()).unwrap_or_default()
}

/// A label with its matched letters picked out.
fn highlighted(label: &SharedString, positions: &[usize], on: bool, t: &Theme) -> StyledText {
    let mut runs = vec![];
    for (start, ch) in label.char_indices() {
        let hit = positions.contains(&start);
        let mut f = font(t.font);
        if hit {
            f.weight = FontWeight::SEMIBOLD;
        }
        runs.push(TextRun {
            len: ch.len_utf8(),
            font: f,
            color: if hit { t.colors.brand_on_tint } else if on { t.colors.text } else { t.colors.text_body },
            background_color: None,
            underline: None,
            strikethrough: None,
        });
    }
    StyledText::new(label.clone()).with_runs(runs)
}

/// A shortcut as key caps: "Ctrl Shift N" is three.
fn keys(t: &Theme, text: &str) -> gpui::Div {
    div().flex().flex_none().gap(px(3.)).children(text.split_whitespace().map(|k| {
        let k = if cfg!(target_os = "macos") && k == "Ctrl" { "⌘" } else { k };
        div()
            .flex()
            .items_center()
            .h(px(18.))
            .px(px(5.))
            .rounded(px(4.))
            .border_1()
            .border_b_2()
            .border_color(t.colors.border)
            .bg(t.colors.surface_sunk)
            .text_size(t.text.micro)
            .font_weight(FontWeight::MEDIUM)
            .text_color(t.colors.text_muted)
            .child(k.to_string())
    }))
}

impl Folio {
    /// Opens the search with `prefix` typed (a scope, or a query).
    pub fn open_palette_with(&mut self, prefix: &str, window: &mut Window, cx: &mut Context<Self>) {
        self.finish_edit(cx);
        self.menu = None;
        self.drawer = false;
        self.palette = true;
        self.palette_ix = 0;
        self.palette_opened += 1;
        let value = prefix.to_string();
        self.palette_input.update(cx, |i, cx| {
            i.set_value(value, window, cx);
            i.focus(window, cx);
        });
        cx.notify();
    }

    pub fn open_palette(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.palette {
            self.close_palette(window, cx);
        } else {
            self.open_palette_with("", window, cx);
        }
    }

    pub fn close_palette(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.palette = false;
        window.focus(&self.focus, cx);
        cx.notify();
    }

    fn rows_all(&self) -> Vec<Row> {
        let mut out: Vec<Row> = CMDS
            .iter()
            .filter(|(c, ..)| *c != Cmd::Rename || self.screen == Screen::Sheet)
            .map(|&(c, label, key, path)| Row {
                label: label.into(),
                group: "Commands",
                icon: path,
                // The quick note's keys are the ones chosen in Settings.
                hint: match c {
                    Cmd::QuickNote => self.board.settings.quick_hotkey.then(|| self.board.settings.quick_keys.replace('+', " ").into()),
                    _ => (!key.is_empty()).then(|| key.into()),
                },
                detail: None,
                emoji: None,
                target: Target::Cmd(c),
            })
            .collect();
        for ix in self.board.ordered() {
            let c = &self.board.categories[ix];
            let n = c.stickers.len();
            out.push(Row {
                label: c.name.clone().into(),
                group: "Sheets",
                icon: "icons/layers.svg",
                hint: None,
                detail: Some(format!("{n} note{}", if n == 1 { "" } else { "s" }).into()),
                emoji: (!c.icon.is_empty()).then(|| c.icon.clone().into()),
                target: Target::Sheet(c.id),
            });
        }
        out
    }

    /// Notes for the query: by tag or colour with `#`/`paper:`, else by
    /// their first line (fuzzy) or anywhere in their text.
    fn note_rows(&self, q: &str) -> Vec<(i32, Row, Vec<usize>)> {
        let mut out = Vec::new();
        let ql = q.to_lowercase();
        let semantic = ql.starts_with('#') || ql.starts_with("paper:");
        let mut notes: Vec<(&crate::model::Category, &crate::model::Sticker)> =
            self.board.categories.iter().flat_map(|c| c.stickers.iter().map(move |s| (c, s))).collect();
        if q.is_empty() {
            // Nothing typed: the latest edited first.
            notes.sort_by(|a, b| b.1.edited.cmp(&a.1.edited));
        }
        // Every note is scored before any is cut, so the best are never
        // left out for coming late.
        let mut hits = Vec::new();
        for (c, s) in notes {
            let line = plain(&first_line(s));
            let found = if q.is_empty() {
                Some((0, vec![]))
            } else if semantic {
                s.matches(&ql).then(|| (1, vec![]))
            } else {
                fuzzy(q, &line).or_else(|| s.matches(&ql).then(|| (IN_TEXT, substring_hits(&line, &ql))))
            };
            if let Some((score, pos)) = found {
                hits.push((score, c, s, line, pos));
                if q.is_empty() && hits.len() >= MAX_NOTES {
                    break;
                }
            }
        }
        // Stable: equal scores keep the sheets' order.
        hits.sort_by(|a, b| b.0.cmp(&a.0));
        hits.truncate(MAX_NOTES);
        for (score, c, s, line, pos) in hits {
            let snippet = (!q.is_empty() && !semantic && pos.is_empty()).then(|| super::home::snippet_of(s, &ql)).filter(|x| !x.is_empty());
            let detail = match snippet {
                Some(sn) => format!("{} · {sn}", c.name),
                None => c.name.clone(),
            };
            out.push((
                score,
                Row { label: line.into(), group: "Notes", icon: "icons/sticky-note.svg", hint: None, detail: Some(detail.into()), emoji: None, target: Target::Note(s.id) },
                pos,
            ));
        }
        out
    }

    fn search(&self, cx: &gpui::App) -> Found {
        let raw = self.palette_input.read(cx).value().to_string();
        let (scope, q) = scope(&raw);
        let q = q.trim();
        let mut scored: Vec<(i32, Row, Vec<usize>)> =
            self.rows_all().into_iter().filter_map(|r| fuzzy(q, &r.label).map(|(s, p)| (s, r, p))).collect();
        if !q.is_empty() || scope == Scope::Notes {
            scored.extend(self.note_rows(q));
        }
        let mut counts = [0; 4];
        for (_, r, _) in &scored {
            counts[Scope::of(r.group).index()] += 1;
        }
        counts[0] = counts.iter().sum();
        scored.retain(|(_, r, _)| scope == Scope::All || Scope::of(r.group) == scope);

        if !q.is_empty() {
            // Groups together, best group first; within one, best first.
            let mut best: std::collections::HashMap<&'static str, i32> = Default::default();
            for (score, r, _) in &scored {
                let b = best.entry(r.group).or_insert(i32::MIN);
                *b = (*b).max(*score);
            }
            scored.sort_by(|a, b| best[b.1.group].cmp(&best[a.1.group]).then(a.1.group.cmp(b.1.group)).then(b.0.cmp(&a.0)));
        } else if scope == Scope::All && !self.palette_recent.is_empty() {
            // Nothing typed: what you ran lately comes first.
            let mut recent = vec![];
            for label in &self.palette_recent {
                if let Some(i) = scored.iter().position(|(_, r, _)| &r.label == label) {
                    let (s, mut r, p) = scored.remove(i);
                    r.group = "Recently used";
                    recent.push((s, r, p));
                }
            }
            recent.append(&mut scored);
            scored = recent;
        }

        let mut cut = vec![];
        if scope == Scope::All {
            let mut seen: Vec<(&'static str, usize)> = vec![];
            scored.retain(|(_, r, _)| {
                if r.group == "Recently used" {
                    return true;
                }
                match seen.iter_mut().find(|(g, _)| *g == r.group) {
                    Some((_, n)) => {
                        *n += 1;
                        *n <= PER_GROUP
                    }
                    None => {
                        seen.push((r.group, 1));
                        true
                    }
                }
            });
            cut = seen.into_iter().filter(|(_, n)| *n > PER_GROUP).collect();
        }

        if scored.is_empty() && !q.is_empty() && matches!(scope, Scope::All | Scope::Notes | Scope::Sheets) {
            let in_sheet = if self.screen == Screen::Sheet { self.sheet().name.clone() } else { "your Inbox".into() };
            if scope != Scope::Sheets {
                scored.push((
                    0,
                    Row {
                        label: format!("New note: {q}").into(),
                        group: "Nothing matches",
                        icon: "icons/sticky-note.svg",
                        hint: Some("Enter".into()),
                        detail: Some(format!("in {in_sheet}").into()),
                        emoji: None,
                        target: Target::NoteOf(q.to_string()),
                    },
                    vec![],
                ));
            }
            scored.push((
                0,
                Row {
                    label: format!("New sheet: {q}").into(),
                    group: "Nothing matches",
                    icon: "icons/layers.svg",
                    hint: None,
                    detail: None,
                    emoji: None,
                    target: Target::SheetOf(q.to_string()),
                },
                vec![],
            ));
        }
        Found { scope, typed: !q.is_empty(), rows: scored.into_iter().map(|(_, r, p)| (r, p)).take(80).collect(), counts, cut }
    }

    /// Switch tab: the query keeps its words under the new scope's prefix.
    fn set_scope(&mut self, to: Scope, window: &mut Window, cx: &mut Context<Self>) {
        let raw = self.palette_input.read(cx).value().to_string();
        let words = scope(&raw).1.to_string();
        let value = format!("{}{}", to.prefix(), words);
        self.palette_input.update(cx, |i, cx| {
            i.set_value(value, window, cx);
            i.focus(window, cx);
        });
        self.palette_ix = 0;
        cx.notify();
    }

    pub fn palette_step(&mut self, by: isize, cx: &mut Context<Self>) {
        let n = self.search(cx).rows.len();
        if n > 0 {
            self.palette_ix = (self.palette_ix as isize + by).rem_euclid(n as isize) as usize;
            self.palette_reveal = true;
        }
        cx.notify();
    }

    pub fn palette_tab(&mut self, forward: bool, window: &mut Window, cx: &mut Context<Self>) {
        let raw = self.palette_input.read(cx).value().to_string();
        let at = scope(&raw).0.index();
        let n = Scope::TABS.len();
        let to = if forward { (at + 1) % n } else { (at + n - 1) % n };
        self.set_scope(Scope::TABS[to], window, cx);
    }

    pub fn palette_run(&mut self, ix: usize, window: &mut Window, cx: &mut Context<Self>) {
        let mut rows = self.search(cx).rows;
        if rows.is_empty() {
            return;
        }
        let (row, _) = rows.swap_remove(ix.min(rows.len() - 1));
        if matches!(row.target, Target::Cmd(_) | Target::Sheet(_)) {
            self.palette_recent.retain(|l| *l != row.label);
            self.palette_recent.insert(0, row.label.clone());
            self.palette_recent.truncate(MAX_RECENT);
        }
        self.palette = false;
        window.focus(&self.focus, cx);
        match row.target {
            Target::Sheet(id) => {
                if let Some(i) = self.board.categories.iter().position(|c| c.id == id) {
                    self.open_sheet(i, window, cx);
                }
            }
            Target::Note(id) => self.reveal_note(id, window, cx),
            Target::NoteOf(text) => {
                if self.screen == Screen::Sheet {
                    let (w, h) = self.viewport_size();
                    let at = self.sheet().camera.to_world(crate::model::v2(w / 2., h / 2.));
                    self.new_note_at(at, window, cx);
                    let (title, body) = match text.split_once('\n') {
                        Some((a, b)) => (a.to_string(), b.to_string()),
                        None => (text.clone(), String::new()),
                    };
                    self.title_input.update(cx, |i, cx| i.set_value(title, window, cx));
                    self.body_input.update(cx, |i, cx| i.set_value(body, window, cx));
                } else {
                    self.quick_note(&text, cx);
                }
            }
            Target::SheetOf(name) => {
                let n = self.board.categories.len();
                self.change(cx, |b| {
                    b.add_sheet(&name);
                    b.sheet_mut().icon = super::emoji::default_icon(n).into();
                });
                let ix = self.board.active;
                self.open_sheet(ix, window, cx);
            }
            Target::Cmd(c) => match c {
                Cmd::NewNote => self.new_note(window, cx),
                Cmd::NewSheet => self.new_sheet_dialog(window, cx),
                Cmd::QuickNote => {
                    // Opened once this update is over: it reads Folio.
                    let f = cx.entity();
                    cx.defer(move |cx| crate::desktop::open_quick_note(f, cx));
                }
                Cmd::Rename => self.start_naming(window, cx),
                Cmd::Home => self.go_home(cx),
                Cmd::Drawer => self.drawer = true,
                Cmd::Settings => self.open_settings(cx),
                Cmd::Trash => self.open_bin(BinTab::Trash, cx),
                Cmd::Archive => self.open_bin(BinTab::Archive, cx),
                Cmd::Backups => self.open_bin(BinTab::Backups, cx),
                Cmd::BackUpNow => self.back_up_now(cx),
                Cmd::Drive if self.sync.signed_in() => {
                    if self.sync.state == super::sync::DriveState::Choice {
                        self.drive_show_choice(cx)
                    } else {
                        self.drive_sync(None, cx)
                    }
                }
                Cmd::Drive => self.sign_in_google(window, cx),
                Cmd::Transfer => self.open_transfer(cx),
                Cmd::Import => self.import_file(cx),
                Cmd::Theme => self.toggle_theme(cx),
                Cmd::Fit | Cmd::Tidy | Cmd::Map => {
                    if self.screen != Screen::Sheet {
                        let a = self.board.active;
                        self.open_sheet(a, window, cx);
                    }
                    match c {
                        Cmd::Fit => self.zoom_to_fit(cx),
                        Cmd::Tidy => self.tidy(cx),
                        _ => self.set_setting(cx, |s| s.minimap = !s.minimap),
                    }
                }
                Cmd::Profile => self.open_welcome(false, window, cx),
                Cmd::Intro => self.intro = Some(super::app::Intro { start: std::time::Instant::now(), word: Default::default() }),
                Cmd::Back => self.go_back(window, cx),
                Cmd::Forward => self.go_forward(window, cx),
            },
        }
        cx.notify();
    }

    /// The title bar's search field. Closed, it says what it searches and
    /// opens on a click; open, you type in it and the results drop below.
    pub(super) fn palette_field(&mut self, window: &mut Window, cx: &mut Context<Self>) -> gpui::AnyElement {
        let t = theme(cx).clone();
        let open = self.palette;
        // A pill, quiet while closed; it widens and lights up as you type.
        let field = div()
            .id("palette-field")
            .occlude()
            .relative()
            .flex()
            .items_center()
            .gap(t.space.sm)
            .h(t.size.bar_control)
            .pl(px(10.))
            .pr(px(4.))
            .rounded(t.radius.full)
            .border_1()
            .child(icon("icons/search.svg", t.size.icon_sm, if open { t.colors.brand } else { t.colors.text_subtle }));
        if !open {
            let (edge, ink) = (t.colors.border, t.colors.text_muted);
            let lift = t.shadow_raised();
            // Just closed: the results sink away as the field narrows.
            let going = self.exits.palette.leaving(t.motion.search_out).map(|(_, n)| n);
            let gone = going.map(|n| self.palette_dropdown(Some(n), window, cx));
            let field = field
                .w(t.size.search)
                .bg(t.colors.surface_sunk)
                .border_color(t.colors.border)
                .cursor(CursorStyle::IBeam)
                .text_color(t.colors.text_subtle)
                .hover(move |d| d.border_color(edge).text_color(ink).bg(t.colors.surface).shadow(lift))
                .child(div().flex_1().min_w_0().truncate().text_size(t.text.caption).child("Search sheets, notes, commands"))
                .tooltip(|window, cx| super::tip(format!("Search  {}", shortcut("K")), window, cx))
                .on_click(cx.listener(|this, _, window, cx| this.open_palette_with("", window, cx)))
                .children(gone.map(|d| gpui::deferred(d).with_priority(2)));
            let (from, to) = (f32::from(t.size.search_open), f32::from(t.size.search));
            return match going {
                Some(n) => field
                    .with_animation(("search-narrow", n), gpui::Animation::new(std::time::Duration::from_millis(motion::ms(t.motion.search_out))), move |d, k| {
                        d.w(px(from + (to - from) * gpui::ease_out_quint()(k)))
                    })
                    .into_any_element(),
                None => field.into_any_element(),
            };
        }
        let dropdown = self.palette_dropdown(None, window, cx);
        let opened = self.palette_opened;
        let (from, to) = (f32::from(t.size.search), f32::from(t.size.search_open));
        let ms = motion::ms(t.motion.search_in);
        field
            .w(t.size.search_open)
            .bg(t.colors.surface)
            .border_color(t.colors.brand)
            .shadow(vec![gpui::BoxShadow {
                color: t.colors.brand.opacity(0.18),
                offset: gpui::point(px(0.), px(0.)),
                blur_radius: px(0.),
                spread_radius: px(3.),
                inset: false,
            }])
            .key_context("Palette")
            .on_action(cx.listener(|this, _: &PaletteUp, _, cx| this.palette_step(-1, cx)))
            .on_action(cx.listener(|this, _: &PaletteDown, _, cx| this.palette_step(1, cx)))
            .on_action(cx.listener(|this, _: &PaletteTab, window, cx| this.palette_tab(true, window, cx)))
            .on_action(cx.listener(|this, _: &PaletteBackTab, window, cx| this.palette_tab(false, window, cx)))
            .text_size(t.text.body)
            .child(div().flex_1().min_w_0().child(Input::new(&self.palette_input).appearance(false).small()))
            .child(keys(&t, "Esc"))
            .child(gpui::deferred(dropdown).with_priority(2))
            // Opening, the field widens to make room for what you type.
            .with_animation(("search-widen", opened), gpui::Animation::new(std::time::Duration::from_millis(ms)).with_easing(gpui::ease_out_quint()), move |d, k| {
                d.w(px(from + (to - from) * k))
            })
            .into_any_element()
    }

    /// The tabs and the results, hung below the field. They rise into
    /// place as the search opens; `leaving` (a number for its animation)
    /// draws them sinking away after it closed, unclickable.
    fn palette_dropdown(&mut self, leaving: Option<usize>, _: &mut Window, cx: &mut Context<Self>) -> gpui::AnyElement {
        let t = theme(cx).clone();
        let found = self.search(cx);
        let count = found.rows.len();
        let selected = self.palette_ix.min(count.saturating_sub(1));
        let opened = self.palette_opened;

        let mut tabs = div().flex().items_end().gap(px(2.)).px(t.space.sm).pt(t.space.xs).border_b_1().border_color(t.colors.border);
        for (i, tab) in Scope::TABS.into_iter().enumerate() {
            let on = tab == found.scope;
            tabs = tabs.child(
                div()
                    .id(("palette-tab", i))
                    .flex()
                    .items_center()
                    .gap(t.space.xs)
                    .px(t.space.md)
                    .py(t.space.sm)
                    .border_b_2()
                    .border_color(if on { t.colors.brand } else { gpui::transparent_black() })
                    .text_size(t.text.caption)
                    .cursor_pointer()
                    .map(|d| {
                        if on {
                            d.font_weight(FontWeight::SEMIBOLD).text_color(t.colors.text)
                        } else {
                            let ink = t.colors.text;
                            d.text_color(t.colors.text_muted).hover(move |s| s.text_color(ink))
                        }
                    })
                    .children(tab.icon().map(|p| icon(p, px(12.), if on { t.colors.brand } else { t.colors.text_subtle })))
                    .child(tab.label())
                    .when(found.typed, |d| {
                        d.child(
                            div()
                                .font_weight(FontWeight::NORMAL)
                                .text_color(if on { t.colors.brand_on_tint } else { t.colors.text_subtle })
                                .child(found.counts[i].to_string()),
                        )
                    })
                    .on_click(cx.listener(move |this, _, window, cx| this.set_scope(tab, window, cx))),
            );
        }

        let mut list = div()
            .id("palette-list")
            .track_scroll(&self.palette_scroll)
            .overflow_y_scroll()
            .flex()
            .flex_col()
            .max_h(px(400.))
            .p(t.space.xs);
        let mut last_group = "";
        // The list's children count group labels too.
        let mut child = 0;
        let mut selected_child = 0;
        for (i, (row, pos)) in found.rows.iter().enumerate() {
            if row.group != last_group {
                last_group = row.group;
                child += 1;
                let more = found.cut.iter().find(|(g, _)| *g == row.group).map(|(_, n)| *n);
                let to = Scope::of(row.group);
                list = list.child(
                    div()
                        .flex()
                        .items_center()
                        .justify_between()
                        .px(t.space.md)
                        .pt(t.space.sm)
                        .pb(t.space.xs)
                        .text_size(t.text.micro)
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(t.colors.text_subtle)
                        .child(row.group.to_uppercase())
                        .children(more.map(|n| {
                            let ink = t.colors.text;
                            div()
                                .id(("palette-more", i))
                                .cursor_pointer()
                                .font_weight(FontWeight::MEDIUM)
                                .text_color(t.colors.brand_on_tint)
                                .hover(move |s| s.text_color(ink))
                                .child(format!("All {n} ›"))
                                .on_click(cx.listener(move |this, _, window, cx| this.set_scope(to, window, cx)))
                        })),
                );
            }
            let on = i == selected;
            if on {
                selected_child = child;
            }
            child += 1;
            let row_el = div()
                .id(("palette-row", i))
                .flex()
                .items_center()
                .gap(t.space.md)
                .min_h(px(36.))
                .py(px(6.))
                .px(t.space.md)
                .rounded(t.radius.button)
                .cursor_pointer()
                .map(|d| if on { d.bg(t.colors.brand_tint) } else { d.hover(|s| s.bg(t.colors.hover)) })
                .child(
                    div()
                        .size(px(24.))
                        .flex_none()
                        .flex()
                        .items_center()
                        .justify_center()
                        .rounded(t.radius.small)
                        .bg(if on { t.colors.surface } else { t.colors.surface_sunk })
                        .border_1()
                        .border_color(t.colors.border)
                        .map(|d| match row.emoji.clone() {
                            Some(e) => d.text_size(px(13.)).child(e),
                            None => d.child(icon(row.icon, px(13.), if on { t.colors.brand } else { t.colors.text_muted })),
                        }),
                )
                // The whole label, wrapping if it must; what it belongs to
                // underneath, never cut short mid-thought.
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .flex()
                        .flex_col()
                        .gap(px(1.))
                        .child(div().text_size(t.text.body).line_height(px(18.)).child(highlighted(&row.label, pos, on, &t)))
                        .children(row.detail.clone().map(|d| {
                            div().text_size(t.text.caption).line_height(px(16.)).text_color(if on { t.colors.text_muted } else { t.colors.text_subtle }).child(d)
                        })),
                )
                .children(row.hint.as_ref().map(|h| keys(&t, h)))
                .when(on, |d| d.child(icon("icons/corner-down-left.svg", px(12.), t.colors.brand_on_tint)))
                .on_mouse_move(cx.listener(move |this, _, _, cx| {
                    if this.palette_ix != i {
                        this.palette_ix = i;
                        cx.notify();
                    }
                }))
                .on_click(cx.listener(move |this, _, window, cx| this.palette_run(i, window, cx)));
            list = list.child(motion::row_in(row_el, ("palette-in", opened * 1000 + i), i, &t));
        }
        if std::mem::take(&mut self.palette_reveal) {
            self.palette_scroll.scroll_to_item(selected_child);
        }
        if count == 0 {
            list = list.child(div().px(t.space.md).py(t.space.lg).text_color(t.colors.text_subtle).child(if found.scope == Scope::Notes {
                "No notes yet: double-click a sheet to write one."
            } else {
                "Nothing matches."
            }));
        }

        let panel = div()
            .id("palette")
            .relative()
            .occlude()
            .flex()
            .flex_col()
            .overflow_hidden()
            .rounded(t.radius.card)
            .bg(t.colors.surface)
            .border_1()
            .border_color(t.colors.border)
            .shadow(t.shadow_float())
            .cursor(CursorStyle::Arrow)
            .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
            .child(tabs)
            .child(list);
        div()
            .absolute()
            .top(t.size.bar_control + px(8.))
            // Wider than the field, centred under it, so nothing gets cut.
            .left(px(-70.))
            .right(px(-70.))
            .child(match leaving {
                Some(n) => motion::leave(panel, ("palette-out", n), t.motion.search_out, t.motion.sink),
                None => motion::enter(panel, ("palette-open", opened), t.motion.search_in, 6.),
            })
            .into_any_element()
    }

    /// The window brightening again after the search closed.
    pub(super) fn render_palette_scrim_leaving(&mut self, cx: &mut Context<Self>) -> Option<gpui::AnyElement> {
        let t = theme(cx).clone();
        let (_, n) = self.exits.palette.leaving(t.motion.search_out)?;
        Some(motion::scrim_out(
            div().absolute().top(t.size.topbar).left_0().right_0().bottom_0(),
            ("palette-scrim-out", n),
            t.colors.scrim,
            t.motion.search_out,
        ))
    }

    /// Behind the open search: the window under the title bar, dimmed. A
    /// click on it closes the search.
    pub(super) fn render_palette_scrim(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let t = theme(cx).clone();
        motion::scrim(
            div()
                .id("palette-scrim")
                .absolute()
                .top(t.size.topbar)
                .left_0()
                .right_0()
                .bottom_0()
                .on_mouse_down(MouseButton::Left, cx.listener(|this, _, window, cx| this.close_palette(window, cx))),
            ("palette-scrim", self.palette_opened),
            t.colors.scrim,
            &t,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fuzzy_matches_in_order_and_prefers_word_starts() {
        assert!(fuzzy("ns", "New sheet").is_some());
        assert!(fuzzy("sn", "New sheet").is_none());
        let (a, _) = fuzzy("set", "Settings").unwrap();
        let (b, _) = fuzzy("set", "Reset the theme").unwrap();
        assert!(a > b, "a word start and a run beat letters scattered");
        assert_eq!(fuzzy("bk", "Back up now").unwrap().1, vec![0, 3]);
    }

    #[test]
    fn whole_words_and_starts_beat_scattered_letters() {
        let score = |q: &str, t: &str| fuzzy(q, t).unwrap().0;
        // The palette once put "Sync & export" above this note for "seed".
        assert!(score("seed", "Seeds to order") > score("seed", "Sync & export: Google Drive"));
        assert!(score("seed", "Show the sheets drawer") < score("seed", "Order the seeds"));
        assert!(score("milk", "Milk") > score("milk", "Milk and eggs"), "the whole label first");
        assert!(score("milk", "Milk and eggs") > score("milk", "Buy milk"), "then its start");
        assert!(score("milk", "Buy milk") > score("milk", "Buttermilk"), "then a word's start");
        assert!(score("eggs milk", "Milk and eggs") > score("ilk", "Buttermilk"), "words in any order");
        assert_eq!(fuzzy("eggs milk", "Milk and eggs").unwrap().1, vec![0, 1, 2, 3, 9, 10, 11, 12]);
        assert!(fuzzy("zz top", "Milk and eggs").is_none());
        // Even a long label's run beats a short one's scattered letters.
        assert!(score("plan", &format!("The plan {}", "x".repeat(400))) > score("plan", "P l a n"));
        assert_eq!(fuzzy("MILK", "Buy milk").unwrap().1, vec![4, 5, 6, 7], "any case");
    }

    #[test]
    fn a_first_character_picks_the_scope() {
        assert_eq!(scope(">sync"), (Scope::Commands, "sync"));
        assert_eq!(scope("/trip"), (Scope::Sheets, "trip"));
        assert_eq!(scope("@#design"), (Scope::Notes, "#design"));
        assert_eq!(scope("#design"), (Scope::All, "#design"), "a tag isn't a scope");
        for s in Scope::TABS {
            assert_eq!(scope(&format!("{}x", s.prefix())).0, s);
        }
    }

    #[test]
    fn note_lines_read_as_plain_text() {
        assert_eq!(plain("Functional testing** = tests **what** it does"), "Functional testing = tests what it does");
        assert_eq!(plain("## Plan for `v2`"), "Plan for v2");
        assert_eq!(plain("- [ ] Call [Sam](https://x.y) <mark color=\"yellow\">today</mark>"), "Call Sam today");
        assert_eq!(plain("   "), "Untitled note");
    }

    #[test]
    fn a_plain_word_lights_up_where_it_is() {
        assert_eq!(substring_hits("Buy Milk", "milk"), vec![4, 5, 6, 7]);
        assert!(substring_hits("Buy", "x").is_empty());
    }
}
