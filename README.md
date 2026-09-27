# Khaṇa v0.7.0

v0.7 is the harmonic-engine overhaul.

The design goal is that **Safe / Colorful / Bold are three levels of
adventurousness inside the same evolving musical story**. Chord selection is
now driven primarily by progression context and harmonic function. Smooth
voice-leading happens after chord selection.

## Recommendation pipeline

```text
last 2–4+ played chords
        +
tonic / mode
        +
Resolve / Neutral / Tension
        ↓
broad candidate generation
        ↓
progression + function scoring
        ↓
contextual novelty classification
        ↓
SAFE / COLORFUL / BOLD
        ↓
selected chord
        ↓
four-voice VoicingEngine
```

### Candidate sources

The engine generates candidates from:

- diatonic harmony,
- common functional continuations,
- secondary dominants,
- borrowed/modal-interchange harmony,
- chromatic mediants,
- diminished approaches,
- tritone-related dominants,
- a small additional rare-path vocabulary when Surprise is active.

These sources do not directly dictate the final column. A contextual novelty
score assigns each candidate to Safe, Colorful, or Bold.

## Ranking priorities

The current implementation uses the following approximate hierarchy:

```text
38% progression context
22% harmonic/function coherence
18% Resolve/Neutral/Tension intent
10% source prior
 7% phrase/repetition history
 5% chord-level voice-leading hint
```

The 5% voice-leading term only rewards shared essential pitch classes. It is a
tie-breaker. Actual four-part voice-leading happens *after* the chord is chosen.

## Progression memory

Khaṇa keeps the last eight actually played/selected chords and uses the recent
history to recognize patterns such as:

```text
ii → V → I
IV → V → I
vi → ii → V
I → vi → ii → V
```

Dominant chords also receive a strong generic resolution relationship down a
fifth / up a fourth, so secondary dominants have meaningful implied targets.

Examples tested in C:

```text
Dm7 → G7 + Resolve  => Cmaj7 ranks first in Safe
Cmaj7 → E7          => Am7 ranks first in Safe
Cmaj7 → Am7 → Dm7  => G7 ranks first in Safe
```

## Safe / Colorful / Bold

The categories are contextual rather than permanent chord lists.

- **Safe**: low-novelty candidates with strong progression/function precedent.
- **Colorful**: secondary dominants, borrowed harmony, or other moderately
  surprising moves with a clear harmonic explanation.
- **Bold**: chromatic-mediant, diminished, tritone, and rarer coherent paths.

A chord's novelty is reduced when the progression strongly supports it or when
that harmonic vocabulary has already appeared recently.

## Reroll versus Surprise

- **Reroll** changes the small stochastic ordering component while preserving
  candidate vocabulary and novelty thresholds.
- **Surprise** adds rarer coherent candidate generators and gives non-diatonic
  paths a modest ranking boost. It does not mean random chromatic chords.

## Recommendation explanation

The Performance screen now shows a compact `WHY` label for the highlighted
recommendation, such as:

```text
WHY II TO V
WHY DOM RESOLVE
WHY BORROWED
WHY CHROM MEDIANT
```

This is intentionally visible during prototype tuning.

## Stable chord identity

Color no longer changes the chord itself.

If you select:

```text
C
```

Khaṇa stores, displays, voices, records, and plays back **C**. It does not
silently convert the chord into Cmaj7 or Cmaj9.

For v0.7:

- Spread still revoices the current chord immediately while playing.
- Dynamics applies on the next attack.
- Density remains reserved for the upcoming rhythmic performer engine.
- Color remains editable but does not alter MIDI yet. In v0.8 it will become a
  role-sensitive performer/decorative parameter.

## Display / spelling

The large CURRENT chord and the four displayed note names now describe the same
harmonic object.

Pitch spelling uses the current tonal context. Common borrowed relationships in
C, for example, display as:

```text
Eb
Ab
Bb
```

rather than D#, G#, A#.

This is a pragmatic contextual spelling layer rather than a complete notation
engine, but it makes borrowed harmony substantially clearer.

## Existing v0.6 functionality retained

- reliable Note On/Off on MIDI channels 1–4,
- four-voice essential-tone voicing,
- internal transport,
- optional Ableton Link,
- optional Link start/stop sync,
- editable tonic/mode/tempo/meter,
- semantic phrase recording,
- phrase playback and queueing,
- Phrase Editor.

## Build

Normal:

```bash
cargo fmt
cargo clippy --all-targets -- -D warnings
cargo test
cargo run
```

With Ableton Link:

```bash
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-features
cargo run --features ableton-link
```

## Next milestone

v0.8 should build role-aware rhythmic performers behind the existing
`VoiceEngine`, making Density and Color musically active without changing the
new progression engine.
