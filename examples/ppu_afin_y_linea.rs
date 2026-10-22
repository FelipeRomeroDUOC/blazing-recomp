//! Experimento de docs/propuestas/2026-10-17_diagnostico-afin-y-por-linea.md.
//!
//! Barre un rango de cuadros y, para cada uno, captura el estado por línea que usa la PPU por
//! software y compara las 160 capturas entre sí. No dibuja nada: mide qué necesita el shim y
//! todavía no puede representar.
//!
//! Por cuadro informa:
//!   - `modo`: modo de vídeo de `DISPCNT`;
//!   - `afines`: capas afines (bit 6 de `BGnCNT` en BG2/BG3 en modo 0; fijas por modo en modo 1/2);
//!   - `cambios`: campos de E/S que cambian dentro del cuadro, con el número de líneas afectadas;
//!   - `lineas_paleta`, `lineas_vram`, `lineas_oam`: líneas cuya copia difiere de la línea 0.
//!
//! El control redibuja el último cuadro del rango fuera del juego con la PPU por software y lo
//! compara con el framebuffer del juego: debe dar 0 píxeles.
//!
//! Escribe volcados/afin-y-linea.txt.
//!
//! Uso:
//!   cargo run --release --features proto --example ppu_afin_y_linea -- <ruta.gba> <secuencia.txt> <desde> <hasta> [paso]
/// Registros de E/S que se vigilan dentro del cuadro, con su offset.

#[path = "comun/mod.rs"]
mod comun;

use comun::entrada::leer_secuencia;
use comun::gpu::{ALTO, ANCHO};
use comun::registro::io16;

const CAMPOS: [(&str, usize); 16] = [
    ("DISPCNT", 0x00),
    ("BG0CNT", 0x08),
    ("BG1CNT", 0x0A),
    ("BG2CNT", 0x0C),
    ("BG3CNT", 0x0E),
    ("BG0HOFS", 0x10),
    ("BG1HOFS", 0x14),
    ("BG2HOFS", 0x18),
    ("BG3HOFS", 0x1C),
    ("BG0VOFS", 0x12),
    ("BG1VOFS", 0x16),
    ("BG2VOFS", 0x1A),
    ("BG3VOFS", 0x1E),
    ("BLDCNT", 0x50),
    ("BLDALPHA", 0x52),
    ("BLDY", 0x54),
];

/// Expande la secuencia a una tecla por cuadro.
fn teclas_por_cuadro(tramos: &[(u32, u16)]) -> Vec<u16> {
    let mut teclas = Vec::new();
    for &(cuadros, presionados) in tramos {
        for _ in 0..cuadros {
            teclas.push(0x03FF & !presionados);
        }
    }
    teclas
}

/// Capas afines del cuadro: por modo de vídeo y por bit 6 de `BGnCNT`.
fn afines(io: &[u8]) -> String {
    let dispcnt = io16(io, 0x00);
    let modo = dispcnt & 7;
    let mut lista: Vec<&str> = Vec::new();
    match modo {
        0 => {
            // En modo 0 el bit 6 de BG2 y BG3 es el de afín; en BG0 y BG1 es mosaico.
            if dispcnt & 0x0400 != 0 && io16(io, 0x0C) & 0x40 != 0 {
                lista.push("BG2");
            }
            if dispcnt & 0x0800 != 0 && io16(io, 0x0E) & 0x40 != 0 {
                lista.push("BG3");
            }
        }
        1 => lista.push("BG2"),
        2 => lista.extend(["BG2", "BG3"]),
        _ => {}
    }
    if lista.is_empty() {
        "-".to_string()
    } else {
        lista.join(",")
    }
}

