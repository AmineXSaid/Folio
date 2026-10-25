//! Undo steps as changes: what one edit did to the board, item by item,
//! rather than a copy of the whole board. A step keeps the notes, strokes,
//! text, links, frames, sheets and Trash entries it touched, each as it was
//! before and after, and where any came, went or moved to, so it plays back
//! either way. Moving one note on a 2,000-note sheet is a step of a few
//! hundred bytes.
//!
//! Undo is about content: each sheet's camera and when it was last opened,
//! the settings and the screen stay as they are. Nothing here knows about
//! GPUI.

use std::collections::{HashMap, HashSet};

use serde::{Deserialize, Serialize};

use crate::model::{Binned, Board, Category, Frame, Id, Label, Link, Sticker, Stroke};

/// Something kept in a list by its id.
pub trait Keyed: Clone + PartialEq {
    fn key(&self) -> Id;
    /// The same as far as undo goes.
    fn same(&self, o: &Self) -> bool {
        self == o
    }
    /// Before undo or redo puts `self` in place of `now`: takes what undo
    /// leaves alone from `now`.
    fn keep(&mut self, _now: &Self) {}
}

macro_rules! keyed {
    ($($t:ty),*) => {
        $(impl Keyed for $t {
            fn key(&self) -> Id {
                self.id
            }
        })*
    };
}
keyed!(Stroke, Label, Link, Frame, Binned, Category);

/// A note's desktop window (where it is, see-through) and whether its
/// reminder was shown change outside undo, by the window and the clock:
/// undo leaves them as they are, as the merge with Drive does. Otherwise
/// undoing a drag could close a desktop window or show a reminder again.
impl Keyed for Sticker {
    fn key(&self) -> Id {
        self.id
    }
    fn same(&self, o: &Self) -> bool {
        if self == o {
            return true;
        }
        if self.popped == o.popped && (self.reminded == o.reminded || self.due != o.due) {
            return false;
        }
        let mut a = self.clone();
        a.keep(o);
        a == *o
    }
    fn keep(&mut self, now: &Self) {
        self.popped = now.popped;
        if self.due == now.due {
            self.reminded = now.reminded;
        }
    }
}

/// Which way a step is played.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Way {
    Back,
    Forward,
}

impl Way {
    fn pick<'a, T>(self, before: &'a T, after: &'a T) -> &'a T {
        match self {
            Way::Back => before,
            Way::Forward => after,
        }
    }
}

/// What happened to one list.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(bound(serialize = "T: Serialize", deserialize = "T: Deserialize<'de>"))]
pub struct Edits<T> {
    /// Items that changed, came or went, by id: before and after (none
    /// where it wasn't there).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub changed: Vec<(Id, Option<T>, Option<T>)>,
    /// Where items came, went or moved to: the index before and after.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub placed: Vec<(Id, Option<usize>, Option<usize>)>,
    /// The whole list before and after, when its ids aren't one each.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub whole: Option<(Vec<T>, Vec<T>)>,
}

impl<T> Default for Edits<T> {
    fn default() -> Self {
        Edits { changed: Vec::new(), placed: Vec::new(), whole: None }
    }
}

impl<T: Keyed> Edits<T> {
    pub fn is_empty(&self) -> bool {
        self.changed.is_empty() && self.placed.is_empty() && self.whole.is_none()
    }

    /// From `before` to `after`.
    pub fn diff(before: &[T], after: &[T]) -> Self {
        if before == after {
            return Edits::default();
        }
        let Some(placed) = places(before, after) else {
            return Edits { whole: Some((before.to_vec(), after.to_vec())), ..Edits::default() };
        };
        let now: HashMap<Id, &T> = after.iter().map(|x| (x.key(), x)).collect();
        let was: HashSet<Id> = before.iter().map(Keyed::key).collect();
        let mut changed = Vec::new();
        for x in before {
            match now.get(&x.key()) {
                Some(y) if y.same(x) => {}
                y => changed.push((x.key(), Some(x.clone()), y.map(|y| (*y).clone()))),
            }
        }
        for y in after.iter().filter(|y| !was.contains(&y.key())) {
            changed.push((y.key(), None, Some(y.clone())));
        }
        Edits { changed, placed, whole: None }
    }

