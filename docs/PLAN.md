# Folio: product and implementation plan

Folio is a desktop sticky-notes app for everyday use. Notes live on
**sheets**, one sheet per category, like the worksheets in an Excel workbook.
Each sheet is an endless canvas holding its own stickers, ink, text and
links. This document sets out the product, the stack, the architecture and
the roadmap. Each section says what v0.1 (in this repository) already does
and what is still to come.

---

## 1. Product

### What it has to do

| Capability | Built (v0.2) | Planned |
|---|---|---|
| Categories as sheets, each with its own content and camera | ✅ home screen of sheet cards with live miniatures (real note titles, on the sheet's own background), Recent/A–Z, pinned first; Trash and Archive, with their counts, held in the window's bottom-right corner; an emoji from a picker of about 200 (sheets have no colours of their own), rename, duplicate, archive, trash; templates (Kanban, weekly planner, brainstorm, meeting notes, reading list); the drawer (hidden sidebar, Ctrl+B); Ctrl+Tab | reorder by drag |
| Getting a thought down fast | ✅ Ctrl+Alt+N (or keys chosen in Settings: click the keys, press new ones; a hint if they lack Ctrl, Alt or the Windows key, or another app holds them; a warning where Ctrl+Alt may type letters) from any app opens a quick-note box; a row of the sheets under the text says where it goes (the Inbox first, any other by a click or Alt+←/→), and the one picked is offered next time (`Settings::quick_sheet`); tray icon (Windows); start with Windows | a phone companion |
| Notes visible outside the app | ✅ pin a note to the desktop: its own always-on-top window, ticks sync both ways, remembers where it was; see-through mode (solid again under the pointer) | |
| Freeform canvas | ✅ pan (wheel, Space+drag, middle-drag, Hand), zoom at the cursor, zoom to fit; box select, Ctrl- or Shift-click to add or take out (a tick on each), group move; with several notes selected a bar above the dock (count, lock, duplicate, move to a sheet, archive, trash, more, clear) and a selected note's menu act on all of them, hiding what fits one note only (edit, picture, desktop); alignment guides; snap to the dot grid (Settings); areas that carry their notes; tidy up; arrow keys between notes; a map in the corner; drag notes to the left edge to drop them on another sheet; notes lift while dragged and settle with a small bounce | |
| Sheet backgrounds | ✅ per sheet, from the sheet's menu › Background (with a live sample of each): dots, grid (squared paper), graph (a heavier line every fifth), lines, crosses, isometric dots, plain; six paper colours, light and dark; a picture or SVG from the computer filling the window behind the sheet, softened toward the paper (on by default); "Use on every sheet". Patterns are in world units, so they move and zoom with the sheet and thin out when zoomed far out | a picture that tiles or moves with the sheet |
| Sticker styling | ✅ 12 papers, 3 sizes, 3 fonts (sans, handwritten, mono), grow-with-text or fixed height (with a "+N more lines" fade), lock, a faint paper sheen; four note types: Card (the default), Taped (squarer paper, clear tape, a curl at the foot), Cheat card (the title on a deeper band of the paper), Index card (a pin, a pink margin, blue rules the text sits on, one thing per line); the type from the edit bar, a note's menu, S on the sheet, or the sheet's menu for a new note; new notes follow the type and font of the note changed last; pins for every type (index cards come with one): 18 shapes in five families (Classic push pin, ball, tack; Tech chip, bug, terminal, rocket, code; Cute heart, flower, bow, butterfly; Stars star, sparkle, moon; Fun paw, smiley, bolt), sizes S/M/L and eight colours, from the edit bar's Pin button or the note menu, for the whole selection at once; a pinned taped note drops its tape | slight tilt: GPUI can't rotate elements, so it needs custom-drawn notes |
| Editing | ✅ type straight on the note with a two-row edit bar. Writing: bold (Ctrl+B), italic (Ctrl+I), strikethrough (Ctrl+Shift+X), code (Ctrl+E), highlight (Ctrl+Shift+M), bullets (Ctrl+Shift+B), numbers (Ctrl+Shift+O), tasks (Ctrl+Shift+C), a table, a Remember box (Ctrl+Shift+R), Markdown (Ctrl+Shift+T), picture, reminder, more. Look: type, pin, paper (one swatch that opens the twelve, on the side away from the note), font, size, Done. The title and text are typed where and as large as they're drawn, so nothing jumps when a note opens. The bar (about 510 px) goes above or below the note, lined up with its left edge, right edge or middle or anywhere along that row, wherever it covers least of the other notes and never the note itself. Enter continues a list; Esc/Done keeps it | WYSIWYG rendering while typing |
| Text on the sheet | ✅ the Text tool (T) writes Markdown straight on the sheet: **bold**, *italic*, strikethrough, highlights, bullets, numbers and tasks, with the same shortcuts as notes and Enter continuing a list; lines wrap (at 480 units, or a width set by dragging the selected text's right-hand handle); a bar above while writing with the marks, the five inks at hand and a palette button for all 17, three typefaces and a slanted toggle, and four sizes (S/M/L/XL); the right-click menu has the sizes, typefaces and inks too; measured as drawn, so clicks find the real box (`src/ui/label.rs`) | alignment; text inside areas |
| Text into Markdown | ✅ `src/markdown.rs` turns plain text (pasted from Word, a PDF or a chat) into Markdown: `•`/`–`/`o` bullets, `1)`/`(a)` numbers, `☐`/`✔` boxes, `Term: meaning` with a bold term, `Tip:`/`Remember:` lines as a Remember box, tab-separated lines as a table, a short first line as the title. On the selection or the whole note (Ctrl+Shift+T, the edit bar's Markdown, a note's menu); Ctrl+Shift+V pastes as Markdown, on the sheet or in a note | rich-text (HTML) paste |
| Remember boxes and tables | ✅ `>` lines draw as a Remember box, labelled by a first line such as `**Tip:**` or `[!NOTE]`; tables draw on the paper with small-caps headers and dashed rules | |
| Tasks | ✅ `- [ ]` lines are real checkboxes you tick with a click, with a done/total chip | |
| Coloured text | ✅ highlights (`<mark color="green">`), in tasks too | foreground colour (§4.1) |
| Tables, images, GIFs, SVG | ✅ as before; link cards from a pasted address: the page's title, site icon and preview picture | crop, captions; animated SVG (§4.2) |
| Reminders | ✅ presets (in an hour, this evening, tomorrow morning, Monday); Windows notification + in-app toast; Today strip on the home screen; overdue in red | custom date and time; repeat |
| Search | ✅ every note on every sheet, by text, `#tag` (the tags on the home screen) or note colour (`paper:sage`); jumps to the note; Ctrl+K palette for commands, sheets and notes (the search field shows it on hover) | |
| Right-click menus | ✅ note, empty sheet, area, label, sheet card; Move to sheet | |
| Copy, cut, paste | ✅ Ctrl+C/X/V (and Copy, Cut in a note's menu, Paste in the sheet's): notes and text on the sheet go on the clipboard as text for other apps and whole for Folio (`app::Clip`: paper, type, pin, links between them), pasted where the pointer is, selected; cut notes wait in the Trash and the first paste takes them out; the one selected note's words can be selected with the mouse (`TextSelection::is_selectable_at` hands the press over), Ctrl+C then copies them; links in a note open on a click, a link card has an ↗ button | |
| Safety | ✅ undo (100 steps, each only what changed; up to 2 MB of them survive a restart), Trash (30 days), Archive, a daily backup (14 kept) restorable whole or one sheet at a time; "untouched for weeks" hint with keep-fresh or snooze for a week | |
| Google Drive | ✅ optional one-click "Continue with Google" (browser, PKCE, `drive.file` only; the client is built in from `google-client.env`, see `docs/GOOGLE_SIGN_IN.md`); board and pictures kept in a Folio folder in Drive, synced after changes, every minute, at start-up and quit; three-way merge per item from the last shared version, ids renumbered where two computers collide; first sync asks merge / keep mine / use Drive's; start over either way (`src/cloud.rs`, `src/merge.rs`, `src/ui/sync.rs`) | merge inside a note's text (today the newest edit wins); verified Google app (needed beyond 100 test users) |
| Export & import | ✅ chosen sheets to Markdown (a folder, one `.md` per sheet, pictures alongside) or a `.folio` file (pictures inside); import a `.folio` as new sheets, from the screen or by dropping it (`src/export.rs`, `src/ui/transfer.rs`) | PNG/PDF of a sheet |
| Settings | ✅ theme (light/dark/follow Windows), interface size, high contrast, reduce motion, default paper and font, guides, map, open on home/last sheet, start with Windows, quick-note shortcut, data folder, backups | |
| Window | ✅ no system title bar: the top bar is the title bar (drag to move, Windows snap, double-click to maximize), with Folio's own minimize / maximize / close; first launch on one empty sheet, no samples; on the right, undo/redo and sync/theme in small sunk trays (the key under the pointer rises; sync's tooltip says where it's at, its arrows turn while it syncs) and the screen's coloured main action; the search is a narrow pill (260 px) that widens to 420 px while you type | |
| Opening | ✅ animated logo intro, played in full at every launch and not skippable (about 4.2 s): Folio's dotted sheet comes up out of the dark from the middle outwards under a slow, breathing pink bloom and a vignette; the tile springs in onto a soft shadow and gives a little as the note lands; pin, highlight, lines and folded corner as before; a sheen of light crosses the whole tile as over glass (each band cut to the rounded square, `intro::clip_convex`); "folio." tracks in, its letters closing up as they rise; a pink hairline draws out under it and the line below fades up. Leaving, the wordmark flies into the top bar to where Folio's own "folio." sits (measured, `Folio::mark_at`), its cream turning the bar's ink as the dark lifts, while the logo draws back and everything else dissolves; the first time (welcome showing) it fades with the rest. `FOLIO_SLOW_MOTION` slows it too. The author signs in the bottom-right corner, quietly (`ui::signature`): small spaced capitals, CREATED & DESIGNED BY, over "M. Amine Said" in italic, written out left to right, then a fine pink swash drawn under it; the first time, a welcome: a butter note pinned to the dotted sheet asks your name (written in the hand on the note's line, Enter to go on), then the same note has you pick a face, a step marker at its top, the same signature in the same corner | |

### Experience principles

1. **Instant.** Every drag, stroke and zoom answers on the next frame. No
   spinner anywhere in normal use (Doherty threshold: under 100 ms for
   direct manipulation, under 400 ms for anything else).
2. **The canvas is the interface.** Chrome stays at the edges: sidebar,
   a thin top bar, one floating tool dock. Double-click the sheet for a note,
   drop a file to pin it.
3. **One way to do each thing, with a shortcut.** Tools have single-letter
   keys shown under the dock. Undo always works.
4. **Calm, tactile visuals.** Soft paper colours, generous radius, a
   two-layer shadow that rises when a sticker is picked up, a dotted sheet
   that coarsens as you zoom out, a warm grotesque typeface (Hanken
   Grotesk).
5. **Your data is files.** One folder: `board.json` plus `media/`. Easy to
   back up, sync or inspect.

### Keyboard

| Key | Action | Key | Action |
|---|---|---|---|
| V | Select / move | Ctrl+N or N | New note |
| H | Pan | Enter / double-click | Edit selected |
| P | Pen | Delete / Backspace | Delete selected |
| M | Marker | Ctrl+D | Duplicate |
| T | Text on the sheet | Ctrl+V | Paste text, picture or files |
| C | Connect | Ctrl+Z / Ctrl+Shift+Z | Undo / redo |
| E | Eraser | Ctrl+0 or Shift+1 | Zoom to fit |
| Esc | Back to Select | Ctrl+= / Ctrl+- | Zoom |
| Ctrl+F | Search | Ctrl+Shift+N | New sheet |
| Ctrl+Tab | Next sheet | Ctrl+Shift+L | Light / dark |
| Ctrl+Enter | Save in the editor | Middle-drag | Pan with any tool |

---

## 2. Technology stack

**Recommendation: GPUI (through `gpui-kit`, pinned at `=0.7.1`) with
gpui-component, serde and JSON in v0.1, and SQLite later.**

| Option | Verdict |
|---|---|
| **GPUI** (Zed's UI framework) | **Chosen.** GPU-rendered (wgpu: Vulkan, Metal, DX12), retained element tree with immediate-mode ergonomics. Text shaping, image decoding (including animated GIF), SVG via resvg, and vector paths via lyon are all built in. gpui-component adds text inputs and a Markdown view that already renders tables. It's the stack you already use in forge-gpui, so the patterns, tooling and screenshot scripts carry over. Apache-2.0. |
| egui | Very easy, but immediate-mode text layout is costly with hundreds of rich-text stickers, and it looks generic without a lot of custom styling. |
| iced | Good architecture, but its canvas and rich text are weaker for this kind of app, and its API still changes a lot. |
| Tauri + web canvas | Fastest way to a rich editor, but it means a webview, a JS runtime and two languages. That works against the "efficient Rust" goal. |
| Slint | Declarative and light, but a freeform canvas with custom paths and Markdown means building most of it yourself. |
| winit + vello/wgpu from scratch | Maximum control, but you'd write your own text layout, input fields and Markdown. Months of work before any notes. |

Risks with GPUI and how to handle them: its API changes between snapshots
(pin exact versions and upgrade on purpose), and its documentation is thin
(read the source; forge-gpui is a working reference).

Licensing: every dependency is Apache-2.0, MIT, BSD or similar; Lucide icons
are ISC and the fonts are OFL. Nothing is copied from Zed's GPL crates.

---

## 3. Architecture

```
src/
  main.rs        window, key bindings
  model.rs       Board → Category (sheet) → Sticker, Stroke, Label, Link; Camera;
                 geometry: hit tests, link curves, RDP simplify, fit  (no GPUI)
  store.rs       data folder, atomic JSON save/load (fsync, then rename),
                 the background saver, undo steps on disk, media import
  history.rs     undo steps as changes: what an edit did, item by item,
                 played back either way  (no GPUI)
  demo.rs        the first-run board
  theme.rs       tokens: colour (light/dark), type, space, radius, shadows
  assets.rs      embedded icons, fonts, logo
  ui/app.rs      Folio view: state, tools, gestures, undo/redo, saving
  ui/board.rs    the sheet: pointer handling, culling, painting dots/ink/links
  markdown.rs    a note's text as blocks (text, tasks, Remember boxes, tables),
                 the edit bar's formatting, text into Markdown  (no GPUI)
  ui/sticker.rs  one sticker: its type, paper, picture, title, Markdown,
                 level of detail
  ui/sidebar.rs  sheets list, theme switch
  ui/topbar.rs   breadcrumb, search, undo/redo, delete sheet, New note
  ui/toolbar.rs  floating tool dock + ink swatches, zoom control
  ui/editor.rs   the note/label editor dialog
```

### 3.1 Data model

```rust
Board    { version, categories: Vec<Category>, active, dark, next_id }
Category { id, name, blurb, accent, camera: Camera,
           stickers: Vec<Sticker>,   // back-to-front: order = z-order
           strokes:  Vec<Stroke>, labels: Vec<Label>, links: Vec<Link> }
Label    { id, at, text /* Markdown */, ink, font, size /* S M L XL */,
           width /* None: wraps at 480 */, italic }
Category { …, backdrop: { pattern, tint, image, soften } }
Sticker  { id, rect, paper, style /* Card, Taped, Cheat, Index */, title,
           body /* Markdown */, image, text_size, font, edited,
           pin: Option<{ shape, size, tone }>, … }
Stroke   { id, ink, width, marker, points: Vec<Vec2> }   // world units
Label    { id, at, text, ink }
Link     { id, from: StickerId, to: StickerId, ink }
Camera   { pan, zoom }          // screen = world × zoom + pan
```

- **World coordinates, one camera per sheet.** Everything is stored in
  world units, so zoom never rewrites data, and each sheet remembers where
  you left it.
- **IDs, not indexes.** Links refer to sticker IDs. Deleting a sticker
  deletes its links, and `normalize()` drops links that point at nothing in
  a hand-edited file.
- **Order is z-order.** Bring-to-front is a move within the `Vec`. No
  separate z field to keep in sync.
- **Versioned format.** `version` and `#[serde(default)]` on newer fields
  mean older files still load.

### 3.2 Rendering

Each frame the sheet is drawn in layers:

1. **Underlay `canvas`** (one element): the dot grid, links (dashed cubic
   Béziers that leave from the facing edges), then ink strokes (quadratic
   smoothing through midpoints). Only items whose bounds meet the viewport
   (plus a 200 px margin) are painted.
2. **Sheet header** (name, blurb, count, save state). Stickers can cover it.
3. **Labels** as text elements.
4. **Stickers** as GPUI elements (rounded quad, shadow, picture, text). They
   are culled to the viewport the same way.
5. **Floating UI**: tool dock, zoom control, editor dialog, toast.

**Level of detail:** below 45 % zoom a sticker shows only its title.
Markdown layout is the most expensive part of a frame, and text that small
can't be read anyway. The dot grid doubles its spacing as you zoom out, so
it never turns into noise.

### 3.3 Interaction model

A **tool** (what a press will do) is combined with a **gesture** (what the
current press is doing):

```
Gesture = None | Pan | Move(sticker) | MoveLabel | Resize(sticker)
        | Draw(stroke) | Erase | Connect(from, cursor)
```

One set of pointer handlers on the sheet does all hit testing against the
model, front-most first. Grip corner → resize; sticker → move (double-click
→ edit); label → move; empty space → pan (double-click → new note). A press
on any child element behaves exactly like a press on the model it shows.
Because of this, Markdown inside a sticker is rendered non-selectable.

When a press starts (or typing into a note or on the sheet begins), the
open sheet is copied (`history::Before`), not the whole board: that is
all a press or an edit can change, and a drag that drops notes on another
sheet copies that one too just before they land. When it ends, the copy
is compared with the board, and what changed becomes one undo step
(`history::Change`): the notes, strokes, text, links, frames, sheets and
Trash entries it touched, each before and after, and where any came, went or
moved to. The snapshot is then dropped. A pan alone records nothing.

### 3.4 Persistence

- `board.json` in the data folder (`%APPDATA%\Folio`, `~/Library/Application
  Support/Folio`, `$XDG_DATA_HOME/folio`, or `FOLIO_DATA_DIR`).
- Saves are **debounced** (350 ms after the last change), **off the UI
  thread** and **atomic**. The UI thread copies the board; a background
  task turns it to text, writes `board.json.tmp`, flushes it to the disk
  and renames it over the old file. Saves are numbered, and one that started
  earlier never lands over a later one. Quitting saves and waits for the
  disk. A file that won't parse is kept as `board.json.bad` and the app
  starts on the demo, so nothing is lost silently.
- `view.json` beside it holds where each sheet is scrolled to, on this
  computer only. Panning and zooming write just this small file, a moment
  after you stop, off the UI thread: the board isn't saved and nothing goes
  to Drive. A board from before it still opens where it was (its cameras
  are read, never written back).
- `history.json` beside it holds the newest undo steps (up to 100 and
  2 MB), so undo works after a restart. They're kept only if the newest
  still fits the board. The older file of whole boards is dropped.
- Pictures are **copied** into `media/` as `<id>.<ext>`. Moving or deleting
  the original never breaks a note.

---

## 4. Rich content

### 4.1 Text and colour

v0.1 renders sticker bodies with gpui-component's Markdown view: CommonMark
plus GitHub tables and task lists, plus `<mark color="…">` for highlights.
Folio maps seven colour names (yellow, green, blue, red, pink, purple,
orange) to its own light and dark shades (`ui/sticker.rs::tint_marks`), so
highlights stay readable in both themes.

**Coloured foreground text.** The Markdown view supports highlight colour
but not text colour. **Assumption:** `{red}text{/}` (or
`<span style="color:…">`) is an acceptable way to write it. **Plan:** a
`MarkdownPlugin` / inline-HTML hook in gpui-component's parser (its
`MarkdownExtensions` API exists for this) that emits a text-colour run.
Until then, highlights carry colour.

**WYSIWYG.** Phase 3 replaces "edit in a dialog" with editing in place on
the sticker: a rope-backed buffer with styled runs, rendered with GPUI's
`StyledText`, saved as Markdown so the file format stays the same.

### 4.2 Pictures and animated SVG

GPUI's image element decodes PNG, JPEG, WebP, GIF and BMP. It plays animated
GIFs (it needs an element id, which every sticker picture has) and renders
SVG through resvg.

**Animated SVG is not built in:** resvg renders one static frame and ignores
SMIL (`<animate>`, `<animateTransform>`, `<animateMotion>`) and CSS
animations. **Assumption:** the SVGs you care about are SMIL or simple CSS
keyframe animations (loaders, icons, diagrams), not scripted ones. **Plan
(Phase 3):**

1. Parse the file with `usvg` once and keep the tree.
2. A small animator evaluates SMIL `animate` / `animateTransform` /
   `set` (and the CSS subset `@keyframes` with `transform`, `opacity`,
   `fill`, `stroke`) at time *t*, producing per-node overrides.
3. Rasterise with `resvg` into a `tiny-skia` pixmap at the sticker's
   **on-screen** size, then hand it to GPUI as a `RenderImage`.
4. Drive it from `request_animation_frame` only while the sticker is
   visible and the window is focused. Cap at 30 fps and cache frames for
   short loops, so a sheet full of animated icons stays cheap.

If that turns out too large to build, a fallback: convert on import to an
animated WebP (pre-rendered frames). It's lossy but simple.

### 4.3 Drawing on the sheet

v0.1 has pen, marker and eraser, saved as world-space polylines simplified
with Ramer–Douglas–Peucker (at a tolerance of about a third of a screen
pixel). **Planned:** pressure from tablets (GPUI pointer events carry it on
some platforms), shape tools (rectangle, ellipse, line, arrow) stored as
`Shape { kind, rect, ink, fill }`, and a lasso to select, move and recolour
ink.

---

## 5. Performance

### 5.1 Targets

- 120 fps pan/zoom on a sheet with **2,000 stickers and 50,000 ink points**
  on a mid-range laptop. Frame budget under 8 ms.
- Cold start under 300 ms. Saving never blocks a frame.

### 5.2 What v0.1 already does

- Viewport culling for stickers, labels, strokes and links.
- Level of detail: titles only below 45 % zoom; dot grid coarsens.
- Strokes simplified when finished. While drawing, points closer than one
  screen pixel are skipped.
- One `canvas` element paints all ink and links (one element, many paths)
  instead of thousands of elements.
- Debounced saves, so a drag writes the file once, not on every frame;
  written off the UI thread.
- Undo steps hold only what changed: moving one note among 2,000 is a few
  hundred bytes, where a copy of the board was 4 MB.
- Optimised dependencies in dev builds; thin LTO in release.

### 5.3 Next steps, in order of payoff

1. **Spatial index.** Hit tests and culling are linear scans. That's fine
   into the low thousands; past that, keep an `rstar` R-tree per sheet,
   updated on move.
2. **Cache tessellated ink.** Each stroke is re-tessellated by lyon every
   frame. Cache the `Path` per stroke in screen space and rebuild it only
   when zoom changes (pan is a translation), or rasterise settled ink into
   tiles.
3. **Smaller saves.** Saving is off the UI thread (done), but it still
   copies the whole board there and writes all of it. Once the board is in
   SQLite, write only the rows that changed. Drive sync turns the board to
   text off the UI thread (only a copy is taken there), sends the same text
   again while nothing changed, and lists Drive's pictures only when the
   board or the pictures here changed.
4. **Picture mipmaps.** Decode large pictures to a downscaled copy at import,
   matched to the largest size a sticker shows them at, so a 24 MP photo
   doesn't sit in GPU memory at full size.
5. **Markdown cache.** Parse once per edit and keep the AST keyed by
   `(sticker id, edited)`.

### 5.4 Memory

Undo steps hold only what an edit changed (`history.rs`), so 100 of them
cost little on any size of board. A step that throws away a whole sheet
holds that sheet, as the Trash does.

---

## 6. Storage roadmap

| Stage | Format | Why |
|---|---|---|
| v0.1 | One JSON file + `media/` | Transparent, easy to back up, trivially correct with atomic rename. |
| v0.3 | **SQLite** (`rusqlite`, bundled; WAL) with one row per item and a `sheets` table | Incremental saves, crash safety, fast search with FTS5 across all sheets. JSON stays as export/import. |
| v1.0 | Daily rotating backups (`backups/YYYY-MM-DD.json.gz`, keep 14), export a sheet to PNG/PDF/Markdown | Peace of mind and sharing. |
| v0.2 | Optional Google Drive sync: board.json by Drive revision, merged per item (three-way, from `sync-base.json`) when both changed | Multi-device with the account people already have, without overwriting anyone's work. |
| Later | The same merge over a plain folder (OneDrive/Syncthing); merging inside a note's text | No account needed; two edits to one note both survive. |

---

## 7. Visual design

- **Tokens** live in `src/theme.rs`: colours (light/dark), the type scale
  (11–26), space (2–32), radii (6/8/12/16), sizes and shadows. Views never
  use raw values from elsewhere.
- **Palette:** one brand pink, the logo's `#F30D6A`, for the main
  buttons, selection, the search's ring, highlights of what's chosen, the
  index card's margin, the pink pin and the opening (hover `#D9085C`, a
  `#FEE6EF` tint with `#B5084F` text on it; in dark, `#FF3D86` for hover
  and selection, a `#3D1424` tint with `#FF8CB6` on it). Paper canvas
  `#F6F7F3` with a `#D6DCD4` dot grid, ink `#18211F`. Sticker papers:
  butter, sage, sky, blush, lilac, plain and more, each with a dark twin
  of the same hue at about 20 % lightness.
