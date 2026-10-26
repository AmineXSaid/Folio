//! Merging two versions of the board: this computer's and Google Drive's,
//! from the last one they shared (the base). Each sheet, note, drawing,
//! text, connector, area, shape and Trash item is matched by id and merged on its
//! own:
//!
//! - changed on one side only: that side's change wins;
//! - changed on both: merged, part by part (counted as a clash). A note's
//!   text keeps both sides' edits: lines changed on one side only, then
//!   words inside a line both changed; where both changed the same words,
//!   both versions stay, the later one first. Its title, place, colour
//!   and the rest go by what each side changed, the later edit winning
//!   where both changed the same thing. Text on the sheet merges the same
//!   way;
//! - added on either side: kept;
//! - deleted on one side and untouched on the other: deleted; deleted on
//!   one side but changed on the other: kept, so no edit is lost.
//!
//! Without a base (the first sync between two computers that each have
//! notes) everything from both sides is kept, and things that are plainly
//! the same (same id, same name or title) count once.
//!
//! Two computers number new things from the same counter, so the same id
//! can mean different things on each side. Drive's side is renumbered
//! first wherever that happens.
//!
//! Some things belong to a computer, not to the notes: settings, where each
//! sheet is scrolled to, which sheet is open, and notes popped out onto the
//! desktop. Those stay as this computer has them.

use std::collections::{HashMap, HashSet};

use crate::model::*;

/// What a merge did, for the person to read.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Report {
    /// Changes that came from Drive (other computers).
    pub from_drive: usize,
    /// Changes made on this computer, kept.
    pub from_here: usize,
    /// Things changed on both sides, merged.
    pub clashes: usize,
}

impl Report {
    pub fn is_empty(&self) -> bool {
        self.from_drive == 0 && self.from_here == 0 && self.clashes == 0
    }
}

/// One thing in a list on a sheet (or in the Trash), as merging sees it.
trait Piece: Clone + PartialEq {
    fn id(&self) -> Id;
    /// Equal in what syncs (leaving out what belongs to one computer).
    fn same(&self, o: &Self) -> bool {
        self == o
    }
    /// Edited after `o`; only notes know.
    fn newer(&self, _o: &Self) -> bool {
        false
    }
    /// Takes back what belongs to this computer from `local`.
    fn keep_local(&mut self, _local: &Self) {}
    /// Changed on both sides since `base` (if known): one thing with both
    /// sides' changes, as far as it can hold them. By default, the newer.
    fn join(_base: Option<&Self>, l: &Self, r: &Self) -> Self {
        if r.newer(l) {
            r.clone()
        } else {
            l.clone()
        }
    }
}

impl Piece for Sticker {
    fn id(&self) -> Id {
        self.id
    }
    /// Leaves out the desktop window and when it was edited: two copies
    /// that say the same thing are the same note.
    fn same(&self, o: &Self) -> bool {
        let mut a = self.clone();
        a.popped = o.popped;
        a.edited = o.edited;
        a == *o
    }
    fn newer(&self, o: &Self) -> bool {
        self.edited > o.edited
    }
    fn keep_local(&mut self, local: &Self) {
        self.popped = local.popped;
    }
    fn join(base: Option<&Self>, l: &Self, r: &Self) -> Self {
        let later = r.edited > l.edited;
        // Every field, so a new one can't be left out of merging.
        let Sticker { id, rect, paper, title, body, image, text_size, edited, due, reminded, locked, fixed, font, style, pin, link, popped, snoozed } = l;
        Sticker {
            id: *id,
            rect: field(base.map(|b| &b.rect), rect, &r.rect, later),
            paper: field(base.map(|b| &b.paper), paper, &r.paper, later),
            title: merge_title(base.map(|b| b.title.as_str()), title, &r.title, later),
            body: merge_text(base.map(|b| b.body.as_str()), body, &r.body, later),
            image: field(base.map(|b| &b.image), image, &r.image, later),
            text_size: field(base.map(|b| &b.text_size), text_size, &r.text_size, later),
            edited: (*edited).max(r.edited),
            due: field(base.map(|b| &b.due), due, &r.due, later),
            reminded: field(base.map(|b| &b.reminded), reminded, &r.reminded, later),
            locked: field(base.map(|b| &b.locked), locked, &r.locked, later),
            fixed: field(base.map(|b| &b.fixed), fixed, &r.fixed, later),
            font: field(base.map(|b| &b.font), font, &r.font, later),
            style: field(base.map(|b| &b.style), style, &r.style, later),
            pin: field(base.map(|b| &b.pin), pin, &r.pin, later),
            link: field(base.map(|b| &b.link), link, &r.link, later),
            popped: *popped,
            snoozed: (*snoozed).max(r.snoozed),
        }
    }
}

