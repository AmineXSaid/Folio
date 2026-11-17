//! Workshop tools on a sheet, as the view shows them: the Workshop button
//! and its card (dot voting, private writing), the vote bar while people
//! vote, the results once it ends, the banner while writing privately, and
//! what they put on notes (a voter's dots, a count, a cover).

use std::time::Duration;

use gpui::component::input::Input;
use gpui::component::Sizable as _;
use gpui::{canvas, div, prelude::*, px, AnyElement, Context, MouseButton, Window};

use super::app::{Folio, Selection};
use super::*;
use crate::model::*;
use crate::theme::{theme, Theme};
use crate::workshop::{total, SOMEONE};

impl Folio {
    /// A vote is open on this sheet.
    pub fn voting(&self) -> bool {
        self.sheet().vote.is_some_and(|v| v.open)
    }

    /// Who's voting at this screen: the name given at "Next person", or yours.
    pub fn voter(&self) -> String {
        let v = if self.voter.trim().is_empty() { self.board.settings.user_name.trim().to_string() } else { self.voter.trim().to_string() };
        if v.is_empty() {
            SOMEONE.to_string()
        } else {
            v
        }
    }

    /// Who's writing: you, by name.
    pub fn writer(&self) -> String {
        self.board.settings.user_name.trim().to_string()
    }

    /// A dot on note `id` from whoever's voting, or (`back`) one taken off.
    pub fn vote_on(&mut self, id: Id, back: bool, cx: &mut Context<Self>) {
        let who = self.voter();
        if back {
            let mut done = false;
            self.change(cx, |b| done = b.sheet_mut().take_back(id, &who));
            if !done {
                self.notify("None of your dots are on that note", cx);
            }
            return;
        }
        if self.sheet().dots_left(&who) == 0 {
            self.notify(format!("{who} has no dots left · Alt+click a note to take one back"), cx);
            return;
        }
        self.change(cx, |b| {
            b.sheet_mut().cast(id, &who);
        });
        self.dot_landed = Some((id, std::time::Instant::now()));
        cx.notify();
    }

    pub fn toggle_workshop(&mut self, cx: &mut Context<Self>) {
        self.finish_edit(cx);
        self.menu = None;
        self.export_pop = false;
        self.workshop_pop = !self.workshop_pop;
        cx.notify();
    }

    /// Starts a vote with the dots chosen in the card.
    pub fn start_vote(&mut self, cx: &mut Context<Self>) {
        let per = self.vote_per;
        self.workshop_pop = false;
        self.voter = String::new();
        self.results_shut = false;
        self.selection.clear();
        self.change(cx, |b| b.sheet_mut().start_vote(per, crate::time::now()));
        self.notify(format!("Voting: {per} dots each · click a note to put one on it"), cx);
    }

    /// Opens the card on its vote part, or ends the vote going on.
    pub fn vote_action(&mut self, cx: &mut Context<Self>) {
        if self.voting() {
            self.end_vote(cx);
        } else {
            self.workshop_pop = true;
            cx.notify();
        }
    }

    pub fn end_vote(&mut self, cx: &mut Context<Self>) {
        self.workshop_pop = false;
        self.naming_voter = false;
        self.results_shut = false;
        self.change(cx, |b| b.sheet_mut().end_vote());
    }

    pub fn clear_votes(&mut self, cx: &mut Context<Self>) {
        self.workshop_pop = false;
        self.change(cx, |b| b.sheet_mut().clear_votes());
    }

    pub fn toggle_private(&mut self, cx: &mut Context<Self>) {
        let on = !self.sheet().private;
        self.workshop_pop = false;
        self.change(cx, |b| b.sheet_mut().private = on);
        self.notify(if on { "Private writing on: new notes stay covered until revealed" } else { "Private writing off" }, cx);
    }

    pub fn reveal_all(&mut self, cx: &mut Context<Self>) {
        let ids: Vec<Id> = self.sheet().stickers.iter().filter(|s| s.hidden_by.is_some()).map(|s| s.id).collect();
        self.workshop_pop = false;
        self.change(cx, |b| b.sheet_mut().reveal_all());
        for id in &ids {
            self.exits.notes.arrive(*id);
        }
        self.notify(match ids.len() {
            0 => "Nothing was hidden".to_string(),
            1 => "1 note revealed".to_string(),
            n => format!("{n} notes revealed"),
        }, cx);
    }

    pub fn uncover_note(&mut self, id: Id, cx: &mut Context<Self>) {
        self.menu = None;
        self.change(cx, |b| b.sheet_mut().reveal(id));
        self.exits.notes.arrive(id);
    }

    /// Hands the screen to the next voter: asks their name.
    pub fn next_voter(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.naming_voter = true;
        let input = self.voter_input.clone();
        input.update(cx, |i, cx| i.set_value("", window, cx));
        window.defer(cx, move |window, cx| input.update(cx, |i, cx| i.focus(window, cx)));
        cx.notify();
    }

    /// The name typed for the next voter, taken.
    pub fn take_voter_name(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let name = self.voter_input.read(cx).value().trim().to_string();
        self.naming_voter = false;
        if !name.is_empty() {
            self.voter = name;
            let left = self.sheet().dots_left(&self.voter());
            self.notify(format!("{}'s turn · {left} dots", self.voter()), cx);
        }
        window.focus(&self.focus, cx);
        cx.notify();
    }

    /// The topbar's Workshop button.
    pub(super) fn workshop_button(&self, cx: &mut Context<Self>) -> AnyElement {
        let t = theme(cx).clone();
        let c = self.sheet();
        let live = c.vote.is_some() || c.private;
        let open = self.workshop_pop || live;
        let ink = if open { t.colors.brand_on_tint } else { t.colors.text_body };
        let at = self.workshop_at.clone();
        div()
            .id("workshop")
            .role(gpui::Role::Button)
            .aria_label("Workshop: dot voting and private writing")
            .occlude()
            .relative()
            .size(t.size.bar_control)
            .flex_none()
            .flex()
            .items_center()
            .justify_center()
            .rounded(t.radius.button)
            .border_1()
            .border_color(if open { t.colors.brand.opacity(0.5) } else { t.colors.border })
            .when(open, |d| d.bg(t.colors.brand_tint))
            .when(!open, |d| d.hover(|d| d.bg(t.colors.hover).border_color(t.colors.text_subtle)))
            .active(|d| d.bg(t.colors.border))
            .cursor_pointer()
            .child(icon("icons/vote.svg", px(15.), ink))
            .child(canvas(move |b, _, _| at.set(Some(b)), |_, _, _, _| {}).absolute().inset_0())
            .tooltip(move |window, cx| {
                super::tip(format!("Workshop: dot voting ({}), private writing ({})", shortcut("Shift+D"), shortcut("Shift+I")), window, cx)
            })
            .on_click(cx.listener(|this, _, _, cx| this.toggle_workshop(cx)))
            .into_any_element()
    }

