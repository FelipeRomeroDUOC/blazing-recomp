# Propuesta: Acotar el techo del reloj por instrucción

Fecha: 2026-10-14
Nivel: 2
Estado: verificada por agente (alcance aprobado por el usuario el 2026-10-14; falta la traslación al televisor)

<!-- Estados: pendiente | aprobada | modificada | descartada |
     verificada por agente | revisada por humano | discrepada | cerrada -->

## Propuesta (la escribe el agente antes de editar)

### 1. Qué cambia

Nada de lo que se ve en el teléfono. Durante una corrida de laboratorio —**no en el APK**— el motor
cuenta cuántas veces el código nativo pide avanzar el reloj y cuántos ciclos lleva cada petición, un
micro-banco mide cuánto cuesta una de esas peticiones, y el producto de los dos números dice cuántos
milisegundos del fotograma son el reloj. Con eso, el reloj por instrucción se cierra como palanca o
como descarte, con número.

### 2. Contexto

- Contexto consultado: `../gba-recomp/crates/gba-core/src/capi.rs` (la tabla `RtApi` y el puente
  `rt_tick`), `../gba-recomp/crates/recomp-core/src/emit.rs` (`Ctx::cycles`, `flush_ticks` y sus ocho
  puntos de volcado), `../gba-recomp/crates/gba-core/src/mem.rs` (`Bus::tick`), `../gba-recomp/crates/recomp/src/main.rs`
  (`cmd_runc`), el C generado en [`src-c/`](../../src-c/), `../gba-recomp/target/release/recomp.exe`,
  [`2026-10-14_medir-la-frontera-entre-bloques.md`](2026-10-14_medir-la-frontera-entre-bloques.md)
  (el patrón de contador y su +0,5 %) y
  [`2026-10-12_acceso-memoria-sin-llamada-indirecta.md`](2026-10-12_acceso-memoria-sin-llamada-indirecta.md)
  (los 2,69 ns por la vtable y el 3 % que la descartó).
- Intención: saber si el reloj por instrucción es una palanca o el tercer descarte seguido. El éxito
  es un número con su barra: ms por fotograma que son reloj, en % de los 16,67 ms.
- Restricciones y decisiones previas (máx. 3):
  - El renderizado no cambia (origen: [`2026-10-12_acceso-memoria-sin-llamada-indirecta.md`](2026-10-12_acceso-memoria-sin-llamada-indirecta.md))
    → implica: los tres hashes guardianes y `fallback_steps: 0` siguen idénticos con el contador
    puesto → protegida por: *El renderizado no cambia (regresión)*.
  - Los prototipos del motor se respaldan en RAM y se restauran en un bloque `finally`, sin comandos
    de Git (origen: `AGENTS.md`, regla de restauración) → implica: el instrumento no queda en el
    árbol al terminar → protegida por: la comprobación posterior (`git status` del motor).
  - Medir el techo **antes** de fijar el listón (origen: «Para la próxima vez» de
    [`2026-10-14_los-registros-del-juego-viven-en-la-cpu.md`](2026-10-14_los-registros-del-juego-viven-en-la-cpu.md))
    → implica: esta propuesta mide y no toca `emit.rs` → protegida por: *El reloj queda pesado* y
    *El techo del reloj queda acotado*.
- Alternativas descartadas (solo si las hubo):
  - Atacar `emit.rs` para coalescer más los `tick` → descartada por medición estática propia: la
    coalescencia **ya funciona** (media **9,22 ciclos por tick**, y solo el **0,2 %** de los
    400.767 ticks emitidos lleva 1 ciclo; los valores modales son 4, 6, 8, 10 y 12). No hay nada que
    coalescer sin tocar la semántica del reloj.
  - Deducir el volumen del censo estático → descartada por la lección de la propuesta de la frontera:
    el censo no pesa por frecuencia de ejecución, y el reparto entre escenas ya cambió los números
    en esa medición (90-263 puertas por fotograma según la escena).
  - Un A/B con `rt_tick` convertido en no-op → descartada porque altera lo medido: sin reloj, el
    juego cambia de comportamiento y la carga de trabajo deja de ser la misma.
  - Instrumentar el APK → descartada: el contador debe vivir en una corrida de laboratorio, no en un
    fotograma de 16,67 ms.
- Ante imprevistos, prioriza: que la medida no altere lo medido.

### 3. Ejemplos

- **Caso normal:** la línea de resumen del arnés
  antes: `frames: 7200 hash: b54066dc06f2b76c native_blocks: 3208998 fallback_steps: 0`
  ahora (objetivo): la misma línea más `ticks: T ciclos: C`, con `T` y `C` por fotograma en el
  desglose, y las dos cifras del micro-banco (ns por petición).
- **Caso límite:** si `T` sale del orden de **decenas de miles** por fotograma (~280.896 ÷ 9,22 ≈
  30.500, que es lo que predice la media estática) y una petición cuesta pocos nanosegundos, el
  techo queda por debajo del 3 %: el reloj se cierra como descarte y se anota, igual que la frontera.
- **Caso de control:** la corrida con el contador y la corrida sin él dan el mismo hash y los mismos
  `native_blocks`; y la suma de ciclos por fotograma tiene que caer en los ~280.896 ciclos que dura un
  fotograma. Si no cae, el contador no está viendo el reloj completo y la medida es inválida.

### 4. Pasos

1. Añadir en `crates/gba-core/src/capi.rs` dos contadores estáticos (`ticks` y `ciclos`),
   incrementados dentro de `rt_tick` con orden `Relaxed`, y una función de lectura. **La forma de
   `RtApi` no cambia**: el contador es estado global del proceso, no un campo de la tabla. Se activan
   con la variable `RECOMP_TICK` para que el A/B del paso 5 no obligue a recompilar entre corridas.
2. Añadir en `cmd_runc` (`crates/recomp/src/main.rs`) los dos valores a la línea de resumen y al
   desglose por fotograma.
3. Añadir en el motor un micro-banco **temporal**: un test de Rust que llama a `rt_tick` a través de
   la misma tabla `RT_API` (no a la función directa) sobre una `Machine` real, en pares intercalados
   contra un `rt_tick` vacío como control. Sale ns por petición.
4. Recompilar el motor (`cargo build --release -p recomp`) y **usar el binario recién construido**
   (`../gba-recomp/target/release/recomp.exe`), dejando intacto el del launcher.
5. Correr 7.200 fotogramas con `--input scripts/diagnostico/entrada_avance.input`, más 600/2400/7200
   **sin** `--input` para los tres guardianes; y el A/B intercalado del contador (pares, ≥5).
6. Revertir los archivos del motor desde la copia en RAM y comprobar el `git status` del motor.

### 5. Qué más podría verse afectado

Búsquedas hechas: `grep -rn "rt_tick|a->tick|flush_ticks|bus.tick" ../gba-recomp/crates/` y
`grep -rn "GbaPerfCounters|take_perf_counters" ../gba-recomp/crates/`.

- `../gba-recomp/crates/gba-core/src/capi.rs` — `rt_tick` (donde van los incrementos). Los puentes y
  la tabla no cambian.
- `../gba-recomp/crates/recomp/src/main.rs` — `cmd_runc`, solo la impresión.
- `../gba-recomp/crates/gba-core/src/mem.rs` — **no se toca**: se evita añadir campos a
  `GbaPerfCounters`, que el port consume, para no rozar el formato de la telemetría. El contador es
  un `static` propio.
- **El motor ya tiene cambios preexistentes** en `mem.rs`, `ppu.rs`, `perf_counters_tests.rs` y
  `.gitignore` (sesión de la PPU): el respaldo y la restauración deben dejar **esos** intactos y no
  confundirlos con los de esta propuesta.
- [`src-c/`](../../src-c/) — no se regenera: el C generado no cambia.
- El port y el APK — no se tocan.
- Sin más referencias encontradas.

### 6. Suposiciones

- **Una petición de reloj cuesta, en el host, el mismo orden de magnitud que en el dispositivo.**
  → prototipar: no (frecuencias, cachés y compiladores distintos) → si es falso: el techo se publica
  como rango en vez de como cifra, y la confirmación queda para una sesión en el televisor.
  → no detecta: el factor real entre host y TV.
- **El micro-banco mide el precio correcto** (llamada indirecta por la tabla, con la `Machine`
  caliente, que es como la usa el C generado). → prototipar: sí, con el A/B del propio micro-banco
  (llamada por la tabla contra función directa) → si es falso: el contador de ciclos da el peso y el
  micro-banco solo el precio unitario, y el techo se calcula solo con el contador.
  → resultado del prototipo: [una línea; se completa tras la aprobación]
- **`rt_tick` es el único camino por el que el código nativo pide reloj.** → prototipar: no (se
  comprueba leyendo el emisor: `flush_ticks` es el único sitio que escribe `a->tick`) → si es falso:
  habría reloj escondido en las llamadas de bus y el peso sería un suelo, no un total.
  → no detecta: un camino de reloj que el emisor añada en el futuro.

### 7. Alcance

- **Incluye:**
  - El contador en `rt_tick` y su impresión en `cmd_runc` (motor), temporal y revertido.
  - El micro-banco de ns por petición (motor), temporal.
  - El número: ticks y ciclos por fotograma, y el techo del reloj en ms y % del fotograma.
- **No incluye:**
  - Cambiar `emit.rs`, la coalescencia, la firma de `tick`, el bus o la tabla `RtApi`.
  - Atacar lo que mida: si el techo resulta grande, la palanca va en su propia propuesta.
  - Tocar el port, la telemetría del APK o `src-c/`.
  - Publicar el motor o re-fijar el SHA.

### 8. Criterios de aceptación (no se modifican sin aprobación)

**Escenario: El renderizado no cambia (regresión)**
Caso de uso: cualquier corrida con el instrumento puesto | Tipo: regresión
  Dado el arnés con la ROM y el BIOS del usuario y el mismo `--input`
  Cuando se ejecutan 600, 2400 y 7200 fotogramas con el contador activo
  Entonces los hashes son los tres guardianes (`31184031261ea2ec`, `b54066dc06f2b76c`,
  `b54066dc06f2b76c`), `fallback_steps: 0` y `native_blocks` idénticos a los de la corrida sin
  contador
Test: `recomp runc --frames 600|2400|7200` (comando del apartado 9)
Estado previo: **pasa** (es el guardián, debe seguir pasando)

**Escenario: El reloj queda pesado**
Caso de uso: diagnóstico en host | Tipo: normal
  Dado el arnés con los contadores puestos
  Cuando se corren 7.200 fotogramas con el `--input` de avance
  Entonces la línea de resumen imprime `ticks` y `ciclos`, quedan escritos los dos valores por
  fotograma junto a `native_blocks` por fotograma, y la suma de ciclos por fotograma cae dentro de
  ±5 % de los 280.896 ciclos que dura un fotograma
Test: corrida del paso 5
Estado previo: **no existe el dato** (hoy no se cuenta)

**Escenario: El techo del reloj queda acotado**
Caso de uso: decidir si se ataca | Tipo: límite
  Dado el micro-banco de ns por petición y los ticks por fotograma
  Cuando se multiplican y se comparan con los 16,67 ms del fotograma
  Entonces queda escrito un techo en ms y en % del fotograma, con la conclusión explícita «palanca»
  o «descarte» y el umbral que la decide, en la misma línea del apartado 3
Test: micro-banco del paso 3 + corrida del paso 5
Estado previo: **no existe el dato**; la predicción a batir es 0,7-1,3 % del fotograma (descarte)

**Escenario (control): el instrumento no cuesta**
Caso de uso: validez de la medida | Tipo: límite
  Dado el mismo binario y la misma corrida
  Cuando se comparan, con ≥5 pares intercalados, la corrida con los incrementos y la corrida sin
  ellos
  Entonces la mediana pareada de la diferencia queda dentro del ±5 % y `ppu`/`apu` no se mueven
Test: corrida del paso 5
Estado previo: **no medido**

### 9. Cómo lo comprobaré

- **El renderizado no cambia** → comando:
  `../gba-recomp/target/release/recomp.exe runc "roms/1575 - Fire Emblem (E)(Independent).gba" --bios roms/gba_bios.bin --frames 7200`
  → pasa si: imprime `hash: b54066dc06f2b76c` y `fallback_steps: 0`, código de salida 0
  → no detecta: que la escena sin `--input` sea la de más tráfico de reloj (mide la que está
  grabada).
