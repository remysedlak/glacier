//! File for handling the user's click and drag actions

use crate::graphics::{
    components::{
        slider::{self, MIXER_THUMB_WIDTH, MIXER_TRACK_HEIGHT},
        toolbar::TOOLBAR_Y,
    },
    mini_window::{mixer::MIXER_ITEM_WIDTH, playlist::PLAYLIST_STEP_GAP, TITLEBAR_HEIGHT},
};

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
    ResizeTrackTray,

    //fs
    DraggingFile,
    None,
}
impl Graphics {
    /// Reset drag state of graphics thread
    pub fn clear_drag_state(&mut self) {
        self.dragging = false;
        self.dragging_window = None;
        self.dragging_knob = None;
        self.dragging_slider = None;
        self.resizing_track_tray = false;
        self.resizing_audio_block = None;
        self.dragging_file = None;
    }
    pub fn is_dragging(&self) -> bool {
        self.resizing_track_tray
            || self.dragging
            || self.dragging_window.is_some()
            || self.dragging_knob.is_some()
            || self.resizing_audio_block.is_some()
            || self.dragging_slider.is_some()
    }
    /// Track if/where the user's mouse is dragging a component
    pub fn handle_drag(&mut self, mouse_x: f32, mouse_y: f32, dy: f32, dx: f32) -> DragResult {
        if self.dragging_file.is_some() {
            return DragResult::DraggingFile;
        }
        // sticky drags — once started, keep going until mouse release
        if self.resizing_track_tray {
            self.track_tray_width = (self.track_tray_width + dx).clamp(80.0, 400.0);
            return DragResult::ResizeTrackTray;
        }

        // DRAGGING WINDOW
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

        // DRAGGING KNOB
        if let Some(track_id) = self.dragging_knob {
            if let Some(track) = self
                .tracks
                .iter_mut()
                .find(|t| t.data.id as usize == track_id)
            {
                track.data.track_volume = (track.data.track_volume - dy * 0.005).clamp(0.0, 1.0);
                self.dragging = true;
                return DragResult::DragTrackVolumeKnob(track_id, track.data.track_volume);
            }
            self.dragging = true;
            return DragResult::None;
        }

        // DRAGGING SLIDER
        if let Some(slider_target) = self.dragging_slider {
            let mixer_window = &self.mini_windows[MIXER_ID];
            let slider_y = slider::slider_y_origin(mixer_window.y, mixer_window.height);
            match slider_target {
                None => {
                    self.master_volume =
                        1.0 - ((mouse_y - slider_y) / MIXER_TRACK_HEIGHT).clamp(0.0, 1.0);
                    self.dragging_slider = Some(None);
                    self.dragging = true;
                    return DragResult::DragMasterVolumeSlider(self.master_volume);
                }
                Some(track_id) => {
                    if let Some(track) = self
                        .tracks
                        .iter_mut()
                        .find(|t| t.data.id as usize == track_id)
                    {
                        track.data.track_volume =
                            1.0 - ((mouse_y - slider_y) / MIXER_TRACK_HEIGHT).clamp(0.0, 1.0);
                        self.dragging_slider = Some(Some(track.data.id as usize));
                        self.dragging = true;
                        return DragResult::DragTrackVolumeSlider(
                            track_id,
                            track.data.track_volume,
                        );
                    }
                    self.dragging = true;
                    return DragResult::None;
                }
            }
        }

        // RESIZING EVENT
        if let Some(audio_block_id) = self.resizing_audio_block {
            if let Some(audio_block) = self
                .audio_blocks
                .iter_mut()
                .find(|audio_block| audio_block.id == audio_block_id)
            {
                self.resize_drag_accumulator += dx;
                let delta_steps = (self.resize_drag_accumulator / PLAYLIST_STEP_GAP) as i32;
                if delta_steps != 0 {
                    self.resize_drag_accumulator -= delta_steps as f32 * PLAYLIST_STEP_GAP;
                    audio_block.length = (audio_block.length as i32 + delta_steps).max(1) as u32;
                    return DragResult::ResizeAudioBlock(audio_block_id, audio_block.length);
                }
            }
            return DragResult::None;
        }
        if self.dragging {
            return DragResult::None;
        }

        // TRACK TRAY
        // initial detection — only runs if nothing is currently active
        let tray_edge = Rectangle {
            x: self.track_tray_width - PAD_8,
            y: TOOLBAR_Y,
            width: PAD_16,
            height: self.surface_config.height as f32,
        };
        if tray_edge.is_hovered(mouse_x, mouse_y) {
            self.resizing_track_tray = true;
            return DragResult::ResizeTrackTray;
        }

        let sequencer_window = &self.mini_windows[SEQUENCER_ID];
        let mixer_window = &self.mini_windows[MIXER_ID];

        // DRAGGING KNOB
        if self.dragging_knob.is_none() {
            // MASTER VOLUME SLIDER (0)
            let slider_hit = Rectangle {
                x: mixer_window.x + PAD_16,
                y: slider::slider_y_origin(mixer_window.y, mixer_window.height),
                width: MIXER_THUMB_WIDTH,
                height: MIXER_TRACK_HEIGHT,
            };
            if slider_hit.is_hovered(mouse_x, mouse_y) {
                self.master_volume =
                    1.0 - ((mouse_y - slider_hit.y) / MIXER_TRACK_HEIGHT).clamp(0.0, 1.0);
                self.dragging_slider = Some(None);
                self.dragging = true;
                return DragResult::DragMasterVolumeSlider(self.master_volume);
            }
            // TRACK VOLUME SLIDERS (1,2,3,4,5,...)
            for (i, track) in self.tracks.iter_mut().enumerate() {
                let slider_hit = Rectangle {
                    x: mixer_window.x
                        + PAD_16
                        + (MIXER_ITEM_WIDTH + PAD_4) * (i + 1) as f32
                        + PAD_8,
                    y: slider::slider_y_origin(mixer_window.y, mixer_window.height),
                    width: MIXER_THUMB_WIDTH,
                    height: MIXER_TRACK_HEIGHT,
                };
                if slider_hit.is_hovered(mouse_x, mouse_y) {
                    track.data.track_volume =
                        1.0 - ((mouse_y - slider_hit.y) / MIXER_TRACK_HEIGHT).clamp(0.0, 1.0);
                    self.dragging_slider = Some(Some(track.data.id as usize));
                    self.dragging = true;
                    return DragResult::DragTrackVolumeSlider(
                        track.data.id as usize,
                        track.data.track_volume,
                    );
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
                    self.dragging_knob = Some(track.data.id as usize);
                    track.data.track_volume = (track.data.track_volume - dy * 0.01).clamp(0.0, 1.0);
                    self.dragging = true;
                    return DragResult::DragTrackVolumeKnob(
                        track.data.id as usize,
                        track.data.track_volume,
                    );
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
