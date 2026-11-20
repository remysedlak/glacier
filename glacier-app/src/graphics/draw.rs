//! Main builder draw method. paints all shapes using core draw methods and handles the app's vertex buffers.
use super::*;
use crate::{
    graphics::{
        components::{modal, toolbar::TOOLBAR_MARGIN},
        font::cursor_x_offset,
        mini_window::InteractionResult,
        regions::*,
        side_panel::track_tray::file_tree,
    },
    project::{PatternID, TrackID},
};
use std::time::Duration;
use wgpu::{SurfaceTexture, TextureView};

impl Graphics {
    /// main graphics drawing method. returns interaction state
    pub fn draw(&mut self, mouse_state: &MouseState, project_is_dirty: bool) -> InteractionResult {
        let real_mouse_state = *mouse_state;
        let masked_for_modal = MouseState {
            x: f32::NEG_INFINITY,
            y: f32::NEG_INFINITY,
            left_clicked: false,
            right_clicked: false,
            left_double_clicked: false,
            ..*mouse_state
        };
        let mouse_state: &MouseState = if self.show_save_modal {
            &masked_for_modal
        } else {
            mouse_state
        };

        let menu_is_hovered = self
            .context_menu
            .as_ref()
            .map(|m| m.is_hovered(mouse_state.x, mouse_state.y))
            .unwrap_or(false);

        if mouse_state.left_clicked && !menu_is_hovered {
            let z_order = self.z_order.clone();
            for &id in z_order.iter().rev() {
                if self.mini_windows[id].is_open
                    && self.mini_windows[id].is_hovered(mouse_state.x, mouse_state.y)
                {
                    bring_to_front(&mut self.z_order, id);
                    break;
                }
            }
        }

        let click_owner: Option<usize> = if mouse_state.left_clicked && !menu_is_hovered {
            self.z_order
                .iter()
                .rev()
                .find(|&&id| {
                    self.mini_windows[id].is_open
                        && self.mini_windows[id].is_hovered(mouse_state.x, mouse_state.y)
                })
                .copied()
        } else {
            None
        };

        // rebuild interface every frame (IMGUI)
        let frame: SurfaceTexture = self
            .render_context
            .surface
            .get_current_texture()
            .expect("Failed to acquire next swap chain texture.");
        let view: TextureView = frame.texture.create_view(&TextureViewDescriptor::default());
        let screen_config = ScreenConfig {
            width: self.render_context.surface_config.width,
            height: self.render_context.surface_config.height,
            palette: &self.palette,
        };
        self.tooltip = None;
        let mut frame_draw_context = FrameDrawContext::new(
            screen_config,
            &self.font_cache,
            &self.glyph_cache,
            &self.icon_cache,
        );

        // --- mini windows ---
        for &id in &self.z_order {
            let vert_start = frame_draw_context.vertices.len() as u32;
            let char_start = frame_draw_context.char_draws.len();
            let icon_start = frame_draw_context.icon_draws.len();

            let blocked = self.context_menu.is_some() && menu_is_hovered
                || self
                    .z_order
                    .iter()
                    .skip_while(|&&z_id| z_id != id)
                    .skip(1)
                    .any(|&above_id| {
                        self.mini_windows[above_id].is_open
                            && self.mini_windows[above_id].is_hovered(mouse_state.x, mouse_state.y)
                    });

            let masked_mouse = MouseState {
                left_clicked: mouse_state.left_clicked && click_owner == Some(id),
                x: if !blocked {
                    mouse_state.x
                } else {
                    f32::NEG_INFINITY
                },
                y: if !blocked {
                    mouse_state.y
                } else {
                    f32::NEG_INFINITY
                },
                ..*mouse_state
            };

            match id {
                SEQUENCER_ID if self.mini_windows[SEQUENCER_ID].is_open => {
                    let window = &self.mini_windows[SEQUENCER_ID];
                    frame_draw_context.with_region(None, |frame| {
                        let sequencer_interaction = sequencer::draw(
                            window,
                            &self.patterns,
                            &self.sequencer_scroll_offset,
                            &mut self.tracks,
                            self.active_pattern_id,
                            self.active_step,
                            &masked_mouse,
                            frame,
                        );
                        frame.add_interaction(sequencer_interaction);
                    });
                    continue;
                }
                PLAYLIST_ID if self.mini_windows[PLAYLIST_ID].is_open => {
                    let window = &self.mini_windows[PLAYLIST_ID];
                    let playlist_interaction = playlist::draw(
                        window,
                        &self.audio_blocks,
                        &self.patterns,
                        &self.tracks,
                        &masked_mouse,
                        self.playlist_action,
                        &self.playlist_scroll_offset,
                        self.playhead_beat,
                        self.resizing_audio_block,
                        self.dragging_file.as_ref(),
                        &self.playlist_tool,
                        &mut self.waveform_cache,
                        self.stretch_mode,
                        &mut frame_draw_context,
                    );
                    frame_draw_context.add_interaction(playlist_interaction);
                    continue;
                }
                MIXER_ID if self.mini_windows[MIXER_ID].is_open => {
                    let window = &self.mini_windows[MIXER_ID];
                    frame_draw_context.with_region(None, |frame| {
                        let mixer_interaction = mixer::draw(
                            window,
                            &self.tracks,
                            self.master_volume,
                            self.master_rms_l,
                            self.master_rms_r,
                            self.master_peak,
                            &screen_config,
                            &masked_mouse,
                            frame,
                        );
                        frame.add_interaction(mixer_interaction);
                    });
                    // This region was recorded by `with_region` above.
                    continue;
                }
                PIANO_ROLL_ID if self.mini_windows[PIANO_ROLL_ID].is_open => {
                    let window = &self.mini_windows[PIANO_ROLL_ID];
                    let piano_interaction = piano_roll::window::draw(
                        window,
                        &masked_mouse,
                        &self.patterns,
                        &self.tracks,
                        self.active_step,
                        self.piano_roll_state.as_ref(),
                        &mut frame_draw_context,
                    );

                    frame_draw_context.add_interaction(piano_interaction);
                    continue;
                }
                track => {
                    let window = &self.mini_windows[track];
                    if window.is_open {
                        if let WindowKind::TrackDetail(track_id) = window.window_kind {
                            if let Some(track) = self.tracks.iter().find(|t| t.data.id == track_id)
                            {
                                frame_draw_context.with_region(None, |frame| {
                                    let track_interaction =
                                        track::draw(window, &masked_mouse, track, frame);
                                    frame.add_interaction(track_interaction);
                                });
                                continue;
                            }
                        }
                    }
                }
            }

            frame_draw_context.record_region(vert_start, char_start, icon_start, None);
        }

        // --- track tray + file tree ---
        if self.show_track_tray {
            let sw = self.render_context.surface_config.width;
            let sh = self.render_context.surface_config.height;

            let selected_track_id: Option<TrackID> = match self.playlist_action {
                PlaylistAction::PlaceFresh(AudioBlockType::Sample(id)) => Some(id),
                _ => None,
            };

            let tray_bottom = sh / 2 + 2;
            let tray_scissor = Some(safe_scissor(
                0,
                0,
                self.track_tray_width as u32,
                tray_bottom,
                sw,
                sh,
            ));
            frame_draw_context.with_region(tray_scissor, |frame| {
                side_panel::track_tray::draw(
                    frame,
                    mouse_state,
                    self.resizing_track_tray,
                    &self.tracks,
                    self.track_tray_width,
                    selected_track_id,
                );
            });

            // DIVIDER (unscissored) — no icons
            frame_draw_context.with_region(None, |frame| {
                Rectangle {
                    x: 0.0,
                    y: (screen_config.height / 2) as f32,
                    width: self.track_tray_width,
                    height: screen_config.height as f32 - (screen_config.height / 2) as f32,
                }
                .draw(
                    &screen_config,
                    screen_config.palette.neutral.shade(1),
                    NO_RADIUS,
                    frame,
                );
                let w_divider = Rectangle {
                    x: PAD_2,
                    y: (screen_config.height / 2) as f32,
                    width: self.track_tray_width - PAD_4,
                    height: 1.0,
                };
                w_divider.draw(
                    &screen_config,
                    screen_config.palette.neutral.shade(3),
                    RADIUS_4,
                    frame,
                );
                let h_divider = Rectangle {
                    x: self.track_tray_width - 1.0,
                    y: TOOLBAR_MARGIN,
                    width: 1.0,
                    height: screen_config.height as f32 - TOOLBAR_MARGIN,
                };
                h_divider.draw(
                    &screen_config,
                    screen_config.palette.neutral.shade(3),
                    NO_RADIUS,
                    frame,
                );
                use crate::graphics::side_panel::draw_title;
                frame.draw_texts(&[draw_title(
                    "File Tree",
                    (w_divider.x - 2.0, w_divider.y),
                    &screen_config,
                )]);
            });

            // FILE TREE — owns its own icons
            let divider_y = sh / 2 + (PAD_32 + PAD_16) as u32;
            let file_tree_scissor = Some(safe_scissor(
                0,
                divider_y,
                self.track_tray_width as u32,
                sh.saturating_sub(divider_y),
                sw,
                sh,
            ));
            frame_draw_context.with_region(file_tree_scissor, |frame| {
                file_tree::draw(
                    frame,
                    mouse_state,
                    &self.user_fs_location,
                    &self.expanded_dirs,
                    &self.fs_cache,
                    self.fs_scroll_offset,
                    self.track_tray_width,
                    (screen_config.height / 2) as f32,
                );
            });
        }

        // --- toolbar (pattern tray + top bar) ---
        frame_draw_context.with_region(None, |frame| {
            let sequencer_is_open = self
                .mini_windows
                .iter()
                .any(|w| matches!(w.window_kind, WindowKind::Sequencer) && w.is_open);

            if self.show_pattern_tray {
                let selected_pattern_id: Option<PatternID> = match self.playlist_action {
                    PlaylistAction::PlaceFresh(AudioBlockType::Pattern(id)) => Some(id),
                    _ => None,
                };
                let rename_cursor_offset: Option<f32> = self.renaming.as_ref().map(|r| {
                    let font = self
                        .font_cache
                        .get(Roboto)
                        .expect("Roboto font missing from cache");
                    cursor_x_offset(font, &r.edited_name, r.cursor, 14.0)
                });

                side_panel::pattern_tray::draw(
                    frame,
                    &self.patterns,
                    selected_pattern_id,
                    &mouse_state.hidden(self.context_menu.is_some() && menu_is_hovered),
                    sequencer_is_open,
                    self.pattern_tray_width,
                    &self.renaming,
                    rename_cursor_offset,
                );
            }

            if self.show_save_modal {
                modal::draw(frame, &real_mouse_state);
            }

            let total_seconds = ((self.playhead_beat / self.bpm) * 60.0) as u32;
            let time_string = format!(
                "{:02}:{:02}:{:02}",
                total_seconds / 3600,
                (total_seconds % 3600) / 60,
                total_seconds % 60
            );
            components::toolbar::draw(
                frame,
                mouse_state,
                self.bpm,
                self.is_playing,
                self.active_step,
                &self.spectrum,
                self.sample_rate,
                time_string,
            );
        });

        // --- footer ---
        Self::draw_footer(
            &self.project_path,
            self.present_fps,
            &mut frame_draw_context,
            project_is_dirty,
            mouse_state,
        );

        // dragging cursor override
        if self.resizing_track_tray {
            frame_draw_context.interaction.cursor = CursorIcon::ColResize;
        } else if self.dragging_window.is_some() || self.dragging || self.dragging_knob.is_some() {
            frame_draw_context.interaction.cursor = CursorIcon::Default;
        }

        if let Some(ref path) = self.dragging_file {
            frame_draw_context.with_region(None, |frame| {
                let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("?");
                let ghost = Rectangle {
                    x: mouse_state.x,
                    y: mouse_state.y - 16.0,
                    width: 128.0,
                    height: 32.0,
                };
                ghost.draw(
                    &screen_config,
                    screen_config.palette.neutral.shade(6),
                    RADIUS_4,
                    frame,
                );
                frame.draw_texts(&[TextItem {
                    text: name.to_string(),
                    x: ghost.x + PAD_4,
                    y: ghost.y + PAD_4,
                    size: 10.0,
                    font: Roboto,
                    color: screen_config.palette.neutral.shade(3),
                }]);
            });
        };

        // context menu
        frame_draw_context.with_region(None, |frame| {
            if let Some(menu) = &self.context_menu {
                menu.draw(frame, mouse_state);
            }
        });

        // tooltip
        let tooltip = frame_draw_context.tooltip.clone();
        frame_draw_context.with_region(None, |frame| {
            if let Some(tt) = &tooltip {
                if mouse_state
                    .hover_duration
                    .is_some_and(|t| t.elapsed() > Duration::from_millis(400))
                {
                    let _tooltip_rectangle = Rectangle::new(tt.x, tt.y, tt.width, 24.0)
                        .draw_style()
                        .draw(
                            &screen_config,
                            screen_config.palette.neutral.shade(3),
                            RADIUS_8,
                            frame,
                        );
                    if let Some(text) = &tt.text {
                        let tooltip_text = [TextItem {
                            text: text.to_string(),
                            x: tt.x + PAD_4,
                            y: tt.y + PAD_2,
                            size: 14.0,
                            font: Mono,
                            color: screen_config.palette.neutral.shade(7),
                        }];
                        frame.draw_texts(&tooltip_text);
                    }
                }
            }
        });

        self.render_context.queue.write_buffer(
            &self.render_context.vertex_buffer,
            0,
            bytemuck::cast_slice(&frame_draw_context.vertices),
        );
        self.render_context.queue.write_buffer(
            &self.render_context.glyph_vertex_buffer,
            0,
            bytemuck::cast_slice(&frame_draw_context.glyph_vertices),
        );
        self.render_context.queue.write_buffer(
            &self.render_context.icon_vertex_buffer,
            0,
            bytemuck::cast_slice(&frame_draw_context.icon_vertices),
        );
        self.num_vertices = frame_draw_context.vertices.len() as u32;

        // Encodes a series of operations to submit to the GPU at once
        let mut encoder = self
            .render_context
            .device
            .create_command_encoder(&CommandEncoderDescriptor { label: None });
        {
            let mut r_pass = encoder.begin_render_pass(&RenderPassDescriptor {
                label: None,
                color_attachments: &[Some(RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: Operations {
                        load: LoadOp::Clear(wgpu::Color {
                            r: 0.005,
                            g: 0.005,
                            b: 0.005,
                            a: 1.0,
                        }),
                        store: StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            r_pass.set_pipeline(&self.render_context.render_pipeline);
            r_pass.set_bind_group(1, &self.render_context.screen_bind_group, &[]);
            let any_bg = self.glyph_cache.any_bind_group().unwrap();

            for region in &frame_draw_context.regions {
                match region.scissor {
                    Some((x, y, w, h)) => r_pass.set_scissor_rect(x, y, w, h),
                    None => r_pass.set_scissor_rect(
                        0,
                        0,
                        self.render_context.surface_config.width,
                        self.render_context.surface_config.height,
                    ),
                }
                // draw everything that was in that draw range
                draw_range(
                    &mut r_pass,
                    &self.render_context.vertex_buffer,
                    &self.render_context.glyph_vertex_buffer,
                    &self.render_context.icon_vertex_buffer,
                    any_bg,
                    &frame_draw_context.char_draws,
                    &frame_draw_context.icon_draws,
                    self.glyph_cache.any_bind_group().unwrap(),
                    self.icon_cache.bind_group(),
                    &region.range,
                );
            }
            r_pass.set_scissor_rect(
                0,
                0,
                self.render_context.surface_config.width,
                self.render_context.surface_config.height,
            );
        }

        // Submit finished command buffer for execution.
        self.render_context.queue.submit(Some(encoder.finish()));
        frame.present();

        // The event loop uses Graphics.tooltip to start the hover-delay timer.
        self.tooltip = frame_draw_context.tooltip.clone();

        let now = std::time::Instant::now();
        if let Some(window_start) = self.fps_window_start {
            self.fps_window_frames += 1;
            let elapsed = now.duration_since(window_start);
            if elapsed >= Duration::from_secs(1) {
                self.present_fps = self.fps_window_frames as f32 / elapsed.as_secs_f32();
                self.fps_window_frames = 0;
                self.fps_window_start = Some(now);
            }
        } else {
            self.fps_window_start = Some(now);
            self.fps_window_frames = 1;
        }

        frame_draw_context.interaction
    }

    fn draw_footer<'a>(
        project_path: &str,
        present_fps: f32,
        frame: &mut FrameDrawContext<'_>,
        project_is_dirty: bool,
        mouse_state: &MouseState,
    ) {
        let title = if project_is_dirty {
            format!("{}*", project_path)
        } else {
            project_path.to_owned()
        };
        frame.with_region(None, |frame| {
            footer::draw(frame, &title, present_fps, mouse_state);
        });
    }
}
