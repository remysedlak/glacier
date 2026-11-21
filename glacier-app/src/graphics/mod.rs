/// custom graphics/UI backend connects to winit using WGPU
pub mod color;
pub mod components;
pub mod drag;
pub mod draw;
pub mod font;
pub mod frame_draw_context;
pub mod geometry;
pub mod icons;
pub mod mini_window;
pub mod primitives;
pub mod regions;
pub mod render_context;

use crate::app::{MouseState, PianoRollState, ScrollOffset};
use crate::config::DEFAULT_BPM;
use crate::graphics::mini_window::playlist::PlaylistAction;
use crate::project::{
    AudioBlock, AudioBlockID, AudioBlockType, PatternData, PatternID, Track, TrackData, TrackID,
};
use color::Palette;
use components::{context_menu::ContextMenu, footer, side_panel, side_panel::DEFAULT_TRAY_WIDTH};
use font::{load_fonts, Font::Mono, Font::Roboto, FontCache, GlyphCache, TextItem};
pub use frame_draw_context::FrameDrawContext;
use geometry::*;
use icons::{IconCache, Tooltip};
use mini_window::playlist::block::WaveformCache;
use mini_window::{
    mixer, piano_roll, playlist,
    playlist::toolbar::PlaylistTool,
    sequencer,
    sequencer::{ACTIONS_Y_OFFSET, KNOB_OFFSET, KNOB_RADIUS, TRACK_GAP},
    track, MiniWindow, WindowKind, MIXER_ID, PIANO_ROLL_ID, PLAYLIST_ID, SEQUENCER_ID,
};
use primitives::*;
use render_context::RenderContext;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;
use wgpu::{
    CommandEncoderDescriptor, LoadOp, Operations, RenderPassColorAttachment, RenderPassDescriptor,
    StoreOp, TextureViewDescriptor,
};
use winit::{
    event_loop::EventLoopProxy,
    window::{CursorIcon, Window},
};
pub(crate) struct DrawItem {
    pub(crate) vertex_offset: u64,
}

/// Initialize the graphics with default/loaded state and find driver/display info
pub async fn create_graphics(window: Arc<Window>, proxy: EventLoopProxy<Graphics>) {
    let render_context = RenderContext::init(&window).await;
    // cache TTF fonts and cache their character glyphs in multiple sizes
    let (font_cache, glyph_cache) = load_fonts(&render_context.device, &render_context.queue);

    // load SVG icons
    let icon_cache = icons::load_icons(&render_context.device, &render_context.queue);

    let waveform_cache = WaveformCache {
        entries: HashMap::new(),
    };

    // TODO: load/store palette from user config
    let palette = Palette::load_from_toml("./assets/themes/theme_test.toml")
        .expect("Error pulling color pallete from disk.");

    // DEVELOPER
    // @@TODO: ALLOW CUSTOM AUDIO FILE ROOT TO ACCESS DRUMKITS
    let audio_root = PathBuf::from("./assets/sounds");

    // setup mini windows
    let mini_windows: Vec<MiniWindow> = MiniWindow::default_windows();

    let gfx = Graphics {
        // graphics
        render_context,
        window: window.clone(),
        project_path: "".to_string(),
        show_save_modal: false,
        track_tray_width: DEFAULT_TRAY_WIDTH,
        pattern_tray_width: DEFAULT_TRAY_WIDTH,
        dragging_file: None,
        playlist_tool: PlaylistTool::Select,
        playlist_action: PlaylistAction::None, // replaces active_tray + active_audio_block

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
        present_fps: 0.0,
        fps_window_start: None,
        fps_window_frames: 0,
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

        // color palette
        palette,

        // iconography
        icon_cache,
        tooltip: None,
        piano_roll_state: None,
        waveform_cache,

        // ui state
        stretch_mode: false,
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

    // Send an event to the EventLoop from which this proxy was created.
    // This emits a UserEvent(event) event in the event loop,
    // where event is the value passed to this function.
    let _ = proxy.send_event(gfx);
}

/// Main struct holding all graphics state, including wgpu objects, loaded fonts and icons, and UI state like open windows and dragging
pub struct Graphics {
    // gui
    pub render_context: RenderContext,
    pub window: std::sync::Arc<Window>,
    num_vertices: u32,
    pub present_fps: f32,
    fps_window_start: Option<Instant>,
    fps_window_frames: u32,
    pub z_order: Vec<usize>, // painters algorithm

    // ui
    pub stretch_mode: bool,
    pub track_tray_width: f32,
    pub pattern_tray_width: f32,
    icon_cache: IconCache,
    glyph_cache: GlyphCache,
    font_cache: FontCache,
    palette: Palette,
    pub playlist_tool: PlaylistTool,
    pub playlist_action: PlaylistAction, // replaces active_tray + active_audio_block
    pub mini_windows: Vec<MiniWindow>,
    pub active_pattern_id: PatternID,
    pub piano_roll_state: Option<PianoRollState>,

    // filesystem
    pub fs_cache: std::collections::HashMap<std::path::PathBuf, Vec<(std::path::PathBuf, bool)>>,
    pub expanded_dirs: std::collections::HashSet<PathBuf>,
    pub user_fs_location: PathBuf,

    // conditional rendering
    pub show_track_tray: bool,
    pub show_pattern_tray: bool,
    pub show_save_modal: bool,
    pub tooltip: Option<Tooltip>,
    pub context_menu: Option<ContextMenu>,
    pub renaming: Option<RenameState>,

    // song
    pub waveform_cache: WaveformCache,
    pub project_path: String,
    pub tracks: Vec<Track>,
    pub patterns: Vec<PatternData>,
    pub audio_blocks: Vec<AudioBlock>,
    pub active_step: usize,
    pub playhead_beat: f32,
    pub sample_rate: f32,
    pub is_playing: bool,
    pub master_volume: f32,
    pub master_rms_l: f32,
    pub master_rms_r: f32,
    pub master_peak: f32,
    pub bpm: f32,
    pub spectrum: Vec<f32>,

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
    /// Requests winit to redraw its graphics
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
}
