//! One note: its paper, an optional picture or link card, a title, and
//! its text, with task lines drawn as real checkboxes you can tick. Laid
//! out at the camera's zoom; when zoomed far out only the title shows,
//! which keeps a crowded sheet legible and cheap to draw.

use gpui::component::text::{TextView, TextViewStyle};
use gpui::{canvas, div, img, prelude::*, px, AnyElement, App, Context, MouseButton, ObjectFit, StyledImage, Window};

use super::app::{Folio, Gesture, Selection};
use super::home::first_line;
use super::*;
use crate::model::*;
use crate::theme::Theme;
use crate::time::{self, now};

/// Below this zoom, a note shows its title only.
const DETAIL_ZOOM: f32 = 0.45;

pub fn size_factor(z: TextSize) -> f32 {
    match z {
        TextSize::Small => 0.9,
        TextSize::Medium => 1.0,
        TextSize::Large => 1.2,
    }
}

/// A note's content: link card, title, then its text, task lines as
/// checkboxes (`on_toggle` gets the task's number). `k` scales sizes.
pub fn note_body(
    s: &Sticker,
    k: f32,
    t: &Theme,
    media: &std::path::Path,
    md_ids: Option<u64>,
    on_toggle: impl Fn(usize, &mut Window, &mut App) + Clone + 'static,
) -> Vec<AnyElement> {
    let mut out: Vec<AnyElement> = Vec::new();
    let base = md_ids.unwrap_or(s.id) * 64;
    if let Some(l) = &s.link {
        let title = if l.title.is_empty() { l.url.clone() } else { l.title.clone() };
        out.push(
            div()
                .flex()
                .flex_col()
                .gap(px(4. * k))
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(6. * k))
                        .text_size(px(11. * k))
                        .text_color(t.colors.text_subtle)
                        .child(match &l.icon {
                            Some(name) => img(media.join(name)).size(px(14. * k)).rounded(px(3.)).into_any_element(),
                            None => icon("icons/globe.svg", px(12. * k), t.colors.text_subtle).into_any_element(),
                        })
                        .child(l.site.clone()),
                )
                .child(
                    div()
                        .text_size(px(15. * k))
                        .line_height(px(20. * k))
                        .font_weight(gpui::FontWeight::SEMIBOLD)
                        .text_color(t.colors.text)
                        .child(title),
                )
                .into_any_element(),
        );
    }
    if !s.title.is_empty() {
        out.push(
            div()
                .text_size(px(15. * k))
                .font_weight(gpui::FontWeight::SEMIBOLD)
                .text_color(t.colors.text)
                .line_height(px(20. * k))
                .child(s.title.clone())
                .into_any_element(),
        );
    }
    // Plain stretches as Markdown, task lines as checkboxes.
    let mut plain: Vec<&str> = Vec::new();
    let mut seg = 0u64;
    let mut task = 0usize;
    let flush = |plain: &mut Vec<&str>, seg: &mut u64, out: &mut Vec<AnyElement>| {
        let text = plain.join("\n");
        plain.clear();
        if text.trim().is_empty() {
            return;
        }
        out.push(
            div()
                .text_size(px(13. * k))
                .line_height(px(20. * k))
                .text_color(t.colors.text_body)
                .child(TextView::markdown(("md", base + *seg), tint_marks(&text, t.is_dark)).selectable(false).style(markdown_style(t)))
                .into_any_element(),
        );
        *seg += 1;
    };
    let mut tasks: Vec<AnyElement> = Vec::new();
    for line in s.body.lines() {
        match task_line(line) {
            Some((done, text)) => {
                if !plain.is_empty() {
                    if !tasks.is_empty() {
                        out.push(div().flex().flex_col().gap(px(5. * k)).children(std::mem::take(&mut tasks)).into_any_element());
                    }
                    flush(&mut plain, &mut seg, &mut out);
                }
                let n = task;
                task += 1;
                let toggle = on_toggle.clone();
                let b = px(15. * k);
                tasks.push(
                    div()
                        .flex()
                        .items_start()
                        .gap(px(8. * k))
                        .child(
                            div()
                                .id(("task", base + n as u64))
                                .mt(px(2.5 * k))
                                .size(b)
                                .flex_none()
                                .rounded(px(4. * k))
                                .border_1()
                                .border_color(if done { t.colors.brand } else { t.colors.text_subtle })
                                .when(done, |d| d.bg(t.colors.brand))
                                .flex()
                                .items_center()
                                .justify_center()
                                .cursor_pointer()
                                .when(done, |d| d.child(icon("icons/check.svg", px(11. * k), gpui::white())))
                                .on_mouse_down(MouseButton::Left, move |_, window, cx| {
                                    cx.stop_propagation();
                                    toggle(n, window, cx);
                                }),
                        )
                        .child({
                            // Markdown too, so **bold** and highlights work in tasks.
                            let md = if done && !text.trim().is_empty() { format!("~~{}~~", text.trim()) } else { text.to_string() };
                            div()
                                .flex_1()
                                .min_w_0()
                                .text_size(px(13. * k))
                                .line_height(px(20. * k))
                                .text_color(if done { t.colors.text_subtle } else { t.colors.text_body })
                                .child(
                                    TextView::markdown(("task-text", base + n as u64), tint_marks(&md, t.is_dark))
                                        .selectable(false)
                                        .style(markdown_style(t)),
                                )
                        })
                        .into_any_element(),
                );
            }
            None => {
                if !tasks.is_empty() {
                    out.push(div().flex().flex_col().gap(px(5. * k)).children(std::mem::take(&mut tasks)).into_any_element());
                }
                plain.push(line);
            }
        }
    }
    if !tasks.is_empty() {
        out.push(div().flex().flex_col().gap(px(5. * k)).children(tasks).into_any_element());
    }
    flush(&mut plain, &mut seg, &mut out);
    out
}

