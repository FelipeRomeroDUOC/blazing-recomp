# Propuesta: Medir la frontera entre bloques

Fecha: 2026-10-14
Nivel: 2
Estado: verificada por agente

<!-- Estados: pendiente | aprobada | modificada | descartada |
     verificada por agente | revisada por humano | discrepada | cerrada -->

## Propuesta (la escribe el agente antes de editar)

### 1. Qué cambia

Durante una corrida de laboratorio —**no en el APK**— el motor cuenta cuántas veces el código nativo
cruza una puerta de bucle y cuántas veces esa puerta manda volver al despachador, y el arnés lo
imprime junto al censo que ya imprime. Es instrumentación temporal, para saber si la frontera entre
bloques cuesta de verdad; se revierte antes de dar la medida por buena.

### 2. Contexto

- Contexto consultado: `../gba-recomp/crates/recomp/src/main.rs` (`run_frame_native` en 2225 y
  `cmd_runc` en 2627, con la línea de resumen en 2781), `../gba-recomp/crates/gba-core/src/capi.rs`
  (`rt_chain_gate` y la tabla `RtApi`), `../gba-recomp/crates/gba-core/src/mem.rs`
  (`GbaPerfCounters`), `../gba-recomp/crates/recomp-core/src/emit.rs` (`transfer`, el que emite la
  puerta y el `goto`), el C generado en [`src-c/`](../../src-c/) y esta misma sesión
  ([`2026-10-14_los-registros-del-juego-viven-en-la-cpu.md`](2026-10-14_los-registros-del-juego-viven-en-la-cpu.md)).
- Intención: poner un número al reparto entre **despachador** y **trabajo dentro del bloque**, porque
  el hallazgo de la palanca 4 dice que la frontera —no el interior— es donde muere el ahorro. El
  éxito es una cifra: cuántas vueltas de bucle y cuántos retornos al despachador se pagan por
  fotograma.
- Restricciones y decisiones previas (máx. 3):
  - El renderizado no cambia (origen: [`2026-10-12_acceso-memoria-sin-llamada-indirecta.md`](2026-10-12_acceso-memoria-sin-llamada-indirecta.md))
    → implica: los tres hashes guardianes y `fallback_steps: 0` siguen idénticos con el contador
    puesto → protegida por: *El renderizado no cambia (regresión)*.
  - Los prototipos del motor se respaldan en RAM y se restauran en un bloque `finally`, sin comandos
    de Git (origen: `AGENTS.md`, regla de restauración) → implica: el instrumento no queda en el
    árbol al terminar → protegida por: la comprobación posterior (`git status` del motor).
  - El motor se desarrolla en local y el port lo consume por `[patch]`, con el SHA sin publicar
    (origen: [`Cargo.toml`](../../Cargo.toml)) → implica: el instrumento no viaja al APK; publicar el
    motor es trabajo aparte → protegida por: no testeable (queda en «Revisión humana»).
- Alternativas descartadas:
  - Contar desde el emisor (emitir un incremento por puerta en el C generado) → descartada porque
    cambiaría el código que se mide; el contador dentro de `rt_chain_gate` cuenta sin tocar la
    emisión.
  - Deducirlo del censo estático → descartada porque el censo no pesa por frecuencia de ejecución:
    ya se midió que hay 5.540 grupos, 196.461 `goto` internos, 19.894 puertas y 81.572 `return`, pero
    un bucle de tres líneas puede dar millones de vueltas y otro, ninguna.
  - Instrumentar el APK → descartada: el contador debe vivir en una corrida de laboratorio, no en un
    fotograma de 16,67 ms.
- Ante imprevistos, prioriza: que la medida no altere lo medido.

### 3. Ejemplos

- **Caso normal:** la línea de resumen del arnés
  antes: `frames: 7200 hash: b54066dc06f2b76c native_blocks: 3208998 fallback_steps: 0`
  ahora (objetivo): la misma línea más `puertas: P` y `puertas_que_vuelven: V`, con `P` y `V` por
  fotograma en el desglose.
- **Caso límite:** si en 7.200 fotogramas `V` sale **0**, la puerta no manda volver nunca y el coste
  de la frontera es solo el despachador por bloque; si `V` es una fracción grande de `P`, la puerta
  es un sospechoso propio.
- **Caso de control:** la corrida con el contador y la corrida sin él dan el mismo hash y los mismos
  `native_blocks`; si no, la medida es inválida.

### 4. Pasos

