pub mod color;
pub mod components;
pub mod context_menu;
pub mod drag;
pub mod draw;
pub mod font;
pub mod geometry;
pub mod icons;
pub mod mini_window;
pub mod primitives;
pub mod regions;

use crate::app::{MouseState, PianoRollState, ScrollOffset};
use crate::config::DEFAULT_BPM;
use crate::graphics::components::side_panel::DEFAULT_TRAY_WIDTH;
use crate::graphics::font::{load_fonts, Font};
use crate::graphics::mini_window::playlist::toolbar::PlaylistTool;
use crate::project::{
    AudioBlock, AudioBlockID, AudioBlockType, PatternData, PatternID, Track, TrackData, TrackID,
};
use std::path::PathBuf;

use color::{Color, DARK_GRAY, WHITE};
use components::{footer, side_panel};
use context_menu::ContextMenu;
use font::{create_bind_group_layout, Font::Mono, Font::Roboto, GlyphCache, TextItem};
use fontdue::layout::{CoordinateSystem, Layout, TextStyle};
use geometry::*;
use icons::{push_icon_draw, Tooltip};
use mini_window::{
    mixer, piano_roll, playlist, sequencer,
    sequencer::{ACTIONS_Y_OFFSET, KNOB_OFFSET, KNOB_RADIUS, TRACK_GAP},
    track, MiniWindow, WindowKind, MIXER_ID, PIANO_ROLL_ID, PLAYLIST_ID, SEQUENCER_ID,
};
use primitives::*;
use std::{borrow::Cow, collections::HashMap};

use wgpu::{
    CommandEncoderDescriptor, DeviceDescriptor, Features, FragmentState, Instance, Limits, LoadOp,
    MemoryHints, Operations, PowerPreference, RenderPassColorAttachment, RenderPassDescriptor,
    RenderPipeline, RenderPipelineDescriptor, RequestAdapterOptions, ShaderModuleDescriptor,
    ShaderSource, StoreOp, Surface, SurfaceConfiguration, TextureFormat, TextureViewDescriptor,
    VertexState,
};

use winit::{
    dpi::PhysicalSize,
    event_loop::EventLoopProxy,
    window::{CursorIcon, Window},
};

pub type Rc<T> = std::sync::Arc<T>;

/// Initialize the graphics with default/loaded state and find driver/display info
pub async fn create_graphics(window: Rc<Window>, proxy: EventLoopProxy<Graphics>) {
    // the entry point into the graphics backend (Vulkan/Metal/DX12/GL)
    let instance: Instance = Instance::default();

    // Surface = the paintable region wgpu is allowed to write pixels into;
    // everything else about the window (chrome, position, events, focus) is winit's job
    let surface: Surface = instance.create_surface(Rc::clone(&window)).unwrap();

    // handle to one specific physical GPU (that the chosen backend (Vulkan/Metal/DX12/GL) can see on the machine
    let adapter = instance
        .request_adapter(&RequestAdapterOptions {
            power_preference: PowerPreference::default(),
            force_fallback_adapter: false,
            compatible_surface: Some(&surface),
        })
        .await
        .expect("Could not get an adapter (GPU).");

    // device: handle for creating GPU-side resources
    // queue: uploades CPU-side Vec<Vertex> data into a GPU buffer, hands recorded CommandEncoder to the GPU to execute
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

    // Returns the physical size of the winit body
    let size = window.inner_size();
    let width = size.width.max(1);
    let height = size.height.max(1);
    let surface_config = surface.get_default_config(&adapter, width, height).unwrap();
    surface.configure(&device, &surface_config);

    // The bind group layout shared by every glyph/icon texture:
    // one filterable 2D texture (binding 0) plus one filtering sampler (binding 1).
    let bind_group_layout = create_bind_group_layout(&device);

    // wgsl shader and render pipeline setup
    let render_pipeline = create_pipeline(&device, surface_config.format, &bind_group_layout);

    // load TTF fonts
    let (font_cache, glyph_cache) = load_fonts(&device, &queue);

    // load SVG icons
    let mut icon_cache = HashMap::new();
    icons::load_icons(&mut icon_cache, &device, &queue);

    // vertex buffer for collecting text characters
    let glyph_vertex_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("Glyph Vertex Buffer"),
        size: ONE_MEGABYTE * 2,
        usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });

    let icon_vertex_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("Icon Vertex Buffer"),
        size: ONE_MEGABYTE * 2,
        usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });

    // vertex buffer for collecting shapes
    let vertex_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("Vertex Buffer"),
        size: ONE_MEGABYTE * 8,
        usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });

    // DEVELOPER
    // @@TODO: ALLOW CUSTOM AUDIO FILE ROOT TO ACCESS DRUMKITS
    let audio_root = PathBuf::from("./assets/free-drum-samples");

    // setup mini windows
    let mini_windows: Vec<MiniWindow> = MiniWindow::default_windows();

    let gfx = Graphics {
        // graphics
        window: window.clone(),
        surface,
        project_path: "".to_string(),
        surface_config,
        device,
        queue,
        render_pipeline,
        show_save_modal: false,
        track_tray_width: DEFAULT_TRAY_WIDTH,
        pattern_tray_width: DEFAULT_TRAY_WIDTH,
        dragging_file: None,
        playlist_tool: PlaylistTool::Select,

        active_tray: AudioBlockType::Mixing, // Pattern(id) or Track(id)
        fs_cache: {
            let mut cache = std::collections::HashMap::new();
            let root = audio_root.clone();
            if let Ok(entries) = std::fs::read_dir(&root) {
                let listing = entries
                    .flatten()
                    .map(|e| {
                        let is_dir = e.file_type().map(|t| t.is_dir()).unwrap_or(false);
                        (e.path(), is_dir)
                    })
                    .collect();
                cache.insert(root, listing);
            }
            cache
        },

        // shapes
        vertex_buffer,
        glyph_vertex_buffer,
        icon_vertex_buffer,
        frame_ms: 0.0,
        num_vertices: 0,

        // song information
        tracks: Vec::new(),
        patterns: Vec::new(),
        audio_blocks: Vec::new(),
        active_step: 0,
        active_pattern_id: PatternID(0),
        bpm: DEFAULT_BPM,
        is_playing: false,
        master_volume: 0.5,
        playhead_beat: 0.0,

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
        dragging_slider: None,
        playlist_scroll_offset: ScrollOffset::default(),
        sequencer_scroll_offset: ScrollOffset::default(),
        fs_scroll_offset: 0.0,
        z_order: vec![SEQUENCER_ID, PLAYLIST_ID, MIXER_ID, PIANO_ROLL_ID],
        context_menu: None,

        resizing_audio_block: None,
        resize_drag_accumulator: 0.0,
        resizing_track_tray: false,

        show_track_tray: true,
        show_pattern_tray: true,
        master_rms_l: 0.0,
        master_rms_r: 0.0,
        master_peak: 0.0,
        expanded_dirs: std::collections::HashSet::new(),
        user_fs_location: audio_root,
        renaming: None,
        spectrum: Vec::new(),
        sample_rate: 0.0,
    };

    let _ = proxy.send_event(gfx);
}

