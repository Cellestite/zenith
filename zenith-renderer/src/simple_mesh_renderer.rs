use std::sync::Arc;
use wgpu::util::DeviceExt;
use zenith_build::{ShaderEntry};
use zenith_render::{MeshData, ModelData, PbrMaterial, TextureData};
use zenith_core::collections::SmallVec;
use zenith_render::{define_shader, GraphicShader, RenderDevice};
use zenith_rendergraph::{Buffer, DepthStencilInfo, RenderGraphBuilder, RenderGraphResource, RenderResource, Texture, TextureDesc};

pub struct SimpleMeshRenderer {
    meshes: Vec<MeshBuffers>,
    materials: Vec<MaterialResources>,
    default_texture: RenderResource<wgpu::Texture>,
    default_sampler: Arc<wgpu::Sampler>,
    shader: Arc<GraphicShader>,
    base_color: [f32; 3],
}

struct MeshBuffers {
    vertex_buffer: RenderResource<Buffer>,
    index_buffer: RenderResource<Buffer>,
    index_count: u32,
    material_index: Option<usize>,
    _name: Option<String>,
}

struct MaterialResources {
    base_color_texture: Option<RenderResource<wgpu::Texture>>,
    base_color_sampler: Arc<wgpu::Sampler>,
    _material: PbrMaterial,
}

impl SimpleMeshRenderer {
    fn gltf_format_to_wgpu(format: gltf::image::Format) -> (wgpu::TextureFormat, u32) {
        match format {
            gltf::image::Format::R8 => (wgpu::TextureFormat::R8Unorm, 1),
            gltf::image::Format::R8G8 => (wgpu::TextureFormat::Rg8Unorm, 2),
            gltf::image::Format::R8G8B8 => (wgpu::TextureFormat::Rgba8UnormSrgb, 4), // Convert RGB to RGBA
            gltf::image::Format::R8G8B8A8 => (wgpu::TextureFormat::Rgba8UnormSrgb, 4),
            gltf::image::Format::R16 => (wgpu::TextureFormat::R16Unorm, 2),
            gltf::image::Format::R16G16 => (wgpu::TextureFormat::Rg16Unorm, 4),
            gltf::image::Format::R16G16B16 => (wgpu::TextureFormat::Rgba16Unorm, 8), // Convert RGB to RGBA
            gltf::image::Format::R16G16B16A16 => (wgpu::TextureFormat::Rgba16Unorm, 8),
            gltf::image::Format::R32G32B32FLOAT => (wgpu::TextureFormat::Rgba32Float, 16), // Convert RGB to RGBA
            gltf::image::Format::R32G32B32A32FLOAT => (wgpu::TextureFormat::Rgba32Float, 16),
        }
    }

    fn convert_texture_data(texture_data: &TextureData) -> Vec<u8> {
        match texture_data.format {
            gltf::image::Format::R8G8B8 => {
                // Convert RGB to RGBA by adding alpha channel
                let mut rgba_data = Vec::with_capacity(texture_data.pixels.len() * 4 / 3);
                for chunk in texture_data.pixels.chunks(3) {
                    rgba_data.extend_from_slice(chunk);
                    rgba_data.push(255); // Add alpha = 1.0
                }
                rgba_data
            }
            gltf::image::Format::R16G16B16 => {
                // Convert RGB16 to RGBA16 by adding alpha channel
                let mut rgba_data = Vec::with_capacity(texture_data.pixels.len() * 4 / 3);
                for chunk in texture_data.pixels.chunks(6) { // 6 bytes = 3 * 16-bit values
                    rgba_data.extend_from_slice(chunk);
                    rgba_data.extend_from_slice(&[255, 255]); // Add alpha = 1.0 (16-bit)
                }
                rgba_data
            }
            gltf::image::Format::R32G32B32FLOAT => {
                // Convert RGB32F to RGBA32F by adding alpha channel
                let mut rgba_data = Vec::with_capacity(texture_data.pixels.len() * 4 / 3);
                for chunk in texture_data.pixels.chunks(12) { // 12 bytes = 3 * 32-bit floats
                    rgba_data.extend_from_slice(chunk);
                    rgba_data.extend_from_slice(&1.0f32.to_le_bytes()); // Add alpha = 1.0 (32-bit float)
                }
                rgba_data
            }
            _ => texture_data.pixels.clone(), // Already in correct format
        }
    }
    pub fn from_model(device: &RenderDevice, model: &ModelData) -> Self {
        let materials = model
            .materials
            .materials
            .iter()
            .map(|material| Self::create_material_resources(device, material))
            .collect();
            
        let meshes = model
            .meshes
            .iter()
            .map(|mesh| Self::create_mesh_buffers(device, mesh))
            .collect();

        let (default_texture, default_sampler) = Self::create_default_texture(device);
        let shader = Self::create_shader();
        
        Self {
            meshes,
            materials,
            default_texture,
            default_sampler,
            shader: Arc::new(shader),
            base_color: [0.8, 0.8, 0.8],
        }
    }