/// Text on the sheet: its words merged as a note's are; the rest as each
/// side changed it, this computer's where both did.
impl Piece for Label {
    fn id(&self) -> Id {
        self.id
    }
    fn join(base: Option<&Self>, l: &Self, r: &Self) -> Self {
        let Label { id, at, text, ink, font, size, width, italic, align } = l;
        Label {
            id: *id,
            at: field(base.map(|b| &b.at), at, &r.at, false),
            text: merge_text(base.map(|b| b.text.as_str()), text, &r.text, false),
            ink: field(base.map(|b| &b.ink), ink, &r.ink, false),
            font: field(base.map(|b| &b.font), font, &r.font, false),
            size: field(base.map(|b| &b.size), size, &r.size, false),
            width: field(base.map(|b| &b.width), width, &r.width, false),
            italic: field(base.map(|b| &b.italic), italic, &r.italic, false),
            align: field(base.map(|b| &b.align), align, &r.align, false),
        }
    }
}

/// A shape: its words merged as text on the sheet is; its place, turn,
/// colours and the rest as each side changed them, this computer's where
/// both did.
impl Piece for Shape {
    fn id(&self) -> Id {
        self.id
    }
    fn join(base: Option<&Self>, l: &Self, r: &Self) -> Self {
        let Shape { id, geom, rect, rot, fill, ink, line, dashed, text, locked } = l;
        Shape {
            id: *id,
            geom: field(base.map(|b| &b.geom), geom, &r.geom, false),
            rect: field(base.map(|b| &b.rect), rect, &r.rect, false),
            rot: field(base.map(|b| &b.rot), rot, &r.rot, false),
            fill: field(base.map(|b| &b.fill), fill, &r.fill, false),
            ink: field(base.map(|b| &b.ink), ink, &r.ink, false),
            line: field(base.map(|b| &b.line), line, &r.line, false),
            dashed: field(base.map(|b| &b.dashed), dashed, &r.dashed, false),
            text: merge_text(base.map(|b| b.text.as_str()), text, &r.text, false),
            locked: field(base.map(|b| &b.locked), locked, &r.locked, false),
        }
    }
}

/// One field changed on both sides: the side that changed it, or the
/// later edit (`later`: Drive's) where both did.
fn field<T: PartialEq + Clone>(base: Option<&T>, l: &T, r: &T, later: bool) -> T {
    match base {
        _ if l == r => l.clone(),
        Some(b) if b == l => r.clone(),
        Some(b) if b == r => l.clone(),
        _ if later => r.clone(),
        _ => l.clone(),
    }
}

macro_rules! plain_piece {
    ($($t:ty),*) => {$(
        impl Piece for $t {
            fn id(&self) -> Id {
                self.id
            }
        }
    )*};
}
plain_piece!(Stroke, Link, Frame, Binned);

/// Three-way merge of one list. This computer's order comes first, then
/// what only Drive has, in Drive's order.
fn merge_list<T: Piece>(base: Option<&[T]>, local: &[T], remote: &[T], rep: &mut Report) -> Vec<T> {
    let find = |list: &[T], id: Id| list.iter().position(|x| x.id() == id);
    let in_base = |id: Id| base.and_then(|b| b.iter().find(|x| x.id() == id));
    let mut out = Vec::with_capacity(local.len().max(remote.len()));
    for l in local {
        match find(remote, l.id()) {
            Some(i) => out.push(pick(in_base(l.id()), l, &remote[i], rep)),
            None => match in_base(l.id()) {
                // Deleted in Drive, untouched here: deleted.
                Some(b) if b.same(l) => rep.from_drive += 1,
                // Deleted in Drive, changed here: kept.
                Some(_) => {
                    rep.clashes += 1;
                    out.push(l.clone());
                }
                // Added here.
                None => {
                    rep.from_here += 1;
                    out.push(l.clone());
                }
            },
        }
    }
    for r in remote {
        if find(local, r.id()).is_some() {
            continue;
        }
        match in_base(r.id()) {
            Some(b) if b.same(r) => rep.from_here += 1,
            Some(_) => {
                rep.clashes += 1;
                out.push(r.clone());
            }
            None => {
                rep.from_drive += 1;
                out.push(r.clone());
            }
        }
    }
    out
}

fn pick<T: Piece>(base: Option<&T>, l: &T, r: &T, rep: &mut Report) -> T {
    if l.same(r) {
        // The same; keep the later edit time.
        return if r.newer(l) {
            let mut x = r.clone();
            x.keep_local(l);
            x
        } else {
            l.clone()
        };
    }
    let take_remote = |rep_field: &mut usize| {
        *rep_field += 1;
        let mut x = r.clone();
        x.keep_local(l);
        x
    };
    match base {
        Some(b) if b.same(l) => take_remote(&mut rep.from_drive),
        Some(b) if b.same(r) => {
            rep.from_here += 1;
            l.clone()
        }
        // Changed on both: merged.
        _ => {
            rep.clashes += 1;
            let mut x = T::join(base, l, r);
            x.keep_local(l);
            x
        }
    }
}

// ---------- Text changed on both sides ----------

/// A note's text changed on both sides since `base` (None: never synced,
/// so what both have in common stands in for it). Each side's changes are
/// kept: whole lines changed on one side only; inside lines both changed,
/// the words each changed; and where both changed the same words, both
/// versions, the later one (`later`: Drive's) first. Nothing typed on
/// either side is lost.
pub fn merge_text(base: Option<&str>, ours: &str, theirs: &str, later: bool) -> String {
    if ours == theirs {
        return ours.to_string();
    }
    let common;
    let base = match base {
        Some(b) => b,
        None => {
            common = shared(ours, theirs);
            &common
        }
    };
    if base == ours {
        return theirs.to_string();
    }
    if base == theirs {
        return ours.to_string();
    }
    let (b, o, t) = (lines(base), lines(ours), lines(theirs));
    let merged = join_tokens(&b, &o, &t, &mut |b, o, t, out| {
        // Lines changed on both sides: as many on each, line by line;
        // else all together. Word by word, unless the same words changed.
        let pairs: Vec<(String, String, String)> = if b.len() == o.len() && o.len() == t.len() {
            (0..b.len()).map(|i| (b[i].clone(), o[i].clone(), t[i].clone())).collect()
        } else {
            vec![(b.concat(), o.concat(), t.concat())]
        };
        for (b, o, t) in pairs {
            if o == b || o == t {
                out.push_str(&t);
            } else if t == b {
                out.push_str(&o);
            } else if let Some(m) = merge_words(&b, &o, &t) {
                out.push_str(&m);
            } else {
                let (first, second) = if later { (t, o) } else { (o, t) };
                out.push_str(&first);
                out.push_str(&second);
            }
        }
    });
    // Each line was taken with its line break; the text keeps its own end.
    match merged.strip_suffix('\n') {
        Some(m) if !ours.ends_with('\n') && !theirs.ends_with('\n') => m.to_string(),
        _ => merged,
    }
}

/// A text's lines, each with its line break (the last given one too).
fn lines(s: &str) -> Vec<String> {
    s.split_inclusive('\n').map(|l| if l.ends_with('\n') { l.to_string() } else { format!("{l}\n") }).collect()
}

/// A title changed on both sides: word by word, else the later one (a
/// title is a name; two names don't make one).
fn merge_title(base: Option<&str>, ours: &str, theirs: &str, later: bool) -> String {
    let common;
    let base = match base {
        Some(b) => b,
        None => {
            common = String::new();
            &common
        }
    };
    match merge_words(base, ours, theirs) {
        Some(m) if !base.is_empty() => m,
        _ => field(Some(&base.to_string()), &ours.to_string(), &theirs.to_string(), later),
    }
}

/// Words and the spaces between them, as tokens.
fn words(s: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for c in s.chars() {
        match out.last_mut() {
            Some(w) if w.chars().last().is_some_and(|p| p.is_whitespace() == c.is_whitespace()) => w.push(c),
            _ => out.push(c.to_string()),
        }
    }
    out
}

/// A three-way merge word by word; None where both sides changed the same
/// words differently.
fn merge_words(base: &str, ours: &str, theirs: &str) -> Option<String> {
    let mut clean = true;
    let out = join_tokens(&words(base), &words(ours), &words(theirs), &mut |_, _, _, _| clean = false);
    clean.then_some(out)
}

/// The lines two texts share, in order: a stand-in base for two that
/// never synced.
fn shared(a: &str, b: &str) -> String {
    let (a, b) = (lines(a), lines(b));
    common(&a, &b).into_iter().map(|(i, _)| a[i].as_str()).collect()
}

/// Three-way merge of token lists: stretches only one side changed take
/// that side; stretches both changed alike take it once; stretches both
/// changed differently go to `clash` (base, ours, theirs).
fn join_tokens(base: &[String], ours: &[String], theirs: &[String], clash: &mut dyn FnMut(&[String], &[String], &[String], &mut String)) -> String {
    let mut to_ours = vec![None; base.len()];
    for (b, o) in common(base, ours) {
        to_ours[b] = Some(o);
    }
    let mut to_theirs = vec![None; base.len()];
    for (b, t) in common(base, theirs) {
        to_theirs[b] = Some(t);
    }
    let mut out = String::new();
    let (mut b, mut o, mut t) = (0, 0, 0);
    let mut settle = |out: &mut String, b_end: usize, o_end: usize, t_end: usize, b: usize, o: usize, t: usize| {
        let (bs, os, ts) = (&base[b..b_end], &ours[o..o_end], &theirs[t..t_end]);
        if os == bs {
            out.extend(ts.iter().map(String::as_str));
        } else if ts == bs || os == ts {
            out.extend(os.iter().map(String::as_str));
        } else {
            clash(bs, os, ts, out);
        }
    };
    for i in 0..base.len() {
        // A token all three still share: what's before it is settled.
        if let (Some(oi), Some(ti)) = (to_ours[i], to_theirs[i]) {
            if oi >= o && ti >= t {
                settle(&mut out, i, oi, ti, b, o, t);
                out.push_str(&base[i]);
                (b, o, t) = (i + 1, oi + 1, ti + 1);
            }
        }
    }
    settle(&mut out, base.len(), ours.len(), theirs.len(), b, o, t);
    out
}

