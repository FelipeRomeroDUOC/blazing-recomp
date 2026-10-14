# Propuesta: el alpha real de la PPU por tramos — componer sin buscar la segunda capa en cada píxel

Fecha: 2026-10-13
Nivel: 2
Estado: aprobada

<!-- Estados: pendiente | aprobada | modificada | descartada |
     verificada por agente | revisada por humano | discrepada | cerrada -->

## Propuesta (la escribe el agente antes de editar)

### 1. Qué cambia

Las escenas con desvanecidos y efectos alpha que llenan la pantalla —186 de los 353 segundos de la última
sesión, con las ~160 líneas de cada fotograma por el camino de mezcla y `comp` en 3,67 ms— vuelven a caber en
el presupuesto de fotograma. El trabajo es dejar de buscar la segunda capa en cada píxel y resolverla una vez
por tramo de línea, **sin cambiar ni un píxel**. Es el caso que la Decisión 35 dejó fuera a propósito.

### 2. Contexto

- Contexto consultado: el log de la sesión 20 (`blazing-perf-tv-20.txt`, 353 s, backend OpenGL, `ppu=recto-2`),
  leído con [`scripts/diagnostico/analizar_telemetria.py`](../../scripts/diagnostico/analizar_telemetria.py);
  [`docs/guias/medir-rendimiento.md`](../guias/medir-rendimiento.md) (presupuesto de fotograma y protocolo de
  comparación pareada); [`docs/explicacion/DECISIONES.md`](../explicacion/DECISIONES.md) (§34 y §35);
  [`2026-10-13_ppu-efectos-inertes-y-camino-especializado.md`](2026-10-13_ppu-efectos-inertes-y-camino-especializado.md)
  (el camino `recto-2`); [`2026-10-13_contadores-de-camino-de-la-ppu.md`](2026-10-13_contadores-de-camino-de-la-ppu.md)
  (la guarda y el reparto); [`2026-10-13_el-reloj-de-la-ppu-en-el-dispositivo.md`](2026-10-13_el-reloj-de-la-ppu-en-el-dispositivo.md)
  (propuesta pendiente dueña de la pregunta del cronómetro). Del motor: `crates/gba-core/src/ppu.rs` —
  `compose` (línea 890), la guarda `razon_camino`, `compose_straight` (834, llamada solo desde `compose`,
  líneas 1068 y 1079), `straight_span`, `top_two_layers` (622) con su contador de tests `BUSQUEDAS_DOS_CAPAS`
  (594), `alpha_blend` (1211), el desvío ya existente de `blend_mode == 0`, y
  `crates/gba-core/tests/perf_counters_tests.rs`. Del port: [`src/runner.rs`](../../src/runner.rs) (de dónde
  sale `ejec_ms`), [`src/recomp.rs`](../../src/recomp.rs) (`PpuPathCounters`) y [`src/perf.rs`](../../src/perf.rs).
- Intención: bajar `comp` de la escena de alpha real de 3,67 a ≤1,5 ms —y con ella `ejec`, que la contiene—,
  sin tocar la fidelidad de píxel.
- Restricciones y decisiones previas (máx. 3):
  - La exactitud de píxel manda sobre la velocidad y se demuestra por comparación diferencial contra el camino
    general forzado; nunca se debilita una aserción ni se modifica código de producción para que un test pase
    (origen: `AGENTS.md` §4.5, Decisión 34) → implica: si un píxel difiere, el atajo se revierte y no se ajusta
    ninguna tolerancia → protegida por: *El camino por tramos del alpha da el mismo framebuffer*.
  - En el modo alpha real (modo 1) el coste no es la aritmética —`alpha_blend` es un producto-accumulate de un
    ciclo por canal en ARM64— sino la búsqueda obligatoria de la segunda capa; por eso el modo 1 quedó en el
    camino general (origen: `DECISIONES.md` §35) → implica: el trabajo va en resolver las dos capas una vez por
    tramo, sin tocar coeficientes, tablas por `EVY` ni el camino del sprite translúcido → protegida por: *El
    camino por tramos no busca la segunda capa en cada píxel*.
  - La guarda y las cuentas nacieron de la misma decisión, y `rectas + mezcla + ventana + semi` suma 160 líneas
    por fotograma (origen: `2026-10-13_contadores-de-camino-de-la-ppu.md`) → implica: las líneas alpha siguen
    contándose donde hoy se cuentan (o en una cuenta nueva con su propio escenario), sin romper esa suma →
    protegida por: *El reparto sigue sumando 160 líneas por fotograma*.
