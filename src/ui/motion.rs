//! How things arrive: popovers and menus rise into place as they fade in,
//! a popover's edge draws an accent line as it opens, lists open row by
//! row, a dialog's backdrop dims in under it, and a page fades up when you
//! go to it. With Settings › Reduce motion on, everything just appears.

use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use gpui::{div, prelude::*, px, relative, Animation, AnimationExt as _, AnyElement, ElementId, Hsla, IntoElement, Styled};

use crate::theme::Theme;

static REDUCED: AtomicBool = AtomicBool::new(false);

/// Follows Settings › Reduce motion.
pub fn set_reduced(on: bool) {
    REDUCED.store(on, Ordering::Relaxed);
}

pub fn reduced() -> bool {
    REDUCED.load(Ordering::Relaxed)
}

fn once(ms: u64) -> Animation {
    let ms = if reduced() { 1 } else { ms };
    Animation::new(Duration::from_millis(ms)).with_easing(gpui::ease_out_quint())
}

/// A row of a list opening: row `index` starts a beat after the one above
/// and rises into place. Rows past the eighth share its beat, so a long
/// list never keeps you waiting.
fn stagger(t: &Theme, index: usize) -> Animation {
    if reduced() {
        return once(1);
    }
    let rise = t.motion.pop_in as f32;
    let delay = (t.motion.stagger * index.min(8) as u64) as f32;
    let total = rise + delay;
    let ease = gpui::ease_out_quint();
    Animation::new(Duration::from_millis(total as u64)).with_easing(move |k| ease(((k * total - delay) / rise).clamp(0., 1.)))
}

/// A staggered row: fades in from a few pixels lower, a beat after the row
/// above. `id` names the opening, so a new list plays it again.
pub fn row_in(row: impl IntoElement + 'static, id: impl Into<ElementId>, index: usize, t: &Theme) -> AnyElement {
    div()
        .flex_none()
        .child(row)
        .with_animation(id, stagger(t, index), |d, k| d.opacity(k).mt(px(4. * (1. - k))).mb(px(-4. * (1. - k))))
        .into_any_element()
}

/// A card in a grid arriving, like `row_in`.
pub fn card_in(card: impl IntoElement + 'static, id: impl Into<ElementId>, index: usize, t: &Theme) -> AnyElement {
    div()
        .child(card)
        .with_animation(id, stagger(t, index), |d, k| d.opacity(k).mt(px(6. * (1. - k))).mb(px(-6. * (1. - k))))
        .into_any_element()
}

/// A popover, menu or toast arriving: it fades in while it rises `rise`
/// pixels into place. Keyed by `id`: a new key plays it again.
pub fn rise_in<E: Styled + IntoElement + 'static>(el: E, id: impl Into<ElementId>, rise: f32, t: &Theme) -> AnyElement {
    el.with_animation(id, once(t.motion.pop_in), move |d, k| d.opacity(k).mt(px(rise * (1. - k))))
        .into_any_element()
}

/// A popover drawing itself as it opens: an accent line runs down its left
/// edge while it rises into place and fades in, then the line fades away.
/// The caller makes `body` relative.
pub fn edge_draw(body: gpui::Stateful<gpui::Div>, id: impl Into<ElementId>, accent: Hsla, t: &Theme) -> AnyElement {
    let inset = f32::from(t.radius.card);
    let bar = t.motion.edge_bar;
    let draw_ms = t.motion.edge_draw as f32;
    let ms = t.motion.pop_in + t.motion.edge_draw;
    let anim = Animation::new(Duration::from_millis(if reduced() { 1 } else { ms }));
    let split = draw_ms / ms as f32;
    let ease = gpui::ease_out_quint();
    body.with_animation(id, anim, move |d, k| {
        let draw = ease((k / split).min(1.));
        let rise = ease((k / (1. - split)).min(1.));
        let fade = ((k - split) / (1. - split)).clamp(0., 1.);
        d.opacity(rise).mt(px(-3. * (1. - rise))).when(k < 1., |d| {
            d.child(
                div().absolute().left_0().top(px(inset)).bottom(px(inset)).w(px(bar)).child(
                    div().w_full().h(relative(draw)).rounded(px(bar)).bg(accent.opacity(1. - fade)),
                ),
            )
        })
    })
    .into_any_element()
}

/// A backdrop dimming in under a dialog or the search.
pub fn scrim<E: Styled + IntoElement + 'static>(el: E, id: impl Into<ElementId>, color: Hsla, t: &Theme) -> AnyElement {
    el.with_animation(id, once(t.motion.pop_in), move |d, k| d.bg(Hsla { a: color.a * k, ..color }))
        .into_any_element()
}

/// A page arriving when you go to it: a short fade up.
pub fn page_in<E: Styled + IntoElement + 'static>(el: E, id: impl Into<ElementId>, t: &Theme) -> AnyElement {
    el.with_animation(id, once(t.motion.pop_in + 60), move |d, k| d.opacity(0.35 + 0.65 * k).mt(px(6. * (1. - k))))
        .into_any_element()
}
