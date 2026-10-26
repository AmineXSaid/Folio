//! Shapes drawn on a sheet: boxes, flowchart symbols, lines and arrows.
//! Each is a kind of outline fitted to a box, turned about the box's
//! centre. Plain geometry only; nothing here knows about GPUI. The board
//! paints these outlines and the PNG/PDF export writes them, so both draw
//! the same thing.

use std::f32::consts::{FRAC_PI_2, PI, TAU};

use serde::{Deserialize, Serialize};

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
    Rect,
    Rounded,
    Ellipse,
    Diamond,
    Triangle,
    Hexagon,
    Parallelogram,
    Cylinder,
    Cloud,
    Document,
    Terminator,
    Delay,
    ManualInput,
    OffPage,
    Predefined,
    ManualOp,
    Line,
    Arrow,
    DoubleArrow,
    Elbow,
}

impl Geom {
    /// The boxes and symbols, in the order the picker shows them.
    pub const BASIC: [Geom; 10] = [
        Geom::Rect,
        Geom::Rounded,
        Geom::Ellipse,
        Geom::Diamond,
        Geom::Triangle,
        Geom::Hexagon,
        Geom::Parallelogram,
        Geom::Cylinder,
        Geom::Cloud,
        Geom::Document,
    ];
    pub const FLOW: [Geom; 6] = [Geom::Terminator, Geom::Delay, Geom::ManualInput, Geom::OffPage, Geom::Predefined, Geom::ManualOp];
    pub const LINES: [Geom; 4] = [Geom::Line, Geom::Arrow, Geom::DoubleArrow, Geom::Elbow];

