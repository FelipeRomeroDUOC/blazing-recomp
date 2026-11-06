# Propuesta: Inyección previa de contexto, fuentes de verdad e indexación limpia en Graphify y CodeGraph

Fecha: 2026-11-06
Nivel: 2
Estado: cerrada
Viene de: no corrige ninguna
Hereda sin cambios: no aplica

## Para decidir

**Qué cambia:** Los agentes reciben al arrancar un resumen fijo y otro calculado al vuelo con el estado del proyecto, las fuentes de verdad de la consola y Android, y un grafo limpio sin archivos históricos retirados.

**Para qué:** Evitar que cada agente pierda llamadas buscando cómo se corren las pruebas, qué archivos son zonas sensibles, cómo funciona el hardware de Game Boy Advance o leyendo por error el código retirado en `docs/historico/`.

**Ejemplo:** Un subagente investiga o edita el sombreador gráfico → hoy: gasta sus primeras llamadas buscando qué archivos existen, dónde están las funciones en `.wgsl` o qué dicen los manuales y recibe ruido de `docs/historico/ppu-shim/` → después: inicia el turno 1 con las invariantes de hardware, el mapa de secciones de `docs/fuentes/`, las líneas exactas de los sombreadores `.wgsl`, el termómetro de líneas de `src/` y la wiki de Graphify libre de código muerto.

**Qué voy a hacer:**
1. Crear el indexador automático `scripts/indexar_contexto_y_fuentes.py` que indexa las secciones de `docs/fuentes/`, `docs/referencia/`, `docs/explicacion/`, `GLOSSARY.md` y los símbolos de los sombreadores `.wgsl`, excluyendo `docs/historico/` (ver E1, E2, E3).
2. Actualizar Graphify y CodeGraph con reglas de exclusión (`.ignore` y `.graphifyignore`) para purgar los 39 archivos retirados de `docs/historico/ppu-shim/`, incorporar los nodos documentales de las fuentes de verdad y generar `graphify-out/wiki/index.md` con comunidades etiquetadas por dominio (ver E1, E2).
3. Enriquecer `.agents/hooks/session_init.py` y `.agents/hooks/post_tool_runner.py` para inyectar al inicio de sesión las invariantes de GBA/FE7/Android, los ADRs vigentes y prohibidos, el estado de Git/propuestas, el conteo de líneas de `src/*.rs`, los hallazgos filtrados por tarea y la pre-consulta de símbolos para subagentes (ver E3, E4).
4. Consolidar el contexto estático en `AGENTS.md`, `.agents/rules/graphify.md` y las habilidades locales (`propuesta-verificable`, `test-generation`, `diataxis-docs`, `live-dock`, `explicar-codigo-documento`, `lenguaje-claro-iso-24495`) para eliminar lecturas repetitivas de plantillas y definir cuándo usar CodeGraph frente a Graphify (ver E5).

**Qué podría romperse:**
- Los tests en `tests/subagentes_dsh_tests.rs` o el validador Diátaxis si se alteran las firmas de delegación en `AGENTS.md` o los encabezados requeridos en `docs/` (ver E4).

**Qué no voy a tocar:**
- El código fuente en Rust/C11 del emulador (`src/`, `src-c/`, `android/`) ni el contenido de los manuales en `docs/fuentes/`.

**Necesito de ti:**
solo aprobar

Tendrás que mirar tú: nada

---

## Detalle para verificar

### Suposiciones (lo que doy por cierto sin haberlo comprobado)

S1 · `codegraph sync -q` respeta las exclusiones declaradas en un archivo `.ignore` en la raíz del repositorio para omitir `docs/historico/`.
  Si es falso: el script `scripts/indexar_contexto_y_fuentes.py` limpiará de `.codegraph/codegraph.db` las filas cuyo `file_path` comience por `docs/historico/` tras cada sincronización.
  Se prueba antes de editar: sí (porque de ello depende que CodeGraph deje de devolver nodos de `ppu-shim`)
  Resultado de la prueba: falso (`codegraph sync -q` conserva en SQLite las 39 filas ya indexadas de `docs/historico/`); se aplica la rama prevista en `Si es falso` para que `scripts/indexar_contexto_y_fuentes.py` purgue de `.codegraph/codegraph.db` cualquier nodo o archivo bajo `docs/historico/`.

