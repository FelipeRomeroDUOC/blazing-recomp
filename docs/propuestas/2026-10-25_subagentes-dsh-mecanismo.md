# Propuesta: adaptar los subagentes al mecanismo de DeepSeek Harness

Fecha: 2026-10-25
Nivel: 2
Estado: verificada por agente

## Propuesta (la escribe el agente antes de editar)

### 1. Qué cambia

Los cuatro subagentes del proyecto siguen existiendo con los mismos roles y los mismos disparadores, pero ahora se pueden invocar desde DeepSeek Harness (DSH) además de desde Antigravity.

### 2. Contexto

- Contexto consultado: `AGENTS.md` (§4), `.agents/hooks/pre_tool_guard.py`, `.agents/hooks/session_init.py`, `.agents/hooks.json`, `.agents/agent_state.json`, `tests/version_sync_tests.rs`, `tests/packaging_tests.rs`, `docs/propuestas/README.md`, `scripts/actualizar_indice_propuestas.py`, `.agents/skills/validar_docs.py` (excluye `propuestas/`), herramienta `subagent` de esta sesión.
- Intención: que una sesión abierta en DSH pueda delegar en Test Runner, APK Builder, Telemetry Analyzer y Label Auditor sin inventarse el encargo ni pedirle al usuario que lo copie a mano.
- Restricciones y decisiones previas:
  - Los cuatro roles, sus disparadores y `flash_lite` no cambian (origen: `AGENTS.md` §4 y respuesta humana, 2026-10-25) → implica: la tabla de §4 se conserva literal y los archivos nuevos no sustituyen a `invoke_subagent`, lo acompañan → protegida por: "La tabla de disparadores no cambia" y "El guardián conserva el mensaje de Antigravity".
  - Todo mensaje de bloqueo debe seguir nombrando a Antigravity (origen: respuesta humana, 2026-10-25: "conservando los originales") → implica: cada mensaje de `pre_tool_guard.py` conserva su línea actual y añade una segunda → protegida por: "El guardián conserva el mensaje de Antigravity".
  - En DSH la herramienta `subagent` solo acepta `description` y `prompt`; no hay parámetro de rol ni de modelo (origen: esquema de la herramienta en esta sesión) → implica: el rol viaja dentro del texto del encargo y `flash_lite` queda como referencia histórica, no como parámetro → protegida por: "El rol viaja en el encargo" (no testeable: que el modelo lo respete).
  - `docs/propuestas/` está excluido de `validar_docs.py` (origen: `validar_docs.py`, línea 329) → implica: esta propuesta no necesita frontmatter Diátaxis.
- Alternativas descartadas:
  - Plugin nativo de DSH que registre agentes con nombre → descartada porque la API de plugins no está documentada en este equipo y el trabajo sale del repositorio (respuesta humana, 2026-10-25).
  - Sustituir `invoke_subagent` por la llamada de DSH → descartada porque perdería las sesiones que aún usan el arnés de Antigravity.
- Ante imprevistos, prioriza: no romper el arnés de Antigravity antes que dejar DSH perfectamente integrado.

### 3. Ejemplos

- Caso normal: quiero correr `cargo test` → antes: el mensaje del guardián ordena `invoke_subagent(TypeName='self', Role='Test Runner', Model='flash_lite', Prompt='...')`, que en DSH no existe → ahora: el mismo mensaje añade `subagent(description='Test Runner', prompt=<.agents/subagents/test-runner.md> + tarea)`.
- Caso límite: un rol sin archivo (Doc Manager, podado) → antes: no hay nada que leer → ahora: `.agents/subagents/README.md` dice explícitamente "Doc Manager: podado, sin archivo" y no se crea ninguno.

### 4. Pasos

1. Creo `.agents/subagents/` con el índice `README.md` y cinco archivos de encargo: `test-runner.md`, `apk-builder.md`, `telemetry-analyzer.md`, `label-auditor.md`, `code-scout.md`.
2. Cada archivo declara rol, modelo original (`flash_lite`), disparador, reglas de trabajo, formato de reporte de 3 a 5 líneas y un encargo de ejemplo.
3. En `AGENTS.md` añado la subsección `### 4.1 Invocación en DeepSeek Harness (DSH)` después de la nota de poda, y ajusto solo la frase de entrada de §4 para que remita a 4.1.
4. En `pre_tool_guard.py` añado una línea DSH a los cuatro mensajes de bloqueo, sin tocar los textos ni las expresiones regulares existentes.
5. En `session_init.py` añado una línea DSH al mensaje de inicio, sin quitar la línea de Antigravity.
6. Escribo `tests/subagentes_dsh_tests.rs` y regenero el índice con `python scripts/actualizar_indice_propuestas.py`.

### 5. Qué más podría verse afectado

