//! The views. `app.rs` holds the state; each other file renders one part.

pub mod app;
mod board;
mod bin;
mod dialog;
mod drawer;
mod emoji;
mod home;
mod intro;
mod menu;
mod mini;
mod palette;
pub mod popout;
pub mod quick;
mod settings;
mod sticker;
mod sync;
mod toolbar;
mod topbar;

use gpui::{div, prelude::*, px, svg, App, Hsla, Pixels, SharedString, Svg};

use crate::theme::{theme, Theme};

gpui::actions!(
    folio,
    [
        Quit,
        Undo,
        Redo,
        ToggleTheme,
        NewSheet,
        NewNote,
        QuickNote,
        NextSheet,
        PrevSheet,
        GoHome,
        ToggleDrawer,
        OpenPalette,
        OpenSettings,
        FocusSearch,
        Dismiss,
        DeleteSelection,
        DuplicateSelection,
        EditSelection,
        SelectAll,
        Paste,
        Escape,
        ZoomIn,
        ZoomOut,
        ZoomToFit,
        Tidy,
        ToggleMinimap,
        StepLeft,
        StepRight,
        StepUp,
        StepDown,
        PaletteUp,
        PaletteDown,
        ToolSelect,
        ToolHand,
        ToolPen,
        ToolMarker,
        ToolText,
        ToolConnect,
        ToolFrame,
        ToolEraser,
        SaveNote,
    ]
);

pub fn icon(path: &'static str, size: Pixels, color: Hsla) -> Svg {
    svg().path(path).size(size).flex_none().text_color(color)
}

/// A shortcut as this platform writes it: Ctrl+Z, or ⌘Z on a Mac.
pub fn shortcut(key: &str) -> String {
    if cfg!(target_os = "macos") {
        format!("⌘{key}")
    } else {
        format!("Ctrl+{key}")
    }
}

/// A square, borderless button holding one icon.
pub fn icon_button(id: impl Into<gpui::ElementId>, path: &'static str, t: &Theme) -> gpui::Stateful<gpui::Div> {
    div()
        .id(id)
        .size(t.size.control)
        .flex_none()
        .flex()
        .items_center()
        .justify_center()
        .rounded(t.radius.button)
        .cursor_pointer()
        .hover(|d| d.bg(t.colors.hover))
        .child(icon(path, t.size.icon, t.colors.text_muted))
}

/// The filled button for a view's one main action.
pub fn primary_button(id: impl Into<gpui::ElementId>, label: &'static str, t: &Theme) -> gpui::Stateful<gpui::Div> {
    let (bg, hover, fg) = (t.colors.primary, t.colors.primary_hover, t.colors.primary_fg);
    div()
        .id(id)
        .h(t.size.control)
        .px(t.space.md)
        .flex()
        .items_center()
        .gap(t.space.xs)
        .rounded(t.radius.button)
        .bg(bg)
        .hover(move |d| d.bg(hover))
        .text_color(fg)
        .text_size(t.text.caption)
        .font_weight(gpui::FontWeight::SEMIBOLD)
        .cursor_pointer()
        .child(label)
}

pub fn secondary_button(id: impl Into<gpui::ElementId>, label: &'static str, t: &Theme) -> gpui::Stateful<gpui::Div> {
    div()
        .id(id)
        .h(t.size.control)
        .px(t.space.md)
        .flex()
        .items_center()
        .rounded(t.radius.button)
        .bg(t.colors.hover)
        .hover(|d| d.bg(t.colors.border))
        .text_color(t.colors.text_muted)
        .text_size(t.text.caption)
        .font_weight(gpui::FontWeight::MEDIUM)
        .cursor_pointer()
        .child(label)
}

/// A sheet's badge: its emoji (or its first letter) on a quiet tile.
pub fn sheet_badge(c: &crate::model::Category, size: f32, t: &Theme) -> gpui::Div {
    let tile = div()
        .size(px(size))
        .flex_none()
        .flex()
        .items_center()
        .justify_center()
        .rounded(px(size * 0.3))
        .bg(t.colors.hover)
        .border_1()
        .border_color(t.colors.border);
    if c.icon.is_empty() {
        let letter: String = c.name.chars().find(|ch| ch.is_alphanumeric()).unwrap_or('•').to_uppercase().collect();
        tile.text_size(px(size * 0.5)).font_weight(gpui::FontWeight::BOLD).text_color(t.colors.text_muted).child(letter)
    } else {
        tile.text_size(px(size * 0.56)).child(c.icon.clone())
    }
}

/// A keyboard hint chip.
pub fn kbd(text: impl Into<SharedString>, t: &Theme) -> gpui::Div {
    div()
        .px(t.space.xs)
        .rounded(px(4.))
        .border_1()
        .border_color(t.colors.border)
        .text_size(px(10.))
        .text_color(t.colors.text_subtle)
        .child(text.into())
}

pub fn toast(t: &Theme, msg: SharedString) -> impl IntoElement {
    div()
        .absolute()
        .bottom(t.space.xl)
        .right(t.space.xl)
        .px(t.space.lg)
        .py(t.space.sm + t.space.xxs)
        .rounded(t.radius.button)
        .bg(t.colors.toast)
        .text_color(t.colors.toast_fg)
        .text_size(t.text.caption)
        .shadow(t.shadow_float())
        .child(msg)
}

/// Keeps gpui-component's inputs and markdown in Folio's colours and type.
pub fn sync_component_theme(cx: &mut App) {
    use gpui::component::theme::{Theme as ComponentTheme, ThemeMode};
    let t = theme(cx).clone();
    ComponentTheme::change(if t.is_dark { ThemeMode::Dark } else { ThemeMode::Light }, None, cx);
    ComponentTheme::update(cx, |c| {
        c.font_family = t.font.into();
        c.font_size = t.text.body;
        c.radius = t.radius.button;
        c.radius_lg = t.radius.card;
        c.shadow = false;
        c.background = t.colors.surface;
        c.foreground = t.colors.text;
        c.border = t.colors.border;
        c.input = t.colors.border;
        c.ring = t.colors.primary_tint;
        c.caret = t.colors.primary;
        c.selection = t.colors.primary_tint;
        c.primary = t.colors.primary;
        c.primary_hover = t.colors.primary_hover;
        c.primary_foreground = t.colors.primary_fg;
        c.muted = t.colors.hover;
        c.muted_foreground = t.colors.text_muted;
        c.accent = t.colors.hover;
        c.table_head = t.colors.hover;
        c.table_head_foreground = t.colors.text;
        c.link = t.colors.primary;
        // Tables sit on the sticker's paper.
        c.table = gpui::transparent_black();
        c.table_even = gpui::transparent_black();
        c.table_row_border = t.colors.border;
    });
}
