//! Módulo de volcado de fotogramas y diagnóstico causal en Android.
//!
//! Permite capturar y volcar en segundo plano imágenes compuestas de 960x160
//! (Software fin, Software línea, Nativa GPU y Diferencias en magenta) y
//! reportes textuales con el estado I/O de la PPU para depurar discrepancias
//! visuales y parpadeos sin degradar la tasa de 60 FPS del juego.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;

pub const NOMBRE_BANDERA_VOLCADO: &str = "dump-frames.flag";

pub const ANCHO_GBA: usize = 240;
pub const ALTO_GBA: usize = 160;
pub const PANELES: usize = 4;
pub const ANCHO_COMPUESTO: usize = ANCHO_GBA * PANELES; // 960

/// Resuelve la ruta del directorio de volcados priorizando la carpeta Descargas
/// accesible por la persona usuaria en Android.
pub fn resolver_directorio_volcados(search_dirs: &[PathBuf]) -> PathBuf {
    let prioridad = [
        (
            PathBuf::from("/sdcard/Download"),
            PathBuf::from("/sdcard/Download/blazing-recomp/volcados"),
        ),
        (
            PathBuf::from("/storage/emulated/0/Download"),
            PathBuf::from("/storage/emulated/0/Download/blazing-recomp/volcados"),
        ),
        (
            PathBuf::from("Download"),
            PathBuf::from("Download/blazing-recomp/volcados"),
        ),
    ];
    for (base, dest) in &prioridad {
        if dest.exists() || base.exists() {
            return dest.clone();
        }
    }
    for dir in search_dirs {
        if dir.exists() {
            return dir.join("blazing-recomp").join("volcados");
        }
    }
    prioridad[0].1.clone()
}

/// Comprueba si existe la bandera `dump-frames.flag` en alguno de los directorios candidatos.
pub fn volcado_activado_por_bandera(search_dirs: &[PathBuf]) -> bool {
    let mut candidatos = crate::perf::resolve_perf_candidates(search_dirs);
    let subrutas = [
        PathBuf::from("/sdcard/Download/blazing-recomp"),
        PathBuf::from("/storage/emulated/0/Download/blazing-recomp"),
        PathBuf::from("Download/blazing-recomp"),
    ];
    for s in subrutas {
        if !candidatos.contains(&s) {
            candidatos.push(s);
        }
    }

    for dir in &candidatos {
        if dir.join(NOMBRE_BANDERA_VOLCADO).exists() {
            return true;
        }
        if dir
            .join("blazing-recomp")
            .join(NOMBRE_BANDERA_VOLCADO)
            .exists()
        {
            return true;
        }
    }
    false
}

/// Alias canónico para consultar si el volcado está activado por bandera en disco.
#[inline]
pub fn volcado_activado(search_dirs: &[PathBuf]) -> bool {
    volcado_activado_por_bandera(search_dirs)
}

fn crc32(datos: &[u8]) -> u32 {
    let mut crc = 0xFFFF_FFFFu32;
    for &b in datos {
        crc ^= b as u32;
        for _ in 0..8 {
            crc = if crc & 1 != 0 {
                (crc >> 1) ^ 0xEDB8_8320
            } else {
                crc >> 1
            };
        }
    }
    !crc
}

fn adler32(datos: &[u8]) -> u32 {
    let (mut a, mut b) = (1u32, 0u32);
    for &x in datos {
        a = (a + x as u32) % 65521;
        b = (b + a) % 65521;
    }
    (b << 16) | a
}

fn zlib_stored(datos: &[u8]) -> Vec<u8> {
    let mut out = vec![0x78, 0x01];
    let trozos: Vec<&[u8]> = if datos.is_empty() {
        vec![&[][..]]
    } else {
        datos.chunks(65535).collect()
    };
    let n = trozos.len();
    for (i, t) in trozos.into_iter().enumerate() {
        out.push(if i + 1 == n { 1 } else { 0 });
        let len = t.len() as u16;
        out.extend_from_slice(&len.to_le_bytes());
        out.extend_from_slice(&(!len).to_le_bytes());
        out.extend_from_slice(t);
    }
    out.extend_from_slice(&adler32(datos).to_be_bytes());
    out
}

