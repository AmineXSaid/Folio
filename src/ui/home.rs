//! The home screen: every sheet as a card that fills the window, the
//! reminders due today, your #tags, and, while you search, the notes that
//! match on every sheet.

use gpui::{div, prelude::*, px, Context, MouseButton, MouseDownEvent, SharedString, Window};

use super::app::{BinTab, Folio, MenuKind, MenuPage, Sort};
use super::mini::miniature;
use super::*;
use crate::model::*;
use crate::theme::{theme, Theme};
use crate::time::{self, now};

impl Folio {
    pub(super) fn render_home(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let t = theme(cx).clone();
        let q = self.search_text(cx);
        let n_notes: usize = self.board.categories.iter().map(|c| c.stickers.len()).sum();
        let pad = f32::from(t.space.xxl);
        let gap = 20.;
        let width = f32::from(window.viewport_size().width) - 2. * pad;
        let min = f32::from(t.size.card_min);
        let cols = ((width + gap) / (min + gap)).floor().max(1.) as usize;
        let card_w = ((width - gap * (cols as f32 - 1.)) / cols as f32).max(150.);

        // Greets you by name, by the time of day, and moves on to another
        // line every five minutes.
        let hour = crate::greeting::hour_now();
        let greet = crate::greeting::pick(&self.board.settings.user_name, hour, now() / 300);
        let (title, sub) = if q.is_empty() {
            let count = |n: usize, one: &str| if n == 1 { format!("1 {one}") } else { format!("{n} {one}s") };
            (
                greet,
                format!("{} · {} · {}", crate::greeting::sub(hour), count(self.board.categories.len(), "sheet"), count(n_notes, "note")),
            )
        } else {
            let hits = self.board.search(&q).len();
            let what = match q.strip_prefix("paper:") {
                Some(p) => format!("{}{} notes", p[..1].to_uppercase(), &p[1..]),
                None => format!("Results for “{q}”"),
            };
            (what, format!("{hits} note{} found", if hits == 1 { "" } else { "s" }))
        };

        let sort = self.sort;
        let header = div()
            .flex()
            .items_end()
            .gap(t.space.lg)
            .child(
                div()
                    .flex_1()
                    .flex()
                    .flex_col()
                    .child(div().text_size(px(28.)).font_weight(gpui::FontWeight::SEMIBOLD).child(title))
                    .child(div().text_color(t.colors.text_muted).child(sub)),
            )
            // A colour or a tag picked: a way back to all sheets.
            .when(!q.is_empty(), |d| {
                d.child(secondary_button("clear-filter", "Clear", &t).on_click(cx.listener(|this, _, window, cx| {
                    this.search_input.update(cx, |i, cx| i.set_value("", window, cx));
                    cx.notify();
                })))
            })
            .when(q.is_empty(), |d| {
                d.child(segmented(
                    "sort",
                    &[("Recent", sort == Sort::Recent), ("A–Z", sort == Sort::Name)],
                    &t,
                    cx.listener(|this, ix: &usize, _, cx| {
                        this.sort = if *ix == 0 { Sort::Recent } else { Sort::Name };
                        cx.notify();
                    }),
                ))
            });

        let mut body = div().flex().flex_col().gap(t.space.xl).child(header);

        if q.is_empty() {
            if let Some(today) = self.render_today(&t, cx) {
                body = body.child(today);
            }
            // Your #tags, to filter by.
            let tags = self.board.all_tags();
            body = body.when(!tags.is_empty(), |b| b.child(
                div()
                    .flex()
                    .flex_wrap()
                    .items_center()
                    .gap(t.space.sm)
                    .children(tags.into_iter().take(14).map(|(tag, n)| -> gpui::AnyElement {
                        let label = format!("#{tag}");
                        let id = SharedString::from(format!("tag-{tag}"));
                        div()
                            .id(id)
                            .flex()
                            .items_center()
                            .gap(t.space.xs)
                            .px(t.space.md)
                            .h(px(28.))
                            .rounded(t.radius.full)
                            .bg(t.colors.surface)
                            .border_1()
                            .border_color(t.colors.border)
                            .text_size(t.text.caption)
                            .text_color(t.colors.text_muted)
                            .cursor_pointer()
                            .hover(|d| d.border_color(t.colors.brand).text_color(t.colors.brand_on_tint))
                            .child(label.clone())
                            .child(div().text_color(t.colors.text_subtle).child(n.to_string()))
                            .on_click(cx.listener(move |this, _, window, cx| {
                                let l = label.clone();
                                this.search_input.update(cx, |i, cx| i.set_value(l, window, cx));
                                cx.notify();
                            }))
                            .into_any_element()
                    })),
            ));
            let mut order = self.board.ordered();
            if self.sort == Sort::Name {
                order.sort_by_key(|&i| {
                    let c = &self.board.categories[i];
                    (!c.inbox, !c.pinned, c.name.to_lowercase())
                });
            } else {
                order.sort_by_key(|&i| {
                    let c = &self.board.categories[i];
                    (!c.inbox, !c.pinned, std::cmp::Reverse(c.opened.max(c.edited)))
                });
            }
            // The cards arrive one after another.
            let cards: Vec<_> = order
                .iter()
                .enumerate()
                .map(|(n, &i)| {
                    let id = self.board.categories[i].id;
                    super::motion::card_in(self.sheet_card(i, card_w, &t, cx), ("card-in", id as usize), n, &t)
                })
                .collect();
            body = body.child(
                div()
                    .flex()
                    .flex_wrap()
                    .gap(px(gap))
                    .children(cards)
                    .child(new_sheet_card(card_w, &t, cx)),
            );
        } else {
            body = body.child(self.render_results(&q, card_w, &t, cx));
        }

        div()
            .relative()
            .size_full()
            .child(
                div()
                    .id("home")
                    .size_full()
                    .overflow_y_scroll()
                    .px(t.space.xxl)
                    .pt(t.space.xl)
                    // Room to scroll the last cards clear of the Trash and Archive.
                    .pb(px(96.))
                    .child(body),
            )
            .child(self.render_bin_dock(&t, cx))
    }

