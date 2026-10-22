//! The bar across the top: the sheets button (opens the drawer), the logo
//! and where you are; the search box in the middle; on the right the
//! screen's main action.

use gpui::component::input::Input;
use gpui::{div, img, prelude::*, px, Context, Window};

use super::app::{Folio, Screen};
use super::*;
use crate::theme::theme;

impl Folio {
    pub(super) fn render_topbar(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let t = theme(cx).clone();
        let here: Option<String> = match self.screen {
            Screen::Home => None,
            Screen::Sheet => {
                let c = self.sheet();
                Some(if c.icon.is_empty() { c.name.clone() } else { format!("{} {}", c.icon, c.name) })
            }
            Screen::Settings => Some("Settings".into()),
            Screen::Bin => Some("Trash & archive".into()),
        };
        let on_sheet = self.screen == Screen::Sheet;
        let theme_icon = if t.is_dark { "icons/sun.svg" } else { "icons/moon.svg" };

        let left = div()
            .flex()
            .items_center()
            .gap(t.space.sm)
            .min_w_0()
            .child(
                // Opens the sheets drawer: a stack of sheets and a label.
                div()
                    .id("drawer")
                    .h(t.size.control)
                    .pl(t.space.sm)
                    .pr(t.space.md)
                    .flex()
                    .items_center()
                    .gap(t.space.sm)
                    .rounded(t.radius.button)
                    .border_1()
                    .border_color(t.colors.border)
                    .bg(t.colors.surface_sunk)
                    .cursor_pointer()
                    .hover(|d| d.bg(t.colors.hover).border_color(t.colors.text_subtle))
                    .child(icon("icons/layers.svg", t.size.icon, t.colors.text))
                    .child(div().text_size(t.text.caption).font_weight(gpui::FontWeight::MEDIUM).child("Sheets"))
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.finish_edit(cx);
                        this.drawer = !this.drawer;
                        cx.notify();
                    })),
            )
            .child(
                div()
                    .id("home-mark")
                    .flex()
                    .items_center()
                    .gap(t.space.sm)
                    .cursor_pointer()
                    .child(img("brand/folio-64.png").size(px(28.)).flex_none())
                    .child(
                        div()
                            .flex()
                            .text_size(t.text.brand)
                            .font_weight(gpui::FontWeight::BOLD)
                            .child("folio")
                            .child(div().text_color(t.colors.brand).child(".")),
                    )
                    .on_click(cx.listener(|this, _, _, cx| this.go_home(cx))),
            )
            .children(here.map(|name| {
                div()
                    .flex()
                    .items_center()
                    .gap(t.space.sm)
                    .min_w_0()
                    .text_size(t.text.body)
                    .child(div().text_color(t.colors.border).child("/"))
                    .child(div().font_weight(gpui::FontWeight::SEMIBOLD).truncate().child(name))
            }));

        let search = div()
            .absolute()
            .left_0()
            .right_0()
            .top_0()
            .bottom_0()
            .flex()
            .items_center()
            .justify_center()
            .child(
                div()
                    .w(px(480.))
                    .h(px(38.))
                    .flex()
                    .items_center()
                    .gap(t.space.sm)
                    .pl(t.space.md)
                    .pr(t.space.xs)
                    .rounded(t.radius.card)
                    .border_1()
                    .border_color(t.colors.border)
                    .bg(t.colors.surface_sunk)
                    .child(icon("icons/search.svg", t.size.icon_sm, t.colors.text_subtle))
                    .child(div().flex_1().child(Input::new(&self.search_input).appearance(false).cleanable(true)))
                    .child(kbd(shortcut("F"), &t)),
            );

        let right = div()
            .flex()
            .items_center()
            .gap(t.space.xs)
            .when(on_sheet, |d| {
                d.child(icon_button("undo", "icons/undo-2.svg", &t).on_click(cx.listener(|this, _, _, cx| this.undo(cx))))
                    .child(icon_button("redo", "icons/redo-2.svg", &t).on_click(cx.listener(|this, _, _, cx| this.redo(cx))))
            })
            .child(
                icon_button("palette", "icons/command.svg", &t)
                    .on_click(cx.listener(|this, _, window, cx| this.open_palette(window, cx))),
            )
            .child(icon_button("theme", theme_icon, &t).on_click(cx.listener(|this, _, _, cx| this.toggle_theme(cx))))
            .child(if on_sheet {
                primary_button("new-note", "New note", &t)
                    .on_click(cx.listener(|this, _, window, cx| this.new_note(window, cx)))
                    .into_any_element()
            } else {
                primary_button("new-sheet", "New sheet", &t)
                    .on_click(cx.listener(|this, _, window, cx| this.new_sheet_dialog(window, cx)))
                    .into_any_element()
            });

        div()
            .h(t.size.topbar)
            .flex_none()
            .relative()
            .flex()
            .items_center()
            .px(t.space.lg)
            .bg(t.colors.surface)
            .border_b_1()
            .border_color(t.colors.border)
            .child(search)
            .child(left)
            .child(div().flex_1())
            .child(right)
    }
}
