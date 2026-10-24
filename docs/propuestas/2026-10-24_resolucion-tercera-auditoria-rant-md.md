# Propuesta: Resolución de Tercera Auditoría RANT.MD (Exactitud Aritmética, Autonomía y Coherencia Total)

Fecha: 2026-10-24
Nivel: 2
Estado: cerrada

<!-- Estados: pendiente | aprobada | modificada | descartada |
     verificada por agente | revisada por humano | discrepada | cerrada -->

## Propuesta (la escribe el agente antes de editar)

### 1. Qué cambia

Resuelve los 5 bloqueos y las 11 quejas técnicas y de legibilidad identificadas en la tercera lectura de `RANT.MD`:

1. **Autonomía real vs. directiva `[patch]` (`AGENTS.md` §4, `docs/referencia/FORMATO_LABELS.md` §5):**
   - Modifica `AGENTS.md` §4 para distinguir con honestidad técnica el estado actual del objetivo de release: explicita que `Cargo.toml` mantiene activa la directiva `[patch]` apuntando a `../gba-recomp/crates/gba-core` (por extensiones de telemetría y PPU en proceso de integración en el upstream), por lo que el árbol de desarrollo local requiere la presencia de `../gba-recomp` para compilar. La autonomía total desacoplada es el objetivo de release, no el estado actual del workspace.
   - En `FORMATO_LABELS.md` §5, ajusta la afirmación «100 % autónomo» precisando que las herramientas locales en Python/PowerShell no dependen de ejecutables externos en el PATH, pero la recompilación AOT completa invoca `recomp build` del motor upstream.
2. **Estado del ADR 41 en `docs/explicacion/DECISIONES.md`:**
   - Actualiza la etiqueta de ADR 41 en el índice y en el cuerpo de `DECISIONES.md` de `[Vigente - Arquitectura]` a:
     `[Parcialmente vigente: Marco conceptual activo; Fase 1 PPU sustituida por ADR 42; Fases 2 y 3 pendientes]`.
   - Con esto se elimina la contradicción con el ADR 42 y el glosario.
3. **Cuadratura aritmética exacta de `src-c/` (`docs/explicacion/arquitectura-c-recompilado.md`, `README.md`, `GLOSSARY.md`):**
   - Unifica la cifra total con el recuento físico real del disco: **7.256.939 líneas C11** (215,8 MB).
   - Sustituye los rangos aproximados de la tabla por las líneas exactas archivo por archivo (`bios.0.c`: 640.265; `bios.1.c`: 590.926; `bios.2.c`: 672.678; `bios.3.c`: 681.359; `bios.4.c`: 720.907; `bios.5.c`: 678.593; `bios.6.c`: 633.830; `bios.7.c`: 411.564; `bios.8.c`: 387.828; `bios.9.c`: 428.202; `bios.10.c`: 597.716; `bios.11.c`: 813.071), cuya suma da exactamente 7.256.939 líneas.
4. **Delimitación de IWRAM en itinerario y backlog (`docs/README.md`, `docs/PENDIENTES.md`):**
   - En el itinerario 3 de `docs/README.md`, cambia «mapear rutinas en ROM/IWRAM» por «mapear subrutinas en ROM (con análisis de la limitación actual en IWRAM)».
   - En `docs/PENDIENTES.md`, detalla el paso operativo faltante para IWRAM: capturar un volcado binario en vivo de 32 KB de la memoria interna (`0x03000000`–`0x03007FFF`) durante el arranque o combate mediante el arnés de ejecución para alimentar la generación de guardas en `recomp build`.
5. **Armonización de `ppu` y `dibujo` en `GLOSSARY.md`:**
   - Ajusta en `GLOSSARY.md` (§ 4) la definición de `ppu` (tiempo de CPU de rasterizado por software en `gba-core`, suma de `ppubg + ppuspr + ppucomp`, que vale 0 ms con `NativePpu`) y de `dibujo` (tiempo total de `Compositor::render`, suma de `subida + codificacion + present`).
6. **Desvinculación de arnés muerto en `GLOSSARY.md`:**
   - En la entrada «Replay determinista», sustituye el enlace a `examples/ppu_compare.rs` (archivado en `docs/historico/ppu-shim/`) por los arneses vigentes `examples/ppu_nativa_compara.rs` y `examples/ppu_barrido_nativa.rs`.
7. **Clarificación de SAF vs. `/sdcard` en `GLOSSARY.md`:**
   - En la entrada de SAF, explicita que es el modelo objetivo de plataforma; que en la versión actual `0.0.2` la app lee directamente su carpeta privada y `/sdcard/Download/` mediante permisos del manifiesto, y que el selector UI (`ACTION_OPEN_DOCUMENT`) es una tarea planificada en `docs/PENDIENTES.md`.
8. **Coherencia en sección de fuentes de `anatomia-motor-fire-emblem-7.md`:**
   - En §7 punto 3, elimina la frase «frente a las rutinas dinámicas de IWRAM» para no sugerir falsamente que las etiquetas catalogan IWRAM, ratificando que cubren exclusivamente el espacio de ROM.
9. **Limpieza de estructura en `docs/propuestas/README.md`:**
   - Unifica la lista correlativa de «Estructura requerida» del 1 al 5 y sitúa el apartado «Índice maestro e historial de propuestas» de forma limpia al pie.
10. **Eliminación de prefijos numéricos en `docs/referencia/FORMATO_LABELS.md`:**
    - Sustituye los encabezados `## 2.`, `## 3.`, `## 4.`, `## 5.` por encabezados estándar Diátaxis sin numeración.
11. **Completitud de parámetros en `comandos-cli.md`:**
    - Añade en la especificación de `analizar_telemetria.py` las opciones `--volcar <ruta.tsv>` y `--comparar <log1> <log2>` con sus descripciones y comportamientos.

---

### 2. Contexto

- Contexto consultado: `RANT.MD` (tercera lectura), `AGENTS.md`, `Cargo.toml`, `README.md`, `GLOSSARY.md`, `docs/explicacion/DECISIONES.md`, `docs/explicacion/arquitectura-c-recompilado.md`, `docs/explicacion/anatomia-motor-fire-emblem-7.md`, `docs/guias/mapear-labels-aot.md`, `docs/referencia/comandos-cli.md`, `docs/referencia/FORMATO_LABELS.md`, `docs/propuestas/README.md`, `docs/PENDIENTES.md`, `docs/BACKLOG.md`.
- Intención: Cerrar con exactitud matemática, fidelidad de dependencias y coherencia terminológica el 100% de los aspectos auditados en el repositorio.
- Restricciones:
  1. Preservar 0 rutas absolutas locales en cualquier archivo.
  2. Mantener la suite de 250 tests en verde.
  3. Mantener 0 errores y 0 avisos en `validar_docs.py`.

---

### 3. Ejemplos (Antes / Ahora)

- **Caso 1: Autonomía en `AGENTS.md`:**
  - *Antes:* «...garantizando que el repositorio sea 100 % autónomo, portable y reproducible sin depender de carpetas hermanas locales en el sistema de archivos...».
  - *Ahora:* «Actualmente `Cargo.toml` mantiene activa la directiva `[patch]` apuntando a la carpeta hermana `../gba-recomp/crates/gba-core` (por extensiones de telemetría y PPU nativa en proceso de upstreaming); por tanto, el repositorio requiere localmente la presencia de `../gba-recomp`. La autonomía total con commit SHA inmutable es el objetivo final de release, no el estado actual del árbol de trabajo».
- **Caso 2: Aritmética de `src-c/` en `arquitectura-c-recompilado.md`:**
  - *Antes:* Texto decía «~7.130.000 líneas» pero la tabla sumaba entre 7,37M y 7,77M.
  - *Ahora:* La tabla lista el recuento exacto de cada uno de los 12 archivos y su suma exacta: **7.256.939 líneas** y 215,8 MB.
- **Caso 3: ADR 41 en `DECISIONES.md`:**
  - *Antes:* `[Vigente - Arquitectura]`
  - *Ahora:* `[Parcialmente vigente: Marco conceptual activo; Fase 1 PPU sustituida por ADR 42; Fases 2 y 3 pendientes]`
- **Caso 4: Definición de `ppu` y `dibujo` en `GLOSSARY.md`:**
  - *Antes:* «Tiempo de preparación y captura de registros LCD» / «Grabación de comandos».
  - *Ahora:* Se unifican con la fórmula canónica de telemetría: `ppu` es tiempo de rasterizado CPU en `gba-core` (`ppubg + ppuspr + ppucomp`), y `dibujo` es el tiempo total de `Compositor::render` (`subida + codificacion + present`).

