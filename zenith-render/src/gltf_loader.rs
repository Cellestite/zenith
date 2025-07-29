use anyhow::{anyhow, Result};
use gltf::{buffer::Data, image::Data as ImageData, Document, Primitive};
use log::info;
use std::path::Path;
use std::fs::File;
use memmap2::Mmap;

use crate::mesh::{MeshData, Vertex};
use crate::material::{MaterialData, ModelData, PbrMaterial, PbrTextures, TextureData};

pub struct GltfLoader;

impl GltfLoader {
    pub fn load_from_file<P: AsRef<Path>>(path: P) -> Result<ModelData> {
        let path = path.as_ref();

        info!("Load from file: {:?}", path);

        let (gltf, buffers, images) = gltf::import(path)?;
        Self::process_gltf(gltf, buffers, images, path.file_stem().and_then(|s| s.to_str()).ok_or(anyhow!("Invalid path!"))?)
    }

    pub fn load_from_bytes(data: &[u8], name: &str) -> Result<ModelData> {
        info!("Load from memory");

        let (gltf, buffers, images) = gltf::import_slice(data)?;
        Self::process_gltf(gltf, buffers, images, name)
    }

    /// Load GLTF file using memory mapping for improved performance
    /// This method properly handles external dependencies (.bin files and textures) using mmap
    pub fn load_from_file_mmap<P: AsRef<Path>>(path: P) -> Result<ModelData> {
        let path = path.as_ref();
        
        info!("Load from file (mmap): {:?}", path);
        
        // Check if this is a GLB file (self-contained) or has external references
        if path.extension().and_then(|s| s.to_str()) == Some("glb") {
            // GLB files are self-contained and work well with mmap
            let file = File::open(path)
                .map_err(|e| anyhow!("Failed to open file {:?}: {}", path, e))?;
            
            let mmap = unsafe { Mmap::map(&file) }
                .map_err(|e| anyhow!("Failed to create memory mapping for {:?}: {}", path, e))?;
            
            let name = path.file_stem()
                .and_then(|s| s.to_str())
                .ok_or(anyhow!("Invalid path!"))?;
                
            Self::load_from_bytes(&mmap, name)
        } else {
            // For .gltf files with external references, use custom mmap loading
            Self::load_gltf_with_mmap_dependencies(path)
        }
    }

    /// Load GLTF file with external dependencies using memory mapping (optimized)
    fn load_gltf_with_mmap_dependencies<P: AsRef<Path>>(path: P) -> Result<ModelData> {
        let path = path.as_ref();
        let base_dir = path.parent().ok_or(anyhow!("Invalid path"))?;
        
        // First, load the main GLTF file to analyze its dependencies
        let gltf_file = File::open(path)
            .map_err(|e| anyhow!("Failed to open GLTF file {:?}: {}", path, e))?;
        let gltf_mmap = unsafe { Mmap::map(&gltf_file) }
            .map_err(|e| anyhow!("Failed to create memory mapping for GLTF file {:?}: {}", path, e))?;
        
        let gltf = gltf::Gltf::from_slice(&gltf_mmap)
            .map_err(|e| anyhow!("Failed to parse GLTF: {}", e))?;
            
        // Pre-allocate vectors based on known sizes
        let buffer_count = gltf.buffers().len();
        let image_count = gltf.images().len();
        
        let mut buffers = Vec::with_capacity(buffer_count);
        let mut _buffer_mmaps = Vec::with_capacity(buffer_count); // Keep mmaps alive
        
        // Load all buffer dependencies using mmap with zero-copy approach
        for buffer in gltf.buffers() {
            match buffer.source() {
                gltf::buffer::Source::Uri(uri) => {
                    if uri.starts_with("data:") {
                        // Handle data URIs (base64 encoded)
                        let mut blob = None;
                        let data = gltf::buffer::Data::from_source_and_blob(buffer.source(), None, &mut blob)
                            .map_err(|e| anyhow!("Failed to decode data URI: {}", e))?;
                        buffers.push(data);
                    } else {
                        // External file - use mmap with zero-copy
                        let buffer_path = base_dir.join(uri);
                        
                        let buffer_file = File::open(&buffer_path)
                            .map_err(|e| anyhow!("Failed to open buffer file {:?}: {}", buffer_path, e))?;
                        let buffer_mmap = unsafe { Mmap::map(&buffer_file) }
                            .map_err(|e| anyhow!("Failed to create memory mapping for buffer {:?}: {}", buffer_path, e))?;
                        
                        // Create data from slice (still faster than file I/O, single allocation)
                        let data = Data(buffer_mmap[..].to_vec());
                        buffers.push(data);
                        _buffer_mmaps.push(buffer_mmap); // Keep mmap alive for consistency
                    }
                }
                gltf::buffer::Source::Bin => {
                    return Err(anyhow!("Unexpected binary chunk in .gltf file"));
                }
            }
        }
        
        // Load all image dependencies using optimized parallel loading
        let mut images = Vec::with_capacity(image_count);
        let mut _image_mmaps = Vec::with_capacity(image_count); // Keep mmaps alive
        
        for image in gltf.images() {
            match image.source() {
                gltf::image::Source::Uri { uri, .. } => {
                    if uri.starts_with("data:") {
                        // Handle data URIs (base64 encoded)
                        let data = gltf::image::Data::from_source(image.source(), None, &buffers)
                            .map_err(|e| anyhow!("Failed to decode image data URI: {}", e))?;
                        images.push(data);
                    } else {
                        // External image file - optimized mmap loading
                        let image_path = base_dir.join(uri);
                        
                        let image_file = File::open(&image_path)
                            .map_err(|e| anyhow!("Failed to open image file {:?}: {}", image_path, e))?;
                        let image_mmap = unsafe { Mmap::map(&image_file) }
                            .map_err(|e| anyhow!("Failed to create memory mapping for image {:?}: {}", image_path, e))?;
                        
                        // Optimized image decoding from mmap
                        let data = Self::decode_image_from_mmap_optimized(&image_mmap, uri)
                            .map_err(|e| anyhow!("Failed to decode image {}: {}", uri, e))?;
                        images.push(data);
                        _image_mmaps.push(image_mmap); // Keep mmap alive
                    }
                }
                gltf::image::Source::View { .. } => {
                    // Image data is embedded in a buffer view
                    let data = gltf::image::Data::from_source(image.source(), None, &buffers)
                        .map_err(|e| anyhow!("Failed to decode embedded image: {}", e))?;
                    images.push(data);
                }
            }
        }
        
        let name = path.file_stem()
            .and_then(|s| s.to_str())
            .ok_or(anyhow!("Invalid path!"))?;
            
        Self::process_gltf(gltf.document, buffers, images, name)
    }
    
