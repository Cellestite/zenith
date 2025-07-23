use std::sync::{Arc, Weak};
use winit::window::Window;
use zenith::{block_on, launch, App, RenderDevice, RenderGraphBuilder, RenderGraphResource, RenderableApp, Texture, TriangleRenderer};

pub struct TriangleApp {
    window: Weak<Window>,
    renderer: Option<TriangleRenderer>,
}

impl App for TriangleApp {
    async fn new(main_window: Arc<Window>) -> Result<Self, anyhow::Error> {
        Ok(Self {
            window: Arc::downgrade(&main_window),
            renderer: None,
        })
    }
}

impl RenderableApp for TriangleApp {
    fn prepare(&mut self, render_device: &mut RenderDevice) -> Result<(), anyhow::Error> {
        let triangle_renderer = TriangleRenderer::new(&render_device);

        self.renderer = Some(triangle_renderer);
        Ok(())
    }

    fn render(&mut self, builder: &mut RenderGraphBuilder) -> Option<RenderGraphResource<Texture>> {
        let (width, height) = if let Some(window) = self.window.upgrade() {
            (window.inner_size().width, window.inner_size().height)
        } else {
            return None;
        };

        if width > 0 && height > 0 {
            Some(self.renderer.as_ref().unwrap().build_render_graph(builder, width, height))
        } else {
            None
        }
    }
}

fn main() {
    let engine_loop = block_on(launch::<TriangleApp>())
        .expect("Failed to create zenith engine loop!");

    engine_loop
        .run()
        .expect("Failed to run zenith engine loop!");
}