- Alternativas descartadas:
  - **Abaratar la aritmética del alpha** (tablas por `EVY`, aproximar la división por 16) → descartada por
    medición: con la escena cargada, el premium está entero en la búsqueda. El prototipo que la elimina baja
    `compose` de 0,227 a 0,085 ms/fotograma dejando `bg` y `spr` planos.
  - **Llevar la composición a la GPU** (plan B de la Decisión 34) → descartada ahora: añade riesgo de
    divergencia de píxel, que es justo la restricción que manda, y el objetivo cabe en CPU.
  - **Atacar primero la instrumentación de `perf-counters`** → descartada como palanca de este cambio: su coste
    en el arnés está caracterizado en la propuesta del reloj, que es la dueña de esa pregunta, y no se duplica
    aquí; además la escena se mide igual con los contadores puestos (los `bg`/`spr` no cambian entre variantes).
  - **Una sola búsqueda de la segunda capa por línea** en vez de por tramo → descartada: la segunda capa cambia
    dentro de la línea (ventana, tramos de sprite, entradas y salidas de transparencia de las capas objetivo),
    así que daría píxeles distintos; con esos bordes ya localizados, hacerlo por tramo es el mismo trabajo y
    los mismos píxeles.
- Ante imprevistos, prioriza: exactitud de píxeles sobre velocidad.

### 3. Ejemplos

- Caso normal (el de la sesión 20): `BLDCNT` en modo 1 con segundos objetivos presentes, en una línea de mapa
  con BG0 sobre BG1 — antes: `razon_camino = 1`, camino general, 240 búsquedas de dos capas por línea, `comp`
  3,67 ms de media en esa escena (hasta 4,4 ms) y 88 de 186 s por debajo de 60 fps → ahora: cortes de línea y
  **una búsqueda por tramo** para los mismos 240 píxeles; la línea sigue contándose en `mezcla`.
- Caso de la sesión 20: los volcados cada 2 s del desglose muestran `mezcla` creciendo 19.200 líneas mientras
  `rectas` no se mueve (162,7 líneas por fotograma en esos intervalos) → `comp` de 3,67 a ≤1,5 ms.
- Caso límite: línea con alpha real **y** un píxel de sprite translúcido en x=100 → antes: camino general →
  ahora: también camino general (esa línea no adopta el camino nuevo) y el reparto no cambia.
- Caso límite: línea con alpha real cuyo segundo objetivo está ausente bajo la capa ganadora (todo transparente
  por debajo) → el camino general mezcla la capa de arriba con el backdrop; el camino nuevo tiene que dar
  exactamente ese píxel, no la capa de arriba sin mezclar.
- Caso límite: un tramo cuyo borde cae en el píxel 100 porque ahí empieza un sprite → si los cortes no incluyen
  los bordes de sprite, el píxel 100 difiere y el escenario 2 salta.

### 4. Pasos

1. Motor, `ppu.rs`: prototipo correcto por tramos (resuelve las dos capas una vez por tramo) detrás del
   interruptor de tests que ya existe, y medirlo con el banco con entrada para saber cuánto del techo de
   0,142 ms/fotograma sobrevive después de pagar los cortes.
2. Motor, `ppu.rs`: construir los cortes de la línea como unión de los bordes de ventana (que `compose_straight`
   ya calcula), los bordes de los tramos de sprite y los puntos donde una capa objetivo pasa de opaca a
   transparente; medir el coste de calcularlos, que es el riesgo real de todo el plan.
3. Motor, `ppu.rs`: por tramo, resolver las dos capas superiores una sola vez (reutilizando la expresión de
   `top_two_layers`) y aplicar `alpha_blend(fc, sc, eva, evb)`, con el desvío ya existente de `blend_mode == 0`
   como modelo de cómo se pasa de "por píxel" a "por tramo".
4. Motor, `ppu.rs`: dejar las líneas con sprite translúcido en el camino general con su comentario de por qué,
   y no reclasificar ninguna línea en la telemetría.
5. Tests del motor: diferencial contra el camino general forzado (los 240 píxeles), el límite del sprite
   translúcido, el límite del borde de tramo y la invariante del reparto.
6. Banco y cierre: llevar a `scripts/diagnostico/` el banco con entrada (script `gba-input v1`, driver de pares
   intercalados y los hashes de referencia) y el log de la sesión 20 que hoy solo vive fuera del repo —en la
   carpeta ya hay `log_tv15.txt`—; actualizar la guía de medición con cómo se lee `comp` en esta escena y
   escribir la decisión en `DECISIONES.md`.

