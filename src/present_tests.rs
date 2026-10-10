//! Pruebas del módulo de calibración de presentación (`present.rs`).
//!
//! Módulo satélite: se enlaza desde `present.rs` con
//! `#[cfg(test)] #[path = "present_tests.rs"] mod tests;`.

use super::*;
use wgpu::PresentMode;

/// Construye un tramo medido con `ventanas` ventanas de 1 s idénticas.
fn tramo(latency: u32, modo: PresentMode, scale: f32, rol: Rol, present: f64, fps: f64) -> Tramo {
    let mut t = Tramo::new(Candidato::new(latency, modo, scale, rol));
    for _ in 0..SEGUNDOS_POR_TRAMO {
        t.muestra(present, fps);
    }
    t
}

/// Recorrido completo con los cinco tramos de la calibración real.
fn recorrido(presents: [f64; 5]) -> Vec<Tramo> {
    vec![
        tramo(1, PresentMode::Mailbox, 1.0, Rol::Referencia, presents[0], 18.0),
        tramo(2, PresentMode::Mailbox, 1.0, Rol::Adoptable, presents[1], 18.0),
        tramo(3, PresentMode::Mailbox, 1.0, Rol::Adoptable, presents[2], 18.0),
        tramo(
            3,
            PresentMode::Mailbox,
            ESCALA_REDUCIDA,
            Rol::Sonda,
            presents[3],
            18.0,
        ),
        tramo(3, PresentMode::Immediate, 1.0, Rol::Sonda, presents[4], 18.0),
    ]
}

#[test]
fn test_descripcion_de_candidato() {
    let c = Candidato::new(2, PresentMode::Mailbox, 1.0, Rol::Adoptable);
    assert_eq!(c.descripcion(), "lat=2 modo=Mailbox escala=1.00");
    // Referencia y sondas se etiquetan: el log no debe sugerir que una sonda
    // que midió mejor fue adoptada por descuido.
    let r = Candidato::new(1, PresentMode::Mailbox, 1.0, Rol::Referencia);
    assert_eq!(r.descripcion(), "lat=1 modo=Mailbox escala=1.00 referencia");
    let s = Candidato::new(3, PresentMode::Immediate, 1.0, Rol::Sonda);
    assert_eq!(s.descripcion(), "lat=3 modo=Immediate escala=1.00 sonda");
}

#[test]
fn test_tramo_promedia_solo_ventanas_con_fotogramas() {
    let mut t = Tramo::new(Candidato::new(1, PresentMode::Mailbox, 1.0, Rol::Referencia));
    t.muestra(100.0, 0.0); // ventana sin fotogramas: no debe contaminar
    t.muestra(20.0, 18.0);
    assert!(!t.completa(), "dos ventanas todavía no cierran el tramo");
    assert_eq!(t.present_medio(), Some(20.0));
    t.muestra(30.0, 18.0);
    assert!(t.completa());
    assert_eq!(t.present_medio(), Some(25.0));
    assert_eq!(t.fps_medio(), 12.0);
    assert!(t.valido());

    let mut vacio = Tramo::new(Candidato::new(1, PresentMode::Fifo, 1.0, Rol::Sonda));
    vacio.muestra(40.0, 0.0);
    assert_eq!(vacio.present_medio(), None);
    assert!(!vacio.valido());
}

#[test]
fn test_sin_mejora_se_mantiene_la_referencia() {
    let (elegido, motivo) = elegir_mejor(&recorrido([52.0, 52.0, 52.0, 55.0, 2.0]));
    assert_eq!(elegido.latency, 1);
    assert_eq!(elegido.scale, 1.0);
    assert_eq!(elegido.rol, Rol::Referencia);
    assert!(motivo.contains("sin mejora medible"), "motivo: {motivo}");
}

#[test]
fn test_adopta_mas_imagenes_con_mejora_medible() {
    let (elegido, motivo) = elegir_mejor(&recorrido([52.0, 18.0, 18.0, 19.0, 2.0]));
    assert_eq!(elegido.latency, 2);
    assert_eq!(elegido.scale, 1.0);
    assert!(motivo.contains("mejora medible sobre la referencia"), "motivo: {motivo}");
}

#[test]
fn test_mejora_insuficiente_no_cambia_nada() {
    // 51 ms no alcanza el margen del 5 % exigido sobre 52 ms.
    let (elegido, _) = elegir_mejor(&recorrido([52.0, 51.0, 51.0, 51.0, 2.0]));
    assert_eq!(elegido.latency, 1);
    // 49 ms sí lo alcanza (49 <= 52 * 0.95 = 49.4).
    let (elegido, _) = elegir_mejor(&recorrido([52.0, 49.0, 49.0, 49.0, 2.0]));
    assert_eq!(elegido.latency, 2);
}

