//! Test de oro del contrato de la telemetría
//! (propuesta `docs/propuestas/2026-10-22_contrato-de-telemetria.md`).
//!
//! Ata las dos mitades que antes podían desenfocarse en silencio: **Rust escribe la
//! línea de segundo y Python la vuelve a leer**, y los dos tienen que coincidir en
//! cada campo, en su nombre, en su texto y en su sello de versión.
//!
//! El escritor vive en `src/telemetria.rs` (guiado por `telemetria/esquema.txt`) y el
//! lector en `scripts/diagnostico/analizar_telemetria.py`. El puente es `--volcar`, que
//! emite en TSV lo que el lector entendió, con el valor impreso por el formateador del
//! esquema: si los dos formateadores divergen, el test lo ve.
//!
//! Python tiene que estar en el PATH: sin él el contrato no se puede comprobar y el
//! test falla en voz alta, como el arnés de fotogramas dorados.

use blazing_recomp::perf::{FramePerf, FrameTiming, PerfReport, PpuHwWindowStats};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

/// Raíz del repositorio, relativa al manifiesto de Cargo.
fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// El analizador, tal como lo invoca una persona.
fn analizador() -> PathBuf {
    repo_root()
        .join("scripts")
        .join("diagnostico")
        .join("analizar_telemetria.py")
}

/// Directorio temporal propio de cada test, limpio al empezar.
fn dir_temporal(nombre: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "br-telemetria-{}-{}",
        nombre,
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("el directorio temporal debe crearse");
    dir
}

/// Escribe un registro de prueba y devuelve su ruta.
fn escribir_registro(dir: &Path, nombre: &str, contenido: &str) -> PathBuf {
    let ruta = dir.join(nombre);
    fs::write(&ruta, contenido).expect("el registro de prueba debe escribirse");
    ruta
}

/// Ejecuta el analizador desde la raíz del repositorio, como lo haría una persona.
fn correr(ruta: &Path, volcar: bool) -> Output {
    let mut orden = Command::new("python");
    orden.arg(analizador()).arg("--archivo").arg(ruta);
    if volcar {
        orden.arg("--volcar");
    }
    orden
        // El informe lleva acentos: sin esto, Python los escribe en la codificación
        // local de la consola y el test los leería como bytes rotos.
        .env("PYTHONIOENCODING", "utf-8")
        .current_dir(repo_root())
        .output()
        .expect("python debe estar en el PATH: el test de oro del contrato es una herramienta mixta Rust + Python")
}

/// Salida completa (estándar y de error) para poder afirmar sobre el mensaje.
fn salida(completada: &Output) -> String {
    assert!(
        completada.status.success(),
        "el analizador falló: {}{}",
        String::from_utf8_lossy(&completada.stdout),
        String::from_utf8_lossy(&completada.stderr)
    );
    format!(
        "{}{}",
        String::from_utf8_lossy(&completada.stdout),
        String::from_utf8_lossy(&completada.stderr)
    )
}

/// Corre el analizador con `--volcar` y devuelve el TSV.
fn volcar(ruta: &Path) -> String {
    salida(&correr(ruta, true))
}

/// Una línea de segundo tal como la entendió el lector.
#[derive(Debug, Default, PartialEq)]
struct LineaVolcada {
    campos: Vec<(String, String)>,
    sello: Option<String>,
    anomalias: Vec<String>,
}

/// El volcado completo: el resumen de la sesión y una entrada por línea.
#[derive(Debug, Default)]
struct Volcado {
    resumen: BTreeMap<String, String>,
    lineas: Vec<LineaVolcada>,
}

impl Volcado {
    /// Valor de una clave del resumen.
    fn valor(&self, clave: &str) -> &str {
        self.resumen
            .get(clave)
            .unwrap_or_else(|| panic!("el resumen no trae la clave '{clave}': {:?}", self.resumen))
    }
}

