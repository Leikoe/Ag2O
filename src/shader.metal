#include <metal_stdlib>
using namespace metal;

constant metal::os_log custom_log("com.custom_log.subsystem", "custom category");

struct VertexOut {
    float4 position [[position]];
    float2 texCoord;
};

struct ViewUniforms {
    float2 offset;
    float scale;
    float aspectRatio;
};

// Standard Quad Vertices
constant float2 positions[4] = {
    {-1.0, -1.0}, { 1.0, -1.0}, {-1.0,  1.0}, { 1.0,  1.0}
};

// Standard Texture Coords (0,0 to 1,1)
constant float2 baseTexCoords[4] = {
    {0.0, 1.0}, {1.0, 1.0}, {0.0, 0.0}, {1.0, 0.0}
};

vertex VertexOut vertex_main(
    uint vertexID [[vertex_id]],
    constant ViewUniforms &uniforms [[buffer(1)]] // <-- We receive data here!
) {
    VertexOut out;

    custom_log.log("custom message hi!!");

    // 1. Calculate Position
    // We keep the quad filling the screen (-1 to 1)
    out.position = float4(positions[vertexID], 0.0, 1.0);

    // 2. Calculate Texture Coordinates
    // This is where the magic happens. We transform the UVs.
    // Start with standard 0..1
    float2 uv = baseTexCoords[vertexID];

    // Apply Zoom (Scale)
    // We center the zoom by subtracting 0.5, scaling, then adding 0.5 back
    uv = (uv - 0.5) * (1.0 / uniforms.scale) + 0.5;

    // Apply Pan (Offset)
    uv -= uniforms.offset;

    // (Optional: Aspect Ratio fix would happen here too)

    out.texCoord = uv;
    return out;
}

fragment float4 fragment_main(VertexOut in [[stage_in]],
                                texture2d<float> texture [[texture(0)]]) {
    constexpr sampler s(mag_filter::linear, min_filter::linear, address::clamp_to_edge);
    return texture.sample(s, in.texCoord);
}
