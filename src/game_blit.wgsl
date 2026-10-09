// Blit del marco de juego GBA con letterbox 3:2 centrado.
// La textura fuente (240x160 RGBA8) ya viene display-encoded por el
// pipeline de color de `screen` (LUT + temporal); este pase solo la
// coloca en la ventana sin re-escalar colores (muestreo nearest: píxel
// nítido como el panel original).

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
    @location(0) uv: vec2<f32>,
};

@vertex
fn vs_main(@builtin(vertex_index) vi: u32) -> VSOut {
    // Triángulo fullscreen; UV en coordenadas de ventana (0..1).
    var xy = array<vec2<f32>, 3>(
        vec2<f32>(-1.0, -1.0), vec2<f32>(3.0, -1.0), vec2<f32>(-1.0, 3.0)
    );
    var out: VSOut;
    let p = xy[vi];
    out.pos = vec4<f32>(p, 0.0, 1.0);
    out.uv = vec2<f32>((p.x + 1.0) * 0.5, (1.0 - (p.y + 1.0) * 0.5));
    return out;
}

@fragment
fn fs_main(in: VSOut) -> @location(0) vec4<f32> {
    let px = in.pos.xy;
    let lo = vec2<f32>(params.origin_x, params.origin_y);
    let hi = lo + vec2<f32>(params.dst_w, params.dst_h);

    // Fuera del marco: negro (bordes de la ventana).
    if px.x < lo.x || px.y < lo.y || px.x >= hi.x || px.y >= hi.y {
        return vec4<f32>(0.0, 0.0, 0.0, 1.0);
    }
    // UV dentro del marco 3:2 (nearest: píxel GBA nítido).
    let uv = (px - lo) / vec2<f32>(params.dst_w, params.dst_h);
    let texel = textureSampleLevel(tex, samp, uv, 0.0);
    return vec4<f32>(texel.rgb, 1.0);
}
