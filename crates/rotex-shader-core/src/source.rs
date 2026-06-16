use rotex_types::ShaderStage;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ShaderSource {
    Glsl {
        source: String,
        stage: ShaderStage,
    },
    Wgsl {
        source: String,
        stage: ShaderStage,
    },
    Slang {
        source: String,
        stage: ShaderStage,
    },
}
