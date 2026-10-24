//! Experimento de docs/propuestas/2026-10-17_shim-contra-aportacion-cuadro-1000.md.
//!
//! Compara el shim GPU con la PPU por software en los píxeles de aportación fuera de cobertura
//! del cuadro N (V0 != V1 y V2 == V3 en la PPU). Para cada variante de DISPCNT (V0 completa,
//! V1 sin OBJ) renderiza el shim de dos formas:
//!   - fin: un solo estado, el de fin de cuadro (lo que hace hoy `pipeline.rs`);
//!   - linea: un estado por línea, agrupando líneas con el mismo estado.
//! Control: el shim renderizado dos veces con el mismo estado debe dar el mismo resultado.
//!
//! Escribe volcados/shim-aportacion-cuadro-<N>.txt. Si el control falla o no hay GPU, no escribe.
//!
//! Uso:
//!   cargo run --release --features proto --example shim_aportacion -- <ruta.gba> <secuencia.txt> <cuadro>

#[path = "comun/mod.rs"]
mod comun;

use comun::color::{cerca, software_a_rgb};
use comun::entrada::leer_secuencia;
use comun::gpu::{ALTO, ANCHO, Gpu};
use comun::registro::io_con_variante;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 4 {
        eprintln!("uso: shim_aportacion <ruta.gba> <secuencia.txt> <cuadro>");
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

    // Juego hasta el cuadro pedido, con captura por línea solo durante ese cuadro.
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
    // Estado de fin de cuadro, el mismo que usa hoy pipeline.rs.
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
    // Aportación fuera de cobertura: V0 != V1 y V2 == V3 en la PPU.
    let mascara: Vec<usize> = (0..n)
        .filter(|&p| ppu[0][p] != ppu[1][p] && ppu[2][p] == ppu[3][p])
        .collect();
    let total = mascara.len();
    let ppu_v0 = software_a_rgb(&ppu[0]);
    let ppu_v1 = software_a_rgb(&ppu[1]);

    let mut gpu = match Gpu::nueva() {
        Ok(g) => g,
        Err(e) => {
            eprintln!("{e}");
            std::process::exit(1);
        }
    };

    // Control: dos renderizados del mismo estado deben coincidir byte a byte.
    let control_a = gpu.renderiza(&vram_fin, &oam_fin, &pal_fin, &io_con_variante(&io_fin, 0));
    let control_b = gpu.renderiza(&vram_fin, &oam_fin, &pal_fin, &io_con_variante(&io_fin, 0));
    let control = control_a
        .iter()
        .zip(control_b.iter())
        .filter(|(a, b)| a != b)
        .count();
    if control != 0 {
        eprintln!("control de determinismo falla: {control} bytes distintos; no se escribe el informe");
        std::process::exit(1);
    }

    // Shim con estado de fin de cuadro, V0 y V1.
    let fin_v0 = gpu.renderiza(&vram_fin, &oam_fin, &pal_fin, &io_con_variante(&io_fin, 0));
    let fin_v1 = gpu.renderiza(&vram_fin, &oam_fin, &pal_fin, &io_con_variante(&io_fin, 1));
    let fin_v2 = gpu.renderiza(&vram_fin, &oam_fin, &pal_fin, &io_con_variante(&io_fin, 2));
    let fin_v3 = gpu.renderiza(&vram_fin, &oam_fin, &pal_fin, &io_con_variante(&io_fin, 3));

    // Shim con un estado por línea, V0 y V1. Las líneas con el mismo estado se dibujan una vez.
    let mut lin_v = Vec::new();
    for v in 0..4 {
        let ios: Vec<Vec<u8>> = capturas.iter().map(|c| io_con_variante(&c.io, v)).collect();
        let mut representantes: Vec<usize> = Vec::new();
        let mut grupos: Vec<Vec<usize>> = Vec::new();
        for k in 0..ALTO {
            let c = &capturas[k];
            let pos = representantes.iter().position(|&r| {
                let cr = &capturas[r];
                cr.vram == c.vram && cr.oam == c.oam && cr.palette == c.palette && ios[r] == ios[k]
            });
            match pos {
                Some(g) => grupos[g].push(k),
                None => {
                    representantes.push(k);
                    grupos.push(vec![k]);
                }
            }
        }
        let mut b = vec![0u8; ANCHO * ALTO * 3];
        for (g, &r) in representantes.iter().enumerate() {
            let cap = &capturas[r];
            let img = gpu.renderiza(&cap.vram, &cap.oam, &cap.palette, &ios[r]);
            for &k in &grupos[g] {
                let ini = k * ANCHO * 3;
                b[ini..ini + ANCHO * 3].copy_from_slice(&img[ini..ini + ANCHO * 3]);
            }
        }
        lin_v.push(b);
    }
    let lin_v0 = &lin_v[0];
    let lin_v1 = &lin_v[1];

    let fin_igual_v0 = mascara.iter().filter(|&&p| cerca(&fin_v0, &ppu_v0, p)).count();
    let fin_igual_v1 = mascara.iter().filter(|&&p| cerca(&fin_v0, &ppu_v1, p)).count();
    let fin_v0_v1_distinto = mascara.iter().filter(|&&p| !cerca(&fin_v0, &fin_v1, p)).count();
    let lin_igual_v0 = mascara.iter().filter(|&&p| cerca(lin_v0, &ppu_v0, p)).count();
    let lin_igual_v1 = mascara.iter().filter(|&&p| cerca(lin_v0, &ppu_v1, p)).count();
    let lin_v0_v1_distinto = mascara.iter().filter(|&&p| !cerca(lin_v0, lin_v1, p)).count();

    // Categorías exclusivas del shim de fin de cuadro, para que sumen el total.
    let mut cat_igual_v0 = 0usize;
    let mut cat_solo_v1 = 0usize;
    let mut cat_otro = 0usize;
    for &p in &mascara {
        if cerca(&fin_v0, &ppu_v0, p) {
            cat_igual_v0 += 1;
        } else if cerca(&fin_v0, &ppu_v1, p) {
            cat_solo_v1 += 1;
        } else {
            cat_otro += 1;
        }
    }

    // Control de arnés: coincidencia en toda la pantalla. Si es casi 0, el arnés está mal y el
    // resultado sobre la aportación no vale.
    let completo_fin_v0 = (0..n).filter(|&p| cerca(&fin_v0, &ppu_v0, p)).count();
    let completo_lin_v0 = (0..n).filter(|&p| cerca(lin_v0, &ppu_v0, p)).count();

    let mut lineas = vec![
        format!("resumen cuadro {objetivo}: shim GPU contra PPU en la aportacion"),
        format!("control determinismo shim: {control}"),
        format!("aportacion total: {total}"),
        format!("shim fin igual PPU V0: {fin_igual_v0}"),
        format!("shim fin igual PPU V1: {fin_igual_v1}"),
        format!("shim fin V0 distinto de V1: {fin_v0_v1_distinto}"),
        format!("shim linea igual PPU V0: {lin_igual_v0}"),
        format!("shim linea igual PPU V1: {lin_igual_v1}"),
        format!("shim linea V0 distinto de V1: {lin_v0_v1_distinto}"),
        format!("shim fin categoria igual V0: {cat_igual_v0}"),
        format!("shim fin categoria solo V1: {cat_solo_v1}"),
        format!("shim fin categoria otro: {cat_otro}"),
        format!(
            "shim fin suma: {}",
            cat_igual_v0 + cat_solo_v1 + cat_otro
        ),
    ];
    lineas.push(format!("control cuadro completo fin V0 igual PPU V0: {completo_fin_v0} de {n}"));
    lineas.push(format!("control cuadro completo linea V0 igual PPU V0: {completo_lin_v0} de {n}"));
    // Propuesta shim-contra-fondo-vacio: el shim frente a V3 (fondo vacío) y V2 (solo OBJ).
    // Va a un archivo aparte; el informe shim-aportacion-cuadro-<N>.txt no cambia.
    let ppu_v2 = software_a_rgb(&ppu[2]);
    let ppu_v3 = software_a_rgb(&ppu[3]);
    let lin_v2 = &lin_v[2];
    let lin_v3 = &lin_v[3];
    let control_fin_v3 = (0..n).filter(|&p| cerca(&fin_v3, &ppu_v3, p)).count();
    let control_lin_v3 = (0..n).filter(|&p| cerca(lin_v3, &ppu_v3, p)).count();
    if control_fin_v3 * 100 < n * 30 {
        eprintln!("control de V3 bajo: {control_fin_v3} de {n} en estado de fin; no se escribe el informe");
        std::process::exit(1);
    }
    let fin_igual_v3 = mascara.iter().filter(|&&p| cerca(&fin_v0, &ppu_v3, p)).count();
    let lin_igual_v3 = mascara.iter().filter(|&&p| cerca(lin_v0, &ppu_v3, p)).count();
    let fin_igual_v2 = mascara.iter().filter(|&&p| cerca(&fin_v2, &ppu_v2, p)).count();
    let lin_igual_v2 = mascara.iter().filter(|&&p| cerca(lin_v2, &ppu_v2, p)).count();
    let (mut c_v0, mut c_v1, mut c_v3, mut c_otro) = (0usize, 0usize, 0usize, 0usize);
    for &p in &mascara {
        if cerca(&fin_v0, &ppu_v0, p) {
            c_v0 += 1;
        } else if cerca(&fin_v0, &ppu_v1, p) {
            c_v1 += 1;
        } else if cerca(&fin_v0, &ppu_v3, p) {
            c_v3 += 1;
        } else {
            c_otro += 1;
        }
    }
    let fondo_lineas = vec![
        format!("resumen cuadro {objetivo}: shim GPU frente a fondo vacio (V3) y solo OBJ (V2) en la aportacion"),
        format!("control cuadro completo fin V3 igual PPU V3: {control_fin_v3} de {n}"),
        format!("control cuadro completo linea V3 igual PPU V3: {control_lin_v3} de {n}"),
        format!("aportacion total: {total}"),
        format!("shim fin igual PPU V3: {fin_igual_v3}"),
        format!("shim linea igual PPU V3: {lin_igual_v3}"),
        format!("shim fin igual PPU V2: {fin_igual_v2}"),
        format!("shim linea igual PPU V2: {lin_igual_v2}"),
        format!("shim fin categoria igual V0: {c_v0}"),
        format!("shim fin categoria solo V1: {c_v1}"),
        format!("shim fin categoria solo V3: {c_v3}"),
        format!("shim fin categoria otro: {c_otro}"),
        format!("shim fin suma: {}", c_v0 + c_v1 + c_v3 + c_otro),
    ];
    for l in &fondo_lineas {
        println!("{l}");
    }
    let _ = std::fs::create_dir_all("volcados");
    let ruta_fondo = format!("volcados/shim-fondo-cuadro-{objetivo}.txt");
    if let Err(e) = std::fs::write(&ruta_fondo, format!("{}\n", fondo_lineas.join("\n"))) {
        eprintln!("no se pudo escribir {ruta_fondo}: {e}");
        std::process::exit(1);
    }

    for l in &lineas {
        println!("{l}");
    }
    let _ = std::fs::create_dir_all("volcados");
    let ruta = format!("volcados/shim-aportacion-cuadro-{objetivo}.txt");
    if let Err(e) = std::fs::write(&ruta, format!("{}\n", lineas.join("\n"))) {
        eprintln!("no se pudo escribir {ruta}: {e}");
        std::process::exit(1);
    }
}