    /// The Workshop button's card.
    pub(super) fn render_workshop_pop(&mut self, cx: &mut Context<Self>) -> Option<AnyElement> {
        if !self.workshop_pop || self.screen != super::app::Screen::Sheet {
            return None;
        }
        let t = theme(cx).clone();
        let b = self.workshop_at.get()?;
        let c = self.sheet();
        let (vote, private, hidden) = (c.vote, c.private, c.hidden_count());
        let per = self.vote_per;
        let head = |title: &'static str, help: String, path: &'static str| {
            div()
                .flex()
                .gap(t.space.sm)
                .child(div().pt(px(1.)).child(icon(path, t.size.icon, t.colors.brand)))
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .flex()
                        .flex_col()
                        .gap(px(2.))
                        .child(div().text_size(t.text.body).font_weight(gpui::FontWeight::SEMIBOLD).child(title))
                        .child(div().text_size(t.text.caption).text_color(t.colors.text_muted).child(help)),
                )
        };
        let step = |id: &'static str, path: &'static str, by: i8, cx: &mut Context<Self>| {
            icon_button(id, path, &t).size(px(26.)).border_1().border_color(t.colors.border).on_click(cx.listener(move |this, _, _, cx| {
                this.vote_per = (this.vote_per as i8 + by).clamp(1, 10) as u8;
                cx.notify();
            }))
        };
        let voting = match vote {
            Some(v) if v.open => div()
                .flex()
                .gap(t.space.sm)
                .child(primary_button("w-end", "End voting and show results", &t).on_click(cx.listener(|this, _, _, cx| this.end_vote(cx))))
                .child(secondary_button("w-cancel", "Cancel vote", &t).on_click(cx.listener(|this, _, _, cx| this.clear_votes(cx))))
                .into_any_element(),
            _ => div()
                .flex()
                .items_center()
                .gap(t.space.sm)
                .child(div().text_size(t.text.caption).text_color(t.colors.text_muted).child("Dots each"))
                .child(step("w-less", "icons/chevron-left.svg", -1, cx))
                .child(div().w(px(22.)).flex().justify_center().text_size(t.text.body).font_weight(gpui::FontWeight::SEMIBOLD).child(per.to_string()))
                .child(step("w-more", "icons/chevron-right.svg", 1, cx))
                .child(div().flex_1())
                .child(
                    primary_button("w-start", "Start voting", &t)
                        .tooltip(|window, cx| super::tip(format!("Start voting  {}", shortcut("Shift+D")), window, cx))
                        .on_click(cx.listener(|this, _, _, cx| this.start_vote(cx))),
                )
                .when(vote.is_some(), |d| d.child(secondary_button("w-clear", "Clear votes", &t).on_click(cx.listener(|this, _, _, cx| this.clear_votes(cx)))))
                .into_any_element(),
        };
        let switch = div()
            .id("w-private")
            .role(gpui::Role::Switch)
            .aria_label("Private writing")
            .w(px(34.))
            .h(px(20.))
            .flex_none()
            .rounded(t.radius.full)
            .bg(if private { t.colors.brand } else { t.colors.border })
            .relative()
            .cursor_pointer()
            .child(div().absolute().top(px(2.)).left(px(if private { 16. } else { 2. })).size(px(16.)).rounded(t.radius.full).bg(t.colors.surface))
            .tooltip(|window, cx| super::tip(format!("Private writing  {}", shortcut("Shift+I")), window, cx))
            .on_click(cx.listener(|this, _, _, cx| this.toggle_private(cx)));
        let card = div()
            .id("workshop-pop")
            .occlude()
            .w(px(380.))
            .p(t.space.lg)
            .flex()
            .flex_col()
            .gap(t.space.md)
            .rounded(t.radius.dialog)
            .bg(t.colors.surface)
            .border_1()
            .border_color(t.colors.border)
            .shadow(t.shadow_float())
            .font_family(t.font)
            .text_color(t.colors.text)
            .on_mouse_down_out(cx.listener(|this, ev: &gpui::MouseDownEvent, _, cx| {
                if !this.workshop_at.get().is_some_and(|b| b.contains(&ev.position)) {
                    this.workshop_pop = false;
                    cx.notify();
                }
            }))
            .child(head(
                "Dot voting",
                match vote {
                    Some(v) if v.open => format!("Going on · {} dots each · click a note to put a dot on it", v.per_person),
                    Some(_) => "Ended · the results are on the notes".into(),
                    None => "Everyone puts a few dots on the notes they back; the notes are then ranked.".into(),
                },
                "icons/vote.svg",
            ))
            .child(voting)
            .child(div().h(px(1.)).bg(t.colors.border))
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(t.space.md)
                    .child(div().flex_1().min_w_0().child(head(
                        "Private writing",
                        "New notes stay covered (lock and name) until revealed, so ideas don't sway each other.".into(),
                        "icons/eye-off.svg",
                    )))
                    .child(switch),
            )
            .when(hidden > 0, |d| {
                d.child(
                    secondary_button("w-reveal", format!("Reveal {hidden} hidden note{}", if hidden == 1 { "" } else { "s" }), &t)
                        .on_click(cx.listener(|this, _, _, cx| this.reveal_all(cx))),
                )
            });
        let at = gpui::point(b.right(), b.bottom() + px(8.));
        Some(
            gpui::deferred(
                gpui::anchored()
                    .anchor(gpui::Anchor::TopRight)
                    .position(at)
                    .snap_to_window_with_margin(px(8.))
                    .child(super::motion::rise_in(card, "workshop-pop-in", &t)),
            )
            .into_any_element(),
        )
    }

    /// Over the sheet: the vote bar while voting, the banner while writing
    /// privately, the results once a vote ends.
    pub(super) fn render_workshop_overlays(&mut self, window: &mut Window, cx: &mut Context<Self>) -> Vec<AnyElement> {
        let t = theme(cx).clone();
        let c = self.sheet();
        let (vote, private) = (c.vote, c.private);
        let mut out = Vec::new();
        let pill = |id: &'static str| {
            div()
                .id(id)
                .occlude()
                .flex()
                .items_center()
                .gap(t.space.sm)
                .px(t.space.md)
                .py(px(6.))
                .rounded(t.radius.full)
                .bg(t.colors.surface)
                .border_1()
                .border_color(t.colors.border)
                .shadow(t.shadow_float())
                .text_size(t.text.caption)
                .text_color(t.colors.text_body)
                .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
        };
        let small = |id: &'static str, label: &'static str| {
            div()
                .id(id)
                .px(px(10.))
                .h(px(24.))
                .flex()
                .items_center()
                .rounded(t.radius.full)
                .border_1()
                .border_color(t.colors.border)
                .cursor_pointer()
                .hover(|d| d.bg(t.colors.hover))
                .font_weight(gpui::FontWeight::MEDIUM)
                .child(label)
        };
        let mut rows: Vec<AnyElement> = Vec::new();
        if let Some(v) = vote.filter(|v| v.open) {
            let who = self.voter();
            let left = self.sheet().dots_left(&who);
            let dots = (0..v.per_person as u32).map(|i| {
                div().size(px(9.)).rounded(t.radius.full).when(i < left, |d| d.bg(t.colors.vote_dot)).when(i >= left, |d| d.border_1().border_color(t.colors.vote_dot))
            });
            let mut bar = pill("vote-bar")
                .child(icon("icons/vote.svg", t.size.icon_sm, t.colors.vote_dot))
                .child(div().font_weight(gpui::FontWeight::SEMIBOLD).text_color(t.colors.text).child("Voting"))
                .child("·");
            if self.naming_voter {
                bar = bar
                    .child("Next:")
                    .child(div().w(px(150.)).child(Input::new(&self.voter_input).small()))
                    .child(small("vote-name-ok", "Go").on_click(cx.listener(|this, _, window, cx| this.take_voter_name(window, cx))));
            } else {
                bar = bar
                    .child(div().font_weight(gpui::FontWeight::SEMIBOLD).text_color(t.colors.text).child(who.clone()))
                    .child(div().flex().gap(px(3.)).children(dots))
                    .child(format!("{left} of {} left", v.per_person))
                    .child(
                        small("vote-next", "Next person")
                            .tooltip(|window, cx| super::tip("Hand the screen on: the next person votes with their own dots", window, cx))
                            .on_click(cx.listener(|this, _, window, cx| this.next_voter(window, cx))),
                    );
            }
            bar = bar.child(
                small("vote-end", "End voting")
                    .text_color(t.colors.accent_ink)
                    .border_color(t.colors.accent_ink)
                    .tooltip(|window, cx| super::tip(format!("End voting and show results  {}", shortcut("Shift+D")), window, cx))
                    .on_click(cx.listener(|this, _, _, cx| this.end_vote(cx))),
            );
            rows.push(bar.into_any_element());
            rows.push(
                div()
                    .text_size(t.text.micro)
                    .text_color(t.colors.text_subtle)
                    .child("Click a note to put a dot on it · Alt+click or right-click takes one back · you see only your own dots")
                    .into_any_element(),
            );
        }
        if private {
            let hidden = self.sheet().hidden_count();
            rows.push(
                pill("private-banner")
                    .child(icon("icons/eye-off.svg", t.size.icon_sm, t.colors.brand))
                    .child(div().font_weight(gpui::FontWeight::SEMIBOLD).text_color(t.colors.text).child("Private writing is on"))
                    .child(format!("new notes stay covered until revealed · {hidden} hidden"))
                    .child(small("private-reveal", "Reveal all").on_click(cx.listener(|this, _, _, cx| this.reveal_all(cx))))
                    .child(small("private-off", "Turn off").on_click(cx.listener(|this, _, _, cx| this.toggle_private(cx))))
                    .into_any_element(),
            );
        }
        if !rows.is_empty() {
            let right = if self.right_panel() { t.size.shapes_panel } else { px(0.) };
            out.push(
                div()
                    .absolute()
                    .top(t.space.md)
                    .left_0()
                    .right(right)
                    .flex()
                    .flex_col()
                    .items_center()
                    .gap(t.space.xs)
                    .children(rows)
                    .into_any_element(),
            );
        }
        if let Some(v) = vote.filter(|v| !v.open) {
            if !self.results_shut {
                out.push(self.results_card(v, &t, window, cx));
            }
        }
        // A dot just put down keeps the sheet drawing while it pops in.
        if self.dot_landed.is_some_and(|(_, at)| at.elapsed() < Duration::from_millis(400)) {
            window.request_animation_frame();
        }
        out
    }

    /// The notes a vote ranked, most dots first.
    fn results_card(&self, _v: crate::workshop::VoteSession, t: &Theme, _: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        let c = self.sheet();
        let results = c.results();
        let voters = c.voters();
        let total_dots: u32 = results.iter().map(|r| r.1).sum();
        let top = results.first().map(|r| r.1).unwrap_or(1).max(1);
        let rows: Vec<AnyElement> = results
            .iter()
            .take(12)
            .enumerate()
            .map(|(i, &(id, n))| {
                let s = c.sticker(id);
                let name = s.map(|s| if s.hidden_by.is_some() { "Hidden note".to_string() } else { s.name() }).filter(|n| !n.is_empty()).unwrap_or_else(|| "Untitled note".into());
                let paper = s.map(|s| t.paper(s.paper)).unwrap_or(t.colors.surface);
                div()
                    .id(("vote-row", id))
                    .flex()
                    .items_center()
                    .gap(t.space.sm)
                    .px(t.space.sm)
                    .py(px(5.))
                    .rounded(t.radius.small)
                    .cursor_pointer()
                    .hover(|d| d.bg(t.colors.hover))
                    .child(div().w(px(16.)).text_size(t.text.micro).text_color(t.colors.text_subtle).child((i + 1).to_string()))
                    .child(div().size(px(14.)).flex_none().rounded(px(2.)).bg(paper).border_1().border_color(t.colors.border))
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .flex()
                            .flex_col()
                            .gap(px(3.))
                            .child(div().text_size(t.text.caption).text_color(t.colors.text).truncate().child(name))
                            .child(div().h(px(4.)).w(gpui::relative(n as f32 / top as f32)).rounded(px(2.)).bg(t.colors.vote_dot)),
                    )
                    .child(div().text_size(t.text.caption).font_weight(gpui::FontWeight::BOLD).text_color(t.colors.vote_badge).child(n.to_string()))
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.select(Selection::Sticker(id));
                        this.reveal_on_screen(id, cx);
                    }))
                    .into_any_element()
            })
            .collect();
        let right = if self.right_panel() { t.space.lg + t.size.shapes_panel } else { t.space.lg };
        div()
            .id("vote-results")
            .occlude()
            .absolute()
            .top(t.space.lg)
            .right(right)
            .w(px(300.))
            .max_h(px(520.))
            .p(t.space.md)
            .flex()
            .flex_col()
            .gap(t.space.sm)
            .rounded(t.radius.card)
            .bg(t.colors.surface)
            .border_1()
            .border_color(t.colors.border)
            .shadow(t.shadow_float())
            .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(t.space.sm)
                    .child(icon("icons/vote.svg", t.size.icon, t.colors.vote_badge))
                    .child(div().flex_1().text_size(t.text.title).font_weight(gpui::FontWeight::SEMIBOLD).child("Results"))
                    .child(
                        icon_button("vote-results-close", "icons/x.svg", t).on_click(cx.listener(|this, _, _, cx| {
                            this.results_shut = true;
                            cx.notify();
                        })),
                    ),
            )
            .child(div().text_size(t.text.caption).text_color(t.colors.text_muted).child(format!(
                "{total_dots} dot{} from {} {} · click one to go to it",
                if total_dots == 1 { "" } else { "s" },
                voters.len(),
                if voters.len() == 1 { "person" } else { "people" }
            )))
            .child(div().id("vote-rows").flex_1().min_h_0().overflow_y_scroll().flex().flex_col().children(rows))
            .when(results.is_empty(), |d| d.child(div().text_size(t.text.caption).text_color(t.colors.text_subtle).child("No dots were put down.")))
            .child(
                div()
                    .flex()
                    .gap(t.space.sm)
                    .child(secondary_button("vote-clear", "Clear votes", t).on_click(cx.listener(|this, _, _, cx| this.clear_votes(cx))))
                    .child(secondary_button("vote-again", "Vote again", t).on_click(cx.listener(|this, _, _, cx| {
                        this.workshop_pop = true;
                        cx.notify();
                    }))),
            )
            .into_any_element()
    }

    /// Brings note `id` into the middle of the view if it's off it.
    fn reveal_on_screen(&mut self, id: Id, cx: &mut Context<Self>) {
        let (w, h) = self.viewport_size();
        if let Some(r) = self.sheet().sticker(id).map(|s| self.note_rect(s)) {
            let c = r.center();
            let cam = &mut self.board.sheet_mut().camera;
            cam.pan = v2(w / 2. - c.x * cam.zoom, h / 2. - c.y * cam.zoom);
            self.schedule_view_save(cx);
        }
        cx.notify();
    }

    /// What the workshop puts on note `s`: a cover while it's hidden (but
    /// not while it's being written), the voter's dots while voting, the
    /// count once the vote ends.
    pub(super) fn workshop_marks(&self, s: &Sticker, z: f32, rt: f32, rb: f32, t: &Theme) -> Vec<AnyElement> {
        let mut out = Vec::new();
        let editing = self.editing.as_ref().is_some_and(|e| e.id == s.id);
        if let (Some(by), false) = (&s.hidden_by, editing) {
            let paper = t.paper(s.paper);
            let hatch = t.colors.hidden_hatch;
            let step = (14. * z).max(6.);
            out.push(
                div()
                    .absolute()
                    .inset_0()
                    .rounded_t(px(rt))
                    .rounded_b(px(rb))
                    .overflow_hidden()
                    .bg(paper)
                    .child(
                        canvas(
                            |_, _, _| {},
                            move |b, _, window, _| {
                                // Fine diagonal stripes, like the back of the paper.
                                let (x0, y0) = (f32::from(b.origin.x), f32::from(b.origin.y));
                                let (w, h) = (f32::from(b.size.width), f32::from(b.size.height));
                                let mut p = gpui::PathBuilder::stroke(px(1.5));
                                let mut d = -h;
                                while d < w {
                                    p.move_to(gpui::point(px(x0 + d), px(y0 + h)));
                                    p.line_to(gpui::point(px(x0 + d + h), px(y0)));
                                    d += step;
                                }
                                if let Ok(path) = p.build() {
                                    window.paint_path(path, hatch);
                                }
                            },
                        )
                        .absolute()
                        .inset_0(),
                    )
                    .child(
                        div()
                            .absolute()
                            .inset_0()
                            .flex()
                            .flex_col()
                            .items_center()
                            .justify_center()
                            .gap(px(4. * z))
                            .child(icon("icons/lock.svg", px((18. * z).max(9.)), t.colors.text_muted))
                            .child(div().text_size(px((12. * z).max(7.))).font_weight(gpui::FontWeight::SEMIBOLD).text_color(t.colors.text_muted).child(format!("Hidden · {by}"))),
                    )
                    .into_any_element(),
            );
        }
        match self.sheet().vote {
            Some(v) if v.open => {
                let mine = s.votes.get(&self.voter()).copied().unwrap_or(0);
                if mine > 0 {
                    let d = f32::from(t.size.vote_dot) * z.max(0.6);
                    let fresh = self.dot_landed.filter(|(id, _)| *id == s.id).map(|(_, at)| (at.elapsed().as_secs_f32() / 0.3).min(1.));
                    let dots = (0..mine).map(|i| {
                        let pop = if i + 1 == mine { fresh.map(|k| 1.3 - 0.3 * k).unwrap_or(1.) } else { 1. };
                        div().size(px(d * pop)).rounded(t.radius.full).bg(t.colors.vote_dot).border_2().border_color(t.colors.surface)
                    });
                    out.push(
                        div()
                            .absolute()
                            .left(px(10. * z))
                            .bottom(px(-d / 2.))
                            .flex()
                            .items_center()
                            .gap(px(2.))
                            .children(dots)
                            .into_any_element(),
                    );
                }
            }
            Some(_) => {
                let n = total(s);
                if n > 0 {
                    let d = (26. * z).max(18.);
                    out.push(
                        div()
                            .absolute()
                            .top(px(-d / 3.))
                            .right(px(-d / 3.))
                            .size(px(d))
                            .flex()
                            .items_center()
                            .justify_center()
                            .rounded(t.radius.full)
                            .bg(t.colors.vote_badge)
                            .border_2()
                            .border_color(t.colors.surface)
                            .text_size(px(d * 0.48))
                            .font_weight(gpui::FontWeight::BOLD)
                            .text_color(gpui::white())
                            .child(n.to_string())
                            .into_any_element(),
                    );
                }
            }
            None => {}
        }
        out
    }
}
