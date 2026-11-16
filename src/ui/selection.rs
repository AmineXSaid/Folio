//! The selection bar: with two or more things picked (Ctrl or Shift and a
//! click, a box drawn on the sheet, or the lasso), a bar rises above the
//! dock. It says what's picked, kind by kind (a kind clicked keeps only
//! those), lines them up or spreads them evenly, wraps them in an area, and
//! deletes them. With notes only it also does what a note's menu does to
//! one: lock, duplicate, move to a sheet, archive, trash; "More" opens the
//! whole menu for the group.

use gpui::{div, prelude::*, px, Context, MouseButton, MouseDownEvent, SharedString};

use super::app::{Folio, Gesture, Menu, MenuKind, MenuPage, Selection, Tool};
use super::*;
use crate::model::{align_moves, distribute_moves, LineUp, Rect, Vec2};
use crate::theme::Theme;

/// The kinds of picked thing, as the bar counts them: (singular, plural).
fn kind_of(s: &Selection) -> Option<(usize, &'static str, &'static str)> {
    match s {
        Selection::Sticker(_) => Some((0, "note", "notes")),
        Selection::Shape(_) => Some((1, "shape", "shapes")),
        Selection::Label(_) => Some((2, "text", "texts")),
        Selection::Stroke(_) => Some((3, "line", "lines")),
        Selection::Frame(_) => Some((4, "area", "areas")),
        Selection::Link(_) => None,
    }
}

/// Room left round things wrapped in a new area, and above them for its title.
const WRAP_PAD: f32 = 32.;
const WRAP_TITLE: f32 = 40.;

/// How far out from what's picked its box is drawn (px), and how near a
/// corner of it a press takes that corner.
const BOX_OUT: f32 = 10.;
const CORNER_REACH: f32 = 9.;

impl Folio {
    /// The box round two or more picked things (world), while they can be
    /// resized together.
    pub(super) fn many_box(&self) -> Option<Rect> {
        let busy = matches!(self.gesture, Gesture::Move { moved: true, .. } | Gesture::Marquee { .. } | Gesture::Lasso { .. });
        if self.tool != Tool::Select || busy || self.editing.is_some() {
            return None;
        }
        let boxes = self.picked_boxes();
        if boxes.len() < 2 {
            return None;
        }
        let z = self.sheet().camera.zoom;
        let all = boxes.into_iter().map(|p| p.1).reduce(|a, b| a.union(&b))?;
        let out = BOX_OUT / z;
        Some(Rect { x: all.x - out, y: all.y - out, w: all.w + 2. * out, h: all.h + 2. * out })
    }

    /// The corner of the selection's box under `at` (screen): the opposite
    /// corner, which stays put, and that corner (world).
    pub(super) fn many_grip_at(&self, at: Vec2) -> Option<(Vec2, Vec2)> {
        let r = self.many_box()?;
        let cam = self.sheet().camera;
        let corners = [(r.x, r.y), (r.x + r.w, r.y), (r.x, r.y + r.h), (r.x + r.w, r.y + r.h)];
        corners.iter().enumerate().find_map(|(i, &(x, y))| {
            let s = cam.to_screen(Vec2 { x, y });
            (s.sub(at).len() <= CORNER_REACH).then(|| {
                let (ox, oy) = corners[3 - i];
                (Vec2 { x: ox, y: oy }, Vec2 { x, y })
            })
        })
    }

    /// Starts resizing the picked things from a corner of their box.
    pub(super) fn start_scale_many(&mut self, anchor: Vec2, start: Vec2) {
        use super::app::Picked;
        let c = self.sheet();
        let was: Vec<Picked> = self
            .selection
            .iter()
            .filter_map(|s| match *s {
                Selection::Sticker(id) => c.sticker(id).filter(|n| !n.locked).cloned().map(|mut n| {
                    n.rect = self.note_rect(&n);
                    Picked::Note(n)
                }),
                Selection::Shape(id) => c.shapes.iter().find(|x| x.id == id && !x.locked).cloned().map(Picked::Shape),
                Selection::Label(id) => c.labels.iter().find(|x| x.id == id).cloned().map(Picked::Label),
                Selection::Stroke(id) => c.strokes.iter().find(|x| x.id == id).cloned().map(Picked::Stroke),
                Selection::Frame(id) => c.frames.iter().find(|x| x.id == id).cloned().map(Picked::Frame),
                Selection::Link(_) => None,
            })
            .collect();
        self.gesture = Gesture::ScaleMany { anchor, start, was };
    }

