//! Experimento de docs/propuestas/2026-10-17_capa-ganadora-por-pixel-obj-cuadro-1000.md.
//!
//! Con las capturas por línea del cuadro N (feature `perf-video`):
//!   1. Redibuja cada línea con la PPU por software en cuatro variantes de DISPCNT:
//!      0 completa, 1 sin OBJ, 2 solo OBJ, 3 nada.
//!   2. Reconstruye en este ejemplo la regla de composición del camino general (fondos de texto,
//!      sprites por OAM con prioridad, y los modos de mezcla 1 alpha, 2 aclarar, 3 oscurecer de
//!      BLDCNT) y la compara con la PPU en las cuatro variantes. Si no coincide en algún píxel,
//!      la clasificación no se hace.
//!   3. Para cada píxel de la aportación fuera de cobertura (V0 != V1 y V2 == V3 en la PPU),
//!      clasifica el caso según H15, H16 y H17 de la propuesta.
//!
//! Escribe volcados/capa-ganadora-cuadro-<N>.txt: controles, conteos por categoría y un renglón
//! `pixel` por cada píxel clasificado.
//!
//! Limitaciones: solo reconstruye el modo de vídeo 0, sin ventanas (WIN0, WIN1, OBJWIN), sin
//! sprites afines ni ventanas de sprites. Si la captura usa algo de eso, el ejemplo para.
//!
//! Uso:
//!   cargo run --release --features proto --example ppu_capa_ganadora -- <ruta.gba> <secuencia.txt> <cuadro>

#[path = "comun/mod.rs"]
mod comun;

use comun::entrada::leer_secuencia;
use comun::gpu::{ALTO, ANCHO};
use comun::registro::{dispcnt_de, io16};

const TRANSPARENTE: u16 = 0xFFFF;
const NINGUNO: usize = usize::MAX;
/// Tamaños de sprite por forma (cuadrado, horizontal, vertical) y tamaño (0..3).
const OBJ_TAMANOS: [[(usize, usize); 4]; 3] = [
    [(8, 8), (16, 16), (32, 32), (64, 64)],
    [(16, 8), (32, 8), (32, 16), (64, 32)],
    [(8, 16), (8, 32), (16, 32), (32, 64)],
];

/// Píxel de sprite: color, prioridad (0xFF = ninguno), semitransparencia, índice OAM del ganador.
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

/// Color de la paleta de índice `i` (0..511), sin el bit 15.
fn color_pal(pal: &[u8], i: usize) -> u16 {
    u16::from_le_bytes([pal[2 * i], pal[2 * i + 1]]) & 0x7FFF
}

/// Lectura de VRAM para sprites (toda la VRAM, 0x18000 bytes).
fn leer_vram(vram: &[u8], addr: usize) -> u8 {
    vram.get(addr).copied().unwrap_or(0)
}

