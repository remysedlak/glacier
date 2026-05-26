# How to build a UI library
A plan for integrating user I/O into UI interactivity.

---

## Design Philosophies
- Updating a field with a one-dimensional range of values (-1.0, 1.0) allows both typed precision and UI interactivity
- UI components should only have one implementation allowing shape changes
- Small repeated components like steps and buttons should look good vertically stacked and in a row
- Drag and drop boundaries are strict
- Colors come from a hardcoded set of named RGB float constants (`colors.rs`)
- App-owned graphics state includes all coordinates of movable components
- IMGUI pattern — no retained widget objects; `draw()` returns a `ClickResult` enum
- `clicked: bool` passed to `draw()` — hover every frame, click only when true
- `draw()` returns `ClickResult`, App dispatches `AudioCommand` — graphics never talks to audio directly

## Infrastructure
- The graphics module only ever takes commands from the app and audio
- All logic related to user input stays in `app.rs`
- All logic related to audio stays in `audio.rs`
- Graphics state lives on the `Graphics` struct — coordinates, window positions, active pattern, z-order

---

## Detecting User Events

### Mouse Movement
All mouse movement is updated by `WindowEvent::CursorMoved` (cursor position = (x: f32, y: f32))
- Mouse coordinates are saved in `MouseState` every frame, enabling hover detection against any component's bounding box
- `prev_mouse_x` / `prev_mouse_y` saved on `App` for computing deltas during drag

### Clicking
All clicks tracked via `MouseButton` press state.
- Left click: buttons, toggles, primary interactions
- Right click: context menus (`ContextMenu` + `ContextMenuKind`)
- `clicked: bool` is set true on `MouseInput` press, consumed after `draw()` — prevents double-firing
- Graphics returns `ClickResult` — App pattern-matches and dispatches `AudioCommand` or mutates state

### Context Menus
- `Option<ContextMenu>` on `Graphics` — only one open at a time
- Right-click on a pattern or track → set `Some(ContextMenu { kind, x, y })`
- Any click on a menu item → `ClickResult::CloseContextMenu` → set `None`
- Click anywhere else → set `None`

---

## Dragging
Dragging is defined by mouse movement while a click is sustained.
- Check if mouse is pressed, then use mouse position delta for logic
- `dragging: bool` on `App` tracks sustained press state
- `prev_mouse_x` / `prev_mouse_y` on `App` used to compute `dx` / `dy` each frame

---

## UI Components

### Buttons (anything you click to interact with)
Hit test:
```
mouse_x > r.x && mouse_x < r.x + r.width && mouse_y > r.y && mouse_y < r.y + r.height
```
- `Rectangle` struct: `is_hovered`, `draw()`, `hover_color`, `active_color`, `active_step_color`
- Button logic fires on click (toggle) or release depending on action type
- Step buttons are 16 (or variable count) per track — clickable, toggleable, carry velocity values 0.0–127.0

### Knobs (rotary dial for continuous values)
- `draw_knob` in `ui.rs` — fan tessellation circle with an indicator line
- Value stored as normalized float, mapped to degrees → radians → cos/sin → NDC
- Per-track volume knobs: `dragging_knob: Option<usize>` on `Graphics`
- Drag vertically to change value — `dy` applied as delta, clamped to valid range
- Returns `DragResult::DragVolumeKnob(track_idx, new_value)`

### Sliders (drag a value along a track from min to max)
- Press and hold `MouseButton::Left`, move mouse to change value
- Normalize input: pixels moved / track max dimension
- Horizontal sliders track x, vertical sliders track y
- Slider UI coordinate reflects current drag position — only updated while mouse is held
- Master volume slider lives in the Mixer window
- Returns `DragResult::DragVolumeSlider(new_value)`

### Drag and Drop (free 2D placement)
- Like sliders but both x and y are free — no min/max clamp
- Developer specifies hit zone for pickup and valid drop targets
- Drop outside a valid target: action ignored
- Used in the Playlist window — dragging patterns onto the timeline
- `ClickResult::AddPlaylistPattern(track, start_step, length, AudioBlockType)` on valid drop
- Items show dragged state visually while in flight

