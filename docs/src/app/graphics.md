# Graphics & Rendering

The graphics module is responsible for two largely separate jobs: painting
windows and widgets to the screen in the right visual order, and resolving
mouse input against whatever's currently on screen. This chapter covers both,
roughly in that order — interaction (click ownership, cursor state) builds on
concepts introduced in rendering (z-order), so rendering comes first.

## Rendering Pipeline

### Painter's Algorithm
Windows and widgets are drawn back-to-front in a single pass, and each one's
vertex/text ranges are recorded so a later render pass can issue draw calls
per-window in the correct order:

```rust
struct WindowDrawRange {
    vert_start: u32,
    vert_end: u32,
    char_start: usize,
    char_end: usize,
}
```

The playlist and piano roll windows have scrollable sub-regions that need
their own scissor rects, so they populate separate `playlist_window_ranges`
and `piano_roll_ranges` instead of the generic `window_ranges` path — the
generic entry is still pushed for these two (for uniformity with the rest of
the loop) but the render pass `continue`s past it unused.

### Z-Order
`z_order: Vec<usize>` stores window indices back-to-front — the *last* entry
is topmost. `bring_to_front(z_order, id)` moves a window to the front by
removing and re-pushing its id. The toolbar is drawn outside this system
entirely, unconditionally last, so it's always on top of every window.

### Text — fontdue
Text is rendered as textured quads, not as a separate draw phase — this is a
deliberate choice, not the obvious default. An earlier attempt using
`glyphon` required all text draws to happen after all geometry, unconditionally,
because its `prepare()` had to run before the render pass opened and `render()`
inside it. That's incompatible with painter's algorithm on overlapping
windows, since a lower window's text would always draw after a higher
window's geometry. Fontdue rasterizes glyphs to CPU bitmaps once, uploads
them as `wgpu::Texture`, and treats each glyph as geometry — so text and
colored rects interleave freely in draw order.

```rust
HashMap<String, HashMap<(char, u32), (wgpu::Texture, wgpu::BindGroup, fontdue::Metrics)>>
```
Outer key is font name, inner key is `(char, size)`. `build_glyph_cache`
pre-rasterizes every size in a fixed startup slice — any `TextItem` size not
in that slice silently produces no glyph. Zero-size glyphs are skipped (a
zero-dimension texture panics wgpu). Sampling uses `FilterMode::Nearest`, not
`Linear`, since linear filtering blurs pixel-exact glyph edges.

The fragment shader branches on UV range to decide what it's drawing:
```wgsl
if in.uv.x < 0.0       { use color }               // geometry quad
else if in.uv.x > 1.0  { sample icon rgba }         // icon quad
else                   { sample glyph alpha * color } // glyph quad
```

### Rectangles
`Rectangle` is pure geometry (`x`, `y`, `width`, `height`) with three drawing
entry points depending on what a given shape actually needs:

- **`rect.draw(...)`** — plain, no border, no hover check. For static
  backgrounds and dividers.
- **`rect.draw_bordered(...)`** — border, no hover. Rarely called directly;
  mostly used internally by the builder below.
- **`rect.draw_style().interactive(Some(mouse_state)).bordered(Some(style)).draw(...)`**
  — the builder, for anything needing hover, a border, or both. Either
  chained method can be skipped or passed `None`.

The builder exists because "one method per combination of optional
behaviors" (`draw`, `draw_bordered`, `draw_interactive`,
`draw_interactive_bordered`, ...) scales as 2ⁿ in the number of independent
axes — it only became worth collapsing once a real call site (toolbar
buttons) needed both axes at once.

```rust
pub struct RectangleCtx<'a> {
    rectangle: &'a Rectangle,             // borrowed, never copied/owned
    interactive: Option<&'a MouseState>,  // None = skip hover check entirely
    border: Option<BorderStyle>,          // None = call draw() not draw_bordered()
}
```
`.interactive(...)` / `.bordered(...)` take and return `Self` by value to
allow chaining; `.draw(...)` is the one terminal call, consuming the ctx and
returning `DrawResponse { hovered, x, y, width, height }`. Geometry fields
are included on the response because most interactive rects (toolbar
buttons) get read again afterward for tooltip/icon positioning — a small,
deliberate cost for this file's usage pattern, not necessarily the right
tradeoff elsewhere.

