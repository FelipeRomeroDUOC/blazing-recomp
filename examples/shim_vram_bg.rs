//! Experimento de docs/propuestas/2026-10-18_subida-vram-bg-shim-cuadro-1000.md.
//!
//! Con solo BG0 activa en el cuadro N, para los píxeles donde el shim pinta color y la referencia
//! `fondo_texto` está vacía compara:
//!   - bytes: lo que la GPU lee de `vram_bg` (96 KB = vram[..98304], little-endian) frente al snapshot
//!     `vram` en la dirección de la entrada de mapa y en la del dato de tile (H90);
//!   - uniformes: BgLayerConfig que recibe la GPU frente a los bits de BG0CNT (H91);
//!   - diagnóstico H93: si la GPU dibuja sprites aunque DISPCNT tenga OBJ apagado (decode_oam no mira el bit 0x1000);
//!   - diagnóstico H94: si la dirección de dato de tile supera 0x10000 (leer_bg de la referencia pone 0 ahí;
//!     la GPU lee hasta 0x18000).
//!
//! Controles: reconstrucción V0 y V1 = 0, paleta = 0 y 710 candidatos. Si no, no escribe el informe.
//!
//! Escribe volcados/subida-vram-bg0-cuadro-<N>.txt.
//!
//! Uso:
//!   cargo run --release --features proto --example shim_vram_bg -- <ruta.gba> <secuencia.txt> <cuadro>

use blazing_recomp::ppu_shim::{BgLayerConfig, PpuShim};

#[path = "comun/mod.rs"]
mod comun;

use comun::color::{canal_8bit, cerca_pixel};
use comun::entrada::leer_secuencia;
use comun::gpu::{ALTO, ANCHO, Gpu};
use comun::registro::{BIT_OBJ, dispcnt_de, io16};

const TRANSPARENTE: u16 = 0xFFFF;
const NINGUNO: usize = usize::MAX;
const OBJ_TAMANOS: [[(usize, usize); 4]; 3] = [
    [(8, 8), (16, 16), (32, 32), (64, 64)],
    [(16, 8), (32, 8), (32, 16), (64, 32)],
    [(8, 16), (8, 32), (16, 32), (32, 64)],
];
const ESPERADOS_710: usize = 710;
const BG_VRAM_LEN: usize = 98304;

#[derive(Clone, Copy)]
struct Pixel {
    color: u16,
    prio: u8,
    semi: bool,
    idx: usize,
}

const SIN_OBJ: Pixel = Pixel {
    color: TRANSPARENTE,
    prio: 0xFF,
    semi: false,
    idx: NINGUNO,
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

/// Byte que ve la GPU: `vram_bg` contiene vram[..98304]; fuera de ese rango lee 0.
fn byte_gpu(vram: &[u8], addr: usize) -> u8 {
    if addr < BG_VRAM_LEN {
        leer_vram(vram, addr)
    } else {
        0
    }
}

/// Byte de referencia de fondo_texto (mismo límite 0x10000 que su leer_bg).
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

/// Resultado de la réplica de fs_bg para un píxel, con la lectura que hace la GPU (`byte_gpu`).
struct Replica {
    entrada_addr: usize,
    dato_addr: usize,
    entrada: u16,
    /// Índice de color leído por la GPU (0 = transparente).
    raw: usize,
    color: u16,
}

/// Réplica de `fs_bg` (src/ppu_shim.wgsl) con la configuración de `PpuShim::decode_io_regs`.
/// Supone que los uniformes llevan los números de bloque de BgLayerConfig.
fn fs_bg_replica(cfg: &BgLayerConfig, vram: &[u8], pal: &[u8], x: usize, linea: usize) -> Replica {
    let px0 = x as i64 + cfg.scroll_x as i64;
    let py0 = linea as i64 + cfg.scroll_y as i64;
    let (map_w, map_h): (i64, i64) = match cfg.screen_size & 3 {
        0 => (256, 256),
        1 => (512, 256),
        2 => (256, 512),
        _ => (512, 512),
    };
    let px = px0.rem_euclid(map_w);
    let py = py0.rem_euclid(map_h);
    let tile_x = (px / 8) as u32;
    let tile_y = (py / 8) as u32;
    let (mut ltx, mut lty, mut sbb) = (tile_x, tile_y, 0u32);
    match cfg.screen_size & 3 {
        1 => {
            if ltx >= 32 {
                sbb = 1;
                ltx -= 32;
            }
        }
        2 => {
            if lty >= 32 {
                sbb = 1;
                lty -= 32;
            }
        }
        3 => {
            let bx = ltx / 32;
            let by = lty / 32;
            sbb = by * 2 + bx;
            ltx %= 32;
            lty %= 32;
        }
        _ => {}
    }
    let entrada_addr = ((cfg.screen_base_block as u32 + sbb) * 2048 + (lty * 32 + ltx) * 2) as usize;
    let entrada = u16::from_le_bytes([
        byte_gpu(vram, entrada_addr),
        byte_gpu(vram, entrada_addr + 1),
    ]);
    let tile = (entrada & 0x3FF) as usize;
    let hf = entrada & (1 << 10) != 0;
    let vf = entrada & (1 << 11) != 0;
    let sub = ((entrada >> 12) & 0xF) as usize;
    let mut fx = (px as u32) % 8;
    let mut fy = (py as u32) % 8;
    if hf {
        fx = 7 - fx;
    }
    if vf {
        fy = 7 - fy;
    }
    let base_car = cfg.char_base_block as usize * 16384;
    let (dato_addr, raw, pal_idx) = if cfg.is_8bpp {
        let a = base_car + tile * 64 + (fy as usize) * 8 + fx as usize;
        let r = byte_gpu(vram, a) as usize;
        (a, r, r)
    } else {
        let a = base_car + tile * 32 + (fy as usize) * 4 + (fx as usize) / 2;
        let b = byte_gpu(vram, a);
        let r = if fx % 2 == 0 { b & 0xF } else { b >> 4 } as usize;
        (a, r, sub * 16 + r)
    };
    let color = if raw == 0 {
        TRANSPARENTE
    } else {
        color_pal(pal, pal_idx)
    };
    Replica {
        entrada_addr,
        dato_addr,
        entrada,
        raw,
        color,
    }
}

/// Sprite ganador por píxel de la línea (copia de shim_bg0_710::sprites).
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
                    idx: i,
                };
            }
        }
    }
    Ok(out)
}

fn separa(c: u16) -> (u32, u32, u32) {
    (
        (c & 31) as u32,
        (((c >> 5) & 31) as u32) << 1,
        ((c >> 10) & 31) as u32,
    )
}

fn junta(r: u32, g: u32, b: u32) -> u16 {
    (r | ((g >> 1) << 5) | (b << 10)) as u16
}

fn alpha(a: u16, b: u16, eva: u32, evb: u32) -> u16 {
    let (ar, ag, ab) = separa(a);
    let (br, bg, bb) = separa(b);
    let r = ((ar * eva + br * evb + 8) >> 4).min(31);
    let g = ((ag * eva + bg * evb + 8) >> 4).min(63);
    let bl = ((ab * eva + bb * evb + 8) >> 4).min(31);
    junta(r, g, bl)
}

fn aclarar(c: u16, evy: u32) -> u16 {
    let (r, g, b) = separa(c);
    junta(
        r + (((31 - r) * evy + 8) >> 4),
        g + (((63 - g) * evy + 8) >> 4),
        b + (((31 - b) * evy + 8) >> 4),
    )
}

fn oscurecer(c: u16, evy: u32) -> u16 {
    let (r, g, b) = separa(c);
    junta(
        r - ((r * evy + 7) >> 4),
        g - ((g * evy + 7) >> 4),
        b - ((b * evy + 7) >> 4),
    )
}

