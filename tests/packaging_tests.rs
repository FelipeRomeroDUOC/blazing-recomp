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

/// Nombres candidatos del APK final, en orden de preferencia (`release` primero).
///
/// El script deriva el nombre del perfil de compilacion, asi que un mismo `target/` puede tener
/// artefactos de los dos perfiles. Se listan ambos para que los tests de estructura sigan
/// validando el paquete despues de cualquier renombrado, en vez de fijar una sola ruta.
fn candidatos_apk() -> Vec<PathBuf> {
    const NOMBRES: [&str; 2] = ["blazing-recomp-release.apk", "blazing-recomp-debug.apk"];
    NOMBRES
        .iter()
        .map(|nombre| repo_root().join("target").join(nombre))
        .collect()
}

/// Primer candidato que existe en disco, si hay alguno.
fn apk_final() -> Option<PathBuf> {
    candidatos_apk().into_iter().find(|ruta| ruta.exists())
}

/// Presupuesto maximo del APK de tres ABIs con las librerias nativas comprimidas (60 MiB).
///
/// El APK sin comprimir pesaba 186.6 MiB y el empaquetado DEFLATE medido da ~44 MB, asi que
/// el umbral deja margen suficiente sin permitir una regresion silenciosa a STORED.
const APK_SIZE_BUDGET_BYTES: u64 = 62_914_560;

/// El nombre del APK final debe derivarse del perfil de compilacion, no de una cadena fija:
/// un paquete de perfil `release` llamandose `-debug` hace indistinguibles dos artefactos.
#[test]
fn test_packaging_script_names_apk_after_profile() {
    let script_path = repo_root().join("scripts").join("package_apk.ps1");
    let script_content =
        fs::read_to_string(&script_path).expect("Debe poder leerse scripts/package_apk.ps1");

    assert!(
        script_content.contains("blazing-recomp-$targetProfile.apk"),
        "El nombre del APK final debe derivarse del perfil de compilacion"
    );
    assert!(
        !script_content.contains("\"blazing-recomp-debug.apk\""),
        "No debe quedar ningun nombre de APK final fijo que ignore el perfil"
    );
}

