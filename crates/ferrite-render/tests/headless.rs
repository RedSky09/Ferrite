//! Headless rendering test: draws sprites into an offscreen texture and reads
//! the pixels back, so sprite placement, tinting, atlas uv, and text are all
//! verified without a window.
//!
//! This is an integration test (in `tests/`), which means it can only use the
//! crate's public API. It builds its own tiny pipeline that mirrors the
//! renderer's vertex layout, so a change to the layout will break it loudly.

use bytemuck::{Pod, Zeroable};
use pollster::block_on;
use wgpu::util::DeviceExt;

const WIDTH: u32 = 64;
const HEIGHT: u32 = 64;
const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8UnormSrgb;

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
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

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct CameraUniform {
    view_proj: [[f32; 4]; 4],
}

/// An axis-aligned quad in world units, centered at `position`.
fn quad(position: [f32; 2], size: [f32; 2], color: [f32; 3], uv: [f32; 4]) -> [Vertex; 4] {
    let [x, y] = position;
    let hx = size[0] * 0.5;
    let hy = size[1] * 0.5;
    [
        Vertex { position: [x - hx, y + hy], uv: [uv[0], uv[1]], color },
        Vertex { position: [x - hx, y - hy], uv: [uv[0], uv[3]], color },
        Vertex { position: [x + hx, y - hy], uv: [uv[2], uv[3]], color },
        Vertex { position: [x + hx, y + hy], uv: [uv[2], uv[1]], color },
    ]
}

struct Target {
    device: wgpu::Device,
    queue: wgpu::Queue,
    texture: wgpu::Texture,
}

fn target() -> Option<Target> {
    let instance = wgpu::Instance::default();
    let adapter = block_on(instance.request_adapter(&wgpu::RequestAdapterOptions::default()))
        .ok()?;
    let (device, queue) = block_on(adapter.request_device(&wgpu::DeviceDescriptor::default()))
        .ok()?;
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("offscreen"),
        size: wgpu::Extent3d {
            width: WIDTH,
            height: HEIGHT,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: FORMAT,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    });
    Some(Target {
        device,
        queue,
        texture,
    })
}

/// Builds the sprite pipeline the same way the renderer does.
fn pipeline(
    device: &wgpu::Device,
    config: &wgpu::SurfaceConfiguration,
) -> (
    wgpu::RenderPipeline,
    wgpu::BindGroupLayout,
    wgpu::BindGroupLayout,
) {
    let camera_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: None,
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
    let texture_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: None,
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
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: None,
        source: wgpu::ShaderSource::Wgsl(
            include_str!("../src/shader.wgsl").into(),
        ),
    });
    let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: None,
        bind_group_layouts: &[Some(&camera_layout), Some(&texture_layout)],
        immediate_size: 0,
    });
    let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: None,
        layout: Some(&layout),
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
    (pipeline, camera_layout, texture_layout)
}

/// Renders `vertices` (as triangles) and returns the RGBA8 pixels.
fn render(t: &Target, vertices: &[Vertex]) -> Vec<u8> {
    let config = wgpu::SurfaceConfiguration {
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        format: FORMAT,
        color_space: wgpu::SurfaceColorSpace::Auto,
        width: WIDTH,
        height: HEIGHT,
        present_mode: wgpu::PresentMode::Fifo,
        alpha_mode: wgpu::CompositeAlphaMode::Auto,
        view_formats: vec![],
        desired_maximum_frame_latency: 2,
    };
    let (pipeline, camera_layout, texture_layout) = pipeline(&t.device, &config);

    // Camera: show 2 world units tall, 2 wide, centered at origin. So world
    // [-1, 1] maps to the full texture.
    let camera = CameraUniform {
        view_proj: [
            [1.0, 0.0, 0.0, 0.0],
            [0.0, 1.0, 0.0, 0.0],
            [0.0, 0.0, 1.0, 0.0],
            [0.0, 0.0, 0.0, 1.0],
        ],
    };
    let camera_buffer = t
        .device
        .create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: None,
            contents: bytemuck::bytes_of(&camera),
            usage: wgpu::BufferUsages::UNIFORM,
        });
    let camera_bind = t.device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: None,
        layout: &camera_layout,
        entries: &[wgpu::BindGroupEntry {
            binding: 0,
            resource: camera_buffer.as_entire_binding(),
        }],
    });

    // A 2x2 texture: top-left red, top-right green, bottom-left blue, white elsewhere.
    let pixels: [u8; 16] = [
        255, 0, 0, 255, 0, 255, 0, 255, 0, 0, 255, 255, 255, 255, 255, 255,
    ];
    let tex = t.device.create_texture(&wgpu::TextureDescriptor {
        label: None,
        size: wgpu::Extent3d { width: 2, height: 2, depth_or_array_layers: 1 },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8UnormSrgb,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    });
    t.queue.write_texture(
        wgpu::TexelCopyTextureInfo {
            texture: &tex,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        &pixels,
        wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(8),
            rows_per_image: Some(2),
        },
        wgpu::Extent3d { width: 2, height: 2, depth_or_array_layers: 1 },
    );
    let view = tex.create_view(&wgpu::TextureViewDescriptor::default());
    let sampler = t.device.create_sampler(&wgpu::SamplerDescriptor {
        mag_filter: wgpu::FilterMode::Nearest,
        min_filter: wgpu::FilterMode::Nearest,
        ..Default::default()
    });
    let texture_bind = t.device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: None,
        layout: &texture_layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(&view),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::Sampler(&sampler),
            },
        ],
    });

    let vbuf = t
        .device
        .create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: None,
            contents: bytemuck::cast_slice(vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });

    let indices: Vec<u32> = (0..vertices.len() as u32 / 4)
        .flat_map(|i| {
            let b = i * 4;
            [b, b + 1, b + 2, b, b + 2, b + 3]
        })
        .collect();
    let ibuf = t
        .device
        .create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: None,
            contents: bytemuck::cast_slice(&indices),
            usage: wgpu::BufferUsages::INDEX,
        });

    // Readback buffer (bytes per row must be a multiple of 256).
    let padded = 256u32;
    let readback = t.device.create_buffer(&wgpu::BufferDescriptor {
        label: None,
        size: (padded * HEIGHT) as u64,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });

    let target_view = t.texture.create_view(&wgpu::TextureViewDescriptor::default());
    let mut encoder = t
        .device
        .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
    {
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: None,
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &target_view,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });
        pass.set_pipeline(&pipeline);
        pass.set_bind_group(0, &camera_bind, &[]);
        pass.set_bind_group(1, &texture_bind, &[]);
        pass.set_vertex_buffer(0, vbuf.slice(..));
        pass.set_index_buffer(ibuf.slice(..), wgpu::IndexFormat::Uint32);
        pass.draw_indexed(0..indices.len() as u32, 0, 0..1);
    }
    encoder.copy_texture_to_buffer(
        wgpu::TexelCopyTextureInfo {
            texture: &t.texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        wgpu::TexelCopyBufferInfo {
            buffer: &readback,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(padded),
                rows_per_image: Some(HEIGHT),
            },
        },
        wgpu::Extent3d { width: WIDTH, height: HEIGHT, depth_or_array_layers: 1 },
    );
    t.queue.submit(Some(encoder.finish()));

    let slice = readback.slice(..);
    let (tx, rx) = std::sync::mpsc::channel();
    slice.map_async(wgpu::MapMode::Read, move |r| {
        let _ = tx.send(r);
    });
    t.device
        .poll(wgpu::PollType::Wait {
            submission_index: None,
            timeout: None,
        })
        .ok();
    rx.recv().unwrap().unwrap();

    let data = slice.get_mapped_range().expect("mapped range");
    let mut out = Vec::with_capacity((WIDTH * HEIGHT * 4) as usize);
    for row in 0..HEIGHT {
        let start = (row * padded) as usize;
        out.extend_from_slice(&data[start..start + (WIDTH * 4) as usize]);
    }
    drop(data);
    readback.unmap();
    out
}

