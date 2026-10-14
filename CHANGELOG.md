# Registro de Cambios (Changelog)

Todos los cambios notables y versiones publicadas de **`blazing-recomp`** se documentan en este archivo.

El formato está basado en [Keep a Changelog](https://keepachangelog.com/es-ES/1.0.0/) y este proyecto se adhiere a [Semantic Versioning (SemVer 2.0.0)](https://semver.org/lang/es/) en concordancia con el `versionCode` de Android ($\text{MAJOR}\times 10000 + \text{MINOR}\times 100 + \text{PATCH}$).

---

## [Sin publicar]

* **Ampliación masiva de cobertura AOT con 22 nuevos hotspots de CPU (`blazing.labels.toml`):** Incorporación de 22 etiquetas de funciones Thumb descubiertas en dos rondas de telemetría (`0x08076bd8`, `0x08077020`, `0x08069bfc`, `0x0801eb20`, `0x08066bf8`, `0x080a2c9c`, `0x080067ac`, etc.) que eliminaron de raíz los cuellos de botella del intérprete en descompresión LZ77, copias de VRAM, interpolación y fórmulas de combate. La base de etiquetas [`blazing.labels.toml`](blazing.labels.toml) crece a **36.966 claves `name`/`address`** (≈18.483 funciones) con cero colisiones.
* **Optimización lineal de composición de capas y blending en la PPU (`gba-core`):** Sustitución del escaneo anidado por fuerza bruta (`0..4u8`) en `compose` por un recorrido lineal directo sobre el arreglo pre-ordenado `bg_prio` con intercalación canónica del sprite, pre-evaluación y atajo de ventanas por scanline (`line_has_window`), e inlining forzado de las rutinas de aritmética de color (`alpha_blend`, `brighten`, `darken`, `split6`, `join6`). Esto reduce las comprobaciones por píxel de hasta 20 a un máximo de 5, eliminando más del 75% de las ramas condicionales en el pipeline de composición y efectos de color.
* **Optimización de desempaquetado de tiles 4bpp y recorte de sprites en la PPU (`gba-core`):** Carga en ráfaga de 32 bits por fila de tile con extracción de nibbles sin divisiones ni bifurcaciones por píxel, inlining forzado de `pal16`, recorte horizontal temprano de sprites y cálculo invariante de coordenadas verticales para sprites no afines. Esto redujo el tiempo de ejecución `ejec` en escenas típicas de 11.7 ms a 8.4 ms (28% de mejora) con 60 FPS estables.
* **Telemetría granular de hardware de GBA con `perf-counters` (`src/runner.rs`, `src/perf.rs`, `src/platform.rs`):** Integración de contadores de rendimiento desacoplados de `gba-core` para medir el tiempo real de renderizado de la PPU, síntesis de audio del APU y cantidad de accesos al bus de memoria (lecturas y escrituras) por fotograma. Se implementó guarda anticipada en `pump_audio` para prevenir lecturas ociosas de reloj, erradicando el efecto observador y aislando un consumo base de ~4.5 ms para la PPU y ~0.7 ms para el APU con 60 FPS estables.
* **Sincronización reactiva del hilo principal con `AndroidAppWaker` (`src/pipeline.rs`, `src/platform.rs`):** El hilo de emulación dedicado despierta al looper nativo de Android mediante un notificador desacoplado (`WakeFn = Arc<dyn Fn() + Send + Sync>`) tras depositar cada fotograma en el buzón. Esto elevó el timeout de sondeo de 1 ms a 50 ms como respaldo de seguridad, reduciendo las vueltas vacías del hilo principal de ~600/s a ~7/s (98.8% de reducción de sondeo) sin busy-waiting y preservando 60 FPS estables.
* **Topes dinámicos de resolución de swapchain diferenciados por orientación (`src/present.rs`):** Límites de resolución independientes para mitigar saturación de fillrate y ancho de banda en paneles 2K/QHD: 1280px para modo apaisado ($1280 \times 768$) y 960px para modo retrato ($576 \times 960$). Esto redujo el coste de `present` de 52 ms a 0.6–3.5 ms, alcanzando 60 FPS estables en vertical y 56–58 FPS en horizontal.
* **Resolución canónica de biblioteca nativa AOT con `dladdr` (`src/recomp.rs`):** Resuelve la ruta física absoluta de `libblazing_recompiled.so` contigua a `libblazing_recomp.so`, superando las restricciones de los namespaces de Bionic linker en hilos secundarios de `NativeActivity`.
* **Identificador SONAME en biblioteca C11 recompilada (`scripts/package_apk.ps1`):** Inclusión de flag `-Wl,-soname,libblazing_recompiled.so` en Clang NDK para garantizar vinculación dinámica válida en Android.
* **Cosecha exhaustiva de cobertura AOT hasta 144.875 bloques (`blazing.labels.toml`):** Cinco oleadas de mapeo de etiquetas sobre telemetría real elevaron `rcg_block_count` de 83k a **144.875 bloques C11**, cerrando rutinas de combate, descompresión LZ77, copias de VRAM e interpolación ([`2026-10-11_cosecha-exhaustiva-leafs-y-switch-tables-run12.md`](docs/propuestas/2026-10-11_cosecha-exhaustiva-leafs-y-switch-tables-run12.md)).
* **Ejecución nativa de la BIOS real:** El intérprete de respaldo queda reducido al arranque de la BIOS; la lógica del juego corre en bloques C11 recompilados ([`2026-10-12_ejecucion-nativa-de-bios-real.md`](docs/propuestas/2026-10-12_ejecucion-nativa-de-bios-real.md)).
* **Composición de la PPU por tramos y alpha real:** Camino especializado por tramos, camino rápido sin cambiar píxeles, efectos de color inertes y alpha real por tramos, con desglose por partes en la telemetría ([`2026-10-13_ppu-software-rapida-sin-cambiar-pixeles.md`](docs/propuestas/2026-10-13_ppu-software-rapida-sin-cambiar-pixeles.md), [`2026-10-13_alpha-real-por-tramos.md`](docs/propuestas/2026-10-13_alpha-real-por-tramos.md)).
* **Contadores de camino de la PPU y línea `# ppucamino`:** El motor reparte cada línea entre la vía rápida y las que necesitan la segunda capa, y lo publica en la telemetría ([`2026-10-13_contadores-de-camino-de-la-ppu.md`](docs/propuestas/2026-10-13_contadores-de-camino-de-la-ppu.md)).
* **Telemetría por región de memoria (`# regiones`) y volcado de hotspots sin pérdida:** El registro de rendimiento desglosa la ejecución por ROM, IWRAM, EWRAM y BIOS, y el volcado consume solo el top conservando el resto ([`2026-10-12_drain-hotspots-sin-perdida.md`](docs/propuestas/2026-10-12_drain-hotspots-sin-perdida.md)).
* **Arnés de fotogramas dorados por escena:** Detección de regresiones de píxel de la PPU que localiza el cambio en espacio (columna, fila, tile 8×8) y en tiempo (escena y fotograma ancla) ([`2026-10-13_arnes-de-fotogramas-dorados.md`](docs/propuestas/2026-10-13_arnes-de-fotogramas-dorados.md)).
* **Techo del reloj por instrucción acotado y medido:** La palanca se descartó con número (0,13-0,41 % del fotograma), el «0 intérprete» deja de ser el camino del reloj ([`2026-10-14_acotar-el-techo-del-reloj-por-instruccion.md`](docs/propuestas/2026-10-14_acotar-el-techo-del-reloj-por-instruccion.md)).
* **Compresión de las librerías nativas del APK y nombre según perfil:** El Fat APK baja a ~43,9 MB comprimido y el artefacto se nombra por perfil de compilación ([`2026-10-12_compresion-librerias-nativas-apk.md`](docs/propuestas/2026-10-12_compresion-librerias-nativas-apk.md), [`2026-10-12_nombre-apk-segun-perfil.md`](docs/propuestas/2026-10-12_nombre-apk-segun-perfil.md)).
* **Registro de hallazgos y saneamiento de la documentación:** El BACKLOG pasa a ser el tablero vivo (índice de las 57 propuestas y registro de los 81 hallazgos con su origen) y la documentación deja de afirmar cosas que el código contradice ([`2026-10-14_saneamiento-de-la-documentacion.md`](docs/propuestas/2026-10-14_saneamiento-de-la-documentacion.md)).

### Cambiado
* **Triple buffering explícito en swapchain Vulkan (`src/platform.rs`):** Incremento de `desired_maximum_frame_latency` a 2 (`minImageCount = 3`), permitiendo que `VK_PRESENT_MODE_MAILBOX_KHR` opere sin degenerar en doble búfer bloqueante.
* **Restauración de Vulkan como backend gráfico preferente (`src/platform.rs`):** `PREFERIR_OPENGL_ES = false` tras confirmar que OpenGL ES no reducía la latencia del swapchain y generaba sobrecarga adicional en CPU.
* **Buzón de fotogramas no bloqueante (`src/pipeline.rs`):** el hilo de emulación deja de bloquearse esperando a la GPU (`sync_channel(1)` → buzón de un solo slot con pool de búferes reutilizables). Elimina la asignación de 153,6 KB por fotograma (~9 MB/s de basura) y desacopla la velocidad de juego del tiempo de dibujado.
* **Desglose del tiempo de dibujo en la telemetría (`src/perf.rs`, `src/platform.rs`):** cada línea de `blazing-perf.txt` separa ahora `subida`, `codificacion` y `present` además de `dibujo` y `espera`, para atribuir el coste por fotograma.
* **Escala de resolución de render en el compositor (`src/platform.rs`, `src/overlay.rs`):** el búfer del swapchain puede ser menor que la ventana (`ElemParams::scaled` reescala la geometría del overlay y del HUD) y el compositor lo reescala a pantalla completa. Con escala 1.0 el render es idéntico al anterior.

### Corregido
* **Fallo silencioso en carga de bloques recompilados:** Solucionado el problema donde `modo=INT` permanecía forzado y `nativos=0` por fallos de enlace relativo en Android.
* **Prioridad de guardado de telemetría:** `init_perf_file` prioriza `/sdcard/Download/blazing-perf.txt` sobre el almacenamiento privado interno para facilitar el acceso sin exploradores de terceros en Android 11+.

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

### Rendimiento y Optimización
* **Compilación Release con ThinLTO y Vectorización (`Cargo.toml` y `scripts/package_apk.ps1`):** Configuración de perfil `[profile.release]` (`opt-level = 3`, `lto = "thin"`, `codegen-units = 1`) y parametrización de `package_apk.ps1` para compilar con `--release` por defecto. El tiempo de cálculo por fotograma de la GBA se reduce de ~150-200 ms a <2 ms, alcanzando 60 FPS fluidos y reduciendo el tamaño del APK de 40.5 MB a 14.0 MB.

### Corregido
* **Pantalla negra al iniciar en Android 16:** Resuelto el conflicto de exclusión mutua donde `ANativeWindow_lock` fallaba silenciosamente tras inicializar `wgpu`, bloqueando el swapchain. Ahora el Compositor GPU siempre dibuja la carcasa de la GBA SP y los controles táctiles, mostrando la guía de instalación rasterizada (`render_guide_screen`) en la pantalla virtual si la ROM no está cargada.
* **Soporte de almacenamiento en Android 16 (Scoped Storage):** Incorporación de sondeo directo de rutas de archivo (`fe7.gba`, `Fire Emblem (Europe).gba`), inclusión de la carpeta propia sin permisos `external_data_path` (`/sdcard/Android/data/com.blazingrecomp.game/files/`), eliminación del límite `maxSdkVersion="32"` para visibilidad permanente en Ajustes de la App y gestor JNI (`PermissionsManager`) que solicita proactivamente `MANAGE_EXTERNAL_STORAGE` y permisos en el arranque reintentando el escaneo en `Resume`.

### Planificado
* Integración del bucle de ejecución de bloques recompilados C11 a 60 FPS coordinado con el hardware emulado.
* Salida de audio nativa mediante AAudio / Oboe de baja latencia.
* Soporte para mandos físicos Bluetooth / USB-C y persistencia de partidas guardadas SRAM.
