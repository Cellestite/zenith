use std::sync::Arc;
use zenith_render::{GraphicShader};
use crate::graph::NodeExecutionContext;
use crate::builder::{ResourceAccessStorage};
use crate::interface::Texture;
use crate::resource::{GraphResourceId, ReadOnly, ReadWrite, RenderGraphResourceAccess};

#[derive(Default)]
pub struct ColorInfo {
    pub blend: Option<wgpu::BlendState>,
    pub write_mask: Option<wgpu::ColorWrites>,
}

pub struct DepthStencilInfo {
    pub depth_write_enabled: bool,
    pub compare: wgpu::CompareFunction,
    pub stencil: wgpu::StencilState,
    pub bias: wgpu::DepthBiasState,
}

pub type BindingId = u32;

#[derive(Default)]
pub struct RasterPipelineDescriptor {
    pub(crate) shader: Option<Arc<GraphicShader>>,
    pub(crate) color_attachments: Vec<(RenderGraphResourceAccess<Texture, ReadWrite>, ColorInfo)>,
    pub(crate) depth_stencil_attachment: Option<(RenderGraphResourceAccess<Texture, ReadWrite>, DepthStencilInfo)>,
    pub(crate) bindings: Vec<(BindingId, GraphResourceId)>
}

impl RasterPipelineDescriptor {
    pub fn name(&self) -> &str {
        self
            .shader
            .as_ref()
            .map(|shader| shader.name.as_str())
            .unwrap_or("Unknown")
    }
}

#[derive(Default)]
pub struct ComputePipelineDescriptor {
}

pub(crate) enum NodePipelineState {
    Graphic(RasterPipelineDescriptor),
    #[allow(dead_code)]
    Compute(ComputePipelineDescriptor)
}

pub struct RenderGraphNode {
    // TODO: debug only
    #[allow(dead_code)]
    pub(crate) name: String,
    pub(crate) inputs: Vec<ResourceAccessStorage<ReadOnly>>,
    pub(crate) outputs: Vec<ResourceAccessStorage<ReadWrite>>,
    pub(crate) record_command_func: Option<Box<dyn FnOnce(&mut NodeExecutionContext)>>,

    pub(crate) pipeline_state: NodePipelineState,
}

impl RenderGraphNode {
    pub fn name(&self) -> &str {
        &self.name
    }
}