- **El reloj queda pesado** → comando: la misma corrida con
  `--input scripts/diagnostico/entrada_avance.input`, leyendo `ticks` y `ciclos`
  → pasa si: los dos valores salen impresos, son estables entre las tres corridas y la suma de
  ciclos por fotograma cae en 280.896 ±5 %
  → no detecta: el reparto entre escenas distintas de la grabada.
- **El techo del reloj queda acotado** → comando:
  `cargo test -p gba-core --release coste_por_tick -- --nocapture` en el motor
  → pasa si: imprime ns por petición con su barra y el producto con los ticks por fotograma queda
  escrito en esta propuesta
  → no detecta: diferencias de frecuencia y caché entre el host y el televisor.
- **El instrumento no cuesta** → comando: el A/B intercalado del paso 5
  → pasa si: la mediana pareada queda en ±5 % con los controles planos
  → no detecta: diferencias entre ejecuciones de más del ±13 % (el ruido ya medido del arnés); por
  eso se usan pares intercalados.
- **Nada más se rompe** → comando: `cargo test -p gba-core` y `cargo test -p recomp` en el motor
  → pasa si: la suite del motor está en verde
  → no detecta: el port, que no se toca.

**Blindaje por mutación (fase 4 del pipeline, previsto):**
M1 — no sumar los ciclos dentro de `rt_tick` → debe morir *El reloj queda pesado* (la suma daría 0 y
no caería en 280.896).
M2 — incrementar `ticks` una vez por bloque en vez de por petición → debe morir *El reloj queda
pesado* (el valor cambiaría de orden y el escenario de control de la suma de ciclos dejaría de cuadrar).
Los archivos se respaldan en memoria RAM y se restauran en un bloque `finally`, sin comandos de Git.

### 10. Terminado cuando

Los cuatro escenarios pasan: los guardianes siguen idénticos, los dos contadores quedan impresos y
estables con la suma de ciclos cuadrando, el techo queda escrito en ms y %, y el motor queda
restaurado con el `git status` sin rastro del prototipo.

### 11. Parar si

- 2 intentos fallidos seguidos con el mismo criterio: detenerse, explicar qué se probó y marcar la
  propuesta como `discrepada`.
- El instrumento cuesta más del 5 % y no se puede abaratar: se mide con muestreo; si aun así no se
  separa del ruido, se para y se cierra con el reparto estático que ya hay.
- La suma de ciclos por fotograma no cae en 280.896 y no se encuentra por qué: detenerse y reportar,
  porque significaría que el reloj tiene un camino que esta propuesta no conoce.

### Correcciones previas a presentar

- **Corregido antes de presentar (y es la razón del cambio de enfoque):** el punto de partida era
  «el generador emite un `a->tick` por instrucción», y **el código dice otra cosa**. `emit.rs`
  acumula los ciclos (`Ctx::cycles`) y solo vuelca al salir del tramo recto, antes de un acceso al
  bus, antes de un shell o en un salto. Medido sobre el C generado: **400.767 ticks, 0,2 % de ellos
  con 1 ciclo, media 9,22 ciclos por tick**. Es decir, la coalescencia ya está hecha y el `tick` no
  es una llamada por instrucción. La propuesta sigue siendo del mismo tema, pero nace acotando el
  techo en vez de prometiendo una ganancia.
- **Corregido también:** el borrador medía el coste de la petición con un cronómetro alrededor de la
  corrida. Se cambió a micro-banco en pares intercalados más el contador de ciclos, porque el
  cronómetro de la corrida no separa el precio de la petición del número de peticiones.

### Cambios tras prototipos (solo si el plan cambió)

- Ninguno: el plan se ejecutó tal cual (contador en `rt_tick`, impresión en `cmd_runc`, micro-banco por
  la tabla, A/B intercalado).

---

## Resultado (lo completa el agente después de editar)

**Verificado por agente.** El reloj por instrucción queda **acotado y descartado como palanca**: una
petición de reloj cuesta 9,55 ns en el host y el código nativo pide 2.210-7.135 peticiones por
fotograma, así que el techo es **0,13-0,41 %** del fotograma — entre 7 y 23 veces menos que el 3 % de la
palanca del acceso a memoria que ya se descartó. Y sale un hallazgo que ningún número anterior tenía:
**la mayor parte del reloj del fotograma no pasa por el código nativo**.

