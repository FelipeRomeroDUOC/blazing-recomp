//! Pruebas de la capa de subagentes para DeepSeek Harness
//! (propuestas `docs/propuestas/2026-10-25_subagentes-dsh-mecanismo.md`,
//! `docs/propuestas/2026-10-25_subagentes-modelo-gemini-lite.md` y
//! `docs/propuestas/2026-11-06_reglas-deterministas-agentes-skills.md`).
//!
//! Verifican que cada rol vivo conserva su encargo reutilizable, que declara su modelo
//! Gemini correspondiente, que el reparto usa ambos modelos y que la matriz de
//! disparadores, los scripts deterministas y los mensajes del guardián siguen sincronizados.

use std::collections::BTreeSet;
use std::fs;
use std::path::PathBuf;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

const MODELO_FLASH: &str = "google/gemini-3.8-flash";
const MODELO_LITE: &str = "google/gemini-3.5-flash-lite";

/// Rol vivo, archivo de encargo y modelo asignado.
const ROLES: [(&str, &str, &str); 4] = [
    ("Investigador", "investigador.md", MODELO_FLASH),
    ("Proposal Reviewer", "proposal-reviewer.md", MODELO_FLASH),
    ("Constructor", "constructor.md", MODELO_FLASH),
    ("Code Scout", "code-scout.md", MODELO_LITE),
];

/// Identificadores `google/...` distintos que aparecen en un texto.
fn modelos_google(texto: &str) -> BTreeSet<String> {
    let mut encontrados = BTreeSet::new();
    for (inicio, _) in texto.match_indices("google/") {
        let resto = &texto[inicio..];
        let fin = resto
            .find(|c: char| !(c.is_alphanumeric() || c == '/' || c == '-' || c == '.'))
            .unwrap_or(resto.len());
        encontrados.insert(resto[..fin].to_string());
    }
    encontrados
}

/// Escenario 1: cada rol declara su modelo aprobado correspondiente.
#[test]
fn test_cada_rol_declara_un_modelo_aprobado() {
    let dir = repo_root().join(".agents").join("subagents");
    for (rol, archivo, modelo) in ROLES {
        let ruta = dir.join(archivo);
        let contenido = fs::read_to_string(&ruta).unwrap_or_else(|e| {
            panic!(
                "Debe existir el encargo de {rol} en {}: {e}",
                ruta.display()
            )
        });

        assert!(
            contenido.contains(rol),
            "El encargo de {archivo} debe nombrar su rol ({rol})"
        );
        assert!(
            contenido.contains("3 a 5 líneas") || contenido.contains("Reporte Ejecutivo"),
            "El encargo de {archivo} debe pedir un reporte ejecutivo o de 3 a 5 líneas"
        );

        let modelos = modelos_google(&contenido);
        assert_eq!(
            modelos.len(),
            1,
            "El encargo de {archivo} debe declarar un solo modelo google, declara {modelos:?}"
        );
        assert!(
            modelos.contains(modelo),
            "El encargo de {archivo} debe declarar {modelo}, declara {modelos:?}"
        );
    }
}

/// Escenario 2: el reparto usa los dos modelos, no uno solo.
#[test]
fn test_reparto_usa_los_dos_modelos() {
    let dir = repo_root().join(".agents").join("subagents");
    let mut reparto: BTreeSet<String> = BTreeSet::new();
    let mut por_modelo: Vec<(String, usize)> = Vec::new();

    for (_, archivo, _) in ROLES {
        let contenido = fs::read_to_string(dir.join(archivo))
            .unwrap_or_else(|e| panic!("Debe poder leerse {archivo}: {e}"));
        for modelo in modelos_google(&contenido) {
            reparto.insert(modelo);
        }
    }

    for modelo in &reparto {
        let cuenta = ROLES
            .iter()
            .filter(|(_, _, asignado)| asignado == modelo)
            .count();
        por_modelo.push((modelo.clone(), cuenta));
    }

    assert_eq!(
        reparto.len(),
        2,
        "Deben usarse los dos modelos aprobados, se usan {reparto:?}"
    );
    assert!(
        reparto.contains(MODELO_FLASH) && reparto.contains(MODELO_LITE),
        "El reparto debe incluir {MODELO_FLASH} y {MODELO_LITE}, incluye {reparto:?}"
    );
    assert!(
        por_modelo.iter().all(|(_, cuenta)| *cuenta < ROLES.len()),
        "Ningún modelo debe quedarse con los cuatro roles: {por_modelo:?}"
    );
}