- `.agents/hooks.json` no se toca: sigue apuntando a los mismos `.cmd`.
- `.agents/ui/dock_widget.html` y `agent_state.json` no leen los archivos nuevos; el dock solo muestra los roles que reporta `state_tracker.py`, que no cambia.
- `scripts/actualizar_indice_propuestas.py` reescribe `docs/propuestas/README.md` (búsqueda: `grep -rn "propuestas" scripts/actualizar_indice_propuestas.py`); lo ejecuto yo porque en DSH no corre el hook `post_tool_runner`.
- Ningún test existente lee `AGENTS.md` §4 (búsqueda: `grep -rn "AGENTS.md" tests/` devuelve solo lectura de rutas en otros tests), así que la tabla puede conservarse sin romper nada.

### 6. Suposiciones

- `cargo test --test subagentes_dsh_tests` compila y corre en Windows sin NDK, igual que `version_sync_tests.rs` → prototipar: no → si es falso: la comprobación pasa a un script Python y lo digo antes de editar.
- El encargo en texto basta para que un subagente de DSH se comporte como el rol → prototipar: no (probarlo exige lanzar un subagente real y juzgar su salida; no es barato ni determinista) → si es falso: el archivo de rol tendría que repetir las reglas en cada llamada, que es lo que ya hace.

### 7. Alcance

- Incluye: capa de archivos de rol, subsección 4.1 en `AGENTS.md`, línea DSH en los cuatro mensajes del guardián y en el mensaje de inicio, test de integración.
- No incluye: plugin de DSH, cambios en `state_tracker.py`, en el dock, en `hooks.json`, ni resucitar Doc Manager.

### 8. Criterios de aceptación (no se modifican sin aprobación)

Escenario: Cada rol conservado tiene su archivo de encargo
Caso de uso: una sesión en DSH necesita delegar una tarea pesada | Tipo: normal
  Dado que existe `.agents/subagents/`
  Cuando leo los cinco archivos de rol
  Entonces cada uno contiene su nombre de rol, la cadena `flash_lite` y la instrucción de reportar de 3 a 5 líneas
Test: `tests/subagentes_dsh_tests.rs::test_cada_rol_tiene_archivo_de_encargo` (se escribe tras la aprobación)
Estado previo: (se completa al escribir el test)

Escenario: El índice cubre exactamente los roles vivos
Caso de uso: alguien busca qué subagentes existen | Tipo: límite
  Dado que `.agents/subagents/README.md` es el índice
  Cuando comparo los enlaces a archivos de rol con los archivos presentes
  Entonces hay 5 enlaces y 5 archivos, coinciden uno a uno, y el índice dice que Doc Manager está podado y sin archivo
Test: `tests/subagentes_dsh_tests.rs::test_indice_cubre_los_roles_vivos` (se escribe tras la aprobación)
Estado previo: (se completa al escribir el test)

Escenario: El guardián conserva el mensaje de Antigravity y añade DSH
Caso de uso: el agente principal intenta `cargo test` y recibe un bloqueo | Tipo: regresión
  Dado que `pre_tool_guard.py` tiene los cuatro mensajes de bloqueo
  Cuando busco en el archivo cada mensaje
  Entonces cada uno contiene su línea original con `invoke_subagent(TypeName='self', Role='<rol>'` y una línea nueva con `subagent(` y la ruta `.agents/subagents/<rol-en-kebab>.md`
Test: `tests/subagentes_dsh_tests.rs::test_guardian_conserva_antigravity_y_anade_dsh` (se escribe tras la aprobación)
Estado previo: (se completa al escribir el test; debe pasar)

Escenario: La tabla de disparadores no cambia
Caso de uso: se consulta la matriz de roles en `AGENTS.md` | Tipo: regresión
  Dado que `AGENTS.md` tiene la tabla de §4
  Cuando busco las cuatro filas originales
  Entonces las cuatro aparecen literalmente, con `self`, `flash_lite` y su disparador, y existe la subsección `### 4.1 Invocación en DeepSeek Harness (DSH)`
Test: `tests/subagentes_dsh_tests.rs::test_tabla_de_disparadores_intacta` (se escribe tras la aprobación)
Estado previo: (se completa al escribir el test; debe pasar)

Nota: que un subagente de DSH respete de verdad el rol no se puede comprobar con código (lo decide el modelo); queda para la revisión humana.

### 9. Cómo lo comprobaré

- Cada rol conservado tiene su archivo de encargo → comando: `cargo test --test subagentes_dsh_tests::test_cada_rol_tiene_archivo_de_encargo` (delegado a un subagente Test Runner, según §4) → pasa si los 4 roles + Code Scout existen, nombran su rol, contienen `flash_lite` y "3 a 5 líneas" → no detecta si el encargo es bueno en la práctica.
- El índice cubre exactamente los roles vivos → comando: `cargo test --test subagentes_dsh_tests::test_indice_cubre_los_roles_vivos` → pasa si hay 5 enlaces y 5 archivos y el README menciona Doc Manager como podado → no detecta enlaces rotos por mayúsculas en otros sistemas operativos.
- El guardián conserva el mensaje de Antigravity y añade DSH → comando: `cargo test --test subagentes_dsh_tests::test_guardian_conserva_antigravity_y_anade_dsh` y, como regresión funcional, `python .agents/hooks/test_guard.py` → pasa si el test verde y `test_guard.py` imprime "TODOS LOS TESTS DE LOS HOOKS PASARON EXITOSAMENTE" → no detecta que Antigravity siga leyendo `hooks.json` (eso no se puede probar desde aquí).
- La tabla de disparadores no cambia → comando: `cargo test --test subagentes_dsh_tests::test_tabla_de_disparadores_intacta` → pasa si las cuatro filas están literales y existe 4.1 → no detecta si la redacción nueva de la frase de entrada confunde a un lector.
- Que el rol se respete en la práctica → revisión humana: lanzar un `subagent` con el encargo de `test-runner.md` y comprobar que el reporte tiene 3 a 5 líneas y nombra el comando ejecutado.

### 10. Terminado cuando

Se cumplen los cuatro criterios de aceptación y el índice de propuestas está regenerado.

### 11. Parar si

2 intentos fallidos seguidos con el mismo criterio.

### Correcciones previas a presentar

- Añadí el criterio "La tabla de disparadores no cambia": sin él, el cambio podía alterar la matriz sin que nada lo detectara.
- Quité la afirmación de que `flash_lite` se puede forzar por parámetro en DSH: la herramienta `subagent` no expone modelo, así que se conserva como referencia en el encargo.

---

## Resultado (lo completa el agente después de editar)

### Predicho vs. real

- Cada rol conservado tiene su archivo de encargo: coincide.
  Resultado real: `4 passed; 0 failed` en `cargo test --test subagentes_dsh_tests`.
  Método: el anunciado (ejecutado por el subagente Test Runner).
- El índice cubre exactamente los roles vivos: coincide.
  Resultado real: el test compara 5 enlaces con 5 archivos y Doc Manager queda fuera.
  Método: el anunciado.
- El guardián conserva el mensaje de Antigravity y añade DSH: coincide.
  Resultado real: test en verde y `python .agents/hooks/test_guard.py` imprime
  "TODOS LOS TESTS DE LOS HOOKS PASARON EXITOSAMENTE (100% OK)" (7 casos).
  Método: el anunciado.
- La tabla de disparadores no cambia: coincide. Las cuatro filas siguen literales y existe §4.1.
  Método: el anunciado.

### Contexto: ¿se respetó?

- Los cuatro roles, disparadores y `flash_lite` no cambian → respetada.
- Los mensajes conservan la invocación de Antigravity → respetada (lo comprueba el test 3).
- En DSH el rol viaja en el `prompt` y el modelo lo fija el perfil → respetada.
- ¿Se tocó algo que alguna restricción prohibía? no.
- Imprevistos que surgieron: el índice maestro no se escribe en `docs/propuestas/README.md` sino en
  `docs/BACKLOG.md`; `scripts/actualizar_indice_propuestas.py` actualizó el segundo. → criterio de
  prioridad aplicado: sí (no romper nada).
- ¿Reapareció alguna alternativa descartada? no.

### Archivos modificados

- `docs/propuestas/2026-10-25_subagentes-dsh-mecanismo.md` (nuevo)
- `.agents/subagents/README.md` y cinco encargos (nuevos)
- `AGENTS.md` (frase de entrada de §4 y subsección 4.1)
- `.agents/hooks/pre_tool_guard.py` (cuatro mensajes; no versionado, ver fuera de alcance)
- `.agents/hooks/session_init.py` (un mensaje; no versionado)
- `tests/subagentes_dsh_tests.rs` (nuevo)
- `docs/BACKLOG.md` (regenerado por el script canónico)

¿Coincide con lo anunciado? sí, más el índice regenerado que el paso 6 ya anunciaba.

### Discrepancias

- Anuncié el índice en `docs/propuestas/README.md` cuando el índice real está en `docs/BACKLOG.md`
  → información faltante (el README solo enlaza al otro archivo).

### Para la próxima vez

Leer a qué ruta escribe un script antes de nombrarla en la propuesta.

### Fuera de alcance (hallazgos que no se arreglaron)

- `.agents/hooks/` y `.agents/hooks.json` están en `.gitignore` (líneas 45-46): los cambios de los
  hooks son locales y no viajan en el repositorio. | impacto: medio

---

## Revisión humana

- Que un subagente de DSH respete el rol y el formato de reporte: ✔ | ✘ | ?
- Que añadir una segunda línea a los mensajes del guardián siga siendo legible en el arnés de Antigravity (no testeable): ✔ | ✘ | ?
- Detectado solo por el humano:
