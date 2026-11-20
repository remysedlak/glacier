//! Icons are loaded from svg

use crate::graphics::{
    icons,
    primitives::{ScreenConfig, Vertex, NO_RADIUS},
    DrawItem,
};
use std::collections::HashMap;

/// List of SVG file names to be loaded into memory
/// (svg file name, svg width, svg height)
pub const ICON_SOURCES: &[(&str, u32, u32)] = &[
    ("play", 128, 128),
    ("stop", 128, 128),
    ("pause", 128, 128),
    ("mixer", 128, 128),
    ("sequencer", 128, 128),
    ("settings", 128, 128),
    ("playlist", 128, 128),
    ("track", 128, 128),
    ("project", 128, 128),
    ("piano", 128, 128),
    ("track_tray", 128, 128),
    ("pattern_tray", 128, 128),
    ("file", 32, 32),
    ("music_dir", 32, 32),
    ("music_file", 32, 32),
    ("add", 32, 32),
    ("left_sidepanel", 32, 32),
    ("bpm_up", 32, 12),
    ("bpm_down", 32, 12),
    ("Mute", 24, 24),
    ("PlaylistPaint", 24, 24),
    ("PlaylistRectangle", 24, 24),
    ("PlaylistSelect", 24, 24),
];

pub const ICON_SIZES: &[(&str, u32, u32)] = &[
    ("play", 32, 32),
    ("stop", 32, 32),
    ("pause", 32, 32),
    ("mixer", 32, 32),
    ("sequencer", 32, 32),
    ("settings", 32, 32),
    ("playlist", 32, 32),
    ("track", 32, 32),
    ("project", 32, 32),
    ("piano", 32, 32),
    ("track_tray", 32, 32),
    ("pattern_tray", 32, 32),
    ("file", 32, 32),
    ("music_dir", 20, 20),
    ("music_dir", 16, 16),
    ("music_file", 16, 16),
    ("add", 20, 20),
    ("left_sidepanel", 20, 20),
    ("bpm_up", 32, 12),
    ("bpm_down", 32, 12),
    ("Mute", 24, 24),
    ("PlaylistPaint", 24, 24),
    ("PlaylistRectangle", 24, 24),
    ("PlaylistSelect", 24, 24),
];

/// Icon File Wrapper. path and size.
pub struct IconSvg {
    pub width: f32,
    pub height: f32,
    pub path: String,
}

#[derive(Clone, PartialEq)]
/// A hover label anchored at (x, y); text: None means no tooltip shown.
pub struct Tooltip {
    pub text: Option<String>,
    pub x: f32,
    pub y: f32,
    pub width: f32,
}
pub const DEFAULT_TOOLTIP_WIDTH: f32 = 128.0;

/// Pre-loaded app Icon with Tooltip defined
pub struct IconDraw {
    pub name: &'static str,
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub tooltip: Tooltip,
}
impl IconDraw {
    /// if an icon is hovered.
    pub fn is_hovered(&self, mx: f32, my: f32) -> bool {
        mx > self.x && mx < self.x + self.width && my > self.y && my < self.y + self.height
    }
}

pub struct IconEntry {
    pub uv_min: [f32; 2],
    pub uv_max: [f32; 2],
}

/// Shared GPU image for all pre-rasterized icons plus CPU-side atlas locations.
pub struct IconCache {
    _texture: wgpu::Texture,
    bind_group: wgpu::BindGroup,
    entries: HashMap<(String, u32, u32), IconEntry>,
}
impl IconCache {
    pub(crate) fn bind_group(&self) -> &wgpu::BindGroup {
        &self.bind_group
    }
}

/// Pushes an icon's quad into the shared icon vertex buffer and records
/// its (offset, bind_group) so the render pass knows which texture to use.
pub(crate) fn push_icon_draw(
    icon_cache: &IconCache,
    screen_config: &ScreenConfig,
    icon: &IconDraw,
    icon_vertices: &mut Vec<Vertex>,
    icon_draws: &mut Vec<DrawItem>,
) {
    if let Some(entry) =
        icon_cache
            .entries
            .get(&(icon.name.to_string(), icon.width as u32, icon.height as u32))
    {
        let verts = icons::draw_icon(
            icon.x,
            icon.y,
            icon.width,
            icon.height,
            entry.uv_min,
            entry.uv_max,
            screen_config,
        );
        let offset = (icon_vertices.len() * std::mem::size_of::<Vertex>()) as u64;
        icon_vertices.extend_from_slice(&verts);
        icon_draws.push(DrawItem {
            vertex_offset: offset,
        });
    }
}

