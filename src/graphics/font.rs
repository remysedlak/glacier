use crate::graphics::primitives::{ScreenConfig, Vertex, NO_RADIUS};
use std::collections::HashMap;

pub struct TextItem {
    pub text: String,
    pub size: f32,
    pub color: (f32, f32, f32),
    pub font: &'static str,
    pub x: f32,
    pub y: f32,
}

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

/// returns the glyph cache (a hashmap of characters to bitmaps for one size)
pub fn build_glyph_cache(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    font: &fontdue::Font,
    sizes: &[f32],
) -> HashMap<(char, u32), (wgpu::Texture, wgpu::BindGroup, fontdue::Metrics)> {
    let mut cache = HashMap::new();
    for &size in sizes {
        for c in ' '..='~' {
            let (metrics, _) = font.rasterize(c, size);
            if metrics.width == 0 || metrics.height == 0 {
                continue;
            }
            let (texture, bind_group, _, _, _) = rasterize_glyph(device, queue, font, c, size);
            let (metrics, _) = font.rasterize(c, size);
            cache.insert((c, size as u32), (texture, bind_group, metrics));
        }
    }
    cache
}

/// convert the font glyph into wgpu renderable pixels
pub fn rasterize_glyph(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    font: &fontdue::Font,
    c: char,
    size: f32,
) -> (wgpu::Texture, wgpu::BindGroup, wgpu::BindGroupLayout, u32, u32) {
    let (metrics, bitmap) = font.rasterize(c, size);

    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: None,
        size: wgpu::Extent3d {
            width: metrics.width as u32,
            height: metrics.height as u32,
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
        &bitmap,
        wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(metrics.width as u32),
            rows_per_image: None,
        },
        wgpu::Extent3d {
            width: metrics.width as u32,
            height: metrics.height as u32,
            depth_or_array_layers: 1,
        },
    );

    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
    let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
        mag_filter: wgpu::FilterMode::Nearest,
        min_filter: wgpu::FilterMode::Nearest,
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

    (texture, bind_group, bgl, metrics.width as u32, metrics.height as u32)
}

// return vec of vertex building the font letter
pub fn draw_glyph(x: f32, y: f32, w: f32, h: f32, screen_config: &ScreenConfig, color: (f32, f32, f32)) -> Vec<Vertex> {
    let ndc_x = 2.0 * (x / screen_config.width as f32) - 1.0;
    let ndc_y = 1.0 - (y / screen_config.height as f32) * 2.0;
    let ndc_w = (w / screen_config.width as f32) * 2.0;
    let ndc_h = (h / screen_config.height as f32) * 2.0;
    let color = [color.0, color.1, color.2];
    vec![
        Vertex {
            position: [ndc_x, ndc_y, 0.0],
            color,
            uv: [0.0, 0.0],
            local_pos: [0.0, 0.0],
            half_size: [0.0, 0.0],
            radius: NO_RADIUS,
        },
        Vertex {
            position: [ndc_x, ndc_y - ndc_h, 0.0],
            color,
            uv: [0.0, 1.0],
            local_pos: [0.0, 0.0],
            half_size: [0.0, 0.0],
            radius: NO_RADIUS,
        },
        Vertex {
            position: [ndc_x + ndc_w, ndc_y, 0.0],
            color,
            uv: [1.0, 0.0],
            local_pos: [0.0, 0.0],
            half_size: [0.0, 0.0],
            radius: NO_RADIUS,
        },
        Vertex {
            position: [ndc_x + ndc_w, ndc_y, 0.0],
            color,
            uv: [1.0, 0.0],
            local_pos: [0.0, 0.0],
            half_size: [0.0, 0.0],
            radius: NO_RADIUS,
        },
        Vertex {
            position: [ndc_x, ndc_y - ndc_h, 0.0],
            color,
            uv: [0.0, 1.0],
            local_pos: [0.0, 0.0],
            half_size: [0.0, 0.0],
            radius: NO_RADIUS,
        },
        Vertex {
            position: [ndc_x + ndc_w, ndc_y - ndc_h, 0.0],
            color,
            uv: [1.0, 1.0],
            local_pos: [0.0, 0.0],
            half_size: [0.0, 0.0],
            radius: NO_RADIUS,
        },
    ]
}
