use log::{error, info, warn};
use std::env;
use std::sync::{Arc, Weak};
use glam::{Quat, Vec2, Vec3};
use winit::event::MouseButton;
use winit::window::Window;
use zenith::{launch, App, RenderableApp, block_on, RenderGraphBuilder, RenderGraphResource, Texture, SimpleMeshRenderer, RenderDevice, TaskResult, submit};
use zenith::asset_loader::{GltfLoader, ModelData};
use zenith::camera::Camera;

pub struct GltfRendererApp {
    load_task: TaskResult<anyhow::Result<ModelData>>,
    main_window: Option<Weak<Window>>,
    mesh_renderer: Option<SimpleMeshRenderer>,

    camera: Camera,
    delta_translation: Vec3,
    delta_rotation: Vec2,
    should_apply_rotation: bool,
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

        Ok(Self {
            load_task,
            main_window: None,
            mesh_renderer: None,
            
            camera: Default::default(),
            delta_translation: Default::default(),
            delta_rotation: Default::default(),

            should_apply_rotation: false,
        })
    }

    fn on_key_input(&mut self, name: Option<&str>, is_pressed: bool) {
        if !is_pressed {
            return;
        }

        if name.is_none() {
            warn!("Unknown input key!");
            return;
        }

        let name = unsafe { name.unwrap_unchecked() };
        if name == "a" {
            self.delta_translation.x -= 30.;
        } else if name == "d" {
            self.delta_translation.x += 0.01;
        } else if name == "w" {
            self.delta_translation.y += 0.01;
        } else if name == "s" {
            self.delta_translation.y -= 0.01;
        } else if name == "q" {
            self.delta_translation.z += 0.01;
        } else if name == "e" {
            self.delta_translation.z -= 0.01;
        }
    }

    fn on_mouse_input(&mut self, button: &MouseButton, is_pressed: bool) {
        match button {
            MouseButton::Left if is_pressed => { self.should_apply_rotation = true; }
            MouseButton::Left if !is_pressed => { self.should_apply_rotation = false; }
            _ => {}
        }
    }

    fn on_mouse_moved(&mut self, delta: &Vec2) {
        self.delta_rotation = delta * 0.004;
    }

    fn tick(&mut self, delta_time: f32) {
        self.delta_translation *= delta_time;
        self.camera.translation(self.delta_translation);

        if self.should_apply_rotation {
            self.delta_rotation *= delta_time;

            self.camera.rotate_yaw(self.delta_rotation.x.into());
            self.camera.rotate_pitch(self.delta_rotation.y.into());
        }

        self.delta_translation = Vec3::ZERO;
        self.delta_rotation = Vec2::ZERO;
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