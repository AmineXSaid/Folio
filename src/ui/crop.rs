//! A note's picture in its frame: the part kept by its crop covering the
//! frame, and its caption under it. Cropping happens in place: the whole
//! picture shows in the frame with the kept part outlined; drag inside it
//! to move it, a corner to resize; Done keeps it, Esc leaves it as it was.

use std::cell::Cell;
use std::rc::Rc;

use gpui::{canvas, div, img, prelude::*, px, AnyElement, Bounds, Context, MouseButton, MouseDownEvent, MouseMoveEvent, ObjectFit, Pixels, StyledImage};

use super::app::Folio;
use super::*;
use crate::model::*;
use crate::theme::Theme;

/// How near a corner of the kept part (px) a press takes that corner.
const CORNER: f32 = 14.;

/// What a press on the picture being cropped took hold of.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum CropGrip {
    Move,
    /// A corner: which side it's on across and down (0 left/top, 1 right/bottom).
    Corner(u8, u8),
}

/// A note's picture being cropped: the part kept so far, the press being
/// dragged (where it began, and the part then), and where the picture
/// lies on screen.
pub struct CropEdit {
    pub id: Id,
    pub crop: Crop,
    pub drag: Option<(CropGrip, gpui::Point<Pixels>, Crop)>,
    pub frame: Rc<Cell<Option<Bounds<Pixels>>>>,
}

/// The frames of a moving SVG drawing, about `width` pixels wide: one
/// round of its movement at 15 a second (at most 120), each drawn by resvg.
/// None for a still drawing or one that won't draw.
pub fn svg_frames(path: &std::path::Path, width: f32) -> Option<gpui::RenderImage> {
    const FPS: f32 = 15.;
    let svg = std::fs::read_to_string(path).ok()?;
    if !crate::svg_anim::is_animated(&svg) {
        return None;
    }
    let period = crate::svg_anim::period(&svg).max(0.2);
    let n = ((period * FPS).ceil() as usize).clamp(2, 120);
    let step = period / n as f32;
    let opt = resvg::usvg::Options { resources_dir: path.parent().map(|p| p.to_path_buf()), ..Default::default() };
    let mut frames = Vec::with_capacity(n);
    for i in 0..n {
        let tree = resvg::usvg::Tree::from_str(&crate::svg_anim::at(&svg, i as f32 * step), &opt).ok()?;
        let size = tree.size();
        let k = (width / size.width()).min(1200. / size.height().max(1.));
        let (w, h) = ((size.width() * k).ceil().max(1.) as u32, (size.height() * k).ceil().max(1.) as u32);
        let mut pixmap = resvg::tiny_skia::Pixmap::new(w, h)?;
        resvg::render(&tree, resvg::tiny_skia::Transform::from_scale(k, k), &mut pixmap.as_mut());
        // Straight (not premultiplied) colour, blue first, as GPUI draws it.
        let mut bgra = Vec::with_capacity((w * h * 4) as usize);
        for p in pixmap.pixels() {
            let c = p.demultiply();
            bgra.extend_from_slice(&[c.blue(), c.green(), c.red(), c.alpha()]);
        }
        let buffer = image::RgbaImage::from_raw(w, h, bgra)?;
        frames.push(image::Frame::from_parts(buffer, 0, 0, image::Delay::from_numer_denom_ms((step * 1000.).round() as u32, 1)));
    }
    Some(gpui::RenderImage::new(frames))
}

impl Folio {
    /// A moving SVG drawing's frames, once made (they're made away from the
    /// screen, the first time it's shown); None while they're being made or
    /// for a still drawing.
    fn svg_player(&self, name: &str, cx: &mut Context<Self>) -> Option<std::sync::Arc<gpui::RenderImage>> {
        if !name.to_ascii_lowercase().ends_with(".svg") {
            return None;
        }
        if let Some(made) = self.svg_players.borrow().get(name) {
            return made.clone();
        }
        self.svg_players.borrow_mut().insert(name.to_string(), None);
        let path = self.image_path(name);
        let key = name.to_string();
        let work = cx.background_spawn(async move { svg_frames(&path, 480.) });
        cx.spawn(async move |this, cx| {
            if let Some(frames) = work.await {
                let _ = this.update(cx, |this, cx| {
                    this.svg_players.borrow_mut().insert(key, Some(std::sync::Arc::new(frames)));
                    cx.notify();
                });
            }
        })
        .detach();
        None
    }

