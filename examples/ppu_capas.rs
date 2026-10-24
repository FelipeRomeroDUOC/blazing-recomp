//! Prototipo 2 de docs/propuestas/2026-10-17_comparar-capas-del-cuadro-fijo.md.
//! Comprueba que una copia del estado de un cuadro (E/S, paleta, VRAM y OAM), dibujada con la
//! PPU por software fuera del juego, reproduce el framebuffer que produjo el juego en ese cuadro.
//!
//! Uso:
//!   cargo run --release --features proto --example ppu_capas -- <ruta.gba> <secuencia.txt> <cuadro>

#[path = "comun/mod.rs"]
mod comun;

use comun::entrada::leer_secuencia;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 4 {
        eprintln!("uso: ppu_capas <ruta.gba> <secuencia.txt> <cuadro>");
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

    // Ejecutar el juego igual que ppu_compare hasta el cuadro pedido, y guardar su estado.
    let mut m = gba_core::Machine::new(rom.clone());
    let mut cuadro: u32 = 0;
    let mut capturado = None;
    'secuencia: for (cuadros, presionados) in tramos {
        m.bus.keys = 0x03FF & !presionados;
        for _ in 0..cuadros {
            m.run_frame(5_000_000);
            if cuadro == objetivo {
                capturado = Some((
                    m.bus.framebuffer.clone(),
                    m.bus.io.clone(),
                    m.bus.palette.clone(),
                    m.bus.vram.clone(),
                    m.bus.oam.clone(),
                ));
                break 'secuencia;
            }
            cuadro += 1;
        }
    }
    let Some((fb_juego, io, paleta, vram, oam)) = capturado else {
        eprintln!("la secuencia no llega al cuadro {objetivo}");
        std::process::exit(1);
    };

    // Copia fuera del juego: mismo estado, dibujado línea a línea con la PPU por software.
    let mut copia = gba_core::Machine::new(rom);
    copia.bus.io = io;
    copia.bus.palette = paleta;
    copia.bus.vram = vram;
    copia.bus.oam = oam;
    for linea in 0..160 {
        gba_core::ppu::render_scanline(&mut copia.bus, linea);
    }

    let distintos = copia
        .bus
        .framebuffer
        .iter()
        .zip(fb_juego.iter())
        .filter(|(a, b)| a != b)
        .count();
    println!(
        "copia vs juego en el cuadro {objetivo}: {distintos} píxeles distintos de {} (esperado 0)",
        fb_juego.len()
    );
}
