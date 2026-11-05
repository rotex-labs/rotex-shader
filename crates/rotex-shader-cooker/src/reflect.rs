use rotex_types::{
    AbstractPipelineLayout, BindGroupLayoutDescriptor, BindGroupLayoutEntry, BindingType,
    ShaderStage, ShaderStageFlags,
};

use rotex_shader_core::CompilerError;

/// Builds an [`AbstractPipelineLayout`] from SPIR-V `spirv_bytes` for `entry` at `stage`.
///
/// The bytes are parsed with naga's SPIR-V options, allowing capabilities
/// outside naga's strict set so Slang's draw-parameter builtins load. Binding and
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
/// * a SPIR-V variable whose type is a sampled image is promoted to [`BindingType::CombinedImageSampler`]
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
    let mut layout = reflect_module(&module, stage, entry)?;
    promote_combined_images(&mut layout, spirv_bytes);
    Ok(layout)
}

pub(crate) fn parse_spirv(spirv_bytes: &[u8]) -> Result<naga::Module, CompilerError> {
    naga::front::spv::parse_u8_slice(
        spirv_bytes,
        &naga::front::spv::Options {
            strict_capabilities: false,
            ..Default::default()
        },
    )
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

fn promote_combined_images(layout: &mut AbstractPipelineLayout, spirv_bytes: &[u8]) {
    let combined = combined_image_bindings(spirv_bytes);
    if combined.is_empty() {
        return;
    }
    for group in &mut layout.bind_groups {
        for entry in &mut group.entries {
            if entry.ty == BindingType::SampledTexture
                && combined.contains(&(group.set, entry.binding))
            {
                entry.ty = BindingType::CombinedImageSampler;
            }
        }
    }
}

fn combined_image_bindings(bytes: &[u8]) -> Vec<(u32, u32)> {
    if bytes.len() < 20 || !bytes.len().is_multiple_of(4) {
        return Vec::new();
    }
    let words: Vec<u32> = bytes
        .as_chunks::<4>()
        .0
        .iter()
        .map(|chunk| u32::from_le_bytes(*chunk))
        .collect();
    if words[0] != 0x0723_0203 {
        return Vec::new();
    }
    let mut sampled = std::collections::HashSet::new();
    let mut pointers = std::collections::HashSet::new();
    let mut variables = Vec::new();
    let mut binding = std::collections::HashMap::new();
    let mut set = std::collections::HashMap::new();
    let mut index = 5;
    while index < words.len() {
        let header = words[index];
        let len = (header >> 16) as usize;
        let op = header & 0xffff;
        if len == 0 || index + len > words.len() {
            break;
        }
        let inst = &words[index + 1..index + len];
        match op {
            27 if !inst.is_empty() => {
                sampled.insert(inst[0]);
            }
            32 if inst.len() >= 3 && sampled.contains(&inst[2]) => {
                pointers.insert(inst[0]);
            }
            59 if inst.len() >= 2 && pointers.contains(&inst[0]) => {
                variables.push(inst[1]);
            }
            71 if inst.len() >= 3 && inst[1] == 33 => {
                binding.insert(inst[0], inst[2]);
            }
            71 if inst.len() >= 3 && inst[1] == 34 => {
                set.insert(inst[0], inst[2]);
            }
            _ => {}
        }
        index += len;
    }
    variables
        .into_iter()
        .filter_map(|id| Some((set.get(&id).copied().unwrap_or(0), *binding.get(&id)?)))
        .collect()
}
