use crate::graphics::{color::*, font::TextItem};

/// Text editing state.
pub struct RenameState {
    pub target: RenameTarget, // which component is being renamed
    pub edited_name: String,  // what the name is after editing (live buffer)
    pub cursor: usize,        // position they are editing at, as a byte/char index into edited_name
}

/// What type of ui component is having text edited
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum RenameTarget {
    Track(usize),
    Pattern(usize),
}

pub const PAD_64: f32 = 64.0;
pub const PAD_32: f32 = 32.0;
pub const PAD_16: f32 = 16.0;
pub const PAD_8: f32 = 8.0;
pub const PAD_4: f32 = 4.0;
pub const PAD_2: f32 = 2.0;

pub const NO_RADIUS: [f32; 4] = [0.0; 4];
// pub const TOP_RADIUS_16: [f32; 4] = [16.0, 0.0, 16.0, 0.0];
pub const BOTTOM_RADIUS_16: [f32; 4] = [0.0, 16.0, 0.0, 16.0];
pub const RADIUS_8: [f32; 4] = [8.0; 4];
pub const RADIUS_4: [f32; 4] = [4.0; 4];
pub const BUTTON_GAP: f32 = 24.0;

pub const ONE_MEGABYTE: u64 = 1024 * 1024;

/// Stores the width and height of the user's application window
pub struct ScreenConfig {
    pub width: u32,
    pub height: u32,
}

