// Blit de elementos HUD (como el contador de FPS) con alpha blending.
// Dibuja un rectángulo especificado por Params (origin_x, origin_y, dst_w, dst_h)
// y descarta (discard) los fragmentos fuera de los límites para no alterar el juego.

struct Params {
    origin_x: f32,
    origin_y: f32,
    dst_w: f32,
    dst_h: f32,
    res_x: f32,
    res_y: f32,
    pad0: f32,
    pad1: f32,
};

@group(0) @binding(0) var samp: sampler;
@group(0) @binding(1) var tex: texture_2d<f32>;
@group(0) @binding(2) var<uniform> params: Params;

struct VSOut {
    @builtin(position) pos: vec4<f32>,
};

@vertex
fn vs_main(@builtin(vertex_index) vi: u32) -> VSOut {
    var xy = array<vec2<f32>, 3>(
        vec2<f32>(-1.0, -1.0), vec2<f32>(3.0, -1.0), vec2<f32>(-1.0, 3.0)
    );
    var out: VSOut;
    out.pos = vec4<f32>(xy[vi], 0.0, 1.0);
    return out;
}

@fragment
fn fs_main(in: VSOut) -> @location(0) vec4<f32> {
    let px = in.pos.xy;
    let lo = vec2<f32>(params.origin_x, params.origin_y);
    let hi = lo + vec2<f32>(params.dst_w, params.dst_h);

    if px.x < lo.x || px.y < lo.y || px.x >= hi.x || px.y >= hi.y {
        discard;
    }

    let uv = (px - lo) / vec2<f32>(params.dst_w, params.dst_h);
    let texel = textureSampleLevel(tex, samp, uv, 0.0);
    return texel;
}
