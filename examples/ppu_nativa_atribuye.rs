//! Atribución de las diferencias de estado por línea a cada componente.
//!
//! Reproduce el cuadro N con la captura por línea activa y construye seis
//! referencias con la PPU por software de `gba-core`:
//!
//! | referencia | estado |
//! |---|---|
//! | `fin` | todo de fin de cuadro: es lo que hace hoy la GPU |
//! | `linea` | todo por línea: es la verdad |
//! | `regs` | solo los registros de E/S (y las referencias afines) por línea |
//! | `paleta` | solo la paleta por línea |
//! | `vram` | solo la VRAM por línea |
//! | `oam` | solo la OAM por línea |
//!
//! Para cada híbrido, el **residuo** son los píxeles que seguirían mal con
//! solo ese componente por línea, y lo que **recupera** son los que se
//! arreglan al añadirlo. El híbrido que deja residuo 0 explica el desajuste
//! entero, y por tanto basta con capturar ese componente por línea.
//!
//! El informe dice además cuántas líneas cambian cada componente y cuántos
//! tramos contiguos de líneas comparten el mismo estado: ese número es el de
//! bandas que la GPU tendría que dibujar si el estado fuera por línea.
//!
//! Escribe `volcados/atribucion-<N>.txt`.
//!
//! Uso:
//!   cargo run --release --features proto --example ppu_nativa_atribuye -- <ruta.gba> <secuencia.txt> <cuadro>

use gba_core::mem::CapturaLinea;

#[path = "comun/mod.rs"]
mod comun;

use comun::color::{canal_8bit, cerca_pixel};
use comun::entrada::leer_secuencia;
use comun::gpu::{ALTO, ANCHO};

fn rgb8(c: u16) -> [u8; 3] {
    [
        canal_8bit(c),
        canal_8bit(c >> 5),
        canal_8bit(c >> 10),
    ]
}

/// El estado completo de la PPU en un momento dado.
struct Estado {
    io: Vec<u8>,
    palette: Vec<u8>,
    vram: Vec<u8>,
    oam: Vec<u8>,
    aff_ref: [i32; 4],
    aff_dirty: [bool; 2],
}

/// Qué componente toma su valor de la línea en vez de del fin de cuadro.
#[derive(Clone, Copy, PartialEq)]
enum Parte {
    Fin,
    Registros,
    Paleta,
    Vram,
    Oam,
}

impl Parte {
    fn nombre(self) -> &'static str {
        match self {
            Parte::Fin => "fin de cuadro",
            Parte::Registros => "registros por línea",
            Parte::Paleta => "paleta por línea",
            Parte::Vram => "VRAM por línea",
            Parte::Oam => "OAM por línea",
        }
    }
}

fn estado_de_fin(m: &gba_core::Machine) -> Estado {
    Estado {
        io: m.bus.io.clone(),
        palette: m.bus.palette.clone(),
        vram: m.bus.vram.clone(),
        oam: m.bus.oam.clone(),
        aff_ref: m.bus.aff_ref,
        aff_dirty: m.bus.aff_dirty,
    }
}

fn estado_de_linea(cap: &CapturaLinea) -> Estado {
    Estado {
        io: cap.io.clone(),
        palette: cap.palette.clone(),
        vram: cap.vram.clone(),
        oam: cap.oam.clone(),
        aff_ref: cap.aff_ref,
        aff_dirty: cap.aff_dirty,
    }
}

/// Dibuja las 160 líneas con la PPU por software, tomando de la línea solo el
/// componente indicado y del resto el estado de fin de cuadro.
fn render(rom: &[u8], fin: &Estado, caps: &[CapturaLinea], parte: Parte) -> Vec<u16> {
    let mut replay = gba_core::Machine::new(rom.to_vec());
    let mut salida = vec![0u16; ANCHO * ALTO];
    for cap in caps {
        let mut e = Estado {
            io: fin.io.clone(),
            palette: fin.palette.clone(),
            vram: fin.vram.clone(),
            oam: fin.oam.clone(),
            aff_ref: fin.aff_ref,
            aff_dirty: fin.aff_dirty,
        };
        match parte {
            Parte::Fin => {}
            Parte::Registros => {
                let l = estado_de_linea(cap);
                e.io = l.io;
                e.aff_ref = l.aff_ref;
                e.aff_dirty = l.aff_dirty;
            }
            Parte::Paleta => e.palette = cap.palette.clone(),
            Parte::Vram => e.vram = cap.vram.clone(),
            Parte::Oam => e.oam = cap.oam.clone(),
        }
        replay.bus.io = e.io;
        replay.bus.palette = e.palette;
        replay.bus.vram = e.vram;
        replay.bus.oam = e.oam;
        replay.bus.aff_ref = e.aff_ref;
        replay.bus.aff_dirty = e.aff_dirty;
        gba_core::ppu::render_scanline(&mut replay.bus, cap.linea);
        let ini = cap.linea * ANCHO;
        salida[ini..ini + ANCHO].copy_from_slice(&replay.bus.framebuffer[ini..ini + ANCHO]);
    }
    salida
}

