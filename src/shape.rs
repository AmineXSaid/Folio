//! Shapes drawn on a sheet: boxes, flowchart and UML symbols, lines,
//! arrows and line icons.
//! Each is a kind of outline fitted to a box, turned about the box's
//! centre. Plain geometry only; nothing here knows about GPUI. The board
//! paints these outlines and the PNG/PDF export writes them, so both draw
//! the same thing.

use std::f32::consts::{FRAC_PI_2, PI, TAU};

use serde::{Deserialize, Serialize};

use crate::icons::{self, Icon, IconLibrary, GRID};
use crate::model::{segment_distance, v2, Id, Ink, Paper, Rect, Vec2};

/// The smallest a shape's box can be made, in world units.
pub const MIN_SHAPE: f32 = 24.;
/// The type size of the words in a shape, and the space between them and
/// its edge, in world units.
pub const SHAPE_TEXT: f32 = 16.;
pub const TEXT_PAD: f32 = 10.;

/// Which outline a shape has.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Geom {
    // General.
    Rect,
    Rounded,
    Square,
    Ellipse,
    Circle,
    Diamond,
    Triangle,
    Hexagon,
    Parallelogram,
    Trapezoid,
    Pentagon,
    Octagon,
    Star,
    Cross,
    Heart,
    Cylinder,
    Cloud,
    Document,
    Note,
    Card,
    Callout,
    Cube,
    Step,
    Tape,
    // Flowchart.
    Terminator,
    Delay,
    ManualInput,
    OffPage,
    Predefined,
    ManualOp,
    Display,
    Collate,
    Sort,
    Merge,
    Or,
    SummingJunction,
    InternalStorage,
    LoopLimit,
    MultiDocument,
    SequentialData,
    // Lines and arrows.
    Line,
    Arrow,
    DoubleArrow,
    Elbow,
    CurvedArrow,
    BlockArrow,
    BlockDoubleArrow,
    Chevron,
    // UML.
    Actor,
    Class,
    Package,
    Component,
    /// A line icon (network, devices, people...).
    Icon(Icon),
}

/// One thing to pick in the shapes panel: a shape under the name its
/// library knows it by.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Entry {
    pub geom: Geom,
    pub name: &'static str,
    /// Other words it's found by.
    pub keywords: &'static str,
}

/// A family of shapes, as the panel shows them.
pub struct Library {
    pub id: &'static str,
    pub name: &'static str,
    pub entries: Vec<Entry>,
}

const fn e(geom: Geom, name: &'static str, keywords: &'static str) -> Entry {
    Entry { geom, name, keywords }
}

/// Every shape there is, family by family.
pub fn libraries() -> Vec<Library> {
    use Geom as G;
    let general = vec![
        e(G::Rect, "Rectangle", "box square process"),
        e(G::Rounded, "Rounded rectangle", "box card button"),
        e(G::Square, "Square", "box"),
        e(G::Ellipse, "Ellipse", "oval"),
        e(G::Circle, "Circle", "round dot"),
        e(G::Diamond, "Diamond", "rhombus decision"),
        e(G::Triangle, "Triangle", "extract"),
        e(G::Hexagon, "Hexagon", "preparation"),
        e(G::Parallelogram, "Parallelogram", "data input output"),
        e(G::Trapezoid, "Trapezoid", ""),
        e(G::Pentagon, "Pentagon", ""),
        e(G::Octagon, "Octagon", "stop"),
        e(G::Star, "Star", "favourite rating"),
        e(G::Cross, "Cross", "plus add"),
        e(G::Heart, "Heart", "love like"),
        e(G::Cylinder, "Cylinder", "database storage"),
        e(G::Cloud, "Cloud", "internet"),
        e(G::Document, "Document", "paper file"),
        e(G::Note, "Note", "folded corner"),
        e(G::Card, "Card", "punched card cut corner"),
        e(G::Callout, "Callout", "speech bubble comment"),
        e(G::Cube, "Cube", "box 3d"),
        e(G::Step, "Step", "chevron process"),
        e(G::Tape, "Tape", "punched tape banner"),
    ];
    let flow = vec![
        e(G::Rect, "Process", "step task"),
        e(G::Diamond, "Decision", "if choice"),
        e(G::Terminator, "Start / end", "terminator begin stop pill"),
        e(G::Parallelogram, "Data", "input output"),
        e(G::Predefined, "Predefined process", "subroutine"),
        e(G::Document, "Document", "report"),
        e(G::MultiDocument, "Documents", "multiple reports"),
        e(G::Cylinder, "Database", "stored data storage"),
        e(G::ManualInput, "Manual input", "keyboard"),
        e(G::ManualOp, "Manual operation", "trapezoid"),
        e(G::Hexagon, "Preparation", "setup"),
        e(G::Delay, "Delay", "wait"),
        e(G::Display, "Display", "screen output"),
        e(G::OffPage, "Off-page link", "connector"),
        e(G::Merge, "Merge", "triangle down"),
        e(G::Triangle, "Extract", "triangle up"),
        e(G::Collate, "Collate", "hourglass"),
        e(G::Sort, "Sort", "order"),
        e(G::Or, "Or", "junction"),
        e(G::SummingJunction, "Summing junction", "and"),
        e(G::InternalStorage, "Internal storage", "memory"),
        e(G::LoopLimit, "Loop limit", "repeat"),
        e(G::Card, "Punched card", "card"),
        e(G::SequentialData, "Sequential data", "tape storage"),
    ];
    let arrows = vec![
        e(G::Line, "Line", "connector"),
        e(G::Arrow, "Arrow", "connector direction"),
        e(G::DoubleArrow, "Double arrow", "both ways"),
        e(G::Elbow, "Elbow arrow", "orthogonal"),
        e(G::CurvedArrow, "Curved arrow", "bend"),
        e(G::BlockArrow, "Block arrow", "fat arrow right"),
        e(G::BlockDoubleArrow, "Block double arrow", "fat both ways"),
        e(G::Chevron, "Chevron", "next step"),
    ];
    let uml = vec![
        e(G::Actor, "Actor", "person user stick figure"),
        e(G::Class, "Class", "object table"),
        e(G::Package, "Package", "module folder"),
        e(G::Component, "Component", "module part"),
        e(G::Note, "Comment", "note"),
    ];
    let icons = |lib: IconLibrary| -> Vec<Entry> { Icon::ALL.iter().filter(|i| i.library() == lib).map(|&i| e(G::Icon(i), i.name(), i.keywords())).collect() };
    vec![
        Library { id: "general", name: "General", entries: general },
        Library { id: "flow", name: "Flowchart", entries: flow },
        Library { id: "arrows", name: "Arrows & lines", entries: arrows },
        Library { id: "uml", name: "UML", entries: uml },
        Library { id: "network", name: "Network", entries: icons(IconLibrary::Network) },
        Library { id: "devices", name: "Devices", entries: icons(IconLibrary::Devices) },
        Library { id: "people", name: "People & office", entries: icons(IconLibrary::People) },
        Library { id: "software", name: "Software", entries: icons(IconLibrary::Software) },
    ]
}

