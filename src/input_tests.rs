//! Pruebas unitarias para el controlador de entrada táctil (`src/input.rs`).

use super::*;

/// Criterio 1 (normal): pulsar A en landscape 2400x1080 activa el bit
/// 0 en bajo; soltar lo restaura a alto.
#[test]
fn test_touch_hitbox_mapping() {
    let mut tc = TouchController::new();
    tc.resize(2400, 1080); // landscape
    assert_eq!(tc.orientation(), Orientation::Landscape);

    let (ax, ay) = {
        let a = tc
            .areas()
            .iter()
            .find(|a| a.button == Button::A)
            .expect("hitbox A existe");
        (a.cx, a.cy)
    };
    // Pulsar en el centro exacto del botón A.
    let pressed = tc.touch_down(7, ax, ay);
    assert_eq!(pressed, Some(Button::A), "el toque en A debe activar A");
    assert_eq!(tc.key_mask() & 0x0001, 0x0000, "bit 0 de A en bajo");

    // Soltar: bit restaurado a alto.
    let released = tc.touch_up(7);
    assert_eq!(released, Some(Button::A));
    assert_eq!(tc.key_mask() & 0x0001, 0x0001, "bit 0 de A en alto");
    assert_eq!(tc.key_mask(), 0x03FF, "sin nada pulsado = 0x3FF");
}

/// Criterio de regresión (Paso 6): verificación estricta de que el mapeo de
/// hitboxes y KEYINPUT permanezca 100% inalterado tras los rediseños estéticos.
#[test]
fn test_touch_hitbox_mapping_untouched() {
    test_touch_hitbox_mapping();
}

/// Criterio de aceptación: separación adecuada entre botones B y A
/// (distancia >= 95% de la unidad base) y botones L/R de tamaño igual o superior a B/A.
#[test]
fn test_action_buttons_separation_and_shoulder_size() {
    let areas = layout(1080, 2400, Orientation::Portrait);
    let b = areas.iter().find(|a| a.button == Button::B).unwrap();
    let a = areas.iter().find(|a| a.button == Button::A).unwrap();
    let l = areas.iter().find(|a| a.button == Button::L).unwrap();
    let r = areas.iter().find(|a| a.button == Button::R).unwrap();
    let up = areas.iter().find(|a| a.button == Button::Up).unwrap();
    let down = areas.iter().find(|a| a.button == Button::Down).unwrap();

    let dpad_r = (down.cy - up.cy) * 0.5 / 0.55;
    let dist_ba = ((a.cx - b.cx).powi(2) + (a.cy - b.cy).powi(2)).sqrt();
    assert!(
        dist_ba >= dpad_r * 0.95,
        "distancia entre B y A debe ser >= 95% de dpad_r (actual: {}, dpad_r: {})",
        dist_ba,
        dpad_r
    );
    assert!(
        l.radius >= b.radius,
        "radio de L ({}) debe ser >= radio de B ({})",
        l.radius,
        b.radius
    );
    assert!(
        r.radius >= a.radius,
        "radio de R ({}) debe ser >= radio de A ({})",
        r.radius,
        a.radius
    );
}

