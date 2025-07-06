use derive_more::TryInto;
use derive_more::From;
use std::marker::PhantomData;
use std::sync::Arc;
use crate::builder::{RenderGraphBuilder};
use crate::resource::{ExportedRenderGraphResource, GraphImportExportResource, GraphResource, GraphResourceDescriptor, RenderGraphResource, GraphResourceState};

#[macro_export]
macro_rules! render_graph_resource_interface {
	($($res:ident => $res_ty:ty, $res_desc:ident => $res_desc_ty:ty, $res_state:ident => $res_state_ty:ty),+) => {
        $(
            pub type $res = $res_ty;
            pub type $res_desc = $res_desc_ty;
            pub type $res_state = $res_state_ty;

            impl GraphResource for $res_ty {
                type Descriptor = $res_desc;
            }

            impl GraphResourceDescriptor for $res_desc_ty {
                type Resource = $res;
            }

            impl GraphResourceState for $res_state_ty {
                type Resource = $res;
            }

            impl GraphImportExportResource for $res_ty {
                fn import(self: Arc<Self>, name: &str, builder: &mut RenderGraphBuilder, access: impl Into<GraphResourceAccess>) -> RenderGraphResource<Self> {
                    let id = builder.initial_resources.len() as u32;
                    let uses = access.into().try_into().expect("Inconsistent import resource access!");
                    builder.initial_resources.push((name.to_owned(), self, uses).into());

                    RenderGraphResource {
                        id,
                        _marker: PhantomData,
                    }
                }

                fn export(_resource: RenderGraphResource<Self>, _builder: &mut RenderGraphBuilder, _access: impl Into<GraphResourceAccess>) -> ExportedRenderGraphResource<Self> {
                    unimplemented!()
                }
            }
        )+

        #[derive(From)]
        pub enum ResourceDescriptor {
            $(
                $res(<$res as GraphResource>::Descriptor),
            )+
        }

        #[derive(Debug, Clone, Copy, PartialEq, Eq, From, TryInto)]
        pub enum GraphResourceAccess {
            $(
                $res($res_state),
            )+
        }
	};
}

render_graph_resource_interface!(
    Buffer => wgpu::Buffer, BufferDesc => wgpu::BufferDescriptor<'static>, BufferState => wgpu::BufferUses,
    Texture => wgpu::Texture, TextureDesc => wgpu::TextureDescriptor<'static>, TextureState => wgpu::TextureUses
);
