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

#[test]
fn test_telemetria_dragon_detecta_anomalia_bldcnt() {
    let mut io_lineas = [[0u8; 0x60]; 160];
    let dispcnt_dragon = 0x0D00u16.to_le_bytes();
    let bldcnt_anomalo = 0x0144u16; // Blending apagado para BG3 / modo normal
    for py in 0..160 {
        io_lineas[py][0x00] = dispcnt_dragon[0];
        io_lineas[py][0x01] = dispcnt_dragon[1];
        let bytes = bldcnt_anomalo.to_le_bytes();
        io_lineas[py][0x50] = bytes[0];
        io_lineas[py][0x51] = bytes[1];
    }

    let mut io_fin = [0u8; 256];
    let bytes_fin = bldcnt_anomalo.to_le_bytes();
    io_fin[0x50] = bytes_fin[0];
    io_fin[0x51] = bytes_fin[1];

    let mut monitor = MonitorRasterDragon::new();
    let diag = monitor.registrar_fotograma(770, Some(&io_lineas), &io_fin);

    assert!(
        diag.hay_anomalia,
        "Debe detectar la anomalía de blending ausente"
    );
    assert_eq!(diag.bldcnt_linea50, 0x0144);
    assert_eq!(diag.bldcnt_fin, 0x0144);
    assert_eq!(diag.lineas_blend, 0);
    assert!(diag.tiene_io_lineas);
    assert_eq!(monitor.anomalias_detectadas, 1);
}

#[test]
fn test_telemetria_dragon_silenciosa_en_fotograma_normal() {
    let mut io_lineas = [[0u8; 0x60]; 160];
    let dispcnt_dragon = 0x0D00u16.to_le_bytes();
    for py in 0..160 {
        io_lineas[py][0x00] = dispcnt_dragon[0];
        io_lineas[py][0x01] = dispcnt_dragon[1];
    }
    let bldcnt_fuego = 0x3F68u16; // Modo alfa (01b) con 1st target BG3 (bit 3)
    for py in 0..100 {
        let bytes = bldcnt_fuego.to_le_bytes();
        io_lineas[py][0x50] = bytes[0];
        io_lineas[py][0x51] = bytes[1];
    }
    let bldcnt_fin = 0x3F61u16;
    for py in 100..160 {
        let bytes = bldcnt_fin.to_le_bytes();
        io_lineas[py][0x50] = bytes[0];
        io_lineas[py][0x51] = bytes[1];
    }

    let mut io_fin = [0u8; 256];
    let bytes_fin = bldcnt_fin.to_le_bytes();
    io_fin[0x50] = bytes_fin[0];
    io_fin[0x51] = bytes_fin[1];

    let mut monitor = MonitorRasterDragon::new();
    let diag = monitor.registrar_fotograma(769, Some(&io_lineas), &io_fin);

    assert!(
        !diag.hay_anomalia,
        "Fotograma con fuego normal no debe ser anómalo"
    );
    assert_eq!(diag.bldcnt_linea50, 0x3F68);
    assert_eq!(diag.bldcnt_fin, 0x3F61);
    assert_eq!(diag.lineas_blend, 100);
    assert!(diag.tiene_io_lineas);
    assert_eq!(monitor.anomalias_detectadas, 0);
}

#[test]
fn test_telemetria_dragon_ignora_escenas_ordinarias() {
    let mut io_lineas = [[0u8; 0x60]; 160];
    // Escena fuera de la cinemática del dragón (por ejemplo menú con DISPCNT = 0x1000 solo OBJ o 0x0000)
    let dispcnt_menu = 0x1000u16.to_le_bytes();
    for py in 0..160 {
        io_lineas[py][0x00] = dispcnt_menu[0];
        io_lineas[py][0x01] = dispcnt_menu[1];
    }

    // BLDCNT sin modo alfa (en la cinemática esto dispararía anomalía)
    let bldcnt_anomalo = 0x0144u16;
    for py in 0..160 {
        let bytes = bldcnt_anomalo.to_le_bytes();
        io_lineas[py][0x50] = bytes[0];
        io_lineas[py][0x51] = bytes[1];
    }

    let mut io_fin = [0u8; 256];
    let bytes_fin = bldcnt_anomalo.to_le_bytes();
    io_fin[0x50] = bytes_fin[0];
    io_fin[0x51] = bytes_fin[1];

    let mut monitor = MonitorRasterDragon::new();
    let diag = monitor.registrar_fotograma(10, Some(&io_lineas), &io_fin);

    assert!(
        !diag.hay_anomalia,
        "No debe reportar anomalía fuera de la cinemática del dragón"
    );
    assert_eq!(diag.bldcnt_linea50, 0x0144);
    assert_eq!(diag.bldcnt_fin, 0x0144);
    assert_eq!(diag.lineas_blend, 0);
    assert!(diag.tiene_io_lineas);
    assert_eq!(monitor.anomalias_detectadas, 0);

    // Caso pinturas estáticas del prólogo: DISPCNT = 0x0C00 (coincide con la firma) pero bldcnt == 0x0000
    let mut io_lineas_prologo = [[0u8; 0x60]; 160];
    let dispcnt_prologo = 0x0C00u16.to_le_bytes();
    for py in 0..160 {
        io_lineas_prologo[py][0x00] = dispcnt_prologo[0];
        io_lineas_prologo[py][0x01] = dispcnt_prologo[1];
        // bldcnt queda en 0x0000
    }
    let io_fin_prologo = [0u8; 256];
    let diag_prologo = monitor.registrar_fotograma(11, Some(&io_lineas_prologo), &io_fin_prologo);
    assert!(
        !diag_prologo.hay_anomalia,
        "Pintura estática del prólogo con bldcnt == 0 no debe reportar anomalía"
    );
    assert_eq!(diag_prologo.bldcnt_linea50, 0x0000);
    assert_eq!(diag_prologo.bldcnt_fin, 0x0000);
    assert_eq!(diag_prologo.lineas_blend, 0);
    assert_eq!(monitor.anomalias_detectadas, 0);
}

#[test]
fn test_drenado_acumulador_anomalias_dragon() {
    let bridge = EmulationBridge::new();
    let diag = DiagnosticoDragon {
        hay_anomalia: true,
        bldcnt_linea50: 0x0144,
        bldcnt_fin: 0x0144,
        lineas_blend: 0,
        tiene_io_lineas: true,
    };

    bridge.record_dragon_anomalia(diag);

    let stats = bridge.drain_dragon_anomalias();
    assert_eq!(stats.count, 1);
    assert_eq!(stats.bldcnt_l50, 0x0144);
    assert_eq!(stats.bldcnt_fin, 0x0144);
    assert_eq!(stats.blend_lines, 0);

    // Segundo drenado debe estar limpio a cero
    let stats2 = bridge.drain_dragon_anomalias();
    assert_eq!(stats2.count, 0);
}

#[test]
fn test_drenado_dragon_sin_anomalias() {
    let bridge = EmulationBridge::new();
    let stats = bridge.drain_dragon_anomalias();
    assert_eq!(stats.count, 0);
    assert_eq!(stats.bldcnt_l50, 0);
    assert_eq!(stats.bldcnt_fin, 0);
    assert_eq!(stats.blend_lines, 0);
}
