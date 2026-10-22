//! Google Drive, from the app's side: signing in and out, syncing a few
//! seconds after each save and once a minute, and merging when notes
//! changed both here and in Drive. The talking to Google is `cloud.rs`;
//! the merging is `merge.rs`.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use gpui::{Context, Window};

use super::app::{Dialog, Folio, Gesture, Screen};
use crate::cloud::{self, ClientId, Drive, Endpoints, Keep, Outcome};
use crate::merge::{self, Report};
use crate::model::Board;
use crate::store::{self, Side};
use crate::time;

/// Where Google Drive sync is at, for the Sync screen and the top bar.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub enum DriveState {
    /// Not signed in.
    #[default]
    Off,
    /// Waiting for the browser.
    SigningIn,
    Idle,
    Syncing,
    /// The first sync, and both sides have notes: waiting for the person.
    Choice,
    Error(String),
}

/// What the person picks the first time both sides have notes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Pick {
    /// Both sets of notes, merged.
    Merge,
    /// Only this computer's; Drive's go to Backups.
    Mine,
    /// Only Drive's; this computer's go to Backups.
    Drive,
}

/// Sync shortly after a save, and at least this often.
const AFTER_SAVE: Duration = Duration::from_secs(4);
const EVERY: u64 = 60;
/// Lines kept in the activity list.
const LOG: usize = 6;

#[derive(Default)]
pub struct DriveSync {
    pub state: DriveState,
    pub email: Option<String>,
    pub synced_at: Option<u64>,
    /// What happened lately, newest first: (Unix seconds, what).
    pub log: Vec<(u64, String)>,
    /// Drive's board while the person chooses.
    pending: Option<String>,
    drive: Option<Arc<Mutex<Drive>>>,
    cancel: Arc<AtomicBool>,
    soon: Option<gpui::Task<()>>,
    /// A save landed during a sync: go again after.
    again: bool,
    /// What the sync under way was asked to do, for the log.
    forced: Option<Keep>,
    /// The sync under way sends a merge's result.
    after_merge: bool,
}

impl DriveSync {
    pub fn signed_in(&self) -> bool {
        self.drive.is_some()
    }

    fn note(&mut self, what: impl Into<String>) {
        let what = what.into();
        if self.log.first().is_some_and(|(_, w)| *w == what) {
            self.log[0].0 = time::now();
            return;
        }
        self.log.insert(0, (time::now(), what));
        self.log.truncate(LOG);
    }
}

fn plural(n: usize, one: &str) -> String {
    if n == 1 {
        format!("1 {one}")
    } else {
        format!("{n} {one}s")
    }
}

/// A merge's report in a few words.
pub fn describe(r: &Report) -> String {
    let mut parts = Vec::new();
    if r.from_drive > 0 {
        parts.push(format!("{} from Drive", plural(r.from_drive, "change")));
    }
    if r.from_here > 0 {
        parts.push(format!("{} from here", r.from_here));
    }
    if r.clashes > 0 {
        parts.push(format!("{} edited in both, newest kept", r.clashes));
    }
    if parts.is_empty() {
        "nothing to change".into()
    } else {
        parts.join(" · ")
    }
}

impl Folio {
    /// At start-up: picks up a saved sign-in and syncs.
    pub fn drive_start(&mut self, cx: &mut Context<Self>) {
        let (Some(client), Some(link)) = (ClientId::configured(), cloud::load_link(&self.dir)) else { return };
        self.drive_attach(client, link, cx);
    }

    fn drive_attach(&mut self, client: ClientId, link: cloud::Link, cx: &mut Context<Self>) {
        let d = &mut self.sync;
        d.email = Some(link.email.clone());
        d.synced_at = link.synced_at;
        if d.log.is_empty() {
            d.log = link.log.clone();
        }
        d.drive = Some(Arc::new(Mutex::new(Drive::new(client, Endpoints::default(), link))));
        d.state = DriveState::Idle;
        self.drive_sync(None, cx);
    }

