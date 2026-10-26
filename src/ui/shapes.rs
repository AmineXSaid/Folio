//! Shapes on the sheet: painting them and the words in them, the frame and
//! handles of a selected one (resize at the corners, turn by the knob above
//! or just outside a corner), the shapes panel on the right, and the edits
//! a shape takes (adding, turning, its words). Shapes spring in as they
//! arrive and swing round when turned by a key or the menu.

use std::collections::HashMap;
use std::time::{Duration, Instant};

use gpui::component::input::Input;
use gpui::component::Sizable as _;
use gpui::{canvas, div, point, prelude::*, px, Animation, AnimationExt, Context, Hsla, MouseButton, PathBuilder, Pixels, Window};

use super::app::{Dialog, Folio, Gesture, Selection, Tool};
use super::*;
use crate::model::*;
use crate::shape::{libraries, rotate, search, Entry, SHAPE_TEXT, TEXT_PAD};
use crate::theme::{theme, Motion, Theme};

/// A selected shape's frame: how far outside the shape it's drawn, how far
/// above it the turning knob sits, and the handles' sizes (screen px).
const FRAME_GAP: f32 = 7.;
const TURN_STALK: f32 = 30.;
const TURN_R: f32 = 10.;
const TURN_R_HOT: f32 = 12.;
const CORNER: f32 = 5.;
/// How close a press must come to take a handle (screen px): a corner to
/// resize, the knob to turn; and the band just outside each corner that
/// turns the shape too.
const REACH: f32 = 9.;
const KNOB_REACH: f32 = 16.;
const TURN_BAND: f32 = 30.;

/// Which handle of a selected shape: a corner (resize) or the knob above
/// it, or the band outside a corner (turn).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ShapeGrip {
    Corner(Grip),
    Turn,
}

/// A shape picked up in the panel, being dragged onto the sheet.
#[derive(Clone, Copy)]
pub struct ShapeDrag(pub Geom);

/// What follows the pointer while a shape is dragged from the panel.
pub struct ShapeGhost {
    geom: Geom,
    ink: Hsla,
    fill: Hsla,
    w: f32,
    h: f32,
}

impl Render for ShapeGhost {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let (geom, ink, fill, w, h) = (self.geom, self.ink, self.fill, self.w, self.h);
        div().w(px(w)).h(px(h)).opacity(0.85).child(
            canvas(
                |_, _, _| {},
                move |b, _, window, _| {
                    let r = Rect { x: f32::from(b.origin.x) + 2., y: f32::from(b.origin.y) + 2., w: w - 4., h: h - 4. };
                    paint_geom(window, geom, r, ink, Some(fill), 1.75);
                },
            )
            .size_full(),
        )
    }
}

/// The corners' grips, in the order `frame` gives the corners.
const CORNER_GRIPS: [Grip; 4] = [
    Grip { left: true, right: false, top: true, bottom: false },
    Grip { left: false, right: true, top: true, bottom: false },
    Grip { left: false, right: true, top: false, bottom: true },
    Grip { left: true, right: false, top: false, bottom: true },
];

/// A selected shape's frame on screen: its four corners (top-left round
/// to bottom-left), the middle of its top edge, and the turning knob.
fn frame(s: &Shape, cam: Camera) -> ([Vec2; 4], Vec2, Vec2) {
    let g = FRAME_GAP / cam.zoom;
    let r = s.rect;
    let (x0, y0, x1, y1) = (r.x - g, r.y - g, r.x + r.w + g, r.y + r.h + g);
    let at = |p: Vec2| cam.to_screen(s.turn(p));
    let cx = r.x + r.w / 2.;
    (
        [at(v2(x0, y0)), at(v2(x1, y0)), at(v2(x1, y1)), at(v2(x0, y1))],
        at(v2(cx, y0)),
        at(v2(cx, y0 - TURN_STALK / cam.zoom)),
    )
}

/// Shapes arriving and turning: when each began, and where a turn began.
#[derive(Default)]
pub struct ShapeMotion {
    arrivals: HashMap<Id, Instant>,
    turns: HashMap<Id, (f32, Instant)>,
}

impl ShapeMotion {
    pub fn arrive(&mut self, id: Id) {
        if !super::motion::reduced() {
            self.arrivals.insert(id, Instant::now());
        }
    }
    pub fn turn_from(&mut self, id: Id, from: f32) {
        if !super::motion::reduced() {
            self.turns.insert(id, (from, Instant::now()));
        }
    }
    /// Anything still moving.
    pub fn busy(&self, m: &Motion) -> bool {
        let live = |at: &Instant, ms: u64| at.elapsed() < Duration::from_millis(super::motion::ms(ms));
        self.arrivals.values().any(|a| live(a, m.shape_in)) || self.turns.values().any(|(_, a)| live(a, m.shape_turn))
    }
    /// How far along a shape's arrival is, 0 to 1, while it lasts.
    fn arriving(&self, id: Id, m: &Motion) -> Option<f32> {
        let k = self.arrivals.get(&id)?.elapsed().as_secs_f32() * 1000. / super::motion::ms(m.shape_in) as f32;
        (k < 1.).then_some(k)
    }
    /// The angle to draw a shape at: on its way from where a turn began.
    fn rot(&self, s: &Shape, m: &Motion) -> f32 {
        let Some((from, at)) = self.turns.get(&s.id) else { return s.rot };
        let k = at.elapsed().as_secs_f32() * 1000. / super::motion::ms(m.shape_turn) as f32;
        if k >= 1. {
            return s.rot;
        }
        // The short way round.
        let d = (s.rot - from + 540.).rem_euclid(360.) - 180.;
        from + d * ease_out_cubic(k)
    }
}

fn ease_out_cubic(k: f32) -> f32 {
    1. - (1. - k).powi(3)
}

