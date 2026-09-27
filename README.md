# Khaṇa v0.6.0

v0.6 turns the v0.5 architecture test into a more complete playable MVP
skeleton for the TrimUI Brick Pro's 1024×768 display.

## Major changes

### Real transport
`Space` starts/stops a shared musical transport. Stopping sends MIDI Note Off,
cancels phrase playback, and preserves beat position. Starting sounds the current
four-voice realization.

### Editable musical context
While transport is stopped, press `M` to edit the top bar:

- tonic/root: all 12 pitch classes,
- mode: Ionian, Dorian, Phrygian, Lydian, Mixolydian, Aeolian, Locrian,
- tempo: 30–300 BPM,
- meter presets: 2/4, 3/4, 4/4, 5/4, 6/8, 7/8, 9/8, 12/8.

Left/Right selects a setting. Up/Down changes it. `Z` or `M` exits.

The recommendation engine is now key/mode-aware rather than fixed to C major.

### Reroll versus Surprise
- **Reroll** changes ranking inside the ordinary harmonic vocabulary.
- **Surprise** additionally broadens the Colorful/Bold candidate pools before
  ranking them.

### Visible chord realization
The Performance screen now distinguishes:

```text
CURRENT C
VOICED CMAJ7
C2 E3 G3 B3
```

Each voice row also shows its current MIDI note.

### Voice-engine boundary
Harmonic state no longer talks directly to MIDI. The path is:

```text
Chord state
    ↓
VoiceEngine
    ↓
VoiceRender
    ↓
MidiEngine
```

This makes the next rhythmic-performer build possible without rewriting harmony
or phrase playback.

### Macro MIDI behavior
- Spread: recomputes voicing immediately.
- Color: recomputes the realized chord/voicing immediately.
- Density: changes participation on the next harmonic attack; no immediate
  retrigger.
- Dynamics: changes velocity on the next harmonic attack; no immediate
  retrigger.

Spread/Color update the displayed realization even while stopped. MIDI is resent
only if the transport is running.

### Semantic phrase playback
Screen 2 can now play a selected phrase.

- `Z` when idle: starts the selected phrase immediately.
- `Z` while another phrase is playing: queues the selected phrase for the next
  bar boundary.
- Screen 2 displays PLAY and QUEUE state.

Phrase playback feeds stored `ChordEvent`s back through the *current* VoiceEngine,
so a phrase can sound different after changing Spread/Color.

### Recording follows meter
Phrase recording still has no predetermined length. `P` toggles quantized
recording, with quantization set to one current bar. In 6/8, for example, the
bar is 3 quarter-note beats.

## Desktop controls

### Global

| Key | Action |
|---|---|
| Space | Transport start/stop |
| Tab | Performance / Phrase Editor |
| Esc | Quit |

### Performance

| Key | Action |
|---|---|
| Left/Right | Safe / Colorful / Bold |
| Up/Down | Candidate within category |
| Z | Commit highlighted chord |
| X | Reroll |
| S | Surprise |
| Q / E / W | Resolve / Neutral / Tension |
| C / V | Previous / next voice scope |
| T / G | Density - / + |
| Y / H | Dynamics - / + |
| U / J | Spread - / + |
| I / K | Color - / + |
| P | Quantized phrase record start/stop |
| N | Arm next phrase |
| M | Edit musical settings when stopped |

### Phrase Editor

| Key | Action |
|---|---|
| A / L | Previous / next phrase |
| Z | Play selected / queue selected |
| X | Change selected chord using Safe suggestion |
| Left/Right | Previous/next chord event |
| D | Duplicate event |
| Backspace | Delete event |
| F / R | Shorten/extend by one beat |
| - / + | Zoom |

## What remains intentionally primitive

The four MIDI voices still sustain one note each per harmonic attack. Density is
still a participation gate rather than a rhythmic density generator.

The next musical milestone should therefore be role-aware voice behavior:
Euclidean/patterned attacks, rests, passing tones, arpeggiation, and melody
behavior—built *behind* the new VoiceEngine interface.

## Quality checks

Run on Xubuntu:

```bash
cargo fmt
cargo clippy --all-targets --all-features -- -D warnings
cargo test
cargo run
```
