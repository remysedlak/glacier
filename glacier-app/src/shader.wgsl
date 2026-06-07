// VertexInput is the raw vertex data uploaded from the CPU.
// Each field maps to a @location in the vertex buffer layout defined in Vertex::desc().
struct VertexInput {
    @location(0) position: vec3<f32>,  // NDC position of this vertex
    @location(1) local_pos: vec2<f32>, // position relative to the center of the rectangle, used for SDF
    @location(2) half_size: vec2<f32>, // half width and half height of the rectangle in NDC, used for SDF
    @location(3) radius: vec4<f32>,    // corner radii in NDC: [top-right, top-left, bottom-right, bottom-left]
    @location(4) color: vec3<f32>,     // RGB color
    @location(5) uv: vec2<f32>,        // texture coordinates. negative = solid color, >1 = icon, 0-1 = glyph alpha
};

// VertexOutput is passed from the vertex shader to the fragment shader.
// The GPU interpolates these values across the triangle for each fragment.
struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>, // final clip space position, required by GPU
    @location(0) local_pos: vec2<f32>,
    @location(1) half_size: vec2<f32>,
    @location(2) radius: vec4<f32>,
    @location(3) color: vec3<f32>,
    @location(4) uv: vec2<f32>,
};

// the glyph texture atlas and sampler, bound from the CPU per draw call
@group(0) @binding(0) var glyph_tex: texture_2d<f32>;
@group(0) @binding(1) var glyph_sampler: sampler;

// vertex shader — runs once per vertex, transforms position and passes data through
@vertex
fn vs_main(model: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    out.local_pos = model.local_pos;
    out.half_size = model.half_size;
    out.radius = model.radius;
    out.color = model.color;
    out.uv = model.uv;
    // position is already in NDC so just pass it through with w=1.0
    out.clip_position = vec4<f32>(model.position, 1.0);
    return out;
}

// fragment shader — runs once per pixel, returns the final color
@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {

    // rounded corner SDF — only runs if any corner radius is non-zero
    if any(in.radius > vec4(0.0)) {
        var r = in.radius;
        // pick the correct radius for the quadrant this fragment is in
        // radius.x=top-right, radius.y=top-left, radius.z=bottom-right, radius.w=bottom-left
        r.x = select(r.z, r.x, in.local_pos.x > 0.0); // left vs right
        r.x = select(r.y, r.x, in.local_pos.y > 0.0); // top vs bottom

        // signed distance field for a rounded rectangle
        // q is the distance from the fragment to the rounded corner arc
        let q = abs(in.local_pos) - in.half_size + r.x;
        // dist < 0 means inside the shape, dist > 0 means outside
        let dist = min(max(q.x, q.y), 0.0) + length(max(q, vec2(0.0))) - r.x;

        // smoothstep gives a 1px antialiased edge instead of a hard cutoff
        let alpha = 1.0 - smoothstep(-0.001, 0.001, dist);
        if alpha < 0.001 { discard; }
        return vec4<f32>(in.color, alpha);
    }

    // uv.x < 0 means this is a solid color rectangle — no texture
    if in.uv.x < 0.0 {
        return vec4<f32>(in.color, 1.0);
    // uv.x > 1 means this is an icon — uv is offset by 2.0 to distinguish from glyph uvs
    } else if in.uv.x > 1.0 {
        let actual_uv = vec2<f32>(in.uv.x - 2.0, in.uv.y);
        return textureSample(glyph_tex, glyph_sampler, actual_uv);
    // uv in 0-1 range means this is a glyph — sample the red channel as alpha
    } else {
        let alpha = textureSample(glyph_tex, glyph_sampler, in.uv).r;
        return vec4<f32>(in.color, alpha);
    }
}
