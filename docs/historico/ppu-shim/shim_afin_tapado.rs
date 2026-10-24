//! Experimento de docs/propuestas/2026-10-18_fondos-tapan-afines-cuadro-7500.md.
//!
//! Cuadro N. Para cada sprite afín de OAM: los píxeles de su caja que caen en pantalla, dentro de la
//! textura con la matriz real y con texel no transparente. De ellos, cuántos quedan tapados por un
//! fondo de texto habilitado, opaco en ese píxel y con prioridad estrictamente menor que la del sprite,
//! y cuántos no quedan tapados.
//! Control: la PPU con la OAM completa debe coincidir con el framebuffer del emulador (0 diferencias).
//!
//! Escribe volcados/afin-tapado-<N>.txt.
//!
//! Uso:
//!   cargo run --release --features proto --example shim_afin_tapado -- <ruta.gba> <secuencia.txt> <cuadro>

#[path = "comun/mod.rs"]
mod comun;

use comun::entrada::leer_secuencia;
use comun::gpu::{ALTO, ANCHO};
use comun::registro::io16;

const TRANSPARENTE: u16 = 0xFFFF;

const OBJ_TAMANOS: [[(usize, usize); 4]; 3] = [
    [(8, 8), (16, 16), (32, 32), (64, 64)],
    [(16, 8), (32, 8), (32, 16), (64, 32)],
    [(8, 16), (8, 32), (16, 32), (32, 64)],
];

struct Afin {
    i: usize,
    x: i64,
    y: i64,
    w: usize,
    h: usize,
    grupo: usize,
    doble: bool,
}

fn ld16(datos: &[u8], o: usize) -> u16 {
    u16::from_le_bytes([datos[o], datos[o + 1]])
}

fn afines_de(oam: &[u8]) -> Vec<Afin> {
    let mut out = Vec::new();
    for i in 0..128 {
        let a0 = ld16(oam, i * 8);
        if a0 & 0x100 == 0 {
            continue;
        }
        let a1 = ld16(oam, i * 8 + 2);
        let forma = ((a0 >> 14) & 3) as usize;
        if forma == 3 {
            continue;
        }
        let talla = ((a1 >> 14) & 3) as usize;
        let (mut w, mut h) = OBJ_TAMANOS[forma][talla];
        let doble = a0 & 0x200 != 0;
        if doble {
            w *= 2;
            h *= 2;
        }
        let mut y = (a0 & 0xFF) as i64;
        if y >= 160 {
            y -= 256;
        }
        let mut x = (a1 & 0x1FF) as i64;
        if x >= 240 {
            x -= 512;
        }
        let grupo = ((a1 >> 9) & 0x1F) as usize;
        out.push(Afin {
            i,
            x,
            y,
            w,
            h,
            grupo,
            doble,
        });
    }
    out
}

/// Parámetro de matriz: en la PPU, g = grupo*32 y se leen los desplazamientos 6, 14, 22, 30.
fn parametro(oam: &[u8], grupo: usize, desplaz: usize) -> i32 {
    ld16(oam, grupo * 32 + desplaz) as i16 as i32
}

