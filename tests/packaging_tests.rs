//! Pruebas unitarias para el arnés de empaquetado de Android (Fase 7,
//! propuesta `2026-10-09_arnes-empaquetado-apk`).
//!
//! Validan la estructura del manifiesto (`android/AndroidManifest.xml`),
//! la portabilidad de los scripts y la integridad del APK empaquetado.

use std::fs;
use std::path::PathBuf;

/// Resuelve la raíz del repositorio de forma relativa al manifiesto de Cargo.
fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

#[test]
fn test_manifest_declares_native_activity() {
    let manifest_path = repo_root().join("android").join("AndroidManifest.xml");
    assert!(
        manifest_path.exists(),
        "El archivo android/AndroidManifest.xml debe existir"
    );

    let content = fs::read_to_string(&manifest_path)
        .expect("Debe poder leerse android/AndroidManifest.xml");

    // Validar declaracion de NativeActivity
    assert!(
        content.contains("android.app.NativeActivity"),
        "El manifiesto debe declarar android.app.NativeActivity"
    );

    // Validar metadata lib_name apuntando a blazing_recomp
    assert!(
        content.contains("android.app.lib_name"),
        "El manifiesto debe incluir la propiedad android.app.lib_name"
    );
    assert!(
        content.contains("blazing_recomp"),
        "El valor de android.app.lib_name debe ser blazing_recomp"
    );

    // Validar permiso de vibracion
    assert!(
        content.contains("android.permission.VIBRATE"),
        "El manifiesto debe solicitar el permiso android.permission.VIBRATE"
    );

    // Validar extractNativeLibs="true" para garantizar extracción física en emuladores como BlueStacks
    assert!(
        content.contains("android:extractNativeLibs=\"true\""),
        "El manifiesto debe declarar android:extractNativeLibs=\"true\" para permitir carga directa y compatibilidad con emuladores"
    );

    // Validar hasCode="false" para NativeActivity pura
    assert!(
        content.contains("android:hasCode=\"false\""),
        "El manifiesto debe declarar android:hasCode=\"false\" para ejecucion sin runtime Java"
    );
}

#[test]
fn test_manifest_extract_native_libs_enabled() {
    let manifest_path = repo_root().join("android").join("AndroidManifest.xml");
    let content = fs::read_to_string(&manifest_path)
        .expect("Debe poder leerse android/AndroidManifest.xml");
    assert!(
        content.contains("android:extractNativeLibs=\"true\""),
        "El manifiesto debe tener android:extractNativeLibs=\"true\" configurado para compatibilidad con emuladores"
    );
}

#[test]
fn test_no_local_paths_in_packaging() {
    let check_dirs = [
        repo_root().join("android"),
        repo_root().join("scripts"),
    ];

    let forbidden_patterns = [
        "c:/", "c:\\", "d:/", "d:\\", "/users/", "\\users\\",
    ];

    for dir in &check_dirs {
        if !dir.exists() {
            continue;
        }

        for entry in fs::read_dir(dir).expect("Debe poder leer el directorio") {
            let entry = entry.expect("Entrada valida");
            let path = entry.path();
            if path.is_file() {
                let content = fs::read_to_string(&path)
                    .unwrap_or_else(|_| panic!("Debe poder leerse el archivo {:?}", path));
                let lower = content.to_lowercase();
                for pat in &forbidden_patterns {
                    assert!(
                        !lower.contains(pat),
                        "El archivo {:?} contiene una ruta absoluta prohibida '{}'",
                        path,
                        pat
                    );
                }
            }
        }
    }
}

#[derive(Debug)]
struct ZipEntryInfo {
    name: String,
    compression_method: u16,
    data_offset: usize,
    comp_size: u32,
    uncomp_size: u32,
}