/// Codificador PNG puro de 24 bits RGB sin dependencias externas.
pub fn png_rgb(rgb: &[u8], ancho: usize, alto: usize) -> Vec<u8> {
    let mut filas = Vec::with_capacity(alto * (ancho * 3 + 1));
    for y in 0..alto {
        filas.push(0); // filtro 0 (None)
        let ini = y * ancho * 3;
        let fin = (ini + ancho * 3).min(rgb.len());
        if ini < rgb.len() {
            filas.extend_from_slice(&rgb[ini..fin]);
            if fin - ini < ancho * 3 {
                filas.resize(filas.len() + (ancho * 3 - (fin - ini)), 0);
            }
        } else {
            filas.resize(filas.len() + ancho * 3, 0);
        }
    }
    let bloque = |tipo: &[u8], datos: &[u8]| {
        let mut b = (datos.len() as u32).to_be_bytes().to_vec();
        let mut cuerpo = tipo.to_vec();
        cuerpo.extend_from_slice(datos);
        b.extend_from_slice(&cuerpo);
        b.extend_from_slice(&crc32(&cuerpo).to_be_bytes());
        b
    };
    let mut ihdr = (ancho as u32).to_be_bytes().to_vec();
    ihdr.extend_from_slice(&(alto as u32).to_be_bytes());
    ihdr.extend_from_slice(&[8, 2, 0, 0, 0]);
    let mut png = b"\x89PNG\r\n\x1a\n".to_vec();
    png.extend(bloque(b"IHDR", &ihdr));
    png.extend(bloque(b"IDAT", &zlib_stored(&filas)));
    png.extend(bloque(b"IEND", &[]));
    png
}

/// Genera un lienzo compuesto de 960x160 en RGB8 organizado en 4 paneles:
/// 1. Software fin (240x160)
/// 2. Software línea (240x160)
/// 3. Nativa GPU (240x160)
/// 4. Diferencias resaltadas en magenta (240x160)
pub fn generar_lienzo_compuesto(sw_fin: &[u8], sw_linea: &[u8], nat: &[u8]) -> Vec<u8> {
    let mut out = vec![0u8; ANCHO_COMPUESTO * ALTO_GBA * 3];
    let fuente_fin = if sw_fin.len() >= ANCHO_GBA * ALTO_GBA * 3 {
        sw_fin
    } else {
        sw_linea
    };

    for y in 0..ALTO_GBA {
        let fila_base = y * ANCHO_COMPUESTO * 3;
        for x in 0..ANCHO_GBA {
            let p = y * ANCHO_GBA + x;
            let idx = p * 3;

            let r_fin = fuente_fin.get(idx).copied().unwrap_or(0);
            let g_fin = fuente_fin.get(idx + 1).copied().unwrap_or(0);
            let b_fin = fuente_fin.get(idx + 2).copied().unwrap_or(0);

            let r_sw = sw_linea.get(idx).copied().unwrap_or(0);
            let g_sw = sw_linea.get(idx + 1).copied().unwrap_or(0);
            let b_sw = sw_linea.get(idx + 2).copied().unwrap_or(0);

            let r_nat = nat.get(idx).copied().unwrap_or(0);
            let g_nat = nat.get(idx + 1).copied().unwrap_or(0);
            let b_nat = nat.get(idx + 2).copied().unwrap_or(0);

            let dif =
                r_sw.abs_diff(r_nat) > 3 || g_sw.abs_diff(g_nat) > 3 || b_sw.abs_diff(b_nat) > 3;

            let px_dif = if dif {
                [255, 0, 255] // Magenta de advertencia
            } else {
                [r_nat, g_nat, b_nat]
            };

            // Panel 0: Software fin
            let o0 = fila_base + (0 * ANCHO_GBA + x) * 3;
            out[o0..o0 + 3].copy_from_slice(&[r_fin, g_fin, b_fin]);

            // Panel 1: Software línea
            let o1 = fila_base + (1 * ANCHO_GBA + x) * 3;
            out[o1..o1 + 3].copy_from_slice(&[r_sw, g_sw, b_sw]);

            // Panel 2: Nativa GPU
            let o2 = fila_base + (2 * ANCHO_GBA + x) * 3;
            out[o2..o2 + 3].copy_from_slice(&[r_nat, g_nat, b_nat]);

            // Panel 3: Mapa de diferencias en magenta
            let o3 = fila_base + (3 * ANCHO_GBA + x) * 3;
            out[o3..o3 + 3].copy_from_slice(&px_dif);
        }
    }
    out
}

