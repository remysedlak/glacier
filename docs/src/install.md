## Installation

Glacier is a Rust workspace with two crates: `glacier-app` (the DAW itself)
and `glacier-dsp` (the signal-processing library it depends on).

### Requirements
- [Rust](https://www.rust-lang.org/tools/install) (stable toolchain)
- A working audio output device (Glacier uses `cpal` for audio I/O)

### Clone
```bash
git clone git@github.com:remysedlak/glacier.git
cd glacier
```

### Run
```bash
cargo run --release -- --dev
```
Run this from the repository root, not from inside `glacier-app/` — asset
paths (fonts, icons, the default project file) are resolved relative to the
working directory you invoke `cargo` from, not the crate's own location.

- `--release` is recommended over a debug build for actual use; the audio
callback and GPU rendering are both real-time-sensitive and a debug build
can introduce audible glitches or dropped frames.

- `-- --dev` loads assets/projects/dev.toml instead of the default new-project file — useful for testing against a project that already has tracks/patterns/audio blocks in place, rather than starting from empty every run.


### Issues

Please open a [Github Issue](https://github.com/remysedlak/glacier/issues) if you have any trouble starting the application or hearing audio.