### 5. Qué más podría verse afectado

- `compose_straight` (834) y sus dos llamadas desde `compose` (1068, 1079): el camino nuevo es una variante de
  esa maquinaria de tramos; su firma solo se usa ahí, según la búsqueda de referencias.
- `top_two_layers` (622) y `BUSQUEDAS_DOS_CAPAS` (594, solo existe en tests): sus tests exigen **cero**
  búsquedas en el camino especializado de los modos 2 y 3 (`oscurecido_real_no_busca_la_segunda_capa`,
  `src/ppu.rs`). El camino nuevo sí busca (una vez por tramo), así que una expectativa reutilizada tendría que
  ser "≤ número de tramos", nunca 0.
- La guarda `razon_camino` y el reparto informativo (`lineas_inertes`, `lineas_efecto_alpha`): se amplían; los
  tests de efectos inertes de `recto-2` no cambian.
- Los dos hashes guardianes del arnés: `b54066dc06f2b76c` con 3.208.998 bloques nativos (7200 fotogramas sin
  entrada) y `ee7dc7ef514b5a3f` con 11.018.334 bloques (7200 con la entrada del banco). Este cambio no puede
  mover un píxel, así que los dos tienen que quedar idénticos.
- Port: [`src/recomp.rs`](../../src/recomp.rs) (`PpuPathCounters::to_line`) y
  [`src/recomp_tests.rs`](../../src/recomp_tests.rs) solo si se añade una cuenta nueva; en ese caso, también la
  tabla de campos de [`docs/guias/medir-rendimiento.md`](../guias/medir-rendimiento.md).
- `scripts/diagnostico/analizar_telemetria.py` no parsea `# ppucamino` (fuera de alcance declarado en las dos
  propuestas anteriores): los campos nuevos no lo afectan.

### 6. Suposiciones

- La segunda capa es constante a trozos con los cortes que se pueden calcular (ventana ∪ tramos de sprite ∪
  entradas y salidas de transparencia de las capas objetivo) → prototipar: sí
  → si es falso: harán falta más cortes, el cálculo puede comerse el ahorro y el plan pasa a activar el camino
  solo en las líneas sin sprites; el banco con entrada cubre mapas y menús, así que el paso 1 lo dice antes de
  escribir el camino definitivo.
  → resultado del prototipo: **cierto**. La pareja (arriba, segunda) se resuelve una vez por subtramo de
  clave constante: 3 búsquedas en la línea de los escenarios 1 y 2 (las dos gráficas de fondo más el
  sprite) frente a las 240 del camino general, y 1 búsqueda en la línea sin sprite del escenario 1. La
  clave es «qué capas habilitadas son opacas y qué prioridad tiene el sprite que tapa», que es todo lo
  que `top_two_layers` necesita de `x`. | no detecta: escenas con muchas capas objetivo a la vez, que el
  banco no contiene.
- El coste de calcular los cortes es menor que las 240 búsquedas que sustituye → prototipar: sí (se mide dentro
  del paso 1)
  → si es falso: la ganancia real se aleja del techo y la decisión se toma con la cifra medida; si no llega al
  listón del §11, se revierte y se cierra como discrepancia.
  → resultado del prototipo: **cierto, con la cifra medida**. El recorrido de claves cuesta 0,059 ms por
  fotograma (0,144 medidos frente a los 0,085 del techo que no busca la segunda capa en absoluto), es
  decir el 41 % del premium, y la ganancia realizada es el 59 % del techo: 0,228 → 0,144 ms/fotograma,
  por debajo del objetivo de 0,145 del §11. | no detecta: el coste por tramo en líneas con muchos
  sprites, que el arnés de host no reproduce con la misma mezcla que el juego en el dispositivo.
- `ejec` contiene a `comp`, así que quitar 2,27 ms de composición baja el paso de simulación en la misma medida
  (verificado en el código: `ejec_ms` es el tiempo de pared de `run_frame_native` y `ppu_compose_ms` sale de los
  contadores de hardware, en [`src/runner.rs`](../../src/runner.rs)) → prototipar: no
  → si es falso: la ganancia no se traslada al presupuesto y se cierra sin cambio.

### 7. Alcance

