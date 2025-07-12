use std::future::Future;
use zenith_rendergraph::{RenderGraphBuilder, RenderGraphResource, Texture};
use crate::Engine;

pub trait App: Sized + 'static {
    fn init(engine: &mut Engine) -> impl Future<Output = Result<Self, anyhow::Error>> + Send;
    fn update(&mut self, _delta_time: f32) {}
}

pub trait RenderableApp: App {
    fn resize(&mut self, _width: u32, _height: u32) {}
    fn render(&mut self, builder: &mut RenderGraphBuilder) -> Option<RenderGraphResource<Texture>>;
}