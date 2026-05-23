use crate::app::MouseState;

use crate::graphics::font::{MONO_FONT, ROBOTO_FONT};
use crate::graphics::mini_window::{piano_roll, PianoRollDrawRanges, PIANO_ROLL_ID};
use crate::graphics::primitives::NO_RADIUS;
use crate::graphics::{
    color::{DARK_GRAY, WHITE},
    components::{footer, pattern_tray},
    context_menu::ContextMenu,
    font::TextItem,
    icons::Tooltip,
    mini_window::{
        instrument, mixer, playlist, sequencer,
        sequencer::{ACTIONS_Y_OFFSET, KNOB_OFFSET, KNOB_RADIUS, TRACK_GAP},
        MiniWindow, PlaylistDrawRanges, WindowDrawRange, WindowKind, MIXER_ID, PLAYLIST_ID, SEQUENCER_ID,
    },
    primitives::{ScreenConfig, Vertex, PAD_2, PAD_4, PAD_8},
    widgets::{Rectangle, TITLEBAR_HEIGHT, TOOLBAR_THICKNESS, TOOLBAR_Y},
};
use crate::project::{AudioBlock, AudioBlockType, Instrument, PatternData};
use fontdue::layout::{CoordinateSystem, Layout, TextStyle};
use std::{borrow::Cow, collections::HashMap};
use wgpu::{
    util::DeviceExt, Color, CommandEncoderDescriptor, DeviceDescriptor, Features, FragmentState, Instance, Limits, LoadOp, MemoryHints, Operations,
    PowerPreference, RenderPassColorAttachment, RenderPassDescriptor, RenderPipeline, RenderPipelineDescriptor, RequestAdapterOptions,
    ShaderModuleDescriptor, ShaderSource, StoreOp, SurfaceConfiguration, TextureFormat, TextureViewDescriptor, VertexState,
};
use winit::{
    dpi::PhysicalSize,
    event_loop::EventLoopProxy,
    window::{CursorIcon, Window},
};

pub mod color;
pub mod components;
pub mod context_menu;
pub mod font;
pub mod icons;
pub mod mini_window;
pub mod primitives;
pub mod widgets;

pub type Rc<T> = std::sync::Arc<T>;

#[derive(Debug)]
pub enum ClickResult {
    Step(usize, usize, usize),
    Mute(usize),
    Stop,
    ChangeBpmUp,
    ChangeBpmDown,
    ChangeBpm(f32),
    TogglePlay,
    ProjectFileDialog,
    InstrumentFileDialog,
    DeleteTrack(usize),
    DeletePlaylistPattern(usize),
    DeletePattern(usize),
    AddPlaylistPattern(usize, u32, usize, AudioBlockType),
    AddPlaylist,
    ToggleSequencerWindow,
    ToggleMixerWindow,
    TogglePlaylistWindow,
    TogglePianoRollWindow,
    ToggleInstrumentWindow(usize),
    SelectPattern(usize),
    OpenPatternMenu(f32, f32, usize),
    OpenTrackMenu(f32, f32, usize),
    CloseContextMenu,
    None,
}
impl ClickResult {
    pub fn or(self, other: ClickResult) -> ClickResult {
        if matches!(self, ClickResult::None) {
            other
        } else {
            self
        }
    }
}
pub enum DragResult {
    DragVolumeSlider(f32),
    DragVolumeKnob(usize, f32),
    None,
}

