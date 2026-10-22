<p align="center"><img src="assets/brand/folio-128.png" width="96" alt="Folio logo"></p>

# Folio

Sticky notes on endless sheets: one sheet per category, like worksheets in a
workbook. Written in Rust on [GPUI](https://www.gpui.rs/) (through
`gpui-kit`) for a GPU-drawn, low-latency canvas.

![Folio home screen](docs/screenshots/home-light.png)

- **Home:** every sheet as a card with a live miniature, today's reminders,
  your #tags, and search across every note. The sheets drawer stays hidden
  until you want it (top-left button or Ctrl+B).
- **Notes:** type straight onto them; Markdown with tables, task lists you
  tick with a click, coloured highlights, pictures and animated GIFs, link
  cards; notes grow with their text; reminders; lock; a handwritten font.
- **The sheet:** pan and zoom, box-select and move groups with alignment
  guides, areas (Kanban columns, days of the week…), pen, marker, eraser,
  text on the sheet, curved links between notes, tidy up, a map.
- **Everywhere:** Ctrl+K for commands, sheets and notes; Ctrl+Alt+N from
  any app for a quick note into your Inbox; notes pinned to the desktop
  above other windows; a tray icon and starting with Windows.
- **Safe:** undo, a Trash that keeps things 30 days, an Archive, and a
  backup every day (the last 14 kept) you can restore from.

![Folio, a sheet in dark mode](docs/screenshots/sheet-dark.png)

## Run it

```sh
cargo run --release
```

On Linux, GPUI needs `libxkbcommon-dev libxkbcommon-x11-dev libwayland-dev
libvulkan-dev libfontconfig1-dev libx11-xcb-dev`, plus a Vulkan driver.

Your workspace is saved in `%APPDATA%\Folio` (Windows),
`~/Library/Application Support/Folio` (macOS) or `~/.local/share/folio`
(Linux). Set `FOLIO_DATA_DIR` to use another folder. It holds `board.json`
and a `media/` folder for pictures.

### Google Drive (optional)

Settings › Your data › **Sign in with Google** opens Google's sign-in in
your browser. After that, Folio keeps `board.json` and `media/` in a
**Folio** folder in your Drive: it syncs a few seconds after each change,
once a minute, at start-up and on quit. Sign in on another computer and the
same notes come down. If notes changed on two computers before a sync, you
choose which to keep; the other side goes to Backups, so nothing is lost.
Folio asks only for `drive.file`, so it sees just the files it made. The
sign-in is kept in `google.json` in the data folder, never in the board.
Your theme, start-up and shortcut settings stay per computer.

Sign-in needs the app's own OAuth client, set when the .exe is built:

1. In [Google Cloud console](https://console.cloud.google.com/), make a
   project and enable the **Google Drive API**.
2. *Google Auth Platform › Branding*: an app name (Folio) and your email.
   *Audience*: External; while it's in Testing, add the Google accounts
   that may sign in as test users. *Data access*: add the
   `.../auth/drive.file` scope.
3. *Clients › Create client*: type **Desktop app**. Copy its ID and secret.
4. Build with them:

   ```sh
   FOLIO_GOOGLE_CLIENT_ID=….apps.googleusercontent.com \
   FOLIO_GOOGLE_CLIENT_SECRET=GOCSPX-… \
   cargo build --release --target x86_64-pc-windows-gnu
   ```

   The same two variables also work at run time. Google doesn't treat a
   desktop app's secret as secret; PKCE protects each sign-in.

Without a client the button says sign-in isn't set up in this build, and
everything else works as before. To try the whole flow without an account,
a debug build talks to a local fake of Google when `FOLIO_GOOGLE_FAKE` is
set to its address.

## Use it

| | |
|---|---|
| Double-click the sheet | A new note, ready to type into |
| Double-click a note, or Enter | Type into it; Esc or Done to finish |
| Right-click | A note's, the sheet's, an area's or a card's menu |
| Drag the empty sheet | Select a group; Space+drag, middle-drag or the wheel pans |
| Ctrl+wheel | Zoom |
| `V H P M T C F E` | Select, pan, pen, marker, text, connect, area, eraser |
| Arrows | Move between notes |
| Ctrl+K | Commands, sheets and notes |
| Ctrl+Alt+N | Quick note from any app |
| Ctrl+N / Ctrl+Shift+N | New note / new sheet |
| Ctrl+B | The sheets drawer |
| Ctrl+Z / Ctrl+Shift+Z | Undo / redo |
| Ctrl+, | Settings |

Drop pictures or `.md`/`.txt` files on a sheet, or paste them; paste a web
address for a link card.

See [docs/PLAN.md](docs/PLAN.md) for the product and architecture plan and
the roadmap.

## Windows build

Cross-compiled from Linux with MinGW, as forge-gpui's installer does:

```sh
rustup target add x86_64-pc-windows-gnu   # plus gcc-mingw-w64-x86-64
cargo build --release --target x86_64-pc-windows-gnu
# → target/x86_64-pc-windows-gnu/release/folio.exe
```

The patched GPUI Windows renderer in `vendor/gpui-pre-windows` compiles its
shaders when the app starts, so an .exe built without the Windows SDK still
draws. The icon (`assets/folio.ico`) is embedded by `build.rs`. A native
build (`cargo build --release` on Windows with MSVC) should work too; it
hasn't been tried yet.

## Develop

```sh
cargo test                  # model, storage and Markdown-tint tests
scripts/screenshots.sh out  # light/dark screenshots on a virtual display (Linux)
scripts/icons.sh            # logo SVGs → PNGs and the Windows .ico
```

Dependencies are pinned to exact versions. Icons are Lucide (ISC); the fonts
are Hanken Grotesk, JetBrains Mono and Comic Neue (OFL).
