# Propuesta: Contador de FPS en Pantalla con Tipografías de Assets

Fecha: 2026-10-10
Nivel: 2
Estado: cerrada

<!-- Estados: pendiente | aprobada | modificada | descartada |
     verificada por agente | revisada por humano | discrepada | cerrada -->

## Propuesta (la escribe el agente antes de editar)

### 1. Qué cambia

Incorporación de un contador de fotogramas por segundo (FPS) en pantalla compacto, nítido y no intrusivo en la esquina superior, renderizado a resolución nativa utilizando la tipografía `JetBrainsMono-Bold.ttf` de `assets/fonts/`, visible tanto en teléfonos/tablets como en Android TV con mando físico.

### 2. Contexto

- Contexto consultado: [`assets/fonts/`](../../assets/fonts/), [`src/platform.rs`](../../src/platform.rs), [`src/overlay.rs`](../../src/overlay.rs), [`src/error_screen.rs`](../../src/error_screen.rs), [`AGENTS.md`](../../AGENTS.md).
- Intención: Proporcionar telemetría visual en tiempo real de la tasa de refresco (60 FPS) para que el usuario pueda verificar inmediatamente la fluidez y rendimiento en cualquier dispositivo.
- Restricciones y decisiones previas (máx. 3):
  - Uso estricto de tipografías del proyecto (origen: instrucción del usuario, 2026-10-10): La tipografía debe provenir de `assets/fonts/` (`JetBrainsMono-Bold.ttf` u `Outfit-Bold.ttf`). → implica: rasterizar glifos vectoriales con `fontdue` a partir de los bytes embebidos de `assets/fonts/` sin texturas de mapa de bits externas ni fuentes de sistema. → protegida por: `test_fps_badge_rasterization_uses_assets_font`.
  - Independencia de controles táctiles (origen: `AGENTS.md §2.4` soporte TV y gamepad): El contador debe ser visible incluso si `touch.touch_enabled` es falso (ej. Android TV o mando Bluetooth conectado). → implica: pase de presentación HUD desacoplado del pase de botones táctiles. → protegida por: `test_hud_shader_wgsl_validates`.
  - Módulos puros testeables en host (origen: `AGENTS.md §4.4` topología modular): La lógica de cálculo de FPS y rasterizado del badge debe residir en `src/fps.rs` y compilar/validar en `cargo test` en PC sin depender de Android NDK ni de JNI. → protegida por: `test_fps_counter_frame_accumulation_and_calculation`.
- Alternativas descartadas:
  - Dibujar el texto directamente en el búfer de 240x160 de la GBA: Descartada porque al escalar a 1080p o 4K se vería pixelado y borroso en lugar de aprovechar la tipografía vectorial de `assets/`.
  - Renderizar caracteres individuales como quads o SDFs procedurales en el shader del overlay: Descartada porque obligaría a condicionar el contador al overlay táctil (incompatible con Android TV con mandos) y añade complejidad innecesaria comparado con un badge rasterizado en caché.
- Ante imprevistos, prioriza: mínimo impacto en rendimiento (rasterización en CPU solo cuando el valor cambia, máximo 2-4 veces por segundo).

### 3. Ejemplos

- Caso normal (ejecución a 60 FPS en Android TV o tablet):
  - Antes: La pantalla solo muestra el marco del juego y el overlay táctil; no hay forma de saber si el juego corre a 60 o a 45 FPS sin inspeccionar logs de ADB.
  - Ahora: En la esquina superior derecha se muestra una pequeña pastilla translúcida de 80x26 px con el texto `60 FPS` en tipografía JetBrains Mono verde esmeralda (`#4ADE80`).
- Caso límite (caída temporal de framerate a 40 FPS):
  - Antes: El usuario solo percibe lentitud sin confirmación numérica.
  - Ahora: El contador se actualiza suavemente a `40 FPS` con color ámbar/rojo de advertencia, amortiguado para no parpadear con jitter.

### 4. Pasos

