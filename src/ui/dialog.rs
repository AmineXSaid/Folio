//! Dialogs over a scrim: a new sheet (name, icon, colour, template),
//! renaming a sheet or an area, and writing text on the sheet.

use gpui::component::input::{Input, Textarea};
use gpui::{div, prelude::*, px, Context, MouseButton, Window};

use super::app::{Dialog, Folio};
use crate::cloud::Keep;
use super::*;
use crate::model::*;
use crate::theme::{theme, Theme};

fn label(text: &'static str, t: &Theme) -> gpui::Div {
    div().text_size(t.text.micro).font_weight(gpui::FontWeight::MEDIUM).text_color(t.colors.text_muted).child(text)
}

fn template_icon(t: Template) -> &'static str {
    match t {
        Template::Blank => "icons/file.svg",
        Template::Kanban => "icons/columns-3.svg",
        Template::Week => "icons/calendar-days.svg",
        Template::Brainstorm => "icons/lightbulb.svg",
        Template::Meeting => "icons/users.svg",
        Template::Reading => "icons/book-open.svg",
    }
}

impl Folio {
    pub(super) fn render_dialog(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let t = theme(cx).clone();
        let Some(d) = self.dialog else { return div().into_any_element() };
        let (heading, ok) = match d {
            Dialog::NewSheet { .. } => ("New sheet", "Create sheet"),
            Dialog::RenameSheet(_) => ("Rename sheet", "Save"),
            Dialog::RenameFrame(_) => ("Name this area", "Save"),
            Dialog::Label { is_new: true, .. } => ("Write on the sheet", "Add"),
            Dialog::Label { .. } => ("Edit text", "Save"),
            Dialog::DriveChoice if self.sync.first => ("Google Drive already has Folio notes", ""),
            Dialog::DriveChoice => ("Your notes changed in two places", ""),
        };
        let mut body = div().flex().flex_col().gap(t.space.lg);
        match d {
            Dialog::NewSheet { icon: chosen, template } => {
                body = body
                    .child(div().flex().flex_col().gap(t.space.xs).child(label("Name", &t)).child(Input::new(&self.dialog_input)))
                    .child(
                        div().flex().flex_col().gap(t.space.xs).child(label("Icon", &t)).child(self.emoji_picker(
                            chosen,
                            &t,
                            cx,
                            |this, e, cx| {
                                if let Some(Dialog::NewSheet { icon, .. }) = this.dialog.as_mut() {
                                    *icon = e;
                                }
                                cx.notify();
                            },
                        )),
                    )
                    .child(
                        div().flex().flex_col().gap(t.space.xs).child(label("Start from", &t)).child(
                            div().flex().flex_wrap().gap(t.space.sm).children(Template::ALL.iter().map(|&tp| {
                                let on = tp == template;
                                div()
                                    .id(("d-template", tp as usize))
                                    .w(px(150.))
                                    .p(t.space.sm)
                                    .flex()
                                    .flex_col()
                                    .gap(px(2.))
                                    .rounded(t.radius.button)
                                    .border_1()
                                    .border_color(if on { t.colors.brand } else { t.colors.border })
                                    .when(on, |d| d.bg(t.colors.brand_tint))
                                    .cursor_pointer()
                                    .child(icon(template_icon(tp), t.size.icon, if on { t.colors.brand_on_tint } else { t.colors.text_muted }))
                                    .child(div().text_size(t.text.caption).font_weight(gpui::FontWeight::SEMIBOLD).child(tp.name()))
                                    .child(div().text_size(t.text.micro).text_color(t.colors.text_subtle).child(tp.blurb()))
                                    .on_click(cx.listener(move |this, _, _, cx| {
                                        if let Some(Dialog::NewSheet { template, .. }) = this.dialog.as_mut() {
                                            *template = tp;
                                        }
                                        cx.notify();
                                    }))
                            })),
                        ),
                    );
            }
            Dialog::DriveChoice => {
                let why = if self.sync.first {
                    "Your Google Drive has notes from another computer, and this computer has its own."
                } else {
                    "Since the last sync, notes were changed both on this computer and in Google Drive."
                };
                body = body
                    .child(div().text_size(t.text.body).child(why))
                    .child(div().text_size(t.text.caption).text_color(t.colors.text_muted).child(
                        "Choose which to keep. The other ones aren't lost: they go to Backups, where you can bring back any sheet.",
                    ));
            }
            Dialog::RenameSheet(_) | Dialog::RenameFrame(_) => {
                body = body.child(Input::new(&self.dialog_input));
            }
            Dialog::Label { ink, .. } => {
                body = body.child(Textarea::new(&self.dialog_body)).child(
                    div().flex().flex_col().gap(t.space.xs).child(label("Ink", &t)).child(
                        div().flex().gap(px(6.)).children(Ink::ALL.iter().map(|&i| {
                            div()
                                .id(("d-ink", i as usize))
                                .size(px(24.))
                                .flex()
                                .items_center()
                                .justify_center()
                                .rounded(t.radius.full)
                                .cursor_pointer()
                                .when(i == ink, |d| d.border_2().border_color(t.ink(i)))
                                .child(div().size(px(14.)).rounded(t.radius.full).bg(t.ink(i)))
                                .on_click(cx.listener(move |this, _, _, cx| {
                                    if let Some(Dialog::Label { ink, .. }) = this.dialog.as_mut() {
                                        *ink = i;
                                    }
                                    cx.notify();
                                }))
                        })),
                    ),
                );
            }
        }
        let wide = matches!(d, Dialog::NewSheet { .. } | Dialog::DriveChoice);
        let footer = div().flex().items_center().gap(t.space.sm).mt(t.space.xl).child(div().flex_1());
        let footer = if d == Dialog::DriveChoice {
            footer
                .child(secondary_button("d-later", "Later", &t).on_click(cx.listener(|this, _, window, cx| this.dialog_cancel(window, cx))))
                .child(secondary_button("d-keep-here", "Keep this computer's", &t).on_click(cx.listener(|this, _, window, cx| this.drive_keep(Keep::ThisComputer, window, cx))))
                .child(primary_button("d-keep-drive", "Use Google Drive's", &t).on_click(cx.listener(|this, _, window, cx| this.drive_keep(Keep::Drive, window, cx))))
        } else {
            footer
                .child(secondary_button("d-cancel", "Cancel", &t).on_click(cx.listener(|this, _, window, cx| this.dialog_cancel(window, cx))))
                .child(primary_button("d-ok", ok, &t).on_click(cx.listener(|this, _, window, cx| this.dialog_ok(window, cx))))
        };
        div()
            .id("scrim")
            .key_context("Dialog")
            .absolute()
            .inset_0()
            .flex()
            .items_center()
            .justify_center()
            .bg(t.colors.scrim)
            .on_action(cx.listener(|this, _: &SaveNote, window, cx| this.dialog_ok(window, cx)))
            .on_mouse_down(MouseButton::Left, cx.listener(|this, _, window, cx| this.dialog_cancel(window, cx)))
            .child(
                div()
                    .id("dialog")
                    .w(if wide { px(520.) } else { t.size.dialog_w })
                    .p(t.space.xl)
                    .rounded(t.radius.dialog)
                    .bg(t.colors.surface)
                    .border_1()
                    .border_color(t.colors.border)
                    .shadow(t.shadow_float())
                    .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                    .child(div().pb(t.space.lg).text_size(px(18.)).font_weight(gpui::FontWeight::SEMIBOLD).child(heading))
                    .child(body)
                    .child(footer),
            )
            .into_any_element()
    }
}