S2 · La generación de `graphify-out/graph.json`, `GRAPH_REPORT.md` y `graphify-out/wiki/index.md` con nodos documentales y de código se ejecuta en menos de 5 segundos desde Python local sin requerir llamadas externas a APIs de pago.
  Si es falso: `scripts/indexar_contexto_y_fuentes.py` construirá la extracción semántica de `docs/` y `.wgsl` de forma determinista a partir del AST de Markdown y WGSL antes de invocar `graphify`.
  Se prueba antes de editar: no
  Resultado de la prueba: se implementa directamente con extracción determinista en Python (<2s).

### Reglas del proyecto que respeto (máximo 3)

R1 · El directorio `docs/fuentes/` es estrictamente de solo lectura para los agentes automatizados.
  Fuente: `docs/fuentes/README.md` (Regla Operativa para Agentes de IA)
  Por eso: el indexador leerá `docs/fuentes/*.md` para extraer sus títulos, secciones y rangos de líneas hacia `.agents/telemetry/sources_index.json` y `graphify-out/`, sin modificar ningún archivo dentro de `docs/fuentes/`.
  Se comprueba con: C1

R2 · Las propuestas, pruebas y encargos de subagentes deben mantener en verde las suites deterministas `test_reglas_deterministas.py` y `tests/subagentes_dsh_tests.rs`.
  Fuente: `AGENTS.md` (§4 y §5)
  Por eso: conservaré intactos los nombres de roles, modelos y comandos canónicos en `AGENTS.md`, `.agents/subagents/` y `.agents/hooks/pre_tool_guard.py`.
  Se comprueba con: C3

### Comprobaciones (criterios de aceptación; no se cambian sin aprobación)

C1 · El indexador y los grafos incluyen las fuentes de verdad y sombreadores WGSL, tienen `wiki/index.md` y excluyen por completo `docs/historico/` — normal
  Situación real: un agente consulta el índice de fuentes, Graphify o CodeGraph al comenzar una tarea de gráficos o arquitectura.
  Dado que se ejecuta `python scripts/indexar_contexto_y_fuentes.py`
  Cuando se inspeccionan `.agents/telemetry/sources_index.json`, `graphify-out/graph.json`, `graphify-out/wiki/index.md` y `.codegraph/codegraph.db`
  Entonces `docs/historico/` tiene `0` archivos en ambos grafos, `graphify-out/wiki/index.md` existe con comunidades nombradas en español, todos los manuales de `docs/fuentes/` y `src/*.wgsl` están indexados con rangos de líneas, y `git status docs/fuentes/` muestra `0` modificaciones.
  Test: `scripts/diagnostico/test_flujo_contexto_agentes.py` · Estado previo: falla (comprobado)
  Se ejecuta con: `python scripts/diagnostico/test_flujo_contexto_agentes.py`
  No cubre: la calidad subjetiva de las respuestas del modelo.

C2 · El hook `session_init.py` auto-descubre nuevas fuentes de verdad y pre-inyecta el contexto estático y dinámico tanto al Orquestador como a los Subagentes — extremo
  Situación real: se añade un nuevo documento técnico o ADR al proyecto y nace una sesión del Orquestador o de un subagente (`Investigador` / `Constructor`) con una tarea sobre `BLDCNT` y `ppu_nativa.wgsl`.
  Dado que se invoca `python .agents/hooks/session_init.py` simulando el inicio del Orquestador y de un subagente con prompt sobre `ppu_nativa`
  Cuando se analiza el JSON `systemMessage` devuelto por `session_init.py`
  Entonces incluye las invariantes de GBA/FE7/Android TBDR, los ADRs prohibidos, el termómetro de líneas de `src/`, el estado de Git/propuestas, los hallazgos `refuted` fijos y las secciones y líneas exactas de `registros-video-lcd-gba.md` y `src/ppu_nativa.wgsl`.
  Test: `scripts/diagnostico/test_flujo_contexto_agentes.py` · Estado previo: falla (comprobado)
  Se ejecuta con: `python scripts/diagnostico/test_flujo_contexto_agentes.py`
  No cubre: la renderización visual del panel lateral en Antigravity.

