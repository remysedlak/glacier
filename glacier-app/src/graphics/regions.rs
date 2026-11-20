//! Glacier does not have a layout system yet but needed a system to track where draw regions or groups of vertexes are for scissor rects and z layering.
use crate::graphics::{DrawItem, Vertex};

/// A recorded region tracks where vertices are in the vertex buffer and the coordinates of a scissor rectangle
pub struct RecordedRegion {
    pub range: WindowDrawRange,
    pub scissor: Option<(u32, u32, u32, u32)>,
}
impl RecordedRegion {
    pub(crate) fn record(
        vertices: &[Vertex],
        char_draws: &[DrawItem],
        icon_draws: &[DrawItem],
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
    /// Half-open vertex indices in the shape buffer; `u32` matches wgpu's draw range.
    pub vert_start: u32,
    pub vert_end: u32,
    /// Half-open indices into the frame's `char_draws` list.
    pub char_start: usize,
    pub char_end: usize,
    /// Half-open indices into the frame's `icon_draws` list.
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

/// Draw a contiguous range of glyph quads from the shared atlas.
pub(crate) fn draw_chars(
    r_pass: &mut wgpu::RenderPass,
    glyph_vertex_buffer: &wgpu::Buffer,
    char_draws: &[DrawItem],
    atlas_bind_group: &wgpu::BindGroup,
    start: usize,
    end: usize,
) {
    if start < end {
        let stride = std::mem::size_of::<Vertex>() as u64;
        let first = char_draws[start].vertex_offset / stride;
        let last = char_draws[end - 1].vertex_offset / stride + 6;
        r_pass.set_bind_group(0, atlas_bind_group, &[]);
        r_pass.set_vertex_buffer(0, glyph_vertex_buffer.slice(..));
        r_pass.draw(first as u32..last as u32, 0..1);
    }
}

/// Draw a range of icon quads from the shared icon atlas.
pub(crate) fn draw_icons(
    r_pass: &mut wgpu::RenderPass,
    icon_vertex_buffer: &wgpu::Buffer,
    icon_draws: &[DrawItem],
    atlas_bind_group: &wgpu::BindGroup,
    start: usize,
    end: usize,
) {
    if start < end {
        let stride = std::mem::size_of::<Vertex>() as u64;
        let first = icon_draws[start].vertex_offset / stride;
        let last = icon_draws[end - 1].vertex_offset / stride + 6;
        r_pass.set_bind_group(0, atlas_bind_group, &[]);
        r_pass.set_vertex_buffer(0, icon_vertex_buffer.slice(..));
        r_pass.draw(first as u32..last as u32, 0..1);
    }
}

/// Draw geometry + glyphs + icons for a WindowDrawRange in one call
pub(crate) fn draw_range(
    r_pass: &mut wgpu::RenderPass,
    vertex_buffer: &wgpu::Buffer,
    glyph_vertex_buffer: &wgpu::Buffer,
    icon_vertex_buffer: &wgpu::Buffer,
    any_bg: &wgpu::BindGroup,
    char_draws: &[DrawItem],
    icon_draws: &[DrawItem],
    glyph_atlas_bind_group: &wgpu::BindGroup,
    icon_atlas_bind_group: &wgpu::BindGroup,
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
        glyph_atlas_bind_group,
        range.char_start,
        range.char_end,
    );
    draw_icons(
        r_pass,
        icon_vertex_buffer,
        icon_draws,
        icon_atlas_bind_group,
        range.icon_start,
        range.icon_end,
    );
}
