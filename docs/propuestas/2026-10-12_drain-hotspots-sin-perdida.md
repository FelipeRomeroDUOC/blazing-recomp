# Propuesta: Volcado de hotspots sin pérdida y desglose regional del intérprete

Fecha: 2026-10-12
Nivel: 2
Estado: verificada por agente

<!-- Estados: pendiente | aprobada | modificada | descartada |
     verificada por agente | revisada por humano | discrepada | cerrada -->

> **Modificación solicitada por el humano en la aprobación.** La propuesta original
> (solo el volcado sin pérdida) se amplió con la opción elegida textualmente:
> "Aprobar y además agregar resumen por región (ROM/IWRAM/EWRAM/BIOS) para medir la
> cobertura AOT directamente; toca también platform.rs y el analizador". El contenido
> de los apartados 1 a 10 es el aprobado, con esa ampliación incorporada.

## Propuesta (la escribe el agente antes de editar)

### 1. Qué cambia

Dos cosas, ambas sobre la telemetría que escribe `blazing-perf.txt` en Android:

1. Cada 2 segundos se escribe una línea `# hotspots top=...` con las 6 direcciones de código
   más ejecutadas en el intérprete. Hoy, al escribirla se **borran para siempre** los contadores
   de todas las demás direcciones de esa ventana. Con este cambio, las que no entran en el `top`
   conservan su contador y se reportan en algún volcado posterior.
2. Se añade una línea `# regiones` con el total ejecutado por región de memoria
   (ROM, IWRAM, EWRAM, BIOS), separando pasos de intérprete y bloques nativos AOT, para poder
   ver de un vistazo **en qué región se está gastando el tiempo el intérprete**.

### 2. Ejemplos

- **Caso normal** (dirección caliente + dirección fría en la misma ventana):
  `0x03002750` acumula 90,000 ticks y `0x0801A2B0` acumula 12 →
  antes: el volcado reporta las 6 calientes y **descarta los 12 ticks** de la fría;
  ahora: el volcado reporta las 6 calientes y la fría **sigue acumulando** hasta que
  un volcado posterior la incluya en su `top`.

- **Caso límite** (ventana con muchísimas direcciones distintas, cada una poco frecuente):
  medido en el log de Android TV aportado (`blazing-perf-tv-13.txt`, 545 ventanas):
  suma de `fps × interprete` = **552,518,990** pasos de intérprete estimados, frente a
  **126,098,474** ticks efectivamente reportados en las líneas `# hotspots top=`
  (22.8%). Es decir, **se descartó el 77.2%** de las ejecuciones contadas.
  Antes: ninguna dirección de ROM aparece nunca en el `top` (0 direcciones `0x08xxxxxx`
  en todo el log) y ese 77.2% se pierde;
  ahora: los 552,518,990 pasos terminan reportados antes o después.

- **Caso normal de la línea regional**: en el mismo log, 122,117,607 de los ticks reportados
  pertenecen al clúster IWRAM `0x0300274C–0x0300279C`, pero eso solo se deduce sumando a mano
  273 líneas. Con `# regiones`, la fila `iwram` mostraría ese total acumulado en una sola línea,
  junto a los bloques nativos ejecutados en esa misma región.

### 3. Pasos

1. `src/recomp.rs`:
   - `drain_top_hotspots` deja de vaciar el acumulador completo: ordena, recorta a `limit` y
     **elimina únicamente las direcciones que devuelve**.
   - Se añaden `Region` (rom/iwram/ewram/bios) y `RegionCounters` (pasos de intérprete y
     bloques nativos por región), más `region_counters()` para leerlos.
   - `record_hotspot` clasifica la región una sola vez y suma el paso correspondiente.
   - `run_frame_native` acumula los bloques nativos por región en un arreglo local y los vuelca
     **una vez por fotograma** (no por bloque), para no añadir sincronización al camino caliente.
2. `src/platform.rs`: tras la línea de hotspots, escribe la línea `# regiones` con los totales.
3. `src/recomp_tests.rs`: test nuevo del volcado sin pérdida, test nuevo de contadores por región,
   y ajuste del test existente para pedir un vaciado explícito con `usize::MAX`.
4. `src/runner_tests.rs`: el test que inyecta una tabla vacía y ejecuta un fotograma completo
   pasa a tomar el mismo candado de tests (ver Suposiciones), porque escribe en los mismos
   acumuladores globales.