impl Folio {
    /// How far (screen px) note `id` is from rest as it settles after a
    /// drop: a quick, damped bounce over a third of a second.
    fn settle_offset(&self, id: Id) -> f32 {
        let Some(start) = self.settling.iter().find(|x| x.0 == id).map(|x| x.1) else { return 0. };
        let t = start.elapsed().as_secs_f32();
        if t > 0.35 || self.board.settings.reduce_motion {
            return 0.;
        }
        -4. * (-9. * t).exp() * (22. * t).cos()
    }

    pub(super) fn render_sticker(&self, s: &Sticker, cam: Camera, t: &Theme, cx: &mut Context<Self>) -> AnyElement {
        let z = cam.zoom;
        let r = self.note_rect(s);
        let at = cam.to_screen(v2(r.x, r.y));
        let selected = self.is_selected(Selection::Sticker(s.id));
        let lifted = selected && matches!(self.gesture, Gesture::Move { moved: true, .. });
        let hovered = self.hovered == Some(s.id);
        let k = size_factor(s.text_size) * z;
        let detail = z >= DETAIL_ZOOM;
        let n = now();

        let picture = s.image.as_ref().map(|name| {
            let has_text = !s.title.is_empty() || !s.body.is_empty();
            // The id gives the picture frame state, so an animated GIF plays.
            img(self.image_path(name))
                .id(("picture", s.id))
                .w_full()
                .when(has_text || !s.fixed, |i| i.h(px((s.rect.w * 0.62).min(s.rect.h) * z)))
                .when(!has_text && s.fixed, |i| i.h(px(s.rect.h * z)))
                .object_fit(ObjectFit::Cover)
                .flex_none()
        });

        let id = s.id;
        let me = cx.entity();
        let toggle = move |n: usize, _: &mut Window, cx: &mut App| me.update(cx, |f, cx| f.toggle_task(id, n, cx));
        let content: Vec<AnyElement> = if detail {
            note_body(s, k, t, &crate::store::media_dir(&self.dir), None, toggle)
        } else {
            vec![div()
                .text_size(px(15. / DETAIL_ZOOM * z))
                .font_weight(gpui::FontWeight::SEMIBOLD)
                .text_color(t.colors.text)
                .child(first_line(s))
                .into_any_element()]
        };

        // Chips: tasks done, reminder, tags, stale, locked, on the desktop.
        let mut chips: Vec<AnyElement> = Vec::new();
        if detail {
            let chip = |text: String, color: gpui::Hsla, path: Option<&'static str>| {
                div()
                    .flex()
                    .items_center()
                    .gap(px(3. * z))
                    .px(px(6. * z))
                    .h(px(18. * z))
                    .rounded(px(9. * z))
                    .bg(gpui::hsla(0., 0., if t.is_dark { 1. } else { 0. }, 0.06))
                    .text_size(px(10.5 * z))
                    .text_color(color)
                    .children(path.map(|p| icon(p, px(10. * z), color)))
                    .child(text)
                    .into_any_element()
            };
            let (done, total) = task_progress(&s.body);
            if total > 0 {
                let c = if done == total { t.colors.brand_on_tint } else { t.colors.text_muted };
                chips.push(chip(format!("{done}/{total}"), c, Some("icons/list-checks.svg")));
            }
            if let Some(d) = s.due {
                let c = if d <= n { t.colors.alert } else { t.colors.brand_on_tint };
                chips.push(chip(time::due_label(d, n), c, Some("icons/bell.svg")));
            }
            for tag in s.tags().into_iter().take(3) {
                chips.push(chip(format!("#{tag}"), t.colors.text_muted, None));
            }
            if s.is_stale(n) {
                chips.push(chip("untouched for weeks".into(), t.colors.text_subtle, Some("icons/clock.svg")));
            }
            if s.popped.is_some() {
                chips.push(chip("on desktop".into(), t.colors.text_subtle, Some("icons/picture-in-picture-2.svg")));
            }
        }
        let measured = self.heights.clone();
        let natural = self.heights.borrow().get(&s.id).copied().unwrap_or(0.);
        let cut = s.fixed && natural > s.rect.h + 1.;
        let lines = ((natural - s.rect.h) / (20. * size_factor(s.text_size))).ceil().max(1.) as usize;
        let more_label = format!("+{lines} more line{}", if lines == 1 { "" } else { "s" });
        // Dropped notes settle with a small bounce; picked-up ones lift.
        let settle = self.settle_offset(s.id);
        let lift = if lifted { -3. } else { 0. };
        let measure = canvas(
            move |b, window, _| {
                let h = f32::from(b.size.height) / z;
                let old = measured.borrow_mut().insert(id, h);
                if old.is_none_or(|o| (o - h).abs() > 0.5) {
                    window.refresh();
                }
            },
            |_, _, _, _| {},
        )
        .absolute()
        .inset_0();

        let inner = div()
            .relative()
            .flex_none()
            .flex()
            .flex_col()
            .children(picture)
            .child(
                div()
                    .px(px(16. * z))
                    .py(px(14. * z))
                    .flex()
                    .flex_col()
                    .gap(px(8. * z))
                    .children(content)
                    .when(!chips.is_empty(), |d| d.child(div().flex().flex_wrap().gap(px(4. * z)).pt(px(2. * z)).children(chips))),
            )
            .child(measure);

        let grip = ((selected || hovered) && !s.locked).then(|| {
            let g = t.size.grip;
            div()
                .absolute()
                .right(px(4.))
                .bottom(px(4.))
                .size(g)
                .child(div().absolute().right_0().bottom_0().w(g).h(px(2.)).rounded(px(1.)).bg(t.colors.text_subtle))
                .child(div().absolute().right_0().bottom_0().w(px(2.)).h(g).rounded(px(1.)).bg(t.colors.text_subtle))
        });

        let shadow = if lifted { t.shadow_lifted(z) } else { t.shadow_card(z) };
        let edge = if t.is_dark { t.colors.border } else { gpui::hsla(0., 0., 0., 0.04) };
        let paper = t.paper(s.paper);

        // A faint sheen, lighter at the top, so the paper isn't flat.
        let mut top = paper;
        top.l = (top.l + if t.is_dark { 0.03 } else { 0.022 }).min(1.);
        div()
            .absolute()
            .left(px(at.x))
            .top(px(at.y + lift + settle))
            .w(px(r.w * z))
            .when(s.fixed, |d| d.h(px(s.rect.h * z)))
            .when(!s.fixed, |d| d.min_h(px(s.rect.h * z)))
            .rounded(px(12. * z))
            .bg(gpui::linear_gradient(180., gpui::linear_color_stop(top, 0.), gpui::linear_color_stop(paper, 0.6)))
            .border_1()
            .border_color(edge)
            .shadow(shadow)
            .font_family(t.note_font(s.font))
            .child(div().size_full().rounded(px(12. * z)).overflow_hidden().child(inner))
            .when(cut, |d| {
                d.child(
                    div()
                        .absolute()
                        .left_0()
                        .right_0()
                        .bottom_0()
                        .h(px(34. * z))
                        .rounded_b(px(12. * z))
                        .bg(gpui::linear_gradient(
                            180.,
                            gpui::linear_color_stop(paper.opacity(0.), 0.),
                            gpui::linear_color_stop(paper, 0.7),
                        ))
                        .flex()
                        .items_end()
                        .justify_center()
                        .pb(px(4. * z))
                        .text_size(px(10.5 * z))
                        .text_color(t.colors.text_subtle)
                        .child(more_label),
                )
            })
            .when(s.locked, |d| d.child(div().absolute().top(px(6. * z)).right(px(6. * z)).child(icon("icons/lock.svg", px(12. * z.max(0.8)), t.colors.text_subtle))))
            .when(selected, |d| {
                d.child(
                    div()
                        .absolute()
                        .top(px(-3.))
                        .left(px(-3.))
                        .right(px(-3.))
                        .bottom(px(-3.))
                        .rounded(px(12. * z + 3.))
                        .border_2()
                        .border_color(t.colors.selection),
                )
            })
            .children(grip)
            .into_any_element()
    }
}

