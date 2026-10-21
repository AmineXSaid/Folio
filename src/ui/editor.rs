//! The note editor, over a scrim: title, Markdown body, paper, type size
//! and picture for a sticker; text and ink for a label on the sheet.

use gpui::component::input::{Input, Textarea};
use gpui::{div, img, prelude::*, px, Context, MouseButton, ObjectFit, StyledImage, Window};

use super::app::{Folio, Selection};
use super::*;
use crate::model::{Ink, Paper, TextSize};
use crate::theme::{theme, Theme};

fn field_label(text: &'static str, t: &Theme) -> gpui::Div {
    div()
        .text_size(t.text.micro)
        .font_weight(gpui::FontWeight::MEDIUM)
        .text_color(t.colors.text_muted)
        .child(text)
}

impl Folio {
    pub(super) fn render_editor(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let t = theme(cx).clone();
        let ed = self.editing.as_ref().unwrap();
        let is_sticker = matches!(ed.target, Selection::Sticker(_));
        let heading = match (is_sticker, ed.is_new) {
            (true, true) => "New note",
            (true, false) => "Edit note",
            (false, true) => "Write on the sheet",
            (false, false) => "Edit text",
        };

        let mut body = div().flex().flex_col().gap(t.space.lg);
        if is_sticker {
            body = body.child(
                div()
                    .flex()
                    .flex_col()
                    .gap(t.space.xs)
                    .child(field_label("Title", &t))
                    .child(Input::new(&self.title_input)),
            );
        }
        body = body.child(
            div()
                .flex()
                .flex_col()
                .gap(t.space.xs)
                .child(
                    div()
                        .flex()
                        .child(field_label(if is_sticker { "Content" } else { "Text" }, &t))
                        .when(is_sticker, |d| {
                            d.child(div().flex_1()).child(
                                div()
                                    .text_size(t.text.micro)
                                    .text_color(t.colors.text_subtle)
                                    .child("Markdown · tables · - [ ] tasks · <mark color=\"yellow\">"),
                            )
                        }),
                )
                .child(Textarea::new(&self.body_input)),
        );

        if is_sticker {
            let papers = Paper::ALL.iter().map(|&p| {
                let on = ed.paper == p;
                div()
                    .id(("paper", p as usize))
                    .size(t.size.swatch)
                    .rounded(t.radius.small)
                    .bg(t.paper(p))
                    .border_1()
                    .border_color(t.colors.border)
                    .cursor_pointer()
                    .when(on, |d| d.border_2().border_color(t.colors.primary))
                    .on_click(cx.listener(move |this, _, _, cx| {
                        if let Some(ed) = this.editing.as_mut() {
                            ed.paper = p;
                        }
                        cx.notify();
                    }))
            });
            let sizes = TextSize::ALL.iter().map(|&s| {
                let on = ed.text_size == s;
                div()
                    .id(("size", s as usize))
                    .w(px(32.))
                    .h(px(26.))
                    .flex()
                    .items_center()
                    .justify_center()
                    .rounded(t.radius.small)
                    .text_size(t.text.caption)
                    .cursor_pointer()
                    .when(on, |d| d.bg(t.colors.surface).shadow(t.shadow_card(0.4)).text_color(t.colors.text))
                    .when(!on, |d| d.text_color(t.colors.text_muted))
                    .child(s.label())
                    .on_click(cx.listener(move |this, _, _, cx| {
                        if let Some(ed) = this.editing.as_mut() {
                            ed.text_size = s;
                        }
                        cx.notify();
                    }))
            });
            body = body.child(
                div()
                    .flex()
                    .gap(t.space.xl)
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap(t.space.xs)
                            .child(field_label("Paper", &t))
                            .child(div().flex().gap(t.space.sm).children(papers)),
                    )
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap(t.space.xs)
                            .child(field_label("Text size", &t))
                            .child(div().flex().p(px(2.)).rounded(t.radius.button).bg(t.colors.hover).children(sizes)),
                    ),
            );
            let picture = match &ed.image {
                Some(name) => div()
                    .flex()
                    .items_center()
                    .gap(t.space.md)
                    .child(img(self.image_path(name)).size(px(48.)).rounded(t.radius.small).object_fit(ObjectFit::Cover))
                    .child(div().flex_1().text_size(t.text.caption).text_color(t.colors.text_muted).child("Picture shown above the text"))
                    .child(secondary_button("change-image", "Replace…", &t).on_click(cx.listener(|this, _, _, cx| this.pick_image(cx))))
                    .child(secondary_button("remove-image", "Remove", &t).on_click(cx.listener(|this, _, _, cx| {
                        if let Some(ed) = this.editing.as_mut() {
                            ed.image = None;
                        }
                        cx.notify();
                    }))),
                None => div().flex().child(
                    secondary_button("add-image", "Add picture…", &t).on_click(cx.listener(|this, _, _, cx| this.pick_image(cx))),
                ),
            };
            body = body.child(div().flex().flex_col().gap(t.space.xs).child(field_label("Picture", &t)).child(picture));
        } else {
            let inks = Ink::ALL.iter().map(|&ink| {
                let on = ed.ink == ink;
                div()
                    .id(("label-ink", ink as usize))
                    .size(px(24.))
                    .flex()
                    .items_center()
                    .justify_center()
                    .rounded(t.radius.full)
                    .cursor_pointer()
                    .when(on, |d| d.border_2().border_color(t.ink(ink)))
                    .child(div().size(px(14.)).rounded(t.radius.full).bg(t.ink(ink)))
                    .on_click(cx.listener(move |this, _, _, cx| {
                        if let Some(ed) = this.editing.as_mut() {
                            ed.ink = ink;
                        }
                        cx.notify();
                    }))
            });
            body = body.child(
                div()
                    .flex()
                    .flex_col()
                    .gap(t.space.xs)
                    .child(field_label("Ink", &t))
                    .child(div().flex().gap(t.space.xs).children(inks)),
            );
        }

        let footer = div()
            .flex()
            .items_center()
            .gap(t.space.sm)
            .mt(t.space.xl)
            .when(!ed.is_new, |d| {
                d.child(
                    div()
                        .id("delete")
                        .h(t.size.control)
                        .px(t.space.md)
                        .flex()
                        .items_center()
                        .rounded(t.radius.button)
                        .text_size(t.text.caption)
                        .text_color(t.colors.danger)
                        .cursor_pointer()
                        .hover(|d| d.bg(t.colors.hover))
                        .child("Delete")
                        .on_click(cx.listener(|this, _, window, cx| this.delete_from_editor(window, cx))),
                )
            })
            .child(div().flex_1())
            .child(kbd(shortcut("Enter"), &t))
            .child(secondary_button("cancel", "Cancel", &t).on_click(cx.listener(|this, _, window, cx| this.cancel_editor(window, cx))))
            .child(
                primary_button("save", if ed.is_new { "Create" } else { "Save" }, &t)
                    .on_click(cx.listener(|this, _, window, cx| this.save_editor(window, cx))),
            );

        div()
            .id("scrim")
            .key_context("Editor")
            .absolute()
            .inset_0()
            .flex()
            .items_center()
            .justify_center()
            .bg(t.colors.scrim)
            .on_action(cx.listener(|this, _: &SaveEditor, window, cx| this.save_editor(window, cx)))
            .on_action(cx.listener(|this, _: &Escape, window, cx| this.cancel_editor(window, cx)))
            .on_mouse_down(MouseButton::Left, cx.listener(|this, _, window, cx| this.cancel_editor(window, cx)))
            .child(
                div()
                    .id("editor")
                    .w(t.size.dialog_w)
                    .p(t.space.xl)
                    .rounded(t.radius.dialog)
                    .bg(t.colors.surface)
                    .border_1()
                    .border_color(t.colors.border)
                    .shadow(t.shadow_float())
                    .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(t.space.sm)
                            .pb(t.space.lg)
                            .child(div().size(px(12.)).rounded(px(3.)).bg(if is_sticker { t.paper(ed.paper) } else { t.ink(ed.ink) }).border_1().border_color(t.colors.border))
                            .child(div().text_size(px(18.)).font_weight(gpui::FontWeight::SEMIBOLD).child(heading)),
                    )
                    .child(body)
                    .child(footer),
            )
    }
}
