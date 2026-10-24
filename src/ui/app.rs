//! The main window's view: the board, which screen is showing, the tool in
//! hand, the gesture under way, the note being typed into, menus, dialogs,
//! undo history and the debounced save. The other files in `ui/` each
//! render one part of it.

use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::Arc;
use std::time::{Duration, Instant};

use gpui::component::input::{InputEvent, InputState, TextareaState};
use gpui::{
    div, prelude::*, Bounds, ClipboardEntry, ClipboardItem, Context, Entity, FocusHandle, Pixels, Point,
    SharedString, Task, Window,
};

use crate::history::{Before, Change, Way};
use crate::markdown::{self, Prefix};
use crate::model::*;
use crate::store;
use crate::theme::{self, theme};
use crate::time::{self, now};
use crate::ui::*;
use super::motion::Exit;

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

/// A writing command, from the edit bar or its shortcut.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Format {
    Bold,
    Italic,
    Strike,
    Code,
    Highlight,
    Bullets,
    Numbers,
    Tasks,
    Callout,
    Table,
    /// Enter: the next list item, or a plain new line.
    NewLine,
    /// Plain text into Markdown: the selection, or the whole note.
    Markdown,
    /// The clipboard's text, tidied.
    Paste,
}

/// Runs `f` on a note's title and text areas. Only what changed is
/// written, so the text keeps its scroll and Ctrl+Z in the note undoes it.
pub fn format_input(title: &Entity<InputState>, body: &Entity<TextareaState>, f: Format, window: &mut Window, cx: &mut gpui::App) {
    if f == Format::Markdown || f == Format::Paste {
        tidy_input(title, body, f == Format::Paste, window, cx);
        return;
    }
    format_text(body, f, window, cx);
}

/// A writing command on a text with no title (text on the sheet): marks,
/// lists, a new line, or tidying into Markdown.
pub fn format_text(body: &Entity<TextareaState>, f: Format, window: &mut Window, cx: &mut gpui::App) {
    if f == Format::Markdown || f == Format::Paste {
        tidy_text(body, f == Format::Paste, window, cx);
        return;
    }
    body.update(cx, |i, cx| {
        let text = i.value().to_string();
        let sel = i.selected_range();
        let (new, range) = match f {
            Format::Bold => markdown::toggle_wrap(&text, sel, "**"),
            Format::Italic => markdown::toggle_wrap(&text, sel, "*"),
            Format::Strike => markdown::toggle_wrap(&text, sel, "~~"),
            Format::Code => markdown::toggle_wrap(&text, sel, "`"),
            Format::Highlight => markdown::toggle_mark(&text, sel, "yellow"),
            Format::Bullets => markdown::toggle_prefix(&text, sel, Prefix::Bullet),
            Format::Numbers => markdown::toggle_prefix(&text, sel, Prefix::Numbered),
            Format::Tasks => markdown::toggle_prefix(&text, sel, Prefix::Task),
            Format::Callout => markdown::toggle_callout(&text, sel),
            Format::Table => markdown::insert_table(&text, sel),
            Format::NewLine => match markdown::continue_list(&text, sel.start).filter(|_| sel.is_empty()) {
                Some((new, at)) => (new, at..at),
                None => {
                    i.replace("\n", window, cx);
                    return;
                }
            },
            Format::Markdown | Format::Paste => return,
        };
        replace_changed(i, &text, &new, range, window, cx);
    });
}

/// A writing shortcut in a note's title or text. In the title, Enter
/// moves on to the text and the marks wait for it; Markdown and Paste work
/// anywhere.
pub fn format_key_input(title: &Entity<InputState>, body: &Entity<TextareaState>, f: Format, window: &mut Window, cx: &mut gpui::App) {
    use gpui::Focusable;
    if title.read(cx).focus_handle(cx).is_focused(window) {
        match f {
            Format::NewLine => body.update(cx, |i, cx| i.focus(window, cx)),
            Format::Markdown | Format::Paste => format_input(title, body, f, window, cx),
            _ => {}
        }
        return;
    }
    format_input(title, body, f, window, cx);
}

/// Text into Markdown: the selection, or the whole text (its first line
/// becomes the title when there's none); with `paste`, the clipboard's
/// text, tidied, goes in at the cursor.
fn tidy_input(title: &Entity<InputState>, body: &Entity<TextareaState>, paste: bool, window: &mut Window, cx: &mut gpui::App) {
    let text = body.read(cx).value().to_string();
    let sel = body.read(cx).selected_range();
    let untitled = title.read(cx).value().trim().is_empty();
    let (head, new, range) = if paste {
        let Some(clip) = cx.read_from_clipboard().and_then(|c| c.text()) else { return };
        let (head, md) = if untitled && text.trim().is_empty() { markdown::tidy(&clip) } else { (None, markdown::tidy_body(&clip)) };
        let new = format!("{}{md}{}", &text[..sel.start], &text[sel.end..]);
        let at = sel.start + md.len();
        (head, new, at..at)
    } else if !sel.is_empty() {
        let md = markdown::tidy_body(&text[sel.clone()]);
        let new = format!("{}{md}{}", &text[..sel.start], &text[sel.end..]);
        let range = sel.start..sel.start + md.len();
        (None, new, range)
    } else {
        let (head, md) = if untitled { markdown::tidy(&text) } else { (None, markdown::tidy_body(&text)) };
        let end = md.len();
        (head, md, end..end)
    };
    if let Some(head) = head {
        title.update(cx, |i, cx| i.set_value(head, window, cx));
    }
    body.update(cx, |i, cx| replace_changed(i, &text, &new, range, window, cx));
}

