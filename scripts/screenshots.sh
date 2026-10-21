#!/usr/bin/env bash
# Screenshots of Folio, light and dark, so "run it and look at it" is one
# command. Linux only: Xvfb, xdotool, ImageMagick's import, and a Vulkan
# driver (with no GPU, Mesa's lavapipe: apt install mesa-vulkan-drivers).
#
#   cargo build && scripts/screenshots.sh [out-dir]
#
# Runs the debug build on a virtual display with a throwaway data folder
# (FOLIO_DATA_DIR), so it opens on the demo board, and saves:
#   sheet-{light,dark}.png    the demo sheet
#   editor-{light,dark}.png   a note open in the editor
#   ink-{light,dark}.png      a pen stroke and the pen's ink swatches

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
export XDG_RUNTIME_DIR="$TMP/run"; mkdir -p -m 700 "$XDG_RUNTIME_DIR"

shoot() { sleep 0.8; import -window root "$OUT/$1.png"; }
key() { xdotool key --clearmodifiers "$@"; sleep 0.4; }

for mode in light dark; do
  rm -rf "$TMP/data"
  mkdir -p "$TMP/data"
  FOLIO_DATA_DIR="$TMP/data" WAYLAND_DISPLAY= "$BIN" >"$TMP/app-$mode.log" 2>&1 &
  APP_PID=$!
  sleep 5
  WID="$(xdotool search --pid "$APP_PID" | tail -1)"
  xdotool windowmove "$WID" 0 0 windowsize "$WID" 1280 800
  sleep 1
  [ "$mode" = dark ] && key ctrl+shift+l
  shoot "sheet-$mode"
  # Double-click the "Website refresh" note to open its editor.
  xdotool mousemove 400 200 click --repeat 2 --delay 80 1; shoot "editor-$mode"
  key Escape
  # The pen: draw a loop on the sheet.
  key p
  xdotool mousemove 1000 640 mousedown 1
  for i in $(seq 0 24); do
    x=$(awk -v i="$i" 'BEGIN{printf "%d", 1000+60*cos(i/24*6.283)}')
    y=$(awk -v i="$i" 'BEGIN{printf "%d", 600+40*sin(i/24*6.283)}')
    xdotool mousemove "$x" "$y"; sleep 0.02
  done
  xdotool mouseup 1
  shoot "ink-$mode"
  kill "$APP_PID"; APP_PID=""
  sleep 0.5
done
echo "Saved to $OUT/"
