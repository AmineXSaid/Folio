//! The selection bar: with two or more notes selected (Ctrl or Shift and a
//! click, or a box drawn on the sheet), a bar rises above the dock. It says
//! how many and does to all of them what a note's menu does to one: lock,
//! duplicate, move to a sheet, archive, trash; "More" opens the whole menu
//! for the group.

use gpui::{div, prelude::*, px, Context, MouseButton, MouseDownEvent, SharedString};

use super::app::{Folio, Gesture, Menu, MenuKind, MenuPage, Tool};
use super::*;
use crate::theme::Theme;

impl Folio {
    pub(super) fn render_selection_bar(&mut self, t: &Theme, cx: &mut Context<Self>) -> Option<gpui::AnyElement> {
        let ids = self.selected_notes();
        let dragging = matches!(self.gesture, Gesture::Move { moved: true, .. });
        if ids.len() < 2 || self.tool != Tool::Select || dragging || self.editing.is_some() {
            return None;
        }
        let first = ids[0];
        let locked = ids.iter().all(|id| self.board.find_note(*id).is_some_and(|s| s.locked));
        let button = |id: &'static str, path: &'static str, hint: String, danger: bool| {
            div()
                .id(id)
                .size(px(34.))
                .flex()
                .items_center()
                .justify_center()
                .rounded(t.radius.button)
                .cursor_pointer()
                .hover(|d| d.bg(t.colors.hover))
                .active(|d| d.bg(t.colors.border))
                .tooltip(move |window, cx| tip(hint.clone(), window, cx))
                .child(icon(path, t.size.icon, if danger { t.colors.danger } else { t.colors.text_muted }))
        };
        // The menu, at the button, for the selection (a selected note's
        // menu acts on all of them).
        let menu_at = move |page: MenuPage| {
            move |this: &mut Folio, ev: &MouseDownEvent, _: &mut gpui::Window, cx: &mut Context<Folio>| {
                cx.stop_propagation();
                this.menu = Some(Menu { at: ev.position, kind: MenuKind::Note(first), page });
                cx.notify();
            }
        };
        let rule = || div().w(px(1.)).h(t.size.icon_lg).mx(t.space.xs).bg(t.colors.border);
        let count = div()
            .flex()
            .items_center()
            .gap(t.space.sm)
            .pl(t.space.sm)
            .pr(t.space.xs)
            .child(
                div()
                    .min_w(t.size.icon_lg)
                    .h(t.size.icon_lg)
                    .px(t.space.xs)
                    .flex()
                    .items_center()
                    .justify_center()
                    .rounded(t.radius.full)
                    .bg(t.colors.brand)
                    .text_color(t.colors.primary_fg)
                    .text_size(t.text.micro)
                    .font_weight(gpui::FontWeight::BOLD)
                    .child(SharedString::from(ids.len().to_string())),
            )
            .child(div().text_size(t.text.body).font_weight(gpui::FontWeight::MEDIUM).text_color(t.colors.text).child("notes selected"));
        let bar = super::toolbar::dock(t)
            .id("selection-bar")
            .occlude()
            .gap(px(2.))
            .p(px(5.))
            .child(count)
            .child(rule())
            .child(
                button("sel-lock", if locked { "icons/lock-open.svg" } else { "icons/lock.svg" }, if locked { "Unlock all" } else { "Lock all" }.into(), false)
                    .on_click({
                        let ids = ids.clone();
                        cx.listener(move |this, _, _, cx| this.lock_notes(&ids, cx))
                    }),
            )
            .child(button("sel-dup", "icons/copy.svg", format!("Duplicate   {}", shortcut("D")), false).on_click(cx.listener(|this, _, _, cx| this.duplicate_selection(cx))))
            .child(button("sel-move", "icons/folder-input.svg", "Move to sheet…".into(), false).on_mouse_down(MouseButton::Left, cx.listener(menu_at(MenuPage::MoveTo))))
            .child(
                button("sel-archive", "icons/archive.svg", "Archive".into(), false).on_click({
                    let ids = ids.clone();
                    cx.listener(move |this, _, _, cx| this.bin_notes(&ids, true, cx))
                }),
            )
            .child(button("sel-trash", "icons/trash.svg", "Move to Trash   Del".into(), true).on_click(cx.listener(move |this, _, _, cx| this.bin_notes(&ids, false, cx))))
            .child(rule())
            .child(button("sel-more", "icons/ellipsis.svg", "More…".into(), false).on_mouse_down(MouseButton::Left, cx.listener(menu_at(MenuPage::Main))))
            .child(
                button("sel-clear", "icons/x.svg", "Clear the selection   Esc".into(), false).on_click(cx.listener(|this, _, _, cx| {
                    this.selection.clear();
                    cx.notify();
                })),
            );
        Some(div().mb(t.space.sm).child(motion::rise_in(bar, "selection-in", t)).into_any_element())
    }
}
