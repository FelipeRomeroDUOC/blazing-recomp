//! Pruebas de docs/propuestas/2026-10-17_capa-bajo-mezcla-ninguno-cuadro-1000.md.
//!
//! Leen el informe que escribe `examples/shim_capa_ninguno` en volcados/. Se omiten si `FE_ROM`
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
    let ruta = raiz().join("volcados").join("capa-ninguno-cuadro-1000.txt");
    fs::read_to_string(&ruta).unwrap_or_else(|_| {
        panic!(
            "debe existir {}: correr antes el ejemplo shim_capa_ninguno",
            ruta.display()
        )
    })
}

/// Número al final de la primera línea que empieza por `prefijo`, como `ninguno total: 2523`.
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
fn test_ninguno_es_2523() {
    if !rom_disponible() {
        eprintln!("omitida: falta FE_ROM");
        return;
    }
    let texto = leer_informe();
    assert_eq!(valor(&texto, "ninguno total"), Some(2523));
}

#[test]
fn test_capa_informe_completo() {
    if !rom_disponible() {
        eprintln!("omitida: falta FE_ROM");
        return;
    }
    let texto = leer_informe();
    for prefijo in [
        "ninguno sprite semi",
        "ninguno sprite no semi",
        "ninguno H50 BG0",
        "ninguno H50 BG1",
        "ninguno H50 BG3",
        "ninguno H51",
        "ninguno H52",
        "ninguno sin hipotesis",
    ] {
        assert!(
            valor(&texto, prefijo).is_some(),
            "debe existir la línea '{prefijo}'"
        );
    }
}
