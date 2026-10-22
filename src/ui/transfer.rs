//! Sync & export: Google Drive (sign in, where sync is at, what happened
//! lately, and starting over either way), exporting sheets to files, and
//! importing a Folio file.

use std::collections::HashSet;

use gpui::{div, prelude::*, px, Context, Hsla, SharedString, Window};

use super::app::{Dialog, Folio, Screen};
use super::sync::DriveState;
use super::*;
use crate::cloud::Keep;
use crate::export;
use crate::model::*;
use crate::store;
use crate::theme::{theme, Theme};
use crate::time;

/// What sheets go out as.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum ExportFormat {
    #[default]
    Markdown,
    Folio,
}

/// The export card's choices.
#[derive(Default)]
pub struct ExportChoice {
    pub format: ExportFormat,
    /// Sheets left out (new sheets start chosen).
    pub skip: HashSet<Id>,
}

fn card(t: &Theme) -> gpui::Div {
    div()
        .w_full()
        .p(t.space.xl)
        .flex()
        .flex_col()
        .gap(t.space.lg)
        .rounded(t.radius.dialog)
        .bg(t.colors.surface)
        .border_1()
        .border_color(t.colors.border)
}

/// A square tile with an icon: the card's emblem.
fn emblem(path: &'static str, fg: Hsla, bg: Hsla, t: &Theme) -> gpui::Div {
    div()
        .size(px(44.))
        .flex_none()
        .flex()
        .items_center()
        .justify_center()
        .rounded(t.radius.card)
        .bg(bg)
        .child(icon(path, t.size.icon_lg, fg))
}

fn heading(title: &'static str, sub: impl Into<SharedString>, t: &Theme) -> gpui::Div {
    div()
        .flex_1()
        .min_w_0()
        .flex()
        .flex_col()
        .gap(px(2.))
        .child(div().text_size(t.text.title).font_weight(gpui::FontWeight::SEMIBOLD).child(title))
        .child(div().text_size(t.text.caption).text_color(t.colors.text_muted).child(sub.into()))
}

/// A button with an icon before its label.
fn action(id: &'static str, path: &'static str, label: impl Into<SharedString>, primary: bool, t: &Theme) -> gpui::Stateful<gpui::Div> {
    let (bg, hover, fg) = if primary {
        (t.colors.primary, t.colors.primary_hover, t.colors.primary_fg)
    } else {
        (t.colors.hover, t.colors.border, t.colors.text)
    };
    div()
        .id(id)
        .h(t.size.control)
        .px(t.space.md)
        .flex()
        .flex_none()
        .items_center()
        .gap(t.space.sm)
        .rounded(t.radius.button)
        .bg(bg)
        .hover(move |d| d.bg(hover))
        .text_color(fg)
        .text_size(t.text.caption)
        .font_weight(gpui::FontWeight::SEMIBOLD)
        .cursor_pointer()
        .child(icon(path, t.size.icon_sm, fg))
        .child(label.into())
}

fn small_label(text: &'static str, t: &Theme) -> gpui::Div {
    div().text_size(t.text.micro).font_weight(gpui::FontWeight::SEMIBOLD).text_color(t.colors.text_subtle).child(text)
}

fn bullet(text: &'static str, t: &Theme) -> gpui::Div {
    div()
        .flex()
        .items_center()
        .gap(t.space.sm)
        .text_size(t.text.body)
        .text_color(t.colors.text_body)
        .child(icon("icons/check.svg", t.size.icon_sm, t.colors.ok))
        .child(text)
}

/// A number and what it counts, side by side with others.
fn stat(value: String, what: &'static str, t: &Theme) -> gpui::Div {
    div()
        .flex_1()
        .min_w_0()
        .px(t.space.md)
        .py(t.space.sm)
        .flex()
        .flex_col()
        .rounded(t.radius.button)
        .bg(t.colors.surface_sunk)
        .border_1()
        .border_color(t.colors.border)
        .child(div().text_size(t.text.title).font_weight(gpui::FontWeight::SEMIBOLD).truncate().child(value))
        .child(div().text_size(t.text.micro).text_color(t.colors.text_muted).child(what))
}

impl Folio {
    pub fn open_transfer(&mut self, cx: &mut Context<Self>) {
        self.open_side(Screen::Transfer, cx);
    }

