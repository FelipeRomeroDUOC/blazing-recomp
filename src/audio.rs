//! Módulo de audio nativo para Blazing Recomp.
//!
//! Gestiona la salida de audio de baja latencia mediante AAudio en Android,
//! el remuestreo de 65.536 Hz al reloj del dispositivo (e.g. 48.000 Hz),
//! el desacoplamiento con búfer circular SPSC y el control dinámico de deriva de reloj.

use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

/// Tasa de muestreo canónica del generador APU de Game Boy Advance (65.536 Hz).
pub const AUDIO_RATE_GBA: u32 = 65536;

/// Tasa de muestreo estándar por defecto para dispositivos Android modernos.
pub const DEFAULT_SAMPLE_RATE: u32 = 48000;

/// Capacidad del búfer circular en fotogramas de audio estéreo (~85 ms a 48 kHz).
pub const DEFAULT_RING_CAPACITY_FRAMES: usize = 4096;

/// Marca de agua objetivo en fotogramas estéreo para control dinámico de latencia (~30 ms a 48 kHz).
pub const TARGET_WATERMARK_FRAMES: usize = 1440;

/// Nombre del archivo de bandera para desactivar la salida de audio en caliente.
pub const NOMBRE_BANDERA_AUDIO_APAGADO: &str = "audio-off.flag";

/// Consulta si el audio nativo debe apagarse según la presencia de [`NOMBRE_BANDERA_AUDIO_APAGADO`].
pub fn audio_apagado_por_marca(search_dirs: &[PathBuf]) -> bool {
    crate::perf::resolve_perf_candidates(search_dirs)
        .iter()
        .any(|dir| dir.join(NOMBRE_BANDERA_AUDIO_APAGADO).exists())
}

/// Indica si el audio nativo debe usarse ahora.
///
/// La respuesta es «sí» salvo que exista la marca de apagado [NOMBRE_BANDERA_AUDIO_APAGADO].
pub fn audio_activado(search_dirs: &[PathBuf]) -> bool {
    !audio_apagado_por_marca(search_dirs)
}

/// Nombre del archivo de bandera para desactivar el Audio Mejorado (HLE shadow) en caliente.
pub const NOMBRE_BANDERA_AUDIO_MEJORADO_APAGADO: &str = "audio-enhanced-off.flag";

/// Consulta si el Audio Mejorado (HLE shadow) debe apagarse según la presencia de [`NOMBRE_BANDERA_AUDIO_MEJORADO_APAGADO`].
pub fn audio_mejorado_apagado_por_marca(search_dirs: &[PathBuf]) -> bool {
    crate::perf::resolve_perf_candidates(search_dirs)
        .iter()
        .any(|dir| dir.join(NOMBRE_BANDERA_AUDIO_MEJORADO_APAGADO).exists())
}

/// Indica si el Audio Mejorado (HLE shadow) debe usarse ahora.
///
/// La respuesta es «sí» por defecto salvo que exista la marca de apagado [`NOMBRE_BANDERA_AUDIO_MEJORADO_APAGADO`].
pub fn audio_mejorado_activado(search_dirs: &[PathBuf]) -> bool {
    !audio_mejorado_apagado_por_marca(search_dirs)
}

/// Mezcla las muestras de voz en punto flotante (`hle_tap`) con los canales PSG (`psg_tap`).
pub fn mix_enhanced_samples(hle: &[f32], psg: &[i16], dest: &mut Vec<i16>) {
    let pairs = (hle.len() / 2).min(psg.len() / 2);
    dest.clear();
    dest.reserve(pairs * 2);

    for i in 0..pairs {
        let hl = (hle[i * 2] * 32768.0) as i32;
        let hr = (hle[i * 2 + 1] * 32768.0) as i32;
        let pl = psg[i * 2] as i32;
        let pr = psg[i * 2 + 1] as i32;

        let out_l = (hl + pl).clamp(-32768, 32767) as i16;
        let out_r = (hr + pr).clamp(-32768, 32767) as i16;

        dest.push(out_l);
        dest.push(out_r);
    }
}

