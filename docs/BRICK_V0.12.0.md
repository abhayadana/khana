# Khaṇa v0.12.0 transpose and musical continuity

Baseline GitHub main `c8b4d6028336bbd94628ed3d76320769658bac2c`.

## Performer transpose

From stopped Performance, press **Select+X** on Brick or **M** on desktop.
**Bass Transpose** is ten Right presses from Tonic. Continue right for Chords,
Texture, and Melody. Alternatively, Left four times from Tonic lands on Melody
Transpose. Up and Down adjust the focused performer by one semitone over
±24 semitones. The performer rows display the output channel and offset.

Set octave shifts independently even when performers share the same output
channel. The offset applies to both Note On and Note Off. When transposed
notes meet on the same pitch and channel, the last performer to release
ends the note. Notes shifted outside the MIDI range are omitted. The displayed
chord, voicing, and captured chord progression remain in their source key.

Edits last for the current session. To set startup offsets, add
`export KHANA_MIDI_TRANSPOSE=-12,0,0,12` to the launcher before its final
`exec`. The values are Bass, Chords, Texture, and Melody. The default is
`0,0,0,0`.

## Musical continuity

- Melody retains a two-bar motif over four phrases: **A, A′, A, answer**.
  A′ changes a note while keeping the original rhythm and gate lengths.
  The answer keeps the opening, lowers the peak, and can leave a breath
  before the cadence. Motif rhythm and pitch intent survive chord changes;
  the notes are realized against the active harmony. The next group or a
  density-family change can introduce a new motif.
- Structural Melody notes use chord tones. The peak gets a small accent,
  fills play softer, and high Color can introduce stepwise chromatic
  approaches. The underlying two-bar timing remains in quarter-note units.
- Sustained Chords keep shared voiced pitches sounding across harmony
  changes when those notes are still held. Rhythmic chord patterns retain
  their attacks. Voicing favors low movement and exact shared pitches while
  preserving every semantic chord tone.
- Bass states the new root on a harmony change. Its chord-tone ostinato
  favors a recurring degree figure. Texture gives more weight to nearby
  notes while preserving its cyclic rhythm.

## Listen and verify

1. Hold one chord for eight bars at moderate Melody Density to hear the
   motif, small variation, return, and answer. Repeat with two chords and
   listen for the recurring rhythm through the changes.
2. At low Chords Density, try C to Am with a sustained voicing. Check that
   held common pitches release cleanly when the new gate ends or you stop.
   Raise Density to compare the rhythmic attack.
3. Try Bass at −12 and Melody at +12. On a MIDI monitor, check shifted
   Note On and Note Off pairs and test two performers that share a channel.

Build with `./scripts/package-khana.sh`. For the existing Link-enabled Brick
setup, use
`CROSS_CONFIG="$PWD/Cross-link-local.toml" KHANA_LINK=1 ./scripts/package-khana.sh`.
