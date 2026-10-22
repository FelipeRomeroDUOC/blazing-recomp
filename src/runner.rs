//! Ejecución y sincronización del motor de emulación GBA (Fase 4 y Fase 5).
//!
//! Encapsula el modelo de hardware (`gba_core::Machine`), el simulador de panel
//! LCD con respuesta temporal (`screen::Temporal`) y la tabla de color (`screen::ColorLut`).
//!
//! Este módulo es puro, sin dependencias de Android ni de ventanas gráficas,
//! testeable en host con `cargo test`.

use screen::{ColorLut, ColorSettings, ResponseMode, ScreenKind, Temporal};

/// Ancho estándar del framebuffer de la Game Boy Advance en píxeles.
pub const GBA_W: u32 = 240;

/// Alto estándar del framebuffer de la Game Boy Advance en píxeles.
pub const GBA_H: u32 = 160;

/// Total de píxeles por fotograma de la Game Boy Advance (38,400).
pub const GBA_PIXELS: usize = (GBA_W * GBA_H) as usize;

/// Ciclos de reloj maestro por fotograma estándar (~16.78 MHz / 59.73 Hz).
pub const CYCLES_PER_FRAME: u64 = 280_896;

/// Métricas temporales registradas en el último avance de fotograma (`step_frame`).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct StepTiming {
    /// Milisegundos empleados en la ejecución de la lógica del juego.
    pub ejec_ms: f64,
    /// Milisegundos empleados en la conversión de color y composición LCD.
    pub color_ms: f64,
    /// Cantidad de bloques nativos AOT ejecutados.
    pub native_blocks: u64,
    /// Cantidad de pasos en intérprete de respaldo ejecutados.
    pub interpreter_steps: u64,
    /// Milisegundos empleados por la PPU en renderizado de scanlines.
    pub ppu_ms: f64,
    /// Milisegundos empleados por la PPU en renderizado de fondos.
    pub ppu_bg_ms: f64,
    /// Milisegundos empleados por la PPU en rasterizado de sprites.
    pub ppu_sprite_ms: f64,
    /// Milisegundos empleados por la PPU en composición de capas y efectos de color.
    pub ppu_compose_ms: f64,
    /// Milisegundos empleados por el APU en síntesis de audio.
    pub apu_ms: f64,
    /// Llamadas a `pump_audio` en este paso (contador entero).
    pub pump_llamadas: u64,
    /// Muestras de audio generadas en este paso.
    pub pump_muestras: u64,
    /// Cantidad de accesos de lectura a memoria realizados.
    pub reads: u64,
    /// Cantidad de accesos de escritura a memoria realizados.
    pub writes: u64,
}

/// Ejecutor del juego que conecta el hardware emulado con la generación del framebuffer RGBA.
pub struct GameRunner {
    pub machine: gba_core::Machine,
    pub block_table: Option<crate::recomp::BlockTable>,
    pub temporal: Temporal,
    pub lut: ColorLut,
    pub screen_kind: ScreenKind,
    pub gpu_ppu_enabled: bool,
    rgba: Vec<[u8; 4]>,
    frames_elapsed: u64,
    last_timing: StepTiming,
}

impl GameRunner {
    /// Inicializa una nueva sesión de juego a partir de los bytes de la ROM
    /// y un BIOS opcional (si es `None`, se utiliza el BIOS HLE embebido en `gba-core`).
    pub fn new(rom: Vec<u8>, bios: Option<&[u8]>, screen_kind: ScreenKind) -> Self {
        let mut machine = match bios {
            Some(b) => gba_core::Machine::new_with_bios(rom, b),
            None => gba_core::Machine::new(rom),
        };
        // Medicion del coste de audio: solo si existe perf-audio-timing.flag en Descargas.
        machine.bus.perf_audio_timing = crate::perf::audio_timing_flag_enabled(&[]);
        let block_table = crate::recomp::load_native_blocks();
        let mut settings = ColorSettings::default();
        settings.screen = screen_kind;
        let lut = ColorLut::build(&settings);
        let temporal = Temporal::new(
            GBA_PIXELS,
            ResponseMode::Simple,
            1,
            screen::blend::default_rho(screen_kind),
        );
        let rgba = vec![[0u8, 0u8, 0u8, 255u8]; GBA_PIXELS];

        Self {
            machine,
            block_table,
            temporal,
            lut,
            screen_kind,
            gpu_ppu_enabled: false,
            rgba,
            frames_elapsed: 0,
            last_timing: StepTiming::default(),
        }
    }

    /// Asigna o reemplaza la tabla de bloques recompilados nativos.
    pub fn set_block_table(&mut self, table: Option<crate::recomp::BlockTable>) {
        self.block_table = table;
    }

    /// Indica si el runner está ejecutando con la tabla nativa AOT cargada.
    #[inline]
    pub fn is_native(&self) -> bool {
        self.block_table.as_ref().map_or(false, |t| !t.is_empty())
    }

    /// Activa o desactiva el renderizado por GPU mediante PPU Shim.
    #[inline]
    pub fn set_gpu_ppu_enabled(&mut self, enabled: bool) {
        self.gpu_ppu_enabled = enabled;
    }

    /// Indica si el renderizado PPU Shim en GPU esta activo.
    #[inline]
    pub fn is_gpu_ppu_enabled(&self) -> bool {
        self.gpu_ppu_enabled
    }

    /// Referencia a la VRAM de la GBA (96 KB).
    #[inline]
    pub fn vram(&self) -> &[u8] {
        &self.machine.bus.vram
    }

