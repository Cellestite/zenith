use crate::main_loop::EngineLoop;

mod engine;
mod main_loop;
mod app;

pub use app::{App, RenderableApp};
pub use engine::Engine;

// zenith-core
pub use zenith_core::*;

// zenith-task
pub use zenith_task::*;

// zenith-render
pub use zenith_render::*;

// zenith-renderer
pub use zenith_renderer::*;

// zenith-rendergraph
pub use zenith_rendergraph::*;

pub async fn launch<A: RenderableApp>() -> Result<EngineLoop<A>, anyhow::Error> {
    let main_loop = EngineLoop::new()?;
    Ok(main_loop)
}
