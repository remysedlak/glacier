# glacier
A DAW built from scratch in Rust as a deliberate learning project. No frameworks — raw wgpu, CPAL, and winit.

<img width="1920" height="1080" alt="image" src="https://github.com/user-attachments/assets/22c30185-dde7-422d-bdd7-3d2c3207451e" />

## status
Glacier is a working DAW with real rough edges. Expect some UI/interaction bugs as active development continues. Core sequencing, playlist arrangement, and playback all work end-to-end; the project is being actively refactored and hardened rather than considered finished. Issues and PRs pointing out bugs are genuinely useful right now. If anyone wants to help me with this project feel free to join and chat: https://discord.gg/YdDbmBzeZT

## getting started
Glacier is a Cargo workspace with two crates: `glacier-app` (the application) and `glacier-dsp` (DSP utilities).

**Linux build dependency:** cpal's ALSA backend requires the ALSA development headers to build:
```bash
# Debian/Ubuntu
sudo apt install libasound2-dev

# Fedora
sudo dnf install alsa-lib-devel
```
No additional system packages are required on Windows or macOS.

```bash
git clone https://github.com/remysedlak/glacier.git
cd glacier
cargo run --release
```

**Always use `--release`** — debug builds can be slow enough that per-sample DSP math misses the audio callback deadline, producing audible glitches and `buffer underrun/overrun` errors that don't occur in release. This isn't a bug, it's debug-build overhead.

Requires a working audio output device (via CPAL) and a Vulkan/Metal/DX12-capable GPU (via wgpu).

To open an existing project, click the project icon and pick a `.toml` project file (see `assets/projects/` for an example). A blank default project loads on first run.

## features
Step sequencer, piano roll, multi-pattern playlist arrangement, mixer with per-track volume/mute, and a fully custom-rendered UI (own windowing, text rendering, and icon pipeline on raw wgpu — no GUI framework). Runtime .wav loading, TOML project save/load.

Full feature list and architecture docs: https://glacier.remy.rs

## stack
wgpu · winit · CPAL · fontdue · ringbuf · hound · serde/toml · rfd · resvg · dirs · showfile · log/env_logger
