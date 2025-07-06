use std::cell::{Cell, RefCell};
use std::sync::Arc;
use derive_more::From;
use zenith_core::collections::SmallVec;
use zenith_render::PipelineCache;
use crate::node::{NodePipelineState, RenderGraphNode};
use crate::interface::{Buffer, BufferState, GraphResourceAccess, Texture, TextureState};
use crate::RasterPipelineDescriptor;
use crate::resource::{GraphResourceId, GraphResourceMutability, GraphResourceState, RenderGraphResourceAccess};

pub(crate) enum ResourceStorage {
    ManagedBuffer {
        name: String,
        resource: Buffer,
        state_tracker: ResourceStateTracker<BufferState>
    },
    ManagedTexture {
        name: String,
        resource: Texture,
        state_tracker: ResourceStateTracker<TextureState>
    },
    ImportedBuffer {
        name: String,
        resource: Arc<Buffer>,
        state_tracker: ResourceStateTracker<BufferState>
    },
    ImportedTexture {
        name: String,
        resource: Arc<Texture>,
        state_tracker: ResourceStateTracker<TextureState>
    },
}

impl ResourceStorage {
    pub(crate) fn name(&self) -> &str {
        match self {
            ResourceStorage::ManagedBuffer { name, .. } => &name,
            ResourceStorage::ManagedTexture { name, .. } => &name,
            ResourceStorage::ImportedBuffer { name, .. } => &name,
            ResourceStorage::ImportedTexture { name, .. } => &name,
        }
    }
}

#[derive(From)]
pub(crate) struct ResourceStateTracker<T: GraphResourceState> {
    current_state: Cell<T>,
}

impl<T: GraphResourceState> ResourceStateTracker<T> {
    #[allow(dead_code)]
    pub(crate) fn current(&self) -> T {
        self.current_state.get()
    }

    pub(crate) fn should_transition_to(&self, next_state: T, skip_if_same: bool) -> bool {
        if skip_if_same {
            self.current_state.get() != next_state
        } else {
            true
        }
    }

    pub(crate) fn transition_to(&self, next_state: T) {
        self.current_state.set(next_state);
    }
}

/// ## TODO
/// Generalize it using derived macro (move to interface.rs)
enum Pipeline {
    Graphic(wgpu::RenderPipeline),
    #[allow(dead_code)]
    Compute(wgpu::ComputePipeline),
}

pub struct RenderGraph {
    pub(crate) nodes: Vec<RenderGraphNode>,
    pub(crate) resources: Vec<ResourceStorage>,
}

impl RenderGraph {
    pub fn validate(&self) {

    }

    pub fn compile(
        self,
        device: &wgpu::Device,
        pipeline_cache: &mut PipelineCache,
    ) -> CompiledRenderGraph {
        let mut pipelines = vec![];

        for node in &self.nodes {
            match &node.pipeline_state {
                NodePipelineState::Graphic(desc) => {
                    let pipeline = self.create_graphic_pipeline(device, pipeline_cache, desc);
                    pipelines.push(Pipeline::Graphic(pipeline));
                }
                NodePipelineState::Compute(_) => { unimplemented!() }
            }
        }

        CompiledRenderGraph {
            nodes: self.nodes,
            resources: self.resources,
            pipelines,
        }
    }

