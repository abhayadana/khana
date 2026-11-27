# Khaṇa v0.14.0

Khaṇa is a playable MIDI instrument for TrimUI Brick Pro with NextUI and for
Linux desktop. Choose a chord, shape the four performers, then record and
loop a progression. Connect it to a synth, DAW, or other MIDI receiver for
sound.

## Build and install

The Brick build uses an AArch64 NextUI toolchain with `cross` and Docker.
For an existing `khana-nextui-link:pic` image, use the checked-in
`Cross-link-local.toml`. That image needs Clang/libclang for the Link
bindings and `CMAKE_POSITION_INDEPENDENT_CODE=ON` in the NextUI CMake
toolchain. This guide uses the local PIC image for both build and deploy.

When installing a patch, start from its stated GitHub commit and use
`git apply --check /path/to/patch` followed by `git apply /path/to/patch`.
The v0.14.0 patch is based on `1c5ea79883039db2733f23384307099d890bde68`.

From the repository root, build a Link-enabled Pak or deploy it over SSH:

```sh
CROSS_CONFIG="$PWD/Cross-link-local.toml" KHANA_LINK=1 \
  ./scripts/package-khana.sh

# Substitute the Brick's IP address. This command also builds the Pak.
CROSS_CONFIG="$PWD/Cross-link-local.toml" KHANA_LINK=1 \
  BRICK_HOST=root@YOUR_BRICK_IP ./scripts/deploy-khana.sh
```

For an SD-card install, extract `dist/Khana.pak.zip` into `Tools/tg5040/`
on the card. Launch Khaṇa from Tools. Exit Khaṇa before replacing the Pak
and back up any local changes to `launch.sh`. The session lives outside
the Pak. The default NextUI log is
`/mnt/SDCARD/.userdata/tg5040/logs/Khana.txt`.

On Linux desktop, install the Rust toolchain, SDL2 and ALSA development
packages, CMake, Clang/libclang, and a C++ compiler. Run with Link and the
desktop ALSA sequencer backend:

```sh
cargo run --locked --features ableton-link
```

## First performance

1. Connect a MIDI receiver to the Brick's USB host port or route the
   desktop sequencer output to a synth. Khaṇa assigns Bass, Chords, Texture,
   and Melody to MIDI channels 1, 2, 3, and 4 by default.
2. Press **Start** and use the **D-pad** to browse Safe, Colorful, and Bold
   suggestions. Press **A** to commit a chord. **L2/R2** step left/right
   through Resolve, Neutral, and Tension. Direction shapes recommendations
   independently of the three suggestion columns.
3. Press **L1/R1** to move the performer focus. Press **Y** to pin or unpin
   the focused performer for stick changes. **Select+L1** mutes the focused
   performer and **Select+R1** solos it.
4. Use **Select+R2** to set the live chord grid. The Performance screen shows
   the pending chord, its time to launch, and a wider flash on the downbeat.
5. Press **Select+A** to start or arm recording, play a progression, then
   press it again to end the capture. Tap **Menu** to open Phrase Editor.

### Performance and transport gestures

| Action | Brick Pro | Desktop keyboard |
|---|---|---|
| Start or stop | Start | Space |
| Switch Performance / Phrase Editor | Tap Menu | Tab |
| Quit | Hold Menu | Escape |
| Choose suggestion column and row | D-pad | Arrows |
| Commit the selected chord | A | Z |
| Reroll / Surprise | B / X | X / S |
| Harmonic direction | L2 / R2 step through Resolve, Neutral, and Tension | Q / E / W select directly |
| Previous / next performer focus | L1 / R1 | C / V |
| Add or remove focused performer from macro group | Y | F4 |
| Density − / + | Left stick ← / → | T / G |
| Dynamics − / + | Left stick ↓ / ↑ | Y / H |
| Spread − / + | Right stick ← / → | U / J |
| Color − / + | Right stick ↓ / ↑ | I / K |
| Toggle Sustain | L3 | F5 |
| Cycle Texture or Melody polyphony | Focus the performer, then Select+L3 | F3 |
| Open Workspace | Select+Start | F2 |
| Cycle chord voicing | R3 | F6 |
| Request scoped Vary | Select+R3 or Select+Y | F7 |
| Toggle scoped Pattern Hold | Select+L2 | F8 |
| Cycle live chord grid | Select+R2 | F9 |
| Toggle scoped Mute | Select+L1 | F10 |
| Toggle Solo | Select+R1 | F11 |
| Start or end phrase capture | Select+A | P |
| Arm or disarm the next capture | Select+B | N |
| Settings when stopped / BPM during Internal playback | Select+X | M |

The sticks make relative changes and leave values in place when centered.
The outline marks button focus, and dots mark members of a custom macro
group. For example, highlight Bass and press Y, then highlight Texture and
press Y. The sticks now edit Bass and Texture together even as you move
the focus. Press Y on All to clear the group. Mute, Solo, Sustain, Pattern
Hold, and Vary follow button focus. Hold Select to reveal contextual
shortcuts in the footer. CAPTURE shows recording state, the phrase count,
and whether the next capture is armed.

L2 and R2 step once per press and stop at the ends of the three-position
direction selector. Brick Pro defaults to raw SDL buttons 11 and 12 for L3
and R3. `KHANA_L3_BUTTON` and `KHANA_R3_BUTTON` override those indices.

**Live grid** cycles Now, 1/8, 1/4, Beat, and Bar. Now commits immediately.
The other choices wait for the next boundary while transport runs. The
latest commit replaces the pending chord. Changing the grid leaves an
already queued chord at its scheduled time. While stopped, A commits
immediately. In 6/8, 9/8, and 12/8, Beat means a dotted quarter.

### How the four macros shape the ensemble

**Density** chooses the kind of pattern and how much space it leaves.
Zero silences the performer. **Dynamics** changes MIDI attack velocity.
The sound depends on the receiving instrument's response to velocity.
**Spread** affects register and spacing. **Color** changes the pitch
palette or brings selected chord tones forward.

| Performer | Density | Dynamics | Spread | Color |
|---|---|---|---|---|
| Bass | Sustained anchors, root/fifth foundations, then moving patterns and turnarounds | Bass attack weight | Higher values favor a higher bass position. Patterns can make octave jumps. | Root/fifth first, then thirds, sevenths, and scale notes where the figure supports them. High Color can allow a chromatic approach. |
| Chords | Long gates, pulses, then comping and stabs | Overall chord attack | Widens the voicing and influences inversion and register. R3 chooses Smooth, Close, or Open. | Brings root/fifth forward at low settings, the third around the middle, and the seventh or ninth up high. High Color on a triad emphasizes the third. Every pitch of the displayed chord stays in the voicing. |
| Texture | In Auto, drones, isolated notes, pulses, then figures and shimmer. An explicit role shapes how it uses that space. | Brings figures forward or lets them sit behind Melody | Raises typical figures and widens polyphonic spacing | Chord tones first, then colors such as the ninth, eleventh, and thirteenth. Drone and Echo favor compatible tones. |
| Melody | Sparse motifs, developed phrases, then ornaments | Lifts the lead. Structural peaks can receive an accent and passing notes play softer. | Raises the lead's register and separates a companion when Harmony is enabled | Structural notes stay on chord tones. Higher settings let fills use more of the scale and can add chromatic steps into a chord tone. |

Try a slow progression with Chords Density low and Texture on Common-tone
Drone. Raise Melody Density while keeping its Dynamics restrained. For a
Chord Color comparison, play Cmaj9 at both ends of the Color range.
Listen for C and G at the low end and B and D at the high end.

### Texture and Melody polyphony

Focus Texture or Melody with **L1/R1**, then press **Select+L3** or **F3**.
The performer row shows the setting. The gesture follows the outlined
performer. The macro group continues to govern stick movements.

- **Texture** cycles Mono, Dyad, and Triad. Figures can add up to two
  chord tones, and Common-tone Drone can keep compatible notes through
  harmony changes. Sparse Echo keeps its single-note reply.
- **Melody** cycles Mono and Harmony. Harmony adds a quieter lower note
  on longer structural notes. Passing notes and fast ornaments stay
  single. Spread affects the separation.
- Extra voices enter on an eligible figure or at the next beat for a
  sounding drone. Reducing to Mono releases the extra voices by the next
  beat. Plain L3 still controls Sustain.

**Sustain** extends sounding notes for the active scope. It holds a stable
note already sounding, or waits for the next one. Chords hold the current
voicing, Texture can carry a pedal pitch, Melody waits for a structural
chord tone, and Bass follows its Root or Voice-led arrival setting on a
new chord. Texture and Melody sustain across a chord change when their
pitch is still a chord tone. Click again to release and resume regular
gates. Stop, mute, soloing another performer, and Density zero release
sounding notes and clear the affected Sustain latch.

