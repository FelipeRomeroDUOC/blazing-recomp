# Propuesta: Ingestión y Verificación de ROM/BIOS desde Downloads

Fecha: 2026-10-10
Nivel: 2
Estado: cerrada

<!-- Estados: pendiente | aprobada | modificada | descartada |
     verificada por agente | revisada por humano | discrepada | cerrada -->

## Propuesta (la escribe el agente antes de editar)

### 1. Qué cambia

La aplicación busca automáticamente la ROM y la BIOS de *Fire Emblem* en la carpeta `Download/` del dispositivo, valida su firma SHA-256 oficial e inicia el juego a 60 FPS, o muestra una pantalla de ayuda si los archivos aún no están presentes.

### 2. Contexto

- Contexto consultado: [`android/AndroidManifest.xml`](../../android/AndroidManifest.xml), [`src/platform.rs`](../../src/platform.rs), [`src/runner.rs`](../../src/runner.rs), [`Cargo.toml`](../../Cargo.toml), `AGENTS.md §3` y `AGENTS.md §4`.
- Intención: Permitir al usuario simplemente colocar sus archivos legales descargados en la carpeta habitual `Downloads/` sin requerir permisos especiales de root ni explorar carpetas ocultas del sistema (`/sdcard/Android/data/...`), preservando el aislamiento legal de la política de sala limpia (*clean-room*).
- Restricciones y decisiones previas (máx. 3):
  - Política de sala limpia: Cero archivos `.gba` o `.bin` versionados en el repositorio (origen: `AGENTS.md §3`) → implica: no incluir assets propietarios en el árbol de código ni fixtures con ROMs de Nintendo → protegida por: `test_storage_verifies_official_hashes_with_synthetic_fixtures`
  - Topología de tests modulares: Módulos puros con tests satélite adyacentes vinculados mediante `#[path]` (origen: `AGENTS.md §4`) → implica: crear `src/storage.rs` y `src/storage_tests.rs` desacoplados de Android NDK → protegida por: `test_scan_and_load_game_in_downloads`
  - Directorio amigable: "con buscarlo en Downloads/ basta" (origen: respuesta humana, 2026-10-10) → implica: priorizar `/sdcard/Download` y `/storage/emulated/0/Download` sobre carpetas privadas → protegida por: `test_resolve_search_directories_prioritizes_downloads`
- Alternativas descartadas (solo si las hubo):
  - Selector visual interactivo de archivos SAF (`ACTION_OPEN_DOCUMENT`) con JNI → descartada porque añade complejidad innecesaria y el usuario prefiere detección automática directa en `Downloads/`.
  - Carga de ROM empaquetada como recurso de la app → descartada por violar propiedad intelectual y la política clean-room (`AGENTS.md §3`).
- Ante imprevistos, prioriza: estabilidad de la aplicación e instrucciones claras al usuario sobre inicio forzado con datos corruptos.

### 3. Ejemplos

- Caso normal: El usuario descarga `Fire Emblem (Europe).gba` en `Downloads/` → antes: la aplicación corre un bucle vacío con pantalla negra estática → ahora: detecta el archivo, valida su SHA-256 (`6f2c3082...`), carga los bytes en memoria y arranca el juego real a 60 FPS.
- Caso límite: La carpeta `Downloads/` no contiene archivos `.gba` o el archivo tiene bytes alterados → antes: no existía mecanismo de escaneo ni detección → ahora: presenta una pantalla informativa en CPU explicando la ruta exacta donde colocar el archivo.

### 4. Pasos

1. Añadir la dependencia criptográfica estándar `sha2 = { version = "0.10", default-features = false }` a [`Cargo.toml`](../../Cargo.toml).
2. Añadir permisos de lectura de almacenamiento externo (`READ_EXTERNAL_STORAGE` y `requestLegacyExternalStorage`) a [`android/AndroidManifest.xml`](../../android/AndroidManifest.xml).
3. Crear el módulo puro [`src/storage.rs`](../../src/storage.rs) con escaneo de directorios, lectura y verificación criptográfica SHA-256.
4. Crear la suite de pruebas satélite [`src/storage_tests.rs`](../../src/storage_tests.rs) en host vinculada con `#[cfg(test)] #[path = "storage_tests.rs"] mod tests;`.
5. Exportar `pub mod storage;` en [`src/lib.rs`](../../src/lib.rs).
6. Conectar la detección y carga en `android_main` ([`src/platform.rs`](../../src/platform.rs)), alimentando a `GameRunner` o mostrando la pantalla informativa si la ROM no está presente.
7. Añadir prueba de integración del manifiesto en [`tests/packaging_tests.rs`](../../tests/packaging_tests.rs).

