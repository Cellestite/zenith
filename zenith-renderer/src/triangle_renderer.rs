use std::sync::Arc;
use wgpu::util::DeviceExt;
use zenith_build::triangle::{self, VertexInput as Vertex};
use zenith_build::{ShaderEntry};
use zenith_core::collections::SmallVec;
use zenith_render::{define_shader, GraphicShader, RenderDevice};
use zenith_rendergraph::{Buffer, BufferDesc, ColorInfoBuilder, RenderGraphBuilder, RenderGraphResource, SharedRenderGraphResource, Texture, TextureDesc};

pub struct TriangleRenderer {
    vertex_buffer: SharedRenderGraphResource<Buffer>,
    index_buffer: SharedRenderGraphResource<Buffer>,
    shader: Arc<GraphicShader>,
    start_time: std::time::Instant,
}

impl TriangleRenderer {
    pub fn new(device: &RenderDevice) -> Self {
        let vertices = [
            Vertex { position: [0.0, 0.5, 0.0].into(), color: [1.0, 0.0, 0.0].into() },
            Vertex { position: [-0.5, -0.5, 0.0].into(), color: [0.0, 1.0, 0.0].into() },
            Vertex { position: [0.5, -0.5, 0.0].into(), color: [0.0, 0.0, 1.0].into() },
        ];
        let indices = [0u16, 1, 2];

        let device = device.device();
        let vertex_buffer = SharedRenderGraphResource::new(device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("triangle vertex buffer"),
            contents: bytemuck::cast_slice(&vertices),
            usage: wgpu::BufferUsages::VERTEX,
        }));

        let index_buffer = SharedRenderGraphResource::new(device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("triangle index buffer"),
            contents: bytemuck::cast_slice(&indices),
            usage: wgpu::BufferUsages::INDEX,
        }));

        define_shader! {
            let shader = Graphic(triangle, "triangle.wgsl", ShaderEntry::Triangle, wgpu::VertexStepMode::Vertex, 1, 1)
        }
        let shader = Arc::new(shader.unwrap());

        Self {
            vertex_buffer,
            index_buffer,
            shader,
            start_time: std::time::Instant::now()
        }
    }

    pub fn build_render_graph(&self, builder: &mut RenderGraphBuilder, width: u32, height: u32) -> RenderGraphResource<Texture> {
        let vb = builder.import("triangle.vertex", self.vertex_buffer.clone(), wgpu::BufferUses::VERTEX);
        let ib = builder.import("triangle.index", self.index_buffer.clone(), wgpu::BufferUses::INDEX);

        let mut output = builder.create("triangle.output", TextureDesc {
            label: Some("triangle output render target"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Bgra8UnormSrgb,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[wgpu::TextureFormat::Bgra8UnormSrgb],
        });

        let uniform = builder.create("triangle.transform", BufferDesc {
            label: Some("triangle uniform buffer"),
            size: size_of::<[[f32; 4]; 4]>() as wgpu::BufferAddress,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        {
            let mut node = builder.add_graphic_node("triangle");

            let vb = node.read(&vb, wgpu::BufferUses::VERTEX);
            let ib = node.read(&ib, wgpu::BufferUses::INDEX);
            let uniform = node.read(&uniform, wgpu::BufferUses::UNIFORM);
            let output = node.write(&mut output, wgpu::TextureUses::COLOR_TARGET);

            node.setup_pipeline()
                .with_shader(self.shader.clone())
                .with_color(output, ColorInfoBuilder::default().build().unwrap());

            let start_time = self.start_time;

            node.execute(move |ctx, encoder| {
                let elapsed = start_time.elapsed().as_secs_f32();
                let rotation_angle = elapsed * std::f32::consts::PI / 2.0;
                let rotation_mat = glam::Mat4::from_rotation_z(rotation_angle);

                let uniform_data = triangle::Uniforms::new(rotation_mat);
                ctx.write_buffer(&uniform, 0, uniform_data);
                
                let uniform_buffer = ctx.get_buffer(&uniform);
                let vertex_buffer = ctx.get_buffer(&vb);
                let index_buffer = ctx.get_buffer(&ib);

                let mut render_pass = ctx.begin_render_pass(encoder);
                ctx.bind_pipeline(&mut render_pass)
                    .with_binding(0, 0, uniform_buffer.as_entire_binding())
                    .bind();

                render_pass.set_vertex_buffer(0, vertex_buffer.slice(..));
                render_pass.set_index_buffer(index_buffer.slice(..), wgpu::IndexFormat::Uint16);
                render_pass.draw_indexed(0..3, 0, 0..1);
            });
        }

        output
    }
}