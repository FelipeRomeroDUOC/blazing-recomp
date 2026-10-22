//! Experimento de docs/propuestas/2026-10-17_localizar-pixeles-obj-y-contenido-bg-cuadro-1000.md.
//!
//! Con las capturas por línea del cuadro N (feature `perf-video`), redibuja cada línea con la
//! PPU por software en cuatro variantes de DISPCNT (BLDCNT original en todas):
//!   0: completa (control: debe coincidir con el juego);
//!   1: sin OBJ;
//!   2: solo OBJ;
//!   3: nada (sin capas).
//! Las definiciones son las mismas que en `ppu_capas_linea`, así que las cifras son comparables:
//!   aportación = píxeles que cambian entre 0 y 1;
//!   cobertura  = píxeles que cambian entre 2 y 3.
//! Cada píxel de la aportación se separa en:
//!   dentro de cobertura: OBJ sola cambia ese píxel (2 != 3);
//!   fuera de cobertura:  OBJ sola no lo cambia (2 == 3), pero sí cambia al quitar OBJ con el resto.
//! `razon_camino` de `compose` es una variable local de ppu.rs, pero sus efectos son públicos: el
//! contador `perf-counters` que incrementa cada rama de la ruta. Se vacía antes de dibujar cada
//! línea y se lee después, así que la ruta se mide sin cambiar `ppu.rs`:
//!   `lineas_rectas` → 0 (vía rápida), `descartes_mezcla` → 1 (alpha por tramos),
//!   `descartes_ventana` → 2 (ventana de sprites), `descartes_semi` → 3 (sprite translúcido).
//!
//! Escribe volcados/pixeles-obj-cuadro-<N>.txt: resumen, un renglón `pixel` por cada píxel fuera
//! de cobertura con sus colores en las variantes 0 a 3, y un renglón `linea` por cada línea afectada
//! con la ruta de `compose` en la variante completa (V0) y sin OBJ (V1).
//!
//! Uso:
//!   cargo run --release --features proto --example ppu_pixeles_obj -- <ruta.gba> <secuencia.txt> <cuadro>

#[path = "comun/mod.rs"]
mod comun;

use comun::entrada::leer_secuencia;
use comun::gpu::{ALTO, ANCHO};
use comun::registro::dispcnt_de;

const VARIANTES: usize = 4;

/// Cuenta los píxeles que difieren entre dos trozos del mismo tamaño.
fn distintos(a: &[u16], b: &[u16]) -> usize {
    a.iter().zip(b.iter()).filter(|(x, y)| x != y).count()
}

/// Ruta de `compose` que tomó la última línea dibujada, leída de los contadores públicos.
/// Devuelve `None` si la línea no incrementó ningún contador de ruta.
fn ruta_camino(c: &gba_core::GbaPerfCounters) -> Option<u8> {
    if c.lineas_rectas > 0 {
        Some(0)
    } else if c.descartes_mezcla > 0 {
        Some(1)
    } else if c.descartes_ventana > 0 {
        Some(2)
    } else if c.descartes_semi > 0 {
        Some(3)
    } else {
        None
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 4 {
        eprintln!("uso: ppu_pixeles_obj <ruta.gba> <secuencia.txt> <cuadro>");
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
    let fb_juego = m.bus.framebuffer.clone();
    let capturas = &m.bus.capturas_linea;
    if capturas.len() != ALTO {
        eprintln!("se esperaban {ALTO} capturas y hay {}", capturas.len());
        std::process::exit(1);
    }

    // Redibujar cada línea en las cuatro variantes, con la PPU por software.
    let mut cuadros_var: Vec<Vec<u16>> = vec![vec![0u16; ANCHO * ALTO]; VARIANTES];
    let mut ruta_v0 = vec![None::<u8>; ALTO];
    let mut ruta_v1 = vec![None::<u8>; ALTO];
    let mut replay = gba_core::Machine::new(rom);
    for cap in capturas.iter() {
        let base = u16::from_le_bytes([cap.io[0], cap.io[1]]);
        replay.bus.palette = cap.palette.clone();
        replay.bus.vram = cap.vram.clone();
        replay.bus.oam = cap.oam.clone();
        for v in 0..VARIANTES {
            let d = dispcnt_de(v, base);
            replay.bus.io = cap.io.clone();
            replay.bus.io[0] = (d & 0xFF) as u8;
            replay.bus.io[1] = (d >> 8) as u8;
            // La PPU avanza aff_ref al dibujar: cada variante parte del valor de la captura.
            replay.bus.aff_ref = cap.aff_ref;
            replay.bus.aff_dirty = cap.aff_dirty;
            // Los contadores se vacían antes de dibujar: la ruta de esta línea es el incremento.
            replay.bus.perf_counters = Default::default();
            gba_core::ppu::render_scanline(&mut replay.bus, cap.linea);
            let ruta = ruta_camino(&replay.bus.perf_counters);
            if v == 0 {
                ruta_v0[cap.linea] = ruta;
            } else if v == 1 {
                ruta_v1[cap.linea] = ruta;
            }

            let ini = cap.linea * ANCHO;
            cuadros_var[v][ini..ini + ANCHO]
                .copy_from_slice(&replay.bus.framebuffer[ini..ini + ANCHO]);
        }
    }

    let v0 = &cuadros_var[0];
    let v1 = &cuadros_var[1];
    let v2 = &cuadros_var[2];
    let v3 = &cuadros_var[3];
    let control = distintos(v0, &fb_juego);
    let cobertura = distintos(v2, v3);

    let mut aportacion = 0usize;
    let mut dentro = 0usize;
    let mut fuera: Vec<usize> = Vec::new();
    for i in 0..ANCHO * ALTO {
        if v0[i] != v1[i] {
            aportacion += 1;
            if v2[i] != v3[i] {
                dentro += 1;
            } else {
                fuera.push(i);
            }
        }
    }

    let mut por_linea = vec![0usize; ALTO];
    for &i in &fuera {
        por_linea[i / ANCHO] += 1;
    }
    let lineas_fuera = por_linea.iter().filter(|&&n| n > 0).count();

    // H8 frente a H9: separar los píxeles fuera de cobertura según si la ruta de `compose`
    // cambia al quitar OBJ en esa misma línea.
    let lineas_cambio_ruta = (0..ALTO).filter(|&l| ruta_v0[l] != ruta_v1[l]).count();
    let mut fuera_con_cambio = 0usize;
    let mut fuera_sin_cambio = 0usize;
    for (l, &n) in por_linea.iter().enumerate() {
        if n == 0 {
            continue;
        }
        if ruta_v0[l] != ruta_v1[l] {
            fuera_con_cambio += n;
        } else {
            fuera_sin_cambio += n;
        }
    }

    let lineas = vec![
        format!(
            "resumen cuadro {objetivo}: variante completa vs juego = {control} píxeles distintos de {} (esperado 0); razon_camino medida con perf-counters",
            fb_juego.len()
        ),
        format!("obj aportacion total: {aportacion}"),
        format!("obj cobertura total: {cobertura}"),
        format!("obj aportacion dentro de cobertura: {dentro}"),
        format!("obj aportacion fuera de cobertura: {}", fuera.len()),
        format!("obj control completa vs juego: {control}"),
        format!("obj lineas con aportacion fuera de cobertura: {lineas_fuera}"),
        format!("obj lineas con cambio de ruta al quitar OBJ: {lineas_cambio_ruta}"),
        format!("obj fuera con cambio de ruta: {fuera_con_cambio}"),
        format!("obj fuera sin cambio de ruta: {fuera_sin_cambio}"),
    ];
    for l in &lineas {
        println!("{l}");
    }

    let mut detalle = format!("{}\n", lineas.join("\n"));
    for &i in &fuera {
        detalle.push_str(&format!(
            "pixel x={} linea={} V0={:04X} V1={:04X} V2={:04X} V3={:04X}\n",
            i % ANCHO,
            i / ANCHO,
            v0[i],
            v1[i],
            v2[i],
            v3[i]
        ));
    }
    let ruta_txt = |r: Option<u8>| match r {
        Some(n) => n.to_string(),
        None => "?".to_string(),
    };
    for (l, &n) in por_linea.iter().enumerate() {
        if n > 0 {
            detalle.push_str(&format!(
                "linea {l} fuera {n} ruta_v0 {} ruta_v1 {}\n",
                ruta_txt(ruta_v0[l]),
                ruta_txt(ruta_v1[l])
            ));
        }
    }

    let _ = std::fs::create_dir_all("volcados");
    let ruta = format!("volcados/pixeles-obj-cuadro-{objetivo}.txt");
    if let Err(e) = std::fs::write(&ruta, detalle) {
        eprintln!("no se pudo escribir {ruta}: {e}");
    }
}
