//! Arnés de comparación de la PPU nativa frente a la PPU por software.
//!
//! Reproduce el cuadro N dos veces:
//!
//! - **Referencia**: la PPU de `gba-core` línea a línea, con el estado de cada
//!   línea (la que produce el emulador).
//! - **Nativa**: la GPU con [`blazing_recomp::ppu_nativa::PpuNativaPipeline`],
//!   con el estado de fin de cuadro.
//!
//! Control: la reconstrucción por software tiene que coincidir con el
//! framebuffer del emulador (0 diferencias). Si no, el informe no vale y el
//! programa sale con código 1 sin escribir nada.
//!
//! Desglose: diferencias totales, por línea, y separadas entre líneas con
//! sprites afines y líneas sin ellos.
//!
//! Hay **dos referencias** y las dos se miden:
//!
//! - **Estado de fin de cuadro**: la comparación que judgea el criterio de
//!   aceptación de la fase 1. La GPU toma VRAM, OAM, paleta e I/O una sola
//!   vez por cuadro, así que la única comparación justa es contra la PPU
//!   alimentada con ese mismo estado.
//! - **Estado por línea**: la PPU que produce el emulador, con el estado que
//!   hay en cada línea. Las diferencias que aparecen aquí son del estado que
//!   cambia *dentro* del cuadro (el VRAM cambia en 146 de las 160 líneas del
//!   cuadro 5300), no de la lógica de dibujo. Quedan fuera del alcance de esta
//!   fase y son el objeto de la fase 1b.
//!
//! Escribe `volcados/ppu-nativa-<N>.txt`. Si la nativa difiere de alguna de las dos
//! referencias, escribe además `volcados/diferencias/cuadro-<N>-<fin|linea>.png` (una imagen
//! compuesta por caso) y cierra el informe con el diagnóstico causal.
//!
//! Uso:
//!   cargo run --release --features proto --example ppu_nativa_compara -- <ruta.gba> <secuencia.txt> <cuadro>

#[path = "comun/mod.rs"]
mod comun;

use comun::color::{canal_8bit, cerca_pixel, software_a_rgb};
use comun::diagnostico::{self, EstadoRef};
use comun::entrada::leer_secuencia;
use comun::gpu::{ALTO, ANCHO, Gpu};

/// Dibuja las 160 líneas con la PPU por software y un mismo estado en todas.
fn render_estado(rom: &[u8], vram: &[u8], oam: &[u8], pal: &[u8], io: &[u8]) -> Vec<u16> {
    let mut replay = gba_core::Machine::new(rom.to_vec());
    replay.bus.palette = pal.to_vec();
    replay.bus.vram = vram.to_vec();
    replay.bus.oam = oam.to_vec();
    replay.bus.io = io.to_vec();
    let mut img = vec![0u16; ANCHO * ALTO];
    for l in 0..ALTO {
        gba_core::ppu::render_scanline(&mut replay.bus, l);
        let ini = l * ANCHO;
        img[ini..ini + ANCHO].copy_from_slice(&replay.bus.framebuffer[ini..ini + ANCHO]);
    }
    img
}

fn rgb8(c: u16) -> [u8; 3] {
    [
        canal_8bit(c),
        canal_8bit(c >> 5),
        canal_8bit(c >> 10),
    ]
}

