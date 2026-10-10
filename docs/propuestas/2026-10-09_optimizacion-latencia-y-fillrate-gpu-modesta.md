# Propuesta: optimizacion-latencia-y-fillrate-gpu-modesta

Fecha: 2026-10-09
Nivel: 2
Estado: revisada por humano

<!-- Estados: pendiente | aprobada | modificada | descartada |
     verificada por agente | revisada por humano | discrepada | cerrada -->

## Propuesta (la escribe el agente antes de editar)

### 1. Qué cambia

Optimización del tiempo de respuesta visual y latencia de animación de los controles táctiles en dispositivos con GPUs de gama de entrada (como Mali-G31 en Android TV y Adreno 610 en tablets):
1. **Poda espacial en el shader (`src/overlay.wgsl`):** AABB (*Axis-Aligned Bounding Box*) temprano para descartar de inmediato el cálculo procedural de SDFs, rotaciones trigonométricas, biseles 3D y muestreo tipográfico en los píxeles de pantalla fuera de los clústeres de botones (reduciendo hasta un ~80% de fragmentos en modo Landscape y >95% de evaluaciones de botones individuales).
2. **Muestreo seguro sin derivadas (`textureSampleLevel`):** Reemplazo de `textureSample` por `textureSampleLevel(..., 0.0)` en el atlas tipográfico para permitir ramificaciones divergentes sin penalización de rendimiento en GPUs móviles.
3. **Latencia del swapchain a 1 fotograma (`src/platform.rs`):** Reducción de `desired_maximum_frame_latency` de 2 a 1 en `wgpu::SurfaceConfiguration`, eliminando el encolado de búferes intermedios de presentación.
4. **Desacoplamiento de redibujado en eventos de entrada (`src/platform.rs`):** `on_motion()` mantiene la respuesta háptica instantánea (`vibrate(15)` a 0 ms) pero devuelve si hubo cambio de estado, permitiendo que `state.redraw()` se invoque una sola vez tras drenar el lote de eventos pendientes en `app.input_events_iter()`, evitando bloqueos síncronos redundantes con el swapchain.

### 2. Contexto

- Contexto consultado: `src/platform.rs`, `src/overlay.wgsl`, `src/overlay.rs`, `src/input.rs`, `AGENTS.md`.
- Intención: Eliminar el retraso perceptible entre la respuesta háptica (que es inmediata) y la animación visual de pulsación/liberación de los botones en dispositivos con GPU modesta (Mali-G31 y Adreno 610).
- Restricciones y decisiones previas (máx. 3):
  - Fidelidad y geometría del overlay invariante (origen: `docs/propuestas/2026-10-09_renderizado-overlay-tactil.md`) → implica: la poda por caja delimitadora debe contener con holgura suficiente (> 20 px) todos los radios, sombras, rotaciones y etiquetas de los botones sin cortar ningún borde visual. → protegida por: `Escenario: poda AABB no trunca la geometría visual de los controles`.
  - Portabilidad y privacidad estricta (origen: `AGENTS.md §6`) → implica: cero rutas locales absolutas en ningún archivo modificado o de test. → protegida por: `no testeable`.
  - Validación estricta del shader WGSL (origen: `src/overlay.rs`) → implica: el shader WGSL optimizado debe parsear y validar limpiamente con Naga. → protegida por: `Escenario: validación Naga de shader WGSL con poda AABB`.
- Alternativas descartadas:
  - Reducir la resolución de la superficie de la ventana completa a 720p o 540p → descartada porque degradaría la nitidez del texto, de las fuentes y del escalado del juego en tablets y televisores de alta resolución.
  - Eliminar los efectos visuales 3D o las sombras → descartada porque rompería la fidelidad estética tipo GBA SP lograda en las fases previas.
- Ante imprevistos, prioriza: máxima fluidez y respuesta inmediata en hardware modesto sin alterar la apariencia visual.

### 3. Ejemplos

