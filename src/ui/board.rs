//! The sheet: an endless dotted canvas you pan and zoom, with ink and links
//! painted beneath the stickers and labels laid on top.
//!
//! One handler set on the sheet does all hit testing against the model
//! (front-most first), so a press means the same thing whatever element the
//! pointer happens to be over. Only what's in view is laid out.

use gpui::{
    canvas, div, point, prelude::*, px, Context, Hsla, MouseButton, MouseDownEvent,
    MouseMoveEvent, PathBuilder, Pixels, ScrollWheelEvent, Window,
};

use super::app::{Folio, Gesture, Selection, Tool};
use super::*;
use crate::model::*;
use crate::theme::theme;

/// How close, in screen pixels, a press must be to a sticker's corner to
/// resize it.
const GRIP_REACH: f32 = 16.;

/// A stroke ready to paint: screen points, colour, width.
struct InkPath {
    points: Vec<gpui::Point<Pixels>>,
    color: Hsla,
    width: f32,
}

impl Folio {
    pub(super) fn render_board(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let t = theme(cx).clone();
        let sheet = self.sheet();
        let cam = sheet.camera;
        let (vw, vh) = self.viewport_size();
        // A margin, so things slide in rather than pop in at the edges.
        let mut view = cam.visible(vw, vh);
        let m = 200. / cam.zoom;
        view = Rect { x: view.x - m, y: view.y - m, w: view.w + 2. * m, h: view.h + 2. * m };

        // Ink beneath the stickers: links, then strokes.
        let mut paths: Vec<InkPath> = Vec::new();
        let screen = |p: Vec2| {
            let s = cam.to_screen(p);
            point(px(s.x), px(s.y))
        };
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
        let mut curves: Vec<([gpui::Point<Pixels>; 4], Hsla)> = Vec::new();
        for l in &sheet.links {
            if let (Some(a), Some(b)) = (sheet.sticker(l.from), sheet.sticker(l.to)) {
                if a.rect.union(&b.rect).intersects(&view) {
                    let c = link_curve(&a.rect, &b.rect);
                    curves.push((c.map(screen), t.ink(l.ink).opacity(0.75)));
                }
            }
        }
        if let Gesture::Connect { from, to } = &self.gesture {
            if let Some(a) = sheet.sticker(*from) {
                let target = Rect { x: to.x - 1., y: to.y - 1., w: 2., h: 2. };
                let c = link_curve(&a.rect, &target);
                curves.push((c.map(screen), t.ink(self.ink)));
            }
        }
        let link_width = (2. * cam.zoom).max(1.25);
        let dash = 6. * cam.zoom.max(0.6);
        let dot = t.colors.canvas_dot;
        let grid = t.size.dot_grid;
        let bounds_cell = self.viewport.clone();
        let underlay = canvas(
            move |bounds, _, _| bounds_cell.set(bounds),
            move |bounds, _, window, _| {
                paint_dots(bounds, cam, grid, dot, window);
                let o = bounds.origin;
                for (c, color) in &curves {
                    let mut b = PathBuilder::stroke(px(link_width)).dash_array(&[px(dash), px(dash)]);
                    b.move_to(c[0] + o);
                    b.cubic_bezier_to(c[3] + o, c[1] + o, c[2] + o);
                    if let Ok(p) = b.build() {
                        window.paint_path(p, *color);
                    }
                    // A small dot where the link lands.
                    let r = px(3. * cam.zoom.max(0.7));
                    let end = c[3] + o;
                    window.paint_quad(gpui::fill(gpui::Bounds::new(point(end.x - r, end.y - r), gpui::size(r * 2., r * 2.)), *color).corner_radii(r));
                }
                for ink in &paths {
                    paint_stroke(ink, o, window);
                }
            },
        )
        .absolute()
        .inset_0();

        let search = self.search(cx);
        let labels: Vec<_> = sheet
            .labels
            .iter()
            .filter(|l| label_rect(l).intersects(&view))
            .map(|l| {
                let at = cam.to_screen(l.at);
                let selected = self.selected == Some(Selection::Label(l.id));
                div()
                    .absolute()
                    .left(px(at.x))
                    .top(px(at.y))
                    .px(px(4. * cam.zoom))
                    .rounded(t.radius.small)
                    .text_size(px(15. * cam.zoom))
                    .italic()
                    .text_color(t.ink(l.ink))
                    .when(selected, |d| d.border_1().border_color(t.colors.selection))
                    .whitespace_nowrap()
                    .children(l.text.lines().map(|line| div().child(line.to_string())))
            })
            .collect();

        let stickers: Vec<_> = sheet
            .stickers
            .iter()
            .filter(|s| s.rect.intersects(&view))
            .map(|s| {
                let dim = !search.is_empty()
                    && !s.title.to_lowercase().contains(&search)
                    && !s.body.to_lowercase().contains(&search);
                self.render_sticker(s, cam, dim, &t, cx)
            })
            .collect();

        let empty = sheet.stickers.is_empty() && sheet.strokes.is_empty() && sheet.labels.is_empty();
        let cursor = match self.tool {
            Tool::Hand => gpui::CursorStyle::OpenHand,
            Tool::Pen | Tool::Marker | Tool::Eraser | Tool::Connect => gpui::CursorStyle::Crosshair,
            Tool::Text => gpui::CursorStyle::IBeam,
            Tool::Select => match (&self.gesture, self.hovered) {
                (Gesture::Pan { .. }, _) => gpui::CursorStyle::ClosedHand,
                (Gesture::Resize { .. }, _) => gpui::CursorStyle::ResizeUpLeftDownRight,
                (_, Some(_)) => gpui::CursorStyle::PointingHand,
                _ => gpui::CursorStyle::Arrow,
            },
        };

        div()
            .id("sheet")
            .key_context("Board")
            .track_focus(&self.focus)
            .relative()
            .flex_1()
            .min_h_0()
            .overflow_hidden()
            .bg(t.colors.canvas)
            .cursor(cursor)
            .on_action(cx.listener(|this, _: &DeleteSelection, _, cx| this.delete_selection(cx)))
            .on_action(cx.listener(|this, _: &DuplicateSelection, _, cx| this.duplicate_selection(cx)))
            .on_action(cx.listener(|this, _: &EditSelection, window, cx| {
                if let Some(sel) = this.selected {
                    this.before = Some(this.board.clone());
                    this.open_editor(sel, false, window, cx);
                }
            }))
            .on_action(cx.listener(|this, _: &Paste, _, cx| this.paste(cx)))
            .on_action(cx.listener(|this, _: &Escape, _, cx| {
                this.selected = None;
                this.gesture = Gesture::None;
                this.set_tool(Tool::Select, cx);
            }))
            .on_action(cx.listener(|this, _: &ZoomIn, _, cx| this.zoom_by(1.2, cx)))
            .on_action(cx.listener(|this, _: &ZoomOut, _, cx| this.zoom_by(1. / 1.2, cx)))
            .on_action(cx.listener(|this, _: &ZoomToFit, _, cx| this.zoom_to_fit(cx)))
            .on_action(cx.listener(|this, _: &ToolSelect, _, cx| this.set_tool(Tool::Select, cx)))
            .on_action(cx.listener(|this, _: &ToolHand, _, cx| this.set_tool(Tool::Hand, cx)))
            .on_action(cx.listener(|this, _: &ToolPen, _, cx| this.set_tool(Tool::Pen, cx)))
            .on_action(cx.listener(|this, _: &ToolMarker, _, cx| this.set_tool(Tool::Marker, cx)))
            .on_action(cx.listener(|this, _: &ToolText, _, cx| this.set_tool(Tool::Text, cx)))
            .on_action(cx.listener(|this, _: &ToolConnect, _, cx| this.set_tool(Tool::Connect, cx)))
            .on_action(cx.listener(|this, _: &ToolEraser, _, cx| this.set_tool(Tool::Eraser, cx)))
            .on_mouse_down(MouseButton::Left, cx.listener(Self::press))
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
            .child(underlay)
            .child(self.render_sheet_head(&t, cx))
            .children(labels)
            .children(stickers)
            .when(empty, |d| d.child(empty_state(&t)))
            .child(self.render_toolbar(window, cx))
            .child(self.render_zoom(cx))
    }

