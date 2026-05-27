pub mod color;
pub mod components;
pub mod context_menu;
pub mod drag;
pub mod draw;
pub mod font;
pub mod icons;
pub mod mini_window;
pub mod primitives;
pub mod widgets;

pub const ONE_MEGABYTE: u64 = 1024 * 1024;

use crate::app::{MouseState, PianoRollState};
use crate::audio::DEFAULT_BPM;
use crate::graphics::{
    color::{Color, DARK_GRAY, WHITE},
    components::{footer, side_panel},
    context_menu::ContextMenu,
    font::{TextItem, MONOSPACED, ROBOTO},
    icons::{push_icon_draw, Tooltip},
    mini_window::{
        mixer, piano_roll,
        piano_roll::PIANO_ROLL_DEFAULT_Y,
        playlist, sequencer,
        sequencer::{ACTIONS_Y_OFFSET, KNOB_OFFSET, KNOB_RADIUS, TRACK_GAP},
        track, MiniWindow, PianoRollDrawRanges, PlaylistDrawRanges, WindowDrawRange, WindowKind, MIXER_ID, PIANO_ROLL_ID, PLAYLIST_ID, SEQUENCER_ID,
    },
    primitives::*,
    widgets::*,
};

use crate::project::{AudioBlock, AudioBlockType, PatternData, Track};
use fontdue::layout::{CoordinateSystem, Layout, TextStyle};
use std::{borrow::Cow, collections::HashMap};

use wgpu::{
    util::DeviceExt, CommandEncoderDescriptor, DeviceDescriptor, Features, FragmentState, Instance, Limits, LoadOp, MemoryHints, Operations,
    PowerPreference, RenderPassColorAttachment, RenderPassDescriptor, RenderPipeline, RenderPipelineDescriptor, RequestAdapterOptions,
    ShaderModuleDescriptor, ShaderSource, StoreOp, SurfaceConfiguration, TextureFormat, TextureViewDescriptor, VertexState,
};

use winit::{
    dpi::PhysicalSize,
    event_loop::EventLoopProxy,
    window::{CursorIcon, Window},
};

pub type Rc<T> = std::sync::Arc<T>;

#[derive(Debug)]
pub enum ClickResult {
    // sequencer
    ToggleStep(usize, usize, usize),   // pattern_id, track_id, step_idx
    ToggleNote(usize, u32, usize, u8), // pattern_id, track_id, step_idx, pitch
    ToggleTrackMute(usize),
    DeleteTrack(usize),
    ToggleSequencerWindow,
    OpenTrackFileLocation(String),

    // toolbar
    Stop,
    ChangeBpmUp,
    ChangeBpmDown,
    ChangeBpm(f32),
    TogglePlay,
    ProjectFileDialog,
    TrackFileDialog,

    // menus
    OpenTrackMenu(f32, f32, usize, usize),
    CloseContextMenu,

    // patterns
    DeletePlaylistPattern(usize),
    DeletePattern(usize),
    DuplicatePattern(usize),
    CreatePattern,
    AddPlaylistPattern(usize, u32, usize, AudioBlockType),
    SelectPattern(usize),
    OpenPatternMenu(f32, f32, usize),
    StartResizeEvent(usize),

    // piano roll
    TogglePianoRollWindow,
    LoadPianoRoll(PianoRollState),

    // toggle ui components
    ToggleMixerWindow,
    TogglePlaylistWindow,
    ToggleTrackWindow(usize),
    TogglePatternTray,
    ToggleTrackTray,

    // no click result
    None,
}
impl ClickResult {
    /// combine click results, prioritizing the first if it's not None
    pub fn or(self, other: ClickResult) -> ClickResult {
        if matches!(self, ClickResult::None) {
            other
        } else {
            self
        }
    }
}

/// Initialize the graphics with default/loaded state and find driver/display info
pub async fn create_graphics(window: Rc<Window>, proxy: EventLoopProxy<Graphics>) {
    // Context for all other wgpu objects. Instance of wgpu.
    let instance = Instance::default();
    // Creates a new surface targeting a given window/canvas/surface/etc..
    // Internally, this creates surfaces for all backends that are enabled for this WGPU instance.
    let surface = instance.create_surface(Rc::clone(&window)).unwrap();

    // Handle to a physical graphics and/or compute device.
    let adapter = instance
        .request_adapter(&RequestAdapterOptions {
            power_preference: PowerPreference::default(),
            force_fallback_adapter: false,
            compatible_surface: Some(&surface),
        })
        .await
        .expect("Could not get an adapter (GPU).");

    // Requests a connection to a physical device, creating a logical device.
    // Returns the Device together with a Queue that executes command buffers.
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

    // Returns the physical size of the window’s client area
    let size = window.inner_size();
    let width = size.width.max(1);
    let height = size.height.max(1);
    let surface_config = surface.get_default_config(&adapter, width, height).unwrap();
    surface.configure(&device, &surface_config);

    // init graphics state
    let vertices: Vec<Vertex> = Vec::new();
    let patterns: Vec<PatternData> = Vec::new();
    let tracks: Vec<Track> = Vec::new();
    let events: Vec<AudioBlock> = Vec::new();
    let mut mini_windows: Vec<MiniWindow> = Vec::new();
    let context_menu = None;

    // vertex buffer for collecting shapes to draw each frame
    let vertex_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("Vertex Buffer"),
        size: ONE_MEGABYTE * 8,
        usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });

    // init windows ; TODO: remove hardcoded coordinates, should be dynamic based on saved state
    let playlist_window = MiniWindow::new(900.0, 600.0, 1500.0, 900.0, "Playlist", WindowKind::Playlist, true);

    let mixer_window = MiniWindow::new(128.0, 500.0, 800.0, 300.0, "Mixer", WindowKind::Mixer, false);

    let piano_window = MiniWindow::new(256.0, 400.0, 1092.0, 600.0, "Piano", WindowKind::PianoRoll, true);

    let sequencer_window = MiniWindow::new(256.0, 128.0, 1092.0, 100.0, "Sequencer", WindowKind::Sequencer, false);

    mini_windows.push(sequencer_window); // 0
    mini_windows.push(playlist_window); // 1
    mini_windows.push(mixer_window); // 2
    mini_windows.push(piano_window); // 3

    // fonts
    let roboto = (ROBOTO, include_bytes!("../../assets/fonts/Roboto-VariableFont_wdth,wght.ttf") as &[u8]);
    let mono = (MONOSPACED, include_bytes!("../../assets/fonts/IBMPlexMono-Regular.ttf") as &[u8]);
    let mut font_cache: HashMap<String, fontdue::Font> = HashMap::new();
    let mut glyph_cache: HashMap<String, HashMap<(char, u32), (wgpu::Texture, wgpu::BindGroup, fontdue::Metrics)>> = HashMap::new();
    let bind_group_layout = font::create_bind_group_layout(&device);
    for (name, bytes) in [roboto, mono] {
        let font = fontdue::Font::from_bytes(bytes, fontdue::FontSettings::default()).unwrap();
        let cache = font::build_glyph_cache(&device, &queue, &font, &[8.0, 10.0, 12.0, 14.0, 16.0, 18.0, 24.0, 32.0]);
        font_cache.insert(name.to_string(), font);
        glyph_cache.insert(name.to_string(), cache);
    }

    // svg icons
    let mut icon_cache = HashMap::new();
    for name in icons::ICON_NAMES_128 {
        let svg_str = std::fs::read_to_string(format!("assets/icons/128x128/{}.svg", name)).unwrap();
        let icon = icons::IconSvg {
            width: 128.0,
            height: 128.0,
            path: svg_str,
        };
        let (texture, bind_group, _, _, _) = icons::rasterize_icon(&device, &queue, icon);
        icon_cache.insert(name.to_string(), (texture, bind_group));
    }
    for name in icons::ICON_NAMES_32 {
        let svg_str = std::fs::read_to_string(format!("assets/icons/32x32/{}.svg", name)).unwrap();
        let icon = icons::IconSvg {
            width: 32.0,
            height: 32.0,
            path: svg_str,
        };
        let (texture, bind_group, _, _, _) = icons::rasterize_icon(&device, &queue, icon);
        icon_cache.insert(name.to_string(), (texture, bind_group));
    }

    // wgsl shader and render pipeline setup
    let render_pipeline = create_pipeline(&device, surface_config.format, &bind_group_layout);

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
        tracks,
        patterns,
        events,
        active_step: 0,
        active_pattern_id: 0,
        bpm: DEFAULT_BPM,
        is_playing: false,
        master_volume: 0.5,

        // fonts
        glyph_cache,
        font_cache,

        // iconography
        icon_cache,
        tooltip: None,
        piano_roll_state: None,

        // ui state
        dragging_knob: None,
        mini_windows,
        dragging_window: None,
        dragging: false,
        playlist_scroll_x: 0.0,
        playlist_scroll_y: 0.0,
        piano_roll_scroll_x: 0.0,
        piano_roll_scroll_y: PIANO_ROLL_DEFAULT_Y,
        z_order: vec![SEQUENCER_ID, PLAYLIST_ID, MIXER_ID, PIANO_ROLL_ID],
        context_menu,
        resizing_event: None,
        resize_drag_accumulator: 0.0,
        show_track_tray: true,
        show_pattern_tray: true,
    };

    let _ = proxy.send_event(gfx);
}

/// create the render pipeline, which describes how to process vertices and fragments, including shaders, blending, and output formats
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

/// Main struct holding all graphics state, including wgpu objects, loaded fonts and icons, and UI state like open windows and dragging
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
    pub piano_roll_state: Option<PianoRollState>,
    pub z_order: Vec<usize>,
    pub context_menu: Option<ContextMenu>,
    icon_cache: HashMap<String, (wgpu::Texture, wgpu::BindGroup)>,
    pub tooltip: Option<Tooltip>,
    pub frame_ms: f32,
    pub show_track_tray: bool,
    pub show_pattern_tray: bool,

    // song
    pub project_path: String,
    pub tracks: Vec<Track>,
    pub patterns: Vec<PatternData>,
    pub events: Vec<AudioBlock>,
    pub active_step: usize,
    pub bpm: f32,
    pub is_playing: bool,
    pub master_volume: f32,

    // dragging
    pub dragging_knob: Option<usize>,   // volume knob
    pub dragging_window: Option<usize>, // window titlebar
    pub dragging: bool,
    pub resizing_event: Option<usize>, // pattern resizing in playlist
    pub resize_drag_accumulator: f32,

    // scrolling
    pub playlist_scroll_x: f32,
    pub playlist_scroll_y: f32,
    pub piano_roll_scroll_x: f32,
    pub piano_roll_scroll_y: f32,
}

/// Bring a window to the front of the z-order
pub fn bring_to_front(z_order: &mut Vec<usize>, id: usize) {
    z_order.retain(|&x| x != id);
    z_order.push(id);
}

impl Graphics {
    // draw a list of icons, each with their own texture and bind group
    pub fn request_redraw(&self) {
        self.window.request_redraw();
    }

    // loading project data into the graphics state, called from the app when a project is loaded or created new
    pub fn load_track(&mut self, i: Track) {
        if i.data.id >= self.tracks.len() as u32 {
            self.tracks.push(i);
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

    /// main window resizing
    pub fn resize(&mut self, new_size: PhysicalSize<u32>) {
        self.surface_config.width = new_size.width.max(1);
        self.surface_config.height = new_size.height.max(1);
        self.surface.configure(&self.device, &self.surface_config);
    }
}
