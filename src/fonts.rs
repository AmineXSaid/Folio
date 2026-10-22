//! Which face Folio's interface is drawn in.
//!
//! Folio bundles only openly licensed fonts (Hanken Grotesk, JetBrains
//! Mono, Comic Neue). When Anthropic Sans is on this computer, Folio uses
//! it everywhere instead: installed in the system, or placed as .ttf/.otf
//! files in the `fonts` folder inside Folio's data folder. Those are read at
//! start and never ship inside the app.

use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use gpui::App;

/// The interface face: the first of these found, else the bundled one.
pub const SANS_PREFERRED: &[&str] = &["Anthropic Sans", "Anthropic Sans Text"];
pub const SANS_BUNDLED: &str = "Hanken Grotesk";

/// A font file larger than this is skipped, and at most this many are read.
const MAX_FILE: u64 = 16 * 1024 * 1024;
const MAX_FILES: usize = 32;

static SANS: OnceLock<&'static str> = OnceLock::new();

/// The interface face (the bundled one until [`choose`] has run).
pub fn sans() -> &'static str {
    SANS.get().copied().unwrap_or(SANS_BUNDLED)
}

/// The first preferred family that's available (any case), else `fallback`.
pub fn pick(available: &[String], preferred: &[&str], fallback: &'static str) -> &'static str {
    preferred
        .iter()
        .find_map(|want| available.iter().find(|name| name.eq_ignore_ascii_case(want)))
        // Chosen once per run, so leaking the name is fine.
        .map(|name| &*Box::leak(name.clone().into_boxed_str()))
        .unwrap_or(fallback)
}

/// Folio's own fonts folder: `fonts` in its data folder.
pub fn user_dir() -> PathBuf {
    crate::store::data_dir().join("fonts")
}

/// The .ttf / .otf files in `dir`, in name order, within the limits.
pub fn read_dir_fonts(dir: &Path) -> Vec<Vec<u8>> {
    let Ok(entries) = std::fs::read_dir(dir) else { return vec![] };
    let mut paths: Vec<PathBuf> = entries
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().and_then(|x| x.to_str()).is_some_and(|x| x.eq_ignore_ascii_case("ttf") || x.eq_ignore_ascii_case("otf")))
        .filter(|p| p.metadata().is_ok_and(|m| m.is_file() && m.len() <= MAX_FILE))
        .collect();
    paths.sort();
    paths.into_iter().take(MAX_FILES).filter_map(|p| std::fs::read(p).ok()).collect()
}

/// At start, after the bundled fonts and before the theme: loads the fonts
/// folder, then settles the face from everything the system can see.
pub fn choose(cx: &App) {
    let files = read_dir_fonts(&user_dir());
    if !files.is_empty() {
        if let Err(e) = cx.text_system().add_fonts(files.into_iter().map(Into::into).collect()) {
            eprintln!("could not load the fonts in {}: {e}", user_dir().display());
        }
    }
    let available = cx.text_system().all_font_names();
    let _ = SANS.set(pick(&available, SANS_PREFERRED, SANS_BUNDLED));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn anthropic_sans_wins_when_present() {
        let names = |v: &[&str]| v.iter().map(|s| s.to_string()).collect::<Vec<_>>();
        assert_eq!(pick(&names(&["Arial", "anthropic sans"]), SANS_PREFERRED, SANS_BUNDLED), "anthropic sans");
        assert_eq!(pick(&names(&["Anthropic Sans Text"]), SANS_PREFERRED, SANS_BUNDLED), "Anthropic Sans Text");
        assert_eq!(pick(&names(&["Arial"]), SANS_PREFERRED, SANS_BUNDLED), SANS_BUNDLED);
    }

    #[test]
    fn only_font_files_are_read() {
        let dir = std::env::temp_dir().join(format!("folio-fonts-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("a.TTF"), b"x").unwrap();
        std::fs::write(dir.join("b.otf"), b"y").unwrap();
        std::fs::write(dir.join("notes.txt"), b"z").unwrap();
        assert_eq!(read_dir_fonts(&dir), vec![b"x".to_vec(), b"y".to_vec()]);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
