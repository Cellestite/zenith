use crate::main_loop::ZenithEngineLoop;

mod app;
mod engine;
mod main_loop;

pub use app::{App, RenderableApp};
pub use engine::{Engine};
pub async fn launch<A: RenderableApp>() -> Result<ZenithEngineLoop<A>, anyhow::Error> {
    zenith_core::log::initialize()?;

    let main_loop = ZenithEngineLoop::new()?;
    Ok(main_loop)
}
