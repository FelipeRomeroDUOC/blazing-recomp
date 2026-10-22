//! Módulo de medición de rendimiento por fase del fotograma.
//!
//! Permite registrar tiempos de ejecución de CPU, conversión de color,
//! dibujo en GPU, espera de swapchain/VSync y vueltas vacías del bucle,
//! generando un informe consolidado por segundo para diagnóstico sin
//! distorsionar el hilo principal.

use crate::telemetria::Valor;
use std::collections::BTreeMap;
use std::time::{Duration, Instant};

/// Medición de las fases de un único fotograma completado.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct FrameTiming {
    /// Tiempo en milisegundos dedicado a la ejecución de la lógica del juego.
    pub ejec_ms: f64,
    /// Tiempo en milisegundos dedicado a la conversión de color y respuesta temporal.
    pub color_ms: f64,
    /// Tiempo en milisegundos dedicado a la presentación y shaders en GPU (`redraw`).
    pub dibujo_ms: f64,
    /// Tiempo en milisegundos bloqueado esperando un búfer de presentación (`get_current_texture`).
    pub espera_ms: f64,
    /// Fase de subida: `write_texture`/`write_buffer` a la GPU (ms).
    pub subida_ms: f64,
    /// Fase de codificación: `get_current_texture` + grabado y `queue.submit` (ms).
    pub codificacion_ms: f64,
    /// Fase de presentación: `frame.present()` (ms).
    pub present_ms: f64,
    /// Bloques nativos recompilados ejecutados en este fotograma.
    pub nativos: u64,
    /// Pasos de intérprete de respaldo ejecutados en este fotograma.
    pub interprete: u64,
    /// Tiempo en milisegundos dedicado a la PPU (renderizado de scanlines).
    pub ppu_ms: f64,
    /// Tiempo en milisegundos dedicado al renderizado de fondos en PPU.
    pub ppu_bg_ms: f64,
    /// Tiempo en milisegundos dedicado al rasterizado de sprites en PPU.
    pub ppu_sprite_ms: f64,
    /// Tiempo en milisegundos dedicado a la composición y blend en PPU.
    pub ppu_compose_ms: f64,
    /// Tiempo en milisegundos dedicado al APU (síntesis de audio).
    pub apu_ms: f64,
    /// Llamadas a `pump_audio` durante el fotograma (contador entero).
    pub pump_llamadas: u64,
    /// Muestras de audio generadas durante el fotograma.
    pub pump_muestras: u64,
    /// Accesos de lectura a memoria realizados en este fotograma.
    pub lecturas: u64,
    /// Accesos de escritura a memoria realizados en este fotograma.
    pub escrituras: u64,
}

/// Muestreo de características de hardware GBA en un fotograma completado.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct PpuHwFrameSample {
    /// Indica si el fotograma se renderizó por hardware en GPU (PpuShim).
    pub is_gpu: bool,
    /// Indica si el fotograma lo dibujó la PPU nativa (y no el shim).
    ///
    /// Las dos rutas van por GPU, así que `is_gpu` no las distingue: este
    /// campo es el que dice quién pintó el cuadro.
    pub ppu_nativa: bool,
    /// Indica si el modo de pantalla es Modo 0 (mosaicos de texto regulares).
    pub is_mode0: bool,
    /// Indica si alguna ventana de hardware (WIN0, WIN1 u OBJWIN) está activa en DISPCNT.
    pub window_active: bool,
    /// Indica si algún efecto de mezcla (alfa, brillo u oscuridad) está activo en BLDCNT.
    pub blend_active: bool,
    /// Total de sprites visibles/activos en OAM en este fotograma.
    pub spr_total: u16,
    /// Sprites activos que usan modo de 256 colores / 8bpp.
    pub spr_8bpp: u16,
    /// Sprites activos que usan rotación/escalado afín.
    pub spr_affine: u16,
}

/// Estadísticas agregadas de hardware GBA para una ventana de telemetría de 1 segundo.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct PpuHwWindowStats {
    /// Fotogramas completados mediante el pipeline acelerado PpuShim en GPU.
    pub gpu_frames: u32,
    /// Fotogramas que requirieron fallback a subida de framebuffer por software.
    pub sw_frames: u32,
    /// Fotogramas que dibujó la PPU nativa (en vez del shim).
    pub nativa_frames: u32,
    /// Fotogramas que operaron en Modo 0.
    pub mode0_frames: u32,
    /// Fotogramas con ventanas de hardware activas (WIN0/WIN1/OBJWIN).
    pub window_frames: u32,
    /// Fotogramas con efectos de color o transparencia activos (BLDCNT).
    pub blend_frames: u32,
    /// Promedio de sprites activos por fotograma.
    pub spr_total_avg: u16,
    /// Promedio de sprites de 256 colores (8bpp) por fotograma.
    pub spr_8bpp_avg: u16,
    /// Promedio de sprites afines (rotación/escala) por fotograma.
    pub spr_affine_avg: u16,
}