    /// Puts `list` as it was before (`Way::Back`) or after. Ids that aren't
    /// where the step expects are passed over rather than trusted.
    pub fn apply(&self, list: &mut Vec<T>, way: Way) {
        if let Some((before, after)) = &self.whole {
            *list = way.pick(before, after).clone();
            return;
        }
        if self.is_empty() {
            return;
        }
        let target: HashMap<Id, &Option<T>> =
            self.changed.iter().map(|(id, before, after)| (*id, way.pick(before, after))).collect();
        // Lift out what comes, goes or moves; a moved item is put back.
        let placed: HashSet<Id> = self.placed.iter().map(|p| p.0).collect();
        let mut lifted: HashMap<Id, T> = HashMap::new();
        if !placed.is_empty() {
            let mut rest = Vec::with_capacity(list.len());
            for x in list.drain(..) {
                if placed.contains(&x.key()) {
                    lifted.insert(x.key(), x);
                } else {
                    rest.push(x);
                }
            }
            *list = rest;
        }
        // What stays where it is changes in place.
        let at: HashMap<Id, usize> = list.iter().enumerate().map(|(i, x)| (x.key(), i)).collect();
        let mut gone = HashSet::new();
        for (id, to) in &target {
            if placed.contains(id) {
                continue;
            }
            match (at.get(id), to) {
                (Some(&i), Some(v)) => {
                    let mut v = v.clone();
                    v.keep(&list[i]);
                    list[i] = v;
                }
                (Some(_), None) => {
                    gone.insert(*id);
                }
                (None, Some(v)) => list.push(v.clone()),
                (None, None) => {}
            }
        }
        if !gone.is_empty() {
            list.retain(|x| !gone.contains(&x.key()));
        }
        // Then what comes or moves goes in at its place, front to back.
        let mut put: Vec<(usize, T)> = self
            .placed
            .iter()
            .filter_map(|(id, before, after)| {
                let i = (*way.pick(before, after))?;
                let item = match (target.get(id), lifted.remove(id)) {
                    (Some(Some(to)), Some(now)) => {
                        let mut to = to.clone();
                        to.keep(&now);
                        to
                    }
                    (Some(to), _) => (*to).clone()?,
                    (None, now) => now?,
                };
                Some((i, item))
            })
            .collect();
        if put.is_empty() {
            return;
        }
        put.sort_by_key(|p| p.0);
        let mut out = Vec::with_capacity(list.len() + put.len());
        let mut rest = std::mem::take(list).into_iter();
        let mut put = put.into_iter().peekable();
        loop {
            if put.peek().is_some_and(|p| p.0 <= out.len()) {
                out.extend(put.next().map(|p| p.1));
                continue;
            }
            match rest.next() {
                Some(x) => out.push(x),
                None => {
                    out.extend(put.map(|p| p.1));
                    break;
                }
            }
        }
        *list = out;
    }

    /// `list` has the items this step leaves on the `way` side, and not the
    /// ones it takes away.
    fn fits(&self, list: &[T], way: Way) -> bool {
        if let Some((before, after)) = &self.whole {
            let want: Vec<Id> = way.pick(before, after).iter().map(Keyed::key).collect();
            return list.iter().map(Keyed::key).eq(want);
        }
        let ids: HashSet<Id> = list.iter().map(Keyed::key).collect();
        self.changed.iter().all(|(id, before, after)| way.pick(before, after).is_some() == ids.contains(id))
    }
}