    /// A picture's own size, read once and kept.
    pub fn picture_size(&self, name: &str) -> Option<(f32, f32)> {
        let path = self.image_path(name);
        *self.picture_sizes.borrow_mut().entry(name.to_string()).or_insert_with(|| crate::store::picture_size(&path))
    }

    /// Note `id`'s picture, `w` by `h` (px), rounded `top` and `bottom`,
    /// cropped as it's kept, with its caption under it; or, while it's
    /// being cropped, the whole picture with the part kept outlined.
    pub(super) fn picture_frame(&self, s: &Sticker, w: f32, h: f32, top: f32, bottom: f32, z: f32, t: &Theme, cx: &mut Context<Self>) -> AnyElement {
        let Some(name) = s.image.as_ref() else { return div().into_any_element() };
        let path = self.image_path(name);
        let size = self.picture_size(name);
        let id = s.id;
        let editing = self.crop_edit.as_ref().filter(|c| c.id == id);
        let frame = div().relative().w(px(w)).h(px(h)).flex_none().overflow_hidden().rounded_t(px(top)).rounded_b(px(bottom));
        let picture = match (editing, size) {
            // The whole picture, fitted in, with the part kept outlined.
            (Some(edit), Some((nw, nh))) => {
                let k = (w / nw).min(h / nh);
                let (pw, ph) = (nw * k, nh * k);
                let (ox, oy) = ((w - pw) / 2., (h - ph) / 2.);
                let c = edit.crop;
                let keep = (ox + c.x * pw, oy + c.y * ph, c.w * pw, c.h * ph);
                let at = edit.frame.clone();
                let shade = t.colors.scrim;
                let dim = |x: f32, y: f32, dw: f32, dh: f32| div().absolute().left(px(x)).top(px(y)).w(px(dw.max(0.))).h(px(dh.max(0.))).bg(shade);
                let knob = |x: f32, y: f32| {
                    div()
                        .absolute()
                        .left(px(x - 5.))
                        .top(px(y - 5.))
                        .size(px(10.))
                        .rounded(t.radius.small)
                        .bg(t.colors.surface)
                        .border_1()
                        .border_color(t.colors.brand)
                };
                let (kx, ky, kw, kh) = keep;
                frame
                    .bg(t.colors.surface_sunk)
                    .child(img(path).id(("crop-picture", id)).absolute().left(px(ox)).top(px(oy)).w(px(pw)).h(px(ph)))
                    // Dimmed round the part kept.
                    .child(dim(0., 0., w, ky))
                    .child(dim(0., ky + kh, w, h - ky - kh))
                    .child(dim(0., ky, kx, kh))
                    .child(dim(kx + kw, ky, w - kx - kw, kh))
                    .child(div().absolute().left(px(kx)).top(px(ky)).w(px(kw)).h(px(kh)).border_2().border_color(t.colors.brand))
                    .child(knob(kx, ky))
                    .child(knob(kx + kw, ky))
                    .child(knob(kx, ky + kh))
                    .child(knob(kx + kw, ky + kh))
                    .child(canvas(move |b, _, _| at.set(Some(b)), |_, _, _, _| {}).absolute().inset_0())
                    .child(
                        div()
                            .id(("crop-grab", id))
                            .absolute()
                            .inset_0()
                            .cursor_move()
                            .on_mouse_down(MouseButton::Left, cx.listener(move |this, ev: &MouseDownEvent, _, cx| {
                                cx.stop_propagation();
                                this.crop_press(ev.position, (ox, oy, pw, ph), cx);
                            }))
                            .on_mouse_move(cx.listener(move |this, ev: &MouseMoveEvent, _, cx| this.crop_drag(ev.position, (pw, ph), cx)))
                            .on_mouse_up(MouseButton::Left, cx.listener(|this, _, _, cx| {
                                if let Some(e) = this.crop_edit.as_mut() {
                                    e.drag = None;
                                }
                                cx.notify();
                            })),
                    )
                    .child(self.crop_bar(t, cx))
                    .into_any_element()
            }
            (_, Some(natural)) if s.crop.is_some_and(|c| !c.is_all()) => {
                let r = crop_place(natural, (w, h), s.crop.unwrap_or(Crop::ALL));
                frame.child(img(path).id(("picture", id)).absolute().left(px(r.x)).top(px(r.y)).w(px(r.w)).h(px(r.h))).into_any_element()
            }
            // A moving SVG plays its frames; the id gives the picture frame
            // state, so they (and an animated GIF's) play.
            _ => match self.svg_player(name, cx) {
                Some(frames) => frame.child(img(frames).id(("picture", id)).size_full().object_fit(ObjectFit::Cover)).into_any_element(),
                None => frame.child(img(path).id(("picture", id)).size_full().object_fit(ObjectFit::Cover)).into_any_element(),
            },
        };
        if s.caption.trim().is_empty() {
            return picture;
        }
        div()
            .flex()
            .flex_col()
            .flex_none()
            .child(picture)
            .child(
                div()
                    .px(px(12. * z))
                    .pt(px(6. * z))
                    .text_size(px(11.5 * z))
                    .line_height(px(15. * z))
                    .italic()
                    .text_color(t.colors.text_muted)
                    .child(s.caption.clone()),
            )
            .into_any_element()
    }

