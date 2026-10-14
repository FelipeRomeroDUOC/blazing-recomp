# Propuesta: Saneamiento de la documentación y registro de los hallazgos sueltos

Fecha: 2026-10-14
Nivel: 2
Estado: pendiente

<!-- Estados: pendiente | aprobada | modificada | descartada |
     verificada por agente | revisada por humano | discrepada | cerrada -->

## Propuesta (la escribe el agente antes de editar)

### 1. Qué cambia

Quien lea el repositorio vuelve a encontrar el rastro: el tablero de tareas refleja lo que de verdad
se está haciendo, las decisiones de los últimos días están registradas, los documentos que afirmaban
cosas que el código ya no hace dejan de afirmarlas, y los hallazgos que hoy duermen al final de las
propuestas aparecen en un solo registro con su prioridad y su origen.

### 2. Contexto

El diagnóstico está hecho y medido sobre el árbol real (catastro del 2026-10-14, solo lectura):

- 56 propuestas; **25 creadas desde el 11-oct** y ninguna mencionada en [`docs/BACKLOG.md`](../BACKLOG.md),
  que sigue anunciando «Fase Actual: Fase 5 — Sistema de entrada y controles táctiles».
- **80 hallazgos** en secciones `Fuera de alcance`, sin índice, sin prioridad y sin cierre.
- **34 propuestas** con `Revisión humana` pendiente y ningún índice que las agrupe.
- El vocabulario de estados no está escrito en ningún sitio: 2 propuestas no tienen línea `Estado:`
  (usan `**Estado:** Implementada y Verificada`, formato antiguo), 1 usa `completada`, que no existe.
- `docs/explicacion/DECISIONES.md` tiene 36 ADR y **ninguno del 14-oct**, aunque ese día se cerraron
  tres líneas de investigación y se reabrió una.
- Contexto consultado: [`docs/BACKLOG.md`](../BACKLOG.md), [`docs/propuestas/README.md`](README.md),
  [`docs/explicacion/DECISIONES.md`](../explicacion/DECISIONES.md) (ADR 15, 26 y cola),
  [`docs/explicacion/arquitectura-port-android.md`](../explicacion/arquitectura-port-android.md),
  [`docs/explicacion/roadmap-y-versionado.md`](../explicacion/roadmap-y-versionado.md),
  [`docs/referencia/api-modulos-rust.md`](../referencia/api-modulos-rust.md), `src/platform.rs`,
  `src/recomp.rs`, `android/AndroidManifest.xml`, `Cargo.toml`,
  `scripts/diagnostico/analizar_telemetria.py`, `.agents/skills/diataxis-docs/scripts/validar_docs.py`
  y las 56 propuestas.
- Intención: que el estado del proyecto se pueda reconstruir leyendo el repositorio, sin depender de
  la memoria de una sesión de chat. El éxito es un BACKLOG que responde «qué falta» y «qué se sabe».
- Restricciones y decisiones previas (máx. 3):
  - `docs/BACKLOG.md` **no** es una página Diátaxis: el validador lo excluye a propósito
    (origen: `validar_docs.py`, `paginas_markdown`) → implica: no se le añade front-matter `tipo:` ni
    se le exigen las secciones de un cuadrante → protegida por: *El validador no gana páginas*.
  - Las propuestas son registro histórico, no documentación viva
    (origen: [`docs/propuestas/README.md`](README.md)) → implica: no se reescribe lo que midieron, solo
    se normaliza su metadato y se corrigen las afirmaciones que hoy son falsas → protegida por:
    *Todo hallazgo registrado tiene origen en una propuesta*.
  - Regla de portabilidad (origen: `AGENTS.md` §6) → implica: toda referencia nueva es relativa al
    repositorio → protegida por: la auditoría de rutas del apartado 9.
