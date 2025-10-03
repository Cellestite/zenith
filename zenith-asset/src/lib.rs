//! Load -> (RawAsset) -> Process into engine format -> (Asset) -> store to disk

use std::any::{Any, TypeId};
use std::fs::File;
use std::io::Write;
use std::marker::PhantomData;
use std::ops::Deref;
use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock};
use anyhow::{anyhow, Result};
use bincode::Encode;
use derive_builder::Builder;
use derive_more::From;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use zenith_core::collections::hashmap::HashMap;
use zenith_task::TaskResult;

pub mod render;
pub mod manager;
pub mod gltf_loader;

static ASSET_REGISTRY: OnceLock<AssetRegistry> = OnceLock::new();

pub fn initialize() -> Result<()> {
    ASSET_REGISTRY.set(AssetRegistry::new()).map_err(|_| anyhow!("Failed to initialize asset registry!"))
}

type AssetMap = HashMap<(AssetUrl, TypeId), Arc<dyn Asset>>;

pub struct AssetRegistry {
    assets_map: RwLock<AssetMap>,
}

unsafe impl Send for AssetRegistry {}
unsafe impl Sync for AssetRegistry {}

impl Default for AssetRegistry {
    fn default() -> Self {
        Self {
            assets_map: Default::default(),
        }
    }
}

impl AssetRegistry {
    pub fn new() -> Self {
        Self {
            ..Default::default()
        }
    }

    pub fn register<A: Asset>(&self, url: impl Into<AssetUrl>, asset: A) {
        let key = (url.into(), TypeId::of::<A>());
        self.assets_map.write().insert(key, Arc::new(asset));
    }

    pub fn unregister<A: Asset>(&self, url: impl Into<AssetUrl>) -> bool {
        let key = (url.into(), TypeId::of::<A>());
        self.assets_map.write().remove(&key).is_some()
    }

    // TODO: versioned asset
    pub fn reload<A: Asset>(&self, url: impl Into<AssetUrl>, new_asset: A) -> bool {
        let key = (url.into(), TypeId::of::<A>());

        // Only replace if it already exists and has the same type
        let mut assets = self.assets_map.write();
        if assets.contains_key(&key) {
            assets.insert(key, Arc::new(new_asset));
            true
        } else {
            false
        }
    }

    fn get<A: Asset>(&self, url: AssetUrl) -> Option<AssetRef<'_, A>> {
        let assets = self.assets_map.read();
        let key = (url, TypeId::of::<A>());

        assets.get(&key)
            .map(Arc::clone)
            .and_then(AssetRef::new)
    }
}

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
pub enum AssetType {
    Mesh,
    Texture,
    Material,
    MeshCollection,
}

fn asset_type_extension(ty: AssetType) -> &'static str {
    match ty {
        AssetType::Mesh => "mesh",
        AssetType::Texture => "tex",
        AssetType::Material => "mat",
        AssetType::MeshCollection => "mscl",
    }
}

fn extension_asset_type(extension: &str) -> AssetType {
    match extension {
        "mesh" => AssetType::Mesh,
        "tex" => AssetType::Texture,
        "mat" => AssetType::Material,
        "mscl" => AssetType::MeshCollection,
        _ => unreachable!()
    }
}

impl AssetType {
    pub fn extension(&self) -> &str {
        asset_type_extension(*self)
    }
}

#[derive(Clone, Debug, From, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub struct AssetUrl {
    path: PathBuf,
}

impl AssetUrl {
    pub fn invalid() -> Self {
        Self {
            path: Default::default(),
        }
    }

    pub fn ty(&self) -> AssetType {
        let extension = self
            .path
            .extension()
            .and_then(|os_str| os_str.to_str())
            .map(|str| str.to_lowercase())
            .unwrap_or("unknown".to_owned());
        extension_asset_type(&extension)
    }
}

impl AsRef<Path> for AssetUrl {
    fn as_ref(&self) -> &Path {
        self.path.as_path()
    }
}

pub struct AssetHandle<A> {
    url: AssetUrl,
    _marker: PhantomData<A>,
}

impl<A: Asset> AssetHandle<A> {
    pub fn invalid() -> Self {
        Self {
            url: AssetUrl::invalid(),
            _marker: PhantomData,
        }
    }
    
    pub fn new(url: AssetUrl) -> Self {
        Self {
            url,
            _marker: PhantomData,
        }
    }

    pub fn get(&self) -> Option<AssetRef<'_, A>> {
        ASSET_REGISTRY.get().unwrap().get(self.url.clone())
    }
}

pub struct AssetRef<'a, T> {
    asset: Arc<dyn Asset>,
    _marker: PhantomData<&'a T>,
}

impl<'a, T: Asset> AssetRef<'a, T> {
    fn new(asset: Arc<dyn Asset>) -> Option<Self> {
        Some(Self {
            asset,
            _marker: PhantomData,
        })
    }
}

impl<'a, T: Asset> Deref for AssetRef<'a, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.asset.as_ref().as_any().downcast_ref::<T>().unwrap()
    }
}

impl<'a, T: Asset> AsRef<T> for AssetRef<'a, T> {
    fn as_ref(&self) -> &T {
        &self
    }
}

pub trait Asset: Any + Send + Sync {
    fn as_any(&self) -> &dyn Any;
    fn url(&self, name: &str) -> AssetUrl;
    fn extension() -> &'static str where Self: Sized;
}

#[derive(Clone, Debug, Builder)]
#[builder(setter(into))]
pub struct RawResourceLoadRequest {
    // relative path in the 'content' folder
    path: PathBuf,
}

pub trait RawResource: Sized {
    fn load_path(&self) -> &Path;
}

/// Stateless loader
pub trait RawResourceLoader {
    type Raw: RawResource;

    fn load(path: &Path) -> Result<Self::Raw>;
    fn load_async(path: &Path) -> TaskResult<Result<Self::Raw>>;
}

#[derive(Clone, Debug, Builder)]
#[builder(setter(into))]
pub struct AssetLoadRequest {
    url: AssetUrl,
}

/// Stateless processor
pub trait RawResourceProcessor {
    type Raw: RawResource;

    fn process(raw: Self::Raw, registry: &AssetRegistry, url: &AssetUrl, directory: &PathBuf) -> Result<()>;
}

fn serialize_asset<A: Asset + Encode>(asset: &A, absolute_path: impl Into<PathBuf>) -> Result<()> {
    let absolute_path = absolute_path.into();
    if let Some(parent) = absolute_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let config = bincode::config::standard();
    let encoded_data = bincode::encode_to_vec(asset, config)?;

    let mut file = File::create(absolute_path)?;
    file.write_all(&encoded_data)?;
    file.flush()?;

    Ok(())
}

// fn deserialize_asset<A: Asset + Encode>(asset: &A, absolute_path: impl Into<PathBuf>) -> Result<()> {
//     let absolute_path = absolute_path.into();
//     if let Some(parent) = absolute_path.parent() {
//         std::fs::create_dir_all(parent)?;
//     }
//
//     let config = bincode::config::standard();
//     let encoded_data = bincode::encode_to_vec(asset, config)?;
//
//     let mut file = File::create(absolute_path)?;
//     file.write_all(&encoded_data)?;
//     file.flush()?;
//
//     Ok(())
// }