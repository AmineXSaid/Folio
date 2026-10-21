//! A note pinned to the desktop: its own small window above the others.
//! Drag its top edge to move it; tick its tasks; open it in Folio or take
//! it back.

use gpui::{div, prelude::*, px, Context, Entity, MouseButton, Window};

use super::app::Folio;
use super::sticker::note_body;
use super::*;
use crate::model::{Id, Popped};
use crate::theme::theme;

pub struct NoteWindow {
    folio: Entity<Folio>,
    id: Id,
    _subs: Vec<gpui::Subscription>,
}

impl NoteWindow {
    pub fn new(folio: Entity<Folio>, id: Id, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let subs = vec![
            cx.observe(&folio, |_, _, cx| cx.notify()),
            // Remember where it was moved to.
            cx.observe_window_bounds(window, |this, window, cx| {
                let b = window.bounds();
                let p = Popped {
                    x: f32::from(b.origin.x),
                    y: f32::from(b.origin.y),
                    w: f32::from(b.size.width),
                    h: f32::from(b.size.height),
                };
                let id = this.id;
                this.folio.update(cx, |f, cx| {
                    if let Some(s) = f.board.find_note_mut(id) {
                        if s.popped.is_some() && s.popped != Some(p) {
                            s.popped = Some(p);
                            f.schedule_save(cx);
                        }
                    }
                });
            }),
        ];
        NoteWindow { folio, id, _subs: subs }
    }
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
        let body = note_body(&s, 1.0, &t, None, toggle);
        let (f_open, f_dock) = (self.folio.clone(), self.folio.clone());
        div()
            .size_full()
            .p(px(6.))
            .child(
                div()
                    .size_full()
                    .flex()
                    .flex_col()
                    .rounded(px(12.))
                    .bg(t.paper(s.paper))
                    .border_1()
                    .border_color(t.colors.border)
                    .shadow(t.shadow_card(0.6))
                    .overflow_hidden()
                    .font_family(t.note_font(s.font))
                    .text_color(t.colors.text)
                    .child(
                        div()
                            .id("grip")
                            .h(px(28.))
                            .flex_none()
                            .flex()
                            .items_center()
                            .gap(px(2.))
                            .px(px(8.))
                            .cursor_grab()
                            .on_mouse_down(MouseButton::Left, |_, window, _| window.start_window_move())
                            .child(icon("icons/pin.svg", px(12.), t.colors.text_subtle))
                            .child(div().flex_1())
                            .child(
                                div()
                                    .id("open")
                                    .size(px(22.))
                                    .rounded(px(6.))
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .cursor_pointer()
                                    .hover(|d| d.bg(gpui::hsla(0., 0., 0., 0.08)))
                                    .child(icon("icons/arrow-up-right.svg", px(13.), t.colors.text_muted))
                                    .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                                    .on_click(move |_, _, cx| {
                                        let main = f_open.read(cx).desktop.main;
                                        if let Some(main) = main {
                                            let fo = f_open.clone();
                                            let _ = main.update(cx, move |_, w, cx| {
                                                w.activate_window();
                                                fo.update(cx, |f, cx| f.reveal_note(id, w, cx));
                                            });
                                        }
                                    }),
                            )
                            .child(
                                div()
                                    .id("dock")
                                    .size(px(22.))
                                    .rounded(px(6.))
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .cursor_pointer()
                                    .hover(|d| d.bg(gpui::hsla(0., 0., 0., 0.08)))
                                    .child(icon("icons/x.svg", px(13.), t.colors.text_muted))
                                    .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                                    .on_click(move |_, _, cx| f_dock.update(cx, |f, cx| f.dock(id, cx))),
                            ),
                    )
                    .child(
                        div()
                            .id("note")
                            .flex_1()
                            .min_h_0()
                            .overflow_y_scroll()
                            .px(px(16.))
                            .pb(px(14.))
                            .flex()
                            .flex_col()
                            .gap(px(8.))
                            .children(body),
                    ),
            )
            .into_any_element()
    }
}
