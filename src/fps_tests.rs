use super::*;

#[test]
fn test_fps_counter_frame_accumulation_and_calculation() {
    let mut counter = FpsCounter::new().expect("FpsCounter debe inicializarse correctamente");

    let t0 = Instant::now();
    // Simular 60 fotogramas distribuidos en 1 segundo (intervalos de 16.66 ms)
    for i in 1..=60 {
        let frame_time = t0 + Duration::from_micros(i * 16_666);
        counter.on_frame_at(frame_time);
    }

    let calculated_fps = counter.fps();
    assert!(
        calculated_fps >= 58.0 && calculated_fps <= 62.0,
        "FPS calculado ({calculated_fps}) debe estar en el rango [58.0, 62.0]"
    );
}

#[test]
fn test_fps_badge_rasterization_uses_assets_font() {
    let mut counter = FpsCounter::new().expect("FpsCounter debe inicializarse correctamente");

    counter.render_badge(60);
    let buffer = counter.badge_rgba();
    assert_eq!(
        buffer.len(),
        BADGE_WIDTH * BADGE_HEIGHT,
        "El búfer debe tener exactamente {} píxeles",
        BADGE_WIDTH * BADGE_HEIGHT
    );

    // Contar píxeles con componente de color verde (#4ADE80: r=74, g=222, b=128)
    let text_pixels = buffer
        .iter()
        .filter(|px| px[1] > 180 && px[3] > 100) // g > 180 (verde brillante) y alfa opaco
        .count();

    assert!(
        text_pixels > 20,
        "Debe haber píxeles de texto visibles renderizados con la tipografía de assets (encontrados: {text_pixels})"
    );
}

#[test]
fn test_fps_counter_throttles_rasterization() {
    let mut counter = FpsCounter::new().expect("FpsCounter debe inicializarse");
    counter.mark_clean();
    assert!(!counter.is_dirty(), "Debe estar limpio tras mark_clean");

    let t0 = Instant::now();
    // Avanzar sólo 10 ms (menos del umbral de 350 ms)
    counter.on_frame_at(t0 + Duration::from_millis(10));
    assert!(
        !counter.is_dirty(),
        "No debe marcarse dirty ni re-rasterizar antes de expirar el intervalo de medición"
    );
}

#[test]
fn test_fps_badge_mode_switch_triggers_dirty() {
    let mut counter = FpsCounter::new().expect("FpsCounter debe inicializarse");
    assert!(counter.is_native());
    counter.mark_clean();
    assert!(!counter.is_dirty());

    counter.set_native(false);
    assert!(!counter.is_native());
    assert!(counter.is_dirty(), "Cambiar a INT debe marcar dirty para actualizar textura");

    counter.mark_clean();
    counter.set_native(false);
    assert!(!counter.is_dirty(), "Mismo modo no debe marcar dirty");

    counter.set_native(true);
    assert!(counter.is_native());
    assert!(counter.is_dirty(), "Cambiar a AOT debe marcar dirty");
}

#[test]
fn test_hud_shader_wgsl_validates() {
    let shader_source = crate::overlay_shader::HUD_BLIT_WGSL;
    assert!(
        !shader_source.is_empty(),
        "HUD_BLIT_WGSL no debe estar vacío"
    );

    let module = naga::front::wgsl::parse_str(shader_source);
    match module {
        Ok(m) => {
            let mut validator = naga::valid::Validator::new(
                naga::valid::ValidationFlags::all(),
                naga::valid::Capabilities::empty(),
            );
            let info = validator.validate(&m);
            assert!(
                info.is_ok(),
                "HUD_BLIT_WGSL debe validar con Naga: {:?}",
                info.err()
            );
        }
        Err(e) => {
            panic!("Error de sintaxis WGSL en HUD_BLIT_WGSL: {e}");
        }
    }
}

#[test]
fn test_fps_badge_renderer_switch_triggers_dirty() {
    let mut counter = FpsCounter::new().expect("FpsCounter debe inicializarse");
    assert_eq!(counter.renderer(), "Vulkan");
    counter.mark_clean();
    assert!(!counter.is_dirty());

    counter.set_renderer("OpenGL");
    assert_eq!(counter.renderer(), "OpenGL");
    assert!(counter.is_dirty(), "Cambiar renderer a OpenGL debe marcar dirty");

    counter.mark_clean();
    counter.set_renderer("OpenGL");
    assert!(!counter.is_dirty(), "Mismo renderer no debe marcar dirty");

    counter.set_renderer("Vulkan");
    assert_eq!(counter.renderer(), "Vulkan");
    assert!(counter.is_dirty(), "Cambiar renderer a Vulkan debe marcar dirty");
}

#[test]
fn test_fps_counter_on_frames_batches_correctly() {
    let mut counter = FpsCounter::new().expect("FpsCounter debe inicializarse");
    let t0 = Instant::now();

    // 30 llamadas batch de 2 frames cada una (60 frames en total en 1 segundo)
    for i in 1..=30 {
        let frame_time = t0 + Duration::from_micros(i * 33_333);
        counter.on_frames_at(2, frame_time);
    }

    let calculated_fps = counter.fps();
    assert!(
        calculated_fps >= 58.0 && calculated_fps <= 62.0,
        "FPS por lotes ({calculated_fps}) debe reflejar 60 GBA frames en [58.0, 62.0]"
    );
}

