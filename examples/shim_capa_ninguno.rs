//! Experimento de docs/propuestas/2026-10-17_capa-bajo-mezcla-ninguno-cuadro-1000.md.
//!
//! Toma los píxeles de aportación del cuadro N que quedan en "origen ninguno" en
//! `shim_origen_x` (no son A, B ni C) y, para cada uno, mide con qué capa de fondo se mezcla
//! el color X del shim:
//!   H50: mezcla_50(sprite, BG0|BG1|BG3), solo para sprites semitransparentes.
//!   H51: mezcla_50(mezcla_50(sprite, BG2), BG_k), k en {0, 1, 3}.
//!   H52: mezcla_50(sprite, fondo vacío).
//! Control: reconstrucción V0 y V1 = 0, control de paleta del fondo = 0, y el grupo "ninguno"
//! debe tener 2.523 píxeles. Si no, no escribe el informe.
//!
//! Escribe volcados/capa-ninguno-cuadro-<N>.txt.
//!
//! Uso:
//!   cargo run --release --features proto --example shim_capa_ninguno -- <ruta.gba> <secuencia.txt> <cuadro>

#[path = "comun/mod.rs"]
mod comun;

use comun::color::{canal_8bit, cerca_pixel};
use comun::entrada::leer_secuencia;
use comun::gpu::{ALTO, ANCHO, Gpu};
use comun::registro::{dispcnt_de, io16, io_con_variante};

const TRANSPARENTE: u16 = 0xFFFF;
const NINGUNO: usize = usize::MAX;
const OBJ_TAMANOS: [[(usize, usize); 4]; 3] = [
    [(8, 8), (16, 16), (32, 32), (64, 64)],
    [(16, 8), (32, 8), (32, 16), (64, 32)],
    [(8, 16), (8, 32), (16, 32), (32, 64)],
];
const PAGINAS_BG: [usize; 3] = [0, 1, 3];
const TAMANO_NINGUNO_ESPERADO: usize = 2523;

/// Píxel de sprite: color, prioridad (0xFF = ninguno), semitransparencia, índice OAM e índice de paleta.
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

/// Sprite ganador por píxel de la línea (OAM en orden; prioridad estrictamente menor gana).
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

/// Canales como la unidad de mezcla: rojo y azul de 5 bits, verde de 6 bits.
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

/// Mezcla alpha, con la misma aritmética que la PPU.
fn alpha(a: u16, b: u16, eva: u32, evb: u32) -> u16 {
    let (ar, ag, ab) = separa(a);
    let (br, bg, bb) = separa(b);
    let r = ((ar * eva + br * evb + 8) >> 4).min(31);
    let g = ((ag * eva + bg * evb + 8) >> 4).min(63);
    let bl = ((ab * eva + bb * evb + 8) >> 4).min(31);
    junta(r, g, bl)
}

/// Aclarar (modo 2), como `brighten` de la PPU.
fn aclarar(c: u16, evy: u32) -> u16 {
    let (r, g, b) = separa(c);
    junta(
        r + (((31 - r) * evy + 8) >> 4),
        g + (((63 - g) * evy + 8) >> 4),
        b + (((31 - b) * evy + 8) >> 4),
    )
}

/// Oscurecer (modo 3), como `darken` de la PPU.
fn oscurecer(c: u16, evy: u32) -> u16 {
    let (r, g, b) = separa(c);
    junta(
        r - ((r * evy + 7) >> 4),
        g - ((g * evy + 7) >> 4),
        b - ((b * evy + 7) >> 4),
    )
}

/// Las dos capas de arriba en un píxel, como `top_two_layers` de la PPU, con control 0x3F.
/// Devuelve (color, capa, color 2.ª, capa 2.ª). Capas: 0-3 fondos, 4 sprite, 5 fondo de pantalla.
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

/// Composición de una línea según la regla del camino general de la PPU (sin ventanas).
fn componer(
    io: &[u8],
    pal: &[u8],
    bgs: &[Option<Vec<u16>>; 4],
    obj: &[Pixel],
) -> (Vec<u16>, Vec<u8>) {
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
    let mut capas = vec![5u8; ANCHO];
    for x in 0..ANCHO {
        let o = obj[x];
        let obj_vis = o.prio != 0xFF;
        let (fc, fl, sc, sl) = dos_capas(bgs, &prio, x, &o, fondo);
        let semi = obj_vis && fl == 4 && o.semi;
        let color = if semi && bldcnt & (0x100u16 << sl) != 0 {
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
        colores[x] = color;
        capas[x] = fl;
    }
    (colores, capas)
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 4 {
        eprintln!("uso: shim_capa_ninguno <ruta.gba> <secuencia.txt> <cuadro>");
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

    // Capas de fondo y sprite ganador por línea, con la disposición de V0 (completa).
    let mut bg_linea: Vec<Vec<Vec<u16>>> = vec![Vec::new(); ALTO];
    let mut obj_linea: Vec<Vec<Pixel>> = vec![Vec::new(); ALTO];
    for cap in capturas.iter() {
        let base = io16(&cap.io, 0);
        bg_linea[cap.linea] = (0..4)
            .map(|b| fondo_texto(&cap.io, &cap.vram, &cap.palette, b, cap.linea))
            .collect();
        obj_linea[cap.linea] = match sprites(base, &cap.oam, &cap.vram, &cap.palette, cap.linea) {
            Ok(o) => o,
            Err(e) => {
                eprintln!("línea {}: {e}", cap.linea);
                std::process::exit(1);
            }
        };
    }

    // Control 1: composición completa (la misma regla que ppu_capa_ganadora) en V0 y V1.
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
            let obj: Vec<Pixel> = if disp & 0x1000 != 0 {
                obj_linea[cap.linea].clone()
            } else {
                vec![SIN_OBJ; ANCHO]
            };
            let (colores, _) = componer(&cap.io, &cap.palette, &bgs, &obj);
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

    if ctrl_v0 != 0 || ctrl_v1 != 0 || ctrl_paleta != 0 {
        eprintln!(
            "control falla: V0 {ctrl_v0}, V1 {ctrl_v1}, paleta {ctrl_paleta}; no se escribe el informe"
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

    let fondo_rgb = rgb8(fondo_fin);
    let mut ninguno_total = 0usize;
    let mut sprite_semi = 0usize;
    let mut sprite_no_semi = 0usize;
    let mut sin_sprite = 0usize;
    let mut h50 = [0usize; 3];
    let mut h51 = 0usize;
    let mut h52 = 0usize;
    let mut sin_hipotesis = 0usize;

    for &p in &mascara {
        let (l, x) = (p / ANCHO, p % ANCHO);
        let xr = [x_rgb[p * 3], x_rgb[p * 3 + 1], x_rgb[p * 3 + 2]];
        let bgs: Vec<u16> = (0..4).map(|b| bg_linea[l][b][x]).collect();
        let o = obj_linea[l][x];
        let tiene_sprite = o.prio != 0xFF;

        // Misma clasificación que shim_origen_x: A, luego B, luego C. Solo interesa el grupo "ninguno".
        let es_a = tiene_sprite && bgs[2] != TRANSPARENTE && cerca_pixel(xr, mezcla_rgb(rgb8(o.color), rgb8(bgs[2])));
        if es_a {
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
        if !tiene_sprite {
            sin_sprite += 1;
            continue;
        }
        if !o.semi {
            sprite_no_semi += 1;
            continue;
        }
        sprite_semi += 1;

        // H50: mezcla 50/50 del sprite con BG0, BG1 o BG3.
        let mut hallada = false;
        for (i, &b) in PAGINAS_BG.iter().enumerate() {
            if bgs[b] != TRANSPARENTE && cerca_pixel(xr, mezcla_rgb(rgb8(o.color), rgb8(bgs[b]))) {
                h50[i] += 1;
                hallada = true;
                break;
            }
        }
        if hallada {
            continue;
        }
        // H51: doble mezcla, sprite con BG2 y luego con BG_k.
        if bgs[2] != TRANSPARENTE {
            let m1 = mezcla_rgb(rgb8(o.color), rgb8(bgs[2]));
            if PAGINAS_BG
                .iter()
                .any(|&b| bgs[b] != TRANSPARENTE && cerca_pixel(xr, mezcla_rgb(m1, rgb8(bgs[b]))))
            {
                h51 += 1;
                continue;
            }
        }
        // H52: mezcla 50/50 del sprite con el fondo vacío.
        if cerca_pixel(xr, mezcla_rgb(rgb8(o.color), fondo_rgb)) {
            h52 += 1;
            continue;
        }
        sin_hipotesis += 1;
    }
    let suma = sprite_semi.min(sprite_semi) + sprite_no_semi + sin_sprite;
    let suma_total = h50.iter().sum::<usize>() + h51 + h52 + sin_hipotesis + sprite_no_semi + sin_sprite;

    let lineas = vec![
        format!("resumen cuadro {objetivo}: que capa hay debajo de los pixeles ninguno del shim"),
        format!("control reconstruccion V0: {}", ctrl_v0),
        format!("control reconstruccion V1: {}", ctrl_v1),
        format!("control paleta fondo: {}", ctrl_paleta),
        format!("aportacion total: {total}"),
        format!("ninguno total: {ninguno_total}"),
        format!("ninguno sprite semi: {sprite_semi}"),
        format!("ninguno sprite no semi: {sprite_no_semi}"),
        format!("ninguno sin sprite: {sin_sprite}"),
        format!("ninguno H50 BG0: {}", h50[0]),
        format!("ninguno H50 BG1: {}", h50[1]),
        format!("ninguno H50 BG3: {}", h50[2]),
        format!("ninguno H51: {h51}"),
        format!("ninguno H52: {h52}"),
        format!("ninguno sin hipotesis: {sin_hipotesis}"),
        format!("ninguno suma: {suma_total}"),
    ];
    let _ = suma;
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
    let ruta = format!("volcados/capa-ninguno-cuadro-{objetivo}.txt");
    if let Err(e) = std::fs::write(&ruta, format!("{}\n", lineas.join("\n"))) {
        eprintln!("no se pudo escribir {ruta}: {e}");
        std::process::exit(1);
    }
}
