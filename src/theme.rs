//! Design tokens: colour, type, space, radius and elevation, in a light and
//! a dark set. Everything the views draw comes from here.

use gpui::{hsla, point, px, rgb, rgba, App, BoxShadow, Global, Hsla, Pixels};

use crate::model::{Accent, Ink, NoteFont, Paper, PinTone, Tint};

#[derive(Clone, Copy)]
pub struct Colors {
    /// The sheet behind the stickers.
    pub canvas: Hsla,
    pub canvas_dot: Hsla,
    /// A ruled pattern's lines: finer than the dots; the heavier lines of
    /// graph paper take `canvas_dot`.
    pub canvas_line: Hsla,
    /// Sidebar, top bar, menus, dialogs.
    pub surface: Hsla,
    pub surface_sunk: Hsla,
    pub border: Hsla,
    pub hover: Hsla,
    pub text: Hsla,
    pub text_body: Hsla,
    pub text_muted: Hsla,
    pub text_subtle: Hsla,
    pub primary: Hsla,
    pub primary_hover: Hsla,
    pub primary_fg: Hsla,
    /// The active row and tool: a wash of the primary.
    pub primary_tint: Hsla,
    pub selection: Hsla,
    pub scrim: Hsla,
    pub toast: Hsla,
    pub toast_fg: Hsla,
    pub danger: Hsla,
    /// Folio's pink: the main action, the active sheet, focus.
    pub brand: Hsla,
    pub brand_tint: Hsla,
    pub brand_on_tint: Hsla,
    /// Overdue reminders.
    pub alert: Hsla,
    /// All's well: synced, done.
    pub ok: Hsla,
    /// The window's close button under the pointer (as Windows draws it).
    pub window_close: Hsla,
    /// Note types: clear tape, the Remember box's wash, table rules, an
    /// index card's blue lines, pink margin and header rule, and the
    /// shade where a taped note's foot curls.
    pub tape: Hsla,
    pub tape_edge: Hsla,
    pub wash: Hsla,
    pub rule: Hsla,
    pub ruled: Hsla,
    pub margin: Hsla,
    pub head_rule: Hsla,
    pub curl: Hsla,
    /// Google's four colours, for its "G" on the sign-in button (blue,
    /// green, yellow, red), as its brand rules ask: never recoloured.
    pub google: [Hsla; 4],
}

#[derive(Clone, Copy)]
pub struct TypeScale {
    pub micro: Pixels,   // 11: meta lines, counts
    pub caption: Pixels, // 12
    pub body: Pixels,    // 13
    pub title: Pixels,   // 14: sticker titles
    pub brand: Pixels,   // 17
    pub heading: Pixels, // 26: the sheet's name
}

#[derive(Clone, Copy)]
pub struct Space {
    pub xxs: Pixels, // 2
    pub xs: Pixels,  // 4
    pub sm: Pixels,  // 8
    pub md: Pixels,  // 12
    pub lg: Pixels,  // 16
    pub xl: Pixels,  // 24
    pub xxl: Pixels, // 32
}

#[derive(Clone, Copy)]
pub struct Radius {
    pub small: Pixels,  // 6
    pub button: Pixels, // 8
    pub card: Pixels,   // 12: stickers
    pub dialog: Pixels, // 16
    pub full: Pixels,
}

#[derive(Clone, Copy)]
pub struct Sizes {
    pub sidebar: Pixels,   // 236
    pub topbar: Pixels,    // 40: the top bar, which is the title bar
    pub bar_control: Pixels, // 28: buttons and the search field in it
    pub search: Pixels,    // 260: the title bar's search field, closed
    pub search_open: Pixels, // 420: and open, while you type
    pub control: Pixels,   // 32: buttons and inputs
    pub icon_sm: Pixels,   // 14
    pub icon: Pixels,      // 16
    pub icon_lg: Pixels,   // 20
    pub dot: Pixels,       // 8: category dots
    pub swatch: Pixels,    // 22
    pub logo: Pixels,      // 30
    pub dialog_w: Pixels,  // 460
    pub drawer: Pixels,    // 304: the sheets drawer
    pub card_min: Pixels,  // 270: a home card's least width
    pub grip: Pixels,      // 14: the resize corner
    pub dot_grid: f32,     // 22: world units between canvas dots
}

