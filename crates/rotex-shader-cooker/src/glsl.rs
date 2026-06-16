use rotex_types::{ShaderPackage, ShaderPayload, ShaderStage, ShaderVariantMap};
use rotex_shader_core::{CompileOptions, CompilerError, ShaderCompiler, ShaderSource};
use shaderc::{Compiler, ShaderKind};
use std::hash::{DefaultHasher, Hash, Hasher};

use crate::reflect::reflect_spirv;
use crate::transpile::transpile_spirv_to_wgsl;

pub struct GlslCompiler;

impl ShaderCompiler for GlslCompiler {
    fn compile(
        &self,
        source: &ShaderSource,
        options: &CompileOptions,
    ) -> Result<ShaderPackage, CompilerError> {
        let ShaderSource::Glsl { source, stage } = source else {
            return Err(CompilerError::Unsupported(
                "glsl_compiler_requires_glsl_source".to_string(),
            ));
        };
        compile_glsl_source(source, *stage, &options.entry_point, &define_slice(&options.defines))
    }
}

fn define_slice(defines: &[(String, String)]) -> Vec<(&str, &str)> {
    defines
        .iter()
        .map(|(name, value)| (name.as_str(), value.as_str()))
        .collect()
}

pub fn compile_glsl_source(
    source: &str,
    stage: ShaderStage,
    entry: &str,
    defines: &[(&str, &str)],
) -> Result<ShaderPackage, CompilerError> {
    let spirv_bytes = compile_glsl_to_spirv(source, stage, entry, defines)?;
    let layout = reflect_spirv(&spirv_bytes, stage, entry)?;
    let wgsl = transpile_spirv_to_wgsl(&spirv_bytes)?;

    let mut hasher = DefaultHasher::new();
    spirv_bytes.hash(&mut hasher);
    let source_hash = hasher.finish();

    Ok(ShaderPackage {
        source_hash,
        stage,
        entry_point: entry.to_string(),
        layout,
        variants: ShaderVariantMap {
            spirv: Some(ShaderPayload::SpirV(spirv_bytes)),
            wgsl: Some(ShaderPayload::Wgsl(wgsl)),
            dxil: None,
            hlsl: None,
        },
    })
}

fn compile_glsl_to_spirv(
    source: &str,
    stage: ShaderStage,
    entry: &str,
    defines: &[(&str, &str)],
) -> Result<Vec<u8>, CompilerError> {
    let kind = match stage {
        ShaderStage::Vertex => ShaderKind::Vertex,
        ShaderStage::Fragment => ShaderKind::Fragment,
        ShaderStage::Compute => ShaderKind::Compute,
    };

    let compiler = Compiler::new().map_err(|err| CompilerError::Shaderc(err.to_string()))?;
    let mut options =
        shaderc::CompileOptions::new().map_err(|err| CompilerError::Shaderc(err.to_string()))?;
    options.set_optimization_level(shaderc::OptimizationLevel::Performance);
    for (name, value) in defines {
        options.add_macro_definition(name, Some(*value));
    }

    let artifact = compiler
        .compile_into_spirv(source, kind, "shader", entry, Some(&options))
        .map_err(|err| CompilerError::Shaderc(err.to_string()))?;

    Ok(artifact.as_binary_u8().to_vec())
}

pub fn compile_glsl(
    source: &str,
    stage: ShaderStage,
    entry: &str,
    defines: &[(&str, &str)],
) -> Result<ShaderPackage, CompilerError> {
    GlslCompiler.compile(
        &ShaderSource::Glsl {
            source: source.to_string(),
            stage,
        },
        &CompileOptions::new(entry).with_defines(
            defines
                .iter()
                .map(|(name, value)| ((*name).to_string(), (*value).to_string()))
                .collect(),
        ),
    )
}
