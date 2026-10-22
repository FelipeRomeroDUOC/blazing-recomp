//! Pruebas de docs/propuestas/2026-10-17_shim-contra-aportacion-cuadro-1000.md.
//!
//! Leen el informe que escribe `examples/shim_aportacion` en volcados/. Se omiten si `FE_ROM`
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
    let ruta = raiz().join("volcados").join("shim-aportacion-cuadro-1000.txt");
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
fn test_shim_determinista() {
    if !rom_disponible() {
        eprintln!("omitida: falta FE_ROM");
        return;
    }
    let texto = leer_informe();
    assert_eq!(
        valor(&texto, "control determinismo shim"),
        Some(0),
        "el shim debe dar el mismo resultado en dos renderizados del mismo estado"
    );
}

#[test]
fn test_shim_suma_total() {
    if !rom_disponible() {
        eprintln!("omitida: falta FE_ROM");
        return;
    }
    let texto = leer_informe();
    let total = valor(&texto, "aportacion total").expect("debe existir el total");
    let suma = valor(&texto, "shim fin suma").expect("debe existir la suma");
    assert_eq!(suma, total, "las categorías del shim de fin de cuadro deben sumar el total");
}

#[test]
fn test_shim_informe_completo() {
    if !rom_disponible() {
        eprintln!("omitida: falta FE_ROM");
        return;
    }
    let texto = leer_informe();
    for prefijo in [
        "shim fin igual PPU V0",
        "shim fin V0 distinto de V1",
        "shim linea igual PPU V0",
    ] {
        assert!(
            valor(&texto, prefijo).is_some(),
            "debe existir la línea '{prefijo}'"
        );
    }
}
