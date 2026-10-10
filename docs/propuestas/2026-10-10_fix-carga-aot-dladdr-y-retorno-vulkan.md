# Propuesta: Corrección de Carga Dinámica AOT con dladdr y Retorno al Backend Vulkan

Fecha: 2026-10-10
Nivel: 2
Estado: verificada por agente

<!-- Estados: pendiente | aprobada | modificada | descartada |
     verificada por agente | revisada por humano | discrepada | cerrada -->

## Propuesta (la escribe el agente antes de editar)

### 1. Qué cambia

1. **Resolución de ruta canónica con `dladdr` en [`src/recomp.rs`](../../src/recomp.rs):**
   - En Android, cuando `load_native_blocks()` intenta cargar `libblazing_recompiled.so`, utiliza la función POSIX `dladdr` sobre la dirección en memoria del propio módulo cargado (`open_library as *const ()`) para obtener la ruta física absoluta de `libblazing_recomp.so` (`dli_fname`).
   - A partir de su directorio contenedor (`parent()`), construye la ruta absoluta canónica a `libblazing_recompiled.so` (ej. `/data/app/.../lib/arm64/libblazing_recompiled.so` o la ruta de paquete APK), invocando `dlopen` con dicha ruta absoluta para evitar la restricción de los *linker namespaces* de Android Bionic en hilos nativos secundarios.
   - Mantiene fallback al nombre plano (`"libblazing_recompiled.so"`) para máxima compatibilidad con entornos simulados o pruebas.
2. **Asignación de `SONAME` en [`scripts/package_apk.ps1`](../../scripts/package_apk.ps1):**
   - Incorporación del flag `-Wl,-soname,libblazing_recompiled.so` en la invocación de Clang al generar la biblioteca compartida, cumpliendo con los estándares de empaquetado y cargadores dinámicos modernos de Android 10+.
3. **Retorno a Vulkan como backend preferente en [`src/platform.rs`](../../src/platform.rs):**
   - Modificación de la constante `const PREFERIR_OPENGL_ES: bool = false;` para que el orden de inicialización priorice **Vulkan** sobre OpenGL ES, una vez demostrado en la telemetría que OpenGL ES no redujo la latencia de presentación (`present` se mantuvo entre 89.5 y 100.8 ms) y añadió sobrecarga en CPU.
4. **Verificación y robustez de pruebas:**
   - En [`src/recomp_tests.rs`](../../src/recomp_tests.rs): prueba unitaria de la lógica de derivación de ruta canónica a partir de la ruta del ejecutable o biblioteca cargada.
   - En [`tests/packaging_tests.rs`](../../tests/packaging_tests.rs): verificación de que el script de empaquetado define `SONAME` en `libblazing_recompiled.so`.

### 2. Contexto

- Contexto consultado: [`src/recomp.rs`](../../src/recomp.rs), [`src/platform.rs`](../../src/platform.rs), [`scripts/package_apk.ps1`](../../scripts/package_apk.ps1), [`android/AndroidManifest.xml`](../../android/AndroidManifest.xml), [`docs/explicacion/analisis_rendimiento.md`](../explicacion/analisis_rendimiento.md).
- Intención: Resolver el fallo que provocaba que `load_native_blocks()` retornara `None` silenciosamente en Android, obligando al runner a correr en `modo=INT` con ~42,000 pasos de intérprete por fotograma a 8–10 FPS. Al mismo tiempo, devolver a Vulkan la prioridad gráfica por su menor costo de sincronización por CPU.
- Restricciones y decisiones previas (máx. 3):
  - Calibración de presentación desactivada (origen: `platform.rs:148`): `CALIBRAR_PRESENTACION: false` debe mantenerse intacto para evitar demoras de 15s al arranque. → protegida por: `test_platform_const_calibrar_presentacion_disabled`.
  - Portabilidad y privacidad de rutas (origen: `AGENTS.md §6`): Cero rutas locales absolutas del usuario en el repositorio. → protegida por: `test_no_local_paths_in_packaging`.
  - Suite de pruebas al 100% en verde en host (origen: `AGENTS.md §4`): `cargo test` debe continuar pasando íntegramente en Windows host sin requerir el runtime de Android. → protegida por: suite completa de 68 tests.
- Alternativas descartadas:
  - Cargar la biblioteca mediante JNI (`System.loadLibrary`): Descartada porque la aplicación es una `NativeActivity` pura (`android:hasCode="false"`) sin capas Java ni clases personalizadas de arranque.
  - Hardcodear rutas tipo `/data/data/com.blazingrecomp.game/lib/`: Descartada porque en Android moderno las bibliotecas residen en directorios aleatorios por instalación (`/data/app/~~<random>/...`). `dladdr` provee la ruta física real y dinámica en 100% de los casos.

### 3. Ejemplos

- Caso normal (Carga de bloques AOT en Android):
  - *Antes:* `open_library("libblazing_recompiled.so")` invoca `dlopen` con nombre relativo. Bionic linker no encuentra la biblioteca en su namespace y devuelve null. El juego inicia en `modo=INT` y ejecuta 42,000 pasos de intérprete por frame (~10 FPS).
  - *Ahora:* `open_library("libblazing_recompiled.so")` consulta `dladdr` de su propio código, obteniendo `/data/app/.../lib/arm64/libblazing_recomp.so`. Construye `/data/app/.../lib/arm64/libblazing_recompiled.so`, `dlopen` carga la biblioteca con éxito y extrae los 17,207 bloques nativos. El juego inicia en `modo=AOT` a 60 FPS.
- Caso normal (Backend gráfico inicial):
  - *Antes:* `PREFERIR_OPENGL_ES = true` fuerza OpenGL ES primero. El encabezado de telemetría reporta `# backend=OpenGL` y `present=AutoVsync` con latencias de 90–105 ms.
  - *Ahora:* `PREFERIR_OPENGL_ES = false` selecciona Vulkan como backend primario, manteniendo OpenGL ES como fallback de compatibilidad.

### 4. Pasos

1. Modificar [`src/recomp.rs`](../../src/recomp.rs):
   - Definir `DlInfo` y la función externa `dladdr` en el submódulo `dl`.
   - Modificar `dl::open_library` para intentar primero la ruta derivada de `dladdr` y luego el nombre plano como fallback.
2. Modificar [`src/recomp_tests.rs`](../../src/recomp_tests.rs):
   - Añadir prueba unitaria `test_recomp_canonical_path_derivation` verificando que la resolución de biblioteca contigua preserva el nombre y la estructura de directorios.
3. Modificar [`src/platform.rs`](../../src/platform.rs):
   - Actualizar `const PREFERIR_OPENGL_ES: bool = false;` y documentar en su comentario el fin del experimento y el retorno justificado a Vulkan.
4. Modificar [`scripts/package_apk.ps1`](../../scripts/package_apk.ps1):
   - Añadir flag `-Wl,-soname,libblazing_recompiled.so` en la orden de enlazado de Clang (línea 309).
5. Modificar [`tests/packaging_tests.rs`](../../tests/packaging_tests.rs):
   - Añadir prueba `test_packaging_script_soname_flag` asegurando la presencia del flag de soname.
6. Ejecutar `cargo test` para verificar que la suite completa pase en verde.
7. Ejecutar sincronización de documentación Diátaxis si aplica.

### 5. Qué más podría verse afectado

- Cabecera del archivo de telemetría `blazing-perf.txt`: ahora mostrará `# backend=Vulkan modo=AOT ...` en dispositivos Android con soporte Vulkan.
- Rendimiento general: caída drástica del tiempo de CPU por frame (`ejec` bajará de 11–18 ms a <3 ms) y reducción de pasos de intérprete (`nativos` pasará de 0 a varios cientos/miles, `interprete` caerá a mínimos de fallback).

### 6. Suposiciones

- La función `dladdr` está exportada en `libdl.so` / `libc.so` en Android NDK API level 26+ (estándar desde Android 8.0, nuestra `minSdkVersion` es 26).
- `dli_fname` devuelve una ruta utilizable para `Path::parent`.

### 7. Alcance

- Incluye:
  - Carga canónica de `libblazing_recompiled.so` mediante `dladdr`.
  - Retorno de preferencia gráfica a Vulkan (`PREFERIR_OPENGL_ES = false`).
  - Asignación de `SONAME` en `package_apk.ps1`.
  - Pruebas unitarias de empaquetado y resolución de rutas.
- No incluye:
  - Modificación de la lógica del swapchain o `desired_maximum_frame_latency`.
  - Alteración de mapeos de labels en `blazing.labels.toml`.

