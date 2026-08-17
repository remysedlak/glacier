//! main state logic for audio and ui decoupling threads
use crate::audio::AudioCommand;
use crate::config::UserSettings;
use crate::graphics::{
    drag::DragResult,
    mini_window::{PIANO_ROLL_ID, PLAYLIST_ID, SEQUENCER_ID},
    primitives::{RenameState, RenameTarget, PAD_32},
    {create_graphics, Graphics, Rc},
};
use crate::project::{spawn_track_load, AudioBlockType, TrackData};
use cpal::Stream;
use rfd::FileDialog;
use ringbuf::{
    traits::{Consumer, Producer},
    {HeapCons, HeapProd},
};
use std::path::PathBuf;
use std::sync::mpsc::{Receiver, TryRecvError};
use std::thread;
use std::time::Instant;
use winit::{
    application::ApplicationHandler,
    dpi::PhysicalSize,
    event::{MouseButton, WindowEvent},
    event_loop::{ActiveEventLoop, EventLoop, EventLoopProxy},
    keyboard::{KeyCode, PhysicalKey},
    window::{Window, WindowId},
};

pub mod click;
mod rename;
mod ui_command;

pub use ui_command::UiCommand;

#[derive(Clone, Copy, Debug)]
pub struct MouseState {
    pub x: f32,
    pub y: f32,
    pub left_clicked: bool,
    pub left_double_clicked: bool,
    pub left_click_held: bool,
    pub right_clicked: bool,
    pub left_released: bool,
    pub scroll_x: f32,
    pub scroll_y: f32,
    pub hover_duration: Option<Instant>,
}

#[derive(Clone, Copy)]
pub struct ScrollOffset {
    pub x: f32,
    pub y: f32,
}
impl Default for ScrollOffset {
    fn default() -> Self {
        ScrollOffset { x: 0.0, y: 0.0 }
    }
}

pub struct PianoRollState {
    pub pattern_id: usize,
    pub track_id: u32,
    pub scroll_offset: ScrollOffset,
}

pub struct App {
    producer: HeapProd<AudioCommand>,
    consumer: HeapCons<UiCommand>,
    state: State,
    config: UserSettings,
    stream: Stream,
    pending_project: Option<String>,
    project_is_dirty: bool,
    ctrl_pressed: bool,
    pub shift_pressed: bool,
    mouse_state: MouseState,
    prev_mouse_x: f32,
    prev_mouse_y: f32,
    right_click_held: bool,
    last_click_time: Option<std::time::Instant>,
    track_file_dialog_rx: Option<Receiver<Option<PathBuf>>>,
    project_file_dialog_rx: Option<Receiver<Option<PathBuf>>>,
    track_load_rx: Option<Receiver<(TrackData, Vec<f32>)>>,
    project_save_dialog_rx: Option<Receiver<Option<PathBuf>>>,
    pending_drop: Option<(usize, usize)>,
}

enum State {
    Ready(Box<Graphics>),
    Init(Option<EventLoopProxy<Graphics>>),
}

impl App {
    /// Initialzie the app's state
    pub fn new(
        producer: HeapProd<AudioCommand>,
        consumer: HeapCons<UiCommand>,
        event_loop: &EventLoop<Graphics>,
        stream: Stream,
        config: UserSettings,
    ) -> Self {
        Self {
            producer,
            consumer,
            state: State::Init(Some(event_loop.create_proxy())),
            stream,
            pending_project: None,
            project_is_dirty: false,
            ctrl_pressed: false,
            shift_pressed: false,
            track_file_dialog_rx: None,
            project_file_dialog_rx: None,
            project_save_dialog_rx: None,
            track_load_rx: None,
            prev_mouse_x: 0.0,
            prev_mouse_y: 0.0,
            last_click_time: None,
            pending_drop: None,
            config,
            right_click_held: false,
            mouse_state: MouseState {
                x: 0.0,
                y: 0.0,
                left_clicked: false,
                left_double_clicked: false,
                right_clicked: false,
                scroll_x: 0.0,
                scroll_y: 0.0,
                left_click_held: false,
                left_released: false,
                hover_duration: None,
            },
        }
    }

    /// if the state is ready, draw each frame and handle it's click results
    fn draw(&mut self, event_loop: &ActiveEventLoop) {
        let mut should_exit = false;

        if !matches!(self.state, State::Ready(_)) {
            return;
        }

        // --- dialogs: only touches gfx fields directly, no self-methods called ---
        if let State::Ready(gfx) = &mut self.state {
            if let Some(rx) = &self.track_file_dialog_rx {
                match rx.try_recv() {
                    Ok(Some(path)) => {
                        let path_str = path.to_string_lossy().to_string();
                        self.track_load_rx = Some(spawn_track_load(path_str));
                        self.project_is_dirty = true;
                        self.track_file_dialog_rx = None;
                    }
                    Ok(None) => self.track_file_dialog_rx = None,
                    Err(TryRecvError::Empty) => {}
                    Err(TryRecvError::Disconnected) => self.track_file_dialog_rx = None,
                }
            }
            if let Some(rx) = &self.project_file_dialog_rx {
                match rx.try_recv() {
                    Ok(Some(path)) => {
                        gfx.is_playing = false;
                        self.pending_project = Some(path.to_string_lossy().to_string());
                        self.producer.try_push(AudioCommand::SaveProject).ok();
                        self.project_file_dialog_rx = None;
                    }
                    Ok(None) => self.project_file_dialog_rx = None,
                    Err(TryRecvError::Empty) => {}
                    Err(TryRecvError::Disconnected) => self.project_file_dialog_rx = None,
                }
            }
            if let Some(rx) = &self.project_save_dialog_rx {
                match rx.try_recv() {
                    Ok(Some(path)) => {
                        let path_str = path.to_string_lossy().to_string();
                        gfx.project_path = path_str.clone();
                        self.producer
                            .try_push(AudioCommand::SetProjectPath(path_str))
                            .ok();
                        self.producer.try_push(AudioCommand::SaveProject).ok();
                        self.project_save_dialog_rx = None;
                    }
                    Ok(None) => self.project_save_dialog_rx = None,
                    Err(TryRecvError::Empty) => {}
                    Err(TryRecvError::Disconnected) => self.project_save_dialog_rx = None,
                }
            }
        } // <- gfx borrow ends here

        // --- drain audio -> ui commands: needs &mut self, so gfx must NOT be held ---
        while let Some(cmd) = self.consumer.try_pop() {
            self.handle_ui_command(cmd, &mut should_exit);
        }

        // --- draw the frame: fresh gfx borrow, scoped tightly ---
        let (result, icon);
        {
            let State::Ready(gfx) = &mut self.state else {
                unreachable!("checked State::Ready above")
            };

            let start = std::time::Instant::now();
            let any_dragging = gfx.resizing_track_tray
                || gfx.dragging
                || gfx.dragging_window.is_some()
                || gfx.dragging_knob.is_some()
                || gfx.resizing_event.is_some()
                || gfx.dragging_slider.is_some();

            let draw_mouse = if any_dragging || gfx.dragging_file.is_some() {
                MouseState {
                    x: if any_dragging {
                        f32::NEG_INFINITY
                    } else {
                        self.mouse_state.x
                    },
                    y: if any_dragging {
                        f32::NEG_INFINITY
                    } else {
                        self.mouse_state.y
                    },
                    left_clicked: false,
                    left_click_held: if gfx.dragging_file.is_some() {
                        false
                    } else {
                        self.mouse_state.left_click_held
                    },
                    ..self.mouse_state
                }
            } else {
                self.mouse_state
            };

            let (r, i) = gfx.draw(&draw_mouse, self.project_is_dirty);
            result = r;
            icon = i;
            gfx.window.set_cursor(icon);
            gfx.frame_ms = start.elapsed().as_secs_f32() * 1000.0;

            if gfx.tooltip.is_some() {
                if self.mouse_state.hover_duration.is_none() {
                    self.mouse_state.hover_duration = Some(Instant::now());
                }
            } else {
                self.mouse_state.hover_duration = None;
            }
        } // <- gfx borrow ends here

        if let Some(rx) = &self.track_load_rx {
            match rx.try_recv() {
                Ok((data, samples)) => {
                    self.producer
                        .try_push(AudioCommand::LoadTrack(data, samples))
                        .ok();
                    self.track_load_rx = None;
                }
                Err(TryRecvError::Empty) => {}
                Err(TryRecvError::Disconnected) => self.track_load_rx = None,
            }
        }

        // --- handle click result: needs &mut self, so gfx must NOT be held ---
        self.handle_click_result(result, event_loop);

        self.mouse_state.left_clicked = false;
        self.mouse_state.left_double_clicked = false;
        self.mouse_state.right_clicked = false;
        self.mouse_state.scroll_x = 0.0;
        self.mouse_state.scroll_y = 0.0;
        self.mouse_state.left_released = false;

        if let State::Ready(gfx) = &mut self.state {
            gfx.request_redraw();
        }

        if should_exit {
            self.state = State::Init(None);
            event_loop.exit();
        }
    }

    /// Response to user resizing the main window
    fn resized(&mut self, size: PhysicalSize<u32>) {
        if let State::Ready(gfx) = &mut self.state {
            gfx.resize(size);
        }
    }
}

impl ApplicationHandler<Graphics> for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if let State::Init(proxy) = &mut self.state {
            if let Some(proxy) = proxy.take() {
                let mut win_attr = Window::default_attributes();
                win_attr = win_attr
                    .with_inner_size(winit::dpi::LogicalSize::new(1800, 1200))
                    .with_title("Glacier");
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
        graphics.request_redraw();
        self.state = State::Ready(Box::new(graphics));
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        event: WindowEvent,
    ) {
        match event {
            WindowEvent::CloseRequested => {
                if let State::Ready(gfx) = &mut self.state {
                    if self.project_is_dirty {
                        gfx.show_save_modal = true;
                    } else {
                        self.producer.try_push(AudioCommand::Shutdown).ok();
                    }
                    gfx.request_redraw();
                }
            }
            WindowEvent::Resized(size) => self.resized(size),
            WindowEvent::RedrawRequested => self.draw(event_loop),

            WindowEvent::KeyboardInput { event, .. } => {
                if !event.state.is_pressed() {
                    match event.physical_key {
                        PhysicalKey::Code(KeyCode::ControlLeft) => self.ctrl_pressed = false,
                        PhysicalKey::Code(KeyCode::ShiftLeft | KeyCode::ShiftRight) => {
                            self.shift_pressed = false;
                        }
                        _ => {}
                    }
                }

                if event.state.is_pressed() {
                    if self.handle_rename_key(&event) {
                        return;
                    }
                    if let State::Ready(gfx) = &mut self.state {
                        match event.physical_key {
                            PhysicalKey::Code(KeyCode::Space) => {
                                self.producer.try_push(AudioCommand::TogglePlay).ok();
                                gfx.is_playing = !gfx.is_playing;
                            }
                            PhysicalKey::Code(KeyCode::ControlLeft) => self.ctrl_pressed = true,
                            PhysicalKey::Code(KeyCode::ShiftLeft | KeyCode::ShiftRight) => {
                                self.shift_pressed = true;
                            }
                            PhysicalKey::Code(KeyCode::F2) => {
                                if let AudioBlockType::Pattern(id) = gfx.active_tray {
                                    if let Some(pattern) = gfx.patterns.iter().find(|p| p.id == id)
                                    {
                                        gfx.renaming = Some(RenameState {
                                            target: RenameTarget::Pattern(id),
                                            original_name: pattern.name.clone(),
                                            edited_name: pattern.name.clone(),
                                            cursor: pattern.name.len(),
                                        });
                                    }
                                }
                            }
                            PhysicalKey::Code(KeyCode::KeyS) if self.ctrl_pressed => {
                                if gfx.project_path
                                    == crate::project::Project::default_project_file()
                                {
                                    if self.project_save_dialog_rx.is_none() {
                                        let (tx, rx) =
                                            std::sync::mpsc::channel::<Option<PathBuf>>();
                                        self.project_save_dialog_rx = Some(rx);
                                        thread::spawn(move || {
                                            let file = FileDialog::new()
                                                .add_filter("toml", &["toml"])
                                                .set_file_name("project.toml")
                                                .save_file();
                                            tx.send(file).ok();
                                        });
                                    }
                                } else {
                                    self.producer.try_push(AudioCommand::SaveProject).ok();
                                }
                            }
                            _ => {}
                        }
                    }
                }
            }

            WindowEvent::MouseWheel { delta, .. } => {
                match delta {
                    winit::event::MouseScrollDelta::LineDelta(x, y) => {
                        self.mouse_state.scroll_x = x;
                        self.mouse_state.scroll_y = y;
                    }
                    winit::event::MouseScrollDelta::PixelDelta(pos) => {
                        self.mouse_state.scroll_x = pos.x as f32;
                        self.mouse_state.scroll_y = pos.y as f32;
                    }
                }

                if let State::Ready(gfx) = &mut self.state {
                    let scroll_owner = gfx
                        .z_order
                        .iter()
                        .rev()
                        .find(|&&id| {
                            gfx.mini_windows[id].is_open
                                && gfx.mini_windows[id]
                                    .is_hovered(self.mouse_state.x, self.mouse_state.y)
                        })
                        .copied();

                    if scroll_owner == Some(PLAYLIST_ID) {
                        if self.shift_pressed {
                            gfx.playlist_scroll_offset.x = (gfx.playlist_scroll_offset.x
                                - self.mouse_state.scroll_y * 35.0)
                                .clamp(0.0, 1448.0);
                        } else {
                            gfx.playlist_scroll_offset.y = (gfx.playlist_scroll_offset.y
                                - self.mouse_state.scroll_y * 35.0)
                                .clamp(0.0, 1448.0);
                        }
                    } else if scroll_owner == Some(PIANO_ROLL_ID) {
                        if let Some(state) = gfx.piano_roll_state.as_mut() {
                            if self.shift_pressed {
                                state.scroll_offset.x = (state.scroll_offset.x
                                    - self.mouse_state.scroll_y * 35.0)
                                    .clamp(0.0, 1448.0);
                            } else {
                                state.scroll_offset.y = (state.scroll_offset.y
                                    - self.mouse_state.scroll_y * 35.0)
                                    .clamp(0.0, 1448.0);
                            }
                        }
                    } else if scroll_owner == Some(SEQUENCER_ID) {
                        if self.shift_pressed {
                            gfx.sequencer_scroll_offset.x = (gfx.sequencer_scroll_offset.x
                                - self.mouse_state.scroll_y * 35.0)
                                .clamp(0.0, 1448.0);
                        } else {
                            gfx.sequencer_scroll_offset.y = (gfx.sequencer_scroll_offset.y
                                - self.mouse_state.scroll_y * 35.0)
                                .clamp(0.0, 1448.0);
                        }
                    } else if self.mouse_state.x < gfx.track_tray_width
                        && self.mouse_state.y > (gfx.surface_config.height as f32 / 2.0)
                    {
                        let divider_y = gfx.surface_config.height as f32 / 2.0;
                        let visible_height = gfx.surface_config.height as f32 - divider_y;
                        let total_rows = crate::project::count_fs_rows(
                            &gfx.user_fs_location,
                            &gfx.expanded_dirs,
                            &gfx.fs_cache,
                        );
                        let content_height = total_rows as f32 * PAD_32;
                        let max_scroll = (content_height - visible_height).max(0.0);
                        gfx.fs_scroll_offset = (gfx.fs_scroll_offset
                            - self.mouse_state.scroll_y * 20.0)
                            .clamp(0.0, max_scroll);
                        gfx.request_redraw();
                    }
                }
            }

            WindowEvent::MouseInput { state, button, .. } => {
                if state.is_pressed() && button == MouseButton::Left {
                    let now = std::time::Instant::now();
                    let is_double_click = self
                        .last_click_time
                        .map(|t| now.duration_since(t).as_millis() < 300)
                        .unwrap_or(false);
                    self.last_click_time = Some(now);

                    if is_double_click {
                        self.mouse_state.left_double_clicked = true;
                    }
                    self.mouse_state.left_click_held = true;
                    self.mouse_state.left_clicked = true;
                    self.draw(event_loop);
                } else {
                    self.mouse_state.left_released = true;
                    self.mouse_state.left_click_held = false;
                    self.mouse_state.left_clicked = false;
                    self.draw(event_loop);

                    if let State::Ready(gfx) = &mut self.state {
                        gfx.dragging = false;
                        gfx.dragging_window = None;
                        gfx.dragging_knob = None;
                        gfx.dragging_slider = None;
                        gfx.resizing_track_tray = false;
                        gfx.resizing_event = None;
                        gfx.dragging_file = None;
                    }
                }
                if state.is_pressed() && button == MouseButton::Right {
                    self.mouse_state.right_clicked = true;
                    self.right_click_held = true;
                    self.draw(event_loop);
                } else {
                    self.right_click_held = false;
                    self.mouse_state.right_clicked = false;
                }
            }

            WindowEvent::CursorMoved { position, .. } => {
                self.mouse_state.x = position.x as f32;
                self.mouse_state.y = position.y as f32;
                let delta_y = position.y as f32 - self.prev_mouse_y;
                let delta_x = position.x as f32 - self.prev_mouse_x;
                self.prev_mouse_y = position.y as f32;
                self.prev_mouse_x = position.x as f32;

                if let State::Ready(gfx) = &mut self.state {
                    if self.mouse_state.left_click_held {
                        match gfx.handle_drag(
                            position.x as f32,
                            position.y as f32,
                            delta_y,
                            delta_x,
                        ) {
                            DragResult::None => {}
                            DragResult::DraggingFile(_) => gfx.request_redraw(),
                            DragResult::ResizeTrackTray(_) => gfx.request_redraw(),
                            DragResult::DragMasterVolumeSlider(new_volume) => {
                                self.producer
                                    .try_push(AudioCommand::ChangeMasterVolume(new_volume))
                                    .ok();
                                gfx.request_redraw();
                            }
                            DragResult::DragTrackVolumeKnob(track_id, new_volume) => {
                                self.producer
                                    .try_push(AudioCommand::ChangeTrackVolume(track_id, new_volume))
                                    .ok();
                                gfx.request_redraw();
                            }
                            DragResult::DragTrackVolumeSlider(track_id, new_volume) => {
                                self.producer
                                    .try_push(AudioCommand::ChangeTrackVolume(track_id, new_volume))
                                    .ok();
                                gfx.request_redraw();
                            }
                            DragResult::ResizeAudioBlock(event_id, amount) => {
                                self.producer
                                    .try_push(AudioCommand::ResizeAudioBlock(event_id, amount))
                                    .ok();
                            }
                        }
                    } else {
                        gfx.dragging_knob = None;
                        gfx.dragging_slider = None;
                    }
                }
            }
            _ => {}
        }
    }
}
