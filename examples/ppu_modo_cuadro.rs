//! Diagnóstico: vuelca el estado de la PPU (modo de fondo, scrolls, ventanas,
//! unidad de mezcla) de un cuadro concreto de la ROM.
//!
//! Sirve para localizar por qué la PPU nativa difiere de la referencia por
//! software: el modo de fondo determina qué capas dibuja cada implementación.
//!
//! Uso:
//!   cargo run --release --features proto --example ppu_modo_cuadro -- <ruta.gba> <secuencia.txt> <cuadro>

#[path = "comun/mod.rs"]
mod comun;

use comun::entrada::leer_secuencia;
use comun::registro::io16;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 4 {
        eprintln!("uso: ppu_modo_cuadro <ruta.gba> <secuencia.txt> <cuadro>");
        std::process::exit(2);
    }
    let rom = match std::fs::read(&args[1]) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("ROM no encontrada: {} ({e})", args[1]);
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

    let mut m = gba_core::Machine::new(rom);
    let mut cuadro: u32 = 0;
    let mut io_fin: Vec<u8> = Vec::new();
    'secuencia: for (cuadros, presionados) in tramos {
        m.bus.keys = 0x03FF & !presionados;
        for _ in 0..cuadros {
            m.run_frame(5_000_000);
            if cuadro == objetivo {
                io_fin = m.bus.io.clone();
                break 'secuencia;
            }
            cuadro += 1;
        }
    }
    if io_fin.is_empty() {
        eprintln!("la secuencia no llega al cuadro {objetivo}");
        std::process::exit(1);
    }

    let dispcnt = io16(&io_fin, 0x00);
    let modo = dispcnt & 0x07;
    println!("cuadro {objetivo}");
    println!("  DISPCNT = 0x{dispcnt:04X}  modo = {modo}");
    for (n, bit) in [(0, 0x0100), (1, 0x0200), (2, 0x0400), (3, 0x0800)] {
        let activo = dispcnt & bit != 0;
        let control = io16(&io_fin, 0x08 + n * 2);
        let hofs = io16(&io_fin, 0x10 + n * 4);
        let vofs = io16(&io_fin, 0x12 + n * 4);
        println!(
            "  BG{n}: activo={} control=0x{control:04X} hofs={hofs} vofs={vofs}",
            if activo { "si" } else { "no" }
        );
    }
    println!("  OBJ: hofs={} vofs={}", io16(&io_fin, 0x28), io16(&io_fin, 0x2A));
    let bldcnt = io16(&io_fin, 0x50);
    println!(
        "  BLDCNT = 0x{bldcnt:04X}  modo_efecto={}  1er_objetivo=0x{:02X}  2o_objetivo=0x{:02X}",
        (bldcnt >> 6) & 3,
        bldcnt & 0x3F,
        (bldcnt >> 8) & 0x3F
    );
    println!("  BLDALPHA = 0x{:04X}  BLDY = 0x{:04X}", io16(&io_fin, 0x52), io16(&io_fin, 0x54));
    println!(
        "  ventanas: win0_on={} win1_on={} objwin_on={}  win0v=0x{:04X} win1v=0x{:04X}",
        dispcnt & 0x2000 != 0,
        dispcnt & 0x4000 != 0,
        dispcnt & 0x8000 != 0,
        io16(&io_fin, 0x44),
        io16(&io_fin, 0x46)
    );
    println!("  WININ = 0x{:04X}  WINOUT = 0x{:04X}", io16(&io_fin, 0x48), io16(&io_fin, 0x4A));
}
