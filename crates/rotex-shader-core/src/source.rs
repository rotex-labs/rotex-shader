use rotex_types::ShaderStage;

/// Source text for one shader stage.
///
/// The active variant is the language. A compiler accepts the variant that
/// matches the language it implements.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ShaderSource {
    /// GLSL source.
    Glsl {
        /// GLSL source text.
        source: String,
        /// Stage this source is compiled for.
        stage: ShaderStage,
    },
    /// WGSL source.
    Wgsl {
        /// WGSL source text.
        source: String,
        /// Stage this source is compiled for.
        stage: ShaderStage,
    },
    /// Slang source.
    Slang {
        /// Slang source text.
        source: String,
        /// Stage this source is compiled for.
        stage: ShaderStage,
    },
}
