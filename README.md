# Khaṇa v0.2.0

The first real Khaṇa prototype.

This version changes the starter from "keyboard keys directly equal MIDI notes" to:

```text
input -> musical intention -> scale degree -> MIDI note
```

That separation is the foundation for later tension/resolution, harmony, voicing, phrase memory, and generative behavior.

## What it does

- 1024x768 SDL2 UI
- C major scale
- seven selectable scale degrees
- Left / Right changes the selected degree
- Z plays the selected degree
- MIDI is generated through a virtual ALSA MIDI port named `Khana MIDI`
- note-off is sent when Z is released
- scale degree selection wraps from 1 <-> 7
- basic music logic has unit tests
- source is split into `input`, `music`, `midi`, and `ui` modules

## Run

```bash
cd khana-v0.2.0
cargo run
```

## Controls

- Left Arrow: previous scale degree
- Right Arrow: next scale degree
- Z: play selected degree
- Esc: quit

## MIDI mapping

In C major:

| Degree | Note | MIDI |
|---|---|---:|
| 1 | C4 | 60 |
| 2 | D4 | 62 |
| 3 | E4 | 64 |
| 4 | F4 | 65 |
| 5 | G4 | 67 |
| 6 | A4 | 69 |
| 7 | B4 | 71 |

## Verify MIDI

While Khaṇa is running:

```bash
aconnect -l
aseqdump -l
```

Find `Khana MIDI`, then monitor it with:

```bash
aseqdump -p CLIENT:PORT
```

## Run tests

```bash
cargo test
```

The tests currently confirm the C-major degree-to-MIDI mapping and wraparound behavior.

## Source layout

```text
src/
├── main.rs   application loop and orchestration
├── input.rs  keyboard -> logical Khaṇa actions
├── music.rs  musical state and scale-degree logic
├── midi.rs   MIDI output
└── ui.rs     SDL2 rendering
```

## Next milestone

The next version should add a first meaningful performance model rather than more infrastructure. A good candidate is a distinction between stable, transitional, and tense degrees, with explicit "move", "hold", and "resolve" actions.
