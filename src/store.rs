//! Where the board lives on disk: one JSON file in the app's data folder,
//! written whole to a temporary file and renamed over the old one, so a
//! crash mid-save never leaves half a file. Pictures dropped on a sheet are
//! copied into `media/` beside it, so the folder is the whole workspace.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use serde::{Deserialize, Serialize};

use crate::demo;
use crate::history::Change;
use crate::model::{Board, Camera, Id};

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

/// The saved board; on first run one empty sheet (the sample sheets with
/// `FOLIO_DEMO=1`); if the file can't be read, an empty board, with the bad
/// file kept aside as `board.json.bad`.
pub fn load(dir: &Path) -> Board {
    let path = board_file(dir);
    let Ok(text) = std::fs::read_to_string(&path) else {
        if std::env::var_os("FOLIO_DEMO").is_some() {
            return demo::board();
        }
        return Board::empty();
    };
    match serde_json::from_str::<Board>(&text) {
        Ok(mut b) => {
            apply_view(dir, &mut b);
            b.normalize();
            b
        }
        Err(_) => {
            let _ = std::fs::rename(&path, dir.join("board.json.bad"));
            Board::empty()
        }
    }
}

/// Saves `board` and its view here and now (the app saves through `Saver`).
#[cfg(test)]
pub fn save(dir: &Path, board: &Board) -> std::io::Result<()> {
    std::fs::create_dir_all(dir)?;
    let text = serde_json::to_string(board).map_err(std::io::Error::other)?;
    write_whole(&view_file(dir), &view_text(board))?;
    write_whole(&board_file(dir), &text)
}

/// Writes `text` to a temporary file, makes sure it's on the disk, then
/// renames it over `path`: after a crash or a power cut the old file or the
/// new one is there, never half of one.
fn write_whole(path: &Path, text: &str) -> std::io::Result<()> {
    use std::io::Write;
    let mut name = path.file_name().unwrap_or_default().to_os_string();
    name.push(".tmp");
    let tmp = path.with_file_name(name);
    let mut f = std::fs::File::create(&tmp)?;
    f.write_all(text.as_bytes())?;
    f.sync_all()?;
    drop(f);
    std::fs::rename(&tmp, path)?;
    // The rename itself, on the disk too.
    #[cfg(unix)]
    if let Some(d) = path.parent() {
        let _ = std::fs::File::open(d).and_then(|d| d.sync_all());
    }
    Ok(())
}

/// Where each sheet is scrolled to, on this computer only. It sits beside
/// the board rather than in it, so panning and zooming never rewrite the
/// notes or send them to Drive.
pub fn view_file(dir: &Path) -> PathBuf {
    dir.join("view.json")
}

#[derive(Serialize, Deserialize, Default)]
struct View {
    #[serde(default)]
    cameras: BTreeMap<Id, Camera>,
}

/// The view's text: each sheet's camera, by sheet.
pub fn view_text(board: &Board) -> String {
    let view = View { cameras: board.categories.iter().map(|c| (c.id, c.camera)).collect() };
    serde_json::to_string(&view).unwrap_or_default()
}

/// Puts the saved view on `board`'s sheets. With no view file (a board
/// from before it, or a new folder) the cameras stay as they were read.
fn apply_view(dir: &Path, board: &mut Board) {
    let Some(view) = std::fs::read_to_string(view_file(dir)).ok().and_then(|t| serde_json::from_str::<View>(&t).ok()) else { return };
    for c in &mut board.categories {
        if let Some(cam) = view.cameras.get(&c.id) {
            c.camera = *cam;
        }
    }
}

/// Saves the board and its undo steps away from the UI thread, in order: a
/// save that started earlier never lands over one that started later. The
/// view is numbered on its own, as it's saved on its own too.
#[derive(Clone)]
pub struct Saver {
    dir: PathBuf,
    /// The number of the newest save on the disk.
    written: Arc<Mutex<u64>>,
    view_written: Arc<Mutex<u64>>,
}

impl Saver {
    pub fn new(dir: &Path) -> Self {
        Saver { dir: dir.to_path_buf(), written: Arc::default(), view_written: Arc::default() }
    }