    pub fn sign_in_google(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.sync.signed_in() || self.sync.state == DriveState::SigningIn {
            return;
        }
        let Some(client) = ClientId::configured() else {
            // Not set up yet: the setup, where the app's Google client is pasted.
            self.dialog = Some(Dialog::GoogleSetup);
            self.client_input.update(cx, |i, cx| i.set_value("", window, cx));
            self.secret_input.update(cx, |i, cx| i.set_value("", window, cx));
            let input = self.client_input.clone();
            window.defer(cx, move |window, cx| input.update(cx, |i, cx| i.focus(window, cx)));
            cx.notify();
            return;
        };
        let ep = Endpoints::default();
        let started = match cloud::begin_sign_in(&client, &ep) {
            Ok(s) => s,
            Err(e) => {
                self.notify(format!("Couldn't start signing in: {e}"), cx);
                return;
            }
        };
        cx.open_url(&started.url);
        self.sync.state = DriveState::SigningIn;
        let cancel = Arc::new(AtomicBool::new(false));
        self.sync.cancel = cancel.clone();
        let c2 = client.clone();
        let job = cx.background_executor().spawn(async move { cloud::finish_sign_in(started, &c2, &ep, &|| cancel.load(Ordering::Relaxed)) });
        cx.spawn(async move |this, cx| {
            let result = job.await;
            let _ = this.update(cx, |this, cx| {
                if this.sync.state != DriveState::SigningIn {
                    return;
                }
                match result {
                    Ok(link) => {
                        if let Err(e) = cloud::save_link(&this.dir, &link) {
                            this.sync.state = DriveState::Off;
                            this.notify(format!("Couldn't keep the sign-in: {e}"), cx);
                            return;
                        }
                        // A new account starts from nothing shared.
                        store::forget_sync_base(&this.dir);
                        this.notify(format!("Signed in as {}", link.email), cx);
                        this.sync.note(format!("Signed in as {}", link.email));
                        this.drive_attach(client, link, cx);
                    }
                    Err(e) => {
                        this.sync.state = DriveState::Off;
                        this.notify(e, cx);
                    }
                }
                cx.notify();
            });
        })
        .detach();
        cx.notify();
    }

    pub fn cancel_sign_in(&mut self, cx: &mut Context<Self>) {
        self.sync.cancel.store(true, Ordering::Relaxed);
        if self.sync.state == DriveState::SigningIn {
            self.sync.state = DriveState::Off;
        }
        cx.notify();
    }

    pub fn sign_out_google(&mut self, cx: &mut Context<Self>) {
        let Some(drive) = self.sync.drive.take() else { return };
        cloud::forget_link(&self.dir);
        store::forget_sync_base(&self.dir);
        // Revoked at Google off the UI thread, once any sync under way is done.
        cx.background_executor()
            .spawn(async move {
                if let Ok(d) = drive.lock() {
                    cloud::revoke(&d.link, &d.ep);
                }
            })
            .detach();
        self.sync = DriveSync::default();
        if matches!(self.dialog, Some(Dialog::DriveChoice(_) | Dialog::DriveReplace(_))) {
            self.dialog = None;
        }
        self.notify("Signed out of Google · your notes stay on this computer", cx);
    }

    /// After each save: sync in a few seconds.
    pub fn drive_after_save(&mut self, cx: &mut Context<Self>) {
        if !self.sync.signed_in() || self.sync.state == DriveState::Choice {
            return;
        }
        self.sync.soon = Some(cx.spawn(async move |this, cx| {
            cx.background_executor().timer(AFTER_SAVE).await;
            let _ = this.update(cx, |this, cx| this.drive_sync(None, cx));
        }));
    }

    /// Every 15 seconds: sync if it's been a minute.
    pub fn drive_tick(&mut self, cx: &mut Context<Self>) {
        let due = self.sync.synced_at.is_none_or(|t| time::now().saturating_sub(t) >= EVERY);
        if due && matches!(self.sync.state, DriveState::Idle | DriveState::Error(_)) {
            self.drive_sync(None, cx);
        }
    }

    /// The board's text, as it's saved here and sent to Drive.
    fn board_text(&self) -> String {
        serde_json::to_string(&self.board).unwrap_or_default()
    }

    pub fn drive_sync(&mut self, force: Option<Keep>, cx: &mut Context<Self>) {
        let Some(drive) = self.sync.drive.clone() else { return };
        if self.sync.state == DriveState::Syncing {
            self.sync.again = true;
            return;
        }
        self.sync.soon = None;
        self.sync.state = DriveState::Syncing;
        self.sync.forced = force;
        let text = self.board_text();
        let media = store::media_dir(&self.dir);
        let sent = text.clone();
        let job = cx.background_executor().spawn(async move {
            let mut d = drive.lock().map_err(|_| "Sync stopped".to_string())?;
            d.sync(&sent, &media, force)
        });
        cx.spawn(async move |this, cx| {
            let result = job.await;
            let _ = this.update(cx, |this, cx| this.drive_done(result, text, cx));
        })
        .detach();
        cx.notify();
    }

