use crate::color::*;
use crate::graphics::ui::*;
use crate::graphics::widgets::{
    ADD_INSTRUMENT_ICON_OFFSET, ICON_HEIGHT, ICON_WIDTH, LOAD_PROJECT_ICON_OFFSET, PLAY_SQUARE_HEIGHT, PLAY_SQUARE_WIDTH, PLAY_X_ORIGIN,
    PLAY_Y_ORIGIN, TITLEBAR_HEIGHT, TOOLBAR_MARGIN, TOOLBAR_THICKNESS, TOOLBAR_Y,
};
use crate::project::{AudioBlock, AudioBlockType, Instrument, PatternData};
use fontdue::layout::{CoordinateSystem, Layout, TextStyle};
use primitives::Vertex;
use std::{borrow::Cow, collections::HashMap};
use wgpu::util::DeviceExt;
use wgpu::{
    Color, CommandEncoderDescriptor, DeviceDescriptor, Features, FragmentState, Instance, Limits, LoadOp, MemoryHints, Operations, PowerPreference,
    RenderPassColorAttachment, RenderPassDescriptor, RenderPipeline, RenderPipelineDescriptor, RequestAdapterOptions, ShaderModuleDescriptor,
    ShaderSource, StoreOp, SurfaceConfiguration, TextureFormat, TextureViewDescriptor, VertexState,
};
use widgets::{draw_toolbar, Rectangle, TextItem};
use winit::{dpi::PhysicalSize, event_loop::EventLoopProxy, window::Window};

pub mod font;
pub mod mixer;
pub mod playlist;
pub mod primitives;
pub mod sequencer;
pub mod ui;
pub mod widgets;

pub const SEQUENCER_ID: usize = 0;
pub const PLAYLIST_ID: usize = 1;
pub const MIXER_ID: usize = 2;

#[cfg(not(target_arch = "wasm32"))]
pub type Rc<T> = std::sync::Arc<T>;

pub struct ScreenConfig {
    pub width: u32,
    pub height: u32,
}

#[derive(Debug)]
pub enum ClickResult {
    Step(usize, usize, usize),
    Mute(usize),
    Stop,
    ChangeBpm(f32),
    TogglePlay,
    ProjectFileDialog,
    InstrumentFileDialog,
    DeleteTrack(usize),
    DeletePlaylistPattern(usize),
    AddPlaylistPattern(usize, u32, usize, AudioBlockType),
    AddPlaylist,
    ToggleSequencerWindow,
    ToggleMixerWindow,
    TogglePlaylistWindow,
    None,
}
pub enum DragResult {
    DragVolumeSlider(f32),
    DragVolumeKnob(usize, f32),
    None,
}

