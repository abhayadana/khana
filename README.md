# BrickMusic starter

First desktop prototype for the TrimUI Brick Pro / NextUI MIDI project.

## Prerequisites

```bash
sudo apt update
sudo apt install build-essential git curl pkg-config libsdl2-dev libasound2-dev alsa-utils
```

Install Rust if needed:

```bash
curl --proto '=https' --tlsv1.2 https://sh.rustup.rs -sSf | sh
source "$HOME/.cargo/env"
```

## Run

```bash
cd brickmusic-starter
cargo run
```

Controls:

- Z -> MIDI C4 (60)
- X -> MIDI D4 (62)
- A -> MIDI E4 (64)
- S -> MIDI G4 (67)
- Arrow keys -> temporary D-pad simulation
- Esc -> quit

## Verify MIDI

Keep BrickMusic running and open a second terminal:

```bash
aconnect -l
aseqdump -l
```

Find the BrickMusic MIDI client:port, then monitor it:

```bash
aseqdump -p CLIENT:PORT
```

Press Z/X/A/S and you should see Note On/Off messages.

This proves the desktop stack: Rust/Cargo, SDL2, input, ALSA MIDI, and MIDI generation.
# khana
