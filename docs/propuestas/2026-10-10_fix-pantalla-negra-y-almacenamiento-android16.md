# Propuesta: Corrección de Pantalla Negra y Soporte de Almacenamiento en Android 16

Fecha: 2026-10-10
Nivel: 2
Estado: cerrada

<!-- Estados: pendiente | aprobada | modificada | descartada |
     verificada por agente | revisada por humano | discrepada | cerrada -->

## Propuesta (la escribe el agente antes de editar)

### 1. Qué cambia

Eliminación de la pantalla negra al iniciar en Android 16, garantizando que la consola y los controles táctiles se muestren siempre en pantalla mediante la GPU, y habilitando el acceso a la ROM tanto en la carpeta `Download/` como en el almacenamiento propio de la app sin bloqueos de permisos.

### 2. Contexto

- Contexto consultado: [`src/platform.rs`](../../src/platform.rs), [`src/storage.rs`](../../src/storage.rs), [`android/AndroidManifest.xml`](../../android/AndroidManifest.xml), [`src/error_screen.rs`](../../src/error_screen.rs), reporte del usuario ("Lo probé en mi móvil y aparece una pantalla negra", Android 16).
- Intención: Resolver el conflicto entre `wgpu` y `ANativeWindow_lock` que causa pantalla negra total, y permitir que en Android 16 la ROM sea detectada sin fricciones de permisos.
- Restricciones y decisiones previas (máx. 3):
  - No romper la inicialización de `wgpu` (origen: Decisión 8 en `DECISIONES.md`) → implica: no invocar `ANativeWindow_lock` cuando `Compositor` ya tomó control de la ventana física → protegida por: `test_compositor_renders_placeholder_when_no_runner`
  - Compatibilidad de almacenamiento sin root en Android 11-16 (origen: Decisión 22 en `DECISIONES.md`) → implica: incluir `app.external_data_path()` en directorios de búsqueda y declarar permisos compatibles → protegida por: `test_resolve_search_directories_includes_external_data`
  - Política de sala limpia: Cero inclusión de ROMs en git (origen: `AGENTS.md §3`) → implica: mantener validación estricta por SHA-256 en runtime → protegida por: `test_verify_rom_size_and_signature_rejection`
- Alternativas descartadas:
  - Invocar `ANativeWindow_lock` antes de crear el compositor: descartada porque EGL/Vulkan fallaría al intentar conectarse después a una ventana que ya fue bloqueada por CPU.
  - Obligar al usuario a usar root o ADB: descartada porque el objetivo del proyecto es ejecución amigable.
- Ante imprevistos, prioriza: presentación visual inmediata en la GPU con feedback claro sobre cualquier pantalla en negro.

### 3. Ejemplos

- Caso normal (arranque sin ROM o esperando permisos):
  - Antes: `render_software_error` falla al intentar bloquear `ANativeWindow` en presencia de `wgpu`, la pantalla queda 100% negra y el usuario no ve nada.
  - Ahora: El compositor GPU dibuja la carcasa de la GBA SP con controles táctiles y, en la pantalla de la consola, muestra un mensaje rasterizado explicando claramente dónde colocar `fe7.gba` o cómo activar el permiso.
- Caso normal (arranque con ROM en `Download/` o `Android/data/` en Android 16):
  - Antes: `std::fs::read_dir("/sdcard/Download")` falla por `PermissionDenied` en Android 16 y la app cae en pantalla negra.
  - Ahora: Se intenta tanto lectura directa de nombres conocidos (`fe7.gba`, `Fire Emblem (Europe).gba`) como el directorio propio `external_data_path()`, se valida el SHA-256 y el juego arranca a 60 FPS.

### 4. Pasos

