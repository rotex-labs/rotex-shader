use rotex_types::ShaderStage;
use thiserror::Error;

/// Failure from compilation, reflection, layout merge, or package I/O.
#[derive(Debug, Error)]
pub enum CompilerError {
    /// The shader compiler rejected the source.
    ///
    /// The string is the compiler diagnostic.
    #[error("shader compilation failed: {0}")]
    Shaderc(
        /// Compiler diagnostic text.
        String,
    ),
    /// Parsing or reflection failed.
    ///
    /// SPIR-V and WGSL parse errors both use this variant. The string is the
    /// diagnostic.
    #[error("SPIR-V reflection failed: {0}")]
    Reflection(
        /// Parser or reflection diagnostic text.
        String,
    ),
    /// Validation or a text-backend write failed.
    ///
    /// Covers module validation and WGSL, MSL, GLSL, and HLSL output. The
    /// string is the diagnostic.
    #[error("WGSL transpile failed: {0}")]
    Transpile(
        /// Validation or transpile diagnostic text.
        String,
    ),
    /// The requested entry point does not exist for this stage.
    #[error("entry point '{0}' not found for stage {1:?}")]
    EntryPointMissing(
        /// Entry point name that was requested.
        String,
        /// Stage that was searched.
        ShaderStage,
    ),
    /// A merged graphics layout is inconsistent.
    ///
    /// The string describes a duplicate binding or a gap between bind-group sets.
    #[error("layout validation failed: {0}")]
    LayoutValidation(
        /// Description of the layout conflict.
        String,
    ),
    /// This compiler does not support the source or operation.
    ///
    /// The string names the unsupported case.
    #[error("unsupported: {0}")]
    Unsupported(
        /// Name of the unsupported case.
        String,
    ),
    /// Declared keywords expand past the variant budget.
    #[error("variant budget exceeded: {0} combinations, budget is {1}")]
    VariantBudget(
        /// Number of combinations the keywords would produce.
        u32,
        /// Maximum combinations the cook allows.
        u32,
    ),
    /// An I/O operation failed.
    #[error("I/O error: {0}")]
    Io(
        /// The underlying I/O error.
        #[from]
        std::io::Error,
    ),
    /// Encoding or decoding a package failed.
    ///
    /// The string is the serializer diagnostic.
    #[error("serialization error: {0}")]
    Serialize(
        /// Serializer diagnostic text.
        String,
    ),
}
