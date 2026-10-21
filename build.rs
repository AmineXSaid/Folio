//! On Windows, the Folio logo becomes the .exe's icon (resource 1, which the
//! window and the taskbar load too): `assets/folio.rc`. A build without a
//! resource compiler fails here rather than ship a blank icon.

fn main() {
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        println!("cargo:rerun-if-changed=assets/folio.rc");
        println!("cargo:rerun-if-changed=assets/folio.ico");
        embed_resource::compile("assets/folio.rc", embed_resource::NONE)
            .manifest_required()
            .expect("the app icon (assets/folio.rc) could not be compiled");
    }
}