- **Type:** Hanken Grotesk (OFL) for UI and notes; JetBrains Mono for code.
- **Logo** (`assets/brand/`): a cream sticky note in a heavy, rounded
  black outline, pinned at the top (the pin sits in a notch in the
  paper), with a pink highlight over two lines of writing and its corner
  folded, on a flat hot-pink tile (`#F30D6A`). Flat, no gradients, so it
  reads the same at 16 px as at 1024. `folio-icon.svg` is the app icon,
  its tile filling the square as Chrome's and Teams' icons do; it is the
  one drawing at every size. `folio-logo.svg` is the same on a smaller
  tile with a light shadow (README, the quick note). `scripts/icons.sh`
  renders the PNGs and `assets/folio.ico` (21 sizes, 16–256, each drawn
  at its size), which the build embeds in the `.exe`. Each window asks
  Windows for the icon at its own display scale, big and small, so the
  taskbar never stretches one.
- **Motion ("Glide", `theme::Motion`, `src/ui/motion.rs`):** everything
  fades up a few pixels and settles with an ease-out, and leaves the same
  way, faster, with an ease-in. Menus, popovers, the edit bar and the ink
  pills: 180 ms in, 120 out, 8 px. Dialogs and the welcome: 240 in, 140
  out, 14 px, over a backdrop that dims in and lifts. The search: the
  field widens 260→420 px, results rise 6 px row by row (22 ms apart), and
  sink as it narrows. The drawer slides in 260 ms (ease-out-expo) and out
  180. Pages fade up 8 px in 280 ms. Toasts slide in 18 px from the right.
  A new note fades up from 96 % in 220 ms (drawn at the camera's zoom
  about its centre, so it really scales); a note leaving fades, sinks and
  shrinks to 97 % in 160; a pin lands from 85 % in 180 (a beat after a new
  note); a new picture fades in. A desktop note and the quick note fade up
  10 px in their own windows and play out before the window closes.
  Tooltips fade in; light ↔ dark fades the old canvas colour off in 220 ms.
  Whatever closes, however it was closed, is kept on screen as an
  unclickable copy while it leaves (`motion::Exit`, `NoteMotion`). Notes
  picked up and dropped keep their own lift and settle. Settings › Reduce
  motion makes all of it instant. `FOLIO_SLOW_MOTION=10` plays everything
  ten times slower, to look at a transition frame by frame.
- **Sheet backgrounds** use `canvas_dot` (dots, crosses, graph paper's
  heavier lines), `canvas_line` (the other lines) and `tint(Tint)`, the
  paper colours, light and dark.
- **Note types** draw from their own tokens: `paper_band` (a cheat card's
  band, per paper), `tape`/`tape_edge`, `curl`, `wash` (the Remember box),
  `rule` (table rules), `ruled`/`margin`/`head_rule` (index cards), and
  `shadow_taped`/`shadow_index`. The type and writing icons (note-*,
  bold, italic, strikethrough, code, list, list-ordered, table) are
  Folio's own, drawn to match Lucide's.
- **Pins** are Folio's own SVGs in `assets/pins/` (64-unit view box). Shapes
  that take a colour use `{main}`, `{shade}` and `{light}`, filled from
  `pin_tone(PinTone)` in light and dark; the others keep their own colours.
  Each is rasterised once per shape and colour and cached. A pin hangs over
  the note's top edge by its `anchor` (the push pin's needle goes deeper).
- **Tilt (planned):** the mockup tilts stickers ±2°. GPUI elements can't be
  rotated, so this needs either painting stickers as textured quads in a
  custom element or rendering them to a texture. That's deferred because
  it costs crisp text and hit-testing simplicity.

---

