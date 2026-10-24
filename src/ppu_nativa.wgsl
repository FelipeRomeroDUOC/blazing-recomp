// ===========================================================================
// PPU nativa: capas en G-buffer + composición por píxel.
//
// El modelo es el de la PPU original de la GBA, no el del shim anterior:
//   1. Cada fondo se rasteriza a su propia textura (G-buffer), igual que cada
//      sprite va a la suya. Cada píxel guarda su color de 15 bits.
//   2. Una pasada final elige, píxel a píxel, las dos capas más altas y les
//      aplica la unidad de mezcla (alfa, aclarar, oscurecer) y las ventanas.
//
// La aritmética de la unidad de mezcla es la de `gba-core` (`ppu.rs`): el verde
// pasa por un carril de 6 bits, la acumulación redondea a media hacia arriba
// (`+8 >> 4`) y el sustraendo del oscurecer redondea a media hacia abajo (`+7`).
// ===========================================================================

struct CapaFondo {
    enabled: u32,
    priority: u32,
    char_base: u32,
    screen_base: u32,
    screen_size: u32,
    is_8bpp: u32,
    scroll_x: u32,
    scroll_y: u32,
    es_afin: u32,
    afin_wrap: u32,
    afin_pa: i32,
    afin_pb: i32,
    afin_pc: i32,
    afin_pd: i32,
    afin_ref_x: i32,
    afin_ref_y: i32,
};

struct ConfigCapa {
    modo: u32,
    _pad0: u32,
    _pad1: u32,
    _pad2: u32,
    capas: array<CapaFondo, 4>,
};

struct ConfigComp {
    dispcnt: u32,
    bg_prio0: u32,
    bg_prio1: u32,
    bg_prio2: u32,
    bg_prio3: u32,
    win0h: u32,
    win1h: u32,
    win0v: u32,
    win1v: u32,
    winin: u32,
    winout: u32,
    bldcnt: u32,
    bldalpha: u32,
    bldy: u32,
    _pad0: u32,
    _pad1: u32,
};

struct SpriteNativa {
    pos_x: f32,
    pos_y: f32,
    caja_x: f32,
    caja_y: f32,
    tex_x: f32,
    tex_y: f32,
    base_tile: u32,
    palette_idx: u32,
    flags: u32,
    prioridad: u32,
    afin_a: i32,
    afin_b: i32,
    afin_c: i32,
    afin_d: i32,
    oam_index: u32,
};

@group(0) @binding(0) var<storage, read> vram: array<u32>;
@group(0) @binding(1) var<storage, read> paleta: array<u32>;
@group(0) @binding(2) var<storage, read> filas_capa: array<ConfigCapa>;
@group(0) @binding(3) var<storage, read> sprites: array<SpriteNativa>;
@group(0) @binding(4) var<storage, read> filas_comp: array<ConfigComp>;
@group(0) @binding(5) var tex_obj: texture_2d<u32>;
@group(0) @binding(6) var tex_obj_ventana: texture_2d<u32>;

// Bandera: sprite que se descarta (solapa con un fondo bitmap).
const FLAG_HFLIP: u32 = 1u;
const FLAG_VFLIP: u32 = 2u;
const FLAG_SEMI: u32 = 4u;
const FLAG_8BPP: u32 = 16u;
const FLAG_1D: u32 = 32u;
const FLAG_AFIN: u32 = 64u;

// ===========================================================================
// Lectura de VRAM
// ===========================================================================

fn vram_byte(addr: u32) -> u32 {
    let word = vram[addr / 4u];
    return (word >> ((addr % 4u) * 8u)) & 0xFFu;
}

fn vram_u16(addr: u32) -> u32 {
    let word = vram[addr / 4u];
    return (word >> ((addr % 4u) * 8u)) & 0xFFFFu;
}

// ===========================================================================
// Unidad de mezcla (la misma aritmética que `gba-core`)
// ===========================================================================

fn split6(c: u32) -> vec3<u32> {
    return vec3<u32>(c & 31u, ((c >> 5u) & 31u) << 1u, (c >> 10u) & 31u);
}

fn join6(r: u32, g: u32, b: u32) -> u32 {
    return r | ((g >> 1u) << 5u) | (b << 10u);
}

