use rotex_shader_core::{CompileOptions, CompilerError, ShaderCompiler, ShaderSource};
use rotex_types::{ShaderPackage, ShaderPayload, ShaderVariantMap};
use std::hash::{DefaultHasher, Hash, Hasher};

use crate::reflect::reflect_wgsl;

/// Compiles [`ShaderSource::Wgsl`] by reflecting it with naga.
///
/// [`ShaderCompiler::compile`] requires [`ShaderSource::Wgsl`]. The layout comes
/// from [`reflect_wgsl`] for [`CompileOptions::entry_point`].
/// [`CompileOptions::defines`] are left unused. [`ShaderPackage::source_hash`]
/// is a [`std::hash::DefaultHasher`] hash of the WGSL text, and the WGSL
/// variant holds that same text. The SPIR-V, DXIL, HLSL, MSL, and GLSL variants
/// are empty. A [`ShaderSource`] of another language returns
/// [`CompilerError::Unsupported`] with the message
/// `wgsl_compiler_requires_wgsl_source`.
pub struct WgslCompiler;

impl ShaderCompiler for WgslCompiler {
    fn compile(
        &self,
        source: &ShaderSource,
        options: &CompileOptions,
    ) -> Result<ShaderPackage, CompilerError> {
        let ShaderSource::Wgsl { source, stage } = source else {
            return Err(CompilerError::Unsupported(
                "wgsl_compiler_requires_wgsl_source".to_string(),
            ));
        };
        let layout = reflect_wgsl(source, *stage, &options.entry_point)?;
        let mut hasher = DefaultHasher::new();
        source.hash(&mut hasher);
        let source_hash = hasher.finish();
        Ok(ShaderPackage {
            source_hash,
            stage: *stage,
            entry_point: options.entry_point.clone(),
            layout,
            variants: ShaderVariantMap {
                wgsl: Some(ShaderPayload::Wgsl(source.clone())),
                ..Default::default()
            },
        })
    }
}
