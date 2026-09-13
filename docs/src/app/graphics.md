# Graphics & Rendering

The graphics module paints windows and widgets to the screen in the right visual order. This chapter covers the rendering pipeline: how shapes and text get drawn, in what order, and the tools built for common drawing patterns.

## Rendering Pipeline

### Painter's Algorithm

Windows and widgets are drawn back-to-front in a single pass. Each one's vertex/text/icon ranges get recorded so a later render pass can issue draw calls per-window in the correct order:

```rust
struct WindowDrawRange {
    pub vert_start: u32,
    pub vert_end: u32,
    pub char_start: usize,
    pub char_end: usize,
    pub icon_start: usize,
    pub icon_end: usize,
}
```

Most windows only need one `RecordedRegion`. Playlist and piano roll are the exception — they have scrollable sub-regions that each need their own scissor rect (playlist has static/ruler/header/timeline; piano roll has static/keys/grid). Instead of going through the generic single-region path, they push multiple `RecordedRegion`s directly and `continue` past the loop's normal bottom-of-loop record call.

When a window has both icons and multiple sub-regions, only one sub-region should own that window's icons — attaching the same icon range to every sub-region would draw them more than once. Playlist's icons conceptually belong to its static region (the one scissored to the whole window), so `icon_start`/`icon_end` get captured once, right after the icon-push loop runs, and only the static region's `record(...)` call gets the real range. The other sub-regions (ruler, header, timeline) get an empty range at that same offset, so nothing extra gets drawn there.

Icons follow the same recording shape as text. A persistent `icon_vertex_buffer` holds the quad geometry, and `icon_draws: Vec<(u64, &wgpu::BindGroup)>` records a byte offset plus bind group per icon — same reason glyphs need this, since every icon has its own texture and needs its own bind group set before drawing. `WindowDrawRange` carries `icon_start`/`icon_end` right alongside `char_start`/`char_end` so icons get clipped and z-ordered exactly like everything else in that region.

Icons didn't always work this way. They used to get collected into one flat list and drawn in a single unscissored pass after every region had already been drawn — which meant they ignored window z-order and scissor rects completely. An icon belonging to a window underneath could paint straight through a window on top of it. The fix was making icons go through the same region-recording path as text. Any new kind of drawable added to this engine needs to go through that same path, or it'll silently skip z-order and scissoring the same way icons did.

### Z-Order

`z_order: Vec<usize>` stores window indices back-to-front — the last entry is topmost. `bring_to_front(z_order, id)` moves a window to the front by removing and re-pushing its id. The toolbar sits outside this system entirely and is always drawn last, so it's always on top of every window.

### Text — fontdue

Text is rendered as textured quads, not as a separate draw phase. That's a deliberate choice, not the obvious default. An earlier attempt used `glyphon`, which required all text draws to happen after all geometry, unconditionally — its `prepare()` had to run before the render pass opened and `render()` inside it. That's incompatible with painter's algorithm on overlapping windows, since a lower window's text would always end up drawn on top of a higher window's geometry. Fontdue rasterizes glyphs to CPU bitmaps once, uploads them as `wgpu::Texture`s, and treats each glyph as geometry — so text and colored rects can interleave freely in draw order.

```rust
HashMap<String, HashMap<(char, u32), (wgpu::Texture, wgpu::BindGroup, fontdue::Metrics)>>
```

Outer key is font name, inner key is `(char, size)`. `build_glyph_cache` pre-rasterizes every size in a fixed startup slice — any `TextItem` size not in that slice silently produces no glyph. Zero-size glyphs get skipped (a zero-dimension texture panics wgpu). Sampling uses `FilterMode::Nearest`, not `Linear`, since linear filtering blurs pixel-exact glyph edges.

The fragment shader branches on UV range to decide what it's drawing:

```wgsl
if in.uv.x < 0.0       { use color }                // geometry quad
else if in.uv.x > 1.0  { sample icon rgba }          // icon quad
else                   { sample glyph alpha * color } // glyph quad
```

### Rectangles

`Rectangle` is pure geometry (`x`, `y`, `width`, `height`) with three drawing entry points depending on what a shape actually needs:

- **`rect.draw(...)`** — plain, no border, no hover check. For static backgrounds and dividers.
- **`rect.draw_bordered(...)`** — border, no hover. Rarely called directly; mostly used internally by the builder below.
- **`rect.draw_style().interactive(Some(mouse_state)).bordered(Some(style)).draw(...)`** — the builder, for anything that needs hover, a border, or both. Either chained method can be skipped or passed `None`.

The builder exists because "one method per combination of optional behaviors" (`draw`, `draw_bordered`, `draw_interactive`, `draw_interactive_bordered`, ...) scales as 2ⁿ in the number of independent axes. It only became worth collapsing once a real call site (toolbar buttons) needed both axes at the same time.

```rust
pub struct RectangleCtx<'a> {
    rectangle: &'a Rectangle,             // borrowed, never copied/owned
    interactive: Option<&'a MouseState>,  // None = skip hover check entirely
    border: Option<BorderStyle>,          // None = call draw() not draw_bordered()
}
```

`.interactive(...)` / `.bordered(...)` take and return `Self` by value so they can chain. `.draw(...)` is the one terminal call — it consumes the ctx and returns `DrawResponse { hovered, x, y, width, height }`. Geometry fields are on the response because most interactive rects (toolbar buttons) get read again afterward for tooltip/icon positioning. Small deliberate cost for this file's usage pattern, not necessarily the right tradeoff everywhere else.

Static shapes with no hover/border need should just stay on plain `.draw(...)`. Routing a divider through the builder "for consistency" just produces a `DrawResponse` nobody ever reads.

> **Gotcha:** building and chaining in one expression discards the `Rectangle` itself. `let btn = Rectangle::new(...).draw_style()...draw(...)` binds `btn` to the `DrawResponse`, not the rectangle — `btn.x` won't compile unless you actually meant to use the response's own geometry fields. If anything downstream needs the *original* rectangle (common — a lot of toolbar buttons position a sibling off it, like `bpm_down.y = bpm_up.y + 18.0`), bind the `Rectangle` separately first.
