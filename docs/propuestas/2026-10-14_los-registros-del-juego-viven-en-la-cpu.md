# Propuesta: Los registros del juego viven en la CPU, no en la memoria

Fecha: 2026-10-14
Nivel: 2
Estado: discrepada

<!-- Estados: pendiente | aprobada | modificada | descartada |
     verificada por agente | revisada por humano | discrepada | cerrada -->

## Propuesta (la escribe el agente antes de editar)

### 1. Qué cambia

El código del juego que corre en el teléfono deja de mover sus dieciséis registros por la
memoria en cada instrucción: el C que genera el recompilador los mantiene en variables locales,
que la CPU del dispositivo puede tener en registros de máquina, y solo los vuelca a la imagen del
procesador antes de las pocas llamadas que de verdad los leen. Donde hoy hay una carga y un
almacén de `c[i]` por cada operando, pasa a haber una operación sobre una variable.

### 2. Contexto

- Contexto consultado: `../gba-recomp/crates/recomp-core/src/emit.rs` (896 líneas, el generador),
  `../gba-recomp/crates/gba-core/src/capi.rs` (la tabla `RtApi` y sus puentes), el C generado real
  en [`src-c/`](../../src-c/) (12 unidades, ~206 MB), [`src/recomp.rs`](../../src/recomp.rs) (el
  despacho del port), [`docs/propuestas/2026-10-12_acceso-memoria-sin-llamada-indirecta.md`](2026-10-12_acceso-memoria-sin-llamada-indirecta.md)
  (la medición del salto indirecto y la lección de método), [`docs/explicacion/analisis_rendimiento.md`](../explicacion/analisis_rendimiento.md)
  (los números vigentes de `ejec`), [`tests/golden_frame_tests.rs`](../../tests/golden_frame_tests.rs)
  y [`scripts/diagnostico/fotogramas_dorados.py`](../../scripts/diagnostico/fotogramas_dorados.py)
  (el arnés instalado).
- Intención: bajar `ejec` —el tiempo de CPU de la emulación— para que desaparezcan las caídas que
  quedan tras la PPU. El éxito se ve como **ningún segundo por debajo de 58 FPS** en la telemetría
  del dispositivo, la misma deuda que dejó la propuesta de la PPU encima de esta.
- Restricciones y decisiones previas (máx. 3):
  - El renderizado no cambia (origen: [`2026-10-12_acceso-memoria-sin-llamada-indirecta.md`](2026-10-12_acceso-memoria-sin-llamada-indirecta.md))
    → implica: los tres hashes guardianes y `fallback_steps: 0` siguen idénticos → protegida por:
    *El renderizado no cambia (regresión)*.
  - Medir con pares intercalados (≥5) y **acotar el techo del premio antes de fijar el listón**
    (origen: «Para la próxima vez» de la propuesta anterior, tras fijar un 10 % que era inalcanzable
    por construcción) → implica: esta propuesta empieza midiendo, no editando → protegida por:
    *El prototipo acota el techo antes de tocar el generador*.
  - El motor se desarrolla en local y el port lo consume por `[patch]`, con el SHA aún sin publicar
    (origen: [`Cargo.toml`](../../Cargo.toml), `rev = "de4edf59…"`) → implica: el port no cambia
    hasta que el motor esté medido; publicar el motor y re-fijar el SHA es trabajo aparte →
    protegida por: no testeable (queda en «Revisión humana»).
- Alternativas descartadas:
  - Subir `-O2`/`-O3` en la compilación del C generado → descartada por medida propia: en código
    real del proyecto apenas mueve ~1 % del `.text`; el problema no es el compilador, es la *forma*
    del C.
  - El «acceso directo al bus» por sí solo (la palanca de la propuesta del 12) → descartada por
    medida de esa misma propuesta: 2,69 ns por la vtable contra 0,95 ns leyendo por puntero
    (2,8×), ≈0,45 ms por fotograma en el TV, ≈3 % del fotograma. Por eso aquí el bus directo no es
    el objetivo: es, como mucho, un acompañante del volcado.
  - Atacar el intérprete de respaldo, la PPU o el APU → descartados antes o ya medidos (la BIOS es
    el único punto vivo del intérprete y va por su propia propuesta).
