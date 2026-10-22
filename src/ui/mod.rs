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
mod motion;
mod mini;
mod palette;
pub mod popout;
pub mod quick;
mod settings;
mod sticker;
mod sync;
mod toolbar;
mod transfer;
mod welcome;
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
        PaletteTab,
        GoBack,
        GoForward,
        PaletteBackTab,
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
        .active(|d| d.bg(t.colors.border))
        .child(icon(path, t.size.icon, t.colors.text_muted))
}

/// The filled button for a view's one main action.
pub fn primary_button(id: impl Into<gpui::ElementId>, label: impl Into<SharedString>, t: &Theme) -> gpui::Stateful<gpui::Div> {
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
        .shadow(t.shadow_primary())
        .hover(move |d| d.bg(hover))
        .active(move |d| d.bg(hover).opacity(0.9))
        .text_color(fg)
        .text_size(t.text.caption)
        .font_weight(gpui::FontWeight::SEMIBOLD)
        .cursor_pointer()
        .child(label.into())
}

pub fn secondary_button(id: impl Into<gpui::ElementId>, label: impl Into<SharedString>, t: &Theme) -> gpui::Stateful<gpui::Div> {
    div()
        .id(id)
        .h(t.size.control)
        .px(t.space.md)
        .flex()
        .items_center()
        .rounded(t.radius.button)
        .bg(t.colors.surface)
        .border_1()
        .border_color(t.colors.border)
        .shadow(t.shadow_raised())
        .hover(|d| d.bg(t.colors.surface_sunk).text_color(t.colors.text))
        .active(|d| d.bg(t.colors.hover))
        .text_color(t.colors.text_body)
        .text_size(t.text.caption)
        .font_weight(gpui::FontWeight::MEDIUM)
        .cursor_pointer()
        .child(label.into())
}

/// A sheet's badge: its emoji (or its first letter) on a quiet tile.
pub fn sheet_badge(c: &crate::model::Category, size: f32, t: &Theme) -> gpui::Div {
    if c.icon.is_empty() {
        let letter: String = c.name.chars().find(|ch| ch.is_alphanumeric()).unwrap_or('•').to_uppercase().collect();
        return mark(&letter, size, t);
    }
    mark(&c.icon, size, t)
}

/// A mark in letters rather than a picture: "CAN", "RS", "BMW".
pub fn is_monogram(icon: &str) -> bool {
    !icon.is_empty() && icon.chars().all(|c| c.is_ascii_graphic())
}

/// A sheet's mark on a tile `size` across: an emoji, or a monogram set in
/// bold on Folio's pink wash, smaller the longer it is.
pub fn mark(icon: &str, size: f32, t: &Theme) -> gpui::Div {
    let tile = div()
        .size(px(size))
        .flex_none()
        .flex()
        .items_center()
        .justify_center()
        .rounded(px(size * 0.3))
        .border_1();
    if is_monogram(icon) {
        let n = icon.chars().count();
        let k = match n {
            1 => 0.5,
            2 => 0.42,
            3 => 0.33,
            4 => 0.27,
            _ => 0.23,
        };
        tile.bg(t.colors.brand_tint)
            .border_color(t.colors.brand.opacity(0.25))
            .text_size(px(size * k))
            .font_weight(gpui::FontWeight::BOLD)
            .text_color(t.colors.brand_on_tint)
            .child(icon.to_string())
    } else {
        tile.bg(t.colors.hover).border_color(t.colors.border).text_size(px(size * 0.56)).child(icon.to_string())
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
    let id = gpui::ElementId::Name(format!("toast-{msg}").into());
    // Crisp rather than soft: small corners, a hairline edge, a lit top
    // edge and a brand mark, sitting on a tight shadow.
    div().absolute().bottom(t.space.xl).right(t.space.xl).child(motion::rise_in(
        div()
            .flex()
            .items_center()
            .gap(t.space.md)
            .pl(t.space.sm)
            .pr(t.space.lg)
            .py(t.space.sm)
            .max_w(px(420.))
            .rounded(t.radius.small)
            .bg(t.colors.toast)
            .border_1()
            .border_color(t.colors.toast_fg.opacity(0.1))
            .text_color(t.colors.toast_fg)
            .text_size(t.text.caption)
            .font_weight(gpui::FontWeight::MEDIUM)
            .shadow({
                let mut s = t.shadow_float();
                s.push(gpui::BoxShadow {
                    color: t.colors.toast_fg.opacity(0.08),
                    offset: gpui::point(px(0.), px(1.)),
                    blur_radius: px(0.),
                    spread_radius: px(0.),
                    inset: true,
                });
                s
            })
            .child(
                div()
                    .size(px(22.))
                    .flex_none()
                    .flex()
                    .items_center()
                    .justify_center()
                    .rounded(t.radius.small)
                    .bg(t.colors.brand)
                    .child(icon("icons/bell.svg", px(12.), t.colors.primary_fg)),
            )
            .child(div().w(px(1.)).h(px(14.)).bg(t.colors.toast_fg.opacity(0.14)))
            .child(div().flex_1().min_w_0().child(msg)),
        id,
        8.,
        t,
    ))
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

/// Lays out and paints `child` with the rem at `rem`: a text field sized
/// in rems (its height, its line) grows with it, so it can be typed into
/// at a heading's size.
pub struct RemScaled {
    rem: Pixels,
    child: gpui::AnyElement,
}

pub fn rem_scaled(rem: Pixels, child: impl IntoElement) -> RemScaled {
    RemScaled { rem, child: child.into_any_element() }
}

impl IntoElement for RemScaled {
    type Element = Self;
    fn into_element(self) -> Self {
        self
    }
}

impl gpui::Element for RemScaled {
    type RequestLayoutState = ();
    type PrepaintState = ();

    fn id(&self) -> Option<gpui::ElementId> {
        None
    }

    fn source_location(&self) -> Option<&'static core::panic::Location<'static>> {
        None
    }

    fn request_layout(
        &mut self,
        _: Option<&gpui::GlobalElementId>,
        _: Option<&gpui::InspectorElementId>,
        window: &mut gpui::Window,
        cx: &mut App,
    ) -> (gpui::LayoutId, ()) {
        let child = &mut self.child;
        (window.with_rem_size(Some(self.rem), |w| child.request_layout(w, cx)), ())
    }

    fn prepaint(
        &mut self,
        _: Option<&gpui::GlobalElementId>,
        _: Option<&gpui::InspectorElementId>,
        _: gpui::Bounds<Pixels>,
        _: &mut (),
        window: &mut gpui::Window,
        cx: &mut App,
    ) {
        let child = &mut self.child;
        window.with_rem_size(Some(self.rem), |w| child.prepaint(w, cx));
    }

    fn paint(
        &mut self,
        _: Option<&gpui::GlobalElementId>,
        _: Option<&gpui::InspectorElementId>,
        _: gpui::Bounds<Pixels>,
        _: &mut (),
        _: &mut (),
        window: &mut gpui::Window,
        cx: &mut App,
    ) {
        let child = &mut self.child;
        window.with_rem_size(Some(self.rem), |w| child.paint(w, cx));
    }
}
