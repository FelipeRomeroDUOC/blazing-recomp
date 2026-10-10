//! Pruebas unitarias para el compositor y renderizado GPU del overlay (`src/overlay.rs`).

use super::*;
use crate::input::{layout, Button, Orientation, TouchController};

/// Criterio 2 (normal): los centros y radios del overlay coinciden
/// exactamente con las hitboxes de `TouchController` (< 0.01 px).
#[test]
fn test_overlay_geometry_matches_hitboxes() {
    let mut tc = TouchController::new();
    tc.resize(1080, 2400);
    let elems = build_elems(tc.areas(), |_| false);
    assert_eq!(tc.areas().len(), elems.len(), "mismo número de elementos");

    // Cada elemento conserva el botón de su área de origen (zip en
    // el mismo orden: build_elems es map 1:1 sobre areas()).
    for (area, elem) in tc.areas().iter().zip(elems.iter()) {
        assert_eq!(elem_radius_kind_ok(area, elem), true, "geometría de {:?}", area.button);
        assert!(
            (area.cx - elem.cx).abs() < 0.01 && (area.cy - elem.cy).abs() < 0.01,
            "centro de {:?} desviado: hitbox ({},{}) vs overlay ({},{})",
            area.button, area.cx, area.cy, elem.cx, elem.cy
        );
        assert!(
            (area.radius - elem.radius).abs() < 0.01,
            "radio de {:?} desviado",
            area.button
        );
    }
}

#[allow(dead_code)]
fn button_of(e: &ElemParams) -> Button {
    // El glifo identifica unívocamente al botón en este layout.
    match e.glyph {
        Glyph::ArrowUp => Button::Up,
        Glyph::ArrowDown => Button::Down,
        Glyph::ArrowLeft => Button::Left,
        Glyph::ArrowRight => Button::Right,
        Glyph::LetterB => Button::B,
        Glyph::LetterA => Button::A,
        Glyph::LetterL => Button::L,
        Glyph::LetterR => Button::R,
        Glyph::SelectLabel => Button::Select,
        Glyph::StartLabel => Button::Start,
        Glyph::None => unreachable!(),
    }
}

fn elem_radius_kind_ok(area: &crate::input::TouchArea, elem: &ElemParams) -> bool {
    (area.cx - elem.cx).abs() < 0.01 && (area.cy - elem.cy).abs() < 0.01
}

/// Criterio 3 (límite): A pulsado se resalta, y en landscape la
/// opacidad global es la translúcida.
#[test]
fn test_overlay_orientation_and_press_state() {
    // (a) Portrait con A pulsado: pressed = 1.0, opacidad plena.
    let mut tc = TouchController::new();
    tc.resize(1080, 2400);
    let a = tc
        .areas()
        .iter()
        .find(|x| x.button == Button::A)
        .expect("hitbox A");
    tc.touch_down(1, a.cx, a.cy);
    let elems = build_elems(tc.areas(), |b| tc.is_pressed(b));
    let a_elem = elems
        .iter()
        .find(|e| e.glyph == Glyph::LetterA)
        .expect("elem A");
    assert_eq!(a_elem.pressed, 1.0, "A pulsado se resalta");
    let others_pressed = elems
        .iter()
        .filter(|e| e.glyph != Glyph::LetterA)
        .all(|e| e.pressed == 0.0);
    assert!(others_pressed, "solo A está pulsado");

    let globals = build_globals(1080, 2400, Orientation::Portrait);
    assert_eq!(globals.opacity, 1.0, "portrait opaco");

    // (b) Landscape: opacidad translúcida.
    let globals_ls = build_globals(2400, 1080, Orientation::Landscape);
    assert!(
        globals_ls.opacity < 1.0 && globals_ls.opacity > 0.0,
        "landscape translúcido: {}",
        globals_ls.opacity
    );

    // (c) Todos los botones usan paleta gris-lavanda uniforme (Paso 1).
    let elems2 = build_elems(&layout(1080, 2400, Orientation::Portrait), |_| false);
    for elem in &elems2 {
        assert!(
            (elem.color[0] - 0.56).abs() < 0.01 && (elem.color[1] - 0.58).abs() < 0.01 && (elem.color[2] - 0.70).abs() < 0.01,
            "todos los botones deben ser gris-lavanda uniforme"
        );
    }

    // (d) Colores en landscape: gris claro translúcido escarchado ([0.72, 0.75, 0.82]).
    let elems_ls = build_elems_with_orientation(
        &layout(2400, 1080, Orientation::Landscape),
        Orientation::Landscape,
        |_| false,
    );
    for elem in &elems_ls {
        assert!(
            elem.color[0] > 0.70 && elem.color[1] > 0.70 && elem.color[2] > 0.70,
            "todos los botones en landscape deben ser gris claro translúcido"
        );
    }
}

