# Khaṇa — project handoff

**Snapshot:** 2026-11-26 (Pacific time).  
**Purpose:** Provide enough context for an LLM with no previous conversation to continue development without reopening settled design decisions.  
**Status:** v0.11.3 has been cross-compiled, installed, and confirmed to send MIDI to an external synth on the physical Brick Pro. v0.11.4 is a delivered source patch, not yet confirmed compiled, applied, committed, or tested on the user's machine. Do not conflate these states.

## 1. Immediate next steps and authoritative source

- Repository: **https://github.com/abhayadana/khana**, branch `main`.
- Latest GitHub commit verified at handoff: **`5a25a8ca43e0f15688c65ed7f36fb137ae8bc675`** (`working midi`, v0.11.3), tree `0bd8a9aaa64032553ea633e28d3e5aa77dab4021`.
- Latest delivered build: **v0.11.4**, `khana-v0.11.4-musical-worker-5a25a8c.patch`. Attach this file together with this handoff if it has not been committed. SHA-256: `a88b4c79ed55e826b37e8c58a4a496a1db9622f94ba5986bcb53d0fb934155e9`. The actual patch was inspected for this handoff; `git apply --stat` reports 12 files, 816 insertions, 259 deletions.
- Previous build preparation reported all 26 old-line hunk contexts verified against GitHub, git apply/reverse and whitespace checks on context fixtures, and launcher shell syntax checks. **Not a full checkout compile or test.** Rust was unavailable in that preparation environment. Do not treat this as evidence that v0.11.4 works.
- User explicitly said v0.11.3 sends MIDI out to a synth through QT-2host. No v0.11.4 validation results yet; no full v0.11.3 GUI/controls/hot-plug/timing acceptance report yet.

**First action in a new thread:** fetch the live GitHub `main` commit and inspect its actual files. If it is still the above SHA, ask for or use the latest patch and validate v0.11.4; do not generate v0.11.5 against the old baseline as if v0.11.4 were committed. If user provides compile/test errors, prioritize a corrective patch against the exact relevant source. If the user has pushed newer code, that becomes the authority. Never assume previous patches were applied or `cargo fmt` left source unchanged.

### Standing development workflow

User wants **git-apply-compatible unified diffs**, preferably downloadable `.patch` files, based on actual GitHub source. Never push/commit remotely unless specifically authorized. Avoid speculative replacement context, stale zip baselines, or approximating `cargo fmt`. Earlier fixes failed to apply for precisely this reason. For each release: identify commit, version, scope and files; patch source, tests, README/version metadata and supporting docs; validate exact contexts and `git apply --check`, then compile/format/Clippy/test if tools permit. Report any limitation explicitly. Keep changes incremental and usable on both desktop and Brick. No silent scope changes. User values concise, practical test instructions and release bullets when requested.

Use GitHub connector/API (if available) to fetch `branches/main`, `git/trees/<tree>?recursive=1`, and individual files by exact commit SHA. Source is more authoritative than this design summary. Earlier assistant patch workflow had limited tool transfer and sometimes used exact-context fixtures; prefer a full clone and real builds whenever possible. Do not invent file links, successful tests, or repo edits. Some empty files in the repo have names resembling compiler locations (e.g., `src/performer.rs:932:5`); leave them alone unless cleanup is requested.

## 2. Product and musical intent

Khaṇa (ASCII repo/binary/Pak name `khana` / `Khana.pak`) is a **performance-oriented MIDI harmony and phrase instrument** for TrimUI Brick Pro, with desktop development on Xubuntu. Primary device firmware is NextUI. It is not a DAW, conventional step sequencer, automatic song generator, audio synthesizer, or audio effects app. It proposes musical material; the user makes decisions. Core workflow:

```
hear possibilities → choose → shape → capture → revisit → vary
```

Aim for an expressive, compact handheld instrument with immediate interaction and a small number of coherent controls. Avoid feature creep, excessive menus, inaccessible controls, and random generation that merely changes notes. Harmonic stasis is valid: a player should be able to hold one or two chords for minutes and obtain compelling music through texture, rhythm, voicing, repetition, subtle variation, and energy. The four independent musical roles should sound like an ensemble, not four disconnected random arpeggiators.

### Musical references and why they matter

- CANON: context-aware harmonic continuation and auditionable choices.
- User's Chorder project (`https://github.com/abhayadana/chorder`): voicing, smooth movement, inversion/register concepts, root-bounce bass.
- DREAMSEQUENCE: independent performers, shared harmony, separation of rhythm and pitch intent.
- HANDS: pattern families, repetition, controlled chromatic resolution, articulation, timing, activity/space and temporary intensity; inversion-aware bass ideas.
- Piano Motifs: constrained pattern pools, target-aware bass, line clichés and slash bass, frozen interval motifs remapped to harmony, voice-led arpeggios, periodic contrast and cadential final bars, imitation/harmony, chaining.

These are conceptual references, not instructions to copy code or claim exact implementations. Prior listening references include Mogwai **D to E** (long harmonic stasis); Squarepusher **Iambic 5 Poetry** (warm keys, shuffle, space); Björk **Come to Me** (modal tension with small harmonic vocabulary); Radiohead **Reckoner** (interlocking rhythm); Ásgeir **Going Home** (motif repetition/arrangement growth); Fred again.. / Duskus / Four Tet / Skrillex **glow** (gradual energy over repetitive harmony). Prior exact chord transcriptions were not independently verified. Use these as musical qualities, not claimed transcriptions. User likes music that can evolve slowly, not incessant new chords.

Related hardware: TrimUI Brick Pro, denki-oto QT-2host (verified), MicroFreak, OP-Z, RK-006, Blokas MIDIHub and USB-C iPad music setup. Microloop is a separate Brick audio project (`~/Documents/GitHub/brick-music-starter`); use its established build/control lessons but do not merge its architecture into Khaṇa indiscriminately.