/// How things arrive and leave ("Glide"): they fade up a few pixels and
/// settle with an ease-out, and leave the same way, faster, with an
/// ease-in. Times in milliseconds, distances in px.
#[derive(Clone, Copy)]
pub struct Motion {
    /// A menu, popover or the edit bar arriving.
    pub pop_in: u64,
    /// And leaving.
    pub pop_out: u64,
    /// Between one row and the next as a list opens.
    pub stagger: u64,
    /// A dialog arriving and leaving.
    pub dialog_in: u64,
    pub dialog_out: u64,
    /// The search opening (its field widens, its results rise).
    pub search_in: u64,
    pub search_out: u64,
    /// The sheets drawer sliding in and out.
    pub drawer_in: u64,
    pub drawer_out: u64,
    /// A page (home, a sheet, settings...) arriving.
    pub page_in: u64,
    /// A toast arriving and leaving.
    pub toast_in: u64,
    pub toast_out: u64,
    /// A note arriving on the sheet, and going to the Trash.
    pub note_in: u64,
    pub note_out: u64,
    /// A pin landing on a note, and after a new note, how long it waits.
    pub pin_in: u64,
    pub pin_wait: u64,
    /// A window of its own (a note on the desktop, the quick note).
    pub window_in: u64,
    pub window_out: u64,
    /// A tooltip.
    pub tip_in: u64,
    /// Light and dark: the old one fades off the window.
    pub theme_fade: u64,
    /// How far things rise as they arrive.
    pub rise: f32,
    pub rise_dialog: f32,
    pub rise_window: f32,
    /// How far a toast slides in from the right.
    pub slide_toast: f32,
    /// How far things sink as they leave.
    pub sink: f32,
}

const MOTION: Motion = Motion {
    pop_in: 180,
    pop_out: 120,
    stagger: 22,
    dialog_in: 240,
    dialog_out: 140,
    search_in: 220,
    search_out: 130,
    drawer_in: 260,
    drawer_out: 180,
    page_in: 280,
    toast_in: 220,
    toast_out: 150,
    note_in: 220,
    note_out: 160,
    pin_in: 180,
    pin_wait: 160,
    window_in: 260,
    window_out: 140,
    tip_in: 120,
    theme_fade: 220,
    rise: 8.,
    rise_dialog: 14.,
    rise_window: 10.,
    slide_toast: 18.,
    sink: 4.,
};

#[derive(Clone)]
pub struct Theme {
    pub is_dark: bool,
    pub colors: Colors,
    pub text: TypeScale,
    pub space: Space,
    pub radius: Radius,
    pub size: Sizes,
    pub motion: Motion,
    pub font: &'static str,
    pub high_contrast: bool,
    /// The interface size it was made at.
    pub scale: f32,
}

fn c(hex: u32) -> Hsla {
    rgb(hex).into()
}
fn ca(hex_rgba: u32) -> Hsla {
    rgba(hex_rgba).into()
}

const TEXT: TypeScale = TypeScale {
    micro: px(11.),
    caption: px(12.),
    body: px(13.),
    title: px(14.),
    brand: px(17.),
    heading: px(26.),
};
const SPACE: Space = Space {
    xxs: px(2.),
    xs: px(4.),
    sm: px(8.),
    md: px(12.),
    lg: px(16.),
    xl: px(24.),
    xxl: px(32.),
};
const RADIUS: Radius = Radius {
    small: px(6.),
    button: px(8.),
    card: px(12.),
    dialog: px(16.),
    full: px(999.),
};
const SIZE: Sizes = Sizes {
    sidebar: px(236.),
    topbar: px(40.),
    bar_control: px(28.),
    search: px(260.),
    search_open: px(420.),
    control: px(32.),
    icon_sm: px(14.),
    icon: px(16.),
    icon_lg: px(20.),
    dot: px(8.),
    swatch: px(22.),
    logo: px(30.),
    dialog_w: px(460.),
    drawer: px(304.),
    card_min: px(270.),
    grip: px(14.),
    dot_grid: 22.,
};

