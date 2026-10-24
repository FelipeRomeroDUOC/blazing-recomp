# Propuesta: Resolución de Segunda Auditoría RANT.MD (Fidelidad, Coherencia y Claridad Operativa)

Fecha: 2026-10-24
Nivel: 2
Estado: cerrada

<!-- Estados: pendiente | aprobada | modificada | descartada |
     verificada por agente | revisada por humano | discrepada | cerrada -->

## Propuesta (la escribe el agente antes de editar)

### 1. Qué cambia

Resuelve la totalidad de los 5 bloqueos graves, 5 quejas de comprensión adicionales y 6 quejas de legibilidad identificadas en la segunda lectura de `RANT.MD`:

1. **Verdad sobre subrutinas en IWRAM (`docs/explicacion/anatomia-motor-fire-emblem-7.md`, `docs/guias/mapear-labels-aot.md`, `docs/referencia/FORMATO_LABELS.md`):**
   - Corrige la afirmación errónea de que las subrutinas de combate (`0x030035C8` a `0x030036CC` / `sub_8059E7C`) están catalogadas en `blazing.labels.toml`. Se explicita que **aún no están catalogadas** (enlace a `docs/PENDIENTES.md`), que en la versión actual caen al intérprete de respaldo durante el combate, y que su mapeo requiere instrumentar la extracción previa de memoria dinámica.
   - En `mapear-labels-aot.md` y `FORMATO_LABELS.md`, se clarifica el estado técnico de las etiquetas en IWRAM y se describe qué es el volcado de memoria viva (*blob*) requerido por el compilador AOT upstream.
2. **Coherencia normativa del fallback por software (`AGENTS.md`, `docs/guias/medir-rendimiento.md`):**
   - Armoniza la directiva de `AGENTS.md` §2.3 y §4: se prohíbe reintroducir emulación PPU por software *como pipeline primario de producción*, pero se autoriza formalmente la rasterización por software de `gba-core` *exclusivamente como red de contingencia y diagnóstico* (activada por la bandera `ppu-nativa-off.flag` o por degradación ante fallo de inicialización en GPU), conforme al ADR 42.
   - En `medir-rendimiento.md`, se enlaza esta excepción explícitamente al ADR 42.
3. **Transparencia en requisitos del repositorio hermano y directiva `[patch]` (`docs/guias/compilar-y-empaquetar-apk.md`, `README.md`):**
   - Incorpora en los «Requisitos» de la guía del APK la presencia obligatoria del repositorio hermano `../gba-recomp`, explicando que la directiva `[patch]` de `Cargo.toml` aplica a todas las invocaciones de `cargo ndk` y `cargo test`.
   - Ajusta la nota del `README.md` para delimitar con exactitud el alcance de `[patch]`.
4. **Estado temporal del Hardware Shim (`GLOSSARY.md`, `docs/explicacion/hardware-shim-y-eliminacion-de-emulacion.md`):**
   - En `GLOSSARY.md`, renombra la entrada a `Hardware Shim (Histórico)` e inicia declarando su retiro de producción en el ADR 42 y archivo en `docs/historico/ppu-shim/`.
   - En `hardware-shim-y-eliminacion-de-emulacion.md`, añade callout inicial con `Estado: [Sustituido en parte por ADR 42 / Registro de Evolución Arquitectónica]` y etiqueta el estado de las 3 fases de la hoja de ruta (Fase 1 completada y sustituida por `NativePpu`; Fases 2 y 3 pendientes/en planificación).
5. **Exactitud histórica en el análisis de rendimiento (`docs/explicacion/analisis_rendimiento.md`):**
   - Corrige la cita a `recomp.rs:4`, aclarando que los 17.207 correspondían al cómputo preliminar de funciones de la fecha del análisis (2026-10-10) frente a las 18.483 catalogadas actuales.
   - En las tablas de acciones (Ejes 1, 2 y 3), actualiza las filas marcadas como "Inmediata" que ya fueron aplicadas (latencia 2 el 2026-10-10, PPU nativa y cosecha exhaustiva de labels).
6. **Desmitificación de herramientas upstream (`docs/referencia/comandos-cli.md`, `pack.toml`, `README.md`):**
   - Explica el rol de `--record-labels` y clarifica que `gba-mapper` fue una utilidad upstream preliminar sustituida en este proyecto por los scripts modulares `cosechar_jump_tables_y_leafs.py` y `sembrar_labels_masivo.py`.
   - Limpia referencias obsoletas en comentarios de `pack.toml` y en `README.md`.
7. **Sincronización de versiones y erradicación del "Hito 6" (`pack.toml`, `README.md`, `docs/explicacion/roadmap-y-versionado.md`):**
   - Alinea `pack.toml` a `version = "0.0.2"`, coincidente con `Cargo.toml`.
   - Sustituye la denominación informal "Hito 6" en `README.md` y `anatomia-motor-fire-emblem-7.md` por su hito formal en el roadmap (`v0.2.0` / Fase 2).
8. **Rigor en tipos de frontmatter y destino del índice de propuestas (`docs/referencia/comandos-cli.md`, `docs/propuestas/README.md`, `docs/README.md`):**
   - Corrige en `comandos-cli.md` los 4 tipos canónicos de frontmatter: `tutorial`, `guia-practica`, `referencia`, `explicacion`.
   - Corrige la descripción de `actualizar_indice_propuestas.py`: documenta que reconstruye el índice en `docs/BACKLOG.md#índice-de-propuestas`, y enlaza esa sección desde `docs/propuestas/README.md` y `docs/README.md`.
9. **Legibilidad y orden operativo en guías (`docs/guias/compilar-y-empaquetar-apk.md`, `docs/guias/medir-rendimiento.md`, `README.md`, `docs/referencia/CONFIGURACION_PACK.md`, `docs/explicacion/README.md`):**
   - Mueve el aprovisionamiento de ROM/BIOS al Paso 4 de la guía del APK (previo al primer arranque) y documenta en `README.md` la pantalla diagnóstica de error.
   - En `medir-rendimiento.md`, prioriza la ruta interna de la app que no requiere permisos compartidos en Android, añade `../gba-recomp` en requisitos y aísla el resultado de mutación limpiando el error tipográfico (`n out/doradas/`).
   - Completa la sección «Estructura del Proyecto» en `README.md` con los directorios reales (`src/`, `src-c/`, `android/`, `scripts/`, `docs/`, `tests/`) eliminando la carpeta inexistente `mods/`.
   - Homogeneiza los encabezados de `CONFIGURACION_PACK.md` eliminando el prefijo `## 2.` para respetar el formato Diátaxis.
   - Añade el distintivo `(Histórico)` en el índice de `docs/explicacion/README.md` para `ppu-caminos-de-composicion.md`.

---

### 2. Contexto

- Contexto consultado: `RANT.MD` (segunda lectura), `AGENTS.md`, `README.md`, `pack.toml`, `Cargo.toml`, `GLOSSARY.md`, `docs/explicacion/anatomia-motor-fire-emblem-7.md`, `docs/explicacion/hardware-shim-y-eliminacion-de-emulacion.md`, `docs/explicacion/analisis_rendimiento.md`, `docs/explicacion/roadmap-y-versionado.md`, `docs/explicacion/README.md`, `docs/guias/compilar-y-empaquetar-apk.md`, `docs/guias/medir-rendimiento.md`, `docs/guias/mapear-labels-aot.md`, `docs/referencia/CONFIGURACION_PACK.md`, `docs/referencia/comandos-cli.md`, `docs/referencia/FORMATO_LABELS.md`, `docs/propuestas/README.md`, `docs/PENDIENTES.md`, `docs/BACKLOG.md`.
- Intención: Erradicar definitivamente cualquier inconsistencia o información engañosa en la documentación, alineándola 100% con la realidad del código fuente y del hardware.
- Restricciones (máx. 3):
  1. Cero rutas absolutas locales en cualquier archivo del repositorio (`AGENTS.md` §7).
  2. Preservar intacta la suite de 250 tests de Rust en verde (`cargo test`).
  3. Conformidad total con el validador Diátaxis (`validar_docs.py`).
- Alternativas descartadas:
  - Catalogar inventando direcciones en `blazing.labels.toml` para las rutinas de IWRAM: descartada porque rompería la fidelidad técnica. La solución honesta es documentar que es una tarea pendiente real en `docs/PENDIENTES.md`.

---

### 3. Ejemplos concretos (Antes / Ahora)

- **Caso 1: Subrutinas de IWRAM en `anatomia-motor-fire-emblem-7.md`:**
  - *Antes:* "...rutinas de interpolación y cálculo de combate como `sub_8059E7C` en `blazing.labels.toml`".
  - *Ahora:* "...rutinas de interpolación y cálculo de combate en ROM como `sub_8059E7C`. **Estado actual:** Estas subrutinas reubicadas en IWRAM aún no están catalogadas en `blazing.labels.toml` (ver tarea en `docs/PENDIENTES.md`); en la versión actual sus llamadas caen al intérprete de respaldo durante el combate hasta que se instrumente su extracción de memoria viva y catalogación con guardas".
- **Caso 2: Fallback por software en `AGENTS.md` y `medir-rendimiento.md`:**
  - *Antes:* `AGENTS.md` decía "Prohibido sugerir o reintroducir emulación PPU por software en producción" sin excepciones, mientras la guía mandaba activarlo con `ppu-nativa-off.flag`.
  - *Ahora:* `AGENTS.md` explicita: "Prohibido sugerir o reintroducir emulación PPU por software como pipeline primario en producción. La rasterización por software de `gba-core` se mantiene exclusivamente como mecanismo de contingencia y diagnóstico (activado por `ppu-nativa-off.flag` o ante fallo en la inicialización de GPU), conforme al ADR 42". Y la guía cita el ADR 42.
- **Caso 3: Estructura del proyecto en `README.md`:**
  - *Antes:* Solo listaba `pack.toml`, `blazing.labels.toml` y `mods/` (inexistente).
  - *Ahora:* Lista exhaustivamente `src/`, `src-c/`, `android/`, `scripts/`, `docs/`, `tests/`, `blazing.labels.toml` y `pack.toml`, describiendo el propósito de cada carpeta real.

---

### 4. Pasos de Ejecución

1. Modificar `AGENTS.md` §2.3 y §4 delimitando la prohibición de la PPU software a la vía primaria de producción y autorizando el fallback de diagnóstico del ADR 42.
2. Modificar `docs/explicacion/anatomia-motor-fire-emblem-7.md`, `docs/guias/mapear-labels-aot.md` y `docs/referencia/FORMATO_LABELS.md` transparentando el estado pendiente de las rutinas de IWRAM.
3. Modificar `GLOSSARY.md` y `docs/explicacion/hardware-shim-y-eliminacion-de-emulacion.md` marcando el Hardware Shim como histórico y etiquetando el estado de las 3 fases de la hoja de ruta.
4. Modificar `docs/explicacion/analisis_rendimiento.md` contextualizando la cifra histórica de funciones y actualizando las tablas de acciones de los Ejes 1, 2 y 3.
5. Modificar `pack.toml`, `README.md`, `docs/explicacion/roadmap-y-versionado.md` sincronizando la versión a `0.0.2` y sustituyendo "Hito 6" por `v0.2.0` / Fase 2.
6. Modificar `docs/guias/compilar-y-empaquetar-apk.md` añadiendo `../gba-recomp` a Requisitos y moviendo el aprovisionamiento de ROM/BIOS a los Pasos.
7. Modificar `docs/guias/medir-rendimiento.md` priorizando la ruta de archivos sin permisos, añadiendo el prerrequisito del motor upstream, citando el ADR 42 y corrigiendo la sección de mutantes.
8. Modificar `docs/referencia/comandos-cli.md`, `docs/referencia/CONFIGURACION_PACK.md`, `docs/propuestas/README.md` y `docs/README.md` corrigiendo tipos de frontmatter, encabezados, aclarando `gba-mapper` y enlazando el índice de propuestas en `docs/BACKLOG.md`.
9. Ejecutar verificaciones del Pre-Commit Gatekeeper: `validar_docs.py`, `cargo test`, `auditar_labels.py`, `extraer_referencias.py --check` y `test_no_local_paths_in_packaging`.
10. Cerrar formalmente la propuesta y reportar resultados.

---

### 5. Qué más podría verse afectado

- El validador de documentación Diátaxis (`validar_docs.py`).
- El test de sincronización de referencias de Rust (`extraer_referencias.py --check`).
- La prueba de higiene de paths absolutos (`test_no_local_paths_in_packaging`).

---

### 6. Criterios de Aceptación (Dado / Cuando / Entonces)

- **Escenario 1: Anatomía no afirma que rutinas de IWRAM están catalogadas**
  - *Dado:* `docs/explicacion/anatomia-motor-fire-emblem-7.md`
  - *Cuando:* Se consulta la sección de subrutinas reubicadas en IWRAM
  - *Entonces:* No afirma que existan en `blazing.labels.toml` y enlaza a `docs/PENDIENTES.md`.
- **Escenario 2: AGENTS.md armoniza con la guía de rendimiento y el ADR 42**
  - *Dado:* `AGENTS.md` y `docs/guias/medir-rendimiento.md`
  - *Cuando:* Se revisa la regla sobre renderizado por software en CPU
  - *Entonces:* Ambos textos concuerdan en que es un mecanismo de contingencia y diagnóstico de seguridad autorizado por el ADR 42.
- **Escenario 3: Requisitos del APK incluyen el repositorio hermano**
  - *Dado:* `docs/guias/compilar-y-empaquetar-apk.md`
  - *Cuando:* Se revisa la sección «Requisitos»
  - *Entonces:* Incluye la presencia de `../gba-recomp` para satisfacer la directiva `[patch]` de `Cargo.toml`.
- **Escenario 4: Suite de pruebas en verde y 0 errores Diátaxis**
  - *Dado:* El repositorio tras las ediciones
  - *Cuando:* Se ejecutan `cargo test` y `validar_docs.py`
  - *Entonces:* Los 250 tests pasan y se reportan 0 errores y 0 avisos.

---

### 7. Cómo lo comprobaré

- `cargo test`: 250 tests pasando en verde.
- `python .agents/skills/diataxis-docs/scripts/validar_docs.py .`: 0 errores, 0 avisos.
- `python scripts/extraer_referencias.py --check`: código de salida 0.
- `python scripts/diagnostico/auditar_labels.py`: código de salida 0.
- `cargo test --test packaging_tests test_no_local_paths_in_packaging`: código de salida 0.

---

## Resultado (lo escribe el agente tras verificar)

### 1. Predicho vs. real

| Criterio / Escenario | Predicho | Real | Desviación |
| :--- | :--- | :--- | :--- |
| **Subrutinas de IWRAM transparentadas** | Desmentir que estén en `blazing.labels.toml` y enlazar a `docs/PENDIENTES.md`. | Corregido en `anatomia-motor-fire-emblem-7.md`, `mapear-labels-aot.md` y `FORMATO_LABELS.md`. | Ninguna. |
| **Armonización del fallback por software** | Coherencia entre `AGENTS.md` (prohibido en producción primaria, autorizado como contingencia/diagnóstico) y `medir-rendimiento.md`. | Actualizado `AGENTS.md` §2.3 y citado el ADR 42 en la guía de rendimiento. | Ninguna. |
| **Prerrequisito de repo hermano en APK** | `../gba-recomp` documentado en Requisitos del APK y alcance de `[patch]` en `README.md`. | Requisitos actualizados con clon adyacente y nota de dependencias unificada. | Ninguna. |
| **Estado histórico del Hardware Shim** | Declarado histórico en `GLOSSARY.md` y fases etiquetadas en `hardware-shim-y-eliminacion-de-emulacion.md`. | Entrada renombrada a `Hardware Shim (Histórico)` y callout de evolución arquitectónica añadido. | Ninguna. |
| **Contextualización histórica de rendimiento** | `analisis_rendimiento.md` contextualizado como medición inicial (17.207 funciones en 2026-10-10) y tablas actualizadas. | Cita ajustada y tablas de Ejes 1, 2 y 3 marcadas con estados completados. | Ninguna. |
| **Desmitificación de herramientas** | Aclarar rol de `--record-labels` y `gba-mapper` en `comandos-cli.md` y `pack.toml`. | Eliminadas referencias fantasmas en `pack.toml`/`README.md` y documentado su reemplazo modular en CLI. | Ninguna. |
| **Sincronización de versiones y erradicación de "Hito 6"** | `pack.toml` a `0.0.2` y sustitución de "Hito 6" por `v0.2.0` / Fase 2. | `pack.toml` sincronizado a `0.0.2`; menciones de Hito 6 reemplazadas en `README.md` y anatomía. | Ninguna. |
| **Frontmatter canónico y destino de propuestas** | `guia-practica` en `comandos-cli.md` y enlace a `docs/BACKLOG.md#índice-de-propuestas`. | Frontmatter corregido y enlaces cruzados creados en `docs/propuestas/README.md` y CLI. | Ninguna. |
| **Orden operativo y legibilidad en guías** | Aprovisionamiento de ROM en pasos de APK; ruta interna sin permisos en rendimiento; estructura real en README; encabezados Diátaxis en `CONFIGURACION_PACK.md`. | Todas las guías actualizadas, estructura de carpetas ampliada y `mods/` inexistente removida. | Ninguna. |
| **Suite de pruebas de Rust** | 250 tests pasando en verde. | 250 tests aprobados al 100% (172 unitarios + 78 integración). | Ninguna. |
| **Validación Diátaxis (`validar_docs.py`)** | 0 errores y 0 avisos. | 23 páginas revisadas, 0 errores, 0 avisos. | Ninguna. |
| **Higiene de rutas locales** | 0 rutas absolutas locales detectadas. | `test_no_local_paths_in_packaging` aprobado en verde. | Ninguna. |

### 2. Qué se respetó del contexto
- Cero rutas absolutas locales introducidas (`AGENTS.md` §7).
- Preservación estricta de los 250 tests de la suite en verde.
- Estándar Diátaxis verificado en las 23 páginas de documentación.

### 3. Fuera de alcance (hallazgos que no se tocaron)
- La catalogación y extracción de blobs de memoria para las subrutinas de IWRAM permanece como tarea pendiente de código en [`docs/PENDIENTES.md`](../PENDIENTES.md), tal como se acordó para no inventar datos.