## 3. Software architecture and file map

Rust edition 2024. Cargo dependencies: SDL2 0.38.0, optional `midir` 0.11.0 (`alsa-sequencer`, enabled by default), optional `rusty_link` 0.4.9 (`ableton-link`). Brick build explicitly disables default features. No new dependency in v0.11.4. User used Rust 1.98.1 in previous development; check current toolchain if needed. Desktop and Brick binaries share musical logic.

| File | Responsibility |
|---|---|
| `src/main.rs` | `App`, actions, harmony/recording/editor integration, SDL lifecycle/input/render loop; v0.11.4 delegates music to worker |
| `src/runtime.rs` | New in v0.11.4 patch: `Worker`, owned `Frame`, request/reply messages, transport tick, MIDI servicing and shutdown |
| `src/music/chord.rs` | Pitch class, semantic chord types/intervals/extensions |
| `src/music/settings.rs` | TonalContext, modes, meter, MusicalSettings, SettingField |
| `src/music/recommendation.rs` | Contextual chord candidates, scoring, recommendation classes, reasons, reroll |
| `src/music/voicing.rs` | Complete chord voicings, inversion/spread/motion cost, fixed-capacity Voicing |
| `src/voice.rs` | Four voice roles, macro parameters, scope, VoiceEngine/VoiceRender |
| `src/performer.rs` | Stateful rhythm/pitch/articulation, pending events/gates, per-voice state, EnsembleContext, diagnostics |
| `src/phrase.rs` | Semantic chord-event capture, quantization, recorder/player/editor/grid, queued next chord |
| `src/sync.rs` | Internal monotonic clock and optional Ableton Link, tempo/transport |
| `src/midi.rs` | Backend selection, four-channel note ownership, output, smoke test, cleanup |
| `src/midi/raw.rs` | Raw device discovery, selection/pinning, nonblocking byte queue, reconnect and diagnostics |
| `src/input.rs` | Raw SDL handheld and desktop input translation, modifier/tap/hold/stick behavior |
| `src/ui.rs` | SDL bitmap/text rendering, Performance and Phrase Editor views |
| `scripts/` | Build, package, SSH deploy, local mock deployment test |
| `packaging/NextUI/Tools/tg5040/Khana.pak/` | Versioned Pak metadata and launch script |
| `docs/BRICK_V0.11.1.md` … `BRICK_V0.11.4.md` | Historical build and current platform acceptance plans (later docs supersede earlier behavior) |

Data flow:

```
TonalContext + ProgressionContext
  → candidate generation/scoring → Safe / Colorful / Bold recommendations
  → committed semantic Chord / active harmony → voicing
  → Bass / Chords / Texture / Melody
  → rhythmic/pitch-intent patterns and phrase state + scoped macros
  → scheduled VoiceEvent (NoteOn/NoteOff) → MidiEngine note ownership → backend
```

## 4. Accepted harmonic design

- User chooses from three columns (Safe, Colorful, Bold), three rows each. Classes are contextual novelty/risk, **not** tension levels. Resolve/Neutral/Tension is a separate direction preference. Surprise broadens candidates; Reroll changes recommendations within the constraints rather than arbitrarily jumping to a random chord.
- Chord semantics and displayed labels are invariant under performer macros. Color cannot silently turn C into Cmaj7. Diatonic seventh chords use stacked thirds; the major leading-tone seventh is half-diminished. Full 11th/13th vocabulary is deferred (storage already supports seven pitch classes).
- Candidate vocabulary includes diatonic chords, functional progressions, secondary dominants, modal interchange, chromatic mediants, diminished approaches and tritone substitutions. History (up to eight recent chords) and functional context matter. Recommendation reasons should be explainable. Chord-level voice-leading is a small preference; actual voicing has stronger movement costs. Earlier v0.7 weighting was progression 38/function 22/direction 18/source 10/history 7/voice-leading 5 plus jitter; verify current source before changing weights.
- Important audited cases: E7→Am is resolution; E7→A7 isn't that resolution. G7→A7 is not automatically a deceptive resolution just because root is scale-degree vi. Neutral does not penalize release. Modal functionality/enharmonics remain pragmatic/incomplete. Harmonic stasis and unresolved modal color are intentional, not failures.
- Do not infer a future chord during live improvisation. Queued/playing phrases can provide actual next harmony and scheduled beat. Same-chord reselect as intentional rearticulation is an unresolved design possibility.

## 5. Performers, macros and continuity

Channels are fixed: **Bass=1, Chords=2, Texture=3, Melody=4** (zero-based role indices 0–3 internally). Bass, Texture and Melody are presently monophonic; Chords polyphonic. User should reserve those channels for Khaṇa. No full routing matrix yet.

### Macros and scope

Scope ALL or one role. Density = structural activity, Dynamics = accent/velocity, Spread = register width/inversion/range, Color = pitch vocabulary and dissonance tolerance. Keep these dimensions distinguishable; e.g. high Density/low Color may be busy and consonant, low Density/high Color sparse and directional. Groove is a future separate global control, not another name for Density. Internally distinguish Activity/Space, Continuity and temporary Intensity. Avoid exposing every hidden parameter as a UI control.

Density changes update values immediately but pattern families change musically: Bass/Chords next bar or harmony change, Texture next bar, Melody next two-bar phrase. Density 0 cleanly mutes immediately and cancels pending attacks/resolutions. Reentry at appropriate boundary. Small parameter adjustments should preserve and develop existing pattern identity; large shifts may choose a new family.

### Bass

