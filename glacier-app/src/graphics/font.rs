//! Fonts are loaded to wgpu with fontdue

use crate::graphics::{
    color::Color,
    primitives::{ScreenConfig, Vertex, NO_RADIUS},
};
use fontdue::layout::{CoordinateSystem, Layout, TextStyle};
use std::collections::HashMap;

/// Font enum represents all of the possible fonts for glacier-app
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Font {
    Roboto,
    Mono,
}

/// text sizes hardcoded as needed
pub const FONT_SIZES: [f32; 9] = [6.0, 8.0, 10.0, 12.0, 14.0, 16.0, 18.0, 24.0, 32.0];

impl Font {
    pub const ALL: [Font; 2] = [Font::Roboto, Font::Mono];

    // load a font file as a reference to a byte array
    fn bytes(&self) -> &'static [u8] {
        match self {
            Font::Roboto => {
                include_bytes!("../../../assets/fonts/Roboto-VariableFont_wdth,wght.ttf")
            }
            Font::Mono => include_bytes!("../../../assets/fonts/IBMPlexMono-Regular.ttf"),
        }
    }
}

/// loads all fonts into a font cache and glyph cache
pub fn load_fonts(device: &wgpu::Device, queue: &wgpu::Queue) -> (FontCache, GlyphCache) {
    // build caches
    let mut font_cache = FontCache::new();
    let mut glyph_entries = HashMap::new();

    // reuse bgl and sampler for all fonts/sizes
    let bgl = create_bind_group_layout(device);
    let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
        mag_filter: wgpu::FilterMode::Nearest,
        min_filter: wgpu::FilterMode::Nearest,
        ..Default::default()
    });

    // for each font,
    // - cache the font data to FontCache
    // - cache multiple sizes of the font bitmaps
    for font in Font::ALL {
        let parsed =
            fontdue::Font::from_bytes(font.bytes(), fontdue::FontSettings::default()).unwrap();

        let cache = rasterize_font_glyphs(&parsed, &FONT_SIZES);
        glyph_entries.extend(
            cache
                .into_iter()
                .map(|(key, value)| ((font, key.0, key.1), value)),
        );
        font_cache.insert(font, parsed);
    }
    let glyph_cache = build_glyph_atlas(device, queue, glyph_entries, &bgl, &sampler);
    (font_cache, glyph_cache)
}

/// A rasterized glyph's atlas coordinates and fontdue layout metrics.
pub struct GlyphEntry {
    pub uv_min: [f32; 2],
    pub uv_max: [f32; 2],
    pub metrics: fontdue::Metrics,
}

pub struct GlyphAtlas {
    // Keep the texture alive for the view/bind group that references it.
    _texture: wgpu::Texture,
    bind_group: wgpu::BindGroup,
}

/// Store a cache of Font enums mapped to rendered fonts
pub struct FontCache(HashMap<Font, fontdue::Font>);

impl FontCache {
    pub fn new() -> Self {
        FontCache(HashMap::new())
    }

    pub fn insert(&mut self, font: Font, parsed: fontdue::Font) {
        self.0.insert(font, parsed);
    }

    pub fn get(&self, font: Font) -> Option<&fontdue::Font> {
        self.0.get(&font)
    }
}

impl Default for FontCache {
    fn default() -> Self {
        Self::new()
    }
}

/// Pre-rasterized glyphs for every loaded font, keyed by (font, character,
/// size). Built once at startup into a shared atlas; `Graphics::draw` looks
/// glyphs up here every frame instead of rasterizing on the fly.
pub struct GlyphCache {
    atlas: GlyphAtlas,
    entries: HashMap<(Font, char, u32), GlyphEntry>,
}

impl GlyphCache {
    // get a cached font
    pub fn get(&self, font: Font, ch: char, size: u32) -> Option<&GlyphEntry> {
        self.entries.get(&(font, ch, size))
    }

    // return bind group
    pub fn any_bind_group(&self) -> Option<&wgpu::BindGroup> {
        Some(&self.atlas.bind_group)
    }
}

