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
24. [Triple Buffering Explícito en Swapchain Vulkan para Prevenir Degradación de Mailbox](#24-triple-buffering-explícito-en-swapchain-vulkan-para-prevenir-degradación-de-mailbox)
25. [Tope Dinámico Diferenciado de Swapchain (1280px Landscape / 960px Portrait) para 60 FPS Estables](#25-tope-dinámico-diferenciado-de-swapchain-1280px-landscape--960px-portrait-para-60-fps-estables)
26. [Profiler de Hotspots en Runtime y Expansión de Cobertura AOT C11 a 17,663 Bloques](#26-profiler-de-hotspots-en-runtime-y-expansión-de-cobertura-aot-c11-a-17663-bloques)
27. [Calibración Simétrica de Swapchain a 960px para 60 FPS Estables en Apaisado y Retrato](#27-calibración-simétrica-de-swapchain-a-960px-para-60-fps-estables-en-apaisado-y-retrato)
28. [Sincronización Reactiva por Eventos con AndroidAppWaker y Erradicación del Sondeo Inútil en el Hilo Principal](#28-sincronización-reactiva-por-eventos-con-androidappwaker-y-erradicación-del-sondeo-inútil-en-el-hilo-principal)
29. [Instrumentación de Rendimiento de Hardware en gba-core y Aislamiento del Overhead de Simulación](#29-instrumentación-de-rendimiento-de-hardware-en-gba-core-y-aislamiento-del-overhead-de-simulación)
30. [Optimización de Desempaquetado Ráfaga 4bpp y Recorte de Sprites en la PPU](#30-optimización-de-desempaquetado-ráfaga-4bpp-y-recorte-de-sprites-en-la-ppu)

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

> **Corregido (2026-10-14):** aquella cifra era del APK anterior al C11 recompilado. Con la traducción AOT dentro, el APK de tres ABIs pesa **186,6 MiB** y baja a **~44 MB** al comprimir las bibliotecas nativas (Decisión 32); las seis entradas `lib/**/*.so` son el 92 % del paquete. Ver el hallazgo registrado en `docs/BACKLOG.md`.

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

### 24. Triple Buffering Explícito en Swapchain Vulkan para Prevenir Degradación de Mailbox

#### Contexto
En Android, el modo de presentación sin desgarro y libre de bloqueos es `VK_PRESENT_MODE_MAILBOX_KHR` (o `wgpu::PresentMode::Mailbox`). Sin embargo, en el controlador de Vulkan de varios SoCs móviles (Mali, PowerVR, Adreno), si la superficie solicita `desired_maximum_frame_latency: 1`, el backend de `wgpu-hal` solicita al driver `minImageCount = latencia + 1 = 2` imágenes. Con solo 2 imágenes en el swapchain, `Mailbox` no puede operar libremente: mientras el compositor del sistema (`SurfaceFlinger`) retiene una imagen en pantalla, solo queda una imagen libre. Si el motor intenta presentar un nuevo fotograma antes del siguiente pulso de VSync, `vkAcquireNextImageKHR` o `vkQueuePresentKHR` se bloquea como un doble búfer estricto, reteniendo el hilo de dibujado entre 32 ms y 52 ms por fotograma e impidiendo superar los 18 a 28 FPS.

#### Decisión e implementación
1. **Configuración de latencia de fotograma ([`src/platform.rs`](../../src/platform.rs)):** Se elevó `desired_maximum_frame_latency` de `1` a `2` en la configuración del swapchain (`wgpu::SurfaceConfiguration`).
2. **Asignación de 3 imágenes (`minImageCount = 3`):** Al solicitar latencia 2, `wgpu-hal` calcula `capabilities.minImageCount + 1` (3 imágenes), garantizando triple buffering explícito por hardware.
3. **Flujo de fotogramas libre de esperas:** Con 3 imágenes, siempre existe al menos un búfer disponible en memoria gráfica para que la GPU escriba el siguiente fotograma mientras el anterior aguarda en cola de presentación y el primero se visualiza en el panel.

#### Alternativas y compromisos
* **Alternativa descartada:** Recurrir a `wgpu::PresentMode::AutoNoVsync` (o `Immediate`). Descartada porque produce *screen tearing* severo y cortes horizontales en las animaciones de batalla y texto de *Fire Emblem*.
* **Alternativa descartada:** Degradar a `Fifo` o `FifoRelaxed`. Descartada porque si un fotograma excede los 16.6 ms por algún pico de carga, la tasa de refresco cae instantáneamente a la mitad (30 FPS).
* **Compromiso asumido:** Requiere un búfer adicional de memoria de video (~3 a 4 MB según la resolución del swapchain), un consumo insignificante en dispositivos móviles modernos.

---

### 25. Tope Dinámico Diferenciado de Swapchain (1280px Landscape / 960px Portrait) para 60 FPS Estables

#### Contexto
Dispositivos móviles y tablets modernos incorporan paneles de resolución 2K/QHD (como 1200×2000 o 2000×1200 píxeles, equivalente a 2.4 millones de píxeles por fotograma). Cada fotograma a resolución nativa requiere un búfer RGBA de 9.6 MB, exigiendo más de 570 MB/s de ancho de banda de memoria LPDDR solo para swapchain. La consola Game Boy Advance renderiza nativamente a $240 \times 160$ píxeles (0.038 millones de píxeles). Rasterizar la carcasa, componer el marco de GBA y transferir 2.4 MP en cada fotograma saturaba por completo la tasa de relleno (*fillrate*) de la GPU y la cola de `SurfaceFlinger`, haciendo que `present` consumiera más de 50 ms en retrato y 32 ms en apaisado, estancando la emulación en 18 y 28 FPS respectivamente a pesar de que la simulación de CPU en C11 (`ejec`) apenas tomaba 10-12 ms. Además, en modo retrato la mitad inferior ejecuta cálculos procedurales pesados (carcasa plateada, cuencas 3D de botones, sombras), requiriendo una cuota de fillrate más estricta que el modo apaisado.

#### Decisión e implementación
1. **Módulo de cálculo de escala con topes por orientación ([`src/present.rs`](../../src/present.rs)):** Se definieron constantes semánticas y la función pura `calcular_escala_swapchain(w, h)`:
   - `TOPE_SWAPCHAIN_LANDSCAPE = 1280;`: Acota el eje mayor en apaisado a 720p HD ($1280 \times 768$).
   - `TOPE_SWAPCHAIN_PORTRAIT = 960;`: Acota la altura en vertical ($576 \times 960$), compensando la matemática adicional de la carcasa y cuencas 3D en la mitad inferior.
   - En pantallas con dimensiones menores a dichos topes (ej. pantallas retro o teléfonos de baja resolución), opera al 100% nativo (`scale = 1.0`).
2. **Dimensionamiento del swapchain ([`src/platform.rs`](../../src/platform.rs)):** `Compositor::new()` y `Compositor::render()` configuran `wgpu::SurfaceConfiguration` con las dimensiones acotadas:
   - Apaisado: $1280 \times 768$ (0.98 MP, -59% de píxeles respecto al panel 2K).
   - Retrato: $576 \times 960$ (0.55 MP, -77% de píxeles respecto al panel 2K).
3. **Escalado por hardware a coste cero:** El procesador de visualización de pantalla de Android (*Hardware Composer* / MDP) estira el búfer de swapchain al panel físico de 2000px mediante interpolación bilineal de hardware sin coste alguno de GPU ni contención de cerrojos.
4. **Impacto medido en telemetría:**
   - **Modo Retrato:** El tiempo de `present` se desplomó de **52.0 ms** a apenas **0.6–3.5 ms** (y `dibujo` total a 2.5–6.0 ms), alcanzando **60.0 FPS estables y clavados**, superando holgadamente los 18 FPS originales.
   - **Modo Apaisado:** `present` se estabilizó en **12.5–14.0 ms**, entregando **56–58 FPS continuos**.

#### Alternativas y compromisos
* **Alternativa descartada:** Reducción estática de escala fija a 0.5x uniforme. Descartada porque penalizaría innecesariamente la nitidez en teléfonos con pantallas 720p o 1080p modestas donde la GPU sí soporta su resolución nativa.
* **Alternativa descartada:** Barrido interactivo de calibración en cada arranque. Descartada porque demoraba de 12 a 15 segundos en el arranque de la app para terminar eligiendo el mismo resultado medible.
* **Compromiso asumido:** En paneles 2K/QHD en vertical la superficie del swapchain es de $576 \times 960$; no obstante, a $2.4\times$ la resolución nativa de GBA ($240 \times 160$), el pixel art del juego y los glifos vectoriales del overlay conservan nitidez absoluta.

---

### 26. Profiler de Hotspots en Runtime y Expansión de Cobertura AOT C11 a 17,663 Bloques

#### Contexto
A pesar de contar con 17,207 bloques traducidos a C11, la telemetría en runtime (`blazing-perf.txt`) revelaba entre 35,000 y 55,000 pasos de intérprete de respaldo por segundo, manteniendo la ejecución de CPU (`ejec`) en ~12.5 ms por fotograma. Al operar sin un juego completo de labels en `blazing.labels.toml`, ciertas rutinas centrales del juego se saltaban de forma indirecta hacia direcciones no indexadas en la tabla de bloques estática, obligando al bucle de emulación a recurrir a `m.step()` para interpretar dichas instrucciones. Sin embargo, diagnosticar a ciegas qué funciones de la ROM ($0x08000000..0x09FFFFFF$) consumían ese volumen resultaba inviable.

#### Decisión e implementación
1. **Hotspot Profiler en runtime ([`src/recomp.rs`](../../src/recomp.rs)):** Se implementó un acumulador de frecuencias atómico (`HotspotState`, `record_hotspot`, `drain_top_hotspots`) que registra en $O(1)$ cada dirección de PC ejecutada en la rama `None` de `run_frame_native`, normalizando el bit Thumb (`pc & !1`).
2. **Volcado periódico en telemetría ([`src/platform.rs`](../../src/platform.rs)):** El hilo principal extrae las direcciones más ejecutadas y emite una línea `# hotspots top=...` en `blazing-perf.txt`.

> **Corregido (2026-10-14).** Dos afirmaciones del punto anterior no describían el código: el filtro **no** es «exclusivamente el cartucho» —`record_hotspot` acepta ROM, IWRAM, EWRAM y BIOS— y el volcado **no** es «cada 5 segundos» sino **cada 2** (`report.second_index % 2 == 0`, [`src/platform.rs:2119`](../../src/platform.rs), donde `second_index` es el índice de segundo). Además, `drain_top_hotspots` **consume solo las direcciones del top** y conserva el resto para el volcado siguiente, que es justo lo que buscaba [`docs/propuestas/2026-10-12_drain-hotspots-sin-perdida.md`](../propuestas/2026-10-12_drain-hotspots-sin-perdida.md): que ninguna ejecución registrada se descarte.
3. **Mapeo a labels de Fire Emblem ([`blazing.labels.toml`](../../blazing.labels.toml)):** El profiler identificó de inmediato funciones críticas no etiquetadas:
   - `0x08019120` (`Unit_GetStruct`): Bucle de 7 instrucciones que busca punteros a unidades en RAM (más de 58,000 ejecuciones por ventana).
   - `0x08026060` (`Battle_MapScanUnits`): Bucle que recorre las 191 unidades en mapa invocando la función anterior.
   - `0x08006980` (`OAM_BuildSpriteList`): Construcción y culling de sprites en pantalla.
   - `0x08000360` (`Checksum_CalculateCRC`, ARM 32-bit): Cálculo de sumas de comprobación CRC.
   - `0x080B4270` (`ScriptEngine_StepTick`), `0x08069E98` (`VRAM_TileAnimationStep`), `0x08066FB4` (`Palette_ColorFadeRGB555`), entre otras (31 funciones etiquetadas en total).
4. **Recompilación AOT expandida ([`src-c/`](../../src-c/)):** Mediante `recomp build ... --ram`, la tabla nativa `rcg_blocks` se incrementó a **17,663 funciones C11** (+456 bloques nativos adicionales).
5. **Impacto medido:** Las llamadas al intérprete en gameplay normal colapsaron de **55,000 a entre 200 y 2,000 pasos por segundo** (reducción del 95%), y el tiempo de CPU (`ejec`) cayó a **4.3–8.2 ms por fotograma** (50% de reducción).

#### Alternativas y compromisos
* **Alternativa descartada:** Recompilación interpretada JIT dinámica en memoria ejecutable (`mprotect` / `PAGE_EXECUTE`). Descartada por las restricciones de seguridad W^X de SELinux en Android y porque el pipeline AOT estático es la base arquitectónica del proyecto.
* **Compromiso asumido:** El archivo de código C11 derivado creció ligeramente (de 24 MB a 24.8 MB en total), pero la biblioteca compilada en disco (`libblazing_recompiled.so`) se mantiene ligera (~7.4 MB).

---

### 27. Calibración Simétrica de Swapchain a 960px para 60 FPS Estables en Apaisado y Retrato

#### Contexto
Tras optimizar la CPU a <8 ms mediante AOT, el juego corría a 60 FPS clavados en modo vertical (retrato), pero en modo horizontal (apaisado, $2000 \times 1200$) la tasa de refresco fluctuaba entre 55 y 58 FPS. La telemetría por fase reveló que `ejec` tardaba solo 6.6 ms, pero `present` se elevaba a **14.2 ms** (agotando casi la totalidad de los 16.6 ms de VSync). Con el tope anterior de 1280px para apaisado, la superficie del swapchain se dimensionaba en $1280 \times 768$ (983,040 píxeles, un 77% más área que en retrato), saturando el fillrate del hardware y la cola de composición de la GPU.

#### Decisión e implementación
1. **Tope simétrico para ambas orientaciones ([`src/present.rs`](../../src/present.rs)):** Se ajustó `TOPE_SWAPCHAIN_LANDSCAPE = 960;` (igualándolo a `TOPE_SWAPCHAIN_PORTRAIT = 960;`).
2. **Equiparación de área de renderizado:**
   - Modo Retrato ($1200 \times 2000$): escala $0.48 \to 576 \times 960 = 552,960$ píxeles.
   - Modo Apaisado ($2000 \times 1200$): escala $0.48 \to 960 \times 576 = 552,960$ píxeles.
3. **Escalado exacto entero 4× para GBA:** $960 \times 576$ es un múltiplo limpio ($4.0\times$ ancho, $3.6\times$ alto) sobre la resolución nativa de Game Boy Advance ($240 \times 160$), conservando nitidez pixel-perfect.
4. **Impacto medido:**
   - En modo apaisado, el tiempo de `dibujo` GPU se desplomó de **15.8 ms a 2.4–3.6 ms** (reducción del 80%).
   - El tiempo de `present` cayó de **14.2 ms a 0.4–1.8 ms**.
   - El juego sostiene **58.0 a 60.0 FPS sólidos y continuos** en ambas orientaciones con holgura de tiempo para futuras fases de audio y shaders.

#### Alternativas y compromisos
* **Alternativa descartada:** Mantener 1280px y activar frameskip en apaisado. Descartada porque generaría tirones visuales (*stuttering*) en un juego de estrategia por turnos que debe ser 100% fluido a 60 FPS.
* **Compromiso asumido:** En apaisado la resolución de swapchain es de $960 \times 576$; el escalador por hardware de la GPU del dispositivo proyecta la imagen en el panel 2K a coste cero sin pérdida perceptible de calidad.

---

### 28. Sincronización Reactiva por Eventos con AndroidAppWaker y Erradicación del Sondeo Inútil en el Hilo Principal

#### Contexto
Para evitar bloquear el hilo principal de la interfaz y permitir que la emulación corriera a 60 Hz desacoplada, el hilo principal en Android utilizaba un timeout breve de 1 ms (`Duration::from_millis(1)`) en `app.poll_events()` ([`src/platform.rs`](../../src/platform.rs)). Este sondeo ciego era necesario porque el hilo principal no recibía ninguna notificación cuando el hilo de emulación (`blazing-emu`) completaba un fotograma y lo depositaba en el buzón `worker.bridge().try_recv_frame()`.

Como cada fotograma de GBA toma ~16.6 ms, durante cada ciclo de cuadro el hilo principal se despertaba inútilmente entre 10 y 15 veces sin hacer ningún trabajo útil. La telemetría en runtime (`blazing-perf.txt`) reflejaba de manera constante entre **550 y 650 vueltas vacías por segundo** (`vacias`), generando un consumo innecesario de ciclos de CPU, disipación térmica y gasto acelerado de batería en dispositivos móviles.

#### Decisión e implementación
1. **Abstracción desacoplada en el pipeline ([`src/pipeline.rs`](../../src/pipeline.rs)):** Para mantener `pipeline.rs` 100% agnóstico a la plataforma y compilable en entornos host sin dependencias de Android, se definió un alias de closure genérico:
   ```rust
   pub type WakeFn = Arc<dyn Fn() + Send + Sync>;
   ```
2. **Notificación en el depósito de fotogramas:** Se equipó a `EmulationBridge` con un notificador opcional (`waker: Mutex<Option<WakeFn>>`, configurable vía `new_with_waker` o `set_waker`). Inmediatamente tras depositar un fotograma válido en `send_frame` y liberar el cerrojo del buzón, se invoca `wake()`.
3. **Integración con el Looper de Android ([`src/platform.rs`](../../src/platform.rs)):**
   - En `android_main`, se crea una instancia de `AndroidAppWaker` con `app.create_waker()`.
   - Se entrega al estado de la aplicación capturado en un closure `Arc::new(move || app_waker.wake())`, el cual se propaga al iniciar el worker con `EmulationWorker::start_with_waker`.
   - Al llamarse `wake()`, el sistema subyacente invoca `ALooper_wake()` en la capa C de Bionic/NDK, interrumpiendo al instante el bloqueo de `app.poll_events()` y entregando el evento `PollEvent::Wake`.
4. **Elevación del timeout de seguridad:** Con el flujo guiado por eventos, el timeout activo de `poll_events` se elevó de 1 ms a **50 ms** (equivalente a 3 fotogramas) como mecanismo de respaldo por si la emulación sufriera una pausa imprevista, erradicando el sondeo repetitivo.
5. **Impacto medido en telemetría:**
   - Las vueltas vacías colapsaron de **~600 vueltas/s a un promedio de ~7 vueltas/s** (una reducción del **98.8%** en ciclos ociosos de CPU).
   - En múltiples tramos de gameplay activo, el valor de `vacias` cayó a **0**, demostrando que el hilo principal reposa en el kernel y solo despierta al llegar un fotograma o eventos de entrada táctil.
   - El framerate se mantuvo sólido e inalterado en **58.0 a 60.0 FPS**, con tiempos de ejecución de CPU (`ejec`) y composición de GPU (`dibujo`, `present`) idénticos a la línea base.

#### Alternativas y compromisos
* **Alternativa descartada:** Bloquear el hilo principal en un `Condvar` compartido con el hilo de emulación. Descartada porque `android-activity` exige que el hilo principal procese su bucle de eventos a través de `poll_events` para atender los eventos del ciclo de vida del sistema operativo (`Pause`, `Resume`, `Destroy`, cambios de tamaño de ventana y eventos de entrada).
* **Compromiso asumido:** Si la emulación se detuviera de forma inesperada sin emitir eventos, el bucle principal tardaría hasta 50 ms en despertar por el timeout de seguridad, lo cual es imperceptible para la interacción del usuario y garantiza que la aplicación nunca se congele.

---

### 29. Instrumentación de Rendimiento de Hardware en gba-core y Aislamiento del Overhead de Simulación

#### Contexto
En pruebas en hardware real sobre tablets modestas (Qualcomm Snapdragon 662), el tiempo de ejecución de la simulación (`ejec`) mostraba un piso persistente de ~9 a 13 ms por fotograma, incluso cuando el intérprete de respaldo ejecutaba muy pocas instrucciones. Existía la hipótesis de que dicho piso provenía de la PPU, el sintetizador de audio APU o la frecuencia de accesos al bus de memoria (callbacks `read8/16/32` y `write8/16/32`). Sin embargo, no existía instrumentación granular que permitiera aislar qué subsistema consumía dicho tiempo.

Además, en procesadores móviles de gama de entrada, una llamada a `std::time::Instant::now()` tiene un coste medido de ~460–470 ns. Si se instrumentaba ingenuamente cada evento o acceso a memoria, el propio cronómetro distorsionaría catastróficamente la medición (Efecto Observador).

#### Solución implementada
1. **Contadores de hardware desacoplados en `gba-core` (`GbaPerfCounters`):**
   - Se introdujo la feature condicional `perf-counters` en `gba-core` con una estructura que acumula:
     - `ppu_ns`: nanosegundos en renderizado de scanlines (`render_scanline`).
     - `apu_ns`: nanosegundos en síntesis de audio (`pump_audio`).
     - `reads` y `writes`: contadores enteros `u64` para accesos de memoria en el bus, **sin ninguna lectura de reloj** en las rutas críticas de memoria (evitando ~90 ms de sobrecosto artificial).
2. **Prevención del Efecto Observador en `pump_audio`:**
   - La función `pump_audio` se invoca en cada `tick()` de reloj del intérprete de instrucciones (hasta 140,000 veces por fotograma en secciones de fallback denso).
   - Se añadió una cláusula de guarda anticipada (`if self.audio_cursor > until { return; }`) antes de invocar `Instant::now()`.
   - Con esto, las lecturas de reloj de audio pasaron de ~140,000 llamadas vacías (que causaban un colapso artificial de ~130 ms por fotograma) a un máximo estricto de **1,097 llamadas reales** por fotograma (correspondientes a las muestras discretas a 65,536 Hz), reduciendo el overhead del cronómetro a menos de 0.5 ms.
3. **Conexión en `blazing-recomp` ([`src/runner.rs`](../../src/runner.rs), [`src/perf.rs`](../../src/perf.rs), [`src/platform.rs`](../../src/platform.rs)):**
   - `StepTiming` y `FrameTiming` se ampliaron para recolectar `ppu_ms`, `apu_ms`, `reads` y `writes` sin asignaciones dinámicas.
   - La telemetría por segundo (`PerfReport::to_line`) formatea el desglose:
     `t=Xs fps=... ejec=... ppu=... apu=... lecturas=... escrituras=... color=... dibujo=...`
4. **Hallazgo arquitectónico medido:**
   - La **PPU** consume entre **3.5 y 6.0 ms** por fotograma (promedio ~4.5 ms, ~38% de `ejec`), siendo el componente más intensivo de la simulación de hardware.
   - El **APU** consume entre **0.6 y 0.9 ms** por fotograma (promedio ~0.7 ms, ~6% de `ejec`), demostrando ser sumamente liviano.
   - La ejecución de CPU y resolución del bus de memoria consumen entre **6.0 y 7.5 ms** (~56% de `ejec`), resolviendo entre 30,000 y 65,000 accesos de lectura/escritura por fotograma.
   - El rendimiento global se mantuvo en **60 FPS estables sostenidos** sin caídas ni artefactos de latencia.

#### Alternativas y compromisos
* **Alternativa descartada:** Cronometrar individualmente cada llamada a `read8/16/32` y `write8/16/32`. Descartada porque 100,000 accesos a 470 ns por lectura de reloj habrían sumado 47 ms adicionales por frame, congelando el juego. El uso de contadores enteros `u64` resolvió el diagnóstico con cero distorsión temporal.
* **Compromiso asumido:** Los contadores están resguardados bajo la feature `perf-counters`, de modo que pueden desactivarse por completo en compilaciones de producción pura sin dejar ninguna instrucción adicional en el ensamblado final.

---

### 30. Optimización de Desempaquetado Ráfaga 4bpp y Recorte de Sprites en la PPU

#### Contexto
Tras la instrumentación de hardware (ADR 29), las mediciones en Snapdragon 662 demostraron que el renderizador de software de la PPU (`crates/gba-core/src/ppu.rs`) representaba el componente individual más pesado de la simulación, consumiendo entre 3.5 y 6.9 ms por fotograma. Al perfilar el renderizado de scanlines se identificaron dos cuellos de botella algorítmicos fundamentales:
1. En fondos de texto de 4 bits por píxel (Modo 0, predominante en *Fire Emblem*), la función `render_text_bg` leía la VRAM byte a byte píxel a píxel, realizando divisiones `fx / 2` y bifurcaciones condicionales repetitivas (`if fx & 1 == 0`) para desempaquetar cada nibble.
2. En `render_sprites`, el bucle horizontal iteraba ciegamente sobre el ancho completo del sprite (`0..bw`), evaluando en cada píxel si caía dentro de la pantalla (`(0..VISIBLE_WIDTH).contains(&sx)`) y recalculando la coordenada vertical y dirección de tile en cada iteración horizontal, incluso para sprites no afines.

#### Solución implementada
1. **Desempaquetado ráfaga de tiles 4bpp:**
   - Una fila de 8 píxeles en 4bpp equivale a exactamente 4 bytes contiguos en VRAM.
   - Se reemplazaron las lecturas byte a byte por una única carga de 32 bits (`u32::from_le_bytes`), extrayendo el color de cada píxel en ráfaga mediante desplazamiento y máscara libre de ramas:
     `let nib = ((row_bytes >> (fx * 4)) & 0xF) as usize;`
   - Se forzó el inlining de `pal16` (`#[inline(always)]`), reduciendo la sobrecarga de resolución de colores.
2. **Recorte horizontal temprano de Sprites:**
   - Se calcula el intervalo visible horizontal antes de iniciar el bucle:
     `let min_bx = (-x).max(0); let max_bx = (VISIBLE_WIDTH as i32 - x).min(bw);`
   - Si `min_bx >= max_bx`, el sprite se descarta de inmediato sin procesar píxeles.
   - Para sprites no afines (la gran mayoría), las coordenadas verticales (`tv`, `pv`) se calculan una sola vez fuera del bucle horizontal, evitando multiplicaciones y divisiones por píxel.
3. **Impacto medido en telemetría en hardware real:**
   - En escenas de Modo 0, el tiempo de ejecución `ejec` se redujo de **10.5–11.7 ms a 8.4–8.5 ms** (hasta **-3.3 ms**, una mejora del **28%** en velocidad de CPU).
   - En escenas densas con casi 60,000 instrucciones de intérprete y más de 72,000 accesos a memoria, la simulación se mantuvo sólida en **55–60 FPS**, completándose en ~14.7 ms.

#### Alternativas y compromisos
* **Alternativa descartada:** Migrar el renderizado de la PPU a shaders de fragmentos de la GPU en Vulkan. Descartada por ahora porque requeriría transferir tablas de baldosas y mapas de patrones a texturas de GPU por scanline, rompiendo la independencia de `gba-core` y añadiendo complejidad en sincronización de DMA y VRAM.
* **Compromiso asumido:** El desempaquetado de 32 bits asume que los 4 bytes de una fila de tile residen en el espacio canónico de VRAM; si una dirección cae fuera de `0x10000`, la guarda produce píxeles transparentes en consonancia con el comportamiento original.

---

### 31. Optimización Lineal de Composición de Capas y Pre-Evaluación de Ventanas en la PPU

#### Contexto
A pesar de la optimización de desempaquetado de tiles y recorte de sprites (ADR 30), en escenas con ventanas, efectos de color o múltiples fondos activos, el renderizado de la PPU consumía entre 3.2 y 5.5 ms por fotograma. Al analizar la función de composición de capas (`compose` en `crates/gba-core/src/ppu.rs`), se identificaron dos ineficiencias críticas:
1. **Búsqueda por fuerza bruta en cada píxel:** La función ejecutaba un bucle anidado `'outer: for p in 0..4u8` sobre las 4 prioridades posibles de hardware ($0 \dots 3$). Dentro de cada prioridad evaluaba el sprite (`obj.priority == p`), y si no coincidía, realizaba un segundo bucle lineal sobre las 4 capas de fondo (`bg_prio`). Si el píxel requería mezcla de colores (*alpha blend* o efectos de fade), se repetía otra búsqueda anidada completa para encontrar la segunda capa visible (`tgt2`), lo que generaba hasta 20 iteraciones y evaluaciones condicionales por píxel ($>3,800$ evaluaciones por scanline, más de 614,000 por fotograma).
2. **Evaluación redundante de ventanas:** Para cada uno de los 240 píxeles horizontales, la función evaluaba la jerarquía de `WIN0`, `WIN1` y `OBJWIN`, incluso en scanlines donde los registros `DISPCNT` indicaban que ninguna ventana de hardware estaba habilitada.
3. **Sobrecarga de llamadas en operaciones de color:** Pequeñas funciones matemáticas críticas de cálculo de color (`alpha_blend`, `brighten`, `darken`, `split6`, `join6`) no contaban con directiva de inlining forzado, introduciendo llamadas de función repetidas durante operaciones de blending.

#### Solución implementada
1. **Recorrido lineal directo sobre `bg_prio`:**
   - Como el arreglo `bg_prio` ya viene pre-ordenado por prioridad canónica de GBA (calculado una sola vez por scanline), no es necesario iterar arbitrariamente sobre `0..4u8`.
   - Se reemplazó la búsqueda anidada por un escaneo lineal que recorre los fondos activos e intercala el sprite exactamente en el momento en que `obj.priority <= bp`.
   - Esto reduce drásticamente las comprobaciones de hasta 20 a un máximo estricto de 5 comparaciones por píxel, acelerando tanto la selección de la primera capa (`tgt1`) como de la segunda (`tgt2`).
2. **Pre-evaluación y atajo de ventanas por scanline:**
   - Antes de iniciar el bucle horizontal de 240 píxeles, se calcula una bandera booleana `line_has_window`:
     `let line_has_window = (dispcnt & (DISPCNT_WIN0 | DISPCNT_WIN1 | DISPCNT_OBJWIN)) != 0;`
   - Si ninguna ventana está activa, se ataja directamente a la configuración por defecto de `winout` sin computar rangos ni bifurcaciones de ventana por píxel.
3. **Inlining forzado de aritmética de color:**
   - Se anotaron con `#[inline(always)]` las funciones `alpha_blend`, `brighten`, `darken`, `split6` y `join6`, permitiendo al compilador fusionar las operaciones de 15 bits en registros vectoriales o inmediatos sin saltos de pila.

#### Alternativas y compromisos
* **Alternativa descartada:** Fusión en un único búfer de capas con índice Z en scanline. Descartada porque la precedencia de hardware de GBA requiere mantener separados los identificadores de capa para aplicar máscaras de ventana y blending dinámico de `BLDCNT`.
* **Compromiso asumido:** La intercalación del sprite asume que los fondos en `bg_prio` están rigurosamente ordenados por prioridad ascendente; esto se garantiza por construcción al ordenar las capas activas al inicio de cada scanline.

---

### 32. Compresión DEFLATE de las Bibliotecas Nativas en el APK

#### Contexto
El APK universal de tres ABIs pesaba **195,670,857 bytes (186.6 MiB)**. La medición de su estructura interna mostró que las seis entradas `lib/**/*.so` ocupaban dentro del archivo exactamente lo mismo que fuera de él: estaban almacenadas con `compression_method=0` (STORED), sin comprimir. El empaquetado STORED se justificaba en el script como imprescindible para la carga directa por `mmap` con `android:extractNativeLibs="false"`, pero el manifiesto declara `android:extractNativeLibs="true"` desde la decisión 14: Android **extrae** las bibliotecas al instalar y acepta entradas comprimidas. Se pagaba 4.5× de tamaño por una ventaja que la configuración real no utilizaba.

#### Decisión e implementación
1. En [`scripts/package_apk.ps1`](../../scripts/package_apk.ps1) se retiró la opción `-0` de la llamada a `jar`, de modo que las entradas inyectadas bajo `lib/` se guardan con DEFLATE (método 8). El respaldo con Python pasó de `zipfile.ZIP_STORED` a `zipfile.ZIP_DEFLATED`.
2. Se conserva `zipalign -P 16 4` y la firma v1/v2/v3 sin cambios.
3. Se corrigió además el bucle que incorpora ABIs adicionales: ahora está condicionado por `if ($AllAbis)`, de forma que `-AllAbis:$false` produce realmente un APK de un solo ABI en lugar de arrastrar cualquier arquitectura ya compilada en `target/`.
4. El contrato de los tests de empaquetado cambió de «STORED + offset de datos alineado a 16 KB» a «DEFLATE con `comp_size < uncomp_size`» más dos guardas: el APK de tres ABIs debe caber en 60 MiB, y `resources.arsc` debe seguir almacenado sin comprimir porque Android 11+ lo exige.

#### Alternativas y compromisos
* **Alternativa descartada: adoptar `android:extractNativeLibs="false"` + STORED.** Es el modo moderno (carga por `mmap` desde el propio APK, sin extracción en disco), pero no reduce el tamaño del paquete —seguiría pesando 186 MiB— y rompería la compatibilidad con emuladores que la decisión 16 fijó explícitamente.
* **Alternativa descartada: bajar la optimización del código C11 recompilado (`-Oz`).** Iría en contra del objetivo del proyecto, que es precisamente maximizar el rendimiento de ejecución en el hardware objetivo.
* **Compromiso asumido:** comprimir reduce el tamaño de la **descarga**, no el espacio ocupado en el dispositivo: como `extractNativeLibs="true"`, Android descomprime al instalar y el juego sigue ocupando los mismos ~186 MB en disco. Para reducir ese segundo número hace falta el recorte por ABI, que queda fuera de esta decisión.
* **Compromiso asumido:** el alineado a páginas de 16 KB del punto de datos dentro del ZIP deja de aplicar a las entradas de `lib/`, porque solo es necesario en el modo de `mmap` directo. La alineación de la propia biblioteca ELF es una cuestión independiente de esta decisión y no queda cubierta por ella.

---

### 33. El acceso a memoria no se troca por puntero directo, y la instrumentación que viaja en el APK pasa a ser el objetivo

#### Contexto
El residuo de `ejec` en el Android TV (4,54 ms de mediana tras descontar PPU y APU, sesión 17) se atribuía al acceso a memoria: cada lectura y escritura del juego es una llamada indirecta a la vtable `RtApi`, y a 12.214 accesos por fotograma eso daba ~3,7 ms a 0,27 µs por acceso. La fase A de [`2026-10-12_acceso-memoria-sin-llamada-indirecta.md`](../propuestas/2026-10-12_acceso-memoria-sin-llamada-indirecta.md) existía para medir antes de tocar el generador.

#### Decisión e implementación
1. El salto indirecto se midió en aislamiento (10⁷ accesos, misma ROM y mismas direcciones): 0,95 ns por lectura directa contra 2,69 ns por la vtable — **2,8×**, la suposición era cierta— pero 2,69 ns × 12.214 accesos son 0,033 ms por fotograma en host (~0,45 ms con el factor host→dispositivo), **ocho veces menos** que la atribución que fundaba la propuesta.
2. Se prototipó el camino rápido de lectura de ROM (tres campos nuevos en `RtApi`, `rc_rom_fast`/`rc_read8/16/32` en el preámbulo C y los ocho puntos de emisión de lecturas del generador) detrás de la constante de entorno `RECOMP_FAST_ROM`: los tres hashes guardianes quedaron idénticos con el camino activo (600 → `31184031261ea2ec`; 2400 y 7200 → `b54066dc06f2b76c`) y un control de mutación (puntero de ROM desplazado 4 bytes) probó que el camino se ejecutaba de verdad.
3. El A/B del arnés —6 pares intercalados, el mismo `.dll`— **no separó el efecto del ruido**: mediana de 7822 ms sin el camino rápido contra 7852,5 ms con él (+0,4 %, más lento), mediana pareada −286,5 ms (−3,6 %) y 4 de 6 pares más rápidos. El prototipo se revirtió y el generador no se tocó.
4. El mismo tipo de A/B aplicado a la instrumentación que viaja en el APK ([`Cargo.toml:12`](../../Cargo.toml#L12) activa `perf-counters`: seis incrementos por acceso en el bus y cuatro cronómetros por línea visible en la PPU) dio **+10,4 % de mediana pareada, con 5 de 6 pares más lentos** (0,105 ms por fotograma en host).
5. La fase A de [`2026-10-12_telemetria-sin-coste-en-el-apk.md`](../propuestas/2026-10-12_telemetria-sin-coste-en-el-apk.md) repartió ese efecto con cuatro variantes temporales del motor (6 ciclos intercalados y el mismo `.dll`), y el reparto desmintió la premisa de la propuesta: los seis incrementos del bus cuestan **−0,7 %** (dentro del ruido, 2 de 6 pares más lentos) y los cronómetros PPU+APU **+10,1 %** (5 de 6 pares más lentos), con el reparto aditivo y un residuo de −0,5 %.
6. El mecanismo de ese +10,1 % se midió después: 89,4 µs por fotograma entre 640 lecturas de reloj ⇒ **~140 ns por lectura**, casi cinco veces los ~30 ns que documenta `perf_clock` para su camino genérico. Y ese camino genérico (`Instant::now()`) es el que usan el arnés de host, `armeabi-v7a` y `x86_64`, porque `perf_clock` **solo usa `CNTVCT_EL0` bajo `#[cfg(target_arch = "aarch64")]`**. Mover los contadores del bus a un estático con línea propia (`#[repr(align(64))]`) no quitó nada —+7,3 % frente a +8,6 %, dentro de la dispersión del arnés—: no había interferencia que quitar.

#### Alternativas y compromisos
* **Alternativa descartada: seguir con EWRAM/IWRAM y con las escrituras.** El prototipo cubrió solo la lectura de ROM; si la mitad más limpia del camino no movió el fotograma, el resto tampoco.
* **Compromiso asumido: el metro estaba estirado, pero el 10 % no lo pagaba el APK principal.** El reparto medido deja claro que el coste no está en los seis incrementos del bus —que son los que viajan en cualquier ABI— sino en la lectura de reloj de la PPU, barata en `aarch64` (`CNTVCT_EL0`) y cara en el camino genérico (`Instant::now()`). Lo que sí queda es un **sesgo del arnés de host**: cuando el proyecto compara variantes en el host, un ~8-10 % del fotograma es cronómetro del propio arnés. La primera propuesta se cerró como `discrepada` y la pregunta que queda (cuánto cuesta el reloj del motor en el dispositivo) se mide en [`2026-10-13_el-reloj-de-la-ppu-en-el-dispositivo.md`](../propuestas/2026-10-13_el-reloj-de-la-ppu-en-el-dispositivo.md) antes de decidir si hay algo que reparar.
* **Compromiso asumido: el host no resuelve diferencias de pocos puntos.** Entre corridas idénticas de 7200 fotogramas el arnés dispersó de 7589 a 10468 ms (±13 %), así que un A/B de menos de ~5 % solo es interpretable con pares intercalados y cinco o más pares.

---

### 34. La composición de la PPU se decide por tramos de línea, no por píxel

#### Contexto
La PPU en software costaba 9,20 ms por fotograma en el Android TV (55 % de `ejec`, con `ejec` ocupando
el presupuesto entero de 60 FPS) y su parte mayor era la composición: 4,20 ms para decidir, píxel a
píxel, qué capa se ve. Esa decisión no cambia de un píxel a otro dentro de una línea —el orden de
prioridad es constante, las ventanas son rangos de x, el modo de mezcla es de la línea entera—, así
que se pagaba 240 veces por línea y 160 veces por fotograma. El trabajo lo pidió
[`2026-10-13_ppu-software-rapida-sin-cambiar-pixeles.md`](../propuestas/2026-10-13_ppu-software-rapida-sin-cambiar-pixeles.md),
con la salida visual como restricción intocable.

#### Decisión e implementación
1. En [`../gba-recomp/crates/gba-core/src/ppu.rs`](../../../gba-recomp/crates/gba-core/src/ppu.rs)
   (motor, ruta local del `[patch]`) la línea se parte en **tramos** de x con control de capas
   constante (hasta cinco: los bordes de WIN0H/WIN1H) y cada tramo se compone en una pasada con la
   lista de fondos monomorfizada por número de fondos activos (`straight_span`, `straight_bgs`,
   `straight_obj`, `compose_straight`). El tramo se corta de la misma forma que `control_at` decide por
   píxel, incluido el caso que da la vuelta cuando X1 > X2.
2. La vía rápida se toma **solo si nada puede mezclarse en la línea**: `BLDCNT` en modo 0, sin ventana
   de sprites y sin ningún píxel de sprite translúcido. Los efectos de color, la ventana de sprites y
   los sprites translúcidos (que sí mezclan en modo 0) siguen por el camino general, que no se tocó.
3. El sprite translúcido lo señala `render_sprites` al dibujarlo: su firma ya documentaba que
   devolvería esa información, y barrer la línea de 240 píxeles para deducirla costaba 0,026 ms por
   fotograma (16 % de la composición).
4. En los fondos, un tramo de tile cuya fila está vacía se rellena de una vez en lugar de
   decodificarse píxel a píxel, y la comprobación de que la fila de tile cabe en VRAM sale del bucle
   de píxeles del camino de 8bpp.
5. La fidelidad se comprueba con los tres hashes guardianes del arnés (`600 → 31184031261ea2ec`;
   `2400` y `7200` → `b54066dc06f2b76c`, con 1.776.798 / 2.167.398 / 3.208.998 bloques y
   `fallback_steps: 0`) y con dos tests nuevos de frontera: los bordes exactos de un tramo de ventana
   (X1 inclusive, X2 exclusivo) y el sprite translúcido con `BLDCNT` en modo 0.
6. Medido en host a 7.200 fotogramas, mismo host y misma sesión contra el binario de referencia:
   `compose` 0,157 → **0,071 ms** (−54 %), fondos 0,284 → 0,197 (−31 %), `ppu` 0,536 → **0,367**
   (−32 %), con los subsistemas que no se tocaron planos (`spr` +4 %, `apu` +1 %) como control.

#### Alternativas y compromisos
* **Compromiso asumido: el listón del 60 % de `ppu` en host no se alcanza** (68,5 %). El prototipo del
  suelo lo explica: con la composición sustituida por un relleno (0,009 ms) el `ppu` sigue en 0,309 ms,
  porque lo que queda no es composición sino dibujar —fondos a 1,3 ns por píxel—. El techo se alcanza
  **dejando de dibujar líneas que no han cambiado** (la fase C de la propuesta, no implementada), no
  optimizando el cálculo. La mejora del dispositivo queda pendiente de medir allí.
* **Alternativa descartada: llevar la PPU a la GPU.** Sigue siendo el plan B (no el primero): a
  resolución nativa no garantiza menos latencia y añade el riesgo de divergencias de píxel con el
  modelo en software, que es la restricción que manda.
* **Alternativa descartada: escribir el framebuffer por pares `u32`.** Los bucles rectos quedan sin
  comprobación de límites y el compilador ya los vectoriza; hacerlo a mano habría pedido `unsafe` sin
  ganancia medida.
* **Compromiso asumido: los búferes por línea siguen reinicializándose enteros.** Quitarlo exige que
  vivan en `MemMap`, y las funciones de dibujado leen de `mem` mientras escriben su línea: eso obliga
  a partir los préstamos o a cambiar sus firmas. Queda como deuda de ~0,02-0,03 ms por fotograma.

---

### 35. Efectos de color inertes y camino especializado para oscurecido y aclarado en la PPU

#### Contexto
En mediciones de telemetría de Android TV (sesión 19), el 78 % de las líneas del fotograma eran
descartadas hacia el camino general de composición por tener `BLDCNT` activo, triplicando `ppucomp`
(3,63 ms de media). La investigación reveló que la mayoría eran efectos identidad (`BLDY = 0` o alpha
sin segundos objetivos) o efectos de oscurecido/aclarado (modos 2 y 3) cuyo resultado depende únicamente
de la capa ganadora superior sin requerir la búsqueda per-píxel de la segunda capa. Esto se planificó
en [`2026-10-13_ppu-efectos-inertes-y-camino-especializado.md`](../propuestas/2026-10-13_ppu-efectos-inertes-y-camino-especializado.md).

#### Decisión e implementación
1. **Regla de efectos inertes en vía rápida ([`../gba-recomp/crates/gba-core/src/ppu.rs`](../../../gba-recomp/crates/gba-core/src/ppu.rs)):**
   Se detectan efectos identidad en tiempo de escaneo: modos 2 y 3 con `EVY == 0` y modo 1 sin bits en la
   máscara de segundos objetivos. Estas líneas entran directamente por la vía rápida de tramos continuos
   (`rectas`), incrementando además el contador informativo `inertes`.
2. **Composición especializada para oscurecido y aclarado real (modos 2 y 3):**
   Las líneas con `BLDY > 0` en modos 2 y 3 se componen por tramos de control constante aplicando la
   transformación de color directamente sobre la capa ganadora si es objetivo de `BLDCNT` y las ventanas
   lo permiten, eliminando por completo la búsqueda de la segunda capa (`BUSQUEDAS_DOS_CAPAS = 0`).
3. **Telemetría diagnóstica detallada ([`src/recomp.rs`](../../src/recomp.rs)):**
   Se versionó el pipeline a `ppu=recto-2` y se amplió la línea `# ppucamino` con los campos `inertes`,
   `efecto`, `efalpha`, `efbrillo` y `efoscuro`, permitiendo auditar en Android TV qué fracción de las
   líneas son efectos identidad frente a efectos reales.
4. **Verificación diferencial:**
   Se contrastó el framebuffer píxel a píxel contra el camino general forzado, demostrando 100 % de
   fidelidad visual sin divergencias.

#### Alternativas y compromisos
* **Alternativa descartada: Especializar el modo alpha real (modo 1).** En ARM64 el producto de mezcla
  por canal toma un ciclo; el coste dominante es la búsqueda obligatoria de dos capas requerida por el
  alpha, por lo que el modo 1 se conservó en el camino general.
* **Alternativa descartada: Tablas precalculadas de color por `EVY`.** Añadir 64 KB de estado sin
  medir previamente en el dispositivo físico no se justificó frente al camino por tramos.

---

### 36. El alpha real de la PPU por tramos: componer sin buscar la segunda capa en cada píxel

#### Contexto
Las líneas con alpha real (`BLDCNT` modo 1 con segundos objetivos) eran el último bloque grande del camino
general: 186 de los 353 segundos de la sesión 20 con las ~160 líneas de cada fotograma por el camino de mezcla,
`comp` en 3,67 ms y 88 s por debajo de 60 fps. La Decisión 35 había aplazado el modo 1 con un argumento
correcto —en ARM64 el producto de mezcla por canal cuesta un ciclo y el coste dominante es la búsqueda
obligatoria de las dos capas—, y la consecuencia que faltaba era la obvia: si el coste es la búsqueda, se puede
buscar una vez por tramo en vez de una vez por píxel. Se planificó en
[`2026-10-13_alpha-real-por-tramos.md`](../propuestas/2026-10-13_alpha-real-por-tramos.md).

#### Decisión e implementación
1. **La pareja de capas se resuelve por subtramo ([`../gba-recomp/crates/gba-core/src/ppu.rs`](../../../gba-recomp/crates/gba-core/src/ppu.rs)):**
   `compose_alpha_tramos` recorre los tramos de control de ventana que ya calculaba el camino recto y los parte
   en subtramos de clave constante. `clave_de_tramo` resume por píxel lo único que `top_two_layers` necesita de
   `x`: qué capas habilitadas son opacas (bits 0-3) y qué prioridad tiene el sprite que tapa (bits 8-15). Dentro
   de un subtramo la pareja (capa de arriba, segunda capa) es la misma, así que se resuelve una sola vez.
2. **La mezcla usa la misma regla y el mismo código que el camino general:** `componer_tramo` decide una vez por
   subtramo con `control & 0x20`, `BLDCNT & (1 << arriba)` y `BLDCNT & (0x100 << segunda)`, y aplica
   `alpha_blend`. La pareja de dos fondos se compone en un bucle sin índices; las parejas con sprite o fondo de
   pantalla caen en un bucle genérico.
3. **Las líneas con sprite translúcido no cambian de camino:** el resultado de un píxel translúcido depende de
   `obj.semi`, que solo mira el camino general, así que el camino nuevo exige `razon_camino == 1 && !obj_semi`.
   Las líneas que adoptan el camino siguen contándose en `descartes_mezcla` y el reparto sigue sumando 160
   líneas por fotograma.
4. **El contador `BUSQUEDAS_DOS_CAPAS` es por hilo** (`Cell<u64>` en vez de `AtomicU64`, solo bajo
   `#[cfg(test)]`): los tests corren en paralelo y una cuenta global mezclaba las búsquedas de la línea de un
   test con las de otro (se vio como 248 en lugar de 240), lo que hacía no determinista la aserción exacta del
   escenario 1.
5. **Verificación:** diferencial de píxel contra el camino general forzado (los 240 píxeles de la línea) con y
   sin sprite translúcido y con varias combinaciones de objetivos y de `EVA`/`EVB`; los dos hashes guardianes de
   7.200 fotogramas; el arnés de fotogramas dorados (5 de 5 anclas); el reparto de líneas; y los mutantes M1
   (volver a buscar por píxel), M2 (ignorar el borde del sprite) y M3 (aplicar el camino a las líneas
   translúcidas), que mueren.

#### Medición (host, 7.200 fotogramas con la entrada del banco, 3 pares intercalados, `RECOMP_PPU_STATS=1`)

| | referencia | camino por tramos |
|---|---|---|
| `compose` | 0,230 ms/fotograma | **0,144** |
| `ppu` | 0,625 | 0,538 |
| `bg` / `spr` | 0,231 / 0,164 | 0,231 / 0,163 |
| lecturas / escrituras | 5.042 / 4.184 por fotograma | idénticas |
| tiempo de muro | — | −713 ms (−7,13 %), 3 de 3 pares negativos |
| hash / bloques nativos | `ee7dc7ef514b5a3f` / 11.018.334 | idénticos |

El techo medido de este cambio —una composición que no busca la segunda capa en absoluto— era 0,085
ms/fotograma: el recorrido de claves cuesta 0,059 (el 41 % del premium) y la ganancia realizada es el 59 %.

#### Alternativas y compromisos
* **Revierte la alternativa del modo 1 de la Decisión 35.** El alpha real ya no se conserva entero en el camino
  general: allí solo quedan las líneas con algún sprite translúcido y las que usan la ventana de sprites.
* **Alternativa descartada: materializar los cortes de la línea** (un arreglo con la unión de bordes de ventana,
  de sprite y de transparencia). El recorrido por clave hace ese mismo trabajo por píxel sin memoria intermedia
  y llega al objetivo, así que el arreglo no se implementó.
* **Compromiso asumido: `descartes_mezcla` pasa a leerse mal.** Ahora cuenta «líneas con alpha real», no «líneas
  descartadas de la vía rápida». Renombrar el campo es un cambio de telemetría, con propuesta propia.
* **Compromiso asumido: el dispositivo sigue sin medir.** El escenario de telemetría del APK (`comp` de 3,67 a
  ≤1,5 ms y `ejec` de 14,33 a ≤12,5 ms) queda pendiente de una sesión con la misma escena.

---

### 37. La imagen del procesador es el corazón del C generado, pero localizarla no ahorra

#### Contexto
El código nativo que corre en el teléfono trabaja sobre la imagen de los dieciséis registros en memoria (`uint32_t c[17]`): cada operando es una carga y cada resultado un almacén. La intuición era que mantenerlos en variables locales del bloque —que el compilador del dispositivo pueda tener en registros de máquina— bajaría el coste de `ejec`.

#### Decisión e implementación
Se midió **antes** de tocar el emisor, con el censo de ensamblado (el host no tiene `cc`/`gcc`/`clang`, así que la fase en ejecución se resolvió con ensamblado del clang del NDK). Ocho grupos reales de [`src-c/`](../../src-c/), a `-O0`, `-O1` y `-O2`.

| grupo | base ins/acc | forma con locales | desvío |
|---|---|---|---|
| 0804da3b | 57.794 / 18.033 | 48.242 / 12.325 | −16,5 % / −31,7 % |
| 0806fe91 | 48.168 / 15.161 | 41.143 / 9.300 | −14,6 % / −38,7 % |
| 089f2d69 | 81.424 / 23.799 | 104.867 / 43.350 | **+28,8 % / +82,2 %** |
| **total (8 grupos)** | **412.555 / 128.485** | **408.958 / 127.555** | **−0,9 % / −0,7 %** |

**Decisión: no se toca el emisor.** El agregado es un empate y hay grupos peores.

#### Por qué (el mecanismo)
El registro en memoria **sí** cuesta —el **31,1 % de las instrucciones generadas** son carga o almacén sobre la imagen del procesador—, pero localizarlos no baja el total: el modelo de bloques del emisor obliga a volver a escribir la imagen completa en **cada salida de bloque** (el despachador y el bloque siguiente leen `c[]`). El ahorro dentro del tramo recto lo devuelve el volcado de frontera, y en los grupos con muchas salidas (089f2d69: 5.256 `goto` y 1.518 puertas de cadena) el volcado domina.

#### Alternativas y compromisos
* **Alternativa descartada: volcar antes de cada llamada de bus («full»).** Medida: **+73,2 %** de instrucciones.
* **Alternativa viva, no ejecutada: volcado parcial** (solo los registros vivos, decidido por el analizador de flujo). Si se retoma, el techo debe calcularse sobre el número de salidas por bloque.
* **Compromiso asumido:** el 31,1 % de carga/almacén sobre la imagen del procesador queda como un hecho medido del perfil del juego, no como una palanca disponible.

---

### 38. La frontera entre bloques no es una palanca

#### Contexto
Cerrada la vía de los registros, la sospecha cayó en la frontera: cada bloque nativo termina volviendo al despachador, y el censo estático decía 5.540 grupos, 196.461 `goto` internos, 19.894 puertas de cadena y 81.572 `return`. Faltaba el dato que decide: **cuántas veces se cruza esa frontera por fotograma**.

#### Decisión e implementación
Se contaron las puertas de bucle dentro de `rt_chain_gate` (contador estático, activado con `RECOMP_PUERTAS`, coste del instrumento **+0,5 %** con pares intercalados). Resultado en 7.200 fotogramas:

| magnitud | total | por fotograma |
|---|---|---|
| vueltas al despachador (`native_blocks`) | 11.018.334 | **1.530,3** |
| puertas de bucle (`puertas`) | 1.896.786 | **263,4** |
| puertas que mandan volver | 22.765 | **3,16** (1,2 %) |

**Decisión: la frontera queda cerrada como palanca.** Atacarla rescataría como mucho el 1-2 % del fotograma, y la puerta de bucle es casi siempre un no-op: en el arranque mandó volver 7 veces en 21.600 fotogramas.

#### Alternativas y compromisos
* **Alternativa descartada: deducirlo del censo estático.** El censo no pesa por frecuencia de ejecución, como demostró esta misma medición (las puertas por fotograma caen de 263 a 90 según la escena).
* **Alternativa descartada: instrumentar el APK.** El contador vive en una corrida de laboratorio, no en un fotograma de 16,67 ms.
* **Compromiso asumido:** el instrumento se retiró y el motor quedó restaurado byte a byte; los números viven en la propuesta.

---

### 39. El reloj por instrucción tampoco es palanca, y el fotograma duerme

#### Contexto
El censo de llamadas del C generado puso a `a->tick` como el **32,1 %** de todas las llamadas emitidas (400.767), por delante de la lectura y la escritura de memoria, y la propuesta anterior lo señaló como sospechoso. Antes de atacarlo se acotó su techo.

#### Decisión e implementación
Contador en `rt_tick` (encendible en caliente con `RECOMP_TICK`, coste mediano **+0,80 %**) más un micro-banco que llama por la misma tabla `RT_API` **en memoria**, con el puntero opaco al optimizador.

| magnitud | menú/arranque | avance (`--input`) |
|---|---|---|
| peticiones de reloj por fotograma | 2.210,4 | 7.134,8 |
| ciclos pedidos por fotograma | 14.577 | 46.710 |
| coste por petición (camino completo) | **9,55 ns** | ídem |
| **techo del fotograma** | **0,13 %** | **0,41 %** |

**Decisión: descartado como palanca.** Es 7-23 veces menos que el 3 % que ya descartó la Decisión 33, y el emisor **ya coalesce** los ciclos (media 9,22 ciclos por petición; solo el 0,2 % lleva 1 ciclo), así que no había nada que agrupar.

#### El hallazgo: el fotograma duerme
Solo **14.577 y 46.710 de los 280.896 ciclos** que dura un fotograma pasan por `a->tick` (5,2 % y 16,6 %). El resto lo avanza `skip_to_next_event` mientras la CPU está detenida esperando el VBlank. Y `fallback_steps: 0` **no** significa que el juego no se detenga: `run_frame_native` llama a `m.step()` en el estado *halted* sin contarlo.

#### Alternativas y compromisos
* **Alternativa descartada: coalescer más los ciclos.** Ya está hecho en el emisor (`Ctx::cycles` + `flush_ticks`).
* **Alternativa descartada: A/B con `rt_tick` en no-op.** Altera lo medido: sin reloj, el juego cambia de comportamiento.
* **Compromiso asumido:** el 83-95 % del reloj del fotograma sigue sin atribuir entre *halt* y los shells de `interp_one` (BIOS), que avanzan el reloj sin pasar por `rt_tick`. Es el hueco más grande del presupuesto y la siguiente pregunta del proyecto.
* **Compromiso asumido:** el despacho indirecto por la tabla `RtApi` resultó **gratuito** en el host (−0,01 ns frente a una llamada directa), lo que relativiza la Decisión 33: allí la diferencia de 2,8× era el *trabajo* del accesor, no el salto indirecto.

---

### 40. La calibración de presentación se reabre para el televisor

#### Contexto
La Decisión 25 y la propuesta del arranque multihilo retiraron el barrido de calibración: en la tablet, solo la reducción de resolución bajaba el coste de `present` (51,4 ms → 2,9 ms), y eso choca con la nitidez nativa, que es una decisión de producto. El mecanismo (`src/present.rs`) se conservó entero para poder reabrir la decisión.

#### Decisión e implementación
Se reactiva el barrido de arranque (`CALIBRAR_PRESENTACION = true` en [`src/platform.rs`](../../src/platform.rs)) **como experimento**, con su guardián actualizado en el mismo cambio para que la decisión siga siendo explícita y comprobable: poner la constante a `false` hace fallar el test.

#### Alternativas y compromisos
* **Alternativa descartada: dejarlo desactivado y no medir.** El resultado de la tablet no tiene por qué trasladarse al Android TV, cuyo SoC y compositor son distintos; sin medirlo, la decisión se apoya en un solo dispositivo.
* **Compromiso asumido: cada arranque gasta 12-15 s** de mediciones y una sonda a media resolución mientras dure el experimento.
* **Compromiso asumido: la confirmación exige el televisor.** Si el resultado de la tablet se traslada, la constante vuelve a `false` y se anota; la sonda a media resolución sigue siendo **sonda**, nunca adoptable.

---

## Alternativas y Compromisos Globales


A nivel arquitectónico y operativo, el proyecto se rige por compromisos transversales:
1. **Rendimiento nativo sobre facilidad de desarrollo:** Se priorizó el desarrollo en C11/Rust con `NativeActivity` en lugar de Java/Kotlin con JNI, garantizando 60 FPS estables y mínimo consumo de batería.
2. **Portabilidad y privacidad sobre conveniencia local:** Se prohibió cualquier ruta absoluta del sistema en archivos versionados, asegurando que el proyecto clone y compile en cualquier entorno.
3. **Robustez ante fallos sobre asunciones tempranas:** Se integró un intérprete de respaldo para instrucciones no mapeadas y una pantalla de diagnóstico en CPU para pánicos no controlados.
