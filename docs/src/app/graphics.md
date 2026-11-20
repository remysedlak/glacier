# Graphics & Rendering

The graphics module paints windows and widgets to the screen in the right visual order. This chapter covers the rendering pipeline: how shapes and text get drawn, in what order, and the tools built for common drawing patterns.

## Rendering Pipeline

### Painter's Algorithm

`Graphics::draw` creates one `FrameDrawContext` for the frame and passes it to components. It owns the CPU-side shape, glyph, and icon buffers, along with the draw ranges and interaction results. Components write into those shared buffers as they draw instead of returning temporary vertex and text collections for the caller to append.

Windows and widgets are drawn back-to-front. `FrameDrawContext::with_region` captures the current buffer positions, runs a closure that draws a component or sub-region, then records the ranges and optional scissor rectangle added by that closure. The order of these calls is the painter's order:

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

Most windows use one region. The playlist records four clipped regions—static content, ruler, track headers, and timeline—and the piano roll records three—static content, keys, and grid. Each region is drawn inside its own `with_region` call, so its shapes, text, icons, and scissor stay associated. For example:

```rust
frame.with_region(Some(scissor), |frame| {
    // Draw this region's shapes, text, and icons into the frame.
});
```

The closures run in draw order. A drawable belongs to the region whose closure adds it, so icons do not need to be manually assigned to a range after drawing.

Icons follow the same recording shape as text. A persistent `icon_vertex_buffer` holds the quad geometry, and `icon_draws` records each icon quad's byte offset so regions can find their contiguous ranges. All configured icons share one color atlas and bind group; each quad's UVs select its icon within the atlas. Glyphs use a separate single-channel atlas and bind group. The renderer binds each atlas once and submits one batched draw for the region's glyphs and one for its icons. `WindowDrawRange` carries `icon_start`/`icon_end` alongside `char_start`/`char_end`, so icons remain clipped and ordered with their region.

Icons didn't always work this way. They used to get collected into one flat list and drawn in a single unscissored pass after every region had already been drawn — which meant they ignored window z-order and scissor rects completely. An icon belonging to a window underneath could paint straight through a window on top of it. The fix was making icons go through the same region-recording path as text. Any new kind of drawable added to this engine needs to go through that same path, or it'll silently skip z-order and scissoring the same way icons did.

### Z-Order

`z_order: Vec<usize>` stores window indices back-to-front — the last entry is topmost. `bring_to_front(z_order, id)` moves a window to the front by removing and re-pushing its id. The toolbar sits outside this system entirely and is always drawn last, so it's always on top of every window.

### Text — fontdue

Text is rendered as textured quads, not as a separate library-managed draw phase. That's a deliberate choice, not the obvious default. An earlier attempt used `glyphon`, which required all text draws to happen after all geometry, unconditionally — its `prepare()` had to run before the render pass opened and `render()` inside it. That's incompatible with painter's algorithm on overlapping windows, since a lower window's text would always end up drawn on top of a higher window's geometry. Fontdue rasterizes the configured glyphs to CPU bitmaps at startup; Glacier packs those bitmaps into one shared GPU atlas. Each glyph remains a quad in the frame's glyph vertex buffer, with UVs pointing to its rectangle in the atlas. The region renderer binds that atlas once and draws the region's contiguous glyph range.

The glyph cache maps `(font, character, size)` to atlas UV bounds and fontdue metrics. One atlas contains both configured fonts and all sizes in the fixed startup slice — any `TextItem` size not in that slice silently produces no glyph. Zero-size glyphs are skipped. Sampling uses `FilterMode::Nearest`, not `Linear`, since linear filtering blurs pixel-exact glyph edges. Icons have their own color atlas because their pixels are RGBA rather than glyph coverage values.

The fragment shader uses the UV and a vertex marker to decide what it's drawing:

```wgsl
if in.uv.x < 0.0       { use color }                  // geometry quad
else if in.local_pos.x > 0.5 { sample icon rgba }    // icon quad marker
else                   { sample glyph alpha * color } // glyph quad
```

Glyph and icon UVs both stay within `0.0..1.0` because they address rectangles inside their respective atlases. The icon marker is carried in `local_pos.x`; the earlier UV-range convention for icons no longer applies.

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
