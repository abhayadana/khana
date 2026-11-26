# Khaṇa v0.9.1

v0.9 changes the four-performer architecture to:

```text
Bass | Chords | Texture | Melody
```

and changes pattern selection from:

```text
Density -> one fixed pattern
```

to:

```text
Density -> behavior family
        -> eligible sibling variants
        -> continuity / age / history / deterministic variation
        -> one evolving pattern
```

The v0.7 progression recommendation engine is otherwise retained so harmony
recommendation can continue to be tuned independently from performer behavior.

## Why this build exists

The previous `Inner` role was too abstract. A basic progression should be
clearly audible from **Bass + Chords alone**. Texture and Melody should enrich
that foundation rather than being required to reconstruct the harmony.

The new architecture is:

```text
progression engine
      ↓
selected active harmony
      ↓
upper chord voicing
      ↓
┌────────┬────────┬─────────┬────────┐
│ Bass   │ Chords │ Texture │ Melody │
│ mono   │ poly   │ mono    │ mono   │
└────────┴────────┴─────────┴────────┘
      ↓
polyphonic-safe MIDI ownership
```

## Four performers

### Bass — MIDI channel 1

Bass remains the harmonic foundation. Density moves it among three families:

```text
FOUNDATION  < 0.32
MOVING      0.32–0.72
CONNECT     >= 0.72
```

Sibling variants include:

```text
FOUNDATION
- root hold
- root pulse
- root octave

MOVING
- root/fifth
- root/fifth/octave
- broken bass

CONNECT
- scale approach
- chromatic approach
- walking fragment
```

Bass strongly acknowledges chord changes and is forced to state the new harmony
when a chord is selected.

### Chords — MIDI channel 2, polyphonic

Chords now **actually plays the selected chord** as a simultaneous four-note
voicing. Density moves it among:

```text
SUSTAIN  < 0.30
COMP     0.30–0.75
ACTIVE   >= 0.75
```

Sibling variants include:

```text
SUSTAIN
- sustained chord
- single pulse
- two-pulse support

COMP
- backbeat
- offbeat
- Charleston-type syncopation
- anticipation/response

ACTIVE
- syncopated A
- syncopated B
- active stabs
```

The rhythmic pattern and the chord voicing are separate systems. A COMP rhythm
can therefore keep its identity while the voicing engine smoothly changes the
actual chord tones.

A newly selected chord forces a Chords attack even if the currently selected
comp pattern would otherwise wait until later in the bar. This keeps harmonic
feedback immediate during improvisation.

### Texture — MIDI channel 3

Texture is deliberately more spacious. Density selects:

```text
SPARSE  < 0.38
MOTION  0.38–0.75
ACTIVE  >= 0.75
```

Sibling variants include:

```text
SPARSE
- isolated upper tone
- delayed echo
- chord sparkle

MOTION
- two-note response
- slow arpeggio
- staggered extensions

ACTIVE
- shimmer
- repeating arpeggio
- extension figure
```

Unlike Bass and Chords, Texture can deterministically omit some candidate hits.
This preserves its role as atmosphere/color rather than another constant rhythm
section.

### Melody — MIDI channel 4

Melody remains phrase-generated rather than mask-generated. Density selects:

```text
MOTIF     < 0.30
PHRASE    0.30–0.76
ORNAMENT  >= 0.76
```

Each family contains sibling phrase behaviors, including sparse motifs,
developed/answering phrases, ornamented phrases, denser passing motion, and
chromatic phrase variants.

Melody still uses structural tones, phrase contour, a peak, fills, neighbors,
and resolving chromatic approaches.

## Pattern family vs pattern variant

The Performance screen shows only the **family**:

```text
BASS     MOVING
CHORDS   COMP
TEXTURE  MOTION
MELODY   PHRASE
```

Internally each family contains several variants.

This is intentional: Khaṇa should expose a small number of playable musical
controls while retaining richer behavior internally.

## How a variant is chosen

At a natural renewal boundary, each eligible sibling receives a score based on:

```text
density fit
+ continuity with current pattern
+ repeat tendency
+ pattern age
- immediate-history penalty
+ small seeded variation
```

Continuity is strong. A newly selected pattern usually survives long enough to
become recognizable. As it ages, Khaṇa gradually becomes more willing to move
to a sibling variant.

The deterministic variation term is seeded. Repeated runs are therefore
inspectable/testable rather than depending on opaque random state.

## Renewal boundaries

Pattern identity is reconsidered at different musical rates:

```text
Bass     one chord-relative bar
Chords   one bar
Texture  two bars
Melody   two-bar phrase
```

A large Density move can change family immediately, but otherwise pattern
identity develops at these musical boundaries rather than being reconsidered on
every note.

## Coherent patterns rather than punched masks

Bass and Chords now play a selected variant as a coherent rhythm. Space no
longer randomly deletes their defining hits.

Texture retains deterministic thinning because leaving air is central to that
role.

