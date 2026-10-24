//! Pruebas de docs/propuestas/2026-10-17_origen-del-color-x-en-el-shim-cuadro-1000.md.
//!
//! Leen el informe que escribe `examples/shim_origen_x` en volcados/. Se omiten si `FE_ROM`
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
    let ruta = raiz().join("volcados").join("origen-x-cuadro-1000.txt");
    fs::read_to_string(&ruta).unwrap_or_else(|_| {
        panic!(
            "debe existir {}: correr antes el ejemplo shim_origen_x",
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
fn test_reconstruccion_coincide() {
    if !rom_disponible() {
        eprintln!("omitida: falta FE_ROM");
        return;
    }
    let texto = leer_informe();
    assert_eq!(valor(&texto, "control reconstruccion V0"), Some(0));
    assert_eq!(valor(&texto, "control reconstruccion V1"), Some(0));
}

#[test]
fn test_origen_suma_total() {
    if !rom_disponible() {
        eprintln!("omitida: falta FE_ROM");
        return;
    }
    let texto = leer_informe();
    let total = valor(&texto, "aportacion total").expect("debe existir el total");
    let suma = valor(&texto, "origen suma").expect("debe existir la suma");
    assert_eq!(suma, total, "cada píxel debe quedar en A, B, C o ninguno");
}

#[test]
fn test_origen_informe_completo() {
    if !rom_disponible() {
        eprintln!("omitida: falta FE_ROM");
        return;
    }
    let texto = leer_informe();
    for prefijo in [
        "origen A mezcla",
        "origen B backdrop",
        "origen C otra paleta",
        "origen ninguno",
    ] {
        assert!(
            valor(&texto, prefijo).is_some(),
            "debe existir la línea '{prefijo}'"
        );
    }
}