- Ante imprevistos, prioriza: el búfer de fotograma idéntico por encima de la ganancia.

### 3. Ejemplos

Los fragmentos son **C generado real** de [`src-c/`](../../src-c/) (bloques de un mismo grupo de
cadena en `1575 - Fire Emblem (E)(Independent)-bios.0.c`), no inventados.

- **Caso normal** (aritmética pura sin llamadas). Bloque `L_08001000_t` de `g_08000feb`:

  antes:
  ```c
  L_08001000_t:;
      {
          uint32_t r = c[13] + 0x00000010u + 0u;
          c[13] = r;
      }
      a->tick(m, 2u);
  ```
  ahora (objetivo):
  ```c
  L_08001000_t:;
      r13 = r13 + 0x00000010u;
      a->tick(m, 2u);
  ```
  Las dos cargas/almacenes de `c[13]` se vuelven una operación sobre una variable local; el volcado
  a `c[13]` ocurre una sola vez, en la salida del bloque o antes de una llamada que lea registros.

- **Caso límite** (mezcla de `LDM` y aritmética con `tick` intercalado, el mismo grupo):

  antes:
  ```c
      uint32_t base = c[13];
      uint32_t addr = base;
      c[13] = base + 8u;
      c[4] = a->read32(m, addr & ~3u); addr += 4u;
      c[7] = a->read32(m, addr & ~3u); addr += 4u;
  ```
  ahora (objetivo): `base` sale de `r13`, los destinos van a `r4`/`r7`, y `a->read32(m, …)` se llama
  igual —porque el bus es la frontera que no se toca salvo que el prototipo diga lo contrario—,
  con el volcado de los locales que la llamada pueda leer.

- **Caso de frontera** (camino que *sí* necesita los registros en memoria): el retorno por `POP`
  con `pc`, la entrada de un grupo de cadena por `switch (entry)`, y el «shell» a `a->interp_one(m)`.
  Ahí los locales deben volcarse a `c[]` antes de salir, porque el despachador y el intérprete leen
  `regs[15]` y `cpsr` de la máquina.

### 4. Pasos

**Fase A — acotar el techo, sin tocar el generador.** Es la que puede matar la propuesta antes de
escribir una línea de `emit.rs`.

1. Extraer de [`src-c/`](../../src-c/) un puñado de grupos de cadena reales y calientes (los que
   contengan bucles con `read32` y aritmética) y compilarlos con el **clang del NDK** para
   `aarch64` a `-O2`, primero tal cual y después con una reescritura a mano de registros locales.
2. Comparar el ensamblado: contar cargas/almacenes sobre la imagen del procesador (`ldr`/`str`
   contra el puntero `m`) en las dos versiones. Si con la forma actual el compilador ya mantiene los
   registros en registros de máquina, la palanca no existe y se para aquí.
3. Micro-banco con la *forma* del código emitido (un núcleo con `c[i]` y llamadas indirectas contra
   el mismo núcleo con locales y volcado), compilado con el mismo clang del NDK, con ≥5 pares
   intercalados. Esto fija el techo en tanto por ciento.
4. Con el techo medido, **fijar el listón** del escenario 3 por debajo de él y anotarlo en esta
   propuesta antes de pasar a la fase B.

**Fase B — el cambio**, solo si la fase A confirma que hay techo.

5. En `../gba-recomp/crates/recomp-core/src/emit.rs`: declarar los dieciséis registros como
   variables locales al inicio de cada función de bloque y de cada grupo de cadena; llevar `cx.reg()`
   y toda escritura de registro a esas variables; cargar desde `c[]` en la entrada (bloque suelto y
   `switch (entry)` de grupo) y volcar antes de los caminos que leen la máquina
   (`a->interp_one`, `a->chain_gate`, y cada `return`).
