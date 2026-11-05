#![deny(missing_docs)]
#![deny(rustdoc::broken_intra_doc_links)]

//! Host-side shader graph.
//!
//! [`ShaderGraph`] is a small typed graph. [`ShaderGraph::compile`] emits Slang
//! text and the [`BindGroupLayoutDescriptor`] list for that program.
//! [`ShaderGraph::unlit`] is built from nodes and emits the triangle program.
//! A backend never sees the graph. Cooking stays behind the `cook` feature and
//! calls `rotex-shader-cooker`.

mod emit;

use std::fmt;

use rotex_shader_core::{CompilerError, DEFAULT_VARIANT_BUDGET, check_variant_budget};
use rotex_types::{BindGroupLayoutDescriptor, BindGroupLayoutEntry, BindingType, ShaderStageFlags};

use emit::{FRAGMENT_ENTRY, VERTEX_ENTRY};

/// Identifier of one node in a [`ShaderGraph`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct NodeId(pub u32);

/// Value carried by a node.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ValueType {
    /// A single float.
    Scalar,
    /// A three-component vector.
    Vec3,
    /// A four-component vector.
    Vec4,
}

/// One operation in the graph.
#[derive(Debug, Clone, PartialEq)]
pub enum Node {
    /// A vertex-attribute input.
    VertexInput {
        /// Attribute location.
        location: u32,
        /// Name used in the emitted Slang.
        name: String,
        /// Type of the attribute.
        ty: ValueType,
    },
    /// A floating-point literal.
    Constant(f32),
    /// Appends a scalar to a vec3, producing a vec4.
    ExtendVec3ToVec4 {
        /// Vec3 value.
        value: NodeId,
        /// Scalar stored in the fourth component.
        w: NodeId,
    },
    /// Carries a vertex value to the fragment stage.
    Interpolate(NodeId),
    /// One field of the uniform block selected by [`MaterialMode`].
    UniformField {
        /// Field name.
        name: String,
        /// Field type.
        ty: ValueType,
    },
    /// One field of the readonly storage element selected by [`MaterialMode::SsboIndexing`].
    StorageElement {
        /// Field name.
        name: String,
        /// Field type.
        ty: ValueType,
    },
}

/// Descriptor-set slot chosen by the caller.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BindingSlot {
    /// Descriptor set.
    pub set: u32,
    /// Binding index inside that set.
    pub binding: u32,
}

/// How the graph binds resources.
///
/// Set and binding numbers come from the caller. The graph does not reserve
/// sets 0–3 for a fixed engine layout.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MaterialMode {
    /// One uniform block addressed with a dynamic offset.
    DynamicUbo {
        /// Set and binding of the uniform block.
        uniform: BindingSlot,
    },
    /// That uniform block plus one readonly storage buffer.
    SsboIndexing {
        /// Set and binding of the uniform block.
        uniform: BindingSlot,
        /// Set and binding of the storage buffer.
        storage: BindingSlot,
    },
}

/// Slang program produced by [`ShaderGraph::compile`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GraphProgram {
    /// Slang source.
    pub slang: String,
    /// Vertex entry point.
    pub vertex_entry: String,
    /// Fragment entry point.
    pub fragment_entry: String,
    /// Bind-group layouts. Each set index is the one the caller supplied.
    pub layout: Vec<BindGroupLayoutDescriptor>,
    /// Shader keywords passed through to a cook.
    pub keywords: Vec<String>,
    /// Preprocessor defines passed through to a cook.
    pub defines: Vec<(String, String)>,
}

/// A graph that failed to validate or that exceeds the variant budget.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GraphError {
    /// The node connections cannot be emitted.
    Invalid(String),
    /// Declared keywords expand past [`DEFAULT_VARIANT_BUDGET`].
    VariantBudget {
        /// Combinations the keywords would produce.
        combinations: u32,
        /// Maximum combinations a cook allows.
        budget: u32,
    },
}

impl fmt::Display for GraphError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Invalid(message) => write!(formatter, "{message}"),
            Self::VariantBudget {
                combinations,
                budget,
            } => write!(
                formatter,
                "variant budget exceeded: {combinations} combinations, budget is {budget}"
            ),
        }
    }
}

impl std::error::Error for GraphError {}

/// Host-side graph of shading operations.
#[derive(Debug, Clone, PartialEq)]
pub struct ShaderGraph {
    nodes: Vec<Node>,
    position: Option<NodeId>,
    fragment_color: Option<NodeId>,
    mode: Option<MaterialMode>,
    keywords: Vec<String>,
    defines: Vec<(String, String)>,
}

