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

struct ScreenUniform {
    resolution: vec2<f32>, // width, height in pixels
};
@group(1) @binding(0) var<uniform> screen: ScreenUniform;

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    if in.uv.x < 0.0 {
        // hw/radius already share a width-based unit (both divided by
        // screen width in the Rust code). hh/local_pos.y use a
        // height-based unit instead — rescale y into the width-based
        // unit so the isotropic SDF below is comparing like with like.
        let aspect = screen.resolution.y / screen.resolution.x; // height/width
        let corrected_local = vec2<f32>(in.local_pos.x, in.local_pos.y * aspect);
        let corrected_half = vec2<f32>(in.half_size.x, in.half_size.y * aspect);

        var r = in.radius;
        r.x = select(r.z, r.x, in.local_pos.x > 0.0);
        r.x = select(r.y, r.x, in.local_pos.y > 0.0);
        // radius already matches x's unit — no correction needed

        let q = abs(corrected_local) - corrected_half + r.x;
        let dist = min(max(q.x, q.y), 0.0) + length(max(q, vec2(0.0))) - r.x;
        let aa = fwidth(dist);

        let outer_alpha = 1.0 - smoothstep(-aa, aa, dist);
        if outer_alpha < aa {
            discard;
        }

        if in.border_width > 0.0 {
            let border_alpha = smoothstep(-in.border_width - aa, -in.border_width + aa, dist);
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
