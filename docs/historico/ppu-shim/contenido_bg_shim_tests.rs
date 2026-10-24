//! Pruebas de docs/propuestas/2026-10-17_contenido-bg-shim-frente-a-fondo-texto-cuadro-1000.md.
//!
//! Leen el informe que escribe `examples/shim_contenido_bg` en volcados/. Se omiten si `FE_ROM`
//! no está definida, igual que las demás pruebas del experimento.

use std::fs;
use std::path::PathBuf;

fn raiz() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn rom_disponible() -> bool {
    std::env::var("FE_ROM").is_ok()
}

fn leer_informe() -> String {
    let ruta = raiz().join("volcados").join("contenido-bg-shim-cuadro-1000.txt");
    fs::read_to_string(&ruta).unwrap_or_else(|_| {
        panic!(
            "debe existir {}: correr antes el ejemplo shim_contenido_bg",
            ruta.display()
        )
    })
}

/// Número al final de la primera línea que empieza por `prefijo`, como `BG0 opacos: 5290`.
fn valor(texto: &str, prefijo: &str) -> Option<usize> {
    texto
        .lines()
        .find(|l| l.starts_with(prefijo))?
        .rsplit(": ")
        .next()?
        .trim()
        .parse()
        .ok()
}

#[test]
fn test_reconstruccion_coincide() {
    if !rom_disponible() {
        eprintln!("omitida: falta FE_ROM");
        return;
    }
    let texto = leer_informe();
    assert_eq!(valor(&texto, "control reconstruccion V0"), Some(0));
    assert_eq!(valor(&texto, "control reconstruccion V1"), Some(0));
    assert_eq!(valor(&texto, "control paleta fondo"), Some(0));
}

#[test]
fn test_capas_informe_completo() {
    if !rom_disponible() {
        eprintln!("omitida: falta FE_ROM");
        return;
    }
    let texto = leer_informe();
    for k in 0..4 {
        for sufijo in ["opacos", "shim vs fin", "shim vs linea", "shim opaco sobre transparente"] {
            let prefijo = format!("BG{k} {sufijo}");
            assert!(
                valor(&texto, &prefijo).is_some(),
                "debe existir la línea '{prefijo}'"
            );
        }
    }
}

#[test]
fn test_lineas_distintas_rango() {
    if !rom_disponible() {
        eprintln!("omitida: falta FE_ROM");
        return;
    }
    let texto = leer_informe();
    let v = valor(&texto, "lineas con registros de fondo distintos")
        .expect("debe existir la línea de registros distintos");
    assert!(v <= 160, "el valor debe estar entre 0 y 160, es {v}");
}
