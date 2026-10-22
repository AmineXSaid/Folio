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

/// Undo steps kept on disk, so undo works after a restart.
pub const HISTORY_KEPT: usize = 30;

/// Writes the newest undo steps to `history.json`.
pub fn save_history(dir: &Path, undo: &[Board]) -> std::io::Result<()> {
    std::fs::create_dir_all(dir)?;
    let keep = &undo[undo.len().saturating_sub(HISTORY_KEPT)..];
    let text = serde_json::to_string(keep).map_err(std::io::Error::other)?;
    let tmp = dir.join("history.json.tmp");
    std::fs::write(&tmp, text)?;
    std::fs::rename(&tmp, dir.join("history.json"))
}

/// The undo steps from the last session (none if unreadable).
pub fn load_history(dir: &Path) -> Vec<Board> {
    std::fs::read_to_string(dir.join("history.json"))
        .ok()
        .and_then(|t| serde_json::from_str::<Vec<Board>>(&t).ok())
        .map(|v| {
            v.into_iter()
                .map(|mut b| {
                    b.normalize();
                    b
                })
                .collect()
        })
        .unwrap_or_default()
}

pub fn backup_dir(dir: &Path) -> PathBuf {
    dir.join("backups")
}

/// Days of backups kept.
pub const BACKUPS_KEPT: usize = 14;

/// Writes today's backup (`backups/YYYY-MM-DD.json`) unless it exists, and
/// keeps the newest 14.
pub fn backup_daily(dir: &Path, board: &Board, day: &str) -> std::io::Result<bool> {
    let b = backup_dir(dir);
    std::fs::create_dir_all(&b)?;
    let path = b.join(format!("{day}.json"));
    if path.exists() {
        return Ok(false);
    }
    let text = serde_json::to_string(board).map_err(std::io::Error::other)?;
    std::fs::write(b.join("backup.tmp"), text)?;
    std::fs::rename(b.join("backup.tmp"), &path)?;
    let mut days = backups(dir);
    while days.len() > BACKUPS_KEPT {
        let old = days.pop().unwrap();
        let _ = std::fs::remove_file(b.join(format!("{old}.json")));
    }
    Ok(true)
}

/// Whose copy a sync kept aside, when notes changed in two places.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Side {
    ThisComputer,
    Drive,
}

impl Side {
    fn tag(self) -> &'static str {
        match self {
            Side::ThisComputer => "computer",
            Side::Drive => "drive",
        }
    }
}

/// Keeps the side a sync didn't use as a backup, `YYYY-MM-DD_HHMM_<side>`,
/// listed and restorable like the daily ones. Returns its name.
pub fn save_copy(dir: &Path, board_text: &str, day: &str, hhmm: &str, side: Side) -> std::io::Result<String> {
    let b = backup_dir(dir);
    std::fs::create_dir_all(&b)?;
    let name = format!("{day}_{hhmm}_{}", side.tag());
    std::fs::write(b.join("copy.tmp"), board_text)?;
    std::fs::rename(b.join("copy.tmp"), b.join(format!("{name}.json")))?;
    Ok(name)
}

/// What a backup is called in the list.
pub fn backup_title(name: &str) -> String {
    let mut parts = name.splitn(3, '_');
    let (day, hhmm, side) = (parts.next().unwrap_or(name), parts.next(), parts.next());
    let at = |t: &str| if t.len() == 4 { format!("{day} {}:{}", &t[..2], &t[2..]) } else { day.to_string() };
    match (hhmm, side) {
        (Some(t), Some("drive")) => format!("Google Drive's copy · {}", at(t)),
        (Some(t), Some("computer")) => format!("This computer's copy · {}", at(t)),
        _ => format!("Backup of {day}"),
    }
}

fn is_backup_name(n: &str) -> bool {
    let day = n.get(..10).unwrap_or("");
    day.len() == 10 && day.as_bytes()[4] == b'-' && (n.len() == 10 || n[10..].starts_with('_'))
}