/// Where items came, went or moved between two lists: those only before,
/// those only after, and of the rest, those off a longest run kept in
/// order. None when either list has an id twice.
fn places<T: Keyed>(before: &[T], after: &[T]) -> Option<Vec<(Id, Option<usize>, Option<usize>)>> {
    let was: HashMap<Id, usize> = before.iter().enumerate().map(|(i, x)| (x.key(), i)).collect();
    let now: HashMap<Id, usize> = after.iter().enumerate().map(|(i, x)| (x.key(), i)).collect();
    if was.len() != before.len() || now.len() != after.len() {
        return None;
    }
    let mut out = Vec::new();
    let mut both = Vec::new();
    for (i, x) in before.iter().enumerate() {
        match now.get(&x.key()) {
            Some(&j) => both.push((x.key(), i, j)),
            None => out.push((x.key(), Some(i), None)),
        }
    }
    for (j, y) in after.iter().enumerate() {
        if !was.contains_key(&y.key()) {
            out.push((y.key(), None, Some(j)));
        }
    }
    let keep = longest_rising(&both.iter().map(|b| b.2).collect::<Vec<_>>());
    for (k, &(id, i, j)) in both.iter().enumerate() {
        if !keep[k] {
            out.push((id, Some(i), Some(j)));
        }
    }
    Some(out)
}

/// Marks one longest rising run in `xs` (its members need not be next to
/// each other).
fn longest_rising(xs: &[usize]) -> Vec<bool> {
    // For each run length, the index of the smallest value ending one.
    let mut ends: Vec<usize> = Vec::new();
    let mut prev = vec![usize::MAX; xs.len()];
    for (i, &x) in xs.iter().enumerate() {
        let n = ends.partition_point(|&e| xs[e] < x);
        if n > 0 {
            prev[i] = ends[n - 1];
        }
        if n == ends.len() {
            ends.push(i);
        } else {
            ends[n] = i;
        }
    }
    let mut keep = vec![false; xs.len()];
    let mut i = ends.last().copied().unwrap_or(usize::MAX);
    while i != usize::MAX {
        keep[i] = true;
        i = prev[i];
    }
    keep
}

/// What changed on one sheet that's there before and after.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SheetChange {
    pub id: Id,
    /// Its name, colour, backdrop and the rest, before and after (with no
    /// notes in them).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub head: Option<(Category, Category)>,
    #[serde(default, skip_serializing_if = "Edits::is_empty")]
    pub stickers: Edits<Sticker>,
    #[serde(default, skip_serializing_if = "Edits::is_empty")]
    pub strokes: Edits<Stroke>,
    #[serde(default, skip_serializing_if = "Edits::is_empty")]
    pub labels: Edits<Label>,
    #[serde(default, skip_serializing_if = "Edits::is_empty")]
    pub links: Edits<Link>,
    #[serde(default, skip_serializing_if = "Edits::is_empty")]
    pub frames: Edits<Frame>,
}

impl SheetChange {
    fn diff(before: &Category, after: &Category) -> Option<Self> {
        let c = SheetChange {
            id: after.id,
            head: (!same_head(before, after)).then(|| (head(before), head(after))),
            stickers: Edits::diff(&before.stickers, &after.stickers),
            strokes: Edits::diff(&before.strokes, &after.strokes),
            labels: Edits::diff(&before.labels, &after.labels),
            links: Edits::diff(&before.links, &after.links),
            frames: Edits::diff(&before.frames, &after.frames),
        };
        let empty = c.head.is_none()
            && c.stickers.is_empty()
            && c.strokes.is_empty()
            && c.labels.is_empty()
            && c.links.is_empty()
            && c.frames.is_empty();
        (!empty).then_some(c)
    }

    fn apply(&self, c: &mut Category, way: Way) {
        if let Some((before, after)) = &self.head {
            put_head(c, way.pick(before, after));
        }
        self.stickers.apply(&mut c.stickers, way);
        self.strokes.apply(&mut c.strokes, way);
        self.labels.apply(&mut c.labels, way);
        self.links.apply(&mut c.links, way);
        self.frames.apply(&mut c.frames, way);
    }