    /// The picked things scaled about `anchor` by how far the pointer is
    /// from it now, against where it started (the same both ways, so
    /// nothing is squashed).
    pub(super) fn scale_many(&mut self, anchor: Vec2, start: Vec2, was: &[super::app::Picked], world: Vec2) {
        use super::app::Picked;
        let k = (world.sub(anchor).len() / start.sub(anchor).len().max(1.)).clamp(0.15, 8.);
        let at = |p: Vec2| Vec2 { x: anchor.x + (p.x - anchor.x) * k, y: anchor.y + (p.y - anchor.y) * k };
        let scaled = |r: Rect| {
            let p = at(Vec2 { x: r.x, y: r.y });
            Rect { x: p.x, y: p.y, w: r.w * k, h: r.h * k }
        };
        let c = self.board.sheet_mut();
        for p in was {
            match p {
                Picked::Note(n) => {
                    if let Some(x) = c.sticker_mut(n.id) {
                        let r = scaled(n.rect);
                        x.rect = Rect { w: r.w.max(80.), h: r.h.max(48.), ..r };
                    }
                }
                Picked::Shape(s) => {
                    if let Some(x) = c.shapes.iter_mut().find(|x| x.id == s.id) {
                        let r = scaled(s.rect);
                        x.rect = Rect { w: r.w.max(8.), h: r.h.max(8.), ..r };
                    }
                }
                Picked::Label(l) => {
                    if let Some(x) = c.labels.iter_mut().find(|x| x.id == l.id) {
                        x.at = at(l.at);
                        x.width = l.width.map(|w| (w * k).max(80.));
                    }
                }
                Picked::Stroke(s) => {
                    if let Some(x) = c.strokes.iter_mut().find(|x| x.id == s.id) {
                        x.points = s.points.iter().map(|p| at(*p)).collect();
                        x.width = (s.width * k).max(0.5);
                    }
                }
                Picked::Frame(f) => {
                    if let Some(x) = c.frames.iter_mut().find(|x| x.id == f.id) {
                        x.rect = scaled(f.rect);
                    }
                }
            }
        }
    }

    /// The box round a mixed selection and its four corner handles.
    pub(super) fn render_many_box(&self, t: &Theme) -> Vec<gpui::AnyElement> {
        let Some(r) = self.many_box() else { return Vec::new() };
        let cam = self.sheet().camera;
        let a = cam.to_screen(Vec2 { x: r.x, y: r.y });
        let (w, h) = (r.w * cam.zoom, r.h * cam.zoom);
        let mut out = vec![div()
            .absolute()
            .left(px(a.x))
            .top(px(a.y))
            .w(px(w))
            .h(px(h))
            .border_1()
            .border_dashed()
            .border_color(t.colors.brand.opacity(0.7))
            .rounded(t.radius.small)
            .into_any_element()];
        for (x, y) in [(a.x, a.y), (a.x + w, a.y), (a.x, a.y + h), (a.x + w, a.y + h)] {
            out.push(
                div()
                    .absolute()
                    .left(px(x - 5.))
                    .top(px(y - 5.))
                    .size(px(10.))
                    .rounded(px(2.))
                    .bg(t.colors.surface)
                    .border_1()
                    .border_color(t.colors.brand)
                    .into_any_element(),
            );
        }
        out
    }

    /// Each picked thing that has a place on the sheet, and its box there.
    pub(super) fn picked_boxes(&self) -> Vec<(Selection, Rect)> {
        let c = self.sheet();
        self.selection
            .iter()
            .filter_map(|s| {
                let r = match *s {
                    Selection::Sticker(id) => c.sticker(id).filter(|n| !n.locked).map(|n| self.note_rect(n)),
                    Selection::Label(id) => c.labels.iter().find(|l| l.id == id).map(|l| self.label_box(l)),
                    Selection::Shape(id) => c.shapes.iter().find(|x| x.id == id && !x.locked).map(|x| x.bounds()),
                    Selection::Stroke(id) => c.strokes.iter().find(|x| x.id == id).map(|x| x.bounds()),
                    Selection::Frame(id) => c.frames.iter().find(|f| f.id == id).map(|f| f.rect),
                    Selection::Link(_) => None,
                }?;
                Some((*s, r))
            })
            .collect()
    }

