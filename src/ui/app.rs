//! The main window's view: the board, which screen is showing, the tool in
//! hand, the gesture under way, the note being typed into, menus, dialogs,
//! undo history and the debounced save. The other files in `ui/` each
//! render one part of it.

use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::path::PathBuf;
use std::rc::Rc;
use std::time::{Duration, Instant};

use gpui::component::input::{InputEvent, InputState, TextareaState};
use gpui::{
    div, prelude::*, Bounds, ClipboardEntry, ClipboardItem, Context, Entity, FocusHandle, Pixels, Point,
    SharedString, Task, Window,
};

use crate::model::*;
use crate::store;
use crate::theme::{self, theme};
use crate::time::{self, now};
use crate::ui::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tool {
    Select,
    Hand,
    Pen,
    Marker,
    Text,
    Connect,
    Frame,
    Eraser,
}

impl Tool {
    pub const ALL: [Tool; 8] =
        [Tool::Select, Tool::Hand, Tool::Pen, Tool::Marker, Tool::Text, Tool::Connect, Tool::Frame, Tool::Eraser];
    pub fn icon(self) -> &'static str {
        match self {
            Tool::Select => "icons/mouse-pointer-2.svg",
            Tool::Hand => "icons/hand.svg",
            Tool::Pen => "icons/pencil.svg",
            Tool::Marker => "icons/highlighter.svg",
            Tool::Text => "icons/type.svg",
            Tool::Connect => "icons/spline.svg",
            Tool::Frame => "icons/square-dashed.svg",
            Tool::Eraser => "icons/eraser.svg",
        }
    }
    pub fn name(self) -> &'static str {
        match self {
            Tool::Select => "Select",
            Tool::Hand => "Pan",
            Tool::Pen => "Pen",
            Tool::Marker => "Marker",
            Tool::Text => "Text",
            Tool::Connect => "Connect",
            Tool::Frame => "Area",
            Tool::Eraser => "Eraser",
        }
    }
    pub fn key(self) -> &'static str {
        match self {
            Tool::Select => "V",
            Tool::Hand => "H",
            Tool::Pen => "P",
            Tool::Marker => "M",
            Tool::Text => "T",
            Tool::Connect => "C",
            Tool::Frame => "F",
            Tool::Eraser => "E",
        }
    }
    /// Tools that put ink down, so the ink swatches show beside them.
    pub fn uses_ink(self) -> bool {
        matches!(self, Tool::Pen | Tool::Marker | Tool::Text | Tool::Connect)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Selection {
    Sticker(Id),
    Label(Id),
    Frame(Id),
}

/// The opening, playing (every launch).
pub struct Intro {
    pub start: Instant,
}

/// Somewhere you can go back to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Place {
    Home,
    Sheet(Id),
    Settings,
    Bin(BinTab),
    Transfer,
}

/// How many places back and forward remember.
const NAV_KEPT: usize = 50;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Screen {
    Home,
    Sheet,
    Settings,
    Bin,
    /// Sync & export.
    Transfer,
}

/// What the pointer is doing between press and release.
pub enum Gesture {
    None,
    Pan { start: Vec2, pan: Vec2 },
    /// Moving the selection; each item with where it started.
    Move { items: Vec<(Selection, Vec2)>, start: Vec2, moved: bool },
    /// Resizing a note from `grip`'s edges; `rect` as it was at the press.
    Resize { id: Id, start: Vec2, rect: Rect, grip: Grip },
    ResizeFrame { id: Id, start: Vec2, rect: Rect, grip: Grip },
    Marquee { start: Vec2, now: Vec2, additive: bool },
    Draw(Stroke),
    Erase { erased: bool },
    Connect { from: Id, to: Vec2 },
    MakeFrame { start: Vec2, now: Vec2 },
}

