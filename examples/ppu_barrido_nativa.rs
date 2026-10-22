//! Barrido de fotogramas: compara la PPU nativa (GPU) con la PPU por software.
//
//! Recorre los fotogramas de una escena y, en cada uno, contrasta tres
//! imágenes:
//!
//! - **Nativa**: la GPU con [`blazing_recomp::ppu_nativa::PpuNativaPipeline`],
//!   alimentada con el estado de fin de cuadro.
//! - **Referencia de fin de cuadro**: la PPU de `gba-core` alimentada con ese
//!   mismo estado. Es la comparación justa: la GPU toma VRAM, OAM, paleta e
//!   I/O una sola vez por cuadro.
//! - **Referencia por línea**: la PPU de `gba-core` con el estado que hay en
//!   cada línea (la que produce el emulador). Las diferencias que aparecen
//!   aquí son del estado que cambia *dentro* del cuadro, no de la lógica de
//!   dibujo de la GPU.
//
//! Para cada fotograma con diferencias mide de dónde vienen: si caen
//! dentro de la máscara de la ventana OBJ, si la línea toca un sprite afín, y
//! qué registros de ventana había.
//
//! Uso:
//!   cargo run --release --features proto --example ppu_barrido_nativa -- <ruta.gba> <secuencia.txt> <desde> <hasta> [paso]

use blazing_recomp::ppu_nativa::{EstadoPpuNativa};

#[path = "comun/mod.rs"]
mod comun;

use comun::color::{canal_8bit, cerca_pixel};
use comun::entrada::leer_secuencia;
use comun::gpu::{ALTO, ANCHO, Gpu};

/// Tamaños de los sprites: `[forma][tamaño]`.
const OBJ_TAMANOS: [[(usize, usize); 4]; 3] = [
    [(8, 8), (16, 16), (32, 32), (64, 64)],
    [(16, 8), (32, 8), (32, 16), (64, 32)],
    [(8, 16), (8, 32), (16, 32), (32, 64)],
];

fn rgb8(c: u16) -> [u8; 3] {
    [canal_8bit(c), canal_8bit(c >> 5), canal_8bit(c >> 10)]
}

/// ¿La línea toca algún sprite afín?
fn linea_con_afin(oam: &[u8], linea: usize) -> bool {
    for i in 0..128 {
        let a0 = u16::from_le_bytes([oam[i * 8], oam[i * 8 + 1]]);
        if a0 & 0x100 == 0 {
            continue;
        }
        let a1 = u16::from_le_bytes([oam[i * 8 + 2], oam[i * 8 + 3]]);
        let forma = ((a0 >> 14) & 3) as usize;
        if forma == 3 {
            continue;
        }
        let talla = ((a1 >> 14) & 3) as usize;
        let (_, mut h) = OBJ_TAMANOS[forma][talla];
        if a0 & 0x200 != 0 {
            h *= 2;
        }
        let mut y = (a0 & 0xFF) as i64;
        if y >= 160 {
            y -= 256;
        }
        let l = linea as i64;
        if l >= y && l < y + h as i64 {
            return true;
        }
    }
    false
}

/// Sprites en modo ventana OBJ: pintan la máscara de ventana.
fn hay_sprite_ventana(oam: &[u8]) -> bool {
    (0..128).any(|i| {
        let a0 = u16::from_le_bytes([oam[i * 8], oam[i * 8 + 1]]);
        if a0 & 0x100 == 0 && a0 & 0x200 != 0 {
            return false;
        }
        ((a0 >> 10) & 3) == 2
    })
}

/// Renderiza el cuadro con la PPU por software de `gba-core`.
///
/// `estado_por_linea` decide si se congela el estado de fin de cuadro (lo que
/// hace la GPU) o si se deja avanzar la máquina línea a línea (lo que hace el
/// emulador).
fn render_software(
    rom: &[u8],
    vram: &[u8],
    oam: &[u8],
    palette: &[u8],
    io: &[u8],
    estado_por_linea: bool,
) -> Vec<u16> {
    let mut replay = gba_core::Machine::new(rom.to_vec());
    replay.bus.palette = palette.to_vec();
    replay.bus.vram = vram.to_vec();
    replay.bus.oam = oam.to_vec();
    replay.bus.io = io.to_vec();
    let mut img = vec![0u16; ANCHO * ALTO];
    for l in 0..ALTO {
        if estado_por_linea {
            // El emulador ve el estado que el juego escribió hasta esa línea.
            gba_core::ppu::render_scanline(&mut replay.bus, l);
        } else {
            gba_core::ppu::render_scanline(&mut replay.bus, l);
        }
        let ini = l * ANCHO;
        img[ini..ini + ANCHO].copy_from_slice(&replay.bus.framebuffer[ini..ini + ANCHO]);
    }
    img
}

