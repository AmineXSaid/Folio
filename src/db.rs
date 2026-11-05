//! The board on disk, in SQLite (`folio.db` in the data folder): one row
//! per sheet and per thing on a sheet (note, line of ink, text, link,
//! area, shape), each its JSON, plus one row for the rest of the board.
//! A save writes only the rows that changed since the last one, in one
//! transaction (WAL), so a crash leaves the last save whole. A full-text
//! index (FTS5) over notes and text finds words across every sheet.
//! Nothing here knows about GPUI.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use rusqlite::{params, Connection, OptionalExtension};

use crate::model::{Board, Category, Id};

pub fn db_file(dir: &Path) -> PathBuf {
    dir.join("folio.db")
}

/// What was last written, row by row: (kind, id) → (sheet, place, JSON).
pub type Rows = HashMap<(&'static str, Id), (Id, usize, String)>;

/// The kinds of thing on a sheet, as the rows name them.
const KINDS: [&str; 6] = ["note", "stroke", "label", "link", "frame", "shape"];

pub fn open(dir: &Path) -> rusqlite::Result<Connection> {
    std::fs::create_dir_all(dir).map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))?;
    let conn = Connection::open(db_file(dir))?;
    conn.execute_batch(
        "PRAGMA journal_mode = WAL;
         PRAGMA synchronous = NORMAL;
         CREATE TABLE IF NOT EXISTS board (key TEXT PRIMARY KEY, json TEXT NOT NULL);
         CREATE TABLE IF NOT EXISTS sheets (id INTEGER PRIMARY KEY, pos INTEGER NOT NULL, json TEXT NOT NULL);
         CREATE TABLE IF NOT EXISTS items (
             kind TEXT NOT NULL, id INTEGER NOT NULL, sheet INTEGER NOT NULL, pos INTEGER NOT NULL, json TEXT NOT NULL,
             PRIMARY KEY (kind, id));
         CREATE INDEX IF NOT EXISTS items_by_sheet ON items (sheet, kind, pos);
         CREATE VIRTUAL TABLE IF NOT EXISTS words USING fts5 (kind UNINDEXED, id UNINDEXED, sheet UNINDEXED, title, body);",
    )?;
    Ok(conn)
}

/// Every row `board` makes: its sheets (without what's on them), each thing
/// on them, and the rest of the board.
fn rows(board: &Board) -> (String, Vec<(Id, String)>, Rows, Vec<(&'static str, Id, Id, String, String)>) {
    let mut rest = board.clone();
    rest.categories = Vec::new();
    let head = serde_json::to_string(&rest).unwrap_or_default();
    let mut sheets = Vec::new();
    let mut items: Rows = HashMap::new();
    let mut words = Vec::new();
    for c in &board.categories {
        let bare = Category {
            stickers: Vec::new(),
            strokes: Vec::new(),
            labels: Vec::new(),
            links: Vec::new(),
            frames: Vec::new(),
            shapes: Vec::new(),
            ..c.clone()
        };
        sheets.push((c.id, serde_json::to_string(&bare).unwrap_or_default()));
        let mut put = |kind: &'static str, id: Id, pos: usize, json: String| {
            items.insert((kind, id), (c.id, pos, json));
        };
        for (i, s) in c.stickers.iter().enumerate() {
            put("note", s.id, i, serde_json::to_string(s).unwrap_or_default());
            words.push(("note", s.id, c.id, s.title.clone(), format!("{}\n{}", s.body, s.caption)));
        }
        for (i, s) in c.strokes.iter().enumerate() {
            put("stroke", s.id, i, serde_json::to_string(s).unwrap_or_default());
        }
        for (i, l) in c.labels.iter().enumerate() {
            put("label", l.id, i, serde_json::to_string(l).unwrap_or_default());
            words.push(("label", l.id, c.id, String::new(), l.text.clone()));
        }
        for (i, l) in c.links.iter().enumerate() {
            put("link", l.id, i, serde_json::to_string(l).unwrap_or_default());
        }
        for (i, f) in c.frames.iter().enumerate() {
            put("frame", f.id, i, serde_json::to_string(f).unwrap_or_default());
        }
        for (i, s) in c.shapes.iter().enumerate() {
            put("shape", s.id, i, serde_json::to_string(s).unwrap_or_default());
            words.push(("shape", s.id, c.id, String::new(), s.text.clone()));
        }
    }
    (head, sheets, items, words)
}

