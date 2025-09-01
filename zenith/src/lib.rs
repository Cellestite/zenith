use crate::main_loop::EngineLoop;

mod engine;
mod main_loop;
mod app;

pub use app::{App, RenderableApp};
pub use engine::Engine;

pub use paste::paste;

macro_rules! module_facade {
    ($name:ident) => {
        $crate::paste!{
            pub mod $name {
                pub use [<zenith_ $name>]::*;
            }
        }
    };
}

module_facade!(core);
module_facade!(asset);
module_facade!(task);
module_facade!(render);
module_facade!(renderer);
module_facade!(rendergraph);

pub async fn launch<A: RenderableApp>() -> Result<EngineLoop<A>, anyhow::Error> {
    let main_loop = EngineLoop::new()?;
    Ok(main_loop)
}
