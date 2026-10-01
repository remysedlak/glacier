//! Custom color class

use palette::{IntoColor, Oklch, Srgb};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
/// a color pallete contains all colors used in the DAW
pub struct Palette {
    pub primary: ColorRamp,
    pub secondary: ColorRamp,
    pub neutral: ColorRamp,
}

impl Palette {
    /// Save the pallete to a location on disk
    pub fn new(primary: Color, secondary: Color, neutral: Color) -> Self {
        Palette {
            primary: ColorRamp::new(primary),
            secondary: ColorRamp::new(secondary),
            neutral: ColorRamp::new(neutral),
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
    pub fn hover(&self, i: usize) -> Color {
        self.shades[(i + 1).min(self.shades.len() - 1)]
    }

    pub fn new(color: Color) -> ColorRamp {
        let srgb = Srgb::new(color.r, color.g, color.b);
        let oklch: Oklch = srgb.into_color();

        const L_STEPS: [f32; 8] = [0.09, 0.20, 0.33, 0.45, 0.58, 0.70, 0.83, 0.95];

        let mut shades = [color; 8];
        for i in 0..8 {
            let l = L_STEPS[i];
            // taper chroma near the extremes (0.0 and 1.0) where sRGB gamut narrows
            let taper = 1.0 - (l - 0.5).abs() * 2.0 * 0.4; // 0.4 = max chroma reduction at extremes
            let shade_oklch = Oklch::new(l, oklch.chroma * taper.max(0.3), oklch.hue);
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

impl Default for Color {
    fn default() -> Self {
        Color {
            r: 0.0,
            g: 0.0,
            b: 0.0,
            a: 0.0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn palette_roundtrip() {
        let neutral = Color {
            r: 0.16470588,
            g: 0.17647059,
            b: 0.20392157,
            a: 1.0,
        }; // jet black
        let primary = Color {
            r: 0.0,
            g: 0.61568627,
            b: 0.8627451,
            a: 1.0,
        }; // blue bell
        let secondary = Color {
            r: 0.94901961,
            g: 0.39215686,
            b: 0.18823529,
            a: 1.0,
        }; // tiger flame
        let original = Palette::new(primary, secondary, neutral);

        let path = "../assets/themes/theme_test.toml";
        original.save_to_toml(path);

        let loaded = Palette::load_from_toml(path).expect("failed to load palette back");

        // spot-check a few shades across ramps
        for i in 0..8 {
            assert_eq!(original.primary.shade(i).r, loaded.primary.shade(i).r);
            assert_eq!(original.primary.shade(i).g, loaded.primary.shade(i).g);
            assert_eq!(original.primary.shade(i).b, loaded.primary.shade(i).b);
        }

        // std::fs::remove_file(path).ok();
    }
}