/// The entries matching `q` (name, keywords or family; every word must
/// match the start of one), best first: names that start with it, then
/// the rest. Each shape once.
pub fn search(q: &str) -> Vec<(Entry, &'static str)> {
    let words: Vec<String> = q.split_whitespace().map(str::to_lowercase).collect();
    if words.is_empty() {
        return Vec::new();
    }
    let mut hits: Vec<(u8, Entry, &'static str)> = Vec::new();
    for lib in libraries() {
        for en in &lib.entries {
            // "Wi-Fi" is found by "wifi" too.
            let compact: String = en.name.chars().filter(|c| c.is_alphanumeric()).collect::<String>().to_lowercase();
            let hay = format!("{} {} {} {compact}", en.name, en.keywords, lib.name).to_lowercase();
            let terms: Vec<&str> = hay.split(|c: char| !c.is_alphanumeric()).filter(|t| !t.is_empty()).collect();
            if !words.iter().all(|w| terms.iter().any(|t| t.starts_with(w.as_str()))) {
                continue;
            }
            if hits.iter().any(|h| h.1.geom == en.geom && h.1.name == en.name) {
                continue;
            }
            let rank = u8::from(!compact.starts_with(words[0].as_str()) && !en.name.to_lowercase().starts_with(words[0].as_str()));
            hits.push((rank, *en, lib.name));
        }
    }
    hits.sort_by_key(|h| h.0);
    hits.into_iter().map(|h| (h.1, h.2)).collect()
}

impl Geom {
    pub fn name(self) -> &'static str {
        if let Geom::Icon(i) = self {
            return i.name();
        }
        libraries().into_iter().flat_map(|l| l.entries).find(|e| e.geom == self).map_or("Shape", |e| e.name)
    }

    /// A line or an arrow: drawn, never filled, and caught along its line.
    pub fn is_line(self) -> bool {
        matches!(self, Geom::Line | Geom::Arrow | Geom::DoubleArrow | Geom::Elbow | Geom::CurvedArrow)
    }

    /// Lines drawn in a box, with no body to fill (icons, the actor): caught
    /// anywhere in the box, their words written under it.
    pub fn is_figure(self) -> bool {
        matches!(self, Geom::Icon(_) | Geom::Actor)
    }

    /// Has a body that takes a fill.
    pub fn fills(self) -> bool {
        !self.is_line() && !self.is_figure()
    }

    /// The box a new one gets, in world units.
    pub fn default_size(self) -> (f32, f32) {
        match self {
            Geom::Line | Geom::Arrow | Geom::DoubleArrow => (160., 24.),
            Geom::Elbow | Geom::CurvedArrow => (160., 80.),
            Geom::BlockArrow | Geom::BlockDoubleArrow | Geom::Chevron => (140., 70.),
            Geom::Square | Geom::Circle | Geom::Star | Geom::Cross | Geom::Heart | Geom::Octagon | Geom::Pentagon => (100., 100.),
            Geom::Or | Geom::SummingJunction | Geom::SequentialData => (80., 80.),
            Geom::Ellipse | Geom::Diamond | Geom::Triangle | Geom::Hexagon | Geom::Merge | Geom::Collate | Geom::Sort => (140., 100.),
            Geom::Cylinder => (110., 120.),
            Geom::Terminator => (150., 60.),
            Geom::Note | Geom::Cube | Geom::Class | Geom::Callout => (160., 120.),
            Geom::Actor => (40., 80.),
            Geom::Icon(_) => (64., 64.),
            _ => (160., 96.),
        }
    }

    /// The outline fitted to `r`, upright: the body first (closed, and
    /// filled when the shape has a fill), then any lines drawn over it.
    pub fn outlines(self, r: Rect) -> Vec<Outline> {
        let (x, y, w, h) = (r.x, r.y, r.w, r.h);
        let (cx, cy) = (x + w / 2., y + h / 2.);
        let closed = |points: Vec<Vec2>| Outline { points, closed: true };
        let open = |points: Vec<Vec2>| Outline { points, closed: false };
        let at = |u: f32, v: f32| v2(x + u * w, y + v * h);
        let poly = |pts: &[(f32, f32)]| closed(pts.iter().map(|&(u, v)| at(u, v)).collect());
        let seg = |a: (f32, f32), b: (f32, f32)| open(vec![at(a.0, a.1), at(b.0, b.1)]);
        let oval = |c: Vec2, rx: f32, ry: f32| {
            let mut p = Vec::new();
            arc(&mut p, c, rx, ry, 0., TAU);
            p.pop();
            closed(p)
        };
        // An arrowhead at `tip`, pointing along (dx, dy), `spread` either
        // side of the line.
        let len = (w.max(h) * 0.3).min(16.);
        let spread = len * 0.55;
        let head = |tip: Vec2, dx: f32, dy: f32| {
            let (bx, by) = (tip.x - dx * len, tip.y - dy * len);
            let (nx, ny) = (-dy * spread, dx * spread);
            open(vec![v2(bx + nx, by + ny), tip, v2(bx - nx, by - ny)])
        };
        let fold = 18f32.min(w * 0.2).min(h * 0.2);
        let wave = |from: f32, to: f32, base: f32, amp: f32| -> Vec<Vec2> {
            (0..=24)
                .map(|i| {
                    let u = from + (to - from) * i as f32 / 24.;
                    v2(x + u * w, base + (u * TAU).sin() * amp)
                })
                .collect()
        };
        match self {
            Geom::Rect | Geom::Square => vec![poly(&[(0., 0.), (1., 0.), (1., 1.), (0., 1.)])],
            Geom::Rounded => {
                let k = 14f32.min(w * 0.2).min(h * 0.2);
                let mut p = Vec::new();
                arc(&mut p, v2(x + w - k, y + k), k, k, -FRAC_PI_2, 0.);
                arc(&mut p, v2(x + w - k, y + h - k), k, k, 0., FRAC_PI_2);
                arc(&mut p, v2(x + k, y + h - k), k, k, FRAC_PI_2, PI);
                arc(&mut p, v2(x + k, y + k), k, k, PI, PI + FRAC_PI_2);
                vec![closed(p)]
            }
            Geom::Ellipse | Geom::Circle => vec![oval(v2(cx, cy), w / 2., h / 2.)],
            Geom::Diamond => vec![poly(&[(0.5, 0.), (1., 0.5), (0.5, 1.), (0., 0.5)])],
            Geom::Triangle => vec![poly(&[(0.5, 0.), (1., 1.), (0., 1.)])],
            Geom::Merge => vec![poly(&[(0., 0.), (1., 0.), (0.5, 1.)])],
            Geom::Hexagon => vec![poly(&[(0.2, 0.), (0.8, 0.), (1., 0.5), (0.8, 1.), (0.2, 1.), (0., 0.5)])],
            Geom::Parallelogram => vec![poly(&[(0.2, 0.), (1., 0.), (0.8, 1.), (0., 1.)])],
            Geom::Trapezoid => vec![poly(&[(0.2, 0.), (0.8, 0.), (1., 1.), (0., 1.)])],
            Geom::Pentagon => vec![poly(&[(0.5, 0.), (1., 0.38), (0.81, 1.), (0.19, 1.), (0., 0.38)])],
            Geom::Octagon => {
                let k = 0.29;
                vec![poly(&[(k, 0.), (1. - k, 0.), (1., k), (1., 1. - k), (1. - k, 1.), (k, 1.), (0., 1. - k), (0., k)])]
            }
            Geom::Star => {
                let p = (0..10)
                    .map(|i| {
                        let a = -FRAC_PI_2 + PI * i as f32 / 5.;
                        let k = if i % 2 == 0 { 0.5 } else { 0.2 };
                        v2(cx + a.cos() * w * k, y + h * 0.53 + a.sin() * h * k)
                    })
                    .collect();
                vec![closed(p)]
            }
            Geom::Cross => {
                let t = 0.33;
                let u = 1. - t;
                vec![poly(&[(t, 0.), (u, 0.), (u, t), (1., t), (1., u), (u, u), (u, 1.), (t, 1.), (t, u), (0., u), (0., t), (t, t)])]
            }
            Geom::Heart => {
                let p = (0..72)
                    .map(|i| {
                        let a = TAU * i as f32 / 72.;
                        let (hx, hy) = (16. * a.sin().powi(3), 13. * a.cos() - 5. * (2. * a).cos() - 2. * (3. * a).cos() - (4. * a).cos());
                        v2(cx + hx / 34. * w, y + h * 0.42 - hy / 30. * h)
                    })
                    .collect();
                vec![closed(p)]
            }
            Geom::ManualInput => vec![poly(&[(0., 0.25), (1., 0.), (1., 1.), (0., 1.)])],
            Geom::OffPage => vec![poly(&[(0., 0.), (1., 0.), (1., 0.65), (0.5, 1.), (0., 0.65)])],
            Geom::ManualOp => vec![poly(&[(0., 0.), (1., 0.), (0.8, 1.), (0.2, 1.)])],
            Geom::Predefined => vec![poly(&[(0., 0.), (1., 0.), (1., 1.), (0., 1.)]), seg((0.1, 0.), (0.1, 1.)), seg((0.9, 0.), (0.9, 1.))],
            Geom::InternalStorage => vec![poly(&[(0., 0.), (1., 0.), (1., 1.), (0., 1.)]), seg((0.15, 0.), (0.15, 1.)), seg((0., 0.2), (1., 0.2))],
            Geom::LoopLimit => vec![poly(&[(0.15, 0.), (0.85, 0.), (1., 0.3), (1., 1.), (0., 1.), (0., 0.3)])],
            Geom::Collate => vec![poly(&[(0., 0.), (1., 0.), (0., 1.), (1., 1.)])],
            Geom::Sort => vec![poly(&[(0.5, 0.), (1., 0.5), (0.5, 1.), (0., 0.5)]), seg((0., 0.5), (1., 0.5))],
            Geom::Or | Geom::SummingJunction => {
                let (rx, ry) = (w / 2., h / 2.);
                let lines = if self == Geom::Or {
                    vec![seg((0.5, 0.), (0.5, 1.)), seg((0., 0.5), (1., 0.5))]
                } else {
                    let k = std::f32::consts::FRAC_1_SQRT_2 / 2.;
                    vec![seg((0.5 - k, 0.5 - k), (0.5 + k, 0.5 + k)), seg((0.5 + k, 0.5 - k), (0.5 - k, 0.5 + k))]
                };
                let mut out = vec![oval(v2(cx, cy), rx, ry)];
                out.extend(lines);
                out
            }
            Geom::SequentialData => vec![oval(v2(cx, cy), w / 2., h / 2.), seg((0.5, 1.), (1., 1.))],
            Geom::Display => {
                let k = (h / 2.).min(w * 0.3);
                let mut p = vec![v2(x, cy), v2(x + w * 0.18, y)];
                arc(&mut p, v2(x + w - k, cy), k, h / 2., -FRAC_PI_2, FRAC_PI_2);
                p.push(v2(x + w * 0.18, y + h));
                vec![closed(p)]
            }
            Geom::Cylinder => {
                let ry = (h * 0.15).min(16.);
                let mut body = Vec::new();
                arc(&mut body, v2(cx, y + h - ry), w / 2., ry, PI, 0.);
                arc(&mut body, v2(cx, y + ry), w / 2., ry, 0., -PI);
                let mut lip = Vec::new();
                arc(&mut lip, v2(cx, y + ry), w / 2., ry, PI, 0.);
                vec![closed(body), open(lip)]
            }
            Geom::Cloud => {
                // Scallops round an ellipse, meeting in small dips.
                let n = 120;
                let p = (0..n)
                    .map(|i| {
                        let a = TAU * i as f32 / n as f32;
                        let k = 0.84 + 0.16 * (3.5 * a).sin().abs();
                        v2(cx + a.cos() * w / 2. * k, cy + a.sin() * h / 2. * k)
                    })
                    .collect();
                vec![closed(p)]
            }
            Geom::Document => {
                let mut p = vec![v2(x, y), v2(x + w, y)];
                p.extend(wave(1., 0., y + h * 0.85, h * 0.08));
                vec![closed(p)]
            }
            Geom::MultiDocument => {
                // The front page, with two more peeking out behind it.
                let (dx, dy) = (8f32.min(w * 0.06), 8f32.min(h * 0.08));
                let fw = w - 2. * dx;
                let front_y = y + 2. * dy;
                let mut p = vec![v2(x, front_y), v2(x + fw, front_y)];
                p.extend((0..=24).map(|i| {
                    let u = 1. - i as f32 / 24.;
                    v2(x + u * fw, y + h * 0.85 + (u * TAU).sin() * h * 0.08)
                }));
                vec![
                    closed(p),
                    open(vec![v2(x + dx, front_y), v2(x + dx, y + dy), v2(x + fw + dx, y + dy), v2(x + fw + dx, y + h * 0.8)]),
                    open(vec![v2(x + 2. * dx, y + dy), v2(x + 2. * dx, y), v2(x + w, y), v2(x + w, y + h * 0.74)]),
                ]
            }
            Geom::Tape => {
                let amp = h * 0.08;
                let mut p = wave(0., 1., y + h * 0.1, amp);
                p.extend(wave(1., 0., y + h * 0.9, amp));
                vec![closed(p)]
            }
            Geom::Note => vec![
                closed(vec![v2(x, y), v2(x + w - fold, y), v2(x + w, y + fold), v2(x + w, y + h), v2(x, y + h)]),
                open(vec![v2(x + w - fold, y), v2(x + w - fold, y + fold), v2(x + w, y + fold)]),
            ],
            Geom::Card => vec![closed(vec![v2(x + fold, y), v2(x + w, y), v2(x + w, y + h), v2(x, y + h), v2(x, y + fold)])],
            Geom::Callout => vec![poly(&[(0., 0.), (1., 0.), (1., 0.75), (0.45, 0.75), (0.25, 1.), (0.3, 0.75), (0., 0.75)])],
            Geom::Cube => {
                let d = 16f32.min(w * 0.2).min(h * 0.2);
                vec![
                    closed(vec![v2(x, y + d), v2(x + d, y), v2(x + w, y), v2(x + w, y + h - d), v2(x + w - d, y + h), v2(x, y + h)]),
                    open(vec![v2(x, y + d), v2(x + w - d, y + d), v2(x + w, y)]),
                    open(vec![v2(x + w - d, y + d), v2(x + w - d, y + h)]),
                ]
            }
            Geom::Step => vec![poly(&[(0., 0.), (0.8, 0.), (1., 0.5), (0.8, 1.), (0., 1.), (0.2, 0.5)])],
            Geom::Chevron => vec![poly(&[(0., 0.), (0.65, 0.), (1., 0.5), (0.65, 1.), (0., 1.), (0.35, 0.5)])],
            Geom::BlockArrow => vec![poly(&[(0., 0.3), (0.6, 0.3), (0.6, 0.), (1., 0.5), (0.6, 1.), (0.6, 0.7), (0., 0.7)])],
            Geom::BlockDoubleArrow => {
                vec![poly(&[(0., 0.5), (0.3, 0.), (0.3, 0.3), (0.7, 0.3), (0.7, 0.), (1., 0.5), (0.7, 1.), (0.7, 0.7), (0.3, 0.7), (0.3, 1.)])]
            }
            Geom::Terminator => {
                let k = (h / 2.).min(w / 2.);
                let mut p = Vec::new();
                arc(&mut p, v2(x + w - k, cy), k, h / 2., -FRAC_PI_2, FRAC_PI_2);
                arc(&mut p, v2(x + k, cy), k, h / 2., FRAC_PI_2, PI + FRAC_PI_2);
                vec![closed(p)]
            }
            Geom::Delay => {
                let k = (h / 2.).min(w / 2.);
                let mut p = vec![v2(x, y)];
                arc(&mut p, v2(x + w - k, cy), k, h / 2., -FRAC_PI_2, FRAC_PI_2);
                p.push(v2(x, y + h));
                vec![closed(p)]
            }
            Geom::Class => vec![poly(&[(0., 0.), (1., 0.), (1., 1.), (0., 1.)]), seg((0., 0.25), (1., 0.25)), seg((0., 0.62), (1., 0.62))],
            Geom::Package => {
                let tab = 18f32.min(h * 0.25);
                vec![
                    closed(vec![v2(x, y), v2(x + w * 0.4, y), v2(x + w * 0.4, y + tab), v2(x + w, y + tab), v2(x + w, y + h), v2(x, y + h)]),
                    open(vec![v2(x, y + tab), v2(x + w * 0.4, y + tab)]),
                ]
            }
            Geom::Component => {
                let (bw, bh) = (16f32.min(w * 0.2), 10f32.min(h * 0.15));
                let lug = |top: f32| closed(vec![v2(x - bw / 2., top), v2(x + bw / 2., top), v2(x + bw / 2., top + bh), v2(x - bw / 2., top + bh)]);
                vec![poly(&[(0., 0.), (1., 0.), (1., 1.), (0., 1.)]), lug(y + h * 0.25 - bh / 2.), lug(y + h * 0.65 - bh / 2.)]
            }
            Geom::Actor => {
                let r = (w * 0.25).min(h * 0.13);
                let (neck, hip) = (y + 2. * r, y + h * 0.62);
                vec![
                    oval(v2(cx, y + r), r, r),
                    open(vec![v2(cx, neck), v2(cx, hip)]),
                    open(vec![v2(x, y + h * 0.38), v2(x + w, y + h * 0.38)]),
                    open(vec![v2(x, y + h), v2(cx, hip), v2(x + w, y + h)]),
                ]
            }
            Geom::Icon(i) => {
                let k = w.min(h) / GRID;
                let (ox, oy) = (cx - GRID * k / 2., cy - GRID * k / 2.);
                icons::lines(i).iter().map(|(pts, c)| Outline { points: pts.iter().map(|p| v2(ox + p.x * k, oy + p.y * k)).collect(), closed: *c }).collect()
            }
            Geom::Line => vec![open(vec![v2(x, cy), v2(x + w, cy)])],
            Geom::Arrow => vec![open(vec![v2(x, cy), v2(x + w, cy)]), head(v2(x + w, cy), 1., 0.)],
            Geom::DoubleArrow => vec![open(vec![v2(x, cy), v2(x + w, cy)]), head(v2(x + w, cy), 1., 0.), head(v2(x, cy), -1., 0.)],
            Geom::Elbow => {
                // The top run sits low enough for its arrowhead to fit.
                let top = y + spread.min(h / 2.);
                vec![open(vec![v2(x, y + h), v2(cx, y + h), v2(cx, top), v2(x + w, top)]), head(v2(x + w, top), 1., 0.)]
            }
            Geom::CurvedArrow => {
                let top = y + spread.min(h / 2.);
                let (a, c, b) = (v2(x, y + h), v2(x + w * 0.15, top), v2(x + w, top));
                let p = (0..=24)
                    .map(|i| {
                        let t = i as f32 / 24.;
                        let u = 1. - t;
                        v2(u * u * a.x + 2. * u * t * c.x + t * t * b.x, u * u * a.y + 2. * u * t * c.y + t * t * b.y)
                    })
                    .collect();
                vec![open(p), head(b, 1., 0.)]
            }
        }
    }
}

