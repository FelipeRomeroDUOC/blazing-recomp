use super::*;

/// Genera una imagen ROM dummy de tamaño suficiente con instrucciones NOP (0x0000)
/// para inicializar la memoria de `gba-core` de forma segura.
fn create_dummy_rom() -> Vec<u8> {
    let mut rom = vec![0u8; 1024 * 1024]; // 1 MB
    // Encabezado básico de GBA: vector de salto en 0x00 y firma Nintendo dummy
    rom[0] = 0x2E; // B 0x080000C0
    rom[1] = 0x00;
    rom[2] = 0x00;
    rom[3] = 0xEA;
    rom
}

#[test]
fn test_runner_buffer_geometry_and_default_state() {
    let rom = create_dummy_rom();
    let runner = GameRunner::new(rom, None, ScreenKind::Frontlit);

    assert_eq!(runner.frame_rgba().len(), GBA_PIXELS);
    assert_eq!(runner.frame_rgba().len(), 240 * 160);
    assert_eq!(runner.frames_elapsed(), 0);
    assert_eq!(runner.keys(), 0x03FF, "por defecto ningún botón está pulsado (0x03FF)");
    assert_eq!(runner.clock(), 0);
}

#[test]
fn test_runner_injects_key_mask() {
    let rom = create_dummy_rom();
    let mut runner = GameRunner::new(rom, None, ScreenKind::Frontlit);

    // Pulsar botón A (bit 0 en bajo -> 0x03FE)
    runner.set_keys(0x03FE);
    assert_eq!(runner.keys(), 0x03FE);

    // Pulsar todos los botones (0x0000)
    runner.set_keys(0x0000);
    assert_eq!(runner.keys(), 0x0000);

    // Liberar todos los botones (0x03FF)
    runner.set_keys(0x03FF);
    assert_eq!(runner.keys(), 0x03FF);
}

#[test]
fn test_runner_alpha_channel_integrity() {
    let rom = create_dummy_rom();
    let runner = GameRunner::new(rom, None, ScreenKind::Frontlit);

    for (i, px) in runner.frame_rgba().iter().enumerate() {
        assert_eq!(
            px[3], 255,
            "el canal alfa en el píxel {i} debe ser 255 (opaco)"
        );
    }
}

#[test]
fn test_runner_clock_and_step_execution() {
    let rom = create_dummy_rom();
    let mut runner = GameRunner::new(rom, None, ScreenKind::Frontlit);

    let initial_clock = runner.clock();
    let _ = runner.step_frame();

    assert!(
        runner.clock() > initial_clock,
        "el reloj maestro de la consola debe avanzar tras step_frame (inicial: {initial_clock}, actual: {})",
        runner.clock()
    );
}

#[test]
fn test_runner_screen_kind_switch() {
    let rom = create_dummy_rom();
    let mut runner = GameRunner::new(rom, None, ScreenKind::Frontlit);
    assert_eq!(runner.screen_kind, ScreenKind::Frontlit);

    runner.set_screen_kind(ScreenKind::Backlit);
    assert_eq!(runner.screen_kind, ScreenKind::Backlit);
    assert_eq!(runner.frame_rgba().len(), GBA_PIXELS);

    runner.set_screen_kind(ScreenKind::Classic);
    assert_eq!(runner.screen_kind, ScreenKind::Classic);
    assert_eq!(runner.frame_rgba().len(), GBA_PIXELS);
}

#[test]
fn test_runner_supports_block_table_injection() {
    // Ejecuta un fotograma completo por el intérprete de respaldo, así que escribe en el
    // acumulador global de hotspots: se serializa con el resto de tests que lo consultan.
    let _guard = crate::recomp::LOCK_ACUMULADORES
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    let rom = create_dummy_rom();
    let mut runner = GameRunner::new(rom, None, ScreenKind::Frontlit);
    assert!(runner.block_table.is_none());

    // Inyectar tabla vacía
    runner.set_block_table(Some(crate::recomp::BlockTable::empty()));
    assert!(runner.block_table.is_some());

    // Debe avanzar fotograma ejecutando con fallback limpio
    let initial_frames = runner.frames_elapsed();
    let _ = runner.step_frame();
    assert_eq!(runner.frames_elapsed(), initial_frames + 1);
}

