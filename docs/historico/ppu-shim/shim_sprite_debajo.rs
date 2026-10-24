//! Experimento de docs/propuestas/2026-10-17_orden-de-pintado-del-shim-cuadro-1000.md.
//!
//! Para el grupo "ninguno" del cuadro N (píxeles de aportación que no son A, B ni C en
//! `shim_origen_x`), mide si el color X del shim sale del orden de pintado de `render_frame`:
//!   H60: X = mezcla_50(sprite ganador, sprite que el shim dibuja antes que él en ese píxel).
//!   H61 (si no H60): X = simulación completa del píxel en el orden del shim (fondos y sprites,
//!        con fondo vacío como base, opacos que reemplazan y semitransparentes que mezclan 50/50).
//! Control: reconstrucción V0 y V1 = 0, control de paleta del fondo = 0, control de ganador
//! (el ganador por lista de sprites coincide con el de `sprites`) = 0, y el grupo "ninguno" = 2.523.
//! Si no, no escribe el informe.
//!
//! Escribe volcados/sprite-debajo-cuadro-<N>.txt.
//!
//! Uso:
//!   cargo run --release --features proto --example shim_sprite_debajo -- <ruta.gba> <secuencia.txt> <cuadro>

#[path = "comun/mod.rs"]
mod comun;

use comun::color::{canal_8bit, cerca_pixel};
use comun::entrada::leer_secuencia;
use comun::gpu::{ALTO, ANCHO, Gpu};
use comun::registro::{BIT_OBJ, dispcnt_de, io16, io_con_variante};

const TRANSPARENTE: u16 = 0xFFFF;
const NINGUNO: usize = usize::MAX;
const OBJ_TAMANOS: [[(usize, usize); 4]; 3] = [
    [(8, 8), (16, 16), (32, 32), (64, 64)],
    [(16, 8), (32, 8), (32, 16), (64, 32)],
    [(8, 16), (8, 32), (16, 32), (32, 64)],
];
const TAMANO_NINGUNO_ESPERADO: usize = 2523;

/// Fragmento de sprite: color, prioridad (0xFF = ninguno), semitransparencia, índice OAM e índice de paleta.
#[derive(Clone, Copy)]
struct Pixel {
    color: u16,
    prio: u8,
    semi: bool,
    idx: usize,
    ind: usize,
}

const SIN_OBJ: Pixel = Pixel {
    color: TRANSPARENTE,
    prio: 0xFF,
    semi: false,
    idx: NINGUNO,
    ind: NINGUNO,
};

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

/// Mezcla 50/50 en el espacio de valores de 8 bits (lo que hace el GPU con alfa 0,5).
fn mezcla_rgb(a: [u8; 3], b: [u8; 3]) -> [u8; 3] {
    [
        ((a[0] as u32 + b[0] as u32 + 1) / 2) as u8,
        ((a[1] as u32 + b[1] as u32 + 1) / 2) as u8,
        ((a[2] as u32 + b[2] as u32 + 1) / 2) as u8,
    ]
}

/// Fondo de texto (modo 0) de la línea: colores ya con paleta, o TRANSPARENTE.
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

/// Todos los fragmentos de sprite que cubren cada píxel de la línea (sin elegir ganador).
fn sprites_todos(disp: u16, oam: &[u8], vram: &[u8], pal: &[u8], linea: usize) -> Result<Vec<Vec<Pixel>>, String> {
    let mut out: Vec<Vec<Pixel>> = vec![Vec::new(); ANCHO];
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
            out[sx as usize].push(Pixel {
                color: color_pal(pal, 256 + idx),
                prio,
                semi: modo == 1,
                idx: i,
                ind: 256 + idx,
            });
        }
    }
    Ok(out)
}

/// Ganador de una lista: prioridad menor; a igual prioridad, OAM menor (como en `sprites`).
fn ganador(lista: &[Pixel]) -> Pixel {
    lista
        .iter()
        .min_by_key(|p| (p.prio, p.idx))
        .copied()
        .unwrap_or(SIN_OBJ)
}

/// Simulación del orden de pintado de `render_frame` para un píxel: prioridad 3 a 0; en cada
/// prioridad, fondos con esa prioridad (BG3 a BG0) y luego sprites con esa prioridad (OAM mayor
/// primero). Opaco reemplaza; semitransparente mezcla 50/50 con lo que hay.
fn simula(
    bgs_px: [u16; 4],
    bg_info: [(bool, u8); 4],
    frags: &[Pixel],
    fondo: [u8; 3],
) -> [u8; 3] {
    let mut cur = fondo;
    for p in (0..=3u8).rev() {
        for b in (0..4).rev() {
            let (activo, prio_bg) = bg_info[b];
            if activo && prio_bg == p && bgs_px[b] != TRANSPARENTE {
                cur = rgb8(bgs_px[b]);
            }
        }
        let mut sp: Vec<&Pixel> = frags.iter().filter(|f| f.prio == p).collect();
        sp.sort_by(|a, b| b.idx.cmp(&a.idx));
        for f in sp {
            cur = if f.semi {
                mezcla_rgb(rgb8(f.color), cur)
            } else {
                rgb8(f.color)
            };
        }
    }
    cur
}

