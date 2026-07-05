#[derive(Clone, Copy, Debug)]
pub struct Color {
    pub r: f32,
    pub g: f32,
    pub b: f32,
}
impl From<(f32, f32, f32)> for Color {
    /// Take in a rgb tuple and return a Color struct
    fn from((r, g, b): (f32, f32, f32)) -> Self {
        Color { r, g, b }
    }
}
impl Color {
    pub fn hovered(self) -> Color {
        let max_channel = self.r.max(self.g).max(self.b);
        let (h, s, l) = rgb_to_hsl(self.r, self.g, self.b);
        let l = if max_channel > 0.85 {
            l * 0.85 // scale down proportionally, never a flat subtract
        } else {
            l + (1.0 - l) * 0.25 // move 25% of the way to white
        };
        let (r, g, b) = hsl_to_rgb(h, s, l);
        Color { r, g, b }
    }
}

/// Convert RGB (0.0-1.0 each) to HSL (h in degrees 0-360, s and l in 0.0-1.0)
fn rgb_to_hsl(r: f32, g: f32, b: f32) -> (f32, f32, f32) {
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let l = (max + min) / 2.0;

    if (max - min).abs() < f32::EPSILON {
        return (0.0, 0.0, l); // achromatic: gray, black, white
    }

    let d = max - min;
    let s = if l > 0.5 {
        d / (2.0 - max - min)
    } else {
        d / (max + min)
    };

    let h = if max == r {
        ((g - b) / d) % 6.0
    } else if max == g {
        (b - r) / d + 2.0
    } else {
        (r - g) / d + 4.0
    };
    let h = h * 60.0;
    let h = if h < 0.0 { h + 360.0 } else { h };

    (h, s, l)
}

/// Convert HSL back to RGB (0.0-1.0 each)
fn hsl_to_rgb(h: f32, s: f32, l: f32) -> (f32, f32, f32) {
    if s.abs() < f32::EPSILON {
        return (l, l, l); // achromatic
    }

    let c = (1.0 - (2.0 * l - 1.0).abs()) * s;
    let x = c * (1.0 - ((h / 60.0) % 2.0 - 1.0).abs());
    let m = l - c / 2.0;

    let (r1, g1, b1) = if h < 60.0 {
        (c, x, 0.0)
    } else if h < 120.0 {
        (x, c, 0.0)
    } else if h < 180.0 {
        (0.0, c, x)
    } else if h < 240.0 {
        (0.0, x, c)
    } else if h < 300.0 {
        (x, 0.0, c)
    } else {
        (c, 0.0, x)
    };

    (r1 + m, g1 + m, b1 + m)
}
// monochromes
pub const LIGHT_GRAY: Color = Color {
    r: 0.53,
    g: 0.53,
    b: 0.53,
};

pub const GHOST: Color = Color {
    r: 0.33,
    g: 0.33,
    b: 0.33,
};

pub const DARK_GRAY: Color = Color {
    r: 0.03,
    g: 0.03,
    b: 0.03,
};

pub const BLACK: Color = Color {
    r: 0.00,
    g: 0.00,
    b: 0.00,
};
pub const WHITE: Color = Color {
    r: 1.0,
    g: 1.0,
    b: 1.0,
};

pub const LL_GRAY: Color = Color {
    r: 0.27,
    g: 0.27,
    b: 0.27,
};
pub const MINI_WINDOW_BACKGROUND: Color = Color {
    r: 0.1,
    g: 0.1,
    b: 0.1,
};
pub const SURFACE: Color = Color {
    r: 0.018,
    g: 0.018,
    b: 0.018,
};

pub const SURFACE_HOVER: Color = DARK_GRAY;

pub const C_NOTE_COLOR: Color = Color {
    r: 0.59,
    g: 0.70,
    b: 0.30,
};

// blues :'Color{r:}
pub const BLUE: Color = Color {
    r: 0.10,
    g: 0.15,
    b: 0.70,
}; // desaturated, medium

pub const DARK_BLUE: Color = Color {
    r: 0.06,
    g: 0.09,
    b: 0.45,
}; // darker but not black

// high contrast
// pub const PURPLE: Color = Color { r: 0.20, g: 0.20, b: 0.99 };
pub const ORANGE: Color = Color {
    r: 0.99,
    g: 0.1,
    b: 0.0,
};

pub const GREEN: Color = Color {
    r: 0.1,
    g: 0.99,
    b: 0.1,
};
// pub const GREEN_HOVER: Color = Color { r: 0.1, g: 0.79, b: 0.1 };
