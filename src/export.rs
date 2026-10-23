//! Sheets out to files, and back in.
//!
//! - **Markdown**: a folder with one `.md` file per sheet and a `media/`
//!   folder for its pictures. It opens in any notes app (Obsidian, Notion,
//!   VS Code, GitHub). Notes read top to bottom, grouped by area; drawings
//!   aren't text, so they stay behind.
//! - **Folio file** (`.folio`): the sheets exactly, with their pictures
//!   inside, to keep, share or open in another Folio (File › Import).

use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::{Path, PathBuf};

use base64::Engine;
use serde::{Deserialize, Serialize};

use crate::model::*;
use crate::store;

/// A sheet name made safe for a file name on Windows, macOS and Linux.
pub fn file_name(name: &str) -> String {
    let cleaned: String = name
        .chars()
        .map(|c| if matches!(c, '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*') || c.is_control() { ' ' } else { c })
        .collect();
    let cleaned = cleaned.split_whitespace().collect::<Vec<_>>().join(" ");
    let cleaned = cleaned.trim_end_matches(['.', ' ']).to_string();
    let reserved = ["CON", "PRN", "AUX", "NUL", "COM1", "COM2", "COM3", "LPT1", "LPT2", "LPT3"];
    if cleaned.is_empty() || reserved.contains(&cleaned.to_ascii_uppercase().as_str()) {
        format!("Sheet {cleaned}").trim().to_string()
    } else {
        cleaned.chars().take(80).collect()
    }
}

/// `base`, or `base 2`, `base 3`… so nothing is written over.
fn unused(dir: &Path, base: &str, ext: &str) -> PathBuf {
    let with = |n: usize| {
        let stem = if n == 1 { base.to_string() } else { format!("{base} {n}") };
        dir.join(if ext.is_empty() { stem } else { format!("{stem}.{ext}") })
    };
    (1..).map(with).find(|p| !p.exists()).expect("some name is free")
}

/// The pictures a sheet uses: note pictures, link-card icons and its
/// background picture.
fn media_of(c: &Category) -> Vec<String> {
    let mut out: Vec<String> = c.backdrop.image.iter().cloned().collect();
    for s in &c.stickers {
        out.extend(s.image.clone());
        out.extend(s.link.as_ref().and_then(|l| l.icon.clone()));
    }
    out.sort();
    out.dedup();
    out
}

/// One sheet as Markdown, with pictures at `media/NAME`.
pub fn sheet_markdown(c: &Category) -> String {
    let mut md = String::new();
    let icon = if c.icon.is_empty() { String::new() } else { format!("{} ", c.icon) };
    md.push_str(&format!("# {icon}{}\n\n", c.name));
    if !c.blurb.trim().is_empty() {
        md.push_str(&format!("{}\n\n", c.blurb.trim()));
    }

    // Each note goes under the area its middle is in, if any.
    let area_of = |s: &Sticker| {
        let (x, y) = (s.rect.x + s.rect.w / 2., s.rect.y + s.rect.h / 2.);
        c.frames.iter().position(|f| x >= f.rect.x && x <= f.rect.x + f.rect.w && y >= f.rect.y && y <= f.rect.y + f.rect.h)
    };
    let mut groups: BTreeMap<Option<usize>, Vec<&Sticker>> = BTreeMap::new();
    for s in &c.stickers {
        groups.entry(area_of(s)).or_default().push(s);
    }
    for (area, mut notes) in groups {
        // Reading order: top to bottom, then left to right.
        notes.sort_by(|a, b| a.rect.y.total_cmp(&b.rect.y).then(a.rect.x.total_cmp(&b.rect.x)));
        let level = if let Some(i) = area {
            md.push_str(&format!("## {}\n\n", if c.frames[i].title.is_empty() { "Area" } else { &c.frames[i].title }));
            "###"
        } else {
            "##"
        };
        for s in notes {
            note_markdown(&mut md, s, level);
        }
    }

    let mut labels: Vec<&Label> = c.labels.iter().filter(|l| !l.text.trim().is_empty()).collect();
    if !labels.is_empty() {
        labels.sort_by(|a, b| a.at.y.total_cmp(&b.at.y).then(a.at.x.total_cmp(&b.at.x)));
        md.push_str("## Written on the sheet\n\n");
        for l in labels {
            for line in l.text.lines() {
                md.push_str(&format!("> {line}\n"));
            }
            md.push('\n');
        }
    }
    md.trim_end().to_string() + "\n"
}

fn note_markdown(md: &mut String, s: &Sticker, level: &str) {
    let title = s.title.trim();
    if !title.is_empty() {
        md.push_str(&format!("{level} {title}\n\n"));
    }
    if let Some(link) = &s.link {
        let name = if link.title.is_empty() { &link.url } else { &link.title };
        md.push_str(&format!("[{name}]({})\n\n", link.url));
    }
    if let Some(img) = &s.image {
        md.push_str(&format!("![](media/{})\n\n", img.replace(' ', "%20")));
    }
    if !s.body.trim().is_empty() {
        md.push_str(s.body.trim_end());
        md.push_str("\n\n");
    }
    if let Some(due) = s.due {
        md.push_str(&format!("*Reminder: {}*\n\n", crate::time::date_time(due)));
    }
    if title.is_empty() && s.body.trim().is_empty() && s.image.is_none() && s.link.is_none() {
        return;
    }
    md.push_str("---\n\n");
}

/// Writes the sheets as Markdown into a new folder inside `dest`, named
/// `folder_name`. Returns the folder.
pub fn export_markdown(sheets: &[&Category], media: &Path, dest: &Path, folder_name: &str) -> std::io::Result<PathBuf> {
    let out = unused(dest, &file_name(folder_name), "");
    std::fs::create_dir_all(&out)?;
    let mut used = HashSet::new();
    for c in sheets {
        let path = unused(&out, &file_name(&c.name), "md");
        std::fs::write(&path, sheet_markdown(c))?;
        used.extend(media_of(c));
    }
    copy_media(&used, media, &out.join("media"))?;
    Ok(out)
}

fn copy_media(names: &HashSet<String>, from: &Path, to: &Path) -> std::io::Result<()> {
    for name in names {
        let src = from.join(name);
        if src.is_file() {
            std::fs::create_dir_all(to)?;
            std::fs::copy(&src, to.join(name))?;
        }
    }
    Ok(())
}

/// A `.folio` file: sheets, and their pictures inside it.
#[derive(Serialize, Deserialize)]
pub struct Package {
    /// Format version.
    pub folio: u32,
    pub sheets: Vec<Category>,
    /// Picture name → its bytes, base64.
    #[serde(default)]
    pub media: BTreeMap<String, String>,
}

pub fn export_folio(sheets: &[&Category], media: &Path, path: &Path) -> std::io::Result<()> {
    let mut pkg = Package { folio: 1, sheets: sheets.iter().map(|c| (*c).clone()).collect(), media: BTreeMap::new() };
    for c in &mut pkg.sheets {
        // Desktop windows belong to this computer.
        c.stickers.iter_mut().for_each(|s| s.popped = None);
        for name in media_of(c) {
            if let Ok(bytes) = std::fs::read(media.join(&name)) {
                pkg.media.insert(name, base64::engine::general_purpose::STANDARD.encode(bytes));
            }
        }
    }
    let tmp = path.with_extension("folio.tmp");
    std::fs::write(&tmp, serde_json::to_string(&pkg).map_err(std::io::Error::other)?)?;
    std::fs::rename(tmp, path)
}

pub fn read_folio(path: &Path) -> Result<Package, String> {
    let text = std::fs::read_to_string(path).map_err(|e| format!("Couldn't open it: {e}"))?;
    let pkg: Package = serde_json::from_str(&text).map_err(|_| "That isn't a Folio file".to_string())?;
    if pkg.sheets.is_empty() {
        return Err("That Folio file has no sheets".into());
    }
    Ok(pkg)
}

