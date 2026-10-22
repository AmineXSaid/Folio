#!/usr/bin/env bash
# Builds Folio's Windows installer: the cross-compiled .exe, the wizard's
# pictures (from the logo), then NSIS. Needs makensis (apt install nsis),
# ImageMagick and the x86_64-pc-windows-gnu target.
#   installer/make.sh   ->  dist/Folio-Setup-<version>.exe
set -euo pipefail
cd "$(dirname "$0")/.."
VERSION=$(grep -m1 '^version' Cargo.toml | cut -d'"' -f2)
cargo build --release --target x86_64-pc-windows-gnu
mkdir -p dist
cp target/x86_64-pc-windows-gnu/release/folio.exe dist/Folio.exe
x86_64-w64-mingw32-strip dist/Folio.exe
# The welcome page's side picture (164x314) and the header (150x57).
convert -size 164x314 gradient:'#f4588a'-'#b8285a' \
  \( assets/brand/folio-256.png -resize 104x104 \) -gravity center -geometry +0-30 -composite \
  -fill white -font DejaVu-Sans-Bold -pointsize 22 -gravity center -annotate +0+50 'folio.' \
  -type TrueColor BMP3:installer/side.bmp
convert -size 150x57 xc:white \( assets/brand/folio-128.png -resize 42x42 \) -gravity east -geometry +8+0 -composite \
  -type TrueColor BMP3:installer/header.bmp
makensis -V2 -DVERSION="$VERSION" installer/folio.nsi
ls -la "dist/Folio-Setup-$VERSION.exe"
