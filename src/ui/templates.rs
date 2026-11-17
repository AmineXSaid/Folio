//! The templates gallery: a panel on the right with a picture of every
//! template, by family, and a search. Click one to drop it in the middle of
//! the view, or drag it onto the sheet; it lands as one group, picked.

use std::time::Duration;

use gpui::component::input::Input;
use gpui::component::Sizable as _;
use gpui::{div, prelude::*, px, Animation, AnimationExt, Context, MouseButton, Window};

use super::app::{Folio, Selection, Tool};
use super::mini::miniature;
use super::*;
use crate::model::*;
use crate::theme::{theme, Theme};

/// A template picked up in the gallery, being dragged onto the sheet.
#[derive(Clone, Copy)]
pub struct TemplateDrag(pub Template);

/// What follows the pointer while a template is dragged.
pub struct TemplateGhost {
    name: &'static str,
    bg: gpui::Hsla,
    fg: gpui::Hsla,
    border: gpui::Hsla,
}

impl Render for TemplateGhost {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
            .px(px(10.))
            .py(px(6.))
            .rounded(px(8.))
            .border_1()
            .border_color(self.border)
            .bg(self.bg)
            .text_color(self.fg)
            .text_size(px(12.))
            .opacity(0.9)
            .child(self.name)
    }
}

/// The templates whose name, family or keywords hold every word of `q`,
/// those whose name starts with it first.
pub fn find(q: &str) -> Vec<Template> {
    let q = q.trim().to_lowercase();
    if q.is_empty() {
        return Vec::new();
    }
    let words: Vec<&str> = q.split_whitespace().collect();
    let mut hits: Vec<(u8, Template)> = Template::ALL
        .iter()
        .filter(|t| **t != Template::Blank)
        .filter_map(|&t| {
            let name = t.name().to_lowercase();
            let all = format!("{name} {} {} {}", t.family().name().to_lowercase(), t.keywords(), t.blurb().to_lowercase());
            words.iter().all(|w| all.contains(w)).then(|| (if name.starts_with(&q) { 0 } else if name.contains(&q) { 1 } else { 2 }, t))
        })
        .collect();
    hits.sort_by_key(|(rank, _)| *rank);
    hits.into_iter().map(|(_, t)| t).collect()
}

fn ease_out_cubic(k: f32) -> f32 {
    1. - (1. - k).powi(3)
}

impl Folio {
    /// The shapes panel or the gallery is open on the right.
    pub fn right_panel(&self) -> bool {
        self.shapes_panel || self.templates_panel
    }

    /// Opens or closes the gallery (closing the shapes panel); open, its
    /// search is ready to type.
    pub fn toggle_templates_panel(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.templates_panel = !self.templates_panel;
        if self.templates_panel {
            self.shapes_panel = false;
            let input = self.template_search.clone();
            window.defer(cx, move |window, cx| input.update(cx, |i, cx| i.focus(window, cx)));
        } else {
            window.focus(&self.focus, cx);
        }
        cx.notify();
    }

    /// Drops template `t` centred on `at` (world), or in the middle of the
    /// view, as one undo step; what lands is picked and brought into view.
    pub fn drop_template(&mut self, t: Template, at: Option<Vec2>, cx: &mut Context<Self>) {
        self.finish_edit(cx);
        let (w, h) = self.viewport_size();
        let cam = self.sheet().camera;
        let centre = at.unwrap_or_else(|| cam.to_world(v2(w / 2., h / 2.)));
        let now = crate::time::now();
        let mut piece = None;
        self.change(cx, |b| piece = Some(b.drop_template(t, centre, now)));
        let Some(piece) = piece else { return };
        let mut picked: Vec<Selection> = Vec::new();
        picked.extend(piece.frames.iter().map(|f| Selection::Frame(f.id)));
        picked.extend(piece.stickers.iter().map(|s| Selection::Sticker(s.id)));
        picked.extend(piece.shapes.iter().map(|s| Selection::Shape(s.id)));
        picked.extend(piece.labels.iter().map(|l| Selection::Label(l.id)));
        for s in &piece.shapes {
            self.shape_motion.arrive(s.id);
        }
        self.tool = Tool::Select;
        self.selection = picked;
        // Brought into view when it isn't all there.
        if let Some(r) = piece.bounds() {
            let room = if self.right_panel() { f32::from(theme(cx).size.shapes_panel) } else { 0. };
            let a = cam.to_screen(v2(r.x, r.y));
            let b = cam.to_screen(v2(r.x + r.w, r.y + r.h));
            if a.x < 20. || a.y < 70. || b.x > w - room - 20. || b.y > h - 90. {
                let c = &mut self.board.sheet_mut().camera;
                let zoom = c.zoom;
                c.fit(r, w - room, h - 60., 80.);
                c.zoom = c.zoom.min(zoom.max(0.6));
                let mid = r.center();
                c.pan = v2((w - room) / 2. - mid.x * c.zoom, h / 2. + 10. - mid.y * c.zoom);
                self.schedule_view_save(cx);
            }
        }
        self.notify(format!("{} added · Ctrl+Z takes it back", t.name()), cx);
        cx.notify();
    }

    /// The dock's Templates button: opens and closes the gallery.
    pub(super) fn templates_button(&self, cx: &mut Context<Self>) -> gpui::AnyElement {
        let t = theme(cx).clone();
        let on = self.templates_panel;
        let tip = format!("Templates   {}", shortcut("Shift+P"));
        div()
            .id("dock-templates")
            .role(gpui::Role::Button)
            .aria_label("Templates")
            .size(px(38.))
            .flex()
            .items_center()
            .justify_center()
            .rounded(px(11.))
            .cursor_pointer()
            .tooltip(move |window, cx| super::tip(tip.clone(), window, cx))
            .when(on, |d| d.bg(t.colors.brand_tint))
            .when(!on, |d| d.hover(|d| d.bg(t.colors.hover)).active(|d| d.bg(t.colors.border)))
            .child(icon("icons/layout-grid.svg", px(17.), if on { t.colors.brand_on_tint } else { t.colors.text_muted }))
            .on_click(cx.listener(|this, _, window, cx| this.toggle_templates_panel(window, cx)))
            .into_any_element()
    }

    /// The gallery, sliding in from the right while open.
    pub(super) fn render_templates_panel(&mut self, cx: &mut Context<Self>) -> Option<gpui::AnyElement> {
        let t = theme(cx).clone();
        let w = f32::from(t.size.shapes_panel);
        if self.templates_panel {
            let ms = super::motion::ms(t.motion.drawer_in);
            let panel = self.templates_panel_card(&t, cx).with_animation(
                "templates-panel-in",
                Animation::new(Duration::from_millis(ms)).with_easing(ease_out_cubic),
                move |d, k| d.right(px(-w * (1. - k))).opacity(0.4 + 0.6 * k),
            );
            return Some(panel.into_any_element());
        }
        let (_, n) = self.exits.templates_panel.leaving(t.motion.drawer_out)?;
        let ms = super::motion::ms(t.motion.drawer_out);
        let panel = self
            .templates_panel_card(&t, cx)
            .child(div().absolute().inset_0().occlude())
            .with_animation(("templates-panel-out", n), Animation::new(Duration::from_millis(ms)).with_easing(|k: f32| k * k * k), move |d, k| {
                d.right(px(-w * k)).opacity(1. - 0.6 * k)
            });
        Some(panel.into_any_element())
    }

    fn templates_panel_card(&mut self, t: &Theme, cx: &mut Context<Self>) -> gpui::Stateful<gpui::Div> {
        let q = self.template_search.read(cx).value().trim().to_string();
        let has_query = !q.is_empty();
        let mut body: Vec<gpui::AnyElement> = Vec::new();
        let grid = |cards: Vec<gpui::Stateful<gpui::Div>>| div().px(t.space.md).pb(t.space.md).flex().flex_wrap().gap(t.space.sm).children(cards);
        if has_query {
            let found = find(&q);
            body.push(
                div()
                    .px(t.space.md)
                    .pt(t.space.xs)
                    .pb(t.space.sm)
                    .text_size(t.text.caption)
                    .text_color(t.colors.text_muted)
                    .child(match found.len() {
                        0 => format!("No template for “{q}”"),
                        1 => format!("1 template for “{q}” · Enter adds it"),
                        n => format!("{n} templates for “{q}” · Enter adds the first"),
                    })
                    .into_any_element(),
            );
            let cards: Vec<_> = found.iter().map(|&tp| self.template_card(tp, t, cx)).collect();
            body.push(grid(cards).into_any_element());
        } else {
            for fam in Family::ALL.iter().copied().filter(|f| *f != Family::Basics) {
                let list: Vec<Template> = Template::ALL.iter().copied().filter(|tp| tp.family() == fam).collect();
                body.push(
                    div()
                        .px(t.space.md)
                        .pt(t.space.sm)
                        .pb(t.space.xs)
                        .flex()
                        .items_center()
                        .child(div().flex_1().text_size(t.text.caption).font_weight(gpui::FontWeight::SEMIBOLD).text_color(t.colors.text).child(fam.name()))
                        .child(div().text_size(t.text.micro).text_color(t.colors.text_subtle).child(list.len().to_string()))
                        .into_any_element(),
                );
                let cards: Vec<_> = list.into_iter().map(|tp| self.template_card(tp, t, cx)).collect();
                body.push(grid(cards).into_any_element());
            }
        }
        let chips = ["retro", "canvas", "matrix", "flow", "plan"].map(|word| {
            div()
                .id(gpui::ElementId::Name(format!("template-chip-{word}").into()))
                .h(px(22.))
                .px(px(9.))
                .flex()
                .items_center()
                .rounded(t.radius.full)
                .border_1()
                .border_color(t.colors.border)
                .bg(t.colors.surface_sunk)
                .text_size(t.text.micro)
                .text_color(t.colors.text_body)
                .cursor_pointer()
                .hover(|d| d.bg(t.colors.hover).border_color(t.colors.text_subtle))
                .on_click(cx.listener(move |this, _, window, cx| {
                    this.template_search.update(cx, |i, cx| i.set_value(word, window, cx));
                    cx.notify();
                }))
                .child(word)
        });
        div()
            .id("templates-panel")
            .occlude()
            .absolute()
            .top_0()
            .bottom_0()
            .right_0()
            .w(t.size.shapes_panel)
            .flex()
            .flex_col()
            .bg(t.colors.surface)
            .border_l_1()
            .border_color(t.colors.border)
            .shadow(t.shadow_float())
            .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
            .child(
                div()
                    .px(t.space.md)
                    .pt(t.space.md)
                    .pb(t.space.sm)
                    .flex()
                    .flex_col()
                    .gap(t.space.sm)
                    .border_b_1()
                    .border_color(t.colors.border)
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(t.space.sm)
                            .child(icon("icons/layout-grid.svg", t.size.icon, t.colors.brand))
                            .child(div().flex_1().text_size(t.text.title).font_weight(gpui::FontWeight::SEMIBOLD).child("Templates"))
                            .child(
                                div()
                                    .px(px(5.))
                                    .py(px(1.))
                                    .rounded(t.radius.small)
                                    .border_1()
                                    .border_color(t.colors.border)
                                    .text_size(t.text.micro)
                                    .text_color(t.colors.text_subtle)
                                    .child(shortcut("Shift+P")),
                            )
                            .child(
                                icon_button("templates-close", "icons/x.svg", t)
                                    .tooltip(|window, cx| super::tip("Close", window, cx))
                                    .on_click(cx.listener(|this, _, window, cx| this.toggle_templates_panel(window, cx))),
                            ),
                    )
                    .child(
                        div()
                            .h(px(34.))
                            .px(t.space.sm)
                            .flex()
                            .items_center()
                            .gap(t.space.xs)
                            .rounded(t.radius.button)
                            .border_1()
                            .border_color(if has_query { t.colors.brand } else { t.colors.border })
                            .bg(t.colors.surface_sunk)
                            .child(icon("icons/search.svg", t.size.icon_sm, t.colors.text_muted))
                            .child(div().flex_1().child(Input::new(&self.template_search).appearance(false).small()))
                            .when(has_query, |d| {
                                d.child(
                                    div()
                                        .id("template-search-clear")
                                        .size(px(20.))
                                        .flex()
                                        .items_center()
                                        .justify_center()
                                        .rounded(t.radius.small)
                                        .cursor_pointer()
                                        .hover(|d| d.bg(t.colors.hover))
                                        .child(icon("icons/x.svg", px(12.), t.colors.text_muted))
                                        .on_click(cx.listener(|this, _, window, cx| {
                                            this.template_search.update(cx, |i, cx| i.set_value("", window, cx));
                                            cx.notify();
                                        })),
                                )
                            }),
                    )
                    .when(!has_query, |d| d.child(div().flex().flex_wrap().gap(px(5.)).children(chips))),
            )
            .child(div().id("templates-list").flex_1().min_h_0().overflow_y_scroll().py(t.space.xs).children(body))
            .child(
                div()
                    .px(t.space.md)
                    .py(t.space.sm)
                    .border_t_1()
                    .border_color(t.colors.border)
                    .text_size(t.text.micro)
                    .text_color(t.colors.text_muted)
                    .child(format!("{} templates · click to add, or drag onto the sheet. It lands as one group.", Template::ALL.len() - 1)),
            )
    }

    /// One template to pick: its picture, name and what it's for.
    fn template_card(&mut self, tp: Template, t: &Theme, cx: &mut Context<Self>) -> gpui::Stateful<gpui::Div> {
        let pw = (f32::from(t.size.shapes_panel) - f32::from(t.space.md) * 2. - f32::from(t.space.sm)) / 2. - 1.;
        let ph = f32::from(t.size.template_card);
        let sheet = self.template_pictures.entry(tp).or_insert_with(|| {
            let mut n = 0;
            crate::templates::build(
                tp,
                &mut || {
                    n += 1;
                    n
                },
                0,
            )
            .as_sheet()
        });
        let picture = miniature(sheet, pw - 2., ph, false, &crate::store::media_dir(&self.dir), t);
        let (name, blurb) = (tp.name(), tp.blurb());
        let (gbg, gfg, gborder) = (t.colors.surface, t.colors.text, t.colors.brand);
        div()
            .id(("template-card", tp as usize))
            .role(gpui::Role::Button)
            .aria_label(format!("Add template {name}"))
            .w(px(pw))
            .flex()
            .flex_col()
            .rounded(t.radius.button)
            .border_1()
            .border_color(t.colors.border)
            .bg(t.colors.surface)
            .overflow_hidden()
            .cursor_pointer()
            .hover(|d| d.border_color(t.colors.brand).shadow(t.shadow_card(1.)))
            .active(|d| d.bg(t.colors.brand_tint))
            .tooltip(move |window, cx| super::tip(format!("{name}: {blurb}"), window, cx))
            .on_click(cx.listener(move |this, _, window, cx| {
                this.drop_template(tp, None, cx);
                window.focus(&this.focus, cx);
            }))
            .on_drag(TemplateDrag(tp), move |d, _, _, cx| cx.new(|_| TemplateGhost { name: d.0.name(), bg: gbg, fg: gfg, border: gborder }))
            .child(div().h(px(ph)).overflow_hidden().border_b_1().border_color(t.colors.border).child(picture))
            .child(
                div()
                    .px(t.space.sm)
                    .py(px(6.))
                    .flex()
                    .flex_col()
                    .gap(px(1.))
                    .child(div().text_size(t.text.caption).font_weight(gpui::FontWeight::SEMIBOLD).text_color(t.colors.text).truncate().child(name))
                    .child(div().text_size(t.text.micro).text_color(t.colors.text_subtle).line_clamp(2).child(blurb)),
            )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn templates_are_found_by_name_family_and_keywords() {
        assert_eq!(find("swot").first(), Some(&Template::Swot));
        assert!(find("retro").contains(&Template::Sailboat));
        assert!(find("prioritize").contains(&Template::Moscow));
        assert!(find("kanban").starts_with(&[Template::Kanban]));
        assert!(find("").is_empty());
        assert!(find("zzzz").is_empty());
    }
}