fn alpha_blend(a: u32, b: u32, eva: u32, evb: u32) -> u32 {
    let x = split6(a);
    let y = split6(b);
    let r = min(31u, (x.r * eva + y.r * evb + 8u) >> 4u);
    let g = min(63u, (x.g * eva + y.g * evb + 8u) >> 4u);
    let bl = min(31u, (x.b * eva + y.b * evb + 8u) >> 4u);
    return join6(r, g, bl);
}

fn brighten(c: u32, evy: u32) -> u32 {
    let x = split6(c);
    return join6(
        x.r + (((31u - x.r) * evy + 8u) >> 4u),
        x.g + (((63u - x.g) * evy + 8u) >> 4u),
        x.b + (((31u - x.b) * evy + 8u) >> 4u),
    );
}

fn darken(c: u32, evy: u32) -> u32 {
    let x = split6(c);
    return join6(
        x.r - ((x.r * evy + 7u) >> 4u),
        x.g - ((x.g * evy + 7u) >> 4u),
        x.b - ((x.b * evy + 7u) >> 4u),
    );
}

// ===========================================================================
// Fondos afines
// ===========================================================================

/// Resto euclídeo: el resultado siempre es mayor o igual que cero.
///
/// WGSL no lo trae: su `%` conserva el signo del dividendo, como el `%` de
/// Rust, y la referencia usa `rem_euclid` para la vuelta al mapa.
fn rem_euclid(a: i32, b: i32) -> i32 {
    let r = a % b;
    if (r < 0) {
        return r + b;
    }
    return r;
}

/// Color del píxel de un fondo afín, o `0xFFFF` si es transparente.
///
/// Reproduce `render_affine_bg` de la referencia:
///
/// - El ancla de la línea es `ref + línea·(PB, PD)`.
/// - El paso por píxel es `PA` en horizontal y `PC` en vertical.
/// - Las coordenadas son de 20.8 con signo: se desplazan 8 bits a la derecha.
/// - Con wrap, las coordenadas dan la vuelta al tamaño del mapa (módulo con
///   signo euclídeo); sin wrap, fuera del mapa el píxel es transparente.
/// - El mapa afín tiene una entrada de **1 byte** por tile y los caracteres
///   son de 8 bits por píxel; el índice 0 es transparente.
fn color_capa_afin(upy: u32, idx_capa: u32, px: u32, py: u32) -> u32 {
    let enabled = filas_capa[upy].capas[idx_capa].enabled;
    if (enabled == 0u) {
        return 0xFFFFu;
    }

    // Tamaño del mapa: 16, 32, 64 o 128 tiles por lado (16 << bits 14-15).
    let screen_size = filas_capa[upy].capas[idx_capa].screen_size;
    let size_tiles = 16u << screen_size;
    let size_px = i32(size_tiles * 8u);

    // Ancla de la línea más el paso por píxel.
    let i = i32(px);
    let linea = i32(py);
    let pa = filas_capa[upy].capas[idx_capa].afin_pa;
    let pb = filas_capa[upy].capas[idx_capa].afin_pb;
    let pc = filas_capa[upy].capas[idx_capa].afin_pc;
    let pd = filas_capa[upy].capas[idx_capa].afin_pd;
    let ref_x = filas_capa[upy].capas[idx_capa].afin_ref_x;
    let ref_y = filas_capa[upy].capas[idx_capa].afin_ref_y;
    let wrap = filas_capa[upy].capas[idx_capa].afin_wrap;

    let fx = ref_x + linea * pb + i * pa;
    let fy = ref_y + linea * pd + i * pc;

    var tx = fx >> 8;
    var ty = fy >> 8;

    if (wrap != 0u) {
        tx = rem_euclid(tx, size_px);
        ty = rem_euclid(ty, size_px);
    } else if (tx < 0 || ty < 0 || tx >= size_px || ty >= size_px) {
        return 0xFFFFu;
    }

    let screen_base = filas_capa[upy].capas[idx_capa].screen_base;
    let char_base = filas_capa[upy].capas[idx_capa].char_base;

    // El mapa afín tiene una entrada de 1 byte por tile.
    let entrada = screen_base + u32(ty / 8) * size_tiles + u32(tx / 8);
    if (entrada >= 0x18000u) {
        return 0xFFFFu;
    }
    let tile = vram_byte(entrada);
    let addr = char_base + tile * 64u + u32(ty % 8) * 8u + u32(tx % 8);
    if (addr >= 0x10000u) {
        return 0xFFFFu;
    }
    let idx = vram_byte(addr);
    if (idx == 0u) {
        return 0xFFFFu;
    }
    return paleta[py * 512u + idx];
}