#[test]
fn test_apk_generation_and_signature() {
    let script_path = repo_root().join("scripts").join("package_apk.ps1");
    assert!(
        script_path.exists(),
        "El script scripts/package_apk.ps1 debe existir"
    );

    let script_content = fs::read_to_string(&script_path)
        .expect("Debe poder leerse scripts/package_apk.ps1");

    // Verificar que el script cubre el pipeline completo
    assert!(
        script_content.contains("cargo ndk"),
        "El script debe compilar con cargo ndk"
    );
    assert!(
        script_content.contains("aapt2 link") || script_content.contains("aapt2"),
        "El script debe ensamblar con aapt2"
    );
    assert!(
        script_content.contains("zipalign"),
        "El script debe alinear con zipalign"
    );
    assert!(
        script_content.contains("apksigner"),
        "El script debe firmar y verificar con apksigner"
    );

    // Si el APK ya fue generado, verificar su estructura como archivo zip valido
    let apk_path = repo_root().join("target").join("blazing-recomp-debug.apk");
    if apk_path.exists() {
        let file = fs::File::open(&apk_path).expect("Debe abrirse el APK generado");
        let metadata = file.metadata().expect("Debe obtener metadatos");
        assert!(
            metadata.len() > 1_000_000,
            "El APK debe contener la biblioteca ELF ARM64 (tamano > 1 MB)"
        );

        // Validar entradas del ZIP del APK
        let entries = zip_read_check(&file);
        assert!(
            entries.iter().any(|e| e.name == "AndroidManifest.xml"),
            "El APK debe contener AndroidManifest.xml"
        );

        let so_entry = entries
            .iter()
            .find(|e| e.name.contains("libblazing_recomp.so"))
            .expect("El APK debe contener libblazing_recomp.so");

        // Validar que la biblioteca esta almacenada sin compresion (STORED = metodo 0)
        assert_eq!(
            so_entry.compression_method, 0,
            "libblazing_recomp.so debe almacenarse SIN compresion (metodo 0 STORED) para permitir mmap directo"
        );
        assert_eq!(
            so_entry.comp_size, so_entry.uncomp_size,
            "Para un archivo STORED, el tamano comprimido debe ser exactamente igual al descomprimido"
        );

        // Validar que el inicio de datos de la biblioteca esta alineado a paginas de 16 KB (Android 15)
        assert_eq!(
            so_entry.data_offset % 16384,
            0,
            "El offset de datos ({}) de libblazing_recomp.so debe estar alineado a 16 KB (16384 bytes) para Android 15",
            so_entry.data_offset
        );
    }
}

/// Lector basico de cabeceras locales de ZIP para no anadir dependencias externas.
fn zip_read_check(mut file: &fs::File) -> Vec<ZipEntryInfo> {
    use std::io::{Read, Seek, SeekFrom};
    let mut entries = Vec::new();
    let mut buffer = Vec::new();
    let _ = file.seek(SeekFrom::Start(0));
    let _ = file.read_to_end(&mut buffer);

    let mut cursor = 0;
    while cursor + 30 <= buffer.len() {
        // Local file header signature: 0x04034b50
        if buffer[cursor..cursor + 4] == [0x50, 0x4b, 0x03, 0x04] {
            let compression_method =
                u16::from_le_bytes([buffer[cursor + 8], buffer[cursor + 9]]);
            let comp_size = u32::from_le_bytes([
                buffer[cursor + 18],
                buffer[cursor + 19],
                buffer[cursor + 20],
                buffer[cursor + 21],
            ]);
            let uncomp_size = u32::from_le_bytes([
                buffer[cursor + 22],
                buffer[cursor + 23],
                buffer[cursor + 24],
                buffer[cursor + 25],
            ]);
            let name_len = u16::from_le_bytes([buffer[cursor + 26], buffer[cursor + 27]]) as usize;
            let extra_len = u16::from_le_bytes([buffer[cursor + 28], buffer[cursor + 29]]) as usize;

            let name_start = cursor + 30;
            let data_offset = name_start + name_len + extra_len;
            if name_start + name_len <= buffer.len() {
                if let Ok(name) = std::str::from_utf8(&buffer[name_start..name_start + name_len]) {
                    entries.push(ZipEntryInfo {
                        name: name.to_string(),
                        compression_method,
                        data_offset,
                        comp_size,
                        uncomp_size,
                    });
                }
            }
            cursor = data_offset + comp_size as usize;
        } else {
            // Avanzar buscando proxima firma PK
            cursor += 1;
        }
    }
    entries
}

