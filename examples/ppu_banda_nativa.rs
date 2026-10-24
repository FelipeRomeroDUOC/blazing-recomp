//! Diagnóstico: detalle de las diferencias de la PPU nativa en una banda de líneas.
//!
//! Reproduce el cuadro N con el estado de fin de cuadro, dibuja el cuadro con
//! la PPU nativa (GPU) y con la PPU por software de referencia, e imprime las
//! rachas de píxeles que difieren en las líneas indicadas, con el color de cada
//! lado. Añade los sprites de OAM que tocan cada línea, con su modo, prioridad
//! y caja, para saber si la diferencia viene de los sprites.
//!
//! Uso:
//!   cargo run --release --features proto --example ppu_banda_nativa -- <ruta.gba> <secuencia.txt> <cuadro> [linea_ini] [linea_fin]

/// Tamaños de los sprites: `[forma][tamaño]`.

#[path = "comun/mod.rs"]
mod comun;

use comun::color::{canal_8bit, cerca_pixel};
use comun::entrada::leer_secuencia;
use comun::gpu::{ALTO, ANCHO, Gpu};

const OBJ_TAMANOS: [[(i32, i32); 4]; 3] = [
    [(8, 8), (16, 16), (32, 32), (64, 64)],
    [(16, 8), (32, 8), (32, 16), (64, 32)],
    [(8, 16), (8, 32), (16, 32), (32, 64)],
];

fn rgb8(c: u16) -> [u8; 3] {
    [canal_8bit(c), canal_8bit(c >> 5), canal_8bit(c >> 10)]
}

/// Sprites de OAM que tocan la línea dada.
fn sprites_en_linea(oam: &[u8], linea: usize) -> Vec<String> {
    let mut salida = Vec::new();
    for i in 0..128 {
        let a0 = u16::from_le_bytes([oam[i * 8], oam[i * 8 + 1]]);
        let a1 = u16::from_le_bytes([oam[i * 8 + 2], oam[i * 8 + 3]]);
        let a2 = u16::from_le_bytes([oam[i * 8 + 4], oam[i * 8 + 5]]);
        let affine = a0 & 0x100 != 0;
        if !affine && a0 & 0x200 != 0 {
            continue;
        }
        let forma = ((a0 >> 14) & 3) as usize;
        if forma == 3 {
            continue;
        }
        let talla = ((a1 >> 14) & 3) as usize;
        let (w, h) = OBJ_TAMANOS[forma][talla];
        let (bw, bh) = if affine && a0 & 0x200 != 0 {
            (w * 2, h * 2)
        } else {
            (w, h)
        };
        let mut y = (a0 & 0xFF) as i32;
        if y >= 160 {
            y -= 256;
        }
        let mut x = (a1 & 0x1FF) as i32;
        if x >= 240 {
            x -= 512;
        }
        if (linea as i32) < y || (linea as i32) >= y + bh {
            continue;
        }
        let modo = (a0 >> 10) & 3;
        let prioridad = (a2 >> 10) & 3;
        let semi = modo == 1;
        let ventana = modo == 2;
        let ocho = a0 & 0x2000 != 0;
        salida.push(format!(
            "oam[{i:3}] x={x:4} y={y:4} caja={bw}x{bh} modo={modo}{}{} prio={prioridad} 8bpp={ocho}",
            if semi { " (semitransparente)" } else { "" },
            if ventana { " (ventana)" } else { "" },
        ));
    }
    salida
}

