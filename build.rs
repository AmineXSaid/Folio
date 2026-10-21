//! Two jobs:
//! - Lists every file under `assets/icons` and `assets/brand` (PNG) for
//!   `src/assets.rs` to embed, so a new icon needs no hand-kept list.
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
    let out = Path::new(&std::env::var("OUT_DIR").unwrap()).join("assets.rs");
    std::fs::write(out, code).unwrap();
}
