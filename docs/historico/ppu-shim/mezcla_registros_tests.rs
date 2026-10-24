//! Pruebas de docs/propuestas/2026-10-18_bldcnt-bldalpha-cuadro-1000.md.
//!
//! Leen volcados/mezcla-registros-cuadro-1000.txt, que escribe el ejemplo shim_mezcla_registros.
//! Se omiten si no está definida FE_ROM.

use std::fs;

fn leer() -> Option<String> {
    if std::env::var("FE_ROM").is_err() {
        eprintln!("omitido: FE_ROM no está definida");
        return None;
    }
    Some(
        fs::read_to_string("volcados/mezcla-registros-cuadro-1000.txt")
            .expect("falta el informe: ejecuta primero shim_mezcla_registros"),
    )
}

#[test]
fn test_control() {
    let Some(t) = leer() else { return };
    assert!(
        t.contains("control aportacion total X: 5290 (esperado 5290)"),
        "el total de X debe ser 5290"
    );
}

#[test]
fn test_suma_por_linea() {
    let Some(t) = leer() else { return };
    let linea = t
        .lines()
        .find(|l| l.starts_with("distribucion EVA EVB por linea"))
        .expect("falta la distribución por línea");
    let resto = linea.split_once("): ").map(|x| x.1).unwrap_or("");
    let suma: usize = resto
        .split_whitespace()
        .filter_map(|par| par.rsplit(':').next()?.parse::<usize>().ok())
        .sum();
    assert_eq!(suma, 5290, "la suma por línea debe ser el total de X");
}