impl Theme {
    pub fn light() -> Self {
        Theme {
            is_dark: false,
            colors: Colors {
                canvas: c(0xf6f7f3),
                canvas_dot: c(0xd6dcd4),
                canvas_line: c(0xe3e7e0),
                surface: c(0xffffff),
                surface_sunk: c(0xfafbf8),
                border: c(0xe7eae5),
                hover: c(0xf2f5f1),
                text: c(0x18211f),
                text_body: c(0x45524a),
                text_muted: c(0x6b7871),
                text_subtle: c(0x939e97),
                primary: c(0xf30d6a),
                primary_hover: c(0xd9085c),
                primary_fg: c(0xffffff),
                primary_tint: c(0xfee6ef),
                selection: c(0xf30d6a),
                scrim: ca(0x18211f55),
                toast: c(0x22392f),
                toast_fg: c(0xffffff),
                danger: c(0xb5473a),
                brand: c(0xf30d6a),
                brand_tint: c(0xfee6ef),
                brand_on_tint: c(0xb5084f),
                alert: c(0xd03b3b),
                ok: c(0x2f8f5b),
                window_close: c(0xc42b1c),
                tape: ca(0xffffffb8),
                tape_edge: ca(0x18251f17),
                wash: ca(0xffffff9e),
                rule: ca(0x18251f38),
                ruled: ca(0x3f68b033),
                margin: ca(0xf30d6a5c),
                head_rule: ca(0xf30d6a94),
                curl: ca(0x18251f0d),
                google: [c(0x4285f4), c(0x34a853), c(0xfbbc05), c(0xea4335)],
            },
            text: TEXT,
            space: SPACE,
            radius: RADIUS,
            size: SIZE,
            motion: MOTION,
            font: crate::fonts::sans(),
            high_contrast: false,
            scale: 1.0,
        }
    }

    pub fn dark() -> Self {
        Theme {
            is_dark: true,
            colors: Colors {
                canvas: c(0x121614),
                canvas_dot: c(0x2a312d),
                canvas_line: c(0x1e2421),
                surface: c(0x191e1c),
                surface_sunk: c(0x151a18),
                border: c(0x29302c),
                hover: c(0x222a26),
                text: c(0xe9ede9),
                text_body: c(0xc3ccc6),
                text_muted: c(0x95a29b),
                text_subtle: c(0x6f7c75),
                primary: c(0xf30d6a),
                primary_hover: c(0xff3d86),
                primary_fg: c(0xffffff),
                primary_tint: c(0x3d1424),
                selection: c(0xff3d86),
                scrim: ca(0x00000088),
                toast: c(0xe6efe9),
                toast_fg: c(0x16201b),
                danger: c(0xe07a6c),
                brand: c(0xf30d6a),
                brand_tint: c(0x3d1424),
                brand_on_tint: c(0xff8cb6),
                alert: c(0xff7b6e),
                ok: c(0x5cc98a),
                window_close: c(0xc42b1c),
                tape: ca(0xffffff29),
                tape_edge: ca(0xffffff1a),
                wash: ca(0xffffff12),
                rule: ca(0xffffff33),
                ruled: ca(0x82a6e626),
                margin: ca(0xff3d8657),
                head_rule: ca(0xff3d8699),
                curl: ca(0x00000029),
                google: [c(0x4285f4), c(0x34a853), c(0xfbbc05), c(0xea4335)],
            },
            text: TEXT,
            space: SPACE,
            radius: RADIUS,
            size: SIZE,
            motion: MOTION,
            font: crate::fonts::sans(),
            high_contrast: false,
            scale: 1.0,
        }
    }

    pub fn paper(&self, p: Paper) -> Hsla {
        let (light, dark) = match p {
            Paper::Butter => (0xfff3d4, 0x3a3323),
            Paper::Sage => (0xe3efe6, 0x22332a),
            Paper::Sky => (0xe6eef8, 0x222d3b),
            Paper::Blush => (0xf8e7e2, 0x3b2a27),
            Paper::Lilac => (0xeee8f7, 0x2e2939),
            Paper::Plain => (0xffffff, 0x232926),
            Paper::Peach => (0xffe7d4, 0x3d2c21),
            Paper::Rose => (0xfbe2eb, 0x3b2530),
            Paper::Ocean => (0xdcf0f1, 0x1f3436),
            Paper::Mint => (0xdff4e8, 0x1f3628),
            Paper::Sand => (0xf2ebdc, 0x35302a),
            Paper::Stone => (0xeceeed, 0x2b2f2e),
        };
        c(if self.is_dark { dark } else { light })
    }

