//! Text written straight on the sheet: Markdown (bold, italic, lists,
//! highlights...) in an ink, a typeface and a size, wrapping at its width.
//! While it's written, a bar above it has the marks, the typefaces, the
//! sizes and the inks; a selected one has a handle on its right edge for
//! where its lines wrap.

use gpui::component::input::Textarea;
use gpui::{canvas, div, prelude::*, px, Context, MouseButton, Styled, Window};

use super::app::{Folio, Format, Selection};
use super::*;
use crate::model::{v2, Align, Camera, Id, Ink, Label, LabelSize, NoteFont, Vec2, LABEL_WRAP};
use crate::theme::Theme;

/// The text area's padding around its text and the margin it keeps on
/// the right of its lines (gpui-component's, at its default size).
const EDIT_PAD_X: f32 = 10.;
const EDIT_PAD_Y: f32 = 8.;
const EDIT_MARGIN: f32 = 10.;
/// How near (px) a selected label's handle the pointer must be to take it.
const GRIP: f32 = 8.;
/// How far out from the text its handle sits (px).
const GRIP_OUT: f32 = 8.;

/// The label's look: its typeface, size and line spacing, its ink, slanted
/// or not.
fn look<E: Styled>(el: E, l: &Label, z: f32, t: &Theme) -> E {
    let el = el
        .font_family(t.note_font(l.font))
        .text_size(px(l.size.units() * z))
        .line_height(px(l.size.line() * z))
        .text_color(t.ink(l.ink))
        .text_align(match l.align {
            Align::Left => gpui::TextAlign::Left,
            Align::Center => gpui::TextAlign::Center,
            Align::Right => gpui::TextAlign::Right,
        });
    if l.italic {
        el.italic()
    } else {
        el
    }
}

/// Lines wrap at the label's width, or at most `LABEL_WRAP` wide.
fn wrapped<E: Styled>(el: E, l: &Label, z: f32) -> E {
    match l.width {
        Some(w) => el.w(px(w * z)),
        None => el.max_w(px(LABEL_WRAP * z)),
    }
}

impl Folio {
    /// A label on the sheet, measured as it's drawn so a click finds it
    /// where it really is.
    pub(super) fn render_label(&self, l: &Label, cam: Camera, t: &Theme) -> gpui::AnyElement {
        let z = cam.zoom;
        let at = cam.to_screen(l.at);
        let selected = self.is_selected(Selection::Label(l.id));
        let sizes = self.label_sizes.clone();
        let id = l.id;
        let measure = canvas(
            move |b, window, _| {
                let (w, h) = (f32::from(b.size.width) / z, f32::from(b.size.height) / z);
                let old = sizes.borrow_mut().insert(id, (w, h));
                if old.is_none_or(|(ow, oh)| (ow - w).abs() > 0.5 || (oh - h).abs() > 0.5) {
                    window.refresh();
                }
            },
            |_, _, _, _| {},
        )
        .absolute()
        .inset_0();
        let text = look(wrapped(div().absolute().left(px(at.x)).top(px(at.y)), l, z), l, z, t)
            .child(super::sticker::md_view_in(l.id * 1024 + 1023, &l.text, t.ink(l.ink), t))
            .child(measure);
        if !selected {
            return text.into_any_element();
        }
        // Selected: a frame around it, and the handle that sets its width.
        text.child(
            div()
                .absolute()
                .top(px(-4.))
                .bottom(px(-4.))
                .left(px(-6.))
                .right(px(-6.))
                .rounded(t.radius.small)
                .border_1()
                .border_color(t.colors.selection),
        )
        .child(
            div()
                .absolute()
                .top_0()
                .bottom_0()
                .right(px(-GRIP_OUT - 2.))
                .flex()
                .items_center()
                .child(div().w(px(4.)).h(px(18.)).rounded(px(2.)).bg(t.colors.selection)),
        )
        .into_any_element()
    }

    /// The selected label whose width handle is under `at` (px in the
    /// sheet), if any.
    pub(super) fn label_grip_at(&self, at: Vec2) -> Option<Id> {
        let cam = self.sheet().camera;
        self.sheet()
            .labels
            .iter()
            .filter(|l| Some(l.id) != self.label_edit && self.is_selected(Selection::Label(l.id)))
            .find(|l| {
                let r = self.label_box(l);
                let top = cam.to_screen(v2(r.x, r.y));
                let bottom = cam.to_screen(v2(r.x + r.w, r.y + r.h));
                (at.x - (bottom.x + GRIP_OUT)).abs() <= GRIP && at.y >= top.y - 4. && at.y <= bottom.y + 4.
            })
            .map(|l| l.id)
    }

    /// Text being written on the sheet: a caret where the text sits, in its
    /// own look; above it, the bar.
    pub(super) fn render_label_edit(&self, t: &Theme, cx: &mut Context<Self>) -> Option<gpui::AnyElement> {
        let id = self.label_edit?;
        let l = self.sheet().labels.iter().find(|l| l.id == id)?;
        let cam = self.sheet().camera;
        let z = cam.zoom;
        let at = cam.to_screen(l.at);
        let me = cx.entity();
        // It wraps where the label will once it's done: the text area
        // insets its text by its padding and keeps a margin on the right,
        // so the field reaches past the text by those.
        let field = look(div().id("label-edit").key_context("LabelEdit"), l, z, t)
            .absolute()
            .left(px(at.x - EDIT_PAD_X))
            .top(px(at.y - EDIT_PAD_Y))
            .w(px(l.width.unwrap_or(LABEL_WRAP) * z + 2. * EDIT_PAD_X + EDIT_MARGIN))
            // A press here is the text field's (to place the caret or pick
            // words out), not the sheet's.
            .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
            // The text area's text is 0.875rem: a rem that gives the label's size.
            .child(rem_scaled(px(l.size.units() * z / 0.875), Textarea::new(&self.label_input).appearance(false)));
        let field = on_format(field, move |f, window, cx| me.update(cx, |this, cx| this.format_label(f, window, cx)));
        let bar = super::motion::rise_in(self.label_bar(l, v2(at.x, at.y), t, cx), ("label-bar", id as usize), t);
        Some(div().absolute().inset_0().child(field).child(bar).into_any_element())
    }

    /// The bar over text being written: marks, lists and the inks at hand
    /// on the first row (the palette button opens every ink), the
    /// typefaces and sizes on the second.
    fn label_bar(&self, l: &Label, at: Vec2, t: &Theme, cx: &mut Context<Self>) -> gpui::Div {
        let id = l.id;
        let tip = |text: String| move |window: &mut Window, cx: &mut gpui::App| super::tip(text.clone(), window, cx);
        let button = |name: gpui::ElementId| {
            div()
                .id(name)
                .h(px(28.))
                .min_w(px(28.))
                .flex()
                .items_center()
                .justify_center()
                .rounded(t.radius.small)
                .cursor_pointer()
                .hover(|d| d.bg(t.colors.hover))
        };
        let write = |name: &'static str, path: &'static str, what: &str, keys: &str, f: Format| {
            button(name.into())
                .child(icon(path, t.size.icon_sm, t.colors.text_muted))
                .tooltip(tip(format!("{what}  {}", shortcut(keys))))
                .on_click(cx.listener(move |this, _, window, cx| this.format_label(f, window, cx)))
        };
        let chosen = |d: gpui::Stateful<gpui::Div>, on: bool| {
            if on {
                d.bg(t.colors.brand_tint).text_color(t.colors.brand_on_tint)
            } else {
                d.text_color(t.colors.text_muted)
            }
        };
        let divider = || div().w(px(1.)).h(px(20.)).mx(px(4.)).bg(t.colors.border);
        let row = || div().flex().items_center().gap(px(3.));

        let fonts = NoteFont::ALL.iter().map(|&f| {
            chosen(button(("l-font", f as usize).into()).px(px(6.)), l.font == f)
                .font_family(t.note_font(f))
                .text_size(t.text.caption)
                .child("Aa")
                .tooltip(tip(f.label().to_string()))
                .on_click(cx.listener(move |this, _, _, cx| this.set_label(id, cx, |l| l.font = f)))
        });
        let slanted = chosen(button("l-slant".into()).px(px(6.)), l.italic)
            .italic()
            .font_weight(gpui::FontWeight::SEMIBOLD)
            .text_size(t.text.caption)
            .child("I")
            .tooltip(tip("Slanted throughout".into()))
            .on_click(cx.listener(move |this, _, _, cx| this.set_label(id, cx, |l| l.italic = !l.italic)));
        let aligns = Align::ALL.iter().map(|&a| {
            chosen(button(("l-align", a as usize).into()).px(px(6.)), l.align == a)
                .child(gpui::svg().path(a.icon()).size(px(14.)).text_color(t.colors.text_muted))
                .tooltip(tip(a.label().to_string()))
                .on_click(cx.listener(move |this, _, _, cx| this.set_label(id, cx, |l| l.align = a)))
        });
        let sizes = LabelSize::ALL.iter().map(|&s| {
            chosen(button(("l-size", s as usize).into()).px(px(5.)), l.size == s)
                .text_size(t.text.caption)
                .child(s.label())
                .on_click(cx.listener(move |this, _, _, cx| this.set_label(id, cx, |l| l.size = s)))
        });
        let done = super::primary_face(div().id("l-done"), &t)
            .h(px(28.))
            .ml(px(6.))
            .px(px(10.))
            .flex()
            .items_center()
            .gap(px(4.))
            .rounded(t.radius.small)
            .text_size(t.text.caption)
            .font_weight(gpui::FontWeight::SEMIBOLD)
            .cursor_pointer()
            .child(icon("icons/check.svg", px(13.), t.colors.accent_ink))
            .child("Done")
            .tooltip(tip(format!("Done  Esc or {}", shortcut("Enter"))))
            .on_click(cx.listener(|this, _, window, cx| {
                this.finish_label(cx);
                window.focus(&this.focus, cx);
            }));
        // The inks at hand, as in the tool dock; the palette button opens
        // every ink.
        let swatch = |ink: Ink, d: f32| {
            let on = l.ink == ink;
            div()
                .id(("l-ink", ink as usize))
                .size(px(d + 8.))
                .flex()
                .items_center()
                .justify_center()
                .rounded(t.radius.full)
                .cursor_pointer()
                .border_2()
                .border_color(if on { t.ink(ink) } else { gpui::transparent_black() })
                .hover(|d| d.bg(t.colors.hover))
                .child(div().size(px(d)).rounded(t.radius.full).bg(t.ink(ink)))
                .on_click(cx.listener(move |this, _, _, cx| {
                    // The next text is written in it too.
                    this.ink = ink;
                    this.label_palette = false;
                    this.set_label(id, cx, |l| l.ink = ink);
                }))
        };
        let mut at_hand: Vec<Ink> = Ink::QUICK.to_vec();
        if !at_hand.contains(&l.ink) {
            at_hand.push(l.ink);
        }
        let open = self.label_palette;
        let palette = chosen(button("l-palette".into()), open)
            .child(icon("icons/palette.svg", t.size.icon_sm, if open { t.colors.brand } else { t.colors.text_muted }))
            .tooltip(tip("Every ink".into()))
            .on_click(cx.listener(|this, _, _, cx| {
                this.label_palette = !this.label_palette;
                cx.notify();
            }));
        let writing = row()
            .child(write("l-bold", "icons/bold.svg", "Bold", "B", Format::Bold))
            .child(write("l-italic", "icons/italic.svg", "Italic", "I", Format::Italic))
            .child(write("l-strike", "icons/strikethrough.svg", "Strikethrough", "Shift+X", Format::Strike))
            .child(write("l-mark", "icons/highlighter.svg", "Highlight", "Shift+M", Format::Highlight))
            .child(divider())
            .child(write("l-bullets", "icons/list.svg", "Bullets", "Shift+B", Format::Bullets))
            .child(write("l-numbers", "icons/list-ordered.svg", "Numbers", "Shift+O", Format::Numbers))
            .child(write("l-task", "icons/list-checks.svg", "Tasks", "Shift+C", Format::Tasks))
            .child(divider())
            .child(row().gap(px(2.)).children(at_hand.into_iter().map(|ink| swatch(ink, 12.))))
            .child(palette);
        let look_row = row()
            .children(fonts)
            .child(slanted)
            .child(divider())
            .children(aligns)
            .child(divider())
            .children(sizes)
            .child(div().flex_1())
            .child(done);
        // Every ink, in a palette hung under the bar.
        let every = open.then(|| {
            super::motion::rise_in(
                div()
                    .id("l-every-ink")
                    .absolute()
                    .top_full()
                    .right_0()
                    .mt(px(6.))
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
                    // Hung outside the bar: it keeps the caret in the text too.
                    .on_mouse_down(MouseButton::Left, |_, window, cx| {
                        window.prevent_default();
                        cx.stop_propagation();
                    })
                    .child(div().text_size(t.text.micro).font_weight(gpui::FontWeight::SEMIBOLD).text_color(t.colors.text_subtle).child("INK"))
                    .child(div().flex().flex_wrap().gap(px(4.)).children(Ink::ALL.iter().map(|&ink| swatch(ink, 16.)))),
                "l-every-ink-in",
                t,
            )
        });
        div()
            .absolute()
            .left(px(at.x))
            .top(px((at.y - 84.).max(4.)))
            .flex()
            .flex_col()
            .gap(px(3.))
            .p(px(5.))
            .rounded(t.radius.card)
            .bg(t.colors.surface)
            .border_1()
            .border_color(t.colors.border)
            .shadow(t.shadow_float())
            // The text keeps its caret: a press here never takes the focus.
            .on_mouse_down(MouseButton::Left, |_, window, cx| {
                window.prevent_default();
                cx.stop_propagation();
            })
            .child(writing)
            .child(look_row)
            .children(every)
    }
}
