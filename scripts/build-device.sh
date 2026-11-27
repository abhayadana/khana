#!/bin/sh
set -eu
cd "$(dirname "$0")/.."
# Microloop's confirmed workflow uses cross. Keep the image in Cross.toml
# configurable; SDL2 target development libraries must be present.
# --no-default-features excludes the desktop ALSA sequencer backend.
command -v cross >/dev/null 2>&1 || { echo 'Install cross: cargo install cross' >&2; exit 1; }
cross build --locked --release --target aarch64-unknown-linux-gnu --no-default-features
