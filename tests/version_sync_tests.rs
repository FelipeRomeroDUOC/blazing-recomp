//! Pruebas de integración para la sincronización unánime de versiones y fórmula Android.
//!
//! Garantizan que `Cargo.toml`, `android/AndroidManifest.xml`, `pack.toml` y `docs/BACKLOG.md`
//! compartan exactamente la misma versión SemVer y que `versionCode` cumpla estrictamente
//! con la fórmula canónica: `versionCode = MAJOR * 10000 + MINOR * 100 + PATCH`.
//!
//! También validan el funcionamiento y compuerta del script canónico `scripts/bump_version.py`.

use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Stdio};

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn python_cmd() -> &'static str {
    if cfg!(windows) {
        "python"
    } else {
        "python3"
    }
}

/// Extrae el campo `version = "..."` de un archivo TOML sencillo sin dependencias pesadas.
fn extraer_version_toml(contenido: &str) -> Option<String> {
    for linea in contenido.lines() {
        let trimmed = linea.trim();
        if trimmed.starts_with("version") {
            if let Some((_, val)) = trimmed.split_once('=') {
                let val = val.trim().trim_matches('"');
                return Some(val.to_string());
            }
        }
    }
    None
}

/// Escenario 1 y 4: Validación matemática de versionCode y sincronización de manifiestos
#[test]
fn test_version_manifests_are_synchronized() {
    let root = repo_root();

    // 1. Cargo.toml
    let cargo_path = root.join("Cargo.toml");
    let cargo_str = fs::read_to_string(&cargo_path).expect("Cargo.toml debe ser legible");
    let cargo_version = extraer_version_toml(&cargo_str).expect("Cargo.toml debe declarar version");

    // 2. pack.toml
    let pack_path = root.join("pack.toml");
    let pack_str = fs::read_to_string(&pack_path).expect("pack.toml debe ser legible");
    let pack_version = extraer_version_toml(&pack_str).expect("pack.toml debe declarar version");

    // 3. android/AndroidManifest.xml
    let manifest_path = root.join("android").join("AndroidManifest.xml");
    let manifest_str =
        fs::read_to_string(&manifest_path).expect("AndroidManifest.xml debe ser legible");

    let version_name = manifest_str
        .lines()
        .find_map(|l| {
            let t = l.trim();
            if let Some(resto) = t.strip_prefix("android:versionName=\"") {
                resto
                    .strip_suffix("\">")
                    .or_else(|| resto.strip_suffix('"'))
                    .map(str::to_string)
            } else {
                None
            }
        })
        .expect("AndroidManifest.xml debe declarar android:versionName");

    let version_code_str = manifest_str
        .lines()
        .find_map(|l| {
            let t = l.trim();
            if let Some(resto) = t.strip_prefix("android:versionCode=\"") {
                resto
                    .strip_suffix("\">")
                    .or_else(|| resto.strip_suffix('"'))
                    .map(str::to_string)
            } else {
                None
            }
        })
        .expect("AndroidManifest.xml debe declarar android:versionCode");

    let version_code: u32 = version_code_str
        .parse()
        .expect("versionCode debe ser un entero u32 válido");

    // Verificar concordancia unánime de cadenas SemVer
    assert_eq!(
        cargo_version, pack_version,
        "Cargo.toml ({cargo_version}) y pack.toml ({pack_version}) deben coincidir"
    );
    assert_eq!(
        cargo_version, version_name,
        "Cargo.toml ({cargo_version}) y AndroidManifest.xml versionName ({version_name}) deben coincidir"
    );

    // Verificar fórmula Android: versionCode = MAJOR * 10000 + MINOR * 100 + PATCH
    let partes: Vec<u32> = cargo_version
        .split('.')
        .map(|p| p.parse().expect("Cada componente SemVer debe ser un u32"))
        .collect();
    assert_eq!(
        partes.len(),
        3,
        "La versión debe tener formato exacto MAJOR.MINOR.PATCH"
    );

    let esperado_code = partes[0] * 10000 + partes[1] * 100 + partes[2];
    assert_eq!(
        version_code, esperado_code,
        "versionCode ({version_code}) debe cumplir con MAJOR*10000 + MINOR*100 + PATCH ({esperado_code})"
    );

    // 4. docs/BACKLOG.md
    let backlog_path = root.join("docs").join("BACKLOG.md");
    if backlog_path.exists() {
        let backlog_str =
            fs::read_to_string(&backlog_path).expect("docs/BACKLOG.md debe ser legible");
        let patron_version = format!("`{cargo_version}`");
        let patron_code = format!("`versionCode` {version_code}");
        assert!(
            backlog_str.contains(&patron_version),
            "docs/BACKLOG.md debe mencionar la versión actual {patron_version}"
        );
        assert!(
            backlog_str.contains(&patron_code),
            "docs/BACKLOG.md debe mencionar el versionCode actual {patron_code}"
        );
    }
}

