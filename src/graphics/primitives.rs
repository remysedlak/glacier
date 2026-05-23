use crate::graphics::color::*;

/*
 *  This file contains the most primitive graphics operations, directly manipulating Vertex's.
 *  These methods may be used on their own, or drawn by widgets requiring them.
 */

// padding constants
pub const PAD_64: f32 = 64.0;
pub const PAD_32: f32 = 32.0;
pub const PAD_24: f32 = 24.0;
pub const PAD_16: f32 = 16.0;
pub const PAD_8: f32 = 8.0;
pub const PAD_4: f32 = 4.0;
pub const PAD_2: f32 = 2.0;

pub const NO_RADIUS: [f32; 4] = [0.0, 0.0, 0.0, 0.0];
pub const TOP_RADIUS: [f32; 4] = [16.0, 0.0, 16.0, 0.0];
pub const BOTTOM_RADIUS: [f32; 4] = [0.0, 16.0, 0.0, 16.0];

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
        0 => Float32x3,  // position
        1 => Float32x2,  // local_pos
        2 => Float32x2,  // half_size
        3 => Float32x4,    // radius
        4 => Float32x3,  // color
        5 => Float32x2,  // uv
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

// fn to_ndc_x(x: f32, width: f32) -> f32 {
//     2.0 * (x / width) - 1.0
// }
// fn to_ndc_y(y: f32, height: f32) -> f32 {
//     1.0 - (y / height) * 2.0
// }
//

// draw one rectangle with one color
pub fn draw_rectangle(
    x: f32,
    y: f32,
    width: f32,
    height: f32,
    screen_config: &ScreenConfig,
    (r, g, b): (f32, f32, f32),
    corner_radius: [f32; 4],
) -> Vec<Vertex> {
    // first normalize the coordinates to fit in decimal form.
    let ndc_x: f32 = 2.0 * (x as f32 / screen_config.width as f32) - 1.0;
    let ndc_y: f32 = 1.0 - (y as f32 / screen_config.height as f32) * 2.0;
    let ndc_r = |r: f32| (r / screen_config.width as f32) * 2.0;
    let radius = [
        ndc_r(corner_radius[0]),
        ndc_r(corner_radius[1]),
        ndc_r(corner_radius[2]),
        ndc_r(corner_radius[3]),
    ];
    let ndc_width: f32 = (width as f32 / screen_config.width as f32) * 2.0;
    let ndc_height: f32 = (height as f32 / screen_config.height as f32) * 2.0;

    let top_right = Vertex {
        position: [ndc_x + ndc_width, ndc_y, 0.0],
        color: [r, g, b],
        uv: [-1.0, -1.0],
        radius,
        half_size: [ndc_width / 2.0, ndc_height / 2.0],
        local_pos: [ndc_width / 2.0, ndc_height / 2.0],
    };

    let bottom_left = Vertex {
        position: [ndc_x, ndc_y - ndc_height, 0.0],
        color: [r, g, b],
        uv: [-1.0, -1.0],
        radius,
        half_size: [ndc_width / 2.0, ndc_height / 2.0],
        local_pos: [-ndc_width / 2.0, -ndc_height / 2.0],
    };

    let bottom_right = Vertex {
        position: [ndc_x + ndc_width, ndc_y - ndc_height, 0.0],
        color: [r, g, b],
        uv: [-1.0, -1.0],
        radius,
        half_size: [ndc_width / 2.0, ndc_height / 2.0],
        local_pos: [ndc_width / 2.0, -ndc_height / 2.0],
    };

    let top_left = Vertex {
        position: [ndc_x, ndc_y, 0.0],
        color: [r, g, b],
        uv: [-1.0, -1.0],
        radius,
        half_size: [ndc_width / 2.0, ndc_height / 2.0],
        local_pos: [-ndc_width / 2.0, ndc_height / 2.0],
    };

    // next add the verticies based on these origins
    return vec![
        //    - --/
        //    |  /
        //    | /
        //    |/
        top_left,
        bottom_left,
        top_right,
        //      /|
        //     / |
        //    /  |
        //   / _ |
        top_right.clone(),
        bottom_left,
        bottom_right,
    ];
}

// DRAW A CIRCLE USING TRIANGLE SEGMENTS
pub fn draw_circle(cx: f32, cy: f32, radius: f32, segments: u32, screen_config: &ScreenConfig, (r, g, b): (f32, f32, f32)) -> Vec<Vertex> {
    let mut vec: Vec<Vertex> = Vec::new();

    // first normalize the coordinates to fit in decimal form.
    let ncx: f32 = 2.0 * (cx as f32 / screen_config.width as f32) - 1.0;
    let ncy: f32 = 1.0 - (cy as f32 / screen_config.height as f32) * 2.0;
    let nrx = (radius / screen_config.width as f32) * 2.0;
    let nry = (radius / screen_config.height as f32) * 2.0;

    // draw the circle
    for k in 0..segments {
        let angle = k as f32 * (2.0 * std::f32::consts::PI / segments as f32);
        let next_angle = (k + 1) as f32 * (2.0 * std::f32::consts::PI / segments as f32);

        let x1 = ncx + nrx * angle.cos();
        let y1 = ncy + nry * angle.sin();
        let x2 = ncx + nrx * next_angle.cos();
        let y2 = ncy + nry * next_angle.sin();

        vec.push(Vertex {
            position: [ncx, ncy, 0.0],
            color: [r, g, b],
            uv: [-1.0, -1.0],
            radius: [0.0, 0.0, 0.0, 0.0],
            half_size: [ncx / 2.0, ncy / 2.0],
            local_pos: [ncx / 2.0, -ncy / 2.0],
        });
        vec.push(Vertex {
            position: [x1, y1, 0.0],
            color: [r, g, b],
            uv: [-1.0, -1.0],
            radius: [0.0, 0.0, 0.0, 0.0],
            half_size: [ncx / 2.0, ncy / 2.0],
            local_pos: [ncx / 2.0, -ncy / 2.0],
        });
        vec.push(Vertex {
            position: [x2, y2, 0.0],
            color: [r, g, b],
            uv: [-1.0, -1.0],
            radius: [0.0, 0.0, 0.0, 0.0],
            half_size: [ncx / 2.0, ncy / 2.0],
            local_pos: [ncx / 2.0, -ncy / 2.0],
        });
    }
    vec
}

// draw a knob using circle and triangles
pub fn draw_knob(vol: f32, cx: f32, cy: f32, radius: f32, segments: u32, screen_config: &ScreenConfig) -> Vec<Vertex> {
    let mut vec: Vec<Vertex> = draw_circle(cx, cy, radius + 3.0, 10, screen_config, BLACK);
    for vert in draw_circle(cx, cy, radius, segments, screen_config, LL_GRAY) {
        vec.push(vert);
    }
    let ncx = |x: f32| 2.0 * (x as f32 / screen_config.width as f32) - 1.0;
    let ncy = |y: f32| 1.0 - (y as f32 / screen_config.height as f32) * 2.0;
    let radians = |degree: f32| (degree * std::f32::consts::PI) / 180.0;

    let angle: f32 = 210.0 - vol * 270.0; // Linear interpolation
    let x = cx + radius * radians(angle).cos();
    let y = cy - radius * radians(angle).sin();

    // draw the dial
    vec.push(Vertex {
        position: [ncx(cx), ncy(cy), 0.0], // center
        color: [0.0, 0.0, 1.0],
        uv: [-1.0, -1.0],
        radius: [0.0, 0.0, 0.0, 0.0],
        half_size: [ncx(cx) / 2.0, ncy(cy) / 2.0],
        local_pos: [ncx(cx) / 2.0, -ncy(cy) / 2.0],
    });
    vec.push(Vertex {
        position: [ncx(x), ncy(y), 0.0], // hits circumfrence
        color: [0.0, 0.0, 1.0],
        uv: [-1.0, -1.0],
        radius: [0.0, 0.0, 0.0, 0.0],
        half_size: [ncx(cx) / 2.0, ncy(cy) / 2.0],
        local_pos: [ncx(cx) / 2.0, -ncy(cy) / 2.0],
    });
    let perp = radians(angle + 90.0);
    vec.push(Vertex {
        position: [ncx(x) + 0.01 * perp.cos(), ncy(y) + 0.01 * perp.sin(), 0.0],
        color: [0.0, 0.0, 1.0],
        uv: [-1.0, -1.0],
        radius: [0.0, 0.0, 0.0, 0.0],
        half_size: [ncx(cx) / 2.0, ncy(cy) / 2.0],
        local_pos: [ncx(cx) / 2.0, -ncy(cy) / 2.0],
    });
    vec
}

// draw a horizontal line
pub fn draw_h_line(y: f32, thickness: f32, screen_config: &ScreenConfig) -> Vec<Vertex> {
    // first normalize the coordinates to fit in decimal form.

    let ndc_y: f32 = 1.0 - (y as f32 / screen_config.height as f32) * 2.0;

    return vec![
        Vertex {
            position: [-1.0, ndc_y, 0.0],
            color: [0.0, 0.0, 0.0],
            uv: [-1.0, -1.0],
            radius: [0.0, 0.0, 0.0, 0.0],
            half_size: [-1.0 / 2.0, ndc_y / 2.0],
            local_pos: [-1.0 / 2.0, -ndc_y / 2.0],
        },
        Vertex {
            position: [1.0, ndc_y, 0.0],
            color: [0.0, 0.0, 0.0],
            uv: [-1.0, -1.0],
            radius: [0.0, 0.0, 0.0, 0.0],
            half_size: [-1.0 / 2.0, ndc_y / 2.0],
            local_pos: [-1.0 / 2.0, -ndc_y / 2.0],
        },
        Vertex {
            position: [1.0, ndc_y - thickness, 0.0],
            color: [0.0, 0.0, 0.0],
            uv: [-1.0, -1.0],
            radius: [0.0, 0.0, 0.0, 0.0],
            half_size: [-1.0 / 2.0, ndc_y / 2.0],
            local_pos: [-1.0 / 2.0, -ndc_y / 2.0],
        },
        Vertex {
            position: [-1.0, ndc_y, 0.0],
            color: [0.0, 0.0, 0.0],
            uv: [-1.0, -1.0],
            radius: [0.0, 0.0, 0.0, 0.0],
            half_size: [-1.0 / 2.0, ndc_y / 2.0],
            local_pos: [-1.0 / 2.0, -ndc_y / 2.0],
        },
        Vertex {
            position: [1.0, ndc_y - thickness, 0.0],
            color: [0.0, 0.0, 0.0],
            uv: [-1.0, -1.0],
            radius: [0.0, 0.0, 0.0, 0.0],
            half_size: [-1.0 / 2.0, ndc_y / 2.0],
            local_pos: [-1.0 / 2.0, -ndc_y / 2.0],
        },
        Vertex {
            position: [-1.0, ndc_y - thickness, 0.0],
            color: [0.0, 0.0, 0.0],
            uv: [-1.0, -1.0],
            radius: [0.0, 0.0, 0.0, 0.0],
            half_size: [-1.0 / 2.0, ndc_y / 2.0],
            local_pos: [-1.0 / 2.0, -ndc_y / 2.0],
        },
    ];
}
