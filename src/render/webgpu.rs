use crate::font::constants::UNITS_PER_EM;
use crate::font::FontUnit;
use crate::render::RenderSettings;
use crate::scene::{Color, DebugBoxKind, MathScene, SceneNode};
use bytemuck::{Pod, Zeroable};
use fontdue::{Font, FontSettings};
use futures_channel::oneshot;
use std::borrow::Cow;
use std::char;
use std::error;
use std::fmt;
use wgpu::util::DeviceExt;

const MAX_ATLAS_DIMENSION: u32 = 2048;
const ATLAS_PADDING: u32 = 1;

#[derive(Clone, Debug, PartialEq)]
pub struct WebGpuImage {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum WebGpuError {
    AdapterUnavailable(String),
    DeviceUnavailable(String),
    InvalidGlyph(u32),
    Font(String),
    AtlasTooLarge,
    BufferMap(String),
    DevicePoll(String),
}

impl fmt::Display for WebGpuError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            WebGpuError::AdapterUnavailable(ref message) => {
                write!(f, "WebGPU adapter unavailable: {}", message)
            }
            WebGpuError::DeviceUnavailable(ref message) => {
                write!(f, "WebGPU device unavailable: {}", message)
            }
            WebGpuError::InvalidGlyph(codepoint) => {
                write!(f, "invalid Unicode code point: {}", codepoint)
            }
            WebGpuError::Font(ref message) => write!(f, "font rasterization failed: {}", message),
            WebGpuError::AtlasTooLarge => {
                write!(f, "glyph atlas exceeds the supported renderer size")
            }
            WebGpuError::BufferMap(ref message) => {
                write!(f, "WebGPU readback buffer mapping failed: {}", message)
            }
            WebGpuError::DevicePoll(ref message) => {
                write!(f, "WebGPU device polling failed: {}", message)
            }
        }
    }
}

impl error::Error for WebGpuError {}

#[derive(Copy, Clone)]
pub struct WebGpuRenderer<'a> {
    settings: &'a RenderSettings,
}

impl<'a> WebGpuRenderer<'a> {
    pub fn new(settings: &'a RenderSettings) -> WebGpuRenderer<'a> {
        WebGpuRenderer { settings }
    }

    pub async fn render_scene(&self, scene: &MathScene) -> Result<WebGpuImage, WebGpuError> {
        let prepared = PreparedScene::new(scene, self.settings)?;

        let instance =
            wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                force_fallback_adapter: false,
                compatible_surface: None,
            })
            .await
            .map_err(|error| WebGpuError::AdapterUnavailable(format!("{:?}", error)))?;

        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: Some("ReX WebGPU device"),
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::downlevel_webgl2_defaults()
                    .using_resolution(adapter.limits()),
                experimental_features: wgpu::ExperimentalFeatures::disabled(),
                memory_hints: wgpu::MemoryHints::MemoryUsage,
                trace: wgpu::Trace::Off,
            })
            .await
            .map_err(|error| WebGpuError::DeviceUnavailable(format!("{:?}", error)))?;

        render_prepared(&device, &queue, prepared).await
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
struct Vertex {
    position: [f32; 2],
    uv: [f32; 2],
    color: [f32; 4],
    textured: f32,
}

const VERTEX_ATTRIBUTES: [wgpu::VertexAttribute; 4] = wgpu::vertex_attr_array![
    0 => Float32x2,
    1 => Float32x2,
    2 => Float32x4,
    3 => Float32
];

impl Vertex {
    fn layout<'a>() -> wgpu::VertexBufferLayout<'a> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &VERTEX_ATTRIBUTES,
        }
    }
}

#[derive(Debug)]
struct RasterGlyph {
    left: f32,
    top: f32,
    width: u32,
    height: u32,
    bitmap: Vec<u8>,
    atlas_x: u32,
    atlas_y: u32,
    color: [f32; 4],
}

#[derive(Debug)]
enum Primitive {
    Glyph(RasterGlyph),
    Solid {
        left: f32,
        top: f32,
        right: f32,
        bottom: f32,
        color: [f32; 4],
    },
}

struct PreparedScene {
    width: u32,
    height: u32,
    atlas_width: u32,
    atlas_height: u32,
    atlas: Vec<u8>,
    vertices: Vec<Vertex>,
    indices: Vec<u32>,
}

impl PreparedScene {
    fn new(scene: &MathScene, settings: &RenderSettings) -> Result<Self, WebGpuError> {
        let font = Font::from_bytes(
            include_bytes!("../../rex-xits.otf") as &[u8],
            FontSettings::default(),
        )
        .map_err(|error| WebGpuError::Font(format!("{:?}", error)))?;

        let pixels_per_font_unit =
            settings.font_size as f32 / f64::from(UNITS_PER_EM) as f32;
        let width = (f64::from(scene.width) as f32 * pixels_per_font_unit)
            .ceil()
            .max(1.0) as u32;
        let height = (f64::from(scene.height) as f32 * pixels_per_font_unit)
            .ceil()
            .max(1.0) as u32;

        let mut primitives = Vec::new();
        collect_primitives(
            &scene.nodes,
            black(),
            &font,
            pixels_per_font_unit,
            settings.font_size as f32,
            &mut primitives,
        )?;

        let (atlas_width, atlas_height, atlas) = pack_atlas(&mut primitives)?;
        let mut vertices = Vec::new();
        let mut indices = Vec::new();

        for primitive in primitives {
            match primitive {
                Primitive::Glyph(glyph) => {
                    if glyph.width == 0 || glyph.height == 0 {
                        continue;
                    }

                    let u0 = glyph.atlas_x as f32 / atlas_width as f32;
                    let v0 = glyph.atlas_y as f32 / atlas_height as f32;
                    let u1 = (glyph.atlas_x + glyph.width) as f32 / atlas_width as f32;
                    let v1 = (glyph.atlas_y + glyph.height) as f32 / atlas_height as f32;

                    push_quad(
                        &mut vertices,
                        &mut indices,
                        width,
                        height,
                        glyph.left,
                        glyph.top,
                        glyph.left + glyph.width as f32,
                        glyph.top + glyph.height as f32,
                        [u0, v0, u1, v1],
                        glyph.color,
                        true,
                    );
                }
                Primitive::Solid {
                    left,
                    top,
                    right,
                    bottom,
                    color,
                } => push_quad(
                    &mut vertices,
                    &mut indices,
                    width,
                    height,
                    left,
                    top,
                    right,
                    bottom,
                    [0.0, 0.0, 0.0, 0.0],
                    color,
                    false,
                ),
            }
        }

        Ok(PreparedScene {
            width,
            height,
            atlas_width,
            atlas_height,
            atlas,
            vertices,
            indices,
        })
    }
}

fn collect_primitives(
    nodes: &[SceneNode],
    inherited_color: [f32; 4],
    font: &Font,
    pixels_per_font_unit: f32,
    font_size: f32,
    output: &mut Vec<Primitive>,
) -> Result<(), WebGpuError> {
    for node in nodes {
        match *node {
            SceneNode::Glyph(ref glyph) => {
                let character =
                    char::from_u32(glyph.unicode).ok_or(WebGpuError::InvalidGlyph(glyph.unicode))?;
                let size = (font_size * glyph.scale as f32).max(1.0);
                let (metrics, bitmap) = font.rasterize(character, size);
                let baseline_x = f64::from(glyph.position.x) as f32 * pixels_per_font_unit;
                let baseline_y = f64::from(glyph.position.y) as f32 * pixels_per_font_unit;

                output.push(Primitive::Glyph(RasterGlyph {
                    left: baseline_x + metrics.xmin as f32,
                    top: baseline_y - (metrics.ymin as f32 + metrics.height as f32),
                    width: metrics.width as u32,
                    height: metrics.height as u32,
                    bitmap,
                    atlas_x: 0,
                    atlas_y: 0,
                    color: inherited_color,
                }));
            }
            SceneNode::Rule(ref rule) => {
                let left = f64::from(rule.position.x) as f32 * pixels_per_font_unit;
                let top = f64::from(rule.position.y) as f32 * pixels_per_font_unit;
                output.push(Primitive::Solid {
                    left,
                    top,
                    right: left + f64::from(rule.width) as f32 * pixels_per_font_unit,
                    bottom: top + f64::from(rule.height) as f32 * pixels_per_font_unit,
                    color: black(),
                });
            }
            SceneNode::DebugBox(ref debug) => {
                let left = f64::from(debug.position.x) as f32 * pixels_per_font_unit;
                let top = f64::from(debug.position.y) as f32 * pixels_per_font_unit;
                let right = left + f64::from(debug.width) as f32 * pixels_per_font_unit;
                let bottom = top + f64::from(debug.height) as f32 * pixels_per_font_unit;
                let thickness =
                    (f64::from(FontUnit::from(8)) as f32 * pixels_per_font_unit).max(0.5);
                let color = debug_color(debug.kind);
                push_outline(output, left, top, right, bottom, thickness, color);
            }
            SceneNode::Color(ref color) => {
                collect_primitives(
                    &color.contents,
                    scene_color(color.color),
                    font,
                    pixels_per_font_unit,
                    font_size,
                    output,
                )?;
            }
        }
    }

    Ok(())
}

fn push_outline(
    output: &mut Vec<Primitive>,
    left: f32,
    top: f32,
    right: f32,
    bottom: f32,
    thickness: f32,
    color: [f32; 4],
) {
    output.push(Primitive::Solid {
        left,
        top,
        right,
        bottom: (top + thickness).min(bottom),
        color,
    });
    output.push(Primitive::Solid {
        left,
        top: (bottom - thickness).max(top),
        right,
        bottom,
        color,
    });
    output.push(Primitive::Solid {
        left,
        top,
        right: (left + thickness).min(right),
        bottom,
        color,
    });
    output.push(Primitive::Solid {
        left: (right - thickness).max(left),
        top,
        right,
        bottom,
        color,
    });
}

fn pack_atlas(primitives: &mut [Primitive]) -> Result<(u32, u32, Vec<u8>), WebGpuError> {
    let mut x = ATLAS_PADDING;
    let mut y = ATLAS_PADDING;
    let mut row_height = 0;
    let mut used_width = 1;
    let mut used_height = 1;

    for primitive in primitives.iter_mut() {
        let glyph = match *primitive {
            Primitive::Glyph(ref mut glyph) if glyph.width > 0 && glyph.height > 0 => glyph,
            _ => continue,
        };

        if glyph.width + 2 * ATLAS_PADDING > MAX_ATLAS_DIMENSION
            || glyph.height + 2 * ATLAS_PADDING > MAX_ATLAS_DIMENSION
        {
            return Err(WebGpuError::AtlasTooLarge);
        }

        if x + glyph.width + ATLAS_PADDING > MAX_ATLAS_DIMENSION {
            x = ATLAS_PADDING;
            y += row_height + ATLAS_PADDING;
            row_height = 0;
        }

        if y + glyph.height + ATLAS_PADDING > MAX_ATLAS_DIMENSION {
            return Err(WebGpuError::AtlasTooLarge);
        }

        glyph.atlas_x = x;
        glyph.atlas_y = y;
        x += glyph.width + ATLAS_PADDING;
        row_height = row_height.max(glyph.height);
        used_width = used_width.max(x);
        used_height = used_height.max(y + glyph.height + ATLAS_PADDING);
    }

    let width = used_width.max(1);
    let height = used_height.max(1);
    let mut atlas = vec![0; (width * height) as usize];

    for primitive in primitives {
        let glyph = match *primitive {
            Primitive::Glyph(ref glyph) if glyph.width > 0 && glyph.height > 0 => glyph,
            _ => continue,
        };

        for row in 0..glyph.height {
            let source_start = (row * glyph.width) as usize;
            let source_end = source_start + glyph.width as usize;
            let destination_start =
                ((glyph.atlas_y + row) * width + glyph.atlas_x) as usize;
            let destination_end = destination_start + glyph.width as usize;
            atlas[destination_start..destination_end]
                .copy_from_slice(&glyph.bitmap[source_start..source_end]);
        }
    }

    Ok((width, height, atlas))
}

fn push_quad(
    vertices: &mut Vec<Vertex>,
    indices: &mut Vec<u32>,
    output_width: u32,
    output_height: u32,
    left: f32,
    top: f32,
    right: f32,
    bottom: f32,
    uv: [f32; 4],
    color: [f32; 4],
    textured: bool,
) {
    if right <= left || bottom <= top {
        return;
    }

    let x0 = pixel_to_ndc_x(left, output_width);
    let x1 = pixel_to_ndc_x(right, output_width);
    let y0 = pixel_to_ndc_y(top, output_height);
    let y1 = pixel_to_ndc_y(bottom, output_height);
    let base = vertices.len() as u32;
    let flag = if textured { 1.0 } else { 0.0 };

    vertices.extend_from_slice(&[
        Vertex {
            position: [x0, y0],
            uv: [uv[0], uv[1]],
            color,
            textured: flag,
        },
        Vertex {
            position: [x1, y0],
            uv: [uv[2], uv[1]],
            color,
            textured: flag,
        },
        Vertex {
            position: [x1, y1],
            uv: [uv[2], uv[3]],
            color,
            textured: flag,
        },
        Vertex {
            position: [x0, y1],
            uv: [uv[0], uv[3]],
            color,
            textured: flag,
        },
    ]);
    indices.extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
}

