use crate::ui::{draw_slider, draw_toolbar};
use glyphon::{
    Attrs, Cache, Family, FontSystem, Metrics, Resolution, Shaping, SwashCache, TextArea, TextAtlas, TextBounds, TextRenderer, Viewport,
};
use std::borrow::Cow;
use std::f32;
use wgpu::{
    Color, CommandEncoderDescriptor, Device, DeviceDescriptor, Features, FragmentState, Instance, Limits, LoadOp, MemoryHints,
    MultisampleState, Operations, PowerPreference, Queue, RenderPassColorAttachment, RenderPassDescriptor, RenderPipeline,
    RenderPipelineDescriptor, RequestAdapterOptions, ShaderModuleDescriptor, ShaderSource, StoreOp, Surface, SurfaceConfiguration,
    TextureFormat, TextureViewDescriptor, VertexState,
};
use winit::{dpi::PhysicalSize, event_loop::EventLoopProxy, window::Window};

use crate::colors::*;
use crate::ui::*;

pub enum ClickResult {
    Step(usize, usize), // track, step
    Mute(usize),        // track
    ChangeBpm(f32),
    TogglePlay,
    ProjectFileDialog,
    InstrumentFileDialog,
    DeleteTrack(usize),
    None,
}

pub enum DragResult {
    DragVolumeSlider(f32),
    DragVolumeKnob(usize, f32),
    None,
}

#[cfg(target_arch = "wasm32")]
pub type Rc<T> = std::rc::Rc<T>;

#[cfg(not(target_arch = "wasm32"))]
pub type Rc<T> = std::sync::Arc<T>;

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Vertex {
    pub position: [f32; 3],
    pub color: [f32; 3],
}

#[derive(Debug)]
struct Track {
    pub steps: Vec<StepButton>,
    name: String,
    is_muted: bool,
    is_solo: bool,
    instrument_index: usize,
    show_velocity: bool,
    track_volume: f32,
}

// im not messing with this;; WGSL setup
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

fn make_buffer(font_system: &mut FontSystem, text: &str, size: f32, line_height: f32, color: Option<(u8, u8, u8)>) -> glyphon::Buffer {
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

// runs once at the beginning of app start up
pub async fn create_graphics(window: Rc<Window>, proxy: EventLoopProxy<Graphics>) {
    // Context for all other wgpu objects. Instance of wgpu. first item u create on wgpu
    let instance = Instance::default();
    //  bridge between wgpu and the actual window pixels, the canvas
    let surface = instance.create_surface(Rc::clone(&window)).unwrap();
    // the thing wgpu queries to find out "what hardware (GPUs) is actually here and what can it do."
    let adapter = instance
        .request_adapter(&RequestAdapterOptions {
            power_preference: PowerPreference::default(), // Power preference for the device
            force_fallback_adapter: false,                // Indicates that only a fallback ("software") adapter can be used
            compatible_surface: Some(&surface),           // Guarantee that the adapter can render to this surface
        })
        .await
        .expect("Could not get an adapter (GPU).");

    // device is our GPU interface, and the queue is how we send commands to it
    let (device, queue) = adapter
        .request_device(&DeviceDescriptor {
            label: None,
            required_features: Features::empty(), // Specifies the required features by the device request. Fails if the adapter can't provide them.
            required_limits: Limits::downlevel_webgl2_defaults().using_resolution(adapter.limits()),
            memory_hints: MemoryHints::Performance,
            trace: Default::default(),
        })
        .await
        .expect("Failed to get device");

    // Get physical pixel dimensions inside the window
    let size = window.inner_size();
    // Make the dimensions at least size 1, otherwise wgpu would panic
    let width = size.width.max(1);
    let height = size.height.max(1);
    let surface_config = surface.get_default_config(&adapter, width, height).unwrap();

    // Initializes Surface for presentation.
    surface.configure(&device, &surface_config);
    let render_pipeline = create_pipeline(&device, surface_config.format);

    // glyphon setup
    let font_system = FontSystem::new();
    let swash_cache = SwashCache::new();
    let cache = Cache::new(&device);
    let viewport = Viewport::new(&device, &cache);
    let mut atlas = TextAtlas::new(&device, &queue, &cache, surface_config.format);
    let renderer = TextRenderer::new(&mut atlas, &device, MultisampleState::default(), None);

    // Vectors to store all triangles for display
    let vertices: Vec<Vertex> = Vec::new();
    let rows: Vec<Track> = Vec::new();

    // the vertex buffer is how we send data to the gpu
    let vertex_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("Vertex Buffer"),
        size: ONE_MEGABYTE, // 1MB, plenty of room
        usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });

    // graphics class state to mutate for the rest of the sessions
    let gfx = Graphics {
        window: window.clone(),
        surface,
        surface_config,
        rows,
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
    };

    // returns the graphics state back to wherever it was requested
    let _ = proxy.send_event(gfx);
}

