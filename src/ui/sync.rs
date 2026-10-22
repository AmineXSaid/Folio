//! Google Drive, from the app's side: signing in and out, syncing a few
//! seconds after each save and once a minute, and the choice when notes
//! changed both here and in Drive. The talking to Google is `cloud.rs`.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use gpui::{Context, Window};

use super::app::{Dialog, Folio, Gesture, Screen};
use crate::cloud::{self, ClientId, Drive, Endpoints, Keep, Outcome};
use crate::model::Board;
use crate::store::{self, Side};
use crate::time;

/// Where Google Drive sync is at, for Settings and the top bar.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub enum DriveState {
    /// Not signed in.
    #[default]
    Off,
    /// Waiting for the browser.
    SigningIn,
    Idle,
    Syncing,
    /// Notes changed in both places: waiting for the person to pick.
    Choice,
    Error(String),
}

/// Sync shortly after a save, and at least this often.
const AFTER_SAVE: Duration = Duration::from_secs(4);
const EVERY: u64 = 60;

#[derive(Default)]
pub struct DriveSync {
    pub state: DriveState,
    pub email: Option<String>,
    pub synced_at: Option<u64>,
    /// The first sync from this computer (the choice reads differently).
    pub first: bool,
    /// Drive's board while the person chooses.
    pending: Option<String>,
    drive: Option<Arc<Mutex<Drive>>>,
    cancel: Arc<AtomicBool>,
    soon: Option<gpui::Task<()>>,
    /// A save landed during a sync: go again after.
    again: bool,
}

impl DriveSync {
    pub fn signed_in(&self) -> bool {
        self.drive.is_some()
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
        d.first = link.synced_hash.is_none();
        d.drive = Some(Arc::new(Mutex::new(Drive::new(client, Endpoints::default(), link))));
        d.state = DriveState::Idle;
        self.drive_sync(None, cx);
    }

