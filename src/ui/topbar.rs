//! The bar above the sheet: where you are, search, undo and redo, the
//! sheet's delete, and the one main action, a new note.

use gpui::component::input::Input;
use gpui::{div, prelude::*, Context, Window};

use super::app::Folio;
use super::*;
use crate::theme::theme;

impl Folio {
    pub(super) fn render_topbar(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let t = theme(cx).clone();
        let name = self.sheet().name.clone();
        div()
            .h(t.size.topbar)
            .flex_none()
            .flex()
            .items_center()
            .gap(t.space.md)
            .px(t.space.xxl)
            .bg(t.colors.surface)
            .border_b_1()
            .border_color(t.colors.border)
            .child(div().text_size(t.text.body).text_color(t.colors.text_subtle).child("Sheets"))
            .child(div().text_color(t.colors.border).child("/"))
            .child(div().text_size(t.text.body).font_weight(gpui::FontWeight::SEMIBOLD).child(name))
            .child(div().flex_1())
            .child(
                div()
                    .w(gpui::px(220.))
                    .h(t.size.control)
                    .flex()
                    .items_center()
                    .gap(t.space.xs)
                    .pl(t.space.md)
                    .pr(t.space.xs)
                    .rounded(t.radius.button)
                    .border_1()
                    .border_color(t.colors.border)
                    .bg(t.colors.surface_sunk)
                    .child(icon("icons/search.svg", t.size.icon_sm, t.colors.text_subtle))
                    .child(div().flex_1().child(Input::new(&self.search_input).appearance(false).cleanable(true)))
                    .child(kbd(shortcut("F"), &t)),
            )
            .child(
                icon_button("undo", "icons/undo-2.svg", &t)
                    .on_click(cx.listener(|this, _, _, cx| this.undo(cx))),
            )
            .child(
                icon_button("redo", "icons/redo-2.svg", &t)
                    .on_click(cx.listener(|this, _, _, cx| this.redo(cx))),
            )
            .child(
                icon_button("delete-sheet", "icons/trash.svg", &t)
                    .on_click(cx.listener(|this, _, _, cx| this.delete_sheet(cx))),
            )
            .child(
                primary_button("new-note", "New note", &t)
                    .on_click(cx.listener(|this, _, window, cx| this.new_sticker(window, cx))),
            )
    }
}