pub const ONE_MEGABYTE: u64 = 1024 * 1024;

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
    let instruments: Vec<Instrument> = Vec::new();
    let events: Vec<AudioBlock> = Vec::new();

    let context_menu = None;

    let vertex_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("Vertex Buffer"),
        size: ONE_MEGABYTE * 8,
        usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });

    let mut mini_windows: Vec<MiniWindow> = Vec::new();

    // init sequencer_window
    let sequencer_window = MiniWindow::new(256.0, 128.0, 1092.0, 100.0, "Sequencer", WindowKind::Sequencer, false);
    mini_windows.push(sequencer_window);

    // init playlist_window
    let playlist_window = MiniWindow::new(900.0, 600.0, 1500.0, 900.0, "Playlist", WindowKind::Playlist, true);
    mini_windows.push(playlist_window);

    // init mixer window
    let mixer_window = MiniWindow::new(128.0, 500.0, 800.0, 300.0, "Mixer", WindowKind::Mixer, false);
    mini_windows.push(mixer_window);

    // init piano window
    let piano_window = MiniWindow::new(256.0, 400.0, 1092.0, 600.0, "Piano", WindowKind::PianoRoll, true);
    mini_windows.push(piano_window);
    let roboto = (
        ROBOTO_FONT,
        include_bytes!("../../assets/fonts/Roboto-VariableFont_wdth,wght.ttf") as &[u8],
    );
    let mono = (MONO_FONT, include_bytes!("../../assets/fonts/IBMPlexMono-Regular.ttf") as &[u8]);
    let mut font_cache: HashMap<String, fontdue::Font> = HashMap::new();
    let mut glyph_cache: HashMap<String, HashMap<(char, u32), (wgpu::Texture, wgpu::BindGroup, fontdue::Metrics)>> = HashMap::new();

    let bind_group_layout = font::create_bind_group_layout(&device);
    let render_pipeline = create_pipeline(&device, surface_config.format, &bind_group_layout);

    for (name, bytes) in [roboto, mono] {
        let font = fontdue::Font::from_bytes(bytes, fontdue::FontSettings::default()).unwrap();
        let cache = font::build_glyph_cache(&device, &queue, &font, &[8.0, 10.0, 12.0, 14.0, 16.0, 18.0, 24.0, 32.0]);
        font_cache.insert(name.to_string(), font);
        glyph_cache.insert(name.to_string(), cache);
    }
    // init icon's
    let mut icon_cache = HashMap::new();
    for name in icons::ICON_NAMES {
        let svg_str = std::fs::read_to_string(format!("assets/icons/{}.svg", name)).unwrap();
        let icon = icons::IconSvg {
            width: 128.0,
            height: 128.0,
            path: svg_str,
        };
        let (texture, bind_group, _, _, _) = icons::rasterize_icon(&device, &queue, icon);
        icon_cache.insert(name.to_string(), (texture, bind_group));
    }

    let gfx = Graphics {
        // graphics
        window: window.clone(),
        surface,
        project_path: "".to_string(),
        surface_config,
        device,
        queue,
        render_pipeline,

        // shapes
        vertex_buffer,
        num_vertices: vertices.len() as u32,
        frame_ms: 0.0,

        // song information
        instruments,
        patterns,
        events,
        active_step: 0,
        active_pattern_id: 0,
        bpm: 120.0,
        is_playing: false,
        master_volume: 0.5,

        // fonts
        glyph_cache,
        font_cache,

        // iconography
        icon_cache,
        tooltip: None,

        // ui state
        dragging_knob: None,
        mini_windows,
        dragging_window: None,
        dragging: false,
        playlist_scroll_x: 0.0,
        playlist_scroll_y: 0.0,
        piano_roll_scroll_x: 0.0,
        piano_roll_scroll_y: 1015.0,
        z_order: vec![SEQUENCER_ID, PLAYLIST_ID, MIXER_ID, PIANO_ROLL_ID],
        context_menu,
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
    pub window: Rc<Window>,
    surface: wgpu::Surface<'static>,
    surface_config: SurfaceConfiguration,
    device: wgpu::Device,
    queue: wgpu::Queue,
    render_pipeline: RenderPipeline,
    vertex_buffer: wgpu::Buffer,

    // text
    glyph_cache: HashMap<String, HashMap<(char, u32), (wgpu::Texture, wgpu::BindGroup, fontdue::Metrics)>>,
    font_cache: HashMap<String, fontdue::Font>,

    //ui
    pub mini_windows: Vec<MiniWindow>,
    num_vertices: u32,
    pub active_pattern_id: usize,
    pub z_order: Vec<usize>,
    pub context_menu: Option<ContextMenu>,
    icon_cache: HashMap<String, (wgpu::Texture, wgpu::BindGroup)>,
    pub tooltip: Option<Tooltip>,
    pub frame_ms: f32,

    // song
    pub project_path: String,
    pub instruments: Vec<Instrument>,
    pub patterns: Vec<PatternData>,
    pub events: Vec<AudioBlock>,
    pub active_step: usize,
    pub bpm: f32,
    pub is_playing: bool,
    pub master_volume: f32,

    // dragging
    pub dragging_knob: Option<usize>,
    pub dragging_window: Option<usize>,
    pub dragging: bool,

    // scrolling
    pub playlist_scroll_x: f32,
    pub playlist_scroll_y: f32,
    pub piano_roll_scroll_x: f32,
    pub piano_roll_scroll_y: f32,
}

pub fn bring_to_front(z_order: &mut Vec<usize>, id: usize) {
    z_order.retain(|&x| x != id);
    z_order.push(id);
}

