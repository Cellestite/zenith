use std::ffi::OsStr;
use std::fs::File;
use std::path::{Path, PathBuf};
use anyhow::{anyhow};
use memmap2::Mmap;
use zenith_core::log::info;
use zenith_task::{submit, submit_after, TaskHandle};
use crate::gltf_loader::{GltfLoader, RawGltfProcessor};
use crate::{RawResourceProcessor, AssetLoadRequest, AssetType, RawResourceLoadRequest, RawResourceLoader, ASSET_REGISTRY, RawResourceLoadRequestBuilder, AssetLoadRequestBuilder, Asset, AssetUrl};
use crate::render::{Material, Mesh, MeshCollection, Texture};

fn workspace_root() -> PathBuf {
    // Get the directory where Cargo.toml for the workspace is located
    let mut current_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    loop {
        let cargo_toml = current_dir.join("Cargo.toml");
        if cargo_toml.exists() {
            if let Ok(content) = std::fs::read_to_string(&cargo_toml) {
                if content.contains("[workspace]") {
                    return current_dir;
                }
            }
        }
        if !current_dir.pop() {
            break;
        }
    }
    // Fallback to parent directory of current crate
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).parent().unwrap().to_path_buf()
}

pub struct AssetManager {
    cache_dir: PathBuf,
    content_dir: PathBuf,
}

#[derive(Debug, Clone)]
pub struct AsyncLoadTask(Vec<TaskHandle>);

impl AsyncLoadTask {
    pub fn wait(&self) {
        for handle in &self.0 {
            handle.wait();
        }
    }
}

impl AssetManager {
    pub fn new() -> Self {
        let root = workspace_root();
        Self {
            cache_dir: root.to_owned().join("cache/"),
            content_dir: root.join("content/"),
        }
    }

    pub fn request_load(&self, path: impl AsRef<Path>) -> AsyncLoadTask {
        if self.should_bake_asset(&path) {
            info!("load raw asset {:?}", path.as_ref());
            self.request_load_raw(RawResourceLoadRequestBuilder::default()
                .path(path.as_ref().to_owned())
                .build().unwrap())
        } else {
            info!("load asset {:?}", path.as_ref());
            let mut path = path.as_ref().to_owned();
            path.set_extension(MeshCollection::extension());
            self.request_load_asset(AssetLoadRequestBuilder::default()
                .url(path)
                .build().unwrap())
        }
    }

    fn should_bake_asset(&self, path: &impl AsRef<Path>) -> bool {
        let raw_path = self.content_dir.join(path.as_ref().to_owned());

        let mesh_collection = MeshCollection::new(path);
        let asset_url = mesh_collection.asset_url();
        let cached_file_path = self.cache_dir.join(asset_url.path);

        if !cached_file_path.exists() {
            return true;
        }

        let asset_metadata = match std::fs::metadata(cached_file_path) {
            Ok(metadata) => metadata,
            Err(_) => return false,
        };

        let source_metadata = match std::fs::metadata(raw_path) {
            Ok(metadata) => metadata,
            Err(_) => return false,
        };

        let asset_last_modified_time = match asset_metadata.modified() {
            Ok(time) => time,
            Err(_) => return false,
        };

        let raw_last_modified_time = match source_metadata.modified() {
            Ok(time) => time,
            Err(_) => return false,
        };

        raw_last_modified_time > asset_last_modified_time
    }

    pub fn request_load_raw(&self, load_request: RawResourceLoadRequest) -> AsyncLoadTask {
        assert_eq!(load_request.path.extension(), Some(OsStr::new("gltf")));

        let path = self.content_dir.join(&load_request.path);
        info!("{:?}", path);
        let result = GltfLoader::load_async(&path);
        
        let inner_result = result.clone();
        let dir = self.cache_dir.clone();
        let task = submit_after(move || {
            inner_result.get_result().and_then(|raw| {
                let asset_url = AssetUrl::from(load_request.path);
                RawGltfProcessor::process(raw, ASSET_REGISTRY.get().unwrap(), &asset_url, &dir)
            }).expect(&format!("Failed to process asset {:?}", path));
        }, [&result]);

        AsyncLoadTask(vec![task.into_handle()])
    }

    pub fn request_load_asset(&self, load_request: AssetLoadRequest) -> AsyncLoadTask {
        let asset_type = load_request.url.ty();

        let load_path = self.cache_dir.join(&load_request.url);
        info!("Try load baked asset: {:?}", load_path);

        // TODO: support load dependencies
        if asset_type == AssetType::MeshCollection {
            let file = File::open(&load_path)
                .map_err(|e| anyhow!("Failed to open asset {:?}: {}", load_path, e))
                .unwrap();
            let mmap = unsafe { Mmap::map(&file) }
                .map_err(|e| anyhow!("Failed to create memory mapping for file {:?}: {}", load_path, e))
                .unwrap();

            let (asset, _): (MeshCollection, usize) = bincode::serde::decode_from_slice(&mmap, bincode::config::standard())
                .expect(&format!("Failed to deserialize asset {:?}", load_path));

            let mut mesh_collection_handles = Vec::with_capacity(asset.meshes.len() + asset.materials.len());
            for mesh_url in &asset.meshes {
                mesh_collection_handles.extend(self.request_load_asset(AssetLoadRequestBuilder::default()
                    .url(mesh_url.clone())
                    .build().unwrap()).0);
            }

            for mat_url in &asset.materials {
                mesh_collection_handles.extend(self.request_load_asset(AssetLoadRequestBuilder::default()
                    .url(mat_url.clone())
                    .build().unwrap()).0);
            }

            return AsyncLoadTask(mesh_collection_handles);
        }

        let task = submit(move || {
            let file = File::open(&load_path)
                .map_err(|e| anyhow!("Failed to open asset {:?}: {}", load_path, e))
                .unwrap();
            let mmap = unsafe { Mmap::map(&file) }
                .map_err(|e| anyhow!("Failed to create memory mapping for GLTF file {:?}: {}", load_path, e))
                .unwrap();

            match asset_type {
                AssetType::Mesh => {
                    let (asset, _): (Mesh, usize) = bincode::serde::decode_from_slice(&mmap, bincode::config::standard())
                        .expect(&format!("Failed to deserialize asset {:?}", load_path));
                    ASSET_REGISTRY
                        .get()
                        .unwrap()
                        .register(load_request.url, asset);
                }
                AssetType::Texture => {
                    let (asset, _): (Texture, usize) = bincode::serde::decode_from_slice(&mmap, bincode::config::standard())
                        .expect(&format!("Failed to deserialize asset {:?}", load_path));
                    ASSET_REGISTRY
                        .get()
                        .unwrap()
                        .register(load_request.url, asset);
                }
                AssetType::Material => {
                    let (asset, _): (Material, usize) = bincode::serde::decode_from_slice(&mmap, bincode::config::standard())
                        .expect(&format!("Failed to deserialize asset {:?}", load_path));
                    ASSET_REGISTRY
                        .get()
                        .unwrap()
                        .register(load_request.url, asset);
                }
                _ => unreachable!()
            }
        });

        AsyncLoadTask(vec![task.into_handle()])
    }
}