**Pattern Hold** keeps the scoped patterns or motifs repeating as the
harmony changes. **Vary** requests related variation at the next
performer boundary. Held patterns return after the variation.

**Mute and Solo** act independently of the macros. Use Mute to silence
one performer. With All selected it mutes all, and repeating it with all
muted clears all mutes. Solo isolates the selected performer. Solo the
same performer again or use Solo with All selected to clear it. Explicit
mutes remain in effect while soloing, including on the selected performer.
Silenced performers keep their pattern timing and can re-enter in time.

**Voicing** cycles Smooth, Close, and Open. While stopped the displayed
voicing changes immediately. During playback the pending style applies
on the next beat and preserves common sounding pitches where possible. The
current style and next style appear beside the chord. A chord change on
the same beat uses the newly selected style.

## Record, edit, and loop phrases

Recording starts and stops at bar boundaries. It captures chord selections
and timing, including the exact landing time of a quantized change. Arm
Next starts another phrase at the current capture's end. When playing a
phrase, the current performer settings supply the arrangement.

| Action | Brick Pro | Desktop keyboard |
|---|---|---|
| Previous / next phrase | L1 / R1 | A / L |
| Select previous / next chord event | D-pad ← / → | Left / Right |
| Play or queue the selected phrase | A | Z |
| Open the selected chord in the chord picker | B | X |
| Open Edit menu | X | S |
| Duplicate event | X → Duplicate | D |
| Delete event | Select+Y | Backspace |
| Shorten / lengthen to the next edit boundary | L2 / R2 | F / R |
| Previous / next duration grid | D-pad ↑ / ↓ | Up / Down or [ / ] |
| Zoom out / in | Select+L1 / Select+R1 | - / = |
| Fit whole phrase / return to previous zoom | X → Fit Phrase / Restore Zoom | S → Fit Phrase / Restore Zoom |
| Toggle selected phrase loop | L3 | F5 |
| Open Workspace | Select+Start | F2 |
| Clear pending phrase or section chain | R3 or Select+B | F6 |
| Undo the selected phrase's last edit | Select+A | F12 |
| Return to Performance | Y or tap Menu | Tab |
| Settings when stopped / BPM during Internal playback | Select+X | M |

### Change, insert, and move chords

Press **B** (**X** on desktop) on a selected event to open Change Chord.
Use Left/Right for Safe, Colorful, and Bold, and Up/Down for the choices in
each column. Press **X** (**S** on desktop) inside the picker for Manual.
Left/Right selects root, quality, or extension. Up/Down changes the value.
The picker shows the adjacent chords. **A** (**Z**) applies the displayed
choice and **B** (**X**) cancels. Changing the chord preserves its duration.

**Edit** groups Insert Before, Insert After, Duplicate, Copy, Cut, Paste
After, Delete, Fit Phrase, and Queue Section. Insert opens the same picker
and adds one unit of the current Duration Grid. Copy and Cut put the event's
chord and exact duration on a clipboard shared between phrases. Paste After
or Duplicate inserts after the selection. The following chords move to
make room or close the gap. Delete and Cut require at least two chords.

The duration grid offers 1/16, 1/8, 1/4, 1/2, Beat, and Bar. On-grid
durations move one increment. Off-grid durations snap down when shortened
and up when lengthened. **Fit Phrase** shows the full progression. Use zoom
to work on individual chords. The ruler marks bars and draws beat divisions
when there is room. A moving line shows the current playback position when
viewing the playing phrase.

**L3** loops the whole phrase at its recorded or edited length, including
off-grid endings. The playing pass uses the chords and durations it started
with. Edits and undo take effect on the next pass or when you next launch
the phrase. Undo restores the event list and selection for up to 32 edits
per phrase. Cut, paste, and insert each use one undo step.

### Chain sections

Choose a phrase and open **Edit → Queue Section**. Use Left/Right to
highlight Full Passes or Enter Section At, then Up/Down to change the
highlighted value. **A** (**Z**) adds the section to the end of the queue.
You can queue up to 16 pending sections and give each section 1–16 full
passes. A chain uses the specified count regardless of the phrase's Loop
setting.

Build the chain while stopped and press **Start** (**Space**) to play it.
With MIDI In selected, start the external transport. You can also add
sections while playing. The first section starts with the transport if
there is no phrase already playing.

