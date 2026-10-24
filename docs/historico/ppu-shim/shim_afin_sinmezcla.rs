//! Experimento de docs/propuestas/2026-10-18_sprites-afines-cuadro-7500.md.
//!
//! Cuadro N, shim completo frente a la PPU por software (V0, DISPCNT completo), sin usar sprites().
//! Para cada línea cuenta: sprites afines en OAM (attr0 & 0x100) y píxeles que difieren
//! (tolerancia ±1 por canal). Control: la reconstrucción por software debe coincidir con el
//! framebuffer del emulador en ese cuadro (0 diferencias); si no, el informe no es válido.
//!
//! Escribe volcados/afin-<N>.txt.
//!
//! Uso:
//!   cargo run --release --features proto --example shim_afin_mide -- <ruta.gba> <secuencia.txt> <cuadro>

#[path = "comun/mod.rs"]
mod comun;

use comun::color::{canal_8bit, cerca_pixel};
use comun::entrada::leer_secuencia;
use comun::gpu::{ALTO, ANCHO, Gpu};

fn rgb8(c: u16) -> [u8; 3] {
    [canal_8bit(c), canal_8bit(c >> 5), canal_8bit(c >> 10)]
}

const OBJ_TAMANOS: [[(usize, usize); 4]; 3] = [
    [(8, 8), (16, 16), (32, 32), (64, 64)],
    [(16, 8), (32, 8), (32, 16), (64, 32)],
    [(8, 16), (8, 32), (16, 32), (32, 64)],
];

/// Sprite afín de OAM. Caja de dibujo: con el bit de doble tamaño (attr0 bit 9), el doble.
struct Afin {
    i: usize,
    x: i64,
    y: i64,
    w: usize,
    h: usize,
    grupo: usize,
    doble: bool,
}

impl Afin {
    fn cubre(&self, linea: usize) -> bool {
        let l = linea as i64;
        l >= self.y && l < self.y + self.h as i64
    }
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

/// Parámetro de matriz del grupo: pa en 0x06 + 0x20·grupo, pb +0x08, pc +0x10, pd +0x18.
fn parametro(oam: &[u8], grupo: usize, desplaz: usize) -> i16 {
    let o = 0x06 + 0x20 * grupo + desplaz;
    i16::from_le_bytes([oam[o], oam[o + 1]])
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 4 {
        eprintln!("uso: shim_afin_mide <ruta.gba> <secuencia.txt> <cuadro>");
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
                m.bus.capturar_linea = true; m.bus.io[0x50] = 0; m.bus.io[0x51] = 0;
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

    // PPU por software (V0, DISPCNT completo), línea a línea con el estado de cada línea.
    let mut ppu = vec![0u16; n];
    let mut replay = gba_core::Machine::new(rom);
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

    // Control: la reconstrucción debe coincidir con el framebuffer del emulador.
    let control_dif = (0..n).filter(|&p| ppu[p] != framebuffer_emu[p]).count();

    // Shim completo con el estado de fin.
    let mut gpu = match Gpu::nueva() {
        Ok(g) => g,
        Err(e) => {
            eprintln!("{e}");
            std::process::exit(1);
        }
    };
    let shim = gpu.renderiza(&vram_fin, &oam_fin, &pal_fin, &io_fin);

    // Por línea: afines en OAM y píxeles distintos entre shim y PPU.
    let mut dif_total = 0usize;
    let mut dif_con_afin = 0usize;
    let mut dif_sin_afin = 0usize;
    let mut lineas_con_afin = 0usize;
    let mut lineas_sin_afin = 0usize;
    let mut detalle = Vec::with_capacity(ALTO);
    for cap in capturas.iter() {
        let l = cap.linea;
        let afines = afines_de(&cap.oam).iter().filter(|a| a.cubre(l)).count();
        let mut dif = 0usize;
        for x in 0..ANCHO {
            let p = l * ANCHO + x;
            let sh = [shim[p * 3], shim[p * 3 + 1], shim[p * 3 + 2]];
            if !cerca_pixel(sh, rgb8(ppu[p])) {
                dif += 1;
            }
        }
        dif_total += dif;
        if afines > 0 {
            lineas_con_afin += 1;
            dif_con_afin += dif;
        } else {
            lineas_sin_afin += 1;
            dif_sin_afin += dif;
        }
        detalle.push(format!("linea {l:3}: afines {afines:3} diferencias {dif:3}"));
    }

    let mut lineas = vec![
        format!("resumen cuadro {objetivo}: shim frente a PPU V0 con afines"),
        format!("control reconstruccion frente a framebuffer del emulador: {control_dif} diferencias (esperado 0)"),
        format!("diferencias totales: {dif_total}"),
        format!("lineas con afines: {lineas_con_afin}; diferencias en ellas: {dif_con_afin}"),
        format!("lineas sin afines: {lineas_sin_afin}; diferencias en ellas: {dif_sin_afin}"),
        "detalle por linea:".to_string(),
    ];
    lineas.extend(detalle);
    lineas.push("afines del cuadro (pa pb pc pd de su grupo en OAM):".to_string());
    for a in afines_de(&oam_fin) {
        let pa = parametro(&oam_fin, a.grupo, 0);
        let pb = parametro(&oam_fin, a.grupo, 8);
        let pc = parametro(&oam_fin, a.grupo, 16);
        let pd = parametro(&oam_fin, a.grupo, 24);
        let vis = (0..ALTO).filter(|&l| a.cubre(l)).count();
        lineas.push(format!(
            "afin {:3}: x {:4} y {:4} tam {}x{} doble {} grupo {:2} pa {:04x} pb {:04x} pc {:04x} pd {:04x} lineas visibles {}",
            a.i, a.x, a.y, a.w, a.h, a.doble, a.grupo, pa, pb, pc, pd, vis
        ));
    }
    for l in &lineas {
        println!("{l}");
    }
    if control_dif != 0 {
        eprintln!("control falla: {control_dif} diferencias entre reconstrucción y emulador; no se escribe el informe");
        std::process::exit(1);
    }
    let _ = std::fs::create_dir_all("volcados");
    let ruta = format!("volcados/afin-sinmezcla-{objetivo}.txt");
    if let Err(e) = std::fs::write(&ruta, format!("{}\n", lineas.join("\n"))) {
        eprintln!("no se pudo escribir {ruta}: {e}");
        std::process::exit(1);
    }
}
