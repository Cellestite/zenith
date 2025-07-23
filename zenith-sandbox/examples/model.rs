use log::{error, info};
use std::env;
use std::sync::{Arc, Weak};
use winit::window::Window;
use zenith::{launch, App, RenderableApp, block_on, RenderGraphBuilder, RenderGraphResource, Texture, SimpleMeshRenderer, RenderDevice, TaskResult, submit};
use zenith::asset_loader::{GltfLoader, ModelData};

pub struct GltfRendererApp {
    load_task: TaskResult<anyhow::Result<ModelData>>,
    main_window: Weak<Window>,
    mesh_renderer: Option<SimpleMeshRenderer>,
}

impl App for GltfRendererApp {
    async fn new(main_window: Arc<Window>) -> Result<Self, anyhow::Error> {
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
            main_window: Arc::downgrade(&main_window),
            mesh_renderer: None,
        })
    }
}

impl RenderableApp for GltfRendererApp {
    fn prepare(&mut self, render_device: &mut RenderDevice) -> Result<(), anyhow::Error> {
        let model = self.load_task.get_result()?;
        let mut mesh_renderer = SimpleMeshRenderer::from_model(&render_device, &model);
        mesh_renderer.set_base_color([0.7, 0.5, 0.3]);

        self.mesh_renderer = Some(mesh_renderer);
        Ok(())
    }

    fn render(&mut self, builder: &mut RenderGraphBuilder) -> Option<RenderGraphResource<Texture>> {
        let (width, height) = if let Some(window) = self.main_window.upgrade() {
            (window.inner_size().width, window.inner_size().height)
        } else {
            return None;
        };

        let eye = glam::Vec3::new(50.0, 30.0, 100.0);
        let target = glam::Vec3::new(0.0, 0.0, 0.0);
        let up = glam::Vec3::new(0.0, 1.0, 0.0);

        let model_matrix = glam::Mat4::from_scale(glam::Vec3::splat(0.5));
        let view_matrix = glam::Mat4::look_at_rh(eye, target, up);

        let aspect_ratio = width as f32 / height as f32;
        let proj_matrix = glam::Mat4::perspective_rh(
            std::f32::consts::PI / 4.0,
            aspect_ratio,
            0.1,
            1000.0
        );

        Some(self.mesh_renderer.as_ref().unwrap().build_render_graph(
            builder,
            view_matrix,
            proj_matrix,
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