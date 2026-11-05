//! Slang text for a validated [`crate::ShaderGraph`].

use crate::{GraphError, MaterialMode, Node, ShaderGraph, ValueType};

pub(crate) const VERTEX_ENTRY: &str = "vertexMain";
pub(crate) const FRAGMENT_ENTRY: &str = "fragmentMain";

pub(crate) fn emit(graph: &ShaderGraph) -> Result<String, GraphError> {
    let position = graph
        .position()
        .ok_or_else(|| GraphError::invalid("missing position"))?;
    let fragment = graph
        .fragment_color()
        .ok_or_else(|| GraphError::invalid("missing fragment color"))?;
    let (position_input, position_w) = clip_input(graph, position)?;
    let (color_input, color_w) = fragment_input(graph, fragment)?;
    let mut source = String::new();
    if let Some(mode) = graph.mode() {
        source.push_str(&resources(graph, mode)?);
    }
    source.push_str(&stages(
        graph,
        position_input,
        position_w,
        color_input,
        color_w,
        graph.mode(),
    )?);
    Ok(source)
}

fn clip_input(graph: &ShaderGraph, id: crate::NodeId) -> Result<(&Node, f32), GraphError> {
    let Node::ExtendVec3ToVec4 { value, w } = *graph.node(id)? else {
        return Err(GraphError::invalid("position must extend a vec3"));
    };
    Ok((graph.node(value)?, constant(graph, w)?))
}

fn fragment_input(graph: &ShaderGraph, id: crate::NodeId) -> Result<(&Node, f32), GraphError> {
    let Node::ExtendVec3ToVec4 { value, w } = *graph.node(id)? else {
        return Err(GraphError::invalid("fragment color must extend a vec3"));
    };
    let Node::Interpolate(inner) = *graph.node(value)? else {
        return Err(GraphError::invalid(
            "fragment color must interpolate a vertex input",
        ));
    };
    Ok((graph.node(inner)?, constant(graph, w)?))
}

fn constant(graph: &ShaderGraph, id: crate::NodeId) -> Result<f32, GraphError> {
    match graph.node(id)? {
        Node::Constant(value) => Ok(*value),
        _ => Err(GraphError::invalid("expected a constant")),
    }
}

fn resources(graph: &ShaderGraph, mode: MaterialMode) -> Result<String, GraphError> {
    let mut source = String::from("struct Global {\n");
    for node in graph.nodes() {
        if let Node::UniformField { name, ty } = node {
            source.push_str(&format!("    {} {name};\n", slang_type(*ty)));
        }
    }
    let uniform = mode.uniform_slot();
    source.push_str(&format!(
        "}};\n\n[[vk::binding({}, {})]]\nConstantBuffer<Global> global;\n\n",
        uniform.binding, uniform.set
    ));
    if let MaterialMode::SsboIndexing { storage, .. } = mode {
        let Node::StorageElement { name, ty } = graph
            .nodes()
            .iter()
            .find(|node| matches!(node, Node::StorageElement { .. }))
            .ok_or_else(|| GraphError::invalid("ssbo indexing requires a storage element"))?
        else {
            return Err(GraphError::invalid(
                "ssbo indexing requires a storage element",
            ));
        };
        source.push_str(&format!(
            "struct StorageRecord {{\n    {} {name};\n}};\n\n[[vk::binding({}, {})]]\nStructuredBuffer<StorageRecord> storage_buffer;\n\n",
            slang_type(*ty),
            storage.binding,
            storage.set
        ));
    }
    Ok(source)
}

fn stages(
    graph: &ShaderGraph,
    position_input: &Node,
    position_w: f32,
    color_input: &Node,
    color_w: f32,
    mode: Option<MaterialMode>,
) -> Result<String, GraphError> {
    let Node::VertexInput {
        location: position_location,
        name: position_name,
        ..
    } = position_input
    else {
        return Err(GraphError::invalid("expected a vertex input"));
    };
    let Node::VertexInput {
        location: color_location,
        name: color_name,
        ..
    } = color_input
    else {
        return Err(GraphError::invalid("expected a vertex input"));
    };
    let position_w = format_scalar(position_w);
    let color_w = format_scalar(color_w);
    let color_assign = storage_color(graph, mode)
        .unwrap_or_else(|| format!("    output.color = input.{color_name};\n"));
    let uniform_use = uniform_addend(graph, mode);
    Ok(format!(
        "\
struct VertexInput {{
    [[vk::location({position_location})]] float3 {position_name};
    [[vk::location({color_location})]] float3 {color_name};
}};

struct VertexOutput {{
    float4 position : SV_Position;
    [[vk::location(0)]] float3 {color_name};
}};

[shader(\"vertex\")]
VertexOutput {VERTEX_ENTRY}(VertexInput input) {{
    VertexOutput output;
    output.position = float4(input.{position_name}, {position_w});
{uniform_use}{color_assign}    return output;
}}

[shader(\"fragment\")]
float4 {FRAGMENT_ENTRY}(VertexOutput input) : SV_Target {{
    return float4(input.{color_name}, {color_w});
}}
"
    ))
}

fn uniform_addend(graph: &ShaderGraph, mode: Option<MaterialMode>) -> String {
    if mode.is_none() {
        return String::new();
    }
    let Some(Node::UniformField { name, ty }) = graph
        .nodes()
        .iter()
        .find(|node| matches!(node, Node::UniformField { .. }))
    else {
        return String::new();
    };
    let swizzle = match ty {
        ValueType::Scalar => "",
        ValueType::Vec3 | ValueType::Vec4 => ".x",
    };
    format!("    output.position.x += global.{name}{swizzle};\n")
}

fn storage_color(graph: &ShaderGraph, mode: Option<MaterialMode>) -> Option<String> {
    if !matches!(mode, Some(MaterialMode::SsboIndexing { .. })) {
        return None;
    }
    let Node::StorageElement { name, ty } = graph
        .nodes()
        .iter()
        .find(|node| matches!(node, Node::StorageElement { .. }))?
    else {
        return None;
    };
    let swizzle = match ty {
        ValueType::Vec4 => ".xyz",
        ValueType::Vec3 | ValueType::Scalar => "",
    };
    Some(format!(
        "    output.color = storage_buffer[0].{name}{swizzle};\n"
    ))
}

fn slang_type(ty: ValueType) -> &'static str {
    match ty {
        ValueType::Scalar => "float",
        ValueType::Vec3 => "float3",
        ValueType::Vec4 => "float4",
    }
}

fn format_scalar(value: f32) -> String {
    format!("{value:.1}")
}

impl MaterialMode {
    pub(crate) fn uniform_slot(self) -> crate::BindingSlot {
        match self {
            Self::DynamicUbo { uniform } | Self::SsboIndexing { uniform, .. } => uniform,
        }
    }
}
