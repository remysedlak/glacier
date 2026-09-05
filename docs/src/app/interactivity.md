# Interaction

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
