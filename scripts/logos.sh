#!/usr/bin/env bash
# Fetches the product logos the shapes panel offers: the Iconify "logos"
# set (SVG Logos by Gil Barbara, CC0-1.0), pinned, from npm. Writes
# assets/logos/logos.json.z: {name: [width, height, svg body]}, aliases
# resolved, zlib-compressed (read by src/logos.rs).
#
#   scripts/logos.sh
#
# The logos are CC0; the trademarks they show stay with their owners.

set -euo pipefail
cd "$(dirname "$0")/.."
VERSION=1.2.17
TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT
curl -sSfL "https://registry.npmjs.org/@iconify-json/logos/-/logos-$VERSION.tgz" -o "$TMP/logos.tgz"
tar xzf "$TMP/logos.tgz" -C "$TMP"
python3 - "$TMP/package/icons.json" assets/logos/logos.json.z <<'PY'
import json, sys, zlib
d = json.load(open(sys.argv[1]))
W, H = d.get("width", 16), d.get("height", 16)
out = {n: [i.get("width", W), i.get("height", H), i["body"]] for n, i in d["icons"].items()}
for n, a in d.get("aliases", {}).items():
    if a.get("parent") in out and n not in out:
        out[n] = out[a["parent"]]
data = json.dumps(out, separators=(",", ":"), sort_keys=True).encode()
open(sys.argv[2], "wb").write(zlib.compress(data, 9))
print(f"{len(out)} logos, {len(data)} bytes, {len(zlib.compress(data, 9))} compressed")
PY