/// El reparto de líneas que el motor mide por fotograma tiene que llegar al acumulador del port:
/// sin este cableado la línea `# ppucamino` de una sesión en Android saldría en cero y no diría
/// por qué no se está usando la vía rápida. Se mide por diferencia y con el candado de los
/// acumuladores, porque el resto de tests que avanzan fotogramas escriben en el mismo estado global.
#[test]
fn test_runner_vuelca_el_reparto_de_camino_de_la_ppu() {
    let _guard = crate::recomp::LOCK_ACUMULADORES
        .lock()
        .unwrap_or_else(|e| e.into_inner());

    let rom = create_dummy_rom();
    let mut runner = GameRunner::new(rom, None, ScreenKind::Frontlit);

    let antes = crate::recomp::ppu_path_counters();
    let _ = runner.step_frame();
    let despues = crate::recomp::ppu_path_counters();

    let rectas = despues.rectas - antes.rectas;
    let efectos = (despues.mezcla - antes.mezcla)
        + (despues.ventana - antes.ventana)
        + (despues.semi - antes.semi);

    assert!(
        rectas >= 160,
        "un fotograma completo (160 líneas) sin efectos de color debe volcarse como líneas rectas \
         al acumulador del port (obtenido: {rectas})"
    );
    assert_eq!(
        efectos, 0,
        "un fotograma sin efectos de color no debe contar ningún descarte (obtenido: {efectos})"
    );
    // Sin `BLDCNT` activo ninguna línea es "inerte" ni aplica transformación: son las dos
    // cuentas que distinguen una línea recta normal de una que lo es por un efecto identidad.
    assert_eq!(
        despues.inertes - antes.inertes,
        0,
        "un fotograma sin efecto de color no puede tener líneas inertes"
    );
    assert_eq!(
        despues.efecto - antes.efecto,
        0,
        "un fotograma sin efecto de color no puede aplicar transformaciones"
    );
}

#[test]
fn test_runner_timing_includes_hardware_counters() {
    let mut runner = GameRunner::new(create_dummy_rom(), None, ScreenKind::Frontlit);
    runner.machine.bus.perf_audio_timing = true;
    runner.step_frame();

    let timing = runner.last_timing();
    assert!(timing.ppu_ms > 0.0, "debe registrar tiempo de PPU (obtenido: {} ms)", timing.ppu_ms);
    assert!(timing.apu_ms > 0.0, "debe registrar tiempo de APU (obtenido: {} ms)", timing.apu_ms);
    assert!(timing.pump_llamadas > 0, "debe registrar llamadas de pump_audio (obtenido: {})", timing.pump_llamadas);
    assert!(timing.pump_muestras > 0, "debe registrar muestras de audio (obtenido: {})", timing.pump_muestras);
    assert!(timing.reads > 0, "debe registrar lecturas de memoria (obtenido: {})", timing.reads);
}

#[test]
fn test_runner_timing_default_audio_timing_disabled() {
    let mut runner = GameRunner::new(create_dummy_rom(), None, ScreenKind::Frontlit);
    assert!(
        !runner.machine.bus.perf_audio_timing,
        "por defecto el flag de audio debe estar inactivo"
    );
    runner.step_frame();

    let timing = runner.last_timing();
    assert_eq!(
        timing.apu_ms, 0.0,
        "con el flag apagado no debe registrar tiempo de APU (coste 0 ns de reloj)"
    );
    assert!(
        timing.pump_llamadas > 0,
        "debe seguir contando llamadas a pump_audio"
    );
    assert!(
        timing.pump_muestras > 0,
        "debe seguir contando muestras generadas de audio"
    );
    assert!(timing.ppu_ms > 0.0, "debe seguir registrando tiempo de PPU normalmente");
    assert!(timing.reads > 0, "debe seguir registrando lecturas de memoria");
}

#[test]
fn test_runner_gpu_ppu_toggle_and_memory_accessors() {
    let dummy_rom = create_dummy_rom();
    let mut runner = GameRunner::new(dummy_rom, None, ScreenKind::Frontlit);

    // 1. Por defecto está desactivado
    assert!(!runner.is_gpu_ppu_enabled());

    // 2. Activación
    runner.set_gpu_ppu_enabled(true);
    assert!(runner.is_gpu_ppu_enabled());

    // 3. Accesores de memoria de video para PPU Shim
    assert_eq!(runner.vram().len(), 96 * 1024, "VRAM debe ser 96 KB");
    assert_eq!(runner.oam().len(), 1024, "OAM debe ser 1 KB");
    assert_eq!(runner.palette().len(), 1024, "Palette debe ser 1 KB");
    assert_eq!(runner.io().len(), 1024, "IO debe ser 1 KB");
}

