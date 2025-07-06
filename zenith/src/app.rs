use std::num::NonZeroU32;
use std::sync::Arc;
use winit::window::Window;
use zenith_renderer::TriangleRenderer;
use zenith_rendergraph::RenderGraphBuilder;
use crate::engine::Engine;

pub trait App: Sized + 'static {
    fn init(window: Arc<Window>) -> impl Future<Output = Result<Self, anyhow::Error>> + Send;
    fn tick(&mut self) {}
}

pub trait RenderableApp: App {
    fn resize(&mut self, _width: NonZeroU32, _height: NonZeroU32) {}
    fn render(&mut self) {}
}

pub struct ZenithDefaultApp {
    main_window: Arc<Window>,

    engine: Engine,
    triangle_renderer: TriangleRenderer,
}

impl App for ZenithDefaultApp {
    async fn init(window: Arc<Window>) -> Result<Self, anyhow::Error> {
        let engine = Engine::init(window.clone()).await.expect("Failed to create engine.");

        let triangle_renderer = TriangleRenderer::new(&engine.render_device);

        Ok(Self {
            main_window: window,
            engine,
            triangle_renderer,
        })
    }
}

impl RenderableApp for ZenithDefaultApp {
    fn resize(&mut self, width: NonZeroU32, height: NonZeroU32) {
        self.engine.render_device.resize(width, height);
    }

    fn render(&mut self) {
        let render_device = &self.engine.render_device;
        let device = render_device.device();
        let queue = render_device.queue();

        let mut builder = RenderGraphBuilder::new();

        let surface_tex = self.triangle_renderer.build_render_graph(&mut builder, &self.engine.render_device);

        let graph = builder.build(device);
        let graph = graph.compile(device, &mut self.engine.pipeline_cache);
        let graph = graph.execute(device, queue);

        self.main_window.pre_present_notify();
        graph.present(surface_tex).unwrap();
        self.main_window.request_redraw();
    }
}