/// Máscara de la ventana OBJ según la regla de la referencia: los píxeles no
/// transparentes de los sprites cuyo modo es 2 (ventana).
fn mascara_referencia(oam: &[u8], vram: &[u8], dispcnt: u16) -> Vec<bool> {
    let mut mascara = vec![false; ANCHO * ALTO];
    let one_dim = dispcnt & 0x40 != 0;
    for i in 0..128 {
        let a0 = u16::from_le_bytes([oam[i * 8], oam[i * 8 + 1]]);
        let a1 = u16::from_le_bytes([oam[i * 8 + 2], oam[i * 8 + 3]]);
        let a2 = u16::from_le_bytes([oam[i * 8 + 4], oam[i * 8 + 5]]);
        let affine = a0 & 0x100 != 0;
        if !affine && a0 & 0x200 != 0 {
            continue;
        }
        let forma = ((a0 >> 14) & 3) as usize;
        if forma == 3 {
            continue;
        }
        let modo = (a0 >> 10) & 3;
        if modo != 2 {
            continue;
        }
        let talla = ((a1 >> 14) & 3) as usize;
        let (w, h) = (
            OBJ_TAMANOS[forma][talla].0 as i32,
            OBJ_TAMANOS[forma][talla].1 as i32,
        );
        let (bw, bh) = if affine && a0 & 0x200 != 0 {
            (w * 2, h * 2)
        } else {
            (w, h)
        };
        let mut y = (a0 & 0xFF) as i32;
        if y >= 160 {
            y -= 256;
        }
        let mut x = (a1 & 0x1FF) as i32;
        if x >= 240 {
            x -= 512;
        }
        let ocho = a0 & 0x2000 != 0;
        let base = (a2 & 0x3FF) as usize;
        let fila_tiles = if one_dim {
            (w / 8) * if ocho { 2 } else { 1 }
        } else {
            32
        };
        let fila_tiles = fila_tiles as usize;
        let paso = if ocho { 2 } else { 1 };
        for ly in 0..bh {
            let linea = y + ly;
            if linea < 0 || linea >= ALTO as i32 {
                continue;
            }
            for bx in 0..bw {
                let sx = x + bx;
                if sx < 0 || sx >= ANCHO as i32 {
                    continue;
                }
                let (u, v) = if affine {
                    let dx = bx - bw / 2;
                    let dy = ly - bh / 2;
                    (
                        ((0x100 * dx) >> 8) + w / 2,
                        ((0x100 * dy) >> 8) + h / 2,
                    )
                } else {
                    let mut u = bx;
                    if a1 & 0x1000 != 0 {
                        u = w - 1 - u;
                    }
                    let mut v = ly;
                    if a1 & 0x2000 != 0 {
                        v = h - 1 - v;
                    }
                    (u, v)
                };
                if u < 0 || v < 0 || u >= w || v >= h {
                    continue;
                }
                let (tu, pu) = (u / 8, u % 8);
                let (tv, pv) = (v / 8, v % 8);
                let (tu, pu, tv, pv) = (tu as usize, pu as usize, tv as usize, pv as usize);
                let tile = base + tv * fila_tiles + tu * paso;
                let addr = if ocho {
                    0x1_0000 + (tile / 2) * 64 + pv * 8 + pu
                } else {
                    0x1_0000 + tile * 32 + pv * 4 + pu / 2
                };
                if addr >= 0x1_8000 {
                    continue;
                }
                let color_idx = if ocho {
                    vram[addr] as usize
                } else {
                    let b = vram[addr];
                    let nib = if pu % 2 == 0 { b & 0xF } else { (b >> 4) & 0xF } as usize;
                    nib
                };
                if color_idx == 0 {
                    continue;
                }
                mascara[linea as usize * ANCHO + sx as usize] = true;
            }
        }
    }
    mascara
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 4 {
        eprintln!("uso: ppu_banda_nativa <ruta.gba> <secuencia.txt> <cuadro> [linea_ini] [linea_fin]");
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
    let ini: usize = args.get(4).and_then(|s| s.parse().ok()).unwrap_or(60);
    let fin: usize = args.get(5).and_then(|s| s.parse().ok()).unwrap_or(96);

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
            m.run_frame(5_000_000);
            if cuadro == objetivo {
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
    let vram_fin = m.bus.vram.clone();
    let oam_fin = m.bus.oam.clone();
    let pal_fin = m.bus.palette.clone();
    let io_fin = m.bus.io.clone();
    let n = ANCHO * ALTO;

    // Referencia: la PPU por software con el estado de fin de cuadro.
    let mut ppu_fin = vec![0u16; n];
    {
        let mut replay = gba_core::Machine::new(rom.clone());
        replay.bus.palette = pal_fin.clone();
        replay.bus.vram = vram_fin.clone();
        replay.bus.oam = oam_fin.clone();
        replay.bus.io = io_fin.clone();
        for l in 0..ALTO {
            gba_core::ppu::render_scanline(&mut replay.bus, l);
            let ini = l * ANCHO;
            ppu_fin[ini..ini + ANCHO]
                .copy_from_slice(&replay.bus.framebuffer[ini..ini + ANCHO]);
        }
    }

    let mut gpu = match Gpu::nueva_nativa() {
        Ok(g) => g,
        Err(e) => {
            eprintln!("{e}");
            std::process::exit(1);
        }
    };
    let nativa = gpu.renderiza(&vram_fin, &oam_fin, &pal_fin, &io_fin);

    // Experimento decisivo: ¿la nativa usa alguna vez la ventana OBJ?
    // Se vuelve a dibujar el mismo cuadro con el bit 15 de DISPCNT apagado y
    // se comparan las dos imágenes nativas. Si no cambia nada, la ventana OBJ
    // no llega a la composición.
    let mut io_sin_ventana = io_fin.clone();
    io_sin_ventana[0] &= !0x80;
    io_sin_ventana[1] &= !0x80;
    let nativa_sin_ventana = gpu.renderiza(&vram_fin, &oam_fin, &pal_fin, &io_sin_ventana);
    let dif_ventana = (0..n)
        .filter(|&p| {
            let a = [
                nativa[p * 3],
                nativa[p * 3 + 1],
                nativa[p * 3 + 2],
            ];
            let b = [
                nativa_sin_ventana[p * 3],
                nativa_sin_ventana[p * 3 + 1],
                nativa_sin_ventana[p * 3 + 2],
            ];
            a != b
        })
        .count();
    println!("nativa con ventana OBJ frente a nativa sin ventana OBJ: {dif_ventana} píxeles distintos");

    let mut ppu_sin_ventana = vec![0u16; n];
    {
        let mut replay = gba_core::Machine::new(rom.clone());
        replay.bus.palette = pal_fin.clone();
        replay.bus.vram = vram_fin.clone();
        replay.bus.oam = oam_fin.clone();
        replay.bus.io = io_sin_ventana.clone();
        for l in 0..ALTO {
            gba_core::ppu::render_scanline(&mut replay.bus, l);
            let ini = l * ANCHO;
            ppu_sin_ventana[ini..ini + ANCHO]
                .copy_from_slice(&replay.bus.framebuffer[ini..ini + ANCHO]);
        }
    }
    let dif_sin_ventana = (0..n)
        .filter(|&p| {
            let nat = [
                nativa_sin_ventana[p * 3],
                nativa_sin_ventana[p * 3 + 1],
                nativa_sin_ventana[p * 3 + 2],
            ];
            !cerca_pixel(nat, rgb8(ppu_sin_ventana[p]))
        })
        .count();
    println!(
        "nativa sin ventana OBJ frente a la referencia sin ventana OBJ: {dif_sin_ventana} píxeles distintos"
    );

    // Referencia con la máscara de la ventana neutra: WINOUT con los mismos
    // bits dentro y fuera (0x3E) hace que la máscara no importe y toda la
    // imagen use el control de fuera de ventana.
    let mut io_neutra = io_fin.clone();
    io_neutra[0x4A] = 0x3E;
    io_neutra[0x4B] = 0x3E;
    let mut ppu_neutra = vec![0u16; n];
    {
        let mut replay = gba_core::Machine::new(rom.clone());
        replay.bus.palette = pal_fin.clone();
        replay.bus.vram = vram_fin.clone();
        replay.bus.oam = oam_fin.clone();
        replay.bus.io = io_neutra.clone();
        for l in 0..ALTO {
            gba_core::ppu::render_scanline(&mut replay.bus, l);
            let ini = l * ANCHO;
            ppu_neutra[ini..ini + ANCHO]
                .copy_from_slice(&replay.bus.framebuffer[ini..ini + ANCHO]);
        }
    }

    // Máscara de la ventana OBJ según la referencia: los píxeles no
    // transparentes de los sprites en modo ventana.
    let mascara_ref =
        mascara_referencia(&oam_fin, &vram_fin, u16::from_le_bytes([io_fin[0], io_fin[1]]));

    // Máscara efectiva de la nativa: en los píxeles donde la ventana cambia el
    // resultado, se deduce comparando la imagen nativa con las dos referencias.
    let mut dif_mascaras = 0usize;
    let mut con_info = 0usize;
    let mut ejemplos: Vec<String> = Vec::new();
    for p in 0..n {
        let rn = rgb8(ppu_neutra[p]);
        let rm = rgb8(ppu_fin[p]);
        if cerca_pixel(rn, rm) {
            continue; // la ventana no cambia el resultado en este píxel
        }
        con_info += 1;
        let nat = [
            nativa[p * 3],
            nativa[p * 3 + 1],
            nativa[p * 3 + 2],
        ];
        let nativa_ventana = cerca_pixel(nat, rm);
        if nativa_ventana != mascara_ref[p] {
            dif_mascaras += 1;
            if ejemplos.len() < 12 {
                ejemplos.push(format!(
                    "x={:3} y={:3} referencia_ventana={} nativa_ventana={}",
                    p % ANCHO,
                    p / ANCHO,
                    mascara_ref[p],
                    nativa_ventana
                ));
            }
        }
    }
    println!(
        "píxeles donde la ventana cambia el resultado: {con_info}; de ellos, con máscara distinta: {dif_mascaras}"
    );
    let pixeles_ref: usize = mascara_ref.iter().filter(|&&v| v).count();
    println!("máscara de la referencia: {pixeles_ref} píxeles");
    for e in ejemplos {
        println!("    {e}");
    }


    // Registros de ventana del cuadro.
    {
        let rd = |off: usize| u16::from_le_bytes([io_fin[off], io_fin[off + 1]]);
        println!(
            "DISPCNT=0x{:04X} WININ=0x{:04X} WINOUT=0x{:04X} BLDCNT=0x{:04X} BLDALPHA=0x{:04X} BLDY=0x{:04X}",
            rd(0x00), rd(0x48), rd(0x4A), rd(0x50), rd(0x52), rd(0x54)
        );
        println!(
            "  win0_on={} win1_on={} objwin_on={} WIN0H=0x{:04X} WIN1H=0x{:04X} WIN0V=0x{:04X} WIN1V=0x{:04X}",
            rd(0x00) & 0x2000 != 0,
            rd(0x00) & 0x4000 != 0,
            rd(0x00) & 0x8000 != 0,
            rd(0x40), rd(0x42), rd(0x44), rd(0x46)
        );
    }


    // Experimento de aislamiento de la máscara: WINOUT = 0x3E00 deja el
    // control de fuera de ventana a 0 (nada visible) y el de dentro de la
    // ventana OBJ a 0x3E (todo visible). Así, solo los píxeles dentro de la
    // máscara conservan contenido, y la diferencia nativa-referencia delata
    // las máscaras distintas.
    let mut io_aisla = io_fin.clone();
    io_aisla[0x4A] = 0x00;
    io_aisla[0x4B] = 0x3E;
    let nativa_aisla = gpu.renderiza(&vram_fin, &oam_fin, &pal_fin, &io_aisla);
    let mut ppu_aisla = vec![0u16; n];
    {
        let mut replay = gba_core::Machine::new(rom.clone());
        replay.bus.palette = pal_fin.clone();
        replay.bus.vram = vram_fin.clone();
        replay.bus.oam = oam_fin.clone();
        replay.bus.io = io_aisla.clone();
        for l in 0..ALTO {
            gba_core::ppu::render_scanline(&mut replay.bus, l);
            let ini = l * ANCHO;
            ppu_aisla[ini..ini + ANCHO]
                .copy_from_slice(&replay.bus.framebuffer[ini..ini + ANCHO]);
        }
    }
    let mut dif_aisla = 0usize;
    let mut filas_afectadas: Vec<(usize, usize, usize, usize)> = Vec::new();
    for l in 0..ALTO {
        let mut ini_f = None;
        let mut fin_f = 0usize;
        let mut cuenta = 0usize;
        for x in 0..ANCHO {
            let p = l * ANCHO + x;
            let nat = [
                nativa_aisla[p * 3],
                nativa_aisla[p * 3 + 1],
                nativa_aisla[p * 3 + 2],
            ];
            if !cerca_pixel(nat, rgb8(ppu_aisla[p])) {
                cuenta += 1;
                dif_aisla += 1;
                if ini_f.is_none() {
                    ini_f = Some(x);
                }
                fin_f = x + 1;
            }
        }
        if cuenta > 0 {
            filas_afectadas.push((l, ini_f.unwrap(), fin_f, cuenta));
        }
    }
    println!("aislamiento de máscara: {dif_aisla} píxeles con contenido distinto");
    // La imagen nativa de aislamiento solo puede tener contenido DENTRO de la
    // máscara de la ventana OBJ: fuera, el control de ventana es 0x00 y no se
    // ve nada. Contar los píxeles no negros dice si la máscara de la GPU está
    // vacía o no.
    {
        let mut no_negros = 0usize;
        let mut filas: Vec<(usize, usize)> = Vec::new();
        for l in 0..ALTO {
            let mut cuenta = 0usize;
            for x in 0..ANCHO {
                let p = l * ANCHO + x;
                let o = p * 3;
                if nativa_aisla[o] != 0 || nativa_aisla[o + 1] != 0 || nativa_aisla[o + 2] != 0 {
                    cuenta += 1;
                }
            }
            if cuenta > 0 {
                filas.push((l, cuenta));
                no_negros += cuenta;
            }
        }
        println!("nativa de aislamiento: {no_negros} píxeles con contenido (la máscara de la GPU tiene ese tamaño)");
        for (l, c) in filas.iter().take(6) {
            println!("    línea {l}: {c} píxeles");
        }
    }
    for (l, a, b, c) in filas_afectadas.iter().take(24) {
        println!("    línea {l:3}: x={a:3}..{b:3} ({c} píxeles)");
    }

    // Atribución de capas: se vuelve a renderizar la referencia con cada capa
    // quitada y se mira cómo cambia el color de un píxel.
    let muestras: [(usize, usize); 6] = [
        (66, 16),
        (67, 30),
        (70, 15),
        (70, 95),
        (80, 210),
        (85, 40),
    ];
    let variantes: [(&str, u16); 5] = [
        ("sin OBJ", 0x1000),
        ("sin BG0", 0x0100),
        ("sin BG1", 0x0200),
        ("sin BG2", 0x0400),
        ("sin BG3", 0x0800),
    ];
    let mut refs: Vec<Vec<u16>> = Vec::new();
    for (nombre, mascara) in [("completa", 0u16)].into_iter().chain(variantes) {
        let mut io_v = io_fin.clone();
        io_v[0] &= !(mascara & 0xFF) as u8;
        io_v[1] &= !((mascara >> 8) & 0xFF) as u8;
        let mut img = vec![0u16; n];
        let mut replay = gba_core::Machine::new(rom.clone());
        replay.bus.palette = pal_fin.clone();
        replay.bus.vram = vram_fin.clone();
        replay.bus.oam = oam_fin.clone();
        replay.bus.io = io_v;
        for l in 0..ALTO {
            gba_core::ppu::render_scanline(&mut replay.bus, l);
            let ini = l * ANCHO;
            img[ini..ini + ANCHO].copy_from_slice(&replay.bus.framebuffer[ini..ini + ANCHO]);
        }
        refs.push(img);
        let _ = nombre;
    }
    // Atribución de capas en la NATIVA: se vuelve a renderizar con cada capa
    // desactivada y se mira cómo cambia el color del píxel. Es la misma
    // técnica que la de la referencia, pero sobre la GPU.
    let variantes_dispcnt: [(&str, u16); 5] = [
        ("sin OBJ", 0x1000),
        ("sin BG0", 0x0100),
        ("sin BG1", 0x0200),
        ("sin BG2", 0x0400),
        ("sin BG3", 0x0800),
    ];
    let mut nats: Vec<Vec<u8>> = Vec::new();
    for (_nombre, mascara) in [("completa", 0u16)]
        .into_iter()
        .chain(variantes_dispcnt)
    {
        let mut io_v = io_fin.clone();
        io_v[0] &= !(mascara & 0xFF) as u8;
        io_v[1] &= !((mascara >> 8) & 0xFF) as u8;
        nats.push(gpu.renderiza(&vram_fin, &oam_fin, &pal_fin, &io_v));
    }
    println!("atribución de capas (nativa):");
    for (y, x) in muestras {
        let p = y * ANCHO + x;
        let nat = [nats[0][p * 3], nats[0][p * 3 + 1], nats[0][p * 3 + 2]];
        let mut linea = format!(
            "  y={y:3} x={x:3} referencia={:02X?} nativa={nat:02X?}",
            rgb8(refs[0][p])
        );
        for (i, (nombre, _)) in variantes_dispcnt.iter().enumerate() {
            let v = [
                nats[i + 1][p * 3],
                nats[i + 1][p * 3 + 1],
                nats[i + 1][p * 3 + 2],
            ];
            if v != nat {
                linea.push_str(&format!(" | {nombre}={v:02X?}"));
            }
        }
        println!("{linea}");
    }

    // Comparación por capa: con WINOUT = (control << 8) | 0x00 solo se ve la
    // capa indicada dentro de la máscara de la ventana OBJ. La diferencia
    // entre la nativa y la referencia aísla los fallos de cada capa.
    // ¿La ventana OBJ funciona en la referencia cuando la mezcla está apagada?
    // Predicción: con BLDCNT = 0 el camino rápido de alpha no se toma y la
    // máscara sí se aplica; con BLDCNT = 0x3F40 (modo alpha) el camino rápido
    // la ignora.
    // Decisivo: la nativa frente a la referencia con el CAMINO GENERAL
    // forzado (el camino del upstream fijado, píxel a píxel). Si coinciden,
    // la nativa está bien y el error está en el camino rápido del motor
    // hermano, que es el que ignora la máscara de la ventana OBJ.
    {
        gba_core::ppu::forzar_camino_general(true);
        let img_nat = gpu.renderiza(&vram_fin, &oam_fin, &pal_fin, &io_fin);
        let mut img_ref = vec![0u16; n];
        {
            let mut replay = gba_core::Machine::new(rom.clone());
            replay.bus.palette = pal_fin.clone();
            replay.bus.vram = vram_fin.clone();
            replay.bus.oam = oam_fin.clone();
            replay.bus.io = io_fin.clone();
            for l in 0..ALTO {
                gba_core::ppu::render_scanline(&mut replay.bus, l);
                let ini = l * ANCHO;
                img_ref[ini..ini + ANCHO]
                    .copy_from_slice(&replay.bus.framebuffer[ini..ini + ANCHO]);
            }
        }
        gba_core::ppu::forzar_camino_general(false);
        let mut dif = 0usize;
        let mut ejemplos: Vec<String> = Vec::new();
        for p in 0..n {
            let nat = [img_nat[p * 3], img_nat[p * 3 + 1], img_nat[p * 3 + 2]];
            let r = rgb8(img_ref[p]);
            if !cerca_pixel(nat, r) {
                dif += 1;
                if ejemplos.len() < 8 {
                    ejemplos.push(format!(
                        "x={:3} y={:3} ref={r:02X?} nat={nat:02X?}",
                        p % ANCHO,
                        p / ANCHO
                    ));
                }
            }
        }
        println!("nativa frente a la referencia con el camino general forzado: {dif} píxeles distintos de {n}");
        for e in ejemplos {
            println!("    {e}");
        }
    }

    println!("efecto de la ventana OBJ según BLDCNT (referencia y nativa):");
    for (nombre, bldcnt) in [("BLDCNT=0x0000", 0x0000u16), ("BLDCNT=0x3F40", 0x3F40)] {
        for (etiqueta, control) in [("dentro=0x00", 0x00u16), ("dentro=0x1F", 0x1F)] {
            let mut io_v = io_fin.clone();
            io_v[0x50] = (bldcnt & 0xFF) as u8;
            io_v[0x51] = (bldcnt >> 8) as u8;
            io_v[0x4A] = 0x00;
            io_v[0x4B] = control as u8;
            let img_nat = gpu.renderiza(&vram_fin, &oam_fin, &pal_fin, &io_v);
            let mut img_ref = vec![0u16; n];
            {
                let mut replay = gba_core::Machine::new(rom.clone());
                replay.bus.palette = pal_fin.clone();
                replay.bus.vram = vram_fin.clone();
                replay.bus.oam = oam_fin.clone();
                replay.bus.io = io_v.clone();
                for l in 0..ALTO {
                    gba_core::ppu::render_scanline(&mut replay.bus, l);
                    let ini = l * ANCHO;
                    img_ref[ini..ini + ANCHO]
                        .copy_from_slice(&replay.bus.framebuffer[ini..ini + ANCHO]);
                }
            }
            let mut no_negros_ref = 0usize;
            let mut no_negros_nat = 0usize;
            for p in 0..n {
                if img_ref[p] != 0 {
                    no_negros_ref += 1;
                }
                if img_nat[p * 3] != 0 || img_nat[p * 3 + 1] != 0 || img_nat[p * 3 + 2] != 0 {
                    no_negros_nat += 1;
                }
            }
            println!(
                "  {nombre} {etiqueta}: referencia {no_negros_ref} píxeles con contenido, nativa {no_negros_nat}"
            );
        }
    }

    println!("comparación por capa (solo dentro de la máscara OBJ):");
    for (nombre, control) in [
        ("BG0", 0x01u16),
        ("BG1", 0x02),
        ("BG2", 0x04),
        ("BG3", 0x08),
        ("OBJ", 0x10),
        ("todas", 0x1F),
    ] {
        let mut io_capa = io_fin.clone();
        io_capa[0x4A] = 0x00;
        io_capa[0x4B] = control as u8;
        // Con BLDCNT = 0 la referencia no toma el camino rápido de alpha, así
        // que su ventana OBJ sí se aplica y la comparación capa a capa es
        // válida.
        io_capa[0x50] = 0x00;
        io_capa[0x51] = 0x00;
        let img_nat = gpu.renderiza(&vram_fin, &oam_fin, &pal_fin, &io_capa);
        let mut img_ref = vec![0u16; n];
        {
            let mut replay = gba_core::Machine::new(rom.clone());
            replay.bus.palette = pal_fin.clone();
            replay.bus.vram = vram_fin.clone();
            replay.bus.oam = oam_fin.clone();
            replay.bus.io = io_capa.clone();
            for l in 0..ALTO {
                gba_core::ppu::render_scanline(&mut replay.bus, l);
                let ini = l * ANCHO;
                img_ref[ini..ini + ANCHO]
                    .copy_from_slice(&replay.bus.framebuffer[ini..ini + ANCHO]);
            }
        }
        let mut dif = 0usize;
        let mut ejemplos: Vec<String> = Vec::new();
        for p in 0..n {
            let nat = [img_nat[p * 3], img_nat[p * 3 + 1], img_nat[p * 3 + 2]];
            let r = rgb8(img_ref[p]);
            if !cerca_pixel(nat, r) {
                dif += 1;
                if ejemplos.len() < 5 {
                    ejemplos.push(format!(
                        "x={:3} y={:3} ref={r:02X?} nat={nat:02X?}",
                        p % ANCHO,
                        p / ANCHO
                    ));
                }
            }
        }
        println!("  {nombre:5}: {dif} píxeles distintos  {}", ejemplos.join(" | "));
    }

    println!("atribución de capas (referencia):");
    for (y, x) in muestras {
        let p = y * ANCHO + x;
        let nat = [nativa[p * 3], nativa[p * 3 + 1], nativa[p * 3 + 2]];
        let mut linea = format!("  y={y:3} x={x:3} referencia={:04X} nativa={:02X?}", refs[0][p], nat);
        for (i, (nombre, _)) in variantes.iter().enumerate() {
            if refs[i + 1][p] != refs[0][p] {
                linea.push_str(&format!(" | {nombre}={:04X}", refs[i + 1][p]));
            }
        }
        linea.push_str(&format!(" | ventana_ref={}", mascara_ref[p]));
        println!("{linea}");
    }

    for l in ini..fin.min(ALTO) {
        let mut racha_ini: Option<usize> = None;
        let mut total = 0usize;
        let mut lineas_racha = Vec::new();
        for x in 0..ANCHO {
            let p = l * ANCHO + x;
            let nat = [nativa[p * 3], nativa[p * 3 + 1], nativa[p * 3 + 2]];
            let refc = rgb8(ppu_fin[p]);
            if !cerca_pixel(nat, refc) {
                total += 1;
                if racha_ini.is_none() {
                    racha_ini = Some(x);
                }
            } else if let Some(a) = racha_ini {
                lineas_racha.push((a, x));
                racha_ini = None;
            }
        }
        if let Some(a) = racha_ini {
            lineas_racha.push((a, ANCHO));
        }
        if total == 0 {
            continue;
        }
        println!("linea {l:3}: {total} píxeles distintos, {lineas_racha:?}");
        for (a, b) in lineas_racha.iter().take(4) {
            let x = a + (b - a) / 2;
            let p = l * ANCHO + x;
            let nat = [nativa[p * 3], nativa[p * 3 + 1], nativa[p * 3 + 2]];
            let refc = rgb8(ppu_fin[p]);
            println!(
                "    x={x:3}..{b:3} referencia={:04X} (rgb {refc:?}) nativa={:02X?} (rgb {nat:?})",
                ppu_fin[p], nat
            );
        }
        for s in sprites_en_linea(&oam_fin, l) {
            println!("    sprite {s}");
        }
    }
}
