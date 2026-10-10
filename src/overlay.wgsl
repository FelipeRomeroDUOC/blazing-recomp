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
// Elem son 16 floats (64 B) para cumplir el alineamiento uniform a múltiplos de 16.
struct Elem {
    cx: f32,
    cy: f32,
    radius: f32,      // radio del círculo (o half-extent X para pastillas)
    r: f32,           // color
    g: f32,           // color
    b: f32,           // color
    pressed: f32,     // 1.0 = resaltado
    kind: f32,        // 0 = circle, 1 = rounded-rect (vertical / oriented pill)
    glyph: f32,       // índice de glifo (0 = ninguno)
    round: f32,       // radio de esquina (para kind 1)
    glyph_scale: f32, // tamaño del glifo relativo al radio (0.42)
    ring: f32,        // intensidad del anillo exterior (0..1)
    angle: f32,       // rotación en radianes (para L y R inclinadas)
    extent_y: f32,    // half-extent vertical
    pad1: f32,
    pad2: f32,
};

struct Globals {
    res_x: f32,
    res_y: f32,
    opacity: f32,     // 1.0 portrait, <1.0 landscape (translúcido)
    shell: f32,       // 1.0 = dibujar carcasa, 0.0 = solo elementos
};

@group(0) @binding(0) var<uniform> globals: Globals;
@group(0) @binding(1) var<uniform> elems: array<Elem, 12>;
@group(0) @binding(2) var font_texture: texture_2d<f32>;
@group(0) @binding(3) var font_sampler: sampler;
// 12 * 64 B = 768 B, múltiplo de 16: válido como uniform.

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

// ---------- Muestreo de Glifos desde el Atlas Tipográfico R8 (Fase 6) ----------

fn glyph_cell_coords(idx: u32) -> vec2<f32> {
    switch idx {
        case 6u: { return vec2<f32>(0.0, 0.0); } // 'A' (Outfit Bold)
        case 5u: { return vec2<f32>(1.0, 0.0); } // 'B' (Outfit Bold)
        case 7u: { return vec2<f32>(2.0, 0.0); } // 'L' (Outfit Bold)
        case 8u: { return vec2<f32>(3.0, 0.0); } // 'R' (Outfit Bold)
        case 1u: { return vec2<f32>(0.0, 1.0); } // Flecha Arriba
        case 2u: { return vec2<f32>(1.0, 1.0); } // Flecha Abajo
        case 3u: { return vec2<f32>(2.0, 1.0); } // Flecha Izquierda
        case 4u: { return vec2<f32>(3.0, 1.0); } // Flecha Derecha
        default: { return vec2<f32>(0.0, 0.0); }
    }
}

fn glyph_color(idx: f32, local: vec2<f32>, u: f32, ink: vec3<f32>) -> vec4<f32> {
    let i = u32(idx + 0.5);
    if i == 0u {
        return vec4<f32>(0.0);
    }

    // Casos 1..8: Glifos individuales en celdas cuadradas de 64x64 px (0.25 x 0.25 UV)
    if i <= 8u {
        // Para botones de acción (A, B, L, R: 5..8), escala generosa y legible.
        // Para flechas de cruceta (1..4), escala adaptada al ancho del brazo.
        let scale_factor = select(5.2, 3.6, i <= 4u);
        let box_size = max(u * scale_factor, 14.0);
        let q = (local / box_size) + vec2<f32>(0.5, 0.5);
        if q.x < 0.0 || q.x > 1.0 || q.y < 0.0 || q.y > 1.0 {
            return vec4<f32>(0.0);
        }
        let cell = glyph_cell_coords(i);
        let uv = vec2<f32>((cell.x + q.x) * 0.25, (cell.y + q.y) * 0.25);
        let coverage = textureSampleLevel(font_texture, font_sampler, uv, 0.0).r;
        return vec4<f32>(ink, coverage);
    }

    // Casos 9..10: Etiquetas SELECT y START (JetBrains Mono Bold, fila 2, celdas de 128x64 px)
    if i == 9u || i == 10u {
        let half_w = max(u * 3.8, 24.0);
        let half_h = max(u * 1.9, 12.0);
        let q = vec2<f32>(local.x / (half_w * 2.0) + 0.5, local.y / (half_h * 2.0) + 0.5);
        if q.x < 0.0 || q.x > 1.0 || q.y < 0.0 || q.y > 1.0 {
            return vec4<f32>(0.0);
        }
        let col_off = select(0.50, 0.00, i == 9u);
        let uv = vec2<f32>(col_off + q.x * 0.50, 0.50 + q.y * 0.25);
        let coverage = textureSampleLevel(font_texture, font_sampler, uv, 0.0).r;
        return vec4<f32>(ink, coverage);
    }

    return vec4<f32>(0.0);
}

