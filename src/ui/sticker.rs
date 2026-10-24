//! One note: its paper, an optional picture or link card, a title, and
//! its text, with task lines drawn as real checkboxes you can tick. Laid
//! out at the camera's zoom; when zoomed far out only the title shows,
//! which keeps a crowded sheet legible and cheap to draw.

use gpui::component::text::{TextView, TextViewStyle};
use gpui::{canvas, div, img, prelude::*, px, AnyElement, App, Context, MouseButton, ObjectFit, StyledImage, Window};

use super::app::{Folio, Gesture, Selection};
use crate::markdown::{blocks, Block};
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

/// A note's content, taken apart so each type can place its title: on a
/// cheat card's band, above an index card's rule, or at the top.
pub struct NoteParts {
    pub title: Option<AnyElement>,
    pub body: Vec<AnyElement>,
}

/// The line height a note's text keeps: index cards write on 24 px rules.
pub fn line_height(style: NoteStyle) -> f32 {
    if style == NoteStyle::Index {
        24.
    } else {
        20.
    }
}

/// Corner radii, top and bottom, in world units: taped paper is squarer
/// at the top and curls at the foot; index cards are card stock.
pub fn radii(style: NoteStyle) -> (f32, f32) {
    match style {
        NoteStyle::Card | NoteStyle::Cheat => (12., 12.),
        NoteStyle::Taped => (4., 10.),
        NoteStyle::Index => (3., 3.),
    }
}

/// A strip of clear tape over a taped note's top edge. Where it lands
/// along the edge varies by note, as tape does. A pinned note goes without.
pub fn tape(id: Id, w: f32, z: f32, t: &Theme) -> gpui::Div {
    let at = [0.5, 0.27, 0.7][(id % 3) as usize];
    div()
        .absolute()
        .top(px(-11. * z))
        .left(px(w * at - 42. * z))
        .w(px(84. * z))
        .h(px(22. * z))
        .bg(t.colors.tape)
        .border_l_1()
        .border_r_1()
        .border_color(t.colors.tape_edge)
        .shadow(t.shadow_raised())
}

/// A cheat card's title band: a deeper shade of the paper.
pub fn band(title: Option<AnyElement>, paper: Paper, z: f32, t: &Theme) -> gpui::Div {
    div()
        .w_full()
        .flex()
        .items_center()
        .px(px(14. * z))
        .py(px(if title.is_some() { 9. * z } else { 4. * z }))
        .bg(t.paper_band(paper))
        .children(title)
}

pub(super) fn md_view(id: u64, text: &str, t: &Theme) -> TextView {
    TextView::markdown(("md", id), tint_marks(text, t.is_dark)).selectable(false).style(markdown_style(t))
}

/// Markdown written in an ink of its own: text on the sheet.
pub(super) fn md_view_in(id: u64, text: &str, ink: gpui::Hsla, t: &Theme) -> TextView {
    md_view(id, text, t).text_color(ink)
}

