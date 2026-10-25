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
mod label;
mod palette;
mod pins;
pub mod popout;
pub mod quick;
mod selection;
mod settings;
mod sticker;
mod sync;
mod toolbar;
mod transfer;
mod welcome;
mod topbar;

use gpui::{div, prelude::*, px, svg, AnyView, App, Context, Hsla, Pixels, SharedString, Svg, Window};

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
        CopyNotes,
        CutNotes,
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
        // Writing in a note: the edit bar's first row.
        Bold,
        Italic,
        Strike,
        InlineCode,
        Highlight,
        Bullets,
        Numbers,
        Tasks,
        Callout,
        InsertTable,
        NewLine,
        ToMarkdown,
        PasteMarkdown,
        CycleStyle,
    ]
);

/// The writing shortcuts on an element (a note being edited): each runs
/// `f` with its formatting command.
pub fn on_format<E: gpui::InteractiveElement>(el: E, f: impl Fn(app::Format, &mut gpui::Window, &mut App) + Clone + 'static) -> E {
    use app::Format as F;
    let g = f.clone();
    let el = el.on_action(move |_: &Bold, w, cx| g(F::Bold, w, cx));
    let g = f.clone();
    let el = el.on_action(move |_: &Italic, w, cx| g(F::Italic, w, cx));
    let g = f.clone();
    let el = el.on_action(move |_: &Strike, w, cx| g(F::Strike, w, cx));
    let g = f.clone();
    let el = el.on_action(move |_: &InlineCode, w, cx| g(F::Code, w, cx));
    let g = f.clone();
    let el = el.on_action(move |_: &Highlight, w, cx| g(F::Highlight, w, cx));
    let g = f.clone();
    let el = el.on_action(move |_: &Bullets, w, cx| g(F::Bullets, w, cx));
    let g = f.clone();
    let el = el.on_action(move |_: &Numbers, w, cx| g(F::Numbers, w, cx));
    let g = f.clone();
    let el = el.on_action(move |_: &Tasks, w, cx| g(F::Tasks, w, cx));
    let g = f.clone();
    let el = el.on_action(move |_: &Callout, w, cx| g(F::Callout, w, cx));
    let g = f.clone();
    let el = el.on_action(move |_: &InsertTable, w, cx| g(F::Table, w, cx));
    let g = f.clone();
    let el = el.on_action(move |_: &ToMarkdown, w, cx| g(F::Markdown, w, cx));
    let g = f.clone();
    let el = el.on_action(move |_: &PasteMarkdown, w, cx| g(F::Paste, w, cx));
    el.on_action(move |_: &NewLine, w, cx| f(F::NewLine, w, cx))
}

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

/// Google's "G", in its own colours: four tinted pieces laid on each other.
pub fn google_g(size: Pixels, t: &Theme) -> gpui::Div {
    let piece = |name: &'static str, c: Hsla| icon(name, size, c).absolute().top_0().left_0();
    let [blue, green, yellow, red] = t.colors.google;
    div()
        .relative()
        .size(size)
        .flex_none()
        .child(piece("icons/google-g-red.svg", red))
        .child(piece("icons/google-g-yellow.svg", yellow))
        .child(piece("icons/google-g-green.svg", green))
        .child(piece("icons/google-g-blue.svg", blue))
}

/// "Continue with Google", the way Google's sign-in buttons look: the G on
/// a plain surface.
pub fn google_button(id: impl Into<gpui::ElementId>, t: &Theme) -> gpui::Stateful<gpui::Div> {
    div()
        .id(id)
        .h(t.size.control)
        .pl(t.space.md)
        .pr(t.space.lg)
        .flex()
        .flex_none()
        .items_center()
        .gap(t.space.sm)
        .rounded(t.radius.button)
        .bg(t.colors.surface)
        .border_1()
        .border_color(t.colors.border)
        .shadow(t.shadow_raised())
        .hover(|d| d.bg(t.colors.surface_sunk))
        .active(|d| d.bg(t.colors.hover))
        .text_color(t.colors.text)
        .text_size(t.text.caption)
        .font_weight(gpui::FontWeight::SEMIBOLD)
        .cursor_pointer()
        .child(google_g(t.size.icon, t))
        .child("Continue with Google")
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

/// A tooltip: gpui-component's, fading in once it shows.
pub fn tip(text: impl Into<SharedString>, window: &mut Window, cx: &mut App) -> AnyView {
    let inner = gpui::component::tooltip::Tooltip::new(text.into()).build(window, cx);
    cx.new(|_| Tip(inner)).into()
}

struct Tip(AnyView);

impl Render for Tip {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let ms = theme(cx).motion.tip_in;
        motion::fade_in(div().child(self.0.clone()), "tip-in", ms)
    }
}

/// A toast in the corner: it slides in from the right as it fades in.
pub fn toast(t: &Theme, msg: SharedString) -> impl IntoElement {
    let id = gpui::ElementId::Name(format!("toast-{msg}").into());
    div().absolute().bottom(t.space.xl).right(t.space.xl).child(motion::slide_in(toast_card(t, msg), id, t))
}