/// Overshoots a little and settles: a shape springing into place.
fn ease_out_back(k: f32) -> f32 {
    let c = 1.7;
    1. + (c + 1.) * (k - 1.).powi(3) + c * (k - 1.).powi(2)
}

fn pt(p: Vec2) -> gpui::Point<Pixels> {
    point(px(p.x), px(p.y))
}

/// Fills `lines`' body (when `fill`) and strokes each line, `o` being where
/// the canvas sits.
fn paint_outlines(window: &mut Window, lines: &[crate::shape::Outline], fill: Option<Hsla>, ink: Hsla, width: f32, dash: Option<f32>) {
    let to = |pts: &[Vec2]| pts.iter().map(|p| pt(*p)).collect::<Vec<_>>();
    if let (Some(color), Some(o)) = (fill, lines.first()) {
        if o.closed {
            let mut b = PathBuilder::fill();
            b.add_polygon(&to(&o.points), true);
            if let Ok(path) = b.build() {
                window.paint_path(path, color);
            }
        }
    }
    for o in lines {
        let mut b = PathBuilder::stroke(px(width));
        if let Some(d) = dash {
            b = b.dash_array(&[px(d), px(d * 0.7)]);
        }
        b.add_polygon(&to(&o.points), o.closed);
        if let Ok(path) = b.build() {
            window.paint_path(path, ink);
        }
    }
}

/// A logo's regions, each filled in its own colour (`o`: the canvas's
/// origin, `alpha`: how far it has faded in).
fn paint_regions(window: &mut Window, regions: &[(Vec<Vec<Vec2>>, bool, [f32; 4])], o: Vec2, alpha: f32) {
    for (rings, even_odd, c) in regions {
        let rule = if *even_odd { gpui::FillRule::EvenOdd } else { gpui::FillRule::NonZero };
        let mut b = PathBuilder::fill().with_style(gpui::PathStyle::Fill(gpui::FillOptions::default().with_fill_rule(rule)));
        for ring in rings {
            b.add_polygon(&ring.iter().map(|p| pt(v2(p.x + o.x, p.y + o.y))).collect::<Vec<_>>(), true);
        }
        if let Ok(path) = b.build() {
            window.paint_path(path, Hsla::from(gpui::Rgba { r: c[0], g: c[1], b: c[2], a: c[3] * alpha }));
        }
    }
}

/// A shape drawn small and upright in `r` (screen px): a logo in its
/// colours, anything else as lines in `ink`, filled with `fill`.
fn paint_geom(window: &mut Window, geom: Geom, r: Rect, ink: Hsla, fill: Option<Hsla>, width: f32) {
    if let Geom::Logo(_) = geom {
        let s = Shape { rect: r, ..Shape::new(0, geom, v2(0., 0.)) };
        paint_regions(window, &s.logo_regions(), v2(0., 0.), 1.);
    } else {
        paint_outlines(window, &geom.outlines(r), fill.filter(|_| geom.fills()), ink, width, None);
    }
}

/// The turning sign: three quarters of a ring with an arrowhead, round `c`.
fn paint_turn_sign(window: &mut Window, c: gpui::Point<Pixels>, r: f32, color: Hsla) {
    let (cx, cy) = (f32::from(c.x), f32::from(c.y));
    let ring: Vec<_> = (0..=18)
        .map(|i| {
            let a = -2.3 + 4.6 * i as f32 / 18.;
            pt(v2(cx + r * a.cos(), cy + r * a.sin()))
        })
        .collect();
    let mut b = PathBuilder::stroke(px(1.6));
    b.add_polygon(&ring, false);
    if let Ok(path) = b.build() {
        window.paint_path(path, color);
    }
    // The head, at the ring's end, pointing on round it.
    let a = 2.3f32;
    let tip = v2(cx + r * a.cos(), cy + r * a.sin());
    let along = v2(-a.sin(), a.cos());
    let side = v2(a.cos(), a.sin());
    let k = r * 0.75;
    let mut b = PathBuilder::fill();
    b.add_polygon(
        &[
            pt(v2(tip.x + along.x * k * 0.6, tip.y + along.y * k * 0.6)),
            pt(v2(tip.x + side.x * k * 0.55 - along.x * k * 0.3, tip.y + side.y * k * 0.55 - along.y * k * 0.3)),
            pt(v2(tip.x - side.x * k * 0.55 - along.x * k * 0.3, tip.y - side.y * k * 0.55 - along.y * k * 0.3)),
        ],
        true,
    );
    if let Ok(path) = b.build() {
        window.paint_path(path, color);
    }
}

/// One shape, ready to paint (screen px, before the canvas's origin).
struct Paint {
    lines: Vec<crate::shape::Outline>,
    fill: Option<Hsla>,
    ink: Hsla,
    width: f32,
    dash: Option<f32>,
    /// The words as outlines, when the shape is turned.
    words: Vec<Vec<Vec2>>,
    word_color: Hsla,
    /// Selected: its frame's corners, top middle and knob, and whether the
    /// knob is hot (under the pointer, or turning).
    chrome: Option<([Vec2; 4], Vec2, Vec2, bool)>,
    /// Under the pointer, not selected: a faint frame.
    hover: Option<[Vec2; 4]>,
    /// A logo's coloured regions, and how far it has faded in; on a dark
    /// sheet, the pale card behind it.
    logo: Vec<(Vec<Vec<Vec2>>, bool, [f32; 4])>,
    plate: Option<Vec<Vec2>>,
    alpha: f32,
}

impl Folio {
    /// The box a link's end `id` (a note or a shape) is drawn to.
    pub fn end_rect(&self, id: Id) -> Option<Rect> {
        let c = self.sheet();
        c.sticker(id).map(|s| self.note_rect(s)).or_else(|| c.shapes.iter().find(|s| s.id == id).map(Shape::bounds))
    }

