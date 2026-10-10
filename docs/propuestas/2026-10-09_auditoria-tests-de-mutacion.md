# Propuesta: Auditoría de Tests de Mutación (Mutation Testing) y Blindaje de Lógica Pura

Fecha: 2026-10-09
Nivel: 2
Estado: cerrada

<!-- Estados: pendiente | aprobada | modificada | descartada |
     verificada por agente | revisada por humano | discrepada | cerrada -->

## Propuesta (la escribe el agente antes de editar)

### 1. Qué cambia

Ejecución y formalización de una auditoría sistemática de **Tests de Mutación** orientada a evaluar la capacidad real de detección de fallos de la suite de pruebas (49 tests unitarios y de integración) sobre los subsistemas de lógica pura:
- [`src/input.rs`](../../src/input.rs) y [`src/input_tests.rs`](../../src/input_tests.rs): Geometría, colisiones, bitmasks de hardware GBA, control multitáctil y normalización de aspect ratios.
- [`src/overlay.rs`](../../src/overlay.rs) y [`src/overlay_tests.rs`](../../src/overlay_tests.rs): Parámetros de shader, empaquetado uniforme de memoria, paletas visuales, simetría geométrica y atlas tipográfico.
- [`src/error_screen.rs`](../../src/error_screen.rs): Negociación segura de superficies gráficas, formateo de framebuffer con padding/stride y renderizado de texto por software.

Para ello se implementó el arnés de evaluación automatizado [`scripts/auditar_mutantes.py`](../../scripts/auditar_mutantes.py), aplicando mutaciones sintéticas intencionales (inversiones lógicas, operadores relacionales, constantes numéricas de bitmask, desplazamiento de buffers de memoria y fallbacks de error) sin modificar el código de producción.

---

### 2. Contexto

- **Contexto consultado:** `.agents/skills/test-generation/references/mutacion.md`, `.agents/skills/test-generation/SKILL.md`, `src/input.rs`, `src/overlay.rs`, `src/error_screen.rs`.
- **Intención:** Superar la métrica superficial de cobertura de líneas de código (*Line Coverage*) auditando si las aserciones de la suite de tests son lo suficientemente estrictas para detectar fallos semánticos sutiles en tiempo de compilación o ejecución.
- **Restricciones y decisiones previas:**
  - Cero modificaciones a código de producción para "forzar" la muerte de mutantes (regla estricta de `test-generation`).
  - Restauración segura de archivos en disco desde memoria RAM sin invocar `git checkout` para prevenir pérdida accidental de archivos no versionados.
  - No usar rutas absolutas en scripts ni documentación (`AGENTS.md`).

---

### 3. Mutantes Diseñados y Clasificación

Se definieron 16 mutantes dirigidos representativos de fallos sutiles en software crítico:

| ID | Archivo | Subsistema | Mutación Inyectada | Tipo de Mutante |
|---|---|---|---|---|
| **M01** | `src/input.rs` | `TouchArea::contains` | `<= radius * radius` $\to$ `< radius * radius` | Relacional / Límite |
| **M02** | `src/input.rs` | `TouchController::touch_down` | `if !self.touch_enabled` $\to$ `if self.touch_enabled` | Inversión Lógica |
| **M03** | `src/input.rs` | `TouchController::touch_down` | Descarte de puntero repetido $\to$ `if false` | Condición Ignorada |
| **M04** | `src/input.rs` | `TouchController::key_mask` | `!pressed_bits & 0x03FF` $\to$ `... & 0x01FF` | Constante / Bitmask GBA |
| **M05** | `src/input.rs` | `TouchController::is_pressed` | `.any(\|(_, b)\| *b == button)` $\to$ `.all(...)` | Operador de Colección |
| **M06** | `src/input.rs` | `Button::key_bit` | `Button::B => 0x0002` $\to$ `0x0001` (colisión con A) | Constante de Registro |
| **M07** | `src/input.rs` | `layout` (Cruceta) | `arm = dpad_radius * 0.55` $\to$ `0.15` (invade deadzone) | Geometría / Tolerancia |
| **M08** | `src/input.rs` | `layout` (Clúster B/A) | `tilt = 16.0f32.to_radians()` $\to$ `0.0` | Inclinación Ergonomía |
| **M09** | `src/overlay.rs` | `build_globals` | Opacidad Landscape `0.65` $\to$ `0.95` | Valor Numérico Presentación |
| **M10** | `src/overlay.rs` | `build_globals` | Shell Portrait `1.0` $\to$ `0.0` | Flag Visual |
| **M11** | `src/overlay.rs` | `Globals::pack` | Intercambio de campos `opacity` y `shell` | Alineación Memoria Uniform |
| **M12** | `src/overlay.rs` | `button_glyph` | `Up => Glyph::ArrowUp` $\to$ `Glyph::ArrowDown` | Mapeo de Símbolos |
| **M13** | `src/overlay.rs` | `ElemParams::pack` | Campo `self.pressed` $\to$ forzado a `0.0` | Estado Dinámico Shader |
| **M14** | `src/error_screen.rs` | `select_surface_format_and_alpha` | `!f.is_srgb()` $\to$ `f.is_srgb()` | Inversión de Preferencia |
| **M15** | `src/error_screen.rs` | `ErrorScreenRenderer::render` | Bucle de fila `0..width` $\to$ `0..stride` | Desbordamiento de Padding |
| **M16** | `src/error_screen.rs` | `select_surface_format_and_alpha` | Fallback `.unwrap_or(...)` $\to$ `.unwrap()` | Supresión de Tolerancia |

---

### 4. Criterios de Aceptación

- [x] Ejecutar la auditoría completa vía `python scripts/auditar_mutantes.py`.
- [x] Todos los 16 mutantes deben ser detectados por al menos un test unitario o de integración y resultar en fallo de la suite (`cargo test --lib != 0`).
- [x] Cero mutantes supervivientes (Mutation Score = 100%).
- [x] Cero líneas de código de producción modificadas.
- [x] Suite completa de 49 tests en verde al restaurar el repositorio.

---

### 5. Comando de Verificación

```bash
python scripts/auditar_mutantes.py
cargo test
```

---

## Verificación (la completa el agente tras ejecutar)

### Resultado de la Auditoría Automatizada

Ejecución de `python scripts/auditar_mutantes.py` sobre la suite en commit base:

```text
Verificando línea base (suite en verde)...
Línea base OK: Suite 100% en verde.

Iniciando auditoría sobre 16 mutantes dirigidos...
======================================================================
[M01] ELIMINADO (Killed): Operador relacional en TouchArea::contains (<= a <)
[M02] ELIMINADO (Killed): Inversión lógica en TouchController::touch_down (!self.touch_enabled)
[M03] ELIMINADO (Killed): Descarte de puntero repetido en TouchController::touch_down
[M04] ELIMINADO (Killed): Constante de máscara KEYINPUT en TouchController::key_mask (0x03FF a 0x01FF)
[M05] ELIMINADO (Killed): Inversión de condición en TouchController::is_pressed (any a all)
[M06] ELIMINADO (Killed): Colisión de bitmask en Button::key_bit (B a 0x0001 en lugar de 0x0002)
[M07] ELIMINADO (Killed): Arm de cruceta colapsado en layout (0.55 a 0.15, invade deadzone)
[M08] ELIMINADO (Killed): Anulación de ángulo de inclinación SP (16.0 a 0.0)
[M09] ELIMINADO (Killed): Opacidad en Landscape en build_globals (0.65 a 0.95)
[M10] ELIMINADO (Killed): Carcasa shell en Portrait en build_globals (1.0 a 0.0)
[M11] ELIMINADO (Killed): Alineación de uniforms en Globals::pack (intercambio opacity y shell)
[M12] ELIMINADO (Killed): Asignación de glifo invertida en button_glyph (Up a ArrowDown)
[M13] ELIMINADO (Killed): Alineación uniforme en ElemParams::pack (pressed forzado a 0.0)
[M14] ELIMINADO (Killed): Preferencia invertida en select_surface_format_and_alpha (!f.is_srgb() a f.is_srgb())
[M15] ELIMINADO (Killed): Desbordamiento de memoria sobre padding en ErrorScreenRenderer::render (width a stride)
[M16] ELIMINADO (Killed): Eliminación de fallback seguro en select_surface_format_and_alpha (unwrap directo)
======================================================================

Resumen de Mutación:
  Total Mutantes: 16
  Eliminados:     16
  Sobrevivientes: 0
  Mutation Score: 100.0%

RESULTADO EXCELENTE: 100% de los mutantes fueron eliminados por la suite de tests.
```

### Predicho vs. Real

| Métrica | Predicho | Real | Desviación |
|---|---|---|---|
| Total Mutantes Evaluados | 16 | 16 | Exacto |
| Mutantes Eliminados (*Killed*) | $\ge 15$ | 16 | Superado (+1) |
| Mutantes Sobrevivientes (*Survived*) | $\le 1$ | 0 | 0 supervivientes |
| **Mutation Score** | $\ge 93.7\%$ | **100.0%** | Resistencia máxima |
| Estado Final de la Suite | 49 tests en verde | 49 tests en verde | Conforme |
| Alteraciones a Código de Producción | 0 líneas | 0 líneas | Conforme |

---

### Fuera de alcance (hallazgos que no se arreglaron)

- Ninguno. La suite de pruebas no presentó mutantes equivalentes ni puntos ciegos semánticos en los módulos puros evaluados.
