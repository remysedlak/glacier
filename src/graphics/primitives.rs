use crate::graphics::color::*;

pub const PAD_64: f32 = 64.0;
pub const PAD_32: f32 = 32.0;
pub const PAD_24: f32 = 24.0;
pub const PAD_16: f32 = 16.0;
pub const PAD_8: f32 = 8.0;
pub const PAD_4: f32 = 4.0;
pub const PAD_2: f32 = 2.0;

pub const NO_RADIUS: [f32; 4] = [0.0; 4];
pub const TOP_RADIUS_16: [f32; 4] = [16.0, 0.0, 16.0, 0.0];
pub const BOTTOM_RADIUS_16: [f32; 4] = [0.0, 16.0, 0.0, 16.0];
pub const RADIUS_8: [f32; 4] = [8.0; 4];

pub const BUTTON_GAP: f32 = 24.0;

pub struct ScreenConfig {
    pub width: u32,
    pub height: u32,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Vertex {
    pub position: [f32; 3],
    pub local_pos: [f32; 2],
    pub half_size: [f32; 2],
    pub radius: [f32; 4],
    pub color: [f32; 3],
    pub uv: [f32; 2],
}

impl Vertex {
    const ATTRIBS: [wgpu::VertexAttribute; 6] = wgpu::vertex_attr_array![
        0 => Float32x3,
        1 => Float32x2,
        2 => Float32x2,
        3 => Float32x4,
        4 => Float32x3,
        5 => Float32x2,
    ];

    pub fn desc() -> wgpu::VertexBufferLayout<'static> {
        use std::mem;
        wgpu::VertexBufferLayout {
            array_stride: mem::size_of::<Self>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &Self::ATTRIBS,
        }
    }
}

pub fn draw_rectangle(
    x: f32,
    y: f32,
    width: f32,
    height: f32,
    screen_config: &ScreenConfig,
    (r, g, b): (f32, f32, f32),
    corner_radius: [f32; 4],
) -> Vec<Vertex> {
    let ndc_x = 2.0 * (x / screen_config.width as f32) - 1.0;
    let ndc_y = 1.0 - (y / screen_config.height as f32) * 2.0;
    let ndc_r = |r: f32| (r / screen_config.width as f32) * 2.0;
    let radius = [
        ndc_r(corner_radius[0]),
        ndc_r(corner_radius[1]),
        ndc_r(corner_radius[2]),
        ndc_r(corner_radius[3]),
    ];
    let ndc_width = (width / screen_config.width as f32) * 2.0;
    let ndc_height = (height / screen_config.height as f32) * 2.0;
    let hw = ndc_width / 2.0;
    let hh = ndc_height / 2.0;

    vec![
        Vertex {
            position: [ndc_x, ndc_y, 0.0],
            color: [r, g, b],
            uv: [-1.0, -1.0],
            radius,
            half_size: [hw, hh],
            local_pos: [-hw, hh],
        },
        Vertex {
            position: [ndc_x, ndc_y - ndc_height, 0.0],
            color: [r, g, b],
            uv: [-1.0, -1.0],
            radius,
            half_size: [hw, hh],
            local_pos: [-hw, -hh],
        },
        Vertex {
            position: [ndc_x + ndc_width, ndc_y, 0.0],
            color: [r, g, b],
            uv: [-1.0, -1.0],
            radius,
            half_size: [hw, hh],
            local_pos: [hw, hh],
        },
        Vertex {
            position: [ndc_x + ndc_width, ndc_y, 0.0],
            color: [r, g, b],
            uv: [-1.0, -1.0],
            radius,
            half_size: [hw, hh],
            local_pos: [hw, hh],
        },
        Vertex {
            position: [ndc_x, ndc_y - ndc_height, 0.0],
            color: [r, g, b],
            uv: [-1.0, -1.0],
            radius,
            half_size: [hw, hh],
            local_pos: [-hw, -hh],
        },
        Vertex {
            position: [ndc_x + ndc_width, ndc_y - ndc_height, 0.0],
            color: [r, g, b],
            uv: [-1.0, -1.0],
            radius,
            half_size: [hw, hh],
            local_pos: [hw, -hh],
        },
    ]
}

