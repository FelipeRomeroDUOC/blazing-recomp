# Registro de Cambios (Changelog)

Todos los cambios notables y versiones publicadas de **`blazing-recomp`** se documentan en este archivo.

El formato está basado en [Keep a Changelog](https://keepachangelog.com/es-ES/1.0.0/) y este proyecto se adhiere a [Semantic Versioning (SemVer 2.0.0)](https://semver.org/lang/es/) en concordancia con el `versionCode` de Android ($\text{MAJOR}\times 10000 + \text{MINOR}\times 100 + \text{PATCH}$).

---

## [0.0.2] - 2026-10-09

* `versionName`: `"0.0.2"`
* `versionCode`: `2`
* Target principal: `arm64-v8a`, `armeabi-v7a`, `x86_64`

### Añadido
* **Arnés de empaquetado Fat APK Multi-ABI (`scripts/package_apk.ps1`):** Pipeline automatizado sin Gradle que compila con `cargo ndk`, procesa recursos con `aapt2`, alinea memoria a 16 KB con `zipalign` y firma el instalador.
* **Overlay táctil procedural GPU (`src/overlay.wgsl`):** Renderizado en tiempo real mediante funciones de distancia con signo (SDF 3D), biseles, sombras de caída y estética metálica inspirada en la Game Boy Advance SP.
* **Atlas tipográfico estático (`src/overlay.rs`):** Rasterizado de tipografía en CPU a textura de escala de grises de 256x256 px con `fontdue` en arranque.
* **Pantalla de diagnóstico de rescate (`src/error_screen.rs`):** Hook global de pánico que intercepta el framebuffer de la ventana física en CPU con `ANativeWindow_lock` para mostrar errores críticos en pantalla sin cierre silencioso de la aplicación.
* **Compatibilidad con Android TV:** Inclusión de categoría Leanback, declaración de `android.hardware.touchscreen` como no obligatorio y firma simultánea en los esquemas v1 (JAR), v2 y v3 con `apksigner`.
* **Herramienta de extracción de referencia técnica (`scripts/extraer_referencias.py`):** Generación automática de especificaciones de API de Rust desde el código fuente y test de sincronización continua en la suite de pruebas.
* **Auditoría de Mutation Testing (`scripts/auditar_mutantes.py`):** Arnés automatizado que evalúa 16 mutantes semánticos (operadores relacionales, bitmasks de hardware GBA, desbordamientos de memoria en stride y empaquetado uniform) alcanzando un **Mutation Score del 100.0%** sobre la lógica pura.
* **Blindaje de pruebas de frontera y estrés (49 tests):** Incorporación de tests de contorno exacto en colisiones con ternas pitagóricas, zona muerta de cruceta, aspect ratios no estándar ($1:1$, $32:9$, QVGA), churn multitáctil y alineación de padding en framebuffers GPU.

### Rendimiento y Optimización
* **Poda espacial AABB temprana en WGSL:** Descarte inmediato de fragmentos fuera de los clústeres de botones táctiles, reduciendo hasta un 80% los cálculos trigonométricos en GPUs de gama de entrada (Mali-G31 y Adreno 610).
* **Latencia de presentación a 1 fotograma:** Reducción de `desired_maximum_frame_latency` de 2 a 1 en el swapchain y despacho consolidado de eventos de entrada por ráfaga.
* **Reducción masiva de peso con `llvm-strip`:** Stripping no destructivo sobre las bibliotecas `.so` en staging, reduciendo el APK de 340 MB a 39 MB preservando los símbolos DWARF en desarrollo local.

### Cambiado
* **Desacoplamiento a Upstream Oficial (`JRickey/gba-recomp`):** Sustitución de rutas locales relativas por dependencias Git directas fijadas al commit inmutable `de4edf59b872d887046d6a3b005e2df551b6d44c`, permitiendo clonar y compilar el repositorio de forma autónoma sin depender de carpetas hermanas.
* **Topología modular de la suite de pruebas:** Reestructuración de la suite de tests extrayendo las pruebas de integración a `tests/packaging_tests.rs` (habilitando `crate-type = ["cdylib", "rlib"]`) y modularizando las pruebas unitarias extensas en archivos satélite (`src/input_tests.rs`, `src/overlay_tests.rs`) mediante directivas `#[path]`. Esto redujo más de 680 líneas de código en los archivos principales sin exponer APIs privadas.
* **Modernización de respuesta háptica:** Migración al API moderno de Android 8+ (`VibrationEffect.createOneShot`) vía JNI con fallback defensivo y aumento de duración de pulso a 35 ms para vencer la inercia estática de motores de vibración ERM en tablets.
* **Backend de renderizado dinámico en `wgpu`:** Configuración de fallback transparente de Vulkan a OpenGL ES 3.0 para emuladores de PC (BlueStacks 5) y dispositivos con controladores Vulkan incompletos.

### Corregido
* **Error de instalación en Android TV:** Resuelto `INSTALL_FAILED_MISSING_FEATURE` al parametrizar los requisitos de hardware en el manifiesto.
* **Error de arquitectura en emuladores y TV boxes:** Resuelto `INSTALL_FAILED_NO_MATCHING_ABIS` mediante la inclusión simultánea de las 3 arquitecturas en el Fat APK.
* **Silenciamiento háptico en Android 11+:** Resuelto el fallo donde el método deprecado `vibrate(long)` con atributos nulos era ignorado por el sistema.

---

## [0.0.1] - 2026-10-08

* `versionName`: `"0.0.1"`
* `versionCode`: `1`
* Target principal: `arm64-v8a`

### Añadido
* **Crate nativo en Rust (`src/lib.rs`):** Configuración de biblioteca dinámica `cdylib` (`libblazing_recomp.so`) con `android-activity` para `NativeActivity` sin capa DEX (`android:hasCode="false"`).
* **Integración del motor de hardware (`gba-core`):** Vinculación de periféricos, bus de memoria y temporizadores mediante dependencias relativas al repositorio hermano `../gba-recomp`.
* **Compositor de gráficos de pantalla (`screen`):** Integración de presentación acelerada por hardware con `wgpu` sobre la superficie nativa `ANativeWindow`.
* **Módulo geométrico de entrada táctil (`src/input.rs`):** Desacoplamiento matemático puro de cálculo de colisiones y generación de la máscara de bits `KEYINPUT` de GBA (activo en bajo).
* **Mapeo base de funciones (`blazing.labels.toml`):** Especificación inicial de etiquetas bajo formato `gba-labels v2` para las rutinas principales de *Fire Emblem*.
* **Estructura de documentación técnica Diátaxis:** Creación de especificaciones de configuración ([`pack.toml`](docs/referencia/CONFIGURACION_PACK.md)) y formato de etiquetas ([`blazing.labels.toml`](docs/referencia/FORMATO_LABELS.md)).

---

## [Sin publicar] (Hacia v0.1.0 - MVP Jugable)

### Añadido
* **Módulo puro de almacenamiento y validación criptográfica (`src/storage.rs`):** Búsqueda automática y amigable en `Downloads/` (`/sdcard/Download`, `/storage/emulated/0/Download`) y validación estricta por SHA-256 de la ROM europea de *Fire Emblem* (16 MB) y la BIOS oficial (16 KB) usando el crate `sha2`.
* **Topología satélite de pruebas de almacenamiento (`src/storage_tests.rs`):** Suite unitaria en host que valida vectores NIST, rechazo ante alteraciones y escaneo en directorios de prueba.
* **Permisos de almacenamiento en Android (`android/AndroidManifest.xml`):** Inclusión de `READ_EXTERNAL_STORAGE` y `requestLegacyExternalStorage="true"` para lectura directa sin fricción de la carpeta Downloads en emuladores y dispositivos móviles.
* **Pantalla de guía de instalación por software (`src/platform.rs`):** Presentación visual de instrucciones en pantalla si la ROM oficial aún no ha sido colocada en la carpeta `Download/`, evitando cierres silenciosos.
* **Módulo puro de ejecución y pipeline de vídeo (`src/runner.rs`):** Abstracción desacoplada `GameRunner` que encapsula `gba_core::Machine`, `screen::Temporal` y `screen::ColorLut`. Ejecuta 280,896 ciclos por frame, compone el framebuffer PPU a RGBA de 38,400 píxeles e inyecta la máscara de botones `KEYINPUT`.
* **Topología de pruebas satélite para ejecución (`src/runner_tests.rs`):** Suite unitaria en host que valida geometría de búfer, inyección de teclas en hardware, avance determinista de reloj y perfiles de panel LCD.
* **Game loop activo a 60 FPS con suspensión pasiva (`src/platform.rs`):** Bucle de presentación a 60 Hz gobernado por `std::time::Instant` y esperas cooperativas para alimentar el `Compositor` GPU en primer plano, con transición a espera bloqueante (`poll_events(None)`) durante `Pause` para garantizar 0% de consumo de batería en segundo plano.

### Corregido
* **Pantalla negra al iniciar en Android 16:** Resuelto el conflicto de exclusión mutua donde `ANativeWindow_lock` fallaba silenciosamente tras inicializar `wgpu`, bloqueando el swapchain. Ahora el Compositor GPU siempre dibuja la carcasa de la GBA SP y los controles táctiles, mostrando la guía de instalación rasterizada (`render_guide_screen`) en la pantalla virtual si la ROM no está cargada.
* **Soporte de almacenamiento en Android 16 (Scoped Storage):** Incorporación de sondeo directo de rutas de archivo (`fe7.gba`, `Fire Emblem (Europe).gba`), inclusión de la carpeta propia sin permisos `external_data_path` (`/sdcard/Android/data/com.blazingrecomp.game/files/`) y declaración del permiso `MANAGE_EXTERNAL_STORAGE` en el manifiesto.

### Planificado
* Integración del bucle de ejecución de bloques recompilados C11 a 60 FPS coordinado con el hardware emulado.
* Salida de audio nativa mediante AAudio / Oboe de baja latencia.
* Soporte para mandos físicos Bluetooth / USB-C y persistencia de partidas guardadas SRAM.
