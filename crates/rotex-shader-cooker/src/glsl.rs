use rotex_shader_core::{CompileOptions, CompilerError, ShaderCompiler, ShaderSource};
use rotex_types::{ShaderPackage, ShaderPayload, ShaderStage, ShaderVariantMap};
use shaderc::{Compiler, ShaderKind};
use std::hash::{DefaultHasher, Hash, Hasher};

use crate::transpile::emit_from_spirv;

/// Compiles [`ShaderSource::Glsl`] through shaderc.
///
/// [`ShaderCompiler::compile`] requires [`ShaderSource::Glsl`] and forwards the
/// entry point and defines to [`compile_glsl_source`]. A [`ShaderSource`] of
/// another language returns [`CompilerError::Unsupported`] with the message
/// `glsl_compiler_requires_glsl_source`.
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
        compile_glsl_source(
            source,
            *stage,
            &options.entry_point,
            &define_slice(&options.defines),
        )
    }
}

fn define_slice(defines: &[(String, String)]) -> Vec<(&str, &str)> {
    defines
        .iter()
        .map(|(name, value)| (name.as_str(), value.as_str()))
        .collect()
}

/// Compiles GLSL `source` for `stage` into a [`ShaderPackage`].
///
/// shaderc compiles the text to SPIR-V for the vertex, fragment, or compute
/// kind that matches `stage`. The compile uses shaderc's performance
/// optimization level, the entry point `entry`, and the file name `shader`.
/// Each `(name, value)` pair in `defines` is added as a macro definition.
///
/// The SPIR-V is validated with naga and written out as WGSL, MSL 3.1, desktop
/// GLSL 450, and HLSL. [`ShaderPackage::source_hash`] is a
/// [`std::hash::DefaultHasher`] hash of the SPIR-V bytes. The package stores
/// that SPIR-V plus the four text variants. The DXIL variant is empty.
///
/// shaderc failures return [`CompilerError::Shaderc`]. A SPIR-V parse failure
/// returns [`CompilerError::Reflection`]. A missing entry point returns
/// [`CompilerError::EntryPointMissing`]. Validation or backend write failures
/// return [`CompilerError::Transpile`].
pub fn compile_glsl_source(
    source: &str,
    stage: ShaderStage,
    entry: &str,
    defines: &[(&str, &str)],
) -> Result<ShaderPackage, CompilerError> {
    let spirv_bytes = compile_glsl_to_spirv(source, stage, entry, defines)?;
    let emitted = emit_from_spirv(&spirv_bytes, stage, entry)?;

    let mut hasher = DefaultHasher::new();
    spirv_bytes.hash(&mut hasher);
    let source_hash = hasher.finish();

    Ok(ShaderPackage {
        source_hash,
        stage,
        entry_point: entry.to_string(),
        layout: emitted.layout,
        variants: ShaderVariantMap {
            spirv: Some(ShaderPayload::SpirV(spirv_bytes)),
            wgsl: Some(ShaderPayload::Wgsl(emitted.wgsl)),
            dxil: None,
            hlsl: Some(ShaderPayload::HlslSource(emitted.hlsl)),
            msl: Some(ShaderPayload::Msl(emitted.msl)),
            glsl: Some(ShaderPayload::Glsl(emitted.glsl)),
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

/// Compiles GLSL `source` for `stage` with [`GlslCompiler`].
///
/// `entry` becomes the [`CompileOptions`] entry point, and `defines` are copied
/// into [`CompileOptions::defines`]. The result matches [`compile_glsl_source`].
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