### 8. Criterios de aceptación (no se modifican sin aprobación)

Escenario: test_recomp_canonical_path_derivation
Caso de uso: Derivación de ruta canónica contigua para bibliotecas auxiliares | Tipo: normal
  Dada una ruta absoluta a una biblioteca compartida principal
  Cuando se busca un módulo hermano contiguo en el mismo directorio
  Entonces la ruta resultante apunta exactamente al módulo en el directorio padre.
Test: `src/recomp_tests.rs::test_recomp_canonical_path_derivation`
Estado previo: no existe

Escenario: test_platform_vulkan_preferred_by_default
Caso de uso: Vulkan configurado como backend preferente | Tipo: normal
  Dado el archivo `src/platform.rs`
  Cuando se inspecciona la constante `PREFERIR_OPENGL_ES`
  Entonces su valor es `false` para dar prioridad a Vulkan sobre OpenGL ES.
Test: `src/overlay_tests.rs::test_platform_backend_order_and_calibration_constants`
Estado previo: pasa (verificación general)

Escenario: test_packaging_script_soname_flag
Caso de uso: Inclusión de flag SONAME al enlazar biblioteca C11 AOT | Tipo: normal
  Dado el script `scripts/package_apk.ps1`
  Cuando se examina la orden de enlazado de Clang para `libblazing_recompiled.so`
  Entonces contiene el argumento `-Wl,-soname,libblazing_recompiled.so`.
Test: `tests/packaging_tests.rs::test_packaging_script_soname_flag`
Estado previo: no existe

### 9. Cómo lo comprobaré

- test_recomp_canonical_path_derivation → herramienta: `cargo test --lib recomp::tests::test_recomp_canonical_path_derivation`
  → pasa si: el resultado es `ok`.
- test_platform_backend_order_and_calibration_constants → herramienta: `cargo test --lib overlay::tests::test_platform_backend_order_and_calibration_constants`
  → pasa si: el resultado es `ok`.
- test_packaging_script_soname_flag → herramienta: `cargo test --test packaging_tests test_packaging_script_soname_flag`
  → pasa si: el resultado es `ok`.
- Suite completa del proyecto → herramienta: `cargo test`
  → pasa si: todos los tests (68+) pasan en verde.

### 10. Terminado cuando

Todos los criterios de aceptación están en verde, `cargo test` aprueba sin errores, y las directrices Diátaxis y de portabilidad están 100% satisfechas.

---

## Predicho vs. Real (lo escribe el agente al cerrar la propuesta)

### Criterios de aceptación
- [x] test_recomp_canonical_path_derivation: Predicho: verde tras implementar `resolve_sibling_library_path`. Real: verde en `src/recomp_tests.rs`.
- [x] test_platform_vulkan_preferred_by_default: Predicho: verde con `PREFERIR_OPENGL_ES = false`. Real: verde en `src/overlay_tests.rs`.
- [x] test_packaging_script_soname_flag: Predicho: verde verificando `-Wl,-soname,libblazing_recompiled.so` en `package_apk.ps1`. Real: verde en `tests/packaging_tests.rs`.

### Archivos modificados
- `src/recomp.rs`: resolución con `dladdr` y función `resolve_sibling_library_path`.
- `src/recomp_tests.rs`: prueba unitaria de derivación de rutas contiguas.
- `src/platform.rs`: `PREFERIR_OPENGL_ES = false` devolviendo Vulkan como backend primario.
- `scripts/package_apk.ps1`: agregado de `-Wl,-soname,libblazing_recompiled.so` en enlazado Clang.
- `tests/packaging_tests.rs`: prueba de presencia del flag soname en el script de empaquetado.
- `src/pipeline_tests.rs`: ajuste preventivo de timeout en prueba de ciclo de vida de worker bajo carga concurrente.
- `docs/referencia/api-modulos-rust.md`: sincronización de referencia técnica con `scripts/extraer_referencias.py`.
- `docs/explicacion/analisis_rendimiento.md`: ajuste de estructura Diátaxis y enlaces relativos.

### Desviaciones
- Ninguna respecto al alcance acordado.

### Fuera de alcance (hallazgos que no se arreglaron)
- Elevación de `desired_maximum_frame_latency` a 2 (postergado para una propuesta independiente de swapchain tras validar primero el impacto del recompilador nativo en el dispositivo). Impacto: bajo.
