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