impl Default for ShaderGraph {
    fn default() -> Self {
        Self::new()
    }
}

impl ShaderGraph {
    /// An empty graph.
    pub fn new() -> Self {
        Self {
            nodes: Vec::new(),
            position: None,
            fragment_color: None,
            mode: None,
            keywords: Vec::new(),
            defines: Vec::new(),
        }
    }

    /// Unlit position and color pass-through, built from nodes.
    ///
    /// The compiled Slang matches the triangle fixture byte for byte, and the
    /// layout is empty.
    pub fn unlit() -> Self {
        let mut graph = Self::new();
        let position = graph.vertex_input(0, "position", ValueType::Vec3);
        let color = graph.vertex_input(1, "color", ValueType::Vec3);
        let one = graph.constant(1.0);
        let clip = graph.extend_vec3(position, one);
        graph.set_position(clip);
        let varying = graph.interpolate(color);
        let fragment = graph.extend_vec3(varying, one);
        graph.set_fragment_color(fragment);
        graph
    }

    /// Appends a vertex input and returns its id.
    pub fn vertex_input(
        &mut self,
        location: u32,
        name: impl Into<String>,
        ty: ValueType,
    ) -> NodeId {
        self.push(Node::VertexInput {
            location,
            name: name.into(),
            ty,
        })
    }

    /// Appends a scalar literal and returns its id.
    pub fn constant(&mut self, value: f32) -> NodeId {
        self.push(Node::Constant(value))
    }

    /// Appends a vec3-to-vec4 extend and returns its id.
    pub fn extend_vec3(&mut self, value: NodeId, w: NodeId) -> NodeId {
        self.push(Node::ExtendVec3ToVec4 { value, w })
    }

    /// Appends an interpolation and returns its id.
    pub fn interpolate(&mut self, value: NodeId) -> NodeId {
        self.push(Node::Interpolate(value))
    }

    /// Appends a uniform-block field and returns its id.
    pub fn uniform_field(&mut self, name: impl Into<String>, ty: ValueType) -> NodeId {
        self.push(Node::UniformField {
            name: name.into(),
            ty,
        })
    }

    /// Appends a storage-element field and returns its id.
    pub fn storage_element(&mut self, name: impl Into<String>, ty: ValueType) -> NodeId {
        self.push(Node::StorageElement {
            name: name.into(),
            ty,
        })
    }

    /// Selects the vec4 written to `SV_Position`.
    pub fn set_position(&mut self, node: NodeId) {
        self.position = Some(node);
    }

    /// Selects the vec4 returned from the fragment entry.
    pub fn set_fragment_color(&mut self, node: NodeId) {
        self.fragment_color = Some(node);
    }

    /// Selects the resource binding mode.
    pub fn set_material_mode(&mut self, mode: MaterialMode) {
        self.mode = Some(mode);
    }

    /// Records a shader keyword. [`ShaderGraph::compile`] checks the variant budget.
    pub fn keyword(&mut self, keyword: impl Into<String>) {
        self.keywords.push(keyword.into());
    }

    /// Records a preprocessor define passed through to a cook.
    pub fn define(&mut self, name: impl Into<String>, value: impl Into<String>) {
        self.defines.push((name.into(), value.into()));
    }

    /// Nodes in insertion order.
    pub fn nodes(&self) -> &[Node] {
        &self.nodes
    }

    /// Checks node types, links, and binding slots.
    pub fn validate(&self) -> Result<(), GraphError> {
        let position = self
            .position
            .ok_or_else(|| GraphError::invalid("missing position"))?;
        let fragment = self
            .fragment_color
            .ok_or_else(|| GraphError::invalid("missing fragment color"))?;
        self.expect_clip_position(position)?;
        self.expect_fragment_color(fragment)?;
        self.expect_unique_locations()?;
        match self.mode {
            None => {
                if self.uniform_fields().next().is_some()
                    || self.storage_elements().next().is_some()
                {
                    return Err(GraphError::invalid(
                        "resource fields require a material mode",
                    ));
                }
            }
            Some(MaterialMode::DynamicUbo { uniform }) => {
                if self.uniform_fields().next().is_none() {
                    return Err(GraphError::invalid("dynamic ubo requires a uniform field"));
                }
                if self.storage_elements().next().is_some() {
                    return Err(GraphError::invalid("dynamic ubo does not include storage"));
                }
                self.expect_slots(&[uniform])?;
            }
            Some(MaterialMode::SsboIndexing { uniform, storage }) => {
                if self.uniform_fields().next().is_none() {
                    return Err(GraphError::invalid(
                        "ssbo indexing requires a uniform field",
                    ));
                }
                let element = self.storage_elements().next().ok_or_else(|| {
                    GraphError::invalid("ssbo indexing requires a storage element")
                })?;
                if !matches!(
                    element,
                    Node::StorageElement {
                        ty: ValueType::Vec3 | ValueType::Vec4,
                        ..
                    }
                ) {
                    return Err(GraphError::invalid(
                        "storage element must be a vec3 or vec4",
                    ));
                }
                self.expect_slots(&[uniform, storage])?;
            }
        }
        Ok(())
    }

