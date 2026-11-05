use rotex_types::{
    AbstractPipelineLayout, BindGroupLayoutDescriptor, BindGroupLayoutEntry, BindingType,
    ShaderStage, ShaderStageFlags,
};

use rotex_shader_core::CompilerError;

/// Builds an [`AbstractPipelineLayout`] from SPIR-V `spirv_bytes` for `entry` at `stage`.
///
/// The bytes are parsed with naga's default SPIR-V options. Binding and
/// push-constant rules match [`reflect_wgsl`].
///
/// A global with a resource binding becomes one [`BindGroupLayoutEntry`].
/// Entries are grouped by set and sorted by binding index. The [`BindingType`]
/// and `readonly` flag come from the variable:
///
/// * uniform address space maps to [`BindingType::UniformBuffer`] with `readonly` false
/// * storage address space maps to [`BindingType::StorageBuffer`], with `readonly` true when the access does not include stores
/// * samplers map to [`BindingType::Sampler`] with `readonly` false
/// * sampled, depth, and external images map to [`BindingType::SampledTexture`] with `readonly` false
/// * storage images map to [`BindingType::StorageTexture`], with `readonly` true when the access does not include stores
///
/// Visibility on every entry is the single stage flag for `stage`
/// ([`ShaderStageFlags::VERTEX`], [`ShaderStageFlags::FRAGMENT`], or
/// [`ShaderStageFlags::COMPUTE`]). Globals without a binding are skipped, as
/// are handle types other than samplers and images.
///
/// Each global in naga's immediate address space becomes a
/// [`rotex_types::PushConstantRange`] at offset `0`, sized from the naga type,
/// with that same stage visibility. A global whose type cannot be resolved is
/// skipped.
///
/// Returns [`CompilerError::EntryPointMissing`] when `entry` is not an entry
/// point of `stage`. Returns [`CompilerError::Reflection`] when naga cannot
/// parse the bytes.
pub fn reflect_spirv(
    spirv_bytes: &[u8],
    stage: ShaderStage,
    entry: &str,
) -> Result<AbstractPipelineLayout, CompilerError> {
    let module = parse_spirv(spirv_bytes)?;
    reflect_module(&module, stage, entry)
}

pub(crate) fn parse_spirv(spirv_bytes: &[u8]) -> Result<naga::Module, CompilerError> {
    naga::front::spv::parse_u8_slice(spirv_bytes, &naga::front::spv::Options::default())
        .map_err(|err| CompilerError::Reflection(err.to_string()))
}

/// Builds an [`AbstractPipelineLayout`] from WGSL `source` for `entry` at `stage`.
///
/// The text is parsed as WGSL. Binding groups, read-only flags, visibility, and
/// push constants then follow [`reflect_spirv`]. Returns
/// [`CompilerError::Reflection`] when naga cannot parse the text, and
/// [`CompilerError::EntryPointMissing`] when `entry` is not an entry point of
/// `stage`.
pub fn reflect_wgsl(
    source: &str,
    stage: ShaderStage,
    entry: &str,
) -> Result<AbstractPipelineLayout, CompilerError> {
    let module = naga::front::wgsl::parse_str(source)
        .map_err(|err| CompilerError::Reflection(err.to_string()))?;
    reflect_module(&module, stage, entry)
}

pub(crate) fn reflect_module(
    module: &naga::Module,
    stage: ShaderStage,
    entry: &str,
) -> Result<AbstractPipelineLayout, CompilerError> {
    let naga_stage = match stage {
        ShaderStage::Vertex => naga::ShaderStage::Vertex,
        ShaderStage::Fragment => naga::ShaderStage::Fragment,
        ShaderStage::Compute => naga::ShaderStage::Compute,
    };

    if !module
        .entry_points
        .iter()
        .any(|ep| ep.name == entry && ep.stage == naga_stage)
    {
        return Err(CompilerError::EntryPointMissing(entry.to_string(), stage));
    }

    let visibility = stage_visibility(stage);
    let mut groups: std::collections::BTreeMap<u32, Vec<BindGroupLayoutEntry>> =
        std::collections::BTreeMap::new();

    for (_, var) in module.global_variables.iter() {
        let Some(binding) = &var.binding else {
            continue;
        };
        let Some((ty, readonly)) = map_binding_type(module, var) else {
            continue;
        };
        groups
            .entry(binding.group)
            .or_default()
            .push(BindGroupLayoutEntry {
                binding: binding.binding,
                visibility,
                ty,
                readonly,
            });
    }

    let bind_groups = groups
        .into_iter()
        .map(|(set, mut entries)| {
            entries.sort_by_key(|entry| entry.binding);
            BindGroupLayoutDescriptor { set, entries }
        })
        .collect();

    let push_constants = module
        .global_variables
        .iter()
        .filter_map(|(_, var)| {
            if !matches!(var.space, naga::AddressSpace::Immediate) {
                return None;
            }
            let ty = module.types.get_handle(var.ty).ok()?;
            let size = ty.inner.size(module.to_ctx());
            Some(rotex_types::PushConstantRange {
                stages: visibility,
                offset: 0,
                size,
            })
        })
        .collect();

    Ok(AbstractPipelineLayout {
        bind_groups,
        push_constants,
    })
}

fn stage_visibility(stage: ShaderStage) -> ShaderStageFlags {
    match stage {
        ShaderStage::Vertex => ShaderStageFlags::VERTEX,
        ShaderStage::Fragment => ShaderStageFlags::FRAGMENT,
        ShaderStage::Compute => ShaderStageFlags::COMPUTE,
    }
}

fn map_binding_type(
    module: &naga::Module,
    var: &naga::GlobalVariable,
) -> Option<(BindingType, bool)> {
    match var.space {
        naga::AddressSpace::Uniform => Some((BindingType::UniformBuffer, false)),
        naga::AddressSpace::Storage { access } => Some((
            BindingType::StorageBuffer,
            !access.contains(naga::StorageAccess::STORE),
        )),
        naga::AddressSpace::Handle => {
            let ty = module.types.get_handle(var.ty).ok()?;
            match ty.inner {
                naga::TypeInner::Sampler { .. } => Some((BindingType::Sampler, false)),
                naga::TypeInner::Image { class, .. } => match class {
                    naga::ImageClass::Storage { access, .. } => Some((
                        BindingType::StorageTexture,
                        !access.contains(naga::StorageAccess::STORE),
                    )),
                    naga::ImageClass::Sampled { .. }
                    | naga::ImageClass::Depth { .. }
                    | naga::ImageClass::External => Some((BindingType::SampledTexture, false)),
                },
                _ => None,
            }
        }
        _ => None,
    }
}
