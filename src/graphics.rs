use crate::render::{draw_rectangle, Button};
use std::borrow::Cow;
use wgpu::util::DeviceExt;
use wgpu::{
    Color, CommandEncoderDescriptor, Device, DeviceDescriptor, Features, FragmentState, Instance,
    Limits, LoadOp, MemoryHints, Operations, PowerPreference, Queue, RenderPassColorAttachment,
    RenderPassDescriptor, RenderPipeline, RenderPipelineDescriptor, RequestAdapterOptions,
    ShaderModuleDescriptor, ShaderSource, StoreOp, Surface, SurfaceConfiguration, TextureFormat,
    TextureViewDescriptor, VertexState,
};
use winit::{dpi::PhysicalSize, event_loop::EventLoopProxy, window::Window};

#[cfg(target_arch = "wasm32")]
pub type Rc<T> = std::rc::Rc<T>;

#[cfg(not(target_arch = "wasm32"))]
pub type Rc<T> = std::sync::Arc<T>;

const LIGHT_GRAY: (f32, f32, f32) = (0.53, 0.53, 0.53);
const DARK_GRAY: (f32, f32, f32) = (0.33, 0.33, 0.33);

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Vertex {
    pub position: [f32; 3],
    pub color: [f32; 3],
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
            experimental_features: Default::default(),
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

    let mut buttons: Vec<Button> = Vec::new();
    let mut vertices: Vec<Vertex> = Vec::new();
    for i in 0..16 {
        buttons.push(Button {
            x: 100 + i * 30,
            y: 100,
            width: 25,
            height: 50,
            is_active: false,
        });
        for vert in draw_rectangle(
            100 + i * 30,
            100,
            20,
            80,
            surface_config.width,
            surface_config.height,
            LIGHT_GRAY,
        ) {
            vertices.push(vert);
        }
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
        buttons,
        device,
        queue,
        render_pipeline,
        vertex_buffer,
        num_vertices,
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
#[derive(Debug)]
pub struct Graphics {
    window: Rc<Window>,
    surface: Surface<'static>,
    surface_config: SurfaceConfiguration,
    device: Device,
    queue: Queue,
    render_pipeline: RenderPipeline,
    vertex_buffer: wgpu::Buffer,
    buttons: Vec<Button>,
    num_vertices: u32,
}

impl Graphics {
    pub fn request_redraw(&self) {
        self.window.request_redraw();
    }

    pub fn handle_button_click(&mut self, x: f64, y: f64) -> Option<usize> {
        for (i, button) in self.buttons.iter_mut().enumerate() {
            if x > button.x as f64
                && x < button.x as f64 + button.width as f64
                && y > button.y as f64
                && y < button.y as f64 + button.height as f64
            {
                button.is_active = !button.is_active;
                return Some(i);
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
        let frame = self
            .surface
            .get_current_texture()
            .expect("Failed to acquire next swap chain texture.");

        let view = frame.texture.create_view(&TextureViewDescriptor::default());

        let mut vertices: Vec<Vertex> = Vec::new();
        for button in &mut self.buttons {
            let color;
            if button.is_active {
                color = DARK_GRAY;
            } else {
                color = LIGHT_GRAY;
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
                    depth_slice: None,
                    resolve_target: None,
                    ops: Operations {
                        load: LoadOp::Clear(Color::GREEN),
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
        } // `r_pass` dropped here

        self.queue.submit(Some(encoder.finish()));
        frame.present();
    }
}
