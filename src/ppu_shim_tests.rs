//! Pruebas satélite para el módulo PPU Shim (src/ppu_shim.rs).
//!
//! Cumple con las directrices de AGENTS.md §4.4 y §4.5:
//! - Archivo satélite adyacente para no sobrecargar el módulo de producción.
//! - Exactitud matemática sin debilitar aserciones.

use super::*;

#[test]
fn test_dimensiones_sprites_matriz_completa() {
    // 1. Cuadrados (Square: 8x8, 16x16, 32x32, 64x64)
    assert_eq!(sprite_dimensions(SpriteShape::Square, 0), (8, 8));
    assert_eq!(sprite_dimensions(SpriteShape::Square, 1), (16, 16));
    assert_eq!(sprite_dimensions(SpriteShape::Square, 2), (32, 32));
    assert_eq!(sprite_dimensions(SpriteShape::Square, 3), (64, 64));

    // 2. Anchos (Wide: 16x8, 32x8, 32x16, 64x32)
    assert_eq!(sprite_dimensions(SpriteShape::Wide, 0), (16, 8));
    assert_eq!(sprite_dimensions(SpriteShape::Wide, 1), (32, 8));
    assert_eq!(sprite_dimensions(SpriteShape::Wide, 2), (32, 16));
    assert_eq!(sprite_dimensions(SpriteShape::Wide, 3), (64, 32));

    // 3. Altos (Tall: 8x16, 8x32, 16x32, 32x64)
    assert_eq!(sprite_dimensions(SpriteShape::Tall, 0), (8, 16));
    assert_eq!(sprite_dimensions(SpriteShape::Tall, 1), (8, 32));
    assert_eq!(sprite_dimensions(SpriteShape::Tall, 2), (16, 32));
    assert_eq!(sprite_dimensions(SpriteShape::Tall, 3), (32, 64));

    // 4. Reservado / Fallback
    assert_eq!(sprite_dimensions(SpriteShape::Reserved, 0), (8, 8));
}

#[test]
fn test_extraccion_atributos_forma_y_modo() {
    // Attr0 con Shape Wide (bits 14-15 = 1), Mode SemiTransparent (bits 10-11 = 1)
    let attr0 = (1 << 14) | (1 << 10) | 42;
    assert_eq!(SpriteShape::from_attr0(attr0), SpriteShape::Wide);
    assert_eq!(SpriteMode::from_attr0(attr0), SpriteMode::SemiTransparent);

    // Attr0 con Shape Tall (bits 14-15 = 2), Mode ObjWindow (bits 10-11 = 2)
    let attr0_tall = (2 << 14) | (2 << 10);
    assert_eq!(SpriteShape::from_attr0(attr0_tall), SpriteShape::Tall);
    assert_eq!(SpriteMode::from_attr0(attr0_tall), SpriteMode::ObjWindow);
}

#[test]
fn test_decodificacion_oam_sprite_valido() {
    let mut shim = PpuShim::new();
    let mut oam = vec![0u8; 1024];

    // Sprite 0:
    // Attr0: Y=80, Square (0), Normal (0), 4bpp (0) -> 80
    let attr0: u16 = 80;
    // Attr1: X=120, Size 1 (16x16, bits 14-15 = 1) -> (1 << 14) | 120
    let attr1: u16 = (1 << 14) | 120;
    // Attr2: Tile=42, Priority=1 (bits 10-11 = 1), Palette=3 (bits 12-15 = 3)
    let attr2: u16 = (3 << 12) | (1 << 10) | 42;

    oam[0..2].copy_from_slice(&attr0.to_le_bytes());
    oam[2..4].copy_from_slice(&attr1.to_le_bytes());
    oam[4..6].copy_from_slice(&attr2.to_le_bytes());

    // El resto de la OAM tiene sprites con bit 9 activo (deshabilitados)
    for i in 1..MAX_SPRITES {
        let off = i * 8;
        let disabled_attr0: u16 = 1 << 9;
        oam[off..off + 2].copy_from_slice(&disabled_attr0.to_le_bytes());
    }

    let count = shim.decode_oam(&oam);
    assert_eq!(count, 1);
    assert_eq!(shim.sprite_count, 1);

    let sp = shim.sprites[0];
    assert_eq!(sp.pos, [120.0, 80.0]);
    assert_eq!(sp.size, [16.0, 16.0]);
    assert_eq!(sp.base_tile, 42);
    assert_eq!(sp.palette_idx, 3);
    assert_eq!(sp.flags & 0x3F, 0); // Ni hflip, ni vflip, ni 8bpp, ni 1d
    assert_eq!((sp.flags >> 8) & 3, 1); // Prioridad 1
    assert_eq!((sp.flags >> 16) & 0x7F, 0); // Índice OAM 0
    assert!((sp.depth - 0.35).abs() < 1e-5); // depth = 0.15 + 1 * 0.2 = 0.35
}

