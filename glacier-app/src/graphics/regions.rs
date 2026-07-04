//! Glacier does not have a layout system yet but needed a system to track where draw regions or groups of vertexes are for scissor rects and z layering.
use crate::graphics::Vertex;

/// A recordeed region tracks where vertices are in the vertex buffer and the coordinates of a scissor rectangle
pub struct RecordedRegion {
    pub range: WindowDrawRange,
    pub scissor: Option<(u32, u32, u32, u32)>,
}

/// Tracks the position of the global vertex buffers of where a windows shapes are.
pub struct WindowDrawRange {
    pub vert_start: u32,
    pub vert_end: u32,
    pub char_start: usize,
    pub char_end: usize,
}

/// the playlist has three DrawRange's: non moving static shapes, the header of the playlist, and the timeline itself
pub struct PlaylistDrawRanges {
    pub static_range: WindowDrawRange,
    pub header_range: WindowDrawRange,
    pub timeline_range: WindowDrawRange,
}

/// the piano roll has three DrawRanges: non moving static shapes, the actual piano, and the piano grid.
pub struct PianoRollDrawRanges {
    pub static_range: WindowDrawRange, // background + titlebar
    pub piano_range: WindowDrawRange,  // fixed piano keys, no scroll
    pub grid_range: WindowDrawRange,   // scrollable note grid
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

/// Draw geometry + glyphs for a WindowDrawRange in one call
pub fn draw_range(
    r_pass: &mut wgpu::RenderPass,
    vertex_buffer: &wgpu::Buffer,
    glyph_vertex_buffer: &wgpu::Buffer,
    any_bg: &wgpu::BindGroup,
    char_draws: &[(u64, &wgpu::BindGroup)],
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
}
