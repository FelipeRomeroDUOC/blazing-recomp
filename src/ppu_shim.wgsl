// Shaders WGSL para el PPU Shim: Renderizado acelerado de fondos y sprites de Game Boy Advance.
//
// Renderiza las 4 capas de fondos de Modo 0 y hasta 128 sprites instanciados directamente
// sobre el búfer de renderizado en resolución nativa GBA (240x160).

// ===========================================================================
// Tipos y Estructuras Comunes
// ===========================================================================

struct VertexInput {
    @location(0) quad_pos: vec2<f32>,
};

struct SpriteInstanceInput {
    @location(1) pos: vec2<f32>,
    @location(2) size: vec2<f32>,
    @location(3) base_tile: u32,
    @location(4) palette_idx: u32,
    @location(5) depth: f32,
    @location(6) flags: u32,
};

struct SpriteVertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) local_pos: vec2<f32>,
    @location(1) @interpolate(flat) size: vec2<f32>,
    @location(2) @interpolate(flat) base_tile: u32,
    @location(3) @interpolate(flat) palette_idx: u32,
    @location(4) @interpolate(flat) flags: u32,
};

// ===========================================================================
// Recursos Compartidos: Atlas de Sprites, Paleta de Color y VRAM de Fondos
// ===========================================================================

@group(0) @binding(0)
var tile_atlas: texture_2d<u32>;

@group(0) @binding(1)
var<storage, read> palette_data: array<vec4<f32>>;

@group(0) @binding(2)
var<storage, read> vram_bg: array<u32>;

struct BgUniforms {
    scroll: vec2<f32>,
    char_base: u32,
    screen_base: u32,
    screen_size: u32,
    is_8bpp: u32,
    depth: f32,
    _pad: u32,
};

@group(0) @binding(3)
var<uniform> bg_uniforms: BgUniforms;

// ===========================================================================
// Shaders para Sprites (OBJ)
// ===========================================================================

@vertex
fn vs_sprite(
    vertex: VertexInput,
    instance: SpriteInstanceInput,
) -> SpriteVertexOutput {
    var out: SpriteVertexOutput;

    out.local_pos = vertex.quad_pos * instance.size;
    out.size = instance.size;
    out.base_tile = instance.base_tile;
    out.palette_idx = instance.palette_idx;
    out.flags = instance.flags;

    // Calcular posición en píxeles GBA: pos + quad_pos * size
    let pixel_pos = instance.pos + vertex.quad_pos * instance.size;

    // Mapear de [0..240, 0..160] a espacio NDC [-1.0..1.0, 1.0..-1.0]
    let ndc_x = (pixel_pos.x / 240.0) * 2.0 - 1.0;
    let ndc_y = 1.0 - (pixel_pos.y / 160.0) * 2.0;

    out.clip_position = vec4<f32>(ndc_x, ndc_y, instance.depth, 1.0);
    return out;
}

@fragment
fn fs_sprite(in: SpriteVertexOutput) -> @location(0) vec4<f32> {
    var lx = in.local_pos.x;
    var ly = in.local_pos.y;

    // Aplicar hflip (bit 0) y vflip (bit 1) a la coordenada local
    if ((in.flags & 1u) != 0u) {
        lx = in.size.x - 1.0 - lx;
    }
    if ((in.flags & 2u) != 0u) {
        ly = in.size.y - 1.0 - ly;
    }

    let clamped_x = clamp(lx, 0.0, in.size.x - 1.0);
    let clamped_y = clamp(ly, 0.0, in.size.y - 1.0);

    let tx = u32(clamped_x / 8.0);
    let ty = u32(clamped_y / 8.0);
    let px = u32(clamped_x) % 8u;
    let py = u32(clamped_y) % 8u;

    let tw = u32(in.size.x / 8.0);

    var raw_pixel = 0u;

    if ((in.flags & 16u) != 0u) {
        // --- Modo 8bpp (256 colores / 1 byte por píxel) ---
        // En 8bpp, la GBA ignora el bit menos significativo del tile base
        let base = in.base_tile & 0x3FEu;
        var target_tile = base;
        if ((in.flags & 32u) != 0u) {
            // 1D mapping: cada tile 8x8 consume 64 bytes (2 bloques de 32 bytes)
            target_tile = base + (ty * tw + tx) * 2u;
        } else {
            // 2D mapping: cada fila consume 32 bloques de 32 bytes (1024 bytes)
            target_tile = base + ty * 32u + tx * 2u;
        }

        // Base de OBJ en VRAM: 0x10000 bytes (65536)
        let byte_addr = 65536u + (target_tile % 1024u) * 32u + py * 8u + px;
        let word_idx = (byte_addr % 98304u) / 4u;
        let shift = (byte_addr % 4u) * 8u;
        raw_pixel = (vram_bg[word_idx] >> shift) & 0xFFu;
    } else {
        // --- Modo 4bpp (16 colores / nibble de 4 bits) ---
        var target_tile = in.base_tile;
        if ((in.flags & 32u) != 0u) {
            // 1D mapping
            target_tile = in.base_tile + (ty * tw + tx);
        } else {
            // 2D mapping
            target_tile = in.base_tile + (ty * 32u + tx);
        }

        // Cada tile ocupa 32 bytes (4 bytes por fila de 8 píxeles)
        let byte_addr = 65536u + (target_tile % 1024u) * 32u + py * 4u + (px / 2u);
        let word_idx = (byte_addr % 98304u) / 4u;
        let shift = (byte_addr % 4u) * 8u;
        let byte_val = (vram_bg[word_idx] >> shift) & 0xFFu;
        if ((px % 2u) == 1u) {
            raw_pixel = (byte_val >> 4u) & 0x0Fu;
        } else {
            raw_pixel = byte_val & 0x0Fu;
        }
    }

    // Color 0 es transparente en sprites GBA
    if (raw_pixel == 0u) {
        discard;
    }

    // Los sprites de GBA indexan la paleta superior (offset 256 de 512 colores)
    var final_color_idx: u32 = 0u;
    if ((in.flags & 16u) != 0u) {
        // 8bpp: índice directo en paleta OBJ (256..511)
        final_color_idx = 256u + raw_pixel;
    } else {
        // 4bpp: subpaleta (16 colores) en paleta OBJ (256..511)
        final_color_idx = 256u + in.palette_idx * 16u + raw_pixel;
    }

    var color = palette_data[final_color_idx];

    // Modo semitransparente (bit 2 en flags)
    if ((in.flags & 4u) != 0u) {
        color.a = 0.5;
    }

    return color;
}

// ===========================================================================
// Shaders para Fondos Modo 0 (BG0..BG3)
// ===========================================================================

struct BgVertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) screen_pos: vec2<f32>,
};

@vertex
fn vs_bg(vertex: VertexInput) -> BgVertexOutput {
    var out: BgVertexOutput;

    // Coordenadas de pantalla GBA (0..240, 0..160)
    out.screen_pos = vertex.quad_pos * vec2<f32>(240.0, 160.0);

    // Mapeo a NDC [-1.0..1.0, 1.0..-1.0]
    let ndc_x = vertex.quad_pos.x * 2.0 - 1.0;
    let ndc_y = 1.0 - vertex.quad_pos.y * 2.0;

    out.clip_position = vec4<f32>(ndc_x, ndc_y, bg_uniforms.depth, 1.0);
    return out;
}

