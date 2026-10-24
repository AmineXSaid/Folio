//! The opening, like a studio's logo before a film: the pink tile springs
//! in, the note drops onto it and settles, the pin pops, the highlight
//! wipes across, the lines write themselves, the corner peels, and
//! "folio." rises letter by letter. It plays in full at every launch and
//! can't be skipped; the credit sits along the bottom.

use gpui::{canvas, div, point, prelude::*, px, rgb, rgba, Bounds, Context, Hsla, PathBuilder, Pixels, Point, Window};

use super::app::{Folio, Intro};

// The logo's own colours (assets/brand/folio-icon.svg).
const PINK: u32 = 0xf30d6a;
const CREAM: u32 = 0xf5f2e9;
const PEEL: u32 = 0xf4f0e2;
const INK: u32 = 0x000000;

/// When the opening starts to fade out, and how long the fade takes.
const OUT: f32 = 3.4;
const FADE: f32 = 0.6;

impl Intro {
    fn elapsed(&self) -> f32 {
        self.start.elapsed().as_secs_f32()
    }
    /// Fading out: what comes next shows through.
    pub fn revealing(&self) -> bool {
        self.elapsed() >= OUT
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
    lines: [f32; 2],
    peel: f32,
    zoom: f32,
}

fn pose(t: f32) -> Pose {
    Pose {
        tile: back_out(span(t, 0.15, 0.8)),
        drop: span(t, 0.7, 1.45),
        pin: back_out(span(t, 1.35, 1.75)),
        wipe: ease_out(span(t, 1.55, 1.95)),
        lines: [span(t, 1.62, 1.98), span(t, 1.82, 2.18)],
        peel: back_out(span(t, 1.9, 2.35)),
        zoom: 1. + 0.25 * ease_out(span(t, OUT, OUT + FADE)),
    }
}

/// The note's fall: from above and turned, overshooting, then settling
/// straight. (How far down, how many degrees.)
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
        if t > OUT + FADE {
            self.intro = None;
            // The first time, the name field is ready to type into.
            if self.welcome.is_some() {
                let input = self.welcome_input.clone();
                window.defer(cx, move |window, cx| input.update(cx, |i, cx| i.focus(window, cx)));
            }
            window.refresh();
            return div().into_any_element();
        }
        window.request_animation_frame();
        let p = pose(t);
        let fade = 1. - span(t, OUT, OUT + FADE);
        let size = 180.;

        let logo = canvas(|_, _, _| {}, move |b, _, window, _| paint_logo(b, p, size, window)).w(px(size * 1.4)).h(px(size * 1.4));

        let word_t = t - 1.95;
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
        let tag = ease_out(span(t, 2.35, 2.9));
        let glow = ease_out(span(t, 0.3, 2.0));
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
            // Covers the app: nothing underneath takes a click until it's done.
            .occlude()
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
                    .bottom(px(26.))
                    .left_0()
                    .right_0()
                    .flex()
                    .justify_center()
                    .text_size(px(12.))
                    .text_color(rgb(0x7d8682))
                    .opacity(tag)
                    .child("Created and designed by M. Amine Said"),
            )
            .into_any_element()
    }
}

