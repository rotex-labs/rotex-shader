mod glsl;
mod package;
mod reflect;
mod slang;
mod transpile;
mod wgsl;

pub use glsl::{GlslCompiler, compile_glsl, compile_glsl_source};
pub use package::{
    compile_graphics_package, read_graphics_package, read_package, write_graphics_package,
    write_package,
};
pub use reflect::{reflect_spirv, reflect_wgsl};
pub use rotex_shader_core::{
    CompileOptions, CompilerError, ShaderCompiler, ShaderSource, merge_graphics_layout,
};
pub use rotex_types::shader::*;
pub use slang::SlangCompiler;
pub use wgsl::WgslCompiler;

#[cfg(test)]
mod tests {
    use rotex_types::BindingType;

    use super::*;

    const VERTEX_GLSL: &str = r#"
#version 450
layout(set = 0, binding = 0) uniform Frame {
    mat4 view_proj;
};
layout(location = 0) in vec3 position;
void main() {
    gl_Position = view_proj * vec4(position, 1.0);
}
"#;

    const FRAGMENT_GLSL: &str = r#"
#version 450
layout(location = 0) out vec4 frag_color;
void main() {
    frag_color = vec4(1.0);
}
"#;

    const VERTEX_WGSL: &str = r#"
struct Frame {
    view_proj: mat4x4<f32>,
}
@group(0) @binding(0) var<uniform> frame: Frame;

@vertex
fn main(@location(0) position: vec3<f32>) -> @builtin(position) vec4<f32> {
    return frame.view_proj * vec4<f32>(position, 1.0);
}
"#;

    const FRAGMENT_WGSL: &str = r#"
@fragment
fn main() -> @location(0) vec4<f32> {
    return vec4<f32>(1.0, 1.0, 1.0, 1.0);
}
"#;

    fn nonempty_text(payload: &Option<ShaderPayload>) -> bool {
        match payload {
            Some(
                ShaderPayload::Msl(source)
                | ShaderPayload::Glsl(source)
                | ShaderPayload::HlslSource(source)
                | ShaderPayload::Wgsl(source),
            ) => !source.is_empty(),
            _ => false,
        }
    }

    #[test]
    fn graphics_targets_share_one_layout() {
        let package =
            compile_graphics_package(VERTEX_GLSL, FRAGMENT_GLSL, "main", &[]).expect("compile");
        let vertex_wgsl =
            reflect_wgsl(VERTEX_WGSL, ShaderStage::Vertex, "main").expect("wgsl vertex");
        let fragment_wgsl =
            reflect_wgsl(FRAGMENT_WGSL, ShaderStage::Fragment, "main").expect("wgsl fragment");

        assert_eq!(package.vertex.layout, vertex_wgsl);
        assert_eq!(package.fragment.layout, fragment_wgsl);
        assert_eq!(
            package.vertex.layout.bind_groups[0].entries[0].ty,
            BindingType::UniformBuffer
        );

        for stage in [&package.vertex, &package.fragment] {
            assert!(nonempty_text(&stage.variants.wgsl));
            assert!(nonempty_text(&stage.variants.msl));
            assert!(nonempty_text(&stage.variants.glsl));
            assert!(nonempty_text(&stage.variants.hlsl));
            assert!(stage.variants.dxil.is_none());
            assert!(stage.spirv_bytes().is_some_and(|bytes| !bytes.is_empty()));
        }
    }
}