    /// The bar on a picture being cropped: back to all of it, and done.
    fn crop_bar(&self, t: &Theme, cx: &mut Context<Self>) -> gpui::Div {
        let chip = |id: &'static str| {
            div()
                .id(id)
                .h(px(26.))
                .px(px(10.))
                .flex()
                .items_center()
                .gap(px(4.))
                .rounded(t.radius.button)
                .text_size(t.text.caption)
                .font_weight(gpui::FontWeight::SEMIBOLD)
                .cursor_pointer()
                .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
        };
        div()
            .absolute()
            .bottom(px(8.))
            .right(px(8.))
            .flex()
            .gap(px(6.))
            .child(
                chip("crop-all")
                    .bg(t.colors.surface)
                    .border_1()
                    .border_color(t.colors.border)
                    .text_color(t.colors.text_body)
                    .hover(|d| d.bg(t.colors.hover))
                    .child("Whole picture")
                    .on_click(cx.listener(|this, _, _, cx| {
                        if let Some(e) = this.crop_edit.as_mut() {
                            e.crop = Crop::ALL;
                        }
                        cx.notify();
                    })),
            )
            .child(
                primary_face(chip("crop-done"), t)
                    .bg(t.colors.surface)
                    .child(icon("icons/check.svg", px(13.), t.colors.accent_ink))
                    .child("Done")
                    .on_click(cx.listener(|this, _, _, cx| this.finish_crop(true, cx))),
            )
    }

    /// Crops note `id`'s picture in place.
    pub fn start_crop(&mut self, id: Id, cx: &mut Context<Self>) {
        let Some(crop) = self.board.find_note(id).filter(|s| s.image.is_some()).map(|s| s.crop.unwrap_or(Crop::ALL)) else { return };
        self.menu = None;
        self.finish_edit(cx);
        self.crop_edit = Some(CropEdit { id, crop, drag: None, frame: Rc::new(Cell::new(None)) });
        cx.notify();
    }

    /// Ends cropping: the part kept (one undo step), or as it was.
    pub fn finish_crop(&mut self, keep: bool, cx: &mut Context<Self>) {
        let Some(e) = self.crop_edit.take() else { return };
        if keep {
            let crop = (!e.crop.is_all()).then_some(e.crop.clamped());
            if self.board.find_note(e.id).is_some_and(|s| s.crop != crop) {
                self.change_note(e.id, cx, |s| s.crop = crop);
            }
        }
        cx.notify();
    }

    /// A press on the picture being cropped: a corner of the kept part to
    /// resize it, inside it to move it.
    fn crop_press(&mut self, at: gpui::Point<Pixels>, picture: (f32, f32, f32, f32), cx: &mut Context<Self>) {
        let Some(e) = self.crop_edit.as_mut() else { return };
        let Some(b) = e.frame.get() else { return };
        let (ox, oy, pw, ph) = picture;
        let (x, y) = (f32::from(at.x - b.origin.x), f32::from(at.y - b.origin.y));
        let c = e.crop;
        let (l, tp, r, bt) = (ox + c.x * pw, oy + c.y * ph, ox + (c.x + c.w) * pw, oy + (c.y + c.h) * ph);
        let near = |a: f32, b: f32| (a - b).abs() <= CORNER;
        let corner = [(l, tp, 0, 0), (r, tp, 1, 0), (l, bt, 0, 1), (r, bt, 1, 1)].into_iter().find(|(cx_, cy_, _, _)| near(x, *cx_) && near(y, *cy_));
        let grip = match corner {
            Some((_, _, i, j)) => Some(CropGrip::Corner(i, j)),
            None if x >= l && x <= r && y >= tp && y <= bt => Some(CropGrip::Move),
            None => None,
        };
        e.drag = grip.map(|g| (g, at, c));
        cx.notify();
    }

    /// The press being dragged across the picture being cropped.
    fn crop_drag(&mut self, at: gpui::Point<Pixels>, picture: (f32, f32), cx: &mut Context<Self>) {
        let Some(e) = self.crop_edit.as_mut() else { return };
        let Some((grip, from, was)) = e.drag else { return };
        let (pw, ph) = picture;
        let (dx, dy) = (f32::from(at.x - from.x) / pw.max(1.), f32::from(at.y - from.y) / ph.max(1.));
        e.crop = crop_moved(was, grip, dx, dy);
        cx.notify();
    }
}

