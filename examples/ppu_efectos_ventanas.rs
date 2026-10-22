//! Experimento de docs/propuestas/2026-10-17_diagnostico-efectos-y-ventanas-cuadro-1000.md.
//!
//! Con las capturas por línea del cuadro N (feature `perf-video`), redibuja cada línea con
//! la PPU por software en 14 variantes de DISPCNT y BLDCNT. Mide:
//!   - cuánto aporta OBJ con los efectos de color, con el modo de efecto apagado, y con
//!     BLDCNT = 0 (que apaga también la semitransparencia de sprites);
//!   - cuánto aportan BG0, BG1 y BG3 cuando las ventanas están apagadas, y cuánto cubren
//!     solos sin ventanas.
//!
//! Escribe volcados/efectos-ventanas-cuadro-<N>.txt.
//!
//! Uso:
//!   cargo run --release --features proto --example ppu_efectos_ventanas -- <ruta.gba> <secuencia.txt> <cuadro>

#[path = "comun/mod.rs"]
mod comun;

use comun::entrada::leer_secuencia;
use comun::registro::{BITS_CAPAS, BIT_OBJ};

const BIT_BG: [u16; 4] = [0x0100, 0x0200, 0x0400, 0x0800];
const BITS_VENTANAS: u16 = 0xE000;
/// Modo de efecto de BLDCNT (bits 6 y 7). Los objetivos (bits 0 a 5 y 8 a 13) no se tocan.
const MODO_EFECTO: u16 = 0x00C0;
const VARIANTES: usize = 14;

/// Cuenta los píxeles que difieren entre dos trozos del mismo tamaño.
fn distintos(a: &[u16], b: &[u16]) -> usize {
    a.iter().zip(b.iter()).filter(|(x, y)| x != y).count()
}

/// DISPCNT y BLDCNT de cada variante, a partir de los de la línea original.
///  0: completa; 1: modo de efecto apagado; 2: sin OBJ, modo apagado; 3: BLDCNT = 0;
///  4: sin OBJ, BLDCNT = 0; 5: sin ventanas; 6..=8: sin BG0, BG1, BG3 (sin ventanas);
///  9..=11: solo BG0, BG1, BG3 (sin ventanas); 12: nada (sin ventanas);
///  13: sin OBJ, configuración original.
fn configuracion(v: usize, disp: u16, bl: u16) -> (u16, u16) {
    let sin_ventanas = disp & !BITS_VENTANAS;
    let nada = sin_ventanas & !BITS_CAPAS;
    match v {
        0 => (disp, bl),
        1 => (disp, bl & !MODO_EFECTO),
        2 => (disp & !BIT_OBJ, bl & !MODO_EFECTO),
        3 => (disp, 0),
        4 => (disp & !BIT_OBJ, 0),
        5 => (sin_ventanas, bl),
        6 => (sin_ventanas & !BIT_BG[0], bl),
        7 => (sin_ventanas & !BIT_BG[1], bl),
        8 => (sin_ventanas & !BIT_BG[3], bl),
        9 => (nada | BIT_BG[0], bl),
        10 => (nada | BIT_BG[1], bl),
        11 => (nada | BIT_BG[3], bl),
        12 => (nada, bl),
        _ => (disp & !BIT_OBJ, bl),
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 4 {
        eprintln!("uso: ppu_efectos_ventanas <ruta.gba> <secuencia.txt> <cuadro>");
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
    let fb_juego = m.bus.framebuffer.clone();
    let capturas = &m.bus.capturas_linea;

    let mut variantes: Vec<Vec<u16>> = vec![vec![0u16; 240 * 160]; VARIANTES];
    let mut lineas_ventanas = 0usize;
    let mut lineas_bldcnt = 0usize;
    let mut replay = gba_core::Machine::new(rom);
    for cap in capturas.iter() {
        let disp = u16::from_le_bytes([cap.io[0], cap.io[1]]);
        let bl = u16::from_le_bytes([cap.io[0x50], cap.io[0x51]]);
        if disp & BITS_VENTANAS != 0 {
            lineas_ventanas += 1;
        }
        if bl != 0 {
            lineas_bldcnt += 1;
        }
        replay.bus.palette = cap.palette.clone();
        replay.bus.vram = cap.vram.clone();
        replay.bus.oam = cap.oam.clone();
        for v in 0..VARIANTES {
            let (d, b) = configuracion(v, disp, bl);
            replay.bus.io = cap.io.clone();
            replay.bus.io[0] = (d & 0xFF) as u8;
            replay.bus.io[1] = (d >> 8) as u8;
            replay.bus.io[0x50] = (b & 0xFF) as u8;
            replay.bus.io[0x51] = (b >> 8) as u8;
            // La PPU avanza aff_ref al dibujar: cada variante parte del valor de la captura.
            replay.bus.aff_ref = cap.aff_ref;
            replay.bus.aff_dirty = cap.aff_dirty;
            gba_core::ppu::render_scanline(&mut replay.bus, cap.linea);

            let ini = cap.linea * 240;
            variantes[v][ini..ini + 240].copy_from_slice(&replay.bus.framebuffer[ini..ini + 240]);
        }
    }

    let d = |a: usize, b: usize| distintos(&variantes[a], &variantes[b]);
    let control = distintos(&variantes[0], &fb_juego);
    let obj_con_efectos = d(0, 13);
    let obj_modo_apagado = d(1, 2);
    let obj_bldcnt_cero = d(3, 4);
    let modo_total = d(0, 1);
    let bldcnt_total = d(0, 3);
    let ventanas_total = d(0, 5);

    let mut lineas = vec![
        format!(
            "resumen cuadro {objetivo}: variante completa vs juego = {control} píxeles distintos de {} (esperado 0); líneas con ventanas: {lineas_ventanas}; líneas con BLDCNT: {lineas_bldcnt}",
            fb_juego.len()
        ),
        format!("efectos OBJ con efectos: {obj_con_efectos}"),
        format!("efectos OBJ modo apagado: {obj_modo_apagado}"),
        format!("efectos OBJ sin efectos (BLDCNT = 0): {obj_bldcnt_cero}"),
        format!("efectos modo total (completa vs modo apagado): {modo_total}"),
        format!("efectos BLDCNT = 0 total (completa vs BLDCNT = 0): {bldcnt_total}"),
    ];
    // BG0 usa la variante 6 (aportación) y 9 (cobertura); BG1, 7 y 10; BG3, 8 y 11.
    for (nombre, sin, solo) in [("BG0", 6, 9), ("BG1", 7, 10), ("BG3", 8, 11)] {
        lineas.push(format!(
            "ventanas {nombre} aportacion_sin_ventanas {} cobertura_sin_ventanas {}",
            d(5, sin),
            d(solo, 12)
        ));
    }
    lineas.push(format!("ventanas lineas con ventanas activas: {lineas_ventanas}"));
    lineas.push(format!("ventanas diferencia completa vs sin ventanas: {ventanas_total}"));

    for l in &lineas {
        println!("{l}");
    }
    let _ = std::fs::create_dir_all("volcados");
    let ruta = format!("volcados/efectos-ventanas-cuadro-{objetivo}.txt");
    if let Err(e) = std::fs::write(&ruta, format!("{}\n", lineas.join("\n"))) {
        eprintln!("no se pudo escribir {ruta}: {e}");
    }
}