/// Stores the vertices and texts of one region of paint
pub struct DrawRegion {
    pub vertices: Vec<Vertex>,
    pub text_items: Vec<TextItem>,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
/// The primitive drawing unit that builds triangles to build all shapes on the screen.
pub struct Vertex {
    pub position: [f32; 3],
    pub local_pos: [f32; 2],
    pub half_size: [f32; 2],
    pub radius: [f32; 4],
    pub color: [f32; 3],
    pub uv: [f32; 2],
    pub border_width: f32,      // new
    pub border_color: [f32; 3], // new
}

impl Vertex {
    const ATTRIBS: [wgpu::VertexAttribute; 8] = [
        wgpu::VertexAttribute {
            format: wgpu::VertexFormat::Float32x3,
            offset: 0,
            shader_location: 0,
        },
        wgpu::VertexAttribute {
            format: wgpu::VertexFormat::Float32x2,
            offset: std::mem::offset_of!(Vertex, local_pos) as wgpu::BufferAddress,
            shader_location: 1,
        },
        wgpu::VertexAttribute {
            format: wgpu::VertexFormat::Float32x2,
            offset: std::mem::offset_of!(Vertex, half_size) as wgpu::BufferAddress,
            shader_location: 2,
        },
        wgpu::VertexAttribute {
            format: wgpu::VertexFormat::Float32x4,
            offset: std::mem::offset_of!(Vertex, radius) as wgpu::BufferAddress,
            shader_location: 3,
        },
        wgpu::VertexAttribute {
            format: wgpu::VertexFormat::Float32x3,
            offset: std::mem::offset_of!(Vertex, color) as wgpu::BufferAddress,
            shader_location: 4,
        },
        wgpu::VertexAttribute {
            format: wgpu::VertexFormat::Float32x2,
            offset: std::mem::offset_of!(Vertex, uv) as wgpu::BufferAddress,
            shader_location: 5,
        },
        wgpu::VertexAttribute {
            format: wgpu::VertexFormat::Float32,
            offset: std::mem::offset_of!(Vertex, border_width) as wgpu::BufferAddress,
            shader_location: 6,
        },
        wgpu::VertexAttribute {
            format: wgpu::VertexFormat::Float32x3,
            offset: std::mem::offset_of!(Vertex, border_color) as wgpu::BufferAddress,
            shader_location: 7,
        },
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
/// returns the normalized device x coordinate for the screen
pub fn to_ndc_x(x: f32, screen_config: &ScreenConfig) -> f32 {
    2.0 * (x / screen_config.width as f32) - 1.0
}
/// returns the normalized device y coordinate for the screen
pub fn to_ndc_y(y: f32, screen_config: &ScreenConfig) -> f32 {
    1.0 - (y / screen_config.height as f32) * 2.0
}

pub fn draw_rectangle_bordered(
    x: f32,
    y: f32,
    width: f32,
    height: f32,
    screen_config: &ScreenConfig,
    color: Color,
    corner_radius: [f32; 4],
    border_width: f32,
    border_color: Color,
    vertex_buffer: &mut Vec<Vertex>,
) {
    let ndc_x = to_ndc_x(x, screen_config);
    let ndc_y = to_ndc_y(y, screen_config);
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
    let ndc_border = ndc_r(border_width);
    let bc = [border_color.r, border_color.g, border_color.b];

    vertex_buffer.push(Vertex {
        position: [ndc_x, ndc_y, 0.0],
        color: [color.r, color.g, color.b],
        uv: [-1.0, -1.0],
        radius,
        half_size: [hw, hh],
        local_pos: [-hw, hh],
        border_width: ndc_border,
        border_color: bc,
    });
    vertex_buffer.push(Vertex {
        position: [ndc_x, ndc_y - ndc_height, 0.0],
        color: [color.r, color.g, color.b],
        uv: [-1.0, -1.0],
        radius,
        half_size: [hw, hh],
        local_pos: [-hw, -hh],
        border_width: ndc_border,
        border_color: bc,
    });
    vertex_buffer.push(Vertex {
        position: [ndc_x + ndc_width, ndc_y, 0.0],
        color: [color.r, color.g, color.b],
        uv: [-1.0, -1.0],
        radius,
        half_size: [hw, hh],
        local_pos: [hw, hh],
        border_width: ndc_border,
        border_color: bc,
    });
    vertex_buffer.push(Vertex {
        position: [ndc_x + ndc_width, ndc_y, 0.0],
        color: [color.r, color.g, color.b],
        uv: [-1.0, -1.0],
        radius,
        half_size: [hw, hh],
        local_pos: [hw, hh],
        border_width: ndc_border,
        border_color: bc,
    });
    vertex_buffer.push(Vertex {
        position: [ndc_x, ndc_y - ndc_height, 0.0],
        color: [color.r, color.g, color.b],
        uv: [-1.0, -1.0],
        radius,
        half_size: [hw, hh],
        local_pos: [-hw, -hh],
        border_width: ndc_border,
        border_color: bc,
    });
    vertex_buffer.push(Vertex {
        position: [ndc_x + ndc_width, ndc_y - ndc_height, 0.0],
        color: [color.r, color.g, color.b],
        uv: [-1.0, -1.0],
        radius,
        half_size: [hw, hh],
        local_pos: [hw, -hh],
        border_width: ndc_border,
        border_color: bc,
    });
}

/// Draws one rectangle to the vertex buffer. Currently customizations include color and corner_radius.
pub fn draw_rectangle(
    x: f32,
    y: f32,
    width: f32,
    height: f32,
    screen_config: &ScreenConfig,
    color: Color,
    corner_radius: [f32; 4],
    vertex_buffer: &mut Vec<Vertex>,
) {
    draw_rectangle_bordered(
        x,
        y,
        width,
        height,
        screen_config,
        color,
        corner_radius,
        0.0,
        BLACK,
        vertex_buffer,
    );
}

/// Draws one circle to the vertex buffer
pub fn draw_circle(
    cx: f32,
    cy: f32,
    radius: f32,
    segments: u32,
    screen_config: &ScreenConfig,
    color: Color,
    vertex_buffer: &mut Vec<Vertex>,
) {
    let to_ndc = |x: f32, y: f32| -> [f32; 3] {
        [
            2.0 * (x / screen_config.width as f32) - 1.0,
            1.0 - (y / screen_config.height as f32) * 2.0,
            0.0,
        ]
    };

    let inert_v = |x: f32, y: f32| Vertex {
        position: to_ndc(x, y),
        color: [color.r, color.g, color.b],
        uv: [-1.0, -1.0],
        radius: [0.0; 4],
        half_size: [1.0, 1.0],
        local_pos: [0.0, 0.0],
        border_width: 0.0,
        border_color: [0.0, 0.0, 0.0],
    };

    for k in 0..segments {
        let a0 = k as f32 * (2.0 * std::f32::consts::PI / segments as f32);
        let a1 = (k + 1) as f32 * (2.0 * std::f32::consts::PI / segments as f32);
        vertex_buffer.push(inert_v(cx, cy));
        vertex_buffer.push(inert_v(cx + radius * a0.cos(), cy + radius * a0.sin()));
        vertex_buffer.push(inert_v(cx + radius * a1.cos(), cy + radius * a1.sin()));
    }
}

/// Draws one volume knob to the vertex buffer
pub fn draw_knob(
    cx: f32,
    cy: f32,
    vol: f32,
    screen_config: &ScreenConfig,
    vertex_buffer: &mut Vec<Vertex>,
) {
    let radius = 10.0_f32;
    draw_circle(cx, cy, radius, 32, screen_config, LL_GRAY, vertex_buffer);

    let ncx = |x: f32| 2.0 * (x / screen_config.width as f32) - 1.0;
    let ncy = |y: f32| 1.0 - (y / screen_config.height as f32) * 2.0;
    let to_rad = |deg: f32| deg * std::f32::consts::PI / 180.0;

    let angle = to_rad(210.0 - vol * 270.0);
    let ex = cx + (radius - 2.0) * angle.cos();
    let ey = cy - (radius - 2.0) * angle.sin();
    let thickness = 1.5;

    let v = |x: f32, y: f32| Vertex {
        position: [ncx(x), ncy(y), 0.0],
        color: [1.0, 1.0, 1.0],
        uv: [-1.0, -1.0],
        radius: [0.0; 4],
        half_size: [1.0, 1.0],
        local_pos: [0.0, 0.0],
        border_width: 0.0,
        border_color: [0.0, 0.0, 0.0],
    };

    let perp_x = -angle.sin();
    let perp_y = -angle.cos();

    let p0 = (cx + thickness * perp_x, cy + thickness * perp_y);
    let p1 = (cx - thickness * perp_x, cy - thickness * perp_y);
    let p2 = (ex + thickness * perp_x, ey + thickness * perp_y);
    let p3 = (ex - thickness * perp_x, ey - thickness * perp_y);

    // always wind counter-clockwise
    vertex_buffer.push(v(p0.0, p0.1));
    vertex_buffer.push(v(p2.0, p2.1));
    vertex_buffer.push(v(p1.0, p1.1));
    vertex_buffer.push(v(p1.0, p1.1));
    vertex_buffer.push(v(p2.0, p2.1));
    vertex_buffer.push(v(p3.0, p3.1));
}

/// draw a horizontal line across the entire screen
pub fn draw_h_line(
    y: f32,
    thickness: f32,
    screen_config: &ScreenConfig,
    vertex_buffer: &mut Vec<Vertex>,
) {
    // top edge of the line
    let ndc_y = 1.0 - (y / screen_config.height as f32) * 2.0;
    // thickness of the line
    let ndc_t = (thickness / screen_config.height as f32) * 2.0;

    let v = |px: f32, py: f32| Vertex {
        position: [px, py, 0.0],
        color: [0.0, 0.0, 0.0],
        uv: [-1.0, -1.0],
        radius: [0.0; 4],
        half_size: [1.0, 1.0],
        local_pos: [0.0, 0.0],
        border_width: 0.0,
        border_color: [0.0, 0.0, 0.0],
    };

    vertex_buffer.extend([
        v(-1.0, ndc_y),
        v(1.0, ndc_y),
        v(1.0, ndc_y - ndc_t),
        v(-1.0, ndc_y),
        v(1.0, ndc_y - ndc_t),
        v(-1.0, ndc_y - ndc_t),
    ]);
}
