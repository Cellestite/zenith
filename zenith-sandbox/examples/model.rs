use log::{error, info};
use std::env;
use std::sync::{Arc, Weak};
use glam::{Quat, Vec3};
use winit::keyboard::KeyCode;
use winit::window::Window;
use zenith::{launch, App, RenderableApp, block_on, RenderGraphBuilder, RenderGraphResource, Texture, SimpleMeshRenderer, RenderDevice, TaskResult, submit};
use zenith::asset_loader::{GltfLoader, ModelData};
use zenith::camera::{Camera, CameraController};
use zenith::input::InputActionMapper;
use zenith::system_event::SystemEventCollector;

pub struct GltfRendererApp {
    load_task: TaskResult<anyhow::Result<ModelData>>,
    main_window: Option<Weak<Window>>,
    mesh_renderer: Option<SimpleMeshRenderer>,

    camera: Camera,
    controller: CameraController,

    mapper: InputActionMapper,
}

impl App for GltfRendererApp {
    async fn new() -> Result<Self, anyhow::Error> {
        let args: Vec<String> = env::args().collect();
        if args.len() != 2 {
            error!("用法: {} <gltf文件路径>", args[0]);
            error!("示例: {} content/mesh/cerberus/scene.gltf", args[0]);
            std::process::exit(1);
        }

        let gltf_path = args[1].clone();
        let load_task = submit(|| {
            info!("Worker thread: {:?} reading gltf...", std::thread::current().name());
            GltfLoader::load_from_file(gltf_path)
        });

        let mut mapper = InputActionMapper::new();
        mapper.register_axis("strafe", [KeyCode::KeyD], [KeyCode::KeyA], 0.5);
        mapper.register_axis("walk", [KeyCode::KeyW], [KeyCode::KeyS], 0.5);
        mapper.register_axis("lift", [KeyCode::KeyE], [KeyCode::KeyQ], 0.5);

        Ok(Self {
            load_task,
            main_window: None,
            mesh_renderer: None,
            
            camera: Default::default(),
            controller: Default::default(),

            mapper,
        })
    }

    fn process_event(&mut self, collector: &SystemEventCollector) {
        self.mapper.process_event(collector);
        if let Some(window) = self.main_window.as_ref().and_then(|window| window.upgrade()) {
            self.controller.process_event(collector, &window);
        }
    }

    fn tick(&mut self, delta_time: f32) {
        self.mapper.tick(delta_time);
        self.controller.update_cameras(delta_time, &self.mapper, [&mut self.camera]);
    }
}

impl RenderableApp for GltfRendererApp {
    fn prepare(&mut self, render_device: &mut RenderDevice, main_window: Arc<Window>) -> Result<(), anyhow::Error> {
        let model = self.load_task.get_result()?;
        let mut mesh_renderer = SimpleMeshRenderer::from_model(&render_device, &model);
        mesh_renderer.set_base_color([0.7, 0.5, 0.3]);

        self.main_window = Some(Arc::downgrade(&main_window));
        self.mesh_renderer = Some(mesh_renderer);
        Ok(())
    }

    fn render(&mut self, builder: &mut RenderGraphBuilder) -> Option<RenderGraphResource<Texture>> {
        let (width, height) = if let Some(window) = self.main_window.as_ref().and_then(|window| window.upgrade()) {
            (window.inner_size().width, window.inner_size().height)
        } else {
            return None;
        };

        let model_matrix = glam::Mat4::from_scale_rotation_translation(Vec3::splat(0.5), Quat::IDENTITY, Vec3::new(0., 100.0, 0.));

        let view = self.camera.view();
        let proj = self.camera.projection();

        Some(self.mesh_renderer.as_ref().unwrap().build_render_graph(
            builder,
            view,
            proj,
            model_matrix,
            width,
            height
        ))
    }
}

fn main() {
    let engine_loop = block_on(launch::<GltfRendererApp>()).unwrap();

    engine_loop
        .run()
        .expect("Failed to start zenith engine loop!");
}