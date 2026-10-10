# Propuesta: Optimización de Rendimiento GPU (Vulkan Primero), Pacing de Entrada, Telemetría de Renderer y Calibración de FPS

Fecha: 2026-10-10
Nivel: 2
Estado: aprobada

<!-- Estados: pendiente | aprobada | modificada | descartada |
     verificada por agente | revisada por humano | discrepada | cerrada -->

## Propuesta (la escribe el agente antes de editar)

### 1. Qué cambia

Resolución exhaustiva de los cuellos de botella de rendimiento y usabilidad reportados en la segunda ronda de pruebas:

1. **Móvil (Snapdragon 7 Gen 3) y Telemetría de Renderer:**
   - Agrandar el badge de FPS a `208x34` px y tipografía vectorial a `16.0` px para incluir el renderer gráfico activo de la GPU (`60 FPS · AOT · Vulkan` o `60 FPS · AOT · OpenGL`), con legibilidad óptima en pantallas de alta densidad.
   - Reubicar el badge con mayor holgura hacia la izquierda (`margin_x = 96.0` en horizontal, `36.0` en vertical; `margin_y = 24.0` / `44.0`), garantizando que ningún dato quede cortado ni oculto tras los bordes redondeados o recortes de cámara (*punch-hole*).

2. **BlueStacks (Android 11):**
   - **Calibración de medición de FPS:** Vincular la acumulación de fotogramas del contador a los fotogramas de simulación GBA efectivamente completados (`runner.step_frame()`), en lugar de los pases de presentación del swapchain host. Así, cuando la GBA emula a 60 FPS, el contador reportará fielmente `60 FPS · AOT`, aun si el compositor del emulador refresca a 30 Hz.
   - **Erradicación del doble input:** Descartar eventos de autorepeat (`event.repeat_count() > 0`) en `KeyEvent` y aplicar arbitraje de entrada para evitar que eventos táctiles inyectados por el mapeador de BlueStacks se solapen con las teclas físicas, eliminando las pulsaciones fantasma repetidas.

3. **Tablet (11-17 FPS) y Android TV (15-30 FPS):**
   - **Prioridad de GPU Vulkan:** Invertir el orden de inicialización en [`src/platform.rs`](../../src/platform.rs) para intentar **Vulkan primero** (`Backends::VULKAN`) y reservar **OpenGL ES** (`Backends::GL`) como fallback. En Android 10+ (incluyendo Android 11), Vulkan elimina el severo overhead de CPU y sincronización de drivers OpenGL ES en GPUs ARM Mali y PowerVR.
   - **Supresión de overlay táctil en Android TV:** Desactivar automáticamente el pase de sombreado del overlay táctil (`touch_enabled = false`) cuando se ejecute en Android TV o se opere exclusivamente con mandos físicos, ahorrando el 100% del fillrate del overlay en pantallas 1080p/4K.
   - **Poda geométrica estricta de fragmentos en [`src/overlay.wgsl`](../../src/overlay.wgsl):** En portrait, las zonas de la carcasa que no contienen botones ni cuencas se sombrean con cálculo analítico plano y se podan las evaluaciones de SDFs 3D, aliviando la carga en paneles 2K/WQXGA.
   - **Optimización de compilación C11 AOT con Clang (`-O2`):** En [`scripts/package_apk.ps1`](../../scripts/package_apk.ps1), compilar `libblazing_recompiled.so` con `-O2 -fPIC -fomit-frame-pointer`, multiplicando la velocidad de ejecución de los 17,207 bloques en núcleos Cortex-A53 y Cortex-A55.

---

### 2. Contexto

- Contexto consultado: [`src/platform.rs`](../../src/platform.rs), [`src/fps.rs`](../../src/fps.rs), [`src/fps_tests.rs`](../../src/fps_tests.rs), [`src/overlay.wgsl`](../../src/overlay.wgsl), [`src/input.rs`](../../src/input.rs), [`scripts/package_apk.ps1`](../../scripts/package_apk.ps1), [`AGENTS.md`](../../AGENTS.md).
- Intención: Llevar el rendimiento de la emulación nativa a 60 FPS fluidos en tabletas y televisores sin comprometer la compatibilidad de emuladores ni la estética visual de los controles.
- Restricciones y decisiones previas (máx. 3):
  - Prioridad de Vulkan con fallback a GLES (origen: `AGENTS.md §2.3` Renderizado & Pantalla): Vulkan debe ser el backend preferente en Android moderno, manteniendo OpenGL ES como red de seguridad para dispositivos o emuladores antiguos.
  - Telemetría FPS basada en el reloj del juego GBA (origen: especificación de emuladores nativos): El badge debe reflejar la velocidad real a la que avanza la lógica del juego (60 FPS = 100% velocidad GBA).
  - Cero regresiones en la suite de pruebas y Diátaxis (origen: `AGENTS.md §6` Checklist Obligatorio).
