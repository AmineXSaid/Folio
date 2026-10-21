//! The opening, like a studio's logo before a film: the pink tile springs
//! in, the note drops onto it and settles, the pin pops, the highlight
//! wipes across, the lines write themselves, the corner peels, and
//! "folio." rises letter by letter. The full version plays the first time;
//! after that a short one (or none, from Settings). A click or a key skips.

use std::time::{Duration, Instant};

use gpui::{canvas, div, point, prelude::*, px, rgb, rgba, Bounds, Context, Hsla, MouseButton, PathBuilder, Pixels, Point, Window};

use super::app::{Folio, Intro};

const PINK: u32 = 0xe5487a;
const CREAM: u32 = 0xf1ebdd;
const PEEL: u32 = 0xddd5c4;
const INK: u32 = 0x111111;

/// When the full opening starts to fade out, and when it's gone.
const FULL_OUT: f32 = 3.4;
const SHORT_OUT: f32 = 0.75;
const FADE: f32 = 0.6;

impl Intro {
    fn elapsed(&self) -> f32 {
        self.start.elapsed().as_secs_f32()
    }
    fn out_at(&self) -> f32 {
        if self.full { FULL_OUT } else { SHORT_OUT }
    }
    /// Jumps to the fade-out.
    pub fn skip(&mut self) {
        let out = self.out_at();
        if self.elapsed() < out {
            self.start = Instant::now() - Duration::from_secs_f32(out);
        }
    }
}

fn clamp01(x: f32) -> f32 {
    x.clamp(0., 1.)
}
fn span(t: f32, a: f32, b: f32) -> f32 {
    clamp01((t - a) / (b - a))
}
fn ease_out(x: f32) -> f32 {
    1. - (1. - x).powi(3)
}
/// Overshoots a little, then settles.
fn back_out(x: f32) -> f32 {
    let c = 1.9;
    1. + (c + 1.) * (x - 1.).powi(3) + c * (x - 1.).powi(2)
}

/// Where each part of the logo is at time `t` (seconds).
#[derive(Clone, Copy)]
struct Pose {
    tile: f32,
    drop: f32,
    pin: f32,
    wipe: f32,
    lines: [f32; 3],
    peel: f32,
    zoom: f32,
}

fn pose(t: f32, full: bool) -> Pose {
    if !full {
        let k = back_out(span(t, 0.0, 0.42));
        return Pose { tile: k, drop: k, pin: k, wipe: 1., lines: [1.; 3], peel: 1., zoom: 1. + 0.25 * span(t, SHORT_OUT, SHORT_OUT + FADE) };
    }
    Pose {
        tile: back_out(span(t, 0.15, 0.8)),
        drop: span(t, 0.7, 1.45),
        pin: back_out(span(t, 1.35, 1.75)),
        wipe: ease_out(span(t, 1.55, 1.95)),
        lines: [span(t, 1.6, 1.95), span(t, 1.78, 2.12), span(t, 1.92, 2.25)],
        peel: back_out(span(t, 1.9, 2.35)),
        zoom: 1. + 0.25 * ease_out(span(t, FULL_OUT, FULL_OUT + FADE)),
    }
}

/// The note's fall: from above and tilted, overshooting, then settling.
fn fall(k: f32) -> (f32, f32) {
    if k <= 0. {
        return (-260., -30.);
    }
    if k < 0.6 {
        let e = ease_out(k / 0.6);
        (-260. + 270. * e, -30. + 32. * e)
    } else if k < 0.8 {
        let e = (k - 0.6) / 0.2;
        (10. - 14. * e, 2. - 3. * e)
    } else {
        let e = (k - 0.8) / 0.2;
        (-4. + 4. * e, -1. + e)
    }
}

impl Folio {
    pub(super) fn render_intro(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let Some(intro) = self.intro.as_ref() else { return div().into_any_element() };
        let t = intro.elapsed();
        let full = intro.full;
        let out = intro.out_at();
        if t > out + FADE {
            self.intro = None;
            window.refresh();
            return div().into_any_element();
        }
        window.request_animation_frame();
        let p = pose(t, full);
        let fade = 1. - span(t, out, out + FADE);
        let size = 180.;

        let logo = canvas(|_, _, _| {}, move |b, _, window, _| paint_logo(b, p, size, window)).w(px(size * 1.4)).h(px(size * 1.4));

        let word_t = if full { t - 1.95 } else { 10. };
        let letters = "folio.".chars().enumerate().map(|(i, ch)| {
            let k = ease_out(span(word_t, i as f32 * 0.07, i as f32 * 0.07 + 0.55));
            let k = if i == 5 { back_out(span(word_t, 0.45, 0.85)) } else { k };
            div()
                .relative()
                .top(px(26. * (1. - k)))
                .opacity(clamp01(k))
                .text_color(if i == 5 { rgb(PINK) } else { rgb(0xf4f1ea) })
                .child(ch.to_string())
        });
        let tag = if full { ease_out(span(t, 2.35, 2.9)) } else { 1. };
        let glow = if full { ease_out(span(t, 0.3, 2.0)) } else { 0.7 };
        let pink: Hsla = rgb(PINK).into();

        div()
            .id("intro")
            .absolute()
            .inset_0()
            .bg(rgb(0x0d0f0e))
            .opacity(fade)
            .flex()
            .flex_col()
            .items_center()
            .justify_center()
            .gap(px(18.))
            .cursor_pointer()
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|this, _, _, cx| {
                    if let Some(i) = this.intro.as_mut() {
                        i.skip();
                    }
                    cx.notify();
                }),
            )
            .child(
                // A soft pink glow behind the logo.
                div()
                    .absolute()
                    .size(px(260.))
                    .rounded(px(130.))
                    .shadow(vec![gpui::BoxShadow {
                        color: pink.opacity(0.32 * glow),
                        offset: point(px(0.), px(0.)),
                        blur_radius: px(170.),
                        spread_radius: px(40.),
                        inset: false,
                    }]),
            )
            .child(logo)
            .child(
                div()
                    .flex()
                    .text_size(px(58. * p.zoom))
                    .font_weight(gpui::FontWeight::BOLD)
                    .line_height(px(64.))
                    .children(letters),
            )
            .child(
                div()
                    .text_size(px(15.))
                    .text_color(rgba(0xa5ada8ff))
                    .opacity(tag)
                    .child("Sticky notes on endless sheets"),
            )
            .child(
                div()
                    .absolute()
                    .right(px(22.))
                    .bottom(px(18.))
                    .text_size(px(12.))
                    .text_color(rgb(0x7d8682))
                    .child("Click or press any key to skip"),
            )
            .into_any_element()
    }
}

/// Paints the logo, `size` across at rest, centred in `b`.
fn paint_logo(b: Bounds<Pixels>, p: Pose, size: f32, window: &mut Window) {
    let c = b.center();
    let k = size / 512. * p.zoom;
    let to_screen = |x: f32, y: f32| point(c.x + px((x - 256.) * k), c.y + px((y - 250.) * k));

    // The tile: a rounded square, scaled from its centre.
    if p.tile > 0. {
        let s = p.tile.max(0.);
        let half = 242. * k * s;
        let centre = to_screen(256., 250.);
        let bounds = Bounds::new(point(centre.x - px(half), centre.y - px(half)), gpui::size(px(half * 2.), px(half * 2.)));
        window.paint_quad(gpui::fill(bounds, rgba(PINK << 8 | (255. * clamp01(p.tile * 1.4)) as u32)).corner_radii(px(112. * k * s)));
    }
    if p.drop <= 0. {
        return;
    }
    // The note, in its own space (the SVG's): turned -6°, plus the fall.
    let (dy, extra) = fall(p.drop);
    let angle = (-6. + extra).to_radians();
    let (sin, cos) = angle.sin_cos();
    let note = |x: f32, y: f32| -> Point<Pixels> {
        let (rx, ry) = (x - 256., y - 256.);
        let (qx, qy) = (rx * cos - ry * sin + 256., rx * sin + ry * cos + 256. + dy);
        to_screen((qx - 256.) * 1.2 + 256., (qy - 256.) * 1.2 + 262.)
    };
    let stroke = |w: f32| px(w * 1.2 * k);
    let poly = |pts: &[(f32, f32)], fill: u32, line: f32, window: &mut Window| {
        let mut f = PathBuilder::fill();
        let mut s = PathBuilder::stroke(stroke(line));
        for (i, (x, y)) in pts.iter().enumerate() {
            let q = note(*x, *y);
            if i == 0 {
                f.move_to(q);
                s.move_to(q);
            } else {
                f.line_to(q);
                s.line_to(q);
            }
        }
        f.close();
        s.close();
        if let Ok(path) = f.build() {
            window.paint_path(path, rgb(fill));
        }
        if let Ok(path) = s.build() {
            window.paint_path(path, rgb(INK));
        }
    };
    poly(&[(128., 128.), (384., 128.), (384., 318.), (318., 384.), (128., 384.)], CREAM, 14., window);
    if p.wipe > 0. {
        let w = 168. * p.wipe;
        poly(&[(160., 204.), (160. + w, 204.), (160. + w, 238.), (160., 238.)], PINK, 10., window);
    }
    for (i, (x0, x1, y)) in [(168., 300., 176.), (168., 286., 276.), (168., 248., 322.)].iter().enumerate() {
        let f = p.lines[i];
        if f <= 0. {
            continue;
        }
        let mut s = PathBuilder::stroke(stroke(14.));
        s.move_to(note(*x0, *y));
        s.line_to(note(x0 + (x1 - x0) * f, *y));
        if let Ok(path) = s.build() {
            window.paint_path(path, rgb(INK));
        }
    }
    if p.peel > 0. {
        // The folded corner grows out of the note's bottom-right.
        let g = |x: f32, y: f32| (384. + (x - 384.) * p.peel, 318. + (y - 318.) * p.peel);
        let pts = [g(384., 318.), g(330., 318.), g(318., 330.), g(318., 384.)];
        poly(&pts, PEEL, 14., window);
    }
    if p.pin > 0. {
        let centre = to_screen(256., (128. - 256.) * 1.2 + 262. + 0.);
        let r = 24. * 1.2 * k * p.pin.max(0.);
        let bounds = Bounds::new(point(centre.x - px(r), centre.y - px(r)), gpui::size(px(r * 2.), px(r * 2.)));
        window.paint_quad(gpui::fill(bounds, rgb(INK)).corner_radii(px(r)));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_fall_lands_flat() {
        let (dy, turn) = fall(1.);
        assert!(dy.abs() < 1e-3 && turn.abs() < 1e-3);
        assert!(fall(0.).0 < -200.);
        let p = pose(5., true);
        assert!((p.tile - 1.).abs() < 1e-4 && p.wipe == 1. && p.lines == [1.; 3]);
        assert!(back_out(0.7) > 1., "the spring overshoots");
    }
}
