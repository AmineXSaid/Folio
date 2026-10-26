//! A sheet as a picture to share: a PNG, or a PDF that stays sharp at any
//! size. The sheet is drawn once as SVG, in world units, the way the board
//! draws it: its paper and pattern, areas, links and ink, its name, text on
//! the sheet, and the notes with their paper, pins, tape and shadows.
//! resvg turns that into pixels; [`pdf`] writes the same shapes as a
//! one-page vector PDF, the words as outlines.

use std::collections::HashMap;
use std::fmt::Write as _;
use std::path::Path;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, OnceLock};

use base64::Engine;
use gpui::{AssetSource, BoxShadow, Hsla};
use resvg::{tiny_skia, usvg};

use crate::markdown::{blocks, ruled_line, Block};
use crate::model::*;
use crate::shape::{SHAPE_TEXT, TEXT_PAD};
use crate::theme::Theme;

/// The room around what's on the sheet, in world units.
const MARGIN: f32 = 56.;
/// A PNG is drawn at twice the sheet's size, as a sharp screen shows it,
/// at most this many pixels across or down, and this many in all.
const PNG_SCALE: f32 = 2.;
const PNG_SIDE: f32 = 8192.;
const PNG_PIXELS: f32 = 48e6;
/// A world unit in PDF points (a CSS pixel), and the largest page side
/// PDF readers take.
const PT: f32 = 0.75;
const PDF_SIDE: f32 = 14400.;
/// A picture in a PDF keeps at most this many pixels across.
const PDF_IMAGE: u32 = 2400;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Format {
    Png,
    Pdf,
}

impl Format {
    pub fn ext(self) -> &'static str {
        match self {
            Format::Png => "png",
            Format::Pdf => "pdf",
        }
    }
}

/// `sheet` as a `format` file in `t`'s colours, its pictures from `media`.
/// `[[links]]` in its notes should already be written as Markdown links.
pub fn render(sheet: &Category, t: &Theme, media: &Path, format: Format) -> Result<Vec<u8>, String> {
    let svg = draw(sheet, t, media, format == Format::Png);
    let opt = usvg::Options { fontdb: fonts(), resources_dir: Some(media.to_path_buf()), ..Default::default() };
    let tree = usvg::Tree::from_str(&svg, &opt).map_err(|e| e.to_string())?;
    match format {
        Format::Png => png(&tree),
        Format::Pdf => Ok(pdf(&tree, &sheet.name)),
    }
}

// ---------- Type ----------

const SANS: &str = "Hanken Grotesk";
const HAND: &str = "Comic Neue";
const MONO: &str = "JetBrains Mono";

/// The bundled faces: drawn with, and measured to wrap lines.
const FACES: [(&str, u16, bool, &[u8]); 8] = [
    (SANS, 400, false, include_bytes!("../assets/fonts/HankenGrotesk-Regular.ttf")),
    (SANS, 500, false, include_bytes!("../assets/fonts/HankenGrotesk-Medium.ttf")),
    (SANS, 600, false, include_bytes!("../assets/fonts/HankenGrotesk-SemiBold.ttf")),
    (SANS, 700, false, include_bytes!("../assets/fonts/HankenGrotesk-Bold.ttf")),
    (SANS, 400, true, include_bytes!("../assets/fonts/HankenGrotesk-Italic.ttf")),
    (HAND, 400, false, include_bytes!("../assets/fonts/ComicNeue-Regular.otf")),
    (HAND, 700, false, include_bytes!("../assets/fonts/ComicNeue-Bold.otf")),
    (MONO, 400, false, include_bytes!("../assets/fonts/JetBrainsMono-Regular.ttf")),
];

/// The bundled faces, then the system's for what they lack (emoji, other
/// scripts). Read once.
fn fonts() -> Arc<usvg::fontdb::Database> {
    static DB: OnceLock<Arc<usvg::fontdb::Database>> = OnceLock::new();
    DB.get_or_init(|| {
        let mut db = usvg::fontdb::Database::new();
        for (_, _, _, data) in FACES {
            db.load_font_data(data.to_vec());
        }
        db.load_system_fonts();
        db.set_sans_serif_family(SANS);
        Arc::new(db)
    })
    .clone()
}

fn family(f: NoteFont) -> &'static str {
    match f {
        NoteFont::Sans => SANS,
        NoteFont::Hand => HAND,
        NoteFont::Mono => MONO,
    }
}

/// The bundled face nearest `family` at `weight`: slanted first if asked,
/// as a font is chosen when drawing.
fn face(family: &str, weight: u16, italic: bool) -> &'static ttf_parser::Face<'static> {
    static PARSED: OnceLock<Vec<ttf_parser::Face<'static>>> = OnceLock::new();
    let all = PARSED.get_or_init(|| FACES.iter().map(|f| ttf_parser::Face::parse(f.3, 0).expect("a bundled font")).collect());
    let best = FACES
        .iter()
        .enumerate()
        .filter(|(_, f)| f.0 == family)
        .min_by_key(|(_, f)| u32::from(f.2 != italic) * 1000 + (i32::from(f.1) - i32::from(weight)).unsigned_abs())
        .map_or(0, |(i, _)| i);
    &all[best]
}

/// The sans face at `weight`, for drawing words as outlines elsewhere
/// (`glyphs.rs`).
pub(crate) fn sans(weight: u16) -> &'static ttf_parser::Face<'static> {
    face(SANS, weight, false)
}

#[derive(Clone, Copy, PartialEq)]
struct Font {
    family: &'static str,
    size: f32,
    weight: u16,
    italic: bool,
}

impl Font {
    fn new(family: &'static str, size: f32, weight: u16) -> Font {
        Font { family, size, weight, italic: false }
    }

    fn width(&self, s: &str) -> f32 {
        let f = face(self.family, self.weight, self.italic);
        let em = f32::from(f.units_per_em());
        let units: f32 = s
            .chars()
            .map(|c| match f.glyph_index(if c == '\t' { ' ' } else { c }) {
                Some(g) => f32::from(f.glyph_hor_advance(g).unwrap_or(0)) / em,
                // Emoji and scripts the face lacks come from another font.
                None => 1.15,
            })
            .sum();
        units * self.size
    }

    /// From the top of a line `lh` high down to its baseline: the text
    /// sits in the middle of its line, as the board sets it.
    fn baseline(&self, lh: f32) -> f32 {
        let f = face(self.family, self.weight, self.italic);
        let em = f32::from(f.units_per_em());
        let (up, down) = (f32::from(f.ascender()) / em * self.size, -f32::from(f.descender()) / em * self.size);
        (lh - up - down) / 2. + up
    }
}

// ---------- Colour ----------

#[derive(Clone, Copy, PartialEq, Debug)]
struct Rgba {
    r: u8,
    g: u8,
    b: u8,
    a: f32,
}

fn rgba(h: Hsla) -> Rgba {
    let c = h.to_rgb();
    let byte = |v: f32| (v.clamp(0., 1.) * 255.).round() as u8;
    Rgba { r: byte(c.r), g: byte(c.g), b: byte(c.b), a: c.a.clamp(0., 1.) }
}

fn hex(v: u32) -> Rgba {
    Rgba { r: (v >> 16) as u8, g: (v >> 8) as u8, b: v as u8, a: 1. }
}

impl Rgba {
    fn alpha(self, k: f32) -> Rgba {
        Rgba { a: self.a * k, ..self }
    }
    fn css(&self) -> String {
        format!("#{:02x}{:02x}{:02x}", self.r, self.g, self.b)
    }
    /// `fill="…"` (or `stroke`), with its opacity when it has one.
    fn attr(&self, what: &str) -> String {
        if self.a >= 0.999 {
            format!(r#"{what}="{}""#, self.css())
        } else {
            format!(r#"{what}="{}" {what}-opacity="{}""#, self.css(), n(self.a))
        }
    }
}

/// The colours a sheet is drawn in, from the theme.
struct Look {
    dark: bool,
    text: Rgba,
    body: Rgba,
    muted: Rgba,
    subtle: Rgba,
    brand: Rgba,
    brand_on_tint: Rgba,
    brand_tint: Rgba,
    wash: Rgba,
    rule: Rgba,
    ruled: Rgba,
    margin: Rgba,
    head_rule: Rgba,
    curl: Rgba,
    tape: Rgba,
    tape_edge: Rgba,
    edge: Rgba,
    chip: Rgba,
    dot: Rgba,
    line: Rgba,
}

impl Look {
    fn of(t: &Theme) -> Look {
        let c = &t.colors;
        Look {
            dark: t.is_dark,
            text: rgba(c.text),
            body: rgba(c.text_body),
            muted: rgba(c.text_muted),
            subtle: rgba(c.text_subtle),
            brand: rgba(c.brand),
            brand_on_tint: rgba(c.brand_on_tint),
            brand_tint: rgba(c.brand_tint),
            wash: rgba(c.wash),
            rule: rgba(c.rule),
            ruled: rgba(c.ruled),
            margin: rgba(c.margin),
            head_rule: rgba(c.head_rule),
            curl: rgba(c.curl),
            tape: rgba(c.tape),
            tape_edge: rgba(c.tape_edge),
            edge: if t.is_dark { rgba(c.border) } else { Rgba { r: 0, g: 0, b: 0, a: 0.04 } },
            chip: if t.is_dark { Rgba { r: 255, g: 255, b: 255, a: 0.06 } } else { Rgba { r: 0, g: 0, b: 0, a: 0.06 } },
            dot: rgba(c.canvas_dot),
            line: rgba(c.canvas_line),
        }
    }

    /// A highlight's colour: a name from the theme's set, `#rrggbb`, or
    /// yellow when none is given.
    fn mark(&self, color: Option<&str>) -> Rgba {
        let named = |name: &str| crate::theme::MARKS.iter().find(|m| m.0 == name).map(|m| hex(if self.dark { m.2 } else { m.1 }));
        match color.map(str::trim) {
            Some(c) if c.starts_with('#') && c.len() == 7 => u32::from_str_radix(&c[1..], 16).map(hex).unwrap_or_else(|_| self.mark(None)),
            Some(c) => named(&c.to_ascii_lowercase()).unwrap_or_else(|| self.mark(None)),
            None => named("yellow").unwrap_or(self.brand_tint),
        }
    }
}

// ---------- SVG ----------

/// A number for SVG and PDF: short, with no trailing zeros.
fn n(v: f32) -> String {
    let s = format!("{:.3}", if v.abs() < 0.0005 { 0. } else { v });
    let s = s.trim_end_matches('0').trim_end_matches('.');
    if s == "-0" {
        "0".into()
    } else {
        s.into()
    }
}

fn esc(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&apos;"),
            '\t' => out.push(' '),
            c if c.is_control() => {}
            c => out.push(c),
        }
    }
    out
}

