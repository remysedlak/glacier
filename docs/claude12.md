# CLAUDE.md — glacier

## Project Goal
Building a DAW from scratch in Rust. Learning properly — no LLM-generated code.
Claude provides intuition, concepts, and small snippets only. No fixing pages of code.

## Learning Philosophy
- Figure things out before asking
- Read compiler errors carefully before asking
- Read the Rust Book when stuck on syntax
- Ask Claude for direction, not solutions
- Don't generalize a pattern (builder, trait, abstraction) until a real call site needs it to combine with something else — but once that real need shows up, build for it directly instead of continuing to wait

## Environment
- OS: Linux
- Editor: Zed
- Git: SSH only (always clone with git@github.com:)
- Project: ~/Documents/projects/glacier

## File Structure
- `main.rs` — event loop setup, audio init, ring buffer creation and split, stream passed to App
- `app.rs` — App struct, input handling, holds HeapProd + HeapCons + Stream + pending_project + ctrl_pressed + prev_mouse_x + prev_mouse_y + file dialog receivers
- `graphics/mod.rs` — Graphics struct, wgpu setup, draw loop, painter's algorithm, per-window draw ranges, fontdue text rendering, click_owner + blocked hover masking
- `graphics/font.rs` — glyph cache (fontdue), texture upload, NDC quad generation, build_glyph_cache, draw_glyph
- `graphics/geometry.rs` — Rectangle (pure geometry only), Rectangle::square() constructor, RectangleCtx builder, BorderStyle, DrawResponse. Square struct removed (see Rectangle Drawing section).
- `graphics/widgets.rs` — draw_slider, window_title_bar, window_background, layout constants
- `graphics/primitives.rs` — ScreenConfig, Vertex, draw_rectangle, draw_knob, draw_h_line, padding constants
- `graphics/components/toolbar.rs` — draw_toolbar, icon positions, tooltip construction
- `graphics/components/pattern_tray.rs` — pattern tray UI, SelectPattern(usize) ClickResult variant
- `graphics/components/footer.rs` — free function draw(screen_config, path, fps)
- `graphics/components/slider.rs` — slider::draw, slider_y_origin (shared by mixer.rs draw AND drag.rs hit-test — see Mixer section)
- `graphics/components/side_panel/track_tray.rs` — file tree browser, recursive draw_fs_tree, FsToggleDir click result
- `graphics/mini_window/mod.rs` — MiniWindow, WindowKind, WindowDrawRange, PlaylistDrawRanges, PianoRollDrawRanges, window ID constants
- `graphics/mini_window/sequencer.rs` — sequencer window geometry and text
- `graphics/mini_window/mixer.rs` — mixer window geometry and text
- `graphics/mini_window/playlist.rs` — playlist window geometry and text
- `graphics/mini_window/piano_roll/` — piano roll window, scrollable key column + note grid
- `graphics/mini_window/track.rs` — track detail mini-window
- `graphics/context_menu.rs` — ContextMenu, ContextMenuKind, ephemeral right-click menu
- `graphics/icons.rs` — IconSvg, IconDraw, Tooltip, icon cache build, rasterize_icon, draw_icon
- `graphics/color.rs` — named color constants as Color structs
- `graphics/drag.rs` — DragResult, Graphics::handle_drag — sticky drag state for knob, slider, window, tray resize, event resize (see Hard Lessons)
- `app/click.rs` — ClickResult enum + handle_click_result + **InteractionResult struct (click + cursor bundled, see Interaction State section) + its `or()` merge method**
- `app/ui_command.rs` — UiCommand enum + handle_ui_command
- `audio.rs` — init(), AudioCommand handling, sequencer callback, event-based trigger resolution
- `project.rs` — Track, TrackData, PatternData, Sequence, Note, AudioBlock, AudioBlockType, get_project, get_tracks, save_project, path_to_vector

---

## Voice Model — Polyphony

### Background
Original `Track` had exactly one `position`/`is_playing`/`playback_rate`/`current_volume`
slot — meaning only one note per track could sound at a time. Triggering a second note
before the first finished just overwrote the slot, killing the first note. This became
a real problem from two directions at once: (1) `AudioBlock.length`-based playback cutoff
needed a per-trigger stop point, not a track-wide one, and (2) piano roll chords need
multiple simultaneous notes per track. Two independent real needs converging on the same
missing thing is what justified building this now instead of continuing to defer it.

### The fix
```rust
pub struct Voice {
    pub position: f32,
    pub is_playing: bool,
    pub playback_rate: f32,
    pub current_volume: f32,
    pub target_volume: f32,
    pub stop_at_frame: Option<f32>,  // wired in and enforced — see below
}

pub struct Track {
    pub data: TrackData,
    pub samples: Vec<f32>,
    pub voices: Vec<Voice>,   // replaces the single position/is_playing/etc. fields
    pub show_velocity: bool,
    pub rms_l: f32,
    pub rms_r: f32,
    pub peak_hold: f32,
}
```
`target_volume` moved from `TrackData` to `Voice` — it's a per-trigger playback parameter,
not a persistent track setting (persistent per-track volume is `TrackData.track_volume`,
unrelated, still on `TrackData`).

### Triggering — always push, no voice stealing yet
Both trigger sites (`AudioBlockType::Sample` trigger loop, pattern-note trigger loop in
audio.rs) `track.voices.push(Voice { ... })` on every trigger — no search for a free/
reusable voice, no cap on simultaneous voices. This is a deliberate simplification:
correct, but voices from finished notes only get removed via the cleanup below, and there's
no limit on how many can pile up if triggers fire faster than they finish. Real "voice
stealing" (reusing the oldest/quietest voice once at a cap) is a known future improvement,
not yet needed.

### Cleanup — retain in the mixing loop, not a separate pass
```rust
for track in &mut tracks {
    if !track.data.is_muted {
        for voice in &mut track.voices {
            if voice.is_playing { /* mix, same math as before, per-voice */ }
        }
        track.voices.retain(|v| v.is_playing);
    }
}
```
`retain` runs once per track per audio callback — O(n) in voice count (small), no
reallocation in steady state (only shrinks in-place). This is NOT garbage collection;
it's a single filter pass, cheap enough for the real-time audio callback.

### stop_at_frame — now wired in, both units matter
At trigger time (`AudioBlockType::Sample` arm), `stop_at_frame` is computed as
`audio_block.length as f32 * samples_per_step * 2.0`. The `* 2.0` matters:
`glacier_dsp::samples_per_step` returns plain frames-per-step, but `voice.position`
and `track.samples.len()` both operate in raw interleaved-array-index space (confirmed
by `voice.position += 2.0 * voice.playback_rate` and `pos + 3 >= track.samples.len()`
already stepping/indexing by 2 per stereo frame) — so frames had to be doubled to
match. In the mixing loop, the cutoff check became:
```rust
let stop = voice.stop_at_frame.unwrap_or(track.samples.len() as f32);
if pos as f32 + 3.0 >= stop.min(track.samples.len() as f32) {
    voice.is_playing = false;
}
```
The `.min()` guards a block resized longer than the actual sample from reading past
`track.samples`'s real length. Pattern-triggered notes still pass `stop_at_frame: None`
(full sample plays) — not yet revisited, see Known Open Issues.

### rms_l/rms_r/peak_hold stay on Track, not Voice
These are a summed/aggregate signal across all of a track's currently-playing voices,
not per-voice state — they stay put, unlike position/is_playing/etc.

---

## Interaction State — InteractionResult

