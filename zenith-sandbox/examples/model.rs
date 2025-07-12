use log::{info};
use std::env;
use std::sync::{Arc, Weak};
use winit::window::Window;
use zenith_core::asset_loader::GltfLoader;
use zenith_renderer::SimpleMeshRenderer;
use zenith_rendergraph::{RenderGraphBuilder, RenderGraphResource, Texture};
use zenith::{launch, App, RenderableApp, Engine};

pub struct GltfRendererApp {
    main_window: Weak<Window>,
    mesh_renderer: SimpleMeshRenderer,
}

impl App for GltfRendererApp {
    async fn init(engine: &mut Engine) -> Result<Self, anyhow::Error> {
        let args: Vec<String> = env::args().collect();
        if args.len() != 2 {
            eprintln!("用法: {} <gltf文件路径>", args[0]);
            eprintln!("示例: {} content/mesh/cerberus/scene.gltf", args[0]);
            std::process::exit(1);
        }

        let gltf_path = &args[1];

        info!("加载 GLTF 模型: {}", gltf_path);
        let model = GltfLoader::load_from_file(gltf_path)?;
        info!("成功加载模型，包含 {} 个网格", model.meshes.len());

        let mut mesh_renderer = SimpleMeshRenderer::from_model(&engine.render_device, &model);
        mesh_renderer.set_base_color([0.7, 0.5, 0.3]); // 设置为暖色调

        info!("GLTF 渲染器初始化完成");

        Ok(Self {
            main_window: Arc::downgrade(&engine.main_window),
            mesh_renderer,
        })
    }
}

impl RenderableApp for GltfRendererApp {
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

        Some(self.mesh_renderer.build_render_graph(
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
    let engine_loop = pollster::block_on(launch::<GltfRendererApp>()).unwrap();

    engine_loop
        .run()
        .expect("Failed to start zenith engine loop!");
}