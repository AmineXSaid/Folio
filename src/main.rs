#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

mod assets;
mod demo;
mod model;
mod store;
mod theme;
mod ui;

use gpui::{px, size, App, AppContext, Bounds, KeyBinding, WindowBounds, WindowOptions};

use ui::*;

fn bind_keys(cx: &mut App) {
    cx.bind_keys([
        KeyBinding::new("secondary-q", Quit, None),
        KeyBinding::new("secondary-z", Undo, Some("Board")),
        KeyBinding::new("secondary-shift-z", Redo, Some("Board")),
        KeyBinding::new("secondary-y", Redo, Some("Board")),
        KeyBinding::new("secondary-shift-l", ToggleTheme, None),
        KeyBinding::new("secondary-shift-n", NewSheet, None),
        KeyBinding::new("secondary-n", NewNote, None),
        KeyBinding::new("secondary-f", FocusSearch, None),
        KeyBinding::new("ctrl-pagedown", NextSheet, None),
        KeyBinding::new("ctrl-pageup", PrevSheet, None),
        KeyBinding::new("ctrl-tab", NextSheet, None),
        KeyBinding::new("ctrl-shift-tab", PrevSheet, None),
        KeyBinding::new("n", NewNote, Some("Board")),
        KeyBinding::new("delete", DeleteSelection, Some("Board")),
        KeyBinding::new("backspace", DeleteSelection, Some("Board")),
        KeyBinding::new("secondary-d", DuplicateSelection, Some("Board")),
        KeyBinding::new("enter", EditSelection, Some("Board")),
        KeyBinding::new("secondary-v", Paste, Some("Board")),
        KeyBinding::new("escape", Escape, Some("Board")),
        KeyBinding::new("escape", Escape, Some("Editor")),
        KeyBinding::new("secondary-enter", SaveEditor, Some("Editor")),
        // Ahead of the text fields' own Ctrl+Enter and Escape.
        KeyBinding::new("secondary-enter", SaveEditor, Some("Editor > Input")),
        KeyBinding::new("escape", Escape, Some("Editor > Input")),
        KeyBinding::new("secondary-=", ZoomIn, Some("Board")),
        KeyBinding::new("secondary-+", ZoomIn, Some("Board")),
        KeyBinding::new("secondary--", ZoomOut, Some("Board")),
        KeyBinding::new("secondary-0", ZoomToFit, Some("Board")),
        KeyBinding::new("shift-1", ZoomToFit, Some("Board")),
        KeyBinding::new("v", ToolSelect, Some("Board")),
        KeyBinding::new("h", ToolHand, Some("Board")),
        KeyBinding::new("p", ToolPen, Some("Board")),
        KeyBinding::new("m", ToolMarker, Some("Board")),
        KeyBinding::new("t", ToolText, Some("Board")),
        KeyBinding::new("c", ToolConnect, Some("Board")),
        KeyBinding::new("e", ToolEraser, Some("Board")),
    ]);
}

fn main() {
    gpui::application().with_assets(assets::Assets).run(|cx: &mut App| {
        gpui::init(cx);
        assets::load_fonts(cx);
        theme::set(false, cx);
        bind_keys(cx);
        cx.on_action(|_: &Quit, cx| cx.quit());

        let bounds = Bounds::centered(None, size(px(1280.), px(800.)), cx);
        let options = WindowOptions {
            window_bounds: Some(WindowBounds::Windowed(bounds)),
            window_min_size: Some(size(px(760.), px(520.))),
            app_id: Some("folio".into()),
            titlebar: Some(gpui::TitlebarOptions { title: Some("Folio".into()), ..Default::default() }),
            ..Default::default()
        };
        gpui::open_window(options, cx, |window, cx| cx.new(|cx| ui::app::Folio::new(window, cx)))
            .expect("the window opens");
        cx.activate(true);
    });
}