/// Writes `new` over `old` in a text area where they differ, as one undo
/// step, then selects `sel`.
/// Text into Markdown in a text with no title: the selection, or all of
/// it; with `paste`, the clipboard's text, tidied, where the caret is.
fn tidy_text(body: &Entity<TextareaState>, paste: bool, window: &mut Window, cx: &mut gpui::App) {
    let text = body.read(cx).value().to_string();
    let sel = body.read(cx).selected_range();
    let (new, range) = if paste {
        let Some(clip) = cx.read_from_clipboard().and_then(|c| c.text()) else { return };
        let md = markdown::tidy_body(&clip);
        let at = sel.start + md.len();
        (format!("{}{md}{}", &text[..sel.start], &text[sel.end..]), at..at)
    } else if sel.is_empty() {
        let md = markdown::tidy_body(&text);
        let end = md.len();
        (md, end..end)
    } else {
        let md = markdown::tidy_body(&text[sel.clone()]);
        let end = sel.start + md.len();
        (format!("{}{md}{}", &text[..sel.start], &text[sel.end..]), sel.start..end)
    };
    body.update(cx, |i, cx| replace_changed(i, &text, &new, range, window, cx));
}

fn replace_changed(i: &mut TextareaState, old: &str, new: &str, sel: std::ops::Range<usize>, window: &mut Window, cx: &mut Context<TextareaState>) {
    let pre: usize = old.chars().zip(new.chars()).take_while(|(a, b)| a == b).map(|(a, _)| a.len_utf8()).sum();
    let suf: usize =
        old[pre..].chars().rev().zip(new[pre..].chars().rev()).take_while(|(a, b)| a == b).map(|(a, _)| a.len_utf8()).sum();
    if old != new {
        i.set_selected_range(pre..old.len() - suf, cx);
        i.replace(new[pre..new.len() - suf].to_string(), window, cx);
    }
    i.set_selected_range(sel, cx);
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
    /// Setting where text on the sheet wraps, from its right edge: the
    /// pointer's x and the width at the press (world units).
    LabelWidth { id: Id, start: f32, w: f32 },
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
    /// The sheet's pattern, colour and picture.
    Background,
    /// What a note is pinned up with.
    Pin,
}

/// Notes (and text on the sheet) copied or cut in Folio, as they were:
/// Ctrl+V makes them again whole, paper, type, pin and the links between
/// them, while the clipboard still holds the text written with them. Other
/// apps get that text.
#[derive(Clone)]
pub struct Clip {
    text: String,
    notes: Vec<Sticker>,
    links: Vec<Link>,
    labels: Vec<Label>,
    /// Cut, not copied: the first paste takes the notes back out of the
    /// Trash, where cutting put them.
    cut: bool,
}

#[derive(Clone)]
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
    /// Setting up Google sign-in: the app's client id and secret.
    GoogleSetup,
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
    /// Notes copied or cut here, for Ctrl+V to make again whole.
    pub clip: Option<Clip>,
    /// The undo snapshot taken when the current gesture or edit began: the
    /// sheets it can change, not the whole board.
    pub before: Option<Before>,
    pub selection: Vec<Selection>,
    pub hovered: Option<Id>,
    /// The note or area edge under the pointer, for the resize arrow.
    pub hover_grip: Option<crate::model::Grip>,
    /// The pointer is on a selected text's width handle.
    pub hover_label_grip: bool,
    /// Space is held: drag pans.
    pub space: bool,
    /// The last pointer position over the sheet, in sheet-local pixels.
    pub cursor: Vec2,
    /// Where the sheet was laid out last frame, for turning window
    /// positions into sheet positions.
    pub viewport: Rc<Cell<Bounds<Pixels>>>,
    /// Each note's natural height (world units), measured as it's drawn.
    pub heights: Rc<RefCell<HashMap<Id, f32>>>,
    /// The edit bar's size as last drawn (screen px), to find it a place.
    pub edit_bar_size: Rc<Cell<(f32, f32)>>,
    /// Each label's size (world units), measured as it's drawn.
    pub label_sizes: Rc<RefCell<HashMap<Id, (f32, f32)>>>,
    /// Where the sheet's name is drawn (window), for clicks and renaming.
    pub head_name: Rc<Cell<Option<Bounds<Pixels>>>>,
    /// Undo and redo steps: what each edit changed, not copies of the board.
    pub undo: Vec<Arc<Change>>,
    pub redo: Vec<Arc<Change>>,
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
    pub exits: Exits,
    pub saved: bool,
    /// The OS's light/dark, when the theme follows it.
    pub system_dark: bool,
    pub title_input: Entity<InputState>,
    pub body_input: Entity<TextareaState>,
    pub search_input: Entity<InputState>,
    pub palette_input: Entity<InputState>,
    pub dialog_input: Entity<InputState>,
    /// The Google client id and secret, in its setup.
    pub client_input: Entity<InputState>,
    pub secret_input: Entity<InputState>,
    /// The sheet's name while it's being renamed in place.
    pub name_input: Entity<InputState>,
    pub naming: bool,
    /// Text being written straight on the sheet.
    pub label_edit: Option<Id>,
    /// Every ink, open under the bar of the text being written.
    pub label_palette: bool,
    /// The edit bar's papers are open.
    pub edit_papers: bool,
    pub label_input: Entity<TextareaState>,
    pub desktop: crate::desktop::Desktop,
    pub sync: super::sync::DriveSync,
    pub export: super::transfer::ExportChoice,
    toast_task: Option<Task<()>>,
    save_task: Option<Task<()>>,
    /// Writes the board off the UI thread; saves started so far.
    saver: store::Saver,
    saves: u64,
    /// Counts changes to the board (each one saved): Drive's sync tells by
    /// it whether anything changed while it ran.
    pub edits: u64,
    /// The view (where each sheet is scrolled to), saved on its own.
    view_task: Option<Task<()>>,
    view_saves: u64,
    _ticker: Task<()>,
    _subs: Vec<gpui::Subscription>,
}

/// What just closed, kept on screen while it leaves (`motion::Exit`).
#[derive(Default)]
pub struct Exits {
    pub menu: Exit<(Point<Pixels>, MenuKind, MenuPage)>,
    pub dialog: Exit<Dialog>,
    pub toast: Exit<SharedString>,
    pub drawer: Exit<()>,
    pub palette: Exit<()>,
    pub notes: super::motion::NoteMotion,
    /// The edit bar, by the note being typed into.
    pub edit: Exit<Id>,
    /// The ink pill beside the tool dock, and the palette over it.
    pub inks: Exit<()>,
    pub ink_palette: Exit<()>,
    pub welcome: Exit<super::welcome::Welcome>,
    /// Light and dark switching: the old canvas colour, fading off, and
    /// when; a number for its animation.
    pub theme: Option<(gpui::Hsla, Instant)>,
    pub theme_n: usize,
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
        let client_input = cx.new(|cx| InputState::new(window, cx).placeholder("1234567890-abc….apps.googleusercontent.com"));
        let secret_input = cx.new(|cx| InputState::new(window, cx).placeholder("GOCSPX-…").masked(true));
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

        let undo = store::load_history(&dir, &board);
        let saver = store::Saver::new(&dir);
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
            clip: None,
            before: None,
            selection: Vec::new(),
            hovered: None,
            hover_grip: None,
            hover_label_grip: false,
            space: false,
            cursor: v2(0., 0.),
            viewport: Rc::new(Cell::new(Bounds::default())),
            heights: Rc::new(RefCell::new(HashMap::new())),
            edit_bar_size: Rc::new(Cell::new((700., 76.))),
            label_sizes: Rc::new(RefCell::new(HashMap::new())),
            head_name: Rc::default(),
            undo,
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
            exits: Exits::default(),
            saved: true,
            system_dark: is_dark(window.appearance()),
            title_input,
            body_input,
            search_input,
            palette_input,
            dialog_input,
            name_input,
            naming: false,
            client_input,
            secret_input,
            label_edit: None,
            label_palette: false,
            edit_papers: false,
            label_input,
            desktop: crate::desktop::Desktop::default(),
            sync: Default::default(),
            export: Default::default(),
            toast_task: None,
            save_task: None,
            saver,
            saves: 0,
            edits: 0,
            view_task: None,
            view_saves: 0,
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
        f._subs.push(cx.on_app_quit(|this, _| {
            this.save_and_wait();
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

    /// What an action on note `id` applies to: every selected note when
    /// `id` is one of them, otherwise just `id`.
    pub fn targets(&self, id: Id) -> Vec<Id> {
        if self.is_selected(Selection::Sticker(id)) {
            self.selected_notes()
        } else {
            vec![id]
        }
    }

    /// Adds `s` to the selection, or takes it out if it's there (Ctrl or
    /// Shift and a click). Says whether it's selected now.
    pub fn toggle_selected(&mut self, s: Selection) -> bool {
        if self.is_selected(s) {
            self.selection.retain(|x| *x != s);
            false
        } else {
            self.selection.push(s);
            true
        }
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
        let before = cx.try_global::<theme::ActiveTheme>().map(|a| (a.0.is_dark, a.0.colors.canvas));
        theme::set(self.dark(), s.high_contrast, s.ui_scale, cx);
        sync_component_theme(cx);
        // Light to dark or back: the old look fades off the window.
        if let Some((was_dark, canvas)) = before {
            if was_dark != self.dark() && !super::motion::reduced() {
                self.exits.theme = Some((canvas, Instant::now()));
                self.exits.theme_n += 1;
                let ms = super::motion::ms(theme(cx).motion.theme_fade);
                cx.spawn(async move |this, cx| {
                    cx.background_executor().timer(Duration::from_millis(ms + 30)).await;
                    let _ = this.update(cx, |_, cx| cx.notify());
                })
                .detach();
            }
        }
        cx.refresh_windows();
    }

    /// The old canvas colour over the window, fading off after a switch
    /// between light and dark. It takes no clicks.
    fn render_theme_fade(&mut self, t: &theme::Theme) -> Option<gpui::AnyElement> {
        let (color, at) = self.exits.theme?;
        if at.elapsed() >= Duration::from_millis(super::motion::ms(t.motion.theme_fade)) {
            self.exits.theme = None;
            return None;
        }
        Some(super::motion::fade_off(div().absolute().inset_0().bg(color), ("theme-fade", self.exits.theme_n), t.motion.theme_fade, 0.85))
    }

    pub fn toggle_theme(&mut self, cx: &mut Context<Self>) {
        self.board.settings.theme = if self.dark() { ThemeMode::Light } else { ThemeMode::Dark };
        self.apply_theme(cx);
        self.schedule_save(cx);
    }

    // ---------- History and saving ----------

    /// Records `before` as an undo step, and saves soon.
    pub fn commit(&mut self, before: Board, cx: &mut Context<Self>) {
        let step = Change::diff(&before, &self.board);
        let moved = step.is_empty() && before != self.board;
        self.record(step, moved, cx, |step, board, id| step.restamp(&before, board, id));
    }

    /// As `commit`, from a copy of only the sheets the edit could change.
    pub fn commit_since(&mut self, before: Before, cx: &mut Context<Self>) {
        let step = Change::since(&before, &self.board);
        let moved = step.is_empty() && before.differs(&self.board);
        self.record(step, moved, cx, |step, board, id| step.restamp_since(&before, board, id));
    }

    /// Keeps `step` for undo and saves. An empty step that `moved` the view
    /// or the settings is saved, with nothing to undo.
    fn record(&mut self, mut step: Change, moved: bool, cx: &mut Context<Self>, restamp: impl FnOnce(&mut Change, &Board, Id)) {
        if step.is_empty() {
            if moved {
                self.schedule_save(cx);
                self.desktop_sync(cx);
            }
            return;
        }
        // Whatever changed was edited now.
        let t = now();
        let active = self.board.active;
        if let Some(c) = self.board.categories.get_mut(active) {
            c.edited = t;
            let id = c.id;
            restamp(&mut step, &self.board, id);
        }
        self.push_undo(step);
        self.schedule_save(cx);
        self.desktop_sync(cx);
    }

    /// A new undo step; redo is over.
    pub fn push_undo(&mut self, step: Change) {
        self.undo.push(Arc::new(step));
        if self.undo.len() > UNDO_DEPTH {
            self.undo.remove(0);
        }
        self.redo.clear();
    }

    /// Runs `f` on the board as one undoable step.
    pub fn change(&mut self, cx: &mut Context<Self>, f: impl FnOnce(&mut Board)) {
        let before = self.board.clone();
        f(&mut self.board);
        self.commit(before, cx);
        cx.notify();
    }

    pub fn undo(&mut self, cx: &mut Context<Self>) {
        if self.editing.is_some() {
            return;
        }
        if let Some(step) = self.undo.pop() {
            step.apply(&mut self.board, Way::Back);
            self.selection.clear();
            self.redo.push(step);
            self.schedule_save(cx);
            self.desktop_sync(cx);
            cx.notify();
        } else {
            self.notify("Nothing to undo", cx);
        }
    }

    pub fn redo(&mut self, cx: &mut Context<Self>) {
        if let Some(step) = self.redo.pop() {
            step.apply(&mut self.board, Way::Forward);
            self.selection.clear();
            self.undo.push(step);
            self.schedule_save(cx);
            self.desktop_sync(cx);
            cx.notify();
        }
    }

    /// After the view moved (a pan, a zoom): where the sheets are scrolled
    /// to is written a moment after it stops, to its own small file. The
    /// board isn't saved and nothing goes to Drive.
    pub fn schedule_view_save(&mut self, cx: &mut Context<Self>) {
        self.view_task = Some(cx.spawn(async move |this, cx| {
            cx.background_executor().timer(Duration::from_millis(350)).await;
            let _ = this.update(cx, |this, cx| {
                this.view_task = None;
                let (n, text) = this.next_view();
                let saver = this.saver.clone();
                cx.background_executor().spawn(async move { saver.write_view(n, &text) }).detach();
            });
        }));
    }

    /// The view's next save number, and its text.
    fn next_view(&mut self) -> (u64, String) {
        self.view_saves += 1;
        (self.view_saves, store::view_text(&self.board))
    }

    pub fn schedule_save(&mut self, cx: &mut Context<Self>) {
        self.edits += 1;
        self.saved = false;
        self.save_task = Some(cx.spawn(async move |this, cx| {
            cx.background_executor().timer(Duration::from_millis(350)).await;
            let _ = this.update(cx, |this, cx| this.save_now(cx));
        }));
    }

    /// Saves now, off the UI thread: the board is copied here, turned to
    /// text and written to the disk in the background.
    pub fn save_now(&mut self, cx: &mut Context<Self>) {
        self.edits += 1;
        self.save_task = None;
        self.saved = false;
        self.saves += 1;
        let n = self.saves;
        let board = self.board.clone();
        let steps = self.undo.clone();
        let saver = self.saver.clone();
        let (view_n, view) = self.next_view();
        let job = cx.background_executor().spawn(async move {
            let _ = saver.write_view(view_n, &view);
            saver.write(n, &board, &steps)
        });
        cx.spawn(async move |this, cx| {
            let result = job.await;
            let _ = this.update(cx, |this, cx| this.saved_now(n, result, cx));
        })
        .detach();
    }

    fn saved_now(&mut self, n: u64, result: std::io::Result<bool>, cx: &mut Context<Self>) {
        match result {
            // Saved, and nothing newer is on its way.
            Ok(_) if n == self.saves && self.save_task.is_none() => {
                self.saved = true;
                self.drive_after_save(cx);
            }
            Ok(_) => {}
            Err(e) => self.notify(format!("Couldn't save: {e}"), cx),
        }
        cx.notify();
    }

    /// Saves before Folio closes, waiting for the disk: nothing is left in
    /// the background.
    pub fn save_and_wait(&mut self) {
        self.save_task = None;
        self.view_task = None;
        let (view_n, view) = self.next_view();
        let _ = self.saver.write_view(view_n, &view);
        self.saves += 1;
        if self.saver.write(self.saves, &self.board, &self.undo).is_ok() {
            self.saved = true;
        }
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
            self.schedule_view_save(cx);
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
            Dialog::GoogleSetup => {
                let id = self.client_input.read(cx).value().trim().to_string();
                let secret = self.secret_input.read(cx).value().trim().to_string();
                if !crate::cloud::ClientId::looks_right(&id) {
                    self.dialog = Some(d);
                    self.notify("That client ID should end in .apps.googleusercontent.com", cx);
                    return;
                }
                match (crate::cloud::ClientId { id, secret }).save(&self.dir) {
                    Ok(()) => self.sign_in_google(window, cx),
                    Err(e) => self.notify(format!("Couldn't keep it: {e}"), cx),
                }
            }
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
        let before = Before::open(&self.board);
        let id = self.board.fresh_id();
        let set = &self.board.settings;
        let paper = self.sheet().next_paper(set.default_paper);
        let (style, font) = self.sheet().next_look(set.default_style, set.default_font);
        let (w, h) = (240., 120.);
        let mut note = Sticker {
            id,
            rect: Rect { x: at.x - w / 2., y: at.y - 30., w, h },
            paper,
            font,
            edited: now(),
            ..Default::default()
        };
        note.set_style(style);
        self.board.sheet_mut().stickers.push(note);
        self.before = Some(before);
        self.select(Selection::Sticker(id));
        self.begin_edit(id, true, window, cx);
    }

    /// A new note of type `style` at `at` (world), being typed into.
    pub fn new_note_styled(&mut self, at: Vec2, style: NoteStyle, window: &mut Window, cx: &mut Context<Self>) {
        self.new_note_at(at, window, cx);
        // Part of the new note's undo step.
        if let Some(s) = self.editing.and_then(|ed| self.board.find_note_mut(ed.id)) {
            s.set_style(style);
        }
        cx.notify();
    }

    /// A new note in the middle of the view.
    pub fn new_note(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.screen != Screen::Sheet {
            let ix = self.board.active;
            self.open_sheet(ix, window, cx);
        }
        let (w, h) = self.viewport_size();
        let (w, h) = if w < 10. { (1000., 600.) } else { (w, h) };
        // In the middle of the view, or the nearest free place around it:
        // never on top of another note or text.
        let cam = self.sheet().camera;
        let c = cam.to_world(v2(w / 2., h / 2.));
        let (a, b) = (cam.to_world(v2(24., 90.)), cam.to_world(v2(w - 24., h - 110.)));
        let view = Rect { x: a.x, y: a.y, w: b.x - a.x, h: b.y - a.y };
        let taken: Vec<Rect> = self
            .sheet()
            .stickers
            .iter()
            .map(|s| self.note_rect(s))
            .chain(self.sheet().labels.iter().map(|l| self.label_box(l)))
            .collect();
        let spot = open_spot(&taken, Rect { x: c.x - 120., y: c.y - 75., w: 240., h: 150. }, view);
        self.new_note_at(v2(spot.x + 120., spot.y + 30.), window, cx);
    }

    /// Starts typing into note `id` on the sheet.
    pub fn begin_edit(&mut self, id: Id, is_new: bool, window: &mut Window, cx: &mut Context<Self>) {
        let Some(s) = self.sheet().sticker(id) else { return };
        if s.locked {
            self.notify("This note is locked · unlock it from its menu", cx);
            return;
        }
        let (title, body) = (s.title.clone(), s.body.clone());
        let hint = match s.style {
            NoteStyle::Cheat => "Key terms in **bold** · > for a Remember box · | for a table",
            NoteStyle::Index => "One thing per line · - [ ] for a task",
            _ => "Write… - [ ] for a task, #tag, **bold**",
        };
        if self.before.is_none() {
            self.before = Some(Before::open(&self.board));
        }
        self.title_input.update(cx, |i, cx| i.set_value(title.clone(), window, cx));
        // The cursor goes to the end of the text, ready to add to it.
        let end = body.len();
        self.body_input.update(cx, |i, cx| {
            i.set_placeholder(hint, window, cx);
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
        self.edit_papers = false;
        let title = self.title_input.read(cx).value().trim().to_string();
        let body = self.body_input.read(cx).value().trim_end().to_string();
        let before = self.before.take().unwrap_or_else(|| Before::open(&self.board));
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
        self.commit_since(before, cx);
        cx.notify();
    }

    /// Changes note `id` as one undoable step (also while typing into it).
    pub fn change_note(&mut self, id: Id, cx: &mut Context<Self>, f: impl FnOnce(&mut Sticker)) {
        let t = now();
        if self.editing.is_some() {
            // The edit's snapshot covers it (a note on another sheet, from
            // its desktop window, gets its sheet kept first).
            if let (Some(b), Some(ix)) = (self.before.as_mut(), self.board.sheet_of(id)) {
                b.keep(&self.board, ix);
            }
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

    /// Adds a task line to the note being typed into (the line the cursor
    /// is on becomes a task), or to `id`.
    pub fn add_task(&mut self, id: Id, window: &mut Window, cx: &mut Context<Self>) {
        if self.editing.map(|e| e.id) == Some(id) {
            self.format_editing(Format::Tasks, window, cx);
        } else {
            self.change_note(id, cx, |s| {
                let sep = if s.body.is_empty() || s.body.ends_with('\n') { "" } else { "\n" };
                s.body = format!("{}{sep}- [ ] New task", s.body);
            });
        }
    }

    /// A writing command from the edit bar: on the note's text, which
    /// keeps the cursor.
    pub fn format_editing(&mut self, f: Format, window: &mut Window, cx: &mut Context<Self>) {
        if self.editing.is_none() {
            return;
        }
        format_input(&self.title_input, &self.body_input, f, window, cx);
        self.body_input.update(cx, |i, cx| i.focus(window, cx));
        cx.notify();
    }

    /// A writing shortcut in the note being typed into.
    pub fn format_key(&mut self, f: Format, window: &mut Window, cx: &mut Context<Self>) {
        if self.editing.is_some() {
            format_key_input(&self.title_input, &self.body_input, f, window, cx);
            cx.notify();
        }
    }

    /// Sets the type of these notes (the one being typed into, while
    /// editing) as one undoable step.
    pub fn set_style(&mut self, ids: &[Id], style: NoteStyle, cx: &mut Context<Self>) {
        if let Some(ed) = self.editing {
            self.change_note(ed.id, cx, |s| s.set_style(style));
            return;
        }
        let t = now();
        self.change(cx, |b| {
            for id in ids {
                if let Some(s) = b.find_note_mut(*id) {
                    s.set_style(style);
                    s.edited = t;
                }
            }
        });
    }

    /// Changes what these notes are pinned up with (the one being typed
    /// into, while editing), as one undoable step.
    pub fn set_pin(&mut self, ids: &[Id], cx: &mut Context<Self>, f: impl Fn(Option<Pin>) -> Option<Pin>) {
        if let Some(ed) = self.editing {
            self.change_note(ed.id, cx, |s| s.pin = f(s.pin));
            return;
        }
        let t = now();
        self.change(cx, |b| {
            for id in ids {
                if let Some(s) = b.find_note_mut(*id) {
                    s.pin = f(s.pin);
                    s.edited = t;
                }
            }
        });
    }

    /// S on the sheet: the selected notes take the next type.
    pub fn cycle_style(&mut self, cx: &mut Context<Self>) {
        let ids = self.selected_notes();
        let Some(first) = ids.first().and_then(|id| self.board.find_note(*id)) else { return };
        let next = first.style.next();
        self.set_style(&ids, next, cx);
        self.notify(format!("{} · S for the next type", next.label()), cx);
    }

    /// Text into Markdown, on these notes: an untitled note's first line
    /// becomes its title.
    pub fn markdown_notes(&mut self, ids: &[Id], cx: &mut Context<Self>) {
        let t = now();
        let mut changed = 0;
        self.change(cx, |b| {
            for id in ids {
                let Some(s) = b.find_note_mut(*id) else { continue };
                let (head, body) = if s.title.trim().is_empty() { markdown::tidy(&s.body) } else { (None, markdown::tidy_body(&s.body)) };
                if body != s.body || head.is_some() {
                    s.body = body;
                    if let Some(h) = head {
                        s.title = h;
                    }
                    s.edited = t;
                    changed += 1;
                }
            }
        });
        let msg = match changed {
            0 => "Nothing to change: already Markdown".to_string(),
            1 => format!("Turned into Markdown · {} to undo", shortcut("Z")),
            n => format!("{n} notes turned into Markdown · {} to undo", shortcut("Z")),
        };
        self.notify(msg, cx);
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

    /// Puts notes in the Trash (or the Archive), as one undo step.
    pub fn bin_notes(&mut self, ids: &[Id], archived: bool, cx: &mut Context<Self>) {
        self.menu = None;
        if self.editing.is_some_and(|e| ids.contains(&e.id)) {
            self.finish_edit(cx);
        }
        let t = now();
        self.change(cx, |b| {
            for &id in ids {
                b.bin_note(id, archived, t);
            }
        });
        self.selection.retain(|s| !matches!(s, Selection::Sticker(id) if ids.contains(id)));
        let what = match (archived, ids.len()) {
            (true, 1) => "Archived".to_string(),
            (true, n) => format!("{n} notes archived"),
            (false, 1) => "Moved to Trash".to_string(),
            (false, n) => format!("{n} notes moved to Trash"),
        };
        self.notify(format!("{what} · {} to undo", shortcut("Z")), cx);
    }

    /// Changes every note in `ids` as one undo step.
    pub fn change_notes(&mut self, ids: &[Id], cx: &mut Context<Self>, f: impl Fn(&mut Sticker)) {
        if let [id] = ids {
            return self.change_note(*id, cx, f);
        }
        self.finish_edit(cx);
        let t = now();
        self.change(cx, |b| {
            for &id in ids {
                if let Some(s) = b.find_note_mut(id) {
                    f(s);
                    s.edited = t;
                }
            }
        });
    }

    /// Locks the notes, or unlocks them when they're all locked already.
    pub fn lock_notes(&mut self, ids: &[Id], cx: &mut Context<Self>) {
        self.menu = None;
        let lock = !ids.iter().all(|id| self.board.find_note(*id).is_some_and(|s| s.locked));
        self.change_notes(ids, cx, |s| s.locked = lock);
        if ids.len() > 1 {
            self.notify(format!("{} notes {}", ids.len(), if lock { "locked" } else { "unlocked" }), cx);
        }
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

    /// Moves notes to sheet `to`, as one undo step.
    pub fn move_notes_to(&mut self, ids: &[Id], to: usize, cx: &mut Context<Self>) {
        self.menu = None;
        self.finish_edit(cx);
        let name = self.board.categories.get(to).map(|c| c.name.clone()).unwrap_or_default();
        let mut moved = 0;
        self.change(cx, |b| moved = ids.iter().filter(|&&id| b.move_note(id, to)).count());
        if moved > 0 {
            self.selection.retain(|s| !matches!(s, Selection::Sticker(id) if ids.contains(id)));
            let what = if moved == 1 { "Moved".to_string() } else { format!("Moved {moved} notes") };
            self.notify(format!("{what} to “{name}”"), cx);
        }
    }

    pub fn set_reminder(&mut self, id: Id, due: Option<u64>, cx: &mut Context<Self>) {
        self.set_reminders(&[id], due, cx);
    }

    pub fn set_reminders(&mut self, ids: &[Id], due: Option<u64>, cx: &mut Context<Self>) {
        self.menu = None;
        self.change_notes(ids, cx, |s| {
            s.due = due;
            s.reminded = false;
        });
        if let Some(d) = due {
            self.notify(format!("Reminder set · {}", time::due_label(d, now())), cx);
        }
    }

    /// Ctrl+C: words picked out of a note's text if there are any, else
    /// the selected notes and text on the sheet.
    pub fn copy_selection(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if gpui::base::TextSelection::has_selection(window, cx) {
            let text = gpui::base::TextSelection::selected_text(window, cx);
            if !text.is_empty() {
                cx.write_to_clipboard(ClipboardItem::new_string(text));
                self.notify("Copied", cx);
                return;
            }
        }
        if let Some((notes, texts)) = self.copy_notes(false, cx) {
            self.notify(format!("Copied {} · {} pastes", things(notes, texts), shortcut("V")), cx);
        }
    }

    /// Ctrl+X: the selected notes and text go on the clipboard and off the
    /// sheet (the notes into the Trash, until they're pasted).
    pub fn cut_selection(&mut self, cx: &mut Context<Self>) {
        let Some((notes, texts)) = self.copy_notes(true, cx) else { return };
        self.delete_selection(cx);
        self.notify(format!("Cut {} · {} pastes", things(notes, texts), shortcut("V")), cx);
    }

    /// Puts the selected notes and text on the clipboard: their text for
    /// other apps, the notes themselves for Folio. How many notes and
    /// texts, if any.
    pub fn copy_notes(&mut self, cut: bool, cx: &mut Context<Self>) -> Option<(usize, usize)> {
        self.menu = None;
        self.finish_edit(cx);
        let sheet = self.sheet();
        let notes: Vec<Sticker> = sheet.stickers.iter().filter(|s| self.is_selected(Selection::Sticker(s.id))).cloned().collect();
        let labels: Vec<Label> = sheet.labels.iter().filter(|l| self.is_selected(Selection::Label(l.id))).cloned().collect();
        if notes.is_empty() && labels.is_empty() {
            return None;
        }
        let ids: std::collections::HashSet<Id> = notes.iter().map(|s| s.id).collect();
        let links = sheet.links.iter().filter(|l| ids.contains(&l.from) && ids.contains(&l.to)).cloned().collect();
        let mut text = notes.iter().map(|s| s.text()).chain(labels.iter().map(|l| l.text.clone())).filter(|t| !t.trim().is_empty()).collect::<Vec<_>>().join("\n\n");
        if text.is_empty() {
            // Pictures alone: still something to paste, and to know as ours.
            text = things(notes.len(), labels.len());
        }
        cx.write_to_clipboard(ClipboardItem::new_string(text.clone()));
        let n = (notes.len(), labels.len());
        self.clip = Some(Clip { text, notes, links, labels, cut });
        Some(n)
    }

    /// Makes the copied notes again, centred where the pointer is (or the
    /// middle of the view), selected, as one undo step.
    fn paste_clip(&mut self, cx: &mut Context<Self>) {
        let Some(clip) = self.clip.as_mut() else { return };
        let cut = std::mem::take(&mut clip.cut);
        let clip = clip.clone();
        let (w, h) = self.viewport_size();
        let inside = self.cursor.x > 0. && self.cursor.y > 0. && self.cursor.x < w && self.cursor.y < h;
        let cam = self.sheet().camera;
        let target = cam.to_world(if inside { self.cursor } else { v2(w / 2., h / 2.) });
        let Some(bounds) = clip.notes.iter().map(|s| s.rect).chain(clip.labels.iter().map(label_rect)).reduce(|a, b| a.union(&b)) else { return };
        let d = target.sub(bounds.center());
        let t = now();
        let mut picked = Vec::new();
        self.change(cx, |b| {
            if cut {
                let gone: std::collections::HashSet<Id> = clip.notes.iter().map(|s| s.id).collect();
                b.bin.retain(|x| !matches!(&x.item, BinItem::Note { sticker, .. } if gone.contains(&sticker.id)));
            }
            let mut ids = HashMap::new();
            for s in &clip.notes {
                let id = b.fresh_id();
                ids.insert(s.id, id);
                let mut n = s.clone();
                n.id = id;
                n.rect.x += d.x;
                n.rect.y += d.y;
                n.edited = t;
                n.popped = None;
                if !cut {
                    // A copy, like a duplicate, has no reminder of its own.
                    n.due = None;
                    n.reminded = false;
                }
                b.sheet_mut().stickers.push(n);
                picked.push(Selection::Sticker(id));
            }
            for l in &clip.links {
                if let (Some(&from), Some(&to)) = (ids.get(&l.from), ids.get(&l.to)) {
                    let id = b.fresh_id();
                    b.sheet_mut().links.push(Link { id, from, to, ink: l.ink });
                }
            }
            for l in &clip.labels {
                let mut n = l.clone();
                n.id = b.fresh_id();
                n.at = n.at.add(d);
                picked.push(Selection::Label(n.id));
                b.sheet_mut().labels.push(n);
            }
        });
        self.selection = picked;
        self.notify(format!("Pasted {}", things(clip.notes.len(), clip.labels.len())), cx);
        cx.notify();
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
        let before = Before::open(&self.board);
        let id = self.board.fresh_id();
        let ink = self.ink;
        let label = Label::new(id, v2(at.x, at.y - LabelSize::M.line() / 2.), ink);
        self.board.sheet_mut().labels.push(label);
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
            self.before = Some(Before::open(&self.board));
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
        self.label_palette = false;
        let text = self.label_input.read(cx).value().trim_end().to_string();
        let before = self.before.take().unwrap_or_else(|| Before::open(&self.board));
        let sheet = self.board.sheet_mut();
        if text.trim().is_empty() {
            sheet.labels.retain(|l| l.id != id);
        } else if let Some(l) = sheet.labels.iter_mut().find(|l| l.id == id) {
            l.text = text;
        }
        self.commit_since(before, cx);
        cx.notify();
    }

    /// A writing command in the text being written on the sheet.
    pub fn format_label(&mut self, f: Format, window: &mut Window, cx: &mut Context<Self>) {
        if self.label_edit.is_none() {
            return;
        }
        format_text(&self.label_input, f, window, cx);
        self.label_input.update(cx, |i, cx| i.focus(window, cx));
        cx.notify();
    }

    /// Changes label `id`'s look: while it's being written, as part of
    /// that edit (one undo step when it's done); otherwise as a step now.
    pub fn set_label(&mut self, id: Id, cx: &mut Context<Self>, f: impl FnOnce(&mut Label)) {
        if self.label_edit == Some(id) {
            if let Some(l) = self.board.sheet_mut().labels.iter_mut().find(|l| l.id == id) {
                f(l);
            }
            cx.notify();
            return;
        }
        self.change(cx, |b| {
            if let Some(l) = b.sheet_mut().labels.iter_mut().find(|l| l.id == id) {
                f(l);
            }
        });
    }

    /// Where label `l` lies: as measured when drawn, or estimated.
    pub fn label_box(&self, l: &Label) -> Rect {
        match self.label_sizes.borrow().get(&l.id) {
            Some(&(w, h)) => Rect { x: l.at.x, y: l.at.y, w, h },
            None => label_rect(l),
        }
    }

    /// The top label under `p` (world).
    pub fn label_at(&self, p: Vec2) -> Option<Id> {
        self.sheet().labels.iter().rev().find(|l| self.label_box(l).contains(p)).map(|l| l.id)
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

    /// Changes what the open sheet is drawn on, as one undoable step.
    pub fn set_backdrop(&mut self, cx: &mut Context<Self>, f: impl FnOnce(&mut Backdrop)) {
        let t = now();
        self.change(cx, |b| {
            let c = b.sheet_mut();
            f(&mut c.backdrop);
            c.edited = t;
        });
    }

    /// A picture (or SVG) from this computer behind the open sheet,
    /// softened to start with.
    pub fn pick_backdrop(&mut self, cx: &mut Context<Self>) {
        self.menu = None;
        let rx = cx.prompt_for_paths(gpui::PathPromptOptions {
            files: true,
            directories: false,
            multiple: false,
            prompt: Some("Background picture".into()),
        });
        cx.spawn(async move |this, cx| {
            let Ok(Ok(Some(paths))) = rx.await else { return };
            let Some(path) = paths.into_iter().next() else { return };
            let _ = this.update(cx, |this, cx| {
                if !store::is_image(&path) {
                    this.notify("That file isn't a picture Folio can show (PNG, JPEG, GIF, WebP, SVG)", cx);
                    return;
                }
                let file_id = this.board.fresh_id();
                match store::import_image(&this.dir, &path, file_id) {
                    Ok(name) => this.set_backdrop(cx, |b| {
                        b.image = Some(name);
                        b.soften = true;
                    }),
                    Err(e) => this.notify(format!("Couldn't use the picture: {e}"), cx),
                }
            });
        })
        .detach();
    }

    /// The open sheet's background on every sheet.
    pub fn backdrop_everywhere(&mut self, cx: &mut Context<Self>) {
        self.menu = None;
        let bd = self.sheet().backdrop.clone();
        self.change(cx, |b| {
            for c in &mut b.categories {
                c.backdrop = bd.clone();
            }
        });
        self.notify(format!("Every sheet has this background · {} to undo", shortcut("Z")), cx);
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
    /// other text a note; with `tidy`, its text turned into Markdown.
    pub fn paste(&mut self, tidy: bool, cx: &mut Context<Self>) {
        let Some(item) = cx.read_from_clipboard() else { return };
        // Notes copied here, while the clipboard still holds their text.
        if !tidy && self.clip.as_ref().is_some_and(|c| item.text().as_deref() == Some(c.text.as_str())) {
            self.paste_clip(cx);
            return;
        }
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
                    let set = self.board.settings.clone();
                    let structured = !tidy && markdown::looks_structured(&text);
                    let (title, body) = if tidy { markdown::tidy(&text) } else { (None, text) };
                    self.change(cx, |b| {
                        new_id = b.fresh_id();
                        let paper = b.sheet().next_paper(set.default_paper);
                        let (style, font) = b.sheet().next_look(set.default_style, set.default_font);
                        let mut note = Sticker {
                            id: new_id,
                            rect: Rect { x: at.x - 120., y: at.y - 60., w: 240., h: 120. },
                            paper,
                            font,
                            title: title.unwrap_or_default(),
                            body,
                            edited: now(),
                            ..Default::default()
                        };
                        note.set_style(style);
                        b.sheet_mut().stickers.push(note);
                    });
                    self.select(Selection::Sticker(new_id));
                    if structured {
                        self.notify(format!("Pasted as it was · {} pastes it as Markdown", shortcut("Shift+V")), cx);
                    }
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
        self.schedule_view_save(cx);
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
        self.schedule_view_save(cx);
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
                self.schedule_view_save(cx);
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

impl Folio {
    /// Notes what's open this frame. What just closed starts leaving, and
    /// a frame is asked for once it has gone, so nothing lingers unseen.
    fn follow_exits(&mut self, cx: &mut Context<Self>) {
        let m = theme(cx).motion;
        let menu = self.menu.as_ref().map(|m| (m.at, m.kind, m.page));
        let mut gone = Vec::new();
        if self.exits.menu.track(menu.as_ref()) {
            gone.push(m.pop_out);
        }
        if self.exits.dialog.track(self.dialog.as_ref()) {
            gone.push(m.dialog_out);
        }
        if self.exits.toast.track(self.toast.as_ref()) {
            gone.push(m.toast_out);
        }
        if self.exits.drawer.track(self.drawer.then_some(&())) {
            gone.push(m.drawer_out);
        }
        if self.exits.palette.track(self.palette.then_some(&())) {
            gone.push(m.search_out);
        }
        if self.exits.edit.track(self.editing.map(|e| e.id).as_ref()) {
            gone.push(m.pop_out);
        }
        let inks = self.screen == Screen::Sheet && self.tool.uses_ink();
        if self.exits.inks.track(inks.then_some(&())) {
            gone.push(m.pop_out);
        }
        if self.exits.ink_palette.track((inks && self.ink_palette).then_some(&())) {
            gone.push(m.pop_out);
        }
        if self.exits.welcome.track(self.welcome.as_ref()) {
            gone.push(m.dialog_out);
        }
        for ms in gone {
            cx.spawn(async move |this, cx| {
                cx.background_executor().timer(Duration::from_millis(super::motion::ms(ms) + 30)).await;
                let _ = this.update(cx, |_, cx| cx.notify());
            })
            .detach();
        }
    }
}

impl Render for Folio {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let t = theme(cx).clone();
        self.track_place();
        self.follow_exits(cx);
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
                this.save_and_wait();
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
            .children(self.render_drawer_leaving(cx))
            .children(self.dialog.is_some().then(|| self.render_dialog(window, cx)))
            .children(self.render_dialog_leaving(cx))
            .children(self.palette.then(|| self.render_palette_scrim(cx)))
            .children(self.render_palette_scrim_leaving(cx))
            .children(self.menu.is_some().then(|| self.render_menu(window, cx)))
            .children(self.render_menu_leaving(cx))
            .children(self.toast.clone().map(|msg| toast(&t, msg)))
            .children(self.exits.toast.leaving(t.motion.toast_out).map(|(msg, n)| toast_leaving(&t, msg, n)))
            .children(self.render_theme_fade(&t))
            // The first time, the welcome rises in as the opening fades
            // away over it.
            .children((self.welcome.is_some() && self.intro.as_ref().is_none_or(|i| i.revealing())).then(|| self.render_welcome(window, cx)))
            .children(self.render_welcome_leaving(window, cx))
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

/// What was copied, in words: "the note", "3 notes", "the text", "4 things".
fn things(notes: usize, texts: usize) -> String {
    match (notes, texts) {
        (1, 0) => "the note".into(),
        (n, 0) => format!("{n} notes"),
        (0, 1) => "the text".into(),
        (0, n) => format!("{n} texts"),
        (a, b) => format!("{} things", a + b),
    }
}
