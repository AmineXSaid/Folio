//! A note pinned to the desktop: its own small window above the others.
//! Drag its top strip to move it and its corner to size it; double-click
//! the text (or the pencil) to edit it in place; tick its tasks; open it in
//! Folio or take it back.

use gpui::component::input::{Input, InputState, Textarea, TextareaState};
use gpui::{div, point, prelude::*, px, size, Context, Entity, MouseButton, Pixels, Point, Window, WindowControlArea};

use super::app::{format_key_input, Folio};
use super::sticker::{band, note_body};
use super::*;
use crate::desktop::{MIN_H, MIN_W};
use crate::model::{Id, NoteStyle};
use crate::theme::theme;

/// The title and text being edited.
struct Editing {
    title: Entity<InputState>,
    body: Entity<TextareaState>,
}

pub struct NoteWindow {
    folio: Entity<Folio>,
    id: Id,
    editing: Option<Editing>,
    /// Sizing from the corner: the window's size less the pointer, so the
    /// corner stays under it.
    sizing: Option<Point<Pixels>>,
    _subs: Vec<gpui::Subscription>,
}

impl NoteWindow {
    pub fn new(folio: Entity<Folio>, id: Id, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let subs = vec![
            cx.observe(&folio, |_, _, cx| cx.notify()),
            // Remember where it was moved to.
            cx.observe_window_bounds(window, |this, window, cx| {
                let b = window.bounds();
                let (x, y) = (f32::from(b.origin.x), f32::from(b.origin.y));
                let (w, h) = (f32::from(b.size.width), f32::from(b.size.height));
                let id = this.id;
                this.folio.update(cx, |f, cx| {
                    if let Some(p) = f.board.find_note_mut(id).and_then(|s| s.popped.as_mut()) {
                        if (p.x, p.y, p.w, p.h) != (x, y, w, h) {
                            (p.x, p.y, p.w, p.h) = (x, y, w, h);
                            f.schedule_save(cx);
                        }
                    }
                });
            }),
            // Clicking away keeps what was written.
            cx.observe_window_activation(window, |this, window, cx| {
                if !window.is_window_active() {
                    this.finish(window, cx);
                }
            }),
        ];
        NoteWindow { folio, id, editing: None, sizing: None, _subs: subs }
    }

    fn start_edit(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.editing.is_some() {
            return;
        }
        let Some(s) = self.folio.read(cx).board.find_note(self.id).cloned() else { return };
        let title = cx.new(|cx| InputState::new(window, cx).placeholder("Title").default_value(s.title.clone()));
        let body = cx.new(|cx| TextareaState::new(window, cx).placeholder("Write…").default_value(s.body.clone()));
        let b = body.clone();
        window.defer(cx, move |window, cx| b.update(cx, |i, cx| i.focus(window, cx)));
        self.editing = Some(Editing { title, body });
        cx.notify();
    }

    /// Ends the edit, keeping it.
    fn finish(&mut self, _: &mut Window, cx: &mut Context<Self>) {
        let Some(e) = self.editing.take() else { return };
        let title = e.title.read(cx).value().trim().to_string();
        let body = e.body.read(cx).value().to_string();
        let id = self.id;
        self.folio.update(cx, |f, cx| {
            let same = f.board.find_note(id).is_some_and(|s| s.title == title && s.body == body);
            if !same {
                f.change_note(id, cx, |s| {
                    s.title = title;
                    s.body = body;
                });
            }
        });
        cx.notify();
    }
}

/// One of the strip's small buttons. It blocks the drag area under it.
fn strip_button(id: &'static str, path: &'static str, t: &crate::theme::Theme) -> gpui::Stateful<gpui::Div> {
    div()
        .id(id)
        .occlude()
        .size(px(22.))
        .rounded(t.radius.small)
        .flex()
        .items_center()
        .justify_center()
        .cursor_pointer()
        .hover(|d| d.bg(gpui::hsla(0., 0., 0., 0.08)))
        .child(icon(path, t.size.icon_sm, t.colors.text_muted))
        .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
}

impl Render for NoteWindow {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let t = theme(cx).clone();
        let id = self.id;
        let Some(s) = self.folio.read(cx).board.find_note(id).cloned() else {
            return div().into_any_element();
        };
        let folio = self.folio.clone();
        let toggle = move |n: usize, _: &mut Window, cx: &mut gpui::App| {
            folio.update(cx, |f, cx| f.toggle_task(id, n, cx));
        };
        let media = crate::store::media_dir(&self.folio.read(cx).dir);
        let editing = self.editing.as_ref().map(|e| (e.title.clone(), e.body.clone()));
        let (f_open, f_dock, f_see) = (self.folio.clone(), self.folio.clone(), self.folio.clone());
        let see_through = s.popped.is_some_and(|p| p.see_through) && editing.is_none();
        let paper = t.paper(s.paper);

