use std::fs;
use std::path::Path;

use rotex_types::{GraphicsShaderPackage, ShaderPackage, ShaderStage};
use rotex_shader_core::{merge_graphics_layout, CompilerError};

use crate::glsl::compile_glsl;

pub fn write_package(path: &Path, package: &ShaderPackage) -> Result<(), CompilerError> {
    let bytes = package
        .to_bytes()
        .map_err(|err| CompilerError::Serialize(err.to_string()))?;
    fs::write(path, bytes)?;
    Ok(())
}

pub fn read_package(path: &Path) -> Result<ShaderPackage, CompilerError> {
    let bytes = fs::read(path)?;
    ShaderPackage::from_bytes(&bytes).map_err(|err| CompilerError::Serialize(err.to_string()))
}

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

pub fn read_graphics_package(path: &Path) -> Result<GraphicsShaderPackage, CompilerError> {
    let bytes = fs::read(path)?;
    GraphicsShaderPackage::from_bytes(&bytes).map_err(|err| CompilerError::Serialize(err.to_string()))
}
