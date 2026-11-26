#!/bin/sh
set -eu
ROOT=$(CDPATH='' cd -- "$(dirname "$0")/.." && pwd)
cd "$ROOT"
BRICK_HOST=${BRICK_HOST:-}
case "$BRICK_HOST" in ''|-*) echo 'Set BRICK_HOST to user@brick-ip' >&2; exit 2;; esac
# Require a successful current build/package before contacting the device.
./scripts/package-khana.sh
TEMP=$(mktemp -d)
trap 'rm -rf "$TEMP"' EXIT
trap 'exit 1' HUP INT TERM
unzip -q "$ROOT/dist/Khana.pak.zip" -d "$TEMP"
# Stage on the SD card, verify the three files, then swap directories. Keep the
# old installed Pak until upload succeeds. Exit Khaṇa before deploying.
tar -C "$TEMP" -cf "$TEMP/khana.tar" Khana.pak
ssh "$BRICK_HOST" 'set -eu
base=/mnt/SDCARD/Tools/tg5040
mkdir -p "$base"
stage=$(mktemp -d "$base/.khana-stage.XXXXXX")
trap '\''if [ -e "$stage/previous" ] && [ ! -e "$base/Khana.pak" ]; then mv "$stage/previous" "$base/Khana.pak" || exit 1; fi; rm -rf "$stage"'\'' EXIT
trap '\''exit 1'\'' HUP INT TERM
tar -xf - -C "$stage"
test -s "$stage/Khana.pak/khana"
test -s "$stage/Khana.pak/launch.sh"
test -s "$stage/Khana.pak/pak.json"
chmod +x "$stage/Khana.pak/khana" "$stage/Khana.pak/launch.sh"
if [ -e "$base/Khana.pak" ]; then mv "$base/Khana.pak" "$stage/previous"; fi
if ! mv "$stage/Khana.pak" "$base/Khana.pak"; then
    if [ -e "$stage/previous" ]; then mv "$stage/previous" "$base/Khana.pak"; fi
    exit 1
fi
printf "Installed %s/Khana.pak\n" "$base"' < "$TEMP/khana.tar"
