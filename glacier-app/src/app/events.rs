use super::*;
use std::path::PathBuf;
use winit::platform::wayland::WindowAttributesExtWayland;
use winit::{
    application::ApplicationHandler,
    event::{MouseButton, WindowEvent},
    event_loop::ActiveEventLoop,
    keyboard::{KeyCode, PhysicalKey},
    window::{Icon, Window, WindowId},
};

fn load_icon() -> Icon {
    let bytes = include_bytes!("../../../assets/glacier_png.png");
    let image = image::load_from_memory(bytes)
        .expect("failed to decode icon")
        .into_rgba8();
    let (width, height) = image.dimensions();
    let rgba = image.into_raw();
    Icon::from_rgba(rgba, width, height).expect("failed to create icon")
}

impl ApplicationHandler<Graphics> for App {
    /// typically fires once at start for OS
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if let GraphicsState::Init(proxy) = &mut self.state {
            if let Some(proxy) = proxy.take() {
                let mut win_attr = Window::default_attributes();
                let icon = load_icon();
                win_attr = win_attr
                    .with_inner_size(winit::dpi::LogicalSize::new(1800, 1200))
                    .with_window_icon(Some(icon)) // windows icon (wayland attaches icon at .desktop, mac uses embeded .icns )
                    .with_title("Glacier") // window decoration title, cross-platform, already confirmed working
                    .with_name("Glacier", ""); // wayland app_id / taskbar label

                let window = Rc::new(
                    event_loop
                        .create_window(win_attr)
                        .expect("create window err."),
                );
                pollster::block_on(create_graphics(window, proxy));
            }
        }
    }

    fn user_event(&mut self, _event_loop: &ActiveEventLoop, graphics: Graphics) {
        let graphics = graphics;
        graphics.request_redraw();
        self.state = GraphicsState::Ready(Box::new(graphics));
    }

    fn exiting(&mut self, _event_loop: &ActiveEventLoop) {
        // Destroy native audio and graphics resources while winit's event loop
        // is still alive. In particular, some window/surface backends depend on
        // the event loop's platform state during destruction.
        self.shutdown_audio_stream();
        self.state = GraphicsState::Init(None);
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        event: WindowEvent,
    ) {
        if (self.shutdown.close_requested || self.shutdown.closing)
            && !matches!(
                event,
                WindowEvent::CloseRequested | WindowEvent::RedrawRequested
            )
        {
            return;
        }
        match event {
            // User clicks the (x) button on the app [soft shutdown]
            WindowEvent::CloseRequested => {
                if self.shutdown.closing || self.shutdown.close_requested {
                    return;
                }
                if self.audio.stream.is_none() {
                    crate::config::save(&self.config);
                    event_loop.exit();
                    return;
                }
                // Cancel an in-flight project switch and close the currently open
                // project. Dropping the receiver makes the worker discard its result.
                if self.project.load_rx.is_some() {
                    self.project.load_rx = None;
                    self.project.pending_path = None;
                }
                self.shutdown.close_requested = true;
                if let GraphicsState::Ready(gfx) = &mut self.state {
                    gfx.request_redraw();
                }
            }
            // resizing the main app window
            WindowEvent::Resized(size) => self.resized(size),
            // draw the next frame
            WindowEvent::RedrawRequested => self.draw(event_loop),
            // manage keyboard presses
            WindowEvent::KeyboardInput { event, .. } => {
                if self.project.load_rx.is_some() || self.audio.stream.is_none() {
                    return;
                }
                if !event.state.is_pressed() {
                    match event.physical_key {
                        PhysicalKey::Code(KeyCode::ControlLeft) => self.input.control_pressed = false,
                        PhysicalKey::Code(KeyCode::ShiftLeft | KeyCode::ShiftRight) => {
                            self.input.shift_pressed = false;
                        }
                        _ => {}
                    }
                }

                if event.state.is_pressed() {
                    // handle rename state and keyboard actions first
                    if self.handle_rename_key(&event) {
                        return;
                    }
                    if let GraphicsState::Ready(gfx) = &mut self.state {
                        match event.physical_key {
                            PhysicalKey::Code(KeyCode::Space) => {
                                gfx.is_playing = !gfx.is_playing;
                                gfx.request_redraw();
                                self.audio.commands
                                    .try_push(AudioCommand::SetPlaying {
                                        is_playing: gfx.is_playing,
                                        reset_position: false,
                                    })
                                    .ok();
                            }
                            PhysicalKey::Code(KeyCode::ControlLeft) => self.input.control_pressed = true,
                            PhysicalKey::Code(KeyCode::ShiftLeft | KeyCode::ShiftRight) => {
                                self.input.shift_pressed = true;
                            }
                            PhysicalKey::Code(KeyCode::F2) => {
                                if let PlaylistAction::PlaceFresh(AudioBlockType::Pattern(id)) =
                                    gfx.playlist_action
                                {
                                    if let Some(pattern) = gfx.patterns.iter().find(|p| p.id == id)
                                    {
                                        gfx.renaming = Some(RenameState {
                                            target: RenameTarget::Pattern(id),
                                            edited_name: pattern.name.clone(),
                                            cursor: pattern.name.len(),
                                        });
                                    }
                                }
                            }
                            PhysicalKey::Code(KeyCode::KeyS)
                                if self.input.control_pressed
                                    && self.project.pending_path.is_none()
                                    && (self.project.dirty || project_needs_save_as(gfx)) =>
                            {
                                if project_needs_save_as(gfx) {
                                    if self.files.project_save_dialog_rx.is_none() {
                                        let (tx, rx) =
                                            std::sync::mpsc::channel::<Option<PathBuf>>();
                                        self.files.project_save_dialog_rx = Some(rx);
                                        thread::spawn(move || {
                                            let file = FileDialog::new()
                                                .add_filter("toml", &["toml"])
                                                .set_file_name("project.toml")
                                                .save_file();
                                            tx.send(file).ok();
                                        });
                                    }
                                } else {
                                    submit_project_save(
                                        &self.project.save_worker,
                                        &mut self.project.saves_in_flight,
                                        &self.project.name,
                                        gfx,
                                        SavePurpose::Regular,
                                        self.project.revision,
                                        None,
                                    );
                                }
                            }
                            _ => {}
                        }
                    }
                }
            }
            // manage mouse wheel scrolls (up and down)
            WindowEvent::MouseWheel { delta, .. } => {
                match delta {
                    winit::event::MouseScrollDelta::LineDelta(x, y) => {
                        self.input.mouse.scroll_x = x;
                        self.input.mouse.scroll_y = y;
                    }
                    winit::event::MouseScrollDelta::PixelDelta(pos) => {
                        self.input.mouse.scroll_x = pos.x as f32;
                        self.input.mouse.scroll_y = pos.y as f32;
                    }
                }

                if let GraphicsState::Ready(gfx) = &mut self.state {
                    let scroll_owner = gfx
                        .z_order
                        .iter()
                        .rev()
                        .find(|&&id| {
                            gfx.mini_windows[id].is_open
                                && gfx.mini_windows[id]
                                    .is_hovered(self.input.mouse.x, self.input.mouse.y)
                        })
                        .copied();

                    // PLAYLIST SCROLLING
                    if scroll_owner == Some(PLAYLIST_ID) {
                        if self.input.shift_pressed {
                            gfx.playlist_scroll_offset.x = (gfx.playlist_scroll_offset.x
                                - self.input.mouse.scroll_y * 35.0)
                                .clamp(0.0, 1448.0);
                        } else {
                            gfx.playlist_scroll_offset.y = (gfx.playlist_scroll_offset.y
                                - self.input.mouse.scroll_y * 35.0)
                                .clamp(0.0, 1448.0);
                        }
                    }
                    // PIANO ROLL SCROLLING
                    else if scroll_owner == Some(PIANO_ROLL_ID) {
                        if let Some(state) = gfx.piano_roll_state.as_mut() {
                            if self.input.shift_pressed {
                                state.scroll_offset.x = (state.scroll_offset.x
                                    - self.input.mouse.scroll_y * 35.0)
                                    .clamp(0.0, 1448.0);
                            } else {
                                state.scroll_offset.y = (state.scroll_offset.y
                                    - self.input.mouse.scroll_y * 35.0)
                                    .clamp(0.0, 1448.0);
                            }
                        }
                    }
                    // SEQUENCER SCROLLING
                    else if scroll_owner == Some(SEQUENCER_ID) {
                        if self.input.shift_pressed {
                            gfx.sequencer_scroll_offset.x = (gfx.sequencer_scroll_offset.x
                                - self.input.mouse.scroll_y * 35.0)
                                .clamp(0.0, 1448.0);
                        } else {
                            gfx.sequencer_scroll_offset.y = (gfx.sequencer_scroll_offset.y
                                - self.input.mouse.scroll_y * 35.0)
                                .clamp(0.0, 1448.0);
                        }
                    }
                    // TRACK TRAY SCROLLING
                    else if self.input.mouse.x < gfx.track_tray_width
                        && self.input.mouse.y
                            > (gfx.render_context.surface_config.height as f32 / 2.0)
                    {
                        let divider_y = gfx.render_context.surface_config.height as f32 / 2.0;
                        let visible_height =
                            gfx.render_context.surface_config.height as f32 - divider_y;
                        let total_rows = crate::project::count_fs_rows(
                            &gfx.user_fs_location,
                            &gfx.expanded_dirs,
                            &gfx.fs_cache,
                        );
                        let content_height = total_rows as f32 * PAD_32;
                        let max_scroll = (content_height - visible_height).max(0.0);
                        gfx.fs_scroll_offset = (gfx.fs_scroll_offset
                            - self.input.mouse.scroll_y * 20.0)
                            .clamp(0.0, max_scroll);
                        gfx.request_redraw();
                    }
                }
            }
            // manage mouse clicks (left + right)
            WindowEvent::MouseInput { state, button, .. } => {
                match button {
                    MouseButton::Left if state.is_pressed() => {
                        // check if part of a double click
                        let now = std::time::Instant::now();
                        let is_double_click = self
                            .input.last_click
                            .map(|t| now.duration_since(t).as_millis() < 300)
                            .unwrap_or(false);
                        self.input.last_click = Some(now);
                        if is_double_click {
                            self.input.mouse.left_double_clicked = true;
                        }
                        self.input.mouse.left_clicked = true;
                        self.input.mouse.left_click_held = true;

                        self.draw(event_loop);
                    }
                    MouseButton::Left => {
                        self.input.mouse.left_released = true;
                        self.input.mouse.left_click_held = false;
                        self.input.mouse.left_clicked = false;
                        self.draw(event_loop);

                        if let GraphicsState::Ready(gfx) = &mut self.state {
                            gfx.clear_drag_state();
                        }
                    }
                    MouseButton::Right => {
                        self.input.right_mouse_held = state.is_pressed();
                        self.input.mouse.right_clicked = state.is_pressed();
                        if state.is_pressed() {
                            self.draw(event_loop);
                        }
                    }
                    _ => {}
                }
            }
            // manage mouse movement (drag / drop + hover)
            WindowEvent::CursorMoved { position, .. } => {
                // calculate mouse distance
                let delta_y = position.y as f32 - self.input.mouse.y;
                let delta_x = position.x as f32 - self.input.mouse.x;
                // update mouse state
                self.input.mouse.x = position.x as f32;
                self.input.mouse.y = position.y as f32;

                if let GraphicsState::Ready(gfx) = &mut self.state {
                    // IF USER IS ALREADY DRAGGING SOMETHING
                    if self.input.mouse.left_click_held {
                        match gfx.handle_drag(
                            position.x as f32,
                            position.y as f32,
                            delta_y,
                            delta_x,
                        ) {
                            DragResult::None => {}
                            DragResult::DraggingFile => gfx.request_redraw(),
                            DragResult::ResizeTrackTray => gfx.request_redraw(),
                            DragResult::DragMasterVolumeSlider(new_volume) => {
                                self.audio.commands
                                    .try_push(AudioCommand::ChangeMasterVolume(new_volume))
                                    .ok();
                                self.project.dirty = true;
                                self.project.revision += 1;
                                gfx.request_redraw();
                            }
                            DragResult::DragTrackVolumeKnob(track_id, new_volume) => {
                                self.audio.commands
                                    .try_push(AudioCommand::ChangeTrackVolume(track_id, new_volume))
                                    .ok();
                                self.project.dirty = true;
                                self.project.revision += 1;
                                gfx.request_redraw();
                            }
                            DragResult::DragTrackVolumeSlider(track_id, new_volume) => {
                                self.audio.commands
                                    .try_push(AudioCommand::ChangeTrackVolume(track_id, new_volume))
                                    .ok();
                                self.project.dirty = true;
                                self.project.revision += 1;
                                gfx.request_redraw();
                            }
                            DragResult::ResizeAudioBlock(event_id, amount) => {
                                self.audio.commands
                                    .try_push(AudioCommand::ResizeAudioBlock(event_id, amount))
                                    .ok();
                                self.project.dirty = true;
                                self.project.revision += 1;
                            }
                        }
                    } else {
                        gfx.clear_drag_state()
                    }
                }
            }
            _ => {}
        }
        self.audio.commands.flush_pending();
    }
}

impl App {
    /// Applies a window resize event to the active graphics context.
    fn resized(&mut self, size: winit::dpi::PhysicalSize<u32>) {
        if let GraphicsState::Ready(gfx) = &mut self.state {
            gfx.resize(size);
        }
    }
}