Density families: SILENT → ANCHOR → FOUNDATION → MOVING → CONNECT. Root sustain/pedal, root–fifth, root–octave, root bounce, broken chord/ostinato, scale/chromatic approach, walking fragment. Generally state root on harmonic arrival. CONNECT is directional, not simply busiest. Known-target connector only when actual next chord imminent (current audit passes next chord beat and targets within one beat). Long held chords must not create false arrivals at every bar. Unknown future harmony means local motion. Later add inversion/slash, pedal retention, line clichés, stepwise bass, persistent semantic-degree motif (prefer semantic interval/degree identity over fixed semitones).

### Chords

**Non-negotiable current semantic rule:** every chord pitch class occurs exactly once in Chords, no omission, no doubling, regardless of Bass. Triad=3, seventh=4, ninth=5, future 11th/13th=6/7. Bass may independently duplicate root. Chords uses fixed-capacity `Voicing` (7 notes); Spread affects register/inversion/width without altering membership. Avoid earlier superseded forced four-note voicings or arbitrary thinning/thickening by Color.

Density continuum SILENT → LONG SUSTAIN (2/4 bars) → BAR SUSTAIN → PULSE (2/4 attacks) → COMP (offbeat/backbeat/Charleston/anticipation) → ACTIVE (syncopation). A genuine harmony change interrupts incompatible old chord, even during long sustain. Future: retain common tones legato where musical, versus intentional rearticulation in rhythmic comping. Long gate must not lock out a newly selected density family.

### Texture

Persistent cyclic rhythmic identity, adapting pitches to harmony. SILENT → DRONE → SPARSE → PULSE → MOTION → ACTIVE. Current variants: LongDrone, BarDrone, IsolatedUpper, DelayedEcho, HalfPulse, QuarterPulse, ThreeStepArp, SlowArp, ExtensionFigure, Shimmer, RepeatingArp, RotatingThree. Drone long, pulse medium, arp legato, shimmer short. Rhythm independent of pitch mapping; phase normally continues through chord changes. Odd pattern lengths should rotate across meter instead of resetting each bar. Texture is currently mono, even if suggesting alternating dyads.

Relationships implemented: INDEPENDENT and COMPLEMENT (can leave space for recent Melody attack). Planned SHADOW/IMITATE/COUNTERLINE, voice-led arp, persistent upper-extension/drone/ostinato, independent pattern lengths, intentional contrast. Preserve recognizable identity even when other performers play.

### Melody

Currently two-bar phrases. Internal events contain onset/step, pitch intent, gate length, articulation; design goal includes structural importance and independent durations. Skeleton/Fill/Neighbor/Suspension/ModalTension/ChromaticApproach/Rest. Short/Normal/Legato/Held. Long structural peaks and cadential arrivals; shorter passing/neighbor/approach notes; suspension/modal tension can persist. Chromatic approach has explicit resolution target. Cadence arrives early enough for held gate to be audible before phrase ends.

Families MOTIF (Call, Pulse, Sparse Arch), PHRASE (Arch, Development/Wave, Answer), ORNAMENT (Flourish, Passing-dense, Chromatic phrase). Avoid uniform eighth notes or unrelated regeneration every two bars. Strong repetition should give A→A′ with same rhythm/rests/gates and a small nonstructural change. Future 1/2/4-bar lengths, coherent phrase grammar and repeat/varied ending/answer/new idea, motif locking, independent duration/contour/articulation.

### Ensemble and counterpoint

Shared advisory `EnsembleContext` holds representative active notes, recent attack times and motion direction. It informs candidates, does not centrally compose all roles. Soft preferences: avoid too-close register collisions, rhythmic complement, contrary/oblique motion, modest discouragement of repeated parallel perfect intervals, Color-dependent dissonance tolerance. Intentional doubling and modal friction allowed. Primary relationship Texture↔Melody, then Bass↔Melody; Chords chiefly register/rhythm. Don't enforce strict species counterpoint or give Melody absolute priority. Current representative-note approach and previous-direction heuristic are not full counterpoint: actual preceding intervals, durations, pending events, oblique handling and asymmetry need refinement.

### Known musical implementation caveats / audit targets

The v0.11 source has a useful first implementation, not final musical grammar. Earlier source audit identified absolute Texture modulo/step resolution and non-4/4 edge cases; variant changes can reset perceived phase. Melody skeleton/fill heuristics are less differentiated than intended rhythmic families; review low-Color chromatic eligibility and phrase contrast. Verify precise current code before addressing. v0.11 audit fixed real next-chord beat, harmony-change preservation (no global panic and no loss of Texture/Melody ownership), earlier cadence and pitch-vector reuse. Bass/Chords force new harmony, Texture/Melody finish gates or next attacks. v0.11.2 discards already-expired pending attacks and clears pending state on mute, logs lateness. Newly generated attacks still originate from poll time, not a full grid lookahead schedule. Do not advertise sample-accurate or hard-real-time MIDI.

## 6. Phrase and synchronization model

Capture semantic `ChordEvent` harmony/duration, not exact performed MIDI. Recorder supports arm/start/stop/next-phrase, phrase storage and current playback/queue. Phrase editor selects events, changes chord, duplicates/deletes, shortens/extends, changes grid and zoom. Current phrase playback reinterprets semantic events with current performers. No persistent phrase files or exact MIDI export yet. More advanced recall and arrangement are future.

Grid: 1/16=.25, 1/8=.5, 1/4=1, 1/2=2 **quarter-note units**, BEAT, BAR. Shorten snaps to strictly previous grid boundary; extend strictly next. On-grid moves one step. Off-grid duration stays intact until edited. Example 4.1 bars, BAR grid → shorten 4.0 / extend 5.0. BEAT compound 6/8, 9/8, 12/8 = dotted quarter 1.5 quarter beats; 7/8 eighth for now. No event nudging via grid currently.

