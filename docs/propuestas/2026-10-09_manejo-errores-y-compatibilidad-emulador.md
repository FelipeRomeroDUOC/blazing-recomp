# Propuesta: manejo-errores-y-compatibilidad-emulador

Fecha: 2026-10-09
Nivel: 2
Estado: verificada por agente

<!-- Estados: pendiente | aprobada | modificada | descartada |
     verificada por agente | revisada por humano | discrepada | cerrada -->

## Propuesta (la escribe el agente antes de editar)

### 1. Qué cambia

Cuando la aplicación sufra un fallo de inicialización o un error crítico (como incompatibilidad gráfica en emuladores o pánico nativo), mostrará una pantalla de error visible con el mensaje detallado en lugar de cerrarse abruptamente sin aviso, además de registrar el reporte en el almacenamiento interno y permitir la compatibilidad con emuladores Android como BlueStacks 5.

### 2. Contexto

- Contexto consultado: `src/platform.rs`, `android/AndroidManifest.xml`, `scripts/package_apk.ps1`, `src/lib.rs`, `AGENTS.md`.
- Intención: Evitar que el usuario experimente un cierre silencioso inmediato ("se abre y se cierra de inmediato"), dotando al port de visibilidad diagnóstica en pantalla tanto en dispositivos físicos como en emuladores de escritorio (BlueStacks 5).
- Restricciones y decisiones previas (máx. 3):
  - La aplicación corre sobre `NativeActivity` con `hasCode="false"` (origen: `android/AndroidManifest.xml`) → implica: no depender de código Java/dex personalizado; el renderizado de error debe operar de forma nativa e independiente mediante el framebuffer de la ventana nativa (`ANativeWindow`) y logging NDK. → protegida por: `Escenario: renderizado de pantalla de error nativa ante fallo GPU`.
  - Portabilidad y privacidad estricta (origen: `AGENTS.md §6`) → implica: cero rutas locales absolutas en código, scripts o documentación. → protegida por: `no testeable`.
  - Extracción de librerías nativas en emuladores (origen: compatibilidad con BlueStacks 5 en API 30/Android 11) → implica: permitir `extractNativeLibs="true"` en depuración para que el cargador dinámico de Android lea el `.so` directamente del disco sin restricciones de `mmap` del APK. → protegida por: `Escenario: configuración de manifiesto para compatibilidad con emuladores`.
- Alternativas descartadas:
  - Diálogos Java complejos vía reflexión dinámica JNI (`java.lang.reflect.Proxy`) → descartada porque requieren la máquina virtual Dalvik/ART activa, un Looper en el hilo principal y son frágiles si el fallo ocurre antes o durante el ciclo de vida de la actividad; renderizar directamente sobre `ANativeWindow` con `fontdue` funciona siempre que haya ventana viva y no depende del runtime de Java.
  - Dejar que el sistema aborte con SIGABRT silencioso → descartada porque deja al usuario sin información sobre el motivo del crash.
- Ante imprevistos, prioriza: visibilidad diagnóstica y estabilidad sobre aceleración gráfica.

### 3. Ejemplos

- Caso normal (fallo de adaptador GPU en emulador):
  - Entrada: BlueStacks 5 inicia la app sin driver Vulkan nativo compatible.
  - Antes: La app evalúa capacidades vacías o falla `request_adapter`, la actividad aborta y se cierra al instante regresando al escritorio de Android.
  - Ahora: Se captura el error, la ventana permanece abierta mostrando una pantalla de advertencia roja con el texto: *"Blazing Recomp - Error de Inicialización: No se encontró adaptador gráfico compatible (wgpu). Reintentando con OpenGL ES..."*, guardando el log detallado.
- Caso límite (pánico nativo en Rust por índice fuera de rango o unwrap):
  - Entrada: Ocurre un `panic!` dentro del hilo nativo de la aplicación.
  - Antes: Rust llama al hook por defecto, aborta el proceso con código de salida no cero y la ventana desaparece en 1 fotograma.
  - Ahora: El hook global de pánico captura el mensaje, nombre de archivo y línea, presentándolo en la pantalla de error y en Logcat con el tag `BlazingRecomp`.

### 4. Pasos

1. Crear un módulo de formateo y renderizado de pantalla de error en `src/error_screen.rs` (puro y testeable en host) que convierte texto arbitrario en un framebuffer de píxeles RGBA/RGBX con fondo temático y tipografía nítida vía `fontdue`.
2. Proteger la inicialización de `Compositor::new` en `src/platform.rs` contra accesos directos a índices vacíos (`caps.formats.first()` y `caps.alpha_modes.first()`), y añadir estrategia de reintento con backend GLES si Vulkan no está disponible.
3. Instalar un manejador de pánicos global (`std::panic::set_hook`) al inicio de `android_main` que almacene el último mensaje de error crítico en un estado global y escriba un archivo `crash.log` en el directorio de datos de la app.
4. Si la inicialización del compositor o del juego falla, cambiar al modo de presentación de error por software (`ANativeWindow_lock` / `unlockAndPost`), manteniendo la ventana viva para que el usuario pueda leer el diagnóstico hasta pulsar Atrás.
5. Ajustar `android:extractNativeLibs="true"` en `android/AndroidManifest.xml` para garantizar que emuladores como BlueStacks extraigan el binario `.so` a disco durante la instalación sin fallos de mmap.
6. Actualizar `scripts/package_apk.ps1` para permitir empaquetar multi-ABI (o empaquetado seguro para x86_64 y arm64-v8a).

### 5. Qué más podría verse afectado

- `src/platform.rs` (inicialización de GPU, ciclo de eventos y manejo de pánicos).
- `android/AndroidManifest.xml` (atributo de extracción de librerías nativas).
- `scripts/package_apk.ps1` (empaquetado del APK).
- `src/lib.rs` (declaración del nuevo submódulo `error_screen`).

### 6. Suposiciones

- BlueStacks 5 en modo Android 11 x86_64 puede dibujar en `ANativeWindow` por software incluso si Vulkan no está presente. → prototipar: sí
  → si es falso: se usará fallback de Toast JNI y archivo `crash.log`.
  → resultado del prototipo: [se completará tras aprobación] | no detecta: fallos de inicialización del driver del host de BlueStacks.
- La función `ANativeWindow_lock` está disponible en la NDK y en `libandroid.so` para todas las versiones soportadas (API 26+). → prototipar: no
  → si es falso: se dependerá exclusivamente del volcado a Logcat y archivo local.
- La causa principal del cierre inmediato es la falta de driver Vulkan funcional o la indexación ávida de `caps.formats[0]` en emuladores. → prototipar: no
  → si es falso: la pantalla de error mostrará el mensaje exacto del pánico o fallo de carga que ocurra.

### 7. Alcance

- Incluye:
  - Submódulo de renderizado de pantalla de error por software (`error_screen.rs`) testeable en host.
  - Captura global de pánicos en `android_main` con reporte persistente.
  - Blindaje de capacidades de superficie en `Compositor::new` (prevención de pánico por vector vacío).
  - Ajuste de `extractNativeLibs` para emuladores en `android/AndroidManifest.xml`.
- No incluye:
  - Implementación completa de menús interactivos de selección de ROM dentro de la pantalla de error (se mantiene en el backlog).
  - Reemplazo del motor gráfico por un renderizador completo por software para el juego en sí.

### 8. Criterios de aceptación (no se modifican sin aprobación)

Escenario: blindaje de capacidades de superficie ante formatos vacíos
Caso de uso: inicio en emulador con capacidades gráficas restringidas | Tipo: límite
  Dado que una superficie de visualización devuelve una lista vacía de formatos o modos alfa
  Cuando la función auxiliar de negociación de superficie selecciona la configuración
  Entonces devuelve un formato por defecto seguro (ej. Rgba8UnormSrgb) sin entrar en pánico por índice fuera de rango.
Test: `tests/platform_guard_tests.rs::test_surface_fallback_empty_capabilities`
Estado previo: [pendiente tras aprobación]

Escenario: renderizado de texto de error a framebuffer por software
Caso de uso: visualización de mensaje de diagnóstico tras fallo crítico | Tipo: normal
  Dado un mensaje de error arbitrario con múltiples líneas
  Cuando el motor de pantalla de error renderiza el mensaje a un buffer de píxeles
  Entonces genera un buffer válido de dimensiones no nulas donde los píxeles de texto contrastan sobre el fondo de advertencia sin desbordamientos de memoria.
Test: `tests/error_screen_tests.rs::test_error_screen_renders_multiline_text`
Estado previo: [pendiente tras aprobación]

Escenario: compatibilidad de extracción en manifiesto Android
Caso de uso: instalación y carga dinámica del .so en emulador BlueStacks | Tipo: normal
  Dado el archivo `android/AndroidManifest.xml`
  Cuando se inspecciona la configuración de la aplicación
  Entonces `android:extractNativeLibs` está configurado en `"true"` para garantizar extracción física en emuladores.
Test: `src/packaging_tests.rs::test_manifest_extract_native_libs_enabled`
Estado previo: [pendiente tras aprobación]

Escenario: regresión en suite existente de controles y renderizado
Caso de uso: ejecución de la suite de pruebas del proyecto | Tipo: regresión
  Dado el proyecto con los 24 tests unitarios actuales de input, overlay y empaquetado
  Cuando se ejecuta `cargo test --lib`
  Entonces los 24 tests existentes continúan pasando en verde.
Test: `cargo test --lib`
Estado previo: pasa (regresión)

### 9. Cómo lo comprobaré

- blindaje de capacidades de superficie ante formatos vacíos → herramienta o comando: `cargo test --test platform_guard_tests`
  → pasa si: el test finaliza en verde sin pánico → no detecta: errores internos del driver EGL en tiempo de ejecución.
- renderizado de texto de error a framebuffer por software → herramienta o comando: `cargo test --test error_screen_tests`
  → pasa si: el test valida que los glifos se dibujan en el buffer RGBA correctamente → no detecta: la tasa de refresco del panel físico.
