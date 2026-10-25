//! The quick note: Ctrl+Alt+N (or the keys set in Settings) from any app
//! opens this small card. Write; the button at its foot says which sheet
//! it goes to (the Inbox, or any other: click it for the list, or Alt+← /
//! Alt+→); Enter saves it there and Esc closes. The sheet picked is the
//! one offered next time.

use gpui::component::input::{InputEvent, Textarea, TextareaState};
use gpui::{div, img, prelude::*, px, Context, Entity, SharedString, Window};

use super::app::Folio;
use super::*;
use crate::model::Id;
use crate::theme::theme;

pub struct QuickNoteWindow {
    folio: Entity<Folio>,
    input: Entity<TextareaState>,
    /// The sheet it goes to; None is the Inbox.
    to: Option<Id>,
    /// The list of sheets is open.
    picking: bool,
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
                .auto_grow(2, 4)
                .submit_on_enter(true)
                .placeholder("What's on your mind? The first line becomes the title")
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
        QuickNoteWindow { folio, input, to, picking: false, leaving: false, _subs: subs }
    }

    /// The next sheet along (or back).
    fn step(&mut self, by: isize, cx: &mut Context<Self>) {
        let all = places(self.folio.read(cx));
        let at = all.iter().position(|p| p.0 == self.to).unwrap_or(0) as isize;
        self.to = all[(at + by).rem_euclid(all.len() as isize) as usize].0;
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

    /// Esc: the list of sheets first, then the card.
    fn dismiss(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if std::mem::take(&mut self.picking) {
            cx.notify();
            return;
        }
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
        let all = places(self.folio.read(cx));
        let (_, name, emoji) = all.iter().find(|p| p.0 == self.to).cloned().unwrap_or_else(|| all[0].clone());

        // Where it goes: one button, the sheet's emoji and name.
        let to_button = div()
            .id("quick-to")
            .h(px(32.))
            .pl(t.space.sm)
            .pr(t.space.sm)
            .flex()
            .items_center()
            .gap(t.space.sm)
            .max_w(px(260.))
            .rounded(t.radius.full)
            .border_1()
            .border_color(if self.picking { t.colors.brand } else { t.colors.border })
            .bg(t.colors.surface_sunk)
            .hover(|d| d.border_color(t.colors.text_subtle))
            .cursor_pointer()
            .tooltip(|window, cx| tip("Where it goes   Alt+← →", window, cx))
            .child(div().flex_none().child(emoji))
            .child(div().min_w_0().truncate().text_size(t.text.body).font_weight(gpui::FontWeight::MEDIUM).child(name))
            .child(icon("icons/chevron-down.svg", t.size.icon_sm, t.colors.text_muted))
            .on_click(cx.listener(|this, _, _, cx| {
                this.picking = !this.picking;
                cx.notify();
            }));

        // The list of sheets, over the writing, from the button up.
        let input = self.input.clone();
        let list = self.picking.then(|| {
            div()
                .id("quick-sheets")
                .occlude()
                .absolute()
                .left(t.space.lg)
                .bottom(px(58.))
                .w(px(260.))
                .max_h(px(144.))
                .overflow_y_scroll()
                .p(t.space.xs)
                .flex()
                .flex_col()
                .rounded(t.radius.card)
                .bg(t.colors.surface)
                .border_1()
                .border_color(t.colors.border)
                .shadow(t.shadow_float())
                .children(all.into_iter().enumerate().map(|(i, (id, name, emoji))| {
                    let on = id == self.to;
                    let input = input.clone();
                    div()
                        .id(("quick-sheet", i))
                        .flex_none()
                        .h(px(30.))
                        .px(t.space.sm)
                        .flex()
                        .items_center()
                        .gap(t.space.sm)
                        .rounded(t.radius.button)
                        .cursor_pointer()
                        .text_size(t.text.body)
                        .when(on, |d| d.bg(t.colors.brand_tint).text_color(t.colors.brand_on_tint).font_weight(gpui::FontWeight::MEDIUM))
                        .when(!on, |d| d.hover(|d| d.bg(t.colors.hover)))
                        .child(div().flex_none().w(px(18.)).child(emoji))
                        .child(div().flex_1().min_w_0().truncate().child(name))
                        .when(on, |d| d.child(icon("icons/check.svg", t.size.icon_sm, t.colors.brand)))
                        .on_click(cx.listener(move |this, _, window, cx| {
                            this.to = id;
                            this.picking = false;
                            // Back to the writing.
                            input.update(cx, |i, cx| i.focus(window, cx));
                            cx.notify();
                        }))
                }))
        });

        let save = div()
            .id("quick-save")
            .h(px(32.))
            .px(t.space.md)
            .flex()
            .items_center()
            .gap(t.space.sm)
            .rounded(t.radius.full)
            .bg(t.colors.primary)
            .hover(|d| d.bg(t.colors.primary_hover))
            .text_color(t.colors.primary_fg)
            .text_size(t.text.body)
            .font_weight(gpui::FontWeight::SEMIBOLD)
            .cursor_pointer()
            .shadow(t.shadow_primary())
            .child("Save")
            .child(icon("icons/corner-down-left.svg", t.size.icon_sm, t.colors.primary_fg))
            .on_click(cx.listener(|this, _, window, cx| this.save(window, cx)));

        let card = div()
            .relative()
            .size_full()
            .flex()
            .flex_col()
            .gap(t.space.md)
            .pt(t.space.lg)
            .px(t.space.lg)
            .pb(t.space.lg)
            .rounded(t.radius.dialog)
            .bg(t.colors.surface)
            .border_1()
            .border_color(t.colors.border)
            .shadow(t.shadow_float())
            // Its top line moves the window.
            .child(
                div()
                    .id("quick-grip")
                    .flex()
                    .items_center()
                    .gap(t.space.sm)
                    .window_control_area(gpui::WindowControlArea::Drag)
                    .when(!cfg!(target_os = "windows"), |d| d.on_mouse_down(gpui::MouseButton::Left, |_, window, _| window.start_window_move()))
                    .child(img("brand/folio-64.png").size(px(18.)))
                    .child(div().text_size(t.text.caption).font_weight(gpui::FontWeight::SEMIBOLD).text_color(t.colors.text_muted).child("Quick note"))
                    .child(div().flex_1())
                    .child(div().text_size(t.text.micro).text_color(t.colors.text_subtle).child("Esc")),
            )
            // The writing: large, no box, the caret where the words start.
            .child(
                div()
                    .flex_1()
                    .min_h_0()
                    .overflow_hidden()
                    .ml(px(-10.))
                    .mt(px(-8.))
                    .child(rem_scaled(t.text.brand * 0.9 / 0.875, Textarea::new(&self.input).appearance(false))),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(t.space.md)
                    .child(to_button)
                    .child(div().flex_1())
                    .child(div().text_size(t.text.micro).text_color(t.colors.text_subtle).child("Shift+↵ new line"))
                    .child(save),
            )
            .children(list);

        div()
            .key_context("QuickNote")
            .size_full()
            // Room round the card for its corners and its shadow.
            .p(t.space.md)
            .font_family(t.font)
            .text_color(t.colors.text)
            .on_action(cx.listener(|this, _: &Dismiss, window, cx| this.dismiss(window, cx)))
            .on_action(cx.listener(|this, _: &QuickSheetNext, _, cx| this.step(1, cx)))
            .on_action(cx.listener(|this, _: &QuickSheetPrev, _, cx| this.step(-1, cx)))
            .child({
                // The card fades up in its window, and sinks away when done.
                let m = t.motion;
                if self.leaving {
                    super::motion::leave(card, "quick-out", m.window_out, m.sink)
                } else {
                    super::motion::enter(card, "quick-in", m.window_in, m.rise_window)
                }
            })
    }
}
