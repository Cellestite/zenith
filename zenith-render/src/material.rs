#[derive(Debug, Clone)]
pub struct TextureData {
    pub pixels: Vec<u8>,
    pub width: u32,
    pub height: u32,
    pub format: gltf::image::Format,
}

#[derive(Debug, Clone)]
pub struct PbrTextures {
    pub base_color: Option<TextureData>,
    pub metallic_roughness: Option<TextureData>,
    pub normal: Option<TextureData>,
    pub occlusion: Option<TextureData>,
    pub emissive: Option<TextureData>,
}

impl Default for PbrTextures {
    fn default() -> Self {
        Self {
            base_color: None,
            metallic_roughness: None,
            normal: None,
            occlusion: None,
            emissive: None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct PbrMaterial {
    pub name: Option<String>,
    pub base_color_factor: [f32; 4],
    pub metallic_factor: f32,
    pub roughness_factor: f32,
    pub emissive_factor: [f32; 3],
    pub textures: PbrTextures,
}

impl Default for PbrMaterial {
    fn default() -> Self {
        Self {
            name: None,
            base_color_factor: [1.0, 1.0, 1.0, 1.0],
            metallic_factor: 1.0,
            roughness_factor: 1.0,
            emissive_factor: [0.0, 0.0, 0.0],
            textures: PbrTextures::default(),
        }
    }
}

#[derive(Debug)]
pub struct MaterialData {
    pub materials: Vec<PbrMaterial>,
}

impl MaterialData {
    pub fn new(materials: Vec<PbrMaterial>) -> Self {
        Self { materials }
    }
}

#[derive(Debug)]
pub struct ModelData {
    pub meshes: Vec<crate::mesh::MeshData>,
    pub materials: MaterialData,
    pub name: Option<String>,
}

impl ModelData {
    pub fn new(meshes: Vec<crate::mesh::MeshData>, materials: MaterialData, name: Option<String>) -> Self {
        Self { meshes, materials, name }
    }
}