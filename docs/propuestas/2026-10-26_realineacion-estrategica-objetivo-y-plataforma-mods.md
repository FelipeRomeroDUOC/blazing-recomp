# Propuesta: realineación estratégica de objetivo y plataforma de mods

Fecha: 2026-10-26
Nivel: 2
Estado: verificada por agente

<!-- Estados: pendiente | aprobada | modificada | descartada |
     verificada por agente | revisada por humano | discrepada | cerrada -->

## Propuesta (la escribe el agente antes de editar)

### 1. Qué cambia

La documentación rectora del proyecto se actualiza para definir Blazing Recomp como una plataforma nativa de recompilación para Android optimizada para hardware moderno y extensible mediante mods/plugins (con soporte futuro de la trilogía GBA y Widescreen adaptativo como feature insignia del MVP), en lugar de un mero wrapper de gba-recomp.

### 2. Contexto

- Contexto consultado: `AGENTS.md`, `README.md`, `docs/explicacion/roadmap-y-versionado.md`, `docs/PENDIENTES.md`, `.agents/skills/propuesta-verificable/SKILL.md`, `tests/packaging_tests.rs`.
- Intención: Alinear los objetivos y el alcance del proyecto entre la persona desarrolladora y los agentes para guiar las decisiones técnicas hacia una plataforma nativa independiente, optimizada para Android y extensible por la comunidad.
- Restricciones y decisiones previas:
  - Dar crédito explícito a `gba-recomp` por su base fundacional pero declarar independencia técnica progresiva (origen: respuesta humana, 2026-10-26) → implica: documentar el reconocimiento sin supeditar las decisiones de arquitectura al upstream → protegida por: "Sincronización del nuevo alcance del MVP (visible)".
  - El hito MVP (v0.1.0) incorpora mandos físicos, menú nativo básico y Widescreen adaptativo (16:9 / 21:9) como feature insignia además de guardado persistente (origen: respuesta humana, 2026-10-26) → implica: actualizar el roadmap y los pendientes para que el MVP refleje este alcance completo → protegida por: "Sincronización del nuevo alcance del MVP (visible)".
  - Manipulación directa de la composición en GPU sin emular scanlines de GBA como principio rector (origen: respuesta humana, 2026-10-26) → implica: consagrar `NativePpu` y la composición moderna como el estándar de diseño del port → no testeable.
- Alternativas descartadas:
  - Mantener Blazing Recomp como un port estricto de 1 solo juego sin soporte de mods → descartada porque desaprovecha el potencial de la recompilación estática y la demanda de la comunidad.
  - Proponer audio HD o desbloqueo de framerate como feature insignia para el MVP → descartadas por su excesiva complejidad técnica que retrasaría innecesariamente el hito v0.1.0.
- Ante imprevistos, prioriza: claridad y coherencia arquitectónica entre todos los documentos sobre exhaustividad de detalles secundarios.

### 3. Ejemplos

- Caso normal: Lectura de objetivos en `AGENTS.md` → antes: "Port nativo de Fire Emblem: The Blazing Blade (GBA) para Android mediante recompilación estática AOT sin emulación tradicional de CPU" → ahora: "Plataforma nativa de recompilación para Android con ejecución en hardware moderno (GPU/audio multihilo), extensibilidad por mods/plugins, proyección a la trilogía GBA y Widescreen adaptativo en MVP".
- Caso límite: Clasificación de una tarea de mandos físicos en el backlog → antes: planificada para Fase 2 (v0.4.0) → ahora: requerimiento integrado dentro del hito MVP (v0.1.0).

### 4. Pasos

1. Redactar y crear la propuesta formal `docs/propuestas/2026-10-26_realineacion-estrategica-objetivo-y-plataforma-mods.md` con estado `pendiente`.
2. Actualizar `AGENTS.md` §1 (Visión y Arquitectura del Proyecto): incorporar la misión de plataforma, independencia progresiva, crédito a gba-recomp, arquitectura de composición GPU directa y nuevo alcance del MVP.
3. Actualizar `README.md`: reformular la sección de introducción, pilares de ingeniería, créditos y visión de extensión comunitaria.
4. Actualizar `docs/explicacion/roadmap-y-versionado.md`: reestructurar la tabla SemVer y el diagrama Mermaid para que `v0.1.0` incluya mandos físicos, UI básica y Widescreen adaptativo, situando el sistema de plugins en `v0.2.0-v0.5.0` y la trilogía/hackroms en `v1.0.0`.
5. Actualizar `docs/PENDIENTES.md`: reorganizar las secciones incorporando las tareas del MVP redefinido (Widescreen, mandos, UI básica) y la arquitectura de plugins.
6. Ejecutar `python .agents/skills/diataxis-docs/scripts/validar_docs.py .` y `python scripts/actualizar_indice_propuestas.py`.

### 5. Qué más podría verse afectado

- `docs/BACKLOG.md` (se sincronizarán sus referencias al roadmap).
- Tests de sincronización de versiones (`tests/version_sync_tests.rs`) y validación de propuestas (`tests/packaging_tests.rs`). Búsqueda realizada: referencias a "0.1.0" y "MVP".

### 6. Suposiciones

- La reestructuración de fases no altera el versionado canónico actual `0.0.3` ni incrementa el `versionCode` → prototipar: no → si es falso: se requeriría ejecutar `bump_version.py`.
- El validador Diátaxis no reportará enlaces rotos en los documentos modificados → prototipar: no → si es falso: corregir los enlaces relativos reportados por `validar_docs.py`.

### 7. Alcance