/// Genera el reporte textual de diagnóstico causal con desglose de diferencias y registros PPU.
pub fn generar_informe_diagnostico(
    cuadro: u64,
    sw_linea: &[u8],
    nat: &[u8],
    ppu_state: &crate::pipeline::PpuFrameState,
) -> String {
    let mut distintos = 0usize;
    let mut min_x = ANCHO_GBA;
    let mut min_y = ALTO_GBA;
    let mut max_x = 0usize;
    let mut max_y = 0usize;
    let mut primer_dif: Option<(usize, usize, [u8; 3], [u8; 3])> = None;
    let mut max_err_val = 0u8;
    let mut max_err_pos = (0usize, 0usize);

    for y in 0..ALTO_GBA {
        for x in 0..ANCHO_GBA {
            let p = y * ANCHO_GBA + x;
            let idx = p * 3;
            let r_sw = sw_linea.get(idx).copied().unwrap_or(0);
            let g_sw = sw_linea.get(idx + 1).copied().unwrap_or(0);
            let b_sw = sw_linea.get(idx + 2).copied().unwrap_or(0);

            let r_nat = nat.get(idx).copied().unwrap_or(0);
            let g_nat = nat.get(idx + 1).copied().unwrap_or(0);
            let b_nat = nat.get(idx + 2).copied().unwrap_or(0);

            let err = r_sw
                .abs_diff(r_nat)
                .max(g_sw.abs_diff(g_nat))
                .max(b_sw.abs_diff(b_nat));

            if err > 3 {
                distintos += 1;
                min_x = min_x.min(x);
                min_y = min_y.min(y);
                max_x = max_x.max(x);
                max_y = max_y.max(y);

                if primer_dif.is_none() {
                    primer_dif = Some((x, y, [r_sw, g_sw, b_sw], [r_nat, g_nat, b_nat]));
                }
                if err > max_err_val {
                    max_err_val = err;
                    max_err_pos = (x, y);
                }
            }
        }
    }

    let read_u16 = |offset: usize| -> u16 {
        if offset + 1 < ppu_state.io.len() {
            u16::from_le_bytes([ppu_state.io[offset], ppu_state.io[offset + 1]])
        } else {
            0
        }
    };

    let dispcnt = read_u16(0x00);
    let dispstat = read_u16(0x04);
    let bg0cnt = read_u16(0x08);
    let bg1cnt = read_u16(0x0A);
    let bg2cnt = read_u16(0x0C);
    let bg3cnt = read_u16(0x0E);
    let winin = read_u16(0x48);
    let winout = read_u16(0x4A);
    let bldcnt = read_u16(0x50);
    let bldalpha = read_u16(0x52);
    let bldy = read_u16(0x54);

    let mut informe = String::with_capacity(1024);
    informe.push_str(
        "================================================================================\n",
    );
    informe.push_str(&format!(
        "INFORME DE DIAGNÓSTICO DE VOLCADO - CUADRO {cuadro}\n"
    ));
    informe.push_str(
        "================================================================================\n",
    );
    let porcentaje = (distintos as f64 / (ANCHO_GBA * ALTO_GBA) as f64) * 100.0;
    informe.push_str(&format!(
        "Píxeles distintos (tolerancia > 3): {distintos} / 38400 ({porcentaje:.2}%)\n"
    ));

    if distintos > 0 {
        informe.push_str(&format!(
            "Caja delimitadora: ({min_x}, {min_y}) a ({max_x}, {max_y})\n"
        ));
        if let Some((px, py, sw_c, nat_c)) = primer_dif {
            informe.push_str(&format!(
                "Primer píxel distinto: ({px}, {py}) | SW: #{:02X}{:02X}{:02X} vs NAT: #{:02X}{:02X}{:02X}\n",
                sw_c[0], sw_c[1], sw_c[2], nat_c[0], nat_c[1], nat_c[2]
            ));
        }
        informe.push_str(&format!(
            "Máximo error: {max_err_val} niveles en ({}, {})\n",
            max_err_pos.0, max_err_pos.1
        ));
    } else {
        informe.push_str(
            "Caja delimitadora: ninguna (cuadro idéntico a referencia dentro de tolerancia)\n",
        );
    }

    informe.push_str("\nREGISTROS PRINCIPALES (FIN DE CUADRO):\n");
    informe.push_str(&format!(
        "  DISPCNT:  0x{dispcnt:04X} (Modo: {}, Capas BG: 0:{}, 1:{}, 2:{}, 3:{}, OBJ:{})\n",
        dispcnt & 0x07,
        (dispcnt >> 8) & 1,
        (dispcnt >> 9) & 1,
        (dispcnt >> 10) & 1,
        (dispcnt >> 11) & 1,
        (dispcnt >> 12) & 1,
    ));
    informe.push_str(&format!("  DISPSTAT: 0x{dispstat:04X}\n"));
    informe.push_str(&format!(
        "  BG0CNT:   0x{bg0cnt:04X} | BG1CNT: 0x{bg1cnt:04X}\n"
    ));
    informe.push_str(&format!(
        "  BG2CNT:   0x{bg2cnt:04X} | BG3CNT: 0x{bg3cnt:04X}\n"
    ));
    informe.push_str(&format!(
        "  WININ:    0x{winin:04X} | WINOUT: 0x{winout:04X}\n"
    ));
    informe.push_str(&format!(
        "  BLDCNT:   0x{bldcnt:04X} (Efecto: {}, 1st: {:02X}, 2nd: {:02X})\n",
        (bldcnt >> 6) & 0x03,
        bldcnt & 0x3F,
        (bldcnt >> 8) & 0x3F,
    ));
    informe.push_str(&format!(
        "  BLDALPHA: 0x{bldalpha:04X} (EVA: {}, EVB: {})\n",
        bldalpha & 0x1F,
        (bldalpha >> 8) & 0x1F
    ));
    informe.push_str(&format!(
        "  BLDY:     0x{bldy:04X} (EVY: {})\n",
        bldy & 0x1F
    ));

    let mut cambios_dispcnt = 0usize;
    let mut cambios_bldcnt = 0usize;
    let primer_linea_io = &ppu_state.io_lineas[0];
    let primer_dispcnt = u16::from_le_bytes([primer_linea_io[0], primer_linea_io[1]]);
    let primer_bldcnt = u16::from_le_bytes([primer_linea_io[0x50], primer_linea_io[0x51]]);

    for l in 1..ALTO_GBA {
        let l_io = &ppu_state.io_lineas[l];
        let d = u16::from_le_bytes([l_io[0], l_io[1]]);
        let b = u16::from_le_bytes([l_io[0x50], l_io[0x51]]);
        if d != primer_dispcnt {
            cambios_dispcnt += 1;
        }
        if b != primer_bldcnt {
            cambios_bldcnt += 1;
        }
    }

    informe.push_str("\nCAMBIOS DETECTADOS POR LÍNEA:\n");
    informe.push_str(&format!(
        "  Líneas con DISPCNT dinámico: {cambios_dispcnt} / 160\n"
    ));
    informe.push_str(&format!(
        "  Líneas con BLDCNT dinámico:  {cambios_bldcnt} / 160\n"
    ));
    informe
}

/// Tarea asíncrona enviada al hilo de fondo para escribir imágenes y diagnósticos.
pub struct TareaVolcado {
    pub cuadro: u64,
    pub sw_fin: Vec<u8>,
    pub sw_linea: Vec<u8>,
    pub nat: Vec<u8>,
    pub ppu_state: Option<Box<crate::pipeline::PpuFrameState>>,
}

/// Gestor de volcado desacoplado para escritura en disco sin degradar el bucle de emulación.
pub struct GestorVolcado {
    tx: Option<std::sync::mpsc::SyncSender<TareaVolcado>>,
    hilo: Option<std::thread::JoinHandle<()>>,
    cuadros_grabados: Arc<AtomicUsize>,
    activo: Arc<AtomicBool>,
    dir_salida: PathBuf,
}

impl GestorVolcado {
    /// Crea un nuevo gestor de volcado resolviendo la ruta óptima de Descargas.
    pub fn new(search_dirs: &[PathBuf]) -> Self {
        let dir = resolver_directorio_volcados(search_dirs);
        Self::with_dir(dir)
    }

    /// Crea un nuevo gestor de volcado apuntando a un directorio explícito.
    pub fn with_dir(dir_salida: PathBuf) -> Self {
        let (tx, rx) = std::sync::mpsc::sync_channel::<TareaVolcado>(16);
        let cuadros_grabados = Arc::new(AtomicUsize::new(0));
        let cuadros_clone = Arc::clone(&cuadros_grabados);
        let activo = Arc::new(AtomicBool::new(true));
        let dir_clone = dir_salida.clone();

        let hilo = std::thread::Builder::new()
            .name("blazing-volcado".to_string())
            .spawn(move || {
                let _ = std::fs::create_dir_all(&dir_clone);

                while let Ok(tarea) = rx.recv() {
                    let compuesto =
                        generar_lienzo_compuesto(&tarea.sw_fin, &tarea.sw_linea, &tarea.nat);
                    let png = png_rgb(&compuesto, ANCHO_COMPUESTO, ALTO_GBA);
                    let ruta_png = dir_clone.join(format!("cuadro-{}-compuesto.png", tarea.cuadro));
                    if let Err(e) = std::fs::write(&ruta_png, png) {
                        log::error!("Error al escribir volcado PNG {:?}: {e}", ruta_png);
                    }

                    let dummy = crate::pipeline::PpuFrameState::new();
                    let st = tarea.ppu_state.as_deref().unwrap_or(&dummy);
                    let txt =
                        generar_informe_diagnostico(tarea.cuadro, &tarea.sw_linea, &tarea.nat, st);
                    let ruta_txt = dir_clone.join(format!("cuadro-{}.txt", tarea.cuadro));
                    if let Err(e) = std::fs::write(&ruta_txt, txt) {
                        log::error!("Error al escribir diagnóstico TXT {:?}: {e}", ruta_txt);
                    }

                    cuadros_clone.fetch_add(1, Ordering::SeqCst);
                }
            })
            .ok();

        Self {
            tx: Some(tx),
            hilo,
            cuadros_grabados,
            activo,
            dir_salida,
        }
    }

    /// Envía un fotograma a la cola bloqueando hasta que haya espacio en el canal.
    pub fn enviar_cuadro(&self, tarea: TareaVolcado) -> bool {
        if !self.esta_grabando() {
            return false;
        }
        if let Some(ref tx) = self.tx {
            tx.send(tarea).is_ok()
        } else {
            false
        }
    }

    /// Intenta enviar un fotograma de forma no bloqueante (ideal para bucle a 60 FPS).
    pub fn intentar_enviar_cuadro(&self, tarea: TareaVolcado) -> bool {
        if !self.esta_grabando() {
            return false;
        }
        if let Some(ref tx) = self.tx {
            tx.try_send(tarea).is_ok()
        } else {
            false
        }
    }

    /// Total de fotogramas procesados y volcados a disco en la sesión actual.
    #[inline]
    pub fn cuadros_grabados(&self) -> usize {
        self.cuadros_grabados.load(Ordering::Acquire)
    }

    /// Indica si el gestor sigue aceptando y grabando fotogramas.
    #[inline]
    pub fn esta_grabando(&self) -> bool {
        self.activo.load(Ordering::Acquire)
    }

    /// Ruta absoluta o relativa al directorio configurado para escribir los volcados.
    #[inline]
    pub fn dir_salida(&self) -> &Path {
        &self.dir_salida
    }

    /// Detiene la recepción y espera a que el hilo termine de procesar las tareas pendientes.
    pub fn detener_y_esperar(&mut self) {
        self.activo.store(false, Ordering::Release);
        drop(self.tx.take());
        if let Some(h) = self.hilo.take() {
            let _ = h.join();
        }
    }
}

impl Drop for GestorVolcado {
    fn drop(&mut self) {
        self.detener_y_esperar();
    }
}

#[cfg(test)]
#[path = "volcado_tests.rs"]
mod tests;