1. Añadir en `crates/gba-core/src/capi.rs` dos contadores estáticos (`puertas` y
   `puertas_que_vuelven`), incrementados dentro de `rt_chain_gate`, con una función de lectura.
   **La forma de `RtApi` no cambia**: el contador es estado global del proceso, no un campo de la
   tabla.
2. Añadir en `cmd_runc` (`crates/recomp/src/main.rs`) los dos valores a la línea de resumen y al
   desglose por fotograma.
3. Medir: 7.200 fotogramas con el mismo `--input` y el mismo BIOS, tres corridas, leyendo
   `native_blocks`, `puertas`, `puertas_que_vuelven` y el tiempo de pared.
4. A/B del propio instrumento: la misma corrida con y sin los incrementos, en pares intercalados
   (≥5), para separar lo que cuesta contar de lo que se quiere medir.
5. Revertir los dos archivos desde la copia en RAM y comprobar el `git status` del motor.

### 5. Qué más podría verse afectado

Búsquedas hechas: `grep -rn "RtApi" ../gba-recomp/crates/`, `grep -rn "chain_gate" ../gba-recomp/crates/`
y `grep -rn "take_perf_counters|GbaPerfCounters" ../gba-recomp/crates/`.

- `../gba-recomp/crates/gba-core/src/capi.rs` — `rt_chain_gate` (donde va el incremento). Los puentes
  y la tabla no cambian; el contador es `static` con orden `Relaxed`.
- `../gba-recomp/crates/recomp/src/main.rs` — `cmd_runc`, solo la impresión.
- `../gba-recomp/crates/gba-core/src/mem.rs` — **no se toca**: se evita añadir campos a
  `GbaPerfCounters`, que el port consume, para no rozar el formato de la telemetría.
- [`src-c/`](../../src-c/) — no se regenera: el C generado no cambia.
- El port — no se toca.
- Sin más referencias encontradas.

### 6. Suposiciones

- **Un incremento atómico relajado no altera de forma medible el fotograma.** → prototipar: sí, con el
  A/B intercalado del paso 4 y `ppu`/`apu` como control → si es falso: se mide con el contador y se
  descuenta el coste medido del propio contador, o se cuenta por muestreo (uno de cada 64).
  → resultado del prototipo: **confirmada**. Los pares intercalados dan +197, +46 y −79 ms sobre
  ~8.800 ms: mediana **+0,5 %**, dentro del ±5 % del criterio. | no detecta: nada relevante; el A/B lo
  cubre.
- **`rt_chain_gate` es la única puerta que el código nativo cruza.** → prototipar: no (se comprueba
  leyendo `emit.rs`: es el único punto que emite `chain_gate`) → si es falso: habría más fronteras sin
  contar y la medida sería un suelo, no un total.
  → no detecta: puertas que el emisor añada en el futuro.
- **La corrida de host es representativa del reparto, aunque no de los milisegundos.** → prototipar:
  no (el host compila con TinyCC y el dispositivo con clang) → si es falso: el reparto se confirma en
  el dispositivo en la propuesta que use estos números.
  → no detecta: diferencias de frecuencia de ejecución entre escenas.

### 7. Alcance

- **Incluye:**
  - El contador de puertas en `capi.rs` y su impresión en `cmd_runc` (motor), temporal.
  - La corrida de 7.200 fotogramas y su A/B intercalado.
  - El número por fotograma: vueltas de bucle y retornos al despachador.
- **No incluye:**
  - Cambiar la emisión, el tamaño de los grupos de cadena o la puerta: si de aquí sale una palanca,
    va en su propia propuesta.
  - Tocar `GbaPerfCounters`, el formato de la telemetría o el port.
  - Publicar el motor o re-fijar el SHA.

### 8. Criterios de aceptación (no se modifican sin aprobación)

**Escenario: El renderizado no cambia (regresión)**
Caso de uso: cualquier corrida con el instrumento puesto | Tipo: regresión
  Dado el arnés con la ROM y el BIOS del usuario y el mismo `--input`
  Cuando se ejecutan 600, 2400 y 7200 fotogramas con los contadores activos
  Entonces los hashes son los tres guardianes (`31184031261ea2ec`, `b54066dc06f2b76c`,
  `b54066dc06f2b76c`), `fallback_steps: 0` y `native_blocks` idénticos a los de la corrida sin
  contador
Test: `recomp runc --frames 600|2400|7200` (comando del apartado 9)
Estado previo: **pasa** (es el guardián, debe seguir pasando)

