//! Experimento de docs/propuestas/2026-10-17_localizar-pixeles-obj-y-contenido-bg-cuadro-1000.md.
//!
//! Con las capturas por línea del cuadro N (feature `perf-video`), mide qué contienen BG0, BG1 y
//! BG3 leyendo la VRAM, la paleta y los registros de cada línea. No usa `compose`: son cifras de
//! contenido, no de composición. Por cada fondo mide, sobre toda la pantalla (160 líneas × 240
//! píxeles):
//!   - pix pantalla: píxeles evaluados;
//!   - pix entrada no cero: píxeles cuya entrada de mapa tiene índice de tile distinto de 0;
//!   - entradas visibles no cero: entradas de mapa distintas (16 bits) distintas de 0;
//!   - pix tile con datos: píxeles cuyo tile tiene algún byte distinto de 0;
//!   - tiles referenciados con datos: tiles distintos con datos que aparecen en pantalla;
//!   - pix opacos: píxeles de tile con índice de color distinto de 0 (el 0 es transparente);
//!   - pix visibles: píxeles opacos cuyo color es distinto del color de fondo de su línea;
//!   - pix opacos iguales al fondo: píxeles opacos cuyo color coincide con el fondo (H11);
//!   - colores distintos: colores de paleta de los píxeles opacos.
//! Además cuenta las líneas en que BGnCNT, HOFS o VOFS cambian respecto de la línea 0, y las
//! líneas en que la VRAM cambia respecto de la línea 0.
//! `tiles con datos (0..1023)` se cuenta sobre los índices del charblock de la línea 0.
//!
//! Escribe volcados/contenido-bg-cuadro-<N>.txt: un renglón `bg BGn ...` por fondo.
//!
//! Uso:
//!   cargo run --release --features proto --example ppu_contenido_bg -- <ruta.gba> <secuencia.txt> <cuadro>

use std::collections::HashSet;

#[path = "comun/mod.rs"]
mod comun;

use comun::entrada::leer_secuencia;
use comun::gpu::{ALTO, ANCHO};
use comun::registro::{dispcnt_de, io16};

/// Registros de control, desplazamiento horizontal y vertical de cada fondo (0x08..0x0E, 0x10..0x1E).
const BG_CNT: [usize; 4] = [0x08, 0x0A, 0x0C, 0x0E];
const BG_HOFS: [usize; 4] = [0x10, 0x14, 0x18, 0x1C];
const BG_VOFS: [usize; 4] = [0x12, 0x16, 0x1A, 0x1E];
/// Bit de habilitación de cada fondo en DISPCNT.
const BIT_BG: [u16; 4] = [0x0100, 0x0200, 0x0400, 0x0800];
/// Fondos medidos: BG0, BG1 y BG3 (BG2 queda fuera a propósito).
const BGS: [usize; 3] = [0, 1, 3];

/// Color de la paleta de índice `i` (0..511), sin el bit 15.
fn color_paleta(palette: &[u8], i: usize) -> u16 {
    u16::from_le_bytes([palette[2 * i], palette[2 * i + 1]]) & 0x7FFF
}

/// Tamaño del mapa en tiles, según los bits 14 y 15 de BGnCNT.
fn tamano_mapa(cnt: u16) -> (usize, usize) {
    match (cnt >> 14) & 3 {
        0 => (32, 32),
        1 => (64, 32),
        2 => (32, 64),
        _ => (64, 64),
    }
}

/// Inicio del charblock (tiles) de BGnCNT: bits 2 y 3.
fn base_char(cnt: u16) -> usize {
    ((cnt >> 2) & 3) as usize * 0x4000
}

/// Bytes de un tile: 32 con 4 bpp, 64 con 8 bpp (bit 7 de BGnCNT).
fn tam_tile(cnt: u16) -> usize {
    if cnt & 0x80 != 0 {
        64
    } else {
        32
    }
}

/// Entrada de mapa del tile (tx, ty). Los mapas grandes ocupan varios bloques de 2 KB en orden
/// superior-izquierda, superior-derecha, inferior-izquierda, inferior-derecha.
fn entrada_mapa(vram: &[u8], cnt: u16, tx: usize, ty: usize) -> u16 {
    let (ancho_tiles, _) = tamano_mapa(cnt);
    let sbb = ((cnt >> 8) & 0x1F) as usize * 0x800;
    let bloque = (ty / 32) * (ancho_tiles / 32) + tx / 32;
    let off = sbb + bloque * 0x800 + ((ty % 32) * 32 + tx % 32) * 2;
    u16::from_le_bytes([
        vram.get(off).copied().unwrap_or(0),
        vram.get(off + 1).copied().unwrap_or(0),
    ])
}

/// Índice de color del píxel (px, py) de un tile, con volteos aplicados.
fn indice_en_tile(
    vram: &[u8],
    cnt: u16,
    tile: usize,
    px: usize,
    py: usize,
    hflip: bool,
    vflip: bool,
) -> usize {
    let tx = if hflip { 7 - px } else { px };
    let ty = if vflip { 7 - py } else { py };
    let base = base_char(cnt);
    if cnt & 0x80 != 0 {
        let off = base + tile * 64 + ty * 8 + tx;
        vram.get(off).copied().unwrap_or(0) as usize
    } else {
        let off = base + tile * 32 + ty * 4 + tx / 2;
        let byte = vram.get(off).copied().unwrap_or(0) as usize;
        if tx % 2 == 0 {
            byte & 0x0F
        } else {
            byte >> 4
        }
    }
}

/// Un tile tiene datos si algún byte de su imagen es distinto de 0.
fn tile_con_datos(vram: &[u8], cnt: u16, tile: usize) -> bool {
    let tam = tam_tile(cnt);
    let ini = base_char(cnt) + tile * tam;
    (ini..ini + tam).any(|o| vram.get(o).copied().unwrap_or(0) != 0)
}

#[derive(Default)]
struct Medidas {
    pix_pantalla: usize,
    pix_entrada_no_cero: usize,
    entradas_no_cero: HashSet<u16>,
    pix_tile_con_datos: usize,
    tiles_referenciados: HashSet<usize>,
    pix_opacos: usize,
    pix_visibles: usize,
    colores: HashSet<u16>,
    lineas_cambio: usize,
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 4 {
        eprintln!("uso: ppu_contenido_bg <ruta.gba> <secuencia.txt> <cuadro>");
        std::process::exit(2);
    }
    let rom = match std::fs::read(&args[1]) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("ROM no encontrado: {}", args[1]);
            eprintln!("detalle: {e}");
            std::process::exit(1);
        }
    };
    let texto = match std::fs::read_to_string(&args[2]) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("secuencia no encontrada: {} ({e})", args[2]);
            std::process::exit(1);
        }
    };
    let objetivo: u32 = match args[3].parse() {
        Ok(n) => n,
        Err(_) => {
            eprintln!("el cuadro debe ser un número entero");
            std::process::exit(2);
        }
    };
    let tramos = match leer_secuencia(&texto) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("secuencia inválida: {e}");
            std::process::exit(1);
        }
    };

    // Ejecutar el juego hasta el cuadro pedido, con la captura activa solo durante ese cuadro.
    let mut m = gba_core::Machine::new(rom.clone());
    let mut cuadro: u32 = 0;
    let mut encontrado = false;
    'secuencia: for (cuadros, presionados) in tramos {
        m.bus.keys = 0x03FF & !presionados;
        for _ in 0..cuadros {
            if cuadro == objetivo {
                m.bus.capturar_linea = true;
            }
            m.run_frame(5_000_000);
            if cuadro == objetivo {
                m.bus.capturar_linea = false;
                encontrado = true;
                break 'secuencia;
            }
            cuadro += 1;
        }
    }
    if !encontrado {
        eprintln!("la secuencia no llega al cuadro {objetivo}");
        std::process::exit(1);
    }
    let capturas = &m.bus.capturas_linea;
    if capturas.len() != ALTO {
        eprintln!("se esperaban {ALTO} capturas y hay {}", capturas.len());
        std::process::exit(1);
    }

    let cap0 = &capturas[0];
    let disp0 = io16(&cap0.io, 0);
    let modo = disp0 & 7;
    let vram0 = &cap0.vram;
    let lineas_vram = capturas.iter().filter(|c| c.vram != *vram0).count();

    let mut lineas = vec![format!(
        "resumen cuadro {objetivo}: modo {modo}; lineas evaluadas {ALTO}; vram distinta de la linea 0 en {lineas_vram} lineas"
    )];

    for &bg in BGS.iter() {
        let cnt0 = io16(&cap0.io, BG_CNT[bg]);
        let hofs0 = (io16(&cap0.io, BG_HOFS[bg]) & 0x1FF) as usize;
        let vofs0 = (io16(&cap0.io, BG_VOFS[bg]) & 0x1FF) as usize;
        let tiles_con_datos = (0..1024)
            .filter(|&t| tile_con_datos(vram0, cnt0, t))
            .count();

        let mut med = Medidas::default();
        for cap in capturas.iter() {
            let cnt = io16(&cap.io, BG_CNT[bg]);
            let hofs = (io16(&cap.io, BG_HOFS[bg]) & 0x1FF) as usize;
            let vofs = (io16(&cap.io, BG_VOFS[bg]) & 0x1FF) as usize;
            if cnt != cnt0 || hofs != hofs0 || vofs != vofs0 {
                med.lineas_cambio += 1;
            }

            let vram = &cap.vram;
            let (ancho_tiles, alto_tiles) = tamano_mapa(cnt);
            let ancho_mapa = ancho_tiles * 8;
            let alto_mapa = alto_tiles * 8;
            let fondo = color_paleta(&cap.palette, 0);
            let bpp8 = cnt & 0x80 != 0;

            for x in 0..ANCHO {
                let mx = (x + hofs) % ancho_mapa;
                let my = (cap.linea + vofs) % alto_mapa;
                let entrada = entrada_mapa(vram, cnt, mx / 8, my / 8);
                let tile = (entrada & 0x3FF) as usize;
                let hflip = entrada & 0x400 != 0;
                let vflip = entrada & 0x800 != 0;
                let banco = if bpp8 { 0 } else { (entrada >> 12) as usize };

                med.pix_pantalla += 1;
                if tile != 0 {
                    med.pix_entrada_no_cero += 1;
                }
                if entrada != 0 {
                    med.entradas_no_cero.insert(entrada);
                }
                if tile_con_datos(vram, cnt, tile) {
                    med.pix_tile_con_datos += 1;
                    med.tiles_referenciados.insert(tile);
                }

                let idx = indice_en_tile(vram, cnt, tile, mx % 8, my % 8, hflip, vflip);
                if idx != 0 {
                    med.pix_opacos += 1;
                    let indice_paleta = if bpp8 { idx } else { banco * 16 + idx };
                    let color = color_paleta(&cap.palette, indice_paleta);
                    med.colores.insert(color);
                    if color != fondo {
                        med.pix_visibles += 1;
                    }
                }
            }
        }

        let activo = disp0 & BIT_BG[bg] != 0;
        lineas.push(format!(
            "bg BG{bg} activo: {} cnt: {cnt0:#06x} hofs: {hofs0} vofs: {vofs0} tiles con datos (0..1023): {tiles_con_datos} pix pantalla: {} pix entrada no cero: {} entradas visibles no cero: {} pix tile con datos: {} tiles referenciados con datos: {} pix opacos: {} pix visibles: {} pix opacos iguales al fondo: {} colores distintos: {} lineas con cambio: {}",
            activo as u8,
            med.pix_pantalla,
            med.pix_entrada_no_cero,
            med.entradas_no_cero.len(),
            med.pix_tile_con_datos,
            med.tiles_referenciados.len(),
            med.pix_opacos,
            med.pix_visibles,
            med.pix_opacos - med.pix_visibles,
            med.colores.len(),
            med.lineas_cambio,
        ));
    }

    // BG2, medido aparte (propuesta contenido-bg2-cuadro-1000). No entra en BGS, así que el
    // informe contenido-bg-cuadro-<N>.txt no cambia.
    // Aportación fuera de cobertura: misma regla que ppu_capa_ganadora (V0 != V1 y V2 == V3 en la PPU).
    let n_px = ANCHO * ALTO;
    let mut ppu: Vec<Vec<u16>> = vec![vec![0u16; n_px]; 4];
    let mut replay = gba_core::Machine::new(rom);
    for cap in capturas.iter() {
        let base = io16(&cap.io, 0);
        replay.bus.palette = cap.palette.clone();
        replay.bus.vram = cap.vram.clone();
        replay.bus.oam = cap.oam.clone();
        let ini = cap.linea * ANCHO;
        for v in 0..4 {
            let d = dispcnt_de(v, base);
            replay.bus.io = cap.io.clone();
            replay.bus.io[0] = (d & 0xFF) as u8;
            replay.bus.io[1] = (d >> 8) as u8;
            replay.bus.aff_ref = cap.aff_ref;
            replay.bus.aff_dirty = cap.aff_dirty;
            gba_core::ppu::render_scanline(&mut replay.bus, cap.linea);
            ppu[v][ini..ini + ANCHO].copy_from_slice(&replay.bus.framebuffer[ini..ini + ANCHO]);
        }
    }
    let mascara: Vec<bool> = (0..n_px)
        .map(|p| ppu[0][p] != ppu[1][p] && ppu[2][p] == ppu[3][p])
        .collect();
    let aportacion_total = mascara.iter().filter(|&&b| b).count();

    const BG2: usize = 2;
    let cnt0_bg2 = io16(&cap0.io, BG_CNT[BG2]);
    let tiles_con_datos_bg2 = (0..1024)
        .filter(|&t| tile_con_datos(vram0, cnt0_bg2, t))
        .count();
    let mut tiles_ref_bg2: HashSet<usize> = HashSet::new();
    let mut opacos_bg2 = 0usize;
    let mut opacos_bg2_dentro = 0usize;
    for cap in capturas.iter() {
        let cnt = io16(&cap.io, BG_CNT[BG2]);
        let hofs = (io16(&cap.io, BG_HOFS[BG2]) & 0x1FF) as usize;
        let vofs = (io16(&cap.io, BG_VOFS[BG2]) & 0x1FF) as usize;
        let vram = &cap.vram;
        let (ancho_tiles, alto_tiles) = tamano_mapa(cnt);
        let ancho_mapa = ancho_tiles * 8;
        let alto_mapa = alto_tiles * 8;
        for x in 0..ANCHO {
            let mx = (x + hofs) % ancho_mapa;
            let my = (cap.linea + vofs) % alto_mapa;
            let entrada = entrada_mapa(vram, cnt, mx / 8, my / 8);
            let tile = (entrada & 0x3FF) as usize;
            let hflip = entrada & 0x400 != 0;
            let vflip = entrada & 0x800 != 0;
            if tile_con_datos(vram, cnt, tile) {
                tiles_ref_bg2.insert(tile);
            }
            let idx = indice_en_tile(vram, cnt, tile, mx % 8, my % 8, hflip, vflip);
            if idx != 0 {
                opacos_bg2 += 1;
                if mascara[cap.linea * ANCHO + x] {
                    opacos_bg2_dentro += 1;
                }
            }
        }
    }
    let bg2_lineas = vec![
        format!("resumen cuadro {objetivo}: contenido de BG2 (medicion aparte)"),
        format!("bg2 activo: {}", (disp0 & BIT_BG[BG2] != 0) as u8),
        format!("bg2 cnt: {cnt0_bg2:#06x}"),
        format!("bg2 tiles con datos (0..1023): {tiles_con_datos_bg2}"),
        format!("bg2 tiles referenciados con datos: {}", tiles_ref_bg2.len()),
        format!("bg2 opacos total: {opacos_bg2}"),
        format!("bg2 opacos dentro aportacion: {opacos_bg2_dentro}"),
        format!("bg2 opacos fuera aportacion: {}", opacos_bg2 - opacos_bg2_dentro),
        format!("bg2 aportacion total: {aportacion_total}"),
    ];
    for l in &bg2_lineas {
        println!("{l}");
    }
    let _ = std::fs::create_dir_all("volcados");
    let ruta_bg2 = format!("volcados/contenido-bg2-cuadro-{objetivo}.txt");
    if let Err(e) = std::fs::write(&ruta_bg2, format!("{}\n", bg2_lineas.join("\n"))) {
        eprintln!("no se pudo escribir {ruta_bg2}: {e}");
    }

    for l in &lineas {
        println!("{l}");
    }
    let _ = std::fs::create_dir_all("volcados");
    let ruta = format!("volcados/contenido-bg-cuadro-{objetivo}.txt");
    if let Err(e) = std::fs::write(&ruta, format!("{}\n", lineas.join("\n"))) {
        eprintln!("no se pudo escribir {ruta}: {e}");
    }
}