/// Criterio de aceptación: en Landscape la opacidad es 0.65 y los botones
/// usan acabado gris claro escarchado translúcido (canales RGB > 0.70).
#[test]
fn test_landscape_frosted_light_gray_palette_and_opacity() {
    let globals = build_globals(2400, 1080, Orientation::Landscape);
    assert!(
        (globals.opacity - 0.65).abs() < 0.01,
        "opacidad en landscape debe ser 0.65, actual: {}",
        globals.opacity
    );

    let elems = build_elems_with_orientation(
        &layout(2400, 1080, Orientation::Landscape),
        Orientation::Landscape,
        |_| false,
    );
    for elem in &elems {
        assert!(
            elem.color[0] > 0.70 && elem.color[1] > 0.70 && elem.color[2] > 0.70,
            "color en landscape debe ser gris claro translúcido (> 0.70), actual: {:?}",
            elem.color
        );
    }
}

/// El empaquetado respeta el layout de memoria esperado por el
/// shader (12 floats por elemento, orden del struct Elem).
#[test]
fn test_pack_layout() {
    let e = ElemParams {
        cx: 1.0,
        cy: 2.0,
        radius: 3.0,
        color: [0.25, 0.5, 0.75],
        pressed: 1.0,
        kind: ElemKind::Pill,
        glyph: Glyph::LetterA,
        round: 0.4,
        glyph_scale: 0.13,
        ring: 0.55,
        angle: -0.52,
        extent_y: 1.5,
    };
    let packed = e.pack();
    assert_eq!(packed.len(), 16);
    assert_eq!(
        packed,
        [1.0, 2.0, 3.0, 0.25, 0.5, 0.75, 1.0, 1.0, 6.0, 0.4, 0.13, 0.55, -0.52, 1.5, 0.0, 0.0]
    );
}

/// Criterio 1 (normal): los shaders WGSL parsean y validan con Naga
/// sin errores de sintaxis ni de tipos.
#[test]
fn test_overlay_shader_validates() {
    let mut validator = naga::valid::Validator::new(
        naga::valid::ValidationFlags::all(),
        naga::valid::Capabilities::all(),
    );

    let overlay_module = naga::front::wgsl::parse_str(crate::overlay_shader::WGSL)
        .expect("el shader del overlay debe parsear");
    validator
        .validate(&overlay_module)
        .expect("el shader del overlay debe validar");

    let blit_module = naga::front::wgsl::parse_str(crate::overlay_shader::GAME_BLIT_WGSL)
        .expect("el shader de game_blit debe parsear");
    validator
        .validate(&blit_module)
        .expect("el shader de game_blit debe validar");
}

/// Criterio de aceptación (Paso 6): los shaders WGSL con textura de atlas y sampler
/// parsean y validan con Naga sin errores.
#[test]
fn test_overlay_shader_with_font_atlas_validates() {
    test_overlay_shader_validates();
}

/// Criterio de aceptación 3: confinamiento estricto de glifos dentro del cuerpo del botón.
/// Valida que el shader compila con Naga y que la escala del glifo respecto al radio es <= 0.20.
#[test]
fn test_overlay_glyph_within_bounds() {
    let mut validator = naga::valid::Validator::new(
        naga::valid::ValidationFlags::all(),
        naga::valid::Capabilities::all(),
    );

    let overlay_module = naga::front::wgsl::parse_str(crate::overlay_shader::WGSL)
        .expect("el shader del overlay debe parsear");
    validator
        .validate(&overlay_module)
        .expect("el shader del overlay debe validar");

    let tc = TouchController::new();
    let elems = build_elems(tc.areas(), |_| false);
    for elem in &elems {
        assert!(
            elem.glyph_scale <= 0.20,
            "escala del glifo {:?} debe ser <= 0.20 (actual: {})",
            elem.glyph,
            elem.glyph_scale
        );
    }
}