/// Remuestreador lineal estéreo de punto fijo.
pub struct AudioResampler {
    in_rate: u32,
    out_rate: u32,
    phase: u64,
    step: u64,
}

impl AudioResampler {
    pub fn new(in_rate: u32, out_rate: u32) -> Self {
        let step = ((in_rate as u64) << 32) / (out_rate.max(1) as u64);
        Self {
            in_rate,
            out_rate,
            phase: 0,
            step,
        }
    }

    #[inline]
    pub fn set_out_rate(&mut self, out_rate: u32) {
        self.out_rate = out_rate;
        self.step = ((self.in_rate as u64) << 32) / (out_rate.max(1) as u64);
    }

    #[inline]
    pub fn set_rate_adjustment(&mut self, delta_ppm: i32) {
        let base_step = ((self.in_rate as u64) << 32) / (self.out_rate.max(1) as u64);
        let adj = (base_step as i64 * delta_ppm as i64) / 1_000_000;
        self.step = (base_step as i64 + adj).max(1) as u64;
    }

    /// Remuestrea muestras PCM estéreo entrelazadas de `in_rate` a `out_rate`.
    pub fn resample(&mut self, input: &[i16], output: &mut Vec<i16>) {
        let in_frames = input.len() / 2;
        if in_frames == 0 {
            return;
        }

        let in_frames_fixed = (in_frames as u64) << 32;

        while self.phase < in_frames_fixed {
            let idx = (self.phase >> 32) as usize;
            let frac = ((self.phase & 0xFFFF_FFFF) >> 16) as i32; // 0..65535

            let (l0, r0) = (input[idx * 2] as i32, input[idx * 2 + 1] as i32);
            let (l1, r1) = if idx + 1 < in_frames {
                (input[(idx + 1) * 2] as i32, input[(idx + 1) * 2 + 1] as i32)
            } else {
                (l0, r0)
            };

            let out_l = l0 + (((l1 - l0) * frac) >> 16);
            let out_r = r0 + (((r1 - r0) * frac) >> 16);

            output.push(out_l.clamp(-32768, 32767) as i16);
            output.push(out_r.clamp(-32768, 32767) as i16);

            self.phase += self.step;
        }

        self.phase -= in_frames_fixed;
    }
}

/// Búfer circular acotado thread-safe (SPSC) para transporte de audio.
pub struct AudioRingBuffer {
    buffer: Mutex<Vec<i16>>,
    capacity_frames: usize,
    underruns: AtomicU64,
}

impl AudioRingBuffer {
    pub fn new(capacity_frames: usize) -> Self {
        Self {
            buffer: Mutex::new(Vec::with_capacity(capacity_frames * 2)),
            capacity_frames,
            underruns: AtomicU64::new(0),
        }
    }

    /// Deposita muestras PCM estéreo entrelazadas en el búfer circular.
    pub fn push(&self, samples: &[i16]) -> usize {
        if samples.is_empty() {
            return 0;
        }
        let in_frames = samples.len() / 2;
        let mut guard = self.buffer.lock().unwrap();

        let max_samples = self.capacity_frames * 2;
        let current_len = guard.len();

        if current_len + samples.len() > max_samples {
            let excess = (current_len + samples.len()) - max_samples;
            let discard = excess.min(current_len);
            guard.drain(0..discard);
        }

        guard.extend_from_slice(samples);
        in_frames
    }

    /// Retira muestras solicitadas por el consumidor (hardware de audio).
    pub fn pop(&self, dest: &mut [i16]) -> usize {
        let req_samples = dest.len();
        let req_frames = req_samples / 2;
        if req_frames == 0 {
            return 0;
        }

        let mut guard = self.buffer.lock().unwrap();
        let avail_samples = guard.len();

        if avail_samples >= req_samples {
            dest.copy_from_slice(&guard[..req_samples]);
            guard.drain(0..req_samples);
            req_frames
        } else {
            self.underruns.fetch_add(1, Ordering::Relaxed);
            let valid_frames = avail_samples / 2;

            if avail_samples > 0 {
                dest[..avail_samples].copy_from_slice(&guard[..avail_samples]);
                guard.clear();
            }
            dest[avail_samples..req_samples].fill(0);
            valid_frames
        }
    }

    pub fn available_frames(&self) -> usize {
        let guard = self.buffer.lock().unwrap();
        guard.len() / 2
    }

    pub fn underruns(&self) -> u64 {
        self.underruns.load(Ordering::Relaxed)
    }

    pub fn reset(&self) {
        let mut guard = self.buffer.lock().unwrap();
        guard.clear();
    }

    /// Precarga el búfer con silencio (muestras a cero) hasta alcanzar `target_frames`.
    ///
    /// Esto previene caídas iniciales (*underruns*) antes de que el hilo de emulación
    /// produzca sus primeros bloques de audio al iniciar o reanudar el flujo.
    pub fn prime(&self, target_frames: usize) {
        let mut guard = self.buffer.lock().unwrap();
        let target_samples = target_frames * 2;
        let max_samples = self.capacity_frames * 2;
        let to_fill = target_samples.min(max_samples);
        if guard.len() < to_fill {
            guard.resize(to_fill, 0);
        }
    }
}

/// Controlador dinámico de deriva de reloj (Dynamic Rate Control).
pub struct DynamicRateControl {
    target_frames: usize,
    tolerance_frames: usize,
}

impl DynamicRateControl {
    pub fn new(target_frames: usize, tolerance_frames: usize) -> Self {
        Self {
            target_frames,
            tolerance_frames,
        }
    }

    pub fn calculate_adjustment_ppm(&self, current_frames: usize) -> i32 {
        let diff = current_frames as i64 - self.target_frames as i64;
        if diff.abs() <= self.tolerance_frames as i64 {
            return 0;
        }
        let ppm = diff * 10;
        ppm.clamp(-5000, 5000) as i32
    }
}

/// Motor de audio nativo de producción para Android (AAudio).
#[cfg(target_os = "android")]
pub struct NativeAudioEngine {
    stream: Option<ndk::audio::AudioStream>,
    ring_buffer: Arc<AudioRingBuffer>,
    resampler: AudioResampler,
    drc: DynamicRateControl,
    sample_rate: u32,
    active: bool,
}

#[cfg(target_os = "android")]
impl NativeAudioEngine {
    pub fn new(sample_rate: u32) -> Self {
        let ring_buffer = Arc::new(AudioRingBuffer::new(DEFAULT_RING_CAPACITY_FRAMES));
        let resampler = AudioResampler::new(AUDIO_RATE_GBA, sample_rate);
        let drc = DynamicRateControl::new(TARGET_WATERMARK_FRAMES, 200);

        Self {
            stream: None,
            ring_buffer,
            resampler,
            drc,
            sample_rate,
            active: false,
        }
    }

    pub fn start(&mut self) -> Result<(), String> {
        let ring = Arc::clone(&self.ring_buffer);
        let callback = Box::new(
            move |_stream: &ndk::audio::AudioStream,
                  data: *mut std::ffi::c_void,
                  num_frames: i32| {
                if num_frames > 0 && !data.is_null() {
                    let slice = unsafe {
                        std::slice::from_raw_parts_mut(data as *mut i16, (num_frames * 2) as usize)
                    };
                    ring.pop(slice);
                }
                ndk::audio::AudioCallbackResult::Continue
            },
        );

        let builder = ndk::audio::AudioStreamBuilder::new()
            .map_err(|e| format!("fallo al crear AudioStreamBuilder: {e}"))?
            .direction(ndk::audio::AudioDirection::Output)
            .format(ndk::audio::AudioFormat::PCM_I16)
            .channel_count(2)
            .sample_rate(self.sample_rate as i32)
            .performance_mode(ndk::audio::AudioPerformanceMode::LowLatency)
            .sharing_mode(ndk::audio::AudioSharingMode::Shared)
            .usage(ndk::audio::AudioUsage::Game)
            .content_type(ndk::audio::AudioContentType::Music)
            .data_callback(callback);

        let stream = builder
            .open_stream()
            .map_err(|e| format!("fallo al abrir AAudioStream: {e}"))?;

        let actual_rate = stream.sample_rate() as u32;
        if actual_rate > 0 && actual_rate != self.sample_rate {
            self.sample_rate = actual_rate;
            self.resampler.set_out_rate(actual_rate);
        }

        self.ring_buffer.prime(TARGET_WATERMARK_FRAMES);
        stream
            .request_start()
            .map_err(|e| format!("fallo al iniciar AAudioStream: {e}"))?;

        log::info!(
            "AAudio iniciado exitosamente: {} Hz, 2 canales, buffer={}",
            self.sample_rate,
            stream.buffer_size_in_frames()
        );

        self.stream = Some(stream);
        self.active = true;
        Ok(())
    }

    pub fn pause(&mut self) {
        if let Some(ref stream) = self.stream {
            let _ = stream.request_pause();
        }
        self.active = false;
    }

    pub fn resume(&mut self) {
        if let Some(ref stream) = self.stream {
            self.ring_buffer.prime(TARGET_WATERMARK_FRAMES);
            let _ = stream.request_start();
            self.active = true;
        }
    }

    pub fn stop(&mut self) {
        if let Some(ref stream) = self.stream {
            let _ = stream.request_stop();
        }
        self.stream = None;
        self.active = false;
        self.ring_buffer.reset();
    }

    pub fn push_raw_gba_samples(&mut self, samples: &[i16]) -> usize {
        if !self.active || samples.is_empty() {
            return 0;
        }
        let current_frames = self.ring_buffer.available_frames();
        let ppm = self.drc.calculate_adjustment_ppm(current_frames);
        self.resampler.set_rate_adjustment(ppm);

        let mut resampled = Vec::with_capacity(samples.len());
        self.resampler.resample(samples, &mut resampled);
        self.ring_buffer.push(&resampled)
    }

    pub fn ring_buffer(&self) -> &Arc<AudioRingBuffer> {
        &self.ring_buffer
    }

    pub fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    pub fn is_active(&self) -> bool {
        self.active
    }
}

/// Motor de audio mock/headless para pruebas y compilación en sistemas anfitriones (no Android).
#[cfg(not(target_os = "android"))]
pub struct NativeAudioEngine {
    ring_buffer: Arc<AudioRingBuffer>,
    resampler: AudioResampler,
    drc: DynamicRateControl,
    sample_rate: u32,
    active: bool,
}

#[cfg(not(target_os = "android"))]
impl NativeAudioEngine {
    pub fn new(sample_rate: u32) -> Self {
        let ring_buffer = Arc::new(AudioRingBuffer::new(DEFAULT_RING_CAPACITY_FRAMES));
        let resampler = AudioResampler::new(AUDIO_RATE_GBA, sample_rate);
        let drc = DynamicRateControl::new(TARGET_WATERMARK_FRAMES, 200);

        Self {
            ring_buffer,
            resampler,
            drc,
            sample_rate,
            active: false,
        }
    }

    pub fn start(&mut self) -> Result<(), String> {
        self.ring_buffer.prime(TARGET_WATERMARK_FRAMES);
        self.active = true;
        Ok(())
    }

    pub fn pause(&mut self) {
        self.active = false;
    }

    pub fn resume(&mut self) {
        self.ring_buffer.prime(TARGET_WATERMARK_FRAMES);
        self.active = true;
    }

    pub fn stop(&mut self) {
        self.active = false;
        self.ring_buffer.reset();
    }

    pub fn push_raw_gba_samples(&mut self, samples: &[i16]) -> usize {
        if !self.active || samples.is_empty() {
            return 0;
        }
        let current_frames = self.ring_buffer.available_frames();
        let ppm = self.drc.calculate_adjustment_ppm(current_frames);
        self.resampler.set_rate_adjustment(ppm);

        let mut resampled = Vec::with_capacity(samples.len());
        self.resampler.resample(samples, &mut resampled);
        self.ring_buffer.push(&resampled)
    }

    pub fn ring_buffer(&self) -> &Arc<AudioRingBuffer> {
        &self.ring_buffer
    }

    pub fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    pub fn is_active(&self) -> bool {
        self.active
    }
}

#[cfg(test)]
#[path = "audio_tests.rs"]
mod tests;