/// Criterio 2 (límite): portrait reubica el clúster en la mitad
/// inferior, y con touch_enabled=false ningún toque altera teclas.
#[test]
fn test_touch_layout_orientation_and_toggle() {
    // (a) Portrait 1080x2400: el clúster B/A cae en la mitad inferior.
    let mut tc = TouchController::new();
    tc.resize(1080, 2400);
    assert_eq!(tc.orientation(), Orientation::Portrait);
    let a = {
        let a = tc
            .areas()
            .iter()
            .find(|a| a.button == Button::A)
            .expect("hitbox A existe");
        (a.cx, a.cy)
    };
    assert!(
        a.1 > 2400.0 * 0.5,
        "el clúster debe estar en la mitad inferior (cy={}, mitad=1200)",
        a.1
    );
    // Y el par diagonal: A arriba a la derecha de B (eje inclinado).
    let b = {
        let b = tc
            .areas()
            .iter()
            .find(|x| x.button == Button::B)
            .expect("hitbox B existe");
        (b.cx, b.cy)
    };
    assert!(a.0 > b.0 && a.1 < b.1, "A arriba-derecha de B (16°)");

    // L sobre B en el eje ortogonal del clúster (nunca esquinas superiores).
    let l = {
        let l = tc
            .areas()
            .iter()
            .find(|x| x.button == Button::L)
            .expect("hitbox L existe");
        (l.cx, l.cy)
    };
    assert!(
        l.1 < b.1 && l.0 <= b.0,
        "L por encima de B alineado al eje del clúster"
    );
    let dot = (a.0 - b.0) * (l.0 - b.0) + (a.1 - b.1) * (l.1 - b.1);
    assert!(
        dot.abs() < 1.0,
        "L y B deben ser ortogonales a la inclinación B->A (dot = {})",
        dot
    );

    // (b) touch_enabled = false: ningún toque altera las teclas.
    tc.touch_enabled = false;
    let pressed = tc.touch_down(1, a.0, a.1);
    assert_eq!(pressed, None, "con controles apagados no hay activación");
    assert_eq!(tc.key_mask(), 0x03FF, "estado de teclas intacto");
}

/// Reglas de precisión individual y cancelación (contexto de la
/// propuesta): toques ambiguos se descartan y Cancel libera todo.
#[test]
fn test_touch_ambiguity_and_cancel() {
    let mut tc = TouchController::new();
    tc.resize(2400, 1080);

    // Toque fuera de cualquier hitbox: descartado.
    assert_eq!(tc.touch_down(1, 5.0, 5.0), None);
    assert_eq!(tc.key_mask(), 0x03FF);

    // Pulsar B y A a la vez (dos punteros): precisión individual.
    let (b, a) = {
        let b = tc.areas().iter().find(|a| a.button == Button::B).unwrap();
        let a = tc.areas().iter().find(|x| x.button == Button::A).unwrap();
        ((b.cx, b.cy), (a.cx, a.cy))
    };
    assert_eq!(tc.touch_down(1, b.0, b.1), Some(Button::B));
    assert_eq!(tc.touch_down(2, a.0, a.1), Some(Button::A));
    assert_eq!(tc.key_mask(), !0x0003 & 0x03FF, "B y A en bajo");

    // Un puntero que ya mantiene no puede reactivar: descartado.
    assert_eq!(tc.touch_down(1, a.0, a.1), None);

    // Cancel libera todo.
    tc.cancel_all();
    assert_eq!(tc.key_mask(), 0x03FF);
}

/// Determinismo geométrico: misma entrada → mismas hitboxes.
#[test]
fn test_layout_deterministic() {
    let l1 = layout(2400, 1080, Orientation::Landscape);
    let l2 = layout(2400, 1080, Orientation::Landscape);
    assert_eq!(l1, l2);
}

/// Criterio de aceptación 1: proporciones físicas ergonómicas en Portrait.
/// Radio de B y A > 60 px y centro vertical entre 65% y 80% de la pantalla.
#[test]
fn test_touch_layout_physical_proportions() {
    let areas = layout(1080, 2400, Orientation::Portrait);
    let a = areas.iter().find(|x| x.button == Button::A).expect("A existe");
    let b = areas.iter().find(|x| x.button == Button::B).expect("B existe");
    assert!(a.radius > 60.0, "radio de A ({}) debe ser > 60 px", a.radius);
    assert!(b.radius > 60.0, "radio de B ({}) debe ser > 60 px", b.radius);

    let cy = (a.cy + b.cy) * 0.5;
    let frac = cy / 2400.0;
    assert!(
        (0.65..=0.80).contains(&frac),
        "centro vertical ({:.2}%) debe estar entre 65% y 80%",
        frac * 100.0
    );
}

