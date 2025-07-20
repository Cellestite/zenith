pub mod log;
pub mod collections;
mod event;

pub mod asset_loader {
    mod gltf_loader;

    pub use gltf_loader::{GltfLoader, MeshData, ModelData};
}