/// Píxeles que difieren entre dos imágenes, con la tolerancia de color del
/// arnés de comparación.
fn diferencias(a: &[u16], b: &[u16]) -> usize {
    (0..a.len()).filter(|&p| !cerca_pixel(rgb8(a[p]), rgb8(b[p]))).count()
}

/// Componentes cuyo valor de la línea difiere del de fin de cuadro.
fn componentes_cambiados(cap: &CapturaLinea, fin: &Estado) -> Vec<&'static str> {
    let mut out = Vec::new();
    if cap.io != fin.io {
        out.push("regs");
    }
    if cap.palette != fin.palette {
        out.push("paleta");
    }
    if cap.vram != fin.vram {
        out.push("vram");
    }
    if cap.oam != fin.oam {
        out.push("oam");
    }
    out
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 4 {
        eprintln!("uso: ppu_nativa_atribuye <ruta.gba> <secuencia.txt> <cuadro>");
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
    let fin = estado_de_fin(&m);

    // La referencia de fin de cuadro: es lo que dibuja hoy la GPU.
    let ref_fin = render(&rom, &fin, capturas, Parte::Fin);
    // La referencia por línea completa: cada componente de su línea.
    let ref_linea = {
        let mut r = vec![0u16; ANCHO * ALTO];
        let mut replay = gba_core::Machine::new(rom.clone());
        for cap in capturas.iter() {
            let e = estado_de_linea(cap);
            replay.bus.io = e.io;
            replay.bus.palette = e.palette;
            replay.bus.vram = e.vram;
            replay.bus.oam = e.oam;
            replay.bus.aff_ref = e.aff_ref;
            replay.bus.aff_dirty = e.aff_dirty;
            gba_core::ppu::render_scanline(&mut replay.bus, cap.linea);
            let ini = cap.linea * ANCHO;
            r[ini..ini + ANCHO].copy_from_slice(&replay.bus.framebuffer[ini..ini + ANCHO]);
        }
        r
    };

    // Control: la referencia por línea tiene que coincidir con el emulador.
    let control = diferencias(&ref_linea, &framebuffer_emu);
    if control != 0 {
        eprintln!(
            "control falla: {control} diferencias entre la referencia por línea y el emulador; \
             el informe no es válido"
        );
        std::process::exit(1);
    }

    let total = diferencias(&ref_fin, &ref_linea);

    let mut informe = vec![
        format!("atribución del estado por línea, cuadro {objetivo}"),
        format!("control referencia por línea frente a framebuffer del emulador: {control} diferencias (esperado 0)"),
        format!("diferencias de la referencia de fin de cuadro frente a la de fin de cuadro por línea: {total} de {}", ANCHO * ALTO),
        String::new(),
        "residuo por componente (píxeles que seguirían mal con solo ese componente por línea):".to_string(),
    ];

    let mut hibridos = Vec::new();
    for parte in [
        Parte::Registros,
        Parte::Paleta,
        Parte::Vram,
        Parte::Oam,
    ] {
        let img = render(&rom, &fin, capturas, parte);
        let residuo = diferencias(&img, &ref_linea);
        let recupera = total.saturating_sub(residuo);
        hibridos.push((parte, residuo, recupera));
        informe.push(format!(
            "  {:<22} residuo {residuo:6}   recupera {recupera:6}",
            parte.nombre()
        ));
    }

    // Cuántas líneas cambian cada componente y cuántos tramos de líneas
    // comparten el mismo estado completo.
    let mut por_componente = [("regs", 0usize), ("paleta", 0), ("vram", 0), ("oam", 0)];
    let mut detalle = Vec::with_capacity(ALTO);
    let mut tramos = 1usize;
    let mut anterior: Option<Vec<&'static str>> = None;
    for cap in capturas.iter() {
        let cambiados = componentes_cambiados(cap, &fin);
        for c in &cambiados {
            if let Some(slot) = por_componente.iter_mut().find(|(n, _)| n == c) {
                slot.1 += 1;
            }
        }
        let distintos = match &anterior {
            Some(prev) => prev != &cambiados,
            None => true,
        };
        if distintos {
            tramos += 1;
        }
        anterior = Some(cambiados.clone());
        let dif_linea = (0..ANCHO)
            .filter(|&x| {
                !cerca_pixel(
                    rgb8(ref_fin[cap.linea * ANCHO + x]),
                    rgb8(ref_linea[cap.linea * ANCHO + x]),
                )
            })
            .count();
        detalle.push(format!(
            "linea {:3}: cambia [{}]  diferencias frente a fin de cuadro {dif_linea:3}",
            cap.linea,
            if cambiados.is_empty() {
                "-".to_string()
            } else {
                cambiados.join(",")
            }
        ));
    }

    informe.push(String::new());
    informe.push("líneas que cambian cada componente:".to_string());
    for (nombre, cuenta) in por_componente {
        informe.push(format!("  {nombre:<8} {cuenta:4} de {ALTO}"));
    }
    informe.push(format!(
        "tramos contiguos de líneas con distinto estado: {tramos}"
    ));

    // Qué dirección de registro de E/S cambia respecto a fin de cuadro, y
    // cuáles cambian *entre líneas consecutivas* (señal real de escritura a
    // mitad de cuadro, frente a registros de solo lectura como VCOUNT).
    {
        let mut difieren = [0usize; 256];
        let mut cambian_entre_lineas = [0usize; 256];
        let mut anterior: Option<&[u8]> = None;
        for cap in capturas.iter() {
            let l = &cap.io;
            for a in 0..l.len().min(fin.io.len()) {
                if l[a] != fin.io[a] {
                    difieren[a] += 1;
                }
                if let Some(prev) = anterior {
                    if l[a] != prev[a] {
                        cambian_entre_lineas[a] += 1;
                    }
                }
            }
            anterior = Some(l);
        }
        informe.push(String::new());
        informe.push("registros que difieren de fin de cuadro (dirección: líneas / cambios entre líneas):".to_string());
        for a in 0..256usize {
            if difieren[a] == 0 && cambian_entre_lineas[a] == 0 {
                continue;
            }
            let nombre = match a {
                0x00 => "DISPCNT",
                0x02 => "DISPSTAT",
                0x04 => "VCOUNT",
                0x08 => "BG0CNT",
                0x0a => "BG1CNT",
                0x0c => "BG2CNT",
                0x0e => "BG3CNT",
                0x10 => "BG0HOFS",
                0x12 => "BG0VOFS",
                0x14 => "BG1HOFS",
                0x16 => "BG1VOFS",
                0x18 => "BG2HOFS",
                0x1a => "BG2VOFS",
                0x1c => "BG3HOFS",
                0x1e => "BG3VOFS",
                0x28 => "BG2X",
                0x2a => "BG2Y",
                0x2c => "BG3X",
                0x2e => "BG3Y",
                0x30 => "WIN0H",
                0x32 => "WIN1H",
                0x34 => "WIN0V",
                0x36 => "WIN1V",
                0x38 => "WININ",
                0x3a => "WINOUT",
                0x40 => "BLDCNT",
                0x42 => "BLDALPHA",
                0x44 => "BLDY",
                0x50 => "MOSAIC",
                _ => "otro",
            };
            let d = difieren[a];
            let c = cambian_entre_lineas[a];
            let v = u16::from_le_bytes([fin.io[a], *fin.io.get(a + 1).unwrap_or(&0)]);
            informe.push(format!(
                "  0x{a:04X} {nombre:<9} difieren {d:4}   entre líneas {c:4}   fin de cuadro 0x{v:04X}"
            ));
        }
    }
    informe.push(String::new());
    informe.push("detalle por línea:".to_string());
    informe.extend(detalle);

    for l in &informe {
        println!("{l}");
    }

    let _ = std::fs::create_dir_all("volcados");
    let ruta = format!("volcados/atribucion-{objetivo}.txt");
    if let Err(e) = std::fs::write(&ruta, format!("{}\n", informe.join("\n"))) {
        eprintln!("no se pudo escribir {ruta}: {e}");
        std::process::exit(1);
    }
    println!("informe escrito en {ruta}");
}
