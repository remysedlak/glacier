# CLAUDE.md — Glacier

Context file for Claude when working in this repo. Keep this up to date as the project evolves.

## What this is

Glacier is a DAW (Digital Audio Workstation) built from scratch in Rust, as a deliberate
learning project — no game/audio frameworks, no GUI toolkit. Raw `wgpu` for rendering,
`winit` for windowing/events, `cpal` for audio I/O. The point is to learn how these systems
work at a low level, not to ship the fastest MVP. When suggesting changes, prefer
explaining *why* something works over just patching it — that's the goal of the project.

Status: core sequencing, playlist arrangement, and playback work end-to-end. Actively being
refactored/hardened, not "finished." Expect rough UI edges.

## Workspace layout

Two-crate Cargo workspace:

- **`glacier-app`** — the application: windowing, rendering, UI, project files, audio engine
  wiring. This is where almost all the code lives today.
- **`glacier-dsp`** — DSP utility library. Currently has analysis-side primitives (RMS, peak,
  ZCR, envelope follower, DFT/Hann window/STFT scaffolding) and a couple of pitch/timing
  helpers. **This is the crate to grow when adding real DSP** (filters, effects, etc.) — keep
  DSP math here, separate from `glacier-app`'s UI/audio-engine glue.

## Threading model (important — read before touching audio or app.rs)

Two threads, decoupled via **lock-free ring buffers** (`ringbuf` crate / `HeapProd`/`HeapCons`):

- **Audio thread** (`audio.rs`, driven by `cpal`) — owns playback, the sequencer callback, and
  event-driven trigger resolution by track ID. Receives `AudioCommand`s from the UI thread via
  a consumer ring buffer, and reports state back via a producer ring buffer of `UiCommand`s.
- **UI/app thread** (`app.rs`, driven by `winit`) — owns the `Graphics` state, all rendering,
  input handling, and file dialogs (which get their own spawned threads communicating back via
  `std::sync::mpsc::Receiver`). Sends `AudioCommand`s, drains `UiCommand`s each frame in `draw`.

Do not call into the audio engine synchronously from the UI thread or vice versa — everything
crosses the boundary as an enum command over the ring buffer. See `AudioCommand` (in
`audio.rs`) and `UiCommand` (in `app.rs`) for the full message vocabulary.

## Rendering model

Immediate-mode-ish, hand-rolled:

- Everything becomes `Vertex` data (`graphics/primitives.rs`) pushed into a shared vertex
  buffer each frame; `shader.wgsl` draws it.
- **Painter's algorithm**: draw order = z-order. `MiniWindow`s track their own z-order
  (`Graphics.z_order`), and `RecordedRegion`/`WindowDrawRange` (`graphics/regions.rs`) track
  where each window's vertices live in the buffer so scissor-rects and click/hover ownership
  can be resolved per-window even when windows overlap.
- Each draw function for a widget/window returns some combination of `TextItem`s, `IconDraw`s,
  a `ClickResult`, a `CursorIcon`, and sometimes a `Tooltip` — the caller (ultimately `app.rs`)
  is responsible for actually acting on the `ClickResult`.
- Text: `fontdue` glyph rasterization, cached per (font, char, size) in `GlyphCache`
  (`graphics/font.rs`), uploaded as textured quads.
- Icons: SVGs rasterized via `resvg`, cached in `Graphics.icon_cache`.

## Data model quick reference

- **`Project`** (`project.rs`) — top-level serializable song: name, bpm, master volume, tracks,
  patterns, events. Saved/loaded as `.toml` via `serde`.
- **`Track`** (runtime) wraps **`TrackData`** (serializable metadata: path, mute, volume,
  root note) plus loaded `samples: Vec<f32>` and live playback/meter state (rms, peak_hold,
  position, playback_rate).
- **`PatternData`** → `Vec<Sequence>` → each `Sequence` is one track's row of `Note` steps
  (`velocity`, `pitch`) for a pattern. This is the step-sequencer/piano-roll data.
- **`AudioBlock`** — places a `Sample`, `Pattern`, or `Mixing` block on the playlist timeline
  (`track`, `start_step`, `length`).
- **`MiniWindow`** — generic draggable/floating window shell; `WindowKind` (Sequencer,
  Playlist, Mixer, PianoRoll, TrackDetail(usize)) picks which `mini_window/*.rs` draw fn runs.

## glacier-dsp — current state and where DSP work should go

Right now `glacier-dsp` is **analysis-only**: RMS/peak/ZCR windowing for meters, an envelope
follower, a basic DFT + Hann window, and STFT is a stub (`pub fn stft()`, unimplemented).
There is **no filtering/EQ/effects code yet** — biquads, EQ bands, etc. do not exist in this
codebase yet and would be new additions to this crate.

When adding a new DSP primitive here:
- Keep it a pure function or small struct with explicit state (mirrors the existing style —
  e.g. `rms_window`/`rms` take slices and window/hop sizes, no hidden global state).
- If it's stateful across calls (like a biquad needs `x1,x2,y1,y2` history), make that an
  explicit struct instance the audio thread owns and steps once per sample/block — do not
  reach for global/static mutable state.
- Add a unit test in the same style as the existing `zero_rms_window`/`sine_rms_window`/etc.
  tests — they use small synthetic signals (a generated sine, a zero buffer) as fixtures.
- Getting this hooked into the live audio path means wiring it into `audio.rs`'s CPAL
  callback / the sequencer's mixing step, not into `glacier-app/graphics`.

## Learning goals for this project (context for how to help)

This project doubles as a DSP-learning exercise. When asked to add DSP features, the person
generally wants to *understand* the technique being added, not just get a working black box —
prefer explaining the math/structure (transfer functions, coefficients, windowing, etc.)
alongside the implementation, and default toward idiomatic-but-transparent Rust over dense
one-liners. Recent focus has been on biquad filters (EQ bands, Butterworth/peaking/shelf
filter math) and FFT/spectrum-analysis fundamentals — natural next DSP additions to
`glacier-dsp` include a biquad filter struct/module and finishing out the `stft` function for
a spectrum analyzer view.

## Stack

`wgpu` · `winit` · `cpal` · `fontdue` · `ringbuf` · `hound` · `serde`/`toml` · `rfd` · `resvg`
· `dirs` · `showfile` · `log`/`env_logger`

## Conventions worth preserving

- `pub` fields are used freely on state/data structs (not much encapsulation-via-privacy) —
  match this style rather than introducing getters/setters unless there's a real invariant to
  protect.
- Enums (`AudioCommand`, `UiCommand`, `ClickResult`, `DragResult`) are the message-passing
  vocabulary between subsystems — adding a new cross-thread or cross-module interaction
  usually means adding a variant here, not a new ad-hoc channel.
- Doc comments (`///` / `//!`-style single-line comments above structs/fns) are used
  consistently and are how `PROJECT_STRUCTS.md`/`PROJECT_STRUCTURE.md` got generated
  (via `project_scan.py`) — keep writing them.