6. Verificar los tres guardianes y el arnés de fotogramas dorados, regenerar
   [`src-c/`](../../src-c/) con [`compilar_recomp_aot.ps1`](../../scripts/diagnostico/compilar_recomp_aot.ps1)
   y medir en el arnés de host con pares intercalados.
7. Publicar el motor y re-fijar el SHA del port; empaquetar el APK con
   [`package_apk.ps1`](../../scripts/package_apk.ps1) para la sesión de telemetría.

### 5. Qué más podría verse afectado

Búsquedas hechas: `grep -rn "RtApi" src/ tests/` en el port y en el motor,
`grep -n "c\[" src-c/*.c`, `grep -rn "regs\[15\]|cpu.cpsr|cpu.thumb" ../gba-recomp/crates/gba-core/src/capi.rs`
y `grep -rn "recv|read8|read16|read32|tick" ../gba-recomp/crates/gba-core/src/capi.rs`.

- `../gba-recomp/crates/recomp-core/src/emit.rs` — **es el único archivo que cambia**. El preámbulo
  (`PREAMBLE`) ya define `f_nz`/`f_c`/`op_adds` tomando `uint32_t* c`; con locales pasan a tomar la
  dirección de una variable local o a usar la convención de bits que ya usan, según decida el
  prototipo.
- `../gba-recomp/crates/gba-core/src/capi.rs` — la tabla `RtApi` **no cambia de forma** en esta
  propuesta (a diferencia de la del 12). Los puentes `rt_read*`/`rt_write*`/`rt_tick` no leen
  `cpu.regs`; `rt_chain_gate` lee `cpu.cpsr` y `rt_interp_one` lee `regs[15]` y `cpsr`, y por eso
  esos dos sitios fuerzan volcado. Si el prototipo mostrara que algún puente sí observa los
  registros, la lista de puntos de volcado se amplía —decisión del prototipo, no de la intuición.
- [`src-c/`](../../src-c/) — se regenera entero (ignorado por `.gitignore`); sube o baja el tiempo de
  compilación y el tamaño del APK.
- [`src/recomp.rs`](../../src/recomp.rs) y [`src/recomp_tests.rs`](../../src/recomp_tests.rs) — usan
  `RtApi` como tipo pero no lo construyen; como la forma no cambia, no deberían tocarse. Se comprueba
  compilando.
- [`tests/packaging_tests.rs`](../../tests/packaging_tests.rs) y [`tests/golden_frame_tests.rs`](../../tests/golden_frame_tests.rs)
  — el APK cambia de tamaño y el C generado de contenido; hay pruebas de empaquetado que pueden asumir
  conteos.
- Sin más referencias encontradas.

### 6. Suposiciones

- **El compilador del dispositivo no mantiene hoy el registro del juego en registros de máquina**,
  porque cada operando es `c[i]` sobre un `void* m` opaco y hay una llamada indirecta entre casi
  cada par de instrucciones, que invalida el valor en caché. → prototipar: sí, con el paso 2
  (ensamblado de un grupo real con el clang del NDK) → si es falso: no se toca `emit.rs`; se cierra
  esta propuesta y el sospechoso que quede en pie pasa a una propuesta nueva
  → resultado del prototipo: **matizada y en parte falsa**. En la forma base, el 31,1 % de las
  instrucciones son carga/almacén sobre la imagen del procesador, así que el compilador *no*
  mantiene el registro en registros de máquina de forma global; pero sí lo mantiene dentro de cada
  tramo recto, y localizarlo no baja el total (empate) porque el volcado obligatorio de cada salida
  de bloque devuelve el ahorro. El «si es falso» se aplicó: no se tocó `emit.rs`. | no detecta:
  diferencias entre el clang del NDK y el compilador real que enlaza el APK en cada ABI.