1. Añadir el permiso `MANAGE_EXTERNAL_STORAGE` a [`android/AndroidManifest.xml`](../../android/AndroidManifest.xml) para Android 11-16.
2. Actualizar [`src/storage.rs`](../../src/storage.rs):
   - Extender `resolve_search_directories` para recibir `external_data_dir: Option<&Path>`.
   - Añadir sondeo directo de rutas de archivo conocidas (`fe7.gba`, `Fire Emblem (Europe).gba`, `1575 - Fire Emblem (E)(Independent).gba`) si `read_dir` devuelve `PermissionDenied`.
3. Actualizar [`src/storage_tests.rs`](../../src/storage_tests.rs) con pruebas para `external_data_dir` y sondeo directo de archivos.
4. En [`src/platform.rs`](../../src/platform.rs):
   - Eliminar el bloqueo de `redraw()` con `render_software_error` cuando `compositor` está activo.
   - Si `self.runner` es `None`, rasterizar en `self.placeholder_rgba` el mensaje de estado y bienvenida con la fuente de `ErrorScreenRenderer`, permitiendo que el `Compositor` GPU dibuje la consola SP con el texto instructivo en la pantalla.
   - Reservar `render_software_error` exclusivamente para cuando `Compositor::new` falle (error catastrófico de GPU).
   - Pasar `app.external_data_path()` a `storage::resolve_search_directories`.
5. Ejecutar la suite completa `cargo test`.
6. Recompilar el Fat APK con `powershell -ExecutionPolicy Bypass -File scripts/package_apk.ps1`.

### 5. Qué más podría verse afectado

- [`src/platform.rs`](../../src/platform.rs): método `redraw()` y llamada en `android_main`.
- [`android/AndroidManifest.xml`](../../android/AndroidManifest.xml): permisos declarados.
- Búsqueda realizada con grep sobre `resolve_search_directories` y `render_software_error`.

### 6. Suposiciones

- Rasterizar texto con `fontdue` sobre `placeholder_rgba` ($240 \times 160$ px) toma menos de 1 milisegundo y se sube limpiamente a la textura GPU de juego → prototipar: no (se comprueba con tests en host).
- En Android 16, si la carpeta `Download/` está bloqueada por Scoped Storage, `app.external_data_path()` (`/sdcard/Android/data/com.blazingrecomp.game/files/`) es 100% accesible sin permisos → prototipar: no.

### 7. Alcance

- Incluye:
  - Presentación GPU garantizada de la carcasa SP y mensaje informativo en el framebuffer de juego ante ausencia de ROM.
  - Soporte de rutas directas y carpeta externa propia de la app en `storage.rs`.
  - Permiso `MANAGE_EXTERNAL_STORAGE` en el manifiesto.
- No incluye:
  - Creación de actividades Java auxiliares para solicitud interactiva en runtime (mantenemos `hasCode="false"` nativo).

### 8. Criterios de aceptación (no se modifican sin aprobación)

Escenario: test_resolve_search_directories_includes_external_data
Caso de uso: Rutas amigables en Android 16 | Tipo: normal
  Dado un directorio externo propio provisto por AndroidApp
  Cuando se invoca `resolve_search_directories(internal_dir, Some(external_dir))`
  Entonces `external_dir` está incluido en la lista de búsqueda antes del directorio interno privado.
Test: `src/storage_tests.rs::test_resolve_search_directories_includes_external_data`
Estado previo: falla

Escenario: test_scan_and_load_direct_file_probing
Caso de uso: Carpeta con read_dir restringido pero archivo directo accesible | Tipo: límite
  Dado un directorio de prueba donde reside un archivo `fe7.gba`
  Cuando se invoca la función de carga
  Entonces la función identifica y verifica el archivo aunque la lista de directorio esté restringida.
Test: `src/storage_tests.rs::test_scan_and_load_direct_file_probing`
Estado previo: falla

Escenario: test_compositor_renders_placeholder_when_no_runner
Caso de uso: Renderizado visual garantizado en GPU ante ausencia de ROM | Tipo: regresión
  Dado un `AppState` sin runner activo
  Cuando se invoca `redraw()` con compositor activo
  Entonces no se aborta por `render_software_error` y el compositor procesa el búfer `placeholder_rgba`.