- Alternativas descartadas (solo si las hubo):
  - Borrar las secciones `Fuera de alcance` y consolidarlas solo en el BACKLOG → descartada: el
    hallazgo debe seguir junto a la medición que lo produjo; el registro **enlaza**, no traslada.
  - Reescribir las 11 propuestas de formato antiguo para que sigan la plantilla actual → descartada:
    son registro histórico y reescribirlas destruiría evidencia para ganar homogeneidad cosmética.
  - Crear un `docs/HALLAZGOS.md` nuevo → descartada: el BACKLOG ya es el documento de trabajo del
    proyecto y está enlazado desde [`roadmap-y-versionado.md`](../explicacion/roadmap-y-versionado.md);
    un cuarto documento suelto es justo lo que ha hecho perder el rastro.
  - Añadir un validador nuevo de documentación contra código → descartada por ahora: primero hay que
    arreglar lo que está mal; el guardián que sí se añade es el de estados (apartado 4, paso 6).
- Ante imprevistos, prioriza: que un documento no afirme algo falso por encima de que esté bonito.

### 3. Ejemplos

- **Caso normal — el tablero.** En [`docs/BACKLOG.md`](../BACKLOG.md):
  antes: `Fase Actual: Fase 5` y una lista de tareas con un único bullet vivo de la tablet.
  ahora: el estado real (versión `0.0.2`, según `Cargo.toml` y el `AndroidManifest`) con las fases del
  [`roadmap`](../explicacion/roadmap-y-versionado.md), el índice de las 56 propuestas agrupadas por
  estado y el registro de hallazgos con prioridad y origen.
- **Caso normal — el registro.** Un hallazgo que hoy solo existe dentro de una propuesta:
  antes: en [`2026-10-12_ppu-60fps-estable.md`](2026-10-12_ppu-60fps-estable.md), «6,00 ms de despacho y
  lógica de juego por fotograma… está sin tocar y ya es el bloque más alto | impacto: alto».
  ahora: la misma frase en el registro del BACKLOG, con su prioridad y su enlace a esa propuesta.
- **Caso límite — la afirmación falsa.** En [`DECISIONES.md`](../explicacion/DECISIONES.md), ADR 26:
  antes: «Cada 5 segundos, el hilo principal extrae las direcciones más ejecutadas y emite una línea
  `# hotspots top=...`» y «filtrando exclusivamente direcciones del cartucho».
  ahora: la cadencia real (cada 2 s: `report.second_index % 2 == 0`, `src/platform.rs:2119`), el
  filtro real (ROM, IWRAM, EWRAM y BIOS: `record_hotspot`, `src/recomp.rs:365`) y la semántica real
  (consume **solo** el top y conserva el resto: `drain_top_hotspots`, `src/recomp.rs:382`).
- **Caso límite — el metadato.** En las dos propuestas de `2026-10-11_cosecha-*`:
  antes: `**Estado:** Implementada y Verificada`.
  ahora: `Estado: verificada por agente` (línea que los demás documentos y scripts ya saben leer).

### 4. Pasos

1. Reescribir [`docs/BACKLOG.md`](../BACKLOG.md): estado real, fases alineadas al roadmap, índice de
   las 56 propuestas por estado, y registro de los 80 hallazgos agrupados por tema con `impacto:` y
   enlace a su propuesta de origen. Conservar la historia completada, condensada.
2. Índice de propuestas con `Revisión humana` pendiente (las 34), marcando las que exigen el televisor.
3. Añadir a [`DECISIONES.md`](../explicacion/DECISIONES.md) los ADR del 14-oct: los registros del juego
   en memoria (discrepada), la frontera entre bloques descartada, el reloj por instrucción descartado y
   la reapertura de la calibración de presentación.
4. Corregir las afirmaciones que el código contradice: ADR 15 (tamaño del APK, que además contradice al
   ADR 32), ADR 26 (cadencia, filtro y semántica del volcado de hotspots) y
   [`arquitectura-port-android.md`](../explicacion/arquitectura-port-android.md) (latencia de swapchain
   declarada 1; real 2 desde el ADR 24, y ajustable en caliente).
5. Normalizar el metadato de las 2 propuestas de formato antiguo y de la que usa `completada`, y
   documentar el vocabulario de estados en [`docs/propuestas/README.md`](README.md).
6. Añadir a `tests/packaging_tests.rs` un guardián que recorra `docs/propuestas/*.md` y exija, en cada
   propuesta, una línea `Estado:` con un valor del vocabulario (mismo patrón que
   `test_decisiones_de_presentacion_declaradas`, que lee el fuente del port).
