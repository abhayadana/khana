# Khaṇa v0.11.1

Khaṇa is a performance-oriented MIDI harmony/phrase instrument for the TrimUI
Brick Pro and desktop development. Its core workflow is:

```text
hear possibilities → choose → shape → capture → revisit → vary
```

v0.11.0 is the first build where the four generators begin behaving like a
small ensemble rather than four independent parts reacting to the same chord.

## v0.11.1 — Brick foundation

This cumulative upgrade applies directly to **v0.9.1** at GitHub commit
`ad880bdfb6714f7c552ed81813a25420ad0d5202` (20261126 baseline). It includes
the v0.10 editor and performer changes, audited v0.11 ensemble changes,
and v0.11.1 Brick foundation. No intermediate patches are required. See [Brick setup and controls](docs/BRICK_V0.11.1.md)
for the complete handheld mapping, build/deploy workflow and hardware test plan.

```sh
./scripts/package-khana.sh
BRICK_HOST=user@brick-ip ./scripts/deploy-khana.sh
```

The package is `dist/Khana.pak.zip`; deployment always rebuilds first. The first
hardware pass may use `KHANA_MIDI_DRY_RUN=1` to validate controls without MIDI.

## v0.11 musical foundation

```text
Bass      foundation / line / destination
Chords    complete harmonic statement
Texture   regular cyclic motion / atmosphere
Melody    phrase / motif / expressive rhythm

                    ↕
             EnsembleContext
                    ↕
       rhythmic + melodic coordination
```

The harmonic recommendation engine, Phrase Editor grid, Brick/NextUI
scaffolding, and physical MIDI discovery from v0.10 remain in place.

## Chords: play the chord that is displayed

The Chords performer no longer forces every harmony into a four-note voicing.

```text
C          C E G             3 notes
Cm         C Eb G            3 notes

Cmaj7      C E G B           4 notes
Cm7        C Eb G Bb         4 notes
G7         G B D F           4 notes

Cmaj9      C E G B D         5 notes
G9         G B D F A         5 notes
```

Every semantic chord tone appears exactly once in the Chords performer.

Bass presence does not remove the root from Chords and does not change the
chord's note count. Spread changes inversion/register width while preserving
membership.

The real-time voicing container remains fixed-capacity and allocation-free.
Seven slots are reserved so future 11th/13th chord support does not require
another storage redesign.

## Chords Density

The v0.10 continuum remains:

```text
SILENT
  ↓
LONG SUSTAIN
  ↓
BAR SUSTAIN
  ↓
PULSE
  ↓
COMP
  ↓
ACTIVE
```

LONG SUSTAIN includes two- and four-bar holds. A real harmony change always
interrupts the prior chord immediately.

## Melody: phrase rhythm is now part of the idea

Melody is still generated as two-bar phrases, but v0.11 no longer treats phrase
events as mostly equal-length sequencer notes.

Each Melody step carries:

```text
intent
gate length
articulation
```

Current intents:

```text
Skeleton
Fill
Neighbor
Suspension
Modal Tension
Chromatic Approach
Rest
```

Current articulation states:

```text
Short
Normal
Legato
Held
```

Typical duration behavior:

```text
passing / neighbor / chromatic approach   short
ordinary fill                            short–medium
structural skeleton                      medium
suspension / modal tension               longer / legato
phrase peak                              longer
cadential arrival                        longest / held
```

A phrase can therefore sound more like:

```text
G──────  A B  C────────
```

rather than a stream of equal eighth notes.

### Melody rhythmic identities

The existing user-facing families remain:

```text
MOTIF → PHRASE → ORNAMENT
```

Their internal variants now carry more recognizable rhythmic behavior:

```text
MOTIF
Call
Pulse
Sparse Arch

PHRASE
Arch
Motif Development / Wave
Answer

ORNAMENT
Flourish
Passing-dense
Chromatic phrase
```

Density still controls overall activity. The hidden phrase identity determines
where that activity tends to occur.

### A → A′ continuity

When repetition/continuity is strong, a new phrase can retain the previous
rhythmic skeleton, rests, gates, and articulation while altering a small
non-structural event.

```text
A  →  A′
```

