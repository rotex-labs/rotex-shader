use crate::{CompileOptions, CompilerError, ShaderSource};
use rotex_types::ShaderPackage;

/// Compiles one [`ShaderSource`] into a [`ShaderPackage`].
pub trait ShaderCompiler {
    /// Compiles `source` using `options`.
    ///
    /// Implementations return [`CompilerError::Unsupported`] when `source` is a
    /// language they do not accept. [`CompileOptions::entry_point`] is the
    /// entry point. [`CompileOptions::defines`] are preprocessor definitions
    /// when the implementation applies them.
    fn compile(
        &self,
        source: &ShaderSource,
        options: &CompileOptions,
    ) -> Result<ShaderPackage, CompilerError>;
}