/// Los tests de estructura abren el APK bajo `if exists`, asi que si la lista de candidatos se
/// desincroniza del nombre que produce el script dejarian de validar sin fallar. Aqui se cruzan
/// las dos cosas: los nombres que el script produce para cada perfil y los que buscan los tests.
#[test]
fn test_candidatos_apk_cubren_ambos_perfiles() {
    let script_content = fs::read_to_string(repo_root().join("scripts").join("package_apk.ps1"))
        .expect("Debe poder leerse scripts/package_apk.ps1");

    let patron = script_content
        .lines()
        .find_map(|linea| {
            linea
                .trim()
                .strip_prefix("$finalApk = Join-Path $targetDir \"")?
                .strip_suffix('"')
                .map(str::to_string)
        })
        .expect("El script debe declarar el nombre del APK final");

    let esperados: Vec<String> = ["release", "debug"]
        .iter()
        .map(|perfil| patron.replace("$targetProfile", perfil))
        .collect();
    let reales: Vec<String> = candidatos_apk()
        .iter()
        .map(|ruta| ruta.file_name().unwrap().to_string_lossy().into_owned())
        .collect();

    assert_eq!(
        reales, esperados,
        "Los candidatos que buscan estos tests deben coincidir con los nombres que el script produce para cada perfil"
    );
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
    if let Some(apk_path) = apk_final() {
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

        // El manifiesto declara android:extractNativeLibs="true", asi que Android extrae las
        // bibliotecas al instalar y acepta entradas comprimidas. El empaquetado STORED solo era
        // necesario en el modo extractNativeLibs="false" (carga por mmap desde el propio APK),
        // y costaba 4.5x de tamano. Contrato nuevo: DEFLATE (metodo 8).
        let lib_so_entries: Vec<&ZipEntryInfo> = entries
            .iter()
            .filter(|e| e.name.starts_with("lib/") && e.name.ends_with(".so"))
            .collect();

        assert_eq!(
            lib_so_entries.len(),
            6,
            "El APK fat debe contener 6 entradas lib/**/*.so (3 ABIs x runtime + AOT), encontradas: {:?}",
            lib_so_entries
                .iter()
                .map(|e| e.name.as_str())
                .collect::<Vec<_>>()
        );

        for entry in &lib_so_entries {
            assert_eq!(
                entry.compression_method, 8,
                "{} debe almacenarse comprimido (metodo 8 DEFLATE) para reducir el tamano del APK",
                entry.name
            );
            assert!(
                entry.comp_size < entry.uncomp_size,
                "{} debe tener tamano comprimido ({}) menor que el descomprimido ({})",
                entry.name,
                entry.comp_size,
                entry.uncomp_size
            );
        }

        assert!(
            metadata.len() <= APK_SIZE_BUDGET_BYTES,
            "El APK de tres ABIs pesa {} bytes, por encima del presupuesto de {} bytes",
            metadata.len(),
            APK_SIZE_BUDGET_BYTES
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

/// Cada propuesta de `docs/propuestas/` declara un estado del vocabulario cerrado.
///
/// El estado es la unica senal de en que punto esta una propuesta, y hasta el 2026-10-14 se habian
/// colado tres variantes que ni los scripts ni los agentes saben leer: dos propuestas con
/// `**Estado:**` en negritas (plantilla anterior) y una con `completada`, que no existe. Las fichas
/// se normalizaron y este guardian impide que la firma vuelva a divergir; el vocabulario esta
/// especificado en `docs/propuestas/README.md`.
#[test]
fn test_propuestas_declaran_estado_valido() {
    const VOCABULARIO: [&str; 8] = [
        "pendiente",
        "aprobada",
        "modificada",
        "descartada",
        "verificada por agente",
        "revisada por humano",
        "discrepada",
        "cerrada",
    ];

    let dir = repo_root().join("docs").join("propuestas");
    assert!(dir.is_dir(), "docs/propuestas debe existir");

    let mut propuestas = 0usize;
    for entrada in fs::read_dir(&dir).expect("docs/propuestas debe poder leerse") {
        let ruta = entrada.expect("entrada legible").path();
        if ruta.extension().and_then(|e| e.to_str()) != Some("md") {
            continue;
        }
        let nombre = ruta
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("<sin nombre>")
            .to_string();
        if nombre == "README.md" {
            continue;
        }
        propuestas += 1;

        let texto = fs::read_to_string(&ruta).expect("la propuesta debe poder leerse");
        let estado = texto
            .lines()
            .find(|linea| linea.starts_with("Estado:"))
            .map(|linea| linea.trim_start_matches("Estado:").trim())
            .unwrap_or("");

        assert!(
            !estado.is_empty(),
            "{nombre}: falta la linea propia `Estado:` (el estado en negritas no vale: ver docs/propuestas/README.md)"
        );
        assert!(
            VOCABULARIO.iter().any(|valido| estado.starts_with(valido)),
            "{nombre}: estado `{estado}` fuera del vocabulario {VOCABULARIO:?}"
        );
    }

    assert!(
        propuestas >= 50,
        "se esperaban las 57 propuestas de docs/propuestas, se leyeron {propuestas}"
    );
}

#[test]
fn test_manifest_declares_storage_permissions() {
    let manifest_path = repo_root().join("android").join("AndroidManifest.xml");
    assert!(
        manifest_path.exists(),
        "El archivo android/AndroidManifest.xml debe existir"
    );

    let content = fs::read_to_string(&manifest_path)
        .expect("Debe poder leerse android/AndroidManifest.xml");

    assert!(
        content.contains("android.permission.READ_EXTERNAL_STORAGE"),
        "El manifiesto debe solicitar android.permission.READ_EXTERNAL_STORAGE para acceder a Downloads"
    );

    assert!(
        content.contains("android.permission.MANAGE_EXTERNAL_STORAGE"),
        "El manifiesto debe solicitar android.permission.MANAGE_EXTERNAL_STORAGE para acceso a archivos en Android 11-16"
    );

    assert!(
        content.contains("android:requestLegacyExternalStorage=\"true\""),
        "El manifiesto debe declarar requestLegacyExternalStorage=\"true\" para lectura directa en emuladores y dispositivos"
    );
}
#[test]
fn test_packaging_script_embeds_recompiled_library() {
    let script_path = repo_root().join("scripts").join("package_apk.ps1");
    assert!(
        script_path.exists(),
        "El script scripts/package_apk.ps1 debe existir"
    );

    let script_content = fs::read_to_string(&script_path)
        .expect("Debe poder leerse scripts/package_apk.ps1");

    assert!(
        script_content.contains("libblazing_recompiled.so"),
        "El script debe gestionar el empaquetado de libblazing_recompiled.so"
    );
    assert!(
        script_content.contains("src-c"),
        "El script debe compilar los fuentes C11 ubicados en src-c"
    );
    assert!(
        script_content.contains("Build-Recompiled-C"),
        "El script debe definir la rutina Build-Recompiled-C"
    );
}

#[test]
fn test_packaging_script_soname_flag() {
    let script_path = repo_root().join("scripts").join("package_apk.ps1");
    assert!(
        script_path.exists(),
        "El script scripts/package_apk.ps1 debe existir"
    );

    let script_content = fs::read_to_string(&script_path)
        .expect("Debe poder leerse scripts/package_apk.ps1");

    assert!(
        script_content.contains("-Wl,-soname,libblazing_recompiled.so"),
        "El script de empaquetado debe incluir el flag de SONAME al enlazar libblazing_recompiled.so"
    );
}

#[test]
fn test_apk_keeps_arsc_stored() {
    // Android 11+ exige resources.arsc sin comprimir. Comprimir lib/ no debe tocarlo:
    // `jar -u -C <staging> lib` solo reemplaza entradas bajo lib/, y este test lo fija.
    let Some(apk_path) = apk_final() else {
        return;
    };

    let file = fs::File::open(&apk_path).expect("Debe abrirse el APK generado");
    let entries = zip_read_check(&file);

    let arsc = entries
        .iter()
        .find(|e| e.name == "resources.arsc")
        .expect("El APK debe contener resources.arsc");

    assert_eq!(
        arsc.compression_method, 0,
        "resources.arsc debe seguir almacenado sin comprimir (metodo 0): Android 11+ lo exige"
    );
}

#[test]
fn test_packaging_script_respects_single_abi_flag() {
    let script_path = repo_root().join("scripts").join("package_apk.ps1");
    let script_content =
        fs::read_to_string(&script_path).expect("Debe poder leerse scripts/package_apk.ps1");

    let lineas: Vec<&str> = script_content.lines().collect();
    let indice_bucle = lineas
        .iter()
        .position(|l| l.trim() == "foreach ($arch in $abiMap.Keys) {")
        .expect("El script debe recorrer los ABIs adicionales presentes en target/");

    // Sin la guarda, `-AllAbis:$false` sigue incluyendo cualquier ABI ya compilado en
    // target/ y el APK sale gordo igual. La primera linea no vacia anterior debe ser la guarda.
    let mut j = indice_bucle;
    let guardado = loop {
        if j == 0 {
            break false;
        }
        j -= 1;
        let linea = lineas[j].trim();
        if linea.is_empty() {
            continue;
        }
        break linea.starts_with("if ($AllAbis)");
    };

    assert!(
        guardado,
        "El bucle de ABIs adicionales debe estar condicionado por `if ($AllAbis)` para que -AllAbis:$false produzca un APK de un solo ABI"
    );
}

/// Documentos que describen el estado **actual** del proyecto.
///
/// Las propuestas (`docs/propuestas/`) quedan fuera a proposito: son registro historico y citan
/// cifras viejas dentro de su medicion. `test_guardian_ignora_el_registro_historico` fija esa
/// exclusion y comprueba que no es letra muerta.
const DOCS_ESTADO_ACTUAL: [&str; 6] = [
    "src/lib.rs",
    "docs/referencia/api-modulos-rust.md",
    "docs/explicacion/arquitectura-c-recompilado.md",
    "AGENTS.md",
    "docs/BACKLOG.md",
    "CHANGELOG.md",
];

/// Frases que el codigo ya desmiente y que no pueden reaparecer en un documento de estado actual.
///
/// Origen de cada una (catastro del 2026-10-14):
/// - `La emulacion aun no produce fotogramas`: la PPU compone fotogramas y el juego corre a 60 FPS.
/// - `17,207 entradas`: `rcg_block_count = 144875` en `src-c/...-bios.11.c`.
/// - `128 tests`: la suite tiene 131 (109 en los satelites de `src/`, 18 aqui y 4 en golden).
/// - `53 funciones verificadas`: `blazing.labels.toml` tiene 36.966 claves name/address.
const FRASES_CADUCAS: [&str; 4] = [
    "La emulación aún no produce fotogramas",
    "17,207 entradas",
    "128 tests",
    "53 funciones verificadas",
];

/// Ningun documento de estado actual afirma algo que el codigo contradice.
///
/// El caso real: `src/lib.rs` declaraba que la emulacion no dibujaba (y la frase viajaba al
/// documento generado), `arquitectura-c-recompilado.md` atribuia a `rcg_blocks` un numero de
/// entradas que no es el suyo, y los conteos de tests y funciones se habian quedado atras.
/// Este guardian impide que cualquiera de las cuatro frases vuelva a colarse.
#[test]
fn test_documentos_sin_afirmaciones_caducas() {
    for ruta_rel in DOCS_ESTADO_ACTUAL {
        let ruta = repo_root().join(ruta_rel);
        assert!(ruta.exists(), "{ruta_rel} debe existir");
        let texto = fs::read_to_string(&ruta).expect("el documento debe poder leerse");
        for frase in FRASES_CADUCAS {
            assert!(
                !texto.contains(frase),
                "{ruta_rel} contiene una afirmacion caduca: `{frase}`"
            );
        }
    }

    // Y la cifra buena tiene que estar donde toca, no solo ausente la mala.
    let arquitectura = fs::read_to_string(
        repo_root()
            .join("docs")
            .join("explicacion")
            .join("arquitectura-c-recompilado.md"),
    )
    .expect("arquitectura-c-recompilado.md debe poder leerse");
    assert!(
        arquitectura.contains("rcg_block_count = 144875"),
        "arquitectura-c-recompilado.md debe nombrar el conteo real de bloques (`rcg_block_count = 144875`)"
    );
}

/// El guardian de frases caducas ignora el registro historico, y esa exclusion importa.
///
/// Si alguien anadiera `docs/propuestas/` a la lista, el guardian empezaria a morder las
/// mediciones que citan las cifras viejas a proposito. Este test lo impide y, ademas, comprueba
/// que hoy hay al menos una propuesta que cita una de esas frases (si no, la exclusion seria una
/// precaucion sin efecto).
#[test]
fn test_guardian_ignora_el_registro_historico() {
    for ruta_rel in DOCS_ESTADO_ACTUAL {
        assert!(
            !ruta_rel.starts_with("docs/propuestas/"),
            "{ruta_rel} es registro historico: no puede entrar en DOCS_ESTADO_ACTUAL"
        );
    }

    let dir = repo_root().join("docs").join("propuestas");
    assert!(dir.is_dir(), "docs/propuestas debe existir");

    let mut propuestas_con_frase_caduca = 0usize;
    for entrada in fs::read_dir(&dir).expect("docs/propuestas debe poder leerse") {
        let ruta = entrada.expect("entrada legible").path();
        if ruta.extension().and_then(|e| e.to_str()) != Some("md") {
            continue;
        }
        let texto = fs::read_to_string(&ruta).expect("la propuesta debe poder leerse");
        if FRASES_CADUCAS.iter().any(|frase| texto.contains(frase)) {
            propuestas_con_frase_caduca += 1;
        }
    }

    assert!(
        propuestas_con_frase_caduca >= 1,
        "se esperaba al menos una propuesta que cite una frase caduca dentro de su medicion; si ya no queda ninguna, revisar si la exclusion de docs/propuestas/ sigue teniendo efecto"
    );
}