5. `scripts/diagnostico/analizar_telemetria.py`: parsea la línea `# regiones` y añade una sección
   con el desglose regional.
6. `docs/referencia/api-modulos-rust.md`: se regenera con `scripts/extraer_referencias.py`.
7. `docs/guias/medir-rendimiento.md`: se documentan las líneas `# hotspots top=` (que hoy no está
   documentada) y `# regiones`.

### 4. Qué más podría verse afectado

Búsqueda hecha: `grep -rn "drain_top_hotspots\|record_hotspot\|hotspots top=" src/ scripts/ docs/`

- [`src/platform.rs:2114`](../../src/platform.rs) — único llamador de producción del volcado.
- [`src/runner_tests.rs:95`](../../src/runner_tests.rs) — inyecta `BlockTable::empty()` y ejecuta
  un fotograma completo por el intérprete; **escribe en los acumuladores globales**.
- [`scripts/diagnostico/analizar_telemetria.py:40`](../../scripts/diagnostico/analizar_telemetria.py) —
  parsea `# hotspots top=`. El formato de esa línea no cambia.
- [`docs/referencia/api-modulos-rust.md:919`](../../docs/referencia/api-modulos-rust.md) — se genera
  desde los comentarios `///`; hay que regenerarlo o `--check` falla.
- Sin más referencias encontradas.

### 5. Suposiciones

- **El acumulador puede crecer hasta el número de direcciones distintas ejecutadas** (práctico: decenas
  de miles de entradas, del orden de 1–3 MB). Se da por cierto que en Android TV con 2 GB es tolerable.
  → Si es falso: acotar el mapa (descartar conteos 1 o fijar un máximo) en una propuesta aparte.
- **Nadie depende de que el acumulador quede vacío tras el volcado.** Comprobado con el `grep` del punto 4.
- **Los tests comparten los acumuladores globales y `cargo test` los ejecuta en paralelo.** Por eso se
  añade un candado de tests (`#[cfg(test)]`) que toman todos los tests que pueden escribir en ellos:
  los tres de `src/recomp_tests.rs` que ejecutan `run_frame_native` o `record_hotspot`, los dos tests
  nuevos, y el test de inyección de `src/runner_tests.rs`. Sin el candado, las aserciones por diferencia
  (`antes` → `después`) serían intermitentes.
  → Si es falso (no hubiera concurrencia), el candado no hace daño: solo serializa tests.
- **`nativos` por región cuenta bloques, no instrucciones**, mientras que `interprete` cuenta pasos de
  instrucción. Dividir ambos da una fracción de *eventos de despacho*, no de instrucciones. Se documenta
  como límite explícito en la guía y en la salida del analizador; no se presenta como "% de cobertura".
- **El coste de medir los bloques nativos por región es despreciable**: un arreglo local y una suma por
  bloque, más una toma del candado por fotograma. No se usa `Instant::now()` en el camino caliente.

### 6. Alcance

- **Incluye:**
  - Volcado sin pérdida en `src/recomp.rs` + contadores por región + lectura `region_counters()`.
  - Línea `# regiones` en `src/platform.rs`.
  - Tests nuevos y candado de tests (`src/recomp_tests.rs`, `src/runner_tests.rs`).
  - Parseo y sección regional en `scripts/diagnostico/analizar_telemetria.py`.
  - Regeneración de `docs/referencia/api-modulos-rust.md` y documentación en `docs/guias/medir-rendimiento.md`.
- **No incluye:**
  - Cambiar el límite de 6 direcciones ni la cadencia de 2 s.
  - Acotar el tamaño del acumulador.
  - Contar instrucciones nativas (solo bloques), que es lo que permitiría un "% de cobertura" exacto.
  - Corregir las imprecisiones de [`docs/explicacion/DECISIONES.md:502`](../../docs/explicacion/DECISIONES.md)
    y `:503` (ver "Fuera de alcance").

### 7. Criterios de aceptación (no se modifican sin aprobación)

**Escenario: El volcado conserva las direcciones que no entran en el top**
  Dado que el acumulador fue vaciado por completo y se registran 4 direcciones ejecutables con
  frecuencias 4.000, 3.000, 2.000 y 1.000 (todas distintas, para que el orden no dependa del desempate)
  Cuando se llama a `drain_top_hotspots(2)` y después a `drain_top_hotspots(2)`
  Entonces el primer volcado devuelve exactamente las 2 de mayor frecuencia y el segundo devuelve
  exactamente las 2 restantes con sus conteos intactos (2.000 y 1.000)
Test: `src/recomp_tests.rs::test_drain_conserva_la_cola_no_reportada` | Estado previo: **falla**

**Escenario (caso límite): Un límite de cero no consume nada**
  Dado el acumulador vaciado por completo y una dirección registrada 5.000 veces
  Cuando se llama a `drain_top_hotspots(0)` y después a `drain_top_hotspots(1)`
  Entonces el primer volcado devuelve una lista vacía y el segundo devuelve esa dirección con conteo 5.000
Test: `src/recomp_tests.rs::test_drain_con_limite_cero_no_consume_nada` | Estado previo: **falla**

**Escenario: La ejecución se clasifica por región y se formatea la línea de telemetría**
  Dado el acumulador vaciado por completo
  Cuando se registran 7 pasos en `0x08016001` (ROM), 5 en `0x03005000` (IWRAM), 3 en `0x02001000`
  (EWRAM), 2 en `0x00000100` (BIOS) y 1 en `0x04000000` (I/O, no ejecutable)
  Entonces los contadores de intérprete aumentan exactamente 7/5/3/2 por región, la dirección de I/O
  no suma en ninguna región, y `RegionCounters { interprete: [1,2,3,4], nativos: [10,20,0,40] }.to_line()`
  devuelve exactamente `# regiones rom=1/10 iwram=2/20 ewram=3/0 bios=4/40`
Test: `src/recomp_tests.rs::test_contadores_por_region_clasifican_y_formatean` | Estado previo: **falla**

**Escenario (no regresión): El top sigue ordenado y normaliza el bit Thumb**
  Dado el test existente con `0x08012000` (3 registros), `0x03001000` (2) y `0x00000020` (1)
  Cuando se llama a `drain_top_hotspots(3)`
  Entonces devuelve exactamente `[(0x08012000, 3), (0x03001000, 2), (0x00000020, 1)]`
Test: `src/recomp_tests.rs::test_hotspot_tracker_accumulation_and_drain` | Estado previo: pasa

> Nota sobre el test existente: sus dos aserciones finales
> (`assert!(drain_top_hotspots(10).is_empty())`, líneas 124 y 143) codifican el contrato antiguo
> de "descarte total". Con el contrato nuevo se sustituyen por vaciados explícitos con `usize::MAX`,
> que siguen exigiendo que un volcado total deje el acumulador vacío. El resto de aserciones queda intacto.

### 8. Cómo lo comprobaré

- **El volcado conserva las direcciones que no entran en el top** → comando:
  `cargo test recomp::tests::test_drain_conserva_la_cola_no_reportada`
  → pasa si: termina en `ok` (dos `assert_eq!` sobre vectores exactos)
  → no detecta: que la telemetría real de Android TV mejore; requiere un log nuevo del dispositivo.
- **Un límite de cero no consume nada** → comando: `cargo test recomp::tests::test_drain_con_limite_cero_no_consume_nada`
  → pasa si: termina en `ok`
  → no detecta: otros bordes no cubiertos (por ejemplo, un `limit` igual al número exacto de direcciones).
- **La ejecución se clasifica por región y se formatea la línea** → comando:
  `cargo test recomp::tests::test_contadores_por_region_clasifican_y_formatean`
  → pasa si: termina en `ok`, incluidas las dos cadenas exactas de `to_line()`
  → no detecta: que la línea llegue a escribirse en el archivo del dispositivo (eso lo cubre el analizador
  y, al final, un log real).
- **El top sigue ordenado y normaliza el bit Thumb** → comando:
  `cargo test recomp::tests::test_hotspot_tracker_accumulation_and_drain`
  → pasa si: termina en `ok`
  → no detecta: el desempate entre direcciones con el mismo conteo (`sort_unstable_by` no lo define).
- **El analizador lee la línea nueva** → comando:
  `python scripts/diagnostico/analizar_telemetria.py --archivo <log de prueba con una línea '# regiones'>`
  → pasa si: imprime la sección regional con los totales exactos de esa línea
  → no detecta: que el motor en Android escriba exactamente ese formato; eso se comprueba comparando
  el formato del test de `to_line()` con el documentado en la guía, no con un log real.
- **El analizador sigue funcionando con un log antiguo sin la línea nueva** → comando:
  `python scripts/diagnostico/analizar_telemetria.py --archivo "blazing-perf-tv-13.txt"`
  → pasa si: sale con código 0 y omite la sección regional en vez de fallar
  → no detecta: otros formatos antiguos distintos de ese archivo.
- **Nada más se rompe** → comando: `cargo test` (línea base medida hoy: 102 unitarios + 11 de integración,
  0 fallos) y `python scripts/extraer_referencias.py --check`
  → pasa si: 0 fallos y `OK: Referencia técnica … sincronizada` con código 0
  → no detecta: regresiones fuera de los tests ni el comportamiento en Android.

### 9. Terminado cuando

Se cumplen los cuatro criterios de aceptación, la suite completa pasa en verde y las tres comprobaciones
del analizador pasan.

### 10. Parar si

2 intentos fallidos seguidos con el mismo criterio. Detenerse, explicar qué se probó y marcar la
propuesta como `discrepada`.

### Correcciones previas a presentar

- Detectado al leer [`src/recomp_tests.rs:117-143`](../../src/recomp_tests.rs) que el test existente termina
  con `assert!(drain_top_hotspots(10).is_empty())`, que verifica el descarte total. Ese aserto tiene que
  cambiar por el contrato nuevo (se explica en el punto 7); no se debilita, se sustituye por `usize::MAX`.
- Detectado al leer `scripts/extraer_referencias.py` que `docs/referencia/api-modulos-rust.md` se genera
  desde los comentarios `///`, así que el cambio de comentario obliga a regenerarlo (línea base: `--check`
  sale con código 0). Se añade al alcance.
- Detectado que el vaciado previo del test (`drain_top_hotspots(100)`, línea 119) ya no garantiza un
  acumulador limpio cuando hay más de 100 direcciones; se cambia a `usize::MAX`.
- Detectado, buscando referencias con `grep`, que [`src/runner_tests.rs:95`](../../src/runner_tests.rs)
  inyecta `BlockTable::empty()` y ejecuta un fotograma completo por el intérprete, escribiendo en los
  mismos acumuladores globales desde otro módulo. Sin un candado compartido, las aserciones por diferencia
  del test regional serían intermitentes. Se añade al alcance.
- Detectadas y corregidas dos citas de línea erróneas antes de presentar: el volcado periódico está en
  [`src/platform.rs:2113`](../../src/platform.rs) (no 2112) y el test que registra hotspots en paralelo
  invoca `run_frame_native` en [`src/recomp_tests.rs:40`](../../src/recomp_tests.rs) (no 41). Comprobado con `grep -n`.

---

## Resultado (lo completa el agente después de editar)

### Predicho vs. real

- **El volcado conserva las direcciones que no entran en el top**: coincide.
  Resultado real: en rojo, antes del arreglo, el segundo volcado devolvía `[]` frente a
  `[(0x00000030, 2000), (0x08014000, 1000)]`; en verde, devuelve los dos pares esperados.
  Método: el anunciado (`cargo test --lib recomp::tests::test_drain_conserva_la_cola_no_reportada`).
- **Un límite de cero no consume nada**: coincide.
  Resultado real: en rojo, `left: []` frente a `right: [(0x08015000, 5000)]`; en verde, pasa.
  Método: el anunciado (`cargo test --lib recomp::tests::test_drain_con_limite_cero_no_consume_nada`).
- **La ejecución se clasifica por región y se formatea la línea**: coincide.
  Resultado real: en rojo por API inexistente (`cannot find function region_of`, `cannot find
  struct RegionCounters`, `cannot find function region_counters`); en verde, pasa con las dos
  cadenas exactas de `to_line()`.
  Método: el anunciado (`cargo test --lib recomp::tests::test_contadores_por_region_clasifican_y_formatean`).
- **(no regresión) El top sigue ordenado y normaliza el bit Thumb**: coincide. Verde antes y después,
  con las dos aserciones finales migradas a `usize::MAX`.
- **El analizador lee la línea nueva**: coincide. Con un log de prueba con dos líneas `# regiones`
  imprimió la sección regional tomando la última (`rom=1,500/700`), código de salida 0.
- **El analizador sigue funcionando con un log antiguo**: coincide. Con `blazing-perf-tv-13.txt` sale
  con código 0 y avisa "sin líneas `# regiones` en el log", omitiendo solo esa sección.