- Alternativas descartadas:
  - Reducir la resolución interna del swapchain a 720p en tablets: Descartada para preservar la nitidez del texto y los gráficos en pantalla completa; la priorización de Vulkan y poda de fragmentos resuelve el cuello de botella sin degradar la resolución de pantalla.
  - Eliminar los botones táctiles en portrait en tabletas: Descartada porque el usuario debe poder jugar con la pantalla táctil si así lo desea.

---

### 3. Ejemplos

- Caso Móvil (legibilidad y posición del badge):
  - Antes: Badge de 112x26 px con texto de 13 px situado a 48 px del margen derecho; en pantallas de 2400x1080 o curvadas se lee diminuto y queda muy próximo a la curvatura.
  - Ahora: Badge de 144x34 px con tipografía de 18 px a 96 px del margen derecho en landscape; texto nítido, claro y cómodamente legible.
- Caso BlueStacks (FPS y control):
  - Antes: Aunque el juego corría a 60 FPS de emulación real, el contador marcaba "30 FPS · AOT" porque contaba presentaciones de ventana. Al pulsar la cruceta, la colisión de eventos táctiles simulados con eventos de teclado causaba doble avance de casilla.
  - Ahora: El contador mide los frames de emulación completados y marca con exactitud "60 FPS · AOT". Los eventos repetidos de teclado y el conflicto táctil se arbitran, avanzando exactamente 1 casilla por pulsación.
- Caso Tablet y Android TV (fluidez y GPU):
  - Antes: La app inicializaba con OpenGL ES, sufriendo caídas severas de fillrate y sincronización en drivers Mali (11 FPS portrait, 17 FPS landscape en tablet; 15-30 FPS en Android TV). En Android TV, la GPU dibujaba el overlay táctil completo sobre 1080p/4K a pesar de no haber pantalla táctil.
  - Ahora: Vulkan se selecciona como motor principal nativo, el overlay se desactiva en Android TV, los bloques C11 se compilan optimizados con `-O2`, y el sombreador en tablet descarta el exceso de fragmentos, recuperando la fluidez del juego.

---

### 4. Pasos

1. **[`src/fps.rs`](../../src/fps.rs) y [`src/fps_tests.rs`](../../src/fps_tests.rs):**
   - Actualizar dimensiones a `BADGE_WIDTH = 144`, `BADGE_HEIGHT = 34`.
   - Ajustar el tamaño de la tipografía rasterizada a `18.0f32` con radio de esquina `8.0f32`.
   - Añadir método `on_frames(count: u32)` (o adaptar `on_frame`) para registrar el número exacto de frames GBA avanzados.
   - Actualizar tests unitarios satélite en `src/fps_tests.rs`.

2. **[`src/platform.rs`](../../src/platform.rs):**
   - Invertir el orden de `create_gpu_context`: crear instancia y superficie con `wgpu::Backends::VULKAN` primero; si falla, recurrir a `wgpu::Backends::GL`.
   - En Android TV (detectado o cuando se usan mandos físicos exclusivos), permitir desactivar el overlay táctil (`touch.touch_enabled = false`).
   - En `android_main`, llamar a `fps_counter.on_frame()` dentro del bucle de avance de frames GBA (`state.step()`), garantizando que la métrica mida fotogramas de emulación reales.
   - En `on_key`, ignorar eventos repetidos (`event.repeat_count() > 0`) y arbitrar `combined_keys()` para que las pulsaciones físicas prevalezcan limpiamente sobre eventos táctiles simultáneos.
   - Actualizar márgenes del HUD: `margin_x = if w > h { 96.0f32 } else { 36.0f32 }`, `margin_y = if w > h { 24.0f32 } else { 44.0f32 }`.

3. **[`src/overlay.wgsl`](../../src/overlay.wgsl):**
   - Refinar el descarte por AABB en la carcasa en portrait y elementos táctiles para minimizar instrucciones por fragmento fuera de las áreas activas.

4. **[`scripts/package_apk.ps1`](../../scripts/package_apk.ps1):**
   - Modificar las banderas de Clang en `Build-Recompiled-C` de `-O1` a `-O2 -fPIC -fomit-frame-pointer` para todos los ABIs.

5. **Documentación y Verificación:**
   - Sincronizar referencias de API con `python scripts/extraer_referencias.py`.
   - Ejecutar suite de pruebas con `cargo test` (100% verde).
   - Validar Diátaxis con `python .agents/skills/diataxis-docs/scripts/validar_docs.py .`.
   - Reempaquetar el Fat APK oficial con `powershell -ExecutionPolicy Bypass -File scripts/package_apk.ps1`.