- **Las llamadas del bus (`a->read*`/`a->write*`/`a->tick`) y `a->guard` no leen ni escriben
  `cpu.regs`**, de modo que no exigen volcar los locales antes de llamarlas. → prototipar: sí, con
  instrumentación temporal en `capi.rs` que afirme que `cpu.regs` no se observa durante esas
  llamadas, más los tres guardianes con el volcado restringido → si es falso: se vuelca antes de
  cada llamada de bus (más seguro, menos ganancia) y el techo baja; se mide igual
  → resultado del prototipo: **no comprobada**, porque la fase B no se ejecutó (la fase A cerró la
  palanca antes). Queda enunciada, sin instrumentar, para quien retome el trabajo. | no detecta:
  rutas de bus poco transitadas (guardado, RTC) que la ventana de 7200 fotogramas no toque.
- **El host es un proxy aceptable del dispositivo para decidir.** → prototipar: no (el host compila
  con TinyCC y el dispositivo con clang; ya se vio que el host dispersa ±13 % y puede subestimar)
  → si es falso: la decisión la toma la sesión de telemetría, no el arnés.
  → no detecta: la arquitectura de memoria del TV frente a la del equipo de desarrollo.

### 7. Alcance

- **Incluye:**
  - El generador `../gba-recomp/crates/recomp-core/src/emit.rs` (motor): preámbulo y sitios de
    emisión de registros.
  - La fase A de medición (extracción de grupos reales, compilación con el clang del NDK,
    micro-banco y conteo de ensamblado).
  - La regeneración de [`src-c/`](../../src-c/) y la verificación en el arnés de host.
- **No incluye:**
  - La tabla `RtApi` y los puentes de `capi.rs` (no cambian de forma, salvo instrumentación temporal).
  - El camino de acceso directo al bus (descartado como objetivo propio por la propuesta del 12).
  - Publicar el motor y re-fijar el SHA: trabajo propio, después de medir.
  - La PPU, el APU, el intérprete de BIOS y el camino de GPU.

### 8. Criterios de aceptación (no se modifican sin aprobación)

**Escenario: El renderizado no cambia (regresión)**
Caso de uso: cualquier cambio del generador | Tipo: regresión
  Dado el arnés del motor con la ROM y el BIOS del usuario
  Cuando se ejecutan 600, 2400 y 7200 fotogramas con el C regenerado
  Entonces los hashes son los tres guardianes, con `fallback_steps: 0` y `native_blocks` idénticos a
  los de antes del cambio
Test: `recomp runc --frames 600|2400|7200` (comando del apartado 9), más
`python scripts/diagnostico/fotogramas_dorados.py`
Estado previo: **pasa** (es el guardián, debe seguir pasando)

**Escenario: El prototipo acota el techo antes de tocar el generador**
Caso de uso: diagnóstico en host | Tipo: normal
  Dado un grupo de cadena real de [`src-c/`](../../src-c/) compilado con el clang del NDK para
  `aarch64 -O2`, en su forma actual y reescrito a mano con registros locales
  Cuando se comparan, con ≥5 pares intercalados, el micro-banco de las dos formas y se cuentan en
  el ensamblado las cargas/almacenes sobre la imagen del procesador
  Entonces la forma con locales queda **dentro del techo** cuantificado (mejor fuera del ruido del
  banco) y el ensamblado actual muestra más accesos a la imagen del procesador que la forma con
  locales; el techo en porcentaje queda escrito en esta propuesta
Test: banco y volcado de ensamblado nuevos (motor u host), escritos tras la aprobación
Estado previo: **no existen** (se escriben tras la aprobación)

