struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) local_pos: vec2<f32>,
    @location(2) half_size: vec2<f32>,
    @location(3) radius: vec4<f32>,
    @location(4) color: vec3<f32>,
    @location(5) uv: vec2<f32>,
    @location(6) border_width: f32,   // 0.0 = no border
    @location(7) border_color: vec3<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) local_pos: vec2<f32>,
    @location(1) half_size: vec2<f32>,
    @location(2) radius: vec4<f32>,
    @location(3) color: vec3<f32>,
    @location(4) uv: vec2<f32>,
    @location(5) border_width: f32,
    @location(6) border_color: vec3<f32>,
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
    out.border_width = model.border_width;
    out.border_color = model.border_color;
    out.clip_position = vec4<f32>(model.position, 1.0);
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    // solid-color rectangles: unified SDF path handles both rounded and
    // square corners (radius = 0 reduces cleanly to an axis-aligned box SDF),
    // so borders and antialiasing now apply uniformly to every rectangle
    if in.uv.x < 0.0 {
        var r = in.radius;
        r.x = select(r.z, r.x, in.local_pos.x > 0.0);
        r.x = select(r.y, r.x, in.local_pos.y > 0.0);

        let q = abs(in.local_pos) - in.half_size + r.x;
        let dist = min(max(q.x, q.y), 0.0) + length(max(q, vec2(0.0))) - r.x;

        let outer_alpha = 1.0 - smoothstep(-0.001, 0.001, dist);
        if outer_alpha < 0.001 {
            discard;
        }

        if in.border_width > 0.0 {
            // dist <= -border_width: interior fill. dist in (-border_width, 0]: border band.
            let border_alpha = smoothstep(-in.border_width - 0.001, -in.border_width + 0.001, dist);
            let final_color = mix(in.color, in.border_color, border_alpha);
            return vec4<f32>(final_color, outer_alpha);
        }

        return vec4<f32>(in.color, outer_alpha);
    } else if in.uv.x > 1.0 {
        let actual_uv = vec2<f32>(in.uv.x - 2.0, in.uv.y);
        return textureSample(glyph_tex, glyph_sampler, actual_uv);
    } else {
        let alpha = textureSample(glyph_tex, glyph_sampler, in.uv).r;
        return vec4<f32>(in.color, alpha);
    }
}