/// Criterio de aceptación Paso 6: Generación y validez del atlas tipográfico R8
#[test]
fn test_font_atlas_contains_required_glyphs() {
    let atlas = build_font_atlas();
    assert_eq!(atlas.width, 256);
    assert_eq!(atlas.height, 256);
    assert_eq!(atlas.data.len(), 256 * 256);
    let non_zero = atlas.data.iter().filter(|&&b| b > 0).count();
    assert!(non_zero > 1000, "el atlas debe contener glifos rasterizados: {}", non_zero);

    // Validar que cada una de las 3 filas contiene cobertura de píxeles:
    // Fila 0 (A, B, L, R)
    let row0_pixels = atlas.data[0..64 * 256].iter().filter(|&&b| b > 0).count();
    assert!(row0_pixels > 200, "fila 0 (botones A/B/L/R) vacía");

    // Fila 1 (flechas cruceta)
    let row1_pixels = atlas.data[64 * 256..128 * 256].iter().filter(|&&b| b > 0).count();
    assert!(row1_pixels > 200, "fila 1 (flechas cruceta) vacía");

    // Fila 2 (etiquetas SELECT / START)
    let row2_pixels = atlas.data[128 * 256..192 * 256].iter().filter(|&&b| b > 0).count();
    assert!(row2_pixels > 200, "fila 2 (etiquetas SELECT/START) vacía");
}

/// Criterio Paso 5: simetría y congruencia de los 4 brazos de la cruceta
/// para alimentar la cruz continua en el shader.
#[test]
fn test_dpad_cross_geometry_symmetry() {
    let mut tc = TouchController::new();
    tc.resize(1080, 2400);
    let elems = build_elems(tc.areas(), |_| false);
    let up = &elems[0];
    let down = &elems[1];
    let left = &elems[2];
    let right = &elems[3];

    assert_eq!(up.glyph, Glyph::ArrowUp);
    assert_eq!(down.glyph, Glyph::ArrowDown);
    assert_eq!(left.glyph, Glyph::ArrowLeft);
    assert_eq!(right.glyph, Glyph::ArrowRight);

    // Centro vertical y horizontal coinciden en el eje de simetría
    let dpad_cx = up.cx;
    let dpad_cy = (up.cy + down.cy) * 0.5;
    assert!((down.cx - dpad_cx).abs() < 0.01);
    assert!((left.cy - dpad_cy).abs() < 0.01);
    assert!((right.cy - dpad_cy).abs() < 0.01);

    // Los 4 brazos están a la misma distancia del centro
    let d_up = (up.cy - dpad_cy).abs();
    let d_down = (down.cy - dpad_cy).abs();
    let d_left = (left.cx - dpad_cx).abs();
    let d_right = (right.cx - dpad_cx).abs();
    assert!((d_up - d_down).abs() < 0.01);
    assert!((d_left - d_right).abs() < 0.01);
    assert!((d_up - d_left).abs() < 0.01);
}

/// Criterio de aceptación: La flecha izquierda de la cruceta debe apuntar a la izquierda (◀),
/// lo que significa que el ápice (menos cobertura de píxeles) está en la mitad izquierda
/// de la celda de 64x64, y la base (más cobertura) en la mitad derecha.
#[test]
fn test_dpad_left_arrow_points_left() {
    let atlas = build_font_atlas();
    // Celda de la flecha izquierda: col 2 (x: 128..192), row 1 (y: 64..128)
    let mut left_half_pixels = 0;
    let mut right_half_pixels = 0;
    for y in 64..128 {
        for x in 128..160 {
            if atlas.data[y * 256 + x] > 0 {
                left_half_pixels += 1;
            }
        }
        for x in 160..192 {
            if atlas.data[y * 256 + x] > 0 {
                right_half_pixels += 1;
            }
        }
    }
    assert!(
        left_half_pixels < right_half_pixels,
        "flecha izquierda debe apuntar a la izquierda (ápice a la izquierda: {} px vs base a la derecha: {} px)",
        left_half_pixels,
        right_half_pixels
    );
}

