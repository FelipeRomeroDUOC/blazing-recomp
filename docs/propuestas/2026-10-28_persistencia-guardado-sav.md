# Propuesta: persistencia de partidas guardadas .sav

Fecha: 2026-10-28
Nivel: 2
Estado: cerrada

<!-- Estados: pendiente | aprobada | modificada | descartada |
     verificada por agente | revisada por humano | discrepada | cerrada -->

## Propuesta (la escribe el agente antes de editar)

### 1. Qué cambia

El juego ahora guarda y carga automáticamente el progreso de la partida en un archivo `.sav` persistente en el almacenamiento de Android, preservando tanto los guardados manuales de fin de capítulo como el autoguardado continuo de mitad de turno (datos de suspensión y semilla PRNG determinista) al salir o pausar la aplicación.

### 2. Contexto

- Contexto consultado: `AGENTS.md` §1 (Alcance MVP v0.1.0), `docs/PENDIENTES.md` §1, `src/storage.rs`, `src/runner.rs`, `src/pipeline.rs`, `src/platform.rs`, `../gba-recomp/crates/gba-core/src/backup.rs`.
- Intención: Permitir al usuario reanudar su progreso en Fire Emblem 7 entre sesiones de juego sin pérdida de datos en Android 11 a 16, respetando la idempotencia y el autoguardado continuo del juego.
- Restricciones y decisiones previas (máx. 3):
  - Inmunidad a Scoped Storage en Android 11-16: el archivo `.sav` debe alojarse prioritariamente en el directorio privado de la app (`internal_data_dir` o `external_data_dir`), donde no se requieren permisos especiales del sistema (origen: `src/storage.rs` L104-L108) → implica: no depender de permisos de `Downloads/` para persistir partidas → protegida por: "Manejo tolerante ante archivo de guardado ausente o truncado".
  - Autoguardado táctico continuo e idempotente en Flash: Fire Emblem escribe en Flash tras cada acción o cambio de turno (datos de suspensión y semilla PRNG para reproducir combates de forma idéntica) (origen: respuesta humana, 2026-10-28) → implica: no esperar a salir del juego; sincronizar con debounce de 1 a 2 segundos tras la última ráfaga de escritura Flash y vaciado inmediato en `suspend()` → protegida por: "Persistencia de autoguardado continuo con debounce".
  - Cero dependencias bloqueantes en el hilo principal de Android / renderizado: la escritura atómica en disco debe realizarse fuera del bucle de UI y de la GPU para evitar ANR y caídas de fotogramas (origen: `AGENTS.md` §1) → implica: delegar la escritura en disco al hilo de emulación `EmulationWorker` mediante sincronización no bloqueante en `EmulationBridge` → protegida por: "Sincronización de guardado multihilo en EmulationBridge".
- Alternativas descartadas:
  - Guardar únicamente al salir de la aplicación (`onDestroy`) → descartada porque Android puede matar el proceso sin avisar y porque violaría el autoguardado mid-turn de Fire Emblem.
  - Guardar sincrónicamente en disco tras cada byte individual de Flash → descartada porque degradaría severamente el rendimiento y la vida útil de la memoria flash del teléfono ante ráfagas de 4 KB.
- Ante imprevistos, prioriza: fidelidad del autoguardado en combate sobre reducción de escrituras.

### 3. Ejemplos

- Caso normal (autoguardado en combate): El jugador realiza un turno táctico o cambia a la fase enemiga en FE7 y cierra abruptamente la app → antes: los datos del turno en Flash se perdían en RAM → ahora: tras la ráfaga de escritura de la Flash, el mecanismo con debounce vacía `filesDir/fe7.sav` a disco; al reiniciar, la opción "Reanudar" (*Resume*) continúa exactamente en el mismo turno con el resultado de combate idempotente.
- Caso límite: La aplicación se pausa o pasa a segundo plano mientras se calcula el turno enemigo → antes: no existía mecanismo de sincronización con almacenamiento → ahora: el evento `suspend` en `AppState` solicita a `EmulationWorker` vaciar el búfer sucio inmediatamente a disco sin bloquear la interfaz.

### 4. Pasos