/// Paints the logo (the drawing in assets/brand/folio-icon.svg, in its
/// 512 units), `size` across at rest, centred in `b`.
fn paint_logo(b: Bounds<Pixels>, p: Pose, size: f32, window: &mut Window) {
    let c = b.center();
    let k = size / 512. * p.zoom;
    let to_screen = |x: f32, y: f32| point(c.x + px((x - 256.) * k), c.y + px((y - 256.) * k));

    // The tile: a rounded square, springing out from its centre.
    if p.tile > 0. {
        let s = p.tile.max(0.);
        let half = 256. * k * s;
        let centre = to_screen(256., 256.);
        let bounds = Bounds::new(point(centre.x - px(half), centre.y - px(half)), gpui::size(px(half * 2.), px(half * 2.)));
        window.paint_quad(gpui::fill(bounds, rgba(PINK << 8 | (255. * clamp01(p.tile * 1.4)) as u32)).corner_radii(px(100. * k * s)));
    }
    if p.drop <= 0. {
        return;
    }
    // The note falls in turned a little and lands straight.
    let (dy, turn) = fall(p.drop);
    let (sin, cos) = turn.to_radians().sin_cos();
    let note = |x: f32, y: f32| -> Point<Pixels> {
        let (rx, ry) = (x - 256., y - 256.);
        to_screen(rx * cos - ry * sin + 256., rx * sin + ry * cos + 256. + dy)
    };
    // A filled shape in the note's space; `map` moves each point first
    // (the fold and the pin grow into place).
    let fill = |start: (f32, f32), segs: &[Seg], color: u32, map: &dyn Fn(f32, f32) -> (f32, f32), window: &mut Window| {
        let at = |(x, y): (f32, f32)| {
            let (x, y) = map(x, y);
            note(x, y)
        };
        let mut f = PathBuilder::fill();
        f.move_to(at(start));
        for seg in segs {
            match *seg {
                Seg::L(x, y) => f.line_to(at((x, y))),
                Seg::Q(c, to) => f.curve_to(at(to), at(c)),
                Seg::C(a, b, to) => f.cubic_bezier_to(at(to), at(a), at(b)),
            }
        }
        f.close();
        if let Ok(path) = f.build() {
            window.paint_path(path, rgb(color));
        }
    };
    let same = |x: f32, y: f32| (x, y);
    let rect = |x0: f32, y0: f32, x1: f32, y1: f32| ((x0, y0), [Seg::L(x1, y0), Seg::L(x1, y1), Seg::L(x0, y1)]);

    // The note: its black edge (the outline's outer side, corners round),
    // then the paper inside it, the corner cut where it folds.
    use Seg::*;
    fill(
        (97., 88.),
        &[
            L(417., 88.),
            Q((441., 88.), (441., 112.)),
            L(441., 336.),
            Q((441., 342.), (437., 346.)),
            L(343., 440.),
            Q((339., 443.), (334., 443.)),
            L(97., 443.),
            Q((73., 443.), (73., 419.)),
            L(73., 112.),
            Q((73., 88.), (97., 88.)),
        ],
        INK,
        &same,
        window,
    );
    fill((106., 121.), &[L(408., 121.), L(408., 326.2), L(324.2, 410.), L(106., 410.)], CREAM, &same, window);
    // The highlight wipes across, then the lines write themselves.
    if p.wipe > 0. {
        let (start, segs) = rect(137., 160., 137. + 240. * p.wipe, 231.);
        fill(start, &segs, PINK, &same, window);
    }
    for (i, (x1, y0, y1)) in [(281., 273., 306.), (230., 347., 378.)].iter().enumerate() {
        let f = p.lines[i];
        if f > 0. {
            let (start, segs) = rect(137., *y0, 137. + (x1 - 137.) * f, *y1);
            fill(start, &segs, INK, &same, window);
        }
    }
    if p.peel > 0. {
        // The folded corner grows out of the note's bottom-right: its
        // black edge, round where it turns, and the paper showing inside.
        let k = p.peel;
        let grow = move |x: f32, y: f32| (424.5 + (x - 424.5) * k, 426.5 + (y - 426.5) * k);
        fill(
            (314.5, 443.),
            &[L(314.5, 352.), Q((314.5, 316.5), (350., 316.5)), L(441., 316.5), L(441., 339.8), L(337.8, 443.)],
            INK,
            &grow,
            window,
        );
        fill((347.5, 349.5), &[L(384.7, 349.5), L(347.5, 386.7)], PEEL, &grow, window);
    }
    if p.pin > 0. {
        // The pin pops into its notch: a round head running straight
        // down into a tab that sits in the paper.
        let k = p.pin.max(0.);
        let pop = move |x: f32, y: f32| (258. + (x - 258.) * k, 105. + (y - 105.) * k);
        let c = 52. * 0.5523;
        fill(
            (206., 90.),
            &[
                C((206., 90. - c), (258. - c, 38.), (258., 38.)),
                C((258. + c, 38.), (310., 90. - c), (310., 90.)),
                L(310., 118.),
                Q((310., 136.), (292., 136.)),
                L(224., 136.),
                Q((206., 136.), (206., 118.)),
            ],
            INK,
            &pop,
            window,
        );
    }
}

/// A piece of a shape's edge: a line, a quadratic curve (its control
/// point, then where it ends), or a cubic one (two controls, the end).
enum Seg {
    L(f32, f32),
    Q((f32, f32), (f32, f32)),
    C((f32, f32), (f32, f32), (f32, f32)),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_fall_lands_flat() {
        let (dy, turn) = fall(1.);
        assert!(dy.abs() < 1e-3 && turn.abs() < 1e-3);
        assert!(fall(0.).0 < -200.);
        let p = pose(5.);
        assert!((p.tile - 1.).abs() < 1e-4 && p.wipe == 1. && p.lines == [1.; 2]);
        assert!(back_out(0.7) > 1., "the spring overshoots");
    }
}
