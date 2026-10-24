//! Arnés de comparación (docs/propuestas/2026-10-16_arnes-comparacion-shim-vs-software.md, paso 3).
//! Ejecuta el juego una sola vez siguiendo una secuencia de botones y, en cada cuadro,
//! compara la salida de la PPU por software con la del shim GPU. No cambia la ruta del juego.
//!
//! Uso:
//!   cargo run --release --features proto --example ppu_compare -- <ruta.gba> <secuencia.txt> [informe.csv]

use std::fmt::Write as _;

#[path = "comun/mod.rs"]
mod comun;

use comun::color::{TOLERANCIA, software_a_rgb};
use comun::entrada::leer_tramos;
use comun::gpu::Gpu;

const W: u32 = 240;
const H: u32 = 160;

struct Comparacion {
    distintos: u64,
    primera_linea: Option<u32>,
}

/// Compara dos imágenes RGB sin relleno. Un píxel es distinto si algún canal difiere más de `TOLERANCIA`.
/// Devuelve también la primera fila con diferencias.
fn compara(sw: &[u8], hw: &[u8]) -> Comparacion {
    let mut distintos = 0u64;
    let mut primera_linea = None;
    for y in 0..H {
        let mut fila_distinta = false;
        for x in 0..W {
            let o = ((y * W + x) * 3) as usize;
            let distinto = (0..3).any(|k| sw[o + k].abs_diff(hw[o + k]) > TOLERANCIA);
            if distinto {
                distintos += 1;
                fila_distinta = true;
            }
        }
        if fila_distinta && primera_linea.is_none() {
            primera_linea = Some(y);
        }
    }
    Comparacion { distintos, primera_linea }
}

