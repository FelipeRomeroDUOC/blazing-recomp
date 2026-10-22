//! Experimento de docs/propuestas/2026-10-18_tiles-afines-cuadro-7500.md.
//!
//! Cuadro N. Para cada sprite afín de OAM: modo (attr0 bits 10-11), profundidad, tile base, paleta,
//! prioridad y mapeo de DISPCNT; cuántos texeles no transparentes tiene su textura; y cuántos píxeles
//! de su caja caen dentro de la textura con la matriz real (fórmula de render_sprites de la PPU),
//! de ellos cuántos no transparentes.
//! Control: la PPU con la OAM completa debe coincidir con el framebuffer del emulador (0 diferencias).
//!
//! Escribe volcados/afin-tiles-<N>.txt.
//!
//! Uso:
//!   cargo run --release --features proto --example shim_afin_tiles -- <ruta.gba> <secuencia.txt> <cuadro>

#[path = "comun/mod.rs"]
mod comun;

use comun::entrada::leer_secuencia;
use comun::gpu::{ALTO, ANCHO};

const MODOS: [&str; 4] = ["normal", "semitransparente", "ventana de OBJ", "prohibido"];

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

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 4 {
        eprintln!("uso: shim_afin_tiles <ruta.gba> <secuencia.txt> <cuadro>");
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

    let afines = afines_de(&oam_fin);
    let mut lineas = vec![
        format!("resumen cuadro {objetivo}: diagnóstico de los afines (tiles, modo, recorte)"),
        format!("control PPU completa frente a framebuffer del emulador: {control_dif} diferencias (esperado 0)"),
        format!("DISPCNT {dispcnt:04x}; mapeo {}", if un_dim { "1D" } else { "2D" }),
        format!("afines en OAM: {}", afines.len()),
    ];
    for a in &afines {
        let a0 = ld16(&oam_fin, a.i * 8);
        let a2 = ld16(&oam_fin, a.i * 8 + 4);
        let modo = ((a0 >> 10) & 3) as usize;
        let ocho = a0 & 0x2000 != 0;
        let tile_base = (a2 & 0x3FF) as usize;
        let paleta = (a2 >> 12) & 0xF;
        let prioridad = (a2 >> 10) & 3;
        let (wt, ht) = if a.doble { (a.w / 2, a.h / 2) } else { (a.w, a.h) };
        let (bw, bh) = (a.w, a.h);
        let pa = parametro(&oam_fin, a.grupo, 6);
        let pb = parametro(&oam_fin, a.grupo, 14);
        let pc = parametro(&oam_fin, a.grupo, 22);
        let pd = parametro(&oam_fin, a.grupo, 30);

        // Texeles no transparentes de la textura entera.
        let mut texels_no_transp = 0usize;
        for v in 0..ht {
            for u in 0..wt {
                if texel(&vram_fin, tile_base, ocho, un_dim, wt, u, v) != 0 {
                    texels_no_transp += 1;
                }
            }
        }

        // Píxeles de la caja que caen dentro de la textura con la matriz real.
        let mut en_textura = 0usize;
        let mut candidatos = 0usize;
        for ly in 0..bh {
            for bx in 0..bw {
                let dx = bx as i32 - (bw / 2) as i32;
                let dy = ly as i32 - (bh / 2) as i32;
                let u = ((pa * dx + pb * dy) >> 8) + (wt / 2) as i32;
                let v = ((pc * dx + pd * dy) >> 8) + (ht / 2) as i32;
                if u < 0 || v < 0 || u >= wt as i32 || v >= ht as i32 {
                    continue;
                }
                en_textura += 1;
                if texel(&vram_fin, tile_base, ocho, un_dim, wt, u as usize, v as usize) != 0 {
                    candidatos += 1;
                }
            }
        }

        lineas.push(format!(
            "afin {:3}: modo {modo} ({}); {}bpp; tile base {tile_base:03x}; paleta {paleta}; prioridad {prioridad}; textura {wt}x{ht}; texels no transparentes {texels_no_transp}/{}; caja {bw}x{bh}: dentro de textura {en_textura}, no transparentes {candidatos}; matriz pa {:04x} pb {:04x} pc {:04x} pd {:04x}",
            a.i,
            MODOS[modo],
            if ocho { 8 } else { 4 },
            wt * ht,
            pa as i16 as u16,
            pb as i16 as u16,
            pc as i16 as u16,
            pd as i16 as u16,
        ));
    }
    for l in &lineas {
        println!("{l}");
    }
    let _ = std::fs::create_dir_all("volcados");
    let ruta = format!("volcados/afin-tiles-{objetivo}.txt");
    if let Err(e) = std::fs::write(&ruta, format!("{}\n", lineas.join("\n"))) {
        eprintln!("no se pudo escribir {ruta}: {e}");
        std::process::exit(1);
    }
}
