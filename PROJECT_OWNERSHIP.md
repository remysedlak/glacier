# Project and audio ownership

This note records the project-loading and saving changes made during the app/audio separation work.

## Ownership

- The app owns the editable, persistent song state: project name and path, tempo, master level, tracks, patterns, and playlist blocks. The current UI `Graphics` value is the app-side store for those fields; graphics widgets render and edit that state.
- The audio callback owns only the runtime state it needs to render playback. Its copy is a playback representation, not the source used for project saves.
- Audio-to-app messages report runtime facts such as playback position, levels, playback state, and sample rate. They do not write project data back into the app.

## Loading and saving

- Opening a project runs TOML reading and referenced track audio loading on a background worker. The audio stream is initialized after that worker returns the project and its loaded tracks.
- Adding a track builds its runtime buffers before sending it to audio; the callback no longer decodes or constructs that track from raw samples.
- Switching projects first saves the current app-side snapshot. A failed save leaves the current project open. A successful save starts background loading; the existing project remains live until the replacement is ready.
- Serialization and file writes run on the long-lived save worker. Save results include the app revision captured by the snapshot, so edits made while a save is in flight do not get marked clean by an older result.
- A regular save is deduplicated by project path and revision while that snapshot is in flight. Ctrl+S skips an already clean project with a normal path; a new/default project still opens Save As so the user can choose its first file.
- Opening another project saves the current project first only when it is dirty. A clean project can switch directly to background loading without a redundant write.
- Closing a dirty project shows the existing Save/Discard/Cancel prompt. Choosing Save writes the app-side snapshot and exits after the result arrives; choosing Discard exits without a project write. Closing a clean project exits without another project write. If an identical regular save is already running when Save is chosen, close waits for that result instead of queueing a second write. If saving fails, the prompt returns and the app stays open.
- Audio has no shutdown command or project-save responsibility; stream teardown runs during winit's exit callback.
- Loaded sample and mono-waveform buffers are immutable shared `Arc` slices. App/audio track copies share those buffers, and the track mini-window reads the prepared mono buffer instead of allocating and averaging all samples on every frame.
- Newly imported tracks are fully constructed on the loader thread, including their mono waveform, before the UI receives them.
- A close request waits for an in-flight track import to finish before taking the save snapshot, so a completed import cannot be omitted from the saved project.

## Review findings and fixes

- **Duplicate project writes:** Ctrl+S key-repeat could enqueue multiple saves for the same unchanged snapshot, clean close always saved again, and close could enqueue another job while an identical Ctrl+S save was in flight. Added regular-save in-flight deduplication, skipped Ctrl+S when clean, made clean close skip project serialization, and let Save-and-close complete from an identical in-flight regular save.
- **Dirty close skipped the existing save-choice prompt:** close now preserves the Save/Discard/Cancel decision and waits for pending imports before showing it.
- **Redundant project-switch writes:** opening a different project always serialized the current one, even when clean. Clean projects now switch straight to background loading; dirty projects still save before switching.
- **Overlapping save and project-switch requests:** opening another project during an identical in-flight Ctrl+S save could queue a second write. The switch now waits on that matching save result and then starts loading; if the app revision changed, it saves the newer snapshot first.
- **Repeated project selection during a pending switch:** choosing another project while the current dirty snapshot was already being saved could enqueue another project-switch save. A later selection now replaces the pending destination and reuses the existing save; Ctrl+S and a completed Save As dialog are ignored while switching.
- **Large audio-buffer copies after background loading:** app/audio track handoff cloned each sample `Vec`, and the track window rebuilt an averaged waveform vector every frame. Sample buffers now use shared immutable `Arc` slices, and drawing reads the precomputed mono samples.
- **Track import still did heavy preparation on the UI thread:** only file decoding was on the worker; `Track::from_data` then built the mono waveform on the UI thread. Track construction now finishes on the worker before sending the track to the app.
- **Closing during track import could lose the imported track:** close previously saved immediately and stopped polling the loader. Close now waits for the loader, installs the result in the app and audio state, then saves the resulting project snapshot.
- **An unfinished track import could land in the wrong project:** switching projects while a track load was pending could apply its result after the new project loaded. A confirmed project switch drops the receiver and clears the pending playlist drop, so the old result is ignored. The detached decode thread itself is not canceled and may continue using CPU/disk until it finishes.
- **Project-switch UI could report stopped while the old audio kept playing:** the app cleared its playing flag as soon as the open dialog returned, before the replacement project was ready. Playback state now changes when the new project is installed, so a failed load leaves the current UI/audio state aligned.
- **Close polling must continue while the app is closing:** the input guard now still lets redraw events run so pending imports and save results can complete; other window input is ignored during close.
- **Pattern and playlist-block creation had two ID/name authorities:** the app created a persistent object while audio independently rebuilt it and allocated another ID. Creation commands now carry the app-created pattern or block, so audio mirrors the exact object without generating IDs or recomputing block lengths.
- **Playback status could lag or remain stale:** the app previously relied on a bounded audio-to-UI ring acknowledgement to update Play/Stop. If that ring was full, the status update could be dropped. Play and Stop now update the UI immediately; the audio acknowledgement assigns the actual state when it arrives.
- **Track mute was toggled twice in the app copy:** the sequencer already changes the UI-owned mute flag before returning the click result. The click handler was toggling it again, so the UI/save copy stayed unmuted while audio became muted. The handler now only marks the project dirty and sends the mirror command to audio.
- **A new sample block could use a zero sample rate:** graphics initially starts with `sample_rate = 0`; depending on callback timing, a sample block could calculate a zero length before the asynchronous rate message arrived. Audio initialization now returns its validated rate directly to the app before input can be handled.

