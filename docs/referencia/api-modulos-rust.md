---
tipo: referencia
verificado: si
---

# Referencia Técnica: Módulos y APIs de Rust

## Descripción general

Este documento describe con exactitud y neutralidad técnica las estructuras, enumeraciones, funciones públicas y constantes del crate nativo `blazing-recomp`. Es generado y auditado automáticamente desde las anotaciones de documentación (`///` y `//!`) del código fuente en `src/` mediante `scripts/extraer_referencias.py`, garantizando sincronización continua con el código.

---

## Índice de Módulos

| Módulo | Elementos Públicos | Descripción |
| :--- | :--- | :--- |
| [`lib.rs`](#módulo-librs) | 0 | Blazing Recomp — punto de entrada nativo para Android. |
| [`input.rs`](#módulo-inputrs) | 19 | Entrada táctil ergonómica (Fase 5.1, propuesta `2026-10-08_controles-tactiles-haptica`). |
| [`overlay.rs`](#módulo-overlayrs) | 12 | Overlay táctil y marco GBA SP (Fase 5.2, propuesta `2026-10-09_renderizado-overlay-tactil`). |
| [`overlay_shader.rs`](#módulo-overlay_shaderrs) | 3 | Módulo del shader del overlay: incluye el WGSL como constante y lo expone como `&str` para el pipeline y para el test de validación con Naga en host (criterio `test_overlay_shader_validates`). |
| [`error_screen.rs`](#módulo-error_screenrs) | 4 | Renderizador de pantalla de error por software y blindaje de GPU. |
| [`platform.rs`](#módulo-platformrs) | 2 | Plataforma Android (solo compila con `cfg(target_os = "android")`): ciclo de vida NativeActivity, presentación GPU, entrada táctil con háptica y logging a Logcat. |
| [`storage.rs`](#módulo-storagers) | 14 | Módulo interno del port. |
| [`runner.rs`](#módulo-runnerrs) | 25 | Ejecución y sincronización del motor de emulación GBA (Fase 4 y Fase 5). |
| [`recomp.rs`](#módulo-recomprs) | 30 | Subsistema de despacho y ejecución de bloques C11 recompilados Ahead-Of-Time (AOT). |
| [`fps.rs`](#módulo-fpsrs) | 18 | Contador de FPS y generador de HUD visual en pantalla. |
| [`perf.rs`](#módulo-perfrs) | 17 | Módulo de medición de rendimiento por fase del fotograma. |
| [`telemetria.rs`](#módulo-telemetriars) | 13 | Contrato de la línea de segundo (`t=`) de la telemetría de rendimiento. |
| [`pipeline.rs`](#módulo-pipeliners) | 32 | Módulo de pipeline multihilo para desacoplar la emulación del renderizado GPU. |
| [`present.rs`](#módulo-presentrs) | 27 | Calibración de presentación del swapchain (módulo puro, testeable en host). |
| [`ppu_nativa.rs`](#módulo-ppu_nativars) | 72 | Módulo PPU nativa: dibuja el cuadro de la GBA con la GPU del dispositivo. |

---

## Módulo `lib.rs`

Blazing Recomp — punto de entrada nativo para Android.

Este crate es la biblioteca dinámica (`cdylib`) que Android carga al
arrancar la NativeActivity. Fases integradas:

- Renderizado GPU nativo (ADR 42): composición acelerada directamente
  en GPU mediante [`crate::ppu_nativa::NativePpu`] con shaders WGSL, tabla
  LUT vertical de paleta y presentación sobre superficie Android en
  [`crate::present`] (wgpu: Vulkan con respaldo automático a OpenGL ES).
- Entrada táctil ergonómica: cruceta con cuenca, clúster diagonal GBA SP con
  L/B/A/R, Select/Start, auto-rotación de hitboxes, bandera `touch_enabled` y
  respuesta háptica por vibración vía JNI.

La lógica geométrica de entrada vive en [`crate::input`] (pura,
testeable en host, sin dependencias de Android). Todo lo que toca
JNI, NativeActivity o Logcat está tras `cfg(target_os = "android")`.

El motor `gba-core` ejecuta la lógica del juego (acelerada por los bloques
C11 nativos de `libblazing_recompiled.so` cargados por [`crate::recomp`]),
y la PPU nativa compone cada fotograma en la GPU coordinada con el refresco
del panel.

*No expone símbolos públicos directos fuera de declaraciones de submódulos.*

## Módulo `input.rs`

Entrada táctil ergonómica (Fase 5.1, propuesta
`2026-10-08_controles-tactiles-haptica`).

Lógica pura: geometría de hitboxes, orientaciones y mapeo a los bits
KEYINPUT de la GBA (activo en bajo, ver `gba-core/src/mem.rs`:
`keys: u16` con 0x3FF = nada pulsado). Sin dependencias de Android,
testeable en host con `cargo test`.

Geometría (validada en `docs/prototipos/layout_tactil_v2.html`):
- **Cruceta** con cuenca circular: 4 zonas direccionales dentro del
  anillo (arriba/abajo/izquierda/derecha), con zona muerta central.
- **Clúster diagonal** estilo GBA SP a la derecha: cápsula inclinada
  con B (abajo-izquierda) y A (arriba-derecha), y las pastillas L y R
  en el mismo eje por encima (L sobre B, R sobre A). Nunca en esquinas
  superiores (decisión de contexto de la propuesta).
- **Select/Start** centrados debajo.
- Portrait: controles agrandados con colores en la mitad inferior.
  Landscape: base inferior translúcida, opacidad regulable (la
  opacidad es presentación; aquí solo cambia la geometría).

Precisión individual sobre multitáctil complejo: hitboxes holgadas,
descarte de toques ambiguos (primer botón que contiene el punto) y
cancelación total en `Cancel`.

### `Button` (Enum)

Botón de GBA (nombres estándar de la consola).

**Firma:**
```rust
pub enum Button
```

### `key_bit` (Función)

Bit de KEYINPUT (activo en bajo) que controla cada botón.
Fuente: `gba-core/src/mem.rs` (`keys: u16`, 0x3FF = nada pulsado)
y mapa estándar de KEYINPUT de la GBA.

**Firma:**
```rust
pub fn key_bit(self) -> u16
```

### `button_from_android_keycode` (Función)

Mapea un código de tecla estándar de Android (`android.view.KeyEvent.KEYCODE_*`) a un botón de GBA.
Soporta mandos Bluetooth, USB, teclados físicos y control remoto de Android TV.

**Firma:**
```rust
pub fn button_from_android_keycode(keycode: i32) -> Option<Button>
```

### `Orientation` (Enum)

Orientación de la pantalla; determina la geometría de los controles.

**Firma:**
```rust
pub enum Orientation
```

### `TouchArea` (Struct)

Caja táctil circular (hitbox holgada con radio > radio visual).

**Firma:**
```rust
pub struct TouchArea
```

### `contains` (Función)

¿Contiene este área el punto (x, y)?

**Firma:**
```rust
pub fn contains(&self, x: f32, y: f32) -> bool
```

### `layout` (Función)

Calcula las hitboxes para una ventana de `w` x `h` píxeles y una
orientación. Determinista y sin estado: la misma entrada produce la
misma salida (regresión `test_touch_layout_orientation_and_toggle`).

Portrait (w < h): controles en la mitad inferior, agrandados y ergonómicos.
Landscape (w >= h): franja inferior flotante translúcida en esquinas.

**Firma:**
```rust
pub fn layout(w: u32, h: u32, orientation: Orientation) -> Vec<TouchArea>
```

### `TouchController` (Struct)

Controlador táctil: recibe puntos de la pantalla y mantiene el estado
KEYINPUT de la GBA.

Reglas (contexto de la propuesta):
- `touch_enabled == false` → ningún toque altera las teclas (uso de
  mando físico).
- Precisión individual: un punto se mapea al primer botón que lo
  contiene; si no contiene ninguno, no cambia nada (descarte de
  toques ambiguos).
- `Cancel` libera todos los toques activos.

Extras de la Fase 5.2: `is_pressed`/`pressed_mask` exponen el estado
vivo para el resaltado visual del overlay.

**Firma:**
```rust
pub struct TouchController
```

### `new` (Función)

Estado inicial: 1080x2400 portrait (típico), controles activos.

**Firma:**
```rust
pub fn new() -> TouchController
```

### `resize` (Función)

Actualiza el tamaño de ventana y la orientación; recalcula
hitboxes (los toques activos se liberan: la geometría cambió).

**Firma:**
```rust
pub fn resize(&mut self, w: u32, h: u32)
```

### `size` (Función)

*Sin comentario de documentación en código fuente.*

**Firma:**
```rust
pub fn size(&self) -> (u32, u32)
```

### `orientation` (Función)

*Sin comentario de documentación en código fuente.*

**Firma:**
```rust
pub fn orientation(&self) -> Orientation
```

### `areas` (Función)

*Sin comentario de documentación en código fuente.*

**Firma:**
```rust
pub fn areas(&self) -> &[TouchArea]
```

### `is_pressed` (Función)

¿Está este botón pulsado ahora mismo (por algún puntero)?
Fuente de verdad para el resaltado visual del overlay (Fase 5.2).

**Firma:**
```rust
pub fn is_pressed(&self, button: Button) -> bool
```

### `pressed_mask` (Función)

Máscara de bits de los botones pulsados (bit = `key_bit()` del
botón). Útil para pasar al overlay en una sola palabra.

**Firma:**
```rust
pub fn pressed_mask(&self) -> u16
```

### `touch_down` (Función)

Procesa un toque hacia abajo. Devuelve el botón si este toque lo
activó (transición nueva, para háptica).

**Firma:**
```rust
pub fn touch_down(&mut self, pointer_id: i32, x: f32, y: f32) -> Option<Button>
```

### `touch_up` (Función)

Procesa un toque hacia arriba. Devuelve el botón si este toque lo
liberó (transición, para háptica de liberación).

**Firma:**
```rust
pub fn touch_up(&mut self, pointer_id: i32) -> Option<Button>
```

### `cancel_all` (Función)

Cancela todos los toques (evento `Cancel` de Android).

**Firma:**
```rust
pub fn cancel_all(&mut self)
```

### `key_mask` (Función)

Máscara KEYINPUT completa, lista para `MemMap.keys` (activo en
bajo: bits de botones no pulsados en 1).

**Firma:**
```rust
pub fn key_mask(&self) -> u16
```

---

## Módulo `overlay.rs`

Overlay táctil y marco GBA SP (Fase 5.2, propuesta
`2026-10-09_renderizado-overlay-tactil`).

Dibuja los controles virtuales **en las mismas coordenadas** que las
hitboxes de [`crate::input`]: los parámetros de dibujo se derivan de
las mismas `TouchArea` que produce `TouchController::resize`, así la
correspondencia geométrica es por construcción, no por duplicación de
constantes (criterio `test_overlay_geometry_matches_hitboxes`).

Todo el dibujo es procedural (shader SDF, sin texturas): bordes
perfectos a cualquier DPI (restricción de la propuesta). Los glifos
son 3x5 bit-maps evaluados en el shader.

Colores (restricción del prototipo `layout_tactil_v2.html`):
- Portrait: B rojo, A verde, L/R blancos, D-Pad y Select/Start
  gris-lavanda de la carcasa.
- Landscape: mismos botones con opacidad translúcida global.

### `Glyph` (Enum)

Índices de glifo del shader (deben coincidir con la tabla en
`overlay.wgsl`).

**Firma:**
```rust
pub enum Glyph
```

### `FontAtlas` (Struct)

Atlas de textura de glifos rasterizados en memoria (R8Unorm).

**Firma:**
```rust
pub struct FontAtlas
```

### `build_font_atlas` (Función)

Construye el atlas tipográfico R8 (256x256 px) rasterizando glifos de Outfit Bold
y JetBrains Mono Bold al arrancar la aplicación.

**Firma:**
```rust
pub fn build_font_atlas() -> FontAtlas
```

### `ElemParams` (Struct)

Un elemento dibujable del overlay, derivado de una `TouchArea`.
Espejo exacto de `struct Elem` en `overlay.wgsl` (layout
uniform-compatible: 16 x f32 = 64 B, stride múltiplo de 16).

**Firma:**
```rust
pub struct ElemParams
```

### `ElemKind` (Enum)

Primitiva del elemento: círculo o pastilla redondeada orientada.

**Firma:**
```rust
pub enum ElemKind
```

### `scaled` (Función)

Reescala la **geometría** del elemento al espacio del búfer de render.

Las hitboxes táctiles viven en píxeles físicos de la ventana, pero el
shader dibuja en píxeles del búfer del swapchain: cuando la calibración
reduce la resolución de render hay que escalar longitudes (centro,
radio, radio de esquina y extent vertical) para que la composición
final en pantalla coincida. Los atributos no geométricos (color, estado
pulsado, primitiva, glifo, ángulo) y las proporciones adimensionales
(`glyph_scale`, `ring`) se conservan intactos.

**Firma:**
```rust
pub fn scaled(&self, factor: f32) -> ElemParams
```

### `pack` (Función)

Empaqueta en el orden del `struct Elem` del shader (16 floats / 64 bytes).

**Firma:**
```rust
pub fn pack(&self) -> [f32; 16]
```

### `Globals` (Struct)

Parámetros globales de dibujo (espejo de `struct Globals` del shader,
4 x f32).

**Firma:**
```rust
pub struct Globals
```

### `pack` (Función)

*Sin comentario de documentación en código fuente.*

**Firma:**
```rust
pub fn pack(&self) -> [f32; 4]
```

### `build_elems_with_orientation` (Función)

Construye la lista de parámetros de dibujo a partir de las áreas de
colisión del controlador táctil, orientación y el estado pulsado actual.

Correspondencia por construcción: cada elemento sale de la misma
`TouchArea` que usa el mapeo de toques, así el dibujo coincide con la
zona de pulsación (criterio de aceptación 2). Máximo 12 elementos;
el layout de `crate::input` produce exactamente 10.

**Firma:**
```rust
pub fn build_elems_with_orientation( areas: &[crate::input::TouchArea], orientation: crate::input::Orientation, is_pressed: impl Fn(crate::input::Button) -> bool, ) -> Vec<ElemParams>
```

### `build_elems` (Función)

Construye la lista de parámetros de dibujo asumiendo orientación Portrait.

**Firma:**
```rust
pub fn build_elems( areas: &[crate::input::TouchArea], is_pressed: impl Fn(crate::input::Button) -> bool, ) -> Vec<ElemParams>
```

### `build_globals` (Función)

Globales de dibujo para una orientación.

**Firma:**
```rust
pub fn build_globals(w: u32, h: u32, orientation: crate::input::Orientation) -> Globals
```

---

## Módulo `overlay_shader.rs`

Módulo del shader del overlay: incluye el WGSL como constante y lo
expone como `&str` para el pipeline y para el test de validación con
Naga en host (criterio `test_overlay_shader_validates`).

### `WGSL` (Constante)

Fuente WGSL del overlay (`src/overlay.wgsl`).

**Firma:**
```rust
pub const WGSL: &str
```

### `GAME_BLIT_WGSL` (Constante)

Fuente WGSL del blit de juego GBA (`src/game_blit.wgsl`).

**Firma:**
```rust
pub const GAME_BLIT_WGSL: &str
```

### `HUD_BLIT_WGSL` (Constante)

Fuente WGSL del blit de elementos HUD (`src/hud_blit.wgsl`).

**Firma:**
```rust
pub const HUD_BLIT_WGSL: &str
```

---

## Módulo `error_screen.rs`

Renderizador de pantalla de error por software y blindaje de GPU.

Diseñado para presentar mensajes de diagnóstico legibles directamente en el
framebuffer de la ventana nativa (`ANativeWindow`) cuando la inicialización
gráfica acelerada (`wgpu`) falla (por ejemplo, en emuladores como BlueStacks
que carecen de driver Vulkan funcional o fallan en la negociación EGL) o
ante cualquier pánico no capturado en el hilo nativo.

Este módulo es puro y compilable/testeable en host.

### `select_surface_format_and_alpha` (Función)

Negocia un formato de textura y un modo alfa seguros a partir de las
capacidades de la superficie reportadas por el adaptador.

Si la lista de formatos está vacía (comportamiento observado en ciertos
emuladores y controladores antes de negociar la ventana), devuelve un
formato estándar seguro (`Rgba8UnormSrgb`) en lugar de entrar en pánico por
indexación ávida fuera de rango (`caps.formats[0]`).

**Firma:**
```rust
pub fn select_surface_format_and_alpha( formats: &[wgpu::TextureFormat], alpha_modes: &[wgpu::CompositeAlphaMode], ) -> (wgpu::TextureFormat, wgpu::CompositeAlphaMode)
```

### `ErrorScreenRenderer` (Struct)

Renderizador por software para presentar pantallas de error y diagnóstico.

**Firma:**
```rust
pub struct ErrorScreenRenderer
```

### `new` (Función)

Carga el rasterizador de fuentes tipográficas para la pantalla de error.

**Firma:**
```rust
pub fn new() -> Result<Self, String>
```

### `render` (Función)

Renderiza un marco de error con fondo temático, barra de título y
mensaje multilínea legible directamente en el búfer de píxeles provisto.

- `buffer`: slice mutable con los bytes del framebuffer nativo.
- `width`, `height`: dimensiones visibles de la ventana en píxeles.
- `stride`: número de píxeles por fila en la memoria (stride >= width).
- `bytes_per_pixel`: típicamente 4 para `RGBA_8888` / `RGBX_8888`.
- `title`: título de la alerta (ej. `[BLAZING RECOMP] ERROR DE INICIALIZACIÓN`).
- `message`: cuerpo del mensaje de error o volcado de pánico.

**Firma:**
```rust
pub fn render( &self, buffer: &mut [u8], width: usize, height: usize, stride: usize, bytes_per_pixel: usize, title: &str, message: &str, )
```

---

## Módulo `platform.rs`

Plataforma Android (solo compila con `cfg(target_os = "android")`):
ciclo de vida NativeActivity, presentación GPU, entrada táctil con
háptica y logging a Logcat.

### `FORZAR_PPU_SOFTWARE` (Constante)

Permite forzar el renderizado por software en CPU para pruebas comparativas A/B.
Por defecto `false` (usar GPU nativa cuando esté disponible).

**Firma:**
```rust
pub const FORZAR_PPU_SOFTWARE: bool
```

### `android_main` (Función)

*Sin comentario de documentación en código fuente.*

**Firma:**
```rust
pub fn android_main(app: android_activity::AndroidApp)
```

---

## Módulo `storage.rs`

### `OFFICIAL_ROM_SHA256` (Constante)

Firma criptográfica SHA-256 oficial de la ROM europea de Fire Emblem (Europe) (16 MB).

**Firma:**
```rust
pub const OFFICIAL_ROM_SHA256: &str
```

### `OFFICIAL_BIOS_SHA256` (Constante)

Firma criptográfica SHA-256 oficial de la BIOS de Game Boy Advance (16 KB).

**Firma:**
```rust
pub const OFFICIAL_BIOS_SHA256: &str
```

### `OFFICIAL_ROM_SIZE` (Constante)

Tamaño exacto de la ROM oficial de Fire Emblem (16 MB en bytes).

**Firma:**
```rust
pub const OFFICIAL_ROM_SIZE: usize
```

### `OFFICIAL_BIOS_SIZE` (Constante)

Tamaño exacto del binario de BIOS oficial de GBA (16 KB en bytes).

**Firma:**
```rust
pub const OFFICIAL_BIOS_SIZE: usize
```

### `KNOWN_ROM_FILENAMES` (Constante)

Nombres de archivo comunes con los que suele guardarse la ROM de Fire Emblem.

**Firma:**
```rust
pub const KNOWN_ROM_FILENAMES: &[&str]
```

### `KNOWN_BIOS_FILENAMES` (Constante)

Nombres de archivo comunes para la BIOS oficial de GBA.

**Firma:**
```rust
pub const KNOWN_BIOS_FILENAMES: &[&str]
```

### `LoadedAssets` (Struct)

Assets del juego cargados y verificados en memoria listos para la ejecución.

**Firma:**
```rust
pub struct LoadedAssets
```

### `StorageError` (Enum)

Errores posibles al buscar, leer o validar los assets del juego.

**Firma:**
```rust
pub enum StorageError
```

### `compute_sha256` (Función)

Calcula la firma criptográfica SHA-256 de un búfer de bytes y la retorna en formato hexadecimal en minúsculas.

**Firma:**
```rust
pub fn compute_sha256(data: &[u8]) -> String
```

### `verify_rom` (Función)

Verifica si un búfer de datos coincide exactamente en tamaño y firma SHA-256 con la ROM oficial europea.

**Firma:**
```rust
pub fn verify_rom(data: &[u8]) -> bool
```

### `verify_bios` (Función)

Verifica si un búfer de datos coincide exactamente en tamaño y firma SHA-256 con la BIOS oficial de GBA.

**Firma:**
```rust
pub fn verify_bios(data: &[u8]) -> bool
```

### `resolve_search_directories` (Función)

Genera la lista ordenada de directorios candidatos de búsqueda, priorizando la carpeta Downloads
y el almacenamiento propio de la app en Android 11-16.

**Firma:**
```rust
pub fn resolve_search_directories( internal_data_dir: Option<&Path>, external_data_dir: Option<&Path>, ) -> Vec<PathBuf>
```

### `scan_and_load_game` (Función)

Escanea los directorios especificados en busca de archivos de ROM y BIOS oficiales.

Combina sondeo directo de nombres estándar (`fe7.gba`, `Fire Emblem (Europe).gba`) con
escaneo general del directorio para máxima compatibilidad con Scoped Storage en Android 16.

**Firma:**
```rust
pub fn scan_and_load_game(search_dirs: &[PathBuf]) -> Result<LoadedAssets, StorageError>
```

### `render_guide_screen` (Función)

Renderiza un mensaje instructivo nítido en el framebuffer de juego de 240x160 px.

Permite que el Compositor GPU presente la carcasa y controles táctiles de la consola
mostrando la guía de instalación en la pantalla virtual, sin recurrir a bloqueos de CPU.

**Firma:**
```rust
pub fn render_guide_screen(buffer: &mut [[u8; 4]], title: &str, lines: &[&str])
```

---

## Módulo `runner.rs`

Ejecución y sincronización del motor de emulación GBA (Fase 4 y Fase 5).

Encapsula el modelo de hardware (`gba_core::Machine`), el simulador de panel
LCD con respuesta temporal (`screen::Temporal`) y la tabla de color (`screen::ColorLut`).

Este módulo es puro, sin dependencias de Android ni de ventanas gráficas,
testeable en host con `cargo test`.

### `GBA_W` (Constante)

Ancho estándar del framebuffer de la Game Boy Advance en píxeles.

**Firma:**
```rust
pub const GBA_W: u32
```

### `GBA_H` (Constante)

Alto estándar del framebuffer de la Game Boy Advance en píxeles.

**Firma:**
```rust
pub const GBA_H: u32
```

### `GBA_PIXELS` (Constante)

Total de píxeles por fotograma de la Game Boy Advance (38,400).

**Firma:**
```rust
pub const GBA_PIXELS: usize
```

### `CYCLES_PER_FRAME` (Constante)

Ciclos de reloj maestro por fotograma estándar (~16.78 MHz / 59.73 Hz).

**Firma:**
```rust
pub const CYCLES_PER_FRAME: u64
```

### `StepTiming` (Struct)

Métricas temporales registradas en el último avance de fotograma (`step_frame`).

**Firma:**
```rust
pub struct StepTiming
```

### `GameRunner` (Struct)

Ejecutor del juego que conecta el hardware emulado con la generación del framebuffer RGBA.

**Firma:**
```rust
pub struct GameRunner
```

### `new` (Función)

Inicializa una nueva sesión de juego a partir de los bytes de la ROM
y un BIOS opcional (si es `None`, se utiliza el BIOS HLE embebido en `gba-core`).

**Firma:**
```rust
pub fn new(rom: Vec<u8>, bios: Option<&[u8]>, screen_kind: ScreenKind) -> Self
```

### `set_block_table` (Función)

Asigna o reemplaza la tabla de bloques recompilados nativos.

**Firma:**
```rust
pub fn set_block_table(&mut self, table: Option<crate::recomp::BlockTable>)
```

### `is_native` (Función)

Indica si el runner está ejecutando con la tabla nativa AOT cargada.

**Firma:**
```rust
pub fn is_native(&self) -> bool
```

### `set_gpu_ppu_enabled` (Función)

Activa o desactiva el renderizado por GPU mediante PPU Shim.

**Firma:**
```rust
pub fn set_gpu_ppu_enabled(&mut self, enabled: bool)
```

### `is_gpu_ppu_enabled` (Función)

Indica si el renderizado PPU Shim en GPU esta activo.

**Firma:**
```rust
pub fn is_gpu_ppu_enabled(&self) -> bool
```

### `io_lineas` (Función)

Registros de E/S capturados por scanline (0..160).

**Firma:**
```rust
pub fn io_lineas(&self) -> &[[u8; 0x60]; 160]
```

### `paleta_lineas` (Función)

Paleta capturada por scanline (0..160).

**Firma:**
```rust
pub fn paleta_lineas(&self) -> &[[u8; 1024]; 160]
```

### `vram` (Función)

Referencia a la VRAM de la GBA (96 KB).

**Firma:**
```rust
pub fn vram(&self) -> &[u8]
```

### `oam` (Función)

Referencia a la OAM de la GBA (1 KB).

**Firma:**
```rust
pub fn oam(&self) -> &[u8]
```

### `palette` (Función)

Referencia a la memoria de paletas de la GBA (1 KB).

**Firma:**
```rust
pub fn palette(&self) -> &[u8]
```

### `io` (Función)

Referencia a los registros de I/O de la GBA.

**Firma:**
```rust
pub fn io(&self) -> &[u8]
```

### `set_keys` (Función)

Inyecta la máscara de botones activa en bajo en el bus de hardware (`KEYINPUT`).

Bit = 0 significa botón pulsado; Bit = 1 significa botón liberado.

**Firma:**
```rust
pub fn set_keys(&mut self, keys: u16)
```

### `keys` (Función)

Obtiene la máscara actual de botones configurada en el bus de hardware.

**Firma:**
```rust
pub fn keys(&self) -> u16
```

### `step_frame` (Función)

Avanza la simulación hasta completar el siguiente fotograma visible.

Retorna `true` si se completó un fotograma (inicio de VBlank) y el búfer
RGBA contiene los nuevos píxeles listos para presentar en GPU.

**Firma:**
```rust
pub fn step_frame(&mut self) -> bool
```

### `last_timing` (Función)

Retorna las métricas temporales del último avance de fotograma.

**Firma:**
```rust
pub fn last_timing(&self) -> StepTiming
```

### `frame_rgba` (Función)

Referencia de solo lectura al búfer de píxeles RGBA (240x160) compuesto.

**Firma:**
```rust
pub fn frame_rgba(&self) -> &[[u8; 4]]
```

### `frames_elapsed` (Función)

Total de fotogramas completados desde la inicialización.

**Firma:**
```rust
pub fn frames_elapsed(&self) -> u64
```

### `clock` (Función)

Ciclos de reloj maestro acumulados por la consola.

**Firma:**
```rust
pub fn clock(&self) -> u64
```

### `set_screen_kind` (Función)

Actualiza el perfil de simulación de pantalla (LUT y respuesta temporal).

**Firma:**
```rust
pub fn set_screen_kind(&mut self, screen_kind: ScreenKind)
```

---

## Módulo `recomp.rs`

Subsistema de despacho y ejecución de bloques C11 recompilados Ahead-Of-Time (AOT).

Conecta los 144.875 bloques nativos de ejecución C11 (`rcg_blocks`), agrupados bajo 18.483 funciones catalogadas,
con el runtime de `gba-core`, ejecutando la lógica del juego directamente
en la CPU física sin emulación de instrucciones por software.

### `BlockTable` (Struct)

Tabla de despacho indexada O(1) para ejecución nativa de bloques.

**Firma:**
```rust
pub struct BlockTable
```

### `empty` (Función)

Crea una tabla vacía (fuerza fallback completo al intérprete).

**Firma:**
```rust
pub fn empty() -> Self
```

### `from_blocks` (Función)

Construye una tabla indexada a partir de un arreglo de bloques `RcgBlock`.

**Firma:**
```rust
pub fn from_blocks(blocks: &[RcgBlock]) -> Self
```

### `len` (Función)

Cantidad total de bloques nativos registrados en la tabla.

**Firma:**
```rust
pub fn len(&self) -> usize
```

### `is_empty` (Función)

Indica si la tabla no contiene ningún bloque nativo.

**Firma:**
```rust
pub fn is_empty(&self) -> bool
```

### `get` (Función)

Resuelve la función nativa para una clave de dirección (con bit 0 = Thumb) en O(1).

**Firma:**
```rust
pub fn get(&self, key: u32) -> Option<BlockFn>
```

### `run_frame_native` (Función)

Ejecuta un fotograma de la consola utilizando bloques C11 nativos con fallback al intérprete.

Retorna la tupla `(bloques_nativos_ejecutados, pasos_interprete_fallback)`.

**Firma:**
```rust
pub fn run_frame_native( m: &mut Machine, table: &BlockTable, max_steps: u64, ) -> (u64, u64)
```

### `ROM_REGION_START` (Constante)

Rango de direcciones de la memoria ROM de Game Boy Advance (Cartucho).

**Firma:**
```rust
pub const ROM_REGION_START: u32
```

### `ROM_REGION_END` (Constante)

*Sin comentario de documentación en código fuente.*

**Firma:**
```rust
pub const ROM_REGION_END: u32
```

### `Region` (Enum)

Región de memoria ejecutable de Game Boy Advance.

**Firma:**
```rust
pub enum Region
```

### `NAMES` (Constante)

Nombres cortos, en el orden canónico de la telemetría `# regiones`.

**Firma:**
```rust
pub const NAMES: [&'static str; 4]
```

### `index` (Función)

Índice de la región dentro de los arreglos de [`RegionCounters`].

**Firma:**
```rust
pub fn index(self) -> usize
```

### `RegionCounters` (Struct)

Ejecución acumulada por región de memoria desde el arranque de la sesión.

**Firma:**
```rust
pub struct RegionCounters
```

### `is_empty` (Función)

Indica si todavía no se ha registrado ninguna ejecución.

**Firma:**
```rust
pub fn is_empty(&self) -> bool
```

### `to_line` (Función)

Formatea la línea de telemetría `# regiones`, con un par `interprete/nativos` por región.

**Firma:**
```rust
pub fn to_line(&self) -> String
```

### `PpuPathCounters` (Struct)

Reparto acumulado de las líneas de la PPU entre los caminos de la composición.

Las cuatro cuentas suman las líneas dibujadas desde el arranque de la sesión: cada línea cae en
una sola razón, la primera que impide usar la vía rápida (efectos de color, ventana de sprites o
sprite translúcido). Es el dato que dice si la vía rápida se ejecuta en juego real y, si no, por
qué.

**Firma:**
```rust
pub struct PpuPathCounters
```

### `is_empty` (Función)

Indica si todavía no se ha contado ninguna línea.

**Firma:**
```rust
pub fn is_empty(&self) -> bool
```

### `total` (Función)

Total de líneas contadas, que debe cerrar en múltiplos de 160 (las líneas de un fotograma).

**Firma:**
```rust
pub fn total(&self) -> u64
```

### `rectas_pct` (Función)

Porcentaje de líneas que tomaron la vía rápida, redondeado al entero (0 sin líneas contadas).

**Firma:**
```rust
pub fn rectas_pct(&self) -> u64
```

### `to_line` (Función)

Formatea la línea de telemetría `# ppucamino` con las cuatro cuentas, el reparto de los
efectos y el porcentaje de líneas por la vía rápida.

**Firma:**
```rust
pub fn to_line(&self) -> String
```

### `record_hotspot` (Función)

Registra una dirección ejecutada en el intérprete de respaldo si pertenece a regiones ejecutables
(ROM, IWRAM, EWRAM o BIOS).

**Firma:**
```rust
pub fn record_hotspot(pc: u32)
```

### `drain_top_hotspots` (Función)

Extrae las `limit` direcciones ejecutadas con más frecuencia y **consume solo esas**.

Las direcciones que no entran en el `top` conservan su contador y vuelven a competir en el
volcado siguiente, de modo que ninguna ejecución registrada se descarta. Un `limit` mayor o
igual que el número de direcciones registradas deja el acumulador vacío.

**Firma:**
```rust
pub fn drain_top_hotspots(limit: usize) -> Vec<(u32, u64)>
```

### `add_native_region_ticks` (Función)

Acumula los bloques nativos ejecutados en un fotograma, agrupados por región.

Se invoca una sola vez por fotograma y no una vez por bloque, para no añadir
sincronización al camino caliente de despacho nativo.

**Firma:**
```rust
pub(crate) fn add_native_region_ticks(counts: &[u64; 4])
```

### `add_ppu_path` (Función)

Acumula el reparto de líneas del fotograma que el motor acaba de medir.

Se invoca una sola vez por fotograma (no una vez por línea) desde el bucle de emulación, así
que no añade sincronización al camino caliente: el motor cuenta por línea sin bloqueo y aquí
se vuelca el total del fotograma.

**Firma:**
```rust
pub(crate) fn add_ppu_path( rectas: u64, mezcla: u64, ventana: u64, semi: u64, inertes: u64, efecto: u64, efalpha: u64, efbrillo: u64, efoscuro: u64, )
```

### `ppu_path_counters` (Función)

Devuelve una copia del reparto acumulado de líneas por camino. No reinicia los contadores.

**Firma:**
```rust
pub fn ppu_path_counters() -> PpuPathCounters
```

### `region_counters` (Función)

Devuelve una copia de la ejecución acumulada por región de memoria. No reinicia los contadores.

**Firma:**
```rust
pub fn region_counters() -> RegionCounters
```

### `resolve_sibling_library_path` (Función)

Resuelve la ruta a una biblioteca hermana contigua a partir de la ruta absoluta
del módulo o ejecutable actual (usado para resolver `libblazing_recompiled.so` junto a `libblazing_recomp.so`).

**Firma:**
```rust
pub(crate) fn resolve_sibling_library_path(self_path: &str, sibling_name: &str) -> Option<std::path::PathBuf>
```

### `open_library` (Función)

*Sin comentario de documentación en código fuente.*

**Firma:**
```rust
pub unsafe fn open_library(name: &str) -> Option<*mut c_void>
```

### `get_symbol` (Función)

*Sin comentario de documentación en código fuente.*

**Firma:**
```rust
pub unsafe fn get_symbol<T>(handle: *mut c_void, name: &str) -> Option<T>
```

### `load_native_blocks` (Función)

Intenta cargar la biblioteca de bloques C11 recompilados (`libblazing_recompiled.so`)
presente en el paquete APK en Android. Retorna `None` si la biblioteca no está presente
(en host o entornos de prueba pura), activando el modo intérprete de respaldo.

**Firma:**
```rust
pub fn load_native_blocks() -> Option<BlockTable>
```

---

## Módulo `fps.rs`

Contador de FPS y generador de HUD visual en pantalla.

Calcula la tasa de refresco en tiempo real con amortiguación y rasteriza
una pastilla (badge) visual estilizada en un búfer RGBA de 80x26 px utilizando
la tipografía vectorial [`crate::overlay::JETBRAINS_FONT_BYTES`].

Este módulo es puro, sin dependencias de Android, y testeable en host.

### `BADGE_WIDTH` (Constante)

*Sin comentario de documentación en código fuente.*

**Firma:**
```rust
pub const BADGE_WIDTH: usize
```

### `BADGE_HEIGHT` (Constante)

*Sin comentario de documentación en código fuente.*

**Firma:**
```rust
pub const BADGE_HEIGHT: usize
```

### `FpsCounter` (Struct)

Contador de fotogramas y generador del badge visual de FPS.

**Firma:**
```rust
pub struct FpsCounter
```

### `new` (Función)

Inicializa el contador cargando la tipografía JetBrains Mono de assets.

**Firma:**
```rust
pub fn new() -> Result<Self, String>
```

### `set_native` (Función)

Actualiza el modo del motor (true = AOT nativo C11, false = Intérprete).

**Firma:**
```rust
pub fn set_native(&mut self, is_native: bool)
```

### `is_native` (Función)

Indica si el motor está reportando modo nativo AOT.

**Firma:**
```rust
pub fn is_native(&self) -> bool
```

### `set_renderer` (Función)

Actualiza el nombre del renderer activo de la GPU (ej. "Vulkan" u "OpenGL").

**Firma:**
```rust
pub fn set_renderer(&mut self, renderer: &'static str)
```

### `renderer` (Función)

Nombre del backend gráfico activo reportado en el badge.

**Firma:**
```rust
pub fn renderer(&self) -> &'static str
```

### `dimensions` (Función)

Dimensiones del badge en píxeles `(ancho, alto)`.

**Firma:**
```rust
pub fn dimensions(&self) -> (u32, u32)
```

### `fps` (Función)

Tasa de fotogramas actual estimada.

**Firma:**
```rust
pub fn fps(&self) -> f32
```

### `is_dirty` (Función)

Indica si la textura en GPU necesita actualizarse con nuevos píxeles.

**Firma:**
```rust
pub fn is_dirty(&self) -> bool
```

### `mark_clean` (Función)

Marca la textura como sincronizada tras subirla a la GPU.

**Firma:**
```rust
pub fn mark_clean(&mut self)
```

### `badge_rgba` (Función)

Píxeles RGBA del badge actual (216x34).

**Firma:**
```rust
pub fn badge_rgba(&self) -> &[[u8; 4]]
```

### `on_frame` (Función)

Registra el avance de un solo fotograma en el instante actual.

**Firma:**
```rust
pub fn on_frame(&mut self)
```

### `on_frames` (Función)

Registra el avance de `count` fotogramas de simulación en el instante actual.

**Firma:**
```rust
pub fn on_frames(&mut self, count: u32)
```

### `on_frame_at` (Función)

Registra el avance de un solo fotograma con un timestamp explícito (tests deterministas).

**Firma:**
```rust
pub fn on_frame_at(&mut self, now: Instant)
```

### `on_frames_at` (Función)

Registra el avance de `count` fotogramas con un timestamp explícito (tests deterministas).

**Firma:**
```rust
pub fn on_frames_at(&mut self, count: u32, now: Instant)
```

### `render_badge` (Función)

Rasteriza el badge con fondo de pastilla translúcida y texto con la tipografía de assets.

**Firma:**
```rust
pub fn render_badge(&mut self, fps_val: u32)
```

---

## Módulo `perf.rs`

Módulo de medición de rendimiento por fase del fotograma.

Permite registrar tiempos de ejecución de CPU, conversión de color,
dibujo en GPU, espera de swapchain/VSync y vueltas vacías del bucle,
generando un informe consolidado por segundo para diagnóstico sin
distorsionar el hilo principal.

### `FrameTiming` (Struct)

Medición de las fases de un único fotograma completado.

**Firma:**
```rust
pub struct FrameTiming
```

### `PpuHwFrameSample` (Struct)

Muestreo de características de hardware GBA en un fotograma completado.

**Firma:**
```rust
pub struct PpuHwFrameSample
```

### `PpuHwWindowStats` (Struct)

Estadísticas agregadas de hardware GBA para una ventana de telemetría de 1 segundo.

**Firma:**
```rust
pub struct PpuHwWindowStats
```

### `to_line` (Función)

Formatea la línea de telemetría de hardware GBA (`# ppu_hw`).

**Firma:**
```rust
pub fn to_line(&self) -> String
```

### `PerfReport` (Struct)

Informe de rendimiento consolidado correspondiente a un segundo de ejecución.

**Firma:**
```rust
pub struct PerfReport
```

### `to_line` (Función)

Formatea el informe en una única línea, guiada por el esquema de [`crate::telemetria`].

Ya no es un `format!` de 28 huecos: los valores se recogen en un mapa y
`telemetria::escribir_linea` los imprime en el orden canónico del esquema y
remata la línea con el sello de versión del contrato. La firma no cambia, así
que `platform.rs` sigue escribiendo el resultado tal cual.

Si el contrato está roto (un campo del esquema sin valor, por ejemplo), devuelve
una línea `#` con el motivo en vez de entrar en pánico: el fallo se ve en
`blazing-perf.txt` y no tumba la partida.

**Firma:**
```rust
pub fn to_line(&self) -> String
```

### `FramePerf` (Struct)

Acumulador de métricas de rendimiento por fase con ventanas temporales de 1 segundo.

**Firma:**
```rust
pub struct FramePerf
```

### `new` (Función)

Inicializa un nuevo acumulador de rendimiento.

**Firma:**
```rust
pub fn new() -> Self
```

### `new_at` (Función)

Inicializa el acumulador con un instante de inicio específico (útil para pruebas deterministas).

**Firma:**
```rust
pub fn new_at(start: Instant) -> Self
```

### `record_empty_turn` (Función)

Registra una vuelta vacía del bucle de eventos (sin avance de fotograma ni redibujado).

**Firma:**
```rust
pub fn record_empty_turn(&mut self)
```

### `record_frame` (Función)

Registra las métricas de un fotograma de simulación completado.

**Firma:**
```rust
pub fn record_frame(&mut self, timing: FrameTiming)
```

### `record_ppu_hw` (Función)

Registra el muestreo de características de hardware GBA para el fotograma.

**Firma:**
```rust
pub fn record_ppu_hw(&mut self, sample: PpuHwFrameSample)
```

### `close_window` (Función)

Cierra la ventana actual y genera un informe de rendimiento con los datos acumulados.

**Firma:**
```rust
pub fn close_window(&mut self, ventana: (u32, u32)) -> PerfReport
```

### `tick_at` (Función)

Evalúa si ha transcurrido un segundo desde el inicio de la ventana y, si es así,
cierra la ventana y retorna el informe.

**Firma:**
```rust
pub fn tick_at(&mut self, now: Instant, ventana: (u32, u32)) -> Option<PerfReport>
```

### `measure_clock_cost_ns` (Función)

Mide el costo medio de una lectura de reloj del sistema (`Instant::now()`) en nanosegundos.

**Firma:**
```rust
pub fn measure_clock_cost_ns() -> u64
```

### `audio_timing_flag_enabled` (Función)

Indica si la medicion del coste de audio esta activada: existe el archivo
`perf-audio-timing.flag` en alguno de los directorios de telemetria.

**Firma:**
```rust
pub fn audio_timing_flag_enabled(search_dirs: &[std::path::PathBuf]) -> bool
```

### `resolve_perf_candidates` (Función)

Resuelve la lista ordenada de directorios candidatos para persistir `blazing-perf.txt`,
priorizando la carpeta compartida Descargas sobre el almacenamiento privado de la app.

**Firma:**
```rust
pub fn resolve_perf_candidates(search_dirs: &[std::path::PathBuf]) -> Vec<std::path::PathBuf>
```

---

## Módulo `telemetria.rs`

Contrato de la línea de segundo (`t=`) de la telemetría de rendimiento.

El contrato vive en [`ESQUEMA_TEXTO`] (`telemetria/esquema.txt`), un fichero de
texto plano que es el **dueño único** de la lista de campos: lo lee este módulo
para escribir y `scripts/diagnostico/analizar_telemetria.py` para leer. Añadir
un campo se hace en el esquema, y los dos lados lo entienden sin tocar código.

Antes de este módulo el contrato estaba escrito cuatro veces (el `format!` de
`PerfReport::to_line`, el regex posicional de Python, la tabla de la guía y su
línea de ejemplo) y la deriva entre ellos fallaba en silencio: el regex no casaba,
el segundo no se contaba y el informe salía con medias sesgadas sin avisar.

El esquema se analiza una vez (`OnceLock`) y un esquema mal formado es un `Err`,
nunca un pánico: un contrato roto tiene que verse, no tumbar el juego.

### `ESQUEMA_TEXTO` (Constante)

Texto del esquema, embebido en la biblioteca en tiempo de compilación.

Va embebido (`include_str!`) y no como asset: el APK no lo lleva como archivo
y el módulo no necesita ni sistema de ficheros ni dependencias nuevas.

**Firma:**
```rust
pub const ESQUEMA_TEXTO: &str
```

### `Tipo` (Enum)

Cómo se imprime un campo de la línea de segundo.

**Firma:**
```rust
pub enum Tipo
```

### `nombre` (Función)

Nombre del tipo tal como se declara en el esquema.

**Firma:**
```rust
pub fn nombre(&self) -> &'static str
```

### `desde_texto` (Función)

Interpreta el nombre de un tipo del esquema.

**Firma:**
```rust
pub fn desde_texto(texto: &str) -> Option<Tipo>
```

### `Campo` (Struct)

Un campo declarado en el esquema.

**Firma:**
```rust
pub struct Campo
```

### `Valor` (Enum)

Valor de un campo, ya tipado, listo para imprimirse.

**Firma:**
```rust
pub enum Valor
```

### `Esquema` (Struct)

El contrato analizado: los campos en orden canónico y el sello de versión.

**Firma:**
```rust
pub struct Esquema
```

### `campos` (Función)

Campos del contrato, en el orden canónico en que se escriben.

**Firma:**
```rust
pub fn campos(&self) -> &[Campo]
```

### `sello_clave` (Función)

Clave del sello de versión (`formato`).

**Firma:**
```rust
pub fn sello_clave(&self) -> &'static str
```

### `sello_version` (Función)

Versión del contrato que emite el escritor.

**Firma:**
```rust
pub fn sello_version(&self) -> u32
```

### `campo` (Función)

Busca un campo por su nombre.

**Firma:**
```rust
pub fn campo(&self, nombre: &str) -> Option<&Campo>
```

### `esquema` (Función)

Devuelve el esquema del contrato, analizado la primera vez que se pide.

**Firma:**
```rust
pub fn esquema() -> Result<&'static Esquema, String>
```

### `escribir_linea` (Función)

Escribe una línea de segundo completa a partir de los valores de sus campos.

Recorre el esquema en orden canónico, imprime cada valor con la forma de su
tipo y remata la línea con el sello (`formato=2`). Falla nombrando el campo en
los tres casos que puede haber: falta un valor, sobra un valor que el esquema
no declara, o el valor no encaja con el tipo declarado.

**Firma:**
```rust
pub fn escribir_linea(valores: &BTreeMap<&'static str, Valor>) -> Result<String, String>
```

---

## Módulo `pipeline.rs`

Módulo de pipeline multihilo para desacoplar la emulación del renderizado GPU.

Permite que la CPU de GBA y el código C recompilado corran en un hilo dedicado
a 60 Hz regulados, depositando fotogramas completos en un puente de comunicación
acotado para que el hilo principal de Android los consuma y presente en la GPU
sin retrasos ni contención de cerrojos.

### `GBA_W` (Constante)

Ancho nativo del panel LCD de Game Boy Advance en píxeles.

**Firma:**
```rust
pub const GBA_W: usize
```

### `GBA_H` (Constante)

Alto nativo del panel LCD de Game Boy Advance en píxeles.

**Firma:**
```rust
pub const GBA_H: usize
```

### `GBA_PIXELS` (Constante)

Total de píxeles en un fotograma de Game Boy Advance.

**Firma:**
```rust
pub const GBA_PIXELS: usize
```

### `FRAME_DURATION` (Constante)

Duración exacta de un fotograma de GBA a 60 FPS (16.666 ms).

**Firma:**
```rust
pub const FRAME_DURATION: Duration
```

### `PpuFrameState` (Struct)

Estado de memoria de video para renderizado acelerado en GPU (PPU Nativa).

**Firma:**
```rust
pub struct PpuFrameState
```

### `new` (Función)

*Sin comentario de documentación en código fuente.*

**Firma:**
```rust
pub fn new() -> Self
```

### `FramePacket` (Struct)

Paquete inmutable transferido desde el hilo de emulación al hilo de renderizado.

**Firma:**
```rust
pub struct FramePacket
```

### `EmulationBridge` (Struct)

*Sin comentario de documentación en código fuente.*

**Firma:**
```rust
pub struct EmulationBridge
```

### `new` (Función)

Crea un nuevo puente con buzón de un solo slot para latencia mínima.

**Firma:**
```rust
pub fn new() -> Self
```

### `new_with_waker` (Función)

Crea un nuevo puente con un notificador de eventos opcional.

**Firma:**
```rust
pub fn new_with_waker(waker: Option<WakeFn>) -> Self
```

### `set_waker` (Función)

Configura o reemplaza el notificador de despertar del puente.

**Firma:**
```rust
pub fn set_waker(&self, waker: Option<WakeFn>)
```

### `acquire_buffer` (Función)

Adquiere un búfer de fotograma reutilizable del pool.

Solo asigna memoria nueva (`Box::new`) si el pool está vacío; en régimen
permanente el pool recicla los búferes que el buzón va liberando.

**Firma:**
```rust
pub fn acquire_buffer(&self) -> Box<[[u8; 4]; GBA_PIXELS]>
```

### `recycle_buffer` (Función)

Devuelve un búfer al pool para su reutilización.

El pool se acota para no retener memoria indefinidamente si un consumidor
lento dejara de reciclar.

**Firma:**
```rust
pub fn recycle_buffer(&self, buf: Box<[[u8; 4]; GBA_PIXELS]>)
```

### `acquire_ppu_state` (Función)

Adquiere un contenedor de estado PPU reutilizable del pool.

**Firma:**
```rust
pub fn acquire_ppu_state(&self) -> PpuFrameState
```

### `recycle_ppu_state` (Función)

Devuelve un contenedor de estado PPU al pool para su reutilización.

**Firma:**
```rust
pub fn recycle_ppu_state(&self, state: PpuFrameState)
```

### `is_gpu_ppu_enabled` (Función)

Consulta si el modo de renderizado acelerado en GPU (PPU Nativa) está activo.

**Firma:**
```rust
pub fn is_gpu_ppu_enabled(&self) -> bool
```

### `set_gpu_ppu_enabled` (Función)

Activa o desactiva el renderizado acelerado en GPU (PPU Nativa).

**Firma:**
```rust
pub fn set_gpu_ppu_enabled(&self, enabled: bool)
```

### `send_frame` (Función)

Deposita el fotograma más reciente en el buzón **sin bloquearse**.

Si ya había un fotograma pendiente, se descarta y su búfer vuelve al pool:
el consumidor recibirá siempre el fotograma más nuevo.

**Firma:**
```rust
pub fn send_frame(&self, packet: FramePacket)
```

### `try_recv_frame` (Función)

Retira el fotograma más reciente del buzón de forma no bloqueante (hilo de GPU).

Devuelve `None` si no hay ningún fotograma pendiente.

**Firma:**
```rust
pub fn try_recv_frame(&self) -> Option<FramePacket>
```

### `allocations` (Función)

Número de asignaciones reales de búfer realizadas desde el arranque.

En régimen permanente debe permanecer constante (pool reciclando); se usa
para verificar la ausencia de asignaciones por fotograma.

**Firma:**
```rust
pub fn allocations(&self) -> u64
```

### `set_keys` (Función)

Actualiza la máscara de entrada (llamada desde el hilo de UI/touch).

**Firma:**
```rust
pub fn set_keys(&self, keys: u16)
```

### `read_keys` (Función)

Lee la máscara de entrada actual (llamada desde el hilo de emulación).

**Firma:**
```rust
pub fn read_keys(&self) -> u16
```

### `is_running` (Función)

Consulta si el hilo de emulación debe continuar en ejecución.

**Firma:**
```rust
pub fn is_running(&self) -> bool
```

### `stop` (Función)

Detiene el hilo de emulación y despierta cualquier espera en pausa.

**Firma:**
```rust
pub fn stop(&self)
```

### `set_paused` (Función)

Activa o desactiva el estado de pausa (reposo energético).

**Firma:**
```rust
pub fn set_paused(&self, paused: bool)
```

### `is_paused` (Función)

Consulta si la emulación está en pausa.

**Firma:**
```rust
pub fn is_paused(&self) -> bool
```

### `wait_if_paused` (Función)

Si está en pausa, bloquea el hilo actual en el Condvar hasta que se reanude o se detenga.

**Firma:**
```rust
pub fn wait_if_paused(&self)
```

### `EmulationWorker` (Struct)

Gestor y controlador del hilo de trabajo de emulación en segundo plano.

**Firma:**
```rust
pub struct EmulationWorker
```

### `start` (Función)

Inicia un nuevo hilo de trabajo dedicado para ejecutar el GameRunner.

**Firma:**
```rust
pub fn start( rom: Vec<u8>, bios: Option<Vec<u8>>, screen_kind: ScreenKind, block_table: Option<crate::recomp::BlockTable>, ) -> Self
```

### `start_with_waker` (Función)

Inicia un nuevo hilo de trabajo dedicado con un notificador de despertar.

**Firma:**
```rust
pub fn start_with_waker( rom: Vec<u8>, bios: Option<Vec<u8>>, screen_kind: ScreenKind, block_table: Option<crate::recomp::BlockTable>, waker: Option<WakeFn>, ) -> Self
```

### `bridge` (Función)

Acceso al puente de sincronización del worker.

**Firma:**
```rust
pub fn bridge(&self) -> &Arc<EmulationBridge>
```

### `stop` (Función)

Detiene y espera a que el hilo finalice de forma limpia.

**Firma:**
```rust
pub fn stop(&mut self)
```

---

## Módulo `present.rs`

Calibración de presentación del swapchain (módulo puro, testeable en host).

**Contexto medido (tablet Snapdragon 610, `blazing-perf-4.txt`):** el
fotograma se bloquea en `frame.present()` (`present` ≈ 52 ms en retrato y
≈ 33 ms en apaisado, esto es 3,34× y 2,18× el intervalo de VSync), mientras
que el coste propio del port (`subida + codificacion + espera`) es de
~2,5 ms y en retrato se dibuja *menos* que en apaisado. La causa no es
nuestra carga, es el acoplamiento con el compositor.

`wgpu-hal` pide al driver `min_image_count = latencia + 1`
(`vulkan/device.rs`, `create_swapchain`), de modo que
`desired_maximum_frame_latency: 1` deja el swapchain con **2 imágenes**:
`Mailbox` no puede reemplazar un fotograma pendiente (necesita una tercera
imagen donde renderizar) y `queue_present` degenera en bloqueo hasta que el
compositor libera un búfer.

Este módulo recorre unas pocas configuraciones al arrancar, mide `present`
en cada una y elige la mejor. La política es conservadora a propósito:
solo adopta cambios con mejora medible sobre la referencia.

Dos de los tramos son **sondas** (rol [`Rol::Sonda`]), medidas pero nunca
adoptadas:
- presentar sin VSync, para saber si el bloqueo de `present` es de
  sincronización o trabajo real del compositor (adoptarlo produciría
  *tearing*);
- renderizar a [`ESCALA_REDUCIDA`], para saber si el coste escala con los
  píxeles. Bajarla de forma permanente es una decisión de nitidez que ya se
  descartó explícitamente en
  `docs/propuestas/2026-10-10_optimizacion-rendimiento-gpu-vulkan-y-calibracion-fps.md`,
  así que esta calibración solo aporta el dato, no la aplica.

### `SEGUNDOS_POR_TRAMO` (Constante)

Segundos de telemetría (ventanas de 1 s) que dura cada configuración.

**Firma:**
```rust
pub const SEGUNDOS_POR_TRAMO: usize
```

### `MARGEN_MEJORA` (Constante)

Mejora mínima exigida para adoptar otra configuración no sonda.

**Firma:**
```rust
pub const MARGEN_MEJORA: f64
```

### `ESCALA_REDUCIDA` (Constante)

Factor de resolución del tramo de diagnóstico que reduce píxeles a renderizar.

**Firma:**
```rust
pub const ESCALA_REDUCIDA: f32
```

### `TOPE_SWAPCHAIN_LANDSCAPE` (Constante)

Dimensión máxima para el swapchain en modo apaisado (landscape).
960px equipara el área de renderizado (960x576 = 552k px) a la de retrato (576x960),
reduciendo el tiempo de presentación a <3 ms para sostener 60 FPS clavados con escalado 4x de GBA (240x160).

**Firma:**
```rust
pub const TOPE_SWAPCHAIN_LANDSCAPE: u32
```

### `TOPE_SWAPCHAIN_PORTRAIT` (Constante)

Dimensión máxima para el swapchain en modo retrato (portrait).
Debido a la carga adicional de fragment shader al componer la carcasa y controles GBA SP
en la mitad inferior, acotar la altura a 960px permite que el frame completo termine
en <16.6 ms, alcanzando los 60 FPS estables en vertical.

**Firma:**
```rust
pub const TOPE_SWAPCHAIN_PORTRAIT: u32
```

### `TOPE_DIMENSION_SWAPCHAIN` (Constante)

Alias de compatibilidad hacia atrás para el tope general.

**Firma:**
```rust
pub const TOPE_DIMENSION_SWAPCHAIN: u32
```

### `calcular_escala_swapchain` (Función)

Calcula la escala de render del swapchain respetando el tope de dimensión máxima
según la orientación de la pantalla (apaisado vs retrato).

**Firma:**
```rust
pub fn calcular_escala_swapchain(w: u32, h: u32) -> f32
```

### `Rol` (Enum)

Papel de un tramo dentro de la calibración.

**Firma:**
```rust
pub enum Rol
```

### `Candidato` (Struct)

Una configuración de presentación concreta del swapchain.

**Firma:**
```rust
pub struct Candidato
```

### `new` (Función)

Crea un candidato.

**Firma:**
```rust
pub fn new(latency: u32, present_mode: PresentMode, scale: f32, rol: Rol) -> Candidato
```

### `descripcion` (Función)

Descripción legible para la telemetría (`lat=2 modo=Mailbox escala=1.00`).
Las sondas y la referencia se marcan para que el log no invite a
preguntarse por qué no se adoptaron.

**Firma:**
```rust
pub fn descripcion(&self) -> String
```

### `Tramo` (Struct)

Medición acumulada de una configuración durante `SEGUNDOS_POR_TRAMO` segundos.

**Firma:**
```rust
pub struct Tramo
```

### `new` (Función)

Crea un tramo vacío para un candidato.

**Firma:**
```rust
pub fn new(candidato: Candidato) -> Tramo
```

### `muestra` (Función)

Registra una ventana de telemetría (1 s) de este tramo.

**Firma:**
```rust
pub fn muestra(&mut self, present_ms: f64, fps: f64)
```

### `completa` (Función)

`true` si el tramo ya reunió las ventanas necesarias.

**Firma:**
```rust
pub fn completa(&self) -> bool
```

### `valido` (Función)

`true` si al menos una ventana dibujó fotogramas: un tramo en el que el
compositor falló (por ejemplo, swapchain rechazado) no es comparable.

**Firma:**
```rust
pub fn valido(&self) -> bool
```

### `present_medio` (Función)

Promedio de `present` de las ventanas que sí dibujaron fotogramas.

**Firma:**
```rust
pub fn present_medio(&self) -> Option<f64>
```

### `fps_medio` (Función)

Promedio de fotogramas por segundo de las ventanas medidas.

**Firma:**
```rust
pub fn fps_medio(&self) -> f64
```

### `elegir_mejor` (Función)

Elige la configuración a adoptar a partir de los tramos medidos.

Devuelve el candidato elegido y el motivo (para la telemetría). Reglas:
1. Sin muestras válidas de la referencia → se mantiene la referencia.
2. La mejor candidata (`Rol::Adoptable`) gana a la referencia solo si la
   mejora en al menos `MARGEN_MEJORA` (5 %).
3. En cualquier otro caso se mantiene la referencia.

Los tramos con `Rol::Sonda` nunca se adoptan, por buenos que sean: son
mediciones para decidir, no configuraciones para aplicar.

**Firma:**
```rust
pub fn elegir_mejor(tramos: &[Tramo]) -> (Candidato, String)
```

### `PresentCalibration` (Struct)

Calibración de presentación: recorre una lista de configuraciones, mide
`present` en cada una y decide cuál aplicar.

**Firma:**
```rust
pub struct PresentCalibration
```

### `new` (Función)

Construye el recorrido de calibración.

`modo_base` es el modo de presentación ya configurado por el compositor
(referencia); `modos_disponibles` son los que ofrece el driver, y solo
se incluye la sonda sin VSync si `Immediate` está entre ellos.

**Firma:**
```rust
pub fn new(modo_base: PresentMode, modos_disponibles: &[PresentMode]) -> PresentCalibration
```

### `actual` (Función)

Configuración que debe estar activa ahora mismo.

**Firma:**
```rust
pub fn actual(&self) -> Candidato
```

### `tomar_pendiente` (Función)

Retira y devuelve la configuración pendiente de aplicar, si la hay.

**Firma:**
```rust
pub fn tomar_pendiente(&mut self) -> Option<Candidato>
```

### `finalizada` (Función)

`true` cuando ya se eligió configuración (no se miden más tramos).

**Firma:**
```rust
pub fn finalizada(&self) -> bool
```

### `total_tramos` (Función)

Número de tramos del recorrido (para diagnósticos y pruebas).

**Firma:**
```rust
pub fn total_tramos(&self) -> usize
```

### `registrar_segundo` (Función)

Registra una ventana de telemetría y devuelve las líneas de comentario
(`# ...`) que correspondan: apertura de tramo, cierre con promedio y
veredicto final.

**Firma:**
```rust
pub fn registrar_segundo(&mut self, present_ms: f64, fps: f64) -> Vec<String>
```

### `notificar_tamano` (Función)

Informa del tamaño de ventana observado. Un cambio de orientación
invalida la comparación entre tramos (el coste de composición depende
de la rotación): se reinicia una vez y, si vuelve a cambiar, se cierra
la calibración con lo medido hasta el momento.

**Firma:**
```rust
pub fn notificar_tamano(&mut self, tamano: (u32, u32)) -> Vec<String>
```

---

## Módulo `ppu_nativa.rs`

Módulo PPU nativa: dibuja el cuadro de la GBA con la GPU del dispositivo.

Sustituye a la PPU por software con un modelo de **composición por píxel**:
los fondos y los sprites se rasterizan a un G-buffer (una capa por fondo y una
para los sprites) y una pasada final elige, píxel a píxel, las dos capas más
altas y les aplica la **unidad de mezcla** (alfa, aclarar, oscurecer) y las
**ventanas 0, 1 y OBJ**.

La especificación normativa es la PPU por software de `gba-core`
(`crates/gba-core/src/ppu.rs`, rev fijado `de4edf59`): su `split6`/`join6`/
`alpha_blend`/`brighten`/`darken` y las reglas de `compose()`. Este módulo no
reimplementa la PPU completa: dibuja el cuadro en la GPU y reproduce esas
reglas, que es lo que el hardware de la GBA hace por sí solo.

### `GBA_SCREEN_W` (Constante)

Ancho de la pantalla visible de la GBA.

**Firma:**
```rust
pub const GBA_SCREEN_W: u32
```

### `GBA_SCREEN_H` (Constante)

Alto de la pantalla visible de la GBA.

**Firma:**
```rust
pub const GBA_SCREEN_H: u32
```

### `MAX_SPRITES` (Constante)

Número máximo de entradas de OAM.

**Firma:**
```rust
pub const MAX_SPRITES: usize
```

### `TRANSPARENTE` (Constante)

Centinela de «píxel transparente». El mismo valor que usa la referencia; no
puede colisionar con un color real porque la paleta va enmascarada a 15 bits.

**Firma:**
```rust
pub const TRANSPARENTE: u16
```

### `PPU_NATIVA_WGSL` (Constante)

Shader de la PPU nativa (capas, sprites y composición).

**Firma:**
```rust
pub const PPU_NATIVA_WGSL: &str
```

### `NOMBRE_BANDERA_APAGADO` (Constante)

Nombre del archivo de marca que **apaga** la PPU nativa en runtime.

La PPU nativa es la ruta de dibujo **por defecto**: un APK recién
instalado compone los píxeles en la GPU con la aritmética de la PPU
original de la GBA, sin que nadie tenga que tocar nada. Para volver al
*shim* (o al render por software) y poder comparar A/B en el propio
dispositivo, se crea este archivo; la app lo relee una vez por segundo en
los mismos directorios donde busca la telemetría.

El nombre es explícito a propósito: la marca de apagado convive sin
ambigüedad con el `ppu-nativa.flag` de las pruebas anteriores, que ya no se
lee (era el interruptor en el otro sentido).

**Firma:**
```rust
pub const NOMBRE_BANDERA_APAGADO: &str
```

### `apagada_por_marca` (Función)

Indica si la PPU nativa debe **apagarse** ahora: existe el archivo
[`NOMBRE_BANDERA_APAGADO`] en alguno de los directorios de telemetría.

Sigue el mismo patrón que [`crate::perf::audio_timing_flag_enabled`]: los
directorios son los candidatos de telemetría, que incluyen el directorio
externo propio de la app (el único accesible sin permisos especiales).

**Firma:**
```rust
pub fn apagada_por_marca(search_dirs: &[std::path::PathBuf]) -> bool
```

### `ppu_nativa_activada` (Función)

Indica si la PPU nativa debe usarse ahora.

La respuesta es «sí» salvo que exista la marca de apagado: la PPU nativa
es la ruta por defecto y el archivo solo sirve para volver atrás.

**Firma:**
```rust
pub fn ppu_nativa_activada(search_dirs: &[std::path::PathBuf]) -> bool
```

### `FILTROS_ERROR` (Constante)

Filtros de error de `wgpu` que cubre el guardián, uno por ámbito.

`wgpu` clasifica cada error en uno de estos tres filtros y lo guarda en el
ámbito más interno **cuyo filtro coincida** con el del error
(`ErrorSinkRaw::handle_error`, en `wgpu-26.0.1/src/backend/wgpu_core.rs`).
Un solo ámbito no bastaría: un error de memoria agotada no entraría en un
ámbito de validación y seguiría entrando en pánico.

**Firma:**
```rust
pub const FILTROS_ERROR: [wgpu::ErrorFilter; 3]
```

### `construir_con_guardian` (Función)

Construye un recurso de la GPU sin que un error de validación mate la app.

Abre un ámbito por cada filtro de error, ejecuta la construcción y cierra
los ámbitos. Si alguno recogió un error, lo devuelve como `Err` para que el
llamante pueda seguir por otra vía (en este caso, el *shim*).

**Firma:**
```rust
pub fn construir_con_guardian<T, M: MotorDeErrores>( motor: &M, etiqueta: &str, construir: impl FnOnce() -> T, ) -> Result<T, String>
```

### `BINDING_VRAM` (Constante)

VRAM completa (96 KB) como búfer de solo lectura.

**Firma:**
```rust
pub const BINDING_VRAM: u32
```

### `BINDING_PALETA` (Constante)

Paleta (512 colores RGB555) como búfer de solo lectura.

**Firma:**
```rust
pub const BINDING_PALETA: u32
```

### `BINDING_FILAS_CAPA` (Constante)

Tabla de configuración de capas por línea (160 filas) como búfer de solo lectura.

**Firma:**
```rust
pub const BINDING_FILAS_CAPA: u32
```

### `BINDING_SPRITES` (Constante)

Sprites de color, como búfer de solo lectura en fragmento.

**Firma:**
```rust
pub const BINDING_SPRITES: u32
```

### `BINDING_FILAS_COMP` (Constante)

Tabla de configuración de composición por línea (160 filas) como búfer de solo lectura.

**Firma:**
```rust
pub const BINDING_FILAS_COMP: u32
```

### `BINDING_TEX_OBJ` (Constante)

Textura del G-buffer de sprites (color, prioridad, semitransparencia).

**Firma:**
```rust
pub const BINDING_TEX_OBJ: u32
```

### `BINDING_TEX_OBJ_VENTANA` (Constante)

Textura de la máscara de la ventana OBJ.

**Firma:**
```rust
pub const BINDING_TEX_OBJ_VENTANA: u32
```

### `split6` (Función)

Canales como los ve la unidad de mezcla: rojo y azul de 5 bits, **verde en un
carril de 6 bits** (el valor de 5 bits duplicado).

El bit bajo real de ese carril es el bit 15 de PRAM, que la referencia
enmascara al leer la paleta (`pal16`); aquí el color llega ya enmascarado.

**Firma:**
```rust
pub fn split6(c: u16) -> (u32, u32, u32)
```

### `join6` (Función)

Reensambla los canales del carril de la unidad de mezcla en un color de 15
bits, truncando el verde (el carril de 6 bits vuelve a 5).

**Firma:**
```rust
pub fn join6(r: u32, g: u32, b: u32) -> u16
```

### `alpha_blend` (Función)

Mezcla alfa de la unidad de mezcla: `a` es la capa superior y `b` la segunda.

Acumula con redondeo a media hacia arriba (`+8 >> 4`) y satura cada canal.

**Firma:**
```rust
pub fn alpha_blend(a: u16, b: u16, eva: u32, evb: u32) -> u16
```

### `brighten` (Función)

Aclarar (modo 2 de la unidad de mezcla): suma una fracción `evy/16` de lo que
le falta a cada canal para llegar al blanco.

**Firma:**
```rust
pub fn brighten(c: u16, evy: u32) -> u16
```

### `darken` (Función)

Oscurecer (modo 3 de la unidad de mezcla): resta una fracción `evy/16` de cada
canal. El sustraendo redondea a media hacia abajo (`+7`) para que el resultado
siga redondeando a media hacia arriba.

**Firma:**
```rust
pub fn darken(c: u16, evy: u32) -> u16
```

### `RegistrosColor` (Struct)

Registros de la unidad de mezcla: `BLDCNT`, `BLDALPHA` y `BLDY`.

**Firma:**
```rust
pub struct RegistrosColor
```

### `de_io` (Función)

Lee los tres registros de la I/O de la PPU.

**Firma:**
```rust
pub fn de_io(io: &[u8]) -> Self
```

### `modo` (Función)

Modo de la unidad de mezcla (bits 6-7 de `BLDCNT`): 0 ninguna,
1 alfa, 2 aclarar, 3 oscurecer.

**Firma:**
```rust
pub fn modo(&self) -> u32
```

### `eva` (Función)

EVA: bits 0-4 de `BLDALPHA`, saturado a 16 (`17..31` significa 16/16).

**Firma:**
```rust
pub fn eva(&self) -> u32
```

### `evb` (Función)

EVB: **bits 8-12** de `BLDALPHA` (no 5-9), saturado a 16.

**Firma:**
```rust
pub fn evb(&self) -> u32
```

### `evy` (Función)

EVY: bits 0-4 de `BLDY`, saturado a 16.

**Firma:**
```rust
pub fn evy(&self) -> u32
```

### `Ventanas` (Struct)

Ventanas 0, 1 y OBJ con sus registros de control.

**Firma:**
```rust
pub struct Ventanas
```

### `de_io` (Función)

Lee los registros de ventana de la I/O de la PPU y su habilitación en
`DISPCNT` (bits 13, 14 y 15).

**Firma:**
```rust
pub fn de_io(io: &[u8], dispcnt: u16) -> Self
```

### `alguna` (Función)

¿Hay alguna ventana activa? Sin ventanas la máscara de control es `0x3F`.

**Firma:**
```rust
pub fn alguna(&self) -> bool
```

### `en_rango` (Función)

Prueba de rango de ventana: `x1` es el borde izquierdo (incluido) y `x2` el
derecho (excluido). Si `x1 > x2` el rango da la vuelta.

**Firma:**
```rust
pub fn en_rango(x1: u8, x2: u8, v: u8) -> bool
```

### `mascara_control` (Función)

Máscara de control de un píxel: qué capas se ven y si hay efectos.

Bits 0-3: un fondo cada uno. Bit 4: sprites. Bit 5: efectos de color.

**Firma:**
```rust
pub fn mascara_control(vent: &Ventanas, x: u32, y: u32, obj_window: bool) -> u16
```

### `CAPA_OBJ` (Constante)

Id de capa: 0-3 fondos, 4 sprites, 5 fondo de pantalla.

**Firma:**
```rust
pub const CAPA_OBJ: u8
```

### `CAPA_FONDO` (Constante)

Id de capa del fondo de pantalla (el color 0 de la paleta).

**Firma:**
```rust
pub const CAPA_FONDO: u8
```

### `PixelObj` (Struct)

Píxel de sprite winner de un píxel de pantalla.

**Firma:**
```rust
pub struct PixelObj
```

### `VACIO` (Constante)

Sprite inexistente en ese píxel.

**Firma:**
```rust
pub const VACIO: PixelObj
```

### `compone_pixel` (Función)

Compone el píxel final aplicando las reglas de `compose()` de la referencia.

`bg` son los colores de los cuatro fondos (`TRANSPARENTE` si no se ve) y
`bg_prio` sus prioridades (`0xFF` si el fondo no se usa en este modo).

**Firma:**
```rust
pub fn compone_pixel( bg: &[u16; 4],
```

### `CapaFondoGpu` (Struct)

Configuración de un fondo (textos) tal y como la lee la GPU.

**Firma:**
```rust
pub struct CapaFondoGpu
```

### `de_io` (Función)

Lee la configuración de un fondo de la I/O de la PPU.

**Firma:**
```rust
pub fn de_io(io: &[u8], bg: usize, enabled: bool) -> Self
```

### `de_io_afin` (Función)

Lee la configuración de un fondo **afín** de la I/O de la PPU.

Los fondos afines tienen sus propios registros, aparte de `BGxCNT`:

- `PA`, `PB`, `PC` y `PD`: enteros de 16 bits con signo en punto fijo
  20.8 (`PA` y `PC` son el paso por píxel; `PB` y `PD`, el del ancla).
- `BGxX` y `BGxY`: el ancla, con **28 bits** con signo: el bit 27 es el
  signo y los cuatro bits de arriba no existen en el registro.

`BG2` usa `0x20`-`0x2C` y `BG3`, `0x30`-`0x3C`. Es la misma aritmética
que la referencia (`sign_extend28` y los registros leídos como `i16`).

**Firma:**
```rust
pub fn de_io_afin(io: &[u8], bg: usize, enabled: bool) -> Self
```

### `ConfigCapaGpu` (Struct)

Configuración de la pasada de capas: el modo de vídeo y los cuatro fondos.

**Firma:**
```rust
pub struct ConfigCapaGpu
```

### `ConfigCompGpu` (Struct)

Configuración de la pasada de composición.

**Firma:**
```rust
pub struct ConfigCompGpu
```

### `SpriteNativa` (Struct)

Sprite decodificado y listo para la GPU.

**Firma:**
```rust
pub struct SpriteNativa
```

### `SpriteInstanciaGpu` (Struct)

Datos de un sprite que necesita la etapa de vértices.

La lista completa de sprites viaja en un *storage buffer* que solo puede
leer la etapa de fragmento: un *storage buffer* visible en la etapa de
vértices exige la capacidad `DownlevelFlags::VERTEX_STORAGE`, que el
estándar de OpenGL ES permite no tener (un dispositivo puede admitir
SSBO solo en fragmento). Por eso el vértice recibe la geometría y la
clave de prioridad como atributos de instancia, que son WebGPU básico.

**Firma:**
```rust
pub struct SpriteInstanciaGpu
```

### `de_sprite` (Función)

Construye la instancia de un sprite ya decodificado.

**Firma:**
```rust
pub fn de_sprite(sprite: &SpriteNativa) -> Self
```

### `FLAG_HFLIP` (Constante)

Bandera: espejo horizontal.

**Firma:**
```rust
pub const FLAG_HFLIP: u32
```

### `FLAG_VFLIP` (Constante)

Bandera: espejo vertical.

**Firma:**
```rust
pub const FLAG_VFLIP: u32
```

### `FLAG_SEMI` (Constante)

Bandera: sprite semitransparente.

**Firma:**
```rust
pub const FLAG_SEMI: u32
```

### `FLAG_8BPP` (Constante)

Bandera: 8 bits por píxel.

**Firma:**
```rust
pub const FLAG_8BPP: u32
```

### `FLAG_1D` (Constante)

Bandera: mapeo 1D de los tiles.

**Firma:**
```rust
pub const FLAG_1D: u32
```

### `FLAG_AFIN` (Constante)

Bandera: sprite afín.

**Firma:**
```rust
pub const FLAG_AFIN: u32
```

### `OBJ_SIZES` (Constante)

Tamaños nominales de los sprites: `[forma][tamaño]`.

**Firma:**
```rust
pub const OBJ_SIZES: [[(u32, u32); 4]; 3]
```

### `EstadoPpuNativa` (Struct)

Estado de vídeo de un cuadro, decodificado de VRAM, OAM, paleta e I/O.

**Firma:**
```rust
pub struct EstadoPpuNativa
```

### `new` (Función)

Crea un estado vacío.

**Firma:**
```rust
pub fn new() -> Self
```

### `decode_io_regs` (Función)

Decodifica `DISPCNT`, `BGxCNT`, `BGxHOFS`, `BGxVOFS`, los registros de
mezcla y los de ventana.

**Firma:**
```rust
pub fn decode_io_regs(&mut self, io: &[u8])
```

### `decode_oam` (Función)

Decodifica la tabla OAM.

Se recorre **del último índice al primero** para que el orden estable por
prioridad (de mayor a menor) deje al frente, dentro de cada prioridad, el
sprite de índice OAM más bajo: es el que gana en la referencia.

La referencia solo dibuja sprites si `DISPCNT` bit 12 está activo.

**Firma:**
```rust
pub fn decode_oam(&mut self, oam: &[u8]) -> usize
```

### `config_capa` (Función)

Configuración de la pasada de capas.

**Firma:**
```rust
pub fn config_capa(&self) -> ConfigCapaGpu
```

### `config_comp` (Función)

Configuración de la pasada de composición.

**Firma:**
```rust
pub fn config_comp(&self) -> ConfigCompGpu
```

### `decodificar_lineas` (Función)

Decodifica las 160 filas de registros I/O en tablas de parámetros para la GPU.

**Firma:**
```rust
pub fn decodificar_lineas( io_lineas: &[[u8; 0x60]; 160],
```

### `decodificar_paletas_linea` (Función)

Decodifica las 160 paletas por línea para la GPU, enmascarando el bit 15 a 15 bits.

**Firma:**
```rust
pub fn decodificar_paletas_linea( paleta_lineas: &[[u8; 1024]; 160],
```

### `decodificar_paleta_uniforme` (Función)

Decodifica una única paleta uniforme replicándola en las 160 líneas.

**Firma:**
```rust
pub fn decodificar_paleta_uniforme( palette: &[u8], destino: &mut [[u32; 512]; 160],
```

### `rango_filas_modificadas` (Función)

Primer y último índice donde `previo` y `actual` difieren, o `None` si son
idénticos. Acota qué filas hay que volver a subir a la GPU.

**Firma:**
```rust
pub fn rango_filas_modificadas<T: PartialEq>(previo: &[T], actual: &[T]) -> Option<(usize, usize)>
```

### `sprites_modificados` (Función)

Comprueba si los sprites decodificados deben subirse a la GPU.

Si la lista actual está vacía (`actuales.is_empty()`), devuelve `false`
porque no hay geometría que dibujar. En caso contrario, devuelve `true`
si es el primer fotograma (`primera`) o si el contenido/longitud de la
lista difiere del fotograma previo.

**Firma:**
```rust
pub fn sprites_modificados<T: PartialEq>( primera: bool, previos: &[T], actuales: &[T], ) -> bool
```

### `QuadVertex` (Struct)

Vértice de quad unitario `[0..1]`.

**Firma:**
```rust
pub struct QuadVertex
```

### `QUAD_VERTICES` (Constante)

Los seis vértices de dos triángulos que cubren la pantalla.

**Firma:**
```rust
pub const QUAD_VERTICES: [QuadVertex; 6]
```

### `PpuNativaPipeline` (Struct)

Pipeline de la PPU nativa: composición directa por píxel y sprites en G-buffer.

**Firma:**
```rust
pub struct PpuNativaPipeline
```

### `new` (Función)

Crea el pipeline: texturas de G-buffer, shaders y grupos de enlaces.

La construcción va dentro del guardián de errores: si la GPU del
dispositivo rechaza algún recurso, se devuelve el error en vez de dejar
que `wgpu` entre en pánico, y el llamante puede seguir con el *shim*.

**Firma:**
```rust
pub fn new(device: &wgpu::Device, target_format: wgpu::TextureFormat) -> Result<Self, String>
```

### `render_frame` (Función)

Dibuja un cuadro completo en la vista de textura destino usando un único I/O.

**Firma:**
```rust
pub fn render_frame( &mut self, device: &wgpu::Device, queue: &wgpu::Queue, target_view: &wgpu::TextureView, vram: &[u8], oam: &[u8], palette: &[u8], io: &[u8], )
```

### `render_frame_lineas` (Función)

Dibuja un cuadro completo en la vista de textura destino, recibiendo opcionalmente
los registros I/O y las paletas capturadas en cada una de las 160 líneas activas.

Sube lo que cambió y envía los pases con un `CommandEncoder` propio. El
compositor de Android usa por separado [`Self::subir_buffers_lineas`] y
[`Self::grabar_pases`] para compartir un único `CommandEncoder` y un solo
`queue.submit()` con el pase de pantalla.

**Firma:**
```rust
pub fn render_frame_lineas( &mut self, device: &wgpu::Device, queue: &wgpu::Queue, target_view: &wgpu::TextureView, vram: &[u8], oam: &[u8], palette: &[u8], io_fin: &[u8], io_lineas: Option<&[[u8; 0x60]; 160]>,
```

### `subir_buffers_lineas` (Función)

Decodifica el cuadro y sube a la GPU **solo lo que cambió** desde el
cuadro anterior (VRAM, paleta por línea y tablas de capas/composición por
línea); la geometría del quad se sube una única vez.

**Firma:**
```rust
pub fn subir_buffers_lineas( &mut self, queue: &wgpu::Queue, vram: &[u8], oam: &[u8], palette: &[u8], io_fin: &[u8], io_lineas: Option<&[[u8; 0x60]; 160]>,
```

### `grabar_pases` (Función)

Graba en `encoder` los tres pases de la PPU nativa (sprites, ventana OBJ y
composición) sobre `target_view`. Requiere haber llamado antes a
[`Self::subir_buffers_lineas`] con los datos del cuadro.

**Firma:**
```rust
pub fn grabar_pases( &mut self, encoder: &mut wgpu::CommandEncoder, target_view: &wgpu::TextureView, )
```

---

## Sincronización y Mantenimiento

Para regenerar este documento ante cualquier modificación en las firmas o comentarios de `src/`:
```bash
python scripts/extraer_referencias.py
```

Para validar que la documentación se encuentre estrictamente actualizada (usado en tests y CI):
```bash
python scripts/extraer_referencias.py --check
```