**Escenario: La frontera queda medida**
Caso de uso: diagnóstico en host | Tipo: normal
  Dado el arnés con los contadores puestos
  Cuando se corren 7.200 fotogramas
  Entonces la línea de resumen imprime `puertas` y `puertas_que_vuelven`, y quedan escritos los dos
  valores por fotograma junto a `native_blocks` por fotograma
Test: corrida del paso 3
Estado previo: **no existe el dato** (hoy no se cuenta)

**Escenario (control): el instrumento no cuesta**
Caso de uso: validez de la medida | Tipo: límite
  Dado el mismo `.dll` y la misma corrida
  Cuando se comparan, con ≥5 pares intercalados, la corrida con los incrementos y la corrida sin
  ellos
  Entonces la mediana pareada de la diferencia queda dentro del ±5 % y `ppu`/`apu` no se mueven
Test: corrida del paso 4
Estado previo: **no medido**

### 9. Cómo lo comprobaré

- **El renderizado no cambia** → comando:
  `../gba-recomp/gba-recomp-launcher-0.4-x86_64-pc-windows-msvc/recomp.exe runc "roms/1575 - Fire Emblem (E)(Independent).gba" --bios roms/gba_bios.bin --input scripts/diagnostico/entrada_avance.input --frames 7200`
  → pasa si: imprime `hash: b54066dc06f2b76c` y `fallback_steps: 0`, código de salida 0
  → no detecta: que la escena del `--input` sea la de más tráfico de frontera (mide la que está
  grabada).
- **La frontera queda medida** → comando: la misma corrida, leyendo `puertas` y `puertas_que_vuelven`
  → pasa si: los dos valores salen impresos y son consistentes entre las tres corridas
  → no detecta: el peso en milisegundos de la frontera; eso pide la fase siguiente.
- **El instrumento no cuesta** → comando: el A/B intercalado del paso 4
  → pasa si: la mediana pareada queda en ±5 % con los controles planos
  → no detecta: diferencias entre ejecuciones de más de ±13 % (el ruido ya medido del arnés); por eso
  se usan pares intercalados.
- **Nada más se rompe** → comando: `cargo test -p gba-core` y `cargo test -p recomp` en el motor
  → pasa si: la suite del motor está en verde
  → no detecta: el port, que no se toca.

**Blindaje por mutación (fase 4 del pipeline, previsto):**
M1 — no incrementar `puertas_que_vuelven` cuando la puerta devuelve 1 → debe morir el escenario
*La frontera queda medida* (el valor quedaría en 0 por construcción).
M2 — incrementar el contador fuera de `rt_chain_gate` (una vez por bloque en vez de por puerta) →
debe morir el escenario *La frontera queda medida*.
M3 — dejar el incremento sin `Relaxed` y con `SeqCst` → debe morir *el instrumento no cuesta* si el
coste se sale del ±5 % (si no se sale, la mutación sobrevive y hay que apretar el criterio).
Los archivos se respaldan en memoria RAM y se restauran en un bloque `finally`, sin comandos de Git.

### 10. Terminado cuando

Los tres escenarios pasan: los guardianes siguen idénticos, los dos contadores quedan impresos y
estables, y el instrumento no se separa del ±5 %. El motor queda restaurado y el `git status` sin
rastro del prototipo.

### 11. Parar si

- 2 intentos fallidos seguidos con el mismo criterio: detenerse, explicar qué se probó y marcar la
  propuesta como `discrepada`.
- El instrumento cuesta más del 5 % y no se puede abaratar: se mide con muestreo; si aun así no se
  separa del ruido, se para y se cierra con lo que el censo estático ya dice.

### Correcciones previas a presentar

- **Corregido antes de presentar:** mi primer borrador iba a añadir los contadores a
  `GbaPerfCounters`. Al leer `src/recomp.rs` y `src/runner.rs` se ve que el port consume esas
  estructuras, así que añadir campos habría rozado el formato de la telemetría sin necesidad: el
  contador pasa a ser un `static` de `capi.rs` con su propia lectura.
- **Corregido también:** el borrador decía «medir la frontera» sin decir qué se cuenta. La frontera
  son dos cosas distintas —la puerta de bucle y el retorno al despachador— y por eso se cuentan por
  separado: `native_blocks` ya cuenta los retornos, faltaba la puerta.

### Cambios tras prototipos (solo si el plan cambió)

