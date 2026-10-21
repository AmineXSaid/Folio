//! A sheet in miniature: its areas, notes (in their papers, with titles
//! when there's room) and links, scaled to fit a box. Used by the home
//! cards and the drawer.

use gpui::{canvas, div, point, prelude::*, px, Div, PathBuilder};

use crate::model::*;
use crate::theme::Theme;

/// A `w`×`h` picture of `c`. `titles`: write note titles when legible.
pub fn miniature(c: &Category, w: f32, h: f32, titles: bool, t: &Theme) -> Div {
    let rects: Vec<Rect> = c.stickers.iter().map(|s| s.rect).collect();
    let bounds = rects.iter().copied().chain(c.frames.iter().map(|f| f.rect)).reduce(|a, b| a.union(&b));
    let base = div()
        .relative()
        .w(px(w))
        .h(px(h))
        .overflow_hidden()
        .rounded(t.radius.button)
        .bg(t.colors.canvas)
        .border_1()
        .border_color(t.colors.border);
    let Some(b) = bounds else {
        return base.child(
            div()
                .absolute()
                .inset_0()
                .flex()
                .items_center()
                .justify_center()
                .text_size(t.text.micro)
                .text_color(t.colors.text_subtle)
                .child("Empty sheet"),
        );
    };
    let pad = 0.08 * w.min(h);
    let k = ((w - 2. * pad) / b.w.max(1.)).min((h - 2. * pad) / b.h.max(1.)).min(0.6);
    let ox = (w - b.w * k) / 2. - b.x * k;
    let oy = (h - b.h * k) / 2. - b.y * k;
    let at = |r: &Rect| (ox + r.x * k, oy + r.y * k, r.w * k, r.h * k);

    let frames = c.frames.iter().map(|f| {
        let (x, y, fw, fh) = at(&f.rect);
        div()
            .absolute()
            .left(px(x))
            .top(px(y))
            .w(px(fw))
            .h(px(fh))
            .rounded(px(4.))
            .bg(t.accent(f.accent).opacity(0.10))
            .border_1()
            .border_color(t.accent(f.accent).opacity(0.35))
    });
    let ink = t.colors.text_subtle;
    let notes = c.stickers.iter().map(|s| {
        let (x, y, nw, nh) = at(&s.rect);
        let show_title = titles && nw > 46. && !s.title.is_empty();
        let bars = (nh / 9.).floor().clamp(0., 3.) as usize;
        div()
            .absolute()
            .left(px(x))
            .top(px(y))
            .w(px(nw))
            .h(px(nh))
            .rounded(px((10. * k).clamp(2., 6.)))
            .bg(t.paper(s.paper))
            .border_1()
            .border_color(if t.is_dark { t.colors.border } else { gpui::hsla(0., 0., 0., 0.06) })
            .overflow_hidden()
            .p(px((12. * k).clamp(2., 7.)))
            .flex()
            .flex_col()
            .gap(px(3.))
            .when(show_title, |d| {
                d.child(
                    div()
                        .text_size(px(9.5))
                        .line_height(px(11.))
                        .font_weight(gpui::FontWeight::SEMIBOLD)
                        .text_color(t.colors.text)
                        .truncate()
                        .child(s.title.clone()),
                )
            })
            .when(!show_title, |d| {
                d.child(div().h(px(3.)).w(relative_w(nw, 0.6)).rounded(px(2.)).bg(t.colors.text).opacity(0.55))
            })
            .children((0..bars.saturating_sub(1)).map(move |i| {
                div().h(px(2.)).w(relative_w(nw, [0.85, 0.7, 0.5][i % 3])).rounded(px(1.)).bg(ink).opacity(0.4)
            }))
    });
    let links: Vec<[(f32, f32); 4]> = c
        .links
        .iter()
        .filter_map(|l| Some((c.sticker(l.from)?, c.sticker(l.to)?)))
        .map(|(a, b2)| link_curve(&a.rect, &b2.rect).map(|p| (ox + p.x * k, oy + p.y * k)))
        .collect();
    let link_color = t.colors.text_subtle.opacity(0.7);
    base.child(
        canvas(
            |_, _, _| {},
            move |bounds, _, window, _| {
                for l in &links {
                    let o = bounds.origin;
                    let p = |i: usize| point(o.x + px(l[i].0), o.y + px(l[i].1));
                    let mut b = PathBuilder::stroke(px(1.)).dash_array(&[px(3.), px(3.)]);
                    b.move_to(p(0));
                    b.cubic_bezier_to(p(3), p(1), p(2));
                    if let Ok(path) = b.build() {
                        window.paint_path(path, link_color);
                    }
                }
            },
        )
        .absolute()
        .inset_0(),
    )
    .children(frames)
    .children(notes)
}

fn relative_w(w: f32, f: f32) -> gpui::Pixels {
    px(((w - 8.) * f).max(4.))
}