// ===========================================================================
// Fondos de tiles
// ===========================================================================

/// Color del píxel de un fondo, o `0xFFFF` si es transparente.
///
/// Reproduce `render_text_bg` de la referencia: mismo desplazamiento, mismo
/// reparto de bloques de pantalla, mismos voltees y mismo límite de 0x10000
/// para los datos de tile.
fn color_capa(upy: u32, idx_capa: u32, px: u32, py: u32) -> u32 {
    let enabled = filas_capa[upy].capas[idx_capa].enabled;
    if (enabled == 0u) {
        return 0xFFFFu;
    }

    // Un fondo afín tiene su propio muestreo: no usa el desplazamiento.
    let es_afin = filas_capa[upy].capas[idx_capa].es_afin;
    if (es_afin != 0u) {
        return color_capa_afin(upy, idx_capa, px, py);
    }

    let screen_size = filas_capa[upy].capas[idx_capa].screen_size;
    var w_tiles: u32 = 32u;
    var h_tiles: u32 = 32u;
    if (screen_size == 1u) {
        w_tiles = 64u;
    } else if (screen_size == 2u) {
        h_tiles = 64u;
    } else if (screen_size == 3u) {
        w_tiles = 64u;
        h_tiles = 64u;
    }

    let scroll_y = filas_capa[upy].capas[idx_capa].scroll_y;
    let scroll_x = filas_capa[upy].capas[idx_capa].scroll_x;
    let screen_base = filas_capa[upy].capas[idx_capa].screen_base;
    let char_base = filas_capa[upy].capas[idx_capa].char_base;
    let is_8bpp = filas_capa[upy].capas[idx_capa].is_8bpp;

    let y = (py + scroll_y) & (h_tiles * 8u - 1u);
    let x = (px + scroll_x) & (w_tiles * 8u - 1u);
    let ty = y / 8u;
    let tx = x / 8u;
    let fy0 = y % 8u;
    let fx0 = x % 8u;

    let block = (ty / 32u) * (w_tiles / 32u) + tx / 32u;
    let entry_idx = block * 1024u + (ty % 32u) * 32u + (tx % 32u);
    let entry = vram_u16(screen_base + entry_idx * 2u);

    let tile = entry & 0x3FFu;
    let fx = select(fx0, 7u - fx0, (entry & 0x400u) != 0u);
    let fy = select(fy0, 7u - fy0, (entry & 0x800u) != 0u);

    if (is_8bpp != 0u) {
        let addr = char_base + tile * 64u + fy * 8u + fx;
        if (addr >= 0x10000u) {
            return 0xFFFFu;
        }
        let idx = vram_byte(addr);
        if (idx == 0u) {
            return 0xFFFFu;
        }
        return paleta[py * 512u + idx];
    }

    let addr = char_base + tile * 32u + fy * 4u;
    if (addr + 4u > 0x10000u) {
        return 0xFFFFu;
    }
    let byte = vram_byte(addr + fx / 2u);
    var nib = byte & 0x0Fu;
    if ((fx & 1u) != 0u) {
        nib = (byte >> 4u) & 0x0Fu;
    }
    if (nib == 0u) {
        return 0xFFFFu;
    }
    let pal_base = ((entry >> 12u) & 0x0Fu) * 16u;
    return paleta[py * 512u + pal_base + nib];
}

// ===========================================================================
// Sprites
// ===========================================================================