/// Points along an ellipse round `c` from angle `a0` to `a1` (radians,
/// clockwise on screen), both ends included.
fn arc(out: &mut Vec<Vec2>, c: Vec2, rx: f32, ry: f32, a0: f32, a1: f32) {
    let n = ((a1 - a0).abs() / TAU * 64.).ceil().max(2.) as usize;
    out.extend((0..=n).map(|i| {
        let a = a0 + (a1 - a0) * i as f32 / n as f32;
        v2(c.x + rx * a.cos(), c.y + ry * a.sin())
    }));
}

/// One run of a shape's outline, in world units.
#[derive(Clone, Debug, PartialEq)]
pub struct Outline {
    pub points: Vec<Vec2>,
    pub closed: bool,
}

/// How heavy a shape's outline is.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum LineWidth {
    Thin,
    #[default]
    Regular,
    Bold,
}

impl LineWidth {
    pub const ALL: [LineWidth; 3] = [LineWidth::Thin, LineWidth::Regular, LineWidth::Bold];
    pub fn units(self) -> f32 {
        match self {
            LineWidth::Thin => 1.25,
            LineWidth::Regular => 2.,
            LineWidth::Bold => 3.5,
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            LineWidth::Thin => "Thin",
            LineWidth::Regular => "Regular",
            LineWidth::Bold => "Bold",
        }
    }
}

