//! The document: categories (sheets), and on each sheet its stickers, ink
//! strokes, labels written on the canvas, and links between stickers.
//! Plain data and geometry only; nothing here knows about GPUI.
//!
//! Coordinates are world units on an unbounded sheet. A sheet's camera maps
//! them to the screen: `screen = world * zoom + pan`.

use serde::{Deserialize, Serialize};

pub type Id = u64;

pub const FORMAT_VERSION: u32 = 3;
pub const MIN_ZOOM: f32 = 0.25;
pub const MAX_ZOOM: f32 = 3.0;
pub const MIN_STICKER: (f32, f32) = (140., 90.);

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Vec2 {
    pub x: f32,
    pub y: f32,
}

pub const fn v2(x: f32, y: f32) -> Vec2 {
    Vec2 { x, y }
}

impl Vec2 {
    pub fn sub(self, o: Vec2) -> Vec2 {
        v2(self.x - o.x, self.y - o.y)
    }
    pub fn add(self, o: Vec2) -> Vec2 {
        v2(self.x + o.x, self.y + o.y)
    }
    pub fn len(self) -> f32 {
        self.x.hypot(self.y)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize, Default)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

impl Rect {
    pub fn contains(&self, p: Vec2) -> bool {
        p.x >= self.x && p.x <= self.x + self.w && p.y >= self.y && p.y <= self.y + self.h
    }
    pub fn intersects(&self, o: &Rect) -> bool {
        self.x < o.x + o.w && o.x < self.x + self.w && self.y < o.y + o.h && o.y < self.y + self.h
    }
    pub fn center(&self) -> Vec2 {
        v2(self.x + self.w / 2., self.y + self.h / 2.)
    }
    pub fn union(&self, o: &Rect) -> Rect {
        let x = self.x.min(o.x);
        let y = self.y.min(o.y);
        Rect {
            x,
            y,
            w: (self.x + self.w).max(o.x + o.w) - x,
            h: (self.y + self.h).max(o.y + o.h) - y,
        }
    }

    /// Which edges of this rectangle `p` is on: up to `inside` within an
    /// edge or `outside` beyond it (a corner is two edges). Within `corner`
    /// of a corner along an edge counts as the corner, so corners are easy
    /// to catch. None away from the edges.
    pub fn grip_at(&self, p: Vec2, inside: f32, outside: f32, corner: f32) -> Option<Grip> {
        let (x1, y1) = (self.x + self.w, self.y + self.h);
        if p.x < self.x - outside || p.x > x1 + outside || p.y < self.y - outside || p.y > y1 + outside {
            return None;
        }
        // How far in from each edge (negative outside it). On a small
        // rectangle the zones overlap: the nearer edge wins.
        let on = |from_a: f32, from_b: f32| {
            let hit = |d: f32| d <= inside && d >= -outside;
            (hit(from_a) && from_a <= from_b, hit(from_b) && from_b < from_a)
        };
        let (mut left, mut right) = on(p.x - self.x, x1 - p.x);
        let (mut top, mut bottom) = on(p.y - self.y, y1 - p.y);
        let near = |from_a: f32, from_b: f32| (from_a <= corner && from_a <= from_b, from_b <= corner && from_b < from_a);
        if (left || right) && !(top || bottom) {
            (top, bottom) = near(p.y - self.y, y1 - p.y);
        } else if (top || bottom) && !(left || right) {
            (left, right) = near(p.x - self.x, x1 - p.x);
        }
        let g = Grip { left, right, top, bottom };
        (left || right || top || bottom).then_some(g)
    }

    /// This rectangle with the gripped edges moved by (dx, dy), no smaller
    /// than (min_w, min_h); the opposite edges stay put.
    pub fn resized(&self, g: Grip, dx: f32, dy: f32, min_w: f32, min_h: f32) -> Rect {
        let mut r = *self;
        if g.right {
            r.w = (self.w + dx).max(min_w);
        } else if g.left {
            r.w = (self.w - dx).max(min_w);
            r.x = self.x + self.w - r.w;
        }
        if g.bottom {
            r.h = (self.h + dy).max(min_h);
        } else if g.top {
            r.h = (self.h - dy).max(min_h);
            r.y = self.y + self.h - r.h;
        }
        r
    }
}

/// The edges a resize holds: one for a side, two for a corner.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct Grip {
    pub left: bool,
    pub right: bool,
    pub top: bool,
    pub bottom: bool,
}

/// A sticker's paper. Each has a light and a dark rendering in the theme.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum Paper {
    #[default]
    Butter,
    Sage,
    Sky,
    Blush,
    Lilac,
    Plain,
    Peach,
    Rose,
    Ocean,
    Mint,
    Sand,
    Stone,
}

impl Paper {
    /// The name used in searches: `paper:sage`.
    pub fn key(self) -> &'static str {
        match self {
            Paper::Butter => "butter",
            Paper::Sage => "sage",
            Paper::Sky => "sky",
            Paper::Blush => "blush",
            Paper::Lilac => "lilac",
            Paper::Plain => "plain",
            Paper::Peach => "peach",
            Paper::Rose => "rose",
            Paper::Ocean => "ocean",
            Paper::Mint => "mint",
            Paper::Sand => "sand",
            Paper::Stone => "stone",
        }
    }
    /// In the order the swatches show: warm to cool, then the quiet ones;
    /// Plain stays last.
    pub const ALL: [Paper; 12] = [
        Paper::Butter,
        Paper::Peach,
        Paper::Blush,
        Paper::Rose,
        Paper::Lilac,
        Paper::Sky,
        Paper::Ocean,
        Paper::Mint,
        Paper::Sage,
        Paper::Sand,
        Paper::Stone,
        Paper::Plain,
    ];
}

/// Ink for strokes, labels and links.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Ink {
    Graphite,
    Forest,
    Coral,
    Cobalt,
    Marigold,
    Ruby,
    Pink,
    Violet,
    Indigo,
    Sky,
    Teal,
    Lime,
    Tangerine,
    Brown,
    Slate,
    Silver,
    Midnight,
}

impl Ink {
    /// The five always at hand in the tool dock.
    pub const QUICK: [Ink; 5] = [Ink::Graphite, Ink::Forest, Ink::Coral, Ink::Cobalt, Ink::Marigold];
    /// Every ink, by hue, for the palette.
    pub const ALL: [Ink; 17] = [
        Ink::Ruby,
        Ink::Coral,
        Ink::Tangerine,
        Ink::Marigold,
        Ink::Lime,
        Ink::Forest,
        Ink::Teal,
        Ink::Sky,
        Ink::Cobalt,
        Ink::Indigo,
        Ink::Violet,
        Ink::Pink,
        Ink::Brown,
        Ink::Slate,
        Ink::Silver,
        Ink::Graphite,
        Ink::Midnight,
    ];
}

/// The type size of a sticker's text.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum TextSize {
    Small,
    #[default]
    Medium,
    Large,
}

impl TextSize {
    pub const ALL: [TextSize; 3] = [TextSize::Small, TextSize::Medium, TextSize::Large];
    pub fn label(self) -> &'static str {
        match self {
            TextSize::Small => "S",
            TextSize::Medium => "M",
            TextSize::Large => "L",
        }
    }
}

/// The typeface a note is written in.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum NoteFont {
    #[default]
    Sans,
    /// A casual handwritten face (Comic Neue).
    Hand,
    Mono,
}

impl NoteFont {
    pub const ALL: [NoteFont; 3] = [NoteFont::Sans, NoteFont::Hand, NoteFont::Mono];
    pub fn label(self) -> &'static str {
        match self {
            NoteFont::Sans => "Sans",
            NoteFont::Hand => "Hand",
            NoteFont::Mono => "Mono",
        }
    }
}

/// How a note is made: today's rounded card, a slip of paper taped to the
/// sheet, a cheat card with its title on a band, or a ruled index card
/// with a pin.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum NoteStyle {
    #[default]
    Card,
    Taped,
    Cheat,
    Index,
}

impl NoteStyle {
    pub const ALL: [NoteStyle; 4] = [NoteStyle::Card, NoteStyle::Taped, NoteStyle::Cheat, NoteStyle::Index];
    pub fn label(self) -> &'static str {
        match self {
            NoteStyle::Card => "Card",
            NoteStyle::Taped => "Taped",
            NoteStyle::Cheat => "Cheat card",
            NoteStyle::Index => "Index card",
        }
    }
    /// The one after this, for cycling with a key.
    pub fn next(self) -> NoteStyle {
        let i = NoteStyle::ALL.iter().position(|s| *s == self).unwrap_or(0);
        NoteStyle::ALL[(i + 1) % NoteStyle::ALL.len()]
    }
}

/// What a note is pinned up with: a shape, from the classic push pin to
/// a star or a little chip, at a size, in a colour where the shape takes
/// one.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct Pin {
    pub shape: PinShape,
    #[serde(default)]
    pub size: PinSize,
    #[serde(default)]
    pub tone: PinTone,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum PinShape {
    /// A push pin seen from the side, its point in the paper.
    #[default]
    Push,
    /// A round glass head.
    Ball,
    /// A flat metal drawing pin.
    Tack,
    Chip,
    Bug,
    Terminal,
    Rocket,
    Code,
    Heart,
    Flower,
    Bow,
    Butterfly,
    Star,
    Sparkle,
    Moon,
    Paw,
    Smiley,
    Bolt,
}