    fn drive_done(&mut self, outcome: Result<Outcome, String>, sent: String, cx: &mut Context<Self>) {
        if !self.sync.signed_in() {
            return;
        }
        self.sync.state = DriveState::Idle;
        let forced = self.sync.forced.take();
        let after_merge = std::mem::take(&mut self.sync.after_merge);
        let had_base = store::has_sync_base(&self.dir);
        match outcome {
            Ok(Outcome::Same) => {
                if !had_base {
                    let _ = store::save_sync_base(&self.dir, &sent);
                }
                if forced.is_some() && !after_merge {
                    self.notify("Already the same as Google Drive · nothing to change", cx);
                }
            }
            Ok(Outcome::Pushed) => {
                let _ = store::save_sync_base(&self.dir, &sent);
                if after_merge {
                    // The merge already said what happened.
                } else if forced == Some(Keep::ThisComputer) {
                    self.sync.note("Sent this computer's notes to Drive");
                } else if !had_base {
                    self.sync.note(format!("Put {} in Google Drive", plural(self.board.categories.len(), "sheet")));
                }
            }
            Ok(Outcome::Pulled(..)) if self.before.is_some() || !matches!(self.gesture, Gesture::None) => {
                // Mid-drag: Drive's comes down again once it's done.
                self.sync.again = true;
            }
            Ok(Outcome::Pulled(remote, rev)) => {
                if self.board_text() != sent || self.editing.is_some() {
                    // Edited here while Drive's came down: merge the two.
                    self.drive_merge(remote, cx);
                } else {
                    match self.drive_apply(&remote, rev, cx) {
                        Ok(()) => {
                            let _ = store::save_sync_base(&self.dir, &remote);
                            self.sync.note(if forced == Some(Keep::Drive) { "Brought Google Drive's notes here" } else { "Got the latest from Google Drive" });
                        }
                        Err(e) => self.sync.state = DriveState::Error(e),
                    }
                }
            }
            Ok(Outcome::Both(remote)) => {
                if had_base {
                    self.drive_merge(remote, cx);
                } else {
                    // Never synced from here, and both have notes: ask.
                    self.sync.pending = Some(remote);
                    self.sync.state = DriveState::Choice;
                    if self.dialog.is_none() {
                        self.dialog = Some(Dialog::DriveChoice(Pick::Merge));
                    }
                }
            }
            Err(e) => {
                if e == cloud::SIGN_IN_AGAIN {
                    // The sign-in was revoked or expired: back to signed out.
                    cloud::forget_link(&self.dir);
                    self.sync = DriveSync::default();
                    self.notify("Google asks you to sign in again · Sync & export", cx);
                    return;
                }
                self.sync.note(e.clone());
                self.sync.state = DriveState::Error(e);
            }
        }
        // The sync is over, so the lock is free.
        if let Some(Ok(mut d)) = self.sync.drive.as_ref().map(|d| d.try_lock()) {
            d.link.log = self.sync.log.clone();
            self.sync.synced_at = d.link.synced_at.or(self.sync.synced_at);
            let _ = cloud::save_link(&self.dir, &d.link);
        }
        if std::mem::take(&mut self.sync.again) && self.sync.state == DriveState::Idle {
            self.drive_after_save(cx);
        }
        cx.notify();
    }

    /// Both sides changed: merges Drive's into this computer's (one undo
    /// step), then sends the result back.
    fn drive_merge(&mut self, remote: String, cx: &mut Context<Self>) {
        if self.before.is_some() || !matches!(self.gesture, Gesture::None) {
            self.sync.again = true;
            return;
        }
        let Ok(mut theirs) = serde_json::from_str::<Board>(&remote) else {
            self.sync.state = DriveState::Error("Drive's board couldn't be read".into());
            return;
        };
        theirs.normalize();
        self.finish_edit(cx);
        let base = store::load_sync_base(&self.dir);
        let first = base.is_none();
        let (merged, report) = merge::merge(base.as_ref(), &self.board, &theirs);
        if report.clashes > 0 || first {
            // Something was edited in both places: this computer's version
            // stays restorable as it was.
            let _ = store::save_copy(&self.dir, &self.board_text(), &time::today(), &time::hhmm(), Side::ThisComputer);
        }
        self.drive_put(merged);
        self.save_now(cx);
        self.desktop_sync(cx);
        if !report.is_empty() {
            self.sync.note(format!("Merged with Drive: {}", describe(&report)));
        }
        if report.from_drive > 0 || first {
            self.notify(format!("Merged with Google Drive · {}", describe(&report)), cx);
        }
        self.sync.state = DriveState::Idle;
        // The merged board goes up; the base is set once it's there.
        self.drive_sync(Some(Keep::ThisComputer), cx);
        self.sync.after_merge = true;
    }

    /// Puts a board in place as one undo step, keeping this computer's
    /// settings and view.
    fn drive_put(&mut self, mut b: Board) {
        b.settings = self.board.settings.clone();
        b.at_home = self.board.at_home;
        b.active = b.active.min(b.categories.len().saturating_sub(1));
        let before = std::mem::replace(&mut self.board, b);
        self.undo.push(before);
        self.redo.clear();
        self.selection.clear();
        if self.board.categories.is_empty() {
            self.screen = Screen::Home;
        }
    }