**Escenario: La ganancia se realiza en el arnés**
Caso de uso: corrida de 7200 fotogramas con la entrada del banco | Tipo: normal
  Dado el C regenerado con el volcado y el mismo `.dll` de host
  Cuando se compara con la corrida anterior con ≥5 pares intercalados, y `ppu`/`apu` como control
  Entonces el tiempo de ejecución baja al menos **la mitad del techo del escenario anterior**, con
  `ppu` y `apu` dentro del ±5 % y los hashes idénticos
Test: `RECOMP_PPU_STATS=1 … runc --frames 7200`, pares intercalados
Estado previo: **falla** (la forma actual es la línea base; el listón se fija en el paso 4)

**Escenario (límite): el fotograma del dispositivo**
Caso de uso: mapa táctico en el Android TV, misma escena que la sesión 17 | Tipo: límite
  Dado el APK con el C regenerado y una sesión de al menos 120 s
  Cuando se lee `blazing-perf.txt` con [`analizar_telemetria.py`](../../scripts/diagnostico/analizar_telemetria.py)
  y se compara con la sesión 17
  Entonces `ejec` baja respecto de los 13,53 ms de la sesión 17 y ninguno de los 166 segundos queda
  por debajo de 58 FPS
Test: revisión humana + `analizar_telemetria.py`
Estado previo: **falla** (13,53 ms de `ejec` y 10 segundos de 166 por debajo de 58 FPS)

### 9. Cómo lo comprobaré

- **El renderizado no cambia** → comando:
  `../gba-recomp/gba-recomp-launcher-0.4-x86_64-pc-windows-msvc/recomp.exe runc "roms/1575 - Fire Emblem (E)(Independent).gba" --bios roms/gba_bios.bin --frames 7200`
  → pasa si: imprime el hash guardián de 7200 y `fallback_steps: 0`, código de salida 0
  → no detecta: que el `.so` del APK lleve el cambio; eso lo cubre el empaquetado y la telemetría.
- **El prototipo acota el techo** → herramienta: el clang del NDK (`aarch64-linux-android*-clang`,
  con `-S -O2`) y el micro-banco, invocados desde la raíz del repositorio
  → pasa si: el ensamblado actual muestra más accesos a la imagen del procesador que la forma con
  locales y el micro-banco separa las dos formas con ≥5 pares intercalados
  → no detecta: el compilador real que enlaza cada ABI del APK.
- **La ganancia se realiza en el arnés** → comando: `RECOMP_PPU_STATS=1 … runc --frames 7200` con el
  C anterior y el regenerado, ≥5 pares intercalados, máquina ociosa
  → pasa si: la mediana de la diferencia supera la mitad del techo y `ppu`/`apu` quedan en ±5 %
  → no detecta: diferencias de arquitectura de memoria entre el host y el TV.
- **Nada más se rompe** → comando: `cargo test` en el port y `cargo test -p gba-core` en el motor
  → pasa si: toda la suite en verde, incluidos `tests/golden_frame_tests.rs` y los tests de la PPU
  → no detecta: regresiones visuales que el hash no vea por casualidad en 7200 fotogramas.
- **Escenario «(visible)»** → revisión humana: jugar de 60 a 120 s en una zona que hoy cae (combate
  con animación o transición de mapa) y leer en `blazing-perf.txt` los segundos por debajo de 58 FPS
  y `ejec`
  → no detecta: nada más; es la medida que decide.

**Blindaje por mutación (fase 4 del pipeline, previsto):**
M1 — no volcar `c[15]` antes de `a->interp_one` → debe morir el escenario de regresión.
M2 — mantener el valor viejo de un local cuando el bloque lo modifica (saltarse una escritura) →
debe morir el escenario de regresión.
M3 — no cargar los locales desde `c[]` en la entrada del grupo de cadena → debe morir el escenario
de regresión.
Los archivos se respaldan en memoria RAM y se restauran en un bloque `finally`, sin comandos de Git.

### 10. Terminado cuando