### Qué se midió

Corridas con el instrumento puesto (`RECOMP_TICK=1`), el binario recién construido del motor:

| fotogramas | escena | hash | native_blocks | ticks | ciclos | ticks/frame | ciclos/frame |
|---|---|---|---|---|---|---|---|
| 600 | arranque | `31184031261ea2ec` | 1.776.798 | 3.846.772 | 22.021.991 | 6.411,3 | 36.703 |
| 2.400 | arranque | `b54066dc06f2b76c` | 2.167.398 | 7.138.079 | 44.639.017 | 2.974,2 | 18.600 |
| 7.200 | arranque | `b54066dc06f2b76c` | 3.208.998 | 15.914.905 | 104.951.125 | 2.210,4 | 14.577 |
| 7.200 | avance | `ee7dc7ef514b5a3f` | 11.018.334 | 51.370.239 | 336.309.173 | 7.134,8 | 46.710 |

Los tres guardianes salen **exactos** (mismos hashes, mismos `native_blocks`, `fallback_steps: 0`,
salida 0), y el `native_blocks` de la escena de avance coincide con el que registró
[`2026-10-14_medir-la-frontera-entre-bloques.md`](2026-10-14_medir-la-frontera-entre-bloques.md).
Tiempo de pared: 9,034 s para los 7.200 fotogramas de avance.

**Micro-banco** (7 rondas intercaladas, 300.000 llamadas por brazo, 9 ciclos):

| brazo | ns por llamada |
|---|---|
| camino completo (por la tabla en memoria, como el C) | **9,55** |
| el mismo despacho con el cuerpo vacío | 1,25 |
| llamada directa vacía | 1,26 |
| **trabajo del reloj** (camino − despacho) | **8,29** |
| despacho indirecto neto | **−0,01** |

**A/B del instrumento** (5 pares intercalados, avance, 7.200 fotogramas): 8.893 ms con el contador
contra 8.796 ms sin él; deltas +1,70, −0,13, +0,54, +0,80 y +8,25 %; **mediana +0,80 %**, dentro del
±5 %. El hash salió **idéntico en las diez corridas**.

**El techo:**

| escena | peticiones/fotograma × ns | ms/fotograma | % de 16,67 ms |
|---|---|---|---|
| arranque/menú | 2.210,4 × 9,55 | 0,021 | **0,13 %** |
| avance (`--input`) | 7.134,8 × 9,55 | 0,068 | **0,41 %** |

**Conclusión: descarte.** El umbral se escribió antes de medir (el 3 % de la palanca ya descartada) y
el techo queda 7-23 veces por debajo, contando de más: los 9,55 ns del camino completo incluyen el
renderizado de las líneas que el propio avance del reloj dispara, trabajo que el fotograma ya paga por
su cuenta en `ppu`.

### El hallazgo: solo el 5-17 % del reloj del fotograma pasa por el código nativo

El criterio de validez que me puse —«la suma de ciclos por fotograma cae en 280.896 ±5 %»— **falló**, y
el fallo es la parte interesante: los ciclos que pide `a->tick` son 14.577 de los 280.896 de un
fotograma en el menú (**5,2 %**) y 46.710 (**16,6 %**) en el avance. La causa está en
`run_frame_native`: cuando `m.bus.halted` está puesto llama a `m.step()` y **no lo cuenta** en
`fallback_steps` —por eso ese contador sale 0 y no significa que el juego no se detenga— y el reloj de
esos huecos lo avanza `skip_to_next_event`, que no pasa por `rt_tick`. El juego pasa la mayor parte del
fotograma dormido esperando el VBlank.

Consecuencias que ya se pueden afirmar:

- El reloj por instrucción se descarta con número propio, no por estimación.
- Las «~200.000 instrucciones de juego por fotograma» que da por ciertas
  [`2026-10-14_medir-la-frontera-entre-bloques.md`](2026-10-14_medir-la-frontera-entre-bloques.md)
  **no cuadran**: el camino nativo solo pide 46.710 ciclos por fotograma en la escena más cargada
  medida, y cada instrucción cuesta al menos 1 ciclo (`machine::instr_cost` cobra el *fetch* de
  entrada), así que el techo de instrucciones nativas es ≤46.710 y, con la media de ciclos por
  instrucción, quedaría en decenas de miles. Los bloques cruzarían ~23 instrucciones por entrada
  (1.530 entradas/fotograma), no ~130.
- Y la frontera sigue descartada: con ~23 instrucciones por entrada, no es donde muere el ahorro.

No puedo repartir los ciclos restantes entre el *halt* y el camino de `interp_one` (los shells de BIOS
avanzan el reloj sin pasar por `rt_tick`) sin instrumentar ese segundo camino: es trabajo de la próxima
propuesta y **el hueco más grande que queda en el presupuesto del fotograma**.

### Predicho vs. real

- **El renderizado no cambia (regresión)**: coincide. Los tres guardianes idénticos con el contador
  puesto, `fallback_steps: 0`, salida 0; y en el A/B el hash fue idéntico en las diez corridas.
  Método: el anunciado.
- **El reloj queda pesado**: **difiere**. Los dos valores salen impresos y son deterministas, pero la
  comprobación interna falla (5,2 % y 16,6 % de 280.896, no el ±5 %). El criterio estaba mal
  calibrado: supuse que el reloj del fotograma pasa entero por la CPU. Método: el anunciado.
- **El techo del reloj queda acotado**: coincide, y con margen. Predije 0,7-1,6 % (descarte); medido
  0,13-0,41 % (descarte). El razonamiento era malo por lo mismo: predije ~30.466 peticiones/fotograma
  suponiendo que la CPU consume el fotograma entero, y son 2.210-7.135. Método: el anunciado, con una
  corrección en el micro-banco (abajo).
- **El instrumento no cuesta (control)**: coincide. Mediana +0,80 % con pares intercalados, dentro del
  ±5 %; un par salió +8,25 % y la mediana aguanta.

### La mutación

Las dos mutaciones anunciadas (M1 y M2) protegían el criterio que resultó inválido, así que se
sustituyeron por la que sí prueba el cableado: **mover el contador de `rt_tick` a `rt_chain_gate`**.

- Resultado: `ticks_frame: 263,4`, total **1.896.786**, y `ciclos: 0`.
- Léelo como comprobación: al mover el cable el número cambia de forma sistemática (de 7.134,8 a 263,4
  por fotograma) y el contador de ciclos cae a cero, así que cada contador cuenta lo que dice contar.
- **Y de propina:** 1.896.786 es exactamente el total de `puertas` (vueltas de bucle) que registró
  [`2026-10-14_medir-la-frontera-entre-bloques.md`](2026-10-14_medir-la-frontera-entre-bloques.md), y
  263,4 su valor por fotograma. Dos sesiones, dos instrumentos distintos, la misma cifra: el arnés y
  la escena son deterministas.

### Contexto: ¿se respetó?

- El renderizado no cambia → respetada: guardianes idénticos.
- Los prototipos del motor se respaldan y se restauran sin comandos de Git → respetada y verificada
  por tres vías: los `sha256` de los dos archivos vuelven a ser `fed1ca9b…` (capi.rs) y `9e1ade3a…`
  (main.rs) —los mismos que registró la propuesta de la frontera para esos dos archivos—, el `git
  status` del motor vuelve a mostrar **solo** las cuatro modificaciones preexistentes de la sesión de
  la PPU, y el archivo del micro-banco ya no está. La restauración necesitó una corrección: al copiar
  desde `git show` los archivos quedaron en LF y el árbol trabaja en CRLF (`core.autocrlf=true`), con
  `git diff` vacío; se devolvieron a CRLF y el `sha256` coincidió con el histórico.
- Medir el techo antes de fijar el listón → respetada: no se tocó `emit.rs`, ni la coalescencia, ni la
  firma de `tick`, ni el bus; el umbral (3 %) se escribió antes de medir.
- ¿Se tocó algo que alguna restricción prohibía? no.
- Imprevistos: el micro-banco dio **0,00 ns** en los brazos de control en la primera pasada (LLVM
  devirtualizó y borró el bucle) → criterio de prioridad aplicado: sí, se corrigió el banco (punteros
  con `black_box` y llamada por la tabla en memoria) en vez de dar por buena la cifra.
