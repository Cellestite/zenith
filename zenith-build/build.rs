use miette::IntoDiagnostic;
use wgsl_bindgen::{GlamWgslTypeMap, WgslBindgenOptionBuilder, WgslShaderSourceType, WgslTypeSerializeStrategy};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    WgslBindgenOptionBuilder::default()
        .workspace_root("shader")
        .add_entry_point("shader/triangle.wgsl")
        .add_entry_point("shader/mesh.wgsl")
        .serialization_strategy(WgslTypeSerializeStrategy::Bytemuck)
        .type_map(GlamWgslTypeMap)
        .shader_source_type(WgslShaderSourceType::ComposerWithRelativePath)
        .output("src/generated/shader_bindings.rs")
        .build()?
        .generate()
        .into_diagnostic()?;
    Ok(())
}