/// Adds a package's sheets as new sheets (they never replace anything),
/// with their pictures. Returns how many were added.
pub fn import(board: &mut Board, pkg: Package, media: &Path) -> std::io::Result<usize> {
    // Pictures: each under a new name, so nothing here is written over.
    let mut renamed: HashMap<String, String> = HashMap::new();
    for (name, data) in &pkg.media {
        let Ok(bytes) = base64::engine::general_purpose::STANDARD.decode(data) else { continue };
        let ext = Path::new(name).extension().and_then(|e| e.to_str()).unwrap_or("png").to_ascii_lowercase();
        let new = store::media_name(board.fresh_id(), &ext);
        std::fs::create_dir_all(media)?;
        std::fs::write(media.join(&new), bytes)?;
        renamed.insert(name.clone(), new);
    }
    let n = pkg.sheets.len();
    for mut c in pkg.sheets {
        if let Some(i) = c.backdrop.image.as_mut() {
            if let Some(new) = renamed.get(i) {
                *i = new.clone();
            }
        }
        for s in &mut c.stickers {
            if let Some(i) = s.image.as_mut() {
                if let Some(new) = renamed.get(i) {
                    *i = new.clone();
                }
            }
            if let Some(icon) = s.link.as_mut().and_then(|l| l.icon.as_mut()) {
                if let Some(new) = renamed.get(icon) {
                    *icon = new.clone();
                }
            }
        }
        let name = c.name.clone();
        let ix = board.adopt_sheet(c, name);
        board.categories[ix].edited = crate::time::now();
    }
    Ok(n)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::demo;

    fn temp(tag: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("folio-export-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn names_are_safe_on_windows() {
        assert_eq!(file_name("Plans: 2026/27?"), "Plans 2026 27");
        assert_eq!(file_name("con"), "Sheet con");
        assert_eq!(file_name("..."), "Sheet");
        assert_eq!(file_name("🎨 Studio"), "🎨 Studio");
    }

    #[test]
    fn a_sheet_reads_well_as_markdown() {
        let mut c = Category::new(1, "Trip", Accent::Slate);
        c.icon = "✈️".into();
        c.frames.push(Frame { id: 9, rect: Rect { x: 0., y: 0., w: 500., h: 500. }, title: "Kyoto".into(), accent: Accent::Rose });
        let note = |id, title: &str, body: &str, x, y| Sticker {
            id,
            title: title.into(),
            body: body.into(),
            rect: Rect { x, y, w: 100., h: 100. },
            ..Default::default()
        };
        c.stickers = vec![note(3, "Later", "x", 900., 50.), note(2, "Temple", "- [ ] Fushimi", 10., 200.), note(4, "First", "y", 800., 10.)];
        c.labels.push(Label { id: 5, at: v2(0., 0.), text: "don't forget".into(), ink: Ink::Graphite });
        let md = sheet_markdown(&c);
        assert!(md.starts_with("# ✈️ Trip\n"));
        let first = md.find("## First").unwrap();
        let later = md.find("## Later").unwrap();
        let area = md.find("## Kyoto").unwrap();
        assert!(first < later && later < area, "{md}");
        assert!(md.contains("### Temple\n\n- [ ] Fushimi"));
        assert!(md.contains("> don't forget"));
    }

    #[test]
    fn markdown_export_writes_sheets_and_pictures() {
        let dir = temp("md");
        let media = dir.join("media-src");
        std::fs::create_dir_all(&media).unwrap();
        std::fs::write(media.join("p.png"), b"png").unwrap();
        let mut b = demo::board();
        b.categories[0].stickers[0].image = Some("p.png".into());
        let sheets: Vec<&Category> = b.categories.iter().take(2).collect();
        let out = export_markdown(&sheets, &media, &dir, "Folio export").unwrap();
        let again = export_markdown(&sheets, &media, &dir, "Folio export").unwrap();
        assert_ne!(out, again, "a second export doesn't write over the first");
        assert_eq!(std::fs::read(out.join("media/p.png")).unwrap(), b"png");
        let files: Vec<_> = std::fs::read_dir(&out).unwrap().flatten().filter(|e| e.path().extension().is_some_and(|x| x == "md")).collect();
        assert_eq!(files.len(), 2);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_folio_file_comes_back_as_new_sheets_with_its_pictures() {
        let dir = temp("folio");
        let (media_a, media_b) = (dir.join("a"), dir.join("b"));
        std::fs::create_dir_all(&media_a).unwrap();
        std::fs::write(media_a.join("1.png"), b"one").unwrap();
        std::fs::write(media_a.join("2.svg"), b"<svg/>").unwrap();
        let mut a = demo::board();
        a.categories[1].stickers[0].image = Some("1.png".into());
        a.categories[1].backdrop = Backdrop { pattern: Pattern::Grid, tint: Tint::Warm, image: Some("2.svg".into()), soften: true };
        let file = dir.join("Trip.folio");
        export_folio(&[&a.categories[1]], &media_a, &file).unwrap();

        let mut b = demo::board();
        let before = b.categories.len();
        let n = import(&mut b, read_folio(&file).unwrap(), &media_b).unwrap();
        assert_eq!(n, 1);
        assert_eq!(b.categories.len(), before + 1);
        let new = b.categories.last().unwrap();
        assert_eq!(new.name, a.categories[1].name);
        assert_eq!(new.stickers.len(), a.categories[1].stickers.len());
        assert_ne!(new.id, a.categories[1].id, "fresh ids");
        let img = new.stickers[0].image.clone().unwrap();
        assert_eq!(std::fs::read(media_b.join(img)).unwrap(), b"one");
        // The background picture comes too, under its new name.
        assert_eq!(new.backdrop.pattern, Pattern::Grid);
        let bg = new.backdrop.image.clone().unwrap();
        assert_ne!(bg, "2.svg");
        assert_eq!(std::fs::read(media_b.join(bg)).unwrap(), b"<svg/>");
        assert!(read_folio(&dir.join("a/1.png")).is_err());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
