# Khaṇa v0.12.1 live harmony and session restore

GitHub main baseline: `c93818061ab0542f90c825da0faba7138ab531d8`.

## Performance controls

| Action | Screen | Brick | Desktop |
|---|---|---|---|
| Cycle the live chord grid | Performance | Select+R1 or R3 | F6 |
| Hold or release pattern identity in the current scope | Performance | Select+L1 or L3 | F5 |
| Request related variation in the current scope | Performance | Select+Y or Select+R3 | F7 |
| Toggle looping for the selected phrase | Phrase Editor | Select+X or L3 | F5 |
| Clear a queued phrase | Phrase Editor | Select+B or R3 | F6 |

The Brick button combinations use the existing raw button map. For
direct stick clicks, press each click separately and find the line
`INPUT: unassigned Brick button index` in the Khaṇa launch log. Set
`KHANA_L3_BUTTON` and `KHANA_R3_BUTTON` in the launcher using the distinct
observed joystick button indices above 8. The default NextUI log is
`/mnt/SDCARD/.userdata/tg5040/logs/Khana.txt`.

On Performance, cycle **NOW → 1/8 → 1/4 → BEAT → BAR**. While running,
**A** commits immediately on NOW or queues the highlighted chord for
the next boundary on a quantized grid. The latest
commit replaces the pending chord. Changing the grid leaves an already
queued chord at its scheduled position. The indicator flashes wider on
the downbeat and shows the pending chord and remaining quarter-note
beats. Recording captures the chord at its landing beat. Stopping clears
the pending chord. While stopped, **A** commits immediately. **BEAT**
follows the meter, including dotted quarters in 6/8, 9/8, and 12/8.
Recording and phrase-duration editing retain their separate grids.

In Phrase Editor, **A** starts or queues playback. With **LOOP ON**, a
phrase wraps at its edited duration, including off-grid lengths.
Identical chords across the seam preserve existing performer gates. A
queued phrase takes over at the next transport bar boundary. Clear Queue
cancels just the queued phrase.

Hold keeps pattern identity while pitches follow harmony. Vary takes
effect at the next performer boundary. Held patterns return after the
variation.

## Settings and performers

From stopped Performance, press **Select+X** on Brick or **M** on desktop.
Left and Right choose the field. Up and Down change its value.

| Field | Location from Tonic | Options |
|---|---|---|
| Tonic | Initial field | Twelve keys |
| Mode | Right once | Existing modes plus Harmonic Minor and Melodic Minor |
| Chord Hold | Left six times | Auto, 4 Bars, 8 Bars, 16 Bars, Until Change |
| Bass Arrival | Left five times | Root, Voice-led |
| Voicing | Left four times | Smooth, Close, Open |

Melodic Minor uses the ascending scale in both directions. Chord Hold
applies to low-density sustain patterns. Actual harmony changes release
incompatible pitches and can keep shared notes sounding. Voicing styles
realize every chord pitch class exactly once. Voice-led Bass weighs step
size and register when choosing a root, third, or fifth on arrival.
Bass families now include a syncopated pedal, two-bar octave answers,
and a walking turnaround. Connect figures can prepare the known chord
from a queued live change or phrase. Melody adds delayed answers and
long notes across the barline.

## Session restore

Settings, current chord, sync preferences, MIDI channels and transpose,
clock out, performer macro values, voicing options, both grids, and the
phrase bank with loop flags save automatically. The next launch opens
stopped with empty playback and recording queues. Press **Start** to
arm MIDI In or resume with Link. The raw MIDI output target follows the
existing launch configuration and device discovery.

The Brick default is `/mnt/SDCARD/.userdata/tg5040/khana/session.json`,
outside the Pak. Desktop uses `$XDG_STATE_HOME/khana/session.json`, or
`~/.local/state/khana/session.json`. Set `KHANA_SESSION_PATH` to override
the location. Valid `KHANA_MIDI_CHANNELS` and `KHANA_MIDI_TRANSPOSE`
launcher values take precedence over saved values. Quit cleanly to flush
the latest edits.

## Build and verify

Run `./scripts/package-khana.sh` for a standard Brick package. For the
existing local Link-enabled image with position-independent code enabled:

```sh
CROSS_CONFIG="$PWD/Cross-link-local.toml" KHANA_LINK=1 ./scripts/package-khana.sh
```

Exit Khaṇa, back up any custom launcher settings, and extract
`Khana.pak` into `Tools/tg5040/` on the SD card. Check the launch log
for v0.12.1. In the Link package, confirm **LINK** appears in the Sync
field and verify peer discovery on the same network.

1. Record with live grid BAR. Queue two chords before the downbeat and
   confirm the latest one lands and appears at that beat in the editor.
2. Play an off-grid loop, listen through the seam, and queue another
   phrase. Confirm replacement on the next transport bar.
3. Compare Chord Hold 8 Bars with Auto at low Chords Density. Try Hold
   and Vary with moderate Melody Density, and compare the Bass Arrival
   and Voicing options on the same progression.
4. Change a MIDI channel and transpose, set Loop on a phrase, and
   relaunch. Confirm the edits return and transport is stopped. Validate
   MIDI In and Link with an external clock or peer.
