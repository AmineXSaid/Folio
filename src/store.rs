//! Where the board lives on disk: one JSON file in the app's data folder,
//! written whole to a temporary file and renamed over the old one, so a
//! crash mid-save never leaves half a file. Pictures dropped on a sheet are
//! copied into `media/` beside it, so the folder is the whole workspace.

use std::path::{Path, PathBuf};

use crate::demo;
use crate::model::Board;

/// `FOLIO_DATA_DIR` if set; otherwise the platform's per-user data folder.
pub fn data_dir() -> PathBuf {
    if let Some(d) = std::env::var_os("FOLIO_DATA_DIR") {
        return PathBuf::from(d);
    }
    #[cfg(windows)]
    if let Some(d) = std::env::var_os("APPDATA") {
        return PathBuf::from(d).join("Folio");
    }
    #[cfg(target_os = "macos")]
    if let Some(h) = std::env::var_os("HOME") {
        return PathBuf::from(h).join("Library/Application Support/Folio");
    }
    if let Some(d) = std::env::var_os("XDG_DATA_HOME") {
        return PathBuf::from(d).join("folio");
    }
    if let Some(h) = std::env::var_os("HOME") {
        return PathBuf::from(h).join(".local/share/folio");
    }
    PathBuf::from("folio-data")
}

pub fn board_file(dir: &Path) -> PathBuf {
    dir.join("board.json")
}

pub fn media_dir(dir: &Path) -> PathBuf {
    dir.join("media")
}

/// The saved board, the demo board on first run, or — if the file can't be
/// read — the demo board, with the bad file kept aside as `board.json.bad`.
pub fn load(dir: &Path) -> Board {
    let path = board_file(dir);
    let Ok(text) = std::fs::read_to_string(&path) else {
        return demo::board();
    };
    match serde_json::from_str::<Board>(&text) {
        Ok(mut b) => {
            b.normalize();
            b
        }
        Err(_) => {
            let _ = std::fs::rename(&path, dir.join("board.json.bad"));
            demo::board()
        }
    }
}

pub fn save(dir: &Path, board: &Board) -> std::io::Result<()> {
    std::fs::create_dir_all(dir)?;
    let tmp = dir.join("board.json.tmp");
    let text = serde_json::to_string(board).map_err(std::io::Error::other)?;
    std::fs::write(&tmp, text)?;
    std::fs::rename(&tmp, board_file(dir))
}

/// The picture formats a sticker shows.
pub fn is_image(path: &Path) -> bool {
    matches!(
        path.extension().and_then(|e| e.to_str()).map(str::to_ascii_lowercase).as_deref(),
        Some("png" | "jpg" | "jpeg" | "gif" | "webp" | "svg" | "bmp")
    )
}

/// Copies a picture into the media folder under a new name; returns the
/// name it was stored as.
pub fn import_image(dir: &Path, src: &Path, id: u64) -> std::io::Result<String> {
    let media = media_dir(dir);
    std::fs::create_dir_all(&media)?;
    let ext = src.extension().and_then(|e| e.to_str()).unwrap_or("png").to_ascii_lowercase();
    let name = format!("{id}.{ext}");
    std::fs::copy(src, media.join(&name))?;
    Ok(name)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(tag: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("folio-test-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        d
    }

    #[test]
    fn saves_and_loads_the_same_board() {
        let dir = temp_dir("roundtrip");
        let board = demo::board();
        save(&dir, &board).unwrap();
        assert_eq!(load(&dir), board);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_broken_file_is_kept_aside() {
        let dir = temp_dir("broken");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(board_file(&dir), "{ not json").unwrap();
        let b = load(&dir);
        assert!(!b.categories.is_empty());
        assert!(dir.join("board.json.bad").exists());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn knows_picture_files() {
        assert!(is_image(Path::new("a/b.PNG")));
        assert!(is_image(Path::new("x.svg")));
        assert!(!is_image(Path::new("notes.txt")));
    }
}
