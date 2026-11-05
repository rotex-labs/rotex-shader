use std::cell::RefCell;
use std::fmt;
use std::rc::Rc;

use rotex_shader_core::CompilerError;
use rotex_types::ShaderStage;

use crate::reflect::{parse_spirv, reflect_module};

pub struct EmittedSources {
    pub wgsl: String,
    pub msl: String,
    pub glsl: String,
    pub glsl_es: Option<String>,
    pub hlsl: String,
    pub layout: rotex_types::AbstractPipelineLayout,
}

pub fn emit_from_spirv(
    spirv_bytes: &[u8],
    stage: ShaderStage,
    entry: &str,
) -> Result<EmittedSources, CompilerError> {
    let module = parse_spirv(spirv_bytes)?;
    let layout = reflect_module(&module, stage, entry)?;
    let naga_stage = naga_stage(stage);
    let info = validate(&module)?;
    let glsl_es = write_glsl(
        &module,
        &info,
        naga_stage,
        entry,
        naga::back::glsl::Version::Embedded {
            version: 300,
            is_webgl: true,
        },
    )
    .ok();
    Ok(EmittedSources {
        wgsl: write_wgsl(&module, &info)?,
        msl: write_msl(&module, &info, naga_stage, entry)?,
        glsl: write_glsl(
            &module,
            &info,
            naga_stage,
            entry,
            naga::back::glsl::Version::Desktop(450),
        )?,
        glsl_es,
        hlsl: write_hlsl(&module, &info, naga_stage, entry)?,
        layout,
    })
}

fn target_capabilities() -> naga::valid::Capabilities {
    naga::valid::Capabilities::IMMEDIATES
        | naga::valid::Capabilities::CLIP_DISTANCE
        | naga::valid::Capabilities::CULL_DISTANCE
        | naga::valid::Capabilities::CUBE_ARRAY_TEXTURES
        | naga::valid::Capabilities::MULTISAMPLED_SHADING
        | naga::valid::Capabilities::TEXTURE_AND_SAMPLER_BINDING_ARRAY
        | naga::valid::Capabilities::BUFFER_BINDING_ARRAY
        | naga::valid::Capabilities::STORAGE_TEXTURE_BINDING_ARRAY
        | naga::valid::Capabilities::STORAGE_BUFFER_BINDING_ARRAY
}

fn validate(module: &naga::Module) -> Result<naga::valid::ModuleInfo, CompilerError> {
    let mut validator =
        naga::valid::Validator::new(naga::valid::ValidationFlags::all(), target_capabilities());
    validator
        .validate(module)
        .map_err(|err| CompilerError::Transpile(err.to_string()))
}

fn write_wgsl(
    module: &naga::Module,
    info: &naga::valid::ModuleInfo,
) -> Result<String, CompilerError> {
    naga::back::wgsl::write_string(module, info, naga::back::wgsl::WriterFlags::empty())
        .map_err(|err| CompilerError::Transpile(err.to_string()))
}

fn write_msl(
    module: &naga::Module,
    info: &naga::valid::ModuleInfo,
    stage: naga::ShaderStage,
    entry: &str,
) -> Result<String, CompilerError> {
    let options = naga::back::msl::Options {
        lang_version: (3, 1),
        fake_missing_bindings: false,
        ..Default::default()
    };
    let pipeline = naga::back::msl::PipelineOptions {
        entry_point: Some((stage, entry.to_string())),
        ..Default::default()
    };
    naga::back::msl::write_string(module, info, &options, &pipeline)
        .map(|(source, _)| source)
        .map_err(|err| CompilerError::Transpile(err.to_string()))
}

fn write_glsl(
    module: &naga::Module,
    info: &naga::valid::ModuleInfo,
    stage: naga::ShaderStage,
    entry: &str,
    version: naga::back::glsl::Version,
) -> Result<String, CompilerError> {
    let options = naga::back::glsl::Options {
        version,
        ..Default::default()
    };
    let pipeline = naga::back::glsl::PipelineOptions {
        shader_stage: stage,
        entry_point: entry.to_string(),
        multiview: None,
    };
    let captured = Rc::new(RefCell::new(String::new()));
    let mut writer = naga::back::glsl::Writer::new(
        SharedWriter(Rc::clone(&captured)),
        module,
        info,
        &options,
        &pipeline,
        naga::proc::BoundsCheckPolicies::default(),
    )
    .map_err(|err| CompilerError::Transpile(err.to_string()))?;
    writer
        .write()
        .map_err(|err| CompilerError::Transpile(err.to_string()))?;
    Ok(captured.borrow().clone())
}

fn write_hlsl(
    module: &naga::Module,
    info: &naga::valid::ModuleInfo,
    stage: naga::ShaderStage,
    entry: &str,
) -> Result<String, CompilerError> {
    let options = naga::back::hlsl::Options {
        fake_missing_bindings: false,
        ..Default::default()
    };
    let pipeline = naga::back::hlsl::PipelineOptions {
        entry_point: Some((stage, entry.to_string())),
    };
    let captured = Rc::new(RefCell::new(String::new()));
    let mut writer =
        naga::back::hlsl::Writer::new(SharedWriter(Rc::clone(&captured)), &options, &pipeline);
    writer
        .write(module, info, None)
        .map_err(|err| CompilerError::Transpile(err.to_string()))?;
    Ok(captured.borrow().clone())
}

pub fn glsl_from_spirv(
    spirv_bytes: &[u8],
    stage: ShaderStage,
    entry: &str,
) -> Result<(String, Option<String>), CompilerError> {
    let module = parse_spirv(spirv_bytes)?;
    let info = validate(&module)?;
    let naga_stage = naga_stage(stage);
    let desktop = write_glsl(
        &module,
        &info,
        naga_stage,
        entry,
        naga::back::glsl::Version::Desktop(450),
    )?;
    let es = write_glsl(
        &module,
        &info,
        naga_stage,
        entry,
        naga::back::glsl::Version::Embedded {
            version: 300,
            is_webgl: true,
        },
    )
    .ok();
    Ok((desktop, es))
}

fn naga_stage(stage: ShaderStage) -> naga::ShaderStage {
    match stage {
        ShaderStage::Vertex => naga::ShaderStage::Vertex,
        ShaderStage::Fragment => naga::ShaderStage::Fragment,
        ShaderStage::Compute => naga::ShaderStage::Compute,
    }
}

struct SharedWriter(Rc<RefCell<String>>);

impl fmt::Write for SharedWriter {
    fn write_str(&mut self, text: &str) -> fmt::Result {
        self.0.borrow_mut().push_str(text);
        Ok(())
    }
}
