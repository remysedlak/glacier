# glacier
A DAW built from scratch in Rust as a deliberate learning project. No frameworks — raw wgpu, CPAL, and winit.

<img width="1920" height="1080" alt="image" src="https://github.com/user-attachments/assets/22c30185-dde7-422d-bdd7-3d2c3207451e" />

## status
Glacier is a working DAW with real rough edges. Expect some UI/interaction bugs as active development continues. Core sequencing, playlist arrangement, and playback all work end-to-end; the project is being actively refactored and hardened rather than considered finished. Issues and PRs pointing out bugs are genuinely useful right now. If anyone wants to help me with this project feel free to join and chat: https://discord.gg/YdDbmBzeZT

## getting started
Glacier is a Cargo workspace with two crates: `glacier-app` (the application) and `glacier-dsp` (DSP utilities).

```bash
git clone https://github.com/remysedlak/glacier.git
cd glacier
cargo run --release
```

Requires a working audio output device (via CPAL) and a Vulkan/Metal/DX12-capable GPU (via wgpu).

To open an existing project, click the project icon and pick a `.toml` project file (see `assets/projects/` for an example). A blank default project loads on first run.

## features
- step sequencer with per-pattern sequences, MIDI velocity, and velocity bar view per track
- piano roll — place and edit notes per track per pattern, scrollable note grid and fixed key column
- load .wav tracks at runtime, add/delete tracks dynamically
- variable step counts per track
- multiple patterns, switchable from the UI, with duplicate support
- playlist view — arrange patterns across a timeline with x/y scroll and scissor clipping per region
- mixer window with master volume slider
- per-track volume knobs and mute controls
- draggable, z-ordered mini-windows with correct click and hover ownership across overlapping windows
- track detail windows per track
- right-click context menus on patterns and tracks
- SVG icon pipeline — toolbar icons rasterized via resvg, tooltip system on hover
- custom text rendering via fontdue — glyph cache, textured quads, painter's algorithm interleaving
- multiple font support (variable + monospace)
- play/pause, stop, BPM control, keyboard shortcuts (space, ctrl+s)
- cursor icon feedback on all interactive elements
- project save/load via TOML
- footer status bar showing project path and FPS
## stack
wgpu · winit · CPAL · fontdue · ringbuf · hound · serde/toml · rfd · resvg · dirs · showfile · log/env_logger
