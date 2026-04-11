use crate::render::{draw_h_line, draw_rectangle, StepButton};
use glyphon::{
    Attrs, Cache, Family, FontSystem, Metrics, Resolution, Shaping, SwashCache, TextArea,
    TextAtlas, TextBounds, TextRenderer, Viewport,
};
use std::borrow::Cow;
use wgpu::util::DeviceExt;
use wgpu::{
    Buffer, Color, CommandEncoderDescriptor, Device, DeviceDescriptor, Features, FragmentState,
    Instance, Limits, LoadOp, MemoryHints, MultisampleState, Operations, PowerPreference, Queue,
    RenderPassColorAttachment, RenderPassDescriptor, RenderPipeline, RenderPipelineDescriptor,
    RequestAdapterOptions, ShaderModuleDescriptor, ShaderSource, StoreOp, Surface,
    SurfaceConfiguration, TextureFormat, TextureViewDescriptor, VertexState,
};
use winit::{dpi::PhysicalSize, event_loop::EventLoopProxy, window::Window};

#[cfg(target_arch = "wasm32")]
pub type Rc<T> = std::rc::Rc<T>;

#[cfg(not(target_arch = "wasm32"))]
pub type Rc<T> = std::sync::Arc<T>;

const LIGHT_GRAY: (f32, f32, f32) = (0.53, 0.53, 0.53);
const DARK_GRAY: (f32, f32, f32) = (0.13, 0.13, 0.13);
const BLUE: (f32, f32, f32) = (0.01, 0.01, 0.98);
const BLACK: (f32, f32, f32) = (0.00, 0.00, 0.00);
const LL_GRAY: (f32, f32, f32) = (0.27, 0.27, 0.27);

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Vertex {
    pub position: [f32; 3],
    pub color: [f32; 3],
}

#[derive(Debug)]
struct Track {
    pub steps: Vec<StepButton>,
    is_muted: bool,
    is_solo: bool,
    instrument_index: usize,
}

impl Vertex {
    const ATTRIBS: [wgpu::VertexAttribute; 2] =
        wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x3];

    fn desc() -> wgpu::VertexBufferLayout<'static> {
        use std::mem;

        wgpu::VertexBufferLayout {
            array_stride: mem::size_of::<Self>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &Self::ATTRIBS,
        }
    }
}

// runs once at the beginning of app start up
pub async fn create_graphics(window: Rc<Window>, proxy: EventLoopProxy<Graphics>) {
    let instance = Instance::default();
    let surface = instance.create_surface(Rc::clone(&window)).unwrap();
    let adapter = instance
        .request_adapter(&RequestAdapterOptions {
            power_preference: PowerPreference::default(), // Power preference for the device
            force_fallback_adapter: false, // Indicates that only a fallback ("software") adapter can be used
            compatible_surface: Some(&surface), // Guarantee that the adapter can render to this surface
        })
        .await
        .expect("Could not get an adapter (GPU).");

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
    let font_system = FontSystem::new();
    let swash_cache = SwashCache::new();
    let cache = Cache::new(&device);
    let viewport = Viewport::new(&device, &cache);
    let mut atlas = TextAtlas::new(&device, &queue, &cache, surface_config.format);
    let renderer = TextRenderer::new(&mut atlas, &device, MultisampleState::default(), None);

    let mut vertices: Vec<Vertex> = Vec::new();
    let mut rows: Vec<Track> = Vec::new();
    dbg!(&surface_config);
    for i in 0..3 {
        let mut buttons: Vec<StepButton> = Vec::new();
        for j in 0..16 {
            let group = j / 4;
            buttons.push(StepButton {
                x: 128 + j * 28 + group * 8,
                y: 64 + i * 72,
                width: 24,
                height: 64,
                is_active: false,
            });
            for vert in draw_rectangle(
                128 + j * 28 + group * 8,
                64 + i * 72,
                24,
                72,
                surface_config.width,
                surface_config.height,
                LIGHT_GRAY,
            ) {
                vertices.push(vert);
            }
        }
        rows.push(Track {
            steps: buttons,
            is_muted: false,
            is_solo: false,
            instrument_index: i as usize,
        });
    }

    for vert in draw_h_line(0.90, 0.003, surface_config.height) {
        vertices.push(vert);
    }

    let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("Vertex Buffer"),
        contents: bytemuck::cast_slice(&vertices),
        usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
    });

    let num_vertices = vertices.len() as u32;

    let gfx = Graphics {
        window: window.clone(),
        surface,
        surface_config,
        rows,
        device,
        queue,
        render_pipeline,
        vertex_buffer,
        num_vertices,
        active_step: 0,
        font_system,
        viewport,
        atlas,
        swash_cache,
        renderer,
    };

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
}