/// Lectura de VRAM para fondos: solo los primeros 64 KB, como en la PPU.
fn leer_bg(vram: &[u8], addr: usize) -> u8 {
    if addr < 0x1_0000 {
        vram.get(addr).copied().unwrap_or(0)
    } else {
        0
    }
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

/// Sprites de la línea, por OAM. Un sprite gana un píxel si su prioridad es menor que la del
/// píxel actual; con prioridad igual gana el que está antes en OAM.
fn sprites(
    disp: u16,
    oam: &[u8],
    vram: &[u8],
    pal: &[u8],
    linea: usize,
) -> Result<Vec<Pixel>, String> {
    let mut out = vec![SIN_OBJ; ANCHO];
    let un_dim = disp & 0x40 != 0;
    let lin = linea as i64;
    for i in 0..128 {
        let a0 = u16::from_le_bytes([oam[i * 8], oam[i * 8 + 1]]);
        if a0 & 0x100 != 0 {
            return Err(format!("sprite afín {i}: no reconstruido"));
        }
        if a0 & 0x200 != 0 {
            continue; // deshabilitado
        }
        let forma = ((a0 >> 14) & 3) as usize;
        if forma == 3 {
            continue; // prohibido
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

/// Canales como la unidad de mezcla: rojo y azul de 5 bits, verde de 6 bits (5 bits duplicados).
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
/// Capas: 0-3 fondos, 4 sprite, 5 fondo de pantalla. Devuelve (color, capa, color 2.ª, capa 2.ª).
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
/// Devuelve el color de cada píxel y la capa de arriba (0-3 fondos, 4 sprite, 5 fondo).
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
        // Misma cadena que el camino general: primero el sprite translúcido, luego el efecto de
        // BLDCNT sobre la capa de arriba (la segunda capa solo importa en el alpha).
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

/// Reconstruye una línea con el DISPCNT `disp` de la variante. Para si la captura usa algo que
/// esta reconstrucción no cubre.
fn linea_reconstruida(
    io: &[u8],
    vram: &[u8],
    pal: &[u8],
    oam: &[u8],
    linea: usize,
    disp: u16,
) -> Result<(Vec<u16>, Vec<u8>, Vec<Pixel>), String> {
    if disp & 0x80 != 0 {
        return Err("forced blank: no reconstruido".into());
    }
    if disp & 7 != 0 {
        return Err(format!("modo {} distinto de 0: no reconstruido", disp & 7));
    }
    if disp & 0xE000 != 0 {
        return Err("ventanas activas: no reconstruido".into());
    }
    let mut bgs: [Option<Vec<u16>>; 4] = [None, None, None, None];
    for (b, capa) in bgs.iter_mut().enumerate() {
        if disp & (0x100 << b) != 0 {
            *capa = Some(fondo_texto(io, vram, pal, b, linea));
        }
    }
    let obj = if disp & 0x1000 != 0 {
        sprites(disp, oam, vram, pal, linea)?
    } else {
        vec![SIN_OBJ; ANCHO]
    };
    let (colores, capas) = componer(io, pal, &bgs, &obj);
    Ok((colores, capas, obj))
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 4 {
        eprintln!("uso: ppu_capa_ganadora <ruta.gba> <secuencia.txt> <cuadro>");
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

    // Ejecutar el juego hasta el cuadro pedido, con la captura activa solo durante ese cuadro.
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

    let n = ANCHO * ALTO;
    let mut ppu: Vec<Vec<u16>> = vec![vec![0u16; n]; 4];
    let mut rec: Vec<Vec<u16>> = vec![vec![0u16; n]; 4];
    let mut rec_fl0 = vec![0u8; n];
    let mut rec_fl1 = vec![0u8; n];
    let mut rec_obj0 = vec![SIN_OBJ; n];
    let mut rec_idx2 = vec![NINGUNO; n];

    let mut replay = gba_core::Machine::new(rom);
    for cap in capturas.iter() {
        let base = u16::from_le_bytes([cap.io[0], cap.io[1]]);
        replay.bus.palette = cap.palette.clone();
        replay.bus.vram = cap.vram.clone();
        replay.bus.oam = cap.oam.clone();
        let ini = cap.linea * ANCHO;
        for v in 0..4 {
            let d = dispcnt_de(v, base);
            replay.bus.io = cap.io.clone();
            replay.bus.io[0] = (d & 0xFF) as u8;
            replay.bus.io[1] = (d >> 8) as u8;
            replay.bus.aff_ref = cap.aff_ref;
            replay.bus.aff_dirty = cap.aff_dirty;
            gba_core::ppu::render_scanline(&mut replay.bus, cap.linea);
            ppu[v][ini..ini + ANCHO].copy_from_slice(&replay.bus.framebuffer[ini..ini + ANCHO]);

            let (colores, capas, objs) =
                match linea_reconstruida(&cap.io, &cap.vram, &cap.palette, &cap.oam, cap.linea, d)
                {
                    Ok(t) => t,
                    Err(e) => {
                        eprintln!("línea {}, variante {v}: {e}", cap.linea);
                        std::process::exit(1);
                    }
                };
            rec[v][ini..ini + ANCHO].copy_from_slice(&colores);
            for x in 0..ANCHO {
                match v {
                    0 => {
                        rec_fl0[ini + x] = capas[x];
                        rec_obj0[ini + x] = objs[x];
                    }
                    1 => rec_fl1[ini + x] = capas[x],
                    2 => rec_idx2[ini + x] = objs[x].idx,
                    _ => {}
                }
            }
        }
    }

    // Control: la reconstrucción debe coincidir con la PPU en las cuatro variantes.
    let mut control = [0usize; 4];
    for v in 0..4 {
        control[v] = (0..n).filter(|&p| rec[v][p] != ppu[v][p]).count();
    }

    let mut lineas = vec![format!(
        "resumen cuadro {objetivo}: reconstruccion vs PPU (esperado 0 en cada variante)"
    )];
    for v in 0..4 {
        lineas.push(format!("control reconstruccion V{v}: {}", control[v]));
    }

    let controlado = control.iter().all(|&c| c == 0);
    let mut detalle = String::new();
    if !controlado {
        lineas.push("clasificacion omitida: la reconstruccion no coincide con la PPU".to_string());
    } else {
        let mut total = 0usize;
        // Categorías: sin_sprite, H15, H16, H17, otro.
        let mut cuenta = [0usize; 5];
        for p in 0..n {
            if !(ppu[0][p] != ppu[1][p] && ppu[2][p] == ppu[3][p]) {
                continue;
            }
            total += 1;
            let x = p % ANCHO;
            let l = p / ANCHO;
            let idx0 = rec_obj0[p].idx;
            let f0 = rec_fl0[p];
            let f1 = rec_fl1[p];
            let (cat, pos) = if idx0 == NINGUNO {
                ("sin_sprite", 0)
            } else if f0 < 4 {
                ("H16", 2)
            } else if rec_idx2[p] != idx0 {
                ("H17", 3)
            } else if rec_obj0[p].color == ppu[3][p] {
                ("H15", 1)
            } else {
                ("otro", 4)
            };
            cuenta[pos] += 1;
            let sprite = if idx0 == NINGUNO {
                "-".to_string()
            } else {
                idx0.to_string()
            };
            detalle.push_str(&format!(
                "pixel x={x} linea={l} fl0={f0} fl1={f1} sprite={sprite} semi={} V0={:04X} V1={:04X} V2={:04X} V3={:04X} cat={cat}\n",
                rec_obj0[p].semi as u8, ppu[0][p], ppu[1][p], ppu[2][p], ppu[3][p]
            ));
        }
        lineas.push(format!("capa aportacion fuera total: {total}"));
        lineas.push(format!("capa categoria sin_sprite: {}", cuenta[0]));
        lineas.push(format!("capa categoria H15: {}", cuenta[1]));
        lineas.push(format!("capa categoria H16: {}", cuenta[2]));
        lineas.push(format!("capa categoria H17: {}", cuenta[3]));
        lineas.push(format!("capa categoria otro: {}", cuenta[4]));
        let suma: usize = cuenta.iter().sum();
        lineas.push(format!("capa categoria suma: {suma}"));
    }

    // Capa debajo del sprite en V1 para los mismos píxeles (propuesta capa-debajo-del-sprite).
    // Se escribe aparte; el informe capa-ganadora-cuadro-<N>.txt no cambia.
    let mut debajo_lineas: Vec<String> = Vec::new();
    if controlado {
        let mut debajo = [0usize; 6]; // fl1: 0..3 fondos, 5 fondo de pantalla
        let mut total_d = 0usize;
        let mut igual_v3 = 0usize;
        let mut igual_sprite = 0usize;
        for p in 0..n {
            if !(ppu[0][p] != ppu[1][p] && ppu[2][p] == ppu[3][p]) {
                continue;
            }
            total_d += 1;
            debajo[(rec_fl1[p] as usize).min(5)] += 1;
            if ppu[1][p] == ppu[3][p] {
                igual_v3 += 1;
            }
            if rec_obj0[p].idx != NINGUNO && ppu[1][p] == rec_obj0[p].color {
                igual_sprite += 1;
            }
        }
        debajo_lineas.push(format!("resumen cuadro {objetivo}: capa debajo del sprite en V1"));
        debajo_lineas.push(format!("capa debajo total: {total_d}"));
        debajo_lineas.push(format!("capa debajo fondo 0: {}", debajo[0]));
        debajo_lineas.push(format!("capa debajo fondo 1: {}", debajo[1]));
        debajo_lineas.push(format!("capa debajo fondo 2: {}", debajo[2]));
        debajo_lineas.push(format!("capa debajo fondo 3: {}", debajo[3]));
        debajo_lineas.push(format!("capa debajo fondo de pantalla: {}", debajo[5]));
        let suma_d: usize = debajo.iter().sum();
        debajo_lineas.push(format!("capa debajo suma: {suma_d}"));
        debajo_lineas.push(format!("capa debajo color igual V3: {igual_v3}"));
        debajo_lineas.push(format!("capa debajo color igual sprite: {igual_sprite}"));
        for l in &debajo_lineas {
            println!("{l}");
        }
        let _ = std::fs::create_dir_all("volcados");
        if let Err(e) = std::fs::write(format!("volcados/capa-debajo-cuadro-{objetivo}.txt"), debajo_lineas.join("\n") + "\n") {
            eprintln!("no se pudo escribir capa-debajo-cuadro-{objetivo}.txt: {e}");
        }
    }

    for l in &lineas {
        println!("{l}");
    }
    let _ = std::fs::create_dir_all("volcados");
    let ruta = format!("volcados/capa-ganadora-cuadro-{objetivo}.txt");
    let contenido = format!("{}\n{}", lineas.join("\n"), detalle);
    if let Err(e) = std::fs::write(&ruta, contenido) {
        eprintln!("no se pudo escribir {ruta}: {e}");
    }
}