### Background
Every window/component draw function (`sequencer::draw`, `mixer::draw`, `playlist::draw`,
`toolbar::draw`, `footer::draw`, etc.) needs to report back two things after hit-testing
its own geometry against the mouse: what got clicked (`ClickResult`), and what cursor
should show (`CursorIcon`). These used to be two loose fields in each function's return
tuple, in inconsistent positions/order across different files (sometimes `(texts, icons,
result, cursor)`, sometimes `(texts, result, cursor, icon, tooltip)`, sometimes cursor
at the very end). Nothing type-checked the *pairing* of these two fields, which let a
real bug ship silently: `mixer::draw`'s cursor field was destructured as `_cursor` at
its call site in `Graphics::draw()` and discarded — no compiler warning, no visible
symptom until a hover-cursor case actually needed it.

### The fix
```rust
pub struct InteractionResult {
    pub click: ClickResult,
    pub cursor: CursorIcon,
}

impl InteractionResult {
    pub fn or(self, other: InteractionResult) -> InteractionResult {
        InteractionResult {
            click: if self.click != 
           
            { self.click } else { other.click },
            cursor: if self.cursor != CursorIcon::Default { self.cursor } else { other.cursor },
        }
    }
}
```
Defined in `app/click.rs`, next to `ClickResult` itself.

**Every window/component draw function now returns `InteractionResult` directly**
instead of loose trailing `result`/`cursor` fields. `Graphics::draw()`'s return type
changed from `(ClickResult, CursorIcon)` to `InteractionResult` to match.

### Why `or()` resolves click and cursor independently
The first draft of `or()` picked a winner by checking `click` alone and returning
`self` or `other` wholesale — this silently broke plain hover (mouse over a control,
nothing clicked): a component with `cursor: SomeIcon` but `click: None` would lose its
cursor contribution the moment merged against another `click: None` result, because the
merge fell through to `other` entirely. Each field must resolve on its own terms:
`click` keeps whichever side has a real click; `cursor` keeps whichever side has a
non-default cursor. These are independent questions, not one bundled fate.

### Identity / "nothing happened" value
```rust
InteractionResult { click: ClickResult::None, cursor: CursorIcon::Default }
```
This is the neutral element for `or()` — merging anything with it returns the other
side unchanged, in either argument position. It's what `Graphics::draw()`'s
accumulator starts as, and what any window function should return when its own
hit-tests found nothing.

### Naming convention at call sites
Inside each window's own `draw()` body, the value being built and returned is named
`interaction` (matches the accumulator name in `Graphics::draw()`, for readability
across files). At each call site *inside* `Graphics::draw()`, the per-component
result is bound to `<component>_interaction` (`sequencer_interaction`,
`mixer_interaction`, `playlist_interaction`, `file_tree_interaction`,
`pattern_interaction`, etc.) — named for the actual source component, not its parent
module (e.g. the pattern tray's result is `pattern_interaction`, not
`side_panel_interaction`, even though it's called via `side_panel::pattern_tray::draw`).
The running total across the whole function stays `interaction`, reassigned via
`interaction = interaction.or(component_interaction);` at each site — never passed
into component functions as `&mut`, since each component computes its own result
fresh from its own geometry every frame; only the top-level reduction is stateful.

### Manual cursor overrides bypass the merge
Drag/resize state overrides (`resizing_track_tray` → `ColResize`, active
window/knob drag → `Default`) write directly to `interaction.cursor` near the end of
`Graphics::draw()`, rather than going through `or()`. These aren't a component
reporting a competing hit-test result — they're an unconditional override that should
win regardless of what any window claimed, so they don't belong in the reduction chain.

### Known limitation, not yet an issue
`or()`'s "first non-empty wins" priority is really "whichever call site runs earlier
in `Graphics::draw()`'s source order wins" — there's no explicit z-order-driven
priority for the click merge across *all* eleven call sites (only mini-windows use
real z-order via `click_owner`/`blocked` masking; toolbar/footer/tray sit outside that
masking and merge in whatever order they're coded). This hasn't caused an observed bug
— toolbar/footer/tray don't currently overlap mini-window bounds — but if that ever
changes, this is the place to revisit (see Hard Lessons).

---

## Rectangle Drawing

### Background
Originally had `Rectangle` and `Square` as separate structs with fully duplicated
`is_hovered`/`draw`/`draw_bordered`/`draw_interactive` methods — `Square` was just
`Rectangle` with `width == height`. Deleted `Square` entirely; `Rectangle::square(x, y, size)`
is the constructor now. Zero call sites needed `Square` as a distinct type (no `Vec<Square>`,
no pattern matching on it) — it was pure convenience, not an invariant worth a type.

Also had a `draw_interactive` method that hard-coded "hover swaps to `.hovered()` color" —
worked fine as long as no shape needed both a border AND hover behavior at once. Once
toolbar buttons needed both simultaneously, doing this as separate methods
(`draw`, `draw_bordered`, `draw_interactive`, `draw_interactive_bordered`, ...) would have
meant one method per *combination* of optional behaviors — combinatorial explosion.
Replaced with a builder.

### Three ways to draw a Rectangle now
- `rect.draw(screen_config, color, radius, out)` — plain, no border, no hover check.
  Use for static backgrounds, dividers, anything nobody ever needs a hover/border state for.
- `rect.draw_bordered(...)` — border, no hover. Rarely called directly; mostly used
  internally by `RectangleCtx::draw` when a border was configured.
- `rect.draw_style().interactive(Some(mouse_state)).bordered(Some(style)).draw(...)` —
  the builder. Use when a shape needs interactivity, a border, or both. Either chain
  method can be omitted/passed `None` if that axis isn't needed; order doesn't matter,
  `.draw(...)` is always the terminal call.

### Builder mechanics (RectangleCtx<'a>)
```rust
pub struct RectangleCtx<'a> {
    rectangle: &'a Rectangle,             // borrowed, never copied/owned
    interactive: Option<&'a MouseState>,  // None = skip hover check entirely
    border: Option<BorderStyle>,          // None = call draw() not draw_bordered()
}
```
- `Rectangle::draw_style(&self) -> RectangleCtx<'_>` is the only entry point — borrows `self`.
- `.interactive(...)` / `.bordered(...)` take `self` **by value**, mutate one field,
  return `self` — this is what makes chaining work. Each one must return the exact
  same type (`RectangleCtx<'a>`) or the chain can't continue.
- `.draw(...)` is the one method that isn't `Self -> Self` — it consumes the ctx,
  does the actual hover check + color swap + calls `draw`/`draw_bordered` on the
  borrowed rectangle, and returns `DrawResponse`.
- `DrawResponse { hovered, x, y, width, height }` — geometry fields included because
  most interactive rects (toolbar buttons) get read again afterward for tooltip/icon
  positioning. This does mean every draw call copies 4 extra floats even when unused —
  accepted as a fine tradeoff for this file's usage pattern (geometry is read back
  almost every time here). Don't assume this tradeoff holds elsewhere without checking.

### When NOT to use the builder
Static shapes with no hover/border need (backgrounds, dividers, `toolbar_divider`,
`step_divider_line`) stay on plain `.draw(...)`. Routing them through `.draw_style()`
for "consistency" adds a `DrawResponse` nobody reads and geometry fields nobody needs —
pure cost, no benefit. Match the tool to whether the axes are actually in play.

### Gotcha: building + chaining in one expression destroys the binding
```rust
// WRONG — bpm_up ends up bound to the DrawResponse, not the Rectangle;
// bpm_up.x later fails to compile, DrawResponse has no relation to Rectangle unless
// you deliberately added geometry fields to it (see above)
let bpm_up = Rectangle::new(x, y, w, h).draw_style().interactive(...).draw(...);

// RIGHT if you need the Rectangle itself again later for layout math —
// bind it first, separately, before chaining off a reference to it
let rect = Rectangle::new(x, y, w, h);
let response = rect.draw_style().interactive(...).draw(...);
```
Whether you need the separate binding depends on whether anything downstream reads
`.x`/`.y`/`.width`/`.height` off the *original rectangle* — many toolbar buttons compute
a sibling's position from it (`bpm_down`'s y = `bpm_up.y + 18.0`) or feed it into an
`IconDraw`'s position, so the binding is usually needed regardless of the builder.

