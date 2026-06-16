use crate::{CompileOptions, CompilerError, ShaderSource};
use rotex_types::ShaderPackage;

pub trait ShaderCompiler {
    fn compile(
        &self,
        source: &ShaderSource,
        options: &CompileOptions,
    ) -> Result<ShaderPackage, CompilerError>;
}