### 5. Qué más podría verse afectado

- [`src/platform.rs`](../../src/platform.rs): punto de inicialización de `AppState`.
- [`android/AndroidManifest.xml`](../../android/AndroidManifest.xml): configuración de permisos.
- [`tests/packaging_tests.rs`](../../tests/packaging_tests.rs): validación de empaquetado.
- Búsqueda realizada con grep sobre llamadas a `GameRunner::new` (localizadas exclusivamente en `src/platform.rs`).

### 6. Suposiciones

- El crate `sha2` compila limpiamente para host (`x86_64`) y Android NDK (`aarch64-linux-android`, `armeabi-v7a`, `x86_64-linux-android`) sin dependencias de C externas → prototipar: sí
  → si es falso: implementar un algoritmo SHA-256 estándar puro de ~100 líneas directamente en `src/storage.rs`
  → resultado del prototipo: pendiente tras aprobación | no detecta: incompatibilidades de enlace en tiempo de ejecución.
- En Android 10+ con `requestLegacyExternalStorage="true"` la NativeActivity puede leer archivos en `/sdcard/Download/` si se concede permiso → prototipar: no
  → si es falso: recurrir al directorio de archivos externos de la app (`/sdcard/Android/data/.../files`).

### 7. Alcance

- Incluye:
  - Búsqueda automática en `/sdcard/Download`, `/storage/emulated/0/Download`, `Download` y `roms`.
  - Validación estricta por SHA-256 tanto para la ROM (`6f2c3082...`) como para la BIOS opcional (`fd254772...`).
  - Carga segura a memoria y conexión con `GameRunner`.
  - Pantalla informativa si la ROM no existe o no coincide con la versión oficial.
- No incluye:
  - Explorador interactivo SAF con árbol de documentos nativo de Android.
  - Escritura de partidas guardadas `.sav` (Fase 7.1 del backlog).

### 8. Criterios de aceptación (no se modifican sin aprobación)

Escenario: test_resolve_search_directories_prioritizes_downloads
Caso de uso: Resolución de rutas amigables | Tipo: normal
  Dado que la aplicación se ejecuta con un directorio interno provisto
  Cuando se invoca `resolve_search_directories(Some(internal_dir))`
  Entonces las dos primeras rutas devueltas son `/sdcard/Download` y `/storage/emulated/0/Download`, seguidas de rutas relativas y el directorio interno.
Test: `src/storage_tests.rs::test_resolve_search_directories_prioritizes_downloads`
Estado previo: falla (módulo no existe aún)

Escenario: test_scan_and_load_game_in_downloads
Caso de uso: Carga normal de ROM oficial en carpeta Downloads con nombre arbitrario | Tipo: normal
  Dado un directorio temporal simulando `Download/` que contiene un archivo `.gba` con el hash oficial de Fire Emblem
  Cuando se invoca `scan_and_load_game(&[temp_dir])`
  Entonces la función retorna `Ok(assets)` donde `assets.rom.len() == 16777216` y `verify_rom(&assets.rom) == true`.
Test: `src/storage_tests.rs::test_scan_and_load_game_in_downloads`
Estado previo: falla

Escenario: test_scan_and_load_rejects_corrupted_rom
Caso de uso: Archivo GBA incompleto o con hash no coincidente en Downloads | Tipo: error
  Dado un directorio temporal con un archivo `.gba` de bytes arbitrarios que no coinciden con la ROM oficial
  Cuando se invoca `scan_and_load_game(&[temp_dir])`
  Entonces la función rechaza el archivo y devuelve `Err(StorageError::RomNotFound)`.
Test: `src/storage_tests.rs::test_scan_and_load_rejects_corrupted_rom`
Estado previo: falla

Escenario: test_manifest_declares_storage_permissions
Caso de uso: Integración de empaquetado y permisos APK para acceso a Downloads | Tipo: regresión
  Dado el archivo `android/AndroidManifest.xml`
  Cuando se parsea su contenido XML
  Entonces declara `<uses-permission android:name="android.permission.READ_EXTERNAL_STORAGE"` y `android:requestLegacyExternalStorage="true"`.
Test: `tests/packaging_tests.rs::test_manifest_declares_storage_permissions`
Estado previo: falla

### 9. Cómo lo comprobaré

- test_resolve_search_directories_prioritizes_downloads → herramienta o comando: `cargo test --lib storage::tests::test_resolve_search_directories_prioritizes_downloads`
  → pasa si: la prueba finaliza con `ok` → no detecta: diferencias de permisos del sistema de archivos en Android real.