/// Índice de paleta del píxel del sprite, o 0 si es transparente.
///
/// Reproduce `render_sprites` de la referencia: voltees, mapeo 1D/2D, 4/8 bits y
/// la transformación afín (con el centro de la caja como origen).
fn indice_paleta_obj(s: SpriteNativa, px: i32, py: i32) -> u32 {
    let bx = px - i32(s.pos_x);
    let by = py - i32(s.pos_y);
    let w = i32(s.tex_x);
    let h = i32(s.tex_y);

    var u: i32 = bx;
    var v: i32 = by;

    if ((s.flags & FLAG_AFIN) != 0u) {
        let bw = i32(s.caja_x);
        let bh = i32(s.caja_y);
        let dx = bx - bw / 2;
        let dy = by - bh / 2;
        u = ((s.afin_a * dx + s.afin_b * dy) >> 8) + w / 2;
        v = ((s.afin_c * dx + s.afin_d * dy) >> 8) + h / 2;
        if (u < 0 || v < 0 || u >= w || v >= h) {
            return 0u;
        }
    } else {
        if (u < 0 || v < 0 || u >= w || v >= h) {
            return 0u;
        }
        if ((s.flags & FLAG_HFLIP) != 0u) {
            u = w - 1 - u;
        }
        if ((s.flags & FLAG_VFLIP) != 0u) {
            v = h - 1 - v;
        }
    }

    let tu = u / 8;
    let pu = u - tu * 8;
    let tv = v / 8;
    let pv = v - tv * 8;

    let ocho = (s.flags & FLAG_8BPP) != 0u;
    var row_tiles: i32 = 32;
    if ((s.flags & FLAG_1D) != 0u) {
        row_tiles = (w / 8) * select(1, 2, ocho);
    }
    let tile_step = select(1, 2, ocho);
    let tile = i32(s.base_tile) + tv * row_tiles + tu * tile_step;

    // En los modos bitmap los primeros 512 tiles se solapan con el bitmap.
    let py_clamped = u32(clamp(py, 0, 159));
    if (filas_capa[py_clamped].modo >= 3u && tile < 512) {
        return 0u;
    }

    var addr: u32;
    if (ocho) {
        addr = 0x10000u + u32((tile / 2) * 64 + pv * 8 + pu);
    } else {
        addr = 0x10000u + u32(tile * 32 + pv * 4 + pu / 2);
    }
    if (addr >= 0x18000u) {
        return 0u;
    }

    var color_idx: u32;
    if (ocho) {
        color_idx = vram_byte(addr);
    } else {
        let byte = vram_byte(addr);
        var nib = byte & 0x0Fu;
        if ((pu & 1) != 0) {
            nib = (byte >> 4u) & 0x0Fu;
        }
        color_idx = nib;
    }
    if (color_idx == 0u) {
        return 0u;
    }
    if (ocho) {
        return 256u + color_idx;
    }
    return 256u + s.palette_idx * 16u + color_idx;
}

// ===========================================================================
// Vértices
// ===========================================================================

struct SalidaQuad {
    @builtin(position) clip: vec4<f32>,
};

struct SalidaObj {
    @builtin(position) clip: vec4<f32>,
    @location(0) @interpolate(flat) indice: u32,
};

@vertex
fn vs_quad(@location(0) quad: vec2<f32>) -> SalidaQuad {
    var out: SalidaQuad;
    out.clip = vec4<f32>(quad.x * 2.0 - 1.0, 1.0 - quad.y * 2.0, 0.0, 1.0);
    return out;
}

// La geometría del sprite llega como atributos de instancia y no leyendo
// `sprites`: un *storage buffer* visible en la etapa de vértices exige la
// capacidad `DownlevelFlags::VERTEX_STORAGE`, que el estándar de OpenGL ES
// permite no tener (un dispositivo puede admitir SSBO solo en fragmento).
@vertex
fn vs_obj(
    @location(0) quad: vec2<f32>,
    @location(1) inst_geometria: vec4<f32>,
    @location(2) inst_clave: vec2<u32>,
    @builtin(instance_index) inst: u32,
) -> SalidaObj {
    let px = inst_geometria.x + quad.x * inst_geometria.z;
    let py = inst_geometria.y + quad.y * inst_geometria.w;
    // Clave de prioridad para la prueba de profundidad: menor número de
    // prioridad gana y, a igualdad, el menor índice de OAM.
    let clave = f32(inst_clave.x * 128u + inst_clave.y) / 512.0;
    var out: SalidaObj;
    out.clip = vec4<f32>((px / 240.0) * 2.0 - 1.0, 1.0 - (py / 160.0) * 2.0, clave, 1.0);
    out.indice = inst;
    return out;
}


// ===========================================================================
// Pasada de sprites
// ===========================================================================

@fragment
fn fs_obj(in: SalidaObj) -> @location(0) vec4<u32> {
    let s = sprites[in.indice];
    let px = i32(floor(in.clip.x));
    let py = i32(floor(in.clip.y));
    let pal = indice_paleta_obj(s, px, py);
    if (pal == 0u) {
        discard;
    }
    let semi = select(0u, 1u, (s.flags & FLAG_SEMI) != 0u);
    let upy = u32(clamp(py, 0, 159));
    return vec4<u32>(paleta[upy * 512u + pal], s.prioridad, semi, 0xFFFFu);
}