    /// A sheet's paper colour; Paper is the canvas itself.
    pub fn tint(&self, tint: Tint) -> Hsla {
        let (light, dark) = match tint {
            Tint::Paper => return self.colors.canvas,
            Tint::Warm => (0xf7f2e7, 0x17140f),
            Tint::Cool => (0xf0f3f8, 0x11151a),
            Tint::Mint => (0xedf5ef, 0x0f1712),
            Tint::Blush => (0xf8efed, 0x181213),
            Tint::Lavender => (0xf3f0f8, 0x141219),
        };
        c(if self.is_dark { dark } else { light })
    }

    /// A cheat card's title band: a deeper shade of its paper.
    pub fn paper_band(&self, p: Paper) -> Hsla {
        let (light, dark) = match p {
            Paper::Butter => (0xfde7a6, 0x4a412b),
            Paper::Sage => (0xcde4d4, 0x2b4236),
            Paper::Sky => (0xd0e0f5, 0x2b3a4c),
            Paper::Blush => (0xf2d2c9, 0x4b3632),
            Paper::Lilac => (0xded3f2, 0x3b3449),
            Paper::Plain => (0xeef1ec, 0x2d3530),
            Paper::Peach => (0xffd4b3, 0x4e3929),
            Paper::Rose => (0xf6cbdb, 0x4b2f3d),
            Paper::Ocean => (0xc3e5e7, 0x284446),
            Paper::Mint => (0xc7ebd7, 0x284633),
            Paper::Sand => (0xe6d9bf, 0x453e35),
            Paper::Stone => (0xdce0de, 0x373c3a),
        };
        c(if self.is_dark { dark } else { light })
    }

    /// A pin's colour: its body, shade and highlight, filled into the pin
    /// pictures (`assets/pins`). Pins are things, the same in dark mode.
    pub fn pin_tone(&self, tone: PinTone) -> [u32; 3] {
        match tone {
            PinTone::Red => [0xe5484d, 0xa8232a, 0xff9b9e],
            PinTone::Pink => [0xf30d6a, 0xa8074a, 0xff9cc2],
            PinTone::Yellow => [0xf5c22e, 0xc48d10, 0xffe7a0],
            PinTone::Green => [0x3d9a6d, 0x22653f, 0xb9ecd0],
            PinTone::Blue => [0x4b6fd0, 0x2a4596, 0xc9d8ff],
            PinTone::Purple => [0x8b5cf6, 0x5b32b8, 0xd9c9ff],
            PinTone::Black => [0x2f353a, 0x111417, 0x8a949b],
            PinTone::Silver => [0xc9d1d6, 0x8d979d, 0xf6f8f9],
        }
    }

    /// A pin colour's swatch.
    pub fn pin_swatch(&self, tone: PinTone) -> Hsla {
        c(self.pin_tone(tone)[0])
    }

    pub fn ink(&self, i: Ink) -> Hsla {
        let (light, dark) = match i {
            Ink::Graphite => (0x2a332f, 0xdfe5e1),
            Ink::Forest => (0x3d7a5d, 0x7cc19d),
            Ink::Coral => (0xd0674a, 0xf08c70),
            Ink::Cobalt => (0x3f68b0, 0x82a6e6),
            Ink::Marigold => (0xc88d1c, 0xe9b54d),
            Ink::Ruby => (0xc0392b, 0xff7a6b),
            Ink::Pink => (0xd6457f, 0xff8fb8),
            Ink::Violet => (0x7e4bc4, 0xb894f2),
            Ink::Indigo => (0x4b4fc4, 0x9a9df2),
            Ink::Sky => (0x2f8fd0, 0x7cc4f2),
            Ink::Teal => (0x1f8f8a, 0x6fd3cc),
            Ink::Lime => (0x6f9a1f, 0xb5dc5c),
            Ink::Tangerine => (0xe07b1a, 0xffaa5c),
            Ink::Brown => (0x8a5a3c, 0xc9977a),
            Ink::Slate => (0x5f6f7f, 0xa9b6c2),
            Ink::Silver => (0xa3a9a6, 0x6e7471),
            Ink::Midnight => (0x101418, 0xf4f4f2),
        };
        c(if self.is_dark { dark } else { light })
    }

    pub fn accent(&self, a: Accent) -> Hsla {
        c(match a {
            Accent::Amber => 0xd5a86a,
            Accent::Slate => 0x8ba7ca,
            Accent::Rose => 0xc88e85,
            Accent::Moss => 0x8fb08f,
            Accent::Plum => 0xa992c4,
        })
    }

