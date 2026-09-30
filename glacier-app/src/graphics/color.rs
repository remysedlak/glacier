//! Custom color class

use palette::{IntoColor, Oklch, Srgb};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
/// a color pallete contains all colors used in the DAW
pub struct Palette {
    pub primary: ColorRamp,
    pub secondary: ColorRamp,
    pub neutral: ColorRamp,
    pub success: ColorRamp,
    pub danger: ColorRamp,
}

impl Palette {
    /// Save the pallete to a location on disk
    pub fn new(
        primary: Color,
        secondary: Color,
        neutral: Color,
        success: Color,
        danger: Color,
    ) -> Self {
        Palette {
            primary: ColorRamp::new(primary),
            secondary: ColorRamp::new(secondary),
            neutral: ColorRamp::new(neutral),
            success: ColorRamp::new(success),
            danger: ColorRamp::new(danger),
        }
    }

    /// load a pallete from a file on disk
    pub fn load_from_toml(file_path: &str) -> Option<Palette> {
        let text = std::fs::read_to_string(file_path).ok()?;
        toml::from_str(&text).ok()
    }

    /// Save the pallete to a location on disk
    pub fn save_to_toml(&self, file_path: &str) {
        let path = std::path::Path::new(file_path);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).ok();
        }

        let text = toml::to_string(self).unwrap();
        if let Err(e) = std::fs::write(file_path, &text) {
            eprintln!("Failed to save project: {}", e);
        }
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct ColorRamp {
    shades: [Color; 8], // index 0 = darkest ... 9 = lightest
}

impl ColorRamp {
    pub fn new(color: Color) -> ColorRamp {
        let srgb = Srgb::new(color.r, color.g, color.b);
        let oklch: Oklch = srgb.into_color();

        const L_STEPS: [f32; 8] = [0.09, 0.20, 0.33, 0.45, 0.58, 0.70, 0.83, 0.95];

        let mut shades = [color; 8];
        for i in 0..8 {
            let shade_oklch = Oklch::new(L_STEPS[i], oklch.chroma, oklch.hue);
            let shade_srgb: Srgb = shade_oklch.into_color();
            shades[i] = Color {
                r: shade_srgb.red,
                g: shade_srgb.green,
                b: shade_srgb.blue,
                a: color.a,
            };
        }
        ColorRamp { shades }
    }

    pub fn shade(&self, i: usize) -> Color {
        self.shades[i]
    }
}

// old code below this line

#[derive(Serialize, Deserialize, Clone, Copy, Debug)]
pub struct Color {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}
impl From<(f32, f32, f32, f32)> for Color {
    /// Take in a rgb tuple and return a Color struct
    fn from((r, g, b, a): (f32, f32, f32, f32)) -> Self {
        Color { r, g, b, a }
    }
}
impl Color {
    /// Return new color to display for interactive state
    pub fn hovered(self) -> Color {
        // hand-picked overrides for colors where the generic formula
        // produces bad contrast (near-black backgrounds under white text/icons)
        if self.is(DARK_GRAY) {
            return DARK_GRAY_HOVER;
        }
        if self.is(SURFACE) {
            return SURFACE_HOVER;
        }

        // generic fallback for colors without a hand-picked hover value
        let max_channel = self.r.max(self.g).max(self.b);
        let (h, s, l) = rgb_to_hsl(self.r, self.g, self.b);
        let l = if max_channel > 0.85 {
            l * 0.85
        } else {
            l + (1.0 - l) * 0.15 // smaller nudge than before — these are small icon buttons, not big flat panels
        };
        let (r, g, b) = hsl_to_rgb(h, s, l);
        let a = self.a;
        Color { r, g, b, a }
    }

    fn is(self, other: Color) -> bool {
        (self.r - other.r).abs() < f32::EPSILON
            && (self.g - other.g).abs() < f32::EPSILON
            && (self.b - other.b).abs() < f32::EPSILON
    }

    pub fn normalize_rgb_value(value: u8) -> f32 {
        return value as f32 / 255.0;
    }

    pub fn shade(self, value: u8) {}
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
    a: 1.0,
};

pub const GHOST: Color = Color {
    r: 0.33,
    g: 0.33,
    b: 0.33,
    a: 1.0,
};

pub const DARK_GRAY: Color = Color {
    r: 0.03,
    g: 0.03,
    b: 0.03,
    a: 1.0,
};

pub const PATTERN_BLOCK: Color = Color {
    r: 0.26,
    g: 0.26,
    b: 0.26,
    a: 0.4,
};

pub const DARK_GRAY_HOVER: Color = Color {
    r: 0.05,
    g: 0.05,
    b: 0.05,
    a: 1.0,
};

pub const BLACK: Color = Color {
    r: 0.00,
    g: 0.00,
    b: 0.00,
    a: 1.0,
};
pub const WHITE: Color = Color {
    r: 1.0,
    g: 1.0,
    b: 1.0,
    a: 1.0,
};

pub const LL_GRAY: Color = Color {
    r: 0.27,
    g: 0.27,
    b: 0.27,
    a: 1.0,
};
pub const MINI_WINDOW_BACKGROUND: Color = Color {
    r: 0.1,
    g: 0.1,
    b: 0.1,
    a: 1.0,
};
pub const SURFACE: Color = Color {
    r: 0.018,
    g: 0.018,
    b: 0.018,
    a: 1.0,
};

pub const SURFACE_HOVER: Color = DARK_GRAY;

pub const C_NOTE_COLOR: Color = Color {
    r: 0.59,
    g: 0.70,
    b: 0.30,
    a: 1.0,
};

// blues :'Color{r:}
pub const BLUE: Color = Color {
    r: 0.10,
    g: 0.15,
    b: 0.70,
    a: 1.0,
}; // desaturated, medium

pub const DARK_BLUE: Color = Color {
    r: 0.06,
    g: 0.09,
    b: 0.45,
    a: 1.0,
}; // darker but not black

pub const NAVY: Color = Color {
    r: 0.0,
    g: 0.0,
    b: 0.1,
    a: 1.0,
}; // darker but not black

pub const ORANGE: Color = Color {
    r: 0.99,
    g: 0.1,
    b: 0.0,
    a: 1.0,
};

pub const GREEN: Color = Color {
    r: 0.1,
    g: 0.99,
    b: 0.1,
    a: 1.0,
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn palette_roundtrip() {
        let original = Palette::new(
            Color {
                r: 0.2,
                g: 0.4,
                b: 0.8,
                a: 1.0,
            }, // primary
            Color {
                r: 0.6,
                g: 0.2,
                b: 0.6,
                a: 1.0,
            }, // secondary
            Color {
                r: 0.5,
                g: 0.5,
                b: 0.5,
                a: 1.0,
            }, // neutral
            Color {
                r: 0.1,
                g: 0.7,
                b: 0.2,
                a: 1.0,
            }, // success
            Color {
                r: 0.9,
                g: 0.1,
                b: 0.1,
                a: 1.0,
            }, // danger
        );

        let path = "test_output/theme_test.toml";
        original.save_to_toml(path);

        let loaded = Palette::load_from_toml(path).expect("failed to load palette back");

        // spot-check a few shades across ramps
        for i in 0..8 {
            assert_eq!(original.primary.shade(i).r, loaded.primary.shade(i).r);
            assert_eq!(original.primary.shade(i).g, loaded.primary.shade(i).g);
            assert_eq!(original.primary.shade(i).b, loaded.primary.shade(i).b);
            assert_eq!(original.danger.shade(i).a, loaded.danger.shade(i).a);
        }

        // std::fs::remove_file(path).ok();
    }
}
