use std::path::Path;
use std::process::Command;

use rotex_shader_core::{
    CompileOptions, CompilerError, ShaderCompiler, ShaderSource, artifact_key,
    check_variant_budget, merge_graphics_layout,
};
use rotex_shader_slang::{compile_entry, slangc_from_env};
use rotex_types::{
    Diagnostic, DiagnosticSeverity, PhysicalRemap, ProgramLayout, ShaderArtifact, ShaderEntry,
    ShaderPackage, ShaderStage, TargetId, TargetPayload, required_capabilities,
};

use crate::reflect::reflect_spirv;
use crate::transpile::glsl_from_spirv;

/// Compiles [`ShaderSource::Slang`] by invoking `slangc`.
///
/// [`ShaderCompiler::compile`] requires `slangc` on `PATH` or in `ROTEX_SLANGC`.
/// It does not download a compiler. A missing compiler returns
/// [`CompilerError::Unsupported`] with the message `slangc_not_found`.
pub struct SlangCompiler;

impl ShaderCompiler for SlangCompiler {
    fn compile(
        &self,
        source: &ShaderSource,
        options: &CompileOptions,
    ) -> Result<ShaderPackage, CompilerError> {
        let ShaderSource::Slang { source, stage } = source else {
            return Err(CompilerError::Unsupported(
                "slang_compiler_requires_slang_source".to_string(),
            ));
        };
        let slangc = slangc_from_env()
            .ok_or_else(|| CompilerError::Unsupported("slangc_not_found".to_string()))?;
        let defines = define_slice(&options.defines);
        let artifact = cook_stage(&slangc, source, *stage, &options.entry_point, &defines, &[])?;
        artifact
            .to_shader_package(*stage, false)
            .map_err(|err| CompilerError::Serialize(err.to_string()))
    }
}

/// Cooks a vertex and fragment entry in one Slang file into a [`ShaderArtifact`].
pub fn cook_graphics(
    slangc: &Path,
    source: &str,
    vertex_entry: &str,
    fragment_entry: &str,
    defines: &[(&str, &str)],
    keywords: &[&str],
) -> Result<ShaderArtifact, CompilerError> {
    check_variant_budget(keywords, rotex_shader_core::DEFAULT_VARIANT_BUDGET)?;
    let vertex = cook_stage(
        slangc,
        source,
        ShaderStage::Vertex,
        vertex_entry,
        defines,
        keywords,
    )?;
    let fragment = cook_stage(
        slangc,
        source,
        ShaderStage::Fragment,
        fragment_entry,
        defines,
        keywords,
    )?;
    let merged = merge_graphics_layout(
        &package_with_layout(ShaderStage::Vertex, vertex.layout.to_abstract()),
        &package_with_layout(ShaderStage::Fragment, fragment.layout.to_abstract()),
    )?;
    let mut payloads = vertex.payloads;
    payloads.extend(fragment.payloads);
    for payload in &mut payloads {
        payload.remap = PhysicalRemap::from_layout(payload.target, &merged);
        payload.required_caps = required_capabilities(payload.stage, &merged);
    }
    let mut diagnostics = vertex.diagnostics;
    diagnostics.extend(fragment.diagnostics);
    let key = artifact_key(
        source,
        defines,
        &[
            (vertex_entry, ShaderStage::Vertex),
            (fragment_entry, ShaderStage::Fragment),
        ],
    );
    Ok(ShaderArtifact {
        version: rotex_types::ARTIFACT_VERSION,
        key,
        layout: ProgramLayout::from_abstract(&merged),
        entries: vec![
            ShaderEntry {
                stage: ShaderStage::Vertex,
                name: vertex_entry.to_string(),
            },
            ShaderEntry {
                stage: ShaderStage::Fragment,
                name: fragment_entry.to_string(),
            },
        ],
        payloads,
        diagnostics,
    })
}

/// Cooks one compute entry in a Slang file into a [`ShaderArtifact`].
pub fn cook_compute(
    slangc: &Path,
    source: &str,
    entry: &str,
    defines: &[(&str, &str)],
    keywords: &[&str],
) -> Result<ShaderArtifact, CompilerError> {
    cook_stage(
        slangc,
        source,
        ShaderStage::Compute,
        entry,
        defines,
        keywords,
    )
}

fn cook_stage(
    slangc: &Path,
    source: &str,
    stage: ShaderStage,
    entry: &str,
    defines: &[(&str, &str)],
    keywords: &[&str],
) -> Result<ShaderArtifact, CompilerError> {
    check_variant_budget(keywords, rotex_shader_core::DEFAULT_VARIANT_BUDGET)?;
    let compiled = compile_entry(slangc, source, stage, entry, defines)?;
    let layout = reflect_spirv(&compiled.spirv, stage, entry)?;
    let program = ProgramLayout::from_abstract(&layout);
    let mut diagnostics = compiled.diagnostics;
    validate_spirv(&compiled.spirv, &mut diagnostics)?;
    let mut payloads = Vec::new();
    payloads.push(payload_from_bytes(
        TargetId::VulkanSpirv,
        stage,
        entry,
        compiled.spirv.clone(),
        &layout,
        false,
    ));
    if let Some(wgsl) = &compiled.wgsl {
        payloads.push(payload_from_text(
            TargetId::Wgsl,
            stage,
            entry,
            wgsl,
            &layout,
            false,
        ));
    }
    match glsl_from_spirv(&compiled.spirv, stage, entry) {
        Ok((desktop, es)) => {
            diagnostics.push(Diagnostic {
                severity: DiagnosticSeverity::Note,
                code: "glsl_from_spirv".to_string(),
                message: "GLSL payloads are translated from Slang SPIR-V so the matrix layout matches the host"
                    .to_string(),
                target: Some(TargetId::GlslDesktop),
            });
            payloads.push(payload_from_text(
                TargetId::GlslDesktop,
                stage,
                entry,
                &desktop,
                &layout,
                false,
            ));
            if let Some(reason) = program.webgl2_rejection(stage) {
                diagnostics.push(Diagnostic {
                    severity: DiagnosticSeverity::Error,
                    code: "webgl2_rejected".to_string(),
                    message: reason.to_string(),
                    target: Some(TargetId::GlslEs300),
                });
            } else if let Some(es) = es {
                payloads.push(payload_from_text(
                    TargetId::GlslEs300,
                    stage,
                    entry,
                    &es,
                    &layout,
                    stage == ShaderStage::Vertex,
                ));
            }
        }
        Err(err) => {
            diagnostics.push(Diagnostic {
                severity: DiagnosticSeverity::Warning,
                code: "glsl_from_spirv_failed".to_string(),
                message: err.to_string(),
                target: Some(TargetId::GlslDesktop),
            });
            if let Some(glsl) = &compiled.glsl {
                payloads.push(payload_from_text(
                    TargetId::GlslDesktop,
                    stage,
                    entry,
                    glsl,
                    &layout,
                    false,
                ));
            }
        }
    }
    if let Some(msl) = &compiled.msl {
        payloads.push(payload_from_text(
            TargetId::Msl,
            stage,
            entry,
            msl,
            &layout,
            false,
        ));
    }
    if let Some(dxil) = &compiled.dxil {
        payloads.push(payload_from_bytes(
            TargetId::D3d12Dxil,
            stage,
            entry,
            dxil.clone(),
            &layout,
            false,
        ));
        diagnostics.extend(validate_dxil(dxil));
    }
    let key = artifact_key(source, defines, &[(entry, stage)]);
    Ok(ShaderArtifact {
        version: rotex_types::ARTIFACT_VERSION,
        key,
        layout: program,
        entries: vec![ShaderEntry {
            stage,
            name: entry.to_string(),
        }],
        payloads,
        diagnostics,
    })
}

fn package_with_layout(
    stage: ShaderStage,
    layout: rotex_types::AbstractPipelineLayout,
) -> ShaderPackage {
    ShaderPackage {
        source_hash: 0,
        stage,
        entry_point: String::new(),
        layout,
        variants: rotex_types::ShaderVariantMap::default(),
    }
}

fn payload_from_bytes(
    target: TargetId,
    stage: ShaderStage,
    entry: &str,
    bytes: Vec<u8>,
    layout: &rotex_types::AbstractPipelineLayout,
    clip_space_fixup: bool,
) -> TargetPayload {
    TargetPayload {
        target,
        stage,
        entry_point: entry.to_string(),
        bytes,
        remap: PhysicalRemap::from_layout(target, layout),
        required_caps: required_capabilities(stage, layout),
        clip_space_fixup,
    }
}

fn payload_from_text(
    target: TargetId,
    stage: ShaderStage,
    entry: &str,
    text: &str,
    layout: &rotex_types::AbstractPipelineLayout,
    clip_space_fixup: bool,
) -> TargetPayload {
    payload_from_bytes(
        target,
        stage,
        entry,
        text.as_bytes().to_vec(),
        layout,
        clip_space_fixup,
    )
}

fn validate_spirv(bytes: &[u8], diagnostics: &mut Vec<Diagnostic>) -> Result<(), CompilerError> {
    let path = std::env::temp_dir().join(format!("rotex-spirv-{}.spv", std::process::id()));
    if std::fs::write(&path, bytes).is_err() {
        diagnostics.push(note(
            "spirv_val_skipped",
            "could not write SPIR-V for spirv-val",
        ));
        return Ok(());
    }
    let result = Command::new("spirv-val").arg(&path).output();
    let _ = std::fs::remove_file(&path);
    match result {
        Err(_) => {
            diagnostics.push(note(
                "spirv_val_not_found",
                "spirv-val is not on PATH; SPIR-V was not validated",
            ));
            Ok(())
        }
        Ok(output) if output.status.success() => Ok(()),
        Ok(output) => Err(CompilerError::Transpile(
            String::from_utf8_lossy(&output.stderr).to_string(),
        )),
    }
}

fn validate_dxil(bytes: &[u8]) -> Vec<Diagnostic> {
    let path = std::env::temp_dir().join(format!("rotex-dxil-{}.dxil", std::process::id()));
    if std::fs::write(&path, bytes).is_err() {
        return vec![note("dxc_val_skipped", "could not write DXIL for dxc")];
    }
    let result = Command::new("dxc").arg("-dumpbin").arg(&path).output();
    let _ = std::fs::remove_file(&path);
    match result {
        Err(_) => vec![note(
            "dxc_validator_not_found",
            "dxc is not on PATH; DXIL was not validated",
        )],
        Ok(output) if output.status.success() => Vec::new(),
        Ok(output) => vec![Diagnostic {
            severity: DiagnosticSeverity::Error,
            code: "dxc_validation_failed".to_string(),
            message: String::from_utf8_lossy(&output.stderr).to_string(),
            target: Some(TargetId::D3d12Dxil),
        }],
    }
}

fn note(code: &str, message: &str) -> Diagnostic {
    Diagnostic {
        severity: DiagnosticSeverity::Warning,
        code: code.to_string(),
        message: message.to_string(),
        target: Some(TargetId::VulkanSpirv),
    }
}

fn define_slice(defines: &[(String, String)]) -> Vec<(&str, &str)> {
    defines
        .iter()
        .map(|(name, value)| (name.as_str(), value.as_str()))
        .collect()
}
