#!/bin/sh
# Exercises package and SSH delivery with local stubs only, no device or SDK.
set -eu
ROOT=$(CDPATH='' cd -- "$(dirname "$0")/.." && pwd)
TEMP=$(mktemp -d)
trap 'rm -rf "$TEMP"' EXIT
trap 'exit 1' HUP INT TERM
mkdir -p "$TEMP/repo/scripts" "$TEMP/bin" "$TEMP/device"
cp "$ROOT/scripts/build-device.sh" "$ROOT/scripts/package-khana.sh" "$ROOT/scripts/deploy-khana.sh" "$TEMP/repo/scripts/"
cp -R "$ROOT/packaging" "$TEMP/repo/"
export TEST_ROOT="$TEMP/repo" TEST_DEVICE="$TEMP/device"
export PATH="$TEMP/bin:$PATH" BRICK_HOST=mock-brick
cat > "$TEMP/bin/cross" <<'STUB'
#!/bin/sh
set -eu
[ "$PWD" = "$TEST_ROOT" ]
[ "${FAIL_BUILD:-0}" != 1 ] || exit 42
printf 'build\n' >> "$TEST_ROOT/build.calls"
mkdir -p target/aarch64-unknown-linux-gnu/release
printf fresh > target/aarch64-unknown-linux-gnu/release/khana
chmod +x target/aarch64-unknown-linux-gnu/release/khana
STUB
cat > "$TEMP/bin/ssh" <<'STUB'
#!/bin/sh
set -eu
[ "$1" = mock-brick ]
shift
printf 'ssh\n' >> "$TEST_ROOT/ssh.calls"
# Execute the real deployment script with only its remote root redirected.
script=$(printf '%s' "$1" | sed "s|/mnt/SDCARD|$TEST_DEVICE|g")
sh -c "$script"
STUB
chmod +x "$TEMP/bin/cross" "$TEMP/bin/ssh"
mkdir -p "$TEST_ROOT/target/aarch64-unknown-linux-gnu/release"
BIN="$TEST_ROOT/target/aarch64-unknown-linux-gnu/release/khana"
printf stale > "$BIN"
chmod +x "$BIN"
# Run from another directory: scripts must resolve their own repository root.
cd "$TEMP"
"$TEST_ROOT/scripts/package-khana.sh"
[ "$(unzip -p "$TEST_ROOT/dist/Khana.pak.zip" Khana.pak/khana)" = fresh ]
printf stale-again > "$BIN"
"$TEST_ROOT/scripts/deploy-khana.sh"
INSTALLED="$TEST_DEVICE/Tools/tg5040/Khana.pak/khana"
[ "$(cat "$INSTALLED")" = fresh ]
[ "$(wc -l < "$TEST_ROOT/build.calls")" -eq 2 ]
# A failed build must not contact the device or overwrite the installed binary.
if FAIL_BUILD=1 "$TEST_ROOT/scripts/deploy-khana.sh"; then
    echo 'Deployment unexpectedly succeeded after build failure' >&2
    exit 1
fi
[ "$(wc -l < "$TEST_ROOT/ssh.calls")" -eq 1 ]
[ "$(cat "$INSTALLED")" = fresh ]
# Replacing an existing installation must also refresh the binary.
printf old-install > "$INSTALLED"
"$TEST_ROOT/scripts/deploy-khana.sh"
[ "$(cat "$INSTALLED")" = fresh ]
echo 'PASS: package/deploy freshness, failure isolation and replacement'
