# Propuesta: Optimización de Rendimiento y Corrección de Escalado para Android TV Box (Mali-G31)

Fecha: 2026-10-11
Nivel: 2
Estado: verificada por agente

<!-- Estados: pendiente | aprobada | modificada | descartada |
     verificada por agente | revisada por humano | discrepada | cerrada -->

## Propuesta (la escribe el agente antes de editar)

### 1. Qué cambia

Resolución de los dos problemas críticos identificados en el dispositivo Android TV Box (4 núcleos Cortex-A53 @ 1.34 GHz, GPU Mali-G31 MP2, 2 GB RAM, resolución 720p $1280 \times 720$, según telemetría `blazing-perf-20.txt` y captura fotográfica):

1. **Corrección de Encuadre y Pantalla Completa en TV Box (720p / 1080p):**
   - Modificar la política de escalado de swapchain en [`src/present.rs`](../../src/present.rs): en pantallas apaisadas con dimensiones estándar de televisión ($w \le 1280$ y $h \le 720$, o paneles 720p nativos), fijar la escala de render en **1.0 (nativa)** en lugar de reducirla a `0.75` ($960 \times 540$).
   - Al renderizar a 1.0 en $1280 \times 720$, el marco del juego 3:2 ocupa $1080 \times 720$ px, llenando el **100% de la altura de la televisión** con letterbox simétrico de 100 px a cada lado, eliminando la ventana diminuta de $810 \times 540$ px (~47% del área) causada por la falta de autoescalado en el driver Vulkan de Mali-G31.
2. **Poda del Render Pass del Overlay Táctil en TV Box y Mandos:**
   - En [`src/platform.rs`](../../src/platform.rs), omitir por completo la codificación y ejecución del render pass de [`src/overlay.wgsl`](../../src/overlay.wgsl) cuando `touch_enabled == false` (dispositivos sin pantalla táctil o cuando se controle mediante gamepad/remoto).
   - Libera entre **6 ms y 12 ms** de cómputo en la GPU Mali-G31 MP2, erradicando la saturación de fillrate por cálculos procedurales de biseles 3D y distancias SDF en interfaces donde no hay pantalla táctil.
3. **Cobertura AOT C11 de los 11 Hotspots de CPU de TV Box:**
   - Incorporar 11 nuevas funciones canónicas en [`blazing.labels.toml`](../../blazing.labels.toml) para compilar nativamente con Clang `-O3` los bucles pesados capturados en `blazing-perf-20.txt`:
     - `0x08002390` (thumb): `Menu_CursorLoopStep`
     - `0x080A55B8` (thumb): `Sound_M4A_VoiceChannelStep`
     - `0x080882A4` (thumb): `Menu_IconInterpolateStep`
     - `0x0800A26C` (thumb): `Font_DecodeGlyphString`
     - `0x08025E1C` (thumb): `Battle_TargetScanUnits`
     - `0x08004794` (thumb): `Entity_LinkedListStep`
     - `0x08019808` (thumb): `Unit_InventoryLookup`
     - `0x08013464` (thumb): `Cursor_DistanceInterpolate`
     - `0x080134A8` (thumb): `Cursor_PathfindingStep`
     - `0x08066A08` (thumb): `Palette_ColorMatrixMultiply`
     - `0x080065B8` (thumb): `OAM_SpriteAffineCalc`
   - Regenerar el código C11 AOT en `src-c/`, reduciendo la carga del intérprete fallback en la CPU Cortex-A53 de 62,000 inst/s a prácticamente 0.

---

### 2. Contexto

- Contexto consultado: [`src/present.rs`](../../src/present.rs), [`src/present_tests.rs`](../../src/present_tests.rs), [`src/platform.rs`](../../src/platform.rs), [`blazing.labels.toml`](../../blazing.labels.toml), `blazing-perf-20.txt`, `IMG_20261011_014938.jpg`, [`AGENTS.md`](../../AGENTS.md).
- Intención: Lograr que el TV Box (hardware modesto de 4 núcleos Cortex-A53 y GPU Mali-G31) ejecute el juego ocupando toda la altura vertical de la TV y sosteniendo 60 FPS estables, sin perjudicar a tablets, smartphones ni emuladores.
- Restricciones y decisiones previas (máx. 3):
  - Proporción 3:2 canónica de GBA (origen: `AGENTS.md §2.3`): Mantener letterbox centrado sin deformar píxeles ni estirar a 16:9 artificialmente. → protegida por: `test_aspect_ratio_3_2`.
  - Prioridad 0-Interpreter (origen: `AGENTS.md §4.2`): Mapear hotspots a C11 nativo AOT en lugar de depender del loop de emulación. → protegida por: verificación de telemetría y build AOT.
  - Portabilidad y privacidad (origen: `AGENTS.md §6`): Cero rutas locales absolutas del usuario en el repositorio. → protegida por: pre-commit gatekeeper.
- Alternativas descartadas:
  - Estirar la imagen a 16:9 fullscreen deformando el juego: Descartada por violar la fidelidad geométrica original de GBA.
  - Eliminar los shaders de overlay en todos los dispositivos: Descartada porque teléfonos y tablets requieren el overlay táctil en modo portátil. La omisión debe ser condicional cuando no haya táctil o se use mando.

---

### 3. Ejemplos

- Caso Encuadre de TV Box ($1280 \times 720$):
  - Antes: `calcular_escala_swapchain(1280, 720)` devolvía `960 / 1280 = 0.75`. El swapchain se creaba a $960 \times 540$. Al no tener autoescalado de hardware en Vulkan con Mali-G31, el juego ocupaba $810 \times 540$ px en una ventana de $1280 \times 720$ (47% del área), dejando enormes franjas negras a la derecha y abajo.
  - Ahora: `calcular_escala_swapchain(1280, 720)` devuelve `1.0`. El swapchain se crea a $1280 \times 720$. El juego ocupa $1080 \times 720$ px, llenando el **100% de la altura de la televisión** con bandas simétricas de 100 px a los lados.
- Caso Fillrate de GPU en TV Box (Mali-G31):
  - Antes: `overlay_pipeline` ejecutaba el shader WGSL sobre toda la superficie en cada frame incluso sin pantalla táctil, consumiendo 14–24 ms en la GPU y provocando caídas a 25–40 FPS.
  - Ahora: Al detectar ausencia de pantalla táctil o uso de mando, el render pass del overlay se omite. El coste de GPU en presentación cae a <3 ms, liberando la GPU para sostener 60 FPS.
- Caso CPU Cortex-A53:
  - Antes: Menús, sonido M4A, fuentes y búsqueda de caminos ejecutaban hasta 62,357 instrucciones/segundo en el intérprete, elevando `ejec` a 31 ms.
  - Ahora: Las 11 funciones clave corren compiladas nativamente en C11 AOT con Clang `-O3`, reduciendo `ejec` a <10 ms.

---

### 4. Pasos

1. **[`src/present.rs`](../../src/present.rs):**
   - Ajustar `calcular_escala_swapchain`:
     - Si $w \le 1280$ y $h \le 720$ en orientación apaisada, la escala es `1.0`.
     - Si la resolución apaisada supera 1280 px (como tablets $2000 \times 1200$), mantener el tope de fillrate con `(1280.0 / max_dim).clamp(0.25, 1.0)`.
   - Actualizar [`src/present_tests.rs`](../../src/present_tests.rs) con aserciones para $1280 \times 720$, $1920 \times 1080$ y resoluciones ultra-anchas.
2. **[`src/platform.rs`](../../src/platform.rs):**
   - En `render_frame`, condicionar la ejecución del render pass de `overlay_pipeline` a `self.touch_enabled`.
   - Inicializar `touch_enabled` en falso si el dispositivo carece del feature `android.hardware.touchscreen` o al recibir eventos `KeyEvent` de gamepad.
3. **[`blazing.labels.toml`](../../blazing.labels.toml):**
   - Añadir las 11 funciones identificadas para TV Box.
   - Recompilar C11 con `recomp build "roms/1575...gba" --bios roms/gba_bios.bin --labels blazing.labels.toml --ram`.
   - Reemplazar archivos C en `src-c/`.
4. **Validación:**
   - Ejecutar `cargo test`.
   - Validar Diátaxis (`validar_docs.py`).
   - Extraer referencias (`extraer_referencias.py --check`).
   - Empaquetar APK oficial (`powershell -ExecutionPolicy Bypass -File scripts/package_apk.ps1`).

---

### 5. Qué más podría verse afectado

- [`src/present_tests.rs`](../../src/present_tests.rs): adaptaciones en los tests que validan los valores numéricos de `calcular_escala_swapchain(1280, 720)`.
- Archivos generados en `src-c/`: actualización de bloques C recompilados.

---

### 6. Suposiciones

- En televisores con resolución física 1080p ($1920 \times 1080$) o 720p ($1280 \times 720$), el aspect ratio físico de la pantalla es 16:9. Un juego GBA (3:2) en 720p debe medir exactamente $1080 \times 720$ px.
- Un dispositivo TV Box emite `KeyEvent` y reporta `hasSystemFeature("android.hardware.touchscreen") == false`.

---

### 7. Alcance

- Incluye:
  - Escala 1.0 nativa para resoluciones apaisadas $\le 1280 \times 720$.
  - Omisión del render pass de overlay cuando `touch_enabled == false`.
  - Mapeo de 11 funciones AOT en `blazing.labels.toml` y sincronización de `src-c/`.
- No incluye:
  - Cambios en el emulador de sonido APU de Rust ni en la PPU base de `gba-core`.
  - Opciones de estiramiento 16:9 con deformación de píxeles en el menú.

---

### 8. Criterios de aceptación (Dado/Cuando/Entonces)

- **Escalado 720p TV Box:**
  - *Dado* un panel de televisión apaisado de $1280 \times 720$ px.
  - *Cuando* se invoca `calcular_escala_swapchain(1280, 720)`.
  - *Entonces* devuelve `1.0` (swapchain $1280 \times 720$), y el marco del juego mide $1080 \times 720$ px ocupando el 100% de la altura útil.
- **Poda de Overlay:**
  - *Dado* un dispositivo TV Box con `touch_enabled == false`.
  - *Cuando* se ejecuta `render_frame`.
  - *Entonces* no se ejecuta ningún comando de dibujo de `overlay_pipeline` en el encoder de Vulkan.
- **Cobertura AOT:**
  - *Dado* el código C11 recompilado con las 11 nuevas etiquetas.
  - *Cuando* el juego ejecuta la subrutina de sonido M4A (`0x080A55B8`) o el bucle de menú (`0x08002390`).
  - *Entonces* se ejecutan a través de bloques nativos AOT sin caer en el intérprete de CPU.

---

### 9. Verificación previa y posterior

- **Comando de verificación de tests:**
  `cargo test`
- **Comando de empaquetado oficial de APK:**
  `powershell -ExecutionPolicy Bypass -File scripts/package_apk.ps1`
- **Comando de validación Diátaxis:**
  `python .agents/skills/diataxis-docs/scripts/validar_docs.py .`

---

## Verificación (la escribe el agente después de editar)

### Predicho vs. real

| Aspecto | Predicho | Real |
| :--- | :--- | :--- |
| **Escalado 720p Landscape** | `calcular_escala_swapchain(1280, 720) == 1.0` | `1.0` verificado por `test_calcular_escala_swapchain_dispositivos` |
| **Poda de Overlay en TV/Gamepad** | Omitir `overlay_pipeline` en render pass si `touch_enabled == false` | Implementado en `src/platform.rs` ante detección TV y en `on_key` |
| **Hotspots AOT en `blazing.labels.toml`** | 11 nuevas funciones agregadas (64 labels totales) | 64 labels compilados, generando 18,212 bloques nativos en C11 |
| **Suite de pruebas de Rust** | 100% de tests en verde | 111 tests pasados (100 unitarios + 11 integración) en 1.36s |
| **Validación Diátaxis** | 0 errores, 0 avisos | `0 error(es), 0 aviso(s)` en 10 páginas |
| **Fat APK multi-ABI** | APK firmado con soporte `arm64-v8a`, `armeabi-v7a`, `x86_64` | `target/blazing-recomp-debug.apk` generado (33.08 MB) con firmas v2 y v3 |

### Hallazgos fuera de alcance

- Ningún hallazgo bloqueante colateral. La detección JNI de TV Box contempla tanto `UI_MODE_TYPE_TELEVISION` como `hasSystemFeature("android.hardware.touchscreen") == false`.

