use rotex_types::ShaderPackage;
use rotex_shader_core::{CompileOptions, CompilerError, ShaderCompiler, ShaderSource};

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
