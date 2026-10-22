//! Escaneo de solo lectura para docs/propuestas/2026-10-18_decode-oam-otro-cuadro-regresion.md.
//!
//! Para cada cuadro de la secuencia registra: DISPCNT, bit OBJ, sprites visibles, sprites afines,
//! sprites de ventana OBJ y una firma de escena (registros de fondo y primeros colores de paleta).
//! Lista los cuadros con OBJ apagado, sprites visibles, sin afines ni ventana OBJ, y cuya firma
//! de escena es distinta de la del cuadro de referencia (argumento opcional, 1000 por defecto).
//!
//! Escribe volcados/escaneo-obj-<etiqueta>.txt.
//!
//! Uso:
//!   cargo run --release --features proto --example shim_escanear_obj -- <ruta.gba> <secuencia.txt> <etiqueta> [cuadro_ref]

#[path = "comun/mod.rs"]
mod comun;

use comun::entrada::leer_secuencia;

fn fnv(datos: &[u8]) -> u64 {
    let mut h: u64 = 0xcbf29ce484222325;
    for &b in datos {
        h ^= b as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    h
}

#[derive(Clone, Copy)]
struct Estado {
    dispcnt: u16,
    visibles: usize,
    afines: usize,
    ventana_obj: usize,
    firma: u64,
}

fn estado(io: &[u8], oam: &[u8], pal: &[u8]) -> Estado {
    let dispcnt = u16::from_le_bytes([io[0], io[1]]);
    let (mut visibles, mut afines, mut ventana_obj) = (0usize, 0usize, 0usize);
    for i in 0..128 {
        let a0 = u16::from_le_bytes([oam[i * 8], oam[i * 8 + 1]]);
        let forma = (a0 >> 14) & 3;
        if a0 & 0x100 != 0 {
            afines += 1;
            continue;
        }
        if a0 & 0x200 != 0 || forma == 3 {
            continue;
        }
        if (a0 >> 10) & 3 == 2 {
            ventana_obj += 1;
        } else {
            visibles += 1;
        }
    }
    let mut datos = io[0x00..0x20].to_vec();
    datos.extend_from_slice(&pal[0..64.min(pal.len())]);
    Estado {
        dispcnt,
        visibles,
        afines,
        ventana_obj,
        firma: fnv(&datos),
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 4 {
        eprintln!("uso: shim_escanear_obj <ruta.gba> <secuencia.txt> <etiqueta> [cuadro_ref]");
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
    let etiqueta = args[3].clone();
    let referencia: u32 = args.get(4).and_then(|s| s.parse().ok()).unwrap_or(1000);
    let tramos = match leer_secuencia(&texto) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("secuencia inválida: {e}");
            std::process::exit(1);
        }
    };

    let mut m = gba_core::Machine::new(rom);
    let mut registro: Vec<Estado> = Vec::new();
    for (cuadros, presionados) in tramos {
        m.bus.keys = 0x03FF & !presionados;
        for _ in 0..cuadros {
            m.run_frame(5_000_000);
            registro.push(estado(&m.bus.io, &m.bus.oam, &m.bus.palette));
        }
    }
    let Some(ref_estado) = registro.get(referencia as usize).copied() else {
        eprintln!("la secuencia no llega al cuadro de referencia {referencia}");
        std::process::exit(1);
    };

    let mut lineas = vec![format!(
        "resumen escaneo {etiqueta}: {} cuadros, referencia {referencia} (firma {:016x})",
        registro.len(),
        ref_estado.firma
    )];
    let mut candidatos = Vec::new();
    for (k, e) in registro.iter().enumerate() {
        let obj_apagado = e.dispcnt & 0x1000 == 0;
        let otra_escena = e.firma != ref_estado.firma;
        let bg0_activa = e.dispcnt & 0x0100 != 0;
        let sin_blanco = e.dispcnt & 0x0080 == 0;
        if obj_apagado && bg0_activa && sin_blanco && e.visibles > 0 && e.afines == 0 && e.ventana_obj == 0 && otra_escena {
            candidatos.push(k);
        }
    }
    lineas.push(format!("cuadros candidatos (OBJ apagado, sprites, sin afines ni ventana, otra escena): {}", candidatos.len()));
    for k in candidatos.iter().take(60) {
        let e = registro[*k];
        lineas.push(format!(
            "candidato cuadro {k}: dispcnt {:04x} sprites {} firma {:016x}",
            e.dispcnt, e.visibles, e.firma
        ));
    }
    for l in &lineas {
        println!("{l}");
    }
    let _ = std::fs::create_dir_all("volcados");
    let ruta = format!("volcados/escaneo-obj-{etiqueta}.txt");
    if let Err(e) = std::fs::write(&ruta, format!("{}\n", lineas.join("\n"))) {
        eprintln!("no se pudo escribir {ruta}: {e}");
        std::process::exit(1);
    }
}
