# Propuesta: El acceso a memoria deja de ser una llamada indirecta por instrucción

Fecha: 2026-10-12
Nivel: 2
Estado: discrepada

<!-- Estados: pendiente | aprobada | modificada | descartada |
     verificada por agente | revisada por humano | discrepada | cerrada -->

## Propuesta (la escribe el agente antes de editar)

### 1. Qué cambia

Cada lectura o escritura que hace el juego deja de pasar por una llamada indirecta a la capa
nativa y se resuelve con unas pocas instrucciones en el propio código generado, salvo donde el
acceso sí tiene efectos (registros de hardware, paleta, OAM, guardado de partida). El objetivo es
que los fotogramas vuelvan a caber en el presupuesto de 60 Hz, que hoy ocupan al **81 %**.

### 2. Contexto

- Contexto consultado: la telemetría de las sesiones 13-17 del Android TV (analizada con
  [`analizar_telemetria.py`](../../scripts/diagnostico/analizar_telemetria.py)), los registros
  versionados `scripts/diagnostico/log_tv15.txt`; del motor: `crates/recomp-core/src/emit.rs`
  (896 líneas: preámbulo C y sitios de emisión), `crates/gba-core/src/mem.rs` (1907 líneas,
  `read8/16/32` y `write8/16/32`), `crates/gba-core/src/capi.rs` (139 líneas) y
  `crates/gba-core/src/bus.rs` (trait `Bus`, con `FlatBus` en `exec.rs`); del port:
  [`src/recomp.rs`](../../src/recomp.rs), [`scripts/package_apk.ps1`](../../scripts/package_apk.ps1)
  y [`tests/packaging_tests.rs`](../../tests/packaging_tests.rs). Más
  [`2026-10-12_ppu-60fps-estable.md`](2026-10-12_ppu-60fps-estable.md), que dejó medido el residuo.
- Intención: bajar el tiempo de CPU de la emulación lo suficiente para que desaparezcan las caídas
  que quedaron tras la PPU. El éxito se ve como **ningún segundo por debajo de 58 FPS** en la
  telemetría del dispositivo, que es el criterio que quedó sin cumplir en la propuesta anterior.
- Restricciones y decisiones previas (máx. 3):
  - El renderizado no cambia (origen: la propuesta anterior y el arnés del motor) → implica: el
    camino de memoria puede reordenarse, pero el resultado debe terminar en el mismo búfer de
    fotograma → protegida por: *El renderizado no cambia (regresión)*.
  - La telemetría es la que decide, no la intuición (origen: «Para la próxima vez» de la propuesta
    anterior, tras invalidar una comparación entre sesiones) → implica: cada ganancia se mide en la
    misma sesión con un subsistema intacto como control → protegida por: *El salto indirecto es lo
    que cuesta* y *El residuo baja en el dispositivo*.
  - El motor se desarrolla en local y el port lo consume por `[patch]`, con el SHA aún sin publicar
    (origen: [`Cargo.toml`](../../Cargo.toml), `rev = "de4edf59…"`) → implica: el port no cambia
    hasta que el motor esté medido; publicar el motor y re-fijar el SHA es trabajo aparte →
    protegida por: no testeable (queda en «Revisión humana»).
- Alternativas descartadas:
  - Optimizar el despacho por bloque → descartada por el dato: la correlación del residuo con los
    bloques ejecutados es **+0,025** frente a **+0,877** con los accesos, y el coeficiente ajustado
    por bloque es indistinguible de cero.
  - Atacar el APU (1,5 ms) o la resolución de render → descartadas en la propuesta anterior con
    medida (`present` no limita: cuando hay caídas es el valor *más bajo* de la sesión).
  - Caché de líneas de la PPU (dirty-line) → descartada por riesgo arquitectónico antes de esta.
  - Contar con que el contador de accesos sea el culpable → descartada: el incremento vive dentro
    del callee y no puede explicar 0,27 µs por acceso; aun así se medirá en la fase A.
- Ante imprevistos, prioriza: el búfer de fotograma idéntico por encima de la ganancia.

### 3. Ejemplos

- **Caso normal** (fotograma típico de la sesión 17: 12.214 accesos y 5.104 bloques nativos):
  antes: residuo (`ejec − ppu − apu`) de **4,5 ms**, de los que el ajuste atribuye **~3,7 ms** al
  acceso a memoria a 0,27 µs cada uno;
  ahora (objetivo): residuo ≤ 2 ms con los mismos accesos.
- **Caso límite** (el segundo de más tráfico medido: `lecturas` 16.598 + `escrituras` 8.827 =
  25.425 accesos, sesión 17 t=143 s):
  antes: `ejec` 15,3 ms y 59 FPS;
  ahora (objetivo): `ejec` ≤ 13 ms y 60 FPS.
- **Caso de instrumentación** (cualquier fotograma con la feature `perf-counters` activa, que es
  como se publica el APK):
  antes: seis incrementos por acceso dentro de `mem.rs` (líneas 1190, 1254, 1300, 1375, 1450 y
  1494) medidos por primera vez solo ahora;
  ahora (objetivo): un valor medido; si pesa, sale del camino caliente como el reloj.

### 4. Pasos

**Fase A — medir, sin cambiar la semántica.** Es la que puede matar la propuesta antes de tocar el
generador.

1. A/B en host de la feature `perf-counters` encendida y apagada sobre la misma ROM y los mismos
   fotogramas, con `bg`/`apu` como control: separa lo que cuesta el contador de lo que cuesta el
   camino.
2. Benchmark en el motor que compare, sobre las mismas direcciones y el mismo patrón, una llamada
   por la vtable contra una lectura por puntero directo (10⁷ accesos, mismo binario, con
   `cfg(feature = "perf-counters")`): dimensiona el premio antes de reescribir el codegen.
3. Prototipo de una sola región: emitir el camino rápido **solo para lectura de ROM** (excluyendo
   las ventanas GPIO/EEPROM de `is_gpio`/`is_eeprom_addr` y los espejos, que caen al camino actual)
   detrás de una constante de compilación, y comprobar los tres hashes guardianes.

**Fase B — el cambio**, solo si la fase A confirma que el salto es el coste.

4. Publicar en `RtApi` los punteros y longitudes de las regiones puras (ROM, EWRAM, IWRAM) y
   añadir en el preámbulo C de `emit.rs` un `static inline` que resuelva el acceso por región y
   caiga a `a->read*`/`a->write*` para todo lo demás.
5. Emitir por ese camino las lecturas y escrituras de las regiones puras, dejando intacto el resto
   (MMIO, VRAM, paleta, OAM, GPIO/EEPROM, espejos y open bus).
6. Medir en el dispositivo con el A/B del apartado 9 y, cuando el premio esté probado, publicar el
   motor y re-fijar el SHA.

### 5. Qué más podría verse afectado

Búsquedas hechas: `grep -rn "RtApi" src/ tests/ crates/`, `grep -rln "src-c" src/ tests/ scripts/`,
`grep -rn "trait Bus|impl Bus for" crates/gba-core/src/`, `grep -rn "perf_counters.reads|writes"`
y `git check-ignore -v src-c/`.

-  `crates/recomp-core/src/emit.rs` — el preámbulo (`PREAMBLE`, línea 23, donde vive la vtable) y los
  sitios de emisión de acceso (`emit.rs:367`, 436, 681 y 688: carga, salto encadenado y retorno)
  definen el formato de **todo** el C generado.
- `src-c/` (port, 206 MB, 12 archivos `.c`) — ignorado por `.gitignore:17` y con 0 archivos en el
  índice: se regenera entero con
  [`compilar_recomp_aot.ps1`](../../scripts/diagnostico/compilar_recomp_aot.ps1), así que suben el
  tiempo de compilación y el tamaño del APK. También lo consumen
  [`compilar_c_paralelo.py`](../../scripts/diagnostico/compilar_c_paralelo.py) y
  [`package_apk.ps1`](../../scripts/package_apk.ps1).
- `crates/gba-core/src/capi.rs` — `RtApi` (línea 23) y los puentes `rt_read8/16/32` (línea 68):
  al añadir campos al struct, todo el que lo construya o lo declare cambia.
- `crates/gba-core/src/mem.rs` y `bus.rs` — el camino lento sigue siendo el mismo; el trait `Bus`
  tiene dos implementaciones (`MemMap` y `FlatBus` en `exec.rs:737`), y solo la primera publica
  regiones.
- [`src/recomp.rs`](../../src/recomp.rs) (port) — usa `gba_core::capi::RtApi` como tipo de los
  bloques (línea 15) y [`src/recomp_tests.rs`](../../src/recomp_tests.rs) lo usa en firmas de
  bloques falsos; si `RtApi` cambia de forma, hay que comprobar si estos archivos siguen
  compilando sin tocarlos.
- [`tests/packaging_tests.rs`](../../tests/packaging_tests.rs) (port) — el APK cambia de tamaño y
  de contenido; hay pruebas de empaquetado que pueden asumir conteos.
- Sin más referencias encontradas.

### 6. Suposiciones

- **Los lectores de ROM, EWRAM e IWRAM no tienen efectos secundarios ni contabilidad de ciclos**, de
  modo que leer por puntero directo es equivalente a leer por el bus. → prototipar: sí, con el
  prototipo de ROM del paso 3 y los tres hashes → si es falso: el camino rápido se limita a las
  regiones puras que sí lo cumplan, o se rellena la contabilidad en el propio camino rápido
  → resultado del prototipo: **confirmada** para la lectura de ROM: con el camino rápido activo los tres
  guardianes son idénticos (600, 2400 y 7200 fotogramas) y `fallback_steps` sigue en 0, así que leer
  bytes de la ROM por puntero no cambió nada del juego en esa ventana. No cubre guardar y cargar partida.
- **Lo que cuesta es el salto indirecto (vtable + frontera FFI), no el `match` de región** → prototipar:
  sí, con el benchmark del paso 2 → si es falso: se optimiza el despacho por región en Rust (tabla
  indexada por los bits altos de la dirección) y se abandona el cambio de generador.
  → resultado del prototipo: **confirmada pero inmaterial**. El salto indirecto cuesta 2,69 ns por acceso
  frente a 0,95 ns de una lectura directa (**2,8×**), así que la suposición es cierta: lo que cuesta *es*
  el salto. El problema es el tamaño: 2,69 ns × 12.214 accesos por fotograma son 0,033 ms por fotograma
  en host, ocho veces menos de lo que el criterio daba por cierto.
- **La medida en host es un proxy válido del coste en el dispositivo.** → prototipar: no (el host
  puede subestimar el coste si en el dispositivo domina el fallo de caché por acceso) → si es falso:
  la decisión la toma el A/B del dispositivo, no el benchmark.
  → no detecta: diferencias de arquitectura de memoria entre el equipo de desarrollo y el TV.
  → resultado del prototipo: **límite detectado**. El host no puede resolver diferencias de pocos puntos
  porcentuales: entre corridas idénticas de 7200 fotogramas dispersó de 7589 a 10468 ms (±13 %). Además
  el host compila el C con TinyCC, que puede subestimar el peso relativo de un salto indirecto frente a
  un `clang -O2` en ARM64, así que la medida en dispositivo sigue pendiente.

### 7. Alcance

- **Incluye:**
  - El preámbulo y la emisión de accesos de `crates/recomp-core/src/emit.rs` (motor).
  - Los lectores y escritores de las regiones puras y la vtable en `crates/gba-core/src/` (motor).
  - La medición en host (arnés y benchmark) y en el dispositivo (telemetría).
- **No incluye:**
  - La PPU, el APU, el despacho por bloque y el camino de GPU: ya medidos y descartados o hechos.
  - La caché de líneas de la PPU entre fotogramas.
  - Publicar el motor en el repositorio oficial y re-fijar el SHA: trabajo propio, después de medir.
  - Cambiar el número o el orden de los campos de la telemetría.

### 8. Criterios de aceptación (no se modifican sin aprobación)

**Escenario: El renderizado no cambia (regresión)**
Caso de uso: cualquier cambio del camino de memoria | Tipo: regresión
  Dado el arnés del motor con la ROM y el BIOS del usuario
  Cuando se ejecutan 600, 2400 y 7200 fotogramas con el camino rápido activo
  Entonces los hashes son `31184031261ea2ec`, `b54066dc06f2b76c` y `b54066dc06f2b76c`, con
  `fallback_steps: 0` y `native_blocks` sin cambios
Test: `recomp runc --frames 600|2400|7200` (comando del apartado 9)
Estado previo: **pasa** (es el guardián, debe seguir pasando)

**Escenario: El salto indirecto es lo que cuesta**
Caso de uso: diagnóstico en host | Tipo: normal
  Dado el benchmark con el mismo patrón de direcciones sobre la misma ROM
  Cuando se comparan 10⁷ accesos por la vtable contra 10⁷ lecturas por puntero directo
  Entonces la lectura directa cuesta **menos de la mitad** que la llamada por vtable, y el total
  del fotograma en el arnés baja al menos un 10 % respecto de la misma corrida con el camino
  rápido desactivado
Test: benchmark nuevo en `crates/gba-core/tests/` más la corrida del arnés con la constante activa
y desactivada
Estado previo: **no existe el benchmark** (se escribe tras la aprobación); la corrida desactivada
debe dar el mismo tiempo que hoy

**Escenario: El residuo baja en el dispositivo**
Caso de uso: mapa táctico en el Android TV | Tipo: normal
  Dado el APK con el camino rápido y una sesión de al menos 120 s en las mismas escenas que la
  sesión 17
  Cuando se lee `blazing-perf.txt` y se compara con la sesión 17 en la ventana alineada de 160 s
  Entonces el residuo medio (`ejec − ppu − apu`) baja de 4,5 ms a **≤ 2 ms**, `bg` y `apu` se
  mantienen dentro del ±5 % y ningún segundo queda por debajo de 58 FPS
Test: revisión humana + [`analizar_telemetria.py`](../../scripts/diagnostico/analizar_telemetria.py)
Estado previo: **falla** (residuo 4,54 ms y 10 segundos de 166 por debajo de 58 FPS en la sesión 17)

**Escenario (límite): el segundo de más tráfico**
Caso de uso: el segundo con 25.425 accesos de la sesión 17 (t=143 s) | Tipo: límite
  Dado ese mismo estado de juego
  Cuando se mide con el camino rápido
  Entonces `ejec` ≤ 13 ms y no se pierde ningún fotograma
Test: revisión humana (el escenario lo elige el jugador al provocar el movimiento de unidades)
Estado previo: **falla** (`ejec` 15,3 ms y 59 FPS)

### 9. Cómo lo comprobaré

- **El renderizado no cambia** → comando:
  `../gba-recomp/gba-recomp-launcher-0.4-x86_64-pc-windows-msvc/recomp.exe runc "roms/1575 - Fire Emblem (E)(Independent).gba" --bios roms/gba_bios.bin --frames 7200`
  → pasa si: imprime `hash: b54066dc06f2b76c` y `fallback_steps: 0`, código de salida 0
  → no detecta: que el `.so` del APK lleve el cambio; eso lo cubre el empaquetado y la telemetría.
- **El salto indirecto es lo que cuesta** → comando:
  `cargo test -p gba-core --features perf-counters --bench acceso_memoria -- --nocapture` (el
  benchmark del paso 2) y `RECOMP_PPU_STATS=1 … runc --frames 7200` con la constante activa y
  desactivada, en la misma sesión y con la máquina ociosa
  → pasa si: la relación entre ambos tiempos supera el doble y la corrida con el camino rápido baja
  al menos un 10 %
  → no detecta: el coste real en el dispositivo (el host puede subestimar si allí domina el fallo
  de caché).
- **El residuo baja en el dispositivo** → revisión humana con `analizar_telemetria.py` sobre la
  telemetría de la sesión nueva, comparada con `scripts/diagnostico/log_tv15.txt` y con la sesión 17
  → pasa si: residuo ≤ 2 ms, `bg`/`apu` dentro del ±5 % y ningún segundo por debajo de 58 FPS
  → no detecta: que la escena jugada sea distinta de la de referencia; por eso se fija la ventana
  alineada y se pide la misma zona.
- **Nada más se rompe** → comando: `cargo test` en el port y `cargo test -p gba-core
  --features perf-counters` en el motor
  → pasa si: 122 en verde en el port (107 + 15) y el motor en verde, incluidos los tests de `ppu.rs`
  → no detecta: regresiones visuales que el hash no vea por casualidad en 7200 fotogramas.
- **Escenario «(visible)»** → revisión humana: jugar de 60 a 120 s en una zona que hoy cae (combate
  con animación o transición de mapa) y leer en `blazing-perf.txt` los segundos con `fps` por
  debajo de 58 y el residuo
  → no detecta: nada más; es la medida que decide.

**Blindaje por mutación (fase 4 del pipeline, previsto):**
M1 — dejar el camino rápido leyendo ROM sin excluir la ventana de EEPROM → debe morir el escenario
de regresión si el juego guarda y lee partida.
M2 — emitir el camino rápido también para MMIO → debe morir el escenario de regresión.
M3 — hacer que el camino rápido ignore el espejo de ROM → debe morir el escenario de regresión.
Los archivos se respaldan en memoria RAM y se restauran en un bloque `finally`, sin comandos de Git.

### 10. Terminado cuando

Los cuatro escenarios pasan, los tres hashes guardianes siguen intactos, la suite del port sigue en
122 verdes, el motor pasa sus tests, el benchmark existe y la sesión en el TV no registra ningún
segundo por debajo de 58 FPS con el residuo en 2 ms o menos.

### 11. Parar si

- 2 intentos fallidos seguidos con el mismo criterio: detenerse, explicar qué se probó y marcar la
  propuesta como `discrepada`.
- **La fase A desmiente la premisa** (la lectura directa no cuesta menos de la mitad que la llamada,
  o el arnés no baja del 10 % con el camino rápido activo): no se toca el generador; se documenta
  el reparto medido y se abre una propuesta nueva con el sospechoso que quede en pie.

### Correcciones previas a presentar

- **Corregido antes de presentar:** en el diagnóstico hablé de que los contadores de bus «podían ser
  un proxy del camino lento». Era infundado: al leer `emit.rs` se ve que **todos** los accesos se
  emiten como `a->read*`/`a->write*`, así que la vtable es el único camino y el contador es un
  censo. Queda escrito en el apartado 2 en vez de dejar la duda.
- **Corregido también:** mi primera sospecha fue el coste del propio contador. Un `+= 1` dentro del
  callee no explica 0,27 µs por acceso; el sospechoso es el salto indirecto más la frontera FFI. Se
  mide igualmente (paso 1) porque la telemetría no puede ser un coste oculto del que se mide a sí
  mismo.

### Cambios tras prototipos (solo si el plan cambió)

- Antes: la fase B iba a publicar en `RtApi` los punteros de **ROM, EWRAM e IWRAM** y a emitir el camino
  rápido en lecturas y escrituras → Ahora: los pasos 4-6 no se ejecutan y el prototipo se revierte
  → Por qué: el A/B del paso 1 (6 pares intercalados, mismo `.dll`) no separa el efecto del ruido del
  arnés, y el benchmark del paso 2 midió el salto indirecto en 2,69 ns por acceso, dos órdenes de
  magnitud por debajo de la atribución de 0,27 µs por acceso que fundaba la propuesta.
- Antes: el paso 1 era un A/B entre dos binarios del arnés (con y sin contadores) con una sola corrida por
  variante → Ahora: 6 pares intercalados → Por qué: la primera corrida daba una diferencia del 7,6 % que
  el ruido del arnés podía explicar por sí solo; con pares intercalados la señal es consistente (5 de 6
  pares más lentos con la instrumentación, mediana pareada +10,4 %) y **más grande que la del objeto de
  esta propuesta**, lo que convierte al contador en el sospechoso que queda en pie.

---

## Resultado (lo completa el agente después de editar)

**Discrepada en la fase A.** La fase A existía para poder matar la premisa antes de tocar el generador, y
eso es lo que pasó: **el salto indirecto no es lo que cuesta** y **no se tocó el generador**. El prototipo
se revirtió, la imagen se regeneró con los tres guardianes reverificados, y el sospechoso que queda en pie
—medido, no supuesto— pasa a
[`2026-10-12_telemetria-sin-coste-en-el-apk.md`](2026-10-12_telemetria-sin-coste-en-el-apk.md).

### Predicho vs. real

- **El renderizado no cambia (regresión)**: coincide.
  Resultado real: con el camino rápido activo, `600 → 31184031261ea2ec` (1.776.798 bloques),
  `2400 → b54066dc06f2b76c` (2.167.398) y `7200 → b54066dc06f2b76c` (3.208.998), con `fallback_steps: 0`:
  idénticos a los de antes del prototipo. Tras revertirlo y regenerar la imagen, los tres coinciden otra
  vez, con los mismos bloques.
  Método: el anunciado (`recomp runc --bios … --frames 600|2400|7200`, desde la raíz del port).
- **El salto indirecto es lo que cuesta**: difiere (la primera mitad pasa, la segunda no).
  Resultado real: la lectura directa cuesta 0,95 ns por acceso frente a 2,69 ns por la vtable (**2,8×**,
  así que sí baja de la mitad), pero el total del fotograma **no baja**: con el camino rápido activo 4 de
  6 pares fueron más rápidos (mediana pareada −286,5 ms, −3,6 %) y aun así la mediana de las corridas
  quedó 0,4 % **por encima** (más lento). El signo cambia según el estimador porque el efecto es menor que
  la dispersión del arnés (±13 %), y desde luego no es el 10 % del criterio.
- **El residuo baja en el dispositivo**: **no se intentó**. La fase B no llegó a ejecutarse (ver
  *Discrepancias*); la medida en el TV queda heredada por la propuesta nueva.

### Medidas de la fase A (datos crudos)

- **A1 — coste de la instrumentación que viaja en el APK.** Dos binarios del mismo arnés (con y sin la
  feature `perf-counters`, que el port activa en `Cargo.toml:12`), 6 pares intercalados, 7200 fotogramas,
  el mismo `.dll`, hash `b54066dc06f2b76c` y 3.208.998 bloques en las 12 corridas. Sin instrumentación:
  9943, 7315, 7168, 7276, 7105, 7256 ms (mediana 7266). Con instrumentación: 8303, 9216, 8132, 7828,
  8315, 7741 ms (mediana 8217). **Mediana pareada +758 ms → +10,4 % (media +579 ms, +7,5 %) y 5 de 6
  pares más lentos** = 0,105 ms por fotograma, más que todo el camino rápido de ROM de esta propuesta.
- **A2 — el salto indirecto, en aislamiento.** 10⁷ accesos sobre las mismas direcciones de la misma ROM y
  el mismo binario: lectura byte a byte 0,95 ns por acceso; llamada real por la vtable (`rt_read8`)
  2,69 ns por acceso. La relación es 2,8×, pero 2,69 ns × 12.214 accesos por fotograma son 0,033 ms por
  fotograma en host (≈0,45 ms con el factor host→dispositivo ≈13,5×, es decir ≈3 % del fotograma del TV),
  **ocho veces menos** que los ~3,7 ms que el criterio daba por ciertos sobre una atribución de
  0,27 µs por acceso.
- **A3 — prototipo de lectura de ROM (revertido).** El C generado pasa de 0 a 23.932 llamadas a
  `rc_read32(` con un único `a->read32` (el del propio ayudante). Equivalencia en los tres guardianes ✔
  (arriba). El control de mutación —desplazar 4 bytes el puntero de la ROM— cambia el hash a
  `c44cc0d7854fcf2f` y baja los bloques a 1.619.465: prueba de que el camino rápido se ejecuta de verdad
  y de que el A/B no estaba midiendo un no-op.
- **El A/B del criterio.** Mismo `.dll` en ambos casos, 6 pares intercalados: desactivado 10468, 7848,
  8757, 7796, 7589, 7743 ms (mediana 7822); activo 8189, 7985, 7919, 7402, 7786, 7564 ms (mediana 7852,5).
  Mediana pareada −286,5 ms (−3,6 %), media pareada −559 ms (−6,8 %), medianas +0,4 %.

### Contexto: ¿se respetó?

- El renderizado no cambia → respetada: los tres guardianes son idénticos con el camino rápido activo y,
  en el binario final, vuelven a coincidir tras revertir.
- La telemetría decide, no la intuición → respetada: la premisa murió por medida y el criterio de tiempo se
  aplicó tal como estaba escrito, sin reescribirlo a posteriori para que pasara.
