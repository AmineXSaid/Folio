//! Folio outside its window: the Ctrl+Alt+N quick note, notes pinned to the
//! desktop (always-on-top windows), the Windows tray icon, starting with
//! Windows, and reminder notifications.

use std::collections::HashMap;
use std::time::Duration;

use gpui::{px, size, AnyWindowHandle, App, AppContext, Bounds, Entity, Point, WindowBounds, WindowKind, WindowOptions};

use crate::model::{Id, Popped};
use crate::ui::app::Folio;
use crate::ui::popout::NoteWindow;
use crate::ui::quick::QuickNoteWindow;

#[derive(Default)]
pub struct Desktop {
    hotkey: Option<(global_hotkey::GlobalHotKeyManager, global_hotkey::hotkey::HotKey)>,
    hotkey_on: bool,
    /// Note windows on the desktop, by note (None while one opens).
    pub notes: HashMap<Id, Option<AnyWindowHandle>>,
    pub quick: Option<AnyWindowHandle>,
    /// The main window, for the tray to bring back.
    pub main: Option<AnyWindowHandle>,
    #[cfg(windows)]
    tray: Option<tray::Tray>,
}

impl Desktop {
    /// Registers or releases Ctrl+Alt+N.
    pub fn set_hotkey(&mut self, on: bool) {
        use global_hotkey::hotkey::{Code, HotKey, Modifiers};
        if on == self.hotkey_on && (self.hotkey.is_some() || !on) {
            return;
        }
        if self.hotkey.is_none() {
            if let Ok(m) = global_hotkey::GlobalHotKeyManager::new() {
                self.hotkey = Some((m, HotKey::new(Some(Modifiers::CONTROL | Modifiers::ALT), Code::KeyN)));
            }
        }
        if let Some((m, k)) = &self.hotkey {
            let r = if on { m.register(*k) } else { m.unregister(*k) };
            self.hotkey_on = on && r.is_ok();
            if let Err(e) = r {
                eprintln!("quick-note shortcut: {e}");
            }
        }
    }
}

/// Starts the shortcut, the tray icon and the loop that listens to both.
pub fn start(folio: &Entity<Folio>, main: AnyWindowHandle, cx: &mut App) {
    let hotkey = folio.read(cx).board.settings.quick_hotkey;
    folio.update(cx, |f, cx| {
        f.desktop.main = Some(main);
        f.desktop.set_hotkey(hotkey);
        #[cfg(windows)]
        {
            f.desktop.tray = tray::Tray::new();
        }
        f.desktop_sync(cx);
    });
    let weak = folio.downgrade();
    cx.spawn(async move |cx| loop {
        cx.background_executor().timer(Duration::from_millis(120)).await;
        let Some(folio) = weak.upgrade() else { return };
        let pressed = std::iter::from_fn(|| global_hotkey::GlobalHotKeyEvent::receiver().try_recv().ok())
            .any(|e| e.state == global_hotkey::HotKeyState::Pressed);
        let _ = cx.update(|cx| {
            if pressed {
                open_quick_note(folio.clone(), cx);
            }
            #[cfg(windows)]
            tray::poll(&folio, cx);
        });
    })
    .detach();
}

/// The quick-note window: a small box above everything, centred near the
/// top of the screen. A second press brings the open one forward.
pub fn open_quick_note(folio: Entity<Folio>, cx: &mut App) {
    if let Some(h) = folio.read(cx).desktop.quick {
        if h.update(cx, |_, window, _| window.activate_window()).is_ok() {
            return;
        }
    }
    let display = cx.primary_display();
    let screen = display.as_ref().map(|d| d.bounds()).unwrap_or(Bounds::new(Point::default(), size(px(1280.), px(800.))));
    let (w, h) = (px(520.), px(216.));
    let origin = Point::new(screen.origin.x + (screen.size.width - w) / 2., screen.origin.y + screen.size.height * 0.22);
    let options = WindowOptions {
        window_bounds: Some(WindowBounds::Windowed(Bounds::new(origin, size(w, h)))),
        titlebar: None,
        kind: WindowKind::PopUp,
        is_movable: true,
        is_resizable: false,
        is_minimizable: false,
        focus: true,
        show: true,
        app_id: Some("folio".into()),
        ..Default::default()
    };
    let f = folio.clone();
    if let Ok((handle, _)) = gpui::open_window(options, cx, move |window, cx| cx.new(|cx| QuickNoteWindow::new(f, window, cx))) {
        folio.update(cx, |f, _| f.desktop.quick = Some(handle));
        let _ = handle.update(cx, |_, window, _| window.activate_window());
    }
}

impl Folio {
    /// Pins note `id` to the desktop in a window of its own.
    pub fn pop_out(&mut self, id: Id, window: &gpui::Window, cx: &mut gpui::Context<Self>) {
        self.menu = None;
        let Some(s) = self.board.find_note(id) else { return };
        let r = self.note_rect(s);
        // Start near where the note is, on screen.
        let cam = self.sheet().camera;
        let at = cam.to_screen(crate::model::v2(r.x, r.y));
        let origin = self.viewport.get().origin;
        let wb = window.bounds();
        let x = f32::from(wb.origin.x + origin.x) + at.x + 24.;
        let y = f32::from(wb.origin.y + origin.y) + at.y + 24.;
        let (w, h) = (r.w.clamp(200., 420.), (r.h + 40.).clamp(140., 520.));
        self.change_note(id, cx, |s| s.popped = Some(Popped { x, y, w, h }));
        self.notify("Pinned to the desktop", cx);
    }

    /// Takes note `id` off the desktop.
    pub fn dock(&mut self, id: Id, cx: &mut gpui::Context<Self>) {
        self.change_note(id, cx, |s| s.popped = None);
    }

    /// Opens a desktop window for each popped note that has none, and
    /// closes the windows of notes no longer popped (or gone).
    pub fn desktop_sync(&mut self, cx: &mut gpui::Context<Self>) {
        let want: Vec<(Id, Popped)> = self
            .board
            .categories
            .iter()
            .flat_map(|c| c.stickers.iter())
            .filter_map(|s| s.popped.map(|p| (s.id, p)))
            .collect();
        let stale: Vec<Id> = self.desktop.notes.keys().filter(|id| !want.iter().any(|w| w.0 == **id)).copied().collect();
        let me = cx.entity();
        for id in stale {
            if let Some(Some(h)) = self.desktop.notes.remove(&id) {
                cx.defer(move |cx| {
                    let _ = h.update(cx, |_, window, _| window.remove_window());
                });
            }
        }
        for (id, p) in want {
            if self.desktop.notes.contains_key(&id) {
                continue;
            }
            let folio = me.clone();
            let options = WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(Bounds::new(
                    Point::new(px(p.x), px(p.y)),
                    size(px(p.w), px(p.h)),
                ))),
                titlebar: None,
                kind: WindowKind::PopUp,
                is_movable: true,
                is_resizable: false,
                is_minimizable: false,
                focus: false,
                show: true,
                app_id: Some("folio".into()),
                ..Default::default()
            };
            // Opened after this update, so the window can read the board.
            let me2 = me.clone();
            cx.defer(move |cx| {
                let made = gpui::open_window(options, cx, move |window, cx| cx.new(|cx| NoteWindow::new(folio, id, window, cx)));
                if let Ok((h, _)) = made {
                    me2.update(cx, |f, _| {
                        if f.desktop.notes.contains_key(&id) {
                            f.desktop.notes.insert(id, Some(h));
                        } else {
                            // Docked again before it opened.
                            let _ = h;
                        }
                    });
                }
            });
            // Marked now, so a second sync in the same update doesn't open twice.
            self.desktop.notes.insert(id, None);
        }
    }
}

/// Starts Folio when you sign in to Windows (the Run key), or stops it.
pub fn set_start_with_windows(on: bool) -> Result<(), String> {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const NO_WINDOW: u32 = 0x0800_0000;
        let key = r"HKCU\Software\Microsoft\Windows\CurrentVersion\Run";
        let exe = std::env::current_exe().map_err(|e| e.to_string())?;
        let status = if on {
            let value = format!("\"{}\" --background", exe.display());
            std::process::Command::new("reg")
                .args(["add", key, "/v", "Folio", "/t", "REG_SZ", "/d", &value, "/f"])
                .creation_flags(NO_WINDOW)
                .status()
        } else {
            std::process::Command::new("reg").args(["delete", key, "/v", "Folio", "/f"]).creation_flags(NO_WINDOW).status()
        };
        return match status {
            Ok(s) if s.success() || !on => Ok(()),
            _ => Err("Couldn't change the Windows start-up setting".into()),
        };
    }
    #[cfg(target_os = "linux")]
    {
        let Some(home) = std::env::var_os("HOME") else { return Err("No home folder".into()) };
        let dir = std::path::PathBuf::from(home).join(".config/autostart");
        let file = dir.join("folio.desktop");
        if on {
            let exe = std::env::current_exe().map_err(|e| e.to_string())?;
            std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
            let text = format!("[Desktop Entry]\nType=Application\nName=Folio\nExec=\"{}\" --background\n", exe.display());
            std::fs::write(file, text).map_err(|e| e.to_string())?;
        } else {
            let _ = std::fs::remove_file(file);
        }
        return Ok(());
    }
    #[allow(unreachable_code)]
    Err("Starting with the system isn't available here".into())
}

/// A system notification. Best effort: nothing happens where there's no
/// notification service.
pub fn notify(title: &str, body: &str) {
    #[cfg(windows)]
    {
        use tauri_winrt_notification::Toast;
        let _ = Toast::new(Toast::POWERSHELL_APP_ID).title(title).text1(body).show();
    }
    #[cfg(target_os = "linux")]
    {
        let _ = std::process::Command::new("notify-send").args(["-a", "Folio", title, body]).spawn();
    }
}

#[cfg(windows)]
mod tray {
    //! The tray icon: click to open Folio; menu for a quick note or quitting.
    use gpui::{App, Entity};
    use tray_icon::menu::{Menu, MenuEvent, MenuItem, PredefinedMenuItem};
    use tray_icon::{Icon, MouseButton, TrayIcon, TrayIconBuilder, TrayIconEvent};

    use crate::ui::app::Folio;

    pub struct Tray {
        _icon: TrayIcon,
        open: tray_icon::menu::MenuId,
        quick: tray_icon::menu::MenuId,
        quit: tray_icon::menu::MenuId,
    }

    impl Tray {
        pub fn new() -> Option<Tray> {
            let png = include_bytes!("../assets/brand/folio-icon-32.png");
            let img = image::load_from_memory(png).ok()?.into_rgba8();
            let (w, h) = img.dimensions();
            let icon = Icon::from_rgba(img.into_raw(), w, h).ok()?;
            let open = MenuItem::new("Open Folio", true, None);
            let quick = MenuItem::new("New quick note\tCtrl+Alt+N", true, None);
            let quit = MenuItem::new("Quit Folio", true, None);
            let menu = Menu::new();
            menu.append_items(&[&open, &quick, &PredefinedMenuItem::separator(), &quit]).ok()?;
            let tray = TrayIconBuilder::new()
                .with_tooltip("Folio")
                .with_icon(icon)
                .with_menu(Box::new(menu))
                .build()
                .ok()?;
            Some(Tray { _icon: tray, open: open.id().clone(), quick: quick.id().clone(), quit: quit.id().clone() })
        }
    }

    pub fn poll(folio: &Entity<Folio>, cx: &mut App) {
        while let Ok(e) = TrayIconEvent::receiver().try_recv() {
            if let TrayIconEvent::Click { button: MouseButton::Left, .. } = e {
                show_main(folio, cx);
            }
        }
        let (open, quick, quit) = match folio.read(cx).desktop.tray.as_ref() {
            Some(t) => (t.open.clone(), t.quick.clone(), t.quit.clone()),
            None => return,
        };
        while let Ok(e) = MenuEvent::receiver().try_recv() {
            if e.id == open {
                show_main(folio, cx);
            } else if e.id == quick {
                super::open_quick_note(folio.clone(), cx);
            } else if e.id == quit {
                folio.update(cx, |f, cx| f.save_now(cx));
                cx.quit();
            }
        }
    }

    fn show_main(folio: &Entity<Folio>, cx: &mut App) {
        if let Some(w) = folio.read(cx).desktop.main {
            let _ = w.update(cx, |_, window, _| window.activate_window());
        }
    }
}