    fn fits(&self, c: &Category, way: Way) -> bool {
        self.stickers.fits(&c.stickers, way)
            && self.strokes.fits(&c.strokes, way)
            && self.labels.fits(&c.labels, way)
            && self.links.fits(&c.links, way)
            && self.frames.fits(&c.frames, way)
    }
}

/// A sheet without its contents. Every field is named, so a new one has to
/// be thought about here.
fn head(c: &Category) -> Category {
    let Category {
        id,
        name,
        blurb,
        accent,
        camera,
        stickers: _,
        strokes: _,
        labels: _,
        links: _,
        frames: _,
        inbox,
        pinned,
        icon,
        opened,
        edited,
        backdrop,
    } = c;
    Category {
        id: *id,
        name: name.clone(),
        blurb: blurb.clone(),
        accent: *accent,
        camera: *camera,
        stickers: Vec::new(),
        strokes: Vec::new(),
        labels: Vec::new(),
        links: Vec::new(),
        frames: Vec::new(),
        inbox: *inbox,
        pinned: *pinned,
        icon: icon.clone(),
        opened: *opened,
        edited: *edited,
        backdrop: backdrop.clone(),
    }
}

/// The same sheet but for its contents, its camera and when it was opened.
fn same_head(a: &Category, b: &Category) -> bool {
    a.id == b.id
        && a.name == b.name
        && a.blurb == b.blurb
        && a.accent == b.accent
        && a.inbox == b.inbox
        && a.pinned == b.pinned
        && a.icon == b.icon
        && a.edited == b.edited
        && a.backdrop == b.backdrop
}

/// Takes `h`'s name, colour and the rest, keeping `c`'s camera.
fn put_head(c: &mut Category, h: &Category) {
    c.name = h.name.clone();
    c.blurb = h.blurb.clone();
    c.accent = h.accent;
    c.inbox = h.inbox;
    c.pinned = h.pinned;
    c.icon = h.icon.clone();
    c.edited = h.edited;
    c.backdrop = h.backdrop.clone();
}

/// The board as an edit found it, for its undo step: copies of the sheets
/// the edit can change and what's board-wide. A press on the sheet or
/// typing into a note can only change the open sheet, so only that one is
/// copied (a drag that drops notes onto another sheet keeps that sheet
/// too, before it changes); copying the whole board on every press was the
/// cost of a click on a big board.
#[derive(Clone, Debug)]
pub struct Before {
    sheets: Vec<Category>,
    active: usize,
    next_id: Id,
}

impl Before {
    /// The open sheet as it is now.
    pub fn open(board: &Board) -> Self {
        Before { sheets: board.categories.get(board.active).cloned().into_iter().collect(), active: board.active, next_id: board.next_id }
    }

    /// Note `id` as it was, if its sheet was kept.
    pub fn note(&self, id: Id) -> Option<&Sticker> {
        self.sheets.iter().find_map(|c| c.sticker(id))
    }

    /// Keeps sheet `ix` as it is now too; call it before changing that sheet.
    pub fn keep(&mut self, board: &Board, ix: usize) {
        if let Some(c) = board.categories.get(ix) {
            if !self.sheets.iter().any(|s| s.id == c.id) {
                self.sheets.push(c.clone());
            }
        }
    }

    fn sheet(&self, id: Id) -> Option<&Category> {
        self.sheets.iter().find(|c| c.id == id)
    }

    /// Takes on what isn't an edit from `board`: which sheet is open, where
    /// the sheets are scrolled to, and the notes' order when a click only
    /// brought one to the front.
    pub fn settle(&mut self, board: &Board) {
        self.active = board.active;
        for c in &mut self.sheets {
            let Some(now) = board.categories.iter().find(|n| n.id == c.id) else { continue };
            c.camera = now.camera;
            if c.stickers.len() == now.stickers.len() {
                let was: HashMap<Id, &Sticker> = c.stickers.iter().map(|s| (s.id, s)).collect();
                if now.stickers.iter().all(|s| was.get(&s.id).is_some_and(|w| *w == s)) {
                    c.stickers = now.stickers.clone();
                }
            }
        }
    }

