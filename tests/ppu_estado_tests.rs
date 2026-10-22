//! Pruebas de docs/propuestas/2026-10-17_buscar-estado-faltante-cuadro-1000.md.
//!
//! Leen los informes que escribe `examples/ppu_estado` en `volcados/`. Se omiten si
//! `FE_ROM` no está definida, igual que las pruebas del desglose por capa.

use std::fs;
use std::path::PathBuf;

fn raiz() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn rom_disponible() -> bool {
    std::env::var("FE_ROM").is_ok()
}

/// Extrae el número que sigue a `etiqueta` en `linea` (por ejemplo "E2 copia completa = ").
fn numero_tras(linea: &str, etiqueta: &str) -> Option<usize> {
    linea
        .split(etiqueta)
        .nth(1)?
        .split_whitespace()
        .next()?
        .parse()
        .ok()
}

#[test]
fn test_informe_estado_tiene_conteo() {
    if !rom_disponible() {
        eprintln!("omitida: falta FE_ROM");
        return;
    }
    let ruta = raiz().join("volcados").join("estado-cuadro-1000.txt");
    let texto = fs::read_to_string(&ruta).unwrap_or_else(|_| {
        panic!(
            "debe existir {}: correr antes el ejemplo ppu_estado",
            ruta.display()
        )
    });
    let linea = texto
        .lines()
        .next()
        .expect("el informe debe tener al menos una línea");

    let e1 = numero_tras(linea, "E1 copia parcial = ").expect("el informe debe dar E1");
    let e2 = numero_tras(linea, "E2 copia completa = ").expect("el informe debe dar E2");
    assert!(e1 <= 38_400, "E1 fuera de rango: {e1}");
    assert!(e2 <= 38_400, "E2 fuera de rango: {e2}");
}

/// Solo existe si la copia completa dio 0 (ver paso 6 de la propuesta). Si el informe de
/// bisección no existe, la prueba se omite en vez de fallar.
#[test]
fn test_biseccion_deja_un_campo_minimo() {
    if !rom_disponible() {
        eprintln!("omitida: falta FE_ROM");
        return;
    }
    let ruta = raiz().join("volcados").join("biseccion-cuadro-1000.txt");
    if !ruta.exists() {
        eprintln!("omitida: la bisección no se ha ejecutado (solo corre si E2 da 0)");
        return;
    }
    let texto = fs::read_to_string(&ruta).expect("debe poder leerse el informe de bisección");
    assert!(
        texto.lines().any(|l| l.starts_with("campo minimo:")),
        "el informe de bisección debe nombrar el campo mínimo con la línea `campo minimo: ...`"
    );
}
