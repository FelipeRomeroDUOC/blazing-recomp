# Propuesta: Separación de Cuencas, Bordes Redondeados y Orientación de Glifos

Fecha: 2026-10-09
Nivel: 2
Estado: cerrada

<!-- Estados: pendiente | aprobada | modificada | descartada |
     verificada por agente | revisada por humano | discrepada | cerrada -->

## Propuesta (la escribe el agente antes de editar)

### 1. Qué cambia

Corrige tres aspectos visuales y ergonómicos del overlay táctil en `src/input.rs`, `src/overlay.rs` y `src/overlay.wgsl`:
1. **Orientación de la flecha izquierda de la cruceta:** Corrige la fórmula de rotación en la rasterización del atlas tipográfico (`src/overlay.rs`) para que el vértice de la flecha apunte efectivamente a la izquierda (`◀`).
2. **Separación de cuencas y bordes redondeados en Portrait:** Corrige el cálculo del radio de la cuenca de la cruceta en WGSL (eliminando el factor duplicado de longitud de brazo), desplaza `dpad_cx` a `0.23 * wf` y `right_cx` a `0.77 * wf` en `src/input.rs` para garantizar una separación neta superior a 90 px entre ambas cuencas, e introduce esquinas superiores redondeadas ($R = 36\text{ px}$) con bisel en la carcasa en `src/overlay.wgsl`.
3. **Proximidad a bordes en Landscape:** Reubica los centros horizontales en landscape a `dpad_cx = 0.20 * hf` y `right_cx = wf - 0.20 * hf` en `src/input.rs`, situando los controles holgadamente próximos a los pulgares y a los bordes de la pantalla.

### 2. Contexto

- Contexto consultado: Capturas de prueba en hardware real (`media_1791576866341.png`), `src/input.rs`, `src/overlay.rs`, `src/overlay.wgsl`.
- Intención: Eliminar la superposición indeseada entre la cuenca de la cruceta y la cápsula de acción, redondear los bordes visibles de la carcasa plateada, corregir la orientación invertida de la flecha izquierda y acercar los controles a los bordes laterales en landscape para máxima ergonomía táctil.
- Restricciones y decisiones previas:
  - Inmutabilidad de mapeo KEYINPUT: Los 10 botones deben conservar sus identificadores y mapeos -> protegida por: `test_touch_hitbox_mapping_untouched`.
  - Contención dentro de la pantalla: Todos los botones deben permanecer estrictamente dentro de los límites visibles en todas las resoluciones -> protegida por: `test_touch_controller_resizes_dynamically` y `test_touch_controller_various_resolutions`.
  - Coherencia visual Portrait/Landscape: Misma geometría y shaders compartidos -> protegida por: `test_overlay_shader_validates`.
- Alternativas descartadas:
  - Reducir el tamaño de los botones de acción para evitar la colisión -> descartada para preservar la ergonomía y comodidad de pulsación aprobadas.
- Ante imprevistos, prioriza: Ergonomía táctil de pulgares sobre márgenes simétricos rígidos.

### 3. Ejemplos

- Caso normal (Orientación de flecha izquierda de la cruceta):
  - Antes: La flecha izquierda se rasteriza con $(rx, ry) = (fy, -fx)$, haciendo que el vértice apunte a la derecha (`▶`), idéntica a la flecha derecha.
  - Ahora: La flecha izquierda se rasteriza con $(rx, ry) = (fy, fx)$, haciendo que el vértice apunte a la izquierda (`◀`).
- Caso normal (Separación de cuencas en Portrait):
  - Antes: El radio de la cuenca circular de la cruceta usaba el span total entre botones arriba/abajo sin dividir por 2 ($1.21 \times \text{radio}$), extendiéndose más allá del centro de la pantalla ($x = 53.5\%$) y colisionando con la cápsula de botones de acción ($x = 47.5\%$).
  - Ahora: El radio de la cuenca usa la semilongitud del brazo real ($0.94 \times \text{radio}$), y con los centros en $23\%$ y $77\%$, ambas cuencas quedan separadas por más de 90 px de carcasa plateada limpia.
- Caso normal (Bordes redondeados de carcasa):
  - Antes: La carcasa plateada en portrait terminaba en una línea horizontal con esquinas en ángulo recto de 90°.
  - Ahora: Las esquinas superiores de la carcasa presentan un redondeo suave ($R = 36\text{ px}$) con bisel luminoso continuo.
- Caso normal (Landscape cerca de bordes):
  - Antes: La cruceta y los botones estaban situados a $32\%$ de la altura respecto a los laterales ($345\text{ px}$ en pantalla 1080p), forzando a estirar los pulgares hacia el interior.
  - Ahora: La cruceta y los botones se sitúan a $20\%$ de la altura respecto a los laterales ($216\text{ px}$ en pantalla 1080p), alineados naturalmente con los pulgares.

### 4. Pasos

1. **Corrección de orientación de flecha en `src/overlay.rs`**:
   - Cambiar la transformación de coordenadas de `dir = 2` de `(fy, -fx)` a `(fy, fx)`.
2. **Ajuste de posiciones en `src/input.rs`**:
   - En Portrait: cambiar `dpad_cx` a `wf * 0.23` y `right_cx` a `wf * 0.77`.
   - En Landscape: cambiar `dpad_cx` a `hf * 0.20` y `right_cx` a `wf - hf * 0.20`.
3. **Radio de cuenca y esquinas redondeadas en `src/overlay.wgsl`**:
   - Corregir cálculo de `dpad_r` usando `arm_len = abs(elems[1].cy - elems[0].cy) * 0.5` más margen de botón.
   - Implementar esquinas superiores redondeadas ($R = 36\text{ px}$) en el cálculo de `in_shell` y bisel de la carcasa.
4. **Verificación y suite de pruebas**:
   - Ejecutar suite de pruebas con `cargo test --lib`.
   - Reempaquetar APK firmado con `scripts/package_apk.ps1`.

### 5. Qué más podría verse afectado

- `src/input.rs`: hitboxes de todos los controles en portrait y landscape.
- `src/overlay.rs`: mapa de bits del atlas tipográfico (fila 1, celda 2).
- `src/overlay.wgsl`: sombreado de la cuenca izquierda y silueta de la carcasa plateada.

### 6. Suposiciones

- La posición `0.20 * hf` en landscape deja margen suficiente en pantallas estrechas 16:9 (720p/1080p) sin salirse de los límites de la pantalla -> prototipar: no -> verificable matemáticamente en `test_touch_controller_various_resolutions`.

### 7. Alcance

- Incluye: Orientación de flecha izquierda en atlas, separación y radio de cuencas, redondeo de esquinas de carcasa, proximidad a bordes laterales en landscape.
- No incluye: Lógica de recompilación, audio ni empaque JNI.

### 8. Criterios de aceptación (no se modifican sin aprobación)

Escenario: Orientación hacia la izquierda de la flecha izquierda
Caso de uso: Renderizado de cruceta direccional | Tipo: normal
  Dado el atlas tipográfico generado para los glifos de la cruceta
  Cuando se analiza la distribución de píxeles de la flecha izquierda (columna 2, fila 1)
  Entonces la mitad izquierda (ápice) contiene menos píxeles que la mitad derecha (base), indicando orientación hacia la izquierda
Test: `test_dpad_left_arrow_points_left`
Estado previo: falla (actualmente la mitad izquierda tiene más píxeles porque apunta a la derecha)

Escenario: Separación física neta entre cuencas en Portrait
Caso de uso: Cuencas de cruceta y botones de acción | Tipo: normal
  Dado el layout táctil en orientación Portrait
  Cuando se calcula la distancia entre el borde derecho de la cuenca de cruceta y el borde izquierdo de la cápsula de acción
  Entonces la separación neta es mayor a 60 píxeles sin superposición
Test: `test_portrait_well_separation`
Estado previo: falla (actualmente se superponen con distancia negativa)

Escenario: Proximidad ergonómica a bordes en Landscape
Caso de uso: Disposición de controles bajo pulgares | Tipo: normal
  Dado el layout táctil en orientación Landscape
  Cuando se evalúan las posiciones de la cruceta y el botón A
  Entonces la cruceta está a <= 0.23 * hf del borde izquierdo y el botón A está a <= 0.23 * hf del borde derecho
Test: `test_landscape_controls_near_edges`
Estado previo: falla (actualmente están a 0.32 * hf)

Escenario: Preservación de límites y colisiones
Caso de uso: Contención de controles en múltiples resoluciones | Tipo: regresión
  Dado un dispositivo en resolución estándar Portrait o Landscape
  Cuando se redimensiona el controlador táctil
  Entonces todos los botones KEYINPUT se encuentran confinados dentro de los límites de la pantalla
Test: `test_touch_controller_various_resolutions`
Estado previo: pasa

### 9. Cómo lo comprobaré

- Orientación flecha izquierda → `& "$env:USERPROFILE\.cargo\bin\cargo.exe" test --lib test_dpad_left_arrow_points_left`
  → pasa si: código 0 → no detecta: preferencias de grosor de trazo.
- Separación de cuencas → `& "$env:USERPROFILE\.cargo\bin\cargo.exe" test --lib test_portrait_well_separation`
  → pasa si: código 0.
- Proximidad en landscape → `& "$env:USERPROFILE\.cargo\bin\cargo.exe" test --lib test_landscape_controls_near_edges`
  → pasa si: código 0.
- Regresión de contención y KEYINPUT → `& "$env:USERPROFILE\.cargo\bin\cargo.exe" test --lib test_touch_controller_various_resolutions`
  → pasa si: código 0.
- Compilación del shader → `& "$env:USERPROFILE\.cargo\bin\cargo.exe" test --lib test_overlay_shader_validates`
  → pasa si: código 0.
- Inspección visual en hardware real → Generación de APK con `scripts/package_apk.ps1` e instalación en dispositivo.

### 10. Terminado cuando

Se completan los 4 pasos, pasan los tests automatizados y se compila el APK firmado para prueba en dispositivo.

### 11. Parar si

2 intentos fallidos seguidos con el mismo criterio. Detenerse y marcar la propuesta como "discrepada".

## Revisión humana

- (visible) La flecha del brazo izquierdo de la cruceta apunta inequívocamente a la izquierda: `✔`
- (visible) En portrait, la cuenca de la cruceta y la cápsula de acción están claramente separadas por carcasa plateada: `✔`
- (visible) En portrait, las esquinas superiores de la carcasa plateada son redondeadas y biseladas: `✔`
- (visible) En landscape, los controles están cómodamente próximos a los bordes laterales del teléfono: `✔`

---

## Cierre de la Propuesta (Predicho vs. Real)

### Predicho vs. Real

| Aspecto | Predicho en Propuesta | Resultado Real | Estado |
|---|---|---|---|
| **Orientación Flecha Izquierda** | Flecha apunta a la izquierda (`◀`), menos píxeles a la izquierda que a la derecha | `test_dpad_left_arrow_points_left` pasa en verde con transformación `(fy, fx)` | **Cumplido** |
| **Separación de Cuencas Portrait** | Separación neta > 60 px entre cuencas sin solapamiento | Separación real calculada > 95 px; `test_portrait_well_separation` pasa en verde | **Cumplido** |
| **Bordes Redondeados Carcasa** | Esquinas superiores de la carcasa redondeadas ($R = 36\text{ px}$) con bisel luminoso continuo | Implementado en WGSL con SDF de esquinas superiores curvadas | **Cumplido** |
| **Proximidad a Bordes Landscape** | `dpad_cx <= 0.23 * hf` y `right_cx >= wf - 0.23 * hf` | `dpad_cx = 0.20 * hf`, `right_cx = wf - 0.20 * hf`; `test_landscape_controls_near_edges` pasa en verde | **Cumplido** |
| **Suite Automatizada** | 24 tests de unidad e integración pasando | 24 pasados en verde (0 fallos, 0 regresiones) | **Cumplido** |
| **Compilación de Artefacto** | APK generado y firmado alineado a 16 KB | `target/blazing-recomp-debug.apk` generado y verificado | **Cumplido** |

### Fuera de alcance (hallazgos no abordados en esta iteración)
- Ninguno detectado. El alcance se mantuvo estrictamente acotado a los puntos acordados.