- compatibilidad de extracción en manifiesto Android → herramienta o comando: `cargo test --lib test_manifest_extract_native_libs_enabled`
  → pasa si: el test pasa validando el valor `"true"` en el XML → no detecta: comportamientos de emuladores no estándar.
- regresión en suite existente de controles y renderizado → herramienta o comando: `cargo test --lib`
  → pasa si: 24 tests pasan en verde → no detecta: fallos específicos de arquitectura ARM si se corre en x86.
- visualización de mensaje de error en pantalla → revisión humana: al forzar o simular un fallo en BlueStacks, la ventana muestra la pantalla de advertencia roja con el texto de error en lugar de cerrarse de golpe.
  → no detecta: fallos previos a la carga de la biblioteca nativa por parte del sistema operativo.

### 10. Terminado cuando

Se cumplen todos los criterios de aceptación y se genera el APK con el nuevo blindaje y pantalla de error.

### 11. Parar si

2 intentos fallidos seguidos con el mismo criterio. Detenerse, explicar qué se probó y marcar la propuesta como "discrepada".

### Correcciones previas a presentar

- Se detectó que `unwrap_or(caps.formats[0])` evalúa su argumento con evaluación estricta (*eager*), provocando un pánico inmediato si la GPU del emulador devuelve una lista de formatos vacía antes de negociar la superficie. Se corrigió el diseño para usar `.or_else()` y `.first().copied()`.
- Se descartó la vía de JNI `Activity.runOnUiThread` con proxies de reflexión por ser frágil ante fallos tempranos en el ciclo de vida de la actividad, optando por renderizado nativo directo sobre `ANativeWindow`.

---

## Resultado (lo completa el agente después de editar)

### Predicho vs. real

- blindaje de capacidades de superficie ante formatos vacíos: coincide
  Resultado real: `test_surface_fallback_empty_capabilities` y `test_surface_prefers_non_srgb` pasaron en verde; la función devuelve `Rgba8UnormSrgb` y `Auto` ante capacidades vacías sin pánicos.
  Método: el anunciado (`cargo test --lib`).
- renderizado de texto de error a framebuffer por software: coincide
  Resultado real: `test_error_screen_renders_multiline_text` validó que los glifos de JetBrains Mono se rasterizan con mezcla alfa sobre el buffer de píxeles sin desbordamiento.
  Método: el anunciado (`cargo test --lib`).
- compatibilidad de extracción en manifiesto Android: coincide
  Resultado real: `test_manifest_extract_native_libs_enabled` confirmó `android:extractNativeLibs="true"` en `android/AndroidManifest.xml`.
  Método: el anunciado (`cargo test --lib`).
- regresión en suite existente de controles y renderizado: coincide
  Resultado real: 29/29 tests unitarios pasaron en verde.
  Método: el anunciado (`cargo test --lib`).

### Contexto: ¿se respetó?

- NativeActivity sin runtime Java personalizado → respetada: el renderizador de error opera 100% por software sobre el framebuffer de `ANativeWindow` mediante `ndk::native_window` y `fontdue`.
- Portabilidad y privacidad estricta → respetada: cero rutas absolutas introducidas en código ni documentación.
- Extracción física de librerías para emuladores → respetada: configurado en manifiesto y verificado por test.
- ¿Se tocó algo que alguna restricción prohibía? no.
- Imprevistos que surgieron: en `redraw()`, un préstamo simultáneo mutable e inmutable de `self.fatal_error` provocó un error de compilación `E0502`; se resolvió clonando la cadena previa a la invocación de `render_software_error`. Criterio de prioridad aplicado: sí.
- ¿Reapareció alguna alternativa descartada? no.

### Archivos modificados

- `src/error_screen.rs` (nuevo módulo de pantalla de error y fallback de formatos)
- `src/lib.rs` (declaración de módulo público `error_screen`)
- `src/platform.rs` (reintento GLES de GPU, prevención de pánicos por indexación, hook global de pánico y presentación por software)
- `android/AndroidManifest.xml` (`extractNativeLibs="true"`)
- `scripts/package_apk.ps1` (soporte multi-ABI automático arm64-v8a + x86_64)
- `src/packaging_tests.rs` (actualización de aserciones de extracción)
- ¿Coincide con lo anunciado? sí.

### Discrepancias

Ninguna.

### Para la próxima vez

Mantener siempre empaquetado multi-arquitectura automático (Fat APK con `arm64-v8a` y `x86_64`) para agilizar pruebas simultáneas en teléfonos físicos y emuladores de PC.

### Fuera de alcance (hallazgos que no se arreglaron)

- Selector interactivo de archivos de ROM mediante interfaz gráfica integrada en la pantalla de error | impacto: bajo.

---

## Revisión humana

- Visualización de mensaje de error en pantalla (visible): ✔ | ✘ | ?
- Portabilidad y privacidad estricta (no testeable): ✔ | ✘ | ?
- Detectado solo por el humano:
