// Overlay táctil de Blazing Recomp — primitivas procedurales SDF
// (Fase 5.2, propuesta 2026-10-09_renderizado-overlay-tactil).
//
// Un solo triángulo fullscreen. El fragment shader evalúa, por píxel:
//  1. El fondo "carcasa" en la mitad inferior (plateado GBA SP).
//  2. Hasta 12 elementos táctiles (uniforms): círculos y rects
//     redondeados con color, glifo (3x5 px) y estado pulsado.
//  3. Antialiasing por distancia (smoothstep en fwidth del SDF).
//
// Sin texturas: bordes matemáticamente perfectos a cualquier DPI
// (restricción de la propuesta). Glifos 3x5: cada uno son 15 bits,
// evaluados con bit-math puro (sin arreglos dinámicos, WGSL uniform).

// Nota de layout: el array uniform exige stride múltiplo de 16 bytes;
// Elem son 12 floats (48 B) para cumplirlo.
struct Elem {
    cx: f32,
    cy: f32,
    radius: f32,      // radio del círculo (o half-extent para rects)
    r: f32,           // color
    g: f32,           // color
    b: f32,           // color
    pressed: f32,     // 1.0 = resaltado
    kind: f32,        // 0 = circle, 1 = rounded-rect (vertical pill)
    glyph: f32,       // índice de glifo (0 = ninguno)
    round: f32,       // radio de esquina (para kind 1)
    glyph_scale: f32, // tamaño del glifo relativo al radio (0.42)
    ring: f32,        // intensidad del anillo exterior (0..1)
};

struct Globals {
    res_x: f32,
    res_y: f32,
    opacity: f32,     // 1.0 portrait, <1.0 landscape (translúcido)
    shell: f32,       // 1.0 = dibujar carcasa, 0.0 = solo elementos
};

@group(0) @binding(0) var<uniform> globals: Globals;
@group(0) @binding(1) var<uniform> elems: array<Elem, 12>;
// 12 * 48 B = 576, múltiplo de 16: válido como uniform.

struct VSOut {
    @builtin(position) pos: vec4<f32>,
};

@vertex
fn vs_main(@builtin(vertex_index) vi: u32) -> VSOut {
    // Triángulo fullscreen (sin vertex buffer).
    var xy = array<vec2<f32>, 3>(
        vec2<f32>(-1.0, -1.0), vec2<f32>(3.0, -1.0), vec2<f32>(-1.0, 3.0)
    );
    var out: VSOut;
    out.pos = vec4<f32>(xy[vi], 0.0, 1.0);
    return out;
}

// ---------- SDFs ----------

fn sd_circle(p: vec2<f32>, c: vec2<f32>, r: f32) -> f32 {
    return length(p - c) - r;
}

fn sd_round_rect(p: vec2<f32>, c: vec2<f32>, half: vec2<f32>, round: f32) -> f32 {
    let q = abs(p - c) - (half - vec2<f32>(round, round));
    return length(max(q, vec2<f32>(0.0, 0.0)))
         + min(max(q.x, q.y), 0.0) - round;
}

// ---------- Glifos 3x5 (15 bits, fila por fila desde el MSB) ----------

// Devuelve [0,1] dentro del glifo; `g` en coords locales del botón,
// con (0,0) en el centro y glifo de 3x5 "unidades" de tamaño u.
fn glyph_bit(idx: u32, row: u32, col: u32) -> f32 {
    // Tabla compacta: glifos x 15 bits, empaquetados en u32 (los 15
    // bits más bajos son el glifo; fila 0 = bit 14 ... fila 4 = bit 0).
    // Glifos: 0=nada, 1=arriba, 2=abajo, 3=izq, 4=der, 5=B, 6=A,
    //         7=L, 8=R, 9=S(select/start resumido)
    var table = array<u32, 10>(
        0x0000u,
        0x2A94u, // flecha arriba
        0x15D4u, // flecha abajo
        0x29E2u, // flecha izquierda (tip col 0, alas col 1, stem col 2)
        0x23CAu, // flecha derecha (stem col 0, alas col 1, tip col 2)
        0x6DB6u, // B
        0x2BEDu, // A
        0x4927u, // L
        0x6DB5u, // R
        0x7CF7u, // S
    );
    let i = min(idx, 9u);
    let bit = 14u - (row * 3u + col);
    return select(0.0, 1.0, ((table[i] >> bit) & 1u) == 1u);
}