    pub fn sign_in_google(&mut self, cx: &mut Context<Self>) {
        if self.sync.signed_in() || self.sync.state == DriveState::SigningIn {
            return;
        }
        let Some(client) = ClientId::configured() else {
            self.notify("Google sign-in isn't set up in this build of Folio", cx);
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
                        this.notify(format!("Signed in as {}", link.email), cx);
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
        // Revoked at Google off the UI thread, once any sync under way is done.
        cx.background_executor()
            .spawn(async move {
                if let Ok(d) = drive.lock() {
                    cloud::revoke(&d.link, &d.ep);
                }
            })
            .detach();
        self.sync = DriveSync::default();
        if matches!(self.dialog, Some(Dialog::DriveChoice)) {
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
        match outcome {
            Ok(Outcome::Same) | Ok(Outcome::Pushed) => {}
            Ok(Outcome::Pulled(..)) if self.before.is_some() || !matches!(self.gesture, Gesture::None) => {
                // Mid-drag: Drive's comes down again once it's done.
                self.sync.again = true;
            }
            Ok(Outcome::Pulled(remote, rev)) => {
                if self.board_text() != sent || self.editing.is_some() {
                    // Edited here while Drive's came down: both changed.
                    self.drive_ask(remote, cx);
                } else if let Err(e) = self.drive_apply(&remote, rev, cx) {
                    self.sync.state = DriveState::Error(e);
                }
            }
            Ok(Outcome::Both(remote)) => self.drive_ask(remote, cx),
            Err(e) => {
                if e == cloud::SIGN_IN_AGAIN {
                    // The sign-in was revoked or expired: back to signed out.
                    cloud::forget_link(&self.dir);
                    self.sync = DriveSync::default();
                    self.notify("Google asks you to sign in again · Settings › Your data", cx);
                    return;
                }
                self.sync.state = DriveState::Error(e);
            }
        }
        // The sync is over, so the lock is free.
        if let Some(Ok(d)) = self.sync.drive.as_ref().map(|d| d.try_lock()) {
            self.sync.first = d.link.synced_hash.is_none();
            self.sync.synced_at = d.link.synced_at.or(self.sync.synced_at);
            let _ = cloud::save_link(&self.dir, &d.link);
        }
        if std::mem::take(&mut self.sync.again) && self.sync.state == DriveState::Idle {
            self.drive_after_save(cx);
        }
        cx.notify();
    }

    fn drive_ask(&mut self, remote: String, cx: &mut Context<Self>) {
        self.sync.pending = Some(remote);
        self.sync.state = DriveState::Choice;
        if self.dialog.is_none() {
            self.dialog = Some(Dialog::DriveChoice);
        }
        cx.notify();
    }

    /// Opens the choice again (from Settings).
    pub fn drive_show_choice(&mut self, cx: &mut Context<Self>) {
        if self.sync.state == DriveState::Choice {
            self.dialog = Some(Dialog::DriveChoice);
            cx.notify();
        }
    }

    /// Settles "both changed": the other side is kept in Backups first.
    pub fn drive_keep(&mut self, keep: Keep, window: &mut Window, cx: &mut Context<Self>) {
        self.dialog = None;
        window.focus(&self.focus, cx);
        let (text, side) = match keep {
            Keep::ThisComputer => (self.sync.pending.take().unwrap_or_default(), Side::Drive),
            Keep::Drive => {
                self.finish_edit(cx);
                (self.board_text(), Side::ThisComputer)
            }
        };
        if let Err(e) = store::save_copy(&self.dir, &text, &time::today(), &time::hhmm(), side) {
            self.notify(format!("Couldn't keep a copy, so nothing changed: {e}"), cx);
            return;
        }
        self.sync.pending = None;
        self.sync.state = DriveState::Idle;
        self.save_now(cx);
        self.drive_sync(Some(keep), cx);
        let other = if side == Side::Drive { "Google Drive's" } else { "This computer's" };
        self.notify(format!("{other} notes are kept in Backups"), cx);
    }

    /// Puts Drive's board in place (undoable). This computer keeps its own
    /// settings: theme, start-up and shortcut are per computer.
    fn drive_apply(&mut self, remote: &str, rev: Option<String>, cx: &mut Context<Self>) -> Result<(), String> {
        let mut b: Board = serde_json::from_str(remote).map_err(|_| "Drive's board couldn't be read".to_string())?;
        b.normalize();
        b.settings = self.board.settings.clone();
        b.at_home = self.board.at_home;
        b.active = b.active.min(b.categories.len().saturating_sub(1));
        if b == self.board {
            self.drive_mark_pulled(rev, cx);
            return Ok(());
        }
        let before = std::mem::replace(&mut self.board, b);
        self.undo.push(before);
        self.redo.clear();
        self.selection.clear();
        if self.board.categories.is_empty() {
            self.screen = Screen::Home;
        }
        self.save_now(cx);
        self.desktop_sync(cx);
        self.drive_mark_pulled(rev, cx);
        self.notify("Updated from Google Drive", cx);
        Ok(())
    }

    fn drive_mark_pulled(&mut self, rev: Option<String>, cx: &mut Context<Self>) {
        let text = self.board_text();
        if let Some(drive) = &self.sync.drive {
            // Not syncing now (this runs as a sync finishes), so it's free.
            if let Ok(mut d) = drive.try_lock() {
                d.pulled(rev, &text);
            }
        }
        self.sync.soon = None;
        cx.notify();
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
                    }
                }
            }
            let _ = tx.send(());
        });
        let _ = rx.recv_timeout(Duration::from_secs(8));
    }

    /// One line on where sync is, for Settings.
    pub fn drive_status(&self) -> String {
        let email = self.sync.email.clone().unwrap_or_default();
        match &self.sync.state {
            DriveState::Off => "Keep your notes in your Google Drive and on every computer you sign in on".into(),
            DriveState::SigningIn => "Finish signing in in your browser…".into(),
            DriveState::Syncing => format!("{email} · syncing…"),
            DriveState::Choice => format!("{email} · your notes changed here and in Drive: choose which to keep"),
            DriveState::Error(e) => format!("{email} · {e}"),
            DriveState::Idle => match self.sync.synced_at {
                Some(t) => format!("{email} · synced {}", time::ago(t, time::now()).to_lowercase()),
                None => email,
            },
        }
    }
}