impl PpuHwWindowStats {
    /// Formatea la línea de telemetría de hardware GBA (`# ppu_hw`).
    pub fn to_line(&self) -> String {
        format!(
            "# ppu_hw gpu={} sw={} modo0={} win={} blend={} spr_total={} spr_8bpp={} spr_afin={} nativa={}",
            self.gpu_frames,
            self.sw_frames,
            self.mode0_frames,
            self.window_frames,
            self.blend_frames,
            self.spr_total_avg,
            self.spr_8bpp_avg,
            self.spr_affine_avg,
            self.nativa_frames,
        )
    }
}

/// Informe de rendimiento consolidado correspondiente a un segundo de ejecución.
#[derive(Clone, Debug, PartialEq)]
pub struct PerfReport {
    /// Estadísticas agregadas de características de hardware GBA en la ventana.
    pub ppu_hw: PpuHwWindowStats,
    /// Índice del segundo registrado (1 para t=1s, 2 para t=2s, etc.).
    pub second_index: u64,
    /// Fotogramas de simulación completados por segundo.
    pub fps: f64,
    /// Promedio de milisegundos de ejecución.
    pub ejec_avg_ms: f64,
    /// Peor caso (máximo) de milisegundos de ejecución.
    pub ejec_max_ms: f64,
    /// Promedio de milisegundos consumidos por la PPU.
    pub ppu_avg_ms: f64,
    /// Promedio de milisegundos consumidos en fondos por la PPU.
    pub ppu_bg_avg_ms: f64,
    /// Promedio de milisegundos consumidos en sprites por la PPU.
    pub ppu_sprite_avg_ms: f64,
    /// Promedio de milisegundos consumidos en composición por la PPU.
    pub ppu_compose_avg_ms: f64,
    /// Promedio de milisegundos consumidos por el APU.
    pub apu_avg_ms: f64,
    /// Promedio de llamadas a `pump_audio` por fotograma.
    pub pump_llamadas_avg: u64,
    /// Promedio de muestras de audio generadas por fotograma.
    pub pump_muestras_avg: u64,
    /// Promedio de lecturas de memoria por fotograma.
    pub lecturas_avg: u64,
    /// Promedio de escrituras de memoria por fotograma.
    pub escrituras_avg: u64,
    /// Promedio de milisegundos de conversión de color.
    pub color_avg_ms: f64,
    /// Peor caso (máximo) de milisegundos de conversión de color.
    pub color_max_ms: f64,
    /// Promedio de milisegundos de dibujo y presentación en GPU.
    pub dibujo_avg_ms: f64,
    /// Peor caso (máximo) de milisegundos de dibujo en GPU.
    pub dibujo_max_ms: f64,
    /// Promedio de milisegundos de espera de swapchain/VSync.
    pub espera_avg_ms: f64,
    /// Peor caso (máximo) de milisegundos de espera de swapchain/VSync.
    pub espera_max_ms: f64,
    /// Promedio de milisegundos de la fase de subida a GPU.
    pub subida_avg_ms: f64,
    /// Promedio de milisegundos de la fase de codificación y `submit`.
    pub codificacion_avg_ms: f64,
    /// Promedio de milisegundos de la fase de presentación (`present`).
    pub present_avg_ms: f64,
    /// Peor caso (máximo) de milisegundos de la fase de presentación (`present`).
    pub present_max_ms: f64,
    /// Promedio de bloques nativos por fotograma.
    pub nativos_avg: u64,
    /// Promedio de pasos de intérprete por fotograma.
    pub interprete_avg: u64,
    /// Cantidad de vueltas vacías del bucle principal en la ventana.
    pub vacias: u64,
    /// Dimensiones de la ventana (ancho x alto).
    pub ventana: (u32, u32),
}

impl PerfReport {
    /// Formatea el informe en una única línea, guiada por el esquema de [`crate::telemetria`].
    ///
    /// Ya no es un `format!` de 28 huecos: los valores se recogen en un mapa y
    /// `telemetria::escribir_linea` los imprime en el orden canónico del esquema y
    /// remata la línea con el sello de versión del contrato. La firma no cambia, así
    /// que `platform.rs` sigue escribiendo el resultado tal cual.
    ///
    /// Si el contrato está roto (un campo del esquema sin valor, por ejemplo), devuelve
    /// una línea `#` con el motivo en vez de entrar en pánico: el fallo se ve en
    /// `blazing-perf.txt` y no tumba la partida.
    pub fn to_line(&self) -> String {
        match crate::telemetria::escribir_linea(&self.valores_de_linea()) {
            Ok(linea) => linea,
            Err(motivo) => format!("# telemetría inválida: {motivo}"),
        }
    }

    /// Valores de la línea de segundo, indexados por el nombre de campo del esquema.
    fn valores_de_linea(&self) -> BTreeMap<&'static str, Valor> {
        let mut valores = BTreeMap::new();
        valores.insert("t", Valor::Entero(self.second_index));
        valores.insert("fps", Valor::Real(self.fps));
        valores.insert("ejec", Valor::RealReal(self.ejec_avg_ms, self.ejec_max_ms));
        valores.insert("ppu", Valor::Real(self.ppu_avg_ms));
        valores.insert("ppubg", Valor::Real(self.ppu_bg_avg_ms));
        valores.insert("ppuspr", Valor::Real(self.ppu_sprite_avg_ms));
        valores.insert("ppucomp", Valor::Real(self.ppu_compose_avg_ms));
        valores.insert("apu", Valor::Real(self.apu_avg_ms));
        valores.insert("lecturas", Valor::Entero(self.lecturas_avg));
        valores.insert("escrituras", Valor::Entero(self.escrituras_avg));
        valores.insert("color", Valor::RealReal(self.color_avg_ms, self.color_max_ms));
        valores.insert("dibujo", Valor::RealReal(self.dibujo_avg_ms, self.dibujo_max_ms));
        valores.insert("espera", Valor::RealReal(self.espera_avg_ms, self.espera_max_ms));
        valores.insert("subida", Valor::Real(self.subida_avg_ms));
        valores.insert("codificacion", Valor::Real(self.codificacion_avg_ms));
        valores.insert("present", Valor::Real(self.present_avg_ms));
        valores.insert("presentmax", Valor::Real(self.present_max_ms));
        valores.insert("nativos", Valor::Entero(self.nativos_avg));
        valores.insert("interprete", Valor::Entero(self.interprete_avg));
        valores.insert("vacias", Valor::Entero(self.vacias));
        valores.insert(
            "ventana",
            Valor::EnteroEntero(self.ventana.0, self.ventana.1),
        );
        valores.insert("pump", Valor::Entero(self.pump_llamadas_avg));
        valores.insert("pumpm", Valor::Entero(self.pump_muestras_avg));
        valores
    }
}

/// Acumulador de métricas de rendimiento por fase con ventanas temporales de 1 segundo.
pub struct FramePerf {
    window_start: Instant,
    seconds_count: u64,
    frames: Vec<FrameTiming>,
    vacias: u64,
    hw_gpu_frames: u32,
    hw_sw_frames: u32,
    hw_nativa_frames: u32,
    hw_mode0_frames: u32,
    hw_window_frames: u32,
    hw_blend_frames: u32,
    hw_spr_total_sum: u32,
    hw_spr_8bpp_sum: u32,
    hw_spr_affine_sum: u32,
}

impl FramePerf {
    /// Inicializa un nuevo acumulador de rendimiento.
    pub fn new() -> Self {
        Self::new_at(Instant::now())
    }

    /// Inicializa el acumulador con un instante de inicio específico (útil para pruebas deterministas).
    pub fn new_at(start: Instant) -> Self {
        Self {
            window_start: start,
            seconds_count: 0,
            frames: Vec::new(),
            vacias: 0,
            hw_gpu_frames: 0,
            hw_sw_frames: 0,
            hw_nativa_frames: 0,
            hw_mode0_frames: 0,
            hw_window_frames: 0,
            hw_blend_frames: 0,
            hw_spr_total_sum: 0,
            hw_spr_8bpp_sum: 0,
            hw_spr_affine_sum: 0,
        }
    }

    /// Registra una vuelta vacía del bucle de eventos (sin avance de fotograma ni redibujado).
    pub fn record_empty_turn(&mut self) {
        self.vacias += 1;
    }

    /// Registra las métricas de un fotograma de simulación completado.
    pub fn record_frame(&mut self, timing: FrameTiming) {
        self.frames.push(timing);
    }

