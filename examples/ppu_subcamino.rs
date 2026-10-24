//! Experimento de docs/propuestas/2026-10-17_subcamino-de-composicion-con-sprite-translucido.md.
//!
//! Con las capturas por línea del cuadro N (feature `perf-video`), dibuja cada línea con la PPU por
//! software en cuatro variantes sobre el mismo estado capturado:
//!   A: natural (OBJ tal cual, camino que elija el motor);
//!   B: sin OBJ (bit 12 de DISPCNT apagado);
//!   G: natural con el camino general por píxel forzado (`forzar_camino_general`);
//!   D: sin OBJ con el camino general forzado.
//!
//! Comparaciones por línea:
//!   diff_ab = A vs B → lo que cambia quitar OBJ, la medida ya conocida;
//!   diff_ad = G vs D → quitar OBJ forzando el mismo camino en las dos;
//!   diff_ag = A vs G → solo el camino, sin tocar DISPCNT;
//!   diff_bd = B vs D → solo el camino, sin OBJ.
//! Si diff_ad es 0 en la banda, la diferencia de quitar OBJ era el camino y no la capa.
//!
//! También mide coste: por cada línea de la banda dibuja 200 veces con cada camino y anota la
//! mediana de `compose_ns`. Con eso decide, con la regla fijada en la propuesta (general ≤ 1,25×
//! tramos), qué camino se corrige.
//!
//! Escribe volcados/subcamino-cuadro-<N>.txt: resumen, coste, decisión y una fila `linea` por línea.
//!
//! Uso:
//!   cargo run --release --features proto --example ppu_subcamino -- <ruta.gba> <secuencia.txt> <cuadro>
/// Repeticiones por camino para la mediana de coste.

#[path = "comun/mod.rs"]
mod comun;

use comun::entrada::leer_secuencia;
use comun::gpu::{ALTO, ANCHO};
use comun::registro::BIT_OBJ;

const MUESTRAS: usize = 200;

/// Lo medido en una línea.
struct Fila {
    linea: usize,
    semi_con_obj: bool,
    semi_sin_obj: bool,
    camino_con_obj: &'static str,
    camino_sin_obj: &'static str,
    bldcnt: u16,
    eva: u32,
    evb: u32,
    diff_ab: usize,
    diff_ad: usize,
    diff_ag: usize,
    diff_bd: usize,
}

impl Fila {
    fn texto(&self) -> String {
        format!(
            "linea {l} semi_con_obj {a} semi_sin_obj {b} camino_con_obj {ca} camino_sin_obj {cb} diff_ab {ab} diff_ad {ad} diff_ag {ag} diff_bd {bd}",
            l = self.linea,
            a = self.semi_con_obj as u8,
            b = self.semi_sin_obj as u8,
            ca = self.camino_con_obj,
            cb = self.camino_sin_obj,
            ab = self.diff_ab,
            ad = self.diff_ad,
            ag = self.diff_ag,
            bd = self.diff_bd,
        ) + &format!(
            " bldcnt {:#06x} eva {} evb {}",
            self.bldcnt, self.eva, self.evb
        )
    }
}

fn distintos(a: &[u16], b: &[u16]) -> usize {
    a.iter().zip(b.iter()).filter(|(x, y)| x != y).count()
}

fn split6(c: u16) -> (u32, u32, u32) {
    (
        (c & 31) as u32,
        (((c >> 5) & 31) as u32) << 1,
        ((c >> 10) & 31) as u32,
    )
}

fn join6(r: u32, g: u32, b: u32) -> u16 {
    (r | ((g >> 1) << 5) | (b << 10)) as u16
}

/// Copia de `alpha_blend` de `ppu.rs` (privada), para comprobar desde fuera qué capa se mezcló.
fn alpha_blend(a: u16, b: u16, eva: u32, evb: u32) -> u16 {
    let (ar, ag, ab) = split6(a);
    let (br, bg, bb) = split6(b);
    join6(
        ((ar * eva + br * evb + 8) >> 4).min(31),
        ((ag * eva + bg * evb + 8) >> 4).min(63),
        ((ab * eva + bb * evb + 8) >> 4).min(31),
    )
}

/// Mediana de una lista de enteros (la ordena).
fn mediana(valores: &mut [u64]) -> u64 {
    if valores.is_empty() {
        return 0;
    }
    valores.sort_unstable();
    valores[valores.len() / 2]
}

/// Nombre del camino que tomó la última línea dibujada, según los contadores públicos.
fn camino(c: &gba_core::GbaPerfCounters) -> &'static str {
    if c.lineas_alpha_tramos > 0 {
        "tramos"
    } else if c.lineas_camino_general > 0 {
        "general"
    } else {
        "recto"
    }
}

