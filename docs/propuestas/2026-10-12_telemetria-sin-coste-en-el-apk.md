# Propuesta: La telemetría que viaja en el APK deja de costar el 10 % del fotograma

Fecha: 2026-10-12
Nivel: 2
Estado: discrepada

<!-- Estados: pendiente | aprobada | modificada | descartada |
     verificada por agente | revisada por humano | discrepada | cerrada -->

## Propuesta (la escribe el agente antes de editar)

### 1. Qué cambia

La instrumentación que hoy viaja encendida en el APK —los contadores de acceso del bus y los
cronometrajes por línea de la PPU— deja de costar un 10 % del fotograma **sin perder ningún campo de la
telemetría**. Primero se descompone de dónde sale ese coste (contadores del bus, cronómetros de la PPU o
el propio `take_perf_counters`) y después se repara con la pieza que resulte culpable: contadores que no
interfieran con el camino caliente, cronómetros más baratos, o —si ninguna reparación llega al listón— un
interruptor de tiempo de ejecución, apagado por defecto, que el port enciende solo cuando pide una sesión
de diagnóstico.

### 2. Contexto

- Contexto consultado: la propuesta cerrada
  [`2026-10-12_acceso-memoria-sin-llamada-indirecta.md`](2026-10-12_acceso-memoria-sin-llamada-indirecta.md)
  (su A1 es el origen de esta), la telemetría de las sesiones 13-17 del Android TV analizada con
  [`analizar_telemetria.py`](../../scripts/diagnostico/analizar_telemetria.py); del motor:
  `crates/gba-core/src/mem.rs` (12 sitios de `cfg(feature = "perf-counters")`, con los seis incrementos de
  `reads`/`writes` en la cabecera de `read8/16/32` y `write8/16/32`), `crates/gba-core/src/ppu.rs` (9
  sitios, cuatro cronómetros por línea visible), `crates/gba-core/src/perf_clock.rs` (el reloj barato del
  que ya se benefició la PPU), `crates/gba-core/src/machine.rs` (`take_perf_counters`),
  `crates/gba-core/tests/perf_counters_tests.rs`; del port:
  [`Cargo.toml`](../../Cargo.toml) (línea 12, `features = ["perf-counters"]`),
  [`src/runner.rs`](../../src/runner.rs) (`take_perf_counters()` en cada fotograma, línea 134),
  [`src/perf.rs`](../../src/perf.rs) (`FrameTiming` y las columnas del `.txt`),
  [`src/platform.rs`](../../src/platform.rs) (`init_perf_file`, línea 1372),
  [`docs/explicacion/analisis_rendimiento.md`](../explicacion/analisis_rendimiento.md) y
  [`docs/guias/medir-rendimiento.md`](../guias/medir-rendimiento.md).
- Intención: que el metro no se estire. Hoy el proyecto mide `ejec` —y decide con él— con una
  instrumentación que infla `ejec`, y el tamaño de esa inflación no se conocía. Medida ya en el A1 de la
  propuesta cerrada: **+10,4 % de mediana pareada** (5 de 6 pares más lentos con la instrumentación puesta,
  mismo `.dll`, 7200 fotogramas).
- Restricciones y decisiones previas (máx. 3):
  - **Ningún campo de la telemetría cambia de significado ni de formato** (origen: las diecisiete sesiones
    ya medidas y el script que las lee) → implica: el arreglo es de *coste*, no de datos; si el arreglo
    necesita un interruptor, apagado debe dejar los campos en cero y encendido debe devolver los mismos
    números → protegida por: *La telemetría no pierde ni un campo*.
  - El renderizado no cambia (origen: el arnés del motor y los tres hashes guardianes) → implica: puede
    reordenarse la instrumentación, pero el búfer de fotograma debe terminar idéntico → protegida por:
    *El renderizado no cambia (regresión)*.
  - En host el arnés dispersa ±13 % entre corridas idénticas (origen: la propuesta cerrada, medido) →
    implica: el listón del 1 % exige pares intercalados y el mismo `.dll`; ninguna corrida suelta sirve
    → protegida por: *La instrumentación deja de costar*.