    /// Writes the view as view save number `n`, unless a later one is
    /// already on the disk.
    pub fn write_view(&self, n: u64, text: &str) -> std::io::Result<bool> {
        let mut written = self.view_written.lock().unwrap_or_else(|e| e.into_inner());
        if n <= *written {
            return Ok(false);
        }
        std::fs::create_dir_all(&self.dir)?;
        write_whole(&view_file(&self.dir), text)?;
        *written = n;
        Ok(true)
    }

    /// Writes `board` and `steps` as save number `n`, unless a later one is
    /// already on the disk (then `Ok(false)`). The board is turned to text
    /// before waiting for an earlier save to finish.
    pub fn write(&self, n: u64, board: &Board, steps: &[Arc<Change>]) -> std::io::Result<bool> {
        let text = serde_json::to_string(board).map_err(std::io::Error::other)?;
        let history = history_text(steps);
        let mut written = self.written.lock().unwrap_or_else(|e| e.into_inner());
        if n <= *written {
            return Ok(false);
        }
        std::fs::create_dir_all(&self.dir)?;
        write_whole(&board_file(&self.dir), &text)?;
        *written = n;
        // The board is safe; undo after a restart is a bonus.
        let _ = write_whole(&history_file(&self.dir), &history);
        Ok(true)
    }
}

pub fn history_file(dir: &Path) -> PathBuf {
    dir.join("history.json")
}

/// Undo steps kept on disk, so undo works after a restart: the newest, up to
/// this many and this many bytes.
pub const HISTORY_KEPT: usize = 100;
pub const HISTORY_BYTES: usize = 2 << 20;

/// What `history.json` holds. The file once held whole boards in a list;
/// that's read as no steps.
#[derive(Serialize, Deserialize)]
struct History {
    steps: Vec<Change>,
}

/// The newest undo steps as `history.json`'s text.
fn history_text(steps: &[Arc<Change>]) -> String {
    let mut kept = Vec::new();
    let mut bytes = 0;
    for step in steps.iter().rev().take(HISTORY_KEPT) {
        let Ok(t) = serde_json::to_string(&**step) else { break };
        bytes += t.len() + 1;
        if bytes > HISTORY_BYTES {
            break;
        }
        kept.push(t);
    }
    kept.reverse();
    format!("{{\"steps\":[{}]}}", kept.join(","))
}

/// Writes the newest undo steps to `history.json`.
#[cfg(test)]
pub fn save_history(dir: &Path, steps: &[Arc<Change>]) -> std::io::Result<()> {
    std::fs::create_dir_all(dir)?;
    write_whole(&history_file(dir), &history_text(steps))
}