---

## Window System

### IDs and push order
```rust
pub const SEQUENCER_ID: usize  = 0;
pub const PLAYLIST_ID: usize   = 1;
pub const MIXER_ID: usize      = 2;
pub const PIANO_ROLL_ID: usize = 3;
```
CRITICAL: mini_windows.push() order in create_graphics must exactly match these constants. The IDs are direct indices into mini_windows. Wrong push order causes wrong window content on wrong geometry — silent, visually obvious.

### Dynamic Track windows
- Track detail windows pushed at runtime: WindowKind::TrackDetail(track)
- ID is mini_windows.len() - 1 after push
- z_order.push(new_id) adds them to draw order
- The track => wildcard arm in the draw match handles all IDs >= 4

### Sequencer window height
The sequencer grows with each track:
```rust
window.height + TRACK_GAP * tracks.len() as f32  // in sequencer::draw
```
MiniWindow.height must be kept in sync or is_hovered returns false for clicks below the initial height.
Recalculate `gfx.mini_windows[SEQUENCER_ID].height = 100.0 + TRACK_GAP * gfx.tracks.len() as f32;`
in **every** UiCommand handler that changes `gfx.tracks`, not just incremental ones:
- `TrackLoaded` ✅
- `TrackDeleted` ✅
- `LoadProject` (bulk load from a save file) — **easy to miss**, since it bulk-assigns
  `gfx.tracks = tracks;` directly and doesn't go through the per-track add/remove path.
  Missing this leaves the window stuck at its `create_graphics`-time default height while
  the drawn track rows (computed independently, per-track, in a loop) extend past it —
  visually the grey background looks "too short" for the number of tracks loaded.

---

## Click and Hover Ownership

### click_owner
```rust
let click_owner: Option<usize> = if mouse_state.left_clicked {
    self.z_order
        .iter()
        .rev()
        .find(|&&id| self.mini_windows[id].is_open && self.mini_windows[id].is_hovered(mouse_state.x, mouse_state.y))
        .copied()
} else {
    None
};
```
Computed once per frame after the bring-to-front pass. The topmost hovered open window owns the click.

### blocked (hover masking)
```rust
let blocked = self.z_order
    .iter()
    .skip_while(|&&z_id| z_id != id)
    .skip(1)
    .any(|&above_id| {
        self.mini_windows[above_id].is_open
            && self.mini_windows[above_id].is_hovered(mouse_state.x, mouse_state.y)
    });
```
z_order is back-to-front — iterating forward from id gives windows drawn on top of it. If any open window above covers the mouse, this window is blocked.

### masked_mouse
```rust
let masked_mouse = MouseState {
    left_clicked: mouse_state.left_clicked && click_owner == Some(id),
    x: if !blocked { mouse_state.x } else { f32::NEG_INFINITY },
    y: if !blocked { mouse_state.y } else { f32::NEG_INFINITY },
    ..*mouse_state
};
```
Every window draw call receives &masked_mouse. Toolbar, context menu, footer use original mouse_state. MouseState must derive Copy for struct update syntax.

Note: only the mini-window loop currently computes `click_owner`/`blocked` masking.
Toolbar, footer, and both trays merge their `InteractionResult` in plain source order
against the running `interaction` accumulator — see "Known limitation" under
Interaction State above.

---

## Text Rendering — fontdue

### Why fontdue (replaced glyphon)
glyphon prepare() must run before the render pass opens, render() inside it — all text draws after all geometry unconditionally. Made painter's algorithm impossible for overlapping windows.

### fontdue approach
- Rasterizes glyphs to CPU bitmaps, uploaded as wgpu::Texture (R8Unorm, single channel alpha)
- Each glyph is a textured quad — text IS geometry, interleaved with colored rects
- One wgpu::BindGroup per cached character, swapped per draw call inside render pass
- Multiple font files: font_cache and glyph_cache are HashMap<String, ...> keyed by font name

### Glyph cache
```rust
HashMap<String, HashMap<(char, u32), (wgpu::Texture, wgpu::BindGroup, fontdue::Metrics)>>
```
- Outer key: font name string
- Inner key: (char, size as u32)
- build_glyph_cache takes sizes: &[f32], loops over all sizes x all ASCII glyphs
- Any size used in a TextItem must be in the startup slice — mismatches silently miss
- Zero-size glyphs skipped — zero-dimension texture panics wgpu
- FilterMode::Nearest — Linear blurs pixel-exact glyphs

### Shader branch
```wgsl
if in.uv.x < 0.0       { use color }              // geometry quad
else if in.uv.x > 1.0  { sample icon rgba }        // icon quad
else                   { sample glyph alpha * color } // glyph quad
```

---

## Painter's Algorithm

### WindowDrawRange
```rust
struct WindowDrawRange {
    vert_start: u32,
    vert_end: u32,
    char_start: usize,
    char_end: usize,
}
```

### Playlist and piano roll special ranges
These have scrollable sub-regions requiring separate scissor rects. They populate playlist_window_ranges and piano_roll_ranges instead of the plain window_ranges path. The render pass continues past the generic path for these two. The window_ranges entry is still pushed but unused — continue skips it.

### Z-order
- z_order: Vec<usize> — back = topmost
- bring_to_front(z_order, id) — retains all, pushes to end
- Toolbar always drawn last — always on top

---

## ID Minting & Lookup — applies to tracks, patterns, AND audio blocks

This whole section generalizes what used to be "Track ID System" — a full-day
debugging session showed the same rule applies identically to `PatternData.id`
and `AudioBlock.id`, not just `TrackData.id`. The old section only documented it
for tracks; patterns and events had the identical bug, just undiscovered until
ids stopped being contiguous (after any delete).

### The problem
Any `.id` field is a stable identity used by other structs to reference it
(`Sequence.track_id`, `AudioBlockType::Pattern(pattern_id)`,
`AudioBlockType::Sample(track_id)`). It is **never** a position index into its
own `Vec`, even though it may coincidentally equal one before anything is ever
deleted — that coincidence is what let three separate bugs (tracks, patterns,
events) hide for a long time.