    fn render_sheet_head(&self, t: &crate::theme::Theme, _cx: &mut Context<Self>) -> impl IntoElement {
        let sheet = self.sheet();
        let n = sheet.stickers.len();
        let meta = format!(
            "{}{} note{} · {}",
            if sheet.blurb.is_empty() { String::new() } else { format!("{} · ", sheet.blurb) },
            n,
            if n == 1 { "" } else { "s" },
            if self.saved { "Saved" } else { "Saving…" }
        );
        div()
            .absolute()
            .top(t.space.xl)
            .left(t.space.xxl)
            .flex()
            .flex_col()
            .gap(t.space.xxs)
            .child(
                div()
                    .text_size(t.text.heading)
                    .font_weight(gpui::FontWeight::SEMIBOLD)
                    .text_color(t.colors.text)
                    .child(sheet.name.clone()),
            )
            .child(div().text_size(t.text.caption).text_color(t.colors.text_muted).child(meta))
    }

    // ---------- Pointer ----------

    fn press(&mut self, ev: &MouseDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        window.focus(&self.focus, cx);
        let at = self.local(ev.position);
        self.cursor = at;
        let cam = self.sheet().camera;
        let world = cam.to_world(at);
        let double = ev.click_count >= 2;
        let pan = Gesture::Pan { start: at, pan: cam.pan };
        self.before = Some(self.board.clone());
        match self.tool {
            Tool::Hand => self.gesture = pan,
            Tool::Select => {
                if let Some(id) = self.grip_at(at) {
                    let r = self.sheet().sticker(id).map(|s| s.rect).unwrap();
                    self.selected = Some(Selection::Sticker(id));
                    self.gesture = Gesture::Resize { id, start: world, size: (r.w, r.h) };
                } else if let Some(id) = self.sheet().sticker_at(world) {
                    self.selected = Some(Selection::Sticker(id));
                    if double {
                        self.gesture = Gesture::None;
                        self.open_editor(Selection::Sticker(id), false, window, cx);
                    } else {
                        let r = self.sheet().sticker(id).map(|s| s.rect).unwrap();
                        self.board.sheet_mut().bring_to_front(id);
                        self.gesture = Gesture::Move { id, grab: world.sub(v2(r.x, r.y)), moved: false };
                    }
                } else if let Some(id) = self.sheet().label_at(world) {
                    self.selected = Some(Selection::Label(id));
                    if double {
                        self.gesture = Gesture::None;
                        self.open_editor(Selection::Label(id), false, window, cx);
                    } else {
                        let l = self.sheet().labels.iter().find(|l| l.id == id).map(|l| l.at).unwrap();
                        self.gesture = Gesture::MoveLabel { id, grab: world.sub(l), moved: false };
                    }
                } else if double {
                    self.gesture = Gesture::None;
                    self.new_sticker_at(world, window, cx);
                } else {
                    self.selected = None;
                    self.gesture = pan;
                }
            }
            Tool::Pen | Tool::Marker => {
                let marker = self.tool == Tool::Marker;
                let id = self.board.fresh_id();
                self.gesture = Gesture::Draw(Stroke {
                    id,
                    ink: self.ink,
                    width: if marker { 16. } else { 2.5 },
                    marker,
                    points: vec![world],
                });
            }
            Tool::Text => {
                self.gesture = Gesture::None;
                if let Some(id) = self.sheet().label_at(world) {
                    self.selected = Some(Selection::Label(id));
                    self.open_editor(Selection::Label(id), false, window, cx);
                } else {
                    let id = self.board.fresh_id();
                    let ink = self.ink;
                    self.board.sheet_mut().labels.push(Label { id, at: v2(world.x, world.y - 11.), text: String::new(), ink });
                    self.selected = Some(Selection::Label(id));
                    self.open_editor(Selection::Label(id), true, window, cx);
                }
            }
            Tool::Connect => match self.sheet().sticker_at(world) {
                Some(from) => self.gesture = Gesture::Connect { from, to: world },
                None => self.gesture = pan,
            },
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
        match &mut self.gesture {
            Gesture::None => {
                let hovered = self.sheet().sticker_at(world);
                if hovered != self.hovered {
                    self.hovered = hovered;
                    cx.notify();
                }
                return;
            }
            Gesture::Pan { start, pan } => {
                let d = at.sub(*start);
                let p = pan.add(d);
                self.board.sheet_mut().camera.pan = p;
            }
            Gesture::Move { id, grab, moved } => {
                let (id, grab) = (*id, *grab);
                *moved = true;
                if let Some(s) = self.board.sheet_mut().sticker_mut(id) {
                    s.rect.x = world.x - grab.x;
                    s.rect.y = world.y - grab.y;
                }
            }
            Gesture::MoveLabel { id, grab, moved } => {
                let (id, grab) = (*id, *grab);
                *moved = true;
                if let Some(l) = self.board.sheet_mut().labels.iter_mut().find(|l| l.id == id) {
                    l.at = world.sub(grab);
                }
            }
            Gesture::Resize { id, start, size } => {
                let (id, start, size) = (*id, *start, *size);
                if let Some(s) = self.board.sheet_mut().sticker_mut(id) {
                    s.rect.w = (size.0 + world.x - start.x).max(MIN_STICKER.0);
                    s.rect.h = (size.1 + world.y - start.y).max(MIN_STICKER.1);
                }
            }
            Gesture::Draw(stroke) => {
                // Skip points closer than a pixel on screen to the last one.
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
        cx.notify();
    }

    fn release(&mut self, cx: &mut Context<Self>) {
        let gesture = std::mem::replace(&mut self.gesture, Gesture::None);
        let before = self.before.take();
        match gesture {
            Gesture::None => {
                // The editor took over this press; keep its snapshot.
                if self.editing.is_some() {
                    self.before = before;
                }
                return;
            }
            Gesture::Pan { .. } => self.schedule_save(cx),
            Gesture::Move { moved: false, .. } | Gesture::MoveLabel { moved: false, .. } => {}
            Gesture::Draw(mut stroke) => {
                stroke.points = simplify(&stroke.points, 0.35 / self.sheet().camera.zoom);
                if stroke.points.len() == 1 {
                    // A tap leaves a dot.
                    let p = stroke.points[0];
                    stroke.points.push(v2(p.x + 0.1, p.y));
                }
                self.board.sheet_mut().strokes.push(stroke);
            }
            Gesture::Connect { from, to } => {
                let target = self.sheet().sticker_at(to);
                match target {
                    Some(to_id) if to_id != from => {
                        let id = self.board.fresh_id();
                        let ink = self.ink;
                        let linked = self.board.sheet_mut().toggle_link(id, from, to_id, ink);
                        self.notify(if linked { "Linked" } else { "Link removed" }, cx);
                    }
                    _ => {}
                }
            }
            Gesture::Erase { erased: false } => {}
            _ => {}
        }
        if let Some(before) = before {
            // Commit compares content, so a pan alone records nothing.
            let mut b = before;
            b.active = self.board.active;
            for (c, now) in b.categories.iter_mut().zip(&self.board.categories) {
                if c.id == now.id {
                    c.camera = now.camera;
                }
            }
            self.commit(b, cx);
        }
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
        self.schedule_save(cx);
        cx.notify();
    }

    /// The sticker whose resize corner is under `at` (screen): the selected
    /// or hovered one, so corners don't catch presses on stickers at rest.
    fn grip_at(&self, at: Vec2) -> Option<Id> {
        let cam = self.sheet().camera;
        let candidates = [
            match self.selected {
                Some(Selection::Sticker(id)) => Some(id),
                _ => None,
            },
            self.hovered,
        ];
        candidates.into_iter().flatten().find(|id| {
            self.sheet().sticker(*id).is_some_and(|s| {
                let corner = cam.to_screen(v2(s.rect.x + s.rect.w, s.rect.y + s.rect.h));
                (corner.x - at.x).abs() <= GRIP_REACH && (corner.y - at.y).abs() <= GRIP_REACH
            })
        })
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

fn empty_state(t: &crate::theme::Theme) -> impl IntoElement {
    div()
        .absolute()
        .inset_0()
        .flex()
        .flex_col()
        .items_center()
        .justify_center()
        .gap(t.space.sm)
        .child(icon("icons/sticky-note.svg", t.size.icon_lg, t.colors.text_subtle))
        .child(div().text_size(t.text.title).text_color(t.colors.text_muted).child("This sheet is empty"))
        .child(
            div()
                .text_size(t.text.caption)
                .text_color(t.colors.text_subtle)
                .child("Double-click anywhere for a note, drop a picture, or pick up the pen"),
        )
}
