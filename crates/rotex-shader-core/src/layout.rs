use std::collections::BTreeMap;

use rotex_types::{AbstractPipelineLayout, BindGroupLayoutDescriptor, ShaderPackage};

use crate::CompilerError;

pub fn merge_graphics_layout(
    vertex: &ShaderPackage,
    fragment: &ShaderPackage,
) -> Result<AbstractPipelineLayout, CompilerError> {
    let mut bind_groups = vertex.layout.bind_groups.clone();
    for group in &fragment.layout.bind_groups {
        if let Some(existing) = bind_groups.iter_mut().find(|g| g.set == group.set) {
            for entry in &group.entries {
                if existing.entries.iter().any(|e| e.binding == entry.binding) {
                    return Err(CompilerError::LayoutValidation(format!(
                        "duplicate binding {} in set {}",
                        entry.binding, group.set
                    )));
                }
                existing.entries.push(entry.clone());
            }
            existing.entries.sort_by_key(|entry| entry.binding);
        } else {
            bind_groups.push(group.clone());
        }
    }
    bind_groups.sort_by_key(|group| group.set);
    validate_bind_groups(&bind_groups)?;

    let mut push_constants = vertex.layout.push_constants.clone();
    push_constants.extend(fragment.layout.push_constants.clone());

    Ok(AbstractPipelineLayout {
        bind_groups,
        push_constants,
    })
}

fn validate_bind_groups(bind_groups: &[BindGroupLayoutDescriptor]) -> Result<(), CompilerError> {
    for window in bind_groups.windows(2) {
        if window[0].set + 1 != window[1].set {
            return Err(CompilerError::LayoutValidation(format!(
                "bind group sets must be contiguous, found gap between {} and {}",
                window[0].set, window[1].set
            )));
        }
    }

    for group in bind_groups {
        let mut seen = BTreeMap::new();
        for entry in &group.entries {
            if seen.insert(entry.binding, ()).is_some() {
                return Err(CompilerError::LayoutValidation(format!(
                    "duplicate binding {} in set {}",
                    entry.binding, group.set
                )));
            }
        }
    }
    Ok(())
}