Static shapes with no hover/border need should stay on plain `.draw(...)` —
routing a divider through the builder "for consistency" produces a
`DrawResponse` nobody reads.

> **Gotcha:** building and chaining in one expression discards the
> `Rectangle` itself — `let btn = Rectangle::new(...).draw_style()...draw(...)`
> binds `btn` to the `DrawResponse`, not the rectangle, so `btn.x` won't
> compile unless you deliberately need the response's own geometry fields.
> If anything downstream needs the *original* rectangle (common — many
> toolbar buttons position a sibling off it, e.g. `bpm_down.y = bpm_up.y +
> 18.0`), bind the `Rectangle` separately first.

## Interaction Model

### Window System
Fixed windows are addressed by constant, and push order in `create_graphics`
must match these exactly — the constants are direct indices into
`mini_windows`, so a wrong push order silently draws the wrong content on the
wrong geometry:
```rust
pub const SEQUENCER_ID: usize  = 0;
pub const PLAYLIST_ID: usize   = 1;
pub const MIXER_ID: usize      = 2;
pub const PIANO_ROLL_ID: usize = 3;
```
Track detail windows are dynamic — pushed at runtime as
`WindowKind::TrackDetail(track)`, with `id = mini_windows.len() - 1` after
push and added to `z_order` immediately. A wildcard arm in the draw match
handles every id ≥ 4.

The sequencer window's height is derived, not fixed — it grows with track
count (`window.height + TRACK_GAP * tracks.len()`), and
`mini_windows[SEQUENCER_ID].height` must be recalculated in *every*
`UiCommand` handler that changes `gfx.tracks`, including bulk-load paths
like `LoadProject` — not just the incremental add/remove handlers. Missing
this leaves the drawn background stuck at its startup height while the
per-track rows extend past it.

### InteractionResult
Every window/component draw function hit-tests its own geometry against the
mouse and needs to report back two things: what got clicked, and what cursor
should show. Both are bundled into one struct rather than returned as loose
tuple fields:
```rust
pub struct InteractionResult {
    pub click: ClickResult,
    pub cursor: CursorIcon,
}
```
This exists because loose tuple fields let a real bug ship silently — 
`mixer::draw`'s cursor field was once destructured as `_cursor` at its call
site with no compiler warning, so a hover-cursor case for the mixer simply
never worked, invisibly, until someone needed it.

Merging two results uses a per-field `or()`, not a whole-struct fallback:
```rust
impl InteractionResult {
    pub fn or(self, other: InteractionResult) -> InteractionResult {
        InteractionResult {
            click: if self.click != ClickResult::None { self.click } else { other.click },
            cursor: if self.cursor != CursorIcon::Default { self.cursor } else { other.cursor },
        }
    }
}
```
`click` and `cursor` resolve independently because they're independent
questions — a first draft that checked only `click` and returned one side
wholesale would drop a real hover cursor any time it was merged against a
`click: None` result on either side, which is exactly the kind of case that
matters (mouse hovering, nothing clicked).

`InteractionResult { click: ClickResult::None, cursor: CursorIcon::Default }`
is the identity value for `or()` — merging anything with it returns the
other side unchanged. It's the accumulator's starting value in
`Graphics::draw()`, and what any component should return when its own
hit-test found nothing.

**Naming convention:** inside a component's own `draw()`, the value being
built is named `interaction`. At each call site inside `Graphics::draw()`,
the result is bound as `<component>_interaction` — named for the actual
source component (`pattern_interaction`, not `side_panel_interaction`, even
though it's reached via `side_panel::pattern_tray::draw`). The running total
stays `interaction`, reassigned via `interaction = interaction.or(x)` — never
passed in as `&mut`, since every component recomputes its result fresh each
frame.

Manual cursor overrides (drag/resize states forcing `ColResize` or
`Default`) write directly to `interaction.cursor` near the end of
`Graphics::draw()`, bypassing `or()` entirely — these are unconditional
overrides, not a component reporting a competing hit-test result, so they
don't belong in the reduction chain.

> **Known limitation:** `or()`'s priority is really "whichever call site
> runs earlier in source order wins." Only mini-windows get real z-order-
> aware click ownership (via `click_owner`/`blocked` masking); toolbar,
> footer, and both trays merge in whatever order they're coded. Not
> currently a bug — none of those overlap mini-window bounds — but worth
> revisiting if that ever changes.
