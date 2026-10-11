//! Módulo de medición de rendimiento por fase del fotograma.
//!
//! Permite registrar tiempos de ejecución de CPU, conversión de color,
//! dibujo en GPU, espera de swapchain/VSync y vueltas vacías del bucle,
//! generando un informe consolidado por segundo para diagnóstico sin
//! distorsionar el hilo principal.

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
    /// Tiempo en milisegundos dedicado al APU (síntesis de audio).
    pub apu_ms: f64,
    /// Accesos de lectura a memoria realizados en este fotograma.
    pub lecturas: u64,
    /// Accesos de escritura a memoria realizados en este fotograma.
    pub escrituras: u64,
}

/// Informe de rendimiento consolidado correspondiente a un segundo de ejecución.
#[derive(Clone, Debug, PartialEq)]
pub struct PerfReport {
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
    /// Promedio de milisegundos consumidos por el APU.
    pub apu_avg_ms: f64,
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
    /// Formatea el informe en una única línea de texto estandarizada.
    pub fn to_line(&self) -> String {
        format!(
            "t={}s fps={:.1} ejec={:.1}/{:.1} ppu={:.1} apu={:.1} lecturas={} escrituras={} color={:.1}/{:.1} dibujo={:.1}/{:.1} espera={:.1}/{:.1} subida={:.1} codificacion={:.1} present={:.1} nativos={} interprete={} vacias={} ventana={}x{}",
            self.second_index,
            self.fps,
            self.ejec_avg_ms,
            self.ejec_max_ms,
            self.ppu_avg_ms,
            self.apu_avg_ms,
            self.lecturas_avg,
            self.escrituras_avg,
            self.color_avg_ms,
            self.color_max_ms,
            self.dibujo_avg_ms,
            self.dibujo_max_ms,
            self.espera_avg_ms,
            self.espera_max_ms,
            self.subida_avg_ms,
            self.codificacion_avg_ms,
            self.present_avg_ms,
            self.nativos_avg,
            self.interprete_avg,
            self.vacias,
            self.ventana.0,
            self.ventana.1,
        )
    }
}

/// Acumulador de métricas de rendimiento por fase con ventanas temporales de 1 segundo.
pub struct FramePerf {
    window_start: Instant,
    seconds_count: u64,
    frames: Vec<FrameTiming>,
    vacias: u64,
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

    /// Cierra la ventana actual y genera un informe de rendimiento con los datos acumulados.
    pub fn close_window(&mut self, ventana: (u32, u32)) -> PerfReport {
        self.seconds_count += 1;
        let count = self.frames.len();

        let report = if count == 0 {
            PerfReport {
                second_index: self.seconds_count,
                fps: 0.0,
                ejec_avg_ms: 0.0,
                ejec_max_ms: 0.0,
                ppu_avg_ms: 0.0,
                apu_avg_ms: 0.0,
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
                nativos_avg: 0,
                interprete_avg: 0,
                vacias: self.vacias,
                ventana,
            }
        } else {
            let mut sum_ejec = 0.0;
            let mut max_ejec = 0.0f64;
            let mut sum_ppu = 0.0;
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
            let mut sum_nativos = 0u64;
            let mut sum_interprete = 0u64;

            for f in &self.frames {
                sum_ejec += f.ejec_ms;
                max_ejec = max_ejec.max(f.ejec_ms);
                sum_ppu += f.ppu_ms;
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
                sum_nativos += f.nativos;
                sum_interprete += f.interprete;
            }

            let c = count as f64;
            PerfReport {
                second_index: self.seconds_count,
                fps: c,
                ejec_avg_ms: sum_ejec / c,
                ejec_max_ms: max_ejec,
                ppu_avg_ms: sum_ppu / c,
                apu_avg_ms: sum_apu / c,
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
                nativos_avg: (sum_nativos as f64 / c).round() as u64,
                interprete_avg: (sum_interprete as f64 / c).round() as u64,
                vacias: self.vacias,
                ventana,
            }
        };

        self.frames.clear();
        self.vacias = 0;
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
