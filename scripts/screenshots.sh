#!/usr/bin/env bash
# Screenshots of Folio, light and dark, so "run it and look at it" is one
# command. Linux only: Xvfb, xdotool, ImageMagick's import, and a Vulkan
# driver (with no GPU, Mesa's lavapipe: apt install mesa-vulkan-drivers).
#
#   cargo build && scripts/screenshots.sh [out-dir]
#
# Runs the debug build on a virtual display with a throwaway data folder
# (FOLIO_DATA_DIR) and FOLIO_DEMO=1, so it opens on the sample sheets, and saves, as
# <name>-light.png and <name>-dark.png:
#   intro   the opening, mid-way      home    the home screen
#   intro-late  the opening, the logo drawn
#   welcome  the first run's note asking your name
#   welcome-look  the same note, picking a face
#   drawer  the sheets drawer         sheet   a sheet
#   edit    typing into a note        menu    a note's right-click menu
#   palette Ctrl+K                    settings
#   types   the cheatsheet sheet: taped notes, cheat cards, index cards
#   background  the sheet menu's Background page: patterns, colours, a picture
#   pins    a note menu's Pin page: shapes, sizes, colours
#   text    writing on the sheet: wrapping Markdown, its bar, an ink

set -euo pipefail
cd "$(dirname "$0")/.."
OUT="${1:-screenshots}"
mkdir -p "$OUT"
BIN="target/debug/folio"
[ -x "$BIN" ] || { echo "Build first: cargo build" >&2; exit 1; }

export DISPLAY=:98
TMP="$(mktemp -d)"
XVFB_PID="" APP_PID=""
cleanup() {
  [ -n "$APP_PID" ] && kill "$APP_PID" 2>/dev/null || true
  [ -n "$XVFB_PID" ] && kill "$XVFB_PID" 2>/dev/null || true
  rm -rf "$TMP"
}
trap cleanup EXIT
Xvfb :98 -screen 0 1280x800x24 >/dev/null 2>&1 &
XVFB_PID=$!
sleep 1
if [ -z "${VK_ICD_FILENAMES:-}" ] && [ -f /usr/share/vulkan/icd.d/lvp_icd.json ] && [ ! -e /dev/dri ]; then
  export VK_ICD_FILENAMES=/usr/share/vulkan/icd.d/lvp_icd.json
fi
export XDG_RUNTIME_DIR="$TMP/run" WAYLAND_DISPLAY=; mkdir -p -m 700 "$XDG_RUNTIME_DIR"

shot() { sleep 0.7; import -window root "$OUT/$1-$mode.png"; }
key() { xdotool key --clearmodifiers "$@"; sleep 0.4; }

for mode in light dark; do
  rm -rf "$TMP/data"
  mkdir -p "$TMP/data"
  FOLIO_DEMO=1 FOLIO_DATA_DIR="$TMP/data" "$BIN" >"$TMP/app-$mode.log" 2>&1 &
  APP_PID=$!
  WID=""
  for _ in $(seq 1 40); do WID="$(xdotool search --pid "$APP_PID" 2>/dev/null | tail -1 || true)"; [ -n "$WID" ] && break; sleep 0.25; done
  xdotool windowmove "$WID" 0 0 windowsize "$WID" 1280 800
  xdotool windowfocus --sync "$WID" 2>/dev/null || true
  sleep 1.6; import -window root "$OUT/intro-$mode.png"
  sleep 1.2; import -window root "$OUT/intro-late-$mode.png"
  sleep 3.3
  # The first run asks your name, then your look: "Start writing".
  [ "$mode" = dark ] && key ctrl+shift+l
  xdotool type --delay 30 "Sam"; shot welcome
  key Return; shot welcome-look
  xdotool mousemove 831 540 click 1; sleep 1.5
  shot home
  key ctrl+b; shot drawer; key Escape
  # The first sheet, from the palette.
  # The cheatsheet: the three note types.
  key ctrl+k; xdotool type --delay 20 "hil testing"; key Return
  xdotool mousemove 640 790; shot types
  xdotool mousemove 680 150 click 3; sleep 0.6; xdotool mousemove 760 360 click 1; shot background; key Escape
  xdotool mousemove 1135 600 click 3; sleep 0.6; xdotool mousemove 1030 432 click 1; shot pins; key Escape
  # Pan to empty sheet, write with the Text tool, pick the coral ink.
  sleep 0.5; xdotool mousemove 640 660 click 1; sleep 0.4; key h
  xdotool mousemove 640 660 mousedown 1; sleep 0.2; xdotool mousemove 640 400; xdotool mousemove 640 250 mouseup 1; sleep 0.5
  key t; xdotool mousemove 120 360 click 1; sleep 0.8
  xdotool type --delay 15 "Software-Defined Vehicles: the car is a **computer** on wheels, kept up to date over the air."
  xdotool mousemove 421 281 click 1; shot text; key Escape; key v
  key ctrl+k; xdotool type --delay 20 "studio"; key Down Return
  shot sheet
  # A note's menu, then typing into it.
  xdotool mousemove 150 260 click 3; shot menu; key Escape
  xdotool mousemove 580 600 click --repeat 2 --delay 80 1; sleep 1.2
  xdotool type --delay 25 " Edited in place."; shot edit; key Escape
  key ctrl+k; xdotool type --delay 20 "seed"; shot palette; key Escape
  key ctrl+comma; shot settings
  kill "$APP_PID"; APP_PID=""
  sleep 0.5
done
echo "Saved to $OUT/"