/// Las dos capas de arriba en un píxel, como `top_two_layers` de la PPU, con control 0x3F.
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

/// Composición de una línea según la regla del camino general de la PPU (sin ventanas).
fn componer(
    io: &[u8],
    pal: &[u8],
    bgs: &[Option<Vec<u16>>; 4],
    obj: &[Pixel],
) -> Vec<u16> {
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


/// Sprite ganador por píxel de la línea (OAM en orden; prioridad estrictamente menor gana).
/// Copia de `shim_origen_x::sprites`; referencia independiente para el control de ganador.
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
                    ind: 256 + idx,
                };
            }
        }
    }
    Ok(out)
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 4 {
        eprintln!("uso: shim_sprite_debajo <ruta.gba> <secuencia.txt> <cuadro>");
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

    // PPU por software en las cuatro variantes, línea a línea.
    let n = ANCHO * ALTO;
    let mut ppu: Vec<Vec<u16>> = vec![vec![0u16; n]; 4];
    let mut replay = gba_core::Machine::new(rom);
    for cap in capturas.iter() {
        replay.bus.palette = cap.palette.clone();
        replay.bus.vram = cap.vram.clone();
        replay.bus.oam = cap.oam.clone();
        let ini = cap.linea * ANCHO;
        for v in 0..4 {
            replay.bus.io = io_con_variante(&cap.io, v);
            replay.bus.aff_ref = cap.aff_ref;
            replay.bus.aff_dirty = cap.aff_dirty;
            gba_core::ppu::render_scanline(&mut replay.bus, cap.linea);
            ppu[v][ini..ini + ANCHO].copy_from_slice(&replay.bus.framebuffer[ini..ini + ANCHO]);
        }
    }
    let mascara: Vec<usize> = (0..n)
        .filter(|&p| ppu[0][p] != ppu[1][p] && ppu[2][p] == ppu[3][p])
        .collect();
    let total = mascara.len();

    // Capas de fondo, sprite ganador (sprites) y lista completa de fragmentos por línea (V0).
    let mut bg_linea: Vec<Vec<Vec<u16>>> = vec![Vec::new(); ALTO];
    let mut obj_linea: Vec<Vec<Pixel>> = vec![Vec::new(); ALTO];
    let mut frags_linea: Vec<Vec<Vec<Pixel>>> = vec![Vec::new(); ALTO];
    let mut info_bg_linea: Vec<[(bool, u8); 4]> = vec![[(false, 0); 4]; ALTO];
    for cap in capturas.iter() {
        let base = io16(&cap.io, 0);
        bg_linea[cap.linea] = (0..4)
            .map(|b| fondo_texto(&cap.io, &cap.vram, &cap.palette, b, cap.linea))
            .collect();
        info_bg_linea[cap.linea] = std::array::from_fn(|b| {
            (
                base & (0x100 << b) != 0,
                (io16(&cap.io, 0x08 + b * 2) & 3) as u8,
            )
        });
        let frags = if base & BIT_OBJ != 0 {
            match sprites_todos(base, &cap.oam, &cap.vram, &cap.palette, cap.linea) {
                Ok(f) => f,
                Err(e) => {
                    eprintln!("línea {}: {e}", cap.linea);
                    std::process::exit(1);
                }
            }
        } else {
            vec![Vec::new(); ANCHO]
        };
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
        frags_linea[cap.linea] = frags;
    }

    // Control 1: composición completa en V0 y V1, en toda la pantalla.
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

    // Control 2: el color de fondo de cada línea debe ser el de la paleta final del shim.
    let fondo_fin = color_pal(&pal_fin, 0);
    let ctrl_paleta = capturas
        .iter()
        .filter(|cap| color_pal(&cap.palette, 0) != fondo_fin)
        .count();

    // Control 3: el ganador de la lista de fragmentos coincide con el de `sprites`.
    let ctrl_ganador = (0..ALTO)
        .map(|l| {
            (0..ANCHO)
                .filter(|&x| {
                    let g = ganador(&frags_linea[l][x]);
                    let o = obj_linea[l][x];
                    g.prio != o.prio || (g.prio != 0xFF && g.idx != o.idx)
                })
                .count()
        })
        .sum::<usize>();

    if ctrl_v0 != 0 || ctrl_v1 != 0 || ctrl_paleta != 0 || ctrl_ganador != 0 {
        eprintln!(
            "control falla: V0 {ctrl_v0}, V1 {ctrl_v1}, paleta {ctrl_paleta}, ganador {ctrl_ganador}; no se escribe el informe"
        );
        std::process::exit(1);
    }

    // X: color del shim con estado de fin en V0.
    let mut gpu = match Gpu::nueva() {
        Ok(g) => g,
        Err(e) => {
            eprintln!("{e}");
            std::process::exit(1);
        }
    };
    let x_rgb = gpu.renderiza(&vram_fin, &oam_fin, &pal_fin, &io_con_variante(&io_fin, 0));

    // Simulación del orden de pintado en toda la pantalla (dato de confianza del modelo).
    let fondo_rgb = rgb8(fondo_fin);
    let mut sim_pantalla = vec![[0u8; 3]; n];
    for p in 0..n {
        let (l, x) = (p / ANCHO, p % ANCHO);
        let bgs_px: [u16; 4] = std::array::from_fn(|b| bg_linea[l][b][x]);
        sim_pantalla[p] = simula(bgs_px, info_bg_linea[l], &frags_linea[l][x], fondo_rgb);
    }
    let modelo_mal = (0..n)
        .filter(|&p| !cerca_pixel(sim_pantalla[p], [x_rgb[p * 3], x_rgb[p * 3 + 1], x_rgb[p * 3 + 2]]))
        .count();

    // Clasificación del grupo "ninguno" (A, luego B, luego C, como en shim_origen_x) y H60/H61.
    let mut ninguno_total = 0usize;
    let mut h60 = 0usize;
    let mut h61 = 0usize;
    let mut sin_hipotesis = 0usize;
    let mut cubre = [0usize; 3];
    for &p in &mascara {
        let (l, x) = (p / ANCHO, p % ANCHO);
        let xr = [x_rgb[p * 3], x_rgb[p * 3 + 1], x_rgb[p * 3 + 2]];
        let bg2 = bg_linea[l][2][x];
        let o = obj_linea[l][x];
        let tiene_sprite = o.prio != 0xFF;
        if tiene_sprite && bg2 != TRANSPARENTE && cerca_pixel(xr, mezcla_rgb(rgb8(o.color), rgb8(bg2))) {
            continue;
        }
        if cerca_pixel(xr, fondo_rgb) {
            continue;
        }
        let en_c = (0..512).any(|k| k != o.ind && cerca_pixel(xr, rgb8(color_pal(&pal_fin, k))));
        if en_c {
            continue;
        }

        ninguno_total += 1;
        let frags = &frags_linea[l][x];
        match frags.len() {
            0 | 1 => cubre[0] += 1,
            2 => cubre[1] += 1,
            _ => cubre[2] += 1,
        }
        let win = ganador(frags);
        if win.prio == 0xFF || !win.semi {
            sin_hipotesis += 1;
            continue;
        }
        // H60: un solo sprite que el shim dibuja antes que el ganador.
        let antes = |f: &Pixel| f.idx != win.idx && (f.prio > win.prio || (f.prio == win.prio && f.idx > win.idx));
        if frags
            .iter()
            .filter(|f| antes(f))
            .any(|f| cerca_pixel(xr, mezcla_rgb(rgb8(win.color), rgb8(f.color))))
        {
            h60 += 1;
            continue;
        }
        // H61: simulación completa del orden del shim en el píxel.
        if cerca_pixel(xr, sim_pantalla[p]) {
            h61 += 1;
            continue;
        }
        sin_hipotesis += 1;
    }
    let suma = h60 + h61 + sin_hipotesis;

    let lineas = vec![
        format!("resumen cuadro {objetivo}: si el color X del shim sale del orden de pintado en el grupo ninguno"),
        format!("control reconstruccion V0: {ctrl_v0}"),
        format!("control reconstruccion V1: {ctrl_v1}"),
        format!("control paleta fondo: {ctrl_paleta}"),
        format!("control ganador: {ctrl_ganador}"),
        format!("aportacion total: {total}"),
        format!("ninguno total: {ninguno_total}"),
        format!("ninguno H60: {h60}"),
        format!("ninguno H61: {h61}"),
        format!("ninguno sin hipotesis: {sin_hipotesis}"),
        format!("ninguno suma: {suma}"),
        format!("ninguno cubre 1 sprite: {}", cubre[0]),
        format!("ninguno cubre 2 sprites: {}", cubre[1]),
        format!("ninguno cubre 3 o mas sprites: {}", cubre[2]),
        format!("modelo vs shim pantalla: {modelo_mal}"),
    ];
    for l in &lineas {
        println!("{l}");
    }
    if ninguno_total != TAMANO_NINGUNO_ESPERADO {
        eprintln!(
            "el grupo ninguno tiene {ninguno_total} píxeles y se esperaban {TAMANO_NINGUNO_ESPERADO}; no se escribe el informe"
        );
        std::process::exit(1);
    }
    let _ = std::fs::create_dir_all("volcados");
    let ruta = format!("volcados/sprite-debajo-cuadro-{objetivo}.txt");
    if let Err(e) = std::fs::write(&ruta, format!("{}\n", lineas.join("\n"))) {
        eprintln!("no se pudo escribir {ruta}: {e}");
        std::process::exit(1);
    }
}