impl PinShape {
    /// The shapes in families, as the picker shows them.
    pub const FAMILIES: [(&'static str, &'static [PinShape]); 5] = [
        ("Classic", &[PinShape::Push, PinShape::Ball, PinShape::Tack]),
        ("Tech", &[PinShape::Chip, PinShape::Bug, PinShape::Terminal, PinShape::Rocket, PinShape::Code]),
        ("Cute", &[PinShape::Heart, PinShape::Flower, PinShape::Bow, PinShape::Butterfly]),
        ("Stars", &[PinShape::Star, PinShape::Sparkle, PinShape::Moon]),
        ("Fun", &[PinShape::Paw, PinShape::Smiley, PinShape::Bolt]),
    ];
    /// Its picture's name in `assets/pins`.
    pub fn key(self) -> &'static str {
        match self {
            PinShape::Push => "push",
            PinShape::Ball => "ball",
            PinShape::Tack => "tack",
            PinShape::Chip => "chip",
            PinShape::Bug => "bug",
            PinShape::Terminal => "terminal",
            PinShape::Rocket => "rocket",
            PinShape::Code => "code",
            PinShape::Heart => "heart",
            PinShape::Flower => "flower",
            PinShape::Bow => "bow",
            PinShape::Butterfly => "butterfly",
            PinShape::Star => "star",
            PinShape::Sparkle => "sparkle",
            PinShape::Moon => "moon",
            PinShape::Paw => "paw",
            PinShape::Smiley => "smiley",
            PinShape::Bolt => "bolt",
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            PinShape::Push => "Push pin",
            PinShape::Ball => "Ball",
            PinShape::Tack => "Tack",
            PinShape::Chip => "Chip",
            PinShape::Bug => "Bug",
            PinShape::Terminal => "Terminal",
            PinShape::Rocket => "Rocket",
            PinShape::Code => "Code",
            PinShape::Heart => "Heart",
            PinShape::Flower => "Flower",
            PinShape::Bow => "Bow",
            PinShape::Butterfly => "Butterfly",
            PinShape::Star => "Star",
            PinShape::Sparkle => "Sparkle",
            PinShape::Moon => "Moon",
            PinShape::Paw => "Paw",
            PinShape::Smiley => "Smiley",
            PinShape::Bolt => "Bolt",
        }
    }
    /// The colour it comes in unless one is picked, for the shapes that
    /// take one; the others keep their own colours.
    pub fn tone(self) -> Option<PinTone> {
        match self {
            PinShape::Push => Some(PinTone::Red),
            PinShape::Ball => Some(PinTone::Pink),
            PinShape::Heart => Some(PinTone::Pink),
            PinShape::Bow => Some(PinTone::Purple),
            PinShape::Star => Some(PinTone::Yellow),
            _ => None,
        }
    }
    /// How far down its picture the point it's pinned at is: a push pin
    /// stands on its point; the others sit across the note's top edge.
    pub fn anchor(self) -> f32 {
        match self {
            PinShape::Push => 0.86,
            _ => 0.55,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum PinSize {
    Small,
    #[default]
    Medium,
    Large,
}

impl PinSize {
    pub const ALL: [PinSize; 3] = [PinSize::Small, PinSize::Medium, PinSize::Large];
    pub fn label(self) -> &'static str {
        match self {
            PinSize::Small => "S",
            PinSize::Medium => "M",
            PinSize::Large => "L",
        }
    }
    /// Its width and height in world units.
    pub fn units(self) -> f32 {
        match self {
            PinSize::Small => 30.,
            PinSize::Medium => 40.,
            PinSize::Large => 56.,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum PinTone {
    #[default]
    Red,
    Pink,
    Yellow,
    Green,
    Blue,
    Purple,
    Black,
    Silver,
}

impl PinTone {
    pub const ALL: [PinTone; 8] =
        [PinTone::Red, PinTone::Pink, PinTone::Yellow, PinTone::Green, PinTone::Blue, PinTone::Purple, PinTone::Black, PinTone::Silver];
    pub fn label(self) -> &'static str {
        match self {
            PinTone::Red => "Red",
            PinTone::Pink => "Pink",
            PinTone::Yellow => "Yellow",
            PinTone::Green => "Green",
            PinTone::Blue => "Blue",
            PinTone::Purple => "Purple",
            PinTone::Black => "Black",
            PinTone::Silver => "Silver",
        }
    }
}

impl Pin {
    /// `shape`, keeping this pin's size, and its colour if the new shape
    /// takes one and this pin had one; else the shape's own.
    pub fn with_shape(self, shape: PinShape, had: bool) -> Pin {
        let tone = match (shape.tone(), had && self.shape.tone().is_some()) {
            (Some(_), true) => self.tone,
            (Some(own), false) => own,
            (None, _) => self.tone,
        };
        Pin { shape, size: self.size, tone }
    }
    /// The pin an index card comes with: a round head, its colour by note.
    pub fn index_card(id: Id) -> Pin {
        let tone = [PinTone::Pink, PinTone::Blue, PinTone::Green, PinTone::Yellow][(id % 4) as usize];
        Pin { shape: PinShape::Ball, size: PinSize::Small, tone }
    }
}

/// A web page pinned as a note.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
pub struct LinkCard {
    pub url: String,
    /// The page's title, once fetched.
    #[serde(default)]
    pub title: String,
    /// The site's icon: a file in the media folder.
    #[serde(default)]
    pub icon: Option<String>,
    /// The host, shown under the title.
    #[serde(default)]
    pub site: String,
}

/// Where a note popped out onto the desktop sits, in screen pixels.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Popped {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
    /// See-through: the desktop shows behind it.
    #[serde(default)]
    pub see_through: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
pub struct Sticker {
    pub id: Id,
    pub rect: Rect,
    pub paper: Paper,
    pub title: String,
    /// Markdown: GitHub tables, task lists, and `<mark color="…">` for
    /// coloured highlights.
    pub body: String,
    /// A picture shown above the text: a file in the media folder.
    #[serde(default)]
    pub image: Option<String>,
    #[serde(default)]
    pub text_size: TextSize,
    /// Unix seconds of the last edit.
    #[serde(default)]
    pub edited: u64,
    /// A reminder: Unix seconds.
    #[serde(default)]
    pub due: Option<u64>,
    /// The reminder has been shown.
    #[serde(default)]
    pub reminded: bool,
    /// Locked notes don't move, resize or change on a click.
    #[serde(default)]
    pub locked: bool,
    /// The height stays as set (the text may be cut off). Otherwise the
    /// note grows with its text and `rect.h` is its least height.
    #[serde(default)]
    pub fixed: bool,
    #[serde(default)]
    pub font: NoteFont,
    #[serde(default)]
    pub style: NoteStyle,
    /// What it's pinned up with, if anything.
    #[serde(default)]
    pub pin: Option<Pin>,
    #[serde(default)]
    pub link: Option<LinkCard>,
    /// Out on the desktop as its own always-on-top window.
    #[serde(default)]
    pub popped: Option<Popped>,
    /// Not flagged as stale before this (Unix seconds).
    #[serde(default)]
    pub snoozed: u64,
}

fn yes() -> bool {
    true
}

impl Sticker {
    /// Makes it `style`; an index card without a pin gets its round one.
    pub fn set_style(&mut self, style: NoteStyle) {
        self.style = style;
        if style == NoteStyle::Index && self.pin.is_none() {
            self.pin = Some(Pin::index_card(self.id));
        }
    }

    /// `#tags` in the title and body, lower-cased, without the `#`.
    pub fn tags(&self) -> Vec<String> {
        let mut t = tags(&self.title);
        for x in tags(&self.body) {
            if !t.contains(&x) {
                t.push(x);
            }
        }
        t
    }
    pub fn text(&self) -> String {
        let mut s = self.title.clone();
        if !self.body.is_empty() {
            if !s.is_empty() {
                s.push_str("\n\n");
            }
            s.push_str(&self.body);
        }
        if let Some(l) = &self.link {
            if !s.is_empty() {
                s.push_str("\n");
            }
            s.push_str(&l.url);
        }
        s
    }
    pub fn matches(&self, q: &str) -> bool {
        let q = q.trim().to_lowercase();
        if q.is_empty() {
            return true;
        }
        if let Some(p) = q.strip_prefix("paper:") {
            return self.paper.key() == p.trim();
        }
        if let Some(tag) = q.strip_prefix('#') {
            return self.tags().iter().any(|t| t.starts_with(tag));
        }
        self.text().to_lowercase().contains(&q)
    }
    /// Untouched for three weeks.
    pub fn is_stale(&self, now: u64) -> bool {
        self.edited > 0 && now.saturating_sub(self.edited) > STALE_AFTER && now >= self.snoozed
    }
}

pub const STALE_AFTER: u64 = 21 * 86_400;
pub const TRASH_KEEPS: u64 = 30 * 86_400;

/// `#word` tokens: letters, digits, `-` and `_`, after a space or the start.
pub fn tags(text: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    let chars: Vec<char> = text.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        if chars[i] == '#' && (i == 0 || chars[i - 1].is_whitespace() || chars[i - 1] == '(') {
            let mut j = i + 1;
            while j < chars.len() && (chars[j].is_alphanumeric() || chars[j] == '-' || chars[j] == '_') {
                j += 1;
            }
            if j > i + 1 && !chars[i + 1..j].iter().all(|c| c.is_ascii_digit()) {
                let t: String = chars[i + 1..j].iter().collect::<String>().to_lowercase();
                if !out.contains(&t) {
                    out.push(t);
                }
            }
            i = j;
        } else {
            i += 1;
        }
    }
    out
}

/// A Markdown task line: `- [ ] text` or `- [x] text` (also `*`, `+`).
pub fn task_line(line: &str) -> Option<(bool, &str)> {
    let t = line.trim_start();
    let rest = t.strip_prefix("- ").or_else(|| t.strip_prefix("* ")).or_else(|| t.strip_prefix("+ "))?;
    if let Some(r) = rest.strip_prefix("[ ] ") {
        Some((false, r))
    } else if let Some(r) = rest.strip_prefix("[x] ").or_else(|| rest.strip_prefix("[X] ")) {
        Some((true, r))
    } else if rest == "[ ]" {
        Some((false, ""))
    } else if rest == "[x]" || rest == "[X]" {
        Some((true, ""))
    } else {
        None
    }
}

/// Done and total tasks in a note.
pub fn task_progress(body: &str) -> (usize, usize) {
    body.lines().filter_map(task_line).fold((0, 0), |(d, n), (done, _)| (d + done as usize, n + 1))
}

/// Ticks or unticks the `n`th task line.
pub fn toggle_task(body: &str, n: usize) -> String {
    let mut seen = 0;
    let mut out: Vec<String> = Vec::new();
    for line in body.split('\n') {
        if let Some((done, _)) = task_line(line) {
            if seen == n {
                let (from, to) = if done { ("[x]", "[ ]") } else { ("[ ]", "[x]") };
                let from = if done && !line.contains("[x]") { "[X]" } else { from };
                out.push(line.replacen(from, to, 1));
                seen += 1;
                continue;
            }
            seen += 1;
        }
        out.push(line.to_string());
    }
    out.join("\n")
}

/// A titled area on a sheet; notes inside it move with it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Frame {
    pub id: Id,
    pub rect: Rect,
    pub title: String,
    pub accent: Accent,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Stroke {
    pub id: Id,
    pub ink: Ink,
    pub width: f32,
    /// A highlighter: wide and translucent.
    #[serde(default)]
    pub marker: bool,
    pub points: Vec<Vec2>,
}

impl Stroke {
    pub fn bounds(&self) -> Rect {
        let pad = self.width / 2.;
        let (mut x0, mut y0, mut x1, mut y1) = (f32::MAX, f32::MAX, f32::MIN, f32::MIN);
        for p in &self.points {
            x0 = x0.min(p.x);
            y0 = y0.min(p.y);
            x1 = x1.max(p.x);
            y1 = y1.max(p.y);
        }
        if self.points.is_empty() {
            return Rect { x: 0., y: 0., w: 0., h: 0. };
        }
        Rect { x: x0 - pad, y: y0 - pad, w: x1 - x0 + 2. * pad, h: y1 - y0 + 2. * pad }
    }

    /// Whether `p` is within `radius` of the line.
    pub fn hit(&self, p: Vec2, radius: f32) -> bool {
        let r = radius + self.width / 2.;
        match self.points.as_slice() {
            [] => false,
            [a] => a.sub(p).len() <= r,
            pts => pts.windows(2).any(|w| segment_distance(p, w[0], w[1]) <= r),
        }
    }
}

/// Text written straight on the sheet: Markdown (bold, italic, lists,
/// highlights...), in an ink, a typeface and a size, wrapping at its width.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Label {
    pub id: Id,
    pub at: Vec2,
    pub text: String,
    pub ink: Ink,
    #[serde(default)]
    pub font: NoteFont,
    /// Text written before sizes was the small size.
    #[serde(default)]
    pub size: LabelSize,
    /// Where its lines wrap, in world units; None wraps at `LABEL_WRAP`
    /// and is no wider than its longest line.
    #[serde(default)]
    pub width: Option<f32>,
    /// Slanted throughout. Text written before this was.
    #[serde(default = "slanted")]
    pub italic: bool,
}

fn slanted() -> bool {
    true
}

/// Where text on the sheet wraps unless given a width of its own.
pub const LABEL_WRAP: f32 = 480.;
/// The narrowest and widest a label's width can be set.
pub const LABEL_MIN_W: f32 = 80.;
pub const LABEL_MAX_W: f32 = 2000.;

impl Label {
    /// New text at `at`, upright, at the medium size.
    pub fn new(id: Id, at: Vec2, ink: Ink) -> Self {
        Label { id, at, text: String::new(), ink, font: NoteFont::Sans, size: LabelSize::M, width: None, italic: false }
    }
}

/// The size of text on the sheet: its type size in world units.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum LabelSize {
    #[default]
    S,
    M,
    L,
    XL,
}

impl LabelSize {
    pub const ALL: [LabelSize; 4] = [LabelSize::S, LabelSize::M, LabelSize::L, LabelSize::XL];
    pub fn units(self) -> f32 {
        match self {
            LabelSize::S => 15.,
            LabelSize::M => 20.,
            LabelSize::L => 28.,
            LabelSize::XL => 40.,
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            LabelSize::S => "S",
            LabelSize::M => "M",
            LabelSize::L => "L",
            LabelSize::XL => "XL",
        }
    }
    /// The space between one line's top and the next's.
    pub fn line(self) -> f32 {
        self.units() * 1.45
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Link {
    pub id: Id,
    pub from: Id,
    pub to: Id,
    pub ink: Ink,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Camera {
    pub pan: Vec2,
    pub zoom: f32,
}

impl Default for Camera {
    fn default() -> Self {
        Camera { pan: v2(48., 120.), zoom: 1. }
    }
}

impl Camera {
    pub fn to_screen(&self, p: Vec2) -> Vec2 {
        v2(p.x * self.zoom + self.pan.x, p.y * self.zoom + self.pan.y)
    }
    pub fn to_world(&self, s: Vec2) -> Vec2 {
        v2((s.x - self.pan.x) / self.zoom, (s.y - self.pan.y) / self.zoom)
    }
    /// The world area a viewport of `w`×`h` shows.
    pub fn visible(&self, w: f32, h: f32) -> Rect {
        let a = self.to_world(v2(0., 0.));
        Rect { x: a.x, y: a.y, w: w / self.zoom, h: h / self.zoom }
    }
    /// Zooms by `factor`, keeping the world point under `anchor` (screen) still.
    pub fn zoom_at(&mut self, anchor: Vec2, factor: f32) {
        let before = self.to_world(anchor);
        self.zoom = (self.zoom * factor).clamp(MIN_ZOOM, MAX_ZOOM);
        self.pan = v2(anchor.x - before.x * self.zoom, anchor.y - before.y * self.zoom);
    }
    /// Frames `r` in a `w`×`h` viewport with `margin` pixels around it.
    pub fn fit(&mut self, r: Rect, w: f32, h: f32, margin: f32) {
        let zx = (w - 2. * margin).max(1.) / r.w.max(1.);
        let zy = (h - 2. * margin).max(1.) / r.h.max(1.);
        self.zoom = zx.min(zy).clamp(MIN_ZOOM, 1.25);
        let c = r.center();
        self.pan = v2(w / 2. - c.x * self.zoom, h / 2. - c.y * self.zoom);
    }
}

/// The dot in the sidebar beside a category.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Accent {
    Amber,
    Slate,
    Rose,
    Moss,
    Plum,
}

impl Accent {
    pub const ALL: [Accent; 5] = [Accent::Amber, Accent::Slate, Accent::Rose, Accent::Moss, Accent::Plum];
}

/// The pattern a sheet is ruled with, in world units so it moves and
/// zooms with the sheet.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum Pattern {
    #[default]
    Dots,
    /// Lines across and down: squared paper.
    Grid,
    /// Squared paper with a heavier line every fifth.
    Graph,
    /// Lines across, like a notebook.
    Lines,
    /// A small cross where the lines would meet.
    Crosses,
    /// Dots on a triangle lattice, for isometric sketches.
    Isometric,
    Plain,
}

impl Pattern {
    pub const ALL: [Pattern; 7] =
        [Pattern::Dots, Pattern::Grid, Pattern::Graph, Pattern::Lines, Pattern::Crosses, Pattern::Isometric, Pattern::Plain];
    pub fn label(self) -> &'static str {
        match self {
            Pattern::Dots => "Dots",
            Pattern::Grid => "Grid",
            Pattern::Graph => "Graph",
            Pattern::Lines => "Lines",
            Pattern::Crosses => "Crosses",
            Pattern::Isometric => "Isometric",
            Pattern::Plain => "Plain",
        }
    }
}

/// The colour of a sheet's paper. Each has a light and a dark rendering.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum Tint {
    #[default]
    Paper,
    Warm,
    Cool,
    Mint,
    Blush,
    Lavender,
}

impl Tint {
    pub const ALL: [Tint; 6] = [Tint::Paper, Tint::Warm, Tint::Cool, Tint::Mint, Tint::Blush, Tint::Lavender];
    pub fn label(self) -> &'static str {
        match self {
            Tint::Paper => "Paper",
            Tint::Warm => "Warm",
            Tint::Cool => "Cool",
            Tint::Mint => "Mint",
            Tint::Blush => "Blush",
            Tint::Lavender => "Lavender",
        }
    }
}

/// What a sheet is drawn on: a pattern on a tint, and maybe a picture
/// behind it all.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct Backdrop {
    pub pattern: Pattern,
    pub tint: Tint,
    /// A picture (PNG, JPEG, GIF, WebP or SVG) filling the sheet's window,
    /// still while the sheet moves: a file in the media folder.
    pub image: Option<String>,
    /// The picture faded toward the paper, so notes stay easy to read.
    pub soften: bool,
}

/// One line of a pattern on screen: across (`y`) or down (`x`), and
/// whether it's a heavier one.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Rule {
    pub across: bool,
    pub at: f32,
    pub major: bool,
}

/// The spacing, in screen pixels, a pattern of `grid` world units is drawn
/// at: doubled while it's tighter than `least`, so lines and dots never
/// crowd together when zoomed out.
pub fn pattern_step(grid: f32, zoom: f32, least: f32) -> f32 {
    let mut step = grid * zoom;
    while step < least {
        step *= 2.;
    }
    step
}

/// The lines a pattern draws over a `w`×`h` view through `cam`: `every`
/// lines along each way, every `major`th one heavier (counted from the
/// world's origin, so they stay put as the sheet moves).
pub fn rules(cam: Camera, w: f32, h: f32, step: f32, across: bool, down: bool, major: Option<u32>) -> Vec<Rule> {
    let mut out = Vec::new();
    let mut add = |vertical: bool, pan: f32, len: f32| {
        let first = (-pan / step).ceil() as i64;
        let mut i = first;
        loop {
            let at = pan + i as f32 * step;
            if at > len {
                break;
            }
            let heavy = major.is_some_and(|m| i.rem_euclid(m as i64) == 0);
            out.push(Rule { across: !vertical, at, major: heavy });
            i += 1;
        }
    };
    if across {
        add(false, cam.pan.y, h);
    }
    if down {
        add(true, cam.pan.x, w);
    }
    out
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Category {
    pub id: Id,
    pub name: String,
    #[serde(default)]
    pub blurb: String,
    pub accent: Accent,
    /// Where the sheet is scrolled to on this computer. Kept in
    /// `view.json`, not in the board: read from older boards, never
    /// written into it.
    #[serde(default, skip_serializing)]
    pub camera: Camera,
    /// Back to front.
    pub stickers: Vec<Sticker>,
    pub strokes: Vec<Stroke>,
    pub labels: Vec<Label>,
    pub links: Vec<Link>,
    #[serde(default)]
    pub frames: Vec<Frame>,
    /// The Inbox: where quick notes land.
    #[serde(default)]
    pub inbox: bool,
    /// Pinned sheets come first.
    #[serde(default)]
    pub pinned: bool,
    /// An emoji shown beside the name.
    #[serde(default)]
    pub icon: String,
    /// Unix seconds of the last visit and the last change.
    #[serde(default)]
    pub opened: u64,
    #[serde(default)]
    pub edited: u64,
    /// What the sheet is drawn on.
    #[serde(default)]
    pub backdrop: Backdrop,
}

impl Category {
    pub fn new(id: Id, name: &str, accent: Accent) -> Self {
        Category {
            id,
            name: name.into(),
            blurb: String::new(),
            accent,
            camera: Camera::default(),
            stickers: Vec::new(),
            strokes: Vec::new(),
            labels: Vec::new(),
            links: Vec::new(),
            frames: Vec::new(),
            inbox: false,
            pinned: false,
            icon: String::new(),
            opened: 0,
            edited: 0,
            backdrop: Backdrop::default(),
        }
    }

    pub fn sticker(&self, id: Id) -> Option<&Sticker> {
        self.stickers.iter().find(|s| s.id == id)
    }
    pub fn sticker_mut(&mut self, id: Id) -> Option<&mut Sticker> {
        self.stickers.iter_mut().find(|s| s.id == id)
    }

    /// The front-most sticker under `p` (by stored size; the views use the
    /// size as drawn).
    #[cfg(test)]
    pub fn sticker_at(&self, p: Vec2) -> Option<Id> {
        self.stickers.iter().rev().find(|s| s.rect.contains(p)).map(|s| s.id)
    }

    pub fn bring_to_front(&mut self, id: Id) {
        if let Some(i) = self.stickers.iter().position(|s| s.id == id) {
            let s = self.stickers.remove(i);
            self.stickers.push(s);
        }
    }

    pub fn remove_sticker(&mut self, id: Id) {
        self.stickers.retain(|s| s.id != id);
        self.links.retain(|l| l.from != id && l.to != id);
    }

    /// Links `a` to `b`, or unlinks them when they already are. Returns
    /// whether a link now exists.
    pub fn toggle_link(&mut self, id: Id, a: Id, b: Id, ink: Ink) -> bool {
        if a == b {
            return false;
        }
        let same = |l: &Link| (l.from == a && l.to == b) || (l.from == b && l.to == a);
        if self.links.iter().any(same) {
            self.links.retain(|l| !same(l));
            false
        } else {
            self.links.push(Link { id, from: a, to: b, ink });
            true
        }
    }

    /// Removes every stroke within `radius` of `p`; true if any went.
    pub fn erase_at(&mut self, p: Vec2, radius: f32) -> bool {
        let before = self.strokes.len();
        self.strokes.retain(|s| !s.hit(p, radius));
        before != self.strokes.len()
    }

    /// Everything on the sheet, or None for an empty one.
    pub fn content_bounds(&self) -> Option<Rect> {
        let rects = self
            .stickers
            .iter()
            .map(|s| s.rect)
            .chain(self.strokes.iter().map(Stroke::bounds))
            .chain(self.labels.iter().map(label_rect))
            .chain(self.frames.iter().map(|f| f.rect));
        rects.reduce(|a, b| a.union(&b))
    }

    /// The front-most frame whose title bar or empty area holds `p`.
    pub fn frame_at(&self, p: Vec2) -> Option<Id> {
        self.frames.iter().rev().find(|f| f.rect.contains(p)).map(|f| f.id)
    }

    /// The stickers whose centre lies inside frame `id`.
    pub fn frame_members(&self, id: Id) -> Vec<Id> {
        let Some(f) = self.frames.iter().find(|f| f.id == id) else { return Vec::new() };
        self.stickers.iter().filter(|s| f.rect.contains(s.rect.center())).map(|s| s.id).collect()
    }

    /// Lays `ids` out in tidy rows, in reading order of where they are now,
    /// from the top-left of where they were.
    pub fn tidy(&mut self, ids: &[Id], heights: &dyn Fn(&Sticker) -> f32) {
        let mut items: Vec<(Id, Rect)> =
            self.stickers.iter().filter(|s| ids.contains(&s.id) && !s.locked).map(|s| (s.id, s.rect)).collect();
        if items.is_empty() {
            return;
        }
        items.sort_by(|a, b| {
            let row = |r: &Rect| (r.y / 120.).round() as i64;
            row(&a.1).cmp(&row(&b.1)).then(a.1.x.total_cmp(&b.1.x))
        });
        let x0 = items.iter().map(|i| i.1.x).fold(f32::MAX, f32::min);
        let y0 = items.iter().map(|i| i.1.y).fold(f32::MAX, f32::min);
        let cols = (items.len() as f32).sqrt().ceil().max(1.) as usize;
        let gap = 28.;
        let (mut x, mut y, mut row_h) = (x0, y0, 0f32);
        for (i, (id, _)) in items.iter().enumerate() {
            if i > 0 && i % cols == 0 {
                x = x0;
                y += row_h + gap;
                row_h = 0.;
            }
            if let Some(s) = self.sticker_mut(*id) {
                s.rect.x = x;
                s.rect.y = y;
                x += s.rect.w + gap;
                let h = heights(s);
                row_h = row_h.max(h);
            }
        }
    }

    /// A new note's paper: the one after the newest note's, so neighbours
    /// differ (Plain is skipped).
    pub fn next_paper(&self, default: Paper) -> Paper {
        let Some(last) = self.stickers.last().map(|s| s.paper) else { return default };
        let i = Paper::ALL.iter().position(|p| *p == last).unwrap_or(0);
        Paper::ALL[(i + 1) % (Paper::ALL.len() - 1)]
    }

    /// A new note's type and font: those of the note changed last (not a
    /// picture or a link), so a run of cheat cards is chosen once.
    pub fn next_look(&self, style: NoteStyle, font: NoteFont) -> (NoteStyle, NoteFont) {
        self.stickers
            .iter()
            .filter(|s| s.image.is_none() && s.link.is_none())
            .max_by_key(|s| s.edited)
            .map_or((style, font), |s| (s.style, s.font))
    }

    /// A free place for a new note: to the right of everything, top-aligned.
    pub fn free_spot(&self, w: f32) -> Vec2 {
        match self.content_bounds() {
            Some(r) if !self.stickers.is_empty() => {
                // Fill the first row before starting a new one.
                let row: Vec<&Sticker> = self.stickers.iter().filter(|s| s.rect.y <= r.y + 40.).collect();
                let right = row.iter().map(|s| s.rect.x + s.rect.w).fold(r.x, f32::max);
                if right + w - r.x <= 1200. {
                    v2(right + 28., r.y)
                } else {
                    v2(r.x, r.y + r.h + 28.)
                }
            }
            _ => v2(0., 0.),
        }
    }

    /// The nearest sticker from `from` in a direction (dx, dy each -1, 0 or 1).
    pub fn neighbour(&self, from: Id, dx: f32, dy: f32) -> Option<Id> {
        let a = self.sticker(from)?.rect.center();
        self.stickers
            .iter()
            .filter(|s| s.id != from)
            .filter_map(|s| {
                let d = s.rect.center().sub(a);
                let along = d.x * dx + d.y * dy;
                if along <= 1. {
                    return None;
                }
                let across = (d.x * dy).abs() + (d.y * dx).abs();
                Some((s.id, along + across * 2.))
            })
            .min_by(|x, y| x.1.total_cmp(&y.1))
            .map(|x| x.0)
    }

}

/// A label's box: 7 units per character at the 14-unit label size.
/// About where a label's text lies, from its length, size and width: each
/// line wraps where it would pass the width. The view measures the real
/// box as it draws; this is for before it has.
pub fn label_rect(l: &Label) -> Rect {
    let glyph = l.size.units() * 0.49;
    let wrap = l.width.unwrap_or(LABEL_WRAP);
    let (mut widest, mut lines) = (4. * glyph, 0.);
    for line in l.text.lines().chain(l.text.is_empty().then_some("")) {
        // Markdown's marks take no room once drawn.
        let chars = line.chars().filter(|c| !matches!(c, '*' | '_' | '~' | '`')).count().max(1) as f32;
        let w = chars * glyph;
        widest = widest.max(w.min(wrap));
        lines += (w / wrap).ceil().max(1.);
    }
    let w = if l.width.is_some() { wrap } else { widest };
    Rect { x: l.at.x, y: l.at.y, w, h: lines * l.size.line() }
}

/// The curve a link follows: from the edge of one rect toward the other,
/// leaving and arriving along the axis that separates them. Returns the
/// start, the two control points and the end.
pub fn link_curve(a: &Rect, b: &Rect) -> [Vec2; 4] {
    let (ca, cb) = (a.center(), b.center());
    let d = cb.sub(ca);
    let horizontal = d.x.abs() * a.h.max(b.h) >= d.y.abs() * a.w.max(b.w);
    if horizontal {
        let sx = d.x.signum();
        let p0 = v2(ca.x + sx * a.w / 2., ca.y);
        let p3 = v2(cb.x - sx * b.w / 2., cb.y);
        let k = ((p3.x - p0.x).abs() * 0.5).max(40.);
        [p0, v2(p0.x + sx * k, p0.y), v2(p3.x - sx * k, p3.y), p3]
    } else {
        let sy = d.y.signum();
        let p0 = v2(ca.x, ca.y + sy * a.h / 2.);
        let p3 = v2(cb.x, cb.y - sy * b.h / 2.);
        let k = ((p3.y - p0.y).abs() * 0.5).max(40.);
        [p0, v2(p0.x, p0.y + sy * k), v2(p3.x, p3.y - sy * k), p3]
    }
}

pub fn segment_distance(p: Vec2, a: Vec2, b: Vec2) -> f32 {
    let ab = b.sub(a);
    let len2 = ab.x * ab.x + ab.y * ab.y;
    if len2 == 0. {
        return p.sub(a).len();
    }
    let t = (((p.x - a.x) * ab.x + (p.y - a.y) * ab.y) / len2).clamp(0., 1.);
    p.sub(v2(a.x + ab.x * t, a.y + ab.y * t)).len()
}

/// Ramer–Douglas–Peucker: drops points that lie within `epsilon` of the line
/// through their neighbours, so a finished stroke keeps its shape at a
/// fraction of the points the mouse produced.
pub fn simplify(points: &[Vec2], epsilon: f32) -> Vec<Vec2> {
    if points.len() < 3 {
        return points.to_vec();
    }
    let mut keep = vec![false; points.len()];
    keep[0] = true;
    keep[points.len() - 1] = true;
    let mut stack = vec![(0, points.len() - 1)];
    while let Some((s, e)) = stack.pop() {
        let mut far = (0., s);
        for i in s + 1..e {
            let d = segment_distance(points[i], points[s], points[e]);
            if d > far.0 {
                far = (d, i);
            }
        }
        if far.0 > epsilon {
            keep[far.1] = true;
            stack.push((s, far.1));
            stack.push((far.1, e));
        }
    }
    points.iter().zip(keep).filter(|(_, k)| *k).map(|(p, _)| *p).collect()
}

/// Where a new note the size of `want` goes: at `want` if nothing is there,
/// otherwise the nearest free place on a grid of note-sized steps around
/// it (right, below, left, above, then further out), inside `view` if one
/// is free there. Never on top of what's in `taken`.
pub fn open_spot(taken: &[Rect], want: Rect, view: Rect) -> Rect {
    const GAP: f32 = 24.;
    let free = |r: &Rect| {
        let room = Rect { x: r.x - GAP / 2., y: r.y - GAP / 2., w: r.w + GAP, h: r.h + GAP };
        !taken.iter().any(|t| t.intersects(&room))
    };
    let inside = |r: &Rect| r.x >= view.x && r.y >= view.y && r.x + r.w <= view.x + view.w && r.y + r.h <= view.y + view.h;
    let (dx, dy) = (want.w + GAP, want.h + GAP);
    let mut outside = None;
    for ring in 0..=8i32 {
        let mut spots: Vec<(i32, f32, Rect)> = Vec::new();
        for i in -ring..=ring {
            for j in -ring..=ring {
                if i.abs().max(j.abs()) == ring {
                    // Clockwise from the right: right, below, left, above.
                    let turn = (j as f32).atan2(i as f32).rem_euclid(std::f32::consts::TAU);
                    spots.push((i * i + j * j, turn, Rect { x: want.x + i as f32 * dx, y: want.y + j as f32 * dy, ..want }));
                }
            }
        }
        spots.sort_by(|a, b| a.0.cmp(&b.0).then(a.1.total_cmp(&b.1)));
        for (_, _, r) in spots {
            if free(&r) {
                if inside(&r) {
                    return r;
                }
                outside.get_or_insert(r);
            }
        }
    }
    outside.unwrap_or(want)
}

/// The whole workspace, as saved.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Board {
    pub version: u32,
    pub categories: Vec<Category>,
    pub active: usize,
    /// Old files: the theme before `settings` existed.
    #[serde(default, skip_serializing)]
    pub dark: bool,
    pub next_id: Id,
    #[serde(default)]
    pub settings: Settings,
    /// Deleted (for 30 days) and archived notes and sheets.
    #[serde(default)]
    pub bin: Vec<Binned>,
    /// The home screen was showing when the app closed.
    #[serde(default = "yes")]
    pub at_home: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum ThemeMode {
    #[default]
    Light,
    Dark,
    /// Follow Windows.
    System,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum OpenOn {
    #[default]
    Home,
    LastSheet,
}

/// The quick note's shortcut until you choose another.
pub const QUICK_KEYS: &str = "Ctrl+Alt+N";

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub theme: ThemeMode,
    pub open_on: OpenOn,
    pub start_with_windows: bool,
    /// A shortcut opens a quick note from anywhere.
    pub quick_hotkey: bool,
    /// That shortcut, as it's written: "Ctrl+Alt+N".
    pub quick_keys: String,
    /// Where quick notes go, the sheet picked last; None is the Inbox.
    pub quick_sheet: Option<Id>,
    pub default_paper: Paper,
    pub default_font: NoteFont,
    pub default_style: NoteStyle,
    /// Interface size, 0.9 to 1.3.
    pub ui_scale: f32,
    pub high_contrast: bool,
    pub reduce_motion: bool,
    pub snap: bool,
    /// Moved notes land on the dot grid.
    pub grid: bool,
    pub minimap: bool,
    /// What Folio calls you (asked on the welcome screen).
    pub user_name: String,
    /// Your avatar, by name (`avatars.rs`).
    pub avatar: String,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            theme: ThemeMode::Light,
            open_on: OpenOn::Home,
            start_with_windows: false,
            quick_hotkey: true,
            quick_keys: QUICK_KEYS.into(),
            quick_sheet: None,
            default_paper: Paper::Butter,
            default_font: NoteFont::Sans,
            default_style: NoteStyle::Card,
            ui_scale: 1.0,
            high_contrast: false,
            reduce_motion: false,
            snap: true,
            grid: false,
            minimap: true,
            user_name: String::new(),
            avatar: String::new(),
        }
    }
}