    pub fn set_base_color(&mut self, color: [f32; 3]) {
        self.base_color = color;
    }
    
    fn create_mesh_buffers(device: &RenderDevice, mesh: &MeshData) -> MeshBuffers {
        let device = device.device();

        let vertex_buffer = RenderResource::new(device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some(&format!("Mesh Vertex Buffer: {:?}", mesh.name)),
            contents: mesh.vertex_bytes(),
            usage: wgpu::BufferUsages::VERTEX,
        }));

        let index_buffer = RenderResource::new(device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some(&format!("Mesh Index Buffer: {:?}", mesh.name)),
            contents: mesh.index_bytes(),
            usage: wgpu::BufferUsages::INDEX,
        }));

        MeshBuffers {
            vertex_buffer,
            index_buffer,
            index_count: mesh.indices.len() as u32,
            material_index: mesh.material_index,
            _name: mesh.name.clone(),
        }
    }
    
    fn create_material_resources(render_device: &RenderDevice, material: &PbrMaterial) -> MaterialResources {
        let device = render_device.device();
        
        let base_color_texture = if let Some(texture_data) = &material.textures.base_color {
            let (wgpu_format, bytes_per_pixel) = Self::gltf_format_to_wgpu(texture_data.format);
            let converted_pixels = Self::convert_texture_data(texture_data);
            
            let texture = device.create_texture(&wgpu::TextureDescriptor {
                label: Some(&format!("Base Color Texture: {:?}", material.name)),
                size: wgpu::Extent3d {
                    width: texture_data.width,
                    height: texture_data.height,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu_format,
                usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
                view_formats: &[],
            });
            
            // Upload the texture data to the GPU
            render_device.queue().write_texture(
                wgpu::TexelCopyTextureInfo {
                    texture: &texture,
                    mip_level: 0,
                    origin: wgpu::Origin3d::ZERO,
                    aspect: wgpu::TextureAspect::All,
                },
                &converted_pixels,
                wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(texture_data.width * bytes_per_pixel),
                    rows_per_image: Some(texture_data.height),
                },
                wgpu::Extent3d {
                    width: texture_data.width,
                    height: texture_data.height,
                    depth_or_array_layers: 1,
                },
            );
            
            Some(RenderResource::new(texture))
        } else {
            None
        };
        
        let base_color_sampler = Arc::new(device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some(&format!("Base Color Sampler: {:?}", material.name)),
            address_mode_u: wgpu::AddressMode::Repeat,
            address_mode_v: wgpu::AddressMode::Repeat,
            address_mode_w: wgpu::AddressMode::Repeat,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        }));
        
        MaterialResources {
            base_color_texture,
            base_color_sampler,
            _material: material.clone(),
        }
    }
    
    fn create_default_texture(render_device: &RenderDevice) -> (RenderResource<wgpu::Texture>, Arc<wgpu::Sampler>) {
        let device = render_device.device();
        
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Default White Texture"),
            size: wgpu::Extent3d {
                width: 1,
                height: 1,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        
        let white_pixel = [255u8; 4]; // White RGBA
        render_device.queue().write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &white_pixel,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(4),
                rows_per_image: Some(1),
            },
            wgpu::Extent3d {
                width: 1,
                height: 1,
                depth_or_array_layers: 1,
            },
        );
        
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("Default Sampler"),
            address_mode_u: wgpu::AddressMode::Repeat,
            address_mode_v: wgpu::AddressMode::Repeat,
            address_mode_w: wgpu::AddressMode::Repeat,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });
        
        (RenderResource::new(texture), Arc::new(sampler))
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

        let mut depth_buffer = builder.create("mesh.depth", TextureDesc {
            label: Some("mesh depth buffer"),
            size: wgpu::Extent3d {
                width: width.max(1),
                height: height.max(1),
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Depth32Float,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });
        
        let view_uniform = builder.create("mesh.camera_uniform", wgpu::BufferDescriptor {
            label: Some("Camera Uniform Buffer"),
            size: size_of::<[[f32; 4]; 4]>() as wgpu::BufferAddress,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let model_uniform = builder.create("mesh.model_uniform", wgpu::BufferDescriptor {
            label: Some("Model Uniform Buffer"),
            size: (size_of::<[[f32; 4]; 4]>() + size_of::<[f32; 3]>() + 4) as wgpu::BufferAddress,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        
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
            (vb, ib, mesh.index_count, mesh.material_index)
        }).collect();
        
        // Import default texture
        let default_texture = builder.import(
            "default_texture",
            self.default_texture.clone(),
            wgpu::TextureUses::empty()
        );
        
        // Import material textures (samplers will be handled directly in execute)
        let material_textures: Vec<_> = self.materials.iter().enumerate().map(|(i, material)| {
            if let Some(texture) = &material.base_color_texture {
                Some(builder.import(
                    &format!("material_texture_{}", i),
                    texture.clone(),
                    wgpu::TextureUses::empty()
                ))
            } else {
                None
            }
        }).collect();

        {
            let mut node = builder.add_graphic_node("mesh_render");

            let view_uniform = node.read(&view_uniform, wgpu::BufferUses::UNIFORM);
            let model_uniform = node.read(&model_uniform, wgpu::BufferUses::UNIFORM);
            let output = node.write(&mut output, wgpu::TextureUses::COLOR_TARGET);
            let depth_buffer = node.write(&mut depth_buffer, wgpu::TextureUses::DEPTH_STENCIL_WRITE);

            let mesh_reads: Vec<_> = mesh_resources.iter().map(|(vb, ib, _, _)| {
                let vb_read = node.read(&vb, wgpu::BufferUses::VERTEX);
                let ib_read = node.read(&ib, wgpu::BufferUses::INDEX);
                (vb_read, ib_read)
            }).collect();
            
            let default_texture_read = node.read(&default_texture, wgpu::TextureUses::RESOURCE);
            
            let material_texture_reads: Vec<_> = material_textures.iter().map(|texture| {
                if let Some(texture) = texture {
                    Some(node.read(texture, wgpu::TextureUses::RESOURCE))
                } else {
                    None
                }
            }).collect();

            node.setup_pipeline()
                .with_shader(self.shader.clone())
                .with_color(output, Default::default())
                .with_depth_stencil(depth_buffer, DepthStencilInfo {
                    depth_write: true,
                    compare: wgpu::CompareFunction::Greater,
                    stencil: wgpu::StencilState::default(),
                    bias: wgpu::DepthBiasState::default(),
                    depth_load_op: wgpu::LoadOp::Clear(0.0),
                    depth_store_op: wgpu::StoreOp::Store,
                    stencil_load_op: wgpu::LoadOp::Clear(0),
                    stencil_store_op: wgpu::StoreOp::Discard,
                });

            let view_proj = proj_matrix * view_matrix;
            let base_color = self.base_color.into();
            let materials_data: Vec<_> = self.materials.iter().map(|m| m.base_color_sampler.clone()).collect();
            let default_sampler_clone = self.default_sampler.clone();

            node.execute(move |ctx, encoder| {
                let view_uniform_data = zenith_build::mesh::ViewUniforms::new(view_proj);
                ctx.write_buffer(&view_uniform, 0, view_uniform_data);
                let model_uniform_data = zenith_build::mesh::ModelUniforms::new(model_matrix, base_color);
                ctx.write_buffer(&model_uniform, 0, model_uniform_data);

                let view_buffer = ctx.get_buffer(&view_uniform);
                let model_buffer = ctx.get_buffer(&model_uniform);

                let mut render_pass = ctx.begin_render_pass(encoder);
                
                for ((vb_read, ib_read), (_, _, index_count, material_index)) in mesh_reads.iter().zip(mesh_resources.iter()) {
                    let vertex_buffer = ctx.get_buffer(vb_read);
                    let index_buffer = ctx.get_buffer(ib_read);
                    
                    // Determine which texture and sampler to use
                    let (texture_binding, sampler_ref) = if let Some(mat_idx) = material_index {
                        if let Some(sampler) = materials_data.get(*mat_idx) {
                            if let Some(texture_read) = material_texture_reads.get(*mat_idx).and_then(|t| t.as_ref()) {
                                let texture = ctx.get_texture(texture_read);
                                (texture, sampler.clone())
                            } else {
                                let default_texture = ctx.get_texture(&default_texture_read);
                                (default_texture, default_sampler_clone.clone())
                            }
                        } else {
                            let default_texture = ctx.get_texture(&default_texture_read);
                            (default_texture, default_sampler_clone.clone())
                        }
                    } else {
                        let default_texture = ctx.get_texture(&default_texture_read);
                        (default_texture, default_sampler_clone.clone())
                    };
                    
                    // Create texture view
                    let texture_view = texture_binding.create_view(&wgpu::TextureViewDescriptor::default());
                    
                    // Bind all resources for this mesh
                    ctx.bind_pipeline(&mut render_pass)
                        .with_binding(0, 0, view_buffer.as_entire_binding())
                        .with_binding(0, 1, model_buffer.as_entire_binding())
                        .with_binding(0, 2, wgpu::BindingResource::TextureView(&texture_view))
                        .with_binding(0, 3, wgpu::BindingResource::Sampler(&*sampler_ref))
                        .bind();

                    render_pass.set_vertex_buffer(0, vertex_buffer.slice(..));
                    render_pass.set_index_buffer(index_buffer.slice(..), wgpu::IndexFormat::Uint32);
                    render_pass.draw_indexed(0..*index_count, 0, 0..1);
                }
            });
        }

        output
    }
} 