Los cuatro escenarios pasan: los tres hashes guardianes siguen intactos, el techo está acotado y
escrito, la ganancia en el arnés supera la mitad de ese techo con los controles planos, la suite del
port y la del motor están en verde, y la sesión en el TV mejora `ejec` sin ningún segundo por debajo
de 58 FPS.

### 11. Parar si

- 2 intentos fallidos seguidos con el mismo criterio: detenerse, explicar qué se probó y marcar la
  propuesta como `discrepada`.
- **La fase A desmiente la premisa** (el ensamblado del clang del NDK ya mantiene el registro del
  juego en registros de máquina, o el micro-banco no separa las dos formas fuera de su ruido): no se
  toca `emit.rs`; se documenta el reparto medido y se cierra la propuesta.
- La ganancia del arnés queda por debajo de la mitad del techo tras dos corridas con pares
  intercalados: se trata como fase A fallida y no se publica nada.

### Correcciones previas a presentar

- **Corregido antes de presentar:** mi primer borrador fundaba la palanca en «los 300 ciclos por
  instrucción». Ese número no aparece en ninguna medición versionada de este repositorio (`ejec` es
  13,53 ms en el TV; el host mide 0,536 ms de `ppu` y 0,225 de `apu` por fotograma), así que lo he
  retirado y la propuesta habla solo en unidades medidas.
- **Corregido también:** el primer borrador incluía el «bus directo» como parte del cambio. La
  propuesta del 12 ya lo midió en 0,45 ms por fotograma en el TV (≈3 %), así que aquí no es un
  objetivo: es, como mucho, un acompañante, y el apartado 2 lo dice.
- **Corregido además:** el borrador pedía un umbral fijo de mejora en host. La propuesta del 12
  enseñó que un umbral fijo puede ser inalcanzable por construcción; aquí el listón se fija **después**
  de acotar el techo (paso 4), antes de la fase B.

### Cambios tras prototipos (solo si el plan cambió)

- Antes: la fase B iba a reescribir `emit.rs` con registros locales y volcado estrecho → Ahora: la
  fase B no se ejecuta y no se toca el generador → Por qué: la fase A midió el agregado en **−0,9 %
  de instrucciones y −0,7 % de accesos a memoria** sobre 8 grupos reales (un empate), con dos grupos
  peores y uno de ellos un **+28,8 %**; el techo no apareció.

---

## Resultado (lo completa el agente después de editar)

**Discrepada en la fase A.** La fase A existía para poder matar la premisa antes de tocar el
recurso, y eso es lo que pasó: **la mejora no está en localizar los registros** y **no se tocó
`emit.rs`**. No queda ningún cambio del motor ni del port; la única escritura es esta propuesta.

### Qué se midió

Ocho grupos de cadena reales de [`src-c/`](../../src-c/) (los de más tráfico de memoria y de ALU de
la imagen), extraídos literalmente, transformados por un script a la forma «locales» —que es lo que
haría el generador— y compilados con el **clang del NDK 30** para `aarch64`, al **nivel real del APK
(`-O1`)**, contando en el ensamblado las instrucciones totales y las de carga/almacén.

- **Forma «full»** (volcado completo antes de cada llamada `a->` y cada `return`):
  **+73,2 % de instrucciones** (412.555 → 714.583) y la fracción de memoria sube del 31,1 % al 51,0 %.
  Es un desastre: el volcado a cada llamada de bus cuesta más que el ahorro.
- **Forma «narrow»** (volcado completo solo antes de `interp_one` y de cada `return`):
  **−0,9 % de instrucciones y −0,7 % de accesos a memoria** (412.555 → 408.958 y 128.485 → 127.555).
  Un empate.

Distribución por grupo (narrow contra base, `-O1`): 4 grupos mejoran (−7 % a −17 %), 2 empatan (±2 %)
y 2 empeoran (uno de ellos **+28,8 %**).

| grupo | base ins/mem | full ins/mem | narrow ins/mem | narrow vs base |
|---|---|---|---|---|
| 084093eb | 42.928 / 14.714 | 107.937 / 57.603 | 42.694 / 14.431 | −0,5 % / −1,9 % |
| 08035ddd | 64.396 / 19.581 | 96.384 / 49.128 | 59.752 / 19.091 | −7,2 % / −2,5 % |
| 089f2d69 | 81.424 / 23.799 | 204.884 / 127.276 | 104.867 / 43.350 | **+28,8 % / +82,2 %** |
| 08001e33 | 35.798 / 11.067 | 54.668 / 22.591 | 35.907 / 8.927 | +0,3 % / −19,3 % |
| 0804da3b | 57.794 / 18.033 | 72.949 / 34.511 | 48.242 / 12.325 | −16,5 % / −31,7 % |
| 080be34d | 27.442 / 8.540 | 53.377 / 23.844 | 30.898 / 8.360 | +12,6 % / −2,1 % |
| 08060c8d | 54.605 / 17.590 | 63.236 / 25.395 | 45.455 / 11.771 | −16,8 % / −33,1 % |
| 0806fe91 | 48.168 / 15.161 | 61.148 / 24.132 | 41.143 / 9.300 | −14,6 % / −38,7 % |
| **total** | **412.555 / 128.485** | **714.583 / 364.480** | **408.958 / 127.555** | **−0,9 % / −0,7 %** |

### Por qué (el mecanismo)

El registro del juego en memoria *sí* cuesta: en la forma base, **3 de cada 10 instrucciones son
carga o almacén** sobre la imagen del procesador (31,1 %). Pero localizarlos no baja el total,
porque el modelo de bloques del emisor obliga a volver a materializar la imagen completa en memoria
en **cada salida de bloque**: el despachador y el bloque siguiente leen `c[]`. El ahorro dentro de un
tramo recto lo devuelve el volcado de frontera (16 almacenes por `return`) más las cargas de entrada,
y en los grupos con muchas salidas (089f2d69: 5.256 `goto` y 1.518 puertas de cadena) el volcado
domina y pierde por goleada.

### Predicho vs. real

- **El renderizado no cambia (regresión)**: **no se intentó**, y con razón: el cambio no llegó a
  `emit.rs`, así que no había C regenerado que guardar. El escenario queda intacto para el día que
  alguien retome la palanca por otra vía.
- **El prototipo acota el techo antes de tocar el generador**: **difiere a la baja**. La primera
  mitad (el ensamblado actual muestra más accesos a la imagen del procesador que la forma con
  locales) se cumple en la mayoría de los grupos, pero la premisa de la propuesta —que localizar
  rinde— **no se cumple**: el agregado es un empate (−0,9 % / −0,7 %) y hay dos grupos peores.
  Resultado real: el de la tabla. Método: el anunciado (extracción de grupos reales + clang del NDK
  del tipo que enlaza el APK), con una corrección: el nivel real es `-O1`, no `-O2`, y a los tres
  niveles (`-O0`, `-O1`, `-O2`) el agregado tampoco mejora.
- **La ganancia se realiza en el arnés** y **el fotograma del dispositivo**: **no se intentaron**;
  no había nada que medir.
- **Nada más se rompe**: no aplica (no hubo cambio).

### Contexto: ¿se respetó?

- El renderizado no cambia → respetada: no se tocó el generador, así que no hay nada que romper.
- Medir con pares intercalados y acotar el techo antes del listón → respetada: se acotó el techo
  antes de fijar ningún listón; el listón nunca se fijó porque el techo no apareció.
- El motor se desarrolla en local y el port lo consume por `[patch]` → respetada: el motor se leyó,
  no se editó.
- ¿Se tocó algo que alguna restricción prohibía? no.
- Imprevistos que surgieron: el optimizador de clang elimina la función del grupo si es `static` y
  nadie la referencia (el primer `.s` salía vacío, 237 bytes) → criterio de prioridad aplicado: sí
  (primero una medición válida).