- Ninguno: el plan se ejecutó tal cual (contador en `rt_chain_gate`, impresión en `cmd_runc`, A/B
del instrumento).

---

## Resultado (lo completa el agente después de editar)

**Verificada por agente.** La frontera quedó medida, el instrumento no costó y el motor se restauró
byte a byte. La conclusión es un **resultado negativo con un hallazgo inesperado**: la frontera es
frecuente pero pequeña, y en su lugar aparece un coste mucho mayor que nadie estaba contando.

### Qué se midió

Contadores en `rt_chain_gate` (`puertas` y `puertas_que_vuelven`), activados con `RECOMP_PUERTAS`,
impresos en la línea de resumen de `cmd_runc`.

**Guardianes (sin `--input`), con el contador puesto — idénticos a los registrados:**

| fotogramas | hash | native_blocks | fallback_steps |
|---|---|---|---|
| 600 | `31184031261ea2ec` | 1.776.798 | 0 |
| 2400 | `b54066dc06f2b76c` | 2.167.398 | 0 |
| 7200 | `b54066dc06f2b76c` | 3.208.998 | 0 |

**Frontera (escena de avance, `--input scripts/diagnostico/entrada_avance.input`, 7.200 fotogramas):**

| magnitud | total | por fotograma |
|---|---|---|
| `native_blocks` (vueltas al despachador) | 11.018.334 | **1.530,3** |
| `puertas` (vueltas de bucle) | 1.896.786 | **263,4** |
| `puertas_que_vuelven` | 22.765 | **3,16** (1,2 % de las puertas) |

Tiempo de pared: 8.976 ms para 7.200 fotogramas (1,25 ms/fotograma).

**Escena de arranque/menú (sin `--input`):** las puertas caen a 182,7 por fotograma a 600
fotogramas, 106,4 a 2.400 y 89,5 a 7.200, y **la puerta manda volver solo 7 veces en las tres
corridas juntas**.

**Coste del propio instrumento:** pares intercalados +197, +46 y −79 ms sobre ~8.800 ms → mediana
**+0,5 %**, dentro del ±5 %.

### Lo que dicen los números

1. **La puerta de bucle es casi siempre un no-op.** Se cruza 90-263 veces por fotograma y devuelve
   «vuelve al despachador» entre el 0 % (7 veces en 21.600 fotogramas en el arranque) y el 1,2 %
   (escena de avance). Es, en la práctica, una llamada indirecta por vuelta de bucle que casi
   siempre dice «sigue».
2. **La puerta no es la frontera dominante; el despachador sí.** Hay **6 a 16 veces más vueltas al
   despachador (1.530/fotograma) que puertas de bucle (263/fotograma)**.
3. **Pero la frontera entera es pequeña frente al fotograma.** Una vuelta al despachador son unas
   40 instrucciones nativas (una búsqueda en tabla, tres comprobaciones de hooks, dos llamadas y un
   `switch`), del orden de 15-20 ns. Aun contando **100 ns por vuelta** —cinco veces lo razonable—
   1.530-2.961 vueltas por fotograma son **0,15-0,30 ms, el 1-2 % de un fotograma de 16,67 ms**.
4. Y eso encaja con el tamaño de los grupos: 1.530 entradas por fotograma para el trabajo de todo un
   fotograma (~200.000 instrucciones de juego) significa que la frontera se cruza cada ~130
   instrucciones. No es un cuello de botella.

> [!NOTE]
> **Actualizado (2026-10-14, por [`2026-10-14_acotar-el-techo-del-reloj-por-instruccion.md`](2026-10-14_acotar-el-techo-del-reloj-por-instruccion.md)).**
> Las «~200.000 instrucciones de juego por fotograma» eran una estimación y **no cuadran**: el contador
> del reloj muestra que el código nativo solo pide **46.710 ciclos por fotograma** en esta misma escena
> de avance, y cada instrucción cuesta al menos 1 ciclo, así que el techo es ≤46.710 instrucciones —del
> orden de decenas de miles con la media real de ciclos por instrucción—. Los bloques cruzarían ~23
> instrucciones por entrada, no ~130. **La conclusión de esta propuesta no cambia** (la frontera queda
> descartada, y con más razón); lo que cambia es el tamaño del tramo entre puertas. La cifra de puertas
> (1.896.786 en total, 263,4 por fotograma) quedó confirmada de forma independiente por ese contador.

