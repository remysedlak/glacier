use winit::window::CursorIcon;

use crate::{
    app::MouseState,
    graphics::{
        color::WHITE,
        primitives::{ScreenConfig, PAD_24, PAD_32, PAD_64},
        widgets::Rectangle,
        ClickResult, Vertex,
    },
};

pub enum ContextMenuKind {
    PatternContext(usize),
    TrackContext(usize),
}

pub struct ContextMenu {
    pub kind: ContextMenuKind,
    pub x: f32,
    pub y: f32,
    pub height: f32,
    pub width: f32,
}

impl ContextMenu {
    pub fn draw(&self, screen_config: &ScreenConfig, mouse_state: &MouseState) -> (Vec<Vertex>, ClickResult, CursorIcon) {
        match &self.kind {
            ContextMenuKind::PatternContext(id) => self.draw_pattern_context(screen_config, mouse_state, *id),
            ContextMenuKind::TrackContext(track) => self.draw_track_context(screen_config, mouse_state, *track),
        }
    }

    fn draw_pattern_context(&self, screen_config: &ScreenConfig, mouse_state: &MouseState, id: usize) -> (Vec<Vertex>, ClickResult, CursorIcon) {
        let mut vertices: Vec<Vertex> = Vec::new();
        let mut cursor_icon = CursorIcon::Default;
        let mut click_result = ClickResult::None;
        for item in 0..6 {
            let context_item = Rectangle {
                height: 24.0,
                width: self.width,
                x: self.x - PAD_64,
                y: (self.y + PAD_24 * item as f32) + PAD_32,
            };
            vertices.extend(context_item.draw(screen_config, context_item.dark_hover_color(mouse_state.x, mouse_state.y)));
            if context_item.is_hovered(mouse_state.x, mouse_state.y) {
                cursor_icon = CursorIcon::Pointer;
                if mouse_state.right_clicked || mouse_state.left_clicked {
                    click_result = ClickResult::CloseContextMenu;
                }
            }
        }
        for line in 0..5 {
            let divider = Rectangle {
                height: 1.0,
                width: self.width - 4.0,
                x: (self.x + 2.0) - PAD_64,
                y: PAD_24 + (self.y + PAD_24 * line as f32) + PAD_32,
            };
            vertices.extend(divider.draw(screen_config, WHITE));
        }
        (vertices, click_result, cursor_icon)
    }

    fn draw_track_context(&self, screen_config: &ScreenConfig, mouse_state: &MouseState, id: usize) -> (Vec<Vertex>, ClickResult, CursorIcon) {
        let mut vertices: Vec<Vertex> = Vec::new();
        let mut cursor_icon = CursorIcon::Default;
        let mut click_result = ClickResult::None;
        for item in 0..5 {
            let context_item = Rectangle {
                height: 24.0,
                width: self.width,
                x: self.x - PAD_64,
                y: (self.y + PAD_24 * item as f32) + PAD_32,
            };
            vertices.extend(context_item.draw(screen_config, context_item.dark_hover_color(mouse_state.x, mouse_state.y)));
            if context_item.is_hovered(mouse_state.x, mouse_state.y) {
                cursor_icon = CursorIcon::Pointer;
                if mouse_state.right_clicked || mouse_state.left_clicked {
                    click_result = ClickResult::CloseContextMenu;
                }
            }
        }
        for line in 0..4 {
            let divider = Rectangle {
                height: 1.0,
                width: self.width - 4.0,
                x: (self.x + 2.0) - PAD_64,
                y: PAD_24 + (self.y + PAD_24 * line as f32) + PAD_32,
            };
            vertices.extend(divider.draw(screen_config, WHITE));
        }
        (vertices, click_result, cursor_icon)
    }
}
