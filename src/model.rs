//! The document: categories (sheets), and on each sheet its stickers, ink
//! strokes, labels written on the canvas, and links between stickers.
//! Plain data and geometry only; nothing here knows about GPUI.
//!
//! Coordinates are world units on an unbounded sheet. A sheet's camera maps
//! them to the screen: `screen = world * zoom + pan`.

use serde::{Deserialize, Serialize};

pub type Id = u64;

pub const FORMAT_VERSION: u32 = 1;
pub const MIN_ZOOM: f32 = 0.25;
pub const MAX_ZOOM: f32 = 3.0;
pub const MIN_STICKER: (f32, f32) = (140., 90.);

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Vec2 {
    pub x: f32,
    pub y: f32,
}

pub const fn v2(x: f32, y: f32) -> Vec2 {
    Vec2 { x, y }
}

impl Vec2 {
    pub fn sub(self, o: Vec2) -> Vec2 {
        v2(self.x - o.x, self.y - o.y)
    }
    pub fn add(self, o: Vec2) -> Vec2 {
        v2(self.x + o.x, self.y + o.y)
    }
    pub fn len(self) -> f32 {
        self.x.hypot(self.y)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

impl Rect {
    pub fn contains(&self, p: Vec2) -> bool {
        p.x >= self.x && p.x <= self.x + self.w && p.y >= self.y && p.y <= self.y + self.h
    }
    pub fn intersects(&self, o: &Rect) -> bool {
        self.x < o.x + o.w && o.x < self.x + self.w && self.y < o.y + o.h && o.y < self.y + self.h
    }
    pub fn center(&self) -> Vec2 {
        v2(self.x + self.w / 2., self.y + self.h / 2.)
    }
    pub fn union(&self, o: &Rect) -> Rect {
        let x = self.x.min(o.x);
        let y = self.y.min(o.y);
        Rect {
            x,
            y,
            w: (self.x + self.w).max(o.x + o.w) - x,
            h: (self.y + self.h).max(o.y + o.h) - y,
        }
    }
}

/// A sticker's paper. Each has a light and a dark rendering in the theme.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Paper {
    Butter,
    Sage,
    Sky,
    Blush,
    Lilac,
    Plain,
}

impl Paper {
    pub const ALL: [Paper; 6] = [
        Paper::Butter,
        Paper::Sage,
        Paper::Sky,
        Paper::Blush,
        Paper::Lilac,
        Paper::Plain,
    ];
}

/// Ink for strokes, labels and links.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Ink {
    Graphite,
    Forest,
    Coral,
    Cobalt,
    Marigold,
}

impl Ink {
    pub const ALL: [Ink; 5] = [Ink::Graphite, Ink::Forest, Ink::Coral, Ink::Cobalt, Ink::Marigold];
}

/// The type size of a sticker's text.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum TextSize {
    Small,
    #[default]
    Medium,
    Large,
}