1. Crear en `src/storage.rs` las funciones `resolve_save_path`, `load_save_file` y `write_save_file_atomic`, con soporte de reemplazo atómico mediante archivo temporal `.tmp`.
2. Añadir en `src/runner.rs` los métodos `load_save_data`, `save_data` y detección de datos modificados (*dirty*) mediante cálculo hash del búfer Flash.
3. Conectar en `src/pipeline.rs` el `EmulationBridge` y `EmulationWorker` con temporizador de debounce (1,5 segundos tras última escritura Flash) para procesar solicitudes de persistencia (`request_save`) y ejecutar el vaciado a disco en el hilo de emulación.
4. Integrar en `src/platform.rs` la lectura de `.sav` durante el arranque (`scan_and_load_game`) y la solicitud de persistencia inmediata al suspender la aplicación (`AppState::suspend`).
5. Añadir la suite de tests unitarios e integración en `src/storage_tests.rs` y `src/pipeline_tests.rs` cubriendo lectura, escritura atómica, tolerancia a datos truncados, autoguardado con debounce y sincronización multihilo.

### 5. Qué más podría verse afectado

- `src/storage.rs` (añade funciones públicas de guardado; callers: `src/platform.rs`).
- `src/runner.rs` (añade métodos a `GameRunner`; callers: `src/pipeline.rs`).
- `src/pipeline.rs` (ajusta inicialización y bucle de `EmulationWorker` y `EmulationBridge`).
- `src/platform.rs` (ajusta `AppState::load` y `AppState::suspend`).
Búsqueda realizada: referencias a `scan_and_load_game`, `AppState::load`, `AppState::suspend` y `GameRunner::new`.

### 6. Suposiciones

- La ROM oficial europea de FE7 utiliza memoria de guardado de 64 KB compatible con `gba_core::backup::BackupKind::Sram` o `Flash64` tanto para ranuras de partida como para autoguardado de suspensión → prototipar: sí → si es falso: adaptar la detección para admitir Flash de 128 KB → resultado del prototipo: verificado; la ROM oficial europea contiene la firma `SRAM_F_V102`, clasificándose como `BackupKind::Sram` (64 KB), por lo que `runner.rs` soportará tanto `Sram` como `Flash64/128` transparentemente | no detecta: fallos en versiones hackeadas sin identificador.
- La función `std::fs::rename` es atómica a nivel de sistema de archivos en Android dentro del mismo punto de montaje (`filesDir`) → prototipar: no → si es falso: recurrir a copia con respaldo previo `.bak`.
- La comprobación de modificación (*dirty*) se evalúa con una cadencia desacoplada (debounce de 1,5 segundos tras escrituras o en pausa) y su coste es inferior a 0.05 ms por evaluación sin sobrecargar el bucle continuo → prototipar: sí → si es falso: añadir una bandera booleana en `gba-core` → resultado del prototipo: verificado; la comprobación en búfer de 64 KB consume < 0.01 ms en Rust, con impacto nulo en el hilo de emulación | no detecta: contención de CPU extrema.

### 7. Alcance

- Incluye:
  - Carga automática de partidas `.sav` existentes al inicio del juego.
  - Guardado atómico en almacenamiento interno/externo privado de Android.
  - Persistencia de autoguardado de mitad de combate (*suspend data*) mediante debounce de 1,5 segundos.
  - Sincronización asíncrona de guardado en el hilo de trabajo durante transiciones de ciclo de vida (`suspend`).
  - Tests unitarios de resolución de rutas, lectura, archivos truncados, escritura atómica, debounce y sincronización multihilo.
- No incluye:
  - Selector interactivo de archivos de guardado en la interfaz gráfica (UI para importar/exportar SAF).
  - Savestates (congelamiento de estado completo de memoria de emulación).

### 8. Criterios de aceptación (no se modifican sin aprobación)

Escenario: Carga de archivo de guardado existente en memoria Flash
Caso de uso: Reanudación de partida previa o de turno suspendido al iniciar el juego | Tipo: normal
  Dado un archivo de guardado `fe7.sav` válido de 65.536 bytes en el almacenamiento
  Cuando se inicializa la sesión de juego con `load_save_data`
  Entonces los datos de la memoria Flash de la máquina coinciden exactamente con los 65.536 bytes del archivo.
Test: `src/storage_tests.rs::test_load_and_restore_save_data`
Estado previo: falla

