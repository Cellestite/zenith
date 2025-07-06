use std::marker::PhantomData;
use std::sync::Arc;
use derive_more::From;
use crate::builder::{RenderGraphBuilder, ResourceAccessStorage};
use crate::interface::{Buffer, BufferState, GraphResourceAccess, ResourceDescriptor, Texture, TextureState};

pub trait GraphResource: Clone {
    type Descriptor: GraphResourceDescriptor;
}

pub trait GraphResourceDescriptor: Clone + Into<ResourceDescriptor> {
    type Resource: GraphResource;
}

pub trait GraphResourceState: Copy + Eq {
    type Resource: GraphResource;
}

pub trait GraphResourceMutability: Copy {}

#[derive(Clone, Copy)]
pub struct ReadOnly;

#[derive(Clone, Copy)]
pub struct ReadWrite;

impl GraphResourceMutability for ReadOnly {}
impl GraphResourceMutability for ReadWrite {}

/// Identifier unique represent an inner resource owned by render graph.
///
/// ## Safety
/// Used in the same render graph context. Should NOT be used across multiple render graph.
pub(crate) type GraphResourceId = u32;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct RenderGraphResource<R: GraphResource> {
    pub(crate) id: GraphResourceId,
    pub(crate) _marker: PhantomData<R>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenderGraphResourceAccess<R: GraphResource, V: GraphResourceMutability> {
    pub(crate) id: GraphResourceId,
    pub(crate) access: GraphResourceAccess,
    pub(crate) _marker: PhantomData<(R, V)>,
}

impl<R: GraphResource, V: GraphResourceMutability> RenderGraphResourceAccess<R, V> {
    pub(crate) fn into_untyped(self) -> ResourceAccessStorage<V> {
        ResourceAccessStorage {
            id: self.id,
            access: self.access,
            _marker: PhantomData,
        }
    }
}

pub trait GraphImportExportResource: GraphResource {
    fn import(self: Arc<Self>, name: &str, builder: &mut RenderGraphBuilder, access: impl Into<GraphResourceAccess>) -> RenderGraphResource<Self>;
    fn export(resource: RenderGraphResource<Self>, builder: &mut RenderGraphBuilder, access: impl Into<GraphResourceAccess>) -> ExportedRenderGraphResource<Self>;
}

pub struct ExportedRenderGraphResource<R: GraphResource> {
    #[allow(dead_code)]
    pub(crate) id: GraphResourceId,
    pub(crate) _marker: PhantomData<R>,
}

/// ## TODO
/// Generalize it using derived macro (move to interface.rs)
#[derive(From)]
pub(crate) enum InitialResourceStorage {
    ManagedBuffer(String, <Buffer as GraphResource>::Descriptor),
    ManagedTexture(String, <Texture as GraphResource>::Descriptor),
    ImportedBuffer(String, Arc<Buffer>, BufferState),
    ImportedTexture(String, Arc<Texture>, TextureState),
}

impl InitialResourceStorage {
    pub(crate) fn name(&self) -> &str {
        match self {
            InitialResourceStorage::ManagedBuffer(name, _) => &name,
            InitialResourceStorage::ManagedTexture(name, _) => &name,
            InitialResourceStorage::ImportedBuffer(name, _, _) => &name,
            InitialResourceStorage::ImportedTexture(name, _, _) => &name,
        }
    }
}

/// ## TODO
/// Generalize it using derived macro (move to interface.rs)
#[allow(dead_code)]
pub(crate) enum ExportResourceStorage {
    ExportedBuffer(BufferState),
    ExportedTexture(TextureState),
}
