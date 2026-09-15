//! Glacier does not have a layout system yet but needed a system to track where draw regions or groups of vertexes are for scissor rects and z layering.
use crate::graphics::Vertex;

/// A recorded region tracks where vertices are in the vertex buffer and the coordinates of a scissor rectangle
pub struct RecordedRegion {
    pub range: WindowDrawRange,
    pub scissor: Option<(u32, u32, u32, u32)>,
}
impl RecordedRegion {
    pub fn record(
        vertices: &Vec<Vertex>,
        char_draws: &Vec<(u64, &wgpu::BindGroup)>,
        icon_draws: &Vec<(u64, &wgpu::BindGroup)>,
        vert_start: u32,
        char_start: usize,
        icon_start: usize,
        scissor: Option<(u32, u32, u32, u32)>,
    ) -> RecordedRegion {
        RecordedRegion {
            range: WindowDrawRange {
                vert_start,
                vert_end: vertices.len() as u32,
                char_start,
                char_end: char_draws.len(),
                icon_start,
                icon_end: icon_draws.len(),
            },
            scissor,
        }
    }
}

/// Tracks the position of the global vertex/glyph/icon buffers of where a window's shapes are.
pub struct WindowDrawRange {
    pub vert_start: u32,
    pub vert_end: u32,
    pub char_start: usize,
    pub char_end: usize,
    pub icon_start: usize,
    pub icon_end: usize,
}

/// Clamps a scissor rect so paint never goes outside screen bounds
pub fn safe_scissor(x: u32, y: u32, w: u32, h: u32, sw: u32, sh: u32) -> (u32, u32, u32, u32) {
    let x = x.min(sw.saturating_sub(1));
    let y = y.min(sh.saturating_sub(1));
    let w = w.min(sw.saturating_sub(x)).max(1);
    let h = h.min(sh.saturating_sub(y)).max(1);
    (x, y, w, h)
}

/// Draw a range of colored/textured geometry quads
pub fn draw_geom(
    r_pass: &mut wgpu::RenderPass,
    vertex_buffer: &wgpu::Buffer,
    any_bg: &wgpu::BindGroup,
    start: u32,
    end: u32,
) {
    if start < end {
        r_pass.set_bind_group(0, any_bg, &[]);
        r_pass.set_vertex_buffer(0, vertex_buffer.slice(..));
        r_pass.draw(start..end, 0..1);
    }
}

/// Draw a range of glyph quads, each with its own bind group
pub fn draw_chars(
    r_pass: &mut wgpu::RenderPass,
    glyph_vertex_buffer: &wgpu::Buffer,
    char_draws: &[(u64, &wgpu::BindGroup)],
    start: usize,
    end: usize,
) {
    let stride = (6 * std::mem::size_of::<Vertex>()) as u64;
    for (offset, bg) in char_draws.iter().skip(start).take(end - start) {
        r_pass.set_bind_group(0, *bg, &[]);
        r_pass.set_vertex_buffer(0, glyph_vertex_buffer.slice(*offset..*offset + stride));
        r_pass.draw(0..6, 0..1);
    }
}

/// Draw a range of icon quads, each with its own bind group — identical
/// shape to draw_chars, since icons and glyphs are both textured quads
/// needing a per-item bind group swap.
pub fn draw_icons(
    r_pass: &mut wgpu::RenderPass,
    icon_vertex_buffer: &wgpu::Buffer,
    icon_draws: &[(u64, &wgpu::BindGroup)],
    start: usize,
    end: usize,
) {
    let stride = (6 * std::mem::size_of::<Vertex>()) as u64;
    for (offset, bg) in icon_draws.iter().skip(start).take(end - start) {
        r_pass.set_bind_group(0, *bg, &[]);
        r_pass.set_vertex_buffer(0, icon_vertex_buffer.slice(*offset..*offset + stride));
        r_pass.draw(0..6, 0..1);
    }
}

/// Draw geometry + glyphs + icons for a WindowDrawRange in one call
pub fn draw_range(
    r_pass: &mut wgpu::RenderPass,
    vertex_buffer: &wgpu::Buffer,
    glyph_vertex_buffer: &wgpu::Buffer,
    icon_vertex_buffer: &wgpu::Buffer,
    any_bg: &wgpu::BindGroup,
    char_draws: &[(u64, &wgpu::BindGroup)],
    icon_draws: &[(u64, &wgpu::BindGroup)],
    range: &WindowDrawRange,
) {
    draw_geom(
        r_pass,
        vertex_buffer,
        any_bg,
        range.vert_start,
        range.vert_end,
    );
    draw_chars(
        r_pass,
        glyph_vertex_buffer,
        char_draws,
        range.char_start,
        range.char_end,
    );
    draw_icons(
        r_pass,
        icon_vertex_buffer,
        icon_draws,
        range.icon_start,
        range.icon_end,
    );
}