    /// How a new link runs: notes join by a dashed curve as they always
    /// have; anything with a shape gets a straight arrow, as in a diagram.
    pub fn link_look(&self, from: Id, to: Option<Id>) -> (LinkStyle, Ends) {
        let shape = |id: Id| self.sheet().shapes.iter().any(|s| s.id == id);
        if shape(from) || to.is_some_and(shape) {
            (LinkStyle::Straight, Ends::End)
        } else {
            (LinkStyle::Curve, Ends::None)
        }
    }

    /// The link under `p` (world), within a few pixels of its run.
    pub fn link_at(&self, p: Vec2) -> Option<Id> {
        let reach = 6. / self.sheet().camera.zoom;
        self.sheet().links.iter().rev().find_map(|l| {
            let run = link_path(&self.end_rect(l.from)?, &self.end_rect(l.to)?, l.style);
            run.windows(2).any(|w| segment_distance(p, w[0], w[1]) <= reach).then_some(l.id)
        })
    }

    /// The front-most shape under `p` (world).
    pub fn shape_at(&self, p: Vec2) -> Option<Id> {
        let reach = 4. / self.sheet().camera.zoom;
        self.sheet().shapes.iter().rev().find(|s| s.hit(p, reach)).map(|s| s.id)
    }

    /// The handle of a selected shape under `at` (screen), and which: the
    /// knob above it or the band just outside a corner turn it; a corner
    /// resizes it.
    pub fn shape_grip_at(&self, at: Vec2) -> Option<(Id, ShapeGrip)> {
        let cam = self.sheet().camera;
        let picked = |s: &&Shape| !s.locked && self.is_selected(Selection::Shape(s.id));
        let shapes = &self.sheet().shapes;
        if let Some(hit) = shapes.iter().rev().filter(picked).find_map(|s| {
            let (corners, _, knob) = frame(s, cam);
            if knob.sub(at).len() <= KNOB_REACH {
                return Some((s.id, ShapeGrip::Turn));
            }
            corners.iter().position(|c| c.sub(at).len() <= REACH).map(|i| (s.id, ShapeGrip::Corner(CORNER_GRIPS[i])))
        }) {
            return Some(hit);
        }
        // Just outside a corner, off the shape: turn.
        let world = cam.to_world(at);
        if self.shape_at(world).is_some() || self.note_at(world).is_some() {
            return None;
        }
        shapes.iter().rev().filter(picked).find_map(|s| {
            let (corners, _, _) = frame(s, cam);
            let g = FRAME_GAP / cam.zoom;
            let local = s.unturn(world);
            let r = s.rect;
            let outside = local.x < r.x - g || local.x > r.x + r.w + g || local.y < r.y - g || local.y > r.y + r.h + g;
            (outside && corners.iter().any(|c| c.sub(at).len() <= TURN_BAND)).then_some((s.id, ShapeGrip::Turn))
        })
    }

    /// The pointer over a shape's handle: a resize arrow for a corner as it
    /// lies once turned, an open hand to turn.
    pub fn shape_grip_cursor(&self, id: Id, g: ShapeGrip) -> gpui::CursorStyle {
        let ShapeGrip::Corner(grip) = g else { return gpui::CursorStyle::OpenHand };
        let rot = self.sheet().shapes.iter().find(|s| s.id == id).map_or(0., |s| s.rot);
        // The corner's direction from the middle, turned, read to the
        // nearest of the two diagonals or the two axes.
        let d = rotate(v2(if grip.left { -1. } else { 1. }, if grip.top { -1. } else { 1. }), v2(0., 0.), rot);
        let a = d.y.atan2(d.x).to_degrees().rem_euclid(180.);
        use gpui::CursorStyle as C;
        match a {
            a if !(22.5..157.5).contains(&a) => C::ResizeLeftRight,
            a if a < 67.5 => C::ResizeUpLeftDownRight,
            a if a < 112.5 => C::ResizeUpDown,
            _ => C::ResizeUpRightDownLeft,
        }
    }

    /// A new `geom` at `at` (world), or in the middle of the view; selected,
    /// springing in.
    pub fn add_shape(&mut self, geom: Geom, at: Option<Vec2>, cx: &mut Context<Self>) {
        self.finish_edit(cx);
        let (w, h) = self.viewport_size();
        let cam = self.sheet().camera;
        let mut at = at.unwrap_or_else(|| cam.to_world(v2(w / 2., h / 2.)));
        // Not right on top of the last one added.
        while self.sheet().shapes.iter().any(|s| s.rect.center().sub(at).len() < 4.) {
            at = v2(at.x + 24., at.y + 24.);
        }
        let mut id = 0;
        self.change(cx, |b| {
            id = b.fresh_id();
            b.sheet_mut().shapes.push(Shape::new(id, geom, at));
        });
        self.shape_motion.arrive(id);
        self.tool = Tool::Select;
        self.select(Selection::Shape(id));
        cx.notify();
    }

    /// Changes every shape in `ids` as one undo step.
    pub fn change_shapes(&mut self, ids: &[Id], cx: &mut Context<Self>, f: impl Fn(&mut Shape)) {
        self.change(cx, |b| {
            for s in b.sheet_mut().shapes.iter_mut().filter(|s| ids.contains(&s.id)) {
                f(s);
            }
        });
    }

    /// What an action on shape `id` applies to: every selected shape when
    /// `id` is one of them, otherwise just `id`.
    pub fn shape_targets(&self, id: Id) -> Vec<Id> {
        if self.is_selected(Selection::Shape(id)) {
            self.selected_shapes()
        } else {
            vec![id]
        }
    }