/// Lee el TSV de `--volcar`.
fn leer_volcado(texto: &str) -> Volcado {
    let mut volcado = Volcado::default();
    for fila in texto.lines() {
        let celdas: Vec<&str> = fila.split('\t').collect();
        match celdas.first().copied() {
            Some("resumen") => {
                let clave = celdas.get(1).expect("una fila resumen trae clave").to_string();
                let valor = celdas.get(2).unwrap_or(&"").to_string();
                volcado.resumen.insert(clave, valor);
            }
            Some("linea") => volcado.lineas.push(LineaVolcada::default()),
            Some("campo") => {
                let nombre = celdas.get(1).expect("una fila campo trae nombre").to_string();
                let valor = celdas.get(2).unwrap_or(&"").to_string();
                volcado
                    .lineas
                    .last_mut()
                    .expect("una fila campo va dentro de una línea")
                    .campos
                    .push((nombre, valor));
            }
            Some("sello") => {
                let version = celdas.get(1).expect("una fila sello trae versión").to_string();
                volcado
                    .lineas
                    .last_mut()
                    .expect("una fila sello va dentro de una línea")
                    .sello = Some(version);
            }
            Some("anomalia") => {
                let mensaje = celdas.get(1).expect("una fila anomalia trae mensaje").to_string();
                volcado
                    .lineas
                    .last_mut()
                    .expect("una fila anomalia va dentro de una línea")
                    .anomalias
                    .push(mensaje);
            }
            _ => {}
        }
    }
    volcado
}

/// Un `PerfReport` con las cifras del criterio, escrito entero para que se vea qué se mide.
fn informe_de_prueba() -> PerfReport {
    PerfReport {
        ppu_hw: PpuHwWindowStats::default(),
        second_index: 1,
        fps: 60.0,
        ejec_avg_ms: 13.6,
        ejec_max_ms: 21.9,
        ppu_avg_ms: 7.7,
        ppu_bg_avg_ms: 3.3,
        ppu_sprite_avg_ms: 1.1,
        ppu_compose_avg_ms: 3.3,
        apu_avg_ms: 1.5,
        pump_llamadas_avg: 2,
        pump_muestras_avg: 480,
        lecturas_avg: 6581,
        escrituras_avg: 4206,
        color_avg_ms: 1.2,
        color_max_ms: 1.3,
        dibujo_avg_ms: 7.0,
        dibujo_max_ms: 15.5,
        espera_avg_ms: 0.0,
        espera_max_ms: 0.0,
        subida_avg_ms: 0.5,
        codificacion_avg_ms: 1.2,
        present_avg_ms: 5.4,
        present_max_ms: 14.0,
        nativos_avg: 4945,
        interprete_avg: 2,
        vacias: 1,
        ventana: (1280, 720),
    }
}

/// Mueve los campos de sitio dejando el sello al final.
///
/// Si el lector fuera posicional, esta línea dejaría de entenderse: es justo lo que
/// el contrato quiere impedir.
fn barajar(linea: &str) -> String {
    let mut tokens: Vec<&str> = linea.split(' ').collect();
    let sello = tokens.pop().expect("la línea de segundo trae sello");
    assert!(
        sello.starts_with("formato="),
        "el sello cierra la línea: {sello}"
    );
    let mut campos = tokens;
    campos.reverse();
    campos.push(sello);
    campos.join(" ")
}

/// Cuatro segundos con el contrato v1: 21 campos, sin `pump`/`pumpm` y sin sello.
///
/// Es el formato que el motor escribía antes del 2026-10-14, el de los registros que
/// ya están en los dispositivos. `ejec` promedia 15, 15, 30 y 40 ms (media 25.00; peor
/// media 40.00) y el tercer segundo trae un fotograma de 90.0 ms, que es el peor de
/// la sesión pero no el de ninguna media.
fn registro_v1() -> String {
    let segundos = [
        ("1", "40.0", "15.0", "15.0", "15.0"),
        ("2", "50.0", "15.0", "15.0", "15.0"),
        ("3", "40.0", "30.0", "90.0", "20.0"),
        ("4", "50.0", "40.0", "50.0", "25.0"),
    ];
    let mut registro = String::new();
    for (indice, fps, ejec_prom, ejec_peor, dibujo_peor) in segundos {
        registro.push_str(&format!(
            "t={indice}s fps={fps} ejec={ejec_prom}/{ejec_peor} ppu=3.0 ppubg=1.5 ppuspr=0.5 \
ppucomp=1.0 apu=1.0 lecturas=1000 escrituras=500 color=1.0/1.0 dibujo=10.0/{dibujo_peor} \
espera=0.0/0.0 subida=0.5 codificacion=1.0 present=8.0 presentmax=9.0 nativos=100 interprete=0 \
vacias=0 ventana=1280x720\n"
        ));
    }
    registro
}