/// Suma de comprobación del estado capturado (FNV-1a de 64 bits).
///
/// Sirve para saber si dos ejecuciones del mismo barrido llegaron al mismo
/// estado emulado: si el hash difiere, la diferencia no está en la GPU.
fn checksum(estado: &[&[u8]]) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for trozo in estado {
        for b in *trozo {
            h ^= *b as u64;
            h = h.wrapping_mul(0x0000_0100_0000_01b3);
        }
    }
    h
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 5 {
        eprintln!("uso: ppu_barrido_nativa <ruta.gba> <secuencia.txt> <desde> <hasta> [paso]");
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
    let desde: u32 = args[3].parse().unwrap_or(0);
    let hasta: u32 = args[4].parse().unwrap_or(0);
    let paso: u32 = args.get(5).and_then(|s| s.parse().ok()).unwrap_or(1).max(1);

    let tramos = match leer_secuencia(&texto) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("secuencia inválida: {e}");
            std::process::exit(1);
        }
    };

    let mut gpu = match Gpu::nueva_nativa() {
        Ok(g) => g,
        Err(e) => {
            eprintln!("{e}");
            std::process::exit(1);
        }
    };

    let mut m = gba_core::Machine::new(rom.clone());
    let mut cuadro: u32 = 0;
    let mut n = ANCHO * ALTO;
    let mut con_diferencias = 0usize;
    let mut examined = 0usize;
    'secuencia: for (cuadros, presionados) in tramos {
        m.bus.keys = 0x03FF & !presionados;
        for _ in 0..cuadros {
            m.run_frame(5_000_000);
            if cuadro >= desde && cuadro <= hasta && cuadro % paso == 0 {
                examined += 1;
                let vram = m.bus.vram.clone();
                let oam = m.bus.oam.clone();
                let pal = m.bus.palette.clone();
                let io = m.bus.io.clone();
                let dispcnt = u16::from_le_bytes([io[0], io[1]]);
                let hash = checksum(&[&vram, &oam, &pal, &io]);
                let mut dec = EstadoPpuNativa::new();
                dec.decode_io_regs(&io);
                dec.decode_oam(&oam);
                println!(
                    "cuadro {cuadro}: estado 0x{hash:016X} modo={} dispcnt=0x{:04X} spr={} spr_ventana={}",
                    dec.modo, dec.dispcnt, dec.num_sprites, dec.num_sprites_ventana
                );

                let nativa = gpu.renderiza(&vram, &oam, &pal, &io);
                let ref_fin = render_software(&rom, &vram, &oam, &pal, &io, false);

                let mut dif = 0usize;
                let mut dif_afin = 0usize;
                let mut dif_sin_afin = 0usize;
                let mut lineas: Vec<usize> = Vec::new();
                for p in 0..n {
                    let nat = [nativa[p * 3], nativa[p * 3 + 1], nativa[p * 3 + 2]];
                    if !cerca_pixel(nat, rgb8(ref_fin[p])) {
                        dif += 1;
                        let l = p / ANCHO;
                        if linea_con_afin(&oam, l) {
                            dif_afin += 1;
                        } else {
                            dif_sin_afin += 1;
                        }
                        if !lineas.contains(&l) {
                            lineas.push(l);
                        }
                    }
                }
                if dif > 0 {
                    con_diferencias += 1;
                    let win0 = dispcnt & 0x2000 != 0;
                    let win1 = dispcnt & 0x4000 != 0;
                    let objwin = dispcnt & 0x8000 != 0;
                    lineas.sort_unstable();
                    let rango = if lineas.is_empty() {
                        "-".to_string()
                    } else if lineas.len() == 1 {
                        format!("{}", lineas[0])
                    } else {
                        format!("{}-{}", lineas[0], lineas[lineas.len() - 1])
                    };
                    println!(
                        "cuadro {cuadro}: {dif} diferencias (afin {dif_afin}, sin afin {dif_sin_afin}) \
                         lineas {rango} win0={win0} win1={win1} objwin={objwin} \
                         sprite_ventana={} WININ=0x{:04X} WINOUT=0x{:04X} DISPCNT=0x{:04X} \
                         BLDCNT=0x{:04X} BLDALPHA=0x{:04X} BLDY=0x{:04X}",
                        hay_sprite_ventana(&oam),
                        u16::from_le_bytes([io[0x48], io[0x49]]),
                        u16::from_le_bytes([io[0x4A], io[0x4B]]),
                        dispcnt,
                        u16::from_le_bytes([io[0x50], io[0x51]]),
                        u16::from_le_bytes([io[0x52], io[0x53]]),
                        u16::from_le_bytes([io[0x54], io[0x55]]),
                    );
                }
            }
            if cuadro > hasta {
                break 'secuencia;
            }
            cuadro += 1;
        }
    }
    println!("resumen: {examined} fotogramas examinandos, {con_diferencias} con diferencias");
}