- **Nada más se rompe**: coincide. `cargo test` = 105 unitarios + 11 de integración, 0 fallos
  (116 en total, +3 nuevos); `extraer_referencias.py --check` código 0; `validar_docs.py` 0 errores
  y 0 avisos; `auditar_labels.py` código 0.
- **Blindaje por mutación** (no anunciado como criterio, añadido en la fase 4 del pipeline):
  M1 — el volcado vacía el mapa entero (`estado.map.clear()`) → mueren los 2 tests de volcado;
  M2 — `region_of` clasifica BIOS hasta `0x8000` → muere el test regional. `src/recomp.rs` restaurado
  byte a byte desde memoria RAM en un bloque `finally`, sin comandos Git destructivos.

### Archivos modificados

Míos: `src/recomp.rs`, `src/recomp_tests.rs`, `src/runner_tests.rs`, `src/platform.rs`,
`scripts/diagnostico/analizar_telemetria.py`, `docs/referencia/api-modulos-rust.md` (regenerado),
`docs/guias/medir-rendimiento.md` y este archivo.

¿Coincide con lo anunciado? sí: los siete archivos ya estaban listados en los apartados 3 y 6.
No se tocó `src/runner.rs` ni `src/pipeline_tests.rs`, que habrían sido necesarios con el diseño
alternativo de devolver el desglose regional desde `run_frame_native`.

Aviso de higiene: `src/recomp.rs`, `src/recomp_tests.rs`, `src/platform.rs`,
`scripts/diagnostico/analizar_telemetria.py` y `docs/referencia/api-modulos-rust.md` **ya tenían
cambios sin commitear** de otra sesión antes de empezar; el `git diff` de esos archivos mezcla
ambos trabajos. Mis cambios y los previos no se solapan: la regeneración de la referencia solo
sustituye la tabla del módulo y las dos entradas de `record_hotspot` y `drain_top_hotspots`.

### Discrepancias

- Al cambiar el tipo del estado global a `HotspotState`, `drain_top_hotspots` dejó de compilar
  (`no method named 'remove' found for &mut HotspotState`) y hubo que bindear el estado y usar
  `estado.map.remove(addr)`. Clasificación: error de implementación, detectado y corregido dentro
  del mismo ciclo, sin afectar a ningún criterio de aceptación.

### Para la próxima vez

- Antes de predecir el diff literal del cuerpo de una función, comprobar de qué tipo es el estado
  global que consume, no solo lo que devuelve.

### Fuera de alcance (hallazgos que no se arreglaron)

- [`docs/explicacion/DECISIONES.md:502`](../../docs/explicacion/DECISIONES.md) afirma que el acumulador filtra
  "exclusivamente direcciones del cartucho (ROM_REGION_START..ROM_REGION_END)", pero
  [`src/recomp.rs:201`](../../src/recomp.rs) incluye también IWRAM, EWRAM y BIOS. | impacto: bajo
- [`docs/explicacion/DECISIONES.md:503`](../../docs/explicacion/DECISIONES.md) afirma un volcado "cada 5
  segundos", pero [`src/platform.rs:2113`](../../src/platform.rs) vuelca cada 2 segundos. | impacto: bajo
- El orden entre direcciones con el mismo conteo no está definido (`sort_unstable_by`). | impacto: bajo
- `nativos` por región cuenta bloques y `interprete` cuenta instrucciones: no se puede derivar un porcentaje
  de cobertura exacto sin contar instrucciones nativas. | impacto: medio

---

## Revisión humana
<!-- El agente SÍ edita esta sección: escribe aquí los escenarios que no se
     pudieron comprobar con código ("(visible)" o "no pudo comprobarse"),
     uno por línea, dejando las marcas ✔ | ✘ | ? sin elegir.
     El agente NO elige marca, NO escribe la nota y NO rellena
     "Detectado solo por el humano": eso es solo de la persona. -->

- Utilidad real de la sección `# regiones` en un log nuevo de Android TV: ✔ | ✘ | ?
- Crecimiento de memoria del acumulador tras una sesión larga en Android TV (2 GB): ✔ | ✘ | ?
- Legibilidad de las líneas nuevas en [`docs/guias/medir-rendimiento.md`](../../docs/guias/medir-rendimiento.md): ✔ | ✘ | ?
- Detectado solo por el humano: [lo completa la persona]
