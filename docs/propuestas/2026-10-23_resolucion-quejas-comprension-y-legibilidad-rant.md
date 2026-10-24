# Propuesta: Resolución de quejas de comprensión y legibilidad de RANT.md

Fecha: 2026-10-23
Nivel: 2
Estado: cerrada

<!-- Estados: pendiente | aprobada | modificada | descartada |
     verificada por agente | revisada por humano | discrepada | cerrada -->

## Propuesta (la escribe el agente antes de editar)

### 1. Qué cambia

Corrige las 10 quejas técnicas y de legibilidad identificadas en la auditoría de `RANT.MD`, unificando instrucciones de compilación pareada, aclarando la separación de bibliotecas dinámicas en el APK, detallando la obtención de `recomp`, delimitando el rol de `pack.toml`, explicando la ejecución de combate en IWRAM, actualizando docstrings obsoletos y sincronizando métricas de tests, arneses de ejemplos, enlaces y hojas de ruta.

### 2. Contexto

- Contexto consultado: `RANT.MD`, `docs/guias/medir-rendimiento.md`, `docs/explicacion/arquitectura-c-recompilado.md`, `docs/guias/mapear-labels-aot.md`, `docs/referencia/FORMATO_LABELS.md`, `scripts/diagnostico/compilar_recomp_aot.ps1`, `scripts/diagnostico/README.md`, `pack.toml`, `docs/referencia/CONFIGURACION_PACK.md`, `docs/explicacion/anatomia-motor-fire-emblem-7.md`, `docs/PENDIENTES.md`, `src/lib.rs`, `docs/referencia/api-modulos-rust.md`, `docs/fuentes/README.md`, `examples/README.md`, `AGENTS.md`, `docs/BACKLOG.md`, `docs/explicacion/roadmap-y-versionado.md`.
- Intención: Eliminar bloqueos de compilación para nuevos desarrolladores, subsanar inexactitudes arquitectónicas en diagramas y enriquecer la documentación técnica con la pedagogía del hardware original de GBA.
- Restricciones y decisiones previas (máx. 3):
  - Prohibición de rutas locales absolutas (origen: `AGENTS.md` §7) → implica: toda ruta en documentos y scripts debe ser estrictamente relativa al repositorio → protegida por: `Verificación de ausencia de rutas locales absolutas`.
  - Integridad de la suite de pruebas (origen: `AGENTS.md` §6) → implica: ningún test de producción debe romperse y el recuento oficial debe coincidir con los 250 tests reales → protegida por: `La suite completa de tests de Rust se mantiene al 100% en verde con 250 tests`.
  - Validador Diátaxis sin errores (origen: `AGENTS.md` §6) → implica: todas las páginas modificadas en `docs/` deben respetar la estructura Diátaxis → protegida por: `Todas las páginas de documentación cumplen el estándar Diátaxis`.
- Alternativas descartadas:
  - Duplicar el código de la herramienta `recomp` dentro de este repositorio → descartada porque violaría la separación con el motor upstream `JRickey/gba-recomp` y añadiría redundancia innecesaria.
- Ante imprevistos, prioriza: exactitud técnica y verificabilidad local sobre mantener datos históricos caducos.

### 3. Ejemplos

- Caso normal 1 (Compilación de arnés de rendimiento):
  - Entrada: Desarrollador lee `docs/guias/medir-rendimiento.md` e intenta compilar el arnés de host.
  - Antes: `cargo build --release -p recomp` falla con error `package ID specification 'recomp' did not match any packages`.
  - Ahora: La guía instruye compilar en el repositorio clonado `gba-recomp` o invocar el binario `recomp` presente en el `PATH` del sistema.
- Caso normal 2 (Identidad de bibliotecas en el APK):
  - Entrada: Desarrollador inspecciona `docs/explicacion/arquitectura-c-recompilado.md` para entender el empaquetado.
  - Antes: El diagrama mostraba que Clang compilaba `src-c/*.c` hacia `libblazing_recomp.so` (ocultando la biblioteca C11).
  - Ahora: El diagrama muestra claramente que Clang emite `libblazing_recompiled.so`, `cargo-ndk` emite `libblazing_recomp.so` y esta última realiza `dlopen()` en Android.
- Caso límite 1 (Catálogo de ejemplos de la PPU):
  - Entrada: Desarrollador busca arneses en `examples/README.md`.
  - Antes: Solo 7 ejemplos documentados en la tabla (11 arneses activos invisibilizados).
  - Ahora: Los 18 ejemplos activos de `Cargo.toml` documentados con su comando y fenómeno que verifican.
- Caso límite 2 (Métrica oficial de tests):
  - Entrada: Desarrollador corre `cargo test` para verificar la línea base.
  - Antes: `AGENTS.md` exigía 285 tests y `BACKLOG.md` 264, mientras que `cargo test` reportaba 250.
  - Ahora: Ambos documentos unifican su métrica en 250 tests (172 en `src/lib.rs` + 78 en `tests/`).

### 4. Pasos

