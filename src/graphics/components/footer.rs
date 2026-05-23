use crate::graphics::{
    color::{BLACK, ORANGE, WHITE},
    font::TextItem,
    primitives::{ScreenConfig, Vertex, NO_RADIUS, PAD_4, PAD_8},
    widgets::Rectangle,
};

pub fn draw(screen_config: &ScreenConfig, path: &String, frame_rate: f32) -> (Vec<Vertex>, Vec<TextItem>) {
    // setup vectors
    let mut vertices: Vec<Vertex> = Vec::new();
    let mut texts: Vec<TextItem> = Vec::new();

    // coordinates
    let footer_x = 0.0;
    let footer_y = screen_config.height as f32 - 32.0;

    let footer = Rectangle {
        x: footer_x,
        y: footer_y,
        width: screen_config.width as f32,
        height: 32.0,
    };
    vertices.extend(footer.draw(screen_config, BLACK, NO_RADIUS));
    texts.push(TextItem {
        text: path.to_string(),
        x: footer_x + PAD_4,
        y: footer_y + PAD_8,
        size: 12.0,
        color: WHITE,
        font: "roboto",
    });

    texts.push(TextItem {
        text: frame_rate.to_string(),
        x: screen_config.width as f32 - 80.0,
        y: footer_y + PAD_8,
        size: 12.0,
        color: ORANGE,
        font: "mono",
    });
    (vertices, texts)
}
