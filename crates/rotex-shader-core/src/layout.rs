use std::collections::BTreeMap;

use rotex_types::{AbstractPipelineLayout, BindGroupLayoutDescriptor, ShaderPackage};

use crate::CompilerError;

/// Merges the layouts of `vertex` and `fragment` into one [`AbstractPipelineLayout`].
///
/// Bind groups are copied from `vertex`. Each fragment group is then merged by
/// set: an existing set gains the fragment entries and those entries are sorted
/// by binding index, and a new set is appended. The combined groups are sorted
/// by set index.
///
/// After sorting, each set index must be one greater than the previous set. A
/// single set at any index is accepted, as is an empty group list. Two entries
/// with the same binding index in the same set produce
/// [`CompilerError::LayoutValidation`]. The message is
/// `duplicate binding {binding} in set {set}` for a repeated binding, or
/// `bind group sets must be contiguous, found gap between {prev} and {next}`
/// for a gap.
///
/// Push-constant ranges are the vertex ranges followed by the fragment ranges.
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