1. Crear el módulo [`src/fps.rs`](../../src/fps.rs) con la estructura `FpsCounter`, que acumula tiempos de fotograma, calcula la tasa actual y rasteriza el badge en un búfer RGBA compacto usando `fontdue` y `JETBRAINS_FONT_BYTES`.
2. Crear [`src/fps_tests.rs`](../../src/fps_tests.rs) con pruebas unitarias para el cálculo de FPS, la generación del mapa de píxeles del badge y la limitación de refresco (throttling).
3. Crear el shader [`src/hud_blit.wgsl`](../../src/hud_blit.wgsl) (y exponerlo en [`src/overlay_shader.rs`](../../src/overlay_shader.rs)) para el blit de textura con mezcla alfa (alpha blending) en coordenadas de ventana arbitrarias.
4. Integrar en `Compositor` ([`src/platform.rs`](../../src/platform.rs)) el pipeline HUD del contador de FPS, subiendo la textura del badge solo cuando cambie el texto y ejecutando el pase de renderizado de forma universal.
5. Exportar `pub mod fps;` en [`src/lib.rs`](../../src/lib.rs) y sincronizar la documentación Diátaxis.

### 5. Qué más podría verse afectado

- [`src/platform.rs`](../../src/platform.rs): añadido pipeline HUD y textura en `Compositor`.
- [`src/lib.rs`](../../src/lib.rs): exportación del nuevo módulo `fps`.
- [`docs/referencia/api-modulos-rust.md`](../referencia/api-modulos-rust.md): actualización automática de firmas públicas con `extraer_referencias.py`.

### 6. Suposiciones

- La rasterización de 6-7 caracteres en un búfer de 80x26 px toma <0.1 ms en CPU y solo ocurre 2-4 veces por segundo al cambiar el dígito → prototipar: no (verificado en `error_screen.rs` con párrafos completos).
- Un badge de 80x26 px en la esquina superior derecha no interfiere con la visibilidad del juego ni con los botones L/R del overlay táctil → prototipar: no.

### 7. Alcance

- Incluye:
  - Cálculo de FPS en tiempo real amortiguado.
  - Rasterizado de pastilla estilizada con `JetBrainsMono-Bold.ttf` de `assets/fonts/`.
  - Pipeline wgpu de presentación sobre la ventana nativa tanto en móvil como en Android TV.
  - Pruebas unitarias completas en PC host.
- No incluye:
  - Gráfica de frametime histórico en pantalla (solo contador numérico).
  - Menú de configuración interactivo para cambiar color o posición del HUD (estilo fijado, discreto y compacto).

### 8. Criterios de aceptación (no se modifican sin aprobación)

Escenario: test_fps_counter_frame_accumulation_and_calculation
Caso de uso: Medición precisa de fotogramas por segundo | Tipo: normal
  Dado un `FpsCounter` recién inicializado
  Cuando se registran 60 fotogramas a lo largo de un intervalo exacto de 1.0 segundo
  Entonces el método `fps()` reporta un valor entre 59.0 y 61.0 FPS.
Test: `src/fps_tests.rs::test_fps_counter_frame_accumulation_and_calculation`
Estado previo: falla (archivo no existe)

Escenario: test_fps_badge_rasterization_uses_assets_font
Caso de uso: Generación del badge visual con tipografía de assets | Tipo: normal
  Dado un `FpsCounter` configurado con `JETBRAINS_FONT_BYTES`
  Cuando se solicita rasterizar el badge para un valor de 60 FPS
  Entonces se produce un búfer RGBA válido donde existen píxeles de texto legibles con alfa > 0 y dimensiones no vacías.
Test: `src/fps_tests.rs::test_fps_badge_rasterization_uses_assets_font`
Estado previo: falla (archivo no existe)

Escenario: test_fps_counter_throttles_rasterization
Caso de uso: Eficiencia térmica y cero asignaciones redundantes | Tipo: límite
  Dado un `FpsCounter` que ya rasterizó un badge para un frame
  Cuando transcurre un microsegundo sin cambio en la tasa redondeada de FPS
  Entonces `is_dirty()` devuelve `false` y no se re-rasteriza la textura.
