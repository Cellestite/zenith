use std::hash::{Hash, Hasher};
use zenith_core::collections::{DefaultHasher, HashMap};
use crate::shader::GraphicShader;

pub struct PipelineCache {
    raster_pipelines: HashMap<u64, wgpu::RenderPipeline>,
}

impl PipelineCache {
    pub fn new() -> Self {
        Self {
            raster_pipelines: HashMap::new(),
        }
    }

    pub fn get_or_create_graphic_pipeline(
        &mut self,
        device: &wgpu::Device,
        shader: &GraphicShader,
        pipeline_layout: &wgpu::PipelineLayout,
        color_attachments: &[Option<wgpu::ColorTargetState>],
        depth_stencil: Option<wgpu::DepthStencilState>,
    ) -> wgpu::RenderPipeline {
        let mut hasher = DefaultHasher::new();
        shader.hash(&mut hasher);
        let hash = hasher.finish();

        self.raster_pipelines.entry(hash)
            .or_insert_with(|| {
                let shader_module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
                    label: Some(&shader.name),
                    source: wgpu::ShaderSource::Wgsl(shader.source.clone().into()),
                });

                let pipeline = device.create_render_pipeline(
                    &wgpu::RenderPipelineDescriptor {
                        label: Some(&shader.name),
                        layout: Some(pipeline_layout),
                        vertex: wgpu::VertexState {
                            module: &shader_module,
                            entry_point: Some(&shader.vertex_entry),
                            compilation_options: Default::default(),
                            buffers: &[shader.vertex_layout.build_as()],
                        },
                        primitive: Default::default(),
                        depth_stencil,
                        multisample: Default::default(),
                        fragment: shader.fragment_entry.as_ref().map(|entry| {
                            wgpu::FragmentState {
                                module: &shader_module,
                                entry_point: Some(entry),
                                targets: &color_attachments,
                                compilation_options: Default::default(),
                            }
                        }),
                        multiview: None,
                        cache: None,
                    }
                );
                pipeline
            })
            .clone()
    }
}