@fragment
fn fs_obj_ventana(in: SalidaObj) -> @location(0) u32 {
    let s = sprites[in.indice];
    let px = i32(floor(in.clip.x));
    let py = i32(floor(in.clip.y));
    let pal = indice_paleta_obj(s, px, py);
    if (pal == 0u) {
        discard;
    }
    return 1u;
}

// ===========================================================================
// Pasada de composición
// ===========================================================================

/// Rango de ventana: `x1` incluido, `x2` excluido; si `x1 > x2` da la vuelta.
fn en_rango(x1: u32, x2: u32, v: u32) -> bool {
    if (x1 <= x2) {
        return v >= x1 && v < x2;
    }
    return v >= x1 || v < x2;
}

fn necesita_segunda(primera_id: u32, obj_visible: bool, obj_semi: bool, efectos: bool, modo: u32, bldcnt: u32) -> bool {
    if (obj_visible && primera_id == 4u && obj_semi) {
        return true;
    }
    if (efectos && modo == 1u && (bldcnt & (1u << primera_id)) != 0u) {
        return true;
    }
    return false;
}

@fragment
fn fs_comp(in: SalidaQuad) -> @location(0) vec4<f32> {
    let px = i32(floor(in.clip.x));
    let py = i32(floor(in.clip.y));
    let coord = vec2<i32>(px, py);
    let upy = u32(clamp(py, 0, 159));
    let upx = u32(clamp(px, 0, 239));

    let cfg_comp = filas_comp[upy];

    // Pantalla en blanco (DISPCNT bit 7).
    if ((cfg_comp.dispcnt & 0x80u) != 0u) {
        return vec4<f32>(1.0, 1.0, 1.0, 1.0);
    }

    let obj = textureLoad(tex_obj, coord, 0);
    let obj_presente = obj.a != 0u;
    let obj_color = obj.r;
    let obj_prio = obj.g;
    let obj_semi = obj.b != 0u;

    // --- Máscara de control del píxel ---
    let win0_on = (cfg_comp.dispcnt & 0x2000u) != 0u;
    let win1_on = (cfg_comp.dispcnt & 0x4000u) != 0u;
    let objwin_on = (cfg_comp.dispcnt & 0x8000u) != 0u;

    var control: u32 = 0x3Fu;
    if (win0_on || win1_on || objwin_on) {
        let ux = u32(px);
        let uy = u32(py);
        let en_win0 = win0_on
            && en_rango((cfg_comp.win0v >> 8u) & 0xFFu, cfg_comp.win0v & 0xFFu, uy)
            && en_rango((cfg_comp.win0h >> 8u) & 0xFFu, cfg_comp.win0h & 0xFFu, ux);
        let en_win1 = win1_on
            && en_rango((cfg_comp.win1v >> 8u) & 0xFFu, cfg_comp.win1v & 0xFFu, uy)
            && en_rango((cfg_comp.win1h >> 8u) & 0xFFu, cfg_comp.win1h & 0xFFu, ux);
        if (en_win0) {
            control = cfg_comp.winin & 0x3Fu;
        } else if (en_win1) {
            control = (cfg_comp.winin >> 8u) & 0x3Fu;
        } else if (objwin_on) {
            let obj_window = textureLoad(tex_obj_ventana, coord, 0).r != 0u;
            if (obj_window) {
                control = (cfg_comp.winout >> 8u) & 0x3Fu;
            } else {
                control = cfg_comp.winout & 0x3Fu;
            }
        } else {
            control = cfg_comp.winout & 0x3Fu;
        }
    }

    let modo = (cfg_comp.bldcnt >> 6u) & 3u;
    let efectos = (control & 0x20u) != 0u;

    // --- Las dos capas más altas ---
    let obj_visible = obj_presente && (control & 0x10u) != 0u;
    var primera: u32 = 0u;
    var primera_id: u32 = 5u;
    var segunda: u32 = 0u;
    var segunda_id: u32 = 5u;
    var hay_primera = false;
    var hay_segunda = false;
    var corte = false;

    for (var p: u32 = 0u; p < 4u; p = p + 1u) {
        if (corte) {
            break;
        }
        if (obj_visible && obj_prio == p) {
            if (!hay_primera) {
                primera = obj_color;
                primera_id = 4u;
                hay_primera = true;
                if (!necesita_segunda(primera_id, obj_visible, obj_semi, efectos, modo, cfg_comp.bldcnt)) {
                    corte = true;
                    break;
                }
            } else {
                segunda = obj_color;
                segunda_id = 4u;
                hay_segunda = true;
                corte = true;
                break;
            }
        }

        // BG0
        if (cfg_comp.bg_prio0 == p && (control & 1u) != 0u) {
            let c0 = color_capa(upy, 0u, upx, upy);
            if (c0 != 0xFFFFu) {
                if (!hay_primera) {
                    primera = c0;
                    primera_id = 0u;
                    hay_primera = true;
                    if (!necesita_segunda(primera_id, obj_visible, obj_semi, efectos, modo, cfg_comp.bldcnt)) {
                        corte = true;
                        break;
                    }
                } else {
                    segunda = c0;
                    segunda_id = 0u;
                    hay_segunda = true;
                    corte = true;
                    break;
                }
            }
        }

        // BG1
        if (cfg_comp.bg_prio1 == p && (control & 2u) != 0u) {
            let c1 = color_capa(upy, 1u, upx, upy);
            if (c1 != 0xFFFFu) {
                if (!hay_primera) {
                    primera = c1;
                    primera_id = 1u;
                    hay_primera = true;
                    if (!necesita_segunda(primera_id, obj_visible, obj_semi, efectos, modo, cfg_comp.bldcnt)) {
                        corte = true;
                        break;
                    }
                } else {
                    segunda = c1;
                    segunda_id = 1u;
                    hay_segunda = true;
                    corte = true;
                    break;
                }
            }
        }

        // BG2
        if (cfg_comp.bg_prio2 == p && (control & 4u) != 0u) {
            let c2 = color_capa(upy, 2u, upx, upy);
            if (c2 != 0xFFFFu) {
                if (!hay_primera) {
                    primera = c2;
                    primera_id = 2u;
                    hay_primera = true;
                    if (!necesita_segunda(primera_id, obj_visible, obj_semi, efectos, modo, cfg_comp.bldcnt)) {
                        corte = true;
                        break;
                    }
                } else {
                    segunda = c2;
                    segunda_id = 2u;
                    hay_segunda = true;
                    corte = true;
                    break;
                }
            }
        }

        // BG3
        if (cfg_comp.bg_prio3 == p && (control & 8u) != 0u) {
            let c3 = color_capa(upy, 3u, upx, upy);
            if (c3 != 0xFFFFu) {
                if (!hay_primera) {
                    primera = c3;
                    primera_id = 3u;
                    hay_primera = true;
                    if (!necesita_segunda(primera_id, obj_visible, obj_semi, efectos, modo, cfg_comp.bldcnt)) {
                        corte = true;
                        break;
                    }
                } else {
                    segunda = c3;
                    segunda_id = 3u;
                    hay_segunda = true;
                    corte = true;
                    break;
                }
            }
        }
    }

    let backdrop = paleta[upy * 512u];
    if (!hay_primera) {
        primera = backdrop;
        primera_id = 5u;
    }
    if (!hay_segunda) {
        segunda = backdrop;
        segunda_id = 5u;
    }

    // --- Unidad de mezcla ---
    let eva = min(16u, cfg_comp.bldalpha & 0x1Fu);
    let evb = min(16u, (cfg_comp.bldalpha >> 8u) & 0x1Fu);
    let evy = min(16u, cfg_comp.bldy & 0x1Fu);
    let semi = obj_visible && primera_id == 4u && obj_semi;

    var color = primera;
    if (semi && (cfg_comp.bldcnt & (0x100u << segunda_id)) != 0u) {
        // Un sprite semitransparente fuerza el alfa contra el 2.º objetivo.
        color = alpha_blend(primera, segunda, eva, evb);
    } else if (efectos && modo != 0u && (cfg_comp.bldcnt & (1u << primera_id)) != 0u) {
        if (modo == 1u) {
            if ((cfg_comp.bldcnt & (0x100u << segunda_id)) != 0u) {
                color = alpha_blend(primera, segunda, eva, evb);
            }
        } else if (modo == 2u) {
            color = brighten(primera, evy);
        } else if (modo == 3u) {
            color = darken(primera, evy);
        }
    }

    // Conversión a 8 bits: la misma que usa el arnés de comparación.
    let r8 = ((color & 0x1Fu) * 255u + 15u) / 31u;
    let g8 = (((color >> 5u) & 0x1Fu) * 255u + 15u) / 31u;
    let b8 = (((color >> 10u) & 0x1Fu) * 255u + 15u) / 31u;
    return vec4<f32>(f32(r8) / 255.0, f32(g8) / 255.0, f32(b8) / 255.0, 1.0);
}
