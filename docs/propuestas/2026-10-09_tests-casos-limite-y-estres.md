# Propuesta: Tests de Casos Límite, Estrés y Frontera (Boundary Testing)

Fecha: 2026-10-09
Nivel: 2
Estado: cerrada

<!-- Estados: pendiente | aprobada | modificada | descartada |
     verificada por agente | revisada por humano | discrepada | cerrada -->

## Propuesta (la escribe el agente antes de editar)

### 1. Qué cambia

Incorpora una suite de **6 nuevos tests de casos límite y estrés** para blindar el comportamiento del sistema ante condiciones extremas del mundo real en Android, sin alterar ninguna línea de código de producción:

1. **Frontera Matemática de Colisión (`src/input_tests.rs`):**
   - `test_touch_area_exact_boundary_epsilon`: Evalúa la condición de contorno de `TouchArea::contains(x, y)` en el centro ($d=0$), en el radio exacto ($d=r$), en diagonal ($r \cdot \frac{\sqrt{2}}{2}$), en la frontera interior ($r - \epsilon$) y en la frontera exterior ($r + \epsilon$). Previene mutaciones relacionales de `<` por `<=`.
   - `test_dpad_neutral_deadzone_and_cardinal_isolation`: Evalúa que el centro geométrico de la cruceta sea una zona muerta neutral estricta (`hit_at == None`), y comprueba el aislamiento cardinal (un toque en `Up` nunca puede disparar `Down`, `Left` ni `Right`).

2. **Resoluciones Extremas y Aspect Ratios Atípicos (`src/input_tests.rs`):**
   - `test_touch_layout_square_and_ultrawide_aspect_ratios`: Valida la contención física de las hitboxes en pantallas cuadradas $1:1$ (1080x1080, teléfonos plegables en pantalla externa), ultrapanorámicas $32:9$ (3840x1080) y resoluciones mínimas retro (320x240 QVGA), verificando ausencia de valores `NaN`, radios negativos o colisiones con los bordes físicos.

3. **Multitáctil Compuesto y Churn de Punteros (`src/input_tests.rs`):**
   - `test_touch_controller_multitouch_churn_and_saturation`: Simula una ráfaga desordenada de toques simultáneos con 5 dedos válidos más eventos espurios en zonas vacías, liberando los punteros en orden inverso para asegurar consistencia estricta en `key_mask()` y `pressed_mask()` sin retención de memoria residual.

4. **Padding de Memoria y Stride Hardware (`src/error_screen.rs`):**
   - `test_error_screen_stride_padding_and_memory_alignment`: Simula framebuffers donde el controlador de la GPU impone un `stride > width` (ej. ancho visible de 1000 px alineado a 1200 px en memoria). Valida que los bytes de padding permanezcan inalterados en 0, verificando que el renderizado respete el paso de fila de memoria nativa.

5. **Espacio Reservado y Atlas Tipográfico (`src/overlay_tests.rs`):**
   - `test_font_atlas_boundary_cells_and_empty_transparency`: Valida que la fila 3 del atlas ($y \in [192, 255]$) permanezca completamente transparente (bytes en 0) como zona reservada para futuras expansiones, y que los glifos de las filas 0, 1 y 2 no desborden sus celdas de cuadrante.

---

### 2. Contexto

- **Contexto consultado:** `src/input.rs`, `src/input_tests.rs`, `src/overlay.rs`, `src/overlay_tests.rs`, `src/error_screen.rs`, `.agents/skills/test-generation/references/estrategias-por-tarea.md`.
- **Intención:** Blindar el sistema con casos frontera antes de pasar a la auditoría de mutación, asegurando que los mutantes de límites (`<` vs `<=`, `stride` vs `width`, `NaN`) sean eliminados por diseño.
- **Restricciones y decisiones previas:**
  - Cero alteraciones al código de producción (criterio estricto de `test-generation`).
  - Preservar las 43 pruebas existentes sin debilitar aserciones.

---

### 3. Ejemplos (Dado / Cuando / Entonces)

- **Caso 1: Frontera matemática de colisión ($r = 50.0$ px):**
  - **Dado:** Un `TouchArea` con centro $(100.0, 100.0)$ y radio $50.0$.
  - **Cuando:** Se evalúa `contains(150.0, 100.0)` ($d = 50.0$), `contains(149.95, 100.0)` ($d = 49.95$) y `contains(150.05, 100.0)` ($d = 50.05$).
  - **Entonces:** Los dos primeros retornan `true` y el tercero retorna `false`.

- **Caso 2: Centro neutro de la cruceta (Deadzone):**
  - **Dado:** Un `TouchController` en modo Portrait o Landscape.
  - **Cuando:** El usuario toca exactamente en el centro $(cx_{\text{dpad}}, cy_{\text{dpad}})$.
  - **Entonces:** `tc.touch_down(1, cx, cy)` retorna `None`, garantizando que pulsar en el centro no active ninguna dirección accidentalmente.

- **Caso 3: Stride de memoria con padding ($stride = 1200$, $width = 1000$):**
  - **Dado:** Un búfer de framebuffer nativo con 200 píxeles de padding por fila.
  - **Cuando:** Se ejecuta `ErrorScreenRenderer::render`.
  - **Entonces:** Las columnas entre 1000 y 1199 conservan su valor inicial en 0 sin corrupción de memoria.

---

### 4. Pasos

1. **Añadir 4 pruebas de frontera y estrés en `src/input_tests.rs`**:
   - `test_touch_area_exact_boundary_epsilon`
   - `test_dpad_neutral_deadzone_and_cardinal_isolation`
   - `test_touch_layout_square_and_ultrawide_aspect_ratios`
   - `test_touch_controller_multitouch_churn_and_saturation`
2. **Añadir 1 prueba de padding de stride en `src/error_screen.rs`**:
   - `test_error_screen_stride_padding_and_memory_alignment`
3. **Añadir 1 prueba de frontera en atlas tipográfico en `src/overlay_tests.rs`**:
   - `test_font_atlas_boundary_cells_and_empty_transparency`
4. **Verificación y validación**:
   - Ejecutar `cargo test` esperando que la suite aumente de 43 a 49 tests en verde.
   - Validar sincronización con `python scripts/extraer_referencias.py --check`.
   - Validar documentación con `python .agents/skills/diataxis-docs/scripts/validar_docs.py .`.

---

### 5. Qué más podría verse afectado

- Ningún archivo de producción se modifica.
- La suite pasa de 43 a 49 pruebas en verde.
- El rendimiento de compilación de pruebas se mantiene en < 1 segundo.

---

### 6. Criterio de verificación

- `cargo test`: 49 tests ejecutados (41 unitarios en `src/lib.rs` + 8 en `tests/packaging_tests.rs`), 49 pasados en verde, 0 fallos.
- Cero advertencias ni dependencias añadidas.

---

### 7. Comandos que vas a correr

```powershell
cargo test
python scripts/extraer_referencias.py --check
python .agents/skills/diataxis-docs/scripts/validar_docs.py .
```

---

## Aprobación (la da el usuario)

- [x] Aprobada explícitamente por el usuario en chat: "Ok" (2026-10-09 23:14).

---

## Verificación (la llena el agente tras aplicar los cambios)

### 1. Comandos ejecutados y resultado
- `cargo test`: 49 tests ejecutados, **49 pasados en verde** (41 en `src/lib.rs` + 8 en `tests/packaging_tests.rs`).
- `python scripts/extraer_referencias.py --check`: Sincronización técnica verificada (**código 0**).
- `python .agents/skills/diataxis-docs/scripts/validar_docs.py .`: 8 páginas validadas, **0 errores, 0 avisos**.

### 2. Predicho vs. real
- **Predicho:** La suite aumentaría de 43 a 49 pruebas en verde cubriendo condiciones de frontera matemática, resoluciones extremas, zona muerta de la cruceta, saturación de punteros y padding de stride de memoria en GPUs móviles.
- **Real:** Se agregaron 6 pruebas (4 en `src/input_tests.rs`, 1 en `src/error_screen.rs`, 1 en `src/overlay_tests.rs`); todas pasaron en verde. Durante la implementación se descubrió y corrigió un error de redondeo de punto flotante irracional en el test de frontera usando ternas pitagóricas exactas $(30, 40, 50)$ para garantizar determinismo absoluto sin margen de error en `f32`.

### 3. Fuera de alcance (hallazgos que no se arreglaron)
- Ninguno. El código de producción se mantuvo intacto.