    /// Validates the graph, checks the variant budget, and emits Slang plus a layout.
    pub fn compile(&self) -> Result<GraphProgram, GraphError> {
        self.validate()?;
        let keyword_refs: Vec<&str> = self.keywords.iter().map(String::as_str).collect();
        check_variant_budget(&keyword_refs, DEFAULT_VARIANT_BUDGET).map_err(|err| match err {
            CompilerError::VariantBudget(combinations, budget) => GraphError::VariantBudget {
                combinations,
                budget,
            },
            other => GraphError::invalid(other.to_string()),
        })?;
        Ok(GraphProgram {
            slang: emit::emit(self)?,
            vertex_entry: VERTEX_ENTRY.to_string(),
            fragment_entry: FRAGMENT_ENTRY.to_string(),
            layout: self.layout(),
            keywords: self.keywords.clone(),
            defines: self.defines.clone(),
        })
    }

    pub(crate) fn node(&self, id: NodeId) -> Result<&Node, GraphError> {
        self.nodes
            .get(id.0 as usize)
            .ok_or_else(|| GraphError::invalid(format!("node {} is out of range", id.0)))
    }

    pub(crate) fn mode(&self) -> Option<MaterialMode> {
        self.mode
    }

    pub(crate) fn position(&self) -> Option<NodeId> {
        self.position
    }

    pub(crate) fn fragment_color(&self) -> Option<NodeId> {
        self.fragment_color
    }

    fn push(&mut self, node: Node) -> NodeId {
        let id = NodeId(self.nodes.len() as u32);
        self.nodes.push(node);
        id
    }

    fn expect_clip_position(&self, id: NodeId) -> Result<(), GraphError> {
        let Node::ExtendVec3ToVec4 { value, w } = *self.node(id)? else {
            return Err(GraphError::invalid("position must extend a vec3"));
        };
        self.expect_vertex_input(value, ValueType::Vec3)?;
        self.expect_constant(w)?;
        Ok(())
    }

    fn expect_fragment_color(&self, id: NodeId) -> Result<(), GraphError> {
        let Node::ExtendVec3ToVec4 { value, w } = *self.node(id)? else {
            return Err(GraphError::invalid("fragment color must extend a vec3"));
        };
        let Node::Interpolate(inner) = *self.node(value)? else {
            return Err(GraphError::invalid(
                "fragment color must interpolate a vertex input",
            ));
        };
        self.expect_vertex_input(inner, ValueType::Vec3)?;
        self.expect_constant(w)?;
        Ok(())
    }

    fn expect_vertex_input(&self, id: NodeId, ty: ValueType) -> Result<(), GraphError> {
        match self.node(id)? {
            Node::VertexInput { ty: found, .. } if *found == ty => Ok(()),
            _ => Err(GraphError::invalid("expected a vertex input")),
        }
    }

    fn expect_constant(&self, id: NodeId) -> Result<(), GraphError> {
        match self.node(id)? {
            Node::Constant(_) => Ok(()),
            _ => Err(GraphError::invalid("expected a constant")),
        }
    }

    fn expect_unique_locations(&self) -> Result<(), GraphError> {
        let mut seen = Vec::new();
        for node in &self.nodes {
            if let Node::VertexInput { location, .. } = node {
                if seen.contains(location) {
                    return Err(GraphError::invalid(format!(
                        "duplicate vertex location {location}"
                    )));
                }
                seen.push(*location);
            }
        }
        Ok(())
    }

    fn expect_slots(&self, slots: &[BindingSlot]) -> Result<(), GraphError> {
        let mut sets: Vec<u32> = slots.iter().map(|slot| slot.set).collect();
        sets.sort_unstable();
        sets.dedup();
        for pair in sets.windows(2) {
            if pair[0] + 1 != pair[1] {
                return Err(GraphError::invalid(format!(
                    "bind group sets must be contiguous, found gap between {} and {}",
                    pair[0], pair[1]
                )));
            }
        }
        for (index, slot) in slots.iter().enumerate() {
            if slots[..index].contains(slot) {
                return Err(GraphError::invalid(format!(
                    "duplicate binding {} in set {}",
                    slot.binding, slot.set
                )));
            }
        }
        Ok(())
    }

    fn uniform_fields(&self) -> impl Iterator<Item = &Node> {
        self.nodes
            .iter()
            .filter(|node| matches!(node, Node::UniformField { .. }))
    }

    fn storage_elements(&self) -> impl Iterator<Item = &Node> {
        self.nodes
            .iter()
            .filter(|node| matches!(node, Node::StorageElement { .. }))
    }

    fn layout(&self) -> Vec<BindGroupLayoutDescriptor> {
        let Some(mode) = self.mode else {
            return Vec::new();
        };
        let uniform = match mode {
            MaterialMode::DynamicUbo { uniform } => uniform,
            MaterialMode::SsboIndexing { uniform, .. } => uniform,
        };
        let uniform_ty = match mode {
            MaterialMode::DynamicUbo { .. } => BindingType::UniformBufferDynamic,
            MaterialMode::SsboIndexing { .. } => BindingType::UniformBuffer,
        };
        let mut groups = vec![group(uniform, uniform_ty, false)];
        if let MaterialMode::SsboIndexing { storage, .. } = mode {
            groups.push(group(storage, BindingType::StorageBuffer, true));
        }
        groups.sort_by_key(|descriptor| descriptor.set);
        let mut merged: Vec<BindGroupLayoutDescriptor> = Vec::new();
        for descriptor in groups {
            if let Some(existing) = merged.iter_mut().find(|group| group.set == descriptor.set) {
                existing.entries.extend(descriptor.entries);
                existing.entries.sort_by_key(|entry| entry.binding);
            } else {
                merged.push(descriptor);
            }
        }
        merged
    }
}

fn group(slot: BindingSlot, ty: BindingType, readonly: bool) -> BindGroupLayoutDescriptor {
    BindGroupLayoutDescriptor {
        set: slot.set,
        entries: vec![BindGroupLayoutEntry {
            binding: slot.binding,
            visibility: ShaderStageFlags::VERTEX,
            ty,
            readonly,
        }],
    }
}

impl GraphError {
    fn invalid(message: impl Into<String>) -> Self {
        Self::Invalid(message.into())
    }
}

/// Cooks `program` with `slangc` when the `cook` feature is enabled.
#[cfg(feature = "cook")]
pub fn cook(program: &GraphProgram) -> Result<rotex_types::ShaderArtifact, CompilerError> {
    let slangc =
        find_slangc().ok_or_else(|| CompilerError::Unsupported("slangc_not_found".to_string()))?;
    let defines: Vec<(&str, &str)> = program
        .defines
        .iter()
        .map(|(name, value)| (name.as_str(), value.as_str()))
        .collect();
    let keywords: Vec<&str> = program.keywords.iter().map(String::as_str).collect();
    rotex_shader_cooker::cook_graphics(
        &slangc,
        &program.slang,
        &program.vertex_entry,
        &program.fragment_entry,
        &defines,
        &keywords,
    )
}

#[cfg(feature = "cook")]
fn find_slangc() -> Option<std::path::PathBuf> {
    if let Ok(configured) = std::env::var("ROTEX_SLANGC") {
        let candidate = std::path::PathBuf::from(configured);
        if candidate.is_file() {
            return Some(candidate);
        }
    }
    let search = std::env::var_os("PATH")?;
    std::env::split_paths(&search)
        .map(|directory| directory.join("slangc"))
        .find(|candidate| candidate.is_file())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unlit_matches_the_triangle_fixture() {
        let program = ShaderGraph::unlit().compile().expect("unlit compiles");
        assert!(program.layout.is_empty());
        assert_eq!(program.vertex_entry, "vertexMain");
        assert_eq!(program.fragment_entry, "fragmentMain");
        let fixture = include_str!("fixtures/triangle.slang");
        assert_eq!(program.slang, fixture);
    }

    #[test]
    fn dynamic_ubo_emits_one_dynamic_uniform() {
        let mut graph = ShaderGraph::unlit();
        graph.uniform_field("time", ValueType::Scalar);
        graph.set_material_mode(MaterialMode::DynamicUbo {
            uniform: BindingSlot { set: 2, binding: 3 },
        });
        let program = graph.compile().expect("dynamic ubo compiles");
        assert_eq!(program.layout.len(), 1);
        assert_eq!(program.layout[0].set, 2);
        assert_eq!(program.layout[0].entries[0].binding, 3);
        assert_eq!(
            program.layout[0].entries[0].ty,
            BindingType::UniformBufferDynamic
        );
        assert!(program.slang.contains("[[vk::binding(3, 2)]]"));
        assert!(program.slang.contains("ConstantBuffer<Global> global;"));
    }

    #[test]
    fn ssbo_indexing_emits_uniform_and_readonly_storage() {
        let program = ssbo_program();
        assert_eq!(program.layout.len(), 2);
        assert_eq!(program.layout[0].set, 0);
        assert_eq!(program.layout[0].entries[0].ty, BindingType::UniformBuffer);
        assert!(!program.layout[0].entries[0].readonly);
        assert_eq!(program.layout[1].set, 1);
        assert_eq!(program.layout[1].entries[0].ty, BindingType::StorageBuffer);
        assert!(program.layout[1].entries[0].readonly);
        assert!(
            program
                .slang
                .contains("StructuredBuffer<StorageRecord> storage_buffer;")
        );
    }

    #[test]
    fn gapped_sets_fail_validation() {
        let mut graph = ShaderGraph::unlit();
        graph.uniform_field("time", ValueType::Scalar);
        graph.storage_element("value", ValueType::Vec4);
        graph.set_material_mode(MaterialMode::SsboIndexing {
            uniform: BindingSlot { set: 0, binding: 0 },
            storage: BindingSlot { set: 2, binding: 0 },
        });
        let err = graph.validate().expect_err("gap");
        assert!(err.to_string().contains("contiguous"));
    }

    #[test]
    fn seven_keywords_exceed_the_variant_budget() {
        let mut graph = ShaderGraph::unlit();
        for name in ["a", "b", "c", "d", "e", "f", "g"] {
            graph.keyword(name);
        }
        let err = graph.compile().expect_err("budget");
        assert!(matches!(err, GraphError::VariantBudget { .. }));
    }

    fn ssbo_program() -> GraphProgram {
        let mut graph = ShaderGraph::unlit();
        graph.uniform_field("time", ValueType::Scalar);
        graph.storage_element("value", ValueType::Vec4);
        graph.set_material_mode(MaterialMode::SsboIndexing {
            uniform: BindingSlot { set: 0, binding: 0 },
            storage: BindingSlot { set: 1, binding: 0 },
        });
        graph.compile().expect("ssbo compiles")
    }

    #[cfg(feature = "cook")]
    #[test]
    fn unlit_cook_matches_triangle_fixture() {
        let Some(slangc) = find_slangc() else {
            return;
        };
        let program = ShaderGraph::unlit().compile().expect("unlit compiles");
        let from_graph = cook(&program).expect("cook graph");
        let fixture = include_str!("fixtures/triangle.slang");
        let direct = rotex_shader_cooker::cook_graphics(
            &slangc,
            fixture,
            "vertexMain",
            "fragmentMain",
            &[],
            &[],
        )
        .expect("cook fixture");
        assert_eq!(from_graph.key, direct.key);
        assert_eq!(spirv(&from_graph), spirv(&direct));
    }

    #[cfg(feature = "cook")]
    #[test]
    fn ssbo_cook_matches_instanced_cubes_layout() {
        let Some(slangc) = find_slangc() else {
            return;
        };
        let program = ssbo_program();
        let from_graph = cook(&program).expect("cook ssbo graph");
        assert_eq!(
            shape(&program.layout),
            shape(&from_graph.layout.bind_groups)
        );
        let fixture = include_str!("fixtures/instanced_cubes.slang");
        let reference = rotex_shader_cooker::cook_graphics(
            &slangc,
            fixture,
            "vertexMain",
            "fragmentMain",
            &[],
            &[],
        )
        .expect("cook instanced cubes");
        assert_eq!(shape(&program.layout), shape(&reference.layout.bind_groups));
    }

    #[cfg(feature = "cook")]
    fn spirv(artifact: &rotex_types::ShaderArtifact) -> Vec<Vec<u8>> {
        let mut payloads: Vec<_> = artifact
            .payloads
            .iter()
            .filter(|payload| payload.target == rotex_types::TargetId::VulkanSpirv)
            .map(|payload| payload.bytes.clone())
            .collect();
        payloads.sort();
        payloads
    }

    #[cfg(feature = "cook")]
    fn shape(groups: &[BindGroupLayoutDescriptor]) -> Vec<(u32, u32, BindingType, bool)> {
        let mut rows = Vec::new();
        for group in groups {
            for entry in &group.entries {
                rows.push((group.set, entry.binding, entry.ty, entry.readonly));
            }
        }
        rows
    }
}
