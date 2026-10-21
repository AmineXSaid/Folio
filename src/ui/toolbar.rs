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
        let tools = Tool::ALL.iter().map(|&tool| {
            let active = self.tool == tool;
            div()
                .id(tool.name())
                .size(px(36.))
                .flex()
                .flex_col()
                .items_center()
                .justify_center()
                .rounded(t.radius.button)
                .cursor_pointer()
                .when(active, |d| d.bg(t.colors.primary_tint))
                .when(!active, |d| d.hover(|d| d.bg(t.colors.hover)))
                .child(icon(tool.icon(), t.size.icon, if active { t.colors.primary_on_tint } else { t.colors.text_muted }))
                .on_click(cx.listener(move |this, _, _, cx| this.set_tool(tool, cx)))
        });
        let swatches = self.tool.uses_ink().then(|| {
            div()
                .flex()
                .items_center()
                .gap(t.space.xs)
                .pl(t.space.sm)
                .ml(t.space.xs)
                .border_l_1()
                .border_color(t.colors.border)
                .children(Ink::ALL.iter().map(|&ink| {
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
                        .on_click(cx.listener(move |this, _, _, cx| {
                            this.ink = ink;
                            cx.notify();
                        }))
                }))
        });
        let hint = format!("{} · {}", self.tool.name(), self.tool.key());
        div()
            .absolute()
            .bottom(t.space.xl)
            .left_0()
            .right_0()
            .flex()
            .flex_col()
            .items_center()
            .gap(t.space.sm)
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(t.space.xxs)
                    .p(t.space.xs)
                    .rounded(t.radius.card)
                    .bg(t.colors.surface)
                    .border_1()
                    .border_color(t.colors.border)
                    .shadow(t.shadow_float())
                    .children(tools)
                    .children(swatches),
            )
            .child(div().text_size(t.text.micro).text_color(t.colors.text_subtle).child(hint))
    }

    pub(super) fn render_zoom(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let t = theme(cx).clone();
        let pct = format!("{}%", (self.sheet().camera.zoom * 100.).round());
        div()
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
