//! How things arrive and leave, the "Glide" set (`theme::Motion`): each
//! thing fades up a few pixels and settles with an ease-out, and leaves
//! the same way, faster, with an ease-in. Lists open row by row, a
//! dialog's backdrop dims in under it, and a page fades up when you go to
//! it. Closing keeps a copy of what closed on screen while it leaves
//! (`Exit`), which can't be clicked. With Settings › Reduce motion on,
//! everything just appears and goes.

use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use gpui::{div, prelude::*, px, Animation, AnimationExt as _, AnyElement, ElementId, Hsla, IntoElement, ParentElement, Styled};

use crate::model::{Category, Id, Sticker};
use crate::theme::{Motion, Theme};

static REDUCED: AtomicBool = AtomicBool::new(false);

/// Follows Settings › Reduce motion.
pub fn set_reduced(on: bool) {
    REDUCED.store(on, Ordering::Relaxed);
}

pub fn reduced() -> bool {
    REDUCED.load(Ordering::Relaxed)
}

/// How long `base` milliseconds of motion take: 1 with reduce motion on.
/// FOLIO_SLOW_MOTION=10 plays everything ten times slower, to look at
/// one frame of a transition (scripts/screenshots.sh can't catch it
/// otherwise).
pub fn ms(base: u64) -> u64 {
    if reduced() { 1 } else { base * slow() }
}

/// How many times slower than life (FOLIO_SLOW_MOTION, 1 without it).
pub fn slow() -> u64 {
    static SLOW: std::sync::OnceLock<u64> = std::sync::OnceLock::new();
    *SLOW.get_or_init(|| std::env::var("FOLIO_SLOW_MOTION").ok().and_then(|v| v.parse().ok()).unwrap_or(1).max(1))
}

/// Arriving: fast, then settling.
fn once(base: u64) -> Animation {
    Animation::new(Duration::from_millis(ms(base))).with_easing(gpui::ease_out_quint())
}

/// Leaving: slow to start, then quickly gone.
fn going(base: u64) -> Animation {
    Animation::new(Duration::from_millis(ms(base))).with_easing(|k: f32| k * k * k)
}

/// A row of a list opening: row `index` starts a beat after the one above
/// and rises into place. Rows past the eighth share its beat, so a long
/// list never keeps you waiting.
fn stagger(t: &Theme, index: usize) -> Animation {
    if reduced() {
        return once(1);
    }
    let rise = ms(t.motion.pop_in) as f32;
    let delay = ms(t.motion.stagger * index.min(8) as u64) as f32;
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

/// Something arriving over `ms`: it fades in while it rises `rise` pixels
/// into place (a negative `rise` drops it from above). Keyed by `id`: a
/// new key plays it again.
pub fn enter<E: Styled + IntoElement + 'static>(el: E, id: impl Into<ElementId>, ms: u64, rise: f32) -> AnyElement {
    el.with_animation(id, once(ms), move |d, k| d.opacity(k).mt(px(rise * (1. - k))))
        .into_any_element()
}

/// A popover, menu or the edit bar arriving.
pub fn rise_in<E: Styled + IntoElement + 'static>(el: E, id: impl Into<ElementId>, t: &Theme) -> AnyElement {
    enter(el, id, t.motion.pop_in, t.motion.rise)
}

/// A toast arriving from the right as it fades in. Its parent anchors it
/// to the right edge, so a negative right margin carries it out there.
pub fn slide_in<E: Styled + IntoElement + 'static>(el: E, id: impl Into<ElementId>, t: &Theme) -> AnyElement {
    let slide = t.motion.slide_toast;
    el.with_animation(id, once(t.motion.toast_in), move |d, k| d.opacity(k).mr(px(-slide * (1. - k))))
        .into_any_element()
}

/// Something that only fades in, such as a tooltip or a picture.
pub fn fade_in<E: Styled + IntoElement + 'static>(el: E, id: impl Into<ElementId>, ms: u64) -> AnyElement {
    el.with_animation(id, once(ms), |d, k| d.opacity(k)).into_any_element()
}

