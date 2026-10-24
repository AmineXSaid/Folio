//! The floating tool dock at the foot of the sheet, its ink swatches, and
//! the zoom control in the corner.

use gpui::{div, prelude::*, px, Context, Window};

use super::app::{Folio, Tool};
use super::*;
use crate::model::Ink;
use crate::theme::theme;

impl Folio {
    pub(super) fn render_toolbar(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let t = theme(cx).clone();
        let tools: Vec<_> = Tool::ALL.iter().map(|&tool| {
            let active = self.tool == tool;
            let tip = format!("{}   {}", tool.name(), tool.key());
            div()
                .id(tool.name())
                .relative()
                .size(px(38.))
                .flex()
                .items_center()
                .justify_center()
                .rounded(px(11.))
                .cursor_pointer()
                .tooltip(move |window, cx| super::tip(tip.clone(), window, cx))
                // The tool in use sits on a raised chip with a dot under it,
                // like a running app in the macOS Dock.
                .when(active, |d| {
                    d.bg(t.colors.surface)
                        .shadow(t.shadow_raised())
                        .child(div().absolute().bottom(px(3.)).size(px(4.)).rounded(t.radius.full).bg(t.colors.brand))
                })
                .when(!active, |d| d.hover(|d| d.bg(t.colors.hover)).active(|d| d.bg(t.colors.border)))
                .child(icon(tool.icon(), px(17.), if active { t.colors.brand } else { t.colors.text_muted }))
                // The tool in use, clicked again, goes back to Select: no
                // tool ever stays stuck on.
                .on_click(cx.listener(move |this, _, _, cx| {
                    let to = if this.tool == tool { Tool::Select } else { tool };
                    this.set_tool(to, cx)
                }))
        }).collect();
        // The inks float in their own pill beside the dock, so the tools
        // never move under the pointer when they appear. The pill and the
        // palette over it rise in, and sink away when put away.
        let m = t.motion;
        let palette_going = self
            .exits
            .ink_palette
            .leaving(m.pop_out)
            .map(|(_, n)| super::motion::leave(self.render_ink_palette(&t, cx), ("ink-palette-out", n), m.pop_out, m.sink));
        let swatches = if self.tool.uses_ink() {
            Some(super::motion::rise_in(self.ink_pill(&t, palette_going, cx), "inks-in", &t))
        } else {
            self.exits.inks.leaving(m.pop_out).map(|(_, n)| super::motion::leave(self.ink_pill(&t, None, cx), ("inks-out", n), m.pop_out, m.sink))
        };
        div()
            .absolute()
            .bottom(t.space.xl)
            .left_0()
            .right_0()
            .flex()
            .flex_col()
            .items_center()
            .children(self.render_selection_bar(&t, cx))
            .child(
                dock(&t)
                    .id("dock")
                    // Clicks on the dock never reach the sheet under it.
                    .occlude()
                    .relative()
                    .gap(px(3.))
                    .p(px(5.))
                    .children(tools)
                    .children(swatches),
            )
    }

    /// The ink for what comes next, and for the text being written now.
    fn pick_ink(&mut self, ink: Ink, cx: &mut Context<Self>) {
        self.ink = ink;
        self.ink_palette = false;
        if let Some(id) = self.label_edit {
            if let Some(l) = self.board.sheet_mut().labels.iter_mut().find(|l| l.id == id) {
                l.ink = ink;
            }
        }
        cx.notify();
    }

