# Khaṇa v0.11.3 — Raw MIDI platform

Apply to GitHub `main` at `9be5515a3637280abb33a4635d2a0058ccb29301`
(v0.11.2 MIDI and timing reliability). No intermediate patch is necessary.
This is a platform milestone, not a change to musical generation or controls.

## Why raw MIDI

SongVerk Studio (`romainguerif/M8Tape`, `app/sv_midi.c`), Twill Studio
(`LiminalKid/TwillStudio`, `src/platform/sdl2/SystemMidiLinux.cpp`) and
Brickio (`sydilaxe/Trimui-Brick-Resources`, `lib/brickio/midi.c`) use Linux
raw MIDI character devices directly. ChooChooTracker uses RtMidi/ALSA sequencer
and a separate timed output thread. Khaṇa keeps its desktop sequencer backend,
but the Brick build uses raw MIDI without the `midir` dependency. This is an
independent Rust implementation, not copied source.

## Build and test on Xubuntu

```sh
cargo fmt --all
cargo clippy --all-targets --all-features --locked -- -D warnings
cargo test --all-features --locked
cargo clippy --all-targets --no-default-features --locked -- -D warnings
cargo test --no-default-features --locked
KHANA_MIDI_DRY_RUN=1 cargo run --locked -- --midi-test
cargo run --no-default-features --locked -- --midi-list
./scripts/test-deployment-freshness.sh
./scripts/package-khana.sh
```

Default features include `alsa-sequencer`, preserving the desktop virtual
port. `--no-default-features` disables that backend and Ableton Link, but
retains raw MIDI and dry-run. The existing device build already uses this flag.
No new dependency is introduced. The lockfile retains optional dependencies.
The device build still needs its matching SDL2 toolchain/library; SDL itself
may have audio-related dependencies. This removes **Khaṇa's direct sequencer
library dependency**, not necessarily every indirect ALSA dependency.

## Device test

Install the Pak, attach a class-compliant USB MIDI device in host mode, then SSH:

```sh
cd /mnt/SDCARD/Tools/tg5040/Khana.pak
./khana --midi-list
cat /proc/asound/cards
ls -l /dev/snd
```

Use the actual card ID from the list and a receiving instrument at moderate
volume. A smoke test requires an explicit destination or dry-run:

```sh
KHANA_MIDI_PORT='actual card ID' KHANA_MIDI_TRACE=1 ./khana --midi-test
# Or, with the actual device path from --midi-list:
KHANA_MIDI_RAW_DEVICE=/dev/snd/midiC1D0 ./khana --midi-test
```

The four-channel audition should return ownership counts `[0, 0, 0, 0]`.
This confirms software ownership cleanup, not that the external synth received
or acted on every byte. Listen and verify on the receiver.

To play from NextUI, launch normally. On Brick, raw MIDI is the default.
If multiple outputs are present, set one unique name or exact device path
in `launch.sh` (not both). On Xubuntu use `KHANA_MIDI_BACKEND=raw` to test
raw selection by name; default desktop builds still use the sequencer.
`KHANA_MIDI_BACKEND=sequencer` requires the corresponding Cargo feature.
`KHANA_MIDI_DRY_RUN=1` overrides both. `--midi-list` reports names and paths
without opening SDL or sending notes. Direction information from procfs is a
hint; the nonblocking open must succeed. No kernel driver is installed or
USB mode changed by this release.

## Connection behavior

- No output: UI stays usable; the status says RAW WAIT. Retry about once a second.
- More than one eligible output in AUTO: wait for a unique selection rather than
  broadcasting or arbitrarily choosing a destination.
- The first connected device is pinned for the session by ALSA card ID, device
  number, USB ID if exposed, and sysfs location (device path fallback). It is
  not an authenticated device identity: identical devices at the same port
  can be indistinguishable. Missing or changed identifiers can prevent automatic
  recovery; restart with an explicit destination in that case.
- Partial writes retain the byte offset. A temporarily full kernel buffer leaves
  bytes queued in FIFO order. At 128 queued messages or a 30 ms stale head,
  disconnect and discard backlog rather than deliver stale notes. These are
  initial conservative limits for hardware testing.
- Following an output fault, if the pinned destination returns, send CC120
  (All Sound Off) on channels 1–4 before fresh notes. Reserve those channels for
  Khaṇa: this affects all sounds on them, including other senders. A synth that
  ignores CC120 may require its own panic. No old note events are replayed.
- Normal stop and quit attempt owned Note Offs with a bounded queue flush.
  No MIDI backend can guarantee cleanup after physical unplug or hard kill.
  Kernel close/driver behavior still needs device testing.

The performance screen uses the live connection label. The five-second log adds
raw transmitted-byte, queue-depth and fault counters. Active-note counts refer
to accepted/queued messages, not confirmation from an external instrument.

## Acceptance checks

1. Dry run: all four channels and polyphony; start/stop; no external output.
2. No device, then plug in: label and output recover without restarting.
3. Attach two outputs before launching: AUTO waits; a unique name works.
4. Unplug during a sustained note, reconnect the same device at the same port:
   no replay of old attacks, fresh attacks after recovery, no hang on exit.
5. Reconnect a different device instead: no silent destination switch.
6. Test several tempos at 30/60 FPS; compare polling/render diagnostics, expired
   attacks, raw queue and fault counts. A render stall can still delay delivery.
7. Test stop/exit while playing. Verify all four channels at the receiver.

## Deferred

The queue is FIFO, with enqueue timestamps for expiration, not a future-event
scheduler. Discovery, generation and delivery still share the application
thread. Move the musical engine and raw I/O to a worker after confirming the
backend works. On-device destination selection, persisted routing, MIDI clock
out/in, panic gesture, USB gadget MIDI and network MIDI are separate milestones.
