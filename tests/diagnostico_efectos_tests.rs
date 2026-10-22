//! Pruebas de docs/propuestas/2026-10-17_diagnostico-efectos-y-ventanas-cuadro-1000.md.
//!
//! Leen el informe que escribe `examples/ppu_efectos_ventanas` en `volcados/`. Se omiten si
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
    let ruta = raiz().join("volcados").join("efectos-ventanas-cuadro-1000.txt");
    fs::read_to_string(&ruta).unwrap_or_else(|_| {
        panic!(
            "debe existir {}: correr antes el ejemplo ppu_efectos_ventanas",
            ruta.display()
        )
    })
}

/// Número al final de una línea como `ventanas diferencia completa vs sin ventanas: 42`.
fn valor_final(linea: &str) -> Option<usize> {
    linea.rsplit(": ").next()?.trim().parse().ok()
}

#[test]
fn test_informe_separa_efectos() {
    if !rom_disponible() {
        eprintln!("omitida: falta FE_ROM");
        return;
    }
    let texto = leer_informe();
    assert!(
        texto.lines().any(|l| l.starts_with("efectos OBJ con efectos")),
        "el informe debe dar la aportación de OBJ con efectos"
    );
    assert!(
        texto.lines().any(|l| l.starts_with("efectos OBJ sin efectos")),
        "el informe debe dar la aportación de OBJ sin efectos (BLDCNT = 0)"
    );
}

#[test]
fn test_informe_separa_ventanas() {
    if !rom_disponible() {
        eprintln!("omitida: falta FE_ROM");
        return;
    }
    let texto = leer_informe();
    for capa in ["BG0", "BG1", "BG3"] {
        let prefijo = format!("ventanas {capa} ");
        assert!(
            texto.lines().any(|l| l.starts_with(&prefijo)),
            "el informe debe dar la línea `ventanas {capa}`"
        );
    }
}

#[test]
fn test_sin_ventanas_no_cambia_sin_lineas() {
    if !rom_disponible() {
        eprintln!("omitida: falta FE_ROM");
        return;
    }
    let texto = leer_informe();
    let lineas_con_ventanas = texto
        .lines()
        .find(|l| l.starts_with("ventanas lineas con ventanas activas"))
        .and_then(valor_final)
        .expect("el informe debe dar el número de líneas con ventanas");
    let diferencia = texto
        .lines()
        .find(|l| l.starts_with("ventanas diferencia completa vs sin ventanas"))
        .and_then(valor_final)
        .expect("el informe debe dar la diferencia con y sin ventanas");
    if lineas_con_ventanas == 0 {
        assert_eq!(
            diferencia, 0,
            "sin líneas con ventanas, quitarlas no puede cambiar la imagen"
        );
    }
}