    /// Referencia a la OAM de la GBA (1 KB).
    #[inline]
    pub fn oam(&self) -> &[u8] {
        &self.machine.bus.oam
    }

    /// Referencia a la memoria de paletas de la GBA (1 KB).
    #[inline]
    pub fn palette(&self) -> &[u8] {
        &self.machine.bus.palette
    }

    /// Referencia a los registros de I/O de la GBA.
    #[inline]
    pub fn io(&self) -> &[u8] {
        &self.machine.bus.io
    }

    /// Inyecta la máscara de botones activa en bajo en el bus de hardware (`KEYINPUT`).
    ///
    /// Bit = 0 significa botón pulsado; Bit = 1 significa botón liberado.
    #[inline]
    pub fn set_keys(&mut self, keys: u16) {
        self.machine.bus.keys = keys;
    }

    /// Obtiene la máscara actual de botones configurada en el bus de hardware.
    #[inline]
    pub fn keys(&self) -> u16 {
        self.machine.bus.keys
    }

    /// Avanza la simulación hasta completar el siguiente fotograma visible.
    ///
    /// Retorna `true` si se completó un fotograma (inicio de VBlank) y el búfer
    /// RGBA contiene los nuevos píxeles listos para presentar en GPU.
    pub fn step_frame(&mut self) -> bool {
        let t0 = std::time::Instant::now();
        let (native, fallback) = if let Some(ref table) = self.block_table {
            crate::recomp::run_frame_native(&mut self.machine, table, 2_000_000)
        } else {
            self.machine.run_frame(CYCLES_PER_FRAME);
            (0, 0)
        };
        let t1 = std::time::Instant::now();
        let ejec_ms = t1.duration_since(t0).as_secs_f64() * 1000.0;

        let hw = self.machine.take_perf_counters();

        // Reparto de las líneas de la PPU entre caminos de composición: se vuelca una vez
        // por fotograma para que el motor no pague sincronización por línea.
        crate::recomp::add_ppu_path(
            hw.lineas_rectas,
            hw.descartes_mezcla,
            hw.descartes_ventana,
            hw.descartes_semi,
            hw.lineas_inertes,
            hw.lineas_efecto,
            hw.lineas_efecto_alpha,
            hw.lineas_efecto_brillo,
            hw.lineas_efecto_oscuridad,
        );
        let ppu_ms = (hw.ppu_ns as f64) / 1_000_000.0;
        let ppu_bg_ms = (hw.bg_ns as f64) / 1_000_000.0;
        let ppu_sprite_ms = (hw.sprite_ns as f64) / 1_000_000.0;
        let ppu_compose_ms = (hw.compose_ns as f64) / 1_000_000.0;
        let apu_ms = (hw.apu_ns as f64) / 1_000_000.0;

        if self.machine.bus.frame_ready {
            let t2 = std::time::Instant::now();
            self.temporal.push(&self.machine.bus.framebuffer, &self.lut);
            self.temporal.compose(&self.lut, &mut self.rgba);
            let t3 = std::time::Instant::now();
            let color_ms = t3.duration_since(t2).as_secs_f64() * 1000.0;

            self.last_timing = StepTiming {
                ejec_ms,
                color_ms,
                native_blocks: native,
                interpreter_steps: fallback,
                ppu_ms,
                ppu_bg_ms,
                ppu_sprite_ms,
                ppu_compose_ms,
                apu_ms,
                pump_llamadas: hw.pump_llamadas,
                pump_muestras: hw.pump_muestras,
                reads: hw.reads,
                writes: hw.writes,
            };

            self.frames_elapsed += 1;
            self.machine.bus.frame_ready = false;
            true
        } else {
            self.last_timing = StepTiming {
                ejec_ms,
                color_ms: 0.0,
                native_blocks: native,
                interpreter_steps: fallback,
                ppu_ms,
                ppu_bg_ms,
                ppu_sprite_ms,
                ppu_compose_ms,
                apu_ms,
                pump_llamadas: hw.pump_llamadas,
                pump_muestras: hw.pump_muestras,
                reads: hw.reads,
                writes: hw.writes,
            };
            false
        }
    }

    /// Retorna las métricas temporales del último avance de fotograma.
    #[inline]
    pub fn last_timing(&self) -> StepTiming {
        self.last_timing
    }

    /// Referencia de solo lectura al búfer de píxeles RGBA (240x160) compuesto.
    #[inline]
    pub fn frame_rgba(&self) -> &[[u8; 4]] {
        &self.rgba
    }

    /// Total de fotogramas completados desde la inicialización.
    #[inline]
    pub fn frames_elapsed(&self) -> u64 {
        self.frames_elapsed
    }

    /// Ciclos de reloj maestro acumulados por la consola.
    #[inline]
    pub fn clock(&self) -> u64 {
        self.machine.bus.clock
    }

    /// Actualiza el perfil de simulación de pantalla (LUT y respuesta temporal).
    pub fn set_screen_kind(&mut self, screen_kind: ScreenKind) {
        self.screen_kind = screen_kind;
        let mut settings = ColorSettings::default();
        settings.screen = screen_kind;
        self.lut = ColorLut::build(&settings);
        self.temporal = Temporal::new(
            GBA_PIXELS,
            ResponseMode::Simple,
            1,
            screen::blend::default_rho(screen_kind),
        );
    }
}

#[cfg(test)]
#[path = "runner_tests.rs"]
mod tests;
