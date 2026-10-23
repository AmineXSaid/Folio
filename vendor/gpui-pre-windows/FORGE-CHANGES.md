# Changes made for Folio (copied from forge-gpui)

This is `gpui-pre-windows` 0.3.8 (Apache-2.0, `LICENSE-APACHE`), used through
`[patch.crates-io]` in Folio's Cargo.toml (made for forge-gpui). Changes:

- `src/directx_renderer.rs`, `build.rs`: the HLSL shaders are embedded with
  `include_str!` and compiled at runtime with `D3DCompile`
  (`d3dcompiler_47.dll`, part of Windows 10 and 11) unless `build.rs` compiled
  them with the Windows SDK's `fxc.exe` (it then sets `gpui_shader_bytes`).
  Upstream compiles them at runtime only in debug builds, from the build
  machine's source path, so a cross-compiled release build could not draw.
- `src/direct_write.rs` (`matching_font_faces`): where a font collection has
  no font set (`IDWriteFontCollection1::GetFontSet` fails, as in Wine), a
  font is looked up by family name with the older DirectWrite calls instead
  of no font at all (which panicked: "failed to resolve font").
- `src/window.rs`, `src/events.rs`: Direct Manipulation (touchpad gestures) is
  optional. Where it can't start, the window opens without it instead of
  failing ("initializing Direct Manipulation").
- Folio's own, not from forge-gpui: `src/window.rs` (`set_window_icons`),
  `src/events.rs` (`WM_DPICHANGED`):
  each window gets the app icon at the exact big and small sizes for its
  display scale with `WM_SETICON`, again when the scale changes. Upstream
  only sets the class icon, one size, which Windows stretches for the
  taskbar and title bar (blurry at 125 % and up).
