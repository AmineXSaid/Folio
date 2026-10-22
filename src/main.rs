#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

mod assets;
mod avatars;
mod cloud;
mod demo;
mod export;
mod fonts;
mod greeting;
mod desktop;
mod links;
mod merge;
mod model;
mod store;
mod theme;
mod time;
mod ui;

use gpui::{px, size, App, AppContext, Bounds, KeyBinding, WindowBounds, WindowOptions};

use ui::*;

fn bind_keys(cx: &mut App) {
    cx.bind_keys([
        // Everywhere in the main window.
        KeyBinding::new("secondary-q", Quit, None),
        KeyBinding::new("secondary-shift-l", ToggleTheme, None),
        KeyBinding::new("secondary-shift-n", NewSheet, None),
        KeyBinding::new("secondary-n", NewNote, None),
        KeyBinding::new("secondary-alt-n", QuickNote, None),
        KeyBinding::new("secondary-f", FocusSearch, None),
        KeyBinding::new("secondary-k", OpenPalette, None),
        KeyBinding::new("secondary-b", ToggleDrawer, None),
        KeyBinding::new("secondary-,", OpenSettings, None),
        KeyBinding::new("secondary-shift-h", GoHome, None),
        KeyBinding::new("alt-left", GoBack, None),
        KeyBinding::new("alt-right", GoForward, None),
        KeyBinding::new("ctrl-pagedown", NextSheet, None),
        KeyBinding::new("ctrl-pageup", PrevSheet, None),
        KeyBinding::new("ctrl-tab", NextSheet, None),
        KeyBinding::new("ctrl-shift-tab", PrevSheet, None),
        KeyBinding::new("escape", Dismiss, None),
        // The sheet.
        KeyBinding::new("secondary-z", Undo, Some("Board")),
        KeyBinding::new("secondary-shift-z", Redo, Some("Board")),
        KeyBinding::new("secondary-y", Redo, Some("Board")),
        KeyBinding::new("n", NewNote, Some("Board")),
        KeyBinding::new("delete", DeleteSelection, Some("Board")),
        KeyBinding::new("backspace", DeleteSelection, Some("Board")),
        KeyBinding::new("secondary-d", DuplicateSelection, Some("Board")),
        KeyBinding::new("secondary-a", SelectAll, Some("Board")),
        KeyBinding::new("enter", EditSelection, Some("Board")),
        KeyBinding::new("secondary-v", Paste, Some("Board")),
        KeyBinding::new("escape", Escape, Some("Board")),
        KeyBinding::new("secondary-=", ZoomIn, Some("Board")),
        KeyBinding::new("secondary-+", ZoomIn, Some("Board")),
        KeyBinding::new("secondary--", ZoomOut, Some("Board")),
        KeyBinding::new("secondary-0", ZoomToFit, Some("Board")),
        KeyBinding::new("shift-1", ZoomToFit, Some("Board")),
        KeyBinding::new("left", StepLeft, Some("Board")),
        KeyBinding::new("right", StepRight, Some("Board")),
        KeyBinding::new("up", StepUp, Some("Board")),
        KeyBinding::new("down", StepDown, Some("Board")),
        KeyBinding::new("v", ToolSelect, Some("Board")),
        KeyBinding::new("h", ToolHand, Some("Board")),
        KeyBinding::new("p", ToolPen, Some("Board")),
        KeyBinding::new("m", ToolMarker, Some("Board")),
        KeyBinding::new("t", ToolText, Some("Board")),
        KeyBinding::new("c", ToolConnect, Some("Board")),
        KeyBinding::new("f", ToolFrame, Some("Board")),
        KeyBinding::new("e", ToolEraser, Some("Board")),
        // Typing into a note, a dialog, the palette: ahead of the text
        // fields' own Ctrl+Enter, Escape and arrows.
        KeyBinding::new("secondary-enter", SaveNote, Some("NoteEdit > Input")),
        KeyBinding::new("escape", Dismiss, Some("NoteEdit > Input")),
        KeyBinding::new("secondary-enter", SaveNote, Some("Dialog")),
        KeyBinding::new("secondary-enter", SaveNote, Some("Dialog > Input")),
        KeyBinding::new("escape", Dismiss, Some("Dialog > Input")),
        KeyBinding::new("escape", Dismiss, Some("SheetName > Input")),
        KeyBinding::new("escape", Dismiss, Some("LabelEdit > Input")),
        KeyBinding::new("secondary-enter", Dismiss, Some("LabelEdit > Input")),
        KeyBinding::new("up", PaletteUp, Some("Palette > Input")),
        KeyBinding::new("down", PaletteDown, Some("Palette > Input")),
        KeyBinding::new("tab", PaletteTab, Some("Palette > Input")),
        KeyBinding::new("shift-tab", PaletteBackTab, Some("Palette > Input")),
        KeyBinding::new("escape", Dismiss, Some("Palette > Input")),
        KeyBinding::new("escape", Dismiss, Some("QuickNote > Input")),
        KeyBinding::new("escape", Dismiss, Some("QuickNote")),
    ]);
}

fn main() {
    // Started with Windows: open, then tuck the window away.
    let background = std::env::args().any(|a| a == "--background");
    gpui::application().with_assets(assets::Assets).run(move |cx: &mut App| {
        gpui::init(cx);
        assets::load_fonts(cx);
        fonts::choose(cx);
        theme::set(false, false, 1.0, cx);
        bind_keys(cx);
        cx.on_action(|_: &Quit, cx| cx.quit());

        let bounds = Bounds::centered(None, size(px(1280.), px(800.)), cx);
        let options = WindowOptions {
            window_bounds: Some(WindowBounds::Windowed(bounds)),
            window_min_size: Some(size(px(760.), px(520.))),
            app_id: Some("folio".into()),
            // No system title bar: Folio's top bar is the title bar, with its
            // own minimize / maximize / close (see ui/topbar.rs). The title
            // still names the window in the taskbar and Alt+Tab.
            titlebar: Some(gpui::TitlebarOptions {
                title: Some("Folio".into()),
                appears_transparent: true,
                traffic_light_position: Some(gpui::point(px(14.), px(22.))),
            }),
            window_decorations: Some(gpui::WindowDecorations::Client),
            app_owns_titlebar_drag: true,
            ..Default::default()
        };
        let (handle, folio) = gpui::open_window(options, cx, |window, cx| cx.new(|cx| ui::app::Folio::new(window, cx)))
            .expect("the window opens");
        desktop::start(&folio, handle, cx);
        if background {
            let _ = handle.update(cx, |_, window, _| window.minimize_window());
        } else {
            cx.activate(true);
        }
    });
}