/// Dibuja una línea de la captura con un DISPCNT concreto y devuelve el tramo y sus contadores.
fn dibujar(
    replay: &mut gba_core::Machine,
    cap: &gba_core::mem::CapturaLinea,
    dispcnt: u16,
) -> (Vec<u16>, &'static str, bool) {
    replay.bus.palette = cap.palette.clone();
    replay.bus.vram = cap.vram.clone();
    replay.bus.oam = cap.oam.clone();
    replay.bus.io = cap.io.clone();
    replay.bus.io[0] = (dispcnt & 0xFF) as u8;
    replay.bus.io[1] = (dispcnt >> 8) as u8;
    // La PPU avanza aff_ref al dibujar: cada variante parte del valor de la captura.
    replay.bus.aff_ref = cap.aff_ref;
    replay.bus.aff_dirty = cap.aff_dirty;
    replay.bus.perf_counters = Default::default();
    gba_core::ppu::render_scanline(&mut replay.bus, cap.linea);
    let c = &replay.bus.perf_counters;
    let ini = cap.linea * ANCHO;
    (
        replay.bus.framebuffer[ini..ini + ANCHO].to_vec(),
        camino(c),
        c.lineas_con_sprite_semi > 0,
    )
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 4 {
        eprintln!("uso: ppu_subcamino <ruta.gba> <secuencia.txt> <cuadro>");
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

    let mut replay = gba_core::Machine::new(rom);
    let mut natural = vec![0u16; ANCHO * ALTO];
    let mut filas: Vec<Fila> = Vec::with_capacity(ALTO);
    // Comprobación de la hipótesis de la capa negra: en los píxeles que cambian al quitar OBJ,
    // ¿es V0 exactamente la mezcla alpha de V1 con negro? Y ¿cuántos de ellos son negros puros?
    let mut blend_negro = 0usize;
    let mut blend_sprite_top = 0usize;
    let mut negros = 0usize;
    let mut sin_explicar = 0usize;
    let mut bldcnt_muestra: u16 = 0;
    let mut bldalpha_muestra: u16 = 0;

    for cap in capturas.iter() {
        let base = u16::from_le_bytes([cap.io[0], cap.io[1]]);
        let sin_obj = base & !BIT_OBJ;

        gba_core::ppu::forzar_alpha_tramos(false);
        gba_core::ppu::forzar_camino_general(false);
        let (pa, ca, sa) = dibujar(&mut replay, cap, base);
        let (pb, cb, sb) = dibujar(&mut replay, cap, sin_obj);

        gba_core::ppu::forzar_camino_general(true);
        let (pg, _cg, _sg) = dibujar(&mut replay, cap, base);
        gba_core::ppu::forzar_alpha_tramos(false);
        let (pd, _cd, _sd) = dibujar(&mut replay, cap, sin_obj);
        gba_core::ppu::forzar_camino_general(false);

        let bldcnt = u16::from_le_bytes([cap.io[0x50], cap.io[0x51]]);
        let bldalpha = u16::from_le_bytes([cap.io[0x52], cap.io[0x53]]);
        let eva = (bldalpha & 0x1F).min(16) as u32;
        let evb = ((bldalpha >> 8) & 0x1F).min(16) as u32;
        for x in 0..ANCHO {
            if pa[x] != pb[x] {
                if pa[x] == alpha_blend(pb[x], 0x0000, eva, evb) {
                    blend_negro += 1;
                    if bldcnt_muestra == 0 {
                        bldcnt_muestra = bldcnt;
                        bldalpha_muestra = bldalpha;
                    }
                } else if pa[x] == alpha_blend(0x0000, pb[x], eva, evb) {
                    blend_sprite_top += 1;
                } else if pa[x] == 0x0000 {
                    negros += 1;
                } else {
                    sin_explicar += 1;
                }
            }
        }

        let ini = cap.linea * ANCHO;
        natural[ini..ini + ANCHO].copy_from_slice(&pa);
        filas.push(Fila {
            linea: cap.linea,
            semi_con_obj: sa,
            semi_sin_obj: sb,
            camino_con_obj: ca,
            camino_sin_obj: cb,
            bldcnt,
            eva,
            evb,
            diff_ab: distintos(&pa, &pb),
            diff_ad: distintos(&pg, &pd),
            diff_ag: distintos(&pa, &pg),
            diff_bd: distintos(&pb, &pd),
        });
    }

    // Banda: las líneas que hoy tienen un sprite translúcido con OBJ activo.
    let banda: Vec<usize> = filas
        .iter()
        .filter(|f| f.semi_con_obj)
        .map(|f| f.linea)
        .collect();
    let suma = |clave: fn(&Fila) -> usize| -> usize {
        filas
            .iter()
            .filter(|f| f.semi_con_obj)
            .map(clave)
            .sum()
    };
    let banda_natural = suma(|f| f.diff_ab);
    let banda_mismo_camino = suma(|f| f.diff_ad);
    let banda_cambio_camino = suma(|f| f.diff_ag);

    let sin_semi: Vec<&Fila> = filas.iter().filter(|f| !f.semi_con_obj).collect();
    let identicas_sin_semi = sin_semi
        .iter()
        .filter(|f| f.diff_ag == 0 && f.diff_bd == 0)
        .count();
    let cambio_camino = filas
        .iter()
        .filter(|f| f.camino_con_obj != f.camino_sin_obj)
        .count();
    let semi_con = filas.iter().filter(|f| f.semi_con_obj).count();
    let semi_sin = filas.iter().filter(|f| f.semi_sin_obj).count();

    // Coste por camino en las líneas de la banda, con OBJ apagado para aislar la composición.
    let mut ns_tramos = Vec::new();
    let mut ns_general = Vec::new();
    for &l in &banda {
        let cap = &capturas[l];
        let sin_obj = u16::from_le_bytes([cap.io[0], cap.io[1]]) & !BIT_OBJ;
        gba_core::ppu::forzar_alpha_tramos(false);
        for _ in 0..MUESTRAS {
            gba_core::ppu::forzar_camino_general(false);
            dibujar(&mut replay, cap, sin_obj);
            ns_tramos.push(replay.bus.perf_counters.compose_ns);
        }
        for _ in 0..MUESTRAS {
            gba_core::ppu::forzar_camino_general(true);
            dibujar(&mut replay, cap, sin_obj);
            ns_general.push(replay.bus.perf_counters.compose_ns);
        }
        gba_core::ppu::forzar_camino_general(false);
    }
    let med_tramos = mediana(&mut ns_tramos);
    let med_general = mediana(&mut ns_general);
    // Regla fijada en la propuesta antes de medir: general ≤ 1,25 × tramos.
    let decision = if med_general * 4 <= med_tramos * 5 {
        "general"
    } else {
        "tramos"
    };

    let control = distintos(&natural, &fb_juego);
    let cap_banda = banda.first().map(|&l| &capturas[l]);
    let dispcnt_banda = cap_banda
        .map(|c| u16::from_le_bytes([c.io[0], c.io[1]]))
        .unwrap_or(0);
    let resumen = format!(
        "resumen cuadro {objetivo}: control A vs juego = {control} píxeles distintos de {} (esperado 0)\n\
         lineas con sprite semi: con OBJ {semi_con}, sin OBJ {semi_sin}\n\
         lineas donde cambia el camino al quitar OBJ: {cambio_camino}\n\
         banda lineas {primera} {ultima}: aportacion natural {banda_natural}, mismo camino general {banda_mismo_camino}, cambio de camino {banda_cambio_camino}\n\
         lineas sin semi identicas al forzar el general: {identicas_sin_semi} de {sin_semi}\n\
         banda desglose de píxeles: alpha_blend(BG2, negro) {blend_negro}, alpha_blend(negro, BG2) {blend_sprite_top}, negro puro {negros}, sin explicar {sin_explicar} (BLDCNT {bldcnt_muestra:#06x}, BLDALPHA {bldalpha_muestra:#06x})\n\
         dispcnt de la banda {dispcnt_banda:#06x}: objwin {objwin}, win0 {win0}, win1 {win1}\n\
         coste compose_ns mediana: tramos {med_tramos}, general {med_general}\n\
         decision: {decision}",
        ANCHO * ALTO,
        primera = banda.first().copied().unwrap_or(0),
        ultima = banda.last().copied().unwrap_or(0),
        sin_semi = sin_semi.len(),
        objwin = (dispcnt_banda >> 15) & 1,
        win0 = (dispcnt_banda >> 13) & 1,
        win1 = (dispcnt_banda >> 14) & 1,
    );
    println!("{resumen}");
    for f in &filas {
        println!("{}", f.texto());
    }

    let _ = std::fs::create_dir_all("volcados");
    let ruta = format!("volcados/subcamino-cuadro-{objetivo}.txt");
    let cuerpo: Vec<String> = filas.iter().map(Fila::texto).collect();
    let contenido = format!("{resumen}\n{}\n", cuerpo.join("\n"));
    if let Err(e) = std::fs::write(&ruta, contenido) {
        eprintln!("no se pudo escribir {ruta}: {e}");
    }
}
