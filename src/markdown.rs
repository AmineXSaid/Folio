//! A note's text as Markdown, apart from GPUI: the blocks a sticker draws
//! (text, tasks, Remember boxes, tables), the edit bar's formatting as
//! changes to the text and its selection, and Tidy, which turns plain
//! pasted text into Folio's Markdown. Offsets are byte offsets.

use std::ops::Range;

use crate::model::task_line;

/// One stretch of a note, as the sticker draws it.
#[derive(Clone, Debug, PartialEq)]
pub enum Block {
    /// Plain Markdown.
    Text(String),
    /// Task lines, in order: done, and the text.
    Tasks(Vec<(bool, String)>),
    /// `>` lines: a Remember box, labelled when its first line is a label.
    Callout { label: Option<String>, text: String },
    /// A pipe table; every row has as many cells as the header.
    Table { head: Vec<String>, rows: Vec<Vec<String>> },
}

/// The note's text in the order it's drawn. Task lines keep their order,
/// so the nth task drawn is the nth `toggle_task` counts.
pub fn blocks(body: &str) -> Vec<Block> {
    let lines: Vec<&str> = body.lines().collect();
    let mut out: Vec<Block> = Vec::new();
    let mut text: Vec<&str> = Vec::new();
    let mut i = 0;
    while i < lines.len() {
        let line = lines[i];
        if let Some((done, t)) = task_line(line) {
            flush(&mut text, &mut out);
            match out.last_mut() {
                Some(Block::Tasks(v)) => v.push((done, t.to_string())),
                _ => out.push(Block::Tasks(vec![(done, t.to_string())])),
            }
            i += 1;
        } else if let Some(q) = quoted(line) {
            flush(&mut text, &mut out);
            let mut quote = vec![q];
            i += 1;
            while let Some(q) = lines.get(i).and_then(|l| quoted(l)) {
                quote.push(q);
                i += 1;
            }
            out.push(callout(&quote));
        } else if lines.get(i + 1).is_some_and(|next| is_separator(next, cells(line).len())) {
            flush(&mut text, &mut out);
            let head = cells(line);
            let mut rows = Vec::new();
            i += 2;
            while let Some(l) = lines.get(i).filter(|l| l.contains('|')) {
                let mut r = cells(l);
                r.resize(head.len(), String::new());
                rows.push(r);
                i += 1;
            }
            out.push(Block::Table { head, rows });
        } else {
            text.push(line);
            i += 1;
        }
    }
    flush(&mut text, &mut out);
    out
}

/// Each line on a line of its own, as typed (Markdown would run them into
/// one paragraph): a hard break at the end of every line that another
/// follows, outside code.
pub fn keep_lines(text: &str) -> String {
    let lines: Vec<&str> = text.split('\n').collect();
    let mut fence = false;
    let mut out = String::with_capacity(text.len() + lines.len() * 2);
    for (i, line) in lines.iter().enumerate() {
        if line.trim_start().starts_with("```") {
            fence = !fence;
        }
        out.push_str(line);
        let next = lines.get(i + 1);
        if !fence && !line.trim().is_empty() && next.is_some_and(|n| !n.trim().is_empty()) && !line.ends_with("  ") {
            out.push_str("  ");
        }
        if next.is_some() {
            out.push('\n');
        }
    }
    out
}

fn flush(text: &mut Vec<&str>, out: &mut Vec<Block>) {
    let t = text.join("\n");
    text.clear();
    if !t.trim().is_empty() {
        out.push(Block::Text(t));
    }
}

/// A `>` line's text.
fn quoted(line: &str) -> Option<&str> {
    let r = line.trim_start().strip_prefix('>')?;
    Some(r.strip_prefix(' ').unwrap_or(r))
}

/// A table row's cells, without the outer pipes.
fn cells(line: &str) -> Vec<String> {
    let t = line.trim();
    let t = t.strip_prefix('|').unwrap_or(t);
    let t = t.strip_suffix('|').unwrap_or(t);
    t.split('|').map(|c| c.trim().to_string()).collect()
}

/// `|---|:--:|` under a header of `n` cells.
fn is_separator(line: &str, n: usize) -> bool {
    let c = cells(line);
    line.contains('|')
        && n >= 1
        && c.len() == n
        && c.iter().all(|x| x.contains('-') && x.trim_matches(':').chars().all(|ch| ch == '-'))
}

/// Labels that open a Remember box: `Tip:`, `**Note:**`, `[!TIP]`.
const LABELS: [&str; 9] = ["tip", "note", "remember", "important", "warning", "memory trick", "key idea", "trick", "hint"];