This is intentionally preferred over redrawing an unrelated melody every two
bars.

Chromatic approaches still carry an explicit resolution target. Suspensions and
modal tensions are allowed to persist rather than being mechanically resolved
on the next note.

## Texture: regularity is intentional

Texture is now the performer most comfortable with persistent, regular cyclic
rhythm.

Its Density continuum is:

```text
SILENT
  ↓
DRONE
  ↓
SPARSE
  ↓
PULSE
  ↓
MOTION
  ↓
ACTIVE
```

Current pattern vocabulary includes:

```text
DRONE
long drone
bar drone

SPARSE
isolated upper tone
delayed echo

PULSE
half-note pulse
quarter-note pulse

MOTION
continuous three-step arpeggio
slow arpeggio
extension figure

ACTIVE
shimmer
repeating arpeggio with a gap
continuous rotating three-note figure
```

### Texture phase continues through the music

Texture uses its own absolute pattern phase.

A three-step cell can therefore rotate across a 4/4 bar rather than restarting
at beat one every measure:

```text
E G B | E G B | E G B | ...
```

When harmony changes, the rhythm normally keeps running and the pitch mapping
adapts to the new chord.

This deliberately separates Texture from Chords:

```text
Chords   articulate harmonic events
Texture  maintain a continuing patterned stream
```

### Texture articulation belongs to the pattern

Texture does not randomly change gate shape note by note.

```text
drone     very long
pulse     medium
arp       more legato
shimmer   short
```

That makes the cyclic identity audible and repeatable.

## First relationship modes

Texture now has two internal relationship behaviors:

```text
INDEPENDENT
COMPLEMENT
```

Independent Texture maintains its own cycle.

Complementary Texture can leave a subdivision open when Melody has just
attacked, creating simple rhythmic interlock without random omission.

Future builds can extend this architecture with:

```text
SHADOW
IMITATE
COUNTERLINE
```

## EnsembleContext and soft counterpoint

Each performer can now consult a small shared snapshot:

```text
currently active representative notes
recent attack times
recent melodic motion direction
```

This is advisory. It does not centrally compose all four parts.

v0.11 uses it for the first counterpoint preferences, primarily:

```text
Melody ↔ Texture
Melody ↔ Bass
```

Candidate-note scoring can:

- penalize very close same-register collisions,
- modestly prefer contrary motion,
- allow oblique motion naturally,
- mildly discourage repeated same-direction perfect-interval motion,
- reduce dissonance penalties as Color increases.

These are soft preferences, never textbook species-counterpoint prohibitions.

Low Color favors clearer separation and stable intervals. Higher Color permits
more seconds, sevenths, tritone/modal friction, and unresolved color.

## Bass

The v0.10 Bass continuum remains:

```text
SILENT
  ↓
ANCHOR
  ↓
FOUNDATION
  ↓
MOVING
  ↓
CONNECT
```

Vocabulary still includes:

```text
root sustain / pedal
root–fifth
root–octave
root bounce
broken bass
chord-tone ostinato
scale approach
chromatic approach
walking fragment
```

### Known-next-chord targeting

v0.11 adds one important CONNECT refinement.

During semantic Phrase playback, the next chord is genuinely known. Khaṇa now
passes that destination to the Bass performer so CONNECT patterns can prepare
the actual next root with diatonic or chromatic approaches.

During live improvisation no future harmony is invented:

```text
future chord unknown → stay local to current harmony
future chord known   → connector may target it
```

## Density-change boundaries

The v0.10 behavior remains:

```text
Bass       next bar or harmony change
Chords     next bar or harmony change
Texture    next bar
Melody     next two-bar phrase

Density 0  immediate clean mute
```

The control value changes immediately; musical behavior changes at an
appropriate boundary.

## Phrase Editor grid

Unchanged from v0.10:

```text
1/16 | 1/8 | 1/4 | 1/2 | BEAT | BAR
```

Shorten/Extend snaps down/up to the adjacent grid boundary instead of adding an
increment to an already off-grid value.

Example in 4/4:

```text
duration = 4.1 bars
grid     = BAR

Shorten → 4.0 bars
Extend  → 5.0 bars
```

