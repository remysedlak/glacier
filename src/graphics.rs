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

// Click results let the app handle what graphics are clicked
pub enum ClickResult {
    Step(usize, usize),
    Mute(usize),
    ChangeBpm(f32),
    TogglePlay,
    ProjectFileDialog,
    InstrumentFileDialog,
    DeleteTrack(usize),
    ToggleSequencer,
    None,
}
pub enum DragResult {
    DragVolumeSlider(f32),
    DragVolumeKnob(usize, f32),
    None,
}

#[cfg(not(target_arch = "wasm32"))]
pub type Rc<T> = std::sync::Arc<T>;

#[derive(Debug)]
struct Track {
    pub steps: Vec<StepButton>,
    name: String,
    is_muted: bool,
    is_solo: bool,
    show_velocity: bool,
    track_volume: f32,
}

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
    let rows: Vec<Track> = Vec::new();

    let vertex_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("Vertex Buffer"),
        size: ONE_MEGABYTE,
        usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });

    // instantiate app movable windows
    let mut mini_windows: Vec<MiniWindow> = Vec::new();

    // declare sequencer
    let sequencer_window = MiniWindow::new(128.0, 128.0, 900.0, 400.0, "Sequencer", WindowKind::Sequencer);
    mini_windows.push(sequencer_window);

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
        mini_windows,
        dragging_window: None,
    };

    let _ = proxy.send_event(gfx);
}

/// Creates a WGSL render pipeline
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

// Graphics state
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
    pub mini_windows: Vec<MiniWindow>,
    pub dragging_window: Option<usize>,
}

impl Graphics {
    pub fn request_redraw(&self) {
        self.window.request_redraw();
    }

    /// Performs operations to load a new track of instrument steps to the state
    pub fn load_track(&mut self, i: usize, name: String, steps: Vec<f32>, mute: bool, vol: f32) {
        if i >= self.rows.len() {
            let mut buttons = Vec::new();
            for j in 0..steps.len() {
                buttons.push(StepButton {
                    width: BUTTON_WIDTH,
                    height: BUTTON_HEIGHT,
                    velocity: steps[j],
                });
            }
            self.rows.push(Track {
                name,
                steps: buttons,
                is_muted: mute,
                is_solo: false,
                show_velocity: false,
                track_volume: vol,
            });
        }
        for (j, &step) in steps.iter().enumerate() {
            self.rows[i].steps[j].velocity = step;
        }
    }

