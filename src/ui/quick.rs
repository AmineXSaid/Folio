//! The quick note: Ctrl+Alt+N from any app opens this small box. Type,
//! press Enter, and it's in your Inbox; Esc closes it.

use gpui::component::input::{InputEvent, Textarea, TextareaState};
use gpui::{div, img, prelude::*, px, Context, Entity, Window};

use super::app::Folio;
use super::*;
use crate::theme::theme;

pub struct QuickNoteWindow {
    folio: Entity<Folio>,
    input: Entity<TextareaState>,
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
            if let InputEvent::PressEnter { .. } = ev {
                this.save(window, cx);
            }
        })];
        let i = input.clone();
        window.defer(cx, move |window, cx| i.update(cx, |i, cx| i.focus(window, cx)));
        QuickNoteWindow { folio, input, _subs: subs }
    }

    fn save(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let text = self.input.read(cx).value().to_string();
        self.folio.update(cx, |f, cx| {
            f.quick_note(&text, cx);
            f.desktop.quick = None;
        });
        window.remove_window();
    }

    fn close(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.folio.update(cx, |f, _| f.desktop.quick = None);
        window.remove_window();
    }
}

impl Render for QuickNoteWindow {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let t = theme(cx).clone();
        div()
            .key_context("QuickNote")
            .size_full()
            .p(px(8.))
            .font_family(t.font)
            .text_color(t.colors.text)
            .on_action(cx.listener(|this, _: &Dismiss, window, cx| this.close(window, cx)))
            .child(
                div()
                    .size_full()
                    .flex()
                    .flex_col()
                    .gap(px(10.))
                    .p(px(16.))
                    .rounded(px(16.))
                    .bg(t.colors.surface)
                    .border_1()
                    .border_color(t.colors.border)
                    .shadow(t.shadow_float())
                    .child(
                        div()
                            .id("quick-grip")
                            .flex()
                            .items_center()
                            .gap(px(8.))
                            .on_mouse_down(gpui::MouseButton::Left, |_, window, _| window.start_window_move())
                            .child(img("brand/folio-64.png").size(px(22.)))
                            .child(div().font_weight(gpui::FontWeight::SEMIBOLD).child("Quick note"))
                            .child(div().text_size(t.text.caption).text_color(t.colors.text_subtle).child("→ Inbox"))
                            .child(div().flex_1())
                            .child(icon_button("quick-close", "icons/x.svg", &t).on_click(cx.listener(|this, _, window, cx| this.close(window, cx)))),
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
                    ),
            )
    }
}