/// Criterio de aceptación 2: sincronización reactiva al cambiar tamaño/orientación.
/// Al cambiar a Landscape (2400x1080), todos los botones quedan contenidos en 0..1080.
#[test]
fn test_touch_controller_resizes_dynamically() {
    let mut tc = TouchController::new();
    assert_eq!(tc.orientation(), Orientation::Portrait);
    assert_eq!(tc.size(), (1080, 2400));

    tc.resize(2400, 1080);
    assert_eq!(tc.orientation(), Orientation::Landscape);
    assert_eq!(tc.size(), (2400, 1080));

    for area in tc.areas() {
        assert!(
            area.cy - area.radius >= 0.0 && area.cy + area.radius <= 1080.0,
            "botón {:?} en y={} r={} fuera de límites verticales 0..1080",
            area.button, area.cy, area.radius
        );
        assert!(
            area.cx - area.radius >= 0.0 && area.cx + area.radius <= 2400.0,
            "botón {:?} en x={} r={} fuera de límites horizontales 0..2400",
            area.button, area.cx, area.radius
        );
    }
}

/// Valida que múltiples resoluciones estándar (16:9, HD, 2K) mantengan todos
/// los controles dentro de los límites de la pantalla y con radios positivos.
#[test]
fn test_touch_controller_various_resolutions() {
    let resolutions = [
        (1080, 1920), // 16:9 portrait
        (720, 1280),  // HD portrait
        (1920, 1080), // 16:9 landscape
        (1280, 720),  // HD landscape
        (2560, 1440), // 2K landscape
    ];

    for (w, h) in resolutions {
        let mut tc = TouchController::new();
        tc.resize(w, h);
        assert_eq!(tc.size(), (w, h));

        let (wf, hf) = (w as f32, h as f32);
        for area in tc.areas() {
            assert!(area.radius > 0.0, "radio positivo para {:?}", area.button);
            assert!(
                area.cx - area.radius >= 0.0 && area.cx + area.radius <= wf,
                "botón {:?} en ({}, {}) r={} fuera de límites horizontales 0..{}",
                area.button, area.cx, area.cy, area.radius, w
            );
            assert!(
                area.cy - area.radius >= 0.0 && area.cy + area.radius <= hf,
                "botón {:?} en ({}, {}) r={} fuera de límites verticales 0..{}",
                area.button, area.cx, area.cy, area.radius, h
            );
        }
    }
}

/// Criterio de aceptación: En Portrait, la cuenca de la cruceta y la cápsula
/// de los botones de acción deben tener una separación neta clara (> 60 px).
#[test]
fn test_portrait_well_separation() {
    let areas = layout(1080, 2400, Orientation::Portrait);
    let up = areas.iter().find(|x| x.button == Button::Up).unwrap();
    let down = areas.iter().find(|x| x.button == Button::Down).unwrap();
    let b = areas.iter().find(|x| x.button == Button::B).unwrap();
    let a = areas.iter().find(|x| x.button == Button::A).unwrap();
    let l = areas.iter().find(|x| x.button == Button::L).unwrap();
    let r = areas.iter().find(|x| x.button == Button::R).unwrap();

    let dpad_cx = up.cx;
    let arm_len = (down.cy - up.cy).abs() * 0.5;
    let dpad_r = arm_len + up.radius * 1.15;
    let dpad_right_x = dpad_cx + dpad_r;

    let ba_mid_x = (b.cx + a.cx) * 0.5;
    let lr_mid_x = (l.cx + r.cx) * 0.5;
    let cluster_center_x = (ba_mid_x + lr_mid_x) * 0.5;

    let ba_len = ((a.cx - b.cx).powi(2) + (a.cy - b.cy).powi(2)).sqrt();
    let half_w = ba_len * 0.5 + b.radius * 1.30;
    let tilt = 16.0f32.to_radians();
    let capsule_left_x = cluster_center_x - half_w * tilt.cos();

    let gap = capsule_left_x - dpad_right_x;
    assert!(
        gap > 60.0,
        "debe haber separación clara entre las cuencas (gap actual: {} px, dpad_right={}, capsule_left={})",
        gap,
        dpad_right_x,
        capsule_left_x
    );
}

/// Criterio de aceptación: En Landscape, los controles deben acercarse
/// a sus respectivos bordes laterales para mayor ergonomía de los pulgares.
#[test]
fn test_landscape_controls_near_edges() {
    let areas = layout(2400, 1080, Orientation::Landscape);
    let up = areas.iter().find(|x| x.button == Button::Up).unwrap();
    let a = areas.iter().find(|x| x.button == Button::A).unwrap();

    // Cruceta cerca del lateral izquierdo (<= 0.23 * hf)
    assert!(
        up.cx <= 1080.0 * 0.23,
        "cruceta debe estar cerca del borde izquierdo (cx={}, umbral={})",
        up.cx,
        1080.0 * 0.23
    );

    // Botón A cerca del lateral derecho (>= wf - 0.23 * hf)
    assert!(
        a.cx >= 2400.0 - 1080.0 * 0.23,
        "botón A debe estar cerca del borde derecho (cx={}, umbral={})",
        a.cx,
        2400.0 - 1080.0 * 0.23
    );
}

/// Blindaje de contrato de hardware GBA: cada uno de los 10 botones mapea a su bit
/// estándar en el registro KEYINPUT sin colisiones y cubriendo la máscara 0x03FF.
#[test]
fn test_all_buttons_key_bits_distinct_and_standard_gba() {
    let all_buttons = [
        (Button::A, 0x0001),
        (Button::B, 0x0002),
        (Button::Select, 0x0004),
        (Button::Start, 0x0008),
        (Button::Right, 0x0010),
        (Button::Left, 0x0020),
        (Button::Up, 0x0040),
        (Button::Down, 0x0080),
        (Button::R, 0x0100),
        (Button::L, 0x0200),
    ];

    let mut combined_mask = 0u16;
    for (btn, expected_bit) in all_buttons {
        let bit = btn.key_bit();
        assert_eq!(
            bit, expected_bit,
            "botón {:?} debe mapear exactamente al bit 0x{:04X}",
            btn, expected_bit
        );
        assert_eq!(
            combined_mask & bit,
            0,
            "colisión detectada para botón {:?} en bit 0x{:04X}",
            btn,
            bit
        );
        combined_mask |= bit;
    }

    assert_eq!(
        combined_mask, 0x03FF,
        "los 10 botones deben cubrir exactamente la máscara de 10 bits de KEYINPUT (0x03FF)"
    );
}

/// Blindaje de estado de pulsación para el compositor GPU: valida `is_pressed`
/// y `pressed_mask` en toques individuales, acordes simultáneos y tras cancel_all.
#[test]
fn test_touch_controller_is_pressed_and_pressed_mask() {
    let mut tc = TouchController::new();
    let areas = tc.areas().to_vec();

    let area_a = areas.iter().find(|a| a.button == Button::A).unwrap();
    let area_b = areas.iter().find(|a| a.button == Button::B).unwrap();
    let area_up = areas.iter().find(|a| a.button == Button::Up).unwrap();

    // Reposo inicial
    assert!(!tc.is_pressed(Button::A));
    assert!(!tc.is_pressed(Button::B));
    assert_eq!(tc.pressed_mask(), 0x0000);

    // Pulsar botón B
    tc.touch_down(1, area_b.cx, area_b.cy);
    assert!(tc.is_pressed(Button::B));
    assert!(!tc.is_pressed(Button::A));
    assert_eq!(tc.pressed_mask(), Button::B.key_bit());

    // Pulsar botón A simultáneamente con otro dedo
    tc.touch_down(2, area_a.cx, area_a.cy);
    assert!(tc.is_pressed(Button::B));
    assert!(tc.is_pressed(Button::A));
    assert_eq!(tc.pressed_mask(), Button::B.key_bit() | Button::A.key_bit());

    // Pulsar botón Up (acorde de 3 dedos)
    tc.touch_down(3, area_up.cx, area_up.cy);
    assert!(tc.is_pressed(Button::Up));
    assert_eq!(
        tc.pressed_mask(),
        Button::B.key_bit() | Button::A.key_bit() | Button::Up.key_bit()
    );

    // Soltar botón B
    let released = tc.touch_up(1);
    assert_eq!(released, Some(Button::B));
    assert!(!tc.is_pressed(Button::B));
    assert!(tc.is_pressed(Button::A));
    assert!(tc.is_pressed(Button::Up));
    assert_eq!(tc.pressed_mask(), Button::A.key_bit() | Button::Up.key_bit());

    // Cancelar todo
    tc.cancel_all();
    assert!(!tc.is_pressed(Button::A));
    assert!(!tc.is_pressed(Button::Up));
    assert_eq!(tc.pressed_mask(), 0x0000);
}

