use rotex_types::{
    AbstractPipelineLayout, BindGroupLayoutDescriptor, BindGroupLayoutEntry, BindingType,
    ShaderStage, ShaderStageFlags,
};

use rotex_shader_core::CompilerError;

pub fn reflect_spirv(
    spirv_bytes: &[u8],
    stage: ShaderStage,
    entry: &str,
) -> Result<AbstractPipelineLayout, CompilerError> {
    let module = naga::front::spv::parse_u8_slice(spirv_bytes, &naga::front::spv::Options::default())
        .map_err(|err| CompilerError::Reflection(err.to_string()))?;

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
        let Some((ty, readonly)) = map_binding_type(&module, var) else {
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
                size: size as u32,
            })
        })
        .collect();

    Ok(AbstractPipelineLayout {
        bind_groups,
        push_constants,
    })
}

pub fn reflect_wgsl(
    source: &str,
    stage: ShaderStage,
    entry: &str,
) -> Result<AbstractPipelineLayout, CompilerError> {
    let module = naga::front::wgsl::parse_str(source)
        .map_err(|err| CompilerError::Reflection(err.to_string()))?;
    reflect_module(&module, stage, entry)
}

fn reflect_module(
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
                size: size as u32,
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
                naga::TypeInner::Sampler { .. } | naga::TypeInner::Image { .. } => {
                    Some((BindingType::CombinedImageSampler, false))
                }
                _ => None,
            }
        }
        _ => None,
    }
}
