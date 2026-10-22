//! Ctrl+K: one box for commands, sheets and notes. Type to filter, arrows
//! to choose, Enter to go.

use std::time::Instant;

use gpui::component::input::Input;
use gpui::{div, prelude::*, px, Context, MouseButton, SharedString, Window};

use super::app::{BinTab, Folio, Intro, Screen};
use super::home::first_line;
use super::*;
use crate::model::Id;
use crate::theme::theme;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Cmd {
    NewNote,
    NewSheet,
    QuickNote,
    Home,
    Drawer,
    Settings,
    Trash,
    Archive,
    Backups,
    BackUpNow,
    Theme,
    Fit,
    Tidy,
    Map,
    Intro,
    Drive,
}

const CMDS: [(Cmd, &str, &str, &str); 16] = [
    (Cmd::NewNote, "New note", "Ctrl+N", "icons/sticky-note.svg"),
    (Cmd::NewSheet, "New sheet", "Ctrl+Shift+N", "icons/plus.svg"),
    (Cmd::QuickNote, "Quick note to Inbox", "Ctrl+Alt+N", "icons/inbox.svg"),
    (Cmd::Home, "Go to all sheets", "", "icons/layout-grid.svg"),
    (Cmd::Drawer, "Show the sheets drawer", "Ctrl+B", "icons/panel-left.svg"),
    (Cmd::Settings, "Settings", "Ctrl+,", "icons/settings.svg"),
    (Cmd::Trash, "Open the Trash", "", "icons/trash.svg"),
    (Cmd::Archive, "Open the Archive", "", "icons/archive.svg"),
    (Cmd::Backups, "Restore a backup", "", "icons/rotate-ccw.svg"),
    (Cmd::BackUpNow, "Back up now", "", "icons/hard-drive-download.svg"),
    (Cmd::Drive, "Sync with Google Drive", "", "icons/cloud.svg"),
    (Cmd::Theme, "Switch light / dark", "Ctrl+Shift+L", "icons/moon.svg"),
    (Cmd::Fit, "Zoom to fit", "Ctrl+0", "icons/scan.svg"),
    (Cmd::Tidy, "Tidy up the sheet", "", "icons/wand-sparkles.svg"),
    (Cmd::Map, "Show / hide the map", "", "icons/map.svg"),
    (Cmd::Intro, "Play the opening again", "", "icons/play.svg"),
];

#[derive(Clone, Debug)]
enum Item {
    Cmd(Cmd),
    Sheet(usize),
    Note(Id),
}

impl Folio {
    pub fn open_palette(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.finish_edit(cx);
        self.menu = None;
        self.palette = true;
        self.palette_ix = 0;
        self.palette_input.update(cx, |i, cx| i.set_value("", window, cx));
        let input = self.palette_input.clone();
        window.defer(cx, move |window, cx| input.update(cx, |i, cx| i.focus(window, cx)));
        cx.notify();
    }

    pub fn close_palette(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.palette = false;
        window.focus(&self.focus, cx);
        cx.notify();
    }

    fn palette_items(&self, cx: &gpui::App) -> Vec<(Item, String, String, &'static str)> {
        let q = self.palette_input.read(cx).value().trim().to_lowercase();
        let words: Vec<&str> = q.split_whitespace().collect();
        let fits = |s: &str| {
            let s = s.to_lowercase();
            words.iter().all(|w| s.contains(w))
        };
        let mut out = Vec::new();
        for (c, label, key, path) in CMDS {
            if fits(label) {
                out.push((Item::Cmd(c), label.to_string(), key.to_string(), path));
            }
        }
        for ix in self.board.ordered() {
            let c = &self.board.categories[ix];
            if fits(&c.name) {
                let label = if c.icon.is_empty() { c.name.clone() } else { format!("{} {}", c.icon, c.name) };
                out.push((Item::Sheet(ix), label, format!("{} notes", c.stickers.len()), "icons/layout-grid.svg"));
            }
        }
        if !q.is_empty() {
            for h in self.board.search(&q).into_iter().take(12) {
                let c = &self.board.categories[h.sheet];
                if let Some(s) = c.sticker(h.note) {
                    out.push((Item::Note(h.note), first_line(s), c.name.clone(), "icons/sticky-note.svg"));
                }
            }
        }
        if q.is_empty() {
            out.truncate(14);
        }
        out
    }

    pub fn palette_run(&mut self, ix: usize, window: &mut Window, cx: &mut Context<Self>) {
        let items = self.palette_items(cx);
        let Some((item, ..)) = items.get(ix).cloned() else { return };
        self.palette = false;
        window.focus(&self.focus, cx);
        match item {
            Item::Sheet(i) => self.open_sheet(i, window, cx),
            Item::Note(id) => self.reveal_note(id, window, cx),
            Item::Cmd(c) => match c {
                Cmd::NewNote => self.new_note(window, cx),
                Cmd::NewSheet => self.new_sheet_dialog(window, cx),
                Cmd::QuickNote => crate::desktop::open_quick_note(cx.entity(), cx),
                Cmd::Home => self.go_home(cx),
                Cmd::Drawer => self.drawer = true,
                Cmd::Settings => self.open_settings(cx),
                Cmd::Trash => self.open_bin(BinTab::Trash, cx),
                Cmd::Archive => self.open_bin(BinTab::Archive, cx),
                Cmd::Backups => self.open_bin(BinTab::Backups, cx),
                Cmd::BackUpNow => self.back_up_now(cx),
                Cmd::Drive if self.sync.signed_in() => {
                    if self.sync.state == super::sync::DriveState::Choice {
                        self.drive_show_choice(cx)
                    } else {
                        self.drive_sync(None, cx)
                    }
                }
                Cmd::Drive => self.sign_in_google(cx),
                Cmd::Theme => self.toggle_theme(cx),
                Cmd::Fit | Cmd::Tidy | Cmd::Map => {
                    if self.screen != Screen::Sheet {
                        let a = self.board.active;
                        self.open_sheet(a, window, cx);
                    }
                    match c {
                        Cmd::Fit => self.zoom_to_fit(cx),
                        Cmd::Tidy => self.tidy(cx),
                        _ => self.set_setting(cx, |s| s.minimap = !s.minimap),
                    }
                }
                Cmd::Intro => self.intro = Some(Intro { start: Instant::now() }),
            },
        }
        cx.notify();
    }

    pub(super) fn render_palette(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let t = theme(cx).clone();
        let items = self.palette_items(cx);
        let sel = self.palette_ix.min(items.len().saturating_sub(1));
        let rows = items.into_iter().enumerate().map(|(i, (item, label, hint, path))| {
            let on = i == sel;
            let kind = match item {
                Item::Cmd(_) => "",
                Item::Sheet(_) => "Sheet",
                Item::Note(_) => "Note",
            };
            div()
                .id(("p-row", i))
                .h(px(38.))
                .px(t.space.md)
                .flex()
                .items_center()
                .gap(t.space.md)
                .rounded(t.radius.button)
                .cursor_pointer()
                .when(on, |d| d.bg(t.colors.brand_tint))
                .child(icon(path, t.size.icon, if on { t.colors.brand_on_tint } else { t.colors.text_muted }))
                .child(div().flex_1().min_w_0().truncate().when(on, |d| d.text_color(t.colors.brand_on_tint)).child(label))
                .when(!kind.is_empty(), |d| d.child(div().text_size(t.text.micro).text_color(t.colors.text_subtle).child(SharedString::from(kind))))
                .when(!hint.is_empty(), |d| d.child(kbd(hint, &t)))
                .on_mouse_move(cx.listener(move |this, _, _, cx| {
                    if this.palette_ix != i {
                        this.palette_ix = i;
                        cx.notify();
                    }
                }))
                .on_click(cx.listener(move |this, _, window, cx| this.palette_run(i, window, cx)))
        });
        div()
            .id("palette-scrim")
            .key_context("Palette")
            .absolute()
            .inset_0()
            .bg(t.colors.scrim)
            .flex()
            .justify_center()
            .pt(px(110.))
            .on_action(cx.listener(|this, _: &PaletteUp, _, cx| {
                this.palette_ix = this.palette_ix.saturating_sub(1);
                cx.notify();
            }))
            .on_action(cx.listener(|this, _: &PaletteDown, _, cx| {
                this.palette_ix += 1;
                let n = this.palette_items(cx).len();
                this.palette_ix = this.palette_ix.min(n.saturating_sub(1));
                cx.notify();
            }))
            .on_mouse_down(MouseButton::Left, cx.listener(|this, _, window, cx| this.close_palette(window, cx)))
            .child(
                div()
                    .id("palette")
                    .w(px(600.))
                    .max_h(px(520.))
                    .flex()
                    .flex_col()
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
                            .px(t.space.lg)
                            .h(px(54.))
                            .border_b_1()
                            .border_color(t.colors.border)
                            .child(icon("icons/search.svg", t.size.icon, t.colors.text_subtle))
                            .child(div().flex_1().child(Input::new(&self.palette_input).appearance(false)))
                            .child(kbd("Esc", &t)),
                    )
                    .child(div().id("palette-list").flex_1().min_h_0().overflow_y_scroll().p(t.space.sm).flex().flex_col().children(rows)),
            )
    }
}