    /// The inks at hand, in their pill beside the dock, with every ink in
    /// a palette above while it's open (or `going`, as it leaves).
    fn ink_pill(&self, t: &crate::theme::Theme, going: Option<gpui::AnyElement>, cx: &mut Context<Self>) -> gpui::Stateful<gpui::Div> {
        div()
            .id("inks")
            .occlude()
            .absolute()
            .top_0()
            .bottom_0()
            .left_full()
            .ml(t.space.sm)
            .gap(t.space.xs)
            .px(t.space.sm)
            .flex()
            .items_center()
            .rounded(px(16.))
            .bg(dock_body(t))
            .border_1()
            .border_color(dock_edge(t))
            .shadow(dock_shadow(t))
            .children({
                // The five at hand, and the current one if it's another.
                let mut quick: Vec<Ink> = Ink::QUICK.to_vec();
                if !quick.contains(&self.ink) {
                    quick.push(self.ink);
                }
                quick.into_iter().map(|ink| {
                let on = self.ink == ink;
                div()
                    .id(("ink", ink as usize))
                    .size(px(24.))
                    .flex()
                    .items_center()
                    .justify_center()
                    .rounded(t.radius.full)
                    .cursor_pointer()
                    .when(on, |d| d.border_2().border_color(t.ink(ink)))
                    .child(div().size(px(14.)).rounded(t.radius.full).bg(t.ink(ink)))
                    .on_click(cx.listener(move |this, _, _, cx| this.pick_ink(ink, cx)))
                }).collect::<Vec<_>>()
            })
            .child(div().w(px(1.)).h(px(18.)).mx(px(2.)).bg(t.colors.border))
            // Every ink, in a palette above.
            .child(
                div()
                    .id("ink-more")
                    .size(px(26.))
                    .flex()
                    .items_center()
                    .justify_center()
                    .rounded(t.radius.small)
                    .cursor_pointer()
                    .when(self.ink_palette, |d| d.bg(t.colors.brand_tint))
                    .hover(|d| d.bg(t.colors.hover))
                    .child(icon("icons/palette.svg", t.size.icon_sm, if self.ink_palette { t.colors.brand } else { t.colors.text_muted }))
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.ink_palette = !this.ink_palette;
                        cx.notify();
                    })),
            )
            .when(self.ink_palette, |d| d.child(super::motion::rise_in(self.render_ink_palette(t, cx), "ink-palette-in", t)))
            .children(going)
    }

    /// All the inks in a grid, hung above the ink pill.
    fn render_ink_palette(&self, t: &crate::theme::Theme, cx: &mut Context<Self>) -> gpui::Stateful<gpui::Div> {
        let current = self.ink;
        div()
                .id("ink-palette")
                .occlude()
                .absolute()
                .bottom_full()
                .right_0()
                .mb(t.space.sm)
                .w(px(196.))
                .p(t.space.sm)
                .flex()
                .flex_col()
                .gap(t.space.sm)
                .rounded(t.radius.card)
                .bg(t.colors.surface)
                .border_1()
                .border_color(t.colors.border)
                .shadow(t.shadow_float())
                .child(div().text_size(t.text.micro).font_weight(gpui::FontWeight::SEMIBOLD).text_color(t.colors.text_subtle).child("INK"))
                .child(div().flex().flex_wrap().gap(px(4.)).children(Ink::ALL.iter().map(|&ink| {
                    let on = ink == current;
                    div()
                        .id(("ink-all", ink as usize))
                        .size(px(26.))
                        .flex()
                        .items_center()
                        .justify_center()
                        .rounded(t.radius.full)
                        .cursor_pointer()
                        .border_2()
                        .border_color(if on { t.ink(ink) } else { gpui::transparent_black() })
                        .hover(|d| d.bg(t.colors.hover))
                        .child(div().size(px(16.)).rounded(t.radius.full).bg(t.ink(ink)))
                        .on_click(cx.listener(move |this, _, _, cx| this.pick_ink(ink, cx)))
                })))
    }

    pub(super) fn render_zoom(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let t = theme(cx).clone();
        let pct = format!("{}%", (self.sheet().camera.zoom * 100.).round());
        div()
            .id("zoom")
            .occlude()
            .absolute()
            .bottom(t.space.xl)
            .right(t.space.xl)
            .flex()
            .items_center()
            .p(t.space.xxs)
            .rounded(t.radius.button)
            .bg(t.colors.surface)
            .border_1()
            .border_color(t.colors.border)
            .shadow(t.shadow_float())
            .child(icon_button("zoom-out", "icons/zoom-out.svg", &t).on_click(cx.listener(|this, _, _, cx| this.zoom_by(1. / 1.2, cx))))
            .child(
                div()
                    .w(px(44.))
                    .flex()
                    .justify_center()
                    .text_size(t.text.caption)
                    .text_color(t.colors.text_muted)
                    .child(pct),
            )
            .child(icon_button("zoom-in", "icons/zoom-in.svg", &t).on_click(cx.listener(|this, _, _, cx| this.zoom_by(1.2, cx))))
            .child(icon_button("zoom-fit", "icons/scan.svg", &t).on_click(cx.listener(|this, _, _, cx| this.zoom_to_fit(cx))))
    }
}

/// The glass the dock is made of: a translucent body, a lit top edge, a
/// deep soft shadow, generous rounding.
pub(super) fn dock(t: &crate::theme::Theme) -> gpui::Div {
    div()
        .flex()
        .items_center()
        .rounded(px(16.))
        .bg(dock_body(t))
        .border_1()
        .border_color(dock_edge(t))
        .shadow(dock_shadow(t))
}

fn dock_body(t: &crate::theme::Theme) -> gpui::Hsla {
    let mut body = t.colors.surface;
    body.a = if t.is_dark { 0.86 } else { 0.78 };
    body
}

fn dock_edge(t: &crate::theme::Theme) -> gpui::Hsla {
    if t.is_dark { gpui::hsla(0., 0., 1., 0.10) } else { gpui::hsla(0., 0., 1., 0.9) }
}

fn dock_shadow(t: &crate::theme::Theme) -> Vec<gpui::BoxShadow> {
    let mut s = t.shadow_float();
    s.push(gpui::BoxShadow {
        color: gpui::hsla(0., 0., 0., if t.is_dark { 0.35 } else { 0.06 }),
        offset: gpui::point(px(0.), px(0.)),
        blur_radius: px(0.),
        spread_radius: px(1.),
        inset: false,
    });
    s
}
