//! Arnés del fondo afín: la PPU nativa frente a la PPU por software.
//!
//! Monta un estado **sintético** (sin ROM y sin juego) con un fondo afín BG2
//! en modo 1 y lo dibuja dos veces:
//!
//! - **Referencia**: `gba_core::ppu::render_scanline` de `gba-core` (la PPU por
//!   software que usa el emulador), alimentada con ese mismo estado.
//! - **Nativa**: la GPU con [`blazing_recomp::ppu_nativa::PpuNativaPipeline`].
//!
//! Los casos cubren lo que el cuadro real no cubre: el mapa visible, el wrap
//! apagado con la referencia fuera del mapa (la capa sale transparente), el
//! wrap activado con la referencia negativa (la vuelta al mapa, no el recorte)
//! y los cuatro tamaños de mapa (16, 32, 64 y 128 tiles).
//!
//! La aritmética de la referencia es la de `gba-core`: punto fijo 20.8, ancla
//! por línea `ref + línea·(PB, PD)`, paso por píxel `PA`/`PC` y vuelta al mapa
//! con `rem_euclid` del tamaño del mapa.
//!
//! Escribe `volcados/ppu-nativa-afin.txt`.
//!
//! Uso:
//!   cargo run --release --features proto --example ppu_nativa_afin -- [ruta.gba]
//!
//! El argumento opcional es la ruta de la ROM. El estado de este arnés es
//! sintético y no la necesita; se acepta para que la invocación sea la misma
//! que la de `ppu_nativa_compara`.

/// Base de los datos de caracteres: 256 tiles de 8 bits por píxel (16 KB).

#[path = "comun/mod.rs"]
mod comun;

use comun::color::{canal_8bit, cerca_pixel};
use comun::gpu::{ALTO, ANCHO, Gpu};

const BASE_CHAR: usize = 0x0000;
/// Base del mapa de pantalla: hasta 128×128 entradas de 1 byte (16 KB).
const BASE_MAPA: usize = 0x4000;
/// Tamaño de la VRAM que ve la PPU nativa (96 KB).
const VRAM: usize = 96 * 1024;

/// Un caso del arnés: un estado de fondo afín.
struct Caso {
    nombre: &'static str,
    /// Tamaño del mapa en tiles (16, 32, 64 o 128).
    tiles: u32,
    /// ¿El mapa da la vuelta?
    wrap: bool,
    /// Matriz afín en 20.8 con signo.
    pa: u16,
    pb: u16,
    pc: u16,
    pd: u16,
    /// Punto de referencia en 20.8 (28 bits con signo).
    x: u32,
    y: u32,
}

/// Los casos del arnés.
///
/// Los tres primeros son los que explican el comportamiento visible; los
/// cuatro últimos comprueban que el tamaño del mapa (16, 32, 64 o 128 tiles) se
/// decodifica bien: con el wrap activo y la referencia negativa, el módulo del
/// mapa es el que decide dónde cae cada píxel.
const CASOS: [Caso; 7] = [
    // El mapa entero visible: la transformación mete una desviación pequeña
    // para que el muestreo no sea el de una copia literal.
    Caso {
        nombre: "mapa visible (32 tiles)",
        tiles: 32,
        wrap: false,
        pa: 0x100,
        pb: 0x004,
        pc: 0x004,
        pd: 0x100,
        x: 0x0000_0000,
        y: 0x0000_0000,
    },
    // Wrap apagado y referencia fuera del mapa (20 píxeles por encima): las
    // primeras líneas salen transparentes y se ve la capa de debajo.
    Caso {
        nombre: "wrap apagado con la referencia fuera del mapa (32 tiles)",
        tiles: 32,
        wrap: false,
        pa: 0x100,
        pb: 0x000,
        pc: 0x000,
        pd: 0x100,
        x: 0x0000_0000,
        y: 0x0FFF_FF40,
    },
    // El mismo estado con el wrap activado: la referencia negativa da la vuelta
    // al mapa en vez de recortarse.
    Caso {
        nombre: "wrap activado con la referencia negativa (32 tiles)",
        tiles: 32,
        wrap: true,
        pa: 0x100,
        pb: 0x000,
        pc: 0x000,
        pd: 0x100,
        x: 0x0000_0000,
        y: 0x0FFF_FF40,
    },
    // Los cuatro tamaños de mapa, con el wrap activado y la referencia negativa:
    // el módulo del mapa decide dónde cae cada píxel.
    Caso {
        nombre: "mapa de 16 tiles",
        tiles: 16,
        wrap: true,
        pa: 0x080,
        pb: 0x000,
        pc: 0x000,
        pd: 0x080,
        x: 0x0FFF_FF40,
        y: 0x0FFF_FF40,
    },
    Caso {
        nombre: "mapa de 32 tiles",
        tiles: 32,
        wrap: true,
        pa: 0x080,
        pb: 0x000,
        pc: 0x000,
        pd: 0x080,
        x: 0x0FFF_FF40,
        y: 0x0FFF_FF40,
    },
    Caso {
        nombre: "mapa de 64 tiles",
        tiles: 64,
        wrap: true,
        pa: 0x080,
        pb: 0x000,
        pc: 0x000,
        pd: 0x080,
        x: 0x0FFF_FF40,
        y: 0x0FFF_FF40,
    },
    Caso {
        nombre: "mapa de 128 tiles",
        tiles: 128,
        wrap: true,
        pa: 0x080,
        pb: 0x000,
        pc: 0x000,
        pd: 0x080,
        x: 0x0FFF_FF40,
        y: 0x0FFF_FF40,
    },
];

