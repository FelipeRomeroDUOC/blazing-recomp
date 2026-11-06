use super::*;
use std::sync::Arc;
use std::time::Duration;

#[test]
fn test_emulation_bridge_single_slot_transfer() {
    let bridge = Arc::new(EmulationBridge::new());

    // Crear un paquete de prueba
    let mut rgba1 = Box::new([[0u8; 4]; GBA_PIXELS]);
    rgba1[0] = [255, 0, 0, 255];
    let packet1 = FramePacket {
        rgba: rgba1,
        frame_number: 1,
        timing: crate::runner::StepTiming {
            ejec_ms: 10.0,
            color_ms: 1.0,
            native_blocks: 100,
            interpreter_steps: 0,
            ppu_ms: 2.0,
            ppu_bg_ms: 1.0,
            ppu_sprite_ms: 0.5,
            ppu_compose_ms: 0.5,
            apu_ms: 0.5,
            pump_llamadas: 0,
            pump_muestras: 0,
            reads: 1000,
            writes: 500,
        },
        is_native: true,
        ppu_state: None,
    };

    bridge.send_frame(packet1);

    let received = bridge.try_recv_frame();
    assert!(received.is_some(), "debe recibir el fotograma disponible");
    let p = received.unwrap();
    assert_eq!(p.frame_number, 1);
    assert_eq!(p.rgba[0], [255, 0, 0, 255]);
    assert_eq!(p.timing.ejec_ms, 10.0);
    assert!(p.is_native);

    // Segundo try_recv debe ser None ya que el búfer fue consumido
    assert!(bridge.try_recv_frame().is_none());
}

#[test]
fn test_atomic_key_injection_lock_free() {
    let bridge = EmulationBridge::new();
    assert_eq!(bridge.read_keys(), 0x03FF, "teclas en reposo");

    // Pulsar botón A y B (bits 0 y 1 en cero = 0x03FC)
    bridge.set_keys(0x03FC);
    assert_eq!(bridge.read_keys(), 0x03FC);

    // Volver a reposo
    bridge.set_keys(0x03FF);
    assert_eq!(bridge.read_keys(), 0x03FF);
}

#[test]
fn test_emulation_bridge_pause_and_resume_sleep() {
    let bridge = Arc::new(EmulationBridge::new());
    assert!(!bridge.is_paused());

    bridge.set_paused(true);
    assert!(bridge.is_paused());

    // Probar en un hilo secundario que se duerma y despierte con la señal
    let b_clone = Arc::clone(&bridge);
    let handle = std::thread::spawn(move || {
        b_clone.wait_if_paused();
        42
    });

    // Dar tiempo para que el hilo entre en espera
    std::thread::sleep(Duration::from_millis(30));
    assert!(
        !handle.is_finished(),
        "el hilo debe estar dormido en la pausa"
    );

    bridge.set_paused(false);
    let res = handle.join().expect("hilo debe finalizar tras despertar");
    assert_eq!(res, 42);
}

fn deposit(bridge: &EmulationBridge, frame_number: u64) {
    let mut rgba = bridge.acquire_buffer();
    rgba[0] = [10, 20, 30, 255];
    bridge.send_frame(FramePacket {
        rgba,
        frame_number,
        timing: StepTiming::default(),
        is_native: false,
        ppu_state: None,
    });
}

#[test]
fn test_mailbox_never_blocks_producer() {
    let bridge = EmulationBridge::new();

    // Buzón con un fotograma ya depositado y sin consumidores activos.
    deposit(&bridge, 0);

    let start = std::time::Instant::now();
    for n in 1..=1000u64 {
        deposit(&bridge, n);
    }
    let elapsed = start.elapsed();

    assert!(
        elapsed < Duration::from_millis(50),
        "1000 depósitos no deben bloquearse (tomó {elapsed:?})"
    );
    // Con cola FIFO de capacidad 3, los últimos 3 fotogramas retenidos son 998, 999 y 1000.
    let f1 = bridge
        .try_recv_frame()
        .expect("el buzón debe conservar los fotogramas en cola (1/3)");
    assert_eq!(f1.frame_number, 998);

    let f2 = bridge
        .try_recv_frame()
        .expect("el buzón debe conservar los fotogramas en cola (2/3)");
    assert_eq!(f2.frame_number, 999);

    let last = bridge
        .try_recv_frame()
        .expect("el buzón debe conservar el último fotograma (3/3)");
    assert_eq!(last.frame_number, 1000, "el buzón conserva el más reciente");
    assert!(
        bridge.try_recv_frame().is_none(),
        "cola vacía tras consumir los 3 retenidos"
    );

    assert!(
        bridge.allocations() <= 4,
        "el pool debe reutilizar búferes (asignaciones={})",
        bridge.allocations()
    );
}

