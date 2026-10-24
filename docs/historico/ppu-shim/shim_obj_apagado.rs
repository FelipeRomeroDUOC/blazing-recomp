//! Experimento de docs/propuestas/2026-10-18_decode-oam-respeta-obj-apagado-cuadro-1000.md.
//!
//! Uso:
//!   cargo run --release --features proto --example shim_obj_apagado -- <ruta.gba> <secuencia.txt> <cuadro> <etiqueta>
//!
//! Etiqueta "antes" o "despues" (solo cambia el nombre del archivo de salida).
//!
//! Mide en el shim:
//!   - huella (FNV-1a 64) de la salida con DISPCNT BG0 + OBJ (regresión H102);
//!   - píxeles de BG0 donde el shim pinta y `fondo_texto` está vacío, con OBJ apagado (H100);
//!   - diferencia de BG2 frente al estado de fin, con OBJ apagado (H101).
//!
//! Escribe volcados/obj-<etiqueta>-cuadro-<N>.txt.

#[path = "comun/mod.rs"]
mod comun;

use comun::color::{canal_8bit, cerca_pixel};
use comun::entrada::leer_secuencia;
use comun::gpu::{ALTO, ANCHO, Gpu};
use comun::registro::io16;

const TRANSPARENTE: u16 = 0xFFFF;

fn io_con_dispcnt(io: &[u8], d: u16) -> Vec<u8> {
    let mut out = io.to_vec();
    out[0] = (d & 0xFF) as u8;
    out[1] = (d >> 8) as u8;
    out
}

fn color_pal(pal: &[u8], i: usize) -> u16 {
    u16::from_le_bytes([pal[2 * i], pal[2 * i + 1]]) & 0x7FFF
}

fn leer_vram(vram: &[u8], addr: usize) -> u8 {
    vram.get(addr).copied().unwrap_or(0)
}

fn leer_bg(vram: &[u8], addr: usize) -> u8 {
    if addr < 0x1_0000 {
        vram.get(addr).copied().unwrap_or(0)
    } else {
        0
    }
}

fn rgb8(c: u16) -> [u8; 3] {
    [canal_8bit(c), canal_8bit(c >> 5), canal_8bit(c >> 10)]
}

/// Fondo de texto (modo 0) de la línea, según los bits de BGnCNT (referencia del CPU).
fn fondo_texto(io: &[u8], vram: &[u8], pal: &[u8], bg: usize, linea: usize) -> Vec<u16> {
    let mut out = vec![TRANSPARENTE; ANCHO];
    let cnt = io16(io, 0x08 + bg * 2);
    let base_car = ((cnt >> 2) & 3) as usize * 0x4000;
    let ocho_bpp = cnt & 0x80 != 0;
    let base_mapa = ((cnt >> 8) & 0x1F) as usize * 0x800;
    let (ancho_t, alto_t) = match (cnt >> 14) & 3 {
        0 => (32, 32),
        1 => (64, 32),
        2 => (32, 64),
        _ => (64, 64),
    };
    let hofs = (io16(io, 0x10 + bg * 4) & 0x1FF) as usize;
    let vofs = (io16(io, 0x12 + bg * 4) & 0x1FF) as usize;
    let y = (linea + vofs) % (alto_t * 8);
    let (ty, py) = (y / 8, y % 8);
    for (sx, salida) in out.iter_mut().enumerate() {
        let x = (sx + hofs) % (ancho_t * 8);
        let (tx, px) = (x / 8, x % 8);
        let bloque = (ty / 32) * (ancho_t / 32) + tx / 32;
        let entrada_idx = bloque * 1024 + (ty % 32) * 32 + (tx % 32);
        let off = base_mapa + entrada_idx * 2;
        let entrada = u16::from_le_bytes([leer_vram(vram, off), leer_vram(vram, off + 1)]);
        let tile = (entrada & 0x3FF) as usize;
        let hf = entrada & 0x400 != 0;
        let vf = entrada & 0x800 != 0;
        let fy = if vf { 7 - py } else { py };
        let fx = if hf { 7 - px } else { px };
        *salida = if ocho_bpp {
            let idx = leer_bg(vram, base_car + tile * 64 + fy * 8 + fx) as usize;
            if idx == 0 {
                TRANSPARENTE
            } else {
                color_pal(pal, idx)
            }
        } else {
            let b = leer_bg(vram, base_car + tile * 32 + fy * 4 + fx / 2);
            let nib = if fx % 2 == 0 { b & 0xF } else { b >> 4 } as usize;
            if nib == 0 {
                TRANSPARENTE
            } else {
                color_pal(pal, ((entrada >> 12) as usize & 0xF) * 16 + nib)
            }
        };
    }
    out
}

