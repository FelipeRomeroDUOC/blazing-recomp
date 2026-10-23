//! Captura visual bajo demanda y diagnóstico causal de las diferencias entre la PPU
//! nativa y la PPU por software
//! (`docs/propuestas/2026-10-22_arnes-comparacion-visual-y-causal.md`).
//!
//! Tres piezas, todas puras (sin GPU ni `gba-core`) para que
//! `tests/arnes_experimentos_tests.rs` las pruebe con `#[path]`:
//!
//! - [`guardar_captura_si_difiere`]: escribe **una** imagen PNG compuesta de 960x160
//!   (`referencia | nativa | mapa de diferencias | error x8`) sólo si hay píxeles que
//!   difieren más de [`TOLERANCIA`]. Sin diferencias no toca el disco.
//! - [`diagnosticar`]: dice por qué difieren. Si la nativa coincide con el estado de fin de
//!   cuadro, la causa es el estado que cambia a mitad de cuadro (y nombra qué cambió); si
//!   no, corre una ablación (apagar cada capa, ventana y la mezcla) mediante un cierre que
//!   pone el experimento, e inspecciona el primer píxel distinto.
//! - [`FiltroFirmas`]: en un barrido, evita repetir la captura de cuadros consecutivos con
//!   el mismo desajuste.

use std::collections::HashMap;
use std::io;
use std::path::{Path, PathBuf};

use super::color::TOLERANCIA;
use super::registro::io16;

pub const ANCHO: usize = 240;
pub const ALTO: usize = 160;
const PIXELES: usize = ANCHO * ALTO;
/// Ancho del lienzo compuesto: cuatro paneles de 240 en fila.
pub const ANCHO_COMPUESTO: usize = ANCHO * 4;

// ------------------------------------------------------------------ diferencias
/// Qué y dónde difiere entre dos imágenes RGB de 240x160.
#[derive(Debug, Clone, PartialEq)]
pub struct Diferencias {
    pub distintos: usize,
    /// Caja que encierra todos los píxeles distintos: `(x0, y0, x1, y1)`, inclusiva.
    pub caja: (usize, usize, usize, usize),
    /// Primer píxel distinto en orden de lectura: `(x, y)`.
    pub primero: (usize, usize),
    /// Píxel con mayor diferencia en un canal (el primero si empatan): `(x, y)`.
    pub max_error: (usize, usize),
}

/// Lo que devuelve una captura escrita: las diferencias y la ruta del PNG.
#[derive(Debug, Clone, PartialEq)]
pub struct ResumenVisual {
    pub distintos: usize,
    pub caja: (usize, usize, usize, usize),
    pub primero: (usize, usize),
    pub max_error: (usize, usize),
    pub ruta: PathBuf,
}

/// Mayor diferencia por canal del píxel `p`.
fn delta(a: &[u8], b: &[u8], p: usize) -> u8 {
    (0..3).map(|k| a[p * 3 + k].abs_diff(b[p * 3 + k])).max().unwrap_or(0)
}

/// Compara dos imágenes con la tolerancia del arnés; `None` si no hay diferencias.
pub fn diferencias(a: &[u8], b: &[u8]) -> Option<Diferencias> {
    let mut r: Option<Diferencias> = None;
    let mut peor = 0u8;
    for p in 0..PIXELES {
        let d = delta(a, b, p);
        if d <= TOLERANCIA {
            continue;
        }
        let (x, y) = (p % ANCHO, p / ANCHO);
        match r.as_mut() {
            None => {
                peor = d;
                r = Some(Diferencias { distintos: 1, caja: (x, y, x, y), primero: (x, y), max_error: (x, y) });
            }
            Some(dif) => {
                dif.distintos += 1;
                dif.caja.0 = dif.caja.0.min(x);
                dif.caja.1 = dif.caja.1.min(y);
                dif.caja.2 = dif.caja.2.max(x);
                dif.caja.3 = dif.caja.3.max(y);
                if d > peor {
                    peor = d;
                    dif.max_error = (x, y);
                }
            }
        }
    }
    r
}

/// Número de píxeles distintos, con la tolerancia del arnés.
pub fn cuenta_distintos(a: &[u8], b: &[u8]) -> usize {
    (0..PIXELES).filter(|&p| delta(a, b, p) > TOLERANCIA).count()
}

// ---------------------------------------------------------------------- lienzo
/// Lienzo RGB de 960x160: referencia, nativa, mapa de diferencias y error amplificado x8.
///
/// En el mapa, el fondo es la referencia atenuada a gris; un píxel distinto en 2-4
/// niveles (redondeo de mezcla) va en amarillo y uno más distinto en rojo. La caja de las
/// diferencias se marca en cian.
pub fn lienzo_compuesto(sw: &[u8], nat: &[u8]) -> Vec<u8> {
    let mut out = vec![0u8; ANCHO_COMPUESTO * ALTO * 3];
    let caja = diferencias(sw, nat).map(|d| d.caja);
    for y in 0..ALTO {
        for x in 0..ANCHO {
            let p = y * ANCHO + x;
            let fila = y * ANCHO_COMPUESTO;
            let mut pon = |panel: usize, rgb: [u8; 3]| {
                let o = (fila + panel * ANCHO + x) * 3;
                out[o..o + 3].copy_from_slice(&rgb);
            };
            pon(0, [sw[p * 3], sw[p * 3 + 1], sw[p * 3 + 2]]);
            pon(1, [nat[p * 3], nat[p * 3 + 1], nat[p * 3 + 2]]);

            let d = delta(sw, nat, p);
            let mapa = if d > 4 {
                [0xFF, 0x00, 0x00]
            } else if d > TOLERANCIA {
                [0xFF, 0xFF, 0x00]
            } else {
                let borde = caja.is_some_and(|(x0, y0, x1, y1)| {
                    (x >= x0 && x <= x1 && y >= y0 && y <= y1)
                        && (x == x0 || x == x1 || y == y0 || y == y1)
                });
                if borde {
                    [0x00, 0xFF, 0xFF]
                } else {
                    let g = ((sw[p * 3] as u16 + sw[p * 3 + 1] as u16 + sw[p * 3 + 2] as u16) / 6) as u8;
                    [g, g, g]
                }
            };
            pon(2, mapa);

            let amp = |k: usize| (sw[p * 3 + k].abs_diff(nat[p * 3 + k]) as u16 * 8).min(255) as u8;
            pon(3, [amp(0), amp(1), amp(2)]);
        }
    }
    out
}

// ------------------------------------------------------------------------- PNG
fn crc32(datos: &[u8]) -> u32 {
    let mut crc = 0xFFFF_FFFFu32;
    for &b in datos {
        crc ^= b as u32;
        for _ in 0..8 {
            crc = if crc & 1 != 0 { (crc >> 1) ^ 0xEDB8_8320 } else { crc >> 1 };
        }
    }
    !crc
}

fn adler32(datos: &[u8]) -> u32 {
    let (mut a, mut b) = (1u32, 0u32);
    for &x in datos {
        a = (a + x as u32) % 65521;
        b = (b + a) % 65521;
    }
    (b << 16) | a
}

/// Envoltorio zlib con bloques deflate *stored* (sin compresión): válido para cualquier
/// visor y sin dependencias. El lienzo de 960x160 ocupa ~450 KiB.
fn zlib_stored(datos: &[u8]) -> Vec<u8> {
    let mut out = vec![0x78, 0x01];
    let trozos: Vec<&[u8]> = if datos.is_empty() { vec![&[][..]] } else { datos.chunks(65535).collect() };
    let n = trozos.len();
    for (i, t) in trozos.into_iter().enumerate() {
        out.push(if i + 1 == n { 1 } else { 0 });
        let len = t.len() as u16;
        out.extend_from_slice(&len.to_le_bytes());
        out.extend_from_slice(&(!len).to_le_bytes());
        out.extend_from_slice(t);
    }
    out.extend_from_slice(&adler32(datos).to_be_bytes());
    out
}

/// Codifica un PNG RGB8 de `ancho` x `alto`.
pub fn png_rgb(rgb: &[u8], ancho: usize, alto: usize) -> Vec<u8> {
    let mut filas = Vec::with_capacity(alto * (ancho * 3 + 1));
    for y in 0..alto {
        filas.push(0); // filtro None
        filas.extend_from_slice(&rgb[y * ancho * 3..(y + 1) * ancho * 3]);
    }
    let bloque = |tipo: &[u8], datos: &[u8]| {
        let mut b = (datos.len() as u32).to_be_bytes().to_vec();
        let mut cuerpo = tipo.to_vec();
        cuerpo.extend_from_slice(datos);
        b.extend_from_slice(&cuerpo);
        b.extend_from_slice(&crc32(&cuerpo).to_be_bytes());
        b
    };
    let mut ihdr = (ancho as u32).to_be_bytes().to_vec();
    ihdr.extend_from_slice(&(alto as u32).to_be_bytes());
    ihdr.extend_from_slice(&[8, 2, 0, 0, 0]);
    let mut png = b"\x89PNG\r\n\x1a\n".to_vec();
    png.extend(bloque(b"IHDR", &ihdr));
    png.extend(bloque(b"IDAT", &zlib_stored(&filas)));
    png.extend(bloque(b"IEND", &[]));
    png
}

/// Escribe `dir/cuadro-<N>-<caso>.png` sólo si `sw` y `nat` difieren más allá de la
/// tolerancia. Sin diferencias devuelve `Ok(None)` y no crea nada (ni la carpeta).
pub fn guardar_captura_si_difiere(
    dir: &Path,
    cuadro: u32,
    caso: &str,
    sw: &[u8],
    nat: &[u8],
) -> io::Result<Option<ResumenVisual>> {
    let Some(d) = diferencias(sw, nat) else {
        return Ok(None);
    };
    std::fs::create_dir_all(dir)?;
    let ruta = dir.join(format!("cuadro-{cuadro}-{caso}.png"));
    std::fs::write(&ruta, png_rgb(&lienzo_compuesto(sw, nat), ANCHO_COMPUESTO, ALTO))?;
    Ok(Some(ResumenVisual {
        distintos: d.distintos,
        caja: d.caja,
        primero: d.primero,
        max_error: d.max_error,
        ruta,
    }))
}

/// Guarda una captura PNG compuesta de 960x160 de forma incondicional (p. ej. forzada por tecla F5).
pub fn guardar_captura_forzada(
    dir: &Path,
    cuadro: u32,
    etiqueta: &str,
    sw: &[u8],
    nat: &[u8],
) -> io::Result<PathBuf> {
    std::fs::create_dir_all(dir)?;
    let ruta = dir.join(format!("cuadro-{cuadro}-{etiqueta}.png"));
    std::fs::write(&ruta, png_rgb(&lienzo_compuesto(sw, nat), ANCHO_COMPUESTO, ALTO))?;
    Ok(ruta)
}

pub const ANCHO_DOBLE: usize = ANCHO * 2; // 480

/// Compone la imagen doble de 480x160 en formato BGRX (0x00RRGGBB de 32 bits para GDI/Windows).
/// Mitad izquierda: PPU por software (240x160).
/// Mitad derecha: PPU nativa (240x160). Si `resaltar_dif` está activo, los píxeles donde difieren
/// se pintan en magenta (0x00FF00FF).
pub fn lienzo_doble_bgrx(sw: &[u8], nat: &[u8], resaltar_dif: bool) -> Vec<u32> {
    let mut salida = vec![0u32; ANCHO_DOBLE * ALTO];
    for y in 0..ALTO {
        let fila_salida = y * ANCHO_DOBLE;
        let fila_origen = y * ANCHO;
        for x in 0..ANCHO {
            let idx_orig = (fila_origen + x) * 3;
            let sw_r = sw[idx_orig];
            let sw_g = sw[idx_orig + 1];
            let sw_b = sw[idx_orig + 2];
            let sw_u32 = ((sw_r as u32) << 16) | ((sw_g as u32) << 8) | (sw_b as u32);

            let nat_r = nat[idx_orig];
            let nat_g = nat[idx_orig + 1];
            let nat_b = nat[idx_orig + 2];

            let difiere = canal_8bit_dif(sw_r, nat_r) > TOLERANCIA
                || canal_8bit_dif(sw_g, nat_g) > TOLERANCIA
                || canal_8bit_dif(sw_b, nat_b) > TOLERANCIA;

            let nat_u32 = if resaltar_dif && difiere {
                0x00FF00FF // magenta de advertencia
            } else {
                ((nat_r as u32) << 16) | ((nat_g as u32) << 8) | (nat_b as u32)
            };

            salida[fila_salida + x] = sw_u32;
            salida[fila_salida + ANCHO + x] = nat_u32;
        }
    }
    salida
}

#[inline]
fn canal_8bit_dif(a: u8, b: u8) -> u8 {
    a.abs_diff(b)
}

/// Decide si un cuadro durante el juego interactivo debe capturarse.
/// Devuelve `Some("manual")` si fue forzado por tecla, `Some("fin")` si hay diferencia nueva,
/// o `None` si es idéntico o repite la firma del cuadro anterior.
pub fn render_software_fin(
    rom: &[u8],
    vram: &[u8],
    oam: &[u8],
    palette: &[u8],
    io: &[u8],
) -> Vec<u8> {
    let mut replay = gba_core::Machine::new(rom.to_vec());
    replay.bus.palette = palette.to_vec();
    replay.bus.vram = vram.to_vec();
    replay.bus.oam = oam.to_vec();
    replay.bus.io = io.to_vec();
    let mut img = vec![0u16; ANCHO * ALTO];
    for l in 0..ALTO {
        gba_core::ppu::render_scanline(&mut replay.bus, l);
        let ini = l * ANCHO;
        img[ini..ini + ANCHO].copy_from_slice(&replay.bus.framebuffer[ini..ini + ANCHO]);
    }
    super::color::software_a_rgb(&img)
}

/// Decide si un cuadro durante el juego interactivo debe capturarse.
/// Devuelve `Some("manual")` si fue forzado por tecla, `Some("fin")` si hay diferencia nueva en fin de cuadro,
/// o `None` si es idéntico o repite la firma del cuadro anterior.
pub fn decidir_captura_en_vivo(
    _cuadro: u32,
    sw: &[u8],
    nat: &[u8],
    dispcnt: u16,
    bldcnt: u16,
    forzar_manual: bool,
    filtro: &mut FiltroFirmas,
) -> Option<String> {
    if forzar_manual {
        return Some("manual".to_string());
    }

    if let Some(d) = diferencias(sw, nat) {
        let f = Firma {
            caso: "fin".to_string(),
            caja: d.caja,
            dispcnt,
            bldcnt,
        };
        if filtro.capturar(f) {
            return Some("fin".to_string());
        }
    } else {
        filtro.reiniciar("fin");
    }
    None
}

/// Evalúa el cuadro distinguiendo fallos reales del shader (`dif_fin > 0`) de desajustes por línea
/// (`dif_fin == 0` y `dif_linea > 0`). Para los desajustes por línea, cuantiza la caja de diferencias
/// en bloques de 32x32 píxeles para agrupar transiciones graduales sin saturar el disco.
pub fn evaluar_cuadro_en_vivo(
    _cuadro: u32,
    dif_fin: usize,
    dif_linea: usize,
    caja_linea: (usize, usize, usize, usize),
    dispcnt: u16,
    bldcnt: u16,
    forzar_manual: bool,
    filtro: &mut FiltroFirmas,
) -> Option<String> {
    if forzar_manual {
        return Some("manual".to_string());
    }

    if dif_fin > 0 {
        // Fallo real de fin de cuadro: captura inmediata con precisión exacta
        let f = Firma {
            caso: "fin".to_string(),
            caja: caja_linea,
            dispcnt,
            bldcnt,
        };
        if filtro.capturar(f) {
            return Some("fin".to_string());
        }
    } else {
        filtro.reiniciar("fin");
    }

    if dif_linea > 0 {
        // Desajuste solo por línea (HBlank/VBlank): caja cuantizada a bloques de 32 px
        let caja_cuantizada = (
            caja_linea.0 / 32,
            caja_linea.1 / 32,
            caja_linea.2 / 32,
            caja_linea.3 / 32,
        );
        let f = Firma {
            caso: "linea".to_string(),
            caja: caja_cuantizada,
            dispcnt,
            bldcnt,
        };
        if filtro.capturar(f) {
            return Some("linea".to_string());
        }
    } else {
        filtro.reiniciar("linea");
    }

    None
}

// ------------------------------------------------------------------ diagnóstico
/// Estado de la PPU en un momento: lo que leen las dos PPU.
#[derive(Clone, Copy)]
pub struct EstadoRef<'a> {
    pub io: &'a [u8],
    pub palette: &'a [u8],
    pub vram: &'a [u8],
    pub oam: &'a [u8],
}

/// Nombre de los registros de vídeo que interesan al diagnóstico. `DISPSTAT` y `VCOUNT`
/// quedan fuera a propósito: cambian solos en cada línea y no los escribe el juego.
pub fn nombre_registro(dir: usize) -> Option<&'static str> {
    Some(match dir {
        0x00 => "DISPCNT",
        0x08 => "BG0CNT",
        0x0A => "BG1CNT",
        0x0C => "BG2CNT",
        0x0E => "BG3CNT",
        0x10 => "BG0HOFS",
        0x12 => "BG0VOFS",
        0x14 => "BG1HOFS",
        0x16 => "BG1VOFS",
        0x18 => "BG2HOFS",
        0x1A => "BG2VOFS",
        0x1C => "BG3HOFS",
        0x1E => "BG3VOFS",
        0x20 => "BG2PA",
        0x22 => "BG2PB",
        0x24 => "BG2PC",
        0x26 => "BG2PD",
        0x28 => "BG2X",
        0x2C => "BG2Y",
        0x30 => "BG3PA",
        0x32 => "BG3PB",
        0x34 => "BG3PC",
        0x36 => "BG3PD",
        0x38 => "BG3X",
        0x3C => "BG3Y",
        0x40 => "WIN0H",
        0x42 => "WIN1H",
        0x44 => "WIN0V",
        0x46 => "WIN1V",
        0x48 => "WININ",
        0x4A => "WINOUT",
        0x4C => "MOSAIC",
        0x50 => "BLDCNT",
        0x52 => "BLDALPHA",
        0x54 => "BLDY",
        _ => return None,
    })
}

/// Direcciones de los registros con nombre (los de 32 bits ocupan dos palabras).
fn rango_registro(dir: usize) -> std::ops::Range<usize> {
    match dir {
        0x28 | 0x2C | 0x38 | 0x3C => dir..dir + 4,
        _ => dir..dir + 2,
    }
}

/// Componentes (`regs`, `paleta`, `vram`, `oam`) y registros con nombre que alguna línea
/// tenía distintos del estado de fin de cuadro.
pub fn cambios_por_linea(fin: EstadoRef, lineas: &[EstadoRef]) -> (Vec<&'static str>, Vec<&'static str>) {
    let mut componentes = Vec::new();
    let mut registros = Vec::new();
    for l in lineas {
        for (nombre, a, b) in [
            ("regs", l.io, fin.io),
            ("paleta", l.palette, fin.palette),
            ("vram", l.vram, fin.vram),
            ("oam", l.oam, fin.oam),
        ] {
            if a != b && !componentes.contains(&nombre) {
                componentes.push(nombre);
            }
        }
        for dir in (0..0x56).step_by(2) {
            let Some(nombre) = nombre_registro(dir) else { continue };
            let r = rango_registro(dir);
            if r.end <= l.io.len() && r.end <= fin.io.len() && l.io[r.clone()] != fin.io[r] && !registros.contains(&nombre) {
                registros.push(nombre);
            }
        }
    }
    // Un cambio sólo en DISPSTAT/VCOUNT no cuenta como registro escrito por el juego.
    if registros.is_empty() {
        componentes.retain(|c| *c != "regs");
    }
    (componentes, registros)
}

/// Variantes de ablación: cada una apaga una capa, una ventana o la mezcla.
pub fn variantes_ablacion(io: &[u8]) -> Vec<(&'static str, Vec<u8>)> {
    let apaga = |bits: u16| {
        let mut v = io.to_vec();
        let d = io16(io, 0) & !bits;
        v[0..2].copy_from_slice(&d.to_le_bytes());
        v
    };
    let mut sin_mezcla = io.to_vec();
    sin_mezcla[0x50] = 0;
    sin_mezcla[0x51] = 0;
    vec![
        ("sin BG0", apaga(0x0100)),
        ("sin BG1", apaga(0x0200)),
        ("sin BG2", apaga(0x0400)),
        ("sin BG3", apaga(0x0800)),
        ("sin OBJ", apaga(0x1000)),
        ("sin WIN0", apaga(0x2000)),
        ("sin WIN1", apaga(0x4000)),
        ("sin OBJWIN", apaga(0x8000)),
        ("sin BLDCNT", sin_mezcla),
    ]
}

const OBJ_TAMANOS: [[(i32, i32); 4]; 3] = [
    [(8, 8), (16, 16), (32, 32), (64, 64)],
    [(16, 8), (32, 8), (32, 16), (64, 32)],
    [(8, 16), (8, 32), (16, 32), (32, 64)],
];

/// Índices de OAM cuya caja cubre `(x, y)` (sin mirar si el píxel es transparente).
pub fn sprites_en_pixel(oam: &[u8], x: usize, y: usize) -> Vec<usize> {
    let mut out = Vec::new();
    for i in 0..128 {
        if oam.len() < i * 8 + 4 {
            break;
        }
        let a0 = u16::from_le_bytes([oam[i * 8], oam[i * 8 + 1]]);
        let a1 = u16::from_le_bytes([oam[i * 8 + 2], oam[i * 8 + 3]]);
        let afin = a0 & 0x100 != 0;
        if !afin && a0 & 0x200 != 0 {
            continue;
        }
        let forma = ((a0 >> 14) & 3) as usize;
        if forma == 3 {
            continue;
        }
        let (w, h) = OBJ_TAMANOS[forma][((a1 >> 14) & 3) as usize];
        let (w, h) = if afin && a0 & 0x200 != 0 { (w * 2, h * 2) } else { (w, h) };
        let mut sy = (a0 & 0xFF) as i32;
        if sy >= 160 {
            sy -= 256;
        }
        let mut sx = (a1 & 0x1FF) as i32;
        if sx >= 240 {
            sx -= 512;
        }
        let (x, y) = (x as i32, y as i32);
        if x >= sx && x < sx + w && y >= sy && y < sy + h {
            out.push(i);
        }
    }
    out
}

/// Ventana que gobierna `(x, y)` según los registros. La máscara de OBJWIN depende de los
/// píxeles de los sprites, así que sólo se dice si podría aplicarse.
pub fn ventana_en_pixel(io: &[u8], x: usize, y: usize) -> &'static str {
    let dispcnt = io16(io, 0);
    let dentro = |h: u16, v: u16| {
        let rango = |ini: u16, fin: u16, p: usize| {
            let (ini, fin, p) = (ini as usize, fin as usize, p);
            if ini <= fin { p >= ini && p < fin } else { p >= ini || p < fin }
        };
        rango(h >> 8, h & 0xFF, x) && rango(v >> 8, v & 0xFF, y)
    };
    if dispcnt & 0x2000 != 0 && dentro(io16(io, 0x40), io16(io, 0x44)) {
        "WIN0"
    } else if dispcnt & 0x4000 != 0 && dentro(io16(io, 0x42), io16(io, 0x46)) {
        "WIN1"
    } else if dispcnt & 0x8000 != 0 {
        "OBJWIN o fuera (WINOUT)"
    } else if dispcnt & 0xE000 != 0 {
        "fuera (WINOUT)"
    } else {
        "sin ventanas"
    }
}

/// Lo que se sabe de un píxel concreto.
#[derive(Debug, Clone, PartialEq)]
pub struct Testigo {
    pub x: usize,
    pub y: usize,
    pub sw: [u8; 3],
    pub nat: [u8; 3],
    pub ventana: &'static str,
    pub capas: Vec<&'static str>,
    pub sprites: Vec<usize>,
}

fn testigo(x: usize, y: usize, sw: &[u8], nat: &[u8], estado: EstadoRef) -> Testigo {
    let p = (y * ANCHO + x) * 3;
    let dispcnt = io16(estado.io, 0);
    let capas = [("BG0", 0x100), ("BG1", 0x200), ("BG2", 0x400), ("BG3", 0x800), ("OBJ", 0x1000)]
        .into_iter()
        .filter(|(_, b)| dispcnt & b != 0)
        .map(|(n, _)| n)
        .collect();
    Testigo {
        x,
        y,
        sw: [sw[p], sw[p + 1], sw[p + 2]],
        nat: [nat[p], nat[p + 1], nat[p + 2]],
        ventana: ventana_en_pixel(estado.io, x, y),
        capas,
        sprites: sprites_en_pixel(estado.oam, x, y),
    }
}

/// Resultado del diagnóstico de un cuadro.
#[derive(Debug, Clone, PartialEq)]
pub struct Diagnostico {
    /// `sin_diferencias`, `estado_por_linea` o `render_fin_de_cuadro`.
    pub causa: &'static str,
    pub dif_fin: usize,
    pub dif_linea: usize,
    pub componentes: Vec<&'static str>,
    pub registros: Vec<&'static str>,
    /// Píxeles que siguen distintos en cada variante de la ablación.
    pub residuos: Vec<(String, usize)>,
    /// Variantes que dejan el residuo en 0.
    pub resuelve: Vec<String>,
    /// Primer píxel distinto frente a la referencia que manda la causa.
    pub testigo: Option<Testigo>,
}

/// Diagnostica por qué la nativa difiere de la PPU por software.
///
/// `nativa`, `sw_fin` (PPU por software con el estado de fin de cuadro) y `sw_linea` (con
/// el estado de cada línea, lo que dibuja el emulador) son RGB de 240x160. `ablacion`
/// recibe el nombre de una variante y su I/O, dibuja con las dos PPU y devuelve cuántos
/// píxeles difieren; sólo se llama si hay fallo con el estado de fin de cuadro.
pub fn diagnosticar(
    nativa: &[u8],
    sw_fin: &[u8],
    sw_linea: &[u8],
    fin: EstadoRef,
    lineas: &[EstadoRef],
    mut ablacion: impl FnMut(&str, &[u8]) -> usize,
) -> Diagnostico {
    let d_fin = diferencias(nativa, sw_fin);
    let d_linea = diferencias(nativa, sw_linea);
    let (componentes, registros) = cambios_por_linea(fin, lineas);
    let mut diag = Diagnostico {
        causa: "sin_diferencias",
        dif_fin: d_fin.as_ref().map_or(0, |d| d.distintos),
        dif_linea: d_linea.as_ref().map_or(0, |d| d.distintos),
        componentes,
        registros,
        residuos: Vec::new(),
        resuelve: Vec::new(),
        testigo: None,
    };
    if let Some(d) = d_fin {
        diag.causa = "render_fin_de_cuadro";
        for (nombre, io_v) in variantes_ablacion(fin.io) {
            let residuo = ablacion(nombre, &io_v);
            if residuo == 0 {
                diag.resuelve.push(nombre.to_string());
            }
            diag.residuos.push((nombre.to_string(), residuo));
        }
        diag.testigo = Some(testigo(d.primero.0, d.primero.1, sw_fin, nativa, fin));
    } else if let Some(d) = d_linea {
        diag.causa = "estado_por_linea";
        let y = d.primero.1;
        let estado = lineas.get(y).copied().unwrap_or(fin);
        diag.testigo = Some(testigo(d.primero.0, y, sw_linea, nativa, estado));
    }
    diag
}

impl Diagnostico {
    /// Líneas de texto para el informe.
    pub fn informe(&self) -> Vec<String> {
        let mut l = vec![format!("causa principal: {}", self.causa)];
        l.push(format!(
            "diferencias nativa: frente a fin de cuadro {}, frente a estado por linea {}",
            self.dif_fin, self.dif_linea
        ));
        let lista = |v: &[&str]| if v.is_empty() { "-".to_string() } else { v.join(",") };
        l.push(format!("componentes que cambian dentro del cuadro: {}", lista(&self.componentes)));
        l.push(format!("registros que cambian dentro del cuadro: {}", lista(&self.registros)));
        match self.causa {
            "estado_por_linea" => l.push(
                "veredicto: la logica de dibujo coincide con el estado congelado; la diferencia \
                 viene del estado que el juego cambia a mitad de cuadro"
                    .to_string(),
            ),
            "render_fin_de_cuadro" => {
                l.push("ablacion (residuo con cada variante):".to_string());
                for (n, r) in &self.residuos {
                    l.push(format!("  {n:<11} {r:6}"));
                }
                l.push(format!(
                    "veredicto: {}",
                    if self.resuelve.is_empty() {
                        "ninguna variante sola deja el residuo en 0 (fallo compartido o del fondo)".to_string()
                    } else {
                        format!("el residuo cae a 0 con {}", self.resuelve.join(", "))
                    }
                ));
            }
            _ => {}
        }
        if let Some(t) = &self.testigo {
            l.push(format!(
                "pixel testigo x={} y={}: software {:02X?} nativa {:02X?} ventana {} capas [{}] sprites {:?}",
                t.x, t.y, t.sw, t.nat, t.ventana, t.capas.join(","), t.sprites
            ));
        }
        l
    }
}

// ----------------------------------------------------------------- deduplicación
/// Firma de un desajuste: si dos cuadros seguidos la comparten, la imagen sería la misma.
#[derive(Debug, Clone, PartialEq)]
pub struct Firma {
    pub caso: String,
    pub caja: (usize, usize, usize, usize),
    pub dispcnt: u16,
    pub bldcnt: u16,
}

/// Recuerda la última firma de cada caso y dice si hay que capturar la siguiente.
#[derive(Debug, Default)]
pub struct FiltroFirmas {
    ultima: HashMap<String, Firma>,
    pub omitidas: usize,
}

impl FiltroFirmas {
    /// `true` si la firma es nueva para su caso (hay que capturar); `false` si repite la
    /// del cuadro anterior de ese caso.
    pub fn capturar(&mut self, firma: Firma) -> bool {
        if self.ultima.get(&firma.caso) == Some(&firma) {
            self.omitidas += 1;
            return false;
        }
        self.ultima.insert(firma.caso.clone(), firma);
        true
    }

    /// Olvida la firma de un caso (un cuadro sin diferencias corta la racha).
    pub fn reiniciar(&mut self, caso: &str) {
        self.ultima.remove(caso);
    }
}
