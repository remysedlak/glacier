# glacier
A DAW built from scratch in Rust as a deliberate learning project. No frameworks — raw wgpu, CPAL, and winit.

<img width="1920" height="1080" alt="image" src="https://github.com/user-attachments/assets/22c30185-dde7-422d-bdd7-3d2c3207451e" />


## features
- step sequencer with per-pattern sequences, MIDI velocity, and velocity bar view per track
- piano roll — place and edit notes per instrument per pattern, scrollable note grid and fixed key column
- load .wav instruments at runtime, add/delete tracks dynamically
- variable step counts per track
- multiple patterns, switchable from the UI, with duplicate support
- playlist view — arrange patterns across a timeline with x/y scroll and scissor clipping per region
- mixer window with master volume slider
- per-track volume knobs and mute controls
- draggable, z-ordered mini-windows with correct click and hover ownership across overlapping windows
- instrument detail windows per track
- right-click context menus on patterns and tracks
- SVG icon pipeline — toolbar icons rasterized via resvg, tooltip system on hover
- custom text rendering via fontdue — glyph cache, textured quads, painter's algorithm interleaving
- multiple font support (variable + monospace)
- play/pause, stop, BPM control, keyboard shortcuts (space, ctrl+s)
- cursor icon feedback on all interactive elements
- project save/load via TOML
- footer status bar showing project path and FPS

## modules
- `audio` — CPAL stream, sequencer callback, event-driven trigger resolution by instrument ID
- `app` — winit event loop, input handling, ring buffer dispatch, file dialog threads
- `project` — serialization structs, WAV loading
- `graphics/mod` — wgpu pipeline, draw loop, painter's algorithm, click owner and hover blocking
- `graphics/font` — fontdue glyph cache, texture upload, NDC quad generation
- `graphics/widgets` — Rectangle, Square, draw_slider, window_title_bar, layout constants
- `graphics/primitives` — ScreenConfig, Vertex, draw_rectangle, draw_knob, padding constants
- `graphics/icons` — SVG rasterization via resvg, icon cache, Tooltip
- `graphics/color` — named color constants
- `graphics/context_menu` — ephemeral right-click menus
- `graphics/components/toolbar` — toolbar draw, icon positions, BPM controls
- `graphics/components/pattern_tray` — pattern list and selection
- `graphics/components/footer` — status bar
- `graphics/mini_window/sequencer` — step sequencer window
- `graphics/mini_window/mixer` — mixer window
- `graphics/mini_window/playlist` — playlist arrangement
- `graphics/mini_window/piano_roll` — piano roll window
- `graphics/mini_window/instrument` — instrument detail window

## stack
wgpu · winit · CPAL · fontdue · ringbuf · hound · serde/toml · rfd · resvg