    /// Trash and Archive, held in the window's bottom-right corner whatever
    /// its size: each with how much is in it.
    fn render_bin_dock(&self, t: &Theme, cx: &mut Context<Self>) -> impl IntoElement {
        let archived = self.board.bin.iter().filter(|b| b.archived).count();
        let trashed = self.board.bin.len() - archived;
        let part = |id: &'static str, path: &'static str, label: &'static str, n: usize, tab: BinTab, tip: &'static str| {
            div()
                .id(id)
                .h(px(32.))
                .pl(px(10.))
                .pr(px(if n > 0 { 6. } else { 12. }))
                .flex()
                .items_center()
                .gap(px(7.))
                .rounded(t.radius.full)
                .text_size(t.text.caption)
                .text_color(t.colors.text_muted)
                .cursor_pointer()
                .hover(|d| d.bg(t.colors.hover).text_color(t.colors.text))
                .child(icon(path, t.size.icon_sm, t.colors.text_muted))
                .child(label)
                .when(n > 0, |d| {
                    d.child(
                        div()
                            .min_w(px(20.))
                            .h(px(20.))
                            .px(px(6.))
                            .flex()
                            .items_center()
                            .justify_center()
                            .rounded(t.radius.full)
                            .bg(t.colors.surface_sunk)
                            .border_1()
                            .border_color(t.colors.border)
                            .text_size(t.text.micro)
                            .font_weight(gpui::FontWeight::SEMIBOLD)
                            .text_color(t.colors.text_body)
                            .child(n.to_string()),
                    )
                })
                .tooltip(move |window, cx| super::tip(tip, window, cx))
                .on_click(cx.listener(move |this, _, _, cx| this.open_bin(tab, cx)))
        };
        div()
            .absolute()
            .right(t.space.xl)
            .bottom(t.space.xl)
            .flex()
            .items_center()
            .gap(px(2.))
            .p(px(4.))
            .rounded(t.radius.full)
            .bg(t.colors.surface)
            .border_1()
            .border_color(t.colors.border)
            .shadow(t.shadow_float())
            .child(part("to-trash", "icons/trash.svg", "Trash", trashed, BinTab::Trash, "Deleted notes and sheets, kept 30 days"))
            .child(div().w(px(1.)).h(px(18.)).bg(t.colors.border))
            .child(part("to-archive", "icons/archive.svg", "Archive", archived, BinTab::Archive, "Archived notes and sheets, kept until you delete them"))
    }