    /// Turns shapes to `to(angle now)`, swinging round to it.
    pub fn set_turn(&mut self, ids: &[Id], cx: &mut Context<Self>, to: impl Fn(f32) -> f32) {
        self.menu = None;
        let ids: Vec<Id> = ids.iter().copied().filter(|id| self.sheet().shapes.iter().any(|s| s.id == *id && !s.locked)).collect();
        if ids.is_empty() {
            return;
        }
        for s in self.sheet().shapes.iter().filter(|s| ids.contains(&s.id)).map(|s| (s.id, s.rot)).collect::<Vec<_>>() {
            self.shape_motion.turn_from(s.0, s.1);
        }
        self.change_shapes(&ids, cx, |s| s.rot = to(s.rot).rem_euclid(360.));
    }

    /// Turns the shapes `by` degrees, from the nearest 15°.
    pub fn turn_shapes(&mut self, ids: &[Id], by: f32, cx: &mut Context<Self>) {
        self.set_turn(ids, cx, |r| (r / 15.).round() * 15. + by);
    }

    /// Asks for the words in shape `id`.
    pub fn edit_shape_text(&mut self, id: Id, window: &mut Window, cx: &mut Context<Self>) {
        let Some(s) = self.sheet().shapes.iter().find(|s| s.id == id) else { return };
        let text = s.text.clone();
        self.dialog = Some(Dialog::ShapeText(id));
        self.menu = None;
        self.dialog_input.update(cx, |i, cx| i.set_value(text, window, cx));
        let input = self.dialog_input.clone();
        window.defer(cx, move |window, cx| input.update(cx, |i, cx| i.focus(window, cx)));
        cx.notify();
    }

    /// Stars `geom` in the shapes panel, or unstars it.
    pub fn toggle_favourite(&mut self, geom: Geom, cx: &mut Context<Self>) {
        self.set_setting(cx, |s| {
            if s.shape_favourites.contains(&geom) {
                s.shape_favourites.retain(|g| *g != geom);
            } else {
                s.shape_favourites.push(geom);
            }
        });
    }

    /// Opens or closes the shapes panel; open, its search is ready to type.
    pub fn toggle_shapes_panel(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.shapes_panel = !self.shapes_panel;
        if self.shapes_panel {
            let input = self.shape_search.clone();
            window.defer(cx, move |window, cx| input.update(cx, |i, cx| i.focus(window, cx)));
        } else {
            window.focus(&self.focus, cx);
        }
        cx.notify();
    }

