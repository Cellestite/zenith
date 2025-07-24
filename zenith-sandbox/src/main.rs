use std::sync::{Arc, Weak};
use winit::window::Window;
use zenith::{block_on, launch, App, RenderDevice, RenderGraphBuilder, RenderGraphResource, RenderableApp, Texture, TriangleRenderer};

pub struct TriangleApp {
    window: Option<Weak<Window>>,
    renderer: Option<TriangleRenderer>,
}

impl App for TriangleApp {
    async fn new() -> Result<Self, anyhow::Error> {
        Ok(Self {
            window: None,
            renderer: None,
        })
    }
}

impl RenderableApp for TriangleApp {
    fn prepare(&mut self, render_device: &mut RenderDevice, main_window: Arc<Window>) -> Result<(), anyhow::Error> {
        let triangle_renderer = TriangleRenderer::new(&render_device);

        self.window = Some(Arc::downgrade(&main_window));
        self.renderer = Some(triangle_renderer);
        Ok(())
    }

    fn render(&mut self, builder: &mut RenderGraphBuilder) -> Option<RenderGraphResource<Texture>> {
        let (width, height) = if let Some(window) = self.window.as_ref().and_then(|window| window.upgrade()) {
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