- Alternativas descartadas:
  - **Quitar la feature del APK y listo** → descartada como primera opción: deja al proyecto ciego justo
    cuando hay caídas que explicar, y la telemetría del dispositivo es el único instrumento que tiene.
    Queda como red si la reparación no llega al listón.
  - **Atribuir el coste al `+= 1`** → descartada: el A1 implica 8,6 ns por acceso (≈0,105 ms por fotograma
    entre 12.214 accesos), entre veinte y treinta veces el coste de un incremento sobre una línea caliente.
    El sospechoso es la interferencia con el camino caliente (línea fría propia, o la tienda serializando
    las cargas del accesor), no la suma.
  - **Muestrear la telemetría (1 de cada N fotogramas)** → descartada por ahora: cambia la varianza de
    series que ya están medidas en diecisiete sesiones y rompería la comparabilidad con ellas.
- Ante imprevistos, prioriza: que los campos sigan significando lo mismo por encima de la ganancia.

### 3. Ejemplos

- **Caso normal** (fotograma típico de la sesión 17: 12.214 accesos y 640 lecturas de reloj de la PPU):
  antes: la instrumentación cuesta +10,4 % (mediana pareada de 6 pares, 0,105 ms por fotograma en host);
  ahora (objetivo): el mismo fotograma con la instrumentación puesta cuesta ≤1 % más que sin ella.
- **Caso límite** (el segundo de más tráfico medido: `lecturas` 16.598 + `escrituras` 8.827 = 25.425
  accesos, sesión 17 t=143 s):
  antes: `ejec` 15,3 ms y 59 FPS;
  ahora (objetivo): `ejec` ≤ 14,6 ms y 60 FPS.
- **Caso de instrumentación** (una sesión de diagnóstico con la telemetría encendida):
  antes: el `.txt` trae los valores de siempre y nadie sabía que estaban medidos con un metro estirado;
  ahora (objetivo): el `.txt` trae los mismos campos, con los mismos valores dentro del ±5 %, y el script
  los procesa sin cambios.

### 4. Pasos

**Fase A — descomponer, sin cambiar la semántica.**

1. Tres variantes temporales del motor, solo para medir (y se revierten): (a) con los seis incrementos del
   bus y sin los cronómetros de la PPU, (b) al revés, (c) ambas (= el APK de hoy). A/B intercalado de seis
   pares contra la variante sin instrumentación, mismo `.dll`: reparto exacto del 10,4 %.
2. Prueba de la hipótesis de interferencia: mover los contadores fuera del `MemMap` (estáticos con línea
   propia) y repetir el mismo A/B. Si el coste no baja, la hipótesis muere y queda el interruptor.

**Fase B — reparar, según lo que haya dominado.**

3. Si domina el bus: contadores que no interfieran con las cargas del accesor (estáticos de línea propia, o
   un acumulador que el compilador pueda solapar con el acceso).
4. Si dominan los cronómetros: contar capas por línea en lugar de leer el reloj cuatro veces por línea (el
   desglose de la PPU ya tiene sus propios números medidos).
5. Si nada llega al listón: interruptor de tiempo de ejecución en el bus (`telemetria: bool`), **apagado
   por defecto**, que el port enciende cuando pide una sesión de diagnóstico; y se documenta el offset de
   una sesión instrumentada. Hoy ese offset existe y es de un 10 % que nadie conocía.
6. Publicar el motor y re-fijar el SHA; el port expone el interruptor y su documentación.

### 5. Qué más podría verse afectado

