# Blazing Recomp — Fire Emblem: The Blazing Blade

Plataforma nativa de recompilación estática para **Android** de **Fire Emblem: The Blazing Blade**, con proyección arquitectónica hacia la trilogía completa de GBA (*Binding Blade*, *Blazing Blade* y *The Sacred Stones*). 

El proyecto busca maximizar el porcentaje de funciones nativas in-house (hacia el 100% nativo), manipular directamente la composición en GPU/hardware moderno y proveer una plataforma extensible mediante mods y plugins para la comunidad (filosofía *Recomp* moderna).

---

## Filosofía del Proyecto y Créditos Upstream

* **Ingeniería Nativa Orientada al Dispositivo (Android-First):** En lugar de un simple wrapper o un port superficial que emula scanlines por CPU, Blazing Recomp manipula directamente la composición en GPU mediante [`NativePpu`](src/ppu_nativa.rs) con shaders WGSL sobre Vulkan y OpenGL ES 3.0, lo que permite capacidades que exceden al hardware original (como Widescreen adaptativo 16:9 / 21:9).
* **Plataforma Extensible de Mods:** Diseñado para que la comunidad de entusiastas y desarrolladores pueda inyectar código nativo C/Rust, sustituir texturas, orquestar música y crear plugins sin alterar la lógica de juego base (inspirado en proyectos de referencia como `gen1-recomp`).
* **Independencia Técnica Progresiva y Reconocimiento Upstream:** El proyecto nació a partir de la tecnología fundacional de recompilación estática de [`JRickey/gba-recomp`](https://github.com/JRickey/gba-recomp) y `gba-core`. Damos crédito explícito a sus aportes y mantenemos compatibilidad upstream en `pack.toml`, al tiempo que desarrollamos nuestra propia arquitectura independiente de runtime, GPU, audio y controles para Android.

---

## Estado del Port

- **ROM Requerida:** `Fire Emblem (Europe)` (volcado de referencia No-Intro: `1575 - Fire Emblem (E)(Independent).gba`). La app valida exclusivamente el hash SHA-256 (`6f2c3082...`), por lo que cualquier nombre de archivo es aceptado si coincide su suma criptográfica.
- **ROM SHA-256:** `6f2c30823ab0237310b138739ccfad879f1a07c4a782a9cbb92317d60d03cee9`
- **BIOS SHA-256:** `fd2547724b505f487e6dcb29ec2ecff3af35a841a77ab2e85fd87350abd36570`
- **Motor de Audio:** Implementado y operativo. Backend nativo de baja latencia con **AAudio NDK**, resampler de 65.536 Hz a 48.000 Hz, control dinámico de deriva (DRC) y mezclador de alta fidelidad **MP2K HLE** en punto flotante de 32 bits ([`src/audio.rs`](src/audio.rs)).
- **Persistencia de Guardado:** En desarrollo activo (rama `implementacion-guardado`) para el hito **`v0.1.0` (MVP Jugable)**. Conecta la SRAM/Flash de GBA con el almacenamiento interno de Android (`filesDir`) para almacenar partidas en archivos `.sav`.
- **Controles y UI:** Overlay táctil procedural por shaders activo. La integración de mandos físicos (Bluetooth/USB) y menús nativos de configuración forma parte del hito `v0.1.0`.
- **Feature Insignia MVP:** Widescreen adaptativo (16:9 y 21:9) en mapas tácticos mediante ampliación del frustum en `NativePpu`.

---

## Estructura del Proyecto

- [`src/`](src/): Runtime principal del port en Rust (`NativeActivity`, pipeline de renderizado `NativePpu` con shaders WGSL, compositor de presentación, motor de audio AAudio, puente multihilo, capa táctil y telemetría).
- [`src-c/`](src-c/): Código fuente C11 recompilado del juego (7.256.939 líneas distribuidas en 12 unidades de traducción, compiladas a `libblazing_recompiled.so` por Clang del NDK con 0 caídas al intérprete en combate).
- [`android/`](android/): Manifiesto `AndroidManifest.xml` y recursos de la aplicación para el empaquetado directo con AAPT2.
- [`scripts/`](scripts/): Suite de herramientas CLI y diagnóstico (`package_apk.ps1`, análisis de telemetría, desensamblador de ROM, auditoría de labels AOT y validación Diátaxis).
- [`docs/`](docs/README.md): Portal de documentación técnica estructurado bajo el estándar Diátaxis (guías prácticas, explicaciones arquitectónicas, referencias formales y fuentes).
- [`examples/`](examples/README.md): Arneses deterministas de micro-benchmarking, replays de entrada y suites de comparación visual contra la referencia.
- [`telemetria/`](telemetria/esquema.txt): Esquema formal único y contrato de telemetría de rendimiento consumido por el runtime y el analizador CLI.
- [`tests/`](tests/): Suites de pruebas de integración (empaquetado APK, higiene arquitectónica, validación de contratos y sincronización de documentación).
- [`blazing.labels.toml`](blazing.labels.toml): Catálogo de 18.483 funciones de la ROM de referencia con sus puntos de entrada canónicos y modos Thumb/ARM.
- [`pack.toml`](pack.toml): Manifiesto declarativo consumido por la herramienta de recompilación upstream (`recomp build`).

---

## Instrucciones de Compilación y Prueba

### 1. Ejecutar localmente en PC con el Visor Interactivo Doble (60 FPS):
```powershell
# Compila y ejecuta el visor en PC con doble pantalla (referencia por software y PPU nativa en GPU):
cargo run --release --features proto --bin ppu_doble -- "roms/1575 - Fire Emblem (E)(Independent).gba"
```
*(Sustituye la ruta si tu ROM oficial europea se encuentra en otra carpeta).*

*Controles de juego:* Flechas de dirección (D-Pad), `Z` (Botón A), `X` (Botón B), `Enter` (Start), `Retroceso` (Select), `A` (Gatillo L), `S` (Gatillo R).  
*Atajos de diagnóstico:* `Tab` (alternar resaltado de diferencias en magenta), `F5` (guardar captura PNG en `volcados/diferencias/`), `L` (alternar referencia de software), `Espacio` (pausar / reanudar ejecución), `N` (avance paso a paso de 1 fotograma durante la pausa), `Shift` (modo turbo sin límite de 60 FPS mientras se mantenga presionado), `Esc` (salir).

> [!NOTE]
> **Nota de arquitectura y dependencias:**  
> 1. En PC, el visor interactivo ejecuta la lógica del juego mediante el intérprete de CPU de `gba-core` para verificar la PPU nativa acelerada por GPU frente al software, ya que la biblioteca C11 recompilada se enlaza dinámicamente de forma exclusiva en el target de Android.  
> 2. Debido a la directiva `[patch]` en `Cargo.toml`, cualquier compilación que involucre `gba-core` (incluyendo `cargo test`, el visor interactivo y el empaquetado de Android) requiere el repositorio hermano clonado adyacente en `../gba-recomp`, donde residen las extensiones de telemetría y PPU nativa en proceso de integración en el upstream.

### 2. Compilar y empaquetar el Fat APK para Android:
```powershell
# Compilación cruzada multi-ABI (arm64-v8a, armeabi-v7a, x86_64), alineación a 16 KB y firma de APK:
powershell -ExecutionPolicy Bypass -File scripts/package_apk.ps1
```
El instalador generado se ubicará en `target/blazing-recomp-release.apk`.

> [!TIP]
> **Primer arranque en Android:**  
> Por razones legales de preservación limpia (*Clean-Room*), el APK no distribuye assets protegidos. Antes de iniciar la partida, es necesario aprovisionar la ROM oficial europea y el BIOS mediante ADB (ver [Guía de empaquetado](docs/guias/compilar-y-empaquetar-apk.md)). Si se ejecuta la aplicación sin estos archivos, mostrará una **pantalla de diagnóstico** con la lista exhaustiva de rutas de almacenamiento escaneadas.

---

## Documentación Técnica

La documentación del proyecto sigue el estándar Diátaxis y se encuentra organizada en [`docs/`](docs/README.md):

- **[Guías Prácticas (`docs/guias/`)](docs/guias/README.md):**
  - [Compilar y Empaquetar APK](docs/guias/compilar-y-empaquetar-apk.md): Pipeline completo para generar e instalar el APK en dispositivos Android.
  - [Ejecutar Visor Doble en PC](docs/guias/ejecutar-visor-doble-pc.md): Pruebas locales y diagnóstico de fidelidad gráfica.
  - [Mapear Labels AOT](docs/guias/mapear-labels-aot.md): Cosecha e ingeniería inversa de funciones de la ROM.
  - [Medir Rendimiento](docs/guias/medir-rendimiento.md): Análisis de telemetría y perfiles de fotograma.
- **[Explicación (`docs/explicacion/`)](docs/explicacion/README.md):**
  - [Registro Histórico de Decisiones (ADR)](docs/explicacion/DECISIONES.md): Decisiones arquitectónicas, compromisos y soluciones desde el inicio del proyecto.
  - [Arquitectura del Port Android](docs/explicacion/arquitectura-port-android.md): Modelo de capas, runtime nativo en Rust y subsistemas.
  - [Estrategia de Versionado y Roadmap](docs/explicacion/roadmap-y-versionado.md): Filosofía de versiones y metas hacia la versión 1.0.
  - [Arquitectura de Código C Recompilado](docs/explicacion/arquitectura-c-recompilado.md): Análisis del pipeline AOT y ABI de registros.
  - [Anatomía del Motor de Fire Emblem 7](docs/explicacion/anatomia-motor-fire-emblem-7.md): Organización de memoria (IWRAM/EWRAM), Modo 0 de la PPU, ciclo de frame, sistema Proc y driver M4A.
  - [Caminos de Composición de la PPU por Software](docs/explicacion/ppu-caminos-de-composicion.md): Estudio histórico del renderizador por software y lógica de composición.
- **[Referencia Técnica (`docs/referencia/`)](docs/referencia/README.md):**
  - [APIs y Módulos de Rust](docs/referencia/api-modulos-rust.md): Referencia pública extraída automáticamente del código fuente.
  - [Registros de Video LCD de GBA](docs/referencia/registros-video-lcd-gba.md): Especificación de mapa MMIO y modos de pantalla.
  - [Comandos CLI y Diagnóstico](docs/referencia/comandos-cli.md): Sintaxis de scripts y herramientas del proyecto.
  - [Configuración de Manifiesto (`pack.toml`)](docs/referencia/CONFIGURACION_PACK.md): Especificación de claves y parámetros.
  - [Formato de Etiquetas (`blazing.labels.toml`)](docs/referencia/FORMATO_LABELS.md): Especificación de labels y análisis de bloques.
  - [Glosario Arquitectónico](GLOSSARY.md): Vocabulario técnico y conceptos clave del port.
- **Planificación y Seguimiento:**
  - [Registro de Cambios (Changelog)](CHANGELOG.md): Historial de versiones publicadas.
  - [Tareas Pendientes y Deuda Técnica](docs/PENDIENTES.md): Catastro de tareas accionables del código.
  - [Backlog del Proyecto](docs/BACKLOG.md): Tareas completadas y hoja de ruta activa.
