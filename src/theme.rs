//! Design tokens: colour, type, space, radius and elevation, in a light and
//! a dark set. Everything the views draw comes from here.

use gpui::{hsla, point, px, rgb, rgba, App, BoxShadow, Global, Hsla, Pixels};

use crate::model::{Accent, Ink, Paper};

#[derive(Clone, Copy)]
pub struct Colors {
    /// The sheet behind the stickers.
    pub canvas: Hsla,
    pub canvas_dot: Hsla,
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
    pub primary_on_tint: Hsla,
    pub selection: Hsla,
    pub scrim: Hsla,
    pub toast: Hsla,
    pub toast_fg: Hsla,
    pub danger: Hsla,
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
    pub topbar: Pixels,    // 60
    pub control: Pixels,   // 32: buttons and inputs
    pub icon_sm: Pixels,   // 14
    pub icon: Pixels,      // 16
    pub icon_lg: Pixels,   // 20
    pub dot: Pixels,       // 8: category dots
    pub swatch: Pixels,    // 22
    pub logo: Pixels,      // 30
    pub dialog_w: Pixels,  // 460
    pub grip: Pixels,      // 14: the resize corner
    pub dot_grid: f32,     // 22: world units between canvas dots
}

#[derive(Clone)]
pub struct Theme {
    pub is_dark: bool,
    pub colors: Colors,
    pub text: TypeScale,
    pub space: Space,
    pub radius: Radius,
    pub size: Sizes,
    pub font: &'static str,
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
    topbar: px(60.),
    control: px(32.),
    icon_sm: px(14.),
    icon: px(16.),
    icon_lg: px(20.),
    dot: px(8.),
    swatch: px(22.),
    logo: px(30.),
    dialog_w: px(460.),
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
                surface: c(0xffffff),
                surface_sunk: c(0xfafbf8),
                border: c(0xe7eae5),
                hover: c(0xf2f5f1),
                text: c(0x18211f),
                text_body: c(0x45524a),
                text_muted: c(0x6b7871),
                text_subtle: c(0x939e97),
                primary: c(0x315c4b),
                primary_hover: c(0x284d3e),
                primary_fg: c(0xffffff),
                primary_tint: c(0xeaf2ec),
                primary_on_tint: c(0x264b3b),
                selection: c(0x4f8a6f),
                scrim: ca(0x18211f55),
                toast: c(0x22392f),
                toast_fg: c(0xffffff),
                danger: c(0xb5473a),
            },
            text: TEXT,
            space: SPACE,
            radius: RADIUS,
            size: SIZE,
            font: "Hanken Grotesk",
        }
    }

    pub fn dark() -> Self {
        Theme {
            is_dark: true,
            colors: Colors {
                canvas: c(0x121614),
                canvas_dot: c(0x2a312d),
                surface: c(0x191e1c),
                surface_sunk: c(0x151a18),
                border: c(0x29302c),
                hover: c(0x222a26),
                text: c(0xe9ede9),
                text_body: c(0xc3ccc6),
                text_muted: c(0x95a29b),
                text_subtle: c(0x6f7c75),
                primary: c(0x3f7a61),
                primary_hover: c(0x4a8a6e),
                primary_fg: c(0xffffff),
                primary_tint: c(0x223229),
                primary_on_tint: c(0xa9d4bc),
                selection: c(0x79b596),
                scrim: ca(0x00000088),
                toast: c(0xe6efe9),
                toast_fg: c(0x16201b),
                danger: c(0xe07a6c),
            },
            text: TEXT,
            space: SPACE,
            radius: RADIUS,
            size: SIZE,
            font: "Hanken Grotesk",
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
        };
        c(if self.is_dark { dark } else { light })
    }

    pub fn ink(&self, i: Ink) -> Hsla {
        let (light, dark) = match i {
            Ink::Graphite => (0x2a332f, 0xdfe5e1),
            Ink::Forest => (0x3d7a5d, 0x7cc19d),
            Ink::Coral => (0xd0674a, 0xf08c70),
            Ink::Cobalt => (0x3f68b0, 0x82a6e6),
            Ink::Marigold => (0xc88d1c, 0xe9b54d),
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

    /// Menus, toolbars and dialogs.
    pub fn shadow_float(&self) -> Vec<BoxShadow> {
        let a = if self.is_dark { 0.5 } else { 0.12 };
        vec![BoxShadow {
            color: hsla(150. / 360., 0.2, 0.1, a),
            offset: point(px(0.), px(12.)),
            blur_radius: px(32.),
            spread_radius: px(0.),
            inset: false,
        }]
    }
}

pub struct ActiveTheme(pub Theme);
impl Global for ActiveTheme {}

pub fn set(dark: bool, cx: &mut App) {
    cx.set_global(ActiveTheme(if dark { Theme::dark() } else { Theme::light() }));
}

pub fn theme(cx: &App) -> &Theme {
    &cx.global::<ActiveTheme>().0
}