Escenario: Escritura atómica de archivo de guardado sin corrupción
Caso de uso: Persistencia segura en disco ante cortes de energía o caídas | Tipo: límite
  Dado un búfer de datos de guardado modificado de 65.536 bytes
  Cuando se invoca `write_save_file_atomic`
  Entonces el archivo de destino contiene exactamente los 65.536 bytes y el archivo temporal `.tmp` queda eliminado.
Test: `src/storage_tests.rs::test_write_save_file_atomic_success_and_cleanup`
Estado previo: falla

Escenario: Manejo tolerante ante archivo de guardado ausente o truncado
Caso de uso: Primer arranque del juego o recuperación tras fallo de escritura previa | Tipo: error
  Dado que no existe ningún archivo `.sav` o el archivo mide menos de 65.536 bytes (archivo truncado a 1.024 bytes)
  Cuando se intenta cargar la partida con `load_save_file`
  Entonces la función retorna `None` sin causar pánico y la memoria Flash inicia limpia con valores 0xFF.
Test: `src/storage_tests.rs::test_load_save_file_missing_or_truncated`
Estado previo: falla

Escenario: Persistencia de autoguardado continuo con debounce
Caso de uso: Autoguardado mid-turn en combate sin degradar la memoria flash | Tipo: normal
  Dado un `EmulationWorker` ejecutando la lógica de combate con escrituras continuas en Flash
  Cuando cesa la ráfaga de escrituras y transcurre la ventana de debounce (1,5 segundos)
  Entonces el búfer de Flash modificado se vacía a disco en `fe7.sav` preservando el estado de suspensión.
Test: `src/pipeline_tests.rs::test_emulation_worker_autosave_debounce`
Estado previo: falla

Escenario: Sincronización de guardado multihilo en EmulationBridge
Caso de uso: Solicitud de persistencia de partida sin bloquear el hilo principal ni la UI | Tipo: límite
  Dado un `EmulationWorker` en ejecución con datos de guardado modificados
  Cuando el hilo principal solicita persistencia con `bridge.request_save()`
  Entonces el hilo de trabajo realiza la escritura atómica en disco y notifica la confirmación sin bloquear el hilo invocador.
Test: `src/pipeline_tests.rs::test_emulation_worker_save_synchronization`
Estado previo: falla

Escenario: Preservación de búsqueda de ROM y BIOS oficiales
Caso de uso: Regresión del sistema de carga de assets del juego | Tipo: regresión
  Dado el conjunto de directorios candidatos estándar
  Cuando se ejecuta `scan_and_load_game`
  Entonces la detección y validación criptográfica de ROM y BIOS mantiene su comportamiento intacto.
Test: `src/storage_tests.rs::test_verify_rom_size_and_signature_rejection`
Estado previo: pasa (regresión)

### 9. Cómo lo comprobaré

- Carga de archivo de guardado existente en memoria Flash → herramienta o comando: `cargo test --lib test_load_and_restore_save_data` → pasa si: el test finaliza en ok con 65.536 bytes idénticos → no detecta: desajustes en el hardware real Android.
- Escritura atómica de archivo de guardado sin corrupción → herramienta o comando: `cargo test --lib test_write_save_file_atomic_success_and_cleanup` → pasa si: el archivo temporal desaparece y el `.sav` final tiene contenido exacto → no detecta: fallos de disco por cuota de almacenamiento llena a nivel de kernel.
- Manejo tolerante ante archivo de guardado ausente o truncado → herramienta o comando: `cargo test --lib test_load_save_file_missing_or_truncated` → pasa si: devuelve `None` limpiamente sin pánico ante archivos ausentes o parciales → no detecta: corrupciones en bloques lógicos dentro de un archivo de tamaño correcto.
- Persistencia de autoguardado continuo con debounce → herramienta o comando: `cargo test --lib test_emulation_worker_autosave_debounce` → pasa si: el archivo se actualiza tras la ventana de inactividad de 1,5 s sin disparar escrituras por cada micro-escritura individual → no detecta: latencia de I/O en medios de almacenamiento externos USB lentos.
- Sincronización de guardado multihilo en EmulationBridge → herramienta o comando: `cargo test --lib test_emulation_worker_save_synchronization` → pasa si: el worker completa la escritura a disco y el canal confirma el guardado en menos de 100 ms → no detecta: latencias extremas por I/O bajo bloqueo de flash en dispositivos físicos lentos.
- Preservación de búsqueda de ROM y BIOS oficiales → herramienta o comando: `cargo test --lib test_verify_rom_size_and_signature_rejection` → pasa si: los tests de regresión pasan con código de salida 0 → no detecta: cambios de rutas en dispositivos Android no emulados.

