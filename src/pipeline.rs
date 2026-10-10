//! Módulo de pipeline multihilo para desacoplar la emulación del renderizado GPU.
//!
//! Permite que la CPU de GBA y el código C recompilado corran en un hilo dedicado
//! a 60 Hz regulados, depositando fotogramas completos en un puente de comunicación
//! acotado para que el hilo principal de Android los consuma y presente en la GPU
//! sin retrasos ni contención de cerrojos.

use std::sync::atomic::{AtomicBool, AtomicU16, Ordering};
use std::sync::mpsc::{sync_channel, Receiver, SyncSender, TryRecvError};
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
}

/// Puente de sincronización y transporte de fotogramas entre el hilo de emulación y el de UI.
pub struct EmulationBridge {
    tx: SyncSender<FramePacket>,
    rx: Mutex<Receiver<FramePacket>>,
    keys: AtomicU16,
    running: AtomicBool,
    paused: AtomicBool,
    pause_sync: (Mutex<bool>, Condvar),
}

impl EmulationBridge {
    /// Crea un nuevo puente con canal acotado a 1 fotograma para latencia mínima.
    pub fn new() -> Self {
        let (tx, rx) = sync_channel(1);
        Self {
            tx,
            rx: Mutex::new(rx),
            keys: AtomicU16::new(0x03FF), // Teclas GBA activas en bajo (0x03FF = todas sueltas)
            running: AtomicBool::new(true),
            paused: AtomicBool::new(false),
            pause_sync: (Mutex::new(false), Condvar::new()),
        }
    }

    /// Envía un fotograma al hilo de renderizado.
    ///
    /// Si el canal está ocupado, espera a que el consumidor lo extraiga, garantizando
    /// un ritmo coordinado de avance de fotogramas.
    pub fn send_frame(&self, packet: FramePacket) -> Result<(), ()> {
        self.tx.send(packet).map_err(|_| ())
    }

    /// Intenta recibir el fotograma más reciente de forma no bloqueante en el hilo de GPU.
    pub fn try_recv_frame(&self) -> Option<FramePacket> {
        let Ok(rx) = self.rx.lock() else { return None; };
        match rx.try_recv() {
            Ok(p) => Some(p),
            Err(TryRecvError::Empty | TryRecvError::Disconnected) => None,
        }
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
        let bridge = Arc::new(EmulationBridge::new());
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
                    runner.step_frame();

                    let timing = runner.last_timing();
                    let is_native = runner.is_native();

                    let mut buf = Box::new([[0u8; 4]; GBA_PIXELS]);
                    buf.copy_from_slice(runner.frame_rgba());

                    frame_count += 1;
                    let packet = FramePacket {
                        rgba: buf,
                        frame_number: frame_count,
                        timing,
                        is_native,
                    };

                    if b_clone.send_frame(packet).is_err() {
                        break;
                    }

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