    /// A sticker at rest: a soft, wide lift.
    pub fn shadow_card(&self, zoom: f32) -> Vec<BoxShadow> {
        let a = if self.is_dark { 0.45 } else { 0.08 };
        vec![
            BoxShadow {
                color: hsla(150. / 360., 0.2, 0.12, a),
                offset: point(px(0.), px(10. * zoom)),
                blur_radius: px(28. * zoom),
                spread_radius: px(0.),
            inset: false,
            },
            BoxShadow {
                color: hsla(150. / 360., 0.2, 0.12, a * 0.6),
                offset: point(px(0.), px(1.)),
                blur_radius: px(2.),
                spread_radius: px(0.),
            inset: false,
            },
        ]
    }

    /// A taped note: low and close, as paper stuck flat to the sheet.
    pub fn shadow_taped(&self, zoom: f32) -> Vec<BoxShadow> {
        let a = if self.is_dark { 0.5 } else { 0.14 };
        let tint = |k: f32| hsla(150. / 360., 0.2, if self.is_dark { 0.02 } else { 0.12 }, a * k);
        vec![
            BoxShadow { color: tint(0.45), offset: point(px(0.), px(1.)), blur_radius: px(1.), spread_radius: px(0.), inset: false },
            BoxShadow { color: tint(1.), offset: point(px(0.), px(6. * zoom)), blur_radius: px(12. * zoom), spread_radius: px(-4. * zoom), inset: false },
            BoxShadow { color: tint(1.1), offset: point(px(0.), px(16. * zoom)), blur_radius: px(24. * zoom), spread_radius: px(-14. * zoom), inset: false },
        ]
    }

    /// An index card: crisp card stock, a hair off the sheet.
    pub fn shadow_index(&self, zoom: f32) -> Vec<BoxShadow> {
        let a = if self.is_dark { 0.5 } else { 0.12 };
        let tint = |k: f32| hsla(150. / 360., 0.2, if self.is_dark { 0.02 } else { 0.12 }, a * k);
        vec![
            BoxShadow { color: tint(0.55), offset: point(px(0.), px(1.)), blur_radius: px(1.), spread_radius: px(0.), inset: false },
            BoxShadow { color: tint(0.65), offset: point(px(0.), px(3. * zoom)), blur_radius: px(6. * zoom), spread_radius: px(-2. * zoom), inset: false },
            BoxShadow { color: tint(1.4), offset: point(px(0.), px(14. * zoom)), blur_radius: px(26. * zoom), spread_radius: px(-12. * zoom), inset: false },
        ]
    }

    /// A sticker being dragged: higher, so it reads as picked up.
    pub fn shadow_lifted(&self, zoom: f32) -> Vec<BoxShadow> {
        let a = if self.is_dark { 0.6 } else { 0.16 };
        vec![BoxShadow {
            color: hsla(150. / 360., 0.2, 0.12, a),
            offset: point(px(0.), px(20. * zoom)),
            blur_radius: px(44. * zoom),
            spread_radius: px(0.),
            inset: false,
        }]
    }

    /// Menus, toolbars, popovers and dialogs: a contact shadow tight under
    /// the edge, a key shadow below, and a wide ambient one, so a surface
    /// sits clearly above what's under it.
    pub fn shadow_float(&self) -> Vec<BoxShadow> {
        let a = if self.is_dark { 0.55 } else { 0.10 };
        let tint = |k: f32| hsla(150. / 360., 0.18, 0.08, a * k);
        let s = |y: f32, blur: f32, spread: f32, k: f32| BoxShadow {
            color: tint(k),
            offset: point(px(0.), px(y)),
            blur_radius: px(blur),
            spread_radius: px(spread),
            inset: false,
        };
        vec![s(1., 2., 0., 0.9), s(8., 20., -4., 1.), s(24., 48., -8., 1.1)]
    }

    /// Buttons and small raised things: a hairline of shadow under them.
    pub fn shadow_raised(&self) -> Vec<BoxShadow> {
        let a = if self.is_dark { 0.5 } else { 0.07 };
        vec![
            BoxShadow { color: hsla(150. / 360., 0.18, 0.08, a), offset: point(px(0.), px(1.)), blur_radius: px(2.), spread_radius: px(0.), inset: false },
            BoxShadow { color: hsla(150. / 360., 0.18, 0.08, a * 0.6), offset: point(px(0.), px(2.)), blur_radius: px(6.), spread_radius: px(-2.), inset: false },
        ]
    }

