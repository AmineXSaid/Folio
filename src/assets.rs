//! What ships inside the binary: Lucide icons (ISC, `assets/icons/LICENSE`),
//! Hanken Grotesk, JetBrains Mono and Comic Neue (OFL), the Folio logo, and
//! any fonts placed in `assets/fonts/extra` at build time (see build.rs).

use std::borrow::Cow;

use gpui::{App, AssetSource, Result, SharedString};

// Every icon and brand picture, listed by build.rs.
include!(concat!(env!("OUT_DIR"), "/assets.rs"));

pub struct Assets;

impl AssetSource for Assets {
    fn load(&self, path: &str) -> Result<Option<Cow<'static, [u8]>>> {
        Ok(FILES
            .iter()
            .find(|(p, _)| *p == path)
            .map(|(_, bytes)| Cow::Borrowed(*bytes)))
    }

    fn list(&self, path: &str) -> Result<Vec<SharedString>> {
        Ok(FILES
            .iter()
            .filter(|(p, _)| p.starts_with(path))
            .map(|(p, _)| SharedString::from(*p))
            .collect())
    }
}

pub fn load_fonts(cx: &App) {
    let fonts: Vec<Cow<'static, [u8]>> = vec![
        Cow::Borrowed(include_bytes!("../assets/fonts/HankenGrotesk-Regular.ttf")),
        Cow::Borrowed(include_bytes!("../assets/fonts/HankenGrotesk-Medium.ttf")),
        Cow::Borrowed(include_bytes!("../assets/fonts/HankenGrotesk-SemiBold.ttf")),
        Cow::Borrowed(include_bytes!("../assets/fonts/HankenGrotesk-Bold.ttf")),
        Cow::Borrowed(include_bytes!("../assets/fonts/HankenGrotesk-Italic.ttf")),
        Cow::Borrowed(include_bytes!("../assets/fonts/JetBrainsMono-Regular.ttf")),
        Cow::Borrowed(include_bytes!("../assets/fonts/ComicNeue-Regular.otf")),
        Cow::Borrowed(include_bytes!("../assets/fonts/ComicNeue-Bold.otf")),
    ];
    // Fonts put in assets/fonts/extra when this was built (see build.rs).
    let fonts: Vec<Cow<'static, [u8]>> = fonts.into_iter().chain(EXTRA_FONTS.iter().map(|f| Cow::Borrowed(*f))).collect();
    if let Err(e) = cx.text_system().add_fonts(fonts) {
        eprintln!("could not load the bundled fonts: {e}");
    }
}

