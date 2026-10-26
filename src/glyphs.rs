//! Words as outlines: the words in a shape, set in the bundled sans and
//! traced into polygons, so they can turn with the shape. GPUI draws text
//! upright only; a turned shape's words are filled as paths instead.
//! Plain geometry only; nothing here knows about GPUI.

use crate::model::{v2, Rect, Vec2};

/// The weight the words in a shape are set in.
pub const WEIGHT: u16 = 500;
/// The space from one line's top to the next's, in type sizes.
pub const LEADING: f32 = 1.3;

/// `text` broken into lines no wider than `width` at type size `size`:
/// at its own line breaks, then between words (a word wider than the line
/// gets a line of its own).
pub fn wrap(text: &str, size: f32, width: f32) -> Vec<String> {
    let mut out = Vec::new();
    for para in text.lines() {
        let mut line = String::new();
        for word in para.split_whitespace() {
            let next = if line.is_empty() { word.to_string() } else { format!("{line} {word}") };
            if !line.is_empty() && advance(&next, size) > width {
                out.push(std::mem::replace(&mut line, word.to_string()));
            } else {
                line = next;
            }
        }
        out.push(line);
    }
    while out.last().is_some_and(String::is_empty) {
        out.pop();
    }
    out
}

/// How wide `s` is at type size `size`.
pub fn advance(s: &str, size: f32) -> f32 {
    let f = crate::picture::sans(WEIGHT);
    let em = f32::from(f.units_per_em());
    s.chars()
        .map(|c| f.glyph_index(c).and_then(|g| f.glyph_hor_advance(g)).map_or(0.6, |a| f32::from(a) / em))
        .sum::<f32>()
        * size
}

/// The outlines of `text` at type size `size`, wrapped to `r` less `pad`
/// each side and centred in it, upright, in world units. Each is one
/// closed contour; filled even-odd, they make the letters.
pub fn outlines(text: &str, size: f32, r: Rect, pad: f32) -> Vec<Vec<Vec2>> {
    let f = crate::picture::sans(WEIGHT);
    let em = f32::from(f.units_per_em());
    let k = size / em;
    let lines = wrap(text, size, (r.w - 2. * pad).max(size));
    let lh = size * LEADING;
    let (up, down) = (f32::from(f.ascender()) * k, -f32::from(f.descender()) * k);
    let top = r.y + (r.h - lh * lines.len() as f32) / 2.;
    let mut out = Vec::new();
    for (i, line) in lines.iter().enumerate() {
        let base = top + lh * i as f32 + (lh - up - down) / 2. + up;
        let mut x = r.x + (r.w - advance(line, size)) / 2.;
        for c in line.chars() {
            let Some(g) = f.glyph_index(c) else {
                x += 0.6 * size;
                continue;
            };
            let mut t = Tracer { at: v2(x, base), k, contours: Vec::new(), now: Vec::new(), last: v2(0., 0.) };
            f.outline_glyph(g, &mut t);
            t.end();
            out.extend(t.contours);
            x += f.glyph_hor_advance(g).map_or(0., |a| f32::from(a) * k);
        }
    }
    out
}

/// Traces a glyph into polygons: font units, y up, from the pen at `at`.
struct Tracer {
    at: Vec2,
    k: f32,
    contours: Vec<Vec<Vec2>>,
    now: Vec<Vec2>,
    last: Vec2,
}

impl Tracer {
    fn put(&mut self, x: f32, y: f32) -> Vec2 {
        v2(self.at.x + x * self.k, self.at.y - y * self.k)
    }
    fn end(&mut self) {
        if self.now.len() > 2 {
            self.contours.push(std::mem::take(&mut self.now));
        }
        self.now.clear();
    }
}

impl ttf_parser::OutlineBuilder for Tracer {
    fn move_to(&mut self, x: f32, y: f32) {
        self.end();
        self.last = self.put(x, y);
        self.now.push(self.last);
    }
    fn line_to(&mut self, x: f32, y: f32) {
        self.last = self.put(x, y);
        self.now.push(self.last);
    }
    fn quad_to(&mut self, x1: f32, y1: f32, x: f32, y: f32) {
        let (a, b, c) = (self.last, self.put(x1, y1), self.put(x, y));
        for i in 1..=6 {
            let t = i as f32 / 6.;
            let u = 1. - t;
            self.now.push(v2(u * u * a.x + 2. * u * t * b.x + t * t * c.x, u * u * a.y + 2. * u * t * b.y + t * t * c.y));
        }
        self.last = c;
    }
    fn curve_to(&mut self, x1: f32, y1: f32, x2: f32, y2: f32, x: f32, y: f32) {
        let (a, b, c, d) = (self.last, self.put(x1, y1), self.put(x2, y2), self.put(x, y));
        for i in 1..=8 {
            let t = i as f32 / 8.;
            let u = 1. - t;
            let (p, q, r, s) = (u * u * u, 3. * u * u * t, 3. * u * t * t, t * t * t);
            self.now.push(v2(p * a.x + q * b.x + r * c.x + s * d.x, p * a.y + q * b.y + r * c.y + s * d.y));
        }
        self.last = d;
    }
    fn close(&mut self) {
        self.end();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn long_text_wraps_between_words() {
        let lines = wrap("Check the router first\nthen the cable", 16., 120.);
        assert!(lines.len() >= 3, "{lines:?}");
        assert_eq!(lines.last().unwrap(), "then the cable");
        assert!(lines.iter().all(|l| advance(l, 16.) <= 120. || !l.contains(' ')));
    }

    #[test]
    fn words_trace_into_closed_letters_inside_the_box() {
        let r = Rect { x: 0., y: 0., w: 160., h: 96. };
        let o = outlines("Router", 16., r, 8.);
        // R has a hole: more contours than letters.
        assert!(o.len() > 6, "{}", o.len());
        for p in o.iter().flatten() {
            assert!(p.x > 0. && p.x < 160. && p.y > 30. && p.y < 66., "{p:?}");
        }
        assert!(outlines("", 16., r, 8.).is_empty());
    }
}