/// Something in the Trash or the Archive.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Binned {
    pub id: Id,
    /// Unix seconds it went in.
    pub at: u64,
    /// Archived: kept until you delete it. Otherwise trashed for 30 days.
    pub archived: bool,
    pub item: BinItem,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum BinItem {
    Note { sheet: Id, sheet_name: String, sticker: Sticker },
    Sheet(Category),
}

impl Binned {
    pub fn name(&self) -> String {
        match &self.item {
            BinItem::Note { sticker, .. } => {
                let t = sticker.text();
                let first = t.lines().find(|l| !l.trim().is_empty()).unwrap_or("Untitled note");
                first.chars().take(60).collect()
            }
            BinItem::Sheet(c) => c.name.clone(),
        }
    }
}

/// A search hit: a note on a sheet.
#[derive(Clone, Debug, PartialEq)]
pub struct Hit {
    pub sheet: usize,
    pub note: Id,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Template {
    Blank,
    Kanban,
    Week,
    Brainstorm,
    Meeting,
    Reading,
}

impl Template {
    pub const ALL: [Template; 6] =
        [Template::Blank, Template::Kanban, Template::Week, Template::Brainstorm, Template::Meeting, Template::Reading];
    pub fn name(self) -> &'static str {
        match self {
            Template::Blank => "Blank",
            Template::Kanban => "Kanban",
            Template::Week => "Weekly planner",
            Template::Brainstorm => "Brainstorm",
            Template::Meeting => "Meeting notes",
            Template::Reading => "Reading list",
        }
    }
    pub fn blurb(self) -> &'static str {
        match self {
            Template::Blank => "An empty sheet",
            Template::Kanban => "To do, Doing, Done",
            Template::Week => "A column for every day",
            Template::Brainstorm => "One idea in the middle, thoughts around it",
            Template::Meeting => "Agenda, notes, decisions, actions",
            Template::Reading => "Books and articles to get to",
        }
    }
}

impl Board {
    pub fn fresh_id(&mut self) -> Id {
        self.next_id += 1;
        self.next_id
    }
    pub fn sheet(&self) -> &Category {
        &self.categories[self.active.min(self.categories.len() - 1)]
    }
    pub fn sheet_mut(&mut self) -> &mut Category {
        let i = self.active.min(self.categories.len() - 1);
        &mut self.categories[i]
    }
    /// Adds a sheet after the others and makes it the active one.
    pub fn add_sheet(&mut self, name: &str) -> Id {
        let id = self.fresh_id();
        let accent = Accent::ALL[self.categories.len() % Accent::ALL.len()];
        self.categories.push(Category::new(id, name, accent));
        self.active = self.categories.len() - 1;
        id
    }
    /// Repairs what a hand-edited or older file might get wrong.
    pub fn normalize(&mut self) {
        if self.version < 3 {
            // Folio no longer starts with sample sheets: the ones nobody
            // touched go (edited ones stay).
            crate::demo::remove_untouched(self);
            if self.categories.is_empty() {
                self.add_sheet_with_icon("My notes", "📝");
            }
        }
        if self.categories.is_empty() {
            *self = Board::empty();
        }
        self.active = self.active.min(self.categories.len() - 1);
        let max = self
            .categories
            .iter()
            .flat_map(|c| {
                std::iter::once(c.id)
                    .chain(c.stickers.iter().map(|s| s.id))
                    .chain(c.strokes.iter().map(|s| s.id))
                    .chain(c.labels.iter().map(|s| s.id))
                    .chain(c.links.iter().map(|s| s.id))
                    .chain(c.frames.iter().map(|s| s.id))
            })
            .chain(self.bin.iter().map(|b| b.id))
            .max()
            .unwrap_or(0);
        self.next_id = self.next_id.max(max);
        if self.version < 2 && self.dark {
            self.settings.theme = ThemeMode::Dark;
        }
        self.version = FORMAT_VERSION;
        self.settings.ui_scale = self.settings.ui_scale.clamp(0.8, 1.4);
        for c in &mut self.categories {
            c.camera.zoom = c.camera.zoom.clamp(MIN_ZOOM, MAX_ZOOM);
            for s in &mut c.stickers {
                s.rect.w = s.rect.w.max(MIN_STICKER.0);
                s.rect.h = s.rect.h.max(MIN_STICKER.1);
            }
            let ids: Vec<Id> = c.stickers.iter().map(|s| s.id).collect();
            c.links.retain(|l| ids.contains(&l.from) && ids.contains(&l.to));
        }
    }
    /// A first launch: one empty sheet.
    pub fn empty() -> Board {
        let mut b = Board::blank();
        b.add_sheet_with_icon("My notes", "📝");
        b
    }

    fn add_sheet_with_icon(&mut self, name: &str, icon: &str) {
        self.add_sheet(name);
        let t = crate::time::now();
        let sheet = self.sheet_mut();
        sheet.icon = icon.into();
        sheet.edited = t;
        sheet.opened = t;
        self.active = 0;
    }

    /// No sheets at all; callers add them.
    pub fn blank() -> Board {
        Board {
            version: FORMAT_VERSION,
            categories: Vec::new(),
            active: 0,
            dark: false,
            next_id: 0,
            settings: Settings::default(),
            bin: Vec::new(),
            at_home: true,
        }
    }

    pub fn sheet_ix(&self, id: Id) -> Option<usize> {
        self.categories.iter().position(|c| c.id == id)
    }

    /// The sheet holding note `id`.
    pub fn sheet_of(&self, id: Id) -> Option<usize> {
        self.categories.iter().position(|c| c.stickers.iter().any(|s| s.id == id))
    }

    pub fn find_note(&self, id: Id) -> Option<&Sticker> {
        self.categories.iter().find_map(|c| c.sticker(id))
    }

    pub fn find_note_mut(&mut self, id: Id) -> Option<&mut Sticker> {
        self.categories.iter_mut().find_map(|c| c.sticker_mut(id))
    }

    /// The Inbox sheet's index, making it (first in the list) if needed.
    pub fn inbox(&mut self) -> usize {
        if let Some(i) = self.categories.iter().position(|c| c.inbox) {
            return i;
        }
        let id = self.fresh_id();
        let mut c = Category::new(id, "Inbox", Accent::Rose);
        c.inbox = true;
        c.pinned = true;
        c.icon = "📥".into();
        c.blurb = "Quick notes land here; sort them when you have a minute".into();
        self.categories.insert(0, c);
        if !self.categories.is_empty() && self.active + 1 < self.categories.len() {
            self.active += 1;
        }
        0
    }

    /// A quick note into the Inbox: the first line is its title.
    #[cfg(test)]
    pub fn quick_note(&mut self, text: &str, now: u64) -> Id {
        self.quick_note_to(text, None, now)
    }

    /// A quick note onto sheet `to` (by id), or the Inbox when that's None
    /// or gone; the first line is its title. Where it went, and its id.
    pub fn quick_note_to(&mut self, text: &str, to: Option<Id>, now: u64) -> Id {
        let text = text.trim();
        let (title, body) = match text.split_once('\n') {
            Some((t, b)) if t.chars().count() <= 60 => (t.trim().to_string(), b.trim().to_string()),
            _ if text.chars().count() <= 60 && !text.contains('\n') => (text.to_string(), String::new()),
            _ => (String::new(), text.to_string()),
        };
        let paper = self.settings.default_paper;
        let font = self.settings.default_font;
        let id = self.fresh_id();
        let ix = match to.and_then(|to| self.categories.iter().position(|c| c.id == to)) {
            Some(ix) => ix,
            None => self.inbox(),
        };
        let sheet = &mut self.categories[ix];
        let at = sheet.free_spot(240.);
        sheet.stickers.push(Sticker {
            id,
            rect: Rect { x: at.x, y: at.y, w: 240., h: 100. },
            paper,
            title,
            body,
            font,
            edited: now,
            ..Default::default()
        });
        sheet.edited = now;
        id
    }

    /// Notes matching `q` (text, or `#tag`), every sheet, in sheet order.
    pub fn search(&self, q: &str) -> Vec<Hit> {
        if q.trim().is_empty() {
            return Vec::new();
        }
        self.categories
            .iter()
            .enumerate()
            .flat_map(|(i, c)| c.stickers.iter().filter(|s| s.matches(q)).map(move |s| Hit { sheet: i, note: s.id }))
            .collect()
    }

    /// Every `#tag` in use, most used first.
    pub fn all_tags(&self) -> Vec<(String, usize)> {
        let mut counts: Vec<(String, usize)> = Vec::new();
        for s in self.categories.iter().flat_map(|c| &c.stickers) {
            for t in s.tags() {
                match counts.iter_mut().find(|c| c.0 == t) {
                    Some(c) => c.1 += 1,
                    None => counts.push((t, 1)),
                }
            }
        }
        counts.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
        counts
    }