Melody creates rests at phrase-planning time.

## Chord voicing theory

The Chords performer uses the existing voice-leading engine in a more natural
place: it now answers **how should the selected harmony be voiced upstairs?**

When Bass is active, the Chords voicing is allowed to omit the root and instead
prioritize the tones that define chord quality and extension.

### Triad with Bass

At minimum, upper Chords must preserve:

```text
third + fifth
```

The fourth Chords note may be a root or a doubling chosen by the voicing engine.

### Seventh chord with Bass

Upper Chords must preserve:

```text
third + fifth + seventh
```

### Ninth chord with Bass

Upper Chords must preserve:

```text
third + seventh + ninth
```

The fifth and/or root are more expendable upstairs because Bass supplies the
foundation.

If Bass Density is zero, the Chords voicing again requires the root so the
harmony remains self-sufficient.

The voicing engine favors smooth motion from the prior chord and keeps the
Chords register above the Bass range.

## Chords Color

Color still does **not** change semantic chord identity.

In v0.9, the Chords performer primarily states the selected chord itself. Future
builds can use Chords Color for suspensions, transient add-tones, and resolving
upper tensions without changing the stored chord name.

This restraint is deliberate for the first polyphonic Chords implementation.

## Polyphonic MIDI ownership

The MIDI layer no longer assumes one note per channel.

Each channel owns a 128-note bitset. Chords can therefore own several notes at
once:

```text
ch 1 Bass      { C2 }
ch 2 Chords    { E3, G3, B3, C4 }
ch 3 Texture   { D5 }
ch 4 Melody    { G5 }
```

A Note Off only clears the exact note it owns. Rearticulating one Chords pitch
cannot accidentally erase the other chord tones.

The current Chords pattern still rearticulates its full chord on a rhythmic
attack. Common-tone carry across harmony changes is a future refinement rather
than being mixed into this first polyphonic build.

## Performance display

Each row now shows the family, not the hidden exact variant:

```text
1 BASS C2       RST CHD   MOVING
2 CHORDS 4N     RST BAR   COMP
3 TEXTURE D5    RST 2BAR  MOTION
4 MELODY G5     RST PHR   PHRASE
```

`4N` means four chord notes are currently sounding on the Chords channel.

The upper `ANCHOR` display is now best understood as the current Chords voicing
and register reference used by the other performers.

## Defaults

The new defaults intentionally start in moderate regions:

```text
Bass     Density 0.56 -> MOVING
Chords   Density 0.52 -> COMP
Texture  Density 0.40 -> MOTION
Melody   Density 0.62 -> PHRASE
```

This should make the initial sound recognizably harmonic without being a static
pad or an overly dense ensemble.

## What this build does not yet do

- Chords does not yet preserve common tones across chord changes; the current
  harmony change safely releases prior notes and states the new chord.
- Bass connecting patterns cannot truly target an *unknown future chord* yet.
  Once a next chord is queued/known, that context can be added to bass-pattern
  scoring.
- Pattern selection does not yet inspect cadence/function directly. The family
  architecture is designed so harmonic-context scoring can be added without
  changing the public control model.
- Exact pattern browsing is intentionally absent from the performance UI.

## Tests added/changed

v0.9 adds tests for:

- default Bass / Chords / Texture / Melody ordering,
- Chords-compatible upper voicing requirements,
- Density selecting a family rather than one fixed pattern,
- multiple sibling variants existing within COMP,
- pattern state remaining inside its selected family,
- four simultaneous Chords Note On events,
- role-specific renewal/reset periods,
- polyphonic MIDI ownership on one channel,
- removing one Chords note without clearing other owned notes,
- duplicate-frame suppression.

## Build validation

On the development machine run:

```bash
cargo fmt
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-features
cargo run --features ableton-link
```

This source package is still the desktop-development build. Brick Pro input,
physical USB-MIDI destination selection, ARM64 cross-compilation, and NextUI Pak
packaging remain the deployment step after desktop validation.

## Listening priorities

For this build, listen specifically for:

1. whether Bass + Chords alone clearly communicate the progression,
2. whether Chords feels like a real comping player rather than one note of a
   distributed block chord,
3. whether leaving Density unchanged produces recognizable but evolving rhythm,
4. whether Density changes feel like movement to a nearby behavior region,
5. whether Texture leaves enough air,
6. whether Melody remains phrase-like over the stronger harmonic foundation,
7. whether Chords voicings stay out of the Bass register,
8. whether four-note channel-2 MIDI remains reliable during rapid chord changes.

## v0.9.1 compiler cleanup

v0.9.1 removes the obsolete `VoicingEngine::realize()` wrapper. Production code already uses the canonical `realize_with_bass(...)` API so the Chords performer can distinguish whether the Bass performer is supplying the root. The no-bass voicing unit test now exercises that same API with `bass_present = false`. No musical behavior changes from v0.9.0.