---

### 5. Qué más podría verse afectado

- [`src/fps_tests.rs`](../../src/fps_tests.rs): actualización de las aserciones de dimensiones del badge (144x34).
- [`docs/referencia/api-modulos-rust.md`](../referencia/api-modulos-rust.md): actualización de constantes o firmas públicas si cambiaran.
- Tiempos de empaquetado del script APK: la compilación inicial con `-O2` tardará unos segundos adicionales, pero se preserva el sistema de caché por timestamp de archivos objeto (`.o`).

---

### 6. Suposiciones

- La GPU de la tablet Android 11 dispone de soporte Vulkan 1.0/1.1 funcional (estándar obligatorio desde Android 10 para certificación GMS).
- Las pantallas de smartphones Android modernos en landscape toleran holguras de hasta 96 px sin colisionar con el botón R táctil (verificado: R se ubica a más de 120 px).

---

### 7. Alcance

- Incluye:
  - Agrandamiento y reposicionamiento del badge de FPS.
  - Medición de FPS vinculada a la emulación GBA.
  - Filtro contra doble input y autorepeat de teclas.
  - Prioridad de Vulkan sobre OpenGL ES.
  - Desactivación de overlay en Android TV / mandos puros.
  - Poda de fragmentos en WGSL.
  - Optimización `-O2` en Clang para `libblazing_recompiled.so`.
- No incluye:
  - Menú de configuración interactivo dentro del juego para alternar opciones gráficas en tiempo de ejecución.
  - Escalado bilineal fraccional de la resolución nativa de GBA (se mantiene 240x160 con escalado entero/proporcional nítido).

---

### 8. Verificación

1. **Tests unitarios:** `cargo test` al 100% en verde.
2. **Validación Diátaxis:** `validar_docs.py` con 0 errores y 0 avisos.
3. **Referencias de API:** `extraer_referencias.py --check` exitoso.
4. **Empaquetado oficial:** `powershell -ExecutionPolicy Bypass -File scripts/package_apk.ps1` generando el Fat APK sin fallos.

---

### 9. Predicho vs. Real

| Aspecto | Predicho | Real | Desviación / Observaciones |
| :--- | :--- | :--- | :--- |
| **Badge y Telemetría de Renderer** | `208x34` px, `{fps_val} FPS · {mode} · {renderer}` | Implementado `216x34` px con tipografía de 15.5 px y margen horizontal de 96 px en landscape | Se ajustó el ancho a 216 px para albergar con holgura los 21 caracteres de `"60 FPS · AOT · Vulkan"` u `"OpenGL"`. |
| **Prioridad GPU Vulkan** | Vulkan primero, OpenGL ES fallback | Implementado en `create_gpu_context`; el backend activo se transmite al badge de FPS | Los dispositivos con Vulkan nativo acceden a sincronización por hardware sin overhead de GLES. |
| **Pacing de FPS en GBA** | Contador acumulando frames de emulación | `on_sim_frames(steps)` invocado en cada ciclo de avance de `GameRunner` | En emuladores o pantallas de 30 Hz, el badge reporta la tasa real de avance del juego (60 FPS). |
| **Filtro de Doble Input** | Filtrar `repeat_count > 0` y arbitrar `combined_keys` | Implementado filtro de autorepeat y enmascaramiento de toques simulados cuando hay tecla física | Previene colisiones de eventos de teclado con toques sobre la pantalla en BlueStacks. |
| **Supresión de Overlay en TV** | `touch_enabled = false` en Android TV | Implementada detección JNI con `UI_MODE_TYPE_TELEVISION` | Ahorra el 100% de la carga de sombreado del overlay en televisores 1080p y 4K. |
| **Optimización Clang `-O2`** | Compilación `-O2 -fPIC -fomit-frame-pointer` | Todos los ABIs recompilados con `-O2` en `libblazing_recompiled.so` (31.68 MB APK) | Ejecución óptima en núcleos ARM Cortex-A53 / A55. |
| **Suite de Pruebas** | 100% verde | 77 tests aprobados (67 unitarios en `src/lib.rs` + 10 en `tests/packaging_tests.rs`) | Incorporados tests para telemetría de renderer y acumulación por lotes. |

---

### 10. Fuera de alcance (hallazgos no modificados)

- *Ningún defecto colateral ni refactor secundario detectado.* La intervención se limitó estrictamente a las solicitudes de rendimiento, telemetría y corrección de entrada reportadas por el usuario.

