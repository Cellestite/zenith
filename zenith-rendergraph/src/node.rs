use std::sync::Arc;
use derive_builder::Builder;
use zenith_render::GraphicShader;
use crate::graph::{GraphicNodeExecutionContext, LambdaNodeExecutionContext};
use crate::builder::{ResourceAccessStorage};
use crate::interface::Texture;
use crate::resource::{RenderGraphResourceAccess, Rt};

#[derive(Default, Debug, Builder)]
#[builder(setter(into))]
pub struct ColorInfo {
    #[builder(default)]
    pub blend: Option<wgpu::BlendState>,
    #[builder(default)]
    pub write_mask: Option<wgpu::ColorWrites>,
    #[builder(default)]
    pub load_op: wgpu::LoadOp<wgpu::Color>,
    #[builder(default)]
    pub store_op: wgpu::StoreOp,
}

#[derive(Debug, Builder)]
#[builder(setter(into))]
pub struct DepthStencilInfo {
    #[builder(default)]
    pub depth_write: bool,
    #[builder(default="wgpu::CompareFunction::Always")]
    pub compare: wgpu::CompareFunction,
    #[builder(default)]
    pub stencil: wgpu::StencilState,
    #[builder(default)]
    pub bias: wgpu::DepthBiasState,
}

#[derive(Default)]
pub struct GraphicPipelineDescriptor {
    pub(crate) shader: Option<Arc<GraphicShader>>,
    pub(crate) color_attachments: Vec<(RenderGraphResourceAccess<Texture, Rt>, ColorInfo)>,
    pub(crate) depth_stencil_attachment: Option<(RenderGraphResourceAccess<Texture, Rt>, DepthStencilInfo)>,
}

impl GraphicPipelineDescriptor {
    pub fn name(&self) -> &str {
        self
            .shader
            .as_ref()
            .map(|shader| shader.name())
            .unwrap_or("Unknown")
    }

    pub fn valid(&self) -> bool {
        self.shader.is_some() && !self.color_attachments.is_empty()
    }
}

#[derive(Default, Debug)]
pub struct ComputePipelineDescriptor {
}

impl ComputePipelineDescriptor {
    #[allow(dead_code)]
    pub fn name(&self) -> &str {
        "Unknown"
    }

    pub fn valid(&self) -> bool {
        false
    }
}

pub(crate) enum NodePipelineState {
    Graphic {
        pipeline_desc: GraphicPipelineDescriptor,
        job_functor: Option<Box<dyn FnOnce(&mut GraphicNodeExecutionContext, &mut wgpu::CommandEncoder)>>,
    },
    #[allow(dead_code)]
    Compute {
        pipeline_desc: ComputePipelineDescriptor,
        job_functor: Option<Box<dyn FnOnce(&mut GraphicNodeExecutionContext, &mut wgpu::CommandEncoder)>>,
    },
    Lambda {
        job_functor: Option<Box<dyn FnOnce(&mut LambdaNodeExecutionContext, &mut wgpu::CommandEncoder)>>,
    }
}

impl NodePipelineState {
    pub(crate) fn valid(&self) -> bool {
        match self {
            NodePipelineState::Graphic { pipeline_desc, job_functor } => {
                pipeline_desc.valid() && job_functor.is_some()
            }
            NodePipelineState::Compute { pipeline_desc, job_functor } => {
                pipeline_desc.valid() && job_functor.is_some()
            }
            NodePipelineState::Lambda { job_functor } => {
                job_functor.is_some()
            }
        }
    }
}

pub struct RenderGraphNode {
    // TODO: debug only
    #[allow(dead_code)]
    pub(crate) name: String,
    pub(crate) inputs: Vec<ResourceAccessStorage>,
    pub(crate) outputs: Vec<ResourceAccessStorage>,

    pub(crate) pipeline_state: NodePipelineState,
}

impl RenderGraphNode {
    pub fn name(&self) -> &str {
        &self.name
    }
}