/// pre-rasterize all SVG files to bitmaps of all sizes needed
pub fn load_icons(device: &wgpu::Device, queue: &wgpu::Queue) -> IconCache {
    let bgl = create_icon_bind_group_layout(device);
    let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
        mag_filter: wgpu::FilterMode::Linear,
        min_filter: wgpu::FilterMode::Linear,
        ..Default::default()
    });

    let mut rasters = Vec::new();
    for (name, target_w, target_h) in icons::ICON_SIZES {
        let (_, src_w, src_h) = icons::ICON_SOURCES.iter().find(|s| s.0 == *name).unwrap();
        let svg_str =
            std::fs::read_to_string(format!("assets/icons/{}x{}/{}.svg", src_w, src_h, name))
                .unwrap_or_else(|e| panic!("failed to load icon {name}: {e}"));
        let svg = icons::IconSvg {
            width: *target_w as f32,
            height: *target_h as f32,
            path: svg_str,
        };
        let rgba = icons::rasterize_icon(svg);
        rasters.push(((name.to_string(), *target_w, *target_h), rgba));
    }
    const WIDTH: u32 = 512;
    const PAD: u32 = 1;
    let (mut x, mut y, mut row_h) = (PAD, PAD, 0u32);
    let mut atlas_h = 1u32;
    let mut placements = Vec::with_capacity(rasters.len());
    for (key, rgba) in rasters {
        let (w, h) = (key.1, key.2);
        if x + w + PAD > WIDTH {
            x = PAD;
            y += row_h + PAD;
            row_h = 0;
        }
        assert!(w + 2 * PAD <= WIDTH, "icon wider than icon atlas");
        placements.push((key, rgba, x, y));
        x += w + PAD;
        row_h = row_h.max(h);
        atlas_h = atlas_h.max(y + h + PAD);
    }
    let height = atlas_h.next_power_of_two();
    let mut pixels = vec![0u8; (WIDTH * height * 4) as usize];
    let mut entries = HashMap::with_capacity(placements.len());
    for (key, rgba, x, y) in placements {
        let (w, h) = (key.1, key.2);
        for row in 0..h {
            let src = (row * w * 4) as usize..((row + 1) * w * 4) as usize;
            let dst = ((y + row) * WIDTH * 4 + x * 4) as usize;
            pixels[dst..dst + (w * 4) as usize].copy_from_slice(&rgba[src]);
        }
        entries.insert(
            key,
            IconEntry {
                uv_min: [
                    (x as f32 + 0.5) / WIDTH as f32,
                    (y as f32 + 0.5) / height as f32,
                ],
                uv_max: [
                    (x as f32 + w as f32 - 0.5) / WIDTH as f32,
                    (y as f32 + h as f32 - 0.5) / height as f32,
                ],
            },
        );
    }
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("Icon Atlas"),
        size: wgpu::Extent3d {
            width: WIDTH,
            height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8UnormSrgb,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    });
    queue.write_texture(
        texture.as_image_copy(),
        &pixels,
        wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(WIDTH * 4),
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
        label: Some("Icon Atlas Bind Group"),
        layout: &bgl,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(&view),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::Sampler(&sampler),
            },
        ],
    });
    IconCache {
        _texture: texture,
        bind_group,
        entries,
    }
}

fn create_icon_bind_group_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("Icon BGL"),
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

/// tiny_skia's Pixmap stores premultiplied RGBA (RGB channels pre-scaled by
/// alpha). Our render pipeline's blend state expects straight (non-
/// premultiplied) alpha, same as the glyph/geometry quads. Divide RGB back
/// out by alpha so the two conventions agree once this hits the GPU blend
/// equation.
fn unpremultiply(bytes: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(bytes.len());
    for px in bytes.as_chunks::<4>().0 {
        let (r, g, b, a) = (px[0], px[1], px[2], px[3]);
        if a == 0 {
            out.extend_from_slice(&[0, 0, 0, 0]);
        } else {
            let a_f = a as f32;
            let unmul = |c: u8| ((c as f32 * 255.0) / a_f).round().clamp(0.0, 255.0) as u8;
            out.extend_from_slice(&[unmul(r), unmul(g), unmul(b), a]);
        }
    }
    out
}