7. Hacer que `scripts/diagnostico/analizar_telemetria.py` lea la línea `# ppucamino`, anotado como
   pendiente en tres propuestas distintas desde el 13-oct.
8. Corregir `AGENTS.md` §5 para que describa los cuadrantes que existen (hoy declara
   `docs/tutoriales/`, que no está, y no menciona `docs/prototipos/`).

### 5. Qué más podría verse afectado

Búsquedas hechas: `grep -rn "propuestas\|docs/" tests/ src/*_tests.rs scripts/extraer_referencias.py`,
`grep -rn "BACKLOG" docs/ src/ scripts/ .agents/` y el catastro completo sobre `docs/`.

- `tests/packaging_tests.rs` — contiene la prueba que sincroniza
  [`api-modulos-rust.md`](../referencia/api-modulos-rust.md) con `src/`. El paso 6 añade otra prueba
  ahí mismo; no se toca la existente.
- `docs/explicacion/roadmap-y-versionado.md` — enlaza al BACKLOG como «Backlog priorizado de tareas»;
  el enlace sigue siendo válido, cambia el contenido del destino.
- `.agents/skills/diataxis-docs/scripts/validar_docs.py` — **no se toca**; sus 10 páginas deben seguir
  en verde. El BACKLOG sigue fuera de su alcance, a propósito.
- `scripts/diagnostico/analizar_telemetria.py` — el paso 7 cambia su salida (aparece la sección de
  caminos de composición); su uso está documentado en
  [`scripts/diagnostico/README.md`](../../scripts/diagnostico/README.md), que hay que sincronizar.
- `AGENTS.md` — el paso 8 toca solo §5. El checklist de §6 no se toca.
- Los 80 hallazgos no se arreglan: solo se registran.
- Sin más referencias encontradas.

### 6. Suposiciones

- **Los 80 hallazgos del catastro están completos.** → prototipar: no (puede que alguna propuesta use
  otro encabezado para la misma sección) → si es falso: el registro crece al detectarse, no invalida
  nada de lo escrito. → no detecta: hallazgos que solo existen en el chat y nunca se anotaron.
- **`second_index % 2 == 0` significa «cada 2 segundos».** → prototipar: sí, leyendo qué cadencia
  produce `PerfReport::tick_at` (el mismo `second_index` que gobierna las ventanas `t=` de la
  telemetría) → si es falso: la corrección del ADR 26 sería otra, y se escribe la que el código diga.
  → resultado del prototipo: [una línea; se completa tras la aprobación]
- **La corrección del ADR 15 no necesita una medición nueva.** → prototipar: no: no hay ningún APK
  construido en el árbol y el paso no va a fabricar uno (minutos de compilación por ABI) → si es falso:
  se registra el dato medido del 12-oct (43,9 MB comprimido) y se cita, sin afirmar un tamaño actual.

### 7. Alcance

- **Incluye:**
  - El BACKLOG como tablero vivo y el registro de los 80 hallazgos con su prioridad y su origen.
  - `scripts/auditar_hallazgos.py`, la comprobación de que el registro y las propuestas coinciden.
  - Los ADR del 14-oct y la corrección de las afirmaciones que el código contradice.
  - El vocabulario de estados documentado, el metadato normalizado y su guardián en `tests/`.
  - `# ppucamino` en el analizador y la sincronización de `AGENTS.md` §5.
- **No incluye:**
  - Arreglar los 80 hallazgos: se registran, no se resuelven.
  - Reescribir las 11 propuestas de formato antiguo ni el contenido de ninguna propuesta.
  - Tocar `src/` (salvo el guardián de `tests/`), el motor, el APK o la telemetría.
  - Commitear: este cambio deja el árbol con más archivos modificados, y commitear es una decisión aparte.

### 8. Criterios de aceptación (no se modifican sin aprobación)

**Escenario: Todo hallazgo registrado tiene origen en una propuesta**
Caso de uso: quien busca qué falta | Tipo: normal
  Dado el catastro de las secciones `Fuera de alcance` de las 56 propuestas
  Cuando se compara con el registro del BACKLOG
  Entonces cada línea del registro enlaza a una propuesta existente y el registro cubre los 80
  hallazgos (o más, si el catastro se quedó corto)
