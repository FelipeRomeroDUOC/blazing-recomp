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
        },
        is_native: true,
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
    assert!(!handle.is_finished(), "el hilo debe estar dormido en la pausa");

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
    let last = bridge
        .try_recv_frame()
        .expect("el buzón debe conservar el último fotograma");
    assert_eq!(last.frame_number, 1000, "el buzón conserva el más reciente");
    assert!(
        bridge.allocations() <= 2,
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

    let p = bridge
        .try_recv_frame()
        .expect("debe recibir el fotograma más reciente");
    assert_eq!(p.frame_number, 5, "el consumidor recibe el más reciente");
    assert_eq!(p.rgba.len(), GBA_PIXELS);
    assert_eq!(p.rgba[0], [10, 20, 30, 255], "sin pérdida de píxeles ni alfa");
    assert!(
        bridge.allocations() <= 2,
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

    assert!(received_frame.is_some(), "worker debe generar al menos 1 fotograma");
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