Búsquedas hechas: `grep -rn "perf-counters" crates/ --include=*.rs` (12 en `mem.rs`, 9 en `ppu.rs`, 1 en
`machine.rs`, 2 en `lib.rs`, 2 en `recomp/src/main.rs`, más los tests), `grep -rn "take_perf_counters"` en
motor y port, `grep -rn "perf_counters.reads|writes"` y `grep -rn "blazing-perf" src/ scripts/ docs/`.

- `crates/gba-core/src/mem.rs` — `perf_counters` es un campo de `MemMap` (línea 159) y los seis
  incrementos están en la cabecera de cada accesor: es el camino más caliente del motor.
- `crates/gba-core/src/ppu.rs` — nueve sitios de la feature, cuatro cronómetros por línea visible (640
  lecturas de reloj por fotograma).
- `crates/gba-core/tests/perf_counters_tests.rs` — 69 líneas que fijan qué cuentan los contadores y que
  `take_perf_counters` los reinicia: cualquier cambio de mecanismo tiene que dejarlo verde.
- [`src/runner.rs`](../../src/runner.rs) — `take_perf_counters()` se llama **en cada fotograma**, sin
  condición; si el arreglo pasa por un interruptor, aquí se decide.
- [`src/perf.rs`](../../src/perf.rs) y [`src/platform.rs`](../../src/platform.rs) — `FrameTiming` define
  las columnas del `.txt` y `init_perf_file` decide si hay archivo; **no existe hoy ningún interruptor de
  entorno en el port** (no hay ni un `env::var`), así que «encender la telemetría» no es una decisión
  actual: es el estado por defecto siempre que haya un directorio escribible.
- [`tests/packaging_tests.rs`](../../tests/packaging_tests.rs) — el APK puede cambiar de tamaño; hay
  pruebas de empaquetado que asumen conteos.
- Sin más referencias encontradas.

### 6. Suposiciones

- **El coste está en la interferencia de los contadores con el camino caliente, no en sumar** → prototipar:
  sí (paso 2) → si es falso: el interruptor de tiempo de ejecución queda como red, con el offset medido y
  documentado.
  → resultado del prototipo: **falsa**. El prototipo del paso 2 (contadores del bus en un estático con
  línea propia) no quita nada: `Vd-static` sigue pagando +7,3 % (6/6 pares más lentos) frente al +8,6 % de
  `Vc`, dentro del ruido del arnés. No había interferencia que quitar porque el grupo del bus no cuesta
  nada en ninguna de las dos posiciones (−0,7 % con los contadores dentro de `MemMap`).
- **Los cronómetros de la PPU son una fracción menor del 10,4 %** (unas 640 lecturas de reloj por fotograma
  a ~30 ns son ≈0,02-0,03 ms de los 0,105 ms medidos) → prototipar: sí (paso 1) → si es falso: se ataca
  primero la PPU y los contadores del bus pasan a segundo lugar.
  → resultado del prototipo: **falsa, y en la dirección inesperada**. Los cronómetros no son una fracción
  menor: son *todo* el efecto (+643,9 ms, +10,1 %, 5/6 pares más lentos), unos 89,4 µs por fotograma a
  ~140 ns por lectura de reloj. La estimación de ~30 ns por lectura era cinco veces corta.
- **Un interruptor apagado por defecto es aceptable para el proyecto** → prototipar: no (es una decisión de
  producto, y el port no tiene hoy ningún mecanismo equivalente) → si es falso: se implementa solo la
  reparación de coste y se acepta el offset.
  → no detecta: que la sesión de diagnóstico se haga con el interruptor encendido y el número medido vuelva
  a estar inflado; quedará como offset conocido en la documentación.

### 7. Alcance

- **Incluye:**
  - Los contadores de `crates/gba-core/src/mem.rs` y los cronómetros de `crates/gba-core/src/ppu.rs` (motor).
  - `take_perf_counters` y el mecanismo del interruptor, si hace falta (motor).
  - La exposición del interruptor y la documentación del offset (port).
  - La medición en host (A/B intercalado) y en el dispositivo (telemetría).
