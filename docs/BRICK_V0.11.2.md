# Khaṇa v0.11.2 — MIDI bring-up and timing

Apply `khana-v0.11.2-brick-midi.patch` at GitHub commit
`30a2b5c041ed14558d30b3117a00bfb452f2800f` (`clippy patch`). No new Cargo
dependencies are required; Cargo.lock and Pak version advance together.
See [v0.11.1 controls and packaging](BRICK_V0.11.1.md) for the existing setup.

## Validate on the desktop

```sh
cargo fmt --all
cargo clippy --all-targets --all-features --locked -- -D warnings
cargo test --all-features --locked
cargo run --locked -- --midi-list
KHANA_MIDI_DRY_RUN=1 KHANA_MIDI_TRACE=1 cargo run --locked -- --midi-test
KHANA_MIDI_DRY_RUN=1 cargo run --locked --features ableton-link
```

The headless test bypasses SDL initialization, but the executable still needs
its linked runtime libraries. The test plays four sequential groups: Bass on
channel 1, three Chords notes on channel 2, Texture on channel 3, and Melody on
channel 4. It releases each group before the next and reports the owned-note
count. Dry-run logs are not evidence of sound reaching an external synth.

## Deploy to the Brick

```sh
./scripts/package-khana.sh
# Alternatively, when SSH is configured:
BRICK_HOST=user@brick-ip ./scripts/deploy-khana.sh
```

Connect a class-compliant USB MIDI device in the Brick's supported host mode.
Use the existing Microloop SSH workflow, and from the installed Pak directory
run the executable directly (not the normal NextUI launcher):

```sh
cd /mnt/SDCARD/Tools/tg5040/Khana.pak
./khana --midi-list
KHANA_MIDI_DRY_RUN=1 KHANA_MIDI_TRACE=1 ./khana --midi-test
```

The first command lists ALSA sequencer destinations and raw MIDI device nodes
independently. If the executable cannot load, inspect the missing shared
libraries before diagnosing MIDI. A raw node does not prove that it supports
output or that its USB driver is functional.

### ALSA sequencer (preferred)

Use an actual unique name substring returned by the list command. For example:

```sh
KHANA_MIDI_PORT='YOUR SYNTH NAME' KHANA_MIDI_TRACE=1 ./khana --midi-test
```

The audition requires an explicitly selected destination (or dry-run) so it
cannot accidentally play an arbitrary destination. Once it works, add the
appropriate `export KHANA_MIDI_PORT='YOUR SYNTH NAME'` to the installed
`launch.sh` before its final `exec`. Launch from NextUI normally.

### Raw MIDI (experimental fallback)

If the ALSA sequencer is unavailable but a raw MIDI node exists, try the node
identified by `--midi-list`. The path below is only an example:

```sh
KHANA_MIDI_RAW_DEVICE=/dev/snd/midiC1D0 KHANA_MIDI_TRACE=1 ./khana --midi-test
```

The raw backend writes MIDI 1.0 messages to the explicitly selected Linux
character device. It opens nonblocking on the supported Linux architectures.
It does not implement USB host support, ALSA driver installation, or a MIDI 2.0
UMP backend. It still requires kernel raw MIDI support and the app's existing
linked ALSA library. Raw MIDI output is not automatically selected; choose the
correct node and use it exclusively. On a failed write (including would-block),
Khaṇa reports the error and attempts cleanup rather than continuing with
uncertain note state. Unplug/replug or use the receiver's panic function if a
connection failure leaves an external note sounding. Hot-plug reconnection is
not implemented.

If the test succeeds, put `export KHANA_MIDI_RAW_DEVICE=/dev/snd/midiC1D0`
(with the actual discovered path) in the installed launcher instead of the
ALSA port selection. Do not set both selection variables. An explicit failed
selection now returns an error; it no longer silently turns into a dry-run.
With neither selection set, the v0.11.1 auto/virtual behavior is retained,
including the Brick dry-run fallback if automatic initialization fails.
Explicit `KHANA_MIDI_DRY_RUN=1` overrides both output selectors.

## Timing and acceptance

The five-second diagnostics now include maximum polling gap, maximum rendering
work including presentation, expired pending attacks, maximum observed pending
attack lateness in quarter-note beats, and currently owned notes per channel.
These are application-level measurements, not MIDI wire timestamps. They do not
count every skipped grid step; rendering and scheduling still share a thread.
Avoid enabling verbose MIDI tracing for timing comparisons.

- Play at 30 FPS, then 60 FPS (`BRICK_UI_FPS`). Compare poll and render maxima.
- Start/stop repeatedly and change chords, including during long gates.
- Set each scoped Density to zero while playing. No queued Note On should
  be sent after that voice is muted on the next poll.
- Check that an expired pending attack is dropped rather than briefly retriggered.
- Confirm receiver notes stop on transport stop and normal Menu-hold exit.
- Confirm the screen/log shows v0.11.2 and the actual destination.
- For a controlled disconnect test, disconnect with the receiver at low volume;
  report any errors and any note that remains sounding. An unplugged device
  cannot be guaranteed to receive cleanup messages.

The musical pattern vocabulary, chord scoring and handheld mapping are
unchanged. This is a source patch, not an ARM binary. Rust compilation, Clippy,
unit-test execution, actual USB MIDI delivery and timing measurements must be
validated on the development computer and Brick. An independent scheduler is
still deferred pending actual device measurements.
