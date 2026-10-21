# Folio — working rules

Sticky notes on endless sheets, in Rust on GPUI (through `gpui-kit`).
`docs/PLAN.md` is the product and architecture plan; keep it current when a
feature lands or a decision changes.

1. **Write everything original.** Never copy code from Zed's GPL crates.
   Every dependency must be Apache-2.0, MIT, BSD, ISC or similar (fonts OFL).
2. **Pin versions** with `=x.y.z`; GPUI's API changes between snapshots.
3. **Model apart from views.** `src/model.rs` and `src/store.rs` know nothing
   of GPUI and carry the unit tests. One file per view under `src/ui/`.
4. **Tokens only.** Colours, sizes, spacing, radii and shadows come from
   `src/theme.rs`, in light and dark.
5. **Every shortcut is an action** bound in `main.rs`, and reachable from a
   visible control too.
6. **No warnings.** `cargo build` with zero warnings and `cargo test` green
   before every commit.
7. **Run it and look at it.** For visible changes run
   `scripts/screenshots.sh` and check light and dark. Say what was seen on
   Linux only and what on Windows.