- **No incluye:**
  - Cambiar qué campos se publican, su formato o el orden de las columnas del `.txt`.
  - `perf_clock` (ya es barato) y la telemetría de GPU/`present` (ya medida y descartada como limitante).
  - Publicar el motor en el repositorio oficial y re-fijar el SHA: trabajo propio, al final.

### 8. Criterios de aceptación (no se modifican sin aprobación)

**Escenario: El renderizado no cambia (regresión)**
Caso de uso: cualquier cambio de la instrumentación | Tipo: regresión
  Dado el arnés del motor con la ROM y el BIOS del usuario
  Cuando se ejecutan 600, 2400 y 7200 fotogramas
  Entonces los hashes son `31184031261ea2ec`, `b54066dc06f2b76c` y `b54066dc06f2b76c`, con `fallback_steps: 0`
  y `native_blocks` en 1.776.798 / 2.167.398 / 3.208.998
Test: `recomp runc --bios roms/gba_bios.bin --frames 600|2400|7200` desde la raíz del port
Estado previo: **pasa** (es el guardián, debe seguir pasando)

**Escenario: La instrumentación deja de costar**
Caso de uso: el fotograma típico del TV | Tipo: normal
  Dados los dos binarios del mismo arnés (con la instrumentación reparada y sin la feature) y el mismo `.dll`
  Cuando se corren seis pares intercalados de 7200 fotogramas y se comparan los tiempos pareados
  Entonces la mediana pareada no supera el **1 %** (hoy: 10,4 % entre binarios, 5 de 6 pares más lentos)
Test: el bucle de pares intercalados del apartado 9
Estado previo: **falla** (mediana pareada +758 ms de 7200, +10,4 %)

**Escenario: La telemetría no pierde ni un campo**
Caso de uso: una sesión de diagnóstico en el TV | Tipo: normal
  Dada una sesión con la instrumentación encendida en las mismas escenas de la sesión 17
  Cuando se comparan los campos del `.txt` con los de esa sesión
  Entonces las columnas son las mismas, `lecturas`/`escrituras` caen dentro del ±5 %, `ppu_ms`/`apu_ms`
  dentro del ±10 %, y [`analizar_telemetria.py`](../../scripts/diagnostico/analizar_telemetria.py) procesa
  el archivo sin cambios
Test: revisión humana + el script sobre el log nuevo
Estado previo: **pasa** (es el comportamiento de hoy; el riesgo lo introduce el cambio)

**Escenario: El residuo baja en el dispositivo**
Caso de uso: mapa táctico en el Android TV | Tipo: normal
  Dado el APK con la instrumentación reparada y una sesión de al menos 120 s en las mismas escenas
  Cuando se lee `blazing-perf.txt` y se compara con la sesión 17 en la ventana alineada de 160 s
  Entonces el residuo medio (`ejec − ppu − apu`) baja de 4,54 ms a **≤ 3,4 ms**, `bg` y `apu` se mantienen
  dentro del ±5 % y no queda ningún segundo por debajo de 58 FPS
Test: revisión humana + [`analizar_telemetria.py`](../../scripts/diagnostico/analizar_telemetria.py)
Estado previo: **falla** (residuo 4,54 ms y 10 segundos de 166 por debajo de 58 FPS en la sesión 17)
Nota: el criterio pide **la mitad** del efecto de host escalado (≈1,4 ms), con margen, porque el reparto
entre contadores del bus y cronómetros no se conoce hasta la fase A.

**Escenario (límite): el segundo de más tráfico**
Caso de uso: el segundo con 25.425 accesos de la sesión 17 (t=143 s) | Tipo: límite
  Dado ese mismo estado de juego
  Cuando se mide con la instrumentación reparada
  Entonces `ejec` ≤ 14,6 ms y no se pierde ningún fotograma
Test: revisión humana (el escenario lo elige el jugador al provocar el movimiento de unidades)
Estado previo: **falla** (`ejec` 15,3 ms y 59 FPS)

