---
tipo: explicacion
verificado: si
---

# Arquitectura y Componentes del Port Nativo de Android

Este documento explica en detalle la arquitectura interna de **`blazing-recomp`**, describiendo cómo están estructurados los diferentes subsistemas, cómo cooperan entre sí y cómo fluyen los datos y eventos desde la pantalla del teléfono hasta el hardware emulado.

## Contexto y Plataforma Android

Blazing Recomp tiene como destino de ejecución el sistema operativo Android en dispositivos móviles, tablets y televisores inteligentes. Ejecutar un juego de GBA recompilado nativamente en Android impone el reto de gestionar un ciclo de vida complejo de ventana nativa (`ANativeWindow`), coordinar la sincronización gráfica a 60 FPS, procesar eventos táctiles asíncronos y comunicarse con servicios de hardware (como el motor de vibración) sin degradar la batería ni incurrir en recolección de basura de la máquina virtual de Java.

---

## Idea Central de la Arquitectura en Capas

Para resolver estos desafíos sin acoplar la lógica de emulación a las APIs móviles, el proyecto está diseñado bajo una arquitectura desacoplada en tres capas principales:

```mermaid
flowchart TD
    subgraph CapaAndroid["1. Capa del Sistema Android (Plataforma y SO)"]
        NA["android.app.NativeActivity (hasCode=false)"]
        Looper["ALooper & Event Loop (android-activity)"]
        TouchOS["MotionEvent (Toques físicos)"]
        HapticOS["Vibrator Service (VibrationEffect via JNI)"]
    end

    subgraph CapaRust["2. Runtime Nativo en Rust (blazing-recomp)"]
        AM["android_main (src/platform.rs)"]
        TC["TouchController (src/input.rs - Pura)"]
        Comp["Compositor GPU wgpu (src/platform.rs)"]
        Over["Overlay Renderer (src/overlay.rs & overlay.wgsl)"]
        ErrScr["ErrorScreenRenderer (src/error_screen.rs)"]
    end

    subgraph CapaMotor["3. Motor de Consola y Recompilación (Upstream)"]
        GbaCore["gba-core (PPU, APU, Bus de Memoria, Timers)"]
        Screen["screen::present::Presenter (ColorLut, Temporal)"]
        JuegoC["Lógica Recompilada C11 (17,207 funciones nativas)"]
    end

    NA --> AM
    TouchOS --> Looper --> AM
    AM --> TC
    TC -->|Máscara KEYINPUT| GbaCore
    AM --> HapticOS
    AM --> Comp
    Comp --> Over
    Comp --> Screen
    GbaCore --> JuegoC
    AM -.->|En pánico/crash| ErrScr
```

---

## 2. Subsistemas y Responsabilidades

### 2.1. Punto de Entrada y Módulo Raíz (`src/lib.rs`)
* **Propósito:** Define la biblioteca dinámica (`cdylib`) que Android carga al instanciar la `NativeActivity`.
* **Separación de entornos:**
  * Expone públicamente los módulos puros (`input`, `overlay`, `overlay_shader`, `error_screen`) compilables y testeables en cualquier máquina de desarrollo (Windows, macOS, Linux).
  * Concatena condicionalmente bajo `#[cfg(target_os = "android")]` el módulo `platform`, exportando el símbolo nativo `android_main`.

### 2.2. Entrada Táctil y Ergonomía (`src/input.rs`)
* **Propósito:** Gestión determinista de colisiones y cálculo de la máscara de botones de la consola Game Boy Advance.
* **Características clave:**
  * **Cero dependencias de plataforma:** Funciona exclusivamente con geometría en números flotantes (`f32`) y matrices matemáticas.
  * **Disposición ergonómica estilo GBA SP:**
    * **Cruceta (D-Pad):** Cuenca circular continua con 4 zonas direccionales independientes y zona muerta central.
    * **Clúster diagonal de acción:** Botones **B** y **A** alineados en un eje inclinado a -16° a la derecha, con los gatillos **L** y **R** ubicados sobre el mismo eje perpendicular por encima.
    * **Select y Start:** Pastillas redondeadas centradas en la base.
  * **Auto-rotación dinámica:** El método `resize(w, h)` recalcula instantáneamente todas las áreas de colisión para los modos **Portrait** (pantalla vertical con controles ampliados en la mitad inferior) y **Landscape** (pantalla horizontal con controles flotantes translúcidos en las esquinas inferiores).
  * **Conversión a hardware:** `pressed_mask()` y `keyinput_mask()` generan el registro `KEYINPUT` de GBA (activo en nivel bajo: `0` = pulsado, `1` = libre), listo para consumirse por `gba-core`.