    pub fn name(self) -> &'static str {
        match self {
            Geom::Rect => "Rectangle",
            Geom::Rounded => "Rounded rectangle",
            Geom::Ellipse => "Ellipse",
            Geom::Diamond => "Diamond",
            Geom::Triangle => "Triangle",
            Geom::Hexagon => "Hexagon",
            Geom::Parallelogram => "Parallelogram",
            Geom::Cylinder => "Cylinder",
            Geom::Cloud => "Cloud",
            Geom::Document => "Document",
            Geom::Terminator => "Start / end",
            Geom::Delay => "Delay",
            Geom::ManualInput => "Manual input",
            Geom::OffPage => "Off-page link",
            Geom::Predefined => "Predefined process",
            Geom::ManualOp => "Manual operation",
            Geom::Line => "Line",
            Geom::Arrow => "Arrow",
            Geom::DoubleArrow => "Double arrow",
            Geom::Elbow => "Elbow arrow",
        }
    }

    /// A line or an arrow: drawn, never filled, and caught along its line.
    pub fn is_line(self) -> bool {
        matches!(self, Geom::Line | Geom::Arrow | Geom::DoubleArrow | Geom::Elbow)
    }

    /// The box a new one gets, in world units.
    pub fn default_size(self) -> (f32, f32) {
        match self {
            Geom::Line | Geom::Arrow | Geom::DoubleArrow => (160., 24.),
            Geom::Elbow => (160., 80.),
            Geom::Ellipse | Geom::Diamond | Geom::Triangle | Geom::Hexagon => (140., 100.),
            Geom::Cylinder => (110., 120.),
            Geom::Terminator => (150., 60.),
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
        let poly = |pts: &[(f32, f32)]| closed(pts.iter().map(|&(u, v)| v2(x + u * w, y + v * h)).collect());
        // An arrowhead at `tip`, pointing along (dx, dy), `spread` either
        // side of the line.
        let len = (w.max(h) * 0.3).min(16.);
        let spread = len * 0.55;
        let head = |tip: Vec2, dx: f32, dy: f32| {
            let (bx, by) = (tip.x - dx * len, tip.y - dy * len);
            let (nx, ny) = (-dy * spread, dx * spread);
            open(vec![v2(bx + nx, by + ny), tip, v2(bx - nx, by - ny)])
        };
        match self {
            Geom::Rect => vec![poly(&[(0., 0.), (1., 0.), (1., 1.), (0., 1.)])],
            Geom::Rounded => {
                let k = 14f32.min(w * 0.2).min(h * 0.2);
                let mut p = Vec::new();
                arc(&mut p, v2(x + w - k, y + k), k, k, -FRAC_PI_2, 0.);
                arc(&mut p, v2(x + w - k, y + h - k), k, k, 0., FRAC_PI_2);
                arc(&mut p, v2(x + k, y + h - k), k, k, FRAC_PI_2, PI);
                arc(&mut p, v2(x + k, y + k), k, k, PI, PI + FRAC_PI_2);
                vec![closed(p)]
            }
            Geom::Ellipse => {
                let mut p = Vec::new();
                arc(&mut p, v2(cx, cy), w / 2., h / 2., 0., TAU);
                p.pop();
                vec![closed(p)]
            }
            Geom::Diamond => vec![poly(&[(0.5, 0.), (1., 0.5), (0.5, 1.), (0., 0.5)])],
            Geom::Triangle => vec![poly(&[(0.5, 0.), (1., 1.), (0., 1.)])],
            Geom::Hexagon => vec![poly(&[(0.2, 0.), (0.8, 0.), (1., 0.5), (0.8, 1.), (0.2, 1.), (0., 0.5)])],
            Geom::Parallelogram => vec![poly(&[(0.2, 0.), (1., 0.), (0.8, 1.), (0., 1.)])],
            Geom::ManualInput => vec![poly(&[(0., 0.25), (1., 0.), (1., 1.), (0., 1.)])],
            Geom::OffPage => vec![poly(&[(0., 0.), (1., 0.), (1., 0.65), (0.5, 1.), (0., 0.65)])],
            Geom::ManualOp => vec![poly(&[(0., 0.), (1., 0.), (0.8, 1.), (0.2, 1.)])],
            Geom::Predefined => vec![
                poly(&[(0., 0.), (1., 0.), (1., 1.), (0., 1.)]),
                open(vec![v2(x + w * 0.1, y), v2(x + w * 0.1, y + h)]),
                open(vec![v2(x + w * 0.9, y), v2(x + w * 0.9, y + h)]),
            ],
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
                let base = y + h * 0.85;
                let mut p = vec![v2(x, y), v2(x + w, y)];
                for i in 0..=24 {
                    let u = 1. - i as f32 / 24.;
                    p.push(v2(x + u * w, base + (u * TAU).sin() * h * 0.08));
                }
                vec![closed(p)]
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
            Geom::Line => vec![open(vec![v2(x, cy), v2(x + w, cy)])],
            Geom::Arrow => vec![open(vec![v2(x, cy), v2(x + w, cy)]), head(v2(x + w, cy), 1., 0.)],
            Geom::DoubleArrow => vec![open(vec![v2(x, cy), v2(x + w, cy)]), head(v2(x + w, cy), 1., 0.), head(v2(x, cy), -1., 0.)],
            Geom::Elbow => {
                // The top run sits low enough for its arrowhead to fit.
                let top = y + spread.min(h / 2.);
                vec![open(vec![v2(x, y + h), v2(cx, y + h), v2(cx, top), v2(x + w, top)]), head(v2(x + w, top), 1., 0.)]
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
            fill: (!geom.is_line()).then_some(Paper::Plain),
            ink: Ink::Graphite,
            line: LineWidth::Regular,
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
        for g in Geom::BASIC.iter().chain(&Geom::FLOW).chain(&Geom::LINES) {
            let o = g.outlines(r);
            assert!(!o.is_empty(), "{g:?}");
            assert_eq!(o[0].closed, !g.is_line(), "{g:?}");
            for p in o.iter().flat_map(|o| &o.points) {
                assert!(p.x >= r.x - 0.01 && p.x <= r.x + r.w + 0.01 && p.y >= r.y - 0.01 && p.y <= r.y + r.h + 0.01, "{g:?} {p:?}");
            }
        }
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
