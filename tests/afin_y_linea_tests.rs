//! Pruebas de docs/propuestas/2026-10-17_diagnostico-afin-y-por-linea.md.
//!
//! Leen el informe que escribe `examples/ppu_afin_y_linea` en `volcados/afin-y-linea.txt`.
//! Se omiten si `FE_ROM` no está definida, igual que las demás pruebas del experimento.

use std::collections::HashSet;
use std::fs;
use std::path::PathBuf;

const DESDE: u32 = 290;
const HASTA: u32 = 310;

fn raiz() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn rom_disponible() -> bool {
    std::env::var("FE_ROM").is_ok()
}

fn leer_informe() -> String {
    let ruta = raiz().join("volcados").join("afin-y-linea.txt");
    fs::read_to_string(&ruta).unwrap_or_else(|_| {
        panic!(
            "debe existir {}: correr antes el ejemplo ppu_afin_y_linea",
            ruta.display()
        )
    })
}

/// Valor de `clave` en una fila como `cuadro 290 modo 0 afines - ...`.
fn campo<'a>(fila: &'a str, clave: &str) -> Option<&'a str> {
    let mut partes = fila.split_whitespace();
    while let Some(p) = partes.next() {
        if p == clave {
            return partes.next();
        }
    }
    None
}

/// Fila del informe con ese prefijo.
fn fila<'a>(texto: &'a str, inicio: &str) -> &'a str {
    texto
        .lines()
        .find(|l| l.starts_with(inicio))
        .unwrap_or_else(|| panic!("falta la fila `{inicio}` en el informe"))
}

/// Lista de cuadros del resumen (`-` si está vacía).
fn lista(texto: &str, inicio: &str) -> HashSet<u32> {
    let linea = fila(texto, inicio);
    let resto = linea[inicio.len()..].trim();
    if resto == "-" || resto.is_empty() {
        return HashSet::new();
    }
    resto
        .split(',')
        .map(|n| {
            n.trim()
                .parse()
                .unwrap_or_else(|_| panic!("cuadro inválido en `{inicio}`: {linea}"))
        })
        .collect()
}

#[test]
fn test_una_fila_por_cuadro() {
    if !rom_disponible() {
        eprintln!("omitida: falta FE_ROM");
        return;
    }
    let texto = leer_informe();
    let filas: Vec<&str> = texto.lines().filter(|l| l.starts_with("cuadro ")).collect();
    assert_eq!(
        filas.len(),
        (HASTA - DESDE + 1) as usize,
        "debe haber una fila por cuadro del rango {DESDE}-{HASTA}"
    );
    for (i, fila) in filas.iter().enumerate() {
        let esperado = DESDE + i as u32;
        assert_eq!(
            campo(fila, "cuadro").and_then(|n| n.parse::<u32>().ok()),
            Some(esperado),
            "las filas deben ir en orden y sin huecos: {fila}"
        );
        for clave in ["modo", "afines", "cambios", "lineas_paleta", "lineas_vram", "lineas_oam"] {
            assert!(
                campo(fila, clave).is_some(),
                "la fila debe traer `{clave}`: {fila}"
            );
        }
    }
}

#[test]
fn test_resumen_coincide_con_filas() {
    if !rom_disponible() {
        eprintln!("omitida: falta FE_ROM");
        return;
    }
    let texto = leer_informe();
    let afines_resumen = lista(&texto, "afines ");
    let cambios_resumen = lista(&texto, "con cambios por linea ");

    let mut afines_filas = HashSet::new();
    let mut cambios_filas = HashSet::new();
    for fila in texto.lines().filter(|l| l.starts_with("cuadro ")) {
        let n: u32 = campo(fila, "cuadro").and_then(|v| v.parse().ok()).expect("cuadro");
        if campo(fila, "afines") != Some("-") {
            afines_filas.insert(n);
        }
        let cambia = campo(fila, "cambios") != Some("-")
            || campo(fila, "lineas_paleta") != Some("0")
            || campo(fila, "lineas_vram") != Some("0")
            || campo(fila, "lineas_oam") != Some("0");
        if cambia {
            cambios_filas.insert(n);
        }
    }

    assert_eq!(
        afines_resumen, afines_filas,
        "la lista `afines` del resumen debe coincidir con las filas"
    );
    assert_eq!(
        cambios_resumen, cambios_filas,
        "la lista `con cambios por linea` del resumen debe coincidir con las filas"
    );
}

#[test]
fn test_control_coincide() {
    if !rom_disponible() {
        eprintln!("omitida: falta FE_ROM");
        return;
    }
    let texto = leer_informe();
    let control = fila(&texto, "control ");
    assert!(
        control.starts_with("control 0 "),
        "el control debe dar 0 píxeles distintos: {control}"
    );
}

#[test]
fn test_cuadro_sin_senales() {
    if !rom_disponible() {
        eprintln!("omitida: falta FE_ROM");
        return;
    }
    let texto = leer_informe();
    let afines = lista(&texto, "afines ");
    let cambios = lista(&texto, "con cambios por linea ");

    let sin_senales: Vec<u32> = texto
        .lines()
        .filter(|l| l.starts_with("cuadro "))
        .filter(|l| {
            campo(l, "afines") == Some("-")
                && campo(l, "cambios") == Some("-")
                && campo(l, "lineas_paleta") == Some("0")
                && campo(l, "lineas_vram") == Some("0")
                && campo(l, "lineas_oam") == Some("0")
        })
        .filter_map(|l| campo(l, "cuadro").and_then(|v| v.parse().ok()))
        .collect();

    assert!(
        !sin_senales.is_empty(),
        "el criterio necesita al menos un cuadro sin señales en el rango"
    );
    for n in sin_senales {
        assert!(!afines.contains(&n), "el cuadro {n} no debe estar en `afines`");
        assert!(
            !cambios.contains(&n),
            "el cuadro {n} no debe estar en `con cambios por linea`"
        );
    }
}

/// Blindaje: los recuentos por línea no pueden pasar de 160 ni el modo de 5.
#[test]
fn test_recuentos_en_rango() {
    if !rom_disponible() {
        eprintln!("omitida: falta FE_ROM");
        return;
    }
    let texto = leer_informe();
    for fila in texto.lines().filter(|l| l.starts_with("cuadro ")) {
        let modo: u32 = campo(fila, "modo").and_then(|v| v.parse().ok()).expect("modo");
        assert!(modo <= 5, "modo de vídeo fuera de rango: {fila}");
        for clave in ["lineas_paleta", "lineas_vram", "lineas_oam"] {
            let n: u32 = campo(fila, clave)
                .and_then(|v| v.parse().ok())
                .unwrap_or_else(|| panic!("falta {clave}: {fila}"));
            assert!(
                n <= 160,
                "`{clave}` no puede superar las 160 líneas: {fila}"
            );
        }
        if let Some(cambios) = campo(fila, "cambios") {
            if cambios != "-" {
                for par in cambios.split(',') {
                    let n: usize = par.split(':').nth(1).and_then(|v| v.parse().ok()).unwrap_or(0);
                    assert!(n <= 160, "recuento de cambios fuera de rango: {fila}");
                }
            }
        }
    }
}