| Entry setting | When the next section starts |
|---|---|
| Phrase End | At the end of the preceding section's last pass, or the end of the current pass of a regular loop. |
| Bar | On the first transport bar at or after a finite section finishes. From a regular looping phrase, it enters on the next transport bar. |

An off-grid finite section can leave a rest before a Bar entry. The final
section plays its specified passes and falls silent with transport still
running. **A** in the editor outside the menu still plays the selected
phrase immediately when idle or replaces the current phrase at the next bar.
This clears queued sections. **R3** or **Select+B** drops pending sections
and lets the current phrase continue. The chain is a live queue cleared
when transport stops.

## Settings and MIDI rig

From Performance or Phrase Editor, press **Select+X** on Brick or **M**
on desktop while stopped. Press **X** to switch between Music and MIDI Rig.
The **D-pad** or arrow keys move the highlight to a value. **L1/R1** or
**- / =** change it immediately. **B** or **Menu** returns to the previous
screen on Brick. Use **M** on desktop. **Start** or Space closes Settings
and toggles transport. During Internal playback the Settings shortcut
opens a live BPM control with the same value buttons.

Music contains Tonic, Mode, Tempo, Meter, Chord Gate, Bass Arrival,
Groove, and Texture Role. The Chord Gate sets automatic duration for
the low-density Chords sustain families. Its choices are Auto, 4, 8, or
16 bars, and Until Change. R3 on Performance controls the voicing style.

MIDI Rig opens on Bass Channel. To set Melody to channel 7 and transpose
it an octave up from the defaults, move down three rows, press **R1**
three times, move right to Transpose, and press **R1** twelve times.
Channel values range from 1 to 16 and transpose from −24 to +24 semitones.
Transposition changes the outgoing pitch while the displayed chord stays
in the source key. When performers share a channel and pitch, Khaṇa
releases the note when the last performer lets go.

Clock and output settings sit below the performers. MIDI input status and
Link peer count are visible as readouts. Select MIDI Panic and press **A**
or **Z** twice to review and send All Sound Off and All Notes Off to the
assigned channels. **B** or **M** cancels confirmation.

### Groove and ensemble space

**Straight** uses the existing spacing. **Light Swing** delays played
offbeat eighth-note attacks. **Laid Back** leaves Bass as the timing
reference and puts Chords, Texture, and Melody slightly behind it.
The groove affects performer attacks. Chord commits, phrase launches,
loop boundaries, MIDI Clock, and Ableton Link keep their grid timing.

Bass uses a turnaround near the actual end of a playing phrase, including
off-grid loops, when its Density has reached the moving range. Melody
answers at the close with fewer passing notes and a lower contour.
Rhythmic Chords leave space around Melody attacks, and Texture can give
Bass room at the close. Pattern Hold retains the performer's pattern.

**Texture Role** offers the following choices.

| Role | Behavior |
|---|---|
| Auto | Select figures, pulses, and drones from Density |
| Upper Figure | Place sparse notes above the chord voicing and give Melody attacks room |
| Common-tone Drone | Prefer a pitch shared with the next known chord and carry it across compatible changes |
| Sparse Echo | Answer a recent Melody attack once in a gap, using a compatible pitch when possible |

### MIDI Clock and Ableton Link

- **Internal** uses Khaṇa's BPM. Turn on MIDI Clock Out in the rig to drive
  an external sequencer.
- **MIDI In** follows a raw MIDI clock input. Arm with Start if required,
  then send Start or Continue from the source. Khaṇa shows the input state
  and releases notes when external transport stops or lock is lost. Clock
  Out is blocked while following MIDI In to avoid feedback.
- **Ableton Link** shares tempo and beat phase with peers on the same local
  network. Enable Link on the other app and select **Ableton Link** as the
  clock source while stopped. The rig displays the peer count. Press Start
  to queue launch at the next bar, or press it again to cancel a pending
  launch. Enable Link Start/Stop Sync on both apps to share transport.
  Match meters when bar alignment matters. MIDI Clock Out can drive
  attached hardware from the Link session.

Raw MIDI Output can select Auto, an available port, or Off. Set
`KHANA_MIDI_PORT` for a name match or `KHANA_MIDI_RAW_DEVICE` for an exact
raw node. Set `KHANA_MIDI_CLOCK_IN_PORT` separately for a clock input by name
or path. `KHANA_INPUT_LOG=1` logs SDL events. `KHANA_MIDI_TRACE=1` shows MIDI
traffic. On desktop, `cargo run --locked -- --midi-list` lists ports.
Use `KHANA_MIDI_DRY_RUN=1` to try the interface without sending MIDI.

