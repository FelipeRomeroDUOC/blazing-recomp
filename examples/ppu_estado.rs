//! Experimento de docs/propuestas/2026-10-17_buscar-estado-faltante-cuadro-1000.md.
//!
//! Mide cuántos píxeles del cuadro N difieren del framebuffer del juego, al dibujar
//! una copia del estado con la PPU por software fuera del juego:
//!   E1: copia parcial (E/S, paleta, VRAM y OAM), igual que ppu_capas.
//!   E2: copia de todos los campos públicos de MemMap que puede leer el renderizado.
//!
//! Rust no deja que ppu.rs lea campos privados de MemMap, así que E2 cubre todo el
//! estado que usa render_scanline. Escribe volcados/estado-cuadro-<N>.txt.
//!
//! Uso:
//!   cargo run --release --features proto --example ppu_estado -- <ruta.gba> <secuencia.txt> <cuadro>

/// Cuenta los píxeles que difieren entre dos framebuffers del mismo tamaño.

#[path = "comun/mod.rs"]
mod comun;

use comun::entrada::leer_secuencia;

fn distintos(a: &[u16], b: &[u16]) -> usize {
    a.iter().zip(b.iter()).filter(|(x, y)| x != y).count()
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 4 {
        eprintln!("uso: ppu_estado <ruta.gba> <secuencia.txt> <cuadro>");
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

    // Ejecutar el juego igual que ppu_capas hasta el cuadro pedido.
    let mut m = gba_core::Machine::new(rom.clone());
    let mut cuadro: u32 = 0;
    let mut encontrado = false;
    'secuencia: for (cuadros, presionados) in tramos {
        m.bus.keys = 0x03FF & !presionados;
        for _ in 0..cuadros {
            m.run_frame(5_000_000);
            if cuadro == objetivo {
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

    // E1: copia parcial (la misma que en ppu_capas).
    let mut e1 = gba_core::Machine::new(rom.clone());
    e1.bus.io = m.bus.io.clone();
    e1.bus.palette = m.bus.palette.clone();
    e1.bus.vram = m.bus.vram.clone();
    e1.bus.oam = m.bus.oam.clone();
    for linea in 0..160 {
        gba_core::ppu::render_scanline(&mut e1.bus, linea);
    }
    let d1 = distintos(&e1.bus.framebuffer, &fb_juego);

    // E2: copia de todos los campos públicos de MemMap (el framebuffer es salida, no se copia).
    let mut e2 = gba_core::Machine::new(rom);
    e2.bus.bios = m.bus.bios.clone();
    e2.bus.ewram = m.bus.ewram.clone();
    e2.bus.iwram = m.bus.iwram.clone();
    e2.bus.io = m.bus.io.clone();
    e2.bus.palette = m.bus.palette.clone();
    e2.bus.vram = m.bus.vram.clone();
    e2.bus.oam = m.bus.oam.clone();
    e2.bus.sram = m.bus.sram.clone();
    e2.bus.aff_ref = m.bus.aff_ref;
    e2.bus.aff_dirty = m.bus.aff_dirty;
    e2.bus.keys = m.bus.keys;
    e2.bus.clock = m.bus.clock;
    e2.bus.frames = m.bus.frames;
    e2.bus.reg_ie = m.bus.reg_ie;
    e2.bus.reg_if = m.bus.reg_if;
    e2.bus.ime = m.bus.ime;
    e2.bus.halted = m.bus.halted;
    e2.bus.intr_wait_armed = m.bus.intr_wait_armed;
    e2.bus.bios_open = m.bus.bios_open;
    e2.bus.real_bios = m.bus.real_bios;
    e2.bus.pc_in_bios = m.bus.pc_in_bios;
    for linea in 0..160 {
        gba_core::ppu::render_scanline(&mut e2.bus, linea);
    }
    let d2 = distintos(&e2.bus.framebuffer, &fb_juego);

    let resultado = format!(
        "cuadro {objetivo}: E1 copia parcial = {d1} píxeles distintos; E2 copia completa = {d2} píxeles distintos (de {})",
        fb_juego.len()
    );
    println!("{resultado}");
    let _ = std::fs::create_dir_all("volcados");
    let ruta = format!("volcados/estado-cuadro-{objetivo}.txt");
    if let Err(e) = std::fs::write(&ruta, format!("{resultado}\n")) {
        eprintln!("no se pudo escribir {ruta}: {e}");
    }
}