### 10. Terminado cuando

Se cumplen todos los criterios de aceptación y los tests pasan en verde.

### 11. Parar si

2 intentos fallidos seguidos con el mismo criterio.

### Cambios tras prototipos (solo si el plan cambió)

- Antes: se asumía soporte exclusivo de `BackupKind::Flash64` para FE7.
- Ahora: `load_save_data` y `save_data` soportan unívocamente `BackupKind::Sram` y `BackupKind::Flash64` / `Flash128` de 65.536 bytes.
- Por qué: el prototipo con la ROM oficial europea de FE7 (`roms/1575 - Fire Emblem (E)(Independent).gba`) confirmó la firma `SRAM_F_V102`, que `gba-core` mapea a `BackupKind::Sram` (64 KB).

---

## Resultado (lo completa el agente después de editar)

### Predicho vs. real

- Carga de archivo de guardado existente en memoria Flash: coincide
  Resultado real: pasa `test_load_and_restore_save_data` (65.536 bytes idénticos restaurados en memoria).
  Método: el anunciado
- Escritura atómica de archivo de guardado sin corrupción: coincide
  Resultado real: pasa `test_write_save_file_atomic_success_and_cleanup` (archivo .sav final íntegro y .tmp eliminado).
  Método: el anunciado
- Manejo tolerante ante archivo de guardado ausente o truncado: coincide
  Resultado real: pasa `test_load_save_file_missing_or_truncated` (retorna None limpiamente sin pánico ante archivos inexistentes y < 64 KB).
  Método: el anunciado
- Persistencia de autoguardado continuo con debounce: coincide
  Resultado real: pasa `test_emulation_worker_autosave_debounce` (vaciado a disco tras debounce de inactividad).
  Método: el anunciado
- Sincronización de guardado multihilo en EmulationBridge: coincide
  Resultado real: pasa `test_emulation_worker_save_synchronization` (solicitud asíncrona no bloqueante).
  Método: el anunciado
- Preservación de búsqueda de ROM y BIOS oficiales: coincide
  Resultado real: pasa `test_verify_rom_size_and_signature_rejection`.
  Método: el anunciado

### Contexto: ¿se respetó?

- Inmunidad a Scoped Storage en Android 11-16 → respetada
- Autoguardado táctico continuo e idempotente en Flash → respetada
- Cero dependencias bloqueantes en el hilo principal de Android / renderizado → respetada
- ¿Se tocó algo que alguna restricción prohibía? no
- Imprevistos que surgieron: en host Windows, `std::env::temp_dir()` contiene el alias 8.3 `FELIPE~1`, provocando fallo de permisos (`os error 5`) en `create_dir_all`; se aisló el directorio de tests bajo `target/` → criterio de prioridad aplicado: sí
- ¿Reapareció alguna alternativa descartada? no

### Archivos modificados

- `src/storage.rs`
- `src/storage_tests.rs`
- `src/runner.rs`
- `src/pipeline.rs`
- `src/pipeline_tests.rs`
- `src/platform.rs`
¿Coincide con lo anunciado? sí

### Discrepancias

Ninguna.

### Para la próxima vez

En pruebas unitarias que creen carpetas temporales en host Windows, utilizar rutas relativas bajo `target/` en vez de `std::env::temp_dir()` para evitar colisiones con alias 8.3 (`FELIPE~1`).

### Fuera de alcance (hallazgos que no se arreglaron)

- 7 pruebas unitarias preexistentes de banderas en `audio_tests.rs`, `ppu_nativa_tests.rs` y `storage_tests.rs` fallan en host Windows por la resolución de `std::env::temp_dir()` con alias 8.3 (`FELIPE~1`). | impacto: bajo

---

## Revisión humana

- Carga y persistencia de partida en dispositivo real Android: ✔ (Probado y validado en dispositivo real)
- Detectado solo por el humano: Ninguno (comportamiento idéntico al esperado)
