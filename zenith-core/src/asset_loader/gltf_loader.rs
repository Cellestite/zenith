use anyhow::{anyhow, Result};
use bytemuck::{Pod, Zeroable};
use glam::{Vec2, Vec3};
use gltf::{buffer::Data, Document, Primitive};
use log::info;
use std::path::Path;

#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct Vertex {
    pub position: [f32; 3],
    pub normal: [f32; 3],
    pub tex_coord: [f32; 2],
}

impl Vertex {
    pub fn new(position: Vec3, normal: Vec3, tex_coord: Vec2) -> Self {
        Self {
            position: position.to_array(),
            normal: normal.to_array(),
            tex_coord: tex_coord.to_array(),
        }
    }
}

#[derive(Debug)]
pub struct MeshData {
    pub vertices: Vec<Vertex>,
    pub indices: Vec<u32>,
    pub name: Option<String>,
}

impl MeshData {
    pub fn new(vertices: Vec<Vertex>, indices: Vec<u32>, name: Option<String>) -> Self {
        Self {
            vertices,
            indices,
            name,
        }
    }

    pub fn vertex_bytes(&self) -> &[u8] {
        bytemuck::cast_slice(&self.vertices)
    }

    pub fn index_bytes(&self) -> &[u8] {
        bytemuck::cast_slice(&self.indices)
    }
}

#[derive(Debug)]
pub struct ModelData {
    pub meshes: Vec<MeshData>,
    pub name: Option<String>,
}

impl ModelData {
    pub fn new(meshes: Vec<MeshData>, name: Option<String>) -> Self {
        Self { meshes, name }
    }
}

pub struct GltfLoader;

impl GltfLoader {
    pub fn load_from_file<P: AsRef<Path>>(path: P) -> Result<ModelData> {
        let path = path.as_ref();

        info!("Load from file: {:?}", path);

        let (gltf, buffers, _images) = gltf::import(path)?;
        Self::process_gltf(gltf, buffers, path.file_stem().and_then(|s| s.to_str()).ok_or(anyhow!("Invalid path!"))?)
    }

    pub fn load_from_bytes(data: &[u8], name: &str) -> Result<ModelData> {
        info!("Load from memory");

        let (gltf, buffers, _images) = gltf::import_slice(data)?;
        Self::process_gltf(gltf, buffers, name)
    }

    fn process_gltf(gltf: Document, buffers: Vec<Data>, name: &str) -> Result<ModelData> {
        let mut model_meshes = Vec::new();

        for scene in gltf.scenes() {
            for node in scene.nodes() {
                Self::process_node(&node, &buffers, &mut model_meshes)?;
            }
        }

        if model_meshes.is_empty() {
            return Err(anyhow!("Empty gltf file!"));
        }

        info!("Loaded successfully, found {} meshes for {}", model_meshes.len(), name);

        Ok(ModelData::new(model_meshes, Some(name.to_owned())))
    }

    fn process_node(
        node: &gltf::Node,
        buffers: &[Data],
        meshes: &mut Vec<MeshData>,
    ) -> Result<()> {
        if let Some(mesh) = node.mesh() {
            for primitive in mesh.primitives() {
                let mesh_data = Self::process_primitive(&primitive, buffers)?;
                meshes.push(mesh_data);
            }
        }

        for child in node.children() {
            Self::process_node(&child, buffers, meshes)?;
        }

        Ok(())
    }

    fn process_primitive(primitive: &Primitive, buffers: &[Data]) -> Result<MeshData> {
        let reader = primitive.reader(|buffer| Some(&buffers[buffer.index()]));

        let positions: Vec<Vec3> = reader
            .read_positions()
            .ok_or_else(|| anyhow!("Missing position attributes!"))?
            .map(Vec3::from)
            .collect();

        let normals: Vec<Vec3> = match reader.read_normals() {
            Some(normals_iter) => normals_iter.map(Vec3::from).collect(),
            None => {
                info!("Missing normal attributes，Generating...");
                Self::generate_normals(&positions)?
            }
        };

        let tex_coords: Vec<Vec2> = match reader.read_tex_coords(0) {
            Some(coords_iter) => coords_iter.into_f32().map(Vec2::from).collect(),
            None => {
                info!("Missing texture coordinates. Use (0, 0) for all vertices.");
                vec![Vec2::ZERO; positions.len()]
            }
        };

        if positions.len() != normals.len() || positions.len() != tex_coords.len() {
            return Err(anyhow!(
                "Inconsistent vertices length: position={}, normal={}, tex coord={}",
                positions.len(),
                normals.len(),
                tex_coords.len()
            ));
        }

        let vertices: Vec<Vertex> = positions
            .into_iter()
            .zip(normals.into_iter())
            .zip(tex_coords.into_iter())
            .map(|((pos, normal), tex_coord)| Vertex::new(pos, normal, tex_coord))
            .collect();

        let indices: Vec<u32> = match reader.read_indices() {
            Some(indices_iter) => indices_iter.into_u32().collect(),
            None => {
                // Assume triangles are separated
                (0..vertices.len() as u32).collect()
            }
        };

        Ok(MeshData::new(
            vertices,
            indices,
            primitive.material().name().map(|s| s.to_string()),
        ))
    }

    fn generate_normals(positions: &[Vec3]) -> Result<Vec<Vec3>> {
        if positions.len() % 3 != 0 {
            return Err(anyhow!("Incorrect data stride. Can NOT generate valid normals!"));
        }

        let mut normals = vec![Vec3::ZERO; positions.len()];

        for i in (0..positions.len()).step_by(3) {
            let v0 = positions[i];
            let v1 = positions[i + 1];
            let v2 = positions[i + 2];

            let normal = (v1 - v0).cross(v2 - v0).normalize();

            normals[i] = normal;
            normals[i + 1] = normal;
            normals[i + 2] = normal;
        }

        Ok(normals)
    }
}
