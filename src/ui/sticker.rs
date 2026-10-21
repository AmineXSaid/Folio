//! One sticker: its paper, an optional picture, a title, and Markdown.
//! Laid out at the camera's zoom; when zoomed far out only the title shows,
//! which keeps a crowded sheet legible and cheap to draw.

use gpui::component::text::{TextView, TextViewStyle};
use gpui::{div, img, prelude::*, px, AnyElement, Context, ObjectFit, StyledImage};

use super::app::{Folio, Gesture, Selection};
use crate::model::*;
use crate::theme::Theme;

/// Below this zoom, a sticker shows its title only.
const DETAIL_ZOOM: f32 = 0.45;

impl Folio {
    pub(super) fn render_sticker(&self, s: &Sticker, cam: Camera, dim: bool, t: &Theme, _cx: &mut Context<Self>) -> AnyElement {
        let z = cam.zoom;
        let at = cam.to_screen(v2(s.rect.x, s.rect.y));
        let selected = self.selected == Some(Selection::Sticker(s.id));
        let lifted = matches!(self.gesture, Gesture::Move { id, moved: true, .. } if id == s.id);
        let hovered = self.hovered == Some(s.id);
        let scale = match s.text_size {
            TextSize::Small => 0.9,
            TextSize::Medium => 1.0,
            TextSize::Large => 1.2,
        };
        let pad = px(16. * z);
        let detail = z >= DETAIL_ZOOM;

        let picture = s.image.as_ref().map(|name| {
            let has_text = !s.title.is_empty() || !s.body.is_empty();
            // The id gives the picture frame state, so an animated GIF plays.
            img(self.image_path(name))
                .id(("picture", s.id))
                .w_full()
                .when(has_text, |i| i.h(px(s.rect.h * 0.55 * z)))
                .when(!has_text, |i| i.h_full())
                .object_fit(ObjectFit::Cover)
                .flex_none()
        });

        let title = (!s.title.is_empty()).then(|| {
            div()
                .text_size(px(15. * scale * z))
                .font_weight(gpui::FontWeight::SEMIBOLD)
                .text_color(t.colors.text)
                .line_height(px(20. * scale * z))
                .child(s.title.clone())
        });

        let body = (detail && !s.body.is_empty()).then(|| {
            div()
                .text_size(px(13. * scale * z))
                .line_height(px(20. * scale * z))
                .text_color(t.colors.text_body)
                .child(
                    TextView::markdown(("md", s.id), tint_marks(&s.body, t.is_dark))
                        .selectable(false)
                        .style(markdown_style(t)),
                )
        });

        let grip = (selected || hovered).then(|| {
            let g = t.size.grip;
            div()
                .absolute()
                .right(px(4.))
                .bottom(px(4.))
                .size(g)
                .child(
                    div()
                        .absolute()
                        .right_0()
                        .bottom_0()
                        .w(g)
                        .h(px(2.))
                        .rounded(px(1.))
                        .bg(t.colors.text_subtle),
                )
                .child(
                    div()
                        .absolute()
                        .right_0()
                        .bottom_0()
                        .w(px(2.))
                        .h(g)
                        .rounded(px(1.))
                        .bg(t.colors.text_subtle),
                )
        });

        let shadow = if lifted { t.shadow_lifted(z) } else { t.shadow_card(z) };
        let edge = if t.is_dark { t.colors.border } else { gpui::hsla(0., 0., 0., 0.04) };

        div()
            .absolute()
            .left(px(at.x))
            .top(px(at.y))
            .w(px(s.rect.w * z))
            .h(px(s.rect.h * z))
            .rounded(px(12. * z))
            .bg(t.paper(s.paper))
            .border_1()
            .border_color(edge)
            .shadow(shadow)
            .when(dim, |d| d.opacity(0.3))
            .child(
                div()
                    .size_full()
                    .rounded(px(12. * z))
                    .overflow_hidden()
                    .flex()
                    .flex_col()
                    .children(picture)
                    .child(
                        div()
                            .flex_1()
                            .min_h_0()
                            .overflow_hidden()
                            .px(pad)
                            .py(px(14. * z))
                            .flex()
                            .flex_col()
                            .gap(px(8. * z))
                            .children(title)
                            .children(body),
                    ),
            )
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
