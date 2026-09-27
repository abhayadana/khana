# Khaṇa v0.6.5

This revision adds optional Ableton Link synchronization and fixes the bitmap
font's `V` glyph (the source of `XOICED` / `XOICES`).

## The font bug

Those labels were meant to be:

- `VOICED`
- `VOICES`

The old five-by-seven `V` glyph was accidentally drawn like an X. v0.6.2 fixes
the glyph.

## Ableton Link

Khaṇa keeps a unified synchronization layer:

```text
                 Internal clock
                      │
Khaṇa SyncTransport ──┤
                      │
                 Ableton Link
                      │
                 beat / tempo
                      │
        phrase playback + recording
```

The harmony, phrase, and voice engines do not depend on Link.

### Why Link is feature-gated

The normal build remains simple:

```bash
cargo run
```

To compile Link support:

```bash
cargo run --features ableton-link
```

The implementation uses `rusty_link 0.4.9`, which wraps Ableton's official
`abl_link` C API. The crate builds the Link C++ code and requires CMake and
libclang on Linux.

Ubuntu/Xubuntu prerequisites are typically:

```bash
sudo apt update
sudo apt install cmake libclang-dev build-essential
```

Your existing SDL2/ALSA development packages are still required.

### Strict quality check with Link

```bash
cargo fmt
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-features
```

## UI / interaction

While transport is stopped, press `M`.

The settings fields are now:

```text
TONIC → MODE → TEMPO → METER → SYNC → START/STOP SYNC
```

For `SYNC`:

```text
INTERNAL
LINK
```

For Link start/stop sharing:

```text
STARTSYNC OFF
STARTSYNC ON
```

If the binary was compiled without `--features ableton-link`, selecting Link
prints a clear message and remains on the internal clock.

The header uses minimal status:

```text
INT
LINK 0
LINK 2
```

where the number is the current count of other Link peers.

## Link behavior

### Tempo
In Internal mode, Khaṇa owns tempo.

In Link mode, the displayed BPM follows the Link session. Changing Khaṇa's tempo
submits a new tempo to the Link session.

### Beat / phase
Khaṇa's transport beat is read from Link while Link mode is active. Phrase
recording and phrase queue boundaries therefore use the shared Link beat grid.

Khaṇa still uses its own meter as the quantum for phrase/bar behavior. Link does
not replace Khaṇa's time-signature setting.

### Start / stop
Start/stop sharing defaults OFF.

With STARTSYNC OFF:
- Khaṇa can start/stop locally while remaining phase-synchronized to Link.

With STARTSYNC ON:
- Khaṇa submits start/stop changes to Link.
- Khaṇa also follows Link's shared playing state.

## Build scope

v0.6.2 is still a timing prototype. Four voices still sustain one note each per
harmonic attack. The next major musical milestone remains role-aware rhythmic
voice generation behind `VoiceEngine`.

## Licensing

Ableton Link is dual-licensed GPLv2+ / proprietary. `rusty_link` is GPLv2+
because it wraps Link. If Khaṇa is distributed with Link under an open-source
license, the combined distribution must be license-compatible; proprietary
distribution requires appropriate licensing from Ableton.


## Session-join behavior

When switching from Internal to Link, Khaṇa sets its private Link timeline to the
current local tempo *before* networking is enabled. It does not push that tempo
after joining. This follows the intended Link behavior that a new participant
should not hijack an existing jam's tempo.

Remote Link start/stop is edge-detected by the app. With STARTSYNC ON, a remote
start sounds Khaṇa's current realization and a remote stop sends MIDI Note Off
and cancels phrase playback.


## v0.6.3 Clippy cleanup

This maintenance revision removes four `clippy::needless_return` findings from
the feature-gated Ableton Link branches in `src/sync.rs`.

Behavior is unchanged. The Link-enabled v0.6.2 build already compiled and all
eight unit tests passed; this revision only makes the strict all-features Clippy
check clean under Rust 1.98.


## v0.6.4 MIDI reliability fix

The previous sustained-chord prototype still used `Density` as a deterministic
probability gate at each harmonic attack. That meant MIDI channels could be
skipped intentionally, and in some transitions every voice could be skipped.
This looked like unreliable MIDI output.

That behavior is removed.

For the current sustained-chord engine:

- every successful chord attack sends exactly four Note On messages,
- those messages use MIDI channels 1, 2, 3, and 4,
- every tracked sounding voice receives a matching Note Off before the next
  four-channel articulation,
- Spread/Color revoicing also produces a complete four-channel articulation
  while transport is running,
- Density remains editable but does not mute channels yet.

Density will become meaningful when the role-aware rhythmic performer engine is
implemented; it will control event frequency/occupancy rather than whether a
sustained chord voice exists.

Two unit tests now verify the exact status bytes:

```text
Note On : 90 91 92 93
Note Off: 80 81 82 83
```

for MIDI channels 1–4 respectively.


## v0.6.5 harmony / voicing correctness

Two musical correctness problems were found and fixed.

### Half-diminished sevenths

The diatonic diminished degree of each standard seven-note mode uses a
half-diminished seventh (0, 3, 6, 10), not a fully diminished seventh
(0, 3, 6, 9).

Example in C Ionian:

```text
B m7b5 = B D F A
```

The previous engine could produce B D F G#.

Khaṇa now has a distinct `HalfDiminished` quality and displays it as `M7B5`
because the bitmap font does not yet include the ø symbol.

### Essential-tone coverage

Previously, smooth voice-leading could choose any four pitches belonging to a
chord. That allowed musically weak realizations that duplicated one chord tone
while omitting an essential one.

Now:
- triads must contain root, third, and fifth (one tone may be doubled),
- sevenths must contain all four chord tones,
- ninths use a four-voice reduction containing root, third, seventh, and ninth;
  the fifth is the default omitted tone.

The engine still minimizes movement after enforcing those harmonic constraints.

### New tests

The test suite now additionally verifies:
- half-diminished interval construction,
- every diatonic seventh chord across all seven modes against stacked scale
  tones,
- full essential-tone coverage for major sevenths,
- G9 four-voice reduction as G/B/F/A.

### SDL X11 unknown-key console message

An occasional message such as:

```text
The key you just pressed is not recognized by SDL...
X11 KeyCode 248 ... KeySym 0x0
```

comes from SDL's X11 keyboard mapping layer when X11 reports a keycode with no
known keysym. It is not generated by Khaṇa's MIDI/harmony code and does not
indicate a MIDI failure. v0.6.5 leaves SDL warnings visible rather than globally
suppressing them, because other SDL warnings may be useful during Brick input
mapping.
