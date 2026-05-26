use std::collections::HashMap;

use wgpu::util::DeviceExt;

use crate::graphics::{
    icons,
    primitives::{ScreenConfig, Vertex, NO_RADIUS},
};

pub const ICON_NAMES_128: &[&str] = &[
    "play",
    "stop",
    "pause",
    "mixer",
    "sequencer",
    "playlist",
    "track",
    "project",
    "piano",
    "track_tray",
    "pattern_tray",
];

pub const ICON_NAMES_32: &[&str] = &["file"];

pub struct IconSvg {
    pub width: f32,
    pub height: f32,
    pub path: String,
}

#[derive(Clone)]
pub struct Tooltip {
    pub text: Option<&'static str>,
    pub x: f32,
    pub y: f32,
}

pub struct IconDraw {
    pub name: &'static str,
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub tooltip: Tooltip,
}
impl IconDraw {
    pub fn is_hovered(&self, mx: f32, my: f32) -> bool {
        mx > self.x && mx < self.x + self.width && my > self.y && my < self.y + self.height
    }
}

/// pushing icons to draw
pub fn push_icon_draw<'a>(
    icon_cache: &'a HashMap<String, (wgpu::Texture, wgpu::BindGroup)>,
    device: &wgpu::Device,
    screen_config: &ScreenConfig,
    name: &str,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    icon_draws: &mut Vec<(wgpu::Buffer, &'a wgpu::BindGroup)>,
) {
    if let Some((_, bind_group)) = icon_cache.get(name) {
        let verts = icons::draw_icon(x, y, w, h, screen_config);
        let buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: None,
            contents: bytemuck::cast_slice(&verts),
            usage: wgpu::BufferUsages::VERTEX,
        });
        icon_draws.push((buf, bind_group));
    }
}

pub fn rasterize_icon(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    icon: IconSvg,
) -> (wgpu::Texture, wgpu::BindGroup, wgpu::BindGroupLayout, u32, u32) {
    let tree = resvg::usvg::Tree::from_str(&icon.path, &Default::default()).unwrap();
    let mut pixmap = resvg::tiny_skia::Pixmap::new(icon.width as u32, icon.height as u32).unwrap();
    resvg::render(&tree, resvg::tiny_skia::Transform::default(), &mut pixmap.as_mut());
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
        &rgba_bytes,
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
    let bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
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

    (texture, bind_group, bgl, icon.width as u32, icon.height as u32)
}

// return vec of vertex building the font letter
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
        },
        Vertex {
            position: [ndc_x, ndc_y - ndc_h, 0.0],
            color,
            uv: [2.0, 1.0],
            local_pos: [0.0, 0.0],
            half_size: [0.0, 0.0],
            radius: NO_RADIUS,
        },
        Vertex {
            position: [ndc_x + ndc_w, ndc_y, 0.0],
            color,
            uv: [3.0, 0.0],
            local_pos: [0.0, 0.0],
            half_size: [0.0, 0.0],
            radius: NO_RADIUS,
        },
        Vertex {
            position: [ndc_x + ndc_w, ndc_y, 0.0],
            color,
            uv: [3.0, 0.0],
            local_pos: [0.0, 0.0],
            half_size: [0.0, 0.0],
            radius: NO_RADIUS,
        },
        Vertex {
            position: [ndc_x, ndc_y - ndc_h, 0.0],
            color,
            uv: [2.0, 1.0],
            local_pos: [0.0, 0.0],
            half_size: [0.0, 0.0],
            radius: NO_RADIUS,
        },
        Vertex {
            position: [ndc_x + ndc_w, ndc_y - ndc_h, 0.0],
            color,
            uv: [3.0, 1.0],
            local_pos: [0.0, 0.0],
            half_size: [0.0, 0.0],
            radius: NO_RADIUS,
        },
    ]
}
