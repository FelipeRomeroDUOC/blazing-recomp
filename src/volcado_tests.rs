use super::*;
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

fn crear_directorio_temporal(prefijo: &str) -> PathBuf {
    let base = std::env::current_dir()
        .map(|d| d.join("target").join("tmp"))
        .unwrap_or_else(|_| std::env::temp_dir());
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let dir = base.join(format!("{}_{}", prefijo, stamp));
    let _ = fs::create_dir_all(&dir);
    dir
}

#[test]
fn test_deteccion_bandera_volcado_activa() {
    let temp_dir = crear_directorio_temporal("blazing_test_bandera");
    let search_dirs = [temp_dir.clone()];

    // Inicialmente no debe detectar ninguna bandera
    assert!(
        !volcado_activado(&search_dirs),
        "No debe detectar volcado si no existe el archivo bandera"
    );
    assert!(!volcado_activado_por_bandera(&search_dirs));

    // Crear el archivo dump-frames.flag
    let bandera = temp_dir.join(NOMBRE_BANDERA_VOLCADO);
    fs::write(&bandera, b"").expect("crear archivo bandera");

    // Ahora debe reportar que el volcado está activo
    assert!(
        volcado_activado(&search_dirs),
        "Debe detectar el volcado activo tras crear la bandera"
    );
    assert!(volcado_activado_por_bandera(&search_dirs));

    let _ = fs::remove_dir_all(&temp_dir);
}

#[test]
fn test_generacion_png_compuesto_y_diagnostico() {
    let sw_fin = vec![50u8; ANCHO_GBA * ALTO_GBA * 3];
    let mut sw_linea = vec![100u8; ANCHO_GBA * ALTO_GBA * 3];
    let mut nat = vec![100u8; ANCHO_GBA * ALTO_GBA * 3];

    // Introducir una diferencia notable en el píxel (50, 60)
    let p_dif = 60 * ANCHO_GBA + 50;
    sw_linea[p_dif * 3] = 30;
    nat[p_dif * 3] = 220; // Diferencia de 190 niveles (> tolerancia 3)

    let compuesto = generar_lienzo_compuesto(&sw_fin, &sw_linea, &nat);
    assert_eq!(
        compuesto.len(),
        ANCHO_COMPUESTO * ALTO_GBA * 3,
        "El lienzo compuesto debe tener exactamente 960x160x3 bytes"
    );

    // Verificar panel 3 (diferencias en magenta):
    // El píxel (50, 60) debe ser magenta [255, 0, 255]
    let o_dif = (60 * ANCHO_COMPUESTO + (3 * ANCHO_GBA + 50)) * 3;
    assert_eq!(
        &compuesto[o_dif..o_dif + 3],
        &[255, 0, 255],
        "El píxel divergente debe estar resaltado en magenta en el panel 3"
    );

    // El píxel (0, 0) no divergente debe coincidir con el color original
    let o_id = (0 * ANCHO_COMPUESTO + (3 * ANCHO_GBA + 0)) * 3;
    assert_eq!(
        &compuesto[o_id..o_id + 3],
        &[100, 100, 100],
        "Un píxel sin diferencias debe conservar el color base en el panel 3"
    );

    // Codificar a PNG puro
    let png = png_rgb(&compuesto, ANCHO_COMPUESTO, ALTO_GBA);
    assert!(
        png.len() > 100,
        "El PNG codificado debe tener un tamaño razonable"
    );
    assert_eq!(
        &png[0..8],
        b"\x89PNG\r\n\x1a\n",
        "El archivo generado debe tener la firma estándar de PNG"
    );

    // Comprobar dimensiones en el bloque IHDR
    let ancho_png = u32::from_be_bytes([png[16], png[17], png[18], png[19]]);
    let alto_png = u32::from_be_bytes([png[20], png[21], png[22], png[23]]);
    assert_eq!(ancho_png, 960, "El ancho del PNG debe ser 960");
    assert_eq!(alto_png, 160, "El alto del PNG debe ser 160");

    // Generar reporte de diagnóstico causal
    let ppu_state = crate::pipeline::PpuFrameState::new();
    let informe = generar_informe_diagnostico(77, &sw_linea, &nat, &ppu_state);

    assert!(
        informe.contains("CUADRO 77"),
        "El informe debe incluir el número de fotograma"
    );
    assert!(
        informe.contains("Píxeles distintos (tolerancia > 3): 1 / 38400"),
        "El informe debe detectar con precisión el único píxel distinto"
    );
    assert!(
        informe.contains("DISPCNT:"),
        "El informe debe volcar los registros I/O principales"
    );
}

#[test]
fn test_hilo_volcado_grabacion_continua_sin_tope() {
    let temp_dir = crear_directorio_temporal("blazing_test_sin_tope");
    let mut gestor = GestorVolcado::with_dir(temp_dir.clone());

    let sw_fin = vec![10u8; ANCHO_GBA * ALTO_GBA * 3];
    let sw_linea = vec![20u8; ANCHO_GBA * ALTO_GBA * 3];
    let nat = vec![20u8; ANCHO_GBA * ALTO_GBA * 3];

    // Enviar 50 fotogramas continuos
    for i in 0..50 {
        let tarea = TareaVolcado {
            cuadro: i,
            sw_fin: sw_fin.clone(),
            sw_linea: sw_linea.clone(),
            nat: nat.clone(),
            ppu_state: None,
        };
        assert!(
            gestor.enviar_cuadro(tarea),
            "Debe poder enviar cada cuadro mientras el gestor esté activo"
        );
    }

    // Esperar a que el hilo termine de procesar las tareas pendientes
    gestor.detener_y_esperar();

    assert_eq!(
        gestor.cuadros_grabados(),
        50,
        "El gestor debe haber grabado los 50 fotogramas sin tope artificial"
    );

    let entradas = fs::read_dir(&temp_dir).expect("leer directorio de volcado");
    let mut pngs = 0;
    let mut txts = 0;
    for e in entradas.flatten() {
        let path = e.path();
        if path.extension().map_or(false, |ext| ext == "png") {
            pngs += 1;
        } else if path.extension().map_or(false, |ext| ext == "txt") {
            txts += 1;
        }
    }

    assert_eq!(
        pngs, 50,
        "Debe haber exactamente 50 archivos PNG en el disco"
    );
    assert_eq!(
        txts, 50,
        "Debe haber exactamente 50 archivos TXT en el disco"
    );

    assert!(
        temp_dir.join("cuadro-0-compuesto.png").exists(),
        "El primer cuadro debe existir"
    );
    assert!(
        temp_dir.join("cuadro-49-compuesto.png").exists(),
        "El cuadro 49 (número 50) debe existir"
    );

    let _ = fs::remove_dir_all(&temp_dir);
}
