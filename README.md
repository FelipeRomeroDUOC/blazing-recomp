# Blazing Recomp — Fire Emblem: The Blazing Blade

Proyecto independiente de port nativo para **Fire Emblem: The Blazing Blade (GBA)** utilizando la tecnología de recompilación estática de `gba-recomp`.

---

## Estado del Port

- **ROM Requerida:** `Fire Emblem (Europe)` (volcado de referencia No-Intro: `1575 - Fire Emblem (E)(Independent).gba`). La app valida exclusivamente el hash SHA-256 (`6f2c3082...`), por lo que cualquier nombre de archivo es aceptado si coincide su suma criptográfica.
- **ROM SHA-256:** `6f2c30823ab0237310b138739ccfad879f1a07c4a782a9cbb92317d60d03cee9`
- **BIOS SHA-256:** `fd2547724b505f487e6dcb29ec2ecff3af35a841a77ab2e85fd87350abd36570`
- **Motor de Audio:** Estructura `M4A / MusicPlayer2000` procesada por el núcleo de hardware (`gba-core`). *Nota:* El backend de audio nativo para Android (`AAudio / Oboe`) se encuentra actualmente en desarrollo (planificado para `v0.2.0` / Fase 2 del Roadmap).
- **Persistencia de Guardado:** En desarrollo. La memoria SRAM opera en memoria volátil durante la sesión activa; la sincronización y persistencia en archivo `.sav` en el almacenamiento de Android se encuentra en desarrollo (Hito `v0.1.0` del Roadmap). Al cerrar la app se reinicia la partida.

---

## Estructura del Proyecto

- [`src/`](src/): Runtime principal del port en Rust (`NativeActivity`, pipeline de renderizado `NativePpu` con shaders WGSL, compositor de presentación, puente de emulación multihilo, capa táctil y telemetría).
- [`src-c/`](src-c/): Código fuente C11 recompilado del juego (7.256.939 líneas distribuidas en 12 unidades de traducción, compiladas a `libblazing_recompiled.so` por Clang del NDK).
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

