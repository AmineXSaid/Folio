//! The sheet: an endless dotted canvas you pan and zoom. Areas, ink and
//! links are painted beneath the notes; labels and notes lie on top.
//!
//! One handler set on the sheet does all hit testing against the model
//! (front-most first), so a press means the same thing whatever element the
//! pointer happens to be over. Only what's in view is laid out. The note
//! being typed into is drawn over the sheet as a sibling of it, so its text
//! fields aren't inside the sheet's single-letter shortcuts.

use gpui::component::input::{Input, Textarea};
use gpui::component::Sizable as _;
use gpui::{
    canvas, div, point, prelude::*, px, Context, Hsla, KeyDownEvent, KeyUpEvent, MouseButton, MouseDownEvent,
    MouseMoveEvent, PathBuilder, Pixels, ScrollWheelEvent, Window,
};

use std::time::Instant;

use super::app::{Folio, Format, Gesture, MenuKind, MenuPage, Selection, Tool};
use super::shapes::{ShapeDrag, ShapeGrip};
use super::sticker::size_factor;
use super::*;
use crate::history::Before;
use crate::model::*;
use crate::shape::{handle_angle, rotate, MIN_SHAPE};
use crate::theme::{theme, Theme};

/// How close, in screen pixels, a press must be to an edge or corner to
/// resize (either side of it).
/// How far a note's edge reaches for a resize, in screen pixels: inside
/// the note, beyond it, and along an edge from a corner.
const GRIP_IN: f32 = 10.;
const GRIP_OUT: f32 = 8.;
const GRIP_CORNER: f32 = 18.;
/// How close, in screen pixels, edges must come to snap together.
const SNAP: f32 = 6.;
/// An area's title bar, in world units: press there to move the area.
const FRAME_BAR: f32 = 40.;
/// The "move to sheet" tray: how close to the left edge a drag must come,
/// and its layout (sheet-local pixels).
const TRAY_REACH: f32 = 56.;
const TRAY_W: f32 = 240.;
const TRAY_TOP: f32 = 12.;
const TRAY_HEAD: f32 = 44.;
const TRAY_ROW: f32 = 44.;

/// A link ready to paint: its run on screen, colour, whether it's dashed,
/// its arrowheads and whether it's picked.
struct LinkPaint {
    run: Vec<gpui::Point<Pixels>>,
    color: Hsla,
    dashed: bool,
    heads: Vec<[gpui::Point<Pixels>; 3]>,
    selected: bool,
}

/// A stroke ready to paint: screen points, colour, width.
struct InkPath {
    points: Vec<gpui::Point<Pixels>>,
    color: Hsla,
    width: f32,
}

