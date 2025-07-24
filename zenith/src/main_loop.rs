use std::sync::Arc;
use std::time::Duration;
use anyhow::anyhow;
use glam::Vec2;
use log::{debug};
use winit::application::ApplicationHandler;
use winit::event::{MouseButton, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::keyboard::Key;
use winit::platform::pump_events::EventLoopExtPumpEvents;
use winit::window::{Window, WindowAttributes, WindowId};
use zenith_core::collections::SmallVec;
use zenith_task::block_on;
use crate::app::{RenderableApp};
use crate::Engine;

#[derive(Debug)]
enum EngineUserEvent {
    CreateWindow,
}

#[derive(Debug)]
enum OsEvent {
    WindowResize,
    CloseRequested,

    OnMouseInput(MouseButton, bool),
    OnCursorMoved(Vec2),
    OnKeyboardInput(Option<String>, bool),
}

struct OsEventCollector {
    events: Vec<OsEvent>,
    windows: SmallVec<[Window; 1]>,

    had_resize_event: bool,
}

impl OsEventCollector {
    fn new() -> Self {
        Self {
            events: Vec::new(),
            windows: SmallVec::new(),

            had_resize_event: false,
        }
    }
}

pub struct EngineLoop<A> {
    event_loop: EventLoop<EngineUserEvent>,
    engine: Engine,
    app: A,
}

impl<A: RenderableApp> EngineLoop<A> {
    pub(super) fn new() -> Result<Self, anyhow::Error> {
        zenith_core::log::initialize()?;

        let mut app = block_on(A::new())?;

        let mut event_loop = EventLoop::<EngineUserEvent>::with_user_event().build()?;
        event_loop.set_control_flow(ControlFlow::Poll);

        let proxy = event_loop.create_proxy();
        proxy.send_event(EngineUserEvent::CreateWindow)
            .expect("Failed to send main window creation request!");

        let mut collector = OsEventCollector::new();
        event_loop.pump_app_events(Some(Duration::ZERO), &mut collector);

        let OsEventCollector { windows, .. } = collector;

        let main_window = Arc::new(windows
            .into_iter()
            .take(1)
            .next()
            .ok_or(anyhow!("Failed to create main window!"))?);

        let mut engine = block_on(Engine::new(main_window.clone()))?;
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
        let mut last_tick = std::time::Instant::now();

        while !should_exit {
            let delta_time = {
                let now = std::time::Instant::now();
                let delta_time = now - last_tick;
                last_tick = now;
                delta_time.as_secs_f32()
            };

            let mut collector = OsEventCollector::new();
            event_loop.pump_app_events(Some(Duration::ZERO), &mut collector);

            let OsEventCollector { events, .. } = collector;
            should_exit = Self::process_os_event(&mut engine, &mut app, events);

            engine.tick(delta_time);
            app.tick(delta_time);

            engine.render(&mut app);
        }

        Ok(())
    }

    fn process_os_event(engine: &mut Engine, app: &mut A, events: impl IntoIterator<Item = OsEvent>) -> bool {
        let mut should_exit = false;

        for event in events {
            match event {
                OsEvent::WindowResize => {
                    let inner_size = engine.main_window.inner_size();
                    engine.resize(inner_size.width, inner_size.height);
                    app.resize(inner_size.width, inner_size.height);
                }
                OsEvent::CloseRequested => {
                    should_exit = true;
                }
                OsEvent::OnMouseInput(button, is_pressed) => {
                    app.on_mouse_input(&button, is_pressed);
                }
                OsEvent::OnCursorMoved(new_pos) => {
                    app.on_mouse_moved(&new_pos);
                }
                OsEvent::OnKeyboardInput(key, is_pressed) => {
                    let str = key.as_ref().map(String::as_str);
                    app.on_key_input(str, is_pressed);
                }
            }
        }

        should_exit
    }
}

impl ApplicationHandler<EngineUserEvent> for OsEventCollector {
    fn resumed(&mut self, _event_loop: &ActiveEventLoop) {}

    fn user_event(&mut self, event_loop: &ActiveEventLoop, event: EngineUserEvent) {
        match event {
            EngineUserEvent::CreateWindow => {
                let window_attributes = WindowAttributes::default()
                    .with_title("Zenith Engine")
                    .with_inner_size(winit::dpi::LogicalSize::new(1280, 720))
                    .with_min_inner_size(winit::dpi::LogicalSize::new(64, 64))
                    .with_resizable(true);

                self.windows.push(event_loop.create_window(window_attributes).expect("Failed to create window!"));
            }
        }
    }

    fn window_event(&mut self,
                    event_loop: &ActiveEventLoop,
                    _window_id: WindowId,
                    event: WindowEvent
    ) {
        match event {
            WindowEvent::Resized(new_size) => {
                if self.had_resize_event {
                    return;
                }

                debug!("Operating system event [Window Resize]: {}x{}", new_size.width, new_size.height);
                self.events.push(OsEvent::WindowResize);
                self.had_resize_event = true;
            }
            WindowEvent::CloseRequested => {
                self.events.push(OsEvent::CloseRequested);
                event_loop.exit();
            }
            WindowEvent::MouseInput { button, state, .. } => {
                self.events.push(OsEvent::OnMouseInput(button, state.is_pressed()));
            }
            WindowEvent::CursorMoved { position, .. } => {
                self.events.push(OsEvent::OnCursorMoved((position.x as f32, position.y as f32).into()));
            }
            WindowEvent::KeyboardInput { event, .. } => {
                match event.logical_key {
                    Key::Named(named) => { self.events.push(OsEvent::OnKeyboardInput(named.to_text().map(|str| str.to_owned()), event.state.is_pressed())); }
                    Key::Character(str) => { self.events.push(OsEvent::OnKeyboardInput(Some(str.as_str().to_owned()), event.state.is_pressed())); }
                    Key::Unidentified(_) => {}
                    Key::Dead(_) => {}
                }
            }
            _ => {}
        }
    }
}
