//! Pruebas de la capa de subagentes para DeepSeek Harness
//! (propuestas `docs/propuestas/2026-10-25_subagentes-dsh-mecanismo.md` y
//! `docs/propuestas/2026-10-25_subagentes-modelo-gemini-lite.md`).
//!
//! Verifican que cada rol conserva su encargo reutilizable, que declara uno de los dos
//! modelos Gemini lite aprobados, que el reparto usa ambos y que la matriz de
//! disparadores y los mensajes del guardián siguen intactos.

use std::collections::BTreeSet;
use std::fs;
use std::path::PathBuf;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

const MODELO_31: &str = "google/gemini-3.1-flash-lite";
const MODELO_35: &str = "google/gemini-3.5-flash-lite";

/// Rol original, archivo de encargo y modelo asignado.
const ROLES: [(&str, &str, &str); 5] = [
    ("Test Runner", "test-runner.md", MODELO_31),
    ("APK Builder", "apk-builder.md", MODELO_35),
    ("Telemetry Analyzer", "telemetry-analyzer.md", MODELO_31),
    ("Label Auditor", "label-auditor.md", MODELO_35),
    ("Code Scout", "code-scout.md", MODELO_35),
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

/// Escenario 1: cada rol declara exactamente uno de los dos modelos aprobados.
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
            contenido.contains("3 a 5 líneas"),
            "El encargo de {archivo} debe pedir un reporte de 3 a 5 líneas"
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
        assert!(
            !contenido.contains("flash_lite"),
            "El encargo de {archivo} no debe seguir anunciando flash_lite como modelo vigente"
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
        reparto.contains(MODELO_31) && reparto.contains(MODELO_35),
        "El reparto debe incluir {MODELO_31} y {MODELO_35}, incluye {reparto:?}"
    );
    assert!(
        por_modelo.iter().all(|(_, cuenta)| *cuenta < ROLES.len()),
        "Ningún modelo debe quedarse con los cinco roles: {por_modelo:?}"
    );
}

/// Escenario 3: el índice lista exactamente los roles vivos y declara el rol podado.
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
        !dir.join("doc-manager.md").exists(),
        "Doc Manager no debe tener archivo de encargo: está podado"
    );
}

/// Escenario 4 (regresión): los mensajes del guardián conservan Antigravity y añaden DSH.
#[test]
fn test_guardian_conserva_antigravity_y_anade_dsh() {
    let ruta = repo_root()
        .join(".agents")
        .join("hooks")
        .join("pre_tool_guard.py");
    let contenido = fs::read_to_string(&ruta).expect("pre_tool_guard.py debe ser legible");

    for (rol, archivo, _) in ROLES.iter().take(4) {
        assert!(
            contenido.contains(&format!("invoke_subagent(TypeName='self', Role='{rol}'")),
            "El mensaje original de {rol} debe conservarse"
        );
        assert!(
            contenido.contains(&format!(".agents/subagents/{archivo}")),
            "El mensaje de {rol} debe apuntar a .agents/subagents/{archivo}"
        );
    }
    assert!(
        contenido.contains("subagent(description="),
        "Los mensajes deben nombrar la herramienta subagent de DSH"
    );

    let inicio = fs::read_to_string(
        repo_root()
            .join(".agents")
            .join("hooks")
            .join("session_init.py"),
    )
    .expect("session_init.py debe ser legible");
    assert!(
        inicio.contains("invoke_subagent(TypeName='self', Role="),
        "El mensaje de inicio debe conservar la invocación de Antigravity"
    );
    assert!(
        inicio.contains(".agents/subagents/"),
        "El mensaje de inicio debe remitir a los encargos de DSH"
    );
}

/// Escenario 5 (regresión): la matriz conserva disparadores y su columna de modelo apunta a Gemini.
#[test]
fn test_tabla_conserva_disparadores() {
    let contenido =
        fs::read_to_string(repo_root().join("AGENTS.md")).expect("AGENTS.md debe ser legible");

    for (rol, _, modelo) in ROLES.iter().take(4) {
        assert!(
            contenido.contains(&format!("| `self` | `{rol}` | `{modelo}` |")),
            "La fila de {rol} debe conservar self y declarar {modelo}"
        );
    }

    for disparador in [
        "cargo test",
        "compilar_recomp_aot.ps1",
        "analizar_telemetria.py",
        "auditar_labels.py",
    ] {
        assert!(
            contenido.contains(disparador),
            "La matriz debe conservar el disparador {disparador}"
        );
    }

    assert!(
        contenido.contains("### 4.1 Invocación en DeepSeek Harness (DSH)"),
        "AGENTS.md debe documentar la invocación en DSH"
    );
}