    fn create_graphic_pipeline(
        &self,
        device: &wgpu::Device,
        pipeline_cache: &mut PipelineCache,
        desc: &RasterPipelineDescriptor,
    ) -> wgpu::RenderPipeline {
        let bind_group_entries = desc.bindings
            .iter()
            .map(|(binding, id)| {
                let storage = utility::resource_storage_ref(&self.resources, *id);

                match storage {
                    ResourceStorage::ManagedBuffer { .. } |
                    ResourceStorage::ImportedBuffer { .. } => {
                        wgpu::BindGroupLayoutEntry {
                            binding: *binding,
                            visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                            ty: wgpu::BindingType::Buffer {
                                // TODO: uniform or readonly storage
                                ty: wgpu::BufferBindingType::Uniform,
                                has_dynamic_offset: false,
                                min_binding_size: None,
                            },
                            count: None,
                        }
                    }
                    ResourceStorage::ManagedTexture { .. } |
                    ResourceStorage::ImportedTexture { .. } => {
                        wgpu::BindGroupLayoutEntry {
                            binding: *binding,
                            visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                            ty: wgpu::BindingType::Texture {
                                sample_type: wgpu::TextureSampleType::Float {
                                    filterable: false,
                                },
                                view_dimension: wgpu::TextureViewDimension::D2,
                                multisampled: false,
                            },
                            count: None,
                        }
                    }
                }
            })
            .collect::<SmallVec<[wgpu::BindGroupLayoutEntry; 4]>>();

        let bind_group_layout = device.create_bind_group_layout(
            &wgpu::BindGroupLayoutDescriptor{
                label: Some(desc.name()),
                entries: &bind_group_entries
            }
        );

        let pipeline_layout = device.create_pipeline_layout(
            &wgpu::PipelineLayoutDescriptor {
                label: Some(desc.name()),
                bind_group_layouts: &[&bind_group_layout],
                push_constant_ranges: &[],
            }
        );

        let color_attachments = desc.color_attachments
            .iter()
            .map(|(resource, color_info)| {
                let storage = utility::resource_storage_ref(&self.resources, resource.id);

                match storage {
                    ResourceStorage::ManagedTexture { resource, .. } => {
                        wgpu::ColorTargetState {
                            format: resource.format(),
                            blend: color_info.blend,
                            write_mask: color_info.write_mask.unwrap_or(wgpu::ColorWrites::ALL),
                        }
                    }
                    ResourceStorage::ImportedTexture { resource, .. } => {
                        wgpu::ColorTargetState {
                            format: resource.format(),
                            blend: color_info.blend,
                            write_mask: color_info.write_mask.unwrap_or(wgpu::ColorWrites::ALL),
                        }
                    }
                    _ => unreachable!("Color attachment had bound to a non-texture resource!")
                }
            })
            .map(Some)
            .collect::<SmallVec<[Option<wgpu::ColorTargetState>; 8]>>();

        let depth_stencil_attachment = desc.depth_stencil_attachment
            .as_ref()
            .map(|(resource, depth)| {
                let storage = utility::resource_storage_ref(&self.resources, resource.id);

                match storage {
                    ResourceStorage::ManagedTexture { resource, .. } => {
                        wgpu::DepthStencilState {
                            format: resource.format(),
                            depth_write_enabled: depth.depth_write_enabled,
                            depth_compare: depth.compare,
                            stencil: depth.stencil.clone(),
                            bias: depth.bias,
                        }
                    }
                    ResourceStorage::ImportedTexture { resource, .. } => {
                        wgpu::DepthStencilState {
                            format: resource.format(),
                            depth_write_enabled: depth.depth_write_enabled,
                            depth_compare: depth.compare,
                            stencil: depth.stencil.clone(),
                            bias: depth.bias,
                        }
                    }
                    _ => unreachable!()
                }
            });

        let shader = desc
            .shader
            .as_ref()
            .expect("Missing raster shader for node...");

        pipeline_cache.get_or_create_graphic_pipeline(
            device,
            shader,
            &pipeline_layout,
            &color_attachments,
            depth_stencil_attachment)
    }
}


pub struct CompiledRenderGraph {
    nodes: Vec<RenderGraphNode>,
    resources: Vec<ResourceStorage>,
    pipelines: Vec<Pipeline>,
}