    /// handles dragging operations and returns location/result to app
    pub fn handle_drag(&mut self, x: f32, y: f32, dy: f32, dx: f32) -> DragResult {
        // find the sequencer position
        let (seq_x, seq_y) = self
            .mini_windows
            .iter()
            .find(|w| matches!(w.window_kind, WindowKind::Sequencer))
            .map(|w| (w.x, w.y))
            .unwrap_or((64.0, 64.0));

        if self.dragging_knob == None {
            let y_ceiling: f32 = 416.0;
            let track_height: f32 = 164.0;
            let padding = 32.0;
            if x > 64.0 - padding && x < 96.0 + padding && y > y_ceiling && y < y_ceiling + track_height {
                self.master_volume = 1.0 - ((y - y_ceiling) / track_height).clamp(0.0, 1.0);
                return DragResult::DragVolumeSlider(self.master_volume);
            }
        }

        if let Some(i) = self.dragging_knob {
            self.rows[i].track_volume = (self.rows[i].track_volume - dy * 0.005).clamp(0.0, 1.0);
            return DragResult::DragVolumeKnob(i, self.rows[i].track_volume);
        }

        for (i, track) in &mut self.rows.iter_mut().enumerate() {
            let knob_rect = Rectangle {
                x: seq_x + 198.0 - KNOB_RADIUS,
                y: seq_y + (i as f32 * TRACK_GAP) + 24.0 - KNOB_RADIUS,
                width: KNOB_RADIUS * 2.0,
                height: KNOB_RADIUS * 2.0,
            };
            if knob_rect.is_hovered(x, y) {
                self.dragging_knob = Some(i);
                track.track_volume = (track.track_volume - dy * 0.01).clamp(0.0, 1.0);
                return DragResult::DragVolumeKnob(i, track.track_volume);
            }
        }

        // if already dragging a window, just move it
        if let Some(i) = self.dragging_window {
            self.mini_windows[i].x += dx;
            self.mini_windows[i].y += dy;
            return DragResult::None;
        }

        // only check titlebar hit if not already dragging
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

        let padding = 16.0;

        let frame = self
            .surface
            .get_current_texture()
            .expect("Failed to acquire next swap chain texture.");

        let view = frame.texture.create_view(&TextureViewDescriptor::default());

        let mut vertices: Vec<Vertex> = Vec::new();
        let mut text_items: Vec<(glyphon::Buffer, f32, f32)> = Vec::new();
        let mut click_result = ClickResult::None;
        let sw = self.surface_config.width;
        let sh = self.surface_config.height;

        // unload sequencer window information from setup
        let (seq_x, seq_y, seq_w, _, seq_t) = self
            .mini_windows
            .iter()
            .find(|w| matches!(w.window_kind, WindowKind::Sequencer))
            .map(|w| (w.x, w.y, w.width, w.height, w.title.clone()))
            .unwrap_or((64.0, 64.0, 1000.0, 600.0, "Title".to_string()));

        let seq_is_open = self
            .mini_windows
            .iter()
            .find(|w| matches!(w.window_kind, WindowKind::Sequencer))
            .map(|w| w.is_open)
            .unwrap_or(false);

        if seq_is_open {
            // background of sequencer
            let seq_h = TITLEBAR_HEIGHT + padding + self.rows.len() as f32 * TRACK_GAP;
            let seq_background = Rectangle {
                x: seq_x,
                y: seq_y,
                width: seq_w,
                height: seq_h,
            };
            vertices.extend(seq_background.draw(sw, sh, BACKGROUND));

            // titlebar rectangle
            let titlebar = Rectangle {
                x: seq_x,
                y: seq_y - TITLEBAR_HEIGHT,
                width: seq_w,
                height: TITLEBAR_HEIGHT,
            };
            vertices.extend(titlebar.draw(sw, sh, DARK_GRAY));

            // titlebar text
            text_items.push((
                make_text_buffer(&mut self.font_system, &seq_t, 14.0, 22.0, None),
                seq_x + 8.0,
                seq_y - TITLEBAR_HEIGHT + 4.0,
            ));

            /* begin per track rendering */
            for (j, track) in &mut self.rows.iter_mut().enumerate() {
                for (i, button) in &mut track.steps.iter_mut().enumerate() {
                    let group = i / 4;
                    let x = 240.0 + padding + seq_x + i as f32 * BUTTON_GAP + group as f32 * BAR_GAP;
                    let y = padding + seq_y + j as f32 * TRACK_GAP;

                    if track.show_velocity {
                        // background
                        let background = Rectangle {
                            x,
                            y,
                            width: button.width,
                            height: button.height,
                        };
                        vertices.extend(background.draw(sw, sh, DARK_GRAY));

                        // velocity bar
                        let filled_height = button.height * (button.velocity / 128.0);
                        let bar = Rectangle {
                            x,
                            y: y + button.height - filled_height,
                            width: button.width,
                            height: filled_height,
                        };
                        vertices.extend(bar.draw(sw, sh, BLUE));
                    } else {
                        let step = Rectangle {
                            x,
                            y,
                            width: button.width,
                            height: button.height,
                        };
                        vertices.extend(step.draw(
                            sw,
                            sh,
                            step.active_step_color(_mouse_x, _mouse_y, i == self.active_step, button.velocity > 0.0),
                        ));

                        if clicked && step.is_hovered(_mouse_x, _mouse_y) {
                            button.velocity = if button.velocity > 0.0 { 0.0 } else { 95.0 };
                            click_result = ClickResult::Step(j, i);
                        }
                    }
                }

                let button_gap = 40.0;

                // mute button
                let mute_button = Rectangle {
                    x: padding + seq_x,
                    y: 32.0 + seq_y + (j as f32 * TRACK_GAP),
                    width: MUTE_SQUARE_LENGTH,
                    height: MUTE_SQUARE_LENGTH,
                };
                vertices.extend(mute_button.draw(sw, sh, mute_button.active_color(_mouse_x, _mouse_y, track.is_muted)));
                if clicked && mute_button.is_hovered(_mouse_x, _mouse_y) {
                    track.is_muted = !track.is_muted;
                    click_result = ClickResult::Mute(j);
                }

                // velocity button
                let velocity_button = Rectangle {
                    x: padding + seq_x + button_gap,
                    y: 32.0 + seq_y + (j as f32 * TRACK_GAP),
                    width: MUTE_SQUARE_LENGTH,
                    height: MUTE_SQUARE_LENGTH,
                };
                vertices.extend(velocity_button.draw(sw, sh, velocity_button.active_color(_mouse_x, _mouse_y, track.show_velocity)));
                if clicked && velocity_button.is_hovered(_mouse_x, _mouse_y) {
                    track.show_velocity = !track.show_velocity;
                }

                // delete button
                let delete_button = Rectangle {
                    x: padding + seq_x + button_gap + 16.0,
                    y: 32.0 + seq_y + (j as f32 * TRACK_GAP),
                    width: MUTE_SQUARE_LENGTH,
                    height: MUTE_SQUARE_LENGTH,
                };
                vertices.extend(delete_button.draw(sw, sh, delete_button.hover_color(_mouse_x, _mouse_y)));
                if clicked && delete_button.is_hovered(_mouse_x, _mouse_y) {
                    click_result = ClickResult::DeleteTrack(j);
                }

                // track volume knob
                for vert in draw_knob(
                    track.track_volume,
                    seq_x + 198.0,
                    seq_y + (j as f32 * TRACK_GAP) + 24.0,
                    KNOB_RADIUS,
                    35,
                    sw,
                    sh,
                ) {
                    vertices.push(vert);
                }
                // text buffers
                text_items.push((
                    make_text_buffer(&mut self.font_system, &track.name, 18.0, 22.0, None),
                    seq_x + 16.0,
                    seq_y + j as f32 * TRACK_GAP,
                ));
                text_items.push((
                    make_text_buffer(&mut self.font_system, "mut", 12.0, 22.0, None),
                    seq_x + 16.0,
                    seq_y + j as f32 * TRACK_GAP + 40.0,
                ));
                text_items.push((
                    make_text_buffer(&mut self.font_system, "vel", 12.0, 22.0, None),
                    seq_x - 32.0 - 16.0,
                    seq_y as f32 * TRACK_GAP + 54.0,
                ));
            }
        }

        // handle delete after loop to avoid borrow issues
        if let ClickResult::DeleteTrack(i) = click_result {
            self.rows.remove(i);
        }

        // bpm up
        let bpm_up = Rectangle {
            x: 48.0,
            y: 4.0,
            width: 32.0,
            height: 10.0,
        };
        vertices.extend(bpm_up.draw(sw, sh, LIGHT_GRAY));
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
        vertices.extend(bpm_down.draw(sw, sh, LIGHT_GRAY));
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
        vertices.extend(sequencer_toggle.draw(sw, sh, sequencer_toggle.hover_color(_mouse_x, _mouse_y)));
        if clicked && sequencer_toggle.is_hovered(_mouse_x, _mouse_y) {
            click_result = ClickResult::ToggleSequencer;
        }

        let user_width = self.surface_config.width as f32;

        // load project
        let load_project = Rectangle {
            x: user_width - LOAD_PROJECT_ICON_OFFSET,
            y: TOOLBAR_MARGIN,
            width: ICON_WIDTH,
            height: ICON_HEIGHT,
        };
        if clicked && load_project.is_hovered(_mouse_x, _mouse_y) {
            click_result = ClickResult::ProjectFileDialog;
        }

        // load instrument
        let load_instrument = Rectangle {
            x: user_width - ADD_INSTRUMENT_ICON_OFFSET,
            y: TOOLBAR_MARGIN,
            width: ICON_WIDTH,
            height: ICON_HEIGHT,
        };
        if clicked && load_instrument.is_hovered(_mouse_x, _mouse_y) {
            click_result = ClickResult::InstrumentFileDialog;
        }

        // master volume slider
        draw_slider(sw, sh, &mut vertices, &mut self.master_volume);

        // text buffers
        text_items.push((
            make_text_buffer(&mut self.font_system, "proj", 14.0, 22.0, Some((0, 0, 0))),
            self.surface_config.width as f32 - 37.0,
            4.0,
        ));
        text_items.push((
            make_text_buffer(&mut self.font_system, "instr", 14.0, 22.0, Some((0, 0, 0))),
            self.surface_config.width as f32 - (37.0 + 40.0 + 1.0),
            4.0,
        ));
        text_items.push((
            make_text_buffer(&mut self.font_system, &self.bpm.to_string(), 18.0, 22.0, None),
            10.0,
            TOOLBAR_MARGIN,
        ));
        text_items.push((
            make_text_buffer(&mut self.font_system, &self.master_volume.to_string(), 18.0, 22.0, None),
            54.0,
            380.0,
        ));

        let label = if self.is_playing { "❚❚" } else { "  ▶" };
        text_items.push((
            make_text_buffer(&mut self.font_system, label, 18.0, 22.0, None),
            PLAY_X_ORIGIN + (PLAY_SQUARE_WIDTH / 4.0),
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

        draw_toolbar(&mut vertices, sw, sh, _mouse_x, _mouse_y);

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