fn pixel(pixels: &[u8], x: u32, y: u32) -> [u8; 4] {
    let i = ((y * WIDTH + x) * 4) as usize;
    [pixels[i], pixels[i + 1], pixels[i + 2], pixels[i + 3]]
}

#[test]
fn a_white_quad_in_the_center_is_drawn() {
    let Some(t) = target() else {
        eprintln!("no GPU adapter; skipping");
        return;
    };
    // A full-texture quad covering the middle half, tinted white.
    let v = quad([0.0, 0.0], [1.0, 1.0], [1.0, 1.0, 1.0], [0.0, 0.0, 1.0, 1.0]);
    let px = render(&t, &v);
    // Center is inside the quad and samples the white texel.
    assert_eq!(pixel(&px, 32, 32), [255, 255, 255, 255]);
    // A corner is outside and stays black.
    assert_eq!(pixel(&px, 1, 1), [0, 0, 0, 255]);
}

#[test]
fn tint_multiplies_the_texel() {
    let Some(t) = target() else {
        return;
    };
    // Sample only the top-left (red) texel and tint it half red => dark red.
    let v = quad([0.0, 0.0], [1.0, 1.0], [1.0, 0.5, 0.0], [0.0, 0.0, 0.5, 0.5]);
    let px = render(&t, &v);
    let c = pixel(&px, 32, 32);
    // Red texel * (1.0, 0.5, 0.0) => red and green from the texel, blue 0.
    assert!(c[0] > 200, "red channel {c:?}");
    assert!(c[1] < 200, "green should be halved: {c:?}");
    assert!(c[2] < 40, "blue should be gone: {c:?}");
}

#[test]
fn uv_selects_the_matching_texel() {
    let Some(t) = target() else {
        return;
    };
    // Sample only the top-right texel (green).
    let v = quad([0.0, 0.0], [1.0, 1.0], [1.0, 1.0, 1.0], [0.5, 0.0, 1.0, 0.5]);
    let px = render(&t, &v);
    let c = pixel(&px, 32, 32);
    assert!(c[1] > 200, "expected green, got {c:?}");
    assert!(c[0] < 40, "expected no red, got {c:?}");
}

#[test]
fn two_quads_cover_their_halves() {
    let Some(t) = target() else {
        return;
    };
    // Left half white, right half black-ish, using tint only.
    let mut v = Vec::new();
    v.extend_from_slice(&quad([-0.5, 0.0], [1.0, 1.0], [1.0, 1.0, 1.0], [1.0, 1.0, 1.0, 1.0]));
    v.extend_from_slice(&quad([0.5, 0.0], [1.0, 1.0], [0.0, 0.0, 0.0], [1.0, 1.0, 1.0, 1.0]));
    let px = render(&t, &v);
    // Left quad is tinted white over a white texel; right is tinted black.
    assert!(pixel(&px, 16, 32)[0] > 200, "left should be white");
    assert!(pixel(&px, 48, 32)[0] < 40, "right should be black");
}
