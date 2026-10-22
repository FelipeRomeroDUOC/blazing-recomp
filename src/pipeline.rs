//! Módulo de pipeline multihilo para desacoplar la emulación del renderizado GPU.
//!
//! Permite que la CPU de GBA y el código C recompilado corran en un hilo dedicado
//! a 60 Hz regulados, depositando fotogramas completos en un puente de comunicación
//! acotado para que el hilo principal de Android los consuma y presente en la GPU
//! sin retrasos ni contención de cerrojos.

use std::sync::atomic::{AtomicBool, AtomicU16, AtomicU64, Ordering};
use std::sync::{Arc, Condvar, Mutex};
use std::time::{Duration, Instant};

use crate::runner::{GameRunner, StepTiming};
use screen::ScreenKind;

/// Ancho nativo del panel LCD de Game Boy Advance en píxeles.
pub const GBA_W: usize = 240;
/// Alto nativo del panel LCD de Game Boy Advance en píxeles.
pub const GBA_H: usize = 160;
/// Total de píxeles en un fotograma de Game Boy Advance.
pub const GBA_PIXELS: usize = GBA_W * GBA_H;

/// Duración exacta de un fotograma de GBA a 60 FPS (16.666 ms).
pub const FRAME_DURATION: Duration = Duration::from_nanos(16_666_667);

/// Estado de memoria de video para renderizado acelerado en GPU (PPU Shim).
#[derive(Clone)]
pub struct PpuFrameState {
    pub vram: Box<[u8; 96 * 1024]>,
    pub oam: Box<[u8; 1024]>,
    pub palette: Box<[u8; 1024]>,
    pub io: Box<[u8; 256]>,
}

impl PpuFrameState {
    pub fn new() -> Self {
        Self {
            vram: Box::new([0u8; 96 * 1024]),
            oam: Box::new([0u8; 1024]),
            palette: Box::new([0u8; 1024]),
            io: Box::new([0u8; 256]),
        }
    }
}

impl Default for PpuFrameState {
    fn default() -> Self {
        Self::new()
    }
}

/// Paquete inmutable transferido desde el hilo de emulación al hilo de renderizado.
#[derive(Clone)]
pub struct FramePacket {
    /// Búfer de píxeles RGBA en memoria heap (153.6 KB) para evitar sobrecargas de pila.
    pub rgba: Box<[[u8; 4]; GBA_PIXELS]>,
    /// Contador monótono de fotograma generado.
    pub frame_number: u64,
    /// Desglose de telemetría medido durante el paso de emulación.
    pub timing: StepTiming,
    /// Indica si el fotograma ejecutó bloques C recompilados nativamente.
    pub is_native: bool,
    /// Volcado de memoria de video si el renderizado acelerado en GPU (PpuShim) está activo.
    pub ppu_state: Option<PpuFrameState>,
}

/// Puente de sincronización y transporte de fotogramas entre el hilo de emulación y el de UI.
///
/// Usa un buzón de un solo slot (*single-slot mailbox*) en lugar de un canal acotado
/// bloqueante: el productor nunca espera a la GPU. Si el consumidor no ha retirado el
/// fotograma anterior, el buzón lo sobrescribe con el más reciente y devuelve su búfer
/// al pool, de modo que no se asigna memoria nueva por fotograma.
/// Notificador invocable al depositar un fotograma para despertar al hilo de presentación.
pub type WakeFn = Arc<dyn Fn() + Send + Sync>;

pub struct EmulationBridge {
    /// Buzón con el fotograma más reciente pendiente de consumir.
    mailbox: Mutex<Option<FramePacket>>,
    /// Pool de búferes RGBA reutilizables (evita `Box::new` por fotograma).
    buffer_pool: Mutex<Vec<Box<[[u8; 4]; GBA_PIXELS]>>>,
    /// Pool de volcados de memoria PPU reutilizables.
    ppu_pool: Mutex<Vec<PpuFrameState>>,
    /// Contador de asignaciones reales de búfer (diagnóstico de reutilización).
    buffer_allocs: AtomicU64,
    keys: AtomicU16,
    running: AtomicBool,
    paused: AtomicBool,
    gpu_ppu: AtomicBool,
    pause_sync: (Mutex<bool>, Condvar),
    /// Notificador opcional para interrumpir la espera del hilo principal.
    waker: Mutex<Option<WakeFn>>,
}

impl EmulationBridge {
    /// Crea un nuevo puente con buzón de un solo slot para latencia mínima.
    pub fn new() -> Self {
        Self::new_with_waker(None)
    }

    /// Crea un nuevo puente con un notificador de eventos opcional.
    pub fn new_with_waker(waker: Option<WakeFn>) -> Self {
        Self {
            mailbox: Mutex::new(None),
            buffer_pool: Mutex::new(Vec::with_capacity(2)),
            ppu_pool: Mutex::new(Vec::with_capacity(2)),
            buffer_allocs: AtomicU64::new(0),
            keys: AtomicU16::new(0x03FF), // Teclas GBA activas en bajo (0x03FF = todas sueltas)
            running: AtomicBool::new(true),
            paused: AtomicBool::new(false),
            gpu_ppu: AtomicBool::new(false),
            pause_sync: (Mutex::new(false), Condvar::new()),
            waker: Mutex::new(waker),
        }
    }

