# Propuesta: Blindaje de Cobertura y Casos Límite en Funciones Críticas

Fecha: 2026-10-09
Nivel: 2
Estado: cerrada

<!-- Estados: pendiente | aprobada | modificada | descartada |
     verificada por agente | revisada por humano | discrepada | cerrada -->

## Propuesta (la escribe el agente antes de editar)

### 1. Qué cambia

Incorpora una suite de 8 nuevas pruebas unitarias y de robustez para cerrar los 5 puntos ciegos detectados en la auditoría de cobertura, sin modificar ninguna línea de código de producción:

1. **Blindaje de Entrada Táctil (`src/input_tests.rs`):**
   - `test_all_buttons_key_bits_distinct_and_standard_gba`: Validación exhaustiva de los 10 botones de GBA contra sus valores de registro estándar en `KEYINPUT` (A=0x1, B=0x2, Select=0x4, Start=0x8, Right=0x10, Left=0x20, Up=0x40, Down=0x80, R=0x100, L=0x200), verificando que la máscara combinada cubra exactamente `0x03FF` con cero colisiones entre bits.
   - `test_touch_controller_is_pressed_and_pressed_mask`: Validación del estado vivo de pulsación (`is_pressed`) y la máscara condensada (`pressed_mask`) bajo pulsaciones individuales, multitáctil compuesto (Up + B + A) y liberación limpia.
   - `test_touch_controller_size_and_default`: Validación de los getters públicos `size()` y la implementación del trait `Default`.
   - `test_touch_controller_unknown_pointer_release`: Validación de caso límite donde un evento `touch_up` con un `pointer_id` desconocido retorna `None` sin alterar ni corromper la lista de toques activos.

2. **Blindaje de Parámetros GPU y Glifos (`src/overlay_tests.rs`):**
   - `test_build_globals_and_pack_layout`: Validación de `build_globals` en Portrait (`opacity = 1.0`, `shell = 1.0`) y Landscape (`opacity = 0.65`, `shell = 0.0`), y comprobación de que `Globals::pack()` genera exactamente el array `[res_x, res_y, opacity, shell]` con alineación uniforme para el shader WGSL.
   - `test_all_buttons_map_to_expected_glyph`: Validación exhaustiva de que los 10 botones de GBA mapean sin excepción a sus glifos correspondientes (`ArrowUp`, `ArrowDown`, `ArrowLeft`, `ArrowRight`, `LetterB`, `LetterA`, `LetterL`, `LetterR`, `SelectLabel`, `StartLabel`).

3. **Blindaje de Robustez en Pantalla de Error (`src/error_screen.rs`):**
   - `test_error_screen_truncates_extremely_long_messages_safely`: Prueba con mensajes de error de más de 80 líneas para validar que el renderizador aplique wrapping, intercepte el desbordamiento vertical y dibuje el texto de corte `"... [mensaje truncado por espacio en pantalla] ..."` sin entrar en pánico ni escribir fuera del framebuffer.
   - `test_surface_fallback_all_srgb_formats`: Validación de `select_surface_format_and_alpha` cuando el adaptador solo ofrece formatos sRGB, verificando que seleccione el primer formato disponible sin fallar.

---

### 2. Contexto

- **Contexto consultado:** `src/input.rs`, `src/input_tests.rs`, `src/overlay.rs`, `src/overlay_tests.rs`, `src/error_screen.rs`, `.agents/skills/test-generation/SKILL.md`.
- **Intención:** Eliminar los puntos ciegos donde métodos clave utilizados por el pipeline gráfico del overlay y la lógica de entrada carecían de verificación automatizada.
- **Restricciones y decisiones previas:**
  - Cero modificaciones al código de producción (estricto a las directrices de `test-generation`).
  - No alterar las 35 pruebas existentes; solo agregar nuevas pruebas ortogonales.
  - Assertions concretas y deterministas sin dependencias externas ni números pseudoaleatorios.

---

### 3. Ejemplos (Dado / Cuando / Entonces)

- **Caso 1: Mapeo de bits de hardware GBA (`KEYINPUT`):**
  - **Dado:** La definición de los 10 botones de `Button`.
  - **Cuando:** Se evalúa `button.key_bit()` para cada botón.
  - **Entonces:** Cada valor coincide exactamente con el mapa oficial del hardware de GBA, ningún bit se repite y la disyunción lógica de todos los bits es exactamente `0x03FF`.

- **Caso 2: Detección de estado vivo para overlay (`is_pressed` / `pressed_mask`):**
  - **Dado:** Un `TouchController` en resolución 1080x2400.
  - **Cuando:** Se pulsan simultáneamente el botón `B` con puntero 1 y el botón `A` con puntero 2.
  - **Entonces:** `tc.is_pressed(Button::B)` y `tc.is_pressed(Button::A)` son `true`, `tc.is_pressed(Button::Up)` es `false`, y `tc.pressed_mask()` es exactamente `0x0001 | 0x0002 = 0x0003`.

- **Caso 3: Uniforms globales de GPU (`build_globals` / `Globals::pack`):**
  - **Dado:** Dimensiones de pantalla 1920x1080 en modo `Landscape`.
  - **Cuando:** Se invoca `build_globals(1920, 1080, Orientation::Landscape).pack()`.
  - **Entonces:** El vector resultante es exactamente `[1920.0, 1080.0, 0.65, 0.0]`.

- **Caso 4: Mensaje de error extralargo:**
  - **Dado:** Un mensaje de error con 100 líneas de volcado de registros y memoria.
  - **Cuando:** Se invoca `ErrorScreenRenderer::render` sobre un búfer de 400x300 px.
  - **Entonces:** La función finaliza con éxito sin pánico, y el búfer contiene el texto de advertencia de truncamiento.

---

### 4. Pasos

1. **Añadir 4 pruebas unitarias a `src/input_tests.rs`**:
   - `test_all_buttons_key_bits_distinct_and_standard_gba`
   - `test_touch_controller_is_pressed_and_pressed_mask`
   - `test_touch_controller_size_and_default`
   - `test_touch_controller_unknown_pointer_release`
2. **Añadir 2 pruebas unitarias a `src/overlay_tests.rs`**:
   - `test_build_globals_and_pack_layout`
   - `test_all_buttons_map_to_expected_glyph`
3. **Añadir 2 pruebas unitarias a `src/error_screen.rs`**:
   - `test_error_screen_truncates_extremely_long_messages_safely`
   - `test_surface_fallback_all_srgb_formats`
4. **Verificación y validación**:
   - Ejecutar `cargo test` esperando que la suite aumente de 35 a 43 tests en verde.
   - Validar sincronización con `python scripts/extraer_referencias.py --check`.
   - Validar documentación con `python .agents/skills/diataxis-docs/scripts/validar_docs.py .`.

---

### 5. Qué más podría verse afectado

- Ningún archivo de producción se modifica.
- La suite pasa de 35 a 43 pruebas en verde.
- No hay riesgo de regresión ni cambios de ABI.

---

### 6. Criterio de verificación

- `cargo test`: Pasan 43 tests (35 previos + 8 nuevos) con 0 fallos.
- Ninguna prueba oculta errores ni debilita aserciones existentes.

---

### 7. Comandos que vas a correr

```powershell
cargo test
python scripts/extraer_referencias.py --check
python .agents/skills/diataxis-docs/scripts/validar_docs.py .
```

---

## Aprobación (la da el usuario)

- [x] Aprobada explícitamente por el usuario en chat: "OK" (2026-10-09 23:07).

---

## Verificación (la llena el agente tras aplicar los cambios)

### 1. Comandos ejecutados y resultado
- `cargo test`: 43 tests ejecutados, **43 pasados en verde** (35 en `src/lib.rs` + 8 en `tests/packaging_tests.rs`).
- `python scripts/extraer_referencias.py --check`: Sincronización técnica exitosa (**código 0**).
- `python .agents/skills/diataxis-docs/scripts/validar_docs.py .`: 8 páginas validadas, **0 errores, 0 avisos**.

### 2. Predicho vs. real
- **Predicho:** La suite aumentaría de 35 a 43 pruebas en verde cubriendo los 5 puntos ciegos sin modificar código de producción.
- **Real:** Se agregaron exactamente 8 pruebas (4 en `src/input_tests.rs`, 2 en `src/overlay_tests.rs`, 2 en `src/error_screen.rs`); los 43 tests pasaron limpiamente. Quedaron blindados contra colisiones de bits, desalineación de uniforms de GPU, estado de pulsación para el compositor y desbordamiento de búfer.

### 3. Fuera de alcance (hallazgos que no se arreglaron)
- Ninguno detectado. El código de producción se mantuvo intacto.