/// A rectangle with its top and bottom corners rounded `rt` and `rb`.
fn rrect(x: f32, y: f32, w: f32, h: f32, rt: f32, rb: f32) -> String {
    let rt = rt.min(w / 2.).min(h / 2.).max(0.);
    let rb = rb.min(w / 2.).min(h / 2.).max(0.);
    format!(
        "M{} {}H{}A{rt} {rt} 0 0 1 {} {}V{}A{rb} {rb} 0 0 1 {} {}H{}A{rb} {rb} 0 0 1 {} {}V{}A{rt} {rt} 0 0 1 {} {}Z",
        n(x + rt),
        n(y),
        n(x + w - rt),
        n(x + w),
        n(y + rt),
        n(y + h - rb),
        n(x + w - rb),
        n(y + h),
        n(x + rb),
        n(x),
        n(y + h - rb),
        n(y + rt),
        n(x + rt),
        n(y),
        rt = n(rt),
        rb = n(rb),
    )
}

fn data_uri(mime: &str, bytes: &[u8]) -> String {
    format!("data:{mime};base64,{}", base64::engine::general_purpose::STANDARD.encode(bytes))
}

/// The SVG being written: shapes, and the gradients, clips and blurs
/// they use.
struct Svg {
    body: String,
    defs: String,
    /// Shadows blurred (for pixels), or built of soft layers (for PDF,
    /// which has no blur).
    blur: bool,
    blurs: HashMap<u32, String>,
    icons: HashMap<(String, String), String>,
}

impl Svg {
    fn new(blur: bool) -> Svg {
        Svg { body: String::new(), defs: String::new(), blur, blurs: HashMap::new(), icons: HashMap::new() }
    }

    /// A name for a gradient, clip or blur, unique among all drawn.
    fn id(&mut self, what: &str) -> String {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        format!("{what}{}", NEXT.fetch_add(1, Ordering::Relaxed))
    }

    /// Another's shapes and what they use, drawn over these.
    fn add(&mut self, other: Svg) {
        self.body.push_str(&other.body);
        self.defs.push_str(&other.defs);
    }

