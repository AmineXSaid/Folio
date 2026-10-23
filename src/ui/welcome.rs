//! The welcome: the first time Folio opens it asks your name, then has you
//! pick an avatar (one is chosen for you to start). Both show in the drawer
//! and the home screen greets you by name. Reopened from the drawer or
//! Ctrl+K to change either; then Esc closes it.

use gpui::component::input::Input;
use gpui::component::Sizable as _;
use gpui::{div, img, prelude::*, px, Context, MouseButton, SharedString, Window};

use super::app::Folio;
use super::*;
use crate::avatars;
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

    pub(super) fn render_welcome(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let t = theme(cx).clone();
        let Some(w) = self.welcome.as_ref() else { return div().into_any_element() };
        let (step, pick, returning) = (w.step, w.pick, w.returning);
        let name = self.welcome_input.read(cx).value().trim().to_string();

        let body = match step {
            Step::Name => div()
                .flex()
                .flex_col()
                .items_center()
                .gap(t.space.lg)
                .child(
                    div()
                        .flex()
                        .text_size(px(44.))
                        .font_weight(gpui::FontWeight::BOLD)
                        .child("folio")
                        .child(div().text_color(t.colors.brand).child(".")),
                )
                .child(
                    div()
                        .text_size(px(22.))
                        .font_weight(gpui::FontWeight::SEMIBOLD)
                        .child(if returning { "What should Folio call you?" } else { "Welcome. What should we call you?" }),
                )
                .child(
                    div()
                        .key_context("Welcome")
                        .w(px(360.))
                        .h(px(48.))
                        .px(t.space.md)
                        .flex()
                        .items_center()
                        .rounded(t.radius.card)
                        .bg(t.colors.surface)
                        .border_1()
                        .border_color(t.colors.brand)
                        .shadow(vec![gpui::BoxShadow {
                            color: t.colors.brand.opacity(0.16),
                            offset: gpui::point(px(0.), px(0.)),
                            blur_radius: px(0.),
                            spread_radius: px(4.),
                            inset: false,
                        }])
                        .text_size(px(17.))
                        .child(Input::new(&self.welcome_input).appearance(false).large()),
                )
                .child(
                    primary_button("w-next", "Continue", &t)
                        .when(name.is_empty(), |d| d.opacity(0.5))
                        .on_click(cx.listener(|this, _, window, cx| this.welcome_next(window, cx))),
                )
                .into_any_element(),
            Step::Avatar => {
                let row = |list: [&'static str; 6], cx: &mut Context<Self>| {
                    div().flex().gap(t.space.md).children(list.into_iter().map(|a| {
                        let on = a == pick;
                        div()
                            .id(SharedString::from(format!("avatar-{a}")))
                            .size(px(72.))
                            .rounded(t.radius.full)
                            .p(px(3.))
                            .border_2()
                            .border_color(if on { t.colors.brand } else { gpui::transparent_black() })
                            .cursor_pointer()
                            .hover(|d| d.border_color(t.colors.text_subtle))
                            .child(img(SharedString::from(avatars::path(a, 62))).size(px(62.)).rounded(t.radius.full))
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
                    .items_center()
                    .gap(t.space.lg)
                    .child(img(SharedString::from(avatars::path(pick, 96))).size(px(96.)).rounded(t.radius.full))
                    .child(div().text_size(px(22.)).font_weight(gpui::FontWeight::SEMIBOLD).child(format!("Pick your look, {name}")))
                    .child(div().text_color(t.colors.text_muted).child("It shows in the drawer. You can change it any time."))
                    .child(row(avatars::WOMEN, cx))
                    .child(row(avatars::MEN, cx))
                    .child(
                        div()
                            .flex()
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
                    )
                    .into_any_element()
            }
        };

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
            .child(
                // A soft pink glow behind the card.
                div().absolute().size(px(420.)).rounded(t.radius.full).shadow(vec![gpui::BoxShadow {
                    color: t.colors.brand.opacity(if t.is_dark { 0.22 } else { 0.14 }),
                    offset: gpui::point(px(0.), px(0.)),
                    blur_radius: px(180.),
                    spread_radius: px(40.),
                    inset: false,
                }]),
            )
            .child(motion::enter(
                div()
                    .px(px(56.))
                    .py(px(44.))
                    .rounded(px(24.))
                    .bg(t.colors.surface)
                    .border_1()
                    .border_color(t.colors.border)
                    .shadow(t.shadow_float())
                    .child(body),
                gpui::ElementId::Name(step_key.into()),
                t.motion.dialog_in,
                t.motion.rise_dialog,
            ))
            .when(returning, |d| {
                d.child(
                    div()
                        .absolute()
                        .top(t.space.lg)
                        .right(t.space.lg)
                        .child(icon_button("w-close", "icons/x.svg", &t).on_click(cx.listener(|this, _, _, cx| this.welcome_close(cx)))),
                )
            })
            .child(
                div()
                    .absolute()
                    .bottom(t.space.xl)
                    .text_size(t.text.caption)
                    .text_color(t.colors.text_subtle)
                    .child("Created and designed by M. Amine Said"),
            )
            .into_any_element()
    }
}
