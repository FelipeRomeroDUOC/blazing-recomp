---
tipo: explicacion
verificado: si
---

# Registro Histórico de Decisiones Arquitectónicas y Técnicas (ADR)

Este documento detalla todas las decisiones de diseño, compromisos técnicos y soluciones a problemas críticos adoptadas a lo largo de la evolución de **`blazing-recomp`**, desde la concepción del proyecto hasta la versión actual. Su propósito es brindar contexto a desarrolladores y agentes para entender **el porqué** de cada pieza del sistema.

---

## Contexto Histórico del Proyecto

Blazing Recomp nace con el objetivo de portar *Fire Emblem: The Blazing Blade* (Game Boy Advance) a dispositivos Android nativos mediante recompilación estática anticipada (AOT) en C11 y un modelo de hardware en Rust (`gba-core`), eliminando por completo la emulación tradicional de la CPU. A lo largo del desarrollo, han surgido retos técnicos críticos: fragmentación de hardware móvil, compatibilidad con emuladores de escritorio, ergonomía táctil en pantallas planas, limitaciones de GPUs de gama baja y restricciones de instalación en Smart TVs.

---

## Idea Central del Registro de Decisiones (ADR)

Este registro documenta de manera exhaustiva y transparente cada decisión de diseño, problema crítico y solución adoptada. Cada entrada detalla el contexto del problema, la solución implementada con sus archivos de código fuente vinculados, y las alternativas descartadas junto a los compromisos asumidos.

---

## Registro Cronológico de Decisiones

