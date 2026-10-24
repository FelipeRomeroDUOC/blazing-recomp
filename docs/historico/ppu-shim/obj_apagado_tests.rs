//! Pruebas de docs/propuestas/2026-10-18_decode-oam-respeta-obj-apagado-cuadro-1000.md.
//!
//! Leen volcados/obj-antes-cuadro-1000.txt y volcados/obj-despues-cuadro-1000.txt, que escribe
//! el ejemplo shim_obj_apagado. Se omiten si no está definida FE_ROM.

use std::fs;

fn leer(ruta: &str) -> Option<String> {
    if std::env::var("FE_ROM").is_err() {
        eprintln!("omitido: FE_ROM no está definida");
        return None;
    }
    match fs::read_to_string(ruta) {
        Ok(t) => Some(t),
        Err(_) => panic!("falta {ruta}: ejecuta primero shim_obj_apagado"),
    }
}

fn valor(texto: &str, clave: &str) -> String {
    texto
        .lines()
        .find(|l| l.starts_with(clave))
        .and_then(|l| l.rsplit(':').next())
        .map(|v| v.trim().to_string())
        .unwrap_or_else(|| panic!("falta la línea '{clave}'"))
}

#[test]
fn test_sin_candidatos() {
    let Some(despues) = leer("volcados/obj-despues-cuadro-1000.txt") else {
        return;
    };
    assert_eq!(valor(&despues, "bg0 candidatos sin OBJ"), "0", "H100: BG0 sin OBJ debe quedar en 0");
}

#[test]
fn test_regresion_obj_encendido() {
    let (Some(antes), Some(despues)) = (
        leer("volcados/obj-antes-cuadro-1000.txt"),
        leer("volcados/obj-despues-cuadro-1000.txt"),
    ) else {
        return;
    };
    assert_eq!(
        valor(&antes, "huella salida BG0 + OBJ"),
        valor(&despues, "huella salida BG0 + OBJ"),
        "H102: la salida con OBJ encendido no debe cambiar"
    );
}

#[test]
fn test_bg2_sin_diferencia_con_obj_apagado() {
    let Some(despues) = leer("volcados/obj-despues-cuadro-1000.txt") else {
        return;
    };
    assert_eq!(valor(&despues, "bg2 shim vs fin sin OBJ"), "0", "H101: BG2 sin OBJ debe coincidir con el estado de fin");
}
