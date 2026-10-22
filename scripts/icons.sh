#!/usr/bin/env bash
# Renders the logo SVGs in assets/brand to PNGs and the Windows .ico.
#
#   scripts/icons.sh
#
#   folio-{512,256,128,64}.png   the logo with its shadow (README, sidebar)
#   assets/folio.ico             16-256 px (21 sizes); the tile fills the square as
#                                Chrome's and Teams' icons do. 16-32 px use
#                                folio-icon-small.svg (heavier, fewer lines).
#
# Each size is drawn at that size by headless Chromium, not shrunk from a
# big one. Needs Chromium (CHROME=...) and ImageMagick.

set -euo pipefail
cd "$(dirname "$0")/.."
CHROME="${CHROME:-$(command -v chromium || command -v google-chrome || echo /opt/pw-browsers/chromium-1194/chrome-linux/chrome)}"
B="$PWD/assets/brand"
TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT

render() { # svg size out
  printf '<html><body style="margin:0;background:transparent"><img src="file://%s" width="%s" height="%s" style="display:block"></body></html>' "$1" "$2" "$2" >"$TMP/p.html"
  "$CHROME" --headless --no-sandbox --disable-gpu --hide-scrollbars --force-device-scale-factor=1 \
    --default-background-color=00000000 --window-size=600,700 \
    --screenshot="$TMP/s.png" "file://$TMP/p.html" >/dev/null 2>&1
  convert "$TMP/s.png" -crop "${2}x${2}+0+0" +repage "$3"
}

for n in 512 256 128 64; do render "$B/folio-logo.svg" $n "$B/folio-$n.png"; done
icons=()
# Every size Windows asks for at 100-250% display scaling, each drawn at
# that size: a missing one gets stretched up from a smaller one, and looks
# pixelated in Explorer and on the taskbar.
for n in 256 192 160 144 128 112 96 80 72 64 60 56 48 40 36; do render "$B/folio-icon.svg" $n "$TMP/i$n.png"; icons+=("$TMP/i$n.png"); done
for n in 32 30 28 24 20 16; do render "$B/folio-icon-small.svg" $n "$TMP/i$n.png"; icons+=("$TMP/i$n.png"); done
cp "$TMP/i256.png" "$B/folio-icon-256.png"
cp "$TMP/i32.png" "$B/folio-icon-32.png"
convert "${icons[@]}" "$PWD/assets/folio.ico"
rm -f "$B/folio.ico" "$B/folio-32.png"
identify "$PWD/assets/folio.ico" | awk '{print $3}' | tr '\n' ' '; echo