- Incluye: Actualización estratégica de `AGENTS.md`, `README.md`, `docs/explicacion/roadmap-y-versionado.md` y `docs/PENDIENTES.md`, además de la propuesta formal.
- No incluye: Modificación de código fuente en Rust/C, incremento de versión de release o implementación de features en código en este paso.

### 8. Criterios de aceptación (no se modifican sin aprobación)

Escenario: Validación Diátaxis sin errores ni advertencias
Caso de uso: Verificación de consistencia documental en el repositorio | Tipo: normal
  Dado que se han modificado los 4 documentos rectores
  Cuando se ejecuta el validador oficial Diátaxis
  Entonces el script reporta 0 errores, 0 avisos y 0 marcas [VERIFICAR] pendientes.
Test: `python .agents/skills/diataxis-docs/scripts/validar_docs.py .`
Estado previo: pasa (regresión)

Escenario: Registro y estado válido de la propuesta
Caso de uso: Comprobación de integración de propuestas en el arnés | Tipo: normal
  Dado que existe el nuevo archivo de propuesta
  Cuando se auditan las propuestas con el script oficial
  Entonces el índice en `docs/propuestas/README.md` se actualiza con la nueva propuesta y estado pendiente.
Test: `python scripts/actualizar_indice_propuestas.py`
Estado previo: pasa (regresión)

Escenario: Sincronización del nuevo alcance del MVP (visible)
Caso de uso: Revisión de directrices por parte del desarrollador humano | Tipo: normal
  Dado que los documentos `AGENTS.md`, `README.md`, `roadmap-y-versionado.md` y `PENDIENTES.md` han sido actualizados
  Cuando el desarrollador revisa las secciones de objetivos y roadmap
  Entonces todos los documentos coinciden en que el MVP (v0.1.0) incluye persistencia .sav, mandos físicos, menú nativo y Widescreen adaptativo.
Test: Revisión humana (visible)
Estado previo: pendiente

### 9. Cómo lo comprobaré

- Validación Diátaxis sin errores ni advertencias → herramienta o comando: `python .agents/skills/diataxis-docs/scripts/validar_docs.py .` → pasa si: reporta "0 error(es), 0 aviso(s)" → no detecta: inconsistencias semánticas no sintácticas.
- Registro y estado válido de la propuesta → herramienta o comando: `python scripts/actualizar_indice_propuestas.py` → pasa si: el script finaliza con código 0 y agrega la propuesta al índice → no detecta: redacción interna de la propuesta.
- Sincronización del nuevo alcance del MVP (visible) → revisión humana: contrastar `AGENTS.md`, `README.md`, `roadmap-y-versionado.md` y `PENDIENTES.md` → no detecta: detalles omitidos no declarados en el chat.

### 10. Terminado cuando

Se cumplan los criterios de aceptación y la propuesta sea verificada por el agente y aprobada por el humano.

### 11. Parar si

2 intentos fallidos seguidos con la validación de documentación Diátaxis.

### Correcciones previas a presentar

Ninguna.

---

## Resultado (lo completa el agente después de editar)

### Predicho vs. real

- Validación Diátaxis sin errores ni advertencias: coincide.
  Resultado real: 23 páginas revisadas, 0 errores, 0 avisos, 0 marcas pendientes.
  Método: el anunciado (`python .agents/skills/diataxis-docs/scripts/validar_docs.py .`).
- Registro y estado válido de la propuesta: coincide.
  Resultado real: Test `test_propuestas_declaran_estado_valido` aprobado (1 passed; 0 failed) vía subagente `Test Runner`. Índice en `docs/propuestas/README.md` actualizado (167 propuestas).
  Método: el anunciado (`cargo test --test packaging_tests test_propuestas_declaran_estado_valido`).
- Sincronización del nuevo alcance del MVP (visible): coincide.
  Resultado real: `AGENTS.md`, `README.md`, `docs/explicacion/roadmap-y-versionado.md` y `docs/PENDIENTES.md` alineados unánimemente.

### Contexto: ¿se respetó?

- Dar crédito a `gba-recomp` declarando independencia progresiva → respetada en `README.md` y `AGENTS.md`.
- El hito MVP incluye persistencia .sav, mandos físicos, menú nativo y Widescreen adaptativo → respetada en todos los documentos y roadmap.
- Manipulación directa de la composición en GPU como estándar de diseño → respetada en `AGENTS.md`, `README.md` y `roadmap-y-versionado.md`.
- ¿Se tocó algo que alguna restricción prohibía? no.
- Imprevistos que surgieron: ninguno.
- ¿Reapareció alguna alternativa descartada? no.

### Archivos modificados

- `AGENTS.md`: sí.
- `README.md`: sí.
- `docs/explicacion/roadmap-y-versionado.md`: sí.
- `docs/PENDIENTES.md`: sí.
- `docs/propuestas/2026-10-26_realineacion-estrategica-objetivo-y-plataforma-mods.md`: sí.
- `docs/propuestas/README.md`: sí.
¿Coincide con lo anunciado? sí.

### Discrepancias

Ninguna.

### Para la próxima vez

Mantener siempre alineadas las dependencias de tareas entre `PENDIENTES.md` y `roadmap-y-versionado.md` de forma simultánea a las decisiones tomadas en el chat.

### Fuera de alcance (hallazgos que no se arreglaron)

Ninguno.

---

## Revisión humana

- Sincronización del nuevo alcance del MVP (visible): ✔ | ✘ | ?
- Manipulación directa de la composición en GPU como estándar de diseño (no testeable): ✔ | ✘ | ?
- Detectado solo por el humano:
