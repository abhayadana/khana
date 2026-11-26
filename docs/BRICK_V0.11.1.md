# Khaṇa v0.11.1 — Brick Pro foundation

Baseline: `abhayadana/khana` main, commit
`ad880bdfb6714f7c552ed81813a25420ad0d5202` (20261126 baseline, v0.9.1).
This cumulative source patch includes v0.10 and audited v0.11 musical changes
as well as the v0.11.1 platform changes. It does not require any intermediate
patches. The v0.11.1 platform work itself does not change the audited v0.11
musical engine. This release adapts Microloop's working principles:
raw SDL input, tap/hold and modifier state, a 0.12 stick dead zone, fullscreen
1024×768 display, 30/60 FPS UI, build identification and rebuild-before-delivery.
The Microloop M5.4.3 delivery and M5.7 controls patches were available; the full
latest M6.1 source and its SDK configuration were not. This is not a claim of
identical SDK setup or a verified Khaṇa executable on Brick.

## Apply and validate

On Xubuntu, from your Khaṇa repository (commit/back up local changes first):

```sh
git apply --check /path/to/khana-v0.11.1-github-baseline.patch
git apply /path/to/khana-v0.11.1-github-baseline.patch
cargo fmt --all
cargo clippy --all-targets --all-features --locked -- -D warnings
cargo test --all-features --locked
./scripts/test-deployment-freshness.sh
KHANA_MIDI_DRY_RUN=1 cargo run
```

The shell regression uses fake cross/SSH commands, real ZIP/tar utilities and
temporary files. It checks that a stale binary cannot be packaged or deployed,
a failed build does not contact the device, and a new deployment replaces an
old install. It does not require a Brick or Docker. Rust compilation, Clippy,
Rust unit tests and real device testing have not been run for this patch.

## Build and install

Use the `cross` workflow that Microloop uses, with the supplied
`Cross.toml` (aarch64-unknown-linux-gnu, tg5040 image). If your Microloop checkout
uses a customized SDK or image, compare its Cross.toml and build settings before
building. The actual target SDL2 and ALSA headers, libraries and dynamic loader
must be compatible with the installed NextUI version. Desktop needs SDL2 and
ALSA development packages; Ableton Link is optional and not built for Brick yet.

```sh
./scripts/build-device.sh
./scripts/package-khana.sh
```

Packaging always invokes the incremental build itself, so the first command is
optional. `./build-brick.sh` is a compatibility alias for packaging. The output
is `dist/Khana.pak.zip`, containing `Khana.pak/khana`, `launch.sh` and `pak.json`.
Extract the Pak folder into `Tools/tg5040/` on the NextUI SD card, or deploy by
SSH with the same user/address you use for Microloop:

```sh
BRICK_HOST=user@brick-ip ./scripts/deploy-khana.sh
```

Exit Khaṇa before deploying. The script rebuilds, stages via SSH/tar in the
same filesystem, checks files and swaps the installed Pak. It does not delete
anything outside its own Pak and temporary staging directory. Replacing the
Pak also replaces its launcher; keep custom launcher environment overrides in
your source launcher if you need them after deployment. Keep a backup if you
store local files in the Pak. The current application does not persist phrases.

## Physical controls (first hardware mapping)

Raw SDL button mapping follows the previously established Brick layout: B=0,
A=1, Y=2, X=3, L1=4, R1=5, Select=6, Start=7, Menu=8. The D-pad uses raw hats
as in the Microloop controls patch. Verify the actual events on your unit.
Raw axes 0/1 and 3/4 map to the two sticks; L2/R2 are digital axes 2/5. The
released/pressed endpoints are -32768/+32767; the implementation uses press and
release thresholds to suppress repeats. The selected raw joystick is logged;
`KHANA_JOYSTICK_INDEX` overrides the index in the list of opened raw joysticks.

| Physical control | Performance | Phrase Editor |
| --- | --- | --- |
| D-pad L/R | Recommendation column | Select chord event |
| D-pad U/D | Recommendation row | Previous/next edit grid |
| A | Commit | Play/queue |
| B | Reroll | Change selected chord |
| X | Surprise | Duplicate chord |
| Y | Neutral direction | Return to performance |
| L1/R1 | Previous/next voice scope | Previous/next phrase |
| L2/R2 | Resolve/tension | Shorten/extend on grid |
| Left stick X / Y | Density / Dynamics | — |
| Right stick X / Y | Spread / Color | — |
| Select+A | Record start/stop | — |
| Select+B | Arm next phrase | — |
| Select+X | Settings (transport stopped) | — |
| Select+Y | — | Delete chord |
| Select+L1/R1 | — | Zoom out/in |
| Start | Start/stop transport | Start/stop transport |
| Menu tap | Switch screen | Switch screen |
| Menu hold (700 ms) | Exit | Exit |

Settings: D-pad L/R selects field, U/D changes value; A, B or Menu tap closes.
Start closes settings and starts transport. Sticks are relative, not absolute:
right/up raises the value, left/down lowers it, centre does nothing. Stick
updates run on a 50 ms tick, not the render clock. Scope changes do not jump
values; a deflected stick intentionally adjusts the new scope on subsequent
ticks. Select suppresses stick edits. Keyboard mappings remain available.
Do not use Select+Menu: NextUI may handle that combination outside Khaṇa.
A raw joystick removal clears held state; reconnect requires app restart.

## Logs and runtime options

The launcher preserves NextUI's SDL environment (no forced `mali` driver).
It exports KHANA_BRICK=1 and defaults BRICK_UI_FPS to 30. Startup logs contain
build version, SDL video driver, output size, joystick capabilities and MIDI
port list. The version appears on both screens. Every five seconds logs report
maximum main-loop poll gap and owned MIDI notes per channel. This is a latency
proxy, not measured MIDI jitter. Rendering is paced independently, but still
runs on the main thread; blocking rendering can still delay MIDI. A dedicated
MIDI scheduler is deferred until the device bring-up is verified.

Log path: `$LOGS_PATH/Khana.txt`, with fallback
`/mnt/SDCARD/.userdata/tg5040/logs/Khana.txt` when PLATFORM is unset.
Edit the source Pak launcher to export these before `exec ./khana`, or export
them in a shell when launching manually:

| Variable | Use |
| --- | --- |
| BRICK_UI_FPS=30 or 60 | Compare display cadence |
| KHANA_INPUT_LOG=1 | Log raw input events; disable for timing tests |
| KHANA_JOYSTICK_INDEX=0 | Override auto-selected raw joystick |
| KHANA_MIDI_DRY_RUN=1 | Explicit UI-only mode; no messages sent |
| KHANA_MIDI_PORT=substring | Match one ALSA output; missing/ambiguous match errors |
| KHANA_MIDI_TRACE=1 | Print actual outgoing/intended MIDI bytes; disable for timing tests |

On Brick, MIDI initialization errors fall back to visible DRY RUN / NO OUTPUT
and log the reason. Otherwise an ALSA destination is selected (first non-Midi
Through port unless explicitly requested), or a labelled VIRTUAL port is made.
Neither a virtual port nor dry run establishes physical USB MIDI support.
Even dry-run mode needs the binary's linked libraries installed. On desktop,
MIDI initialization failures are errors unless dry run is explicitly enabled.
Normal exit and Rust error unwinding attempt to release all owned MIDI notes;
forced kills, unplugging a synth and backend failures cannot guarantee delivery.

## First device acceptance test

1. Start in dry run. Confirm the displayed version is V0.11.1. Verify all inputs
   with KHANA_INPUT_LOG=1, including D-pad hats, stick centres, both triggers,
   modifier release order, and clean Menu exit. Do not try Select+Menu.
2. Start transport; navigate recommendations, change harmony, adjust each macro
   and scope, record a phrase, edit duration/grid, queue playback and exit.
3. Compare 30 and 60 FPS with input/MIDI tracing off. Review poll-gap logs.
4. Plug in your USB MIDI synth before launch; disable dry run and check the port
   list, chosen destination and channels 1–4. Use a unique port substring if
   auto-selection is wrong. Check stopped/exit note counts, then restart.
5. Report cross-build failures, actual input events, and the startup log. If
   `libasound` or `/dev/snd/seq` is missing, the MIDI backend needs separate work.

The next milestone is v0.11.2: real MIDI discovery/delivery, disconnect handling,
measured timing and scheduler hardening, after platform results are available.
