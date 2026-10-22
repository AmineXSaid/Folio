//! The sheets drawer (the sidebar): hidden until the top-left button opens
//! it. A preview card per sheet, plus the way to Home, Trash and Settings.

use std::time::Duration;

use gpui::{div, img, prelude::*, px, Animation, AnimationExt, Context, Hsla, MouseButton, MouseDownEvent, SharedString, Window};

use super::app::{BinTab, Folio, MenuKind, Screen};
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
        // Each place has its own tinted tile and something to say on the
        // right; the one you're on is raised out of the panel.
        let nav = |id: &'static str, path: &'static str, label: &'static str, on: bool, tint: Hsla, ink: Hsla, right: Option<gpui::AnyElement>| {
            div()
                .id(id)
                .h(px(36.))
                .px(t.space.sm)
                .flex()
                .items_center()
                .gap(t.space.md)
                .rounded(t.radius.button)
                .cursor_pointer()
                .text_size(t.text.body)
                .when(on, |d| d.bg(t.colors.surface).shadow(t.shadow_raised()).font_weight(gpui::FontWeight::SEMIBOLD).text_color(t.colors.text))
                .when(!on, |d| d.text_color(t.colors.text_body).hover(|d| d.bg(t.colors.surface)))
                .active(|d| d.bg(t.colors.hover))
                .child(
                    div()
                        .size(px(26.))
                        .flex_none()
                        .flex()
                        .items_center()
                        .justify_center()
                        .rounded(t.radius.small)
                        .bg(tint)
                        .child(icon(path, t.size.icon_sm, ink)),
                )
                .child(div().flex_1().child(label))
                .children(right)
        };
        let pill = |text: String| {
            div()
                .px(px(7.))
                .h(px(18.))
                .flex()
                .items_center()
                .rounded(t.radius.full)
                .bg(t.colors.hover)
                .text_size(t.text.micro)
                .font_weight(gpui::FontWeight::MEDIUM)
                .text_color(t.colors.text_muted)
                .child(text)
                .into_any_element()
        };
        let trash = self.board.bin.iter().filter(|b| !b.archived).count();
        let sync_dot = {
            use super::sync::DriveState as S;
            match self.sync.state {
                S::Off | S::SigningIn => None,
                S::Idle => Some(t.colors.ok),
                S::Syncing => Some(t.colors.brand),
                S::Choice => Some(t.colors.alert),
                S::Error(_) => Some(t.colors.danger),
            }
        };
        let neutral = (t.colors.hover, t.colors.text_muted);
        let green = (t.colors.ok.opacity(0.16), t.colors.ok);
        let sheets_n = self.board.categories.len();
        let nav_group = div()
            .flex()
            .flex_col()
            .gap(px(2.))
            .p(t.space.xs)
            .rounded(t.radius.card)
            .bg(t.colors.surface_sunk)
            .border_1()
            .border_color(t.colors.border)
            .child(
                nav("nav-home", "icons/layout-grid.svg", "All sheets", screen == Screen::Home, t.colors.brand, t.colors.primary_fg, Some(pill(sheets_n.to_string())))
                    .on_click(cx.listener(|this, _, _, cx| this.go_home(cx))),
            )
            .child(
                nav("nav-bin", "icons/archive.svg", "Trash & archive", screen == Screen::Bin, neutral.0, neutral.1, (trash > 0).then(|| pill(trash.to_string())))
                    .on_click(cx.listener(|this, _, _, cx| this.open_bin(BinTab::Trash, cx))),
            )
            .child(
                nav(
                    "nav-transfer",
                    "icons/cloud.svg",
                    "Sync & export",
                    screen == Screen::Transfer,
                    green.0,
                    green.1,
                    sync_dot.map(|c| div().size(px(7.)).mr(px(5.)).rounded(t.radius.full).bg(c).into_any_element()),
                )
                .on_click(cx.listener(|this, _, _, cx| this.open_transfer(cx))),
            )
            .child(
                nav("nav-settings", "icons/settings.svg", "Settings", screen == Screen::Settings, neutral.0, neutral.1, Some(kbd(shortcut(","), &t).into_any_element()))
                    .on_click(cx.listener(|this, _, _, cx| this.open_settings(cx))),
            );
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
            .child(nav_group)
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
            .child(self.drawer_profile(&t, cx));
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

    /// You, at the foot of the drawer: your avatar and name, whether your
    /// work is saved (and synced), and Settings. Click it to change your
    /// avatar.
    fn drawer_profile(&self, t: &Theme, cx: &mut Context<Self>) -> gpui::AnyElement {
        let s = &self.board.settings;
        let name = if s.user_name.trim().is_empty() { "You".to_string() } else { s.user_name.clone() };
        let avatar = crate::avatars::all().find(|a| *a == s.avatar).unwrap_or("bow");
        let status = match (&self.sync.state, self.saved) {
            (_, false) => "Saving…".to_string(),
            (super::sync::DriveState::Idle, true) => "Saved · in Google Drive".to_string(),
            (super::sync::DriveState::Syncing, true) => "Saved · syncing…".to_string(),
            _ => "Saved on this computer".to_string(),
        };
        div()
            .flex()
            .items_center()
            .gap(t.space.sm)
            .p(t.space.sm)
            .rounded(t.radius.card)
            .bg(t.colors.surface_sunk)
            .border_1()
            .border_color(t.colors.border)
            .child(
                div()
                    .id("drawer-profile")
                    .flex_1()
                    .min_w_0()
                    .flex()
                    .items_center()
                    .gap(t.space.sm)
                    .rounded(t.radius.button)
                    .cursor_pointer()
                    .child(
                        div()
                            .size(px(38.))
                            .flex_none()
                            .p(px(2.))
                            .rounded(t.radius.full)
                            .border_2()
                            .border_color(t.colors.brand)
                            .child(img(SharedString::from(crate::avatars::path(avatar, 30))).size(px(30.)).rounded(t.radius.full)),
                    )
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .flex()
                            .flex_col()
                            .child(div().text_size(t.text.body).font_weight(gpui::FontWeight::SEMIBOLD).truncate().child(name))
                            .child(div().text_size(t.text.micro).text_color(t.colors.text_subtle).truncate().child(status)),
                    )
                    .on_click(cx.listener(|this, _, window, cx| this.open_welcome(true, window, cx))),
            )
            .child(icon_button("drawer-settings", "icons/settings.svg", t).on_click(cx.listener(|this, _, _, cx| this.open_settings(cx))))
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