### 9. Cómo lo comprobaré

- **El renderizado no cambia** → comando:
  `../gba-recomp/target/release/recomp.exe runc "roms/1575 - Fire Emblem (E)(Independent).gba" --bios roms/gba_bios.bin --frames 7200`
  → pasa si: imprime `hash: b54066dc06f2b76c`, `native_blocks: 3208998` y `fallback_steps: 0`, código 0
  → no detecta: que el `.so` del APK lleve el cambio; eso lo cubre el empaquetado y la telemetría.
- **La instrumentación deja de costar** → comando: dos binarios del arnés
  (`cargo build --release -p recomp` y `… --features gba-core/perf-counters`), copiados aparte, y el bucle de
  seis pares intercalados de `runc --frames 7200` contra el mismo `.dll`
  → pasa si: la mediana pareada ≤1 % y la dispersión de los pares no cambia de signo entre estimadores
  → no detecta: el efecto en el dispositivo (el host compila el C con TinyCC, que no es `clang -O2` en ARM64).
- **La telemetría no pierde ni un campo** → comando: `python scripts/diagnostico/analizar_telemetria.py
  --archivo <log nuevo> --top 5` comparado con el mismo comando sobre `scripts/diagnostico/log_tv15.txt`
  → pasa si: el desglose por fases y los contadores salen con los mismos campos y valores comparables
  → no detecta: que la escena jugada sea otra; por eso se alinea la ventana.
- **Nada más se rompe** → comando: `cargo test` en el port (122 en verde) y
  `cargo test -p gba-core --features perf-counters` en el motor (incluye `perf_counters_tests.rs`)
  → pasa si: todo en verde en ambos repositorios
  → no detecta: regresiones visuales que el hash no vea por casualidad en 7200 fotogramas.

**Blindaje por mutación (fase 4 del pipeline, previsto):**
M1 — dejar un contador sin incrementar → debe morir el escenario de telemetría (los campos cambian).
M2 — hacer que el interruptor apagado no detenga los cronómetros → debe morir el escenario de coste.
M3 — cambiar el orden o el nombre de una columna del `.txt` → debe morir el escenario de telemetría.
Los archivos se respaldan en memoria RAM y se restauran en un bloque `finally`, sin comandos de Git.

### 10. Terminado cuando

La descomposición está medida y escrita, la reparación elegida deja la instrumentación en ≤1 % con los
mismos campos, los tres hashes guardianes siguen intactos, la suite del port sigue en 122 verdes, el motor
pasa sus tests, y la sesión en el TV baja el residuo a 3,4 ms o menos sin ningún segundo por debajo de
58 FPS.

### 11. Parar si

- 2 intentos fallidos seguidos con el mismo criterio: detenerse, explicar qué se probó y marcar la
  propuesta como `discrepada`.
- **La descomposición muestra que el coste no está en los contadores del bus ni en los cronómetros de la
  PPU** (por ejemplo, está en el `take_perf_counters` por fotograma o en el `Instant::now` del port): no se
  sigue tocando a ciegas; se documenta el reparto medido y se abre otra propuesta con lo que quede.
- Si ninguna reparación llega al listón del 1 %, **no se encadenan intentos**: se implementa el interruptor
  (apagado por defecto), se documenta el offset medido de una sesión instrumentada y se cierra la propuesta
  con lo que se haya ganado.

### Correcciones previas a presentar

- **Corregido antes de presentar:** al escribir esta propuesta descubrí que el port **no tiene ni un solo
  `env::var`**: no existe ningún interruptor de diagnóstico y la telemetría se escribe siempre que
  `init_perf_file` encuentre un directorio escribible (`src/platform.rs:1372`). Es decir, la opción del
  interruptor necesita un **mecanismo nuevo**, no uno existente, y eso forma parte del alcance.