impl Folio {
    pub(super) fn render_board(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let t = theme(cx).clone();
        // Keep drawing while dropped notes settle.
        self.settling.retain(|(_, at)| at.elapsed().as_secs_f32() < 0.4);
        if !self.settling.is_empty() {
            window.request_animation_frame();
        }
        // Notes arriving and leaving, pins landing: keep drawing while they do.
        if self.exits.notes.follow(self.board.sheet(), &t.motion) {
            window.request_animation_frame();
        }
        let sheet = self.sheet();
        let cam = sheet.camera;
        let (vw, vh) = self.viewport_size();
        // A margin, so things slide in rather than pop in at the edges.
        let mut view = cam.visible(vw, vh);
        let m = 200. / cam.zoom;
        view = Rect { x: view.x - m, y: view.y - m, w: view.w + 2. * m, h: view.h + 2. * m };
        let screen = |p: Vec2| {
            let s = cam.to_screen(p);
            point(px(s.x), px(s.y))
        };

        // Ink beneath the notes: links, then strokes.
        let mut paths: Vec<InkPath> = Vec::new();
        for s in sheet.strokes.iter().filter(|s| s.bounds().intersects(&view)) {
            let mut color = t.ink(s.ink);
            if s.marker {
                color = color.opacity(if t.is_dark { 0.35 } else { 0.3 });
            }
            paths.push(InkPath { points: s.points.iter().map(|p| screen(*p)).collect(), color, width: s.width * cam.zoom });
        }
        if let Gesture::Draw(s) = &self.gesture {
            let mut color = t.ink(s.ink);
            if s.marker {
                color = color.opacity(0.3);
            }
            paths.push(InkPath { points: s.points.iter().map(|p| screen(*p)).collect(), color, width: s.width * cam.zoom });
        }
        // Links: their runs on screen, colour, dashed or not, arrowheads,
        // and a dot where a plain one ends.
        let mut curves: Vec<LinkPaint> = Vec::new();
        let head = 12. * cam.zoom.max(0.6);
        let mut add_link = |run: Vec<Vec2>, color: Hsla, style: LinkStyle, ends: Ends, selected: bool| {
            let n = run.len();
            let mut heads = Vec::new();
            if n >= 2 && ends != Ends::None {
                heads.push(arrow_head(run[n - 1], run[n - 2], head / cam.zoom).map(screen));
                if ends == Ends::Both {
                    heads.push(arrow_head(run[0], run[1], head / cam.zoom).map(screen));
                }
            }
            curves.push(LinkPaint {
                run: run.into_iter().map(screen).collect(),
                color,
                dashed: style == LinkStyle::Curve && ends == Ends::None,
                heads,
                selected,
            });
        };
        for l in &sheet.links {
            if let (Some(ra), Some(rb)) = (self.end_rect(l.from), self.end_rect(l.to)) {
                if ra.union(&rb).intersects(&view) {
                    let selected = self.is_selected(Selection::Link(l.id));
                    add_link(link_path(&ra, &rb, l.style), t.ink(l.ink).opacity(0.85), l.style, l.ends, selected);
                }
            }
        }
        if let Gesture::Connect { from, to } = &self.gesture {
            if let Some(a) = self.end_rect(*from) {
                let target = Rect { x: to.x - 1., y: to.y - 1., w: 2., h: 2. };
                let (style, ends) = self.link_look(*from, None);
                add_link(link_path(&a, &target, style), t.ink(self.ink), style, ends, false);
            }
        }
        // Rubber bands: the selection box or the area being drawn.
        let band = match &self.gesture {
            Gesture::Marquee { start, now, .. } | Gesture::MakeFrame { start, now } => {
                let (a, b) = (cam.to_screen(*start), cam.to_screen(*now));
                Some((a.x.min(b.x), a.y.min(b.y), (a.x - b.x).abs(), (a.y - b.y).abs()))
            }
            _ => None,
        };
        let guides: Vec<(bool, f32)> = self
            .guides
            .iter()
            .map(|(v, w)| (*v, if *v { cam.to_screen(v2(*w, 0.)).x } else { cam.to_screen(v2(0., *w)).y }))
            .collect();
        let guide_color = t.colors.brand;

        let link_width = (2. * cam.zoom).max(1.25);
        let dash = 6. * cam.zoom.max(0.6);
        let (dot, line) = (t.colors.canvas_dot, t.colors.canvas_line);
        let grid = t.size.dot_grid;
        let pattern = sheet.backdrop.pattern;
        let bounds_cell = self.viewport.clone();
        let underlay = canvas(
            move |bounds, _, _| bounds_cell.set(bounds),
            move |bounds, _, window, _| {
                paint_pattern(bounds, cam, grid, pattern, dot, line, window);
                let o = bounds.origin;
                for l in &curves {
                    let run: Vec<_> = l.run.iter().map(|p| *p + o).collect();
                    if l.selected {
                        // A soft band under a picked link.
                        let mut b = PathBuilder::stroke(px(link_width + 6.));
                        b.add_polygon(&run, false);
                        if let Ok(p) = b.build() {
                            window.paint_path(p, guide_color.opacity(0.18));
                        }
                    }
                    let mut b = PathBuilder::stroke(px(link_width));
                    if l.dashed {
                        b = b.dash_array(&[px(dash), px(dash)]);
                    }
                    b.add_polygon(&run, false);
                    if let Ok(p) = b.build() {
                        window.paint_path(p, l.color);
                    }
                    for h in &l.heads {
                        let mut b = PathBuilder::fill();
                        b.add_polygon(&h.iter().map(|p| *p + o).collect::<Vec<_>>(), true);
                        if let Ok(p) = b.build() {
                            window.paint_path(p, l.color);
                        }
                    }
                    if l.heads.is_empty() {
                        if let Some(end) = run.last() {
                            let r = px(3. * cam.zoom.max(0.7));
                            window.paint_quad(
                                gpui::fill(gpui::Bounds::new(point(end.x - r, end.y - r), gpui::size(r * 2., r * 2.)), l.color).corner_radii(r),
                            );
                        }
                    }
                }
                for ink in &paths {
                    paint_stroke(ink, o, window);
                }
            },
        )
        .absolute()
        .inset_0();

        let frames: Vec<_> = sheet
            .frames
            .iter()
            .filter(|f| f.rect.intersects(&view))
            .map(|f| {
                let a = cam.to_screen(v2(f.rect.x, f.rect.y));
                let z = cam.zoom;
                let c = t.accent(f.accent);
                let selected = self.is_selected(Selection::Frame(f.id));
                div()
                    .absolute()
                    .left(px(a.x))
                    .top(px(a.y))
                    .w(px(f.rect.w * z))
                    .h(px(f.rect.h * z))
                    .rounded(px(16. * z))
                    .bg(c.opacity(if t.is_dark { 0.10 } else { 0.08 }))
                    .border_1()
                    .border_color(c.opacity(if selected { 0.9 } else { 0.35 }))
                    .child(
                        div()
                            .absolute()
                            .left(px(14. * z))
                            .top(px(10. * z))
                            .flex()
                            .items_center()
                            .gap(px(6. * z))
                            .child(div().size(px(8. * z)).rounded(px(4. * z)).bg(c))
                            .child(
                                div()
                                    .text_size(px(14. * z))
                                    .font_weight(gpui::FontWeight::SEMIBOLD)
                                    .text_color(t.colors.text_muted)
                                    .child(if f.title.is_empty() { "Untitled area".to_string() } else { f.title.clone() }),
                            ),
                    )
                    .when(selected, |d| {
                        d.child(
                            div()
                                .absolute()
                                .right(px(4.))
                                .bottom(px(4.))
                                .size(t.size.grip)
                                .rounded(px(3.))
                                .border_r_2()
                                .border_b_2()
                                .border_color(c),
                        )
                    })
            })
            .collect();

        let (shape_layer, shape_words) = self.render_shapes(view, &t, window);

        let labels: Vec<_> = sheet
            .labels
            .iter()
            .filter(|l| Some(l.id) != self.label_edit && self.label_box(l).intersects(&view))
            .map(|l| self.render_label(l, cam, &t))
            .collect();

        let editing = self.editing.map(|e| e.id);
        let m = t.motion;
        // A note drawn grown or shrunk `scale` times about its centre and
        // moved down `dy` px, seen through `opacity`: arriving, it fades up
        // from 96 %; leaving, it fades and sinks a little as it shrinks.
        let moving = |this: &Self, s: &Sticker, scale: f32, dy: f32, opacity: f32, cx: &mut Context<Self>| {
            let r = this.note_rect(s);
            let c = v2(r.x + r.w / 2., r.y + r.h / 2.);
            let zoom = cam.zoom * scale;
            let pan = v2(cam.pan.x + c.x * (cam.zoom - zoom), cam.pan.y + c.y * (cam.zoom - zoom) + dy);
            div().absolute().top_0().left_0().opacity(opacity).child(this.render_sticker(s, Camera { pan, zoom }, &t, cx)).into_any_element()
        };
        let stickers: Vec<_> = sheet
            .stickers
            .iter()
            .filter(|s| Some(s.id) != editing && self.note_rect(s).intersects(&view))
            .map(|s| match self.exits.notes.arriving(s.id, &m) {
                Some(k) => moving(self, s, 0.96 + 0.04 * k, m.rise * 0.75 * (1. - k), k, cx),
                None => self.render_sticker(s, cam, &t, cx),
            })
            .collect();
        let gone: Vec<_> = self
            .exits
            .notes
            .leaving(&m)
            .into_iter()
            .filter(|(s, _)| self.note_rect(s).intersects(&view))
            .map(|(s, k)| moving(self, &s, 1. - 0.03 * k, m.sink * k, 1. - k, cx))
            .collect();

        // A picture behind the sheet: still while the sheet moves, softened
        // toward the paper so notes stay easy to read.
        let picture = sheet.backdrop.image.as_ref().map(|name| {
            let paper = t.tint(sheet.backdrop.tint);
            div()
                .absolute()
                .inset_0()
                .child(gpui::img(self.image_path(name)).id("backdrop").size_full().object_fit(gpui::ObjectFit::Cover))
                .when(sheet.backdrop.soften, |d| d.child(div().absolute().inset_0().bg(paper.opacity(0.55))))
        });

        let empty = sheet.stickers.is_empty()
            && sheet.strokes.is_empty()
            && sheet.labels.is_empty()
            && sheet.frames.is_empty()
            && sheet.shapes.is_empty();
        let shape_grip = self.shape_grip_at(self.cursor);
        let cursor = match self.tool {
            _ if self.space => gpui::CursorStyle::OpenHand,
            Tool::Hand => gpui::CursorStyle::OpenHand,
            Tool::Pen | Tool::Marker | Tool::Eraser | Tool::Connect | Tool::Frame => gpui::CursorStyle::Crosshair,
            Tool::Text if self.label_grip_at(self.cursor).is_some() || matches!(self.gesture, Gesture::LabelWidth { .. }) => {
                gpui::CursorStyle::ResizeLeftRight
            }
            Tool::Text => gpui::CursorStyle::IBeam,
            Tool::Select => match (&self.gesture, self.hovered) {
                (Gesture::Pan { .. } | Gesture::TurnShape { .. }, _) => gpui::CursorStyle::ClosedHand,
                (Gesture::ResizeShape { was, grip, .. }, _) => self.shape_grip_cursor(was.id, ShapeGrip::Corner(*grip)),
                (Gesture::None, _) if shape_grip.is_some() => shape_grip.map_or(gpui::CursorStyle::Arrow, |(id, g)| self.shape_grip_cursor(id, g)),
                (Gesture::Resize { grip, .. } | Gesture::ResizeFrame { grip, .. }, _) => resize_cursor(*grip),
                (Gesture::LabelWidth { .. }, _) => gpui::CursorStyle::ResizeLeftRight,
                (Gesture::None, _) if self.label_grip_at(self.cursor).is_some() => gpui::CursorStyle::ResizeLeftRight,
                (Gesture::None, _) if self.on_title(self.cursor) => gpui::CursorStyle::IBeam,
                (Gesture::None, _) if self.grip_at(self.cursor).is_some() || self.frame_grip_at(self.cursor).is_some() => {
                    let grip = self.grip_at(self.cursor).or(self.frame_grip_at(self.cursor)).map(|(_, g)| g).unwrap_or_default();
                    resize_cursor(grip)
                }
                (_, Some(_)) => gpui::CursorStyle::PointingHand,
                _ => gpui::CursorStyle::Arrow,
            },
        };

        let board = div()
            .id("sheet")
            .key_context("Board")
            .track_focus(&self.focus)
            .absolute()
            .inset_0()
            .overflow_hidden()
            .bg(t.tint(sheet.backdrop.tint))
            .cursor(cursor)
            .on_action(cx.listener(|this, _: &DeleteSelection, _, cx| this.delete_selection(cx)))
            .on_action(cx.listener(|this, _: &DuplicateSelection, _, cx| this.duplicate_selection(cx)))
            .on_action(cx.listener(|this, _: &SelectAll, _, cx| {
                let c = this.sheet();
                this.selection = c
                    .stickers
                    .iter()
                    .map(|s| Selection::Sticker(s.id))
                    .chain(c.labels.iter().map(|l| Selection::Label(l.id)))
                    .chain(c.shapes.iter().map(|s| Selection::Shape(s.id)))
                    .collect();
                cx.notify();
            }))
            .on_action(cx.listener(|this, _: &ToggleShapes, window, cx| this.toggle_shapes_panel(window, cx)))
            .on_action(cx.listener(|this, _: &TurnLeft, _, cx| this.turn_shapes(&this.selected_shapes(), -15., cx)))
            .on_action(cx.listener(|this, _: &TurnRight, _, cx| this.turn_shapes(&this.selected_shapes(), 15., cx)))
            .on_action(cx.listener(|this, _: &EditSelection, window, cx| match this.selection.first().copied() {
                Some(Selection::Sticker(id)) => this.begin_edit(id, false, window, cx),
                Some(Selection::Label(id)) => this.edit_label(id, false, window, cx),
                Some(Selection::Frame(id)) => this.rename_frame(id, window, cx),
                Some(Selection::Shape(id)) => this.edit_shape_text(id, window, cx),
                Some(Selection::Link(_)) => {}
                None => {}
            }))
            .on_action(cx.listener(|this, _: &Paste, _, cx| this.paste(false, cx)))
            .on_action(cx.listener(|this, _: &CopyNotes, window, cx| this.copy_selection(window, cx)))
            .on_action(cx.listener(|this, _: &CutNotes, _, cx| this.cut_selection(cx)))
            .on_action(cx.listener(|this, _: &PasteMarkdown, _, cx| this.paste(true, cx)))
            .on_action(cx.listener(|this, _: &ToMarkdown, _, cx| {
                let ids = this.selected_notes();
                if !ids.is_empty() {
                    this.markdown_notes(&ids, cx);
                }
            }))
            .on_action(cx.listener(|this, _: &CycleStyle, _, cx| this.cycle_style(cx)))
            .on_action(cx.listener(|this, _: &Escape, _, cx| {
                // An open menu or the drawer goes first.
                if this.menu.take().is_some() || std::mem::take(&mut this.drawer) || std::mem::take(&mut this.shapes_panel) {
                    cx.notify();
                    return;
                }
                this.selection.clear();
                this.gesture = Gesture::None;
                this.set_tool(Tool::Select, cx);
            }))
            .on_action(cx.listener(|this, _: &ZoomIn, _, cx| this.zoom_by(1.2, cx)))
            .on_action(cx.listener(|this, _: &ZoomOut, _, cx| this.zoom_by(1. / 1.2, cx)))
            .on_action(cx.listener(|this, _: &ZoomToFit, _, cx| this.zoom_to_fit(cx)))
            .on_action(cx.listener(|this, _: &Tidy, _, cx| this.tidy(cx)))
            .on_action(cx.listener(|this, _: &ToggleMinimap, _, cx| this.set_setting(cx, |s| s.minimap = !s.minimap)))
            .on_action(cx.listener(|this, _: &StepLeft, _, cx| this.step_selection(-1., 0., cx)))
            .on_action(cx.listener(|this, _: &StepRight, _, cx| this.step_selection(1., 0., cx)))
            .on_action(cx.listener(|this, _: &StepUp, _, cx| this.step_selection(0., -1., cx)))
            .on_action(cx.listener(|this, _: &StepDown, _, cx| this.step_selection(0., 1., cx)))
            .on_action(cx.listener(|this, _: &ToolSelect, _, cx| this.set_tool(Tool::Select, cx)))
            .on_action(cx.listener(|this, _: &ToolHand, _, cx| this.set_tool(Tool::Hand, cx)))
            .on_action(cx.listener(|this, _: &ToolPen, _, cx| this.set_tool(Tool::Pen, cx)))
            .on_action(cx.listener(|this, _: &ToolMarker, _, cx| this.set_tool(Tool::Marker, cx)))
            .on_action(cx.listener(|this, _: &ToolText, _, cx| this.set_tool(Tool::Text, cx)))
            .on_action(cx.listener(|this, _: &ToolConnect, _, cx| this.set_tool(Tool::Connect, cx)))
            .on_action(cx.listener(|this, _: &ToolFrame, _, cx| this.set_tool(Tool::Frame, cx)))
            .on_action(cx.listener(|this, _: &ToolEraser, _, cx| this.set_tool(Tool::Eraser, cx)))
            .on_key_down(cx.listener(|this, ev: &KeyDownEvent, _, cx| {
                if ev.keystroke.key == "space" && !this.space {
                    this.space = true;
                    cx.notify();
                }
            }))
            .on_key_up(cx.listener(|this, ev: &KeyUpEvent, _, cx| {
                if ev.keystroke.key == "space" {
                    this.space = false;
                    cx.notify();
                }
            }))
            .on_mouse_down(MouseButton::Left, cx.listener(Self::press))
            .on_mouse_down(MouseButton::Right, cx.listener(Self::press_right))
            .on_mouse_down(
                MouseButton::Middle,
                cx.listener(|this, ev: &MouseDownEvent, window, cx| {
                    window.focus(&this.focus, cx);
                    let at = this.local(ev.position);
                    this.gesture = Gesture::Pan { start: at, pan: this.sheet().camera.pan };
                    cx.notify();
                }),
            )
            .on_mouse_move(cx.listener(Self::drag))
            .on_mouse_up(MouseButton::Left, cx.listener(|this, _, _, cx| this.release(cx)))
            .on_mouse_up(MouseButton::Middle, cx.listener(|this, _, _, cx| this.release(cx)))
            .on_mouse_up_out(MouseButton::Left, cx.listener(|this, _, _, cx| this.release(cx)))
            .on_scroll_wheel(cx.listener(Self::wheel))
            .drag_over::<gpui::ExternalPaths>({
                let wash = t.colors.primary_tint;
                move |d, _, _, _| d.bg(wash)
            })
            .on_drop(cx.listener(|this, paths: &gpui::ExternalPaths, _, cx| {
                this.drop_paths(paths.paths().to_vec(), cx);
            }))
            // A shape dragged from the panel lands where it's let go.
            .drag_over::<ShapeDrag>({
                let wash = t.colors.brand.opacity(0.05);
                move |d, _, _, _| d.bg(wash)
            })
            .on_drop(cx.listener(|this, d: &ShapeDrag, window, cx| {
                let at = this.sheet().camera.to_world(this.local(window.mouse_position()));
                this.add_shape(d.0, Some(at), cx);
                window.focus(&this.focus, cx);
            }))
            .children(picture)
            .child(underlay)
            .children(frames)
            .child(shape_layer)
            .children(shape_words)
            // Written on the sheet: under the notes, moving with them.
            .child(self.render_sheet_head(&t))
            .children(labels)
            .children(stickers)
            .children(gone)
            .children(band.map(|(x, y, w, h)| {
                let making = matches!(self.gesture, Gesture::MakeFrame { .. });
                div()
                    .absolute()
                    .left(px(x))
                    .top(px(y))
                    .w(px(w))
                    .h(px(h))
                    .rounded(px(if making { 14. } else { 4. }))
                    .border_1()
                    .border_color(t.colors.brand)
                    .bg(t.colors.brand.opacity(0.06))
            }))
            .children(guides.into_iter().map(move |(v, at)| {
                let d = div().absolute().bg(guide_color.opacity(0.8));
                if v {
                    d.left(px(at)).top_0().bottom_0().w(px(1.))
                } else {
                    d.top(px(at)).left_0().right_0().h(px(1.))
                }
            }))
            .when(empty, |d| d.child(empty_state(&t)))
            .when(self.drop_tray, |d| d.child(self.render_tray(&t)));

        let edit_layer = self.render_edit_layer(&t, window, cx);
        let edit_bar_leaving = self.render_edit_bar_leaving(&t, cx);
        div()
            .relative()
            .flex_1()
            .min_h_0()
            .child(board)
            // Outside the sheet's key context, so typing a new name never
            // trips a one-letter tool key.
            .children(self.render_name_field(&t))
            .children(self.render_label_edit(&t, cx))
            .children(edit_layer)
            .children(edit_bar_leaving)
            .children(self.render_shapes_panel(cx))
            .child(self.render_toolbar(window, cx))
            .child(self.render_zoom(cx))
            .when(self.board.settings.minimap && !empty, |d| d.child(self.render_minimap(&t, cx)))
    }

