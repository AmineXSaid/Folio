//! Right-click menus: for a note, the empty sheet, a sheet's card, an
//! area, a label, a shape. Sub-pages (Move to, Remind me, Icon, Colour) open in
//! place with a way back.

use gpui::{anchored, canvas, deferred, div, prelude::*, px, Context, MouseButton, SharedString, Stateful, Div, Window};

use super::app::{Folio, MenuKind, MenuPage, Screen};
use super::*;
use crate::model::*;
use crate::theme::{theme, Theme};
use crate::time::{self, now};

fn row(id: impl Into<SharedString>, path: &'static str, label: impl Into<SharedString>, hint: Option<String>, t: &Theme) -> Stateful<Div> {
    div()
        .id(gpui::ElementId::Name(id.into()))
        .h(px(32.))
        .px(t.space.sm)
        .flex()
        .items_center()
        .gap(t.space.md)
        .rounded(t.radius.small)
        .cursor_pointer()
        .text_size(t.text.body)
        .hover(|d| d.bg(t.colors.hover))
        .child(icon(path, t.size.icon_sm, t.colors.text_muted))
        .child(div().flex_1().child(label.into()))
        .children(hint.map(|h| div().text_size(t.text.micro).text_color(t.colors.text_subtle).child(h)))
}

fn sep(t: &Theme) -> Div {
    div().h(px(1.)).my(t.space.xs).bg(t.colors.border)
}

/// A note type to pick: its icon and a short name.
fn style_chip(id: impl Into<gpui::ElementId>, st: NoteStyle, on: bool, t: &Theme) -> Stateful<Div> {
    let path = match st {
        NoteStyle::Card => "icons/note-card.svg",
        NoteStyle::Taped => "icons/note-taped.svg",
        NoteStyle::Cheat => "icons/note-cheat.svg",
        NoteStyle::Index => "icons/note-index.svg",
    };
    let short = match st {
        NoteStyle::Card => "Card",
        NoteStyle::Taped => "Taped",
        NoteStyle::Cheat => "Cheat",
        NoteStyle::Index => "Index",
    };
    div()
        .id(id)
        .h(px(26.))
        .px(px(6.))
        .flex()
        .items_center()
        .gap(px(4.))
        .rounded(t.radius.small)
        .border_1()
        .border_color(if on { t.colors.brand } else { t.colors.border })
        .text_size(t.text.caption)
        .cursor_pointer()
        .hover(|d| d.bg(t.colors.hover))
        .child(icon(path, t.size.icon_sm, t.colors.text_muted))
        .child(short)
}

fn caption(text: &'static str, t: &Theme) -> Div {
    div().px(t.space.sm).pt(t.space.xs).pb(px(2.)).text_size(t.text.micro).text_color(t.colors.text_subtle).child(text)
}

