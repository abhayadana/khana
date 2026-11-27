# Khaṇa v0.11.6 MIDI routing and external sync

Baseline commit `d7a172370f528597d4f68f5659be6fa923a22126` on GitHub main.

## Assign performer channels

Stop playback on Performance. Hold **Select** and press **X** on Brick
(**M** on desktop). Move right from **Tonic** six times to reach **Bass MIDI
Channel**. Continue right for **Chords**, **Texture**, and **Melody**. Each
field wraps through channels 1–16 with Up/Down. The performer rows show the
actual output channels. The default mapping is Bass 1, Chords 2, Texture 3,
Melody 4.

The four performers use the selected MIDI output. They can share a channel.
If two performers hold the same pitch on that channel, the first attack is
heard and the pitch releases when the last performer lets it go.

Settings changes last for the current session. To start each session with a
specific mapping, set `KHANA_MIDI_CHANNELS` in the launcher before `exec`.
Enter four comma-separated channel numbers in Bass, Chords, Texture, Melody
order, for example `export KHANA_MIDI_CHANNELS=5,8,8,16`. An invalid value
falls back to the default mapping and is logged.

The existing MIDI Panic gesture sends CC120 and CC123 once on each currently
assigned channel. Raw reconnect cleanup targets the channels whose notes may
have been sounding before the fault, including a channel reassigned while the
port was disconnected. If you deliberately switch away from a disconnected
destination, clear any stuck notes on that device directly.

## Follow an external MIDI clock

1. Connect the clock source to a raw MIDI input on the Brick, for example through
   QT-2host. Configure the source to transmit clock and transport.
2. In stopped Settings, go right four times from Tonic to **Sync** and select
   **MIDI IN**. Return to Performance. The header shows the selected input
   and **WAIT CLOCK**, **WAIT START**, **ACQUIRING**, or **LOCKED**.
3. Start transport at the source. The first clock after MIDI Start starts
   Khaṇa at beat zero. MIDI Stop stops playback and releases notes. MIDI
   Continue resumes the retained pulse count.

Clock pulses alone provide tempo while stopped. Start the external transport
before recording or launching a phrase. Khaṇa stops and clears sounding notes
if the input disconnects or clocks lapse for about 750 ms at normal tempos.
Send a fresh MIDI Start or Continue after recovery. Pressing **Start** locally
while following disarms local playback. Press **Start** again to re-arm, then
restart or continue at the external source. A new MIDI Start resets the beat
timeline and clears unfinished capture. Completed phrases remain available.

Input selection is independent of MIDI Output. AUTO favors a unique USB
input and waits when several candidates match. To specify one, set
`KHANA_MIDI_CLOCK_IN_PORT` to a case-insensitive name substring, such as
`QT2host`, or a raw node path, such as `/dev/snd/midiC1D0`. The setting applies
on entering MIDI In. This release follows raw MIDI on Linux. MIDI Song Position
Pointer and ALSA sequencer input are future work.

Khaṇa sends performed notes to its output in MIDI In mode and blocks Clock
Out to prevent a clock loop. Stopped tempo edits are disabled in MIDI In.

## Use Ableton Link

Build the optional Link variant for Brick:

```sh
KHANA_LINK=1 ./scripts/package-khana.sh
KHANA_LINK=1 BRICK_HOST=root@YOUR_BRICK_IP ./scripts/deploy-khana.sh
```

Use either the SD card package or SSH deployment. The Link variant uses the
NextUI SDK C++ runtime (`libstdc++.so.6`). The regular package command builds
the smaller Internal/MIDI In variant. On desktop, build with
`cargo run --locked --features ableton-link`.

Put the Brick and Ableton Live on the same local network. Turn on Link in
Live and select **Link** in Khaṇa's stopped **Sync** field. The header shows
the peer count and session tempo. A local **Start** queues entry at the next
bar in Khaṇa's meter. **WAIT** marks a pending launch, and pressing **Start**
again cancels it. To share transport, enable **Start Stop Sync** for both
apps. Selecting Link joins the session without publishing a Stop.

For a hardware bridge, enable **Clock Out** in Khaṇa while stopped. A queued
Link launch prepares the outgoing MIDI Start ahead of the launch beat. If
Khaṇa receives a peer Start that is already in effect, clock output joins
at the next bar. Match the meters where bar alignment matters.

## Brick acceptance

1. Move Bass to channel 16 and another performer to the same channel. Confirm
   note-on and note-off on a MIDI monitor. Stop playback and check that
   panic targets each assigned channel once.
2. Select MIDI In. Test Start, Stop, Continue, a tempo change, and unplug
   and reconnect. Look for **LOCKED** at steady tempo and verify that no
   clock is retransmitted. Compare output onsets to the source's MIDI clock.
3. Run the Link build with Live on the same network. Verify peer discovery,
   shared tempo, local launch and cancellation, and shared Start/Stop with
   both peers opted in. With Clock Out enabled, test a receiver on QT-2host.