    /// The first-sync choice.
    pub fn drive_pick(&mut self, pick: Pick, window: &mut Window, cx: &mut Context<Self>) {
        self.dialog = None;
        window.focus(&self.focus, cx);
        let Some(remote) = self.sync.pending.take() else { return };
        self.sync.state = DriveState::Idle;
        match pick {
            Pick::Merge => self.drive_merge(remote, cx),
            Pick::Mine => {
                if let Err(e) = store::save_copy(&self.dir, &remote, &time::today(), &time::hhmm(), Side::Drive) {
                    self.notify(format!("Couldn't keep a copy, so nothing changed: {e}"), cx);
                    return;
                }
                self.drive_sync(Some(Keep::ThisComputer), cx);
                self.notify("Google Drive's notes are kept in Backups", cx);
            }
            Pick::Drive => {
                self.finish_edit(cx);
                if let Err(e) = store::save_copy(&self.dir, &self.board_text(), &time::today(), &time::hhmm(), Side::ThisComputer) {
                    self.notify(format!("Couldn't keep a copy, so nothing changed: {e}"), cx);
                    return;
                }
                self.save_now(cx);
                self.drive_sync(Some(Keep::Drive), cx);
                self.notify("This computer's notes are kept in Backups", cx);
            }
        }
    }

    /// Opens the choice again (from the Sync screen).
    pub fn drive_show_choice(&mut self, cx: &mut Context<Self>) {
        if self.sync.state == DriveState::Choice {
            self.dialog = Some(Dialog::DriveChoice(Pick::Merge));
            cx.notify();
        }
    }

    /// "Replace Drive with this computer's" or the other way, after the
    /// person confirmed. This computer's notes are backed up first; Drive
    /// keeps its own earlier versions.
    pub fn drive_replace(&mut self, keep: Keep, window: &mut Window, cx: &mut Context<Self>) {
        self.dialog = None;
        window.focus(&self.focus, cx);
        self.finish_edit(cx);
        if keep == Keep::Drive {
            if let Err(e) = store::save_copy(&self.dir, &self.board_text(), &time::today(), &time::hhmm(), Side::ThisComputer) {
                self.notify(format!("Couldn't keep a copy, so nothing changed: {e}"), cx);
                return;
            }
        }
        self.save_now(cx);
        self.sync.pending = None;
        self.sync.state = DriveState::Idle;
        self.drive_sync(Some(keep), cx);
    }

    /// Puts Drive's board in place (undoable).
    fn drive_apply(&mut self, remote: &str, rev: Option<String>, cx: &mut Context<Self>) -> Result<(), String> {
        let mut b: Board = serde_json::from_str(remote).map_err(|_| "Drive's board couldn't be read".to_string())?;
        b.normalize();
        let mut same = b.clone();
        same.settings = self.board.settings.clone();
        same.at_home = self.board.at_home;
        same.active = same.active.min(same.categories.len().saturating_sub(1));
        if same != self.board {
            self.drive_put(b);
            self.save_now(cx);
            self.desktop_sync(cx);
            self.notify("Updated from Google Drive", cx);
        }
        let text = self.board_text();
        if let Some(drive) = &self.sync.drive {
            // Not syncing now (this runs as a sync finishes), so it's free.
            if let Ok(mut d) = drive.try_lock() {
                d.pulled(rev, &text);
            }
        }
        self.sync.soon = None;
        Ok(())
    }

    /// On quit: a last push if this computer has changes Drive doesn't,
    /// waiting a few seconds at most.
    pub fn drive_flush(&mut self) {
        if self.sync.state != DriveState::Idle {
            return;
        }
        let Some(drive) = self.sync.drive.clone() else { return };
        let text = self.board_text();
        let media = store::media_dir(&self.dir);
        let dir = self.dir.clone();
        let (tx, rx) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            if let Ok(mut d) = drive.try_lock() {
                if d.link.synced_hash.as_deref() != Some(cloud::hash(&text).as_str()) {
                    if let Ok(Outcome::Pushed) = d.sync(&text, &media, None) {
                        let _ = cloud::save_link(&dir, &d.link);
                        let _ = store::save_sync_base(&dir, &text);
                    }
                }
            }
            let _ = tx.send(());
        });
        let _ = rx.recv_timeout(Duration::from_secs(8));
    }

    /// One line on where sync is.
    pub fn drive_status(&self) -> String {
        match &self.sync.state {
            DriveState::Off => "Not connected".into(),
            DriveState::SigningIn => "Finish signing in in your browser…".into(),
            DriveState::Syncing => "Syncing…".into(),
            DriveState::Choice => "Waiting for you: both have notes".into(),
            DriveState::Error(e) => e.clone(),
            DriveState::Idle => match self.sync.synced_at {
                Some(t) => format!("Up to date · synced {}", time::ago(t, time::now()).to_lowercase()),
                None => "Up to date".into(),
            },
        }
    }
}