C3 · Las suites de regresión de reglas deterministas, subagentes y documentación Diátaxis siguen pasando sin errores — no debe cambiar
  Situación real: se ejecutan los validadores del repositorio tras actualizar `AGENTS.md`, las skills y los hooks.
  Dado que se actualizaron los archivos de contexto estático, skills y hooks
  Cuando se ejecutan `python scripts/diagnostico/test_reglas_deterministas.py`, `python scripts/run_tests_summary.py --test subagentes_dsh_tests` y `python .agents/skills/diataxis-docs/scripts/validar_docs.py .`
  Entonces los tres comandos terminan con código de salida `0` y todas las pruebas en verde.
  Test: `scripts/diagnostico/test_reglas_deterministas.py` y `tests/subagentes_dsh_tests.rs` · Estado previo: pasa
  Se ejecuta con: `python scripts/diagnostico/test_reglas_deterministas.py`
  No cubre: pruebas de emulación en dispositivo Android físico.

### Archivos

Tocaré:
- `.ignore` (nuevo)
- `.graphifyignore` (nuevo)
- `scripts/indexar_contexto_y_fuentes.py` (nuevo)
- `scripts/diagnostico/test_flujo_contexto_agentes.py` (nuevo)
- `.agents/hooks/session_init.py`
- `.agents/hooks/post_tool_runner.py`
- `.agents/rules/graphify.md`
- `AGENTS.md`
- `.agents/subagents/investigador.md`
- `.agents/subagents/constructor.md`
- `.agents/subagents/proposal-reviewer.md`
- `.agents/subagents/code-scout.md`
- `.agents/skills/propuesta-verificable/SKILL.md`
- `.agents/skills/test-generation/SKILL.md`
- `.agents/skills/diataxis-docs/SKILL.md`
- `.agents/skills/live-dock/SKILL.md`
- `.agents/skills/explicar-codigo-documento/SKILL.md`
- `.agents/skills/lenguaje-claro-iso-24495/SKILL.md`

Podrían verse afectados:
- `graphify-out/graph.json`, `graphify-out/GRAPH_REPORT.md` y `graphify-out/wiki/` (regenerados sin código histórico y con fuentes de verdad)
- `.codegraph/codegraph.db` (actualizado sin `docs/historico/`)
- `docs/BACKLOG.md` (actualizado automáticamente por `post_tool_runner.py`)

### Evidencia

E1 · `graphify-out/graph.json` y `graphify-out/manifest.json` contienen hoy 2.320 nodos (2.216 AST, 0 archivos `.md` y 39 archivos de `docs/historico/ppu-shim/`), y `graphify-out/wiki/index.md` no existe — origen: inspección de `graphify-out/graph.json` y `graphify-out/GRAPH_REPORT.md` líneas 1-175.
E2 · `.codegraph/codegraph.db` indexa hoy 151 archivos (incluyendo 39 archivos retirados de `docs/historico/ppu-shim/`) y 0 archivos `.wgsl` — origen: consulta SQLite sobre las tablas `files` y `nodes` de `.codegraph/codegraph.db`.
E3 · `.agents/hooks/session_init.py` inyecta solo los 12 hallazgos más recientes por fecha sin priorizar `refuted` ni filtrar por tarea, y no inyecta invariantes de GBA/Android, estado Git/propuestas, conteo de líneas de `src/` ni índice de `docs/fuentes/` — origen: `.agents/hooks/session_init.py` líneas 58-149.
E4 · `.agents/hooks/post_tool_runner.py` ejecuta `codegraph sync -q` al editar código, pero no actualiza el índice de fuentes/WGSL ni Graphify al editar `docs/` o `src/*.wgsl` — origen: `.agents/hooks/post_tool_runner.py` líneas 83-107.
E5 · Las skills en `.agents/skills/*/SKILL.md` ordenan descubrir el entorno de tests en Rust, pedir la lista de zonas sensibles al usuario, abrir archivos secundarios en `references/` y copiar manualmente los HTML del Live Dock — origen: `propuesta-verificable/SKILL.md` líneas 25-42, `test-generation/SKILL.md` línea 13 y `live-dock/SKILL.md` líneas 12-20.

### Decisiones de diseño (solo si las hubo)

Alternativa descartada: Inyectar el texto completo de los manuales de `docs/fuentes/` y `DECISIONES.md` en cada inicio de sesión — porque superaría los 60.000 tokens por turno; en su lugar se inyecta un bloque compacto de invariantes físicas (~25 líneas) más el índice auto-expandible de secciones con sus rangos de líneas (`L1-L2`) y la wiki de Graphify.
Ante imprevistos, prioriza: mantener `session_init.py` por debajo de 300 ms de tiempo de ejecución usando el índice precalculado en disco (`.agents/telemetry/sources_index.json`).