/// The old look lifting off the window as the new one shows: a sheet of
/// its colour, fading from `from` to nothing.
pub fn fade_off<E: Styled + IntoElement + 'static>(el: E, id: impl Into<ElementId>, ms: u64, from: f32) -> AnyElement {
    el.with_animation(id, once(ms), move |d, k| d.opacity(from * (1. - k))).into_any_element()
}

/// Something closing: it fades as it sinks `sink` pixels over `ms`, and
/// can't be clicked on the way out. `el` must be positioned (relative or
/// absolute), for the cover that keeps the clicks off it.
pub fn leave<E: Styled + ParentElement + IntoElement + 'static>(el: E, id: impl Into<ElementId>, ms: u64, sink: f32) -> AnyElement {
    el.child(div().absolute().inset_0().occlude())
        .with_animation(id, going(ms), move |d, k| d.opacity(1. - k).mt(px(sink * k)))
        .into_any_element()
}

/// A backdrop dimming in under a dialog or the search.
pub fn scrim<E: Styled + IntoElement + 'static>(el: E, id: impl Into<ElementId>, color: Hsla, t: &Theme) -> AnyElement {
    el.with_animation(id, once(t.motion.pop_in), move |d, k| d.bg(Hsla { a: color.a * k, ..color }))
        .into_any_element()
}

/// A backdrop lifting as what was over it leaves; it takes no clicks.
pub fn scrim_out<E: Styled + IntoElement + 'static>(el: E, id: impl Into<ElementId>, color: Hsla, ms: u64) -> AnyElement {
    el.with_animation(id, going(ms), move |d, k| d.bg(Hsla { a: color.a * (1. - k), ..color }))
        .into_any_element()
}

/// A page arriving when you go to it: it fades up into place.
pub fn page_in<E: Styled + IntoElement + 'static>(el: E, id: impl Into<ElementId>, t: &Theme) -> AnyElement {
    enter(el, id, t.motion.page_in, t.motion.rise)
}

/// What's showing in one place (a menu, a dialog, a toast...), followed
/// from one frame to the next so that when it closes, the last one stays
/// on screen while it leaves. Nothing about closing has to change: each
/// render says what's showing now.
pub struct Exit<T> {
    shown: Option<T>,
    gone: Option<(T, Instant)>,
    /// Counts the leavings, so each plays its own animation.
    serial: usize,
}

impl<T> Default for Exit<T> {
    fn default() -> Self {
        Exit { shown: None, gone: None, serial: 0 }
    }
}

impl<T: Clone> Exit<T> {
    /// What's showing now. True the moment it closes: the caller then asks
    /// for a frame once it has gone, so its cover doesn't linger.
    pub fn track(&mut self, now: Option<&T>) -> bool {
        match now {
            Some(v) => {
                self.shown = Some(v.clone());
                self.gone = None;
                false
            }
            None => match self.shown.take() {
                Some(v) if !reduced() => {
                    self.gone = Some((v, Instant::now()));
                    self.serial += 1;
                    true
                }
                _ => false,
            },
        }
    }

    /// What's leaving, for the `base` ms it takes, with a number that keys
    /// its animation.
    pub fn leaving(&mut self, base: u64) -> Option<(T, usize)> {
        match &self.gone {
            Some((v, at)) if at.elapsed() < Duration::from_millis(ms(base)) => Some((v.clone(), self.serial)),
            _ => {
                self.gone = None;
                None
            }
        }
    }
}

/// Notes arriving on the sheet on screen and leaving it, and pins landing
/// on them, followed from one frame to the next by what changed on the
/// sheet, whatever changed it (a new note, the Trash, undo, a move to
/// another sheet). Notes picked up and dropped are left to their own
/// motion.
#[derive(Default)]
pub struct NoteMotion {
    sheet: Option<Id>,
    /// Sums up the notes as they were last frame, to see cheaply whether
    /// anything changed.
    stamp: u64,
    last: Vec<Sticker>,
    born: HashMap<Id, Instant>,
    gone: Vec<(Sticker, Instant)>,
    /// When each changed pin starts landing (a new note's waits a beat).
    pins: HashMap<Id, Instant>,
    /// When a picture was added to a note, or swapped.
    pictures: HashMap<Id, Instant>,
}

