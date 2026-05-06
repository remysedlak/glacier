use crate::project::{AudioBlock, Instrument, PatternData};
use crate::ui::draw_toolbar;
use glyphon::{Attrs, Cache, Family, FontSystem, Metrics, Resolution, Shaping, SwashCache, TextArea, TextAtlas, TextBounds, TextRenderer, Viewport};
use std::borrow::Cow;
use std::f32;
use wgpu::{
    Color, CommandEncoderDescriptor, Device, DeviceDescriptor, Features, FragmentState, Instance, Limits, LoadOp, MemoryHints, MultisampleState,
    Operations, PowerPreference, Queue, RenderPassColorAttachment, RenderPassDescriptor, RenderPipeline, RenderPipelineDescriptor,
    RequestAdapterOptions, ShaderModuleDescriptor, ShaderSource, StoreOp, Surface, SurfaceConfiguration, TextureFormat, TextureViewDescriptor,
    VertexState,
};
use winit::{dpi::PhysicalSize, event_loop::EventLoopProxy, window::Window};

pub mod mixer;
pub mod playlist;
pub mod sequencer;

use crate::colors::*;
use crate::ui::*;
pub const SEQUENCER_ID: usize = 0;
pub const PLAYLIST_ID: usize = 1;
pub const MIXER_ID: usize = 2;
#[cfg(not(target_arch = "wasm32"))]
pub type Rc<T> = std::sync::Arc<T>;

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Vertex {
    pub position: [f32; 3],
    pub color: [f32; 3],
}
impl Vertex {
    const ATTRIBS: [wgpu::VertexAttribute; 2] = wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x3];

    fn desc() -> wgpu::VertexBufferLayout<'static> {
        use std::mem;
        wgpu::VertexBufferLayout {
            array_stride: mem::size_of::<Self>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &Self::ATTRIBS,
        }
    }
}

// Click results let the app handle what graphics are clicked
pub enum ClickResult {
    Step(usize, usize, usize),
    Mute(usize),
    ChangeBpm(f32),
    TogglePlay,
    ProjectFileDialog,
    InstrumentFileDialog,
    DeleteTrack(usize),
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

fn make_text_buffer(font_system: &mut FontSystem, text: &str, size: f32, line_height: f32, color: Option<(u8, u8, u8)>) -> glyphon::Buffer {
    let (r, g, b) = color.unwrap_or((255, 255, 255));
    let mut buffer = glyphon::Buffer::new(font_system, Metrics::new(size, line_height));
    buffer.set_size(font_system, Some(400.0), Some(50.0));
    buffer.set_text(
        font_system,
        text,
        &Attrs::new().family(Family::SansSerif).color(glyphon::Color::rgb(r, g, b)),
        Shaping::Advanced,
    );
    buffer.shape_until_scroll(font_system, false);
    buffer
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
    let render_pipeline = create_pipeline(&device, surface_config.format);

    let font_system = FontSystem::new();
    let swash_cache = SwashCache::new();
    let cache = Cache::new(&device);
    let viewport = Viewport::new(&device, &cache);
    let mut atlas = TextAtlas::new(&device, &queue, &cache, surface_config.format);
    let renderer = TextRenderer::new(&mut atlas, &device, MultisampleState::default(), None);

    let vertices: Vec<Vertex> = Vec::new();
    let patterns: Vec<PatternData> = Vec::new();

    let vertex_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("Vertex Buffer"),
        size: ONE_MEGABYTE,
        usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });

    // instantiate app movable windows
    let mut mini_windows: Vec<MiniWindow> = Vec::new();

    let instruments: Vec<Instrument> = Vec::new();

    // declare sequencer
    let sequencer_window = MiniWindow::new(0, 256.0, 128.0, 1300.0, 400.0, "Sequencer", WindowKind::Sequencer);
    mini_windows.push(sequencer_window);

    // declare playlist
    let playlist_window = MiniWindow::new(PLAYLIST_ID, 64.0, 64.0, 1300.0, 800.0, "Playlist", WindowKind::Playlist);
    mini_windows.push(playlist_window);

    // declare mixer
    let mixer_window = MiniWindow::new(2, 128.0, 500.0, 800.0, 300.0, "Mixer", WindowKind::Mixer);
    mini_windows.push(mixer_window);

    let events: Vec<AudioBlock> = Vec::new();

    let gfx = Graphics {
        window: window.clone(),
        surface,
        surface_config,
        instruments,
        patterns,
        device,
        queue,
        render_pipeline,
        vertex_buffer,
        num_vertices: vertices.len() as u32,
        active_step: 0,
        font_system,
        viewport,
        atlas,
        swash_cache,
        renderer,
        bpm: 120.0,
        is_playing: false,
        master_volume: 0.5,
        dragging_knob: None,
        mini_windows,
        dragging_window: None,
        active_pattern_id: 0,
        dragging: false,
        events,
    };

    let _ = proxy.send_event(gfx);
}

