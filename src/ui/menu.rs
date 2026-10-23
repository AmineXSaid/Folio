//! Right-click menus: for a note, the empty sheet, a sheet's card, an
//! area, a label. Sub-pages (Move to, Remind me, Icon, Colour) open in
//! place with a way back.

use gpui::{anchored, deferred, div, prelude::*, px, Context, MouseButton, SharedString, Stateful, Div, Window};

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

fn caption(text: &'static str, t: &Theme) -> Div {
    div().px(t.space.sm).pt(t.space.xs).pb(px(2.)).text_size(t.text.micro).text_color(t.colors.text_subtle).child(text)
}

impl Folio {
    pub(super) fn render_menu(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let t = theme(cx).clone();
        let Some(menu) = self.menu.as_ref() else { return div().into_any_element() };
        let (at, kind, page) = (menu.at, menu.kind, menu.page);
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
                let Some(s) = self.board.find_note(id).cloned() else { return div().into_any_element() };
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
                        .on_click(cx.listener(move |this, _, _, cx| this.change_note(id, cx, |s| s.paper = p)))
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
                        .on_click(cx.listener(move |this, _, _, cx| this.change_note(id, cx, |s| s.font = f)))
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
                        .on_click(cx.listener(move |this, _, _, cx| this.change_note(id, cx, |s| s.text_size = z)))
                })));
                items.push(papers.into_any_element());
                items.push(fonts.into_any_element());
                items.push(sep(&t).into_any_element());
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
                items.push(
                    row("m-pic", "icons/image.svg", if s.image.is_some() { "Replace picture…" } else { "Add picture…" }, None, &t)
                        .on_click(cx.listener(move |this, _, _, cx| this.pick_image(id, cx)))
                        .into_any_element(),
                );
                let remind = match s.due {
                    Some(d) => format!("Reminder · {}", time::due_label(d, now())),
                    None => "Remind me…".into(),
                };
                items.push(row("m-remind", "icons/bell.svg", remind, Some("›".into()), &t).on_click(cx.listener(goto(MenuPage::Remind))).into_any_element());
                if let Some(l) = &s.link {
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
                items.push(if s.popped.is_some() {
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
                });
                items.push(row("m-move", "icons/folder-input.svg", "Move to sheet", Some("›".into()), &t).on_click(cx.listener(goto(MenuPage::MoveTo))).into_any_element());
                items.push(
                    row("m-dup", "icons/copy.svg", "Duplicate", Some(shortcut("D")), &t)
                        .on_click(cx.listener(move |this, _, _, cx| {
                            this.menu = None;
                            this.select(super::app::Selection::Sticker(id));
                            this.duplicate_selection(cx)
                        }))
                        .into_any_element(),
                );
                items.push(
                    row("m-copy", "icons/file-text.svg", "Copy as text", None, &t)
                        .on_click(cx.listener(move |this, _, _, cx| this.copy_note(id, cx)))
                        .into_any_element(),
                );
                let locked = s.locked;
                items.push(
                    row("m-lock", if locked { "icons/lock-open.svg" } else { "icons/lock.svg" }, if locked { "Unlock" } else { "Lock" }, None, &t)
                        .on_click(cx.listener(move |this, _, _, cx| {
                            this.menu = None;
                            this.change_note(id, cx, |s| s.locked = !locked)
                        }))
                        .into_any_element(),
                );
                let fixed = s.fixed;
                items.push(
                    row("m-fit", if fixed { "icons/maximize-2.svg" } else { "icons/minimize-2.svg" }, if fixed { "Grow with the text" } else { "Keep this height" }, None, &t)
                        .on_click(cx.listener(move |this, _, _, cx| {
                            this.menu = None;
                            let h = this.board.find_note(id).map(|s| this.note_rect(s).h);
                            this.change_note(id, cx, |s| {
                                s.fixed = !fixed;
                                if let (false, Some(h)) = (fixed, h) {
                                    s.rect.h = h;
                                }
                            })
                        }))
                        .into_any_element(),
                );
                if s.is_stale(now()) {
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
                        .on_click(cx.listener(move |this, _, _, cx| this.bin_note(id, true, cx)))
                        .into_any_element(),
                );
                items.push(
                    row("m-trash", "icons/trash.svg", "Move to Trash", Some("Del".into()), &t)
                        .text_color(t.colors.danger)
                        .on_click(cx.listener(move |this, _, _, cx| this.bin_note(id, false, cx)))
                        .into_any_element(),
                );
            }
            (MenuKind::Note(id), MenuPage::MoveTo) => {
                items.push(back("Move to sheet", cx).into_any_element());
                items.push(sep(&t).into_any_element());
                let here = self.board.sheet_of(id);
                for ix in self.board.ordered() {
                    if Some(ix) == here {
                        continue;
                    }
                    let c = &self.board.categories[ix];
                    let label = if c.icon.is_empty() { c.name.clone() } else { format!("{} {}", c.icon, c.name) };
                    items.push(
                        row(format!("m-to-{ix}"), "icons/folder-input.svg", label, None, &t)
                            .on_click(cx.listener(move |this, _, _, cx| this.move_note_to(id, ix, cx)))
                            .into_any_element(),
                    );
                }
            }
            (MenuKind::Note(id), MenuPage::Remind) => {
                items.push(back("Remind me", cx).into_any_element());
                items.push(sep(&t).into_any_element());
                for (i, (label, when)) in time::presets(now()).into_iter().enumerate() {
                    items.push(
                        row(format!("m-at-{i}"), "icons/clock.svg", label, None, &t)
                            .on_click(cx.listener(move |this, _, _, cx| this.set_reminder(id, Some(when), cx)))
                            .into_any_element(),
                    );
                }
                if self.board.find_note(id).is_some_and(|s| s.due.is_some()) {
                    items.push(sep(&t).into_any_element());
                    items.push(
                        row("m-clear", "icons/bell-off.svg", "Clear reminder", None, &t)
                            .on_click(cx.listener(move |this, _, _, cx| this.set_reminder(id, None, cx)))
                            .into_any_element(),
                    );
                }
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
            }
            (MenuKind::Card(ix), MenuPage::Main) => {
                let Some(c) = self.board.categories.get(ix) else { return div().into_any_element() };
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
            _ => {}
        }
        let heading = match kind {
            MenuKind::Card(ix) => self.board.categories.get(ix).map(|c| c.name.clone()),
            _ => None,
        };
        let _ = self.screen == Screen::Home;
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
            .child(deferred(
                anchored().position(at).snap_to_window_with_margin(px(8.)).child(super::motion::rise_in(
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
                        .children(heading.map(|h| caption_owned(h, &t)))
                        .children(items),
                    // Each opening (and each page) rises into place.
                    gpui::ElementId::Name(format!("menu-{at:?}-{kind:?}-{page:?}").into()),
                    -4.,
                    &t,
                )),
            ))
            .into_any_element()
    }
}

fn caption_owned(text: String, t: &Theme) -> Div {
    let _ = caption;
    div().px(t.space.sm).pt(t.space.xs).pb(px(2.)).text_size(t.text.micro).text_color(t.colors.text_subtle).child(text)
}