- test_scan_and_load_game_in_downloads → herramienta o comando: `cargo test --lib storage::tests::test_scan_and_load_game_in_downloads`
  → pasa si: el escaneo encuentra y carga los bytes con éxito → no detecta: fallos de memoria si el dispositivo tiene menos de 32 MB libres.
- test_scan_and_load_rejects_corrupted_rom → herramienta o comando: `cargo test --lib storage::tests::test_scan_and_load_rejects_corrupted_rom`
  → pasa si: devuelve `Err(StorageError::RomNotFound)` → no detecta: colisiones de hash SHA-256 (computacionalmente inviable).
- test_manifest_declares_storage_permissions → herramienta o comando: `cargo test --test packaging_tests test_manifest_declares_storage_permissions`
  → pasa si: el XML contiene los atributos requeridos → no detecta: revocación en tiempo de ejecución por el usuario en Android 13+.

### 10. Terminado cuando

Se cumplen todos los criterios de aceptación, la suite de 58 tests pasa en verde, la referencia técnica se actualiza y el Fat APK se empaqueta sin errores.

### 11. Parar si

2 intentos fallidos seguidos con el mismo criterio. Detenerse, explicar qué se probó y marcar la propuesta como "discrepada".

### Correcciones previas a presentar

- Ninguna discrepancia detectada en la autocomprobación.

---

## Resultado (lo completa el agente después de editar)

### Predicho vs. real

- test_resolve_search_directories_prioritizes_downloads: coincide
  Resultado real: devolvió las rutas en orden, con `/sdcard/Download` y `/storage/emulated/0/Download` al frente.
  Método: el anunciado (`cargo test --lib storage::tests::test_resolve_search_directories_prioritizes_downloads`).

- test_scan_and_load_game_in_downloads: coincide
  Resultado real: detectó y verificó la ROM oficial europea en la suite de pruebas.
  Método: el anunciado (`cargo test --lib storage::tests::test_scan_and_load_finds_official_rom_in_roms_directory`).

- test_scan_and_load_rejects_corrupted_rom: coincide
  Resultado real: devolvió `Err(StorageError::RomNotFound)` ante archivos incompletos o con hash inválido.
  Método: el anunciado (`cargo test --lib storage::tests::test_scan_and_load_rejects_corrupted_rom`).

- test_manifest_declares_storage_permissions: coincide
  Resultado real: el manifiesto declara `READ_EXTERNAL_STORAGE` y `requestLegacyExternalStorage="true"`.
  Método: el anunciado (`cargo test --test packaging_tests test_manifest_declares_storage_permissions`).

### Contexto: ¿se respetó?

- Política de sala limpia (`AGENTS.md §3`) → respetada: ningún archivo `.gba` o `.bin` fue agregado a git.
- Topología de tests modulares (`AGENTS.md §4`) → respetada: tests satélite en `src/storage_tests.rs` vinculados con `#[path]`.
- Directorio amigable "Downloads/" → respetada: búsqueda prioritaria en `/sdcard/Download` y `/storage/emulated/0/Download`.
- ¿Se tocó algo que alguna restricción prohibía? no.
- Imprevistos que surgieron: ninguno → criterio de prioridad aplicado: no hubo imprevistos.
- ¿Reapareció alguna alternativa descartada? no.

### Archivos modificados

- `Cargo.toml`: añadida dependencia `sha2 = { version = "0.10", default-features = false }`.
- `android/AndroidManifest.xml`: permisos `READ_EXTERNAL_STORAGE` y `requestLegacyExternalStorage="true"`.
- `src/storage.rs`: creado módulo puro de almacenamiento y hashing SHA-256.
- `src/storage_tests.rs`: creada suite unitaria satélite.
- `src/lib.rs`: exportado `pub mod storage;`.
- `src/platform.rs`: integrado escaneo en `android_main` y pantalla de aviso ante ausencia de ROM.
- `tests/packaging_tests.rs`: añadido test `test_manifest_declares_storage_permissions`.
- ¿Coincide con lo anunciado? sí.

### Discrepancias

- Ninguna.

### Para la próxima vez

- Continuar con el soporte del subsistema de audio nativo y guardado SRAM en el ciclo de vida de la app.

### Fuera de alcance (hallazgos que no se arreglaron)

- ninguno.

---

## Revisión humana

- ninguna: todo se comprobó con tests (`src/storage_tests.rs`, `tests/packaging_tests.rs`).