/// Creates a WGSL render pipeline
fn create_pipeline(device: &Device, swap_chain_format: TextureFormat) -> RenderPipeline {
    let shader = device.create_shader_module(ShaderModuleDescriptor {
        label: None,
        source: ShaderSource::Wgsl(Cow::Borrowed(include_str!("../shader.wgsl"))),
    });

    device.create_render_pipeline(&RenderPipelineDescriptor {
        label: None,
        layout: None,
        vertex: VertexState {
            module: &shader,
            entry_point: Some("vs_main"),
            buffers: &[Vertex::desc()],
            compilation_options: Default::default(),
        },
        fragment: Some(FragmentState {
            module: &shader,
            entry_point: Some("fs_main"),
            targets: &[Some(swap_chain_format.into())],
            compilation_options: Default::default(),
        }),
        primitive: Default::default(),
        depth_stencil: None,
        multisample: Default::default(),
        multiview: None,
        cache: None,
    })
}

// Graphics state
pub struct Graphics {
    // system state
    window: Rc<Window>,
    surface: Surface<'static>,
    surface_config: SurfaceConfiguration,
    device: Device,
    queue: Queue,
    render_pipeline: RenderPipeline,
    vertex_buffer: wgpu::Buffer,
    font_system: FontSystem,
    viewport: Viewport,
    atlas: TextAtlas,
    swash_cache: SwashCache,
    renderer: TextRenderer,

    // user state
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
}

impl Graphics {
    pub fn request_redraw(&self) {
        self.window.request_redraw();
    }

    /// Performs operations to load a new track of instrument steps to the state
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