/// The kept part `was`, after a drag by `dx`, `dy` (fractions of the
/// picture) on `grip`.
pub fn crop_moved(was: Crop, grip: CropGrip, dx: f32, dy: f32) -> Crop {
    match grip {
        CropGrip::Move => Crop { x: (was.x + dx).clamp(0., 1. - was.w), y: (was.y + dy).clamp(0., 1. - was.h), ..was },
        CropGrip::Corner(i, j) => {
            let (mut l, mut tp, mut r, mut b) = (was.x, was.y, was.x + was.w, was.y + was.h);
            if i == 0 {
                l = (l + dx).clamp(0., r - Crop::LEAST);
            } else {
                r = (r + dx).clamp(l + Crop::LEAST, 1.);
            }
            if j == 0 {
                tp = (tp + dy).clamp(0., b - Crop::LEAST);
            } else {
                b = (b + dy).clamp(tp + Crop::LEAST, 1.);
            }
            Crop { x: l, y: tp, w: r - l, h: b - tp }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dragging_moves_or_resizes_the_kept_part() {
        let c = Crop { x: 0.2, y: 0.2, w: 0.5, h: 0.5 };
        let m = crop_moved(c, CropGrip::Move, 0.5, -0.5);
        assert_eq!((m.x, m.y, m.w, m.h), (0.5, 0., 0.5, 0.5));
        let r = crop_moved(c, CropGrip::Corner(1, 1), 0.1, 0.1);
        assert!((r.w - 0.6).abs() < 1e-5 && (r.h - 0.6).abs() < 1e-5 && r.x == 0.2);
        let tiny = crop_moved(c, CropGrip::Corner(0, 0), 0.9, 0.9);
        assert!((tiny.w - Crop::LEAST).abs() < 1e-5 && (tiny.h - Crop::LEAST).abs() < 1e-5);
    }
}