- Caso normal (toque de botón en tablet con Adreno 610 a 2000x1200 en Landscape):
  - Entrada: El usuario pulsa el botón A.
  - Antes: `vibrate(15)` vibra de inmediato. La GPU ejecuta un fragment shader fullscreen evaluando 8 botones rotados, cuencas 3D y SDFs sobre los 2.4 millones de píxeles; además el swapchain encola 2 fotogramas (`desired_maximum_frame_latency: 2`). La animación de pulsación tarda 100–200 ms en reflejarse en pantalla.
  - Ahora: `vibrate(15)` vibra de inmediato. En el ~80% de la pantalla fuera de los clústeres de botones, el shader retorna de inmediato en 2 instrucciones; para los píxeles del clúster A/B, solo se evalúan los botones cercanos; y el swapchain presenta a latencia 1. La animación se visualiza en el siguiente ciclo VSync (~16 ms).
- Caso límite (pulsación rápida y ráfaga de eventos de puntero en Android TV):
  - Entrada: Llegan en una misma ráfaga de eventos `Down` y micro-movimiento `Move`.
  - Antes: Cada evento en el bucle invocaba `self.redraw()` síncrono, bloqueando el hilo de eventos en `frame.present()`.
  - Ahora: Los eventos se drenan actualizando el estado y disparando háptica sin bloqueos, y `state.redraw()` se ejecuta de forma consolidada una sola vez al terminar el lote.

### 4. Pasos

1. Optimizar `src/overlay.wgsl`:
   - Sustituir `textureSample` por `textureSampleLevel(font_texture, font_sampler, uv, 0.0)` en `glyph_color`.
   - Incorporar descarte global temprano en modo Landscape cuando el fragmento cae fuera de las zonas envolventes de la cruceta, clúster de acción y Select/Start.
   - En modo Portrait, incorporar descarte temprano para píxeles situados en la mitad superior (`p.y < shell_top - 12.0`).
   - Añadir comprobaciones AABB por botón individual en el bucle `elems[4..12]` y en la cruceta `dpad_c` para omitir cálculo de SDFs, sombras y rotaciones para píxeles lejanos.
2. Actualizar `src/platform.rs`:
   - Cambiar `desired_maximum_frame_latency: 2` a `1` en `Compositor::new`.
   - Modificar `on_motion(&mut self, ...)` para que retorne `bool` indicando si hubo cambio de estado en lugar de llamar síncronamente a `self.redraw()`.
   - En el bucle de eventos de `android_main`, acumular si hubo cambios en el lote de `input_events_iter()` y llamar a `state.redraw()` al finalizar el bucle interno si `needs_redraw == true`.
3. Crear tests en `src/overlay.rs`:
   - `test_overlay_bounding_boxes_cover_visual_elements`: valida que los márgenes de poda del shader cubren holgadamente las hitboxes y radios visuales.
   - Re-ejecutar `test_overlay_shader_validates` para confirmar que Naga valida el WGSL optimizado.
4. Compilar con `cargo test` y verificar que los 32+ tests pasen en verde.
5. Recompilar el APK con `scripts/package_apk.ps1` para prueba en dispositivos.

### 5. Qué más podría verse afectado

- `src/overlay.wgsl` (lógica interna del fragment shader).
- `src/platform.rs` (configuración del swapchain y despacho de eventos de entrada).
- `src/overlay.rs` (suite de tests de validación del shader).

### 6. Suposiciones

- `desired_maximum_frame_latency: 1` es soportado por los adaptadores Vulkan y OpenGL ES en Android. → prototipar: no
  → si es falso: wgpu degrada automáticamente a la latencia mínima soportada por la superficie.
- `textureSampleLevel(..., 0.0)` es compatible con la especificación WGSL y no requiere extensiones opcionales. → prototipar: no
  → si es falso: se mantiene `textureSample` evaluado en bloque no divergente.

### 7. Alcance

- Incluye:
  - Poda espacial AABB en `overlay.wgsl`.
  - Mip-level 0 explícito (`textureSampleLevel`) en muestreo tipográfico.
  - Latencia de swapchain reducida a 1 fotograma en `platform.rs`.
  - Desacoplamiento de `redraw()` por lote de eventos en `android_main`.
  - Tests unitarios y de validación Naga.
- No incluye:
  - Cambios en el modelo de hardware o emulación GBA (`gba-core`).
  - Cambios en las hitboxes o geometría lógica de `TouchController` en `src/input.rs`.

### 8. Criterios de aceptación (no se modifican sin aprobación)

