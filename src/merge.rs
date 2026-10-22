//! Merging two versions of the board: this computer's and Google Drive's,
//! from the last one they shared (the base). Each sheet, note, drawing,
//! text, connector, area and Trash item is matched by id and merged on its
//! own:
//!
//! - changed on one side only: that side's change wins;
//! - changed on both: the note edited last wins (counted as a clash);
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
    /// Things changed on both sides; the newer edit won.
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
plain_piece!(Stroke, Label, Link, Frame, Binned);

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
        _ if r.newer(l) => take_remote(&mut rep.clashes),
        _ => {
            rep.clashes += 1;
            l.clone()
        }
    }
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
        && a.stickers.len() == b.stickers.len()
        && a.stickers.iter().zip(&b.stickers).all(|(x, y)| x.same(y))
}

fn merge_sheet(base: Option<&Category>, l: &Category, r: &Category, rep: &mut Report) -> Category {
    let (hb, hl, hr) = (base.map(header), header(l), header(r));
    let mut c = if hl == hr {
        hl
    } else {
        match &hb {
            Some(b) if *b == hl => {
                rep.from_drive += 1;
                hr
            }
            Some(b) if *b == hr => {
                rep.from_here += 1;
                hl
            }
            _ => {
                rep.clashes += 1;
                if r.edited > l.edited {
                    hr
                } else {
                    hl
                }
            }
        }
    };
    c.stickers = merge_list(base.map(|b| b.stickers.as_slice()), &l.stickers, &r.stickers, rep);
    c.strokes = merge_list(base.map(|b| b.strokes.as_slice()), &l.strokes, &r.strokes, rep);
    c.labels = merge_list(base.map(|b| b.labels.as_slice()), &l.labels, &r.labels, rep);
    c.links = merge_list(base.map(|b| b.links.as_slice()), &l.links, &r.links, rep);
    c.frames = merge_list(base.map(|b| b.frames.as_slice()), &l.frames, &r.frames, rep);
    c.camera = l.camera;
    c.opened = l.opened.max(r.opened);
    c.edited = l.edited.max(r.edited);
    c
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
    fn the_newer_edit_wins_a_clash() {
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
        there.categories[0].links.push(Link { id: l, from: first, to: b, ink: Ink::Graphite });

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

        let (m, rep) = merge(None, &here, &there);
        assert_eq!(rep.clashes, 1, "only the note that really differs: {rep:?}");
        let names: Vec<&str> = m.categories.iter().map(|c| c.name.as_str()).collect();
        let demo_sheets = demo::board().categories.len();
        assert_eq!(m.categories.len(), demo_sheets + 2, "{names:?}");
        assert!(names.contains(&"Only here") && names.contains(&"Only in Drive"));
        assert_eq!(m.categories[0].stickers[0].body, "edited in Drive");
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