    /// Something it holds is different in `board`, undoable or not.
    pub fn differs(&self, board: &Board) -> bool {
        board.active != self.active
            || board.next_id != self.next_id
            || self.sheets.iter().any(|c| board.categories.iter().find(|n| n.id == c.id) != Some(c))
    }
}

/// One undo step.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Change {
    /// Sheets that came or went (whole), and where sheets came, went or
    /// moved to.
    #[serde(default, skip_serializing_if = "Edits::is_empty")]
    pub sheets: Edits<Category>,
    /// What changed inside sheets there before and after.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub inside: Vec<SheetChange>,
    /// The Trash.
    #[serde(default, skip_serializing_if = "Edits::is_empty")]
    pub bin: Edits<Binned>,
    /// The sheet showing, before and after.
    pub active: (usize, usize),
    /// Ids handed out by then. It never goes back, so no id is used twice.
    pub next_id: Id,
}

impl Change {
    /// What changed from `before` to `after`.
    pub fn diff(before: &Board, after: &Board) -> Self {
        let mut sheets = match places(&before.categories, &after.categories) {
            Some(placed) => Edits { placed, ..Edits::default() },
            None => Edits { whole: Some((before.categories.clone(), after.categories.clone())), ..Edits::default() },
        };
        let mut inside = Vec::new();
        if sheets.whole.is_none() {
            let was: HashMap<Id, &Category> = before.categories.iter().map(|c| (c.id, c)).collect();
            let now: HashSet<Id> = after.categories.iter().map(|c| c.id).collect();
            for c in &before.categories {
                if !now.contains(&c.id) {
                    sheets.changed.push((c.id, Some(c.clone()), None));
                }
            }
            for c in &after.categories {
                match was.get(&c.id) {
                    Some(b) => inside.extend(SheetChange::diff(b, c)),
                    None => sheets.changed.push((c.id, None, Some(c.clone()))),
                }
            }
        }
        Change {
            sheets,
            inside,
            bin: Edits::diff(&before.bin, &after.bin),
            active: (before.active, after.active),
            next_id: before.next_id.max(after.next_id),
        }
    }

    /// What changed from `before` to `after` in the sheets `before` kept
    /// (the rest weren't changed by the edit it was taken for).
    pub fn since(before: &Before, after: &Board) -> Self {
        let inside = before
            .sheets
            .iter()
            .filter_map(|c| after.categories.iter().find(|n| n.id == c.id).and_then(|now| SheetChange::diff(c, now)))
            .collect();
        Change {
            sheets: Edits::default(),
            inside,
            bin: Edits::default(),
            active: (before.active, after.active),
            next_id: before.next_id.max(after.next_id),
        }
    }

    /// As `restamp`, from the sheets `before` kept.
    pub fn restamp_since(&mut self, before: &Before, after: &Board, id: Id) {
        self.restamp_was(before.sheet(id), after, id);
    }

    /// Takes in a new `edited` time on sheet `id`, set on `after` once the
    /// step was made, so the step needn't be worked out again.
    pub fn restamp(&mut self, before: &Board, after: &Board, id: Id) {
        self.restamp_was(before.categories.iter().find(|c| c.id == id), after, id);
    }

    fn restamp_was(&mut self, was: Option<&Category>, after: &Board, id: Id) {
        let Some(now) = after.categories.iter().find(|c| c.id == id) else { return };
        if let Some((_, after)) = &mut self.sheets.whole {
            if let Some(c) = after.iter_mut().find(|c| c.id == id) {
                c.edited = now.edited;
            }
            return;
        }
        if let Some((_, _, Some(c))) = self.sheets.changed.iter_mut().find(|c| c.0 == id) {
            c.edited = now.edited;
            return;
        }
        let Some(was) = was else { return };
        let changed = (!same_head(was, now)).then(|| (head(was), head(now)));
        match self.inside.iter_mut().find(|s| s.id == id) {
            Some(s) => s.head = changed,
            None if changed.is_some() => self.inside.push(SheetChange {
                id,
                head: changed,
                stickers: Edits::default(),
                strokes: Edits::default(),
                labels: Edits::default(),
                links: Edits::default(),
                frames: Edits::default(),
            }),
            None => {}
        }
    }