    /// The sheet's name, blurb, note count and save state, pinned to the
    /// window's top-left (it stays put as the sheet pans and zooms; notes
    /// pass under nothing, they go over it). A click on the name renames it in
    /// place (`press`, `render_name_field`).
    fn render_sheet_head(&self, t: &Theme) -> impl IntoElement {
        let sheet = self.sheet();
        // Pinned to the window's top-left, whatever the sheet's pan and
        // zoom: the sheet's name is always where you look for it.
        let z = 1.;
        let at = v2(f32::from(t.space.xxl), f32::from(t.space.xl));
        let n = sheet.stickers.len();
        let meta = format!(
            "{}{} note{} · {}",
            if sheet.blurb.is_empty() { String::new() } else { format!("{} · ", sheet.blurb) },
            n,
            if n == 1 { "" } else { "s" },
            if self.saved { "Saved" } else { "Saving…" }
        );
        let icon_text = sheet.icon.clone();
        let measured = self.head_name.clone();
        let name = div()
            .relative()
            .text_size(t.text.heading * z)
            .font_weight(gpui::FontWeight::SEMIBOLD)
            .text_color(t.colors.text)
            // Typed into in place: the field sits where the name was.
            .when(self.naming, |d| d.invisible())
            .child(sheet.name.clone())
            .child(canvas(move |b, _, _| measured.set(Some(b)), |_, _, _, _| {}).absolute().inset_0());
        div()
            .absolute()
            .left(px(at.x))
            .top(px(at.y))
            .flex()
            .flex_col()
            .gap(t.space.xxs * z)
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(t.space.sm * z)
                    .when(!icon_text.is_empty(), |d| {
                        d.child(if is_monogram(&icon_text) {
                            mark(&icon_text, 34. * z, t).into_any_element()
                        } else {
                            div().text_size(t.text.heading * z).child(icon_text).into_any_element()
                        })
                    })
                    .child(name),
            )
            .child(div().whitespace_nowrap().text_size(t.text.caption * z).text_color(t.colors.text_muted).child(meta))
    }

    /// The sheet's name being typed into, over where it's written: same
    /// size, same weight, no box; only the caret says it's being edited.
    fn render_name_field(&self, t: &Theme) -> Option<impl IntoElement> {
        if !self.naming {
            return None;
        }
        let z = 1.;
        let b = self.head_name.get()?;
        let o = self.viewport.get().origin;
        Some(
            div()
                .id("sheet-name")
                .key_context("SheetName")
                .absolute()
                .left(b.origin.x - o.x - px(8. * z))
                .top(b.origin.y - o.y)
                .w(px(560. * z))
                .h(b.size.height)
                .flex()
                .items_center()
                .font_weight(gpui::FontWeight::SEMIBOLD)
                .text_color(t.colors.text)
                // The field's small text is 0.875rem: a rem that makes it
                // the heading's size.
                .child(rem_scaled(t.text.heading * z / 0.875, Input::new(&self.name_input).appearance(false).small())),
        )
    }

    /// `at` (sheet-local) is on the sheet's name, with no note or text
    /// over it.
    pub fn on_title(&self, at: Vec2) -> bool {
        let Some(b) = self.head_name.get() else { return false };
        let o = self.viewport.get().origin;
        let r = Rect {
            x: f32::from(b.origin.x - o.x),
            y: f32::from(b.origin.y - o.y),
            w: f32::from(b.size.width),
            h: f32::from(b.size.height),
        };
        let world = self.sheet().camera.to_world(at);
        r.contains(at) && self.note_at(world).is_none() && self.label_at(world).is_none()
    }

    /// The note being typed into, drawn over the sheet, with its edit bar
    /// above it: writing on the first row, the note's look on the second.
    /// The edit bar over the note being typed into: writing on the first
    /// row, the note's look on the second.
    fn edit_bar(&self, s: &Sticker, t: &Theme, cx: &mut Context<Self>) -> gpui::Div {
        let at = self.edit_bar_spot(s);
        let id = s.id;
        let tip = |text: String| move |window: &mut Window, cx: &mut gpui::App| super::tip(text.clone(), window, cx);
        let bar_btn = |name: &'static str, path: &'static str| {
            div()
                .id(name)
                .size(px(30.))
                .flex()
                .items_center()
                .justify_center()
                .rounded(t.radius.small)
                .cursor_pointer()
                .hover(|d| d.bg(t.colors.hover))
                .child(icon(path, t.size.icon_sm, t.colors.text_muted))
        };
        // A writing command: the button, its tooltip with the shortcut.
        let write = |name: &'static str, path: &'static str, what: &str, keys: &str, f: Format| {
            bar_btn(name, path)
                .tooltip(tip(format!("{what}  {}", shortcut(keys))))
                .on_click(cx.listener(move |this, _, window, cx| this.format_editing(f, window, cx)))
        };
        // The papers wait behind one swatch of the note's own paper, so the
        // bar stays narrow; they open on the side away from the note.
        let up = at.y < self.note_rect(s).y * self.sheet().camera.zoom + self.sheet().camera.pan.y;
        let papers_open = self.edit_papers;
        let paper_button = div()
            .id("e-paper")
            .relative()
            .size(px(30.))
            .flex()
            .items_center()
            .justify_center()
            .rounded(t.radius.small)
            .cursor_pointer()
            .when(papers_open, |d| d.bg(t.colors.hover))
            .hover(|d| d.bg(t.colors.hover))
            .tooltip(tip("Paper".into()))
            .child(div().size(px(16.)).rounded(t.radius.full).bg(t.paper(s.paper)).border_1().border_color(t.colors.text_subtle))
            .on_click(cx.listener(|this, _, _, cx| {
                this.edit_papers = !this.edit_papers;
                cx.notify();
            }))
            .when(papers_open, |d| {
                d.child(
                    div()
                        .id("e-papers")
                        .occlude()
                        .absolute()
                        .left(px(-8.))
                        .when(up, |p| p.bottom_full().mb(px(48.)))
                        .when(!up, |p| p.top_full().mt(px(12.)))
                        .p(t.space.sm)
                        .flex()
                        .gap(px(5.))
                        .rounded(t.radius.card)
                        .bg(t.colors.surface)
                        .border_1()
                        .border_color(t.colors.border)
                        .shadow(t.shadow_float())
                        .children(Paper::ALL.iter().map(|&p| {
                            div()
                                .id(("e-paper", p as usize))
                                .size(px(20.))
                                .rounded(t.radius.full)
                                .bg(t.paper(p))
                                .border_1()
                                .border_color(t.colors.border)
                                .hover(|d| d.border_color(t.colors.text_subtle))
                                .cursor_pointer()
                                .when(s.paper == p, |d| d.border_2().border_color(t.colors.brand))
                                .on_click(cx.listener(move |this, _, _, cx| {
                                    this.edit_papers = false;
                                    this.change_note(id, cx, |s| s.paper = p)
                                }))
                        })),
                )
            });
        // The note's type: the chosen one says its name.
        let styles = NoteStyle::ALL.iter().map(|&st| {
            let on = s.style == st;
            let path = match st {
                NoteStyle::Card => "icons/note-card.svg",
                NoteStyle::Taped => "icons/note-taped.svg",
                NoteStyle::Cheat => "icons/note-cheat.svg",
                NoteStyle::Index => "icons/note-index.svg",
            };
            let color = if on { t.colors.brand_on_tint } else { t.colors.text_muted };
            div()
                .id(("e-style", st as usize))
                .h(px(26.))
                .px(px(if on { 8. } else { 6. }))
                .flex()
                .items_center()
                .gap(px(5.))
                .rounded(t.radius.small)
                .text_size(t.text.caption)
                .cursor_pointer()
                .when(on, |d| d.bg(t.colors.brand_tint).text_color(color).font_weight(gpui::FontWeight::SEMIBOLD))
                .when(!on, |d| d.hover(|d| d.bg(t.colors.hover)))
                .child(icon(path, t.size.icon_sm, color))
                .when(on, |d| d.child(st.label()))
                .tooltip(tip(format!("{}  S on the sheet", st.label())))
                .on_click(cx.listener(move |this, _, _, cx| this.set_style(&[id], st, cx)))
        });
        let fonts = NoteFont::ALL.iter().map(|&f| {
            let on = s.font == f;
            div()
                .id(("e-font", f as usize))
                .h(px(26.))
                .px(px(7.))
                .flex()
                .items_center()
                .rounded(t.radius.small)
                .font_family(t.note_font(f))
                .text_size(t.text.caption)
                .cursor_pointer()
                .when(on, |d| d.bg(t.colors.brand_tint).text_color(t.colors.brand_on_tint))
                .when(!on, |d| d.text_color(t.colors.text_muted).hover(|d| d.bg(t.colors.hover)))
                .child("Aa")
                .on_click(cx.listener(move |this, _, _, cx| this.change_note(id, cx, |s| s.font = f)))
        });
        let sizes = TextSize::ALL.iter().map(|&zs| {
            let on = s.text_size == zs;
            div()
                .id(("e-size", zs as usize))
                .size(px(26.))
                .flex()
                .items_center()
                .justify_center()
                .rounded(t.radius.small)
                .text_size(t.text.caption)
                .cursor_pointer()
                .when(on, |d| d.bg(t.colors.brand_tint).text_color(t.colors.brand_on_tint))
                .when(!on, |d| d.text_color(t.colors.text_muted).hover(|d| d.bg(t.colors.hover)))
                .child(zs.label())
                .on_click(cx.listener(move |this, _, _, cx| this.change_note(id, cx, |s| s.text_size = zs)))
        });
        let divider = || div().w(px(1.)).h(px(20.)).mx(px(4.)).bg(t.colors.border);
        let row = || div().flex().items_center().gap(px(3.));
        let writing = row()
            .child(write("e-bold", "icons/bold.svg", "Bold", "B", Format::Bold))
            .child(write("e-italic", "icons/italic.svg", "Italic", "I", Format::Italic))
            .child(write("e-strike", "icons/strikethrough.svg", "Strikethrough", "Shift+X", Format::Strike))
            .child(write("e-code", "icons/code.svg", "Code", "E", Format::Code))
            .child(write("e-mark", "icons/highlighter.svg", "Highlight", "Shift+M", Format::Highlight))
            .child(divider())
            .child(write("e-bullets", "icons/list.svg", "Bullets", "Shift+B", Format::Bullets))
            .child(write("e-numbers", "icons/list-ordered.svg", "Numbers", "Shift+O", Format::Numbers))
            .child(write("e-task", "icons/list-checks.svg", "Tasks", "Shift+C", Format::Tasks))
            .child(divider())
            .child(
                bar_btn("e-table", "icons/table.svg")
                    .tooltip(tip("A table: | between cells".into()))
                    .on_click(cx.listener(|this, _, window, cx| this.format_editing(Format::Table, window, cx))),
            )
            .child(write("e-callout", "icons/lightbulb.svg", "Remember box", "Shift+R", Format::Callout))
            .child(divider())
            .child(
                bar_btn("e-markdown", "icons/wand-sparkles.svg")
                    .tooltip(tip(format!("Turn pasted text into Markdown  {}", shortcut("Shift+T"))))
                    .on_click(cx.listener(|this, _, window, cx| this.format_editing(Format::Markdown, window, cx))),
            )
            .child(div().flex_1())
            .child(bar_btn("e-pic", "icons/image.svg").tooltip(tip("Picture".into())).on_click(cx.listener(move |this, _, _, cx| this.pick_image(id, cx))))
            .child(bar_btn("e-bell", "icons/bell.svg").tooltip(tip("Remind me".into())).on_mouse_down(
                MouseButton::Left,
                cx.listener(move |this, ev: &MouseDownEvent, _, cx| {
                    cx.stop_propagation();
                    this.menu = Some(super::app::Menu { at: ev.position, kind: MenuKind::Note(id), page: MenuPage::Remind });
                    cx.notify();
                }),
            ))
            .child(bar_btn("e-more", "icons/ellipsis.svg").on_mouse_down(
                MouseButton::Left,
                cx.listener(move |this, ev: &MouseDownEvent, _, cx| {
                    cx.stop_propagation();
                    this.open_menu(ev.position, MenuKind::Note(id), cx);
                }),
            ));
        let look = row()
            .children(styles)
            .child(
                bar_btn("e-pin", "icons/pin.svg").tooltip(tip("Pin: push pins, stars, hearts, chips…".into())).on_mouse_down(
                    MouseButton::Left,
                    cx.listener(move |this, ev: &MouseDownEvent, _, cx| {
                        cx.stop_propagation();
                        this.menu = Some(super::app::Menu { at: ev.position, kind: MenuKind::Note(id), page: MenuPage::Pin });
                        cx.notify();
                    }),
                ),
            )
            .child(paper_button)
            .child(divider())
            .children(fonts)
            .child(divider())
            .children(sizes)
            .child(div().flex_1())
            .child(
                div()
                    .id("e-done")
                    .h(px(28.))
                    .ml(px(6.))
                    .px(px(10.))
                    .flex()
                    .items_center()
                    .gap(px(4.))
                    .rounded(t.radius.small)
                    .bg(t.colors.brand)
                    .text_color(t.colors.primary_fg)
                    .text_size(t.text.caption)
                    .font_weight(gpui::FontWeight::SEMIBOLD)
                    .cursor_pointer()
                    .child(icon("icons/check.svg", px(13.), t.colors.primary_fg))
                    .child("Done")
                    .on_click(cx.listener(|this, _, window, cx| {
                        this.finish_edit(cx);
                        window.focus(&this.focus, cx);
                    })),
            );
        let size = self.edit_bar_size.clone();
        div()
            .absolute()
            .left(px(at.x))
            .top(px(at.y))
            .flex()
            .flex_col()
            .gap(px(3.))
            .p(px(5.))
            .rounded(t.radius.card)
            .bg(t.colors.surface)
            .border_1()
            .border_color(t.colors.border)
            .shadow(t.shadow_float())
            .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
            .child(writing)
            .child(look)
            // Its size, for finding it a place next time.
            .child(
                canvas(
                    move |b, window, _| {
                        let now = (f32::from(b.size.width), f32::from(b.size.height));
                        if (now.0 - size.get().0).abs() > 1. || (now.1 - size.get().1).abs() > 1. {
                            size.set(now);
                            window.refresh();
                        }
                    },
                    |_, _, _, _| {},
                )
                .absolute()
                .inset_0(),
            )
    }

    /// Where the edit bar goes (sheet-local screen px): above or below the
    /// note, lined up with its left or right edge, inside the window, on
    /// as little of the other notes as it can, never on the note itself.
    fn edit_bar_spot(&self, s: &Sticker) -> Vec2 {
        let cam = self.sheet().camera;
        let r = self.note_rect(s);
        let z = cam.zoom.max(0.8);
        let at = cam.to_screen(v2(r.x, r.y));
        let note = Rect { x: at.x, y: at.y, w: r.w * z, h: r.h * z };
        let (bw, bh) = self.edit_bar_size.get();
        let (vw, vh) = self.viewport_size();
        let gap = 10.;
        // Clear of the dock at the bottom.
        let floor = vh - 96.;
        let place = |x: f32, y: f32| v2(x.clamp(8., (vw - bw - 8.).max(8.)), y.clamp(4., (floor - bh).max(4.)));
        let above = note.y - bh - gap;
        let below = note.y + note.h + gap;
        // Lined up with the note's left edge, its right edge, its middle,
        // then anywhere along the row above or below it.
        let lined = [note.x, note.x + note.w - bw, note.x + (note.w - bw) / 2.];
        let along = (0..).map(|i| 8. + i as f32 * 24.).take_while(|x| *x + bw <= vw - 8.);
        let spots: Vec<Vec2> = [above, below]
            .into_iter()
            .flat_map(|y| lined.into_iter().chain(along.clone()).map(move |x| (x, y)))
            .map(|(x, y)| place(x, y))
            .collect();
        let others: Vec<Rect> = self
            .sheet()
            .stickers
            .iter()
            .filter(|o| o.id != s.id)
            .map(|o| {
                let r = self.note_rect(o);
                let a = cam.to_screen(v2(r.x, r.y));
                Rect { x: a.x, y: a.y, w: r.w * cam.zoom, h: r.h * cam.zoom }
            })
            .collect();
        let covered = |p: &Vec2| {
            let bar = Rect { x: p.x, y: p.y, w: bw, h: bh };
            let on_note = overlap(&bar, &note);
            // Covering the note being typed into is worse than any other.
            on_note * 100. + others.iter().map(|o| overlap(&bar, o)).sum::<f32>()
        };
        // The first of the least covered: above on the left when all's clear.
        spots.into_iter().fold(None, |best: Option<(Vec2, f32)>, p| {
            let c = covered(&p);
            match best {
                Some((_, b)) if b <= c + 0.5 => best,
                _ => Some((p, c)),
            }
        })
        .map(|(p, _)| p)
        .unwrap_or(v2(note.x, above))
    }

    /// The edit bar after Done, sinking away over its note.
    fn render_edit_bar_leaving(&mut self, t: &Theme, cx: &mut Context<Self>) -> Option<gpui::AnyElement> {
        let (id, n) = self.exits.edit.leaving(t.motion.pop_out)?;
        let s = self.sheet().sticker(id)?.clone();
        Some(super::motion::leave(self.edit_bar(&s, t, cx), ("edit-bar-out", n), t.motion.pop_out, t.motion.sink))
    }

    fn render_edit_layer(&self, t: &Theme, _: &mut Window, cx: &mut Context<Self>) -> Option<gpui::AnyElement> {
        let ed = self.editing?;
        let s = self.sheet().sticker(ed.id)?;
        let cam = self.sheet().camera;
        let r = self.note_rect(s);
        let z = cam.zoom.max(0.8);
        let at = cam.to_screen(v2(r.x, r.y));
        let id = s.id;
        let k = size_factor(s.text_size);
        // The bar rises into place each time a note opens for typing.
        let bar = super::motion::rise_in(self.edit_bar(s, t, cx), ("edit-bar", id as usize), t);
        let (rt, rb) = super::sticker::radii(s.style);
        let (rt, rb) = (rt * z, rb * z);
        let picture = s.image.as_ref().map(|name| {
            gpui::img(self.image_path(name))
                .id(("edit-picture", s.id))
                .w_full()
                .h(px((s.rect.w * 0.62).min(s.rect.h) * z))
                .object_fit(gpui::ObjectFit::Cover)
                .rounded_t(px(rt))
        });
        use gpui::StyledImage;
        // The title and text are typed where they're drawn, the same size:
        // nothing jumps as the note opens for typing. The fields' own text
        // is 0.875 rem and they inset it by INSET, so the rem is set to give
        // the note's size and the inset is taken back.
        const INSET: f32 = 10.;
        // And the text field's own top padding.
        const INSET_TOP: f32 = 8.;
        let title_size = if s.style == NoteStyle::Cheat { 14. } else { 15. } * k * z;
        let title_line = if s.style == NoteStyle::Index { 24. } else { 20. } * k * z;
        let title = div()
            .flex_1()
            .h(px(title_line))
            .flex()
            .items_center()
            .ml(px(-INSET))
            .font_weight(gpui::FontWeight::SEMIBOLD)
            .text_color(t.colors.text)
            .child(rem_scaled(px(title_size / 0.875), Input::new(&self.title_input).appearance(false)));
        let text = div().ml(px(-INSET)).mt(px(-INSET_TOP)).child(rem_scaled(px(13. * k * z / 0.875), Textarea::new(&self.body_input).appearance(false)));
        // Laid out as the note's type lays out its title and text.
        let (title, text) = match s.style {
            NoteStyle::Cheat => (
                super::sticker::band(Some(title.into_any_element()), s.paper, z, t).rounded_t(px((rt - 2.).max(0.))),
                div().px(px(14. * z)).pt(px(10. * z)).pb(px(14. * z)).child(text),
            ),
            NoteStyle::Index => (
                div().pl(px(40. * z)).pr(px(16. * z)).pt(px(12. * z)).pb(px(4. * z)).border_b_1().border_color(t.colors.head_rule).child(title),
                div().pl(px(40. * z)).pr(px(14. * z)).pb(px(10. * z)).child(text),
            ),
            style => (
                div().px(px(16. * z)).pt(px(if style == NoteStyle::Taped { 18. } else { 14. } * z)).child(title),
                div().px(px(16. * z)).pt(px(8. * z)).pb(px(14. * z)).child(text),
            ),
        };
        let me = cx.entity();
        let w = r.w * z;
        // A new note fades up into place as it opens for typing.
        let arriving = self.exits.notes.arriving(id, &t.motion).unwrap_or(1.);
        let note = div()
            .id("editing")
            .key_context("NoteEdit")
            .absolute()
            .left(px(at.x))
            .top(px(at.y + t.motion.rise * 0.75 * (1. - arriving)))
            .opacity(arriving)
            .w(px(w))
            .min_h(px(r.h * z))
            .rounded_t(px(rt))
            .rounded_b(px(rb))
            .bg(t.paper(s.paper))
            .border_2()
            .border_color(t.colors.brand)
            .shadow(t.shadow_lifted(z))
            .font_family(t.note_font(s.font))
            .text_size(px(13. * k * z))
            .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
            .on_action(cx.listener(|this, _: &SaveNote, window, cx| {
                this.finish_edit(cx);
                window.focus(&this.focus, cx);
            }))
            .on_action(cx.listener(|this, _: &Dismiss, window, cx| {
                this.finish_edit(cx);
                window.focus(&this.focus, cx);
            }))
            .children(picture)
            .child(title)
            .child(text)
            .when(s.style == NoteStyle::Taped && s.pin.is_none(), |d| d.child(super::sticker::tape(id, w, z, t)))
            .children(s.pin.map(|p| super::pins::pin_view(p, w, z, self.exits.notes.pin_landing(id, &t.motion), t)));
        let note = on_format(note, move |f, window, cx| me.update(cx, |this, cx| this.format_key(f, window, cx)));
        Some(div().absolute().inset_0().child(note).child(bar).into_any_element())
    }

    fn render_minimap(&self, t: &Theme, cx: &mut Context<Self>) -> impl IntoElement {
        let (w, h) = (200., 130.);
        let sheet = self.sheet();
        let (vw, vh) = self.viewport_size();
        let view = sheet.camera.visible(vw, vh);
        let rects: Vec<(Rect, Hsla)> = sheet.stickers.iter().map(|s| (self.note_rect(s), t.paper(s.paper))).collect();
        let frames: Vec<(Rect, Hsla)> = sheet.frames.iter().map(|f| (f.rect, t.accent(f.accent))).collect();
        let shapes: Vec<(Rect, Hsla, Hsla)> = sheet
            .shapes
            .iter()
            .map(|s| (s.bounds(), s.fill.map_or(gpui::transparent_black(), |p| t.paper(p)), t.ink(s.ink)))
            .collect();
        let world = rects
            .iter()
            .map(|r| r.0)
            .chain(frames.iter().map(|f| f.0))
            .chain(shapes.iter().map(|s| s.0))
            .fold(view, |a, b| a.union(&b));
        let pad = 8.;
        let k = ((w - 2. * pad) / world.w).min((h - 2. * pad) / world.h);
        let ox = (w - world.w * k) / 2. - world.x * k;
        let oy = (h - world.h * k) / 2. - world.y * k;
        let edge = t.colors.border;
        let brand = t.colors.brand;
        let to_world = move |x: f32, y: f32| v2((x - ox) / k, (y - oy) / k);
        div()
            .id("minimap")
            .occlude()
            .absolute()
            .left(t.space.xl)
            .bottom(t.space.xl)
            .w(px(w))
            .h(px(h))
            .rounded(t.radius.card)
            .bg(t.colors.surface)
            .border_1()
            .border_color(t.colors.border)
            .shadow(t.shadow_float())
            .overflow_hidden()
            .cursor_pointer()
            .child(
                canvas(
                    |_, _, _| {},
                    move |b, _, window, _| {
                        let o = b.origin;
                        let quad = |r: &Rect| {
                            gpui::Bounds::new(
                                point(o.x + px(ox + r.x * k), o.y + px(oy + r.y * k)),
                                gpui::size(px((r.w * k).max(2.)), px((r.h * k).max(2.))),
                            )
                        };
                        for (r, c) in &frames {
                            window.paint_quad(gpui::fill(quad(r), c.opacity(0.12)).corner_radii(px(3.)));
                        }
                        for (r, fill, ink) in &shapes {
                            window.paint_quad(gpui::fill(quad(r), *fill).corner_radii(px(2.)).border_widths(px(0.5)).border_color(ink.opacity(0.6)));
                        }
                        for (r, c) in &rects {
                            window.paint_quad(gpui::fill(quad(r), *c).corner_radii(px(2.)).border_widths(px(0.5)).border_color(edge));
                        }
                        window.paint_quad(gpui::outline(quad(&view), brand, gpui::BorderStyle::Solid).corner_radii(px(3.)));
                    },
                )
                .absolute()
                .inset_0(),
            )
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(move |this, ev: &MouseDownEvent, _, cx| {
                    cx.stop_propagation();
                    this.minimap_jump(ev.position, to_world, cx);
                }),
            )
            .on_mouse_move(cx.listener(move |this, ev: &MouseMoveEvent, _, cx| {
                if ev.pressed_button == Some(MouseButton::Left) {
                    this.minimap_jump(ev.position, to_world, cx);
                }
            }))
    }

    /// Centres the view on the world point under the map position.
    fn minimap_jump(&mut self, p: gpui::Point<Pixels>, to_world: impl Fn(f32, f32) -> Vec2, cx: &mut Context<Self>) {
        let vb = self.viewport.get();
        let t = theme(cx);
        let left = f32::from(vb.origin.x + t.space.xl);
        let top = f32::from(vb.origin.y + vb.size.height - t.space.xl) - 130.;
        let w = to_world(f32::from(p.x) - left, f32::from(p.y) - top);
        let (vw, vh) = self.viewport_size();
        let cam = &mut self.board.sheet_mut().camera;
        cam.pan = v2(vw / 2. - w.x * cam.zoom, vh / 2. - w.y * cam.zoom);
        self.schedule_view_save(cx);
        cx.notify();
    }

    // ---------- Pointer ----------

    fn press_right(&mut self, ev: &MouseDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        window.focus(&self.focus, cx);
        self.finish_edit(cx);
        let world = self.sheet().camera.to_world(self.local(ev.position));
        let kind = if let Some(id) = self.note_at(world) {
            if !self.is_selected(Selection::Sticker(id)) {
                self.select(Selection::Sticker(id));
            }
            MenuKind::Note(id)
        } else if let Some(id) = self.label_at(world) {
            self.select(Selection::Label(id));
            MenuKind::Label(id)
        } else if let Some(id) = self.shape_at(world) {
            if !self.is_selected(Selection::Shape(id)) {
                self.select(Selection::Shape(id));
            }
            MenuKind::Shape(id)
        } else if let Some(id) = self.link_at(world) {
            self.select(Selection::Link(id));
            MenuKind::Link(id)
        } else if let Some(id) = self.frame_bar_at(world).or_else(|| self.sheet().frame_at(world)) {
            self.select(Selection::Frame(id));
            MenuKind::Frame(id)
        } else {
            MenuKind::Canvas(world)
        };
        self.open_menu(ev.position, kind, cx);
    }

    /// The area whose title bar is under `p`.
    fn frame_bar_at(&self, p: Vec2) -> Option<Id> {
        self.sheet()
            .frames
            .iter()
            .rev()
            .find(|f| p.x >= f.rect.x && p.x <= f.rect.x + f.rect.w && p.y >= f.rect.y && p.y <= f.rect.y + FRAME_BAR)
            .map(|f| f.id)
    }

    /// Everything a move of the selection carries, with where it started:
    /// the selected notes and labels, the selected areas and the notes in them.
    fn move_set(&self) -> Vec<(Selection, Vec2)> {
        let c = self.sheet();
        let mut out: Vec<(Selection, Vec2)> = Vec::new();
        let add = |s: Selection, at: Vec2, out: &mut Vec<(Selection, Vec2)>| {
            if !out.iter().any(|x| x.0 == s) {
                out.push((s, at));
            }
        };
        for sel in &self.selection {
            match *sel {
                Selection::Sticker(id) => {
                    if let Some(s) = c.sticker(id).filter(|s| !s.locked) {
                        add(*sel, v2(s.rect.x, s.rect.y), &mut out);
                    }
                }
                Selection::Label(id) => {
                    if let Some(l) = c.labels.iter().find(|l| l.id == id) {
                        add(*sel, l.at, &mut out);
                    }
                }
                Selection::Shape(id) => {
                    if let Some(s) = c.shapes.iter().find(|s| s.id == id && !s.locked) {
                        add(*sel, v2(s.rect.x, s.rect.y), &mut out);
                    }
                }
                // A link follows its ends.
                Selection::Link(_) => {}
                Selection::Frame(id) => {
                    if let Some(f) = c.frames.iter().find(|f| f.id == id) {
                        add(*sel, v2(f.rect.x, f.rect.y), &mut out);
                        for m in c.frame_members(id) {
                            if let Some(s) = c.sticker(m).filter(|s| !s.locked) {
                                add(Selection::Sticker(m), v2(s.rect.x, s.rect.y), &mut out);
                            }
                        }
                        for m in c.frame_shapes(id) {
                            if let Some(s) = c.shapes.iter().find(|s| s.id == m) {
                                add(Selection::Shape(m), v2(s.rect.x, s.rect.y), &mut out);
                            }
                        }
                    }
                }
            }
        }
        out
    }

    fn press(&mut self, ev: &MouseDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        window.focus(&self.focus, cx);
        self.menu = None;
        self.ink_palette = false;
        // Clicking away from text being written just finishes it.
        if self.label_edit.is_some() {
            self.finish_label(cx);
            return;
        }
        self.finish_edit(cx);
        let at = self.local(ev.position);
        self.cursor = at;
        let cam = self.sheet().camera;
        let world = cam.to_world(at);
        let double = ev.click_count >= 2;
        // Ctrl (Cmd on a Mac) or Shift and a click adds to the selection or
        // takes out of it, as in a file window; a box drawn with either
        // adds to it.
        let shift = ev.modifiers.shift || ev.modifiers.secondary();
        let pan = Gesture::Pan { start: at, pan: cam.pan };
        // A click on the sheet's name renames it, right there.
        if !self.space && matches!(self.tool, Tool::Select | Tool::Text) && self.on_title(at) {
            self.start_naming(window, cx);
            return;
        }
        self.before = Some(Before::open(&self.board));
        if self.space {
            self.gesture = pan;
            cx.notify();
            return;
        }
        match self.tool {
            Tool::Hand => self.gesture = pan,
            // A selected text's handle sets where it wraps, with the text
            // tool in hand too.
            Tool::Select | Tool::Text if self.label_grip_at(at).is_some() => {
                let id = self.label_grip_at(at).unwrap_or_default();
                let w = self.sheet().labels.iter().find(|l| l.id == id).map(|l| self.label_box(l).w).unwrap_or(LABEL_WRAP);
                self.gesture = Gesture::LabelWidth { id, start: world.x, w };
            }
            Tool::Select => {
                if let Some((id, g)) = self.shape_grip_at(at) {
                    self.gesture = match (g, self.sheet().shapes.iter().find(|s| s.id == id).cloned()) {
                        (ShapeGrip::Corner(grip), Some(was)) => Gesture::ResizeShape { start: world, was, grip },
                        _ => Gesture::TurnShape { id },
                    };
                } else if let Some((id, grip)) = self.grip_at(at) {
                    let rect = self.sheet().sticker(id).map(|s| self.note_rect(s)).unwrap();
                    self.select(Selection::Sticker(id));
                    self.gesture = Gesture::Resize { id, start: world, rect, grip };
                } else if let Some((id, grip)) = self.frame_grip_at(at) {
                    let rect = self.sheet().frames.iter().find(|f| f.id == id).map(|f| f.rect).unwrap();
                    self.gesture = Gesture::ResizeFrame { id, start: world, rect, grip };
                } else if !shift && !double && gpui::base::TextSelection::is_selectable_at(ev.position, window, cx) {
                    // The words of the one selected note: the press picks
                    // out text (Ctrl+C copies it); the note moves by its
                    // title, its edges or anywhere else on it.
                    self.gesture = Gesture::None;
                    cx.notify();
                    return;
                } else if let Some(id) = self.note_at(world) {
                    let sel = Selection::Sticker(id);
                    if double {
                        self.gesture = Gesture::None;
                        self.begin_edit(id, false, window, cx);
                        return;
                    }
                    if shift {
                        if !self.toggle_selected(sel) {
                            // Taken out: nothing to drag.
                            self.gesture = Gesture::None;
                            cx.notify();
                            return;
                        }
                    } else if !self.is_selected(sel) {
                        self.select(sel);
                    }
                    self.board.sheet_mut().bring_to_front(id);
                    self.gesture = Gesture::Move { items: self.move_set(), start: world, moved: false };
                } else if let Some(id) = self.label_at(world) {
                    let sel = Selection::Label(id);
                    if double {
                        self.gesture = Gesture::None;
                        self.edit_label(id, false, window, cx);
                        return;
                    }
                    if shift {
                        if !self.toggle_selected(sel) {
                            self.gesture = Gesture::None;
                            cx.notify();
                            return;
                        }
                    } else if !self.is_selected(sel) {
                        self.select(sel);
                    }
                    self.gesture = Gesture::Move { items: self.move_set(), start: world, moved: false };
                } else if let Some(id) = self.shape_at(world) {
                    let sel = Selection::Shape(id);
                    if double {
                        self.gesture = Gesture::None;
                        self.edit_shape_text(id, window, cx);
                        return;
                    }
                    if shift {
                        if !self.toggle_selected(sel) {
                            self.gesture = Gesture::None;
                            cx.notify();
                            return;
                        }
                    } else if !self.is_selected(sel) {
                        self.select(sel);
                    }
                    let shapes = &mut self.board.sheet_mut().shapes;
                    if let Some(i) = shapes.iter().position(|s| s.id == id) {
                        let s = shapes.remove(i);
                        shapes.push(s);
                    }
                    self.gesture = Gesture::Move { items: self.move_set(), start: world, moved: false };
                } else if let Some(id) = self.link_at(world) {
                    if shift {
                        self.toggle_selected(Selection::Link(id));
                    } else {
                        self.select(Selection::Link(id));
                    }
                    self.gesture = Gesture::None;
                    cx.notify();
                    return;
                } else if let Some(id) = self.frame_bar_at(world) {
                    let sel = Selection::Frame(id);
                    if double {
                        self.gesture = Gesture::None;
                        self.rename_frame(id, window, cx);
                        return;
                    }
                    if shift {
                        if !self.toggle_selected(sel) {
                            self.gesture = Gesture::None;
                            cx.notify();
                            return;
                        }
                    } else if !self.is_selected(sel) {
                        self.select(sel);
                    }
                    self.gesture = Gesture::Move { items: self.move_set(), start: world, moved: false };
                } else if double {
                    self.gesture = Gesture::None;
                    self.new_note_at(world, window, cx);
                    return;
                } else {
                    if !shift {
                        self.selection.clear();
                    }
                    self.gesture = Gesture::Marquee { start: world, now: world, additive: shift };
                }
            }
            Tool::Pen | Tool::Marker => {
                let marker = self.tool == Tool::Marker;
                let id = self.board.fresh_id();
                self.gesture = Gesture::Draw(Stroke { id, ink: self.ink, width: if marker { 16. } else { 2.5 }, marker, points: vec![world] });
            }
            Tool::Text => {
                self.gesture = Gesture::None;
                if let Some(id) = self.label_at(world) {
                    self.select(Selection::Label(id));
                    self.edit_label(id, false, window, cx);
                } else {
                    self.new_label_at(world, window, cx);
                }
                return;
            }
            Tool::Connect => match self.note_at(world).or_else(|| self.shape_at(world)) {
                Some(from) => self.gesture = Gesture::Connect { from, to: world },
                None => self.gesture = pan,
            },
            Tool::Frame => self.gesture = Gesture::MakeFrame { start: world, now: world },
            Tool::Eraser => {
                let erased = self.board.sheet_mut().erase_at(world, 6. / cam.zoom);
                self.gesture = Gesture::Erase { erased };
            }
        }
        cx.notify();
    }

    fn drag(&mut self, ev: &MouseMoveEvent, _: &mut Window, cx: &mut Context<Self>) {
        let at = self.local(ev.position);
        self.cursor = at;
        let cam = self.sheet().camera;
        let world = cam.to_world(at);
        // The button can come up outside the window; end the gesture then.
        if ev.pressed_button.is_none() && !matches!(self.gesture, Gesture::None) {
            self.release(cx);
            return;
        }
        let snap_on = self.board.settings.snap || self.board.settings.grid;
        match &mut self.gesture {
            Gesture::None => {
                let hovered = self.note_at(world);
                // The pointer changes to a resize arrow on an edge: draw
                // again whenever what's under it changes, edge included.
                let grip = self.grip_at(at).or_else(|| self.frame_grip_at(at)).map(|(_, g)| g);
                let label_grip = self.label_grip_at(at).is_some();
                let shape_grip = self.shape_grip_at(at).map(|(_, g)| g);
                let hover_shape = if hovered.is_none() && self.tool == Tool::Select { self.shape_at(world) } else { None };
                if hovered != self.hovered
                    || grip != self.hover_grip
                    || label_grip != self.hover_label_grip
                    || shape_grip != self.hover_shape_grip
                    || hover_shape != self.hover_shape
                    || shape_grip.is_some()
                {
                    self.hover_shape = hover_shape;
                    self.hovered = hovered;
                    self.hover_grip = grip;
                    self.hover_label_grip = label_grip;
                    self.hover_shape_grip = shape_grip;
                    cx.notify();
                }
                return;
            }
            Gesture::Pan { start, pan } => {
                let p = pan.add(at.sub(*start));
                self.board.sheet_mut().camera.pan = p;
            }
            Gesture::Move { items, start, moved } => {
                let mut d = world.sub(*start);
                if !*moved && d.len() * cam.zoom < 3. {
                    return;
                }
                *moved = true;
                let items = items.clone();
                // Snap the moving notes' box to other notes' edges.
                self.guides.clear();
                if snap_on {
                    let (dd, guides) = self.snap(&items, d, cam.zoom);
                    d = dd;
                    self.guides = guides;
                }
                let sheet = self.board.sheet_mut();
                for (sel, from) in &items {
                    let p = from.add(d);
                    match *sel {
                        Selection::Sticker(id) => {
                            if let Some(s) = sheet.sticker_mut(id) {
                                s.rect.x = p.x;
                                s.rect.y = p.y;
                            }
                        }
                        Selection::Label(id) => {
                            if let Some(l) = sheet.labels.iter_mut().find(|l| l.id == id) {
                                l.at = p;
                            }
                        }
                        Selection::Frame(id) => {
                            if let Some(f) = sheet.frames.iter_mut().find(|f| f.id == id) {
                                f.rect.x = p.x;
                                f.rect.y = p.y;
                            }
                        }
                        Selection::Shape(id) => {
                            if let Some(s) = sheet.shapes.iter_mut().find(|s| s.id == id) {
                                s.rect.x = p.x;
                                s.rect.y = p.y;
                            }
                        }
                        Selection::Link(_) => {}
                    }
                }
            }
            Gesture::Resize { id, start, rect, grip } => {
                let (id, grip, rect, start) = (*id, *grip, *rect, *start);
                // A note that grows with its text is never shorter than it.
                let grows = self.sheet().sticker(id).is_some_and(|s| !s.fixed);
                let text_h = if grows { self.heights.borrow().get(&id).copied().unwrap_or(0.) } else { 0. };
                let r = rect.resized(grip, world.x - start.x, world.y - start.y, MIN_STICKER.0, (MIN_STICKER.1 * 0.6).max(text_h));
                if let Some(s) = self.board.sheet_mut().sticker_mut(id) {
                    s.rect = r;
                }
            }
            Gesture::ResizeShape { start, was, grip } => {
                // The pull, in the upright box's terms; the opposite corner
                // stays where it is on the sheet.
                let d = rotate(world.sub(*start), v2(0., 0.), -was.rot);
                let min_h = if was.geom.is_line() { 8. } else { MIN_SHAPE };
                let mut r = was.rect.resized(*grip, d.x, d.y, MIN_SHAPE, min_h);
                let shift = rotate(r.center().sub(was.rect.center()), v2(0., 0.), was.rot);
                let c = was.rect.center().add(shift);
                r.x = c.x - r.w / 2.;
                r.y = c.y - r.h / 2.;
                let id = was.id;
                if let Some(s) = self.board.sheet_mut().shapes.iter_mut().find(|s| s.id == id) {
                    s.rect = r;
                }
            }
            Gesture::TurnShape { id } => {
                let id = *id;
                if let Some(s) = self.board.sheet_mut().shapes.iter_mut().find(|s| s.id == id) {
                    // Shift turns freely; otherwise it settles on every 15°.
                    s.rot = handle_angle(s.center(), world, if ev.modifiers.shift { 0. } else { 4. });
                }
            }
            Gesture::LabelWidth { id, start, w } => {
                let (id, w) = (*id, (*w + world.x - *start).clamp(LABEL_MIN_W, LABEL_MAX_W));
                if let Some(l) = self.board.sheet_mut().labels.iter_mut().find(|l| l.id == id) {
                    l.width = Some(w);
                }
            }
            Gesture::ResizeFrame { id, start, rect, grip } => {
                let (id, grip) = (*id, *grip);
                let r = rect.resized(grip, world.x - start.x, world.y - start.y, 160., 120.);
                if let Some(f) = self.board.sheet_mut().frames.iter_mut().find(|f| f.id == id) {
                    f.rect = r;
                }
            }
            Gesture::Marquee { start, now, additive } => {
                *now = world;
                let r = Rect { x: start.x.min(world.x), y: start.y.min(world.y), w: (start.x - world.x).abs(), h: (start.y - world.y).abs() };
                let additive = *additive;
                let c = self.sheet();
                let mut sel: Vec<Selection> = c
                    .stickers
                    .iter()
                    .filter(|s| r.intersects(&self.note_rect(s)))
                    .map(|s| Selection::Sticker(s.id))
                    .chain(c.labels.iter().filter(|l| r.intersects(&self.label_box(l))).map(|l| Selection::Label(l.id)))
                    .chain(c.frames.iter().filter(|f| contains(&r, &f.rect)).map(|f| Selection::Frame(f.id)))
                    .chain(c.shapes.iter().filter(|s| r.intersects(&s.bounds())).map(|s| Selection::Shape(s.id)))
                    .collect();
                if additive {
                    for s in std::mem::take(&mut self.selection) {
                        if !sel.contains(&s) {
                            sel.push(s);
                        }
                    }
                }
                self.selection = sel;
            }
            Gesture::MakeFrame { now, .. } => *now = world,
            Gesture::Draw(stroke) => {
                let last = *stroke.points.last().unwrap();
                if world.sub(last).len() * cam.zoom >= 1.0 {
                    stroke.points.push(world);
                }
            }
            Gesture::Erase { erased } => {
                if self.board.sheet_mut().erase_at(world, 6. / cam.zoom) {
                    *erased = true;
                }
            }
            Gesture::Connect { to, .. } => *to = world,
        }
        // Dragging notes to the left edge offers the other sheets to drop on.
        if let Gesture::Move { moved: true, items, .. } = &self.gesture {
            let notes = items.iter().any(|(s, _)| matches!(s, Selection::Sticker(_)));
            let near = notes && (at.x < TRAY_REACH || (self.drop_tray && at.x < TRAY_W + 20.));
            self.drop_tray = near;
            self.drop_sheet = None;
            if near {
                let row = ((at.y - TRAY_TOP - TRAY_HEAD) / TRAY_ROW).floor();
                let targets = self.tray_targets();
                if row >= 0. && (row as usize) < targets.len() && at.x < TRAY_W + 12. {
                    self.drop_sheet = Some(targets[row as usize]);
                }
            }
        }
        cx.notify();
    }

    /// The sheets a dragged note can be dropped on: every other one.
    fn tray_targets(&self) -> Vec<usize> {
        self.board.ordered().into_iter().filter(|&i| i != self.board.active).collect()
    }

    fn render_tray(&self, t: &Theme) -> impl IntoElement {
        let rows = self.tray_targets().into_iter().map(|ix| {
            let c = &self.board.categories[ix];
            let on = self.drop_sheet == Some(ix);
            div()
                .h(px(TRAY_ROW - 6.))
                .mb(px(6.))
                .px(t.space.sm)
                .flex()
                .items_center()
                .gap(t.space.sm)
                .rounded(t.radius.button)
                .border_1()
                .border_color(if on { t.colors.brand } else { t.colors.border })
                .bg(if on { t.colors.brand_tint } else { t.colors.surface })
                .child(sheet_badge(c, 24., t))
                .child(div().flex_1().min_w_0().truncate().text_size(t.text.body).child(c.name.clone()))
        });
        div()
            .absolute()
            .left(px(12.))
            .top(px(TRAY_TOP))
            .w(px(TRAY_W))
            .max_h(px(560.))
            .overflow_hidden()
            .p(t.space.sm)
            .rounded(t.radius.card)
            .bg(t.colors.surface)
            .border_1()
            .border_color(t.colors.border)
            .shadow(t.shadow_float())
            .child(
                div()
                    .h(px(TRAY_HEAD - 8.))
                    .flex()
                    .items_center()
                    .gap(t.space.xs)
                    .text_size(t.text.caption)
                    .text_color(t.colors.text_muted)
                    .child(icon("icons/folder-input.svg", t.size.icon_sm, t.colors.text_muted))
                    .child("Drop on a sheet to move"),
            )
            .children(rows)
    }

    /// The move `d` adjusted so the moving box lines up with another note's
    /// edge or centre when it comes within a few pixels, and the guides.
    fn snap(&self, items: &[(Selection, Vec2)], d: Vec2, zoom: f32) -> (Vec2, Vec<(bool, f32)>) {
        let c = self.sheet();
        let moving: Vec<Id> = items
            .iter()
            .filter_map(|(s, _)| match s {
                Selection::Sticker(id) => Some(*id),
                _ => None,
            })
            .collect();
        let boxes: Vec<Rect> = items
            .iter()
            .filter_map(|(s, from)| match s {
                Selection::Sticker(id) => c.sticker(*id).map(|n| {
                    let r = self.note_rect(n);
                    Rect { x: from.x + d.x, y: from.y + d.y, w: r.w, h: r.h }
                }),
                _ => None,
            })
            .collect();
        let Some(b) = boxes.into_iter().reduce(|a, b| a.union(&b)) else { return (d, Vec::new()) };
        let reach = if self.board.settings.snap { SNAP / zoom } else { -1. };
        let xs = [b.x, b.x + b.w / 2., b.x + b.w];
        let ys = [b.y, b.y + b.h / 2., b.y + b.h];
        let (mut best_x, mut best_y): (Option<(f32, f32)>, Option<(f32, f32)>) = (None, None);
        for o in c.stickers.iter().filter(|s| !moving.contains(&s.id)) {
            let r = self.note_rect(o);
            for tx in [r.x, r.x + r.w / 2., r.x + r.w] {
                for x in xs {
                    let off = tx - x;
                    if off.abs() <= reach && best_x.is_none_or(|(b, _)| off.abs() < b.abs()) {
                        best_x = Some((off, tx));
                    }
                }
            }
            for ty in [r.y, r.y + r.h / 2., r.y + r.h] {
                for y in ys {
                    let off = ty - y;
                    if off.abs() <= reach && best_y.is_none_or(|(b, _)| off.abs() < b.abs()) {
                        best_y = Some((off, ty));
                    }
                }
            }
        }
        let mut guides = Vec::new();
        let mut d = d;
        let grid = self.board.settings.grid.then_some(theme_grid());
        match (best_x, grid) {
            (Some((off, at)), _) => {
                d.x += off;
                guides.push((true, at));
            }
            // No neighbour to line up with: land on the dot grid.
            (None, Some(g)) => d.x += (b.x / g).round() * g - b.x,
            _ => {}
        }
        match (best_y, grid) {
            (Some((off, at)), _) => {
                d.y += off;
                guides.push((false, at));
            }
            (None, Some(g)) => d.y += (b.y / g).round() * g - b.y,
            _ => {}
        }
        (d, guides)
    }

    fn release(&mut self, cx: &mut Context<Self>) {
        let gesture = std::mem::replace(&mut self.gesture, Gesture::None);
        self.guides.clear();
        let mut before = self.before.take();
        match gesture {
            Gesture::None => {
                // The editor or a dialog took over this press; keep its snapshot.
                if self.editing.is_some() || self.dialog.is_some() {
                    self.before = before;
                }
                return;
            }
            Gesture::Pan { .. } => self.schedule_view_save(cx),
            Gesture::Move { items, moved: true, .. } => {
                let target = self.drop_sheet.take();
                self.drop_tray = false;
                let ids: Vec<Id> = items
                    .iter()
                    .filter_map(|(s, _)| match s {
                        Selection::Sticker(id) => Some(*id),
                        _ => None,
                    })
                    .collect();
                if let Some(to) = target {
                    // Back where they were, then over to the other sheet.
                    for (sel, from) in &items {
                        if let Selection::Sticker(id) = sel {
                            if let Some(s) = self.board.sheet_mut().sticker_mut(*id) {
                                s.rect.x = from.x;
                                s.rect.y = from.y;
                            }
                        }
                    }
                    // The other sheet changes too: keep it as it was first.
                    if let Some(b) = before.as_mut() {
                        b.keep(&self.board, to);
                    }
                    for id in &ids {
                        self.board.move_note(*id, to);
                    }
                    let name = self.board.categories[to].name.clone();
                    let n = ids.len();
                    self.selection.clear();
                    self.notify(format!("Moved {} to “{name}”", if n == 1 { "the note".to_string() } else { format!("{n} notes") }), cx);
                } else {
                    let t = Instant::now();
                    self.settling = ids.into_iter().map(|id| (id, t)).collect();
                }
            }
            Gesture::Marquee { start, now, .. } => {
                if start.sub(now).len() * self.sheet().camera.zoom < 3. {
                    // A click on the empty sheet.
                }
            }
            Gesture::MakeFrame { start, now } => {
                let r = Rect { x: start.x.min(now.x), y: start.y.min(now.y), w: (start.x - now.x).abs(), h: (start.y - now.y).abs() };
                if r.w > 60. && r.h > 60. {
                    self.before = before;
                    self.new_frame(r, cx);
                    self.tool = Tool::Select;
                    return;
                }
            }
            Gesture::Draw(mut stroke) => {
                stroke.points = simplify(&stroke.points, 0.35 / self.sheet().camera.zoom);
                if stroke.points.len() == 1 {
                    let p = stroke.points[0];
                    stroke.points.push(v2(p.x + 0.1, p.y));
                }
                self.board.sheet_mut().strokes.push(stroke);
            }
            Gesture::Connect { from, to } => {
                if let Some(to_id) = self.note_at(to).or_else(|| self.shape_at(to)).filter(|t| *t != from) {
                    let id = self.board.fresh_id();
                    let ink = self.ink;
                    let (style, ends) = self.link_look(from, Some(to_id));
                    let linked = self.board.sheet_mut().toggle_link_styled(id, from, to_id, ink, style, ends);
                    self.notify(if linked { "Linked" } else { "Link removed" }, cx);
                }
            }
            _ => {}
        }
        if let Some(mut b) = before {
            // A pan alone, or a click bringing a note to the front, records
            // nothing.
            b.settle(&self.board);
            self.commit_since(b, cx);
        }
        cx.notify();
    }

    /// A new area over `r`, named next.
    pub fn new_frame(&mut self, r: Rect, cx: &mut Context<Self>) {
        let before = self.before.take().unwrap_or_else(|| Before::open(&self.board));
        let id = self.board.fresh_id();
        let n = self.sheet().frames.len();
        let accent = Accent::ALL[(n + 1) % Accent::ALL.len()];
        self.board.sheet_mut().frames.insert(0, Frame { id, rect: r, title: String::new(), accent });
        self.commit_since(before, cx);
        self.select(Selection::Frame(id));
        self.dialog = Some(super::app::Dialog::RenameFrame(id));
        cx.notify();
    }

    fn wheel(&mut self, ev: &ScrollWheelEvent, _: &mut Window, cx: &mut Context<Self>) {
        let at = self.local(ev.position);
        let d = ev.delta.pixel_delta(px(20.));
        let (dx, dy) = (f32::from(d.x), f32::from(d.y));
        let cam = &mut self.board.sheet_mut().camera;
        if ev.modifiers.secondary() || ev.modifiers.control {
            cam.zoom_at(at, (dy * 0.0025).exp());
        } else if ev.modifiers.shift && dx == 0. {
            cam.pan.x += dy;
        } else {
            cam.pan.x += dx;
            cam.pan.y += dy;
        }
        self.schedule_view_save(cx);
        cx.notify();
    }

    /// The note whose edge or corner is under `at` (screen), and which.
    /// Over a note, only its own edges (or a selected note's) count; in the
    /// open, the edge of any note just beside it does, the top one first.
    pub(super) fn grip_at(&self, at: Vec2) -> Option<(Id, Grip)> {
        let cam = self.sheet().camera;
        let over = self.note_at(cam.to_world(at));
        let mut candidates: Vec<Id> = self.selected_notes();
        match over {
            Some(id) => candidates.push(id),
            None => candidates.extend(self.sheet().stickers.iter().rev().map(|s| s.id)),
        }
        candidates.into_iter().find_map(|id| {
            let s = self.sheet().sticker(id).filter(|s| !s.locked)?;
            let r = self.note_rect(s);
            let a = cam.to_screen(v2(r.x, r.y));
            let b = cam.to_screen(v2(r.x + r.w, r.y + r.h));
            Rect { x: a.x, y: a.y, w: b.x - a.x, h: b.y - a.y }.grip_at(at, GRIP_IN, GRIP_OUT, GRIP_CORNER).map(|g| (id, g))
        })
    }

    pub(super) fn frame_grip_at(&self, at: Vec2) -> Option<(Id, Grip)> {
        let cam = self.sheet().camera;
        self.sheet().frames.iter().filter(|f| self.is_selected(Selection::Frame(f.id))).find_map(|f| {
            let a = cam.to_screen(v2(f.rect.x, f.rect.y));
            let b = cam.to_screen(v2(f.rect.x + f.rect.w, f.rect.y + f.rect.h));
            Rect { x: a.x, y: a.y, w: b.x - a.x, h: b.y - a.y }.grip_at(at, GRIP_IN, GRIP_OUT, GRIP_CORNER).map(|g| (f.id, g))
        })
    }
}

