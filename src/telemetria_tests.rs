//! Pruebas del contrato de la telemetría (`src/telemetria.rs`).
//!
//! Cubren el esquema declarado, la forma impresa de cada tipo, el sello de versión
//! y los tres fallos con los que `escribir_linea` puede toparse. El test de oro que
//! ata este escritor con el lector de Python vive en `tests/telemetria_contrato_tests.rs`.

use super::*;
use std::collections::BTreeMap;

/// Los 23 valores de una línea de la versión 2, con cifras reconocibles.
fn valores_completos() -> BTreeMap<&'static str, Valor> {
    let mut valores = BTreeMap::new();
    valores.insert("t", Valor::Entero(1));
    valores.insert("fps", Valor::Real(60.0));
    valores.insert("ejec", Valor::RealReal(13.6, 21.9));
    valores.insert("ppu", Valor::Real(7.7));
    valores.insert("ppubg", Valor::Real(3.3));
    valores.insert("ppuspr", Valor::Real(1.1));
    valores.insert("ppucomp", Valor::Real(3.3));
    valores.insert("apu", Valor::Real(1.5));
    valores.insert("lecturas", Valor::Entero(6581));
    valores.insert("escrituras", Valor::Entero(4206));
    valores.insert("color", Valor::RealReal(1.2, 1.3));
    valores.insert("dibujo", Valor::RealReal(7.0, 15.5));
    valores.insert("espera", Valor::RealReal(0.0, 0.0));
    valores.insert("subida", Valor::Real(0.5));
    valores.insert("codificacion", Valor::Real(1.2));
    valores.insert("present", Valor::Real(5.4));
    valores.insert("presentmax", Valor::Real(14.0));
    valores.insert("nativos", Valor::Entero(4945));
    valores.insert("interprete", Valor::Entero(2));
    valores.insert("vacias", Valor::Entero(1));
    valores.insert("ventana", Valor::EnteroEntero(1280, 720));
    valores.insert("pump", Valor::Entero(2));
    valores.insert("pumpm", Valor::Entero(480));
    valores
}

#[test]
fn test_el_esquema_declara_los_23_campos_en_orden_canonico() {
    let esquema = esquema().expect("el esquema embebido tiene que analizarse");

    let nombres: Vec<&str> = esquema.campos().iter().map(|campo| campo.nombre).collect();
    assert_eq!(nombres.len(), 23, "la versión 2 tiene 23 campos: {nombres:?}");
    assert_eq!(&nombres[..3], &["t", "fps", "ejec"]);
    assert_eq!(&nombres[21..], &["pump", "pumpm"]);
    assert_eq!(esquema.sello_clave(), "formato");
    assert_eq!(esquema.sello_version(), 2);
}

#[test]
fn test_la_linea_sale_en_orden_canonico_y_remata_con_el_sello() {
    let linea = escribir_linea(&valores_completos()).expect("los 23 valores declarados deben escribir");

    assert_eq!(
        linea,
        "t=1s fps=60.0 ejec=13.6/21.9 ppu=7.7 ppubg=3.3 ppuspr=1.1 ppucomp=3.3 apu=1.5 \
lecturas=6581 escrituras=4206 color=1.2/1.3 dibujo=7.0/15.5 espera=0.0/0.0 subida=0.5 \
codificacion=1.2 present=5.4 presentmax=14.0 nativos=4945 interprete=2 vacias=1 \
ventana=1280x720 pump=2 pumpm=480 formato=2"
    );
}

#[test]
fn test_falta_un_valor_se_nombra_el_campo() {
    let mut valores = valores_completos();
    valores.remove("ppuspr");

    let error = escribir_linea(&valores).expect_err("sin ppuspr la línea no se puede escribir");
    assert!(
        error.contains("falta el valor del campo 'ppuspr'"),
        "el error tiene que nombrar el campo que falta: {error}"
    );
}

#[test]
fn test_un_valor_que_el_esquema_no_declara_es_un_error() {
    let mut valores = valores_completos();
    valores.insert("zzz", Valor::Entero(1));

    let error = escribir_linea(&valores).expect_err("un campo fuera del esquema no se puede escribir");
    assert!(
        error.contains("el valor 'zzz' no está en el esquema"),
        "el error tiene que nombrar el valor sobrante: {error}"
    );
}

#[test]
fn test_un_valor_que_no_encaja_con_su_tipo_es_un_error() {
    let mut valores = valores_completos();
    valores.insert("fps", Valor::Entero(60));

    let error = escribir_linea(&valores).expect_err("fps es real, no entero");
    assert!(
        error.contains("el valor 'fps' no encaja con el tipo Real"),
        "el error tiene que nombrar el campo y el tipo: {error}"
    );
}

#[test]
fn test_el_tipo_entero_s_imprime_el_sufijo() {
    let mut valores = valores_completos();
    valores.insert("t", Valor::Entero(45));

    let linea = escribir_linea(&valores).expect("la línea debe escribirse");
    assert!(
        linea.starts_with("t=45s "),
        "el tipo entero_s imprime el sufijo s: {linea}"
    );
}

#[test]
fn test_un_esquema_mal_formado_es_un_error_y_no_un_panico() {
    let roto = "version | 1 | hereda: - | anade: t, fps | prueba\ncampo | t | entero_s | s | segundo\n";

    let error = analizar(roto).expect_err("faltan los campos que el bloque declara");
    assert!(
        error.contains("declara añadir") && error.contains("recoge"),
        "el error tiene que explicar la discrepancia: {error}"
    );
}

#[test]
fn test_un_tipo_desconocido_en_el_esquema_es_un_error() {
    let roto = "version | 1 | hereda: - | anade: t | prueba\ncampo | t | flotante | s | segundo\n";

    let error = analizar(roto).expect_err("'flotante' no es un tipo del contrato");
    assert!(
        error.contains("tipo desconocido") && error.contains("flotante"),
        "el error tiene que nombrar el tipo: {error}"
    );
}