/// A shape on a sheet: an outline in a box, turned by `rot`, with words in
/// the middle.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Shape {
    pub id: Id,
    pub geom: Geom,
    /// The box before turning.
    pub rect: Rect,
    /// Degrees clockwise about the box's centre, 0 to 360.
    #[serde(default)]
    pub rot: f32,
    /// The paper inside; none for a see-through shape (and lines).
    #[serde(default)]
    pub fill: Option<Paper>,
    pub ink: Ink,
    #[serde(default)]
    pub line: LineWidth,
    #[serde(default)]
    pub dashed: bool,
    #[serde(default)]
    pub text: String,
    #[serde(default)]
    pub locked: bool,
}

impl Shape {
    /// A new `geom` centred on `at`, in plain paper and graphite ink.
    pub fn new(id: Id, geom: Geom, at: Vec2) -> Shape {
        let (w, h) = geom.default_size();
        Shape {
            id,
            geom,
            rect: Rect { x: at.x - w / 2., y: at.y - h / 2., w, h },
            rot: 0.,
            fill: geom.fills().then_some(Paper::Plain),
            ink: Ink::Graphite,
            // Icons are drawn small on a fine grid: a heavier line reads.
            line: if matches!(geom, Geom::Icon(_)) { LineWidth::Bold } else { LineWidth::Regular },
            dashed: false,
            text: String::new(),
            locked: false,
        }
    }

