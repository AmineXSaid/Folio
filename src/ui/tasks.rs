//! Tasks: every `- [ ]` line in every note on every sheet, in one list,
//! grouped by sheet. Tick one here and it's ticked in its note (one undo
//! step); click its words to go to the note. Open, Done or All.

use gpui::{div, prelude::*, px, Context, SharedString, Window};

use super::app::{Folio, TaskFilter};
use super::home::segmented;
use super::*;
use crate::model::TaskRef;
use crate::theme::theme;

impl Folio {
    pub(super) fn render_tasks(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let t = theme(cx).clone();
        let all = self.board.tasks();
        let open = all.iter().filter(|x| !x.done).count();
        let done = all.len() - open;
        let filter = self.task_filter;
        let shown: Vec<TaskRef> = all
            .into_iter()
            .filter(|x| match filter {
                TaskFilter::Open => !x.done,
                TaskFilter::Done => x.done,
                TaskFilter::All => true,
            })
            .collect();

        let tabs = segmented(
            "task-filter",
            &[("Open", filter == TaskFilter::Open), ("Done", filter == TaskFilter::Done), ("All", filter == TaskFilter::All)],
            &t,
            {
                let me = cx.entity();
                move |i, _, cx| {
                    me.update(cx, |this, cx| {
                        this.task_filter = [TaskFilter::Open, TaskFilter::Done, TaskFilter::All][*i];
                        cx.notify();
                    })
                }
            },
        );

        // One card per sheet, its tasks in it.
        let mut groups: Vec<gpui::AnyElement> = Vec::new();
        let mut i = 0;
        while i < shown.len() {
            let sheet = shown[i].sheet;
            let end = shown[i..].iter().position(|x| x.sheet != sheet).map_or(shown.len(), |k| i + k);
            let c = &self.board.categories[sheet];
            let left = shown[i..end].iter().filter(|x| !x.done).count();
            let head = div()
                .flex()
                .items_center()
                .gap(t.space.sm)
                .px(t.space.md)
                .pt(t.space.md)
                .pb(t.space.xs)
                .when(!c.icon.is_empty(), |d| d.child(mark(&c.icon, 22., &t)))
                .child(div().font_weight(gpui::FontWeight::SEMIBOLD).text_size(t.text.title).child(c.name.clone()))
                .child(div().text_size(t.text.caption).text_color(t.colors.text_subtle).child(if left == 0 {
                    "all done".to_string()
                } else {
                    format!("{left} open")
                }));
            let rows = shown[i..end].iter().map(|task| self.task_row(task, &t, cx));
            groups.push(
                div()
                    .flex()
                    .flex_col()
                    .pb(t.space.sm)
                    .rounded(t.radius.dialog)
                    .bg(t.colors.surface)
                    .border_1()
                    .border_color(t.colors.border)
                    .child(head)
                    .children(rows)
                    .into_any_element(),
            );
            i = end;
        }

        let empty = groups.is_empty().then(|| {
            let (title, line) = match (filter, open + done) {
                (_, 0) => ("No tasks yet", "Write - [ ] in any note, or press Ctrl+Shift+C while typing in one."),
                (TaskFilter::Open, _) => ("Nothing left to do", "Every task on every sheet is ticked."),
                _ => ("Nothing ticked yet", "Tasks you tick show up here."),
            };
            div()
                .py(px(64.))
                .flex()
                .flex_col()
                .items_center()
                .gap(t.space.sm)
                .child(icon("icons/list-checks.svg", t.size.icon_lg, t.colors.text_subtle))
                .child(div().font_weight(gpui::FontWeight::SEMIBOLD).child(title))
                .child(div().text_size(t.text.caption).text_color(t.colors.text_muted).child(line))
        });

        div().id("tasks").size_full().overflow_y_scroll().child(
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
                                .child(div().text_size(t.text.heading).font_weight(gpui::FontWeight::SEMIBOLD).child("Tasks"))
                                .child(div().text_color(t.colors.text_muted).child(format!("{open} open · {done} done, on every sheet"))),
                        )
                        .child(tabs),
                )
                .children(groups)
                .children(empty),
        )
    }

    /// A task: its box (ticks it in its note), its words (go to the note),
    /// and the note it's in.
    fn task_row(&self, task: &TaskRef, t: &crate::theme::Theme, cx: &mut Context<Self>) -> gpui::AnyElement {
        let (note, n, done) = (task.note, task.n, task.done);
        let from: SharedString = self.board.find_note(note).map(super::home::first_line).unwrap_or_default().into();
        let text = if task.text.is_empty() { "(empty task)".to_string() } else { task.text.clone() };
        let md = if done { format!("~~{text}~~") } else { text };
        div()
            .id(("task-row", note as usize * 1024 + n))
            .mx(t.space.xs)
            .px(t.space.sm)
            .py(px(6.))
            .flex()
            .items_start()
            .gap(t.space.md)
            .rounded(t.radius.button)
            .hover(|d| d.bg(t.colors.hover))
            .child(
                div()
                    .id(("task-box", note as usize * 1024 + n))
                    .mt(px(2.))
                    .size(px(16.))
                    .flex_none()
                    .flex()
                    .items_center()
                    .justify_center()
                    .rounded(px(4.))
                    .border_1()
                    .border_color(if done { t.colors.brand } else { t.colors.text_subtle })
                    .when(done, |d| d.bg(t.colors.brand).child(icon("icons/check.svg", px(11.), t.colors.primary_fg)))
                    .cursor_pointer()
                    .on_click(cx.listener(move |this, _, _, cx| this.toggle_task(note, n, cx))),
            )
            .child(
                div()
                    .id(("task-go", note as usize * 1024 + n))
                    .flex_1()
                    .min_w_0()
                    .flex()
                    .flex_col()
                    .cursor_pointer()
                    .text_size(t.text.body)
                    .text_color(if done { t.colors.text_subtle } else { t.colors.text })
                    .child(super::sticker::md_view(note * 1024 + 512 + n as u64, &md, t))
                    .child(div().text_size(t.text.micro).text_color(t.colors.text_subtle).truncate().child(from))
                    .on_click(cx.listener(move |this, _, window, cx| this.reveal_note(note, window, cx))),
            )
            .into_any_element()
    }
}
