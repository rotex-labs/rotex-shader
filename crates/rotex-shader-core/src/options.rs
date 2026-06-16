#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CompileOptions {
    pub entry_point: String,
    pub defines: Vec<(String, String)>,
}

impl CompileOptions {
    pub fn new(entry_point: impl Into<String>) -> Self {
        Self {
            entry_point: entry_point.into(),
            defines: Vec::new(),
        }
    }

    pub fn with_defines(mut self, defines: Vec<(String, String)>) -> Self {
        self.defines = defines;
        self
    }

    pub fn define_refs(&self) -> Vec<(&str, &str)> {
        self.defines
            .iter()
            .map(|(name, value)| (name.as_str(), value.as_str()))
            .collect()
    }
}