/// Escenario 3: el índice lista exactamente los roles vivos y declara los roles podados.
#[test]
fn test_indice_cubre_los_roles_vivos() {
    let dir = repo_root().join(".agents").join("subagents");
    let indice = fs::read_to_string(dir.join("README.md"))
        .expect("Debe existir .agents/subagents/README.md como índice");

    for (_, archivo, _) in ROLES {
        assert!(
            indice.contains(&format!("]({archivo})")),
            "El índice debe enlazar {archivo}"
        );
    }

    let mut archivos_de_rol: Vec<String> = fs::read_dir(&dir)
        .expect("Debe poder listarse .agents/subagents/")
        .filter_map(|entrada| entrada.ok())
        .map(|entrada| entrada.file_name().to_string_lossy().to_string())
        .filter(|nombre| nombre.ends_with(".md") && nombre != "README.md")
        .collect();
    archivos_de_rol.sort();

    let mut esperados: Vec<String> = ROLES.iter().map(|(_, a, _)| a.to_string()).collect();
    esperados.sort();

    assert_eq!(
        archivos_de_rol, esperados,
        "El índice y los archivos de rol deben coincidir uno a uno"
    );
    assert!(
        indice.contains("Doc Manager") && indice.to_lowercase().contains("podado"),
        "El índice debe declarar que Doc Manager está podado"
    );
    assert!(
        indice.contains("Test Runner") && indice.to_lowercase().contains("podado"),
        "El índice debe declarar que Test Runner está podado"
    );
    assert!(
        indice.contains("APK Builder") && indice.to_lowercase().contains("podado"),
        "El índice debe declarar que APK Builder está podado"
    );
    assert!(
        !dir.join("doc-manager.md").exists(),
        "Doc Manager no debe tener archivo de encargo: está podado"
    );
    assert!(
        !dir.join("test-runner.md").exists(),
        "Test Runner no debe tener archivo de encargo: reemplazado por script"
    );
    assert!(
        !dir.join("apk-builder.md").exists(),
        "APK Builder no debe tener archivo de encargo: reemplazado por script"
    );
}

/// Escenario 4 (regresión): el guardián redirige comandos deterministas y conserva delegaciones.
#[test]
fn test_guardian_conserva_antigravity_y_anade_dsh() {
    let ruta = repo_root()
        .join(".agents")
        .join("hooks")
        .join("pre_tool_guard.py");
    let contenido = fs::read_to_string(&ruta).expect("pre_tool_guard.py debe ser legible");

    // Redirecciones deterministas
    assert!(
        contenido.contains("scripts/run_tests_summary.py"),
        "pre_tool_guard.py debe redirigir cargo test a run_tests_summary.py"
    );
    assert!(
        contenido.contains("scripts/build_apk_summary.py"),
        "pre_tool_guard.py debe redirigir la compilación/empaquetado a build_apk_summary.py"
    );

    // Delegaciones a Constructor, Investigador y Code Scout (Antigravity y DSH)
    let roles_delegados = ["Constructor", "Investigador", "Code Scout"];
    for rol in roles_delegados {
        assert!(
            contenido.contains(&format!("invoke_subagent(TypeName='self', Role='{rol}'")),
            "pre_tool_guard.py debe delegar a {rol} vía invoke_subagent"
        );
        assert!(
            contenido.contains(&format!("subagent(description='{rol}'")),
            "pre_tool_guard.py debe delegar a {rol} vía subagent DSH"
        );
    }

    let inicio = fs::read_to_string(
        repo_root()
            .join(".agents")
            .join("hooks")
            .join("session_init.py"),
    )
    .expect("session_init.py debe ser legible");
    assert!(
        inicio.contains("invoke_subagent"),
        "El mensaje de inicio debe mencionar invoke_subagent"
    );
    assert!(
        inicio.contains(".agents"),
        "El mensaje de inicio debe referenciar el directorio .agents"
    );
}

/// Escenario 5 (regresión): la tabla en AGENTS.md documenta los roles vivos, scripts y DSH.
#[test]
fn test_tabla_conserva_disparadores() {
    let contenido =
        fs::read_to_string(repo_root().join("AGENTS.md")).expect("AGENTS.md debe ser legible");

    let roles_esperados = [
        ("Investigador", "google/gemini-3.8-flash"),
        ("Proposal Reviewer", "google/gemini-3.8-flash"),
        ("Constructor", "google/gemini-3.8-flash"),
        ("Code Scout", "google/gemini-3.5-flash-lite"),
        ("Deep Debugger", "google/gemini-4-argon"),
    ];

    for (rol, modelo) in roles_esperados {
        assert!(
            contenido.contains(&format!("`{rol}`")) && contenido.contains(modelo),
            "AGENTS.md debe documentar el rol {rol} con el modelo {modelo}"
        );
    }

    assert!(
        contenido.contains("run_tests_summary.py"),
        "AGENTS.md debe mencionar run_tests_summary.py"
    );
    assert!(
        contenido.contains("build_apk_summary.py"),
        "AGENTS.md debe mencionar build_apk_summary.py"
    );

    assert!(
        contenido.contains("### 4.1 Invocación en DeepSeek Harness (DSH)"),
        "AGENTS.md debe documentar la invocación en DSH"
    );
}
