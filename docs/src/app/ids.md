# IDs and Project Authority

IDs are stable identities, not positions in a vector. Glacier uses distinct
newtypes—`TrackID`, `PatternID`, and `AudioBlockID`—so Rust can help prevent
mixing unrelated kinds of IDs. References such as `Sequence.track_id` and
`AudioBlockType::Pattern` use those identities. `AudioBlock.track`, by
contrast, is the playlist lane/index field; it is not the block's ID.

## Minting IDs

The app mints IDs when it creates tracks, patterns, and playlist blocks. The
shared `next_id` helper chooses one greater than the maximum existing ID, or
zero for an empty collection. Display names such as `Pattern 2` are labels, not
identities, and may use a collection count.

Audio does not mint project IDs. For a structural edit, the app first updates
its project state, then sends playback data for the changed collection to the
audio engine. This keeps saves and UI decisions tied to one app-owned project
model while audio maintains the runtime copy it needs to play the song.

## Lookup

Resolve a stable ID by matching the object's ID, not by indexing the vector
with the ID:

```rust
tracks.iter().find(|track| track.data.id == track_id)
blocks.iter().position(|block| block.id == block_id)
```

An index from `.position()` or `.enumerate()` is fine for an immediate
operation on that same collection. Fixed UI arrays also use actual positions;
for example, a mini-window constant is an array index, not a project ID.

## Mirroring structural edits

The app is authoritative for project edits. Pattern, track, and block changes
are applied to the app's editable state, which is also the source of save
snapshots. The app prepares an `AudioProjectUpdate` for the changed playback
collections; the audio callback swaps it into its runtime state at a buffer
boundary and returns the retired collection to the app for destruction.

The audio copy exists to render playback. It does not independently create
project objects, choose IDs, serialize project data, or send project mutations
back as confirmation commands. Playback status, meters, and playhead position
do flow from audio to the app, but those are runtime facts rather than project
authority.

When adding a structural edit, update the app's project state and send the
corresponding playback projection. When adding a scalar playback control such
as mute, volume, tempo, or transport, keep the app's persistent value current
and send the audio control that mirrors it. Keep the two operations ordered so
the audio state follows the app's latest value.
