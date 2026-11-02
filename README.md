# yo_dawg

A terminal MIDI sequencer for macOS and Linux with piano roll and tracker views, up to 16 tracks, VST3 synthesizers, and external MIDI support.

## Run

Requires a recent stable Rust toolchain. On Debian/Ubuntu, install `build-essential`, `pkg-config`, `libasound2-dev`, and `libxcb1-dev` first.

```sh
cargo xtask bundle -p yo_dawg_basic -p yo_dawg_tb303 --release
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

Basic and TB-303 are standalone VST3 plugins, built into `target/bundled/`. The app loads them through the same host as third-party plugins. For a relocated application, put the bundles in a `plugins/` directory beside its executable. They can also be installed in another DAW's standard VST3 directory.

Press `p` to choose a synthesizer for the selected track, then `Enter` for its instruments or parameters. Use `←/→` to adjust parameters and `Esc` to return. Basic has 12 timbres; unused programs show `--`. `--soundfont` loads an SF2 bank inside Basic instead. No SF2 bank is bundled.

`F10` or `Shift+P` toggles the selected track's synth panel inside the terminal. Ratatui and `tui-slider` render parameter cards, sliders, switches, and the TB-303 waveform selection; no external windows are opened. Use `↑/↓` to select, `←/→` to adjust, `Shift+←/→` for fine changes, `Enter` to toggle switches, and `0` to restore a parameter's default. `Ctrl+Z/Y` undo/redo and `Space` playback work inside the panel. Third-party VST3s use the same TUI for their exposed parameters; their graphical editors are not loaded. Parameter changes and plugin state are saved in the project.

TB-303 is an acid-inspired monophonic synth with saw/square waves, a resonant four-pole filter, envelope modulation, decay, accent, and tuning. With Legato slide enabled, overlapping notes glide; velocity 100 or above triggers accent. It is not a circuit-exact Roland emulation.

Third-party VST3 bundles are found in the OS's standard directories and in `YO_DAWG_VST3_PATH` (a colon-separated directory list). Only native plugins for the current OS and CPU work. Each track has its own instance; plugin state and parameter values are saved in JSON projects, not MIDI exports. Missing plugins leave the track silent and report an error without replacing its saved settings.

## Controls

| Keys | Action |
| --- | --- |
| `Space` | Play / stop |
| `F4` | Switch piano roll / tracker |
| Arrows | Move the cursor |
| `Enter` / `Delete` | Add / remove notes |
| `Shift+arrows` | Move selected notes |
| `Ctrl+←/→`, `Ctrl+↑/↓` | Note length, velocity |
| `[` / `]`, `+` / `-` | Grid resolution, zoom |
| `Tab` / `Shift+Tab`, `a` | Select / add a track |
| `p`, `m`, `s` | Synthesizer / instrument picker, mute, solo |
| `F10` / `Shift+P` | Open / close the selected track's TUI synth panel |
| `b`, `l`, `L` | Tempo, selected track length, project cycle length |
| `Ctrl+C/X/V`, `Ctrl+Z/Y` | Copy / cut / paste, undo / redo |
| `Ctrl+S/O`, `Ctrl+E`, `F3` | Save / open project, export / import MIDI |
| `!`, `?`, `q` | Panic, full help, quit |

Click to draw or select notes; drag the body to move them or the right edge to resize. Right-click deletes. The wheel scrolls pitches, `Shift+wheel` scrolls time, and `Ctrl+wheel` zooms.

In tracker view, time runs down the rows and tracks use horizontal note columns. `↑/↓` move through time, `←/→` select columns, and `PageUp/PageDown` move by a page. `e` edits pitch, velocity, length, and start time; `Shift+Enter` adds a chord voice. `Alt+↑/↓` changes the entry pitch. File, instrument, playback, and editing commands are shared. Switching views preserves project data and each view's scroll position; chords and off-grid timing are not converted or quantized. Start directly in tracker mode with `--view tracker`.

Each track can loop independently. Press `l` and enter bars (`1`, `3`, etc.) or ticks (`480t`); `0` inherits the project cycle, shown with `~`. `L` changes the project cycle. For example, 3-bar and 5-bar tracks keep their own phases across project-cycle boundaries. Notes outside a shortened track are preserved but not played; sounding notes are cut at that track's boundary. Both views show each track's length and local playhead. JSON projects retain independent lengths; MIDI export bakes repetitions into a finite arrangement covering at least the project cycle and longest track. MIDI import is linear.

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

Fixed 4/4 and constant tempo. No independent patterns, audio clips, audio export, or MIDI CC editing. VST2, external plugin windows, crash isolation, automation lanes, and latency compensation are not implemented. Plugin-specific features not exposed as VST3 parameters cannot be controlled by the TUI. Plugins execute native code: only open trusted plugins and projects. Plugin audio is buffered on a worker thread. Audio monitoring has basic resampling without clock-drift compensation. Linux and FM-1 USB/BLE hardware compatibility require verification; FM-1 multitimbrality is unconfirmed.