Settings: tonic 12 pitch classes, seven modes, tempo 30–300 BPM, 2/4,3/4,4/4,5/4,6/8,7/8,9/8,12/8 meter, Internal/optional Link and Link start/stop sync. `SyncTransport` internal clock uses monotonic Instant and preserves beat over tempo changes. Link optional desktop only in current Brick build. MIDI clock input/output, Song Position Pointer and external sync not yet implemented.

## 7. UI and physical controls

Two screens: Performance (harmony recommendations, current chord/voicing, performers/macros, scope, recording and MIDI state) and Phrase Editor (semantic event timeline). Settings overlays/focus existing Performance header; keep simple. Native fullscreen logical 1024×768; 30 FPS default, 60 optional. Do not assume physical controls map identically on every SDL build; verify actual log.

| Control | Performance | Phrase Editor |
|---|---|---|
| D-pad L/R | Select recommendation column | Select chord event |
| D-pad U/D | Select recommendation row | Previous/next duration edit grid |
| A | Commit harmony | Play/queue phrase |
| B | Reroll | Change selected chord |
| X | Surprise | Duplicate chord |
| Y | Neutral direction | Return to performance |
| L1/R1 | Previous/next voice scope | Previous/next phrase |
| L2/R2 | Resolve/Tension | Shorten/Extend |
| Left stick X/Y | Density/Dynamics | — |
| Right stick X/Y | Spread/Color | — |
| Select+A | Record start/stop | — |
| Select+B | Arm next recording | — |
| Select+X | Settings, stopped only | Opens Performance Settings when stopped |
| Select+Y | — | Delete selected chord |
| Select+L1/R1 | — | Zoom out/in |
| Start | Transport | Transport |
| Menu tap | Switch screen | Switch screen |
| Menu hold ~700 ms | Exit | Exit |

Settings: left/right field, up/down value, A/Menu close; B is documented in initial controls, verify mapping. Start closes Settings and starts transport. v0.11.4 adds MIDI Output as last field: left from Tonic wraps directly there. L3/R3 reserved. **Do not assign Select+Menu** (Microloop experienced crash/system handling). Physical L2/R2 are digital despite raw SDL axes: L2 axis 2, R2 axis 5, release −32768, press +32767; no pressure sensitivity. Raw buttons B0/A1/Y2/X3/L1=4/R1=5/Select6/Start7/Menu8, raw D-pad hats, sticks axes 0/1 and 3/4. Relative stick changes with ~0.12 dead zone/50 ms tick: centered = no change, scope switching no absolute jumps. Select suppresses stick edits. Reconnecting joystick currently may require restart.

Desktop mapping: arrows recommendation nav; Z primary, X reroll, S surprise, Q/E/W direction (check exact mapping), C/V scope, T/G density, Y/H dynamics, U/J spread, I/K color, P capture, N arm next, Tab screen, Space transport, M Settings stopped, D duplicate, Backspace delete, F/R shorten/extend, A/L prev/next phrase, −/+ zoom, brackets grid, Escape quit. Inspect `print_controls`/`input.rs` for exact mapping before revisions.

## 8. Brick MIDI implementation: decisions and evidence

### Why raw MIDI

v0.11.1 used `midir`/ALSA sequencer; v0.11.2 added explicitly selected raw path and diagnostics, but still linked ALSA. Source review of comparable Brick apps supported making raw device nodes default and removing mandatory ALSA userspace dependency for the Brick. No libusb needed for ordinary class-compliant MIDI; kernel USB MIDI driver still required. The decision is now supported by actual Khaṇa hardware output.

