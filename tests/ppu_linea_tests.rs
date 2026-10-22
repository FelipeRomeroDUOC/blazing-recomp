//! Pruebas de docs/propuestas/2026-10-17_captura-por-linea-cuadro-1000.md.
//!
//! Leen el informe que escribe `examples/ppu_linea` en `volcados/`. Se omiten si
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
    let ruta = raiz().join("volcados").join("replay-cuadro-1000.txt");
    fs::read_to_string(&ruta).unwrap_or_else(|_| {
        panic!(
            "debe existir {}: correr antes el ejemplo ppu_linea",
            ruta.display()
        )
    })
}

#[test]
fn test_replay_por_linea_coincide() {
    if !rom_disponible() {
        eprintln!("omitida: falta FE_ROM");
        return;
    }
    let texto = leer_informe();
    let resumen = texto.lines().next().expect("el informe debe tener resumen");
    assert!(
        resumen.contains("píxeles distintos 0 "),
        "el replay por línea debe dar 0 píxeles distintos: {resumen}"
    );
}

#[test]
fn test_informe_tiene_160_lineas() {
    if !rom_disponible() {
        eprintln!("omitida: falta FE_ROM");
        return;
    }
    let texto = leer_informe();
    let filas = texto.lines().filter(|l| l.starts_with("linea ")).count();
    assert_eq!(filas, 160, "el informe debe tener una fila por línea visible");
}
