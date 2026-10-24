//! Pruebas de docs/propuestas/2026-10-17_shim-contra-fondo-vacio-cuadro-1000.md.
//!
//! Leen el informe que escribe `examples/shim_aportacion` en volcados/shim-fondo-cuadro-1000.txt.
//! Se omiten si `FE_ROM` no está definida, igual que las demás pruebas del experimento.

use std::fs;
use std::path::PathBuf;

fn raiz() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn rom_disponible() -> bool {
    std::env::var("FE_ROM").is_ok()
}

fn leer_informe() -> String {
    let ruta = raiz().join("volcados").join("shim-fondo-cuadro-1000.txt");
    fs::read_to_string(&ruta).unwrap_or_else(|_| {
        panic!(
            "debe existir {}: correr antes el ejemplo shim_aportacion",
            ruta.display()
        )
    })
}

/// Número al final de la primera línea que empieza por `prefijo`, como `aportacion total: 5290`.
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
fn test_shim_fondo_informe() {
    if !rom_disponible() {
        eprintln!("omitida: falta FE_ROM");
        return;
    }
    let texto = leer_informe();
    for prefijo in [
        "shim fin igual PPU V3",
        "shim linea igual PPU V3",
        "shim fin categoria otro",
    ] {
        assert!(
            valor(&texto, prefijo).is_some(),
            "debe existir la línea '{prefijo}'"
        );
    }
}

#[test]
fn test_shim_fondo_suma_total() {
    if !rom_disponible() {
        eprintln!("omitida: falta FE_ROM");
        return;
    }
    let texto = leer_informe();
    let total = valor(&texto, "aportacion total").expect("debe existir el total");
    let suma = valor(&texto, "shim fin suma").expect("debe existir la suma");
    assert_eq!(suma, total, "las categorías del shim de fin de cuadro deben sumar el total");
}
