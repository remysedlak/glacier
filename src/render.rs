use crate::graphics::Vertex;

// this file holds my shape abstractions

#[derive(Debug)]
pub struct StepButton {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
    pub is_active: bool,
}

pub fn draw_rectangle(
    x: u32,
    y: u32,
    width: u32,
    height: u32,
    screen_width: u32,
    screen_height: u32,
    (r, g, b): (f32, f32, f32),
) -> Vec<Vertex> {
    // first normalize the coordinates to fit in decimal form.
    let ndc_x: f32 = 2.0 * (x as f32 / screen_width as f32) - 1.0;
    let ndc_y: f32 = 1.0 - (y as f32 / screen_height as f32) * 2.0;

    let ndc_width: f32 = (width as f32 / screen_width as f32) * 2.0;
    let ndc_height: f32 = (height as f32 / screen_height as f32) * 2.0;

    // next add the verticies based on these origins
    return vec![
        Vertex {
            position: [ndc_x, ndc_y, 0.0],
            color: [r, g, b],
        },
        Vertex {
            position: [ndc_x, ndc_y - ndc_height, 0.0],
            color: [r, g, b],
        },
        Vertex {
            position: [ndc_x + ndc_width, ndc_y, 0.0],
            color: [r, g, b],
        },
        Vertex {
            position: [ndc_x + ndc_width, ndc_y, 0.0],
            color: [r, g, b],
        },
        Vertex {
            position: [ndc_x, ndc_y - ndc_height, 0.0],
            color: [r, g, b],
        },
        Vertex {
            position: [ndc_x + ndc_width, ndc_y - ndc_height, 0.0],
            color: [r, g, b],
        },
    ];
}

pub fn draw_h_line(y: f32, thickness: f32, screen_height: u32) -> Vec<Vertex> {
    // first normalize the coordinates to fit in decimal form.
    let mut vertices: Vec<Vertex> = Vec::new();

    let ndc_y: f32 = 1.0 - (y as f32 / screen_height as f32) * 2.0;

    return vec![
        Vertex {
            position: [-1.0, ndc_y, 0.0],
            color: [0.0, 0.0, 0.0],
        },
        Vertex {
            position: [1.0, ndc_y, 0.0],
            color: [0.0, 0.0, 0.0],
        },
        Vertex {
            position: [1.0, ndc_y - thickness, 0.0],
            color: [0.0, 0.0, 0.0],
        },
        Vertex {
            position: [-1.0, ndc_y, 0.0],
            color: [0.0, 0.0, 0.0],
        },
        Vertex {
            position: [1.0, ndc_y - thickness, 0.0],
            color: [0.0, 0.0, 0.0],
        },
        Vertex {
            position: [-1.0, ndc_y - thickness, 0.0],
            color: [0.0, 0.0, 0.0],
        },
    ];
}
