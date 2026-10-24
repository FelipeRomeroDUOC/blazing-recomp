//! Experimento de docs/propuestas/2026-10-17_capas-por-aportacion-cuadro-1000.md.
//!
//! Con las capturas por línea del cuadro N (feature `perf-video`), redibuja cada línea
//! con la PPU por software en 12 variantes de DISPCNT:
//!   0: completa; 1..=5: sin BG0, BG1, BG2, BG3, OBJ; 6..=10: solo BG0, BG1, BG2, BG3, OBJ;
//!   11: nada (solo el color de fondo).
//!
//! Medidas por capa:
//!   aportación = píxeles que cambian al quitar la capa (completa vs sin la capa);
//!   cobertura  = píxeles donde la capa sola difiere del fondo (solo la capa vs nada).
//! La variante completa se compara con el framebuffer del juego como control.
//!
//! Escribe volcados/capas-cuadro-<N>.txt: un resumen y cinco filas con prefijo `capa `.
//!
//! Uso:
//!   cargo run --release --features proto --example ppu_capas_linea -- <ruta.gba> <secuencia.txt> <cuadro>

/// Capas medidas con su bit en DISPCNT: BG0..BG3 en `0x100 << n`, OBJ en `0x1000`.

#[path = "comun/mod.rs"]
mod comun;

use comun::entrada::leer_secuencia;
use comun::registro::BITS_CAPAS;

const CAPAS: [(&str, u16); 5] = [
    ("BG0", 0x0100),
    ("BG1", 0x0200),
    ("BG2", 0x0400),
    ("BG3", 0x0800),
    ("OBJ", 0x1000),
];
const VARIANTES: usize = 12;

/// Cuenta los píxeles que difieren entre dos trozos del mismo tamaño.
fn distintos(a: &[u16], b: &[u16]) -> usize {
    a.iter().zip(b.iter()).filter(|(x, y)| x != y).count()
}

/// DISPCNT de cada variante, a partir del DISPCNT original de la línea.
fn dispcnt_de(variante: usize, base: u16) -> u16 {
    match variante {
        0 => base,
        1..=5 => base & !CAPAS[variante - 1].1,
        6..=10 => (base & !BITS_CAPAS) | CAPAS[variante - 6].1,
        _ => base & !BITS_CAPAS,
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 4 {
        eprintln!("uso: ppu_capas_linea <ruta.gba> <secuencia.txt> <cuadro>");
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

    // Redibujar cada línea en las 12 variantes, con la PPU por software.
    let mut cuadros_var: Vec<Vec<u16>> = vec![vec![0u16; 240 * 160]; VARIANTES];
    let mut modos = std::collections::BTreeSet::new();
    let mut activa = [false; 5];
    let mut lineas_activa = [0usize; 5];
    let mut replay = gba_core::Machine::new(rom);
    for cap in capturas.iter() {
        let base = u16::from_le_bytes([cap.io[0], cap.io[1]]);
        modos.insert(base & 7);
        for (i, (_, bit)) in CAPAS.iter().enumerate() {
            if base & bit != 0 {
                activa[i] = true;
                lineas_activa[i] += 1;
            }
        }
        replay.bus.palette = cap.palette.clone();
        replay.bus.vram = cap.vram.clone();
        replay.bus.oam = cap.oam.clone();
        for v in 0..VARIANTES {
            let d = dispcnt_de(v, base);
            replay.bus.io = cap.io.clone();
            replay.bus.io[0] = (d & 0xFF) as u8;
            replay.bus.io[1] = (d >> 8) as u8;
            // La PPU avanza aff_ref al dibujar: cada variante parte del valor de la captura.
            replay.bus.aff_ref = cap.aff_ref;
            replay.bus.aff_dirty = cap.aff_dirty;
            gba_core::ppu::render_scanline(&mut replay.bus, cap.linea);

            let ini = cap.linea * 240;
            cuadros_var[v][ini..ini + 240]
                .copy_from_slice(&replay.bus.framebuffer[ini..ini + 240]);
        }
    }

    let completa = &cuadros_var[0];
    let d_juego = distintos(completa, &fb_juego);
    let nada = &cuadros_var[11];
    let mut filas = Vec::new();
    for (i, (nombre, _)) in CAPAS.iter().enumerate() {
        let aportacion = distintos(&cuadros_var[1 + i], completa);
        let cobertura = distintos(&cuadros_var[6 + i], nada);
        filas.push(format!(
            "capa {nombre} activa {} lineas_activa {} aportacion {aportacion} cobertura {cobertura}",
            activa[i] as u8, lineas_activa[i]
        ));
    }

    let modos_txt: Vec<String> = modos.iter().map(|m| m.to_string()).collect();
    let aviso = if capturas.len() == 160 {
        String::new()
    } else {
        format!(" (AVISO: se esperaban 160 capturas y hay {})", capturas.len())
    };
    let resumen = format!(
        "resumen cuadro {objetivo}: variante completa vs juego = {d_juego} píxeles distintos de {} (esperado 0); modos de vídeo: {}{aviso}",
        fb_juego.len(),
        modos_txt.join(", ")
    );
    println!("{resumen}");
    for f in &filas {
        println!("{f}");
    }

    let _ = std::fs::create_dir_all("volcados");
    let ruta = format!("volcados/capas-cuadro-{objetivo}.txt");
    let contenido = format!("{resumen}\n{}\n", filas.join("\n"));
    if let Err(e) = std::fs::write(&ruta, contenido) {
        eprintln!("no se pudo escribir {ruta}: {e}");
    }
}
