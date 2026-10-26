# Propuesta: subagentes repartidos entre los dos Gemini lite

Fecha: 2026-10-25
Nivel: 2
Estado: verificada por agente

## Propuesta (la escribe el agente antes de editar)

### 1. Qué cambia

Cada subagente pasa a ejecutarse en uno de los dos modelos Gemini lite, repartidos para sumar sus cuotas diarias, en vez de heredar el modelo del hilo principal.

### 2. Contexto

- Contexto consultado: `.dsh/profiles/desktop/cordis.patch.yml`, bundle web del producto (entrada `subagent-model-selection-settings`), código y README de `@deepseek-ai/dsh-tool-subagent`, `AGENTS.md` §4 y §4.1, `.agents/subagents/*.md`, `.agents/hooks/pre_tool_guard.py`, `.agents/hooks/session_init.py`, `tests/subagentes_dsh_tests.rs` y una llamada real a los dos modelos.
- Intención: que delegar cueste lo mínimo y que las 500 peticiones diarias de cada modelo se usen como un total de 1000.
- Restricciones y decisiones previas:
  - Los dos modelos son `gemini-3.1-flash-lite` y `gemini-3.5-flash-lite`, con 500 peticiones diarias cada uno (origen: respuesta humana, 2026-10-25) → implica: cada rol declara uno de los dos y el reparto debe usar ambos → protegida por: "El reparto usa los dos modelos".
  - La clave de Gemini ya está registrada y los dos modelos responden (origen: medición propia, 2026-10-25: `agent` devolvió `ok` con ambos ids) → implica: no toco credenciales ni `apiKeyEnv` → no testeable.
  - DSH no tiene failover entre modelos (origen: búsqueda en el bundle, sin coincidencias de fallback por cuota) → implica: el reparto es manual y cambiar de modelo es editar el id en el encargo → no testeable.
  - Los roles y sus disparadores no cambian (origen: propuesta anterior, 2026-10-25) → implica: la matriz conserva sus cuatro filas y sus disparadores → protegida por: "La tabla conserva disparadores".
- Alternativas descartadas:
  - Un solo modelo para todos → descartada porque desaprovecha la mitad de la cuota diaria.
  - Repartir por llamada con `workflow` → descartada porque el arnés reserva esa herramienta para orquestaciones, y aquí existe un mecanismo nativo.
  - `agentOptions` global en el tool → descartada porque fija un solo modelo para todos los subagentes.
- Ante imprevistos, prioriza: no romper el arranque de DSH antes que repartir la cuota.

### 3. Ejemplos

- Caso normal: delegar `cargo test` al Test Runner → antes: su sesión dice `agentModel: deepseek-flash` → ahora: dice `agentProvider: google`, `agentModel: gemini-3.1-flash-lite`.
- Caso límite: se agota la cuota de 3.1 a media tarde → antes: no hay a quién recurrir sin reiniciar → ahora: el encargo del rol se cambia a `gemini-3.5-flash-lite` en una línea y sigue funcionando.

### 4. Pasos

1. Copio el perfil a `cordis.patch.yml.bak` antes de tocarlo.
2. En `.dsh/profiles/desktop/cordis.patch.yml` añado `modelSelectionSettings: true` a la entrada existente `tool-subagent` y configuro la entrada `subagent-model-selection-settings` con `enabled: true` y las dos rutas permitidas.
3. Reparto: Test Runner y Telemetry Analyzer → `gemini-3.1-flash-lite`; APK Builder, Label Auditor y Code Scout → `gemini-3.5-flash-lite`.
4. Cada encargo cambia `Modelo original: flash_lite` por `Modelo: google/<id>` y una línea de invocación con `provider` y `model`.
5. `.agents/subagents/README.md` y `AGENTS.md` §4/§4.1 muestran el modelo por rol; `session_init.py` deja de anunciar `flash_lite`.
6. Ajusto `tests/subagentes_dsh_tests.rs`: cada rol declara un modelo aprobado, el reparto usa ambos y la tabla conserva disparadores.
7. Regenero el índice de propuestas.

