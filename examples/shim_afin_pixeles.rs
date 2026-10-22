//! Experimento de docs/propuestas/2026-10-18_pixeles-afines-cuadro-7500.md.
//!
//! Cuadro N. Para cada sprite afín de OAM se compara la PPU de referencia y el shim con una OAM
//! en la que solo ese afín queda activo (los demás afines pasan a forma prohibida, attr0 bits 14-15 = 3).
//! Los píxeles que aporta un afín son los que cambian frente a la línea base (todos los afines apagados).
//! Control: la PPU con la OAM completa debe coincidir con el framebuffer del emulador (0 diferencias).
//!
//! Escribe volcados/afin-pixeles-<N>.txt.
//!
//! Uso:
//!   cargo run --release --features proto --example shim_afin_pixeles -- <ruta.gba> <secuencia.txt> <cuadro>

#[path = "comun/mod.rs"]
mod comun;

use comun::color::{canal_8bit, cerca_pixel};
use comun::entrada::leer_secuencia;
use comun::gpu::{ALTO, ANCHO, Gpu};

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

fn afines_de(oam: &[u8]) -> Vec<Afin> {
    let mut out = Vec::new();
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

/// Copia de la OAM con los sprites indicados en forma prohibida (no se dibujan ni en la PPU ni en el shim).
fn oam_con(oam: &[u8], apagados: &[usize]) -> Vec<u8> {
    let mut out = oam.to_vec();
    for &i in apagados {
        let a0 = u16::from_le_bytes([out[i * 8], out[i * 8 + 1]]) | 0xC000;
        let b = a0.to_le_bytes();
        out[i * 8] = b[0];
        out[i * 8 + 1] = b[1];
    }
    out
}

fn rgb8(c: u16) -> [u8; 3] {
    [canal_8bit(c), canal_8bit(c >> 5), canal_8bit(c >> 10)]
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 4 {
        eprintln!("uso: shim_afin_pixeles <ruta.gba> <secuencia.txt> <cuadro>");
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
    let pal_fin = m.bus.palette.clone();
    let io_fin = m.bus.io.clone();
    let n = ANCHO * ALTO;

    // Variantes de OAM: 0 completa; 1 base (todos los afines apagados); 2.. solo el afín k.
    let afines = afines_de(&oam_fin);
    let todos: Vec<usize> = afines.iter().map(|a| a.i).collect();
    let mut variantes: Vec<Vec<u8>> = vec![oam_fin.clone(), oam_con(&oam_fin, &todos)];
    for a in &afines {
        let otros: Vec<usize> = afines.iter().filter(|b| b.i != a.i).map(|b| b.i).collect();
        variantes.push(oam_con(&oam_fin, &otros));
    }

    // PPU por software para cada variante, línea a línea.
    let mut replay = gba_core::Machine::new(rom);
    let mut ppu_v: Vec<Vec<u16>> = Vec::new();
    for oam_v in &variantes {
        let mut ppu = vec![0u16; n];
        for cap in capturas.iter() {
            replay.bus.palette = cap.palette.clone();
            replay.bus.vram = cap.vram.clone();
            replay.bus.oam = oam_v.clone();
            replay.bus.aff_ref = cap.aff_ref;
            replay.bus.aff_dirty = cap.aff_dirty;
            replay.bus.io = cap.io.clone();
            gba_core::ppu::render_scanline(&mut replay.bus, cap.linea);
            let ini = cap.linea * ANCHO;
            ppu[ini..ini + ANCHO].copy_from_slice(&replay.bus.framebuffer[ini..ini + ANCHO]);
        }
        ppu_v.push(ppu);
    }

    // Shim para cada variante, con el estado de fin.
    let mut gpu = match Gpu::nueva() {
        Ok(g) => g,
        Err(e) => {
            eprintln!("{e}");
            std::process::exit(1);
        }
    };
    let shim_v: Vec<Vec<u8>> = variantes
        .iter()
        .map(|oam_v| gpu.renderiza(&vram_fin, oam_v, &pal_fin, &io_fin))
        .collect();

    // Control: la PPU con la OAM completa debe coincidir con el emulador.
    let control_dif = (0..n).filter(|&p| ppu_v[0][p] != framebuffer_emu[p]).count();
    let base_dif_emu = (0..n).filter(|&p| ppu_v[1][p] != framebuffer_emu[p]).count();
    if control_dif != 0 {
        eprintln!("control falla: {control_dif} diferencias entre PPU completa y emulador; no se escribe el informe");
        std::process::exit(1);
    }

    let mut lineas = vec![
        format!("resumen cuadro {objetivo}: píxeles que aporta cada afín (PPU y shim)"),
        format!("control PPU completa frente a framebuffer del emulador: {control_dif} diferencias (esperado 0)"),
        format!("informativo: línea base (afines apagados) frente a emulador: {base_dif_emu} diferencias"),
        format!("afines en OAM: {}", afines.len()),
    ];
    for (k, a) in afines.iter().enumerate() {
        let v = 2 + k;
        let mut aporta_ppu = 0usize;
        let mut aporta_shim = 0usize;
        let mut dif_en_aporte = 0usize;
        for p in 0..n {
            let en_ppu = ppu_v[v][p] != ppu_v[1][p];
            let sh = [shim_v[v][p * 3], shim_v[v][p * 3 + 1], shim_v[v][p * 3 + 2]];
            let sh_base = [shim_v[1][p * 3], shim_v[1][p * 3 + 1], shim_v[1][p * 3 + 2]];
            let en_shim = !cerca_pixel(sh, sh_base);
            if en_ppu {
                aporta_ppu += 1;
            }
            if en_shim {
                aporta_shim += 1;
            }
            if (en_ppu || en_shim) && !cerca_pixel(rgb8(ppu_v[v][p]), sh) {
                dif_en_aporte += 1;
            }
        }
        lineas.push(format!(
            "afin {:3}: caja x {:4} y {:4} tam {}x{} doble {} grupo {:2}; PPU aporta {aporta_ppu} píxeles; shim aporta {aporta_shim}; diferencias PPU-shim en esos píxeles {dif_en_aporte}",
            a.i, a.x, a.y, a.w, a.h, a.doble, a.grupo
        ));
    }
    for l in &lineas {
        println!("{l}");
    }
    let _ = std::fs::create_dir_all("volcados");
    let ruta = format!("volcados/afin-pixeles-{objetivo}.txt");
    if let Err(e) = std::fs::write(&ruta, format!("{}\n", lineas.join("\n"))) {
        eprintln!("no se pudo escribir {ruta}: {e}");
        std::process::exit(1);
    }
}
