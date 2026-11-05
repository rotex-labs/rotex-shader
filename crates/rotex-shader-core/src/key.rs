use rotex_types::ShaderStage;

/// blake3 digest of the canonical cook inputs.
///
/// The digest covers the source text, the defines sorted by name, and the
/// entry points in the order given. Two cooks with the same inputs produce
/// the same key. The digest is not a hash of the compiled bytes.
pub fn artifact_key(
    source: &str,
    defines: &[(&str, &str)],
    entries: &[(&str, ShaderStage)],
) -> [u8; 32] {
    let mut canonical = Vec::new();
    canonical.extend_from_slice(source.as_bytes());
    canonical.push(0xff);
    let mut defines = defines.to_vec();
    defines.sort_by(|left, right| left.0.cmp(right.0).then(left.1.cmp(right.1)));
    for (name, value) in defines {
        canonical.extend_from_slice(name.as_bytes());
        canonical.push(b'=');
        canonical.extend_from_slice(value.as_bytes());
        canonical.push(b'\n');
    }
    canonical.push(0xff);
    for (name, stage) in entries {
        canonical.extend_from_slice(name.as_bytes());
        canonical.push(b':');
        canonical.extend_from_slice(stage_name(*stage).as_bytes());
        canonical.push(b'\n');
    }
    blake3::hash(&canonical).as_bytes().to_owned()
}

fn stage_name(stage: ShaderStage) -> &'static str {
    match stage {
        ShaderStage::Vertex => "vertex",
        ShaderStage::Fragment => "fragment",
        ShaderStage::Compute => "compute",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn key_is_stable_and_define_order_does_not_matter() {
        let first = artifact_key(
            "void main() {}",
            &[("B", "2"), ("A", "1")],
            &[("vertexMain", ShaderStage::Vertex)],
        );
        let second = artifact_key(
            "void main() {}",
            &[("A", "1"), ("B", "2")],
            &[("vertexMain", ShaderStage::Vertex)],
        );
        assert_eq!(first, second);
        let changed = artifact_key(
            "void main() { return; }",
            &[("A", "1"), ("B", "2")],
            &[("vertexMain", ShaderStage::Vertex)],
        );
        assert_ne!(first, changed);
    }
}