/// Initialize the graphics with default/loaded state and find driver/display info
pub async fn create_graphics(window: Rc<Window>, proxy: EventLoopProxy<Graphics>) {
    let instance = Instance::default();
    let surface = instance.create_surface(Rc::clone(&window)).unwrap();
    let adapter = instance
        .request_adapter(&RequestAdapterOptions {
            power_preference: PowerPreference::default(),
            force_fallback_adapter: false,
            compatible_surface: Some(&surface),
        })
        .await
        .expect("Could not get an adapter (GPU).");

    let (device, queue) = adapter
        .request_device(&DeviceDescriptor {
            label: None,
            required_features: Features::empty(),
            required_limits: Limits::downlevel_webgl2_defaults().using_resolution(adapter.limits()),
            memory_hints: MemoryHints::Performance,
            trace: Default::default(),
        })
        .await
        .expect("Failed to get device");

    let size = window.inner_size();
    let width = size.width.max(1);
    let height = size.height.max(1);
    let surface_config = surface.get_default_config(&adapter, width, height).unwrap();
    surface.configure(&device, &surface_config);

    let vertices: Vec<Vertex> = Vec::new();
    let patterns: Vec<PatternData> = Vec::new();

    let vertex_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("Vertex Buffer"),
        size: ONE_MEGABYTE,
        usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });

    let mut mini_windows: Vec<MiniWindow> = Vec::new();
    let instruments: Vec<Instrument> = Vec::new();

    let sequencer_window = MiniWindow::new(256.0, 128.0, 1300.0, 400.0, "Sequencer", WindowKind::Sequencer, true);
    mini_windows.push(sequencer_window);
    let playlist_window = MiniWindow::new(900.0, 600.0, 1500.0, 900.0, "Playlist", WindowKind::Playlist, true);
    mini_windows.push(playlist_window);
    let mixer_window = MiniWindow::new(128.0, 500.0, 800.0, 300.0, "Mixer", WindowKind::Mixer, false);
    mini_windows.push(mixer_window);

    let events: Vec<AudioBlock> = Vec::new();

    let font_data = include_bytes!("Roboto-VariableFont_wdth,wght.ttf") as &[u8];
    let font = fontdue::Font::from_bytes(font_data, fontdue::FontSettings::default()).unwrap();
    let bind_group_layout = font::create_bind_group_layout(&device);
    let render_pipeline = create_pipeline(&device, surface_config.format, &bind_group_layout);
    let glyph_cache = font::build_glyph_cache(&device, &queue, &font, 18.0);

    let gfx = Graphics {
        window: window.clone(),
        surface,
        surface_config,
        instruments,
        patterns,
        device,
        glyph_cache,
        font,
        queue,
        render_pipeline,
        vertex_buffer,
        num_vertices: vertices.len() as u32,
        active_step: 0,

        bpm: 120.0,
        is_playing: false,
        master_volume: 0.5,
        dragging_knob: None,
        mini_windows,
        dragging_window: None,
        active_pattern_id: 0,
        dragging: false,
        events,
        z_order: vec![SEQUENCER_ID, PLAYLIST_ID, MIXER_ID],

        playlist_scroll_x: 0.0,
        playlist_scroll_y: 0.0,
    };

    let _ = proxy.send_event(gfx);
}

fn create_pipeline(device: &wgpu::Device, swap_chain_format: TextureFormat, bind_group_layout: &wgpu::BindGroupLayout) -> RenderPipeline {
    let shader = device.create_shader_module(ShaderModuleDescriptor {
        label: None,
        source: ShaderSource::Wgsl(Cow::Borrowed(include_str!("../shader.wgsl"))),
    });

    device.create_render_pipeline(&RenderPipelineDescriptor {
        label: None,
        layout: Some(&device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[bind_group_layout],
            push_constant_ranges: &[],
        })),
        vertex: VertexState {
            module: &shader,
            entry_point: Some("vs_main"),
            buffers: &[Vertex::desc()],
            compilation_options: Default::default(),
        },
        fragment: Some(FragmentState {
            module: &shader,
            entry_point: Some("fs_main"),
            targets: &[Some(wgpu::ColorTargetState {
                format: swap_chain_format,
                blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                write_mask: wgpu::ColorWrites::ALL,
            })],
            compilation_options: Default::default(),
        }),
        primitive: Default::default(),
        depth_stencil: None,
        multisample: Default::default(),
        multiview: None,
        cache: None,
    })
}

pub struct Graphics {
    //wgpu
    window: Rc<Window>,
    surface: wgpu::Surface<'static>,
    surface_config: SurfaceConfiguration,
    device: wgpu::Device,
    queue: wgpu::Queue,
    render_pipeline: RenderPipeline,
    vertex_buffer: wgpu::Buffer,

    // text
    glyph_cache: HashMap<char, (wgpu::Texture, wgpu::BindGroup, fontdue::Metrics)>,
    font: fontdue::Font,

    pub instruments: Vec<Instrument>,
    pub patterns: Vec<PatternData>,
    pub events: Vec<AudioBlock>,

    num_vertices: u32,
    pub active_step: usize,
    pub bpm: f32,
    pub is_playing: bool,
    pub master_volume: f32,
    pub dragging_knob: Option<usize>,
    pub mini_windows: Vec<MiniWindow>,
    pub dragging_window: Option<usize>,
    pub dragging: bool,
    pub active_pattern_id: usize,
    pub z_order: Vec<usize>,

