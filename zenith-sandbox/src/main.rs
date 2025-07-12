use std::sync::{Arc, Weak};
use log::info;
use winit::window::Window;
use zenith::{launch, App, Engine, RenderableApp};
use zenith_renderer::TriangleRenderer;
use zenith_rendergraph::{RenderGraphBuilder, RenderGraphResource, Texture};

pub struct TriangleApp {
    window: Weak<Window>,
    triangle_renderer: TriangleRenderer,
}

impl App for TriangleApp {
    async fn init(engine: &mut Engine) -> Result<Self, anyhow::Error> {
        let triangle_renderer = TriangleRenderer::new(&engine.render_device);

        info!("App init successfully!");

        Ok(Self {
            window: Arc::downgrade(&engine.main_window),
            triangle_renderer,
        })
    }
}

impl RenderableApp for TriangleApp {
    fn render(&mut self, builder: &mut RenderGraphBuilder) -> Option<RenderGraphResource<Texture>> {
        let (width, height) = if let Some(window) = self.window.upgrade() {
            (window.inner_size().width, window.inner_size().height)
        } else {
            return None;
        };

        if width > 0 && height > 0 {
            Some(self.triangle_renderer.build_render_graph(builder, width, height))
        } else {
            None
        }
    }
}

fn main() {
    let engine_loop = pollster::block_on(launch::<TriangleApp>())
        .expect("Failed to create zenith engine loop!");

    engine_loop
        .run()
        .expect("Failed to run zenith engine loop!");
}