/// returns the float width in pixels of a character for a font
pub fn cursor_x_offset(font: &fontdue::Font, text: &str, cursor: usize, size: f32) -> f32 {
    // computes where every individual glyph should be positioned, handling all the messy details of turning characters into placed positions
    let mut layout = Layout::new(CoordinateSystem::PositiveYDown);
    layout.append(&[font], &TextStyle::new(text, size, 0));
    let glyphs = layout.glyphs();

    match glyphs.get(cursor) {
        // cursor sits before this glyph — use its actual pen-start x
        Some(g) => g.x,
        // cursor is at the very end — no glyph there, so use the last
        // glyph's x + its advance (need metrics for the advance)
        None => glyphs.last().map_or(0.0, |g| {
            let m = font.metrics(text.chars().last().unwrap(), size);
            g.x + m.advance_width
        }),
    }
}

/// A text item stores the text visual information for wgpu to later draw as vertices
pub struct TextItem {
    pub text: String,
    pub size: f32,
    pub color: Color,
    pub font: Font,
    pub x: f32,
    pub y: f32,
}

/// Shortens text to a certain length for visual purposes. E.g.: Piano_Key_A6.wav -> Piano_Ke...
pub fn truncate_text(name: &str, length: usize) -> String {
    let track_button_text: String = if name.len() > length {
        let end = name.floor_char_boundary(length);
        let truncated_name = &name[..end].to_string(); // Safely gets "こん" (6 bytes)
        format!("{}{}", truncated_name, "...",)
    } else {
        name.to_string()
    };
    track_button_text
}

pub const TITLE: f32 = 18.0;
pub const BODY: f32 = 12.0;

/// The bind group layout shared by the glyph and icon atlases: one filterable
/// 2D texture (binding 0) plus one filtering sampler (binding 1).
pub fn create_bind_group_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: None,
        entries: &[
            wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    view_dimension: wgpu::TextureViewDimension::D2,
                    multisampled: false,
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 1,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                count: None,
            },
        ],
    })
}

/// returns the glyph cache (a hashmap of characters to bitmaps for multiple size)
fn rasterize_font_glyphs(
    font: &fontdue::Font,
    sizes: &[f32],
) -> HashMap<(char, u32), (fontdue::Metrics, Vec<u8>)> {
    let mut cache = HashMap::new();
    for &size in sizes {
        for c in ' '..='~' {
            let (metrics, bitmap) = font.rasterize(c, size);
            if metrics.width == 0 || metrics.height == 0 {
                continue;
            }
            cache.insert((c, size as u32), (metrics, bitmap));
        }
    }
    cache
}

fn build_glyph_atlas(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    rasters: HashMap<(Font, char, u32), (fontdue::Metrics, Vec<u8>)>,
    bgl: &wgpu::BindGroupLayout,
    sampler: &wgpu::Sampler,
) -> GlyphCache {
    const WIDTH: u32 = 1024;
    const PAD: u32 = 1;
    let mut atlas_h = 1;
    let mut placements = Vec::with_capacity(rasters.len());
    let (mut x, mut y, mut row_h) = (PAD, PAD, 0u32);
    for (key, (metrics, bitmap)) in rasters {
        let (w, h) = (metrics.width as u32, metrics.height as u32);
        if x + w + PAD > WIDTH {
            x = PAD;
            y += row_h + PAD;
            row_h = 0;
        }
        assert!(w + 2 * PAD <= WIDTH, "glyph wider than glyph atlas");
        placements.push((key, metrics, bitmap, x, y));
        x += w + PAD;
        row_h = row_h.max(h);
        atlas_h = atlas_h.max(y + h + PAD);
    }
    let height = atlas_h.next_power_of_two();
    let mut pixels = vec![0u8; (WIDTH * height) as usize];
    let mut entries = HashMap::with_capacity(placements.len());
    for (key, metrics, bitmap, x, y) in placements {
        let w = metrics.width as u32;
        let h = metrics.height as u32;
        for row in 0..h {
            let src = (row * w) as usize..((row + 1) * w) as usize;
            let dst = ((y + row) * WIDTH + x) as usize;
            pixels[dst..dst + w as usize].copy_from_slice(&bitmap[src]);
        }
        entries.insert(
            key,
            GlyphEntry {
                uv_min: [
                    (x as f32 + 0.5) / WIDTH as f32,
                    (y as f32 + 0.5) / height as f32,
                ],
                uv_max: [
                    (x as f32 + w as f32 - 0.5) / WIDTH as f32,
                    (y as f32 + h as f32 - 0.5) / height as f32,
                ],
                metrics,
            },
        );
    }
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("Glyph Atlas"),
        size: wgpu::Extent3d {
            width: WIDTH,
            height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::R8Unorm,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    });
    queue.write_texture(
        texture.as_image_copy(),
        &pixels,
        wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(WIDTH),
            rows_per_image: None,
        },
        wgpu::Extent3d {
            width: WIDTH,
            height,
            depth_or_array_layers: 1,
        },
    );
    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
    let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("Glyph Atlas Bind Group"),
        layout: bgl,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(&view),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::Sampler(sampler),
            },
        ],
    });
    GlyphCache {
        atlas: GlyphAtlas {
            _texture: texture,
            bind_group,
        },
        entries,
    }
}

