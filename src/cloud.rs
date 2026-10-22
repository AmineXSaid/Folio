//! Google Drive sync, optional: sign in with Google in the browser, and
//! the board and its pictures are kept in a "Folio" folder in your Drive.
//!
//! Sign-in is OAuth for installed apps: the browser opens Google's page,
//! Google sends the answer back to a one-off address on this computer
//! (127.0.0.1), and PKCE ties it to this sign-in. Folio asks only for
//! `drive.file`, so it sees the files it made and nothing else in your
//! Drive. The sign-in is kept in `google.json` beside the board, never in
//! the board itself (that file goes to Drive).
//!
//! Nothing here knows about GPUI: every call blocks, and the app runs them
//! off the UI thread.

use std::collections::HashSet;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use base64::Engine;
use percent_encoding::{percent_decode_str, utf8_percent_encode, NON_ALPHANUMERIC};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

const SCOPE: &str = "openid email https://www.googleapis.com/auth/drive.file";
const FOLDER: &str = "application/vnd.google-apps.folder";
/// The error that means the sign-in is no good any more.
pub const SIGN_IN_AGAIN: &str = "Google needs you to sign in again";
/// How long the browser has to come back with an answer.
pub const SIGN_IN_WAIT: Duration = Duration::from_secs(300);

/// The app's OAuth client ("Desktop app" in Google Cloud). Set when the
/// .exe is built, or at run time, from `FOLIO_GOOGLE_CLIENT_ID` and
/// `FOLIO_GOOGLE_CLIENT_SECRET`. Google doesn't treat a desktop app's
/// secret as secret; PKCE is what protects the sign-in.
#[derive(Clone, Debug)]
pub struct ClientId {
    pub id: String,
    pub secret: String,
}

impl ClientId {
    pub fn configured() -> Option<ClientId> {
        let get = |name: &str, built: Option<&str>| std::env::var(name).ok().or(built.map(String::from)).filter(|v| !v.trim().is_empty());
        Some(ClientId {
            id: get("FOLIO_GOOGLE_CLIENT_ID", option_env!("FOLIO_GOOGLE_CLIENT_ID"))?,
            secret: get("FOLIO_GOOGLE_CLIENT_SECRET", option_env!("FOLIO_GOOGLE_CLIENT_SECRET")).unwrap_or_default(),
        })
    }
}

/// Where Google's services are (a fake server in the tests).
#[derive(Clone, Debug)]
pub struct Endpoints {
    pub auth: String,
    pub token: String,
    pub revoke: String,
    pub api: String,
    pub upload: String,
}

impl Default for Endpoints {
    fn default() -> Self {
        // Debug builds can be pointed at a local fake of Google, to try the
        // whole flow without an account.
        #[cfg(debug_assertions)]
        if let Ok(base) = std::env::var("FOLIO_GOOGLE_FAKE") {
            return Endpoints {
                auth: format!("{base}/auth"),
                token: format!("{base}/token"),
                revoke: format!("{base}/revoke"),
                api: format!("{base}/api"),
                upload: format!("{base}/upload"),
            };
        }
        Endpoints {
            auth: "https://accounts.google.com/o/oauth2/v2/auth".into(),
            token: "https://oauth2.googleapis.com/token".into(),
            revoke: "https://oauth2.googleapis.com/revoke".into(),
            api: "https://www.googleapis.com/drive/v3".into(),
            upload: "https://www.googleapis.com/upload/drive/v3".into(),
        }
    }
}

/// The signed-in account and where things are in its Drive: `google.json`.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Link {
    pub email: String,
    pub refresh_token: String,
    pub folder_id: Option<String>,
    pub media_folder_id: Option<String>,
    pub file_id: Option<String>,
    /// Drive's revision of board.json when the two last matched.
    pub synced_rev: Option<String>,
    /// The hash of the board as it was then (see [`hash`]).
    pub synced_hash: Option<String>,
    /// When the last sync finished (Unix seconds).
    pub synced_at: Option<u64>,
}

pub fn link_file(dir: &Path) -> PathBuf {
    dir.join("google.json")
}

pub fn load_link(dir: &Path) -> Option<Link> {
    let text = std::fs::read_to_string(link_file(dir)).ok()?;
    serde_json::from_str::<Link>(&text).ok().filter(|l| !l.refresh_token.is_empty())
}

pub fn save_link(dir: &Path, link: &Link) -> std::io::Result<()> {
    std::fs::create_dir_all(dir)?;
    let tmp = dir.join("google.json.tmp");
    std::fs::write(&tmp, serde_json::to_string_pretty(link).map_err(std::io::Error::other)?)?;
    std::fs::rename(tmp, link_file(dir))
}

pub fn forget_link(dir: &Path) {
    let _ = std::fs::remove_file(link_file(dir));
}

/// A board's fingerprint, to tell whether it changed since the last sync.
pub fn hash(text: &str) -> String {
    Sha256::digest(text.as_bytes()).iter().map(|b| format!("{b:02x}")).collect()
}

fn b64(bytes: &[u8]) -> String {
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(bytes)
}

fn random(n: usize) -> String {
    let mut buf = vec![0u8; n];
    getrandom::getrandom(&mut buf).expect("the OS has randomness");
    b64(&buf)
}

fn enc(s: &str) -> String {
    utf8_percent_encode(s, NON_ALPHANUMERIC).to_string()
}

// ---------- Signing in ----------

/// A sign-in under way: the browser page to open, and the local address
/// Google comes back to.
pub struct SignIn {
    pub url: String,
    listener: TcpListener,
    redirect: String,
    verifier: String,
    state: String,
}

pub fn pkce_challenge(verifier: &str) -> String {
    b64(&Sha256::digest(verifier.as_bytes()))
}

pub fn begin_sign_in(client: &ClientId, ep: &Endpoints) -> std::io::Result<SignIn> {
    let listener = TcpListener::bind("127.0.0.1:0")?;
    let redirect = format!("http://127.0.0.1:{}", listener.local_addr()?.port());
    let verifier = random(48);
    let state = random(16);
    let url = format!(
        "{}?client_id={}&redirect_uri={}&response_type=code&scope={}&code_challenge={}&code_challenge_method=S256&state={}&access_type=offline&prompt=consent",
        ep.auth,
        enc(&client.id),
        enc(&redirect),
        enc(SCOPE),
        pkce_challenge(&verifier),
        state
    );
    Ok(SignIn { url, listener, redirect, verifier, state })
}

/// What came back to the local address: `GET /?code=…&state=…`.
pub fn parse_callback(request_line: &str) -> Option<Vec<(String, String)>> {
    let target = request_line.split_whitespace().nth(1)?;
    let query = target.split_once('?')?.1;
    Some(
        query
            .split('&')
            .filter_map(|kv| kv.split_once('='))
            .map(|(k, v)| (k.to_string(), percent_decode_str(&v.replace('+', " ")).decode_utf8_lossy().into_owned()))
            .collect(),
    )
}

const PAGE: &str = "<!doctype html><meta charset=utf-8><title>Folio</title>\
<body style=\"font:16px system-ui,sans-serif;background:#f6f4ef;color:#1d1b20;display:grid;place-items:center;height:90vh\">\
<div style=\"text-align:center\"><h2 style=\"margin:0 0 8px\">{H}</h2><p style=\"color:#6b6870\">{P}</p></div>";

fn answer(stream: &mut std::net::TcpStream, heading: &str, text: &str) {
    let body = PAGE.replace("{H}", heading).replace("{P}", text);
    let _ = write!(stream, "HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len());
}

/// Waits for the browser, then trades its code for the account. `cancel`
/// stops the wait early (it's checked a few times a second).
pub fn finish_sign_in(s: SignIn, client: &ClientId, ep: &Endpoints, cancel: &dyn Fn() -> bool) -> Result<Link, String> {
    s.listener.set_nonblocking(true).map_err(|e| e.to_string())?;
    let deadline = Instant::now() + SIGN_IN_WAIT;
    let code = loop {
        if cancel() {
            return Err("Sign-in cancelled".into());
        }
        if Instant::now() > deadline {
            return Err("Sign-in timed out · try again".into());
        }
        let Ok((mut stream, _)) = s.listener.accept() else {
            std::thread::sleep(Duration::from_millis(150));
            continue;
        };
        let _ = stream.set_nonblocking(false);
        let _ = stream.set_read_timeout(Some(Duration::from_secs(5)));
        let mut buf = [0u8; 8192];
        let n = stream.read(&mut buf).unwrap_or(0);
        let head = String::from_utf8_lossy(&buf[..n]);
        let Some(params) = parse_callback(head.lines().next().unwrap_or("")) else {
            // The browser asking for a favicon, say.
            let _ = write!(stream, "HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n");
            continue;
        };
        let get = |k: &str| params.iter().find(|(n, _)| n == k).map(|(_, v)| v.clone());
        if get("state").as_deref() != Some(s.state.as_str()) {
            answer(&mut stream, "That didn't work", "This answer wasn't for this sign-in. Go back to Folio and try again.");
            return Err("Sign-in didn't match · try again".into());
        }
        if let Some(code) = get("code") {
            answer(&mut stream, "You're signed in", "Folio will keep your notes in Google Drive. You can close this tab.");
            break code;
        }
        answer(&mut stream, "Not signed in", "Google sign-in was cancelled. You can close this tab.");
        return Err("Google sign-in was cancelled".into());
    };

    let resp = agent()
        .post(&ep.token)
        .send_form(&[
            ("code", code.as_str()),
            ("client_id", client.id.as_str()),
            ("client_secret", client.secret.as_str()),
            ("code_verifier", s.verifier.as_str()),
            ("redirect_uri", s.redirect.as_str()),
            ("grant_type", "authorization_code"),
        ])
        .map_err(describe)?;
    let tokens: Tokens = read_json(resp)?;
    let refresh_token = tokens.refresh_token.ok_or("Google gave no lasting sign-in · try again")?;
    let email = tokens.id_token.as_deref().and_then(email_of).unwrap_or_else(|| "your Google account".into());
    Ok(Link { email, refresh_token, ..Default::default() })
}

#[derive(Deserialize)]
struct Tokens {
    access_token: String,
    #[serde(default)]
    expires_in: u64,
    refresh_token: Option<String>,
    id_token: Option<String>,
}

/// The `email` in an ID token. It came straight from Google over TLS, so
/// it's read, not verified.
pub fn email_of(id_token: &str) -> Option<String> {
    let payload = id_token.split('.').nth(1)?;
    let bytes = base64::engine::general_purpose::URL_SAFE_NO_PAD.decode(payload.trim_end_matches('=')).ok()?;
    let v: serde_json::Value = serde_json::from_slice(&bytes).ok()?;
    v.get("email")?.as_str().map(String::from)
}

/// Signs out at Google too (best effort).
pub fn revoke(link: &Link, ep: &Endpoints) {
    let _ = agent().post(&ep.revoke).send_form(&[("token", link.refresh_token.as_str())]);
}

// ---------- Talking to Drive ----------

fn agent() -> ureq::Agent {
    ureq::AgentBuilder::new().timeout(Duration::from_secs(30)).user_agent("Folio").build()
}

fn describe(e: ureq::Error) -> String {
    match e {
        ureq::Error::Status(401, _) => "Google Drive's session ran out · trying again soon".into(),
        ureq::Error::Status(403, r) => {
            let body = r.into_string().unwrap_or_default();
            if body.contains("accessNotConfigured") || body.contains("SERVICE_DISABLED") {
                "The Drive API isn't turned on for this app's Google project".into()
            } else {
                "Google Drive refused · try again later".into()
            }
        }
        ureq::Error::Status(code, _) => format!("Google Drive answered {code} · try again later"),
        ureq::Error::Transport(_) => "Can't reach Google Drive · you're offline?".into(),
    }
}

fn read_json<T: for<'de> Deserialize<'de>>(resp: ureq::Response) -> Result<T, String> {
    let text = resp.into_string().map_err(|e| e.to_string())?;
    serde_json::from_str(&text).map_err(|e| format!("Unexpected answer from Google: {e}"))
}

#[derive(Deserialize)]
struct FileList {
    #[serde(default)]
    files: Vec<DriveFile>,
    #[serde(rename = "nextPageToken")]
    next: Option<String>,
}

#[derive(Clone, Deserialize)]
struct DriveFile {
    id: String,
    #[serde(default)]
    name: String,
    #[serde(rename = "headRevisionId")]
    rev: Option<String>,
}

/// A signed-in connection: the account, and an access token while it lasts.
pub struct Drive {
    pub client: ClientId,
    pub ep: Endpoints,
    pub link: Link,
    access: Option<(String, Instant)>,
}

impl Drive {
    pub fn new(client: ClientId, ep: Endpoints, link: Link) -> Drive {
        Drive { client, ep, link, access: None }
    }

    fn token(&mut self) -> Result<String, String> {
        if let Some((t, until)) = &self.access {
            if Instant::now() < *until {
                return Ok(t.clone());
            }
        }
        let resp = agent()
            .post(&self.ep.token)
            .send_form(&[
                ("client_id", self.client.id.as_str()),
                ("client_secret", self.client.secret.as_str()),
                ("refresh_token", self.link.refresh_token.as_str()),
                ("grant_type", "refresh_token"),
            ])
            .map_err(|e| match e {
                // The sign-in was revoked, or has expired.
                ureq::Error::Status(400 | 401, _) => SIGN_IN_AGAIN.to_string(),
                e => describe(e),
            })?;
        let t: Tokens = read_json(resp)?;
        let life = Duration::from_secs(t.expires_in.max(120) - 60);
        self.access = Some((t.access_token.clone(), Instant::now() + life));
        Ok(t.access_token)
    }

    fn get(&mut self, url: &str) -> Result<ureq::Request, String> {
        Ok(agent().get(url).set("Authorization", &format!("Bearer {}", self.token()?)))
    }

    fn find(&mut self, q: &str) -> Result<Vec<DriveFile>, String> {
        let mut out = Vec::new();
        let mut page: Option<String> = None;
        loop {
            let url = format!("{}/files", self.ep.api);
            let mut req = self.get(&url)?.query("q", q).query("fields", "nextPageToken,files(id,name,headRevisionId)").query("pageSize", "1000").query("spaces", "drive");
            if let Some(p) = &page {
                req = req.query("pageToken", p);
            }
            let list: FileList = read_json(req.call().map_err(describe)?)?;
            out.extend(list.files);
            match list.next {
                Some(n) => page = Some(n),
                None => return Ok(out),
            }
        }
    }

    fn folder(&mut self, name: &str, parent: Option<&str>) -> Result<String, String> {
        let within = parent.map(|p| format!(" and '{p}' in parents")).unwrap_or_default();
        let q = format!("name = '{name}' and mimeType = '{FOLDER}' and trashed = false{within}");
        if let Some(f) = self.find(&q)?.into_iter().next() {
            return Ok(f.id);
        }
        let mut meta = serde_json::json!({ "name": name, "mimeType": FOLDER });
        if let Some(p) = parent {
            meta["parents"] = serde_json::json!([p]);
        }
        let token = self.token()?;
        let resp = agent()
            .post(&format!("{}/files", self.ep.api))
            .set("Authorization", &format!("Bearer {token}"))
            .set("Content-Type", "application/json")
            .send_string(&meta.to_string())
            .map_err(describe)?;
        Ok(read_json::<DriveFile>(resp)?.id)
    }

    /// The Folio folder and its media folder, made the first time.
    fn folders(&mut self) -> Result<(String, String), String> {
        let folder = match self.link.folder_id.clone() {
            Some(f) => f,
            None => self.folder("Folio", None)?,
        };
        let media = match self.link.media_folder_id.clone() {
            Some(m) => m,
            None => self.folder("media", Some(&folder))?,
        };
        self.link.folder_id = Some(folder.clone());
        self.link.media_folder_id = Some(media.clone());
        Ok((folder, media))
    }

    fn download(&mut self, id: &str) -> Result<Vec<u8>, String> {
        let resp = self.get(&format!("{}/files/{id}", self.ep.api))?.query("alt", "media").call().map_err(describe)?;
        let mut bytes = Vec::new();
        resp.into_reader().take(256 << 20).read_to_end(&mut bytes).map_err(|e| e.to_string())?;
        Ok(bytes)
    }

    /// Writes a file: a new one in `parent`, or new content for `id`.
    /// Returns the file's id and revision.
    fn upload(&mut self, id: Option<&str>, name: &str, parent: &str, bytes: &[u8], kind: &str) -> Result<(String, Option<String>), String> {
        let token = self.token()?;
        let resp = match id {
            Some(id) => agent()
                .request("PATCH", &format!("{}/files/{id}", self.ep.upload))
                .query("uploadType", "media")
                .query("fields", "id,name,headRevisionId")
                .set("Authorization", &format!("Bearer {token}"))
                .set("Content-Type", kind)
                .send_bytes(bytes),
            None => {
                let (body, boundary) = multipart(&serde_json::json!({ "name": name, "parents": [parent] }).to_string(), bytes, kind);
                agent()
                    .post(&format!("{}/files", self.ep.upload))
                    .query("uploadType", "multipart")
                    .query("fields", "id,name,headRevisionId")
                    .set("Authorization", &format!("Bearer {token}"))
                    .set("Content-Type", &format!("multipart/related; boundary={boundary}"))
                    .send_bytes(&body)
            }
        }
        .map_err(describe)?;
        let f: DriveFile = read_json(resp)?;
        Ok((f.id, f.rev))
    }
}

/// A Drive upload with its details and its content in one request.
pub fn multipart(meta: &str, bytes: &[u8], kind: &str) -> (Vec<u8>, String) {
    let boundary = format!("folio{}", random(12).replace(['-', '_'], "x"));
    let mut body = format!("--{boundary}\r\nContent-Type: application/json; charset=UTF-8\r\n\r\n{meta}\r\n--{boundary}\r\nContent-Type: {kind}\r\n\r\n").into_bytes();
    body.extend_from_slice(bytes);
    body.extend_from_slice(format!("\r\n--{boundary}--\r\n").as_bytes());
    (body, boundary)
}

// ---------- Syncing ----------

/// What to do, from what changed since the two last matched.
#[derive(Debug, PartialEq, Eq)]
pub enum Plan {
    Nothing,
    Push,
    Pull,
    /// Both changed: the person picks.
    Both,
}

pub fn plan(remote_rev: Option<&str>, local_hash: &str, link: &Link) -> Plan {
    let local_changed = link.synced_hash.as_deref() != Some(local_hash);
    match remote_rev {
        None => Plan::Push,
        Some(r) if link.synced_rev.as_deref() == Some(r) => {
            if local_changed {
                Plan::Push
            } else {
                Plan::Nothing
            }
        }
        // Drive's copy is new to us. Never synced from here before counts
        // as "both changed": this computer may have notes of its own.
        Some(_) if link.synced_hash.is_some() && !local_changed => Plan::Pull,
        Some(_) => Plan::Both,
    }
}

/// How a sync ended.
#[derive(Debug)]
pub enum Outcome {
    /// Already the same.
    Same,
    /// This computer's board is in Drive now.
    Pushed,
    /// Drive had a newer board: here it is, to put in place, and its
    /// revision, for [`Drive::pulled`] once it is.
    Pulled(String, Option<String>),
    /// Both changed: Drive's board, for the person to choose.
    Both(String),
}

/// Which side wins when both changed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Keep {
    ThisComputer,
    Drive,
}

impl Drive {
    /// Brings board.json and the pictures up to date both ways. `board` is
    /// the board's text as saved here; `media` the app's media folder.
    /// `force` settles a "both changed" one way.
    pub fn sync(&mut self, board: &str, media: &Path, force: Option<Keep>) -> Result<Outcome, String> {
        let r = self.sync_once(board, media, force);
        if r.is_err() {
            // Start the next try with a fresh access token.
            self.access = None;
        }
        r
    }

    fn sync_once(&mut self, board: &str, media: &Path, force: Option<Keep>) -> Result<Outcome, String> {
        let (folder, media_folder) = self.folders()?;
        let remote = match self.link.file_id.clone() {
            Some(id) => {
                let found = self.find(&format!("'{folder}' in parents and name = 'board.json' and trashed = false"))?;
                found.into_iter().find(|f| f.id == id)
            }
            None => self.find(&format!("'{folder}' in parents and name = 'board.json' and trashed = false"))?.into_iter().next(),
        };
        self.link.file_id = remote.as_ref().map(|f| f.id.clone());
        let local_hash = hash(board);
        let mut what = plan(remote.as_ref().and_then(|f| f.rev.as_deref()), &local_hash, &self.link);
        if what == Plan::Both {
            match force {
                Some(Keep::ThisComputer) => what = Plan::Push,
                Some(Keep::Drive) => what = Plan::Pull,
                None => {}
            }
        }
        self.sync_media(media, &media_folder)?;
        let outcome = match what {
            Plan::Nothing => Outcome::Same,
            Plan::Push => {
                let (id, rev) = self.upload(self.link.file_id.clone().as_deref(), "board.json", &folder, board.as_bytes(), "application/json")?;
                self.link.file_id = Some(id);
                self.link.synced_rev = rev;
                self.link.synced_hash = Some(local_hash);
                Outcome::Pushed
            }
            Plan::Pull | Plan::Both => {
                let f = remote.expect("a pull has a file");
                let text = String::from_utf8(self.download(&f.id)?).map_err(|_| "Drive's board.json isn't text".to_string())?;
                if what == Plan::Both {
                    return Ok(Outcome::Both(text));
                }
                // Recorded by `pulled`, once the app has it in place.
                Outcome::Pulled(text, f.rev)
            }
        };
        self.link.synced_at = Some(crate::time::now());
        Ok(outcome)
    }

    /// After a pull is in place: Drive's revision, and the board as this
    /// computer now saves it.
    pub fn pulled(&mut self, rev: Option<String>, saved_text: &str) {
        self.link.synced_rev = rev;
        self.link.synced_hash = Some(hash(saved_text));
    }

    /// Pictures go both ways by name (each name is unique).
    fn sync_media(&mut self, media: &Path, folder: &str) -> Result<(), String> {
        let remote = self.find(&format!("'{folder}' in parents and trashed = false"))?;
        let there: HashSet<String> = remote.iter().map(|f| f.name.clone()).collect();
        let here: HashSet<String> = std::fs::read_dir(media)
            .map(|r| r.flatten().filter(|e| e.path().is_file()).filter_map(|e| e.file_name().to_str().map(String::from)).collect())
            .unwrap_or_default();
        for name in here.difference(&there) {
            if name.ends_with(".tmp") {
                continue;
            }
            if let Ok(bytes) = std::fs::read(media.join(name)) {
                self.upload(None, name, folder, &bytes, "application/octet-stream")?;
            }
        }
        let missing: Vec<DriveFile> = remote.into_iter().filter(|f| !here.contains(&f.name) && safe_name(&f.name)).collect();
        if !missing.is_empty() {
            std::fs::create_dir_all(media).map_err(|e| e.to_string())?;
        }
        for f in missing {
            let bytes = self.download(&f.id)?;
            let tmp = media.join(format!("{}.tmp", f.name));
            std::fs::write(&tmp, bytes).map_err(|e| e.to_string())?;
            std::fs::rename(tmp, media.join(&f.name)).map_err(|e| e.to_string())?;
        }
        Ok(())
    }
}

/// A picture name from Drive that stays inside the media folder.
fn safe_name(name: &str) -> bool {
    !name.is_empty() && !name.contains(['/', '\\', ':']) && name != "." && name != ".." && !name.starts_with('.')
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use std::sync::{Arc, Mutex};

    #[test]
    fn plans_follow_what_changed() {
        let mut l = Link { synced_rev: Some("r1".into()), synced_hash: Some("h1".into()), ..Default::default() };
        assert_eq!(plan(Some("r1"), "h1", &l), Plan::Nothing);
        assert_eq!(plan(Some("r1"), "h2", &l), Plan::Push);
        assert_eq!(plan(Some("r2"), "h1", &l), Plan::Pull);
        assert_eq!(plan(Some("r2"), "h2", &l), Plan::Both);
        assert_eq!(plan(None, "h1", &l), Plan::Push);
        l = Link::default();
        assert_eq!(plan(None, "h", &l), Plan::Push, "first sign-in, empty Drive");
        assert_eq!(plan(Some("r"), "h", &l), Plan::Both, "first sign-in, Drive has notes");
    }

    #[test]
    fn pkce_matches_the_rfc_example() {
        // RFC 7636, appendix B.
        assert_eq!(pkce_challenge("dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk"), "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM");
    }

    #[test]
    fn reads_the_browser_answer() {
        let p = parse_callback("GET /?state=abc&code=4%2F0Ab_x&scope=email+openid HTTP/1.1").unwrap();
        assert_eq!(p[0], ("state".into(), "abc".into()));
        assert_eq!(p[1], ("code".into(), "4/0Ab_x".into()));
        assert_eq!(p[2].1, "email openid");
        assert!(parse_callback("GET /favicon.ico HTTP/1.1").is_none());
    }

    #[test]
    fn reads_the_email_from_an_id_token() {
        let payload = b64(br#"{"email":"amine@example.com","sub":"1"}"#);
        assert_eq!(email_of(&format!("e30.{payload}.sig")).as_deref(), Some("amine@example.com"));
        assert_eq!(email_of("nonsense"), None);
    }

    #[test]
    fn picture_names_stay_in_the_folder() {
        assert!(safe_name("123.png"));
        assert!(!safe_name("../board.json") && !safe_name("a\\b") && !safe_name(".hidden") && !safe_name(""));
    }

    // ---------- A small fake of Google's token and Drive services ----------

    #[derive(Default)]
    struct Fake {
        files: HashMap<String, (String, Option<String>, Vec<u8>, u32)>, // id → (name, parent, bytes, revision)
        next: u32,
        refreshes: u32,
    }

    fn query_of(target: &str) -> HashMap<String, String> {
        parse_callback(&format!("GET {target} HTTP/1.1")).unwrap_or_default().into_iter().collect()
    }

    fn respond(stream: &mut std::net::TcpStream, code: u16, body: &str) {
        let _ = write!(stream, "HTTP/1.1 {code} X\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len());
    }

    fn serve(fake: Arc<Mutex<Fake>>) -> String {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let base = format!("http://{}", listener.local_addr().unwrap());
        std::thread::spawn(move || {
            for stream in listener.incoming() {
                let Ok(mut stream) = stream else { continue };
                let mut data = Vec::new();
                let mut buf = [0u8; 65536];
                // Read the head, then as much body as it says.
                let (head, mut body) = loop {
                    let n = stream.read(&mut buf).unwrap_or(0);
                    data.extend_from_slice(&buf[..n]);
                    if let Some(i) = data.windows(4).position(|w| w == b"\r\n\r\n") {
                        break (String::from_utf8_lossy(&data[..i]).to_string(), data[i + 4..].to_vec());
                    }
                    if n == 0 {
                        break (String::new(), Vec::new());
                    }
                };
                let len = head.lines().find_map(|l| l.to_lowercase().strip_prefix("content-length:").map(|v| v.trim().parse::<usize>().unwrap_or(0))).unwrap_or(0);
                while body.len() < len {
                    let n = stream.read(&mut buf).unwrap_or(0);
                    if n == 0 {
                        break;
                    }
                    body.extend_from_slice(&buf[..n]);
                }
                let first = head.lines().next().unwrap_or("").to_string();
                let mut parts = first.split_whitespace();
                let (method, target) = (parts.next().unwrap_or(""), parts.next().unwrap_or(""));
                let path = target.split('?').next().unwrap_or("");
                let q = query_of(target);
                let authed = head.contains("Bearer access-1");
                let mut f = fake.lock().unwrap();
                match (method, path) {
                    ("POST", "/token") => {
                        f.refreshes += 1;
                        respond(&mut stream, 200, r#"{"access_token":"access-1","expires_in":3600}"#);
                    }
                    _ if !authed => respond(&mut stream, 401, "{}"),
                    ("GET", "/api/files") => {
                        let query = q.get("q").cloned().unwrap_or_default();
                        let list: Vec<_> = f
                            .files
                            .iter()
                            .filter(|(_, (name, parent, _, _))| {
                                let name_ok = query.split("name = '").nth(1).map(|r| r.split('\'').next().unwrap() == name).unwrap_or(true);
                                let parent_ok = match query.split_once("' in parents") {
                                    Some((before, _)) => parent.as_deref() == before.rsplit('\'').next(),
                                    None => parent.is_none(),
                                };
                                name_ok && parent_ok
                            })
                            .map(|(id, (name, _, _, rev))| serde_json::json!({"id": id, "name": name, "headRevisionId": format!("rev{rev}")}))
                            .collect();
                        respond(&mut stream, 200, &serde_json::json!({ "files": list }).to_string());
                    }
                    ("POST", "/api/files") => {
                        let meta: serde_json::Value = serde_json::from_slice(&body).unwrap();
                        f.next += 1;
                        let id = format!("f{}", f.next);
                        let parent = meta["parents"][0].as_str().map(String::from);
                        f.files.insert(id.clone(), (meta["name"].as_str().unwrap().into(), parent, Vec::new(), 1));
                        respond(&mut stream, 200, &serde_json::json!({"id": id}).to_string());
                    }
                    ("GET", p) if p.starts_with("/api/files/") => {
                        let id = &p["/api/files/".len()..];
                        let bytes = f.files.get(id).map(|x| x.2.clone()).unwrap_or_default();
                        let _ = write!(stream, "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n", bytes.len());
                        let _ = stream.write_all(&bytes);
                    }
                    ("POST", "/upload/files") => {
                        let text = String::from_utf8_lossy(&body).to_string();
                        let boundary = head.split("boundary=").nth(1).unwrap().lines().next().unwrap().trim().to_string();
                        let sections: Vec<&str> = text.split(&format!("--{boundary}")).collect();
                        let meta: serde_json::Value = serde_json::from_str(sections[1].split("\r\n\r\n").nth(1).unwrap().trim()).unwrap();
                        let content = sections[2].split_once("\r\n\r\n").unwrap().1.strip_suffix("\r\n").unwrap().as_bytes().to_vec();
                        f.next += 1;
                        let id = format!("f{}", f.next);
                        let name = meta["name"].as_str().unwrap().to_string();
                        f.files.insert(id.clone(), (name.clone(), meta["parents"][0].as_str().map(String::from), content, 1));
                        respond(&mut stream, 200, &serde_json::json!({"id": id, "name": name, "headRevisionId": "rev1"}).to_string());
                    }
                    ("PATCH", p) if p.starts_with("/upload/files/") => {
                        let id = p["/upload/files/".len()..].to_string();
                        let e = f.files.get_mut(&id).unwrap();
                        e.2 = body;
                        e.3 += 1;
                        let out = serde_json::json!({"id": id, "name": e.0, "headRevisionId": format!("rev{}", e.3)});
                        respond(&mut stream, 200, &out.to_string());
                    }
                    _ => respond(&mut stream, 404, "{}"),
                }
            }
        });
        base
    }

    fn endpoints(base: &str) -> Endpoints {
        Endpoints {
            auth: format!("{base}/auth"),
            token: format!("{base}/token"),
            revoke: format!("{base}/revoke"),
            api: format!("{base}/api"),
            upload: format!("{base}/upload"),
        }
    }

    fn temp(tag: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("folio-cloud-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn two_computers_share_one_drive() {
        let fake = Arc::new(Mutex::new(Fake::default()));
        let ep = endpoints(&serve(fake.clone()));
        let client = ClientId { id: "id".into(), secret: "s".into() };
        let account = Link { email: "a@b.c".into(), refresh_token: "r".into(), ..Default::default() };
        let (media_a, media_b) = (temp("a"), temp("b"));
        std::fs::write(media_a.join("7.png"), b"png!").unwrap();

        // Computer A signs in first: its board and picture go up.
        let mut a = Drive::new(client.clone(), ep.clone(), account.clone());
        assert!(matches!(a.sync("board one", &media_a, None).unwrap(), Outcome::Pushed));
        assert!(matches!(a.sync("board one", &media_a, None).unwrap(), Outcome::Same));
        assert_eq!(fake.lock().unwrap().refreshes, 1, "the access token is reused");

        // Computer B signs in with notes of its own: it's asked, and picks Drive's.
        let mut b = Drive::new(client.clone(), ep.clone(), account.clone());
        match b.sync("b's own board", &media_b, None).unwrap() {
            Outcome::Both(text) => assert_eq!(text, "board one"),
            o => panic!("expected a choice, got {o:?}"),
        }
        match b.sync("b's own board", &media_b, Some(Keep::Drive)).unwrap() {
            Outcome::Pulled(text, rev) => b.pulled(rev, &text),
            o => panic!("expected a pull, got {o:?}"),
        }
        assert_eq!(std::fs::read(media_b.join("7.png")).unwrap(), b"png!", "pictures come down too");

        // B edits; A, unchanged since, gets the edit.
        assert!(matches!(b.sync("board two", &media_b, None).unwrap(), Outcome::Pushed));
        match a.sync("board one", &media_a, None).unwrap() {
            Outcome::Pulled(text, rev) => {
                assert_eq!(text, "board two");
                a.pulled(rev, &text);
            }
            o => panic!("expected a pull, got {o:?}"),
        }

        // A pull the app didn't take is offered again, not overwritten.
        assert!(matches!(b.sync("board two + b", &media_b, None).unwrap(), Outcome::Pushed));
        assert!(matches!(a.sync("board two", &media_a, None).unwrap(), Outcome::Pulled(..)));
        assert!(matches!(a.sync("board two, edited meanwhile", &media_a, None).unwrap(), Outcome::Both(_)));
        assert!(matches!(a.sync("board two", &media_a, Some(Keep::Drive)).unwrap(), Outcome::Pulled(..)));
        match a.sync("board two", &media_a, Some(Keep::Drive)).unwrap() {
            Outcome::Pulled(text, rev) => a.pulled(rev, &text),
            o => panic!("expected a pull, got {o:?}"),
        }

        // Both edit: A is asked, and keeps its own.
        assert!(matches!(b.sync("board three", &media_b, None).unwrap(), Outcome::Pushed));
        assert!(matches!(a.sync("board two + a", &media_a, None).unwrap(), Outcome::Both(_)));
        assert!(matches!(a.sync("board two + a", &media_a, Some(Keep::ThisComputer)).unwrap(), Outcome::Pushed));
        let f = fake.lock().unwrap();
        let board = f.files.values().find(|x| x.0 == "board.json").unwrap();
        assert_eq!(board.2, b"board two + a");
        assert_eq!(f.files.values().filter(|x| x.0 == "Folio").count(), 1, "one Folio folder");
    }

    #[test]
    fn the_sign_in_round_trip() {
        let fake = Arc::new(Mutex::new(Fake::default()));
        let base = serve(fake);
        let ep = endpoints(&base);
        let client = ClientId { id: "id".into(), secret: "s".into() };
        let s = begin_sign_in(&client, &ep).unwrap();
        assert!(s.url.contains("code_challenge_method=S256") && s.url.contains("access_type=offline"));
        assert!(s.url.contains("drive%2Efile"));
        let redirect = s.redirect.clone();
        let state = s.state.clone();
        // The browser, coming back with a code.
        let browser = std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(200));
            ureq::get(&format!("{redirect}/?state={state}&code=c0de")).call().unwrap().into_string().unwrap()
        });
        // The fake token endpoint gives no refresh token, so this fails at
        // the last step, after the browser got its page.
        let r = finish_sign_in(s, &client, &ep, &|| false);
        assert!(browser.join().unwrap().contains("You're signed in"));
        assert_eq!(r.unwrap_err(), "Google gave no lasting sign-in · try again");
    }
}