- **Corregido también:** mi primera lectura del A1 atribuía todo el coste a los seis incrementos del bus.
  Con el reloj a ~30 ns por lectura y 640 lecturas por fotograma, los cronómetros de la PPU explican
  ≈0,02-0,03 ms de los 0,105 medidos, así que el reparto no está cerrado: es justamente el paso 1.

### Cambios tras prototipos (solo si el plan cambió)

- Antes: el paso 2 existía para separar "interferencia" de "suma" en los contadores del bus → Ahora: se
  ejecuta igual, pero su lectura es la de un nulo, no la de una reparación → Por qué: el paso 1 midió el
  grupo del bus en −0,7 % (dentro del ruido), así que no había coste que la reubicación pudiera quitar.
  Se ejecutó para dejar constancia, y confirmó el nulo (+7,3 % con el estático delante, 6/6 pares lentos).
- Antes: el paso 4 proponía "contar capas por línea en lugar de leer el reloj cuatro veces" → Ahora: se
  descarta → Por qué: cambiaría el significado de `bg_ns`/`sprite_ns`/`compose_ns` (una de las tres
  restricciones de la propuesta) y el coste no está en el número de campos sino en el precio de cada
  lectura de reloj.
- Antes: el paso 5 ofrecía un interruptor de tiempo de ejecución apagado por defecto como red → Ahora: se
  descarta → Por qué: el grupo del bus cuesta ~0 (un interruptor sobre él no compra nada) y el grupo que
  cuesta es el reloj de la PPU, que en el target principal (`aarch64`) ya es `CNTVCT_EL0`. Un interruptor
  apagado por defecto para un coste que el APK principal no paga es complejidad sin ganancia.

---

## Resultado (lo completa el agente después de editar)

La fase A está medida y la fase B no se ejecutó: el reparto medido desmiente la premisa de la propuesta.
El coste no está en los contadores del bus que viajan en el APK, sino en el reloj que cronometra la PPU, y
solo fuera de `aarch64`. Se para según la regla del apartado 11 (el coste resulta ser el reloj) y se abre
[`2026-10-13_el-reloj-de-la-ppu-en-el-dispositivo.md`](2026-10-13_el-reloj-de-la-ppu-en-el-dispositivo.md).

### Predicho vs. real

- **El renderizado no cambia (regresión)**: coincide.
  Resultado real: los tres guardianes idénticos en las **cuatro** variantes y en las 42 corridas del
  experimento —600 fotogramas → `31184031261ea2ec` con 1.776.798 bloques; 2400 → `b54066dc06f2b76c` con
  2.167.398; 7200 → `b54066dc06f2b76c` con 3.208.998—, siempre con `fallback_steps: 0`. La imagen
  (`out/…-bios.dll`, sha256 `639335e235cbd0e4…`) no cambió entre la primera y la última corrida.
  Método: el anunciado.
- **La instrumentación deja de costar (≤1 %)**: no pudo comprobarse, y ya no se intenta.
  Resultado real: el reparto (abajo) dice que el 10,4 % no es de los contadores del bus; con el paso 5
  descartado, no queda reparación que perseguir en esta propuesta.
- **La telemetría no pierde ni un campo**: no pudo comprobarse (no se cambió nada del `.txt`; el escenario
  lo hereda la propuesta nueva, que no toca campos).
- **El residuo baja en el dispositivo** (4,54 → ≤3,4 ms) y **el caso límite** (`ejec` ≤ 14,6 ms): no se
  intentaron (fase B no ejecutada).

### El reparto medido (fase A, paso 1)

Seis ciclos intercalados de `V0-base`, `Va-bus`, `Vb-hw` y `Vc-ambas` a 7200 fotogramas, el mismo `.dll`,
con el guardián comprobado en cada corrida. La identidad de cada variante no se supone: se comprueba con
`RECOMP_PPU_STATS=1` a 600 fotogramas, donde `Va` sale con los cuatro cronómetros a `0.000 ms` y los
contadores vivos (`reads=3305.4`, `writes=2863.4` por fotograma) y `Vb` al revés (`reads=0.0`,
`writes=0.0`, `ppu=0.531`).

| Variante | Qué enciende | Mediana pareada vs. `V0` | Pares más lentos |
| :-- | :-- | --: | --: |
| `Va-bus` | los seis incrementos del bus | **−44,4 ms (−0,7 %)** | 2/6 |
| `Vb-hw` | los cronómetros PPU+APU | **+643,9 ms (+10,1 %)** | 5/6 |
| `Vc-ambas` | las dos (= el APK de hoy) | +570,5 ms (+8,9 %) | 5/6 |

Reparto aditivo: `Va + Vb` = +599,5 ms contra `Vc` = +570,5 ms → **residuo −29,0 ms (−0,5 %)**, dentro del
ruido. Mediana de `V0` = 6382,2 ms.

**El paso 2 mata la hipótesis de interferencia.** Con los contadores del bus movidos a un estático con
línea propia (`#[repr(align(64))]`, plegado en `take_perf_counters`), otro A/B intercalado —ahora `V0`,
`Vc` y `Vd`— deja al prototipo **pagando lo mismo**: `Vc` +551,9 ms (+8,6 %, 6/6 más lentos) y
`Vd-static` +467,9 ms (+7,3 %, 6/6). La diferencia entre `Vc` y `Vd` (−84 ms, −1,3 %) no se separa de la
dispersión del arnés (±13 %), así que mover los contadores no quita nada: no había nada que quitar.

**El mecanismo del 10,1 % es el reloj, no el contador.** 643,9 ms entre 7200 fotogramas son 89,4 µs por
fotograma; la PPU toma cuatro marcas por línea visible (640 lecturas por fotograma), así que cada lectura
sale a **~140 ns** —casi cinco veces los ~30 ns que documenta
`crates/gba-core/src/perf_clock.rs` para su camino
genérico—. Ese camino genérico es `Instant::now()`, y `perf_clock` **solo usa `CNTVCT_EL0` bajo
`#[cfg(target_arch = "aarch64")]`**: en el arnés de host, en `armeabi-v7a` y en `x86_64` el cronómetro de la
PPU es `Instant::now()`, el mismo reloj que el port mide en 1,4-2,2 µs en esa familia de TV. El +10,4 % que
arrastraba la propuesta **no lo paga el APK de `arm64-v8a`** (el target principal del proyecto): lo paga
quien compile los otros dos ABIs, y lo paga el propio arnés de host al medirse.

### Contexto: ¿se respetó?

- **Ningún campo de la telemetría cambia de significado ni de formato** → respetada: no se tocó
  [`src/perf.rs`](../../src/perf.rs) ni la lista de columnas; los prototipos fueron del motor y se
  revirtieron.
- **El renderizado no cambia** → respetada: tres guardianes idénticos en las cuatro variantes.
- **En host el arnés dispersa ±13 %: hacen falta pares intercalados** → respetada: seis ciclos intercalados
  por experimento, medianas pareadas y comprobación de que el reparto es aditivo.
- ¿Se tocó algo que alguna restricción prohibía? No: las ediciones al motor vivieron dentro de un respaldo en
  RAM y una restauración en un bloque `finally`, con el sha256 de `mem.rs` y `ppu.rs` comprobado tras cada
  prototipo (`5930117b49fc9814` y `8824cec521cff709`), sin un solo comando de Git.
- Imprevistos que surgieron: el ancla del parche del paso 2 falló una vez por el fin de línea CRLF del motor
  → criterio de prioridad aplicado: sí (se corrigió el parche, no la premisa).
- ¿Reapareció alguna alternativa descartada? Sí, el interruptor (paso 5), y se vuelve a descartar, ahora con
  números.

### Archivos modificados