    /// The main action: lifted by a shadow in its own colour, with a light
    /// line along its top like a lit edge.
    pub fn shadow_primary(&self) -> Vec<BoxShadow> {
        let glow = self.colors.primary;
        vec![
            BoxShadow { color: hsla(0., 0., 1., if self.is_dark { 0.14 } else { 0.22 }), offset: point(px(0.), px(1.)), blur_radius: px(0.), spread_radius: px(0.), inset: true },
            BoxShadow { color: Hsla { a: 0.28, ..glow }, offset: point(px(0.), px(1.)), blur_radius: px(2.), spread_radius: px(0.), inset: false },
            BoxShadow { color: Hsla { a: 0.22, ..glow }, offset: point(px(0.), px(4.)), blur_radius: px(10.), spread_radius: px(-3.), inset: false },
        ]
    }
}

pub struct ActiveTheme(pub Theme);
impl Global for ActiveTheme {}

/// Installs the theme: light or dark, optionally high contrast, at an
/// interface size (1.0 = as designed).
pub fn set(dark: bool, high_contrast: bool, scale: f32, cx: &mut App) {
    let mut t = if dark { Theme::dark() } else { Theme::light() };
    if high_contrast {
        t = t.high_contrast();
    }
    cx.set_global(ActiveTheme(t.scaled(scale)));
}

impl Theme {
    /// Stronger text and edges, for low vision or bright rooms.
    pub fn high_contrast(mut self) -> Self {
        let c = &mut self.colors;
        if self.is_dark {
            c.text = rgb(0xffffff).into();
            c.text_body = rgb(0xf2f4f3).into();
            c.text_muted = rgb(0xd5dbd8).into();
            c.text_subtle = rgb(0xb3bcb7).into();
            c.border = rgb(0x8a9590).into();
            c.canvas_dot = rgb(0x46504b).into();
            c.canvas_line = rgb(0x343c38).into();
            c.rule = rgba(0xffffff80).into();
            c.ruled = rgba(0x82a6e666).into();
            c.margin = rgba(0xff3d86b3).into();
        } else {
            c.text = rgb(0x000000).into();
            c.text_body = rgb(0x111815).into();
            c.text_muted = rgb(0x2f3934).into();
            c.text_subtle = rgb(0x4d5852).into();
            c.border = rgb(0x6f7a74).into();
            c.canvas_dot = rgb(0xa9b3ab).into();
            c.canvas_line = rgb(0xc4ccc5).into();
            c.rule = rgba(0x18251f80).into();
            c.ruled = rgba(0x3f68b066).into();
            c.margin = rgba(0xf30d6ab3).into();
        }
        self.high_contrast = true;
        self
    }

    /// Every type size, space, radius and size multiplied by `k`.
    pub fn scaled(mut self, k: f32) -> Self {
        let k = k.clamp(0.8, 1.4);
        self.scale = k;
        if (k - 1.).abs() < 0.001 {
            return self;
        }
        let m = |p: &mut Pixels| *p = px((f32::from(*p) * k).round());
        let t = &mut self.text;
        for p in [&mut t.micro, &mut t.caption, &mut t.body, &mut t.title, &mut t.brand, &mut t.heading] {
            m(p);
        }
        let s = &mut self.space;
        for p in [&mut s.xxs, &mut s.xs, &mut s.sm, &mut s.md, &mut s.lg, &mut s.xl, &mut s.xxl] {
            m(p);
        }
        let r = &mut self.radius;
        for p in [&mut r.small, &mut r.button, &mut r.card, &mut r.dialog] {
            m(p);
        }
        let z = &mut self.size;
        for p in [
            &mut z.sidebar, &mut z.topbar, &mut z.control, &mut z.icon_sm, &mut z.icon, &mut z.icon_lg, &mut z.dot,
            &mut z.swatch, &mut z.logo, &mut z.dialog_w, &mut z.grip, &mut z.drawer, &mut z.card_min,
            &mut z.search, &mut z.search_open,
        ] {
            m(p);
        }
        self
    }

    /// The font family a note is written in.
    pub fn note_font(&self, f: NoteFont) -> &'static str {
        match f {
            NoteFont::Sans => self.font,
            NoteFont::Hand => "Comic Neue",
            NoteFont::Mono => "JetBrains Mono",
        }
    }

}

pub fn theme(cx: &App) -> &Theme {
    &cx.global::<ActiveTheme>().0
}