// ---------- Fragmento ----------

@fragment
fn fs_main(in: VSOut) -> @location(0) vec4<f32> {
    let res = vec2<f32>(globals.res_x, globals.res_y);
    let p = in.pos.xy; // píxeles físicos (y hacia abajo)

    // En Landscape (shell <= 0.5), el fondo es 100% transparente salvo donde hay botones.
    // Poda temprana global: si el fragmento no cae dentro del margen holgado de ninguno de
    // los tres grupos de controles (cruceta, clúster B/A/L/R o Select/Start), descartar de inmediato.
    if globals.shell <= 0.5 {
        // En landscape, todos los controles residen en la mitad inferior (y >= 0.40).
        // Descarte inmediato del tercio superior (área pura del juego) ahorra 40% de fillrate en tablets.
        if p.y < res.y * 0.38 {
            return vec4<f32>(0.0, 0.0, 0.0, 0.0);
        }
        var near_any = false;
        // 1. Cruceta (elems 0..3)
        if elems[0].radius > 0.0 && elems[1].radius > 0.0 {
            let dpad_c = vec2<f32>(elems[0].cx, (elems[0].cy + elems[1].cy) * 0.5);
            let arm_len = abs(elems[1].cy - elems[0].cy) * 0.5;
            let dpad_bb = arm_len + elems[0].radius * 0.85 + 28.0;
            if abs(p.x - dpad_c.x) <= dpad_bb && abs(p.y - dpad_c.y) <= dpad_bb {
                near_any = true;
            }
        }
        // 2. Clúster de acción (elems 4..7: B, A, L, R)
        if !near_any && elems[4].radius > 0.0 && elems[5].radius > 0.0 {
            let b_pos = vec2<f32>(elems[4].cx, elems[4].cy);
            let a_pos = vec2<f32>(elems[5].cx, elems[5].cy);
            let cluster_mid = (b_pos + a_pos) * 0.5;
            let cluster_span = length(a_pos - b_pos) * 0.5 + max(elems[4].radius, elems[6].radius) * 1.8 + 28.0;
            if abs(p.x - cluster_mid.x) <= cluster_span && abs(p.y - cluster_mid.y) <= cluster_span {
                near_any = true;
            }
        }
        // 3. Select y Start (elems 8..9)
        if !near_any && elems[8].radius > 0.0 && elems[9].radius > 0.0 {
            let ss_mid = vec2<f32>((elems[8].cx + elems[9].cx) * 0.5, elems[8].cy);
            let ss_span_x = abs(elems[9].cx - elems[8].cx) * 0.5 + elems[8].radius + 28.0;
            let ss_span_y = elems[8].extent_y * 2.8 + 36.0;
            if abs(p.x - ss_mid.x) <= ss_span_x && abs(p.y - ss_mid.y) <= ss_span_y {
                near_any = true;
            }
        }
        if !near_any {
            return vec4<f32>(0.0, 0.0, 0.0, 0.0);
        }
    } else {
        // En Portrait (shell > 0.5), la mitad superior es la pantalla del juego.
        // Poda temprana: por encima de la carcasa no hay ni botones ni fondo.
        let shell_top = res.y * 0.49;
        if p.y < (shell_top - 12.0) {
            return vec4<f32>(0.0, 0.0, 0.0, 0.0);
        }
    }

    // 1. Fondo: carcasa plateada GBA SP en portrait con esquinas redondeadas
    //    y márgenes laterales (fidelidad al prototipo layout_tactil_v2.html).
    var color = vec3<f32>(0.0, 0.0, 0.0);
    var total_a = 0.0;
    if globals.shell > 0.5 {
        let shell_top = res.y * 0.49;
        // Poda temprana: en portrait, toda la mitad superior (pantalla de juego) está vacía
        // en el overlay; descartar píxeles superiores ahorra 50% de fillrate en GPUs de tablet.
        if p.y < shell_top - 8.0 {
            return vec4<f32>(0.0);
        }
        let corner_r = min(res.x * 0.08, 36.0);
        var d_shell = p.y - shell_top;
        if p.y < shell_top + corner_r {
            if p.x < corner_r {
                d_shell = corner_r - length(p - vec2<f32>(corner_r, shell_top + corner_r));
            } else if p.x > res.x - corner_r {
                d_shell = corner_r - length(p - vec2<f32>(res.x - corner_r, shell_top + corner_r));
            }
        }
        let in_shell = smoothstep(-1.0, 1.0, d_shell);

        // Gradiente plateado claro (#dcdee6 a #b3b7c7) que cubre el cuerpo del control
        let t_plate = clamp((p.y - shell_top) / max(res.y - shell_top, 1.0), 0.0, 1.0);
        let plate_light = vec3<f32>(0.86, 0.87, 0.90);
        let plate_dark  = vec3<f32>(0.70, 0.72, 0.78);
        var plate = mix(plate_light, plate_dark, t_plate);

        // Bisel superior iluminado continuo a lo largo del borde y esquinas redondeadas
        let bevel_t = (1.0 - smoothstep(0.0, 5.0, abs(d_shell))) * in_shell;
        plate = mix(plate, vec3<f32>(0.96, 0.97, 1.0), bevel_t * 0.55);

        // Sombra suave superior proyectada sobre el marco del juego
        let top_shadow = (1.0 - smoothstep(-8.0, 0.0, d_shell)) * 0.45;

        color = mix(vec3<f32>(0.0, 0.0, 0.0), plate, in_shell);
        total_a = max(in_shell, top_shadow);

        // 1. Hendidura 3D para la cruceta (cuenca circular hundida)
        if elems[0].radius > 0.0 && elems[1].radius > 0.0 {
            let dpad_c = vec2<f32>(elems[0].cx, (elems[0].cy + elems[1].cy) * 0.5);
            let arm_len = abs(elems[1].cy - elems[0].cy) * 0.5;
            let dpad_r = arm_len + elems[0].radius * 1.15;
            let d_well_bb = dpad_r + 6.0;

            if abs(p.x - dpad_c.x) <= d_well_bb && abs(p.y - dpad_c.y) <= d_well_bb {
                let d_well = length(p - dpad_c) - dpad_r;

                // Bisel exterior iluminado (reborde exterior inferior de la hendidura)
                let outer_lip = (1.0 - smoothstep(0.0, 3.0, abs(d_well))) * clamp((p.y - dpad_c.y) / dpad_r * 0.8 + 0.2, 0.0, 1.0) * 0.55;
                color = mix(color, vec3<f32>(0.96, 0.97, 1.0), outer_lip * in_shell);

                if d_well < 0.0 {
                    let well_mask = 1.0 - smoothstep(-2.5, 0.0, d_well);
                    // Degradado del fondo de la cuenca
                    let well_t = (p.y - (dpad_c.y - dpad_r)) / (2.0 * dpad_r);
                    var well_col = mix(vec3<f32>(0.38, 0.39, 0.45), vec3<f32>(0.60, 0.61, 0.67), well_t);

                    // Profundidad 3D: Sombra interior superior (pared superior de la cuenca)
                    let norm_w = (p - dpad_c) / dpad_r;
                    let top_inner_shadow = smoothstep(-dpad_r * 0.45, 0.0, d_well) * clamp(-norm_w.y * 0.80 - norm_w.x * 0.35 + 0.20, 0.0, 1.0);
                    well_col = mix(well_col, vec3<f32>(0.12, 0.13, 0.16), top_inner_shadow * 0.90);

                    // Profundidad 3D: Labio interior inferior iluminado (pared inferior)
                    let bottom_inner_lip = smoothstep(-dpad_r * 0.25, 0.0, d_well) * clamp(norm_w.y * 0.80 + norm_w.x * 0.35, 0.0, 1.0);
                    well_col = mix(well_col, vec3<f32>(0.88, 0.90, 0.96), bottom_inner_lip * 0.50);

                    color = mix(color, well_col, well_mask);
                }
            }
        }

        // 2. Hendidura 3D para clúster de acción (cápsula redondeada simétricamente balanceada)
        if elems[4].radius > 0.0 && elems[5].radius > 0.0 {
            let b_pos = vec2<f32>(elems[4].cx, elems[4].cy);
            let a_pos = vec2<f32>(elems[5].cx, elems[5].cy);
            let ba_vec = a_pos - b_pos;
            let ba_len = length(ba_vec);
            let u_axis = ba_vec / max(ba_len, 0.001);
            let v_axis = vec2<f32>(u_axis.y, -u_axis.x);

            var cluster_center = (b_pos + a_pos) * 0.5;
            var half_h = elems[4].radius * 1.55;
            if elems[6].radius > 0.0 && elems[7].radius > 0.0 {
                let l_pos = vec2<f32>(elems[6].cx, elems[6].cy);
                let r_pos = vec2<f32>(elems[7].cx, elems[7].cy);
                let lr_mid = (l_pos + r_pos) * 0.5;
                let ba_mid = (b_pos + a_pos) * 0.5;
                cluster_center = (ba_mid + lr_mid) * 0.5;
                let row_dist = length(lr_mid - ba_mid);
                half_h = row_dist * 0.5 + max(elems[4].radius, elems[6].radius) * 1.35;
            }

            let half_w = ba_len * 0.5 + elems[4].radius * 1.30;
            let p_rel = p - cluster_center;
            let ab_bb = max(half_w, half_h) * 1.25 + 8.0;

            if abs(p_rel.x) <= ab_bb && abs(p_rel.y) <= ab_bb {
                let p_local = vec2<f32>(dot(p_rel, u_axis), dot(p_rel, v_axis));
                let round_r = min(half_w, half_h) * 0.65;
                let d_ab_well = sd_round_rect(p_local, vec2<f32>(0.0, 0.0), vec2<f32>(half_w, half_h), round_r);

                // Bisel exterior iluminado (reborde exterior inferior de la cápsula)
                let norm_ab = p_rel / max(half_w, half_h);
                let ab_outer_lip = (1.0 - smoothstep(0.0, 3.0, abs(d_ab_well))) * clamp(norm_ab.y * 0.80 + norm_ab.x * 0.35 + 0.20, 0.0, 1.0) * 0.55;
                color = mix(color, vec3<f32>(0.96, 0.97, 1.0), ab_outer_lip * in_shell);

                if d_ab_well < 0.0 {
                    let ab_mask = 1.0 - smoothstep(-2.5, 0.0, d_ab_well);
                    // Degradado del fondo de la cápsula
                    let t_ab = clamp(norm_ab.y * 0.5 + 0.5, 0.0, 1.0);
                    var ab_col = mix(vec3<f32>(0.38, 0.39, 0.45), vec3<f32>(0.60, 0.61, 0.67), t_ab);

                    // Profundidad 3D: Sombra interior superior (pared superior de la cápsula hundida)
                    let top_inner_shadow = smoothstep(-half_h * 0.45, 0.0, d_ab_well) * clamp(-norm_ab.y * 0.80 - norm_ab.x * 0.35 + 0.20, 0.0, 1.0);
                    ab_col = mix(ab_col, vec3<f32>(0.12, 0.13, 0.16), top_inner_shadow * 0.90);

                    // Profundidad 3D: Labio interior inferior iluminado (pared inferior)
                    let bottom_inner_lip = smoothstep(-half_h * 0.25, 0.0, d_ab_well) * clamp(norm_ab.y * 0.80 + norm_ab.x * 0.35, 0.0, 1.0);
                    ab_col = mix(ab_col, vec3<f32>(0.88, 0.90, 0.96), bottom_inner_lip * 0.50);

                    color = mix(color, ab_col, ab_mask);
                }
            }
        }
    }

    // 2. Cruceta continua unificada en cruz (elementos 0..3: Up, Down, Left, Right)
    //    con brazos redondeados y centro cuadrado (estilo GBA SP / layout_tactil_v2.html).
    if elems[0].radius > 0.0 && elems[1].radius > 0.0 {
        let dpad_c = vec2<f32>(elems[0].cx, (elems[0].cy + elems[1].cy) * 0.5);
        let arm_len = abs(elems[1].cy - elems[0].cy) * 0.5;
        let r_base = elems[0].radius;
        let half_len = arm_len + r_base * 0.85;
        let half_thick = r_base * 0.82;
        let corner_r = r_base * 0.30;
        let cross_bb = half_len + 28.0;

        // Poda AABB: solo evaluar si el píxel cae dentro de la caja de la cruz + sombras
        if abs(p.x - dpad_c.x) <= cross_bb && abs(p.y - dpad_c.y) <= cross_bb {
            let d_v = sd_round_rect(p, dpad_c, vec2<f32>(half_thick, half_len), corner_r);
            let d_h = sd_round_rect(p, dpad_c, vec2<f32>(half_len, half_thick), corner_r);
            let d_cross = min(d_v, d_h);

            let aa = 1.25;
            let fill = 1.0 - smoothstep(-aa, aa, d_cross);

            // 2a. Sombra proyectada exterior inferior de la cruz completa
            let shadow_off = vec2<f32>(0.0, max(half_thick * 0.15, 3.5));
            let p_sh = p - shadow_off;
            let d_sh_v = sd_round_rect(p_sh, dpad_c, vec2<f32>(half_thick, half_len), corner_r);
            let d_sh_h = sd_round_rect(p_sh, dpad_c, vec2<f32>(half_len, half_thick), corner_r);
            let d_cross_sh = min(d_sh_v, d_sh_h);
            let btn_shadow = (1.0 - smoothstep(-2.0, half_thick * 0.35 + 8.0, d_cross_sh)) * 0.42 * (1.0 - fill);
            color = mix(color, vec3<f32>(0.06, 0.07, 0.10), btn_shadow * globals.opacity);
            total_a = max(total_a, btn_shadow * globals.opacity);

            if fill > 0.001 {
                let p_rel = p - dpad_c;
                // Identificar qué dirección o núcleo corresponde a este punto
                var el = elems[0];
                var arm_local = p - vec2<f32>(elems[0].cx, elems[0].cy);
                if abs(p_rel.y) >= abs(p_rel.x) {
                    if p_rel.y > 0.0 {
                        el = elems[1]; // Down
                        arm_local = p - vec2<f32>(elems[1].cx, elems[1].cy);
                    }
                } else {
                    if p_rel.x < 0.0 {
                        el = elems[2]; // Left
                        arm_local = p - vec2<f32>(elems[2].cx, elems[2].cy);
                    } else {
                        el = elems[3]; // Right
                        arm_local = p - vec2<f32>(elems[3].cx, elems[3].cy);
                    }
                }

                let is_center = max(abs(p_rel.x), abs(p_rel.y)) < half_thick * 0.68;
                let pressed = select(el.pressed, 0.0, is_center);

                // Degradado volumétrico a 145° (#a4a9c5 a #7b819f)
                let norm_cross = p_rel / max(half_len, 1.0);
                let t_btn = clamp(dot(norm_cross, vec2<f32>(0.57, 0.82)) * 0.5 + 0.5, 0.0, 1.0);
                let col_light = vec3<f32>(el.r * 1.18, el.g * 1.18, el.b * 1.12);
                let col_dark  = vec3<f32>(el.r * 0.80, el.g * 0.80, el.b * 0.82);
                var base = mix(col_light, col_dark, t_btn);

                let is_landscape = globals.shell < 0.5;

                if is_center {
                    if !is_landscape {
                        base = base * 0.90; // Núcleo central ligeramente hundido (solo en portrait)
                        let d_pivot = length(p_rel) - half_thick * 0.40;
                        if d_pivot < 0.0 {
                            base = mix(base, base * 0.84, 1.0 - smoothstep(-1.5, 0.0, d_pivot));
                        }
                    }
                }

                // Brillo especular interior / bisel superior
                let inset_spec = clamp(-norm_cross.y, 0.0, 1.0) * (1.0 - smoothstep(-half_thick * 0.45, 0.0, d_cross));
                let spec_mult = select(0.28, 0.16, is_landscape);
                base = mix(base, vec3<f32>(1.0, 1.0, 1.0), inset_spec * spec_mult);

                // Feedback de pulsado (Paso 7: teñir brazo activo con azul eléctrico #5b8def)
                let dpad_blue = mix(vec3<f32>(0.42, 0.62, 0.98), vec3<f32>(0.32, 0.50, 0.90), t_btn);
                base = mix(base, dpad_blue, pressed);

                // Borde exterior biselado (borde blanco tenue de cristal en landscape)
                let ring_d = abs(d_cross) - 1.5;
                let ring_a = (1.0 - smoothstep(0.0, aa + 1.5, ring_d)) * el.ring;
                let ring_base = select(
                    mix(vec3<f32>(0.50, 0.52, 0.58), vec3<f32>(0.95, 0.96, 1.0), clamp(-norm_cross.y * 0.5 + 0.5, 0.0, 1.0)),
                    vec3<f32>(0.85, 0.88, 0.95),
                    is_landscape
                );
                let ring_light = mix(ring_base, vec3<f32>(0.75, 0.88, 1.0), pressed);

                var cross_rgba = vec4<f32>(base, fill * globals.opacity);

                // Glifo de flecha en cada brazo si no es el núcleo central
                if !is_center {
                    let lum = dot(base, vec3<f32>(0.299, 0.587, 0.114));
                    var ink = select(vec3<f32>(0.12, 0.13, 0.15), vec3<f32>(0.95, 0.95, 0.97), lum < 0.5);
                    if pressed > 0.5 {
                        ink = vec3<f32>(1.0, 1.0, 1.0); // flecha blanca pura sobre el fondo azul #5b8def
                    }
                    let u = el.radius * el.glyph_scale;
                    let g = glyph_color(el.glyph, arm_local, u, ink);
                    let glyph_a = g.a * fill;
                    cross_rgba = vec4<f32>(
                        mix(cross_rgba.rgb, g.rgb, glyph_a),
                        max(cross_rgba.a, glyph_a * globals.opacity)
                    );
                }

                cross_rgba = vec4<f32>(
                    mix(cross_rgba.rgb, ring_light, ring_a * globals.opacity),
                    max(cross_rgba.a, ring_a * globals.opacity)
                );

                let a = cross_rgba.a;
                color = mix(color, cross_rgba.rgb, a);
                total_a = max(total_a, a);
            }
        }
    }

    // 3. Resto de elementos táctiles (B, A, L, R, Select, Start: índices 4..11)
    for (var e = 4; e < 12; e = e + 1) {
        let el = elems[e];
        if el.radius <= 0.0 {
            continue;
        }
        let c = vec2<f32>(el.cx, el.cy);
        let local = p - c;
        let max_r = max(el.radius, el.extent_y);
        let bb_x = max_r * 1.40 + 20.0;
        let bb_y_down = max_r * 1.40 + 20.0;
        let bb_y_up = select(bb_y_down, max_r * 2.80 + 36.0, el.glyph > 8.5);

        // Poda AABB: descartar píxeles que no tocan este botón ni su sombra/etiqueta
        if abs(local.x) > bb_x || local.y > bb_y_down || local.y < -bb_y_up {
            continue;
        }

        let cos_a = cos(el.angle);
        let sin_a = sin(el.angle);
        let rot_l = vec2<f32>(
            local.x * cos_a + local.y * sin_a,
           -local.x * sin_a + local.y * cos_a
        );

        // Paso 7: Escala visual al 92% al estar presionado (el.pressed > 0.5)
        let scale_press = select(1.0, 0.92, el.pressed > 0.5);
        let vis_rad = el.radius * scale_press;
        let vis_extent_y = el.extent_y * scale_press;
        let vis_round = el.round * scale_press;

        let d = select(
            sd_circle(p, c, vis_rad),
            sd_round_rect(rot_l, vec2<f32>(0.0, 0.0), vec2<f32>(vis_rad, vis_extent_y), vis_round),
            el.kind > 0.5,
        );
        // Antialias de ~1.25 px.
        let aa = 1.25;
        let fill = 1.0 - smoothstep(-aa, aa, d);

        // 2a. Sombra proyectada exterior inferior (0 5px 12px rgba(0,0,0,0.45)), comprimida al pulsar
        let shadow_off = vec2<f32>(0.0, max(vis_extent_y * 0.12, 2.5) * scale_press);
        let p_sh = p - shadow_off;
        let sh_local = p_sh - c;
        let rot_sh = vec2<f32>(
            sh_local.x * cos_a + sh_local.y * sin_a,
           -sh_local.x * sin_a + sh_local.y * cos_a
        );
        let d_sh = select(
            sd_circle(p_sh, c, vis_rad),
            sd_round_rect(rot_sh, vec2<f32>(0.0, 0.0), vec2<f32>(vis_rad, vis_extent_y), vis_round),
            el.kind > 0.5,
        );
        let btn_shadow = (1.0 - smoothstep(-2.0, vis_extent_y * 0.35 + 8.0, d_sh)) * 0.42 * (1.0 - fill);
        color = mix(color, vec3<f32>(0.06, 0.07, 0.10), btn_shadow * globals.opacity);
        total_a = max(total_a, btn_shadow * globals.opacity);

        // 2b. Degradado volumétrico a 145° (#a4a9c5 a #7b819f)
        let norm_l = local / max(vis_rad, 1.0);
        let t_btn = clamp(dot(norm_l, vec2<f32>(0.57, 0.82)) * 0.5 + 0.5, 0.0, 1.0);
        let col_light = vec3<f32>(el.r * 1.18, el.g * 1.18, el.b * 1.12);
        let col_dark  = vec3<f32>(el.r * 0.80, el.g * 0.80, el.b * 0.82);
        var base = mix(col_light, col_dark, t_btn);

        // 2c. Brillo especular interior / bisel superior (inset 0 2px 4px)
        let inset_spec = clamp(-norm_l.y, 0.0, 1.0) * (1.0 - smoothstep(-vis_rad * 0.45, 0.0, d));
        let spec_mult = select(0.28, 0.16, globals.shell < 0.5);
        base = mix(base, vec3<f32>(1.0, 1.0, 1.0), inset_spec * spec_mult);

        // Feedback pulsado (brillo / realce)
        let glow = mix(base, vec3<f32>(1.0, 1.0, 1.0), 0.35);
        base = mix(base, glow, el.pressed);

        // 2d. Borde exterior biselado (border: 2px solid rgba(255,255,255,0.35) portrait; 1.5px rgba(255,255,255,0.28) landscape)
        let ring_d = abs(d) - 1.5;
        let ring_a = (1.0 - smoothstep(0.0, aa + 1.5, ring_d)) * el.ring;
        let ring_light = select(
            mix(vec3<f32>(0.50, 0.52, 0.58), vec3<f32>(0.95, 0.96, 1.0), clamp(-norm_l.y * 0.5 + 0.5, 0.0, 1.0)),
            vec3<f32>(0.85, 0.88, 0.95),
            globals.shell < 0.5
        );

        var elem_rgba = vec4<f32>(base, fill * globals.opacity);
        if el.glyph > 8.5 {
            // Etiqueta SELECT / START en JetBrains Mono situada arriba de la pastilla
            let label_c = vec2<f32>(el.cx, el.cy - el.extent_y - max(el.extent_y * 0.70, 14.0));
            let label_local = p - label_c;
            let label_ink = select(vec3<f32>(0.29, 0.31, 0.41), vec3<f32>(0.92, 0.94, 0.98), globals.shell < 0.5);
            let g_lbl = glyph_color(el.glyph, label_local, el.radius * 0.35, label_ink);
            if g_lbl.a > 0.001 {
                let la = g_lbl.a * globals.opacity;
                color = mix(color, g_lbl.rgb, la);
                total_a = max(total_a, la);
            }
        } else {
            // Glifos de acción 'A', 'B', 'L', 'R' dentro del cuerpo del botón (escalados con vis_rad)
            let lum = dot(base, vec3<f32>(0.299, 0.587, 0.114));
            let ink = select(vec3<f32>(0.12, 0.13, 0.15), vec3<f32>(0.95, 0.95, 0.97), lum < 0.5);
            let u = vis_rad * el.glyph_scale;
            let g = glyph_color(el.glyph, local, u, ink);
            let glyph_a = g.a * fill;
            elem_rgba = vec4<f32>(
                mix(elem_rgba.rgb, g.rgb, glyph_a),
                max(elem_rgba.a, glyph_a * globals.opacity)
            );
        }

        elem_rgba = vec4<f32>(
            mix(elem_rgba.rgb, ring_light, ring_a * globals.opacity),
            max(elem_rgba.a, ring_a * globals.opacity)
        );

        // Composición "source-over" del elemento sobre el fondo.
        let a = elem_rgba.a;
        color = mix(color, elem_rgba.rgb, a);
        total_a = max(total_a, a);
    }

    return vec4<f32>(color, total_a);
}
