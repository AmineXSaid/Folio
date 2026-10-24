//! The opening, like a studio's logo before a film. Folio's dotted sheet
//! comes up out of the dark from the middle outwards under a slow pink
//! bloom; the pink tile springs in and settles on a soft shadow; the note
//! drops onto it (the tile gives a little as it lands), the pin pops, the
//! highlight wipes across, the lines write themselves, the corner peels,
//! and a sheen of light crosses the paper. "folio." tracks in (its letters
//! closing up as they rise), a pink hairline draws out under it, and the
//! line beneath fades up. Then the wordmark flies into the top bar, to
//! where Folio's own "folio." sits, as everything else dissolves. It plays
//! in full at every launch and can't be skipped; the author's credit sits
//! in the bottom-left corner.

use gpui::{canvas, div, point, prelude::*, px, rgb, rgba, Bounds, BoxShadow, Context, Corners, Hsla, PathBuilder, Pixels, Point, Window};

use super::app::{Folio, Intro};

// The logo's own colours (assets/brand/folio-icon.svg).
const PINK: u32 = 0xf30d6a;
const CREAM: u32 = 0xf5f2e9;
const PEEL: u32 = 0xf4f0e2;
const INK: u32 = 0x000000;
/// The dark it all comes out of.
const NIGHT: u32 = 0x0b0d0c;

/// When the opening starts to leave, how long the wordmark's flight takes,
/// and how long the rest takes to dissolve.
const OUT: f32 = 3.2;
const FLY: f32 = 0.75;
const FADE: f32 = 0.6;
/// The wordmark's size at rest, and the top bar's.
const WORD: f32 = 58.;
const MARK: f32 = 15.;

