//! Icons are loaded from svg

use std::collections::HashMap;

use wgpu::{util::DeviceExt, BindGroup, BindGroupLayout, Texture};

use crate::graphics::{
    icons,
    primitives::{ScreenConfig, Vertex, NO_RADIUS},
};

pub const ICONS: &[(&str, u32, u32)] = &[
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

/// Pushes an icon's quad into the shared icon vertex buffer and records
/// its (offset, bind_group) so the render pass knows which texture to use.
pub fn push_icon_draw<'a>(
    icon_cache: &'a HashMap<String, (wgpu::Texture, wgpu::BindGroup)>,
    screen_config: &ScreenConfig,
    icon: &IconDraw,
    icon_vertices: &mut Vec<Vertex>,
    icon_draws: &mut Vec<(u64, &'a wgpu::BindGroup)>,
) {
    if let Some((_, bind_group)) = icon_cache.get(icon.name) {
        let verts = icons::draw_icon(icon.x, icon.y, icon.width, icon.height, screen_config);
        let offset = (icon_vertices.len() * std::mem::size_of::<Vertex>()) as u64;
        icon_vertices.extend_from_slice(&verts);
        icon_draws.push((offset, bind_group));
    }
}

pub fn load_icons(
    icon_cache: &mut HashMap<String, (Texture, BindGroup)>,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
) {
    for icon in icons::ICONS {
        let svg_str =
            std::fs::read_to_string(format!("assets/icons/{}x{}/{}.svg", icon.1, icon.2, icon.0))
                .unwrap_or_else(|e| panic!("failed to load icon {}: {e}", icon.0));
        let svg = icons::IconSvg {
            width: icon.1 as f32,
            height: icon.2 as f32,
            path: svg_str,
        };
        let (texture, bind_group, _, _, _) = icons::rasterize_icon(&device, &queue, svg);
        icon_cache.insert(icon.0.to_string(), (texture, bind_group));
    }
}

/// Input's an SVG path and outputs a wgpu texture and icon size information
pub fn rasterize_icon(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    icon: IconSvg,
) -> (
    wgpu::Texture,
    wgpu::BindGroup,
    wgpu::BindGroupLayout,
    u32,
    u32,
) {
    let tree = resvg::usvg::Tree::from_str(&icon.path, &Default::default()).unwrap();
    let mut pixmap = resvg::tiny_skia::Pixmap::new(icon.width as u32, icon.height as u32).unwrap();
    resvg::render(
        &tree,
        resvg::tiny_skia::Transform::default(),
        &mut pixmap.as_mut(),
    );
    let rgba_bytes = pixmap.data();

    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: None,
        size: wgpu::Extent3d {
            width: icon.width as u32,
            height: icon.height as u32,
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
        rgba_bytes,
        wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(icon.width as u32 * 4),
            rows_per_image: None,
        },
        wgpu::Extent3d {
            width: icon.width as u32,
            height: icon.height as u32,
            depth_or_array_layers: 1,
        },
    );

    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
    let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
        mag_filter: wgpu::FilterMode::Linear,
        min_filter: wgpu::FilterMode::Linear,
        ..Default::default()
    });
    let bgl: BindGroupLayout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
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
    });
    let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: None,
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

    (
        texture,
        bind_group,
        bgl,
        icon.width as u32,
        icon.height as u32,
    )
}

/// return vector of vertices building the icon
pub fn draw_icon(x: f32, y: f32, w: f32, h: f32, screen_config: &ScreenConfig) -> Vec<Vertex> {
    let ndc_x = 2.0 * (x / screen_config.width as f32) - 1.0;
    let ndc_y = 1.0 - (y / screen_config.height as f32) * 2.0;
    let ndc_w = (w / screen_config.width as f32) * 2.0;
    let ndc_h = (h / screen_config.height as f32) * 2.0;
    let color = [1.0, 1.0, 1.0];
    vec![
        Vertex {
            position: [ndc_x, ndc_y, 0.0],
            color,
            uv: [2.0, 0.0],
            local_pos: [0.0, 0.0],
            half_size: [0.0, 0.0],
            radius: NO_RADIUS,
            border_width: 0.0,
            border_color: [0.0, 0.0, 0.0],
        },
        Vertex {
            position: [ndc_x, ndc_y - ndc_h, 0.0],
            color,
            uv: [2.0, 1.0],
            local_pos: [0.0, 0.0],
            half_size: [0.0, 0.0],
            radius: NO_RADIUS,
            border_width: 0.0,
            border_color: [0.0, 0.0, 0.0],
        },
        Vertex {
            position: [ndc_x + ndc_w, ndc_y, 0.0],
            color,
            uv: [3.0, 0.0],
            local_pos: [0.0, 0.0],
            half_size: [0.0, 0.0],
            radius: NO_RADIUS,
            border_width: 0.0,
            border_color: [0.0, 0.0, 0.0],
        },
        Vertex {
            position: [ndc_x + ndc_w, ndc_y, 0.0],
            color,
            uv: [3.0, 0.0],
            local_pos: [0.0, 0.0],
            half_size: [0.0, 0.0],
            radius: NO_RADIUS,
            border_width: 0.0,
            border_color: [0.0, 0.0, 0.0],
        },
        Vertex {
            position: [ndc_x, ndc_y - ndc_h, 0.0],
            color,
            uv: [2.0, 1.0],
            local_pos: [0.0, 0.0],
            half_size: [0.0, 0.0],
            radius: NO_RADIUS,
            border_width: 0.0,
            border_color: [0.0, 0.0, 0.0],
        },
        Vertex {
            position: [ndc_x + ndc_w, ndc_y - ndc_h, 0.0],
            color,
            uv: [3.0, 1.0],
            local_pos: [0.0, 0.0],
            half_size: [0.0, 0.0],
            radius: NO_RADIUS,
            border_width: 0.0,
            border_color: [0.0, 0.0, 0.0],
        },
    ]
}
