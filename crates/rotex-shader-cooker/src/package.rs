use std::fs;
use std::path::Path;

use rotex_shader_core::{CompilerError, merge_graphics_layout};
use rotex_types::{GraphicsShaderPackage, ShaderPackage, ShaderStage};

use crate::glsl::compile_glsl;

/// Writes `package` to `path` as the bytes from [`ShaderPackage::to_bytes`].
///
/// The file is created or replaced. Encoding failures become
/// [`CompilerError::Serialize`]. Filesystem failures become [`CompilerError::Io`].
pub fn write_package(path: &Path, package: &ShaderPackage) -> Result<(), CompilerError> {
    let bytes = package
        .to_bytes()
        .map_err(|err| CompilerError::Serialize(err.to_string()))?;
    fs::write(path, bytes)?;
    Ok(())
}

/// Reads a [`ShaderPackage`] from the postcard bytes in `path`.
///
/// Decoding uses [`ShaderPackage::from_bytes`]. Decoding failures become
/// [`CompilerError::Serialize`]. Filesystem failures become [`CompilerError::Io`].
pub fn read_package(path: &Path) -> Result<ShaderPackage, CompilerError> {
    let bytes = fs::read(path)?;
    ShaderPackage::from_bytes(&bytes).map_err(|err| CompilerError::Serialize(err.to_string()))
}

/// Compiles GLSL vertex and fragment sources into a [`GraphicsShaderPackage`].
///
/// Both stages are compiled with [`compile_glsl`] using `entry` and `defines`.
/// [`merge_graphics_layout`] combines the stage layouts. The returned package
/// keeps each stage package as compiled and stores the merged layout as its
/// graphics layout.
pub fn compile_graphics_package(
    vertex_source: &str,
    fragment_source: &str,
    entry: &str,
    defines: &[(&str, &str)],
) -> Result<GraphicsShaderPackage, CompilerError> {
    let vertex = compile_glsl(vertex_source, ShaderStage::Vertex, entry, defines)?;
    let fragment = compile_glsl(fragment_source, ShaderStage::Fragment, entry, defines)?;
    let layout = merge_graphics_layout(&vertex, &fragment)?;
    Ok(GraphicsShaderPackage::new(vertex, fragment, layout))
}

/// Writes `package` to `path` as the bytes from [`GraphicsShaderPackage::to_bytes`].
///
/// The file is created or replaced. Encoding failures become
/// [`CompilerError::Serialize`]. Filesystem failures become [`CompilerError::Io`].
pub fn write_graphics_package(
    path: &Path,
    package: &GraphicsShaderPackage,
) -> Result<(), CompilerError> {
    let bytes = package
        .to_bytes()
        .map_err(|err| CompilerError::Serialize(err.to_string()))?;
    fs::write(path, bytes)?;
    Ok(())
}

/// Reads a [`GraphicsShaderPackage`] from the postcard bytes in `path`.
///
/// Decoding uses [`GraphicsShaderPackage::from_bytes`]. Decoding failures become
/// [`CompilerError::Serialize`]. Filesystem failures become [`CompilerError::Io`].
pub fn read_graphics_package(path: &Path) -> Result<GraphicsShaderPackage, CompilerError> {
    let bytes = fs::read(path)?;
    GraphicsShaderPackage::from_bytes(&bytes)
        .map_err(|err| CompilerError::Serialize(err.to_string()))
}