/// The backups there are (days, and copies kept by a sync), newest first.
pub fn backups(dir: &Path) -> Vec<String> {
    let mut days: Vec<String> = std::fs::read_dir(backup_dir(dir))
        .map(|r| {
            r.flatten()
                .filter_map(|e| e.file_name().to_str().and_then(|n| n.strip_suffix(".json")).map(String::from))
                .filter(|n| is_backup_name(n))
                .collect()
        })
        .unwrap_or_default();
    days.sort_by(|a, b| b.cmp(a));
    days
}

pub fn load_backup(dir: &Path, day: &str) -> Option<Board> {
    let text = std::fs::read_to_string(backup_dir(dir).join(format!("{day}.json"))).ok()?;
    let mut b: Board = serde_json::from_str(&text).ok()?;
    b.normalize();
    Some(b)
}

/// The board as it was when this computer and Google Drive last matched:
/// what a merge works out changes from.
pub fn save_sync_base(dir: &Path, board_text: &str) -> std::io::Result<()> {
    std::fs::create_dir_all(dir)?;
    std::fs::write(dir.join("sync-base.tmp"), board_text)?;
    std::fs::rename(dir.join("sync-base.tmp"), dir.join("sync-base.json"))
}

pub fn load_sync_base(dir: &Path) -> Option<Board> {
    let text = std::fs::read_to_string(dir.join("sync-base.json")).ok()?;
    let mut b: Board = serde_json::from_str(&text).ok()?;
    b.normalize();
    Some(b)
}

pub fn has_sync_base(dir: &Path) -> bool {
    dir.join("sync-base.json").exists()
}

pub fn forget_sync_base(dir: &Path) {
    let _ = std::fs::remove_file(dir.join("sync-base.json"));
}

/// A new picture's file name: its id and a random tail, so two computers
/// that sync never give different pictures the same name.
pub fn media_name(id: u64, ext: &str) -> String {
    let mut r = [0u8; 3];
    let _ = getrandom::getrandom(&mut r);
    format!("{id}-{:02x}{:02x}{:02x}.{ext}", r[0], r[1], r[2])
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
    let name = media_name(id, &ext);
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

    #[test]
    fn daily_backups_keep_two_weeks() {
        let dir = temp_dir("backups");
        let board = demo::board();
        for d in 1..=16 {
            assert!(backup_daily(&dir, &board, &format!("2026-10-{d:02}")).unwrap());
        }
        assert!(!backup_daily(&dir, &board, "2026-10-16").unwrap());
        let days = backups(&dir);
        assert_eq!(days.len(), BACKUPS_KEPT);
        assert_eq!(days[0], "2026-10-16");
        assert_eq!(load_backup(&dir, "2026-10-10"), Some(board));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn copies_kept_by_a_sync_are_backups_too() {
        let dir = temp_dir("copies");
        let board = demo::board();
        backup_daily(&dir, &board, "2026-10-21").unwrap();
        let text = serde_json::to_string(&board).unwrap();
        let name = save_copy(&dir, &text, "2026-10-22", "0941", Side::Drive).unwrap();
        assert_eq!(backups(&dir), vec![name.clone(), "2026-10-21".to_string()]);
        assert_eq!(load_backup(&dir, &name), Some(board));
        assert_eq!(backup_title(&name), "Google Drive's copy · 2026-10-22 09:41");
        assert_eq!(backup_title("2026-10-22_1200_computer"), "This computer's copy · 2026-10-22 12:00");
        assert_eq!(backup_title("2026-10-21"), "Backup of 2026-10-21");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn undo_history_survives_a_restart() {
        let dir = temp_dir("history");
        let steps: Vec<Board> = (0..40).map(|i| {
            let mut b = demo::board();
            b.next_id += i;
            b
        }).collect();
        save_history(&dir, &steps).unwrap();
        let back = load_history(&dir);
        assert_eq!(back.len(), HISTORY_KEPT);
        assert_eq!(back.last(), steps.last());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
