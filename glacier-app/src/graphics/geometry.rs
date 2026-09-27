//! Store builder logic for custom shapes like Rectangle

use crate::{
    app::MouseState,
    graphics::{
        color::{Color, DARK_GRAY, LL_GRAY, WHITE},
        font::{
            Font::{Mono, Roboto},
            TextItem,
        },
        primitives::{
            draw_rectangle, draw_rectangle_bordered, Vertex, PAD_4, PAD_64, PAD_8, RADIUS_4,
        },
        ScreenConfig,
    },
};

#[derive(Copy, Clone)]
/// Used for styling bordered rectangles
pub struct BorderStyle {
    pub size: f32,
    pub color: Color,
}

// Default border for now.
pub const ICON_BORDER: BorderStyle = BorderStyle {
    color: LL_GRAY,
    size: 0.5,
};

/// Helper for building rectangles
pub struct RectangleCtx<'a> {
    rectangle: &'a Rectangle,
    interactive: Option<&'a MouseState>,
    hover_effect: bool,
    border: Option<BorderStyle>,
}

/// Helper for storing placed rectangle information
pub struct DrawResponse {
    pub hovered: bool,
    pub right_hovered: bool,
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

impl<'a> RectangleCtx<'a> {
    /// Apply BorderStyle to Rectangle context
    pub fn bordered(mut self, border_style: Option<BorderStyle>) -> RectangleCtx<'a> {
        self.border = border_style;
        self
    }
    /// Apply interaction to Rectangle context
    pub fn interactive(mut self, mouse_state: Option<&'a MouseState>) -> RectangleCtx<'a> {
        self.interactive = mouse_state;
        self
    }
    /// Apply disable interaction state for Rectangle context
    pub fn disabled(mut self) -> RectangleCtx<'a> {
        self.hover_effect = false;
        self
    }
    /// Draw a Rectangle based on built RectangleCtx
    pub fn draw(
        self,
        screen_config: &ScreenConfig,
        color: Color,
        corner_radius: [f32; 4],
        out: &mut Vec<Vertex>,
    ) -> DrawResponse {
        let mut rectangle_color = color;
        let mut hovered = false;
        let mut right_hovered = false;
        if let Some(mouse_state) = self.interactive {
            hovered = self.rectangle.is_hovered(mouse_state.x, mouse_state.y);
            if self.hover_effect {
                rectangle_color = if hovered { color.hovered() } else { color };
            }
            right_hovered = self
                .rectangle
                .is_hovered_right_edge(mouse_state.x, mouse_state.y);
        }

        if let Some(border_style) = self.border {
            Rectangle::draw_bordered(
                self.rectangle,
                screen_config,
                rectangle_color,
                corner_radius,
                border_style.size,
                border_style.color,
                out,
            );
        } else {
            Rectangle::draw(
                self.rectangle,
                screen_config,
                rectangle_color,
                corner_radius,
                out,
            );
        }
        DrawResponse {
            hovered,
            right_hovered,
            x: self.rectangle.x,
            y: self.rectangle.y,
            width: self.rectangle.width,
            height: self.rectangle.height,
        }
    }
}
//  window.x + PAD_64 + PAD_32
//  window.y + PAD_32 + PAD_8,
pub fn checkbox(
    x: f32,
    y: f32,
    is_toggled: bool,
    mouse_state: &MouseState,
    screen_config: &ScreenConfig,
    out_vertices: &mut Vec<Vertex>,
    out_texts: &mut Vec<TextItem>,
) -> DrawResponse {
    let checkbox = Rectangle::new(x, y, 16.0, 16.0)
        .draw_style()
        .interactive(Some(mouse_state))
        .bordered(Some(ICON_BORDER))
        .draw(screen_config, DARK_GRAY, RADIUS_4, out_vertices);

    if is_toggled {
        let check = TextItem {
            text: "x".to_string(),
            color: WHITE,
            size: 12.0,
            font: Roboto,
            x: checkbox.x + PAD_4,
            y: checkbox.y,
        };
        out_texts.push(check);
    }

    let label = TextItem {
        text: "stretch".to_string(),
        size: 12.0,
        font: Mono,
        x: checkbox.x - PAD_64,
        y: checkbox.y,
        color: WHITE,
    };
    out_texts.push(label);
    return checkbox;
}

/// A rectangle stores 2D position,width, and height
#[derive(Debug)]
pub struct Rectangle {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}
impl<'a> Rectangle {
    pub fn draw_style(&'a self) -> RectangleCtx<'a> {
        RectangleCtx {
            rectangle: self,
            interactive: None,
            border: None,
            hover_effect: true,
        }
    }

    pub fn new(x: f32, y: f32, width: f32, height: f32) -> Rectangle {
        Rectangle {
            x,
            y,
            width,
            height,
        }
    }

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

    pub fn square(x: f32, y: f32, size: f32) -> Self {
        Rectangle {
            x,
            y,
            width: size,
            height: size,
        }
    }

    pub fn inset(&self, amount: f32) -> Rectangle {
        Rectangle {
            x: self.x + amount,
            y: self.y + amount,
            width: self.width - amount * 2.0,
            height: self.height - amount * 2.0,
        }
    }

    /// Shrinks horizontally only (left + right), leaving y/height untouched.
    pub fn inset_x(&self, amount: f32) -> Rectangle {
        Rectangle {
            x: self.x + amount,
            width: self.width - amount * 2.0,
            ..*self
        }
    }

    /// Offsets by (dx, dy) without resizing — for positioning a smaller
    /// element (icon, text) inside a parent rect.
    pub fn offset(&self, dx: f32, dy: f32) -> Rectangle {
        Rectangle {
            x: self.x + dx,
            y: self.y + dy,
            ..*self
        }
    }
}
