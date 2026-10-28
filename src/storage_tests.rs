use super::*;
use std::fs::File;
use std::io::Write;

#[test]
fn test_sha256_nist_vectors() {
    let hash_abc = compute_sha256(b"abc");
    assert_eq!(
        hash_abc,
        "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
    );

    let hash_empty = compute_sha256(b"");
    assert_eq!(
        hash_empty,
        "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
    );
}

#[test]
fn test_verify_rom_size_and_signature_rejection() {
    let small_data = vec![0u8; 1024];
    assert!(!verify_rom(&small_data));

    let dummy_16mb = vec![0u8; OFFICIAL_ROM_SIZE];
    assert!(!verify_rom(&dummy_16mb));

    let large_data = vec![0u8; OFFICIAL_ROM_SIZE + 1];
    assert!(!verify_rom(&large_data));
}

#[test]
fn test_verify_bios_size_and_signature_rejection() {
    let small_bios = vec![0u8; 100];
    assert!(!verify_bios(&small_bios));

    let dummy_16kb = vec![0u8; OFFICIAL_BIOS_SIZE];
    assert!(!verify_bios(&dummy_16kb));
}

#[test]
fn test_resolve_search_directories_includes_external_data() {
    let internal = PathBuf::from("/data/user/0/com.blazingrecomp.game/files");
    let external = PathBuf::from("/sdcard/Android/data/com.blazingrecomp.game/files");
    let dirs = resolve_search_directories(Some(&internal), Some(&external));

    assert!(dirs.len() >= 8);
    // Prioridad 1 debe ser la carpeta propia de la app (inmune a Scoped Storage)
    assert_eq!(dirs[0], external);
    // Prioridad 2 y 3 deben ser Downloads de Android
    assert_eq!(dirs[1], PathBuf::from("/sdcard/Download"));
    assert_eq!(dirs[2], PathBuf::from("/storage/emulated/0/Download"));
    // Rutas relativas locales
    assert_eq!(dirs[4], PathBuf::from("Download"));
    assert_eq!(dirs[5], PathBuf::from("roms"));
    // Rutas de emuladores (Android 9 / BlueStacks / Nox)
    assert_eq!(dirs[6], PathBuf::from("/mnt/sdcard/Download"));
    assert_eq!(dirs[7], PathBuf::from("/storage/self/primary/Download"));
    // Directorio interno al final
    assert_eq!(dirs[8], internal);
}

#[test]
fn test_scan_and_load_rejects_corrupted_rom() {
    let temp_dir = std::env::temp_dir().join("blazing_test_corrupt_rom");
    let _ = std::fs::create_dir_all(&temp_dir);

    let corrupt_rom_path = temp_dir.join("bad_game.gba");
    {
        let mut file = File::create(&corrupt_rom_path).expect("crear archivo de prueba");
        file.write_all(&[0xAA; 1024]).expect("escribir archivo");
    }

    let result = scan_and_load_game(&[temp_dir.clone()]);
    assert_eq!(result.unwrap_err(), StorageError::RomNotFound);

    let _ = std::fs::remove_dir_all(&temp_dir);
}

#[test]
fn test_scan_and_load_direct_file_probing() {
    let temp_dir = std::env::temp_dir().join("blazing_test_probing");
    let _ = std::fs::create_dir_all(&temp_dir);

    // Archivo con nombre conocido pero tamaño erróneo
    let probe_path = temp_dir.join("fe7.gba");
    {
        let mut file = File::create(&probe_path).expect("crear fe7.gba");
        file.write_all(&[0xBB; 2048]).expect("escribir archivo");
    }

    // Debe ser rechazado porque no tiene 16 MB
    let result = scan_and_load_game(&[temp_dir.clone()]);
    assert_eq!(result.unwrap_err(), StorageError::RomNotFound);

    let _ = std::fs::remove_dir_all(&temp_dir);
}

#[test]
fn test_render_guide_screen_rasterizes_pixels() {
    let mut buffer = vec![[0u8; 4]; 240 * 160];
    let lines = [
        "BLAZING RECOMP - GUIA",
        "Coloca fe7.gba en Download/",
        "o en Android/data/.../files/",
    ];
    render_guide_screen(&mut buffer, "GUIA DE INSTALACION", &lines);

    // Debe haber píxeles no negros en el fondo
    assert_eq!(buffer[0], [27, 73, 101, 255]); // Barra de título
    assert_eq!(buffer[100 * 240], [13, 27, 42, 255]); // Fondo azul noche

    // Debe haber píxeles claros de texto rasterizado
    let text_pixels = buffer.iter().filter(|px| px[0] > 200).count();
    assert!(text_pixels > 50, "Debe haber texto rasterizado visible");
}

#[test]
fn test_render_splash_screen_rasterizes_typography() {
    let mut buffer = vec![[0u8, 0u8, 0u8, 255u8]; 240 * 160];
    render_splash_screen(&mut buffer);

    // Debe contener píxeles dorados del título "BLAZING RECOMP" (Outfit Bold, #FFD60A)
    let gold_pixels = buffer
        .iter()
        .filter(|px| px[0] == 255 && px[1] == 214 && px[2] == 10)
        .count();
    assert!(
        gold_pixels > 50,
        "El splash screen debe rasterizar el título en Outfit Bold: {}",
        gold_pixels
    );

    // Debe contener píxeles celestes del subtítulo "CARGANDO MOTOR..." (JetBrains Mono Bold, #B4CDED)
    let sub_pixels = buffer
        .iter()
        .filter(|px| px[0] == 180 && px[1] == 205 && px[2] == 237)
        .count();
    assert!(
        sub_pixels > 30,
        "El splash screen debe rasterizar el subtítulo en JetBrains Mono Bold: {}",
        sub_pixels
    );
}

#[test]
fn test_scan_and_load_finds_official_rom_in_roms_directory() {
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

#[test]
fn test_load_and_restore_save_data() {
    let dummy_save = vec![0x42u8; SAVE_FILE_SIZE];
    let rom = vec![0u8; 1024];
    let mut runner = crate::runner::GameRunner::new(rom, None, screen::ScreenKind::Frontlit);
    assert!(runner.load_save_data(&dummy_save));
    assert_eq!(runner.save_data(), Some(dummy_save.as_slice()));
}

#[test]
fn test_write_save_file_atomic_success_and_cleanup() {
    let temp_dir = std::path::PathBuf::from("target").join(format!(
        "blazing_test_save_{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let _ = std::fs::create_dir_all(&temp_dir);
    let target_save = temp_dir.join("test_game.sav");
    let tmp_save = temp_dir.join("test_game.sav.tmp");

    let data = vec![0x37u8; SAVE_FILE_SIZE];
    assert!(write_save_file_atomic(&target_save, &data).is_ok());
    assert!(target_save.exists());
    assert!(
        !tmp_save.exists(),
        "El archivo temporal .tmp debe eliminarse tras el renombramiento atómico"
    );
    assert_eq!(std::fs::read(&target_save).unwrap(), data);

    let _ = std::fs::remove_dir_all(&temp_dir);
}

#[test]
fn test_load_save_file_missing_or_truncated() {
    let temp_dir = std::path::PathBuf::from("target").join(format!(
        "blazing_test_trunc_{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let _ = std::fs::create_dir_all(&temp_dir);

    // 1. Archivo inexistente
    let missing_save = temp_dir.join("non_existent.sav");
    assert_eq!(load_save_file(&missing_save), None);

    // 2. Archivo truncado (< 64 KB)
    let truncated_save = temp_dir.join("truncated.sav");
    std::fs::write(&truncated_save, vec![0xAAu8; 1024]).unwrap();
    assert_eq!(load_save_file(&truncated_save), None);

    // 3. Archivo válido (64 KB)
    let valid_save = temp_dir.join("valid.sav");
    let valid_data = vec![0x55u8; SAVE_FILE_SIZE];
    std::fs::write(&valid_save, &valid_data).unwrap();
    assert_eq!(load_save_file(&valid_save), Some(valid_data));

    let _ = std::fs::remove_dir_all(&temp_dir);
}