/// Blindaje de getters y traits estándar: valida `size()` y `Default`.
#[test]
fn test_touch_controller_size_and_default() {
    let tc = TouchController::default();
    assert_eq!(tc.size(), (1080, 2400));
    assert_eq!(tc.orientation(), Orientation::Portrait);
    assert!(tc.touch_enabled);

    let mut tc2 = TouchController::new();
    tc2.resize(1920, 1080);
    assert_eq!(tc2.size(), (1920, 1080));
    assert_eq!(tc2.orientation(), Orientation::Landscape);
}

/// Caso límite de robustez: liberar un pointer_id desconocido no altera los toques activos.
#[test]
fn test_touch_controller_unknown_pointer_release() {
    let mut tc = TouchController::new();
    let area_a = tc.areas().iter().find(|a| a.button == Button::A).unwrap();

    tc.touch_down(10, area_a.cx, area_a.cy);
    assert!(tc.is_pressed(Button::A));

    // Liberar puntero inexistente (ID 999)
    let released = tc.touch_up(999);
    assert_eq!(released, None, "un puntero no registrado debe retornar None");
    assert!(
        tc.is_pressed(Button::A),
        "el botón A debe seguir pulsado tras soltar un puntero desconocido"
    );
    assert_eq!(tc.pressed_mask(), Button::A.key_bit());
}

/// Caso límite matemático: evalúa las condiciones de contorno de `TouchArea::contains(x, y)`
/// en el centro (d=0), borde exacto (d=r), diagonal (r*sqrt(0.5)), frontera interior (r - eps)
/// y frontera exterior (r + eps). Previene mutantes que alteren `<` por `<=`.
#[test]
fn test_touch_area_exact_boundary_epsilon() {
    let area = TouchArea {
        button: Button::A,
        cx: 200.0,
        cy: 300.0,
        radius: 50.0,
    };

    // 1. Centro exacto (d = 0)
    assert!(area.contains(200.0, 300.0), "el centro debe estar contenido");

    // 2. Fronteras cardinales exactas (d = r = 50.0) -> d^2 <= r^2 debe ser true
    assert!(area.contains(250.0, 300.0), "borde derecho exacto debe estar contenido");
    assert!(area.contains(150.0, 300.0), "borde izquierdo exacto debe estar contenido");
    assert!(area.contains(200.0, 350.0), "borde inferior exacto debe estar contenido");
    assert!(area.contains(200.0, 250.0), "borde superior exacto debe estar contenido");

    // 3. Frontera no cardinal exacta mediante terna pitagórica (30^2 + 40^2 = 50^2 = 2500, cero error f32)
    assert!(
        area.contains(200.0 + 30.0, 300.0 + 40.0),
        "borde exacto por terna pitagórica (+30, +40) debe estar contenido"
    );
    assert!(
        area.contains(200.0 - 40.0, 300.0 + 30.0),
        "borde exacto por terna pitagórica (-40, +30) debe estar contenido"
    );

    // 4. Frontera interior (d = r - 0.05)
    assert!(
        area.contains(249.95, 300.0),
        "punto justo adentro debe estar contenido"
    );

    // 5. Frontera exterior (d = r + 0.05)
    assert!(
        !area.contains(250.05, 300.0),
        "punto justo afuera no debe estar contenido"
    );
    assert!(
        !area.contains(200.0 + 30.1, 300.0 + 40.1),
        "frontera exterior no cardinal (+30.1, +40.1) no debe estar contenida"
    );
}