/// Rasterize an SVG to straight-alpha RGBA pixels for insertion into the icon atlas.
pub fn rasterize_icon(icon: IconSvg) -> Vec<u8> {
    let tree = resvg::usvg::Tree::from_str(&icon.path, &Default::default()).unwrap();
    let mut pixmap = resvg::tiny_skia::Pixmap::new(icon.width as u32, icon.height as u32).unwrap();
    let tree_size = tree.size(); // usvg::Size — .width(), .height()
    let transform = resvg::tiny_skia::Transform::from_scale(
        icon.width / tree_size.width(),
        icon.height / tree_size.height(),
    );
    resvg::render(&tree, transform, &mut pixmap.as_mut());

    let rgba_bytes = unpremultiply(pixmap.data());

    rgba_bytes
}

/// return vector of vertices building the icon
pub fn draw_icon(
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    uv_min: [f32; 2],
    uv_max: [f32; 2],
    screen_config: &ScreenConfig,
) -> Vec<Vertex> {
    // force each icon's quad edges to land exactly on pixel boundaries on screen
    // instead of falling between two pixels
    let x = x.round();
    let y = y.round();
    let w = w.round();
    let h = h.round();

    let ndc_x = 2.0 * (x / screen_config.width as f32) - 1.0;
    let ndc_y = 1.0 - (y / screen_config.height as f32) * 2.0;
    let ndc_w = (w / screen_config.width as f32) * 2.0;
    let ndc_h = (h / screen_config.height as f32) * 2.0;

    let color = [1.0, 1.0, 1.0, 1.0];
    vec![
        Vertex {
            position: [ndc_x, ndc_y, 0.0],
            color,
            uv: uv_min,
            local_pos: [1.0, 0.0],
            half_size: [0.0, 0.0],
            radius: NO_RADIUS,
            border_width: 0.0,
            border_color: [0.0, 0.0, 0.0, 1.0],
        },
        Vertex {
            position: [ndc_x, ndc_y - ndc_h, 0.0],
            color,
            uv: [uv_min[0], uv_max[1]],
            local_pos: [1.0, 0.0],
            half_size: [0.0, 0.0],
            radius: NO_RADIUS,
            border_width: 0.0,
            border_color: [0.0, 0.0, 0.0, 1.0],
        },
        Vertex {
            position: [ndc_x + ndc_w, ndc_y, 0.0],
            color,
            uv: [uv_max[0], uv_min[1]],
            local_pos: [1.0, 0.0],
            half_size: [0.0, 0.0],
            radius: NO_RADIUS,
            border_width: 0.0,
            border_color: [0.0, 0.0, 0.0, 1.0],
        },
        Vertex {
            position: [ndc_x + ndc_w, ndc_y, 0.0],
            color,
            uv: [uv_max[0], uv_min[1]],
            local_pos: [1.0, 0.0],
            half_size: [0.0, 0.0],
            radius: NO_RADIUS,
            border_width: 0.0,
            border_color: [0.0, 0.0, 0.0, 1.0],
        },
        Vertex {
            position: [ndc_x, ndc_y - ndc_h, 0.0],
            color,
            uv: [uv_min[0], uv_max[1]],
            local_pos: [1.0, 0.0],
            half_size: [0.0, 0.0],
            radius: NO_RADIUS,
            border_width: 0.0,
            border_color: [0.0, 0.0, 0.0, 1.0],
        },
        Vertex {
            position: [ndc_x + ndc_w, ndc_y - ndc_h, 0.0],
            color,
            uv: uv_max,
            local_pos: [1.0, 0.0],
            half_size: [0.0, 0.0],
            radius: NO_RADIUS,
            border_width: 0.0,
            border_color: [0.0, 0.0, 0.0, 1.0],
        },
    ]
}
