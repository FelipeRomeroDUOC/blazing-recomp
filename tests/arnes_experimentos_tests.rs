//! Pruebas del arnés compartido de los experimentos de `examples/`
//! (propuesta `docs/propuestas/2026-10-22_arnes-de-experimentos-comun.md`).
//!
//! Los 47 experimentos comparten `examples/comun/`, que vive **fuera** del crate que se
//! empaqueta en el APK. Eso lo deja sin `cargo test --lib`, así que se incluye aquí con
//! `#[path]`: es el mismo fichero que compilan los ejemplos, probado de verdad.
//!
//! Se incluye `entrada.rs` suelto y no `mod.rs` entero: `gpu.rs` arrastra wgpu y los
//! pipelines del motor, que este test no necesita para probar el parser.

//! Los escenarios de estructura (inventario y ausencia de copias) leen `examples/` y
//! `Cargo.toml` con `std::fs`; no compilan ni ejecutan ningún experimento, así que no
//! necesitan ROM, BIOS ni GPU.

use std::collections::BTreeSet;
use std::fs;
use std::path::PathBuf;

// `leer_tramos` sólo lo usa `ppu_compare`; aquí se incluye el fichero entero.
#[allow(dead_code)]
#[path = "../examples/comun/entrada.rs"]
mod entrada;

/// Raíz del repositorio, relativa al manifiesto de Cargo.
fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn dir_examples() -> PathBuf {
    repo_root().join("examples")
}

/// Los 47 ficheros de experimento, ordenados por nombre.
fn experimentos() -> Vec<PathBuf> {
    let mut rutas: Vec<PathBuf> = fs::read_dir(dir_examples())
        .expect("examples/ tiene que existir")
        .filter_map(|entrada| entrada.ok())
        .map(|entrada| entrada.path())
        .filter(|ruta| ruta.is_file() && ruta.extension().is_some_and(|ext| ext == "rs"))
        .collect();
    rutas.sort();
    rutas
}

fn leer(ruta: &PathBuf) -> String {
    fs::read_to_string(ruta).unwrap_or_else(|e| panic!("no se pudo leer {}: {e}", ruta.display()))
}

#[test]
fn test_los_experimentos_usan_el_modulo_comun() {
    let rutas = experimentos();
    assert_eq!(rutas.len(), 47, "tienen que quedar los 47 experimentos");

    // Lo que deja de poder estar declarado dentro de un experimento: si reaparece una
    // copia, el arreglo del arnés vuelve a tener que hacerse 47 veces.
    let declaraciones_prohibidas = [
        "fn leer_secuencia",
        "const BOTONES",
        "fn canal_8bit",
        "fn cerca(",
        "struct Gpu",
        "fn io16",
        "fn io_con_variante",
        "fn software_a_rgb",
    ];

    for ruta in &rutas {
        let nombre = ruta.file_name().unwrap().to_string_lossy().into_owned();
        let texto = leer(ruta);

        assert!(
            texto.contains("mod comun;"),
            "{nombre}: tiene que declarar el módulo común"
        );

        for prohibida in declaraciones_prohibidas {
            assert!(
                !texto.contains(prohibida),
                "{nombre}: vuelve a declarar '{prohibida}', que ya vive en examples/comun/"
            );
        }
    }

    // `dispcnt_de` tiene una única definición legítima fuera del módulo: la de
    // `ppu_capas_linea.rs`, que resuelve 11 variantes desde una tabla y no es una copia.
    let con_dispcnt: Vec<String> = rutas
        .iter()
        .filter(|ruta| leer(ruta).contains("fn dispcnt_de"))
        .map(|ruta| ruta.file_name().unwrap().to_string_lossy().into_owned())
        .collect();
    assert_eq!(
        con_dispcnt,
        vec!["ppu_capas_linea.rs".to_string()],
        "sólo ppu_capas_linea conserva su propia variante de dispcnt_de"
    );

    // Y el módulo tiene que declarar de verdad lo que los experimentos dejaron de declarar.
    let esperado = [
        ("mod.rs", "pub mod entrada;"),
        ("mod.rs", "pub mod gpu;"),
        ("entrada.rs", "pub fn leer_secuencia"),
        ("entrada.rs", "pub fn leer_tramos"),
        ("color.rs", "pub fn canal_8bit"),
        ("color.rs", "pub fn cerca("),
        ("color.rs", "pub fn software_a_rgb"),
        ("registro.rs", "pub fn io16"),
        ("registro.rs", "pub fn dispcnt_de"),
        ("registro.rs", "pub fn io_con_variante"),
        ("gpu.rs", "pub struct Gpu"),
        ("gpu.rs", "pub fn renderiza"),
    ];
    for (fichero, fragmento) in esperado {
        let ruta = dir_examples().join("comun").join(fichero);
        assert!(ruta.is_file(), "falta examples/comun/{fichero}");
        assert!(
            leer(&ruta).contains(fragmento),
            "examples/comun/{fichero} no declara '{fragmento}'"
        );
    }
}

