use std::num::NonZeroU32;
use std::sync::Arc;
use winit::window::Window;
use zenith_core::log::info;

mod pipeline_cache;
mod shader;

pub use pipeline_cache::PipelineCache;
pub use shader::{GraphicShader, VertexBufferLayout};

pub struct RenderDevice {
    #[allow(dead_code)]
    instance: wgpu::Instance,
    #[allow(dead_code)]
    adapter: wgpu::Adapter,
    device: wgpu::Device,
    queue: wgpu::Queue,
    surface: wgpu::Surface<'static>,
    surface_config: wgpu::SurfaceConfiguration,
}

impl RenderDevice {
    pub async fn new(window: Arc<Window>) -> Result<Self, anyhow::Error> {
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            backends: wgpu::Backends::VULKAN,
            // TODO: debug only
            flags: wgpu::InstanceFlags::GPU_BASED_VALIDATION,
            ..Default::default()
        });

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                ..Default::default()
            })
            .await?;
        let adapter_info = adapter.get_info();
        info!("Selected adapter: {} ({:?})\n\tDriver {}: {}",
            adapter_info.name,
            adapter_info.backend,
            adapter_info.driver,
            adapter_info.driver_info);

        let window_size = window.inner_size();
        let width = window_size.width.max(1);
        let height = window_size.height.max(1);
        let surface = instance.create_surface(window)?;

        let surface_caps = surface.get_capabilities(&adapter);
        let picked_surface_format = surface_caps
            .formats
            .iter()
            .copied()
            .find(|f| f.is_srgb())
            .unwrap_or(surface_caps.formats[0]);
        let mut surface_config = surface
            .get_default_config(&adapter, width, height)
            .expect("Surface isn't supported by the adapter.");

        surface_config.format = picked_surface_format;
        surface_config.view_formats.push(picked_surface_format);
        info!("Picked surface pixel format: {:?}, resolution({}x{})", picked_surface_format, width, height);

        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: Some("zenith rhi device"),
                    ..Default::default()
                },
            )
            .await?;

        surface.configure(&device, &surface_config);

        Ok(Self {
            instance,
            adapter,
            device,
            queue,
            surface,
            surface_config,
        })
    }

    pub fn wait_until_idle(&self) {
        self
            .device
            .poll(wgpu::PollType::Wait)
            .expect("Failed to wait until all queues idle");
    }

    pub fn device(&self) -> &wgpu::Device {
        &self.device
    }

    pub fn queue(&self) -> &wgpu::Queue {
        &self.queue
    }

    pub fn require_presentation(&self) -> wgpu::SurfaceTexture {
        match self.surface.get_current_texture() {
            Ok(frame) => frame,
            // If we timed out, just try again
            Err(wgpu::SurfaceError::Timeout) => self.surface
                .get_current_texture()
                .expect("Failed to acquire next surface texture!"),
            Err(
                // If the surface is outdated, or was lost, reconfigure it.
                wgpu::SurfaceError::Outdated
                | wgpu::SurfaceError::Lost
                | wgpu::SurfaceError::Other
                // If OutOfMemory happens, reconfiguring may not help, but we might as well try
                | wgpu::SurfaceError::OutOfMemory,
            ) => {
                self.surface.configure(&self.device, &self.surface_config);
                self.surface
                    .get_current_texture()
                    .expect("Failed to acquire next surface texture!")
            }
        }
    }

    pub fn resize(&mut self, width: NonZeroU32, height: NonZeroU32) {
        self.surface_config.width = width.get();
        self.surface_config.height = height.get();
        self.surface.configure(&self.device, &self.surface_config);
    }
}