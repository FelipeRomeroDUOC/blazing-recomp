//! Experimento de docs/propuestas/2026-10-17_camino-rapido-contra-general-cuadro-1000.md.
//!
//! Con las capturas por línea del cuadro N (feature `perf-video`), redibuja cada línea con la
//! PPU por software en cuatro salidas:
//!   0: completa, camino por defecto (V0r);
//!   1: sin OBJ, camino por defecto (V1r);
//!   2: completa, camino general forzado (V0g);
//!   3: sin OBJ, camino general forzado (V1g).
//! Mide:
//!   control: V0r frente al juego (debe ser 0);
//!   D0: píxeles distintos entre V0r y V0g; D1: entre V1r y V1g (inconsistencia de caminos);
//!   aportación rápida: V0r frente a V1r; aportación general: V0g frente a V1g;
//!   líneas con diferencia: líneas donde D0 o D1 no son 0.
//! El camino general se fuerza con `gba_core::ppu::forzar_camino_general` y se restaura al final.
//!
//! Escribe volcados/camino-general-cuadro-<N>.txt con una línea por medida.
//!
//! Uso:
//!   cargo run --release --features proto --example ppu_camino_general -- <ruta.gba> <secuencia.txt> <cuadro>

/// Cuenta los píxeles que difieren entre dos trozos del mismo tamaño.

#[path = "comun/mod.rs"]
mod comun;

use comun::entrada::leer_secuencia;
use comun::gpu::{ALTO, ANCHO};
use comun::registro::BIT_OBJ;

fn distintos(a: &[u16], b: &[u16]) -> usize {
    a.iter().zip(b.iter()).filter(|(x, y)| x != y).count()
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 4 {
        eprintln!("uso: ppu_camino_general <ruta.gba> <secuencia.txt> <cuadro>");
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
    if capturas.len() != ALTO {
        eprintln!("se esperaban {ALTO} capturas y hay {}", capturas.len());
        std::process::exit(1);
    }

    // Cuatro salidas: [V0r, V1r, V0g, V1g].
    let mut salidas: Vec<Vec<u16>> = vec![vec![0u16; ANCHO * ALTO]; 4];
    let mut replay = gba_core::Machine::new(rom);
    for cap in capturas.iter() {
        let base = u16::from_le_bytes([cap.io[0], cap.io[1]]);
        replay.bus.palette = cap.palette.clone();
        replay.bus.vram = cap.vram.clone();
        replay.bus.oam = cap.oam.clone();
        for (bloque, general) in [(0usize, false), (2usize, true)] {
            gba_core::ppu::forzar_camino_general(general);
            for (k, disp) in [(0usize, base), (1usize, base & !BIT_OBJ)] {
                replay.bus.io = cap.io.clone();
                replay.bus.io[0] = (disp & 0xFF) as u8;
                replay.bus.io[1] = (disp >> 8) as u8;
                // La PPU avanza aff_ref al dibujar: cada salida parte del valor de la captura.
                replay.bus.aff_ref = cap.aff_ref;
                replay.bus.aff_dirty = cap.aff_dirty;
                gba_core::ppu::render_scanline(&mut replay.bus, cap.linea);

                let ini = cap.linea * ANCHO;
                salidas[bloque + k][ini..ini + ANCHO]
                    .copy_from_slice(&replay.bus.framebuffer[ini..ini + ANCHO]);
            }
        }
    }
    gba_core::ppu::forzar_camino_general(false);

    let control = distintos(&salidas[0], &fb_juego);
    let d0 = distintos(&salidas[0], &salidas[2]);
    let d1 = distintos(&salidas[1], &salidas[3]);
    let aportacion_rapida = distintos(&salidas[0], &salidas[1]);
    let aportacion_general = distintos(&salidas[2], &salidas[3]);
    let mut lineas_dif = 0usize;
    for l in 0..ALTO {
        let r = l * ANCHO..(l + 1) * ANCHO;
        if salidas[0][r.clone()] != salidas[2][r.clone()]
            || salidas[1][r.clone()] != salidas[3][r.clone()]
        {
            lineas_dif += 1;
        }
    }

    let lineas = vec![
        format!(
            "resumen cuadro {objetivo}: camino rápido vs juego = {control} píxeles distintos de {} (esperado 0)",
            fb_juego.len()
        ),
        format!("camino control: {control}"),
        format!("camino D0: {d0}"),
        format!("camino D1: {d1}"),
        format!("camino aportacion rapida: {aportacion_rapida}"),
        format!("camino aportacion general: {aportacion_general}"),
        format!("camino lineas con diferencia: {lineas_dif}"),
    ];
    for l in &lineas {
        println!("{l}");
    }

    let _ = std::fs::create_dir_all("volcados");
    let ruta = format!("volcados/camino-general-cuadro-{objetivo}.txt");
    if let Err(e) = std::fs::write(&ruta, format!("{}\n", lineas.join("\n"))) {
        eprintln!("no se pudo escribir {ruta}: {e}");
    }
}