**Conclusión: la frontera no es una palanca.** Atacarla (grupos más grandes, menos `return`, volcado
parcial) rescataría como mucho uno o dos puntos porcentuales. Queda cerrada, igual que la palanca 4,
que la había señalado como sospechosa.

### El hallazgo inesperado: el reloj por instrucción

Buscando por qué la frontera no explicaba nada, apareció el coste que **sí** domina el camino de
llamadas. Contando las llamadas emitidas en todo el C generado:

| llamada | emitidas | % |
|---|---|---|
| `a->tick(...)` | **400.767** | **32,1 %** |
| `a->read*` | 263.232 | 21,1 % |
| `a->write*` | 583.583 | 46,8 % |
| total | 1.247.582 | |

El generador emite **un `a->tick(m, ciclos)` después de casi cada instrucción** (el C lo muestra: un
`a->tick` cada una o dos sentencias). Comparado con lo que se acaba de medir, eso son del orden de
**4 veces más llamadas que vueltas al despachador y ~20 veces más que puertas de bucle**. La frontera
de la que veníamos a hablar no era la frontera real: la real es el reloj.

### Predicho vs. real

- **El renderizado no cambia (regresión)**: coincide. Los tres hashes guardianes y `fallback_steps:
  0` salen idénticos con el contador puesto. Matiz: el comando del apartado 9 llevaba `--input`, y
  los guardianes están grabados **sin** él; se corrieron de las dos formas (sin `--input` para el
  guardián, con `--input` para medir la frontera), que es lo correcto.
- **La frontera queda medida**: coincide, y con más de lo pedido. Se obtuvieron vueltas al
  despachador y vueltas de bucle por fotograma, y además la tasa de retorno de la puerta.
- **El instrumento no cuesta**: coincide. Mediana +0,5 % con pares intercalados.

### Contexto: ¿se respetó?

- El renderizado no cambia → respetada: los tres guardianes idénticos.
- Los prototipos del motor se respaldan en RAM y se restauran sin comandos de Git → respetada: los
dos archivos quedaron **byte a byte idénticos** (`sha256` `fed1ca9b…` y `9e1ade3a…`) y el `git
status` del motor no ganó ninguna línea.
- El motor no se publica → respetada: no se tocó el SHA del port.
- ¿Se tocó algo que alguna restricción prohibía? no.
- Imprevistos: ninguno.
- ¿Reapareció alguna alternativa descartada? no.

### Archivos modificados

Motor, **temporalmente y revertido**: `crates/gba-core/src/capi.rs` y `crates/recomp/src/main.rs`.
Port: solo esta propuesta. `GbaPerfCounters`, el formato de la telemetría y `src-c/` no se tocaron.
¿Coincide con lo anunciado? sí.

### Para la próxima vez

Antes de creer que la frontera es el cuello de botella, medir su **recuento por fotograma**: resultó
ser 1.530 vueltas de despachador frente a ~200.000 instrucciones de juego, es decir, un 1-2 % del
fotograma. El coste estaba en otra llamada, mucho más frecuente.

### Fuera de alcance (hallazgos que no se arreglaron)

- **`a->tick` se emite prácticamente una vez por instrucción de juego** (32,1 % de todas las
  llamadas del C generado; ~4× las vueltas al despachador). Es una llamada indirecta por instrucción
  para sumar ciclos. Es el candidato con más volumen de todo el camino de llamadas y merece su propia
  propuesta. | impacto: alto
- **La puerta de bucle devuelve «sigue» casi siempre** (0 % en el arranque; 1,2 % en el avance): su
  llamada indirecta por vuelta es casi puro coste, aunque el recuento es bajo (~90-263 por fotograma).
  | impacto: bajo
- **La frontera queda descartada como palanca** con número: 1-2 % del fotograma como techo. |
  impacto: bajo (cierra una vía abierta por la palanca 4)

---

## Revisión humana
<!-- El agente SÍ edita esta sección: escribe aquí los escenarios que no se
     pudieron comprobar con código ("(visible)" o "no pudo comprobarse") y las restricciones
     marcadas "no testeable",
     uno por línea, dejando las marcas ✔ | ✘ | ? sin elegir.
     El agente NO elige marca, NO escribe la nota y NO rellena
     "Detectado solo por el humano": eso es solo de la persona. -->

- Restricción no testeable: el motor se desarrolla en local y el port lo consume por `[patch]`, con el
  SHA sin publicar: ✔ | ✘ | ?
- Detectado solo por el humano: [lo completa la persona]
