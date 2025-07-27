use std::ops::RangeBounds;
use std::vec::Drain;
use winit::application::ApplicationHandler;
use winit::event::{DeviceEvent, DeviceId, WindowEvent};
use winit::event_loop::ActiveEventLoop;
use winit::window::{Window, WindowAttributes, WindowId};
use crate::collections::SmallVec;

#[derive(Debug)]
pub enum UserEvent {
    CreateWindow,
}

pub struct SystemEventCollector {
    window_events: Vec<WindowEvent>,
    device_events: Vec<DeviceEvent>,
    pub windows: SmallVec<[Window; 1]>,
}

impl SystemEventCollector {
    pub fn new() -> Self {
        Self {
            window_events: Vec::new(),
            device_events: Vec::new(),
            windows: SmallVec::new(),
        }
    }

    #[inline]
    pub fn window_events(&self) -> &Vec<WindowEvent> {
        &self.window_events
    }

    #[inline]
    pub fn device_events(&self) -> &Vec<DeviceEvent> {
        &self.device_events
    }

    #[inline]
    pub fn drain_window_events<R: RangeBounds<usize>>(&mut self, range: R) -> Drain<WindowEvent> {
        self.window_events.drain(range)
    }

    #[inline]
    pub fn drain_device_events<R: RangeBounds<usize>>(&mut self, range: R) -> Drain<DeviceEvent> {
        self.device_events.drain(range)
    }
}

impl ApplicationHandler<UserEvent> for SystemEventCollector {
    fn resumed(&mut self, _event_loop: &ActiveEventLoop) {}

    fn user_event(&mut self, event_loop: &ActiveEventLoop, event: UserEvent) {
        match event {
            UserEvent::CreateWindow => {
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
                    _event_loop: &ActiveEventLoop,
                    _window_id: WindowId,
                    event: WindowEvent
    ) {
        self.window_events.push(event);
    }

    fn device_event(&mut self, _event_loop: &ActiveEventLoop, _device_id: DeviceId, event: DeviceEvent) {
        self.device_events.push(event);
    }
}