    // scrolling
    pub playlist_scroll_x: f32,
    pub playlist_scroll_y: f32,
}

pub fn bring_to_front(z_order: &mut Vec<usize>, id: usize) {
    z_order.retain(|&x| x != id);
    z_order.push(id);
}

impl Graphics {
    pub fn request_redraw(&self) {
        self.window.request_redraw();
    }

    pub fn load_instrument(&mut self, i: Instrument) {
        if i.data.id >= self.instruments.len() as u32 {
            self.instruments.push(i);
        }
    }
    pub fn load_pattern(&mut self, p: PatternData) {
        if p.id >= self.patterns.len() {
            self.patterns.push(p)
        }
    }
    pub fn load_event(&mut self, a: AudioBlock) {
        self.events.push(a);
    }

    pub fn handle_drag(&mut self, x: f32, y: f32, dy: f32, dx: f32) -> DragResult {
        let sequencer_window = &self.mini_windows[SEQUENCER_ID];
        let mixer_window = &self.mini_windows[MIXER_ID];

        if self.dragging_knob == None {
            let y_ceiling: f32 = mixer_window.y;
            let track_height: f32 = 164.0;
            let padding = 32.0;
            if x > mixer_window.x - padding + 24.0 && x < mixer_window.x + padding + 24.0 && y > mixer_window.y && y < mixer_window.y + track_height {
                self.master_volume = 1.0 - ((y - y_ceiling) / track_height).clamp(0.0, 1.0);
                self.dragging = true;
                return DragResult::DragVolumeSlider(self.master_volume);
            }
        }

        if let Some(i) = self.dragging_knob {
            self.instruments[i].data.track_volume = (self.instruments[i].data.track_volume - dy * 0.005).clamp(0.0, 1.0);
            self.dragging = true;
            return DragResult::DragVolumeKnob(i, self.instruments[i].data.track_volume);
        }

        for (i, track) in &mut self.instruments.iter_mut().enumerate() {
            let knob_rect = Rectangle {
                x: sequencer_window.x + 198.0 - KNOB_RADIUS,
                y: sequencer_window.y + (i as f32 * TRACK_GAP) + 24.0 - KNOB_RADIUS,
                width: KNOB_RADIUS * 2.0,
                height: KNOB_RADIUS * 2.0,
            };
            if knob_rect.is_hovered(x, y) {
                self.dragging_knob = Some(i);
                track.data.track_volume = (track.data.track_volume - dy * 0.01).clamp(0.0, 1.0);
                self.dragging = true;
                return DragResult::DragVolumeKnob(i, track.data.track_volume);
            }
        }

        if let Some(i) = self.dragging_window {
            self.mini_windows[i].x += dx;
            self.mini_windows[i].y += dy;

            return DragResult::None;
        }

        if !self.dragging {
            for (i, win) in self.mini_windows.iter().enumerate() {
                let titlebar = Rectangle {
                    x: win.x,
                    y: win.y - TITLEBAR_HEIGHT,
                    width: win.width,
                    height: TITLEBAR_HEIGHT,
                };
                if titlebar.is_hovered(x, y) {
                    self.dragging_window = Some(i);
                    return DragResult::None;
                }
            }
        }
        DragResult::None
    }

    pub fn resize(&mut self, new_size: PhysicalSize<u32>) {
        self.surface_config.width = new_size.width.max(1);
        self.surface_config.height = new_size.height.max(1);
        self.surface.configure(&self.device, &self.surface_config);
    }

    fn push_text_draws<'a>(
        texts: &[TextItem],
        font: &fontdue::Font,
        glyph_cache: &'a HashMap<char, (wgpu::Texture, wgpu::BindGroup, fontdue::Metrics)>,
        device: &wgpu::Device,
        screen_config: &ScreenConfig,
        char_draws: &mut Vec<(wgpu::Buffer, &'a wgpu::BindGroup)>,
    ) {
        for text_item in texts {
            let mut layout = Layout::new(CoordinateSystem::PositiveYDown);
            layout.append(&[font], &TextStyle::new(&text_item.text, 18.0, 0));
            for glyph in layout.glyphs() {
                if let Some((_, bind_group, _)) = glyph_cache.get(&glyph.parent) {
                    let gverts = font::draw_glyph(
                        text_item.x + glyph.x,
                        text_item.y + glyph.y,
                        glyph.width as f32,
                        glyph.height as f32,
                        screen_config,
                    );
                    let buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                        label: None,
                        contents: bytemuck::cast_slice(&gverts),
                        usage: wgpu::BufferUsages::VERTEX,
                    });
                    char_draws.push((buf, bind_group));
                }
            }
        }
    }

    pub fn draw(&mut self, mouse_state: &MouseState) -> ClickResult {
        let box_padding = 8.0;
        let padding = 16.0;

        let frame = self.surface.get_current_texture().expect("Failed to acquire next swap chain texture.");
        let view = frame.texture.create_view(&TextureViewDescriptor::default());
        let mut vertices: Vec<Vertex> = Vec::new();
        let mut click_result = ClickResult::None;
        let screen_config = ScreenConfig {
            width: self.surface_config.width,
            height: self.surface_config.height,
        };

        // handle window re-ordering
        if mouse_state.left_clicked {
            let z_order = self.z_order.clone();
            for &id in z_order.iter().rev() {
                if self.mini_windows[id].is_open && self.mini_windows[id].is_hovered(mouse_state.x, mouse_state.y) {
                    bring_to_front(&mut self.z_order, id);
                    break;
                }
            }
        }

        let mut char_draws: Vec<(wgpu::Buffer, &wgpu::BindGroup)> = Vec::new();
        let mut window_ranges: Vec<WindowDrawRange> = Vec::new();
        let mut playlist_window_ranges: Option<PlaylistDrawRanges> = None;

        // helper closure to convert text items into char_draws
        // (inlined per window below)

        for &id in &self.z_order {
            let vert_start = vertices.len() as u32;
            let char_start = char_draws.len();

            match id {
                SEQUENCER_ID if self.mini_windows[SEQUENCER_ID].is_open => {
                    let window = &self.mini_windows[SEQUENCER_ID];
                    let (verts, texts, result) = sequencer::draw(
                        window,
                        &mut self.patterns,
                        &mut self.instruments,
                        self.active_pattern_id,
                        self.active_step,
                        &mouse_state,
                        &screen_config,
                    );
                    vertices.extend(verts);
                    Graphics::push_text_draws(&texts, &self.font, &self.glyph_cache, &self.device, &screen_config, &mut char_draws);
                    click_result = result;
                }
                PLAYLIST_ID if self.mini_windows[PLAYLIST_ID].is_open => {
                    let window = &self.mini_windows[PLAYLIST_ID];
                    let (static_verts, static_texts, timeline_verts, timeline_texts, header_verts, header_texts, result) = playlist::draw(
                        window,
                        &self.events,
                        &self.patterns,
                        &mouse_state,
                        self.active_pattern_id,
                        self.playlist_scroll_x,
                        self.playlist_scroll_y,
                        self.active_step,
                        &screen_config,
                    );

                    // static shapes
                    let static_vert_start = vertices.len() as u32;
                    let static_char_start = char_draws.len();
                    vertices.extend(static_verts);
                    Graphics::push_text_draws(
                        &static_texts,
                        &self.font,
                        &self.glyph_cache,
                        &self.device,
                        &screen_config,
                        &mut char_draws,
                    );
                    let static_range: WindowDrawRange = WindowDrawRange {
                        vert_start: static_vert_start,
                        vert_end: vertices.len() as u32,
                        char_start: static_char_start,
                        char_end: char_draws.len(),
                    };

                    // track block scissor rectangle
                    let header_vert_start = vertices.len() as u32;
                    let header_char_start = char_draws.len();
                    vertices.extend(header_verts);
                    Graphics::push_text_draws(
                        &header_texts,
                        &self.font,
                        &self.glyph_cache,
                        &self.device,
                        &screen_config,
                        &mut char_draws,
                    );
                    let header_range: WindowDrawRange = WindowDrawRange {
                        vert_start: header_vert_start,
                        vert_end: vertices.len() as u32,
                        char_start: header_char_start,
                        char_end: char_draws.len(),
                    };

                    // timeline scissor rectangle
                    let timeline_vert_start = vertices.len() as u32;
                    let timeline_char_start = char_draws.len();
                    vertices.extend(timeline_verts);
                    Graphics::push_text_draws(
                        &timeline_texts,
                        &self.font,
                        &self.glyph_cache,
                        &self.device,
                        &screen_config,
                        &mut char_draws,
                    );
                    let timeline_range: WindowDrawRange = WindowDrawRange {
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
                    click_result = result;
                }
                MIXER_ID if self.mini_windows[MIXER_ID].is_open => {
                    let window = &self.mini_windows[MIXER_ID];
                    let (verts, texts) = mixer::draw(window, self.master_volume, &screen_config);
                    vertices.extend(verts);
                    Graphics::push_text_draws(&texts, &self.font, &self.glyph_cache, &self.device, &screen_config, &mut char_draws);
                }
                _ => {}
            }

            window_ranges.push(WindowDrawRange {
                vert_start,
                vert_end: vertices.len() as u32,
                char_start,
                char_end: char_draws.len(),
            });
        }

        // handle delete after loop to avoid borrow issues
        if let ClickResult::DeleteTrack(i) = click_result {
            self.instruments.remove(i);
        }
        if let ClickResult::DeletePlaylistPattern(id) = click_result {
            self.events.retain(|e| e.id != id);
        }

        // --- toolbar / UI layer (always on top) ---
        let toolbar_vert_start = vertices.len() as u32;

        // patterns tray
        let pattern_tray = Rectangle {
            x: screen_config.width as f32 - 128.0,
            y: TOOLBAR_Y,
            width: 128.0,
            height: self.surface_config.height as f32 - TOOLBAR_THICKNESS,
        };
        vertices.extend(pattern_tray.draw(&screen_config, PASCAL));

        // add pattern button
        let add_pattern_button = Rectangle {
            x: screen_config.width as f32 - 32.0,
            y: TOOLBAR_Y + 12.0,
            width: 16.0,
            height: 16.0,
        };
        vertices.extend(add_pattern_button.draw(&screen_config, add_pattern_button.hover_color(mouse_state.x, mouse_state.y)));
        if add_pattern_button.is_hovered(mouse_state.x, mouse_state.y) && mouse_state.left_clicked {
            click_result = ClickResult::AddPlaylist;
        }

        for (i, pattern) in &mut self.patterns.iter_mut().enumerate() {
            let pattern_button = Rectangle {
                x: screen_config.width as f32 - 128.0 + padding,
                y: 48.0 + (32.0 * i as f32) + 24.0,
                width: 96.0,
                height: 24.0,
            };
            if i == self.active_pattern_id {
                let indicator = Rectangle {
                    x: screen_config.width as f32 - 128.0 + box_padding,
                    y: 48.0 + (32.0 * i as f32) + 24.0 + box_padding,
                    width: 4.0,
                    height: 4.0,
                };
                vertices.extend(indicator.draw(&screen_config, ORANGE));
            }
            vertices.extend(pattern_button.draw(&screen_config, pattern_button.hover_color(mouse_state.x, mouse_state.y)));
            if mouse_state.left_clicked && pattern_button.is_hovered(mouse_state.x, mouse_state.y) {
                self.active_pattern_id = pattern.id as usize;
            }
        }

        let bpm_up = Rectangle {
            x: 48.0,
            y: 4.0,
            width: 32.0,
            height: 10.0,
        };
        vertices.extend(bpm_up.draw(&screen_config, LIGHT_GRAY));
        if mouse_state.left_clicked && bpm_up.is_hovered(mouse_state.x, mouse_state.y) {
            self.bpm += 1.0;
            click_result = ClickResult::ChangeBpm(self.bpm);
        }

        let bpm_down = Rectangle {
            x: 48.0,
            y: 16.0,
            width: 32.0,
            height: 10.0,
        };
        vertices.extend(bpm_down.draw(&screen_config, LIGHT_GRAY));
        if mouse_state.left_clicked && bpm_down.is_hovered(mouse_state.x, mouse_state.y) {
            self.bpm -= 1.0;
            click_result = ClickResult::ChangeBpm(self.bpm);
        }

        // play / pauses button
        let play_button = Rectangle {
            x: PLAY_X_ORIGIN,
            y: PLAY_Y_ORIGIN,
            width: PLAY_SQUARE_WIDTH,
            height: PLAY_SQUARE_HEIGHT,
        };
        if mouse_state.left_clicked && play_button.is_hovered(mouse_state.x, mouse_state.y) {
            self.is_playing = !self.is_playing;
            click_result = ClickResult::TogglePlay;
        }
        vertices.extend(play_button.draw(&screen_config, play_button.hover_color(mouse_state.x, mouse_state.y)));

        // stop button
        let stop_button = Rectangle {
            x: PLAY_X_ORIGIN + 64.0,
            y: PLAY_Y_ORIGIN,
            width: PLAY_SQUARE_WIDTH,
            height: PLAY_SQUARE_HEIGHT,
        };
        if mouse_state.left_clicked && stop_button.is_hovered(mouse_state.x, mouse_state.y) && self.active_step != 0 {
            self.is_playing = !self.is_playing;
            self.active_step = 0;
            click_result = ClickResult::Stop;
        }
        vertices.extend(stop_button.draw(&screen_config, stop_button.hover_color(mouse_state.x, mouse_state.y)));

        let sequencer_toggle = Rectangle {
            x: PLAY_X_ORIGIN + 256.0,
            y: PLAY_Y_ORIGIN,
            width: PLAY_SQUARE_WIDTH,
            height: PLAY_SQUARE_HEIGHT,
        };
        vertices.extend(sequencer_toggle.draw(&screen_config, sequencer_toggle.hover_color(mouse_state.x, mouse_state.y)));
        if mouse_state.left_clicked && sequencer_toggle.is_hovered(mouse_state.x, mouse_state.y) {
            click_result = ClickResult::ToggleSequencerWindow;
        }

        let mixer_toggle = Rectangle {
            x: PLAY_X_ORIGIN + 256.0 + (BUTTON_GAP * 3.0),
            y: PLAY_Y_ORIGIN,
            width: PLAY_SQUARE_WIDTH,
            height: PLAY_SQUARE_HEIGHT,
        };
        vertices.extend(mixer_toggle.draw(&screen_config, mixer_toggle.hover_color(mouse_state.x, mouse_state.y)));
        if mouse_state.left_clicked && mixer_toggle.is_hovered(mouse_state.x, mouse_state.y) {
            click_result = ClickResult::ToggleMixerWindow;
        }

        let playlist_toggle = Rectangle {
            x: PLAY_X_ORIGIN + 256.0 + (BUTTON_GAP * 3.0) * 2.0,
            y: PLAY_Y_ORIGIN,
            width: PLAY_SQUARE_WIDTH,
            height: PLAY_SQUARE_HEIGHT,
        };
        vertices.extend(playlist_toggle.draw(&screen_config, playlist_toggle.hover_color(mouse_state.x, mouse_state.y)));
        if mouse_state.left_clicked && playlist_toggle.is_hovered(mouse_state.x, mouse_state.y) {
            click_result = ClickResult::TogglePlaylistWindow;
        }

        let load_project = Rectangle {
            x: screen_config.width as f32 - LOAD_PROJECT_ICON_OFFSET,
            y: TOOLBAR_MARGIN,
            width: ICON_WIDTH,
            height: ICON_HEIGHT,
        };
        if mouse_state.left_clicked && load_project.is_hovered(mouse_state.x, mouse_state.y) {
            click_result = ClickResult::ProjectFileDialog;
        }

        let load_instrument = Rectangle {
            x: screen_config.width as f32 - ADD_INSTRUMENT_ICON_OFFSET,
            y: TOOLBAR_MARGIN,
            width: ICON_WIDTH,
            height: ICON_HEIGHT,
        };
        if mouse_state.left_clicked && load_instrument.is_hovered(mouse_state.x, mouse_state.y) {
            click_result = ClickResult::InstrumentFileDialog;
        }

        draw_toolbar(&mut vertices, &screen_config, mouse_state.x, mouse_state.y);

        // build toolbar text items
        let toolbar_char_start = char_draws.len();

        let mut toolbar_texts: Vec<TextItem> = Vec::new();
        toolbar_texts.push(TextItem {
            text: "Patterns".to_string(),
            y: screen_config.width as f32 - 128.0 + box_padding,
            x: TOOLBAR_Y + box_padding,
        });

        for (i, pattern) in self.patterns.iter().enumerate() {
            toolbar_texts.push(TextItem {
                text: pattern.name.to_string(),
                x: screen_config.width as f32 - 96.0,
                y: 48.0 + (32.0 * i as f32) + 24.0,
            });
        }
        toolbar_texts.push(TextItem {
            text: "stop".to_string(),
            x: PLAY_X_ORIGIN + 64.0 + (PLAY_SQUARE_WIDTH / 4.0),
            y: 5.0,
        });
        toolbar_texts.push(TextItem {
            text: "sequence".to_string(),
            x: PLAY_X_ORIGIN + 256.0,
            y: 4.0,
        });
        toolbar_texts.push(TextItem {
            text: "mixer".to_string(),
            x: PLAY_X_ORIGIN + 256.0 + (BUTTON_GAP * 3.0),
            y: 4.0,
        });
        toolbar_texts.push(TextItem {
            text: "pl".to_string(),
            x: PLAY_X_ORIGIN + 256.0 + (BUTTON_GAP * 3.0) * 2.0,
            y: 4.0,
        });
        toolbar_texts.push(TextItem {
            text: "proj".to_string(),
            x: screen_config.width as f32 - 37.0,
            y: 4.0,
        });
        toolbar_texts.push(TextItem {
            text: "instr".to_string(),
            x: screen_config.width as f32 - (37.0 + 40.0 + 1.0),
            y: 4.0,
        });
        toolbar_texts.push(TextItem {
            text: self.bpm.to_string(),
            x: 10.0,
            y: TOOLBAR_MARGIN,
        });
        let label = if self.is_playing { "pause" } else { "play" };
        toolbar_texts.push(TextItem {
            text: label.to_string(),
            x: PLAY_X_ORIGIN + (PLAY_SQUARE_WIDTH / 4.0),
            y: 5.0,
        });

        Graphics::push_text_draws(
            &toolbar_texts,
            &self.font,
            &self.glyph_cache,
            &self.device,
            &screen_config,
            &mut char_draws,
        );

        self.queue.write_buffer(&self.vertex_buffer, 0, bytemuck::cast_slice(&vertices));
        self.num_vertices = vertices.len() as u32;

        let mut encoder = self.device.create_command_encoder(&CommandEncoderDescriptor { label: None });
        {
            let mut r_pass = encoder.begin_render_pass(&RenderPassDescriptor {
                label: None,
                color_attachments: &[Some(RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: Operations {
                        load: LoadOp::Clear(Color {
                            r: 0.1,
                            g: 0.1,
                            b: 0.1,
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
            let any_bg = self.glyph_cache.values().next().unwrap();

            // draw each window: geometry then its text (painter's algorithm)
            for (idx, range) in window_ranges.iter().enumerate() {
                let is_playlist = self.z_order[idx] == PLAYLIST_ID;

                if is_playlist {
                    if let Some(ref pl) = playlist_window_ranges {
                        let win = &self.mini_windows[PLAYLIST_ID];
                        let win_right = ((win.x + win.width) as u32).min(self.surface_config.width);
                        let wx = if win.x < 0.0 { 0u32 } else { win.x as u32 };
                        let ww = win_right.saturating_sub(wx);
                        let wy = if (win.y - TITLEBAR_HEIGHT) < 0.0 {
                            0u32
                        } else {
                            ((win.y - TITLEBAR_HEIGHT) as u32).min(self.surface_config.height)
                        };
                        let wh = (win.height as u32 + TITLEBAR_HEIGHT as u32).min(self.surface_config.height.saturating_sub(wy));

                        let content_y = (win.y as u32 + 48).min(self.surface_config.height);
                        let win_bottom = ((win.y + win.height) as u32).min(self.surface_config.height);
                        let content_h = win_bottom.saturating_sub(content_y);

                        let header_width = 144u32;
                        let step_start_x = win.x + header_width as f32;
                        let tx = if step_start_x < 0.0 {
                            0u32
                        } else {
                            (step_start_x as u32).min(self.surface_config.width)
                        };
                        let tw = win_right.saturating_sub(tx);
                        let header_tw = tx.saturating_sub(wx);

                        // static — full window scissor
                        r_pass.set_scissor_rect(wx, wy, ww, wh);
                        if pl.static_range.vert_start < pl.static_range.vert_end {
                            r_pass.set_bind_group(0, &any_bg.1, &[]);
                            r_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
                            r_pass.draw(pl.static_range.vert_start..pl.static_range.vert_end, 0..1);
                        }
                        for i in pl.static_range.char_start..pl.static_range.char_end {
                            r_pass.set_bind_group(0, char_draws[i].1, &[]);
                            r_pass.set_vertex_buffer(0, char_draws[i].0.slice(..));
                            r_pass.draw(0..6, 0..1);
                        }

                        // header — only covers header column width
                        r_pass.set_scissor_rect(wx, content_y, header_tw.max(1), content_h);
                        if pl.header_range.vert_start < pl.header_range.vert_end {
                            r_pass.set_bind_group(0, &any_bg.1, &[]);
                            r_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
                            r_pass.draw(pl.header_range.vert_start..pl.header_range.vert_end, 0..1);
                        }
                        for i in pl.header_range.char_start..pl.header_range.char_end {
                            r_pass.set_bind_group(0, char_draws[i].1, &[]);
                            r_pass.set_vertex_buffer(0, char_draws[i].0.slice(..));
                            r_pass.draw(0..6, 0..1);
                        }

                        // timeline — starts where steps actually begin
                        r_pass.set_scissor_rect(tx, content_y, tw.max(1), content_h);
                        if pl.timeline_range.vert_start < pl.timeline_range.vert_end {
                            r_pass.set_bind_group(0, &any_bg.1, &[]);
                            r_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
                            r_pass.draw(pl.timeline_range.vert_start..pl.timeline_range.vert_end, 0..1);
                        }
                        for i in pl.timeline_range.char_start..pl.timeline_range.char_end {
                            r_pass.set_bind_group(0, char_draws[i].1, &[]);
                            r_pass.set_vertex_buffer(0, char_draws[i].0.slice(..));
                            r_pass.draw(0..6, 0..1);
                        }
                    }
                    r_pass.set_scissor_rect(0, 0, self.surface_config.width, self.surface_config.height); // add this

                    continue;
                }

                if range.vert_start < range.vert_end {
                    r_pass.set_bind_group(0, &any_bg.1, &[]);
                    r_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
                    r_pass.draw(range.vert_start..range.vert_end, 0..1);
                }
                for i in range.char_start..range.char_end {
                    r_pass.set_bind_group(0, char_draws[i].1, &[]);
                    r_pass.set_vertex_buffer(0, char_draws[i].0.slice(..));
                    r_pass.draw(0..6, 0..1);
                }

                if is_playlist {
                    // reset to full surface
                    r_pass.set_scissor_rect(0, 0, self.surface_config.width, self.surface_config.height);
                }
            }
            // toolbar on top of everything
            if toolbar_vert_start < self.num_vertices {
                r_pass.set_bind_group(0, &any_bg.1, &[]);
                r_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
                r_pass.draw(toolbar_vert_start..self.num_vertices, 0..1);
            }
            for i in toolbar_char_start..char_draws.len() {
                r_pass.set_bind_group(0, char_draws[i].1, &[]);
                r_pass.set_vertex_buffer(0, char_draws[i].0.slice(..));
                r_pass.draw(0..6, 0..1);
            }
        }

        self.queue.submit(Some(encoder.finish()));
        frame.present();

        click_result
    }
}
