#[derive(Clone, Copy, Debug)]
pub struct Color {
    pub r: f32,
    pub g: f32,
    pub b: f32,
}
impl From<(f32, f32, f32)> for Color {
    fn from((r, g, b): (f32, f32, f32)) -> Self {
        Color { r, g, b }
    }
}

// monochrome
pub const LIGHT_GRAY: Color = Color { r: 0.53, g: 0.53, b: 0.53 };
pub const DARK_GRAY: Color = Color { r: 0.03, g: 0.03, b: 0.03 };
pub const DARK_GRAY_HOVER: Color = Color { r: 0.05, g: 0.05, b: 0.05 };
pub const DARK_GRAY_HOVER_HOVER: Color = Color { r: 0.06, g: 0.06, b: 0.06 };
pub const BLACK: Color = Color { r: 0.00, g: 0.00, b: 0.00 };
pub const LL_GRAY: Color = Color { r: 0.27, g: 0.27, b: 0.27 };
pub const WHITE: Color = Color { r: 1.0, g: 1.0, b: 1.0 };
pub const MINI_WINDOW_BACKGROUND: Color = Color { r: 0.1, g: 0.1, b: 0.1 };
pub const PEBBLE: Color = Color {
    r: 0.018,
    g: 0.018,
    b: 0.018,
};
pub const C_NOTE_COLOR: Color = Color { r: 0.59, g: 0.70, b: 0.30 };

// blues :'Color{r:}
pub const BLUE: Color = Color { r: 0.10, g: 0.15, b: 0.70 }; // desaturated, medium
pub const BLUE_HOVER: Color = Color { r: 0.15, g: 0.20, b: 0.85 }; // lighter on hover
pub const DARK_BLUE: Color = Color { r: 0.06, g: 0.09, b: 0.45 }; // darker but not black
pub const DARK_BLUE_HOVER: Color = Color { r: 0.10, g: 0.13, b: 0.58 };

// high contrast
pub const PURPLE: Color = Color { r: 0.20, g: 0.20, b: 0.99 };
pub const ORANGE: Color = Color { r: 0.99, g: 0.1, b: 0.0 };
pub const ORANGE_HOVER: Color = Color { r: 0.79, g: 0.2, b: 0.0 };
pub const GREEN: Color = Color { r: 0.1, g: 0.99, b: 0.1 };
pub const GREEN_HOVER: Color = Color { r: 0.1, g: 0.79, b: 0.1 };
