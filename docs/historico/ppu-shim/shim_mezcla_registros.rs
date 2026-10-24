//! Experimento de docs/propuestas/2026-10-18_bldcnt-bldalpha-cuadro-1000.md.
//!
//! Cuadro N, shim completo frente a la PPU por software. Para los píxeles donde difieren (X):
//!   - grupo "semi sin fondo": gana un sprite semitransparente y ningún fondo es opaco debajo;
//!   - grupo "semi con fondo": gana un sprite semitransparente y hay algún fondo opaco debajo;
//!   - grupo "no semi": el resto.
//! Para cada grupo cuenta EVA = EVB = 8 (H141), bit de backdrop 0x2000 de BLDCNT apagado (H142)
//! y bit de objeto 0x10 de BLDCNT apagado. Imprime la distribución de (EVA, EVB) de los semitransparentes.
//!
//! Desviación respecto a la propuesta: los grupos se definen sobre el conjunto X completo
//! (no con la clasificación de shim_origen_x). Control: el total de X debe ser 5290.
//!
//! Escribe volcados/mezcla-registros-cuadro-<N>.txt.
//!
//! Uso:
//!   cargo run --release --features proto --example shim_mezcla_registros -- <ruta.gba> <secuencia.txt> <cuadro>

use std::collections::BTreeMap;

#[path = "comun/mod.rs"]
mod comun;

use comun::color::{canal_8bit, cerca_pixel};
use comun::entrada::leer_secuencia;
use comun::gpu::{ALTO, ANCHO, Gpu};
use comun::registro::{BIT_OBJ, dispcnt_de, io16};

const TRANSPARENTE: u16 = 0xFFFF;
const OBJ_TAMANOS: [[(usize, usize); 4]; 3] = [
    [(8, 8), (16, 16), (32, 32), (64, 64)],
    [(16, 8), (32, 8), (32, 16), (64, 32)],
    [(8, 16), (8, 32), (16, 32), (32, 64)],
];
const ESPERADO_X: usize = 5290;

#[derive(Clone, Copy)]
struct Pixel {
    color: u16,
    prio: u8,
    semi: bool,
}

const SIN_OBJ: Pixel = Pixel {
    color: TRANSPARENTE,
    prio: 0xFF,
    semi: false,
};

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

/// Sprite ganador por píxel de la línea (copia de shim_contenido_bg::sprites, sin índice).
fn sprites(disp: u16, oam: &[u8], vram: &[u8], pal: &[u8], linea: usize) -> Result<Vec<Pixel>, String> {
    let mut out = vec![SIN_OBJ; ANCHO];
    let un_dim = disp & 0x40 != 0;
    let lin = linea as i64;
    for i in 0..128 {
        let a0 = u16::from_le_bytes([oam[i * 8], oam[i * 8 + 1]]);
        if a0 & 0x100 != 0 {
            return Err(format!("sprite afín {i}: no reconstruido"));
        }
        if a0 & 0x200 != 0 {
            continue;
        }
        let forma = ((a0 >> 14) & 3) as usize;
        if forma == 3 {
            continue;
        }
        let a1 = u16::from_le_bytes([oam[i * 8 + 2], oam[i * 8 + 3]]);
        let a2 = u16::from_le_bytes([oam[i * 8 + 4], oam[i * 8 + 5]]);
        let talla = ((a1 >> 14) & 3) as usize;
        let (w, h) = OBJ_TAMANOS[forma][talla];
        let mut y = (a0 & 0xFF) as i64;
        if y >= 160 {
            y -= 256;
        }
        let mut x = (a1 & 0x1FF) as i64;
        if x >= 240 {
            x -= 512;
        }
        if lin < y || lin >= y + h as i64 {
            continue;
        }
        let modo = (a0 >> 10) & 3;
        if modo == 2 {
            return Err(format!("ventana de sprites {i}: no reconstruida"));
        }
        let ocho = a0 & 0x2000 != 0;
        let tile_base = (a2 & 0x3FF) as usize;
        let prio = ((a2 >> 10) & 3) as u8;
        let banco = ((a2 >> 12) & 0xF) as usize;
        let fila_tiles = if un_dim {
            (w / 8) * if ocho { 2 } else { 1 }
        } else {
            32
        };
        let ly = (lin - y) as usize;
        let hflip = a1 & 0x1000 != 0;
        let vflip = a1 & 0x2000 != 0;
        for bx in 0..w {
            let sx = x + bx as i64;
            if sx < 0 || sx >= ANCHO as i64 {
                continue;
            }
            let u = if hflip { w - 1 - bx } else { bx };
            let v = if vflip { h - 1 - ly } else { ly };
            let (tu, pu) = (u / 8, u % 8);
            let (tv, pv) = (v / 8, v % 8);
            let paso = if ocho { 2 } else { 1 };
            let tile = tile_base + tv * fila_tiles + tu * paso;
            let addr = 0x1_0000
                + if ocho {
                    (tile / 2) * 64 + pv * 8 + pu
                } else {
                    tile * 32 + pv * 4 + pu / 2
                };
            if addr >= 0x1_8000 {
                continue;
            }
            let idx = if ocho {
                leer_vram(vram, addr) as usize
            } else {
                let b = leer_vram(vram, addr);
                let nib = if pu % 2 == 0 { b & 0xF } else { b >> 4 } as usize;
                if nib == 0 {
                    0
                } else {
                    banco * 16 + nib
                }
            };
            if idx == 0 {
                continue;
            }
            let color = color_pal(pal, 256 + idx);
            let sxu = sx as usize;
            if out[sxu].prio == 0xFF || prio < out[sxu].prio {
                out[sxu] = Pixel {
                    color,
                    prio,
                    semi: modo == 1,
                };
            }
        }
    }
    Ok(out)
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 4 {
        eprintln!("uso: shim_mezcla_registros <ruta.gba> <secuencia.txt> <cuadro>");
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
    let vram_fin = m.bus.vram.clone();
    let oam_fin = m.bus.oam.clone();
    let pal_fin = m.bus.palette.clone();
    let io_fin = m.bus.io.clone();
    let n = ANCHO * ALTO;

    // PPU por software, línea a línea, con el estado de cada línea.
    let mut ppu: Vec<Vec<u16>> = vec![vec![0u16; n]; 4];
    let mut replay = gba_core::Machine::new(rom);
    for cap in capturas.iter() {
        replay.bus.palette = cap.palette.clone();
        replay.bus.vram = cap.vram.clone();
        replay.bus.oam = cap.oam.clone();
        replay.bus.aff_ref = cap.aff_ref;
        replay.bus.aff_dirty = cap.aff_dirty;
        let ini = cap.linea * ANCHO;
        for v in 0..4 {
            replay.bus.io = io_con_dispcnt(&cap.io, dispcnt_de(v, io16(&cap.io, 0)));
            gba_core::ppu::render_scanline(&mut replay.bus, cap.linea);
            ppu[v][ini..ini + ANCHO].copy_from_slice(&replay.bus.framebuffer[ini..ini + ANCHO]);
        }
    }

    // Sprites y fondos por línea (para clasificar los píxeles).
    let mut bg_linea: Vec<Vec<Vec<u16>>> = vec![Vec::new(); ALTO];
    let mut obj_linea: Vec<Vec<Pixel>> = vec![Vec::new(); ALTO];
    for cap in capturas.iter() {
        let base = io16(&cap.io, 0);
        bg_linea[cap.linea] = (0..4)
            .map(|b| fondo_texto(&cap.io, &cap.vram, &cap.palette, b, cap.linea))
            .collect();
        obj_linea[cap.linea] = if base & BIT_OBJ != 0 {
            match sprites(base, &cap.oam, &cap.vram, &cap.palette, cap.linea) {
                Ok(o) => o,
                Err(e) => {
                    eprintln!("línea {}: {e}", cap.linea);
                    std::process::exit(1);
                }
            }
        } else {
            vec![SIN_OBJ; ANCHO]
        };
    }

    // Shim completo con el estado de fin.
    let mut gpu = match Gpu::nueva() {
        Ok(g) => g,
        Err(e) => {
            eprintln!("{e}");
            std::process::exit(1);
        }
    };
    let shim = gpu.renderiza(&vram_fin, &oam_fin, &pal_fin, &io_fin);

    // Píxeles X: el shim y la PPU difieren.
    let mut x_total = 0usize;
    let mut grupos: [(usize, usize, usize, usize); 3] = [(0, 0, 0, 0); 3];
    let mut distribucion: BTreeMap<(u32, u32), usize> = BTreeMap::new();
    let base_fin = io16(&io_fin, 0);
    let bldcnt = io16(&io_fin, 0x50);
    let bldalpha = io16(&io_fin, 0x52);
    let eva = (bldalpha & 0x1F) as u32;
    let evb = ((bldalpha >> 8) & 0x1F) as u32;
    let bit_backdrop = bldcnt & 0x2000 != 0;
    let bit_obj_t1 = bldcnt & 0x0010 != 0;
    // EVA/EVB del registro de cada línea (lo que lee la PPU), no solo del estado de fin.
    let mut dist_linea: BTreeMap<(u32, u32), usize> = BTreeMap::new();
    let mut x8_linea = 0usize;
    // BLDCNT de cada línea para los píxeles X (H142 a H144).
    let mut bc_linea: BTreeMap<u16, usize> = BTreeMap::new();
    let mut bd_on = 0usize;
    let mut obj1_on = 0usize;
    for p in 0..n {
        let (l, x) = (p / ANCHO, p % ANCHO);
        let sh = [shim[p * 3], shim[p * 3 + 1], shim[p * 3 + 2]];
        // Aportación (igual que shim_origen_x): la PPU cambia al apagar OBJ y V2 == V3.
        let en_aportacion = ppu[0][p] != ppu[1][p] && ppu[2][p] == ppu[3][p];
        if !en_aportacion || cerca_pixel(sh, rgb8(ppu[0][p])) {
            continue;
        }
        x_total += 1;
        let o = obj_linea[l][x];
        let hay_fondo = (0..4).any(|k| {
            base_fin & (0x100 << k) != 0 && bg_linea[l][k][x] != TRANSPARENTE
        });
        let grupo = if o.prio != 0xFF && o.semi && !hay_fondo {
            0
        } else if o.prio != 0xFF && o.semi && hay_fondo {
            1
        } else {
            2
        };
        let cap = capturas.iter().find(|c| c.linea == l).expect("captura de línea");
        let bcl = io16(&cap.io, 0x50);
        *bc_linea.entry(bcl).or_insert(0) += 1;
        if bcl & 0x2000 != 0 {
            bd_on += 1;
        }
        if bcl & 0x0010 != 0 {
            obj1_on += 1;
        }
        let bla_l = io16(&cap.io, 0x52);
        let (ea, eb) = ((bla_l & 0x1F) as u32, ((bla_l >> 8) & 0x1F) as u32);
        *dist_linea.entry((ea, eb)).or_insert(0) += 1;
        if ea == 8 && eb == 8 {
            x8_linea += 1;
        }
        let g = &mut grupos[grupo];
        g.0 += 1;
        if eva == 8 && evb == 8 {
            g.1 += 1;
        }
        if !bit_backdrop {
            g.2 += 1;
        }
        if !bit_obj_t1 {
            g.3 += 1;
        }
        if grupo < 2 {
            *distribucion.entry((eva, evb)).or_insert(0) += 1;
        }
    }

    let nombres = ["semi sin fondo", "semi con fondo", "no semi"];
    let mut lineas = vec![
        format!("resumen cuadro {objetivo}: registros de mezcla frente a sprites semitransparentes"),
        format!("bldcnt {bldcnt:04x} bldalpha {bldalpha:04x} bldy {:04x}", io16(&io_fin, 0x54)),
        format!("control aportacion total X: {x_total} (esperado {ESPERADO_X})"),
    ];
    for (i, nombre) in nombres.iter().enumerate() {
        let (total, eva88, bd_off, t1_off) = grupos[i];
        lineas.push(format!("{nombre} total: {total}"));
        lineas.push(format!("{nombre} EVA=EVB=8 (H141): {eva88}"));
        lineas.push(format!("{nombre} bit backdrop 0x2000 apagado (H142): {bd_off}"));
        lineas.push(format!("{nombre} bit objeto 0x10 apagado: {t1_off}"));
    }
    lineas.push(format!(
        "distribucion EVA EVB (semitransparentes): {}",
        distribucion
            .iter()
            .map(|((a, b), c)| format!("{a}/{b}:{c}"))
            .collect::<Vec<_>>()
            .join(" ")
    ));
    lineas.push(format!(
        "distribucion EVA EVB por linea (X): {}",
        dist_linea
            .iter()
            .map(|((a, b), c)| format!("{a}/{b}:{c}"))
            .collect::<Vec<_>>()
            .join(" ")
    ));
    lineas.push(format!("X con EVA=EVB=8 en su linea (H141): {x8_linea}"));
    lineas.push(format!(
        "X con backdrop 0x2000 encendido por linea: {bd_on}; apagado: {}",
        x_total - bd_on
    ));
    lineas.push(format!("X con OBJ primer destino 0x0010 encendido por linea: {obj1_on}"));
    lineas.push(format!(
        "bldcnt por linea (X): {}",
        bc_linea
            .iter()
            .map(|(v, c)| format!("{v:04x}:{c}"))
            .collect::<Vec<_>>()
            .join(" ")
    ));
    for l in &lineas {
        println!("{l}");
    }
    // El control de 5.290 solo aplica al cuadro 1000 (medición base); en otros cuadros se informa el total.
    if objetivo == 1000 && x_total != ESPERADO_X {
        eprintln!("control falla: X {x_total} distinto de {ESPERADO_X}; no se escribe el informe");
        std::process::exit(1);
    }
    let _ = std::fs::create_dir_all("volcados");
    let ruta = format!("volcados/mezcla-registros-cuadro-{objetivo}.txt");
    if let Err(e) = std::fs::write(&ruta, format!("{}\n", lineas.join("\n"))) {
        eprintln!("no se pudo escribir {ruta}: {e}");
        std::process::exit(1);
    }
}
