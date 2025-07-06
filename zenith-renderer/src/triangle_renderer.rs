use std::sync::{Arc};
use wgpu::util::DeviceExt;
use zenith_render::{GraphicShader, RenderDevice, VertexBufferLayout};
use zenith_rendergraph::{RenderGraphBuilder};

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct Vertex {
    position: [f32; 3],
    color: [f32; 3],
}

pub struct TriangleRenderer {
    vertex_buffer: Arc<wgpu::Buffer>,
    index_buffer: Arc<wgpu::Buffer>,
    shader: Arc<GraphicShader>,
    start_time: std::time::Instant,
}

impl TriangleRenderer {
    pub fn new(device: &RenderDevice) -> Self {
        let device = device.device();
        let vertices = [
            Vertex { position: [0.0, 0.5, 0.0], color: [1.0, 0.0, 0.0] },
            Vertex { position: [-0.5, -0.5, 0.0], color: [0.0, 1.0, 0.0] },
            Vertex { position: [0.5, -0.5, 0.0], color: [0.0, 0.0, 1.0] },
        ];

        let indices = [0u16, 1, 2];

        let vertex_buffer = Arc::new(device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("triangle vertex buffer"),
            contents: bytemuck::cast_slice(&vertices),
            usage: wgpu::BufferUsages::VERTEX,
        }));

        let index_buffer = Arc::new(device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("triangle index buffer"),
            contents: bytemuck::cast_slice(&indices),
            usage: wgpu::BufferUsages::INDEX,
        }));

        let shader_source = r#"
            struct Uniforms {
                transform: mat4x4<f32>,
            }

            @group(0) @binding(0)
            var<uniform> uniforms: Uniforms;

            struct VertexInput {
                @location(0) position: vec3<f32>,
                @location(1) color: vec3<f32>,
            }

            struct VertexOutput {
                @builtin(position) position: vec4<f32>,
                @location(0) color: vec3<f32>,
            }

            @vertex
            fn vs_main(input: VertexInput) -> VertexOutput {
                var output: VertexOutput;
                output.position = uniforms.transform * vec4<f32>(input.position, 1.0);
                output.color = input.color;
                return output;
            }

            @fragment
            fn fs_main(@location(0) color: vec3<f32>) -> @location(0) vec4<f32> {
                return vec4<f32>(color, 1.0);
            }
        "#;

        // TODO: shader reflection
        let vertex_layout = VertexBufferLayout::with_attributes_count(2)
            .push_attribute(wgpu::VertexFormat::Float32x3)
            .push_attribute(wgpu::VertexFormat::Float32x3);

        let shader = Arc::new(GraphicShader::new(
            "triangle",
            shader_source.to_string(),
            "vs_main",
            Some("fs_main"),
            vertex_layout,
        ));

        Self {
            vertex_buffer,
            index_buffer,
            shader,
            start_time: std::time::Instant::now()
        }
    }

    pub fn build_render_graph(&self, builder: &mut RenderGraphBuilder, device: &RenderDevice) -> wgpu::SurfaceTexture {
        let surface_tex = device.require_presentation();
        let output_tex = Arc::new(surface_tex.texture.clone());

        let vb = builder.import("triangle.vertex", self.vertex_buffer.clone(), wgpu::BufferUses::empty());
        let ib = builder.import("triangle.index", self.index_buffer.clone(), wgpu::BufferUses::empty());
        let output = builder.import("triangle.output", output_tex, wgpu::TextureUses::PRESENT);

        let uniform = builder.create("triangle.transform", wgpu::BufferDescriptor {
            label: Some("triangle uniform buffer"),
            size: size_of::<[[f32; 4]; 4]>() as wgpu::BufferAddress,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        {
            let mut node = builder.add_graphic_node("triangle");

            let vb = node.read(vb, wgpu::BufferUses::VERTEX);
            let ib = node.read(ib, wgpu::BufferUses::INDEX);
            let uniform = node.read(uniform, wgpu::BufferUses::UNIFORM);
            let output = node.write(output, wgpu::TextureUses::COLOR_TARGET);

            node.setup_pipeline()
                .with_shader(self.shader.clone())
                .with_color(output, Default::default())
                .with_binding(0, uniform.clone());

            let start_time = self.start_time;

            node.record_command(move |ctx| {
                let elapsed = start_time.elapsed().as_secs_f32();
                let rotation_angle = elapsed * std::f32::consts::PI / 2.0;

                let rotation_mat = glam::Mat4::from_rotation_z(rotation_angle);
                ctx.write_buffer(&uniform, 0, bytemuck::cast_slice(rotation_mat.as_ref()));
                
                let uniform_buffer = ctx.get_buffer(&uniform);
                let vertex_buffer = ctx.get_buffer(&vb);
                let index_buffer = ctx.get_buffer(&ib);

                ctx.render_pass.borrow_mut().set_vertex_buffer(0, vertex_buffer.slice(..));
                ctx.render_pass.borrow_mut().set_index_buffer(index_buffer.slice(..), wgpu::IndexFormat::Uint16);
                ctx.bind_pipeline()
                    .with_binding(0, uniform_buffer.as_entire_binding())
                    .bind();

                ctx.render_pass.borrow_mut().draw_indexed(0..3, 0, 0..1);
            });
        }

        surface_tex
    }
}