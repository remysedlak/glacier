use crate::graphics::mini_window::{mixer::MIXER_ITEM_WIDTH, playlist::PLAYLIST_STEP_GAP};

use super::*;

pub enum DragResult {
    // mixer
    DragMasterVolumeSlider(f32),
    DragTrackVolumeSlider(usize, f32),
    // sequencer
    DragTrackVolumeKnob(usize, f32),
    // playlist
    ResizeAudioBlock(usize, u32),

    // tray resizing
    ResizeTrackTray(f32),
    None,
}
impl Graphics {
    /// Track if/where the user's mouse is dragging a component
    pub fn handle_drag(&mut self, mouse_x: f32, mouse_y: f32, dy: f32, dx: f32) -> DragResult {
        // sticky drags — once started, keep going until mouse release
        if self.resizing_track_tray {
            self.track_tray_width = (self.track_tray_width + dx).clamp(80.0, 400.0);
            return DragResult::ResizeTrackTray(self.track_tray_width);
        }
        if let Some(i) = self.dragging_window {
            let win = &mut self.mini_windows[i];
            let max_y = self.surface_config.height as f32 - TITLEBAR_HEIGHT;
            win.x = (win.x + dx).clamp(
                -(win.width - 64.0),
                self.surface_config.width as f32 - 246.0,
            );
            win.y = (win.y + dy).clamp(TITLEBAR_HEIGHT + TOOLBAR_Y, max_y);
            return DragResult::None;
        }
        if let Some(i) = self.dragging_knob {
            self.tracks[i].data.track_volume =
                (self.tracks[i].data.track_volume - dy * 0.005).clamp(0.0, 1.0);
            self.dragging = true;
            return DragResult::DragTrackVolumeKnob(i, self.tracks[i].data.track_volume);
        }
        if let Some(event_id) = self.resizing_event {
            if let Some(event) = self.events.iter_mut().find(|event| event.id == event_id) {
                self.resize_drag_accumulator += dx;
                let delta_steps = (self.resize_drag_accumulator / PLAYLIST_STEP_GAP) as i32;
                if delta_steps != 0 {
                    self.resize_drag_accumulator -= delta_steps as f32 * PLAYLIST_STEP_GAP;
                    event.length = (event.length as i32 + delta_steps).max(1) as u32;
                    return DragResult::ResizeAudioBlock(event_id, event.length);
                }
            }
            return DragResult::None;
        }
        if self.dragging {
            return DragResult::None;
        }

        // initial detection — only runs if nothing is currently active
        let tray_edge = Rectangle {
            x: self.track_tray_width - PAD_8,
            y: TOOLBAR_Y,
            width: PAD_16,
            height: self.surface_config.height as f32,
        };
        if tray_edge.is_hovered(mouse_x, mouse_y) {
            self.track_tray_width = (self.track_tray_width + dx).clamp(80.0, 400.0);
            self.resizing_track_tray = true;
            return DragResult::ResizeTrackTray(self.track_tray_width);
        }

        let sequencer_window = &self.mini_windows[SEQUENCER_ID];
        let mixer_window = &self.mini_windows[MIXER_ID];

        if self.dragging_knob.is_none() {
            // MASTER VOLUME SLIDER
            let slider_hit = Rectangle {
                x: mixer_window.x + PAD_16,
                y: mixer_window.y + mixer_window.height - 172.0,
                width: MIXER_THUMB_WIDTH,
                height: MIXER_TRACK_HEIGHT,
            };
            if slider_hit.is_hovered(mouse_x, mouse_y) {
                self.master_volume =
                    1.0 - ((mouse_y - slider_hit.y) / MIXER_TRACK_HEIGHT).clamp(0.0, 1.0);
                self.dragging = true;
                return DragResult::DragMasterVolumeSlider(self.master_volume);
            }

            for (i, track) in self.tracks.iter_mut().enumerate() {
                let slider_hit = Rectangle {
                    x: mixer_window.x
                        + PAD_16
                        + (MIXER_ITEM_WIDTH + PAD_4) * (track.data.id + 1) as f32
                        + PAD_8,
                    y: mixer_window.y + mixer_window.height - 172.0,
                    width: MIXER_THUMB_WIDTH,
                    height: MIXER_TRACK_HEIGHT,
                };
                if slider_hit.is_hovered(mouse_x, mouse_y) {
                    track.data.track_volume =
                        1.0 - ((mouse_y - slider_hit.y) / MIXER_TRACK_HEIGHT).clamp(0.0, 1.0);
                    self.dragging = true;
                    return DragResult::DragTrackVolumeSlider(i, track.data.track_volume);
                }
            }

            // TRACK VOLUME KNOB
            for (i, track) in &mut self.tracks.iter_mut().enumerate() {
                let knob_rect = Rectangle {
                    x: sequencer_window.x + KNOB_OFFSET,
                    y: sequencer_window.y + (i as f32 * TRACK_GAP) + ACTIONS_Y_OFFSET + PAD_8,
                    width: KNOB_RADIUS * 2.0,
                    height: KNOB_RADIUS * 2.0,
                };
                if knob_rect.is_hovered(mouse_x, mouse_y) {
                    self.dragging_knob = Some(i);
                    track.data.track_volume = (track.data.track_volume - dy * 0.01).clamp(0.0, 1.0);
                    self.dragging = true;
                    return DragResult::DragTrackVolumeKnob(i, track.data.track_volume);
                }
            }
        }

        // WINDOW TITLE BAR
        for (i, win) in self.mini_windows.iter().enumerate() {
            let titlebar = Rectangle {
                x: win.x,
                y: win.y - TITLEBAR_HEIGHT,
                width: win.width,
                height: TITLEBAR_HEIGHT,
            };
            if titlebar.is_hovered(mouse_x, mouse_y) {
                self.dragging_window = Some(i);
                return DragResult::None;
            }
        }

        DragResult::None
    }
}