/// The note being typed into, on the sheet.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Edit {
    pub id: Id,
    /// Made for this edit: an empty one goes away again.
    pub is_new: bool,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum MenuKind {
    Note(Id),
    /// On the empty sheet, at this world point.
    Canvas(Vec2),
    /// A sheet's card on the home screen (sheet index).
    Card(usize),
    Frame(Id),
    Label(Id),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MenuPage {
    Main,
    MoveTo,
    Remind,
    Icon,
    Colour,
}

pub struct Menu {
    /// Window position.
    pub at: Point<Pixels>,
    pub kind: MenuKind,
    pub page: MenuPage,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Dialog {
    NewSheet { icon: &'static str, template: Template },
    RenameSheet(Id),
    RenameFrame(Id),
    /// The first sync, and both sides have notes: merge, or keep one.
    DriveChoice(super::sync::Pick),
    /// Confirming "replace Drive with this computer's" (or the reverse).
    DriveReplace(crate::cloud::Keep),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Sort {
    Recent,
    Name,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BinTab {
    Trash,
    Archive,
    Backups,
}

pub struct Folio {
    pub board: Board,
    pub dir: PathBuf,
    /// The sheet's focus (its shortcuts work while it has it).
    pub focus: FocusHandle,
    /// The window's own focus, off the sheet: the app-wide shortcuts.
    pub root_focus: FocusHandle,
    pub screen: Screen,
    /// Where a side screen (Settings, Trash, Sync & export) goes back to.
    pub back_to: Screen,
    /// Back and forward through the places you were.
    pub nav_back: Vec<Place>,
    pub nav_forward: Vec<Place>,
    last_place: Option<Place>,
    /// The next place change is a step back or forward, not a new visit.
    nav_stepping: bool,
    pub drawer: bool,
    pub tool: Tool,
    pub ink: Ink,
    /// The ink palette (every ink) is open above the dock.
    pub ink_palette: bool,
    pub gesture: Gesture,
    /// Alignment guides while moving: (vertical?, world coordinate).
    pub guides: Vec<(bool, f32)>,
    /// Notes just dropped, and when: they settle with a small bounce.
    pub settling: Vec<(Id, Instant)>,
    /// Dragging notes over the left edge: the tray of sheets, and the one
    /// under the pointer.
    pub drop_tray: bool,
    /// The top bar was pressed: moving now moves the window (Linux; on
    /// Windows the system does it).
    pub bar_drag: bool,
    pub drop_sheet: Option<usize>,
    /// The undo snapshot taken when the current gesture began.
    pub before: Option<Board>,
    pub selection: Vec<Selection>,
    pub hovered: Option<Id>,
    /// Space is held: drag pans.
    pub space: bool,
    /// The last pointer position over the sheet, in sheet-local pixels.
    pub cursor: Vec2,
    /// Where the sheet was laid out last frame, for turning window
    /// positions into sheet positions.
    pub viewport: Rc<Cell<Bounds<Pixels>>>,
    /// Each note's natural height (world units), measured as it's drawn.
    pub heights: Rc<RefCell<HashMap<Id, f32>>>,
    pub undo: Vec<Board>,
    pub redo: Vec<Board>,
    pub editing: Option<Edit>,
    pub menu: Option<Menu>,
    pub dialog: Option<Dialog>,
    pub palette: bool,
    pub palette_ix: usize,
    /// Counts openings, so each plays its arrival again.
    pub palette_opened: usize,
    /// Commands and sheets run from the search lately, newest first.
    pub palette_recent: Vec<SharedString>,
    pub palette_scroll: gpui::ScrollHandle,
    /// The selection moved by key: scroll it into view next frame.
    pub palette_reveal: bool,
    /// The emoji picker's tab.
    pub emoji_tab: usize,
    pub intro: Option<Intro>,
    /// The welcome (your name, your avatar), first launch or on request.
    pub welcome: Option<super::welcome::Welcome>,
    pub welcome_input: Entity<InputState>,
    pub sort: Sort,
    pub bin_tab: BinTab,
    /// The backup whose sheets are listed: (day, [(id, name, notes, icon)]).
    pub backup_open: Option<(String, Vec<(Id, String, usize, String)>)>,
    pub toast: Option<SharedString>,
    pub saved: bool,
    /// The OS's light/dark, when the theme follows it.
    pub system_dark: bool,
    pub title_input: Entity<InputState>,
    pub body_input: Entity<TextareaState>,
    pub search_input: Entity<InputState>,
    pub palette_input: Entity<InputState>,
    pub dialog_input: Entity<InputState>,
    /// The sheet's name while it's being renamed in place.
    pub name_input: Entity<InputState>,
    pub naming: bool,
    /// Text being written straight on the sheet.
    pub label_edit: Option<Id>,
    pub label_input: Entity<TextareaState>,
    pub desktop: crate::desktop::Desktop,
    pub sync: super::sync::DriveSync,
    pub export: super::transfer::ExportChoice,
    toast_task: Option<Task<()>>,
    save_task: Option<Task<()>>,
    _ticker: Task<()>,
    _subs: Vec<gpui::Subscription>,
}

const UNDO_DEPTH: usize = 100;

impl Folio {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let dir = store::data_dir();
        let mut board = store::load(&dir);
        let t = now();
        board.purge(t);
        let _ = store::backup_daily(&dir, &board, &time::today());
        let title_input = cx.new(|cx| InputState::new(window, cx).placeholder("Title"));
        let body_input = cx.new(|cx| {
            TextareaState::new(window, cx).auto_grow(3, 24).placeholder("Write… - [ ] for a task, #tag, **bold**")
        });
        let search_input = cx.new(|cx| InputState::new(window, cx).placeholder("Search sheets and notes"));
        let palette_input =
            cx.new(|cx| InputState::new(window, cx).placeholder("Type a command, a sheet or a note…"));
        let dialog_input = cx.new(|cx| InputState::new(window, cx).placeholder("Name"));
        let name_input = cx.new(|cx| InputState::new(window, cx).placeholder("Name this sheet"));
        let welcome_input = cx.new(|cx| InputState::new(window, cx).placeholder("Your name"));
        let label_input = cx.new(|cx| TextareaState::new(window, cx).auto_grow(1, 30).placeholder("Write…"));
        let subs = vec![
            cx.observe(&search_input, |this, _, cx| {
                if this.screen != Screen::Home && !this.search_text(cx).is_empty() {
                    this.screen = Screen::Home;
                    this.board.at_home = true;
                }
                cx.notify()
            }),
            cx.subscribe_in(&palette_input, window, |this, _, ev, window, cx| match ev {
                InputEvent::Change => {
                    this.palette_ix = 0;
                    cx.notify();
                }
                InputEvent::PressEnter { .. } => this.palette_run(this.palette_ix, window, cx),
                _ => {}
            }),
            // The sheet's name, typed over its title: Enter or clicking away
            // keeps it.
            cx.subscribe_in(&name_input, window, |this, _, ev, window, cx| match ev {
                InputEvent::PressEnter { .. } => {
                    this.finish_naming(true, cx);
                    window.focus(&this.focus, cx);
                }
                InputEvent::Blur => this.finish_naming(true, cx),
                _ => {}
            }),
            // Writing on the sheet: clicking away keeps it.
            cx.subscribe_in(&label_input, window, |this, _, ev, _, cx| {
                if matches!(ev, InputEvent::Blur) {
                    this.finish_label(cx);
                }
            }),
            cx.subscribe_in(&welcome_input, window, |this, _, ev, window, cx| match ev {
                InputEvent::PressEnter { .. } => this.welcome_next(window, cx),
                InputEvent::Change => cx.notify(),
                _ => {}
            }),
            cx.subscribe_in(&dialog_input, window, |this, _, ev, window, cx| {
                if let InputEvent::PressEnter { .. } = ev {
                    this.dialog_ok(window, cx);
                }
            }),
            cx.observe_window_appearance(window, |this, window, cx| {
                this.system_dark = is_dark(window.appearance());
                this.apply_theme(cx);
            }),
        ];
        let focus = cx.focus_handle();
        let root_focus = cx.focus_handle();
        window.focus(&root_focus, cx);

        let s = board.settings.clone();
        let screen = if s.open_on == OpenOn::LastSheet && !board.at_home { Screen::Sheet } else { Screen::Home };
        board.at_home = screen == Screen::Home;

        // Reminders: checked every 15 seconds.
        let ticker = cx.spawn(async move |this, cx| loop {
            cx.background_executor().timer(Duration::from_secs(15)).await;
            if this
                .update(cx, |this, cx| {
                    this.check_reminders(cx);
                    this.drive_tick(cx);
                    // The home greeting moves on with the clock.
                    if this.screen == Screen::Home {
                        cx.notify();
                    }
                })
                .is_err()
            {
                return;
            }
        });

        let mut f = Folio {
            board,
            dir,
            focus,
            root_focus,
            screen,
            back_to: screen,
            nav_back: Vec::new(),
            nav_forward: Vec::new(),
            last_place: None,
            nav_stepping: false,
            drawer: false,
            tool: Tool::Select,
            ink: Ink::Graphite,
            ink_palette: false,
            gesture: Gesture::None,
            guides: Vec::new(),
            settling: Vec::new(),
            drop_tray: false,
            bar_drag: false,
            drop_sheet: None,
            before: None,
            selection: Vec::new(),
            hovered: None,
            space: false,
            cursor: v2(0., 0.),
            viewport: Rc::new(Cell::new(Bounds::default())),
            heights: Rc::new(RefCell::new(HashMap::new())),
            undo: store::load_history(&store::data_dir()),
            redo: Vec::new(),
            editing: None,
            menu: None,
            dialog: None,
            palette: false,
            palette_ix: 0,
            palette_opened: 0,
            palette_recent: Vec::new(),
            palette_scroll: gpui::ScrollHandle::new(),
            palette_reveal: false,
            emoji_tab: 0,
            // The opening plays in full at every launch.
            intro: Some(Intro { start: Instant::now() }),
            welcome: None,
            welcome_input,
            sort: Sort::Recent,
            bin_tab: BinTab::Trash,
            backup_open: None,
            toast: None,
            saved: true,
            system_dark: is_dark(window.appearance()),
            title_input,
            body_input,
            search_input,
            palette_input,
            dialog_input,
            name_input,
            naming: false,
            label_edit: None,
            label_input,
            desktop: crate::desktop::Desktop::default(),
            sync: Default::default(),
            export: Default::default(),
            toast_task: None,
            save_task: None,
            _ticker: ticker,
            _subs: subs,
        };
        f.apply_theme(cx);
        f.schedule_save(cx);
        f.drive_start(cx);
        // First launch: who are you?
        if f.board.settings.user_name.trim().is_empty() {
            f.open_welcome(false, window, cx);
        }
        // However Folio ends (Quit, the tray, the last window closed): the
        // board saved, and a last push to Google Drive.
        f._subs.push(cx.on_app_quit(|this, cx| {
            this.save_now(cx);
            this.drive_flush();
            async {}
        }));
        f
    }

    pub fn sheet(&self) -> &Category {
        self.board.sheet()
    }

    pub fn viewport_size(&self) -> (f32, f32) {
        let b = self.viewport.get();
        (f32::from(b.size.width).max(1.), f32::from(b.size.height).max(1.))
    }

    /// A window position as a sheet-local screen position.
    pub fn local(&self, p: Point<Pixels>) -> Vec2 {
        let o = self.viewport.get().origin;
        v2(f32::from(p.x - o.x), f32::from(p.y - o.y))
    }

    pub fn search_text(&self, cx: &gpui::App) -> String {
        self.search_input.read(cx).value().trim().to_lowercase()
    }

    /// A note's rectangle as drawn: as tall as its text when it grows.
    pub fn note_rect(&self, s: &Sticker) -> Rect {
        let mut r = s.rect;
        if !s.fixed {
            if let Some(h) = self.heights.borrow().get(&s.id) {
                r.h = r.h.max(*h);
            }
        }
        r
    }

    /// The front-most note under `p` (world), as drawn.
    pub fn note_at(&self, p: Vec2) -> Option<Id> {
        self.sheet().stickers.iter().rev().find(|s| self.note_rect(s).contains(p)).map(|s| s.id)
    }

    pub fn is_selected(&self, s: Selection) -> bool {
        self.selection.contains(&s)
    }

    pub fn selected_notes(&self) -> Vec<Id> {
        self.selection
            .iter()
            .filter_map(|s| match s {
                Selection::Sticker(id) => Some(*id),
                _ => None,
            })
            .collect()
    }

    pub fn select(&mut self, s: Selection) {
        self.selection = vec![s];
    }

    // ---------- Theme ----------

    pub fn dark(&self) -> bool {
        match self.board.settings.theme {
            ThemeMode::Light => false,
            ThemeMode::Dark => true,
            ThemeMode::System => self.system_dark,
        }
    }

    pub fn apply_theme(&mut self, cx: &mut Context<Self>) {
        let s = &self.board.settings;
        super::motion::set_reduced(s.reduce_motion);
        theme::set(self.dark(), s.high_contrast, s.ui_scale, cx);
        sync_component_theme(cx);
        cx.refresh_windows();
    }

    pub fn toggle_theme(&mut self, cx: &mut Context<Self>) {
        self.board.settings.theme = if self.dark() { ThemeMode::Light } else { ThemeMode::Dark };
        self.apply_theme(cx);
        self.schedule_save(cx);
    }

    // ---------- History and saving ----------

    /// Records `before` as an undo step, and saves soon.
    pub fn commit(&mut self, before: Board, cx: &mut Context<Self>) {
        if before == self.board {
            return;
        }
        // Whatever changed was edited now.
        let t = now();
        let active = self.board.active;
        if let Some(c) = self.board.categories.get_mut(active) {
            c.edited = t;
        }
        self.undo.push(before);
        if self.undo.len() > UNDO_DEPTH {
            self.undo.remove(0);
        }
        self.redo.clear();
        self.schedule_save(cx);
        self.desktop_sync(cx);
    }

    /// Runs `f` on the board as one undoable step.
    pub fn change(&mut self, cx: &mut Context<Self>, f: impl FnOnce(&mut Board)) {
        let before = self.board.clone();
        f(&mut self.board);
        self.commit(before, cx);
        cx.notify();
    }

    /// Puts `snapshot` in place, keeping each sheet's camera and the
    /// settings: undo is about content.
    fn restore_snapshot(&mut self, mut snapshot: Board) -> Board {
        for c in &mut snapshot.categories {
            if let Some(now) = self.board.categories.iter().find(|n| n.id == c.id) {
                c.camera = now.camera;
            }
        }
        snapshot.settings = self.board.settings.clone();
        snapshot.at_home = self.board.at_home;
        self.selection.clear();
        std::mem::replace(&mut self.board, snapshot)
    }

    pub fn undo(&mut self, cx: &mut Context<Self>) {
        if self.editing.is_some() {
            return;
        }
        if let Some(prev) = self.undo.pop() {
            let now = self.restore_snapshot(prev);
            self.redo.push(now);
            self.schedule_save(cx);
            self.desktop_sync(cx);
            cx.notify();
        } else {
            self.notify("Nothing to undo", cx);
        }
    }

    pub fn redo(&mut self, cx: &mut Context<Self>) {
        if let Some(next) = self.redo.pop() {
            let now = self.restore_snapshot(next);
            self.undo.push(now);
            self.schedule_save(cx);
            self.desktop_sync(cx);
            cx.notify();
        }
    }

    pub fn schedule_save(&mut self, cx: &mut Context<Self>) {
        self.saved = false;
        self.save_task = Some(cx.spawn(async move |this, cx| {
            cx.background_executor().timer(Duration::from_millis(350)).await;
            let _ = this.update(cx, |this, cx| this.save_now(cx));
        }));
    }

    pub fn save_now(&mut self, cx: &mut Context<Self>) {
        self.save_task = None;
        let _ = store::save_history(&self.dir, &self.undo);
        match store::save(&self.dir, &self.board) {
            Ok(()) => {
                self.saved = true;
                self.drive_after_save(cx);
            }
            Err(e) => self.notify(format!("Couldn't save: {e}"), cx),
        }
        cx.notify();
    }

    pub fn notify(&mut self, msg: impl Into<SharedString>, cx: &mut Context<Self>) {
        self.toast = Some(msg.into());
        self.toast_task = Some(cx.spawn(async move |this, cx| {
            cx.background_executor().timer(Duration::from_millis(2600)).await;
            let _ = this.update(cx, |this, cx| {
                this.toast = None;
                cx.notify();
            });
        }));
        cx.notify();
    }

    // ---------- Screens ----------

    pub fn go_home(&mut self, cx: &mut Context<Self>) {
        self.finish_edit(cx);
        self.screen = Screen::Home;
        self.board.at_home = true;
        self.drawer = false;
        self.selection.clear();
        self.schedule_save(cx);
        cx.notify();
    }

    pub fn open_sheet(&mut self, ix: usize, window: &mut Window, cx: &mut Context<Self>) {
        if ix >= self.board.categories.len() {
            return;
        }
        self.finish_edit(cx);
        if ix != self.board.active {
            self.selection.clear();
            self.gesture = Gesture::None;
        }
        self.board.active = ix;
        self.board.categories[ix].opened = now();
        self.board.at_home = false;
        self.screen = Screen::Sheet;
        self.drawer = false;
        self.search_input.update(cx, |i, cx| i.set_value("", window, cx));
        window.focus(&self.focus, cx);
        self.schedule_save(cx);
        cx.notify();
    }

    /// Settings, Trash and Sync & export open over where you were; going
    /// to the one already open takes you back.
    pub fn open_side(&mut self, to: Screen, cx: &mut Context<Self>) {
        self.finish_edit(cx);
        self.drawer = false;
        self.menu = None;
        if self.screen == to {
            self.close_side(cx);
            return;
        }
        if matches!(self.screen, Screen::Home | Screen::Sheet) {
            self.back_to = self.screen;
        }
        self.screen = to;
        cx.notify();
    }

    /// Where you are now.
    pub fn place(&self) -> Place {
        match self.screen {
            Screen::Home => Place::Home,
            Screen::Sheet => Place::Sheet(self.sheet().id),
            Screen::Settings => Place::Settings,
            Screen::Bin => Place::Bin(self.bin_tab),
            Screen::Transfer => Place::Transfer,
        }
    }

    /// Notes a move to a new place (each frame; only changes count).
    fn track_place(&mut self) {
        let here = self.place();
        match self.last_place {
            Some(was) if was != here => {
                if !std::mem::take(&mut self.nav_stepping) {
                    self.nav_back.push(was);
                    if self.nav_back.len() > NAV_KEPT {
                        self.nav_back.remove(0);
                    }
                    self.nav_forward.clear();
                }
            }
            _ => self.nav_stepping = false,
        }
        self.last_place = Some(here);
    }

    fn go_to(&mut self, p: Place, window: &mut Window, cx: &mut Context<Self>) -> bool {
        self.finish_edit(cx);
        self.drawer = false;
        self.menu = None;
        match p {
            Place::Home => self.go_home(cx),
            Place::Sheet(id) => match self.board.categories.iter().position(|c| c.id == id) {
                Some(ix) => self.open_sheet(ix, window, cx),
                // A sheet since deleted: skip it.
                None => return false,
            },
            Place::Settings => self.screen = Screen::Settings,
            Place::Bin(tab) => {
                self.bin_tab = tab;
                self.screen = Screen::Bin;
            }
            Place::Transfer => self.screen = Screen::Transfer,
        }
        cx.notify();
        true
    }

    pub fn go_back(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let here = self.place();
        while let Some(p) = self.nav_back.pop() {
            self.nav_stepping = true;
            if self.go_to(p, window, cx) {
                self.nav_forward.push(here);
                self.last_place = Some(self.place());
                return;
            }
        }
        self.nav_stepping = false;
    }

    pub fn go_forward(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let here = self.place();
        while let Some(p) = self.nav_forward.pop() {
            self.nav_stepping = true;
            if self.go_to(p, window, cx) {
                self.nav_back.push(here);
                self.last_place = Some(self.place());
                return;
            }
        }
        self.nav_stepping = false;
    }

    /// Back from Settings, Trash or Sync & export to the sheet or home.
    pub fn close_side(&mut self, cx: &mut Context<Self>) {
        self.screen = if self.back_to == Screen::Sheet && !self.board.categories.is_empty() { Screen::Sheet } else { Screen::Home };
        self.board.at_home = self.screen == Screen::Home;
        cx.notify();
    }

    pub fn open_settings(&mut self, cx: &mut Context<Self>) {
        self.open_side(Screen::Settings, cx);
    }

    pub fn open_bin(&mut self, tab: BinTab, cx: &mut Context<Self>) {
        if self.screen == Screen::Bin && self.bin_tab != tab {
            self.bin_tab = tab;
            cx.notify();
            return;
        }
        self.open_side(Screen::Bin, cx);
        self.bin_tab = tab;
        self.drawer = false;
        cx.notify();
    }

    /// Opens the sheet holding note `id`, frames the note and selects it.
    pub fn reveal_note(&mut self, id: Id, window: &mut Window, cx: &mut Context<Self>) {
        let Some(ix) = self.board.sheet_of(id) else { return };
        self.open_sheet(ix, window, cx);
        let (w, h) = self.viewport_size();
        let (w, h) = if w < 10. { (1000., 640.) } else { (w, h) };
        if let Some(s) = self.sheet().sticker(id) {
            let r = self.note_rect(s);
            let cam = &mut self.board.sheet_mut().camera;
            cam.zoom = cam.zoom.max(0.8);
            let c = r.center();
            cam.pan = v2(w / 2. - c.x * cam.zoom, h / 2. - c.y * cam.zoom);
        }
        self.select(Selection::Sticker(id));
        cx.notify();
    }

    pub fn next_sheet(&mut self, step: isize, window: &mut Window, cx: &mut Context<Self>) {
        let order = self.board.ordered();
        let n = order.len() as isize;
        let at = order.iter().position(|&i| i == self.board.active).unwrap_or(0) as isize;
        let next = order[((at + step).rem_euclid(n)) as usize];
        self.open_sheet(next, window, cx);
    }

    // ---------- Sheets ----------

    pub fn new_sheet_dialog(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let n = self.board.categories.len();
        self.dialog = Some(Dialog::NewSheet { icon: super::emoji::default_icon(n), template: Template::Blank });
        self.menu = None;
        self.drawer = false;
        self.dialog_input.update(cx, |i, cx| {
            i.set_value("", window, cx);
        });
        let input = self.dialog_input.clone();
        window.defer(cx, move |window, cx| input.update(cx, |i, cx| i.focus(window, cx)));
        cx.notify();
    }

    /// Renaming the open sheet in place: its title becomes a text field.
    pub fn start_naming(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.finish_edit(cx);
        let name = self.sheet().name.clone();
        self.naming = true;
        self.name_input.update(cx, |i, cx| {
            i.set_value(name.clone(), window, cx);
            i.set_selected_range(0..name.len(), cx);
        });
        let input = self.name_input.clone();
        window.defer(cx, move |window, cx| input.update(cx, |i, cx| i.focus(window, cx)));
        cx.notify();
    }

    /// Ends renaming; `keep` saves the name (one undo step), else it's
    /// left as it was. An empty name changes nothing.
    pub fn finish_naming(&mut self, keep: bool, cx: &mut Context<Self>) {
        if !std::mem::take(&mut self.naming) {
            return;
        }
        let name = self.name_input.read(cx).value().trim().to_string();
        if keep && !name.is_empty() && name != self.sheet().name {
            self.change(cx, |b| b.sheet_mut().name = name);
        }
        cx.notify();
    }

    pub fn rename_sheet_dialog(&mut self, ix: usize, window: &mut Window, cx: &mut Context<Self>) {
        let Some(c) = self.board.categories.get(ix) else { return };
        let (id, name) = (c.id, c.name.clone());
        self.dialog = Some(Dialog::RenameSheet(id));
        self.menu = None;
        self.dialog_input.update(cx, |i, cx| i.set_value(name, window, cx));
        let input = self.dialog_input.clone();
        window.defer(cx, move |window, cx| input.update(cx, |i, cx| i.focus(window, cx)));
        cx.notify();
    }

    pub fn dialog_ok(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(d) = self.dialog.take() else { return };
        let name = self.dialog_input.read(cx).value().trim().to_string();
        match d {
            Dialog::NewSheet { icon, template } => {
                let name = if name.is_empty() { template_default_name(template) } else { name };
                let t = now();
                self.change(cx, |b| {
                    b.add_from_template(&name, template, Accent::Slate, t);
                    b.sheet_mut().icon = icon.into();
                });
                let ix = self.board.active;
                self.open_sheet(ix, window, cx);
                if template != Template::Blank {
                    self.zoom_to_fit(cx);
                }
            }
            Dialog::RenameSheet(id) => {
                if !name.is_empty() {
                    self.change(cx, |b| {
                        if let Some(c) = b.categories.iter_mut().find(|c| c.id == id) {
                            c.name = name;
                        }
                    });
                }
            }
            Dialog::RenameFrame(id) => {
                self.change(cx, |b| {
                    if let Some(f) = b.sheet_mut().frames.iter_mut().find(|f| f.id == id) {
                        f.title = name;
                    }
                });
            }
            // Its own buttons decide; Ctrl+Enter doesn't pick for you.
            Dialog::DriveChoice(_) | Dialog::DriveReplace(_) => {
                self.dialog = Some(d);
                return;
            }
        }
        window.focus(&self.focus, cx);
        cx.notify();
    }

    pub fn dialog_cancel(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.dialog = None;
        window.focus(&self.focus, cx);
        cx.notify();
    }

    pub fn toggle_pin_sheet(&mut self, ix: usize, cx: &mut Context<Self>) {
        self.menu = None;
        self.change(cx, |b| {
            if let Some(c) = b.categories.get_mut(ix) {
                c.pinned = !c.pinned;
            }
        });
    }

    pub fn duplicate_sheet(&mut self, ix: usize, cx: &mut Context<Self>) {
        self.menu = None;
        self.change(cx, |b| {
            let Some(c) = b.categories.get(ix).cloned() else { return };
            let name = format!("{} copy", c.name);
            let at = b.adopt_sheet(c, name);
            let c = b.categories.remove(at);
            b.categories.insert(ix + 1, c);
        });
    }

    /// Sends sheet `ix` to the Trash (archived: to the Archive).
    pub fn bin_sheet(&mut self, ix: usize, archived: bool, cx: &mut Context<Self>) {
        self.menu = None;
        let Some(name) = self.board.categories.get(ix).map(|c| c.name.clone()) else { return };
        let t = now();
        let mut ok = false;
        self.change(cx, |b| ok = b.bin_sheet(ix, archived, t));
        if ok {
            if self.screen == Screen::Sheet {
                self.screen = Screen::Home;
                self.board.at_home = true;
            }
            let what = if archived { "Archived" } else { "Moved to Trash" };
            self.notify(format!("{what}: “{name}” · {} to undo", shortcut("Z")), cx);
        } else {
            self.notify("A workspace keeps at least one sheet", cx);
        }
    }

    // ---------- Notes ----------

    /// A new note with its top-left near `at` (world), ready to type into.
    pub fn new_note_at(&mut self, at: Vec2, window: &mut Window, cx: &mut Context<Self>) {
        self.finish_edit(cx);
        let before = self.board.clone();
        let id = self.board.fresh_id();
        let paper = next_paper(self.sheet(), self.board.settings.default_paper);
        let font = self.board.settings.default_font;
        let (w, h) = (240., 120.);
        self.board.sheet_mut().stickers.push(Sticker {
            id,
            rect: Rect { x: at.x - w / 2., y: at.y - 30., w, h },
            paper,
            font,
            edited: now(),
            ..Default::default()
        });
        self.before = Some(before);
        self.select(Selection::Sticker(id));
        self.begin_edit(id, true, window, cx);
    }

    /// A new note in the middle of the view.
    pub fn new_note(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.screen != Screen::Sheet {
            let ix = self.board.active;
            self.open_sheet(ix, window, cx);
        }
        let (w, h) = self.viewport_size();
        let (w, h) = if w < 10. { (1000., 600.) } else { (w, h) };
        let cam = self.sheet().camera;
        let n = self.sheet().stickers.len() as f32 % 6.;
        let at = cam.to_world(v2(w / 2. + n * 18., h / 2. + n * 18.));
        self.new_note_at(at, window, cx);
    }

    /// Starts typing into note `id` on the sheet.
    pub fn begin_edit(&mut self, id: Id, is_new: bool, window: &mut Window, cx: &mut Context<Self>) {
        let Some(s) = self.sheet().sticker(id) else { return };
        if s.locked {
            self.notify("This note is locked · unlock it from its menu", cx);
            return;
        }
        let (title, body) = (s.title.clone(), s.body.clone());
        if self.before.is_none() {
            self.before = Some(self.board.clone());
        }
        self.title_input.update(cx, |i, cx| i.set_value(title.clone(), window, cx));
        // The cursor goes to the end of the text, ready to add to it.
        let end = body.len();
        self.body_input.update(cx, |i, cx| {
            i.set_value(body, window, cx);
            i.set_selected_range(end..end, cx);
        });
        self.editing = Some(Edit { id, is_new });
        self.menu = None;
        self.select(Selection::Sticker(id));
        let (ti, bi) = (self.title_input.clone(), self.body_input.clone());
        // A new note starts on its title; an existing one on its text.
        let to_title = is_new;
        window.defer(cx, move |window, cx| {
            if to_title {
                ti.update(cx, |i, cx| i.focus(window, cx));
            } else {
                bi.update(cx, |i, cx| i.focus(window, cx));
            }
        });
        cx.notify();
    }

    /// Ends typing: keeps the text (an empty new note goes away).
    pub fn finish_edit(&mut self, cx: &mut Context<Self>) {
        let Some(ed) = self.editing.take() else { return };
        let title = self.title_input.read(cx).value().trim().to_string();
        let body = self.body_input.read(cx).value().trim_end().to_string();
        let before = self.before.take().unwrap_or_else(|| self.board.clone());
        let t = now();
        let empty = title.is_empty() && body.trim().is_empty();
        if let Some(ix) = self.board.sheet_of(ed.id) {
            let sheet = &mut self.board.categories[ix];
            let keep = sheet.sticker(ed.id).is_some_and(|s| s.image.is_some() || s.link.is_some());
            if ed.is_new && empty && !keep {
                sheet.remove_sticker(ed.id);
                self.selection.clear();
            } else if let Some(s) = sheet.sticker_mut(ed.id) {
                if s.title != title || s.body != body {
                    s.title = title;
                    s.body = body;
                    s.edited = t;
                }
            }
        }
        self.commit(before, cx);
        cx.notify();
    }

    /// Changes note `id` as one undoable step (also while typing into it).
    pub fn change_note(&mut self, id: Id, cx: &mut Context<Self>, f: impl FnOnce(&mut Sticker)) {
        let t = now();
        if self.editing.is_some() {
            // The edit's snapshot covers it.
            if let Some(s) = self.board.find_note_mut(id) {
                f(s);
                s.edited = t;
            }
            self.desktop_sync(cx);
            cx.notify();
            return;
        }
        self.change(cx, |b| {
            if let Some(s) = b.find_note_mut(id) {
                f(s);
                s.edited = t;
            }
        });
    }

    pub fn toggle_task(&mut self, id: Id, n: usize, cx: &mut Context<Self>) {
        if self.board.find_note(id).is_some_and(|s| s.locked) {
            return;
        }
        self.change_note(id, cx, |s| s.body = toggle_task(&s.body, n));
    }

    /// Adds a task line to the note being typed into, or to `id`.
    pub fn add_task(&mut self, id: Id, window: &mut Window, cx: &mut Context<Self>) {
        if self.editing.map(|e| e.id) == Some(id) {
            let v = self.body_input.read(cx).value().to_string();
            let sep = if v.is_empty() || v.ends_with('\n') { "" } else { "\n" };
            self.body_input.update(cx, |i, cx| {
                i.set_value(format!("{v}{sep}- [ ] "), window, cx);
                i.focus(window, cx);
            });
        } else {
            self.change_note(id, cx, |s| {
                let sep = if s.body.is_empty() || s.body.ends_with('\n') { "" } else { "\n" };
                s.body = format!("{}{sep}- [ ] New task", s.body);
            });
        }
    }

    pub fn delete_selection(&mut self, cx: &mut Context<Self>) {
        if self.selection.is_empty() {
            return;
        }
        let sel = std::mem::take(&mut self.selection);
        let t = now();
        let notes = sel.iter().filter(|s| matches!(s, Selection::Sticker(_))).count();
        self.change(cx, |b| {
            for s in &sel {
                match *s {
                    Selection::Sticker(id) => {
                        b.bin_note(id, false, t);
                    }
                    Selection::Label(id) => b.sheet_mut().labels.retain(|l| l.id != id),
                    Selection::Frame(id) => b.sheet_mut().frames.retain(|f| f.id != id),
                }
            }
        });
        if notes > 0 {
            let what = if notes == 1 { "Note".to_string() } else { format!("{notes} notes") };
            self.notify(format!("{what} moved to Trash · {} to undo", shortcut("Z")), cx);
        }
    }

    pub fn bin_note(&mut self, id: Id, archived: bool, cx: &mut Context<Self>) {
        self.menu = None;
        if self.editing.map(|e| e.id) == Some(id) {
            self.finish_edit(cx);
        }
        let t = now();
        self.change(cx, |b| {
            b.bin_note(id, archived, t);
        });
        self.selection.retain(|s| *s != Selection::Sticker(id));
        let what = if archived { "Archived" } else { "Moved to Trash" };
        self.notify(format!("{what} · {} to undo", shortcut("Z")), cx);
    }

    pub fn duplicate_selection(&mut self, cx: &mut Context<Self>) {
        let ids = self.selected_notes();
        if ids.is_empty() {
            return;
        }
        let mut new = Vec::new();
        let t = now();
        self.change(cx, |b| {
            for id in ids {
                let nid = b.fresh_id();
                let sheet = b.sheet_mut();
                if let Some(mut s) = sheet.sticker(id).cloned() {
                    s.id = nid;
                    s.rect.x += 24.;
                    s.rect.y += 24.;
                    s.edited = t;
                    s.popped = None;
                    s.due = None;
                    s.reminded = false;
                    sheet.stickers.push(s);
                    new.push(Selection::Sticker(nid));
                }
            }
        });
        self.selection = new;
    }

    pub fn move_note_to(&mut self, id: Id, to: usize, cx: &mut Context<Self>) {
        self.menu = None;
        let name = self.board.categories.get(to).map(|c| c.name.clone()).unwrap_or_default();
        let mut ok = false;
        self.change(cx, |b| ok = b.move_note(id, to));
        if ok {
            self.selection.retain(|s| *s != Selection::Sticker(id));
            self.notify(format!("Moved to “{name}”"), cx);
        }
    }

    pub fn set_reminder(&mut self, id: Id, due: Option<u64>, cx: &mut Context<Self>) {
        self.menu = None;
        self.change_note(id, cx, |s| {
            s.due = due;
            s.reminded = false;
        });
        if let Some(d) = due {
            self.notify(format!("Reminder set · {}", time::due_label(d, now())), cx);
        }
    }

    pub fn copy_note(&mut self, id: Id, cx: &mut Context<Self>) {
        self.menu = None;
        if let Some(s) = self.board.find_note(id) {
            cx.write_to_clipboard(ClipboardItem::new_string(s.text()));
            self.notify("Copied as text", cx);
        }
    }

    /// Lays the selected notes (or every note) out in tidy rows.
    pub fn tidy(&mut self, cx: &mut Context<Self>) {
        self.menu = None;
        let mut ids = self.selected_notes();
        if ids.len() < 2 {
            ids = self.sheet().stickers.iter().map(|s| s.id).collect();
        }
        let heights = self.heights.borrow().clone();
        self.change(cx, |b| {
            b.sheet_mut().tidy(&ids, &|s: &Sticker| {
                if s.fixed {
                    s.rect.h
                } else {
                    s.rect.h.max(heights.get(&s.id).copied().unwrap_or(0.))
                }
            })
        });
        self.zoom_to_fit(cx);
    }

    /// Starts writing a label on the sheet at `at` (world).
    pub fn new_label_at(&mut self, at: Vec2, window: &mut Window, cx: &mut Context<Self>) {
        let before = self.board.clone();
        let id = self.board.fresh_id();
        let ink = self.ink;
        self.board.sheet_mut().labels.push(Label { id, at: v2(at.x, at.y - 11.), text: String::new(), ink });
        self.before = Some(before);
        self.select(Selection::Label(id));
        self.edit_label(id, true, window, cx);
    }

    /// Writes on the sheet in place: the text gets a caret where it sits,
    /// in its own style; clicking away, Ctrl+Enter or Esc keeps it.
    pub fn edit_label(&mut self, id: Id, _is_new: bool, window: &mut Window, cx: &mut Context<Self>) {
        let Some(l) = self.sheet().labels.iter().find(|l| l.id == id) else { return };
        let text = l.text.clone();
        self.finish_label(cx);
        if self.before.is_none() {
            self.before = Some(self.board.clone());
        }
        self.label_edit = Some(id);
        self.menu = None;
        let end = text.len();
        self.label_input.update(cx, |i, cx| {
            i.set_value(text, window, cx);
            i.set_selected_range(end..end, cx);
        });
        let input = self.label_input.clone();
        window.defer(cx, move |window, cx| input.update(cx, |i, cx| i.focus(window, cx)));
        cx.notify();
    }

    /// Ends writing on the sheet: the text is kept (one undo step), or
    /// gone if it was left empty.
    pub fn finish_label(&mut self, cx: &mut Context<Self>) {
        let Some(id) = self.label_edit.take() else { return };
        let text = self.label_input.read(cx).value().trim_end().to_string();
        let before = self.before.take().unwrap_or_else(|| self.board.clone());
        let sheet = self.board.sheet_mut();
        if text.trim().is_empty() {
            sheet.labels.retain(|l| l.id != id);
        } else if let Some(l) = sheet.labels.iter_mut().find(|l| l.id == id) {
            l.text = text;
        }
        self.commit(before, cx);
        cx.notify();
    }

    pub fn rename_frame(&mut self, id: Id, window: &mut Window, cx: &mut Context<Self>) {
        let Some(f) = self.sheet().frames.iter().find(|f| f.id == id) else { return };
        let title = f.title.clone();
        self.dialog = Some(Dialog::RenameFrame(id));
        self.menu = None;
        self.dialog_input.update(cx, |i, cx| i.set_value(title, window, cx));
        let input = self.dialog_input.clone();
        window.defer(cx, move |window, cx| input.update(cx, |i, cx| i.focus(window, cx)));
        cx.notify();
    }

    /// Asks for a picture and puts it on note `id`.
    pub fn pick_image(&mut self, id: Id, cx: &mut Context<Self>) {
        self.menu = None;
        let rx = cx.prompt_for_paths(gpui::PathPromptOptions {
            files: true,
            directories: false,
            multiple: false,
            prompt: Some("Add picture".into()),
        });
        cx.spawn(async move |this, cx| {
            let Ok(Ok(Some(paths))) = rx.await else { return };
            let Some(path) = paths.into_iter().next() else { return };
            let _ = this.update(cx, |this, cx| {
                if !store::is_image(&path) {
                    this.notify("That file isn't a picture Folio can show", cx);
                    return;
                }
                let file_id = this.board.fresh_id();
                match store::import_image(&this.dir, &path, file_id) {
                    Ok(name) => this.change_note(id, cx, |s| s.image = Some(name)),
                    Err(e) => this.notify(format!("Couldn't add the picture: {e}"), cx),
                }
            });
        })
        .detach();
    }

    pub fn image_path(&self, name: &str) -> PathBuf {
        store::media_dir(&self.dir).join(name)
    }

    // ---------- Bringing things in ----------

    /// Files dropped on the sheet: pictures become picture notes, text and
    /// Markdown files become notes holding their text, and Folio files add
    /// their sheets.
    pub fn drop_paths(&mut self, paths: Vec<PathBuf>, cx: &mut Context<Self>) {
        let (folio, paths): (Vec<PathBuf>, Vec<PathBuf>) =
            paths.into_iter().partition(|p| p.extension().is_some_and(|e| e.eq_ignore_ascii_case("folio")));
        for p in &folio {
            self.import_path(p, cx);
        }
        if paths.is_empty() {
            return;
        }
        let before = self.board.clone();
        let mut at = self.sheet().camera.to_world(self.cursor);
        let mut added = 0;
        for path in paths {
            let id = self.board.fresh_id();
            let name = path.file_stem().and_then(|s| s.to_str()).unwrap_or("").to_string();
            let sticker = if store::is_image(&path) {
                store::import_image(&self.dir, &path, id).ok().map(|file| image_sticker(id, at, file, name))
            } else if is_text_file(&path) {
                std::fs::read_to_string(&path).ok().map(|body| Sticker {
                    id,
                    rect: Rect { x: at.x, y: at.y, w: 280., h: 140. },
                    paper: Paper::Plain,
                    title: name,
                    body,
                    text_size: TextSize::Small,
                    edited: now(),
                    ..Default::default()
                })
            } else {
                None
            };
            if let Some(s) = sticker {
                self.board.sheet_mut().stickers.push(s);
                at = at.add(v2(28., 28.));
                added += 1;
            }
        }
        if added == 0 {
            self.notify("Drop pictures, Markdown or text files", cx);
        }
        self.commit(before, cx);
        cx.notify();
    }

    /// Paste: a picture becomes a picture note, a web address a link card,
    /// other text a note.
    pub fn paste(&mut self, cx: &mut Context<Self>) {
        let Some(item) = cx.read_from_clipboard() else { return };
        let (w, h) = self.viewport_size();
        let at = self.sheet().camera.to_world(v2(w / 2., h / 2.));
        for entry in item.entries() {
            match entry {
                ClipboardEntry::Image(img) => {
                    let before = self.board.clone();
                    let id = self.board.fresh_id();
                    let media = store::media_dir(&self.dir);
                    let file = store::media_name(id, img.format.extension());
                    let written =
                        std::fs::create_dir_all(&media).and_then(|_| std::fs::write(media.join(&file), &img.bytes));
                    if written.is_ok() {
                        self.board.sheet_mut().stickers.push(image_sticker(id, at, file, String::new()));
                        self.select(Selection::Sticker(id));
                        self.commit(before, cx);
                    }
                    cx.notify();
                    return;
                }
                ClipboardEntry::ExternalPaths(paths) => {
                    self.cursor = v2(w / 2., h / 2.);
                    self.drop_paths(paths.paths().to_vec(), cx);
                    return;
                }
                ClipboardEntry::String(s) => {
                    let text = s.text().trim().to_string();
                    if text.is_empty() {
                        continue;
                    }
                    if crate::links::is_url(&text) {
                        self.add_link(text, at, cx);
                        return;
                    }
                    let mut new_id = 0;
                    let def = self.board.settings.default_paper;
                    self.change(cx, |b| {
                        new_id = b.fresh_id();
                        let paper = next_paper(b.sheet(), def);
                        b.sheet_mut().stickers.push(Sticker {
                            id: new_id,
                            rect: Rect { x: at.x - 120., y: at.y - 60., w: 240., h: 120. },
                            paper,
                            body: text,
                            edited: now(),
                            ..Default::default()
                        });
                    });
                    self.select(Selection::Sticker(new_id));
                    return;
                }
            }
        }
    }

    /// A link card for `url` at `at`; its title is fetched in the background.
    pub fn add_link(&mut self, url: String, at: Vec2, cx: &mut Context<Self>) {
        let mut id = 0;
        let site = crate::links::site(&url);
        self.change(cx, |b| {
            id = b.fresh_id();
            b.sheet_mut().stickers.push(Sticker {
                id,
                rect: Rect { x: at.x - 140., y: at.y - 50., w: 280., h: 96. },
                paper: Paper::Plain,
                link: Some(LinkCard { url: url.clone(), title: String::new(), site, icon: None }),
                edited: now(),
                ..Default::default()
            });
        });
        self.select(Selection::Sticker(id));
        let task = cx.background_executor().spawn(async move { crate::links::fetch_page(&url) });
        cx.spawn(async move |this, cx| {
            let page = task.await;
            let _ = this.update(cx, |this, cx| {
                // Not an undo step of its own: it completes the paste.
                let media = store::media_dir(&this.dir);
                let _ = std::fs::create_dir_all(&media);
                let save = |this: &mut Folio, file: Option<(Vec<u8>, String)>| {
                    let (bytes, ext) = file?;
                    let name = store::media_name(this.board.fresh_id(), &ext);
                    std::fs::write(media.join(&name), bytes).ok()?;
                    Some(name)
                };
                let image = save(this, page.image);
                let icon = save(this, page.icon);
                if let Some(s) = this.board.find_note_mut(id) {
                    if image.is_some() {
                        s.image = image;
                        s.rect.h = s.rect.h.max(200.);
                    }
                    if let Some(l) = s.link.as_mut() {
                        if let Some(t) = page.title {
                            l.title = t;
                        }
                        l.icon = icon;
                    }
                }
                this.schedule_save(cx);
                cx.notify();
            });
        })
        .detach();
    }

    // ---------- The view ----------

    pub fn set_tool(&mut self, tool: Tool, cx: &mut Context<Self>) {
        self.finish_edit(cx);
        self.tool = tool;
        if !matches!(tool, Tool::Select) {
            self.selection.clear();
        }
        cx.notify();
    }

    pub fn zoom_by(&mut self, factor: f32, cx: &mut Context<Self>) {
        let (w, h) = self.viewport_size();
        self.board.sheet_mut().camera.zoom_at(v2(w / 2., h / 2.), factor);
        self.schedule_save(cx);
        cx.notify();
    }

    pub fn zoom_to_fit(&mut self, cx: &mut Context<Self>) {
        let (w, h) = self.viewport_size();
        let (w, h) = if w < 10. { (1000., 640.) } else { (w, h) };
        let rects: Vec<Rect> = self.sheet().stickers.iter().map(|s| self.note_rect(s)).collect();
        let sheet = self.board.sheet_mut();
        let all = sheet.content_bounds().into_iter().chain(rects).reduce(|a, b| a.union(&b));
        match all {
            Some(r) => {
                sheet.camera.fit(r, w, h - 60., 80.);
                sheet.camera.pan.y += 50.;
            }
            None => sheet.camera = Camera::default(),
        }
        self.schedule_save(cx);
        cx.notify();
    }

    /// Selects the nearest note in a direction and brings it into view.
    pub fn step_selection(&mut self, dx: f32, dy: f32, cx: &mut Context<Self>) {
        let current = self.selected_notes().first().copied();
        let next = match current {
            Some(id) => self.sheet().neighbour(id, dx, dy),
            None => self.sheet().stickers.first().map(|s| s.id),
        };
        let Some(next) = next else { return };
        self.select(Selection::Sticker(next));
        let (w, h) = self.viewport_size();
        if let Some(s) = self.sheet().sticker(next) {
            let r = self.note_rect(s);
            let cam = self.sheet().camera;
            let a = cam.to_screen(v2(r.x, r.y));
            let b = cam.to_screen(v2(r.x + r.w, r.y + r.h));
            if a.x < 40. || a.y < 80. || b.x > w - 40. || b.y > h - 100. {
                let c = r.center();
                let cam = &mut self.board.sheet_mut().camera;
                cam.pan = v2(w / 2. - c.x * cam.zoom, h / 2. - c.y * cam.zoom);
                self.schedule_save(cx);
            }
        }
        cx.notify();
    }

    // ---------- Trash, archive, backups ----------

    pub fn restore_binned(&mut self, bin_id: Id, cx: &mut Context<Self>) {
        let mut to = None;
        self.change(cx, |b| to = b.restore(bin_id));
        if let Some(ix) = to {
            let name = self.board.categories[ix].name.clone();
            self.notify(format!("Restored to “{name}”"), cx);
        }
    }

    pub fn delete_forever(&mut self, bin_id: Id, cx: &mut Context<Self>) {
        self.change(cx, |b| b.bin.retain(|x| x.id != bin_id));
    }

    pub fn empty_trash(&mut self, cx: &mut Context<Self>) {
        self.change(cx, |b| b.bin.retain(|x| x.archived));
        self.notify(format!("Trash emptied · {} to undo", shortcut("Z")), cx);
    }

    pub fn back_up_now(&mut self, cx: &mut Context<Self>) {
        let day = time::today();
        let _ = std::fs::remove_file(store::backup_dir(&self.dir).join(format!("{day}.json")));
        match store::backup_daily(&self.dir, &self.board, &day) {
            Ok(_) => self.notify(format!("Backed up · {day}"), cx),
            Err(e) => self.notify(format!("Couldn't back up: {e}"), cx),
        }
    }

    /// Lists (or hides) the sheets in a day's backup.
    pub fn toggle_backup(&mut self, day: &str, cx: &mut Context<Self>) {
        if self.backup_open.as_ref().is_some_and(|(d, _)| d == day) {
            self.backup_open = None;
        } else if let Some(b) = store::load_backup(&self.dir, day) {
            let sheets = b.categories.iter().map(|c| (c.id, c.name.clone(), c.stickers.len(), c.icon.clone())).collect();
            self.backup_open = Some((day.to_string(), sheets));
        }
        cx.notify();
    }

    /// Brings back one sheet as it was in a day's backup, as a new sheet
    /// beside the others (undoable).
    pub fn restore_backup_sheet(&mut self, day: &str, sheet: Id, cx: &mut Context<Self>) {
        let Some(c) = store::load_backup(&self.dir, day).and_then(|b| b.categories.into_iter().find(|c| c.id == sheet)) else {
            self.notify("That backup couldn't be read", cx);
            return;
        };
        let name = format!("{} ({day})", c.name);
        self.change(cx, |b| {
            b.adopt_sheet(c, name.clone());
        });
        self.notify(format!("Restored “{name}”"), cx);
    }

    /// Puts a day's backup in place of the board (undoable).
    pub fn restore_backup(&mut self, day: &str, cx: &mut Context<Self>) {
        let Some(mut b) = store::load_backup(&self.dir, day) else {
            self.notify("That backup couldn't be read", cx);
            return;
        };
        b.settings = self.board.settings.clone();
        self.change(cx, |board| *board = b);
        self.selection.clear();
        self.screen = Screen::Home;
        self.board.at_home = true;
        self.notify(format!("Restored the backup of {day} · {} to undo", shortcut("Z")), cx);
    }

    // ---------- Settings ----------

    pub fn set_setting(&mut self, cx: &mut Context<Self>, f: impl FnOnce(&mut Settings)) {
        let before = self.board.settings.clone();
        f(&mut self.board.settings);
        let s = self.board.settings.clone();
        if s.theme != before.theme || s.high_contrast != before.high_contrast || s.ui_scale != before.ui_scale || s.reduce_motion != before.reduce_motion {
            self.apply_theme(cx);
        }
        if s.start_with_windows != before.start_with_windows {
            if let Err(e) = crate::desktop::set_start_with_windows(s.start_with_windows) {
                self.notify(e, cx);
            }
        }
        if s.quick_hotkey != before.quick_hotkey {
            self.desktop.set_hotkey(s.quick_hotkey);
        }
        self.schedule_save(cx);
        cx.notify();
    }

    // ---------- Reminders ----------

    pub fn check_reminders(&mut self, cx: &mut Context<Self>) {
        let due = self.board.take_due(now());
        if due.is_empty() {
            return;
        }
        for id in &due {
            if let Some(s) = self.board.find_note(*id) {
                let t = s.text();
                let first = t.lines().find(|l| !l.trim().is_empty()).unwrap_or("A note").to_string();
                crate::desktop::notify("Folio reminder", &first);
                self.notify(format!("⏰ {first}"), cx);
            }
        }
        self.schedule_save(cx);
        cx.notify();
    }

    // ---------- Quick notes ----------

    /// A note from the quick-note window, into the Inbox.
    pub fn quick_note(&mut self, text: &str, cx: &mut Context<Self>) {
        if text.trim().is_empty() {
            return;
        }
        let t = now();
        self.change(cx, |b| {
            b.quick_note(text, t);
        });
        self.notify("Saved to Inbox", cx);
    }
}

impl Render for Folio {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let t = theme(cx).clone();
        self.track_place();
        // Off the sheet, the sheet's focus is stale: keys go to the window.
        if self.screen != Screen::Sheet && self.focus.is_focused(window) {
            window.focus(&self.root_focus, cx);
        }
        if self.screen == Screen::Sheet && self.root_focus.is_focused(window) {
            window.focus(&self.focus, cx);
        }
        let main = match self.screen {
            Screen::Home => self.render_home(window, cx).into_any_element(),
            Screen::Sheet => self.render_board(window, cx).into_any_element(),
            Screen::Settings => self.render_settings(window, cx).into_any_element(),
            Screen::Bin => self.render_bin(window, cx).into_any_element(),
            Screen::Transfer => self.render_transfer(window, cx).into_any_element(),
        };
        div()
            .id("folio")
            .key_context("Folio")
            .track_focus(&self.root_focus)
            .size_full()
            .relative()
            .flex()
            .flex_col()
            .font_family(t.font)
            .text_size(t.text.body)
            .text_color(t.colors.text)
            .bg(t.colors.canvas)
            .on_action(cx.listener(|this, _: &Undo, _, cx| this.undo(cx)))
            .on_action(cx.listener(|this, _: &Redo, _, cx| this.redo(cx)))
            .on_action(cx.listener(|this, _: &ToggleTheme, _, cx| this.toggle_theme(cx)))
            .on_action(cx.listener(|this, _: &NewSheet, window, cx| this.new_sheet_dialog(window, cx)))
            .on_action(cx.listener(|this, _: &NewNote, window, cx| this.new_note(window, cx)))
            .on_action(cx.listener(|this, _: &NextSheet, window, cx| this.next_sheet(1, window, cx)))
            .on_action(cx.listener(|this, _: &PrevSheet, window, cx| this.next_sheet(-1, window, cx)))
            .on_action(cx.listener(|this, _: &GoHome, _, cx| this.go_home(cx)))
            .on_action(cx.listener(|this, _: &GoBack, window, cx| this.go_back(window, cx)))
            .on_action(cx.listener(|this, _: &GoForward, window, cx| this.go_forward(window, cx)))
            .on_action(cx.listener(|this, _: &ToggleDrawer, _, cx| {
                this.drawer = !this.drawer;
                cx.notify();
            }))
            .on_action(cx.listener(|this, _: &OpenPalette, window, cx| this.open_palette(window, cx)))
            .on_action(cx.listener(|this, _: &FocusSearch, window, cx| {
                this.open_palette_with("@", window, cx);
            }))
            .on_action(cx.listener(|this, _: &OpenSettings, _, cx| this.open_settings(cx)))
            .on_action(cx.listener(|_, _: &QuickNote, _, cx| crate::desktop::open_quick_note(cx.entity(), cx)))
            .on_action(cx.listener(|this, _: &Quit, _, cx| {
                this.finish_edit(cx);
                this.save_now(cx);
                cx.quit();
            }))
            .on_action(cx.listener(|this, _: &Dismiss, window, cx| {
                if this.welcome.is_some() {
                    this.welcome_close(cx);
                } else if this.label_edit.is_some() {
                    this.finish_label(cx);
                    window.focus(&this.focus, cx);
                } else if this.naming {
                    this.finish_naming(false, cx);
                    window.focus(&this.focus, cx);
                } else if this.editing.is_some() {
                    this.finish_edit(cx);
                    window.focus(&this.focus, cx);
                } else if this.palette {
                    this.close_palette(window, cx);
                } else if this.menu.is_some() {
                    this.menu = None;
                } else if this.dialog.is_some() {
                    this.dialog_cancel(window, cx);
                } else if this.drawer {
                    this.drawer = false;
                } else if matches!(this.screen, Screen::Settings | Screen::Bin | Screen::Transfer) {
                    this.close_side(cx);
                }
                cx.notify();
            }))
            .child(self.render_topbar(window, cx))
            .child({
                // Going somewhere: the page fades up into place.
                let key = match self.screen {
                    Screen::Sheet => format!("page-sheet-{}", self.sheet().id),
                    s => format!("page-{s:?}"),
                };
                super::motion::page_in(div().flex_1().min_h_0().relative().flex().flex_col().child(main), gpui::ElementId::Name(key.into()), &t)
            })
            .child(self.render_drawer(window, cx))
            .children(self.dialog.is_some().then(|| self.render_dialog(window, cx)))
            .children(self.palette.then(|| self.render_palette_scrim(cx)))
            .children(self.menu.is_some().then(|| self.render_menu(window, cx)))
            .children(self.toast.clone().map(|msg| toast(&t, msg)))
            // The first time, the welcome rises in as the opening fades
            // away over it.
            .children((self.welcome.is_some() && self.intro.as_ref().is_none_or(|i| i.revealing())).then(|| self.render_welcome(window, cx)))
            .children(self.intro.is_some().then(|| self.render_intro(window, cx)))
    }
}

pub fn is_dark(a: gpui::WindowAppearance) -> bool {
    matches!(a, gpui::WindowAppearance::Dark | gpui::WindowAppearance::VibrantDark)
}

fn template_default_name(t: Template) -> String {
    match t {
        Template::Blank => "Untitled sheet".into(),
        other => other.name().into(),
    }
}

/// The paper after the newest note's, so a run of new notes alternates;
/// the first follows the default.
fn next_paper(sheet: &Category, default: Paper) -> Paper {
    let Some(last) = sheet.stickers.last().map(|s| s.paper) else { return default };
    let i = Paper::ALL.iter().position(|p| *p == last).unwrap_or(0);
    Paper::ALL[(i + 1) % (Paper::ALL.len() - 1)]
}

fn image_sticker(id: Id, at: Vec2, file: String, title: String) -> Sticker {
    Sticker {
        id,
        rect: Rect { x: at.x, y: at.y, w: 240., h: 200. },
        paper: Paper::Plain,
        title,
        image: Some(file),
        fixed: true,
        edited: now(),
        ..Default::default()
    }
}

fn is_text_file(path: &std::path::Path) -> bool {
    matches!(
        path.extension().and_then(|e| e.to_str()).map(str::to_ascii_lowercase).as_deref(),
        Some("md" | "markdown" | "txt")
    )
}