/// Campo de tamaño del `BGxCNT`: 16, 32, 64 o 128 tiles.
fn campo_de_tamano(tiles: u32) -> u16 {
    match tiles {
        16 => 0,
        32 => 1,
        64 => 2,
        128 => 3,
        otro => panic!("tamaño de mapa afín no válido: {otro}"),
    }
}

/// Registros de I/O del caso: `DISPCNT` en modo 1 con el fondo afín BG2
/// activado, `BG2CNT` con las bases, el wrap y el tamaño, y la matriz afín.
fn io_del_caso(c: &Caso) -> Vec<u8> {
    let mut io = vec![0u8; 0x400];
    // DISPCNT: modo 1 (bits 0-2 = 1) y fondo afín BG2 (bit 10).
    io[0x00..0x02].copy_from_slice(&0x0401u16.to_le_bytes());
    // BG2CNT: prioridad 0, caracteres en 0x0000, mapa en 0x4000, wrap y tamaño.
    let cnt = ((BASE_MAPA / 0x800) as u16) << 8
        | if c.wrap { 0x2000 } else { 0 }
        | (campo_de_tamano(c.tiles) << 14);
    io[0x0C..0x0E].copy_from_slice(&cnt.to_le_bytes());
    // BG2PA, BG2PB, BG2PC y BG2PD.
    io[0x20..0x22].copy_from_slice(&c.pa.to_le_bytes());
    io[0x22..0x24].copy_from_slice(&c.pb.to_le_bytes());
    io[0x24..0x26].copy_from_slice(&c.pc.to_le_bytes());
    io[0x26..0x28].copy_from_slice(&c.pd.to_le_bytes());
    // BG2X y BG2Y.
    io[0x28..0x2C].copy_from_slice(&c.x.to_le_bytes());
    io[0x2C..0x30].copy_from_slice(&c.y.to_le_bytes());
    io
}

/// VRAM sintética: datos de caracteres y mapa.
fn vram_del_caso(c: &Caso) -> Vec<u8> {
    let mut vram = vec![0u8; VRAM];
    // Caracteres: 256 tiles de 8 bits por píxel. El índice de color nunca es 0
    // (el 0 es transparente), así que cualquier muestreo equivocado se ve.
    for tile in 0..256usize {
        for fila in 0..8usize {
            for col in 0..8usize {
                let valor = 1 + (tile * 5 + fila * 3 + col) % 255;
                vram[BASE_CHAR + tile * 64 + fila * 8 + col] = valor as u8;
            }
        }
    }
    // Mapa: una entrada de 1 byte por tile de 8×8, en filas de `tiles` columnas.
    let lado = c.tiles as usize;
    for i in 0..(lado * lado) {
        vram[BASE_MAPA + i] = ((i * 7 + 13) % 256) as u8;
    }
    vram
}

/// Paleta del fondo: 256 colores RGB555 en el banco 0.
fn paleta() -> Vec<u8> {
    let mut pal = vec![0u8; 0x400];
    for i in 0..256usize {
        let color = ((i * 5) & 31) as u16
            | ((((i * 11) & 31) as u16) << 5)
            | ((((i * 17) & 31) as u16) << 10);
        pal[i * 2..i * 2 + 2].copy_from_slice(&color.to_le_bytes());
    }
    pal
}

/// Cuadro de la referencia por software con el estado del caso.
fn referencia(io: &[u8], vram: &[u8], pal: &[u8]) -> Vec<u16> {
    let mut mem = gba_core::MemMap::new(Vec::new());
    mem.io = io.to_vec();
    mem.vram = vram.to_vec();
    mem.palette = pal.to_vec();
    for linea in 0..ALTO {
        gba_core::ppu::render_scanline(&mut mem, linea);
    }
    mem.framebuffer.clone()
}

fn rgb8(c: u16) -> [u8; 3] {
    [
        canal_8bit(c),
        canal_8bit(c >> 5),
        canal_8bit(c >> 10),
    ]
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() > 2 {
        eprintln!("uso: ppu_nativa_afin [ruta.gba]");
        std::process::exit(2);
    }
    // La ROM es opcional: el estado del arnés es sintético.
    if let Some(ruta) = args.get(1) {
        if let Err(e) = std::fs::read(ruta) {
            eprintln!("no se pudo leer la ROM {ruta}: {e} (no hace falta: el estado es sintético)");
        }
    }

    let pal = paleta();
    let oam = vec![0u8; 0x400];

    let mut gpu = match Gpu::nueva_nativa() {
        Ok(g) => g,
        Err(e) => {
            eprintln!("{e}");
            std::process::exit(1);
        }
    };

    let mut lineas = vec![
        "resumen arnés del fondo afín: PPU nativa (GPU) frente a gba_core::ppu::render_scanline"
            .to_string(),
    ];
    let mut total = 0usize;
    for caso in CASOS.iter() {
        let io = io_del_caso(caso);
        let vram = vram_del_caso(caso);
        let ref_pixeles = referencia(&io, &vram, &pal);
        let nativa = gpu.renderiza(&vram, &oam, &pal, &io);

        let mut dif = 0usize;
        for y in 0..ALTO {
            for x in 0..ANCHO {
                let p = y * ANCHO + x;
                let nat = [nativa[p * 3], nativa[p * 3 + 1], nativa[p * 3 + 2]];
                if !cerca_pixel(nat, rgb8(ref_pixeles[p])) {
                    dif += 1;
                }
            }
        }
        total += dif;

        // Control del caso del wrap: el píxel (0, 0) está fuera del mapa, así
        // que con el wrap tiene que estar dibujado (y no recortado) en las dos.
        let mut control = "";
        if caso.wrap {
            let ref_0 = [nativa[0], nativa[1], nativa[2]];
            let dibujado = ref_pixeles[0] != 0xFFFF && cerca_pixel(ref_0, rgb8(ref_pixeles[0]));
            control = if dibujado {
                "; pixel fuera del mapa dibujado: si"
            } else {
                "; pixel fuera del mapa dibujado: no"
            };
        }
        lineas.push(format!(
            "caso {}: {dif} diferencias de {}{control}",
            caso.nombre,
            ANCHO * ALTO
        ));
    }
    lineas.push(format!(
        "total: {total} diferencias de {}",
        ANCHO * ALTO * CASOS.len()
    ));

    for l in &lineas {
        println!("{l}");
    }

    let _ = std::fs::create_dir_all("volcados");
    let ruta = "volcados/ppu-nativa-afin.txt";
    if let Err(e) = std::fs::write(ruta, format!("{}\n", lineas.join("\n"))) {
        eprintln!("no se pudo escribir {ruta}: {e}");
        std::process::exit(1);
    }
    println!("informe escrito en {ruta}");
}