1. [Seguridad Legal y Aislamiento de Assets Propietarios (Validación SHA-256 en Runtime)](#1-seguridad-legal-y-aislamiento-de-assets-propietarios-validación-sha-256-en-runtime)
2. [Cero Emulación Tradicional: Recompilación Estática AOT a C11](#2-cero-emulación-tradicional-recompilación-estática-aot-a-c11)
3. [Modelo de Cobertura Híbrido: Mapeo de Labels y Red de Seguridad con Intérprete](#3-modelo-de-cobertura-híbrido-mapeo-de-labels-y-red-de-seguridad-con-intérprete)
4. [Desacoplamiento de Repositorio Hermano a Dependencia Git Upstream Oficial (JRickey/gba-recomp)](#4-desacoplamiento-de-repositorio-hermano-a-dependencia-git-upstream-oficial-jrickeygba-recomp)
5. [Entorno de Ejecución Android: NativeActivity Pura sin Capa DEX (`hasCode="false"`)](#5-entorno-de-ejecución-android-nativeactivity-pura-sin-capa-dex-hascodefalse)
6. [Pipeline de Construcción CLI sin Gradle (`aapt2`, `zipalign`, `apksigner`)](#6-pipeline-de-construcción-cli-sin-gradle-aapt2-zipalign-apksigner)
7. [Gestión Segura del Ciclo de Vida de `ANativeWindow` y Liberación de Superficies](#7-gestión-segura-del-ciclo-de-vida-de-anativewindow-y-liberación-de-superficies)
8. [Backend Gráfico `wgpu`: Priorización de OpenGL ES 3.0 sobre Vulkan para Emuladores](#8-backend-gráfico-wgpu-priorización-de-opengl-es-30-sobre-vulkan-para-emuladores)
9. [Desacoplamiento Matemático de la Entrada Táctil (`src/input.rs`)](#9-desacoplamiento-matemático-de-la-entrada-táctil-srcinputrs)
10. [Ergonomía del Mando Virtual: Disposición Diagonal a 16° estilo GBA SP y Cuencas Cóncavas](#10-ergonomía-del-mando-virtual-disposición-diagonal-a-16-estilo-gba-sp-y-cuencas-cóncavas)
11. [Overlay Táctil Procedural por Shaders WGSL (SDF 3D) vs. Sprites PNG](#11-overlay-táctil-procedural-por-shaders-wgsl-sdf-3d-vs-sprites-png)
12. [Rasterizado de Tipografía en Atlas Estático de 256x256 en CPU con `fontdue`](#12-rasterizado-de-tipografía-en-atlas-estático-de-256x256-en-cpu-con-fontdue)
13. [Blindaje de Fallos: Pantalla de Diagnóstico de Emergencia en Framebuffer ANativeWindow](#13-blindaje-de-fallos-pantalla-de-diagnóstico-de-emergencia-en-framebuffer-anativewindow)
14. [Arnés de Empaquetado Fat APK Multi-ABI (`arm64-v8a`, `armeabi-v7a`, `x86_64`)](#14-arnés-de-empaquetado-fat-apk-multi-abi-arm64-v8a-armeabi-v7a-x86_64)
15. [Reducción de Peso sin Pérdida de Debugging: Stripping con `llvm-strip`](#15-reducción-de-peso-sin-pérdida-de-debugging-stripping-con-llvm-strip)
16. [Firma Híbrida v1/v2/v3 y Compatibilidad con Android TV sin Pantalla Táctil](#16-firma-híbrida-v1v2v3-y-compatibilidad-con-android-tv-sin-pantalla-táctil)
17. [Poda Espacial Temprana AABB en Shader y Latencia de Swapchain a 1 Fotograma](#17-poda-espacial-temprana-aabb-en-shader-y-latencia-de-swapchain-a-1-fotograma)
18. [Modernización Háptica a `VibrationEffect` y Pulso de 35 ms](#18-modernización-háptica-a-vibrationeffect-y-pulso-de-35-ms)
19. [Topología de Pruebas: Crate Híbrido cdylib+rlib, Integración en tests/ y Tests Satélite *_tests.rs](#19-topología-de-pruebas-crate-híbrido-cdylibrlib-integración-en-tests-y-tests-satélite-_testsrs)
20. [Blindaje de Suite por Mutation Testing (Mutation Score 100% sobre Lógica Pura)](#20-blindaje-de-suite-por-mutation-testing-mutation-score-100-sobre-lógica-pura)
21. [Desacoplamiento de GameRunner y Pacing de Fotogramas a 60 FPS con Suspensión Pasiva](#21-desacoplamiento-de-gamerunner-y-pacing-de-fotogramas-a-60-fps-con-suspensión-pasiva)
22. [Ingestión y Verificación de ROM/BIOS con Resolución Amigable en Downloads y Hash Criptográfico SHA-256](#22-ingestión-y-verificación-de-rombios-con-resolución-amigable-en-downloads-y-hash-criptográfico-sha-256)
23. [Prevención de Conflicto de Superficie GPU y Guía Visual en Framebuffer GBA para Android 16](#23-prevención-de-conflicto-de-superficie-gpu-y-guía-visual-en-framebuffer-gba-para-android-16)

---

### 1. Seguridad Legal y Aislamiento de Assets Propietarios (Validación SHA-256 en Runtime)

#### Contexto
Distribuir o versionar la ROM de *Fire Emblem: The Blazing Blade* o el archivo binario del BIOS de la Game Boy Advance violaría la propiedad intelectual de Nintendo y comprometería legalmente el repositorio público del proyecto.

#### Decisión e implementación
Establecer una política de sala limpia (*clean-room boundary*):
1. El repositorio **NUNCA** incluye archivos `.gba`, `.bin` ni assets propietarios del juego.
2. La aplicación requiere que el usuario final provea sus propios archivos legales desde el almacenamiento de su dispositivo.
3. Se verifica criptográficamente la integridad de los archivos cotejando sus firmas SHA-256 antes de permitir la ejecución:
   * **ROM oficial:** `Fire Emblem (Europe)` (16 MB) — SHA-256: `6f2c30823ab0237310b138739ccfad879f1a07c4a782a9cbb92317d60d03cee9`.
   * **BIOS oficial:** `gba_bios.bin` (16 KB) — SHA-256: `fd2547724b505f487e6dcb29ec2ecff3af35a841a77ab2e85fd87350abd36570`.

#### Alternativas y compromisos
* **Alternativa descartada:** Incluir la BIOS de código abierto sustituta (como la de Normatt). Descartada porque introduce divergencias sutiles de sincronización con la lógica original de Fire Emblem.
* **Compromiso asumido:** Obliga a implementar un flujo inicial de selección de archivos en Android con validación explícita de hashes.

---

### 2. Cero Emulación Tradicional: Recompilación Estática AOT a C11

#### Contexto
Los emuladores clásicos de Game Boy Advance en Android interpretan o recompilan dinámicamente (JIT) las instrucciones ARMv4T en tiempo de ejecución. Esto introduce sobrecarga de CPU, consumo intensivo de batería y caídas esporádicas de fotogramas por compilación de bloques.

#### Decisión e implementación
Recompilar estáticamente (Ahead-Of-Time) el binario oficial de *Fire Emblem: The Blazing Blade* a código C11 nativo, compilado directamente para las arquitecturas del dispositivo (`aarch64`, `armv7`, `x86_64`). El runtime en Rust provisto por `gba-core` emula los periféricos (PPU, APU, DMA, timers, bus de memoria), pero la lógica del juego corre a velocidad nativa sin intérprete de CPU.

#### Alternativas y compromisos
* **Alternativa descartada:** Emulación tradicional con JIT (como mGBA o VBA-M). Descartada por ineficiencia energética y complejidad de integración JNI.
* **Compromiso asumido:** Requiere un mapeo exhaustivo de etiquetas de funciones ([`blazing.labels.toml`](../../blazing.labels.toml)) para que el 100% de la lógica se traduzca estáticamente a C11, requiriendo validación rigurosa de SHA-256 de la ROM.

---

### 3. Modelo de Cobertura Híbrido: Mapeo de Labels y Red de Seguridad con Intérprete

#### Contexto
En un juego con 17,207 funciones y cientos de punteros a funciones indirectos o tablas de saltos (*jump tables*), traducir el 100% del binario sin errores desde el primer día es prácticamente imposible. Una sola bifurcación no catalogada causaría un cierre inmediato de la aplicación.

#### Decisión e implementación
Adoptar un modelo híbrido documentado en [`arquitectura-c-recompilado.md`](arquitectura-c-recompilado.md):
1. **Ruta primaria nativa (99%+):** La mayor parte de la lógica se compila estáticamente como bloques C11 ultrarrápidos gobernados por el archivo de etiquetas [`blazing.labels.toml`](../../blazing.labels.toml).
2. **Red de seguridad con intérprete:** Se habilita la flag `interpreter = true` en [`pack.toml`](../../pack.toml). Si el contador de programa salta a una dirección de memoria sin recompilar, el motor no crashea: `gba-core` recurre a su intérprete de instrucciones de respaldo de forma transparente, permitiendo depurar y continuar la partida hasta catalogar la función faltante.

#### Alternativas y compromisos
* **Alternativa descartada:** Modo puramente AOT sin intérprete desde el día 1. Descartada porque cualquier salto indirecto desconocido abortaba toda la sesión de juego.
* **Compromiso asumido:** El binario final incluye el submódulo decodificador `armv4t`, aumentando levemente el peso del código a cambio de inmunidad ante cuelgues imprevistos.

---

### 4. Desacoplamiento de Repositorio Hermano a Dependencia Git Upstream Oficial (JRickey/gba-recomp)

#### Contexto
El proyecto requiere los subsistemas de bus de hardware (`gba-core`) y presentación de pantalla (`screen`) desarrollados en el toolkit base `gba-recomp`. Inicialmente, las dependencias en `Cargo.toml` se enlazaron mediante rutas relativas a una carpeta hermana (`path = "../gba-recomp/crates/..."`). Sin embargo, este esquema generaba un acoplamiento frágil de entorno local: cualquier clonado en un entorno de integración continua (CI/CD) o en la máquina de otro desarrollador fallaba de inmediato al no existir la carpeta hermana en el sistema de archivos.

#### Decisión e implementación
1. **Dependencia Git upstream con pin inmutable:** Enlazar directamente [`Cargo.toml`](../../Cargo.toml) al repositorio oficial de Jack Rickey (`https://github.com/JRickey/gba-recomp.git`), fijando la revisión exacta mediante su hash criptográfico SHA (`rev = "de4edf59b872d887046d6a3b005e2df551b6d44c"`).
2. **Autonomía total del repositorio:** Garantizar que cualquier usuario o runner de CI pueda clonar únicamente `blazing-recomp` y compilarlo sin requerir dependencias locales previas.
3. **Desarrollo local flexible:** Para iteraciones simultáneas en el motor y el port, Cargo permite sobreescribir temporalmente la fuente con la sección `[patch."https://github.com/JRickey/gba-recomp.git"]` apuntando a `../gba-recomp`.
4. **Regla de portabilidad y privacidad:** Prohibir terminantemente cualquier ruta absoluta del sistema de archivos local (`C:\Users\...` o `/home/...`) en cualquier archivo versionado del proyecto.

#### Alternativas y compromisos
* **Alternativa descartada (Carpetas hermanas obligatorias):** Requerir `../gba-recomp` en el disco. Descartada por romper el flujo estándar de `git clone` y fallar en CI/CD.
* **Alternativa descartada (Fork personal sin sincronizar):** Apuntar al fork del usuario en GitHub. Descartada porque se desincronizaría de las mejoras oficiales upstream de Jack Rickey.
* **Compromiso asumido:** La primera compilación en una máquina limpia requiere conexión a internet para descargar y cachear el repositorio Git del motor.

---

### 5. Entorno de Ejecución Android: NativeActivity Pura sin Capa DEX (`hasCode="false"`)

#### Contexto
La mayoría de las aplicaciones Android usan Gradle, Kotlin y Android SDK para inicializar la ventana y luego llaman al código nativo vía JNI (`System.loadLibrary`).

#### Decisión e implementación
Configurar la aplicación como una `android.app.NativeActivity` pura con `android:hasCode="false"` en [`android/AndroidManifest.xml`](../../android/AndroidManifest.xml), apoyada en la biblioteca Rust `android-activity`. Toda la ejecución inicia y reside dentro de la función nativa `android_main` en [`src/platform.rs`](../../src/platform.rs).

#### Alternativas y compromisos
* **Alternativa descartada:** Proyecto Android estándar con Gradle, Activity en Kotlin y enlaces JNI bidireccionales. Descartada para evitar dependencias del runtime de JVM para el arranque gráfico y permitir compilaciones ultrarrápidas desde CLI.
* **Compromiso asumido:** Cualquier servicio del sistema operativo (como la vibración o ventanas del sistema) debe invocarse manualmente mediante JNI nativo hacia la instancia de la `NativeActivity`.

---

### 6. Pipeline de Construcción CLI sin Gradle (`aapt2`, `zipalign`, `apksigner`)

#### Contexto
El sistema de compilación de Android estándar (Gradle + Android Gradle Plugin) requiere descargas masivas de paquetes, configuración pesada de Java y tiempos de sincronización lentos (15–30 segundos por cambio menor). Para un proyecto nativo en Rust/C, Gradle añade una capa innecesaria de fricción.

#### Decisión e implementación
Diseñar un arnés de empaquetado autónomo en PowerShell ([`scripts/package_apk.ps1`](../../scripts/package_apk.ps1)) que orquesta directamente las herramientas oficiales del Android SDK:
1. `cargo ndk` compila los binarios `.so` en milisegundos.
2. `aapt2 compile` y `aapt2 link` procesan los recursos y el `AndroidManifest.xml`.
3. `jar` empaqueta las bibliotecas nativas de forma alineada.
4. `zipalign -p 16` alinea los recursos a páginas de memoria de 16 KB (requerido por Android 15+).
5. `apksigner` firma el APK final en los esquemas v1, v2 y v3.

#### Alternativas y compromisos
* **Alternativa descartada:** Proyecto Gradle con `cargo-ndk-android-gradle`. Descartada por peso, fragilidad de versiones y lentitud de ciclo de desarrollo.
* **Compromiso asumido:** El script de empaquetado debe mantenerse y validarse con pruebas automatizadas ([`tests/packaging_tests.rs`](../../tests/packaging_tests.rs)).

---

### 7. Gestión Segura del Ciclo de Vida de `ANativeWindow` y Liberación de Superficies

#### Contexto
Cuando un usuario bloquea el teléfono, recibe una llamada telefónica o rota la pantalla, el sistema operativo Android destruye inmediatamente la superficie gráfica (`ANativeWindow`). Si el motor gráfico continúa intentando renderizar o retiene punteros a dicha ventana, la aplicación experimenta un fallo de segmentación (SIGSEGV).

#### Decisión e implementación
En [`src/platform.rs`](../../src/platform.rs), orquestar la inicialización y destrucción gráfica de forma estricta según los eventos del looper:
1. **`InitWindow`:** Solo cuando Android entrega un puntero válido a la ventana, se instancia el `Compositor` y se configura la superficie `wgpu`.
2. **`WindowDestroyed`:** Al recibir este evento, se destruye de inmediato la `Surface` y se anula la referencia gráfica antes de devolver el control al sistema.
3. **`WindowResized`:** Se reconfiguran los tamaños de swapchain y se recalculan las áreas táctiles (`touch_controller.resize(w, h)`).

#### Alternativas y compromisos
* **Alternativa descartada:** Mantener la superficie activa en segundo plano. Descartada porque viola el modelo de memoria de Android y causa crashes inmediatos.
* **Compromiso asumido:** El motor gráfico debe ser capaz de recrear o restaurar la superficie en cualquier momento durante la ejecución de la app.

---

### 8. Backend Gráfico `wgpu`: Priorización de OpenGL ES 3.0 sobre Vulkan para Emuladores

#### Contexto
Al probar la aplicación en emuladores de PC (como BlueStacks 5) o dispositivos Android antiguos, la inicialización gráfica fallaba con pantalla negra o pánico: los controladores Vulkan en emuladores suelen estar rotos o ausentes.

#### Decisión e implementación
En [`src/platform.rs`](../../src/platform.rs), configurar la instancia de `wgpu` con `Backends::GL | Backends::VULKAN` solicitando el adaptador con prioridad `HighPerformance` o fallback a GLES 3.0. Si el dispositivo no tiene Vulkan completamente funcional, wgpu degrada limpiamente a OpenGL ES 3.0, garantizando renderizado fluido tanto en dispositivos físicos como en cualquier emulador de escritorio.

#### Alternativas y compromisos
* **Alternativa descartada:** Exigir Vulkan como requisito indispensable (`android:required="true"` para Vulkan en el manifiesto). Descartada porque impedía la ejecución en BlueStacks 5 y dispositivos de gama media/baja.
* **Compromiso asumido:** GLES 3.0 impone límites más restrictivos de buffers y formatos de textura, requiriendo consultar `adapter.limits()` al inicializar el dispositivo.

---

### 9. Desacoplamiento Matemático de la Entrada Táctil (`src/input.rs`)

#### Contexto
En aplicaciones móviles es común mezclar la detección de toques con las APIs del sistema (`MotionEvent`) y el pipeline de renderizado gráfico, haciendo imposible testear la lógica de colisiones en PC sin un dispositivo Android.

#### Decisión e implementación
Diseñar [`src/input.rs`](../../src/input.rs) como un módulo puro de lógica y geometría matemática. `TouchController` no conoce Android, NDK ni GPU; solo recibe coordenadas `(x, y)` de pantalla y una orientación (`Portrait`/`Landscape`), devolviendo áreas circulares o pastillas (`TouchArea`) y la máscara de bits `KEYINPUT` de GBA (activo en bajo).

#### Alternativas y compromisos
* **Alternativa descartada:** Usar los listeners táctiles de Android para actualizar variables globales directamente en `src/platform.rs`.
* **Compromiso asumido:** Todo el sistema cuenta con pruebas unitarias deterministas que se ejecutan en milisegundos en Windows/Linux con `cargo test`.

---

### 10. Ergonomía del Mando Virtual: Disposición Diagonal a 16° estilo GBA SP y Cuencas Cóncavas

#### Contexto
En pantallas táctiles planas, los jugadores carecen de retroalimentación táctil física. Los emuladores convencionales colocan los botones A y B alineados horizontalmente o en ángulos genéricos de 45°, provocando fatiga en el pulgar y pulsaciones erróneas durante combates intensos.

#### Decisión e implementación
Inspirarse fielmente en la geometría física de la Game Boy Advance SP:
1. **Eje de acción a 16°:** Los botones B y A se orientan a lo largo de un eje inclinado a -16° (la trayectoria natural del pulgar en reposo), con B a la izquierda y A a la derecha.
2. **Gatillos L y R sobre el eje perpendicular:** Los botones de hombro se ubican inmediatamente por encima de B y A siguiendo la misma inclinación.
3. **Cuencas cóncavas esculpidas:** Tanto la cruceta direccional como los botones de acción se ubican dentro de áreas circulares deprimidas visualmente, delimitando un umbral claro de interacción.
4. **Zona muerta central en D-Pad:** Evita registrar pulsaciones de diagonales accidentales al reposar el pulgar en el centro.

#### Alternativas y compromisos
* **Alternativa descartada:** Botones rectos ortogonales (horizontal o vertical). Descartada por mala ergonomía móvil.
* **Compromiso asumido:** Requiere transformaciones de rotación matemática 2D en `src/input.rs` y `src/overlay.wgsl`.

---

### 11. Overlay Táctil Procedural por Shaders WGSL (SDF 3D) vs. Sprites PNG

#### Contexto
Los emuladores convencionales utilizan imágenes PNG estáticas para los botones táctiles. En pantallas modernas (con densidades DPI extremas y relaciones de aspecto de 16:9, 19.5:9, 21:9 o 4:3), los PNGs se pixelan, se deforman o requieren cientos de megabytes en texturas para diferentes resoluciones.

#### Decisión e implementación
Generar los controles táctiles proceduralmente en el fragment shader GPU ([`src/overlay.wgsl`](../../src/overlay.wgsl)) usando funciones de distancia con signo (**SDF** - *Signed Distance Fields*). El shader calcula en tiempo real biseles 3D, sombras de caída, iluminación especular cóncava para las cuencas y acabado metálico estilo Game Boy Advance SP.

#### Alternativas y compromisos
* **Alternativa descartada:** Sprites PNG con quad-blending. Descartada por baja fidelidad visual y falta de adaptación a resoluciones arbitrarias.
* **Compromiso asumido:** Mayor demanda de procesamiento por fragmento en la GPU, lo que requirió optimizaciones posteriores de poda espacial.

---

### 12. Rasterizado de Tipografía en Atlas Estático de 256x256 en CPU con `fontdue`

#### Contexto
El shader WGSL necesita dibujar etiquetas sobre los botones ("A", "B", "L", "R", "SELECT", "START" y flechas de la cruceta). Evaluar fuentes vectoriales o curvas Bézier en cada píxel en el fragment shader es inviable para GPUs móviles.

#### Decisión e implementación
Durante la carga de la aplicación (`src/overlay.rs`), se utiliza la biblioteca `fontdue` en la CPU para rasterizar las tipografías *Outfit-Bold* y *JetBrainsMono-Bold* dentro de un atlas de textura de escala de grises de solo **256x256 píxeles** (64 KB). El shader consulta este atlas mediante coordenadas UV normalizadas y aplica tinte dinámico.

#### Alternativas y compromisos
* **Alternativa descartada:** Shaders de renderizado vectorial de curvas (como Slug o Pathfinder). Descartada por peso de dependencias y complejidad en OpenGL ES 3.0.
* **Compromiso asumido:** Tamaño de glifos predefinido a 64x64 px en el atlas; el shader aplica interpolación bilineal para escalados suaves.

---

### 13. Blindaje de Fallos: Pantalla de Diagnóstico de Emergencia en Framebuffer ANativeWindow

#### Contexto
Cuando una biblioteca nativa en Android sufre un pánico en Rust o un fallo de wgpu, el comportamiento por defecto es un cierre forzoso inmediato (*crash to launcher*). El usuario solo ve que la app "se cierra sola" y el desarrollador no puede ver el error sin conectar `adb logcat`.

#### Decisión e implementación
En [`src/platform.rs`](../../src/platform.rs), se instala un hook global con `std::panic::set_hook` y se envuelve el hilo principal en `std::panic::catch_unwind`. Si ocurre un pánico crítico, el subsistema intercepta la ventana física (`ANativeWindow`), bloquea su buffer de memoria en CPU con `ANativeWindow_lock` y dibuja una pantalla de rescate con tipografía bitmap incrustada ([`src/error_screen.rs`](../../src/error_screen.rs)), mostrando el mensaje del fallo y la ubicación del archivo y línea en pantalla hasta que el usuario cierre la app.

#### Alternativas y compromisos
* **Alternativa descartada:** Confiar únicamente en `adb logcat`. Descartada porque los usuarios finales y testeadores en televisores no disponen de conexión USB de depuración.
* **Compromiso asumido:** Mantener un renderizador tipográfico bitmap mínimo por software en CPU dentro del binario.

---

### 14. Arnés de Empaquetado Fat APK Multi-ABI (`arm64-v8a`, `armeabi-v7a`, `x86_64`)

#### Contexto
Android se ejecuta en una diversidad de procesadores: smartphones modernos (ARM64), emuladores en PC (x86_64) y Smart TVs o TV boxes económicas (procesadores ARM de 32 bits `armeabi-v7a`). Compilar solo para ARM64 causaba el error `INSTALL_FAILED_NO_MATCHING_ABIS`.

#### Decisión e implementación
El script [`scripts/package_apk.ps1`](../../scripts/package_apk.ps1) compila y empaqueta un **Fat APK universal** incorporando simultáneamente las bibliotecas `.so` para los tres targets:
* `arm64-v8a` (`aarch64-linux-android`)
* `armeabi-v7a` (`armv7-linux-androideabi`)
* `x86_64` (`x86_64-linux-android`)
Además, se declara `android:extractNativeLibs="true"` para compatibilidad con instaladores heredados que no ejecutan bibliotecas directamente desde el archivo zip.

#### Alternativas y compromisos
* **Alternativa descartada:** Generar 3 APKs separados (Split APKs). Descartada porque complica la distribución manual por sideloading y pendrive USB para los usuarios.
* **Compromiso asumido:** El APK final pesa más al contener tres copias del runtime nativo.

---

### 15. Reducción de Peso sin Pérdida de Debugging: Stripping con `llvm-strip`

#### Contexto
Al compilar las 3 arquitecturas nativas en modo debug con símbolos DWARF, el APK resultante alcanzaba **340 MB**. Sin embargo, compilar en modo `--release` eliminaba las aserciones de seguridad y dificultaba el diagnóstico temprano.

#### Decisión e implementación
En [`scripts/package_apk.ps1`](../../scripts/package_apk.ps1), se incorporó un paso de stripping no destructivo usando `llvm-strip.exe --strip-all` provisto por el Android NDK. El strip se ejecuta únicamente sobre las copias que van dentro del staging del APK (`staging/lib/<abi>/*.so`), manteniendo los archivos originales intactos en `target/<abi>/debug/` para depuración con LLDB/GDB. El tamaño del APK se redujo drásticamente de **340 MB a 39 MB**.

#### Alternativas y compromisos
* **Alternativa descartada:** Configurar `strip = true` global en `Cargo.toml`. Descartada porque mutilaba los archivos de desarrollo local.
* **Compromiso asumido:** Requiere que el NDK de Android esté presente en la máquina para localizar `llvm-strip.exe`.

---

### 16. Firma Híbrida v1/v2/v3 y Compatibilidad con Android TV sin Pantalla Táctil

#### Contexto
Al intentar instalar el APK en televisores inteligentes con Android 10, el sistema abortaba con `INSTALL_FAILED_MISSING_FEATURE` ("No se instaló la aplicación"). Esto ocurría porque Android asume que toda aplicación requiere panel táctil físico y porque los instaladores básicos de TV rechazan paquetes firmados únicamente con esquemas modernos v2/v3.

#### Decisión e implementación
1. En [`android/AndroidManifest.xml`](../../android/AndroidManifest.xml), se declaró explícitamente:
   ```xml
   <uses-feature android:name="android.hardware.touchscreen" android:required="false" />
   <uses-feature android:name="android.software.leanback" android:required="false" />
   <category android:name="android.intent.category.LEANBACK_LAUNCHER" />
   ```
2. En [`scripts/package_apk.ps1`](../../scripts/package_apk.ps1), se forzó la firma con `apksigner` en los 3 esquemas simultáneos: `--v1-signing-enabled true`, `--v2-signing-enabled true` y `--v3-signing-enabled true`.

#### Alternativas y compromisos
* **Alternativa descartada:** Crear una variante separada para Android TV. Descartada por complejidad de mantenimiento.
* **Compromiso asumido:** La aplicación aparece tanto en el lanzador táctil como en la fila de apps/juegos de Android TV Leanback.

---

### 17. Poda Espacial Temprana AABB en Shader y Latencia de Swapchain a 1 Fotograma

#### Contexto
En dispositivos con GPUs de gama de entrada (GPU Mali-G31 en Android TV y Adreno 610 en tablets), el overlay sufría una latencia de 100–200 ms entre el toque y la animación del botón. El fragment shader calculaba los SDFs y rotaciones de los 8 botones sobre todos los millones de píxeles de pantalla, saturando la tasa de relleno (*fillrate*), sumado a un swapchain que encolaba 2 fotogramas (`desired_maximum_frame_latency: 2`).

#### Decisión e implementación
1. **Poda AABB temprana en WGSL:** En [`src/overlay.wgsl`](../../src/overlay.wgsl), se implementó un descarte temprano por caja delimitadora. Si un fragmento cae fuera de los clústeres de botones, el shader retorna en solo 2 instrucciones (ahorrando un ~80% de cálculos en Landscape).
2. **Muestreo tipográfico mip-level 0:** Se sustituyó `textureSample` por `textureSampleLevel(..., 0.0)` para permitir ramificaciones divergentes sin degradación de rendimiento.
3. **Latencia a 1 fotograma:** En `SurfaceConfiguration`, se redujo `desired_maximum_frame_latency` de 2 a 1, eliminando la cola de presentación intermedia.
4. **Despacho consolidado de eventos:** Los toques se acumulan y procesan inmediatamente, llamando a `state.redraw()` una sola vez por ráfaga de eventos.

#### Alternativas y compromisos
* **Alternativa descartada:** Bajar la resolución de pantalla a 720p. Descartada porque arruinaría la nitidez del texto y del juego en pantallas grandes.
* **Compromiso asumido:** Requiere calcular márgenes de seguridad (> 16 px) en los AABB para no recortar sombras ni rotaciones.

---

### 18. Modernización Háptica a `VibrationEffect` y Pulso de 35 ms

#### Contexto
Al optimizar la latencia visual, la respuesta háptica dejó de sentirse en tablets con Android 11. El método primitivo `vibrator.vibrate(long)` de la API 1 está deprecado y Android 11 silencia las vibraciones con atributos nulos (`USAGE_UNKNOWN`). Además, los motores de vibración rotatorios mecánicos (ERM) de las tablets tienen una inercia de arranque de ~30 ms, haciendo imperceptible el pulso original de 15 ms al no haber bloqueo en la GPU.

#### Decisión e implementación
En [`src/platform.rs`](../../src/platform.rs):
1. Se aumentó [`HAPTIC_MS`](../../src/platform.rs#L28) de 15 ms a **35 ms** para superar la inercia estática de los motores ERM sin demorar la sensación táctil.
2. Se implementó la API moderna de Android 8 a 15+ vía JNI:
   ```java
   VibrationEffect.createOneShot(ms, VibrationEffect.DEFAULT_AMPLITUDE);
   vibrator.vibrate(vibrationEffect);
   ```
   con limpieza de excepciones (`env.exception_clear()`) y fallback a `vibrate(long)`.
3. Se elevaron los logs de fallo a `log::warn!` para garantizar visibilidad en Logcat.

#### Alternativas y compromisos
* **Alternativa descartada:** Usar vibraciones predefinidas como `EFFECT_CLICK` (API 29+). Descartada porque no permite graduar la duración en dispositivos de API 26-28.
* **Compromiso asumido:** Código JNI dual con manejo defensivo de excepciones Java.

---

### 19. Topología de Pruebas: Crate Híbrido cdylib+rlib, Integración en tests/ y Tests Satélite `*_tests.rs`

#### Contexto
A medida que la suite de pruebas creció hasta 35 tests, módulos como `src/overlay.rs` (714 líneas) e `src/input.rs` (616 líneas) tenían más del 50% de su código dedicado a bloques `mod tests`. Esto saturaba la legibilidad del código de producción. Además, `packaging_tests.rs` residía dentro de `src/lib.rs` pese a no probar lógica interna de Rust, sino la integridad del manifiesto XML, empaquetado del APK y sincronización de documentación. Sin embargo, configurar `blazing-recomp` exclusivamente como `crate-type = ["cdylib"]` impedía a Cargo enlazar una carpeta de integración externa `tests/`.

#### Decisión e implementación
1. **Crate híbrido en `Cargo.toml`:** Se amplió a `crate-type = ["cdylib", "rlib"]`, permitiendo a Cargo generar la metadata de enlace para tests de integración externos sin alterar el `.so` nativo de Android.
2. **Directorio externo `tests/`:** Se trasladó `packaging_tests.rs` a [`tests/packaging_tests.rs`](../../tests/packaging_tests.rs), separando las pruebas de caja negra del árbol de código fuente.
3. **Módulos de test satélite en `src/`:** Para `src/overlay.rs` e `src/input.rs`, los tests unitarios se extrajeron a archivos satélite adyacentes ([`src/overlay_tests.rs`](../../src/overlay_tests.rs) e [`src/input_tests.rs`](../../src/input_tests.rs)) conectados mediante `#[cfg(test)] #[path = "..."] mod tests;`. Esto redujo `overlay.rs` a 338 líneas y `input.rs` a 310 líneas de lógica pura, manteniendo pleno acceso a structs y funciones privadas sin violar la encapsulación.
4. **Módulos compactos:** En módulos de tamaño moderado ([`src/error_screen.rs`](../../src/error_screen.rs)), se mantuvo el `mod tests` al pie del archivo.

#### Alternativas y compromisos
* **Alternativa descartada:** Mover todos los tests unitarios a `tests/`. Descartada porque obligaría a hacer públicas (`pub`) estructuras y funciones internas de cálculo trigonométrico y rasterizado.
* **Compromiso asumido:** Requiere mantener `crate-type = ["cdylib", "rlib"]` en `Cargo.toml`.

---

### 20. Blindaje de Suite por Mutation Testing (Mutation Score 100% sobre Lógica Pura)

#### Contexto
Una alta cobertura de código por líneas (*line coverage*) no garantiza la detección efectiva de errores semánticos; una aserción laxa o incompleta puede ejecutar una línea al 100% sin detectar una mutación relacional (`<` en vez de `<=`), una constante de registro GBA errónea o un desbordamiento de padding de memoria. Para verificar la resistencia de la suite (49 tests), era indispensable aplicar una auditoría de mutación sistemática sin depender de herramientas externas pesadas o invasivas.

#### Decisión e implementación
1. **Arnés de auditoría en host:** Se diseñó [`scripts/auditar_mutantes.py`](../../scripts/auditar_mutantes.py), aplicando mutaciones sintéticas precisas en memoria RAM y restaurando los archivos automáticamente tras evaluar la suite con `cargo test --lib`.
2. **Mutantes sintéticos representativos:** Se catalogaron 16 mutantes distribuidos en los tres módulos de lógica pura:
   - `src/input.rs`: Operadores relacionales de colisión en frontera (`contains`), inversión de banderas de entrada, colisión de bitmasks de registro GBA (`KEYINPUT`), alteración de zona muerta de la cruceta y anulación de ángulos trigonométricos.
   - `src/overlay.rs`: Inversión de flags de renderizado de la carcasa SP, intercambio de campos en el empaquetado de memoria uniform para el shader (`Globals::pack` y `ElemParams::pack`), y asignación cruzada de glifos.
   - `src/error_screen.rs`: Inversión de prioridad de formatos sRGB, desbordamiento de memoria sobre padding en bucles de fila (`width` a `stride`), y eliminación de fallbacks ante adaptadores GPU degradados.
3. **Puntuación de mutación perfecta:** La suite eliminó el 100% de los mutantes evaluados (16 eliminados / 0 supervivientes), alcanzando un **Mutation Score del 100.0%** sin alterar una sola línea de código de producción.

#### Alternativas y compromisos
* **Alternativa descartada:** Usar herramientas automatizadas de mutación genéricas para Rust (como `cargo-mutants`). Descartada en esta fase debido a la necesidad de evaluar mutaciones semánticas de hardware y shaders con feedback determinista e instantáneo.
* **Compromiso asumido:** Mantener actualizado el script de mutantes representativos a medida que se agreguen nuevos subsistemas puros al proyecto.

---

### 21. Desacoplamiento de GameRunner y Pacing de Fotogramas a 60 FPS con Suspensión Pasiva

#### Contexto
En Android, una aplicación de juego no puede ejecutar bucles de espera activa (*busy-waiting*) ni acoplar el avance de hardware de la consola (`Machine`) directamente al hilo de la interfaz o al ciclo de vida de la actividad sin ocasionar estrangulamiento térmico (*thermal throttling*), drenaje excesivo de batería y pérdida de la capacidad de ejecutar pruebas unitarias automáticas en máquinas de desarrollo host. Además, la PPU de GBA genera un búfer crudo RGB555 de $240 \times 160$ que debe pasar por los filtros temporales del panel LCD (`Temporal`) y transformarse con la tabla de color (`ColorLut`) antes de subirse a la textura GPU del `Compositor`.

#### Decisión e implementación
1. **Módulo puro desacoplado (`src/runner.rs`):** Se creó la abstracción [`GameRunner`](../../src/runner.rs), encapsulando `gba_core::Machine`, `screen::Temporal` y `screen::ColorLut` con cero dependencias del NDK de Android o de `wgpu`. El método `step_frame()` avanza exactamente 280,896 ciclos de reloj (un fotograma GBA completo), procesa el búfer temporal y genera el búfer RGBA final de 38,400 píxeles listo para presentación.
2. **Topología de pruebas satélite (`src/runner_tests.rs`):** Cumpliendo la directriz modular de `AGENTS.md`, la suite de pruebas unitarias de `GameRunner` reside en el archivo satélite adyacente [`src/runner_tests.rs`](../../src/runner_tests.rs) vinculado mediante `#[cfg(test)] #[path = "runner_tests.rs"] mod tests;`. Esto permite verificar en host (`cargo test`) la integridad geométrica, canal alfa (255), inyección de teclas `KEYINPUT`, avance de ciclos y perfiles de panel LCD.
3. **Pacing a 60 FPS en el ciclo de vida Android (`src/platform.rs`):** Se transformó `android_main` en un game loop activo a 60 Hz gobernado por `std::time::Instant` y pausas cooperativas (`thread::sleep`) cuando la aplicación está en primer plano (`Resume`).
4. **Suspensión pasiva a 0% de batería (`Pause`):** Cuando la aplicación pasa a segundo plano o se apaga la pantalla, el bucle activo de fotogramas se suspende y el hilo entra en espera bloqueante mediante `app.poll_events(None, ...)`, liberando por completo la CPU y GPU del dispositivo.

#### Alternativas y compromisos
* **Alternativa descartada:** Acoplar la instancia de `Machine` directamente dentro de `Compositor` o en callbacks JNI. Descartada porque impediría escribir tests unitarios en host y mezclaría responsabilidades gráficas con lógica de hardware.
* **Alternativa descartada:** Mantener `app.poll_events(Some(Duration::ZERO))` indefinidamente en segundo plano. Descartada porque causaría un drenaje severo de batería al mantener la CPU despierta.
* **Compromiso asumido:** Requiere coordinar el temporizador de fotogramas del host/NDK con el modo VSync de `wgpu` para evitar tearing visual.

---

### 22. Ingestión y Verificación de ROM/BIOS con Resolución Amigable en Downloads y Hash Criptográfico SHA-256

#### Contexto
Para respetar la política de sala limpia (*clean-room*) del proyecto (`AGENTS.md §3`), el repositorio no distribuye ningún archivo propietario de *Fire Emblem: The Blazing Blade* ni de la BIOS oficial de Game Boy Advance. Sin embargo, en Android 11+ el acceso al almacenamiento privado de aplicaciones (`/sdcard/Android/data/...`) está fuertemente restringido para el usuario común mediante Scoped Storage, requiriendo herramientas complejas o permisos de administrador. Era indispensable brindar una experiencia de usuario amigable y libre de fricción: permitir que el usuario simplemente descargue su copia legal a la carpeta habitual de descargas (`Downloads/`) y que la aplicación detecte, verifique y cargue el juego automáticamente.

#### Decisión e implementación
1. **Módulo puro de almacenamiento ([`src/storage.rs`](../../src/storage.rs)):** Se diseñó una abstracción desacoplada de la plataforma que encapsula el cálculo de hashes criptográficos con el crate `sha2` y valida las firmas oficiales pinneadas:
   - ROM oficial: `Fire Emblem (Europe)` (16 MB) — SHA-256 `6f2c30823ab0237310b138739ccfad879f1a07c4a782a9cbb92317d60d03cee9`.
   - BIOS oficial: `gba_bios.bin` (16 KB) — SHA-256 `fd2547724b505f487e6dcb29ec2ecff3af35a841a77ab2e85fd87350abd36570`.
2. **Resolución amigable de directorios:** La función `resolve_search_directories` establece un orden estricto de búsqueda:
   - Prioridad 1 y 2: `/sdcard/Download` y `/storage/emulated/0/Download`.
   - Prioridad 3 y 4: `Download` y `roms` (relativas locales para pruebas en emuladores de PC y host).
   - Prioridad 5: Directorio privado interno de la app provisto por `AndroidApp`.
3. **Escaneo inteligente por firma digital:** `scan_and_load_game` inspecciona archivos `.gba` candidatos en `Downloads/` comprobando su tamaño y hash SHA-256. Esto permite que el usuario nombre el archivo con cualquier convención común (`fe7.gba`, `Fire Emblem.gba` o el nombre completo del volcado legal) sin fallos. Si la BIOS oficial está presente en la misma carpeta, la inyecta; si no, recurre de forma transparente a la BIOS HLE de `gba-core`.
4. **Pantalla de guía ante ausencia de ROM:** Si la ROM no se encuentra en `Downloads/`, la aplicación no crashea ni entra en pánico; en su lugar, `src/platform.rs` activa un aviso informativo visual por software explicando la ruta exacta donde colocar el archivo legal para comenzar a jugar.
5. **Permisos en el manifiesto:** Se añadieron `<uses-permission android:name="android.permission.READ_EXTERNAL_STORAGE" android:maxSdkVersion="32" />` y `android:requestLegacyExternalStorage="true"` en [`android/AndroidManifest.xml`](../../android/AndroidManifest.xml).

#### Alternativas y compromisos
* **Alternativa descartada:** Integrar un selector de archivos visual Storage Access Framework (SAF) con `ACTION_OPEN_DOCUMENT`. Descartada en esta etapa por involucrar llamadas complejas a través de la capa JNI de Android sin aportar ventajas prácticas frente a la detección directa en `Downloads/`.
* **Alternativa descartada:** Distribuir o empaquetar ROMs de prueba dentro del APK. Descartada de raíz por violar la política clean-room y los derechos de autor de Nintendo (`AGENTS.md §3`).
* **Compromiso asumido:** Requiere que en Android 13+ el usuario conceda permisos si la carpeta no es accesible de forma predeterminada por la NativeActivity sin capa Java.

---

### 23. Prevención de Conflicto de Superficie GPU y Guía Visual en Framebuffer GBA para Android 16

#### Contexto
Al probar el APK en un dispositivo con Android 16, la pantalla quedaba completamente negra en el arranque. La investigación de la causa raíz reveló dos fallos concurrentes:
1. **Conflicto EGL/Vulkan vs. ANativeWindow_lock:** Cuando `Compositor::new` toma control de la ventana física para inicializar `wgpu`, la superficie queda acoplada al buffer queue de hardware. En Android NDK, cualquier intento posterior de invocar `ANativeWindow_lock` (renderizado por software en CPU) es rechazado por el sistema con error. Al activarse un aviso informativo de estado, `redraw()` llamaba a `render_software_error`, que fallaba silenciosamente y abortaba el dibujado con `return;`, impidiendo que el Compositor GPU ejecutara el pase de dibujado y congelando la pantalla en negro.
2. **Scoped Storage estricto en Android 16:** En Android 13-16, `std::fs::read_dir("/sdcard/Download")` devuelve `EACCES` (Permission denied) si la aplicación no posee permisos especiales o si el usuario no los otorgó manualmente.

#### Decisión e implementación
1. **Separación de responsabilidades de renderizado ([`src/platform.rs`](../../src/platform.rs)):** Se eliminó la intercepción de `render_software_error` cuando `self.compositor` está activo. `render_software_error` queda reservado estrictamente para caídas críticas donde `Compositor::new` no puede inicializarse (fallo real del adaptador gráfico).
2. **Guía interactiva en framebuffer virtual ([`src/storage.rs`](../../src/storage.rs)):** Se implementó `render_guide_screen`, que rasteriza tipografía nítida con `fontdue` directamente en el búfer `placeholder_rgba` ($240 \times 160$ píxeles). De esta forma, si la ROM aún no está cargada, el Compositor GPU presenta la carcasa completa de la GBA SP, el marco y los botones táctiles activos, mostrando las instrucciones en la pantalla virtual de la consola con cero pantallas negras.
3. **Sondeo directo de archivos y carpeta propia de la app:**
   - Se añadió `app.external_data_path()` (`/sdcard/Android/data/com.blazingrecomp.game/files/`) a la lista de búsqueda; esta ruta **nunca requiere permisos** en Android 11 a 16.
   - Si `read_dir` devuelve error de permisos, `scan_and_load_game` realiza sondeo directo de rutas fijas (`fe7.gba`, `Fire Emblem (Europe).gba`), permitiendo leer el archivo si el sistema otorga acceso de lectura directa.
4. **Declaración de permisos en manifiesto ([`android/AndroidManifest.xml`](../../android/AndroidManifest.xml)):** Se añadió `<uses-permission android:name="android.permission.MANAGE_EXTERNAL_STORAGE" />`.

#### Alternativas y compromisos
* **Alternativa descartada:** Crear una actividad Java previa para solicitar permisos interactivos en diálogo del sistema. Descartada para preservar la arquitectura de NativeActivity pura (`hasCode="false"`) sin sobrecarga del runtime DEX.
* **Compromiso asumido:** En Android 16, si el usuario prefiere colocar la ROM en `Downloads/` en vez de la carpeta propia de la app, puede requerir activar el interruptor de "Acceso a todos los archivos" en los ajustes de la aplicación si el sistema bloquea el sondeo directo.

---

## Alternativas y Compromisos Globales

A nivel arquitectónico y operativo, el proyecto se rige por compromisos transversales:
1. **Rendimiento nativo sobre facilidad de desarrollo:** Se priorizó el desarrollo en C11/Rust con `NativeActivity` en lugar de Java/Kotlin con JNI, garantizando 60 FPS estables y mínimo consumo de batería.
2. **Portabilidad y privacidad sobre conveniencia local:** Se prohibió cualquier ruta absoluta del sistema en archivos versionados, asegurando que el proyecto clone y compile en cualquier entorno.
3. **Robustez ante fallos sobre asunciones tempranas:** Se integró un intérprete de respaldo para instrucciones no mapeadas y una pantalla de diagnóstico en CPU para pánicos no controlados.