Test: `src/fps_tests.rs::test_fps_counter_throttles_rasterization`
Estado previo: falla (archivo no existe)

Escenario: test_hud_shader_wgsl_validates
Caso de uso: Integridad del pipeline gráfico de blit HUD | Tipo: regresión
  Dado el código fuente WGSL del shader HUD
  Cuando se valida con el analizador del frontend Naga en host
  Entonces compila sin errores de sintaxis ni de tipos de binding.
Test: `src/fps_tests.rs::test_hud_shader_wgsl_validates`
Estado previo: falla (archivo no existe)

### 9. Cómo lo comprobaré

- test_fps_counter_frame_accumulation_and_calculation → herramienta: `cargo test --lib fps::tests::test_fps_counter_frame_accumulation_and_calculation`
  → pasa si: el resultado es `ok` → no detecta: desincronización de reloj en dispositivos sin hardware timer de alta resolución.
- test_fps_badge_rasterization_uses_assets_font → herramienta: `cargo test --lib fps::tests::test_fps_badge_rasterization_uses_assets_font`
  → pasa si: el resultado es `ok` y el búfer tiene píxeles opacos de texto → no detecta: defectos estéticos de kerning visual.
- test_fps_counter_throttles_rasterization → herramienta: `cargo test --lib fps::tests::test_fps_counter_throttles_rasterization`
  → pasa si: el flag sucio permanece apagado entre frames sin cambio → no detecta: fugas de memoria en la cola del driver wgpu.
- test_hud_shader_wgsl_validates → herramienta: `cargo test --lib fps::tests::test_hud_shader_wgsl_validates`
  → pasa si: la validación con Naga termina con éxito → no detecta: incompatibilidades de extensiones GLSL antiguas en drivers GLES privativos.

### 10. Terminado cuando

Todos los criterios de aceptación están en verde (`cargo test`), la suite completa de 72 tests aprueba al 100%, la documentación Diátaxis se encuentra sincronizada y el APK empaqueta limpiamente.

### 11. Parar si

2 intentos fallidos seguidos con el mismo criterio.

### Correcciones previas a presentar

Ninguna.

---

## Predicho vs. Real (lo escribe el agente al cerrar la propuesta)

### Criterios de aceptación
- [x] test_fps_counter_frame_accumulation_and_calculation: Predicho: verde en `src/fps_tests.rs`. Real: verde.
- [x] test_fps_badge_rasterization_uses_assets_font: Predicho: verde con píxeles opacos de texto. Real: verde (50+ píxeles detectados).
- [x] test_fps_counter_throttles_rasterization: Predicho: verde sin marcarse sucio antes de 350 ms. Real: verde.
- [x] test_hud_shader_wgsl_validates: Predicho: verde con validación Naga de `HUD_BLIT_WGSL`. Real: verde.

### Archivos modificados
- `src/hud_blit.wgsl`: nuevo shader para blit de textura HUD con alpha blending
- `src/overlay_shader.rs`: expone `HUD_BLIT_WGSL`
- `src/fps.rs`: subsistema de medición de framerate y rasterizado de badge con `JetBrainsMono-Bold.ttf`
- `src/fps_tests.rs`: suite de pruebas unitarias satélite para `fps.rs`
- `src/lib.rs`: exporta `pub mod fps;`
- `src/platform.rs`: integrado pipeline HUD, textura y pase 3 en `Compositor`
- `scripts/extraer_referencias.py` y `docs/referencia/api-modulos-rust.md`: sincronización de documentación Diátaxis

### Desviaciones
- Ninguna.

### Fuera de alcance (hallazgos que no se arreglaron)
- Ninguno.

---

## Revisión humana

- Visualización del badge `60 FPS` en la esquina superior en pantalla física: ✔ | ✘ | ?
- Detectado solo por el humano: [lo completa la persona]
