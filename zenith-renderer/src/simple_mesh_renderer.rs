use std::sync::Arc;
use wgpu::util::DeviceExt;
use zenith_build::{ShaderEntry};
use zenith_core::asset_loader::{MeshData, ModelData};
use zenith_core::collections::SmallVec;
use zenith_render::{define_shader, GraphicShader, RenderDevice};
use zenith_rendergraph::{Buffer, RenderGraphBuilder, RenderGraphResource, SharedRenderGraphResource, Texture, TextureDesc};

pub struct SimpleMeshRenderer {
    meshes: Vec<MeshBuffers>,
    shader: Arc<GraphicShader>,
    base_color: [f32; 3],
}

struct MeshBuffers {
    vertex_buffer: SharedRenderGraphResource<Buffer>,
    index_buffer: SharedRenderGraphResource<Buffer>,
    index_count: u32,
    _name: Option<String>,
}

impl SimpleMeshRenderer {
    pub fn from_model(device: &RenderDevice, model: &ModelData) -> Self {
        let meshes = model
            .meshes
            .iter()
            .map(|mesh| Self::create_mesh_buffers(device, mesh))
            .collect();

        let shader = Self::create_shader();
        
        Self {
            meshes,
            shader: Arc::new(shader),
            base_color: [0.8, 0.8, 0.8], // 默认灰色
        }
    }

    pub fn set_base_color(&mut self, color: [f32; 3]) {
        self.base_color = color;
    }
    
    fn create_mesh_buffers(device: &RenderDevice, mesh: &MeshData) -> MeshBuffers {
        let device = device.device();

        let vertex_buffer = SharedRenderGraphResource::new(device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some(&format!("Mesh Vertex Buffer: {:?}", mesh.name)),
            contents: mesh.vertex_bytes(),
            usage: wgpu::BufferUsages::VERTEX,
        }));

        let index_buffer = SharedRenderGraphResource::new(device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some(&format!("Mesh Index Buffer: {:?}", mesh.name)),
            contents: mesh.index_bytes(),
            usage: wgpu::BufferUsages::INDEX,
        }));

        MeshBuffers {
            vertex_buffer,
            index_buffer,
            index_count: mesh.indices.len() as u32,
            _name: mesh.name.clone(),
        }
    }
    
    fn create_shader() -> GraphicShader {
        define_shader! {
            let shader = Graphic(mesh, "mesh.wgsl", ShaderEntry::Mesh, wgpu::VertexStepMode::Vertex, 1, 1)
        }
        shader.unwrap()
    }

    pub fn build_render_graph(
        &self, 
        builder: &mut RenderGraphBuilder, 
        view_matrix: glam::Mat4,
        proj_matrix: glam::Mat4,
        model_matrix: glam::Mat4,
        width: u32,
        height: u32,
    ) -> RenderGraphResource<Texture>  {
        let mut output = builder.create("triangle.output", TextureDesc {
            label: Some("mesh output render target"),
            size: wgpu::Extent3d {
                width: width.max(1),
                height: height.max(1),
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Bgra8UnormSrgb,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[wgpu::TextureFormat::Bgra8UnormSrgb],
        });

        // 创建 view uniform 缓冲区
        let view_uniform = builder.create("mesh.view_uniform", wgpu::BufferDescriptor {
            label: Some("View Uniform Buffer"),
            size: std::mem::size_of::<[[f32; 4]; 4]>() as wgpu::BufferAddress,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // 创建 model uniform 缓冲区
        let model_uniform = builder.create("mesh.model_uniform", wgpu::BufferDescriptor {
            label: Some("Model Uniform Buffer"),
            size: (std::mem::size_of::<[[f32; 4]; 4]>() + std::mem::size_of::<[f32; 3]>() + 4) as wgpu::BufferAddress, // 添加 padding
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // 导入所有网格缓冲区
        let mesh_resources: Vec<_> = self.meshes.iter().enumerate().map(|(i, mesh)| {
            let vb = builder.import(
                &format!("mesh.vertex.{}", i), 
                mesh.vertex_buffer.clone(), 
                wgpu::BufferUses::empty()
            );
            let ib = builder.import(
                &format!("mesh.index.{}", i), 
                mesh.index_buffer.clone(), 
                wgpu::BufferUses::empty()
            );
            (vb, ib, mesh.index_count)
        }).collect();

        {
            let mut node = builder.add_graphic_node("mesh_render");

            let view_uniform = node.read(&view_uniform, wgpu::BufferUses::UNIFORM);
            let model_uniform = node.read(&model_uniform, wgpu::BufferUses::UNIFORM);
            let output = node.write(&mut output, wgpu::TextureUses::COLOR_TARGET);

            let mesh_reads: Vec<_> = mesh_resources.iter().map(|(vb, ib, _)| {
                let vb_read = node.read(&vb, wgpu::BufferUses::VERTEX);
                let ib_read = node.read(&ib, wgpu::BufferUses::INDEX);
                (vb_read, ib_read)
            }).collect();

            node.setup_pipeline()
                .with_shader(self.shader.clone())
                .with_color(output, Default::default());

            let view_proj = proj_matrix * view_matrix;
            let base_color = self.base_color.into();

            node.execute(move |ctx, encoder| {
                let view_uniform_data = zenith_build::mesh::ViewUniforms::new(view_proj);
                ctx.write_buffer(&view_uniform, 0, view_uniform_data);
                let model_uniform_data = zenith_build::mesh::ModelUniforms::new(model_matrix, base_color);
                ctx.write_buffer(&model_uniform, 0, model_uniform_data);

                let view_buffer = ctx.get_buffer(&view_uniform);
                let model_buffer = ctx.get_buffer(&model_uniform);

                let mut render_pass = ctx.begin_render_pass(encoder);

                ctx.bind_pipeline(&mut render_pass)
                    .with_binding(0, 0, view_buffer.as_entire_binding())
                    .with_binding(0, 1, model_buffer.as_entire_binding())
                    .bind();
                
                for ((vb_read, ib_read), (_, _, index_count)) in mesh_reads.iter().zip(mesh_resources.iter()) {
                    let vertex_buffer = ctx.get_buffer(vb_read);
                    let index_buffer = ctx.get_buffer(ib_read);

                    render_pass.set_vertex_buffer(0, vertex_buffer.slice(..));
                    render_pass.set_index_buffer(index_buffer.slice(..), wgpu::IndexFormat::Uint32);
                    render_pass.draw_indexed(0..*index_count, 0, 0..1);
                }
            });
        }

        output
    }
} 