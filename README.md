# glacier

A DAW built from scratch in Rust as a deliberate learning project. No frameworks — raw wgpu, CPAL, and winit.

## features

- step sequencer with per-pattern sequences and MIDI velocity
- load .wav instruments at runtime
- multiple patterns, switchable from the UI
- playlist view — arrange patterns across a timeline
- mixer window with master volume slider
- per-track volume knobs and mute controls
- draggable, z-ordered mini-windows (sequencer, playlist, mixer)
- custom text rendering via fontdue — glyph cache, textured quads, painter's algorithm interleaving
- play/pause, BPM control, keyboard shortcuts
- project save/load via TOML

## modules

- `audio` — CPAL stream, sequencer callback, event-driven trigger resolution
- `graphics` — wgpu pipeline, fontdue glyph cache, per-window draw ranges, painter's algorithm
- `graphics/font` — glyph rasterization, texture upload, NDC quad generation
- `graphics/sequencer`, `mixer`, `playlist` — per-window geometry and text
- `ui` — shape primitives, layout constants, widget helpers
- `project` — serialization structs, WAV loading
- `app` — winit event loop, input handling, ring buffer dispatch

## stack

wgpu · winit · CPAL · fontdue · ringbuf · hound · serde/toml · rfd
