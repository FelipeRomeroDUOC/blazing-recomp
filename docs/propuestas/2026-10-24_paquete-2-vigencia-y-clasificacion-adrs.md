# Propuesta: Paquete 2 — Vigencia y clasificación de los 46 ADRs en DECISIONES.md

Fecha: 2026-10-24
Nivel: 2
Estado: cerrada

<!-- Estados: pendiente | aprobada | modificada | descartada |
     verificada por agente | revisada por humano | discrepada | cerrada -->

## Propuesta (la escribe el agente antes de editar)

### 1. Qué cambia

Incorpora un sistema formal y explícito de clasificación de vigencia (`[Vigente]`, `[Sustituida por ADR XX]`, `[Histórica / Exploratoria]`) en la totalidad de los 46 Registros de Decisiones Arquitectónicas (ADR) de [`docs/explicacion/DECISIONES.md`](../explicacion/DECISIONES.md), tanto en la tabla de índice inicial como en la cabecera de cada decisión individual. Resuelve de forma definitiva el Bloqueo 3 y la Queja 4 formulados en [`RANT.MD`](../../RANT.MD), eliminando la confusión producida por decisiones cronológicas contradictorias (ADR 17 vs 24, ADR 21 vs 45, ADR 25 vs 27, y ADR 30-36 vs 41-42) y permitiendo al lector distinguir al instante qué arquitectura gobierna el código de producción actual frente a qué decisiones representan la memoria evolutiva del motor.

### 2. Contexto

- Contexto consultado: [`RANT.MD`](../../RANT.MD) (§ "Los ADR no dicen cuáles siguen vigentes" y Bloqueo 3), [`docs/explicacion/DECISIONES.md`](../explicacion/DECISIONES.md), [`docs/explicacion/hardware-shim-y-eliminacion-de-emulacion.md`](../explicacion/hardware-shim-y-eliminacion-de-emulacion.md), [`src/platform.rs`](../../src/platform.rs), [`src/pipeline.rs`](../../src/pipeline.rs), [`AGENTS.md`](../../AGENTS.md).
- Intención: Dotar a [`docs/explicacion/DECISIONES.md`](../explicacion/DECISIONES.md) de rigor documental sin perder su valor como registro histórico. Quien consulte un ADR específico debe saber de inmediato si la decisión sigue activa en el código o si fue reemplazada por una iteración posterior con mediciones empíricas más favorables.
- Restricciones y decisiones previas (máx. 3):
  - Preservación de la cronología y anclas existentes (origen: `RANT.MD` y `docs/explicacion/DECISIONES.md`) → implica: no reordenar los ADRs ni alterar sus identificadores `#N-...` para no romper referencias cruzadas → protegida por: `Integridad de anclas e índice de decisiones`.
  - Conformidad Diátaxis sin errores (origen: `AGENTS.md` §6) → implica: mantener los encabezados canónicos obligatorios de Explicación (`Contexto`, `Idea central`, `Alternativas y compromisos`) → protegida por: `Validación Diátaxis sin errores ni avisos`.
  - Integridad de la suite de pruebas sin regresiones (origen: `AGENTS.md` §6) → implica: los 250 tests de Rust deben mantenerse al 100% en verde → protegida por: `La suite completa de tests de Rust se mantiene al 100% en verde con 250 tests`.
- Alternativas descartadas:
  - Eliminar del documento las decisiones superadas (ej. ADR 17, 21, 25) → descartada porque un registro ADR debe preservar la memoria técnica de qué se intentó, qué se midió y por qué se cambió.
  - Reorganizar el archivo agrupando "Vigentes" arriba e "Históricas" abajo → descartada porque rompería la trazabilidad cronológica de cómo evolucionó el port.
- Ante imprevistos, prioriza: claridad de estado y trazabilidad sobre concisión extrema.

### 3. Ejemplos

- Caso normal 1 (Entrada en el índice cronológico):
  - Entrada: El lector revisa el índice para entender la configuración de swapchain.
  - Antes: `25. [Tope Dinámico Diferenciado de Swapchain (1280px Landscape / 960px Portrait) para 60 FPS Estables](#25-tope-dinámico-diferenciado-de-swapchain-1280px-landscape--960px-portrait-para-60-fps-estables)` seguido de `27. [Calibración Simétrica de Swapchain a 960px...]`.
  - Ahora: Se añade el distintivo de vigencia en la lista:
    `25. [Tope Dinámico Diferenciado de Swapchain...](#25-...) — [Sustituida por ADR 27]`
    `27. [Calibración Simétrica de Swapchain a 960px...](#27-...) — [Vigente]`
- Caso normal 2 (Cuerpo de decisión reemplazada):
  - Entrada: El lector entra al ADR 21 buscando el sistema de pacing de fotogramas.
  - Antes: El texto describe la suspensión pasiva con `thread::sleep` sin indicar que fue descartada después.
  - Ahora: La cabecera incluye:
    `> **Estado:** Sustituida por [ADR 45](#45-pacing-de-cuadros-coordinado-con-choreographer--vsync-y-cola-de-entrada-lock-free). La sincronización mediante `std::thread::sleep` o bucles `spin` fue retirada en favor de la coordinación directa con el VSync de la pantalla y el `Choreographer` de Android.`
- Caso límite 1 (Decisiones de optimización de la PPU software de `gba-core`):
  - Entrada: El lector lee los ADRs 30, 31, 34, 35 y 36 que optimizan el rasterizado en CPU.
  - Antes: Contradicción aparente con los ADRs 41 y 42 que adoptaron `NativePpu` en GPU.
  - Ahora: Se etiquetan como `[Histórica - Motor upstream / Fallback]`, indicando que documentan las mejoras aplicadas al núcleo de CPU de `gba-core`, activas hoy en el arnés de comparación `ppu_doble` en PC y en el fallback pasivo por software de Android ante fallos de GPU o con `ppu-nativa-off.flag`, pero no en la ruta primaria de producción.

### 4. Pasos

1. Clasificar la totalidad de los 46 ADRs en las tres categorías canónicas:
   - **`[Vigente]` (29 decisiones):** Decisiones que definen el comportamiento, arquitectura, seguridad, empaquetado y testing del código actual de producción (ej. 1, 2, 3, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 18, 19, 20, 22, 23, 24, 27, 28, 32, 41, 42, 43, 44, 45, 46).
   - **`[Sustituida por ADR XX]` (4 decisiones):**
     - ADR 17 (Swapchain a 1 cuadro) $\to$ Sustituida por ADR 24 (Triple buffering explícito, latencia = 2).
     - ADR 21 (Pacing con `thread::sleep`) $\to$ Sustituida por ADR 45 (Pacing con Choreographer/VSync y cola lock-free).
     - ADR 25 (Tope diferenciado 1280px/960px) $\to$ Sustituida por ADR 27 (Calibración simétrica a 960px).
     - ADR 40 (Calibración experimental) $\to$ Sustituida por ADR 27 / ADR 42.
   - **`[Histórica / Exploratoria]` (13 decisiones):**
     - ADR 4: Contexto de dependencia upstream vs. worktree local con `[patch]`.
     - ADR 26: Campaña preliminar de cosecha de hotspots (17.663 bloques frente a los 144.875 finales).
     - ADR 29, 30, 31, 34, 35, 36: Optimizaciones sobre el rasterizador por software en CPU de `gba-core` (vigente en arnés PC `ppu_doble` y fallback, no en la ruta primaria GPU).
     - ADR 33, 37, 38, 39: Experimentos con mediciones empíricas de resultado negativo (hipótesis descartadas documentadas).
2. Modificar la sección `## Registro Cronológico de Decisiones` en [`docs/explicacion/DECISIONES.md`](../explicacion/DECISIONES.md), añadiendo el estado de vigencia y enlaces a cada una de las 46 entradas del índice.
3. Insertar un bloque destacado `> **Estado:** ...` inmediatamente después de cada uno de los 46 títulos H3 (`### N. ...`), declarando el estado exacto y la relación con otras decisiones.
4. Validar que ninguna ancla existente se rompa y que el archivo cumpla con los guardianes de pre-commit.

### 5. Qué más podría verse afectado

- Enlaces internos en [`docs/explicacion/README.md`](../explicacion/README.md) y [`README.md`](../../README.md) que apunten a secciones de `DECISIONES.md`.
- El validador Diátaxis (`validar_docs.py`).

### 6. Suposiciones

- Suposición 1: Mantener intactos los textos de los títulos H3 (sin meter el estado dentro del encabezado `### N.`) previene la alteración de los slugs de anclas HTML (`#17-...`, `#24-...`), evitando romper enlaces existentes en otros documentos. → prototipar: no → si es falso: habría que actualizar todos los enlaces entrantes.

### 7. Alcance

- Incluye:
  - Clasificación y etiquetado de los 46 ADRs en el índice y en el cuerpo de [`docs/explicacion/DECISIONES.md`](../explicacion/DECISIONES.md).
  - Explicitación de las relaciones de sustitución (17 $\to$ 24, 21 $\to$ 45, 25 $\to$ 27) y clarificación de los roles de los ADRs 30-36 frente al 41-42.
- No incluye:
  - Modificación de código fuente en `src/` o `src-c/`.
  - Reestructuración de la anatomía del motor (reservada para el Paquete 3).

### 8. Criterios de aceptación (no se modifican sin aprobación)

Escenario: El índice de DECISIONES.md declara el estado de vigencia de las 46 decisiones
Caso de uso: Lector consulta la lista general de decisiones arquitectónicas | Tipo: normal
  Dado el archivo `docs/explicacion/DECISIONES.md`
  Cuando se inspecciona la sección `## Registro Cronológico de Decisiones`
  Entonces las 46 entradas cuentan con su distintivo de estado (`[Vigente]`, `[Sustituida por ADR XX]` o `[Histórica...]`) sin romper sus anclas de navegación.
Test: inspección de enlaces y estados en `docs/explicacion/DECISIONES.md`
Estado previo: falla (hoy ninguna entrada del índice lleva estado)

Escenario: Las cabeceras H3 de cada ADR incluyen el bloque de estado correspondiente
Caso de uso: Lector accede a un ADR específico mediante un enlace directo | Tipo: normal
  Dado cualquier ADR de `docs/explicacion/DECISIONES.md`
  Cuando se examinan las líneas inmediatamente posteriores al título `### N. ...`
  Entonces existe un bloque `> **Estado:**` que especifica si la decisión es vigente, sustituida o histórica, enlazando al ADR sustituto cuando corresponda.
Test: `Select-String -Path "docs/explicacion/DECISIONES.md" -Pattern "> \*\*Estado:\*\*"` (debe arrojar 46 coincidencias)
Estado previo: falla (hoy arroja 0 coincidencias)

Escenario: La suite completa de tests de Rust se mantiene al 100% en verde con 250 tests
Caso de uso: Verificación de no regresión | Tipo: regresión
  Dado el árbol de trabajo tras la actualización de DECISIONES.md
  Cuando se ejecuta `cargo test`
  Entonces los 250 tests (172 unitarios y 78 de integración) pasan en verde.
Test: `cargo test`
Estado previo: pasa (250 tests en verde)

Escenario: Validación Diátaxis sin errores ni avisos
Caso de uso: Verificación de estructura documental | Tipo: regresión
  Dado el archivo `docs/explicacion/DECISIONES.md` modificado
  Cuando se ejecuta `python .agents/skills/diataxis-docs/scripts/validar_docs.py .`
  Entonces reporta 22 páginas revisadas, 0 errores y 0 avisos.
Test: `python .agents/skills/diataxis-docs/scripts/validar_docs.py .`
Estado previo: pasa

### 9. Cómo lo comprobaré

- [Conteo de estados en ADRs] → herramienta: `Select-String -Path "docs/explicacion/DECISIONES.md" -Pattern "> \*\*Estado:\*\*"` → pasa si: 46 coincidencias exactas → no detecta: erratas ortográficas dentro de las notas de justificación.
- [Validación Diátaxis] → herramienta: `python .agents/skills/diataxis-docs/scripts/validar_docs.py .` → pasa si: 0 errores y 0 avisos → no detecta: enlaces externos que apunten a dominios caídos.
- [Suite de tests en verde] → herramienta: `cargo test` → pasa si: 250 tests aprobados → no detecta: fallos en hardware exclusivo de Android.

---

## Resultado (lo escribe el agente tras verificar)

### 1. Predicho vs. real

### 1. Predicho vs. real

| Predicción | Resultado real | Estado |
| :--- | :--- | :---: |
| Incorporar distintivos de estado en el índice cronológico de [`DECISIONES.md`](../explicacion/DECISIONES.md) para los 46 ADRs. | Modificadas las 46 entradas del índice con etiquetas `— **[Vigente]**`, `— **[Sustituida por ADR XX]**` o `— **[Histórica / Exploratoria]**`. | Conforme |
| Añadir bloque `> **Estado:**` con nota de justificación bajo cada uno de los 46 títulos H3. | Verificadas 46 ocurrencias de `> **Estado:**` en el cuerpo del documento mediante búsqueda automatizada. | Conforme |
| Articular las 4 contradicciones directas señaladas en `RANT.MD` (17/24, 21/45, 25/27, 30-36/41-42). | ADR 17 remite a 24; ADR 21 remite a 45; ADR 25 remite a 27; ADRs 30 a 36 quedan catalogados como optimizaciones del motor upstream / fallback sin contradecir a 41 y 42. | Conforme |
| Preservar las anclas existentes `#N-...` sin romper enlaces entrantes. | Los títulos H3 `### N. ...` se mantuvieron intactos, preservando el slug canónico de cada sección. | Conforme |
| Suite completa de 250 tests en verde. | `cargo test` ejecutado y aprobado con 250 tests (172 unitarios y 78 de integración). | Conforme |
| Validador Diátaxis sin errores ni avisos. | `validar_docs.py` ejecutado: 22 páginas revisadas, 0 errores, 0 avisos. | Conforme |

### 2. Qué se respetó del contexto

- Se preservó la cronología histórica completa de [`docs/explicacion/DECISIONES.md`](../explicacion/DECISIONES.md) sin eliminar ninguna decisión.
- Se respetó la prohibición de rutas locales absolutas (`AGENTS.md` §7): comprobado con `test_no_local_paths_in_packaging`.
- Se respetó el estándar Diátaxis: encabezados `Contexto`, `Idea central`, `Alternativas y compromisos` preservados.

### 3. Fuera de alcance (hallazgos que no se arreglaron)

- **Verificabilidad metodológica y direcciones de ROM/labels en `anatomia-motor-fire-emblem-7.md` (Bloqueo 4):** Reservado para el Paquete 3. — impacto: medio
- **Aritmética de líneas de `src-c/` en `arquitectura-c-recompilado.md` (Queja 14):** Reservado para el Paquete 3. — impacto: bajo
- **Reestructuración Diátaxis de `medir-rendimiento.md` e itinerarios de lectura en `docs/README.md` (Quejas 16 y 17):** Reservado para el Paquete 3. — impacto: medio