/// Where `a` and `b` match, in order: pairs of indices of a longest
/// common run of tokens (ends that match taken first, so small edits to
/// long texts stay quick).
fn common(a: &[String], b: &[String]) -> Vec<(usize, usize)> {
    let pre = a.iter().zip(b).take_while(|(x, y)| x == y).count();
    let suf = a[pre..].iter().rev().zip(b[pre..].iter().rev()).take_while(|(x, y)| x == y).count();
    let (am, bm) = (&a[pre..a.len() - suf], &b[pre..b.len() - suf]);
    let mut out: Vec<(usize, usize)> = (0..pre).map(|i| (i, i)).collect();
    let (n, m) = (am.len(), bm.len());
    // Too long to compare token by token: the middle counts as all changed.
    if n * m <= 4_000_000 {
        let w = m + 1;
        let mut len = vec![0u32; (n + 1) * w];
        for i in (0..n).rev() {
            for j in (0..m).rev() {
                len[i * w + j] = if am[i] == bm[j] { len[(i + 1) * w + j + 1] + 1 } else { len[(i + 1) * w + j].max(len[i * w + j + 1]) };
            }
        }
        let (mut i, mut j) = (0, 0);
        while i < n && j < m {
            if am[i] == bm[j] {
                out.push((pre + i, pre + j));
                i += 1;
                j += 1;
            } else if len[(i + 1) * w + j] >= len[i * w + j + 1] {
                i += 1;
            } else {
                j += 1;
            }
        }
    }
    out.extend((0..suf).map(|k| (a.len() - suf + k, b.len() - suf + k)));
    out
}

/// A sheet without its contents and what belongs to one computer: what
/// its name, icon, pin and so on are compared by.
fn header(c: &Category) -> Category {
    Category {
        stickers: Vec::new(),
        strokes: Vec::new(),
        labels: Vec::new(),
        links: Vec::new(),
        frames: Vec::new(),
        shapes: Vec::new(),
        camera: Camera::default(),
        opened: 0,
        edited: 0,
        ..c.clone()
    }
}

/// Equal in what syncs: contents and header, not the view.
fn same_sheet(a: &Category, b: &Category) -> bool {
    header(a) == header(b)
        && a.strokes == b.strokes
        && a.labels == b.labels
        && a.links == b.links
        && a.frames == b.frames
        && a.shapes == b.shapes
        && a.stickers.len() == b.stickers.len()
        && a.stickers.iter().zip(&b.stickers).all(|(x, y)| x.same(y))
}

fn merge_sheet(base: Option<&Category>, l: &Category, r: &Category, rep: &mut Report) -> Category {
    let (hb, hl, hr) = (base.map(header), header(l), header(r));
    let mut c = if hl == hr { hl } else { merge_header(hb.as_ref(), &hl, &hr, r.edited > l.edited, rep) };
    c.stickers = merge_list(base.map(|b| b.stickers.as_slice()), &l.stickers, &r.stickers, rep);
    c.strokes = merge_list(base.map(|b| b.strokes.as_slice()), &l.strokes, &r.strokes, rep);
    c.labels = merge_list(base.map(|b| b.labels.as_slice()), &l.labels, &r.labels, rep);
    c.links = merge_list(base.map(|b| b.links.as_slice()), &l.links, &r.links, rep);
    c.frames = merge_list(base.map(|b| b.frames.as_slice()), &l.frames, &r.frames, rep);
    c.shapes = merge_list(base.map(|b| b.shapes.as_slice()), &l.shapes, &r.shapes, rep);
    c.camera = l.camera;
    c.opened = l.opened.max(r.opened);
    c.edited = l.edited.max(r.edited);
    c
}

/// Two sheet headers that differ, field by field: a field changed on one
/// side only takes that side; changed on both, the name merges like a title
/// and anything else takes the later side. So a sheet renamed here and
/// recoloured there keeps both changes.
fn merge_header(base: Option<&Category>, l: &Category, r: &Category, later: bool, rep: &mut Report) -> Category {
    use serde_json::{Map, Value};
    let obj = |c: &Category| match serde_json::to_value(c) {
        Ok(Value::Object(m)) => m,
        _ => Map::new(),
    };
    let (lo, ro) = (obj(l), obj(r));
    let bo = base.map(obj);
    let (mut here, mut there, mut clash) = (false, false, false);
    let mut out = Map::new();
    for key in lo.keys().chain(ro.keys().filter(|k| !lo.contains_key(*k))) {
        let (a, b) = (lo.get(key).cloned().unwrap_or(Value::Null), ro.get(key).cloned().unwrap_or(Value::Null));
        let was = bo.as_ref().map(|m| m.get(key).cloned().unwrap_or(Value::Null));
        let v = if a == b {
            a
        } else if was.as_ref() == Some(&a) {
            there = true;
            b
        } else if was.as_ref() == Some(&b) {
            here = true;
            a
        } else {
            clash = true;
            if key == "name" {
                Value::String(merge_title(base.map(|c| c.name.as_str()), &l.name, &r.name, later))
            } else if later {
                b
            } else {
                a
            }
        };
        out.insert(key.clone(), v);
    }
    if clash {
        rep.clashes += 1;
    } else {
        rep.from_drive += there as usize;
        rep.from_here += here as usize;
    }
    serde_json::from_value(Value::Object(out)).unwrap_or_else(|_| if later { r.clone() } else { l.clone() })
}

