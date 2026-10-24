# yo_dawg

A terminal MIDI sequencer for **macOS and Linux**, featuring a piano roll, tracks, built-in audio, and external MIDI instruments.

## Getting started

Requires a recent stable Rust toolchain (edition 2024).

```sh
cargo run --release -- --demo
```

On Linux, install the ALSA development dependencies and build tools first. For Debian/Ubuntu:

```sh
sudo apt install build-essential pkg-config libasound2-dev
```

macOS uses CoreAudio/CoreMIDI. The minimum terminal size is 70×20; 120×35 or larger is recommended. Mouse input requires a terminal that supports mouse reporting.

```sh
# List audio devices and MIDI ports
cargo run --release -- --devices

# SoundFont: full instruments instead of the basic built-in synthesizer
cargo run --release -- --soundfont /path/to/bank.sf2

# Monitor external audio: use indices from --devices; start at low volume
cargo run --release -- --input 0 --output 0

# MIDI only, without an audio device
cargo run --release -- --no-audio

# Open a saved project
cargo run --release -- --project song.yodawg.json
```

No SoundFont is bundled with the repository; supply your own appropriately licensed SF2 file. Without an SF2, a basic polyphonic synthesizer is used. Its four timbres repeat across program numbers and do not emulate General MIDI.

Instruments are displayed as `<n>_<name>`, for example `1_Soft_Sine`. With an SF2 loaded, preset names come from the bank, with spaces replaced by `_`. The `_Fallback` suffix indicates that another preset is substituted for a missing program. Without an SF2, the four timbres are `Soft_Sine`, `Bright_Sine`, `Rich_Sine`, and `Pure_Sine`. Press `p` to open a list of 128 programs with the current one highlighted: `↑/↓` move the selection, `PageUp/PageDown` move by one page, and `Home/End` jump to the beginning/end. `Enter` applies the selection; `Esc` cancels. Browsing does not change the instrument. External preset names are not automatically retrieved over MIDI; they are shown as `<n>_External_Program`, without assuming General MIDI mappings.

## Keyboard shortcuts

| Keys | Action |
| --- | --- |
| `Space` | Play/stop looped playback |
| Arrow keys, `PgUp/PgDn` | Move the cursor, shift by octaves |
| `Enter`, `Delete` | Add/remove a note, delete the selection |
| `v`, `Shift+click`, `Ctrl+A` | Select a note, multiple notes, all notes |
| `Shift+arrows` | Move selected notes |
| `Ctrl+←/→` | Change note length |
| `Ctrl+↑/↓` | Change velocity |
| `Ctrl+C/X/V` | Copy/cut/paste at the cursor |
| `Ctrl+Z/Y` | Undo/redo |
| `g`, `[` / `]`, `+` / `-` | Quantize, grid resolution, zoom |
| `Tab`, `Shift+Tab`, `a` | Select a track, add a track |
| `m`, `s`, `r` | Mute, solo, Internal/External/Both routing |
| `n`, `p`, `c`, `<` / `>` | Track name, instrument picker, MIDI channel, track volume |
| `b`, `{` / `}` | Tempo, loop start/end in bars |
| `l` (lowercase L) | Loop length in bars: `1` means one bar |
| `R` | Toggle MIDI recording; notes are recorded during playback |
| `Ctrl+S/O`, `Ctrl+E/I` | Save/open a project, export/import MIDI |
| `F2`, `F5` | Routing panel, refresh/reconnect MIDI ports |
| `F6`, `F7` | Cycle the MIDI input and selected track's MIDI output (including none) |
| `F8`, `F9` | External MIDI Thru, audio input monitoring |
| `!` | Panic: turn off all notes |
| `?`, `q` | Help, quit with confirmation for unsaved changes |

Mouse: click an empty cell to draw a note. Drag a note's body to move the selection or its right edge to resize it. Right-click to delete a note. The wheel scrolls pitches, `Shift+wheel` scrolls time, and `Ctrl+wheel` zooms.

The loop end is **exclusive**: start `1` and end `5` define four bars. Currently, only a constant tempo and a 4/4 time signature are supported.

For a short passage, press `l`, enter the number of bars, and press Enter. For example, `l` → `1` → Enter sets a one-bar loop from the loop start. Numeric fields are initially selected, so typing replaces the old value instead of appending to it. The end is marked `END`, and the area outside the loop is dimmed; notes there are preserved. This sets the shared playback length for all tracks; independent patterns are not yet supported.

## M-VAVE FM-1

1. Connect the device over USB-C or establish a BLE MIDI connection using your OS tools.
2. Press `F5`, then `F7` to select its MIDI output for the track.
3. Press `r` to select `External` or `Both`; use `c` to match the FM-1's receive channel.
4. To record the FM-1 keyboard, select its MIDI input with `F6`, arm recording with `R`, then press `Space`.

On macOS, BLE MIDI is usually connected through **Audio MIDI Setup → MIDI Studio → Bluetooth**. Linux requires a working system BLE MIDI bridge/stack that exposes the device as an ALSA MIDI port. The application uses ports already exposed by the OS; it does not perform BLE pairing itself. FM-1 compatibility with Linux requires verification on actual hardware.

**MIDI does not transmit audio.** The FM-1 has its own synthesizer, speaker, and 3.5 mm headphone output. USB Audio is not advertised by the manufacturer. To hear its actual sound through the computer, connect the 3.5 mm output to a suitable audio interface line input and start the application with `--input N`. Use an appropriate stereo cable/splitter and match signal levels. Plugging a headphone cable into the FM-1 disables its built-in speaker.

The DAW's internal synthesizer and input monitoring can run simultaneously. The `Both` MIDI route can play through the FM-1's built-in speaker and the computer without an audio cable; the two sound sources have independent timbres.

Do not send the same notes to the FM-1 over both USB and BLE simultaneously. External live Thru is disabled by default to avoid doubling locally played notes. For a separate controller and external synthesizer, enable it with `F8`. Avoid creating a hardware MIDI Thru feedback loop.

## Current limitations

- This is an initial working version, not a replacement for a full DAW: there is no pattern arranger, audio clip support, plugin hosting, audio export, or MIDI CC editing.
- Up to 16 tracks with independent internal instruments. External MIDI ports and channels are selected per track. Different timbres on multiple FM-1 channels are not guaranteed; multitimbrality is unconfirmed.
- MIDI files carry notes, programs, volume, and a constant tempo. Automation and other messages are not yet imported. SMPTE timing, format 2, tempo changes, and time signatures other than 4/4 are rejected.
- Stopping, changing routing, or editing resets sounding notes to prevent stuck voices. Long notes are cut off at the loop boundary. Recording across the boundary also truncates the note at the loop end.
- Audio monitoring uses basic linear sample-rate conversion. There is no adaptive compensation for independent hardware clock drift, buffer-size configuration, or recording latency compensation. Prefer the input and output of the same interface. Input is enabled only via `--input` to avoid accidental acoustic feedback.
- MIDI outputs are saved by system port name. After disconnecting a device, reconnect it in the OS and press `F5`.
- The MIDI input connection, Thru setting, and audio devices are session settings and are not saved in the project.
- Linux builds and hardware compatibility, as well as actual FM-1 operation over USB/BLE, require separate verification.