    /// Registra el muestreo de características de hardware GBA para el fotograma.
    pub fn record_ppu_hw(&mut self, sample: PpuHwFrameSample) {
        if sample.is_gpu {
            self.hw_gpu_frames += 1;
        } else {
            self.hw_sw_frames += 1;
        }
        if sample.ppu_nativa {
            self.hw_nativa_frames += 1;
        }
        if sample.is_mode0 {
            self.hw_mode0_frames += 1;
        }
        if sample.window_active {
            self.hw_window_frames += 1;
        }
        if sample.blend_active {
            self.hw_blend_frames += 1;
        }
        self.hw_spr_total_sum += sample.spr_total as u32;
        self.hw_spr_8bpp_sum += sample.spr_8bpp as u32;
        self.hw_spr_affine_sum += sample.spr_affine as u32;
    }

    /// Cierra la ventana actual y genera un informe de rendimiento con los datos acumulados.
    pub fn close_window(&mut self, ventana: (u32, u32)) -> PerfReport {
        self.seconds_count += 1;
        let count = self.frames.len();

        let hw_stats = if count == 0 {
            PpuHwWindowStats {
                gpu_frames: self.hw_gpu_frames,
                sw_frames: self.hw_sw_frames,
                nativa_frames: self.hw_nativa_frames,
                mode0_frames: self.hw_mode0_frames,
                window_frames: self.hw_window_frames,
                blend_frames: self.hw_blend_frames,
                spr_total_avg: 0,
                spr_8bpp_avg: 0,
                spr_affine_avg: 0,
            }
        } else {
            let c = count as f64;
            PpuHwWindowStats {
                gpu_frames: self.hw_gpu_frames,
                sw_frames: self.hw_sw_frames,
                nativa_frames: self.hw_nativa_frames,
                mode0_frames: self.hw_mode0_frames,
                window_frames: self.hw_window_frames,
                blend_frames: self.hw_blend_frames,
                spr_total_avg: (self.hw_spr_total_sum as f64 / c).round() as u16,
                spr_8bpp_avg: (self.hw_spr_8bpp_sum as f64 / c).round() as u16,
                spr_affine_avg: (self.hw_spr_affine_sum as f64 / c).round() as u16,
            }
        };

        let report = if count == 0 {
            PerfReport {
                second_index: self.seconds_count,
                fps: 0.0,
                ppu_hw: hw_stats,
                ejec_avg_ms: 0.0,
                ejec_max_ms: 0.0,
                ppu_avg_ms: 0.0,
                ppu_bg_avg_ms: 0.0,
                ppu_sprite_avg_ms: 0.0,
                ppu_compose_avg_ms: 0.0,
                apu_avg_ms: 0.0,
                pump_llamadas_avg: 0,
                pump_muestras_avg: 0,
                lecturas_avg: 0,
                escrituras_avg: 0,
                color_avg_ms: 0.0,
                color_max_ms: 0.0,
                dibujo_avg_ms: 0.0,
                dibujo_max_ms: 0.0,
                espera_avg_ms: 0.0,
                espera_max_ms: 0.0,
                subida_avg_ms: 0.0,
                codificacion_avg_ms: 0.0,
                present_avg_ms: 0.0,
                present_max_ms: 0.0,
                nativos_avg: 0,
                interprete_avg: 0,
                vacias: self.vacias,
                ventana,
            }
        } else {
            let mut sum_ejec = 0.0;
            let mut max_ejec = 0.0f64;
            let mut sum_ppu = 0.0;
            let mut sum_ppu_bg = 0.0;
            let mut sum_ppu_sprite = 0.0;
            let mut sum_ppu_compose = 0.0;
            let mut sum_apu = 0.0;
            let mut sum_lecturas = 0u64;
            let mut sum_escrituras = 0u64;
            let mut sum_color = 0.0;
            let mut max_color = 0.0f64;
            let mut sum_dibujo = 0.0;
            let mut max_dibujo = 0.0f64;
            let mut sum_espera = 0.0;
            let mut max_espera = 0.0f64;
            let mut sum_subida = 0.0;
            let mut sum_codificacion = 0.0;
            let mut sum_present = 0.0;
            let mut max_present = 0.0f64;
            let mut sum_nativos = 0u64;
            let mut sum_interprete = 0u64;
            let mut sum_pump_llamadas = 0u64;
            let mut sum_pump_muestras = 0u64;

            for f in &self.frames {
                sum_ejec += f.ejec_ms;
                max_ejec = max_ejec.max(f.ejec_ms);
                sum_ppu += f.ppu_ms;
                sum_ppu_bg += f.ppu_bg_ms;
                sum_ppu_sprite += f.ppu_sprite_ms;
                sum_ppu_compose += f.ppu_compose_ms;
                sum_apu += f.apu_ms;
                sum_lecturas += f.lecturas;
                sum_escrituras += f.escrituras;
                sum_color += f.color_ms;
                max_color = max_color.max(f.color_ms);
                sum_dibujo += f.dibujo_ms;
                max_dibujo = max_dibujo.max(f.dibujo_ms);
                sum_espera += f.espera_ms;
                max_espera = max_espera.max(f.espera_ms);
                sum_subida += f.subida_ms;
                sum_codificacion += f.codificacion_ms;
                sum_present += f.present_ms;
                max_present = max_present.max(f.present_ms);
                sum_nativos += f.nativos;
                sum_interprete += f.interprete;
                sum_pump_llamadas += f.pump_llamadas;
                sum_pump_muestras += f.pump_muestras;
            }

            let c = count as f64;
            PerfReport {
                second_index: self.seconds_count,
                fps: c,
                ppu_hw: hw_stats,
                ejec_avg_ms: sum_ejec / c,
                ejec_max_ms: max_ejec,
                ppu_avg_ms: sum_ppu / c,
                ppu_bg_avg_ms: sum_ppu_bg / c,
                ppu_sprite_avg_ms: sum_ppu_sprite / c,
                ppu_compose_avg_ms: sum_ppu_compose / c,
                apu_avg_ms: sum_apu / c,
                pump_llamadas_avg: (sum_pump_llamadas as f64 / c).round() as u64,
                pump_muestras_avg: (sum_pump_muestras as f64 / c).round() as u64,
                lecturas_avg: (sum_lecturas as f64 / c).round() as u64,
                escrituras_avg: (sum_escrituras as f64 / c).round() as u64,
                color_avg_ms: sum_color / c,
                color_max_ms: max_color,
                dibujo_avg_ms: sum_dibujo / c,
                dibujo_max_ms: max_dibujo,
                espera_avg_ms: sum_espera / c,
                espera_max_ms: max_espera,
                subida_avg_ms: sum_subida / c,
                codificacion_avg_ms: sum_codificacion / c,
                present_avg_ms: sum_present / c,
                present_max_ms: max_present,
                nativos_avg: (sum_nativos as f64 / c).round() as u64,
                interprete_avg: (sum_interprete as f64 / c).round() as u64,
                vacias: self.vacias,
                ventana,
            }
        };

        self.frames.clear();
        self.vacias = 0;
        self.hw_gpu_frames = 0;
        self.hw_sw_frames = 0;
        self.hw_nativa_frames = 0;
        self.hw_mode0_frames = 0;
        self.hw_window_frames = 0;
        self.hw_blend_frames = 0;
        self.hw_spr_total_sum = 0;
        self.hw_spr_8bpp_sum = 0;
        self.hw_spr_affine_sum = 0;
        report
    }