fn callout(lines: &[&str]) -> Block {
    let mut rest: Vec<String> = lines.iter().map(|s| s.to_string()).collect();
    let first = rest[0].trim().to_string();
    let mut label = None;
    if let Some(kind) = first.strip_prefix("[!").and_then(|r| r.strip_suffix(']')) {
        label = Some(capitalised(kind));
        rest.remove(0);
    } else if let Some((l, after)) = first.strip_prefix("**").and_then(|r| r.split_once("**")) {
        // **Label**, **Label:** text or **Label**: text; not **a phrase** in a sentence.
        let after = after.trim_start();
        let colon = l.ends_with(':') || after.starts_with(':');
        let l = l.trim_end_matches(':').trim();
        if !l.is_empty() && l.chars().count() <= 24 && (colon || after.is_empty()) {
            label = Some(l.to_string());
            rest[0] = after.trim_start_matches(':').trim_start().to_string();
        }
    } else if let Some((l, after)) = first.split_once(':') {
        if LABELS.contains(&l.trim().to_lowercase().as_str()) {
            label = Some(capitalised(l.trim()));
            rest[0] = after.trim_start().to_string();
        }
    }
    let text = rest.join("\n").trim().to_string();
    Block::Callout { label, text }
}

fn capitalised(s: &str) -> String {
    let lower = s.to_lowercase();
    let mut c = lower.chars();
    c.next().map(|f| f.to_uppercase().chain(c).collect()).unwrap_or_default()
}

// ---- Formatting: each takes the text and selection, gives back both. ----

/// A list marker the edit bar puts at the start of lines.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Prefix {
    Bullet,
    Numbered,
    Task,
    Quote,
}

/// A 3×3 table to fill in.
pub const TABLE: &str = "| Column | Column | Column |\n|---|---|---|\n|  |  |  |\n|  |  |  |";

/// Pulls a range inside the text and onto character boundaries.
fn clamp(text: &str, r: Range<usize>) -> Range<usize> {
    let fix = |mut i: usize| {
        i = i.min(text.len());
        while !text.is_char_boundary(i) {
            i -= 1;
        }
        i
    };
    let (a, b) = (fix(r.start), fix(r.end));
    a.min(b)..a.max(b)
}

/// The word around `at`, if the cursor touches one.
fn word_at(text: &str, at: usize) -> Option<Range<usize>> {
    let word = |c: char| c.is_alphanumeric() || c == '_' || c == '\'';
    let start = text[..at].char_indices().rev().take_while(|(_, c)| word(*c)).last().map_or(at, |(i, _)| i);
    let end = text[at..].char_indices().take_while(|(_, c)| word(*c)).last().map_or(at, |(i, c)| at + i + c.len_utf8());
    (start < end).then_some(start..end)
}

/// The selection without the spaces at its ends.
fn trimmed(text: &str, r: Range<usize>) -> Range<usize> {
    let s = &text[r.clone()];
    if s.trim().is_empty() {
        return r;
    }
    let lead = s.len() - s.trim_start().len();
    let tail = s.len() - s.trim_end().len();
    r.start + lead..r.end - tail
}

/// Whether `mark` sits just before `at`. Stars count as runs: one is
/// italic, two bold, three both.
fn mark_before(text: &str, at: usize, mark: &str) -> bool {
    let before = &text[..at];
    if !before.ends_with(mark) {
        return false;
    }
    if mark.starts_with('*') {
        let run = before.chars().rev().take_while(|c| *c == '*').count();
        return stars_hold(run, mark.len());
    }
    true
}

fn mark_after(text: &str, at: usize, mark: &str) -> bool {
    let after = &text[at..];
    if !after.starts_with(mark) {
        return false;
    }
    if mark.starts_with('*') {
        let run = after.chars().take_while(|c| *c == '*').count();
        return stars_hold(run, mark.len());
    }
    true
}

fn stars_hold(run: usize, mark: usize) -> bool {
    if mark == 1 {
        run % 2 == 1
    } else {
        run >= 2
    }
}

/// Bold (`**`), italic (`*`), strikethrough (`~~`) or code (`` ` ``): wraps
/// the selection, or the word under the cursor, or unwraps it if it's
/// already wrapped. With nothing to wrap, puts the cursor between a pair.
pub fn toggle_wrap(text: &str, sel: Range<usize>, mark: &str) -> (String, Range<usize>) {
    let sel = clamp(text, sel);
    let sel = if sel.is_empty() { word_at(text, sel.start).unwrap_or(sel) } else { trimmed(text, sel) };
    let m = mark.len();
    if sel.start >= m && mark_before(text, sel.start, mark) && mark_after(text, sel.end, mark) {
        let s = format!("{}{}{}", &text[..sel.start - m], &text[sel.clone()], &text[sel.end + m..]);
        return (s, sel.start - m..sel.end - m);
    }
    let inner = &text[sel.clone()];
    if inner.len() >= 2 * m && mark_after(inner, 0, mark) && mark_before(inner, inner.len(), mark) {
        let s = format!("{}{}{}", &text[..sel.start], &inner[m..inner.len() - m], &text[sel.end..]);
        return (s, sel.start..sel.end - 2 * m);
    }
    let s = format!("{}{mark}{inner}{mark}{}", &text[..sel.start], &text[sel.end..]);
    (s, sel.start + m..sel.end + m)
}

