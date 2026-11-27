# Khaṇa v0.11.4 — independent musical worker

Baseline: `5a25a8ca43e0f15688c65ed7f36fb137ae8bc675` (working MIDI, v0.11.3).
The preceding build was confirmed to deliver MIDI via QT-2host on Brick Pro.

## Runtime ownership

- The main thread owns SDL, physical input interpretation and drawing/present.
- The music worker owns the App, SyncTransport, phrase recorder/player,
  PerformerEngine and MidiEngine. Link and MIDI objects are created inside it.
- The worker services musical time at a nominal 1 ms cadence, waking sooner for
  requests. Existing beat-based pending-attack and gate logic is preserved.
- The UI requests an owned snapshot only when rendering is due. SDL draw and
  present never hold a music-state lock. UI actions are acknowledged in order,
  including the new input page. No stale page interpretation or unbounded
  request backlog is intended. No additional lookahead MIDI scheduler is added.
- Shutdown drops the sender, wakes the worker, releases owned notes and joins.
  Worker errors propagate to the UI or shutdown log. A blocked system call or
  driver can still stall shutdown; this is not a hard real-time implementation.
- Raw output discovery, recommendation computation, logging and bounded MIDI
  cleanup still run on the worker and may add timing jitter. Trace is best
  disabled for performance measurement. Desktop sequencer behavior is retained.

## Selecting MIDI output

With transport stopped, Select+X opens Settings (desktop M). Left/right moves
between fields; left from TONIC wraps to MIDI OUTPUT. Up/down cycles AUTO,
currently detected raw output devices, and OFF. A/Menu closes Settings.
The new selection is session-local; use KHANA_MIDI_PORT or
KHANA_MIDI_RAW_DEVICE in the launcher for a persistent starting selection.
The raw selector does not change desktop ALSA sequencer or explicit dry-run.

AUTO prefers USB over other candidates when USB is available, but refuses
ambiguous matches. Once connected, it pins identity until explicitly changed.
Explicit devices are selected by identity, not the changing card number alone.
Off suppresses discovery and delivery. Selection only changes while stopped.
The old output is released before switching; if it was already disconnected or
faulted, external panic may still be necessary. Recovery cleanup remains
restricted to the previously pinned device until a deliberate selection change.
Multiple identical ports with indistinguishable identities remain a limitation.

`--midi-list` reports product/card name, path, IN/OUT/UNKNOWN, USB/virtual/other
and output owner PID when available. Procfs information is a hint; actual open
is the writability test. VirMIDI is not classified as USB.

## Build and deployment

The committed `.cargo/config.toml` selects the NextUI cross linker. The launcher
now prepends `/usr/trimui/lib` to LD_LIBRARY_PATH and forwards arguments.
The raw-only build continues to omit the direct ALSA userspace dependency.

```sh
cargo fmt --all
cargo clippy --all-targets --all-features --locked -- -D warnings
cargo test --all-features --locked
cargo clippy --all-targets --no-default-features --locked -- -D warnings
cargo test --no-default-features --locked
./scripts/test-deployment-freshness.sh
./scripts/package-khana.sh
BRICK_HOST=root@YOUR_BRICK_IP ./scripts/deploy-khana.sh
```

For SSH diagnostics, use the native SDL library path before running directly:

```sh
cd /mnt/SDCARD/Tools/tg5040/Khana.pak
export LD_LIBRARY_PATH="/usr/trimui/lib${LD_LIBRARY_PATH:+:$LD_LIBRARY_PATH}"
./khana --midi-list
KHANA_MIDI_PORT=QT2host KHANA_MIDI_TRACE=1 ./khana --midi-test
```

Close the graphical app before headless audition. Reserve MIDI channels 1–4.

## Acceptance tests

1. Compare GUI layout and physical controls with v0.11.3; rapid screen changes
   followed by navigation should be handled in order.
2. Start playback via QT-2host and compare musical timing at 30 and 60 FPS.
   Switch screens, adjust macros, capture and play phrases during playback.
3. Inspect `MUSIC DIAG` poll/work maxima, expired attacks and lateness separately
   from `UI DIAG` render duration. Music tick count must continue increasing.
4. Stop transport: no owned MIDI notes. Start again: fresh attacks. Exit during
   playback: clean worker shutdown and note release.
5. With transport stopped, select OFF, AUTO, and the explicit port. Confirm
   connection state, no MIDI while OFF and correct destination after switching.
6. Unplug/reconnect the same MIDI device while playing. Confirm no UI freeze or
   stale attacks; verify the four channels are cleared on recovery.
7. Repeat with no hardware attached, desktop sequencer and raw-only dry-run.

This release intentionally does not implement MIDI clock out/in, a separate
lookahead scheduler, persistent Settings, or a multi-port/channel routing matrix.
