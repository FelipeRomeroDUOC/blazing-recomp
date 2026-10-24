//! Experimento de docs/propuestas/2026-10-17_frente-a-frente-del-estado-por-linea.md.
//!
//! Corre el juego una sola vez y, para cada cuadro de un rango, compara **dos** shims contra la PPU
//! por software del mismo cuadro:
//!
//!   - `A` (fin): el shim llamado una vez con el estado de fin de cuadro, que es lo que hace hoy
//!     `pipeline.rs` y aplica a las 160 líneas.
//!   - `B` (por línea): el shim llamado una vez por **estado distinto** y cada fila tomada del render
//!     del estado con el que la PPU por software dibujó esa línea.
//!
//! La diferencia entre A y B es lo que el modelo de un estado por fotograma puede arreglar; lo que
//! queda de B es el residuo de las demás causas (blend, ventanas, afines, VRAM).
//!
//! Escribe `volcados/frente-a-frente.txt`: líneas de resumen y después un CSV con una fila por cuadro.
//!
//! Uso:
//!   cargo run --release --features proto --example ppu_frente_a_frente -- <ruta.gba> <secuencia.txt> <desde> <hasta> [paso]

#[path = "comun/mod.rs"]
mod comun;

use comun::color::{TOLERANCIA, software_a_rgb};
use comun::entrada::leer_secuencia;
use comun::gpu::Gpu;
use comun::registro::io16;

const ANCHO: u32 = 240;
const ALTO: u32 = 160;

/// Expande la secuencia a una tecla por cuadro (activo en bajo, como `KEYINPUT`).
fn teclas_por_cuadro(tramos: &[(u32, u16)]) -> Vec<u16> {
    let mut teclas = Vec::new();
    for &(cuadros, presionados) in tramos {
        for _ in 0..cuadros {
            teclas.push(0x03FF & !presionados);
        }
    }
    teclas
}

struct Comparacion {
    distintos: u64,
    primera_linea: Option<u32>,
}

/// Compara dos imágenes RGB sin relleno. Un píxel es distinto si algún canal difiere más de `TOLERANCIA`.
fn compara(sw: &[u8], hw: &[u8]) -> Comparacion {
    let mut distintos = 0u64;
    let mut primera_linea = None;
    for y in 0..ALTO {
        let mut fila_distinta = false;
        for x in 0..ANCHO {
            let o = ((y * ANCHO + x) * 3) as usize;
            let distinto = (0..3).any(|k| sw[o + k].abs_diff(hw[o + k]) > TOLERANCIA);
            if distinto {
                distintos += 1;
                fila_distinta = true;
            }
        }
        if fila_distinta && primera_linea.is_none() {
            primera_linea = Some(y);
        }
    }
    Comparacion { distintos, primera_linea }
}

/// Porcentaje de píxeles distintos sobre el total.
fn porcentaje(c: &Comparacion) -> f64 {
    c.distintos as f64 * 100.0 / (ANCHO * ALTO) as f64
}

/// Los cuatro buffers que lee el shim, en el orden en que se comparan: VRAM, OAM, paleta e I/O.
type Estado<'a> = (&'a [u8], &'a [u8], &'a [u8], &'a [u8]);

/// El estado que el shim necesita para dibujar una captura.
fn estado(c: &gba_core::mem::CapturaLinea) -> Estado<'_> {
    (&c.vram, &c.oam, &c.palette, &c.io)
}

/// Dos estados son el mismo si coinciden los cuatro buffers que lee el shim.
fn mismo_estado(a: Estado<'_>, b: Estado<'_>) -> bool {
    a.0 == b.0 && a.1 == b.1 && a.2 == b.2 && a.3 == b.3
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let (Some(ruta_rom), Some(ruta_secuencia)) = (args.get(1), args.get(2)) else {
        eprintln!(
            "uso: ppu_frente_a_frente <ruta.gba> <secuencia.txt> <desde> <hasta> [paso]"
        );
        std::process::exit(2);
    };
    let desde: u32 = match args.get(3).and_then(|s| s.parse().ok()) {
        Some(n) => n,
        None => {
            eprintln!("falta el cuadro inicial");
            std::process::exit(2);
        }
    };
    let hasta: u32 = match args.get(4).and_then(|s| s.parse().ok()) {
        Some(n) => n,
        None => {
            eprintln!("falta el cuadro final");
            std::process::exit(2);
        }
    };
    let paso: u32 = args
        .get(5)
        .and_then(|s| s.parse().ok())
        .unwrap_or(1)
        .max(1);
    if hasta < desde {
        eprintln!("el rango está al revés");
        std::process::exit(2);
    }

    let rom = match std::fs::read(ruta_rom) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("ROM no encontrado: {ruta_rom}");
            eprintln!("detalle: {e}");
            std::process::exit(1);
        }
    };
    let texto = match std::fs::read_to_string(ruta_secuencia) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("secuencia no encontrada: {ruta_secuencia} ({e})");
            std::process::exit(1);
        }
    };
    let tramos = match leer_secuencia(&texto) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("secuencia inválida: {e}");
            std::process::exit(1);
        }
    };
    let teclas = teclas_por_cuadro(&tramos);
    if teclas.is_empty() {
        eprintln!("la secuencia no tiene cuadros");
        std::process::exit(1);
    }

    let mut gpu = match Gpu::nueva() {
        Ok(g) => g,
        Err(e) => {
            eprintln!("{e}");
            std::process::exit(1);
        }
    };

    let mut m = gba_core::Machine::new(rom);
    let mut cuadro: u32 = 0;
    let mut objetivo = desde;
    let mut filas: Vec<String> = Vec::new();
    let (mut suma_fin, mut suma_linea) = (0.0f64, 0.0f64);
    let (mut estados_min, mut estados_max) = (u32::MAX, 0u32);

    while objetivo <= hasta {
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

        if m.bus.capturas_linea.len() != ALTO as usize {
            eprintln!(
                "cuadro {objetivo}: se esperaban {ALTO} capturas y hay {}",
                m.bus.capturas_linea.len()
            );
            objetivo += paso;
            continue;
        }
        let capturas = &m.bus.capturas_linea;
        if m.bus.framebuffer.len() != (ANCHO * ALTO) as usize {
            eprintln!("cuadro {objetivo}: framebuffer inesperado");
            std::process::exit(1);
        }
        let sw = software_a_rgb(&m.bus.framebuffer);

        // A: el shim de hoy, un estado para las 160 líneas.
        let a = gpu.renderiza(
            &m.bus.vram,
            &m.bus.oam,
            &m.bus.palette,
            &m.bus.io,
        );

        // B: agrupar las capturas por estado y renderizar una vez por grupo.
        let mut representantes: Vec<usize> = Vec::new();
        let mut lineas_por_grupo: Vec<Vec<usize>> = Vec::new();
        for k in 0..ALTO as usize {
            let grupo = representantes
                .iter()
                .position(|&rep| mismo_estado(estado(&capturas[rep]), estado(&capturas[k])));
            let g = match grupo {
                Some(g) => g,
                None => {
                    representantes.push(k);
                    lineas_por_grupo.push(Vec::new());
                    representantes.len() - 1
                }
            };
            lineas_por_grupo[g].push(k);
        }
        let mut b = vec![0u8; (ANCHO * ALTO * 3) as usize];
        for (g, &rep) in representantes.iter().enumerate() {
            let cap = &capturas[rep];
            let img = gpu.renderiza(&cap.vram, &cap.oam, &cap.palette, &cap.io);
            for &k in &lineas_por_grupo[g] {
                let ini = k * ANCHO as usize * 3;
                b[ini..ini + ANCHO as usize * 3]
                    .copy_from_slice(&img[ini..ini + ANCHO as usize * 3]);
            }
        }

        let lineas_con_cambio = capturas
            .iter()
            .filter(|c| !mismo_estado(estado(&capturas[0]), estado(c)))
            .count();
        // Líneas que el shim de hoy dibuja con un estado distinto del que usó el software.
        let fin: Estado = (&m.bus.vram, &m.bus.oam, &m.bus.palette, &m.bus.io);
        let lineas_ajenas_al_fin = capturas
            .iter()
            .filter(|c| !mismo_estado(fin, estado(c)))
            .count();

        // Mezcla: cuántas líneas se dibujaron con un efecto de `BLDCNT` activo (bits 6 y 7).
        let blend_lineas = capturas
            .iter()
            .filter(|c| io16(&c.io, 0x50) & 0x00C0 != 0)
            .count();
        let bldcnt = io16(&m.bus.io, 0x50);

        let ca = compara(&sw, &a);
        let cb = compara(&sw, &b);
        let (pa, pb) = (porcentaje(&ca), porcentaje(&cb));
        suma_fin += pa;
        suma_linea += pb;
        let estados = representantes.len() as u32;
        estados_min = estados_min.min(estados);
        estados_max = estados_max.max(estados);
        let primera = |c: &Comparacion| {
            c.primera_linea
                .map_or("-".to_string(), |y| y.to_string())
        };
        filas.push(format!(
            "{objetivo},{estados},{lineas_con_cambio},{lineas_ajenas_al_fin},{pa:.4},{pb:.4},{:.4},{},{},{bldcnt:04x},{blend_lineas}",
            pa - pb,
            primera(&ca),
            primera(&cb),
        ));
        println!(
            "cuadro {objetivo}: estados {estados} ajenas_al_fin {lineas_ajenas_al_fin} fin {pa:.2}% linea {pb:.2}%"
        );
        objetivo += paso;
    }

    if filas.is_empty() {
        eprintln!("no se midió ningún cuadro");
        std::process::exit(1);
    }
    let n = filas.len() as f64;
    let media_fin = suma_fin / n;
    let media_linea = suma_linea / n;
    let mut informe = String::new();
    informe.push_str(&format!(
        "resumen cuadros {} desde {desde} hasta {hasta} paso {paso}\n",
        filas.len()
    ));
    informe.push_str(&format!("estados min {estados_min} max {estados_max}\n"));
    informe.push_str(&format!("error medio fin {media_fin:.4}\n"));
    informe.push_str(&format!("error medio linea {media_linea:.4}\n"));
    informe.push_str(&format!("reduccion {:.4}\n", media_fin - media_linea));
    informe.push_str(&format!("residuo {media_linea:.4}\n"));
    informe.push_str("cuadro,estados,lineas_con_cambio,lineas_ajenas_al_fin,pct_fin,pct_linea,delta_pct,primera_fin,primera_linea,bldcnt,blend_lineas\n");
    for fila in &filas {
        informe.push_str(fila);
        informe.push('\n');
    }
    if let Err(e) = std::fs::create_dir_all("volcados") {
        eprintln!("no se pudo crear volcados: {e}");
        std::process::exit(1);
    }
    if let Err(e) = std::fs::write("volcados/frente-a-frente.txt", &informe) {
        eprintln!("no se pudo escribir el informe: {e}");
        std::process::exit(1);
    }

    println!();
    println!("cuadros: {}", filas.len());
    println!("estados por cuadro: {estados_min} a {estados_max}");
    println!("error medio con el shim de fin de cuadro: {media_fin:.4} %");
    println!("error medio con estado por línea: {media_linea:.4} %");
    println!("reducción atribuible al estado por línea: {:.4} puntos", media_fin - media_linea);
    println!("residuo (blend, ventanas, afines, VRAM): {media_linea:.4} %");
    println!("informe: volcados/frente-a-frente.txt");
}
