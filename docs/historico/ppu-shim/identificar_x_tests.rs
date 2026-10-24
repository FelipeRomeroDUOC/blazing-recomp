//! Pruebas de docs/propuestas/2026-10-17_identificar-color-x-del-shim-cuadro-1000.md.
//!
//! Leen el informe que escribe `examples/shim_capa_x` en volcados/. Se omiten si `FE_ROM`
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
    let ruta = raiz().join("volcados").join("identificar-x-cuadro-1000.txt");
    fs::read_to_string(&ruta).unwrap_or_else(|_| {
        panic!(
            "debe existir {}: correr antes el ejemplo shim_capa_x",
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
fn test_x_suma_total() {
    if !rom_disponible() {
        eprintln!("omitida: falta FE_ROM");
        return;
    }
    let texto = leer_informe();
    let total = valor(&texto, "aportacion total").expect("debe existir el total");
    let suma = valor(&texto, "x suma").expect("debe existir la suma");
    assert_eq!(suma, total, "el color X debe quedar asignado a un fondo o a ninguno");
}

#[test]
fn test_x_informe_completo() {
    if !rom_disponible() {
        eprintln!("omitida: falta FE_ROM");
        return;
    }
    let texto = leer_informe();
    for prefijo in [
        "x coincide BG0",
        "x coincide BG1",
        "x coincide BG2",
        "x coincide BG3",
        "x sin fondo",
        "prioridad empate",
    ] {
        assert!(
            valor(&texto, prefijo).is_some(),
            "debe existir la línea '{prefijo}'"
        );
    }
}