## Capture Take and Workspace

Open **Workspace** from either main screen with **Select+Start** on Brick
or **F2** on desktop. Left/Right switches tabs. Up/Down selects an item.

| Tab | A / Z | X | Other controls |
|---|---|---|---|
| Takes | Play highlighted take | Switch Phrase / Take capture | Y / F1 recalls setup while stopped. L3 / F5 copies harmony into Phrase Editor. R3 / F6 recovers the last complete phrase pass. |
| Projects | Review, then press again to load | Save the current workspace under a name | Stop transport first. B / M cancels. |
| Presets | Load the highlighted performer setup | Save the current setup under a name | Stop transport first. B / M cancels. |

To capture, choose **Take** with X in the Takes tab, return to Performance,
and use the usual **Select+A** or **P**. An existing phrase loop records
one complete pass from its next seam. During live play, capture starts
on the next bar and a second press schedules the end. Press again while
armed to cancel. Stopping transport ends an active take. The status strip
shows which capture mode is selected.

**Play Phrase** generates a fresh rendition from the chord events.
**Play Take** sends the notes, voicings, velocities, and beat-relative
timing from the original pass over the current MIDI routing. The active
Internal, MIDI Clock, or Link tempo sets replay speed. Take settings are
stored with it and can be recalled separately. Copy Harmony adds an
editable looping phrase alongside the take.

### Recover the last phrase pass

Khaṇa also buffers the most recent complete pass played from a phrase or a
chain section. Open **Workspace → Takes**, then press **R3** (**F6** on
desktop) when recovery says Ready. The pass becomes a normal saved take.
It includes the actual generated notes and velocities, notes already
held at the beginning, note releases at the end, and the harmony and
performer setup at the start. During playback, it follows the active tempo
and current MIDI routing like a regular take.

The buffer covers one complete pass with sounding notes, including one
pass of a multi-section chain. For free live playing or a take spanning
several sections, arm Capture Take before playing. A recovered take saves
with the session. Until you recover it, the pass is held in memory and
replaced by the next complete pass.

Projects save key, meter, internal tempo, phrases, takes, and the current
performer setup. Edit and save the project again to update its named
snapshot. Presets save the reusable performer setup, including macros,
voicing, polyphony, Chord Gate, Groove, and Texture Role. A preset leaves
the current project and phrase bank in place. MIDI channels, transpose,
clock source, and port settings belong to the rig.

For a name, use Left/Right to move between characters and Up/Down or
L1/R1 to cycle letters, digits, spaces, and a hyphen. Y or F1 clears the
name. A or Z saves. Saving under an existing name prompts for confirmation.
The workspace supports up to 64 takes, projects, and presets each.

## Save and resume

Khaṇa saves the key, meter, clock mode, performer settings, output channels
and transpose, voicing, polyphony, groove, Texture Role, both grids, phrases, takes,
and the project and preset lists.
Quit cleanly to flush the last changes. The next launch restores the
setup with transport stopped and empty playback and recording queues.
Mute, Solo, Sustain, the macro group, the clipboard, and undo history are
performance-session state.

On Brick, the default file is
`/mnt/SDCARD/.userdata/tg5040/khana/session.json`. On desktop it is
`$XDG_STATE_HOME/khana/session.json` or `~/.local/state/khana/session.json`.
Set `KHANA_SESSION_PATH` to override the location. Valid launcher values
for `KHANA_MIDI_CHANNELS` and `KHANA_MIDI_TRANSPOSE` override the saved
routing values in Bass, Chords, Texture, Melody order.

## Validation

The Rust tests cover performer note transitions, chord edits, phrase seams,
section pass counts and clock-grid timing, and recovering a pass as a take.
The Link-enabled Brick build and desktop checks exercise the compiled code.

On a Brick Pro, try a 2.5-beat phrase twice followed by a Bar entry for
another phrase. Listen for the rest before the next bar and verify that
the next phrase starts in time. Try a Phrase End entry for a continuous
transition. While looping, resize or replace a chord and check that the
change lands at the next seam. Recover the last pass and replay it with
MIDI Clock or Link. Check note releases, routing, and the original timing.

## LLM disclosure

Khaṇa was developed with assistance from OpenAI Codex for design,
implementation, documentation, and automated testing. Musical decisions
and device tests are guided by the project maintainer.
