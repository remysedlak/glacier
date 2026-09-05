# Welcome to Glacier Docs

Glacier is a digital audio workstation (DAW) built from scratch in Rust —
audio engine, GPU-rendered UI, and sequencer, all hand-written with minimal
dependencies. It's an active, ongoing learning project, so expect the
architecture to keep evolving.

This book exists because the project outgrew a single README a while ago —
it's where the *why* behind the code lives, not just the *what* (that part's
covered by generated API docs via `cargo doc`).

## Getting Started

Documentation is split to mirror the workspace's own structure:

- **[glacier-app](./app/index.md)** — the application itself: graphics,
  audio thread, UI state, and the sequencer/playlist/piano roll windows.
- **[glacier-dsp](./dsp/index.md)** — the standalone signal-processing
  crate: RMS/peak/ZCR analysis, FFT, envelope following.

## Download

View the  **[installation guide](./install.md)** or the **[repository](https://github.com/remysedlak/glacier)** to get started!