### 2.3. Renderizado de Controles GPU y Shaders Procedurales (`src/overlay.rs` y `src/overlay.wgsl`)
* **Propósito:** Dibujar los controles táctiles y la carcasa de la consola directamente en la GPU mediante `wgpu`.
* **Características clave:**
  * **Shader procedural continuo (SDF):** En lugar de cargar texturas PNG que se deforman o pixelan al cambiar de resolución, [`src/overlay.wgsl`](../../src/overlay.wgsl) calcula matemáticamente biseles 3D, sombras de caída (*drop shadows*) e iluminación cóncava mediante funciones de distancia con signo (*Signed Distance Fields*).
  * **Atlas de glifos en arranque:** Durante el inicio, `src/overlay.rs` usa la biblioteca `fontdue` para rasterizar los caracteres tipográficos de los botones en una textura interna de 256x256 px en escala de grises.
  * **Poda espacial AABB:** Para permitir 60 FPS en GPUs de gama de entrada (como Mali-G31 y Adreno 610), el shader descarta de inmediato los píxeles fuera de las áreas de colisión, ahorrando hasta un 80% de cálculos trigonométricos.

### 2.4. Plataforma Android y Ciclo de Vida (`src/platform.rs`)
* **Propósito:** Orquestar el ciclo de vida de la aplicación Android, la superficie gráfica y los eventos de hardware.
* **Componentes principales:**
  * **`Compositor`:** Encapsula la instancia `wgpu::Device`, `wgpu::Queue` y la `Surface` configurada sobre la ventana `ANativeWindow`. Gestiona una latencia de swapchain estricta a 1 fotograma (`desired_maximum_frame_latency: 1`).
  * **Bucle de eventos no bloqueante:** Procesa eventos del sistema (`InitWindow`, `WindowResized`, `Pause`, `Resume`) mediante `poll_events`, drena los lotes de eventos táctiles en `input_events_iter()` y presenta el fotograma una sola vez por ráfaga de entrada para eliminar latencias perceptibles.
  * **Subsistema háptico (`Haptics`):** Conecta con el `VibratorService` de Android vía JNI nativo. Utiliza `VibrationEffect.createOneShot(35, -1)` (API 26+) con fallback al método legado `vibrate(long)` y purga de excepciones (`exception_clear`).

### 2.5. Diagnóstico de Fallos y Pantalla de Rescate (`src/error_screen.rs`)
* **Propósito:** Blindar la aplicación ante cualquier pánico nativo no controlado en Rust o fallo de wgpu.
* **Mecanismo:**
  * Un hook global en `std::panic::set_hook` captura el mensaje de error, archivo y línea.
  * Si la GPU falla o el motor colapsa, el renderizador bloquea el framebuffer en CPU con `ANativeWindow_lock` y dibuja un volcado de diagnóstico de emergencia con una tipografía bitmap incrustada de 8x16 px, evitando que la aplicación se cierre silenciosamente hacia el lanzador.

### 2.6. Arnés de Empaquetado Fat APK (`scripts/package_apk.ps1`)
* **Propósito:** Automatizar la compilación cruzada, stripping y firma del paquete instalable sin necesidad de Gradle o Android Studio.
* **Pipeline automatizado:**
  1. Compilación cruzada paralela con `cargo ndk` para `arm64-v8a`, `armeabi-v7a` y `x86_64`.
  2. Compilación de recursos y manifiesto con `aapt2 compile` y `aapt2 link`.
  3. Stripping no destructivo de bibliotecas `.so` en el staging con `llvm-strip --strip-all`.
  4. Empaquetado `STORED` (sin compresión interna) de bibliotecas con `jar`.
  5. Alineación de memoria a 16 KB con `zipalign`.
  6. Firma multi-esquema simultánea (v1 JAR, v2 y v3) con `apksigner`.

---

## 3. Integración con el Motor Hermano Upstream (`../gba-recomp`)

El proyecto no reinventa la emulación del hardware de la consola; se enlaza directamente con los componentes de hardware del repositorio hermano:

* **`gba-core` (`../gba-recomp/crates/gba-core`):**
  * Provee la estructura `Machine`, que modela la memoria de la consola, el bus del sistema, la paleta de colores y el procesamiento de audio.
  * Recibe la máscara `KEYINPUT` generada por `TouchController` en cada ciclo.
* **`screen` (`../gba-recomp/crates/screen`):**
  * Realiza la conversión de color BGR555 a sRGB/DCI-P3 mediante tablas LUT calibradas para emular el panel original con luz frontal (*Frontlit*) de la GBA SP.
  * Aplica mezcla temporal (*ghosting* LCD realista) para reproducir fielmente la respuesta de refresco del hardware original.

---

## Alternativas y Compromisos

1. **Sin capa Java/Kotlin:** Simplifica radicalmente la arquitectura y acelera la compilación, pero exige interactuar manualmente con JNI mediante firmas crudas (`jni_sig!`, `jni_str!`) para acceder a servicios del sistema.
2. **Carga directa de ROM pendiente (Fase MVP 0.1.0):** Actualmente la ventana gráfica muestra el panel base con el overlay interactivo; el enlace con la lectura de la ROM física del usuario se completará al integrar el menú nativo en pantalla táctil.