    /// Configura o reemplaza el notificador de despertar del puente.
    pub fn set_waker(&self, waker: Option<WakeFn>) {
        if let Ok(mut slot) = self.waker.lock() {
            *slot = waker;
        }
    }

    /// Adquiere un búfer de fotograma reutilizable del pool.
    ///
    /// Solo asigna memoria nueva (`Box::new`) si el pool está vacío; en régimen
    /// permanente el pool recicla los búferes que el buzón va liberando.
    pub fn acquire_buffer(&self) -> Box<[[u8; 4]; GBA_PIXELS]> {
        if let Ok(mut pool) = self.buffer_pool.lock() {
            if let Some(buf) = pool.pop() {
                return buf;
            }
        }
        self.buffer_allocs.fetch_add(1, Ordering::Relaxed);
        Box::new([[0u8; 4]; GBA_PIXELS])
    }

    /// Devuelve un búfer al pool para su reutilización.
    ///
    /// El pool se acota para no retener memoria indefinidamente si un consumidor
    /// lento dejara de reciclar.
    pub fn recycle_buffer(&self, buf: Box<[[u8; 4]; GBA_PIXELS]>) {
        if let Ok(mut pool) = self.buffer_pool.lock() {
            if pool.len() < 4 {
                pool.push(buf);
            }
        }
    }

    /// Adquiere un contenedor de estado PPU reutilizable del pool.
    pub fn acquire_ppu_state(&self) -> PpuFrameState {
        if let Ok(mut pool) = self.ppu_pool.lock() {
            if let Some(state) = pool.pop() {
                return state;
            }
        }
        PpuFrameState::new()
    }

    /// Devuelve un contenedor de estado PPU al pool para su reutilización.
    pub fn recycle_ppu_state(&self, state: PpuFrameState) {
        if let Ok(mut pool) = self.ppu_pool.lock() {
            if pool.len() < 4 {
                pool.push(state);
            }
        }
    }

    /// Consulta si el modo de renderizado acelerado en GPU (PpuShim) está activo.
    #[inline]
    pub fn is_gpu_ppu_enabled(&self) -> bool {
        self.gpu_ppu.load(Ordering::Acquire)
    }

    /// Activa o desactiva el renderizado acelerado en GPU (PpuShim).
    #[inline]
    pub fn set_gpu_ppu_enabled(&self, enabled: bool) {
        self.gpu_ppu.store(enabled, Ordering::Release);
    }

    /// Deposita el fotograma más reciente en el buzón **sin bloquearse**.
    ///
    /// Si ya había un fotograma pendiente, se descarta y su búfer vuelve al pool:
    /// el consumidor recibirá siempre el fotograma más nuevo.
    pub fn send_frame(&self, packet: FramePacket) {
        let displaced = match self.mailbox.lock() {
            Ok(mut slot) => slot.replace(packet),
            Err(_) => return,
        };
        if let Some(old) = displaced {
            self.recycle_buffer(old.rgba);
            if let Some(ppu) = old.ppu_state {
                self.recycle_ppu_state(ppu);
            }
        }
        if let Ok(guard) = self.waker.lock() {
            if let Some(ref wake) = *guard {
                wake();
            }
        }
    }

    /// Retira el fotograma más reciente del buzón de forma no bloqueante (hilo de GPU).
    ///
    /// Devuelve `None` si no hay ningún fotograma pendiente.
    pub fn try_recv_frame(&self) -> Option<FramePacket> {
        match self.mailbox.lock() {
            Ok(mut slot) => slot.take(),
            Err(_) => None,
        }
    }

    /// Número de asignaciones reales de búfer realizadas desde el arranque.
    ///
    /// En régimen permanente debe permanecer constante (pool reciclando); se usa
    /// para verificar la ausencia de asignaciones por fotograma.
    pub fn allocations(&self) -> u64 {
        self.buffer_allocs.load(Ordering::Relaxed)
    }

    /// Actualiza la máscara de entrada (llamada desde el hilo de UI/touch).
    #[inline]
    pub fn set_keys(&self, keys: u16) {
        self.keys.store(keys, Ordering::Release);
    }

    /// Lee la máscara de entrada actual (llamada desde el hilo de emulación).
    #[inline]
    pub fn read_keys(&self) -> u16 {
        self.keys.load(Ordering::Acquire)
    }