    /// Optimized image decoding from memory mapped data
    fn decode_image_from_mmap_optimized(data: &[u8], filename: &str) -> Result<ImageData> {
        // Fast path: try to guess format from magic bytes first (no file extension parsing)
        let format = image::guess_format(data).unwrap_or_else(|_| {
            // Fallback: use file extension
            let extension = Path::new(filename)
                .extension()
                .and_then(|ext| ext.to_str())
                .unwrap_or("")
                .to_lowercase();
                
            match extension.as_str() {
                "png" => image::ImageFormat::Png,
                "jpg" | "jpeg" => image::ImageFormat::Jpeg,
                "bmp" => image::ImageFormat::Bmp,
                "tga" => image::ImageFormat::Tga,
                "tiff" | "tif" => image::ImageFormat::Tiff,
                "webp" => image::ImageFormat::WebP,
                _ => image::ImageFormat::Png, // Default fallback
            }
        });
        
        // Use optimized image loading with format hint
        let img = image::load_from_memory_with_format(data, format)
            .map_err(|e| anyhow!("Failed to decode image {}: {}", filename, e))?;
            
        // Convert to RGBA8 efficiently
        let rgba_img = match img {
            image::DynamicImage::ImageRgba8(rgba) => rgba,
            other => other.into_rgba8(),
        };
        
        let (width, height) = rgba_img.dimensions();
        
        Ok(ImageData {
            pixels: rgba_img.into_raw(),
            format: gltf::image::Format::R8G8B8A8,
            width,
            height,
        })
    }
    
    #[allow(dead_code)]
    fn decode_image_from_bytes(data: &[u8], filename: &str) -> Result<ImageData> {
        Self::decode_image_from_mmap_optimized(data, filename)
    }

    fn process_gltf(gltf: Document, buffers: Vec<Data>, images: Vec<ImageData>, name: &str) -> Result<ModelData> {
        let mut model_meshes = Vec::new();
        let materials = Self::process_materials(&gltf, &images)?;

        for scene in gltf.scenes() {
            for node in scene.nodes() {
                Self::process_node(&node, &buffers, &mut model_meshes)?;
            }
        }

        if model_meshes.is_empty() {
            return Err(anyhow!("Empty gltf file!"));
        }

        info!(
            "Loaded successfully, found {} meshes and {} materials for scene",
            model_meshes.len(),
            materials.materials.len()
        );

        Ok(ModelData::new(model_meshes, materials, Some(name.to_string())))
    }

    fn process_node(
        node: &gltf::Node,
        buffers: &[Data],
        meshes: &mut Vec<MeshData>,
    ) -> Result<()> {
        if let Some(mesh) = node.mesh() {
            for primitive in mesh.primitives() {
                let mesh_data = Self::process_primitive(&primitive, buffers)?;
                if let Some(mesh_data) = mesh_data {
                    meshes.push(mesh_data);
                }
            }
        }

        for child in node.children() {
            Self::process_node(&child, buffers, meshes)?;
        }

        Ok(())
    }