- Incluye: el camino por tramos del alpha real (modo 1) con su verificación diferencial de píxel; los cortes de
  línea que necesita; la medición de la ganancia real frente al techo ya medido; y llevar a
  `scripts/diagnostico/` el banco con entrada y el log de la sesión 20.
- No incluye: la instrumentación `perf-counters` ni el reloj de la PPU (propuesta pendiente aparte); la cola de
  presentación (fotogramas que se pierden con la CPU dentro del presupuesto: `espera` y `present` tienen sus
  propios picos); el APU; el modo 5, las transformaciones afines de la OAM y la composición en GPU; y el
  hallazgo de los modos 2 y 3 que se registra abajo.

### 8. Criterios de aceptación (no se modifican sin aprobación)

La fidelidad se comprueba en dos escalas y las dos son obligatorias: **diff de framebuffer**, píxel a píxel
contra el camino general forzado (escenarios 1 y 2, la que decide si el atajo es correcto), y **hashes
guardianes** de la sesión completa sin entrada y con la entrada del banco (escenario 4, la que protege todo lo
que los escenarios 1 y 2 no visitan).

Escenario: El camino por tramos del alpha da el mismo framebuffer que el camino general
Caso de uso: jugar una escena con desvanecido de pantalla completa | Tipo: normal
  Dado un `MemMap` con dos capas de fondo visibles, sin ventana ni sprite en la línea, con `BLDCNT = 0x241`
  (modo alpha, BG0 como primer objetivo, BG1 en la máscara de segundos objetivos), `BLDALPHA = 0x0808` y
  píxeles distintos en las dos capas
  Cuando se renderiza la misma línea con el camino nuevo activo y con `SIN_VIA_RAPIDA` (el camino general)
  Entonces los 240 píxeles coinciden exactamente y `BUSQUEDAS_DOS_CAPAS` sube exactamente 1 en esa línea
  (hoy subiría 240)
Test: `alpha_por_tramos_da_los_mismos_pixeles` en `crates/gba-core/src/ppu.rs` (tests del motor)
Estado previo: **falla** (el camino no existe; el contador sube 240)

Escenario: Una línea con alpha real y un sprite translúcido no cambia de camino
Caso de uso: jugar un combate con semitransparencias y desvanecido a la vez | Tipo: límite
  Dado el mismo montaje con un píxel de sprite translúcido en x=100
  Cuando se compone la línea
  Entonces los 240 píxeles coinciden con el camino general y `BUSQUEDAS_DOS_CAPAS` sube 240
Test: `alpha_con_sprite_translucido_se_queda_en_el_camino_general` en `crates/gba-core/src/ppu.rs`
Estado previo: pasa (regresión: es el comportamiento de hoy)

Escenario: El reparto sigue sumando 160 líneas por fotograma
Caso de uso: leer la telemetría de una sesión en Android | Tipo: regresión
  Dado un fotograma completo con líneas de alpha real y líneas rectas
  Cuando se leen los contadores del motor
  Entonces `lineas_rectas + descartes_mezcla + descartes_ventana + descartes_semi` es 160 y las líneas del
  camino nuevo siguen contándose en `descartes_mezcla`
Test: `test_el_desglose_de_efectos_cuadra_con_mezcla` en `crates/gba-core/tests/perf_counters_tests.rs`
Estado previo: pasa (regresión)

Escenario: El juego completo conserva sus píxeles y sus bloques
Caso de uso: comprobar que la optimización no cambia el juego | Tipo: regresión
  Dada la misma imagen `.dll` traducida y el BIOS oficial
  Cuando se corren 7200 fotogramas con el arnés, sin entrada y con la entrada del banco
  Entonces `hash: b54066dc06f2b76c / native_blocks: 3208998 / fallback_steps: 0` sin entrada y
  `hash: ee7dc7ef514b5a3f / native_blocks: 11018334 / fallback_steps: 0` con entrada
Test: los dos comandos del arnés (`recomp runc`, con y sin `--input`); no es un test unitario
Estado previo: pasa (regresión, medido hoy)

Escenario: La escena de alpha vuelve al presupuesto en la telemetría del dispositivo
Caso de uso: jugar una escena con el desvanecido de pantalla completa | Tipo: normal
  Dada la misma escena de la sesión 20, con las ~160 líneas de cada fotograma en el camino de mezcla
  Cuando se lee la telemetría del APK con `analizar_telemetria.py`
  Entonces `comp` baja de 3,67 a ≤1,5 ms y `ejec` de 14,33 a ≤12,5 ms (y de 16,00 a ≤13,7 ms en los segundos
  que hoy pierden fotogramas), sin cambiar el reparto