/// create the render pipeline, which describes how to process vertices and fragments, including shaders, blending, and output formats
fn create_pipeline(
    device: &wgpu::Device,
    swap_chain_format: TextureFormat,
    bind_group_layout: &wgpu::BindGroupLayout,
) -> RenderPipeline {
    let shader = device.create_shader_module(ShaderModuleDescriptor {
        label: None,
        source: ShaderSource::Wgsl(Cow::Borrowed(include_str!("../shader.wgsl"))),
    });

    device.create_render_pipeline(&RenderPipelineDescriptor {
        label: None,
        layout: Some(
            &device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: None,
                bind_group_layouts: &[bind_group_layout],
                push_constant_ranges: &[],
            }),
        ),
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
    pub surface_config: SurfaceConfiguration,
    device: wgpu::Device,
    queue: wgpu::Queue,
    render_pipeline: RenderPipeline,

    // buffers
    vertex_buffer: wgpu::Buffer,
    glyph_vertex_buffer: wgpu::Buffer,
    icon_vertex_buffer: wgpu::Buffer,

    pub fs_cache: std::collections::HashMap<std::path::PathBuf, Vec<(std::path::PathBuf, bool)>>,

    // text
    glyph_cache: GlyphCache,
    font_cache: HashMap<Font, fontdue::Font>,

    //ui
    pub expanded_dirs: std::collections::HashSet<PathBuf>,
    pub user_fs_location: PathBuf,
    pub track_tray_width: f32,
    pub pattern_tray_width: f32,
    pub active_tray: AudioBlockType, // Pattern(id) or Track(id)
    pub renaming: Option<RenameState>,

    pub mini_windows: Vec<MiniWindow>,
    num_vertices: u32,
    pub active_pattern_id: PatternID,
    pub piano_roll_state: Option<PianoRollState>,
    pub z_order: Vec<usize>,
    pub context_menu: Option<ContextMenu>,
    icon_cache: HashMap<String, (wgpu::Texture, wgpu::BindGroup)>,
    pub tooltip: Option<Tooltip>,
    pub frame_ms: f32,
    pub show_track_tray: bool,
    pub show_pattern_tray: bool,
    pub show_save_modal: bool,
    pub playlist_tool: PlaylistTool,

    // song
    pub project_path: String,
    pub tracks: Vec<Track>,
    pub patterns: Vec<PatternData>,
    pub audio_blocks: Vec<AudioBlock>,
    pub active_step: usize,
    pub playhead_beat: f32,
    pub bpm: f32,
    pub is_playing: bool,
    pub master_volume: f32,
    pub master_rms_l: f32,
    pub master_rms_r: f32,
    pub master_peak: f32,
    pub spectrum: Vec<f32>,
    pub sample_rate: f32,

    // dragging
    pub dragging_knob: Option<TrackID>, // volume knob
    pub dragging_window: Option<usize>, // window titlebar
    pub resizing_track_tray: bool,
    pub dragging: bool,
    pub dragging_file: Option<PathBuf>,
    pub dragging_slider: Option<Option<TrackID>>,
    pub resizing_audio_block: Option<AudioBlockID>, // pattern resizing in playlist
    pub resize_drag_accumulator: f32,

    // scrolling
    pub playlist_scroll_offset: ScrollOffset,
    pub sequencer_scroll_offset: ScrollOffset,
    pub fs_scroll_offset: f32,
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

    /// update existing pattern by matching id, if id not found create new pattern
    pub fn update_pattern(&mut self, p: PatternData) {
        if let Some(existing) = self
            .patterns
            .iter_mut()
            .find(|existing| existing.id == p.id)
        {
            *existing = p;
        } else {
            self.patterns.push(p);
        }
    }
    /// update existing track data by matching track data id
    pub fn update_track_data(&mut self, t: TrackData) {
        if let Some(existing) = self
            .tracks
            .iter_mut()
            .find(|existing| existing.data.id == t.id)
        {
            existing.data = t;
        }
    }

    /// main window resizing
    pub fn resize(&mut self, new_size: PhysicalSize<u32>) {
        self.surface_config.width = new_size.width.max(1);
        self.surface_config.height = new_size.height.max(1);
        self.surface.configure(&self.device, &self.surface_config);
    }
}
