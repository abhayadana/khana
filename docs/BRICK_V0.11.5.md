# Khaṇa v0.11.5 MIDI Clock Out and live tempo

Baseline `395a904530a25ce9c7d079a589e43788326eea77` on GitHub main.

## Settings and controls

From Performance, hold Select and press X on Brick or press M on desktop.
With playback stopped, Settings opens on Tonic. Left/right selects a field
and up/down changes its value.

| From Tonic | Field | Action |
|---|---|---|
| Left once | MIDI OUTPUT | Up/down selects the raw destination or OFF. |
| Left twice | MIDI PANIC | A on Brick or Z on desktop sends on channels 1–4. |
| Left three times | CLOCK OUT | Up/down toggles MIDI Clock Out on the selected output. |

B or a Menu tap closes Brick Settings from any field. A closes ordinary
Settings and sends panic when MIDI PANIC is focused. On desktop, use M to
cancel or close Settings. Panic releases owned notes and sends All Sound Off
and All Notes Off (CC120/CC123) on channels 1–4. Stop first if playback is
still running. An unavailable output cannot receive the panic messages.

During internal-clock playback, Select+X or M opens live tempo on Performance.
Up/down changes the tempo by 1 BPM from 30 to 300 without moving the beat.
Close with A, B, or a Menu tap on Brick or Z/M on desktop. Start/Space stops
transport and closes live tempo. Link tempo edits remain in stopped Settings.

## Clock and transport

- Clock Out defaults to OFF at launch. The setting applies to the current
  session and uses the same MIDI destination as performed notes. Selecting raw
  MIDI OFF suppresses both. Dry-run trace shows intended bytes.
- The music worker derives `F8` at 24 pulses per quarter note from the active
  beat timeline. A normal stop or app exit sends `FC` for an active outgoing
  transport. Clock pulses run during playback.
- On internal clock, the first launch sends `FA` and an uninterrupted stop
  and resume on the same destination sends `FB`. The first clock follows on the
  next pulse boundary. Khaṇa's Start button retains its local stop/resume
  behavior. If a new `FA` is needed at a nonzero local beat, the receiver
  starts its sequence from zero. Song Position Pointer is a future feature.
- In a Link-enabled desktop build, the outgoing transport sends a fresh `FA`
  ahead of the next bar boundary in Khaṇa's meter, with the first `F8` at that
  boundary. Khaṇa's own Link playback behavior continues as before. Standard
  Brick packaging builds without Link. MIDI Clock In is also future work.
- A raw-port fault interrupts clock delivery. Reconnecting during playback
  leaves clock transport waiting for a deliberate stop and start. Existing
  raw note routing and pinned-device cleanup still apply. A disconnected
  receiver may need to be stopped locally.
- The worker skips missed pulses instead of sending a burst. The raw queue
  retains at most one pending `F8` during backpressure. The worker logs skipped
  pulses and uses a fresh `FA` on the next internal launch after a skipped
  pulse. Use `MIDI CLOCK OUT` diagnostics to spot stalls. The raw queue spaces
  the first clock after a delayed Start/Continue write. These checks describe
  scheduling and writes, while wire-level jitter requires hardware measurement.

## Acceptance checks

1. Build with `./scripts/package-khana.sh` and launch from NextUI. Confirm
   `0.11.5`, CLOCK OUT OFF, and the original four-role output on QT-2host.
2. Switch to CLOCK OUT and enable it. Use a MIDI monitor or a receiver set to
   external MIDI sync. Verify `FA`, `F8` spacing, `FC`, then `FB` on an
   uninterrupted resume. At 120 BPM expect about 20.8 ms between pulses.
   A normal exit from playback should also send `FC`.
3. Change live tempo during playback. Verify the BPM display, receiver tempo,
   and continuity of the performance. Compare 30 and 60 FPS with tracing off
   while changing screens, chord choices, and macros.
4. Unplug and reconnect QT-2host during playback. Check the clock status and
   verify a fresh `FA` after stop/start. Select MIDI OUTPUT OFF and verify
   silence. Restore a destination and use MIDI PANIC while stopped to verify
   CC120/123 on channels 1–4.
5. On desktop, test default ALSA, raw-only, and `--features ableton-link`.
   In a real Link session verify BPM changes, first-clock alignment to the
   configured meter, and shared start/stop with both peers opted in.

For a desktop trace without external MIDI, launch with
`KHANA_MIDI_DRY_RUN=1 KHANA_MIDI_TRACE=1 cargo run --locked --no-default-features`.
Enable Clock Out in stopped Settings, then start playback.
