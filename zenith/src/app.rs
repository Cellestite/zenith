use std::future::Future;
use std::sync::Arc;
use glam::Vec2;
use winit::event::MouseButton;
use winit::window::Window;
use zenith_render::RenderDevice;
use zenith_rendergraph::{RenderGraphBuilder, RenderGraphResource, Texture};

pub trait App: Sized + 'static {
    fn new() -> impl Future<Output = Result<Self, anyhow::Error>>;
    fn on_key_input(&mut self, _name: Option<&str>, _is_pressed: bool) {}
    fn on_mouse_input(&mut self, _button: &MouseButton, _is_pressed: bool) {}
    fn on_mouse_moved(&mut self, _delta: &Vec2) {}
    fn tick(&mut self, _delta_time: f32) {}
}

pub trait RenderableApp: App {
    fn prepare(&mut self, render_device: &mut RenderDevice, main_window: Arc<Window>) -> Result<(), anyhow::Error>;
    fn resize(&mut self, _width: u32, _height: u32) {}
    fn render(&mut self, builder: &mut RenderGraphBuilder) -> Option<RenderGraphResource<Texture>>;
}