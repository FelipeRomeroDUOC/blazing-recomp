//! Pruebas de docs/propuestas/2026-10-17_orden-de-pintado-del-shim-cuadro-1000.md.
//!
//! Leen el informe que escribe `examples/shim_sprite_debajo` en volcados/. Se omiten si `FE_ROM`
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
    let ruta = raiz().join("volcados").join("sprite-debajo-cuadro-1000.txt");
    fs::read_to_string(&ruta).unwrap_or_else(|_| {
        panic!(
            "debe existir {}: correr antes el ejemplo shim_sprite_debajo",
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
    assert_eq!(valor(&texto, "control ganador"), Some(0));
}

#[test]
fn test_grupo_suma() {
    if !rom_disponible() {
        eprintln!("omitida: falta FE_ROM");
        return;
    }
    let texto = leer_informe();
    assert_eq!(valor(&texto, "ninguno total"), Some(2523));
    let total = valor(&texto, "ninguno total").expect("debe existir el total");
    let h60 = valor(&texto, "ninguno H60").expect("debe existir H60");
    let h61 = valor(&texto, "ninguno H61").expect("debe existir H61");
    let sin = valor(&texto, "ninguno sin hipotesis").expect("debe existir sin hipótesis");
    assert_eq!(h60 + h61 + sin, total, "H60, H61 y sin hipótesis deben particionar el grupo");
}

#[test]
fn test_sprite_informe_completo() {
    if !rom_disponible() {
        eprintln!("omitida: falta FE_ROM");
        return;
    }
    let texto = leer_informe();
    for prefijo in [
        "ninguno H60",
        "ninguno H61",
        "ninguno sin hipotesis",
        "modelo vs shim pantalla",
    ] {
        assert!(
            valor(&texto, prefijo).is_some(),
            "debe existir la línea '{prefijo}'"
        );
    }
}