    fn fill(&mut self, d: &str, c: Rgba) {
        if c.a > 0.001 {
            let _ = writeln!(self.body, r#"<path d="{d}" {}/>"#, c.attr("fill"));
        }
    }

    fn rect(&mut self, x: f32, y: f32, w: f32, h: f32, r: f32, c: Rgba) {
        if w > 0. && h > 0. {
            self.fill(&rrect(x, y, w, h, r, r), c);
        }
    }

    fn line(&mut self, x1: f32, y1: f32, x2: f32, y2: f32, c: Rgba, width: f32, dash: Option<f32>) {
        let dash = dash.map(|d| format!(r#" stroke-dasharray="{} {}""#, n(d), n(d))).unwrap_or_default();
        let _ = writeln!(
            self.body,
            r#"<path d="M{} {}L{} {}" fill="none" {} stroke-width="{}"{dash}/>"#,
            n(x1),
            n(y1),
            n(x2),
            n(y2),
            c.attr("stroke"),
            n(width)
        );
    }

    /// A box (its foot rounded `rb`) filled from `top` down to `bottom`.
    /// For PDF, bands that step from one to the other.
    fn gradient(&mut self, r: Rect, rb: f32, top: Rgba, bottom: Rgba) {
        if !self.blur {
            const BANDS: usize = 14;
            let step = r.h / BANDS as f32;
            for i in 0..BANDS {
                let k = (i as f32 + 0.5) / BANDS as f32;
                let c = Rgba { a: top.a + (bottom.a - top.a) * k, ..bottom };
                let last = i + 1 == BANDS;
                self.fill(&rrect(r.x, r.y + i as f32 * step, r.w, step + if last { 0. } else { 0.3 }, 0., if last { rb } else { 0. }), c);
            }
            return;
        }
        let d = rrect(r.x, r.y, r.w, r.h, 0., rb);
        let id = self.id("g");
        let _ = writeln!(
            self.defs,
            r#"<linearGradient id="{id}" x1="0" y1="0" x2="0" y2="1"><stop offset="0" stop-color="{}" stop-opacity="{}"/><stop offset="1" stop-color="{}" stop-opacity="{}"/></linearGradient>"#,
            top.css(),
            n(top.a),
            bottom.css(),
            n(bottom.a)
        );
        let _ = writeln!(self.body, r#"<path d="{d}" fill="url(#{id})"/>"#);
    }

    fn text(&mut self, x: f32, base: f32, s: &str, f: Font, c: Rgba) {
        if s.trim().is_empty() {
            return;
        }
        let style = if f.italic { r#" font-style="italic""# } else { "" };
        let _ = writeln!(
            self.body,
            r#"<text x="{}" y="{}" font-family="{}" font-size="{}" font-weight="{}"{style} {} xml:space="preserve">{}</text>"#,
            n(x),
            n(base),
            f.family,
            n(f.size),
            f.weight,
            c.attr("fill"),
            esc(s)
        );
    }

    /// A picture file filling the box, cropped to it, and to `clip` when
    /// given (a path).
    fn image(&mut self, href: &str, x: f32, y: f32, w: f32, h: f32, clip: Option<String>) {
        let clip = clip.map(|d| {
            let id = self.id("c");
            let _ = writeln!(self.defs, r#"<clipPath id="{id}"><path d="{d}"/></clipPath>"#);
            format!(r#" clip-path="url(#{id})""#)
        });
        let _ = writeln!(
            self.body,
            r#"<g{}><image href="{}" x="{}" y="{}" width="{}" height="{}" preserveAspectRatio="xMidYMid slice"/></g>"#,
            clip.unwrap_or_default(),
            esc(href),
            n(x),
            n(y),
            n(w),
            n(h)
        );
    }

    /// One of the bundled icons (`icons/NAME.svg`), `size` square, in `c`.
    fn icon(&mut self, name: &str, x: f32, y: f32, size: f32, c: Rgba) {
        let key = (name.to_string(), c.css());
        let uri = match self.icons.get(&key) {
            Some(u) => u.clone(),
            None => {
                let Some(svg) = asset(&format!("icons/{name}.svg")) else { return };
                let svg = svg.replace("currentColor", &c.css());
                let u = data_uri("image/svg+xml", svg.as_bytes());
                self.icons.insert(key, u.clone());
                u
            }
        };
        let _ = writeln!(
            self.body,
            r#"<image href="{uri}" x="{}" y="{}" width="{}" height="{}" opacity="{}"/>"#,
            n(x),
            n(y),
            n(size),
            n(size),
            n(c.a)
        );
    }

    /// Box shadows under a box `r` with corners `rt`/`rb`.
    fn shadows(&mut self, r: Rect, rt: f32, rb: f32, shadows: &[BoxShadow]) {
        let Rect { x, y, w, h } = r;
        for s in shadows.iter().filter(|s| !s.inset) {
            let c = rgba(s.color);
            let (dx, dy) = (f32::from(s.offset.x), f32::from(s.offset.y));
            let (blur, spread) = (f32::from(s.blur_radius), f32::from(s.spread_radius));
            let grow = |e: f32| rrect(x + dx - e, y + dy - e, w + 2. * e, h + 2. * e, (rt + e).max(0.), (rb + e).max(0.));
            if blur <= 0.5 {
                self.fill(&grow(spread), c);
            } else if self.blur {
                // A box shadow's blur is two standard deviations.
                let sigma = blur / 2.;
                let key = (sigma * 100.).round() as u32;
                let id = match self.blurs.get(&key) {
                    Some(id) => id.clone(),
                    None => {
                        let id = self.id("b");
                        let _ = writeln!(
                            self.defs,
                            r#"<filter id="{id}" x="-50%" y="-50%" width="200%" height="200%"><feGaussianBlur stdDeviation="{}"/></filter>"#,
                            n(sigma)
                        );
                        self.blurs.insert(key, id.clone());
                        id
                    }
                };
                let _ = writeln!(self.body, r#"<path d="{}" {} filter="url(#{id})"/>"#, grow(spread), c.attr("fill"));
            } else {
                // Layers growing outward, each faint: together a soft
                // edge as wide as the blur, as dark as the shadow inside.
                const LAYERS: usize = 8;
                let each = 1. - (1. - c.a).powf(1. / LAYERS as f32);
                for i in 0..LAYERS {
                    let e = spread + blur * ((i as f32 + 0.5) / LAYERS as f32 - 0.5) * 1.6;
                    self.fill(&grow(e), c.alpha(each / c.a.max(0.001)));
                }
            }
        }
    }

    fn open(&mut self, x: f32, y: f32) {
        let _ = writeln!(self.body, r#"<g transform="translate({} {})">"#, n(x), n(y));
    }

    fn close(&mut self) {
        self.body.push_str("</g>\n");
    }

    fn finish(self, bounds: Rect) -> String {
        format!(
            r#"<svg xmlns="http://www.w3.org/2000/svg" width="{}" height="{}" viewBox="{} {} {} {}"><defs>{}</defs>{}</svg>"#,
            n(bounds.w),
            n(bounds.h),
            n(bounds.x),
            n(bounds.y),
            n(bounds.w),
            n(bounds.h),
            self.defs,
            self.body
        )
    }
}

fn asset(path: &str) -> Option<String> {
    let bytes = crate::assets::Assets.load(path).ok()??;
    String::from_utf8(bytes.into_owned()).ok()
}

// ---------- Words ----------

#[derive(Clone, Copy, Default, PartialEq)]
struct Style {
    bold: bool,
    italic: bool,
    code: bool,
    strike: bool,
    link: bool,
    mark: Option<(u8, u8, u8, u8)>,
}

struct Run {
    text: String,
    style: Style,
}

/// A paragraph of Markdown as runs of one style each: **bold**, *italic*,
/// `code`, ~~struck~~, `<mark color="…">` highlights and [links](…), the
/// marks themselves taken out.
fn runs(md: &str, look: &Look) -> Vec<Run> {
    let mut out: Vec<Run> = Vec::new();
    let mut style = Style::default();
    let mut buf = String::new();
    let flush = |buf: &mut String, out: &mut Vec<Run>, style: Style| {
        if !buf.is_empty() {
            out.push(Run { text: std::mem::take(buf), style });
        }
    };
    let s = md;
    let mut i = 0;
    let at = |i: usize, p: &str| s[i..].starts_with(p);
    let prev = |i: usize| s[..i].chars().next_back();
    while i < s.len() {
        let c = s[i..].chars().next().expect("in the string");
        if style.code {
            if c == '`' {
                flush(&mut buf, &mut out, style);
                style.code = false;
            } else {
                buf.push(c);
            }
            i += c.len_utf8();
            continue;
        }
        // Whether a mark at i opens (something follows it, and it closes
        // later) or closes (it follows something).
        let opens = |i: usize, len: usize, mark: &str| s[i + len..].chars().next().is_some_and(|c| !c.is_whitespace()) && s[i + len..].contains(mark);
        let closes = |i: usize| prev(i).is_some_and(|c| !c.is_whitespace());
        if c == '\\' && i + 1 < s.len() {
            let next = s[i + 1..].chars().next().expect("a char");
            if next.is_ascii_punctuation() {
                buf.push(next);
                i += 1 + next.len_utf8();
                continue;
            }
        }
        if c == '`' && s[i + 1..].contains('`') {
            flush(&mut buf, &mut out, style);
            style.code = true;
            i += 1;
            continue;
        }
        let double = [("**", true), ("__", true), ("~~", false)].into_iter().find(|(mark, bold)| {
            let on = if *bold { style.bold } else { style.strike };
            at(i, mark) && ((on && closes(i)) || (!on && opens(i, 2, mark)))
        });
        if let Some((_, bold)) = double {
            flush(&mut buf, &mut out, style);
            if bold {
                style.bold = !style.bold;
            } else {
                style.strike = !style.strike;
            }
            i += 2;
            continue;
        }
        // A single * or _ (not inside a word) slants.
        let underscore = at(i, "_") && (style.italic || !prev(i).is_some_and(char::is_alphanumeric));
        if at(i, "*") || underscore {
            let mark = &s[i..i + 1];
            if (style.italic && closes(i)) || (!style.italic && opens(i, 1, mark)) {
                flush(&mut buf, &mut out, style);
                style.italic = !style.italic;
                i += 1;
                continue;
            }
        }
        if at(i, "<mark") {
            if let Some(end) = s[i..].find('>') {
                let tag = &s[i..i + end];
                let color = tag.split("color=").nth(1).map(|v| v.trim_matches(|c| c == '"' || c == '\'' || c == ' ' || c == '/'));
                flush(&mut buf, &mut out, style);
                let m = look.mark(color);
                style.mark = Some((m.r, m.g, m.b, (m.a * 255.) as u8));
                i += end + 1;
                continue;
            }
        }
        if at(i, "</mark>") {
            flush(&mut buf, &mut out, style);
            style.mark = None;
            i += "</mark>".len();
            continue;
        }
        // Other tags (<br>, <u>…) aren't drawn.
        if c == '<' {
            if let Some(end) = s[i..].find('>') {
                let tag = s[i + 1..i + end].trim_start_matches('/');
                let name = tag.split(|c: char| !c.is_ascii_alphanumeric()).next().unwrap_or("");
                if TAGS.contains(&name.to_ascii_lowercase().as_str()) {
                    i += end + 1;
                    continue;
                }
            }
        }
        // [words](address): the words, as a link.
        if c == '[' && !at(i, "[[") {
            if let Some(mid) = s[i..].find("](") {
                if let Some(end) = s[i + mid..].find(')') {
                    let words = &s[i + 1..i + mid];
                    flush(&mut buf, &mut out, style);
                    for r in runs(words, look) {
                        out.push(Run { text: r.text, style: Style { link: true, ..merge(style, r.style) } });
                    }
                    i += mid + end + 1;
                    continue;
                }
            }
        }
        buf.push(c);
        i += c.len_utf8();
    }
    flush(&mut buf, &mut out, style);
    out
}

/// HTML tags a note may hold that are left out of the picture.
const TAGS: [&str; 16] = ["br", "u", "b", "i", "em", "strong", "s", "del", "ins", "sub", "sup", "span", "small", "kbd", "p", "div"];

fn merge(a: Style, b: Style) -> Style {
    Style {
        bold: a.bold || b.bold,
        italic: a.italic || b.italic,
        code: a.code || b.code,
        strike: a.strike || b.strike,
        link: a.link || b.link,
        mark: b.mark.or(a.mark),
    }
}

/// How a block of words is set: its face, size, line and colour.
#[derive(Clone, Copy)]
struct Set {
    family: &'static str,
    size: f32,
    lh: f32,
    weight: u16,
    italic: bool,
    color: Rgba,
}

impl Set {
    fn font(&self, s: Style) -> Font {
        if s.code {
            return Font { family: MONO, size: self.size * 0.92, weight: 400, italic: false };
        }
        Font { family: self.family, size: self.size, weight: if s.bold { 700.max(self.weight) } else { self.weight }, italic: self.italic || s.italic }
    }
}

/// A piece of a line, at `x` from the line's start.
struct Piece {
    x: f32,
    w: f32,
    text: String,
    style: Style,
}

/// Runs broken into lines no wider than `width`: at spaces, or inside a
/// word too long for a line of its own.
fn wrap(runs: &[Run], set: &Set, width: f32) -> Vec<Vec<Piece>> {
    let mut lines: Vec<Vec<Piece>> = vec![Vec::new()];
    let mut x = 0.;
    let mut space: Option<(f32, Style)> = None;
    let push = |lines: &mut Vec<Vec<Piece>>, x: f32, w: f32, text: &str, style: Style| {
        let line = lines.last_mut().expect("a line");
        match line.last_mut() {
            Some(p) if p.style == style && (p.x + p.w - x).abs() < 0.01 => {
                p.text.push_str(text);
                p.w += w;
            }
            _ => line.push(Piece { x, w, text: text.to_string(), style }),
        }
    };
    for run in runs {
        let font = set.font(run.style);
        let mut rest = run.text.as_str();
        while !rest.is_empty() {
            let blank = rest.starts_with(char::is_whitespace);
            let len = rest.find(|c: char| c.is_whitespace() != blank).unwrap_or(rest.len());
            let (tok, after) = rest.split_at(len);
            rest = after;
            if blank {
                if x > 0. {
                    space = Some((font.width(" "), run.style));
                }
                continue;
            }
            let w = font.width(tok);
            let gap = space.map_or(0., |s| s.0);
            if x > 0. && x + gap + w > width {
                lines.push(Vec::new());
                x = 0.;
                space = None;
            }
            if let Some((sw, st)) = space.take() {
                push(&mut lines, x, sw, " ", st);
                x += sw;
            }
            if w <= width || x > 0. {
                push(&mut lines, x, w, tok, run.style);
                x += w;
                continue;
            }
            // Too long for any line: broken where it fills one.
            for ch in tok.chars() {
                let cw = font.width(ch.encode_utf8(&mut [0; 4]));
                if x > 0. && x + cw > width {
                    lines.push(Vec::new());
                    x = 0.;
                }
                push(&mut lines, x, cw, ch.encode_utf8(&mut [0; 4]), run.style);
                x += cw;
            }
        }
    }
    lines
}

/// Lines of pieces drawn from (`x`, `y`), `set.lh` apart. Returns the height.
fn draw_lines(svg: &mut Svg, lines: &[Vec<Piece>], x: f32, y: f32, set: &Set, look: &Look) -> f32 {
    for (i, line) in lines.iter().enumerate() {
        let top = y + i as f32 * set.lh;
        for p in line {
            let font = set.font(p.style);
            let base = top + font.baseline(set.lh);
            let px = x + p.x;
            if let Some((r, g, b, a)) = p.style.mark {
                let h = set.size * 1.3;
                svg.rect(px - 1., top + (set.lh - h) / 2., p.w + 2., h, 3., Rgba { r, g, b, a: f32::from(a) / 255. });
            }
            if p.style.code {
                let h = set.size * 1.3;
                svg.rect(px - 2., top + (set.lh - h) / 2., p.w + 4., h, 3., look.wash);
            }
            let color = if p.style.link { look.brand } else { set.color };
            svg.text(px, base, &p.text, font, color);
            let thick = (set.size / 14.).max(0.75);
            if p.style.strike {
                svg.line(px, base - set.size * 0.3, px + p.w, base - set.size * 0.3, color, thick, None);
            }
            if p.style.link && !p.text.trim().is_empty() {
                svg.line(px, base + set.size * 0.14, px + p.w, base + set.size * 0.14, color.alpha(0.5), thick, None);
            }
        }
    }
    lines.len() as f32 * set.lh
}

/// A paragraph of Markdown set in `width` at (`x`, `y`). Returns its height.
fn paragraph(svg: &mut Svg, md: &str, x: f32, y: f32, width: f32, set: &Set, look: &Look) -> f32 {
    let lines = wrap(&runs(md, look), set, width.max(1.));
    draw_lines(svg, &lines, x, y, set, look)
}

/// A list line's marker: a round bullet, or its number. Returns where
/// the line's words start, from `x`.
fn marker(svg: &mut Svg, m: &str, x: f32, top: f32, set: &Set, look: &Look) -> f32 {
    let font = set.font(Style::default());
    let base = top + font.baseline(set.lh);
    if m == "•" {
        let r = set.size * 0.15;
        svg.rect(x + set.size * 0.62 - r, base - set.size * 0.28 - r, 2. * r, 2. * r, r, set.color.alpha(0.85));
        return set.size * 1.4;
    }
    svg.text(x + 2., base, m, font, look.muted);
    (font.width(m) + 2. + set.size * 0.45).max(set.size * 1.4)
}

/// Markdown text: paragraphs (lines run together, a blank line between),
/// headings, bullets and numbered lines, and rules. Returns its height.
fn prose(svg: &mut Svg, text: &str, x: f32, y: f32, width: f32, set: &Set, look: &Look) -> f32 {
    #[derive(PartialEq)]
    enum Last {
        Nothing,
        Block,
        Item,
    }
    let gap = set.size * 0.55;
    let mut h = 0.;
    let mut para: Vec<String> = Vec::new();
    let mut last = Last::Nothing;
    // The space before what comes next: a little between list items,
    // more between paragraphs.
    let space = |last: &Last, item: bool| match last {
        Last::Nothing => 0.,
        Last::Item if item => set.size * 0.23,
        _ => gap,
    };
    let flush = |svg: &mut Svg, para: &mut Vec<String>, h: &mut f32, last: &mut Last| {
        if para.is_empty() {
            return;
        }
        *h += space(last, false);
        *last = Last::Block;
        *h += paragraph(svg, &para.join(" "), x, y + *h, width, set, look);
        para.clear();
    };
    for raw in text.lines() {
        let line = raw.trim();
        if line.is_empty() {
            flush(svg, &mut para, &mut h, &mut last);
            continue;
        }
        if line.starts_with('#') {
            flush(svg, &mut para, &mut h, &mut last);
            h += space(&last, false);
            last = Last::Block;
            let level = line.chars().take_while(|c| *c == '#').count();
            let k = match level {
                1 => 1.35,
                2 => 1.2,
                _ => 1.08,
            };
            let head = Set { size: set.size * k, lh: set.lh * k, weight: 700, color: look.text, ..*set };
            h += paragraph(svg, line.trim_start_matches('#').trim(), x, y + h, width, &head, look);
            continue;
        }
        let ruled = ruled_line(raw);
        if ruled.text.is_empty() && ruled.marker.is_none() {
            // A rule.
            flush(svg, &mut para, &mut h, &mut last);
            h += gap;
            svg.line(x, y + h + 0.5, x + width, y + h + 0.5, look.rule, 1., None);
            h += gap;
            last = Last::Nothing;
            continue;
        }
        if let Some(m) = &ruled.marker {
            flush(svg, &mut para, &mut h, &mut last);
            h += space(&last, true);
            last = Last::Item;
            let at = x + ruled.indent as f32 * set.size * 1.1;
            let left = marker(svg, m, at, y + h, set, look);
            h += paragraph(svg, &ruled.text, at + left, y + h, x + width - at - left, set, look);
            continue;
        }
        para.push(line.to_string());
    }
    flush(svg, &mut para, &mut h, &mut last);
    h
}

// ---------- Notes ----------

fn size_factor(z: TextSize) -> f32 {
    match z {
        TextSize::Small => 0.9,
        TextSize::Medium => 1.0,
        TextSize::Large => 1.2,
    }
}

fn radii(style: NoteStyle) -> (f32, f32) {
    match style {
        NoteStyle::Card | NoteStyle::Cheat => (12., 12.),
        NoteStyle::Taped => (4., 10.),
        NoteStyle::Index => (3., 3.),
    }
}

/// A note laid out: what's drawn on its paper (from its top-left corner),
/// and how tall it is.
struct Laid {
    svg: Svg,
    h: f32,
    /// An index card's ruled lines: from under its head to the end of its
    /// words.
    rules: (f32, f32),
    /// Lines of words a note kept at its height leaves out.
    more: usize,
}

/// The words on a note: its title, link card, text, tasks, Remember boxes
/// and tables, and its chips, laid out at its width.
fn lay_note(s: &Sticker, t: &Theme, look: &Look, media: &Path, blur: bool) -> Laid {
    let mut svg = Svg::new(blur);
    let k = size_factor(s.text_size);
    let style = s.style;
    let index = style == NoteStyle::Index;
    let lh = if index { 24. } else { 20. } * k;
    let w = s.rect.w;
    let fam = family(s.font);
    let body = Set { family: fam, size: 13. * k, lh, weight: 400, italic: false, color: look.body };
    let (rt, _) = radii(style);
    let mut y = 0.;

    // A picture across the top.
    let has_text = !s.title.is_empty() || !s.body.is_empty();
    if let Some(name) = &s.image {
        let ph = if has_text || !s.fixed { (w * 0.62).min(s.rect.h) } else { s.rect.h };
        let inner = (rt - 1.).max(0.);
        let clip = rrect(0., 0., w, ph, inner, if has_text || !s.fixed { 0. } else { inner });
        svg.image(&media.join(name).to_string_lossy(), 0., 0., w, ph, Some(clip));
        y += ph;
    }

    // Where the words go, by type.
    let (left, right, top, bottom, gap) = match style {
        NoteStyle::Card => (16., 16., 14., 14., 8.),
        NoteStyle::Taped => (16., 16., 18., 14., 8.),
        NoteStyle::Cheat => (14., 14., 10., 14., 8.),
        NoteStyle::Index => (40., 14., 0., 10., 0.),
    };
    let title_set = Set { family: fam, size: if style == NoteStyle::Cheat { 14. } else { 15. } * k, lh, weight: 600, italic: false, color: look.text };
    let width = w - left - right;
    match style {
        NoteStyle::Cheat => {
            // The title on a band of deeper paper.
            let mut band = Svg::new(blur);
            let th = if s.title.is_empty() { 0. } else { paragraph(&mut band, &s.title, 14., y + 9., w - 28., &title_set, look) };
            let bh = if s.title.is_empty() { 8. } else { th + 18. };
            let inner = (rt - 1.).max(0.);
            svg.fill(&rrect(0., y, w, bh, if s.image.is_none() { inner } else { 0. }, 0.), rgba(t.paper_band(s.paper)));
            svg.add(band);
            y += bh;
        }
        NoteStyle::Index => {
            // The title above a pink rule.
            let th = if s.title.is_empty() { 0. } else { paragraph(&mut svg, &s.title, 40., y + 12., w - 56., &Set { lh: 24. * k, ..title_set }, look) };
            y += (12. + th + 4.).max(16.) + 1.;
            svg.line(0., y - 0.5, w, y - 0.5, look.head_rule, 1., None);
        }
        _ => {}
    }
    let head = y;
    y += top;
    let mut first = true;
    let gapped = |y: &mut f32, first: &mut bool| {
        if !*first {
            *y += gap;
        }
        *first = false;
    };
    if matches!(style, NoteStyle::Card | NoteStyle::Taped) && !s.title.is_empty() {
        gapped(&mut y, &mut first);
        y += paragraph(&mut svg, &s.title, left, y, width, &title_set, look);
    }

    // A web page pinned as a note.
    if let Some(l) = &s.link {
        gapped(&mut y, &mut first);
        let row = 22. * k;
        match &l.icon {
            Some(name) => svg.image(&media.join(name).to_string_lossy(), left, y + (row - 14. * k) / 2., 14. * k, 14. * k, Some(rrect(left, y + (row - 14. * k) / 2., 14. * k, 14. * k, 3., 3.))),
            None => svg.icon("globe", left, y + (row - 12. * k) / 2., 12. * k, look.subtle),
        }
        let site = Set { family: fam, size: 11. * k, lh: row, weight: 400, italic: false, color: look.subtle };
        paragraph(&mut svg, &l.site, left + 20. * k, y, width - 20. * k, &site, look);
        y += row + 4. * k;
        let title = if l.title.is_empty() { l.url.clone() } else { l.title.clone() };
        y += paragraph(&mut svg, &title, left, y, width, &Set { size: 15. * k, lh: 20. * k, weight: 600, color: look.text, ..site }, look);
    }

    let mut done = 0;
    let mut total = 0;
    for block in blocks(&s.body) {
        gapped(&mut y, &mut first);
        match block {
            Block::Text(text) if index => {
                // One ruled line for each line typed, blank ones too.
                for line in text.lines() {
                    let l = ruled_line(line);
                    let at = left + 14. * k * l.indent as f32;
                    let mut tx = at;
                    if let Some(m) = &l.marker {
                        tx += marker(&mut svg, m, at, y, &body, look);
                    }
                    y += if l.text.is_empty() { lh } else { paragraph(&mut svg, &l.text, tx, y, w - right - tx, &body, look) };
                }
            }
            Block::Text(text) => y += prose(&mut svg, &text, left, y, width, &body, look),
            Block::Tasks(items) => {
                let row_gap = if index { 0. } else { 5. * k };
                for (i, (ticked, text)) in items.into_iter().enumerate() {
                    if i > 0 {
                        y += row_gap;
                    }
                    total += 1;
                    let b = 15. * k;
                    let by = y + (lh - b) / 2.;
                    if ticked {
                        done += 1;
                        svg.rect(left, by, b, b, 4. * k, look.brand);
                        svg.icon("check", left + (b - 11. * k) / 2., by + (b - 11. * k) / 2., 11. * k, Rgba { r: 255, g: 255, b: 255, a: 1. });
                    } else {
                        let _ = writeln!(
                            svg.body,
                            r#"<path d="{}" fill="none" {} stroke-width="1"/>"#,
                            rrect(left + 0.5, by + 0.5, b - 1., b - 1., 3.5 * k, 3.5 * k),
                            look.subtle.attr("stroke")
                        );
                    }
                    let set = Set { color: if ticked { look.subtle } else { look.body }, ..body };
                    let md = if ticked && !text.trim().is_empty() { format!("~~{}~~", text.trim()) } else { text };
                    let th = paragraph(&mut svg, &md, left + b + 8. * k, y, width - b - 8. * k, &set, look);
                    y += th.max(lh);
                }
            }
            Block::Callout { label, text } => {
                let (px, py) = (10. * k, if index { 0. } else { 8. * k });
                let mut inside = Svg::new(blur);
                let mut h = py;
                if let Some(label) = &label {
                    let row = if index { lh } else { 14. * k };
                    inside.icon("lightbulb", left + px, y + h + (row - 11. * k) / 2., 11. * k, look.body);
                    let set = Set { family: fam, size: 10. * k, lh: row, weight: 700, italic: false, color: look.body };
                    paragraph(&mut inside, &label.to_uppercase(), left + px + 16. * k, y + h, width - 2. * px - 16. * k, &set, look);
                    h += row + if index { 0. } else { 2. * k };
                }
                if !text.is_empty() {
                    let set = Set { family: fam, size: 12.5 * k, lh: if index { lh } else { 18. * k }, weight: 400, italic: false, color: look.text };
                    h += prose(&mut inside, &text, left + px, y + h, width - 2. * px, &set, look);
                }
                h += py;
                svg.rect(left, y, width, h, 8. * k, look.wash);
                svg.add(inside);
                y += h;
            }
            Block::Table { head, rows } => {
                let cols = head.len().max(1);
                let cg = 6. * k;
                let cw = (width - cg * (cols as f32 - 1.)) / cols as f32;
                let row_lh = if index { lh } else { 16. * k };
                let pad = if index { 0. } else { 5. * k };
                // The header: small capitals over a rule.
                let hs = Set { family: fam, size: 10. * k, lh: row_lh, weight: 700, italic: false, color: look.body };
                let mut hh: f32 = 0.;
                for (c, cell) in head.iter().enumerate() {
                    hh = hh.max(paragraph(&mut svg, &cell.to_uppercase(), left + c as f32 * (cw + cg), y, cw, &hs, look));
                }
                y += hh + pad;
                svg.line(left, y - 0.5, left + width, y - 0.5, look.rule, 1., None);
                let last = rows.len();
                for (i, row) in rows.into_iter().enumerate() {
                    y += pad;
                    let mut rh: f32 = row_lh;
                    for (c, cell) in row.iter().enumerate() {
                        let set = if c == 0 { Set { size: 12. * k, lh: row_lh, weight: 600, color: look.text, ..hs } } else { Set { size: 12. * k, lh: row_lh, weight: 400, color: look.body, ..hs } };
                        rh = rh.max(paragraph(&mut svg, cell, left + c as f32 * (cw + cg), y, cw, &set, look));
                    }
                    y += rh;
                    if i + 1 < last {
                        y += pad;
                        svg.line(left, y - 0.5, left + width, y - 0.5, look.rule, 1., Some(3.));
                    }
                }
            }
        }
    }

    // Chips: tasks done and tags.
    let mut chips: Vec<(Option<&str>, String, Rgba)> = Vec::new();
    if total > 0 {
        chips.push((Some("list-checks"), format!("{done}/{total}"), if done == total { look.brand_on_tint } else { look.muted }));
    }
    for tag in s.tags().into_iter().take(3) {
        chips.push((None, format!("#{tag}"), look.muted));
    }
    if !chips.is_empty() {
        gapped(&mut y, &mut first);
        let row_top = if index { y + (24. * k - 18.) / 2. } else { y + 2. };
        let mut x = left;
        let mut row_y = row_top;
        let font = Font::new(fam, 10.5, 400);
        for (icon, text, color) in chips {
            let cw = 12. + font.width(&text) + if icon.is_some() { 13. } else { 0. };
            if x > left && x + cw > left + width {
                x = left;
                row_y += 22.;
            }
            svg.rect(x, row_y, cw, 18., 9., look.chip);
            let mut tx = x + 6.;
            if let Some(i) = icon {
                svg.icon(i, tx, row_y + 4., 10., color);
                tx += 13.;
            }
            svg.text(tx, row_y + font.baseline(18.), &text, font, color);
            x += cw + 4.;
        }
        y = (row_y + 18.).max(if index { y + 24. * k } else { 0. });
    }
    // An index card's body is at least one ruled line.
    y = if index { (y + bottom).max(head + 24. * k) } else { y + bottom };
    let h = if s.fixed { s.rect.h } else { s.rect.h.max(y) };
    let more = if s.fixed && y > s.rect.h + 1. { ((y - s.rect.h) / lh).ceil().max(1.) as usize } else { 0 };
    Laid { svg, h, rules: (head, y), more }
}

/// A note drawn at its place: shadow, paper, its words, and its tape or pin.
fn draw_note(out: &mut Svg, s: &Sticker, laid: Laid, t: &Theme, look: &Look) {
    let (x, y, w, h) = (s.rect.x, s.rect.y, s.rect.w, laid.h);
    let style = s.style;
    let (rt, rb) = radii(style);
    let shadows = match style {
        NoteStyle::Taped => t.shadow_taped(1.),
        NoteStyle::Index => t.shadow_index(1.),
        _ => t.shadow_card(1.),
    };
    out.shadows(Rect { x, y, w, h }, rt, rb, &shadows);
    out.open(x, y);
    let paper = rgba(t.paper(s.paper));
    out.fill(&rrect(0., 0., w, h, rt, rb), paper);
    // The words stay on the paper: a note kept at its height cuts them off.
    let clip = out.id("c");
    let _ = writeln!(out.defs, r#"<clipPath id="{clip}"><path d="{}"/></clipPath>"#, rrect(0., 0., w, h, rt, rb));
    let _ = writeln!(out.body, r#"<g clip-path="url(#{clip})">"#);
    if style == NoteStyle::Taped {
        // Where taped paper lifts a little at its foot.
        out.gradient(Rect { x: 0., y: h - 28., w, h: 28. }, rb, look.curl.alpha(0.), look.curl);
    }
    if style == NoteStyle::Index {
        // Ruled lines under the head, and the pink margin.
        let step = 24. * size_factor(s.text_size);
        let (head, end) = laid.rules;
        let mut ry = head + step;
        while ry <= end.min(h) + 0.5 {
            out.line(0., ry - 0.5, w, ry - 0.5, look.ruled, 1., None);
            ry += step;
        }
        out.line(30.5, 0., 30.5, h, look.margin, 1., None);
    }
    let more = laid.more;
    out.add(laid.svg);
    if more > 0 {
        // Cut off: the words fade into the paper, and say how much more.
        out.gradient(Rect { x: 0., y: h - 34., w, h: 34. }, rb, paper.alpha(0.), paper);
        let f = Font::new(SANS, 10.5, 400);
        let label = format!("+{more} more line{}", if more == 1 { "" } else { "s" });
        out.text((w - f.width(&label)) / 2., h - 4. - f.size * 0.3, &label, f, look.subtle);
    }
    out.close();
    // Its edge.
    let _ = writeln!(out.body, r#"<path d="{}" fill="none" {} stroke-width="1"/>"#, rrect(0.5, 0.5, w - 1., h - 1., rt - 0.5, rb - 0.5), look.edge.attr("stroke"));
    match s.pin {
        Some(pin) => draw_pin(out, pin, w, t),
        None if style == NoteStyle::Taped => {
            // Clear tape across the top, where it lands varying by note.
            let at = [0.5, 0.27, 0.7][(s.id % 3) as usize];
            let tx = w * at - 42.;
            out.rect(tx, -11., 84., 22., 0., look.tape);
            out.line(tx + 0.5, -11., tx + 0.5, 11., look.tape_edge, 1., None);
            out.line(tx + 83.5, -11., tx + 83.5, 11., look.tape_edge, 1., None);
        }
        None => {}
    }
    out.close();
}

/// A pin across the middle of a note's top edge, `w` wide.
fn draw_pin(out: &mut Svg, pin: Pin, w: f32, t: &Theme) {
    let Some(mut svg) = asset(&format!("pins/{}.svg", pin.shape.key())) else { return };
    if pin.shape.tone().is_some() {
        let [main, shade, light] = t.pin_tone(pin.tone);
        svg = svg.replace("{main}", &format!("#{main:06x}")).replace("{shade}", &format!("#{shade:06x}")).replace("{light}", &format!("#{light:06x}"));
    }
    let d = pin.size.units();
    let middle = -d * pin.shape.anchor() + d / 2.;
    let _ = writeln!(
        out.body,
        r#"<image href="{}" x="{}" y="{}" width="{}" height="{}"/>"#,
        data_uri("image/svg+xml", svg.as_bytes()),
        n(w / 2. - d / 2.),
        n(middle - d / 2.),
        n(d),
        n(d)
    );
}

// ---------- The sheet ----------

/// A shape as the board draws it: its paper, its outline, and its words
/// turned with it.
fn draw_shape(svg: &mut Svg, s: &Shape, t: &Theme) {
    let path = |o: &crate::shape::Outline| {
        let mut d = String::new();
        for (i, p) in o.points.iter().enumerate() {
            let _ = write!(d, "{}{} {}", if i == 0 { "M" } else { "L" }, n(p.x), n(p.y));
        }
        if o.closed {
            d.push('Z');
        }
        d
    };
    let outlines = s.outlines();
    if let (Some(p), Some(o)) = (s.fill, outlines.first()) {
        if o.closed {
            svg.fill(&path(o), rgba(t.paper(p)));
        }
    }
    let dash = if s.dashed { r#" stroke-dasharray="6 4.2""# } else { "" };
    for o in &outlines {
        let _ = writeln!(
            svg.body,
            r#"<path d="{}" fill="none" {} stroke-width="{}" stroke-linejoin="round" stroke-linecap="round"{dash}/>"#,
            path(o),
            rgba(t.ink(s.ink)).attr("stroke"),
            n(s.line.units())
        );
    }
    let text = s.text.trim();
    if text.is_empty() {
        return;
    }
    let font = Font::new(SANS, SHAPE_TEXT, crate::glyphs::WEIGHT);
    let r = s.rect;
    let lines = crate::glyphs::wrap(text, SHAPE_TEXT, (r.w - 2. * TEXT_PAD).max(SHAPE_TEXT));
    let lh = SHAPE_TEXT * crate::glyphs::LEADING;
    let top = r.y + (r.h - lh * lines.len() as f32) / 2.;
    let c = s.center();
    let _ = writeln!(svg.body, r#"<g transform="rotate({} {} {})">"#, n(s.rot), n(c.x), n(c.y));
    for (i, line) in lines.iter().enumerate() {
        svg.text(r.x + (r.w - font.width(line)) / 2., top + lh * i as f32 + font.baseline(lh), line, font, rgba(t.colors.text));
    }
    svg.close();
}

/// Where the sheet's name is written: as on the board, where a fresh
/// sheet's camera shows it in the top-left corner.
fn head_at(t: &Theme) -> Vec2 {
    let home = Camera::default().pan;
    v2(f32::from(t.space.xxl) - home.x, f32::from(t.space.xl) - home.y)
}

/// The whole sheet as SVG, framed round what's on it.
fn draw(sheet: &Category, t: &Theme, media: &Path, blur: bool) -> String {
    let look = Look::of(t);
    let mut svg = Svg::new(blur);

    // Notes first laid out, so their heights are known to what joins them.
    let laid: Vec<Laid> = sheet.stickers.iter().map(|s| lay_note(s, t, &look, media, blur)).collect();
    let rect_of = |s: &Sticker, l: &Laid| Rect { h: l.h, ..s.rect };
    let mut stuff: Vec<Rect> = sheet.stickers.iter().zip(&laid).map(|(s, l)| rect_of(s, l)).collect();
    stuff.extend(sheet.frames.iter().map(|f| f.rect));
    stuff.extend(sheet.strokes.iter().map(|s| s.bounds()));
    stuff.extend(sheet.shapes.iter().map(Shape::bounds));

    // Text on the sheet, set so its size is known.
    let mut labels = Svg::new(blur);
    for l in &sheet.labels {
        let set = Set { family: family(l.font), size: l.size.units(), lh: l.size.line(), weight: 400, italic: l.italic, color: rgba(t.ink(l.ink)) };
        let width = l.width.unwrap_or(LABEL_WRAP);
        // As wide as its longest line, when it has no width of its own.
        let natural = l
            .text
            .lines()
            .map(|line| {
                let r = ruled_line(line);
                wrap(&runs(&r.text, &look), &set, width).iter().map(|ln| ln.last().map_or(0., |p| p.x + p.w)).fold(0., f32::max)
                    + if r.marker.is_some() { set.size * 1.5 } else { 0. }
            })
            .fold(0., f32::max)
            .min(width);
        let h = prose(&mut labels, &l.text, l.at.x, l.at.y, width, &set, &look);
        stuff.push(Rect { x: l.at.x, y: l.at.y, w: if l.width.is_some() { width } else { natural }, h: h.max(set.lh) });
    }

    // The sheet's name, its emoji or monogram, and a line under it.
    let at = head_at(t);
    let name = Font::new(SANS, f32::from(t.text.heading), 600);
    let n_notes = sheet.stickers.len();
    let meta = format!(
        "{}{} note{}",
        if sheet.blurb.is_empty() { String::new() } else { format!("{} · ", sheet.blurb) },
        n_notes,
        if n_notes == 1 { "" } else { "s" }
    );
    let caption = Font::new(SANS, f32::from(t.text.caption), 400);
    let row = 34.;
    let mut head = Svg::new(blur);
    let mut nx = at.x;
    if !sheet.icon.is_empty() {
        let icon = sheet.icon.clone();
        if icon.chars().all(|c| c.is_ascii_graphic()) {
            let k = match icon.chars().count() {
                1 => 0.5,
                2 => 0.42,
                3 => 0.33,
                4 => 0.27,
                _ => 0.23,
            };
            head.rect(at.x, at.y, row, row, row * 0.3, look.brand_tint);
            let _ = writeln!(head.body, r#"<path d="{}" fill="none" {} stroke-width="1"/>"#, rrect(at.x + 0.5, at.y + 0.5, row - 1., row - 1., row * 0.3 - 0.5, row * 0.3 - 0.5), look.brand.alpha(0.25).attr("stroke"));
            let f = Font::new(SANS, row * k, 700);
            head.text(at.x + (row - f.width(&icon)) / 2., at.y + f.baseline(row), &icon, f, look.brand_on_tint);
        } else {
            head.text(at.x, at.y + name.baseline(row), &icon, name, look.text);
        }
        nx += row + f32::from(t.space.sm);
    }
    head.text(nx, at.y + name.baseline(row), &sheet.name, name, look.text);
    let my = at.y + row + f32::from(t.space.xxs);
    head.text(at.x, my + caption.baseline(16.), &meta, caption, look.muted);
    stuff.push(Rect { x: at.x, y: at.y, w: (nx - at.x + name.width(&sheet.name)).max(caption.width(&meta)), h: row + 18. });

    // The frame: round all of it, with room.
    let mut b = stuff[0];
    for r in &stuff[1..] {
        b = b.union(r);
    }
    let bounds = Rect { x: (b.x - MARGIN).floor(), y: (b.y - MARGIN).floor(), w: (b.w + 2. * MARGIN).ceil(), h: (b.h + 2. * MARGIN).ceil() };

    // The paper, its picture, its pattern.
    let tint = rgba(t.tint(sheet.backdrop.tint));
    svg.rect(bounds.x, bounds.y, bounds.w, bounds.h, 0., tint);
    if let Some(name) = &sheet.backdrop.image {
        svg.image(&media.join(name).to_string_lossy(), bounds.x, bounds.y, bounds.w, bounds.h, None);
        if sheet.backdrop.soften {
            svg.rect(bounds.x, bounds.y, bounds.w, bounds.h, 0., tint.alpha(0.55));
        }
    }
    pattern(&mut svg, sheet.backdrop.pattern, bounds, t.size.dot_grid, &look);

    // Links between notes, then ink.
    for l in &sheet.links {
        let find = |id: Id| sheet.stickers.iter().zip(&laid).find(|(s, _)| s.id == id).map(|(s, l)| rect_of(s, l));
        if let (Some(a), Some(b)) = (find(l.from), find(l.to)) {
            let c = link_curve(&a, &b);
            let color = rgba(t.ink(l.ink)).alpha(0.75);
            let _ = writeln!(
                svg.body,
                r#"<path d="M{} {}C{} {} {} {} {} {}" fill="none" {} stroke-width="2" stroke-dasharray="6 6"/>"#,
                n(c[0].x),
                n(c[0].y),
                n(c[1].x),
                n(c[1].y),
                n(c[2].x),
                n(c[2].y),
                n(c[3].x),
                n(c[3].y),
                color.attr("stroke")
            );
            svg.rect(c[3].x - 3., c[3].y - 3., 6., 6., 3., color);
        }
    }
    for s in &sheet.strokes {
        let Some(first) = s.points.first() else { continue };
        let mut d = format!("M{} {}", n(first.x), n(first.y));
        if s.points.len() == 2 {
            let _ = write!(d, "L{} {}", n(s.points[1].x), n(s.points[1].y));
        } else if s.points.len() > 2 {
            // Smooth through the midpoints, each point a control point.
            for w in s.points.windows(2).skip(1) {
                let _ = write!(d, "Q{} {} {} {}", n(w[0].x), n(w[0].y), n((w[0].x + w[1].x) / 2.), n((w[0].y + w[1].y) / 2.));
            }
            let last = s.points.last().expect("points");
            let _ = write!(d, "L{} {}", n(last.x), n(last.y));
        } else {
            // A dot.
            let _ = write!(d, "L{} {}", n(first.x + 0.01), n(first.y));
        }
        let mut color = rgba(t.ink(s.ink));
        if s.marker {
            color = color.alpha(if t.is_dark { 0.35 } else { 0.3 });
        }
        let _ = writeln!(
            svg.body,
            r#"<path d="{d}" fill="none" {} stroke-width="{}" stroke-linecap="round" stroke-linejoin="round"/>"#,
            color.attr("stroke"),
            n(s.width.max(0.75))
        );
    }

    // Areas.
    for f in &sheet.frames {
        let c = rgba(t.accent(f.accent));
        let r = f.rect;
        svg.rect(r.x, r.y, r.w, r.h, 16., c.alpha(if t.is_dark { 0.10 } else { 0.08 }));
        let _ = writeln!(svg.body, r#"<path d="{}" fill="none" {} stroke-width="1"/>"#, rrect(r.x + 0.5, r.y + 0.5, r.w - 1., r.h - 1., 15.5, 15.5), c.alpha(0.35).attr("stroke"));
        let font = Font::new(SANS, 14., 600);
        let row = 18.;
        svg.rect(r.x + 14., r.y + 10. + (row - 8.) / 2., 8., 8., 4., c);
        let title = if f.title.is_empty() { "Untitled area" } else { f.title.as_str() };
        svg.text(r.x + 28., r.y + 10. + font.baseline(row), title, font, look.muted);
    }

    // Shapes, over the areas and the ink.
    for s in &sheet.shapes {
        draw_shape(&mut svg, s, t);
    }

    // The name, text on the sheet, then the notes, back to front.
    svg.add(head);
    svg.add(labels);
    for (s, l) in sheet.stickers.iter().zip(laid) {
        draw_note(&mut svg, s, l, t, &look);
    }

    // Folio's mark, small and quiet, in the corner.
    let mark = Font::new(SANS, 13., 600);
    let word = "folio";
    let ww = mark.width(word);
    let dw = mark.width(".");
    let (mx, my) = (bounds.x + bounds.w - MARGIN / 2. - ww - dw, bounds.y + bounds.h - MARGIN / 2. - 4.);
    svg.text(mx, my, word, mark, look.subtle.alpha(0.8));
    svg.text(mx + ww, my, ".", mark, look.brand.alpha(0.8));

    svg.finish(bounds)
}

/// The sheet's pattern over `b`, from the world's origin as on the board.
fn pattern(svg: &mut Svg, p: Pattern, b: Rect, grid: f32, look: &Look) {
    let from = |step: f32, v: f32| (v / step).floor() as i64;
    let mut dots = String::new();
    let dot = |d: &mut String, x: f32, y: f32, r: f32| {
        let _ = write!(d, "M{} {}a{r} {r} 0 1 0 {} 0a{r} {r} 0 1 0 {} 0", n(x - r), n(y), n(2. * r), n(-2. * r), r = n(r));
    };
    let lines = |across: bool, step: f32, every: Option<i64>, major: bool| {
        let mut d = String::new();
        let (start, end) = if across { (b.y, b.y + b.h) } else { (b.x, b.x + b.w) };
        for i in from(step, start)..=from(step, end) + 1 {
            let at = i as f32 * step;
            if at < start || at > end || every.is_some_and(|m| (i.rem_euclid(m) == 0) != major) {
                continue;
            }
            if across {
                let _ = write!(d, "M{} {}H{}", n(b.x), n(at), n(b.x + b.w));
            } else {
                let _ = write!(d, "M{} {}V{}", n(at), n(b.y), n(b.y + b.h));
            }
        }
        d
    };
    let stroke = |svg: &mut Svg, d: String, c: Rgba| {
        if !d.is_empty() {
            let _ = writeln!(svg.body, r#"<path d="{d}" fill="none" {} stroke-width="1"/>"#, c.attr("stroke"));
        }
    };
    match p {
        Pattern::Plain => {}
        Pattern::Dots => {
            for j in from(grid, b.y)..=from(grid, b.y + b.h) + 1 {
                for i in from(grid, b.x)..=from(grid, b.x + b.w) + 1 {
                    dot(&mut dots, i as f32 * grid, j as f32 * grid, 1.1);
                }
            }
            svg.fill(&dots, look.dot);
        }
        Pattern::Grid => {
            let d = lines(true, grid, None, false) + &lines(false, grid, None, false);
            stroke(svg, d, look.line);
        }
        Pattern::Graph => {
            let minor = lines(true, grid, Some(5), false) + &lines(false, grid, Some(5), false);
            stroke(svg, minor, look.line);
            let major = lines(true, grid, Some(5), true) + &lines(false, grid, Some(5), true);
            stroke(svg, major, look.dot);
        }
        Pattern::Lines => {
            let d = lines(true, grid * 1.25, None, false);
            stroke(svg, d, look.line);
        }
        Pattern::Crosses => {
            let mut d = String::new();
            for j in from(grid, b.y)..=from(grid, b.y + b.h) + 1 {
                for i in from(grid, b.x)..=from(grid, b.x + b.w) + 1 {
                    let (x, y) = (i as f32 * grid, j as f32 * grid);
                    let _ = write!(d, "M{} {}h7M{} {}v7", n(x - 3.5), n(y), n(x), n(y - 3.5));
                }
            }
            stroke(svg, d, look.dot);
        }
        Pattern::Isometric => {
            let row = grid * 0.866;
            for j in from(row, b.y)..=from(row, b.y + b.h) + 1 {
                let shift = if j.rem_euclid(2) == 1 { grid / 2. } else { 0. };
                for i in from(grid, b.x) - 1..=from(grid, b.x + b.w) + 1 {
                    dot(&mut dots, i as f32 * grid + shift, j as f32 * row, 1.1);
                }
            }
            svg.fill(&dots, look.dot);
        }
    }
}

// ---------- PNG ----------

fn png(tree: &usvg::Tree) -> Result<Vec<u8>, String> {
    let size = tree.size();
    let (w, h) = (size.width(), size.height());
    let scale = PNG_SCALE.min(PNG_SIDE / w).min(PNG_SIDE / h).min((PNG_PIXELS / (w * h)).sqrt());
    let mut pixmap = tiny_skia::Pixmap::new((w * scale).ceil() as u32, (h * scale).ceil() as u32).ok_or("The sheet is too large for a picture")?;
    resvg::render(tree, tiny_skia::Transform::from_scale(scale, scale), &mut pixmap.as_mut());
    pixmap.encode_png().map_err(|e| e.to_string())
}

// ---------- PDF ----------

/// A one-page PDF of `tree`, drawn as it is: shapes and outlined words as
/// paths, pictures as images.
fn pdf(tree: &usvg::Tree, title: &str) -> Vec<u8> {
    let size = tree.size();
    let k = PT.min(PDF_SIDE / size.width()).min(PDF_SIDE / size.height());
    let (w, h) = (size.width() * k, size.height() * k);
    let mut p = Pdf::default();
    // Objects 1 to 5 are the catalog, pages, the page, its drawing and
    // the document's info; pictures follow.
    p.objects = vec![Vec::new(); 5];
    let _ = writeln!(p.ops, "{} 0 0 {} 0 {} cm", n(k), n(-k), n(h));
    p.group(tree.root(), tiny_skia::Transform::identity(), 1.);

    let drawing = miniz_oxide::deflate::compress_to_vec_zlib(p.ops.as_bytes(), 6);
    let mut gs = String::new();
    for (i, a) in p.alphas.iter().enumerate() {
        let _ = write!(gs, "/A{i} << /ca {a} /CA {a} >> ", a = n(*a));
    }
    let mut xo = String::new();
    for (i, id) in p.images.iter().enumerate() {
        let _ = write!(xo, "/I{i} {id} 0 R ");
    }
    let mut sh = String::new();
    for (i, id) in p.shadings.iter().enumerate() {
        let _ = write!(sh, "/S{i} {id} 0 R ");
    }
    p.objects[0] = b"<< /Type /Catalog /Pages 2 0 R >>".to_vec();
    p.objects[1] = b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_vec();
    p.objects[2] = format!(
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 {} {}] /Resources << /ExtGState << {gs}>> /XObject << {xo}>> /Shading << {sh}>> >> /Contents 4 0 R >>",
        n(w),
        n(h)
    )
    .into_bytes();
    p.objects[3] = stream(&format!("/Length {} /Filter /FlateDecode", drawing.len()), &drawing);
    let utf16: String = std::iter::once(0xfeffu16).chain(title.encode_utf16()).map(|u| format!("{u:04X}")).collect();
    let date = chrono::Local::now().format("D:%Y%m%d%H%M%S");
    p.objects[4] = format!("<< /Title <{utf16}> /Creator (Folio) /Producer (Folio) /CreationDate ({date}) >>").into_bytes();

    let mut out: Vec<u8> = b"%PDF-1.4\n%\xe2\xe3\xcf\xd3\n".to_vec();
    let mut offsets = Vec::new();
    for (i, obj) in p.objects.iter().enumerate() {
        offsets.push(out.len());
        out.extend_from_slice(format!("{} 0 obj\n", i + 1).as_bytes());
        out.extend_from_slice(obj);
        out.extend_from_slice(b"\nendobj\n");
    }
    let xref = out.len();
    let _ = writeln!(Bytes(&mut out), "xref\n0 {}\n0000000000 65535 f ", p.objects.len() + 1);
    for o in offsets {
        let _ = writeln!(Bytes(&mut out), "{o:010} 00000 n ");
    }
    let _ = write!(Bytes(&mut out), "trailer\n<< /Size {} /Root 1 0 R /Info 5 0 R >>\nstartxref\n{xref}\n%%EOF\n", p.objects.len() + 1);
    out
}

/// `write!` into bytes.
struct Bytes<'a>(&'a mut Vec<u8>);

impl std::fmt::Write for Bytes<'_> {
    fn write_str(&mut self, s: &str) -> std::fmt::Result {
        self.0.extend_from_slice(s.as_bytes());
        Ok(())
    }
}

fn stream(dict: &str, data: &[u8]) -> Vec<u8> {
    let mut o = format!("<< {dict} >>\nstream\n").into_bytes();
    o.extend_from_slice(data);
    o.extend_from_slice(b"\nendstream");
    o
}

#[derive(Default)]
struct Pdf {
    objects: Vec<Vec<u8>>,
    /// The page's drawing operators.
    ops: String,
    /// Opacities used, each a graphics state `/A{i}`.
    alphas: Vec<f32>,
    /// Pictures as image objects, `/I{i}`, and which data each came from.
    images: Vec<usize>,
    seen: HashMap<usize, usize>,
    /// Gradients as shading objects, `/S{i}`.
    shadings: Vec<usize>,
}

impl Pdf {
    fn alpha(&mut self, a: f32) {
        let a = (a * 1000.).round() / 1000.;
        let i = match self.alphas.iter().position(|x| *x == a) {
            Some(i) => i,
            None => {
                self.alphas.push(a);
                self.alphas.len() - 1
            }
        };
        let _ = writeln!(self.ops, "/A{i} gs");
    }

    fn group(&mut self, g: &usvg::Group, ts: tiny_skia::Transform, alpha: f32) {
        let ts = ts.pre_concat(g.transform());
        let alpha = alpha * g.opacity().get();
        if alpha < 0.002 {
            return;
        }
        let clipped = g.clip_path().is_some_and(|c| {
            self.ops.push_str("q\n");
            let mut d = String::new();
            clip_shapes(c.root(), ts.pre_concat(c.transform()), &mut d);
            if d.is_empty() {
                // Nothing to show through.
                d.push_str("0 0 m h\n");
            }
            self.ops.push_str(&d);
            self.ops.push_str("W n\n");
            true
        });
        for node in g.children() {
            match node {
                usvg::Node::Group(g) => self.group(g, ts, alpha),
                usvg::Node::Path(p) => self.path(p, ts, alpha),
                usvg::Node::Image(i) => self.image(i, ts, alpha),
                usvg::Node::Text(t) => self.group(t.flattened(), ts, alpha),
            }
        }
        if clipped {
            self.ops.push_str("Q\n");
        }
    }

    fn path(&mut self, p: &usvg::Path, ts: tiny_skia::Transform, alpha: f32) {
        if !p.is_visible() {
            return;
        }
        let mut d = String::new();
        outline(p.data(), ts, &mut d);
        if let Some(f) = p.fill() {
            let even_odd = f.rule() == usvg::FillRule::EvenOdd;
            if self.shade(f.paint(), &d, even_odd, ts, f.opacity().get() * alpha) {
                // Filled with its gradient.
            } else if let Some((c, a)) = paint(f.paint()) {
                let a = a * f.opacity().get() * alpha;
                if a > 0.002 {
                    self.alpha(a);
                    let _ = writeln!(self.ops, "{} {} {} rg", n(c.0), n(c.1), n(c.2));
                    self.ops.push_str(&d);
                    self.ops.push_str(if even_odd { "f*\n" } else { "f\n" });
                }
            }
        }
        if let Some(s) = p.stroke() {
            if let Some((c, a)) = paint(s.paint()) {
                let a = a * s.opacity().get() * alpha;
                if a > 0.002 {
                    self.alpha(a);
                    let scale = (ts.sx * ts.sy - ts.kx * ts.ky).abs().sqrt();
                    let cap = match s.linecap() {
                        usvg::LineCap::Butt => 0,
                        usvg::LineCap::Round => 1,
                        usvg::LineCap::Square => 2,
                    };
                    let join = match s.linejoin() {
                        usvg::LineJoin::Round => 1,
                        usvg::LineJoin::Bevel => 2,
                        _ => 0,
                    };
                    let dash = s
                        .dasharray()
                        .map(|d| format!("[{}] {}", d.iter().map(|v| n(v * scale)).collect::<Vec<_>>().join(" "), n(s.dashoffset() * scale)))
                        .unwrap_or_else(|| "[] 0".into());
                    let _ = writeln!(self.ops, "{} w {cap} J {join} j {dash} d {} {} {} RG", n(s.width().get() * scale), n(c.0), n(c.1), n(c.2));
                    self.ops.push_str(&d);
                    self.ops.push_str("S\n");
                }
            }
        }
    }

    fn image(&mut self, i: &usvg::Image, ts: tiny_skia::Transform, alpha: f32) {
        if !i.is_visible() {
            return;
        }
        let data = match i.kind() {
            // An SVG picture is drawn as shapes too.
            usvg::ImageKind::SVG(tree) => return self.group(tree.root(), ts, alpha),
            usvg::ImageKind::JPEG(d) | usvg::ImageKind::PNG(d) | usvg::ImageKind::GIF(d) | usvg::ImageKind::WEBP(d) => d,
        };
        let key = Arc::as_ptr(data) as usize;
        let at = match self.seen.get(&key) {
            Some(at) => *at,
            None => {
                let Some(id) = self.embed(data) else { return };
                self.images.push(id);
                let at = self.images.len() - 1;
                self.seen.insert(key, at);
                at
            }
        };
        let size = i.size();
        self.ops.push_str("q\n");
        self.alpha(alpha);
        let _ = writeln!(
            self.ops,
            "{} {} {} {} {} {} cm {} 0 0 {} 0 {} cm /I{at} Do\nQ",
            n(ts.sx),
            n(ts.ky),
            n(ts.kx),
            n(ts.sy),
            n(ts.tx),
            n(ts.ty),
            n(size.width()),
            n(-size.height()),
            n(size.height())
        );
    }

    /// Fills outline `d` with a gradient paint, as a PDF shading clipped
    /// to it. False when the paint isn't a gradient.
    fn shade(&mut self, paint: &usvg::Paint, d: &str, even_odd: bool, ts: tiny_skia::Transform, alpha: f32) -> bool {
        let (base, kind, coords): (&usvg::BaseGradient, u8, String) = match paint {
            usvg::Paint::LinearGradient(g) => (g, 2, format!("[{} {} {} {}]", n(g.x1()), n(g.y1()), n(g.x2()), n(g.y2()))),
            usvg::Paint::RadialGradient(g) => (g, 3, format!("[{} {} 0 {} {} {}]", n(g.fx()), n(g.fy()), n(g.cx()), n(g.cy()), n(g.r().get()))),
            usvg::Paint::Color(_) | usvg::Paint::Pattern(_) => return false,
        };
        let stops = base.stops();
        if stops.is_empty() {
            return false;
        }
        // Opacity along a gradient isn't kept: it takes the stops' average.
        let opacity = stops.iter().map(|s| s.opacity().get()).sum::<f32>() / stops.len() as f32;
        if alpha * opacity < 0.002 {
            return true;
        }
        self.objects.push(format!("<< /ShadingType {kind} /ColorSpace /DeviceRGB /Coords {coords} /Function {} /Extend [true true] >>", ramp(stops)).into_bytes());
        self.shadings.push(self.objects.len());
        let i = self.shadings.len() - 1;
        let m = ts.pre_concat(base.transform());
        self.ops.push_str("q\n");
        self.alpha(alpha * opacity);
        self.ops.push_str(d);
        self.ops.push_str(if even_odd { "W* n\n" } else { "W n\n" });
        let _ = writeln!(self.ops, "{} {} {} {} {} {} cm /S{i} sh\nQ", n(m.sx), n(m.ky), n(m.kx), n(m.sy), n(m.tx), n(m.ty));
        true
    }

    /// A picture's pixels as an image object (and its transparency as a
    /// soft mask); its id.
    fn embed(&mut self, data: &[u8]) -> Option<usize> {
        let mut img = image::load_from_memory(data).ok()?;
        if img.width().max(img.height()) > PDF_IMAGE {
            img = img.resize(PDF_IMAGE, PDF_IMAGE, image::imageops::FilterType::Triangle);
        }
        let rgba = img.to_rgba8();
        let (w, h) = rgba.dimensions();
        let mut rgb = Vec::with_capacity((w * h * 3) as usize);
        let mut alpha = Vec::with_capacity((w * h) as usize);
        for p in rgba.pixels() {
            rgb.extend_from_slice(&p.0[..3]);
            alpha.push(p.0[3]);
        }
        let mask = alpha.iter().any(|a| *a < 255).then(|| {
            let z = miniz_oxide::deflate::compress_to_vec_zlib(&alpha, 6);
            self.objects.push(stream(&format!("/Type /XObject /Subtype /Image /Width {w} /Height {h} /ColorSpace /DeviceGray /BitsPerComponent 8 /Filter /FlateDecode /Length {}", z.len()), &z));
            self.objects.len()
        });
        let z = miniz_oxide::deflate::compress_to_vec_zlib(&rgb, 6);
        let smask = mask.map(|m| format!(" /SMask {m} 0 R")).unwrap_or_default();
        self.objects.push(stream(
            &format!("/Type /XObject /Subtype /Image /Width {w} /Height {h} /ColorSpace /DeviceRGB /BitsPerComponent 8 /Filter /FlateDecode /Length {}{smask}", z.len()),
            &z,
        ));
        Some(self.objects.len())
    }
}

/// A clip path's shapes, as one outline.
fn clip_shapes(g: &usvg::Group, ts: tiny_skia::Transform, d: &mut String) {
    for node in g.children() {
        match node {
            usvg::Node::Path(p) if p.is_visible() => outline(p.data(), ts, d),
            usvg::Node::Group(g) => clip_shapes(g, ts.pre_concat(g.transform()), d),
            usvg::Node::Text(t) => clip_shapes(t.flattened(), ts.pre_concat(t.flattened().transform()), d),
            _ => {}
        }
    }
}

/// A path's outline as PDF path operators, through `ts`.
fn outline(path: &tiny_skia::Path, ts: tiny_skia::Transform, d: &mut String) {
    let map = |p: tiny_skia::Point| {
        let mut q = [p];
        ts.map_points(&mut q);
        q[0]
    };
    let mut last = tiny_skia::Point::zero();
    for seg in path.segments() {
        match seg {
            tiny_skia::PathSegment::MoveTo(p) => {
                let q = map(p);
                let _ = writeln!(d, "{} {} m", n(q.x), n(q.y));
                last = p;
            }
            tiny_skia::PathSegment::LineTo(p) => {
                let q = map(p);
                let _ = writeln!(d, "{} {} l", n(q.x), n(q.y));
                last = p;
            }
            tiny_skia::PathSegment::QuadTo(c, p) => {
                // As a cubic: its controls two thirds of the way to the quad's.
                let c1 = tiny_skia::Point::from_xy(last.x + (c.x - last.x) * 2. / 3., last.y + (c.y - last.y) * 2. / 3.);
                let c2 = tiny_skia::Point::from_xy(p.x + (c.x - p.x) * 2. / 3., p.y + (c.y - p.y) * 2. / 3.);
                let (a, b, e) = (map(c1), map(c2), map(p));
                let _ = writeln!(d, "{} {} {} {} {} {} c", n(a.x), n(a.y), n(b.x), n(b.y), n(e.x), n(e.y));
                last = p;
            }
            tiny_skia::PathSegment::CubicTo(c1, c2, p) => {
                let (a, b, e) = (map(c1), map(c2), map(p));
                let _ = writeln!(d, "{} {} {} {} {} {} c", n(a.x), n(a.y), n(b.x), n(b.y), n(e.x), n(e.y));
                last = p;
            }
            tiny_skia::PathSegment::Close => d.push_str("h\n"),
        }
    }
}

/// A gradient's stops as a PDF function from 0 to 1: one blend for each
/// pair of stops, joined, the end colours held to the ends.
fn ramp(stops: &[usvg::Stop]) -> String {
    let rgb = |s: &usvg::Stop| {
        let c = s.color();
        format!("[{} {} {}]", n(f32::from(c.red) / 255.), n(f32::from(c.green) / 255.), n(f32::from(c.blue) / 255.))
    };
    let mut at: Vec<(f32, String)> = Vec::new();
    for s in stops {
        // Each a little past the one before: PDF wants them rising.
        let o = s.offset().get().max(at.last().map_or(0., |l| l.0 + 0.0001)).min(1.);
        at.push((o, rgb(s)));
    }
    if at[0].0 > 0. {
        at.insert(0, (0., at[0].1.clone()));
    }
    if at[at.len() - 1].0 < 1. {
        at.push((1., at[at.len() - 1].1.clone()));
    }
    let blend = |a: &str, b: &str| format!("<< /FunctionType 2 /Domain [0 1] /C0 {a} /C1 {b} /N 1 >>");
    if at.len() == 1 {
        return blend(&at[0].1, &at[0].1);
    }
    let parts: Vec<String> = at.windows(2).map(|w| blend(&w[0].1, &w[1].1)).collect();
    if parts.len() == 1 {
        return parts[0].clone();
    }
    let bounds: Vec<String> = at[1..at.len() - 1].iter().map(|s| n(s.0)).collect();
    format!(
        "<< /FunctionType 3 /Domain [0 1] /Functions [{}] /Bounds [{}] /Encode [{}] >>",
        parts.join(" "),
        bounds.join(" "),
        vec!["0 1"; parts.len()].join(" ")
    )
}

/// A paint's colour (0 to 1) and opacity: a gradient takes its first stop.
fn paint(p: &usvg::Paint) -> Option<((f32, f32, f32), f32)> {
    let (c, a) = match p {
        usvg::Paint::Color(c) => (*c, 1.),
        usvg::Paint::LinearGradient(g) => g.stops().first().map(|s| (s.color(), s.opacity().get()))?,
        usvg::Paint::RadialGradient(g) => g.stops().first().map(|s| (s.color(), s.opacity().get()))?,
        usvg::Paint::Pattern(_) => return None,
    };
    Some(((f32::from(c.red) / 255., f32::from(c.green) / 255., f32::from(c.blue) / 255.), a))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sheet() -> Category {
        let b = crate::demo::board();
        b.categories.iter().find(|c| !c.stickers.is_empty()).cloned().expect("a sheet with notes")
    }

    #[test]
    fn markdown_runs_lose_their_marks() {
        let look = Look::of(&Theme::light());
        let r = runs("a **bold** and *slanted* `code` ~~gone~~ [link](https://x.y) <mark color=\"green\">hi</mark> 2 * 3", &look);
        let text: String = r.iter().map(|r| r.text.as_str()).collect();
        assert_eq!(text, "a bold and slanted code gone link hi 2 * 3");
        assert!(r.iter().any(|r| r.text == "bold" && r.style.bold));
        assert!(r.iter().any(|r| r.text == "slanted" && r.style.italic));
        assert!(r.iter().any(|r| r.text == "code" && r.style.code));
        assert!(r.iter().any(|r| r.text == "link" && r.style.link));
        assert!(r.iter().any(|r| r.text == "hi" && r.style.mark.is_some()));
    }

    #[test]
    fn lines_wrap_within_their_width() {
        let look = Look::of(&Theme::light());
        let set = Set { family: SANS, size: 13., lh: 20., weight: 400, italic: false, color: look.text };
        let lines = wrap(&runs("the quick brown fox jumps over the lazy dog again and again", &look), &set, 120.);
        assert!(lines.len() > 2);
        for line in &lines {
            let end = line.last().map_or(0., |p| p.x + p.w);
            assert!(end <= 120.5, "{end}");
            assert!(!line.first().is_some_and(|p| p.text.starts_with(' ')));
        }
    }

    #[test]
    fn shapes_are_drawn_with_their_words_turned() {
        let b = crate::demo::board();
        let net = b.categories.iter().find(|c| !c.shapes.is_empty()).expect("the shapes sheet");
        let svg = draw(net, &Theme::light(), &std::env::temp_dir(), false);
        assert!(svg.contains(r#"<g transform="rotate(15 "#), "the access point's words turn with it");
        assert!(svg.contains(">Access point</text>"));
        assert!(svg.contains(r#"stroke-dasharray="6 4.2""#));
        let png = render(net, &Theme::dark(), &std::env::temp_dir(), Format::Png).expect("a png");
        assert_eq!(&png[..8], b"\x89PNG\r\n\x1a\n");
    }

    #[test]
    fn a_sheet_makes_a_png_and_a_pdf() {
        let s = sheet();
        let dir = std::env::temp_dir();
        for t in [Theme::light(), Theme::dark()] {
            let png = render(&s, &t, &dir, Format::Png).expect("a png");
            assert_eq!(&png[..8], b"\x89PNG\r\n\x1a\n");
            let pdf = render(&s, &t, &dir, Format::Pdf).expect("a pdf");
            assert!(pdf.starts_with(b"%PDF-1.4"));
            assert!(pdf.ends_with(b"%%EOF\n"));
            // The table of where each object starts is right.
            let tail = String::from_utf8_lossy(&pdf[pdf.len() - 32..]).to_string();
            let at: usize = tail.rsplit("startxref\n").next().and_then(|r| r.lines().next()).and_then(|l| l.parse().ok()).expect("startxref");
            assert!(pdf[at..].starts_with(b"xref"));
            let table = String::from_utf8_lossy(&pdf[at..]).to_string();
            for (i, line) in table.lines().skip(3).take(5).enumerate() {
                let offset: usize = line.split(' ').next().and_then(|v| v.parse().ok()).expect("an offset");
                assert!(pdf[offset..].starts_with(format!("{} 0 obj", i + 1).as_bytes()), "object {}", i + 1);
            }
        }
    }
}
