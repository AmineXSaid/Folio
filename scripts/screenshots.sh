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
#   drawer  the sheets drawer         sheet   a sheet
#   edit    typing into a note        menu    a note's right-click menu
#   palette Ctrl+K                    settings

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
  sleep 3
  [ "$mode" = dark ] && key ctrl+shift+l
  shot home
  key ctrl+b; shot drawer; key Escape
  # The first sheet, from the palette.
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
