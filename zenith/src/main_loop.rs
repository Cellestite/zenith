use std::num::NonZeroU32;
use std::sync::Arc;
use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::window::{WindowAttributes, WindowId};
use crate::app::RenderableApp;

struct AppEventForwarder<A: RenderableApp> {
    app: Option<A>,
}

impl<A: RenderableApp> AppEventForwarder<A> {
    fn new() -> Self {
        Self {
            app: None,
        }
    }

    #[inline]
    fn tick(&mut self) {
        if let Some(app) = &mut self.app {
            app.tick();
        }
    }
}

pub struct ZenithEngineLoop<A: RenderableApp> {
    event_loop: EventLoop<()>,
    app_event_forwarder: AppEventForwarder<A>
}

impl<A: RenderableApp> ZenithEngineLoop<A> {
    pub(super) fn new() -> Result<Self, anyhow::Error> {
        let event_loop = EventLoop::new()?;

        Ok(Self {
            event_loop,
            app_event_forwarder: AppEventForwarder::new(),
        })
    }

    pub fn run(self) -> Result<(), anyhow::Error> {
        let mut forwarder = self.app_event_forwarder;
        self.event_loop.run_app(&mut forwarder)?;
        Ok(())
    }
}

impl<A: RenderableApp> ApplicationHandler for AppEventForwarder<A> {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let window_attributes = WindowAttributes::default()
            .with_title("Zenith Engine")
            .with_inner_size(winit::dpi::LogicalSize::new(1280, 720))
            .with_resizable(true);

        let window = Arc::new(event_loop.create_window(window_attributes)
            .expect("Failed to create window."));

        let app = pollster::block_on(A::init(window))
            .expect("Failed to initialize zenith application!");
        self.app = Some(app);
    }

    fn window_event(&mut self,
                    event_loop: &ActiveEventLoop,
                    _window_id: WindowId,
                    event: WindowEvent
    ) {
        self.tick();

        match event {
            WindowEvent::Resized(new_size) => {
                // Safety: manually clamp to non-zero u32
                let (width, height) = unsafe {
                    (
                        NonZeroU32::new_unchecked(new_size.width.max(1)),
                        NonZeroU32::new_unchecked(new_size.height.max(1))
                    )
                };

                if let Some(app) = &mut self.app {
                    app.resize(width, height);
                }
            }
            WindowEvent::CloseRequested => {
                event_loop.exit();
            }
            WindowEvent::RedrawRequested => {
                if let Some(app) = &mut self.app {
                    app.render();
                }
            }
            _ => {}
        }
    }
}