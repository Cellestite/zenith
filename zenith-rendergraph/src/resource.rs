use std::fmt::{Debug};
use std::marker::PhantomData;
use derive_more::From;
use crate::builder::{RenderGraphBuilder, ResourceAccessStorage};
use crate::interface::{Buffer, BufferState, GraphResourceAccess, ResourceDescriptor, Texture, TextureState};
use crate::RenderResource;

pub trait GraphResource: Clone {
    type Descriptor: GraphResourceDescriptor;
}

pub trait GraphResourceDescriptor: Clone + Into<ResourceDescriptor> {
    type Resource: GraphResource;
}

pub trait GraphResourceState: Copy + Eq {
    type Resource: GraphResource;
}

pub trait GraphResourceView: Copy {}

#[derive(Clone, Copy, Debug)]
pub struct Srv;

#[derive(Clone, Copy, Debug)]
pub struct Uav;

#[derive(Clone, Copy, Debug)]
pub struct Rt;

impl GraphResourceView for Srv {}
impl GraphResourceView for Uav {}
impl GraphResourceView for Rt {}

/// Identifier unique represent an inner resource owned by render graph.
///
/// ## Safety
/// Used in the same render graph context. Should NOT be used across multiple render graph.
pub(crate) type GraphResourceId = u32;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct RenderGraphResource<R: GraphResource> {
    pub(crate) id: GraphResourceId,
    pub(crate) _marker: PhantomData<R>,
}

impl<R: GraphResource> Copy for RenderGraphResource<R> {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenderGraphResourceAccess<R: GraphResource, V: GraphResourceView> {
    pub(crate) id: GraphResourceId,
    pub(crate) access: GraphResourceAccess,
    pub(crate) _marker: PhantomData<(R, V)>,
}

impl<R: GraphResource, V: GraphResourceView> Copy for RenderGraphResourceAccess<R, V> {}

impl<R: GraphResource, V: GraphResourceView> RenderGraphResourceAccess<R, V> {
    pub(crate) fn into_untyped(self) -> ResourceAccessStorage {
        ResourceAccessStorage {
            id: self.id,
            access: self.access,
        }
    }
}

pub trait GraphImportExportResource: GraphResource {
    fn import(shared_resource: impl Into<RenderResource<Self>>, name: &str, builder: &mut RenderGraphBuilder, access: impl Into<GraphResourceAccess>) -> RenderGraphResource<Self>;
    fn export(resource: RenderGraphResource<Self>, builder: &mut RenderGraphBuilder, access: impl Into<GraphResourceAccess>) -> ExportedRenderGraphResource<Self>;
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ExportedRenderGraphResource<R: GraphResource> {
    #[allow(dead_code)]
    pub(crate) id: GraphResourceId,
    pub(crate) _marker: PhantomData<R>,
}

impl<R: GraphResource> Copy for ExportedRenderGraphResource<R> {}

#[derive(From)]
pub(crate) enum InitialResourceStorage {
    ManagedBuffer(String, <Buffer as GraphResource>::Descriptor),
    ManagedTexture(String, <Texture as GraphResource>::Descriptor),
    ImportedBuffer(String, RenderResource<Buffer>, BufferState),
    ImportedTexture(String, RenderResource<Texture>, TextureState),
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

#[allow(dead_code)]
pub(crate) enum ExportResourceStorage {
    ExportedBuffer(BufferState),
    ExportedTexture(TextureState),
}
