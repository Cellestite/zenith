use std::hash::{Hash, Hasher};
use zenith_build::ShaderEntry;
use zenith_core::collections::SmallVec;

pub const SHADER_ASSET_ABSOLUTE_DIR: &str = include_absolute_path::include_absolute_path!("../../zenith-build/shader");

// TODO: robust shader hash
pub struct GraphicShader {
    name: String,
    reflection_info: ShaderEntry,

    vertex_entry: String,
    vertex_layout: Vec<wgpu::VertexBufferLayout<'static>>,
    vertex_constants: Vec<(&'static str, f64)>,

    fragment_entry: String,
    fragment_constants: Vec<(&'static str, f64)>,

    bind_group_layouts: SmallVec<[wgpu::BindGroupLayoutDescriptor<'static>; 4]>,

    num_color_targets: u32,
    _has_depth_stencil: bool,
}

impl GraphicShader {
    pub fn new(
        name: &str,
        reflection_info: ShaderEntry,
        vertex_entry: &str,
        vertex_layout: Vec<wgpu::VertexBufferLayout<'static>>,
        fragment_constants: Vec<(&'static str, f64)>,

        fragment_entry: &str,
        vertex_constants: Vec<(&'static str, f64)>,

        num_color_targets: u32,
        _has_depth_stencil: bool,

        bind_group_layouts: SmallVec<[wgpu::BindGroupLayoutDescriptor<'static>; 4]>,
    ) -> anyhow::Result<Self> {

        Ok(Self {
            name: name.to_owned(),
            reflection_info,
            vertex_entry: vertex_entry.to_owned(),
            vertex_layout,
            vertex_constants,
            fragment_entry: fragment_entry.to_owned(),
            fragment_constants,
            num_color_targets,
            _has_depth_stencil,
            bind_group_layouts,
        })
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn create_vertex_state<'a>(&'a self, module: &'a wgpu::ShaderModule) -> wgpu::VertexState<'a> {
        wgpu::VertexState {
            module,
            entry_point: Some(self.vertex_entry_name()),
            compilation_options: wgpu::PipelineCompilationOptions {
                constants: &self.vertex_constants,
                ..Default::default()
            },
            buffers: &self.vertex_layout,
        }
    }

    pub fn create_fragment_state<'a>(&'a self, module: &'a wgpu::ShaderModule, color_targets: &'a [Option<wgpu::ColorTargetState>]) -> Option<wgpu::FragmentState<'a>> {
        if self.num_color_targets != 0 {
            Some(wgpu::FragmentState {
                module,
                entry_point: Some(self.fragment_entry_name()),
                compilation_options: wgpu::PipelineCompilationOptions {
                    constants: &self.fragment_constants,
                    ..Default::default()
                },
                targets: color_targets,
            })
        } else {
            None
        }
    }

    pub fn create_pipeline_layout(&self, device: &wgpu::Device) -> wgpu::PipelineLayout {
        self.reflection_info.create_pipeline_layout(device)
    }
    pub fn create_shader_module_relative_path(
        &self,
        device: &wgpu::Device,
        shader_defs: std::collections::HashMap<String, naga_oil::compose::ShaderDefValue>,
    ) -> Result<wgpu::ShaderModule, naga_oil::compose::ComposerError> {
        self.reflection_info.create_shader_module_relative_path(
            device,
            SHADER_ASSET_ABSOLUTE_DIR,
            self.reflection_info,
            shader_defs,
            |path| {
                let path = path.replace("/", "\\");
                std::fs::read_to_string(path)
            }
        )
    }

    pub fn create_bind_group_layout(&self, device: &wgpu::Device, group: u32) -> Option<wgpu::BindGroupLayout> {
        self.bind_group_layouts.get(group as usize).map(|binding| device.create_bind_group_layout(binding))
    }

    pub fn relative_path(&self) -> &'static str {
        self.reflection_info.relative_path()
    }

    pub fn num_bind_groups(&self) -> u32 {
        self.bind_group_layouts.len() as u32
    }

    pub fn num_bindings(&self, group: u32) -> Option<u32> {
        self.bind_group_layouts.get(group as usize).map(|binding| binding.entries.len() as u32)
    }

    pub fn vertex_entry_name(&self) -> &str {
        &self.vertex_entry
    }

    pub fn fragment_entry_name(&self) -> &str {
        &self.fragment_entry
    }
}

impl Hash for GraphicShader {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.name.hash(state)
    }
}