    /// Reminders due today or overdue, as a strip of chips.
    fn render_today(&self, t: &Theme, cx: &mut Context<Self>) -> Option<impl IntoElement> {
        let n = now();
        let due: Vec<(usize, Id, u64)> =
            self.board.reminders().into_iter().filter(|r| time::due_by_today(r.2, n)).collect();
        if due.is_empty() {
            return None;
        }
        let rows = due.into_iter().map(|(ix, id, d)| {
            let c = &self.board.categories[ix];
            let s = c.sticker(id).unwrap();
            let name = first_line(s);
            let overdue = d <= n;
            let tint = if overdue { t.colors.alert } else { t.colors.brand_on_tint };
            div()
                .id(("today", id))
                .flex()
                .items_center()
                .gap(t.space.md)
                .pl(t.space.md)
                .pr(t.space.xs)
                .h(px(44.))
                .rounded(t.radius.card)
                .bg(t.paper(s.paper))
                .border_1()
                .border_color(t.colors.border)
                .cursor_pointer()
                .child(icon("icons/bell.svg", t.size.icon_sm, tint))
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .child(div().text_size(t.text.body).font_weight(gpui::FontWeight::MEDIUM).child(name))
                        .child(
                            div()
                                .text_size(t.text.micro)
                                .text_color(tint)
                                .child(format!("{} · {}", time::due_label(d, n), c.name)),
                        ),
                )
                .child(
                    div()
                        .id(("today-done", id))
                        .ml(t.space.sm)
                        .size(px(32.))
                        .rounded(t.radius.button)
                        .flex()
                        .items_center()
                        .justify_center()
                        .hover(|d| d.bg(t.colors.hover))
                        .child(icon("icons/check.svg", t.size.icon, t.colors.text_muted))
                        .on_click(cx.listener(move |this, _, _, cx| {
                            cx.stop_propagation();
                            this.set_reminder(id, None, cx);
                            this.notify("Done · reminder cleared", cx);
                        })),
                )
                .on_click(cx.listener(move |this, _, window, cx| this.reveal_note(id, window, cx)))
        });
        Some(
            div()
                .flex()
                .flex_col()
                .gap(t.space.sm)
                .child(section_label("TODAY", t))
                .child(div().flex().flex_wrap().gap(t.space.sm).children(rows)),
        )
    }

    fn sheet_card(&self, ix: usize, w: f32, t: &Theme, cx: &mut Context<Self>) -> impl IntoElement {
        let c = &self.board.categories[ix];
        let inner = w - 20.;
        let thumb = miniature(c, inner, (inner * 0.66).round(), true, &crate::store::media_dir(&self.dir), t);
        let n = c.stickers.len();
        let when = format!("{n} note{} · {}", if n == 1 { "" } else { "s" }, time::ago(c.opened.max(c.edited), now()));
        let name = c.name.clone();
        let pinned = c.pinned && !c.inbox;
        let shadow = t.shadow_card(0.6);
        div()
            .id(("card", ix))
            .w(px(w))
            .p(px(10.))
            .flex()
            .flex_col()
            .gap(px(10.))
            .rounded(px(18.))
            .bg(t.colors.surface)
            .border_1()
            .border_color(t.colors.border)
            .shadow(shadow)
            .cursor_pointer()
            .hover(|d| d.border_color(t.colors.brand))
            .child(thumb)
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(t.space.sm)
                    .px(px(4.))
                    .child(sheet_badge(c, 32., t))
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .flex()
                            .flex_col()
                            .child(div().text_size(px(15.)).font_weight(gpui::FontWeight::SEMIBOLD).truncate().child(name))
                            .child(div().text_size(t.text.micro).text_color(t.colors.text_subtle).child(when)),
                    )
                    .when(pinned, |d| d.child(icon("icons/pin.svg", t.size.icon_sm, t.colors.text_subtle)))
                    .child(
                        div()
                            .id(("card-more", ix))
                            .size(px(28.))
                            .flex_none()
                            .rounded(t.radius.button)
                            .flex()
                            .items_center()
                            .justify_center()
                            .hover(|d| d.bg(t.colors.hover))
                            .child(icon("icons/ellipsis.svg", t.size.icon, t.colors.text_muted))
                            .on_mouse_down(
                                MouseButton::Left,
                                cx.listener(move |this, ev: &MouseDownEvent, _, cx| {
                                    cx.stop_propagation();
                                    this.open_menu(ev.position, MenuKind::Card(ix), cx);
                                }),
                            ),
                    ),
            )
            .on_mouse_down(
                MouseButton::Right,
                cx.listener(move |this, ev: &MouseDownEvent, _, cx| this.open_menu(ev.position, MenuKind::Card(ix), cx)),
            )
            .on_click(cx.listener(move |this, _, window, cx| this.open_sheet(ix, window, cx)))
    }

    /// While searching: the matching notes, then the matching sheets.
    fn render_results(&self, q: &str, card_w: f32, t: &Theme, cx: &mut Context<Self>) -> impl IntoElement {
        let hits = self.board.search(q);
        let shown = hits.len().min(60);
        // Each note found fades up into the list; ones already there stay.
        let rows = hits.into_iter().take(60).enumerate().map(|(i, h)| {
            let c = &self.board.categories[h.sheet];
            let s = c.sticker(h.note).unwrap();
            let title = first_line(s);
            let snippet = snippet(s, q);
            let tags = s.tags();
            let id = h.note;
            let row = div()
                .id(("hit", id))
                .flex()
                .items_start()
                .gap(t.space.md)
                .p(t.space.md)
                .rounded(t.radius.card)
                .bg(t.colors.surface)
                .border_1()
                .border_color(t.colors.border)
                .cursor_pointer()
                .hover(|d| d.border_color(t.colors.brand))
                .child(div().mt(px(3.)).size(px(14.)).flex_none().rounded(px(4.)).bg(t.paper(s.paper)).border_1().border_color(t.colors.border))
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .flex()
                        .flex_col()
                        .gap(px(2.))
                        .child(div().font_weight(gpui::FontWeight::SEMIBOLD).truncate().child(title))
                        .when(!snippet.is_empty(), |d| {
                            d.child(div().text_size(t.text.caption).text_color(t.colors.text_muted).truncate().child(snippet))
                        })
                        .when(!tags.is_empty(), |d| {
                            d.child(
                                div().flex().gap(t.space.xs).children(tags.iter().take(4).map(|tg| {
                                    div().text_size(t.text.micro).text_color(t.colors.brand_on_tint).child(format!("#{tg}"))
                                })),
                            )
                        }),
                )
                .child(
                    div()
                        .flex_none()
                        .flex()
                        .items_center()
                        .gap(t.space.xs)
                        .px(t.space.sm)
                        .h(px(24.))
                        .rounded(t.radius.full)
                        .bg(t.colors.hover)
                        .text_size(t.text.micro)
                        .text_color(t.colors.text_muted)
                        .when(!c.icon.is_empty(), |d| d.child(mark(&c.icon, 18., &t)))
                        .child(c.name.clone()),
                )
                .on_click(cx.listener(move |this, _, window, cx| this.reveal_note(id, window, cx)));
            super::motion::row_in(row, ("hit-in", id as usize), i, t)
        });
        let ql = q.to_lowercase();
        let sheets: Vec<usize> = (0..self.board.categories.len())
            .filter(|&i| self.board.categories[i].name.to_lowercase().contains(&ql))
            .collect();
        let nothing = shown == 0 && sheets.is_empty();
        let mut col = div().flex().flex_col().gap(t.space.lg);
        if nothing {
            col = col.child(
                div()
                    .py(px(60.))
                    .flex()
                    .justify_center()
                    .text_color(t.colors.text_muted)
                    .child(format!("No notes or sheets match “{q}”.")),
            );
        }
        if shown > 0 {
            col = col.child(section_label("NOTES", t)).child(div().flex().flex_col().gap(t.space.sm).children(rows));
        }
        if !sheets.is_empty() {
            col = col.child(section_label("SHEETS", t)).child(
                div()
                    .flex()
                    .flex_wrap()
                    .gap(px(20.))
                    .children(sheets.into_iter().map(|i| self.sheet_card(i, card_w, t, cx).into_any_element())),
            );
        }
        col
    }
}