#[test]
fn test_el_inventario_coincide_con_cargo_toml() {
    let en_disco: BTreeSet<String> = experimentos()
        .iter()
        .map(|ruta| ruta.file_stem().unwrap().to_string_lossy().into_owned())
        .collect();

    // Dos de los 47 compilan sin la feature: `ppu_barrido_nativa` y `ppu_modo_cuadro`
    // no usan la captura por línea de `perf-video` (medido: `cargo build --example
    // ppu_barrido_nativa --example ppu_modo_cuadro` termina con Finished). Los demás sí.
    const SIN_FEATURE: [&str; 2] = ["ppu_barrido_nativa", "ppu_modo_cuadro"];

    let cargo = leer(&repo_root().join("Cargo.toml"));
    let mut declarados = BTreeSet::new();
    for bloque in cargo.split("[[example]]").skip(1) {
        // El bloque acaba en el siguiente encabezado de sección.
        let fin = bloque.find("\n[").unwrap_or(bloque.len());
        let cuerpo = &bloque[..fin];
        let nombre = cuerpo
            .lines()
            .find_map(|linea| linea.trim().strip_prefix("name = "))
            .map(|valor| valor.trim().trim_matches('"').to_string())
            .unwrap_or_else(|| panic!("un bloque [[example]] no declara name: {cuerpo}"));
        let exige = cuerpo.contains("required-features = [\"proto\"]");
        let exento = SIN_FEATURE.contains(&nombre.as_str());
        assert!(
            exige != exento,
            "{nombre}: o sobra required-features = [\"proto\"] o falta; los dos casos son incompatibles"
        );
        declarados.insert(nombre);
    }

    assert_eq!(declarados.len(), 47, "Cargo.toml declara los 47 experimentos");
    assert_eq!(
        declarados, en_disco,
        "los nombres de Cargo.toml y los ficheros de examples/ tienen que coincidir: \
         las propuestas antiguas los citan por nombre"
    );

    let con_feature = declarados
        .iter()
        .filter(|nombre| !SIN_FEATURE.contains(&nombre.as_str()))
        .count();
    assert_eq!(con_feature, 45, "45 experimentos exigen la feature proto");
}

#[test]
fn test_la_secuencia_se_lee_igual_que_antes() {
    // El mismo texto que tragaba cualquiera de las 45 copias: comentario, tramo con
    // `ninguno`, línea en blanco y dos tramos con varios botones.
    let tramos = entrada::leer_secuencia("# comentario\n120 ninguno\n\n60 A+B\n30 R\n")
        .expect("la secuencia es válida");

    // A = bit 0, B = bit 1, R = bit 8 (orden de KEYINPUT, activo en bajo al aplicarlo).
    assert_eq!(tramos, vec![(120, 0x000), (60, 0x003), (30, 0x100)]);

    // Y un caso por botón: con sólo `A+B` el intercambio de A y B pasaría inadvertido,
    // porque los dos juntos dan el mismo par de bits.
    let por_boton = [
        ("A", 0x001u16),
        ("B", 0x002),
        ("Select", 0x004),
        ("Start", 0x008),
        ("Right", 0x010),
        ("Left", 0x020),
        ("Up", 0x040),
        ("Down", 0x080),
        ("R", 0x100),
        ("L", 0x200),
    ];
    for (boton, bits) in por_boton {
        let tramos = entrada::leer_secuencia(&format!("1 {boton}\n"))
            .unwrap_or_else(|e| panic!("'{boton}' tiene que ser un botón válido: {e}"));
        assert_eq!(tramos, vec![(1, bits)], "'{boton}' ocupa otro bit de KEYINPUT");
    }
}

#[test]
fn test_la_secuencia_rechaza_un_boton_desconocido() {
    let error = entrada::leer_secuencia("10 A+Foo\n")
        .expect_err("un botón que no existe tiene que ser un error");
    assert!(
        error.contains("línea 1") && error.contains("Foo"),
        "el mensaje tiene que nombrar la línea y el botón; fue: {error}"
    );
}