    /// The Sync & export screen with only sheet `ix` chosen for export.
    pub fn export_sheet(&mut self, ix: usize, cx: &mut Context<Self>) {
        let Some(id) = self.board.categories.get(ix).map(|c| c.id) else { return };
        self.export.skip = self.board.categories.iter().map(|c| c.id).filter(|&c| c != id).collect();
        if self.screen != Screen::Transfer {
            self.open_transfer(cx);
        }
    }

    fn chosen_sheets(&self) -> Vec<&Category> {
        self.board.categories.iter().filter(|c| !self.export.skip.contains(&c.id)).collect()
    }

    /// Asks where, then writes the chosen sheets there.
    pub fn export_now(&mut self, cx: &mut Context<Self>) {
        if self.chosen_sheets().is_empty() {
            self.notify("Choose at least one sheet to export", cx);
            return;
        }
        let rx = cx.prompt_for_paths(gpui::PathPromptOptions {
            files: false,
            directories: true,
            multiple: false,
            prompt: Some("Export here".into()),
        });
        cx.spawn(async move |this, cx| {
            let answer = rx.await;
            let _ = this.update(cx, |this, cx| {
                let dest = match answer {
                    Ok(Ok(Some(mut p))) if !p.is_empty() => p.remove(0),
                    // Cancelled.
                    Ok(Ok(_)) => return,
                    // No folder picker on this system: Documents.
                    _ => export_fallback(),
                };
                this.export_to(&dest, cx);
            });
        })
        .detach();
    }

    fn export_to(&mut self, dest: &std::path::Path, cx: &mut Context<Self>) {
        let sheets = self.chosen_sheets();
        let n = sheets.len();
        let media = store::media_dir(&self.dir);
        let stamp = time::today();
        let result = match self.export.format {
            ExportFormat::Markdown => export::export_markdown(&sheets, &media, dest, &format!("Folio {stamp}")),
            ExportFormat::Folio => {
                let base = if n == 1 { export::file_name(&sheets[0].name) } else { format!("Folio {stamp}") };
                let path = (1..).map(|i| if i == 1 { dest.join(format!("{base}.folio")) } else { dest.join(format!("{base} {i}.folio")) }).find(|p| !p.exists()).expect("a free name");
                export::export_folio(&sheets, &media, &path).map(|_| path)
            }
        };
        match result {
            Ok(path) => {
                let what = if n == 1 { "1 sheet".to_string() } else { format!("{n} sheets") };
                let name = path.file_name().map(|f| f.to_string_lossy().to_string()).unwrap_or_default();
                self.notify(format!("Exported {what} · {name}"), cx);
                cx.reveal_path(&path);
            }
            Err(e) => self.notify(format!("Couldn't export: {e}"), cx),
        }
    }

    /// Asks for a .folio file and adds its sheets.
    pub fn import_file(&mut self, cx: &mut Context<Self>) {
        let rx = cx.prompt_for_paths(gpui::PathPromptOptions { files: true, directories: false, multiple: true, prompt: Some("Import".into()) });
        cx.spawn(async move |this, cx| {
            let answer = rx.await;
            let _ = this.update(cx, |this, cx| match answer {
                Ok(Ok(Some(paths))) => {
                    for p in paths {
                        this.import_path(&p, cx);
                    }
                }
                Ok(Ok(None)) => {}
                _ => this.notify("No file picker here · drop the .folio file onto a sheet instead", cx),
            });
        })
        .detach();
    }

    pub fn import_path(&mut self, path: &std::path::Path, cx: &mut Context<Self>) {
        let pkg = match export::read_folio(path) {
            Ok(p) => p,
            Err(e) => {
                self.notify(e, cx);
                return;
            }
        };
        let media = store::media_dir(&self.dir);
        let mut result = Ok(0);
        self.change(cx, |b| result = export::import(b, pkg, &media));
        match result {
            Ok(n) => self.notify(format!("Added {} from {}", if n == 1 { "1 sheet".into() } else { format!("{n} sheets") }, path.file_name().map(|f| f.to_string_lossy().to_string()).unwrap_or_default()), cx),
            Err(e) => self.notify(format!("Couldn't import: {e}"), cx),
        }
    }