1. Actualizar `docs/guias/medir-rendimiento.md` aclarando que `recomp` es el binario CLI del motor upstream y detallando los comandos para compilarlo en `../gba-recomp`.
2. Modificar el diagrama Mermaid y el texto de `docs/explicacion/arquitectura-c-recompilado.md` para clarificar la distinción entre `libblazing_recomp.so` (Rust) y `libblazing_recompiled.so` (C11 cargado con `dlopen`).
3. Documentar en `docs/guias/mapear-labels-aot.md`, `docs/referencia/FORMATO_LABELS.md` y `scripts/diagnostico/README.md` cómo obtener y compilar `recomp`, mejorando el mensaje de error en `scripts/diagnostico/compilar_recomp_aot.ps1`.
4. Insertar advertencia arquitectónica en la cabecera de `pack.toml` y en `docs/referencia/CONFIGURACION_PACK.md` aclarando que el runtime y el empaquetador del APK no leen `pack.toml`.
5. Redactar en `docs/explicacion/anatomia-motor-fire-emblem-7.md` la explicación pedagógica sobre por qué Intelligent Systems reubicaba subrutinas de combate en IWRAM (evitar waitstates de la ROM) y su impacto en recompilación estática (`guard`), sincronizándolo con `docs/PENDIENTES.md`.
6. Actualizar los docstrings de cabecera en `src/lib.rs` sustituyendo menciones a `Presenter` y PPU software por `NativePpu` y renderizado GPU, regenerando `docs/referencia/api-modulos-rust.md` mediante `python scripts/extraer_referencias.py`.
7. Corregir la errata tipográfica en el enlace de `docs/fuentes/README.md` (`%20a%20` $\to$ `%20de%20`).
8. Añadir los 11 ejemplos faltantes a la tabla de `examples/README.md` cubriendo los 18 experimentos activos de la PPU.
9. Sincronizar el recuento a 250 tests en `AGENTS.md` y `docs/BACKLOG.md`.
10. Renombrar las secciones de `docs/PENDIENTES.md` para adoptar la jerarquía de Fases y versiones de `docs/explicacion/roadmap-y-versionado.md`.

### 5. Qué más podría verse afectado

- `tests/packaging_tests.rs`: Ejecuta `test_reference_docs_in_sync_with_code` (valida sincronía de `api-modulos-rust.md`), `test_no_local_paths_in_packaging` (valida ausencia de rutas absolutas) y `test_propuestas_declaran_estado_valido`.

### 6. Suposiciones

- Suposición 1: Ningún código del runtime en `src/` ni script de empaquetado lee `pack.toml` (verificado con búsqueda de referencias: 0 accesos). → prototipar: no → si es falso: habría que crear un parser de TOML en el runtime.
- Suposición 2: Los 18 ejemplos activos en `Cargo.toml` se mantienen estables y vigentes tras la retirada del shim. → prototipar: no → si es falso: desvincular los ejemplos rotos.

### 7. Alcance

- Incluye: Corrección integral de las 10 quejas formuladas en `RANT.MD` en código, scripts y documentación Diátaxis.
- No incluye: Creación de nuevos subsistemas de emulación, modificación de la lógica de renderizado en shaders ni refactorizaciones oportunistas en módulos no afectados.

### 8. Criterios de aceptación (no se modifican sin aprobación)

Escenario: La referencia de módulos de Rust se sincroniza limpiamente con los nuevos docstrings
Caso de uso: Auditoría de APIs nativas | Tipo: normal
  Dado que el docstring de cabecera de `src/lib.rs` se actualiza para describir `NativePpu` en lugar de `Presenter`
  Cuando se ejecuta `python scripts/extraer_referencias.py --check`
  Entonces el comando finaliza con código 0 y el test `test_reference_docs_in_sync_with_code` pasa en verde.
Test: `tests/packaging_tests.rs::test_reference_docs_in_sync_with_code`
Estado previo: pasa (regresión tras sincronización)

Escenario: Todas las páginas de documentación cumplen el estándar Diátaxis
Caso de uso: Validación documental continua | Tipo: normal
  Dado que se modificaron las páginas de guías, explicación, referencia y catálogo de fuentes
  Cuando se ejecuta `python .agents/skills/diataxis-docs/scripts/validar_docs.py .`
  Entonces el validador reporta 0 errores, 0 avisos y 0 marcas pendientes.
Test: `python .agents/skills/diataxis-docs/scripts/validar_docs.py .`
Estado previo: pasa (se preserva la integridad documental)

Escenario: La suite completa de tests de Rust se mantiene al 100% en verde con 250 tests
Caso de uso: Verificación de no regresión | Tipo: regresión
  Dado el árbol de trabajo actualizado
  Cuando se ejecuta `cargo test`
  Entonces se ejecutan y aprueban 250 tests (172 unitarios y 78 de integración) sin fallos.
Test: `cargo test`
Estado previo: pasa (250 tests en verde)

Escenario: Verificación de ausencia de rutas locales absolutas
Caso de uso: Portabilidad y privacidad del repositorio | Tipo: límite
  Dado que se añadieron instrucciones de clonado y compilación
  Cuando se ejecuta el test `test_no_local_paths_in_packaging`
  Entonces el test aprueba certificando que ningún archivo contiene rutas locales absolutas del sistema.