### Minting new ids — never `.len()`
`.len()`-derived ids collide with existing ids the moment anything has ever been
deleted (`len()` shrinks; surviving ids don't renumber). Always scan for the
current max and add one, at every creation site, on whichever side mints:
```rust
id: collection.iter().map(|x| x.id).max().map(|m| m + 1).unwrap_or(0)
```
Applies to: `LoadTrack` (audio.rs), `AddPattern`/`DuplicatePattern` (audio.rs),
`CreateAudioBlock` (audio.rs). Display labels (e.g. `"Pattern {}"`) can still use
`.len() + 1` — that's a human-facing count, not an identity, so no collision risk.

### Lookup — always find()/position(), never bracket-index by id
`collection[x]` is only correct when `x` is a genuine position, computed
same-frame with no async gap between capture and use. The instant `x` came from
an `.id` field, or crossed a ring-buffer round-trip (click → AudioCommand →
UiCommand → gfx mutation), it must resolve via:
```rust
collection.iter_mut().find(|item| item.id == x)          // mutate
collection.iter().position(|item| item.id == x)           // then .remove(pos)
```
Bit us today in: `ToggleNote`, `ToggleStep` (patterns[pattern_id]),
`ChangeTrackVolume`, `ToggleTrackMute` (tracks[track_id]), `DeleteTrack`/
`TrackDeleted` (both audio.rs and ui_command.rs), mixer.rs's per-track column
x-position (used `track.data.id` for layout, `drag.rs` used loop position `i`
for the hit-test — silently agreed only while ids happened to equal positions).

**Not every bracket-index is wrong** — `mini_windows[SEQUENCER_ID]` is a real
fixed-slot constant, and anything resolved via `.position()`/`.enumerate()`
*within the same synchronous call*, with no round-trip in between, is fine
staying positional (e.g. click.rs's `gfx.tracks[track]` inside
`ToggleTrackWindow`, since it runs same-frame as the click). The tell is: does
this value's *meaning* survive being passed to a different thread or a later
frame? If yes, it must be an id, resolved by find().

### Audio thread is sole minting/mutating authority
`ClickResult` handlers (app/click.rs) never mutate `gfx`'s song data (tracks,
patterns, events) directly for create/delete — they only send an `AudioCommand`.
The audio thread mints the id, mutates its own copy, and sends a `UiCommand`
back with the *confirmed* result. `ui_command.rs`'s handler applies that to
`gfx` and sets `project_is_dirty = true` there (not at the click site — a
`try_push` can silently fail, and dirty should track confirmed state, not
attempted state). All confirmation commands are named past-tense:
`TrackLoaded`, `TrackDeleted`, `TrackRenamed`, `PatternLoaded`, `PatternDeleted`,
`PatternRenamed`, `AudioBlockLoaded`, `AudioBlockDeleted`.

Two independent copies minting/mutating the same conceptual thing independently
was the root cause of several today: `AddPlaylistAudioBlock` dual-minting an
`AudioBlock` on both the UI side and the audio side; the file-tray-drop-onto-
playlist path (`pending_drop`) doing the same; `DeletePattern`'s old resequencing
loop corrupting `AudioBlockType::Pattern(usize)` references on the UI side while
the audio thread's copy didn't resequence. Audio-confirms-first eliminates the
whole class — there is exactly one place per collection that ever decides the
"real" value, everything else mirrors it.

### Delete-cascade symmetry
Every delete needs *identical* cleanup applied on **both** the audio thread's
copy and gfx's copy — cleanup logic that exists on only one side is a live bug
waiting for a save/reload or an out-of-sync UI to expose it. `DeleteTrack` must
remove orphaned `AudioBlockType::Sample` events referencing the deleted track on
both sides, not just one (this was missing on the audio-thread side for a while
today — orphaned `Sample(id)` events pointing at nonexistent tracks showed up in
a saved TOML before the fix). Whenever adding a new delete path, check both
`audio.rs` and `ui_command.rs`/`click.rs`.

---

## Draw Function Mutation Rule
If a draw function directly mutates state AND returns a ClickResult that causes app.rs to send an AudioCommand doing the same mutation — that is correct. But if app.rs ALSO mutates the UI copy in response, the toggle happens twice and the net effect is nothing.

Rule: mutation of UI state happens in exactly one place. Either the draw function mutates it directly and app.rs only forwards the audio command, OR app.rs mutates UI state and sends audio command. Never both.

Current pattern for mute:
- sequencer::draw mutates track.data.is_muted directly
- Returns ClickResult::ToggleTrackMute(i) (now carried inside an InteractionResult, see Interaction State section — the mutation rule itself is unchanged, only the return type wrapping ClickResult changed)
- app.rs sends AudioCommand::ToggleTrackMute(i) only — does NOT re-toggle UI copy

