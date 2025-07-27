use std::future::Future;
use std::sync::Arc;
use winit::window::Window;
use zenith_core::system_event::SystemEventCollector;
use zenith_render::RenderDevice;
use zenith_rendergraph::{RenderGraphBuilder, RenderGraphResource, Texture};

pub trait App: Sized + 'static {
    fn new() -> impl Future<Output = Result<Self, anyhow::Error>>;
    fn process_event(&mut self, _collector: &SystemEventCollector) {}
    fn tick(&mut self, _delta_time: f32) {}
}

pub trait RenderableApp: App {
    fn prepare(&mut self, render_device: &mut RenderDevice, main_window: Arc<Window>) -> Result<(), anyhow::Error>;
    fn resize(&mut self, _width: u32, _height: u32) {}
    fn render(&mut self, builder: &mut RenderGraphBuilder) -> Option<RenderGraphResource<Texture>>;
}