const MARK_CLOSE: &str = "</mark>";

/// Where the `<mark …>` tag right before `at` starts.
fn mark_tag_before(text: &str, at: usize) -> Option<usize> {
    let before = &text[..at];
    if !before.ends_with('>') {
        return None;
    }
    let i = before.rfind("<mark")?;
    (!before[i + 1..].contains('<')).then_some(i)
}

/// A highlight in `color` (a name `tint_marks` knows), on or off.
pub fn toggle_mark(text: &str, sel: Range<usize>, color: &str) -> (String, Range<usize>) {
    let sel = clamp(text, sel);
    let sel = if sel.is_empty() { word_at(text, sel.start).unwrap_or(sel) } else { trimmed(text, sel) };
    if let Some(open) = mark_tag_before(text, sel.start).filter(|_| text[sel.end..].starts_with(MARK_CLOSE)) {
        let gone = sel.start - open;
        let s = format!("{}{}{}", &text[..open], &text[sel.clone()], &text[sel.end + MARK_CLOSE.len()..]);
        return (s, sel.start - gone..sel.end - gone);
    }
    let tag = format!("<mark color=\"{color}\">");
    let s = format!("{}{tag}{}{MARK_CLOSE}{}", &text[..sel.start], &text[sel.clone()], &text[sel.end..]);
    (s, sel.start + tag.len()..sel.end + tag.len())
}

fn line_start(text: &str, at: usize) -> usize {
    text[..at].rfind('\n').map_or(0, |i| i + 1)
}

fn line_end(text: &str, at: usize) -> usize {
    text[at..].find('\n').map_or(text.len(), |i| at + i)
}

/// A line taken apart: its indent, its `> ` and its list marker.
struct Parts<'a> {
    indent: &'a str,
    quote: &'a str,
    marker: Option<(Prefix, &'a str)>,
    content: &'a str,
}

fn parts(line: &str) -> Parts<'_> {
    let rest = line.trim_start_matches([' ', '\t']);
    let indent = &line[..line.len() - rest.len()];
    let q = if rest.starts_with("> ") { 2 } else if rest.starts_with('>') { 1 } else { 0 };
    let (quote, rest) = rest.split_at(q);
    let marker = list_marker(rest);
    let content = marker.map_or(rest, |(_, m)| &rest[m.len()..]);
    Parts { indent, quote, marker, content }
}

/// `- [ ] `, `- `, `3. ` or `3) ` at the start of `s`.
fn list_marker(s: &str) -> Option<(Prefix, &str)> {
    let b = s.as_bytes();
    if b.len() >= 2 && matches!(b[0], b'-' | b'*' | b'+') && b[1] == b' ' {
        for task in ["[ ] ", "[x] ", "[X] "] {
            if s[2..].starts_with(task) {
                return Some((Prefix::Task, &s[..6]));
            }
        }
        if matches!(&s[2..], "[ ]" | "[x]" | "[X]") {
            return Some((Prefix::Task, s));
        }
        return Some((Prefix::Bullet, &s[..2]));
    }
    let digits = s.bytes().take_while(u8::is_ascii_digit).count();
    if (1..=3).contains(&digits) && b.len() > digits + 1 && matches!(b[digits], b'.' | b')') && b[digits + 1] == b' ' {
        return Some((Prefix::Numbered, &s[..digits + 2]));
    }
    None
}

