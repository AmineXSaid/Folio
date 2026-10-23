//! The bar across the top, which is also the window's title bar: the
//! sheets button (opens the drawer), the wordmark and where you are; the
//! search box in the middle; on the right the screen's main action, then
//! minimize, maximize and close. Dragging the bar moves the window and a
//! double-click maximizes it.
//!
//! On Windows the system does the moving, snapping and the caption buttons'
//! clicks (each area says what it is); elsewhere the bar does them itself.
//! Everything clickable in the bar blocks the drag area under it.

use gpui::component::InteractiveElementExt;
use gpui::{div, prelude::*, px, AnimationExt as _, Context, MouseButton, Window, WindowControlArea};

use super::app::{Folio, Screen};
use super::*;
use crate::theme::theme;

impl Folio {
    pub(super) fn render_topbar(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let t = theme(cx).clone();
        let here: Option<String> = match self.screen {
            Screen::Home => None,
            Screen::Sheet => Some(self.sheet().name.clone()),
            Screen::Settings => Some("Settings".into()),
            Screen::Bin => Some("Trash & archive".into()),
            Screen::Transfer => Some("Sync & export".into()),
        };
        // Everything in the bar is small and quiet: 28px controls, 14px icons.
        let bar_button = |id: &'static str, path: &'static str| {
            div()
                .id(id)
                .occlude()
                .size(t.size.bar_control)
                .flex_none()
                .flex()
                .items_center()
                .justify_center()
                .rounded(t.radius.small)
                .cursor_pointer()
                .hover(|d| d.bg(t.colors.hover))
                .active(|d| d.bg(t.colors.border))
                .child(icon(path, t.size.icon_sm, t.colors.text_muted))
        };
        // Tools that belong together sit in a sunk tray with a hairline
        // edge; the one under the pointer rises out of it, as a key would.
        let tray = |id: &'static str| {
            div()
                .id(id)
                .occlude()
                .h(t.size.bar_control)
                .p(px(2.))
                .flex()
                .items_center()
                .gap(px(2.))
                .rounded(t.radius.button)
                .bg(t.colors.surface_sunk)
                .border_1()
                .border_color(t.colors.border)
        };
        let glyph = |id: &'static str, path: &'static str| {
            let ink = t.colors.text;
            icon(path, t.size.icon_sm, t.colors.text_muted).group_hover(id, move |s| s.text_color(ink))
        };
        let key = |id: &'static str, glyph: gpui::AnyElement, tip: String, live: bool| {
            div()
                .id(id)
                .group(id)
                .relative()
                .size(t.size.bar_control - px(6.))
                .flex()
                .items_center()
                .justify_center()
                .rounded(t.radius.small)
                .child(glyph)
                .tooltip(move |window, cx| gpui::component::tooltip::Tooltip::new(tip.clone()).build(window, cx))
                .map(|d| {
                    if live {
                        d.cursor_pointer()
                            .hover(|d| d.bg(t.colors.surface).shadow(t.shadow_raised()))
                            .active(|d| d.bg(t.colors.hover).shadow(Vec::new()))
                    } else {
                        // Nothing to do yet: faded, and the pointer says so.
                        d.opacity(0.4)
                    }
                })
        };

        // Where Google Drive sync is at, at a glance; opens Sync & export.
        let (sync_icon, sync_dot, sync_tip) = {
            use super::sync::DriveState as S;
            match &self.sync.state {
                S::Off => ("icons/cloud.svg", None, "Sync with Google Drive".to_string()),
                S::SigningIn => ("icons/cloud.svg", None, "Signing in to Google…".to_string()),
                S::Idle => (
                    "icons/cloud.svg",
                    Some(t.colors.ok),
                    match self.sync.synced_at {
                        Some(at) => format!("Google Drive · synced {}", crate::time::ago(at, crate::time::now()).replace("Yesterday", "yesterday")),
                        None => "Google Drive · on".to_string(),
                    },
                ),
                S::Syncing => ("icons/refresh-cw.svg", Some(t.colors.brand), "Syncing with Google Drive…".to_string()),
                S::Choice => ("icons/cloud.svg", Some(t.colors.alert), "Google Drive is waiting for you".to_string()),
                S::Error(e) => ("icons/cloud-off.svg", Some(t.colors.danger), format!("Sync stopped: {e}")),
            }
        };
        let sync_glyph = glyph("sync", sync_icon);
        // While it syncs, the arrows go round.
        let sync_glyph = if self.sync.state == super::sync::DriveState::Syncing && !super::motion::reduced() {
            sync_glyph
                .with_animation("sync-spin", gpui::Animation::new(std::time::Duration::from_millis(1100)).repeat(), |s, k| {
                    s.with_transformation(gpui::Transformation::rotate(gpui::percentage(k)))
                })
                .into_any_element()
        } else {
            sync_glyph.into_any_element()
        };
        let sync_button = key("sync", sync_glyph, sync_tip, true)
            .children(sync_dot.map(|c| {
                div()
                    .absolute()
                    .bottom(px(3.))
                    .right(px(3.))
                    .size(px(7.))
                    .rounded(t.radius.full)
                    .bg(c)
                    .border_1()
                    .border_color(t.colors.surface_sunk)
            }))
            .on_click(cx.listener(|this, _, _, cx| this.open_transfer(cx)));
        let on_sheet = self.screen == Screen::Sheet;
        let theme_icon = if t.is_dark { "icons/sun.svg" } else { "icons/moon.svg" };

        let left = div()
            .flex()
            .items_center()
            .gap(t.space.xs)
            .min_w_0()
            .child(
                // Opens the sheets drawer.
                div()
                    .id("drawer")
                    .occlude()
                    .h(t.size.bar_control)
                    .px(t.space.sm)
                    .flex()
                    .items_center()
                    .gap(t.space.sm)
                    .rounded(t.radius.small)
                    .text_color(t.colors.text_muted)
                    .cursor_pointer()
                    .hover(|d| d.bg(t.colors.hover).text_color(t.colors.text))
                    .when(self.drawer, |d| d.bg(t.colors.hover))
                    .child(icon("icons/layers.svg", t.size.icon_sm, t.colors.text_muted))
                    .child(div().text_size(t.text.caption).font_weight(gpui::FontWeight::MEDIUM).child("Sheets"))
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.finish_edit(cx);
                        this.drawer = !this.drawer;
                        cx.notify();
                    })),
            )
            .child(div().w(px(1.)).h(px(16.)).mx(t.space.xs).bg(t.colors.border))
            .child(
                div()
                    .id("home-mark")
                    .occlude()
                    .px(t.space.xs)
                    .flex()
                    .items_center()
                    .cursor_pointer()
                    .text_size(px(15.))
                    .font_weight(gpui::FontWeight::BOLD)
                    .child("folio")
                    .child(div().text_color(t.colors.brand).child("."))
                    .on_click(cx.listener(|this, _, _, cx| this.go_home(cx))),
            )
            .children(here.map(|name| {
                let icon_text = (self.screen == Screen::Sheet).then(|| self.sheet().icon.clone()).filter(|i| !i.is_empty());
                div()
                    .flex()
                    .items_center()
                    .gap(t.space.sm)
                    .min_w_0()
                    .text_size(t.text.caption)
                    .child(div().text_color(t.colors.text_subtle).child("/"))
                    .children(icon_text.map(|i| mark(&i, 18., &t)))
                    .child(div().font_weight(gpui::FontWeight::MEDIUM).text_color(t.colors.text_body).truncate().child(name))
            }));

        // Back and forward through where you were, just left of the
        // search, dimmed when there's nowhere to go. Placed off the field's
        // left edge, so the field stays centred.
        let nav = div()
            .absolute()
            .top_0()
            .bottom_0()
            .right(gpui::relative(1.))
            .mr(t.space.sm)
            .flex()
            .items_center()
            .gap(px(2.))
            .child({
                let can = !self.nav_back.is_empty();
                bar_button("go-back", "icons/arrow-left.svg")
                    .when(!can, |d| d.opacity(0.35).cursor_default())
                    .tooltip(|window, cx| gpui::component::tooltip::Tooltip::new("Back  Alt+←").build(window, cx))
                    .on_click(cx.listener(|this, _, window, cx| this.go_back(window, cx)))
            })
            .child({
                let can = !self.nav_forward.is_empty();
                bar_button("go-forward", "icons/arrow-right.svg")
                    .when(!can, |d| d.opacity(0.35).cursor_default())
                    .tooltip(|window, cx| gpui::component::tooltip::Tooltip::new("Forward  Alt+→").build(window, cx))
                    .on_click(cx.listener(|this, _, window, cx| this.go_forward(window, cx)))
            });
        let field = div().relative().child(nav).child(self.palette_field(window, cx));
        let search = div()
            .absolute()
            .left_0()
            .right_0()
            .top_0()
            .bottom_0()
            .flex()
            .items_center()
            .justify_center()
            .child(field);

        // The screen's main action: the one coloured thing in the bar.
        let new_button = |id: &'static str, label: &'static str, tip: String| {
            div()
                .id(id)
                .occlude()
                .h(t.size.bar_control)
                .pl(px(10.))
                .pr(t.space.md)
                .ml(t.space.xs)
                .flex()
                .items_center()
                .gap(px(6.))
                .rounded(t.radius.button)
                .bg(t.colors.primary)
                .shadow(t.shadow_primary())
                .hover(|d| d.bg(t.colors.primary_hover))
                .active(|d| d.bg(t.colors.primary_hover).opacity(0.9))
                .text_color(t.colors.primary_fg)
                .text_size(t.text.caption)
                .font_weight(gpui::FontWeight::SEMIBOLD)
                .cursor_pointer()
                .child(icon("icons/plus.svg", px(14.), t.colors.primary_fg))
                .child(label)
                .tooltip(move |window, cx| gpui::component::tooltip::Tooltip::new(tip.clone()).build(window, cx))
        };
        let (can_undo, can_redo) = (!self.undo.is_empty() && self.editing.is_none(), !self.redo.is_empty());
        let right = div()
            .id("bar-actions")
            .flex()
            .items_center()
            .gap(t.space.sm)
            .when(on_sheet, |d| {
                d.child(
                    tray("history")
                        .child(
                            key("undo", glyph("undo", "icons/undo-2.svg").into_any_element(), format!("Undo  {}", shortcut("Z")), can_undo)
                                .on_click(cx.listener(|this, _, _, cx| this.undo(cx))),
                        )
                        .child(
                            key("redo", glyph("redo", "icons/redo-2.svg").into_any_element(), format!("Redo  {}", shortcut("Y")), can_redo)
                                .on_click(cx.listener(|this, _, _, cx| this.redo(cx))),
                        ),
                )
            })
            .child(
                tray("look").child(sync_button).child(
                    key(
                        "theme",
                        glyph("theme", theme_icon).into_any_element(),
                        format!("{} mode  {}", if t.is_dark { "Light" } else { "Dark" }, shortcut("Shift+L")),
                        true,
                    )
                    .on_click(cx.listener(|this, _, _, cx| this.toggle_theme(cx))),
                ),
            )
            .child(if on_sheet {
                new_button("new-note", "Note", "New note  N".into())
                    .on_click(cx.listener(|this, _, window, cx| this.new_note(window, cx)))
                    .into_any_element()
            } else {
                new_button("new-sheet", "Sheet", format!("New sheet  {}", shortcut("Shift+N")))
                    .on_click(cx.listener(|this, _, window, cx| this.new_sheet_dialog(window, cx)))
                    .into_any_element()
            });

        let controls = window_controls(window, &t);
        let has_controls = controls.is_some();
        div()
            .id("topbar")
            .h(t.size.topbar)
            .flex_none()
            .relative()
            .flex()
            .items_center()
            .pl(if cfg!(target_os = "macos") { px(80.) } else { t.space.sm })
            .when(!has_controls, |d| d.pr(t.space.md))
            .bg(t.colors.surface)
            .border_b_1()
            .border_color(t.colors.border)
            .window_control_area(WindowControlArea::Drag)
            .when(cfg!(target_os = "linux"), |d| {
                d.on_double_click(|_, window, _| window.zoom_window())
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(|this, _, _, _| this.bar_drag = true),
                    )
                    .on_mouse_up(MouseButton::Left, cx.listener(|this, _, _, _| this.bar_drag = false))
                    .on_mouse_move(cx.listener(|this, _, window, _| {
                        if std::mem::take(&mut this.bar_drag) {
                            window.start_window_move();
                        }
                    }))
            })
            .child(search)
            .child(left)
            .child(div().flex_1())
            .child(right)
            .children(controls)
    }
}

/// Minimize, maximize (or restore) and close, at the bar's right end, the
/// size Windows draws them. None on a Mac, where the window has its own.
fn window_controls(window: &Window, t: &crate::theme::Theme) -> Option<gpui::Div> {
    if cfg!(target_os = "macos") {
        return None;
    }
    let maximized = window.is_maximized();
    let button = |id: &'static str, glyph: &'static str, area: WindowControlArea| {
        let close = area == WindowControlArea::Close;
        let (hover_bg, hover_fg) = if close { (t.colors.window_close, t.colors.primary_fg) } else { (t.colors.hover, t.colors.text) };
        div()
            .id(id)
            .group(id)
            .occlude()
            .w(px(44.))
            .h_full()
            .flex()
            .items_center()
            .justify_center()
            .hover(move |d| d.bg(hover_bg))
            .window_control_area(area)
            .child(svg_glyph(glyph, t).group_hover(id, move |s| s.text_color(hover_fg)))
            .when(!cfg!(target_os = "windows"), |d| {
                d.on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation()).on_click(move |_, window, cx| {
                    cx.stop_propagation();
                    match area {
                        WindowControlArea::Min => window.minimize_window(),
                        WindowControlArea::Max => window.zoom_window(),
                        _ => window.dispatch_action(Box::new(Quit), cx),
                    }
                })
            })
    };
    Some(
        div()
            .h_full()
            .flex()
            .items_center()
            .child(div().w(px(1.)).h(px(16.)).mx(t.space.sm).bg(t.colors.border))
            .child(button("win-min", "icons/caption-minimize.svg", WindowControlArea::Min))
            .child(button(
                "win-max",
                if maximized { "icons/caption-restore.svg" } else { "icons/caption-maximize.svg" },
                WindowControlArea::Max,
            ))
            .child(button("win-close", "icons/caption-close.svg", WindowControlArea::Close)),
    )
}

fn svg_glyph(path: &'static str, t: &crate::theme::Theme) -> gpui::Svg {
    icon(path, t.size.icon_sm, t.colors.text_muted)
}