### 5. Qué más podría verse afectado

- `tool-subagent-fork` (herramienta `subagent_fork`) sigue sin selección de modelo: los hijos heredan el modelo del padre, como documenta la configuración base (búsqueda: entrada `tool-subagent-fork`).
- El agente principal no cambia de modelo: el ajuste solo afecta a hijos nuevos.
- Si el ajuste queda desactivado, el tool vuelve a su comportamiento actual sin romperse (código: `allowedModels === undefined` → sin parámetros de modelo).

### 6. Suposiciones

- El patch de perfil acepta una entrada nueva (`subagent-model-selection-settings`) además de sobrescribir `tool-subagent` → prototipar: sí (es la que puede impedir que DSH arranque) → si es falso: el arranque falla y se revierte desde la copia de seguridad.
- La política se graba al crear la sesión, así que el cambio no surte efecto en esta sesión ya abierta → prototipar: no → si es falso: los parámetros aparecen sin abrir sesión nueva.
- 500 peticiones diarias por modelo → prototipar: no (no hay forma de leer la cuota sin agotarla) → si es falso: el reparto sigue igual, solo cambia el total disponible.

### 7. Alcance

- Incluye: selección de modelo por subagente, reparto por rol, encargos, README, §4/§4.1, mensaje de inicio, tests e índice.
- No incluye: credenciales, failover automático, modelo del agente principal, `subagent_fork`.

### 8. Criterios de aceptación (no se modifican sin aprobación)

Escenario: Cada rol declara uno de los dos modelos
Caso de uso: alguien lee un encargo para saber en qué modelo corre | Tipo: normal
  Dado que existen los cinco encargos
  Cuando extraigo de cada uno su identificador `google/...`
  Entonces cada uno tiene exactamente uno y pertenece al conjunto de dos ids aprobados
Test: `tests/subagentes_dsh_tests.rs::test_cada_rol_declara_un_modelo_aprobado` (se escribe tras la aprobación)
Estado previo: (se completa al escribir el test)

Escenario: El reparto usa los dos modelos
Caso de uso: se quiere sumar la cuota diaria de ambos | Tipo: límite
  Dado que los cinco encargos declaran su modelo
  Cuando cuento cuántos roles hay por modelo
  Entonces aparecen los dos ids y ninguno se queda con los cinco roles
Test: `tests/subagentes_dsh_tests.rs::test_reparto_usa_los_dos_modelos` (se escribe tras la aprobación)
Estado previo: (se completa al escribir el test)

Escenario: La tabla conserva disparadores
Caso de uso: se revisa la matriz de roles | Tipo: regresión
  Dado que `AGENTS.md` §4 tiene las cuatro filas
  Cuando busco cada fila
  Entonces conserva su `Role`, su `self` y su texto de disparador, y su columna de modelo nombra uno de los dos ids
Test: `tests/subagentes_dsh_tests.rs::test_tabla_conserva_disparadores` (se escribe tras la aprobación)
Estado previo: (se completa al escribir el test; debe pasar)

Escenario: El guardián conserva Antigravity
Caso de uso: una sesión antigua de Antigravity sigue igual | Tipo: regresión
  Dado que `pre_tool_guard.py` tiene los cuatro mensajes
  Cuando busco `invoke_subagent(TypeName='self'`
  Entonces sigue en los cuatro y cada uno mantiene la línea de DSH
Test: `tests/subagentes_dsh_tests.rs::test_guardian_conserva_antigravity_y_anade_dsh` (existente)
Estado previo: (se completa al escribir el test; debe pasar)

Nota (visible): que DSH acepte la configuración y que una sesión nueva reciba los parámetros de modelo solo se comprueba arrancando y leyendo el descriptor de un subagente real.

### 9. Cómo lo comprobaré

- Cada rol declara uno de los dos modelos → `cargo test --test subagentes_dsh_tests::test_cada_rol_declara_un_modelo_aprobado` (delegado a Test Runner) → pasa si los cinco tienen un id del conjunto → no detecta si el arnés lo aplica.
- El reparto usa los dos modelos → mismo comando, test `test_reparto_usa_los_dos_modelos` → pasa si hay al menos un rol en cada modelo → no detecta desequilibrios de uso real.
- La tabla conserva disparadores → test `test_tabla_conserva_disparadores` → pasa si las cuatro filas mantienen rol, `self` y disparador → no detecta redacción confusa.
- Antigravity sigue igual → test existente ajustado → pasa si los cuatro mensajes conservan `invoke_subagent` → no detecta que Antigravity funcione.
- Que DSH cargue la configuración → revisión humana: reiniciar, abrir sesión nueva y lanzar un subagente; su descriptor debe decir `agentProvider: google` con el id del rol → no detecta cuota restante.

### 10. Terminado cuando

Se cumplen los cuatro criterios, el índice está regenerado y el usuario confirma que DSH arrancó con la configuración nueva.

### 11. Parar si

2 intentos fallidos seguidos con el mismo criterio, o DSH no arranca tras el cambio de perfil.

### Correcciones previas a presentar

- Descarté `workflow` como vía de reparto: el arnés lo reserva para orquestaciones y aquí existe un mecanismo nativo.
- Añadí la copia de seguridad del perfil como paso previo: es el único cambio que puede impedir arrancar.

---

## Resultado (lo completa el agente después de editar)

### Predicho vs. real

- Cada rol declara uno de los dos modelos: coincide. `5 passed; 0 failed` en `cargo test --test subagentes_dsh_tests`.
  Método: el anunciado (ejecutado por el subagente Test Runner).
- El reparto usa los dos modelos: coincide. Dos roles en 3.1 y tres en 3.5.
  Método: el anunciado.
- La tabla conserva disparadores: coincide. Las cuatro filas mantienen rol, `self` y disparador.
  Método: el anunciado.
- Que DSH cargue la configuración: no pudo comprobarse. El perfil se editó y se guardó copia
  (`cordis.patch.yml.bak`), pero el efecto solo se ve tras reiniciar y abrir una sesión nueva.
  Método: pendiente de la revisión humana.

### Contexto: ¿se respetó?

- Los roles y sus disparadores no cambian → respetada.
- No tocar credenciales ni `apiKeyEnv` → respetada: no se editó `.credentials.yaml`.
- Sin failover automático → respetada: el reparto es manual y está documentado.
- ¿Se tocó algo que alguna restricción prohibía? no.
- Imprevistos: ninguno.
- ¿Reapareció alguna alternativa descartada? no.

### Archivos modificados

- `docs/propuestas/2026-10-25_subagentes-modelo-gemini-lite.md` (reescrito)
- `.agents/subagents/README.md` y los cinco encargos
- `AGENTS.md` (§4 y §4.1)
- `.agents/hooks/session_init.py` (mensaje de inicio)
- `tests/subagentes_dsh_tests.rs`
- `docs/BACKLOG.md` (regenerado)
- `cordis.patch.yml` del perfil de DSH y su copia `.bak` (fuera del repositorio)

¿Coincide con lo anunciado? sí.

### Discrepancias

- Ninguna.

### Para la próxima vez

Cuando un cambio dependa de reiniciar la aplicación, dejarlo como último paso y avisar antes de
declararlo terminado.

### Fuera de alcance (hallazgos que no se arreglaron)

- Ninguno.

---

## Revisión humana

- Que 3.1 y 3.5 tengan 500 peticiones diarias cada uno: ✔ | ✘ | ?
- Que DSH arranque y una sesión nueva reciba `provider`/`model` en `subagent`: ✔ | ✘ | ?
- Que Antigravity siga aceptando sus mensajes (no testeable): ✔ | ✘ | ?
- Detectado solo por el humano:
