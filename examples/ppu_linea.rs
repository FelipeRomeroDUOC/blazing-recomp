//! Experimento de docs/propuestas/2026-10-17_captura-por-linea-cuadro-1000.md.
//!
//! Durante el cuadro N, el juego guarda el estado de cada línea visible justo antes
//! de dibujarla (campo `capturas_linea`, solo con la feature `perf-video`). Después
//! cada línea se redibuja desde su propia copia con la PPU por software, en una
//! máquina auxiliar, y se compara con el framebuffer del juego.
//!
//! Escribe volcados/replay-cuadro-<N>.txt: un resumen y una fila por línea visible.
//!
//! Uso:
//!   cargo run --release --features proto --example ppu_linea -- <ruta.gba> <secuencia.txt> <cuadro>

/// Cuenta los píxeles que difieren entre dos trozos del mismo tamaño.

#[path = "comun/mod.rs"]
mod comun;

use comun::entrada::leer_secuencia;

fn distintos(a: &[u16], b: &[u16]) -> usize {
    a.iter().zip(b.iter()).filter(|(x, y)| x != y).count()
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 4 {
        eprintln!("uso: ppu_linea <ruta.gba> <secuencia.txt> <cuadro>");
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
    let ancho: usize = 240; // ancho visible de la GBA

    // Redibujar cada línea desde su propia copia, con la PPU por software.
    let mut replay = gba_core::Machine::new(rom);
    let mut filas = Vec::new();
    let mut total = 0usize;
    let mut lineas_con_diferencias = 0usize;
    for cap in capturas.iter() {
        replay.bus.io = cap.io.clone();
        replay.bus.palette = cap.palette.clone();
        replay.bus.vram = cap.vram.clone();
        replay.bus.oam = cap.oam.clone();
        replay.bus.aff_ref = cap.aff_ref;
        replay.bus.aff_dirty = cap.aff_dirty;
        gba_core::ppu::render_scanline(&mut replay.bus, cap.linea);

        let ini = cap.linea * ancho;
        let fin = ini + ancho;
        let d = distintos(&replay.bus.framebuffer[ini..fin], &fb_juego[ini..fin]);
        total += d;
        if d > 0 {
            lineas_con_diferencias += 1;
        }
        filas.push(format!("linea {:3} distintos {}", cap.linea, d));
    }

    let aviso = if capturas.len() == 160 {
        String::new()
    } else {
        format!(" (AVISO: se esperaban 160 capturas y hay {})", capturas.len())
    };
    let resumen = format!(
        "resumen cuadro {objetivo}: capturas {}, píxeles distintos {total} de {} en {lineas_con_diferencias} líneas (esperado 0){aviso}",
        capturas.len(),
        fb_juego.len()
    );
    println!("{resumen}");

    let _ = std::fs::create_dir_all("volcados");
    let ruta = format!("volcados/replay-cuadro-{objetivo}.txt");
    let contenido = format!("{resumen}\n{}\n", filas.join("\n"));
    if let Err(e) = std::fs::write(&ruta, contenido) {
        eprintln!("no se pudo escribir {ruta}: {e}");
    }
}
