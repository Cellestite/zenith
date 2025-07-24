use std::sync::Arc;
use winit::window::Window;
use zenith_render::{RenderDevice, PipelineCache};
use zenith_rendergraph::{RenderGraphBuilder, SharedRenderGraphResource, TextureState};
use crate::RenderableApp;

pub struct Engine {
    pub render_device: RenderDevice,
    pipeline_cache: PipelineCache,

    pub main_window: Arc<Window>,
}

impl Engine {
    pub async fn new(main_window: Arc<Window>) -> Result<Self, anyhow::Error> {
        let render_device = RenderDevice::new(main_window.clone()).await?;
        let pipeline_cache = PipelineCache::new();

        Ok(Self {
            render_device,
            pipeline_cache,

            main_window,
        })
    }

    pub fn tick(&mut self, _delta_time: f32) {
    }

    pub fn render<A: RenderableApp>(&mut self, app: &mut A) {
        let device = self.render_device.device();
        let queue = self.render_device.queue();

        let mut builder = RenderGraphBuilder::new();

        let app_output_tex = app.render(&mut builder);

        if app_output_tex.is_some() {
            let surface_tex = self.render_device.acquire_next_frame();
            let swapchain_tex = SharedRenderGraphResource::new(surface_tex.texture.clone());
            let app_output_tex = app_output_tex.unwrap();

            {
                let mut swapchain_tex = builder.import("swapchain.output", swapchain_tex.clone(), wgpu::TextureUses::PRESENT);

                let mut node = builder.add_lambda_node("copy_output_to_swapchain");

                let app_output_tex = node.read(&app_output_tex, TextureState::COPY_SRC);
                let swapchain_tex = node.write(&mut swapchain_tex, TextureState::COPY_DST);

                node.execute(move |ctx, encoder| {
                    let src = ctx.get_texture(&app_output_tex);
                    let dst = ctx.get_texture(&swapchain_tex);

                    let width = dst.width();
                    let height = dst.height();

                    encoder.copy_texture_to_texture(
                        wgpu::TexelCopyTextureInfo {
                            texture: &src,
                            mip_level: 0,
                            origin: Default::default(),
                            aspect: Default::default(),
                        },
                        wgpu::TexelCopyTextureInfo {
                            texture: &dst,
                            mip_level: 0,
                            origin: Default::default(),
                            aspect: Default::default(),
                        },
                        wgpu::Extent3d {
                            width,
                            height,
                            depth_or_array_layers: 1,
                        }
                    );
                });
            }

            let graph = builder.build(device);
            let graph = graph.compile(device, &mut self.pipeline_cache);
            let graph = graph.execute(device, queue);

            self.main_window.pre_present_notify();
            graph.present(surface_tex).unwrap();
            self.main_window.request_redraw();
        }
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        self.render_device.resize(width, height);
    }
}