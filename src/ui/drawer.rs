//! The sheets drawer (the sidebar): hidden until the top-left button opens
//! it. A preview card per sheet, plus the way to Home, Trash and Settings.

use std::time::{Duration, Instant};

use gpui::{div, img, prelude::*, px, Animation, AnimationExt, Context, MouseButton, MouseDownEvent, Window};

use super::app::{BinTab, Folio, Intro, MenuKind, Screen};
use super::mini::miniature;
use super::*;
use crate::theme::{theme, Theme};
use crate::time::{self, now};

impl Folio {
    pub(super) fn render_drawer(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let t = theme(cx).clone();
        if !self.drawer {
            return div().into_any_element();
        }
        let w = f32::from(t.size.drawer);
        let reduce = self.board.settings.reduce_motion;
        let order = self.board.ordered();
        let active = (self.screen == Screen::Sheet).then_some(self.board.active);
        let cards: Vec<_> = order.into_iter().map(|ix| self.drawer_card(ix, active == Some(ix), &t, cx)).collect();
        let screen = self.screen;
        let nav = |id: &'static str, path: &'static str, label: &'static str, on: bool| {
            div()
                .id(id)
                .h(px(38.))
                .px(t.space.md)
                .flex()
                .items_center()
                .gap(t.space.md)
                .rounded(t.radius.button)
                .cursor_pointer()
                .text_size(t.text.body)
                .when(on, |d| d.bg(t.colors.brand_tint).text_color(t.colors.brand_on_tint).font_weight(gpui::FontWeight::SEMIBOLD))
                .when(!on, |d| d.text_color(t.colors.text_muted).hover(|d| d.bg(t.colors.hover)))
                .child(icon(path, t.size.icon, if on { t.colors.brand_on_tint } else { t.colors.text_muted }))
                .child(label)
        };
        let panel = div()
            .id("drawer-panel")
            .absolute()
            .top_0()
            .bottom_0()
            .w(px(w))
            .bg(t.colors.surface)
            .border_r_1()
            .border_color(t.colors.border)
            .shadow(t.shadow_float())
            .flex()
            .flex_col()
            .p(t.space.md)
            .gap(t.space.xs)
            .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(t.space.sm)
                    .px(t.space.xs)
                    .pb(t.space.md)
                    .child(img("brand/folio-64.png").size(px(28.)))
                    .child(
                        div()
                            .flex()
                            .flex_1()
                            .text_size(t.text.brand)
                            .font_weight(gpui::FontWeight::BOLD)
                            .child("folio")
                            .child(div().text_color(t.colors.brand).child(".")),
                    )
                    .child(icon_button("drawer-close", "icons/x.svg", &t).on_click(cx.listener(|this, _, _, cx| {
                        this.drawer = false;
                        cx.notify();
                    }))),
            )
            .child(nav("nav-home", "icons/layout-grid.svg", "All sheets", screen == Screen::Home).on_click(
                cx.listener(|this, _, _, cx| this.go_home(cx)),
            ))
            .child(nav("nav-bin", "icons/archive.svg", "Trash & archive", screen == Screen::Bin).on_click(
                cx.listener(|this, _, _, cx| this.open_bin(BinTab::Trash, cx)),
            ))
            .child(nav("nav-settings", "icons/settings.svg", "Settings", screen == Screen::Settings).on_click(
                cx.listener(|this, _, _, cx| this.open_settings(cx)),
            ))
            .child(
                div()
                    .flex()
                    .items_center()
                    .px(t.space.xs)
                    .pt(t.space.md)
                    .pb(t.space.xs)
                    .child(div().flex_1().child(super::home::section_label("SHEETS", &t)))
                    .child(kbd("Ctrl+Tab", &t)),
            )
            .child(
                div()
                    .id("drawer-list")
                    .flex_1()
                    .min_h_0()
                    .overflow_y_scroll()
                    .flex()
                    .flex_col()
                    .gap(t.space.sm)
                    .p(px(2.))
                    .children(cards)
                    .child(
                        div()
                            .id("drawer-new")
                            .h(px(44.))
                            .flex_none()
                            .flex()
                            .items_center()
                            .justify_center()
                            .rounded(t.radius.card)
                            .border_1()
                            .border_dashed()
                            .border_color(t.colors.border)
                            .text_color(t.colors.text_subtle)
                            .cursor_pointer()
                            .hover(|d| d.border_color(t.colors.brand).text_color(t.colors.brand))
                            .child("+ New sheet")
                            .on_click(cx.listener(|this, _, window, cx| this.new_sheet_dialog(window, cx))),
                    ),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(t.space.sm)
                    .pt(t.space.md)
                    .border_t_1()
                    .border_color(t.colors.border)
                    .child(
                        div()
                            .size(px(32.))
                            .rounded(t.radius.full)
                            .bg(t.colors.brand_tint)
                            .text_color(t.colors.brand_on_tint)
                            .font_weight(gpui::FontWeight::BOLD)
                            .flex()
                            .items_center()
                            .justify_center()
                            .child("F"),
                    )
                    .child(
                        div()
                            .flex_1()
                            .flex()
                            .flex_col()
                            .child(div().text_size(t.text.caption).child("My workspace"))
                            .child(
                                div()
                                    .text_size(t.text.micro)
                                    .text_color(t.colors.text_subtle)
                                    .child(if self.saved { "Saved just now" } else { "Saving…" }),
                            ),
                    )
                    .child(icon_button("replay-intro", "icons/play.svg", &t).on_click(cx.listener(|this, _, _, cx| {
                        this.drawer = false;
                        this.intro = Some(Intro { start: Instant::now(), full: true });
                        cx.notify();
                    }))),
            );
        let panel = if reduce {
            panel.left_0().into_any_element()
        } else {
            panel
                .with_animation("drawer-in", Animation::new(Duration::from_millis(220)).with_easing(ease_out), move |d, k| {
                    d.left(px(-w * (1. - k)))
                })
                .into_any_element()
        };
        div()
            .id("drawer")
            .absolute()
            .inset_0()
            .bg(t.colors.scrim)
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|this, _, _, cx| {
                    this.drawer = false;
                    cx.notify();
                }),
            )
            .child(panel)
            .into_any_element()
    }

    fn drawer_card(&self, ix: usize, active: bool, t: &Theme, cx: &mut Context<Self>) -> gpui::AnyElement {
        let c = &self.board.categories[ix];
        let n = c.stickers.len();
        let name = c.name.clone();
        let meta = format!("{n} note{} · {}", if n == 1 { "" } else { "s" }, time::ago(c.opened.max(c.edited), now()));
        div()
            .id(("drawer-card", ix))
            .flex_none()
            .flex()
            .items_center()
            .gap(t.space.md)
            .p(px(7.))
            .rounded(t.radius.card)
            .bg(t.colors.surface)
            .border_1()
            .border_color(if active { t.colors.brand } else { t.colors.border })
            .when(active, |d| d.shadow(vec![gpui::BoxShadow {
                color: t.colors.brand_tint,
                offset: gpui::point(px(0.), px(0.)),
                blur_radius: px(0.),
                spread_radius: px(3.),
                inset: false,
            }]))
            .cursor_pointer()
            .hover(|d| d.border_color(t.colors.text_subtle))
            .child(miniature(c, 66., 50., false, t).flex_none())
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .flex()
                    .flex_col()
                    .gap(px(2.))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(t.space.sm)
                            .child(sheet_badge(c, 20., t))
                            .child(div().flex_1().min_w_0().text_size(t.text.body).font_weight(gpui::FontWeight::SEMIBOLD).truncate().child(name)),
                    )
                    .child(div().text_size(t.text.micro).text_color(t.colors.text_subtle).child(meta)),
            )
            .on_mouse_down(
                MouseButton::Right,
                cx.listener(move |this, ev: &MouseDownEvent, _, cx| this.open_menu(ev.position, MenuKind::Card(ix), cx)),
            )
            .on_click(cx.listener(move |this, _, window, cx| this.open_sheet(ix, window, cx)))
            .into_any_element()
    }
}

pub fn ease_out(t: f32) -> f32 {
    1. - (1. - t).powi(3)
}