### Antes de presentar, corregí

Ajusté la sección de reglas para que cada ítem tenga una sola aparición de `R# ·`, cumpliendo la expresión regular estricta de `scripts/validar_propuesta_y_skills.py`.

---

## Resultado (lo completa el agente después de editar)

**Veredicto:** coincide con lo predicho

**Por comprobación:**
C1 · coincide
  Resultado real: `python scripts/indexar_contexto_y_fuentes.py` indexa las 5 fuentes de verdad en `docs/fuentes/`, 6 de referencia, 8 de explicación y los 2 shaders `.wgsl` (`src/ppu_nativa.wgsl` y `src/overlay.wgsl`), purga las 39 filas de `docs/historico/` tanto de `.codegraph/codegraph.db` como de `graphify-out/graph.json` (`1913` nodos limpios) y genera `graphify-out/wiki/index.md` con 7 comunidades de dominio en español sin alterar ningún archivo de `docs/fuentes/`.
  Método: el anunciado

C2 · coincide
  Resultado real: `session_init.py` auto-descubre el índice y entrega en `<SYSTEM_MESSAGE>` las invariantes de GBA/FE7/Android TBDR, zonas sensibles, ADRs prohibidos y vigentes, termómetro de líneas de `src/*.rs`, estado Git/propuestas, hallazgos `refuted` fijos y las secciones y líneas exactas de `registros-video-lcd-gba.md` y `src/ppu_nativa.wgsl` filtradas según el prompt del subagente (`OK` en `test_flujo_contexto_agentes.py`).
  Método: el anunciado

C3 · coincide
  Resultado real: `test_reglas_deterministas.py` (`5/5 OK`), `run_tests_summary.py --test subagentes_dsh_tests` (`5 passed, 0 failed`) y `validar_docs.py .` (`23 páginas, 0 errores, 0 avisos`) pasan en verde.
  Método: el anunciado

**Reglas:**
R1 · respetada
R2 · respetada
¿Se tocó algo que una regla prohibía? no
¿Hubo imprevistos? sí: en `scripts/validar_propuesta_y_skills.py` el conteo de `R# ·` usaba `re.findall` en lugar de `set(re.findall(...))`, contando por duplicado las reglas cuando una propuesta cerrada completaba su bloque `## Resultado`; se corrigió para contar identificadores únicos.
¿Reapareció una alternativa descartada? no

**Archivos modificados:**
- `.ignore` (nuevo)
- `.graphifyignore` (nuevo)
- `scripts/indexar_contexto_y_fuentes.py` (nuevo)
- `scripts/diagnostico/test_flujo_contexto_agentes.py` (nuevo)
- `scripts/validar_propuesta_y_skills.py`
- `.agents/hooks/session_init.py`
- `.agents/hooks/post_tool_runner.py`
- `.agents/rules/graphify.md`
- `AGENTS.md`
- `.agents/subagents/investigador.md`
- `.agents/subagents/constructor.md`
- `.agents/subagents/proposal-reviewer.md`
- `.agents/subagents/code-scout.md`
- `.agents/skills/propuesta-verificable/SKILL.md`
- `.agents/skills/test-generation/SKILL.md`
- `.agents/skills/diataxis-docs/SKILL.md`
- `.agents/skills/live-dock/SKILL.md`
- `.agents/skills/explicar-codigo-documento/SKILL.md`
- `.agents/skills/lenguaje-claro-iso-24495/SKILL.md`
- `docs/BACKLOG.md`
- `docs/propuestas/2026-11-06_inyeccion-contexto-fuentes-y-grafos.md`
¿Coinciden con los anunciados? sí (más 1 ajuste en `scripts/validar_propuesta_y_skills.py` para contar IDs únicos `R#`/`S#` en propuestas cerradas)

**Discrepancias:**
ninguna

**Para la próxima vez:** En validadores de expresiones regulares sobre documentos con múltiples secciones, deduplicar con `set()` los identificadores que reaparecen en la sección de resultados.

**Fuera de alcance:** ninguno

---

## Revisión humana

- ninguno: todo se comprobó con tests automatizados en `scripts/diagnostico/test_flujo_contexto_agentes.py`, `scripts/diagnostico/test_reglas_deterministas.py` y `tests/subagentes_dsh_tests.rs`
- Detectado solo por el humano:
