//! Mide el coste de la PPU por software con el camino rápido y con el camino general forzado.
//!
//! Ejecuta la ROM con una secuencia de entrada y, en la ventana medida, acumula los
//! contadores de rendimiento del motor (`ppu_ns`, `compose_ns`) y cuenta cuántas líneas
//! fueron por cada camino. Sirve para cuantificar el precio de que una línea con la
//! ventana de sprites (`OBJWIN`) tenga que ir al camino general por píxel.
//!
//! La misma secuencia se ejecuta dos veces: una con el camino rápido (el de por defecto) y
//! otra con `gba_core::ppu::forzar_camino_general(true)`, que manda todas las líneas al
//! camino por píxel, que es el del upstream fijado.
//!
//! Uso:
//!   cargo run --release --features proto --example ppu_tiempos -- <ruta.gba> <secuencia.txt> <cuadros_calentamiento> <cuadros_medidos>

use std::time::Instant;

#[path = "comun/mod.rs"]
mod comun;

use comun::entrada::leer_secuencia;

/// Lo que se mide en la ventana medida.
struct Medida {
    /// Nanoseconds que la PPU tardó en componer las líneas medidas.
    ppu_ns: u64,
    /// Nanoseconds dentro de la composición (prioridades, ventanas y efectos).
    compose_ns: u64,
    /// Líneas que fueron por la vía rápida.
    lineas_rectas: u64,
    /// Líneas que fueron por la vía por tramos del alpha.
    lineas_alpha_tramos: u64,
    /// Líneas que fueron por el camino general por píxel.
    lineas_camino_general: u64,
    /// Líneas descartadas de la vía rápida por la ventana de sprites.
    descartes_ventana: u64,
    /// Líneas descartadas de la vía rápida por un sprite translúcido.
    descartes_semi: u64,
    /// Líneas descartadas de la vía rápida por un efecto que exige la segunda capa.
    descartes_mezcla: u64,
    /// Reloj de pared de la ventana medida.
    reloj: std::time::Duration,
}

/// Ejecuta la secuencia y mide la ventana `[calentamiento, calentamiento + medidos)`.
fn medir(
    rom: &[u8],
    tramos: &[(u32, u16)],
    calentamiento: u32,
    medidos: u32,
    general: bool,
) -> Result<Medida, String> {
    gba_core::ppu::forzar_camino_general(general);
    let mut m = gba_core::Machine::new(rom.to_vec());
    let mut cuadro: u32 = 0;
    let mut en_medida = false;
    let mut reloj = None;
    let mut c = m.take_perf_counters();
    'secuencia: for &(cuadros, presionados) in tramos {
        m.bus.keys = 0x03FF & !presionados;
        for _ in 0..cuadros {
            if cuadro == calentamiento {
                c = m.take_perf_counters();
                reloj = Some(Instant::now());
                en_medida = true;
            }
            m.run_frame(5_000_000);
            cuadro += 1;
            if cuadro == calentamiento + medidos {
                break 'secuencia;
            }
        }
    }
    let reloj = reloj.ok_or_else(|| {
        format!("la secuencia no llega al cuadro {calentamiento}")
    })?;
    let c = m.take_perf_counters();
    gba_core::ppu::forzar_camino_general(false);
    if !en_medida {
        return Err("no se midió ninguna ventana".to_string());
    }
    Ok(Medida {
        ppu_ns: c.ppu_ns,
        compose_ns: c.compose_ns,
        lineas_rectas: c.lineas_rectas,
        lineas_alpha_tramos: c.lineas_alpha_tramos,
        lineas_camino_general: c.lineas_camino_general,
        descartes_ventana: c.descartes_ventana,
        descartes_semi: c.descartes_semi,
        descartes_mezcla: c.descartes_mezcla,
        reloj: reloj.elapsed(),
    })
}

fn ms(ns: u64) -> f64 {
    ns as f64 / 1_000_000.0
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 5 {
        eprintln!("uso: ppu_tiempos <ruta.gba> <secuencia.txt> <cuadros_calentamiento> <cuadros_medidos>");
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
    let calentamiento: u32 = match args[3].parse() {
        Ok(n) => n,
        Err(_) => {
            eprintln!("los cuadros de calentamiento deben ser un número entero");
            std::process::exit(2);
        }
    };
    let medidos: u32 = match args[4].parse() {
        Ok(n) => n,
        Err(_) => {
            eprintln!("los cuadros medidos deben ser un número entero");
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

    let rapida = match medir(&rom, &tramos, calentamiento, medidos, false) {
        Ok(m) => m,
        Err(e) => {
            eprintln!("no se pudo medir: {e}");
            std::process::exit(1);
        }
    };
    let general = match medir(&rom, &tramos, calentamiento, medidos, true) {
        Ok(m) => m,
        Err(e) => {
            eprintln!("no se pudo medir: {e}");
            std::process::exit(1);
        }
    };

    println!("ventana medida: cuadros {calentamiento}..{}", calentamiento + medidos);
    println!(
        "  camino rápido:  PPU {:.2} ms, composición {:.2} ms, reloj {:.2} ms",
        ms(rapida.ppu_ns),
        ms(rapida.compose_ns),
        rapida.reloj.as_secs_f64() * 1000.0
    );
    println!(
        "    líneas: rectas {}, alpha por tramos {}, camino general {}, descartes (ventana {}, semi {}, mezcla {})",
        rapida.lineas_rectas,
        rapida.lineas_alpha_tramos,
        rapida.lineas_camino_general,
        rapida.descartes_ventana,
        rapida.descartes_semi,
        rapida.descartes_mezcla
    );
    println!(
        "  camino general: PPU {:.2} ms, composición {:.2} ms, reloj {:.2} ms",
        ms(general.ppu_ns),
        ms(general.compose_ns),
        general.reloj.as_secs_f64() * 1000.0
    );
    println!(
        "    líneas: rectas {}, alpha por tramos {}, camino general {}, descartes (ventana {}, semi {}, mezcla {})",
        general.lineas_rectas,
        general.lineas_alpha_tramos,
        general.lineas_camino_general,
        general.descartes_ventana,
        general.descartes_semi,
        general.descartes_mezcla
    );
    let diferencia = general.ppu_ns as f64 - rapida.ppu_ns as f64;
    let porcentaje = if rapida.ppu_ns > 0 {
        100.0 * diferencia / rapida.ppu_ns as f64
    } else {
        0.0
    };
    println!(
        "  el camino general cuesta {diferencia:.0} ns más de PPU ({porcentaje:.1} % del camino rápido)"
    );
}