Test: `python scripts/auditar_hallazgos.py` (script de comprobación, junto a `extraer_referencias.py`)
Estado previo: **no existe el registro** (hoy no hay ninguno)

**Escenario: Ninguna afirmación corregida la contradice el código**
Caso de uso: quien lee las decisiones | Tipo: regresión
  Dado que `src/platform.rs:300` fija `desired_maximum_frame_latency: 2` y `src/recomp.rs:365` acepta
  ROM, IWRAM, EWRAM y BIOS
  Cuando se leen el ADR 26 y `arquitectura-port-android.md`
  Entonces ninguno afirma latencia 1, cadencia de 5 s ni filtro exclusivo de ROM
Test: los comandos del apartado 9
Estado previo: **falla** (hoy los tres afirman lo contrario)

**Escenario: Toda propuesta declara un estado del vocabulario**
Caso de uso: quien recorre el histórico | Tipo: normal
  Dado el árbol de `docs/propuestas/`
  Cuando se ejecuta la suite del port
  Entonces cada propuesta tiene una línea `Estado:` y su valor está en el vocabulario, y el test falla
  si alguien añade una propuesta sin estado o con un valor inventado
Test: `tests/packaging_tests.rs::test_propuestas_declaran_estado_valido`
Estado previo: **no existe el test**; hoy 3 propuestas no lo cumplen

**Escenario: El validador no gana páginas**
Caso de uso: la documentación Diátaxis sigue siendo la misma | Tipo: regresión
  Dado `docs/` con el BACKLOG reescrito
  Cuando se ejecuta el validador
  Entonces sigue revisando 10 páginas con 0 errores y 0 avisos (el BACKLOG sigue excluido a propósito)
Test: `python .agents/skills/diataxis-docs/scripts/validar_docs.py .`
Estado previo: **pasa** (es el guardián, debe seguir pasando)

### 9. Cómo lo comprobaré

- **Todo hallazgo registrado tiene origen en una propuesta** → comando:
  `python scripts/auditar_hallazgos.py` (compara las secciones `Fuera de alcance` de las propuestas con
  el registro del BACKLOG) → pasa si: los conjuntos coinciden y todo enlace del registro resuelve a un
  archivo existente → no detecta: hallazgos que nunca se anotaron en ninguna propuesta.
- **Ninguna afirmación corregida la contradice el código** → comandos:
  `grep -n "desired_maximum_frame_latency" src/platform.rs` (debe dar 2 en la construcción),
  `grep -n "segundos" docs/explicacion/DECISIONES.md` (el ADR 26 no debe decir 5),
  `grep -n "record_hotspot" -A 4 src/recomp.rs` (el filtro real) → pasa si: los tres documentos dicen
  lo que dice el código → no detecta: afirmaciones falsas que el catastro no listó.
- **Toda propuesta declara un estado del vocabulario** → comando:
  `cargo test --test packaging_tests test_propuestas_declaran_estado_valido`
  → pasa si: el test está en verde y muere al quitar el `Estado:` de una propuesta
  → no detecta: que el estado declarado sea *cierto* (un `verificada` falso no se puede detectar aquí).
- **El validador no gana páginas** → comando:
  `python .agents/skills/diataxis-docs/scripts/validar_docs.py .`
  → pasa si: `10 página(s) revisada(s), 0 error(es), 0 aviso(s)` y código de salida 0
  → no detecta: la calidad de lo escrito en el BACKLOG, que no pasa por el validador.
- **Nada más se rompe** → comando: `cargo test` (los 128) y
  `python scripts/extraer_referencias.py --check`, `python scripts/diagnostico/auditar_labels.py`
  → pasa si: 128 en verde, `--check` en 0 y la auditoría de labels en 0
  → no detecta: que el árbol siga sin commitear, que es el estado de partida.