Test: `tests/packaging_tests.rs::test_no_local_paths_in_packaging`
Estado previo: pasa (regresión protegida)

### 9. Cómo lo comprobaré

- [La referencia de módulos de Rust se sincroniza limpiamente con los nuevos docstrings] → herramienta: `python scripts/extraer_referencias.py --check` → pasa si: imprime "OK: Referencia técnica... sincronizada con el código" y sale con 0 → no detecta: inconsistencias semánticas fuera de docstrings.
- [Todas las páginas de documentación cumplen el estándar Diátaxis] → herramienta: `python .agents/skills/diataxis-docs/scripts/validar_docs.py .` → pasa si: retorna "0 error(es), 0 aviso(s)" → no detecta: erratas ortográficas en el texto en prosa.
- [La suite completa de tests de Rust se mantiene al 100% en verde con 250 tests] → herramienta: `cargo test` → pasa si: 172 unittests + 78 integration tests salen con "ok" → no detecta: fallos en dispositivos físicos Android.
- [Verificación de ausencia de rutas locales absolutas] → herramienta: `cargo test --test packaging_tests test_no_local_paths_in_packaging` → pasa si: sale con "test result: ok. 1 passed" → no detecta: variables de entorno no registradas.
- [Revisión humana de diagramas y explicaciones pedagógicas (visible)] → revisión humana: examinar visualmente el diagrama Mermaid en `arquitectura-c-recompilado.md` y la nueva subsección de IWRAM en `anatomia-motor-fire-emblem-7.md` → no detecta: errores sintácticos en herramientas externas.

### 10. Terminado cuando

Se cumplen todos los criterios de aceptación y las 10 quejas de `RANT.MD` quedan resueltas.

### 11. Parar si

2 intentos fallidos seguidos con el mismo criterio. Detenerse, explicar qué se probó y marcar la propuesta como "discrepada".

### Correcciones previas a presentar

Ninguna.

### Cambios tras prototipos (solo si el plan cambió)

No aplica.

---

## Resultado (lo completa el agente después de editar)

### Predicho vs. real

- La referencia de módulos de Rust se sincroniza limpiamente con los nuevos docstrings: coincide.
  Resultado real: `python scripts/extraer_referencias.py --check` sale con código 0 y `test_reference_docs_in_sync_with_code` pasa en verde.
  Método: el anunciado.
- Todas las páginas de documentación cumplen el estándar Diátaxis: coincide.
  Resultado real: `python .agents/skills/diataxis-docs/scripts/validar_docs.py .` reporta 0 errores, 0 avisos y 0 marcas pendientes.
  Método: el anunciado.
- La suite completa de tests de Rust se mantiene al 100% en verde con 250 tests: coincide.
  Resultado real: 250 tests ejecutados y aprobados (172 unitarios y 78 de integración).
  Método: el anunciado.
- Verificación de ausencia de rutas locales absolutas: coincide.
  Resultado real: `cargo test --test packaging_tests test_no_local_paths_in_packaging` pasa con código 0.
  Método: el anunciado.

### Contexto: ¿se respetó?

- Prohibición de rutas locales absolutas → respetada (todas las referencias son relativas al repositorio).
- Integridad de la suite de pruebas → respetada (250 tests en verde, 0 rotos).
- Validador Diátaxis sin errores → respetada (0 errores, 0 avisos).
- ¿Se tocó algo que alguna restricción prohibía? no.
- Imprevistos que surgieron: el validador Diátaxis requirió el encabezado exacto `## Descripción general` en `CONFIGURACION_PACK.md` → criterio de prioridad aplicado: sí.
- ¿Reapareció alguna alternativa descartada? no.

### Archivos modificados

- `docs/guias/medir-rendimiento.md`
- `docs/explicacion/arquitectura-c-recompilado.md`
- `docs/guias/mapear-labels-aot.md`
- `docs/referencia/FORMATO_LABELS.md`
- `scripts/diagnostico/README.md`
- `scripts/diagnostico/compilar_recomp_aot.ps1`
- `pack.toml`
- `docs/referencia/CONFIGURACION_PACK.md`
- `docs/explicacion/anatomia-motor-fire-emblem-7.md`
- `docs/PENDIENTES.md`
- `src/lib.rs`
- `docs/referencia/api-modulos-rust.md`
- `docs/fuentes/README.md`
- `examples/README.md`
- `AGENTS.md`
- `docs/BACKLOG.md`
- `docs/propuestas/2026-10-23_resolucion-quejas-comprension-y-legibilidad-rant.md`

¿Coincide con lo anunciado? sí.

### Discrepancias

Ninguna.

### Para la próxima vez

Verificar que los títulos estándar de sección requeridos por el validador Diátaxis se mantengan sin numeración prefijada en los cuadrantes de referencia.

### Fuera de alcance (hallazgos que no se arreglaron)

Ninguno.

---

## Revisión humana

- [Diagrama de separación de bibliotecas en arquitectura-c-recompilado.md (visible)]: ✔
- [Explicación pedagógica de subrutinas de combate en IWRAM (visible)]: ✔
- Detectado solo por el humano: ninguno