/// Tables on the note's own paper, with a faint header row.
fn markdown_style(t: &Theme) -> TextViewStyle {
    let head = if t.is_dark { gpui::hsla(0., 0., 1., 0.05) } else { gpui::hsla(0., 0., 0., 0.035) };
    TextViewStyle::default()
        .paragraph_gap(gpui::rems(0.5))
        .table(gpui::StyleRefinement::default().bg(gpui::transparent_black()))
        .table_head(gpui::StyleRefinement::default().bg(head))
}

/// Highlight colours by name, as `<mark color="green">` in a note, light
/// and dark: soft enough that the note's text stays readable on them.
const MARKS: [(&str, u32, u32); 7] = [
    ("yellow", 0xfde68a, 0x6b5310),
    ("green", 0xbbf0cc, 0x1f5133),
    ("blue", 0xc7dcfb, 0x1e3a66),
    ("red", 0xfbcaca, 0x6b2525),
    ("pink", 0xf9d0e6, 0x6a2850),
    ("purple", 0xddd6fe, 0x44336f),
    ("orange", 0xfdd6ad, 0x6b3d14),
];

/// Rewrites named highlight colours to this theme's shades.
pub fn tint_marks(body: &str, dark: bool) -> String {
    if !body.contains("<mark") {
        return body.to_string();
    }
    let mut out = body.to_string();
    for (name, light, dim) in MARKS {
        let hex = format!("color=\"#{:06x}\"", if dark { dim } else { light });
        out = out
            .replace(&format!("color=\"{name}\""), &hex)
            .replace(&format!("color='{name}'"), &hex);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::tint_marks;

    #[test]
    fn named_marks_take_the_theme_shade() {
        let s = "a <mark color=\"yellow\">b</mark> <mark color='red'>c</mark>";
        assert_eq!(tint_marks(s, false), "a <mark color=\"#fde68a\">b</mark> <mark color=\"#fbcaca\">c</mark>");
        assert!(tint_marks(s, true).contains("#6b5310"));
        assert_eq!(tint_marks("plain", true), "plain");
    }
}