    /// The shapes in view, painted, with a selected one's frame and
    /// handles, the turning sign by the pointer and the angle while one
    /// turns; and the words of the upright ones, as text over them.
    pub(super) fn render_shapes(&self, view: Rect, t: &Theme, window: &mut Window) -> (gpui::AnyElement, Vec<gpui::AnyElement>) {
        let sheet = self.sheet();
        let cam = sheet.camera;
        let z = cam.zoom;
        let m = t.motion;
        if self.shape_motion.busy(&m) {
            window.request_animation_frame();
        }
        let turning = match &self.gesture {
            Gesture::TurnShape { id } => Some(*id),
            _ => None,
        };
        let hot = |id: Id| turning == Some(id) || (self.hover_shape_grip == Some(ShapeGrip::Turn) && self.shape_grip_at(self.cursor).is_some_and(|(g, _)| g == id));
        let mut paints = Vec::new();
        let mut words = Vec::new();
        for s0 in sheet.shapes.iter().filter(|s| s.bounds().intersects(&view)) {
            // As drawn this frame: swinging round, springing in.
            let mut s = s0.clone();
            s.rot = self.shape_motion.rot(s0, &m);
            let arriving = self.shape_motion.arriving(s.id, &m);
            if let Some(k) = arriving {
                let f = 0.82 + 0.18 * ease_out_back(k);
                let c = s.rect.center();
                s.rect = Rect { x: c.x - s.rect.w * f / 2., y: c.y - s.rect.h * f / 2., w: s.rect.w * f, h: s.rect.h * f };
            }
            let alpha = arriving.map_or(1., |k| (k * 2.5).min(1.));
            let logo: Vec<_> = s
                .logo_regions()
                .into_iter()
                .map(|(rings, eo, c)| (rings.into_iter().map(|r| r.into_iter().map(|p| cam.to_screen(p)).collect()).collect(), eo, c))
                .collect();
            let lines = if logo.is_empty() { s.outlines() } else { Vec::new() }
                .into_iter()
                .map(|mut o| {
                    o.points.iter_mut().for_each(|p| *p = cam.to_screen(*p));
                    o
                })
                .collect();
            let text = s.text.trim();
            let turned = s.rot.rem_euclid(360.) != 0.;
            let tr = s.text_rect();
            let glyphs = if turned && !text.is_empty() {
                let pad = if s.geom.is_figure() { 0. } else { TEXT_PAD };
                crate::glyphs::outlines(text, SHAPE_TEXT, tr, pad)
                    .into_iter()
                    .map(|c| c.into_iter().map(|p| cam.to_screen(s.turn(p))).collect())
                    .collect()
            } else {
                Vec::new()
            };
            if !turned && !text.is_empty() {
                let a = cam.to_screen(v2(tr.x, tr.y));
                words.push(
                    div()
                        .absolute()
                        .left(px(a.x))
                        .top(px(a.y))
                        .w(px(tr.w * z))
                        .h(px(tr.h * z))
                        .when(!s.geom.is_figure(), |d| d.p(px(TEXT_PAD * z)))
                        .flex()
                        .items_center()
                        .justify_center()
                        .overflow_hidden()
                        .text_center()
                        .text_size(px(SHAPE_TEXT * z))
                        .line_height(px(SHAPE_TEXT * crate::glyphs::LEADING * z))
                        .font_weight(gpui::FontWeight::MEDIUM)
                        .text_color(t.colors.text)
                        .opacity(alpha)
                        // A block as wide as the shape, so the words wrap
                        // inside it rather than running out of its sides.
                        .child(div().w_full().child(text.to_string()))
                        .into_any_element(),
                );
            }
            let selected = self.is_selected(Selection::Shape(s.id));
            paints.push(Paint {
                lines,
                fill: s.fill.filter(|_| s.geom.fills()).map(|p| t.paper(p).opacity(alpha)),
                ink: t.ink(s.ink).opacity(alpha),
                width: (s.line.units() * z).max(1.),
                dash: s.dashed.then_some((6. * z).max(3.)),
                words: glyphs,
                word_color: t.colors.text.opacity(alpha),
                chrome: selected.then(|| {
                    let (c, top, knob) = frame(&s, cam);
                    (c, top, knob, hot(s.id))
                }),
                hover: (!selected && self.hover_shape == Some(s.id)).then(|| frame(&s, cam).0),
                logo,
                plate: s.logo_plate().filter(|_| t.is_dark).map(|p| p.into_iter().map(|q| cam.to_screen(q)).collect()),
                alpha,
            });
            // The angle, by the knob, while it turns.
            if turning == Some(s.id) {
                let (_, _, knob) = frame(&s, cam);
                words.push(
                    div()
                        .absolute()
                        .left(px(knob.x + 18.))
                        .top(px(knob.y - 12.))
                        .px(px(7.))
                        .py(px(2.))
                        .rounded(t.radius.full)
                        .bg(t.colors.brand)
                        .text_color(t.colors.primary_fg)
                        .text_size(t.text.caption)
                        .font_weight(gpui::FontWeight::SEMIBOLD)
                        .child(format!("{}°", s.rot.round() as i32 % 360))
                        .into_any_element(),
                );
            }
        }
        // The turning sign by the pointer, over a turning handle or while
        // turning, so it's plain what a drag there does.
        let sign = (turning.is_some() || self.hover_shape_grip == Some(ShapeGrip::Turn)).then_some(self.cursor);
        // The circle a turn follows, faintly.
        let orbit = turning.and_then(|id| sheet.shapes.iter().find(|s| s.id == id)).map(|s| {
            let c = cam.to_screen(s.center());
            (c, frame(s, cam).2.sub(c).len())
        });
        let (brand, surface, on_brand) = (t.colors.brand, t.colors.surface, t.colors.primary_fg);
        let logo_plate = t.colors.logo_plate;
        let layer = canvas(
            |_, _, _| {},
            move |bounds, _, window, _| {
                let o = bounds.origin;
                let shift = |p: Vec2| v2(p.x + f32::from(o.x), p.y + f32::from(o.y));
                let at = |p: Vec2| pt(shift(p));
                for p in &paints {
                    if let Some(plate) = &p.plate {
                        let mut b = PathBuilder::fill();
                        b.add_polygon(&plate.iter().map(|q| at(*q)).collect::<Vec<_>>(), true);
                        if let Ok(path) = b.build() {
                            window.paint_path(path, logo_plate.opacity(p.alpha));
                        }
                    }
                    paint_regions(window, &p.logo, v2(f32::from(o.x), f32::from(o.y)), p.alpha);
                    let lines: Vec<crate::shape::Outline> = p
                        .lines
                        .iter()
                        .map(|l| crate::shape::Outline { points: l.points.iter().map(|q| shift(*q)).collect(), closed: l.closed })
                        .collect();
                    paint_outlines(window, &lines, p.fill, p.ink, p.width, p.dash);
                    if !p.words.is_empty() {
                        let mut b = PathBuilder::fill();
                        for c in &p.words {
                            b.add_polygon(&c.iter().map(|q| at(*q)).collect::<Vec<_>>(), true);
                        }
                        if let Ok(path) = b.build() {
                            window.paint_path(path, p.word_color);
                        }
                    }
                    if let Some(corners) = p.hover {
                        let mut b = PathBuilder::stroke(px(1.));
                        b.add_polygon(&corners.iter().map(|c| at(*c)).collect::<Vec<_>>(), true);
                        if let Ok(path) = b.build() {
                            window.paint_path(path, brand.opacity(0.4));
                        }
                    }
                    if let Some((corners, top, knob, hot)) = p.chrome {
                        let mut b = PathBuilder::stroke(px(1.25)).dash_array(&[px(4.), px(3.)]);
                        b.add_polygon(&corners.iter().map(|c| at(*c)).collect::<Vec<_>>(), true);
                        if let Ok(path) = b.build() {
                            window.paint_path(path, brand);
                        }
                        let mut b = PathBuilder::stroke(px(1.5));
                        b.add_polygon(&[at(top), at(knob)], false);
                        if let Ok(path) = b.build() {
                            window.paint_path(path, brand);
                        }
                        for c in corners {
                            let c = at(c);
                            let r = px(CORNER);
                            let q = gpui::fill(gpui::Bounds::new(point(c.x - r, c.y - r), gpui::size(r * 2., r * 2.)), surface);
                            window.paint_quad(q.border_widths(px(1.5)).border_color(brand).corner_radii(px(1.5)));
                        }
                        // The knob: white with the turning sign, brand and
                        // a little bigger when it's under the pointer.
                        let r = if hot { TURN_R_HOT } else { TURN_R };
                        let c = at(knob);
                        let bounds = gpui::Bounds::new(point(c.x - px(r), c.y - px(r)), gpui::size(px(2. * r), px(2. * r)));
                        let face = if hot { brand } else { surface };
                        window.paint_quad(gpui::fill(bounds, face).border_widths(px(1.75)).border_color(brand).corner_radii(px(r)));
                        paint_turn_sign(window, c, r * 0.5, if hot { on_brand } else { brand });
                    }
                }
                if let Some((c, r)) = orbit {
                    let ring: Vec<_> = (0..72).map(|i| {
                        let a = std::f32::consts::TAU * i as f32 / 72.;
                        at(v2(c.x + r * a.cos(), c.y + r * a.sin()))
                    }).collect();
                    let mut b = PathBuilder::stroke(px(1.)).dash_array(&[px(3.), px(4.)]);
                    b.add_polygon(&ring, true);
                    if let Ok(path) = b.build() {
                        window.paint_path(path, brand.opacity(0.35));
                    }
                }
                if let Some(p) = sign {
                    let c = at(v2(p.x + 22., p.y + 22.));
                    let r = px(10.);
                    let bounds = gpui::Bounds::new(point(c.x - r, c.y - r), gpui::size(r * 2., r * 2.));
                    window.paint_quad(gpui::fill(bounds, brand).corner_radii(r));
                    paint_turn_sign(window, c, 5., on_brand);
                }
            },
        )
        .absolute()
        .inset_0()
        .into_any_element();
        (layer, words)
    }