/// Bullets, numbers or tasks on every line the selection touches, or off
/// again when they all have them already. Numbers count from 1.
pub fn toggle_prefix(text: &str, sel: Range<usize>, kind: Prefix) -> (String, Range<usize>) {
    let sel = clamp(text, sel);
    let a = line_start(text, sel.start);
    let last = if sel.end > sel.start && text[..sel.end].ends_with('\n') { sel.end - 1 } else { sel.end };
    let b = line_end(text, last.max(a));
    let block = &text[a..b];
    let lines: Vec<&str> = block.split('\n').collect();
    let one = lines.len() == 1;
    let counts = |l: &&str| one || !l.trim().is_empty();
    let all_on = lines.iter().filter(|l| counts(l)).all(|l| {
        let p = parts(l);
        if kind == Prefix::Quote {
            !p.quote.is_empty()
        } else {
            p.marker.is_some_and(|(k, _)| k == kind)
        }
    });
    let mut n = 0;
    let new: Vec<String> = lines
        .iter()
        .map(|l| {
            if !counts(l) {
                return l.to_string();
            }
            let p = parts(l);
            let marker = p.marker.map_or("", |(_, m)| m);
            if kind == Prefix::Quote {
                return if all_on {
                    format!("{}{marker}{}", p.indent, p.content)
                } else {
                    format!("> {}{marker}{}", p.indent, p.content)
                };
            }
            if all_on {
                return format!("{}{}{}", p.indent, p.quote, p.content);
            }
            n += 1;
            let m = match kind {
                Prefix::Bullet => "- ".to_string(),
                Prefix::Numbered => format!("{n}. "),
                _ => "- [ ] ".to_string(),
            };
            format!("{}{}{m}{}", p.indent, p.quote, p.content)
        })
        .collect();
    let new = new.join("\n");
    let s = format!("{}{new}{}", &text[..a], &text[b..]);
    let range = if sel.is_empty() {
        let delta = new.len() as isize - block.len() as isize;
        let lead = new.len() - parts(&new).content.len();
        let at = (sel.start as isize + delta).max((a + lead) as isize) as usize;
        at.min(a + new.len())..at.min(a + new.len())
    } else {
        a..a + new.len()
    };
    let range = clamp(&s, range);
    (s, range)
}

/// A Remember box: `>` on the lines, with a "Remember" label on the first
/// unless it has one.
pub fn toggle_callout(text: &str, sel: Range<usize>) -> (String, Range<usize>) {
    let (s, r) = toggle_prefix(text, sel.clone(), Prefix::Quote);
    let a = line_start(&s, r.start.min(s.len()));
    let first = &s[a..line_end(&s, a)];
    let Some(rest) = first.strip_prefix("> ") else { return (s, r) };
    if quoted(&text[line_start(text, clamp(text, sel).start)..]).is_some() || rest.starts_with("**") || rest.starts_with("[!") {
        return (s, r);
    }
    const LABEL: &str = "**Remember:** ";
    let at = a + 2;
    let s2 = format!("{}{LABEL}{}", &s[..at], &s[at..]);
    let shift = |i: usize| if i >= at { i + LABEL.len() } else { i };
    let r2 = if r.is_empty() { shift(r.start)..shift(r.start) } else { r.start..shift(r.end) };
    (s2, r2)
}

/// Enter in a list: the next item, numbered on. Enter on an empty item
/// ends the list. `None` when the line isn't a list (a plain newline).
pub fn continue_list(text: &str, at: usize) -> Option<(String, usize)> {
    let at = clamp(text, at..at).start;
    let a = line_start(text, at);
    let b = line_end(text, at);
    let p = parts(&text[a..b]);
    if p.quote.is_empty() && p.marker.is_none() {
        return None;
    }
    let head = p.indent.len() + p.quote.len() + p.marker.map_or(0, |(_, m)| m.len());
    if at < a + head {
        return None;
    }
    if p.content.trim().is_empty() && at == b {
        // Ends the innermost: the list marker, else the quote.
        let kept = match p.marker {
            Some(_) => format!("{}{}", p.indent, p.quote),
            None => p.indent.to_string(),
        };
        let s = format!("{}{kept}{}", &text[..a], &text[b..]);
        return Some((s, a + kept.len()));
    }
    let next = match p.marker {
        Some((Prefix::Numbered, m)) => {
            let n: usize = m[..m.len() - 2].parse().unwrap_or(0);
            format!("{}{}", n + 1, &m[m.len() - 2..])
        }
        Some((Prefix::Task, m)) => format!("{}[ ] ", &m[..2]),
        Some((_, m)) => m.to_string(),
        None => String::new(),
    };
    let insert = format!("\n{}{}{next}", p.indent, p.quote);
    let s = format!("{}{insert}{}", &text[..at], &text[at..]);
    Some((s, at + insert.len()))
}

/// A blank table on lines of its own, with its first header cell selected
/// to type over.
pub fn insert_table(text: &str, sel: Range<usize>) -> (String, Range<usize>) {
    let sel = clamp(text, sel);
    let (before, after) = (&text[..sel.start], &text[sel.end..]);
    let lead = if before.is_empty() || before.ends_with('\n') { "" } else { "\n" };
    let tail = if after.is_empty() || after.starts_with('\n') { "" } else { "\n" };
    let s = format!("{before}{lead}{TABLE}{tail}{after}");
    let start = sel.start + lead.len() + 2;
    (s, start..start + "Column".len())
}

// ---- Tidy: plain text into Markdown. ----

