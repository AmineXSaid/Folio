//! The sidebar: the logo, the sheets (one per category), and the theme
//! switch. Double-click a sheet to rename it.

use gpui::component::input::Input;
use gpui::{div, img, prelude::*, px, Context, MouseButton, Window};

use super::app::Folio;
use super::*;
use crate::theme::theme;

impl Folio {
    pub(super) fn render_sidebar(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let t = theme(cx).clone();
        let active = self.board.active;
        let rows: Vec<_> = self
            .board
            .categories
            .iter()
            .enumerate()
            .map(|(ix, c)| {
                let is_active = ix == active;
                let renaming = self.renaming == Some(c.id);
                let id = c.id;
                let row = div()
                    .id(("sheet", id))
                    .h(px(36.))
                    .px(t.space.md)
                    .flex()
                    .items_center()
                    .gap(t.space.md)
                    .rounded(t.radius.button)
                    .cursor_pointer()
                    .text_size(t.text.body)
                    .when(is_active, |d| {
                        d.bg(t.colors.primary_tint)
                            .text_color(t.colors.primary_on_tint)
                            .font_weight(gpui::FontWeight::SEMIBOLD)
                    })
                    .when(!is_active, |d| d.text_color(t.colors.text_muted).hover(|d| d.bg(t.colors.hover)))
                    .child(div().size(t.size.dot).flex_none().rounded(t.radius.full).bg(t.accent(c.accent)));
                if renaming {
                    row.child(div().flex_1().child(Input::new(&self.rename_input).appearance(false)))
                } else {
                    row.child(div().flex_1().min_w_0().truncate().child(c.name.clone()))
                        .child(
                            div()
                                .text_size(t.text.micro)
                                .text_color(t.colors.text_subtle)
                                .child(c.item_count().to_string()),
                        )
                        .on_mouse_down(
                            MouseButton::Left,
                            cx.listener(move |this, ev: &gpui::MouseDownEvent, window, cx| {
                                if ev.click_count >= 2 {
                                    this.start_rename(id, window, cx);
                                } else {
                                    this.switch_sheet(ix, cx);
                                }
                            }),
                        )
                }
            })
            .collect();

        let theme_icon = if t.is_dark { "icons/sun.svg" } else { "icons/moon.svg" };
        div()
            .w(t.size.sidebar)
            .flex_none()
            .h_full()
            .flex()
            .flex_col()
            .bg(t.colors.surface)
            .border_r_1()
            .border_color(t.colors.border)
            .px(t.space.lg)
            .py(t.space.xl)
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(t.space.md)
                    .px(t.space.sm)
                    .pb(t.space.xl)
                    .child(img("brand/folio-64.png").size(t.size.logo).flex_none())
                    .child(
                        div()
                            .flex()
                            .text_size(t.text.brand)
                            .font_weight(gpui::FontWeight::BOLD)
                            .child("folio")
                            .child(div().text_color(t.colors.text_subtle).font_weight(gpui::FontWeight::NORMAL).child(".")),
                    ),
            )
            .child(
                div()
                    .px(t.space.md)
                    .pb(t.space.sm)
                    .text_size(px(10.))
                    .font_weight(gpui::FontWeight::BOLD)
                    .text_color(t.colors.text_subtle)
                    .child("SHEETS"),
            )
            .child(div().flex().flex_col().gap(t.space.xxs).children(rows))
            .child(
                div()
                    .id("new-sheet")
                    .mt(t.space.xs)
                    .h(px(36.))
                    .px(t.space.md)
                    .flex()
                    .items_center()
                    .gap(t.space.md)
                    .rounded(t.radius.button)
                    .cursor_pointer()
                    .text_color(t.colors.text_subtle)
                    .hover(|d| d.bg(t.colors.hover).text_color(t.colors.text_muted))
                    .child(icon("icons/plus.svg", t.size.icon_sm, t.colors.text_subtle))
                    .child("New sheet")
                    .on_click(cx.listener(|this, _, window, cx| this.new_sheet(window, cx))),
            )
            .child(div().flex_1())
            .child(
                div()
                    .pt(t.space.lg)
                    .border_t_1()
                    .border_color(t.colors.border)
                    .flex()
                    .items_center()
                    .gap(t.space.sm)
                    .px(t.space.sm)
                    .child(
                        div()
                            .flex_1()
                            .flex()
                            .flex_col()
                            .child(div().text_size(t.text.caption).text_color(t.colors.text_muted).child("Personal workspace"))
                            .child(
                                div()
                                    .text_size(t.text.micro)
                                    .text_color(t.colors.text_subtle)
                                    .child(if self.saved { "All changes saved" } else { "Saving…" }),
                            ),
                    )
                    .child(
                        icon_button("theme", theme_icon, &t)
                            .on_click(cx.listener(|this, _, _, cx| this.toggle_theme(cx))),
                    ),
            )
    }
}