    pub fn center(&self) -> Vec2 {
        self.rect.center()
    }

    /// `p` turned by the shape's angle about its centre: from the upright
    /// box to the sheet.
    pub fn turn(&self, p: Vec2) -> Vec2 {
        rotate(p, self.center(), self.rot)
    }

    /// `p` on the sheet, in the upright box's terms.
    pub fn unturn(&self, p: Vec2) -> Vec2 {
        rotate(p, self.center(), -self.rot)
    }

    /// The outline as it lies on the sheet, turned.
    pub fn outlines(&self) -> Vec<Outline> {
        let mut out = self.geom.outlines(self.rect);
        if self.rot != 0. {
            for o in &mut out {
                for p in &mut o.points {
                    *p = self.turn(*p);
                }
            }
        }
        out
    }

    /// Where its words go, upright: inside it, or under an icon or actor,
    /// as many lines deep as they wrap to.
    pub fn text_rect(&self) -> Rect {
        let r = self.rect;
        if !self.geom.is_figure() {
            return r;
        }
        let w = r.w.max(160.);
        let lines = crate::glyphs::wrap(self.text.trim(), SHAPE_TEXT, w).len().max(1);
        Rect { x: r.x + (r.w - w) / 2., y: r.y + r.h + 4., w, h: lines as f32 * SHAPE_TEXT * crate::glyphs::LEADING }
    }