/// A toast that just went: it fades as it sinks.
pub fn toast_leaving(t: &Theme, msg: SharedString, n: usize) -> impl IntoElement {
    div().absolute().bottom(t.space.xl).right(t.space.xl).child(motion::leave(
        toast_card(t, msg).relative(),
        ("toast-out", n),
        t.motion.toast_out,
        t.motion.sink,
    ))
}

/// Crisp rather than soft: small corners, a hairline edge, a lit top edge,
/// sitting on a tight shadow. The toast and the author's credit.
fn crisp(t: &Theme, bg: Hsla, fg: Hsla) -> gpui::Div {
    div()
        .flex()
        .items_center()
        .gap(t.space.md)
        .pl(t.space.sm)
        .pr(t.space.lg)
        .py(t.space.sm)
        .rounded(t.radius.small)
        .bg(bg)
        .border_1()
        .border_color(fg.opacity(0.1))
        .text_color(fg)
        .shadow({
            let mut s = t.shadow_float();
            s.push(gpui::BoxShadow {
                color: fg.opacity(0.08),
                offset: gpui::point(px(0.), px(1.)),
                blur_radius: px(0.),
                spread_radius: px(0.),
                inset: true,
            });
            s
        })
}

/// The brand mark a crisp card leads with: `path` on a pink tile `d` across.
fn brand_tile(path: &'static str, d: Pixels, t: &Theme) -> gpui::Div {
    div()
        .size(d)
        .flex_none()
        .flex()
        .items_center()
        .justify_center()
        .rounded(t.radius.small)
        .bg(t.colors.brand)
        .child(icon(path, d * 0.55, t.colors.primary_fg))
}

fn toast_card(t: &Theme, msg: SharedString) -> gpui::Div {
    crisp(t, t.colors.toast, t.colors.toast_fg)
        .max_w(px(420.))
        .text_size(t.text.caption)
        .font_weight(gpui::FontWeight::MEDIUM)
        .child(brand_tile("icons/bell.svg", px(22.), t))
        .child(div().w(px(1.)).h(px(14.)).bg(t.colors.toast_fg.opacity(0.14)))
        .child(div().flex_1().min_w_0().child(msg))
}

/// Who made Folio, signed: a whisper of a credit for a corner. Small
/// spaced capitals, "CREATED & DESIGNED BY", over the name in italic,
/// written out from left to right as `writing` goes 0 to 1; then a fine
/// swash of `accent` drawn under it as `swash` does. `ink` is the name's
/// colour; the capitals are fainter still.
pub fn signature(t: &Theme, ink: Hsla, accent: Hsla, writing: f32, swash: f32) -> gpui::Div {
    let caption = div()
        .flex()
        .gap(px(1.5))
        .text_size(t.text.micro * 0.8)
        .font_weight(gpui::FontWeight::MEDIUM)
        .text_color(ink.opacity(0.62))
        .children("CREATED & DESIGNED BY".chars().map(|c| div().child(c.to_string())));
    let name = || {
        div()
            .whitespace_nowrap()
            .italic()
            .font_weight(gpui::FontWeight::MEDIUM)
            .text_size(t.text.brand)
            .text_color(ink)
            .child("M. Amine Said")
    };
    let written = div()
        .relative()
        // Its room, then the name written into it a stroke at a time.
        .child(name().invisible())
        .child(div().absolute().top_0().left_0().h_full().w(gpui::relative(writing.clamp(0., 1.))).overflow_hidden().child(name()))
        .child(
            gpui::canvas(|_, _, _| {}, move |b, _, window, _| paint_swash(b, swash, accent, window))
                .absolute()
                .left(px(-2.))
                .right(px(-12.))
                .bottom(px(-10.))
                .h(px(12.)),
        );
    div().flex().flex_col().items_end().gap(px(5.)).child(caption).child(written)
}

/// The signature's swash: one long, easy wave under the name, drawn as far
/// as `k` (0 to 1) of its length.
fn paint_swash(b: gpui::Bounds<Pixels>, k: f32, color: Hsla, window: &mut Window) {
    if k <= 0. {
        return;
    }
    let (x, y, w, h) = (f32::from(b.origin.x), f32::from(b.origin.y), f32::from(b.size.width), f32::from(b.size.height));
    let at = |s: f32| {
        // A cubic through the box: in low on the left, up and away right.
        let p = [(x, y + h * 0.55), (x + w * 0.38, y + h * 1.05), (x + w * 0.72, y - h * 0.05), (x + w, y + h * 0.25)];
        let u = 1. - s;
        let c = [u * u * u, 3. * u * u * s, 3. * u * s * s, s * s * s];
        gpui::point(px(c.iter().zip(p).map(|(c, p)| c * p.0).sum()), px(c.iter().zip(p).map(|(c, p)| c * p.1).sum()))
    };
    let n = 48;
    let mut path = gpui::PathBuilder::stroke(px(1.1));
    path.move_to(at(0.));
    for i in 1..=n {
        let s = k.min(1.) * i as f32 / n as f32;
        path.line_to(at(s));
    }
    if let Ok(path) = path.build() {
        window.paint_path(path, color);
    }
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