/// The pointer for resizing from these edges.
fn resize_cursor(g: Grip) -> gpui::CursorStyle {
    use gpui::CursorStyle as C;
    match (g.left || g.right, g.top || g.bottom) {
        (true, true) if (g.left && g.top) || (g.right && g.bottom) => C::ResizeUpLeftDownRight,
        (true, true) => C::ResizeUpRightDownLeft,
        (true, false) => C::ResizeLeftRight,
        _ => C::ResizeUpDown,
    }
}

/// The dot grid's spacing, in world units.
fn theme_grid() -> f32 {
    crate::theme::Theme::light().size.dot_grid
}

fn contains(outer: &Rect, inner: &Rect) -> bool {
    inner.x >= outer.x && inner.y >= outer.y && inner.x + inner.w <= outer.x + outer.w && inner.y + inner.h <= outer.y + outer.h
}

/// The sheet's pattern, in world units so it moves and zooms with the
/// sheet: `dot` for dots, crosses and graph paper's heavier lines, `line`
/// for the rest.
pub(super) fn paint_pattern(bounds: gpui::Bounds<Pixels>, cam: Camera, grid: f32, pattern: Pattern, dot: Hsla, line: Hsla, window: &mut Window) {
    let (w, h) = (f32::from(bounds.size.width), f32::from(bounds.size.height));
    let o = bounds.origin;
    let mut hair = |r: &Rule, color: Hsla| {
        let b = if r.across {
            gpui::Bounds::new(point(o.x, o.y + px(r.at - 0.5)), gpui::size(bounds.size.width, px(1.)))
        } else {
            gpui::Bounds::new(point(o.x + px(r.at - 0.5), o.y), gpui::size(px(1.), bounds.size.height))
        };
        window.paint_quad(gpui::fill(b, color));
    };
    match pattern {
        Pattern::Plain => {}
        Pattern::Dots => paint_dots(bounds, cam, grid, dot, window),
        Pattern::Grid => {
            let step = pattern_step(grid, cam.zoom, 10.);
            for r in rules(cam, w, h, step, true, true, None) {
                hair(&r, line);
            }
        }
        Pattern::Graph => {
            let step = pattern_step(grid, cam.zoom, 8.);
            // Minor lines first, so the heavier ones cross over them.
            let all = rules(cam, w, h, step, true, true, Some(5));
            for r in all.iter().filter(|r| !r.major) {
                hair(r, line);
            }
            for r in all.iter().filter(|r| r.major) {
                hair(r, dot);
            }
        }
        Pattern::Lines => {
            let step = pattern_step(grid * 1.25, cam.zoom, 12.);
            for r in rules(cam, w, h, step, true, false, None) {
                hair(&r, line);
            }
        }
        Pattern::Crosses => {
            let step = pattern_step(grid, cam.zoom, 16.);
            let all = rules(cam, w, h, step, true, true, None);
            let arm = px(3.5);
            for y in all.iter().filter(|r| r.across) {
                for x in all.iter().filter(|r| !r.across) {
                    let c = o + point(px(x.at), px(y.at));
                    window.paint_quad(gpui::fill(gpui::Bounds::new(point(c.x - arm, c.y - px(0.5)), gpui::size(arm * 2., px(1.))), dot));
                    window.paint_quad(gpui::fill(gpui::Bounds::new(point(c.x - px(0.5), c.y - arm), gpui::size(px(1.), arm * 2.)), dot));
                }
            }
        }
        Pattern::Isometric => {
            // Rows of dots, every other row shifted half a step: a lattice of
            // equal triangles.
            let step = pattern_step(grid, cam.zoom, 14.);
            let row = step * 0.866;
            let r = px(1.1);
            let mut j = (-cam.pan.y / row).ceil() as i64;
            loop {
                let y = cam.pan.y + j as f32 * row;
                if y > h {
                    break;
                }
                let x0 = cam.pan.x + if j.rem_euclid(2) == 1 { step / 2. } else { 0. };
                let mut x = x0.rem_euclid(step);
                while x < w {
                    let p = o + point(px(x), px(y));
                    window.paint_quad(gpui::fill(gpui::Bounds::new(point(p.x - r, p.y - r), gpui::size(r * 2., r * 2.)), dot).corner_radii(r));
                    x += step;
                }
                j += 1;
            }
        }
    }
}