    /// The box round the turned shape and its line.
    pub fn bounds(&self) -> Rect {
        let pad = self.line.units() / 2.;
        let (mut x0, mut y0, mut x1, mut y1) = (f32::MAX, f32::MAX, f32::MIN, f32::MIN);
        for o in self.outlines() {
            for p in o.points {
                x0 = x0.min(p.x);
                y0 = y0.min(p.y);
                x1 = x1.max(p.x);
                y1 = y1.max(p.y);
            }
        }
        Rect { x: x0 - pad, y: y0 - pad, w: x1 - x0 + 2. * pad, h: y1 - y0 + 2. * pad }
    }

    /// Whether `p` (on the sheet) is on the shape: inside its body, or
    /// within `radius` of a line.
    pub fn hit(&self, p: Vec2, radius: f32) -> bool {
        let local = self.unturn(p);
        let outlines = self.geom.outlines(self.rect);
        let reach = radius + self.line.units() / 2.;
        let near = |o: &Outline| {
            let pts = &o.points;
            let mut segs: Vec<(Vec2, Vec2)> = pts.windows(2).map(|w| (w[0], w[1])).collect();
            if o.closed && pts.len() > 2 {
                segs.push((pts[pts.len() - 1], pts[0]));
            }
            segs.iter().any(|&(a, b)| segment_distance(local, a, b) <= reach)
        };
        if self.geom.is_line() {
            return outlines.iter().any(near);
        }
        if self.geom.is_figure() {
            let r = self.rect;
            return Rect { x: r.x - radius, y: r.y - radius, w: r.w + 2. * radius, h: r.h + 2. * radius }.contains(local);
        }
        outlines.first().is_some_and(|o| inside(local, &o.points)) || outlines.iter().any(near)
    }
}