## Command boundary

The app decides what a project edit means and holds the values that are saved. Audio commands apply corresponding changes to the runtime playback representation. Keep project file access, serialization, ID allocation, and save policy out of the audio callback. For newly created objects, send the app-created object with its ID instead of asking audio to construct a second version.

When editing this boundary, keep the intentional runtime copy distinct from a second authority: audio must not generate project IDs, decide project mutations, or report project state as canonical.

## Remaining real-time risk

The project file decode and save work is off the audio callback, but command application can still allocate or free memory on that callback. Examples include growing the pattern, playlist-block, and track vectors; extending a sequence for a step; cloning renamed strings; and replacing preview sample buffers. The Hann window is now precomputed once, but the spectrum path still allocates each frame: `SpectrumAnalyzer::process` allocates its complex FFT buffer and magnitude output, and moving `db` into the UI message leaves it empty so the next frame allocates again. If the UI ring is full, the failed spectrum message is dropped in the callback and frees its buffer there. Deleting/replacing values can also free memory on the callback. The command drain processes every queued command in one callback, so a backlog can increase callback work. This architecture is **not allocation-free or fully real-time safe**.

### Provisional direction for callback safety

Treat callback safety as a separate follow-up, and keep the current project ownership/save work from being mistaken for that fix. The next concrete step should be moving spectrum analysis out of the callback: feed a fixed-capacity SPSC sample/frame queue to a persistent analyzer worker, and define that analysis frames may be dropped if its queue is full. The Hann window itself is already precomputed. Then inventory and measure remaining callback allocations and worst-case command work. Divide communication by purpose:

- Keep small, bounded, fixed-size controls (play/stop, tempo, volume, note toggles) on the existing SPSC audio command ring. Bound how many commands are applied per callback and define overflow/coalescing behavior for replaceable controls.
- Prepare structural project changes off the callback. Send prepared state through a bounded handoff; at an audio block boundary, switch to it and return the retired state through a second bounded queue so its vectors/strings are destroyed off the callback. Do not send `Track`, `PatternData`, or `String` payloads that the callback must grow, clone, or destroy in place.
- Reuse preallocated spectrum/DSP buffers and use a bounded buffer-return scheme for frames sent to the UI. Avoid solving this with locks or blocking sends in the callback.

This is a design direction, not implemented behavior. Fixed capacities alone are insufficient if deletion or replacement still frees memory on the callback, and the current unbounded `while try_pop()` command drain also needs a deliberate budget.

## Worker and channel mechanics

- Project loading and track import each use a detached `std::thread` and a `std::sync::mpsc` result channel polled by the app. Dropping the receiver prevents a stale result from being applied, but does not interrupt file decoding or stop the worker thread. There is no cancellation token or generation counter.
- Project saving uses one long-lived `std::thread`, with `std::sync::mpsc` job and result channels. `ProjectSaveWorker::drop` queues shutdown and joins the worker; queued saves are processed before shutdown because the channel preserves message order.
- Audio commands and audio-to-app telemetry still use the existing `ringbuf` SPSC queues. `AudioCommandSender` adds an app-side `VecDeque` to retain UI commands if the audio ring is full. This does not replace the audio-to-app ring; telemetry/status pushes there remain best-effort and may be dropped.

## Feature-by-feature review notes

- **App-owned project edits:** pattern, block, track, tempo, mute, and mixer edits are reflected in the app snapshot before saving. The mute path had been toggling the app copy twice; that is fixed. Playback UI now reflects Play/Stop immediately so a dropped status message cannot leave it stale.
- **Background project loading:** project metadata and referenced WAV reads happen on a detached worker. Open failures keep the currently active project, but the failure is only logged; there is no visible error dialog. Startup treats any unreadable/unparseable file as a blank default project, which can be risky if the user later saves over the supplied path.
- **Audio stream replacement:** the old stream is dropped before the replacement starts, and closing drops stream/graphics resources from winit's exit callback. CPAL device/configuration/build/start still happens synchronously on the UI thread and uses `expect`, so a slow or unavailable audio device can stall or panic the app during project install.
- **Close/discard during a project-switch save:** a project-switch write already queued on the save worker cannot currently be canceled. If the user closes during that save and chooses Discard, the write may still finish and persist the current project's edits. The app still closes safely, but Discard is not guaranteed to undo a write already in progress.
- **Audio shutdown behavior:** the old callback-driven fade-out was removed with the audio-owned shutdown/save protocol. Closing now pauses and drops the stream directly, so the final audio can be cut off abruptly.
- **Background saving:** snapshots are serialized and written by one worker; regular saves are deduplicated for a matching path/revision, and close/switch wait for save results. Snapshot construction still clones project metadata/sequences on the UI thread, which may become noticeable for very large projects. Save failures are logged; close restores its prompt, while regular-save and project-switch errors have no visible UI notification.
- **Track import and shared samples:** imported audio and its mono waveform are prepared on a worker, then sample buffers are shared through `Arc`. The loader still treats decode/open failure as an empty track, and current decoding is WAV-only even though some file pickers/extensions advertise other formats.
- **Audio/UI command queues:** UI-to-audio commands are retained and retried in order if the ring is full. Audio-to-UI telemetry and status messages still use best-effort pushes and can be dropped when their ring is full; Play/Stop state no longer depends on those acknowledgements.
- **Save file replacement:** non-Windows writes use a same-directory temporary file followed by rename. Windows still writes directly to the destination, so interruption during a write can leave a truncated project file.
