//! Pruebas de docs/propuestas/2026-10-17_localizar-pixeles-obj-y-contenido-bg-cuadro-1000.md.
//!
//! Leen los informes de `examples/ppu_pixeles_obj` y `examples/ppu_contenido_bg` en volcados/.
//! Se omiten si `FE_ROM` no está definida, igual que las demás pruebas del experimento.

use std::fs;
use std::path::PathBuf;

fn raiz() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn rom_disponible() -> bool {
    std::env::var("FE_ROM").is_ok()
}

fn leer(nombre: &str) -> String {
    let ruta = raiz().join("volcados").join(nombre);
    fs::read_to_string(&ruta).unwrap_or_else(|_| {
        panic!(
            "debe existir {}: correr antes el ejemplo correspondiente",
            ruta.display()
        )
    })
}

/// Número al final de la primera línea que empieza por `prefijo`, como `obj aportacion total: 42`.
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
fn test_obj_suma_cuadra() {
    if !rom_disponible() {
        eprintln!("omitida: falta FE_ROM");
        return;
    }
    let texto = leer("pixeles-obj-cuadro-1000.txt");
    let total = valor(&texto, "obj aportacion total").expect("debe dar la aportación total");
    let dentro = valor(&texto, "obj aportacion dentro de cobertura")
        .expect("debe dar la aportación dentro de cobertura");
    let fuera = valor(&texto, "obj aportacion fuera de cobertura")
        .expect("debe dar la aportación fuera de cobertura");
    assert_eq!(
        dentro + fuera,
        total,
        "dentro + fuera debe dar la aportación total"
    );
}

#[test]
fn test_control_coincide_con_juego() {
    if !rom_disponible() {
        eprintln!("omitida: falta FE_ROM");
        return;
    }
    let texto = leer("pixeles-obj-cuadro-1000.txt");
    let control = valor(&texto, "obj control completa vs juego")
        .expect("debe dar la diferencia de control con el juego");
    assert_eq!(control, 0, "la variante completa debe coincidir con el juego");
}

#[test]
fn test_bg_tiene_tres_capas() {
    if !rom_disponible() {
        eprintln!("omitida: falta FE_ROM");
        return;
    }
    let texto = leer("contenido-bg-cuadro-1000.txt");
    for capa in ["BG0", "BG1", "BG3"] {
        let prefijo = format!("bg {capa} ");
        assert!(
            texto.lines().any(|l| l.starts_with(&prefijo)),
            "el informe debe dar la línea `bg {capa}`"
        );
    }
}

// --- BG2 (propuesta contenido-bg2-cuadro-1000) ---

fn bg2_raiz() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn bg2_rom_disponible() -> bool {
    std::env::var("FE_ROM").is_ok()
}

fn bg2_leer_informe() -> String {
    let ruta = bg2_raiz().join("volcados").join("contenido-bg2-cuadro-1000.txt");
    std::fs::read_to_string(&ruta).unwrap_or_else(|_| {
        panic!(
            "debe existir {}: correr antes el ejemplo ppu_contenido_bg",
            ruta.display()
        )
    })
}

/// Número al final de la primera línea que empieza por `prefijo`, como `bg2 opacos total: 12`.
fn bg2_valor(texto: &str, prefijo: &str) -> Option<usize> {
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
fn test_bg2_informe_completo() {
    if !bg2_rom_disponible() {
        eprintln!("omitida: falta FE_ROM");
        return;
    }
    let texto = bg2_leer_informe();
    for prefijo in [
        "bg2 tiles con datos",
        "bg2 opacos total",
        "bg2 opacos dentro aportacion",
    ] {
        assert!(
            bg2_valor(&texto, prefijo).is_some(),
            "debe existir la línea '{prefijo}'"
        );
    }
}

#[test]
fn test_bg2_cubre_aportacion() {
    if !bg2_rom_disponible() {
        eprintln!("omitida: falta FE_ROM");
        return;
    }
    let texto = bg2_leer_informe();
    assert_eq!(
        bg2_valor(&texto, "bg2 opacos dentro aportacion"),
        Some(5290),
        "BG2 debe ser opaco en los 5.290 píxeles de la aportación"
    );
}