    /// Notes with a reminder, soonest first: (sheet index, note id, due).
    pub fn reminders(&self) -> Vec<(usize, Id, u64)> {
        let mut v: Vec<(usize, Id, u64)> = self
            .categories
            .iter()
            .enumerate()
            .flat_map(|(i, c)| c.stickers.iter().filter_map(move |s| s.due.map(|d| (i, s.id, d))))
            .collect();
        v.sort_by_key(|x| x.2);
        v
    }

    /// Reminders that came due and haven't been shown; marks them shown.
    pub fn take_due(&mut self, now: u64) -> Vec<Id> {
        let mut out = Vec::new();
        for s in self.categories.iter_mut().flat_map(|c| c.stickers.iter_mut()) {
            if let Some(d) = s.due {
                if d <= now && !s.reminded {
                    s.reminded = true;
                    out.push(s.id);
                }
            }
        }
        out
    }

    /// Moves note `id` to sheet `to`, at a free spot there. Links go.
    pub fn move_note(&mut self, id: Id, to: usize) -> bool {
        let Some(from) = self.sheet_of(id) else { return false };
        if from == to || to >= self.categories.len() {
            return false;
        }
        let Some(mut s) = self.categories[from].sticker(id).cloned() else { return false };
        self.categories[from].remove_sticker(id);
        let at = self.categories[to].free_spot(s.rect.w);
        s.rect.x = at.x;
        s.rect.y = at.y;
        self.categories[to].stickers.push(s);
        true
    }

    /// Puts note `id` in the Trash (or the Archive).
    pub fn bin_note(&mut self, id: Id, archived: bool, now: u64) -> bool {
        let Some(ix) = self.sheet_of(id) else { return false };
        let c = &mut self.categories[ix];
        let Some(sticker) = c.sticker(id).cloned() else { return false };
        c.remove_sticker(id);
        let (sheet, sheet_name) = (c.id, c.name.clone());
        let bid = self.fresh_id();
        self.bin.push(Binned { id: bid, at: now, archived, item: BinItem::Note { sheet, sheet_name, sticker } });
        true
    }

    /// Puts sheet `ix` in the Trash (or the Archive); the last can't go.
    pub fn bin_sheet(&mut self, ix: usize, archived: bool, now: u64) -> bool {
        if self.categories.len() <= 1 || ix >= self.categories.len() {
            return false;
        }
        let c = self.categories.remove(ix);
        self.active = self.active.min(self.categories.len() - 1);
        let bid = self.fresh_id();
        self.bin.push(Binned { id: bid, at: now, archived, item: BinItem::Sheet(c) });
        true
    }

    /// Takes `bin_id` out of the bin, back where it came from (a note
    /// whose sheet is gone goes to the Inbox). Returns the sheet index.
    pub fn restore(&mut self, bin_id: Id) -> Option<usize> {
        let i = self.bin.iter().position(|b| b.id == bin_id)?;
        let b = self.bin.remove(i);
        match b.item {
            BinItem::Sheet(c) => {
                self.categories.push(c);
                Some(self.categories.len() - 1)
            }
            BinItem::Note { sheet, mut sticker, .. } => {
                let ix = match self.sheet_ix(sheet) {
                    Some(ix) => ix,
                    None => self.inbox(),
                };
                let c = &mut self.categories[ix];
                if c.stickers.iter().any(|s| s.rect.intersects(&sticker.rect)) {
                    let at = c.free_spot(sticker.rect.w);
                    sticker.rect.x = at.x;
                    sticker.rect.y = at.y;
                }
                c.stickers.push(sticker);
                Some(ix)
            }
        }
    }

    /// Empties trash older than 30 days.
    pub fn purge(&mut self, now: u64) {
        self.bin.retain(|b| b.archived || now.saturating_sub(b.at) < TRASH_KEEPS);
    }

    /// Adds a copy of `c` (from a backup, or a duplicate) with fresh ids
    /// throughout, after the other sheets; returns its index.
    pub fn adopt_sheet(&mut self, mut c: Category, name: String) -> usize {
        c.id = self.fresh_id();
        c.name = name;
        c.inbox = false;
        c.pinned = false;
        let mut map = std::collections::HashMap::new();
        for s in &mut c.stickers {
            let n = self.fresh_id();
            map.insert(s.id, n);
            s.id = n;
            s.popped = None;
        }
        for l in &mut c.links {
            l.id = self.fresh_id();
            l.from = map.get(&l.from).copied().unwrap_or(0);
            l.to = map.get(&l.to).copied().unwrap_or(0);
        }
        c.links.retain(|l| l.from != 0 && l.to != 0);
        for x in &mut c.strokes {
            x.id = self.fresh_id();
        }
        for x in &mut c.labels {
            x.id = self.fresh_id();
        }
        for x in &mut c.frames {
            x.id = self.fresh_id();
        }
        self.categories.push(c);
        self.categories.len() - 1
    }

    /// Sheets for lists: pinned first (Inbox first of those), then the rest
    /// in their own order.
    pub fn ordered(&self) -> Vec<usize> {
        let mut v: Vec<usize> = (0..self.categories.len()).collect();
        v.sort_by_key(|&i| {
            let c = &self.categories[i];
            (!c.inbox, !c.pinned)
        });
        v
    }

    /// A new sheet from a template, made active.
    pub fn add_from_template(&mut self, name: &str, t: Template, accent: Accent, now: u64) -> Id {
        let id = self.add_sheet(name);
        let mut ids = (0..32).map(|_| 0).collect::<Vec<Id>>();
        for x in ids.iter_mut() {
            *x = self.fresh_id();
        }
        let mut next = ids.into_iter();
        let mut id_ = || next.next().unwrap();
        let note = |id: Id, x: f32, y: f32, w: f32, paper: Paper, title: &str, body: &str| Sticker {
            id,
            rect: Rect { x, y, w, h: 110. },
            paper,
            title: title.into(),
            body: body.into(),
            edited: now,
            ..Default::default()
        };
        let frame = |id: Id, x: f32, y: f32, w: f32, h: f32, title: &str, accent: Accent| Frame {
            id,
            rect: Rect { x, y, w, h },
            title: title.into(),
            accent,
        };
        let c = self.sheet_mut();
        c.accent = accent;
        c.opened = now;
        c.edited = now;
        match t {
            Template::Blank => {}
            Template::Kanban => {
                c.blurb = "To do · Doing · Done".into();
                for (i, (t, a)) in [("To do", Accent::Slate), ("Doing", Accent::Amber), ("Done", Accent::Moss)].iter().enumerate() {
                    c.frames.push(frame(id_(), i as f32 * 320., 0., 296., 560., t, *a));
                }
                c.stickers.push(note(id_(), 24., 64., 248., Paper::Sky, "First task", "Drag me to Doing when you start."));
            }
            Template::Week => {
                c.blurb = "This week, day by day".into();
                for (i, d) in ["Monday", "Tuesday", "Wednesday", "Thursday", "Friday", "Saturday", "Sunday"].iter().enumerate() {
                    let a = if i >= 5 { Accent::Rose } else { Accent::Slate };
                    c.frames.push(frame(id_(), (i % 4) as f32 * 280., (i / 4) as f32 * 360., 260., 340., d, a));
                }
            }
            Template::Brainstorm => {
                c.blurb = "One idea, many directions".into();
                let centre = id_();
                c.stickers.push(note(centre, 300., 220., 240., Paper::Blush, "The big question", "What are we trying to figure out?"));
                let spots = [(0., 0.), (600., 0.), (0., 440.), (600., 440.)];
                for (i, (x, y)) in spots.iter().enumerate() {
                    let nid = id_();
                    c.stickers.push(note(nid, *x, *y, 220., [Paper::Butter, Paper::Sage, Paper::Sky, Paper::Lilac][i], "", "An idea…"));
                    let lid = id_();
                    c.links.push(Link { id: lid, from: centre, to: nid, ink: Ink::Forest });
                }
            }
            Template::Meeting => {
                c.blurb = "Agenda, notes, decisions, actions".into();
                c.stickers.push(note(id_(), 0., 0., 260., Paper::Sky, "Agenda", "1. \n2. \n3. "));
                c.stickers.push(note(id_(), 290., 0., 300., Paper::Plain, "Notes", ""));
                c.stickers.push(note(id_(), 0., 200., 260., Paper::Sage, "Decisions", "- "));
                c.stickers.push(note(id_(), 290., 200., 300., Paper::Butter, "Actions", "- [ ] \n- [ ] "));
            }
            Template::Reading => {
                c.blurb = "Books and long reads".into();
                c.stickers.push(note(id_(), 0., 0., 320., Paper::Plain, "Up next", "| Title | Author |\n|---|---|\n|  |  |"));
                c.stickers.push(note(id_(), 350., 0., 240., Paper::Lilac, "Reading now", ""));
                c.stickers.push(note(id_(), 350., 160., 240., Paper::Sage, "Finished", "- [x] "));
            }
        }
        id
    }
}

#[cfg(test)]
mod tests {

