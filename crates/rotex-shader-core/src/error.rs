use rotex_types::ShaderStage;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum CompilerError {
    #[error("shader compilation failed: {0}")]
    Shaderc(String),
    #[error("SPIR-V reflection failed: {0}")]
    Reflection(String),
    #[error("WGSL transpile failed: {0}")]
    Transpile(String),
    #[error("entry point '{0}' not found for stage {1:?}")]
    EntryPointMissing(String, ShaderStage),
    #[error("layout validation failed: {0}")]
    LayoutValidation(String),
    #[error("unsupported: {0}")]
    Unsupported(String),
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("serialization error: {0}")]
    Serialize(String),
}