#[test]
fn test_la_linea_de_segundo_no_pierde_campos() {
    let dir = dir_temporal("no-pierde");
    let linea = informe_de_prueba().to_line();
    let ruta = escribir_registro(&dir, "blazing-perf.txt", &format!("{linea}\n"));

    let volcado = leer_volcado(&volcar(&ruta));
    assert_eq!(volcado.lineas.len(), 1, "una línea de segundo, una entrada");
    let leida = &volcado.lineas[0];

    assert_eq!(leida.sello.as_deref(), Some("2"), "la línea declara su versión");
    assert!(
        leida.anomalias.is_empty(),
        "una línea bien formada no debe tener anomalías: {:?}",
        leida.anomalias
    );

    // Las 23 filas `campo`, en el orden canónico, recomponen la línea entera sin el sello.
    let nombres: Vec<&str> = leida.campos.iter().map(|(nombre, _)| nombre.as_str()).collect();
    assert_eq!(
        nombres,
        vec![
            "t", "fps", "ejec", "ppu", "ppubg", "ppuspr", "ppucomp", "apu", "lecturas",
            "escrituras", "color", "dibujo", "espera", "subida", "codificacion", "present",
            "presentmax", "nativos", "interprete", "vacias", "ventana", "pump", "pumpm",
        ],
        "los 23 campos del esquema vuelven en su orden canónico"
    );

    let recompuesta: Vec<String> = leida
        .campos
        .iter()
        .map(|(nombre, valor)| format!("{nombre}={valor}"))
        .collect();
    let esperada = linea
        .strip_suffix(" formato=2")
        .expect("la línea termina con el sello");
    assert_eq!(
        recompuesta.join(" "),
        esperada,
        "lo que el lector entendió tiene que ser, byte a byte, lo que escribió el motor"
    );

    // Las formas especiales vuelven intactas: sufijo del segundo, par promedio/peor y ancho x alto.
    let valor = |clave: &str| {
        leida
            .campos
            .iter()
            .find(|(nombre, _)| nombre == clave)
            .map(|(_, valor)| valor.as_str())
            .unwrap_or_else(|| panic!("el volcado no trae el campo '{clave}'"))
    };
    assert_eq!(valor("t"), "1s");
    assert_eq!(valor("ejec"), "13.6/21.9");
    assert_eq!(valor("color"), "1.2/1.3");
    assert_eq!(valor("ventana"), "1280x720");
    assert_eq!(valor("nativos"), "4945");

    // El lector va por clave, no por posición: con los campos barajados sale lo mismo.
    let ruta_barajada = escribir_registro(&dir, "barajado.txt", &format!("{}\n", barajar(&linea)));
    let volcado_barajado = leer_volcado(&volcar(&ruta_barajada));
    assert_eq!(
        volcado_barajado.lineas.len(),
        1,
        "la línea barajada se sigue reconociendo como línea de segundo"
    );
    assert_eq!(
        volcado_barajado.lineas[0].campos, leida.campos,
        "reordenar los campos no puede cambiar lo que se lee"
    );
    assert!(volcado_barajado.lineas[0].anomalias.is_empty());
}