pub fn draw_circle(cx: f32, cy: f32, radius: f32, segments: u32, screen_config: &ScreenConfig, (r, g, b): (f32, f32, f32)) -> Vec<Vertex> {
    let mut vec: Vec<Vertex> = Vec::new();
    let to_ndc = |x: f32, y: f32| -> [f32; 3] {
        [
            2.0 * (x / screen_config.width as f32) - 1.0,
            1.0 - (y / screen_config.height as f32) * 2.0,
            0.0,
        ]
    };

    let inert_v = |x: f32, y: f32| Vertex {
        position: to_ndc(x, y),
        color: [r, g, b],
        uv: [-1.0, -1.0],
        radius: [0.0; 4],
        half_size: [1.0, 1.0],
        local_pos: [0.0, 0.0],
    };

    for k in 0..segments {
        let a0 = k as f32 * (2.0 * std::f32::consts::PI / segments as f32);
        let a1 = (k + 1) as f32 * (2.0 * std::f32::consts::PI / segments as f32);
        vec.push(inert_v(cx, cy));
        vec.push(inert_v(cx + radius * a0.cos(), cy + radius * a0.sin()));
        vec.push(inert_v(cx + radius * a1.cos(), cy + radius * a1.sin()));
    }
    vec
}
pub fn draw_knob(cx: f32, cy: f32, vol: f32, screen_config: &ScreenConfig) -> Vec<Vertex> {
    let radius = 10.0_f32;
    let mut vec = draw_circle(cx, cy, radius, 32, screen_config, LL_GRAY);

    let ncx = |x: f32| 2.0 * (x / screen_config.width as f32) - 1.0;
    let ncy = |y: f32| 1.0 - (y / screen_config.height as f32) * 2.0;
    let to_rad = |deg: f32| deg * std::f32::consts::PI / 180.0;

    let angle = to_rad(210.0 - vol * 270.0);
    let ex = cx + (radius - 2.0) * angle.cos();
    let ey = cy - (radius - 2.0) * angle.sin();
    let perp = angle + std::f32::consts::FRAC_PI_2;
    let thickness = 1.5;

    let mut v = |x: f32, y: f32| Vertex {
        position: [ncx(x), ncy(y), 0.0],
        color: [1.0, 1.0, 1.0],
        uv: [-1.0, -1.0],
        radius: [0.0; 4],
        half_size: [1.0, 1.0],
        local_pos: [0.0, 0.0],
    };

    let perp_x = -angle.sin();
    let perp_y = -angle.cos();

    let p0 = (cx + thickness * perp_x, cy + thickness * perp_y);
    let p1 = (cx - thickness * perp_x, cy - thickness * perp_y);
    let p2 = (ex + thickness * perp_x, ey + thickness * perp_y);
    let p3 = (ex - thickness * perp_x, ey - thickness * perp_y);

    // always wind counter-clockwise
    vec.push(v(p0.0, p0.1));
    vec.push(v(p2.0, p2.1));
    vec.push(v(p1.0, p1.1));
    vec.push(v(p1.0, p1.1));
    vec.push(v(p2.0, p2.1));
    vec.push(v(p3.0, p3.1));

    vec
}

pub fn draw_h_line(y: f32, thickness: f32, screen_config: &ScreenConfig) -> Vec<Vertex> {
    let ndc_y = 1.0 - (y / screen_config.height as f32) * 2.0;
    let ndc_t = (thickness / screen_config.height as f32) * 2.0;

    let v = |px: f32, py: f32| Vertex {
        position: [px, py, 0.0],
        color: [0.0, 0.0, 0.0],
        uv: [-1.0, -1.0],
        radius: [0.0; 4],
        half_size: [1.0, 1.0],
        local_pos: [0.0, 0.0],
    };

    vec![
        v(-1.0, ndc_y),
        v(1.0, ndc_y),
        v(1.0, ndc_y - ndc_t),
        v(-1.0, ndc_y),
        v(1.0, ndc_y - ndc_t),
        v(-1.0, ndc_y - ndc_t),
    ]
}