fn paint_dots(bounds: gpui::Bounds<Pixels>, cam: Camera, grid: f32, color: Hsla, window: &mut Window) {
    // Coarsen the grid as you zoom out so dots never crowd together.
    let mut step = grid * cam.zoom;
    while step < 14. {
        step *= 2.;
    }
    let (w, h) = (f32::from(bounds.size.width), f32::from(bounds.size.height));
    let x0 = cam.pan.x.rem_euclid(step);
    let y0 = cam.pan.y.rem_euclid(step);
    let r = px(1.1);
    let mut y = y0;
    while y < h {
        let mut x = x0;
        while x < w {
            let p = bounds.origin + point(px(x), px(y));
            window.paint_quad(gpui::fill(gpui::Bounds::new(point(p.x - r, p.y - r), gpui::size(r * 2., r * 2.)), color).corner_radii(r));
            x += step;
        }
        y += step;
    }
}

fn paint_stroke(ink: &InkPath, origin: gpui::Point<Pixels>, window: &mut Window) {
    let pts = &ink.points;
    if pts.is_empty() {
        return;
    }
    let mut b = PathBuilder::stroke(px(ink.width.max(0.75)));
    b.move_to(pts[0] + origin);
    if pts.len() == 2 {
        b.line_to(pts[1] + origin);
    } else {
        // Smooth through midpoints: each recorded point is a control point.
        for w in pts.windows(2).skip(1) {
            let mid = point((w[0].x + w[1].x) / 2., (w[0].y + w[1].y) / 2.);
            b.curve_to(mid + origin, w[0] + origin);
        }
        b.line_to(*pts.last().unwrap() + origin);
    }
    if let Ok(p) = b.build() {
        window.paint_path(p, ink.color);
    }
}

fn empty_state(t: &Theme) -> impl IntoElement {
    div()
        .absolute()
        .inset_0()
        .flex()
        .flex_col()
        .items_center()
        .justify_center()
        .gap(t.space.sm)
        .child(icon("icons/sticky-note.svg", t.size.icon_lg, t.colors.text_subtle))
        .child(div().text_size(t.text.title).text_color(t.colors.text_muted).child("A fresh sheet"))
        .child(
            div()
                .text_size(t.text.caption)
                .text_color(t.colors.text_subtle)
                .child("Double-click anywhere for a note, drop a picture, or right-click for more"),
        )
}

/// The area two screen rectangles share.
fn overlap(a: &Rect, b: &Rect) -> f32 {
    let w = (a.x + a.w).min(b.x + b.w) - a.x.max(b.x);
    let h = (a.y + a.h).min(b.y + b.h) - a.y.max(b.y);
    w.max(0.) * h.max(0.)
}