#[test]
fn test_mailbox_keeps_latest_frame() {
    let bridge = EmulationBridge::new();

    // Productor deposita 5 fotogramas numerados 1..5 sin que el consumidor lea.
    for n in 1..=5u64 {
        deposit(&bridge, n);
    }

    // Con cola FIFO de capacidad 3, se descartan 1 y 2, reteniendo 3, 4 y 5 en orden FIFO.
    let p1 = bridge
        .try_recv_frame()
        .expect("debe recibir el fotograma 3");
    assert_eq!(p1.frame_number, 3);
    assert_eq!(p1.rgba.len(), GBA_PIXELS);
    assert_eq!(
        p1.rgba[0],
        [10, 20, 30, 255],
        "sin pérdida de píxeles ni alfa"
    );

    let p2 = bridge
        .try_recv_frame()
        .expect("debe recibir el fotograma 4");
    assert_eq!(p2.frame_number, 4);

    let p3 = bridge
        .try_recv_frame()
        .expect("debe recibir el fotograma 5");
    assert_eq!(
        p3.frame_number, 5,
        "el último recibido es el más reciente depositado"
    );

    assert!(
        bridge.try_recv_frame().is_none(),
        "la cola debe quedar vacía tras consumir los 3 retenidos"
    );

    assert!(
        bridge.allocations() <= 4,
        "el pool reutiliza búferes (asignaciones={})",
        bridge.allocations()
    );
}

fn create_dummy_rom() -> Vec<u8> {
    let mut rom = vec![0u8; 1024 * 1024]; // 1 MB
    rom[0] = 0x2E;
    rom[1] = 0x00;
    rom[2] = 0x00;
    rom[3] = 0xEA;
    rom
}

#[test]
fn test_emulation_worker_lifecycle_and_frame_generation() {
    let mut worker = EmulationWorker::start(create_dummy_rom(), None, ScreenKind::Frontlit, None);

    // Esperar a que el worker produzca al menos 1 fotograma
    let start = std::time::Instant::now();
    let mut received_frame = None;
    while start.elapsed() < Duration::from_millis(1500) {
        if let Some(p) = worker.bridge().try_recv_frame() {
            received_frame = Some(p);
            break;
        }
        std::thread::sleep(Duration::from_millis(10));
    }

    assert!(
        received_frame.is_some(),
        "worker debe generar al menos 1 fotograma"
    );
    let packet = received_frame.unwrap();
    assert!(packet.frame_number >= 1);
    assert_eq!(packet.rgba.len(), GBA_PIXELS);

    // Inyectar teclas y comprobar que el worker las lee
    worker.bridge().set_keys(0x03FE);
    assert_eq!(worker.bridge().read_keys(), 0x03FE);

    // Detener el worker limpiamente
    worker.stop();
}

#[test]
fn test_emulation_bridge_waker_invoked_on_send_frame() {
    use std::sync::atomic::{AtomicUsize, Ordering};

    let wake_count = Arc::new(AtomicUsize::new(0));
    let count_clone = Arc::clone(&wake_count);
    let bridge = EmulationBridge::new_with_waker(Some(Arc::new(move || {
        count_clone.fetch_add(1, Ordering::SeqCst);
    })));

    assert_eq!(wake_count.load(Ordering::SeqCst), 0);

    deposit(&bridge, 1);
    assert_eq!(
        wake_count.load(Ordering::SeqCst),
        1,
        "el waker debe invocarse tras send_frame"
    );

    deposit(&bridge, 2);
    assert_eq!(
        wake_count.load(Ordering::SeqCst),
        2,
        "el waker debe invocarse en cada send_frame exitoso"
    );
}

#[test]
fn test_ppu_frame_state_pool_recycling() {
    let bridge = EmulationBridge::new();
    assert!(!bridge.is_gpu_ppu_enabled());

    bridge.set_gpu_ppu_enabled(true);
    assert!(bridge.is_gpu_ppu_enabled());

    // Adquirir un estado PPU y depositarlo
    let mut state = bridge.acquire_ppu_state();
    state.vram[0] = 0xAA;
    state.oam[0] = 0xBB;
    state.palette[0] = 0xCC;
    state.io[0] = 0xDD;

    let rgba = bridge.acquire_buffer();
    bridge.send_frame(FramePacket {
        rgba,
        frame_number: 1,
        timing: StepTiming::default(),
        is_native: true,
        ppu_state: Some(state),
    });

    // Depositar un segundo paquete sin consumir el primero
    let mut state2 = bridge.acquire_ppu_state();
    state2.vram[0] = 0x11;
    let rgba2 = bridge.acquire_buffer();
    bridge.send_frame(FramePacket {
        rgba: rgba2,
        frame_number: 2,
        timing: StepTiming::default(),
        is_native: true,
        ppu_state: Some(state2),
    });

    // Con cola FIFO de capacidad 3, se consumen en orden: primero frame 1, luego frame 2
    let recv1 = bridge
        .try_recv_frame()
        .expect("debe recibir el primer fotograma en orden FIFO");
    assert_eq!(recv1.frame_number, 1);
    let ppu1 = recv1.ppu_state.expect("debe tener ppu_state frame 1");
    assert_eq!(ppu1.vram[0], 0xAA);

    let recv2 = bridge
        .try_recv_frame()
        .expect("debe recibir el segundo fotograma en orden FIFO");
    assert_eq!(recv2.frame_number, 2);
    let ppu2 = recv2.ppu_state.expect("debe tener ppu_state frame 2");
    assert_eq!(ppu2.vram[0], 0x11);

    assert!(
        bridge.try_recv_frame().is_none(),
        "la cola debe quedar vacía tras consumir ambos fotogramas"
    );

    // Al reciclar, el pool no asigna memoria nueva y devuelve el búfer reciclado
    bridge.recycle_ppu_state(ppu2);
    let reused = bridge.acquire_ppu_state();
    assert_eq!(reused.vram[0], 0x11, "debe ser el búfer reciclado del pool");

    bridge.recycle_ppu_state(ppu1);
    let reused1 = bridge.acquire_ppu_state();
    assert_eq!(
        reused1.vram[0], 0xAA,
        "debe ser el búfer reciclado del pool"
    );
}

