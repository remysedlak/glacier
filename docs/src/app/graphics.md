# Graphics & Rendering

The graphics module is responsible for painting windows and widgets to the screen in the right visual order. This chapter covers the rendering pipeline: how shapes and text get drawn, in what order, and the tools built for common drawing patterns.

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
and `piano_roll_ranges` instead of the generic `window_ranges` path. The
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