## 8. Roadmap

**v0.1.** Sheets, canvas, stickers with Markdown, tables, highlights,
pictures and GIFs, pen, marker, eraser, labels, links, undo, dark mode,
autosave, the logo.

**v0.2, this repository.** The home screen and hidden drawer, the opening
animation, typing on notes, real checkboxes, notes that grow, right-click
menus, multi-select and guides, areas and templates, reminders and Today,
search across sheets and tags, Ctrl+K, quick notes from anywhere, notes on
the desktop, tray and start-up, Trash/Archive/backups, Settings, link
cards, the handwritten font, high contrast and interface size, and
optional Google Drive sync with merging, and export to Markdown or a
Folio file.

**v0.3: data and scale.** SQLite store with full-text search; R-tree;
cached ink tessellation; picture mipmaps; reorder
sheets by drag.

**v0.4: richer content.** Coloured text; WYSIWYG while typing; shape
tools; animated SVG player; image crop; custom reminder times and repeats.

**v1.0: ship.** Windows installer and signed build (with its own
notification identity), export (PNG/PDF/Markdown), screen-reader labels,
crash log.

---

## 9. Assumptions

- **Desktop only, single user, local files.** No accounts or cloud in v1.
- **Windows is the main target, but nothing is Windows-only.** v0.1 has been
  run on Linux (Xvfb with Mesa lavapipe), and its Windows `.exe`
  (cross-compiled with MinGW) under Wine 9, where it draws the whole app.
  Under Wine it needed Microsoft's `d3dcompiler_47.dll`, a stand-in
  `icuuc.dll` and `GPUI_DISABLE_DIRECT_COMPOSITION=1`. Real Windows 10
  (1703+) and 11 ship the first two and support the third, but the `.exe`
  has not yet been run on a real Windows. macOS hasn't been built.
- **Markdown is the storage format for note text.** Rich text, when it
  arrives, still saves as Markdown so files stay readable.
- **Coloured text** means highlight now and foreground later (§4.1).
- **Animated SVG** means SMIL/CSS animation, not script (§4.2). Animated GIF
  works today.
- **Ink is drawn beneath stickers** (it annotates the sheet, as asked). A
  per-stroke "on top" layer can come with shape tools.
- **Windows-only parts were built, not run.** The tray icon, Windows
  notifications (sent as PowerShell until an installer gives Folio its own
  identity) and the start-up registry entry compile into the `.exe` but
  haven't been seen on a real Windows; Wine has no tray to look at. The
  Ctrl+Alt+N shortcut and desktop notes were tested on Linux (X11).
- **Drop position:** GPUI's drop event carries no position, so dropped files
  land at the last pointer position over the sheet.
