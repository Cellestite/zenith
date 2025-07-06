use std::sync::Arc;
use winit::window::Window;
use zenith_render::{RenderDevice, PipelineCache};

pub struct Engine {
    pub render_device: RenderDevice,
    pub pipeline_cache: PipelineCache,
}

impl Engine {
    pub async fn init(window: Arc<Window>) -> Result<Self, anyhow::Error> {
        let render_device = RenderDevice::new(window).await?;
        let pipeline_cache = PipelineCache::new();

        Ok(Self {
            render_device,
            pipeline_cache,
        })
    }
}