// ---------- Ids that mean different things on each side ----------

/// Everything with an id on a board, and what it is: its kind, and what
/// makes two of them "plainly the same" when there's no base to go by.
fn things(b: &Board) -> HashMap<Id, (u8, String)> {
    let mut m = HashMap::new();
    for c in &b.categories {
        m.insert(c.id, (0, if c.inbox { "\u{0}inbox".into() } else { c.name.clone() }));
        for s in &c.stickers {
            m.insert(s.id, (1, s.title.clone()));
        }
        for x in &c.strokes {
            m.insert(x.id, (2, json(x)));
        }
        for x in &c.labels {
            m.insert(x.id, (3, json(x)));
        }
        for x in &c.links {
            m.insert(x.id, (4, format!("{}>{}", x.from, x.to)));
        }
        for x in &c.frames {
            m.insert(x.id, (5, x.title.clone()));
        }
        for x in &c.shapes {
            m.insert(x.id, (7, json(x)));
        }
    }
    for x in &b.bin {
        m.insert(x.id, (6, x.name()));
    }
    m
}

/// A thing's content as text, without its id.
fn json<T: serde::Serialize>(x: &T) -> String {
    let mut v = serde_json::to_value(x).unwrap_or_default();
    if let Some(o) = v.as_object_mut() {
        o.remove("id");
    }
    v.to_string()
}

fn max_id(b: &Board) -> Id {
    things(b).keys().copied().max().unwrap_or(0).max(b.next_id)
}

/// How Drive's ids become this computer's: renumbering where the same id
/// is something else here, and joining the two Inboxes.
fn renumbering(base: Option<&Board>, local: &Board, remote: &Board) -> HashMap<Id, Id> {
    let shared: HashSet<Id> = base.map(|b| things(b).into_keys().collect()).unwrap_or_default();
    let here = things(local);
    let there = things(remote);
    let mut next = max_id(local).max(max_id(remote)).max(base.map(max_id).unwrap_or(0));
    let mut map = HashMap::new();
    let mut ids: Vec<&Id> = there.keys().collect();
    ids.sort();
    for &id in ids {
        if shared.contains(&id) {
            continue;
        }
        if let Some(mine) = here.get(&id) {
            if *mine != there[&id] {
                next += 1;
                map.insert(id, next);
            }
        }
    }
    // One Inbox: Drive's joins this computer's.
    let inbox = |b: &Board| b.categories.iter().find(|c| c.inbox).map(|c| c.id);
    if let (Some(li), Some(ri)) = (inbox(local), inbox(remote)) {
        if li != ri && !there.contains_key(&li) && !shared.contains(&ri) {
            map.insert(ri, li);
        }
    }
    map
}

fn renumber_sheet(c: &mut Category, map: &HashMap<Id, Id>) {
    let m = |id: &mut Id| {
        if let Some(n) = map.get(id) {
            *id = *n;
        }
    };
    m(&mut c.id);
    c.stickers.iter_mut().for_each(|x| m(&mut x.id));
    c.strokes.iter_mut().for_each(|x| m(&mut x.id));
    c.labels.iter_mut().for_each(|x| m(&mut x.id));
    c.frames.iter_mut().for_each(|x| m(&mut x.id));
    c.shapes.iter_mut().for_each(|x| m(&mut x.id));
    for l in &mut c.links {
        m(&mut l.id);
        m(&mut l.from);
        m(&mut l.to);
    }
}

fn renumber(b: &mut Board, map: &HashMap<Id, Id>) {
    if map.is_empty() {
        return;
    }
    for c in &mut b.categories {
        renumber_sheet(c, map);
    }
    for x in &mut b.bin {
        if let Some(n) = map.get(&x.id) {
            x.id = *n;
        }
        match &mut x.item {
            BinItem::Note { sheet, sticker, .. } => {
                if let Some(n) = map.get(sheet) {
                    *sheet = *n;
                }
                if let Some(n) = map.get(&sticker.id) {
                    sticker.id = *n;
                }
            }
            BinItem::Sheet(c) => renumber_sheet(c, map),
        }
    }
}

// ---------- The whole board ----------