/// Build vertices for one glyph quad at the given screen position, sized to
/// (w, h) and tinted `color`. Two triangles (6 vertices), UV-mapped 0..1
/// so the fragment shader samples the glyph's rasterized texture directly.
pub fn draw_glyph(
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    uv_min: [f32; 2],
    uv_max: [f32; 2],
    screen_config: &ScreenConfig,
    color: (f32, f32, f32, f32),
) -> Vec<Vertex> {
    let ndc_x = 2.0 * (x / screen_config.width as f32) - 1.0;
    let ndc_y = 1.0 - (y / screen_config.height as f32) * 2.0;
    let ndc_w = (w / screen_config.width as f32) * 2.0;
    let ndc_h = (h / screen_config.height as f32) * 2.0;
    let color = [color.0, color.1, color.2, color.3];
    vec![
        Vertex {
            position: [ndc_x, ndc_y, 0.0],
            color,
            uv: uv_min,
            local_pos: [0.0, 0.0],
            half_size: [0.0, 0.0],
            radius: NO_RADIUS,
            border_width: 0.0,
            border_color: [0.0, 0.0, 0.0, 1.0],
        },
        Vertex {
            position: [ndc_x, ndc_y - ndc_h, 0.0],
            color,
            uv: [uv_min[0], uv_max[1]],
            local_pos: [0.0, 0.0],
            half_size: [0.0, 0.0],
            radius: NO_RADIUS,
            border_width: 0.0,
            border_color: [0.0, 0.0, 0.0, 1.0],
        },
        Vertex {
            position: [ndc_x + ndc_w, ndc_y, 0.0],
            color,
            uv: [uv_max[0], uv_min[1]],
            local_pos: [0.0, 0.0],
            half_size: [0.0, 0.0],
            radius: NO_RADIUS,
            border_width: 0.0,
            border_color: [0.0, 0.0, 0.0, 1.0],
        },
        Vertex {
            position: [ndc_x + ndc_w, ndc_y, 0.0],
            color,
            uv: [uv_max[0], uv_min[1]],
            local_pos: [0.0, 0.0],
            half_size: [0.0, 0.0],
            radius: NO_RADIUS,
            border_width: 0.0,
            border_color: [0.0, 0.0, 0.0, 1.0],
        },
        Vertex {
            position: [ndc_x, ndc_y - ndc_h, 0.0],
            color,
            uv: [uv_min[0], uv_max[1]],
            local_pos: [0.0, 0.0],
            half_size: [0.0, 0.0],
            radius: NO_RADIUS,
            border_width: 0.0,
            border_color: [0.0, 0.0, 0.0, 1.0],
        },
        Vertex {
            position: [ndc_x + ndc_w, ndc_y - ndc_h, 0.0],
            color,
            uv: uv_max,
            local_pos: [0.0, 0.0],
            half_size: [0.0, 0.0],
            radius: NO_RADIUS,
            border_width: 0.0,
            border_color: [0.0, 0.0, 0.0, 1.0],
        },
    ]
}
