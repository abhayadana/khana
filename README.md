# Khaṇa v0.5.1

This prototype is the architectural reset discussed after reviewing the original
Norns **Chorder** project.

It proves the intended Khaṇa MVP skeleton:

```text
Safe / Colorful / Bold chord recommendation
                    ↓
              chord identity
                    ↓
        smooth four-voice voicing
                    ↓
          MIDI channels 1–4
                    ↓
       quantized phrase capture
                    ↓
          Phrase Editor timeline
```

## What changed from v0.4

v0.4 centered the application on a scale degree and a candidate mode. v0.5 makes
the central object a real `Chord`, separates chord identity from voicing, and
introduces four performer voices and semantic phrase events.

The prototype borrows these proven concepts from Chorder:

- triad / seventh / ninth chord construction,
- chord identity separate from voicing,
- nearest/smooth voice-leading as the default voicing behavior,
- chord context feeding downstream voices.

Chorder's richer voicing vocabulary (Drop-2, Drop-3, open, wide, quartal,
quintal, etc.) is intentionally *not* all ported yet. The Rust architecture leaves
room for those to become Khaṇa's Spread/Color vocabulary later.

Reference: https://github.com/abhayadana/chorder

## Performance screen

The Performance screen contains:

- current chord and elapsed harmonic time,
- three Safe / Colorful / Bold recommendation columns,
- Resolve / Neutral / Tension bias,
- four voice rows,
- Density / Dynamics / Spread / Color values,
- voice scope (ALL or one voice),
- phrase capture status.

Chord duration is not programmed. A chord lasts until you commit another chord.

### Desktop controls

| Key | Action |
|---|---|
| Left / Right | Move between Safe / Colorful / Bold |
| Up / Down | Move within a recommendation category |
| Z | Commit highlighted chord |
| X | Reroll recommendations |
| S | Surprise / broaden randomness |
| Q | Resolve bias |
| E | Neutral bias |
| W | Tension bias |
| C / V | Previous / next voice scope |
| T / G | Density - / + |
| Y / H | Dynamics - / + |
| U / J | Spread - / + |
| I / K | Color - / + |
| P | Quantized phrase record start/stop |
| N | Arm next phrase recording |
| Tab | Phrase Editor |
| Esc | Quit |

These are desktop emulation controls, not final Brick Pro bindings.

## Four voices

Voices use MIDI channels 1–4:

1. Bass
2. Inner
3. Texture
4. Melody

For v0.5:

- **Dynamics** controls MIDI velocity.
- **Spread** changes the target register spacing used by the smooth voicing engine.
- **Color** can enrich a triad/seventh realization to a seventh/ninth without
  changing the semantic chord stored in a phrase.
- **Density** determines whether that voice participates on a chord attack.

Later prototypes will replace the simple Density gate with real role-aware rhythm
generation, including ideas borrowed from Chorder's pattern library.

## Chord recommendations

v0.5 uses transparent prototype pools:

### Safe
Diatonic seventh chords in C major.

### Colorful
Diatonic ninths plus secondary dominants.

### Bold
Borrowed/chromatic colors.

Resolve/Neutral/Tension changes ranking independently of Safe/Colorful/Bold.

Reroll adds controlled deterministic variation. Surprise increases the weight of
random/novel candidates. This is deliberately simple; the API is ready for a more
contextual recommendation/scoring engine later.

## Phrase recording

Phrase recording is semantic, not frozen MIDI.

The recorder stores:

```text
ChordEvent {
    chord
    start_beat
    duration_beats
}
```

Recording start and stop are quantized to a 4-beat bar for this MVP.

`N` arms one additional phrase so that when the current phrase ends, the next
phrase begins at the same quantized boundary.

Phrases are auto-numbered `P01`, `P02`, etc.

## Phrase Editor

Press `Tab` to enter Screen 2.

The phrase browser is deliberately tiny. Most of the 1024×768 screen is reserved
for a horizontally zoomable chord timeline.

| Key | Phrase Editor action |
|---|---|
| Left / Right | Select previous/next chord event |
| A / L | Previous/next phrase |
| Z | Replace selected chord with a Safe suggestion |
| D | Duplicate selected chord |
| Backspace | Delete selected chord |
| F / R | Shorten/extend by 1 beat |
| - / + | Timeline zoom |
| Tab | Return to Performance |

No phrase names, sections, or song arranger are included in the MVP.

## Code organization

```text
src/
├── main.rs
├── clock.rs
├── input.rs
├── midi.rs
├── phrase.rs
├── ui.rs
├── voice.rs
└── music/
    ├── mod.rs
    ├── chord.rs
    ├── recommendation.rs
    └── voicing.rs
```

The music and phrase domain code has no dependency on SDL, ALSA, or NextUI.

## Quality checks

On Xubuntu:

```bash
cargo fmt
cargo clippy --all-targets --all-features -- -D warnings
cargo test
cargo run
```

This artifact environment does not contain Rust, so your Xubuntu machine is the
authoritative compiler/Clippy test.

## What to evaluate

The goal of v0.5 is architecture and interaction, not final theory.

Please pay attention to:

1. Does choosing chords from Safe / Colorful / Bold feel faster than v0.4?
2. Does Resolve / Neutral / Tension feel useful as an independent bias?
3. Do the four smoothly moving MIDI voices feel like one harmonic ensemble?
4. Does `P` quantized capture match how you imagine phrase recording?
5. Is the simplified Phrase Editor sufficient for navigating and correcting a
   captured progression?
6. Do Density / Dynamics / Spread / Color feel like the right four voice
   dimensions, even though rhythmic Density is still primitive?


## v0.5.1 Clippy cleanup

This maintenance revision addresses the first strict `cargo clippy -- -D warnings`
run on Rust 1.98:

- exercises the `Triad` extension in the runtime Safe vocabulary,
- removes the unused `VoiceParameters::normalized()` helper,
- adopts Clippy's collapsed conditional form in recorder/editor navigation,
- removes an unnecessary integer cast in the bitmap-font renderer.

No architecture or interaction model changed.