    pub(super) fn render_transfer(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let t = theme(cx).clone();
        div().id("transfer").size_full().overflow_y_scroll().child(
            div()
                .max_w(px(760.))
                .mx_auto()
                .px(t.space.xl)
                .py(t.space.xxl)
                .flex()
                .flex_col()
                .gap(t.space.xl)
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .gap(t.space.xs)
                        .child(div().text_size(px(28.)).font_weight(gpui::FontWeight::SEMIBOLD).child("Sync & export"))
                        .child(div().text_color(t.colors.text_muted).child("Keep your sheets in Google Drive on every computer, or take them out as files.")),
                )
                .child(self.drive_card(&t, cx))
                .child(self.export_card(&t, cx))
                .child(self.import_card(&t, cx)),
        )
    }

    fn drive_card(&mut self, t: &Theme, cx: &mut Context<Self>) -> gpui::Div {
        let state = self.sync.state.clone();
        if !self.sync.signed_in() {
            let waiting = state == DriveState::SigningIn;
            let button = if waiting {
                action("t-cancel", "icons/x.svg", "Cancel", false, t).on_click(cx.listener(|this, _, _, cx| this.cancel_sign_in(cx)))
            } else {
                google_button("t-google", t).on_click(cx.listener(|this, _, window, cx| this.sign_in_google(window, cx)))
            };
            return card(t)
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap(t.space.lg)
                        .child(emblem("icons/cloud.svg", t.colors.brand, t.colors.brand_tint, t))
                        .child(heading(
                            "Google Drive",
                            if waiting { "Finish signing in in your browser, then come back here." } else { "Your sheets on every computer you sign in on." },
                            t,
                        ))
                        .child(button),
                )
                .child(
                    div()
                        .pl(px(60.))
                        .flex()
                        .flex_col()
                        .gap(t.space.sm)
                        .child(bullet("Work on two computers: changes from both are merged, never overwritten", t))
                        .child(bullet("Pictures and pinned notes come along", t))
                        .child(bullet("Folio sees only its own folder, nothing else in your Drive", t)),
                );
        }

        let email = self.sync.email.clone().unwrap_or_default();
        let initial: String = email.chars().next().map(|c| c.to_uppercase().collect()).unwrap_or_else(|| "?".into());
        let (dot, line) = match &state {
            DriveState::Syncing => (t.colors.brand, "Syncing…".to_string()),
            DriveState::Error(_) => (t.colors.danger, self.drive_status()),
            DriveState::Choice => (t.colors.alert, "Waiting for you · both have notes".to_string()),
            _ => (t.colors.ok, self.drive_status()),
        };
        let account = div()
            .flex()
            .items_center()
            .gap(t.space.lg)
            .child(
                div()
                    .size(px(44.))
                    .flex_none()
                    .flex()
                    .items_center()
                    .justify_center()
                    .rounded(t.radius.full)
                    .bg(t.colors.brand)
                    .text_color(t.colors.primary_fg)
                    .text_size(px(18.))
                    .font_weight(gpui::FontWeight::BOLD)
                    .child(initial),
            )
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .flex()
                    .flex_col()
                    .gap(px(2.))
                    .child(div().text_size(t.text.title).font_weight(gpui::FontWeight::SEMIBOLD).truncate().child(email))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(t.space.sm)
                            .text_size(t.text.caption)
                            .text_color(t.colors.text_muted)
                            .child(div().size(t.size.dot).flex_none().rounded(t.radius.full).bg(dot))
                            .child(div().truncate().child(line)),
                    ),
            )
            .child(if state == DriveState::Choice {
                action("t-choose", "icons/git-merge.svg", "Choose…", true, t).on_click(cx.listener(|this, _, _, cx| this.drive_show_choice(cx)))
            } else {
                action("t-sync", "icons/refresh-cw.svg", if state == DriveState::Syncing { "Syncing…" } else { "Sync now" }, false, t)
                    .on_click(cx.listener(|this, _, _, cx| this.drive_sync(None, cx)))
            })
            .child(action("t-out", "icons/log-out.svg", "Sign out", false, t).on_click(cx.listener(|this, _, _, cx| this.sign_out_google(cx))));

        let notes: usize = self.board.categories.iter().map(|c| c.stickers.len()).sum();
        let pictures = std::fs::read_dir(store::media_dir(&self.dir)).map(|r| r.flatten().count()).unwrap_or(0);
        let stats = div()
            .flex()
            .gap(t.space.sm)
            .child(stat(self.board.categories.len().to_string(), "sheets", t))
            .child(stat(notes.to_string(), "notes", t))
            .child(stat(pictures.to_string(), "pictures", t));

        let mut activity = div().flex().flex_col();
        let now = time::now();
        let log = self.sync.log.clone();
        if log.is_empty() {
            activity = activity.child(div().py(t.space.sm).text_size(t.text.caption).text_color(t.colors.text_subtle).child(
                "Folio syncs a few seconds after each change and once a minute. Merges and anything that needs you show up here.",
            ));
        }
        for (i, (at, what)) in log.into_iter().enumerate() {
            activity = activity.child(
                div()
                    .flex()
                    .items_center()
                    .gap(t.space.md)
                    .py(t.space.sm)
                    .when(i > 0, |d| d.border_t_1().border_color(t.colors.border))
                    .child(icon(
                        if what.starts_with("Merged") { "icons/git-merge.svg" } else { "icons/cloud.svg" },
                        t.size.icon_sm,
                        t.colors.text_subtle,
                    ))
                    .child(div().flex_1().min_w_0().text_size(t.text.body).truncate().child(what))
                    .child(div().flex_none().text_size(t.text.caption).text_color(t.colors.text_subtle).child(time::ago(at, now))),
            );
        }

        let over = div()
            .flex()
            .flex_col()
            .gap(t.space.sm)
            .pt(t.space.lg)
            .border_t_1()
            .border_color(t.colors.border)
            .child(small_label("START OVER", t))
            .child(div().text_size(t.text.caption).text_color(t.colors.text_muted).child(
                "Folio merges both ways by itself. Use these only to make one side an exact copy of the other.",
            ))
            .child(
                div()
                    .flex()
                    .flex_wrap()
                    .gap(t.space.sm)
                    .child(
                        action("t-push-all", "icons/cloud-upload.svg", "Send everything from here", false, t)
                            .on_click(cx.listener(|this, _, _, cx| this.confirm_replace(Keep::ThisComputer, cx))),
                    )
                    .child(
                        action("t-pull-all", "icons/cloud-download.svg", "Bring everything from Drive", false, t)
                            .on_click(cx.listener(|this, _, _, cx| this.confirm_replace(Keep::Drive, cx))),
                    ),
            );

        card(t)
            .child(account)
            .child(stats)
            .child(div().flex().flex_col().gap(t.space.xs).child(small_label("RECENT ACTIVITY", t)).child(activity))
            .child(over)
    }

    fn confirm_replace(&mut self, keep: Keep, cx: &mut Context<Self>) {
        self.dialog = Some(Dialog::DriveReplace(keep));
        cx.notify();
    }

    fn export_card(&mut self, t: &Theme, cx: &mut Context<Self>) -> gpui::Div {
        let format = self.export.format;
        let tile = |f: ExportFormat, path: &'static str, name: &'static str, help: &'static str, cx: &mut Context<Self>| {
            let on = f == format;
            div()
                .id(("t-format", f as usize))
                .flex_1()
                .min_w_0()
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
                        .child(icon(path, t.size.icon, if on { t.colors.primary_fg } else { t.colors.text_muted })),
                )
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .flex()
                        .flex_col()
                        .gap(px(2.))
                        .child(div().text_size(t.text.body).font_weight(gpui::FontWeight::SEMIBOLD).child(name))
                        .child(div().text_size(t.text.caption).text_color(t.colors.text_muted).child(help)),
                )
                .on_click(cx.listener(move |this, _, _, cx| {
                    this.export.format = f;
                    cx.notify();
                }))
        };

        let total = self.board.categories.len();
        let chosen = self.chosen_sheets().len();
        let all = chosen == total;
        let chips = div().flex().flex_wrap().gap(t.space.sm).children(self.board.categories.iter().map(|c| {
            let on = !self.export.skip.contains(&c.id);
            let id = c.id;
            div()
                .id(("t-sheet", id as usize))
                .h(px(36.))
                .pl(t.space.xs)
                .pr(t.space.md)
                .flex()
                .items_center()
                .gap(t.space.sm)
                .rounded(t.radius.button)
                .border_1()
                .border_color(t.colors.border)
                .bg(if on { t.colors.surface } else { t.colors.surface_sunk })
                .hover(|d| d.border_color(t.colors.text_subtle))
                .cursor_pointer()
                .child(div().when(!on, |d| d.opacity(0.5)).child(sheet_badge(c, 26., t)))
                .child(div().text_size(t.text.body).text_color(if on { t.colors.text } else { t.colors.text_muted }).child(c.name.clone()))
                .child(
                    div()
                        .size(px(16.))
                        .flex()
                        .items_center()
                        .justify_center()
                        .rounded(t.radius.small)
                        .border_1()
                        .border_color(if on { t.colors.brand } else { t.colors.border })
                        .when(on, |d| d.bg(t.colors.brand))
                        .when(on, |d| d.child(icon("icons/check.svg", px(12.), t.colors.primary_fg))),
                )
                .on_click(cx.listener(move |this, _, _, cx| {
                    if !this.export.skip.remove(&id) {
                        this.export.skip.insert(id);
                    }
                    cx.notify();
                }))
        }));

        let label = match (chosen, format) {
            (0, _) => "Choose sheets".to_string(),
            (1, ExportFormat::Markdown) => "Export 1 sheet…".to_string(),
            (1, ExportFormat::Folio) => "Save 1 sheet…".to_string(),
            (n, ExportFormat::Markdown) => format!("Export {n} sheets…"),
            (n, ExportFormat::Folio) => format!("Save {n} sheets…"),
        };
        card(t)
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(t.space.lg)
                    .child(emblem("icons/file-down.svg", t.colors.text_muted, t.colors.hover, t))
                    .child(heading("Export to files", "Take sheets out of Folio: to read anywhere, or to keep and share.", t)),
            )
            .child(
                div()
                    .flex()
                    .gap(t.space.sm)
                    .child(tile(ExportFormat::Markdown, "icons/file-text.svg", "Markdown", "A folder with one .md file per sheet and its pictures. Opens in any notes app.", cx))
                    .child(tile(ExportFormat::Folio, "icons/package.svg", "Folio file", "The sheets exactly, pictures inside, in one .folio file to keep or open elsewhere.", cx)),
            )
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap(t.space.sm)
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .child(div().flex_1().child(small_label("SHEETS", t)))
                            .child(
                                div()
                                    .id("t-all")
                                    .text_size(t.text.caption)
                                    .font_weight(gpui::FontWeight::MEDIUM)
                                    .text_color(t.colors.brand_on_tint)
                                    .cursor_pointer()
                                    .hover(|d| d.underline())
                                    .child(if all { "Choose none" } else { "Choose all" })
                                    .on_click(cx.listener(move |this, _, _, cx| {
                                        this.export.skip = if all { this.board.categories.iter().map(|c| c.id).collect() } else { HashSet::new() };
                                        cx.notify();
                                    })),
                            ),
                    )
                    .child(chips),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(t.space.md)
                    .pt(t.space.lg)
                    .border_t_1()
                    .border_color(t.colors.border)
                    .child(div().flex_1().text_size(t.text.caption).text_color(t.colors.text_muted).child(format!("{chosen} of {total} sheets chosen")))
                    .child(action("t-export", "icons/download.svg", label, true, t).when(chosen == 0, |d| d.opacity(0.5)).on_click(cx.listener(|this, _, _, cx| this.export_now(cx)))),
            )
    }

    fn import_card(&mut self, t: &Theme, cx: &mut Context<Self>) -> gpui::Div {
        card(t).child(
            div()
                .flex()
                .items_center()
                .gap(t.space.lg)
                .child(emblem("icons/folder-open.svg", t.colors.text_muted, t.colors.hover, t))
                .child(heading("Import a Folio file", "Its sheets are added beside yours; nothing here is replaced.", t))
                .child(action("t-import", "icons/upload.svg", "Import…", false, t).on_click(cx.listener(|this, _, _, cx| this.import_file(cx)))),
        )
    }
}

/// Where exports go when the system has no folder picker: Documents, or
/// home.
fn export_fallback() -> std::path::PathBuf {
    let home = std::env::var_os("USERPROFILE").or_else(|| std::env::var_os("HOME")).map(std::path::PathBuf::from).unwrap_or_else(|| ".".into());
    let docs = home.join("Documents");
    if docs.is_dir() {
        docs
    } else {
        home
    }
}
