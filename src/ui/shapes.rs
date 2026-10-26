//! Shapes on the sheet: painting them and the words in them, the frame and
//! handles of a selected one, the picker above the dock, and the edits a
//! shape takes (adding, turning, its words).

use gpui::{canvas, div, point, prelude::*, px, Context, Hsla, PathBuilder, Pixels, Window};

use super::app::{Dialog, Folio, Selection, Tool};
use super::*;
use crate::model::*;
use crate::shape::{rotate, SHAPE_TEXT, TEXT_PAD};
use crate::theme::{theme, Theme};

/// A selected shape's frame: how far outside the shape it's drawn, how far
/// above it the turning handle sits, and the handles' sizes (screen px).
const FRAME_GAP: f32 = 6.;
const TURN_STALK: f32 = 22.;
const TURN_R: f32 = 6.5;
const CORNER: f32 = 4.5;
/// How close a press must come to a handle to take it (screen px).
const REACH: f32 = 9.;

/// Which handle of a selected shape: a corner (resize) or the knob above
/// it (turn).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ShapeGrip {
    Corner(Grip),
    Turn,
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

/// One shape, ready to paint (screen px).
struct Paint {
    fill: Option<(Vec<gpui::Point<Pixels>>, Hsla)>,
    lines: Vec<(Vec<gpui::Point<Pixels>>, bool)>,
    ink: Hsla,
    width: f32,
    dash: Option<f32>,
    /// The words as outlines, when the shape is turned.
    words: Vec<Vec<gpui::Point<Pixels>>>,
    word_color: Hsla,
    /// Selected: its frame's corners, top middle and knob.
    chrome: Option<([Vec2; 4], Vec2, Vec2)>,
}

fn pt(p: Vec2) -> gpui::Point<Pixels> {
    point(px(p.x), px(p.y))
}

impl Folio {
    /// The front-most shape under `p` (world).
    pub fn shape_at(&self, p: Vec2) -> Option<Id> {
        let reach = 4. / self.sheet().camera.zoom;
        self.sheet().shapes.iter().rev().find(|s| s.hit(p, reach)).map(|s| s.id)
    }

    /// The handle of a selected shape under `at` (screen), and which.
    pub fn shape_grip_at(&self, at: Vec2) -> Option<(Id, ShapeGrip)> {
        let cam = self.sheet().camera;
        self.sheet().shapes.iter().rev().filter(|s| !s.locked && self.is_selected(Selection::Shape(s.id))).find_map(|s| {
            let (corners, _, knob) = frame(s, cam);
            if knob.sub(at).len() <= REACH {
                return Some((s.id, ShapeGrip::Turn));
            }
            corners.iter().position(|c| c.sub(at).len() <= REACH).map(|i| (s.id, ShapeGrip::Corner(CORNER_GRIPS[i])))
        })
    }