/// A note's content: link card, title, then its text, with task lines as
/// checkboxes (`on_toggle` gets the task's number), `>` lines as a
/// Remember box and tables on the paper. `k` scales sizes. With
/// `selectable` its text can be picked out with the mouse and copied.
pub fn note_body(
    s: &Sticker,
    k: f32,
    t: &Theme,
    media: &std::path::Path,
    md_ids: Option<u64>,
    selectable: bool,
    on_toggle: impl Fn(usize, &mut Window, &mut App) + Clone + 'static,
) -> NoteParts {
    let md_view = |id: u64, text: &str, t: &Theme| md_view(id, text, t).selectable(selectable);
    let lh = line_height(s.style);
    let ruled = s.style == NoteStyle::Index;
    let mut body: Vec<AnyElement> = Vec::new();
    let base = md_ids.unwrap_or(s.id) * 1024;
    if let Some(l) = &s.link {
        let title = if l.title.is_empty() { l.url.clone() } else { l.title.clone() };
        let url = l.url.clone();
        let tip = format!("Open {}", l.site);
        body.push(
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
                        .child(div().flex_1().min_w_0().truncate().child(l.site.clone()))
                        // Opens the page; pressing it never picks up the note.
                        .child(
                            div()
                                .id(("open-link", s.id))
                                .flex_none()
                                .size(px(22. * k))
                                .flex()
                                .items_center()
                                .justify_center()
                                .rounded(px(6. * k))
                                .cursor_pointer()
                                .hover(|d| d.bg(t.colors.hover))
                                .child(icon("icons/arrow-up-right.svg", px(13. * k), t.colors.text_muted))
                                .tooltip(move |window, cx| super::tip(tip.clone(), window, cx))
                                .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                                .on_click(move |_, _, cx| cx.open_url(&url)),
                        ),
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
    let title = (!s.title.is_empty()).then(|| {
        let size = if s.style == NoteStyle::Cheat { 14. } else { 15. };
        div()
            .text_size(px(size * k))
            .font_weight(gpui::FontWeight::SEMIBOLD)
            .text_color(t.colors.text)
            .line_height(px(if s.style == NoteStyle::Index { 24. } else { 20. } * k))
            .child(s.title.clone())
            .into_any_element()
    });
    let mut seg = 0u64;
    let mut next_id = || {
        seg += 1;
        base + seg
    };
    let mut task = 0usize;
    for block in blocks(&s.body) {
        match block {
            // An index card keeps a ruled line for every line typed, blank
            // ones too, so the text never drifts off its rules.
            Block::Text(text) if s.style == NoteStyle::Index => {
                for line in text.lines() {
                    let l = crate::markdown::ruled_line(line);
                    body.push(
                        div()
                            .flex()
                            .gap(px(6. * k))
                            .min_h(px(lh * k))
                            .pl(px(14. * k * l.indent as f32))
                            .text_size(px(13. * k))
                            .line_height(px(lh * k))
                            .text_color(t.colors.text_body)
                            .children(l.marker.map(|m| div().flex_none().text_color(t.colors.text_muted).child(m)))
                            .when(!l.text.is_empty(), |d| d.child(div().flex_1().min_w_0().child(md_view(next_id(), &l.text, t))))
                            .into_any_element(),
                    );
                }
            }
            Block::Text(text) => {
                body.push(
                    div()
                        .text_size(px(13. * k))
                        .line_height(px(lh * k))
                        .text_color(t.colors.text_body)
                        .child(md_view(next_id(), &text, t))
                        .into_any_element(),
                )
            }
            Block::Tasks(items) => {
                let rows = items.into_iter().map(|(done, text)| {
                    let n = task;
                    task += 1;
                    let toggle = on_toggle.clone();
                    let b = px(15. * k);
                    // Markdown too, so **bold** and highlights work in tasks.
                    let md = if done && !text.trim().is_empty() { format!("~~{}~~", text.trim()) } else { text };
                    div()
                        .flex()
                        .items_start()
                        .gap(px(8. * k))
                        .child(
                            div()
                                .id(("task", base + n as u64))
                                .mt(px((lh - 15.) / 2. * k))
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
                        .child(
                            div()
                                .flex_1()
                                .min_w_0()
                                .text_size(px(13. * k))
                                .line_height(px(lh * k))
                                .text_color(if done { t.colors.text_subtle } else { t.colors.text_body })
                                .child(md_view(next_id(), &md, t)),
                        )
                        .into_any_element()
                });
                let rows: Vec<AnyElement> = rows.collect();
                let gap = if s.style == NoteStyle::Index { 0. } else { 5. * k };
                body.push(div().flex().flex_col().gap(px(gap)).children(rows).into_any_element());
            }
            // On an index card the box spans whole ruled lines.
            Block::Callout { label, text } => body.push(
                div()
                    .flex()
                    .flex_col()
                    .when(!ruled, |d| d.gap(px(2. * k)).py(px(8. * k)))
                    .px(px(10. * k))
                    .rounded(px(8. * k))
                    .bg(t.colors.wash)
                    .when_some(label, |d, label| {
                        d.child(
                            div()
                                .flex()
                                .items_center()
                                .when(ruled, |d| d.h(px(lh * k)))
                                .gap(px(5. * k))
                                .text_size(px(10. * k))
                                .font_weight(gpui::FontWeight::BOLD)
                                .text_color(t.colors.text_body)
                                .child(icon("icons/lightbulb.svg", px(11. * k), t.colors.text_body))
                                .child(label.to_uppercase()),
                        )
                    })
                    .when(!text.is_empty(), |d| {
                        d.child(
                            div()
                                .text_size(px(12.5 * k))
                                .line_height(px(if ruled { lh } else { 18. } * k))
                                .text_color(t.colors.text)
                                .child(md_view(next_id(), &text, t)),
                        )
                    })
                    .into_any_element(),
            ),
            Block::Table { head, rows } => {
                let last = rows.len();
                let mut row = |cells: Vec<String>, i: usize| {
                    let head_row = i == 0;
                    div()
                        .flex()
                        .gap(px(6. * k))
                        .when(!ruled, |d| d.py(px(5. * k)))
                        .when(ruled, |d| d.h(px(lh * k)).items_center())
                        .when(head_row && !ruled, |d| d.pt_0())
                        .when(head_row, |d| {
                            d
                                .border_b_1()
                                .border_color(t.colors.rule)
                                .text_size(px(10. * k))
                                .font_weight(gpui::FontWeight::BOLD)
                                .text_color(t.colors.text_body)
                        })
                        .when(!head_row && i < last, |d| d.border_b_1().border_dashed().border_color(t.colors.rule))
                        .when(i == last && !head_row && !ruled, |d| d.pb_0())
                        .children(cells.into_iter().enumerate().map(|(c, cell)| {
                            let plain = !cell.contains(['*', '`', '<', '[', '~', '_']);
                            div()
                                .flex_1()
                                .min_w_0()
                                .when(c == 0 && !head_row, |d| d.font_weight(gpui::FontWeight::SEMIBOLD).text_color(t.colors.text))
                                .map(|d| match (head_row, plain) {
                                    (true, _) => d.child(cell.to_uppercase()),
                                    (false, true) => d.child(cell),
                                    (false, false) => d.child(md_view(next_id(), &cell, t)),
                                })
                        }))
                        .into_any_element()
                };
                let mut out = vec![row(head, 0)];
                for (i, r) in rows.into_iter().enumerate() {
                    out.push(row(r, i + 1));
                }
                body.push(
                    div()
                        .flex()
                        .flex_col()
                        .text_size(px(12. * k))
                        .line_height(px(16. * k))
                        .text_color(t.colors.text_body)
                        .children(out)
                        .into_any_element(),
                );
            }
        }
    }
    NoteParts { title, body }
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
        let style = s.style;
        let (rt, rb) = radii(style);
        let (rt, rb) = (rt * z, rb * z);
        let w = r.w * z;

        // What reaches a corner rounds it itself: the note clips its
        // contents to a rectangle, inside its 1px edge.
        let (it, ib) = ((rt - 1.).max(0.), (rb - 1.).max(0.));
        let picture = s.image.as_ref().map(|name| {
            let has_text = !s.title.is_empty() || !s.body.is_empty();
            // The id gives the picture frame state, so an animated GIF plays.
            img(self.image_path(name))
                .id(("picture", s.id))
                .w_full()
                .rounded_t(px(it))
                .when(has_text || !s.fixed, |i| i.h(px((s.rect.w * 0.62).min(s.rect.h) * z)))
                .when(!has_text && s.fixed, |i| i.h(px(s.rect.h * z)).rounded_b(px(ib)))
                .object_fit(ObjectFit::Cover)
                .flex_none()
                // A picture just added fades in.
                .opacity(self.exits.notes.picture_showing(s.id, &t.motion).unwrap_or(1.))
        });

        let id = s.id;
        let me = cx.entity();
        let toggle = move |n: usize, _: &mut Window, cx: &mut App| me.update(cx, |f, cx| f.toggle_task(id, n, cx));
        let parts = if detail {
            // The one selected note's text can be selected: a first click
            // picks the note, a press on its text then picks words.
            let reading = selected && self.selection.len() == 1 && !matches!(self.gesture, Gesture::Move { moved: true, .. });
            note_body(s, k, t, &crate::store::media_dir(&self.dir), None, reading, toggle)
        } else {
            let title = div()
                .text_size(px(15. / DETAIL_ZOOM * z))
                .font_weight(gpui::FontWeight::SEMIBOLD)
                .text_color(t.colors.text)
                .child(first_line(s))
                .into_any_element();
            NoteParts { title: Some(title), body: Vec::new() }
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
        let chips = (!chips.is_empty()).then(|| {
            let row = div().flex().flex_wrap().items_center().gap(px(4. * z)).children(chips);
            // On an index card the chips take a ruled line of their own.
            if style == NoteStyle::Index {
                row.min_h(px(24. * k))
            } else {
                row.pt(px(2. * z))
            }
        });
        let measured = self.heights.clone();
        let natural = self.heights.borrow().get(&s.id).copied().unwrap_or(0.);
        let cut = s.fixed && natural > s.rect.h + 1.;
        let lines = ((natural - s.rect.h) / (line_height(style) * size_factor(s.text_size))).ceil().max(1.) as usize;
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

        // The text, laid out the way this type of note is made.
        let NoteParts { title, body } = parts;
        let text = match style {
            NoteStyle::Card | NoteStyle::Taped => div()
                .px(px(16. * z))
                .pt(px(if style == NoteStyle::Taped { 18. } else { 14. } * z))
                .pb(px(14. * z))
                .flex()
                .flex_col()
                .gap(px(8. * z))
                .children(title)
                .children(body)
                .children(chips),
            NoteStyle::Cheat => div().flex().flex_col().child(band(title, s.paper, z, t).when(s.image.is_none(), |d| d.rounded_t(px(it)))).child(
                div().px(px(14. * z)).pt(px(10. * z)).pb(px(14. * z)).flex().flex_col().gap(px(8. * z)).children(body).children(chips),
            ),
            NoteStyle::Index => {
                let ruled = t.colors.ruled;
                let step = 24. * k;
                let rules = detail.then(|| {
                    canvas(
                        |_, _, _| {},
                        move |b, _, window, _| {
                            let mut y = b.origin.y + px(step);
                            while y <= b.origin.y + b.size.height {
                                let line = gpui::Bounds::new(gpui::point(b.origin.x, y - px(1.)), gpui::size(b.size.width, px(1.)));
                                window.paint_quad(gpui::fill(line, ruled));
                                y += px(step);
                            }
                        },
                    )
                    .absolute()
                    .inset_0()
                });
                div()
                    .relative()
                    .flex()
                    .flex_col()
                    .child(div().absolute().left(px(30. * z)).top_0().bottom_0().w(px(1.)).bg(t.colors.margin))
                    .child(
                        div()
                            .pl(px(40. * z))
                            .pr(px(16. * z))
                            .pt(px(12. * z))
                            .pb(px(4. * z))
                            .min_h(px(16. * z))
                            .border_b_1()
                            .border_color(t.colors.head_rule)
                            .children(title),
                    )
                    .child(
                        div()
                            .relative()
                            .pl(px(40. * z))
                            .pr(px(14. * z))
                            .pb(px(10. * z))
                            .min_h(px(24. * k))
                            .flex()
                            .flex_col()
                            .children(rules)
                            .children(body)
                            .children(chips),
                    )
            }
        };

        let inner = div().relative().flex_none().flex().flex_col().children(picture).child(text).child(measure);

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
        let shadow = if lifted {
            t.shadow_lifted(z)
        } else {
            match style {
                NoteStyle::Taped => t.shadow_taped(z),
                NoteStyle::Index => t.shadow_index(z),
                _ => t.shadow_card(z),
            }
        };
        let edge = if t.is_dark { t.colors.border } else { gpui::hsla(0., 0., 0., 0.04) };
        let paper = t.paper(s.paper);

        // A faint sheen, lighter at the top, so the paper isn't flat; index
        // cards are flat card stock.
        let mut top = paper;
        if style != NoteStyle::Index {
            top.l = (top.l + if t.is_dark { 0.03 } else { 0.022 }).min(1.);
        }
        // Where taped paper lifts a little at its foot.
        let curl = (style == NoteStyle::Taped).then(|| {
            div()
                .absolute()
                .left_0()
                .right_0()
                .bottom_0()
                .h(px(28. * z))
                .rounded_b(px(rb))
                .bg(gpui::linear_gradient(180., gpui::linear_color_stop(t.colors.curl.opacity(0.), 0.), gpui::linear_color_stop(t.colors.curl, 1.)))
        });
        div()
            .absolute()
            .left(px(at.x))
            .top(px(at.y + lift + settle))
            .w(px(w))
            .when(s.fixed, |d| d.h(px(s.rect.h * z)))
            // Grows with its text: the measured height, once known.
            .when(!s.fixed, |d| d.min_h(px(r.h * z)))
            .rounded_t(px(rt))
            .rounded_b(px(rb))
            .bg(gpui::linear_gradient(180., gpui::linear_color_stop(top, 0.), gpui::linear_color_stop(paper, 0.6)))
            .border_1()
            .border_color(edge)
            .shadow(shadow)
            .font_family(t.note_font(s.font))
            .child(div().size_full().rounded_t(px(rt)).rounded_b(px(rb)).overflow_hidden().children(curl).child(inner))
            .when(cut, |d| {
                d.child(
                    div()
                        .absolute()
                        .left_0()
                        .right_0()
                        .bottom_0()
                        .h(px(34. * z))
                        .rounded_b(px(rb))
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
            .when(style == NoteStyle::Taped && s.pin.is_none(), |d| d.child(tape(s.id, w, z, t)))
            .children(s.pin.map(|p| super::pins::pin_view(p, w, z, self.exits.notes.pin_landing(s.id, &t.motion), t)))
            .when(s.locked, |d| d.child(div().absolute().top(px(6. * z)).right(px(6. * z)).child(icon("icons/lock.svg", px(12. * z.max(0.8)), t.colors.text_subtle))))
            .when(selected, |d| {
                d.child(
                    div()
                        .absolute()
                        .top(px(-3.))
                        .left(px(-3.))
                        .right(px(-3.))
                        .bottom(px(-3.))
                        .rounded_t(px(rt + 3.))
                        .rounded_b(px(rb + 3.))
                        .border_2()
                        .border_color(t.colors.selection),
                )
            })
            // One of several selected: a tick in its corner, as a file
            // window marks what's picked.
            .when(selected && self.selection.len() > 1, |d| {
                d.child(
                    div()
                        .absolute()
                        .top(-t.size.icon_lg / 2. + px(2.))
                        .left(-t.size.icon_lg / 2. + px(2.))
                        .size(t.size.icon_lg)
                        .flex()
                        .items_center()
                        .justify_center()
                        .rounded(t.radius.full)
                        .bg(t.colors.selection)
                        .border_2()
                        .border_color(t.colors.canvas)
                        .child(icon("icons/check.svg", t.size.icon_sm * 0.8, t.colors.primary_fg)),
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
