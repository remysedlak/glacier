struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) local_pos: vec2<f32>,
    @location(2) half_size: vec2<f32>,
    @location(3) radius: vec4<f32>,
    @location(4) color: vec3<f32>,
    @location(5) uv: vec2<f32>,
};
struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) local_pos: vec2<f32>,
    @location(1) half_size: vec2<f32>,
    @location(2) radius: vec4<f32>,
    @location(3) color: vec3<f32>,
    @location(4) uv: vec2<f32>,
};

@group(0) @binding(0) var glyph_tex: texture_2d<f32>;
@group(0) @binding(1) var glyph_sampler: sampler;

@vertex
fn vs_main(model: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    out.local_pos = model.local_pos;
    out.half_size = model.half_size;
    out.radius = model.radius;
    out.color = model.color;
    out.uv = model.uv;
    out.clip_position = vec4<f32>(model.position, 1.0);
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {

    // rounded corners
    if any(in.radius > vec4(0.0)) {
        var r = in.radius;
        r.x = select(r.z, r.x, in.local_pos.x > 0.0);
        r.x = select(r.y, r.x, in.local_pos.y > 0.0);
        let q = abs(in.local_pos) - in.half_size + r.x;
        let dist = min(max(q.x, q.y), 0.0) + length(max(q, vec2(0.0))) - r.x;
        if dist > 0.0 {
            discard;
        }
    }
    if in.uv.x < 0.0 {
        return vec4<f32>(in.color, 1.0);
    } else if in.uv.x > 1.0 {
        let actual_uv = vec2<f32>(in.uv.x - 2.0, in.uv.y);
        return textureSample(glyph_tex, glyph_sampler, actual_uv);
    } else {
        let alpha = textureSample(glyph_tex, glyph_sampler, in.uv).r;
        return vec4<f32>(in.color, alpha);
    }
}
