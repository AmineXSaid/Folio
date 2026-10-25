//! The quick note: Ctrl+Alt+N (or the keys set in Settings) from any app
//! opens this small box. Type, pick the sheet it goes to (the Inbox, or
//! any other: click it, or Alt+← / Alt+→), press Enter, and it's there;
//! Esc closes it. The sheet picked is the one offered next time.

use gpui::component::input::{InputEvent, Textarea, TextareaState};
use gpui::{div, img, prelude::*, px, Context, Entity, ScrollHandle, SharedString, Window};

use super::app::Folio;
use super::*;
use crate::model::Id;
use crate::theme::theme;

pub struct QuickNoteWindow {
    folio: Entity<Folio>,
    input: Entity<TextareaState>,
    /// The sheet it goes to; None is the Inbox.
    to: Option<Id>,
    sheets: ScrollHandle,
    /// Saved or closed: the box plays its way out, then the window goes.
    leaving: bool,
    _subs: Vec<gpui::Subscription>,
}

/// Where a quick note can go: the Inbox first (None), then every other
/// sheet in the drawer's order, with its emoji.
fn places(f: &Folio) -> Vec<(Option<Id>, SharedString, String)> {
    let b = &f.board;
    let mut out = vec![(None, "Inbox".into(), "📥".to_string())];
    for ix in b.ordered() {
        let c = &b.categories[ix];
        if !c.inbox {
            out.push((Some(c.id), c.name.clone().into(), c.icon.clone()));
        }
    }
    out
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
        // The sheet picked last time, if it's still there.
        let to = {
            let b = &folio.read(cx).board;
            b.settings.quick_sheet.filter(|id| b.categories.iter().any(|c| c.id == *id && !c.inbox))
        };
        QuickNoteWindow { folio, input, to, sheets: ScrollHandle::new(), leaving: false, _subs: subs }
    }

    /// The next sheet along (or back), shown in the row.
    fn step(&mut self, by: isize, cx: &mut Context<Self>) {
        let all = places(self.folio.read(cx));
        let at = all.iter().position(|p| p.0 == self.to).unwrap_or(0) as isize;
        let next = (at + by).rem_euclid(all.len() as isize) as usize;
        self.to = all[next].0;
        self.sheets.scroll_to_item(next);
        cx.notify();
    }

    fn save(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.leaving {
            return;
        }
        let text = self.input.read(cx).value().to_string();
        let to = self.to;
        self.folio.update(cx, |f, cx| {
            f.board.settings.quick_sheet = to;
            f.quick_note(&text, to, cx);
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
        let input = self.input.clone();
        // The sheets it can go to, the chosen one lit.
        let chips = places(self.folio.read(cx)).into_iter().enumerate().map(|(i, (id, name, icon))| {
            let on = id == self.to;
            let input = input.clone();
            div()
                .id(("quick-to", i))
                .flex_none()
                .h(px(26.))
                .max_w(px(160.))
                .px(t.space.sm)
                .flex()
                .items_center()
                .gap(t.space.xs)
                .rounded(t.radius.full)
                .border_1()
                .text_size(t.text.caption)
                .cursor_pointer()
                .when(on, |d| d.bg(t.colors.brand_tint).border_color(t.colors.brand).text_color(t.colors.brand_on_tint).font_weight(gpui::FontWeight::SEMIBOLD))
                .when(!on, |d| d.border_color(t.colors.border).text_color(t.colors.text_muted).hover(|d| d.bg(t.colors.hover)))
                .when(!icon.is_empty(), |d| d.child(div().flex_none().child(icon)))
                .child(div().min_w_0().truncate().child(name))
                .on_click(cx.listener(move |this, _, window, cx| {
                    this.to = id;
                    // Back to the text, to go on typing.
                    input.update(cx, |i, cx| i.focus(window, cx));
                    cx.notify();
                }))
        });
        let row = div()
            .flex()
            .items_center()
            .gap(t.space.sm)
            .child(div().flex_none().text_size(t.text.caption).text_color(t.colors.text_subtle).child("To"))
            .child(div().id("quick-sheets").flex_1().min_w_0().flex().gap(t.space.xs).overflow_x_scroll().track_scroll(&self.sheets).children(chips));
        div()
            .key_context("QuickNote")
            .size_full()
            .font_family(t.font)
            .text_color(t.colors.text)
            .on_action(cx.listener(|this, _: &Dismiss, window, cx| this.close(window, cx)))
            .on_action(cx.listener(|this, _: &QuickSheetNext, _, cx| this.step(1, cx)))
            .on_action(cx.listener(|this, _: &QuickSheetPrev, _, cx| this.step(-1, cx)))
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
                            .child(div().flex_1())
                            .child(icon_button("quick-close", "icons/x.svg", &t).occlude().on_click(cx.listener(|this, _, window, cx| this.close(window, cx)))),
                    )
                    .child(div().flex_1().child(Textarea::new(&self.input)))
                    .child(row)
                    .child(
                        div()
                            .flex()
                            .gap(px(14.))
                            .text_size(t.text.micro)
                            .text_color(t.colors.text_subtle)
                            .child("Enter saves")
                            .child("Shift+Enter: new line")
                            .child("Alt+← →: the sheet")
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
