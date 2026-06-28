use super::*;
use crate::graphics::{
    color::{DARK_GRAY_HOVER, LIGHT_GRAY, LL_GRAY, PEBBLE},
    components::modal,
};
use std::time::Duration;

impl Graphics {
    /// converts TextItem into vertices.
    fn push_text_draws<'a>(
        texts: &[TextItem],
        font_cache: &HashMap<String, fontdue::Font>,
        glyph_cache: &'a GlyphCache,
        screen_config: &ScreenConfig,
        glyph_vertices: &mut Vec<Vertex>,
        char_draws: &mut Vec<(u64, &'a wgpu::BindGroup)>,
    ) {
        for text_item in texts {
            // Looks up the font by name
            let Some(font) = font_cache.get(text_item.font) else {
                continue;
            };
            // Use fontdue's Layout to compute where each character glyph should be positioned
            let mut layout = Layout::new(CoordinateSystem::PositiveYDown);
            let Color { r, g, b } = text_item.color;
            layout.append(&[font], &TextStyle::new(&text_item.text, text_item.size, 0));

            // For each glyph, looks up the pre-rasterized texture in glyph_cache
            for glyph in layout.glyphs() {
                if let Some(entry) =
                    glyph_cache.get(text_item.font, glyph.parent, text_item.size as u32)
                {
                    // generate 6 vertices (a quad) for that glyph at the correct screen position
                    let gverts = font::draw_glyph(
                        text_item.x + glyph.x,
                        text_item.y + glyph.y,
                        glyph.width as f32,
                        glyph.height as f32,
                        screen_config,
                        (r, g, b),
                    );

                    // Pushes those vertices into glyph_vertices'
                    //
                    // Records the byte offset + bind group (the glyph texture) into char_draws
                    // so the render pass knows which texture to use when drawing that glyph
                    let offset = (glyph_vertices.len() * std::mem::size_of::<Vertex>()) as u64;
                    glyph_vertices.extend_from_slice(&gverts);
                    char_draws.push((offset, entry.bind_group()));
                }
            }
        }
    }

    /// Clamps a scissor rect so paint never goes outside screen bounds
    fn safe_scissor(x: u32, y: u32, w: u32, h: u32, sw: u32, sh: u32) -> (u32, u32, u32, u32) {
        let x = x.min(sw.saturating_sub(1));
        let y = y.min(sh.saturating_sub(1));
        let w = w.min(sw.saturating_sub(x)).max(1);
        let h = h.min(sh.saturating_sub(y)).max(1);
        (x, y, w, h)
    }

    /// Draw a range of colored/textured geometry quads
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

    /// Draw a range of glyph quads, each with its own bind group
    fn draw_chars(
        r_pass: &mut wgpu::RenderPass,
        glyph_vertex_buffer: &wgpu::Buffer,
        char_draws: &[(u64, &wgpu::BindGroup)],
        start: usize,
        end: usize,
    ) {
        let stride = (6 * std::mem::size_of::<Vertex>()) as u64;
        for (offset, bg) in char_draws.iter().skip(start).take(end - start) {
            r_pass.set_bind_group(0, *bg, &[]);
            r_pass.set_vertex_buffer(0, glyph_vertex_buffer.slice(*offset..*offset + stride));
            r_pass.draw(0..6, 0..1);
        }
    }

    /// Draw geometry + glyphs for a WindowDrawRange in one call
    fn draw_range(
        r_pass: &mut wgpu::RenderPass,
        vertex_buffer: &wgpu::Buffer,
        glyph_vertex_buffer: &wgpu::Buffer,
        any_bg: &wgpu::BindGroup,
        char_draws: &[(u64, &wgpu::BindGroup)],
        range: &WindowDrawRange,
    ) {
        Self::draw_geom(
            r_pass,
            vertex_buffer,
            any_bg,
            range.vert_start,
            range.vert_end,
        );
        Self::draw_chars(
            r_pass,
            glyph_vertex_buffer,
            char_draws,
            range.char_start,
            range.char_end,
        );
    }

    pub fn draw(
        &mut self,
        mouse_state: &MouseState,
        project_is_dirty: bool,
    ) -> (ClickResult, CursorIcon) {
        // SETUP OBJECTS
        let frame = self
            .surface
            .get_current_texture()
            .expect("Failed to acquire next swap chain texture.");
        let view = frame.texture.create_view(&TextureViewDescriptor::default());
        let mut vertices: Vec<Vertex> = Vec::new();
        self.tooltip = None;
        let mut click_result = ClickResult::None;
        let mut cursor_icon = CursorIcon::Default;
        let screen_config = ScreenConfig {
            width: self.surface_config.width,
            height: self.surface_config.height,
        };

        let menu_is_hovered = self
            .context_menu
            .as_ref()
            .map(|m| m.is_hovered(mouse_state.x, mouse_state.y))
            .unwrap_or(false);

        // If a user clicks on a mini window out of view, bring it to the front of the z stack
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

        let mut glyph_vertices: Vec<Vertex> = Vec::new();
        let mut char_draws: Vec<(u64, &wgpu::BindGroup)> = Vec::new();
        let mut icon_draws: Vec<(wgpu::Buffer, &wgpu::BindGroup)> = Vec::new();
        let mut window_ranges: Vec<WindowDrawRange> = Vec::new();
        let mut playlist_window_ranges: Option<PlaylistDrawRanges> = None;
        let mut piano_roll_ranges: Option<PianoRollDrawRanges> = None;

        // --- mini windows ---
        for &id in &self.z_order {
            //
            let vert_start = vertices.len() as u32;
            let char_start = char_draws.len();

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

            // match each z ID to a window ID
            match id {
                // draw the sequencer window
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
                        &screen_config,
                        &mut glyph_vertices,
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
                // draw the playlist window
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
                        &self.tracks,
                        &masked_mouse,
                        &self.active_tray,
                        &self.playlist_scroll_offset,
                        self.active_step,
                        self.resizing_event,
                        self.dragging_file.as_ref(),
                        &screen_config,
                    );

                    let static_vert_start = vertices.len() as u32;
                    let static_char_start = char_draws.len();
                    vertices.extend(static_draw_region.vertices);
                    Graphics::push_text_draws(
                        &static_draw_region.text_items,
                        &self.font_cache,
                        &self.glyph_cache,
                        &screen_config,
                        &mut glyph_vertices,
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
                        &screen_config,
                        &mut glyph_vertices,
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
                        &screen_config,
                        &mut glyph_vertices,
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
                // draw the mixer window
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
                        &screen_config,
                        &mut glyph_vertices,
                        &mut char_draws,
                    );
                    click_result = click_result.or(result);
                }
                // draw the piano roll window
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

                    vertices.extend(static_draw_region.vertices);
                    Graphics::push_text_draws(
                        &static_draw_region.text_items,
                        &self.font_cache,
                        &self.glyph_cache,
                        &screen_config,
                        &mut glyph_vertices,
                        &mut char_draws,
                    );

                    let piano_content_vert_start = vertices.len() as u32;
                    let piano_content_char_start = char_draws.len();
                    vertices.extend(piano_key_draw_region.vertices);
                    Graphics::push_text_draws(
                        &piano_key_draw_region.text_items,
                        &self.font_cache,
                        &self.glyph_cache,
                        &screen_config,
                        &mut glyph_vertices,
                        &mut char_draws,
                    );

                    let grid_vert_start = vertices.len() as u32;
                    let grid_char_start = char_draws.len();
                    vertices.extend(grid_draw_region.vertices);
                    Graphics::push_text_draws(
                        &grid_draw_region.text_items,
                        &self.font_cache,
                        &self.glyph_cache,
                        &screen_config,
                        &mut glyph_vertices,
                        &mut char_draws,
                    );

                    piano_roll_ranges = Some(PianoRollDrawRanges {
                        static_range: WindowDrawRange {
                            vert_start,
                            vert_end: piano_content_vert_start,
                            char_start,
                            char_end: piano_content_char_start,
                        },
                        piano_range: WindowDrawRange {
                            vert_start: piano_content_vert_start,
                            vert_end: grid_vert_start,
                            char_start: piano_content_char_start,
                            char_end: grid_char_start,
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
                /// draw the track details window
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
                                &screen_config,
                                &mut glyph_vertices,
                                &mut char_draws,
                            );
                            self.tooltip = tooltip;
                        }
                    }
                }
            }

            // position of vertexes and texts in the buffers
            window_ranges.push(WindowDrawRange {
                vert_start,
                vert_end: vertices.len() as u32,
                char_start,
                char_end: char_draws.len(),
            });
        }

        // --- toolbar (pattern tray + top bar) ---
        let toolbar_vert_start = vertices.len() as u32;
        let toolbar_char_start = char_draws.len();

        let sequencer_is_open = self
            .mini_windows
            .iter()
            .any(|w| matches!(w.window_kind, WindowKind::Sequencer) && w.is_open);

        if self.show_pattern_tray {
            let (texts, result, cursor, icon, tooltip) = side_panel::pattern_tray::draw(
                &screen_config,
                &self.patterns,
                self.active_pattern_id,
                mouse_state,
                sequencer_is_open,
                self.pattern_tray_width,
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
                &screen_config,
                &mut glyph_vertices,
                &mut char_draws,
            );
            push_icon_draw(
                &self.icon_cache,
                &self.device,
                &screen_config,
                &icon,
                &mut icon_draws,
            );
            self.tooltip = tooltip;
        }

        if self.show_save_modal {
            let (verts, texts) = modal::draw(&screen_config);
            vertices.extend(verts);
            Graphics::push_text_draws(
                &texts,
                &self.font_cache,
                &self.glyph_cache,
                &screen_config,
                &mut glyph_vertices,
                &mut char_draws,
            );
        }

        let total_seconds = ((self.playhead_beat / self.bpm) * 60.0) as u32;
        let time_string = format!(
            "{:02}:{:02}:{:02}",
            total_seconds / 3600,
            (total_seconds % 3600) / 60,
            total_seconds % 60
        );
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

        // tooltip
        let tooltip_vert_start = vertices.len() as u32;
        let tooltip_char_start = char_draws.len();
        if let Some(tt) = &self.tooltip {
            if mouse_state
                .hover_duration
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
                        &screen_config,
                        &mut glyph_vertices,
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
            &screen_config,
            &mut glyph_vertices,
            &mut char_draws,
        );
        let toolbar_range = WindowDrawRange {
            vert_start: toolbar_vert_start,
            vert_end: vertices.len() as u32,
            char_start: toolbar_char_start,
            char_end: char_draws.len(),
        };
        let tooltip_range = WindowDrawRange {
            vert_start: tooltip_vert_start,
            vert_end: tooltip_vert_end,
            char_start: tooltip_char_start,
            char_end: tooltip_char_end,
        };

        // --- context menu ---
        let context_menu_vert_start = vertices.len() as u32;
        let context_menu_char_start = char_draws.len();
        if let Some(menu) = &self.context_menu {
            let (texts, result, cursor) = menu.draw(&screen_config, mouse_state, &mut vertices);
            Graphics::push_text_draws(
                &texts,
                &self.font_cache,
                &self.glyph_cache,
                &screen_config,
                &mut glyph_vertices,
                &mut char_draws,
            );
            if cursor != CursorIcon::Default {
                cursor_icon = cursor;
            }
            click_result = click_result.or(result);
        }
        let context_menu_range = WindowDrawRange {
            vert_start: context_menu_vert_start,
            vert_end: vertices.len() as u32,
            char_start: context_menu_char_start,
            char_end: char_draws.len(),
        };

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
            &screen_config,
            &mut glyph_vertices,
            &mut char_draws,
        );
        let footer_range = WindowDrawRange {
            vert_start: footer_vert_start,
            vert_end: vertices.len() as u32,
            char_start: footer_char_start,
            char_end: char_draws.len(),
        };

        // --- track tray + file tree ---
        let mut track_tray_range: Option<WindowDrawRange> = None;
        let mut file_tree_range: Option<WindowDrawRange> = None;
        let mut divider_range: Option<WindowDrawRange> = None;
        let mut tray_icon_start = 0;
        let mut tray_icon_end = 0;

        if self.show_track_tray {
            let tray_vert_start = vertices.len() as u32;
            let tray_char_start = char_draws.len();

            let (texts, result, cursor) = side_panel::track_tray::draw(
                mouse_state,
                &screen_config,
                self.resizing_track_tray,
                &self.tracks,
                self.track_tray_width,
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
                &screen_config,
                &mut glyph_vertices,
                &mut char_draws,
            );

            // divider + File Tree title (unscissored, must stay in track_tray_range)

            divider_range = if self.show_track_tray {
                let vert_start = vertices.len() as u32;
                let char_start = char_draws.len();
                Rectangle {
                    x: 0.0,
                    y: (screen_config.height / 2) as f32,
                    width: self.track_tray_width,
                    height: screen_config.height as f32 - (screen_config.height / 2) as f32,
                }
                .draw(&screen_config, PEBBLE, NO_RADIUS, &mut vertices);
                let w_divider = Rectangle {
                    x: PAD_2,
                    y: (screen_config.height / 2) as f32,
                    width: self.track_tray_width - PAD_4,
                    height: 1.0,
                };

                w_divider.draw(&screen_config, DARK_GRAY_HOVER, RADIUS_4, &mut vertices);
                let h_divider = Rectangle {
                    x: self.track_tray_width - 1.0,
                    y: TOOLBAR_MARGIN,
                    width: 1.0,
                    height: screen_config.height as f32 - TOOLBAR_MARGIN,
                };
                h_divider.draw(&screen_config, DARK_GRAY_HOVER, NO_RADIUS, &mut vertices);

                use crate::graphics::side_panel::draw_title;
                Graphics::push_text_draws(
                    &[draw_title("File Tree", (w_divider.x - 2.0, w_divider.y))],
                    &self.font_cache,
                    &self.glyph_cache,
                    &screen_config,
                    &mut glyph_vertices,
                    &mut char_draws,
                );
                Some(WindowDrawRange {
                    vert_start,
                    vert_end: vertices.len() as u32,
                    char_start,
                    char_end: char_draws.len(),
                })
            } else {
                None
            };

            let file_tree_vert_start = vertices.len() as u32;
            let file_tree_char_start = char_draws.len();

            use crate::graphics::side_panel::track_tray::file_tree;
            let (icons, text_items, ft_result, ft_cursor) = file_tree::draw(
                mouse_state,
                &screen_config,
                &self.user_fs_location,
                &self.expanded_dirs,
                &self.fs_cache,
                self.fs_scroll_offset,
                self.track_tray_width,
                &mut vertices,
                (screen_config.height / 2) as f32,
            );
            click_result = click_result.or(ft_result);
            if ft_cursor != CursorIcon::Default {
                cursor_icon = ft_cursor;
            }
            Graphics::push_text_draws(
                &text_items,
                &self.font_cache,
                &self.glyph_cache,
                &screen_config,
                &mut glyph_vertices,
                &mut char_draws,
            );
            tray_icon_start = icon_draws.len();
            for icon in icons {
                push_icon_draw(
                    &self.icon_cache,
                    &self.device,
                    &screen_config,
                    &icon,
                    &mut icon_draws,
                );
            }
            tray_icon_end = icon_draws.len();

            track_tray_range = Some(WindowDrawRange {
                vert_start: tray_vert_start,
                vert_end: file_tree_vert_start,
                char_start: tray_char_start,
                char_end: file_tree_char_start,
            });
            file_tree_range = Some(WindowDrawRange {
                vert_start: file_tree_vert_start,
                vert_end: vertices.len() as u32,
                char_start: file_tree_char_start,
                char_end: char_draws.len(),
            });
        }

        // dragging cursor override
        if self.resizing_track_tray {
            cursor_icon = CursorIcon::ColResize;
        } else if self.dragging_window.is_some() || self.dragging || self.dragging_knob.is_some() {
            cursor_icon = CursorIcon::Default;
        }

        let drag_ghost_range = if let Some(ref path) = self.dragging_file {
            let ghost_vert_start = vertices.len() as u32;
            let ghost_char_start = char_draws.len();
            let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("?");
            let ghost = Rectangle {
                x: mouse_state.x,
                y: mouse_state.y - 16.0,
                width: 128.0,
                height: 32.0,
            };
            ghost.draw(&screen_config, LIGHT_GRAY, RADIUS_4, &mut vertices);
            Graphics::push_text_draws(
                &[TextItem {
                    text: name.to_string(),
                    x: ghost.x + PAD_4,
                    y: ghost.y + PAD_4,
                    size: 10.0,
                    font: ROBOTO,
                    color: DARK_GRAY,
                }],
                &self.font_cache,
                &self.glyph_cache,
                &screen_config,
                &mut glyph_vertices,
                &mut char_draws,
            );
            Some(WindowDrawRange {
                vert_start: ghost_vert_start,
                vert_end: vertices.len() as u32,
                char_start: ghost_char_start,
                char_end: char_draws.len(),
            })
        } else {
            None
        };

        // === RENDER PASS ===
        // Draw order (back to front):
        //
        // 1. Mini windows (sequencer, playlist, mixer, piano roll, track detail)
        //    - Each scissored to their own window bounds
        //    - Playlist and piano roll have sub-regions (header, timeline, grid)
        //
        // 2. Track tray — scissored to x=0..tray_width, y=0..sh/2
        //
        // 3. Divider + File Tree title + section background — unscissored chrome
        //
        // 4. File tree (scrollable list) — scissored to x=0..tray_width, y=sh/2+PAD_32+PAD_16..sh
        //
        // 5. Drag ghost — no scissor
        //
        // 6. Toolbar (top bar + pattern tray) — no scissor, always on top
        //
        // 7. Non-tray icons — no scissor
        //
        // 8. Tooltip — no scissor, always on top
        //
        // 9. Context menu — no scissor, always on top
        //
        // 10. Footer — no scissor, always on top
        //
        self.queue
            .write_buffer(&self.vertex_buffer, 0, bytemuck::cast_slice(&vertices));
        self.queue.write_buffer(
            &self.glyph_vertex_buffer,
            0,
            bytemuck::cast_slice(&glyph_vertices),
        );
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
                        Graphics::draw_range(
                            &mut r_pass,
                            &self.vertex_buffer,
                            &self.glyph_vertex_buffer,
                            any_bg,
                            &char_draws,
                            &pr.static_range,
                        );

                        let (sx, sy, sw2, sh2) =
                            Self::safe_scissor(wx, content_y, key_w, content_h, sw, sh);
                        r_pass.set_scissor_rect(sx, sy, sw2, sh2);
                        Graphics::draw_range(
                            &mut r_pass,
                            &self.vertex_buffer,
                            &self.glyph_vertex_buffer,
                            any_bg,
                            &char_draws,
                            &pr.piano_range,
                        );

                        let (sx, sy, sw2, sh2) =
                            Self::safe_scissor(grid_x, content_y, grid_w, content_h, sw, sh);
                        r_pass.set_scissor_rect(sx, sy, sw2, sh2);
                        Graphics::draw_range(
                            &mut r_pass,
                            &self.vertex_buffer,
                            &self.glyph_vertex_buffer,
                            any_bg,
                            &char_draws,
                            &pr.grid_range,
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
                        Graphics::draw_range(
                            &mut r_pass,
                            &self.vertex_buffer,
                            &self.glyph_vertex_buffer,
                            any_bg,
                            &char_draws,
                            &pl.static_range,
                        );

                        let (sx, sy, sw2, sh2) =
                            Self::safe_scissor(wx, content_y, header_w, content_h, sw, sh);
                        r_pass.set_scissor_rect(sx, sy, sw2, sh2);
                        Graphics::draw_range(
                            &mut r_pass,
                            &self.vertex_buffer,
                            &self.glyph_vertex_buffer,
                            any_bg,
                            &char_draws,
                            &pl.header_range,
                        );

                        let (sx, sy, sw2, sh2) =
                            Self::safe_scissor(header_x, content_y, timeline_w, content_h, sw, sh);
                        r_pass.set_scissor_rect(sx, sy, sw2, sh2);
                        Graphics::draw_range(
                            &mut r_pass,
                            &self.vertex_buffer,
                            &self.glyph_vertex_buffer,
                            any_bg,
                            &char_draws,
                            &pl.timeline_range,
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

                Graphics::draw_range(
                    &mut r_pass,
                    &self.vertex_buffer,
                    &self.glyph_vertex_buffer,
                    any_bg,
                    &char_draws,
                    range,
                );
            }

            // track tray — clipped to tray width
            if let Some(ref tr) = track_tray_range {
                let sw = self.surface_config.width;
                let sh = self.surface_config.height;
                let tray_bottom = sh / 2 + 2;
                let (sx, sy, sw2, sh2) =
                    Self::safe_scissor(0, 0, self.track_tray_width as u32, tray_bottom, sw, sh);
                r_pass.set_scissor_rect(sx, sy, sw2, sh2);
                Graphics::draw_range(
                    &mut r_pass,
                    &self.vertex_buffer,
                    &self.glyph_vertex_buffer,
                    any_bg,
                    &char_draws,
                    tr,
                );
                r_pass.set_scissor_rect(0, 0, sw, sh);
            }

            // divider + title — unscissored, drawn on top of both tray sections
            if let Some(ref dr) = divider_range {
                Graphics::draw_range(
                    &mut r_pass,
                    &self.vertex_buffer,
                    &self.glyph_vertex_buffer,
                    any_bg,
                    &char_draws,
                    dr,
                );
            }

            // file tree — scissored to below divider
            if let Some(ref ft) = file_tree_range {
                let sw = self.surface_config.width;
                let sh = self.surface_config.height;
                let divider_y = sh / 2 + (PAD_32 + PAD_16) as u32;
                let (sx, sy, sw2, sh2) = Self::safe_scissor(
                    0,
                    divider_y,
                    self.track_tray_width as u32,
                    sh - divider_y,
                    sw,
                    sh,
                );
                r_pass.set_scissor_rect(sx, sy, sw2, sh2);
                Graphics::draw_range(
                    &mut r_pass,
                    &self.vertex_buffer,
                    &self.glyph_vertex_buffer,
                    any_bg,
                    &char_draws,
                    ft,
                );
                for icon in &icon_draws[tray_icon_start..tray_icon_end] {
                    r_pass.set_bind_group(0, icon.1, &[]);
                    r_pass.set_vertex_buffer(0, icon.0.slice(..));
                    r_pass.draw(0..6, 0..1);
                }
                r_pass.set_scissor_rect(0, 0, sw, sh);
            }

            if let Some(ref gr) = drag_ghost_range {
                Graphics::draw_range(
                    &mut r_pass,
                    &self.vertex_buffer,
                    &self.glyph_vertex_buffer,
                    any_bg,
                    &char_draws,
                    gr,
                );
            }

            // toolbar
            Graphics::draw_range(
                &mut r_pass,
                &self.vertex_buffer,
                &self.glyph_vertex_buffer,
                any_bg,
                &char_draws,
                &toolbar_range,
            );

            // non-tray icons
            for icon in icon_draws[..tray_icon_start]
                .iter()
                .chain(icon_draws[tray_icon_end..].iter())
            {
                r_pass.set_bind_group(0, icon.1, &[]);
                r_pass.set_vertex_buffer(0, icon.0.slice(..));
                r_pass.draw(0..6, 0..1);
            }

            // tooltip
            Graphics::draw_range(
                &mut r_pass,
                &self.vertex_buffer,
                &self.glyph_vertex_buffer,
                any_bg,
                &char_draws,
                &tooltip_range,
            );

            // context menu
            Graphics::draw_range(
                &mut r_pass,
                &self.vertex_buffer,
                &self.glyph_vertex_buffer,
                any_bg,
                &char_draws,
                &context_menu_range,
            );

            // footer
            Graphics::draw_range(
                &mut r_pass,
                &self.vertex_buffer,
                &self.glyph_vertex_buffer,
                any_bg,
                &char_draws,
                &footer_range,
            );
        }

        self.queue.submit(Some(encoder.finish()));
        frame.present();

        (click_result, cursor_icon)
    }
}
