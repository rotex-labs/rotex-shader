#![deny(missing_docs)]
#![deny(rustdoc::broken_intra_doc_links)]

//! Cooks one Slang entry point by invoking `slangc`.
//!
//! [`compile_entry`] writes SPIR-V, WGSL, desktop GLSL, MSL, and DXIL when the
//! pinned compiler accepts that target. A missing `slangc` is an error.
//! A missing DXC compiler omits the DXIL payload and records a diagnostic.
//! The crate does not link a compiler into the shipped runtime.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use rotex_shader_core::{CompilerError, ShaderStage};
use rotex_types::{Diagnostic, DiagnosticSeverity, TargetId};

/// Linux slangc release invoked when `ROTEX_SLANGC` is unset and `slangc` is not on `PATH`.
pub const SLANGC_VERSION: &str = "2026.21";

const SLANGC_URL: &str = "https://github.com/shader-slang/slang/releases/download/v2026.21/slang-2026.21-linux-x86_64.tar.gz";

/// One entry point compiled to the targets `slangc` accepted.
#[derive(Debug, Clone)]
pub struct SlangOutputs {
    /// Entry-point name requested from `slangc`.
    pub entry_point: String,
    /// Stage that was compiled.
    pub stage: ShaderStage,
    /// SPIR-V words.
    pub spirv: Vec<u8>,
    /// WGSL source, when the WGSL target succeeded.
    pub wgsl: Option<String>,
    /// Desktop GLSL source emitted by `slangc`.
    pub glsl: Option<String>,
    /// Metal source, when the Metal target succeeded.
    pub msl: Option<String>,
    /// DXIL bytes, when DXC signed the target.
    pub dxil: Option<Vec<u8>>,
    /// Target omissions and compiler notes.
    pub diagnostics: Vec<Diagnostic>,
}

/// Returns `ROTEX_SLANGC` or a `slangc` found on `PATH`.
pub fn slangc_from_env() -> Option<PathBuf> {
    if let Ok(configured) = std::env::var("ROTEX_SLANGC") {
        let candidate = PathBuf::from(configured);
        if candidate.is_file() {
            return Some(candidate);
        }
    }
    let Ok(search) = std::env::var("PATH") else {
        return None;
    };
    for directory in std::env::split_paths(&search) {
        let candidate = directory.join("slangc");
        if candidate.is_file() {
            return Some(candidate);
        }
    }
    None
}

/// Returns a usable `slangc`, downloading the pinned Linux archive into `cache_dir` when needed.
pub fn ensure_slangc(cache_dir: &Path) -> Result<PathBuf, CompilerError> {
    if let Some(found) = slangc_from_env() {
        return Ok(found);
    }
    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    {
        let binary = cache_dir.join("bin").join("slangc");
        if binary.is_file() {
            return Ok(binary);
        }
        fs::create_dir_all(cache_dir)?;
        let archive = cache_dir.join("slangc.tar.gz");
        let status = Command::new("curl")
            .args(["-fsSL", "-o"])
            .arg(&archive)
            .arg(SLANGC_URL)
            .status()?;
        if !status.success() {
            return Err(CompilerError::Unsupported(
                "failed to download slangc".to_string(),
            ));
        }
        let status = Command::new("tar")
            .args(["-xzf"])
            .arg(&archive)
            .arg("-C")
            .arg(cache_dir)
            .status()?;
        if !status.success() || !binary.is_file() {
            return Err(CompilerError::Unsupported(
                "failed to unpack slangc".to_string(),
            ));
        }
        Ok(binary)
    }
    #[cfg(not(all(target_os = "linux", target_arch = "x86_64")))]
    {
        let _ = cache_dir;
        Err(CompilerError::Unsupported(
            "set ROTEX_SLANGC to a slangc binary".to_string(),
        ))
    }
}