impl NoteMotion {
    /// Looks at `sheet` this frame. True while anything is still moving,
    /// so the caller asks for another frame.
    pub fn follow(&mut self, sheet: &Category, m: &Motion) -> bool {
        let now = Instant::now();
        let stamp = sheet.stickers.iter().fold(sheet.stickers.len() as u64, |h, s| {
            let pin = s.pin.map_or(0, |p| 1 + ((p.shape as u64) << 8 | (p.size as u64) << 4 | p.tone as u64));
            let picture = s.image.as_ref().map_or(0, |i| i.bytes().fold(7u64, |h, b| h.wrapping_mul(31).wrapping_add(b as u64)));
            h.wrapping_mul(1_000_003).wrapping_add(s.id ^ s.edited.rotate_left(17) ^ pin.rotate_left(40) ^ picture)
        });
        if self.sheet != Some(sheet.id) {
            // Another sheet: it arrives as a page, its notes as they are.
            *self = NoteMotion { sheet: Some(sheet.id), stamp, last: sheet.stickers.clone(), ..Default::default() };
            return false;
        }
        if stamp != self.stamp {
            if !reduced() {
                let before: HashMap<Id, &Sticker> = self.last.iter().map(|s| (s.id, s)).collect();
                for s in &sheet.stickers {
                    match before.get(&s.id) {
                        None => {
                            self.born.insert(s.id, now);
                            if s.pin.is_some() {
                                self.pins.insert(s.id, now + Duration::from_millis(ms(m.pin_wait)));
                            }
                        }
                        Some(old) => {
                            if old.pin != s.pin && s.pin.is_some() {
                                self.pins.insert(s.id, now);
                            }
                            if old.image != s.image && s.image.is_some() {
                                self.pictures.insert(s.id, now);
                            }
                        }
                    }
                }
                let after: HashSet<Id> = sheet.stickers.iter().map(|s| s.id).collect();
                for s in self.last.iter().filter(|s| !after.contains(&s.id)) {
                    self.gone.push((s.clone(), now));
                }
            }
            self.stamp = stamp;
            self.last = sheet.stickers.clone();
        }
        let done = |at: &Instant, base: u64| now.saturating_duration_since(*at) >= Duration::from_millis(ms(base));
        self.born.retain(|_, at| !done(at, m.note_in));
        self.gone.retain(|(_, at)| !done(at, m.note_out));
        self.pins.retain(|_, at| !done(at, m.pin_in));
        self.pictures.retain(|_, at| !done(at, m.page_in));
        !self.born.is_empty() || !self.gone.is_empty() || !self.pins.is_empty() || !self.pictures.is_empty()
    }

    /// How far note `id` has come in, eased, while it arrives.
    pub fn arriving(&self, id: Id, m: &Motion) -> Option<f32> {
        let at = self.born.get(&id)?;
        Some(gpui::ease_out_quint()(progress(*at, m.note_in)))
    }

    /// How far note `id`'s pin has landed, eased, while it does.
    pub fn pin_landing(&self, id: Id, m: &Motion) -> Option<f32> {
        let at = self.pins.get(&id)?;
        Some(gpui::ease_out_quint()(progress(*at, m.pin_in)))
    }

    /// How far note `id`'s new picture has faded in, while it does.
    pub fn picture_showing(&self, id: Id, m: &Motion) -> Option<f32> {
        let at = self.pictures.get(&id)?;
        Some(gpui::ease_out_quint()(progress(*at, m.page_in)))
    }

    /// Notes that just went, each with how far it has gone, eased.
    pub fn leaving(&self, m: &Motion) -> Vec<(Sticker, f32)> {
        self.gone
            .iter()
            .map(|(s, at)| {
                let k = progress(*at, m.note_out);
                (s.clone(), k * k * k)
            })
            .collect()
    }
}

/// 0 to 1 over `base` ms from `at` (0 before it).
fn progress(at: Instant, base: u64) -> f32 {
    (Instant::now().saturating_duration_since(at).as_secs_f32() * 1000. / ms(base) as f32).clamp(0., 1.)
}