This is a special case of the broader ID Minting & Lookup rule above — "UI state
entangled with a specific mutation's outcome moves with that mutation," e.g.
`context_menu = None` after a delete belongs in the delete's confirmation
handler, not the click handler, because closing the menu optimistically before
the audio thread confirms would show a menu for something that (from the audio
thread's perspective) hasn't actually been deleted yet if the command is lost.

---

## Data Model (project.rs)
```rust
struct PatternData {
    id: usize,
    name: String,
    sequences: Vec<Sequence>,
}
struct Sequence {
    track_id: u32,
    steps: Vec<Note>,
}
struct Note {
    velocity: f32,
    pitch: u8,
}
struct Voice {
    position: f32,
    is_playing: bool,
    playback_rate: f32,
    current_volume: f32,
    target_volume: f32,
    stop_at_frame: Option<f32>,
}
struct Track {
    data: TrackData,
    samples: Vec<f32>,
    voices: Vec<Voice>,
    show_velocity: bool,
    rms_l: f32,
    rms_r: f32,
    peak_hold: f32,
}
struct TrackData {
    id: u32,
    path: String,
    name: String,
    is_muted: bool,
    channels: u16,
    track_volume: f32,
    root_note: u8,
}
struct AudioBlock {
    id: usize,
    track_id: usize,
    start_step: u32,
    length: u32,
    block_type: AudioBlockType,
}
enum AudioBlockType {
    Sample(usize),
    Pattern(usize),
    Mixing,
}
```

---

## Commands

Both enums below reflect the current, real state as of today's id-integrity
refactor — every confirmation-style `UiCommand` is past-tense on purpose (see
ID Minting & Lookup section).

```rust
pub enum UiCommand {
    TrackLevel(u32, f32, f32, f32),
    TrackLoaded(Track),
    TrackRenamed(u32, String),
    TrackDeleted(u32),               // carries the stable id, NOT a position

    PatternRenamed(usize, String),
    PatternLoaded(PatternData),
    PatternDeleted(usize),

    AudioBlockDeleted(usize),
    AudioBlockLoaded(AudioBlock),

    MasterLevel(f32, f32, f32),
    SampleRateLoaded(f32),
    StepAdvanced(usize),
    PlayheadPosition(f32),
    SpectrumFrame(Vec<f32>),

    ShutdownComplete,
    SaveComplete,

    LoadProject {
        tracks: Vec<Track>,
        patterns: Vec<PatternData>,
        audio_block: Vec<AudioBlock>,
        bpm: f32,
        master_volume: f32,
        project_path: String,
    },
}

pub enum AudioCommand {
    ToggleStep(usize, u32, usize),        // pattern_id, track_id, step_idx — middle field IS an id, not a loop index (see below)
    ToggleNote(usize, u32, usize, u8),    // pattern_id, track_id, step_idx, pitch
    ChangeBpm(f32),
    DeleteAudioBlock(usize),
    CreateAudioBlock(usize, u32, usize, AudioBlockType),
    ResizeAudioBlock(usize, u32),         // ⚠️ KNOWN GAP — see Known Open Issues

    ChangeMasterVolume(f32),
    ToggleTrackMute(usize),
    ChangeTrackVolume(usize, f32),

    TogglePlay,
    Stop,
    PreviewSample(Vec<f32>),

    Shutdown,
    ShutdownWithoutSaving,
    SaveProject,
    SetProjectPath(String),

    DuplicatePattern(usize),
    AddPattern,
    DeletePattern(usize),
    ClearPattern(usize),

    RenamePattern(usize, String),
    RenameTrack(usize, String),

    LoadTrack(TrackData, Vec<f32>),
    DeleteTrack(usize),
}
```

**Gotcha worth remembering:** `ToggleStep`'s second field is typed `u32` and
named suggestively — for a while it was misread as a loop position (`track_idx`)
and someone tried adding a spurious `.find()` lookup to "convert" it. It's
already the real track id, straight from `track.data.id` at the point
`sequencer.rs` constructs the `ClickResult`. Trace a value back to where it's
*first constructed* before assuming its name tells you its type.

---

## Graphics struct (current)
```rust
pub struct Graphics {
    // wgpu
    window, surface, surface_config, device, queue, render_pipeline, vertex_buffer,
    glyph_vertex_buffer,
    // text
    glyph_cache: GlyphCache,
    font_cache: HashMap<String, fontdue::Font>,
    // file system
    pub expanded_dirs: HashSet<PathBuf>,
    pub user_fs_location: PathBuf,
    pub fs_cache: HashMap<PathBuf, Vec<(PathBuf, bool)>>,
    // ui
    pub mini_windows: Vec<MiniWindow>,
    pub z_order: Vec<usize>,
    pub context_menu: Option<ContextMenu>,
    pub active_pattern_id: usize,
    pub piano_roll_state: Option<PianoRollState>,
    icon_cache: HashMap<String, (wgpu::Texture, wgpu::BindGroup)>,
    pub tooltip: Option<Tooltip>,
    pub frame_ms: f32,
    pub track_tray_width: f32,
    pub pattern_tray_width: f32,
    pub active_tray: AudioBlockType,
    pub renaming: Option<RenameState>,
    pub show_track_tray: bool,
    pub show_pattern_tray: bool,
    pub show_save_modal: bool,
    // song
    pub project_path: String,
    pub tracks: Vec<Track>,
    pub patterns: Vec<PatternData>,
    pub audio_block: Vec<AudioBlock>,
    pub active_step: usize,
    pub playhead_beat: f32,
    pub bpm: f32,
    pub is_playing: bool,
    pub master_volume: f32,
    pub master_rms_l: f32,
    pub master_rms_r: f32,
    pub master_peak: f32,
    pub spectrum: Vec<f32>,
    pub sample_rate: f32,
    // dragging — one Option<...> field PER draggable control, not shared
    pub dragging_knob: Option<usize>,             // stores track id
    pub dragging_slider: Option<Option<usize>>,   // None = idle; Some(None) = master; Some(Some(id)) = per-track
    pub dragging_window: Option<usize>,           // window index (position — fine, same-frame use only)
    pub dragging_file: Option<PathBuf>,
    pub resizing_track_tray: bool,
    pub dragging: bool,                            // generic "something is being dragged" bailout flag
    pub resizing_event: Option<usize>,
    pub resize_drag_accumulator: f32,
    // scrolling
    pub playlist_scroll_offset: ScrollOffset,
    pub sequencer_scroll_offset: ScrollOffset,
    pub fs_scroll_offset: f32,
}
```

---

## File Tree Browser

### Design
Tree view (VS Code / FL Studio style), not nav stack (file manager style). User sees one root dir and can expand/collapse folders in place. Children are indented under their parent with a vertical guide line.

### State (on Graphics)
```rust
pub expanded_dirs: HashSet<PathBuf>,  // which dirs are currently open
pub user_fs_location: PathBuf,        // root — set once, never changes during session
pub fs_cache: HashMap<PathBuf, Vec<(PathBuf, bool)>>,  // (path, is_dir) per dir
```

### Cache pattern
- `read_dir` is NEVER called inside a draw function — disk reads only happen on toggle
- Root dir seeded into `fs_cache` in `create_graphics` at startup
- On `FsToggleDir(path)` in app.rs:
  - Expand: insert into `expanded_dirs`, read dir, store listing in `fs_cache`
  - Collapse: remove from `expanded_dirs`, remove from `fs_cache`
- `draw_fs_tree` receives `&fs_cache` and looks up entries with `fs_cache.get(dir)`

### ClickResult
```rust
FsToggleDir(PathBuf)  // replaces old FsNavPush/FsNavPop
```

### draw_fs_tree
- Recursive free function in `track_tray.rs`
- `row: &mut f32` tracks vertical position across all levels — shared mutable counter
- `depth` drives indent: `depth as f32 * PAD_16`
- Guide line drawn AFTER recursion so `*row` reflects total children height:
  ```rust
  let line_top = y + 24.0;
  // ... recurse ...
  let line_bottom = base_y + *row * PAD_32;
  // draw vertical rect from line_top to line_bottom
  ```
- Returns `Vec<IconDraw>` — recursive call's icons must be appended: `icons.append(&mut child_icons)`

### Icons
- `music_dir` — folder icon (128x128 SVG, rendered at 16x16)
- `music_file` — file icon (128x128 SVG, rendered at 16x16)
- Icons designed thick at 128px so they stay readable when scaled down

### Tray resize
- Right edge of track tray detected via `Rectangle::is_hovered_right_edge`
- Returns `DragResult::ResizeTrackTray(f32)` from `handle_drag`
- `gfx.track_tray_width` updated directly in `handle_drag`, clamped 80..400
- `tray_width` passed as parameter to `track_tray::draw` and `draw_fs_tree` — not a constant
- Same pattern applies to `pattern_tray_width` on the right side

### Future: multiple roots
Current single `user_fs_location` is designed to migrate cleanly — promote to `Vec<PathBuf>`, each with its own nav subtree in `expanded_dirs`.

---

## Hard Lessons (cumulative)

- default_output_config() not with_max_sample_rate()
- Normalization divisor is 2^(bits-1)
- reader.spec() before reader.samples()
- sample_counter increments by data.len()/2 (frames not samples)
- Stream must be stored in main or it drops and audio stops
- audio::init() must be called before run_app() — run_app blocks
- On mute, reset is_playing and position or unmute resumes mid-sample
- First beat always skipped — initialize current_step to max_steps - 1
- Shutdown fade must decrement inside the sample loop, not per callback
- Closures for build_output_stream must be 'static — pass String not &str
- SaveProject race condition: wait for SaveComplete before swapping ring buffers
- gfx.tracks and gfx.patterns must be cleared before reinitializing audio on project switch
- Segfault on Wayland on exit: Surface outlives window — set State::Init(None) before event_loop.exit()
- Cannot assign to self.state inside if let State::Ready(gfx) borrow — use should_exit: bool flag
- Scissor rects are in pixel space — derive from win.x in float space before casting
- win.x as u32 wraps to u32::MAX if negative — always check before casting
- content_h should use saturating_sub not win.height - constant
- Scroll accumulation belongs in app.rs MouseWheel handler with hover check
- Scissor rect x + w must be strictly <= surface_config.width
- Window drag clamp must account for pattern tray width on right edge
- Mutating active_pattern_id inside a draw function has no effect — use ClickResult::SelectPattern
- mini_windows push order must exactly match ID constants — wrong order causes wrong window content on wrong geometry, silent bug
- window_ranges push for playlist/piano roll is harmless — the continue in the render pass skips it
- Sequencer window height is dynamic — keep mini_windows[SEQUENCER_ID].height in sync with tracks.len() in EVERY handler that changes gfx.tracks, including bulk-load paths like LoadProject, not just incremental add/remove
- Track id must be sequential in save file — id: 0 hardcoded in file dialog produces corrupt saves
- Direct mutation in draw + ClickResult causing same mutation in app.rs = double toggle, net no-op
- Hover blocking: iterate z_order forward from current id to find windows above — skip_while(id) + skip(1)
- CursorIcon: only update cursor_icon when returned value is not Default — later draw calls overwrite it back
- glyphon made painter's algorithm impossible — switched to fontdue where text is geometry
- wgpu is immediate-mode: rebuild everything every frame
- Step button positions computed at draw time, not stored — avoids stale positions after track deletion
- serde tag = kind content = id on AudioBlockType produces clean TOML
- A step equals a beat equals 1/4 of a bar in standard 4/4 time
- read_dir in a draw function = disk read every frame — always cache dir listings in app state
- Recursive draw_fs_tree: child icons must be appended (icons.append), not just returned and dropped
- Guide line must be drawn after recursion, not before — *row isn't fully advanced until all children are processed
- fs_cache root must be seeded at startup or nothing renders on first frame
- TRAY_WIDTH as a constant breaks resizing — thread tray_width as f32 parameter through all draw calls
- per-glyph create_buffer_init is a severe perf killer — hundreds of GPU allocations per frame; replace with a persistent glyph_vertex_buffer, accumulate all glyph quads into a Vec<Vertex>, upload once with queue.write_buffer, store (byte_offset, &BindGroup) pairs instead — nearly 2x FPS improvement
- drag types must be sticky — once a drag starts, check the active flag first and return immediately; without this, moving mouse off the original hit zone starts a different drag type mid-gesture
- drag-masked mouse state: when any drag is active, pass NEG_INFINITY x/y and left_click_held: false to draw() so hover colors and window drag detection don't fire during an unrelated drag; compute in app.rs before calling gfx.draw()
- draw_chars used .take(end) instead of .take(end - start) — this caused text to render incorrectly everywhere for a long time
- track tray scissor clipping requires two separate ranges: track_tray_range (no scissor, clips to tray width) and file_tree_range (scissored to below divider_y + PAD_32). Icons need separate tray_icon_start/tray_icon_end indices to split icon_draws into scissored and unscissored draws
- divider line and File Tree title must be in track_tray_range (unscissored) not file_tree_range — moving them to draw.rs directly avoids the clipping problem
- Square was a full duplicate of Rectangle (identical methods, width==height) — no call site ever relied on Square as a distinct type, so it was pure ceremony. Deleted; Rectangle::square(x, y, size) is the constructor now.
- A field is "optional per call" only if it's actually Option<T> — a bare bool (e.g. an early `interactive: bool` attempt on RectangleCtx) can't carry the data (&MouseState) a later step needs; the field type has to carry both the yes/no AND the payload
- RectangleCtx<'a>'s lifetime must be declared somewhere in scope before use — either on the impl block (`impl<'a> RectangleCtx<'a>`) or on the individual method — plain elision rules for `&self`/`&Type` don't extend to a lifetime-parameterized struct used as a return type; write `-> RectangleCtx<'_>` or name it explicitly
- Builder chain methods must take `self` by value and return `Self` (not `&self`/`&mut self` + return something dereferenced) — mutating through a borrow and trying to return `*self` needs Copy and fights the ownership model; taking `mut self` and returning it plain is simpler and is what makes `.a().b().c()` chaining possible at all
- Building a Rectangle inline and immediately chaining off it (`Rectangle::new(..).draw_style()...`) with no separate `let` binding means the original geometry is unrecoverable afterward unless the terminal response type (DrawResponse) was deliberately given geometry fields — decide this per-callsite based on whether anything reads position again later
- Combinatorial explosion: one draw method per *combination* of optional behaviors (draw / draw_bordered / draw_interactive / draw_interactive_bordered / ...) grows as 2^n for n independent axes — the tell to switch to a builder is a second axis actually needing to combine with the first in a real call site, not just a hypothetical one
- ID minting from `.len()` collides after deletion, across tracks/patterns/events alike — use `.iter().map(|x| x.id).max().map(|m+1).unwrap_or(0)` at every creation site instead. Display labels (like "Pattern N") can still use `.len()`, since they're just a count, not an identity.
- Position vs. id confusion isn't limited to tracks — patterns and audio blocks had the identical bug, hidden as long as ids happened to equal positions (i.e. before anything was ever deleted). Any bracket-index using a value that crossed a ring-buffer round-trip, or that came from an `.id` field, must become `.find()`/`.position()`.
- Dual-copy state (audio thread's Vec vs. gfx's mirrored Vec) needs identical cleanup/minting logic applied on BOTH sides for every mutation — id-minting and delete-cascade bugs have each shown up on the wrong side (or only one side) more than once.
- Moving `ClickResult` from `graphics/mod.rs` to `app/click.rs` (to match `UiCommand`'s placement next to its handler, not scattered near producers) compiled cleanly with no import cycle — graphics code only ever *returns* `ClickResult` by value, never needs to *name* the type via an import that would create app→graphics→app.
- Sticky drag state is required PER draggable control, not shared via one generic flag — `dragging_knob` existed but `dragging_slider` didn't; the slider could only respond to the very first frame of a click, then every subsequent frame hit the generic `if self.dragging { return None }` bailout before ever reaching the slider's hit-test code again. Any new draggable needs its own `Option<...>` field, set at initial hit-detection, checked in a continuation branch positioned BEFORE the generic bailout — mirror `dragging_knob`'s shape exactly.
- Two formulas computing "the same" screen coordinate from two different files drift apart silently unless they call one shared function. Bit the mixer twice: (1) the per-track slider's y-origin — mixer.rs's draw computed it inline from window.y/height/several PAD constants, drag.rs's hit-test used a different hardcoded approximation, off by ~24px; fixed by extracting `slider::slider_y_origin(window_y, window_height)` and calling it from both places. (2) the per-track column x-position — mixer.rs's draw laid out columns using `track.data.id`, drag.rs's hit-test used the loop position `i`; only agreed by coincidence before any track was ever deleted (see ID Minting & Lookup) — fixed by switching draw to use loop position too, since layout order should reflect "which track slot," not "which id happens to be attached."
- draw_circle/draw_knob treat their (cx, cy) parameters as the CENTER of the shape; Rectangle treats (x, y) as the TOP-LEFT CORNER. Building a knob's hit-rect directly from the same (cx, cy) values as if they were a corner shifts the clickable zone by a full radius down-and-right — only the overlapping quarter of the visible circle is actually clickable, producing an intermittent "sometimes works" symptom depending on exactly where within the knob you click. Hit-rects for circular controls need `x: cx - radius, y: cy - radius`.
- `AudioCommand` payloads that read as positional (`track_idx`) aren't necessarily positions — trace a value back to where it's FIRST constructed (the ClickResult built in the draw function) before assuming its parameter name describes its type. `ToggleStep`'s middle field looked like a loop index but was always `track.data.id` at the actual construction site in sequencer.rs; "fixing" it by adding a lookup would have been actively wrong.
- **A positional tuple return can silently drop a field with zero compiler warning if the discard is explicit (`_cursor`).** `mixer::draw`'s cursor was discarded this way at its call site in `Graphics::draw()` for a while — no red squiggle, no unused-variable warning (that's what `_` suppresses on purpose), just a cursor that would never visibly update if mixer ever needed a non-default one. Fixed by wiring the field through, then eliminated the whole bug class by bundling click+cursor into one `InteractionResult` struct so there's no second field left to silently discard (see Interaction State section).
- **A merge function that decides two fields' fate from one field's check reintroduces the same silent-drop bug one level up.** The first version of `InteractionResult::or()` checked only `click` and returned `self` or `other` wholesale — this discarded a real hover-only cursor (no click, non-default cursor) the instant it was merged against a `click: None` result on either side. Each field of a bundled struct still needs its own independent resolution rule inside the merge, or bundling just moves the bug rather than fixing it.
- **Reusing a call-site variable name across different components in the same function is a readability trap, even when scoping makes it compile-safe.** Copy-pasting a merge line as a template (e.g. from `track_tray_interaction` while actually wiring up `pattern_tray::draw`'s result) and forgetting to rename it produces code that runs correctly but reads as if two different components share one identity. Name each call site's binding after its real source component, not the template it was copied from.
- **Two "correct" refactors done independently can still disagree on units.** `AudioBlock.length` was made frame-accurate for `Sample` blocks (`CreateAudioBlock` now computes it as a real step count derived from the track's actual sample count), and separately `Voice.stop_at_frame` was added for playback cutoff — but the two only line up if both agree whether they're working in frames or in raw interleaved-array indices. `samples_per_step` returns frames; `voice.position`/`track.samples.len()` are raw-index-based (stepped by 2 for stereo). Missing the `* 2.0` conversion between them silently draws/cuts at half or double the intended length. Any time two independently-built pieces of math are meant to compare against the same value, confirm they're using the same unit before wiring them together, not after.

---

## Ring Buffer
- ringbuf 0.4.8 — HeapProd/HeapCons not Producer/Consumer (trait not type)
- HeapRb::new(64).split() in main.rs
- try_push / try_pop in 0.4
- Consumer/Producer traits must be in scope
- Two ring buffers for two-way communication
- Recreate on project switch

---

## Piano Roll
- WindowKind::PianoRoll, fixed at PIANO_ROLL_ID = 3
- PianoRollState { pattern_id: usize, track_id: u32, scroll_offset: ScrollOffset } on Graphics
- Three draw regions: static (titlebar+background), piano key column (fixed), note grid (scrollable x+y)
- PianoRollDrawRanges: static_range, piano_range, grid_range
- Y scroll clamped 0.0..1448.0
- Opened via ClickResult::LoadPianoRoll(PianoRollState) from sequencer track button
- ClickResult::ToggleNote(pattern_id, track_id, step_idx, pitch)

---

## Mixer
- WindowKind::Mixer, fixed at MIXER_ID = 2
- Master + per-track volume sliders via `slider::draw` (components/slider.rs)
- **The y-origin for both draw and hit-test MUST come from `slider::slider_y_origin(window_y, window_height)`** — do not recompute this inline in more than one place; see Hard Lessons for what happens when mixer.rs and drag.rs disagree.
- Per-track column x-position is laid out by loop position (`(i + 1)`), not by `track.data.id` — see ID Minting & Lookup / Hard Lessons.
- MIXER_TRACK_HEIGHT, MIXER_THUMB_WIDTH constants in components/slider.rs (not widgets.rs — that file no longer holds these)
- Slider dragging requires its own sticky state, `Graphics.dragging_slider: Option<Option<usize>>` (see Graphics struct + Hard Lessons) — it does NOT work correctly with only the generic `dragging: bool` flag.
- `mixer::draw` now returns `InteractionResult` directly (see Interaction State section) — this is the function whose dropped cursor field motivated that refactor.

---

## Playlist
- WindowKind::Playlist, fixed at PLAYLIST_ID = 1
- Three scissor regions: static (titlebar), header (track labels, fixed x), timeline (scrollable)
- PlaylistDrawRanges: static_range, header_range, timeline_range
- Y scroll mousewheel, shift+scroll horizontal, only when playlist hovered and not covered
- AudioBlock placed via ClickResult::AddPlaylistAudioBlock(track, start_step, length, block_type) — renamed from AddPlaylistPattern once Sample/Mixing blocks needed the same click result, not just Pattern
- Block width = 32.0 * event.length as f32
- Playlist steps/patterns don't use RectangleCtx — hover color logic here depends on extra
  state (dragging_file, resizing_event, alternating group color) that a simple interactive()
  hover-swap can't express. Manual is_hovered() + hand-rolled color branches remain correct
  here; this is the right tool for genuinely more complex per-shape conditionals, not a gap
  to "fix" by forcing it through the builder.
- **`AudioBlockType::Sample` blocks now get a real, computed `length`** at creation
  time in `CreateAudioBlock` (audio.rs) — the track's real sample count, converted
  frames → steps via `samples_per_step` with a ceiling division, instead of the old
  hardcoded `1`. `AudioBlockType::Pattern` blocks are unaffected (their length was
  always correctly step-based and is untouched). See Voice Model section for how
  this same length now also drives actual playback cutoff via `stop_at_frame`.

---

## SVG Icons
- resvg — rasterized at load time, 128x128, Rgba8UnormSrgb textures
- LinearFilter sampler for downscaling
- icon_cache: HashMap<String, (wgpu::Texture, wgpu::BindGroup)>
- Shader branch: uv.x > 1.0 signals icon; actual UV = (uv.x - 2.0, uv.y)
- Tooltip cleared each frame, set on IconDraw hover, drawn last
- Icons designed at 128x128 in Figma — thick strokes so they stay readable at 16x16 render size

---

## Known Open Issues (as of this session)

Things identified but deliberately left unfixed — either out of scope for the
day's focus, or blocked on a decision not yet made. Listed so they don't get
silently lost.

**⚠️ Not re-verified this session** — carried forward unchanged from the previous
pass. If any of these were already resolved since, update/remove them; this file
only reflects what was actually discussed and fixed in the InteractionResult
session (see Interaction State section above).

- **`track.rs`'s waveform drawing has known latent bugs**, not yet triggered by
  any file tested so far: `for pixel_column ina 0..199` should be `0..200`
  (drops the last column); `sample_stride = samples_averaged.len() /
  TRACK_GRAPHICS_WIDTH` truncates to 0 on short/low-sample-rate files, which
  would produce `f32::NEG_INFINITY`/`INFINITY` from folding over an empty
  slice; `chunks(2)` assumes even-length, always-stereo sample data without
  checking `TrackData.channels`.
- **`Graphics::draw()` is a single ~500+ line function** doing at least four
  distinct jobs: click-ownership/masking setup, mini-window drawing (bespoke
  per-window scissor math), a second tier of tray/toolbar/footer/modal widgets,
  and wgpu command submission. Worth splitting into per-tier methods now that
  every window function returns a consistent `InteractionResult` — that
  consistency was the main blocker to doing this cleanly before.
- ~~`playlist.rs`'s length-computation match still has `_ => 1` for the Sample
  arm.~~ **FIXED** — `CreateAudioBlock` (audio.rs) now computes real length from
  the track's actual sample count. See Playlist section.
-  **FIXED** — sample-triggered voices now set `stop_at_frame` at trigger time and the mixing loop respects it
  (see Voice Model section). This was the original motivating bug for the whole
  session.
- **Pattern-triggered notes never set `stop_at_frame`** (`None`, full sample
  always plays). Not necessarily wrong — patterns are step-sequenced and "note
  length" may not map the same way samples do — but it's an unmade decision, not
  a deliberate one yet.
- **No voice cap / voice stealing.** `track.voices.push(...)` on every trigger
  with no limit — see Voice Model section. Fine for now, worth revisiting if
  rapid triggering ever causes a perf or memory concern.
- **Waveform-inside-the-block rendering for Sample blocks is still unbuilt** —
  the length fix makes the *block* the right size, but the block still draws as
  a plain rectangle, not a waveform preview. Would reuse the reduction math
  (mono-down → stride → per-stride min/max) already written for the track
  detail window's waveform, parameterized by width instead of hardcoded to
  `TRACK_GRAPHICS_WIDTH`.

---

## Build Order
1.  ✅ Sine wave
2.  ✅ WAV loading and playback
3.  ✅ Stereo mixing
4.  ✅ Step sequencer
5.  ✅ Voice model
6.  ✅ Volume ramping
7.  ✅ draw_rectangle with NDC conversion
8.  ✅ StepButton struct with hover detection
9.  ✅ 16 step buttons — clickable, toggleable
10. ✅ Audio and UI running simultaneously
11. ✅ Ring buffer connecting UI to audio sequencer
12. ✅ Playhead indicator
13. ✅ Multi-track UI
14. ✅ UI tracks wired to audio tracks
15. ✅ Track names rendering
16. ✅ Mute button per track
17. ✅ BPM up/down buttons in toolbar
18. ✅ Project state loads from TOML on startup
19. ✅ MIDI velocity on steps
20. ✅ Play/pause
21. ✅ Graceful shutdown
22. ✅ Project switching
23. ✅ Segfault on exit fixed
24. ✅ Keyboard shortcuts — Space, Ctrl+S
25. ✅ Master volume slider
26. ✅ Variable step counts
27. ✅ Add/delete tracks at runtime
28. ✅ draw_circle
29. ✅ Per-track volume knob
30. ✅ IMGUI refactor
31. ✅ Non-blocking file dialogs
32. ✅ Window manager — MiniWindow, WindowKind, Vec<MiniWindow>
33. ✅ Draggable windows
34. ✅ Pattern/Event system
35. ✅ Track split into Track + TrackData
36. ✅ Event-driven trigger resolution
37. ✅ Pattern tray UI
38. ✅ Velocity view per track
39. ✅ Mixer window
40. ✅ Playlist window
41. ✅ Replaced glyphon with fontdue
42. ✅ Painter's algorithm fixed — WindowDrawRange
43. ✅ Z-ordering — z_order Vec, bring_to_front
44. ✅ Playlist CRUD
45. ✅ Playlist scroll
46. ✅ Playlist scissor clipping
47. ✅ Playhead in playlist
48. ✅ Scroll only when playlist active and hot
49. ✅ Stop button
50. ✅ Cursor icon
51. ✅ Playlist labels
52. ✅ UI Audio sync
53. ✅ UI Timeline
54. ✅ Pattern tray modularized
55. ✅ Window drag boundaries
56. ✅ Context menu
57. ✅ Footer
58. ✅ Multiple font colors
59. ✅ Close context menu on outside click
60. ✅ SVG icon pipeline
61. ✅ Icon rendering
62. ✅ Toolbar SVG icons
63. ✅ IconDraw system
64. ✅ Tooltip system
65. ✅ Multiple font files — font_cache and glyph_cache as HashMaps
66. ✅ Piano roll window — PianoRollState, three scissor regions, note grid, key column
67. ✅ Track detail mini-windows — dynamic WindowKind::TrackDetail(track), pushed at runtime
68. ✅ Click ownership — click_owner from z_order, masked_mouse per window
69. ✅ Hover blocking — blocked by scanning z_order above current window, x/y set to NEG_INFINITY
70. ✅ Window push order fixed — push order matches ID constants exactly
71. ✅ Sequencer dynamic height — mini_windows[SEQUENCER_ID].height kept in sync with tracks.len()
72. ✅ Track ID trigger fix — trigger loop uses find() by track_id not direct index
73. ✅ Save file ID corruption fix — tracks must have sequential IDs in TOML
74. ✅ Track window has 200 pixel instrument wavefile generated with rectangles and strides
75. ✅ Track window has file button that goes to file path via crate `showfile`
76. ✅ File tree browser — tree view, recursive draw_fs_tree, expand/collapse via FsToggleDir
77. ✅ fs_cache — dir listings cached on toggle, never read_dir in draw loop
78. ✅ Resizable track tray — drag right edge, DragResult::ResizeTrackTray, track_tray_width on Graphics
79. ✅ SVG icons for file tree — music_dir and music_file at 16x16
80. ✅ Persistent glyph vertex buffer — eliminated per-glyph GPU allocations, ~2x FPS
81. ✅ Sticky drag system — handle_drag checks active flags first, prevents drag type switching mid-gesture
82. ✅ Drag-masked mouse state — app.rs masks mouse before passing to gfx.draw() when any drag is active
83. ✅ File tree scroll — fs_scroll_offset, count_fs_rows in project.rs, scroll handler in app.rs MouseWheel
84. ✅ File tree scissor clipping — track_tray_range + file_tree_range, tray_icon_start/end, draw_chars take(end-start) fix
85. ✅ Sample preview — path_to_preview, AudioCommand::PreviewSample, preview_samples/preview_position in audio callback
86. ✅ Rectangle draw builder — RectangleCtx, BorderStyle, DrawResponse; Square removed; toolbar.rs migrated off draw_interactive/draw_bordered
87. ✅ ID-minting fix — tracks/patterns/events/duplicates all derive new ids via max()+1 instead of .len(), on both audio.rs and app.rs sides
88. ✅ Audio-thread-as-sole-authority refactor — ClickResult handlers no longer mutate gfx song data directly for create/delete; every create/delete round-trips through AudioCommand → audio.rs mutation → confirmation UiCommand (past-tense naming) → ui_command.rs applies to gfx
89. ✅ Delete-cascade symmetry fix — DeleteTrack now cleans up orphaned Sample events on both the audio thread's copy and gfx's copy, not just one
90. ✅ Position-vs-id lookup audit — ToggleNote, ToggleStep, ChangeTrackVolume, ToggleTrackMute, DeleteTrack/TrackDeleted all converted from bracket-indexing to find()/position(), across audio.rs, ui_command.rs, and drag.rs
91. ✅ ClickResult relocated from graphics/mod.rs to app/click.rs, matching UiCommand's placement convention (defined next to its handler)
92. ✅ Mixer coordinate fixes — slider_y_origin extracted as a shared function (mixer.rs draw + drag.rs hit-test), per-track column layout switched from id-based to position-based, knob hit-rect corrected for center-vs-corner mismatch
93. ✅ Slider sticky-drag state — dragging_slider field added, mirroring dragging_knob, fixing sliders that previously only responded to the first frame of a drag
94. ✅ Sequencer height sync on LoadProject — bulk project load now recalculates mini_windows[SEQUENCER_ID].height, previously only incremental TrackLoaded/TrackDeleted did
95. ✅ InteractionResult unification — click + cursor bundled into one struct with a field-independent `or()` merge, replacing loose `(ClickResult, CursorIcon)` tuple fields across all eleven `Graphics::draw()` call sites (sequencer, playlist, mixer, piano roll, track, track tray, file tree, pattern tray, modal, toolbar, footer, context menu); fixes the silent-drop bug class that let mixer's cursor field go discarded with no compiler warning
96. ✅ Sample block length fix — CreateAudioBlock computes real step-count length from the track's actual sample count instead of the old hardcoded 1; TrackData gained track_volume back after a regression, and the fix required threading it separately from the length change
97. ✅ Voice/polyphony refactor — Track's single position/is_playing/playback_rate/current_volume fields replaced with Vec<Voice>, one Voice per simultaneously-playing note; mixing loop now nests a per-voice loop with retain-based cleanup; both Sample and pattern-note trigger sites push new voices instead of overwriting a single slot
98. ✅ stop_at_frame wired into playback — sample-triggered voices compute a real cutoff point from AudioBlock.length (steps → frames → raw-array-index via samples_per_step * 2.0) and the mixing loop's cutoff check now respects it instead of always playing to the sample's full length; this was the original bug that started the whole session

---

## Crates
- cpal — audio I/O
- hound — WAV loading
- wgpu 25 — GPU rendering
- winit 0.30 — windowing
- bytemuck — vertex casting
- ringbuf 0.4.8 — SPSC ring buffers
- fontdue — glyph rasterization and layout
- serde + toml — project file serialization
- rfd — native OS file dialog
- resvg — SVG rasterization (includes tiny-skia)
- dirs — cross-platform user directory paths
- showfile — open file in OS file manager

## Old Project
github: remysedlak/remdaw — reference only
