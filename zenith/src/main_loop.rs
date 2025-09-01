use std::sync::Arc;
use std::time::Duration;
use anyhow::anyhow;
use log::info;
use winit::event::{WindowEvent};
use winit::event_loop::{EventLoop};
use winit::platform::pump_events::EventLoopExtPumpEvents;
use zenith_core::system_event::{SystemEventCollector, UserEvent};
use crate::app::{RenderableApp};
use crate::Engine;

pub struct EngineLoop<A> {
    event_loop: EventLoop<UserEvent>,
    engine: Engine,
    app: A,
}

impl<A: RenderableApp> EngineLoop<A> {
    pub(super) fn new() -> Result<Self, anyhow::Error> {
        zenith_task::initialize();
        zenith_core::log::initialize()?;
        zenith_asset::initialize()?;

        let mut app = smol::block_on(A::new())?;

        let mut event_loop = EventLoop::with_user_event().build()?;

        let proxy = event_loop.create_proxy();
        proxy.send_event(UserEvent::CreateWindow)
            .expect("Failed to send main window creation request!");

        let mut collector = SystemEventCollector::new();
        event_loop.pump_app_events(Some(Duration::ZERO), &mut collector);
        let SystemEventCollector { windows, .. } = collector;

        let main_window = Arc::new(windows
            .into_iter()
            .take(1)
            .next()
            .ok_or(anyhow!("Failed to create main window!"))?);

        let mut engine = smol::block_on(Engine::new(main_window.clone()))?;
        app.prepare(&mut engine.render_device, main_window.clone())?;

        Ok(Self {
            event_loop,
            engine,
            app,
        })
    }

    pub fn run(self) -> Result<(), anyhow::Error> {
        let mut event_loop = self.event_loop;
        let mut engine = self.engine;
        let mut app = self.app;

        let mut should_exit = false;
        let mut frame_count = 0u64;
        let mut last_tick = std::time::Instant::now();
        let mut last_time_printed = last_tick;

        while !should_exit {
            let delta_time = {
                let now = std::time::Instant::now();
                let delta_time = now - last_tick;
                last_tick = now;

                let last_time_print_elapsed = (now - last_time_printed).as_secs_f32();
                if last_time_print_elapsed > 1. {
                    info!("Frame rate: {} fps", frame_count as f32 / last_time_print_elapsed);
                    last_time_printed = now;
                    frame_count = 0;
                }

                delta_time.as_secs_f32()
            };

            let mut collector = SystemEventCollector::new();
            event_loop.pump_app_events(Some(Duration::ZERO), &mut collector);

            should_exit = Self::process_event(&mut engine, &mut app, &collector);
            app.process_event(&collector);

            engine.tick(delta_time);
            app.tick(delta_time);

            engine.render(&mut app);

            frame_count += 1;
        }

        Ok(())
    }

    fn process_event(engine: &mut Engine, app: &mut A, collector: &SystemEventCollector) -> bool {
        let mut should_exit = false;
        let mut had_resized = false;

        for event in collector.window_events() {
            match event {
                WindowEvent::Resized(_) => {
                    if had_resized {
                        continue;
                    }

                    let inner_size = engine.main_window.inner_size();
                    engine.resize(inner_size.width, inner_size.height);
                    app.resize(inner_size.width, inner_size.height);
                    had_resized = true;
                }
                WindowEvent::CloseRequested => {
                    should_exit = true;
                }
                _ => {}
            }
        }

        should_exit
    }
}