fn new_sheet_card(w: f32, t: &Theme, cx: &mut Context<Folio>) -> impl IntoElement {
    let inner = w - 20.;
    div()
        .id("new-sheet-card")
        .w(px(w))
        .h(px((inner * 0.66).round() + 20. + 46.))
        .flex()
        .flex_col()
        .items_center()
        .justify_center()
        .gap(t.space.sm)
        .rounded(px(18.))
        .border_2()
        .border_dashed()
        .border_color(t.colors.border)
        .text_color(t.colors.text_subtle)
        .cursor_pointer()
        .hover(|d| d.border_color(t.colors.brand).text_color(t.colors.brand))
        .child(
            div()
                .size(px(52.))
                .rounded(px(16.))
                .bg(t.colors.brand_tint)
                .flex()
                .items_center()
                .justify_center()
                .child(icon("icons/plus.svg", t.size.icon_lg, t.colors.brand)),
        )
        .child("New sheet")
        .on_click(cx.listener(|this, _, window, cx| this.new_sheet_dialog(window, cx)))
}

pub fn section_label(text: &'static str, t: &Theme) -> impl IntoElement {
    div()
        .text_size(px(10.5))
        .font_weight(gpui::FontWeight::BOLD)
        .text_color(t.colors.text_subtle)
        .child(text)
}