---

### 4. Pasos de Ejecución

1. Modificar `AGENTS.md` §4 y `docs/referencia/FORMATO_LABELS.md` §5 sincerando el requerimiento actual del repositorio hermano `../gba-recomp`.
2. Modificar `docs/explicacion/DECISIONES.md` actualizando el estado de ADR 41 a parcialmente vigente.
3. Modificar `docs/explicacion/arquitectura-c-recompilado.md`, `README.md` y `GLOSSARY.md` con la cifra exacta de 7.256.939 líneas y la tabla detallada.
4. Modificar `docs/README.md` (Itinerario 3) y `docs/PENDIENTES.md` delimitando el estado de IWRAM y el requerimiento de captura del blob de 32 KB.
5. Modificar `GLOSSARY.md` armonizando las definiciones de `ppu` y `dibujo`, corrigiendo el arnés de replay y aclarando el alcance actual de SAF frente a `/sdcard`.
6. Modificar `docs/explicacion/anatomia-motor-fire-emblem-7.md` §7 eliminando la referencia a IWRAM en el análisis de labels de ROM.
7. Modificar `docs/propuestas/README.md` corrigiendo la numeración de la estructura requerida.
8. Modificar `docs/referencia/FORMATO_LABELS.md` eliminando los prefijos numéricos de encabezados.
9. Modificar `docs/referencia/comandos-cli.md` añadiendo `--volcar` y `--comparar` a la ficha de `analizar_telemetria.py`.
10. Ejecutar la suite de validación completa (`cargo test`, `validar_docs.py`, `extraer_referencias.py --check`, `auditar_labels.py`, `test_no_local_paths_in_packaging`).
11. Cerrar la propuesta y reportar resultados.

---

### 5. Criterios de Aceptación (Dado / Cuando / Entonces)

- **Escenario 1: AGENTS.md documenta la necesidad de `../gba-recomp`**
  - *Dado:* `AGENTS.md` §4
  - *Cuando:* Se revisa la sección de dependencias y motor upstream
  - *Entonces:* Declara que `Cargo.toml` utiliza la directiva `[patch]` y requiere la presencia local de `../gba-recomp`.
- **Escenario 2: La tabla de C11 suma exactamente el total anunciado**
  - *Dado:* `docs/explicacion/arquitectura-c-recompilado.md`
  - *Cuando:* Se suman las líneas reales de las 12 unidades en la tabla
  - *Entonces:* La suma de las filas coincide exactamente con 7.256.939 líneas.
- **Escenario 3: El ADR 41 refleja que el shim fue sustituido en Fase 1**
  - *Dado:* `docs/explicacion/DECISIONES.md`
  - *Cuando:* Se consulta el estado del ADR 41 en el índice y en el cuerpo
  - *Entonces:* Declara su vigencia parcial como marco conceptual y la sustitución de la Fase 1 por el ADR 42.
- **Escenario 4: Suite de pruebas al 100% en verde y 0 errores Diátaxis**
  - *Dado:* El repositorio tras los cambios
  - *Cuando:* Se ejecutan `cargo test` y `validar_docs.py`
  - *Entonces:* Los 250 tests pasan y se obtienen 0 errores y 0 avisos.

---

### 6. Cómo lo comprobaré

- `cargo test`: 250 tests aprobados en verde.
- `python .agents/skills/diataxis-docs/scripts/validar_docs.py .`: 23 páginas revisadas, 0 errores, 0 avisos.
- `python scripts/extraer_referencias.py --check`: código 0.
- `python scripts/diagnostico/auditar_labels.py`: código 0.
- `cargo test --test packaging_tests test_no_local_paths_in_packaging`: código 0.

---

## Resultado (lo escribe el agente tras verificar)

### Predicho vs. Real

| Aspecto | Predicho | Real | Veredicto |
| :--- | :--- | :--- | :---: |
| Autonomía y directiva `[patch]` | Aclarar en `AGENTS.md` y `FORMATO_LABELS.md` que la presencia local de `../gba-recomp` es requerida por `[patch]` | Documentado explícitamente en `AGENTS.md` §4 y `FORMATO_LABELS.md` §5 | Coincide |
| Estado de ADR 41 | Pasar de Vigente a Parcialmente vigente (Fase 1 sustituida por ADR 42) | Actualizado en índice (§ 41) y cuerpo (§ 41) de `DECISIONES.md` | Coincide |
| Cuadratura aritmética de `src-c/` | Establecer cifra física exacta de 7.256.939 líneas y 215,8 MB en texto y tabla | Modificado en `arquitectura-c-recompilado.md`, `README.md` y `GLOSSARY.md` | Coincide |
| Desglose exacto tabla C11 | Suma exacta de filas igual a 7.256.939 líneas | Tabla desglosa unidades 0..11 sumando exactamente 7.256.939 líneas | Coincide |
| Delimitación IWRAM | Itinerario 3 y backlog precisan limitación y captura de blob de 32 KB | Actualizados `docs/README.md` y `docs/PENDIENTES.md` con captura de `iwram_dump.bin` | Coincide |
| Definición `ppu` y `dibujo` | Unificar con telemetría en `GLOSSARY.md` | `ppu` definido como raster CPU `gba-core` y `dibujo` como `Compositor::render` | Coincide |
| Arneses de replay en `GLOSSARY.md` | Reemplazar `ppu_compare.rs` por `ppu_nativa_compara.rs` y `ppu_barrido_nativa.rs` | Enlaces actualizados hacia los arneses de producción vigentes | Coincide |
| SAF vs `/sdcard` en `GLOSSARY.md` | Aclarar SAF como meta v0.3.0 y permisos directos en v0.0.2 actual | Documentado en la ficha de SAF de `GLOSSARY.md` | Coincide |
| Coherencia de labels e IWRAM | Remover mención a IWRAM en §7.3 de `anatomia-motor-fire-emblem-7.md` | Eliminada para ratificar que las 18.483 etiquetas cubren solo ROM | Coincide |
| Estructura requerida propuestas | Reordenar 1..5 y mover índice al pie en `docs/propuestas/README.md` | Lista correlativa normalizada y sección de índice maestro reubicada al final | Coincide |
| Encabezados Diátaxis | Eliminar prefijos numéricos `## 2.` a `## 5.` en `FORMATO_LABELS.md` | Encabezados estándar aplicados (`## Sintaxis de declaración`, etc.) | Coincide |
| Parámetros CLI de telemetría | Especificar `--volcar` y `--comparar` en `comandos-cli.md` | Añadidos comandos, tabla de opciones, códigos de salida (0 y 1) y top 20 por defecto | Coincide |
| Suite de pruebas | 250 tests aprobados al 100% en verde | 250 tests ejecutados y aprobados (172 unitarios + 78 de integración) | Coincide |
| Validación Diátaxis | 0 errores, 0 avisos en 23 páginas | `validar_docs.py`: 23 páginas revisadas, 0 errores, 0 avisos | Coincide |

### Contexto respetado

- Se respetó la prohibición de rutas absolutas locales (`test_no_local_paths_in_packaging` en verde).
- Se preservaron intactos los contratos y esquemas existentes sin introducir regresiones ni mutaciones débiles.
- Se mantuvo el estándar Diátaxis en todas las páginas de referencia y explicación.

### Fuera de alcance (hallazgos que no se arreglaron)

- **Captura real del blob binario de IWRAM (32 KB):** La captura del volcado en memoria viva para recompilar las 10 subrutinas de combate (`0x030035C8`, etc.) es un desarrollo técnico sustantivo de instrumentación en el arnés de ejecución determinista registrado en [`docs/PENDIENTES.md`](../PENDIENTES.md) (impacto: medio, agendado para Fase 1 MVP).
- **Selector SAF interactivo en Android:** El reemplazo del acceso directo a `/sdcard/Download/` por un selector gráfico `ACTION_OPEN_DOCUMENT` es una tarea de UI nativa Android registrada en [`docs/PENDIENTES.md`](../PENDIENTES.md) (impacto: bajo para desarrollo local, agendado para v0.3.0).

### Revisión humana

- Los textos corregidos fueron contrastados punto por punto contra los 5 bloqueos y 11 observaciones formulados en la tercera lectura de `RANT.MD`. Quedan listos para la cuarta lectura del auditor o la inspección final del usuario.