Test: el log de la sesión siguiente analizado con `scripts/diagnostico/analizar_telemetria.py` (escenario
visible; no hay test unitario)
Estado previo: **falla** (sesión 20: `comp` 3,67 ms, `ejec` 14,33 ms, 88 s de 186 por debajo de 60 fps)

### 9. Cómo lo comprobaré

- **El camino por tramos da el mismo framebuffer** → comando:
  `cd ../gba-recomp && cargo test -p gba-core --features perf-counters alpha_por_tramos_da_los_mismos_pixeles`
  → pasa si: los 240 píxeles coinciden y `BUSQUEDAS_DOS_CAPAS` sube 1 en esa línea
  → no detecta: ventanas de sprite, combinaciones con `semi` ni el coste (cuenta llamadas, no mide tiempo); el
  coste lo cubren el escenario 5 y el banco.
- **La línea con sprite translúcido no cambia de camino** → comando:
  `cargo test -p gba-core alpha_con_sprite_translucido_se_queda_en_el_camino_general`
  → pasa si: los 240 píxeles coinciden y el contador sube 240
  → no detecta: que esa línea siga siendo la más lenta de la escena (no es el objetivo de esta ronda).
- **El reparto sigue sumando** → comando:
  `cargo test -p gba-core --features perf-counters --test perf_counters_tests`
  → pasa si: la suma da 160 y ningún camino nuevo se sale del reparto
  → no detecta: que el reparto describa la escena que se ve (eso lo dice el log del dispositivo).
- **El juego completo conserva píxeles** → comandos, desde la raíz del port:
  `../gba-recomp/target/release/recomp.exe runc "roms/1575 - Fire Emblem (E)(Independent).gba" --bios roms/gba_bios.bin --frames 7200`
  y el mismo con `--input scripts/diagnostico/entrada_avance.input`
  → pasa si: los dos hashes, los dos recuentos de bloques y `fallback_steps: 0` son los anotados
  → no detecta: el backend Vulkan, `arm64` ni las escenas que la entrada no visita.
- **El techo y la ganancia real (predicción medida, no promesa)** → comando: los dos binarios medidos por pares
  intercalados con `RECOMP_PPU_STATS=1` sobre 7200 fotogramas con `--input` (3 pares, misma sesión, misma imagen
  `.dll`), que es el protocolo del banco
  → techo medido hoy: `compose` 0,227 → **0,085 ms/fotograma**, es decir 0,142 de premium (la primera
  campaña de la sesión dio 0,225 → 0,085, es decir 0,140: las dos coinciden dentro del ruido del arnés)
  con `ppu` 0,617 → 0,479, `bg`/`spr` planos, lecturas y escrituras idénticas y los 11.018.334 bloques
  idénticos; tiempo de muro −1069 ms de mediana, 3 de 3 pares negativos, coherente con 0,142 ms × 7200 = 1,02 s
  → pasa si: el camino definitivo baja `compose` de 0,227 a **≤0,156** con los guardianes intactos, y termina
  cuando **≤0,145** (el techo ideal es 0,085)
  → no detecta: el dispositivo; lo cubre el escenario de telemetría, y la extrapolación host→dispositivo usa
  cocientes de tiempo de fotograma, no cifras absolutas.
- Conjunto: `cargo test` en los dos repositorios, `python .agents/skills/diataxis-docs/scripts/validar_docs.py .`,
  `python scripts/extraer_referencias.py --check` y `python scripts/diagnostico/auditar_labels.py`.
- **Blindaje por mutación:** M1 — que el camino nuevo vuelva a buscar la segunda capa por píxel → debe morir
  *El camino por tramos del alpha da el mismo framebuffer* (el contador subiría 240). M2 — que los cortes
  ignoren los bordes de sprite → debe morir el escenario 2 (píxeles distintos en x=100). M3 — que el camino
  nuevo se aplique también a las líneas con sprite translúcido → debe morir el escenario 2 (el contador bajaría
  de 240). Los archivos se respaldan en RAM y se restauran en un bloque `finally`, sin comandos de Git.

### 10. Terminado cuando

Se cumplen los cinco criterios, los dos hashes guardianes siguen intactos, la suite de los dos repositorios
pasa, el banco con entrada y el log de la sesión 20 están en `scripts/diagnostico/`, y la decisión está en
`DECISIONES.md` con su tabla de medición.

### 11. Parar si

- 2 intentos fallidos seguidos con el mismo criterio: detenerse, explicar qué se probó y marcar la propuesta
  como `discrepada`.
- **Si el prototipo por tramos no baja `compose` de 0,227 a 0,156** (menos de la mitad del techo de 0,142
  recuperada), no se encadenan variantes: se documenta el reparto medido y se cierra.
- Si el camino nuevo exige más cortes de los que el cálculo puede pagar en las líneas con sprites, no se fuerza
  una heurística silenciosa: se acota el camino a las líneas sin sprites, se mide esa ganancia y se declara como
  el alcance real.

### Correcciones previas a presentar

- **Corregido antes de presentar:** mi primer borrador ponía como criterio de éxito "recuperar el techo"
  (0,140 ms/fotograma, el premium medido en la primera campaña; el reproducido hoy es 0,142). No es honesto: el
  prototipo del techo **no busca la segunda capa en absoluto**, mientras que un camino correcto sí lo hace, una
  vez por tramo. El §8 y el §11 usan ahora la mitad del techo como listón (≤0,156) y 0,145 como objetivo.
- **Corregido también:** la primera medición del banco (sin entrada) daba `hash` idéntico en diez corridas y
  `compose` −0,006 ms/fotograma, como si no hubiera nada que ganar. El arnés sin entrada vive en el título
  (`compose` 0,078 ms/fotograma, 3.208.998 bloques nativos, 445 por fotograma); con la entrada que sale del
  título son 11.018.334 bloques y `compose` 0,227. Por eso el escenario de medida de esta propuesta es el banco
  **con** entrada.
- **Corregido también:** iba a incluir bajar la instrumentación como parte del plan; esa pregunta ya tiene
  propuesta pendiente propia y aquí se descarta como palanca, sin duplicar su medición.
- **Corregido también:** había escrito que la escena costaba "17,92 ms por fotograma". Lo que dicen los números
  de la sesión 20 es: `ejec` medio 14,33 ms en los 186 s de la escena, 16,00 ms de media (17,97 ms de peor) en
  los 88 s que pierden fotogramas. El §1, el §8 y el §9 usan esas cifras.

### Cambios tras prototipos (solo si el plan cambió)

- **Los cortes no se materializan.** El §4.2 pedía «construir los cortes de la línea como unión de bordes de
  ventana, de sprite y de transparencia». El prototipo que llegó al objetivo no los guarda en un arreglo: recorre
  la línea calculando una clave por píxel (cuatro comparaciones de transparencia y una prioridad) y corta cuando
  la clave cambia, resolviendo la pareja una vez por subtramo. Es el mismo trabajo por píxel que leería los
  bordes, sin memoria intermedia, y los tramos de ventana que `compose_straight` ya calculaba se reutilizan tal
  cual. Medido dentro del objetivo del §11, así que no hace falta el arreglo de cortes.
- **El contador de tests `BUSQUEDAS_DOS_CAPAS` pasó de global a por hilo.** Los tests del motor corren en
  paralelo y una cuenta `AtomicU64` compartida mezclaba las búsquedas de la línea de un test con las de otro:
  el escenario 1 medía 248 en vez de 240 en el camino general. Sin ese cambio la aserción *exacta* del escenario 1
  no es determinista. Es instrumentación que solo existe bajo `#[cfg(test)]`; el binario no cambia (los dos
  hashes guardianes y el de los dorados siguen idénticos).
- El **techo** del alpha se midió antes de esta propuesta por petición explícita (respuesta humana del
  2026-10-13) y se reprodujo para el §9; no es un prototipo de esta propuesta.

### Fuera de alcance (hallazgos que no se arreglaron)

- **Los modos 2 y 3 (brillo y oscuridad) buscan la segunda capa en cada píxel sin necesitarla.** El desvío de
  `blend_mode == 0` ya aplaza la búsqueda a los píxeles con sprite translúcido, pero en los modos 2 y 3 el bucle
  general llama a `top_two_layers` 240 veces por línea aunque el resultado solo dependa de la capa de arriba
  (el propio test del motor lo deja escrito: `oscurecido_real_no_busca_la_segunda_capa` exige 0 búsquedas en el
  camino especializado y >0 en el general). Los destellos de pantalla del juego caen aquí. **Impacto: medio.**
- **`analizar_telemetria.py` no lee `# ppucamino`.** El desglose por caminos de composición se analiza a mano.
  **Impacto: bajo** (arrastrado de las dos propuestas anteriores).
- **`descartes_mezcla` ya no describe lo que su nombre dice.** Con el camino por tramos, una línea contada en
  `descartes_mezcla` ya no es «una línea descartada de la vía rápida»: es «una línea con alpha real que
  necesita la segunda capa», y ahora se compone por tramos. El §8 obliga a seguir contándola ahí y el §4.4
  prohíbe reclasificar la telemetría en esta ronda, así que el campo (y su texto en `# ppucamino`) se lee mal
  hasta que una propuesta de telemetría lo renombre o añada una cuenta propia. **Impacto: bajo.**
- **La telemetría no identifica el APK que la emite.** La cabecera trae backend, modo, `reloj` y la versión de
  pipeline (`ppu=recto-2`), pero ninguna marca del binario, así que atribuir una sesión a un APK depende de la
  hora del archivo. Un campo con la versión de la aplicación o el hash corto del `.so` cerraría esa duda.
  **Impacto: bajo.**
- **El coste del reloj no es estable entre ejecuciones.** La misma familia de binarios declara 1.514 ns (sesión
  19), 1.504 (20) y **2.073** (21). Si la instrumentación toma una marca de tiempo por línea o por fase, esa
  diferencia son décimas de milisegundo por fotograma dentro de `comp`, justo la partida que se acaba de ganar.
  Es la pregunta de la propuesta
  [`2026-10-13_el-reloj-de-la-ppu-en-el-dispositivo.md`](2026-10-13_el-reloj-de-la-ppu-en-el-dispositivo.md),
  que sigue pendiente. **Impacto: medio.**

---

## Resultado (lo completa el agente después de editar)

**Estado: paso 1 del §4 cerrado y verificado, con el paso 3 y los escenarios 1, 2, 3 y 4 del §8 dentro.** El
camino por tramos del alpha real es `compose_alpha_tramos` en `crates/gba-core/src/ppu.rs`: recorre los tramos
de control de ventana que ya calculaba el camino recto, los parte en subtramos de clave constante
(`clave_de_tramo`: capas de fondo habilitadas y opacas más la prioridad del sprite), resuelve la pareja con
`top_two_layers` una vez por subtramo y la compone con la misma regla del camino general —`control & 0x20`,
`BLDCNT & (1 << arriba)`, `BLDCNT & (0x100 << segunda)`, `alpha_blend`— en `componer_tramo`. Una línea con
alpha real (`razon_camino == 1`) adopta ese camino solo si no tiene ningún sprite translúcido (`&& !obj_semi`),
y sigue contándose en `descartes_mezcla`.

### Predicho vs. real

| Predicho (§8, §9, §11) | Real medido |
|---|---|
| Escenario 1: 240 píxeles iguales y **una** búsqueda de la segunda capa (hoy 240) | ✅ `alpha_por_tramos_da_los_mismos_pixeles`: 1 búsqueda, 240 píxeles idénticos |
| Escenario 2: con un sprite translúcido, 240 píxeles y 240 búsquedas (regresión) | ✅ `alpha_con_sprite_translucido_se_queda_en_el_camino_general`: 240 búsquedas, 240 píxeles idénticos |
| Escenario 3: el reparto sigue sumando 160 líneas y las de alpha siguen en `descartes_mezcla` | ✅ `perf_counters_tests`: 3/3 verdes |
| Escenario 4: `b54066dc06f2b76c / 3208998` sin entrada y `ee7dc7ef514b5a3f / 11018334` con entrada, `fallback_steps: 0` | ✅ los dos hashes, los dos recuentos y el `fallback_steps` idénticos |
| §9: `compose` de 0,227 a **≤0,156** para parar y **≤0,145** como objetivo (techo ideal 0,085) | ✅ **0,230 → 0,144 ms/fotograma** (medianas de 3 pares intercalados, binario definitivo); la campaña previa del mismo prototipo dio 0,228 → 0,144 |
| §9: `ppu` ─?, `bg`/`spr` planos, lecturas y escrituras idénticas, bloques idénticos | ✅ `ppu` 0,625 → 0,538; `bg` 0,231 → 0,231 y `spr` 0,164 → 0,163; 5.042/4.184 por fotograma en las dos variantes; 11.018.334 bloques en las dos |
| §9: tiempo de muro coherente con el premium recuperado (0,086 ms × 7200 = 0,62 s) | ✅ mediana **−713 ms (−7,13 %)**, 3 de 3 pares negativos (la campaña previa: −590 ms) |
| §9: **M1** (volver a buscar por píxel), **M2** (ignorar el borde del sprite), **M3** (aplicar el camino a las líneas con sprite translúcido) deben morir | ✅ los tres mueren: M1 mata el escenario 1 y el diferencial ampliado, M2 el diferencial ampliado, M3 el escenario 2 |
| Marcas (§9): los dorados de la escena real, 5 anclas | ✅ 5/5 `OK`, los mismos hashes que antes del cambio |
| §10: suite de los dos repositorios, `validar_docs.py`, `extraer_referencias.py --check`, `auditar_labels.py` | ✅ motor 87+3 verdes; port 109+4+15; los tres scripts en verde |

### Lo que falta de esta propuesta

- **Escenario 5 (telemetría del dispositivo): parcialmente verificado** con la sesión 21
  (`blazing-perf-tv-21.txt`, 117 s, Android TV, OpenGL, `ppu=recto-2`, 100 % AOT) frente a la 20:
  - `comp` mediana **3,50 → 1,80 ms**, y la población de líneas no cambia de composición: el 78 % de las líneas
    del fotograma son alpha real (`efalpha`, 855.929 de 1.096.641; en la sesión 20 era el 99 %). El objetivo de
    **≤1,5 ms no se alcanza** (1,80).
  - `ppu` mediana 6,20 → 5,65 ms, y **7,50 → 6,20** en los segundos que pierden fotogramas (−17 %).
  - `ejec` mediana 12,30 → **14,10 ms: no cumple** el objetivo de ≤12,5 (en los segundos que pierden
    fotogramas baja de 16,00 a 15,30). La escena de la sesión 21 es más pesada en lógica de juego (~6,6 ms
    frente a ~4,6), así que los fps no son comparables entre sesiones; sí lo son las fases del motor.
  - `bg` 2,30 → 2,20, `dibujo` 5,50 → 5,40 y `present` 3,80 → 3,70 quedan planos, que es lo que se esperaba de
    un cambio que solo toca la composición; `spr` sube 1,40 → 1,60 (+0,2, dentro del ruido de la escena) y `apu`
    1,50 → 1,90.
  - Extrapolación (modelo, no medida del binario anterior): con el `comp` de la sesión 20 (+1,7 ms en todos los
    segundos), 85 de los 115 s habrían cruzado el umbral de fotograma frente a los 25 de ahora.
  - Todos los segundos por debajo de 60 tienen el camino de CPU sobre presupuesto (`ejec ≥ 15,3 ms`) con
    `dibujo` y `present` planos: en esta sesión el techo no es la GPU.
- **§4.6 y §10:** llevar a `scripts/diagnostico/` el banco con entrada y el log de la sesión 20, actualizar la guía
  de medición con cómo se lee `comp` en esta escena y escribir la decisión en `DECISIONES.md`. Pendiente.

### Cómo reproducir la medición

Desde la raíz del port (necesita el binario del motor con `--features perf-counters` y el respaldo del camino
general, que en un binario normal no existe):

```
cargo test -p gba-core --features perf-counters            # en ../gba-recomp
python scripts/diagnostico/fotogramas_dorados.py comparar --bin <recomp> --rom <rom> --bios <bios>
```
y la comparación pareada de dos binarios con `RECOMP_PPU_STATS=1` sobre 7200 fotogramas con
`--input scripts/diagnostico/entrada_avance.input` (3 pares), que es la que da el `compose` de la tabla.

---

## Revisión humana
<!-- El agente SÍ edita esta sección: escribe aquí los escenarios que no se
     pudieron comprobar con código ("(visible)" o "no pudo comprobarse") y las restricciones
     marcadas "no testeable",
     uno por línea, dejando las marcas ✔ | ✘ | ? sin elegir.
     El agente NO elige marca, NO escribe la nota y NO rellena
     "Detectado solo por el humano": eso es solo de la persona.
     Si no queda nada para revisar: "ninguno: todo se comprobó con tests
     ([cuáles])". -->

- La escena larga de alpha vuelve a 60 fps en el dispositivo, con `comp` ≤1,5 ms y `ejec` ≤12,5 ms
  (escenario visible): ✔ | ✘ | ?
- Los desvanecidos y los efectos de color se ven exactamente igual que antes del cambio (visible): ✔ | ✘ | ?
- ¿Merece la pena mantener un segundo camino de composición en la PPU? (no testeable): ✔ | ✘ | ?
- Detectado solo por el humano: [lo completa la persona]
