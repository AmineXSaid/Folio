# Changes made for Folio

This is `gpui-base` 0.7.2 (Apache-2.0, `LICENSE-APACHE`), used through
`[patch.crates-io]` in Folio's Cargo.toml. Changes:

- `src/text/node.rs`: `TextMark` has a `color` (the text's own colour),
  merged like `highlight` and drawn as the run's foreground colour.
- `src/text/format/markdown.rs`, `src/text/format/html.rs`: inline
  `<font color="#rrggbb">` and `<span style="color: #rrggbb">` colour the
  text inside them (hex colours only).
- `Cargo.toml`: the crate's own tests and benches are left out (not copied).