/// Caso límite de cruceta: el centro geométrico debe ser una zona muerta neutral estricta,
/// y pulsar en una dirección cardinal debe aislar totalmente a las 3 direcciones restantes.
#[test]
fn test_dpad_neutral_deadzone_and_cardinal_isolation() {
    let tc = TouchController::new();
    let areas = tc.areas();

    let up = areas.iter().find(|a| a.button == Button::Up).unwrap();
    let down = areas.iter().find(|a| a.button == Button::Down).unwrap();
    let left = areas.iter().find(|a| a.button == Button::Left).unwrap();
    let right = areas.iter().find(|a| a.button == Button::Right).unwrap();

    // Centro geométrico entre Up/Down y Left/Right
    let center_x = (left.cx + right.cx) * 0.5;
    let center_y = (up.cy + down.cy) * 0.5;

    // 1. Zona muerta central: en el centro exacto no debe activarse ninguna dirección
    assert!(
        !up.contains(center_x, center_y),
        "el centro no debe contener Up"
    );
    assert!(
        !down.contains(center_x, center_y),
        "el centro no debe contener Down"
    );
    assert!(
        !left.contains(center_x, center_y),
        "el centro no debe contener Left"
    );
    assert!(
        !right.contains(center_x, center_y),
        "el centro no debe contener Right"
    );

    // 2. Aislamiento cardinal: en el centro de Up solo debe estar activo Up
    assert!(up.contains(up.cx, up.cy));
    assert!(!down.contains(up.cx, up.cy), "Up no debe solapar con Down");
    assert!(!left.contains(up.cx, up.cy), "Up no debe solapar con Left");
    assert!(!right.contains(up.cx, up.cy), "Up no debe solapar con Right");

    // 3. Aislamiento cardinal: en el centro de Left solo debe estar activo Left
    assert!(left.contains(left.cx, left.cy));
    assert!(!right.contains(left.cx, left.cy), "Left no debe solapar con Right");
}

/// Estrés de layout: evalúa resoluciones exóticas (cuadrada 1:1, ultrapanorámica 32:9 y QVGA retro).
/// Comprueba ausencia de NaN, radios positivos y confinamiento dentro de la pantalla visible.
#[test]
fn test_touch_layout_square_and_ultrawide_aspect_ratios() {
    // 1. Pantalla cuadrada 1:1 (ej. teléfonos plegables cerrados / smartwatch: 1080x1080)
    let square_areas = layout(1080, 1080, Orientation::Landscape);
    assert_eq!(square_areas.len(), 10);
    for a in &square_areas {
        assert!(!a.cx.is_nan() && !a.cy.is_nan(), "coords no deben ser NaN en 1:1");
        assert!(a.radius > 0.0, "radio debe ser positivo en 1:1");
        assert!(
            a.cx >= 0.0 && a.cx <= 1080.0,
            "botón {:?} fuera de límites horizontales en 1:1: cx={}",
            a.button,
            a.cx
        );
        assert!(
            a.cy >= 0.0 && a.cy <= 1080.0,
            "botón {:?} fuera de límites verticales en 1:1: cy={}",
            a.button,
            a.cy
        );
    }

    // 2. Ultrapanorámica 32:9 (3840x1080)
    let uw_areas = layout(3840, 1080, Orientation::Landscape);
    let uw_dpad = uw_areas.iter().find(|a| a.button == Button::Up).unwrap();
    let uw_a = uw_areas.iter().find(|a| a.button == Button::A).unwrap();
    assert!(
        uw_dpad.cx <= 1080.0 * 0.25,
        "cruceta debe permanecer cercana al lateral izquierdo en 32:9"
    );
    assert!(
        uw_a.cx >= 3840.0 - 1080.0 * 0.25,
        "botón A debe permanecer cercano al lateral derecho en 32:9"
    );

    // 3. Resolución mínima retro (320x240 QVGA)
    let retro_areas = layout(320, 240, Orientation::Landscape);
    for a in &retro_areas {
        assert!(!a.cx.is_nan() && !a.cy.is_nan() && !a.radius.is_nan());
        assert!(a.radius > 5.0, "botones deben mantener tamaño mínimo interactuable");
    }
}