impl Intro {
    fn elapsed(&self) -> f32 {
        // FOLIO_SLOW_MOTION slows it too, to look at one moment of it.
        self.start.elapsed().as_secs_f32() / super::motion::slow() as f32
    }
    /// Leaving: what comes next shows through.
    pub fn revealing(&self) -> bool {
        self.elapsed() >= OUT
    }
    /// The wordmark has landed in the top bar: the bar shows its own.
    pub fn landed(&self) -> bool {
        self.elapsed() >= OUT + FLY
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
/// Slow at both ends: the flight.
fn ease_in_out(x: f32) -> f32 {
    if x < 0.5 {
        4. * x * x * x
    } else {
        1. - (-2. * x + 2.).powi(3) / 2.
    }
}
/// Overshoots a little, then settles.
fn back_out(x: f32) -> f32 {
    let c = 1.4;
    1. + (c + 1.) * (x - 1.).powi(3) + c * (x - 1.).powi(2)
}
fn mix(a: f32, b: f32, k: f32) -> f32 {
    a + (b - a) * k
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
    /// The light crossing the paper, 0 to 1 (none outside that).
    sheen: f32,
    /// How much the tile gives as the note lands on it.
    give: f32,
    zoom: f32,
}

fn pose(t: f32) -> Pose {
    let drop = span(t, 0.7, 1.45);
    Pose {
        tile: back_out(span(t, 0.15, 0.85)),
        drop,
        pin: back_out(span(t, 1.35, 1.75)),
        wipe: ease_out(span(t, 1.55, 1.95)),
        lines: [span(t, 1.62, 1.98), span(t, 1.82, 2.18)],
        peel: back_out(span(t, 1.9, 2.35)),
        sheen: ease_in_out(span(t, 2.3, 2.95)),
        // A short dip as the note's fall turns into its bounce.
        give: (std::f32::consts::PI * span(drop, 0.55, 0.85)).sin(),
        // Leaving, the logo draws back a little as it fades.
        zoom: 1. - 0.08 * ease_out(span(t, OUT, OUT + FADE)),
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

/// Folio's sheet in the dark: dots on its grid, coming up from the middle
/// outwards, settling from a little nearer, fading to nothing at the edges.
fn paint_sheet(b: Bounds<Pixels>, t: f32, window: &mut Window) {
    let c = b.center();
    let (cx, cy) = (f32::from(c.x), f32::from(c.y));
    let reach = (f32::from(b.size.width).powi(2) + f32::from(b.size.height).powi(2)).sqrt() / 2.;
    let near = 1.06 - 0.06 * ease_out(span(t, 0., 1.8));
    let step = 22. * near;
    let (nx, ny) = ((f32::from(b.size.width) / 2. / step).ceil() as i32 + 1, (f32::from(b.size.height) / 2. / step).ceil() as i32 + 1);
    let dot: Hsla = rgb(CREAM).into();
    for i in -nx..=nx {
        for j in -ny..=ny {
            let (x, y) = (cx + i as f32 * step, cy + j as f32 * step);
            let d = (((x - cx).powi(2) + (y - cy).powi(2)).sqrt() / reach).min(1.);
            let shown = ease_out(span(t, 0.05 + d * 0.9, 0.55 + d * 0.9));
            let a = 0.3 * shown * (1. - d).powf(1.2);
            if a < 0.004 {
                continue;
            }
            let r = px(1.25);
            let at = Bounds::new(point(px(x) - r, px(y) - r), gpui::size(r * 2., r * 2.));
            window.paint_quad(gpui::fill(at, dot.opacity(a)).corner_radii(r));
        }
    }
}

impl Folio {
    pub(super) fn render_intro(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let Some(intro) = self.intro.as_ref() else { return div().into_any_element() };
        let t = intro.elapsed();
        if t > OUT + FLY.max(FADE) {
            self.intro = None;
            // The first time, the name field is ready to type into.
            if self.welcome.is_some() {
                let input = self.welcome_input.clone();
                window.defer(cx, move |window, cx| input.update(cx, |i, cx| i.focus(window, cx)));
            }
            // One more frame, so what waited for the opening shows.
            window.request_animation_frame();
            return div().into_any_element();
        }
        window.request_animation_frame();
        let ui = crate::theme::theme(cx).clone();
        let p = pose(t);
        let size = 180.;
        // The dark lifts; what was drawn on it goes a little sooner.
        let night = 1. - ease_out(span(t, OUT, OUT + FADE));
        let content = 1. - ease_out(span(t, OUT, OUT + FADE * 0.6));

        let logo = canvas(|_, _, _| {}, move |b, _, window, _| paint_logo(b, p, size, window)).w(px(size * 1.4)).h(px(size * 1.4));
        let sheet = canvas(|_, _, _| {}, move |b, _, window, _| paint_sheet(b, t, window)).absolute().inset_0();

        // "folio." tracks in: each letter rises and comes up, and the gaps
        // between them close as they do.
        let word_t = t - 1.95;
        let tracking = 16. * (1. - ease_out(span(word_t, 0., 0.95)));
        let cream: Hsla = rgb(0xf4f1ea).into();
        let pink: Hsla = rgb(PINK).into();
        let letters = |color: Hsla, size: f32, settled: bool| {
            "folio.".chars().enumerate().map(move |(i, ch)| {
                let k = if settled { 1. } else { ease_out(span(word_t, i as f32 * 0.07, i as f32 * 0.07 + 0.6)) };
                let k = if i == 5 && !settled { back_out(span(word_t, 0.45, 0.85)) } else { k };
                div()
                    .relative()
                    .top(px(22. * (1. - k)))
                    .opacity(clamp01(k))
                    .text_color(if i == 5 { pink } else { color })
                    .text_size(px(size))
                    .child(ch.to_string())
            })
        };
        // Where the wordmark goes: the top bar's, once Folio shows one (not
        // behind the first welcome).
        let from = intro.word.clone();
        let to = self.mark_at.get().filter(|_| self.welcome.is_none());
        let flying = t >= OUT && to.is_some() && from.get().is_some();
        let measure = intro.word.clone();
        let word = div()
            .relative()
            .flex()
            .gap(px(tracking))
            .font_weight(gpui::FontWeight::BOLD)
            .line_height(px(WORD * 1.1))
            // Its place in the middle, for the flight.
            .child(canvas(move |b, _, _| measure.set(Some(b)), |_, _, _, _| {}).absolute().inset_0())
            .when(flying, |d| d.opacity(0.))
            .children(letters(cream, WORD, false));
        let rule = div().h(px(1.)).w(px(132. * ease_out(span(t, 2.45, 3.05)))).bg(pink.opacity(0.85));
        let tag = ease_out(span(t, 2.55, 3.05));
        let glow = ease_out(span(t, 0.2, 2.0)) * (0.9 + 0.1 * (t * 2.2).sin());

        let overlay = div()
            .id("intro")
            .absolute()
            .inset_0()
            .bg(rgb(NIGHT))
            .opacity(night)
            .flex()
            .flex_col()
            .items_center()
            .justify_center()
            .gap(px(18.))
            // Covers the app: nothing underneath takes a click until it's done.
            .occlude()
            .child(sheet)
            // A slow pink bloom behind the logo, and a wider, fainter warmth.
            .child(div().absolute().size(px(300.)).rounded(px(150.)).shadow(vec![
                BoxShadow { color: pink.opacity(0.30 * glow), offset: point(px(0.), px(0.)), blur_radius: px(180.), spread_radius: px(50.), inset: false },
                BoxShadow { color: rgba(0xf5f2e9ff).into(), offset: point(px(0.), px(0.)), blur_radius: px(320.), spread_radius: px(0.), inset: false },
            ]).opacity(0.9))
            // The corners stay darkest.
            .child(div().absolute().inset_0().shadow(vec![BoxShadow {
                color: gpui::black().opacity(0.55),
                offset: point(px(0.), px(0.)),
                blur_radius: px(260.),
                spread_radius: px(0.),
                inset: true,
            }]))
            .child(div().opacity(content).flex().flex_col().items_center().gap(px(18.)).child(logo).child(word))
            .child(div().opacity(content).child(rule))
            .child(
                div()
                    .relative()
                    .top(px(6. * (1. - tag)))
                    .text_size(px(15.))
                    .text_color(rgba(0xa5ada8ff))
                    .opacity(tag * content)
                    .child("Sticky notes on endless sheets"),
            )
            .child(
                // The credit, in the corner where the welcome keeps it: a
                // glassy card of the logo's cream on the dark.
                div()
                    .absolute()
                    .left(ui.space.xl)
                    .bottom(ui.space.xl)
                    .opacity(tag)
                    .child(super::author_badge(&ui, rgba(CREAM << 8 | 0x0f).into(), rgb(CREAM).into())),
            );

        // The flight: from the middle to the top bar, growing small, its
        // cream turning the colour of the bar's text, over everything.
        let flight = match (flying, from.get(), to) {
            (true, Some(a), Some(b)) => {
                let k = ease_in_out(span(t, OUT, OUT + FLY));
                let size = mix(WORD, MARK, k);
                let lh = size * 1.1;
                // Lined up by its left edge and its middle, from the middle
                // of the opening to the bar's mark (inside its padding).
                let (ax, ay) = (f32::from(a.origin.x), f32::from(a.center().y));
                let (bx, by) = (f32::from(b.origin.x + ui.space.xs), f32::from(b.center().y));
                let (x, y) = (mix(ax, bx, k), mix(ay, by, k));
                // Its colour follows the dark lifting, so it always reads.
                let color = mix_color(cream, ui.colors.text, k.max(1. - night));
                Some(
                    div()
                        .absolute()
                        .left(px(x))
                        .top(px(y - lh / 2.))
                        .flex()
                        .font_weight(gpui::FontWeight::BOLD)
                        .line_height(px(lh))
                        .children(letters(color, size, true)),
                )
            }
            _ => None,
        };
        div().absolute().inset_0().child(overlay).children(flight).into_any_element()
    }
}

/// `a` turning into `b`, `k` of the way.
fn mix_color(a: Hsla, b: Hsla, k: f32) -> Hsla {
    let (a, b) = (a.to_rgb(), b.to_rgb());
    gpui::Rgba { r: mix(a.r, b.r, k), g: mix(a.g, b.g, k), b: mix(a.b, b.b, k), a: mix(a.a, b.a, k) }.into()
}

/// Paints the logo (the drawing in assets/brand/folio-icon.svg, in its
/// 512 units), `size` across at rest, centred in `b`.
fn paint_logo(b: Bounds<Pixels>, p: Pose, size: f32, window: &mut Window) {
    let c = b.center();
    let k = size / 512. * p.zoom;
    let to_screen = |x: f32, y: f32| point(c.x + px((x - 256.) * k), c.y + px((y - 256.) * k));

    // The tile: a rounded square, springing out from its centre onto a
    // soft shadow, giving a little as the note lands on it.
    if p.tile > 0. {
        let s = p.tile.max(0.);
        let (sx, sy) = (s * (1. + 0.012 * p.give), s * (1. - 0.03 * p.give));
        let (hw, hh) = (256. * k * sx, 256. * k * sy);
        let centre = to_screen(256., 256. + 7. * p.give);
        let bounds = Bounds::new(point(centre.x - px(hw), centre.y - px(hh)), gpui::size(px(hw * 2.), px(hh * 2.)));
        let radius = px(100. * k * s);
        let settled = clamp01(p.tile);
        window.paint_drop_shadows(
            bounds,
            Corners::all(radius),
            &[
                BoxShadow { color: gpui::black().opacity(0.5 * settled), offset: point(px(0.), px(size * 0.1)), blur_radius: px(size * 0.26), spread_radius: px(-size * 0.05), inset: false },
                BoxShadow { color: rgba(PINK << 8 | 0xff).into(), offset: point(px(0.), px(size * 0.06)), blur_radius: px(size * 0.4), spread_radius: px(-size * 0.12), inset: false },
            ],
        );
        window.paint_quad(gpui::fill(bounds, rgba(PINK << 8 | (255. * clamp01(p.tile * 1.4)) as u32)).corner_radii(radius));
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
    paint_sheen(p, &to_screen, window);
}

/// A sheen of light crossing the whole tile, over the note, as over glass:
/// soft-edged (three bands, each wider and fainter), slanted, each band
/// cut to the tile's rounded square so none of it falls outside.
fn paint_sheen(p: Pose, to_screen: &dyn Fn(f32, f32) -> Point<Pixels>, window: &mut Window) {
    if p.sheen <= 0. || p.sheen >= 1. {
        return;
    }
    let tile = rounded_square(512., 100., 10);
    let x = -200. + 900. * p.sheen;
    for (w, alpha) in [(120., 0.06), (64., 0.08), (26., 0.11)] {
        let band = vec![(x - w / 2., -10.), (x + w / 2., -10.), (x + w / 2. - 200., 522.), (x - w / 2. - 200., 522.)];
        let lit = clip_convex(band, &tile);
        let Some((&first, rest)) = lit.split_first() else { continue };
        let mut f = PathBuilder::fill();
        f.move_to(to_screen(first.0, first.1));
        for &(px_, py) in rest {
            f.line_to(to_screen(px_, py));
        }
        f.close();
        if let Ok(path) = f.build() {
            window.paint_path(path, gpui::white().opacity(alpha));
        }
    }
}

/// A square `side` across with corners rounded by `r`, as a polygon going
/// round it, `steps` segments to each corner.
fn rounded_square(side: f32, r: f32, steps: usize) -> Vec<(f32, f32)> {
    let centres = [(side - r, r, -90.), (side - r, side - r, 0.), (r, side - r, 90.), (r, r, 180.)];
    let mut out = Vec::new();
    for (cx, cy, from) in centres {
        for i in 0..=steps {
            let a = (from + 90. * i as f32 / steps as f32).to_radians();
            out.push((cx + r * a.cos(), cy + r * a.sin()));
        }
    }
    out
}

/// The part of polygon `subject` inside the convex polygon `edge`
/// (Sutherland–Hodgman: cut by each of its sides in turn).
fn clip_convex(subject: Vec<(f32, f32)>, edge: &[(f32, f32)]) -> Vec<(f32, f32)> {
    // Which side is inside: the side the polygon turns towards.
    let area: f32 = edge.iter().zip(edge.iter().cycle().skip(1)).map(|(a, b)| a.0 * b.1 - b.0 * a.1).sum();
    let turn = area.signum();
    let side = |a: (f32, f32), b: (f32, f32), p: (f32, f32)| ((b.0 - a.0) * (p.1 - a.1) - (b.1 - a.1) * (p.0 - a.0)) * turn;
    let mut out = subject;
    for (i, &a) in edge.iter().enumerate() {
        let b = edge[(i + 1) % edge.len()];
        let input = std::mem::take(&mut out);
        for (j, &p) in input.iter().enumerate() {
            let q = input[(j + 1) % input.len()];
            let (sp, sq) = (side(a, b, p), side(a, b, q));
            if sp >= 0. {
                out.push(p);
            }
            if (sp >= 0.) != (sq >= 0.) {
                let k = sp / (sp - sq);
                out.push((p.0 + (q.0 - p.0) * k, p.1 + (q.1 - p.1) * k));
            }
        }
        if out.is_empty() {
            break;
        }
    }
    out
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
        let p = pose(OUT - 0.01);
        assert!((p.tile - 1.).abs() < 1e-4 && p.wipe == 1. && p.lines == [1.; 2]);
        assert!(p.give.abs() < 1e-4 && p.sheen == 1. && (p.zoom - 1.).abs() < 1e-4, "at rest before leaving");
        assert!(back_out(0.7) > 1., "the spring overshoots");
        assert!(pose(0.7 + 0.75 * 0.7).give > 0.5, "the tile gives as the note lands");
        assert!((ease_in_out(0.5) - 0.5).abs() < 1e-4 && ease_in_out(1.) == 1.);
    }

    #[test]
    fn the_sheen_stays_on_the_tile() {
        let tile = rounded_square(512., 100., 10);
        // A band across a corner keeps only what's inside the curve.
        let lit = clip_convex(vec![(-50., -50.), (60., -50.), (60., 600.), (-50., 600.)], &tile);
        assert!(!lit.is_empty());
        for (x, y) in &lit {
            let (cx, cy) = (x.clamp(100., 412.), y.clamp(100., 412.));
            let d = ((x - cx).powi(2) + (y - cy).powi(2)).sqrt();
            assert!(d <= 100.5 && *x >= -0.01 && *y >= -0.01, "({x}, {y}) is off the tile");
        }
        // Wholly outside: nothing.
        assert!(clip_convex(vec![(600., 0.), (700., 0.), (700., 50.), (600., 50.)], &tile).is_empty());
    }
}
