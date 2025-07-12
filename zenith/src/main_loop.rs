use std::sync::Arc;
use log::{debug, info};
use winit::application::ApplicationHandler;
use winit::event::{StartCause, WindowEvent};
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::window::{WindowAttributes, WindowId};
use crate::app::RenderableApp;
use crate::Engine;

struct EventForwarder<A: RenderableApp> {
    engine: Option<Engine>,
    app: Option<A>,

    is_initializing: bool,
}

impl<A: RenderableApp> EventForwarder<A> {
    fn new() -> Self {
        Self {
            engine: None,
            app: None,

            is_initializing: false,
        }
    }
}

pub struct ZenithEngineLoop<A: RenderableApp> {
    event_loop: EventLoop<()>,
    event_forwarder: EventForwarder<A>
}

impl<A: RenderableApp> ZenithEngineLoop<A> {
    pub(super) fn new() -> Result<Self, anyhow::Error> {
        Ok(Self {
            event_loop: EventLoop::new()?,
            event_forwarder: EventForwarder::new(),
        })
    }

    pub fn run(self) -> Result<(), anyhow::Error> {
        let mut forwarder = self.event_forwarder;
        self.event_loop.run_app(&mut forwarder)?;
        Ok(())
    }
}

impl<A: RenderableApp> ApplicationHandler for EventForwarder<A> {
    fn new_events(&mut self, _: &ActiveEventLoop, cause: StartCause) {
        if let StartCause::Init = cause {
            self.is_initializing = true;
        } else {
            self.is_initializing = false;
        }
    }

    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let window_attributes = WindowAttributes::default()
            .with_title("Zenith Engine")
            .with_inner_size(winit::dpi::LogicalSize::new(1280, 720))
            .with_min_inner_size(winit::dpi::LogicalSize::new(64, 64))
            .with_resizable(true);

        let window = Arc::new(event_loop.create_window(window_attributes)
            .expect("Failed to create window."));

        let (engine, app) = pollster::block_on(async {
            let mut engine = Engine::init(window.clone()).await
                .expect("Failed to initialize zenith engine.");

            let mut app = A::init(&mut engine).await
                .expect("Failed to initialize zenith application.");
            app.resize(window.inner_size().width, window.inner_size().height);

            (engine, app)
        });

        self.engine = Some(engine);
        self.app = Some(app);
        info!("Engine init successfully!");
    }

    fn window_event(&mut self,
                    event_loop: &ActiveEventLoop,
                    _window_id: WindowId,
                    event: WindowEvent
    ) {
        match event {
            WindowEvent::Resized(new_size) => {
                if self.is_initializing {
                   return;
                }

                debug!("system event resize: {}x{}", new_size.width, new_size.height);
                let engine = self.engine.as_mut().unwrap();

                engine.resize(new_size.width, new_size.height);
                self.app.as_mut().unwrap().resize(new_size.width, new_size.height);
                engine.main_window.request_redraw();
            }
            WindowEvent::CloseRequested => {
                event_loop.exit();
            }
            WindowEvent::RedrawRequested => {
                let app = self.app.as_mut().unwrap();
                self.engine.as_mut().unwrap().render(app);
            }
            _ => {
                self.engine.as_mut().unwrap().update(0.0);
                self.app.as_mut().unwrap().update(0.0);
            }
        }
    }
}