/// Escenario 1 (CLI): `scripts/bump_version.py --check` verifica sincronización sin errores
#[test]
fn test_bump_version_script_check() {
    let root = repo_root();
    let script_path = root.join("scripts").join("bump_version.py");

    let output = Command::new(python_cmd())
        .arg(&script_path)
        .arg("--check")
        .current_dir(&root)
        .output()
        .expect("Debe poder ejecutarse python scripts/bump_version.py --check");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(
        output.status.success(),
        "bump_version.py --check falló.\nStdout: {stdout}\nStderr: {stderr}"
    );
    assert!(
        stdout.contains("sincronizados") || stdout.contains("OK") || stdout.contains("0.0.3"),
        "bump_version.py --check debe confirmar la sincronización unánime"
    );
}

/// Escenario 2: `scripts/bump_version.py --dry-run --patch` simula el incremento sin modificar archivos
#[test]
fn test_bump_version_dry_run_leaves_files_untouched() {
    let root = repo_root();
    let script_path = root.join("scripts").join("bump_version.py");
    let cargo_path = root.join("Cargo.toml");

    let antes = fs::read_to_string(&cargo_path).expect("Cargo.toml debe ser legible");

    let output = Command::new(python_cmd())
        .arg(&script_path)
        .arg("--dry-run")
        .arg("--patch")
        .current_dir(&root)
        .output()
        .expect("Debe poder ejecutarse python scripts/bump_version.py --dry-run --patch");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(
        output.status.success(),
        "bump_version.py --dry-run falló.\nStdout: {stdout}\nStderr: {stderr}"
    );
    assert!(
        stdout.contains("0.0.4"),
        "El dry-run de patch sobre 0.0.3 debe anunciar la versión 0.0.4"
    );

    let despues = fs::read_to_string(&cargo_path).expect("Cargo.toml debe ser legible");
    assert_eq!(
        antes, despues,
        "El modo --dry-run no debe alterar Cargo.toml en disco"
    );
}

/// Escenario 3: La compuerta interactiva aborta si el usuario no responde afirmativamente
#[test]
fn test_bump_version_aborts_without_confirmation() {
    let root = repo_root();
    let script_path = root.join("scripts").join("bump_version.py");
    let cargo_path = root.join("Cargo.toml");

    let antes = fs::read_to_string(&cargo_path).expect("Cargo.toml debe ser legible");

    let mut child = Command::new(python_cmd())
        .arg(&script_path)
        .arg("--patch")
        .current_dir(&root)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Debe poder iniciar bump_version.py");

    // Simulamos respuesta 'n' (rechazo)
    if let Some(mut stdin) = child.stdin.take() {
        let _ = stdin.write_all(b"n\n");
    }

    let output = child
        .wait_with_output()
        .expect("El proceso hijo debe finalizar");
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(
        !output.status.success() || stdout.contains("cancelad") || stdout.contains("abortad"),
        "El script debe cancelar o abortar si la persona no confirma explícitamente"
    );

    let despues = fs::read_to_string(&cargo_path).expect("Cargo.toml debe ser legible");
    assert_eq!(
        antes, despues,
        "Si la persona no confirma, los archivos en disco deben permanecer idénticos"
    );
}