    /// Moves each of `items` by its own amount, as one undo step.
    fn shift_picked(&mut self, items: Vec<(Selection, Vec2)>, cx: &mut Context<Self>) {
        if items.iter().all(|(_, d)| d.x.abs() < 0.01 && d.y.abs() < 0.01) {
            return;
        }
        self.change(cx, |b| {
            let c = b.sheet_mut();
            for (s, d) in &items {
                match *s {
                    Selection::Sticker(id) => {
                        if let Some(n) = c.sticker_mut(id) {
                            n.rect.x += d.x;
                            n.rect.y += d.y;
                        }
                    }
                    Selection::Label(id) => {
                        if let Some(l) = c.labels.iter_mut().find(|l| l.id == id) {
                            l.at = l.at.add(*d);
                        }
                    }
                    Selection::Shape(id) => {
                        if let Some(x) = c.shapes.iter_mut().find(|x| x.id == id) {
                            x.rect.x += d.x;
                            x.rect.y += d.y;
                        }
                    }
                    Selection::Stroke(id) => {
                        if let Some(x) = c.strokes.iter_mut().find(|x| x.id == id) {
                            x.translate(*d);
                        }
                    }
                    Selection::Frame(id) => {
                        if let Some(f) = c.frames.iter_mut().find(|f| f.id == id) {
                            f.rect.x += d.x;
                            f.rect.y += d.y;
                        }
                    }
                    Selection::Link(_) => {}
                }
            }
        });
    }

    /// Lines the picked things up by an edge or their middles.
    pub fn line_up(&mut self, how: LineUp, cx: &mut Context<Self>) {
        let picked = self.picked_boxes();
        let boxes: Vec<Rect> = picked.iter().map(|p| p.1).collect();
        let moves = align_moves(&boxes, how);
        self.shift_picked(picked.into_iter().map(|p| p.0).zip(moves).collect(), cx);
    }

    /// Spreads the picked things evenly across (or down).
    pub fn spread(&mut self, across: bool, cx: &mut Context<Self>) {
        let picked = self.picked_boxes();
        let boxes: Vec<Rect> = picked.iter().map(|p| p.1).collect();
        let moves = distribute_moves(&boxes, across);
        self.shift_picked(picked.into_iter().map(|p| p.0).zip(moves).collect(), cx);
    }

    /// A new area just round the picked things, its name asked for next.
    pub fn wrap_in_area(&mut self, cx: &mut Context<Self>) {
        let Some(all) = self.picked_boxes().into_iter().map(|p| p.1).reduce(|a, b| a.union(&b)) else { return };
        let r = Rect { x: all.x - WRAP_PAD, y: all.y - WRAP_PAD - WRAP_TITLE, w: all.w + 2. * WRAP_PAD, h: all.h + 2. * WRAP_PAD + WRAP_TITLE };
        self.new_frame(r, cx);
    }

    /// Keeps only the picked things of kind `kind` (as `kind_of` numbers them).
    fn keep_only(&mut self, kind: usize, cx: &mut Context<Self>) {
        self.selection.retain(|s| kind_of(s).is_some_and(|k| k.0 == kind));
        cx.notify();
    }

    pub(super) fn render_selection_bar(&mut self, t: &Theme, cx: &mut Context<Self>) -> Option<gpui::AnyElement> {
        let dragging = matches!(self.gesture, Gesture::Move { moved: true, .. });
        let picked: Vec<Selection> = self.selection.iter().copied().filter(|s| kind_of(s).is_some()).collect();
        if picked.len() < 2 || self.tool != Tool::Select || dragging || self.editing.is_some() {
            return None;
        }
        let ids = self.selected_notes();
        let notes_only = ids.len() == picked.len();
        // All of one group already: the button takes them apart.
        let first_group = picked.first().and_then(|s| self.group_of(*s));
        let grouped = first_group.is_some() && picked.iter().filter(|s| !matches!(s, Selection::Frame(_))).all(|s| self.group_of(*s) == first_group);
        let button = |id: &'static str, path: &'static str, hint: String, danger: bool| {
            div()
                .id(id)
                .size(px(34.))
                .flex()
                .items_center()
                .justify_center()
                .rounded(t.radius.button)
                .cursor_pointer()
                .role(gpui::Role::Button)
                .aria_label(hint.split("   ").next().unwrap_or("").to_string())
                .hover(|d| d.bg(t.colors.hover))
                .active(|d| d.bg(t.colors.border))
                .tooltip(move |window, cx| tip(hint.clone(), window, cx))
                .child(icon(path, t.size.icon, if danger { t.colors.danger } else { t.colors.text_muted }))
        };
        let rule = || div().w(px(1.)).h(t.size.icon_lg).mx(t.space.xs).bg(t.colors.border);

        // What's picked, kind by kind: each a chip that keeps only its kind.
        let mut kinds: Vec<(usize, &'static str, &'static str, usize)> = Vec::new();
        for s in &picked {
            let (k, one, many) = kind_of(s).expect("filtered above");
            match kinds.iter_mut().find(|x| x.0 == k) {
                Some(x) => x.3 += 1,
                None => kinds.push((k, one, many, 1)),
            }
        }
        kinds.sort_by_key(|x| x.0);
        let several_kinds = kinds.len() > 1;
        let count = div()
            .flex()
            .items_center()
            .gap(px(4.))
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
                    .child(SharedString::from(picked.len().to_string())),
            )
            .children(kinds.into_iter().map(|(k, one, many, n)| {
                let label = format!("{n} {}", if n == 1 { one } else { many });
                div()
                    .id(("sel-kind", k))
                    .h(px(26.))
                    .px(px(8.))
                    .flex()
                    .items_center()
                    .rounded(t.radius.full)
                    .text_size(t.text.body)
                    .font_weight(gpui::FontWeight::MEDIUM)
                    .text_color(t.colors.text)
                    .when(several_kinds, |d| {
                        let tip_text = format!("Keep only the {many}");
                        d.cursor_pointer()
                            .hover(|d| d.bg(t.colors.hover))
                            .role(gpui::Role::Button)
                            .aria_label(tip_text.clone())
                            .tooltip(move |window, cx| tip(tip_text.clone(), window, cx))
                            .on_click(cx.listener(move |this, _, _, cx| this.keep_only(k, cx)))
                    })
                    .child(label)
            }));