    /// Nothing to undo: only the view or the settings changed.
    pub fn is_empty(&self) -> bool {
        self.sheets.is_empty() && self.inside.is_empty() && self.bin.is_empty()
    }

    /// Puts the board as it was before this step (`Way::Back`) or after.
    pub fn apply(&self, board: &mut Board, way: Way) {
        self.sheets.apply(&mut board.categories, way);
        for s in &self.inside {
            if let Some(c) = board.categories.iter_mut().find(|c| c.id == s.id) {
                s.apply(c, way);
            }
        }
        self.bin.apply(&mut board.bin, way);
        if board.categories.is_empty() {
            // Not a state Folio is ever in; a step that doesn't fit the
            // board could leave it so.
            board.next_id = board.next_id.max(self.next_id);
            board.add_sheet("My notes");
        }
        let (before, after) = self.active;
        board.active = (*way.pick(&before, &after)).min(board.categories.len() - 1);
        board.next_id = board.next_id.max(self.next_id);
    }

    /// `board` looks like it did after this step: the sheets and items it
    /// made are there and the ones it took away aren't. Undo steps kept
    /// from an earlier run are dropped when the last one doesn't fit.
    pub fn fits(&self, board: &Board) -> bool {
        self.sheets.fits(&board.categories, Way::Forward)
            && self.bin.fits(&board.bin, Way::Forward)
            && self.inside.iter().all(|s| board.categories.iter().find(|c| c.id == s.id).is_some_and(|c| s.fits(c, Way::Forward)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Ink, Rect};

    fn note(id: Id, x: f32) -> Sticker {
        Sticker { id, rect: Rect { x, y: 0., w: 200., h: 140. }, title: format!("Note {id}"), ..Default::default() }
    }

    /// A board with one sheet of `n` notes, ids 100 up.
    fn board(n: u64) -> Board {
        let mut b = Board::empty();
        b.sheet_mut().stickers = (0..n).map(|i| note(100 + i, i as f32 * 10.)).collect();
        b.next_id = 100 + n;
        b
    }

    #[test]
    fn a_copy_of_the_open_sheet_makes_the_same_step() {
        let mut b = board(50);
        b.add_sheet("Other");
        b.active = 0;
        let whole = b.clone();
        let mut before = Before::open(&b);
        assert_eq!(before.sheets.len(), 1, "only the open sheet is copied");

        // A note moved, one added: the same step either way, and it undoes.
        b.sheet_mut().stickers[3].rect.x += 40.;
        let id = b.fresh_id();
        b.sheet_mut().stickers.push(note(id, 5.));
        let step = Change::since(&before, &b);
        assert_eq!(step, Change::diff(&whole, &b));
        let mut back = b.clone();
        step.apply(&mut back, Way::Back);
        assert_eq!(content(&back), content(&whole));

        // Notes dropped onto another sheet: that sheet is kept before it changes.
        let after_move = b.clone();
        before = Before::open(&b);
        before.keep(&b, 1);
        assert!(b.move_note(id, 1));
        let step = Change::since(&before, &b);
        assert_eq!(step, Change::diff(&after_move, &b));
        step.apply(&mut b, Way::Back);
        assert_eq!(content(&b), content(&after_move));
    }

    #[test]
    fn a_pan_or_a_click_to_the_front_is_no_step() {
        let mut b = board(5);
        let mut before = Before::open(&b);
        b.sheet_mut().camera.pan.x += 300.;
        b.sheet_mut().bring_to_front(101);
        before.settle(&b);
        assert!(Change::since(&before, &b).is_empty());
        assert!(!before.differs(&b), "nothing to save either");
        b.sheet_mut().stickers[0].title = "Changed".into();
        assert!(before.differs(&b));
    }

    /// The sheets as undo sees them: without cameras.
    fn content(b: &Board) -> Vec<Category> {
        b.categories.iter().cloned().map(|mut c| {
            c.camera = Default::default();
            c
        }).collect()
    }

    /// The step from `before` to `after` takes each to the other, and
    /// survives being written down.
    fn round_trip(before: &Board, after: &Board) -> Change {
        let step = Change::diff(before, after);
        let text = serde_json::to_string(&step).unwrap();
        let step: Change = serde_json::from_str(&text).unwrap();
        let mut b = after.clone();
        step.apply(&mut b, Way::Back);
        assert_eq!(content(&b), content(before));
        assert_eq!(b.bin, before.bin);
        assert_eq!(b.active, before.active);
        step.apply(&mut b, Way::Forward);
        assert_eq!(content(&b), content(after));
        assert_eq!(b.bin, after.bin);
        assert_eq!(b.active, after.active);
        assert!(step.fits(after));
        step
    }

    #[test]
    fn moving_one_note_among_two_thousand_is_a_small_step() {
        let before = board(2000);
        let mut after = before.clone();
        after.sheet_mut().stickers[1234].rect.x += 300.;
        let step = round_trip(&before, &after);
        let size = serde_json::to_string(&step).unwrap().len();
        let whole = serde_json::to_string(&before).unwrap().len();
        assert!(size < 1_000, "a moved note took {size} bytes");
        assert!(whole > 200 * size);
    }

    #[test]
    fn notes_come_go_and_change_order() {
        let before = board(8);
        let mut after = before.clone();
        let c = after.sheet_mut();
        c.stickers.remove(2);
        c.stickers.insert(5, note(900, 1.));
        c.bring_to_front(101);
        c.stickers[0].body = "edited".into();
        c.stickers.push(note(901, 2.));
        after.next_id = 902;
        let step = round_trip(&before, &after);
        // The note brought to the front moved; the rest kept their order.
        let moved: Vec<Id> = step.inside[0].stickers.placed.iter().filter(|p| p.1.is_some() && p.2.is_some()).map(|p| p.0).collect();
        assert_eq!(moved, vec![101]);
    }

    #[test]
    fn reversing_a_list_round_trips() {
        let before = board(30);
        let mut after = before.clone();
        after.sheet_mut().stickers.reverse();
        round_trip(&before, &after);
    }

    #[test]
    fn every_kind_of_thing_on_a_sheet() {
        let mut before = board(3);
        let c = before.sheet_mut();
        c.links.push(Link { id: 50, from: 100, to: 101, ink: Ink::Forest });
        c.labels.push(crate::model::Label::new(51, crate::model::v2(0., 0.), Ink::Graphite));
        let mut after = before.clone();
        let c = after.sheet_mut();
        c.links.clear();
        c.labels[0].text = "Hello".into();
        c.name = "Renamed".into();
        c.camera.zoom = 2.;
        let step = round_trip(&before, &after);
        // The camera isn't content: undo leaves it where it is.
        let mut b = after.clone();
        step.apply(&mut b, Way::Back);
        assert_eq!(b.categories[0].camera.zoom, 2.);
        assert_eq!(b.categories[0].name, before.categories[0].name);
    }

    #[test]
    fn sheets_come_go_and_move_whole() {
        let mut before = board(4);
        before.add_sheet("Second");
        before.add_sheet("Third");
        let mut after = before.clone();
        let gone = after.categories[0].id;
        after.bin_sheet(0, false, 1_000);
        after.categories.swap(0, 1);
        after.add_sheet("Fourth");
        after.active = 2;
        let step = round_trip(&before, &after);
        assert!(step.bin.changed.iter().any(|c| c.1.is_none()));
        assert!(step.sheets.changed.iter().any(|c| c.0 == gone && c.2.is_none()));
    }

    #[test]
    fn only_the_view_is_nothing_to_undo() {
        let before = board(5);
        let mut after = before.clone();
        after.sheet_mut().camera.pan.x = 40.;
        after.sheet_mut().opened = 99;
        after.settings.user_name = "Ada".into();
        after.at_home = !before.at_home;
        assert!(Change::diff(&before, &after).is_empty());
    }

    #[test]
    fn ids_twice_fall_back_to_the_whole_list() {
        let mut before = board(3);
        before.sheet_mut().stickers.push(note(100, 9.));
        let mut after = before.clone();
        after.sheet_mut().stickers[3].body = "twin".into();
        let step = round_trip(&before, &after);
        assert!(step.inside[0].stickers.whole.is_some());
    }

    #[test]
    fn ids_never_go_back() {
        let before = board(2);
        let mut after = before.clone();
        after.sheet_mut().stickers.push(note(500, 0.));
        after.next_id = 501;
        let step = Change::diff(&before, &after);
        let mut b = after.clone();
        step.apply(&mut b, Way::Back);
        assert_eq!(b.next_id, 501);
    }

    #[test]
    fn a_step_that_doesnt_fit_is_noticed() {
        let before = board(3);
        let mut after = before.clone();
        after.sheet_mut().stickers.push(note(700, 0.));
        let step = Change::diff(&before, &after);
        assert!(step.fits(&after));
        assert!(!step.fits(&before));
    }

    #[test]
    fn a_later_edited_time_is_the_same_as_working_it_out_again() {
        let before = board(4);
        let mut after = before.clone();
        after.sheet_mut().stickers[2].body = "changed".into();
        let mut step = Change::diff(&before, &after);
        after.sheet_mut().edited += 60;
        step.restamp(&before, &after, after.categories[0].id);
        assert_eq!(step, Change::diff(&before, &after));
        // A sheet made in this step carries the time too.
        let mut later = after.clone();
        let id = later.add_sheet("New");
        let mut step = Change::diff(&after, &later);
        later.categories.last_mut().unwrap().edited = 5;
        step.restamp(&after, &later, id);
        assert_eq!(step, Change::diff(&after, &later));
    }

    #[test]
    fn desktop_windows_and_shown_reminders_stay_out_of_undo() {
        let mut before = board(3);
        before.sheet_mut().stickers[0].due = Some(10);
        let mut after = before.clone();
        after.sheet_mut().stickers[1].body = "edited".into();
        // Mid-edit, the clock shows a reminder and a desktop window moves.
        after.sheet_mut().stickers[0].reminded = true;
        after.sheet_mut().stickers[2].popped = Some(crate::model::Popped { x: 1., y: 2., w: 3., h: 4., see_through: false });
        let step = Change::diff(&before, &after);
        assert_eq!(step.inside[0].stickers.changed.len(), 1);
        let mut b = after.clone();
        step.apply(&mut b, Way::Back);
        assert_eq!(b.categories[0].stickers[1].body, "");
        assert!(b.categories[0].stickers[0].reminded);
        assert!(b.categories[0].stickers[2].popped.is_some());
        // A new time for the reminder is content: undo brings the old one
        // back, shown or not as it was.
        let mut later = b.clone();
        later.sheet_mut().stickers[0].due = Some(99);
        later.sheet_mut().stickers[0].reminded = false;
        let step = Change::diff(&b, &later);
        let mut c = later.clone();
        step.apply(&mut c, Way::Back);
        assert_eq!((c.categories[0].stickers[0].due, c.categories[0].stickers[0].reminded), (Some(10), true));
    }

    #[test]
    fn longest_rising_run() {
        let keep = longest_rising(&[3, 0, 1, 4, 2, 5]);
        let run: Vec<usize> = [3, 0, 1, 4, 2, 5].iter().zip(&keep).filter(|p| *p.1).map(|p| *p.0).collect();
        assert_eq!(run.len(), 4);
        assert!(run.windows(2).all(|w| w[0] < w[1]));
        assert!(longest_rising(&[]).is_empty());
    }
}
