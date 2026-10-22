//! Pruebas de docs/propuestas/2026-10-17_capa-ganadora-por-pixel-obj-cuadro-1000.md.
//!
//! Leen el informe que escribe `examples/ppu_capa_ganadora` en volcados/. Se omiten si
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
    let ruta = raiz().join("volcados").join("capa-ganadora-cuadro-1000.txt");
    fs::read_to_string(&ruta).unwrap_or_else(|_| {
        panic!(
            "debe existir {}: correr antes el ejemplo ppu_capa_ganadora",
            ruta.display()
        )
    })
}

/// Número al final de la primera línea que empieza por `prefijo`, como `capa categoria H15: 3`.
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
    assert_eq!(
        valor(&texto, "control reconstruccion V0"),
        Some(0),
        "la reconstrucción de V0 debe coincidir con la PPU en todos los píxeles"
    );
    assert_eq!(
        valor(&texto, "control reconstruccion V1"),
        Some(0),
        "la reconstrucción de V1 debe coincidir con la PPU en todos los píxeles"
    );
}

#[test]
fn test_categorias_suman_total() {
    if !rom_disponible() {
        eprintln!("omitida: falta FE_ROM");
        return;
    }
    let texto = leer_informe();
    let total = valor(&texto, "capa aportacion fuera total").expect("debe existir el total");
    let suma = valor(&texto, "capa categoria suma").expect("debe existir la suma");
    assert_eq!(suma, total, "las categorías deben sumar el total fuera de cobertura");
}

#[test]
fn test_informe_da_categorias() {
    if !rom_disponible() {
        eprintln!("omitida: falta FE_ROM");
        return;
    }
    let texto = leer_informe();
    for h in ["H15", "H16", "H17"] {
        assert!(
            valor(&texto, &format!("capa categoria {h}")).is_some(),
            "debe existir la categoría {h}"
        );
    }
}

/// Lee el informe de la capa debajo del sprite (propuesta capa-debajo-del-sprite-cuadro-1000).
fn leer_capa_debajo() -> String {
    let ruta = raiz().join("volcados").join("capa-debajo-cuadro-1000.txt");
    fs::read_to_string(&ruta).unwrap_or_else(|_| {
        panic!(
            "debe existir {}: correr antes el ejemplo ppu_capa_ganadora",
            ruta.display()
        )
    })
}

#[test]
fn test_capa_debajo_suma_total() {
    if !rom_disponible() {
        eprintln!("omitida: falta FE_ROM");
        return;
    }
    let texto = leer_capa_debajo();
    let total = valor(&texto, "capa debajo total").expect("debe existir el total");
    let suma = valor(&texto, "capa debajo suma").expect("debe existir la suma");
    assert_eq!(suma, total, "las capas debajo deben sumar el total");
}

#[test]
fn test_capa_debajo_color() {
    if !rom_disponible() {
        eprintln!("omitida: falta FE_ROM");
        return;
    }
    let texto = leer_capa_debajo();
    assert!(
        valor(&texto, "capa debajo color igual V3").is_some(),
        "debe existir la comparación con V3"
    );
    assert_eq!(
        valor(&texto, "capa debajo color igual sprite"),
        Some(0),
        "V1 nunca debe mostrar el color del sprite ganador"
    );
}