        // The strip along the top: drag it to move the note. Windows moves
        // the window itself (the strip says it's a drag area); elsewhere the
        // strip asks for the move.
        let strip = div()
            .id("grip")
            .h(px(28.))
            .flex_none()
            .flex()
            .items_center()
            .gap(px(2.))
            .px(px(6.))
            .cursor_grab()
            .window_control_area(WindowControlArea::Drag)
            .when(!cfg!(target_os = "windows"), |d| d.on_mouse_down(MouseButton::Left, |_, window, _| window.start_window_move()))
            .child(icon("icons/pin.svg", px(12.), t.colors.text_subtle))
            .child(div().flex_1())
            .child(if editing.is_some() {
                strip_button("done", "icons/check.svg", &t)
                    .on_click(cx.listener(|this, _, window, cx| this.finish(window, cx)))
                    .into_any_element()
            } else {
                strip_button("edit", "icons/pencil.svg", &t)
                    .on_click(cx.listener(|this, _, window, cx| this.start_edit(window, cx)))
                    .into_any_element()
            })
            .child(strip_button("open", "icons/arrow-up-right.svg", &t).on_click(move |_, _, cx| {
                let main = f_open.read(cx).desktop.main;
                if let Some(main) = main {
                    let fo = f_open.clone();
                    let _ = main.update(cx, move |_, w, cx| {
                        w.activate_window();
                        fo.update(cx, |f, cx| f.reveal_note(id, w, cx));
                    });
                }
            }))
            .child(
                strip_button("see", if see_through { "icons/eye-off.svg" } else { "icons/eye.svg" }, &t).on_click(move |_, _, cx| {
                    f_see.update(cx, |f, cx| {
                        if let Some(p) = f.board.find_note_mut(id).and_then(|s| s.popped.as_mut()) {
                            p.see_through = !p.see_through;
                        }
                        f.schedule_save(cx);
                        cx.notify();
                    })
                }),
            )
            .child(strip_button("dock", "icons/x.svg", &t).on_click(move |_, _, cx| f_dock.update(cx, |f, cx| f.dock(id, cx))));

        let content = match editing {
            Some((title, body)) => on_format(div().id("note").key_context("NoteEdit"), {
                let (title, body) = (title.clone(), body.clone());
                move |f, window, cx| format_key_input(&title, &body, f, window, cx)
            })
                .flex_1()
                .min_h_0()
                .px(px(10.))
                .pb(px(10.))
                .flex()
                .flex_col()
                .gap(px(6.))
                .on_action(cx.listener(|this, _: &SaveNote, window, cx| this.finish(window, cx)))
                .on_action(cx.listener(|this, _: &Dismiss, window, cx| this.finish(window, cx)))
                .child(Input::new(&title).appearance(false))
                .child(div().flex_1().min_h_0().child(Textarea::new(&body).appearance(false).h_full()))
                .child(div().px(px(6.)).text_size(t.text.micro).text_color(t.colors.text_subtle).child(format!("{} or Esc: done", shortcut("Enter")))),
            None => div()
                .id("note")
                .flex_1()
                .min_h_0()
                .overflow_y_scroll()
                .px(px(16.))
                .pb(px(14.))
                .flex()
                .flex_col()
                .gap(px(8.))
                .cursor_text()
                .on_click(cx.listener(|this, e: &gpui::ClickEvent, window, cx| {
                    if e.click_count() >= 2 {
                        this.start_edit(window, cx);
                    }
                }))
                .map(|d| {
                    let parts = note_body(&s, 1.0, &t, &media, None, toggle);
                    // A cheat card's title keeps its band, inset in the window.
                    let title = match s.style {
                        NoteStyle::Cheat => Some(band(parts.title, s.paper, 1.0, &t).rounded(t.radius.button).into_any_element()),
                        _ => parts.title,
                    };
                    d.children(title).children(parts.body)
                }),
        };

        // The corner: drag it to size the note.
        let corner = div()
            .id("size")
            .absolute()
            .right_0()
            .bottom_0()
            .size(px(16.))
            .cursor(gpui::CursorStyle::ResizeUpLeftDownRight)
            .flex()
            .items_end()
            .justify_end()
            .p(px(3.))
            .child(div().size(px(6.)).border_r_2().border_b_2().border_color(t.colors.text_subtle.opacity(0.6)))
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|this, e: &gpui::MouseDownEvent, window, cx| {
                    cx.stop_propagation();
                    let s = window.bounds().size;
                    this.sizing = Some(point(s.width - e.position.x, s.height - e.position.y));
                }),
            );

        div()
            .size_full()
            .on_mouse_move(cx.listener(|this, e: &gpui::MouseMoveEvent, window, _| {
                let Some(off) = this.sizing else { return };
                if e.pressed_button != Some(MouseButton::Left) {
                    this.sizing = None;
                    return;
                }
                let w = (e.position.x + off.x).max(px(MIN_W));
                let h = (e.position.y + off.y).max(px(MIN_H));
                window.resize(size(w, h));
            }))
            .on_mouse_up(MouseButton::Left, cx.listener(|this, _, _, _| this.sizing = None))
            .child(
                div()
                    .relative()
                    .size_full()
                    .flex()
                    .flex_col()
                    .id("paper")
                    .bg(if see_through { paper.opacity(0.5) } else { paper })
                    // Solid again under the pointer, to read and tick.
                    .hover(move |d| d.bg(paper))
                    .border_1()
                    .border_color(t.colors.border)
                    .overflow_hidden()
                    .font_family(t.note_font(s.font))
                    .text_color(t.colors.text)
                    .child(strip)
                    .child(content)
                    .child(corner),
            )
            .into_any_element()
    }
}
