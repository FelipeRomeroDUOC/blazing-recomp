//! Pruebas de docs/propuestas/2026-10-17_capas-por-aportacion-cuadro-1000.md.
//!
//! Leen el informe que escribe `examples/ppu_capas_linea` en `volcados/`. Se omiten si
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
    let ruta = raiz().join("volcados").join("capas-cuadro-1000.txt");
    fs::read_to_string(&ruta).unwrap_or_else(|_| {
        panic!(
            "debe existir {}: correr antes el ejemplo ppu_capas_linea",
            ruta.display()
        )
    })
}

/// Valor del campo `clave` en una fila como `capa BG0 activa 1 aportacion 42 ...`.
fn campo(fila: &str, clave: &str) -> Option<usize> {
    let mut partes = fila.split_whitespace();
    while let Some(p) = partes.next() {
        if p == clave {
            return partes.next()?.parse().ok();
        }
    }
    None
}

#[test]
fn test_variante_completa_coincide() {
    if !rom_disponible() {
        eprintln!("omitida: falta FE_ROM");
        return;
    }
    let texto = leer_informe();
    let resumen = texto.lines().next().expect("el informe debe tener resumen");
    assert!(
        resumen.contains("juego = 0 píxeles"),
        "la variante completa debe coincidir con el juego: {resumen}"
    );
}

#[test]
fn test_informe_capas_tiene_cinco_filas() {
    if !rom_disponible() {
        eprintln!("omitida: falta FE_ROM");
        return;
    }
    let texto = leer_informe();
    let filas: Vec<&str> = texto.lines().filter(|l| l.starts_with("capa ")).collect();
    assert_eq!(filas.len(), 5, "el informe debe tener 5 filas de capa");
    for fila in &filas {
        let aportacion = campo(fila, "aportacion").expect("cada fila debe dar aportacion");
        let cobertura = campo(fila, "cobertura").expect("cada fila debe dar cobertura");
        assert!(aportacion <= 38_400, "aportacion fuera de rango: {fila}");
        assert!(cobertura <= 38_400, "cobertura fuera de rango: {fila}");
    }
}

#[test]
fn test_capa_apagada_no_aporta() {
    if !rom_disponible() {
        eprintln!("omitida: falta FE_ROM");
        return;
    }
    let texto = leer_informe();
    for fila in texto.lines().filter(|l| l.starts_with("capa ")) {
        if campo(fila, "activa") == Some(0) {
            assert_eq!(
                campo(fila, "aportacion"),
                Some(0),
                "una capa apagada en todo el cuadro no puede aportar píxeles: {fila}"
            );
        }
    }
}
