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
| [`input.rs`](#módulo-inputrs) | 19 | Entrada táctil ergonómica (Fase 5.1, propuesta |
| [`overlay.rs`](#módulo-overlayrs) | 12 | Overlay táctil y marco GBA SP (Fase 5.2, propuesta |
| [`error_screen.rs`](#módulo-error_screenrs) | 4 | Renderizador de pantalla de error por software y blindaje de GPU. |
| [`platform.rs`](#módulo-platformrs) | 2 | Plataforma Android (solo compila con `cfg(target_os = "android")`): |
| [`storage.rs`](#módulo-storagers) | 14 | Módulo interno del port. |
| [`runner.rs`](#módulo-runnerrs) | 25 | Ejecución y sincronización del motor de emulación GBA (Fase 4 y Fase 5). |
| [`recomp.rs`](#módulo-recomprs) | 30 | Subsistema de despacho y ejecución de bloques C11 recompilados Ahead-Of-Time (AOT). |
| [`fps.rs`](#módulo-fpsrs) | 18 | Contador de FPS y generador de HUD visual en pantalla. |
| [`telemetria.rs`](#módulo-telemetriars) | 13 | Contrato de la línea de segundo (`t=`) de la telemetría de rendimiento. |

---

## Módulo `lib.rs`

Blazing Recomp — punto de entrada nativo para Android.

Este crate es la biblioteca dinámica (`cdylib`) que Android carga al
arrancar la NativeActivity. Fases integradas:

- Fase 4 (propuesta `2026-10-08_renderizado-gpu-android`): presentación
  GPU vía [`screen::present::Presenter`] (wgpu: Vulkan/GLES), LUT de
  color y temporización temporal del panel.
- Fase 5.1 (propuesta `2026-10-08_controles-tactiles-haptica`): entrada
  táctil ergonómica (cruceta con cuenca, clúster diagonal GBA SP con
  L/B/A/R, Select/Start), auto-rotación de hitboxes, bandera
  `touch_enabled` y respuesta háptica por vibración vía JNI.

La lógica geométrica de entrada vive en [`crate::input`] (pura,
testeable en host, sin dependencias de Android). Todo lo que toca
JNI, NativeActivity o Logcat está tras `cfg(target_os = "android")`.

El motor `gba-core` ejecuta la lógica del juego y compone cada fotograma
en la PPU; la presentación lo muestra con la sincronía del panel.

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
Por defecto `false` (usar GPU PpuShim cuando esté disponible).

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

Conecta la tabla maestra de 17,207 funciones nativas (`rcg_blocks`)
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

## Sincronización y Mantenimiento

Para regenerar este documento ante cualquier modificación en las firmas o comentarios de `src/`:
```bash
python scripts/extraer_referencias.py
```

Para validar que la documentación se encuentre estrictamente actualizada (usado en tests y CI):
```bash
python scripts/extraer_referencias.py --check
```
