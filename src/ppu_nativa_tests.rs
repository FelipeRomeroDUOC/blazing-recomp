//! Pruebas satélite de [`crate::ppu_nativa`].
//!
//! Verifican la parte pura del módulo (unidad de mezcla, ventanas y selección
//! de capas) contra los valores de la PPU de referencia de `gba-core`.

use super::*;
use wgpu;

/// El shader tiene que parsear y validar: un error de WGSL solo aparecería al
/// compilar en el dispositivo.
#[test]
fn el_shader_de_la_ppu_nativa_valida_con_naga() {
    let mut validator = naga::valid::Validator::new(
        naga::valid::ValidationFlags::all(),
        naga::valid::Capabilities::all(),
    );
    let module = naga::front::wgsl::parse_str(PPU_NATIVA_WGSL)
        .expect("el shader de ppu_nativa debe parsear correctamente");
    validator
        .validate(&module)
        .expect("el shader de ppu_nativa debe validar con naga");
}

// ===========================================================================
// Unidad de mezcla: identidades y extremos (criterios 2 y 3)
// ===========================================================================

#[test]
fn alfa_identidad_y_extremos() {
    // eva=16/evb=0 deja pasar cada color exactamente, incluido el viaje de ida
    // y vuelta por el carril de 6 bits del verde.
    for c in [0x0000u16, 0x7FFF, 0x7C1F, 0x03E0, 0x1234, 0x5A5A] {
        assert_eq!(alpha_blend(c, 0x7FFF ^ c, 16, 0), c);
        assert_eq!(alpha_blend(0x7FFF ^ c, c, 0, 16), c);
    }
    // Saturación: blanco + blanco a coeficiente pleno satura.
    assert_eq!(alpha_blend(0x7FFF, 0x7FFF, 16, 16), 0x7FFF);
    // evy=0 no hace nada en ninguna dirección de brillo.
    for c in [0x7FFF, 0x1234, 0x5A5A] {
        assert_eq!(brighten(c, 0), c);
        assert_eq!(darken(c, 0), c);
    }
}

#[test]
fn alfa_redondea_a_media_hacia_arriba() {
    // 50/50 de rojo 15 y 16: el valor real es 15.5 y el hardware redondea
    // hacia arriba. (15*8 + 16*8 + 8) >> 4 = 16.
    assert_eq!(alpha_blend(15, 16, 8, 8) & 31, 16);
    // Rojo 0 con 1 a evb=8: 0.5 redondea a 1 (truncar daría 0).
    assert_eq!(alpha_blend(0, 1, 8, 8) & 31, 1);
    // El carril de 6 bits del verde redondea hacia arriba solo a partir de
    // 12/16: con 15/16 a 8/8 el medio paso se queda abajo.
    assert_eq!((alpha_blend(15 << 5, 16 << 5, 8, 8) >> 5) & 31, 15);
    // ...pero verde 15 a eva=4 y 16 a evb=12 (real 15.75) redondea arriba.
    assert_eq!((alpha_blend(15 << 5, 16 << 5, 4, 12) >> 5) & 31, 16);
}

#[test]
fn brillo_redondea() {
    // Aclarar rojo 0 en 1/16: (31*1 + 8) >> 4 = 2 (truncar daría 1).
    assert_eq!(brighten(0, 1) & 31, 2);
    // Oscurecer rojo 31 en 1/16: 31 - ((31 + 7) >> 4) = 29.
    assert_eq!(darken(31, 1) & 31, 29);
    // Carril de 6 bits: aclarar verde 0 en 1/16 da g6 = (63+8)>>4 = 4, que
    // al volver a 5 bits es 2.
    assert_eq!((brighten(0, 1) >> 5) & 31, 2);
}

#[test]
fn brillo_extremos() {
    assert_eq!(brighten(0x0000, 16), 0x7FFF);
    assert_eq!(darken(0x7FFF, 16), 0x0000);
}

#[test]
fn split6_y_join6_son_inversas_en_el_carril_de_verde() {
    // Los canales van en los bits 0-4 (rojo), 5-9 (verde) y 10-14 (azul).
    assert_eq!(split6(0x001F), (31, 0, 0));
    assert_eq!(split6(0x7FFF), (31, 62, 31));
    assert_eq!(split6(0x03E0), (0, 62, 0));
    // El verde entra en un carril de 6 bits (el valor de 5 bits duplicado) y el
    // viaje de vuelta trunca su bit bajo.
    assert_eq!(join6(31, 62, 31), 0x7FFF);
    assert_eq!(join6(31, 63, 31), 0x7FFF);
}

// ===========================================================================
// Registros de la unidad de mezcla
// ===========================================================================

#[test]
fn evb_usa_los_bits_8_a_12_de_bldalpha() {
    let reg = RegistrosColor {
        bldcnt: 0,
        // Bits 8-12 = 8 (EVB = 8). Los bits 5-9 valen 0, así que leerlos mal
        // daría EVB = 0 y la mezcla saldría negra.
        bldalpha: 8 << 8,
        bldy: 0,
    };
    assert_eq!(reg.eva(), 0);
    assert_eq!(reg.evb(), 8);
    assert_eq!(reg.evy(), 0);
}

#[test]
fn los_coeficientes_se_saturan_a_16() {
    let reg = RegistrosColor {
        bldcnt: 0,
        bldalpha: (31 << 8) | 31,
        bldy: 31,
    };
    assert_eq!(reg.eva(), 16);
    assert_eq!(reg.evb(), 16);
    assert_eq!(reg.evy(), 16);
}

#[test]
fn el_modo_de_mezcla_sale_de_los_bits_6_y_7() {
    for (bldcnt, modo) in [(0x0000u16, 0u32), (0x0040, 1), (0x0080, 2), (0x00C0, 3)] {
        assert_eq!(RegistrosColor { bldcnt, ..Default::default() }.modo(), modo);
    }
}

// ===========================================================================
// Ventanas
// ===========================================================================

#[test]
fn rango_de_ventana_incluye_el_borde_izquierdo_y_excluye_el_derecho() {
    assert!(en_rango(10, 20, 10));
    assert!(en_rango(10, 20, 19));
    assert!(!en_rango(10, 20, 20));
    assert!(!en_rango(10, 20, 9));
}

#[test]
fn rango_de_ventana_da_la_vuelta_cuando_el_izquierdo_es_mayor() {
    // x1 > x2: el rango envuelve el cero.
    assert!(en_rango(200, 10, 205));
    assert!(en_rango(200, 10, 5));
    assert!(!en_rango(200, 10, 100));
}

#[test]
fn sin_ventanas_la_mascara_de_control_es_todo() {
    let vent = Ventanas::default();
    assert!(!vent.alguna());
    assert_eq!(mascara_control(&vent, 0, 0, false), 0x3F);
}

#[test]
fn la_ventana_0_manda_sobre_la_ventana_1() {
    let vent = Ventanas {
        win0h: (10 << 8) | 20,
        win1h: (30 << 8) | 40,
        win0v: (0 << 8) | 160,
        win1v: (0 << 8) | 160,
        // Bits 0-5 para la ventana 0 (0x11) y bits 8-13 para la 1 (0x22).
        winin: 0x2211,
        winout: 0x3F,
        win0_on: true,
        win1_on: true,
        objwin_on: false,
    };
    // Dentro de la ventana 0 (x = 10..19).
    assert_eq!(mascara_control(&vent, 15, 0, false), 0x11);
    // Dentro de la ventana 1 pero fuera de la 0 (x = 30..39).
    assert_eq!(mascara_control(&vent, 35, 0, false), 0x22);
    // Fuera de las dos: WINOUT bits 0-5.
    assert_eq!(mascara_control(&vent, 25, 0, false), 0x3F);
}

#[test]
fn la_ventana_obj_manda_sobre_el_fuera_de_ventana() {
    let vent = Ventanas {
        win0h: 0,
        win1h: 0,
        win0v: 0,
        win1v: 0,
        winin: 0x0000,
        // Bits 0-5 fuera de ventana (0x0F) y bits 8-13 dentro de la ventana
        // OBJ (0x30).
        winout: 0x300F,
        win0_on: false,
        win1_on: false,
        objwin_on: true,
    };
    assert_eq!(mascara_control(&vent, 10, 10, true), 0x30);
    assert_eq!(mascara_control(&vent, 10, 10, false), 0x0F);
}

// ===========================================================================
// Composición por píxel
// ===========================================================================

fn registro_modo1() -> RegistrosColor {
    RegistrosColor {
        bldcnt: 0,
        bldalpha: 0,
        bldy: 0,
    }
}

#[test]
fn sin_capas_visibles_sale_el_fondo_de_pantalla() {
    let bg = [TRANSPARENTE; 4];
    let prio = [0xFF; 4];
    let out = compone_pixel(
        &bg,
        &prio,
        PixelObj::VACIO,
        0x3F,
        &registro_modo1(),
        0x1234,
    );
    assert_eq!(out, 0x1234);
}

#[test]
fn el_fondo_manda_si_el_sprite_no_pasa_la_mascara() {
    // Bit 4 de la máscara (sprites) apagado: 0x3F & !0x10 = 0x2F. El sprite no
    // se ve aunque esté ahí.
    let bg = [0x0100, TRANSPARENTE, TRANSPARENTE, TRANSPARENTE];
    let prio = [0, 0xFF, 0xFF, 0xFF];
    let obj = PixelObj {
        color: 0x7FFF,
        prioridad: 0,
        semi: false,
    };
    let out = compone_pixel(&bg, &prio, obj, 0x2F, &registro_modo1(), 0);
    assert_eq!(out, 0x0100);
}

#[test]
fn el_sprite_manda_a_igual_prioridad_que_un_fondo() {
    let bg = [0x0100, TRANSPARENTE, TRANSPARENTE, TRANSPARENTE];
    let prio = [0, 0xFF, 0xFF, 0xFF];
    let obj = PixelObj {
        color: 0x7FFF,
        prioridad: 0,
        semi: false,
    };
    let out = compone_pixel(&bg, &prio, obj, 0x3F, &registro_modo1(), 0);
    assert_eq!(out, 0x7FFF);
}

#[test]
fn un_fondo_con_menor_numero_de_prioridad_manda_al_sprite() {
    // El fondo tiene prioridad 0 y el sprite 1: el fondo va delante.
    let bg = [0x0100, TRANSPARENTE, TRANSPARENTE, TRANSPARENTE];
    let prio = [0, 0xFF, 0xFF, 0xFF];
    let obj = PixelObj {
        color: 0x7FFF,
        prioridad: 1,
        semi: false,
    };
    let out = compone_pixel(&bg, &prio, obj, 0x3F, &registro_modo1(), 0);
    assert_eq!(out, 0x0100);
}

#[test]
fn a_igual_prioridad_gana_el_fondo_de_indice_menor() {
    // Los dos fondos tienen prioridad 0: gana el de índice menor.
    let bg = [0x0100, 0x0200, TRANSPARENTE, TRANSPARENTE];
    let prio = [0, 0, 0xFF, 0xFF];
    let out = compone_pixel(
        &bg,
        &prio,
        PixelObj::VACIO,
        0x3F,
        &registro_modo1(),
        0,
    );
    assert_eq!(out, 0x0100);
}

#[test]
fn el_segundo_fondo_solo_importa_si_hay_efectos() {
    // Dos fondos visibles de prioridad 0 y 1. Sin efectos de color, solo cuenta
    // el primero.
    let bg = [0x0100, 0x0200, TRANSPARENTE, TRANSPARENTE];
    let prio = [0, 1, 0xFF, 0xFF];
    let reg = RegistrosColor {
        bldcnt: 0,
        bldalpha: 0,
        bldy: 0,
    };
    assert_eq!(
        compone_pixel(&bg, &prio, PixelObj::VACIO, 0x3F, &reg, 0),
        0x0100
    );
}

#[test]
fn un_sprite_semitransparente_fuerza_el_alfa_contra_el_segundo_objetivo() {
    // Sprite semitransparente encima, fondo debajo. `BLDCNT` bit 8 (OBJ como
    // segundo objetivo de alfa) activa la mezcla contra el fondo.
    let bg = [0x0000, TRANSPARENTE, TRANSPARENTE, TRANSPARENTE];
    let prio = [1, 0xFF, 0xFF, 0xFF];
    let obj = PixelObj {
        color: 0x7FFF,
        prioridad: 0,
        semi: true,
    };
    let reg = RegistrosColor {
        // Modo 0 (sin efectos), pero el bit 8 marca el OBJ como 2.º objetivo.
        bldcnt: 0x0100,
        // EVA = 8 (sprite) y EVB = 8 (fondo): mezcla al 50 %.
        bldalpha: (8 << 8) | 8,
        bldy: 0,
    };
    let out = compone_pixel(&bg, &prio, obj, 0x3F, &reg, 0);
    let (r, _g, b) = split6(out);
    assert_eq!(r, 16);
    assert_eq!(b, 16);
    // El verde viaja por un carril de 6 bits: el blanco (g5 = 31) da 62, y
    // 62 * 8 / 16 redondea a 31 en el carril, que al volver a 5 bits es 15.
    assert_eq!(out, 0x41F0);
}

#[test]
fn el_modo_aclarar_solo_actua_si_la_mascara_permite_efectos() {
    let bg = [0x0000, TRANSPARENTE, TRANSPARENTE, TRANSPARENTE];
    let prio = [0, 0xFF, 0xFF, 0xFF];
    let reg = RegistrosColor {
        // Modo 2 (aclarar) y bit 0: BG0 como primer objetivo.
        bldcnt: 0x0081,
        bldalpha: 0,
        bldy: 16,
    };
    // Con efectos permitidos (bit 5 de la máscara).
    assert_eq!(
        compone_pixel(&bg, &prio, PixelObj::VACIO, 0x3F, &reg, 0),
        0x7FFF
    );
    // Sin efectos permitidos: el color pasa intacto.
    assert_eq!(
        compone_pixel(&bg, &prio, PixelObj::VACIO, 0x1F, &reg, 0),
        0x0000
    );
}

#[test]
fn el_modo_oscurecer_reduce_el_color() {
    let bg = [0x7FFF, TRANSPARENTE, TRANSPARENTE, TRANSPARENTE];
    let prio = [0, 0xFF, 0xFF, 0xFF];
    let reg = RegistrosColor {
        // Modo 3 (oscurecer) y bit 0: BG0 como primer objetivo.
        bldcnt: 0x00C1,
        bldalpha: 0,
        bldy: 16,
    };
    assert_eq!(
        compone_pixel(&bg, &prio, PixelObj::VACIO, 0x3F, &reg, 0),
        0x0000
    );
}

// ===========================================================================
// Decodificación de OAM
// ===========================================================================

/// OAM con las 128 entradas deshabilitadas (bit 9 de `attr0`).
fn oam_vacio() -> Vec<u8> {
    let mut oam = vec![0u8; 1024];
    for i in 0..128 {
        oam[i * 8..i * 8 + 2].copy_from_slice(&0x0200u16.to_le_bytes());
    }
    oam
}

/// OAM con las entradas indicadas activas y el resto deshabilitado.
fn oam_con(entradas: &[(usize, u16, u16, u16)]) -> Vec<u8> {
    let mut oam = oam_vacio();
    for (i, a0, a1, a2) in entradas {
        let off = i * 8;
        oam[off..off + 2].copy_from_slice(&a0.to_le_bytes());
        oam[off + 2..off + 4].copy_from_slice(&a1.to_le_bytes());
        oam[off + 4..off + 6].copy_from_slice(&a2.to_le_bytes());
    }
    oam
}

/// OAM con una sola entrada activa.
fn oam_de_una_entrada(attr0: u16, attr1: u16, attr2: u16) -> Vec<u8> {
    oam_con(&[(0, attr0, attr1, attr2)])
}

#[test]
fn los_sprites_deshabilitados_no_se_decodifican() {
    let mut estado = EstadoPpuNativa::new();
    estado.dispcnt = 0x1000;
    // Bit 9 de attr0: deshabilitado (en sprites no afines).
    let oam = oam_de_una_entrada(0x0200 | 10, 20, 0);
    assert_eq!(estado.decode_oam(&oam), 0);
}

#[test]
fn la_forma_prohibida_no_se_decodifica() {
    let mut estado = EstadoPpuNativa::new();
    estado.dispcnt = 0x1000;
    // Bits 14-15 = 3: forma prohibida.
    let oam = oam_de_una_entrada(0xC000 | 10, 20, 0);
    assert_eq!(estado.decode_oam(&oam), 0);
}

#[test]
fn sin_obj_activo_no_hay_sprites() {
    let mut estado = EstadoPpuNativa::new();
    // DISPCNT sin el bit 12 (OBJ enable).
    estado.dispcnt = 0x0000;
    let oam = oam_de_una_entrada(10, 20, 0);
    assert_eq!(estado.decode_oam(&oam), 0);
}

#[test]
fn la_posicion_del_sprite_se_decodifica_con_signo() {
    let mut estado = EstadoPpuNativa::new();
    estado.dispcnt = 0x1000;
    // Y = 250 se interpreta como -6 y X = 505 como -7: la esquina del sprite
    // asoma por arriba a la izquierda, así que sigue siendo visible.
    let oam = oam_de_una_entrada(250, 505, 0);
    assert_eq!(estado.decode_oam(&oam), 1);
    let s = estado.sprites[0];
    assert_eq!(s.pos_x, -7.0);
    assert_eq!(s.pos_y, -6.0);
}

#[test]
fn un_sprite_completamente_fuera_de_pantalla_se_descarta() {
    let mut estado = EstadoPpuNativa::new();
    estado.dispcnt = 0x1000;
    // X = 248 se interpreta como -264: enteramente a la izquierda.
    let oam = oam_de_una_entrada(10, 248, 0);
    assert_eq!(estado.decode_oam(&oam), 0);
}

#[test]
fn el_tamano_del_sprite_sale_de_la_forma_y_el_tamano() {
    let mut estado = EstadoPpuNativa::new();
    estado.dispcnt = 0x1000;
    // Forma horizontal (1), tamaño 1: 32x8.
    let oam = oam_de_una_entrada((1 << 14) | 10, (1 << 14) | 20, 0);
    assert_eq!(estado.decode_oam(&oam), 1);
    let s = estado.sprites[0];
    assert_eq!(s.caja_x, 32.0);
    assert_eq!(s.caja_y, 8.0);
}

#[test]
fn el_modo_obj_separa_sprites_de_color_y_de_ventana() {
    let mut estado = EstadoPpuNativa::new();
    estado.dispcnt = 0x1000;

    // Sprite 0 en modo ventana (attr0 bits 10-11 = 2) y sprite 1 en modo normal.
    let oam = oam_con(&[
        (0, (2 << 10) | 10, 20, 0),
        (1, (1 << 10) | 30, 40, 0),
    ]);

    assert_eq!(estado.decode_oam(&oam), 1);
    assert_eq!(estado.num_sprites, 1);
    assert_eq!(estado.num_sprites_ventana, 1);
    assert_eq!(estado.sprites[0].pos_y, 30.0);
    assert_eq!(estado.sprites_ventana[0].pos_y, 10.0);
}

#[test]
fn el_indice_de_oam_se_conserva_para_el_desempate() {
    let mut estado = EstadoPpuNativa::new();
    estado.dispcnt = 0x1000;
    // Sprites 0 y 5, los dos con prioridad 0.
    let oam = oam_con(&[(0, 10, 20, 0), (5, 30, 40, 0)]);
    assert_eq!(estado.decode_oam(&oam), 2);
    // La clave de profundidad ordena por (prioridad, índice de OAM): el
    // desempate lo gana el índice menor, que es el 0. El orden del array es el
    // de dibujado (índice descendente), así que el ganador se dibuja el
    // último y la clave menor es la de este último elemento.
    let claves: Vec<u32> = estado.sprites[..estado.num_sprites]
        .iter()
        .map(|s| s.prioridad * 128 + s.oam_index)
        .collect();
    assert_eq!(claves, vec![5, 0]);
    assert_eq!(*claves.iter().min().unwrap(), 0);
}

// ===========================================================================
// Decodificación de registros de fondo
// ===========================================================================

#[test]
fn la_configuracion_del_fondo_sale_de_bgx_cnt() {
    let mut io = vec![0u8; 0x60];
    // BG0CNT: prioridad 1, base de caracteres 2, base de pantalla 0x1F,
    // 8 bits por píxel, tamaño 64x64.
    let cnt: u16 = 1 | (2 << 2) | (0x80) | (0x1F << 8) | (3 << 14);
    io[0x08..0x0A].copy_from_slice(&cnt.to_le_bytes());
    // BG0HOFS = 0x123, BG0VOFS = 0x045 (más de 9 bits: se enmascara a 0x1FF).
    io[0x10..0x12].copy_from_slice(&0x123u16.to_le_bytes());
    io[0x12..0x14].copy_from_slice(&0x145u16.to_le_bytes());

    let capa = CapaFondoGpu::de_io(&io, 0, true);
    assert_eq!(capa.priority, 1);
    assert_eq!(capa.char_base, 2 * 0x4000);
    assert_eq!(capa.screen_base, 0x1F * 0x800);
    assert_eq!(capa.screen_size, 3);
    assert_eq!(capa.is_8bpp, 1);
    assert_eq!(capa.scroll_x, 0x123);
    assert_eq!(capa.scroll_y, 0x145 & 0x1FF);
}

#[test]
fn un_fondo_no_usado_tiene_prioridad_0xff() {
    let mut estado = EstadoPpuNativa::new();
    let mut io = vec![0u8; 0x60];
    // Modo 0, solo BG0 activo.
    io[0x00..0x02].copy_from_slice(&0x0100u16.to_le_bytes());
    estado.decode_io_regs(&io);
    assert_eq!(estado.bg_prio[0], 0);
    assert_eq!(estado.bg_prio[1], 0xFF);
    assert_eq!(estado.bg_prio[2], 0xFF);
    assert_eq!(estado.bg_prio[3], 0xFF);
}

// ===========================================================================
// Decodificación de los registros del fondo afín
// ===========================================================================

/// Los registros del fondo afín se decodifican con su signo.
///
/// `PA`, `PB`, `PC` y `PD` son enteros de 16 bits con signo en punto fijo 20.8,
/// y `BG2X` y `BG2Y` son de 28 bits con signo: el bit 28 está fuera del
/// registro y se descarta, igual que hace `sign_extend28` de la referencia.
#[test]
fn los_registros_del_fondo_afin_llevan_el_signo() {
    let mut io = vec![0u8; 0x60];
    // DISPCNT: modo 1 (bits 0-2 = 1) y fondo afín BG2 activado (bit 10).
    io[0x00..0x02].copy_from_slice(&0x0401u16.to_le_bytes());
    // BG2CNT: wrap activo (bit 13) y tamaño de 128 tiles (bits 14-15 = 3).
    io[0x0C..0x0E].copy_from_slice(&0xE000u16.to_le_bytes());
    // BG2PA = 0xFF9C (-100), BG2PB = 0x0001, BG2PC = 0xFF85 (-123), BG2PD = 0x00E8.
    io[0x20..0x22].copy_from_slice(&0xFF9Cu16.to_le_bytes());
    io[0x22..0x24].copy_from_slice(&0x0001u16.to_le_bytes());
    io[0x24..0x26].copy_from_slice(&0xFF85u16.to_le_bytes());
    io[0x26..0x28].copy_from_slice(&0x00E8u16.to_le_bytes());
    // BG2X = 0x0FFFFFFF (28 bits con signo: -1) y BG2Y = 0x10000000: el bit 28
    // está fuera de los 28 bits del registro, así que vale 0.
    io[0x28..0x2C].copy_from_slice(&0x0FFFFFFFu32.to_le_bytes());
    io[0x2C..0x30].copy_from_slice(&0x10000000u32.to_le_bytes());

    let mut estado = EstadoPpuNativa::new();
    estado.decode_io_regs(&io);

    let capa = estado.capas[2];
    assert_eq!(capa.afin_pa, -100, "BG2PA con signo");
    assert_eq!(capa.afin_pb, 1, "BG2PB con signo");
    assert_eq!(capa.afin_pc, -123, "BG2PC con signo");
    assert_eq!(capa.afin_pd, 0xE8, "BG2PD con signo");
    assert_eq!(capa.afin_ref_x, -1, "BG2X con 28 bits con signo");
    assert_eq!(capa.afin_ref_y, 0, "BG2Y descarta el bit 28");
    assert_eq!(capa.afin_wrap, 1, "el wrap es el bit 13 de BGxCNT");
    assert_eq!(capa.screen_size, 3, "el tamaño del mapa afín son 16 << 3 tiles");
    // Y el fondo afín está activo, con su prioridad.
    assert_eq!(capa.enabled, 1, "BG2 se dibuja en el modo 1 con el bit 10");
    assert_eq!(estado.bg_prio[2], 0, "la prioridad de BG2 sale de BG2CNT");
    assert_eq!(estado.bg_prio[3], 0xFF, "BG3 no se usa en el modo 1");
}

// ===========================================================================
// Marca de archivo en runtime: encender y apagar la PPU nativa sin compilar
// ===========================================================================

/// Crea un directorio temporal único para una prueba de la marca.
fn dir_temporal(nombre: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "blazing-ppu-nativa-{}-{}",
        std::process::id(),
        nombre
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("se debe poder crear el directorio temporal de la prueba");
    dir
}

#[test]
fn sin_marca_la_ppu_nativa_esta_activa() {
    let dir = dir_temporal("sin-marca");
    // La PPU nativa es la ruta por defecto: sin ningún archivo, está activa.
    // Es el estado de un APK recién instalado.
    assert!(ppu_nativa_activada(std::slice::from_ref(&dir)));
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn la_marca_de_apagado_apaga_la_ppu_nativa() {
    let dir = dir_temporal("con-marca-apagado");
    std::fs::write(dir.join(NOMBRE_BANDERA_APAGADO), b"")
        .expect("se debe poder escribir la marca");
    assert!(!ppu_nativa_activada(std::slice::from_ref(&dir)));
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn quitar_la_marca_de_apagado_vuelve_a_encender_la_ppu_nativa() {
    let dir = dir_temporal("quitar-marca-apagado");
    let ruta = dir.join(NOMBRE_BANDERA_APAGADO);
    std::fs::write(&ruta, b"").expect("se debe poder escribir la marca");
    assert!(!ppu_nativa_activada(std::slice::from_ref(&dir)));
    // Quitar el archivo con la app en marcha debe devolver la PPU nativa.
    std::fs::remove_file(&ruta).expect("se debe poder borrar la marca");
    assert!(ppu_nativa_activada(std::slice::from_ref(&dir)));
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn el_nombre_de_la_marca_de_apagado_es_el_documentado() {
    // La guía de medición dice el nombre exacto del archivo. Si cambia, hay
    // que actualizar la guía, y este test lo hace inevitable.
    assert_eq!(NOMBRE_BANDERA_APAGADO, "ppu-nativa-off.flag");
}

// ===========================================================================
// Guardián de errores de la GPU
// ===========================================================================

/// Motor de errores de mentira: registra los ámbitos que se abren y devuelve
/// los errores que se le configuren, sin necesidad de GPU.
struct MotorFalso {
    /// Errores que devolverá `cerrar_ambito` (el último es el primero que
    /// sale, como en los ámbitos reales, que son una pila).
    errores: std::cell::RefCell<Vec<String>>,
    /// Filtros de los ámbitos abiertos, en orden.
    filtros: std::cell::RefCell<Vec<wgpu::ErrorFilter>>,
}

impl MotorFalso {
    fn nuevo(errores: &[&str]) -> Self {
        MotorFalso {
            errores: std::cell::RefCell::new(errores.iter().map(|e| e.to_string()).collect()),
            filtros: std::cell::RefCell::new(Vec::new()),
        }
    }
}

impl MotorDeErrores for MotorFalso {
    fn abrir_ambito(&self, filtro: wgpu::ErrorFilter) {
        self.filtros.borrow_mut().push(filtro);
    }

    fn cerrar_ambito(&self) -> Option<String> {
        self.errores.borrow_mut().pop()
    }
}

#[test]
fn el_guardian_devuelve_el_error_en_vez_de_dejar_la_ppu_nativa_a_medias() {
    let motor = MotorFalso::nuevo(&["la GPU rechazó la textura"]);
    let resultado = construir_con_guardian(&motor, "PPU nativa", || 42);
    let error = resultado.expect_err("un error de la GPU debe devolver Err");
    // El error debe decir qué falló y con qué mensaje.
    assert!(
        error.starts_with("PPU nativa"),
        "el error debe nombrar el recurso: {error}"
    );
    assert!(
        error.contains("la GPU rechazó la textura"),
        "el error debe llevar el mensaje de la GPU: {error}"
    );
    // Se abre un ámbito por cada filtro de error: con menos, un error de ese
    // tipo no entraría en el ámbito y `wgpu` entraría en pánico.
    let filtros = motor.filtros.borrow();
    assert_eq!(
        filtros.len(),
        FILTROS_ERROR.len(),
        "debe abrirse un ámbito por cada filtro de error"
    );
    for filtro in FILTROS_ERROR {
        assert!(
            filtros.contains(&filtro),
            "falta el ámbito para el filtro {filtro:?}"
        );
    }
}

#[test]
fn el_guardian_deja_pasar_una_construccion_sin_error() {
    let motor = MotorFalso::nuevo(&[]);
    let valor = construir_con_guardian(&motor, "PPU nativa", || 42)
        .expect("sin errores de la GPU debe devolver Ok");
    assert_eq!(valor, 42);
}

#[test]
fn el_guardian_informa_de_todos_los_errores_recogidos() {
    let motor = MotorFalso::nuevo(&["primer error", "segundo error"]);
    let error = construir_con_guardian(&motor, "PPU nativa", || 0)
        .expect_err("un error de la GPU debe devolver Err");
    assert!(
        error.contains("primer error") && error.contains("segundo error"),
        "el error debe recoger todos los ámbitos con fallo: {error}"
    );
}

// ===========================================================================
// Layouts de enlaces y búfer de instancias
// ===========================================================================

/// Ningún *storage buffer* puede ser visible desde la etapa de vértices.
///
/// Exigirlo aborta la creación del layout en los dispositivos sin
/// `DownlevelFlags::VERTEX_STORAGE`: el estándar de OpenGL ES permite que un
/// dispositivo tenga SSBO solo en fragmento, y en ese caso la app muere con
/// un error de validación de wgpu.
#[test]
fn ningun_storage_buffer_es_visible_en_la_etapa_de_vertices() {
    let layouts: [(&str, Vec<wgpu::BindGroupLayoutEntry>); 3] = [
        ("obj", super::entradas_bgl_obj().to_vec()),
        ("obj-ventana", super::entradas_bgl_obj_ventana().to_vec()),
        ("comp", super::entradas_bgl_comp().to_vec()),
    ];
    for (nombre, entradas) in layouts {
        for entrada in &entradas {
            if let wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Storage { .. },
                ..
            } = &entrada.ty
            {
                assert!(
                    !entrada.visibility.contains(wgpu::ShaderStages::VERTEX),
                    "el layout `{nombre}` declara el binding {} como storage \
                     visible en la etapa de vértices",
                    entrada.binding
                );
            }
        }
    }
}

/// El búfer de instancias lleva la geometría y la clave de prioridad que el
/// vértice necesita, con el formato que declara el layout del búfer.
#[test]
fn el_buffer_de_instancias_lleva_lo_que_el_vertices_necesita() {
    let sprite = SpriteNativa {
        pos_x: 10.0,
        pos_y: 20.0,
        caja_x: 16.0,
        caja_y: 16.0,
        prioridad: 1,
        oam_index: 5,
        ..SpriteNativa::default()
    };
    let instancia = SpriteInstanciaGpu::de_sprite(&sprite);
    assert_eq!(instancia.geometria, [10.0, 20.0, 16.0, 16.0]);
    assert_eq!(instancia.clave, [1, 5]);

    let layout = super::layout_instancia();
    assert_eq!(layout.step_mode, wgpu::VertexStepMode::Instance);
    assert_eq!(
        layout.array_stride, 24,
        "la instancia son 16 bytes de geometría y 8 de clave"
    );
    assert_eq!(layout.attributes.len(), 2);

    let geometria = &layout.attributes[0];
    assert_eq!(geometria.shader_location, 1);
    assert_eq!(geometria.format, wgpu::VertexFormat::Float32x4);
    assert_eq!(geometria.offset, 0);

    let clave = &layout.attributes[1];
    assert_eq!(clave.shader_location, 2);
    assert_eq!(clave.format, wgpu::VertexFormat::Uint32x2);
    assert_eq!(clave.offset, 16);
}

#[test]
fn la_lut_vertical_conserva_scroll_y_mezcla_por_linea() {
    let mut io_lineas = [[0u8; 0x60]; 160];

    // Mitad superior (líneas 0..80): scroll Y = 0, BLDCNT = 0x0041 (mezcla alfa activa en BG0)
    for io in io_lineas[0..80].iter_mut() {
        // DISPCNT = 0x0100 (BG0 activo en modo 0)
        io[0x00..0x02].copy_from_slice(&0x0100u16.to_le_bytes());
        // BG0VOFS = 0
        io[0x12..0x14].copy_from_slice(&0u16.to_le_bytes());
        // BLDCNT = 0x0041
        io[0x50..0x52].copy_from_slice(&0x0041u16.to_le_bytes());
        // BLDALPHA = (8 << 8) | 8
        io[0x52..0x54].copy_from_slice(&((8u16 << 8) | 8u16).to_le_bytes());
    }

    // Mitad inferior (líneas 80..160): scroll Y = 4, BLDCNT = 0 (sin efectos)
    for io in io_lineas[80..160].iter_mut() {
        io[0x00..0x02].copy_from_slice(&0x0100u16.to_le_bytes());
        io[0x12..0x14].copy_from_slice(&4u16.to_le_bytes());
        io[0x50..0x52].copy_from_slice(&0u16.to_le_bytes());
    }

    let (capas, comps) = EstadoPpuNativa::decodificar_lineas(&io_lineas);

    // Verificar línea 20
    assert_eq!(capas[20].capas[0].scroll_y, 0);
    assert_eq!(comps[20].bldcnt, 0x0041);
    assert_eq!(comps[20].bldalpha, (8 << 8) | 8);

    // Verificar línea 120
    assert_eq!(capas[120].capas[0].scroll_y, 4);
    assert_eq!(comps[120].bldcnt, 0);
}

#[test]
fn la_lut_vertical_de_paleta_conserva_colores_por_linea_y_enmascara_bit_15() {
    let mut paleta_lineas = [[0u8; 1024]; 160];

    // En la línea 90, el índice 20 vale 0x7AD2 (azul celeste)
    let off = 20 * 2;
    paleta_lineas[90][off..off + 2].copy_from_slice(&0x7AD2u16.to_le_bytes());

    // En la línea 110, el índice 20 vale 0xFFFF (blanco con bit 15 activo)
    paleta_lineas[110][off..off + 2].copy_from_slice(&0xFFFFu16.to_le_bytes());

    let mut destino = Box::new([[0u32; 512]; 160]);
    EstadoPpuNativa::decodificar_paletas_linea(&paleta_lineas, &mut destino);

    assert_eq!(
        destino[90][20], 0x7AD2,
        "la línea 90 debe conservar el azul celeste 0x7AD2"
    );
    assert_eq!(
        destino[110][20], 0x7FFF,
        "la línea 110 debe enmascarar el bit 15 a 0x7FFF"
    );
    assert_eq!(
        destino[0][20], 0x0000,
        "otras líneas deben permanecer sin contaminarse"
    );
}