impl TextSize {
    pub const ALL: [TextSize; 3] = [TextSize::Small, TextSize::Medium, TextSize::Large];
    pub fn label(self) -> &'static str {
        match self {
            TextSize::Small => "S",
            TextSize::Medium => "M",
            TextSize::Large => "L",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Sticker {
    pub id: Id,
    pub rect: Rect,
    pub paper: Paper,
    pub title: String,
    /// Markdown: GitHub tables, task lists, and `<mark color="…">` for
    /// coloured highlights.
    pub body: String,
    /// A picture shown above the text: a file in the media folder.
    #[serde(default)]
    pub image: Option<String>,
    #[serde(default)]
    pub text_size: TextSize,
    /// Unix seconds of the last edit.
    #[serde(default)]
    pub edited: u64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Stroke {
    pub id: Id,
    pub ink: Ink,
    pub width: f32,
    /// A highlighter: wide and translucent.
    #[serde(default)]
    pub marker: bool,
    pub points: Vec<Vec2>,
}

impl Stroke {
    pub fn bounds(&self) -> Rect {
        let pad = self.width / 2.;
        let (mut x0, mut y0, mut x1, mut y1) = (f32::MAX, f32::MAX, f32::MIN, f32::MIN);
        for p in &self.points {
            x0 = x0.min(p.x);
            y0 = y0.min(p.y);
            x1 = x1.max(p.x);
            y1 = y1.max(p.y);
        }
        if self.points.is_empty() {
            return Rect { x: 0., y: 0., w: 0., h: 0. };
        }
        Rect { x: x0 - pad, y: y0 - pad, w: x1 - x0 + 2. * pad, h: y1 - y0 + 2. * pad }
    }

    /// Whether `p` is within `radius` of the line.
    pub fn hit(&self, p: Vec2, radius: f32) -> bool {
        let r = radius + self.width / 2.;
        match self.points.as_slice() {
            [] => false,
            [a] => a.sub(p).len() <= r,
            pts => pts.windows(2).any(|w| segment_distance(p, w[0], w[1]) <= r),
        }
    }
}

/// Text written straight on the sheet.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Label {
    pub id: Id,
    pub at: Vec2,
    pub text: String,
    pub ink: Ink,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Link {
    pub id: Id,
    pub from: Id,
    pub to: Id,
    pub ink: Ink,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Camera {
    pub pan: Vec2,
    pub zoom: f32,
}

impl Default for Camera {
    fn default() -> Self {
        Camera { pan: v2(48., 120.), zoom: 1. }
    }
}

impl Camera {
    pub fn to_screen(&self, p: Vec2) -> Vec2 {
        v2(p.x * self.zoom + self.pan.x, p.y * self.zoom + self.pan.y)
    }
    pub fn to_world(&self, s: Vec2) -> Vec2 {
        v2((s.x - self.pan.x) / self.zoom, (s.y - self.pan.y) / self.zoom)
    }
    /// The world area a viewport of `w`×`h` shows.
    pub fn visible(&self, w: f32, h: f32) -> Rect {
        let a = self.to_world(v2(0., 0.));
        Rect { x: a.x, y: a.y, w: w / self.zoom, h: h / self.zoom }
    }
    /// Zooms by `factor`, keeping the world point under `anchor` (screen) still.
    pub fn zoom_at(&mut self, anchor: Vec2, factor: f32) {
        let before = self.to_world(anchor);
        self.zoom = (self.zoom * factor).clamp(MIN_ZOOM, MAX_ZOOM);
        self.pan = v2(anchor.x - before.x * self.zoom, anchor.y - before.y * self.zoom);
    }
    /// Frames `r` in a `w`×`h` viewport with `margin` pixels around it.
    pub fn fit(&mut self, r: Rect, w: f32, h: f32, margin: f32) {
        let zx = (w - 2. * margin).max(1.) / r.w.max(1.);
        let zy = (h - 2. * margin).max(1.) / r.h.max(1.);
        self.zoom = zx.min(zy).clamp(MIN_ZOOM, 1.25);
        let c = r.center();
        self.pan = v2(w / 2. - c.x * self.zoom, h / 2. - c.y * self.zoom);
    }
}

/// The dot in the sidebar beside a category.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Accent {
    Amber,
    Slate,
    Rose,
    Moss,
    Plum,
}

impl Accent {
    pub const ALL: [Accent; 5] = [Accent::Amber, Accent::Slate, Accent::Rose, Accent::Moss, Accent::Plum];
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Category {
    pub id: Id,
    pub name: String,
    #[serde(default)]
    pub blurb: String,
    pub accent: Accent,
    pub camera: Camera,
    /// Back to front.
    pub stickers: Vec<Sticker>,
    pub strokes: Vec<Stroke>,
    pub labels: Vec<Label>,
    pub links: Vec<Link>,
}

impl Category {
    pub fn new(id: Id, name: &str, accent: Accent) -> Self {
        Category {
            id,
            name: name.into(),
            blurb: String::new(),
            accent,
            camera: Camera::default(),
            stickers: Vec::new(),
            strokes: Vec::new(),
            labels: Vec::new(),
            links: Vec::new(),
        }
    }

    pub fn sticker(&self, id: Id) -> Option<&Sticker> {
        self.stickers.iter().find(|s| s.id == id)
    }
    pub fn sticker_mut(&mut self, id: Id) -> Option<&mut Sticker> {
        self.stickers.iter_mut().find(|s| s.id == id)
    }

    /// The front-most sticker under `p`.
    pub fn sticker_at(&self, p: Vec2) -> Option<Id> {
        self.stickers.iter().rev().find(|s| s.rect.contains(p)).map(|s| s.id)
    }

    /// The label whose text box (estimated from its length) holds `p`.
    pub fn label_at(&self, p: Vec2) -> Option<Id> {
        self.labels
            .iter()
            .rev()
            .find(|l| label_rect(l).contains(p))
            .map(|l| l.id)
    }

    pub fn bring_to_front(&mut self, id: Id) {
        if let Some(i) = self.stickers.iter().position(|s| s.id == id) {
            let s = self.stickers.remove(i);
            self.stickers.push(s);
        }
    }

    pub fn remove_sticker(&mut self, id: Id) {
        self.stickers.retain(|s| s.id != id);
        self.links.retain(|l| l.from != id && l.to != id);
    }

    /// Links `a` to `b`, or unlinks them when they already are. Returns
    /// whether a link now exists.
    pub fn toggle_link(&mut self, id: Id, a: Id, b: Id, ink: Ink) -> bool {
        if a == b {
            return false;
        }
        let same = |l: &Link| (l.from == a && l.to == b) || (l.from == b && l.to == a);
        if self.links.iter().any(same) {
            self.links.retain(|l| !same(l));
            false
        } else {
            self.links.push(Link { id, from: a, to: b, ink });
            true
        }
    }

    /// Removes every stroke within `radius` of `p`; true if any went.
    pub fn erase_at(&mut self, p: Vec2, radius: f32) -> bool {
        let before = self.strokes.len();
        self.strokes.retain(|s| !s.hit(p, radius));
        before != self.strokes.len()
    }

    /// Everything on the sheet, or None for an empty one.
    pub fn content_bounds(&self) -> Option<Rect> {
        let rects = self
            .stickers
            .iter()
            .map(|s| s.rect)
            .chain(self.strokes.iter().map(Stroke::bounds))
            .chain(self.labels.iter().map(label_rect));
        rects.reduce(|a, b| a.union(&b))
    }

    pub fn item_count(&self) -> usize {
        self.stickers.len()
    }
}

/// A label's box: 7 units per character at the 14-unit label size.
pub fn label_rect(l: &Label) -> Rect {
    let longest = l.text.lines().map(|s| s.chars().count()).max().unwrap_or(1).max(4);
    let lines = l.text.lines().count().max(1);
    Rect { x: l.at.x, y: l.at.y, w: longest as f32 * 7.4, h: lines as f32 * 22. }
}

/// The curve a link follows: from the edge of one rect toward the other,
/// leaving and arriving along the axis that separates them. Returns the
/// start, the two control points and the end.
pub fn link_curve(a: &Rect, b: &Rect) -> [Vec2; 4] {
    let (ca, cb) = (a.center(), b.center());
    let d = cb.sub(ca);
    let horizontal = d.x.abs() * a.h.max(b.h) >= d.y.abs() * a.w.max(b.w);
    if horizontal {
        let sx = d.x.signum();
        let p0 = v2(ca.x + sx * a.w / 2., ca.y);
        let p3 = v2(cb.x - sx * b.w / 2., cb.y);
        let k = ((p3.x - p0.x).abs() * 0.5).max(40.);
        [p0, v2(p0.x + sx * k, p0.y), v2(p3.x - sx * k, p3.y), p3]
    } else {
        let sy = d.y.signum();
        let p0 = v2(ca.x, ca.y + sy * a.h / 2.);
        let p3 = v2(cb.x, cb.y - sy * b.h / 2.);
        let k = ((p3.y - p0.y).abs() * 0.5).max(40.);
        [p0, v2(p0.x, p0.y + sy * k), v2(p3.x, p3.y - sy * k), p3]
    }
}

pub fn segment_distance(p: Vec2, a: Vec2, b: Vec2) -> f32 {
    let ab = b.sub(a);
    let len2 = ab.x * ab.x + ab.y * ab.y;
    if len2 == 0. {
        return p.sub(a).len();
    }
    let t = (((p.x - a.x) * ab.x + (p.y - a.y) * ab.y) / len2).clamp(0., 1.);
    p.sub(v2(a.x + ab.x * t, a.y + ab.y * t)).len()
}

/// Ramer–Douglas–Peucker: drops points that lie within `epsilon` of the line
/// through their neighbours, so a finished stroke keeps its shape at a
/// fraction of the points the mouse produced.
pub fn simplify(points: &[Vec2], epsilon: f32) -> Vec<Vec2> {
    if points.len() < 3 {
        return points.to_vec();
    }
    let mut keep = vec![false; points.len()];
    keep[0] = true;
    keep[points.len() - 1] = true;
    let mut stack = vec![(0, points.len() - 1)];
    while let Some((s, e)) = stack.pop() {
        let mut far = (0., s);
        for i in s + 1..e {
            let d = segment_distance(points[i], points[s], points[e]);
            if d > far.0 {
                far = (d, i);
            }
        }
        if far.0 > epsilon {
            keep[far.1] = true;
            stack.push((s, far.1));
            stack.push((far.1, e));
        }
    }
    points.iter().zip(keep).filter(|(_, k)| *k).map(|(p, _)| *p).collect()
}

/// The whole workspace, as saved.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Board {
    pub version: u32,
    pub categories: Vec<Category>,
    pub active: usize,
    #[serde(default)]
    pub dark: bool,
    pub next_id: Id,
}

impl Board {
    pub fn fresh_id(&mut self) -> Id {
        self.next_id += 1;
        self.next_id
    }
    pub fn sheet(&self) -> &Category {
        &self.categories[self.active.min(self.categories.len() - 1)]
    }
    pub fn sheet_mut(&mut self) -> &mut Category {
        let i = self.active.min(self.categories.len() - 1);
        &mut self.categories[i]
    }
    /// Adds a sheet after the others and makes it the active one.
    pub fn add_sheet(&mut self, name: &str) -> Id {
        let id = self.fresh_id();
        let accent = Accent::ALL[self.categories.len() % Accent::ALL.len()];
        self.categories.push(Category::new(id, name, accent));
        self.active = self.categories.len() - 1;
        id
    }
    /// Removes the active sheet; the last one can't go.
    pub fn remove_active_sheet(&mut self) -> bool {
        if self.categories.len() <= 1 {
            return false;
        }
        self.categories.remove(self.active);
        self.active = self.active.min(self.categories.len() - 1);
        true
    }
    /// Repairs what a hand-edited or older file might get wrong.
    pub fn normalize(&mut self) {
        if self.categories.is_empty() {
            *self = Board::empty();
        }
        self.active = self.active.min(self.categories.len() - 1);
        let max = self
            .categories
            .iter()
            .flat_map(|c| {
                std::iter::once(c.id)
                    .chain(c.stickers.iter().map(|s| s.id))
                    .chain(c.strokes.iter().map(|s| s.id))
                    .chain(c.labels.iter().map(|s| s.id))
                    .chain(c.links.iter().map(|s| s.id))
            })
            .max()
            .unwrap_or(0);
        self.next_id = self.next_id.max(max);
        for c in &mut self.categories {
            c.camera.zoom = c.camera.zoom.clamp(MIN_ZOOM, MAX_ZOOM);
            for s in &mut c.stickers {
                s.rect.w = s.rect.w.max(MIN_STICKER.0);
                s.rect.h = s.rect.h.max(MIN_STICKER.1);
            }
            let ids: Vec<Id> = c.stickers.iter().map(|s| s.id).collect();
            c.links.retain(|l| ids.contains(&l.from) && ids.contains(&l.to));
        }
    }
    pub fn empty() -> Board {
        let mut b = Board { version: FORMAT_VERSION, categories: Vec::new(), active: 0, dark: false, next_id: 0 };
        b.add_sheet("My first sheet");
        b
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sticker(id: Id, x: f32, y: f32) -> Sticker {
        Sticker {
            id,
            rect: Rect { x, y, w: 200., h: 150. },
            paper: Paper::Butter,
            title: String::new(),
            body: String::new(),
            image: None,
            text_size: TextSize::Medium,
            edited: 0,
        }
    }

    #[test]
    fn camera_round_trips_and_zooms_about_the_cursor() {
        let mut c = Camera { pan: v2(30., -12.), zoom: 1.5 };
        let p = v2(123., 45.);
        let back = c.to_world(c.to_screen(p));
        assert!((back.x - p.x).abs() < 1e-3 && (back.y - p.y).abs() < 1e-3);
        let anchor = v2(400., 300.);
        let under = c.to_world(anchor);
        c.zoom_at(anchor, 1.7);
        let still = c.to_world(anchor);
        assert!((under.x - still.x).abs() < 1e-3 && (under.y - still.y).abs() < 1e-3);
        c.zoom_at(anchor, 100.);
        assert_eq!(c.zoom, MAX_ZOOM);
    }

    #[test]
    fn front_most_sticker_wins_and_bring_to_front_reorders() {
        let mut c = Category::new(1, "t", Accent::Amber);
        c.stickers.push(sticker(2, 0., 0.));
        c.stickers.push(sticker(3, 100., 50.));
        assert_eq!(c.sticker_at(v2(150., 100.)), Some(3));
        c.bring_to_front(2);
        assert_eq!(c.sticker_at(v2(150., 100.)), Some(2));
        assert_eq!(c.sticker_at(v2(999., 999.)), None);
    }

    #[test]
    fn links_toggle_and_go_with_their_stickers() {
        let mut c = Category::new(1, "t", Accent::Amber);
        c.stickers.push(sticker(2, 0., 0.));
        c.stickers.push(sticker(3, 400., 0.));
        assert!(c.toggle_link(10, 2, 3, Ink::Forest));
        assert!(!c.toggle_link(11, 3, 2, Ink::Forest));
        assert!(c.links.is_empty());
        assert!(!c.toggle_link(12, 2, 2, Ink::Forest));
        c.toggle_link(13, 2, 3, Ink::Forest);
        c.remove_sticker(3);
        assert!(c.links.is_empty());
    }

    #[test]
    fn link_curves_leave_from_facing_edges() {
        let a = Rect { x: 0., y: 0., w: 100., h: 100. };
        let b = Rect { x: 300., y: 20., w: 100., h: 100. };
        let [p0, _, _, p3] = link_curve(&a, &b);
        assert_eq!(p0, v2(100., 50.));
        assert_eq!(p3, v2(300., 70.));
        let below = Rect { x: 10., y: 400., w: 100., h: 100. };
        let [q0, _, _, q3] = link_curve(&a, &below);
        assert_eq!(q0, v2(50., 100.));
        assert_eq!(q3, v2(60., 400.));
    }

    #[test]
    fn simplify_keeps_corners_and_drops_collinear_points() {
        let line: Vec<Vec2> = (0..50).map(|i| v2(i as f32, 0.)).collect();
        assert_eq!(simplify(&line, 0.5), vec![v2(0., 0.), v2(49., 0.)]);
        let corner = vec![v2(0., 0.), v2(5., 0.1), v2(10., 0.), v2(10., 5.), v2(10., 10.)];
        assert_eq!(simplify(&corner, 0.5), vec![v2(0., 0.), v2(10., 0.), v2(10., 10.)]);
    }

    #[test]
    fn eraser_removes_only_strokes_it_touches() {
        let mut c = Category::new(1, "t", Accent::Amber);
        c.strokes.push(Stroke { id: 2, ink: Ink::Graphite, width: 2., marker: false, points: vec![v2(0., 0.), v2(100., 0.)] });
        c.strokes.push(Stroke { id: 3, ink: Ink::Graphite, width: 2., marker: false, points: vec![v2(0., 50.), v2(100., 50.)] });
        assert!(c.erase_at(v2(50., 4.), 4.));
        assert_eq!(c.strokes.len(), 1);
        assert!(!c.erase_at(v2(50., 30.), 4.));
    }

    #[test]
    fn normalize_repairs_ids_sizes_and_dangling_links() {
        let mut b = Board::empty();
        b.next_id = 0;
        let c = b.sheet_mut();
        let mut s = sticker(40, 0., 0.);
        s.rect.w = 10.;
        c.stickers.push(s);
        c.links.push(Link { id: 41, from: 40, to: 99, ink: Ink::Forest });
        c.camera.zoom = 50.;
        b.active = 9;
        b.normalize();
        assert_eq!(b.next_id, 41);
        assert_eq!(b.active, 0);
        let c = b.sheet();
        assert!(c.links.is_empty());
        assert_eq!(c.stickers[0].rect.w, MIN_STICKER.0);
        assert_eq!(c.camera.zoom, MAX_ZOOM);
    }

    #[test]
    fn last_sheet_cannot_be_removed() {
        let mut b = Board::empty();
        assert!(!b.remove_active_sheet());
        b.add_sheet("two");
        assert_eq!(b.active, 1);
        assert!(b.remove_active_sheet());
        assert_eq!(b.active, 0);
    }

    #[test]
    fn fit_frames_content() {
        let mut c = Camera::default();
        let r = Rect { x: 1000., y: 1000., w: 400., h: 200. };
        c.fit(r, 800., 600., 40.);
        let tl = c.to_screen(v2(r.x, r.y));
        let br = c.to_screen(v2(r.x + r.w, r.y + r.h));
        assert!(tl.x >= 39. && br.x <= 761. && tl.y >= 0. && br.y <= 600.);
    }
}
