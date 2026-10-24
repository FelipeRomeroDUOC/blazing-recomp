//! Pruebas de docs/propuestas/2026-10-17_camino-rapido-contra-general-cuadro-1000.md.
//!
//! Leen el informe que escribe `examples/ppu_camino_general` en volcados/. Se omiten si
//! `FE_ROM` no está definida, igual que las demás pruebas del experimento.

use std::fs;
use std::path::PathBuf;

fn raiz() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn rom_disponible() -> bool {
    std::env::var("FE_ROM").is_ok()
}

fn leer_informe() -> String {
    let ruta = raiz().join("volcados").join("camino-general-cuadro-1000.txt");
    fs::read_to_string(&ruta).unwrap_or_else(|_| {
        panic!(
            "debe existir {}: correr antes el ejemplo ppu_camino_general",
            ruta.display()
        )
    })
}

/// Número al final de la primera línea que empieza por `prefijo`, como `camino D0: 0`.
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
fn test_informe_da_diferencias() {
    if !rom_disponible() {
        eprintln!("omitida: falta FE_ROM");
        return;
    }
    let texto = leer_informe();
    assert!(valor(&texto, "camino D0").is_some(), "debe dar D0 como entero");
    assert!(valor(&texto, "camino D1").is_some(), "debe dar D1 como entero");
}

#[test]
fn test_control_coincide_con_juego() {
    if !rom_disponible() {
        eprintln!("omitida: falta FE_ROM");
        return;
    }
    let texto = leer_informe();
    assert_eq!(
        valor(&texto, "camino control"),
        Some(0),
        "el camino por defecto debe coincidir con el juego"
    );
}

#[test]
fn test_aportaciones_separadas() {
    if !rom_disponible() {
        eprintln!("omitida: falta FE_ROM");
        return;
    }
    let texto = leer_informe();
    assert!(
        valor(&texto, "camino aportacion rapida").is_some(),
        "debe dar la aportación con el camino rápido"
    );
    assert!(
        valor(&texto, "camino aportacion general").is_some(),
        "debe dar la aportación con el camino general"
    );
}
