//! The quick note: Ctrl+Alt+N (or the keys set in Settings) from any app
//! opens this small box. Type,
//! press Enter, and it's in your Inbox; Esc closes it.

use gpui::component::input::{InputEvent, Textarea, TextareaState};
use gpui::{div, img, prelude::*, px, Context, Entity, Window};

use super::app::Folio;
use super::*;
use crate::theme::theme;

pub struct QuickNoteWindow {
    folio: Entity<Folio>,
    input: Entity<TextareaState>,
    /// Saved or closed: the box plays its way out, then the window goes.
    leaving: bool,
    _subs: Vec<gpui::Subscription>,
}

impl QuickNoteWindow {
    pub fn new(folio: Entity<Folio>, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let input = cx.new(|cx| {
            TextareaState::new(window, cx)
                .auto_grow(3, 6)
                .submit_on_enter(true)
                .placeholder("Jot it down… the first line becomes the title")
        });
        let subs = vec![cx.subscribe_in(&input, window, |this, _, ev, window, cx| {
            // Shift+Enter is a new line; plain Enter saves.
            if let InputEvent::PressEnter { shift: false, .. } = ev {
                this.save(window, cx);
            }
        })];
        let i = input.clone();
        window.defer(cx, move |window, cx| i.update(cx, |i, cx| i.focus(window, cx)));
        QuickNoteWindow { folio, input, leaving: false, _subs: subs }
    }

    fn save(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.leaving {
            return;
        }
        let text = self.input.read(cx).value().to_string();
        self.folio.update(cx, |f, cx| {
            f.quick_note(&text, cx);
            f.desktop.quick = None;
        });
        self.leave(window, cx);
    }

    fn close(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.folio.update(cx, |f, _| f.desktop.quick = None);
        self.leave(window, cx);
    }

    /// Fades the box out, then closes the window.
    fn leave(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if std::mem::replace(&mut self.leaving, true) {
            return;
        }
        cx.notify();
        let ms = super::motion::ms(theme(cx).motion.window_out);
        cx.spawn_in(window, async move |_, cx| {
            cx.background_executor().timer(std::time::Duration::from_millis(ms)).await;
            let _ = cx.update(|window, _| window.remove_window());
        })
        .detach();
    }
}

impl Render for QuickNoteWindow {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let t = theme(cx).clone();
        div()
            .key_context("QuickNote")
            .size_full()
            .font_family(t.font)
            .text_color(t.colors.text)
            .on_action(cx.listener(|this, _: &Dismiss, window, cx| this.close(window, cx)))
            .child({
                let card = div()
                    .relative()
                    .size_full()
                    .flex()
                    .flex_col()
                    .gap(px(10.))
                    .p(px(16.))
                    .rounded(t.radius.dialog)
                    .bg(t.colors.surface)
                    .border_1()
                    .border_color(t.colors.border)
                    .child(
                        div()
                            .id("quick-grip")
                            .flex()
                            .items_center()
                            .gap(px(8.))
                            .window_control_area(gpui::WindowControlArea::Drag)
                            .when(!cfg!(target_os = "windows"), |d| {
                                d.on_mouse_down(gpui::MouseButton::Left, |_, window, _| window.start_window_move())
                            })
                            .child(img("brand/folio-64.png").size(px(22.)))
                            .child(div().font_weight(gpui::FontWeight::SEMIBOLD).child("Quick note"))
                            .child(div().text_size(t.text.caption).text_color(t.colors.text_subtle).child("→ Inbox"))
                            .child(div().flex_1())
                            .child(icon_button("quick-close", "icons/x.svg", &t).occlude().on_click(cx.listener(|this, _, window, cx| this.close(window, cx)))),
                    )
                    .child(div().flex_1().child(Textarea::new(&self.input)))
                    .child(
                        div()
                            .flex()
                            .gap(px(14.))
                            .text_size(t.text.micro)
                            .text_color(t.colors.text_subtle)
                            .child("Enter saves")
                            .child("Shift+Enter: new line")
                            .child("Esc closes"),
                    );
                // The box fades up in its window, and sinks away when done.
                let m = t.motion;
                if self.leaving {
                    super::motion::leave(card, "quick-out", m.window_out, m.sink)
                } else {
                    super::motion::enter(card, "quick-in", m.window_in, m.rise_window)
                }
            })
    }
}
