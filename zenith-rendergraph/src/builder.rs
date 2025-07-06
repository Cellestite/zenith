use std::cell::Cell;
use std::marker::PhantomData;
use std::sync::Arc;
use log::warn;
use crate::node::{NodePipelineState, RenderGraphNode};
use crate::graph::{NodeExecutionContext, RenderGraph, ResourceStorage};
use crate::node::{ColorInfo, DepthStencilInfo};
use crate::interface::{GraphResourceAccess, ResourceDescriptor, Texture};
use crate::resource::{ExportResourceStorage, ExportedRenderGraphResource, GraphImportExportResource, GraphResource, GraphResourceDescriptor, GraphResourceId, GraphResourceMutability, InitialResourceStorage, ReadOnly, ReadWrite, RenderGraphResource, RenderGraphResourceAccess};
use zenith_render::GraphicShader;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ResourceAccessStorage<V: GraphResourceMutability> {
    pub(crate) id: GraphResourceId,
    pub(crate) access: GraphResourceAccess,
    pub(crate) _marker: PhantomData<V>,
}

#[derive(Default)]
pub struct RenderGraphBuilder {
    nodes: Vec<RenderGraphNode>,
    pub(crate) initial_resources: Vec<InitialResourceStorage>,
    #[allow(dead_code)]
    pub(crate) export_resources: Vec<ExportResourceStorage>,
}

impl RenderGraphBuilder {
    pub fn new() -> Self {
        Self {
            ..Default::default()
        }
    }

    #[must_use]
    pub fn create<D: GraphResourceDescriptor>(
        &mut self,
        name: &str,
        desc: D,
    ) -> RenderGraphResource<D::Resource> {
        let id = self.initial_resources.len() as u32;
        let desc: ResourceDescriptor = desc.into();

        match desc {
            ResourceDescriptor::Buffer(desc) => {
                self.initial_resources.push((name.to_owned(), desc).into());
            }
            ResourceDescriptor::Texture(desc) => {
                self.initial_resources.push((name.to_owned(), desc).into());
            }
        }

        RenderGraphResource {
            id,
            _marker: PhantomData,
        }
    }

    #[must_use]
    pub fn import<R: GraphImportExportResource>(
        &mut self,
        name: &str,
        import_resource: Arc<R>,
        access: impl Into<GraphResourceAccess>,
    ) -> RenderGraphResource<R> {
        GraphImportExportResource::import(import_resource, name, self, access)
    }

    #[must_use]
    pub fn export<R: GraphImportExportResource>(
        &mut self,
        resource: RenderGraphResource<R>,
        access: impl Into<GraphResourceAccess>,
    ) -> ExportedRenderGraphResource<R> {
        GraphImportExportResource::export(resource, self, access)
    }

    #[must_use]
    pub fn add_graphic_node(&mut self, name: &str) -> GraphicNodeBuilder {
        let index = self.nodes.len();

        self.nodes.push(RenderGraphNode {
            name: name.to_string(),
            inputs: vec![],
            outputs: vec![],
            record_command_func: None,
            pipeline_state: NodePipelineState::Graphic(Default::default()),
        });

        GraphicNodeBuilder {
            node: &mut self.nodes[index],
            resources: &self.initial_resources,
        }
    }

    // #[must_use]
    // pub fn add_compute_node(&mut self, name: &str) -> GraphComputeNodeBuilder {
    //     let index = self.nodes.len();
    //     self.nodes.push(RenderGraphNode {
    //         node_name: name.to_string(),
    //         ..Default::default()
    //     });
    //
    //     GraphComputeNodeBuilder {
    //         node: &mut self.nodes[index]
    //     }
    // }

    pub fn build(self, device: &wgpu::Device) -> RenderGraph {
        let resources = self.initial_resources
            .into_iter()
            .map(|res| {
                match res {
                    InitialResourceStorage::ManagedBuffer(name, desc) => {
                        let buffer = device.create_buffer(&desc);
                        ResourceStorage::ManagedBuffer {
                            name,
                            resource: buffer,
                            state_tracker: Cell::new(wgpu::BufferUses::empty()).into()
                        }
                    }
                    InitialResourceStorage::ManagedTexture(name, desc) => {
                        let tex = device.create_texture(&desc);
                        ResourceStorage::ManagedTexture {
                            name,
                            resource: tex,
                            state_tracker: Cell::new(wgpu::TextureUses::UNINITIALIZED).into()
                        }
                    }
                    InitialResourceStorage::ImportedBuffer(name, buffer, init_access) => ResourceStorage::ImportedBuffer {
                        name,
                        resource: buffer,
                        state_tracker: Cell::new(init_access).into(),
                    },
                    InitialResourceStorage::ImportedTexture(name, tex, init_access) => ResourceStorage::ImportedTexture {
                        name,
                        resource: tex,
                        state_tracker: Cell::new(init_access).into(),
                    },
                }
            })
            .collect();

        RenderGraph {
            nodes: self.nodes,
            resources
        }
    }
}