### Draggable Windows
- `MiniWindow` system — `Vec<MiniWindow>` on `Graphics`, draggable via titlebar
- `dragging_window: Option<usize>` on `App`
- Drag clamp: x bounded by surface width minus pattern tray width; y keeps titlebar on screen
- `z_order: Vec<usize>` controls painter's algorithm draw order
- Clicking a window calls `bring_to_front` — pushes to end of z_order (drawn last = on top)
- Toolbar layer always drawn last — always on top of all windows

---

## Rendering Pipeline

### Immediate Mode
- wgpu geometry rebuilt every frame — no retained geometry objects
- `draw()` functions return `Vec<Vertex>` + `ClickResult`; vertices pushed into the render pipeline each frame
- `request_redraw()` called at end of every draw to keep loop continuous

### Text (fontdue)
- Glyphs rasterized to CPU bitmaps → uploaded as `wgpu::Texture` (R8Unorm)
- Each glyph is a textured quad — text IS geometry, interleaved with colored rects
- Glyph cache: `HashMap<(char, u32), (wgpu::Texture, wgpu::BindGroup, fontdue::Metrics)>`
- Any size used at draw time must be present in the cache built at startup
- `FilterMode::Nearest` — linear blurs pixel-exact glyphs

### Painter's Algorithm / Z-Ordering
- `WindowDrawRange` tracks vertex and char_draws ranges per window
- Draw loop: for each window in z_order → record ranges → draw geometry → draw text
- Context menu drawn after all windows, toolbar drawn last
- Scissor rects used in Playlist for per-region clipping — x/w must be strictly <= surface width

### Shader
```wgsl
// uv.x < 0    → geometry (color-only)
// uv.x in 0..1 → glyph quad (sample alpha texture, tint with vertex color)
// uv.x > 1    → icon quad (RGBA texture, actual UV = uv.x - 2.0)
```

### SVG Icons
- resvg rasterization at load time → `Rgba8UnormSrgb` textures
- `icon_cache: HashMap<String, (wgpu::Texture, wgpu::BindGroup)>`
- `IconDraw` returned from toolbar, iterated in `graphics.rs`
- Tooltip drawn topmost — cleared each frame, set on icon hover

---

## Layout Constants (ui.rs)
```rust
TITLEBAR_HEIGHT: f32 = 32.0
BAR_GAP: f32 = 12.0
BUTTON_GAP: f32 = 24.0
TRACK_GAP: f32 = 72.0
KNOB_RADIUS: f32 = 13.0
TOOLBAR_Y: f32 = 32.0
TOOLBAR_MARGIN: f32 = 4.0
PADDING: f32 = 16.0
ICON_WIDTH: f32 = 32.0
ICON_HEIGHT: f32 = 24.0
```

---

## Result Types
```rust
pub enum ClickResult {
    Step(usize, usize, usize),         // pattern_id, track_idx, step_idx
    Mute(usize),
    ChangeBpm(f32),
    TogglePlay,
    ProjectFileDialog,
    TrackFileDialog,
    DeleteTrack(usize),
    ToggleSequencerWindow,
    ToggleMixerWindow,
    TogglePlaylistWindow,
    SelectPattern(usize),
    AddPlaylistPattern(usize, u32, usize, AudioBlockType),
    DeletePlaylistPattern(usize),
    CloseContextMenu,
    None,
}

pub enum DragResult {
    DragVolumeSlider(f32),
    DragVolumeKnob(usize, f32),
    None,
}
```

---

## Hard-Won Lessons
- Mutating `active_pattern_id` inside a draw function has no effect — it's a local copy; use `ClickResult::SelectPattern(id)` and mutate in `Graphics::draw`
- Cursor icon overwrite bug: only update `cursor_icon` when returned value is not `Default`
- Borrow checker conflicts in draw loops: clone needed data (e.g. `steps_data`) before the loop
- `win.x as u32` wraps to `u32::MAX` if negative — always check `if win.x < 0.0 { 0u32 }` before casting
- Scissor rect `x + w` must be strictly <= `surface_config.width` — clamp before `set_scissor_rect`
- `content_h` should be `win_bottom.saturating_sub(content_y)`, not `win.height - constant`
- Scroll accumulation belongs in `app.rs` MouseWheel handler with hover check, not in `Graphics::draw`
- Window drag clamp must account for pattern tray width on right edge
- `ClickResult` enum replacing `Option<(usize, usize)>` enables multiple click targets from one handler