@fragment
fn fs_bg(in: BgVertexOutput) -> @location(0) vec4<f32> {
    if (in.screen_pos.x < 0.0 || in.screen_pos.x >= 240.0 || in.screen_pos.y < 0.0 || in.screen_pos.y >= 160.0) {
        discard;
    }

    // Coordenada virtual en el tilemap desplazada por scroll
    var px = in.screen_pos.x + bg_uniforms.scroll.x;
    var py = in.screen_pos.y + bg_uniforms.scroll.y;

    // Dimensiones virtuales del mapa según screen_size (0: 256x256, 1: 512x256, 2: 256x512, 3: 512x512)
    var map_w = 256.0;
    var map_h = 256.0;
    if (bg_uniforms.screen_size == 1u || bg_uniforms.screen_size == 3u) {
        map_w = 512.0;
    }
    if (bg_uniforms.screen_size == 2u || bg_uniforms.screen_size == 3u) {
        map_h = 512.0;
    }

    px = (px % map_w + map_w) % map_w;
    py = (py % map_h + map_h) % map_h;

    let tile_x = u32(px / 8.0);
    let tile_y = u32(py / 8.0);

    // Selección del screen block base (SBB) en mapas multisección
    var sbb_offset = 0u;
    var local_tx = tile_x;
    var local_ty = tile_y;

    if (bg_uniforms.screen_size == 1u) {
        // 64x32: 2 bloques horizontales
        if (local_tx >= 32u) {
            sbb_offset = 1u;
            local_tx = local_tx - 32u;
        }
    } else if (bg_uniforms.screen_size == 2u) {
        // 32x64: 2 bloques verticales
        if (local_ty >= 32u) {
            sbb_offset = 1u;
            local_ty = local_ty - 32u;
        }
    } else if (bg_uniforms.screen_size == 3u) {
        // 64x64: 4 bloques (2x2)
        let bx = local_tx / 32u;
        let by = local_ty / 32u;
        sbb_offset = by * 2u + bx;
        local_tx = local_tx % 32u;
        local_ty = local_ty % 32u;
    }

    // Cada SBB ocupa 2048 bytes (32x32 entradas u16)
    let tile_entry_byte_addr = (bg_uniforms.screen_base + sbb_offset) * 2048u + (local_ty * 32u + local_tx) * 2u;

    // Leer entrada de 16 bits de VRAM
    let word_idx = tile_entry_byte_addr / 4u;
    let shift = (tile_entry_byte_addr % 4u) * 8u;
    let tile_entry = (vram_bg[word_idx] >> shift) & 0xFFFFu;

    let tile_base_idx = tile_entry & 0x03FFu;
    let hflip = (tile_entry & (1u << 10u)) != 0u;
    let vflip = (tile_entry & (1u << 11u)) != 0u;
    let subpalette = (tile_entry >> 12u) & 0x0Fu;

    var fx = u32(px) % 8u;
    var fy = u32(py) % 8u;
    if (hflip) {
        fx = 7u - fx;
    }
    if (vflip) {
        fy = 7u - fy;
    }

    var color_raw = 0u;
    if (bg_uniforms.is_8bpp != 0u) {
        // 8bpp: 64 bytes por tile
        let tile_byte_addr = bg_uniforms.char_base * 16384u + tile_base_idx * 64u + fy * 8u + fx;
        let t_word = tile_byte_addr / 4u;
        let t_shift = (tile_byte_addr % 4u) * 8u;
        color_raw = (vram_bg[t_word] >> t_shift) & 0xFFu;
    } else {
        // 4bpp: 32 bytes por tile (4 bytes por fila de 8 píxeles)
        let tile_byte_addr = bg_uniforms.char_base * 16384u + tile_base_idx * 32u + fy * 4u + (fx / 2u);
        let t_word = tile_byte_addr / 4u;
        let t_shift = (tile_byte_addr % 4u) * 8u;
        let byte_val = (vram_bg[t_word] >> t_shift) & 0xFFu;
        if ((fx % 2u) == 0u) {
            color_raw = byte_val & 0x0Fu;
        } else {
            color_raw = (byte_val >> 4u) & 0x0Fu;
        }
    }

    // Color 0 es transparente en fondos Modo 0
    if (color_raw == 0u) {
        discard;
    }

    // Paleta de fondos (0..255)
    var pal_idx = 0u;
    if (bg_uniforms.is_8bpp != 0u) {
        pal_idx = color_raw;
    } else {
        pal_idx = subpalette * 16u + color_raw;
    }

    return palette_data[pal_idx];
}
