//! Trash (30 days), Archive (kept), and the daily backups, each with a way
//! back.

use gpui::{div, prelude::*, px, Context, Window};

use super::app::{BinTab, Folio};
use super::home::segmented;
use super::*;
use crate::model::*;
use crate::store;
use crate::theme::theme;
use crate::time::{self, now};

impl Folio {
    pub(super) fn render_bin(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let t = theme(cx).clone();
        let tab = self.bin_tab;
        let n = now();
        let trash = self.board.bin.iter().filter(|b| !b.archived).count();
        let archive = self.board.bin.len() - trash;
        let tabs = segmented(
            "bin-tab",
            &[("Trash", tab == BinTab::Trash), ("Archive", tab == BinTab::Archive), ("Backups", tab == BinTab::Backups)],
            &t,
            cx.listener(|this, ix: &usize, _, cx| {
                this.bin_tab = [BinTab::Trash, BinTab::Archive, BinTab::Backups][*ix];
                cx.notify();
            }),
        );
        let mut rows: Vec<gpui::AnyElement> = Vec::new();
        match tab {
            BinTab::Trash | BinTab::Archive => {
                let archived = tab == BinTab::Archive;
                let mut items: Vec<&Binned> = self.board.bin.iter().filter(|b| b.archived == archived).collect();
                items.sort_by_key(|b| std::cmp::Reverse(b.at));
                for b in items {
                    let (path, kind, from) = match &b.item {
                        BinItem::Note { sheet_name, .. } => ("icons/sticky-note.svg", "Note", format!("from “{sheet_name}”")),
                        BinItem::Sheet(c) => ("icons/layout-grid.svg", "Sheet", format!("{} notes", c.stickers.len())),
                    };
                    let when = if archived {
                        format!("Archived {}", time::ago(b.at, n).to_lowercase())
                    } else {
                        let left = (TRASH_KEEPS.saturating_sub(n.saturating_sub(b.at)) / 86_400).max(1);
                        format!("Deleted {} · {left} days left", time::ago(b.at, n).to_lowercase())
                    };
                    let id = b.id;
                    rows.push(
                        div()
                            .flex()
                            .items_center()
                            .gap(t.space.md)
                            .p(t.space.md)
                            .rounded(t.radius.card)
                            .bg(t.colors.surface)
                            .border_1()
                            .border_color(t.colors.border)
                            .child(icon(path, t.size.icon, t.colors.text_muted))
                            .child(
                                div()
                                    .flex_1()
                                    .min_w_0()
                                    .flex()
                                    .flex_col()
                                    .child(div().font_weight(gpui::FontWeight::MEDIUM).truncate().child(b.name()))
                                    .child(div().text_size(t.text.caption).text_color(t.colors.text_subtle).child(format!("{kind} {from} · {when}"))),
                            )
                            .child(secondary_button(("restore", id), "Restore", &t).on_click(cx.listener(move |this, _, _, cx| this.restore_binned(id, cx))))
                            .child(
                                div()
                                    .id(("forever", id))
                                    .h(t.size.control)
                                    .px(t.space.md)
                                    .flex()
                                    .items_center()
                                    .rounded(t.radius.button)
                                    .text_size(t.text.caption)
                                    .text_color(t.colors.danger)
                                    .cursor_pointer()
                                    .hover(|d| d.bg(t.colors.hover))
                                    .child("Delete forever")
                                    .on_click(cx.listener(move |this, _, _, cx| this.delete_forever(id, cx))),
                            )
                            .into_any_element(),
                    );
                }
                if rows.is_empty() {
                    rows.push(
                        div()
                            .py(px(60.))
                            .flex()
                            .justify_center()
                            .text_color(t.colors.text_muted)
                            .child(if archived { "Nothing archived. Archive a note or sheet from its menu to keep it out of the way." } else { "The Trash is empty." })
                            .into_any_element(),
                    );
                }
            }
            BinTab::Backups => {
                for day in store::backups(&self.dir) {
                    let d = day.clone();
                    rows.push(
                        div()
                            .flex()
                            .items_center()
                            .gap(t.space.md)
                            .p(t.space.md)
                            .rounded(t.radius.card)
                            .bg(t.colors.surface)
                            .border_1()
                            .border_color(t.colors.border)
                            .child(icon("icons/hard-drive-download.svg", t.size.icon, t.colors.text_muted))
                            .child(div().flex_1().child(format!("Backup of {day}")))
                            .child(
                                secondary_button(gpui::ElementId::Name(format!("restore-{day}").into()), "Restore this day", &t)
                                    .on_click(cx.listener(move |this, _, _, cx| this.restore_backup(&d, cx))),
                            )
                            .into_any_element(),
                    );
                }
                if rows.is_empty() {
                    rows.push(div().py(px(60.)).flex().justify_center().text_color(t.colors.text_muted).child("No backups yet.").into_any_element());
                }
            }
        }
        let action = match tab {
            BinTab::Trash if trash > 0 => Some(secondary_button("empty", "Empty Trash", &t).on_click(cx.listener(|this, _, _, cx| this.empty_trash(cx))).into_any_element()),
            BinTab::Backups => Some(secondary_button("backup-now", "Back up now", &t).on_click(cx.listener(|this, _, _, cx| this.back_up_now(cx))).into_any_element()),
            _ => None,
        };
        let sub = match tab {
            BinTab::Trash => format!("{trash} item{} · kept for 30 days, then gone", if trash == 1 { "" } else { "s" }),
            BinTab::Archive => format!("{archive} item{} · kept until you delete them", if archive == 1 { "" } else { "s" }),
            BinTab::Backups => format!("Folio backs up once a day and keeps the last {}", store::BACKUPS_KEPT),
        };
        div().id("bin").size_full().overflow_y_scroll().child(
            div()
                .max_w(px(820.))
                .mx_auto()
                .px(t.space.xl)
                .py(t.space.xxl)
                .flex()
                .flex_col()
                .gap(t.space.lg)
                .child(
                    div()
                        .flex()
                        .items_end()
                        .gap(t.space.lg)
                        .child(
                            div()
                                .flex_1()
                                .flex()
                                .flex_col()
                                .child(div().text_size(px(28.)).font_weight(gpui::FontWeight::SEMIBOLD).child("Trash & archive"))
                                .child(div().text_color(t.colors.text_muted).child(sub)),
                        )
                        .children(action)
                        .child(tabs),
                )
                .child(div().flex().flex_col().gap(t.space.sm).children(rows)),
        )
    }
}
