use super::*;
use std::time::Duration;

impl Graphics {
    /// pushing texts to draw
    fn push_text_draws<'a>(
        texts: &[TextItem],
        font_cache: &HashMap<String, fontdue::Font>,
        glyph_cache: &'a GlyphCache,
        device: &wgpu::Device,
        screen_config: &ScreenConfig,
        char_draws: &mut Vec<(wgpu::Buffer, &'a wgpu::BindGroup)>,
    ) {
        for text_item in texts {
            let Some(font) = font_cache.get(text_item.font) else {
                continue;
            };
            let mut layout = Layout::new(CoordinateSystem::PositiveYDown);
            let Color { r, g, b } = text_item.color;
            layout.append(&[font], &TextStyle::new(&text_item.text, text_item.size, 0));
            for glyph in layout.glyphs() {
                if let Some(entry) =
                    glyph_cache.get(text_item.font, glyph.parent, text_item.size as u32)
                {
                    let gverts = font::draw_glyph(
                        text_item.x + glyph.x,
                        text_item.y + glyph.y,
                        glyph.width as f32,
                        glyph.height as f32,
                        screen_config,
                        (r, g, b),
                    );
                    let buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                        label: None,
                        contents: bytemuck::cast_slice(&gverts),
                        usage: wgpu::BufferUsages::VERTEX,
                    });
                    char_draws.push((buf, entry.bind_group()));
                }
            }
        }
    }

    /// Clamps a scissor rect, so that pain never goes outside of the screen bounds
    /// - clamps x and y, so they don't exceed the screen's edges
    /// - clamps w and h, so that the rectangle doesn't extend the right/bottom edge
    /// - also ensures that w and h are at least 1 so wgpu doesn't get a zero-size scissor (error)
    fn safe_scissor(x: u32, y: u32, w: u32, h: u32, sw: u32, sh: u32) -> (u32, u32, u32, u32) {
        let x = x.min(sw.saturating_sub(1));
        let y = y.min(sh.saturating_sub(1));
        let w = w.min(sw.saturating_sub(x)).max(1);
        let h = h.min(sh.saturating_sub(y)).max(1);
        (x, y, w, h)
    }
    //  helper function to draw a list of vertices with the same bind group (for shapes with the same color or texture)
    fn draw_geom(
        r_pass: &mut wgpu::RenderPass,
        vertex_buffer: &wgpu::Buffer,
        any_bg: &wgpu::BindGroup,
        start: u32,
        end: u32,
    ) {
        if start < end {
            r_pass.set_bind_group(0, any_bg, &[]);
            r_pass.set_vertex_buffer(0, vertex_buffer.slice(..));
            r_pass.draw(start..end, 0..1);
        }
    }

    // draw a list of characters, each with their own texture and bind group, so they can be different colors and fonts
    fn draw_chars(
        r_pass: &mut wgpu::RenderPass,
        char_draws: &[(wgpu::Buffer, &wgpu::BindGroup)],
        start: usize,
        end: usize,
    ) {
        for char in char_draws.iter().skip(start).take(end) {
            r_pass.set_bind_group(0, char.1, &[]);
            r_pass.set_vertex_buffer(0, char.0.slice(..));
            r_pass.draw(0..6, 0..1);
        }
    }
    /// main draw loop for the GUI - uses mouse state to return mouse input interactivity
    pub fn draw(
        &mut self,
        mouse_state: &MouseState,
        project_is_dirty: bool,
    ) -> (ClickResult, CursorIcon) {
        let frame = self
            .surface
            .get_current_texture()
            .expect("Failed to acquire next swap chain texture.");
        let view = frame.texture.create_view(&TextureViewDescriptor::default());
        let mut vertices: Vec<Vertex> = Vec::new();
        self.tooltip = None;
        let mut click_result = ClickResult::None;
        let mut cursor_icon = CursorIcon::Default;

        // custom struct to hold screen size
        let screen_config = ScreenConfig {
            width: self.surface_config.width,
            height: self.surface_config.height,
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

        let mut char_draws: Vec<(wgpu::Buffer, &wgpu::BindGroup)> = Vec::new();
        let mut icon_draws: Vec<(wgpu::Buffer, &wgpu::BindGroup)> = Vec::new();
        let mut window_ranges: Vec<WindowDrawRange> = Vec::new();
        let mut playlist_window_ranges: Option<PlaylistDrawRanges> = None;
        let mut piano_roll_ranges: Option<PianoRollDrawRanges> = None;
        for &id in &self.z_order {
            let vert_start = vertices.len() as u32;
            let char_start = char_draws.len();

            // is there any open window above this one that also covers the mouse?
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
                    let (texts, icons, result, cursor) = sequencer::draw(
                        window,
                        &mut self.patterns,
                        &self.sequencer_scroll_offset,
                        &mut self.tracks,
                        self.active_pattern_id,
                        self.active_step,
                        &masked_mouse,
                        &screen_config,
                        &mut vertices,
                    );

                    Graphics::push_text_draws(
                        &texts,
                        &self.font_cache,
                        &self.glyph_cache,
                        &self.device,
                        &screen_config,
                        &mut char_draws,
                    );
                    if cursor != CursorIcon::Default {
                        cursor_icon = cursor;
                    }

                    for icon in icons {
                        push_icon_draw(
                            &self.icon_cache,
                            &self.device,
                            &screen_config,
                            &icon,
                            &mut icon_draws,
                        )
                    }

                    click_result = click_result.or(result);
                }
                PLAYLIST_ID if self.mini_windows[PLAYLIST_ID].is_open => {
                    let window = &self.mini_windows[PLAYLIST_ID];
                    let (
                        static_draw_region,
                        timeline_draw_region,
                        header_draw_region,
                        result,
                        cursor,
                    ) = playlist::draw(
                        window,
                        &self.events,
                        &self.patterns,
                        &masked_mouse,
                        self.active_pattern_id,
                        &self.playlist_scroll_offset,
                        self.active_step,
                        self.resizing_event,
                        &screen_config,
                    );

                    let static_vert_start = vertices.len() as u32;
                    let static_char_start = char_draws.len();
                    vertices.extend(static_draw_region.vertices);
                    Graphics::push_text_draws(
                        &static_draw_region.text_items,
                        &self.font_cache,
                        &self.glyph_cache,
                        &self.device,
                        &screen_config,
                        &mut char_draws,
                    );
                    let static_range = WindowDrawRange {
                        vert_start: static_vert_start,
                        vert_end: vertices.len() as u32,
                        char_start: static_char_start,
                        char_end: char_draws.len(),
                    };

                    let header_vert_start = vertices.len() as u32;
                    let header_char_start = char_draws.len();
                    vertices.extend(header_draw_region.vertices);
                    Graphics::push_text_draws(
                        &header_draw_region.text_items,
                        &self.font_cache,
                        &self.glyph_cache,
                        &self.device,
                        &screen_config,
                        &mut char_draws,
                    );
                    let header_range = WindowDrawRange {
                        vert_start: header_vert_start,
                        vert_end: vertices.len() as u32,
                        char_start: header_char_start,
                        char_end: char_draws.len(),
                    };

                    let timeline_vert_start = vertices.len() as u32;
                    let timeline_char_start = char_draws.len();
                    vertices.extend(timeline_draw_region.vertices);
                    Graphics::push_text_draws(
                        &timeline_draw_region.text_items,
                        &self.font_cache,
                        &self.glyph_cache,
                        &self.device,
                        &screen_config,
                        &mut char_draws,
                    );
                    let timeline_range = WindowDrawRange {
                        vert_start: timeline_vert_start,
                        vert_end: vertices.len() as u32,
                        char_start: timeline_char_start,
                        char_end: char_draws.len(),
                    };

                    playlist_window_ranges = Some(PlaylistDrawRanges {
                        static_range,
                        header_range,
                        timeline_range,
                    });
                    if cursor != CursorIcon::Default {
                        cursor_icon = cursor;
                    }
                    click_result = click_result.or(result);
                }
                MIXER_ID if self.mini_windows[MIXER_ID].is_open => {
                    let window = &self.mini_windows[MIXER_ID];
                    let (texts, result, _cursor) = mixer::draw(
                        window,
                        &self.tracks,
                        self.master_volume,
                        self.master_rms_l,
                        self.master_rms_r,
                        self.master_peak,
                        &screen_config,
                        &masked_mouse,
                        &mut vertices,
                    );

                    Graphics::push_text_draws(
                        &texts,
                        &self.font_cache,
                        &self.glyph_cache,
                        &self.device,
                        &screen_config,
                        &mut char_draws,
                    );
                    click_result = click_result.or(result);
                }
                PIANO_ROLL_ID if self.mini_windows[PIANO_ROLL_ID].is_open => {
                    let window = &self.mini_windows[PIANO_ROLL_ID];
                    let (
                        static_draw_region,
                        piano_key_draw_region,
                        grid_draw_region,
                        result,
                        cursor,
                    ) = piano_roll::window::draw(
                        window,
                        &masked_mouse,
                        &screen_config,
                        &self.patterns,
                        &self.tracks,
                        self.active_step,
                        self.piano_roll_state.as_ref(),
                    );

                    // static (titlebar + background) — no scroll
                    vertices.extend(static_draw_region.vertices);
                    Graphics::push_text_draws(
                        &static_draw_region.text_items,
                        &self.font_cache,
                        &self.glyph_cache,
                        &self.device,
                        &screen_config,
                        &mut char_draws,
                    );

                    // scrollable content
                    let piano_content_vert_start = vertices.len() as u32;
                    let piano_content_char_start = char_draws.len();
                    vertices.extend(piano_key_draw_region.vertices);
                    Graphics::push_text_draws(
                        &piano_key_draw_region.text_items,
                        &self.font_cache,
                        &self.glyph_cache,
                        &self.device,
                        &screen_config,
                        &mut char_draws,
                    );

                    // grid content
                    let grid_vert_start = vertices.len() as u32;
                    let grid_char_start = char_draws.len();
                    vertices.extend(grid_draw_region.vertices);
                    Graphics::push_text_draws(
                        &grid_draw_region.text_items,
                        &self.font_cache,
                        &self.glyph_cache,
                        &self.device,
                        &screen_config,
                        &mut char_draws,
                    );
                    //
                    piano_roll_ranges = Some(PianoRollDrawRanges {
                        static_range: WindowDrawRange {
                            vert_start,
                            vert_end: piano_content_vert_start,
                            char_start,
                            char_end: piano_content_char_start,
                        },
                        piano_range: WindowDrawRange {
                            vert_start: piano_content_vert_start,
                            vert_end: grid_vert_start, // stop here
                            char_start: piano_content_char_start,
                            char_end: grid_char_start, // stop here
                        },
                        grid_range: WindowDrawRange {
                            vert_start: grid_vert_start,
                            vert_end: vertices.len() as u32,
                            char_start: grid_char_start,
                            char_end: char_draws.len(),
                        },
                    });

                    click_result = click_result.or(result);
                    if cursor != CursorIcon::Default {
                        cursor_icon = cursor;
                    }
                }
                track => {
                    let window = &self.mini_windows[track];
                    if window.is_open {
                        if let WindowKind::TrackDetail(track) = window.window_kind {
                            let (texts, icons, result, cursor, tooltip) = track::draw(
                                window,
                                &masked_mouse,
                                &screen_config,
                                &self.tracks[track],
                                &mut vertices,
                            );

                            click_result = click_result.or(result);
                            if !matches!(cursor, CursorIcon::Default) {
                                cursor_icon = cursor;
                            }
                            for icon in icons {
                                push_icon_draw(
                                    &self.icon_cache,
                                    &self.device,
                                    &screen_config,
                                    &icon,
                                    &mut icon_draws,
                                )
                            }
                            Graphics::push_text_draws(
                                &texts,
                                &self.font_cache,
                                &self.glyph_cache,
                                &self.device,
                                &screen_config,
                                &mut char_draws,
                            );
                            self.tooltip = tooltip;
                        }
                    }
                }
            }

            window_ranges.push(WindowDrawRange {
                vert_start,
                vert_end: vertices.len() as u32,
                char_start,
                char_end: char_draws.len(),
            });
        }

        // --- toolbar (pattern tray + toolbar bar) ---
        let toolbar_vert_start = vertices.len() as u32;
        let toolbar_char_start = char_draws.len();

        let sequencer_is_open = self
            .mini_windows
            .iter()
            .any(|w| matches!(w.window_kind, WindowKind::Sequencer) && w.is_open);

        // tray of project patterns
        if self.show_pattern_tray {
            let (texts, result, cursor, icon, tooltip) = side_panel::pattern_tray::draw(
                &screen_config,
                &self.patterns,
                self.active_pattern_id,
                mouse_state,
                sequencer_is_open,
                &mut vertices,
            );

            if cursor != CursorIcon::Default {
                cursor_icon = cursor;
            }
            click_result = click_result.or(result);
            Graphics::push_text_draws(
                &texts,
                &self.font_cache,
                &self.glyph_cache,
                &self.device,
                &screen_config,
                &mut char_draws,
            );
            push_icon_draw(
                &self.icon_cache,
                &self.device,
                &screen_config,
                &icon,
                &mut icon_draws,
            );
            self.tooltip = tooltip
        }

        // tray of audio files / folders
        if self.show_track_tray {
            let (texts, icons, result, cursor) = side_panel::track_tray::draw(
                mouse_state,
                &screen_config,
                &self.tracks,
                &self.user_fs_location,
                &self.expanded_dirs,
                &mut vertices,
            );
            for icon in icons {
                push_icon_draw(
                    &self.icon_cache,
                    &self.device,
                    &screen_config,
                    &icon,
                    &mut icon_draws,
                )
            }

            if cursor != CursorIcon::Default {
                cursor_icon = cursor;
            }
            click_result = click_result.or(result);
            Graphics::push_text_draws(
                &texts,
                &self.font_cache,
                &self.glyph_cache,
                &self.device,
                &screen_config,
                &mut char_draws,
            );
        }

        if self.show_save_modal {
            let (verts, texts) = modal::draw(&screen_config);
            vertices.extend(verts);
            Graphics::push_text_draws(
                &texts,
                &self.font_cache,
                &self.glyph_cache,
                &self.device,
                &screen_config,
                &mut char_draws,
            );
        }

        // top toolbar
        // 00:00:00
        let total_seconds = ((self.playhead_beat / self.bpm) * 60.0) as u32;
        let hours = total_seconds / 3600;
        let minutes = (total_seconds % 3600) / 60;
        let seconds = total_seconds % 60;
        let time_string = format!("{:02}:{:02}:{:02}", hours, minutes, seconds);
        let (texts, icons, result, cursor, tooltip) = components::toolbar::draw_toolbar(
            mouse_state,
            &screen_config,
            self.bpm,
            self.is_playing,
            self.active_step,
            time_string,
            &mut vertices,
        );

        click_result = click_result.or(result);
        if cursor != CursorIcon::Default {
            cursor_icon = cursor;
        }

        for icon in icons {
            push_icon_draw(
                &self.icon_cache,
                &self.device,
                &screen_config,
                &icon,
                &mut icon_draws,
            )
        }

        self.tooltip = tooltip;
        let tooltip_vert_start = vertices.len() as u32;
        let tooltip_char_start = char_draws.len();
        // tool tip

        if let Some(tt) = &self.tooltip {
            if mouse_state
                .hover_state
                .map_or(false, |t| t.elapsed() > Duration::from_millis(400))
            {
                let tooltip_rectangle = Rectangle {
                    x: tt.x,
                    y: tt.y,
                    width: 128.0,
                    height: 24.0,
                };
                tooltip_rectangle.draw(&screen_config, DARK_GRAY, RADIUS_8, &mut vertices);
                if let Some(text) = tt.text {
                    let tooltip_text = [TextItem {
                        text: text.to_string(),
                        x: tt.x + PAD_4,
                        y: tt.y + PAD_2,
                        size: 14.0,
                        font: MONOSPACED,
                        color: WHITE,
                    }];
                    Graphics::push_text_draws(
                        &tooltip_text,
                        &self.font_cache,
                        &self.glyph_cache,
                        &self.device,
                        &screen_config,
                        &mut char_draws,
                    );
                }
            }
        }

        let tooltip_vert_end = vertices.len() as u32;
        let tooltip_char_end = char_draws.len();

        Graphics::push_text_draws(
            &texts,
            &self.font_cache,
            &self.glyph_cache,
            &self.device,
            &screen_config,
            &mut char_draws,
        );

        let toolbar_vert_end = vertices.len() as u32;
        let toolbar_char_end = char_draws.len();

        // --- context menu (above toolbar) ---
        let context_menu_vert_start = vertices.len() as u32;
        let context_menu_char_start = char_draws.len();

        if let Some(menu) = &self.context_menu {
            let (texts, result, cursor) = menu.draw(&screen_config, mouse_state, &mut vertices);

            Graphics::push_text_draws(
                &texts,
                &self.font_cache,
                &self.glyph_cache,
                &self.device,
                &screen_config,
                &mut char_draws,
            );

            if cursor != CursorIcon::Default {
                cursor_icon = cursor;
            }
            click_result = click_result.or(result);
        }

        let context_menu_vert_end = vertices.len() as u32;
        let context_menu_char_end = char_draws.len();

        // --- footer ---
        let footer_vert_start = vertices.len() as u32;
        let footer_char_start = char_draws.len();

        let title = if project_is_dirty {
            format!("{}*", self.project_path)
        } else {
            self.project_path.clone()
        };
        let texts = footer::draw(
            &screen_config,
            &title,
            1000.0 / self.frame_ms,
            &mut vertices,
        );

        Graphics::push_text_draws(
            &texts,
            &self.font_cache,
            &self.glyph_cache,
            &self.device,
            &screen_config,
            &mut char_draws,
        );

        let footer_vert_end = vertices.len() as u32;
        let footer_char_end = char_draws.len();

        if mouse_state.left_click_held {
            cursor_icon = CursorIcon::Default
        }

        self.queue
            .write_buffer(&self.vertex_buffer, 0, bytemuck::cast_slice(&vertices));
        self.num_vertices = vertices.len() as u32;

        let mut encoder = self
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

            r_pass.set_pipeline(&self.render_pipeline);
            let any_bg = self.glyph_cache.any_bind_group().unwrap();

            // windows
            for (idx, range) in window_ranges.iter().enumerate() {
                let is_playlist = self.z_order[idx] == PLAYLIST_ID;
                let is_piano = self.z_order[idx] == PIANO_ROLL_ID;

                if is_piano {
                    if let Some(ref pr) = piano_roll_ranges {
                        let win = &self.mini_windows[PIANO_ROLL_ID];
                        let sw = self.surface_config.width;
                        let sh = self.surface_config.height;

                        let wx = (win.x.max(0.0) as u32).min(sw);
                        let wy = ((win.y - TITLEBAR_HEIGHT).max(0.0) as u32).min(sh);
                        let win_right = ((win.x + win.width) as u32).min(sw);
                        let win_bottom = ((win.y + win.height) as u32).min(sh);
                        let ww = win_right.saturating_sub(wx);
                        let wh = win_bottom.saturating_sub(wy);

                        let content_y = (win.y as u32 + 72).min(sh);
                        let content_h = win_bottom.saturating_sub(content_y).saturating_sub(32);

                        let key_col_right = (win.x + 72.0).max(0.0) as u32;
                        let grid_x = key_col_right.min(sw);
                        let key_w = grid_x.saturating_sub(wx);
                        let grid_w = win_right.saturating_sub(grid_x).saturating_sub(16);

                        let (sx, sy, sw2, sh2) = Self::safe_scissor(wx, wy, ww, wh, sw, sh);
                        r_pass.set_scissor_rect(sx, sy, sw2, sh2);
                        Graphics::draw_geom(
                            &mut r_pass,
                            &self.vertex_buffer,
                            any_bg,
                            pr.static_range.vert_start,
                            pr.static_range.vert_end,
                        );
                        Graphics::draw_chars(
                            &mut r_pass,
                            &char_draws,
                            pr.static_range.char_start,
                            pr.static_range.char_end,
                        );

                        let (sx, sy, sw2, sh2) =
                            Self::safe_scissor(wx, content_y, key_w, content_h, sw, sh);
                        r_pass.set_scissor_rect(sx, sy, sw2, sh2);
                        Graphics::draw_geom(
                            &mut r_pass,
                            &self.vertex_buffer,
                            any_bg,
                            pr.piano_range.vert_start,
                            pr.piano_range.vert_end,
                        );
                        Graphics::draw_chars(
                            &mut r_pass,
                            &char_draws,
                            pr.piano_range.char_start,
                            pr.piano_range.char_end,
                        );

                        let (sx, sy, sw2, sh2) =
                            Self::safe_scissor(grid_x, content_y, grid_w, content_h, sw, sh);
                        r_pass.set_scissor_rect(sx, sy, sw2, sh2);
                        Graphics::draw_geom(
                            &mut r_pass,
                            &self.vertex_buffer,
                            any_bg,
                            pr.grid_range.vert_start,
                            pr.grid_range.vert_end,
                        );
                        Graphics::draw_chars(
                            &mut r_pass,
                            &char_draws,
                            pr.grid_range.char_start,
                            pr.grid_range.char_end,
                        );
                    }
                    r_pass.set_scissor_rect(
                        0,
                        0,
                        self.surface_config.width,
                        self.surface_config.height,
                    );
                    continue;
                }

                if is_playlist {
                    if let Some(ref pl) = playlist_window_ranges {
                        let win = &self.mini_windows[PLAYLIST_ID];
                        let sw = self.surface_config.width;
                        let sh = self.surface_config.height;

                        let wx = (win.x.max(0.0) as u32).min(sw);
                        let wy = ((win.y - TITLEBAR_HEIGHT).max(0.0) as u32).min(sh);
                        let win_right = ((win.x + win.width) as u32).min(sw);
                        let win_bottom = ((win.y + win.height) as u32).min(sh);
                        let ww = win_right.saturating_sub(wx);
                        let wh = win_bottom.saturating_sub(wy);

                        let content_y = (win.y as u32 + 64).min(sh);
                        let content_h = win_bottom.saturating_sub(content_y);
                        let header_x = ((win.x + 144.0).max(0.0) as u32).min(sw);
                        let header_w = header_x.saturating_sub(wx);
                        let timeline_w = win_right.saturating_sub(header_x);

                        let (sx, sy, sw2, sh2) = Self::safe_scissor(wx, wy, ww, wh, sw, sh);
                        r_pass.set_scissor_rect(sx, sy, sw2, sh2);
                        Graphics::draw_geom(
                            &mut r_pass,
                            &self.vertex_buffer,
                            any_bg,
                            pl.static_range.vert_start,
                            pl.static_range.vert_end,
                        );
                        Graphics::draw_chars(
                            &mut r_pass,
                            &char_draws,
                            pl.static_range.char_start,
                            pl.static_range.char_end,
                        );

                        let (sx, sy, sw2, sh2) =
                            Self::safe_scissor(wx, content_y, header_w, content_h, sw, sh);
                        r_pass.set_scissor_rect(sx, sy, sw2, sh2);
                        Graphics::draw_geom(
                            &mut r_pass,
                            &self.vertex_buffer,
                            any_bg,
                            pl.header_range.vert_start,
                            pl.header_range.vert_end,
                        );
                        Graphics::draw_chars(
                            &mut r_pass,
                            &char_draws,
                            pl.header_range.char_start,
                            pl.header_range.char_end,
                        );

                        let (sx, sy, sw2, sh2) =
                            Self::safe_scissor(header_x, content_y, timeline_w, content_h, sw, sh);
                        r_pass.set_scissor_rect(sx, sy, sw2, sh2);
                        Graphics::draw_geom(
                            &mut r_pass,
                            &self.vertex_buffer,
                            any_bg,
                            pl.timeline_range.vert_start,
                            pl.timeline_range.vert_end,
                        );
                        Graphics::draw_chars(
                            &mut r_pass,
                            &char_draws,
                            pl.timeline_range.char_start,
                            pl.timeline_range.char_end,
                        );
                    }
                    r_pass.set_scissor_rect(
                        0,
                        0,
                        self.surface_config.width,
                        self.surface_config.height,
                    );
                    continue;
                }
                if range.vert_start < range.vert_end {
                    r_pass.set_bind_group(0, any_bg, &[]);
                    r_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
                    r_pass.draw(range.vert_start..range.vert_end, 0..1);
                }
                for char in char_draws
                    .iter()
                    .skip(range.char_start)
                    .take(range.char_end)
                {
                    r_pass.set_bind_group(0, char.1, &[]);
                    r_pass.set_vertex_buffer(0, char.0.slice(..));
                    r_pass.draw(0..6, 0..1);
                }
            }

            // toolbar

            Graphics::draw_geom(
                &mut r_pass,
                &self.vertex_buffer,
                any_bg,
                toolbar_vert_start,
                toolbar_vert_end,
            );
            Graphics::draw_chars(
                &mut r_pass,
                &char_draws,
                toolbar_char_start,
                toolbar_char_end,
            );

            for icon in &icon_draws {
                r_pass.set_bind_group(0, icon.1, &[]);
                r_pass.set_vertex_buffer(0, icon.0.slice(..));
                r_pass.draw(0..6, 0..1);
            }
            // tooltip
            Graphics::draw_geom(
                &mut r_pass,
                &self.vertex_buffer,
                any_bg,
                tooltip_vert_start,
                tooltip_vert_end,
            );
            Graphics::draw_chars(
                &mut r_pass,
                &char_draws,
                tooltip_char_start,
                tooltip_char_end,
            );

            // context menu
            Graphics::draw_geom(
                &mut r_pass,
                &self.vertex_buffer,
                any_bg,
                context_menu_vert_start,
                context_menu_vert_end,
            );
            Graphics::draw_chars(
                &mut r_pass,
                &char_draws,
                context_menu_char_start,
                context_menu_char_end,
            );

            // footer
            Graphics::draw_geom(
                &mut r_pass,
                &self.vertex_buffer,
                any_bg,
                footer_vert_start,
                footer_vert_end,
            );
            Graphics::draw_chars(&mut r_pass, &char_draws, footer_char_start, footer_char_end);
        }

        self.queue.submit(Some(encoder.finish()));
        frame.present();

        (click_result, cursor_icon)
    }
}
