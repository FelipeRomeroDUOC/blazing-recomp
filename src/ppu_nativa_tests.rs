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
        assert_eq!(
            RegistrosColor {
                bldcnt,
                ..Default::default()
            }
            .modo(),
            modo
        );
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
    let out = compone_pixel(&bg, &prio, PixelObj::VACIO, 0x3F, &registro_modo1(), 0x1234);
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
    let out = compone_pixel(&bg, &prio, PixelObj::VACIO, 0x3F, &registro_modo1(), 0);
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
    let oam = oam_con(&[(0, (2 << 10) | 10, 20, 0), (1, (1 << 10) | 30, 40, 0)]);

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
    assert_eq!(
        capa.screen_size, 3,
        "el tamaño del mapa afín son 16 << 3 tiles"
    );
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
    use std::sync::atomic::{AtomicUsize, Ordering};
    static CONTADOR: AtomicUsize = AtomicUsize::new(0);
    let id = CONTADOR.fetch_add(1, Ordering::SeqCst);
    let base = std::env::current_dir()
        .map(|d| d.join("target").join("tmp"))
        .unwrap_or_else(|_| std::env::temp_dir());
    let dir = base.join(format!(
        "blazing-ppu-nativa-{}-{}-{}",
        std::process::id(),
        nombre,
        id
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
    std::fs::write(dir.join(NOMBRE_BANDERA_APAGADO), b"").expect("se debe poder escribir la marca");
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

// ===========================================================================
// Subida diferencial y deduplicación por línea
// ===========================================================================

#[test]
fn rango_filas_modificadas_detecta_identicos_y_acota_subrango() {
    let base = vec![0u32; 160];
    assert_eq!(rango_filas_modificadas(&base, &base.clone()), None);

    let mut una = base.clone();
    una[42] = 7;
    assert_eq!(rango_filas_modificadas(&base, &una), Some((42, 42)));

    let mut dos = base.clone();
    dos[80] = 1;
    dos[115] = 2;
    assert_eq!(rango_filas_modificadas(&base, &dos), Some((80, 115)));

    // Bordes: primera y ultima fila.
    let mut bordes = base.clone();
    bordes[0] = 9;
    bordes[159] = 9;
    assert_eq!(rango_filas_modificadas(&base, &bordes), Some((0, 159)));
}

#[test]
fn deduplicacion_por_linea_conserva_resultado_exacto() {
    // Tramos: 0..50 iguales, 50..120 distintos entre si, 120..160 iguales.
    let mut io_lineas = [[0u8; 0x60]; 160];
    let mut paleta_lineas = [[0u8; 1024]; 160];
    for py in 0..160 {
        let tramo = if py < 50 {
            0u8
        } else if py < 120 {
            py as u8
        } else {
            200u8
        };
        io_lineas[py][0] = tramo; // DISPCNT bajo
        io_lineas[py][0x50] = tramo; // BLDCNT bajo
        io_lineas[py][0x54] = tramo & 0x1F; // BLDY
        paleta_lineas[py][0] = tramo;
        paleta_lineas[py][1] = 0xFF; // bit 15 activo: debe enmascararse
        paleta_lineas[py][1023] = tramo;
    }

    // Referencia ingenua: decodificar cada linea de forma independiente.
    let mut esperado_capas = [ConfigCapaGpu::default(); 160];
    let mut esperado_comps = [ConfigCompGpu::default(); 160];
    let mut esperado_pal = Box::new([[0u32; 512]; 160]);
    for py in 0..160 {
        let mut e = EstadoPpuNativa::new();
        e.decode_io_regs(&io_lineas[py]);
        esperado_capas[py] = e.config_capa();
        esperado_comps[py] = e.config_comp();
        for idx in 0..512 {
            let off = idx * 2;
            esperado_pal[py][idx] =
                (u16::from_le_bytes([paleta_lineas[py][off], paleta_lineas[py][off + 1]]) & 0x7FFF)
                    as u32;
        }
    }

    let (capas, comps) = EstadoPpuNativa::decodificar_lineas(&io_lineas);
    assert_eq!(capas, esperado_capas);
    assert_eq!(comps, esperado_comps);

    let mut pal = Box::new([[0u32; 512]; 160]);
    EstadoPpuNativa::decodificar_paletas_linea(&paleta_lineas, &mut pal);
    assert_eq!(*pal, *esperado_pal);
}

#[test]
fn test_sprites_modificados_detecta_cambios_y_omite_vacios() {
    let mut sprite_a = SpriteNativa::default();
    sprite_a.pos_x = 10.0;
    sprite_a.pos_y = 20.0;

    let mut sprite_b = SpriteNativa::default();
    sprite_b.pos_x = 15.0;
    sprite_b.pos_y = 25.0;

    // Caso 1: lista actual vacía -> NUNCA sube a GPU
    assert!(!sprites_modificados::<SpriteNativa>(true, &[], &[]));
    assert!(!sprites_modificados(false, &[sprite_a], &[]));

    // Caso 2: primera subida con sprites -> SIEMPRE sube
    assert!(sprites_modificados(true, &[], &[sprite_a]));

    // Caso 3: cuadro subsecuente con lista idéntica -> NO sube
    assert!(!sprites_modificados(false, &[sprite_a], &[sprite_a]));

    // Caso 4: cambio de coordenadas o atributos -> SÍ sube
    assert!(sprites_modificados(false, &[sprite_a], &[sprite_b]));

    // Caso 5: cambio en la cantidad de sprites -> SÍ sube
    assert!(sprites_modificados(
        false,
        &[sprite_a],
        &[sprite_a, sprite_b]
    ));
    assert!(sprites_modificados(
        false,
        &[sprite_a, sprite_b],
        &[sprite_a]
    ));
}

#[test]
fn test_sprites_decodificados_si_lineas_activas_tienen_obj_aunque_fin_lo_apague() {
    let mut io_lineas = [[0u8; 0x60]; 160];
    for py in 0..160 {
        io_lineas[py][0] = 0x40; // 1D mapping
        io_lineas[py][1] = 0x10; // bit 12 DISPCNT: OBJ activo (0x1040)
    }
    let mut io_fin = [0u8; 0x60];
    io_fin[0] = 0x01; // Modo 1, OBJ inactivo (0x0001)

    let (_capas, mut comps) = EstadoPpuNativa::decodificar_lineas(&io_lineas);
    let mut estado = EstadoPpuNativa::new();
    estado.decode_io_regs(&io_fin);
    assert_eq!(
        estado.dispcnt & 0x1000,
        0,
        "Al inicio io_fin tiene OBJ apagado"
    );

    estado.consolidar_dispcnt_sprites(&mut comps);
    assert_ne!(
        estado.dispcnt & 0x1000,
        0,
        "Tras consolidar, OBJ debe estar habilitado"
    );

    let oam = oam_de_una_entrada(10, 20, 0);
    estado.decode_oam(&oam);
    assert!(
        estado.num_sprites > 0,
        "Los sprites deben decodificarse si las líneas visibles los tienen activos"
    );
    assert_ne!(estado.dispcnt & 0x1000, 0);
}

#[test]
fn test_visibilidad_sprite_respeta_bit_obj_en_config_comp() {
    let mut io_inactivo = [0u8; 0x60];
    io_inactivo[0] = 0x01; // DISPCNT = 0x0001 (bit 12 inactivo)
    let mut estado = EstadoPpuNativa::new();
    estado.decode_io_regs(&io_inactivo);
    let cfg = estado.config_comp();
    assert_eq!(cfg.dispcnt & 0x1000, 0);

    let disp_obj_on = (cfg.dispcnt & 0x1000) != 0;
    let control_default = 0x3Fu32;
    let obj_presente = true;
    let obj_visible = obj_presente && disp_obj_on && (control_default & 0x10) != 0;
    assert!(
        !obj_visible,
        "Sprite no debe ser visible si DISPCNT bit 12 está inactivo"
    );

    assert!(PPU_NATIVA_WGSL.contains("let disp_obj_on = (cfg_comp.dispcnt & 0x1000u) != 0u;"));
    assert!(PPU_NATIVA_WGSL
        .contains("let obj_visible = obj_presente && disp_obj_on && (control & 0x10u) != 0u;"));
}

#[test]
fn test_cache_sprites_estable_entre_fotogramas_con_io_lineas() {
    let mut io_lineas = [[0u8; 0x60]; 160];
    for py in 0..160 {
        io_lineas[py][0] = 0x40;
        io_lineas[py][1] = 0x10;
    }
    let mut io_fin = [0u8; 0x60];
    io_fin[0] = 0x01;
    let oam = oam_de_una_entrada(10, 20, 0);

    // Cuadro 1 (primer fotograma)
    let (_capas1, mut comps1) = EstadoPpuNativa::decodificar_lineas(&io_lineas);
    let mut estado1 = EstadoPpuNativa::new();
    estado1.decode_io_regs(&io_fin);
    estado1.consolidar_dispcnt_sprites(&mut comps1);
    let dispcnt_oam1 = estado1.dispcnt & 0x1040;
    assert_eq!(dispcnt_oam1, 0x1040);

    let mut oam_previa = [0u8; 1024];
    oam_previa.copy_from_slice(&oam[..1024]);
    let dispcnt_oam_previo = dispcnt_oam1;

    estado1.decode_oam(&oam);
    let n1 = estado1.num_sprites;
    assert!(n1 > 0);

    let mut sprites_previos = [SpriteNativa::default(); MAX_SPRITES];
    sprites_previos[..n1].copy_from_slice(&estado1.sprites[..n1]);
    let num_sprites_previo = n1;

    // Cuadro 2 (siguiente fotograma con mismos datos)
    let (_capas2, mut comps2) = EstadoPpuNativa::decodificar_lineas(&io_lineas);
    let mut estado2 = EstadoPpuNativa::new();
    estado2.decode_io_regs(&io_fin);
    estado2.consolidar_dispcnt_sprites(&mut comps2);
    let dispcnt_oam2 = estado2.dispcnt & 0x1040;

    let oam_valida = oam.len() >= 1024;
    let primera = false;
    let oam_cambio = primera
        || !oam_valida
        || dispcnt_oam2 != dispcnt_oam_previo
        || (oam_valida && oam_previa[..] != oam[..1024]);

    assert!(
        !oam_cambio,
        "oam_cambio debe ser false si OAM y dispcnt_oam consolidado se mantienen idénticos"
    );

    estado2.decode_oam(&oam);
    let sprites_cambiaron = sprites_modificados(
        primera,
        &sprites_previos[..num_sprites_previo],
        &estado2.sprites[..estado2.num_sprites],
    );
    assert!(
        !sprites_cambiaron,
        "sprites_modificados debe devolver false cuando dos cuadros sucesivos procesan sprites idénticos"
    );
}

#[test]
fn test_fondos_consolidados_si_lineas_activas_tienen_bg_aunque_fin_lo_apague() {
    let mut io_lineas = [[0u8; 0x60]; 160];
    for py in 0..160 {
        // DISPCNT: modo 0 (0x00), BG2 activo (bit 10 = 0x04 en byte 1 -> 0x0400)
        io_lineas[py][0] = 0x00;
        io_lineas[py][1] = 0x04; // BG2 activo
        io_lineas[py][0x0C] = 0x02; // BG2CNT prioridad 2
    }
    let mut io_fin = [0u8; 0x60];
    io_fin[0] = 0x00;
    io_fin[1] = 0x00; // BG2 inactivo en fin de cuadro
    io_fin[0x0C] = 0x02;

    let (_capas, comps) = EstadoPpuNativa::decodificar_lineas(&io_lineas);
    let mut estado = EstadoPpuNativa::new();
    estado.decode_io_regs(&io_fin);
    assert_eq!(
        estado.dispcnt & 0x0400,
        0,
        "Al inicio io_fin tiene BG2 apagado"
    );
    assert_eq!(
        estado.bg_prio[2], 0xFF,
        "Al inicio io_fin deja prioridad de BG2 inactiva"
    );

    estado.consolidar_dispcnt_fondos(&comps);
    assert_ne!(
        estado.dispcnt & 0x0400,
        0,
        "Tras consolidar, BG2 debe estar habilitado en DISPCNT"
    );
    assert_eq!(
        estado.capas[2].enabled, 1,
        "Tras consolidar, capa 2 debe estar habilitada"
    );
    assert_eq!(
        estado.bg_prio[2], 2,
        "Tras consolidar, prioridad de BG2 debe preservarse"
    );
}

#[test]
fn test_necesita_segunda_activa_para_fondo_con_alpha_blending() {
    // BLDCNT: modo 1 (bits 6-7 = 1 -> 0x0040), 1st target BG2 (bit 2 -> 0x0004),
    // 2nd target BG3 (bit 11 -> 0x0800). Total = 0x0844
    let bldcnt = 0x0844u32;
    let modo = (bldcnt >> 6) & 3; // 1 (Alpha Blending)
    let primera_id = 2u32; // BG2
    let obj_visible = false;
    let obj_semi = false;
    let efectos = true;

    assert!(
        necesita_segunda(primera_id, obj_visible, obj_semi, efectos, modo, bldcnt),
        "Alpha Blending en BG2 requiere buscar segunda capa para mezclar"
    );

    // Si modo == 0 (sin efecto), no necesita segunda capa
    assert!(
        !necesita_segunda(primera_id, obj_visible, obj_semi, efectos, 0, bldcnt),
        "Modo 0 no requiere segunda capa"
    );

    // Si primera capa no es 1st target en BLDCNT, no necesita segunda capa
    let bldcnt_sin_bg2 = 0x0840u32;
    assert!(
        !necesita_segunda(
            primera_id,
            obj_visible,
            obj_semi,
            efectos,
            modo,
            bldcnt_sin_bg2
        ),
        "Capa que no es 1st target no requiere segunda capa"
    );

    // Verificar además que compone_pixel realiza la mezcla esperada
    let bg = [0x0000, 0x0000, 0x001F, 0x03E0]; // BG2 rojo puro (0x001F), BG3 verde puro (0x03E0)
    let bg_prio = [0xFF, 0xFF, 1, 2]; // BG2 delante (prio 1), BG3 detrás (prio 2)
    let obj = PixelObj::default();
    let control = 0x3F; // efectos y fondos activos
    let reg = RegistrosColor {
        bldcnt: bldcnt as u16,
        bldalpha: 0x0808, // eva = 8 (50%), evb = 8 (50%)
        bldy: 0,
    };
    let backdrop = 0x0000;
    let resultado = compone_pixel(&bg, &bg_prio, obj, control, &reg, backdrop);
    // Mezcla de 50% rojo y 50% verde = 0x01F0 (496)
    assert_ne!(
        resultado, 0x001F,
        "El color debe ser mezclado, no rojo puro"
    );
    assert_ne!(
        resultado, 0x03E0,
        "El color debe ser mezclado, no verde puro"
    );
    assert_eq!(
        resultado, 0x01F0,
        "El color debe ser la mezcla 50/50 esperada"
    );
}

#[test]
fn test_subida_diferencial_filas_capa_y_comp_rango_exacto() {
    let prev = [ConfigCompGpu::default(); 160];
    let mut act = [ConfigCompGpu::default(); 160];

    // Idénticos -> None
    assert_eq!(rango_filas_modificadas(&prev, &act), None);

    // Cambio en línea 45
    act[45].dispcnt = 0x0400;
    assert_eq!(rango_filas_modificadas(&prev, &act), Some((45, 45)));

    // Cambio en tramo 20..=80
    act[20].bldcnt = 0x0040;
    act[80].bldy = 8;
    assert_eq!(rango_filas_modificadas(&prev, &act), Some((20, 80)));
}

#[test]
fn test_dispcnt_consolida_bit_objwin() {
    let mut estado = EstadoPpuNativa::default();
    estado.dispcnt = 0;
    estado.num_sprites_ventana = 1;
    let mut comps = [ConfigCompGpu::default(); 160];
    comps[50].dispcnt = 0x8000;

    estado.consolidar_dispcnt_sprites(&mut comps);
    assert_eq!(
        estado.dispcnt & 0x8000,
        0x8000,
        "DISPCNT debe consolidar el bit 15 (OBJWIN) cuando hay sprites de ventana y líneas con 0x8000"
    );

    // Si num_sprites_ventana == 0, 0x8000 se apaga
    estado.num_sprites_ventana = 0;
    estado.consolidar_dispcnt_sprites(&mut comps);
    assert_eq!(
        estado.dispcnt & 0x8000,
        0,
        "DISPCNT debe apagar 0x8000 si no hay sprites de ventana"
    );
}

#[test]
fn test_propagacion_fondos_consolidados_a_staging_buffers() {
    let mut estado = EstadoPpuNativa::default();
    estado.capas[0].priority = 1;

    let mut staging_capas = [ConfigCapaGpu::default(); 160];
    let mut staging_comps = [ConfigCompGpu::default(); 160];
    for py in 0..160 {
        staging_capas[py].capas[0].enabled = 0;
        staging_comps[py].bg_prio0 = 0xFF;
    }
    staging_comps[42].dispcnt = 0x0100;

    estado.consolidar_dispcnt_fondos(&staging_comps);
    assert_eq!(estado.dispcnt & 0x0100, 0x0100);

    estado.propagar_fondos_a_staging(&mut staging_capas, &mut staging_comps);

    for py in 0..160 {
        assert_eq!(
            staging_capas[py].capas[0].enabled, 1,
            "Capa 0 debe propagarse habilitada a todas las líneas de staging"
        );
        assert_eq!(
            staging_comps[py].bg_prio0, 1,
            "bg_prio0 debe propagar la prioridad del fondo en staging si estaba en 0xFF"
        );
    }
}

#[test]
fn test_sprites_ventana_independientes_de_sprites_normales() {
    let mut estado = EstadoPpuNativa::default();
    estado.dispcnt = 0x1000; // Habilita sprites (bit 12)

    // Sprite 0: normal (modo_obj = 0), Sprite 1: ventana OBJ (modo_obj = 2 -> 2 << 10)
    let mut oam = oam_con(&[(0, 10, 20, 0), (1, (2 << 10) | 30, 40, 0)]);

    estado.decode_oam(&oam);
    assert_eq!(estado.num_sprites, 1);
    assert_eq!(estado.num_sprites_ventana, 1);
    let ventana_sprite_cuadro1 = estado.sprites_ventana[0];
    assert_eq!(estado.sprites[0].flags & FLAG_SEMI, 0);

    // Cuadro 2: Sprite 0 conmuta a semitransparente (modo_obj = 1 -> 1 << 10)
    let attr0_semi: u16 = (1 << 10) | 10;
    oam[0..2].copy_from_slice(&attr0_semi.to_le_bytes());
    estado.decode_oam(&oam);
    assert_eq!(estado.num_sprites, 1);
    assert_eq!(estado.num_sprites_ventana, 1);
    assert_eq!(
        estado.sprites_ventana[0], ventana_sprite_cuadro1,
        "Los sprites de ventana no deben alterarse al conmutar modo en sprites normales"
    );
    assert_ne!(estado.sprites[0].flags & FLAG_SEMI, 0);
}

// ===========================================================================
// Pruebas de sincronización Halt/IRQ y consolidación de prioridades (Propuesta 2026-10-30)
// ===========================================================================

#[test]
fn rt_interp_one_respeta_halt_e_irq_pendiente_antes_de_ejecutar() {
    let mut m = gba_core::machine::Machine::new(vec![0u8; 256]);
    m.bus.iwram[0x1B4..0x1B8].copy_from_slice(&0xE12FFF1Eu32.to_le_bytes());
    m.cpu.cpsr = 0x1F;
    m.cpu.regs[14] = 0x08000200;
    m.cpu.regs[15] = 0x030001B4;

    (gba_core::capi::RT_API.write8)(&mut m as *mut _ as *mut core::ffi::c_void, 0x04000301, 0);
    let cycles_before = m.bus.clock;
    (gba_core::capi::RT_API.tick)(&mut m as *mut _ as *mut core::ffi::c_void, 3);
    let next_key = gba_core::capi::rt_interp_one(&mut m as *mut _ as *mut core::ffi::c_void);

    assert_eq!(next_key, 0x030001B4);
    assert_eq!(m.cpu.regs[15], 0x030001B4);
    assert!(m.bus.halted);
    assert_eq!(m.bus.clock, cycles_before);
}

#[test]
fn consolidar_fondos_recupera_prioridad_activa_cuando_io_fin_la_altera() {
    let mut io_lineas = [[0u8; 0x60]; 160];
    for py in 0..100 {
        io_lineas[py][0x00] = 0x00;
        io_lineas[py][0x01] = 0x9F; // DISPCNT = 0x9F00
        io_lineas[py][0x0E] = 0x03; // BG3CNT lo: prioridad 3
        io_lineas[py][0x0F] = 0x1F; // BG3CNT hi
    }
    for py in 100..160 {
        io_lineas[py][0x00] = 0x00;
        io_lineas[py][0x01] = 0x97; // DISPCNT = 0x9700 (BG3 apagado)
        io_lineas[py][0x0E] = 0x00; // BG3CNT = 0x0000
        io_lineas[py][0x0F] = 0x00;
    }

    let mut io_fin = [0u8; 0x60];
    io_fin[0x00] = 0x00;
    io_fin[0x01] = 0x97; // DISPCNT = 0x9700 (BG3 apagado)
    io_fin[0x0E] = 0x00; // BG3CNT = 0x0000 (prioridad 0 en io_fin)
    io_fin[0x0F] = 0x00;

    let (mut capas, mut comps) = EstadoPpuNativa::decodificar_lineas(&io_lineas);
    let mut estado = EstadoPpuNativa::new();
    estado.decode_io_regs(&io_fin);
    estado.consolidar_dispcnt_fondos(&comps);
    estado.propagar_fondos_a_staging(&mut capas, &mut comps);

    assert_eq!(estado.bg_prio[3], 3);
    assert_eq!(comps[50].bg_prio3, 3);
    assert_eq!(comps[120].bg_prio3, 3);
}

#[test]
fn consolidar_dispcnt_sprites_propaga_objwin_a_staging_filas_comp() {
    let mut estado = EstadoPpuNativa::default();
    estado.num_sprites_ventana = 1;

    let mut staging_filas_comp = [ConfigCompGpu::default(); 160];
    staging_filas_comp[0].dispcnt = 0x1F00;
    staging_filas_comp[10].dispcnt = 0x1F00 | 0x8000;

    estado.consolidar_dispcnt_sprites(&mut staging_filas_comp);

    assert_eq!(
        estado.dispcnt & 0x8000,
        0x8000,
        "self.dispcnt debe reflejar 0x8000 cuando hay sprites de ventana en OAM y líneas con 0x8000"
    );
    assert_eq!(
        staging_filas_comp[0].dispcnt & 0x8000,
        0,
        "No debe forzar 0x8000 en líneas que tenían OBJWIN apagado en DISPCNT"
    );
    assert_eq!(
        staging_filas_comp[10].dispcnt & 0x8000,
        0x8000,
        "Debe conservar 0x8000 en líneas que lo tenían activo cuando hay sprites de ventana"
    );

    // Cuando num_sprites_ventana == 0, purga 0x8000 de todas las filas
    estado.num_sprites_ventana = 0;
    estado.consolidar_dispcnt_sprites(&mut staging_filas_comp);
    assert_eq!(
        estado.dispcnt & 0x8000,
        0,
        "self.dispcnt debe purgar 0x8000 si num_sprites_ventana == 0"
    );
    assert_eq!(
        staging_filas_comp[10].dispcnt & 0x8000,
        0,
        "staging_filas_comp debe purgar 0x8000 de todas las filas cuando num_sprites_ventana == 0"
    );
}

#[test]
fn test_staging_lineas_aislado_de_estado_residual() {
    let mut io_lineas = [[0u8; 0x60]; 160];
    // Scanline 50: DISPCNT = 0x1F00 (Modo 0, BG0-3 y OBJ activos, bit 15 OBJWIN = 0)
    // BG3CNT = 0x1F03 (prioridad 3)
    // BLDCNT = 0x2844 (Modo 1 Alpha, 1er target BG2, 2do target BG3)
    io_lineas[50][0x00] = 0x00;
    io_lineas[50][0x01] = 0x1F;
    io_lineas[50][0x0E] = 0x03;
    io_lineas[50][0x0F] = 0x1F;
    io_lineas[50][0x50] = 0x44;
    io_lineas[50][0x51] = 0x28;

    // Estado residual al final del cuadro:
    // DISPCNT = 0x9F00 (bit 15 forzado: OBJWIN activo)
    // BLDCNT = 0x0144 (blend residual de UI)
    let mut io_fin = [0u8; 0x60];
    io_fin[0x00] = 0x00;
    io_fin[0x01] = 0x9F;
    io_fin[0x50] = 0x44;
    io_fin[0x51] = 0x01;

    let mut estado = EstadoPpuNativa::new();
    estado.decode_io_regs(&io_fin);
    estado.num_sprites_ventana = 1;

    let mut staging_capas = [ConfigCapaGpu::default(); 160];
    let mut staging_comps = [ConfigCompGpu::default(); 160];

    estado.preparar_staging_filas(Some(&io_lineas), &mut staging_capas, &mut staging_comps);

    // Scanline 50 debe preservar intactos los valores de io_lineas sin ser contaminados por estado/io_fin.
    assert_eq!(staging_comps[50].bldcnt, 0x2844);
    assert_eq!(
        staging_comps[50].dispcnt & 0x8000,
        0,
        "OBJWIN no debe propagarse a staging cuando io_lineas está presente"
    );
    assert_eq!(staging_comps[50].dispcnt, 0x1F00);
    assert_eq!(staging_capas[50].capas[3].enabled, 1);
    assert_eq!(staging_comps[50].bg_prio3, 3);
}

#[test]
fn test_staging_fallback_cuando_io_lineas_es_none() {
    let mut io_fin = [0u8; 0x60];
    // DISPCNT = 0x1F00 (Modo 0, BG0-3 y OBJ activos)
    io_fin[0x00] = 0x00;
    io_fin[0x01] = 0x1F;
    // BG2CNT = prioridad 2
    io_fin[0x0C] = 0x02;
    // BLDCNT = 0x0144
    io_fin[0x50] = 0x44;
    io_fin[0x51] = 0x01;

    let mut estado = EstadoPpuNativa::new();
    estado.decode_io_regs(&io_fin);

    let mut staging_capas = [ConfigCapaGpu::default(); 160];
    let mut staging_comps = [ConfigCompGpu::default(); 160];

    estado.preparar_staging_filas(None, &mut staging_capas, &mut staging_comps);

    // Fallback: todas las 160 scanlines se pueblan con la configuración consolidada de self.estado
    for py in 0..160 {
        assert_eq!(staging_comps[py].bldcnt, 0x0144);
        assert_eq!(staging_comps[py].dispcnt, 0x1F00);
        assert_eq!(staging_capas[py].capas[2].enabled, 1);
        assert_eq!(staging_comps[py].bg_prio2, 2);
    }
}

#[test]
fn test_sanear_bldcnt_coexistencia_fuego_y_texto_160_scanlines() {
    let mut comps = [ConfigCompGpu::default(); 160];
    // Escena del dragón: DISPCNT = 0x0D00 (Modo 0, BG0 texto, BG2 dragón, BG3 fuego)
    for py in 0..160 {
        comps[py].dispcnt = 0x0D00;
        comps[py].bg_prio3 = 3;
        comps[py].bg_prio2 = 3;
    }
    // Scanlines 0..49: Fuego BG3 sobre BD en modo alfa (0x3F68) con alpha atenuado
    for py in 0..50 {
        comps[py].bldcnt = 0x3F68;
        comps[py].bldalpha = 0x0408;
    }
    // Scanlines 50..99: Variante 0x2869 (BG3 + BG0 sobre BD en modo alfa) con alpha atenuado
    for py in 50..100 {
        comps[py].bldcnt = 0x2869;
        comps[py].bldalpha = 0x0408;
    }
    // Scanlines 100..159: Coexistencia 0x3F69 (BG3 + BG0 sobre BD en modo alfa) con alpha atenuado
    for py in 100..160 {
        comps[py].bldcnt = 0x3F69;
        comps[py].bldalpha = 0x0408;
    }

    EstadoPpuNativa::sanear_bldcnt_cinematica_dragon(&mut comps);

    // Scanlines 0..49: BLDCNT preservado y BLDALPHA restaurado a nominal 0x0810
    for py in 0..50 {
        assert_eq!(
            comps[py].bldcnt, 0x3F68,
            "Scanline {} en cielo/fuego debe preservar 0x3F68",
            py
        );
        assert_eq!(
            comps[py].bldalpha, 0x0810,
            "Scanline {} debe restaurar BLDALPHA = 0x0810",
            py
        );
    }

    // Scanlines 50..99: variante 0x2869 preserva BLDCNT y restaura BLDALPHA
    for py in 50..100 {
        assert_eq!(
            comps[py].bldcnt, 0x2869,
            "Scanline {} con 0x2869 debe preservar 0x2869",
            py
        );
        assert_eq!(
            comps[py].bldalpha, 0x0810,
            "Scanline {} debe tener BLDALPHA = 0x0810",
            py
        );
    }

    // Scanlines 100..159: texto y fuego 0x3F69 preserva BLDCNT y restaura BLDALPHA
    for py in 100..160 {
        assert_eq!(
            comps[py].bldcnt, 0x3F69,
            "Scanline {} en franja de subtítulos debe preservar 0x3F69",
            py
        );
        assert_eq!(
            comps[py].bldalpha, 0x0810,
            "Scanline {} en franja de subtítulos debe tener BLDALPHA = 0x0810",
            py
        );
    }

    // Coexistencia garantizada en las 160 scanlines completas
    for py in 0..160 {
        assert_ne!(
            comps[py].bldcnt & 0x0008,
            0,
            "Scanline {} debe tener BG3 como primer objetivo",
            py
        );
        assert_ne!(
            comps[py].bldcnt & 0x2000,
            0,
            "Scanline {} debe tener Backdrop como segundo objetivo",
            py
        );
        assert_eq!(
            (comps[py].bldcnt >> 6) & 3,
            1,
            "Scanline {} debe estar en modo de mezcla alfa",
            py
        );
        assert_eq!(
            comps[py].bldalpha, 0x0810,
            "Scanline {} debe tener coeficientes EVA=16, EVB=8",
            py
        );
    }
}

#[test]
fn test_sanear_bldcnt_restaura_variante_dialogo_2869() {
    let mut comps = [ConfigCompGpu::default(); 160];
    for py in 0..160 {
        comps[py].dispcnt = 0x0D00;
        comps[py].bg_prio2 = 3;
        comps[py].bg_prio3 = 3;
        comps[py].bldcnt = 0x2869;
        comps[py].bldalpha = 0x0408;
    }

    EstadoPpuNativa::sanear_bldcnt_cinematica_dragon(&mut comps);

    for py in 0..160 {
        assert_eq!(
            comps[py].bldcnt, 0x2869,
            "Scanline {} con variante 0x2869 debe preservar BLDCNT = 0x2869",
            py
        );
        assert_eq!(
            comps[py].bldalpha, 0x0810,
            "Scanline {} debe restaurarse a BLDALPHA = 0x0810",
            py
        );
    }
}

#[test]
fn test_sanear_bldcnt_restaura_bldalpha_desacoplado() {
    let mut comps = [ConfigCompGpu::default(); 160];
    for py in 0..160 {
        comps[py].dispcnt = 0x0D00;
        comps[py].bg_prio2 = 3;
        comps[py].bg_prio3 = 3;
        comps[py].bldcnt = 0x3F68;
        comps[py].bldalpha = 0x0408;
    }

    EstadoPpuNativa::sanear_bldcnt_cinematica_dragon(&mut comps);

    for py in 0..160 {
        assert_eq!(
            comps[py].bldcnt, 0x3F68,
            "Scanline {} con BLDCNT = 0x3F68 debe preservarse intacto",
            py
        );
        assert_eq!(
            comps[py].bldalpha, 0x0810,
            "Scanline {} con BLDALPHA atenuado debe restaurarse a 0x0810",
            py
        );
    }
}

#[test]
fn test_sanear_bldcnt_permite_sprites_obj_y_cursor() {
    let mut comps = [ConfigCompGpu::default(); 160];
    // Escena del dragón con sprites activos y diálogo: DISPCNT = 0x1D00 (Modo 0, BG0, BG2, BG3, OBJ)
    for py in 0..160 {
        comps[py].dispcnt = 0x1D00;
        comps[py].bg_prio2 = 3;
        comps[py].bg_prio3 = 3;
        comps[py].bldcnt = 0x3F69;
        comps[py].bldalpha = 0x0408;
    }

    EstadoPpuNativa::sanear_bldcnt_cinematica_dragon(&mut comps);

    for py in 0..160 {
        assert_eq!(
            comps[py].bldcnt, 0x3F69,
            "Scanline {} con sprites y diálogo (0x1D00) debe preservar BLDCNT = 0x3F69",
            py
        );
        assert_eq!(
            comps[py].bldalpha, 0x0810,
            "Scanline {} con sprites activos debe tener BLDALPHA = 0x0810",
            py
        );
    }
}

#[test]
fn test_sanear_bldcnt_permite_sprites_aislados() {
    let mut comps = [ConfigCompGpu::default(); 160];
    // Escena del dragón con sprites aislados sin texto: DISPCNT = 0x1C00 (Modo 0, BG2, BG3, OBJ)
    for py in 0..160 {
        comps[py].dispcnt = 0x1C00;
        comps[py].bg_prio2 = 3;
        comps[py].bg_prio3 = 3;
        comps[py].bldcnt = 0x3F68;
        comps[py].bldalpha = 0x0408;
    }

    EstadoPpuNativa::sanear_bldcnt_cinematica_dragon(&mut comps);

    for py in 0..160 {
        assert_eq!(
            comps[py].bldcnt, 0x3F68,
            "Scanline {} con sprites aislados (0x1C00) debe preservar BLDCNT = 0x3F68",
            py
        );
        assert_eq!(
            comps[py].bldalpha, 0x0810,
            "Scanline {} con sprites aislados debe tener BLDALPHA = 0x0810",
            py
        );
    }
}

#[test]
fn test_sanear_bldcnt_no_altera_otras_escenas() {
    let mut comps = [ConfigCompGpu::default(); 160];
    // Caso 1: dispcnt solo BG0 habilitado (0x0100)
    for py in 0..26 {
        comps[py].dispcnt = 0x0100;
        comps[py].bldcnt = 0x2861;
        comps[py].bldalpha = 0x0408;
    }
    // Caso 2: dispcnt sin BG3 habilitado (0x0400, solo BG2)
    for py in 26..52 {
        comps[py].dispcnt = 0x0400;
        comps[py].bldcnt = 0x2861;
        comps[py].bldalpha = 0x0408;
    }
    // Caso 3: dispcnt sin BG2 habilitado (0x0800, solo BG3)
    for py in 52..78 {
        comps[py].dispcnt = 0x0800;
        comps[py].bldcnt = 0x2861;
        comps[py].bldalpha = 0x0408;
    }
    // Caso 4: dispcnt en Modo 1 en vez de Modo 0 (0x0C01)
    for py in 78..104 {
        comps[py].dispcnt = 0x0C01;
        comps[py].bldcnt = 0x2861;
        comps[py].bldalpha = 0x0408;
    }
    // Caso 5: dispcnt con BG1 habilitado (0x1E00: Modo 0, BG1, BG2, BG3 y OBJ activos, mapa)
    for py in 104..130 {
        comps[py].dispcnt = 0x1E00;
        comps[py].bldcnt = 0x2861;
        comps[py].bldalpha = 0x0408;
    }
    // Caso 6: dispcnt con firma del dragón (0x0C00) pero bldcnt == 0 (ilustraciones estáticas del prólogo)
    for py in 130..160 {
        comps[py].dispcnt = 0x0C00;
        comps[py].bldcnt = 0x0000;
        comps[py].bldalpha = 0x0000;
    }

    let copia_inicial = comps;
    EstadoPpuNativa::sanear_bldcnt_cinematica_dragon(&mut comps);

    assert_eq!(
        comps, copia_inicial,
        "Ninguna scanline fuera de las condiciones de la escena del dragón debe ser alterada"
    );
}

#[test]
fn test_preparar_staging_filas_sanea_en_ramal_none() {
    let mut estado = EstadoPpuNativa::new();
    estado.dispcnt = 0x0D00;
    estado.bg_prio[2] = 3;
    estado.bg_prio[3] = 3;
    estado.color.bldcnt = 0x3F68; // Fuego BG3 sobre Backdrop en modo alfa
    estado.color.bldalpha = 0x0408;

    let mut staging_capas = [ConfigCapaGpu::default(); 160];
    let mut staging_comps = [ConfigCompGpu::default(); 160];

    estado.preparar_staging_filas(None, &mut staging_capas, &mut staging_comps);

    for py in 0..160 {
        assert_eq!(
            staging_comps[py].bldcnt, 0x3F68,
            "Scanline {} en ramal None debe preservar BLDCNT = 0x3F68",
            py
        );
        assert_eq!(
            staging_comps[py].bldalpha, 0x0810,
            "Scanline {} en ramal None debe tener BLDALPHA saneado a 0x0810",
            py
        );
    }
}

#[test]
fn test_composicion_multinivel_coexistencia_4_capas() {
    // Escenario de la cinemática del dragón:
    // Píxel donde coinciden simultáneamente Backdrop, BG3 (fuego), BG2 (transparente en los huecos del dragón),
    // BG0 (texto del diálogo) y OBJ (cursor/sprite).
    let backdrop = 0x0000u16; // Fondo negro
    let c3_fuego = 0x001Fu16; // Rojo fuego puro (31, 0, 0)
    let c2_dragon_transparente = TRANSPARENTE;
    let c0_texto = 0x7FFFu16; // Blanco texto puro (31, 31, 31)
    let obj_cursor = 0x03E0u16; // Verde cursor (0, 31, 0)

    let control = 0x3Fu16; // Efectos activos y todas las capas habilitadas
    let bldcnt_dialogo = 0x3F61u16; // Modo 1 (alfa en BG0), 1st target BG0
    let eva = 10u32;
    let evb = 6u32;

    // 1. Caso sin sprite OBJ: el texto blendea sobre el fuego (sin que el fuego sea descartado)
    let color_sin_obj = compone_pixel_jerarquica_dragon(
        c0_texto,
        TRANSPARENTE,
        c2_dragon_transparente,
        c3_fuego,
        obj_cursor,
        false, // obj_visible = false
        false, // obj_semi = false
        control,
        bldcnt_dialogo,
        eva,
        evb,
        0, // evy
        backdrop,
    );

    // El fuego se mezcla con backdrop: con eva_fuego=10, evb_fuego=6 sobre backdrop negro (0x0000):
    // alpha_blend(0x001F, 0x0000, 10, 6) -> r = (31 * 10 + 8) >> 4 = 19
    let fuego_fondo = alpha_blend(c3_fuego, backdrop, eva, evb);
    let esperado_texto_sobre_fuego = alpha_blend(c0_texto, fuego_fondo, eva, evb);
    assert_eq!(
        color_sin_obj, esperado_texto_sobre_fuego,
        "El texto debe mezclarse suavemente con el fuego acumulado de fondo"
    );
    assert_ne!(
        color_sin_obj, 0x0000,
        "El fuego de fondo no debe caer a negro al entrar el texto"
    );

    // 2. Coexistencia de las 4 capas simultáneas cuando entra el cursor OBJ (semitransparente o encima):
    let color_con_obj = compone_pixel_jerarquica_dragon(
        c0_texto,
        TRANSPARENTE,
        c2_dragon_transparente,
        c3_fuego,
        obj_cursor,
        true, // obj_visible = true
        true, // obj_semi = true
        control,
        bldcnt_dialogo,
        eva,
        evb,
        0, // evy
        backdrop,
    );

    let esperado_4_capas = alpha_blend(obj_cursor, esperado_texto_sobre_fuego, eva, evb);
    assert_eq!(
        color_con_obj, esperado_4_capas,
        "Las 4 capas (Backdrop + BG3 fuego + BG0 texto + OBJ cursor) deben coexistir armónicamente"
    );

    // 3. Demostración de la ventaja sobre el bucle tradicional de 2 capas:
    // En el bucle de 2 capas tradicional, con OBJ activo (prio 0), BG0 texto (prio 0), BG2 (prio 2) y BG3 (prio 3):
    // El bucle tradicional se cortaría tras las 2 primeras capas y jamás leería BG3 (fuego).
    let bg_tradicional = [c0_texto, TRANSPARENTE, c2_dragon_transparente, c3_fuego];
    let bg_prio = [0, 0xFF, 2, 3];
    let obj_pixel = PixelObj {
        color: obj_cursor,
        prioridad: 0,
        semi: true,
    };
    let reg_tradicional = RegistrosColor {
        bldcnt: bldcnt_dialogo,
        bldalpha: (evb as u16) << 8 | (eva as u16),
        bldy: 0,
    };
    let color_bucle_2_capas = compone_pixel(
        &bg_tradicional,
        &bg_prio,
        obj_pixel,
        control,
        &reg_tradicional,
        backdrop,
    );

    // En el bucle tradicional, la 2ª capa fue BG0 (texto) y se cortó inmediatamente, truncando BG3 (fuego).
    // Su resultado mezcla OBJ únicamente con BG0, ignorando por completo el fuego de BG3.
    let truncado_sin_fuego = alpha_blend(obj_cursor, c0_texto, eva, evb);
    assert_eq!(
        color_bucle_2_capas, truncado_sin_fuego,
        "El bucle tradicional de 2 capas sufre corte prematuro descartando BG3"
    );
    assert_ne!(
        color_con_obj, color_bucle_2_capas,
        "La composición jerárquica bottom-up preserva el fuego donde el bucle de 2 capas lo perdía"
    );
}

// ===========================================================================
// Consolidación incondicional de sprites y transición de cinemática
// ===========================================================================

#[test]
fn test_consolidacion_incondicional_sprites_con_io_lineas() {
    let mut estado = EstadoPpuNativa::new();
    // Simulamos que el estado de fin de cuadro (io_fin / VBlank) tiene el bit 12 apagado:
    estado.dispcnt = 0x0C00;

    // Las scanlines visibles en staging_filas_comp tienen OBJ activo (bit 12) y mapeo 1D (bit 6):
    let mut staging_comps = [ConfigCompGpu::default(); 160];
    for comp in staging_comps.iter_mut() {
        comp.dispcnt = 0x1C40;
    }

    // La consolidación incondicional debe encender el bit 12 (y bit 6 de 1D) en estado.dispcnt:
    estado.consolidar_dispcnt_sprites(&mut staging_comps);
    assert_ne!(
        estado.dispcnt & 0x1000,
        0,
        "La consolidación debe rescatar el bit 12 de sprites activo en scanlines"
    );
    assert_ne!(
        estado.dispcnt & 0x40,
        0,
        "La consolidación debe rescatar el mapeo 1D activo en scanlines"
    );

    // Al decodificar OAM con un sprite visible, num_sprites no se vacía a 0:
    let oam = oam_de_una_entrada(10, 20, 0);
    let count = estado.decode_oam(&oam);
    assert_eq!(count, 1);
    assert_eq!(estado.num_sprites, 1);
}

#[test]
fn test_deteccion_escena_dragon_por_prioridades_con_bldcnt_cero() {
    let mut comps = [ConfigCompGpu::default(); 160];

    // Scanlines 0..80: Fondos con bldcnt == 0 y prioridades bg_prio2 == 3, bg_prio3 == 3
    for py in 0..80 {
        comps[py].dispcnt = 0x0C00;
        comps[py].bldcnt = 0;
        comps[py].bg_prio2 = 3;
        comps[py].bg_prio3 = 3;
    }

    // Scanlines 80..160: Pinturas fijas del prólogo
    for py in 80..160 {
        comps[py].dispcnt = 0x0C00;
        comps[py].bldcnt = 0;
        comps[py].bg_prio2 = 0;
        comps[py].bg_prio3 = 1;
    }

    let copia = comps;
    EstadoPpuNativa::sanear_bldcnt_cinematica_dragon(&mut comps);

    // Con bldcnt == 0, ninguna scanline debe ser modificada (no hay mezcla alfa de fuego activa)
    assert_eq!(
        comps, copia,
        "sanear_bldcnt no debe mutar ninguna línea con bldcnt == 0, preservando el pipeline estándar"
    );
}

#[test]
fn test_consolidar_dispcnt_purga_objwin_sin_sprites() {
    let mut estado = EstadoPpuNativa::new();
    let mut comps = [ConfigCompGpu::default(); 160];

    // Caso 1: Líneas traen 0x8000 pero num_sprites_ventana == 0.
    // Todas las líneas de comps deben ser purgadas de 0x8000 para que el sombreador WGSL
    // no aplique recortes fantasma que apaguen capas hacia negro.
    comps[10].dispcnt = 0x9F00;
    comps[20].dispcnt = 0x8F00;
    estado.num_sprites_ventana = 0;

    estado.consolidar_dispcnt_sprites(&mut comps);

    assert!(
        comps.iter().all(|c| c.dispcnt & 0x8000 == 0),
        "Todas las líneas de staging_comps deben purgar 0x8000 si num_sprites_ventana == 0"
    );

    // Caso 2: Sin líneas con 0x8000 y sin sprites de ventana: estado.dispcnt se limpia.
    estado.dispcnt = 0x8000; // residuo previo
    comps.fill(ConfigCompGpu::default());
    estado.num_sprites_ventana = 0;

    estado.consolidar_dispcnt_sprites(&mut comps);

    assert_eq!(
        estado.dispcnt & 0x8000,
        0,
        "self.dispcnt debe purgar 0x8000 residual si no hay líneas ni sprites con OBJWIN"
    );

    // Caso 3: Línea con 0x8000 Y sprites de ventana presentes: se conserva en la línea y en estado.dispcnt
    comps[15].dispcnt = 0x9F00;
    estado.num_sprites_ventana = 1;

    estado.consolidar_dispcnt_sprites(&mut comps);

    assert_eq!(
        estado.dispcnt & 0x8000,
        0x8000,
        "self.dispcnt debe tener 0x8000 si hay sprites de ventana y al menos una línea con 0x8000"
    );
    assert_eq!(
        comps[15].dispcnt & 0x8000,
        0x8000,
        "La línea 15 debe conservar 0x8000 si num_sprites_ventana > 0"
    );
}

#[test]
fn test_dragon_fades_brighten_darken() {
    let c0 = 0x7FFF; // Blanco (31, 31, 31)
    let c2 = 0x0000;
    let c3 = 0x001F; // Rojo (31, 0, 0)
    let backdrop = 0x0000;
    let control = 0x3F; // Efectos activos (bit 5) y capas habilitadas
    let eva = 16;
    let evb = 0;
    let evy = 8;

    // Caso base sin efectos de fade (modo 0):
    let color_base = compone_pixel_jerarquica_dragon(
        c0,
        TRANSPARENTE,
        c2,
        c3,
        0,
        false,
        false,
        control,
        0x0000,
        eva,
        evb,
        0,
        backdrop,
    );

    // Caso modo 2 (Brighten):
    let bldcnt_brighten = 0x0080; // (bldcnt >> 6) & 3 == 2
    let color_brighten = compone_pixel_jerarquica_dragon(
        c0,
        TRANSPARENTE,
        c2,
        c3,
        0,
        false,
        false,
        control,
        bldcnt_brighten,
        eva,
        evb,
        evy,
        backdrop,
    );
    let esperado_brighten = brighten(color_base, evy);
    assert_eq!(
        color_brighten, esperado_brighten,
        "Modo 2 debe aplicar brighten(color_acc, evy)"
    );

    // Caso modo 3 (Darken):
    let bldcnt_darken = 0x00C0; // (bldcnt >> 6) & 3 == 3
    let color_darken = compone_pixel_jerarquica_dragon(
        c0,
        TRANSPARENTE,
        c2,
        c3,
        0,
        false,
        false,
        control,
        bldcnt_darken,
        eva,
        evb,
        evy,
        backdrop,
    );
    let esperado_darken = darken(color_base, evy);
    assert_eq!(
        color_darken, esperado_darken,
        "Modo 3 debe aplicar darken(color_acc, evy)"
    );

    // Caso efectos inactivos en control (control & 0x20 == 0):
    let control_sin_efectos = control & !0x20;
    let color_sin_efecto = compone_pixel_jerarquica_dragon(
        c0,
        TRANSPARENTE,
        c2,
        c3,
        0,
        false,
        false,
        control_sin_efectos,
        bldcnt_brighten,
        eva,
        evb,
        evy,
        backdrop,
    );
    assert_eq!(
        color_sin_efecto, color_base,
        "Si efectos está apagado en control, no debe aplicar brighten ni darken"
    );
}

#[test]
fn test_deteccion_escena_dragon_con_bg1_activo() {
    let mut comps = [ConfigCompGpu::default(); 160];
    // Escenario de telemetría real: DISPCNT con BG1, BG2, BG3 encendidos (0x0E00) y mezcla de fuego activa (0x3F69)
    for py in 0..160 {
        comps[py].dispcnt = 0x0E00; // Modo 0, BG1, BG2, BG3 encendidos
        comps[py].bg_prio2 = 3;
        comps[py].bg_prio3 = 3;
        comps[py].bldcnt = 0x3F69;
        comps[py].bldalpha = 0x0408;
    }

    EstadoPpuNativa::sanear_bldcnt_cinematica_dragon(&mut comps);

    for py in 0..160 {
        assert_eq!(
            comps[py].bldcnt, 0x3F69,
            "Scanline {py} con BG1 activo debe detectarse como escena de dragón y preservar 0x3F69"
        );
        assert_eq!(
            comps[py].bldalpha, 0x0810,
            "Scanline {py} debe tener BLDALPHA nominal 0x0810"
        );
    }
}

#[test]
fn test_sanear_bldcnt_respeta_fondos_ordinarios_prio3() {
    let mut comps = [ConfigCompGpu::default(); 160];
    for py in 0..160 {
        // Modo 0, BG2 y BG3 activos, prioridades en 3 (fondos ordinarios lejanos)
        comps[py].dispcnt = 0x0C00;
        comps[py].bg_prio2 = 3;
        comps[py].bg_prio3 = 3;
    }
    // Caso A (0..40): Mezcla desactivada (bldcnt = 0, bldalpha = 0)
    for py in 0..40 {
        comps[py].bldcnt = 0x0000;
        comps[py].bldalpha = 0x0000;
    }
    // Caso B (40..80): Mezcla normal modo 0 (0x3F28, no es modo alfa)
    for py in 40..80 {
        comps[py].bldcnt = 0x3F28;
        comps[py].bldalpha = 0x0408;
    }
    // Caso C (80..120): Diálogo estándar sin BG3 (0x2861, bit 3 apagado)
    for py in 80..120 {
        comps[py].bldcnt = 0x2861;
        comps[py].bldalpha = 0x0408;
    }
    // Caso D (120..160): Diálogo estándar BG0 (0x3F61, bit 3 apagado)
    for py in 120..160 {
        comps[py].bldcnt = 0x3F61;
        comps[py].bldalpha = 0x0408;
    }

    let copia = comps;
    EstadoPpuNativa::sanear_bldcnt_cinematica_dragon(&mut comps);

    assert_eq!(
        comps, copia,
        "sanear_bldcnt no debe alterar bldcnt ni bldalpha en fotogramas ordinarios con prio3"
    );
}

#[test]
fn test_staging_lineas_preserva_dragon_e_inmune_a_intro() {
    let mut estado = EstadoPpuNativa::new();
    let mut staging_capas = [ConfigCapaGpu::default(); 160];
    let mut staging_comps = [ConfigCompGpu::default(); 160];

    // Caso 1: Cinemática del dragón (DISPCNT = 0x0D00, BLDCNT = 0x3F68, BLDALPHA = 0x0410).
    let mut lineas_dragon = [[0u8; 0x60]; 160];
    for py in 0..160 {
        lineas_dragon[py][0x00] = 0x00;
        lineas_dragon[py][0x01] = 0x0D;
        lineas_dragon[py][0x50] = 0x68;
        lineas_dragon[py][0x51] = 0x3F;
        lineas_dragon[py][0x52] = 0x10;
        lineas_dragon[py][0x53] = 0x04;
    }
    estado.preparar_staging_filas(Some(&lineas_dragon), &mut staging_capas, &mut staging_comps);
    for py in 0..160 {
        assert_eq!(staging_comps[py].bldcnt, 0x3F68);
        assert_eq!(
            staging_comps[py].bldalpha, 0x0410,
            "io_lineas debe preservar bldalpha auténtico del juego"
        );
    }

    // Caso 2: Intro (DISPCNT = 0x1F00, BLDCNT = 0x3F68, BLDALPHA = 0x0610).
    let mut lineas_intro = [[0u8; 0x60]; 160];
    for py in 0..160 {
        lineas_intro[py][0x00] = 0x00;
        lineas_intro[py][0x01] = 0x1F;
        lineas_intro[py][0x50] = 0x68;
        lineas_intro[py][0x51] = 0x3F;
        lineas_intro[py][0x52] = 0x10;
        lineas_intro[py][0x53] = 0x06;
    }
    estado.preparar_staging_filas(Some(&lineas_intro), &mut staging_capas, &mut staging_comps);
    for py in 0..160 {
        assert_eq!(staging_comps[py].bldcnt, 0x3F68);
        assert_eq!(staging_comps[py].bldalpha, 0x0610);
    }
}

#[test]
fn test_transicion_oam_con_sprites_ventana_a_cero_purga_objwin() {
    let mut estado = EstadoPpuNativa::new();
    estado.dispcnt = 0x1000;
    let mut oam = [0u8; 1024];
    let a0 = 20u16 | (2 << 10);
    oam[0] = (a0 & 0xFF) as u8;
    oam[1] = (a0 >> 8) as u8;
    oam[2] = 30;
    oam[3] = 0;
    let _ = estado.decode_oam(&oam);
    assert_eq!(estado.num_sprites_ventana, 1);

    let mut comps = [ConfigCompGpu::default(); 160];
    for py in 20..40 {
        comps[py].dispcnt = 0x1F00 | 0x8000;
    }
    estado.consolidar_dispcnt_sprites(&mut comps);
    assert_eq!(estado.dispcnt & 0x8000, 0x8000);

    let oam_vacia = [0u8; 1024];
    let _ = estado.decode_oam(&oam_vacia);
    assert_eq!(estado.num_sprites_ventana, 0);

    for py in 20..40 {
        comps[py].dispcnt = 0x1F00 | 0x8000;
    }
    estado.consolidar_dispcnt_sprites(&mut comps);
    assert_eq!(estado.dispcnt & 0x8000, 0);
    assert!(comps.iter().all(|c| c.dispcnt & 0x8000 == 0));
}

#[test]
fn test_deteccion_escena_dragon_estricta() {
    let es_dragon = |dispcnt: u32, bldcnt: u32| -> bool {
        (dispcnt & 7) == 0
            && (dispcnt & 0x0C00) == 0x0C00
            && (bldcnt & 0x2008) == 0x2008
            && ((bldcnt >> 6) & 3) == 1
            && (dispcnt & 0x0300) != 0x0300
    };

    // Caso 1: Cinemática dragón nominal (Modo 0, BG0, BG2, BG3 encendidos, mezcla fuego 0x3F68)
    assert!(es_dragon(0x0D00, 0x3F68));

    // Caso 2: Cinemática dragón nominal con solo BG2 y BG3 (0x0C00)
    assert!(es_dragon(0x0C00, 0x3F68));

    // Caso 2b: Escena intro o juego regular con BG0 y BG1 activos a la vez (0x0F00 o 0x1F00 con sprites)
    // No debe considerarse dragón para evitar secuestrar los coeficientes alfa de la intro
    assert!(!es_dragon(0x0F00, 0x3F68));
    assert!(!es_dragon(0x1F00, 0x3F69));
    assert!(!es_dragon(0x1F00, 0x3F68));

    // Caso 3: Cuadros ordinarios con prioridades prio2=3, prio3=3 pero BLDCNT apagado (bldcnt = 0)
    assert!(!es_dragon(0x0C00, 0x0000));

    // Caso 4: Cuadros ordinarios con diálogo BG0 sin mezcla BG3 (0x3F61 o 0x2861)
    assert!(!es_dragon(0x0D00, 0x3F61));
    assert!(!es_dragon(0x0D00, 0x2861));

    // Caso 5: Modo no alfa (bldcnt = 0x3F28, modo 0)
    assert!(!es_dragon(0x0D00, 0x3F28));

    // Caso 6: Modo 1 (no Modo 0)
    assert!(!es_dragon(0x0D01, 0x3F68));

    // Caso 7: Falta BG2 o BG3
    assert!(!es_dragon(0x0900, 0x3F68)); // Sin BG2
    assert!(!es_dragon(0x0500, 0x3F68)); // Sin BG3
}

#[test]
fn test_sanear_bldcnt_conserva_bldalpha_nativo_en_intro() {
    let mut comps = [ConfigCompGpu::default(); 160];
    for c in comps.iter_mut() {
        c.dispcnt = 0x1F00; // Intro: todas las capas activas (BG0+BG1+BG2+BG3+OBJ)
        c.bldcnt = 0x3F68;
        c.bldalpha = 0x0610; // Nominal en intro: EVA=16, EVB=6
    }
    EstadoPpuNativa::sanear_bldcnt_cinematica_dragon(&mut comps);
    assert_eq!(comps[0].bldalpha, 0x0610);
    assert_eq!(comps[159].bldalpha, 0x0610);
}

#[test]
fn test_sanear_bldcnt_sanea_en_cinematica_dragon() {
    let mut comps = [ConfigCompGpu::default(); 160];
    for c in comps.iter_mut() {
        c.dispcnt = 0x0D00; // Dragón: BG0, BG2, BG3
        c.bldcnt = 0x3F68;
        c.bldalpha = 0x0410; // Atenuado
    }
    EstadoPpuNativa::sanear_bldcnt_cinematica_dragon(&mut comps);
    assert_eq!(comps[0].bldalpha, 0x0810);
    assert_eq!(comps[159].bldalpha, 0x0810);
}

#[test]
fn test_composicion_jerarquica_dragon_cuatro_capas() {
    let backdrop = 0x0000u16; // Negro
    let c3_fuego = 0x001Fu16; // Rojo fuego (31, 0, 0)
    let c2_dragon = 0x03E0u16; // Verde dragón (0, 31, 0)
    let c1_humo = 0x7C00u16; // Azul humo (0, 0, 31)
    let c0_texto = 0x7FFFu16; // Blanco texto (31, 31, 31)
    let obj_cursor = 0x03FFu16; // Cyan cursor

    let control = 0x3Fu16; // Efectos y todas las capas habilitadas (BG0..BG3 y OBJ)
    let bldcnt_dialogo = 0x3F61u16; // Modo 1 (alfa en BG0), 1st target BG0
    let eva = 10u32;
    let evb = 6u32;

    let resultado = compone_pixel_jerarquica_dragon(
        c0_texto,
        c1_humo,
        c2_dragon,
        c3_fuego,
        obj_cursor,
        true, // obj_visible
        true, // obj_semi
        control,
        bldcnt_dialogo,
        eva,
        evb,
        0,
        backdrop,
    );

    // BG3 fuego se mezcla con backdrop:
    let _fuego_fondo = alpha_blend(c3_fuego, backdrop, eva, evb);
    // BG2 dragón sobre fuego (opaco):
    let _sobre_dragon = c2_dragon;
    // BG1 humo sobre dragón (opaco):
    let sobre_humo = c1_humo;
    // BG0 texto con blend sobre humo acumulado:
    let sobre_texto = alpha_blend(c0_texto, sobre_humo, eva, evb);
    // OBJ cursor con blend sobre texto:
    let esperado = alpha_blend(obj_cursor, sobre_texto, eva, evb);

    assert_eq!(
        resultado, esperado,
        "La composición jerárquica debe superponer BG3 -> BG2 -> BG1 -> BG0 -> OBJ armónicamente"
    );
}

#[test]
fn test_composicion_dragon_bg1_transparente() {
    let backdrop = 0x0000u16;
    let c3_fuego = 0x001Fu16;
    let c2_dragon = 0x03E0u16; // Verde dragón
    let c1_transparente = TRANSPARENTE;
    let c0_transparente = TRANSPARENTE;
    let obj_color = 0x0000u16;

    let control_con_bg1 = 0x3Fu16; // Todas las capas habilitadas
    let bldcnt = 0x0000u16; // Sin blend

    // Caso A: c1 es TRANSPARENTE, BG2 debe permanecer visible
    let res_transparente = compone_pixel_jerarquica_dragon(
        c0_transparente,
        c1_transparente,
        c2_dragon,
        c3_fuego,
        obj_color,
        false,
        false,
        control_con_bg1,
        bldcnt,
        16,
        0,
        0,
        backdrop,
    );
    assert_eq!(
        res_transparente, c2_dragon,
        "Cuando BG1 es transparente, el dragón (BG2) debe permanecer visible"
    );

    // Caso B: c1 es opaco pero control & 2 == 0 (BG1 deshabilitado por ventana/control)
    let c1_humo = 0x7C00u16;
    let control_sin_bg1 = control_con_bg1 & !2; // Desactiva bit 1 (BG1)
    let res_bg1_deshabilitado = compone_pixel_jerarquica_dragon(
        c0_transparente,
        c1_humo,
        c2_dragon,
        c3_fuego,
        obj_color,
        false,
        false,
        control_sin_bg1,
        bldcnt,
        16,
        0,
        0,
        backdrop,
    );
    assert_eq!(
        res_bg1_deshabilitado, c2_dragon,
        "Cuando BG1 está deshabilitado en control, el dragón (BG2) debe permanecer visible"
    );

    // Caso C: c1 es opaco y control & 2 != 0, BG1 debe superponerse sobre BG2
    let res_bg1_opaco = compone_pixel_jerarquica_dragon(
        c0_transparente,
        c1_humo,
        c2_dragon,
        c3_fuego,
        obj_color,
        false,
        false,
        control_con_bg1,
        bldcnt,
        16,
        0,
        0,
        backdrop,
    );
    assert_eq!(
        res_bg1_opaco, c1_humo,
        "Cuando BG1 es opaco y visible, debe superponerse sobre el dragón (BG2)"
    );
}

#[test]
fn test_hash_entrada_detecta_cambio_bldcnt_linea_50() {
    let mut comps = [ConfigCompGpu::zeroed(); 160];
    for c in comps.iter_mut() {
        c.bldcnt = 0x3F48;
    }
    let vram = vec![0u8; 100];
    let oam = vec![0u8; 100];
    let paleta = vec![0u8; 100];

    let h1 = calcular_hash_entrada(&comps, &vram, &oam, &paleta);
    let h2 = calcular_hash_entrada(&comps, &vram, &oam, &paleta);
    assert_eq!(h1, h2);
    assert_eq!(h1.bld50, 0x3F48);
    assert_eq!(h1.bg3_blend, 160);

    comps[50].bldcnt = 0x3F41;
    let h3 = calcular_hash_entrada(&comps, &vram, &oam, &paleta);
    assert_ne!(h1.hash, h3.hash);
    assert_eq!(h3.bld50, 0x3F41);
    assert_eq!(h3.bg3_blend, 159);
}

#[test]
fn test_hash_cuadros_inactivo_sin_bandera() {
    let estado = EstadoPpuNativa::new();
    assert!(!estado.hash_activo);
}

#[test]
fn test_hash_por_componente_aisla_paleta_lineas() {
    let comps = [ConfigCompGpu::zeroed(); 160];
    let vram = vec![1u8; 100];
    let oam = vec![2u8; 100];
    let paleta = vec![3u8; 100];
    let mut pal_lineas = Box::new([[0u8; 1024]; 160]);

    let h1 = calcular_huellas_componentes(&comps, &vram, &oam, &paleta, &pal_lineas);
    pal_lineas[77][500] = 0xAB;
    let h2 = calcular_huellas_componentes(&comps, &vram, &oam, &paleta, &pal_lineas);

    assert_ne!(h1.paleta_lineas, h2.paleta_lineas);
    assert_eq!(h1.comps, h2.comps);
    assert_eq!(h1.vram, h2.vram);
    assert_eq!(h1.oam, h2.oam);
    assert_eq!(h1.paleta, h2.paleta);
}

#[test]
fn test_hash_por_componente_estable() {
    let comps = [ConfigCompGpu::zeroed(); 160];
    let vram = vec![1u8; 100];
    let oam = vec![2u8; 100];
    let paleta = vec![3u8; 100];
    let pal_lineas = Box::new([[7u8; 1024]; 160]);

    let h1 = calcular_huellas_componentes(&comps, &vram, &oam, &paleta, &pal_lineas);
    let h2 = calcular_huellas_componentes(&comps, &vram, &oam, &paleta, &pal_lineas);
    assert_eq!(h1, h2);
}

#[test]
fn test_sync_vram_inactivo_por_defecto() {
    let estado = crate::ppu_nativa::EstadoPpuNativa::new();
    assert!(!estado.hash_activo);
}

#[test]
fn test_volcar_registros_lineas() {
    let mut lineas = [[0u8; 0x60]; 3];
    let set_u16 = |buf: &mut [u8; 0x60], offset: usize, val: u16| {
        let b = val.to_le_bytes();
        buf[offset] = b[0];
        buf[offset + 1] = b[1];
    };

    set_u16(&mut lineas[0], 0x00, 0x1234);
    set_u16(&mut lineas[0], 0x10, 0x0010);
    set_u16(&mut lineas[0], 0x12, 0x0012);
    set_u16(&mut lineas[0], 0x14, 0x0014);
    set_u16(&mut lineas[0], 0x16, 0x0016);
    set_u16(&mut lineas[0], 0x18, 0x0018);
    set_u16(&mut lineas[0], 0x1A, 0x001A);
    set_u16(&mut lineas[0], 0x1C, 0x001C);
    set_u16(&mut lineas[0], 0x1E, 0x001E);
    set_u16(&mut lineas[0], 0x48, 0x0048);
    set_u16(&mut lineas[0], 0x4A, 0x004A);
    set_u16(&mut lineas[0], 0x50, 0x0050);
    set_u16(&mut lineas[0], 0x52, 0x0052);
    set_u16(&mut lineas[0], 0x54, 0x0054);

    set_u16(&mut lineas[1], 0x00, 0xABCD);

    let res = volcar_registros_lineas(&lineas, 1);
    let esperada = "linea=0 dispcnt=1234 bg0hofs=0010 bg0vofs=0012 bg1hofs=0014 bg1vofs=0016 bg2hofs=0018 bg2vofs=001A bg3hofs=001C bg3vofs=001E winin=0048 winout=004A bldcnt=0050 bldalpha=0052 bldy=0054\n";
    assert_eq!(res, esperada);

    let res_todas = volcar_registros_lineas(&lineas, 10);
    let mut lineas_out = res_todas.lines();
    assert_eq!(lineas_out.next(), Some("linea=0 dispcnt=1234 bg0hofs=0010 bg0vofs=0012 bg1hofs=0014 bg1vofs=0016 bg2hofs=0018 bg2vofs=001A bg3hofs=001C bg3vofs=001E winin=0048 winout=004A bldcnt=0050 bldalpha=0052 bldy=0054"));
    assert_eq!(lineas_out.next(), Some("linea=1 dispcnt=ABCD bg0hofs=0000 bg0vofs=0000 bg1hofs=0000 bg1vofs=0000 bg2hofs=0000 bg2vofs=0000 bg3hofs=0000 bg3vofs=0000 winin=0000 winout=0000 bldcnt=0000 bldalpha=0000 bldy=0000"));
    assert_eq!(lineas_out.next(), Some("linea=2 dispcnt=0000 bg0hofs=0000 bg0vofs=0000 bg1hofs=0000 bg1vofs=0000 bg2hofs=0000 bg2vofs=0000 bg3hofs=0000 bg3vofs=0000 winin=0000 winout=0000 bldcnt=0000 bldalpha=0000 bldy=0000"));
    assert_eq!(lineas_out.next(), None);
}

#[test]
fn test_volcar_bg2_y_dma() {
    let mut lineas = [[0u8; 0x60]; 2];
    let set_u16 = |buf: &mut [u8; 0x60], offset: usize, val: u16| {
        let b = val.to_le_bytes();
        buf[offset] = b[0];
        buf[offset + 1] = b[1];
    };
    set_u16(&mut lineas[0], 0x18, 0x0123);
    set_u16(&mut lineas[0], 0x1A, 0x0456);
    set_u16(&mut lineas[1], 0x18, 0x00A0);
    set_u16(&mut lineas[1], 0x1A, 0x020E);

    let mut io = vec![0u8; 256];
    let set_u32 = |buf: &mut [u8], offset: usize, val: u32| {
        let b = val.to_le_bytes();
        buf[offset..offset + 4].copy_from_slice(&b);
    };
    // DMA0 (0xB0): SAD=0x02001000, DAD=0x04000018, CNT=0x80000010
    set_u32(&mut io, 0xB0, 0x02001000);
    set_u32(&mut io, 0xB4, 0x04000018);
    set_u32(&mut io, 0xB8, 0x80000010);
    // DMA1 (0xBC): SAD=0x03000000, DAD=0x0400001A, CNT=0x84000020
    set_u32(&mut io, 0xBC, 0x03000000);
    set_u32(&mut io, 0xC0, 0x0400001A);
    set_u32(&mut io, 0xC4, 0x84000020);
    // DMA2 (0xC8): todos 0 por defecto
    // DMA3 (0xD4): SAD=0x08000000, DAD=0x02000000, CNT=0x80000100
    set_u32(&mut io, 0xD4, 0x08000000);
    set_u32(&mut io, 0xD8, 0x02000000);
    set_u32(&mut io, 0xDC, 0x80000100);

    let res = volcar_bg2_y_dma(&lineas, &io);
    let mut lineas_out = res.lines();
    assert_eq!(lineas_out.next(), Some("linea=0 bg2hofs=0123 bg2vofs=0456"));
    assert_eq!(lineas_out.next(), Some("linea=1 bg2hofs=00A0 bg2vofs=020E"));
    assert_eq!(lineas_out.next(), Some("# dma"));
    assert_eq!(
        lineas_out.next(),
        Some("dma0 sad=02001000 dad=04000018 cnt=80000010")
    );
    assert_eq!(
        lineas_out.next(),
        Some("dma1 sad=03000000 dad=0400001A cnt=84000020")
    );
    assert_eq!(
        lineas_out.next(),
        Some("dma2 sad=00000000 dad=00000000 cnt=00000000")
    );
    assert_eq!(
        lineas_out.next(),
        Some("dma3 sad=08000000 dad=02000000 cnt=80000100")
    );
    assert_eq!(lineas_out.next(), None);

    // Caso de borde: io corto (< 0xB0) no causa pánico y reporta 0
    let io_corta = vec![0u8; 0x50];
    let res_corta = volcar_bg2_y_dma(&lineas[..1], &io_corta);
    let mut lineas_corta = res_corta.lines();
    assert_eq!(
        lineas_corta.next(),
        Some("linea=0 bg2hofs=0123 bg2vofs=0456")
    );
    assert_eq!(lineas_corta.next(), Some("# dma"));
    assert_eq!(
        lineas_corta.next(),
        Some("dma0 sad=00000000 dad=00000000 cnt=00000000")
    );
    assert_eq!(
        lineas_corta.next(),
        Some("dma1 sad=00000000 dad=00000000 cnt=00000000")
    );
    assert_eq!(
        lineas_corta.next(),
        Some("dma2 sad=00000000 dad=00000000 cnt=00000000")
    );
    assert_eq!(
        lineas_corta.next(),
        Some("dma3 sad=00000000 dad=00000000 cnt=00000000")
    );
    assert_eq!(lineas_corta.next(), None);

    // Caso de borde: más de 160 líneas queda truncado a 160
    let lineas_muchas = vec![[0u8; 0x60]; 180];
    let res_160 = volcar_bg2_y_dma(&lineas_muchas, &io_corta);
    let cuenta_lineas = res_160.lines().filter(|l| l.starts_with("linea=")).count();
    assert_eq!(cuenta_lineas, 160);
}

#[test]
fn test_wgsl_fuego_dragon_mezcla_unica_cuadro_677() {
    let c3_fuego = 0x3420u16;
    let c2_dragon = 0x624Fu16;
    let backdrop = 0x1084u16;
    let bg_prio = [0, 0, 3, 1];
    let obj_prio = 0;
    let control = 0x3Fu16;
    let dispcnt = 0x0C00u16;
    let bldcnt = 0x3F68u16; // modo 1, 1st BG3 (bit 3), 2nd BG2 (bit 10)
    let eva = 16u32;
    let evb = 6u32;

    let res = compone_pixel_fuego_dragon_wgsl(
        TRANSPARENTE,
        TRANSPARENTE,
        c2_dragon,
        c3_fuego,
        0,
        false,
        false,
        bg_prio,
        obj_prio,
        control,
        dispcnt,
        bldcnt,
        eva,
        evb,
        0,
        backdrop,
    );

    // Debe ser mezcla pura de BG3 sobre BG2 con eva=16, evb=6, sin doble mezcla con backdrop
    let esperado = alpha_blend(c3_fuego, c2_dragon, eva, evb);
    assert_eq!(esperado, 0x5906u16);
    assert_eq!(
        res, 0x5906u16,
        "El cuadro 677 debe coincidir exactamente con el hardware GBA"
    );
}

#[test]
fn test_wgsl_fuego_dragon_preserva_bg2_con_bldcnt_texto() {
    let c3_fuego = 0x3420u16;
    let c2_dragon = 0x624Fu16;
    let backdrop = 0x0000u16;
    let bg_prio = [0, 0, 3, 1];
    let obj_prio = 0;
    let control = 0x3Fu16;
    let dispcnt = 0x0C00u16;
    let bldcnt_texto = 0x2861u16; // Modo texto (1st BG0), sin bit 3 (BG3)

    let res = compone_pixel_fuego_dragon_wgsl(
        TRANSPARENTE,
        TRANSPARENTE,
        c2_dragon,
        c3_fuego,
        0,
        false,
        false,
        bg_prio,
        obj_prio,
        control,
        dispcnt,
        bldcnt_texto,
        16,
        8,
        0,
        backdrop,
    );

    // Debe mezclar fuego (BG3) sobre el dragón (BG2) usando pesos de respaldo 16, 8
    let esperado = alpha_blend(c3_fuego, c2_dragon, 16, 8);
    assert_eq!(
        res, esperado,
        "Debe preservar el cuerpo del dragón debajo del fuego"
    );
    assert_ne!(
        res, c3_fuego,
        "El fuego no debe tapar de forma opaca al dragón"
    );
}

#[test]
fn test_wgsl_fuego_dragon_subtitulo_bg0_sobre_bg3_y_no_regresion() {
    let c0_texto = 0x7FFFu16; // Blanco
    let c3_fuego = 0x3420u16;
    let backdrop = 0x0000u16;
    let bg_prio = [0, 1, 3, 1]; // BG0 prio 0, BG3 prio 1
    let obj_prio = 0;
    let control = 0x3Fu16;
    let dispcnt = 0x0C00u16;
    let bldcnt_texto = 0x2861u16; // 1st BG0, 2nd BG3

    let res = compone_pixel_fuego_dragon_wgsl(
        c0_texto,
        TRANSPARENTE,
        TRANSPARENTE,
        c3_fuego,
        0,
        false,
        false,
        bg_prio,
        obj_prio,
        control,
        dispcnt,
        bldcnt_texto,
        16,
        8,
        0,
        backdrop,
    );

    let fuego_fondo = alpha_blend(c3_fuego, backdrop, 16, 8);
    let esperado = alpha_blend(c0_texto, fuego_fondo, 16, 8);
    assert_eq!(
        res, esperado,
        "El texto debe mezclarse suavemente sobre el fuego precompuesto"
    );
}