fn huella(datos: &[u8]) -> u64 {
    let mut h: u64 = 0xcbf29ce484222325;
    for &b in datos {
        h ^= b as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    h
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 5 {
        eprintln!("uso: shim_obj_apagado <ruta.gba> <secuencia.txt> <cuadro> <antes|despues>");
        std::process::exit(2);
    }
    let rom = match std::fs::read(&args[1]) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("ROM no encontrado: {} ({e})", args[1]);
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
    let etiqueta = args[4].clone();
    let tramos = match leer_secuencia(&texto) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("secuencia inválida: {e}");
            std::process::exit(1);
        }
    };

    let mut m = gba_core::Machine::new(rom);
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
    let vram = m.bus.vram.clone();
    let oam = m.bus.oam.clone();
    let pal = m.bus.palette.clone();
    let io = m.bus.io.clone();
    let n = ANCHO * ALTO;

    let mut gpu = match Gpu::nueva() {
        Ok(g) => g,
        Err(e) => {
            eprintln!("{e}");
            std::process::exit(1);
        }
    };
    let backdrop = rgb8(color_pal(&pal, 0));

    // H102: huella de la salida con BG0 + OBJ.
    let con_obj = gpu.renderiza(&vram, &oam, &pal, &io_con_dispcnt(&io, 0x1100));
    let huella_obj = huella(&con_obj);

    // H100: BG0 sola, OBJ apagado. Píxeles donde el shim pinta y fondo_texto está vacío.
    let bg0 = gpu.renderiza(&vram, &oam, &pal, &io_con_dispcnt(&io, 0x0100));
    let mut candidatos_bg0 = 0usize;
    for p in 0..n {
        let (l, x) = (p / ANCHO, p % ANCHO);
        let ref0 = fondo_texto(&io, &vram, &pal, 0, l)[x];
        let s = [bg0[p * 3], bg0[p * 3 + 1], bg0[p * 3 + 2]];
        if ref0 == TRANSPARENTE && !cerca_pixel(s, backdrop) {
            candidatos_bg0 += 1;
        }
    }

    // H101: BG2 sola, OBJ apagado. Diferencia frente al estado de fin (misma métrica que contenido-bg).
    let bg2 = gpu.renderiza(&vram, &oam, &pal, &io_con_dispcnt(&io, 0x0400));
    let mut dif_bg2_fin = 0usize;
    let mut opacos_bg2 = 0usize;
    for p in 0..n {
        let (l, x) = (p / ANCHO, p % ANCHO);
        let ref2 = fondo_texto(&io, &vram, &pal, 2, l)[x];
        if ref2 != TRANSPARENTE {
            opacos_bg2 += 1;
            let s = [bg2[p * 3], bg2[p * 3 + 1], bg2[p * 3 + 2]];
            if !cerca_pixel(s, rgb8(ref2)) {
                dif_bg2_fin += 1;
            }
        }
    }

    let lineas = vec![
        format!("resumen cuadro {objetivo} ({etiqueta}): shim con OBJ apagado frente a BG0 y BG2"),
        format!("huella salida BG0 + OBJ (H102): {huella_obj:016x}"),
        format!("bg0 candidatos sin OBJ (H100): {candidatos_bg0}"),
        format!("bg2 opacos en referencia: {opacos_bg2}"),
        format!("bg2 shim vs fin sin OBJ (H101): {dif_bg2_fin}"),
    ];
    for l in &lineas {
        println!("{l}");
    }
    let _ = std::fs::create_dir_all("volcados");
    let ruta = format!("volcados/obj-{etiqueta}-cuadro-{objetivo}.txt");
    if let Err(e) = std::fs::write(&ruta, format!("{}\n", lineas.join("\n"))) {
        eprintln!("no se pudo escribir {ruta}: {e}");
        std::process::exit(1);
    }
}
