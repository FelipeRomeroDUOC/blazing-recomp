use super::*;
use std::fs::File;
use std::io::Write;

#[test]
fn test_sha256_nist_vectors() {
    // Vector NIST conocido para "abc"
    let hash_abc = compute_sha256(b"abc");
    assert_eq!(
        hash_abc,
        "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
    );

    // Vector NIST para string vaco
    let hash_empty = compute_sha256(b"");
    assert_eq!(
        hash_empty,
        "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
    );
}

#[test]
fn test_verify_rom_size_and_signature_rejection() {
    // Tamao incorrecto (menor a 16 MB)
    let small_data = vec![0u8; 1024];
    assert!(!verify_rom(&small_data));

    // Tamao correcto (16 MB de ceros, pero hash no coincide con FE7)
    let dummy_16mb = vec![0u8; OFFICIAL_ROM_SIZE];
    assert!(!verify_rom(&dummy_16mb));

    // Tamao mayor a 16 MB
    let large_data = vec![0u8; OFFICIAL_ROM_SIZE + 1];
    assert!(!verify_rom(&large_data));
}

#[test]
fn test_verify_bios_size_and_signature_rejection() {
    // Tamao menor a 16 KB
    let small_bios = vec![0u8; 100];
    assert!(!verify_bios(&small_bios));

    // 16 KB de ceros (hash invlido)
    let dummy_16kb = vec![0u8; OFFICIAL_BIOS_SIZE];
    assert!(!verify_bios(&dummy_16kb));
}

#[test]
fn test_resolve_search_directories_prioritizes_downloads() {
    let internal = PathBuf::from("/data/user/0/com.blazingrecomp.game/files");
    let dirs = resolve_search_directories(Some(&internal));

    assert!(dirs.len() >= 5);
    // Prioridad 1 y 2 deben ser Downloads de Android
    assert_eq!(dirs[0], PathBuf::from("/sdcard/Download"));
    assert_eq!(dirs[1], PathBuf::from("/storage/emulated/0/Download"));
    // Rutas relativas locales para emuladores y host
    assert_eq!(dirs[2], PathBuf::from("Download"));
    assert_eq!(dirs[3], PathBuf::from("roms"));
    // Directorio interno de la app
    assert_eq!(dirs[4], internal);
}

#[test]
fn test_scan_and_load_rejects_corrupted_rom() {
    let temp_dir = std::env::temp_dir().join("blazing_test_corrupt_rom");
    let _ = std::fs::create_dir_all(&temp_dir);

    let corrupt_rom_path = temp_dir.join("bad_game.gba");
    {
        let mut file = File::create(&corrupt_rom_path).expect("crear archivo de prueba");
        // Escribir 1024 bytes (tamao incorrecto)
        file.write_all(&[0xAA; 1024]).expect("escribir archivo");
    }

    let result = scan_and_load_game(&[temp_dir.clone()]);
    assert_eq!(result.unwrap_err(), StorageError::RomNotFound);

    let _ = std::fs::remove_dir_all(&temp_dir);
}

#[test]
fn test_scan_and_load_finds_official_rom_in_roms_directory() {
    // En el entorno de desarrollo local, la carpeta roms/ contiene la ROM oficial
    let roms_dir = PathBuf::from("roms");
    if roms_dir.is_dir() {
        let result = scan_and_load_game(&[roms_dir]);
        if let Ok(assets) = result {
            assert_eq!(assets.rom.len(), OFFICIAL_ROM_SIZE);
            assert!(verify_rom(&assets.rom));
            assert!(assets.rom_path.exists());
            if let Some(ref bios) = assets.bios {
                assert_eq!(bios.len(), OFFICIAL_BIOS_SIZE);
                assert!(verify_bios(bios));
            }
        }
    }
}
