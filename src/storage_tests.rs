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

    assert!(dirs.len() >= 6);
    // Prioridad 1 debe ser la carpeta propia de la app (inmune a Scoped Storage)
    assert_eq!(dirs[0], external);
    // Prioridad 2 y 3 deben ser Downloads de Android
    assert_eq!(dirs[1], PathBuf::from("/sdcard/Download"));
    assert_eq!(dirs[2], PathBuf::from("/storage/emulated/0/Download"));
    // Rutas relativas locales
    assert_eq!(dirs[4], PathBuf::from("Download"));
    assert_eq!(dirs[5], PathBuf::from("roms"));
    // Directorio interno al final
    assert_eq!(dirs[6], internal);
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
