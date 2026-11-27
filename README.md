# Khaṇa v0.12.2

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
The v0.12.2 patch is based on `1f20363ad1679fdeaf5e428e97b3f969b0f5d57e`.

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
   suggestions. Press **A** to commit a chord. **L2**, **L2+R2**, and **R2**
   set the harmonic direction to Resolve, Neutral, and Tension. **Y** also
   selects Neutral. Direction shapes recommendations independently of the
   three suggestion columns.
3. Press **L1/R1** to scope the sticks to one performer or All. Try lowering
   Chords Density for longer gates, then adjust its Spread. Turn off a busy
   part briefly with **Select+L2**, or audition one with **Select+R2**.
4. Use **Select+R1** to set the live chord grid. The Performance screen shows
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
| Resolve / Neutral / Tension | L2 / L2+R2 or Y / R2 | Q / E / W |
| Previous / next performer scope | L1 / R1 | C / V |
| Density − / + | Left stick ← / → | T / G |
| Dynamics − / + | Left stick ↓ / ↑ | Y / H |
| Spread − / + | Right stick ← / → | U / J |
| Color − / + | Right stick ↓ / ↑ | I / K |
| Toggle Sustain | L3 | F5 |
| Cycle chord voicing | R3 | F6 |
| Request scoped Vary | Select+R3 or Select+Y | F7 |
| Toggle scoped Pattern Hold | Select+L1 | F8 |
| Cycle live chord grid | Select+R1 | F9 |
| Toggle scoped Mute | Select+L2 | F10 |
| Toggle Solo | Select+R2 | F11 |
| Start or end phrase capture | Select+A | P |
| Arm or disarm the next capture | Select+B | N |
| Settings when stopped / BPM during Internal playback | Select+X | M |

The sticks make relative changes. Let them return to center to leave the
values where you set them. L2/R2 are digital triggers. For Neutral, press
both triggers together or use Y. Press and release the triggers for each new
direction gesture. Brick Pro defaults to raw SDL buttons 11 and 12 for L3
and R3. `KHANA_L3_BUTTON` and `KHANA_R3_BUTTON` override those indices.

**Live grid** cycles Now, 1/8, 1/4, Beat, and Bar. Now commits immediately.
The other choices wait for the next boundary while transport runs. The
latest commit replaces the pending chord. Changing the grid leaves an
already queued chord at its scheduled time. While stopped, A commits
immediately. In 6/8, 9/8, and 12/8, Beat means a dotted quarter.

**Performer macros** shape independent roles. Bass anchors the harmony
and can walk into a known chord. Chords play the complete displayed chord
with register and spacing set by Spread and voicing. Texture contributes
repeating figures and drones. Melody uses phrase-shaped motifs, peaks,
and answers. Density changes activity, Dynamics changes velocity, Spread
changes register and voicing width, and Color opens up each role's pitch
choices. At Density zero a performer is silent.

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
| Replace selected chord with a Safe suggestion | B | X |
| Duplicate event | X | D |
| Delete event | Select+Y | Backspace |
| Shorten / lengthen to the next edit boundary | L2 / R2 | F / R |
| Previous / next duration grid | D-pad ↑ / ↓ | Up / Down or [ / ] |
| Zoom out / in | Select+L1 / Select+R1 | - / = |
| Toggle selected phrase loop | L3 or Select+X | F5 |
| Clear queued phrase | R3 or Select+B | F6 |
| Undo the selected phrase's last edit | Select+A | F12 |
| Return to Performance | Y or tap Menu | Tab |

The duration grid offers 1/16, 1/8, 1/4, 1/2, Beat, and Bar. On-grid
durations move one increment. Off-grid durations snap down when shortened
and up when lengthened. An enabled phrase loop wraps at its full length,
including off-grid lengths. A new phrase queued during playback replaces
the playing phrase at the next transport bar. Clear Queue leaves the
playing phrase in place.

Undo restores duration edits, chord replacements, duplicates, and deletions
in reverse order for the selected phrase. It restores the selection for
that edit. Each phrase keeps up to 32 edits in the current session.

## Settings and MIDI rig

From stopped Performance, press **Select+X** on Brick or **M** on desktop.
Left/Right selects a field and Up/Down changes its value. A, B, or Menu
closes Settings on Brick. On desktop use Z or M. Press Start or Space
to close Settings and toggle the transport. While Internal playback is
running, the Settings shortcut opens the live tempo control.

The general fields are Tonic, Mode, Tempo, Meter, Chord Hold, Bass
Arrival, and Voicing. Mode includes Harmonic Minor and the ascending
Melodic Minor scale used in both directions. Chord Hold applies to the
low-density Chords sustain families and offers Auto, 4, 8, or 16 bars,
and Until Change.

In Settings, **Left from Tonic** opens **MIDI Rig** at MIDI Output.
Here you can see each performer's channel and transpose alongside the
clock source, MIDI input and output, Link peers, and shared transport
status. Channels range from 1 to 16 and transpose from −24 to +24
semitones. Assigning the same channel to multiple performers is supported.
If they share a sounding pitch, Khaṇa releases it when the last performer
lets go. Transposition changes the outgoing pitch while the progression
and displayed chord stay in the source key. Choose MIDI Panic and press
A or Z while stopped to send All Sound Off and All Notes Off on the
assigned channels.

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

## Save and resume

Khaṇa saves the key, meter, clock mode, performer settings, output channels
and transpose, voicing, both grids, and phrases with their loop flags.
Quit cleanly to flush the last changes. The next launch restores the
setup with transport stopped and empty playback and recording queues.
Mute, Solo, Sustain, and the undo history are performance-session state.

On Brick, the default file is
`/mnt/SDCARD/.userdata/tg5040/khana/session.json`. On desktop it is
`$XDG_STATE_HOME/khana/session.json` or `~/.local/state/khana/session.json`.
Set `KHANA_SESSION_PATH` to override the location. Valid launcher values
for `KHANA_MIDI_CHANNELS` and `KHANA_MIDI_TRANSPOSE` override the saved
routing values in Bass, Chords, Texture, Melody order.

## Validation

The Rust tests cover performer note transitions, phrase editing, routing,
and transport. The Link-enabled Brick build and desktop checks exercise
the compiled code. Physical Brick controls and external MIDI and Link
timing need a device test. Try sustaining Texture, soloing Melody,
switching voicing over a queued chord, then clearing Solo. Listen for
continuous shared notes, clean releases, and the previously muted
performers returning in phase.

## LLM disclosure

Khaṇa was developed with assistance from OpenAI Codex for design,
implementation, documentation, and automated testing. Musical decisions
and device tests are guided by the project maintainer.