impl CompiledRenderGraph {
    pub fn execute(self, device: &wgpu::Device, queue: &wgpu::Queue) -> PresentableRenderGraph {
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("render graph main command encoder"),
        });

        for (index, mut node) in self.nodes.into_iter().enumerate() {
            Self::transition_resources(
                &mut encoder,
                &self.resources,
                node
                    .inputs
                    .iter()
                    .map(|access| (access.id, access.access))
                    .chain(node.outputs.iter().map(|access| (access.id, access.access)))
            );

            let render_pass = Self::begin_render_pass(
                &node,
                &mut encoder,
                &self.resources,
            );

            if let Pipeline::Graphic(pipeline) = self.pipelines.get(index).unwrap() {
                if let Some(record) = node.record_command_func.take() {
                    let mut ctx = NodeExecutionContext {
                        render_pass: RefCell::new(render_pass),
                        device,
                        queue,
                        resources: &self.resources,
                        pipeline: pipeline.clone(),
                    };
                    record(&mut ctx);
                }
            } else {
                unimplemented!();
            };
        }

        queue.submit(Some(encoder.finish()));

        PresentableRenderGraph {
        }
    }

    fn begin_render_pass<'a>(
        node: &RenderGraphNode,
        encoder: &'a mut wgpu::CommandEncoder,
        resources: &Vec<ResourceStorage>,
    ) -> wgpu::RenderPass<'a> {
        let create_texture_view = |id| {
            let storage = utility::resource_storage_ref(resources, id);

            match storage {
                ResourceStorage::ManagedTexture { resource, .. } => {
                    resource.create_view(&wgpu::TextureViewDescriptor::default())
                }
                ResourceStorage::ImportedTexture { resource, .. } => {
                    resource.create_view(&wgpu::TextureViewDescriptor::default())
                }
                _ => unreachable!()
            }
        };

        // TODO: use iterator-valid container
        let color_views = match &node.pipeline_state {
            NodePipelineState::Graphic(pipeline) => {
                pipeline.color_attachments
                    .iter()
                    .map(|(res, _)| res.id)
                    .map(create_texture_view)
                    .collect::<SmallVec<[wgpu::TextureView; 8]>>()
            }
            NodePipelineState::Compute(_) => unimplemented!()
        };
        let depth_view = match &node.pipeline_state {
            NodePipelineState::Graphic(pipeline) => {
                pipeline.depth_stencil_attachment
                    .as_ref()
                    .map(|(res, _)| res.id)
                    .map(create_texture_view)
            }
            NodePipelineState::Compute(_) => unimplemented!()
        };

        let (color_attachments, depth_stencil_attachment) = match &node.pipeline_state {
            NodePipelineState::Graphic(pipeline) => {
                (
                    pipeline.color_attachments
                        .iter()
                        .zip(color_views.iter())
                        .map(|(_, view)| {
                            Some(wgpu::RenderPassColorAttachment {
                                view,
                                resolve_target: None,
                                // TODO
                                ops: wgpu::Operations {
                                    load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                                    store: wgpu::StoreOp::Store,
                                },
                            })
                        })
                        .collect::<SmallVec<[Option<wgpu::RenderPassColorAttachment>; 8]>>(),
                    depth_view.as_ref().map(|view| {
                        wgpu::RenderPassDepthStencilAttachment {
                            view: &view,
                            depth_ops: None,
                            stencil_ops: None
                        }
                    })
                )
            }
            NodePipelineState::Compute(_) => unimplemented!()
        };

        encoder.begin_render_pass(
            &wgpu::RenderPassDescriptor {
                label: Some(node.name()),
                color_attachments: &color_attachments,
                depth_stencil_attachment,
                timestamp_writes: None,
                occlusion_query_set: None,
            }
        )
    }

    fn transition_resources(
        encoder: &mut wgpu::CommandEncoder,
        resources: &Vec<ResourceStorage>,
        resources_to_transition: impl Iterator<Item = (GraphResourceId, GraphResourceAccess)>,
    ) {
        let mut buffer_transitions: SmallVec<[wgpu::BufferTransition<&Buffer>; 8]> = SmallVec::new();
        let mut texture_transitions: SmallVec<[wgpu::TextureTransition<&Texture>; 8]> = SmallVec::new();

        let mut add_buffer_transition = |next_state, buffer, state_tracker: &ResourceStateTracker<BufferState>| {
            if state_tracker.should_transition_to(next_state, true) {
                buffer_transitions.push(wgpu::BufferTransition {
                    buffer,
                    state: next_state,
                });
                state_tracker.transition_to(next_state);
            }
        };

        let mut add_texture_transition = |next_state, texture, state_tracker: &ResourceStateTracker<TextureState>| {
            if state_tracker.should_transition_to(next_state, true) {
                texture_transitions.push(wgpu::TextureTransition {
                    texture,
                    selector: None,
                    state: next_state,
                });
                state_tracker.transition_to(next_state);
            }
        };

        for (id, access) in resources_to_transition {
            let storage = utility::resource_storage_ref(resources, id);

            match access {
                GraphResourceAccess::Buffer(next_state) => {
                    match storage {
                        ResourceStorage::ManagedBuffer { resource, state_tracker, .. } => {
                            add_buffer_transition(next_state, &*resource, state_tracker);
                        }
                        ResourceStorage::ImportedBuffer { resource, state_tracker, .. } => {
                            add_buffer_transition(next_state, &*resource, state_tracker);
                        }
                        _ =>  {
                            unreachable!("Resource[{}] is a texture, but a non-texture state[{:?}] is provided when read/write!", storage.name(), next_state)
                        }
                    }
                }
                GraphResourceAccess::Texture(next_state) => {
                    match storage {
                        ResourceStorage::ManagedTexture { resource, state_tracker, .. } => {
                            add_texture_transition(next_state, &*resource, state_tracker);
                        }
                        ResourceStorage::ImportedTexture { resource, state_tracker, .. } => {
                            add_texture_transition(next_state, &*resource, state_tracker);
                        }
                        _ => {
                            unreachable!("Resource[{}] is a buffer, but a non-buffer state[{:?}] is provided when read/write!", storage.name(), next_state)
                        }
                    }
                }
            }
        }

        encoder.transition_resources(
            buffer_transitions.into_iter(),
            texture_transitions.into_iter()
        );
    }
}

