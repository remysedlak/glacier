use crate::{
    app::MouseState,
    graphics::{
        color::Color,
        primitives::{draw_rectangle, draw_rectangle_bordered, Vertex, PAD_4, PAD_8},
        ScreenConfig,
    },
};

/// A square stores 2D position and size
#[derive(Debug)]
pub struct Square {
    pub x: f32,
    pub y: f32,
    pub size: f32,
}
impl Square {
    pub fn is_hovered(&self, mouse_x: f32, mouse_y: f32) -> bool {
        mouse_x > self.x
            && mouse_x < self.x + self.size
            && mouse_y > self.y
            && mouse_y < self.y + self.size
    }

    // draw vertices with rectangle details
    pub fn draw(
        &self,
        screen_config: &ScreenConfig,
        color: Color,
        corner_radius: [f32; 4],
        out: &mut Vec<Vertex>,
    ) {
        draw_rectangle(
            self.x,
            self.y,
            self.size,
            self.size,
            screen_config,
            color,
            corner_radius,
            out,
        );
    }

    /// draw vertices with rectangle details, including a border
    pub fn draw_bordered(
        &self,
        screen_config: &ScreenConfig,
        color: Color,
        corner_radius: [f32; 4],
        border_width: f32,
        border_color: Color,
        out: &mut Vec<Vertex>,
    ) {
        draw_rectangle_bordered(
            self.x,
            self.y,
            self.size,
            self.size,
            screen_config,
            color,
            corner_radius,
            border_width,
            border_color,
            out,
        );
    }

    pub fn draw_interactive(
        &self,
        screen_config: &ScreenConfig,
        base_color: Color,
        mouse_state: &MouseState,
        radius: [f32; 4],
        out: &mut Vec<Vertex>,
    ) -> bool {
        let hovered = self.is_hovered(mouse_state.x, mouse_state.y);
        let color = if hovered {
            base_color.hovered()
        } else {
            base_color
        };
        self.draw(screen_config, color, radius, out);
        hovered
    }
}

/// A rectangle stores 2D position,width, and height
#[derive(Debug)]
pub struct Rectangle {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}
impl Rectangle {
    /// if a rectangle has the mouse hovered
    pub fn is_hovered(&self, mouse_x: f32, mouse_y: f32) -> bool {
        mouse_x > self.x
            && mouse_x < self.x + self.width
            && mouse_y > self.y
            && mouse_y < self.y + self.height
    }
    /// if the left edge of a rectangle has the mouse hovered
    pub fn is_hovered_left_edge(&self, mouse_x: f32, mouse_y: f32) -> bool {
        // on left edge within y range
        (mouse_x > self.x - PAD_4 && mouse_x < self.x + PAD_4)
            && mouse_y > self.y
            && mouse_y < self.y + self.height
    }
    /// if the right edge of a rectangle has the mouse hovered
    pub fn is_hovered_right_edge(&self, mouse_x: f32, mouse_y: f32) -> bool {
        // on r edge within y range
        (mouse_x > self.x + self.width - PAD_8 && mouse_x < self.x + self.width + PAD_8)
            && mouse_y > self.y
            && mouse_y < self.y + self.height
    }
    /// draw vertices with rectangle details
    pub fn draw(
        &self,
        screen_config: &ScreenConfig,
        color: Color,
        corner_radius: [f32; 4],
        out: &mut Vec<Vertex>,
    ) {
        draw_rectangle(
            self.x,
            self.y,
            self.width,
            self.height,
            screen_config,
            color,
            corner_radius,
            out,
        );
    }

    /// draw vertices with rectangle details, including a border
    pub fn draw_bordered(
        &self,
        screen_config: &ScreenConfig,
        color: Color,
        corner_radius: [f32; 4],
        border_width: f32,
        border_color: Color,
        out: &mut Vec<Vertex>,
    ) {
        draw_rectangle_bordered(
            self.x,
            self.y,
            self.width,
            self.height,
            screen_config,
            color,
            corner_radius,
            border_width,
            border_color,
            out,
        );
    }

    pub fn draw_interactive(
        &self,
        screen_config: &ScreenConfig,
        base_color: Color,
        mouse_state: &MouseState,
        radius: [f32; 4],
        out: &mut Vec<Vertex>,
    ) -> bool {
        let hovered = self.is_hovered(mouse_state.x, mouse_state.y);
        let color = if hovered {
            base_color.hovered()
        } else {
            base_color
        };
        self.draw(screen_config, color, radius, out);
        hovered
    }
}