    /// The dock's Shapes button: opens and closes the panel.
    pub(super) fn shapes_button(&self, cx: &mut Context<Self>) -> gpui::AnyElement {
        let t = theme(cx).clone();
        let on = self.shapes_panel;
        let tip = format!("Shapes   {}", shortcut("Shift+S"));
        div()
            .id("dock-shapes")
            .size(px(38.))
            .flex()
            .items_center()
            .justify_center()
            .rounded(px(11.))
            .cursor_pointer()
            .tooltip(move |window, cx| super::tip(tip.clone(), window, cx))
            .when(on, |d| d.bg(t.colors.brand_tint))
            .when(!on, |d| d.hover(|d| d.bg(t.colors.hover)).active(|d| d.bg(t.colors.border)))
            .child(icon("icons/shapes.svg", px(17.), if on { t.colors.brand_on_tint } else { t.colors.text_muted }))
            .on_click(cx.listener(|this, _, window, cx| this.toggle_shapes_panel(window, cx)))
            .into_any_element()
    }

    /// The shapes panel, sliding in from the right while open.
    pub(super) fn render_shapes_panel(&mut self, cx: &mut Context<Self>) -> Option<gpui::AnyElement> {
        let t = theme(cx).clone();
        let w = f32::from(t.size.shapes_panel);
        if self.shapes_panel {
            let ms = super::motion::ms(t.motion.drawer_in);
            let panel = self.shapes_panel_card(&t, cx).with_animation(
                "shapes-panel-in",
                Animation::new(Duration::from_millis(ms)).with_easing(ease_out_cubic),
                move |d, k| d.right(px(-w * (1. - k))).opacity(0.4 + 0.6 * k),
            );
            return Some(panel.into_any_element());
        }
        let (_, n) = self.exits.shapes_panel.leaving(t.motion.drawer_out)?;
        let ms = super::motion::ms(t.motion.drawer_out);
        let panel = self
            .shapes_panel_card(&t, cx)
            .child(div().absolute().inset_0().occlude())
            .with_animation(("shapes-panel-out", n), Animation::new(Duration::from_millis(ms)).with_easing(|k: f32| k * k * k), move |d, k| {
                d.right(px(-w * k)).opacity(1. - 0.6 * k)
            });
        Some(panel.into_any_element())
    }

