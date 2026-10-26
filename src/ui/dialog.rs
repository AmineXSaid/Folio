//! Dialogs over a scrim: a new sheet (name, icon, colour, template),
//! renaming a sheet or an area, and writing text on the sheet.

use gpui::component::input::Input;
use gpui::{div, prelude::*, px, Context, MouseButton, Window};

use super::app::{Dialog, Folio};
use super::sync::Pick;
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
    /// The open dialog: the backdrop dims in, the dialog rises into place.
    /// A click on the backdrop cancels it.
    pub(super) fn render_dialog(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let t = theme(cx).clone();
        let Some(d) = self.dialog else { return div().into_any_element() };
        // Each kind of dialog arrives on its own.
        let key = format!("dialog-{:?}", std::mem::discriminant(&d));
        let scrim = div()
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
            .child(super::motion::enter(
                self.dialog_box(d, cx),
                gpui::ElementId::Name(format!("{key}-box").into()),
                t.motion.dialog_in,
                t.motion.rise_dialog,
            ));
        super::motion::scrim(scrim, gpui::ElementId::Name(key.into()), t.colors.scrim, &t)
    }

    /// A dialog that just closed: it sinks away as the backdrop lifts.
    pub(super) fn render_dialog_leaving(&mut self, cx: &mut Context<Self>) -> Option<gpui::AnyElement> {
        let t = theme(cx).clone();
        let (d, n) = self.exits.dialog.leaving(t.motion.dialog_out)?;
        let ms = t.motion.dialog_out;
        let backdrop = div().absolute().inset_0().flex().items_center().justify_center().child(super::motion::leave(
            self.dialog_box(d, cx).relative(),
            ("dialog-out", n),
            ms,
            t.motion.sink,
        ));
        Some(super::motion::scrim_out(backdrop, ("dialog-scrim-out", n), t.colors.scrim, ms))
    }

    /// The dialog itself: its heading, what it asks, and its buttons.
    fn dialog_box(&mut self, d: Dialog, cx: &mut Context<Self>) -> gpui::Stateful<gpui::Div> {
        let t = theme(cx).clone();
        let (heading, ok) = match d {
            Dialog::NewSheet { .. } => ("New sheet", "Create sheet"),
            Dialog::RenameSheet(_) => ("Rename sheet", "Save"),
            Dialog::RenameFrame(_) => ("Name this area", "Save"),
            Dialog::ShapeText(_) => ("Words in the shape", "Save"),
            Dialog::DriveChoice(_) => ("Google Drive already has Folio notes", "Continue"),
            Dialog::GoogleSetup => ("Connect Folio to Google", "Save and sign in"),
            Dialog::DriveReplace(Keep::ThisComputer) => ("Replace Google Drive's notes?", "Replace Drive's"),
            Dialog::DriveReplace(Keep::Drive) => ("Replace this computer's notes?", "Replace mine"),
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
            Dialog::DriveChoice(chosen) => {
                let option = |pick: Pick, icon_path: &'static str, name: &'static str, help: &'static str, cx: &mut Context<Self>| {
                    let on = pick == chosen;
                    div()
                        .id(("d-pick", pick as usize))
                        .p(t.space.md)
                        .flex()
                        .gap(t.space.md)
                        .rounded(t.radius.card)
                        .border_1()
                        .border_color(if on { t.colors.brand } else { t.colors.border })
                        .when(on, |d| d.bg(t.colors.brand_tint))
                        .hover(|d| d.border_color(t.colors.brand))
                        .cursor_pointer()
                        .child(
                            div()
                                .size(t.size.control)
                                .flex_none()
                                .flex()
                                .items_center()
                                .justify_center()
                                .rounded(t.radius.button)
                                .bg(if on { t.colors.brand } else { t.colors.hover })
                                .child(icon(icon_path, t.size.icon, if on { t.colors.primary_fg } else { t.colors.text_muted })),
                        )
                        .child(
                            div()
                                .flex_1()
                                .min_w_0()
                                .flex()
                                .flex_col()
                                .gap(px(2.))
                                .child(
                                    div()
                                        .flex()
                                        .items_center()
                                        .gap(t.space.sm)
                                        .child(div().text_size(t.text.body).font_weight(gpui::FontWeight::SEMIBOLD).child(name))
                                        .when(pick == Pick::Merge, |d| {
                                            d.child(
                                                div()
                                                    .px(t.space.sm)
                                                    .rounded(t.radius.full)
                                                    .bg(t.colors.surface)
                                                    .border_1()
                                                    .border_color(t.colors.brand)
                                                    .text_color(t.colors.brand_on_tint)
                                                    .text_size(t.text.micro)
                                                    .font_weight(gpui::FontWeight::SEMIBOLD)
                                                    .child("Recommended"),
                                            )
                                        }),
                                )
                                .child(div().text_size(t.text.caption).text_color(t.colors.text_muted).child(help)),
                        )
                        .child(
                            div()
                                .size(px(18.))
                                .flex_none()
                                .mt(px(2.))
                                .rounded(t.radius.full)
                                .border_2()
                                .border_color(if on { t.colors.brand } else { t.colors.border })
                                .flex()
                                .items_center()
                                .justify_center()
                                .when(on, |d| d.child(div().size(px(8.)).rounded(t.radius.full).bg(t.colors.brand))),
                        )
                        .on_click(cx.listener(move |this, _, _, cx| {
                            this.dialog = Some(Dialog::DriveChoice(pick));
                            cx.notify();
                        }))
                };
                body = body
                    .child(div().text_size(t.text.body).text_color(t.colors.text_muted).child(
                        "Your Drive has notes from another computer, and this computer has its own. How should they come together?",
                    ))
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap(t.space.sm)
                            .child(option(
                                Pick::Merge,
                                "icons/git-merge.svg",
                                "Merge both",
                                "Every sheet and note from here and from Drive, together. A note edited in both keeps its newest edit.",
                                cx,
                            ))
                            .child(option(Pick::Mine, "icons/laptop.svg", "Keep this computer's", "Drive's notes are replaced; a copy goes to Backups.", cx))
                            .child(option(Pick::Drive, "icons/cloud-download.svg", "Use Google Drive's", "This computer's notes are replaced; a copy goes to Backups.", cx)),
                    );
            }
            Dialog::DriveReplace(keep) => {
                let (what, safe) = match keep {
                    Keep::ThisComputer => (
                        "Google Drive will hold exactly what's on this computer. Notes that are only in Drive, from your other computers, will be gone there.",
                        "Drive keeps its earlier versions for 30 days (right-click board.json in Drive › Manage versions).",
                    ),
                    Keep::Drive => (
                        "This computer will hold exactly what's in Google Drive. Notes that are only here will be replaced.",
                        "This computer's notes are saved to Backups first, so you can bring any sheet back.",
                    ),
                };
                body = body
                    .child(div().text_size(t.text.body).child(what))
                    .child(
                        div()
                            .flex()
                            .gap(t.space.sm)
                            .p(t.space.md)
                            .rounded(t.radius.button)
                            .bg(t.colors.hover)
                            .child(icon("icons/circle-check.svg", t.size.icon, t.colors.text_muted))
                            .child(div().flex_1().min_w_0().text_size(t.text.caption).text_color(t.colors.text_muted).child(safe)),
                    )
                    .child(div().text_size(t.text.caption).text_color(t.colors.text_subtle).child(
                        "Usually you don't need this: Folio merges changes from both sides by itself.",
                    ));
            }
            Dialog::GoogleSetup => {
                // Folio signs in as your own Google Cloud app, so your notes
                // only ever go between you and your Drive. Set up once.
                let step = |n: &'static str, text: &'static str| {
                    div()
                        .flex()
                        .gap(t.space.md)
                        .child(
                            div()
                                .size(px(20.))
                                .flex_none()
                                .flex()
                                .items_center()
                                .justify_center()
                                .rounded(t.radius.small)
                                .bg(t.colors.brand_tint)
                                .text_color(t.colors.brand_on_tint)
                                .text_size(t.text.micro)
                                .font_weight(gpui::FontWeight::BOLD)
                                .child(n),
                        )
                        .child(div().flex_1().min_w_0().text_size(t.text.caption).text_color(t.colors.text_body).child(text))
                };
                body = body
                    .child(div().text_size(t.text.body).text_color(t.colors.text_muted).child(
                        "Folio signs in through your own Google Cloud app, so your notes only travel between you and your Drive. It takes a few minutes, once.",
                    ))
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap(t.space.sm)
                            .p(t.space.md)
                            .rounded(t.radius.button)
                            .bg(t.colors.surface_sunk)
                            .border_1()
                            .border_color(t.colors.border)
                            .child(step("1", "In Google Cloud Console, create a project and enable the Google Drive API."))
                            .child(step("2", "OAuth consent screen: External, add the drive.file scope and yourself as a test user."))
                            .child(step("3", "Credentials › Create credentials › OAuth client ID › Desktop app."))
                            .child(step("4", "Paste the client ID and secret below."))
                            .child(
                                div().flex().child(
                                    secondary_button("d-console", "Open Google Cloud Console", &t)
                                        .on_click(|_, _, cx| cx.open_url("https://console.cloud.google.com/apis/credentials")),
                                ),
                            ),
                    )
                    .child(div().flex().flex_col().gap(t.space.xs).child(label("Client ID", &t)).child(Input::new(&self.client_input)))
                    .child(div().flex().flex_col().gap(t.space.xs).child(label("Client secret", &t)).child(Input::new(&self.secret_input)))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(t.space.sm)
                            .text_size(t.text.micro)
                            .text_color(t.colors.text_subtle)
                            .child(icon("icons/circle-check.svg", px(12.), t.colors.ok))
                            .child("Kept on this computer only, in Folio's data folder."),
                    );
            }
            Dialog::RenameSheet(_) | Dialog::RenameFrame(_) | Dialog::ShapeText(_) => {
                body = body.child(Input::new(&self.dialog_input));
            }
        }
        let wide = matches!(d, Dialog::NewSheet { .. } | Dialog::DriveChoice(_) | Dialog::GoogleSetup);
        let footer = div().flex().items_center().gap(t.space.sm).mt(t.space.xl).child(div().flex_1());
        let footer = if let Dialog::DriveChoice(pick) = d {
            footer
                .child(secondary_button("d-later", "Later", &t).on_click(cx.listener(|this, _, window, cx| this.dialog_cancel(window, cx))))
                .child(primary_button("d-pick-go", ok, &t).on_click(cx.listener(move |this, _, window, cx| this.drive_pick(pick, window, cx))))
        } else if let Dialog::DriveReplace(keep) = d {
            footer
                .child(secondary_button("d-cancel", "Cancel", &t).on_click(cx.listener(|this, _, window, cx| this.dialog_cancel(window, cx))))
                .child(primary_button("d-replace", ok, &t).on_click(cx.listener(move |this, _, window, cx| this.drive_replace(keep, window, cx))))
        } else {
            footer
                .child(secondary_button("d-cancel", "Cancel", &t).on_click(cx.listener(|this, _, window, cx| this.dialog_cancel(window, cx))))
                .child(primary_button("d-ok", ok, &t).on_click(cx.listener(|this, _, window, cx| this.dialog_ok(window, cx))))
        };
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
            .child(footer)
    }
}