impl Graphics {
    pub fn request_redraw(&self) {
        self.window.request_redraw();
    }

    pub fn handle_button_click(&mut self, x: f64, y: f64) -> Option<(usize, usize)> {
        for (i, track) in &mut self.rows.iter_mut().enumerate() {
            for (j, button) in &mut track.steps.iter_mut().enumerate() {
                if x > button.x as f64
                    && x < button.x as f64 + button.width as f64
                    && y > button.y as f64
                    && y < button.y as f64 + button.height as f64
                {
                    button.is_active = !button.is_active;
                    return Some((i, j));
                }
            }
        }

        None
    }

    // react to resize events from user like minimize
    pub fn resize(&mut self, new_size: PhysicalSize<u32>) {
        self.surface_config.width = new_size.width.max(1);
        self.surface_config.height = new_size.height.max(1);
        self.surface.configure(&self.device, &self.surface_config);
    }

    // called every frame to update the canvas
    pub fn draw(&mut self, _mouse_x: f64, _mouse_y: f64) {
        // create a buffer describing your text
        let mut buffer = glyphon::Buffer::new(&mut self.font_system, Metrics::new(18.0, 22.0));
        buffer.set_size(&mut self.font_system, Some(400.0), Some(50.0));
        buffer.set_text(
            &mut self.font_system,
            "BPM: 120",
            &Attrs::new().family(Family::SansSerif),
            Shaping::Advanced,
        );
        buffer.shape_until_scroll(&mut self.font_system, false);

        // update viewport to current screen size
        self.viewport.update(
            &self.queue,
            Resolution {
                width: 800,
                height: 600,
            },
        );

        // prepare uploads glyphs to GPU atlas
        self.renderer
            .prepare(
                &self.device,
                &self.queue,
                &mut self.font_system,
                &mut self.atlas,
                &self.viewport,
                [TextArea {
                    buffer: &buffer,
                    left: 10.0, // pixel x
                    top: 10.0,  // pixel y
                    scale: 1.0,
                    bounds: TextBounds {
                        left: 0,
                        top: 0,
                        right: 400,
                        bottom: 50,
                    },
                    default_color: glyphon::Color::rgb(0, 0, 0),
                    custom_glyphs: &[],
                }],
                &mut self.swash_cache,
            )
            .unwrap();
        let frame = self
            .surface
            .get_current_texture()
            .expect("Failed to acquire next swap chain texture.");

        let view = frame.texture.create_view(&TextureViewDescriptor::default());

        let mut vertices: Vec<Vertex> = Vec::new();
        for (j, track) in &mut self.rows.iter_mut().enumerate() {
            for (i, button) in &mut track.steps.iter_mut().enumerate() {
                let color;
                if i == self.active_step {
                    color = BLUE;
                } else {
                    if button.is_active {
                        color = BLACK;
                    } else {
                        if _mouse_x > button.x as f64
                            && _mouse_x < button.x as f64 + button.width as f64
                            && _mouse_y > button.y as f64
                            && _mouse_y < button.y as f64 + button.height as f64
                        {
                            color = LL_GRAY
                        } else {
                            color = LIGHT_GRAY;
                        }
                    }
                }

                for vert in draw_rectangle(
                    button.x,
                    button.y,
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
        for vert in draw_h_line(32.0, 0.003, self.surface_config.height) {
            vertices.push(vert);
        }

        self.queue
            .write_buffer(&self.vertex_buffer, 0, bytemuck::cast_slice(&vertices));
        self.num_vertices = vertices.len() as u32;

        let mut encoder = self
            .device
            .create_command_encoder(&CommandEncoderDescriptor { label: None });

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
            self.renderer
                .render(&self.atlas, &self.viewport, &mut r_pass)
                .unwrap();
        } // `r_pass` dropped here

        self.queue.submit(Some(encoder.finish()));
        frame.present();
    }
}
