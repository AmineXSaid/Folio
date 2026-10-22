//! Settings: how Folio looks, how notes start, what happens at start-up,
//! the quick-note shortcut, and where your data lives.

use gpui::{div, prelude::*, px, Context, SharedString, Window};

use super::app::{BinTab, Folio};
use super::home::segmented;
use super::*;
use crate::model::*;
use crate::store;
use crate::theme::{theme, Theme};

fn card(title: &'static str, t: &Theme) -> gpui::Div {
    div()
        .w_full()
        .p(t.space.xl)
        .flex()
        .flex_col()
        .gap(t.space.lg)
        .rounded(px(16.))
        .bg(t.colors.surface)
        .border_1()
        .border_color(t.colors.border)
        .child(div().text_size(t.text.title).font_weight(gpui::FontWeight::SEMIBOLD).child(title))
}

fn setting(name: &'static str, help: impl Into<SharedString>, control: impl IntoElement, t: &Theme) -> gpui::Div {
    div()
        .flex()
        .items_center()
        .gap(t.space.xl)
        .child(
            div()
                .flex_1()
                .min_w_0()
                .flex()
                .flex_col()
                .child(div().text_size(t.text.body).child(name))
                .child(div().text_size(t.text.caption).text_color(t.colors.text_subtle).child(help.into())),
        )
        .child(control)
}

/// An on/off switch.
fn switch(id: &'static str, on: bool, t: &Theme) -> gpui::Stateful<gpui::Div> {
    div()
        .id(id)
        .w(px(40.))
        .h(px(24.))
        .flex_none()
        .rounded(t.radius.full)
        .bg(if on { t.colors.brand } else { t.colors.border })
        .cursor_pointer()
        .relative()
        .child(
            div()
                .absolute()
                .top(px(3.))
                .left(px(if on { 19. } else { 3. }))
                .size(px(18.))
                .rounded(t.radius.full)
                .bg(gpui::white())
                .shadow(t.shadow_card(0.3)),
        )
}

impl Folio {
    pub(super) fn render_settings(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let t = theme(cx).clone();
        let s = self.board.settings.clone();
        let l = |f: fn(&mut Settings, usize)| cx.listener(move |this: &mut Folio, ix: &usize, _, cx| this.set_setting(cx, |s| f(s, *ix)));

        let appearance = card("Appearance", &t)
            .child(setting(
                "Theme",
                "Light, dark, or follow Windows",
                segmented(
                    "s-theme",
                    &[("Light", s.theme == ThemeMode::Light), ("Dark", s.theme == ThemeMode::Dark), ("System", s.theme == ThemeMode::System)],
                    &t,
                    l(|s, i| s.theme = [ThemeMode::Light, ThemeMode::Dark, ThemeMode::System][i]),
                ),
                &t,
            ))
            .child(setting(
                "Interface size",
                "Text and controls everywhere",
                segmented(
                    "s-scale",
                    &[("90%", s.ui_scale < 0.95), ("100%", (s.ui_scale - 1.).abs() < 0.05), ("115%", (s.ui_scale - 1.15).abs() < 0.05), ("130%", s.ui_scale > 1.25)],
                    &t,
                    l(|s, i| s.ui_scale = [0.9, 1.0, 1.15, 1.3][i]),
                ),
                &t,
            ))
            .child(setting(
                "High contrast",
                "Darker text and stronger edges",
                switch("s-contrast", s.high_contrast, &t).on_click(cx.listener(|this, _, _, cx| this.set_setting(cx, |s| s.high_contrast = !s.high_contrast))),
                &t,
            ))
            .child(setting(
                "Reduce motion",
                "No opening animation, no sliding panels",
                switch("s-motion", s.reduce_motion, &t).on_click(cx.listener(|this, _, _, cx| this.set_setting(cx, |s| s.reduce_motion = !s.reduce_motion))),
                &t,
            ));

        let notes = card("Notes", &t)
            .child(setting(
                "New notes start on",
                "The paper of a sheet's first note",
                div().flex().gap(px(6.)).children(Paper::ALL.iter().map(|&p| {
                    div()
                        .id(("s-paper", p as usize))
                        .size(px(26.))
                        .rounded(t.radius.small)
                        .bg(t.paper(p))
                        .border_1()
                        .border_color(t.colors.border)
                        .when(s.default_paper == p, |d| d.border_2().border_color(t.colors.brand))
                        .cursor_pointer()
                        .on_click(cx.listener(move |this, _, _, cx| this.set_setting(cx, |s| s.default_paper = p)))
                })),
                &t,
            ))
            .child(setting(
                "Handwriting",
                "The typeface new notes are written in",
                segmented(
                    "s-font",
                    &[("Sans", s.default_font == NoteFont::Sans), ("Hand", s.default_font == NoteFont::Hand), ("Mono", s.default_font == NoteFont::Mono)],
                    &t,
                    l(|s, i| s.default_font = NoteFont::ALL[i]),
                ),
                &t,
            ))
            .child(setting(
                "Alignment guides",
                "Notes snap to each other's edges as you drag",
                switch("s-snap", s.snap, &t).on_click(cx.listener(|this, _, _, cx| this.set_setting(cx, |s| s.snap = !s.snap))),
                &t,
            ))
            .child(setting(
                "Snap to the grid",
                "Dropped notes line up with the sheet's dots",
                switch("s-grid", s.grid, &t).on_click(cx.listener(|this, _, _, cx| this.set_setting(cx, |s| s.grid = !s.grid))),
                &t,
            ))
            .child(setting(
                "Map",
                "A small overview of the sheet in the corner",
                switch("s-map", s.minimap, &t).on_click(cx.listener(|this, _, _, cx| this.set_setting(cx, |s| s.minimap = !s.minimap))),
                &t,
            ));

        let startup = card("Starting up", &t)
            .child(setting(
                "Open on",
                "Where Folio starts",
                segmented(
                    "s-open",
                    &[("All sheets", s.open_on == OpenOn::Home), ("The last sheet", s.open_on == OpenOn::LastSheet)],
                    &t,
                    l(|s, i| s.open_on = [OpenOn::Home, OpenOn::LastSheet][i]),
                ),
                &t,
            ))
            .child(setting(
                "Start with Windows",
                "Folio is ready (in the tray) when you sign in",
                switch("s-start", s.start_with_windows, &t).on_click(cx.listener(|this, _, _, cx| this.set_setting(cx, |s| s.start_with_windows = !s.start_with_windows))),
                &t,
            ))
            .child(setting(
                "Quick note shortcut",
                "Ctrl+Alt+N from any app opens a small note that goes to your Inbox",
                switch("s-hotkey", s.quick_hotkey, &t).on_click(cx.listener(|this, _, _, cx| this.set_setting(cx, |s| s.quick_hotkey = !s.quick_hotkey))),
                &t,
            ));

        let dir = self.dir.clone();
        let days = store::backups(&self.dir).len();
        let data = card("Your data", &t)
            .child(setting(
                "Data folder",
                self.dir.display().to_string(),
                secondary_button("s-open-dir", "Open folder", &t).on_click(move |_, _, cx| cx.reveal_path(&dir.join("board.json"))),
                &t,
            ))
            .child(setting(
                "Backups",
                format!("One a day, the last {} kept · {days} now", store::BACKUPS_KEPT),
                div()
                    .flex()
                    .gap(t.space.sm)
                    .child(secondary_button("s-backups", "See backups", &t).on_click(cx.listener(|this, _, _, cx| this.open_bin(BinTab::Backups, cx))))
                    .child(secondary_button("s-backup-now", "Back up now", &t).on_click(cx.listener(|this, _, _, cx| this.back_up_now(cx)))),
                &t,
            ));

        let keys = card("Keyboard", &t).child(
            div().flex().flex_wrap().gap_x(t.space.xl).gap_y(t.space.sm).children(
                [
                    ("Ctrl+K", "Commands, sheets, notes"),
                    ("Ctrl+Alt+N", "Quick note from anywhere"),
                    ("Ctrl+N / N", "New note"),
                    ("Ctrl+Shift+N", "New sheet"),
                    ("Ctrl+F", "Search"),
                    ("Ctrl+B", "Sheets drawer"),
                    ("Ctrl+Tab", "Next sheet"),
                    ("Enter", "Edit the selected note"),
                    ("Arrows", "Move between notes"),
                    ("Space + drag", "Pan the sheet"),
                    ("V H P M T C F E", "Tools"),
                    ("Ctrl+Z / Ctrl+Shift+Z", "Undo / redo"),
                ]
                .into_iter()
                .map(|(k, what)| {
                    div()
                        .w(px(300.))
                        .flex()
                        .items_center()
                        .gap(t.space.md)
                        .child(div().w(px(150.)).child(kbd(k, &t)))
                        .child(div().text_size(t.text.caption).text_color(t.colors.text_muted).child(what))
                }),
            ),
        );

        div().id("settings").size_full().overflow_y_scroll().child(
            div()
                .max_w(px(760.))
                .mx_auto()
                .px(t.space.xl)
                .py(t.space.xxl)
                .flex()
                .flex_col()
                .gap(t.space.xl)
                .child(div().text_size(px(28.)).font_weight(gpui::FontWeight::SEMIBOLD).child("Settings"))
                .child(appearance)
                .child(notes)
                .child(startup)
                .child(data)
                .child(keys),
        )
    }
}
