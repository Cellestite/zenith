use crate::material::{PbrMaterial, PbrTextures, TextureData};
use crate::mesh::{MeshData, Vertex};
use glam::{Vec2, Vec3};

/// Helper functions for creating mesh and material data
pub struct MeshHelpers;

impl MeshHelpers {
    /// Create a simple quad mesh with positions, normals, and UV coordinates
    pub fn create_quad(size: f32) -> MeshData {
        let half_size = size * 0.5;
        
        let vertices = vec![
            // Bottom-left
            Vertex::new(
                Vec3::new(-half_size, -half_size, 0.0),
                Vec3::new(0.0, 0.0, 1.0),
                Vec2::new(0.0, 0.0),
            ),
            // Bottom-right
            Vertex::new(
                Vec3::new(half_size, -half_size, 0.0),
                Vec3::new(0.0, 0.0, 1.0),
                Vec2::new(1.0, 0.0),
            ),
            // Top-right
            Vertex::new(
                Vec3::new(half_size, half_size, 0.0),
                Vec3::new(0.0, 0.0, 1.0),
                Vec2::new(1.0, 1.0),
            ),
            // Top-left
            Vertex::new(
                Vec3::new(-half_size, half_size, 0.0),
                Vec3::new(0.0, 0.0, 1.0),
                Vec2::new(0.0, 1.0),
            ),
        ];

        let indices = vec![0, 1, 2, 2, 3, 0];

        MeshData::new(vertices, indices, Some("Quad".to_string()), Some(0))
    }

    /// Create a simple cube mesh
    pub fn create_cube(size: f32) -> MeshData {
        let half_size = size * 0.5;
        
        let vertices = vec![
            // Front face
            Vertex::new(Vec3::new(-half_size, -half_size, half_size), Vec3::new(0.0, 0.0, 1.0), Vec2::new(0.0, 0.0)),
            Vertex::new(Vec3::new(half_size, -half_size, half_size), Vec3::new(0.0, 0.0, 1.0), Vec2::new(1.0, 0.0)),
            Vertex::new(Vec3::new(half_size, half_size, half_size), Vec3::new(0.0, 0.0, 1.0), Vec2::new(1.0, 1.0)),
            Vertex::new(Vec3::new(-half_size, half_size, half_size), Vec3::new(0.0, 0.0, 1.0), Vec2::new(0.0, 1.0)),
            
            // Back face
            Vertex::new(Vec3::new(-half_size, -half_size, -half_size), Vec3::new(0.0, 0.0, -1.0), Vec2::new(1.0, 0.0)),
            Vertex::new(Vec3::new(-half_size, half_size, -half_size), Vec3::new(0.0, 0.0, -1.0), Vec2::new(1.0, 1.0)),
            Vertex::new(Vec3::new(half_size, half_size, -half_size), Vec3::new(0.0, 0.0, -1.0), Vec2::new(0.0, 1.0)),
            Vertex::new(Vec3::new(half_size, -half_size, -half_size), Vec3::new(0.0, 0.0, -1.0), Vec2::new(0.0, 0.0)),
        ];

        let indices = vec![
            // Front face
            0, 1, 2, 2, 3, 0,
            // Back face
            4, 5, 6, 6, 7, 4,
            // Left face
            4, 0, 3, 3, 5, 4,
            // Right face
            1, 7, 6, 6, 2, 1,
            // Bottom face
            4, 7, 1, 1, 0, 4,
            // Top face
            3, 2, 6, 6, 5, 3,
        ];

        MeshData::new(vertices, indices, Some("Cube".to_string()), Some(0))
    }

    /// Create a triangle mesh for testing
    pub fn create_triangle() -> MeshData {
        let vertices = vec![
            Vertex::new(
                Vec3::new(0.0, 0.5, 0.0),
                Vec3::new(0.0, 0.0, 1.0),
                Vec2::new(0.5, 1.0),
            ),
            Vertex::new(
                Vec3::new(-0.5, -0.5, 0.0),
                Vec3::new(0.0, 0.0, 1.0),
                Vec2::new(0.0, 0.0),
            ),
            Vertex::new(
                Vec3::new(0.5, -0.5, 0.0),
                Vec3::new(0.0, 0.0, 1.0),
                Vec2::new(1.0, 0.0),
            ),
        ];

        let indices = vec![0, 1, 2];

        MeshData::new(vertices, indices, Some("Triangle".to_string()), Some(0))
    }
}

/// Helper functions for creating materials
pub struct MaterialHelpers;

impl MaterialHelpers {
    /// Create a default PBR material with specified base color
    pub fn create_colored_material(color: [f32; 4]) -> PbrMaterial {
        PbrMaterial {
            name: Some("Colored Material".to_string()),
            base_color_factor: color,
            metallic_factor: 0.0,
            roughness_factor: 0.5,
            emissive_factor: [0.0, 0.0, 0.0],
            textures: PbrTextures::default(),
        }
    }

    /// Create a metallic material
    pub fn create_metallic_material(base_color: [f32; 4], metallic: f32, roughness: f32) -> PbrMaterial {
        PbrMaterial {
            name: Some("Metallic Material".to_string()),
            base_color_factor: base_color,
            metallic_factor: metallic,
            roughness_factor: roughness,
            emissive_factor: [0.0, 0.0, 0.0],
            textures: PbrTextures::default(),
        }
    }

    /// Create a simple texture data from solid color (useful for testing)
    pub fn create_solid_color_texture(width: u32, height: u32, color: [u8; 4]) -> TextureData {
        let pixel_count = (width * height) as usize;
        let mut pixels = Vec::with_capacity(pixel_count * 4);
        
        for _ in 0..pixel_count {
            pixels.extend_from_slice(&color);
        }

        TextureData {
            pixels,
            width,
            height,
            format: gltf::image::Format::R8G8B8A8,
        }
    }

    /// Create a checkerboard texture for testing
    pub fn create_checkerboard_texture(width: u32, height: u32, color1: [u8; 4], color2: [u8; 4]) -> TextureData {
        let mut pixels = Vec::with_capacity((width * height * 4) as usize);
        
        for y in 0..height {
            for x in 0..width {
                let checker_size = 8;
                let checker_x = (x / checker_size) % 2;
                let checker_y = (y / checker_size) % 2;
                
                let color = if (checker_x + checker_y) % 2 == 0 {
                    color1
                } else {
                    color2
                };
                
                pixels.extend_from_slice(&color);
            }
        }

        TextureData {
            pixels,
            width,
            height,
            format: gltf::image::Format::R8G8B8A8,
        }
    }
}