Literal note-value grids remain literal. `BEAT` is meter-aware; compound
6/8, 9/8, and 12/8 use dotted-quarter beats.

## MIDI / Brick / NextUI

v0.11.1 keeps MIDI channels 1–4 for Bass / Chords / Texture / Melody and
polyphonic per-note ownership. See [Brick setup](docs/BRICK_V0.11.1.md) for
physical-port diagnostics, virtual and dry-run modes, raw joystick handling,
30/60 FPS rendering, build/package/deploy scripts and device checks.

## Static-audit hardening

Before release handoff, v0.11 received an additional source-level audit. That
audit made four important corrections:

- Phrase-known Bass CONNECT targets now include the destination's scheduled
  transport beat; CONNECT only treats that chord as an imminent destination
  when the arrival is within one beat. Long-held phrase chords therefore do not
  produce a false approach at the end of every intervening bar.
- Harmony changes no longer call a global MIDI panic-stop or erase internal
  Texture/Melody note ownership. Bass and Chords still force the new harmony,
  while sounding Texture/Melody notes finish through their normal gate/next
  attack and retain phase/phrase identity.
- Melody cadential arrivals are placed early enough to make their held gate
  audible before the next two-bar phrase starts.
- Voicing candidate validation reuses the already-computed chord pitch classes
  instead of allocating another vector for every candidate voicing.

Regression tests were added for exhaustive chord membership across all current
roots/qualities/extensions, phrase-next-chord timing, harmony-change note
ownership, audible cadence holds, Texture regularity, Texture phase continuity,
and soft counterpoint motion preference.

## Desktop validation

Run on the Xubuntu development machine:

```bash
cargo fmt
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-features
cargo run --features ableton-link
```

The build environment used to prepare this source does not contain Rust, so
v0.11.1 has **not** been compiled here. Shell syntax and the local mocked
package/deploy freshness regression have been checked.
The user's Rust 1.98.1 machine remains the authoritative compile/Clippy/test
environment.

## Suggested v0.11 listening tests

### Chords

1. Play a major/minor triad and confirm channel 2 emits three notes.
2. Play a seventh chord and confirm four notes.
3. Play a ninth chord and confirm five notes.
4. Change Spread and confirm chord membership is unchanged.

### Melody

1. Listen for short passing/neighbor notes versus sustained structural notes.
2. Confirm cadential/peak notes can sustain over multiple grid steps.
3. Leave the phrase running and listen for recognizable A → A′ continuity.
4. Increase Color and verify modal/suspension tension is not always immediately
   resolved.
5. Increase Density and verify the phrase becomes more active without losing
   its contour/rhythmic identity.

### Texture

1. Move Density through DRONE → SPARSE → PULSE → MOTION → ACTIVE.
2. Listen for regular pulse/arp behavior rather than random holes.
3. Use a rotating three-step figure and confirm it crosses bar boundaries
   without restarting.
4. Change chords and confirm the Texture phase continues.
5. Listen for complementary Texture occasionally leaving room for Melody.

### Ensemble / counterpoint

1. Run Texture + Melody and listen for fewer same-register collisions.
2. Listen for a mixture of contrary, oblique, and parallel motion rather than
   identical line movement.
3. Increase Color and verify interval friction becomes more permissible rather
   than being treated as an error.
4. Mute each performer independently; the remaining ensemble should retain a
   coherent musical role.

### Bass

During Phrase playback, use high Bass Density and verify CONNECT behavior can
prepare the actual following chord. During live performance with no queued
future chord, Bass should never fabricate a harmonic destination.

## Deliberately deferred

v0.11 does not yet implement:

- variable 1-/2-/4-bar Melody phrase lengths,
- full Texture SHADOW / IMITATE / COUNTERLINE modes,
- explicit motif locking in the UI,
- cadence-aware whole-ensemble arrival behavior,
- user-facing Intensity gesture,
- Chords common-tone legato versus deliberate comp rearticulation distinction,
- full 11th/13th chord vocabulary,
- final verification of the Brick-specific physical button map and ALSA output,
- independent real-time MIDI scheduler and USB MIDI reconnection.

The guiding test remains:

> Can a small amount of harmonic material stay compelling because four
> performers retain identity, vary gradually, and listen to one another?