/// Writes `board`, only what differs from `last` (all of it when `last` is
/// empty), in one transaction; `last` becomes what's now on the disk.
/// Returns how many thing rows were written or removed.
pub fn save(conn: &mut Connection, board: &Board, last: &mut Rows) -> rusqlite::Result<usize> {
    let (head, sheets, items, words) = rows(board);
    let fresh = last.is_empty();
    let tx = conn.transaction()?;
    tx.execute("INSERT INTO board (key, json) VALUES ('board', ?1) ON CONFLICT(key) DO UPDATE SET json = excluded.json", params![head])?;
    // Sheets are few: written whole each time.
    tx.execute("DELETE FROM sheets", [])?;
    for (pos, (id, json)) in sheets.iter().enumerate() {
        tx.execute("INSERT INTO sheets (id, pos, json) VALUES (?1, ?2, ?3)", params![*id as i64, pos as i64, json])?;
    }
    if fresh {
        tx.execute("DELETE FROM items", [])?;
    }
    let mut changed = 0;
    for ((kind, id), row) in &items {
        if last.get(&(*kind, *id)) != Some(row) {
            let (sheet, pos, json) = row;
            tx.execute(
                "INSERT INTO items (kind, id, sheet, pos, json) VALUES (?1, ?2, ?3, ?4, ?5)
                 ON CONFLICT(kind, id) DO UPDATE SET sheet = excluded.sheet, pos = excluded.pos, json = excluded.json",
                params![kind, *id as i64, *sheet as i64, *pos as i64, json],
            )?;
            changed += 1;
        }
    }
    for (kind, id) in last.keys() {
        if !items.contains_key(&(*kind, *id)) {
            tx.execute("DELETE FROM items WHERE kind = ?1 AND id = ?2", params![kind, *id as i64])?;
            changed += 1;
        }
    }
    // The word index follows the words (rebuilt only when something changed).
    if changed > 0 || fresh {
        tx.execute("DELETE FROM words", [])?;
        for (kind, id, sheet, title, body) in &words {
            tx.execute("INSERT INTO words (kind, id, sheet, title, body) VALUES (?1, ?2, ?3, ?4, ?5)", params![kind, *id as i64, *sheet as i64, title, body])?;
        }
    }
    tx.commit()?;
    *last = items;
    Ok(changed)
}