- ¿Reapareció alguna alternativa descartada? no.

### Archivos modificados

Solo esta propuesta. El motor (`../gba-recomp/`) no se tocó: su `git status` sigue mostrando
únicamente los cambios preexistentes de la sesión de la PPU. Los artefactos del prototipo viven
fuera del repositorio (en el directorio temporal del sistema), así que no ensucian el árbol.

### Discrepancias

- La premisa «localizar los registros baja el coste» es **falsa en el modelo actual del emisor**
  → **suposición falsa**: el coste real está en la materialización de frontera (el volcado por salida
  de bloque), no en el acceso a los registros dentro del bloque.
- El nivel de optimización del APK es `-O1`, no `-O2` → **información faltante**, descubierta al
  construir el prototipo (estaba en [`package_apk.ps1`](../../scripts/package_apk.ps1)); se midió a
  los tres niveles para no depender de un solo ajuste.

### Para la próxima vez

Antes de prometer que localizar un valor ahorra memoria, medir el coste del **volcado de frontera**
(cuántas salidas tiene cada bloque), no solo el del acceso dentro del bloque: en este emisor el
registro se vuelve a escribir en memoria en cada `return`, y eso anula el ahorro.

### Fuera de alcance (hallazgos que no se arreglaron)

- **El registro del juego en memoria cuesta el 31,1 % de las instrucciones generadas** (media de 8
  grupos reales a `-O1`). Es un hecho nuevo y sólido del perfil del juego, aunque localizar no lo
  aproveche. | impacto: bajo
- Los grupos de cadena con **muchas salidas** (1.518 puertas y 5.256 `goto` en 089f2d69) hacen que
  cualquier esquema de volcado por `return` explote: si algún día se retoma, el volcado debe ser
  **parcial** (solo los registros vivos) y decidido por el analizador de flujo. | impacto: medio
- La forma «full» (volcado antes de cada llamada de bus) queda medida y descartada: +73,2 % de
  instrucciones. | impacto: bajo
- El host no tiene compilador nativo (`cc`/`gcc`/`clang`), así que el micro-banco *en ejecución* del
  escenario 2 no se pudo correr: la fase A se resolvió con censo estático de ensamblado, más barato y
  ya decisivo. | impacto: bajo

---

## Discrepancia

Abierta por: agente
Punto afectado: *El prototipo acota el techo antes de tocar el generador* (la premisa «localizar los
registros rinde»)
Motivo: el agregado de 8 grupos reales a `-O1` es un empate (−0,9 % de instrucciones, −0,7 % de
accesos), con 2 grupos peores y uno de ellos +28,8 %. El ahorro por localizar se devuelve en el
volcado obligatorio de cada salida de bloque.
Clasificación: suposición falsa
Propuesta de corrección: ninguna por ahora. Si se retoma, la vía es un volcado **parcial** (solo los
registros vivos, decidido por el analizador de flujo) y una propuesta nueva con el techo calculado
sobre el número de salidas por bloque.

---

## Revisión humana
<!-- El agente SÍ edita esta sección: escribe aquí los escenarios que no se
     pudieron comprobar con código ("(visible)" o "no pudo comprobarse") y las restricciones
     marcadas "no testeable",
     uno por línea, dejando las marcas ✔ | ✘ | ? sin elegir.
     El agente NO elige marca, NO escribe la nota y NO rellena
     "Detectado solo por el humano": eso es solo de la persona. -->

- Escenario «(visible)»: el fotograma del dispositivo (mapa táctico en el TV) — no se intentó,
  porque la fase A cerró la palanca: ✔ | ✘ | ?
- Restricción no testeable: el motor se desarrolla en local y el port lo consume por `[patch]`, con el
  SHA sin publicar: ✔ | ✘ | ?
- Detectado solo por el humano: [lo completa la persona]