- El motor se desarrolla en local y el port lo consume por `[patch]`, con el SHA sin publicar → respetada:
  no se publicó nada y **no queda ningún cambio del motor** (su `git status` muestra solo el `.gitignore`
  preexistente y el directorio del launcher).
- ¿Se tocó algo que alguna restricción prohibía? no.
- Imprevistos que surgieron: el prototipo dejó `out/` con un C y un `.dll` compilados contra un `RtApi` con
  tres campos de más, incompatibles con el motor revertido; y esta máquina no tiene `cc`/`gcc`/`clang`, así
  que la imagen se forja con el TinyCC embebido del launcher. → criterio de prioridad aplicado: sí
  (primero la equivalencia y la integridad del repositorio; la imagen se regeneró y los guardianes se
  reverificaron después).
- ¿Reapareció alguna alternativa descartada? no.

### Archivos modificados

Motor (fase A, **todo revertido**; el diff del prototipo está en el apéndice):
`crates/recomp-core/src/emit.rs`, `crates/gba-core/src/capi.rs`, `crates/gba-core/src/mem.rs`,
`crates/recomp/src/main.rs`, y el benchmark temporal `crates/gba-core/tests/acceso_memoria_bench.rs`
(borrado). Port: esta propuesta, la nueva, [`analisis_rendimiento.md`](../explicacion/analisis_rendimiento.md)
y [`medir-rendimiento.md`](../guias/medir-rendimiento.md).
¿Coincide con lo anunciado? sí, con un matiz: lo que se tocó en el motor **no se queda**, que es lo que la
fase A debía garantizar.

### Discrepancias

- El criterio pedía que el fotograma del arnés bajara un 10 % y lo medido es ≤6 %, sin separarse del ruido
  → **error de juicio**: el listón se fijó sobre una atribución (0,27 µs por acceso) que la propia fase A
  desmintió (2,69 ns por acceso en aislamiento). El techo del premio era ≈0,45 ms por fotograma en el TV,
  así que pedir un 10 % del fotograma era inalcanzable por construcción: el criterio pedía el 10 % del
  total a una fracción del camino de acceso.
- La corrida suelta del arnés dispersa ±13 %: con los 3 pares del primer intento el resultado no era
  interpretable → **información faltante** (la dispersión no se conocía hasta medirla).
- El A/B podía estar midiendo un no-op si el camino rápido no llegaba al C generado → **omisión**,
  corregida con la mutación deliberada del puntero de ROM.

### Para la próxima vez

Antes de escribir un criterio de tiempo, calcular el techo del premio (coste por unidad × número de
unidades × factor host→dispositivo) y poner el listón por debajo de ese techo; y medir siempre con pares
intercalados (≥5), porque una corrida suelta de este arnés dispersa ±13 %.

### Fuera de alcance (hallazgos que no se arreglaron)

- **La instrumentación que viaja en el APK** (`Cargo.toml:12` activa `perf-counters`) cuesta 10,4 % del
  fotograma en host, medido con 6 pares intercalados. Es el sospechoso que queda en pie y abre la
  propuesta nueva. | impacto: alto
- El arnés en host dispersa ±13 % entre corridas idénticas: ningún A/B de menos de ~5 % es interpretable
  sin pares intercalados. | impacto: medio
- No hay `cc`/`gcc`/`clang` en esta máquina: el host compila el C con TinyCC, que puede subestimar el peso
  relativo de un salto indirecto frente a un `clang -O2` en ARM64. | impacto: medio
- El camino rápido de ROM queda revertido y medido (≤6 %, dentro del ruido): no descartado para siempre,
  con el diff reproducible en el apéndice. | impacto: bajo
- Las 8 emisiones de **escritura** y las lecturas de EWRAM/IWRAM siguen por la vtable, sin prototipar: el
  prototipo cubrió solo la lectura de ROM. | impacto: bajo
- El `.dll` (174 MB, 12 unidades) se forja entero en cada prueba: cada A/B paga minutos de compilación
  antes de medir 8 segundos. | impacto: bajo

### Apéndice: el diff del prototipo (revertido, reproducible)

Los cuatro cambios del motor, con nombres reales, para poder rehacer el experimento el día que haya un
instrumento con menos ruido o medida en dispositivo:

1. `crates/gba-core/src/capi.rs` — tres campos al final de `RtApi` y una tabla por máquina:

```c
    const uint8_t* rom;   /* imagen del cartucho; NULL = sin camino rápido */
    uint32_t rom_len;
    uint32_t rom_flags;   /* bit0 = reloj RTC adjunto, bit1 = EEPROM adjunta */
```

   más `RtApi::for_bus(&MemMap)`, que devuelve la misma tabla de punteros con `rom`/`rom_len`/`rom_flags`
   tomados del bus (el `Vec` de la ROM nunca se reasigna después de `MemMap::new`). `RT_API` deja `rom`
   en `NULL`, así que un host que no llama a `for_bus` mantiene el comportamiento anterior intacto.
2. `crates/gba-core/src/mem.rs` — `MemMap::rom_fast_flags()`, que responde si el cartucho tiene RTC
   (`0x080000C4..C9` es un protocolo, no ROM) o EEPROM (todo el espejo `0x0D` habla el protocolo de bits).
3. `crates/recomp-core/src/emit.rs` — en el preámbulo C, `rc_rom_fast()` (región 8..13, dentro de
   `rom_len`, sin las ventanas anteriores) más `rc_read8/16/32`, que cargan del puntero si el acceso es ROM
   pura y caen a `a->read8/16/32` en cualquier otro caso; RAM, VRAM, paleta, OAM, SRAM/Flash, GPIO/EEPROM y
   espejos quedan **deliberadamente** fuera. Los 8 puntos de emisión: las cinco variantes de carga de
   `inline_body()`, el `LDM` inlineado, el `LDM` de `emit_block()` y el retorno por `POP`.
4. `crates/recomp/src/main.rs` — el interruptor `RECOMP_FAST_ROM` en los dos sitios que llaman a los
   bloques (`run_frame_native()` y el bucle de coste de `runc`): con la variable puesta se pasa
   `RtApi::for_bus(&m.bus)` y sin ella `RT_API`. **El mismo `.dll` sirve para las dos ramas**, que es lo
   que hace limpio el A/B.

El control de mutación usado para probar que el camino se ejecuta: en `for_bus`, `bus.rom.as_ptr()` →
`bus.rom.as_ptr().add(4)` (hash `c44cc0d7854fcf2f`, bloques 1.619.465).

---

## Discrepancia (solo si el estado es "discrepada")

Abierta por: agente
Punto afectado: *El salto indirecto es lo que cuesta* (segundo tramo: el total del fotograma baja al menos
un 10 %) y, en consecuencia, *El residuo baja en el dispositivo*, que no se intentó.
Motivo: el efecto medido del salto indirecto (2,69 ns por acceso en aislamiento; ≤6 % en el arnés, dentro
del ruido de ±13 %) es dos órdenes de magnitud menor que la atribución de 0,27 µs por acceso con la que se
escribió el criterio. El residuo del dispositivo no es de despacho de memoria.
Clasificación: suposición falsa
Propuesta de corrección: [`2026-10-12_telemetria-sin-coste-en-el-apk.md`](2026-10-12_telemetria-sin-coste-en-el-apk.md),
que ataca lo que sí se midió: la instrumentación que viaja en el APK.

---

## Revisión humana
<!-- El agente SÍ edita esta sección: escribe aquí los escenarios que no se
     pudieron comprobar con código ("(visible)" o "no pudo comprobarse") y las restricciones
     marcadas "no testeable",
     uno por línea, dejando las marcas ✔ | ✘ | ? sin elegir.
     El agente NO elige marca, NO escribe la nota y NO rellena
     "Detectado solo por el humano": eso es solo de la persona. -->

Ninguno: no queda nada que revisar de esta propuesta, porque la fase B no se ejecutó y el prototipo se
revirtió (ver *Resultado* y *Discrepancia*). Los escenarios de dispositivo que aquí estaban pendientes
—el residuo en el TV, el segundo de más tráfico, la ventana de EEPROM al guardar partida y la publicación
del motor— pasan a la propuesta nueva, que los declara con sus propios criterios.

- Detectado solo por el humano: [lo completa la persona]