/// Compiles `entry` in `source` for `stage`.
///
/// SPIR-V is required. WGSL, GLSL, MSL, and DXIL are recorded when `slangc`
/// accepts them. DXIL failure is a diagnostic on [`SlangOutputs::diagnostics`]
/// and leaves [`SlangOutputs::dxil`] empty.
pub fn compile_entry(
    slangc: &Path,
    source: &str,
    stage: ShaderStage,
    entry: &str,
    defines: &[(&str, &str)],
) -> Result<SlangOutputs, CompilerError> {
    let temp = TempDir::new()?;
    let source_path = temp.0.join("shader.slang");
    fs::write(&source_path, source)?;
    let invoke = SlangInvoke {
        slangc,
        source_path: &source_path,
        stage,
        entry,
        defines,
    };
    let spirv_path = temp.0.join("stage.spv");
    invoke.run("spirv", Some("spirv_1_0"), &spirv_path, &[])?;
    let spirv = fs::read(&spirv_path)?;
    if spirv.is_empty() {
        return Err(CompilerError::Unsupported(
            "slangc produced empty SPIR-V".to_string(),
        ));
    }

    let mut diagnostics = Vec::new();
    let wgsl = invoke.optional_text(
        "wgsl",
        None,
        &temp.0.join("stage.wgsl"),
        TargetId::Wgsl,
        &["-DROTEX_WGSL=1"],
        &mut diagnostics,
    );
    let glsl = invoke.optional_text(
        "glsl",
        Some("glsl_450"),
        &temp.0.join("stage.glsl"),
        TargetId::GlslDesktop,
        &[],
        &mut diagnostics,
    );
    let msl = invoke.optional_text(
        "metal",
        None,
        &temp.0.join("stage.metal"),
        TargetId::Msl,
        &[],
        &mut diagnostics,
    );
    let dxil = invoke.optional_bytes(
        "dxil",
        Some("sm_6_6"),
        &temp.0.join("stage.dxil"),
        &mut diagnostics,
    );
    if msl.is_some() {
        diagnostics.push(Diagnostic {
            severity: DiagnosticSeverity::Note,
            code: "msl_experimental".to_string(),
            message: "Metal payload is experimental until conformance images pass".to_string(),
            target: Some(TargetId::Msl),
        });
    }

    Ok(SlangOutputs {
        entry_point: entry.to_string(),
        stage,
        spirv,
        wgsl,
        glsl,
        msl,
        dxil,
        diagnostics,
    })
}

struct SlangInvoke<'a> {
    slangc: &'a Path,
    source_path: &'a Path,
    stage: ShaderStage,
    entry: &'a str,
    defines: &'a [(&'a str, &'a str)],
}

impl SlangInvoke<'_> {
    fn optional_text(
        &self,
        target: &str,
        profile: Option<&str>,
        output: &Path,
        target_id: TargetId,
        extra: &[&str],
        diagnostics: &mut Vec<Diagnostic>,
    ) -> Option<String> {
        match self.run(target, profile, output, extra) {
            Ok(()) => fs::read_to_string(output)
                .ok()
                .filter(|text| !text.is_empty()),
            Err(err) => {
                diagnostics.push(Diagnostic {
                    severity: DiagnosticSeverity::Warning,
                    code: format!("{target}_omitted"),
                    message: err.to_string(),
                    target: Some(target_id),
                });
                None
            }
        }
    }

    fn optional_bytes(
        &self,
        target: &str,
        profile: Option<&str>,
        output: &Path,
        diagnostics: &mut Vec<Diagnostic>,
    ) -> Option<Vec<u8>> {
        match self.run(target, profile, output, &["-DROTEX_DXIL=1"]) {
            Ok(()) => fs::read(output).ok().filter(|bytes| !bytes.is_empty()),
            Err(err) => {
                diagnostics.push(Diagnostic {
                    severity: DiagnosticSeverity::Error,
                    code: "dxc_not_found".to_string(),
                    message: format!("slangc -target {target} failed: {err}"),
                    target: Some(TargetId::D3d12Dxil),
                });
                None
            }
        }
    }

    fn run(
        &self,
        target: &str,
        profile: Option<&str>,
        output: &Path,
        extra: &[&str],
    ) -> Result<(), CompilerError> {
        let mut command = Command::new(self.slangc);
        command
            .arg(self.source_path)
            .arg("-entry")
            .arg(self.entry)
            .arg("-stage")
            .arg(stage_flag(self.stage))
            .arg("-target")
            .arg(target)
            .arg("-matrix-layout-column-major")
            .arg("-fvk-use-entrypoint-name")
            .arg("-fvk-use-gl-layout")
            .arg("-O2")
            .args(extra)
            .arg("-o")
            .arg(output);
        if let Some(profile) = profile {
            command.arg("-profile").arg(profile);
        }
        for (name, value) in self.defines {
            command.arg(format!("-D{name}={value}"));
        }
        let ran = command
            .output()
            .map_err(|err| CompilerError::Unsupported(err.to_string()))?;
        if ran.status.success() {
            Ok(())
        } else {
            let stderr = String::from_utf8_lossy(&ran.stderr);
            Err(CompilerError::Unsupported(format!(
                "slangc -target {target} failed: {stderr}"
            )))
        }
    }
}

fn stage_flag(stage: ShaderStage) -> &'static str {
    match stage {
        ShaderStage::Vertex => "vertex",
        ShaderStage::Fragment => "fragment",
        ShaderStage::Compute => "compute",
    }
}

struct TempDir(PathBuf);

impl TempDir {
    fn new() -> Result<Self, CompilerError> {
        let directory = std::env::temp_dir().join(format!(
            "rotex-slang-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|duration| duration.as_nanos())
                .unwrap_or(0)
        ));
        fs::create_dir_all(&directory)?;
        Ok(Self(directory))
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