fn pixel_to_ndc_x(value: f32, width: u32) -> f32 {
    value / width as f32 * 2.0 - 1.0
}

fn pixel_to_ndc_y(value: f32, height: u32) -> f32 {
    1.0 - value / height as f32 * 2.0
}

fn black() -> [f32; 4] {
    [0.0, 0.0, 0.0, 1.0]
}

fn scene_color(color: Color) -> [f32; 4] {
    [
        color.red as f32 / 255.0,
        color.green as f32 / 255.0,
        color.blue as f32 / 255.0,
        color.alpha as f32 / 255.0,
    ]
}

fn debug_color(kind: DebugBoxKind) -> [f32; 4] {
    match kind {
        DebugBoxKind::HorizontalBox => [0.0, 0.0, 1.0, 1.0],
        DebugBoxKind::VerticalBox => [1.0, 0.0, 0.0, 1.0],
        DebugBoxKind::Glyph => [0.0, 0.5, 0.0, 1.0],
    }
}

async fn render_prepared(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    prepared: PreparedScene,
) -> Result<WebGpuImage, WebGpuError> {
    let atlas_texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("ReX glyph atlas"),
        size: wgpu::Extent3d {
            width: prepared.atlas_width,
            height: prepared.atlas_height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::R8Unorm,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    });
    queue.write_texture(
        wgpu::TexelCopyTextureInfo {
            texture: &atlas_texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        &prepared.atlas,
        wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(prepared.atlas_width),
            rows_per_image: Some(prepared.atlas_height),
        },
        wgpu::Extent3d {
            width: prepared.atlas_width,
            height: prepared.atlas_height,
            depth_or_array_layers: 1,
        },
    );

    let atlas_view = atlas_texture.create_view(&wgpu::TextureViewDescriptor::default());
    let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
        label: Some("ReX glyph atlas sampler"),
        mag_filter: wgpu::FilterMode::Linear,
        min_filter: wgpu::FilterMode::Linear,
        ..Default::default()
    });

    let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("ReX glyph atlas bind group layout"),
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
    let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("ReX glyph atlas bind group"),
        layout: &bind_group_layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(&atlas_view),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::Sampler(&sampler),
            },
        ],
    });

    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("ReX WebGPU shader"),
        source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(include_str!("webgpu.wgsl"))),
    });
    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("ReX WebGPU pipeline layout"),
        bind_group_layouts: &[Some(&bind_group_layout)],
        immediate_size: 0,
    });
    let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("ReX WebGPU pipeline"),
        layout: Some(&pipeline_layout),
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: Some("vs_main"),
            buffers: &[Vertex::layout()],
            compilation_options: wgpu::PipelineCompilationOptions::default(),
        },
        fragment: Some(wgpu::FragmentState {
            module: &shader,
            entry_point: Some("fs_main"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            targets: &[Some(wgpu::ColorTargetState {
                format: wgpu::TextureFormat::Rgba8Unorm,
                blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                write_mask: wgpu::ColorWrites::ALL,
            })],
        }),
        primitive: wgpu::PrimitiveState::default(),
        depth_stencil: None,
        multisample: wgpu::MultisampleState::default(),
        multiview_mask: None,
        cache: None,
    });

    let target = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("ReX WebGPU target"),
        size: wgpu::Extent3d {
            width: prepared.width,
            height: prepared.height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8Unorm,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    });
    let target_view = target.create_view(&wgpu::TextureViewDescriptor::default());

    let vertex_buffer = if prepared.vertices.is_empty() {
        device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("ReX WebGPU empty vertex buffer"),
            size: std::mem::size_of::<Vertex>() as u64,
            usage: wgpu::BufferUsages::VERTEX,
            mapped_at_creation: false,
        })
    } else {
        device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("ReX WebGPU vertices"),
            contents: bytemuck::cast_slice(&prepared.vertices),
            usage: wgpu::BufferUsages::VERTEX,
        })
    };
    let index_buffer = if prepared.indices.is_empty() {
        device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("ReX WebGPU empty index buffer"),
            size: std::mem::size_of::<u32>() as u64,
            usage: wgpu::BufferUsages::INDEX,
            mapped_at_creation: false,
        })
    } else {
        device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("ReX WebGPU indices"),
            contents: bytemuck::cast_slice(&prepared.indices),
            usage: wgpu::BufferUsages::INDEX,
        })
    };

    let unpadded_bytes_per_row = prepared.width * 4;
    let padded_bytes_per_row = align_to(
        unpadded_bytes_per_row,
        wgpu::COPY_BYTES_PER_ROW_ALIGNMENT,
    );
    let readback_size = padded_bytes_per_row as u64 * prepared.height as u64;
    let readback = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("ReX WebGPU readback"),
        size: readback_size,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });

    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("ReX WebGPU encoder"),
    });
    {
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("ReX WebGPU render pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &target_view,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color {
                        r: 0.0,
                        g: 0.0,
                        b: 0.0,
                        a: 0.0,
                    }),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });
        pass.set_pipeline(&pipeline);
        pass.set_bind_group(0, &bind_group, &[]);
        if !prepared.indices.is_empty() {
            pass.set_vertex_buffer(0, vertex_buffer.slice(..));
            pass.set_index_buffer(index_buffer.slice(..), wgpu::IndexFormat::Uint32);
            pass.draw_indexed(0..prepared.indices.len() as u32, 0, 0..1);
        }
    }

    encoder.copy_texture_to_buffer(
        wgpu::TexelCopyTextureInfo {
            texture: &target,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        wgpu::TexelCopyBufferInfo {
            buffer: &readback,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(padded_bytes_per_row),
                rows_per_image: Some(prepared.height),
            },
        },
        wgpu::Extent3d {
            width: prepared.width,
            height: prepared.height,
            depth_or_array_layers: 1,
        },
    );
    queue.submit([encoder.finish()]);

    let slice = readback.slice(..);
    let (sender, receiver) = oneshot::channel();
    slice.map_async(wgpu::MapMode::Read, move |result| {
        let _ = sender.send(result);
    });

    #[cfg(not(target_arch = "wasm32"))]
    device
        .poll(wgpu::PollType::wait_indefinitely())
        .map_err(|error| WebGpuError::DevicePoll(format!("{:?}", error)))?;

    let map_result = receiver
        .await
        .map_err(|_| WebGpuError::BufferMap("mapping callback was dropped".into()))?;
    map_result.map_err(|error| WebGpuError::BufferMap(format!("{:?}", error)))?;

    let mapped = slice.get_mapped_range();
    let mut rgba =
        Vec::with_capacity((unpadded_bytes_per_row * prepared.height) as usize);
    for row in 0..prepared.height {
        let start = (row * padded_bytes_per_row) as usize;
        let end = start + unpadded_bytes_per_row as usize;
        rgba.extend_from_slice(&mapped[start..end]);
    }
    drop(mapped);
    readback.unmap();

    Ok(WebGpuImage {
        width: prepared.width,
        height: prepared.height,
        rgba,
    })
}

fn align_to(value: u32, alignment: u32) -> u32 {
    ((value + alignment - 1) / alignment) * alignment
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scene::{Point, SceneGlyph};

    #[test]
    fn prepares_scene_without_touching_a_gpu() {
        let settings = RenderSettings::default().font_size(48);
        let scene = MathScene {
            width: FontUnit::from(1000),
            height: FontUnit::from(1000),
            nodes: vec![SceneNode::Glyph(SceneGlyph {
                position: Point {
                    x: FontUnit::from(0),
                    y: FontUnit::from(800),
                },
                unicode: 'x' as u32,
                scale: 1.0,
            })],
        };

        let prepared = PreparedScene::new(&scene, &settings).unwrap();
        assert_eq!(prepared.width, 48);
        assert_eq!(prepared.height, 48);
        assert!(!prepared.atlas.is_empty());
        assert!(!prepared.vertices.is_empty());
        assert!(!prepared.indices.is_empty());
    }

    #[test]
    fn aligns_readback_rows_to_webgpu_requirement() {
        assert_eq!(align_to(4, 256), 256);
        assert_eq!(align_to(256, 256), 256);
        assert_eq!(align_to(260, 256), 512);
    }
}