/// Estrés multitáctil: ráfaga de 5 dedos en botones + 3 toques en zonas vacías,
/// con liberación en orden inverso desordenado, asegurando retorno limpio a 0x03FF.
#[test]
fn test_touch_controller_multitouch_churn_and_saturation() {
    let mut tc = TouchController::new();
    let areas = tc.areas().to_vec();

    let btn_a = areas.iter().find(|a| a.button == Button::A).unwrap();
    let btn_b = areas.iter().find(|a| a.button == Button::B).unwrap();
    let btn_up = areas.iter().find(|a| a.button == Button::Up).unwrap();
    let btn_l = areas.iter().find(|a| a.button == Button::L).unwrap();
    let btn_start = areas.iter().find(|a| a.button == Button::Start).unwrap();

    // 1. Tocar 5 botones válidos
    assert_eq!(tc.touch_down(101, btn_a.cx, btn_a.cy), Some(Button::A));
    assert_eq!(tc.touch_down(102, btn_b.cx, btn_b.cy), Some(Button::B));
    assert_eq!(tc.touch_down(103, btn_up.cx, btn_up.cy), Some(Button::Up));
    assert_eq!(tc.touch_down(104, btn_l.cx, btn_l.cy), Some(Button::L));
    assert_eq!(tc.touch_down(105, btn_start.cx, btn_start.cy), Some(Button::Start));

    // 2. Tocar 3 zonas vacías (arriba a la izquierda, fuera de hitboxes)
    assert_eq!(tc.touch_down(106, 50.0, 50.0), None);
    assert_eq!(tc.touch_down(107, 500.0, 100.0), None);
    assert_eq!(tc.touch_down(108, 900.0, 50.0), None);

    // Máscara combinada con los 5 botones activos
    let expected_mask = Button::A.key_bit()
        | Button::B.key_bit()
        | Button::Up.key_bit()
        | Button::L.key_bit()
        | Button::Start.key_bit();
    assert_eq!(tc.pressed_mask(), expected_mask);
    assert_eq!(tc.key_mask(), !expected_mask & 0x03FF);

    // 3. Liberar punteros en orden inverso y desordenado
    assert_eq!(tc.touch_up(103), Some(Button::Up)); // Soltar Up
    assert!(!tc.is_pressed(Button::Up));
    assert!(tc.is_pressed(Button::A));

    assert_eq!(tc.touch_up(107), None); // Soltar puntero espurio
    assert_eq!(tc.touch_up(105), Some(Button::Start)); // Soltar Start
    assert_eq!(tc.touch_up(101), Some(Button::A)); // Soltar A
    assert_eq!(tc.touch_up(104), Some(Button::L)); // Soltar L
    assert_eq!(tc.touch_up(102), Some(Button::B)); // Soltar B

    // 4. Todo liberado: retorno a reposo absoluto
    assert_eq!(tc.pressed_mask(), 0x0000);
    assert_eq!(tc.key_mask(), 0x03FF);
}