#[test]
fn test_decodificacion_oam_flips_y_8bpp() {
    let mut shim = PpuShim::new();
    let mut oam = vec![0u8; 1024];

    // Sprite 0 con 8bpp (bit 13 en attr0), HFlip (bit 12 en attr1), VFlip (bit 13 en attr1)
    let attr0: u16 = (1 << 13) | 50; // Y = 50
    let attr1: u16 = (1 << 13) | (1 << 12) | 60; // X = 60
    let attr2: u16 = (5 << 12) | 10; // Tile 10, Palette 5 (en 8bpp la paleta debe ser 0)

    oam[0..2].copy_from_slice(&attr0.to_le_bytes());
    oam[2..4].copy_from_slice(&attr1.to_le_bytes());
    oam[4..6].copy_from_slice(&attr2.to_le_bytes());

    for i in 1..MAX_SPRITES {
        let off = i * 8;
        oam[off..off + 2].copy_from_slice(&(1u16 << 9).to_le_bytes());
    }

    shim.decode_oam(&oam);
    let sp = shim.sprites[0];

    // Banderas esperadas: hflip (bit 0), vflip (bit 1), 8bpp (bit 4) -> 1 | 2 | 16 = 19
    assert_eq!(sp.flags & 0x3F, 1 | 2 | 16);
    assert_eq!(sp.palette_idx, 0); // En 8bpp se fuerza a 0
    assert_eq!(sp.base_tile, 10);
}

#[test]
fn test_oam_1d_mapping_flag_en_dispcnt() {
    let mut shim = PpuShim::new();
    let mut oam = vec![0u8; 1024];

    // Activar bit 6 en dispcnt (1D mapping)
    shim.dispcnt = 1 << 6;

    let attr0: u16 = 10;
    let attr1: u16 = 20;
    let attr2: u16 = 5;
    oam[0..2].copy_from_slice(&attr0.to_le_bytes());
    oam[2..4].copy_from_slice(&attr1.to_le_bytes());
    oam[4..6].copy_from_slice(&attr2.to_le_bytes());

    for i in 1..MAX_SPRITES {
        let off = i * 8;
        oam[off..off + 2].copy_from_slice(&(1u16 << 9).to_le_bytes());
    }

    shim.decode_oam(&oam);
    let sp = shim.sprites[0];

    // Bit 5 debe estar activo (1D mapping)
    assert_ne!(sp.flags & (1 << 5), 0, "debe activar bandera de 1D mapping");
}

#[test]
fn test_unpack_obj_tiles_desde_offset_0x10000() {
    let mut vram = vec![0u8; 0x18000]; // 96 KB
    // Colocar un tile distintivo en Charblock 0 (offset 0, fondos)
    vram[0] = 0x11;
    // Colocar un tile distintivo en Charblock 4 (offset 0x10000 = 65536, sprites)
    vram[0x10000] = 0x42; // p0 = 2, p1 = 4

    let mut atlas = Box::new([0u8; 256 * 256]);
    unpack_obj_tiles_4bpp(&vram, &mut atlas);

    // El atlas de sprites debe contener el tile de 0x10000, NO el de 0x0000
    assert_eq!(atlas[0], 2);
    assert_eq!(atlas[1], 4);
}