/// Índice de color de un texel de sprite (0 = transparente), con la misma dirección que render_sprites.
fn texel(vram: &[u8], tile_base: usize, ocho: bool, un_dim: bool, w: usize, u: usize, v: usize) -> u8 {
    let filas = if un_dim {
        (w / 8) * if ocho { 2 } else { 1 }
    } else {
        32
    };
    let (tu, pu) = (u / 8, u % 8);
    let (tv, pv) = (v / 8, v % 8);
    let paso = if ocho { 2 } else { 1 };
    let tile = tile_base + tv * filas + tu * paso;
    let addr = 0x1_0000
        + if ocho {
            (tile / 2) * 64 + pv * 8 + pu
        } else {
            tile * 32 + pv * 4 + pu / 2
        };
    if addr >= 0x1_8000 {
        return 0;
    }
    let b = vram.get(addr).copied().unwrap_or(0);
    if ocho {
        b
    } else if pu % 2 == 0 {
        b & 0xF
    } else {
        b >> 4
    }
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

fn color_pal(pal: &[u8], i: usize) -> u16 {
    u16::from_le_bytes([pal[2 * i], pal[2 * i + 1]]) & 0x7FFF
}

/// Fondo de texto (modo 0) de la línea, con los bits de BGnCNT (copia de shim_mezcla_registros).
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

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 4 {
        eprintln!("uso: shim_afin_tapado <ruta.gba> <secuencia.txt> <cuadro>");
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
    let tramos = match leer_secuencia(&texto) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("secuencia inválida: {e}");
            std::process::exit(1);
        }
    };

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
    let capturas = &m.bus.capturas_linea;
    if capturas.len() != ALTO {
        eprintln!("se esperaban {ALTO} capturas y hay {}", capturas.len());
        std::process::exit(1);
    }
    let framebuffer_emu: Vec<u16> = m.bus.framebuffer.to_vec();
    let vram_fin = m.bus.vram.clone();
    let oam_fin = m.bus.oam.clone();
    let io_fin = m.bus.io.clone();
    let dispcnt = ld16(&io_fin, 0);
    let un_dim = dispcnt & 0x40 != 0;
    let n = ANCHO * ALTO;

    // Control: PPU con la OAM completa frente al emulador.
    let mut replay = gba_core::Machine::new(rom);
    let mut ppu = vec![0u16; n];
    for cap in capturas.iter() {
        replay.bus.palette = cap.palette.clone();
        replay.bus.vram = cap.vram.clone();
        replay.bus.oam = cap.oam.clone();
        replay.bus.aff_ref = cap.aff_ref;
        replay.bus.aff_dirty = cap.aff_dirty;
        replay.bus.io = cap.io.clone();
        gba_core::ppu::render_scanline(&mut replay.bus, cap.linea);
        let ini = cap.linea * ANCHO;
        ppu[ini..ini + ANCHO].copy_from_slice(&replay.bus.framebuffer[ini..ini + ANCHO]);
    }
    let control_dif = (0..n).filter(|&p| ppu[p] != framebuffer_emu[p]).count();
    if control_dif != 0 {
        eprintln!("control falla: {control_dif} diferencias entre PPU y emulador; no se escribe el informe");
        std::process::exit(1);
    }

    // Capas de texto por línea, para saber si un fondo tapa el sprite.
    let mut bg_linea: Vec<Vec<Vec<u16>>> = vec![Vec::new(); ALTO];
    for cap in capturas.iter() {
        bg_linea[cap.linea] = (0..4)
            .map(|b| fondo_texto(&cap.io, &cap.vram, &cap.palette, b, cap.linea))
            .collect();
    }

    let afines = afines_de(&oam_fin);
    let mut lineas = vec![
        format!("resumen cuadro {objetivo}: píxeles de cada afín tapados por fondos"),
        format!("control PPU completa frente a framebuffer del emulador: {control_dif} diferencias (esperado 0)"),
        format!("DISPCNT {dispcnt:04x}; mapeo {}", if un_dim { "1D" } else { "2D" }),
        "regla: un fondo tapa al sprite si está habilitado, es opaco en el píxel y su prioridad es estrictamente menor".to_string(),
    ];
    for a in &afines {
        let a2 = ld16(&oam_fin, a.i * 8 + 4);
        let ocho = ld16(&oam_fin, a.i * 8) & 0x2000 != 0;
        let tile_base = (a2 & 0x3FF) as usize;
        let prio_sprite = ((a2 >> 10) & 3) as u8;
        let (wt, ht) = if a.doble { (a.w / 2, a.h / 2) } else { (a.w, a.h) };
        let (bw, bh) = (a.w, a.h);
        let pa = parametro(&oam_fin, a.grupo, 6);
        let pb = parametro(&oam_fin, a.grupo, 14);
        let pc = parametro(&oam_fin, a.grupo, 22);
        let pd = parametro(&oam_fin, a.grupo, 30);

        let mut visibles = 0usize;
        let mut tapados = 0usize;
        let mut no_tapados = 0usize;
        for ly in 0..bh {
            let linea = a.y + ly as i64;
            if linea < 0 || linea >= ALTO as i64 {
                continue;
            }
            let linea = linea as usize;
            for bx in 0..bw {
                let sx = a.x + bx as i64;
                if sx < 0 || sx >= ANCHO as i64 {
                    continue;
                }
                let sx = sx as usize;
                let dx = bx as i32 - (bw / 2) as i32;
                let dy = ly as i32 - (bh / 2) as i32;
                let u = ((pa * dx + pb * dy) >> 8) + (wt / 2) as i32;
                let v = ((pc * dx + pd * dy) >> 8) + (ht / 2) as i32;
                if u < 0 || v < 0 || u >= wt as i32 || v >= ht as i32 {
                    continue;
                }
                if texel(&vram_fin, tile_base, ocho, un_dim, wt, u as usize, v as usize) == 0 {
                    continue;
                }
                visibles += 1;
                let tapa = (0..4).any(|b| {
                    let prio_fondo = (io16(&io_fin, 0x08 + b * 2) & 3) as u8;
                    dispcnt & (0x100 << b) != 0
                        && prio_fondo < prio_sprite
                        && bg_linea[linea][b][sx] != TRANSPARENTE
                });
                if tapa {
                    tapados += 1;
                } else {
                    no_tapados += 1;
                }
            }
        }

        lineas.push(format!(
            "afin {:3}: prioridad sprite {prio_sprite}; píxeles con color en pantalla {visibles}; tapados por fondo {tapados}; no tapados {no_tapados}",
            a.i
        ));
    }
    for l in &lineas {
        println!("{l}");
    }
    let _ = std::fs::create_dir_all("volcados");
    let ruta = format!("volcados/afin-tapado-{objetivo}.txt");
    if let Err(e) = std::fs::write(&ruta, format!("{}\n", lineas.join("\n"))) {
        eprintln!("no se pudo escribir {ruta}: {e}");
        std::process::exit(1);
    }
}
