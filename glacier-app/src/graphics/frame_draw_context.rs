use super::{
    color::Color,
    font::{self, FontCache, GlyphCache, TextItem},
    icons::{push_icon_draw, IconCache, IconDraw},
    mini_window::InteractionResult,
    primitives::{ScreenConfig, Vertex, VertexSink},
    regions::RecordedRegion,
    DrawItem,
};
use fontdue::layout::{CoordinateSystem, Layout, TextStyle};

/// Owns the CPU-side drawing data collected for one frame.
pub struct FrameDrawContext<'a> {
    pub(super) screen_config: ScreenConfig<'a>,
    pub(super) font_cache: &'a FontCache,
    pub(super) glyph_cache: &'a GlyphCache,
    pub(super) icon_cache: &'a IconCache,
    pub(super) vertices: Vec<Vertex>,
    pub(super) glyph_vertices: Vec<Vertex>,
    pub(super) char_draws: Vec<DrawItem>,
    pub(super) icon_vertices: Vec<Vertex>,
    pub(super) icon_draws: Vec<DrawItem>,
    pub(super) regions: Vec<RecordedRegion>,
    pub(super) interaction: InteractionResult,
    pub(super) tooltip: Option<crate::graphics::icons::Tooltip>,
}

impl<'a> FrameDrawContext<'a> {
    pub(super) fn new(
        screen_config: ScreenConfig<'a>,
        font_cache: &'a FontCache,
        glyph_cache: &'a GlyphCache,
        icon_cache: &'a IconCache,
    ) -> Self {
        Self {
            screen_config,
            font_cache,
            glyph_cache,
            icon_cache,
            vertices: Vec::new(),
            glyph_vertices: Vec::new(),
            char_draws: Vec::new(),
            icon_vertices: Vec::new(),
            icon_draws: Vec::new(),
            regions: Vec::new(),
            interaction: InteractionResult::default(),
            tooltip: None,
        }
    }

    pub(super) fn record_region(
        &mut self,
        vertex_start: u32,
        char_start: usize,
        icon_start: usize,
        scissor: Option<(u32, u32, u32, u32)>,
    ) {
        self.regions.push(RecordedRegion::record(
            &self.vertices,
            &self.char_draws,
            &self.icon_draws,
            vertex_start,
            char_start,
            icon_start,
            scissor,
        ));
    }

    pub(crate) fn draw_texts(&mut self, texts: &[TextItem]) {
        append_text_draws(
            texts,
            self.font_cache,
            self.glyph_cache,
            &self.screen_config,
            &mut self.glyph_vertices,
            &mut self.char_draws,
        );
    }

    pub(crate) fn draw_icon(&mut self, icon: &IconDraw) {
        let icon_cache = self.icon_cache;
        let screen_config = self.screen_config;
        push_icon_draw(
            icon_cache,
            &screen_config,
            icon,
            &mut self.icon_vertices,
            &mut self.icon_draws,
        );
    }

    pub(crate) fn add_interaction(&mut self, interaction: InteractionResult) {
        self.interaction = std::mem::take(&mut self.interaction).or(interaction);
    }

    pub(crate) fn set_tooltip_if_some(&mut self, tooltip: Option<crate::graphics::icons::Tooltip>) {
        if tooltip.is_some() {
            self.tooltip = tooltip;
        }
    }

    pub(crate) fn set_tooltip(&mut self, tooltip: Option<crate::graphics::icons::Tooltip>) {
        self.tooltip = tooltip;
    }

    pub(crate) fn screen_config(&self) -> &ScreenConfig<'a> {
        &self.screen_config
    }
}

impl VertexSink for FrameDrawContext<'_> {
    fn push_vertex(&mut self, vertex: Vertex) {
        self.vertices.push(vertex);
    }
}

fn append_text_draws(
    texts: &[TextItem],
    font_cache: &FontCache,
    glyph_cache: &GlyphCache,
    screen_config: &ScreenConfig,
    glyph_vertices: &mut Vec<Vertex>,
    char_draws: &mut Vec<DrawItem>,
) {
    for text_item in texts {
        let Some(font) = font_cache.get(text_item.font) else {
            continue;
        };
        let mut layout = Layout::new(CoordinateSystem::PositiveYDown);
        let Color { r, g, b, a } = text_item.color;
        layout.append(&[font], &TextStyle::new(&text_item.text, text_item.size, 0));

        for glyph in layout.glyphs() {
            if let Some(entry) =
                glyph_cache.get(text_item.font, glyph.parent, text_item.size as u32)
            {
                let vertices = font::draw_glyph(
                    (text_item.x + glyph.x).round(),
                    (text_item.y + glyph.y).round(),
                    glyph.width as f32,
                    glyph.height as f32,
                    entry.uv_min,
                    entry.uv_max,
                    screen_config,
                    (r, g, b, a),
                );
                let offset = (glyph_vertices.len() * std::mem::size_of::<Vertex>()) as u64;
                glyph_vertices.extend_from_slice(&vertices);
                char_draws.push(DrawItem {
                    vertex_offset: offset,
                });
            }
        }
    }
}
