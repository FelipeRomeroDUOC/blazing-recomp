//! Pruebas de higiene arquitectónica para la eliminación definitiva del PpuShim
//! (propuesta `docs/propuestas/2026-10-23_eliminacion-del-ppu-shim.md`).
//!
//! Validan que el rasterizador antiguo haya sido retirado por completo del crate,
//! de la app y de los ejemplos activos, preservándose documentalmente en `docs/historico/ppu-shim/`.

use std::fs;
use std::path::PathBuf;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

#[test]
fn test_el_shim_ya_no_esta_en_el_crate_ni_en_la_app() {
    let root = repo_root();

    // 1. Ficheros borrados de src/
    assert!(!root.join("src/ppu_shim.rs").exists(), "src/ppu_shim.rs debe haber sido eliminado");
    assert!(!root.join("src/ppu_shim.wgsl").exists(), "src/ppu_shim.wgsl debe haber sido eliminado");
    assert!(!root.join("src/ppu_shim_tests.rs").exists(), "src/ppu_shim_tests.rs debe haber sido eliminado");

    // 2. Declaración en lib.rs
    let lib_rs = fs::read_to_string(root.join("src/lib.rs")).expect("src/lib.rs debe existir");
    assert!(!lib_rs.contains("ppu_shim"), "src/lib.rs no debe declarar ni mencionar ppu_shim");

    // 3. Ausencia en platform.rs
    let platform_rs = fs::read_to_string(root.join("src/platform.rs")).expect("src/platform.rs debe existir");
    assert!(!platform_rs.contains("ppu_shim"), "src/platform.rs no debe contener ppu_shim");
    assert!(!platform_rs.contains("PpuShim"), "src/platform.rs no debe contener PpuShim");
    assert!(!platform_rs.contains("gpu_shim"), "src/platform.rs no debe contener gpu_shim");
}

#[test]
fn test_quedan_18_experimentos_y_el_arnes_solo_usa_la_ppu_nativa() {
    let root = repo_root();
    let examples_dir = root.join("examples");

    let mut archivos_rs: Vec<String> = fs::read_dir(&examples_dir)
        .expect("examples/ debe existir")
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.is_file() && p.extension().is_some_and(|ext| ext == "rs"))
        .map(|p| p.file_name().unwrap().to_string_lossy().into_owned())
        .collect();
    archivos_rs.sort();

    assert_eq!(archivos_rs.len(), 18, "Deben quedar exactamente 18 experimentos en examples/");
    for nombre in &archivos_rs {
        assert!(!nombre.starts_with("shim_"), "Ningún ejemplo puede comenzar con 'shim_': {nombre}");
    }

    // Cargo.toml
    let cargo_toml = fs::read_to_string(root.join("Cargo.toml")).expect("Cargo.toml debe existir");
    let bloques_example = cargo_toml.matches("[[example]]").count();
    assert_eq!(bloques_example, 18, "Cargo.toml debe declarar exactamente 18 [[example]]");

    // examples/comun/gpu.rs
    let gpu_rs = fs::read_to_string(examples_dir.join("comun/gpu.rs")).expect("examples/comun/gpu.rs debe existir");
    assert!(!gpu_rs.contains("PpuShimPipeline"), "gpu.rs no debe importar ni usar PpuShimPipeline");
    assert!(!gpu_rs.contains("Backend::Shim"), "gpu.rs no debe contener Backend::Shim");
    assert!(gpu_rs.contains("PpuNativaPipeline"), "gpu.rs debe usar PpuNativaPipeline");
}

#[test]
fn test_el_archivo_documental_conserva_los_39_ficheros() {
    let root = repo_root();
    let historico_dir = root.join("docs/historico/ppu-shim");

    assert!(historico_dir.is_dir(), "docs/historico/ppu-shim/ debe existir");

    let mut ficheros_rs: Vec<String> = fs::read_dir(&historico_dir)
        .expect("docs/historico/ppu-shim debe poder leerse")
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.is_file() && p.extension().is_some_and(|ext| ext == "rs"))
        .map(|p| p.file_name().unwrap().to_string_lossy().into_owned())
        .collect();
    ficheros_rs.sort();

    assert_eq!(ficheros_rs.len(), 39, "Deben conservarse exactamente 39 ficheros .rs archivados (29 ejemplos + 10 tests)");

    let readme = fs::read_to_string(historico_dir.join("README.md")).expect("README.md histórico debe existir");
    assert!(readme.contains("ADR 42") || readme.contains("ADR-42") || readme.contains("42"), "El README.md histórico debe referenciar el ADR 42");
}