/// Merges this computer's board with Drive's, from the last one they
/// shared (`None` if they never synced).
pub fn merge(base: Option<&Board>, local: &Board, remote: &Board) -> (Board, Report) {
    let mut remote = remote.clone();
    let map = renumbering(base, local, &remote);
    renumber(&mut remote, &map);
    let mut rep = Report::default();

    let sheet_in = |b: &Board, id: Id| b.categories.iter().find(|c| c.id == id).cloned();
    let mut sheets = Vec::new();
    for l in &local.categories {
        let b = base.and_then(|b| sheet_in(b, l.id));
        match sheet_in(&remote, l.id) {
            Some(r) => sheets.push(merge_sheet(b.as_ref(), l, &r, &mut rep)),
            None => match b {
                Some(b) if same_sheet(&b, l) => rep.from_drive += 1,
                Some(_) => {
                    rep.clashes += 1;
                    sheets.push(l.clone());
                }
                None => {
                    rep.from_here += 1;
                    sheets.push(l.clone());
                }
            },
        }
    }
    for r in &remote.categories {
        if sheet_in(local, r.id).is_some() {
            continue;
        }
        match base.and_then(|b| sheet_in(b, r.id)) {
            Some(b) if same_sheet(&b, r) => rep.from_here += 1,
            Some(_) => {
                rep.clashes += 1;
                sheets.push(r.clone());
            }
            None => {
                rep.from_drive += 1;
                sheets.push(r.clone());
            }
        }
    }

    let mut out = local.clone();
    let active = local.categories.get(local.active).map(|c| c.id);
    out.categories = sheets;
    out.bin = merge_list(base.map(|b| b.bin.as_slice()), &local.bin, &remote.bin, &mut rep);
    // A sheet both restored here and kept in Drive's Trash: it's back.
    let live: HashSet<Id> = out.categories.iter().map(|c| c.id).collect();
    out.bin.retain(|x| !matches!(&x.item, BinItem::Sheet(c) if live.contains(&c.id)));
    out.active = active.and_then(|id| out.categories.iter().position(|c| c.id == id)).unwrap_or(0);
    out.next_id = local.next_id.max(remote.next_id).max(max_id(&out));
    out.normalize();
    (out, rep)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::demo;

    fn note(id: Id, title: &str, edited: u64) -> Sticker {
        Sticker { id, title: title.into(), edited, rect: Rect { x: 0., y: 0., w: 200., h: 120. }, ..Default::default() }
    }

    fn titles(b: &Board, sheet: usize) -> Vec<String> {
        b.categories[sheet].stickers.iter().map(|s| s.title.clone()).collect()
    }

    fn base() -> Board {
        let mut b = Board::blank();
        b.add_sheet("Work");
        b.add_sheet("Home");
        let n1 = b.fresh_id();
        let n2 = b.fresh_id();
        b.categories[0].stickers = vec![note(n1, "Plan", 10), note(n2, "Draft", 10)];
        b
    }

    #[test]
    fn shapes_merge_by_id_and_by_field() {
        let mut base = base();
        let id = base.fresh_id();
        base.categories[0].shapes.push(Shape::new(id, Geom::Rect, v2(0., 0.)));
        let mut here = base.clone();
        here.categories[0].shapes[0].rot = 30.;
        let mut there = base.clone();
        there.categories[0].shapes[0].text = "Router".into();
        let added = there.fresh_id();
        there.categories[0].shapes.push(Shape::new(added, Geom::Arrow, v2(50., 0.)));

        let (m, _) = merge(Some(&base), &here, &there);
        let shapes = &m.categories[0].shapes;
        assert_eq!(shapes.len(), 2);
        // Both sides changed the same shape, different parts: both land.
        assert_eq!((shapes[0].rot, shapes[0].text.as_str()), (30., "Router"));
        assert!(shapes.iter().any(|s| s.id == added && s.geom == Geom::Arrow));

        // The same id added on each side for different shapes: Drive's is
        // renumbered, so both stay.
        let mut here = base.clone();
        let mut there = base.clone();
        let a = here.fresh_id();
        here.categories[0].shapes.push(Shape::new(a, Geom::Ellipse, v2(0., 0.)));
        let b = there.fresh_id();
        assert_eq!(a, b);
        there.categories[0].shapes.push(Shape::new(b, Geom::Cloud, v2(0., 0.)));
        let (m, _) = merge(Some(&base), &here, &there);
        let geoms: Vec<Geom> = m.categories[0].shapes.iter().map(|s| s.geom).collect();
        assert!(geoms.contains(&Geom::Ellipse) && geoms.contains(&Geom::Cloud), "{geoms:?}");
    }

    #[test]
    fn changes_on_each_side_both_land() {
        let base = base();
        let mut here = base.clone();
        here.categories[0].stickers[0].title = "Plan v2".into();
        here.categories[0].stickers[0].edited = 20;
        let mut there = base.clone();
        let id = there.fresh_id();
        there.categories[1].stickers.push(note(id, "Groceries", 30));
        there.categories[1].name = "House".into();

        let (m, rep) = merge(Some(&base), &here, &there);
        assert_eq!(titles(&m, 0), ["Plan v2", "Draft"]);
        assert_eq!(titles(&m, 1), ["Groceries"]);
        assert_eq!(m.categories[1].name, "House");
        assert_eq!(rep, Report { from_drive: 2, from_here: 1, clashes: 0 });
    }

    #[test]
    fn a_title_both_renamed_takes_the_newer_name() {
        let base = base();
        let mut here = base.clone();
        here.categories[0].stickers[0].title = "Mine".into();
        here.categories[0].stickers[0].edited = 50;
        let mut there = base.clone();
        there.categories[0].stickers[0].title = "Theirs".into();
        there.categories[0].stickers[0].edited = 60;
        let (m, rep) = merge(Some(&base), &here, &there);
        assert_eq!(titles(&m, 0)[0], "Theirs");
        assert_eq!(rep.clashes, 1);
        // Words added at either end of a title both stay.
        here.categories[0].stickers[0].title = "Plan v2".into();
        there.categories[0].stickers[0].title = "Big Plan".into();
        let (m, _) = merge(Some(&base), &here, &there);
        assert_eq!(titles(&m, 0)[0], "Big Plan v2");
    }

    #[test]
    fn a_sheet_renamed_here_and_recoloured_there_keeps_both() {
        let base = base();
        let mut here = base.clone();
        here.categories[0].name = "Work".into();
        here.categories[0].edited = 50;
        let mut there = base.clone();
        there.categories[0].icon = "🚗".into();
        there.categories[0].edited = 60;
        let (m, rep) = merge(Some(&base), &here, &there);
        assert_eq!((m.categories[0].name.as_str(), m.categories[0].icon.as_str()), ("Work", "🚗"));
        assert_eq!(rep.clashes, 0);

        // Renamed on both: the later name wins.
        let mut here = base.clone();
        here.categories[0].name = "Mine".into();
        here.categories[0].edited = 50;
        let mut there = base.clone();
        there.categories[0].name = "Theirs".into();
        there.categories[0].edited = 60;
        let (m, rep) = merge(Some(&base), &here, &there);
        assert_eq!(m.categories[0].name, "Theirs");
        assert_eq!(rep.clashes, 1);
    }

    /// The first note's body, written on the base, here and in Drive.
    fn bodies(base_body: &str, here_body: &str, there_body: &str) -> String {
        let mut base = base();
        base.categories[0].stickers[0].body = base_body.into();
        let mut here = base.clone();
        here.categories[0].stickers[0].body = here_body.into();
        here.categories[0].stickers[0].edited = 50;
        let mut there = base.clone();
        there.categories[0].stickers[0].body = there_body.into();
        there.categories[0].stickers[0].edited = 60;
        let (m, _) = merge(Some(&base), &here, &there);
        m.categories[0].stickers[0].body.clone()
    }

    #[test]
    fn a_note_edited_on_two_computers_keeps_both_edits() {
        // Different lines.
        assert_eq!(bodies("one\ntwo\nthree", "ONE\ntwo\nthree", "one\ntwo\nthree\nfour"), "ONE\ntwo\nthree\nfour");
        // The same line, different words.
        assert_eq!(bodies("Buy milk and bread", "Buy oat milk and bread", "Buy milk and fresh bread"), "Buy oat milk and fresh bread");
        // Tasks ticked on one side, added on the other.
        assert_eq!(bodies("- [ ] a\n- [ ] b", "- [x] a\n- [ ] b", "- [ ] a\n- [ ] b\n- [ ] c"), "- [x] a\n- [ ] b\n- [ ] c");
        // Both deleted the same line: gone once.
        assert_eq!(bodies("a\nb\nc", "a\nc", "a\nc"), "a\nc");
    }

    #[test]
    fn the_same_words_changed_both_ways_keep_both_versions() {
        let m = bodies("Meet at 10", "Meet at 11", "Meet at noon");
        assert_eq!(m, "Meet at noon\nMeet at 11", "Drive's, the later, first");
        // Lines around a clash still merge.
        let m = bodies("top\nMeet at 10\nend", "TOP\nMeet at 11\nend", "top\nMeet at noon\nend!");
        assert_eq!(m, "TOP\nMeet at noon\nMeet at 11\nend!");
    }

    #[test]
    fn other_changes_to_a_note_both_land() {
        let base = base();
        let mut here = base.clone();
        here.categories[0].stickers[0].rect.x = 300.;
        here.categories[0].stickers[0].body = "here".into();
        here.categories[0].stickers[0].edited = 50;
        let mut there = base.clone();
        there.categories[0].stickers[0].paper = Paper::Sage;
        there.categories[0].stickers[0].locked = true;
        there.categories[0].stickers[0].edited = 60;
        let (m, _) = merge(Some(&base), &here, &there);
        let s = &m.categories[0].stickers[0];
        assert_eq!((s.rect.x, s.paper, s.locked, s.body.as_str(), s.edited), (300., Paper::Sage, true, "here", 60));
    }

    #[test]
    fn text_on_the_sheet_merges_too() {
        let mut base = base();
        let id = base.fresh_id();
        let mut l = Label::new(id, v2(0., 0.), Ink::Graphite);
        l.text = "Ideas for the week".into();
        base.categories[0].labels.push(l);
        let mut here = base.clone();
        here.categories[0].labels[0].text = "Big ideas for the week".into();
        let mut there = base.clone();
        there.categories[0].labels[0].text = "Ideas for the weekend".into();
        there.categories[0].labels[0].ink = Ink::Coral;
        let (m, _) = merge(Some(&base), &here, &there);
        let l = &m.categories[0].labels[0];
        assert_eq!((l.text.as_str(), l.ink), ("Big ideas for the weekend", Ink::Coral));
    }

    #[test]
    fn text_from_two_computers_that_never_synced_keeps_both_once() {
        // One added a line: what they share stands in for the base.
        assert_eq!(merge_text(None, "a\nb", "a\nb\nc", true), "a\nb\nc");
        // Both wrote something different: both, the later first.
        assert_eq!(merge_text(None, "mine", "theirs", true), "theirs\nmine");
        assert_eq!(merge_text(None, "mine", "theirs", false), "mine\ntheirs");
    }

    #[test]
    fn deletes_win_over_nothing_but_not_over_an_edit() {
        let base = base();
        // Deleted in Drive, untouched here: gone.
        let mut there = base.clone();
        there.categories[0].stickers.remove(1);
        let (m, _) = merge(Some(&base), &base.clone(), &there);
        assert_eq!(titles(&m, 0), ["Plan"]);
        // Deleted in Drive, edited here: kept.
        let mut here = base.clone();
        here.categories[0].stickers[1].body = "more".into();
        let (m, rep) = merge(Some(&base), &here, &there);
        assert_eq!(titles(&m, 0), ["Plan", "Draft"]);
        assert_eq!(rep.clashes, 1);
        // A whole sheet deleted here, untouched in Drive: gone.
        let mut here = base.clone();
        here.categories.remove(1);
        let (m, _) = merge(Some(&base), &here, &base.clone());
        assert_eq!(m.categories.len(), 1);
    }

    #[test]
    fn the_same_new_id_on_both_sides_becomes_two_notes() {
        let base = base();
        let mut here = base.clone();
        let a = here.fresh_id();
        here.categories[0].stickers.push(note(a, "Buy milk", 40));
        let mut there = base.clone();
        let b = there.fresh_id();
        assert_eq!(a, b, "both computers count from the same number");
        there.categories[0].stickers.push(note(b, "Call mom", 41));
        let l = there.fresh_id();
        let first = there.categories[0].stickers[0].id;
        there.categories[0].links.push(Link::new(l, first, b, Ink::Graphite));

        let (m, rep) = merge(Some(&base), &here, &there);
        assert_eq!(titles(&m, 0), ["Plan", "Draft", "Buy milk", "Call mom"]);
        let ids: HashSet<Id> = m.categories[0].stickers.iter().map(|s| s.id).collect();
        assert_eq!(ids.len(), 4, "all different ids");
        let link = &m.categories[0].links[0];
        let call = m.categories[0].stickers.iter().find(|s| s.title == "Call mom").unwrap();
        assert_eq!(link.to, call.id, "the connector follows the renumbered note");
        assert!(m.next_id >= call.id);
        assert_eq!(rep.from_drive, 2);
    }

    #[test]
    fn two_computers_that_never_synced_keep_everything_once() {
        let mut here = demo::board();
        here.add_sheet("Only here");
        let mut there = demo::board();
        there.add_sheet("Only in Drive");
        // The same demo note, edited later in Drive.
        let t = there.categories[0].stickers[0].edited + 100;
        there.categories[0].stickers[0].body = "edited in Drive".into();
        there.categories[0].stickers[0].edited = t;

        let original = here.categories[0].stickers[0].body.clone();
        let (m, rep) = merge(None, &here, &there);
        assert_eq!(rep.clashes, 1, "only the note that really differs: {rep:?}");
        let names: Vec<&str> = m.categories.iter().map(|c| c.name.as_str()).collect();
        let demo_sheets = demo::board().categories.len();
        assert_eq!(m.categories.len(), demo_sheets + 2, "{names:?}");
        assert!(names.contains(&"Only here") && names.contains(&"Only in Drive"));
        // Never synced: no telling which came first, so both stay, Drive's
        // (the later) first.
        assert_eq!(m.categories[0].stickers[0].body, format!("edited in Drive\n{original}"));
        assert_eq!(m.categories.iter().filter(|c| c.inbox).count(), demo::board().categories.iter().filter(|c| c.inbox).count());
    }

    #[test]
    fn what_belongs_to_this_computer_stays() {
        let base = base();
        let mut here = base.clone();
        here.categories[0].camera.zoom = 2.;
        here.categories[0].stickers[0].popped = Some(Popped { x: 1., y: 2., w: 3., h: 4., see_through: false });
        here.settings.ui_scale = 1.2;
        here.active = 1;
        let mut there = base.clone();
        there.categories[0].camera.zoom = 0.5;
        there.categories[0].stickers[0].title = "From Drive".into();
        there.settings.ui_scale = 0.9;
        let (m, _) = merge(Some(&base), &here, &there);
        assert_eq!(m.categories[0].camera.zoom, 2.);
        assert!(m.categories[0].stickers[0].popped.is_some());
        assert_eq!(m.categories[0].stickers[0].title, "From Drive");
        assert_eq!(m.settings.ui_scale, 1.2);
        assert_eq!(m.active, 1);
    }

    #[test]
    fn merging_with_itself_changes_nothing() {
        let b = demo::board();
        let (m, rep) = merge(Some(&b), &b, &b);
        assert!(rep.is_empty());
        assert_eq!(m.categories, b.categories);
    }
}
