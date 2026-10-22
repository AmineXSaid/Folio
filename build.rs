//! Three jobs:
//! - Lists every file under `assets/icons` and `assets/brand` (PNG) for
//!   `src/assets.rs` to embed, so a new icon needs no hand-kept list.
//! - Embeds any .ttf/.otf in `assets/fonts/extra` (git-ignored): fonts you
//!   hold a licence to ship, such as Anthropic Sans, go in the .exe from
//!   there and never into the repository.
//! - Builds in Folio's Google sign-in (its OAuth client) from
//!   `google-client.env` (git-ignored), unless the environment already
//!   sets `FOLIO_GOOGLE_CLIENT_ID`. With it, signing in is one click.
//! - On Windows, embeds the Folio logo as the .exe's icon (resource 1, which
//!   the window and the taskbar load too): `assets/folio.rc`. A build
//!   without a resource compiler fails here rather than ship a blank icon.

use std::fmt::Write as _;
use std::path::Path;

fn main() {
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        println!("cargo:rerun-if-changed=assets/folio.rc");
        println!("cargo:rerun-if-changed=assets/folio.ico");
        embed_resource::compile("assets/folio.rc", embed_resource::NONE)
            .manifest_required()
            .expect("the app icon (assets/folio.rc) could not be compiled");
    }
    google_client();
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("assets");
    let mut files = Vec::new();
    for (dir, ext) in [("icons", "svg"), ("brand", "png")] {
        println!("cargo:rerun-if-changed=assets/{dir}");
        let mut names: Vec<String> = std::fs::read_dir(root.join(dir))
            .expect("assets dir")
            .flatten()
            .filter_map(|e| e.file_name().to_str().map(String::from))
            .filter(|n| n.ends_with(ext))
            .collect();
        names.sort();
        files.extend(names.into_iter().map(|n| format!("{dir}/{n}")));
    }
    let mut code = String::from("const FILES: &[(&str, &[u8])] = &[\n");
    for f in &files {
        let full = root.join(f);
        writeln!(code, "    ({f:?}, include_bytes!({:?})),", full.display().to_string()).unwrap();
    }
    code.push_str("];\n");

    println!("cargo:rerun-if-changed=assets/fonts/extra");
    let mut extra: Vec<String> = std::fs::read_dir(root.join("fonts/extra"))
        .map(|r| {
            r.flatten()
                .filter_map(|e| e.file_name().to_str().map(String::from))
                .filter(|n| {
                    let l = n.to_ascii_lowercase();
                    l.ends_with(".ttf") || l.ends_with(".otf")
                })
                .collect()
        })
        .unwrap_or_default();
    extra.sort();
    code.push_str("const EXTRA_FONTS: &[&[u8]] = &[\n");
    for f in &extra {
        let full = root.join("fonts/extra").join(f);
        writeln!(code, "    include_bytes!({:?}),", full.display().to_string()).unwrap();
    }
    code.push_str("];\n");
    let out = Path::new(&std::env::var("OUT_DIR").unwrap()).join("assets.rs");
    std::fs::write(out, code).unwrap();
}

/// `google-client.env` holds `FOLIO_GOOGLE_CLIENT_ID=…` and
/// `FOLIO_GOOGLE_CLIENT_SECRET=…`; each becomes a compile-time variable
/// (read by `option_env!` in cloud.rs) unless the environment has it.
fn google_client() {
    println!("cargo:rerun-if-changed=google-client.env");
    for key in ["FOLIO_GOOGLE_CLIENT_ID", "FOLIO_GOOGLE_CLIENT_SECRET"] {
        println!("cargo:rerun-if-env-changed={key}");
    }
    let file = Path::new(env!("CARGO_MANIFEST_DIR")).join("google-client.env");
    let Ok(text) = std::fs::read_to_string(file) else { return };
    for line in text.lines() {
        let Some((key, value)) = line.split_once('=') else { continue };
        let (key, value) = (key.trim(), value.trim().trim_matches('"'));
        if matches!(key, "FOLIO_GOOGLE_CLIENT_ID" | "FOLIO_GOOGLE_CLIENT_SECRET")
            && !value.is_empty()
            && std::env::var_os(key).is_none()
        {
            println!("cargo:rustc-env={key}={value}");
        }
    }
}
