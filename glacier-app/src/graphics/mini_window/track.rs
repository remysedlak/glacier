use crate::app::{MouseState, click::ClickResult};
use crate::graphics::icons::DEFAULT_TOOLTIP_WIDTH;
use crate::graphics::mini_window::InteractionResult;
use crate::graphics::{
    components::toolbar::TOOLTIP_MARGIN,
    icons::{IconDraw, Tooltip},
    mini_window::{MiniWindow, TITLEBAR_HEIGHT},
    primitives::{NO_RADIUS, PAD_8, PAD_16, RADIUS_4},
    {FrameDrawContext, Rectangle},
};
use crate::project::Track;

const TRACK_GRAPHICS_WIDTH: f32 = 200.0;
const TRACK_GRAPHICS_HEIGHT: f32 = 128.0;
const TRACK_GRAPHICS_HEIGHT_HALF: f32 = 128.0 / 2.0;

/// Draw a Mini Window that contains the track information and audio tools
pub fn draw(
    window: &MiniWindow,
    mouse_state: &MouseState,
    track: &Track,
    frame: &mut FrameDrawContext<'_>,
) -> InteractionResult {
    let screen_config = *frame.screen_config();
    let mut interaction = InteractionResult::default();
    // window background
    let window_background = window.background();
    window_background.draw(
        &screen_config,
        screen_config.palette.neutral.shade(4),
        [0.0, 16.0, 0.0, 16.0],
        frame,
    );

    // titlebar
    let (titlebar_texts, titlebar_interaction) = window.title_bar(
        &format!("Track: {}", track.data.name),
        &screen_config,
        mouse_state,
        frame,
    );
    interaction = interaction.or(titlebar_interaction);
    frame.draw_texts(&[titlebar_texts]);

    // draw background of wave form for track
    let graphics_y = (window.y) + TITLEBAR_HEIGHT - PAD_16;
    let center_y = graphics_y + TRACK_GRAPHICS_HEIGHT / 2.0;
    let track_wave_background = Rectangle {
        x: (window.x + window.width) - PAD_16 - TRACK_GRAPHICS_WIDTH,
        y: graphics_y,
        width: TRACK_GRAPHICS_WIDTH,
        height: TRACK_GRAPHICS_HEIGHT,
    };
    track_wave_background.draw(
        &screen_config,
        screen_config.palette.neutral.shade(3),
        NO_RADIUS,
        frame,
    );

    let samples_averaged: &[f32] = if track.data.channels == 1 {
        &track.samples
    } else {
        &track.mono_samples
    };
    let sample_stride = samples_averaged.len() / TRACK_GRAPHICS_WIDTH as usize;

    // 200 pixel columns for 200 pixel graphics
    for pixel_column in 0..199 {
        // get the first and last position of the stride
        let start = pixel_column * sample_stride;
        let end = (start + sample_stride).min(samples_averaged.len());
        // using  the start and end range,  find the highest and lowest amplitude within that stride
        let max = samples_averaged[start..end]
            .iter()
            .cloned()
            .fold(f32::NEG_INFINITY, f32::max);
        let min = samples_averaged[start..end]
            .iter()
            .cloned()
            .fold(f32::INFINITY, f32::min);
        let pixel_line = Rectangle {
            x: track_wave_background.x + pixel_column as f32,
            y: center_y - (max * TRACK_GRAPHICS_HEIGHT_HALF),
            height: (max - min) * TRACK_GRAPHICS_HEIGHT_HALF,
            width: 1.0,
        };
        pixel_line.draw(
            &screen_config,
            screen_config.palette.neutral.shade(7),
            NO_RADIUS,
            frame,
        );
    }

    let open_file_button_x = (window.x + window.width) - PAD_16 - TRACK_GRAPHICS_WIDTH;
    let open_file_button_y = graphics_y + TRACK_GRAPHICS_HEIGHT + PAD_8;
    const SVG_PADDING: f32 = 4.0;

    let open_file_background = Rectangle::square(
        open_file_button_x - SVG_PADDING,
        open_file_button_y - SVG_PADDING,
        32.0 + SVG_PADDING,
    )
    .draw_style()
    .interactive(Some(mouse_state))
    .draw(
        &screen_config,
        screen_config.palette.neutral.shade(3),
        RADIUS_4,
        frame,
    );

    if open_file_background.hovered && mouse_state.left_clicked {
        interaction.click = ClickResult::OpenTrackFileLocation(track.data.path.clone())
    };

    let icon = IconDraw {
        name: "file",
        x: open_file_button_x - 2.0,
        y: open_file_button_y - 2.0,
        width: 32.0,
        height: 32.0,
        tooltip: Tooltip {
            text: Some("Open File".to_string()),
            x: (open_file_button_x),
            y: (open_file_button_y + TOOLTIP_MARGIN),
            width: DEFAULT_TOOLTIP_WIDTH,
        },
    };
    frame.draw_icon(&icon);

    if !mouse_state.left_click_held {
        if icon.is_hovered(mouse_state.x, mouse_state.y) {
            frame.set_tooltip_if_some(Some(icon.tooltip.clone()));
        }
    }

    interaction
}