        // Lining up and spreading out.
        let lines = div().flex().items_center().gap(px(2.)).children(LineUp::ALL.iter().map(|&how| {
            let id = match how {
                LineUp::Left => "sel-al",
                LineUp::Centre => "sel-ac",
                LineUp::Right => "sel-ar",
                LineUp::Top => "sel-at",
                LineUp::Middle => "sel-am",
                LineUp::Bottom => "sel-ab",
            };
            button(id, how.icon(), how.label().into(), false).on_click(cx.listener(move |this, _, _, cx| this.line_up(how, cx)))
        }));
        let spread_ok = picked.len() >= 3;
        let spreads = div()
            .flex()
            .items_center()
            .gap(px(2.))
            .when(!spread_ok, |d| d.opacity(0.4))
            .child(button("sel-spread-x", "icons/distribute-across.svg", "Spread evenly across (three or more)".into(), false).on_click(cx.listener(|this, _, _, cx| this.spread(true, cx))))
            .child(button("sel-spread-y", "icons/distribute-down.svg", "Spread evenly down (three or more)".into(), false).on_click(cx.listener(|this, _, _, cx| this.spread(false, cx))));

        let mut bar = super::toolbar::dock(t)
            .id("selection-bar")
            .occlude()
            .gap(px(2.))
            .p(px(5.))
            .child(count)
            .child(rule())
            .child(lines)
            .child(spreads)
            .child(button("sel-wrap", "icons/square-dashed.svg", "Wrap in an area".into(), false).on_click(cx.listener(|this, _, _, cx| this.wrap_in_area(cx))))
            .child(if grouped {
                button("sel-ungroup", "icons/ungroup.svg", format!("Ungroup   {}", shortcut("Shift+G")), false).on_click(cx.listener(|this, _, _, cx| this.group_selection(true, cx)))
            } else {
                button("sel-group", "icons/group.svg", format!("Group   {}", shortcut("G")), false).on_click(cx.listener(|this, _, _, cx| this.group_selection(false, cx)))
            })
            .child(rule());
        if notes_only {
            // Notes only: what a note's menu does, to all of them.
            let first = ids[0];
            let locked = ids.iter().all(|id| self.board.find_note(*id).is_some_and(|s| s.locked));
            let menu_at = move |page: MenuPage| {
                move |this: &mut Folio, ev: &MouseDownEvent, _: &mut gpui::Window, cx: &mut Context<Folio>| {
                    cx.stop_propagation();
                    this.menu = Some(Menu { at: ev.position, kind: MenuKind::Note(first), page });
                    cx.notify();
                }
            };
            bar = bar
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
                .child(button("sel-more", "icons/ellipsis.svg", "More…".into(), false).on_mouse_down(MouseButton::Left, cx.listener(menu_at(MenuPage::Main))));
        } else {
            bar = bar
                .child(button("sel-dup", "icons/copy.svg", format!("Duplicate   {}", shortcut("D")), false).on_click(cx.listener(|this, _, _, cx| this.duplicate_selection(cx))))
                .child(button("sel-delete", "icons/trash.svg", "Delete   Del".into(), true).on_click(cx.listener(|this, _, _, cx| this.delete_selection(cx))));
        }
        let bar = bar.child(button("sel-clear", "icons/x.svg", "Clear the selection   Esc".into(), false).on_click(cx.listener(|this, _, _, cx| {
            this.selection.clear();
            cx.notify();
        })));
        Some(div().mb(t.space.sm).child(motion::rise_in(bar, "selection-in", t)).into_any_element())
    }
}
