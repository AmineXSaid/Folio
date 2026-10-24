//! The welcome: the first time Folio opens it asks your name, then has you
//! pick an avatar (one is chosen for you to start). Both show in the drawer
//! and the home screen greets you by name. Reopened from the drawer or
//! Ctrl+K to change either; then Esc closes it.

use gpui::component::input::Input;
use gpui::{canvas, div, img, prelude::*, px, Context, MouseButton, SharedString, Window};

use super::app::Folio;
use super::*;
use crate::avatars;
use crate::model::{Camera, NoteFont, Paper, Pattern, Pin, PinShape, PinSize, PinTone};
use crate::theme::theme;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Step {
    Name,
    Avatar,
}

#[derive(Clone)]
pub struct Welcome {
    pub step: Step,
    /// The avatar picked (or picked for you).
    pub pick: &'static str,
    /// You've been here before: it can be closed.
    pub returning: bool,
}

impl Folio {
    /// Opens the welcome at your name, or at the avatars.
    pub fn open_welcome(&mut self, at_avatar: bool, window: &mut Window, cx: &mut Context<Self>) {
        self.finish_edit(cx);
        self.drawer = false;
        self.menu = None;
        self.palette = false;
        let s = &self.board.settings;
        let pick = avatars::all().find(|a| *a == s.avatar).unwrap_or_else(|| avatars::random(crate::time::now()));
        let name = s.user_name.clone();
        self.welcome = Some(Welcome { step: if at_avatar { Step::Avatar } else { Step::Name }, pick, returning: !name.is_empty() });
        self.welcome_input.update(cx, |i, cx| i.set_value(name, window, cx));
        let input = self.welcome_input.clone();
        window.defer(cx, move |window, cx| input.update(cx, |i, cx| i.focus(window, cx)));
        cx.notify();
    }

    pub fn welcome_next(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(w) = self.welcome.as_mut() else { return };
        match w.step {
            Step::Name => {
                if self.welcome_input.read(cx).value().trim().is_empty() {
                    return;
                }
                w.step = Step::Avatar;
                window.focus(&self.root_focus, cx);
            }
            Step::Avatar => self.welcome_finish(window, cx),
        }
        cx.notify();
    }

    fn welcome_finish(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(w) = self.welcome.take() else { return };
        let name: String = self.welcome_input.read(cx).value().trim().chars().take(40).collect();
        let first = !w.returning;
        self.set_setting(cx, |s| {
            s.user_name = name.clone();
            s.avatar = w.pick.to_string();
        });
        window.focus(&self.root_focus, cx);
        if first {
            self.notify(format!("Welcome to Folio, {name}"), cx);
        }
    }

    /// Esc: closes it, unless you haven't told it your name yet.
    pub fn welcome_close(&mut self, cx: &mut Context<Self>) {
        if self.welcome.as_ref().is_some_and(|w| w.returning) {
            self.welcome = None;
            cx.notify();
        }
    }

    /// The welcome after it's done: it sinks away as the window shows.
    pub(super) fn render_welcome_leaving(&mut self, window: &mut Window, cx: &mut Context<Self>) -> Option<gpui::AnyElement> {
        let t = theme(cx).clone();
        let (w, n) = self.exits.welcome.leaving(t.motion.dialog_out)?;
        let real = self.welcome.replace(w);
        let el = self.render_welcome(window, cx).into_any_element();
        self.welcome = real;
        Some(motion::leave(div().absolute().inset_0().child(el), ("welcome-out", n), t.motion.dialog_out, t.motion.sink))
    }