/// Guarda un cuadro como RGB crudo (240 x 160 x 3 bytes) en la carpeta `volcados`.
fn volca(cuadro: u32, origen: &str, rgb: &[u8]) {
    let _ = std::fs::create_dir_all("volcados");
    let ruta = format!("volcados/cuadro-{cuadro}-{origen}.rgb");
    match std::fs::write(&ruta, rgb) {
        Ok(()) => println!("volcado: {ruta}"),
        Err(e) => eprintln!("no se pudo guardar {ruta}: {e}"),
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let (Some(ruta_rom), Some(ruta_secuencia)) = (args.get(1), args.get(2)) else {
        eprintln!("uso: ppu_compare <ruta.gba> <secuencia.txt> [informe.csv]");
        std::process::exit(2);
    };
    let ruta_informe = args
        .get(3)
        .cloned()
        .unwrap_or_else(|| "ppu-compare-informe.csv".to_string());

    // Argumentos desde el 4.º en adelante: números de cuadro a volcar como imagen cruda.
    let volcar: Vec<u32> = args.iter().skip(4).filter_map(|s| s.parse().ok()).collect();

    let rom = match std::fs::read(ruta_rom) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("ROM no encontrado: {ruta_rom}");
            eprintln!("detalle: {e}");
            std::process::exit(1);
        }
    };
    let texto = match std::fs::read_to_string(ruta_secuencia) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("secuencia no encontrada: {ruta_secuencia} ({e})");
            std::process::exit(1);
        }
    };
    let tramos = match leer_tramos(&texto) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("secuencia inválida: {e}");
            std::process::exit(1);
        }
    };
    let total: u32 = tramos.iter().map(|t| t.cuadros).sum();
    if total == 0 {
        eprintln!("la secuencia no tiene cuadros");
        std::process::exit(1);
    }

    let mut gpu = match Gpu::nueva() {
        Ok(g) => g,
        Err(e) => {
            eprintln!("{e}");
            std::process::exit(1);
        }
    };

    let mut m = gba_core::Machine::new(rom);
    let mut informe = String::from("cuadro,tramo,botones,pct_distinto,primera_linea_distinta,dispcnt,bldcnt,bldalpha,bldy,escrituras_video,escrituras_es,lineas_con_cambio,lineas_vram,lineas_oam\n");
    let mut cuadro: u32 = 0;
    let mut con_diferencias: u32 = 0;
    let mut distintos_total: u64 = 0;
    let t0 = std::time::Instant::now();

    for (indice, tramo) in tramos.iter().enumerate() {
        // KEYINPUT es activo en bajo: 0x03FF = sin botones.
        m.bus.keys = 0x03FF & !tramo.presionados;
        for _ in 0..tramo.cuadros {
            m.bus.perf_counters.mascara_lineas = [0; 3];
            m.bus.perf_counters.mascara_vram = [0; 3];
            m.bus.perf_counters.mascara_oam = [0; 3];
            let antes = m.bus.perf_counters.escrituras_video;
            let antes_es = m.bus.perf_counters.escrituras_es;
            let antes_lineas = m.bus.perf_counters.lineas_con_cambio;
            let antes_vram = m.bus.perf_counters.lineas_vram_cambio;
            let antes_oam = m.bus.perf_counters.lineas_oam_cambio;
            m.run_frame(5_000_000);
            let escrituras = m.bus.perf_counters.escrituras_video - antes;
            let escrituras_es = m.bus.perf_counters.escrituras_es - antes_es;
            let lineas_con_cambio = m.bus.perf_counters.lineas_con_cambio - antes_lineas;
            let lineas_vram = m.bus.perf_counters.lineas_vram_cambio - antes_vram;
            let lineas_oam = m.bus.perf_counters.lineas_oam_cambio - antes_oam;
            if m.bus.framebuffer.len() != (W * H) as usize {
                eprintln!("framebuffer inesperado: {} píxeles", m.bus.framebuffer.len());
                std::process::exit(1);
            }
            let sw = software_a_rgb(&m.bus.framebuffer);
            let hw = gpu.renderiza(&m.bus.vram, &m.bus.oam, &m.bus.palette, &m.bus.io);
            let reg = |o: usize| u16::from_le_bytes([m.bus.io[o], m.bus.io[o + 1]]);
            if volcar.contains(&cuadro) {
                volca(cuadro, "software", &sw);
                volca(cuadro, "shim", &hw);
                let lineas: Vec<String> = (0..160usize)
                    .filter(|&l| (m.bus.perf_counters.mascara_lineas[l / 64] >> (l % 64)) & 1 == 1)
                    .map(|l| l.to_string())
                    .collect();
                let _ = std::fs::write(format!("volcados/cuadro-{cuadro}-lineas.txt"), lineas.join("\n"));
                for (nombre, mascara) in [("vram", m.bus.perf_counters.mascara_vram), ("oam", m.bus.perf_counters.mascara_oam)] {
                    let lista: Vec<String> = (0..160usize)
                        .filter(|&l| (mascara[l / 64] >> (l % 64)) & 1 == 1)
                        .map(|l| l.to_string())
                        .collect();
                    let _ = std::fs::write(format!("volcados/cuadro-{cuadro}-lineas-{nombre}.txt"), lista.join("\n"));
                }
                println!("cuadro {cuadro}: DISPCNT={:04x} BLDCNT={:04x} BLDALPHA={:04x} BLDY={:04x}", reg(0x00), reg(0x50), reg(0x52), reg(0x54));
                let oam = &m.bus.oam;
                let (mut activos, mut en_8bpp, mut afines) = (0u32, 0u32, 0u32);
                for i in 0..128usize {
                    let a0 = u16::from_le_bytes([oam[i * 8], oam[i * 8 + 1]]);
                    let es_afin = a0 & (1 << 8) != 0;
                    if !es_afin && a0 & (1 << 9) != 0 {
                        continue;
                    }
                    activos += 1;
                    if a0 & (1 << 13) != 0 {
                        en_8bpp += 1;
                    }
                    if es_afin {
                        afines += 1;
                    }
                }
                println!("cuadro {cuadro}: BG0CNT={:04x} BG1CNT={:04x} BG2CNT={:04x} BG3CNT={:04x} MOSAIC={:04x} sprites={activos} 8bpp={en_8bpp} afines={afines}", reg(0x08), reg(0x0A), reg(0x0C), reg(0x0E), reg(0x4C));
            }
            let c = compara(&sw, &hw);
            if c.distintos > 0 {
                con_diferencias += 1;
            }
            distintos_total += c.distintos;
            let pct = c.distintos as f64 * 100.0 / (W * H) as f64;
            let primera = c.primera_linea.map_or("-".to_string(), |y| y.to_string());
            let _ = writeln!(informe, "{cuadro},{indice},{},{pct:.4},{primera},{:04x},{:04x},{:04x},{:04x},{escrituras},{escrituras_es},{lineas_con_cambio},{lineas_vram},{lineas_oam}", tramo.nombre, reg(0x00), reg(0x50), reg(0x52), reg(0x54));
            cuadro += 1;
            if cuadro % 1800 == 0 {
                println!("cuadro {cuadro} de {total} ({:.0} s)", t0.elapsed().as_secs_f64());
            }
        }
    }

    if let Err(e) = std::fs::write(&ruta_informe, &informe) {
        eprintln!("no se pudo escribir el informe {ruta_informe}: {e}");
        std::process::exit(1);
    }
    println!("cuadros comparados: {cuadro}");
    println!("cuadros con diferencias: {con_diferencias}");
    println!("píxeles distintos en total: {distintos_total}");
    println!("informe: {ruta_informe}");
}
