//! Pins: what a note is pinned up with. Each shape is a picture in
//! `assets/pins`; the shapes that take a colour have it filled in from the
//! theme. Each shape in each colour is made once.

use std::cell::RefCell;
use std::collections::HashMap;
use std::sync::Arc;

use gpui::{div, img, prelude::*, px, AssetSource, Image, ImageFormat};

use crate::model::{Pin, PinShape, PinTone};
use crate::theme::Theme;

thread_local! {
    static MADE: RefCell<HashMap<(PinShape, PinTone), Arc<Image>>> = RefCell::new(HashMap::new());
}

/// `shape`'s picture, in `tone` if it takes a colour.
pub fn pin_image(shape: PinShape, tone: PinTone, t: &Theme) -> Option<Arc<Image>> {
    // A shape with colours of its own is the same picture in any tone.
    let tone = if shape.tone().is_some() { tone } else { PinTone::default() };
    if let Some(i) = MADE.with(|m| m.borrow().get(&(shape, tone)).cloned()) {
        return Some(i);
    }
    let bytes = crate::assets::Assets.load(&format!("pins/{}.svg", shape.key())).ok()??;
    let mut svg = String::from_utf8(bytes.into_owned()).ok()?;
    if shape.tone().is_some() {
        let [main, shade, light] = t.pin_tone(tone);
        svg = svg
            .replace("{main}", &format!("#{main:06x}"))
            .replace("{shade}", &format!("#{shade:06x}"))
            .replace("{light}", &format!("#{light:06x}"));
    }
    let image = Arc::new(Image::from_bytes(ImageFormat::Svg, svg.into_bytes()));
    MADE.with(|m| m.borrow_mut().insert((shape, tone), image.clone()));
    Some(image)
}

/// `pin` across the middle of the top edge of a note `w` px wide, at zoom
/// `z`: a push pin stands on its point, the rest sit across the edge.
pub fn pin_view(pin: Pin, w: f32, z: f32, t: &Theme) -> gpui::Div {
    let d = pin.size.units() * z;
    div()
        .absolute()
        .left(px(w / 2. - d / 2.))
        .top(px(-d * pin.shape.anchor()))
        .size(px(d))
        .children(pin_image(pin.shape, pin.tone, t).map(|i| img(i).size_full()))
}

/// `shape` at `d` px, for the pin picker.
pub fn pin_icon(shape: PinShape, tone: PinTone, d: f32, t: &Theme) -> gpui::Div {
    div().size(px(d)).flex_none().children(pin_image(shape, tone, t).map(|i| img(i).size_full()))
}