| Reference | What was inspected / lesson |
|---|---|
| [SongVerk Studio / M8Tape](https://github.com/romainguerif/M8Tape/blob/master/app/sv_midi.c) | `/dev/snd/midiC*D*`, procfs names/directions, nonblocking writes, dedicated scheduler, retries ~1 s, stall diagnostics. Firmware ALSA userspace had problems even though raw nodes existed. Avoid copying device-specific Tonverk USB driver rebind hacks or its short-write handling. |
| [Twill Studio](https://github.com/LiminalKid/TwillStudio/blob/main/src/platform/sdl2/SystemMidiLinux.cpp) | Raw Linux MIDI, periodic scans, timestamped reader thread; direct writes, platform separation. Don't broadcast to every device. [NextUI launcher](https://github.com/LiminalKid/TwillStudio/blob/main/packaging/nextui/launch.sh) uses `/usr/trimui/lib` — proven useful here. |
| [brickio](https://github.com/sydilaxe/Trimui-Brick-Resources) | `lib/brickio/midi.c`, `lib/brickio/README.md`: raw, threaded parser, timed input, audio+MIDI, hot-plug; primarily MustardOS, verify portability. GPL-3.0-or-later; independently implement Rust logic. Useful standing reference for Microloop too. |
| [ChooChooTracker](https://github.com/paiheulevrai/Choochootracker/blob/main/chipnomad_lib/midi_io.cpp) | RtMidi ALSA sequencer; `tracker/Makefile.portmaster` builds NextUI from PortMaster binary with ALSA. Timed output queue drained ~1 ms on separate thread. ALSA sequencer is not inherently impossible on Brick. |
| [Brickchord](https://github.com/michaeltietjen/brickchord/blob/main/brickchord/midi.py) | Standard Python glob/open/read nonblocking raw MIDI **input**; not evidence of output. |

OZMA describes host/gadget MIDI; application implementation not inspected, no claim about its internal backend. Public Grampus/Euclidtrack Pak repos did not expose MIDI implementation; Macrocosm inspected Lua MIDI input is Norns-oriented, not proof of Brick output. USB gadget/network MIDI are future ideas, not requirements now.

### v0.11.3 (committed and physical MIDI verified)

Default raw backend if Brick, explicit path, or no sequencer feature. Desktop default sequencer (`midir`) with virtual fallback, explicit destination name matching; raw selectable. `KHANA_MIDI_DRY_RUN=1` takes precedence. Raw auto connects only if one eligible port; names from `/proc/asound/cardN/id`, USB id and sysfs location. Once connected, pin identity rather than card number alone; avoid silently changing devices. Identity is approximate for identical hardware. Open character device with `O_WRONLY | O_NONBLOCK` via Rust `OpenOptions`; 128-message FIFO, each 3-byte message offset tracked, retain unaccepted bytes on short write/WouldBlock/Interrupted, fault on queue full/stale >~30 ms/hard error. Rescan ~1 second, discard queued stale music on disconnect, clear ownership, reconnect same pinned identity and send CC120 All Sound Off on channels 1–4 before fresh events. Connected flush on stop/exit bounded ~30 ms; forced kill/unplug/stall cannot guarantee delivery. Do not treat queued ownership as physical audible confirmation. Explicit path/name conflict errors. Headless `--midi-list` and `--midi-test` before SDL initialization still require shared SDL library at process load.

Note ownership bitset per channel (`u128`); reattack releases same pitch first, stale unrelated offs ignored; stop releases all owned notes. No MIDI clock or panic command yet, recovery CC120 isn't an explicit user panic. Normal desktop sequencer behavior remains.

### v0.11.4 patch (not yet runtime verified)

- `src/runtime.rs`: worker thread named `khana-music` constructs `App`, `SyncTransport`, `MidiEngine` internally (no need for Link/midir handle Send); owns mutable musical state. Main owns SDL, raw input, draws/presents. `Command::Action` / `Snapshot` over mpsc, per-request one-slot reply; only one request in flight from UI. Actions return new `InputPage` and quit bool, apply transport transitions before ack. Nominal 1 ms polling via `recv_timeout`, wake early on commands, service MIDI before/after tick. `Frame` copies settings, recommendations, voice snapshots, selected phrase only in editor, destination, recorder state, diagnostics. No mutable state lock held while rendering. Drop sender wakes worker, then joins and cleanup. Request timeout 2 s; join itself **not bounded**. Worker errors propagate via shutdown/result or response error. Tests: advances without UI requests, acknowledges stop and zero ownership, screen/action context and quit.
- Existing beat scheduler preserved, **not** new timestamped lookahead scheduling. UI rendering cannot directly block clock, but recommendations, discovery, log printing, snapshot copying, MIDI sends/flush and action processing can still add jitter. Synchronous action/snapshot waits can delay UI; if worker blocked in driver, join can stall. UI render metric includes snapshot wait; worker `work` max measures loop tick/service, not all command work. Poll gap includes other delays. No measured jitter claim. Consider better metrics and bounded shutdown only if evidence warrants.
- New Settings `SettingField::MidiOutput`, raw-only; up/down cycles AUTO, currently enumerated eligible identities, OFF; only while stopped; release old owned notes first. AUTO prefers USB candidates over other devices, refuses multiple matches, pins until explicitly changed. `Selection::Identity` and `Off`. OFF suppresses discovery and send. Labels `RAW AUTO: ...`, `RAW SELECT: ...`, `RAW OFF`, `RAW WAIT ...`, `RAW LOST ...`. Session-local choice, no persistence. Explicit dry-run/desktop sequencer cannot be changed by this field. Leaving a faulted/disconnected device may require manual panic on the old synth; cannot clean remote device not present. Port identity fallback based on sysfs may change for non-USB/card renumbering. Verify behavior when explicit target absent from list and switching across hot-plugs.
- Raw listing parses `Output N`/`Input N` sections, first output Owner PID, product from first procfs line; classify USB by nonempty USB id, virtual via sysfs/`VirMIDI`, other/unknown. Actual open still definitive. Tests for procfs directions, virtual/USB auto preference, no output selection. No input parsing/receive support merely because IN capability reported.
- Native SDL path in launcher; committed `.cargo/config.toml` preserved. Version bumps Cargo.toml/lock, pak.json, launcher, README and docs. No new dependency. This patch changes no musical generation algorithm. Follow `docs/BRICK_V0.11.4.md`.

## 9. Verified build environment, deployment and troubleshooting

Local checkout: `~/Documents/GitHub/khana` on Xubuntu. Cross image/toolchain from working Microloop setup. **Two separate files** (don't merge the duplicate TOML target headings):

`Cross.toml`:
```toml
[target.aarch64-unknown-linux-gnu]
image = "ghcr.io/loveretro/tg5040-toolchain:main"
```

`.cargo/config.toml` (committed and proven working):
```toml
[target.aarch64-unknown-linux-gnu]
linker = "aarch64-nextui-linux-gnu-gcc"

[env]
CC_aarch64_unknown_linux_gnu = "aarch64-nextui-linux-gnu-gcc"
CXX_aarch64_unknown_linux_gnu = "aarch64-nextui-linux-gnu-g++"
AR_aarch64_unknown_linux_gnu = "aarch64-nextui-linux-gnu-ar"
```

`./scripts/build-device.sh` calls `cross build --locked --release --target aarch64-unknown-linux-gnu --no-default-features`. Use this target/config, not generic ARM64 SDL2. First attempt without explicit linker failed `rust-lld: error: unable to find library -lSDL2` because `cc`/default linker selected wrong SDK path. Fixed by `.cargo/config.toml`. Old GCC 8.3.0/ld.bfd emits `unsupported GNU_PROPERTY_TYPE (5) type: 0xc0000000` warnings with modern Rust metadata; successful build and physical run confirmed these warnings nonfatal for v0.11.3. No need to change linker simply to suppress them.

Verified v0.11.3 binary: ELF 64-bit LSB PIE, ARM aarch64, interpreter `/lib/ld-linux-aarch64.so.1`, GNU/Linux 4.19.0 ABI note, not stripped. Direct `NEEDED`: `libSDL2-2.0.so.0`, `libgcc_s.so.1`, `libpthread.so.0`, `libdl.so.2`, `libc.so.6`. No direct libasound. ABI note alone didn't prevent physical execution. SDL may have transitive dependencies; do not claim entire system is ALSA-free.

`./scripts/package-khana.sh` always builds incrementally then stages zip atomically; output `dist/Khana.pak.zip`, `Khana.pak/{khana,launch.sh,pak.json}`. `./scripts/deploy-khana.sh` requires `BRICK_HOST`, always packages first; stages via SSH/tar and swaps/attempts restoration on failure. `./build-brick.sh` alias. Existing mock test verifies fresh builds, failed build prevents SSH/overwrite, replacement. Exit app before deployment. Replacing Pak overwrites custom launcher or files inside; keep overrides in repo launcher/back up local files. Actual package installed at `/mnt/SDCARD/Tools/tg5040/Khana.pak`; manually extract to SD `Tools/tg5040/` or SSH. Brick SSH root works; IP is not specified in handoff.

```sh
cd ~/Documents/GitHub/khana
git status --short
git pull --ff-only
# only if still on the exact baseline and patch is not applied:
git apply --check /path/to/khana-v0.11.4-musical-worker-5a25a8c.patch
git apply /path/to/khana-v0.11.4-musical-worker-5a25a8c.patch
cargo fmt --all
cargo clippy --all-targets --all-features --locked -- -D warnings
cargo test --all-features --locked
cargo clippy --all-targets --no-default-features --locked -- -D warnings
cargo test --no-default-features --locked
./scripts/test-deployment-freshness.sh
./scripts/package-khana.sh
file target/aarch64-unknown-linux-gnu/release/khana
readelf -d target/aarch64-unknown-linux-gnu/release/khana | grep NEEDED
BRICK_HOST=root@YOUR_BRICK_IP ./scripts/deploy-khana.sh
```

Runtime failure in SSH initially: `error while loading shared libraries: libSDL2-2.0.so.0: cannot open shared object file`. **Verified fix:** prepend `/usr/trimui/lib` to `LD_LIBRARY_PATH`. Do not bundle generic SDL or force unverified `SDL_VIDEODRIVER=mali`. v0.11.4 launcher includes fix, but when directly invoking executable in SSH use:

```sh
cd /mnt/SDCARD/Tools/tg5040/Khana.pak
export LD_LIBRARY_PATH="/usr/trimui/lib${LD_LIBRARY_PATH:+:$LD_LIBRARY_PATH}"
./khana --midi-list
KHANA_MIDI_PORT=QT2host KHANA_MIDI_TRACE=1 ./khana --midi-test
# Or explicit raw path (card number can change):
KHANA_MIDI_RAW_DEVICE=/dev/snd/midiC1D0 KHANA_MIDI_TRACE=1 ./khana --midi-test
```

Close GUI/other MIDI apps before headless audition. Test sends 350 ms groups: MIDI ch1 C3 (48), ch2 C4 E4 G4 (60/64/67), ch3 C5 (72), ch4 E5 (76), then note-off and zero ownership. Synth must receive relevant channels. Use `KHANA_MIDI_BACKEND=raw` if needed on desktop with sequencer built. For no-hardware desktop: `KHANA_MIDI_DRY_RUN=1 cargo run --locked`; `cargo run --no-default-features --locked -- --midi-list`; `KHANA_MIDI_DRY_RUN=1 KHANA_MIDI_TRACE=1 cargo run --no-default-features --locked -- --midi-test`.

### Actual hardware evidence (v0.11.3)

Without device: raw nodes 0, executable starts, sequencer not built. With QT-2host connected via USB host:
```
ALSA sequencer backend not built
Raw MIDI device nodes: 1
  QT2host (/dev/snd/midiC1D0) [OUT / UNKNOWN] USB 239a:80cb
```
`/proc/asound/cards`: card 0 `audiocodec`, card 1 `QT2host`, USB-Audio `QT-2host`, denki-oto QT-2host at usb-sunxi-ohci-1, full speed. `/proc/asound/card*/midi*`: product QT-2host, Output 0, Tx bytes 0 at that inspection, Owner PID 4091, native mode, Buffer size/Avail 4096, Input 0, Rx bytes 0. Owner PID process was not identified in conversation. User subsequently explicitly confirmed synth receives MIDI. `/dev/snd/seq` present but unused by raw backend; `lsusb` not installed and not necessary. `dmesg` showed USB full-speed device enumeration. No need to change kernel modules or firmware based on these results. Desktop raw-only build also worked; listed four `VirMIDI` nodes incorrectly tagged USB by v0.11.3; v0.11.4 addresses that classification.

If no device appears, check cable/host port (top USB-C), power, then `ls -l /dev/snd`, `cat /proc/asound/cards`, `cat /proc/asound/card*/midi* 2>/dev/null`, `dmesg | tail -60`. Existing owner: `cat /proc/<PID>/comm`, `cat /proc/<PID>/cmdline`; don't indiscriminately kill processes. Use actual open result as output availability check, not only device listing. Do not copy SongVerk's device-specific USB unbind/rebind quirks unless separately justified.

### Runtime options and logs

| Variable | Effect |
|---|---|
| `KHANA_BRICK=1` / `PLATFORM=tg5040` | Handheld mode |
| `BRICK_UI_FPS=30` or `60` | UI cadence |
| `KHANA_INPUT_LOG=1` | Input tracing, disable for performance measurements |
| `KHANA_JOYSTICK_INDEX=0` | Raw joystick override |
| `KHANA_MIDI_BACKEND=raw` or `sequencer` | Backend override; sequencer feature required |
| `KHANA_MIDI_DRY_RUN=1` | Explicit no-output mode, still tests ownership |
| `KHANA_MIDI_PORT=substring` | Case-insensitive unique destination match; raw name on Brick |
| `KHANA_MIDI_RAW_DEVICE=/dev/snd/midiC1D0` | Explicit raw path, mutually exclusive with PORT |
| `KHANA_MIDI_TRACE=1` | Hex output messages/intended bytes, disable for timing tests |
| `LD_LIBRARY_PATH=/usr/trimui/lib...` | Native SDL runtime path |
| `BRICK_HOST=root@IP` | SSH deployment target |

Log `$LOGS_PATH/Khana.txt` fallback `/mnt/SDCARD/.userdata/tg5040/logs/Khana.txt`. Launcher sets `KHANA_BRICK=1`, raw default, FPS30 default, native SDL path in v0.11.4; passes `"$@"`. Direct shell invocation needs env manually. Startup version, SDL driver, display, joystick and destination. v0.11.3 `DIAG poll ... render ... expired attacks ... max attack lateness ... active MIDI ...`, raw bytes/queued/faults. v0.11.4 separate `MUSIC DIAG poll ... work ... expired attacks ... max attack lateness ... active MIDI ...`, `UI DIAG render ... music ticks ... active MIDI ...`. Compare under identical workload, traces off. Lateness in beats; expired attacks not total missed grid events; logs are proxies, not wire-level timing.

## 10. v0.11.4 acceptance plan (next concrete work)

1. Apply patch once, `cargo fmt`, all-feature and no-default-feature Clippy/tests; resolve compile/Clippy errors. Run shell regression and cross build/package. Commit verified fixes and push; use new SHA for next release. Do not rely on the previous assistant's static check as compilation.
2. Launch through NextUI (not only SSH); version, screen layout, physical mappings, D-pad hats, trigger digital press/release, stick relative updates, scope, Select modifier, Menu tap/hold. Verify rapid screen changes + subsequent input context. Avoid Select+Menu.
3. No hardware/dry-run, then QT-2host. Confirm four roles and exact chord membership. Settings stopped: AUTO, selected QT-2host, OFF, automatic pin, resume after switch; test ambiguous/missing targets, virtual classification on Xubuntu. Verify that OFF truly sends no notes and old output cleanly releases.
4. Start/stop repeatedly, exit playing, mute each voice to Density 0, reconnect same device, don't redirect to different synth. Inspect owned note counts, raw queue/faults, audible stuck notes. Unplug can't guarantee old synth receives offs; test reconnection cleanup and manual synth panic as needed.
5. Compare 30/60 FPS; play with continuous UI/rapid macro changes, screen switches, capture/edit/queue phrase. Check worker ticks increase and musical gaps/lateness remain low even if presentation slow. Compare against v0.11.3 when possible. No fixed real-time thresholds were accepted; measure before optimizing. Distinguish UI latency, worker scheduling latency and actual MIDI arrival jitter.
6. Desktop default ALSA, raw-only and dry-run; optional Link builds and timing tests. Inspect tests added in `runtime.rs` and `raw.rs`. If worker stalls, preserve evidence (logs, circumstances) and isolate driver/discovery/recommendation/logging costs before expanding scope.

## 11. Roadmap and rationale

Versions are the agreed working plan, not guaranteed release dates. Fix regressions before feature work; small maintenance version if appropriate. User wants hardware infrastructure stabilized so focus returns to musical playability.

| Version | Scope / status |
|---|---|
| v0.10.0 → v0.11.0 | Musical grid, semantic chord voicing, ensemble/rhythm/gates/counterpoint; audited, superseded source in GitHub |
| v0.11.1 | Brick foundation and build workflow; Clippy fixes committed |
| v0.11.2 | Headless MIDI diagnostics, raw explicit path, expired attack/mute fixes |
| v0.11.3 | Raw-first platform, optional sequencer, discovery, bounded queue, reconnect; cross-build + external synth verified |
| v0.11.4 | **Delivered patch, pending validation**: independent music worker, MIDI output Settings, discovery improvements, SDL launcher |
| v0.11.5 | MIDI clock output/transport, explicit panic; possible timing hardening based on v0.11.4 evidence |
| v0.12.0 | Musical continuity and persistent motifs, voice-led Bass/Chords/Texture/Melody, better phrase structure |
| v0.12.1 | Hold/Vary/Renew, groove, momentary Intensity and refined counterpoint |
| v0.13.0 | Musical recall/state persistence and reinterpretation |
| v0.14.0 | Phrase chaining, retrospective capture and arrangement/cadence structure |
| Later | MIDI export, presets, expanded harmony vocabulary, deeper sync/input/routing, USB gadget/network MIDI as justified |

### v0.11.5 — MIDI transport and panic (planned, not implemented)

Add optional MIDI Clock Out `F8` at 24 pulses per quarter note, Start `FA`, Continue `FB`, Stop `FC`; consider Song Position Pointer `F2` for nonzero beat positioning; define transport semantics deliberately. Add explicit user panic (CC123 All Notes Off, CC120 All Sound Off if needed; channels 1–4 only) without interfering with other instruments. Raw queue currently fixed three-byte messages; extend length handling and test interleaving/realtime bytes, short writes, expiry, reconnection, cleanup, clock phase. Use worker clock, not UI cadence or independent drifting clock. MIDI clock input/following is later (optional settings `INTERNAL / LINK / MIDI IN`, clock out off/on); avoid output echo loops. Decide controls/setting fields with user before assigning conflicting combos; physical triggers digital. Don't promise exact sync jitter. Confirm how stop/restart/reconnect should send transport and cleanup before implementing. Lookahead/timestamped queue remains a possible improvement, not part of v0.11.4.

### v0.12.0 — musical continuity

Priority: make modest harmonic material compelling. Bass root bounce, persistent repeating degree motif, pedal and stepwise inversion, connectors based on actual remaining time to known chord. Chords common-tone retention vs full rhythmic reattack. Texture voice-led arps, persistent ostinato/drone/upper-extension, independent pattern lengths/meter and recent Melody SHADOW. Melody 1/2/4-bar phrases with recognizable motif repeat, varied ending/rhythm/contour/articulation, answer/renew and meaningful rests. Preserve semantic harmony and role identities, re-evaluate gates, meter and density changes. Listening tests on one chord/two chords, longer durations, contrast low/high density/color, same rhythmic identity across harmony and no mechanically repetitive false cadences. Avoid over-engineering new UI before musical behavior sounds good.

### v0.12.1 — expressive performance gestures

Hold/Vary/Renew scoped ALL or role: Hold preserves identity, not literal pitch; Vary related; Renew genuinely distinct. Global Groove (Straight, Light Swing, Shuffle, Laid Back), separate from Density. Temporary momentary Intensity with ramps/accent/eligible extra events/range/articulation/less space, then return without overwriting macro values. Do not treat L2/R2 as pressure-sensitive. Plan bindings to avoid breaking Resolve/Tension or edit grid; no decided final bindings. Refine counterpoint with attack timing/gate durations/actual preceding intervals/contrary/oblique, purposeful doubling and modal dissonance. Keep patterns coherent rather than random omissions. Verify musical references above.

### v0.13.0 — capture and recall

Capture semantic harmony plus macro/motif identity, seed, groove and held state. Recall restores idea; Reinterpret uses current settings against captured material. Exact recorded MIDI, momentary Intensity history and full DAW editing not initial scope. Need persistent schema/versioning, validation and migration; path TBD, use user-data location that survives Pak replacement. Consider richer role relationships, not exhaustive controls.

### v0.14.0 — arrangement

Phrase chaining, retrospective capture, deliberate cadence and arrangement evolution. User remains director, not automatic composer. Later exports/presets/expanded extensions/Link refinements. Keep interaction simple enough for handheld.

## 12. Non-negotiables and uncertainty checklist for the next LLM

- Source of truth is current GitHub + current user local errors, not this document's historical descriptions. Retrieve exact commit before new patch. v0.11.4 latest patch is supplied separately, not reprinted here; old patches are superseded. Do not reconstruct current code from prose. If patch isn't available and repo still v0.11.3, request patch or have user push verified code.
- v0.11.4 was prepared but not verified by Rust compiler/device. Do not claim v0.11.5 ready without validation. Preserve cross config and native SDL loader fix. Use optional sequencer and raw-only Brick build.
- Current output hardware works (QT-2host). Physical GUI, controls, hot-plug and timing still need a complete test record. No reason to replace the proven raw MIDI backend. Don't confuse successful MIDI list with physical transmission; here both were explicitly confirmed for v0.11.3.
- Musical semantics: Safe/Colorful/Bold ≠ tension, Color ≠ chord extension, density ≠ velocity/groove, Hold ≠ fixed pitch, no fabricated future harmony, exact complete chord membership, meaningful sustained stasis. Preserve role/phase/ownership across harmony and graceful mute/stop.
- Threading: SDL on UI, musical worker owns mutable state, no draw lock, no added competing clock. Current worker still has non-real-time work and unbounded join; consider measured evidence and regression tests. No hard real-time claims. MIDI event ownership tracks software state, not a guarantee of delivery. Keep cleanup safe on disconnected devices.
- No new dependencies unless justified; update Cargo lock/version/README/Pak/docs consistently. Use proper Rustdoc/idiomatic style and `cargo fmt`, Clippy `-D warnings`, tests in both feature configurations. If Python helper scripts requested, use PEP 484 annotations for parameters/returns/variables, Google-style docstrings, comments explaining intent, PEP 8 and Black-clean.
- When code changes are requested, provide actual tested-as-far-as-possible patch file and application commands, explicitly state tests unavailable or pending, and don't silently edit the remote repository. If user requests Git-style diff inline, ensure `git apply` compatibility. User likes detailed design discussions before major changes, then incremental builds and practical hardware/listening tests.

## 13. Useful source references and companion artifact

Authoritative v0.11.3 source: https://github.com/abhayadana/khana/tree/5a25a8ca43e0f15688c65ed7f36fb137ae8bc675 . In particular, `README.md`, `docs/BRICK_V0.11.1.md`, `docs/BRICK_V0.11.2.md`, `docs/BRICK_V0.11.3.md`, `src/main.rs`, `src/midi.rs`, `src/midi/raw.rs`. Older docs describe superseded behavior; use newest source. Companion `khana-v0.11.4-musical-worker-5a25a8c.patch` contains exact current proposed changes and `docs/BRICK_V0.11.4.md`; attach it if not committed. Handoff intentionally summarizes architecture and decisions instead of embedding a second full patch or obsolete historical archives. Earlier design/audit context is historical; any fine implementation detail should be checked in current source.

**Suggested opening request for a new thread:** “Read the attached Khaṇa handoff and latest patch. Check the current GitHub main baseline, then help me validate v0.11.4 or, if it is already validated and committed, plan/build v0.11.5. Always base git-apply-compatible patches on exact GitHub source and disclose validation limitations.”