pub struct NodeExecutionContext<'encoder, 'device, 'queue, 'res> {
    pub render_pass: RefCell<wgpu::RenderPass<'encoder>>,
    device: &'device wgpu::Device,
    queue: &'queue wgpu::Queue,
    resources: &'res Vec<ResourceStorage>,
    pipeline: wgpu::RenderPipeline,
}

pub struct PipelineBinder<'ctx, 'encoder, 'device, 'queue, 'res> {
    context: &'ctx NodeExecutionContext<'encoder, 'device, 'queue, 'res>,
    bindings: Vec<wgpu::BindGroupEntry<'res>>,
}

impl<'ctx, 'encoder, 'device, 'queue, 'res> PipelineBinder<'ctx, 'encoder, 'device, 'queue, 'res> {
    pub fn with_binding(mut self, binding: u32, resource: wgpu::BindingResource<'res>) -> Self {
        self.bindings.push(wgpu::BindGroupEntry {
            binding,
            resource,
        });
        self
    }

    pub fn bind(self) {
        let layout = self.context.pipeline.get_bind_group_layout(0);
        let bind_group = self.context.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &layout,
            entries: &self.bindings,
        });

        self.context.render_pass.borrow_mut().set_bind_group(0, &bind_group, &[]);
    }
}

impl<'encoder, 'device, 'queue, 'res> NodeExecutionContext<'encoder, 'device, 'queue, 'res> {
    pub fn get_buffer<V: GraphResourceMutability>(&self, resource_access: &RenderGraphResourceAccess<Buffer, V>) -> &Buffer {
        match self.resources.get(resource_access.id as usize).expect("Graph resource index out of bound!") {
            ResourceStorage::ManagedBuffer { resource, .. } => {
                resource
            }
            ResourceStorage::ImportedBuffer { resource, .. } => {
                resource
            }
            _ => unreachable!("Expect buffer, but pass in a texture resource handle!")
        }
    }

    pub fn get_texture<V: GraphResourceMutability>(&self, resource_access: &RenderGraphResourceAccess<Texture, V>) -> &Texture {
        match self.resources.get(resource_access.id as usize).expect("Graph resource index out of bound!") {
            ResourceStorage::ManagedTexture { resource, .. } => {
                resource
            }
            ResourceStorage::ImportedTexture { resource, .. } => {
                resource
            }
            _ => unreachable!("Expect texture, but pass in a buffer resource handle!")
        }
    }

    pub fn write_buffer<V: GraphResourceMutability>(&self, resource_access: &RenderGraphResourceAccess<Buffer, V>, offset: wgpu::BufferAddress, data: &[u8]) {
        match self.resources.get(resource_access.id as usize).expect("Graph resource index out of bound!") {
            ResourceStorage::ManagedBuffer { resource, .. } => {
                self.queue.write_buffer(resource, offset, data);
            }
            ResourceStorage::ImportedBuffer { resource, .. } => {
                self.queue.write_buffer(resource, offset, data);
            }
            _ => unreachable!("Expect buffer, but pass in a texture resource handle!")
        }
    }

    pub fn bind_pipeline<'ctx>(&'ctx self) -> PipelineBinder<'ctx, 'encoder, 'device, 'queue, 'res> {
        self.render_pass.borrow_mut().set_pipeline(&self.pipeline);
        PipelineBinder {
            context: self,
            bindings: vec![],
        }
    }
}

pub struct PresentableRenderGraph {}

impl PresentableRenderGraph {
    pub fn present(self, present_surface: wgpu::SurfaceTexture) -> Result<(), Box<anyhow::Error>> {
        present_surface.present();

        Ok(())
    }
}

pub(crate) mod utility {
    use crate::graph::ResourceStorage;
    use crate::resource::GraphResourceId;

    #[inline]
    pub(crate) fn resource_storage_ref(storage: &Vec<ResourceStorage>, id: GraphResourceId) -> &ResourceStorage {
        storage.get(id as usize).expect("Graph resource id out of bound!")
    }
}