    /// Consulta si el hilo de emulación debe continuar en ejecución.
    #[inline]
    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::Acquire)
    }

    /// Detiene el hilo de emulación y despierta cualquier espera en pausa.
    pub fn stop(&self) {
        self.running.store(false, Ordering::Release);
        let (lock, cvar) = &self.pause_sync;
        if let Ok(mut guard) = lock.lock() {
            *guard = false;
            cvar.notify_all();
        }
    }

    /// Activa o desactiva el estado de pausa (reposo energético).
    pub fn set_paused(&self, paused: bool) {
        self.paused.store(paused, Ordering::Release);
        let (lock, cvar) = &self.pause_sync;
        if let Ok(mut guard) = lock.lock() {
            *guard = paused;
            if !paused {
                cvar.notify_all();
            }
        }
    }

    /// Consulta si la emulación está en pausa.
    #[inline]
    pub fn is_paused(&self) -> bool {
        self.paused.load(Ordering::Acquire)
    }

    /// Si está en pausa, bloquea el hilo actual en el Condvar hasta que se reanude o se detenga.
    pub fn wait_if_paused(&self) {
        let (lock, cvar) = &self.pause_sync;
        if let Ok(mut guard) = lock.lock() {
            while *guard && self.is_running() {
                match cvar.wait(guard) {
                    Ok(g) => guard = g,
                    Err(poisoned) => {
                        guard = poisoned.into_inner();
                    }
                }
            }
        }
    }
}

impl Default for EmulationBridge {
    fn default() -> Self {
        Self::new()
    }
}

/// Gestor y controlador del hilo de trabajo de emulación en segundo plano.
pub struct EmulationWorker {
    bridge: Arc<EmulationBridge>,
    handle: Option<std::thread::JoinHandle<()>>,
}

impl EmulationWorker {
    /// Inicia un nuevo hilo de trabajo dedicado para ejecutar el GameRunner.
    pub fn start(
        rom: Vec<u8>,
        bios: Option<Vec<u8>>,
        screen_kind: ScreenKind,
        block_table: Option<crate::recomp::BlockTable>,
    ) -> Self {
        Self::start_with_waker(rom, bios, screen_kind, block_table, None)
    }

    /// Inicia un nuevo hilo de trabajo dedicado con un notificador de despertar.
    pub fn start_with_waker(
        rom: Vec<u8>,
        bios: Option<Vec<u8>>,
        screen_kind: ScreenKind,
        block_table: Option<crate::recomp::BlockTable>,
        waker: Option<WakeFn>,
    ) -> Self {
        let bridge = Arc::new(EmulationBridge::new_with_waker(waker));
        let b_clone = Arc::clone(&bridge);

        let handle = std::thread::Builder::new()
            .name("blazing-emu".to_string())
            .spawn(move || {
                let mut runner = GameRunner::new(rom, bios.as_deref(), screen_kind);
                if let Some(table) = block_table {
                    runner.set_block_table(Some(table));
                }

                let mut frame_count = 0u64;

                while b_clone.is_running() {
                    b_clone.wait_if_paused();
                    if !b_clone.is_running() {
                        break;
                    }

                    let t_start = Instant::now();
                    let keys = b_clone.read_keys();
                    runner.set_keys(keys);

                    let gpu_ppu = b_clone.is_gpu_ppu_enabled();
                    if runner.is_gpu_ppu_enabled() != gpu_ppu {
                        runner.set_gpu_ppu_enabled(gpu_ppu);
                    }

                    runner.step_frame();

                    let timing = runner.last_timing();
                    let is_native = runner.is_native();

                    // Búfer reciclado del pool: sin asignación dinámica por fotograma.
                    let mut buf = b_clone.acquire_buffer();
                    buf.copy_from_slice(runner.frame_rgba());

                    let ppu_state = if gpu_ppu {
                        let mut state = b_clone.acquire_ppu_state();
                        state.vram.copy_from_slice(runner.vram());
                        state.oam.copy_from_slice(runner.oam());
                        state.palette.copy_from_slice(runner.palette());
                        let io_slice = runner.io();
                        let io_len = io_slice.len().min(256);
                        state.io[..io_len].copy_from_slice(&io_slice[..io_len]);
                        Some(state)
                    } else {
                        None
                    };

                    frame_count += 1;
                    let packet = FramePacket {
                        rgba: buf,
                        frame_number: frame_count,
                        timing,
                        is_native,
                        ppu_state,
                    };

                    // Depósito no bloqueante: el productor nunca espera a la GPU.
                    b_clone.send_frame(packet);

                    let elapsed = t_start.elapsed();
                    if elapsed < FRAME_DURATION {
                        std::thread::sleep(FRAME_DURATION - elapsed);
                    }
                }
            })
            .ok();

        Self { bridge, handle }
    }

    /// Acceso al puente de sincronización del worker.
    pub fn bridge(&self) -> &Arc<EmulationBridge> {
        &self.bridge
    }

    /// Detiene y espera a que el hilo finalice de forma limpia.
    pub fn stop(&mut self) {
        self.bridge.stop();
        if let Some(h) = self.handle.take() {
            let _ = h.join();
        }
    }
}

impl Drop for EmulationWorker {
    fn drop(&mut self) {
        self.stop();
    }
}

#[cfg(test)]
#[path = "pipeline_tests.rs"]
mod tests;