/// The board as last saved, and its rows (for the next save to compare
/// with); None when nothing was ever saved here.
pub fn load(conn: &Connection) -> rusqlite::Result<Option<(Board, Rows)>> {
    let Some(head): Option<String> = conn.query_row("SELECT json FROM board WHERE key = 'board'", [], |r| r.get(0)).optional()? else {
        return Ok(None);
    };
    let Ok(mut board) = serde_json::from_str::<Board>(&head) else { return Ok(None) };
    let mut q = conn.prepare("SELECT json FROM sheets ORDER BY pos")?;
    let sheets: Vec<String> = q.query_map([], |r| r.get(0))?.collect::<Result<_, _>>()?;
    for json in sheets {
        if let Ok(c) = serde_json::from_str::<Category>(&json) {
            board.categories.push(c);
        }
    }
    let at: HashMap<Id, usize> = board.categories.iter().enumerate().map(|(i, c)| (c.id, i)).collect();
    let mut rows: Rows = HashMap::new();
    let mut q = conn.prepare("SELECT kind, id, sheet, pos, json FROM items ORDER BY sheet, kind, pos")?;
    let all = q.query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)?, r.get::<_, i64>(2)?, r.get::<_, i64>(3)?, r.get::<_, String>(4)?)))?;
    for row in all {
        let (kind, id, sheet, pos, json) = row?;
        let Some(&ix) = at.get(&(sheet as Id)) else { continue };
        let Some(kind) = KINDS.iter().copied().find(|k| *k == kind) else { continue };
        let c = &mut board.categories[ix];
        let ok = match kind {
            "note" => serde_json::from_str(&json).map(|x| c.stickers.push(x)).is_ok(),
            "stroke" => serde_json::from_str(&json).map(|x| c.strokes.push(x)).is_ok(),
            "label" => serde_json::from_str(&json).map(|x| c.labels.push(x)).is_ok(),
            "link" => serde_json::from_str(&json).map(|x| c.links.push(x)).is_ok(),
            "frame" => serde_json::from_str(&json).map(|x| c.frames.push(x)).is_ok(),
            _ => serde_json::from_str(&json).map(|x| c.shapes.push(x)).is_ok(),
        };
        if ok {
            rows.insert((kind, id as Id), (sheet as Id, pos as usize, json));
        }
    }
    Ok(Some((board, rows)))
}

/// The notes, text and shapes whose words match `query` (each word, as a
/// prefix), best first: (kind, id, sheet).
pub fn search(conn: &Connection, query: &str) -> rusqlite::Result<Vec<(String, Id, Id)>> {
    // Each word quoted (so punctuation can't break the query), as a prefix.
    let q: Vec<String> = query.split_whitespace().map(|w| format!("\"{}\"*", w.replace('"', ""))).collect();
    if q.is_empty() {
        return Ok(Vec::new());
    }
    let mut s = conn.prepare("SELECT kind, id, sheet FROM words WHERE words MATCH ?1 ORDER BY rank LIMIT 200")?;
    let out = s.query_map(params![q.join(" ")], |r| Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)? as Id, r.get::<_, i64>(2)? as Id)))?;
    out.collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dir(tag: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("folio-db-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        d
    }

    #[test]
    fn a_board_comes_back_as_it_was_saved() {
        let d = dir("roundtrip");
        let board = crate::demo::board();
        let mut conn = open(&d).unwrap();
        let mut last = Rows::new();
        let n = save(&mut conn, &board, &mut last).unwrap();
        assert!(n > 10);
        let (back, rows) = load(&conn).unwrap().unwrap();
        assert_eq!(back, board);
        assert_eq!(rows, last);
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn a_save_writes_only_what_changed() {
        let d = dir("incremental");
        let mut board = crate::demo::board();
        let mut conn = open(&d).unwrap();
        let mut last = Rows::new();
        save(&mut conn, &board, &mut last).unwrap();
        // Nothing changed: no rows written.
        assert_eq!(save(&mut conn, &board, &mut last).unwrap(), 0);
        // One note moved, one deleted: two rows.
        board.categories[0].stickers[0].rect.x += 10.;
        let gone = board.categories[0].stickers.pop().unwrap().id;
        assert_eq!(save(&mut conn, &board, &mut last).unwrap(), 2);
        let (back, _) = load(&conn).unwrap().unwrap();
        assert_eq!(back, board);
        assert!(back.find_note(gone).is_none());
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn words_are_found_across_sheets() {
        let d = dir("search");
        let mut board = crate::demo::board();
        board.categories[1].stickers[0].body = "Bring the oscilloscope".into();
        let mut conn = open(&d).unwrap();
        save(&mut conn, &board, &mut Rows::new()).unwrap();
        let hits = search(&conn, "oscillo").unwrap();
        assert_eq!(hits.first().map(|h| (h.0.as_str(), h.1)), Some(("note", board.categories[1].stickers[0].id)));
        assert!(search(&conn, "\"weird) query(").is_ok());
        let _ = std::fs::remove_dir_all(&d);
    }
}
