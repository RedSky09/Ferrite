mod graphics;
mod texture;

use std::borrow::Cow;
use std::sync::Arc;

use wgpu::util::DeviceExt;
use wgpu::CurrentSurfaceTexture;
use winit::event_loop::OwnedDisplayHandle;
use winit::window::Window;

pub use crate::graphics::{Graphics, Sprite};
use crate::texture::Texture;

const INITIAL_SPRITE_CAPACITY: usize = 256;
const VERTICES_PER_SPRITE: usize = 4;
const INDICES_PER_SPRITE: usize = 6;

/// One vertex as it is laid out in GPU memory.
#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct Vertex {
    position: [f32; 2],
    uv: [f32; 2],
    color: [f32; 3],
}

impl Vertex {
    const ATTRIBS: [wgpu::VertexAttribute; 3] =
        wgpu::vertex_attr_array![0 => Float32x2, 1 => Float32x2, 2 => Float32x3];

    fn layout() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &Self::ATTRIBS,
        }
    }
}

/// The camera matrix as the shader sees it.
#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct CameraUniform {
    view_proj: [[f32; 4]; 4],
}

/// The four corners of a sprite: top left, bottom left, bottom right, top right.
/// uv (0, 0) is the top-left corner of the image, (1, 1) the bottom-right.
fn sprite_vertices(sprite: &Sprite) -> [Vertex; 4] {
    let [x, y] = sprite.position;
    let hx = sprite.size[0] * 0.5;
    let hy = sprite.size[1] * 0.5;
    let color = sprite.color;
    [
        Vertex {
            position: [x - hx, y + hy],
            uv: [0.0, 0.0],
            color,
        },
        Vertex {
            position: [x - hx, y - hy],
            uv: [0.0, 1.0],
            color,
        },
        Vertex {
            position: [x + hx, y - hy],
            uv: [1.0, 1.0],
            color,
        },
        Vertex {
            position: [x + hx, y + hy],
            uv: [1.0, 0.0],
            color,
        },
    ]
}

/// A vertex buffer big enough for `sprite_capacity` sprites. Filled every frame.
fn create_vertex_buffer(device: &wgpu::Device, sprite_capacity: usize) -> wgpu::Buffer {
    device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("sprite vertices"),
        size: (sprite_capacity * VERTICES_PER_SPRITE * std::mem::size_of::<Vertex>()) as u64,
        usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    })
}

/// Every sprite uses the same index pattern, shifted by 4 vertices per sprite,
/// so the index buffer can be filled once when the buffers are (re)created.
fn create_index_buffer(device: &wgpu::Device, sprite_capacity: usize) -> wgpu::Buffer {
    let mut indices: Vec<u32> = Vec::with_capacity(sprite_capacity * INDICES_PER_SPRITE);
    for i in 0..sprite_capacity as u32 {
        let base = i * VERTICES_PER_SPRITE as u32;
        indices.extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
    }
    device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("sprite indices"),
        contents: bytemuck::cast_slice(&indices),
        usage: wgpu::BufferUsages::INDEX,
    })
}

/// Builds an 8x8 checkerboard. The top-left pixel is red so a flipped
/// image is easy to spot.
fn checkerboard_8x8() -> Vec<u8> {
    let mut data = Vec::with_capacity(8 * 8 * 4);
    for y in 0..8 {
        for x in 0..8 {
            let color: [u8; 4] = if x == 0 && y == 0 {
                [255, 0, 0, 255]
            } else if (x + y) % 2 == 0 {
                [230, 230, 230, 255]
            } else {
                [60, 60, 90, 255]
            };
            data.extend_from_slice(&color);
        }
    }
    data
}

/// Owns the GPU resources and draws each frame.
pub struct Renderer {
    instance: wgpu::Instance,
    window: Arc<Window>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    surface: wgpu::Surface<'static>,
    config: wgpu::SurfaceConfiguration,
    render_pipeline: wgpu::RenderPipeline,
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    sprite_capacity: usize,
    /// Reused every frame to avoid allocating a new Vec each time.
    vertex_scratch: Vec<Vertex>,
    texture_bind_group: wgpu::BindGroup,
    camera_buffer: wgpu::Buffer,
    camera_bind_group: wgpu::BindGroup,
}

impl Renderer {
    /// Blocks until the GPU is ready (fine for native targets).
    pub fn new(window: Arc<Window>, display_handle: OwnedDisplayHandle) -> Self {
        pollster::block_on(Self::new_async(window, display_handle))
    }

    async fn new_async(window: Arc<Window>, display_handle: OwnedDisplayHandle) -> Self {
        let size = window.inner_size();
        let (width, height) = (size.width.max(1), size.height.max(1));

        let instance = wgpu::Instance::new(
            wgpu::InstanceDescriptor::new_with_display_handle_from_env(Box::new(display_handle)),
        );
        let surface = instance
            .create_surface(window.clone())
            .expect("failed to create surface");

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                compatible_surface: Some(&surface),
                ..Default::default()
            })
            .await
            .expect("no suitable GPU adapter found");

        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor::default())
            .await
            .expect("failed to create device");

        let config = surface
            .get_default_config(&adapter, width, height)
            .expect("surface is not supported by the adapter");
        surface.configure(&device, &config);

        // Camera: a uniform buffer the shader reads, updated every frame.
        let camera_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("camera uniform"),
            size: std::mem::size_of::<CameraUniform>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let camera_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("camera bind group layout"),
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
            });

        let camera_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("camera bind group"),
            layout: &camera_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: camera_buffer.as_entire_binding(),
            }],
        });

        // Texture, and the bind group that connects it to the shader.
        let pixels = checkerboard_8x8();
        let sprite_texture = Texture::from_rgba8(&device, &queue, &pixels, 8, 8, "checkerboard");

        let texture_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("texture bind group layout"),
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            multisampled: false,
                            view_dimension: wgpu::TextureViewDimension::D2,
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                ],
            });

        let texture_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("sprite bind group"),
            layout: &texture_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&sprite_texture.view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&sprite_texture.sampler),
                },
            ],
        });

        let vertex_buffer = create_vertex_buffer(&device, INITIAL_SPRITE_CAPACITY);
        let index_buffer = create_index_buffer(&device, INITIAL_SPRITE_CAPACITY);

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("sprite shader"),
            source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(include_str!("shader.wgsl"))),
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: None,
            // Index in this list = @group(N) in the shader.
            bind_group_layouts: &[
                Some(&camera_bind_group_layout),
                Some(&texture_bind_group_layout),
            ],
            immediate_size: 0,
        });

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("sprite pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[Some(Vertex::layout())],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                compilation_options: Default::default(),
                targets: &[Some(config.format.into())],
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview_mask: None,
            cache: None,
        });

        log::info!("GPU: {:?}", adapter.get_info().name);

        Self {
            instance,
            window,
            device,
            queue,
            surface,
            config,
            render_pipeline,
            vertex_buffer,
            index_buffer,
            sprite_capacity: INITIAL_SPRITE_CAPACITY,
            vertex_scratch: Vec::new(),
            texture_bind_group,
            camera_buffer,
            camera_bind_group,
        }
    }

    /// Reconfigures the surface after the window size changes.
    pub fn resize(&mut self, width: u32, height: u32) {
        // A minimized window can report a size of 0, which is invalid.
        self.config.width = width.max(1);
        self.config.height = height.max(1);
        self.surface.configure(&self.device, &self.config);
    }

    fn reconfigure(&self) {
        self.surface.configure(&self.device, &self.config);
    }

    /// Makes sure the GPU buffers can hold `needed` sprites, doubling their size if not.
    fn ensure_sprite_capacity(&mut self, needed: usize) {
        if needed <= self.sprite_capacity {
            return;
        }
        let mut capacity = self.sprite_capacity;
        while capacity < needed {
            capacity *= 2;
        }
        self.vertex_buffer = create_vertex_buffer(&self.device, capacity);
        self.index_buffer = create_index_buffer(&self.device, capacity);
        self.sprite_capacity = capacity;
        log::info!("sprite buffers grown to {capacity} sprites");
    }

    /// Draws one frame: everything queued in `gfx`, in a single draw call.
    pub fn render(&mut self, gfx: &Graphics) {
        let frame = match self.surface.get_current_texture() {
            CurrentSurfaceTexture::Success(frame) => frame,
            // Skip this frame; the next redraw request will try again.
            CurrentSurfaceTexture::Timeout | CurrentSurfaceTexture::Occluded => return,
            CurrentSurfaceTexture::Suboptimal(texture) => {
                drop(texture);
                self.reconfigure();
                return;
            }
            CurrentSurfaceTexture::Outdated => {
                self.reconfigure();
                return;
            }
            CurrentSurfaceTexture::Lost => {
                self.surface = self
                    .instance
                    .create_surface(self.window.clone())
                    .expect("failed to recreate surface");
                self.reconfigure();
                return;
            }
            CurrentSurfaceTexture::Validation => {
                panic!("wgpu validation error while acquiring the surface texture")
            }
        };

        // Upload this frame's camera matrix.
        let aspect = self.config.width as f32 / self.config.height as f32;
        let uniform = CameraUniform {
            view_proj: gfx.camera.view_projection(aspect),
        };
        self.queue
            .write_buffer(&self.camera_buffer, 0, bytemuck::bytes_of(&uniform));

        // Turn the queued sprites into vertices and upload them in one go.
        let sprites = gfx.sprites();
        self.ensure_sprite_capacity(sprites.len());
        self.vertex_scratch.clear();
        for sprite in sprites {
            self.vertex_scratch
                .extend_from_slice(&sprite_vertices(sprite));
        }
        if !self.vertex_scratch.is_empty() {
            self.queue.write_buffer(
                &self.vertex_buffer,
                0,
                bytemuck::cast_slice(&self.vertex_scratch),
            );
        }
        let index_count = (sprites.len() * INDICES_PER_SPRITE) as u32;

        let view = frame
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });

        {
            let clear = wgpu::Color {
                r: gfx.clear_color[0] as f64,
                g: gfx.clear_color[1] as f64,
                b: gfx.clear_color[2] as f64,
                a: 1.0,
            };
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: None,
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        // Clear the screen to a single color first.
                        load: wgpu::LoadOp::Clear(clear),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            if index_count > 0 {
                pass.set_pipeline(&self.render_pipeline);
                pass.set_bind_group(0, &self.camera_bind_group, &[]);
                pass.set_bind_group(1, &self.texture_bind_group, &[]);
                pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
                pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
                // All sprites in one draw call.
                pass.draw_indexed(0..index_count, 0, 0..1);
            }
        }

        self.queue.submit(Some(encoder.finish()));
        self.window.pre_present_notify();
        self.queue.present(frame);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sprite_corners_are_placed_around_the_center() {
        let sprite = Sprite::new([1.0, 2.0], [2.0, 4.0]);
        let v = sprite_vertices(&sprite);
        assert_eq!(v[0].position, [0.0, 4.0]); // top left
        assert_eq!(v[1].position, [0.0, 0.0]); // bottom left
        assert_eq!(v[2].position, [2.0, 0.0]); // bottom right
        assert_eq!(v[3].position, [2.0, 4.0]); // top right
        assert_eq!(v[0].uv, [0.0, 0.0]);
        assert_eq!(v[2].uv, [1.0, 1.0]);
    }
}