// Lectura segura de píxeles del glifo con comprobación previa de límites
// para evitar desbordamientos o corrimientos indefinidos en GPUs móviles.
fn glyph_pixel(idx: u32, row: i32, col: i32) -> f32 {
    if col < 0 || col > 2 || row < 0 || row > 4 {
        return 0.0;
    }
    return glyph_bit(idx, u32(row), u32(col));
}

fn glyph_color(idx: f32, local: vec2<f32>, u: f32, ink: vec3<f32>) -> vec4<f32> {
    let i = u32(idx + 0.5);
    if i == 0u {
        return vec4<f32>(0.0);
    }
    // Glifo centrado: 3 cols x 5 filas de tamaño u
    let origin = vec2<f32>(-1.5 * u, -2.5 * u);
    let q = local - origin;
    let col = i32(floor(q.x / u));
    let row = i32(floor(q.y / u));
    if col < 0 || col > 2 || row < 0 || row > 4 {
        return vec4<f32>(0.0);
    }
    let on = glyph_pixel(i, row, col);
    if on < 0.5 {
        return vec4<f32>(0.0);
    }

    // Suavizado del borde del píxel del glifo con comprobación de vecinos adyacentes
    let cell_frac = fract(q / u);
    let left_on   = glyph_pixel(i, row, col - 1);
    let right_on  = glyph_pixel(i, row, col + 1);
    let top_on    = glyph_pixel(i, row - 1, col);
    let bottom_on = glyph_pixel(i, row + 1, col);

    var d_x = 0.0;
    if left_on < 0.5 && cell_frac.x < 0.5 {
        d_x = max(d_x, (0.5 - cell_frac.x) * 2.0);
    }
    if right_on < 0.5 && cell_frac.x > 0.5 {
        d_x = max(d_x, (cell_frac.x - 0.5) * 2.0);
    }

    var d_y = 0.0;
    if top_on < 0.5 && cell_frac.y < 0.5 {
        d_y = max(d_y, (0.5 - cell_frac.y) * 2.0);
    }
    if bottom_on < 0.5 && cell_frac.y > 0.5 {
        d_y = max(d_y, (cell_frac.y - 0.5) * 2.0);
    }

    let edge = max(d_x, d_y);
    let aa = 1.0 - smoothstep(0.70, 0.98, edge);
    return vec4<f32>(ink * aa, aa);
}

// ---------- Fragmento ----------

@fragment
fn fs_main(in: VSOut) -> @location(0) vec4<f32> {
    let res = vec2<f32>(globals.res_x, globals.res_y);
    let p = in.pos.xy; // píxeles físicos (y hacia abajo)

    // 1. Fondo: carcasa plateada en la franja de controles (mitad
    //    inferior en portrait; franja base en landscape). Se dibuja solo
    //    si globals.shell == 1.0.
    var color = vec3<f32>(0.0, 0.0, 0.0);
    var total_a = 0.0;
    if globals.shell > 0.5 {
        let shell_top = res.y * 0.48;
        let in_shell = step(shell_top, p.y);
        // Gradiente sutil de carcasa (plateado GBA SP).
        let t = (p.y - shell_top) / max(res.y - shell_top, 1.0);
        let plate = mix(vec3<f32>(0.66, 0.67, 0.70), vec3<f32>(0.50, 0.51, 0.56), t);
        color = plate * in_shell;
        total_a = in_shell;

        // Pozos de fondo para cruceta y clúster AB (cuencas y cápsulas hundidas)
        if elems[0].radius > 0.0 && elems[1].radius > 0.0 {
            let dpad_c = vec2<f32>(elems[0].cx, (elems[0].cy + elems[1].cy) * 0.5);
            let dpad_r = abs(elems[1].cy - elems[0].cy) * 1.08;
            let d_well = length(p - dpad_c) - dpad_r;
            if d_well < 0.0 {
                let well_mask = 1.0 - smoothstep(-2.0, 0.0, d_well);
                let well_t = (p.y - (dpad_c.y - dpad_r)) / (2.0 * dpad_r);
                let well_col = mix(vec3<f32>(0.42, 0.43, 0.48), vec3<f32>(0.58, 0.59, 0.64), well_t);
                color = mix(color, well_col, well_mask * 0.75);
            }
        }

        if elems[4].radius > 0.0 && elems[5].radius > 0.0 {
            let b_pos = vec2<f32>(elems[4].cx, elems[4].cy);
            let a_pos = vec2<f32>(elems[5].cx, elems[5].cy);
            let ba_mid = (b_pos + a_pos) * 0.5;
            let ba_vec = a_pos - b_pos;
            let ba_len = length(ba_vec);
            let u_axis = ba_vec / max(ba_len, 0.001);
            let v_axis = vec2<f32>(u_axis.y, -u_axis.x);

            var cluster_center = ba_mid;
            var half_h = elems[4].radius * 1.35;
            if elems[6].radius > 0.0 && elems[7].radius > 0.0 {
                let l_pos = vec2<f32>(elems[6].cx, elems[6].cy);
                let r_pos = vec2<f32>(elems[7].cx, elems[7].cy);
                let lr_mid = (l_pos + r_pos) * 0.5;
                cluster_center = (ba_mid + lr_mid) * 0.5;
                let lr_dist = length(lr_mid - ba_mid);
                half_h = lr_dist * 0.5 + elems[4].radius * 1.30;
            }

            let half_w = ba_len * 0.5 + elems[4].radius * 1.30;
            let p_rel = p - cluster_center;
            let p_local = vec2<f32>(dot(p_rel, u_axis), dot(p_rel, v_axis));
            let round_r = elems[4].radius * 0.95;
            let d_ab_well = sd_round_rect(p_local, vec2<f32>(0.0, 0.0), vec2<f32>(half_w, half_h), round_r);
            if d_ab_well < 0.0 {
                let ab_mask = 1.0 - smoothstep(-2.5, 0.0, d_ab_well);
                let ab_col = vec3<f32>(0.44, 0.45, 0.50);
                color = mix(color, ab_col, ab_mask * 0.65);
            }
        }
    }

    // 2. Elementos táctiles: el menor SDF gana (los elementos son
    //    disjuntos por diseño del layout).
    for (var e = 0; e < 12; e = e + 1) {
        let el = elems[e];
        if el.radius <= 0.0 {
            continue;
        }
        let c = vec2<f32>(el.cx, el.cy);
        let local = p - c;
        let d = select(
            sd_circle(p, c, el.radius),
            sd_round_rect(p, c, vec2<f32>(el.radius, el.radius * 0.62), el.round),
            el.kind > 0.5,
        );
        // Antialias de ~1.25 px.
        let aa = 1.25;
        let fill = 1.0 - smoothstep(-aa, aa, d);

        // Color: base vs pulsado (más claro y con anillo).
        var base = vec3<f32>(el.r, el.g, el.b);
        let glow = mix(base, vec3<f32>(1.0, 1.0, 1.0), 0.35);
        base = mix(base, glow, el.pressed);

        // Anillo exterior sutil siempre visible (definición del botón).
        let ring_d = abs(d) - 1.5;
        let ring_a = (1.0 - smoothstep(0.0, aa + 1.5, ring_d)) * el.ring;

        var elem_rgba = vec4<f32>(base, fill * globals.opacity);
        // Glifo encima (contraste automático con el fondo del botón, confinado por fill).
        let lum = dot(base, vec3<f32>(0.299, 0.587, 0.114));
        let ink = select(vec3<f32>(0.12, 0.13, 0.15), vec3<f32>(0.95, 0.95, 0.97), lum < 0.5);
        let u = el.radius * el.glyph_scale;
        let g = glyph_color(el.glyph, local, u, ink);
        let glyph_a = g.a * fill;
        elem_rgba = vec4<f32>(
            mix(elem_rgba.rgb, g.rgb, glyph_a),
            max(elem_rgba.a, glyph_a * globals.opacity)
        );

        let ring_col = vec3<f32>(0.95, 0.96, 1.0);
        elem_rgba = vec4<f32>(
            mix(elem_rgba.rgb, ring_col, ring_a * globals.opacity),
            max(elem_rgba.a, ring_a * globals.opacity)
        );

        // Composición "source-over" del elemento sobre el fondo.
        let a = elem_rgba.a;
        color = mix(color, elem_rgba.rgb, a);
        total_a = max(total_a, a);
    }

    return vec4<f32>(color, total_a);
}