/// A note's name for lists: its title, or its first line of text.
pub fn first_line(s: &Sticker) -> String {
    if !s.title.trim().is_empty() {
        return s.title.trim().to_string();
    }
    if let Some(l) = &s.link {
        return if l.title.is_empty() { l.url.clone() } else { l.title.clone() };
    }
    let line = s.body.lines().map(|l| task_line(l).map(|x| x.1).unwrap_or(l)).find(|l| !l.trim().is_empty());
    let line = line.unwrap_or(if s.image.is_some() { "Picture" } else { "Untitled note" });
    let clean: String = line.trim_start_matches(['#', '-', '*', '>', ' ']).chars().take(80).collect();
    clean
}

/// The part of the body around the first match.
/// A few words of a note around where `q` is.
pub fn snippet_of(s: &Sticker, q: &str) -> String {
    snippet(s, q)
}

fn snippet(s: &Sticker, q: &str) -> String {
    let body = s.body.replace('\n', " · ");
    if q.starts_with('#') {
        return body.chars().take(100).collect();
    }
    let lower = body.to_lowercase();
    match lower.find(q) {
        Some(i) => {
            let start = body[..i].char_indices().rev().nth(30).map(|x| x.0).unwrap_or(0);
            let s: String = body[start..].chars().take(110).collect();
            if start > 0 {
                format!("…{s}")
            } else {
                s
            }
        }
        None => body.chars().take(100).collect(),
    }
}

/// A row of options, one highlighted; `on` gets the index clicked.
pub fn segmented(
    id: &'static str,
    items: &[(&'static str, bool)],
    t: &Theme,
    on: impl Fn(&usize, &mut Window, &mut gpui::App) + 'static,
) -> impl IntoElement {
    let on = std::rc::Rc::new(on);
    div()
        .flex()
        .p(px(3.))
        .gap(px(2.))
        .rounded(t.radius.button)
        .bg(t.colors.surface)
        .border_1()
        .border_color(t.colors.border)
        .children(items.iter().enumerate().map(|(i, (label, active))| {
            let on = on.clone();
            div()
                .id((id, i))
                .px(t.space.md)
                .h(px(26.))
                .flex()
                .items_center()
                .rounded(t.radius.small)
                .text_size(t.text.caption)
                .cursor_pointer()
                .when(*active, |d| d.bg(t.colors.text).text_color(t.colors.surface))
                .when(!*active, |d| d.text_color(t.colors.text_muted).hover(|d| d.bg(t.colors.hover)))
                .child(*label)
                .on_click(move |_, window, cx| on(&i, window, cx))
        }))
}

impl Folio {
    pub fn open_menu(&mut self, at: gpui::Point<gpui::Pixels>, kind: MenuKind, cx: &mut Context<Self>) {
        // A card's "…" clicked again closes its menu.
        if matches!(kind, MenuKind::Card(_)) && self.menu.as_ref().is_some_and(|m| m.kind == kind) {
            self.menu = None;
            cx.notify();
            return;
        }
        self.menu = Some(super::app::Menu { at, kind, page: MenuPage::Main });
        cx.notify();
    }
}
