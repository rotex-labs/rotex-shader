#![deny(missing_docs)]
#![deny(rustdoc::broken_intra_doc_links)]

//! Shared vocabulary for compiling shaders into packages.
//!
//! [`ShaderSource`] holds source text and a [`ShaderStage`]. [`CompileOptions`]
//! holds the entry point and preprocessor defines. A [`ShaderCompiler`] turns
//! that pair into a [`ShaderPackage`] or a [`CompilerError`].
//! [`merge_graphics_layout`] combines the layouts from a vertex package and a
//! fragment package into one [`AbstractPipelineLayout`].
//!
//! [`ShaderStage`], [`ShaderPackage`], [`ShaderPayload`], [`ShaderVariantMap`],
//! [`GraphicsShaderPackage`], [`AbstractPipelineLayout`], and
//! [`PushConstantRange`] are re-exported from `rotex-types`.

mod budget;
mod compiler;
mod error;
mod key;
mod layout;
mod options;
mod reload;
mod source;

pub use budget::{DEFAULT_VARIANT_BUDGET, check_variant_budget};
pub use compiler::ShaderCompiler;
pub use error::CompilerError;
pub use key::artifact_key;
pub use layout::merge_graphics_layout;
pub use options::CompileOptions;
pub use reload::ReloadTransaction;
pub use rotex_types::shader::*;
pub use source::ShaderSource;