/// Plain text into Markdown, with a title: a short first line, no full
/// stop, followed by more. The caller uses it when the note has none.
pub fn tidy(text: &str) -> (Option<String>, String) {
    let norm = normalise(text);
    let lines: Vec<&str> = norm.lines().collect();
    let Some(first) = lines.iter().position(|l| !l.trim().is_empty()) else { return (None, String::new()) };
    let more = lines[first + 1..].iter().any(|l| !l.trim().is_empty());
    let line = lines[first].trim();
    let plain = !line.contains('\t')
        && !line.contains('|')
        && tidy_line(line) == line
        && list_marker(line).is_none()
        && !line.starts_with('>');
    if more && plain && line.chars().count() <= 60 && !line.ends_with('.') {
        let title = line.trim_start_matches('#').trim().trim_end_matches(':').trim();
        let title = title.strip_prefix("**").and_then(|t| t.strip_suffix("**")).unwrap_or(title);
        return (Some(title.to_string()), tidy_body(&lines[first + 1..].join("\n")));
    }
    (None, tidy_body(&norm))
}

/// Plain text into Markdown, line by line. Running it twice changes
/// nothing more.
pub fn tidy_body(text: &str) -> String {
    let norm = normalise(text);
    let lines: Vec<&str> = norm.split('\n').collect();
    let mut out: Vec<String> = Vec::new();
    let mut fence = false;
    let mut i = 0;
    while i < lines.len() {
        let line = lines[i];
        if line.trim_start().starts_with("```") {
            fence = !fence;
        }
        if fence || line.trim_start().starts_with("```") {
            out.push(line.to_string());
            i += 1;
            continue;
        }
        if let Some(n) = columns(line) {
            let run = lines[i..].iter().take_while(|l| columns(l) == Some(n)).count();
            if run >= 2 {
                out.extend(table(&lines[i..i + run]));
                i += run;
                continue;
            }
        }
        out.push(tidy_line(line));
        i += 1;
    }
    tighten(out)
}

/// Whether Tidy would change this text: it has lists, tasks, a table or
/// labels written the plain way.
pub fn looks_structured(text: &str) -> bool {
    tidy_body(text).trim() != normalise(text).trim()
}

fn normalise(text: &str) -> String {
    let t = text.replace("\r\n", "\n").replace('\r', "\n").replace('\u{a0}', " ").replace('\u{200b}', "");
    t.split('\n').map(|l| l.trim_end_matches(' ')).collect::<Vec<_>>().join("\n")
}

/// Cells split by tabs (as Word and Excel copy them) or ` | `.
fn split_row(line: &str) -> Option<Vec<String>> {
    let t = line.trim_matches([' ', '\t']);
    if t.starts_with('|') || bullet(t).is_some() || task_glyph(t).is_some() || number(t).is_some() {
        return None;
    }
    let c: Vec<String> = if t.contains('\t') {
        t.split('\t').map(|c| c.trim().to_string()).collect()
    } else if t.contains(" | ") {
        t.split(" | ").map(|c| c.trim().to_string()).collect()
    } else {
        return None;
    };
    (c.len() >= 2).then_some(c)
}

fn columns(line: &str) -> Option<usize> {
    split_row(line).map(|c| c.len())
}

fn table(lines: &[&str]) -> Vec<String> {
    let row = |c: Vec<String>| format!("| {} |", c.iter().map(|x| x.replace('|', "\\|")).collect::<Vec<_>>().join(" | "));
    let mut rows = lines.iter().filter_map(|l| split_row(l));
    let head = rows.next().unwrap_or_default();
    let n = head.len();
    let mut out = vec![row(head), format!("|{}", "---|".repeat(n))];
    out.extend(rows.map(row));
    out
}

const BULLETS: [char; 12] = ['•', '●', '◦', '▪', '▫', '■', '‣', '⁃', '○', '–', '—', '·'];

/// The text after a bullet: `•`, `◦`, `–`, Word's `o` and tab, or `- `.
fn bullet(s: &str) -> Option<&str> {
    let c = s.chars().next()?;
    let after = &s[c.len_utf8()..];
    let spaced = after.starts_with([' ', '\t']);
    if BULLETS.contains(&c) {
        // A dash needs a space after it, or it's a range or a minus.
        return ((c != '–' && c != '—') || spaced).then_some(after);
    }
    if matches!(c, '-' | '*' | '+') && spaced {
        return Some(after);
    }
    (c == 'o' && after.starts_with('\t')).then_some(after)
}