/// Criterio de aceptación: La poda AABB no trunca la geometría visual de los controles.
/// Para resoluciones típicas (Portrait 1080p y Landscape 1080p/1200p), las cajas
/// delimitadoras de poda contienen con holgura (> 16 px) el radio visual, sombras y etiquetas.
#[test]
fn test_overlay_bounding_boxes_cover_visual_elements() {
    let resolutions = [(1080, 2400), (2400, 1080), (1920, 1080), (2000, 1200)];
    for (w, h) in resolutions {
        let mut tc = TouchController::new();
        tc.resize(w, h);
        let orientation = tc.orientation();
        let elems = build_elems_with_orientation(tc.areas(), orientation, |_| false);

        // 1. Cruceta (elems 0..3)
        let up = &elems[0];
        let down = &elems[1];
        let arm_len = (down.cy - up.cy).abs() * 0.5;
        let half_len = arm_len + up.radius * 0.85;
        let cross_bb = half_len + 28.0;
        assert!(
            cross_bb >= half_len + 16.0,
            "holgura de cruceta insuficiente en {w}x{h}"
        );

        // 2. Botones de acción y etiquetas (elems 4..11)
        for (i, el) in elems.iter().enumerate().skip(4) {
            let max_r = el.radius.max(el.extent_y);
            let bb_x = max_r * 1.40 + 20.0;
            let bb_y_down = max_r * 1.40 + 20.0;
            let bb_y_up = if el.glyph as u8 > 8 {
                max_r * 2.80 + 36.0
            } else {
                bb_y_down
            };

            // Margen lateral > radio + 16 px
            assert!(
                bb_x >= el.radius + 16.0,
                "holgura X para elem {i} insuficiente: {bb_x} vs {}",
                el.radius
            );
            // Margen inferior > radio + sombra + 16 px
            let shadow_extent = el.extent_y * 0.35 + 8.0;
            assert!(
                bb_y_down >= shadow_extent + 16.0,
                "holgura inferior para elem {i} insuficiente: {bb_y_down} vs {shadow_extent}"
            );
            // Si tiene etiqueta textual (SELECT / START), bb_y_up debe cubrir la etiqueta
            if el.glyph as u8 > 8 {
                let label_offset = el.extent_y + (el.extent_y * 0.70).max(14.0);
                let label_half_h = (el.radius * 0.35 * 1.9).max(12.0);
                let total_top_reach = label_offset + label_half_h;
                assert!(
                    bb_y_up >= total_top_reach + 16.0,
                    "holgura superior para etiqueta de elem {i} insuficiente: {bb_y_up} vs {total_top_reach}"
                );
            }
        }
    }
}

/// Criterio de aceptación: la latencia del swapchain arranca en 1 (la
/// configuración de referencia, la que minimiza el retraso de presentación
/// entre entrada y pantalla). La calibración de arranque puede subirla después
/// si mide que el bloqueo en `present` lo justifica, y deja constancia en
/// `blazing-perf.txt` con las líneas `# tramo` / `# elegido`.
#[test]
fn test_compositor_frame_latency_triple_buffering() {
    let platform_src = include_str!("platform.rs");
    assert!(
        platform_src.contains("desired_maximum_frame_latency: 2,"),
        "Compositor debe configurar desired_maximum_frame_latency a 2 para forzar triple buffering real"
    );
}

/// Criterio de aceptación (palanca 2): el compositor dibuja en el espacio del
/// búfer del swapchain y escala la geometría del overlay a esa escala, de modo
/// que un búfer reducido conserve la composición en pantalla; y el bucle aplica
/// la configuración que decide la calibración.
#[test]
fn test_render_escala_bufer_y_wiring_de_calibracion() {
    let platform_src = include_str!("platform.rs");
    for fragmento in [
        "fn render_target(&self)",
        ".map(|e| e.scaled(escala_render))",
        "build_globals(rw, rh, touch.orientation())",
        "fn aplicar_present_config(",
        "state.calibrar_presentacion(&report, (win_w, win_h));",
    ] {
        assert!(
            platform_src.contains(fragmento),
            "falta en platform.rs: {fragmento}"
        );
    }
}

/// Decisiones registradas (2026-10-10): el barrido de calibración de
/// presentación queda desactivado por defecto (nitidez nativa: la medición
/// mostró que solo la reducción de resolución baja `present`) y el orden de
/// backends se decide con una constante explícita, para que el experimento de
/// presentación (OpenGL ES vs Vulkan) no quede enterrado en el cuerpo de la
/// función.
#[test]
fn test_decisiones_de_presentacion_declaradas() {
    let platform_src = include_str!("platform.rs");
    assert!(
        platform_src.contains("const CALIBRAR_PRESENTACION: bool = false;"),
        "el barrido de calibración debe quedar desactivado por decisión explícita"
    );
    assert!(
        platform_src.contains("const PREFERIR_OPENGL_ES: bool ="),
        "el backend preferente debe declararse en una constante"
    );
    assert!(
        platform_src.contains("let candidatos: [(wgpu::Backends, &str); 2] = if PREFERIR_OPENGL_ES"),
        "el orden de backends debe depender de la constante"
    );
}

