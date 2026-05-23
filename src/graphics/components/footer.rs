use crate::graphics::{
    color::{BLACK, ORANGE, WHITE},
    font::{TextItem, MONO_FONT, ROBOTO_FONT},
    primitives::{ScreenConfig, Vertex, NO_RADIUS, PAD_4, PAD_8},
    widgets::Rectangle,
};

pub const FOOTER_Y_HEIGHT: f32 = 32.0;
pub const FPS_COUNTER_X_OFFSET: f32 = 80.0;

pub fn draw(screen_config: &ScreenConfig, path: &String, frame_rate: f32) -> (Vec<Vertex>, Vec<TextItem>) {
    // setup vectors
    let mut vertices: Vec<Vertex> = Vec::new();
    let mut texts: Vec<TextItem> = Vec::new();

    // coordinates
    let footer_x = 0.0;

    let footer = Rectangle {
        x: footer_x,
        y: screen_config.height as f32 - FOOTER_Y_HEIGHT,
        width: screen_config.width as f32,
        height: FOOTER_Y_HEIGHT,
    };
    vertices.extend(footer.draw(screen_config, BLACK, NO_RADIUS));
    texts.push(TextItem {
        text: path.to_string(),
        x: footer_x + PAD_4,
        y: screen_config.height as f32 - FOOTER_Y_HEIGHT + PAD_8,
        size: 12.0,
        color: WHITE,
        font: ROBOTO_FONT,
    });

    // display frames per second
    texts.push(TextItem {
        text: frame_rate.to_string(),
        x: screen_config.width as f32 - FPS_COUNTER_X_OFFSET,
        y: screen_config.height as f32 - FOOTER_Y_HEIGHT + PAD_8,
        size: 12.0,
        color: ORANGE,
        font: MONO_FONT,
    });
    (vertices, texts)
}
