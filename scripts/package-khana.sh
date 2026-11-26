#!/bin/sh
set -eu
ROOT=$(CDPATH='' cd -- "$(dirname "$0")/.." && pwd)
cd "$ROOT"
# Always call the incremental build. Never package merely because a binary exists.
./scripts/build-device.sh
BIN="$ROOT/target/aarch64-unknown-linux-gnu/release/khana"
[ -x "$BIN" ] || { echo "Missing executable: $BIN" >&2; exit 1; }
mkdir -p "$ROOT/dist"
TEMP=$(mktemp -d "$ROOT/dist/.khana.XXXXXX")
trap 'rm -rf "$TEMP"' EXIT
trap 'exit 1' HUP INT TERM
mkdir -p "$TEMP/Khana.pak"
cp packaging/NextUI/Tools/tg5040/Khana.pak/launch.sh packaging/NextUI/Tools/tg5040/Khana.pak/pak.json "$TEMP/Khana.pak/"
cp "$BIN" "$TEMP/Khana.pak/khana"
chmod +x "$TEMP/Khana.pak/khana" "$TEMP/Khana.pak/launch.sh"
(cd "$TEMP" && zip -q -r Khana.pak.zip Khana.pak)
mv "$TEMP/Khana.pak.zip" "$ROOT/dist/Khana.pak.zip"
echo "Packaged $ROOT/dist/Khana.pak.zip"