    #[test]
    fn text_written_before_sizes_keeps_its_look() {
        let old: Label = serde_json::from_str(r#"{"id":4,"at":{"x":1.0,"y":2.0},"text":"ask Sam","ink":"Graphite"}"#).unwrap();
        assert!(old.italic, "it was slanted");
        assert_eq!(old.size, LabelSize::S);
        assert_eq!(old.font, NoteFont::Sans);
        assert_eq!(old.width, None);
        let new = Label::new(5, v2(0., 0.), Ink::Graphite);
        assert!(!new.italic);
        assert_eq!(new.size, LabelSize::M);
    }

    #[test]
    fn long_text_wraps_in_its_box() {
        let mut l = Label::new(1, v2(10., 20.), Ink::Graphite);
        l.text = "short".into();
        let short = label_rect(&l);
        assert_eq!((short.x, short.y), (10., 20.));
        assert_eq!(short.h, LabelSize::M.line());
        l.text = "word ".repeat(200);
        let long = label_rect(&l);
        assert!(long.w <= LABEL_WRAP + 0.01, "no wider than the wrap");
        assert!(long.h > 5. * LabelSize::M.line(), "it wraps onto many lines");
        // A set width wraps sooner, and is the box's width.
        l.width = Some(200.);
        let narrow = label_rect(&l);
        assert_eq!(narrow.w, 200.);
        assert!(narrow.h > long.h);
        // Bigger text, bigger box; Markdown's marks take no room.
        l.width = None;
        l.text = "**important**".into();
        let marked = label_rect(&l);
        l.text = "important".into();
        assert_eq!(marked, label_rect(&l));
        l.size = LabelSize::XL;
        assert!(label_rect(&l).h > marked.h && label_rect(&l).w > marked.w);
    }
    use super::*;

    fn sticker(id: Id, x: f32, y: f32) -> Sticker {
        Sticker {
            id,
            rect: Rect { x, y, w: 200., h: 150. },
            paper: Paper::Butter,
            title: String::new(),
            body: String::new(),
            image: None,
            text_size: TextSize::Medium,
            edited: 0,
            ..Default::default()
        }
    }

    #[test]
    fn camera_round_trips_and_zooms_about_the_cursor() {
        let mut c = Camera { pan: v2(30., -12.), zoom: 1.5 };
        let p = v2(123., 45.);
        let back = c.to_world(c.to_screen(p));
        assert!((back.x - p.x).abs() < 1e-3 && (back.y - p.y).abs() < 1e-3);
        let anchor = v2(400., 300.);
        let under = c.to_world(anchor);
        c.zoom_at(anchor, 1.7);
        let still = c.to_world(anchor);
        assert!((under.x - still.x).abs() < 1e-3 && (under.y - still.y).abs() < 1e-3);
        c.zoom_at(anchor, 100.);
        assert_eq!(c.zoom, MAX_ZOOM);
    }

    #[test]
    fn front_most_sticker_wins_and_bring_to_front_reorders() {
        let mut c = Category::new(1, "t", Accent::Amber);
        c.stickers.push(sticker(2, 0., 0.));
        c.stickers.push(sticker(3, 100., 50.));
        assert_eq!(c.sticker_at(v2(150., 100.)), Some(3));
        c.bring_to_front(2);
        assert_eq!(c.sticker_at(v2(150., 100.)), Some(2));
        assert_eq!(c.sticker_at(v2(999., 999.)), None);
    }

    #[test]
    fn links_toggle_and_go_with_their_stickers() {
        let mut c = Category::new(1, "t", Accent::Amber);
        c.stickers.push(sticker(2, 0., 0.));
        c.stickers.push(sticker(3, 400., 0.));
        assert!(c.toggle_link(10, 2, 3, Ink::Forest));
        assert!(!c.toggle_link(11, 3, 2, Ink::Forest));
        assert!(c.links.is_empty());
        assert!(!c.toggle_link(12, 2, 2, Ink::Forest));
        c.toggle_link(13, 2, 3, Ink::Forest);
        c.remove_sticker(3);
        assert!(c.links.is_empty());
    }

    #[test]
    fn link_curves_leave_from_facing_edges() {
        let a = Rect { x: 0., y: 0., w: 100., h: 100. };
        let b = Rect { x: 300., y: 20., w: 100., h: 100. };
        let [p0, _, _, p3] = link_curve(&a, &b);
        assert_eq!(p0, v2(100., 50.));
        assert_eq!(p3, v2(300., 70.));
        let below = Rect { x: 10., y: 400., w: 100., h: 100. };
        let [q0, _, _, q3] = link_curve(&a, &below);
        assert_eq!(q0, v2(50., 100.));
        assert_eq!(q3, v2(60., 400.));
    }

    #[test]
    fn simplify_keeps_corners_and_drops_collinear_points() {
        let line: Vec<Vec2> = (0..50).map(|i| v2(i as f32, 0.)).collect();
        assert_eq!(simplify(&line, 0.5), vec![v2(0., 0.), v2(49., 0.)]);
        let corner = vec![v2(0., 0.), v2(5., 0.1), v2(10., 0.), v2(10., 5.), v2(10., 10.)];
        assert_eq!(simplify(&corner, 0.5), vec![v2(0., 0.), v2(10., 0.), v2(10., 10.)]);
    }

    #[test]
    fn eraser_removes_only_strokes_it_touches() {
        let mut c = Category::new(1, "t", Accent::Amber);
        c.strokes.push(Stroke { id: 2, ink: Ink::Graphite, width: 2., marker: false, points: vec![v2(0., 0.), v2(100., 0.)] });
        c.strokes.push(Stroke { id: 3, ink: Ink::Graphite, width: 2., marker: false, points: vec![v2(0., 50.), v2(100., 50.)] });
        assert!(c.erase_at(v2(50., 4.), 4.));
        assert_eq!(c.strokes.len(), 1);
        assert!(!c.erase_at(v2(50., 30.), 4.));
    }

    #[test]
    fn new_notes_find_room_beside_the_others() {
        let view = Rect { x: -1000., y: -1000., w: 2000., h: 2000. };
        let want = Rect { x: 0., y: 0., w: 240., h: 150. };
        let mut taken = Vec::new();
        for _ in 0..12 {
            let r = open_spot(&taken, want, view);
            assert!(taken.iter().all(|t: &Rect| !t.intersects(&r)), "{r:?} lands on another note");
            taken.push(r);
        }
        // The first where asked, the second to its right, the third below.
        assert_eq!((taken[0].x, taken[0].y), (0., 0.));
        assert_eq!((taken[1].x, taken[1].y), (264., 0.));
        assert_eq!((taken[2].x, taken[2].y), (0., 174.));
        // With no room in view, the nearest free place outside it.
        let small = Rect { x: 0., y: 0., w: 260., h: 160. };
        let r = open_spot(&[want], want, small);
        assert!(!r.intersects(&want));
    }

    #[test]
    fn normalize_repairs_ids_sizes_and_dangling_links() {
        let mut b = Board::empty();
        b.next_id = 0;
        let c = b.sheet_mut();
        let mut s = sticker(40, 0., 0.);
        s.rect.w = 10.;
        c.stickers.push(s);
        c.links.push(Link { id: 41, from: 40, to: 99, ink: Ink::Forest });
        c.camera.zoom = 50.;
        b.active = 9;
        b.normalize();
        assert_eq!(b.next_id, 41);
        assert_eq!(b.active, 0);
        let c = b.sheet();
        assert!(c.links.is_empty());
        assert_eq!(c.stickers[0].rect.w, MIN_STICKER.0);
        assert_eq!(c.camera.zoom, MAX_ZOOM);
    }

    #[test]
    fn last_sheet_cannot_be_removed() {
        let mut b = Board::empty();
        assert!(!b.bin_sheet(0, false, 1));
        b.add_sheet("two");
        assert_eq!(b.active, 1);
        assert!(b.bin_sheet(1, false, 1));
        assert_eq!(b.active, 0);
    }

    #[test]
    fn fit_frames_content() {
        let mut c = Camera::default();
        let r = Rect { x: 1000., y: 1000., w: 400., h: 200. };
        c.fit(r, 800., 600., 40.);
        let tl = c.to_screen(v2(r.x, r.y));
        let br = c.to_screen(v2(r.x + r.w, r.y + r.h));
        assert!(tl.x >= 39. && br.x <= 761. && tl.y >= 0. && br.y <= 600.);
    }

    #[test]
    fn tags_are_words_after_a_hash() {
        assert_eq!(tags("Fix #bug and #UI-polish, not # heading or #123 or a#b"), vec!["bug", "ui-polish"]);
        let mut s = sticker(1, 0., 0.);
        s.title = "Plan #work".into();
        s.body = "#idea and #work".into();
        assert_eq!(s.tags(), vec!["work", "idea"]);
        assert!(s.matches("#ide"));
        assert!(!s.matches("#home"));
        assert!(s.matches("PLAN"));
    }

    #[test]
    fn tasks_toggle_in_place() {
        let body = "Intro\n- [ ] one\n- [x] two\n* [ ] three";
        assert_eq!(task_progress(body), (1, 3));
        let t = toggle_task(body, 0);
        assert_eq!(t, "Intro\n- [x] one\n- [x] two\n* [ ] three");
        assert_eq!(toggle_task(&t, 1), "Intro\n- [x] one\n- [ ] two\n* [ ] three");
        assert_eq!(task_line("- [ ]"), Some((false, "")));
        assert_eq!(task_line("- plain"), None);
    }

    #[test]
    fn notes_resize_from_any_edge_or_corner() {
        let r = Rect { x: 100., y: 100., w: 200., h: 100. };
        let g = |x, y| r.grip_at(v2(x, y), 10., 8., 18.);
        let (l, rt, t, b) = (Grip { left: true, ..Default::default() }, Grip { right: true, ..Default::default() }, Grip { top: true, ..Default::default() }, Grip { bottom: true, ..Default::default() });
        assert_eq!(g(100., 100.), Some(Grip { left: true, top: true, ..Default::default() }));
        assert_eq!(g(305., 150.), Some(rt), "just outside the edge");
        assert_eq!(g(292., 150.), Some(rt), "just inside it");
        assert_eq!(g(200., 96.), Some(t));
        assert_eq!(g(98., 203.), Some(Grip { left: true, bottom: true, ..Default::default() }));
        // Along an edge, near the corner is the corner.
        assert_eq!(g(104., 115.), Some(Grip { left: true, top: true, ..Default::default() }));
        assert_eq!(g(285., 196.), Some(Grip { right: true, bottom: true, ..Default::default() }));
        assert_eq!(g(104., 150.), Some(l));
        assert_eq!(g(200., 197.), Some(b));
        assert_eq!(g(200., 150.), None, "the middle moves, it doesn't resize");
        assert_eq!(g(285., 150.), None);
        assert_eq!(g(310., 150.), None, "too far out");

        let left_top = Grip { left: true, top: true, ..Default::default() };
        let out = r.resized(left_top, -20., 30., 50., 40.);
        assert_eq!(out, Rect { x: 80., y: 130., w: 220., h: 70. }, "right and bottom stay put");
        let tiny = r.resized(left_top, 500., 500., 50., 40.);
        assert_eq!((tiny.x + tiny.w, tiny.w, tiny.y + tiny.h, tiny.h), (300., 50., 200., 40.));
        let right = r.resized(Grip { right: true, ..Default::default() }, 10., 99., 50., 40.);
        assert_eq!(right, Rect { w: 210., ..r });
    }

    #[test]
    fn quick_notes_land_in_a_new_inbox() {
        let mut b = Board::empty();
        let first = b.categories[0].id;
        let id = b.quick_note("Call Sam\nabout the lease", 100);
        assert!(b.categories[0].inbox);
        assert_eq!(b.categories[1].id, first);
        let n = b.find_note(id).unwrap();
        assert_eq!((n.title.as_str(), n.body.as_str()), ("Call Sam", "about the lease"));
        let id2 = b.quick_note("milk", 101);
        assert_eq!(b.categories.iter().filter(|c| c.inbox).count(), 1);
        let a = b.find_note(id).unwrap().rect;
        let c = b.find_note(id2).unwrap().rect;
        assert!(!a.intersects(&c), "quick notes don't pile up");
        // Onto a sheet of your choosing; one that's gone means the Inbox.
        let other = b.categories[1].id;
        let id3 = b.quick_note_to("for the other sheet", Some(other), 102);
        assert_eq!(b.sheet_of(id3), Some(1));
        let id4 = b.quick_note_to("nowhere", Some(9_999), 103);
        assert!(b.categories[b.sheet_of(id4).unwrap()].inbox);
    }

    #[test]
    fn trash_restores_and_purges() {
        let mut b = Board::empty();
        b.add_sheet("two");
        b.sheet_mut().stickers.push(sticker(500, 0., 0.));
        b.normalize();
        assert!(b.bin_note(500, false, 1000));
        assert!(b.find_note(500).is_none());
        let bin_id = b.bin[0].id;
        assert_eq!(b.restore(bin_id), Some(1));
        assert!(b.find_note(500).is_some());
        assert!(b.bin_sheet(1, false, 1000));
        assert!(!b.bin_sheet(0, false, 1000), "the last sheet stays");
        b.bin_note(500, true, 1000);
        b.purge(1000 + TRASH_KEEPS + 1);
        assert_eq!(b.bin.len(), 0, "the sheet went with its note; nothing archived left");
    }

    #[test]
    fn notes_move_between_sheets_and_lose_links() {
        let mut b = Board::empty();
        b.sheet_mut().stickers.push(sticker(10, 0., 0.));
        b.sheet_mut().stickers.push(sticker(11, 300., 0.));
        b.sheet_mut().links.push(Link { id: 12, from: 10, to: 11, ink: Ink::Forest });
        b.add_sheet("other");
        b.normalize();
        assert!(b.move_note(10, 1));
        assert_eq!(b.sheet_of(10), Some(1));
        assert!(b.categories[0].links.is_empty());
        assert!(!b.move_note(10, 1));
    }

    #[test]
    fn search_finds_notes_on_every_sheet() {
        let mut b = Board::empty();
        let mut s = sticker(10, 0., 0.);
        s.body = "Buy #garden seeds".into();
        b.sheet_mut().stickers.push(s);
        b.add_sheet("x");
        let mut s = sticker(11, 0., 0.);
        s.title = "Seed catalogue".into();
        b.sheet_mut().stickers.push(s);
        let hits = b.search("seed");
        assert_eq!(hits.len(), 2);
        assert_eq!(b.search("#garden"), vec![Hit { sheet: 0, note: 10 }]);
        assert_eq!(b.all_tags(), vec![("garden".to_string(), 1)]);
    }

    #[test]
    fn reminders_fire_once() {
        let mut b = Board::empty();
        let mut s = sticker(10, 0., 0.);
        s.due = Some(50);
        b.sheet_mut().stickers.push(s);
        assert!(b.take_due(40).is_empty());
        assert_eq!(b.take_due(60), vec![10]);
        assert!(b.take_due(70).is_empty());
        assert_eq!(b.reminders(), vec![(0, 10, 50)]);
    }

    #[test]
    fn tidy_lays_out_rows_without_overlap() {
        let mut c = Category::new(1, "t", Accent::Amber);
        for i in 0..5 {
            c.stickers.push(sticker(10 + i, (i * 37 % 5) as f32 * 90., (i * 53 % 7) as f32 * 40.));
        }
        let ids: Vec<Id> = c.stickers.iter().map(|s| s.id).collect();
        c.tidy(&ids, &|s| s.rect.h);
        for a in &c.stickers {
            for b in &c.stickers {
                assert!(a.id == b.id || !a.rect.intersects(&b.rect));
            }
        }
    }

    #[test]
    fn templates_fill_a_new_sheet() {
        let mut b = Board::empty();
        b.add_from_template("Board", Template::Kanban, Accent::Slate, 5);
        assert_eq!(b.sheet().frames.len(), 3);
        assert_eq!(b.sheet().name, "Board");
        b.add_from_template("Ideas", Template::Brainstorm, Accent::Plum, 5);
        assert_eq!(b.sheet().links.len(), 4);
        b.normalize();
        let mut ids: Vec<Id> = b.categories.iter().flat_map(|c| c.stickers.iter().map(|s| s.id)).collect();
        let n = ids.len();
        ids.dedup();
        assert_eq!(ids.len(), n);
    }

    #[test]
    fn frames_carry_the_notes_inside() {
        let mut c = Category::new(1, "t", Accent::Amber);
        c.frames.push(Frame { id: 2, rect: Rect { x: 0., y: 0., w: 400., h: 400. }, title: "A".into(), accent: Accent::Amber });
        c.stickers.push(sticker(3, 10., 10.));
        c.stickers.push(sticker(4, 500., 10.));
        assert_eq!(c.frame_members(2), vec![3]);
        assert_eq!(c.neighbour(3, 1., 0.), Some(4));
        assert_eq!(c.neighbour(3, -1., 0.), None);
    }

    #[test]
    fn adopted_sheets_get_fresh_ids_and_keep_links() {
        let mut b = Board::empty();
        b.sheet_mut().stickers.push(sticker(10, 0., 0.));
        b.sheet_mut().stickers.push(sticker(11, 300., 0.));
        b.sheet_mut().links.push(Link { id: 12, from: 10, to: 11, ink: Ink::Forest });
        b.normalize();
        let copy = b.sheet().clone();
        let ix = b.adopt_sheet(copy, "Copy".into());
        let c = &b.categories[ix];
        assert!(c.stickers.iter().all(|s| s.id > 12));
        assert_eq!(c.links.len(), 1);
        assert!(c.sticker(c.links[0].from).is_some());
    }

    #[test]
    fn paper_search_and_snooze() {
        let mut s = sticker(1, 0., 0.);
        s.paper = Paper::Sage;
        assert!(s.matches("paper:sage"));
        assert!(!s.matches("paper:sky"));
        s.edited = 1;
        assert!(s.is_stale(STALE_AFTER + 10));
        s.snoozed = STALE_AFTER + 100;
        assert!(!s.is_stale(STALE_AFTER + 10));
    }

    #[test]
    fn notes_saved_before_types_are_cards() {
        let json = r#"{"id":4,"rect":{"x":0,"y":0,"w":200,"h":120},"paper":"Sage","title":"t","body":""}"#;
        let s: Sticker = serde_json::from_str(json).unwrap();
        assert_eq!(s.style, NoteStyle::Card);
        let back: Sticker = serde_json::from_str(&serde_json::to_string(&Sticker { style: NoteStyle::Index, ..s }).unwrap()).unwrap();
        assert_eq!(back.style, NoteStyle::Index);
        assert_eq!(NoteStyle::Index.next(), NoteStyle::Card);
    }

    #[test]
    fn new_notes_follow_the_last_one() {
        let mut c = Category::new(1, "t", Accent::Amber);
        assert_eq!(c.next_paper(Paper::Sky), Paper::Sky);
        assert_eq!(c.next_look(NoteStyle::Taped, NoteFont::Mono), (NoteStyle::Taped, NoteFont::Mono));
        let mut a = sticker(2, 0., 0.);
        a.style = NoteStyle::Cheat;
        a.font = NoteFont::Hand;
        a.edited = 50;
        let mut b = sticker(3, 300., 0.);
        b.style = NoteStyle::Index;
        b.paper = Paper::Plain;
        b.edited = 10;
        c.stickers.push(a);
        c.stickers.push(b);
        // The paper follows the newest note, skipping Plain.
        assert_eq!(c.next_paper(Paper::Sky), Paper::Peach);
        // The type and font follow the note changed last.
        assert_eq!(c.next_look(NoteStyle::Card, NoteFont::Sans), (NoteStyle::Cheat, NoteFont::Hand));
        // A picture doesn't count.
        let mut pic = sticker(4, 600., 0.);
        pic.image = Some("p.png".into());
        pic.edited = 99;
        c.stickers.push(pic);
        assert_eq!(c.next_look(NoteStyle::Card, NoteFont::Sans).0, NoteStyle::Cheat);
    }

    #[test]
    fn sheets_saved_before_backdrops_are_dotted_paper() {
        let mut c = Category::new(1, "t", Accent::Amber);
        c.backdrop = Backdrop { pattern: Pattern::Graph, tint: Tint::Mint, image: Some("9.svg".into()), soften: true };
        let mut v: serde_json::Value = serde_json::to_value(&c).unwrap();
        assert_eq!(serde_json::from_value::<Category>(v.clone()).unwrap().backdrop, c.backdrop);
        v.as_object_mut().unwrap().remove("backdrop");
        assert_eq!(serde_json::from_value::<Category>(v).unwrap().backdrop, Backdrop::default());
        assert_eq!(Backdrop::default().pattern, Pattern::Dots);
    }

    #[test]
    fn pattern_rules_stay_put_and_thin_out_zoomed_out() {
        assert_eq!(pattern_step(22., 1., 14.), 22.);
        assert_eq!(pattern_step(22., 0.25, 14.), 22.);
        assert_eq!(pattern_step(22., 0.5, 14.), 22.);
        let cam = Camera { pan: v2(30., -5.), zoom: 1. };
        let r = rules(cam, 100., 50., 20., true, true, Some(5));
        let across: Vec<f32> = r.iter().filter(|x| x.across).map(|x| x.at).collect();
        let down: Vec<f32> = r.iter().filter(|x| !x.across).map(|x| x.at).collect();
        assert_eq!(across, [15., 35.]);
        assert_eq!(down, [10., 30., 50., 70., 90.]);
        // The heavier line is the world's x = 0, wherever the view is.
        assert!(r.iter().any(|x| !x.across && x.at == 30. && x.major));
        assert_eq!(r.iter().filter(|x| x.major).count(), 1);
    }

    #[test]
    fn pins_keep_their_size_and_colour_across_shapes() {
        let red_push = Pin { shape: PinShape::Push, size: PinSize::Large, tone: PinTone::Red };
        let blue = Pin { tone: PinTone::Blue, ..red_push };
        // A shape that takes a colour keeps the one picked.
        assert_eq!(blue.with_shape(PinShape::Heart, true), Pin { shape: PinShape::Heart, size: PinSize::Large, tone: PinTone::Blue });
        // A first pin takes the shape's own colour.
        assert_eq!(Pin::default().with_shape(PinShape::Star, false).tone, PinTone::Yellow);
        // From a fixed-colour shape, the next one takes its own colour too.
        let chip = blue.with_shape(PinShape::Chip, true);
        assert_eq!(chip.with_shape(PinShape::Heart, true).tone, PinTone::Pink);
        let all: Vec<PinShape> = PinShape::FAMILIES.iter().flat_map(|f| f.1.iter().copied()).collect();
        assert_eq!(all.len(), 18);
        let s: Sticker = serde_json::from_str(r#"{"id":1,"rect":{"x":0,"y":0,"w":1,"h":1},"paper":"Sage","title":"","body":""}"#).unwrap();
        assert_eq!(s.pin, None);
    }

    #[test]
    fn index_cards_come_with_a_pin_that_can_go() {
        let mut s = Sticker { id: 6, ..Default::default() };
        s.set_style(NoteStyle::Index);
        assert_eq!(s.pin, Some(Pin::index_card(6)));
        s.pin = None;
        s.set_style(NoteStyle::Taped);
        s.set_style(NoteStyle::Index);
        assert!(s.pin.is_some(), "made an index card again: a pin again");
        let mut star = Sticker { id: 7, pin: Some(Pin { shape: PinShape::Star, ..Pin::default() }), ..Default::default() };
        star.set_style(NoteStyle::Index);
        assert_eq!(star.pin.unwrap().shape, PinShape::Star, "a chosen pin stays");
    }
}