    /// A note pinned to the sheet asks your name, then has you pick a face;
    /// the same note, the same size, for both steps. The author's credit
    /// sits in the bottom-left corner.
    pub(super) fn render_welcome(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let t = theme(cx).clone();
        let Some(w) = self.welcome.as_ref() else { return div().into_any_element() };
        let (step, pick, returning) = (w.step, w.pick, w.returning);
        let name = self.welcome_input.read(cx).value().trim().to_string();

        let heading = |text: String| {
            div().text_size(t.text.heading).font_weight(gpui::FontWeight::SEMIBOLD).text_color(t.colors.text).child(text)
        };
        let lede = |text: &'static str| div().text_size(t.text.title).text_color(t.colors.text_muted).child(text);
        let intro = |title: String, line: &'static str| div().flex().flex_col().gap(t.space.sm).child(heading(title)).child(lede(line));
        let footer = || div().flex().items_center().justify_between().gap(t.space.md);

        let body = match step {
            Step::Name => div()
                .flex()
                .flex_col()
                .gap(t.space.xl)
                .child(intro(
                    if returning { "What should Folio call you?" } else { "Hello! What should we call you?" }.into(),
                    "Folio greets you by name. It stays on this computer.",
                ))
                .child(
                    // Your name, written on the note's line in its hand.
                    div()
                        .key_context("Welcome")
                        .font_family(t.note_font(NoteFont::Hand))
                        .border_b_2()
                        .border_color(if name.is_empty() { t.colors.rule } else { t.colors.brand })
                        .child(
                            // Back by the field's own inset, so the name starts
                            // where the heading does.
                            div().ml(px(-10.)).child(rem_scaled(t.text.heading / 0.875, Input::new(&self.welcome_input).appearance(false))),
                        ),
                )
                .child(
                    footer()
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .gap(t.space.xs)
                                .text_size(t.text.caption)
                                .text_color(t.colors.text_subtle)
                                .child(icon("icons/corner-down-left.svg", t.size.icon_sm, t.colors.text_subtle))
                                .child("Enter to continue"),
                        )
                        .child(
                            primary_button("w-next", "Continue", &t)
                                .when(name.is_empty(), |d| d.opacity(0.5))
                                .on_click(cx.listener(|this, _, window, cx| this.welcome_next(window, cx))),
                        ),
                )
                .into_any_element(),
            Step::Avatar => {
                let d = t.size.avatar;
                let row = |list: [&'static str; 6], cx: &mut Context<Self>| {
                    div().flex().justify_between().children(list.into_iter().map(|a| {
                        let on = a == pick;
                        div()
                            .id(SharedString::from(format!("avatar-{a}")))
                            .size(d)
                            .flex()
                            .items_center()
                            .justify_center()
                            .rounded(t.radius.full)
                            .border_2()
                            .border_color(if on { t.colors.brand } else { gpui::transparent_black() })
                            .cursor_pointer()
                            .hover(|h| h.border_color(if on { t.colors.brand } else { t.colors.text_subtle }))
                            .child(img(SharedString::from(avatars::path(a, f32::from(d) as u32 - 10))).size(d - px(10.)).rounded(t.radius.full))
                            .on_click(cx.listener(move |this, _, _, cx| {
                                if let Some(w) = this.welcome.as_mut() {
                                    w.pick = a;
                                }
                                cx.notify();
                            }))
                    }))
                };
                div()
                    .flex()
                    .flex_col()
                    .gap(t.space.xl)
                    .child(intro(format!("Nice to meet you, {name}"), "Pick a face. It sits beside your name in the drawer."))
                    .child(div().flex().flex_col().gap(t.space.md).child(row(avatars::WOMEN, cx)).child(row(avatars::MEN, cx)))
                    .child(
                        footer()
                            // How you'll look: as the drawer shows you.
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap(t.space.sm)
                                    .min_w_0()
                                    .child(img(SharedString::from(avatars::path(pick, 32))).size(t.size.control).rounded(t.radius.full))
                                    .child(div().truncate().text_size(t.text.body).font_weight(gpui::FontWeight::SEMIBOLD).text_color(t.colors.text).child(name)),
                            )
                            .child(
                                div()
                                    .flex()
                                    .flex_none()
                                    .gap(t.space.sm)
                                    .child(secondary_button("w-back", "Back", &t).on_click(cx.listener(|this, _, window, cx| {
                                        if let Some(w) = this.welcome.as_mut() {
                                            w.step = Step::Name;
                                        }
                                        let input = this.welcome_input.clone();
                                        window.defer(cx, move |window, cx| input.update(cx, |i, cx| i.focus(window, cx)));
                                        cx.notify();
                                    })))
                                    .child(
                                        primary_button("w-done", if returning { "Save" } else { "Start writing" }, &t)
                                            .on_click(cx.listener(|this, _, window, cx| this.welcome_next(window, cx))),
                                    ),
                            ),
                    )
                    .into_any_element()
            }
        };

        // Where you are: two short bars, the step you're on long and pink.
        let steps = div().flex().gap(t.space.xs).children([Step::Name, Step::Avatar].map(|s| {
            let on = s == step;
            div().h(t.space.xs).w(if on { t.space.xl } else { t.space.md }).rounded(t.radius.full).bg(if on {
                t.colors.brand
            } else {
                t.colors.rule
            })
        }));
        let pin = Pin { shape: PinShape::Push, size: PinSize::Large, tone: PinTone::Pink };
        let note = div()
            .relative()
            .w(t.size.welcome)
            .px(t.space.xxl)
            .pt(t.space.xxl + t.space.sm)
            .pb(t.space.xxl)
            .flex()
            .flex_col()
            .gap(t.space.xl)
            .rounded(t.radius.card)
            .bg(t.paper(Paper::Butter))
            .border_1()
            .border_color(t.colors.tape_edge)
            .shadow(t.shadow_card(1.))
            .child(steps)
            .child(body)
            .child(super::pins::pin_view(pin, f32::from(t.size.welcome), 1., None, &t));

        let (grid, dot, line) = (t.size.dot_grid, t.colors.canvas_dot, t.colors.canvas_line);
        let step_key = format!("welcome-{step:?}");
        div()
            .id("welcome")
            .absolute()
            .inset_0()
            .occlude()
            .bg(t.colors.canvas)
            .flex()
            .flex_col()
            .items_center()
            .justify_center()
            .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
            // The sheet the note is pinned to.
            .child(
                canvas(|_, _, _| {}, move |b, _, window, _| super::board::paint_pattern(b, Camera::default(), grid, Pattern::Dots, dot, line, window))
                    .absolute()
                    .inset_0(),
            )
            // The note hangs from the top of its room: a taller step grows
            // downwards and the pin stays where it was.
            .child(div().h(t.size.welcome_h).flex().flex_col().items_center().child(motion::enter(
                note,
                gpui::ElementId::Name(step_key.into()),
                t.motion.dialog_in,
                t.motion.rise_dialog,
            )))
            .when(returning, |d| {
                d.child(
                    div()
                        .absolute()
                        .top(t.space.lg)
                        .right(t.space.lg)
                        .child(icon_button("w-close", "icons/x.svg", &t).on_click(cx.listener(|this, _, _, cx| this.welcome_close(cx)))),
                )
            })
            .child(div().absolute().left(t.space.xl).bottom(t.space.xl).child(author_badge(&t, t.colors.toast, t.colors.toast_fg)))
            .into_any_element()
    }
}