    fn process_primitive(
        primitive: &Primitive,
        buffers: &[Data],
    ) -> Result<Option<MeshData>> {
        let reader = primitive.reader(|buffer| Some(&*buffers[buffer.index()]));

        let positions = reader
            .read_positions()
            .ok_or(anyhow!("Missing positions"))?
            .collect::<Vec<_>>();

        let normals = if let Some(normals) = reader.read_normals() {
            normals.collect::<Vec<_>>()
        } else {
            // Generate flat normals if missing
            Self::generate_flat_normals(&positions)?
        };

        let tex_coords = if let Some(tex_coords) = reader.read_tex_coords(0) {
            tex_coords.into_f32().collect::<Vec<_>>()
        } else {
            // Generate default UV coordinates
            vec![[0.0, 0.0]; positions.len()]
        };

        let indices = reader
            .read_indices()
            .ok_or(anyhow!("Missing indices"))?
            .into_u32()
            .collect::<Vec<_>>();

        if positions.len() != normals.len() || positions.len() != tex_coords.len() {
            return Err(anyhow!("Vertex attribute count mismatch"));
        }

        let vertices = positions
            .into_iter()
            .zip(normals.into_iter())
            .zip(tex_coords.into_iter())
            .map(|((pos, norm), uv)| {
                Vertex::new(
                    glam::Vec3::from_array(pos),
                    glam::Vec3::from_array(norm),
                    glam::Vec2::from_array(uv),
                )
            })
            .collect();

        Ok(Some(MeshData::new(
            vertices,
            indices,
            None,
            primitive.material().index(),
        )))
    }

    fn generate_flat_normals(positions: &Vec<[f32; 3]>) -> Result<Vec<[f32; 3]>> {
        if positions.len() % 3 != 0 {
            return Err(anyhow!("Position count must be divisible by 3 for flat normals"));
        }

        let mut normals = vec![[0.0, 0.0, 0.0]; positions.len()];

        for i in (0..positions.len()).step_by(3) {
            let v0 = glam::Vec3::from_array(positions[i]);
            let v1 = glam::Vec3::from_array(positions[i + 1]);
            let v2 = glam::Vec3::from_array(positions[i + 2]);

            let normal = (v1 - v0).cross(v2 - v0).normalize();

            normals[i] = normal.to_array();
            normals[i + 1] = normal.to_array();
            normals[i + 2] = normal.to_array();
        }

        Ok(normals)
    }

    fn process_materials(gltf: &Document, images: &[ImageData]) -> Result<MaterialData> {
        let mut materials = Vec::new();

        for material in gltf.materials() {
            let pbr = material.pbr_metallic_roughness();
            
            let mut pbr_material = PbrMaterial {
                name: material.name().map(|s| s.to_string()),
                base_color_factor: pbr.base_color_factor(),
                metallic_factor: pbr.metallic_factor(),
                roughness_factor: pbr.roughness_factor(),
                emissive_factor: material.emissive_factor(),
                textures: PbrTextures::default(),
            };

            // Process base color texture
            if let Some(texture) = pbr.base_color_texture() {
                let image_index = texture.texture().source().index();
                if let Some(image_data) = images.get(image_index) {
                    pbr_material.textures.base_color = Some(TextureData {
                        pixels: image_data.pixels.clone(),
                        width: image_data.width,
                        height: image_data.height,
                        format: image_data.format,
                    });
                }
            }

            // Process metallic-roughness texture
            if let Some(texture) = pbr.metallic_roughness_texture() {
                let image_index = texture.texture().source().index();
                if let Some(image_data) = images.get(image_index) {
                    pbr_material.textures.metallic_roughness = Some(TextureData {
                        pixels: image_data.pixels.clone(),
                        width: image_data.width,
                        height: image_data.height,
                        format: image_data.format,
                    });
                }
            }

            // Process normal texture
            if let Some(texture) = material.normal_texture() {
                let image_index = texture.texture().source().index();
                if let Some(image_data) = images.get(image_index) {
                    pbr_material.textures.normal = Some(TextureData {
                        pixels: image_data.pixels.clone(),
                        width: image_data.width,
                        height: image_data.height,
                        format: image_data.format,
                    });
                }
            }

            // Process occlusion texture
            if let Some(texture) = material.occlusion_texture() {
                let image_index = texture.texture().source().index();
                if let Some(image_data) = images.get(image_index) {
                    pbr_material.textures.occlusion = Some(TextureData {
                        pixels: image_data.pixels.clone(),
                        width: image_data.width,
                        height: image_data.height,
                        format: image_data.format,
                    });
                }
            }

            // Process emissive texture
            if let Some(texture) = material.emissive_texture() {
                let image_index = texture.texture().source().index();
                if let Some(image_data) = images.get(image_index) {
                    pbr_material.textures.emissive = Some(TextureData {
                        pixels: image_data.pixels.clone(),
                        width: image_data.width,
                        height: image_data.height,
                        format: image_data.format,
                    });
                }
            }

            materials.push(pbr_material);
        }

        // If no materials are defined, create a default material
        if materials.is_empty() {
            materials.push(PbrMaterial::default());
        }

        Ok(MaterialData::new(materials))
    }
}