const OBJ_TAMANOS: [[(usize, usize); 4]; 3] = [
    [(8, 8), (16, 16), (32, 32), (64, 64)],
    [(16, 8), (32, 8), (32, 16), (64, 32)],
    [(8, 16), (8, 32), (16, 32), (32, 64)],
];

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

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 4 {
        eprintln!("uso: ppu_nativa_compara <ruta.gba> <secuencia.txt> <cuadro>");
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

    // Referencia: la PPU por software, línea a línea con el estado de cada línea.
    let mut ppu = vec![0u16; n];
    let mut replay = gba_core::Machine::new(rom.clone());
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

    // Segunda referencia: la misma PPU por software, pero con el estado de fin
    // de cuadro en todas las líneas. La diferencia entre esta referencia y la
    // anterior es exactamente el estado que cambia dentro del cuadro (VRAM,
    // scrolls, registros), que la GPU no puede reproducir sin capturar el
    // estado por línea. Comparar la nativa contra las dos separa "la lógica
    // de dibujo está mal" de "el estado por línea difiere".
    let mut ppu_fin = vec![0u16; n];
    {
        let mut replay_fin = gba_core::Machine::new(rom.clone());
        replay_fin.bus.palette = pal_fin.clone();
        replay_fin.bus.vram = vram_fin.clone();
        replay_fin.bus.oam = oam_fin.clone();
        replay_fin.bus.io = io_fin.clone();
        for l in 0..ALTO {
            gba_core::ppu::render_scanline(&mut replay_fin.bus, l);
            let ini = l * ANCHO;
            ppu_fin[ini..ini + ANCHO]
                .copy_from_slice(&replay_fin.bus.framebuffer[ini..ini + ANCHO]);
        }
    }

    // Control: la reconstrucción debe coincidir con el framebuffer del emulador.
    let control_dif = (0..n).filter(|&p| ppu[p] != framebuffer_emu[p]).count();
    if control_dif != 0 {
        eprintln!(
            "control falla: {control_dif} diferencias entre la reconstrucción y el emulador; \
             el informe no es válido"
        );
        std::process::exit(1);
    }

    // Cuadro nativo: la GPU con el estado de fin de cuadro.
    let mut gpu = match Gpu::nueva_nativa() {
        Ok(g) => g,
        Err(e) => {
            eprintln!("{e}");
            std::process::exit(1);
        }
    };
    let nativa = gpu.renderiza(&vram_fin, &oam_fin, &pal_fin, &io_fin);

    // Comparación píxel a píxel.
    let mut dif_total = 0usize;
    let mut dif_con_afin = 0usize;
    let mut dif_sin_afin = 0usize;
    let mut lineas_con_afin = 0usize;
    let mut lineas_sin_afin = 0usize;
    let mut lineas_limpias = 0usize;
    // Diferencias contra la referencia de estado de fin de cuadro.
    let mut dif_total_fin = 0usize;
    let mut dif_fin_sin_afin = 0usize;
    // Diferencias entre las dos referencias, sin la nativa de por medio: aísla
    // cuánto del desajuste es del estado que cambia dentro del cuadro.
    let mut dif_referencias = 0usize;
    let mut detalle = Vec::with_capacity(ALTO);
    for cap in capturas.iter() {
        let l = cap.linea;
        let con_afin = linea_con_afin(&cap.oam, l);
        let mut dif = 0usize;
        let mut primera_x = None;
        let mut dif_fin = 0usize;
        for x in 0..ANCHO {
            let p = l * ANCHO + x;
            let nat = [nativa[p * 3], nativa[p * 3 + 1], nativa[p * 3 + 2]];
            if !cerca_pixel(nat, rgb8(ppu[p])) {
                dif += 1;
                if primera_x.is_none() {
                    primera_x = Some(x);
                }
            }
            if !cerca_pixel(nat, rgb8(ppu_fin[p])) {
                dif_fin += 1;
            }
            if !cerca_pixel(rgb8(ppu[p]), rgb8(ppu_fin[p])) {
                dif_referencias += 1;
            }
        }
        dif_total += dif;
        dif_total_fin += dif_fin;
        if con_afin {
            lineas_con_afin += 1;
            dif_con_afin += dif;
        } else {
            lineas_sin_afin += 1;
            dif_sin_afin += dif;
            dif_fin_sin_afin += dif_fin;
        }
        if dif == 0 {
            lineas_limpias += 1;
        }
        detalle.push(format!(
            "linea {l:3}: {:?} diferencias por linea {dif:3} (estado fin de cuadro: {dif_fin:3}){}",
            if con_afin { "con afili " } else { "sin afili " },
            primera_x
                .map(|x| format!("(primera en x={x})"))
                .unwrap_or_default()
        ));
    }

    let mut lineas = vec![
        format!("resumen cuadro {objetivo}: PPU nativa (GPU) frente a PPU por software"),
        format!("control reconstruccion frente a framebuffer del emulador: {control_dif} diferencias (esperado 0)"),
        format!("diferencias frente a la referencia de estado de fin de cuadro: {dif_total_fin} de {n}"),
        format!("criterio de la fase 1 (0 diferencias en las lineas sin afines): {}", if dif_fin_sin_afin == 0 && dif_total_fin == 0 { "CUMPLIDO" } else { "NO CUMPLIDO" }),
        format!("lineas con afines: {lineas_con_afin}; lineas sin afines: {lineas_sin_afin}"),
        format!("referencia de estado por linea (fuera del alcance de esta fase): {dif_total} diferencias (con afines: {dif_con_afin}; sin afines: {dif_sin_afin})"),
        format!("diferencias entre las dos referencias (solo efecto del estado dentro del cuadro): {dif_referencias}"),
        format!("lineas limpias frente a la referencia por linea: {lineas_limpias} de {ALTO}"),
        "detalle por linea:".to_string(),
    ];
    lineas.extend(detalle);

    // Captura visual bajo demanda: una imagen compuesta por caso, sólo si difiere.
    let sw_fin_rgb = software_a_rgb(&ppu_fin);
    let sw_linea_rgb = software_a_rgb(&ppu);
    let dir_capturas = std::path::Path::new("volcados/diferencias");
    let mut escritas = Vec::new();
    for (caso, sw) in [("fin", &sw_fin_rgb), ("linea", &sw_linea_rgb)] {
        match diagnostico::guardar_captura_si_difiere(dir_capturas, objetivo, caso, sw, &nativa) {
            Ok(Some(r)) => escritas.push(format!(
                "  {caso}: {} ({} px, caja x{}-{} y{}-{})",
                r.ruta.display(),
                r.distintos,
                r.caja.0,
                r.caja.2,
                r.caja.1,
                r.caja.3
            )),
            Ok(None) => {}
            Err(e) => {
                eprintln!("no se pudo escribir la captura {caso}: {e}");
                std::process::exit(1);
            }
        }
    }
    if escritas.is_empty() {
        lineas.push("capturas generadas: ninguna (0 diferencias)".to_string());
    } else {
        lineas.push("capturas generadas:".to_string());
        lineas.extend(escritas);
    }

    // Diagnóstico causal: estado a mitad de cuadro o fallo de dibujo (con ablación).
    let fin_estado = EstadoRef { io: &io_fin, palette: &pal_fin, vram: &vram_fin, oam: &oam_fin };
    let lineas_estado: Vec<EstadoRef> = capturas
        .iter()
        .map(|c| EstadoRef { io: &c.io, palette: &c.palette, vram: &c.vram, oam: &c.oam })
        .collect();
    let diag = diagnostico::diagnosticar(
        &nativa,
        &sw_fin_rgb,
        &sw_linea_rgb,
        fin_estado,
        &lineas_estado,
        |_, io_v| {
            let nat_v = gpu.renderiza(&vram_fin, &oam_fin, &pal_fin, io_v);
            let sw_v = software_a_rgb(&render_estado(&rom, &vram_fin, &oam_fin, &pal_fin, io_v));
            diagnostico::cuenta_distintos(&nat_v, &sw_v)
        },
    );
    lineas.push("diagnostico causal:".to_string());
    lineas.extend(diag.informe().into_iter().map(|l| format!("  {l}")));
    for l in &lineas {
        println!("{l}");
    }

    let _ = std::fs::create_dir_all("volcados");
    let ruta = format!("volcados/ppu-nativa-{objetivo}.txt");
    if let Err(e) = std::fs::write(&ruta, format!("{}\n", lineas.join("\n"))) {
        eprintln!("no se pudo escribir {ruta}: {e}");
        std::process::exit(1);
    }
    println!("informe escrito en {ruta}");
}
