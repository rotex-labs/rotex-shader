use rotex_shader_core::{CompileOptions, CompilerError, ShaderCompiler, ShaderSource};
use rotex_types::ShaderPackage;

/// Placeholder [`ShaderCompiler`] for [`ShaderSource::Slang`].
///
/// [`ShaderCompiler::compile`] returns [`CompilerError::Unsupported`] with the
/// message `slang_not_implemented` for every source and options pair.
pub struct SlangCompiler;

impl ShaderCompiler for SlangCompiler {
    fn compile(
        &self,
        _source: &ShaderSource,
        _options: &CompileOptions,
    ) -> Result<ShaderPackage, CompilerError> {
        Err(CompilerError::Unsupported(
            "slang_not_implemented".to_string(),
        ))
    }
}