impl Folio {
    /// The open menu, rising into place where it was asked for; a click
    /// anywhere else closes it.
    pub(super) fn render_menu(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let t = theme(cx).clone();
        let Some(menu) = self.menu.as_ref() else { return div().into_any_element() };
        let (at, kind, page) = (menu.at, menu.kind, menu.page);
        let Some(card) = self.menu_card(kind, page, cx) else { return div().into_any_element() };
        div()
            .absolute()
            .inset_0()
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|this, _, _, cx| {
                    this.menu = None;
                    cx.notify();
                }),
            )
            .on_mouse_down(
                MouseButton::Right,
                cx.listener(|this, _, _, cx| {
                    this.menu = None;
                    cx.notify();
                }),
            )
            .child(deferred(anchored().position(at).snap_to_window_with_margin(px(8.)).child(super::motion::rise_in(
                card,
                gpui::ElementId::Name(format!("menu-{at:?}-{kind:?}").into()),
                &t,
            ))))
            .into_any_element()
    }

    /// A menu that just closed, fading where it was.
    pub(super) fn render_menu_leaving(&mut self, cx: &mut Context<Self>) -> Option<gpui::AnyElement> {
        let t = theme(cx).clone();
        let ((at, kind, page), n) = self.exits.menu.leaving(t.motion.pop_out)?;
        let card = self.menu_card(kind, page, cx)?;
        Some(
            deferred(anchored().position(at).snap_to_window_with_margin(px(8.)).child(super::motion::leave(
                card.relative(),
                ("menu-out", n),
                t.motion.pop_out,
                t.motion.sink,
            )))
            .into_any_element(),
        )
    }

    /// A menu's card: its rows for `kind` on `page`.
    fn menu_card(&mut self, kind: MenuKind, page: MenuPage, cx: &mut Context<Self>) -> Option<gpui::Stateful<Div>> {
        let t = theme(cx).clone();
        let back = |label: &'static str, cx: &mut Context<Self>| {
            row("back", "icons/chevron-left.svg", label, None, &t).on_click(cx.listener(|this, _, _, cx| {
                if let Some(m) = this.menu.as_mut() {
                    m.page = MenuPage::Main;
                }
                cx.notify();
            }))
        };
        let goto = move |p: MenuPage| {
            move |this: &mut Folio, _: &gpui::ClickEvent, _: &mut Window, cx: &mut Context<Folio>| {
                if let Some(m) = this.menu.as_mut() {
                    m.page = p;
                }
                cx.notify();
            }
        };
        let mut items: Vec<gpui::AnyElement> = Vec::new();
        match (kind, page) {
            (MenuKind::Note(id), MenuPage::Main) => {
                let Some(s) = self.board.find_note(id).cloned() else { return None };
                // Everything here applies to every selected note when this
                // one is among them; what only makes sense for one note
                // isn't offered for several.
                let targets = self.targets(id);
                let many = targets.len() > 1;
                if many {
                    items.push(
                        div()
                            .px(t.space.sm)
                            .pt(t.space.xs)
                            .text_size(t.text.caption)
                            .font_weight(gpui::FontWeight::SEMIBOLD)
                            .text_color(t.colors.brand_on_tint)
                            .child(format!("{} notes selected", targets.len()))
                            .into_any_element(),
                    );
                }
                // Twelve papers as small circles: the same width six squares took.
                let papers = div().flex().items_center().gap(px(3.)).px(t.space.sm).py(t.space.xs).children(Paper::ALL.iter().map(|&p| {
                    div()
                        .id(("m-paper", p as usize))
                        .size(px(15.))
                        .rounded(t.radius.full)
                        .hover(|d| d.border_color(t.colors.text_subtle))
                        .bg(t.paper(p))
                        .border_1()
                        .border_color(t.colors.border)
                        .cursor_pointer()
                        .when(s.paper == p, |d| d.border_2().border_color(t.colors.brand))
                        .on_click({
                            let targets = targets.clone();
                            cx.listener(move |this, _, _, cx| this.change_notes(&targets, cx, |s| s.paper = p))
                        })
                }));
                let fonts = div().flex().gap(px(4.)).px(t.space.sm).pb(t.space.xs).children(NoteFont::ALL.iter().map(|&f| {
                    let on = s.font == f;
                    div()
                        .id(("m-font", f as usize))
                        .px(t.space.sm)
                        .h(px(26.))
                        .flex()
                        .items_center()
                        .rounded(t.radius.small)
                        .border_1()
                        .border_color(if on { t.colors.brand } else { t.colors.border })
                        .font_family(t.note_font(f))
                        .text_size(t.text.caption)
                        .cursor_pointer()
                        .child(f.label())
                        .on_click({
                            let targets = targets.clone();
                            cx.listener(move |this, _, _, cx| this.change_notes(&targets, cx, |s| s.font = f))
                        })
                }).chain(TextSize::ALL.iter().map(|&z| {
                    let on = s.text_size == z;
                    div()
                        .id(("m-size", z as usize))
                        .w(px(26.))
                        .h(px(26.))
                        .flex()
                        .items_center()
                        .justify_center()
                        .rounded(t.radius.small)
                        .border_1()
                        .border_color(if on { t.colors.brand } else { t.colors.border })
                        .text_size(t.text.caption)
                        .cursor_pointer()
                        .child(z.label())
                        .on_click({
                            let targets = targets.clone();
                            cx.listener(move |this, _, _, cx| this.change_notes(&targets, cx, |s| s.text_size = z))
                        })
                })));
                // The note's type.
                let styles = div().flex().gap(px(4.)).px(t.space.sm).pb(t.space.xs).children(NoteStyle::ALL.iter().map(|&st| {
                    let on = s.style == st;
                    let targets = targets.clone();
                    style_chip(("m-style", st as usize), st, on, &t)
                        .on_click(cx.listener(move |this, _, _, cx| this.set_style(&targets, st, cx)))
                }));
                items.push(papers.into_any_element());
                items.push(fonts.into_any_element());
                items.push(styles.into_any_element());
                items.push(sep(&t).into_any_element());
                if !many {
                    items.push(
                        row("m-edit", "icons/pencil.svg", "Edit", Some("Enter".into()), &t)
                            .on_click(cx.listener(move |this, _, window, cx| this.begin_edit(id, false, window, cx)))
                            .into_any_element(),
                    );
                    items.push(
                        row("m-task", "icons/list-checks.svg", "Add a task", None, &t)
                            .on_click(cx.listener(move |this, _, window, cx| {
                                this.menu = None;
                                this.add_task(id, window, cx)
                            }))
                            .into_any_element(),
                    );
                }
                items.push(
                    row("m-markdown", "icons/wand-sparkles.svg", "Turn into Markdown", Some(shortcut("Shift+T")), &t)
                        .on_click({
                            let targets = targets.clone();
                            cx.listener(move |this, _, _, cx| {
                                this.menu = None;
                                this.markdown_notes(&targets, cx)
                            })
                        })
                        .into_any_element(),
                );
                let pinned = s.pin.map_or("None", |p| p.shape.label());
                items.push(row("m-pin", "icons/pin.svg", "Pin", Some(format!("{pinned}  ›")), &t).on_click(cx.listener(goto(MenuPage::Pin))).into_any_element());
                if !many {
                    items.push(
                        row("m-pic", "icons/image.svg", if s.image.is_some() { "Replace picture…" } else { "Add picture…" }, None, &t)
                            .on_click(cx.listener(move |this, _, _, cx| this.pick_image(id, cx)))
                            .into_any_element(),
                    );
                }
                let remind = match s.due {
                    Some(d) => format!("Reminder · {}", time::due_label(d, now())),
                    None => "Remind me…".into(),
                };
                items.push(row("m-remind", "icons/bell.svg", remind, Some("›".into()), &t).on_click(cx.listener(goto(MenuPage::Remind))).into_any_element());
                if let Some(l) = s.link.as_ref().filter(|_| !many) {
                    let url = l.url.clone();
                    items.push(
                        row("m-open", "icons/external-link.svg", "Open link", None, &t)
                            .on_click(cx.listener(move |this, _, _, cx| {
                                this.menu = None;
                                cx.open_url(&url);
                            }))
                            .into_any_element(),
                    );
                }
                items.push(sep(&t).into_any_element());
                items.extend((!many).then(|| if s.popped.is_some() {
                    row("m-dock", "icons/minimize-2.svg", "Take back from the desktop", None, &t)
                        .on_click(cx.listener(move |this, _, _, cx| {
                            this.menu = None;
                            this.dock(id, cx)
                        }))
                        .into_any_element()
                } else {
                    row("m-pop", "icons/picture-in-picture-2.svg", "Pin to the desktop", None, &t)
                        .on_click(cx.listener(move |this, _, window, cx| this.pop_out(id, window, cx)))
                        .into_any_element()
                }));
                items.push(row("m-move", "icons/folder-input.svg", "Move to sheet", Some("›".into()), &t).on_click(cx.listener(goto(MenuPage::MoveTo))).into_any_element());
                items.push(
                    row("m-link", "icons/link.svg", "Link to a note…", None, &t)
                        .on_click(cx.listener(move |this, _, window, cx| {
                            this.menu = None;
                            this.start_link(id, window, cx)
                        }))
                        .into_any_element(),
                );
                let back_n = self.backlinks().get(&id).map_or(0, |v| v.len());
                if back_n > 0 {
                    items.push(row("m-backlinks", "icons/link.svg", "Linked from", Some(format!("{back_n}  ›")), &t).on_click(cx.listener(goto(MenuPage::Backlinks))).into_any_element());
                }
                items.push(
                    row("m-dup", "icons/copy.svg", "Duplicate", Some(shortcut("D")), &t)
                        .on_click(cx.listener(move |this, _, _, cx| {
                            this.menu = None;
                            if !this.is_selected(super::app::Selection::Sticker(id)) {
                                this.select(super::app::Selection::Sticker(id));
                            }
                            this.duplicate_selection(cx)
                        }))
                        .into_any_element(),
                );
                // The note goes on the clipboard whole (its text for other
                // apps); a right-click picked it if it wasn't selected.
                items.push(
                    row("m-copy", "icons/file-text.svg", "Copy", Some(shortcut("C")), &t)
                        .on_click(cx.listener(|this, _, window, cx| this.copy_selection(window, cx)))
                        .into_any_element(),
                );
                items.push(
                    row("m-cut", "icons/scissors.svg", "Cut", Some(shortcut("X")), &t)
                        .on_click(cx.listener(|this, _, _, cx| this.cut_selection(cx)))
                        .into_any_element(),
                );
                // Lock unless they're all locked already.
                let locked = targets.iter().all(|id| self.board.find_note(*id).is_some_and(|s| s.locked));
                items.push(
                    row("m-lock", if locked { "icons/lock-open.svg" } else { "icons/lock.svg" }, if locked { "Unlock" } else { "Lock" }, None, &t)
                        .on_click({
                            let targets = targets.clone();
                            cx.listener(move |this, _, _, cx| this.lock_notes(&targets, cx))
                        })
                        .into_any_element(),
                );
                let fixed = targets.iter().all(|id| self.board.find_note(*id).is_some_and(|s| s.fixed));
                items.push(
                    row("m-fit", if fixed { "icons/maximize-2.svg" } else { "icons/minimize-2.svg" }, if fixed { "Grow with the text" } else { "Keep this height" }, None, &t)
                        .on_click({
                            let targets = targets.clone();
                            cx.listener(move |this, _, _, cx| {
                                this.menu = None;
                                // Each keeps the height it's drawn at now.
                                let heights: std::collections::HashMap<Id, f32> =
                                    targets.iter().filter_map(|id| this.board.find_note(*id).map(|s| (*id, this.note_rect(s).h))).collect();
                                this.change_notes(&targets, cx, |s| {
                                    s.fixed = !fixed;
                                    if let (false, Some(h)) = (fixed, heights.get(&s.id)) {
                                        s.rect.h = *h;
                                    }
                                })
                            })
                        })
                        .into_any_element(),
                );
                if !many && s.is_stale(now()) {
                    items.push(
                        row("m-fresh", "icons/sparkles.svg", "Still relevant (keep fresh)", None, &t)
                            .on_click(cx.listener(move |this, _, _, cx| {
                                this.menu = None;
                                this.change_note(id, cx, |_| {})
                            }))
                            .into_any_element(),
                    );
                    items.push(
                        row("m-snooze", "icons/clock.svg", "Snooze for a week", None, &t)
                            .on_click(cx.listener(move |this, _, _, cx| {
                                this.menu = None;
                                let until = now() + 7 * 86_400;
                                this.change_note(id, cx, |s| s.snoozed = until);
                                this.notify("Snoozed for a week", cx);
                            }))
                            .into_any_element(),
                    );
                }
                items.push(sep(&t).into_any_element());
                items.push(
                    row("m-archive", "icons/archive.svg", "Archive", None, &t)
                        .on_click({
                            let targets = targets.clone();
                            cx.listener(move |this, _, _, cx| this.bin_notes(&targets, true, cx))
                        })
                        .into_any_element(),
                );
                items.push(
                    row("m-trash", "icons/trash.svg", "Move to Trash", Some("Del".into()), &t)
                        .text_color(t.colors.danger)
                        .on_click(cx.listener(move |this, _, _, cx| this.bin_notes(&targets, false, cx)))
                        .into_any_element(),
                );
            }
            (MenuKind::Note(id), MenuPage::Pin) => {
                let Some(s) = self.board.find_note(id).cloned() else { return None };
                let targets = self.targets(id);
                let current = s.pin;
                items.push(back("Pin", cx).into_any_element());
                items.push(sep(&t).into_any_element());
                for (family, shapes) in PinShape::FAMILIES {
                    items.push(caption(family, &t).into_any_element());
                    items.push(
                        div()
                            .flex()
                            .flex_wrap()
                            .gap(px(4.))
                            .px(t.space.sm)
                            .pb(t.space.xs)
                            .children(shapes.iter().map(|&shape| {
                                let on = current.is_some_and(|p| p.shape == shape);
                                // Shown in the colour it would take.
                                let tone = match current {
                                    Some(p) if p.shape.tone().is_some() && shape.tone().is_some() => p.tone,
                                    _ => shape.tone().unwrap_or_default(),
                                };
                                let targets = targets.clone();
                                div()
                                    .id(("m-pin", shape as usize))
                                    .size(px(46.))
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .rounded(t.radius.button)
                                    .border_1()
                                    .border_color(if on { t.colors.brand } else { gpui::transparent_black() })
                                    .when(on, |d| d.bg(t.colors.brand_tint))
                                    .when(!on, |d| d.hover(|d| d.bg(t.colors.hover)))
                                    .cursor_pointer()
                                    .child(super::pins::pin_icon(shape, tone, 34., &t))
                                    .tooltip(move |window, cx| super::tip(shape.label(), window, cx))
                                    .on_click(cx.listener(move |this, _, _, cx| {
                                        this.set_pin(&targets, cx, |p| {
                                            Some(match p {
                                                Some(p) => p.with_shape(shape, true),
                                                None => Pin::default().with_shape(shape, false),
                                            })
                                        })
                                    }))
                            }))
                            .into_any_element(),
                    );
                }
                if let Some(p) = current {
                    items.push(sep(&t).into_any_element());
                    let sizes = div().flex().items_center().gap(px(4.)).px(t.space.sm).pb(t.space.xs).child(div().w(px(52.)).text_size(t.text.caption).text_color(t.colors.text_muted).child("Size")).children(PinSize::ALL.iter().map(|&z| {
                        let on = p.size == z;
                        let targets = targets.clone();
                        div()
                            .id(("m-pin-size", z as usize))
                            .size(px(26.))
                            .flex()
                            .items_center()
                            .justify_center()
                            .rounded(t.radius.small)
                            .border_1()
                            .border_color(if on { t.colors.brand } else { t.colors.border })
                            .text_size(t.text.caption)
                            .cursor_pointer()
                            .child(z.label())
                            .on_click(cx.listener(move |this, _, _, cx| this.set_pin(&targets, cx, |p| p.map(|p| Pin { size: z, ..p }))))
                    }));
                    items.push(sizes.into_any_element());
                    if p.shape.tone().is_some() {
                        let tones = div().flex().items_center().gap(px(5.)).px(t.space.sm).pb(t.space.xs).child(div().w(px(52.)).text_size(t.text.caption).text_color(t.colors.text_muted).child("Colour")).children(PinTone::ALL.iter().map(|&tone| {
                            let on = p.tone == tone;
                            let targets = targets.clone();
                            div()
                                .id(("m-pin-tone", tone as usize))
                                .size(px(20.))
                                .rounded(t.radius.full)
                                .bg(t.pin_swatch(tone))
                                .border_1()
                                .border_color(t.colors.border)
                                .when(on, |d| d.border_2().border_color(t.colors.brand))
                                .cursor_pointer()
                                .tooltip(move |window, cx| super::tip(tone.label(), window, cx))
                                .on_click(cx.listener(move |this, _, _, cx| this.set_pin(&targets, cx, |p| p.map(|p| Pin { tone, ..p }))))
                        }));
                        items.push(tones.into_any_element());
                    }
                    let targets = targets.clone();
                    items.push(
                        row("m-nopin", "icons/x.svg", "No pin", None, &t)
                            .on_click(cx.listener(move |this, _, _, cx| this.set_pin(&targets, cx, |_| None)))
                            .into_any_element(),
                    );
                }
            }
            (MenuKind::Note(id), MenuPage::MoveTo) => {
                items.push(back("Move to sheet", cx).into_any_element());
                items.push(sep(&t).into_any_element());
                let here = self.board.sheet_of(id);
                let targets = self.targets(id);
                for ix in self.board.ordered() {
                    if Some(ix) == here {
                        continue;
                    }
                    let c = &self.board.categories[ix];
                    let label = if c.icon.is_empty() { c.name.clone() } else { format!("{} {}", c.icon, c.name) };
                    items.push(
                        row(format!("m-to-{ix}"), "icons/folder-input.svg", label, None, &t)
                            .on_click({
                                let targets = targets.clone();
                                cx.listener(move |this, _, _, cx| this.move_notes_to(&targets, ix, cx))
                            })
                            .into_any_element(),
                    );
                }
            }
            (MenuKind::Note(id), MenuPage::Backlinks) => {
                items.push(back("Linked from", cx).into_any_element());
                items.push(sep(&t).into_any_element());
                let from = self.backlinks().get(&id).cloned().unwrap_or_default();
                for (i, other) in from.into_iter().enumerate() {
                    let Some(s) = self.board.find_note(other) else { continue };
                    let sheet = self.board.sheet_of(other).map(|ix| self.board.categories[ix].name.clone()).unwrap_or_default();
                    items.push(
                        row(format!("m-from-{i}"), "icons/sticky-note.svg", s.name(), Some(sheet), &t)
                            .on_click(cx.listener(move |this, _, window, cx| {
                                this.menu = None;
                                this.reveal_note(other, window, cx)
                            }))
                            .into_any_element(),
                    );
                }
            }
            (MenuKind::Note(id), MenuPage::Remind) => {
                let targets = self.targets(id);
                items.push(back("Remind me", cx).into_any_element());
                items.push(sep(&t).into_any_element());
                for (i, (label, when)) in time::presets(now()).into_iter().enumerate() {
                    items.push(
                        row(format!("m-at-{i}"), "icons/clock.svg", label, None, &t)
                            .on_click({
                                let targets = targets.clone();
                                cx.listener(move |this, _, _, cx| this.set_reminders(&targets, Some(when), cx))
                            })
                            .into_any_element(),
                    );
                }
                if targets.iter().any(|id| self.board.find_note(*id).is_some_and(|s| s.due.is_some())) {
                    items.push(sep(&t).into_any_element());
                    items.push(
                        row("m-clear", "icons/bell-off.svg", "Clear reminder", None, &t)
                            .on_click(cx.listener(move |this, _, _, cx| this.set_reminders(&targets, None, cx)))
                            .into_any_element(),
                    );
                }
            }
            (MenuKind::Canvas(_), MenuPage::Background) => {
                let bd = self.sheet().backdrop.clone();
                items.push(back("Background", cx).into_any_element());
                items.push(sep(&t).into_any_element());
                items.push(caption("Pattern", &t).into_any_element());
                // A sample of each pattern, on this sheet's colour.
                let paper = t.tint(bd.tint);
                let (dot, line) = (t.colors.canvas_dot, t.colors.canvas_line);
                items.push(
                    div()
                        .flex()
                        .flex_wrap()
                        .gap(px(6.))
                        .px(t.space.sm)
                        .pb(t.space.sm)
                        .children(Pattern::ALL.iter().map(|&p| {
                            let on = bd.pattern == p;
                            let sample = canvas(
                                |_, _, _| {},
                                move |b, _, window, _| super::board::paint_pattern(b, Camera { pan: v2(5., 5.), zoom: 1.75 }, 8., p, dot, line, window),
                            )
                            .absolute()
                            .inset_0();
                            div()
                                .id(("m-pattern", p as usize))
                                .flex()
                                .flex_col()
                                .items_center()
                                .gap(px(3.))
                                .cursor_pointer()
                                .child(
                                    div()
                                        .relative()
                                        .w(px(70.))
                                        .h(px(46.))
                                        .rounded(t.radius.small)
                                        .overflow_hidden()
                                        .bg(paper)
                                        .border_1()
                                        .border_color(t.colors.border)
                                        .when(on, |d| d.border_2().border_color(t.colors.brand))
                                        .hover(|d| d.border_color(t.colors.text_subtle))
                                        .child(sample),
                                )
                                .child(div().text_size(t.text.micro).text_color(if on { t.colors.brand_on_tint } else { t.colors.text_muted }).child(p.label()))
                                .on_click(cx.listener(move |this, _, _, cx| this.set_backdrop(cx, |b| b.pattern = p)))
                        }))
                        .into_any_element(),
                );
                items.push(caption("Colour", &t).into_any_element());
                items.push(
                    div()
                        .flex()
                        .gap(px(6.))
                        .px(t.space.sm)
                        .pb(t.space.sm)
                        .children(Tint::ALL.iter().map(|&tint| {
                            let on = bd.tint == tint;
                            div()
                                .id(("m-tint", tint as usize))
                                .size(px(24.))
                                .rounded(t.radius.full)
                                .bg(t.tint(tint))
                                .border_1()
                                .border_color(t.colors.border)
                                .when(on, |d| d.border_2().border_color(t.colors.brand))
                                .cursor_pointer()
                                .hover(|d| d.border_color(t.colors.text_subtle))
                                .tooltip(move |window, cx| super::tip(tint.label(), window, cx))
                                .on_click(cx.listener(move |this, _, _, cx| this.set_backdrop(cx, |b| b.tint = tint)))
                        }))
                        .into_any_element(),
                );
                items.push(sep(&t).into_any_element());
                let has_picture = bd.image.is_some();
                items.push(
                    row("m-bg-pic", "icons/image.svg", if has_picture { "Another picture…" } else { "Picture or SVG from your computer…" }, None, &t)
                        .on_click(cx.listener(|this, _, _, cx| this.pick_backdrop(cx)))
                        .into_any_element(),
                );
                if has_picture {
                    let soft = bd.soften;
                    items.push(
                        row("m-bg-soften", if soft { "icons/eye-off.svg" } else { "icons/eye.svg" }, "Soften the picture", Some(if soft { "On" } else { "Off" }.into()), &t)
                            .on_click(cx.listener(move |this, _, _, cx| this.set_backdrop(cx, |b| b.soften = !soft)))
                            .into_any_element(),
                    );
                    items.push(
                        row("m-bg-nopic", "icons/x.svg", "Remove the picture", None, &t)
                            .on_click(cx.listener(|this, _, _, cx| this.set_backdrop(cx, |b| b.image = None)))
                            .into_any_element(),
                    );
                }
                items.push(sep(&t).into_any_element());
                items.push(
                    row("m-bg-all", "icons/layers.svg", "Use on every sheet", None, &t)
                        .on_click(cx.listener(|this, _, _, cx| this.backdrop_everywhere(cx)))
                        .into_any_element(),
                );
            }
            (MenuKind::Canvas(at_w), _) => {
                items.push(
                    row("m-new", "icons/sticky-note.svg", "New note here", Some("Double-click".into()), &t)
                        .on_click(cx.listener(move |this, _, window, cx| {
                            this.menu = None;
                            this.new_note_at(at_w, window, cx)
                        }))
                        .into_any_element(),
                );
                // Or a note of a given type, straight away.
                items.push(
                    div()
                        .flex()
                        .gap(px(4.))
                        .px(t.space.sm)
                        .pb(t.space.xs)
                        .children(NoteStyle::ALL.iter().map(|&st| {
                            style_chip(("m-new-style", st as usize), st, false, &t).on_click(cx.listener(move |this, _, window, cx| {
                                this.menu = None;
                                this.new_note_styled(at_w, st, window, cx)
                            }))
                        }))
                        .into_any_element(),
                );
                items.push(
                    row("m-text", "icons/type.svg", "Write on the sheet here", None, &t)
                        .on_click(cx.listener(move |this, _, window, cx| {
                            this.menu = None;
                            this.new_label_at(at_w, window, cx)
                        }))
                        .into_any_element(),
                );
                items.push(
                    row("m-area", "icons/square-dashed.svg", "New area here", None, &t)
                        .on_click(cx.listener(move |this, _, _, cx| {
                            this.menu = None;
                            this.new_frame(Rect { x: at_w.x, y: at_w.y, w: 420., h: 320. }, cx)
                        }))
                        .into_any_element(),
                );
                items.push(
                    row("m-paste", "icons/clipboard-paste.svg", "Paste", Some(shortcut("V")), &t)
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.menu = None;
                            this.paste(false, cx)
                        }))
                        .into_any_element(),
                );
                items.push(
                    row("m-paste-md", "icons/wand-sparkles.svg", "Paste as Markdown", Some(shortcut("Shift+V")), &t)
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.menu = None;
                            this.paste(true, cx)
                        }))
                        .into_any_element(),
                );
                items.push(row("m-bg", "icons/palette.svg", "Background", Some("›".into()), &t).on_click(cx.listener(goto(MenuPage::Background))).into_any_element());
                items.push(sep(&t).into_any_element());
                items.push(
                    row("m-tidy", "icons/wand-sparkles.svg", "Tidy up", None, &t)
                        .on_click(cx.listener(|this, _, _, cx| this.tidy(cx)))
                        .into_any_element(),
                );
                items.push(
                    row("m-fit-all", "icons/scan.svg", "Zoom to fit", Some(shortcut("0")), &t)
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.menu = None;
                            this.zoom_to_fit(cx)
                        }))
                        .into_any_element(),
                );
                let mm = self.board.settings.minimap;
                items.push(
                    row("m-map", "icons/map.svg", if mm { "Hide the map" } else { "Show the map" }, None, &t)
                        .on_click(cx.listener(move |this, _, _, cx| {
                            this.menu = None;
                            this.set_setting(cx, |s| s.minimap = !mm)
                        }))
                        .into_any_element(),
                );
                // The sheet, out to share.
                items.push(sep(&t).into_any_element());
                let ix = self.board.active;
                items.push(
                    row("m-png", "icons/image.svg", "Save as PNG…", None, &t)
                        .on_click(cx.listener(move |this, _, _, cx| this.save_sheet_as(ix, crate::picture::Format::Png, cx)))
                        .into_any_element(),
                );
                items.push(
                    row("m-pdf", "icons/file-output.svg", "Save as PDF…", None, &t)
                        .on_click(cx.listener(move |this, _, _, cx| this.save_sheet_as(ix, crate::picture::Format::Pdf, cx)))
                        .into_any_element(),
                );
            }
            (MenuKind::Card(ix), MenuPage::Main) => {
                let Some(c) = self.board.categories.get(ix) else { return None };
                let pinned = c.pinned;
                items.push(
                    row("m-open-sheet", "icons/arrow-up-right.svg", "Open", None, &t)
                        .on_click(cx.listener(move |this, _, window, cx| {
                            this.menu = None;
                            this.open_sheet(ix, window, cx)
                        }))
                        .into_any_element(),
                );
                items.push(
                    row("m-rename", "icons/pencil.svg", "Rename…", None, &t)
                        .on_click(cx.listener(move |this, _, window, cx| this.rename_sheet_dialog(ix, window, cx)))
                        .into_any_element(),
                );
                items.push(
                    row("m-pin", if pinned { "icons/pin-off.svg" } else { "icons/pin.svg" }, if pinned { "Unpin" } else { "Pin to the top" }, None, &t)
                        .on_click(cx.listener(move |this, _, _, cx| this.toggle_pin_sheet(ix, cx)))
                        .into_any_element(),
                );
                items.push(row("m-icon", "icons/face-slightly-smiling.svg", "Change emoji", Some("›".into()), &t).on_click(cx.listener(goto(MenuPage::Icon))).into_any_element());
                items.push(
                    row("m-dup-sheet", "icons/copy.svg", "Duplicate", None, &t)
                        .on_click(cx.listener(move |this, _, _, cx| this.duplicate_sheet(ix, cx)))
                        .into_any_element(),
                );
                items.push(
                    row("m-card-png", "icons/image.svg", "Save as PNG…", None, &t)
                        .on_click(cx.listener(move |this, _, _, cx| this.save_sheet_as(ix, crate::picture::Format::Png, cx)))
                        .into_any_element(),
                );
                items.push(
                    row("m-card-pdf", "icons/file-output.svg", "Save as PDF…", None, &t)
                        .on_click(cx.listener(move |this, _, _, cx| this.save_sheet_as(ix, crate::picture::Format::Pdf, cx)))
                        .into_any_element(),
                );
                items.push(
                    row("m-export-sheet", "icons/file-down.svg", "Export…", None, &t)
                        .on_click(cx.listener(move |this, _, _, cx| this.export_sheet(ix, cx)))
                        .into_any_element(),
                );
                items.push(sep(&t).into_any_element());
                items.push(
                    row("m-arch-sheet", "icons/archive.svg", "Archive", None, &t)
                        .on_click(cx.listener(move |this, _, _, cx| this.bin_sheet(ix, true, cx)))
                        .into_any_element(),
                );
                items.push(
                    row("m-trash-sheet", "icons/trash.svg", "Move to Trash", None, &t)
                        .text_color(t.colors.danger)
                        .on_click(cx.listener(move |this, _, _, cx| this.bin_sheet(ix, false, cx)))
                        .into_any_element(),
                );
            }
            (MenuKind::Card(ix), MenuPage::Icon) => {
                items.push(back("Change emoji", cx).into_any_element());
                items.push(sep(&t).into_any_element());
                let current = self.board.categories.get(ix).map(|c| c.icon.clone()).unwrap_or_default();
                let picker = div().w(px(340.)).p(t.space.xs).child(self.emoji_picker(&current, &t, cx, move |this, e, cx| {
                    this.menu = None;
                    this.change(cx, |b| {
                        if let Some(c) = b.categories.get_mut(ix) {
                            c.icon = e.into();
                        }
                    });
                }));
                items.push(picker.into_any_element());
            }
            (MenuKind::Frame(_), MenuPage::Colour) => {
                let (frame, ix) = match kind {
                    MenuKind::Frame(id) => (true, id as usize),
                    MenuKind::Card(ix) => (false, ix),
                    _ => (false, 0),
                };
                items.push(back("Colour", cx).into_any_element());
                items.push(sep(&t).into_any_element());
                items.push(
                    div()
                        .flex()
                        .gap(px(6.))
                        .p(t.space.sm)
                        .children(Accent::ALL.iter().map(|&a| {
                            div()
                                .id(("m-accent", a as usize))
                                .size(px(26.))
                                .rounded(t.radius.small)
                                .bg(t.accent(a))
                                .cursor_pointer()
                                .on_click(cx.listener(move |this, _, _, cx| {
                                    this.menu = None;
                                    this.change(cx, |b| {
                                        if frame {
                                            let id = ix as Id;
                                            if let Some(f) = b.sheet_mut().frames.iter_mut().find(|f| f.id == id) {
                                                f.accent = a;
                                            }
                                        } else if let Some(c) = b.categories.get_mut(ix) {
                                            c.accent = a;
                                        }
                                    });
                                }))
                        }))
                        .into_any_element(),
                );
            }
            (MenuKind::Frame(id), _) => {
                items.push(
                    row("m-frame-name", "icons/pencil.svg", "Rename…", None, &t)
                        .on_click(cx.listener(move |this, _, window, cx| this.rename_frame(id, window, cx)))
                        .into_any_element(),
                );
                items.push(row("m-frame-colour", "icons/palette.svg", "Colour", Some("›".into()), &t).on_click(cx.listener(goto(MenuPage::Colour))).into_any_element());
                items.push(
                    row("m-frame-tidy", "icons/wand-sparkles.svg", "Tidy the notes inside", None, &t)
                        .on_click(cx.listener(move |this, _, _, cx| {
                            this.selection = this.sheet().frame_members(id).into_iter().map(super::app::Selection::Sticker).collect();
                            this.tidy(cx)
                        }))
                        .into_any_element(),
                );
                items.push(sep(&t).into_any_element());
                items.push(
                    row("m-frame-del", "icons/trash.svg", "Remove the area (keeps the notes)", None, &t)
                        .text_color(t.colors.danger)
                        .on_click(cx.listener(move |this, _, _, cx| {
                            this.menu = None;
                            this.change(cx, |b| b.sheet_mut().frames.retain(|f| f.id != id));
                        }))
                        .into_any_element(),
                );
            }
            (MenuKind::Label(id), _) => {
                items.push(
                    row("m-label-edit", "icons/pencil.svg", "Edit…", None, &t)
                        .on_click(cx.listener(move |this, _, window, cx| this.edit_label(id, false, window, cx)))
                        .into_any_element(),
                );
                items.push(
                    div()
                        .flex()
                        .flex_wrap()
                        .gap(px(5.))
                        .p(t.space.sm)
                        .max_w(px(240.))
                        .children(Ink::ALL.iter().map(|&ink| {
                            div()
                                .id(("m-ink", ink as usize))
                                .size(px(18.))
                                .hover(|d| d.border_2().border_color(t.colors.surface))
                                .rounded(t.radius.full)
                                .bg(t.ink(ink))
                                .cursor_pointer()
                                .on_click(cx.listener(move |this, _, _, cx| {
                                    this.menu = None;
                                    this.change(cx, |b| {
                                        if let Some(l) = b.sheet_mut().labels.iter_mut().find(|l| l.id == id) {
                                            l.ink = ink;
                                        }
                                    });
                                }))
                        }))
                        .into_any_element(),
                );
                // Its size and typeface, as in the bar while writing.
                let (size, font, slanted) = self
                    .sheet()
                    .labels
                    .iter()
                    .find(|l| l.id == id)
                    .map(|l| (l.size, l.font, l.italic))
                    .unwrap_or_default();
                let chip = |id: gpui::ElementId, on: bool| {
                    div()
                        .id(id)
                        .h(px(24.))
                        .min_w(px(26.))
                        .px(px(6.))
                        .flex()
                        .items_center()
                        .justify_center()
                        .rounded(t.radius.small)
                        .text_size(t.text.caption)
                        .cursor_pointer()
                        .when(on, |d| d.bg(t.colors.brand_tint).text_color(t.colors.brand_on_tint))
                        .when(!on, |d| d.text_color(t.colors.text_muted).hover(|d| d.bg(t.colors.hover)))
                };
                items.push(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(2.))
                        .px(t.space.sm)
                        .py(px(2.))
                        .children(LabelSize::ALL.iter().map(|&z| {
                            chip(("m-label-size", z as usize).into(), size == z)
                                .child(z.label())
                                .on_click(cx.listener(move |this, _, _, cx| this.set_label(id, cx, |l| l.size = z)))
                        }))
                        .child(div().w(px(1.)).h(px(16.)).mx(px(4.)).bg(t.colors.border))
                        .children(NoteFont::ALL.iter().map(|&f| {
                            chip(("m-label-font", f as usize).into(), font == f)
                                .font_family(t.note_font(f))
                                .child("Aa")
                                .on_click(cx.listener(move |this, _, _, cx| this.set_label(id, cx, |l| l.font = f)))
                        }))
                        .child(
                            chip("m-label-slant".into(), slanted)
                                .italic()
                                .font_weight(gpui::FontWeight::SEMIBOLD)
                                .child("I")
                                .on_click(cx.listener(move |this, _, _, cx| this.set_label(id, cx, |l| l.italic = !l.italic))),
                        )
                        .into_any_element(),
                );
                items.push(
                    row("m-label-del", "icons/trash.svg", "Delete", None, &t)
                        .text_color(t.colors.danger)
                        .on_click(cx.listener(move |this, _, _, cx| {
                            this.menu = None;
                            this.change(cx, |b| b.sheet_mut().labels.retain(|l| l.id != id));
                        }))
                        .into_any_element(),
                );
            }
            (MenuKind::Shape(id), _) => {
                let Some(shape) = self.sheet().shapes.iter().find(|s| s.id == id).cloned() else { return None };
                let ids = self.shape_targets(id);
                items.push(
                    row("m-shape-text", "icons/type.svg", if shape.text.is_empty() { "Add words…" } else { "Edit words…" }, Some("Enter".into()), &t)
                        .on_click(cx.listener(move |this, _, window, cx| this.edit_shape_text(id, window, cx)))
                        .into_any_element(),
                );
                let swatch = |id: gpui::ElementId, color: gpui::Hsla, on: bool, round: bool| {
                    div()
                        .id(id)
                        .size(px(20.))
                        .rounded(if round { t.radius.full } else { px(5.) })
                        .bg(color)
                        .border_1()
                        .border_color(if on { t.colors.brand } else { t.colors.border })
                        .when(on, |d| d.border_2())
                        .cursor_pointer()
                };
                if !shape.geom.is_line() {
                    items.push(caption("Fill", &t).into_any_element());
                    let none = {
                        let ids = ids.clone();
                        swatch("m-fill-none".into(), t.colors.surface, shape.fill.is_none(), false)
                            .flex()
                            .items_center()
                            .justify_center()
                            .child(icon("icons/x.svg", px(12.), t.colors.text_subtle))
                            .on_click(cx.listener(move |this, _, _, cx| this.change_shapes(&ids, cx, |s| s.fill = None)))
                    };
                    items.push(
                        div()
                            .flex()
                            .flex_wrap()
                            .gap(px(5.))
                            .px(t.space.sm)
                            .py(px(4.))
                            .max_w(px(260.))
                            .child(none)
                            .children(Paper::ALL.iter().map(|&p| {
                                let ids = ids.clone();
                                swatch(("m-fill", p as usize).into(), t.paper(p), shape.fill == Some(p), false)
                                    .on_click(cx.listener(move |this, _, _, cx| this.change_shapes(&ids, cx, |s| s.fill = Some(p))))
                            }))
                            .into_any_element(),
                    );
                }
                items.push(caption("Line", &t).into_any_element());
                items.push(
                    div()
                        .flex()
                        .flex_wrap()
                        .gap(px(5.))
                        .px(t.space.sm)
                        .py(px(4.))
                        .max_w(px(260.))
                        .children(Ink::ALL.iter().map(|&ink| {
                            let ids = ids.clone();
                            swatch(("m-shape-ink", ink as usize).into(), t.ink(ink), shape.ink == ink, true)
                                .on_click(cx.listener(move |this, _, _, cx| this.change_shapes(&ids, cx, |s| s.ink = ink)))
                        }))
                        .into_any_element(),
                );
                let chip = |id: gpui::ElementId, on: bool| {
                    div()
                        .id(id)
                        .h(px(24.))
                        .px(px(7.))
                        .flex()
                        .items_center()
                        .justify_center()
                        .rounded(t.radius.small)
                        .text_size(t.text.caption)
                        .cursor_pointer()
                        .when(on, |d| d.bg(t.colors.brand_tint).text_color(t.colors.brand_on_tint))
                        .when(!on, |d| d.text_color(t.colors.text_muted).hover(|d| d.bg(t.colors.hover)))
                };
                items.push(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(2.))
                        .px(t.space.sm)
                        .py(px(2.))
                        .children(LineWidth::ALL.iter().map(|&w| {
                            let ids = ids.clone();
                            chip(("m-shape-line", w as usize).into(), shape.line == w)
                                .child(w.label())
                                .on_click(cx.listener(move |this, _, _, cx| this.change_shapes(&ids, cx, |s| s.line = w)))
                        }))
                        .child(div().w(px(1.)).h(px(16.)).mx(px(4.)).bg(t.colors.border))
                        .child({
                            let ids = ids.clone();
                            let dashed = !shape.dashed;
                            chip("m-shape-dash".into(), shape.dashed)
                                .child("Dashed")
                                .on_click(cx.listener(move |this, _, _, cx| this.change_shapes(&ids, cx, |s| s.dashed = dashed)))
                        })
                        .into_any_element(),
                );
                items.push(sep(&t).into_any_element());
                items.push(caption("Turn", &t).into_any_element());
                let turn_ids = ids.clone();
                items.push(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(2.))
                        .px(t.space.sm)
                        .py(px(2.))
                        .child({
                            let ids = turn_ids.clone();
                            chip("m-turn-left".into(), false)
                                .gap(px(4.))
                                .child(icon("icons/rotate-ccw.svg", px(13.), t.colors.text_muted))
                                .child("15°")
                                .on_click(cx.listener(move |this, _, _, cx| this.turn_shapes(&ids, -15., cx)))
                        })
                        .child({
                            let ids = turn_ids.clone();
                            chip("m-turn-right".into(), false)
                                .gap(px(4.))
                                .child(icon("icons/rotate-cw.svg", px(13.), t.colors.text_muted))
                                .child("15°")
                                .on_click(cx.listener(move |this, _, _, cx| this.turn_shapes(&ids, 15., cx)))
                        })
                        .child({
                            let ids = turn_ids.clone();
                            chip("m-turn-straight".into(), false)
                                .child("Straighten")
                                .on_click(cx.listener(move |this, _, _, cx| {
                                    this.menu = None;
                                    this.change_shapes(&ids, cx, |s| s.rot = 0.)
                                }))
                        })
                        .child(div().flex_1())
                        .child(div().text_size(t.text.micro).text_color(t.colors.text_subtle).child(format!("{}°  [ ]", shape.rot.round() as i32)))
                        .into_any_element(),
                );
                items.push(sep(&t).into_any_element());
                items.push(
                    row("m-shape-dup", "icons/copy.svg", "Duplicate", Some(shortcut("D")), &t)
                        .on_click(cx.listener(move |this, _, _, cx| {
                            this.menu = None;
                            this.duplicate_selection(cx)
                        }))
                        .into_any_element(),
                );
                let lock_ids = ids.clone();
                let locked = shape.locked;
                items.push(
                    row("m-shape-lock", if locked { "icons/lock-open.svg" } else { "icons/lock.svg" }, if locked { "Unlock" } else { "Lock" }, None, &t)
                        .on_click(cx.listener(move |this, _, _, cx| {
                            this.menu = None;
                            this.change_shapes(&lock_ids, cx, |s| s.locked = !locked)
                        }))
                        .into_any_element(),
                );
                items.push(
                    row("m-shape-del", "icons/trash.svg", "Delete", Some("Del".into()), &t)
                        .text_color(t.colors.danger)
                        .on_click(cx.listener(move |this, _, _, cx| {
                            this.menu = None;
                            this.change(cx, |b| b.sheet_mut().shapes.retain(|s| !ids.contains(&s.id)));
                            this.selection.retain(|s| !matches!(s, super::app::Selection::Shape(_)));
                        }))
                        .into_any_element(),
                );
            }
            _ => {}
        }
        let heading = match kind {
            MenuKind::Card(ix) => self.board.categories.get(ix).map(|c| c.name.clone()),
            _ => None,
        };
        let _ = self.screen == Screen::Home;
        Some(
            div()
                .id("menu")
                .occlude()
                .min_w(px(240.))
                .max_w(px(360.))
                .p(t.space.xs)
                .rounded(t.radius.card)
                .bg(t.colors.surface)
                .border_1()
                .border_color(t.colors.border)
                .shadow(t.shadow_float())
                .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                // Each page fades up in place as you go to it.
                .child(super::motion::enter(
                    div().children(heading.map(|h| caption_owned(h, &t))).children(items),
                    gpui::ElementId::Name(format!("menu-page-{kind:?}-{page:?}").into()),
                    t.motion.pop_in,
                    t.motion.sink,
                )),
        )
    }
}

fn caption_owned(text: String, t: &Theme) -> Div {
    let _ = caption;
    div().px(t.space.sm).pt(t.space.xs).pb(px(2.)).text_size(t.text.micro).text_color(t.colors.text_subtle).child(text)
}