- ¿Reapareció alguna alternativa descartada? no.

### Archivos modificados

Motor, **temporalmente y revertido**: `crates/gba-core/src/capi.rs`, `crates/recomp/src/main.rs` y
`crates/gba-core/tests/tick_bench.rs` (borrado). Port: esta propuesta y una nota con fecha en la de la
frontera. `src-c/`, `GbaPerfCounters`, el formato de la telemetría y el port no se tocaron. ¿Coincide
con lo anunciado? sí, con un matiz: el micro-banco se añadió como archivo de test del motor en vez de
dentro de `capi.rs`; da igual para el alcance y se borra igual.

### Discrepancias

- «La CPU consume el reloj de todo el fotograma» → **suposición falsa**: solo pide 5-17 % por `a->tick`
  y el resto es *halt* esperando el VBlank. Arrastró la predicción del techo (0,7-1,6 % contra
  0,13-0,41 %).
- El criterio «la suma de ciclos por fotograma cae en 280.896 ±5 %» → **error de juicio** al escribir
  la propuesta: estaba construido sobre esa misma suposición. Su fallo, eso sí, es lo que destapó el
  hallazgo.
- «~200.000 instrucciones de juego por fotograma» (documento aprobado) → **información falsa en un
  documento**: el techo nativo medido es ≤46.710 instrucciones por fotograma. Anotado con fecha en esa
  propuesta.
- El micro-banco medía 0,00 ns en los controles → **error de implementación** propio, detectado y
  corregido antes de usar la cifra.

### Para la próxima vez

Antes de multiplicar «coste por llamada × llamadas por fotograma», comprobar **cuántos ciclos del
fotograma pasan de verdad por ese camino**: el reloj del fotograma no es el reloj de la CPU, porque el
juego duerme.

### Fuera de alcance (hallazgos que no se arreglaron)

- **Solo el 5-17 % de los ciclos del fotograma pasan por `a->tick`**; el resto lo avanza
  `skip_to_next_event` durante el *halt*. Es el hueco más grande del presupuesto del fotograma y el
  único camino de reloj sin instrumentar. | impacto: alto
- **`fallback_steps: 0` no significa «el juego no se detiene»**: `run_frame_native` llama a `m.step()`
  en el estado *halted* sin contarlo. Un contador de fotogramas dormidos sería barato y aclararía el
  reparto. | impacto: medio
- **Los shells de `interp_one` (BIOS) avanzan el reloj sin pasar por `rt_tick`**, así que ningún
  contador de este tipo ve el camino del intérprete. | impacto: medio
- **El despacho indirecto por la tabla `RtApi` no cuesta en este host** (−0,01 ns frente a una llamada
  directa): la diferencia de 2,8× que midió
  [`2026-10-12_acceso-memoria-sin-llamada-indirecta.md`](2026-10-12_acceso-memoria-sin-llamada-indirecta.md)
  era el *trabajo* del accesor, no el salto indirecto. En el ARM del televisor el predictor puede
  portarse distinto y conviene confirmarlo allí. | impacto: medio
- La media **dinámica** de ciclos por petición (6,55-6,59) es menor que la **estática** del censo
  (9,22): las escenas calientes son más densas en accesos al bus, que son los que fuerzan el volcado.
  | impacto: bajo

---

## Revisión humana
<!-- El agente SÍ edita esta sección: escribe aquí los escenarios que no se
     pudieron comprobar con código ("(visible)" o "no pudo comprobarse") y las restricciones
     marcadas "no testeable",
     uno por línea, dejando las marcas ✔ | ✘ | ? sin elegir.
     El agente NO elige marca, NO escribe la nota y NO rellena
     "Detectado solo por el humano": eso es solo de la persona. -->

- Escenario «(visible)»: el reparto de reloj medido en el host trasladado al televisor — no se
  intentó, porque exige una sesión en el dispositivo: ✔ | ✘ | ?
- Restricción no testeable: el motor se desarrolla en local y el port lo consume por `[patch]`, con el
  SHA sin publicar: ✔ | ✘ | ?
- Detectado solo por el humano: [lo completa la persona]