    /// The pointer over a shape's handle: a resize arrow for a corner as
    /// it lies once turned, a hand for the knob.
    pub fn shape_grip_cursor(&self, id: Id, g: ShapeGrip) -> gpui::CursorStyle {
        let ShapeGrip::Corner(grip) = g else { return gpui::CursorStyle::PointingHand };
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

    /// A new `geom` in the middle of the view, selected.
    pub fn add_shape(&mut self, geom: Geom, cx: &mut Context<Self>) {
        self.finish_edit(cx);
        self.shape_picker = false;
        let (w, h) = self.viewport_size();
        let cam = self.sheet().camera;
        let mut at = cam.to_world(v2(w / 2., h / 2.));
        // Not right on top of the last one added.
        while self.sheet().shapes.iter().any(|s| s.rect.center().sub(at).len() < 4.) {
            at = v2(at.x + 24., at.y + 24.);
        }
        let mut id = 0;
        self.change(cx, |b| {
            id = b.fresh_id();
            b.sheet_mut().shapes.push(Shape::new(id, geom, at));
        });
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

    /// Turns the selected shapes `by` degrees, from the nearest 15°.
    pub fn turn_shapes(&mut self, ids: &[Id], by: f32, cx: &mut Context<Self>) {
        self.menu = None;
        let ids: Vec<Id> = ids.iter().copied().filter(|id| self.sheet().shapes.iter().any(|s| s.id == *id && !s.locked)).collect();
        if ids.is_empty() {
            return;
        }
        self.change_shapes(&ids, cx, |s| s.rot = ((s.rot / 15.).round() * 15. + by).rem_euclid(360.));
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

    /// The shapes in view, painted, with a selected one's frame and
    /// handles; and the words of the upright ones, as text over them.
    pub(super) fn render_shapes(&self, view: Rect, t: &Theme) -> (gpui::AnyElement, Vec<gpui::AnyElement>) {
        let sheet = self.sheet();
        let cam = sheet.camera;
        let z = cam.zoom;
        let mut paints = Vec::new();
        let mut words = Vec::new();
        for s in sheet.shapes.iter().filter(|s| s.bounds().intersects(&view)) {
            let outlines = s.outlines();
            let screen = |pts: &[Vec2]| pts.iter().map(|p| pt(cam.to_screen(*p))).collect::<Vec<_>>();
            let fill = match (s.fill, outlines.first()) {
                (Some(p), Some(o)) if o.closed => Some((screen(&o.points), t.paper(p))),
                _ => None,
            };
            let width = (s.line.units() * z).max(1.);
            let text = s.text.trim();
            let turned = s.rot.rem_euclid(360.) != 0.;
            let glyphs = if turned && !text.is_empty() {
                crate::glyphs::outlines(text, SHAPE_TEXT, s.rect, TEXT_PAD)
                    .into_iter()
                    .map(|c| c.into_iter().map(|p| pt(cam.to_screen(s.turn(p)))).collect())
                    .collect()
            } else {
                Vec::new()
            };
            if !turned && !text.is_empty() {
                let a = cam.to_screen(v2(s.rect.x, s.rect.y));
                words.push(
                    div()
                        .absolute()
                        .left(px(a.x))
                        .top(px(a.y))
                        .w(px(s.rect.w * z))
                        .h(px(s.rect.h * z))
                        .p(px(TEXT_PAD * z))
                        .flex()
                        .items_center()
                        .justify_center()
                        .overflow_hidden()
                        .text_center()
                        .text_size(px(SHAPE_TEXT * z))
                        .line_height(px(SHAPE_TEXT * crate::glyphs::LEADING * z))
                        .font_weight(gpui::FontWeight::MEDIUM)
                        .text_color(t.colors.text)
                        .child(text.to_string())
                        .into_any_element(),
                );
            }
            paints.push(Paint {
                fill,
                lines: outlines.iter().map(|o| (screen(&o.points), o.closed)).collect(),
                ink: t.ink(s.ink),
                width,
                dash: s.dashed.then_some((6. * z).max(3.)),
                words: glyphs,
                word_color: t.colors.text,
                chrome: self.is_selected(Selection::Shape(s.id)).then(|| frame(s, cam)),
            });
        }
        let (brand, surface) = (t.colors.brand, t.colors.surface);
        let layer = canvas(
            |_, _, _| {},
            move |bounds, _, window, _| {
                let o = bounds.origin;
                let at = |p: &gpui::Point<Pixels>| *p + o;
                for p in &paints {
                    if let Some((pts, color)) = &p.fill {
                        let mut b = PathBuilder::fill();
                        b.add_polygon(&pts.iter().map(at).collect::<Vec<_>>(), true);
                        if let Ok(path) = b.build() {
                            window.paint_path(path, *color);
                        }
                    }
                    for (pts, closed) in &p.lines {
                        let mut b = PathBuilder::stroke(px(p.width));
                        if let Some(d) = p.dash {
                            b = b.dash_array(&[px(d), px(d * 0.7)]);
                        }
                        b.add_polygon(&pts.iter().map(at).collect::<Vec<_>>(), *closed);
                        if let Ok(path) = b.build() {
                            window.paint_path(path, p.ink);
                        }
                    }
                    if !p.words.is_empty() {
                        let mut b = PathBuilder::fill();
                        for c in &p.words {
                            b.add_polygon(&c.iter().map(at).collect::<Vec<_>>(), true);
                        }
                        if let Ok(path) = b.build() {
                            window.paint_path(path, p.word_color);
                        }
                    }
                    if let Some((corners, top, knob)) = p.chrome {
                        let ring: Vec<_> = corners.iter().map(|c| at(&pt(*c))).collect();
                        let mut b = PathBuilder::stroke(px(1.25)).dash_array(&[px(4.), px(3.)]);
                        b.add_polygon(&ring, true);
                        if let Ok(path) = b.build() {
                            window.paint_path(path, brand);
                        }
                        let mut b = PathBuilder::stroke(px(1.25));
                        b.add_polygon(&[at(&pt(top)), at(&pt(knob))], false);
                        if let Ok(path) = b.build() {
                            window.paint_path(path, brand);
                        }
                        let dot = |c: Vec2, r: f32, round: bool, window: &mut Window| {
                            let c = at(&pt(c));
                            let bounds = gpui::Bounds::new(point(c.x - px(r), c.y - px(r)), gpui::size(px(2. * r), px(2. * r)));
                            let q = gpui::fill(bounds, surface).border_widths(px(1.5)).border_color(brand);
                            window.paint_quad(if round { q.corner_radii(px(r)) } else { q.corner_radii(px(1.5)) });
                        };
                        for c in corners {
                            dot(c, CORNER, false, window);
                        }
                        dot(knob, TURN_R, true, window);
                    }
                }
            },
        )
        .absolute()
        .inset_0()
        .into_any_element();
        (layer, words)
    }

    /// The shapes to add, above the dock: boxes, flowchart symbols, lines
    /// and arrows, each drawn as it comes.
    pub(super) fn render_shape_picker(&self, t: &Theme, cx: &mut Context<Self>) -> Option<gpui::AnyElement> {
        if !self.shape_picker {
            return None;
        }
        let ink = t.colors.text_body;
        let tile = |g: Geom, cx: &mut Context<Self>| {
            let name = g.name();
            div()
                .id(("shape-tile", g as usize))
                .size(px(40.))
                .rounded(t.radius.button)
                .cursor_pointer()
                .hover(|d| d.bg(t.colors.hover))
                .tooltip(move |window, cx| super::tip(name, window, cx))
                .on_click(cx.listener(move |this, _, _, cx| this.add_shape(g, cx)))
                .child(
                    canvas(
                        |_, _, _| {},
                        move |bounds, _, window, _| {
                            let (w, h) = (f32::from(bounds.size.width), f32::from(bounds.size.height));
                            let o = bounds.origin;
                            let r = Rect { x: f32::from(o.x) + 7., y: f32::from(o.y) + 10., w: w - 14., h: h - 20. };
                            for ol in g.outlines(r) {
                                let mut b = PathBuilder::stroke(px(1.5));
                                b.add_polygon(&ol.points.iter().map(|p| pt(*p)).collect::<Vec<_>>(), ol.closed);
                                if let Ok(path) = b.build() {
                                    window.paint_path(path, ink);
                                }
                            }
                        },
                    )
                    .size_full(),
                )
        };
        let group = |title: &'static str, geoms: &[Geom], cx: &mut Context<Self>| {
            div()
                .flex()
                .flex_col()
                .gap(px(2.))
                .child(div().px(px(4.)).text_size(t.text.micro).text_color(t.colors.text_muted).child(title))
                .child(div().flex().flex_wrap().w(px(5. * 40. + 4. * 2.)).gap(px(2.)).children(geoms.iter().map(|&g| tile(g, cx))))
        };
        Some(
            super::motion::rise_in(
                div()
                    .id("shape-picker")
                    .occlude()
                    .mb(t.space.sm)
                    .p(t.space.sm)
                    .flex()
                    .gap(t.space.md)
                    .rounded(t.radius.card)
                    .bg(t.colors.surface)
                    .border_1()
                    .border_color(t.colors.border)
                    .shadow(t.shadow_float())
                    .on_mouse_down(gpui::MouseButton::Left, |_, _, cx| cx.stop_propagation())
                    .child(group("Basic", &Geom::BASIC, cx))
                    .child(div().flex().flex_col().gap(t.space.sm).child(group("Flowchart", &Geom::FLOW, cx)).child(group("Lines & arrows", &Geom::LINES, cx))),
                "shape-picker-in",
                t,
            )
            .into_any_element(),
        )
    }

    /// The dock's Shapes button: opens and closes the picker.
    pub(super) fn shapes_button(&self, cx: &mut Context<Self>) -> gpui::AnyElement {
        let t = theme(cx).clone();
        let on = self.shape_picker;
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
            .when(!on, |d| d.hover(|d| d.bg(t.colors.hover)))
            .child(icon("icons/shapes.svg", px(17.), if on { t.colors.brand_on_tint } else { t.colors.text_muted }))
            .on_click(cx.listener(|this, _, _, cx| {
                this.shape_picker = !this.shape_picker;
                cx.notify();
            }))
            .into_any_element()
    }
}