#[test]
fn test_manifest_declares_android_tv_compatibility() {
    let manifest_path = repo_root().join("android").join("AndroidManifest.xml");
    let content = fs::read_to_string(&manifest_path)
        .expect("Debe poder leerse android/AndroidManifest.xml");

    // Validar que la pantalla tactil NO es obligatoria para permitir instalacion en Android TV
    assert!(
        content.contains("android.hardware.touchscreen") && content.contains("android:required=\"false\""),
        "El manifiesto debe declarar android.hardware.touchscreen con required=\"false\" para permitir instalacion en televisores"
    );

    // Validar compatibilidad con Leanback UI
    assert!(
        content.contains("android.software.leanback"),
        "El manifiesto debe declarar android.software.leanback con required=\"false\""
    );

    // Validar categoria de launcher de Android TV
    assert!(
        content.contains("android.intent.category.LEANBACK_LAUNCHER"),
        "El manifiesto debe declarar la categoria LEANBACK_LAUNCHER para visibilidad en pantalla de inicio de TV"
    );

    // Validar metadata/atributo de juego
    assert!(
        content.contains("android:isGame=\"true\""),
        "El manifiesto debe declarar android:isGame=\"true\" en el elemento application"
    );
}

#[test]
fn test_packaging_script_enables_v1_v2_v3_signatures() {
    let script_path = repo_root().join("scripts").join("package_apk.ps1");
    let content = fs::read_to_string(&script_path)
        .expect("Debe poder leerse scripts/package_apk.ps1");

    assert!(
        content.contains("--v1-signing-enabled true"),
        "El script package_apk.ps1 debe habilitar --v1-signing-enabled true para compatibilidad con instaladores heredados y Android TV"
    );
    assert!(
        content.contains("--v2-signing-enabled true"),
        "El script package_apk.ps1 debe habilitar --v2-signing-enabled true"
    );
    assert!(
        content.contains("--v3-signing-enabled true"),
        "El script package_apk.ps1 debe habilitar --v3-signing-enabled true"
    );
}

#[test]
fn test_packaging_script_supports_armeabi_v7a() {
    let script_path = repo_root().join("scripts").join("package_apk.ps1");
    let content = fs::read_to_string(&script_path)
        .expect("Debe poder leerse scripts/package_apk.ps1");

    assert!(
        content.contains("\"armv7-linux-androideabi\"  = \"armeabi-v7a\"")
            || content.contains("\"armv7-linux-androideabi\" = \"armeabi-v7a\""),
        "El script package_apk.ps1 debe mapear armv7-linux-androideabi a armeabi-v7a"
    );
}

#[test]
fn test_reference_docs_in_sync_with_code() {
    let script_path = repo_root().join("scripts").join("extraer_referencias.py");
    assert!(
        script_path.exists(),
        "El script scripts/extraer_referencias.py debe existir"
    );

    let doc_path = repo_root()
        .join("docs")
        .join("referencia")
        .join("api-modulos-rust.md");
    assert!(
        doc_path.exists(),
        "El archivo docs/referencia/api-modulos-rust.md debe existir"
    );

    // Si python está disponible en el entorno, ejecutar chequeo estricto
    let status = std::process::Command::new("python")
        .arg(&script_path)
        .arg("--check")
        .current_dir(repo_root())
        .status();

    if let Ok(st) = status {
        assert!(
            st.success(),
            "docs/referencia/api-modulos-rust.md está desactualizado respecto a src/. Ejecuta 'python scripts/extraer_referencias.py'."
        );
    }
}