Escenario: validación Naga de shader WGSL con poda AABB
Caso de uso: compilación y verificación de shaders WGSL | Tipo: normal
  Dado el archivo `src/overlay.wgsl` con poda espacial AABB y `textureSampleLevel`
  Cuando se parsea y valida con el validador estricto de Naga
  Entonces valida sin errores de tipos, sintaxis ni violaciones de control de flujo.
Test: `src/overlay.rs::test_overlay_shader_validates`
Estado previo: pasa (test_overlay_shader_validates en verde)

Escenario: poda AABB no trunca la geometría visual de los controles
Caso de uso: cálculo de holguras de recorte de controles | Tipo: normal
  Dado el conjunto de botones de `build_elems` en resoluciones típicas (1080p y 1200p)
  Cuando se comprueba que el margen de la caja delimitadora supera el radio visual más sombras y etiquetas
  Entonces la holgura es de al menos 16 píxeles en todos los botones.
Test: `src/overlay.rs::test_overlay_bounding_boxes_cover_visual_elements`
Estado previo: pasa (test_overlay_bounding_boxes_cover_visual_elements en verde)

Escenario: latencia de presentación de fotogramas configurada a 1
Caso de uso: configuración de swapchain de baja latencia | Tipo: límite
  Dado el código fuente de `src/platform.rs`
  Cuando se inspecciona la configuración de `wgpu::SurfaceConfiguration`
  Entonces `desired_maximum_frame_latency` es exactamente 1.
Test: `src/overlay.rs::test_compositor_frame_latency_is_one`
Estado previo: pasa (test_compositor_frame_latency_is_one en verde)

Escenario: regresión en suite de tests existente
Caso de uso: ejecución de la suite de pruebas del proyecto | Tipo: regresión
  Dado el proyecto con los 32 tests actuales
  Cuando se ejecuta `cargo test`
  Entonces los 32 tests previos pasan en verde junto a los nuevos tests.
Test: suite completa de tests de Rust
Estado previo: pasa (34 tests en verde, 0 fallos)

### 9. Cómo lo comprobaré

- Comando de verificación: `cargo test`
- Resultado esperado: 34 tests pasando en verde, 0 fallos.
- No detecta: la percepción táctil subjetiva en la mano del usuario (requiere prueba física en los dos dispositivos).

### 10. Correcciones previas

- Ninguna.

### 11. Predicho vs. Real

- **Poda espacial en el shader:** Predicho: descarte temprano de fragmentos fuera de los controles y uso de `textureSampleLevel(..., 0.0)`. Real: `fs_main` descarta en ~2 instrucciones los píxeles fuera de las áreas de control; el validador estricto de Naga valida el WGSL al 100% sin advertencias.
- **Latencia de swapchain y consolidación:** Predicho: `desired_maximum_frame_latency: 1` y redibujado consolidado por lote. Real: implementado en `src/platform.rs`; el bucle de eventos drena el lote antes de presentar fotograma.
- **Respuesta háptica y compatibilidad:** Predicho: vibración táctil inmediata. Hallazgo colateral y ajuste: los motores ERM en tablets requerían 35 ms para vencer la inercia mecánica, y Android 11 requería la API `VibrationEffect.createOneShot(ms, -1)` en lugar del método legado; implementado con fallback y validado físicamente por el usuario.
- **Suite de pruebas:** Predicho: 34 tests en verde. Real: 34 tests unitarios y de integración pasando en verde (0 fallos).
- **Empaquetado APK:** Predicho: APK listo para prueba. Real: `target/blazing-recomp-debug.apk` generado con éxito (39.2 MB) con los binarios nativos optimizados para `arm64-v8a`, `armeabi-v7a` y `x86_64`.

## Revisión humana (la llena la persona después de verificar)

- [x] Escenario: validación Naga de shader WGSL con poda AABB: ✔
- [x] Escenario: poda AABB no trunca la geometría visual de los controles: ✔
- [x] Escenario: latencia de presentación de fotogramas configurada a 1: ✔
- [x] Escenario: regresión en suite de tests existente: ✔

Nota de la persona:
Vibración física y latencia visual validadas correctamente en tablet y dispositivos objetivo.