// compiles the shader code and describes the full rendering pipeline to the GPU  (runs once per session)
fn create_pipeline(device: &Device, swap_chain_format: TextureFormat) -> RenderPipeline {
    let shader = device.create_shader_module(ShaderModuleDescriptor {
        label: None,
        source: ShaderSource::Wgsl(Cow::Borrowed(include_str!("shader.wgsl"))),
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

// graphics state
pub struct Graphics {
    window: Rc<Window>,
    surface: Surface<'static>,
    surface_config: SurfaceConfiguration,
    device: Device,
    queue: Queue,
    render_pipeline: RenderPipeline,
    vertex_buffer: wgpu::Buffer,
    rows: Vec<Track>,
    num_vertices: u32,
    pub active_step: usize,
    font_system: FontSystem,
    viewport: Viewport,
    atlas: TextAtlas,
    swash_cache: SwashCache,
    renderer: TextRenderer,
    pub bpm: f32,
    pub is_playing: bool,
    pub master_volume: f32,
    pub dragging_knob: Option<usize>,
}

impl Graphics {
    pub fn request_redraw(&self) {
        self.window.request_redraw();
    }

    // handler for : UiCommand::LoadTrack
    pub fn load_track(&mut self, i: usize, name: String, steps: Vec<f32>, mute: bool, vol: f32) {
        if i >= self.rows.len() {
            // add a new row
            let mut buttons = Vec::new();
            for j in 0..steps.len() {
                buttons.push(StepButton {
                    width: BUTTON_WIDTH,
                    height: BUTTON_HEIGHT,
                    velocity: steps[j as usize],
                });
            }
            self.rows.push(Track {
                name,
                steps: buttons,
                is_muted: mute,
                is_solo: false,
                instrument_index: i,
                show_velocity: false,
                track_volume: vol,
            });
        }
        // set step states
        for (j, &step) in steps.iter().enumerate() {
            self.rows[i].steps[j].velocity = step;
        }
    }

    pub fn handle_drag(&mut self, x: f64, y: f64, dy: f64) -> DragResult {
        // master volume tracking
        if self.dragging_knob == None {
            let y_ceiling: f64 = 416.0;
            let track_height: f64 = 164.0;
            let padding = 32.0;
            if x > 64.0 - padding && x < 96.0 + padding && y > y_ceiling && y < y_ceiling + track_height {
                self.master_volume = 1.0 - ((y as f32 - y_ceiling as f32) / track_height as f32).clamp(0.0, 1.0);
                dbg!(self.master_volume);
                return DragResult::DragVolumeSlider(self.master_volume);
            }
        }

        // knob tracking
        if let Some(i) = self.dragging_knob {
            self.rows[i].track_volume = (self.rows[i].track_volume - dy as f32 * 0.005).clamp(0.0, 1.0);
            return DragResult::DragVolumeKnob(i, self.rows[i].track_volume);
        }
        for (i, track) in &mut self.rows.iter_mut().enumerate() {
            if x > (BUTTON_X_ORIGIN - 24 - KNOB_RADIUS as u32) as f64
                && x < (BUTTON_X_ORIGIN - 24 as u32) as f64 + KNOB_RADIUS as f64
                && y > (BUTTON_Y_ORIGIN as f64 + (i as f64 * TRACK_GAP as f64) + 24.0) as f64 - KNOB_RADIUS as f64
                && y < (BUTTON_Y_ORIGIN as f64 + (i as f64 * TRACK_GAP as f64) + 24.0) as f64 + KNOB_RADIUS as f64
            {
                self.dragging_knob = Some(i);
                track.track_volume = (track.track_volume - dy as f32 * 0.01).clamp(0.0, 1.0);
                return DragResult::DragVolumeKnob(i, track.track_volume);
            }
        }

        return DragResult::None;
    }

    // handler for UiCommand::StepAdvanced, UiCmomand::MuteTrack
    pub fn handle_button_click(&mut self, x: f64, y: f64) -> ClickResult {
        for (i, track) in &mut self.rows.iter_mut().enumerate() {
            if track.show_velocity {
            } else {
                for (j, button) in &mut track.steps.iter_mut().enumerate() {
                    let group = j / 4;
                    let x2 = BUTTON_X_ORIGIN + i as u32 * BUTTON_GAP as u32 + group as u32 * BAR_GAP as u32;
                    let y2 = BUTTON_Y_ORIGIN + j as u32 * TRACK_GAP;
                    if x > x2 as f64 && x < x2 as f64 + button.width as f64 && y > y2 as f64 && y < y2 as f64 + button.height as f64 {
                        button.velocity = if button.velocity > 0.0 { 0.0 } else { 95.0 };
                        dbg!(button.velocity);
                        return ClickResult::Step(i, j);
                    }
                }
            }

            // check for mute
            if x > (BUTTON_X_ORIGIN - 24) as f64
                && x < (BUTTON_X_ORIGIN - 24 + MUTE_SQUARE_LENGTH) as f64
                && y > (BUTTON_Y_ORIGIN + (i as u32 * TRACK_GAP) + 48) as f64
                && y < ((BUTTON_Y_ORIGIN + (i as u32 * TRACK_GAP) + 48) + MUTE_SQUARE_LENGTH) as f64
            {
                track.is_muted = !track.is_muted;
                return ClickResult::Mute(i);
            }

            // check for velocity
            if x > (BUTTON_X_ORIGIN - 40) as f64
                && x < (BUTTON_X_ORIGIN - 40 + MUTE_SQUARE_LENGTH) as f64
                && y > (BUTTON_Y_ORIGIN + (i as u32 * TRACK_GAP) + 48) as f64
                && y < ((BUTTON_Y_ORIGIN + (i as u32 * TRACK_GAP) + 48) + MUTE_SQUARE_LENGTH) as f64
            {
                track.show_velocity = !track.show_velocity;
                return ClickResult::None;
            }

            // check for delete
            if x > (BUTTON_X_ORIGIN - 40 - 16) as f64
                && x < (BUTTON_X_ORIGIN - 40 - 16 + MUTE_SQUARE_LENGTH) as f64
                && y > (BUTTON_Y_ORIGIN + (i as u32 * TRACK_GAP) + 48) as f64
                && y < ((BUTTON_Y_ORIGIN + (i as u32 * TRACK_GAP) + 48) + MUTE_SQUARE_LENGTH) as f64
            {
                self.rows.remove(i);
                return ClickResult::DeleteTrack(i);
            }
        }

        // check for bpm
        if x > 48 as f64 && x < 48 as f64 + ICON_WIDTH as f64 && y > 4 as f64 && y < 4 as f64 + 10 as f64 {
            self.bpm = self.bpm + 1.0;
            return ClickResult::ChangeBpm(self.bpm);
        }
        if x > 48 as f64 && x < 48 as f64 + ICON_WIDTH as f64 && y > (16) as f64 && y < (16 + 10) as f64 {
            self.bpm = self.bpm - 1.0;
            return ClickResult::ChangeBpm(self.bpm);
        }

        // play / pause
        if x > PLAY_X_ORIGIN as f64
            && x < (PLAY_X_ORIGIN + PLAY_SQUARE_WIDTH) as f64
            && y > PLAY_Y_ORIGIN as f64
            && y < (PLAY_Y_ORIGIN + PLAY_SQUARE_HEIGHT) as f64
        {
            self.is_playing = !self.is_playing;
            return ClickResult::TogglePlay;
        }

        let user_width = self.surface_config.width;

        // load project
        if x > (user_width - LOAD_PROJECT_ICON_OFFSET) as f64
            && x < (user_width - LOAD_PROJECT_ICON_OFFSET + ICON_WIDTH) as f64
            && y > TOOLBAR_MARGIN as f64
            && y < (TOOLBAR_MARGIN + ICON_HEIGHT) as f64
        {
            return ClickResult::ProjectFileDialog;
        }

        // load instrument
        if x > (user_width - ADD_INSTRUMENT_ICON_OFFSET) as f64
            && x < (user_width - ADD_INSTRUMENT_ICON_OFFSET + ICON_WIDTH) as f64
            && y > TOOLBAR_MARGIN as f64
            && y < (TOOLBAR_MARGIN + ICON_HEIGHT) as f64
        {
            return ClickResult::InstrumentFileDialog;
        }

        ClickResult::None
    }

    // react to resize events from user like minimize
    pub fn resize(&mut self, new_size: PhysicalSize<u32>) {
        self.surface_config.width = new_size.width.max(1);
        self.surface_config.height = new_size.height.max(1);
        self.surface.configure(&self.device, &self.surface_config);
    }

    // called every frame to update the canvas
    pub fn draw(&mut self, _mouse_x: f64, _mouse_y: f64) {
        self.viewport.update(
            &self.queue,
            Resolution {
                width: self.surface_config.width,
                height: self.surface_config.height,
            },
        );

        let frame = self
            .surface
            .get_current_texture()
            .expect("Failed to acquire next swap chain texture.");

        let view = frame.texture.create_view(&TextureViewDescriptor::default());

        // draw the steps
        let mut vertices: Vec<Vertex> = Vec::new();
        let mut text_items: Vec<(glyphon::Buffer, f32, f32)> = Vec::new();

        /* dark background */

        // for vert in draw_rectangle(
        //     4,
        //     TOOLBAR_MARGIN + 48,
        //     1200,
        //     248,
        //     self.surface_config.width,
        //     self.surface_config.height,
        //     DARK_GRAY,
        // ) {
        //     vertices.push(vert);
        // }

        /* begin per track rendering */
        for (j, track) in &mut self.rows.iter_mut().enumerate() {
            for (i, button) in &mut track.steps.iter_mut().enumerate() {
                let group = j / 4;
                let x = BUTTON_X_ORIGIN + i as u32 * BUTTON_GAP as u32 + group as u32 * BAR_GAP as u32;
                let y = BUTTON_Y_ORIGIN + j as u32 * TRACK_GAP;
                if track.show_velocity {
                    // background
                    for vert in draw_rectangle(
                        x,
                        y,
                        button.width,
                        button.height,
                        self.surface_config.width,
                        self.surface_config.height,
                        DARK_GRAY,
                    ) {
                        vertices.push(vert);
                    }

                    // drag

                    let filled_height = (button.height as f32 * (button.velocity / 128.0)) as u32;
                    let bar_y = y + button.height - filled_height;
                    for vert in draw_rectangle(
                        x, // stays the same
                        bar_y,
                        button.width, // stays the same
                        filled_height,
                        self.surface_config.width,  // stays the same
                        self.surface_config.height, // stays the same
                        BLUE,
                    ) {
                        vertices.push(vert);
                    }
                } else {
                    let color;
                    if i == self.active_step {
                        if button.velocity > 0.0 {
                            color = DARK_BLUE;
                        } else {
                            color = BLUE;
                        }
                    } else {
                        if button.velocity > 0.0 {
                            if _mouse_x > x as f64
                                && _mouse_x < x as f64 + button.width as f64
                                && _mouse_y > y as f64
                                && _mouse_y < y as f64 + button.height as f64
                            {
                                color = DARK_GRAY
                            } else {
                                color = BLACK;
                            }
                        } else {
                            if _mouse_x > x as f64
                                && _mouse_x < x as f64 + button.width as f64
                                && _mouse_y > y as f64
                                && _mouse_y < y as f64 + button.height as f64
                            {
                                color = LL_GRAY
                            } else {
                                color = LIGHT_GRAY;
                            }
                        }
                    }
                    for vert in draw_rectangle(
                        x,
                        y,
                        button.width,
                        button.height,
                        self.surface_config.width,
                        self.surface_config.height,
                        color,
                    ) {
                        vertices.push(vert);
                    }
                }
            }

            let button_color = |is_active: bool, hovering: bool| {
                if hovering {
                    if !is_active {
                        LL_GRAY
                    } else {
                        DARK_GRAY
                    }
                } else if is_active {
                    BLACK
                } else {
                    LIGHT_GRAY
                }
            };

            let hover = _mouse_x > (BUTTON_X_ORIGIN - 24) as f64
                && _mouse_x < (BUTTON_X_ORIGIN - 24 + MUTE_SQUARE_LENGTH) as f64
                && _mouse_y > (BUTTON_Y_ORIGIN + (j as u32 * TRACK_GAP) + 48) as f64
                && _mouse_y < ((BUTTON_Y_ORIGIN + (j as u32 * TRACK_GAP) + 48) + MUTE_SQUARE_LENGTH) as f64;

            // mute button
            for vert in draw_rectangle(
                BUTTON_X_ORIGIN - 24,
                BUTTON_Y_ORIGIN + (j as u32 * TRACK_GAP) + 48,
                MUTE_SQUARE_LENGTH,
                MUTE_SQUARE_LENGTH,
                self.surface_config.width,
                self.surface_config.height,
                button_color(track.is_muted, hover),
            ) {
                vertices.push(vert);
            }

            let button_gap = 40;

            let hover = _mouse_x > (BUTTON_X_ORIGIN - button_gap) as f64
                && _mouse_x < (BUTTON_X_ORIGIN + MUTE_SQUARE_LENGTH - button_gap) as f64
                && _mouse_y > (BUTTON_Y_ORIGIN + (j as u32 * TRACK_GAP) + 48) as f64
                && _mouse_y < ((BUTTON_Y_ORIGIN + (j as u32 * TRACK_GAP) + 48) + MUTE_SQUARE_LENGTH) as f64;

            // velocity button
            for vert in draw_rectangle(
                BUTTON_X_ORIGIN - button_gap,
                BUTTON_Y_ORIGIN + (j as u32 * TRACK_GAP) + 48,
                MUTE_SQUARE_LENGTH,
                MUTE_SQUARE_LENGTH,
                self.surface_config.width,
                self.surface_config.height,
                button_color(track.show_velocity, hover),
            ) {
                vertices.push(vert);
            }

            // delete button
            for vert in draw_rectangle(
                BUTTON_X_ORIGIN - button_gap - 16,
                BUTTON_Y_ORIGIN + (j as u32 * TRACK_GAP) + 48,
                MUTE_SQUARE_LENGTH,
                MUTE_SQUARE_LENGTH,
                self.surface_config.width,
                self.surface_config.height,
                RED,
            ) {
                vertices.push(vert);
            }

            // track volume knob
            for vert in draw_knob(
                track.track_volume,
                (BUTTON_X_ORIGIN - 24) as f32,
                (BUTTON_Y_ORIGIN as f32 + (j as f32 * TRACK_GAP as f32) + 24.0) as f32,
                KNOB_RADIUS,
                35,
                self.surface_config.width,
                self.surface_config.height,
            ) {
                vertices.push(vert);
            }

            // track text buffer
            text_items.push((
                make_buffer(&mut self.font_system, &track.name, 18.0, 22.0, None),
                10.0,
                BUTTON_Y_ORIGIN as f32 + j as f32 * TRACK_GAP as f32,
            ));

            // mute text buffer
            text_items.push((
                make_buffer(&mut self.font_system, "mut", 12.0, 22.0, None),
                (BUTTON_X_ORIGIN - 32 + 4) as f32,
                BUTTON_Y_ORIGIN as f32 + j as f32 * TRACK_GAP as f32 + 54.0,
            ));

            // velocity mode text buffer
            text_items.push((
                make_buffer(&mut self.font_system, "vel", 12.0, 22.0, None),
                (BUTTON_X_ORIGIN - 32 - 16) as f32,
                BUTTON_Y_ORIGIN as f32 + j as f32 * TRACK_GAP as f32 + 54.0,
            ));
        }

        // master volume slider
        draw_slider(
            self.surface_config.width,
            self.surface_config.height,
            &mut vertices,
            &mut self.master_volume,
        );

        // project text buffer
        text_items.push((
            make_buffer(&mut self.font_system, "proj", 14.0, 22.0, Some((0, 0, 0))),
            self.surface_config.width as f32 - 37.0,
            4.0,
        ));

        // instrument text buffer
        text_items.push((
            make_buffer(&mut self.font_system, "instr", 14.0, 22.0, Some((0, 0, 0))),
            self.surface_config.width as f32 - (37.0 + 40.0 + 1.0),
            4.0,
        ));

        // bpm text buffer
        text_items.push((
            make_buffer(&mut self.font_system, &self.bpm.to_string(), 18.0, 22.0, None),
            10.0,
            TOOLBAR_MARGIN as f32,
        ));

        // volume text buffer
        text_items.push((
            make_buffer(&mut self.font_system, &self.master_volume.to_string(), 18.0, 22.0, None),
            54.0,
            380.0,
        ));

        let label = if self.is_playing { "❚❚" } else { "  ▶" };

        // play/pause text buffer
        text_items.push((
            make_buffer(&mut self.font_system, label, 18.0, 22.0, None),
            (PLAY_X_ORIGIN as f32 + (PLAY_SQUARE_WIDTH as f32 / 4.0)),
            5.0,
        ));

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
                    right: self.surface_config.width as i32,
                    bottom: self.surface_config.height as i32,
                },
                default_color: glyphon::Color::rgb(255, 255, 255), // white
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

        draw_toolbar(
            &mut vertices,
            self.surface_config.width,
            self.surface_config.height,
            _mouse_x,
            _mouse_y,
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
            r_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
            r_pass.draw(0..self.num_vertices, 0..1);
            self.renderer.render(&self.atlas, &self.viewport, &mut r_pass).unwrap();
        } // `r_pass` dropped here

        self.queue.submit(Some(encoder.finish()));
        frame.present();
    }
}
