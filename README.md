# yo_dawg

A terminal MIDI sequencer for macOS and Linux with a piano roll, up to 16 tracks, built-in synthesis, and external MIDI support.

## Run

Requires a recent stable Rust toolchain. On Debian/Ubuntu, install `build-essential`, `pkg-config`, and `libasound2-dev` first.

```sh
cargo run --release -- --demo
```

Use a terminal of at least 70×20 cells; 120×35 or larger is recommended.

```sh
# Open a project
cargo run --release -- --project samples/neon_terminal_preview.yodawg.json

# Use a SoundFont bank
cargo run --release -- --soundfont /path/to/bank.sf2

# List audio devices and MIDI ports
cargo run --release -- --devices

# Monitor an external audio input
cargo run --release -- --input 0 --output 0

# MIDI only
cargo run --release -- --no-audio
```

Without a SoundFont, the built-in synthesizer provides four basic timbres. No SF2 bank is bundled.

## Controls

| Keys | Action |
| --- | --- |
| `Space` | Play / stop |
| Arrows | Move the cursor |
| `Enter` / `Delete` | Add / remove notes |
| `Shift+arrows` | Move selected notes |
| `Ctrl+←/→`, `Ctrl+↑/↓` | Note length, velocity |
| `[` / `]`, `+` / `-` | Grid resolution, zoom |
| `Tab` / `Shift+Tab`, `a` | Select / add a track |
| `p`, `m`, `s` | Instrument picker, mute, solo |
| `b`, `l` | Tempo, loop length in bars |
| `Ctrl+C/X/V`, `Ctrl+Z/Y` | Copy / cut / paste, undo / redo |
| `Ctrl+S/O`, `Ctrl+E`, `F3` | Save / open project, export / import MIDI |
| `!`, `?`, `q` | Panic, full help, quit |

Click to draw or select notes; drag the body to move them or the right edge to resize. Right-click deletes. The wheel scrolls pitches, `Shift+wheel` scrolls time, and `Ctrl+wheel` zooms.

File dialogs support arrows, PageUp/PageDown, `Enter` to confirm, `Tab` to switch to the path field, and `Esc` to cancel. Existing files require overwrite confirmation. `Ctrl+I` also imports MIDI in terminals that distinguish it from `Tab`; otherwise use `F3`.

## MIDI and audio

Connect the device by USB or establish BLE MIDI using your OS tools. On macOS, use **Audio MIDI Setup → MIDI Studio → Bluetooth**. Linux needs the device exposed as a system MIDI port.

- `F2`: routing panel; `F5`: refresh ports.
- `F6`: MIDI input; `F7`: selected track's MIDI output.
- `r`: Internal / External / Both; `c`: MIDI channel.
- `R`, then `Space`: record incoming MIDI notes.
- `F8`: external MIDI Thru, off by default; `F9`: audio input monitoring.

Incoming MIDI notes are highlighted on the piano roll. Match the output channel to your device's receive channel and avoid sending duplicate notes over USB and BLE.

MIDI does not transmit audio. To hear the FM-1's own sound through the computer, connect its 3.5 mm output to an audio interface and use `--input N`. USB Audio support is not confirmed. The `Both` route plays the internal and external synthesizers independently.

## Limitations

Fixed 4/4 and constant tempo. No independent patterns, audio clips, plugins, audio export, or MIDI CC editing. Audio monitoring has basic resampling without clock-drift or latency compensation. Linux and FM-1 USB/BLE hardware compatibility require verification; FM-1 multitimbrality is unconfirmed.