- Motor (`../gba-recomp`): `crates/gba-core/src/mem.rs` y `crates/gba-core/src/ppu.rs` — **modificados y
  restaurados** (idénticos, sha256 comprobado); no queda ningún cambio en el árbol del motor.
- Port: esta propuesta, la propuesta de corrección y las dos páginas de documentación.

¿Coincide con lo anunciado? Sí: el alcance declarado (contadores de `mem.rs`, cronómetros de `ppu.rs`,
`take_perf_counters` y la medición en host) es exactamente lo que se tocó.

### Discrepancias

- La propuesta se escribió dando por hecho que la instrumentación que viaja en el APK cuesta ~10 % del
  fotograma → **suposición falsa**: ese 10,4 % pertenece al reloj de la PPU en el camino genérico de
  `perf_clock` (host, `armeabi-v7a`, `x86_64`), no a lo que ejecuta el APK de `arm64-v8a`.
- El paso 2 se escribió para separar "interferencia" de "suma" → **omisión**: la medición directa del paso 1
  (grupo del bus a −0,7 %) ya lo dejaba sin objeto; se ejecutó igual para dejar constancia y confirmó el nulo.

### Para la próxima vez

Antes de escribir "eso lo paga el APK", comprobar qué camino de plataforma toma el código que se mide
(incluidos los `#[cfg(target_arch)]`): un `cfg` de arquitectura convirtió un presupuesto de host en una
acusación al paquete.

### Fuera de alcance (hallazgos que no se arreglaron)

- `perf_clock.rs` documenta ~30 ns para su camino
  genérico y la medición lo deja en ~140 ns; ese camino lo usan `armeabi-v7a` y `x86_64`, que
  [`package_apk.ps1`](../../scripts/package_apk.ps1) construye | impacto: medio.
- El coste de `CNTVCT_EL0` en el dispositivo sigue sin medir: la sonda `reloj=` del port mide
  `Instant::now()`, no el reloj del motor | impacto: medio.
- El arnés de host se cronometra a sí mismo: toda comparación A/B del proyecto arrastra ~8-10 % de reloj |
  impacto: medio.

---

## Discrepancia (solo si el estado es "discrepada")

Abierta por: agente
Punto afectado: la premisa de la propuesta (*la instrumentación que viaja en el APK cuesta ~10 % del
fotograma*) y, con ella, los pasos 3-5 y el escenario *La instrumentación deja de costar*.
Motivo: el reparto medido deja el grupo del bus en −0,7 % (dentro del ruido) y el grupo de cronómetros en
+10,1 %, y el prototipo de estáticos del paso 2 no quita nada. El coste es la lectura de reloj del camino
genérico de `perf_clock` (`Instant::now()` fuera de `aarch64`), no los contadores que el APK de `arm64-v8a`
paga.
Clasificación: suposición falsa
Propuesta de corrección: [`2026-10-13_el-reloj-de-la-ppu-en-el-dispositivo.md`](2026-10-13_el-reloj-de-la-ppu-en-el-dispositivo.md)

---

## Revisión humana
<!-- El agente SÍ edita esta sección: escribe aquí los escenarios que no se
     pudieron comprobar con código ("(visible)" o "no pudo comprobarse") y las restricciones
     marcadas "no testeable",
     uno por línea, dejando las marcas ✔ | ✘ | ? sin elegir.
     El agente NO elige marca, NO escribe la nota y NO rellena
     "Detectado solo por el humano": eso es solo de la persona. -->

- El residuo baja en el dispositivo y ningún segundo por debajo de 58 FPS (escenario visible): ✔ | ✘ | ?
- El caso límite del segundo con más tráfico mantiene `ejec` ≤ 14,6 ms: ✔ | ✘ | ?
- Los campos del `.txt` siguen significando lo mismo en una sesión de diagnóstico: ✔ | ✘ | ?
- El reparto medido en host se corresponde con el coste real en el dispositivo: ✔ | ✘ | ?
- Detectado solo por el humano: [lo completa la persona]