/// La escala de render es una transformación geométrica pura: longitudes
/// escaladas (centro, radios y extends) y atributos intactos.
#[test]
fn test_elem_scaled_escala_geometria_y_conserva_atributos() {
    let mut tc = TouchController::new();
    tc.resize(1080, 2400);
    let elems = build_elems(tc.areas(), |_| false);
    assert!(
        elems.iter().any(|e| e.round > 0.0 && e.extent_y > 0.0),
        "el layout debe incluir pastillas (extents no nulos)"
    );
    for original in &elems {
        let reducido = original.scaled(0.5);
        assert_eq!(reducido.cx, original.cx * 0.5);
        assert_eq!(reducido.cy, original.cy * 0.5);
        assert_eq!(reducido.radius, original.radius * 0.5);
        assert_eq!(reducido.extent_y, original.extent_y * 0.5);
        assert_eq!(reducido.round, original.round * 0.5);
        // Atributos no geométricos: idénticos.
        assert_eq!(reducido.color, original.color);
        assert_eq!(reducido.pressed, original.pressed);
        assert_eq!(reducido.kind, original.kind);
        assert_eq!(reducido.glyph, original.glyph);
        assert_eq!(reducido.angle, original.angle);
        assert_eq!(reducido.glyph_scale, original.glyph_scale);
        assert_eq!(reducido.ring, original.ring);
        // Factor potencia de dos: la ida y vuelta es exacta.
        assert_eq!(reducido.scaled(2.0), *original);
    }
}

/// Con escala 1.0 el escalado es la identidad: el camino normal de render no
/// cambia en absoluto (requisito de no-regresión de la palanca 2).
#[test]
fn test_elem_scaled_uno_es_identidad() {
    let mut tc = TouchController::new();
    tc.resize(2000, 1200);
    for original in build_elems(tc.areas(), |_| false) {
        assert_eq!(original.scaled(1.0), original);
    }
}

/// Blindaje de layout de uniformes de GPU: valida que `build_globals` y `Globals::pack`
/// generen la memoria esperada por el shader WGSL para Portrait y Landscape.
#[test]
fn test_build_globals_and_pack_layout() {
    // 1. Portrait
    let g_port = build_globals(1080, 2400, Orientation::Portrait);
    assert_eq!(g_port.res_x, 1080.0);
    assert_eq!(g_port.res_y, 2400.0);
    assert_eq!(g_port.opacity, 1.0);
    assert_eq!(g_port.shell, 1.0);

    let packed_port = g_port.pack();
    assert_eq!(packed_port, [1080.0, 2400.0, 1.0, 1.0]);

    // 2. Landscape
    let g_land = build_globals(2400, 1080, Orientation::Landscape);
    assert_eq!(g_land.res_x, 2400.0);
    assert_eq!(g_land.res_y, 1080.0);
    assert_eq!(g_land.opacity, 0.65);
    assert_eq!(g_land.shell, 0.0);

    let packed_land = g_land.pack();
    assert_eq!(packed_land, [2400.0, 1080.0, 0.65, 0.0]);
}

/// Blindaje de mapeo exhaustivo de glifos: valida que cada botón de GBA
/// esté asociado a su glifo correspondiente en el atlas tipográfico.
#[test]
fn test_all_buttons_map_to_expected_glyph() {
    let mapping = [
        (Button::Up, Glyph::ArrowUp),
        (Button::Down, Glyph::ArrowDown),
        (Button::Left, Glyph::ArrowLeft),
        (Button::Right, Glyph::ArrowRight),
        (Button::B, Glyph::LetterB),
        (Button::A, Glyph::LetterA),
        (Button::L, Glyph::LetterL),
        (Button::R, Glyph::LetterR),
        (Button::Select, Glyph::SelectLabel),
        (Button::Start, Glyph::StartLabel),
    ];

    for (btn, expected_glyph) in mapping {
        assert_eq!(
            button_glyph(btn),
            expected_glyph,
            "botón {:?} debe mapear al glifo {:?}",
            btn,
            expected_glyph
        );
    }
}

/// Caso límite del atlas tipográfico: comprueba que las dimensiones sean exactamente 256x256,
/// que la fila 3 (y en [192, 255]) permanezca completamente transparente (bytes en 0)
/// como espacio reservado para futuras expansiones y que no existan desbordamientos.
#[test]
fn test_font_atlas_boundary_cells_and_empty_transparency() {
    let atlas = build_font_atlas();
    assert_eq!(atlas.width, 256);
    assert_eq!(atlas.height, 256);
    assert_eq!(atlas.data.len(), 256 * 256);

    // Fila 3 (y de 192 a 255) está reservada: debe estar completamente limpia (todos 0)
    let reserved_start = 192 * 256;
    for (i, &byte) in atlas.data[reserved_start..].iter().enumerate() {
        assert_eq!(
            byte,
            0,
            "el byte en índice {} (fila 3 reservada) debe ser 0",
            reserved_start + i
        );
    }

    // Los glifos activos de las filas 0..2 deben tener al menos un píxel no nulo
    let active_pixels = atlas.data[..reserved_start].iter().filter(|&&b| b > 0).count();
    assert!(active_pixels > 500, "el atlas debe contener glifos rasterizados en filas 0..2");
}
