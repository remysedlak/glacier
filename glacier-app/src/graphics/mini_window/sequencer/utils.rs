use crate::graphics::color::{Color, Palette};

pub fn step_color(
    palette: &Palette,
    hovered: bool,
    is_active: bool,
    is_ghost: bool,
    active_step: bool,
) -> Color {
    // Keep the playhead visible even when it moves beyond the recorded steps.
    if active_step {
        return if hovered {
            palette.primary.shade(5)
        } else {
            palette.primary.shade(4)
        };
    }

    if is_ghost {
        return if hovered {
            palette.neutral.shade(6)
        } else {
            palette.neutral.shade(5)
        };
    }

    match (hovered, is_active) {
        (true, true) => palette.neutral.shade(3),
        (true, false) => palette.neutral.shade(6),
        (false, true) => palette.neutral.shade(0),
        (false, false) => palette.neutral.shade(7),
    }
}