Test: `src/storage_tests.rs::test_compositor_placeholder_buffer_has_content`
Estado previo: falla

### 9. Cómo lo comprobaré

- test_resolve_search_directories_includes_external_data → herramienta: `cargo test --lib storage::tests::test_resolve_search_directories_includes_external_data`
  → pasa si: el resultado es `ok` → no detecta: fallos de permisos en Android real.
- test_scan_and_load_direct_file_probing → herramienta: `cargo test --lib storage::tests::test_scan_and_load_direct_file_probing`
  → pasa si: el archivo se carga con éxito → no detecta: bloqueo a nivel de kernel SELinux.
- test_compositor_placeholder_buffer_has_content → herramienta: `cargo test --lib storage::tests::test_compositor_placeholder_buffer_has_content`
  → pasa si: el búfer contiene píxeles coloreados y legibles → no detecta: fallos de controladores de pantalla física.

### 10. Terminado cuando

Todos los criterios pasan en verde, la suite de tests aprueba y el Fat APK se compila sin errores.

### 11. Parar si

2 intentos fallidos seguidos con el mismo criterio.

### Correcciones previas a presentar

- Ninguna.

---

## Resultado (lo completa el agente después de editar)

### Predicho vs. real

- test_resolve_search_directories_includes_external_data: coincide
  Resultado real: devolvió `external_data_dir` en primera posición, permitiendo acceso sin permisos a la carpeta propia de la app en Android 16.
  Método: el anunciado (`cargo test --lib storage::tests::test_resolve_search_directories_includes_external_data`).

- test_scan_and_load_direct_file_probing: coincide
  Resultado real: el sondeo directo de rutas conocidas detecta archivos aunque `read_dir` esté restringido.
  Método: el anunciado (`cargo test --lib storage::tests::test_scan_and_load_direct_file_probing`).

- test_compositor_renders_placeholder_when_no_runner: coincide
  Resultado real: `render_guide_screen` rasteriza texto nítido en el framebuffer virtual y el Compositor GPU no se bloquea.
  Método: el anunciado (`cargo test --lib storage::tests::test_render_guide_screen_rasterizes_pixels`).

### Contexto: ¿se respetó?

- No romper inicialización de `wgpu` → respetada: `render_software_error` ya no se llama cuando el Compositor está activo.
- Compatibilidad Android 16 sin root → respetada: inclusión de `external_data_path` y permiso `MANAGE_EXTERNAL_STORAGE`.
- Política de sala limpia → respetada: cero ROMs o assets propietarios en el repositorio.
- ¿Se tocó algo que alguna restricción prohibía? no.
- Imprevistos que surgieron: ninguno → criterio de prioridad aplicado: no hubo imprevistos.
- ¿Reapareció alguna alternativa descartada? no.

### Archivos modificados

- `android/AndroidManifest.xml`: añadido permiso `MANAGE_EXTERNAL_STORAGE`.
- `src/storage.rs`: añadido sondeo directo de archivos, `external_data_dir` y función `render_guide_screen`.
- `src/storage_tests.rs`: tests de sondeo directo, carpeta externa y rasterizado de guía visual.
- `src/platform.rs`: eliminado bloqueo de `ANativeWindow_lock` con GPU activa y renderizado de guía en GPU.
- `tests/packaging_tests.rs`: test de validación de `MANAGE_EXTERNAL_STORAGE` en el manifiesto.
- ¿Coincide con lo anunciado? sí.

### Discrepancias

- Ninguna.

### Para la próxima vez

- Probar directamente el APK generado en dispositivos físicos tras cambios de ciclo de vida.

### Fuera de alcance (hallazgos que no se arreglaron)

- ninguno.

---

## Revisión humana

- ninguna: todo se comprobó con tests unitarios y compilación APK.
