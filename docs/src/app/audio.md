# Audio Engine and Callback

The audio engine owns the runtime state needed to render playback. The app owns
the editable project and the values written to the project file. The engine
receives playback updates from the app through bounded queues; it does not load
or save project files or mint project IDs.

## Runtime playback state

The app's `Track` contains persistent `TrackData`, shared sample buffers, a
waveform buffer, UI state, and metering state. Audio receives an `AudioTrack`
projection containing only the fields needed for playback: track ID, channel
count, mute/volume, root note, shared sample data, voice pool, and meters. The
sample data is held as `Arc<[f32]>`, so app and audio can share immutable sample
storage without copying the audio buffer.

Audio uses `AudioPattern` projections too. They contain a pattern ID and its
sequences, but omit the display name. Playlist blocks are copied as
`AudioBlock`s. These are runtime mirrors of app-owned project data, not a second
place where project edits are decided.

Each track has a fixed-capacity `VoicePool` with 64 slots. Its arrays store
voice positions, playback rates, volumes, stop positions, active flags, and
ages. A fixed-size scratch array records active voice indices once per output
buffer; it does not allocate as voices start or stop. When all 64 slots are in
use, a new voice reuses the oldest slot.

## Commands and project updates

The app sends `AudioCommand`s through a bounded SPSC ring. Small controls such
as tempo, volume, mute, resize, and play state are scalar commands. Structural
edits use `ReplaceProjectParts`: the app prepares playback-only snapshots for
the changed collections, then audio swaps them into the engine at a callback
boundary. The callback returns the replaced collections through a separate
retirement ring so their heap storage is dropped by the app thread.

Preview samples use the same retirement path when replaced. If the retirement
ring has no free slot, audio holds the pending command and continues rendering
without consuming more commands. The callback applies no more than 64 commands
per output buffer. The command ring has 64 slots, the app-side retry queue is
capped at 256 commands, and the retirement ring has two slots. The app
coalesces replaceable pending controls and project updates; if its retry queue
still reaches capacity, it logs and rejects the new command.

This handoff keeps project-vector growth and destruction off the callback, but
it is not a proof that the entire callback is real-time safe. Preparing a large
snapshot can cost time and memory on the UI thread, and applying a track
replacement does work proportional to track count. The callback also performs
mixing, metering, and spectrum analysis.

## Callback and timing

CPAL invokes the output callback to fill each audio buffer. Commands are
processed at the start of that callback, then the engine mixes active voices
and preview audio into the output. There is no project-file I/O in this path.
The callback sends playback position and meter updates to the app through a
bounded ring; these telemetry messages are best-effort and can be dropped if
the UI falls behind. Spectrum frames use their own two-frame ring so they do
not crowd out transport and meter messages.

Sequencer timing uses `samples_per_step(sample_rate, bpm)`, with the negotiated
output sample rate and channel count. The engine supports stereo-or-better
output devices; extra output channels are left silent. Sample positions and
stop points account for mono versus interleaved stereo source data.

## Voice playback

`VoicePool::spawn` finds an inactive slot or steals the oldest active voice.
At the start of a buffer, `refresh_active_indices` fills its fixed scratch
array. Mixing iterates only those active indices. Finished voices mark their
slot inactive rather than being removed from a growable collection.

`frame_advance` and `frame_lookahead` account for source channel count. Mono
samples occupy one value per frame; stereo samples are interleaved and occupy
two. `interpolate` selects the appropriate mono or stereo interpolation path.

## Metering and spectrum

Metering smooths RMS values and reports levels periodically rather than on
every sample. Spectrum analysis buffers a 2048-sample mono mix, applies a
precomputed Hann window, runs the FFT, corrects for window attenuation, and
converts the result to decibels. FFT input and scratch storage are initialized
ahead of callback use, and the result is written into a fixed-size frame.

The FFT still runs on the audio callback, so eliminating command-driven heap
allocation does not eliminate all callback workload or guarantee underrun-free
playback.

## Stream lifetime

The audio stream runs for the active project's lifetime. Loading a project and
its referenced audio files happens on a worker; when loading completes, the app
tears down the old stream and initializes a new engine/stream for the loaded
project. Closing the app pauses and drops the stream from the app's exit path.
There is currently no callback-driven shutdown fade, so audio can stop
abruptly when the stream is dropped.