    /// handles dragging operations and returns location/result to app
    pub fn handle_drag(&mut self, x: f32, y: f32, dy: f32, dx: f32) -> DragResult {
        // find the sequencer position
        let sequencer_window = &self.mini_windows[SEQUENCER_ID];

        // mixer window
        let mixer_window = &self.mini_windows[MIXER_ID];

        // master/mixer
        if self.dragging_knob == None {
            let y_ceiling: f32 = mixer_window.y;
            let track_height: f32 = 164.0;
            let padding = 32.0;
            // replace "+ 24.0" with "i * 24.0", when multiple sliders are added to mixer
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

        // track(instrument) volume
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

        // if already dragging a window, just move it
        if let Some(i) = self.dragging_window {
            self.mini_windows[i].x += dx;
            self.mini_windows[i].y += dy;
            return DragResult::None;
        }

        // only check titlebar hit if not already dragging
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

    /// Resize the user's main window
    pub fn resize(&mut self, new_size: PhysicalSize<u32>) {
        self.surface_config.width = new_size.width.max(1);
        self.surface_config.height = new_size.height.max(1);
        self.surface.configure(&self.device, &self.surface_config);
    }

    // called every frame — returns ClickResult so app.rs can dispatch audio commands
    pub fn draw(&mut self, _mouse_x: f32, _mouse_y: f32, clicked: bool) -> ClickResult {
        self.viewport.update(
            &self.queue,
            Resolution {
                width: self.surface_config.width,
                height: self.surface_config.height,
            },
        );
        let box_padding = 8.0;
        let padding = 16.0;

        let frame = self.surface.get_current_texture().expect("Failed to acquire next swap chain texture.");

        let view = frame.texture.create_view(&TextureViewDescriptor::default());

        let mut vertices: Vec<Vertex> = Vec::new();
        let mut text_items: Vec<(glyphon::Buffer, f32, f32)> = Vec::new();
        let mut click_result = ClickResult::None;

        let screen_width = self.surface_config.width;
        let screen_height = self.surface_config.height;

        // sequencer window
        let sequencer_is_open = self.mini_windows[SEQUENCER_ID].is_open;
        if sequencer_is_open {
            dbg!(&self.mini_windows[SEQUENCER_ID].width);
            let window = &self.mini_windows[SEQUENCER_ID];
            let (verts, texts, result) = sequencer::draw(
                window,
                &mut self.patterns,
                &mut self.font_system,
                &mut self.instruments,
                self.active_pattern_id,
                self.active_step,
                clicked,
                _mouse_x,
                _mouse_y,
                screen_width,
                screen_height,
            );
            vertices.extend(verts);
            text_items.extend(texts);
            click_result = result;
        }
        // handle delete after loop to avoid borrow issues
        if let ClickResult::DeleteTrack(i) = click_result {
            self.instruments.remove(i);
        }

        // component for stacking user created patterns
        let pattern_tray = Rectangle {
            x: screen_width as f32 - 128.0,
            y: TOOLBAR_Y,
            width: 128.0,
            height: self.surface_config.height as f32 - TOOLBAR_THICKNESS,
        };
        vertices.extend(pattern_tray.draw(screen_width, screen_height, PASCAL));
        // Pattern tray label
        text_items.push((
            make_text_buffer(&mut self.font_system, "Patterns", 18.0, 20.0, Some((255, 255, 255))),
            screen_width as f32 - 128.0 + box_padding,
            TOOLBAR_Y + box_padding,
        ));

        for (i, pattern) in &mut self.patterns.iter_mut().enumerate() {
            // Pattern button
            let pattern_button = Rectangle {
                x: screen_width as f32 - 128.0 + padding,
                y: 48.0 + (32.0 * i as f32) + 24.0,
                width: 96.0,
                height: 24.0,
            };
            if i == self.active_pattern_id {
                let indicator = Rectangle {
                    x: screen_width as f32 - 128.0 + box_padding,
                    y: 48.0 + (32.0 * i as f32) + 24.0 + box_padding,
                    width: 4.0,
                    height: 4.0,
                };
                vertices.extend(indicator.draw(screen_width, screen_height, ORANGE));
            }
            // Pattern label
            text_items.push((
                make_text_buffer(&mut self.font_system, &pattern.name, 14.0, 22.0, Some((0, 0, 0))),
                screen_width as f32 - 96.0,
                48.0 + (32.0 * i as f32) + 24.0,
            ));
            // click to change current pattern on sequencer
            vertices.extend(pattern_button.draw(screen_width, screen_height, pattern_button.hover_color(_mouse_x, _mouse_y)));
            if clicked && pattern_button.is_hovered(_mouse_x, _mouse_y) {
                self.active_pattern_id = pattern.id as usize;
            }
        }

        // bpm up
        let bpm_up = Rectangle {
            x: 48.0,
            y: 4.0,
            width: 32.0,
            height: 10.0,
        };
        vertices.extend(bpm_up.draw(screen_width, screen_height, LIGHT_GRAY));
        if clicked && bpm_up.is_hovered(_mouse_x, _mouse_y) {
            self.bpm += 1.0;
            click_result = ClickResult::ChangeBpm(self.bpm);
        }
        // bpm down
        let bpm_down = Rectangle {
            x: 48.0,
            y: 16.0,
            width: 32.0,
            height: 10.0,
        };
        vertices.extend(bpm_down.draw(screen_width, screen_height, LIGHT_GRAY));
        if clicked && bpm_down.is_hovered(_mouse_x, _mouse_y) {
            self.bpm -= 1.0;
            click_result = ClickResult::ChangeBpm(self.bpm);
        }

        // play / pause
        let play_button = Rectangle {
            x: PLAY_X_ORIGIN,
            y: PLAY_Y_ORIGIN,
            width: PLAY_SQUARE_WIDTH,
            height: PLAY_SQUARE_HEIGHT,
        };
        if clicked && play_button.is_hovered(_mouse_x, _mouse_y) {
            self.is_playing = !self.is_playing;
            click_result = ClickResult::TogglePlay;
        }

        // sequencer button
        let sequencer_toggle = Rectangle {
            x: PLAY_X_ORIGIN + 256.0,
            y: PLAY_Y_ORIGIN,
            width: PLAY_SQUARE_WIDTH,
            height: PLAY_SQUARE_HEIGHT,
        };
        vertices.extend(sequencer_toggle.draw(screen_width, screen_height, sequencer_toggle.hover_color(_mouse_x, _mouse_y)));
        if clicked && sequencer_toggle.is_hovered(_mouse_x, _mouse_y) {
            click_result = ClickResult::ToggleSequencerWindow;
        }
        text_items.push((
            make_text_buffer(&mut self.font_system, "sequence", 14.0, 22.0, Some((0, 0, 0))),
            PLAY_X_ORIGIN + 256.0,
            4.0,
        ));

        // mixer button
        let mixer_toggle = Rectangle {
            x: PLAY_X_ORIGIN + 256.0 + (BUTTON_GAP * 3.0),
            y: PLAY_Y_ORIGIN,
            width: PLAY_SQUARE_WIDTH,
            height: PLAY_SQUARE_HEIGHT,
        };
        vertices.extend(mixer_toggle.draw(screen_width, screen_height, mixer_toggle.hover_color(_mouse_x, _mouse_y)));
        if clicked && mixer_toggle.is_hovered(_mouse_x, _mouse_y) {
            click_result = ClickResult::ToggleMixerWindow;
        }
        text_items.push((
            make_text_buffer(&mut self.font_system, "mixer", 14.0, 22.0, Some((0, 0, 0))),
            PLAY_X_ORIGIN + 256.0 + (BUTTON_GAP * 3.0),
            4.0,
        ));

        // playlist button
        let playlist_toggle = Rectangle {
            x: PLAY_X_ORIGIN + 256.0 + (BUTTON_GAP * 3.0) * 2.0,
            y: PLAY_Y_ORIGIN,
            width: PLAY_SQUARE_WIDTH,
            height: PLAY_SQUARE_HEIGHT,
        };
        vertices.extend(playlist_toggle.draw(screen_width, screen_height, playlist_toggle.hover_color(_mouse_x, _mouse_y)));
        if clicked && playlist_toggle.is_hovered(_mouse_x, _mouse_y) {
            click_result = ClickResult::TogglePlaylistWindow;
        }
        text_items.push((
            make_text_buffer(&mut self.font_system, "pl", 14.0, 22.0, Some((0, 0, 0))),
            PLAY_X_ORIGIN + 256.0 + (BUTTON_GAP * 3.0) * 2.0,
            4.0,
        ));

        // load project
        let load_project = Rectangle {
            x: screen_width as f32 - LOAD_PROJECT_ICON_OFFSET,
            y: TOOLBAR_MARGIN,
            width: ICON_WIDTH,
            height: ICON_HEIGHT,
        };
        if clicked && load_project.is_hovered(_mouse_x, _mouse_y) {
            click_result = ClickResult::ProjectFileDialog;
        }

        // load instrument
        let load_instrument = Rectangle {
            x: screen_width as f32 - ADD_INSTRUMENT_ICON_OFFSET,
            y: TOOLBAR_MARGIN,
            width: ICON_WIDTH,
            height: ICON_HEIGHT,
        };
        if clicked && load_instrument.is_hovered(_mouse_x, _mouse_y) {
            click_result = ClickResult::InstrumentFileDialog;
        }

        // playlist window
        let playlist_is_open = self.mini_windows[PLAYLIST_ID].is_open;
        if playlist_is_open {
            let window = &self.mini_windows[PLAYLIST_ID];
            let (verts, texts) = playlist::draw(window, &self.events, &self.patterns, &mut self.font_system, screen_width, screen_height);
            vertices.extend(verts);
            text_items.extend(texts);
        }

        // mixer window
        let mixer_is_open = self.mini_windows[MIXER_ID].is_open;

        if mixer_is_open {
            let window = &self.mini_windows[MIXER_ID];
            let (verts, texts) = mixer::draw(window, &mut self.master_volume, &mut self.font_system, screen_width, screen_height);
            vertices.extend(verts);
            text_items.extend(texts);
        }

        // project file dialog
        text_items.push((
            make_text_buffer(&mut self.font_system, "proj", 14.0, 22.0, Some((0, 0, 0))),
            screen_width as f32 - 37.0,
            4.0,
        ));
        // instrument file dialog
        text_items.push((
            make_text_buffer(&mut self.font_system, "instr", 14.0, 22.0, Some((0, 0, 0))),
            screen_width as f32 - (37.0 + 40.0 + 1.0),
            4.0,
        ));
        // bpm text
        text_items.push((
            make_text_buffer(&mut self.font_system, &self.bpm.to_string(), 18.0, 22.0, None),
            10.0,
            TOOLBAR_MARGIN,
        ));
        // play and pause label
        let label = if self.is_playing { "❚❚" } else { "  ▶" };
        text_items.push((
            make_text_buffer(&mut self.font_system, label, 18.0, 22.0, None),
            PLAY_X_ORIGIN + (PLAY_SQUARE_WIDTH / 4.0),
            5.0,
        ));

        // render all texts
        let text_areas: Vec<TextArea> = text_items
            .iter()
            .map(|buf| TextArea {
                buffer: &buf.0,
                left: buf.1,
                top: buf.2,
                scale: 1.0,
                bounds: TextBounds {
                    left: 0,
                    top: 0,
                    right: screen_width as i32,
                    bottom: self.surface_config.height as i32,
                },
                default_color: glyphon::Color::rgb(255, 255, 255),
                custom_glyphs: &[],
            })
            .collect();
        self.renderer
            .prepare(
                &self.device,
                &self.queue,
                &mut self.font_system,
                &mut self.atlas,
                &self.viewport,
                text_areas,
                &mut self.swash_cache,
            )
            .unwrap();

        // draw the toolbar at the top
        draw_toolbar(&mut vertices, screen_width, screen_height, _mouse_x, _mouse_y);

        // load all vertices from built UI
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
            r_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
            r_pass.draw(0..self.num_vertices, 0..1);
            self.renderer.render(&self.atlas, &self.viewport, &mut r_pass).unwrap();
        }

        self.queue.submit(Some(encoder.finish()));
        frame.present();

        click_result
    }
}
