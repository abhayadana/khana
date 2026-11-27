#!/bin/sh
set -eu
PAK_DIR=$(CDPATH='' cd -- "$(dirname "$0")" && pwd)
LOG_DIR=${LOGS_PATH:-/mnt/SDCARD/.userdata/${PLATFORM:-tg5040}/logs}
mkdir -p "$LOG_DIR"
exec > "$LOG_DIR/Khana.txt" 2>&1
cd "$PAK_DIR"
export KHANA_BRICK=1
export KHANA_MIDI_BACKEND="${KHANA_MIDI_BACKEND:-raw}"
export BRICK_UI_FPS="${BRICK_UI_FPS:-30}"
# Preserve NextUI's SDL environment. Do not force an unverified video backend.
echo "Khana v0.11.3 NextUI launch"
date
printf 'PAK=%s PLATFORM=%s SDL_VIDEODRIVER=%s UI_FPS=%s\n' "$PAK_DIR" "${PLATFORM:-unknown}" "${SDL_VIDEODRIVER:-auto}" "$BRICK_UI_FPS"
if [ ! -x ./khana ]; then
    echo "ERROR: missing executable $PAK_DIR/khana" >&2
    exit 1
fi
exec ./khana "$@"
