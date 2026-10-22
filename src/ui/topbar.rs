//! The bar across the top, which is also the window's title bar: the
//! sheets button (opens the drawer), the wordmark and where you are; the
//! search box in the middle; on the right the screen's main action, then
//! minimize, maximize and close. Dragging the bar moves the window and a
//! double-click maximizes it.
//!
//! On Windows the system does the moving, snapping and the caption buttons'
//! clicks (each area says what it is); elsewhere the bar does them itself.
//! Everything clickable in the bar blocks the drag area under it.

use gpui::component::input::Input;
use gpui::component::InteractiveElementExt;
use gpui::{div, prelude::*, px, Context, MouseButton, Window, WindowControlArea};

use super::app::{Folio, Screen};
use super::*;
use crate::theme::theme;

impl Folio {
    pub(super) fn render_topbar(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let t = theme(cx).clone();
        let here: Option<String> = match self.screen {
            Screen::Home => None,
            Screen::Sheet => {
                let c = self.sheet();
                Some(if c.icon.is_empty() { c.name.clone() } else { format!("{} {}", c.icon, c.name) })
            }
            Screen::Settings => Some("Settings".into()),
            Screen::Bin => Some("Trash & archive".into()),
            Screen::Transfer => Some("Sync & export".into()),
        };
        // Where Google Drive sync is at, at a glance; opens Sync & export.
        let (sync_icon, sync_dot) = {
            use super::sync::DriveState as S;
            match self.sync.state {
                S::Off | S::SigningIn => ("icons/cloud.svg", None),
                S::Idle => ("icons/cloud.svg", Some(t.colors.ok)),
                S::Syncing => ("icons/refresh-cw.svg", Some(t.colors.brand)),
                S::Choice => ("icons/cloud.svg", Some(t.colors.alert)),
                S::Error(_) => ("icons/cloud-off.svg", Some(t.colors.danger)),
            }
        };
        let sync_button = icon_button("sync", sync_icon, &t)
            .occlude()
            .relative()
            .children(sync_dot.map(|c| {
                div()
                    .absolute()
                    .top(px(6.))
                    .right(px(6.))
                    .size(t.size.dot)
                    .rounded(t.radius.full)
                    .bg(c)
                    .border_1()
                    .border_color(t.colors.surface)
            }))
            .on_click(cx.listener(|this, _, _, cx| this.open_transfer(cx)));
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
                    .occlude()
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
                    .occlude()
                    .flex()
                    .items_center()
                    .gap(t.space.sm)
                    .cursor_pointer()
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
                    .id("search")
                    .occlude()
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
            .id("bar-actions")
            .occlude()
            .flex()
            .items_center()
            .gap(t.space.xs)
            .when(on_sheet, |d| {
                d.child(icon_button("undo", "icons/undo-2.svg", &t).on_click(cx.listener(|this, _, _, cx| this.undo(cx))))
                    .child(icon_button("redo", "icons/redo-2.svg", &t).on_click(cx.listener(|this, _, _, cx| this.redo(cx))))
            })
            .child(sync_button)
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

        let controls = window_controls(window, &t);
        let has_controls = controls.is_some();
        div()
            .id("topbar")
            .h(t.size.topbar)
            .flex_none()
            .relative()
            .flex()
            .items_center()
            .pl(if cfg!(target_os = "macos") { px(84.) } else { t.space.lg })
            .when(!has_controls, |d| d.pr(t.space.lg))
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
            .w(px(46.))
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
            .ml(t.space.sm)
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
    icon(path, t.size.icon, t.colors.text_muted)
}