/// `p` turned `deg` degrees clockwise (on screen, y down) about `c`.
pub fn rotate(p: Vec2, c: Vec2, deg: f32) -> Vec2 {
    if deg == 0. {
        return p;
    }
    let (s, k) = deg.to_radians().sin_cos();
    let (dx, dy) = (p.x - c.x, p.y - c.y);
    v2(c.x + dx * k - dy * s, c.y + dx * s + dy * k)
}

/// The angle in degrees, 0 to 360, of a handle above `c` dragged to `p`:
/// straight up is 0. Within `snap` degrees of a multiple of 15, it's that.
pub fn handle_angle(c: Vec2, p: Vec2, snap: f32) -> f32 {
    let a = (p.y - c.y).atan2(p.x - c.x).to_degrees() + 90.;
    let a = a.rem_euclid(360.);
    let near = (a / 15.).round() * 15.;
    if (a - near).abs() <= snap {
        near.rem_euclid(360.)
    } else {
        a
    }
}

/// Whether `p` is inside the polygon `pts` (even-odd).
fn inside(p: Vec2, pts: &[Vec2]) -> bool {
    let mut odd = false;
    let n = pts.len();
    for i in 0..n {
        let (a, b) = (pts[i], pts[(i + n - 1) % n]);
        if (a.y > p.y) != (b.y > p.y) && p.x < (b.x - a.x) * (p.y - a.y) / (b.y - a.y) + a.x {
            odd = !odd;
        }
    }
    odd
}

#[cfg(test)]
mod tests {
    use super::*;

    fn near(a: Vec2, b: Vec2) -> bool {
        a.sub(b).len() < 1e-3
    }