impl Graphics {
    fn draw_geom(r_pass: &mut wgpu::RenderPass, vertex_buffer: &wgpu::Buffer, any_bg: &wgpu::BindGroup, start: u32, end: u32) {
        if start < end {
            r_pass.set_bind_group(0, any_bg, &[]);
            r_pass.set_vertex_buffer(0, vertex_buffer.slice(..));
            r_pass.draw(start..end, 0..1);
        }
    }

    fn draw_chars(r_pass: &mut wgpu::RenderPass, char_draws: &[(wgpu::Buffer, &wgpu::BindGroup)], start: usize, end: usize) {
        for i in start..end {
            r_pass.set_bind_group(0, char_draws[i].1, &[]);
            r_pass.set_vertex_buffer(0, char_draws[i].0.slice(..));
            r_pass.draw(0..6, 0..1);
        }
    }
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

    /// Track if/where the user's mouse is dragging a component
    pub fn handle_drag(&mut self, x: f32, y: f32, dy: f32, dx: f32) -> DragResult {
        let sequencer_window = &self.mini_windows[SEQUENCER_ID];
        let mixer_window = &self.mini_windows[MIXER_ID];

        // volume knobs
        if self.dragging_window == None {
            if self.dragging_knob == None {
                let y_ceiling: f32 = mixer_window.y;
                let track_height: f32 = 164.0;
                let padding = 32.0;
                if x > mixer_window.x - padding + 24.0
                    && x < mixer_window.x + padding + 24.0
                    && y > mixer_window.y
                    && y < mixer_window.y + track_height
                {
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
                    x: sequencer_window.x + KNOB_OFFSET,
                    y: sequencer_window.y + (i as f32 * TRACK_GAP) + ACTIONS_Y_OFFSET + PAD_8,
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
        }

        // update window dragging
        if let Some(i) = self.dragging_window {
            let win = &mut self.mini_windows[i];

            // only enforce that the titlebar stays reachable
            let max_y = self.surface_config.height as f32 - TITLEBAR_HEIGHT;

            win.x = (win.x + dx).clamp(-(win.width - 64.0), self.surface_config.width as f32 - 246.0);
            win.y = (win.y + dy).clamp(TITLEBAR_HEIGHT + TOOLBAR_Y, max_y);

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

    /// main window resizing
    pub fn resize(&mut self, new_size: PhysicalSize<u32>) {
        self.surface_config.width = new_size.width.max(1);
        self.surface_config.height = new_size.height.max(1);
        self.surface.configure(&self.device, &self.surface_config);
    }

    /// pushing icons to draw
    fn push_icon_draw<'a>(
        icon_cache: &'a HashMap<String, (wgpu::Texture, wgpu::BindGroup)>,
        device: &wgpu::Device,
        screen_config: &ScreenConfig,
        name: &str,
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        icon_draws: &mut Vec<(wgpu::Buffer, &'a wgpu::BindGroup)>,
    ) {
        if let Some((_, bind_group)) = icon_cache.get(name) {
            let verts = icons::draw_icon(x, y, w, h, screen_config);
            let buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: None,
                contents: bytemuck::cast_slice(&verts),
                usage: wgpu::BufferUsages::VERTEX,
            });
            icon_draws.push((buf, bind_group));
        }
    }

    /// pushing texts to draw
    fn push_text_draws<'a>(
        texts: &[TextItem],
        font_cache: &HashMap<String, fontdue::Font>,
        glyph_cache: &'a HashMap<String, HashMap<(char, u32), (wgpu::Texture, wgpu::BindGroup, fontdue::Metrics)>>,
        device: &wgpu::Device,
        screen_config: &ScreenConfig,
        char_draws: &mut Vec<(wgpu::Buffer, &'a wgpu::BindGroup)>,
    ) {
        for text_item in texts {
            let Some(font) = font_cache.get(text_item.font) else { continue };
            let Some(gcache) = glyph_cache.get(text_item.font) else { continue };
            let mut layout = Layout::new(CoordinateSystem::PositiveYDown);
            layout.append(&[font], &TextStyle::new(&text_item.text, text_item.size, 0));
            for glyph in layout.glyphs() {
                if let Some((_, bind_group, _)) = gcache.get(&(glyph.parent, text_item.size as u32)) {
                    let gverts = font::draw_glyph(
                        text_item.x + glyph.x,
                        text_item.y + glyph.y,
                        glyph.width as f32,
                        glyph.height as f32,
                        screen_config,
                        text_item.color,
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

    /// main draw loop for the GUI - uses mouse state to return mouse input interactivity
    pub fn draw(&mut self, mouse_state: &MouseState) -> (ClickResult, CursorIcon) {
        let frame = self.surface.get_current_texture().expect("Failed to acquire next swap chain texture.");
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
        let mut icon_draws: Vec<(wgpu::Buffer, &wgpu::BindGroup)> = Vec::new();
        let mut window_ranges: Vec<WindowDrawRange> = Vec::new();
        let mut playlist_window_ranges: Option<PlaylistDrawRanges> = None;
        let mut piano_roll_ranges: Option<PianoRollDrawRanges> = None;
        for &id in &self.z_order {
            let vert_start = vertices.len() as u32;
            let char_start = char_draws.len();

            match id {
                SEQUENCER_ID if self.mini_windows[SEQUENCER_ID].is_open => {
                    let window = &self.mini_windows[SEQUENCER_ID];
                    let (verts, texts, icons, result, cursor) = sequencer::draw(
                        window,
                        &mut self.patterns,
                        &mut self.instruments,
                        self.active_pattern_id,
                        self.active_step,
                        &mouse_state,
                        &screen_config,
                    );
                    vertices.extend(verts);
                    Graphics::push_text_draws(&texts, &self.font_cache, &self.glyph_cache, &self.device, &screen_config, &mut char_draws);
                    if cursor != CursorIcon::Default {
                        cursor_icon = cursor;
                    }

                    for icon in icons {
                        Graphics::push_icon_draw(
                            &self.icon_cache,
                            &self.device,
                            &screen_config,
                            icon.name,
                            icon.x,
                            icon.y,
                            icon.width,
                            icon.height,
                            &mut icon_draws,
                        )
                    }

                    click_result = click_result.or(result);
                }
                PLAYLIST_ID if self.mini_windows[PLAYLIST_ID].is_open => {
                    let window = &self.mini_windows[PLAYLIST_ID];
                    let (static_verts, static_texts, timeline_verts, timeline_texts, header_verts, header_texts, result, cursor) = playlist::draw(
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

                    let static_vert_start = vertices.len() as u32;
                    let static_char_start = char_draws.len();
                    vertices.extend(static_verts);
                    Graphics::push_text_draws(
                        &static_texts,
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
                    vertices.extend(header_verts);
                    Graphics::push_text_draws(
                        &header_texts,
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
                    vertices.extend(timeline_verts);
                    Graphics::push_text_draws(
                        &timeline_texts,
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
                    let (verts, texts, result, cursor) = mixer::draw(window, self.master_volume, &screen_config, &mouse_state);
                    vertices.extend(verts);
                    Graphics::push_text_draws(&texts, &self.font_cache, &self.glyph_cache, &self.device, &screen_config, &mut char_draws);
                    click_result = click_result.or(result);
                }
                PIANO_ROLL_ID if self.mini_windows[PIANO_ROLL_ID].is_open => {
                    let window = &self.mini_windows[PIANO_ROLL_ID];
                    let (verts, texts, piano_key_verts, piano_key_texts, grid_verts, grid_texts, result, cursor) =
                        piano_roll::window::draw(window, &mouse_state, self.piano_roll_scroll_x, self.piano_roll_scroll_y, &screen_config);

                    // static (titlebar + background) — no scroll
                    vertices.extend(verts);
                    Graphics::push_text_draws(&texts, &self.font_cache, &self.glyph_cache, &self.device, &screen_config, &mut char_draws);

                    // scrollable content
                    let piano_content_vert_start = vertices.len() as u32;
                    let piano_content_char_start = char_draws.len();
                    vertices.extend(piano_key_verts);
                    Graphics::push_text_draws(
                        &piano_key_texts,
                        &self.font_cache,
                        &self.glyph_cache,
                        &self.device,
                        &screen_config,
                        &mut char_draws,
                    );

                    // grid content
                    let grid_vert_start = vertices.len() as u32;
                    let grid_char_start = char_draws.len();
                    vertices.extend(grid_verts);
                    Graphics::push_text_draws(
                        &grid_texts,
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
                }
                instrument => {
                    let window = &self.mini_windows[instrument];
                    if window.is_open {
                        if let WindowKind::InstrumentDetail(track) = window.window_kind {
                            let (verts, texts, result, cursor) = instrument::draw(window, &mouse_state, &screen_config, &self.instruments[track]);
                            vertices.extend(verts);
                            click_result = click_result.or(result);
                            if !matches!(cursor, CursorIcon::Default) {
                                cursor_icon = cursor;
                            }
                            Graphics::push_text_draws(&texts, &self.font_cache, &self.glyph_cache, &self.device, &screen_config, &mut char_draws);
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

        if let ClickResult::DeleteTrack(i) = click_result {
            self.instruments.remove(i);
        }
        if let ClickResult::DeletePlaylistPattern(id) = click_result {
            self.events.retain(|e| e.id != id);
        }

        // --- toolbar (pattern tray + toolbar bar) ---
        let toolbar_vert_start = vertices.len() as u32;
        let toolbar_char_start = char_draws.len();

        let (verts, texts, result, icon) = pattern_tray::draw(&screen_config, &self.patterns, self.active_pattern_id, mouse_state);
        vertices.extend(verts);
        if icon != CursorIcon::Default {
            cursor_icon = icon;
        }
        click_result = click_result.or(result);
        Graphics::push_text_draws(&texts, &self.font_cache, &self.glyph_cache, &self.device, &screen_config, &mut char_draws);

        let (toolbar_verts, toolbar_texts, toolbar_icons, toolbar_result, toolbar_cursor, toolbar_tooltip) =
            components::toolbar::draw_toolbar(mouse_state, &screen_config, self.bpm, self.is_playing, self.active_step);
        vertices.extend(toolbar_verts);

        for icon in toolbar_icons {
            Graphics::push_icon_draw(
                &self.icon_cache,
                &self.device,
                &screen_config,
                icon.name,
                icon.x,
                icon.y,
                icon.width,
                icon.height,
                &mut icon_draws,
            )
        }

        if toolbar_cursor != CursorIcon::Default {
            cursor_icon = toolbar_cursor;
        }

        if toolbar_tooltip.is_some() {
            self.tooltip = toolbar_tooltip;
        }
        let tooltip_vert_start = vertices.len() as u32;
        let tooltip_char_start = char_draws.len();
        // tool tip
        if let Some(tt) = &self.tooltip {
            let tooltip_rectangle = Rectangle {
                x: tt.x,
                y: tt.y,
                width: 128.0,
                height: 24.0,
            };
            vertices.extend(tooltip_rectangle.draw(&screen_config, DARK_GRAY, NO_RADIUS));
            if let Some(text) = tt.text {
                let tooltip_text = [TextItem {
                    text: text.to_string(),
                    x: tt.x + PAD_4,
                    y: tt.y + PAD_2,
                    size: 14.0,
                    font: MONO_FONT,
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

        let tooltip_vert_end = vertices.len() as u32;
        let tooltip_char_end = char_draws.len();

        match toolbar_result {
            ClickResult::Stop => {
                self.is_playing = !self.is_playing;
                self.active_step = 0;
                click_result = ClickResult::Stop;
            }
            ClickResult::None => {}
            other => {
                click_result = click_result.or(other);
            }
        }
        Graphics::push_text_draws(
            &toolbar_texts,
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
            let (verts, menu_texts, menu_result, menu_cursor) = menu.draw(&screen_config, mouse_state);
            vertices.extend(verts);
            Graphics::push_text_draws(
                &menu_texts,
                &self.font_cache,
                &self.glyph_cache,
                &self.device,
                &screen_config,
                &mut char_draws,
            );

            if menu_cursor != CursorIcon::Default {
                cursor_icon = menu_cursor;
            }
            click_result = click_result.or(menu_result);

            // delete the track and close the menu
            if let ClickResult::DeleteTrack(i) = click_result {
                self.instruments.remove(i);
                self.context_menu = None;
            }
        }

        let context_menu_vert_end = vertices.len() as u32;
        let context_menu_char_end = char_draws.len();

        // --- footer ---
        let footer_vert_start = vertices.len() as u32;
        let footer_char_start = char_draws.len();

        let (verts, footer_texts) = footer::draw(&screen_config, &self.project_path, 1000.0 / self.frame_ms);
        vertices.extend(verts);
        Graphics::push_text_draws(
            &footer_texts,
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
            let any_bg = self.glyph_cache.values().next().unwrap().values().next().unwrap();

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
                            &any_bg.1,
                            pr.static_range.vert_start,
                            pr.static_range.vert_end,
                        );
                        Graphics::draw_chars(&mut r_pass, &char_draws, pr.static_range.char_start, pr.static_range.char_end);

                        let (sx, sy, sw2, sh2) = Self::safe_scissor(wx, content_y, key_w, content_h, sw, sh);
                        r_pass.set_scissor_rect(sx, sy, sw2, sh2);
                        Graphics::draw_geom(
                            &mut r_pass,
                            &self.vertex_buffer,
                            &any_bg.1,
                            pr.piano_range.vert_start,
                            pr.piano_range.vert_end,
                        );
                        Graphics::draw_chars(&mut r_pass, &char_draws, pr.piano_range.char_start, pr.piano_range.char_end);

                        let (sx, sy, sw2, sh2) = Self::safe_scissor(grid_x, content_y, grid_w, content_h, sw, sh);
                        r_pass.set_scissor_rect(sx, sy, sw2, sh2);
                        Graphics::draw_geom(
                            &mut r_pass,
                            &self.vertex_buffer,
                            &any_bg.1,
                            pr.grid_range.vert_start,
                            pr.grid_range.vert_end,
                        );
                        Graphics::draw_chars(&mut r_pass, &char_draws, pr.grid_range.char_start, pr.grid_range.char_end);
                    }
                    r_pass.set_scissor_rect(0, 0, self.surface_config.width, self.surface_config.height);
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
                            &any_bg.1,
                            pl.static_range.vert_start,
                            pl.static_range.vert_end,
                        );
                        Graphics::draw_chars(&mut r_pass, &char_draws, pl.static_range.char_start, pl.static_range.char_end);

                        let (sx, sy, sw2, sh2) = Self::safe_scissor(wx, content_y, header_w, content_h, sw, sh);
                        r_pass.set_scissor_rect(sx, sy, sw2, sh2);
                        Graphics::draw_geom(
                            &mut r_pass,
                            &self.vertex_buffer,
                            &any_bg.1,
                            pl.header_range.vert_start,
                            pl.header_range.vert_end,
                        );
                        Graphics::draw_chars(&mut r_pass, &char_draws, pl.header_range.char_start, pl.header_range.char_end);

                        let (sx, sy, sw2, sh2) = Self::safe_scissor(header_x, content_y, timeline_w, content_h, sw, sh);
                        r_pass.set_scissor_rect(sx, sy, sw2, sh2);
                        Graphics::draw_geom(
                            &mut r_pass,
                            &self.vertex_buffer,
                            &any_bg.1,
                            pl.timeline_range.vert_start,
                            pl.timeline_range.vert_end,
                        );
                        Graphics::draw_chars(&mut r_pass, &char_draws, pl.timeline_range.char_start, pl.timeline_range.char_end);
                    }
                    r_pass.set_scissor_rect(0, 0, self.surface_config.width, self.surface_config.height);
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
            }

            // toolbar

            Graphics::draw_geom(&mut r_pass, &self.vertex_buffer, &any_bg.1, toolbar_vert_start, toolbar_vert_end);
            Graphics::draw_chars(&mut r_pass, &char_draws, toolbar_char_start, toolbar_char_end);

            for i in 0..icon_draws.len() {
                r_pass.set_bind_group(0, icon_draws[i].1, &[]);
                r_pass.set_vertex_buffer(0, icon_draws[i].0.slice(..));
                r_pass.draw(0..6, 0..1);
            }
            // tooltip
            Graphics::draw_geom(&mut r_pass, &self.vertex_buffer, &any_bg.1, tooltip_vert_start, tooltip_vert_end);
            Graphics::draw_chars(&mut r_pass, &char_draws, tooltip_char_start, tooltip_char_end);

            // context menu
            Graphics::draw_geom(
                &mut r_pass,
                &self.vertex_buffer,
                &any_bg.1,
                context_menu_vert_start,
                context_menu_vert_end,
            );
            Graphics::draw_chars(&mut r_pass, &char_draws, context_menu_char_start, context_menu_char_end);

            // footer
            Graphics::draw_geom(&mut r_pass, &self.vertex_buffer, &any_bg.1, footer_vert_start, footer_vert_end);
            Graphics::draw_chars(&mut r_pass, &char_draws, footer_char_start, footer_char_end);
        }

        self.queue.submit(Some(encoder.finish()));
        frame.present();

        (click_result, cursor_icon)
    }
    fn safe_scissor(x: u32, y: u32, w: u32, h: u32, sw: u32, sh: u32) -> (u32, u32, u32, u32) {
        let x = x.min(sw.saturating_sub(1));
        let y = y.min(sh.saturating_sub(1));
        let w = w.min(sw.saturating_sub(x)).max(1);
        let h = h.min(sh.saturating_sub(y)).max(1);
        (x, y, w, h)
    }
}