/// A box drawn as a character, or `[ ]`: done, and the text after.
fn task_glyph(s: &str) -> Option<(bool, &str)> {
    const BOXES: [(&str, bool); 11] = [
        ("☐", false),
        ("□", false),
        ("❏", false),
        ("[ ]", false),
        ("☑", true),
        ("☒", true),
        ("✓", true),
        ("✔", true),
        ("✅", true),
        ("[x]", true),
        ("[X]", true),
    ];
    BOXES.iter().find_map(|(g, done)| {
        let after = s.strip_prefix(g)?;
        (after.is_empty() || after.starts_with(char::is_whitespace)).then_some((*done, after))
    })
}

/// `1)`, `(1)`, `1.` or `a)`: the number, and the text after.
fn number(s: &str) -> Option<(usize, &str)> {
    let spaced = |r: &str| r.starts_with([' ', '\t']);
    if let Some(r) = s.strip_prefix('(') {
        let d = r.bytes().take_while(u8::is_ascii_digit).count();
        let after = r.get(d..)?.strip_prefix(')')?;
        if (1..=3).contains(&d) && spaced(after) {
            return Some((r[..d].parse().ok()?, after));
        }
        return None;
    }
    let d = s.bytes().take_while(u8::is_ascii_digit).count();
    if (1..=3).contains(&d) {
        let after = s[d..].strip_prefix(['.', ')'])?;
        return spaced(after).then_some((s[..d].parse().ok()?, after));
    }
    let c = s.chars().next()?;
    let after = s[c.len_utf8()..].strip_prefix(')')?;
    (c.is_ascii_lowercase() && spaced(after)).then_some(((c as u8 - b'a') as usize + 1, after))
}

/// A line that opens with a label such as `Tip:`: the label, the rest.
fn label_line(s: &str) -> Option<(String, &str)> {
    let (l, after) = s.split_once(':')?;
    LABELS.contains(&l.trim().to_lowercase().as_str()).then(|| (capitalised(l.trim()), after))
}

/// A short term before the definition: a few words, not a link.
fn is_term(t: &str) -> bool {
    let t = t.trim();
    !t.is_empty() && t.chars().count() <= 30 && t.split_whitespace().count() <= 4 && !t.contains(['*', '[', '`', '/', '<'])
}

/// `Term: meaning` with the term in bold; in a list, `Term – meaning` too.
fn define(s: &str, in_list: bool) -> String {
    if s.starts_with("**") || s.contains('`') {
        return s.to_string();
    }
    if let Some((term, rest)) = s.split_once(": ") {
        if is_term(term) {
            return format!("**{}:** {}", term.trim(), rest.trim_start());
        }
    }
    if in_list {
        for dash in [" – ", " — "] {
            if let Some((term, rest)) = s.split_once(dash) {
                if is_term(term) {
                    return format!("**{}**{dash}{}", term.trim(), rest);
                }
            }
        }
    }
    s.to_string()
}

fn tidy_line(line: &str) -> String {
    tidy_line_inner(line).trim_end().to_string()
}

fn tidy_line_inner(line: &str) -> String {
    let t = line.trim_end_matches([' ', '\t']);
    let rest = t.trim_start_matches([' ', '\t']);
    if rest.is_empty() {
        return String::new();
    }
    let lead = &t[..t.len() - rest.len()];
    let level = lead.chars().filter(|c| *c == '\t').count() + lead.chars().filter(|c| *c == ' ').count() / 2;
    let indent = "  ".repeat(level);
    if rest.starts_with(['>', '|', '#']) {
        return t.to_string();
    }
    if let Some((done, after)) = task_line(rest) {
        return format!("{indent}- [{}] {after}", if done { "x" } else { " " });
    }
    if let Some((done, after)) = task_glyph(rest) {
        return format!("{indent}- [{}] {}", if done { "x" } else { " " }, after.trim_start());
    }
    if let Some(after) = bullet(rest) {
        return format!("{indent}- {}", define(after.trim_start(), true));
    }
    if let Some((n, after)) = number(rest) {
        return format!("{indent}{n}. {}", define(after.trim_start(), true));
    }
    if let Some((label, after)) = label_line(rest) {
        return format!("> **{label}:** {}", after.trim());
    }
    define(rest, false)
}

fn is_list(line: &str) -> bool {
    list_marker(line.trim_start()).is_some()
}