#[test]
fn test_la_escala_reducida_se_mide_pero_no_se_adopta() {
    // La sonda de 0,5× mide mucho mejor (12 ms), pero bajar la resolución de
    // forma permanente es una decisión de nitidez ya descartada: se adopta la
    // mejor candidata de resolución nativa.
    let (elegido, motivo) = elegir_mejor(&recorrido([52.0, 18.0, 18.0, 12.0, 2.0]));
    assert_eq!(elegido.scale, 1.0);
    assert_eq!(elegido.latency, 2);
    assert!(!motivo.contains("escala"), "motivo: {motivo}");
}

#[test]
fn test_sonda_nunca_se_adopta() {
    // La sonda es la más rápida de todas, pero presentar sin VSync produce tearing.
    let (elegido, _) = elegir_mejor(&recorrido([52.0, 52.0, 52.0, 52.0, 0.5]));
    assert_eq!(elegido.rol, Rol::Referencia);
    assert_eq!(elegido.present_mode, PresentMode::Mailbox);
}

#[test]
fn test_referencia_sin_muestras_validas_conserva_la_configuracion() {
    let tramos = vec![
        tramo(1, PresentMode::Mailbox, 1.0, Rol::Referencia, 0.0, 0.0),
        tramo(2, PresentMode::Mailbox, 1.0, Rol::Adoptable, 10.0, 18.0),
    ];
    let (elegido, motivo) = elegir_mejor(&tramos);
    assert_eq!(elegido.latency, 1);
    assert!(motivo.contains("la referencia no tiene muestras válidas"), "motivo: {motivo}");
}

#[test]
fn test_tramo_fallido_no_se_adopta() {
    // El swapchain de latencia 3 falló (0 fps): no debe ganar por tener
    // un `present` acumulado de 0 ms.
    let tramos = vec![
        tramo(1, PresentMode::Mailbox, 1.0, Rol::Referencia, 52.0, 18.0),
        tramo(3, PresentMode::Mailbox, 1.0, Rol::Adoptable, 0.0, 0.0),
    ];
    let (elegido, _) = elegir_mejor(&tramos);
    assert_eq!(elegido.latency, 1);
}

#[test]
fn test_recorrido_incluye_sonda_solo_si_hay_immediate() {
    let completo = PresentCalibration::new(
        PresentMode::Mailbox,
        &[PresentMode::Fifo, PresentMode::Mailbox, PresentMode::Immediate],
    );
    assert_eq!(completo.total_tramos(), 5);

    let sin_immediate =
        PresentCalibration::new(PresentMode::Mailbox, &[PresentMode::Fifo, PresentMode::Mailbox]);
    assert_eq!(sin_immediate.total_tramos(), 4);
}

#[test]
fn test_recorrido_completo_emite_telemetria_y_elige() {
    let mut cal = PresentCalibration::new(
        PresentMode::Mailbox,
        &[PresentMode::Fifo, PresentMode::Mailbox, PresentMode::Immediate],
    );
    assert!(!cal.finalizada());
    // El primer candidato (la referencia) queda pendiente de aplicar.
    assert_eq!(cal.tomar_pendiente(), Some(cal.actual()));
    assert_eq!(cal.tomar_pendiente(), None);

    let presents = [52.0, 18.0, 18.0, 12.0, 1.0];
    let mut lineas = Vec::new();
    for indice in 0..cal.total_tramos() {
        for _ in 0..SEGUNDOS_POR_TRAMO {
            lineas.extend(cal.registrar_segundo(presents[indice], 18.0));
        }
        // Al cerrar cada tramo queda pendiente la siguiente configuración.
        if indice + 1 < presents.len() {
            assert!(cal.tomar_pendiente().is_some(), "tramo {indice} sin siguiente");
        }
    }

    assert!(cal.finalizada());
    // 5 aperturas + 5 resultados + 1 veredicto.
    assert_eq!(lineas.len(), 11, "líneas: {lineas:#?}");
    assert!(lineas[0].starts_with("# tramo idx=0 lat=1 modo=Mailbox escala=1.00 referencia"));
    assert!(lineas[1].starts_with("# resultado idx=0 present=52.0ms"));
    // La sonda de escala (12 ms) se midió, pero no se adopta.
    assert!(
        lineas
            .iter()
            .any(|l| l.contains("idx=3") && l.contains("escala=0.50 sonda")),
        "falta el tramo de escala reducida: {lineas:#?}"
    );
    let veredicto = lineas.last().unwrap();
    assert!(veredicto.starts_with("# elegido lat=2 modo=Mailbox escala=1.00"), "veredicto: {veredicto}");
    assert_eq!(cal.actual().latency, 2);
    assert_eq!(cal.actual().scale, 1.0);

    // Terminada la calibración no se registran más ventanas.
    assert!(cal.registrar_segundo(99.0, 18.0).is_empty());
}

