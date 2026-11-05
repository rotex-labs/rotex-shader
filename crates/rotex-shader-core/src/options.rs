/// Entry point and preprocessor defines for one [`crate::ShaderCompiler::compile`] call.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CompileOptions {
    /// Name of the entry point passed to the compiler.
    pub entry_point: String,
    /// Preprocessor definitions as `(name, value)` pairs, in declaration order.
    pub defines: Vec<(String, String)>,
}

impl CompileOptions {
    /// Options whose entry point is `entry_point` and whose define list is empty.
    pub fn new(entry_point: impl Into<String>) -> Self {
        Self {
            entry_point: entry_point.into(),
            defines: Vec::new(),
        }
    }

    /// Returns `self` with [`defines`](Self::defines) replaced by `defines`.
    pub fn with_defines(mut self, defines: Vec<(String, String)>) -> Self {
        self.defines = defines;
        self
    }

    /// Borrows each define as `(&name, &value)`, in declaration order.
    pub fn define_refs(&self) -> Vec<(&str, &str)> {
        self.defines
            .iter()
            .map(|(name, value)| (name.as_str(), value.as_str()))
            .collect()
    }
}