fn dos_capas(
    bgs: &[Option<Vec<u16>>; 4],
    prio: &[(u8, usize)],
    x: usize,
    o: &Pixel,
    fondo: u16,
) -> (u16, u8, u16, u8) {
    let mut fc = fondo;
    let mut fl = 5u8;
    let mut sc = fondo;
    let mut sl = 5u8;
    let mut encontrado = false;
    let mut obj_hecho = o.prio == 0xFF;
    for &(bp, b) in prio {
        if !obj_hecho && o.prio <= bp {
            if !encontrado {
                fc = o.color;
                fl = 4;
                encontrado = true;
            } else {
                sc = o.color;
                sl = 4;
                obj_hecho = true;
                break;
            }
            obj_hecho = true;
        }
        if let Some(l) = &bgs[b] {
            let c = l[x];
            if c != TRANSPARENTE {
                if !encontrado {
                    fc = c;
                    fl = b as u8;
                    encontrado = true;
                } else {
                    sc = c;
                    sl = b as u8;
                    break;
                }
            }
        }
    }
    if !obj_hecho {
        if !encontrado {
            fc = o.color;
            fl = 4;
        } else {
            sc = o.color;
            sl = 4;
        }
    }
    (fc, fl, sc, sl)
}

fn componer(io: &[u8], pal: &[u8], bgs: &[Option<Vec<u16>>; 4], obj: &[Pixel]) -> Vec<u16> {
    let fondo = color_pal(pal, 0);
    let mut prio: Vec<(u8, usize)> = Vec::new();
    for b in 0..4 {
        if bgs[b].is_some() {
            prio.push(((io16(io, 0x08 + b * 2) & 3) as u8, b));
        }
    }
    prio.sort_unstable();
    let bldcnt = io16(io, 0x50);
    let modo_mezcla = (bldcnt >> 6) & 3;
    let bldalpha = io16(io, 0x52);
    let eva = (bldalpha & 0x1F).min(16) as u32;
    let evb = ((bldalpha >> 8) & 0x1F).min(16) as u32;
    let evy = (io16(io, 0x54) & 0x1F).min(16) as u32;
    let mut colores = vec![0u16; ANCHO];
    for x in 0..ANCHO {
        let o = obj[x];
        let obj_vis = o.prio != 0xFF;
        let (fc, fl, sc, sl) = dos_capas(bgs, &prio, x, &o, fondo);
        let semi = obj_vis && fl == 4 && o.semi;
        colores[x] = if semi && bldcnt & (0x100u16 << sl) != 0 {
            alpha(fc, sc, eva, evb)
        } else if bldcnt & (1u16 << fl) != 0 {
            match modo_mezcla {
                1 if bldcnt & (0x100u16 << sl) != 0 => alpha(fc, sc, eva, evb),
                2 => aclarar(fc, evy),
                3 => oscurecer(fc, evy),
                _ => fc,
            }
        } else {
            fc
        };
    }
    colores
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 4 {
        eprintln!("uso: shim_vram_bg <ruta.gba> <secuencia.txt> <cuadro>");
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

    // PPU por software en V0 y V1, línea a línea.
    let mut ppu: Vec<Vec<u16>> = vec![vec![0u16; n]; 2];
    let mut replay = gba_core::Machine::new(rom);
    for cap in capturas.iter() {
        replay.bus.palette = cap.palette.clone();
        replay.bus.vram = cap.vram.clone();
        replay.bus.oam = cap.oam.clone();
        let ini = cap.linea * ANCHO;
        for v in 0..2 {
            replay.bus.io = io_con_dispcnt(&cap.io, dispcnt_de(v, io16(&cap.io, 0)));
            replay.bus.aff_ref = cap.aff_ref;
            replay.bus.aff_dirty = cap.aff_dirty;
            gba_core::ppu::render_scanline(&mut replay.bus, cap.linea);
            ppu[v][ini..ini + ANCHO].copy_from_slice(&replay.bus.framebuffer[ini..ini + ANCHO]);
        }
    }

    // Capas de fondo y sprites por línea.
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

    // Control 1: composición completa en V0 y V1.
    let mut rec: Vec<Vec<u16>> = vec![vec![0u16; n]; 2];
    for cap in capturas.iter() {
        let base = io16(&cap.io, 0);
        if base & 7 != 0 || base & 0xE000 != 0 || base & 0x80 != 0 {
            eprintln!("línea {}: modo de vídeo, ventana o forced blank no reconstruidos", cap.linea);
            std::process::exit(1);
        }
        let ini = cap.linea * ANCHO;
        for v in 0..2 {
            let disp = dispcnt_de(v, base);
            let bgs: [Option<Vec<u16>>; 4] = std::array::from_fn(|b| {
                if disp & (0x100 << b) != 0 {
                    Some(bg_linea[cap.linea][b].clone())
                } else {
                    None
                }
            });
            let obj: Vec<Pixel> = if disp & BIT_OBJ != 0 {
                obj_linea[cap.linea].clone()
            } else {
                vec![SIN_OBJ; ANCHO]
            };
            let colores = componer(&cap.io, &cap.palette, &bgs, &obj);
            rec[v][ini..ini + ANCHO].copy_from_slice(&colores);
        }
    }
    let ctrl_v0 = (0..n).filter(|&p| rec[0][p] != ppu[0][p]).count();
    let ctrl_v1 = (0..n).filter(|&p| rec[1][p] != ppu[1][p]).count();

    // Control 2: el color de fondo por línea debe ser el de la paleta final del shim.
    let fondo_fin = color_pal(&pal_fin, 0);
    let ctrl_paleta = capturas
        .iter()
        .filter(|cap| color_pal(&cap.palette, 0) != fondo_fin)
        .count();

    if ctrl_v0 != 0 || ctrl_v1 != 0 || ctrl_paleta != 0 {
        eprintln!(
            "control falla: V0 {ctrl_v0}, V1 {ctrl_v1}, paleta {ctrl_paleta}; no se escribe el informe"
        );
        std::process::exit(1);
    }

    // Shim con solo BG0 activa y estado de fin.
    let mut gpu = match Gpu::nueva() {
        Ok(g) => g,
        Err(e) => {
            eprintln!("{e}");
            std::process::exit(1);
        }
    };
    let io_k = io_con_dispcnt(&io_fin, 0x100);
    let shim = gpu.renderiza(&vram_fin, &oam_fin, &pal_fin, &io_k);
    let shim_px = |p: usize| [shim[p * 3], shim[p * 3 + 1], shim[p * 3 + 2]];
    let backdrop = rgb8(fondo_fin);

    // Configuración que recibe la GPU (PpuShim::decode_io_regs) y bits de BG0CNT leídos directamente.
    let mut pp = PpuShim::new();
    pp.decode_io_regs(&io_k);
    let cfg = pp.bg_layers[0];
    let cnt = io16(&io_k, 0x08);
    let base_car_io = ((cnt >> 2) & 3) as u8;
    let base_mapa_io = ((cnt >> 8) & 0x1F) as u8;
    let ocho_io = cnt & 0x80 != 0;
    let tam_io = ((cnt >> 14) & 3) as u8;

    // OBJ apagado en DISPCNT de esta medición.
    let obj_apagado = io16(&io_k, 0) & BIT_OBJ == 0;

    let mut candidatos = 0usize;
    let mut bytes_entrada_distintos = 0usize;
    let mut bytes_dato_distintos = 0usize;
    let mut uniformes_distintos = 0usize;
    let mut ninguno_distinto = 0usize;
    let mut replica_reproduce = 0usize;
    let mut con_sprite_dibujado = 0usize;
    let mut con_sprite_si_obj_activo = 0usize;
    let mut dato_mas_alla_64k = 0usize;

    for p in 0..n {
        let (l, x) = (p / ANCHO, p % ANCHO);
        let ref_fin = fondo_texto(&io_fin, &vram_fin, &pal_fin, 0, l)[x];
        if ref_fin != TRANSPARENTE || cerca_pixel(shim_px(p), backdrop) {
            continue;
        }
        candidatos += 1;

        let rep = fs_bg_replica(&cfg, &vram_fin, &pal_fin, x, l);
        if rep.color != TRANSPARENTE && cerca_pixel(shim_px(p), rgb8(rep.color)) {
            replica_reproduce += 1;
        }

        // H90: bytes que lee la GPU frente al snapshot en las direcciones de la entrada y del dato.
        let dif_entrada = byte_gpu(&vram_fin, rep.entrada_addr) != leer_vram(&vram_fin, rep.entrada_addr)
            || byte_gpu(&vram_fin, rep.entrada_addr + 1) != leer_vram(&vram_fin, rep.entrada_addr + 1);
        let dif_dato = byte_gpu(&vram_fin, rep.dato_addr) != leer_vram(&vram_fin, rep.dato_addr);
        if dif_entrada {
            bytes_entrada_distintos += 1;
        }
        if dif_dato {
            bytes_dato_distintos += 1;
        }
        if rep.dato_addr >= 0x1_0000 {
            dato_mas_alla_64k += 1;
        }

        // H91: uniformes recibidos frente a los bits de BG0CNT.
        let uni_dif = cfg.char_base_block != base_car_io
            || cfg.screen_base_block != base_mapa_io
            || cfg.is_8bpp != ocho_io
            || cfg.screen_size != tam_io;
        if uni_dif {
            uniformes_distintos += 1;
        }
        if !dif_entrada && !dif_dato && !uni_dif {
            ninguno_distinto += 1;
        }

        // H93: ¿hay un sprite ganador en este píxel si se ignora el bit OBJ de DISPCNT?
        let io_obj = io_con_dispcnt(&io_fin, 0x100 | BIT_OBJ);
        let obj_si_activo = sprites(io16(&io_obj, 0), &oam_fin, &vram_fin, &pal_fin, l)
            .map(|v| v[x].prio != 0xFF)
            .unwrap_or(false);
        if obj_si_activo {
            con_sprite_si_obj_activo += 1;
        }
        if obj_apagado && obj_si_activo {
            con_sprite_dibujado += 1;
        }
    }

    let mut lineas = vec![
        format!("resumen cuadro {objetivo}: subida de VRAM y uniformes de BG0 frente a fondo_texto"),
        format!("control reconstruccion V0: {ctrl_v0}"),
        format!("control reconstruccion V1: {ctrl_v1}"),
        format!("control paleta fondo: {ctrl_paleta}"),
        format!("bg0 710 candidatos: {candidatos} (esperados {ESPERADOS_710})"),
        format!("bg0 710 bytes entrada distintos (H90): {bytes_entrada_distintos}"),
        format!("bg0 710 bytes dato distintos (H90): {bytes_dato_distintos}"),
        format!("bg0 710 uniformes distintos (H91): {uniformes_distintos}"),
        format!("bg0 710 ninguno distinto (H92): {ninguno_distinto}"),
        format!("bg0 710 replica reproduce color shim: {replica_reproduce}"),
        format!("bg0 710 dato mas alla de 64K (H94): {dato_mas_alla_64k}"),
        format!("bg0 710 obj apagado en dispcnt: {}", obj_apagado as u8),
        format!("bg0 710 sprite ganador si obj activo: {con_sprite_si_obj_activo}"),
        format!("bg0 710 sprite dibujado con obj apagado (H93): {con_sprite_dibujado}"),
    ];

    if candidatos != ESPERADOS_710 {
        lineas.push(format!(
            "PARAR: candidatos {candidatos} distinto de {ESPERADOS_710}; no se escribe el informe"
        ));
        for l in &lineas {
            println!("{l}");
        }
        std::process::exit(1);
    }

    for l in &lineas {
        println!("{l}");
    }
    let _ = std::fs::create_dir_all("volcados");
    let ruta = format!("volcados/subida-vram-bg0-cuadro-{objetivo}.txt");
    if let Err(e) = std::fs::write(&ruta, format!("{}\n", lineas.join("\n"))) {
        eprintln!("no se pudo escribir {ruta}: {e}");
        std::process::exit(1);
    }
}