- **Portabilidad** → comando:
  `grep -rIn -F -e 'C:\' -e 'C:/' -e 'file:///' -e '<nombre-de-usuario>' AGENTS.md docs/ src/ tests/ scripts/`
  → pasa si: las únicas coincidencias son el texto de la propia regla y los comandos de auditoría
  → no detecta: rutas de otros equipos con otro formato.
- **El analizador lee `# ppucamino`** → comando:
  `python scripts/diagnostico/analizar_telemetria.py --archivo <log con ppucamino> --top 5`
  → pasa si: la línea aparece desglosada → no detecta: que las etiquetas de camino sean correctas.

**Blindaje por mutación (fase 4 del pipeline, previsto):**
M1 — quitar la línea `Estado:` de una propuesta → debe morir *Toda propuesta declara un estado del
vocabulario*.
M2 — poner un estado inventado (`en curso`) en una propuesta → debe morir el mismo escenario.
M3 — devolver el ADR 26 a «cada 5 segundos» → debe morir *Ninguna afirmación corregida la contradice
el código*.
Los archivos se respaldan en memoria RAM y se restauran en un bloque `finally`, sin comandos de Git.

### 10. Terminado cuando

Los cuatro escenarios pasan, la suite sigue en 128, el validador sigue en 0 errores con 10 páginas y
la auditoría de rutas sigue limpia. El BACKLOG responde «qué falta» y «qué se sabe», y cada hallazgo
tiene su origen.

### 11. Parar si

- 2 intentos fallidos seguidos con el mismo criterio: detenerse, explicar qué se probó y marcar la
  propuesta como `discrepada`.
- El registro del BACKLOG crece hasta volverse ilegible (más de ~120 líneas de hallazgos): detenerse y
  consultar si conviene moverlo a un documento propio, en vez de decidirlo solo.
- Aparece una contradicción entre dos documentos que no se pueda resolver leyendo el código: se anota
  como discrepancia y no se toca ninguno de los dos.

### Correcciones previas a presentar

- **Corregido tras verificar contra el código (dos notas del catastro inicial eran falsas):**
  1. Decía que `docs/BACKLOG.md` «queda fuera del validador por no tener front-matter». Es **falso**:
     el validador lo excluye por nombre, a propósito (`if ruta.name.lower() == "backlog.md": continue`).
     Por eso esta propuesta **no** le añade front-matter.
  2. Decía que [`api-modulos-rust.md`](../referencia/api-modulos-rust.md) no incluía `src/present.rs`.
     Es **falso**: lo menciona 10 veces y `present.rs` tiene 58 líneas de comentario de documentación.
     La nota venía de una propuesta del 10-oct y está caduca; no se toca el documento.
- **Corregido también:** el catastro contaba «2 propuestas sin estado»; verificado, no les falta el
  estado, usan el formato antiguo `**Estado:** Implementada y Verificada`. El problema real es de
  formato y de vocabulario, y así entra en el apartado 4 (paso 5).
- **Y una tercera:** el prototipo `docs/prototipos/layout_tactil_v2.html` parecía huérfano; está
  referenciado por 5 propuestas y por `src/input.rs`, `src/overlay.rs` y `src/overlay.wgsl`. No se toca.

### Cambios tras prototipos (solo si el plan cambió)

- Ninguno todavía: los prototipos se ejecutan tras la aprobación.

---

## Resultado (lo completa el agente después de editar)

_(pendiente de aprobación)_

---

## Revisión humana
<!-- El agente SÍ edita esta sección: escribe aquí los escenarios que no se
     pudieron comprobar con código ("(visible)" o "no pudo comprobarse") y las restricciones
     marcadas "no testeable",
     uno por línea, dejando las marcas ✔ | ✘ | ? sin elegir.
     El agente NO elige marca, NO escribe la nota y NO rellena
     "Detectado solo por el humano": eso es solo de la persona. -->

- Escenario «(visible)»: el BACKLOG reescrito se lee como un tablero útil y no como un volcado de
  datos — solo lo puede juzgar una persona: ✔ | ✘ | ?
- Escenario «(visible)»: el registro de hallazgos prioriza lo que tú considerarías prioritario — el
  orden lo puse yo y no lo prueba ningún test: ✔ | ✘ | ?
- Detectado solo por el humano: [lo completa la persona]
