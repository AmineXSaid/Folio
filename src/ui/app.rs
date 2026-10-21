//! The window's one view: the board being edited, the tool in hand, the
//! gesture under way, undo history, and the debounced save. The other files
//! in `ui/` each render one part of it.

use std::cell::Cell;
use std::path::PathBuf;
use std::rc::Rc;
use std::time::Duration;

use gpui::component::input::{InputState, TextareaState};
use gpui::{
    div, prelude::*, Bounds, ClipboardEntry, Context, Entity, FocusHandle, Pixels,
    SharedString, Task, Window,
};

use crate::model::*;
use crate::store;
use crate::theme::{self, theme};
use crate::ui::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tool {
    Select,
    Hand,
    Pen,
    Marker,
    Text,
    Connect,
    Eraser,
}

impl Tool {
    pub const ALL: [Tool; 7] = [
        Tool::Select,
        Tool::Hand,
        Tool::Pen,
        Tool::Marker,
        Tool::Text,
        Tool::Connect,
        Tool::Eraser,
    ];
    pub fn icon(self) -> &'static str {
        match self {
            Tool::Select => "icons/mouse-pointer-2.svg",
            Tool::Hand => "icons/hand.svg",
            Tool::Pen => "icons/pencil.svg",
            Tool::Marker => "icons/highlighter.svg",
            Tool::Text => "icons/type.svg",
            Tool::Connect => "icons/spline.svg",
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
}

/// What the pointer is doing between press and release.
pub enum Gesture {
    None,
    Pan { start: Vec2, pan: Vec2 },
    Move { id: Id, grab: Vec2, moved: bool },
    MoveLabel { id: Id, grab: Vec2, moved: bool },
    Resize { id: Id, start: Vec2, size: (f32, f32) },
    Draw(Stroke),
    Erase { erased: bool },
    Connect { from: Id, to: Vec2 },
}

/// The open dialog: a sticker's or a label's editor.
pub struct Editing {
    pub target: Selection,
    /// Made by the gesture that opened the editor: Cancel removes it again.
    pub is_new: bool,
    pub paper: Paper,
    pub text_size: TextSize,
    pub image: Option<String>,
    pub ink: Ink,
}

pub struct Folio {
    pub board: Board,
    pub dir: PathBuf,
    pub focus: FocusHandle,
    pub tool: Tool,
    pub ink: Ink,
    pub gesture: Gesture,
    /// The undo snapshot taken when the current gesture began.
    pub before: Option<Board>,
    pub selected: Option<Selection>,
    pub hovered: Option<Id>,
    /// The last pointer position over the sheet, in sheet-local pixels.
    pub cursor: Vec2,
    /// Where the sheet was laid out last frame, for turning window
    /// positions into sheet positions.
    pub viewport: Rc<Cell<Bounds<Pixels>>>,
    pub undo: Vec<Board>,
    pub redo: Vec<Board>,
    pub editing: Option<Editing>,
    pub renaming: Option<Id>,
    pub toast: Option<SharedString>,
    pub saved: bool,
    pub title_input: Entity<InputState>,
    pub body_input: Entity<TextareaState>,
    pub rename_input: Entity<InputState>,
    pub search_input: Entity<InputState>,
    toast_task: Option<Task<()>>,
    save_task: Option<Task<()>>,
    _subs: Vec<gpui::Subscription>,
}

const UNDO_DEPTH: usize = 100;

impl Folio {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let dir = store::data_dir();
        let board = store::load(&dir);
        theme::set(board.dark, cx);
        sync_component_theme(cx);
        let title_input = cx.new(|cx| InputState::new(window, cx).placeholder("A thought worth keeping…"));
        let body_input = cx.new(|cx| {
            TextareaState::new(window, cx)
                .auto_grow(6, 14)
                .placeholder("Write something… **bold**, - [ ] tasks, | tables |")
        });
        let rename_input = cx.new(|cx| InputState::new(window, cx).placeholder("Sheet name"));
        let search_input = cx.new(|cx| InputState::new(window, cx).placeholder("Search this sheet"));
        let subs = vec![
            cx.subscribe_in(&rename_input, window, |this, _, ev, window, cx| {
                use gpui::component::input::InputEvent;
                match ev {
                    InputEvent::PressEnter { .. } | InputEvent::Blur => this.finish_rename(window, cx),
                    _ => {}
                }
            }),
            cx.observe(&search_input, |_, _, cx| cx.notify()),
        ];
        let focus = cx.focus_handle();
        window.focus(&focus, cx);
        Folio {
            board,
            dir,
            focus,
            tool: Tool::Select,
            ink: Ink::Graphite,
            gesture: Gesture::None,
            before: None,
            selected: None,
            hovered: None,
            cursor: v2(0., 0.),
            viewport: Rc::new(Cell::new(Bounds::default())),
            undo: Vec::new(),
            redo: Vec::new(),
            editing: None,
            renaming: None,
            toast: None,
            saved: true,
            title_input,
            body_input,
            rename_input,
            search_input,
            toast_task: None,
            save_task: None,
            _subs: subs,
        }
    }

    pub fn sheet(&self) -> &Category {
        self.board.sheet()
    }

    pub fn viewport_size(&self) -> (f32, f32) {
        let b = self.viewport.get();
        (f32::from(b.size.width).max(1.), f32::from(b.size.height).max(1.))
    }

    /// A window position as a sheet-local screen position.
    pub fn local(&self, p: gpui::Point<Pixels>) -> Vec2 {
        let o = self.viewport.get().origin;
        v2(f32::from(p.x - o.x), f32::from(p.y - o.y))
    }

    pub fn search(&self, cx: &gpui::App) -> String {
        self.search_input.read(cx).value().trim().to_lowercase()
    }

    // ---------- History and saving ----------

    /// Records `before` as an undo step, and saves soon.
    pub fn commit(&mut self, before: Board, cx: &mut Context<Self>) {
        if before == self.board {
            return;
        }
        self.undo.push(before);
        if self.undo.len() > UNDO_DEPTH {
            self.undo.remove(0);
        }
        self.redo.clear();
        self.schedule_save(cx);
    }

    /// Runs `f` on the board as one undoable step.
    pub fn change(&mut self, cx: &mut Context<Self>, f: impl FnOnce(&mut Board)) {
        let before = self.board.clone();
        f(&mut self.board);
        self.commit(before, cx);
        cx.notify();
    }

    /// Puts `snapshot` in place, keeping each sheet's current camera: undo
    /// is about content, not about where you were looking.
    fn restore(&mut self, mut snapshot: Board) -> Board {
        for c in &mut snapshot.categories {
            if let Some(now) = self.board.categories.iter().find(|n| n.id == c.id) {
                c.camera = now.camera;
            }
        }
        snapshot.dark = self.board.dark;
        self.selected = None;
        std::mem::replace(&mut self.board, snapshot)
    }

    pub fn undo(&mut self, cx: &mut Context<Self>) {
        if let Some(prev) = self.undo.pop() {
            let now = self.restore(prev);
            self.redo.push(now);
            self.schedule_save(cx);
            cx.notify();
        } else {
            self.notify("Nothing to undo", cx);
        }
    }

    pub fn redo(&mut self, cx: &mut Context<Self>) {
        if let Some(next) = self.redo.pop() {
            let now = self.restore(next);
            self.undo.push(now);
            self.schedule_save(cx);
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
        match store::save(&self.dir, &self.board) {
            Ok(()) => self.saved = true,
            Err(e) => self.notify(format!("Couldn't save: {e}"), cx),
        }
        cx.notify();
    }

    pub fn notify(&mut self, msg: impl Into<SharedString>, cx: &mut Context<Self>) {
        self.toast = Some(msg.into());
        self.toast_task = Some(cx.spawn(async move |this, cx| {
            cx.background_executor().timer(Duration::from_millis(2200)).await;
            let _ = this.update(cx, |this, cx| {
                this.toast = None;
                cx.notify();
            });
        }));
        cx.notify();
    }

    // ---------- Sheets ----------

    pub fn switch_sheet(&mut self, ix: usize, cx: &mut Context<Self>) {
        if ix < self.board.categories.len() && ix != self.board.active {
            self.board.active = ix;
            self.selected = None;
            self.gesture = Gesture::None;
            self.schedule_save(cx);
            cx.notify();
        }
    }

    pub fn new_sheet(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let mut id = 0;
        self.change(cx, |b| id = b.add_sheet("Untitled sheet"));
        self.start_rename(id, window, cx);
    }

    pub fn delete_sheet(&mut self, cx: &mut Context<Self>) {
        if self.board.categories.len() <= 1 {
            self.notify("A workspace keeps at least one sheet", cx);
            return;
        }
        let name = self.sheet().name.clone();
        self.change(cx, |b| {
            b.remove_active_sheet();
        });
        self.notify(format!("Deleted “{name}” · {} to undo", shortcut("Z")), cx);
    }

    pub fn start_rename(&mut self, id: Id, window: &mut Window, cx: &mut Context<Self>) {
        let name = self.board.categories.iter().find(|c| c.id == id).map(|c| c.name.clone());
        if let Some(name) = name {
            self.renaming = Some(id);
            self.rename_input.update(cx, |i, cx| {
                i.set_value(name, window, cx);
                i.focus(window, cx);
            });
            cx.notify();
        }
    }

    pub fn finish_rename(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(id) = self.renaming.take() else { return };
        let name = self.rename_input.read(cx).value().trim().to_string();
        if !name.is_empty() {
            self.change(cx, |b| {
                if let Some(c) = b.categories.iter_mut().find(|c| c.id == id) {
                    c.name = name;
                }
            });
        }
        window.focus(&self.focus, cx);
        cx.notify();
    }

    // ---------- Stickers ----------

    /// A new sticker centred on `at` (world), opened in the editor.
    pub fn new_sticker_at(&mut self, at: Vec2, window: &mut Window, cx: &mut Context<Self>) {
        let before = self.board.clone();
        let id = self.board.fresh_id();
        let paper = next_paper(self.sheet());
        let (w, h) = (230., 170.);
        self.board.sheet_mut().stickers.push(Sticker {
            id,
            rect: Rect { x: at.x - w / 2., y: at.y - h / 2., w, h },
            paper,
            title: String::new(),
            body: String::new(),
            image: None,
            text_size: TextSize::Medium,
            edited: now(),
        });
        self.before = Some(before);
        self.selected = Some(Selection::Sticker(id));
        self.open_editor(Selection::Sticker(id), true, window, cx);
    }

    /// A new sticker in the middle of the view.
    pub fn new_sticker(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let (w, h) = self.viewport_size();
        let cam = self.sheet().camera;
        let n = self.sheet().stickers.len() as f32 % 6.;
        let at = cam.to_world(v2(w / 2. + n * 18., h / 2. + n * 18.));
        self.new_sticker_at(at, window, cx);
    }

    pub fn delete_selection(&mut self, cx: &mut Context<Self>) {
        match self.selected.take() {
            Some(Selection::Sticker(id)) => {
                self.change(cx, |b| b.sheet_mut().remove_sticker(id));
                self.notify(format!("Note deleted · {} to undo", shortcut("Z")), cx);
            }
            Some(Selection::Label(id)) => {
                self.change(cx, |b| b.sheet_mut().labels.retain(|l| l.id != id));
            }
            None => {}
        }
    }

    pub fn duplicate_selection(&mut self, cx: &mut Context<Self>) {
        let Some(Selection::Sticker(id)) = self.selected else { return };
        let mut new_id = 0;
        self.change(cx, |b| {
            new_id = b.fresh_id();
            let sheet = b.sheet_mut();
            if let Some(mut s) = sheet.sticker(id).cloned() {
                s.id = new_id;
                s.rect.x += 24.;
                s.rect.y += 24.;
                s.edited = now();
                sheet.stickers.push(s);
            }
        });
        self.selected = Some(Selection::Sticker(new_id));
    }

    // ---------- The editor ----------

    pub fn open_editor(&mut self, target: Selection, is_new: bool, window: &mut Window, cx: &mut Context<Self>) {
        let sheet = self.sheet();
        let (title, body, editing) = match target {
            Selection::Sticker(id) => {
                let Some(s) = sheet.sticker(id) else { return };
                (
                    s.title.clone(),
                    s.body.clone(),
                    Editing { target, is_new, paper: s.paper, text_size: s.text_size, image: s.image.clone(), ink: self.ink },
                )
            }
            Selection::Label(id) => {
                let Some(l) = sheet.labels.iter().find(|l| l.id == id) else { return };
                (
                    String::new(),
                    l.text.clone(),
                    Editing { target, is_new, paper: Paper::Plain, text_size: TextSize::Medium, image: None, ink: l.ink },
                )
            }
        };
        self.title_input.update(cx, |i, cx| i.set_value(title, window, cx));
        self.body_input.update(cx, |i, cx| i.set_value(body, window, cx));
        // Deferred: the press that opened the editor focuses the sheet
        // once its handlers have run.
        let (title_input, body_input) = (self.title_input.clone(), self.body_input.clone());
        let focus_title = matches!(target, Selection::Sticker(_));
        window.defer(cx, move |window, cx| {
            if focus_title {
                title_input.update(cx, |i, cx| i.focus(window, cx));
            } else {
                body_input.update(cx, |i, cx| i.focus(window, cx));
            }
        });
        self.editing = Some(editing);
        cx.notify();
    }

    pub fn save_editor(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(ed) = self.editing.take() else { return };
        let title = self.title_input.read(cx).value().trim().to_string();
        let body = self.body_input.read(cx).value().trim_end().to_string();
        let before = self.before.take().unwrap_or_else(|| self.board.clone());
        let sheet = self.board.sheet_mut();
        match ed.target {
            Selection::Sticker(id) => {
                if let Some(s) = sheet.sticker_mut(id) {
                    s.title = title;
                    s.body = body;
                    s.paper = ed.paper;
                    s.text_size = ed.text_size;
                    s.image = ed.image;
                    s.edited = now();
                }
            }
            Selection::Label(id) => {
                if body.trim().is_empty() {
                    sheet.labels.retain(|l| l.id != id);
                } else if let Some(l) = sheet.labels.iter_mut().find(|l| l.id == id) {
                    l.text = body;
                    l.ink = ed.ink;
                }
            }
        }
        self.commit(before, cx);
        window.focus(&self.focus, cx);
        cx.notify();
    }

    pub fn cancel_editor(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(ed) = self.editing.take() else { return };
        if ed.is_new {
            if let Some(before) = self.before.take() {
                self.board = before;
                self.selected = None;
            }
        }
        self.before = None;
        window.focus(&self.focus, cx);
        cx.notify();
    }

    pub fn delete_from_editor(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(ed) = self.editing.take() else { return };
        let before = self.before.take().unwrap_or_else(|| self.board.clone());
        match ed.target {
            Selection::Sticker(id) => self.board.sheet_mut().remove_sticker(id),
            Selection::Label(id) => self.board.sheet_mut().labels.retain(|l| l.id != id),
        }
        self.selected = None;
        self.commit(before, cx);
        window.focus(&self.focus, cx);
        if !ed.is_new {
            self.notify(format!("Deleted · {} to undo", shortcut("Z")), cx);
        }
        cx.notify();
    }

    /// Asks for a picture and puts it on the sticker being edited.
    pub fn pick_image(&mut self, cx: &mut Context<Self>) {
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
                let id = this.board.fresh_id();
                match store::import_image(&this.dir, &path, id) {
                    Ok(name) => {
                        if let Some(ed) = this.editing.as_mut() {
                            ed.image = Some(name);
                        }
                    }
                    Err(e) => this.notify(format!("Couldn't add the picture: {e}"), cx),
                }
                cx.notify();
            });
        })
        .detach();
    }

    pub fn image_path(&self, name: &str) -> PathBuf {
        store::media_dir(&self.dir).join(name)
    }

    // ---------- Bringing things in ----------

    /// Files dropped on the sheet: pictures become picture notes, text and
    /// Markdown files become notes holding their text.
    pub fn drop_paths(&mut self, paths: Vec<PathBuf>, cx: &mut Context<Self>) {
        let before = self.board.clone();
        let mut at = self.sheet().camera.to_world(self.cursor);
        let mut added = 0;
        for path in paths {
            let id = self.board.fresh_id();
            let name = path.file_stem().and_then(|s| s.to_str()).unwrap_or("").to_string();
            let sticker = if store::is_image(&path) {
                match store::import_image(&self.dir, &path, id) {
                    Ok(file) => Some(image_sticker(id, at, file, name)),
                    Err(_) => None,
                }
            } else if is_text_file(&path) {
                std::fs::read_to_string(&path).ok().map(|body| Sticker {
                    id,
                    rect: Rect { x: at.x, y: at.y, w: 280., h: 220. },
                    paper: Paper::Plain,
                    title: name,
                    body,
                    image: None,
                    text_size: TextSize::Small,
                    edited: now(),
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

    /// Paste: a picture becomes a picture note, text a note.
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
                    let file = format!("{id}.{}", img.format.extension());
                    let written = std::fs::create_dir_all(&media).and_then(|_| std::fs::write(media.join(&file), &img.bytes));
                    if written.is_ok() {
                        self.board.sheet_mut().stickers.push(image_sticker(id, at, file, "Pasted picture".into()));
                        self.selected = Some(Selection::Sticker(id));
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
                    let mut new_id = 0;
                    self.change(cx, |b| {
                        new_id = b.fresh_id();
                        let paper = next_paper(b.sheet());
                        b.sheet_mut().stickers.push(Sticker {
                            id: new_id,
                            rect: Rect { x: at.x - 120., y: at.y - 90., w: 240., h: 180. },
                            paper,
                            title: String::new(),
                            body: text,
                            image: None,
                            text_size: TextSize::Medium,
                            edited: now(),
                        });
                    });
                    self.selected = Some(Selection::Sticker(new_id));
                    return;
                }
            }
        }
    }

    // ---------- The view ----------

    pub fn set_tool(&mut self, tool: Tool, cx: &mut Context<Self>) {
        self.tool = tool;
        if !matches!(tool, Tool::Select) {
            self.selected = None;
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
        let sheet = self.board.sheet_mut();
        match sheet.content_bounds() {
            Some(r) => sheet.camera.fit(r, w, h, 80.),
            None => sheet.camera = Camera::default(),
        }
        self.schedule_save(cx);
        cx.notify();
    }

    pub fn toggle_theme(&mut self, cx: &mut Context<Self>) {
        self.board.dark = !self.board.dark;
        theme::set(self.board.dark, cx);
        sync_component_theme(cx);
        self.schedule_save(cx);
        cx.refresh_windows();
    }
}

impl Render for Folio {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let t = theme(cx).clone();
        div()
            .id("folio")
            .key_context("Folio")
            .size_full()
            .relative()
            .flex()
            .font_family(t.font)
            .text_size(t.text.body)
            .text_color(t.colors.text)
            .bg(t.colors.canvas)
            .on_action(cx.listener(|this, _: &Undo, _, cx| this.undo(cx)))
            .on_action(cx.listener(|this, _: &Redo, _, cx| this.redo(cx)))
            .on_action(cx.listener(|this, _: &ToggleTheme, _, cx| this.toggle_theme(cx)))
            .on_action(cx.listener(|this, _: &NewSheet, window, cx| this.new_sheet(window, cx)))
            .on_action(cx.listener(|this, _: &NewNote, window, cx| this.new_sticker(window, cx)))
            .on_action(cx.listener(|this, _: &NextSheet, _, cx| {
                let n = this.board.categories.len();
                this.switch_sheet((this.board.active + 1) % n, cx)
            }))
            .on_action(cx.listener(|this, _: &PrevSheet, _, cx| {
                let n = this.board.categories.len();
                this.switch_sheet((this.board.active + n - 1) % n, cx)
            }))
            .on_action(cx.listener(|this, _: &FocusSearch, window, cx| {
                this.search_input.update(cx, |i, cx| i.focus(window, cx));
            }))
            .on_action(cx.listener(|this, _: &Quit, _, cx| {
                this.save_now(cx);
                cx.quit();
            }))
            .child(self.render_sidebar(window, cx))
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .flex()
                    .flex_col()
                    .child(self.render_topbar(window, cx))
                    .child(self.render_board(window, cx)),
            )
            .children(self.editing.is_some().then(|| self.render_editor(window, cx)))
            .children(self.toast.clone().map(|msg| toast(&t, msg)))
    }
}

pub fn now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// The paper after the newest sticker's, so a run of new notes alternates.
fn next_paper(sheet: &Category) -> Paper {
    let last = sheet.stickers.last().map(|s| s.paper).unwrap_or(Paper::Plain);
    let i = Paper::ALL.iter().position(|p| *p == last).unwrap_or(0);
    Paper::ALL[(i + 1) % (Paper::ALL.len() - 1)]
}

fn image_sticker(id: Id, at: Vec2, file: String, title: String) -> Sticker {
    Sticker {
        id,
        rect: Rect { x: at.x, y: at.y, w: 240., h: 220. },
        paper: Paper::Plain,
        title,
        body: String::new(),
        image: Some(file),
        text_size: TextSize::Medium,
        edited: now(),
    }
}

fn is_text_file(path: &std::path::Path) -> bool {
    matches!(
        path.extension().and_then(|e| e.to_str()).map(str::to_ascii_lowercase).as_deref(),
        Some("md" | "markdown" | "txt")
    )
}