fn distintos(a: &[u16], b: &[u16]) -> usize {
    a.iter().zip(b.iter()).filter(|(x, y)| x != y).count()
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 5 {
        eprintln!("uso: ppu_afin_y_linea <ruta.gba> <secuencia.txt> <desde> <hasta> [paso]");
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
    let desde: u32 = args[3].parse().unwrap_or_else(|_| {
        eprintln!("desde debe ser un entero");
        std::process::exit(2);
    });
    let hasta: u32 = args[4].parse().unwrap_or_else(|_| {
        eprintln!("hasta debe ser un entero");
        std::process::exit(2);
    });
    let paso: u32 = match args.get(5) {
        Some(s) => s.parse().unwrap_or(1),
        None => 1,
    }
    .max(1);
    if hasta < desde {
        eprintln!("el rango está al revés");
        std::process::exit(2);
    }
    let tramos = match leer_secuencia(&texto) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("secuencia inválida: {e}");
            std::process::exit(1);
        }
    };
    let teclas = teclas_por_cuadro(&tramos);

    let mut m = gba_core::Machine::new(rom.clone());
    let mut cuadro: u32 = 0;
    let mut filas: Vec<String> = Vec::new();
    let mut lista_afines: Vec<u32> = Vec::new();
    let mut lista_cambios: Vec<u32> = Vec::new();
    let mut cuenta_campo: std::collections::BTreeMap<String, u32> = Default::default();
    let mut ultimas_capturas = None;
    let mut ultimo_fb: Vec<u16> = Vec::new();

    let mut objetivo = desde;
    while objetivo <= hasta {
        // Avanzar hasta el cuadro pedido.
        while cuadro < objetivo {
            m.bus.keys = teclas.get(cuadro as usize).copied().unwrap_or(0x03FF);
            m.run_frame(5_000_000);
            cuadro += 1;
        }
        m.bus.capturas_linea.clear();
        m.bus.capturar_linea = true;
        m.bus.keys = teclas.get(cuadro as usize).copied().unwrap_or(0x03FF);
        m.run_frame(5_000_000);
        m.bus.capturar_linea = false;
        cuadro += 1;

        if m.bus.capturas_linea.len() != ALTO {
            eprintln!(
                "cuadro {objetivo}: se esperaban {ALTO} capturas y hay {}",
                m.bus.capturas_linea.len()
            );
        } else {
            let capturas = &m.bus.capturas_linea;
            let base = &capturas[0];

            let mut cambios: Vec<String> = Vec::new();
            for (nombre, off) in CAMPOS.iter() {
                let valor = io16(&base.io, *off);
                let n = capturas
                    .iter()
                    .filter(|c| io16(&c.io, *off) != valor)
                    .count();
                if n > 0 {
                    cambios.push(format!("{nombre}:{n}"));
                    *cuenta_campo.entry((*nombre).to_string()).or_insert(0) += 1;
                }
            }
            let lineas_paleta = capturas
                .iter()
                .filter(|c| c.palette != base.palette)
                .count();
            let lineas_vram = capturas.iter().filter(|c| c.vram != base.vram).count();
            let lineas_oam = capturas.iter().filter(|c| c.oam != base.oam).count();
            if lineas_vram > 0 {
                *cuenta_campo.entry("VRAM".to_string()).or_insert(0) += 1;
            }
            if lineas_oam > 0 {
                *cuenta_campo.entry("OAM".to_string()).or_insert(0) += 1;
            }
            if lineas_paleta > 0 {
                *cuenta_campo.entry("PALETA".to_string()).or_insert(0) += 1;
            }

            let modo = io16(&base.io, 0x00) & 7;
            let afin = afines(&base.io);
            let cambios_txt = if cambios.is_empty() {
                "-".to_string()
            } else {
                cambios.join(",")
            };
            if afin != "-" {
                lista_afines.push(objetivo);
            }
            if cambios_txt != "-" || lineas_vram > 0 || lineas_oam > 0 || lineas_paleta > 0 {
                lista_cambios.push(objetivo);
            }
            filas.push(format!(
                "cuadro {objetivo} modo {modo} afines {afin} cambios {cambios_txt} lineas_paleta {lineas_paleta} lineas_vram {lineas_vram} lineas_oam {lineas_oam}"
            ));

            ultimas_capturas = Some(capturas.clone());
            ultimo_fb = m.bus.framebuffer.clone();
        }
        objetivo += paso;
    }

    // Control: redibujar el último cuadro fuera del juego y compararlo con su framebuffer.
    let mut control = 0usize;
    if let Some(capturas) = &ultimas_capturas {
        let mut replay = gba_core::Machine::new(rom);
        let mut fb = vec![0u16; ANCHO * ALTO];
        for cap in capturas.iter() {
            replay.bus.palette = cap.palette.clone();
            replay.bus.vram = cap.vram.clone();
            replay.bus.oam = cap.oam.clone();
            replay.bus.io = cap.io.clone();
            replay.bus.aff_ref = cap.aff_ref;
            replay.bus.aff_dirty = cap.aff_dirty;
            gba_core::ppu::render_scanline(&mut replay.bus, cap.linea);
            let ini = cap.linea * ANCHO;
            fb[ini..ini + ANCHO].copy_from_slice(&replay.bus.framebuffer[ini..ini + ANCHO]);
        }
        control = distintos(&fb, &ultimo_fb);
    }

    let lista = |v: &[u32]| -> String {
        if v.is_empty() {
            "-".to_string()
        } else {
            v.iter()
                .map(|n| n.to_string())
                .collect::<Vec<_>>()
                .join(",")
        }
    };
    let campo_mas = cuenta_campo
        .iter()
        .max_by_key(|(_, n)| **n)
        .map(|(nombre, n)| format!("{nombre} en {n} cuadros"))
        .unwrap_or_else(|| "-".to_string());
    let resumen = format!(
        "resumen cuadro {} {} paso {paso}: cuadros {}\n\
         afines {}\n\
         con cambios por linea {}\n\
         campo mas frecuente {campo_mas}\n\
         control {control} pixeles distintos de {} (esperado 0)",
        desde,
        hasta,
        filas.len(),
        lista(&lista_afines),
        lista(&lista_cambios),
        ANCHO * ALTO,
    );
    println!("{resumen}");
    for f in &filas {
        println!("{f}");
    }

    let _ = std::fs::create_dir_all("volcados");
    let contenido = format!("{resumen}\n{}\n", filas.join("\n"));
    if let Err(e) = std::fs::write("volcados/afin-y-linea.txt", contenido) {
        eprintln!("no se pudo escribir volcados/afin-y-linea.txt: {e}");
    }
}