    /// Evalúa si ha transcurrido un segundo desde el inicio de la ventana y, si es así,
    /// cierra la ventana y retorna el informe.
    pub fn tick_at(&mut self, now: Instant, ventana: (u32, u32)) -> Option<PerfReport> {
        if now.duration_since(self.window_start) >= Duration::from_secs(1) {
            self.window_start = now;
            Some(self.close_window(ventana))
        } else {
            None
        }
    }
}

impl Default for FramePerf {
    fn default() -> Self {
        Self::new()
    }
}

/// Mide el costo medio de una lectura de reloj del sistema (`Instant::now()`) en nanosegundos.
pub fn measure_clock_cost_ns() -> u64 {
    let start = Instant::now();
    let iters = 10_000;
    for _ in 0..iters {
        let _ = Instant::now();
    }
    let elapsed = start.elapsed().as_nanos() as u64;
    (elapsed / iters).max(1)
}

/// Indica si la medicion del coste de audio esta activada: existe el archivo
/// `perf-audio-timing.flag` en alguno de los directorios de telemetria.
pub fn audio_timing_flag_enabled(search_dirs: &[std::path::PathBuf]) -> bool {
    resolve_perf_candidates(search_dirs)
        .iter()
        .any(|dir| dir.join("perf-audio-timing.flag").exists())
}

/// Resuelve la lista ordenada de directorios candidatos para persistir `blazing-perf.txt`,
/// priorizando la carpeta compartida Descargas sobre el almacenamiento privado de la app.
pub fn resolve_perf_candidates(search_dirs: &[std::path::PathBuf]) -> Vec<std::path::PathBuf> {
    let mut priority = vec![
        std::path::PathBuf::from("/sdcard/Download"),
        std::path::PathBuf::from("/storage/emulated/0/Download"),
    ];
    for dir in search_dirs {
        if !priority.contains(dir) {
            priority.push(dir.clone());
        }
    }
    priority
}

#[cfg(test)]
#[path = "perf_tests.rs"]
mod tests;