/// The undo steps from the last run, if the newest of them fits `board`
/// (none if the file is unreadable, from an older Folio, or belongs to a
/// board that changed since).
pub fn load_history(dir: &Path, board: &Board) -> Vec<Arc<Change>> {
    let path = history_file(dir);
    let Ok(text) = std::fs::read_to_string(&path) else { return Vec::new() };
    let Ok(h) = serde_json::from_str::<History>(&text) else {
        // Whole boards, from before steps: they go.
        let _ = std::fs::remove_file(&path);
        return Vec::new();
    };
    match h.steps.last() {
        Some(last) if last.fits(board) => h.steps.into_iter().map(Arc::new).collect(),
        _ => Vec::new(),
    }
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
    fn where_sheets_are_scrolled_to_stays_out_of_the_board() {
        let dir = temp_dir("view");
        let mut board = demo::board();
        board.categories[0].camera = Camera { pan: crate::model::v2(-300., 42.), zoom: 1.75 };
        save(&dir, &board).unwrap();
        let text = std::fs::read_to_string(board_file(&dir)).unwrap();
        assert!(!text.contains("\"camera\""), "the board file holds no cameras");
        assert_eq!(load(&dir).categories[0].camera, board.categories[0].camera);

        // Only the view saved: the board file isn't touched.
        board.categories[0].camera.zoom = 0.5;
        let saver = Saver::new(&dir);
        assert!(saver.write_view(2, &view_text(&board)).unwrap());
        assert!(!saver.write_view(1, "{}").unwrap(), "an older view never lands over a newer one");
        assert_eq!(std::fs::read_to_string(board_file(&dir)).unwrap(), text);
        assert_eq!(load(&dir).categories[0].camera.zoom, 0.5);

        // A board from before view.json still opens where it was.
        std::fs::remove_file(view_file(&dir)).unwrap();
        let old = text.replacen("\"accent\"", "\"camera\":{\"pan\":{\"x\":5.0,\"y\":6.0},\"zoom\":2.0},\"accent\"", 1);
        std::fs::write(board_file(&dir), old).unwrap();
        assert_eq!(load(&dir).categories[0].camera.zoom, 2.);
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
        let mut board = demo::board();
        let mut steps = Vec::new();
        for i in 0..120 {
            let before = board.clone();
            board.sheet_mut().stickers[0].body = format!("Edit {i}");
            steps.push(Arc::new(Change::diff(&before, &board)));
        }
        save_history(&dir, &steps).unwrap();
        let back = load_history(&dir, &board);
        assert_eq!(back.len(), HISTORY_KEPT);
        assert_eq!(back.last(), steps.last());
        // Small steps make a small file.
        let size = std::fs::metadata(history_file(&dir)).unwrap().len();
        assert!(size < 200_000, "history.json is {size} bytes");
        // Steps that belong to another board are dropped.
        let mut other = board.clone();
        other.sheet_mut().stickers.remove(0);
        assert!(load_history(&dir, &other).is_empty());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn whole_boards_from_older_history_are_dropped() {
        let dir = temp_dir("old-history");
        std::fs::create_dir_all(&dir).unwrap();
        let old = serde_json::to_string(&vec![demo::board(), demo::board()]).unwrap();
        std::fs::write(history_file(&dir), old).unwrap();
        assert!(load_history(&dir, &demo::board()).is_empty());
        assert!(!history_file(&dir).exists());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn an_older_save_never_lands_over_a_newer_one() {
        let dir = temp_dir("saver");
        let saver = Saver::new(&dir);
        let mut newer = demo::board();
        newer.sheet_mut().name = "Newer".into();
        assert!(saver.write(2, &newer, &[]).unwrap());
        assert!(!saver.write(1, &demo::board(), &[]).unwrap());
        assert_eq!(load(&dir).sheet().name, "Newer");
        assert!(!dir.join("board.json.tmp").exists());
        let _ = std::fs::remove_dir_all(&dir);
    }
}

/// Where a crash is written down: `crash.log` beside the board.
pub fn crash_file(dir: &Path) -> PathBuf {
    dir.join("crash.log")
}

/// One crash entry: when, where, what, and the stack if Rust kept one.
pub fn crash_entry(when: &str, location: &str, message: &str, stack: &str) -> String {
    let mut s = format!("--- {when} · Folio {}\n{message}\nat {location}\n", env!("CARGO_PKG_VERSION"));
    if !stack.trim().is_empty() {
        s.push_str(stack.trim_end());
        s.push('\n');
    }
    s
}

/// Adds an entry to the crash log, keeping only the newest 64 KB so the file
/// never grows without bound.
pub fn log_crash(dir: &Path, entry: &str) -> std::io::Result<()> {
    const KEEP: usize = 64 * 1024;
    std::fs::create_dir_all(dir)?;
    let path = crash_file(dir);
    let mut text = std::fs::read_to_string(&path).unwrap_or_default();
    text.push_str(entry);
    if text.len() > KEEP {
        let mut cut = text.len() - KEEP;
        while !text.is_char_boundary(cut) {
            cut += 1;
        }
        // Start at a whole entry.
        let from = text[cut..].find("--- ").map_or(cut, |i| cut + i);
        text = text[from..].to_string();
    }
    std::fs::write(path, text)
}

#[cfg(test)]
mod crash_tests {
    use super::*;

    #[test]
    fn crash_log_keeps_whole_recent_entries() {
        let dir = std::env::temp_dir().join(format!("folio-crash-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let entry = crash_entry("2026-10-26 12:00", "src/x.rs:1:1", "boom", "");
        assert!(entry.starts_with("--- 2026-10-26 12:00 · Folio "));
        assert!(entry.contains("boom\nat src/x.rs:1:1\n"));
        for _ in 0..2000 {
            log_crash(&dir, &entry).unwrap();
        }
        let text = std::fs::read_to_string(crash_file(&dir)).unwrap();
        assert!(text.len() <= 64 * 1024);
        assert!(text.starts_with("--- "));
        let _ = std::fs::remove_dir_all(&dir);
    }
}