#[test]
fn test_un_campo_desconocido_se_reporta() {
    let dir = dir_temporal("desconocido");
    let linea = informe_de_prueba().to_line();
    let con_extra = linea.replace(" formato=2", " zzz=1 formato=2");
    let ruta = escribir_registro(&dir, "con-zzz.txt", &format!("{con_extra}\n"));

    let volcado = leer_volcado(&volcar(&ruta));
    let leida = &volcado.lineas[0];

    assert_eq!(
        leida.campos.len(),
        23,
        "el campo desconocido no aporta fila `campo`: no está en el esquema"
    );
    assert_eq!(
        leida.anomalias,
        vec!["campo desconocido: zzz".to_string()],
        "el campo tiene que salir nombrado, no desaparecer"
    );
    assert_eq!(volcado.valor("anomalias"), "1");
    // La línea sigue contando para las medias: reportar no es descartar.
    assert_eq!(volcado.valor("segundos"), "1");
    assert_eq!(volcado.valor("prom_fps"), "60.0");

    // El informe humano también lo dice, no sólo el volcado.
    let informe = salida(&correr(&ruta, false));
    assert!(
        informe.contains("campo desconocido: zzz"),
        "el informe tiene que nombrar el campo desconocido: {informe}"
    );
    assert!(
        informe.contains("ANOMALÍAS DE FORMATO"),
        "el informe tiene que tener la sección de anomalías: {informe}"
    );
}

#[test]
fn test_los_registros_antiguos_se_siguen_leyendo() {
    let dir = dir_temporal("antiguos");
    let ruta = escribir_registro(&dir, "v1.txt", &registro_v1());

    let volcado = leer_volcado(&volcar(&ruta));

    assert_eq!(volcado.lineas.len(), 4, "los cuatro segundos del registro");
    for leida in &volcado.lineas {
        assert!(
            leida.anomalias.is_empty(),
            "un registro sin sello se lee en modo tolerante: {:?}",
            leida.anomalias
        );
        assert!(leida.sello.is_none(), "el registro v1 no trae sello");
        assert_eq!(
            leida.campos.len(),
            21,
            "el registro v1 trae 21 campos: sin pump ni pumpm"
        );
    }

    assert_eq!(volcado.valor("segundos"), "4");
    assert_eq!(volcado.valor("prom_fps"), "45.0");
    assert_eq!(volcado.valor("prom_ejec"), "25.00");
    assert_eq!(volcado.valor("anomalias"), "0");
}

#[test]
fn test_el_informe_dice_el_peor_fotograma() {
    let dir = dir_temporal("peor-fotograma");
    let ruta = escribir_registro(&dir, "v1.txt", &registro_v1());

    let volcado = leer_volcado(&volcar(&ruta));

    // La media de la ventana sigue siendo la de antes; lo que se añade es el máximo
    // de los máximos, que no es el máximo de las medias.
    assert_eq!(volcado.valor("prom_ejec"), "25.00");
    assert_eq!(volcado.valor("max_ejec"), "40.00", "peor media de segundo");
    assert_eq!(
        volcado.valor("max_ejec_fotograma"),
        "90.00",
        "peor fotograma de la CPU"
    );

    let informe = salida(&correr(&ruta, false));
    assert!(
        informe.contains("peor media de segundo: 40.00 ms"),
        "el informe conserva el número de antes con su etiqueta honesta: {informe}"
    );
    assert!(
        informe.contains("peor fotograma:") && informe.contains("90.00 ms"),
        "el informe añade el peor fotograma: {informe}"
    );
}

#[test]
fn test_el_esquema_no_necesita_dependencias_nuevas() {
    // El contrato se lee del propio crate: si alguien lo cambiara por serde/toml,
    // el esquema dejaría de ser un artefacto legible sin compilar.
    let texto = fs::read_to_string(repo_root().join("telemetria").join("esquema.txt"))
        .expect("el esquema tiene que existir");
    assert!(texto.contains("sello | formato | 2"), "el sello declara la versión 2");
    assert!(
        texto.contains("version | 2 | hereda: 1 | anade: pump, pumpm"),
        "la versión 2 añade pump y pumpm"
    );
}

/// El acumulador también escribe la línea: se comprueba que la usa y no un `format!` suelto.
#[test]
fn test_el_acumulador_escribe_la_linea_del_contrato() {
    let mut acumulador = FramePerf::new();
    acumulador.record_frame(FrameTiming {
        ejec_ms: 20.0,
        ..Default::default()
    });
    let informe = acumulador.close_window((1200, 1920));

    let linea = informe.to_line();
    assert!(linea.ends_with(" formato=2"), "la línea cierra con el sello: {linea}");
    assert!(!linea.starts_with('#'), "una línea de segundo no es una anotación: {linea}");
}
