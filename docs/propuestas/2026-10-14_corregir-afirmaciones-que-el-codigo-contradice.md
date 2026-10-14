# Propuesta: Corregir las afirmaciones que el código contradice y reconstruir el CHANGELOG

Fecha: 2026-10-14
Nivel: 2
Estado: revisada por humano

<!-- Estados: pendiente | aprobada | modificada | descartada |
     verificada por agente | revisada por humano | discrepada | cerrada -->

## Propuesta (la escribe el agente antes de editar)

### 1. Qué cambia

Quien lea el repositorio deja de encontrarse frases que el código ya desmiente: el encabezado del crate
deja de decir que el juego no dibuja, la arquitectura deja de atribuir a la tabla de despacho un número
de entradas que no es el suyo, los conteos de tests dejan de estar un paso por detrás, y el registro de
cambios cubre por fin las tres jornadas que faltan.

### 2. Contexto

El catastro está hecho y cada cifra verificada contra el árbol real (solo lectura):

- **`src/lib.rs:18-20`** afirma *«La emulación aún no produce fotogramas… negro de panel»*. Es falso: el
  juego corre con la PPU a 60 FPS ([`docs/BACKLOG.md`](../BACKLOG.md) §5). La frase se **propaga** al
  documento generado [`api-modulos-rust.md:49`](../referencia/api-modulos-rust.md#L49) porque
  `extraer_referencias.py` copia el doc-comment.
- **`arquitectura-c-recompilado.md:38`** dice que `rcg_blocks[]` tiene *«las 17,207 entradas»*. El
  código dice otra cosa: `src-c/1575 - Fire Emblem (E)(Independent)-bios.11.c:813071` define
  `RCG_EXPORT const uint64_t rcg_block_count = 144875;`. La línea 26 del mismo documento mezcla
  «funciones» y «bloques» en *«17,207 funciones de bloques»*.
- **`AGENTS.md` §6 y `docs/BACKLOG.md:24`** citan **128 tests** (109+15+4). `cargo test` da hoy **129**
  (109 en los satélites de `src/`, **16** en [`tests/packaging_tests.rs`](../tests/packaging_tests.rs) y
  4 en [`tests/golden_frame_tests.rs`](../tests/golden_frame_tests.rs)).
- **`CHANGELOG.md`** está congelado el 11-oct (`git log -1 -- CHANGELOG.md` → `0d00e16`). Su sección
  `[Sin publicar]` no recoge la PPU por tramos, los fotogramas dorados, el techo del reloj ni el
  saneamiento del 11→14-oct, y afirma *«La base de etiquetas alcanza 53 funciones verificadas»* cuando
  [`blazing.labels.toml`](../../blazing.labels.toml) tiene **36.966 claves `name`/`address`** (≈18.483
  funciones).
- Contexto consultado: [`src/lib.rs`](../../src/lib.rs), `src-c/*.c`, `blazing.labels.toml`, `pack.toml`,
  [`docs/BACKLOG.md`](../BACKLOG.md), [`docs/explicacion/DECISIONES.md`](../explicacion/DECISIONES.md),
  [`arquitectura-c-recompilado.md`](../explicacion/arquitectura-c-recompilado.md),
  [`arquitectura-port-android.md`](../explicacion/arquitectura-port-android.md),
  [`analisis_rendimiento.md`](../explicacion/analisis_rendimiento.md), [`CHANGELOG.md`](../../CHANGELOG.md),
  [`README.md`](../../README.md), [`AGENTS.md`](../../AGENTS.md), `scripts/extraer_referencias.py` y
  `cargo test`.
- Intención: que ninguna página del repositorio pueda leerse como estado actual y estar mintiendo. El
  éxito es un lector que reconstruye el estado del proyecto sin tropezar con cifras caducas.
- Restricciones y decisiones previas (máx. 3):
  - Las propuestas son **registro histórico** (origen: [`docs/propuestas/README.md`](README.md)) → implica:
    no se reescribe ninguna propuesta, ni siquiera las que citan las cifras viejas → protegida por:
    *El guardián ignora el registro histórico*.
  - La referencia de API es **generada**, no se edita a mano (origen: `scripts/extraer_referencias.py`) →
    implica: el cambio real vive en el doc-comment de `src/lib.rs` y el `.md` se regenera → protegida
    por: *La referencia generada sigue sincronizada*.
  - Respuesta humana del 2026-10-14: el saneamiento v2 se limita a **afirmaciones falsas contra el
    código** y sí reconstruye el CHANGELOG → implica: los huecos de cobertura (perf/pipeline/present en
    la referencia, scripts sin documentar) **salen** de esta propuesta → protegida por: *Nada fuera del
    alcance se toca* (revisión humana).
- Alternativas descartadas (solo si las hubo):
  - Reescribir `analisis_rendimiento.md` → descartada: ya lleva un `[!WARNING]` que lo declara histórico
    y ofrece las cifras vigentes; sus números viejos están enmarcados, no sueltos.
  - Cambiar `arquitectura-port-android.md:40` y `DECISIONES.md:95` («17,207 funciones») → descartada:
    `17.207` es el número de **funciones** del comentario de `src/recomp.rs:4`; la afirmación falsa es
    sobre las **entradas de `rcg_blocks`**, no sobre las funciones.
  - Reescribir `README.md` raíz → pospuesta: describe el flujo viejo `recomp.exe play` y una carpeta
    `mods/` que no existe (verificado), pero rehacer la portada es una decisión de contenido aparte.
- Ante imprevistos, prioriza: que un documento no afirme algo falso por encima de que esté completo.

### 3. Ejemplos

- **Caso normal — el encabezado del crate.** En `src/lib.rs`:
  antes: *«La emulación aún no produce fotogramas: la presentación muestra el estado actual del búfer
  (negro de panel hasta conectar el bucle del motor `gba-core`).»*
  ahora: una frase que declara que el motor `gba-core` ejecuta el juego y compone cada fotograma, sin
  prometer una cifra de FPS que el documento no pueda sostener.
- **Caso normal — la tabla de despacho.** En `arquitectura-c-recompilado.md:38`:
  antes: *«el arreglo `rcg_blocks[]` con las 17,207 entradas y `rcg_block_count`»*
  ahora: *«el arreglo `rcg_blocks[]` con las 144.875 entradas (`rcg_block_count = 144875`)»*.
- **Caso límite — el guardián no debe morder el registro.** Una propuesta histórica (por ejemplo
  [`2026-10-12_ppu-60fps-estable.md`](2026-10-12_ppu-60fps-estable.md)) cita el número viejo dentro de su
  medición. El guardián **no** puede fallar por eso: solo mira los documentos de estado actual.

### 4. Pasos

1. Corregir el doc-comment de [`src/lib.rs`](../../src/lib.rs) (líneas 18-20) para que describa la
   emulación actual, y regenerar [`api-modulos-rust.md`](../referencia/api-modulos-rust.md) con
   `python scripts/extraer_referencias.py`.
2. Corregir las dos cifras de [`arquitectura-c-recompilado.md`](../explicacion/arquitectura-c-recompilado.md)
   (líneas 26 y 38) para separar «funciones del juego» de «entradas de `rcg_blocks[]`».
3. Actualizar el conteo de tests en [`AGENTS.md`](../../AGENTS.md) §6 y en
   [`docs/BACKLOG.md`](../BACKLOG.md) a 129, con su desglose real (109 + 16 + 4).
4. Reconstruir la sección `[Sin publicar]` de [`CHANGELOG.md`](../../CHANGELOG.md) con los hitos del
   11→14-oct (cobertura AOT, PPU por tramos y alpha real, fotogramas dorados, telemetría de regiones y
   `# ppucamino`, techo del reloj, saneamiento), retirando la cifra *«53 funciones»*.
5. Añadir a [`tests/packaging_tests.rs`](../tests/packaging_tests.rs) un guardián que recorra los
   documentos de estado actual y falle si reaparece una frase caduca, ignorando `docs/propuestas/`.

### 5. Qué más podría verse afectado

Búsquedas hechas: `grep -rn "17,207\|17.207\|aún no produce fotogramas\|128 tests" docs/ src/ AGENTS.md`
y el catastro sobre `docs/`.

- [`tests/packaging_tests.rs`](../tests/packaging_tests.rs) ya aloja `test_reference_docs_in_sync_with_code`
  y `test_decisiones_de_presentacion_declaradas`; el paso 5 añade otra prueba ahí, sin tocar las existentes.
- [`scripts/extraer_referencias.py`](../../scripts/extraer_referencias.py) — **no se toca**; sólo se
  ejecuta para regenerar el `.md`. Su `--check` debe seguir en 0.
- [`docs/explicacion/DECISIONES.md`](../explicacion/DECISIONES.md) — **no se toca**: sus menciones a
  `17,207` son de *funciones*, verificadas contra `src/recomp.rs:4`.
- [`README.md`](../../README.md) raíz y `docs/prototipos/` — fuera de alcance, sin cambios.
- `.agents/skills/diataxis-docs/scripts/validar_docs.py` — **no se toca**; sus 10 páginas deben seguir
  en verde y la lista no gana ni pierde páginas.
- `docs/BACKLOG.md` — el paso 3 sólo toca la línea del conteo de tests, no el registro de hallazgos.

### 6. Suposiciones

- **`144.875` es el conteo de bloques vigente del árbol.** → prototipar: sí, leyendo
  `src-c/…-bios.11.c:813071` y contando símbolos `b_…` únicos → si es falso: se escribe la cifra medida
  en su lugar → resultado del prototipo: [una línea; se completa tras la aprobación]
- **Regenerar la referencia sólo cambia el párrafo de `lib.rs`.** → prototipar: sí, ejecutando
  `extraer_referencias.py` y mirando el `git diff` del `.md` → si es falso: se revisa el diff completo
  antes de aceptarlo → resultado del prototipo: [una línea; se completa tras la aprobación]
- **En el 11→14-oct no hubo cambios de audio ni de mandos físicos que el CHANGELOG deba registrar.**
  → prototipar: no (los hitos salen de las 57 propuestas, no de una medición).

### 7. Alcance

- **Incluye:**
  - Las cuatro afirmaciones falsas verificadas: `src/lib.rs` (+ referencia generada),
    `arquitectura-c-recompilado.md`, el conteo de tests en `AGENTS.md`/`BACKLOG.md` y el `CHANGELOG.md`.
  - El guardián en `tests/` que impide que reaparezcan.
- **No incluye:**
  - Los huecos de cobertura (perf/pipeline/present/overlay_shader en la referencia y los 7 scripts sin
    documentar): van a otra propuesta.
  - `analisis_rendimiento.md`, `arquitectura-port-android.md` y `DECISIONES.md` (no contienen la
    afirmación falsa; se explica en el bloque Contexto).
  - Rehacer el `README.md` raíz ni `pack.toml` (versión `0.1.0` frente al `0.0.2` de la app: decisión aparte).

### 8. Criterios de aceptación (no se modifican sin aprobación)

**Escenario: Ningún documento de estado actual afirma algo que el código contradice**
Caso de uso: quien lee el repositorio para saber cómo está | Tipo: normal
  Dado `src/lib.rs`, `arquitectura-c-recompilado.md`, `AGENTS.md`, `docs/BACKLOG.md` y `CHANGELOG.md`
  Cuando se buscan las frases caducas («aún no produce fotogramas», «las 17,207 entradas», «128 tests»,
  «53 funciones verificadas»)
  Entonces ninguna aparece, y `arquitectura-c-recompilado.md` nombra `rcg_block_count = 144875`
Test: `tests/packaging_tests.rs::test_documentos_sin_afirmaciones_caducas`
Estado previo: **falla** (hoy las cinco frases están en su sitio)

**Escenario: El guardián ignora el registro histórico**
Caso de uso: quien consulta una medición antigua | Tipo: límite
  Dado `docs/propuestas/2026-10-12_ppu-60fps-estable.md`, que cita la cifra vieja dentro de su medición
  Cuando el guardián recorre los documentos
  Entonces no la lee y no falla; sólo mira las cinco rutas de estado actual
Test: `tests/packaging_tests.rs::test_guardian_ignora_el_registro_historico`
Estado previo: **pasa** (la lista de rutas no incluye `docs/propuestas/`); debe seguir pasando al añadir una propuesta que cite la cifra vieja

**Escenario: La referencia generada sigue sincronizada**
Caso de uso: quien consulta la API de los módulos | Tipo: regresión
  Dado `src/lib.rs` con el doc-comment corregido
  Cuando se ejecuta `python scripts/extraer_referencias.py --check`
  Entonces sale con código 0, porque el `.md` se regeneró
Test: `tests/packaging_tests.rs::test_reference_docs_in_sync_with_code` (existente)
Estado previo: **pasa** (es el guardián, debe seguir pasando)

**Escenario: El validador Diátaxis no gana ni pierde páginas**
Caso de uso: la documentación sigue siendo la misma | Tipo: regresión
  Dado `docs/` con los documentos corregidos
  Cuando se ejecuta el validador
  Entonces sigue revisando 10 páginas con 0 errores y 0 avisos
Test: `python .agents/skills/diataxis-docs/scripts/validar_docs.py .`
Estado previo: **pasa** (es el guardián, debe seguir pasando)

### 9. Cómo lo comprobaré

- **Ningún documento afirma algo contradicho** → comando:
  `cargo test --test packaging_tests test_documentos_sin_afirmaciones_caducas` → pasa si: verde, y muere
  al reponer la frase en cualquier documento de la lista → no detecta: afirmaciones falsas que el
  catastro no listó.
- **El guardián ignora el histórico** → comando: el mismo test con una propuesta que cite la cifra vieja
  → pasa si: sigue verde → no detecta: que el catastro de documentos actuales esté incompleto.
- **La referencia sigue sincronizada** → comando:
  `python scripts/extraer_referencias.py --check && echo OK` → pasa si: `OK` y código 0
  → no detecta: que el doc-comment nuevo describa bien el motor (sólo comprueba sincronía).
- **El validador no gana páginas** → comando:
  `python .agents/skills/diataxis-docs/scripts/validar_docs.py .` → pasa si:
  `10 página(s) revisada(s), 0 error(es), 0 aviso(s)` y código 0 → no detecta: la calidad del CHANGELOG.
- **Cifra de bloques** → comando:
  `grep -n "rcg_block_count" "src-c/1575 - Fire Emblem (E)(Independent)-bios.11.c"` → pasa si: imprime
  `= 144875` → no detecta: que `src-c/` corresponda a la ROM oficial (fuera de alcance).
- **Nada más se rompe** → comando: `cargo test` y `python scripts/auditar_hallazgos.py`
  → pasa si: 129 en verde y el registro de hallazgos sigue cubriendo los 81
  → no detecta: que el árbol siga sin commitear.
- **Portabilidad** → comando:
  `grep -rIn -F -e 'C:\' -e 'C:/' -e 'file:///' AGENTS.md docs/ src/ tests/ scripts/`
  → pasa si: las únicas coincidencias son el texto de la propia regla y las auditorías.

**Blindaje por mutación (fase 4 del pipeline, previsto):**
M1 — volver a poner «La emulación aún no produce fotogramas» en `src/lib.rs` → debe morir
*Ningún documento afirma algo que el código contradice*.
M2 — devolver «17,207 entradas» a `arquitectura-c-recompilado.md` → debe morir el mismo escenario.
M3 — reponer «128 tests» en `AGENTS.md` → debe morir el mismo escenario.
Los archivos se respaldan en memoria RAM y se restauran en un bloque `finally`, sin comandos de Git.

### 10. Terminado cuando

Los cuatro escenarios pasan, la suite queda en 129, el validador sigue en 0 errores con 10 páginas y
`extraer_referencias.py --check` sale en 0.

### 11. Parar si

- 2 intentos fallidos seguidos con el mismo criterio: detenerse, explicar qué se probó y marcar la
  propuesta como `discrepada`.
- La corrección de `arquitectura-c-recompilado.md` destapa que la tabla de archivos generados (tamaños y
  líneas por unidad de traducción) también es caduca: se anota como hallazgo y no se reescribe aquí.
- `extraer_referencias.py` produce un diff que toca mucho más que el párrafo de `lib.rs`: detenerse y
  consultar antes de aceptarlo.

### Correcciones previas a presentar

- **Tres notas del catastro inicial eran falsas y se corrigieron antes de presentar:**
  1. Se listaba `analisis_rendimiento.md` como documento que miente. Es **falso**: lleva un `[!WARNING]`
     que lo declara histórico y ofrece las cifras vigentes. No se toca.
  2. Se listaban `arquitectura-port-android.md` y `DECISIONES.md` por sus `17,207`. Es **falso** que
     mientan: `17.207` es el número de *funciones* (`src/recomp.rs:4`). La afirmación falsa es la de
     `arquitectura-c-recompilado.md` sobre las *entradas de `rcg_blocks[]`*, y sólo esa se corrige.
  3. Se titulaba «la emulación no produce fotogramas» como un problema de documentación; es un
     doc-comment de **código** en `src/lib.rs`, así que el cambio toca `src/` y obliga a regenerar la
     referencia. Queda reflejado como restricción.

### Cambios tras prototipos (solo si el plan cambió)

- Ninguno todavía: los prototipos se ejecutan tras la aprobación.

---

## Resultado (lo completa el agente después de editar)

### Predicho vs. real

- **Ningún documento afirma algo que el código contradice**: coincide
  Resultado real: las cuatro frases desaparecieron; `test_documentos_sin_afirmaciones_caducas` pasó de
  rojo (fallaba en `src/lib.rs`) a verde. Método: el anunciado.
- **El guardián ignora el registro histórico**: coincide
  Resultado real: verde antes y después; `test_guardian_ignora_el_registro_historico` cuenta ≥1 propuesta
  que cita una frase caduca (la de reactivar la calibración nombra «128 tests»), así que la exclusión
  de `docs/propuestas/` no es letra muerta. Método: el anunciado.
- **La referencia generada sigue sincronizada**: coincide
  Resultado real: `extraer_referencias.py --check` → 0. El diff de la regeneración tocó **solo** el
  párrafo de `lib.rs` (2 inserciones, 3 borrados), como predecía el prototipo. Método: el anunciado.
- **El validador Diátaxis no gana ni pierde páginas**: coincide
  Resultado real: `10 página(s) revisada(s), 0 error(es), 0 aviso(s)`, código 0. Método: el anunciado.

### Contexto: ¿se respetó?

- Propuestas como registro histórico → respetada: no se reescribió ninguna; el guardián las ignora.
- La referencia de API es generada, no se edita a mano → respetada: el cambio vive en el doc-comment de
  `src/lib.rs` y el `.md` se regeneró con el script.
- Respuesta humana del 2026-10-14 (solo afirmaciones falsas; sí al CHANGELOG) → respetada: los huecos de
  cobertura salieron a «Fuera de alcance».
- ¿Se tocó algo que alguna restricción prohibía? no.
- Imprevistos: el conteo de tests documentado no es 129 sino **131**, porque los dos guardianes nuevos
  también son tests. Criterio de prioridad aplicado: sí (se documentó el conteo real medido por
  `cargo test`, no el que predecía el paso 3).
- ¿Reapareció alguna alternativa descartada? no.

### Archivos modificados

`src/lib.rs`, `docs/referencia/api-modulos-rust.md` (regenerado), `docs/explicacion/arquitectura-c-recompilado.md`,
`AGENTS.md`, `docs/BACKLOG.md`, `CHANGELOG.md`, `tests/packaging_tests.rs`.
¿Coincide con lo anunciado? **no en el conteo**: el paso 3 anunciaba 129 y el valor real es 131
(109 + 18 + 4). El resto coincide.

### Discrepancias

- El paso 3 fijaba «129 tests» y el resultado es 131 → error de juicio: no conté que los dos tests nuevos
  de esta misma propuesta entran en el total. Se corrigió documentando el número medido.

### Para la próxima vez

Al actualizar un conteo de tests en un cambio que además añade tests, contar el estado final (después de
los tests nuevos), no el de partida.

### Fuera de alcance (hallazgos que no se arreglaron)

- `docs/explicacion/arquitectura-c-recompilado.md` conserva cifras caducas fuera de las dos líneas del
  alcance: «más de 115.000 instrucciones descubiertas» y una tabla de unidades de traducción que describe
  2-3 archivos cuando `src-c/` tiene 12. Corregirlo necesita una medición fresca de `src-c/` | impacto: medio
- El `README.md` raíz sigue describiendo el flujo `recomp.exe play` y una carpeta `mods/` que no existe
  (verificado con `ls`), y sólo menciona Android una vez | impacto: bajo
- `pack.toml` declara `version = "0.1.0"` mientras la app es `0.0.2` (`Cargo.toml`, `AndroidManifest.xml`) | impacto: bajo
- El número de tests en la documentación puede volver a quedar obsoleto en silencio: ningún test lo deriva
  del árbol, sólo se prohíben las frases viejas. Un guardián que cuente `#[test]` y lo compare sería la
  solución, pero excede los criterios de esta propuesta | impacto: bajo

---

## Revisión humana
<!-- El agente SÍ edita esta sección: escribe aquí los escenarios que no se
     pudieron comprobar con código ("(visible)" o "no pudo comprobarse") y las restricciones
     marcadas "no testeable",
     uno por línea, dejando las marcas ✔ | ✘ | ? sin elegir.
     El agente NO elige marca, NO escribe la nota y NO rellena
     "Detectado solo por el humano": eso es solo de la persona. -->

- Restricción «no testeable»: el alcance pactado deja fuera los huecos de cobertura (referencia sin
  `perf`, `present`, `pipeline`, `overlay_shader`, y 7 scripts sin documentar) → ¿es el recorte que
  querías?: ✔
- Escenario «(visible)»: el `CHANGELOG.md` reconstruido se lee como una historia útil de las tres
  jornadas y no como un volcado de bullets — sólo lo puede juzgar una persona: ✔
- Escenario «(visible)»: la portada `README.md` sigue describiendo un flujo antiguo y una carpeta `mods/`
  que no existe; se dejó fuera a propósito: ✔
- Detectado solo por el humano: [lo completa la persona]
