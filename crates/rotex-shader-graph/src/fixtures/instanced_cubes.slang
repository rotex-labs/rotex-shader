struct Global {
    float4x4 view;
    float4x4 proj;
    float time;
};

[[vk::binding(0, 0)]]
ConstantBuffer<Global> global;

struct Instance {
    float4 pos_and_speed;
    float4 rot_axis;
    float4 color;
};

[[vk::binding(0, 1)]]
StructuredBuffer<Instance> instances;

struct VertexInput {
    [[vk::location(0)]] float3 position;
};

struct VertexOutput {
    float4 position : SV_Position;
    [[vk::location(0)]] float3 color;
};

float4x4 rotation_y(float angle) {
    float c = cos(angle);
    float s = sin(angle);
    return float4x4(
        float4(c, 0.0, s, 0.0),
        float4(0.0, 1.0, 0.0, 0.0),
        float4(-s, 0.0, c, 0.0),
        float4(0.0, 0.0, 0.0, 1.0)
    );
}

[shader("vertex")]
VertexOutput vertexMain(VertexInput input, uint instance_id : SV_InstanceID) {
    VertexOutput output;
    Instance inst = instances[instance_id];
    float angle = global.time * inst.pos_and_speed.w;
    float4x4 model = rotation_y(angle);
    model[3] = float4(inst.pos_and_speed.xyz, 1.0);
    output.color = inst.color.xyz;
    output.position = mul(global.proj, mul(global.view, mul(model, float4(input.position, 1.0))));
    return output;
}

[shader("fragment")]
float4 fragmentMain(VertexOutput input) : SV_Target {
    return float4(input.color, 1.0);
}