    /// The panel: its search, then every family of shapes, each folding
    /// open; or what the search found.
    fn shapes_panel_card(&mut self, t: &Theme, cx: &mut Context<Self>) -> gpui::Stateful<gpui::Div> {
        let q = self.shape_search.read(cx).value().trim().to_string();
        let mut body: Vec<gpui::AnyElement> = Vec::new();
        if q.is_empty() {
            // Starred shapes first, then every family not put away.
            let favs = &self.board.settings.shape_favourites;
            let starred = crate::shape::Library {
                id: "favourites",
                name: "Favourites",
                entries: favs.iter().map(|&g| Entry { geom: g, name: g.name(), keywords: "" }).collect(),
            };
            let hidden = self.board.settings.shape_hidden.clone();
            let libs = std::iter::once(starred)
                .filter(|l| !l.entries.is_empty())
                .chain(libraries().into_iter().filter(|l| !hidden.iter().any(|h| h == l.id)));
            if favs.is_empty() {
                body.push(
                    div()
                        .mx(t.space.md)
                        .mb(t.space.sm)
                        .flex()
                        .items_center()
                        .gap(px(6.))
                        .text_size(t.text.micro)
                        .text_color(t.colors.text_subtle)
                        .child(icon("icons/star.svg", px(11.), t.colors.text_subtle))
                        .child("Right-click a shape to star it: starred ones stay up here.")
                        .into_any_element(),
                );
            }
            for (li, lib) in libs.enumerate() {
                let open = self.shape_open.contains(lib.id);
                let id = lib.id;
                let head = div()
                    .id(gpui::ElementId::Name(format!("shape-lib-{id}").into()))
                    .h(px(34.))
                    .px(t.space.sm)
                    .mx(t.space.xs)
                    .flex()
                    .items_center()
                    .gap(t.space.sm)
                    .rounded(t.radius.button)
                    .cursor_pointer()
                    .hover(|d| d.bg(t.colors.hover))
                    .active(|d| d.bg(t.colors.border))
                    .child(icon(if open { "icons/chevron-down.svg" } else { "icons/chevron-right.svg" }, t.size.icon_sm, t.colors.text_muted))
                    .child(div().flex_1().text_size(t.text.body).font_weight(gpui::FontWeight::SEMIBOLD).text_color(t.colors.text).child(lib.name))
                    .child(div().text_size(t.text.micro).text_color(t.colors.text_subtle).child(lib.entries.len().to_string()))
                    .on_click(cx.listener(move |this, _, _, cx| {
                        if !this.shape_open.remove(id) {
                            this.shape_open.insert(id);
                        }
                        cx.notify();
                    }));
                body.push(super::motion::row_in(head, ("shape-lib-in", li), li, t));
                if open {
                    let tiles: Vec<_> = lib.entries.iter().enumerate().map(|(i, e)| self.shape_tile(*e, lib.name, format!("{id}-{i}"), t, cx)).collect();
                    let grid = div().flex().flex_wrap().gap(px(2.)).px(t.space.sm).pb(t.space.sm).children(tiles);
                    body.push(super::motion::enter(grid, gpui::ElementId::Name(format!("shape-grid-{id}").into()), t.motion.pop_in, t.motion.rise));
                }
            }
        } else {
            let found = search(&q);
            body.push(
                div()
                    .px(t.space.md)
                    .pt(t.space.xs)
                    .pb(t.space.sm)
                    .text_size(t.text.caption)
                    .text_color(t.colors.text_muted)
                    .child(match found.len() {
                        0 => format!("Nothing called “{q}”"),
                        1 => format!("1 shape for “{q}” · Enter adds it"),
                        n => format!("{n} shapes for “{q}” · Enter adds the first"),
                    })
                    .into_any_element(),
            );
            if found.is_empty() {
                body.push(
                    div().px(t.space.md).text_size(t.text.caption).text_color(t.colors.text_subtle).child("Try another word: router, person, decision, arrow…").into_any_element(),
                );
            }
            let tiles: Vec<_> = found.iter().enumerate().map(|(i, (e, lib))| self.shape_tile(*e, lib, format!("found-{i}"), t, cx)).collect();
            body.push(
                super::motion::enter(
                    div().flex().flex_wrap().gap(px(2.)).px(t.space.sm).children(tiles),
                    gpui::ElementId::Name(format!("shape-found-{q}").into()),
                    t.motion.pop_in,
                    t.motion.rise,
                ),
            );
        }
        let chips = ["wifi", "router", "person", "decision", "arrow"].map(|word| {
            div()
                .id(gpui::ElementId::Name(format!("shape-chip-{word}").into()))
                .h(px(22.))
                .px(px(9.))
                .flex()
                .items_center()
                .rounded(t.radius.full)
                .border_1()
                .border_color(t.colors.border)
                .bg(t.colors.surface_sunk)
                .text_size(t.text.micro)
                .text_color(t.colors.text_body)
                .cursor_pointer()
                .hover(|d| d.bg(t.colors.hover).border_color(t.colors.text_subtle))
                .on_click(cx.listener(move |this, _, window, cx| {
                    this.shape_search.update(cx, |i, cx| i.set_value(word, window, cx));
                    cx.notify();
                }))
                .child(word)
        });
        let has_query = !q.is_empty();
        // The families to show, ticked; opened from the panel's foot.
        let families = self.shape_families.then(|| {
            let hidden = self.board.settings.shape_hidden.clone();
            let rows: Vec<_> = libraries()
                .into_iter()
                .map(|lib| {
                    let id = lib.id;
                    let on = !hidden.iter().any(|h| h == id);
                    div()
                        .id(gpui::ElementId::Name(format!("shape-family-{id}").into()))
                        .h(px(28.))
                        .px(t.space.sm)
                        .flex()
                        .items_center()
                        .gap(t.space.sm)
                        .rounded(t.radius.small)
                        .cursor_pointer()
                        .hover(|d| d.bg(t.colors.hover))
                        .child(
                            div()
                                .size(px(16.))
                                .flex()
                                .items_center()
                                .justify_center()
                                .rounded(px(4.))
                                .border_1()
                                .border_color(if on { t.colors.brand } else { t.colors.border })
                                .when(on, |d| d.bg(t.colors.brand))
                                .when(on, |d| d.child(icon("icons/check.svg", px(11.), t.colors.primary_fg))),
                        )
                        .child(div().flex_1().text_size(t.text.caption).text_color(t.colors.text).child(lib.name))
                        .child(div().text_size(t.text.micro).text_color(t.colors.text_subtle).child(lib.entries.len().to_string()))
                        .on_click(cx.listener(move |this, _, _, cx| {
                            this.set_setting(cx, |s| {
                                if s.shape_hidden.iter().any(|h| h == id) {
                                    s.shape_hidden.retain(|h| h != id);
                                } else {
                                    s.shape_hidden.push(id.to_string());
                                }
                            })
                        }))
                })
                .collect();
            super::motion::enter(
                div().px(t.space.sm).pb(t.space.xs).flex().flex_col().gap(px(1.)).children(rows),
                "shape-families",
                t.motion.pop_in,
                t.motion.rise,
            )
        });
        div()
            .id("shapes-panel")
            .occlude()
            .absolute()
            .top_0()
            .bottom_0()
            .right_0()
            .w(t.size.shapes_panel)
            .flex()
            .flex_col()
            .bg(t.colors.surface)
            .border_l_1()
            .border_color(t.colors.border)
            .shadow(t.shadow_float())
            .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
            .child(
                div()
                    .px(t.space.md)
                    .pt(t.space.md)
                    .pb(t.space.sm)
                    .flex()
                    .flex_col()
                    .gap(t.space.sm)
                    .border_b_1()
                    .border_color(t.colors.border)
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(t.space.sm)
                            .child(icon("icons/shapes.svg", t.size.icon, t.colors.brand))
                            .child(div().flex_1().text_size(t.text.title).font_weight(gpui::FontWeight::SEMIBOLD).child("Shapes"))
                            .child(div().px(px(5.)).py(px(1.)).rounded(t.radius.small).border_1().border_color(t.colors.border).text_size(t.text.micro).text_color(t.colors.text_subtle).child(shortcut("Shift+S")))
                            .child(
                                icon_button("shapes-close", "icons/x.svg", t)
                                    .tooltip(|window, cx| super::tip("Close", window, cx))
                                    .on_click(cx.listener(|this, _, window, cx| this.toggle_shapes_panel(window, cx))),
                            ),
                    )
                    .child(
                        div()
                            .h(px(34.))
                            .px(t.space.sm)
                            .flex()
                            .items_center()
                            .gap(t.space.xs)
                            .rounded(t.radius.button)
                            .border_1()
                            .border_color(if has_query { t.colors.brand } else { t.colors.border })
                            .bg(t.colors.surface_sunk)
                            .child(icon("icons/search.svg", t.size.icon_sm, t.colors.text_muted))
                            .child(div().flex_1().child(Input::new(&self.shape_search).appearance(false).small()))
                            .when(has_query, |d| {
                                d.child(
                                    div()
                                        .id("shape-search-clear")
                                        .size(px(20.))
                                        .flex()
                                        .items_center()
                                        .justify_center()
                                        .rounded(t.radius.small)
                                        .cursor_pointer()
                                        .hover(|d| d.bg(t.colors.hover))
                                        .child(icon("icons/x.svg", px(12.), t.colors.text_muted))
                                        .on_click(cx.listener(|this, _, window, cx| {
                                            this.shape_search.update(cx, |i, cx| i.set_value("", window, cx));
                                            cx.notify();
                                        })),
                                )
                            }),
                    )
                    .when(!has_query, |d| d.child(div().flex().flex_wrap().gap(px(5.)).children(chips))),
            )
            .child(div().id("shapes-list").flex_1().min_h_0().overflow_y_scroll().py(t.space.sm).children(body))
            .children(families.map(|f| div().border_t_1().border_color(t.colors.border).pt(t.space.xs).child(f)))
            .child(
                div()
                    .px(t.space.md)
                    .py(t.space.sm)
                    .border_t_1()
                    .border_color(t.colors.border)
                    .flex()
                    .flex_col()
                    .gap(px(3.))
                    .text_size(t.text.micro)
                    .text_color(t.colors.text_muted)
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .child(div().flex_1().child("Click to add, or drag onto the sheet."))
                            .child(
                                div()
                                    .id("shape-families-toggle")
                                    .px(px(6.))
                                    .py(px(2.))
                                    .rounded(t.radius.small)
                                    .flex()
                                    .items_center()
                                    .gap(px(3.))
                                    .cursor_pointer()
                                    .text_color(if self.shape_families { t.colors.brand } else { t.colors.text_muted })
                                    .hover(|d| d.bg(t.colors.hover))
                                    .child("Families")
                                    .child(icon(if self.shape_families { "icons/chevron-down.svg" } else { "icons/chevron-right.svg" }, px(11.), t.colors.text_muted))
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.shape_families = !this.shape_families;
                                        cx.notify();
                                    })),
                            ),
                    )
                    .child(
                        div()
                            .flex()
                            .gap(px(5.))
                            .child(div().pt(px(2.)).child(icon("icons/rotate-cw.svg", px(11.), t.colors.text_muted)))
                            .child(div().flex_1().min_w_0().child("Turn: drag the round knob or just outside a corner. [ and ] turn by 15°, Shift turns freely.")),
                    ),
            )
    }

    /// One shape to pick: drawn as it comes; click to add it in the middle
    /// of the view, or drag it onto the sheet.
    fn shape_tile(&self, e: Entry, lib: &'static str, key: String, t: &Theme, cx: &mut Context<Self>) -> gpui::Stateful<gpui::Div> {
        let (geom, name) = (e.geom, e.name);
        let ink = t.colors.text_body;
        let starred = self.board.settings.shape_favourites.contains(&geom);
        let tip = format!("{name}  ·  {lib}  ·  right-click to {}", if starred { "unstar" } else { "star" });
        let ghost_ink = t.ink(Ink::Graphite);
        let ghost_fill = t.paper(Paper::Plain);
        let zoom = self.sheet().camera.zoom;
        div()
            .id(gpui::ElementId::Name(format!("shape-tile-{key}").into()))
            .size(t.size.shape_tile)
            .rounded(t.radius.button)
            .border_1()
            .border_color(gpui::transparent_black())
            // A logo on a dark panel sits on a pale card, as on the sheet.
            .when(t.is_dark && matches!(geom, Geom::Logo(_)), |d| d.bg(t.colors.logo_plate))
            .cursor_pointer()
            .hover(|d| d.bg(t.colors.hover).border_color(t.colors.border))
            .active(|d| d.bg(t.colors.brand_tint).border_color(t.colors.brand))
            .tooltip(move |window, cx| super::tip(tip.clone(), window, cx))
            .on_click(cx.listener(move |this, _, _, cx| this.add_shape(geom, None, cx)))
            .on_mouse_down(MouseButton::Right, cx.listener(move |this, _, _, cx| {
                cx.stop_propagation();
                this.toggle_favourite(geom, cx);
            }))
            .relative()
            .when(starred, |d| d.child(div().absolute().top(px(2.)).right(px(2.)).child(icon("icons/star.svg", px(9.), t.colors.brand))))
            .on_drag(ShapeDrag(geom), move |d, _, _, cx| {
                let (w, h) = d.0.default_size();
                let k = zoom.clamp(0.4, 1.2);
                cx.new(|_| ShapeGhost { geom: d.0, ink: ghost_ink, fill: ghost_fill, w: w * k, h: h * k })
            })
            .child(
                canvas(
                    |_, _, _| {},
                    move |b, _, window, _| {
                        let (bw, bh) = (f32::from(b.size.width), f32::from(b.size.height));
                        // Fitted to the tile, keeping its own proportions.
                        let (dw, dh) = geom.default_size();
                        let room = if geom.is_figure() { 26. } else { bw - 16. };
                        let k = (room / dw).min((bh - 18.) / dh).min(if geom.is_figure() { 26. / dw.max(dh) } else { f32::MAX });
                        let (w, h) = ((dw * k).max(6.), (dh * k).max(6.));
                        let r = Rect { x: f32::from(b.origin.x) + (bw - w) / 2., y: f32::from(b.origin.y) + (bh - h) / 2., w, h };
                        paint_geom(window, geom, r, ink, None, 1.4);
                    },
                )
                .size_full(),
            )
    }
}