pub struct GraphicNodeBuilder<'node, 'res> {
    node: &'node mut RenderGraphNode,
    resources: &'res Vec<InitialResourceStorage>,
}

impl<'node, 'res> GraphicNodeBuilder<'node, 'res> {
    #[must_use]
    pub fn read<R: GraphResource>(
        &mut self,
        resource: RenderGraphResource<R>,
        access: impl Into<GraphResourceAccess>
    ) -> RenderGraphResourceAccess<R, ReadOnly> {
        let access = RenderGraphResourceAccess {
            id: resource.id,
            access: access.into(),
            _marker: PhantomData,
        };

        if let None = self.node.inputs.iter().find(|h| h.id == resource.id) {
            self.node.inputs.push(access.clone().into_untyped());
        } else {
            let name = self.resources
                .get(resource.id as usize)
                .expect("Graph resource id out of bound!")
                .name();

            warn!("Try to read resource[{name}] multiple time!")
        }

        access
    }

    #[must_use]
    pub fn write<R: GraphResource>(
        &mut self,
        resource: RenderGraphResource<R>,
        access: impl Into<GraphResourceAccess>,
    ) -> RenderGraphResourceAccess<R, ReadWrite>  {
        let access = RenderGraphResourceAccess {
            id: resource.id,
            access: access.into(),
            _marker: PhantomData,
        };

        if let None = self.node.outputs.iter().find(|h| h.id == resource.id) {
            self.node.outputs.push(access.clone().into_untyped());
        } else {
            let name = self.resources
                .get(resource.id as usize)
                .expect("Graph resource id out of bound!")
                .name();

            warn!("Try to write to resource[{name}] multiple time!")
        }

        access
    }

    #[inline]
    #[must_use]
    pub fn setup_pipeline(&mut self) -> GraphicPipelineBuilder {
        GraphicPipelineBuilder {
            node: self.node
        }
    }

    pub fn record_command<F>(&mut self, record_command_func: F)
    where
        F: FnOnce(&mut NodeExecutionContext) + 'static
    {
        self.node.record_command_func.replace(Box::new(record_command_func));
    }
}

pub struct GraphicPipelineBuilder<'a> {
    node: &'a mut RenderGraphNode,
}

impl<'a> GraphicPipelineBuilder<'a> {
    pub fn with_shader(self, shader: Arc<GraphicShader>) -> Self {
        if let NodePipelineState::Graphic(pipeline) = &mut self.node.pipeline_state {
            pipeline.shader = Some(shader);
            self
        } else {
            panic!("Try to attach raster shader to a non-graphic pipeline!")
        }
    }

    pub fn with_color(self, color: RenderGraphResourceAccess<Texture, ReadWrite>, color_info: ColorInfo) -> Self {
        if let NodePipelineState::Graphic(pipeline) = &mut self.node.pipeline_state {
            pipeline.color_attachments.push((color, color_info));
            self
        } else {
            panic!("Try to add color attachment to a non-graphic pipeline!")
        }
    }

    pub fn with_depth_stencil(self, depth_stencil: RenderGraphResourceAccess<Texture, ReadWrite>, depth_stencil_info: DepthStencilInfo) -> Self {
        if let NodePipelineState::Graphic(pipeline) = &mut self.node.pipeline_state {
            pipeline.depth_stencil_attachment = Some((depth_stencil, depth_stencil_info));
            self
        } else {
            panic!("Try to add depth stencil attachment to a non-graphic pipeline!")
        }
    }

    pub fn with_binding<R: GraphResource, V: GraphResourceMutability>(self, binding: u32, color: RenderGraphResourceAccess<R, V>) -> Self {
        if let NodePipelineState::Graphic(pipeline) = &mut self.node.pipeline_state {
            pipeline.bindings.push((binding, color.id));
            self
        } else {
            panic!("Try to add color attachment to a non-graphic pipeline!")
        }
    }
}
