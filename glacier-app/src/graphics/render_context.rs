use std::{borrow::Cow, sync::Arc};

use wgpu::{
    Adapter, DeviceDescriptor, Features, FragmentState, Instance, Limits, MemoryHints,
    PowerPreference, RenderPipeline, RenderPipelineDescriptor, RequestAdapterOptions,
    ShaderModuleDescriptor, ShaderSource, Surface, TextureFormat, VertexState,
};
use winit::{dpi::PhysicalSize, window::Window};

use crate::graphics::{
    font::create_bind_group_layout,
    primitives::{Vertex, ONE_MEGABYTE},
};

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub(crate) struct ScreenUniform {
    pub(crate) resolution: [f32; 2],
}

/// wgpu data for rendering
pub struct RenderContext {
    pub surface: wgpu::Surface<'static>,
    pub surface_config: wgpu::SurfaceConfiguration,
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub render_pipeline: wgpu::RenderPipeline,
    pub screen_uniform_buffer: wgpu::Buffer,
    pub screen_bind_group: wgpu::BindGroup,
    // buffers
    pub vertex_buffer: wgpu::Buffer,
    pub glyph_vertex_buffer: wgpu::Buffer,
    pub icon_vertex_buffer: wgpu::Buffer,
}
impl RenderContext {
    pub async fn init(window: &Arc<Window>) -> Self {
        // the entry point into the graphics backend (Vulkan/Metal/DX12/GL)
        let instance: Instance = Instance::default();

        // Surface = the paintable region wgpu is allowed to write pixels into;
        // everything else about the window (chrome, position, events, focus) is winit's job
        let surface: Surface = instance.create_surface(Arc::clone(window)).unwrap();

        // handle to one specific physical GPU (that the chosen backend (Vulkan/Metal/DX12/GL) can see on the machine
        let adapter: Adapter = instance
            .request_adapter(&RequestAdapterOptions {
                power_preference: PowerPreference::HighPerformance,
                force_fallback_adapter: false,
                compatible_surface: Some(&surface),
            })
            .await
            .expect("Could not get an adapter (GPU).");

        // device: handle for creating GPU-side resources (buffers, textures, pipelines)
        // queue: queue.write_buffer() uploads CPU-side data (e.g. Vec<Vertex>) into a GPU buffer
        //        queue.submit() hands a finished CommandEncoder's commands to the GPU for execution
        let (device, queue) = adapter
            .request_device(&DeviceDescriptor {
                label: Some("Daw device"),
                required_features: Features::empty(),
                required_limits: Limits::default().using_resolution(adapter.limits()),
                memory_hints: MemoryHints::Performance,
                trace: Default::default(),
            })
            .await
            .expect("Failed to get device");

        // Returns the physical size of the winit body
        let size = window.inner_size();
        let width = size.width.max(1);
        let height = size.height.max(1);

        let mut surface_config = surface.get_default_config(&adapter, width, height).unwrap();

        surface_config.format = wgpu::TextureFormat::Bgra8Unorm; // drop the Srgb suffix
        surface.configure(&device, &surface_config);

        // The bind group layout shared by the glyph and icon atlases:
        // one filterable 2D texture (binding 0) plus one filtering sampler (binding 1).
        let bind_group_layout = create_bind_group_layout(&device);

        use wgpu::util::DeviceExt;

        let screen_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("screen_bind_group_layout"),
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
            });

        let screen_uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("screen_uniform_buffer"),
            contents: bytemuck::cast_slice(&[ScreenUniform {
                resolution: [width as f32, height as f32],
            }]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let screen_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("screen_bind_group"),
            layout: &screen_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: screen_uniform_buffer.as_entire_binding(),
            }],
        });

        // A RenderPipeline object represents a graphics pipeline and its stages, bindings, vertex buffers and targets.
        //
        // used with
        // `r_pass.set_pipeline(&self.render_pipeline);`
        let render_pipeline: RenderPipeline = create_pipeline(
            &device,
            surface_config.format,
            &bind_group_layout,
            &screen_bind_group_layout,
        );

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

        RenderContext {
            surface,
            surface_config,
            device,
            queue,
            render_pipeline,
            screen_bind_group,
            screen_uniform_buffer,
            vertex_buffer,
            glyph_vertex_buffer,
            icon_vertex_buffer,
        }
    }
    /// respond to winit WindowEvent::Resized(size) by updating surface dimension information
    pub fn resize(&mut self, new_size: PhysicalSize<u32>) {
        self.surface_config.width = new_size.width.max(1);
        self.surface_config.height = new_size.height.max(1);
        self.surface.configure(&self.device, &self.surface_config);
        self.queue.write_buffer(
            &self.screen_uniform_buffer,
            0,
            bytemuck::cast_slice(&[ScreenUniform {
                resolution: [
                    self.surface_config.width as f32,
                    self.surface_config.height as f32,
                ],
            }]),
        );
    }
}

/// create the render pipeline, which describes how to process vertices and fragments, including shaders, blending, and output formats
fn create_pipeline(
    device: &wgpu::Device,
    swap_chain_format: TextureFormat,
    bind_group_layout: &wgpu::BindGroupLayout,
    screen_bind_group_layout: &wgpu::BindGroupLayout,
) -> wgpu::RenderPipeline {
    // load WGSL shader file
    let shader = device.create_shader_module(ShaderModuleDescriptor {
        label: Some("Main shader module"),
        source: ShaderSource::Wgsl(Cow::Borrowed(include_str!("../shader.wgsl"))),
    });

    device.create_render_pipeline(&RenderPipelineDescriptor {
        label: Some("Main render pipeline"),
        layout: Some(
            &device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Main pipeline layout"),
                bind_group_layouts: &[bind_group_layout, screen_bind_group_layout],
                push_constant_ranges: &[],
            }),
        ),
        // vertex shader
        vertex: VertexState {
            module: &shader,
            entry_point: Some("vs_main"),
            buffers: &[Vertex::desc()],
            compilation_options: Default::default(),
        },
        // fragment shader
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
        depth_stencil: None, // not needed; renderer is 2D and immediate-mode
        multisample: Default::default(),
        multiview: None,
        cache: None,
    })
}