#[test]
fn test_decodificacion_oam_wrap_around_coordenadas_negativas() {
    let mut shim = PpuShim::new();
    let mut oam = vec![0u8; 1024];

    // Sprite asomándose por la esquina superior izquierda:
    // raw_y = 248 -> 248 - 256 = -8
    // raw_x = 504 -> 504 - 512 = -8
    // Tamaño 16x16 (sobresale 8 píxeles en pantalla)
    let attr0: u16 = 248;
    let attr1: u16 = (1 << 14) | 504;
    let attr2: u16 = 0;

    oam[0..2].copy_from_slice(&attr0.to_le_bytes());
    oam[2..4].copy_from_slice(&attr1.to_le_bytes());
    oam[4..6].copy_from_slice(&attr2.to_le_bytes());

    for i in 1..MAX_SPRITES {
        let off = i * 8;
        oam[off..off + 2].copy_from_slice(&(1u16 << 9).to_le_bytes());
    }

    let count = shim.decode_oam(&oam);
    assert_eq!(count, 1);
    assert_eq!(shim.sprites[0].pos, [-8.0, -8.0]);
}

#[test]
fn test_poda_temprana_sprites_fuera_de_pantalla() {
    let mut shim = PpuShim::new();
    let mut oam = vec![0u8; 1024];

    // Sprite a X = 245 (completamente a la derecha de la pantalla 240x160)
    let attr0: u16 = 50;
    let attr1: u16 = 245;
    oam[0..2].copy_from_slice(&attr0.to_le_bytes());
    oam[2..4].copy_from_slice(&attr1.to_le_bytes());

    for i in 1..MAX_SPRITES {
        let off = i * 8;
        oam[off..off + 2].copy_from_slice(&(1u16 << 9).to_le_bytes());
    }

    let count = shim.decode_oam(&oam);
    assert_eq!(count, 0);
    assert_eq!(shim.sprite_count, 0);
}

#[test]
fn test_decodificacion_registros_io_fondos_y_dispcnt() {
    let mut shim = PpuShim::new();
    let mut io = vec![0u8; 1024];

    // DISPCNT: Habilitar BG0 (bit 8), BG2 (bit 10) y OBJ (bit 12) -> (1 << 12) | (1 << 10) | (1 << 8)
    let dispcnt: u16 = (1 << 12) | (1 << 10) | (1 << 8);
    io[0x00..0x02].copy_from_slice(&dispcnt.to_le_bytes());

    // BG0CNT (0x08): Prioridad 0, CharBase 1 (bits 2-3 = 1), ScreenBase 28 (bits 8-12 = 28), Size 0 (32x32)
    let bg0cnt: u16 = (28 << 8) | (1 << 2) | 0;
    io[0x08..0x0A].copy_from_slice(&bg0cnt.to_le_bytes());

    // BG0HOFS (0x10): 45 píxeles de scroll X
    io[0x10..0x12].copy_from_slice(&45u16.to_le_bytes());
    // BG0VOFS (0x12): 120 píxeles de scroll Y
    io[0x12..0x14].copy_from_slice(&120u16.to_le_bytes());

    // BG1 no habilitado en DISPCNT
    let bg1cnt: u16 = 2; // Prioridad 2
    io[0x0A..0x0C].copy_from_slice(&bg1cnt.to_le_bytes());

    shim.decode_io_regs(&io);

    assert_eq!(shim.dispcnt, dispcnt);

    // BG0:
    let bg0 = shim.bg_layers[0];
    assert!(bg0.enabled);
    assert_eq!(bg0.priority, 0);
    assert_eq!(bg0.char_base_block, 1);
    assert_eq!(bg0.screen_base_block, 28);
    assert_eq!(bg0.tile_dimensions(), (32, 32));
    assert_eq!(bg0.scroll_x, 45);
    assert_eq!(bg0.scroll_y, 120);

    // BG1:
    let bg1 = shim.bg_layers[1];
    assert!(!bg1.enabled);
    assert_eq!(bg1.priority, 2);

    // BG2:
    assert!(shim.bg_layers[2].enabled);
    // BG3:
    assert!(!shim.bg_layers[3].enabled);
}

#[test]
fn test_desactivacion_maestra_obj_en_dispcnt() {
    let mut shim = PpuShim::new();
    let mut io = vec![0u8; 1024];

    // DISPCNT sin el bit 12 (OBJ desactivado)
    let dispcnt: u16 = 1 << 8; // Solo BG0
    io[0x00..0x02].copy_from_slice(&dispcnt.to_le_bytes());

    // Simulamos que había 5 sprites previamente activos
    shim.sprite_count = 5;

    shim.decode_io_regs(&io);
    assert_eq!(shim.sprite_count, 0);
}

#[test]
fn test_gate_maestro_obj_se_aplica_al_dibujar_no_al_decodificar() {
    let mut shim = PpuShim::new();
    let mut oam = vec![0u8; 1024];

    // Sprite 0: Y=80, Square, Normal, 4bpp; X=120, 16x16; Tile=42.
    oam[0..2].copy_from_slice(&80u16.to_le_bytes());
    oam[2..4].copy_from_slice(&((1u16 << 14) | 120).to_le_bytes());
    oam[4..6].copy_from_slice(&42u16.to_le_bytes());
    for i in 1..MAX_SPRITES {
        let off = i * 8;
        oam[off..off + 2].copy_from_slice(&(1u16 << 9).to_le_bytes());
    }

    // Con OBJ apagado la OAM se decodifica igual, pero no se dibuja nada:
    // el hardware no apaga la decodificación, apaga el dibujo.
    shim.dispcnt = 1 << 8; // Solo BG0
    let decoded = shim.decode_oam(&oam);
    assert_eq!(decoded, 1, "la decodificacion de OAM no depende de DISPCNT");
    assert_eq!(shim.active_sprite_count(decoded), 0);

    // Con OBJ habilitado el mismo sprite se dibuja.
    shim.dispcnt = (1 << 12) | (1 << 8);
    let decoded = shim.decode_oam(&oam);
    assert_eq!(decoded, 1);
    assert_eq!(shim.active_sprite_count(decoded), 1);
}

#[test]
fn test_ppu_shim_shader_validates() {
    let mut validator = naga::valid::Validator::new(
        naga::valid::ValidationFlags::all(),
        naga::valid::Capabilities::all(),
    );
    let module = naga::front::wgsl::parse_str(PPU_SHIM_WGSL)
        .expect("el shader de ppu_shim debe parsear correctamente");
    validator
        .validate(&module)
        .expect("el shader de ppu_shim debe validar con naga");
}

#[test]
fn test_unpack_palette_colores_exactos() {
    let mut raw = vec![0u8; 1024];

    // Color 0: Negro (0x0000)
    raw[0..2].copy_from_slice(&0x0000u16.to_le_bytes());
    // Color 1: Blanco (0x7FFF)
    raw[2..4].copy_from_slice(&0x7FFFu16.to_le_bytes());
    // Color 2: Rojo puro (0x001F)
    raw[4..6].copy_from_slice(&0x001Fu16.to_le_bytes());
    // Color 3: Verde puro (0x03E0)
    raw[6..8].copy_from_slice(&0x03E0u16.to_le_bytes());
    // Color 4: Azul puro (0x7C00)
    raw[8..10].copy_from_slice(&0x7C00u16.to_le_bytes());

    let mut out = [[0.0f32; 4]; 512];
    unpack_palette(&raw, &mut out);

    // Negro
    assert_eq!(out[0], [0.0, 0.0, 0.0, 1.0]);
    // Blanco
    assert_eq!(out[1], [1.0, 1.0, 1.0, 1.0]);
    // Rojo
    assert_eq!(out[2], [1.0, 0.0, 0.0, 1.0]);
    // Verde
    assert_eq!(out[3], [0.0, 1.0, 0.0, 1.0]);
    // Azul
    assert_eq!(out[4], [0.0, 0.0, 1.0, 1.0]);
}

#[test]
fn test_unpack_tiles_4bpp_desempaquetado_nibbles() {
    let mut vram = vec![0u8; 64]; // 2 tiles de 32 bytes
    // Tile 0, fila 0: primer byte = 0x21 (p0 = 1, p1 = 2)
    vram[0] = 0x21;
    // Tile 1, fila 0: primer byte = 0x54 (p0 = 4, p1 = 5)
    vram[32] = 0x54;

    let mut atlas = Box::new([0u8; 256 * 256]);
    unpack_tiles_4bpp(&vram, &mut atlas);

    // Tile 0 en (x=0, y=0):
    assert_eq!(atlas[0], 1);
    assert_eq!(atlas[1], 2);

    // Tile 1 en (x=8, y=0):
    assert_eq!(atlas[8], 4);
    assert_eq!(atlas[9], 5);
}

#[test]
fn test_bg_uniforms_gpu_memoria() {
    assert_eq!(std::mem::size_of::<BgUniformsGpu>(), 32);
    assert_eq!(std::mem::align_of::<BgUniformsGpu>(), 4);
}

#[test]
fn test_sprite_gpu_instance_memoria() {
    assert_eq!(std::mem::size_of::<SpriteGpuInstance>(), 32);
    assert_eq!(std::mem::align_of::<SpriteGpuInstance>(), 4);
}

#[test]
fn test_descarte_estricto_obj_window() {
    let mut shim = PpuShim::new();
    let mut oam = vec![0u8; 1024];

    // Sprite 0: Modo ObjWindow (bits 10-11 = 2 de attr0)
    let attr0_objwin: u16 = (2 << 10) | 40; // Y = 40
    let attr1: u16 = 50; // X = 50
    let attr2: u16 = 0;

    oam[0..2].copy_from_slice(&attr0_objwin.to_le_bytes());
    oam[2..4].copy_from_slice(&attr1.to_le_bytes());
    oam[4..6].copy_from_slice(&attr2.to_le_bytes());

    // Sprite 1: Modo Prohibited (bits 10-11 = 3 de attr0)
    let attr0_proh: u16 = (3 << 10) | 60;
    oam[8..10].copy_from_slice(&attr0_proh.to_le_bytes());
    oam[10..12].copy_from_slice(&attr1.to_le_bytes());
    oam[12..14].copy_from_slice(&attr2.to_le_bytes());

    // Sprite 2: Modo Normal (bits 10-11 = 0), debe ser el único aceptado
    let attr0_normal: u16 = 80;
    oam[16..18].copy_from_slice(&attr0_normal.to_le_bytes());
    oam[18..20].copy_from_slice(&attr1.to_le_bytes());
    oam[20..22].copy_from_slice(&attr2.to_le_bytes());

    for i in 3..MAX_SPRITES {
        let off = i * 8;
        oam[off..off + 2].copy_from_slice(&(1u16 << 9).to_le_bytes());
    }

    let count = shim.decode_oam(&oam);
    assert_eq!(count, 1, "solo el sprite normal debe ser decodificado");
    assert_eq!(shim.sprites[0].pos[1], 80.0);
}

#[test]
fn test_orden_oam_descendente_painters_algorithm() {
    let mut shim = PpuShim::new();
    let mut oam = vec![0u8; 1024];

    // Sprite 0: Prioridad 1 (bits 10-11 = 1) -> frente respecto a Sprite 2 por menor índice OAM
    let attr0: u16 = 20;
    let attr1: u16 = 30;
    let attr2_prio1: u16 = 1 << 10;
    oam[0..2].copy_from_slice(&attr0.to_le_bytes());
    oam[2..4].copy_from_slice(&attr1.to_le_bytes());
    oam[4..6].copy_from_slice(&attr2_prio1.to_le_bytes());

    // Sprite 1: Prioridad 3 (bits 10-11 = 3) -> fondo más lejano
    let attr2_prio3: u16 = 3 << 10;
    oam[8..10].copy_from_slice(&attr0.to_le_bytes());
    oam[10..12].copy_from_slice(&attr1.to_le_bytes());
    oam[12..14].copy_from_slice(&attr2_prio3.to_le_bytes());

    // Sprite 2: Prioridad 1 (bits 10-11 = 1) -> fondo respecto a Sprite 0 por mayor índice OAM
    oam[16..18].copy_from_slice(&attr0.to_le_bytes());
    oam[18..20].copy_from_slice(&attr1.to_le_bytes());
    oam[20..22].copy_from_slice(&attr2_prio1.to_le_bytes());

    for i in 3..MAX_SPRITES {
        let off = i * 8;
        oam[off..off + 2].copy_from_slice(&(1u16 << 9).to_le_bytes());
    }

    let count = shim.decode_oam(&oam);
    assert_eq!(count, 3);

    // Orden Painter's esperado:
    // Pos 0: Sprite 1 (prioridad 3, el más profundo)
    // Pos 1: Sprite 2 (prioridad 1, índice OAM 2, se dibuja antes)
    // Pos 2: Sprite 0 (prioridad 1, índice OAM 0, se dibuja al final para quedar al frente)
    let sp0 = shim.sprites[0];
    let sp1 = shim.sprites[1];
    let sp2 = shim.sprites[2];

    assert_eq!((sp0.flags >> 8) & 3, 3, "el sprite de mayor prioridad numérica (3) se dibuja primero");
    assert_eq!((sp0.flags >> 16) & 0x7F, 1);

    assert_eq!((sp1.flags >> 8) & 3, 1);
    assert_eq!((sp1.flags >> 16) & 0x7F, 2, "entre misma prioridad, el de mayor índice OAM (2) se dibuja primero");

    assert_eq!((sp2.flags >> 8) & 3, 1);
    assert_eq!((sp2.flags >> 16) & 0x7F, 0, "el sprite con menor índice OAM (0) se dibuja al final para estar al frente");
}

#[test]
fn test_decodificacion_sprite_8bpp_indices_completos() {
    let mut vram = vec![0u8; 98304];

    // Sprite 8bpp en 1D: base_tile = 4 (tile_offset = 4).
    // Tile 0 del sprite (tx=0, ty=0): dirección = 0x10000 + 4 * 32 = 0x10080.
    // Tile 1 del sprite (tx=1, ty=0): dirección = 0x10000 + (4 + 2) * 32 = 0x100C0.
    let tile0_addr = 0x10000 + 4 * 32;
    let tile1_addr = 0x10000 + 6 * 32;

    // Escribir bytes con índices > 15 (que en 4bpp se truncarían/perderían)
    vram[tile0_addr] = 0x42; // px=0, py=0 -> índice 66
    vram[tile0_addr + 1] = 0xFF; // px=1, py=0 -> índice 255
    vram[tile1_addr] = 0x88; // px=0, py=0 del tile 1 -> índice 136

    // Verificar lectura 8bpp
    let p0 = calcular_pixel_sprite_vram(&vram, 4, 2, 0, 0, 0, 0, true, true);
    assert_eq!(p0, 0x42, "el píxel 8bpp debe preservar el valor completo 66");

    let p1 = calcular_pixel_sprite_vram(&vram, 4, 2, 0, 0, 1, 0, true, true);
    assert_eq!(p1, 0xFF, "el píxel 8bpp debe preservar el valor completo 255");

    let p2 = calcular_pixel_sprite_vram(&vram, 4, 2, 1, 0, 0, 0, true, true);
    assert_eq!(p2, 0x88, "el tile 1 en 1D debe avanzar 64 bytes (2 bloques de 32 bytes)");

    // Verificar lectura 4bpp (misma base_tile = 4)
    // En 4bpp, byte 0x42 contiene px=0 -> 0x2, px=1 -> 0x4
    let p4bpp_0 = calcular_pixel_sprite_vram(&vram, 4, 2, 0, 0, 0, 0, false, true);
    assert_eq!(p4bpp_0, 0x02, "nibble bajo en 4bpp");

    let p4bpp_1 = calcular_pixel_sprite_vram(&vram, 4, 2, 0, 0, 1, 0, false, true);
    assert_eq!(p4bpp_1, 0x04, "nibble alto en 4bpp");
}

