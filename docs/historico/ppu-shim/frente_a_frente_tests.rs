//! Pruebas de docs/propuestas/2026-10-17_frente-a-frente-del-estado-por-linea.md.
//!
//! Leen el informe que escribe `examples/ppu_frente_a_frente` en `volcados/frente-a-frente.txt`.
//! Se omiten si `FE_ROM` no está definida, igual que las demás pruebas del experimento.

use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;

/// Rango canónico del informe: el mismo que documenta la propuesta.
const DESDE: u32 = 6280;
const HASTA: u32 = 6300;
const PASO: u32 = 5;

fn raiz() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn rom_disponible() -> bool {
    std::env::var("FE_ROM").is_ok()
}

fn leer_informe() -> String {
    let ruta = raiz().join("volcados").join("frente-a-frente.txt");
    fs::read_to_string(&ruta).unwrap_or_else(|_| {
        panic!(
            "debe existir {}: correr antes el ejemplo ppu_frente_a_frente",
            ruta.display()
        )
    })
}

/// Una fila del CSV con sus columnas por nombre.
type Fila = BTreeMap<String, String>;

/// Cabecera del CSV, y filas ya interpretadas.
fn filas(texto: &str) -> (Vec<String>, Vec<Fila>) {
    let mut lineas = texto.lines().filter(|l| !l.trim().is_empty());
    let cabecera: Vec<String> = lineas
        .find(|l| l.starts_with("cuadro,"))
        .unwrap_or_else(|| panic!("falta la cabecera del CSV en el informe"))
        .split(',')
        .map(|c| c.trim().to_string())
        .collect();
    let mut salida = Vec::new();
    for linea in lineas {
        let valores: Vec<&str> = linea.split(',').collect();
        assert_eq!(
            valores.len(),
            cabecera.len(),
            "fila con {} columnas y la cabecera con {}: {linea}",
            valores.len(),
            cabecera.len()
        );
        let fila: Fila = cabecera
            .iter()
            .cloned()
            .zip(valores.iter().map(|v| v.to_string()))
            .collect();
        salida.push(fila);
    }
    (cabecera, salida)
}

fn numero(fila: &Fila, clave: &str) -> f64 {
    fila.get(clave)
        .unwrap_or_else(|| panic!("falta la columna `{clave}` en {fila:?}"))
        .parse()
        .unwrap_or_else(|_| panic!("`{clave}` no es un número en {fila:?}"))
}

/// Valor de una línea de resumen como `error medio fin 1.2345`.
fn resumen<'a>(texto: &'a str, clave: &str) -> &'a str {
    let linea = texto
        .lines()
        .find(|l| l.starts_with(clave))
        .unwrap_or_else(|| panic!("falta la línea de resumen `{clave}`"));
    linea[clave.len()..].trim()
}

#[test]
fn test_una_fila_por_cuadro() {
    if !rom_disponible() {
        eprintln!("omitida: falta FE_ROM");
        return;
    }
    let texto = leer_informe();
    let (_, filas) = filas(&texto);
    let esperados: Vec<u32> = (DESDE..=HASTA).step_by(PASO as usize).collect();
    assert_eq!(
        filas.len(),
        esperados.len(),
        "debe haber una fila por cuadro muestreado del rango {DESDE}-{HASTA} paso {PASO}"
    );
    for (fila, esperado) in filas.iter().zip(esperados.iter()) {
        assert_eq!(
            numero(fila, "cuadro") as u32,
            *esperado,
            "las filas deben ir en orden y sin huecos"
        );
    }
}

#[test]
fn test_formato_del_informe() {
    if !rom_disponible() {
        eprintln!("omitida: falta FE_ROM");
        return;
    }
    let texto = leer_informe();
    let (cabecera, filas) = filas(&texto);
    for clave in [
        "cuadro",
        "estados",
        "lineas_con_cambio",
        "lineas_ajenas_al_fin",
        "pct_fin",
        "pct_linea",
        "delta_pct",
        "primera_fin",
        "primera_linea",
        "bldcnt",
        "blend_lineas",
    ] {
        assert!(
            cabecera.iter().any(|c| c == clave),
            "la cabecera debe traer `{clave}`: {cabecera:?}"
        );
    }
    for fila in &filas {
        let (fin, linea, delta) = (
            numero(fila, "pct_fin"),
            numero(fila, "pct_linea"),
            numero(fila, "delta_pct"),
        );
        for (clave, valor) in [("pct_fin", fin), ("pct_linea", linea)] {
            assert!(
                (0.0..=100.0).contains(&valor),
                "`{clave}` fuera de [0, 100]: {valor}"
            );
        }
        assert!(
            (delta - (fin - linea)).abs() < 0.0003,
            "`delta_pct` debe ser `pct_fin - pct_linea`: {delta} vs {:.4}",
            fin - linea
        );
        assert!(
            numero(fila, "estados") >= 1.0,
            "un cuadro siempre tiene al menos un estado"
        );
        for clave in ["lineas_con_cambio", "lineas_ajenas_al_fin", "blend_lineas"] {
            let n = numero(fila, clave);
            assert!(
                (0.0..=160.0).contains(&n),
                "`{clave}` no puede salir de 0..=160: {n}"
            );
        }
        for clave in ["primera_fin", "primera_linea"] {
            let valor = fila.get(clave).expect("columna de primera línea");
            if valor != "-" {
                let n: u32 = valor
                    .parse()
                    .unwrap_or_else(|_| panic!("`{clave}` no es un número ni `-`: {valor}"));
                assert!(n < 160, "`{clave}` debe ser una línea visible: {n}");
            }
        }
    }
}

#[test]
fn test_resumen_coincide() {
    if !rom_disponible() {
        eprintln!("omitida: falta FE_ROM");
        return;
    }
    let texto = leer_informe();
    let (_, filas) = filas(&texto);
    let n = filas.len() as f64;
    assert_eq!(
        resumen(&texto, "resumen cuadros"),
        format!("{} desde {DESDE} hasta {HASTA} paso {PASO}", filas.len()),
        "el resumen debe declarar el rango que se midió"
    );

    let media_fin: f64 = filas.iter().map(|f| numero(f, "pct_fin")).sum::<f64>() / n;
    let media_linea: f64 = filas.iter().map(|f| numero(f, "pct_linea")).sum::<f64>() / n;

    let declarado_fin: f64 = resumen(&texto, "error medio fin").parse().expect("media fin");
    let declarado_linea: f64 = resumen(&texto, "error medio linea")
        .parse()
        .expect("media linea");
    let reduccion: f64 = resumen(&texto, "reduccion").parse().expect("reduccion");
    let residuo: f64 = resumen(&texto, "residuo").parse().expect("residuo");

    assert!(
        (declarado_fin - media_fin).abs() < 0.001,
        "la media de `pct_fin` no coincide: {declarado_fin} vs {media_fin}"
    );
    assert!(
        (declarado_linea - media_linea).abs() < 0.001,
        "la media de `pct_linea` no coincide: {declarado_linea} vs {media_linea}"
    );
    assert!(
        (reduccion - (media_fin - media_linea)).abs() < 0.001,
        "la reducción debe ser la diferencia de medias: {reduccion}"
    );
    assert!(
        (residuo - media_linea).abs() < 0.001,
        "el residuo debe ser la media con estado por línea: {residuo}"
    );
}

/// Blindaje: si B fuese una copia de A, todos los cuadros tendrían un solo estado y la prueba falla.
#[test]
fn test_estado_por_linea_se_ejercita() {
    if !rom_disponible() {
        eprintln!("omitida: falta FE_ROM");
        return;
    }
    let texto = leer_informe();
    let (_, filas) = filas(&texto);
    let con_varios = filas.iter().any(|f| {
        numero(f, "estados") > 1.0 && numero(f, "lineas_ajenas_al_fin") > 0.0
    });
    assert!(
        con_varios,
        "el rango debe tener al menos un cuadro con más de un estado por línea"
    );
}
