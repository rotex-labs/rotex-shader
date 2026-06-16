use rotex_types::{ShaderPackage, ShaderPayload, ShaderVariantMap};
use rotex_shader_core::{CompileOptions, CompilerError, ShaderCompiler, ShaderSource};
use std::hash::{DefaultHasher, Hash, Hasher};

use crate::reflect::reflect_wgsl;

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
                spirv: None,
                wgsl: Some(ShaderPayload::Wgsl(source.clone())),
                dxil: None,
                hlsl: None,
            },
        })
    }
}