    #[test]
    fn every_shape_has_a_body_inside_its_box() {
        let r = Rect { x: 10., y: 20., w: 160., h: 96. };
        for lib in libraries() {
            for en in lib.entries {
                let g = en.geom;
                let o = g.outlines(r);
                assert!(!o.is_empty(), "{g:?}");
                if g.is_line() || g.fills() {
                    assert_eq!(o[0].closed, g.fills(), "{g:?}");
                }
                // A component's lugs sit across its left edge.
                let slack = if g == Geom::Component { 9. } else { 0.01 };
                for p in o.iter().flat_map(|o| &o.points) {
                    assert!(p.x >= r.x - slack && p.x <= r.x + r.w + 0.01 && p.y >= r.y - 0.01 && p.y <= r.y + r.h + 0.01, "{g:?} {p:?}");
                }
            }
        }
    }

    #[test]
    fn search_finds_by_name_keyword_and_family() {
        let names = |q: &str| search(q).iter().map(|(e, _)| e.name).collect::<Vec<_>>();
        assert_eq!(names("wifi")[0], "Wi-Fi");
        assert!(names("wifi").contains(&"Router") && names("wifi").contains(&"Access point"));
        assert_eq!(names("anten")[0], "Antenna");
        assert!(names("decision").contains(&"Decision"));
        assert!(names("network serv").contains(&"Server"));
        assert!(names("zzz").is_empty() && names("  ").is_empty());
        // Every shape is somewhere, and icons and the actor have no fill.
        assert!(libraries().iter().map(|l| l.entries.len()).sum::<usize>() > 100);
        assert!(Shape::new(1, Geom::Icon(Icon::Router), v2(0., 0.)).fill.is_none());
    }

    #[test]
    fn icons_are_caught_anywhere_in_their_box_and_labelled_under_it() {
        let mut s = Shape::new(1, Geom::Icon(Icon::Wifi), v2(0., 0.));
        assert!(s.hit(v2(-30., 30.), 0.));
        s.text = "Office Wi-Fi".into();
        let t = s.text_rect();
        assert!(t.y > s.rect.y + s.rect.h && t.w >= s.rect.w);
    }

    #[test]
    fn a_quarter_turn_stands_a_box_on_its_end() {
        let mut s = Shape::new(1, Geom::Rect, v2(0., 0.));
        s.rect = Rect { x: -50., y: -10., w: 100., h: 20. };
        s.rot = 90.;
        let b = s.bounds();
        // 20 by 100 on end, and the line's width round it.
        assert!((b.w - 22.).abs() < 0.01 && (b.h - 102.).abs() < 0.01, "{b:?}");
        // The top-left corner swings round to the top-right.
        assert!(near(s.turn(v2(-50., -10.)), v2(10., -50.)));
        assert!(near(s.unturn(s.turn(v2(3., 4.))), v2(3., 4.)));
    }

    #[test]
    fn hits_follow_the_turned_outline() {
        let mut s = Shape::new(1, Geom::Rect, v2(0., 0.));
        s.rect = Rect { x: -50., y: -10., w: 100., h: 20. };
        assert!(s.hit(v2(40., 0.), 0.));
        assert!(!s.hit(v2(0., 40.), 0.));
        s.rot = 90.;
        assert!(!s.hit(v2(40., 0.), 0.));
        assert!(s.hit(v2(0., 40.), 0.));
        // A diamond's corners are outside it.
        let d = Shape { geom: Geom::Diamond, ..s.clone() };
        let d = Shape { rot: 0., rect: Rect { x: 0., y: 0., w: 100., h: 100. }, ..d };
        assert!(d.hit(v2(50., 50.), 0.));
        assert!(!d.hit(v2(5., 5.), 0.));
    }

    #[test]
    fn lines_are_caught_along_the_line_only() {
        let s = Shape::new(1, Geom::Arrow, v2(100., 100.));
        assert!(s.hit(v2(100., 100.), 2.));
        assert!(!s.hit(v2(100., 108.), 2.));
        assert!(s.fill.is_none());
    }

    #[test]
    fn the_handle_snaps_to_fifteen_degrees() {
        let c = v2(0., 0.);
        assert_eq!(handle_angle(c, v2(0., -10.), 4.), 0.);
        assert_eq!(handle_angle(c, v2(10., 0.), 4.), 90.);
        assert_eq!(handle_angle(c, v2(0., 10.), 4.), 180.);
        assert_eq!(handle_angle(c, v2(-10., 0.), 4.), 270.);
        // 47 degrees is near 45; 52 isn't near anything.
        let at = |deg: f32| {
            let a = (deg - 90.).to_radians();
            v2(a.cos() * 10., a.sin() * 10.)
        };
        assert_eq!(handle_angle(c, at(47.), 4.), 45.);
        assert!((handle_angle(c, at(52.), 4.) - 52.).abs() < 0.01);
    }

    #[test]
    fn old_shapes_without_the_newer_fields_load() {
        let json = r#"{"id":4,"geom":"Diamond","rect":{"x":0,"y":0,"w":10,"h":10},"ink":"Cobalt"}"#;
        let s: Shape = serde_json::from_str(json).unwrap();
        assert_eq!((s.rot, s.fill, s.line, s.dashed, s.locked), (0., None, LineWidth::Regular, false, false));
    }
}
