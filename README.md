# glacier_daw

A DAW built from scratch in Rust as a learning project. No frameworks — raw wgpu, CPAL, and winit.

## features

- step sequencer with per-pattern sequences and MIDI velocity
- load .wav instruments at runtime
- multiple patterns, switchable from the UI
- playlist view — arrange patterns across a timeline
- mixer window with master volume slider
- per-track volume knobs and mute controls
- draggable windows (sequencer, mixer)
- play/pause, BPM control, keyboard shortcuts
- project save/load via TOML

## modules

- `audio` — CPAL stream, sequencer callback, event-driven trigger resolution
- `graphics` — wgpu setup, draw loop, window manager
- `ui` — shape primitives, layout constants, widget helpers
- `project` — serialization structs, WAV loading
- `app` — winit event loop, input handling, ring buffer dispatch