#[test]
fn test_cambio_de_orientacion_reinicia_y_luego_aborta() {
    let mut cal = PresentCalibration::new(PresentMode::Mailbox, &[PresentMode::Fifo]);
    assert!(cal.notificar_tamano((1200, 2000)).is_empty());

    // Un tramo medido y un cambio de orientación: se reinicia.
    for _ in 0..SEGUNDOS_POR_TRAMO {
        cal.registrar_segundo(52.0, 18.0);
    }
    let lineas = cal.notificar_tamano((2000, 1200));
    assert_eq!(lineas, vec!["# calibracion reiniciada (cambio de orientacion)".to_string()]);
    assert!(!cal.finalizada());
    assert_eq!(cal.actual().latency, 1);
    assert!(cal.tomar_pendiente().is_some());

    // Un segundo cambio ya no reinicia: cierra con lo medido.
    let lineas = cal.notificar_tamano((1200, 2000));
    assert_eq!(lineas.len(), 1);
    assert!(lineas[0].starts_with("# elegido lat=1"), "línea: {}", lineas[0]);
    assert!(lineas[0].contains("abortada por cambio de orientacion"));
    assert!(cal.finalizada());
}

#[test]
fn test_tamano_estable_no_emite_lineas() {
    let mut cal = PresentCalibration::new(PresentMode::Mailbox, &[PresentMode::Fifo]);
    assert!(cal.notificar_tamano((1200, 2000)).is_empty());
    assert!(cal.notificar_tamano((1200, 2000)).is_empty());
    assert!(!cal.finalizada());
}

/// Pruebas unitarias para el tope dinámico de resolución del swapchain.
/// Garantiza que en pantallas de alta densidad (2K/QHD) se reduzca proporcionalmente
/// el buffer para respetar el tope de 1280px (720p) sin sobrecargar la GPU ni memoria,
/// y que en pantallas ≤ 1280px opere al 100% nativo.
#[test]
fn test_calcular_escala_swapchain_dispositivos() {
    assert_eq!(TOPE_SWAPCHAIN_LANDSCAPE, 1280);
    assert_eq!(TOPE_SWAPCHAIN_PORTRAIT, 960);
    assert_eq!(TOPE_DIMENSION_SWAPCHAIN, 1280);

    // 1. Tablet del usuario: 1200x2000 retrato (tope 960) y 2000x1200 apaisado (tope 1280)
    let escala_portrait = calcular_escala_swapchain(1200, 2000);
    assert!((escala_portrait - 0.48).abs() < 1e-5);
    let rw_portrait = (1200.0f32 * escala_portrait).round() as u32;
    let rh_portrait = (2000.0f32 * escala_portrait).round() as u32;
    assert_eq!(rw_portrait, 576);
    assert_eq!(rh_portrait, 960);

    let escala_landscape = calcular_escala_swapchain(2000, 1200);
    assert!((escala_landscape - 0.64).abs() < 1e-5);
    let rw_landscape = (2000.0f32 * escala_landscape).round() as u32;
    let rh_landscape = (1200.0f32 * escala_landscape).round() as u32;
    assert_eq!(rw_landscape, 1280);
    assert_eq!(rh_landscape, 768);

    // 2. Teléfono FHD estándar (1080x1920 retrato -> tope 960; 1920x1080 apaisado -> tope 1280)
    let escala_fhd_p = calcular_escala_swapchain(1080, 1920);
    assert!((escala_fhd_p - 0.50).abs() < 1e-5);
    let rh_fhd_p = (1920.0f32 * escala_fhd_p).round() as u32;
    let rw_fhd_p = (1080.0f32 * escala_fhd_p).round() as u32;
    assert_eq!(rw_fhd_p, 540);
    assert_eq!(rh_fhd_p, 960);

    let escala_fhd_l = calcular_escala_swapchain(1920, 1080);
    let rw_fhd_l = (1920.0f32 * escala_fhd_l).round() as u32;
    assert_eq!(rw_fhd_l, 1280);

    // 3. Pantalla HD nativa apaisada (1280x720) -> 1.0 (nativo)
    let escala_hd_l = calcular_escala_swapchain(1280, 720);
    assert_eq!(escala_hd_l, 1.0);

    // 4. Pantallas pequeñas o retro (< 960) -> 1.0 (nativo)
    assert_eq!(calcular_escala_swapchain(800, 600), 1.0);
    assert_eq!(calcular_escala_swapchain(480, 800), 1.0);
    assert_eq!(calcular_escala_swapchain(240, 160), 1.0);
    assert_eq!(calcular_escala_swapchain(0, 0), 1.0);

    // 5. Pantalla extrema (ej. 4K/8K) -> clamp a 0.25 mínimo
    let escala_extrema = calcular_escala_swapchain(4000, 8000);
    assert_eq!(escala_extrema, 0.25);
}