/// One blank line at most, none between list items, none at the ends.
fn tighten(lines: Vec<String>) -> String {
    let mut out: Vec<String> = Vec::new();
    for (i, l) in lines.iter().enumerate() {
        if l.trim().is_empty() {
            let prev = out.last();
            let next = lines[i + 1..].iter().find(|x| !x.trim().is_empty());
            let between_items = prev.is_some_and(|p| is_list(p)) && next.is_some_and(|n| is_list(n));
            if prev.is_none_or(|p| p.trim().is_empty()) || next.is_none() || between_items {
                continue;
            }
            out.push(String::new());
        } else {
            out.push(l.clone());
        }
    }
    out.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_note_splits_into_text_tasks_callouts_and_tables() {
        let body = "Intro **bold**\n- [ ] one\n- [x] two\n> **Remember:** each step\n> one more thing\n| A | B |\n|---|:-:|\n| 1 | 2 |\n| 3 |\nAfter";
        let b = blocks(body);
        assert_eq!(b[0], Block::Text("Intro **bold**".into()));
        assert_eq!(b[1], Block::Tasks(vec![(false, "one".into()), (true, "two".into())]));
        assert_eq!(b[2], Block::Callout { label: Some("Remember".into()), text: "each step\none more thing".into() });
        assert_eq!(
            b[3],
            Block::Table { head: vec!["A".into(), "B".into()], rows: vec![vec!["1".into(), "2".into()], vec!["3".into(), String::new()]] }
        );
        assert_eq!(b[4], Block::Text("After".into()));
    }

    #[test]
    fn callout_labels() {
        let label = |s: &str| match blocks(s).remove(0) {
            Block::Callout { label, text } => (label, text),
            other => panic!("{other:?}"),
        };
        assert_eq!(label("> **Tip**\n> text"), (Some("Tip".into()), "text".into()));
        assert_eq!(label("> [!NOTE]\n> text"), (Some("Note".into()), "text".into()));
        assert_eq!(label("> Memory trick: who controls"), (Some("Memory trick".into()), "who controls".into()));
        assert_eq!(label("> **Each step** makes it real"), (None, "**Each step** makes it real".into()));
        assert_eq!(label("> just text"), (None, "just text".into()));
    }

    #[test]
    fn lines_stay_as_typed() {
        assert_eq!(keep_lines("milk\neggs\n\nbread"), "milk  \neggs\n\nbread");
        assert_eq!(keep_lines("```\na\nb\n```"), "```\na\nb\n```");
        assert_eq!(keep_lines("one  \ntwo"), "one  \ntwo");
    }

    #[test]
    fn a_rule_without_pipes_is_not_a_table() {
        assert_eq!(blocks("Title\n---"), vec![Block::Text("Title\n---".into())]);
    }

    #[test]
    fn wrapping_toggles() {
        let (s, r) = toggle_wrap("make it bold", 8..12, "**");
        assert_eq!((s.as_str(), r.clone()), ("make it **bold**", 10..14));
        let (s, r) = toggle_wrap(&s, r, "**");
        assert_eq!((s.as_str(), r), ("make it bold", 8..12));
        // The word under the cursor, and a trailing space left out.
        assert_eq!(toggle_wrap("one two", 5..5, "*").0, "one *two*");
        assert_eq!(toggle_wrap("one two ", 4..8, "~~").0, "one ~~two~~ ");
        // Italic on a bold word adds a star; then takes it off again.
        let (s, r) = toggle_wrap("**word**", 2..6, "*");
        assert_eq!(s, "***word***");
        assert_eq!(toggle_wrap(&s, r, "*").0, "**word**");
        // Nothing to wrap: a pair with the cursor inside.
        let (s, r) = toggle_wrap("a ", 2..2, "`");
        assert_eq!((s.as_str(), r), ("a ``", 3..3));
        // The marks inside the selection.
        assert_eq!(toggle_wrap("x **y** z", 2..7, "**").0, "x y z");
    }

    #[test]
    fn highlights_toggle() {
        let (s, r) = toggle_mark("a key word", 2..5, "yellow");
        assert_eq!(s, "a <mark color=\"yellow\">key</mark> word");
        let (s, r) = toggle_mark(&s, r, "yellow");
        assert_eq!((s.as_str(), r), ("a key word", 2..5));
    }

    #[test]
    fn list_prefixes_toggle_across_lines() {
        let text = "intro\none\n\ntwo\nend";
        let (s, r) = toggle_prefix(text, 6..13, Prefix::Bullet);
        assert_eq!(s, "intro\n- one\n\n- two\nend");
        assert_eq!(&s[r.clone()], "- one\n\n- two");
        let (s, r) = toggle_prefix(&s, r, Prefix::Numbered);
        assert_eq!(s, "intro\n1. one\n\n2. two\nend");
        let (s, r) = toggle_prefix(&s, r, Prefix::Task);
        assert_eq!(s, "intro\n- [ ] one\n\n- [ ] two\nend");
        let (s, _) = toggle_prefix(&s, r, Prefix::Task);
        assert_eq!(s, "intro\none\n\ntwo\nend");
        // A cursor on an empty line: the marker, with the cursor after it.
        let (s, r) = toggle_prefix("a\n", 2..2, Prefix::Task);
        assert_eq!((s.as_str(), r), ("a\n- [ ] ", 8..8));
        // The cursor stays in its word.
        let (s, r) = toggle_prefix("hello", 3..3, Prefix::Bullet);
        assert_eq!((s.as_str(), r), ("- hello", 5..5));
    }

    #[test]
    fn remember_boxes_toggle_with_a_label() {
        let (s, r) = toggle_callout("x\n", 2..2);
        assert_eq!(s, "x\n> **Remember:** ");
        assert_eq!(r, s.len()..s.len());
        let (s, _) = toggle_callout("each step", 0..9);
        assert_eq!(s, "> **Remember:** each step");
        let (s, _) = toggle_prefix(&s, 0..s.len(), Prefix::Quote);
        assert_eq!(s, "**Remember:** each step");
        assert_eq!(toggle_callout("> **Tip:** a", 3..3).0, "**Tip:** a");
    }

    #[test]
    fn enter_continues_lists() {
        assert_eq!(continue_list("- one", 5), Some(("- one\n- ".into(), 8)));
        assert_eq!(continue_list("9) nine", 7), Some(("9) nine\n10) ".into(), 12)));
        assert_eq!(continue_list("  - [x] done", 12), Some(("  - [x] done\n  - [ ] ".into(), 21)));
        assert_eq!(continue_list("> a thought", 11), Some(("> a thought\n> ".into(), 14)));
        assert_eq!(continue_list("> - item", 8), Some(("> - item\n> - ".into(), 13)));
        // An empty item ends the list; the cursor in the marker doesn't continue it.
        assert_eq!(continue_list("- one\n- ", 8), Some(("- one\n".into(), 6)));
        assert_eq!(continue_list("> - ", 4), Some(("> ".into(), 2)));
        assert_eq!(continue_list("- one", 1), None);
        assert_eq!(continue_list("plain", 5), None);
    }

    #[test]
    fn tables_go_on_their_own_lines() {
        let (s, r) = insert_table("before", 6..6);
        assert!(s.starts_with("before\n| Column"));
        assert_eq!(&s[r], "Column");
        assert!(matches!(blocks(&s)[1], Block::Table { .. }));
    }

    #[test]
    fn tidy_turns_pasted_lists_into_markdown() {
        assert_eq!(tidy_body("•\tFirst\n\n•\tSecond\no\tUnder it"), "- First\n- Second\n- Under it");
        assert_eq!(tidy_body("\t– nested"), "  - nested");
        assert_eq!(tidy_body("1) one\n(2) two\nb) bee"), "1. one\n2. two\n2. bee");
        assert_eq!(tidy_body("☐ call\n✔ booked\n[ ] later"), "- [ ] call\n- [x] booked\n- [ ] later");
        assert_eq!(tidy_body("• Plant: what is controlled"), "- **Plant:** what is controlled");
        assert_eq!(tidy_body("• MIL – model in the loop"), "- **MIL** – model in the loop");
        assert_eq!(tidy_body("Tip: one more real thing"), "> **Tip:** one more real thing");
        assert_eq!(tidy_body("See http://x.org: it works"), "See http://x.org: it works");
        assert_eq!(tidy_body("a\r\n\r\n\r\n\r\nb  "), "a\n\nb");
    }

    #[test]
    fn tidy_makes_tables_from_tabs() {
        let excel = "Type\tController\tPlant\nMIL\tModel\tModel\nHIL\tReal ECU\tSim";
        assert_eq!(
            tidy_body(excel),
            "| Type | Controller | Plant |\n|---|---|---|\n| MIL | Model | Model |\n| HIL | Real ECU | Sim |"
        );
        // One line with a tab isn't a table; bullets with tabs aren't either.
        assert_eq!(tidy_body("just\tone"), "just\tone");
    }

    #[test]
    fn tidy_finds_a_title() {
        assert_eq!(tidy("HIL testing\n• one\n• two"), (Some("HIL testing".into()), "- one\n- two".into()));
        assert_eq!(tidy("A whole sentence.\nMore"), (None, "A whole sentence.\nMore".into()));
        assert_eq!(tidy("Only line"), (None, "Only line".into()));
        assert_eq!(tidy("• a list first\n• b").0, None);
    }

    #[test]
    fn tidying_twice_changes_nothing_more() {
        let raw = "Intro: words\n•\tOne\n\t◦ Two: deep\n1) first\nTip: remember\n☐ todo\nA\tB\nC\tD\n```\n• code\n```";
        let once = tidy_body(raw);
        assert_eq!(tidy_body(&once), once);
        assert!(once.contains("```\n• code\n```"));
        assert!(looks_structured(raw));
        assert!(!looks_structured("Just a sentence, nothing more."));
    }
}
