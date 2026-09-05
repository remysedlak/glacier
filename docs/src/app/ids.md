# ID Minting & Lookup

This rule applies uniformly across every id-bearing collection in the
project — tracks, patterns, and audio blocks alike. It was discovered
piecemeal (tracks first, then patterns and blocks turned out to have the
identical bug), but the rule itself is one rule, not three.

## The Core Distinction: Identity vs. Position

Every `.id` field (`TrackData.id`, `PatternData.id`, `AudioBlock.id`) is a
**stable identity**, referenced by other structs across the codebase
(`Sequence.track_id`, `AudioBlockType::Pattern(pattern_id)`,
`AudioBlockType::Sample(track_id)`). It is *never* a position index into its
own `Vec` — even though, before anything is ever deleted, an id will
coincidentally equal its position. That coincidence is exactly what makes
this bug easy to introduce and hard to notice: everything works until the
first deletion breaks the assumption.

The test that decides which one you're holding: **does this value's meaning
need to survive being passed to a different thread, or to a later frame?**
If yes, it's an id — resolve it with `.find()`. If it's consumed
synchronously, in the same call that produced it, a plain position is fine.

## Minting New IDs — Never `.len()`

```rust
id: collection.iter().map(|x| x.id).max().map(|m| m + 1).unwrap_or(0)
```

`.len()`-derived ids collide with existing ids the moment anything has ever
been deleted — `len()` shrinks on delete, but surviving ids don't renumber
to fill the gap. Scanning for the current max and adding one is the only
minting strategy that survives deletions. This applies at every creation
site: `LoadTrack`, `CreatePattern`/`DuplicatePattern`, `CreateAudioBlock`
(all in `audio.rs`, since it's the sole minting authority — see below).

Display labels are the one exception — `format!("Pattern {}", patterns.len() + 1)`
is a human-facing count, not an identity, so it has no collision risk and
can stay `.len()`-based.

## Lookup — `.find()`/`.position()`, Never Bracket-Indexed by ID

```rust
collection.iter_mut().find(|item| item.id == x)   // to mutate
collection.iter().position(|item| item.id == x)   // then .remove(pos)
```

`collection[x]` is only correct when `x` is a genuine position, computed and
consumed in the same synchronous call — with no round-trip through a
different thread or a later frame in between. The instant a value comes
from an `.id` field, or has crossed the ring-buffer boundary (UI → audio
thread → confirmation back), it must be resolved by scanning for a matching
id, not indexed directly.

Fixed-slot constants are the legitimate exception —
`mini_windows[SEQUENCER_ID]` is a real constant array index, not an id
masquerading as one. Likewise, values resolved via `.position()`/
`.enumerate()` and used immediately, within the same frame, are fine staying
positional.

## Audio Thread Is the Sole Minting/Mutating Authority

UI-side click handlers never mutate `tracks`/`patterns`/`audio_blocks`
directly for create or delete — they only send an `AudioCommand`. The audio
thread mints the id, mutates its own copy, and reports back with a
confirmation `UiCommand`, which the UI applies to its own mirrored state.
Confirmation commands are named past-tense on purpose — `TrackLoaded`,
`TrackDeleted`, `PatternLoaded`, `PatternDeleted`, `AudioBlockLoaded`,
`AudioBlockDeleted` — to make clear they represent something that *already
happened* on the audio thread's authoritative copy, not a request.

This exists specifically to prevent two independent copies from minting or
mutating the same conceptual thing on their own — the failure mode when
that happens is silent divergence between the UI's view and the audio
thread's real state, which surfaces later as a corrupted save or a UI that
shows something the audio thread disagrees with.

## Delete-Cascade Symmetry

Every delete needs *identical* cleanup applied on both the audio thread's
copy and the UI's mirrored copy. Cleanup logic present on only one side is a
live bug waiting for a save/reload, or any moment the two copies are
compared, to expose it — for example, `DeleteTrack` must remove orphaned
`AudioBlockType::Sample` events referencing the deleted track on *both*
sides; missing it on either side leaves dangling references pointing at a
track that no longer exists.

**When adding any new delete path, check both sides of the boundary** —
the audio thread's handler and whatever applies its confirmation back on
the UI side — for whether cascading cleanup is needed, and whether it's
actually present on both.