#[test]
fn test_emulation_worker_save_synchronization() {
    let bridge = EmulationBridge::new();
    assert!(!bridge.take_save_requested());
    bridge.request_save();
    assert!(bridge.take_save_requested());
    assert!(!bridge.take_save_requested());
}

#[test]
fn test_emulation_worker_autosave_debounce() {
    let temp_dir = std::path::PathBuf::from("target").join(format!(
        "blazing_debounce_{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let _ = std::fs::create_dir_all(&temp_dir);
    let save_path = temp_dir.join("debounce.sav");

    let rom = vec![0u8; 1024];
    let worker = EmulationWorker::start_with_save(
        rom,
        None,
        ScreenKind::Frontlit,
        None,
        None,
        Some(save_path.clone()),
        None,
    );

    worker.bridge().request_save();
    std::thread::sleep(Duration::from_millis(150));
    drop(worker);

    assert!(
        save_path.exists(),
        "El archivo de guardado debe haber sido creado por el worker"
    );
    let _ = std::fs::remove_dir_all(&temp_dir);
}

#[test]
fn test_frame_pacer_cumulative_drift_compensation() {
    let start = Instant::now();
    let target = Duration::from_nanos(16_666_667);
    let mut pacer = FramePacer::with_start_time(target, FramePacer::DEFAULT_MAX_LAG, start);

    let mut current_time = start;
    let mut total_slept = Duration::ZERO;

    for i in 0..60 {
        // Simular ejecución variable (entre 1 ms y 4 ms)
        let work = Duration::from_millis(1 + (i % 4));
        current_time += work;

        let sleep = pacer.advance(current_time);
        total_slept += sleep;

        // Simular exceso de reposo del kernel (0.8 ms extra cada frame que se duerme)
        let oversleep = if sleep > Duration::ZERO {
            Duration::from_micros(800)
        } else {
            Duration::ZERO
        };
        current_time += sleep + oversleep;
    }

    // Tras 60 pasos, el tiempo programado para el frame 61 es start + 61 * target
    let expected_next = start + target * 61;
    assert_eq!(
        pacer.next_frame_time(),
        expected_next,
        "next_frame_time debe converger exactamente al objetivo acumulativo tras 60 frames"
    );

    // Con el pacing acumulativo, current_time no debe desviarse más de 1 sobreduerme (800 us)
    let expected_target = start + target * 60;
    let drift = if current_time > expected_target {
        current_time - expected_target
    } else {
        expected_target - current_time
    };
    assert!(
        drift <= Duration::from_micros(800),
        "La deriva acumulada debe ser <= 800 us (obtenido: {:?})",
        drift
    );
}

#[test]
fn test_frame_pacer_clamp_on_large_lag() {
    let start = Instant::now();
    let target = Duration::from_nanos(16_666_667);
    let mut pacer = FramePacer::with_start_time(target, Duration::from_millis(100), start);

    // Simular que la app se suspende y pasan 250 ms (excede max_lag de 100 ms)
    let jump_time = start + Duration::from_millis(250);
    let sleep = pacer.advance(jump_time);

    // Al exceder el lag máximo, debe retornar 0 de reposo y reiniciar el ancla
    assert_eq!(sleep, Duration::ZERO);
    assert_eq!(pacer.next_frame_time(), jump_time + target);

    // El siguiente paso normal debe programar reposo normalmente sin arrastrar deuda
    let normal_step = jump_time + Duration::from_millis(2);
    let next_sleep = pacer.advance(normal_step);
    assert_eq!(next_sleep, target - Duration::from_millis(2));
    assert_eq!(pacer.next_frame_time(), jump_time + target * 2);
}
