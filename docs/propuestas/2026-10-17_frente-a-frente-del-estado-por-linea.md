# Propuesta: Poner frente a frente el shim y la PPU por software con el estado de cada línea

Fecha: 2026-10-17
Nivel: 2
Estado: verificada por agente (la aprobación inicial fue la instrucción «pongámoslos frente a frente»; no hubo un «Aprobado» sobre texto escrito)

<!-- Estados: pendiente | aprobada | modificada | descartada |
     verificada por agente | revisada por humano | discrepada | cerrada -->

## Propuesta (la escribe el agente antes de editar)

### 1. Qué cambia

Un ejemplo que corre el juego una sola vez y, para cada cuadro de un rango, compara **dos** shims
contra la PPU por software: el de hoy (un estado para las 160 líneas) y uno hipotético que dibuja cada
línea con el estado capturado justo antes de esa línea. La diferencia entre ambos mide cuánto del error
visible se debe al modelo de un estado por fotograma y cuánto queda para el resto (blend, ventanas,
afines, VRAM).

### 2. Contexto

- Contexto consultado:
  - `docs/propuestas/2026-10-17_diagnostico-afin-y-por-linea.md` (Resultado): 0 de 1.112 cuadros con
    capas afines; 900 de 1.112 escriben E/S a mitad de cuadro; `BG0CNT`/`BG2CNT` se reescriben en 269
    cuadros con hasta 109 líneas dibujadas con un valor distinto al de la línea 0.
  - `docs/propuestas/2026-10-16_arnes-comparacion-shim-vs-software.md` (aprobada): ya existe
    `examples/ppu_compare.rs`, que compara el shim de hoy contra el software una vez por cuadro y
    escribe un CSV. **No** separa las causas ni usa el estado por línea.
  - `src/ppu_shim.rs`: `render_frame(device, queue, view, vram, oam, palette, io)` recibe el estado por
    parámetro, así que se puede invocar 160 veces con estados distintos sin tocar el shim.
  - `gba-core/src/mem.rs`: con `capturar_linea = true`, `capturas_linea` guarda `io`, `palette`,
    `vram` y `oam` de cada línea visible justo antes de dibujarla.
  - `tests/afin_y_linea_tests.rs`: patrón de prueba que lee un informe ya escrito y se omite sin `FE_ROM`.
- Intención: atribuir el error de píxeles a cada causa con números, sin cambiar el shim. Éxito: un
  informe donde el error del shim de hoy y el del shim con estado por línea aparecen por cuadro, y su
  diferencia se puede sumar.
- Restricciones y decisiones previas:
  - El shim no se modifica en esta propuesta (decisión previa: cada corrección del shim va en su
    propuesta) → implica: el experimento solo llama a su API pública → protegida por: alcance.
  - La PPU por software es la referencia (decisión previa, 2026-10-16) → implica: se compara contra
    `m.bus.framebuffer` del mismo cuadro → protegida por: «La referencia es la de siempre».
  - El trabajo es en el PC con Windows, no en Android (decisión previa, 2026-10-16) → implica: el
    ejemplo usa `wgpu` sin superficie y no entra en el APK → protegida por: alcance.
- Alternativas descartadas:
  - Comparar solo el shim de hoy contra el software → descartada: ya existe (`ppu_compare`) y no dice
    a qué se debe el error.
  - Dar estado por línea al shim de verdad (tabla por línea en el shader) → descartada para esta
    propuesta: es una corrección, no una medición, y merece su propio cambio.
  - Renderizar las 160 líneas una a una con una copia por línea → descartada por coste: agrupar las
    líneas que comparten estado reduce los renders a los estados distintos (normalmente 1 a 4).
- Ante imprevistos, prioriza: que la comparación sea honesta sobre que el informe sea rápido.

### 3. Ejemplos

- Caso normal: cuadro con 109 líneas dibujadas con un `BG2CNT` distinto del de la línea 0. Antes: el
  shim de hoy pinta las 160 líneas con el valor de fin de cuadro y se compara una vez. Ahora: el
  informe trae `estados 2`, `lineas_ajenas_al_fin 109`, `pct_fin 12.5`, `pct_linea 4.0`.
- Caso límite: cuadro de pantalla quieta (290-310). Ahora: `estados 1`, `lineas_ajenas_al_fin 0`,
  `pct_fin` y `pct_linea` idénticos, `delta_pct 0.0000`.

### 4. Pasos

1. Crear `examples/ppu_frente_a_frente.rs` (feature `proto`) con args
   `<ruta.gba> <secuencia.txt> <desde> <hasta> [paso]`.
2. Por cada cuadro del rango: activar `capturar_linea`, correr el cuadro y apagarlo.
3. Imagen **A**: shim llamado una vez con el estado de fin de cuadro (lo que hace hoy `pipeline.rs`).
4. Imagen **B**: agrupar las 160 capturas por estado idéntico (`io`, `palette`, `vram`, `oam`);
   renderizar una vez por grupo y tomar de cada render las filas de ese grupo.
5. Comparar A y B contra `m.bus.framebuffer` del mismo cuadro con la misma tolerancia que
   `ppu_compare` (±1 por canal) y escribir una fila por cuadro en `volcados/frente-a-frente.csv`.
6. Resumen: cuadros, rango de estados por cuadro, error medio de A, error medio de B, reducción
   atribuible al estado por línea y residuo.
7. Escribir `tests/frente_a_frente_tests.rs` y la entrada `[[example]]` en `Cargo.toml`.
8. No se toca `src/`, `gba-core` ni el juego.

### 5. Qué más podría verse afectado

- `Cargo.toml`: una entrada `[[example]]`.
- `tests/frente_a_frente_tests.rs` y `volcados/frente-a-frente.csv`: nuevos (el volcado no se versiona).
- Nada de `src/`: el ejemplo usa `PpuShimPipeline` y `Machine`, ambos públicos.
- Coste: un render por grupo y cuadro (1 a 4 típicos) más el render de A; la emulación es la misma que
  en `ppu_afin_y_linea`.

### 6. Suposiciones

- Las 160 capturas son el estado con el que la PPU por software dibuja cada fila → prototipar: no →
  si es falsa: la imagen B compararía filas con estados desfasados y el error de B subiría sin motivo,
  lo que se vería como B peor que A en cuadros quietos.
- `render_frame` solo depende del estado que recibe, no del número de línea → prototipar: no → si es
  falsa (el shader recibiera la línea por fuera), B sería imposible sin tocar el shim.
- La diferencia A−B es cota superior de lo que el estado por línea puede arreglar → prototipar: no →
  si es falsa: la reducción estaría sobrestimada por el redondeo de la paleta.

### 7. Alcance

- Incluye: ejemplo de comparación A/B, agrupación de estados, CSV por cuadro, resumen, pruebas.
- No incluye: cambiar el shim; blend; ventanas; sprites afines; el visor en vivo; Android; el informe
  por línea de `ppu_compare`.

### 8. Criterios de aceptación (no se modifican sin aprobación)

Escenario: El informe cubre el rango pedido con una fila por cuadro
Caso de uso: comparar cuadros concretos sin abrir el shim | Tipo: normal
  Dado el rango 6280 a 6300 con paso 5
  Cuando se corre el ejemplo
  Entonces `volcados/frente-a-frente.csv` tiene 5 filas con los cuadros 6280, 6285, 6290, 6295 y 6300,
  cada una con `estados`, `lineas_con_cambio`, `lineas_ajenas_al_fin`, `pct_fin`, `pct_linea`,
  `delta_pct`, `primera_fin` y `primera_linea`
Test: `tests/frente_a_frente_tests.rs::test_una_fila_por_cuadro` (se omite si falta `FE_ROM`)
Estado previo: falla (el informe no existe)

Escenario: Los valores del informe son posibles
Caso de uso: no perseguir un porcentaje inventado | Tipo: límite
  Dado el informe del ejemplo
  Cuando se leen sus filas
  Entonces todo `pct_*` está entre 0 y 100 con cuatro decimales, `delta_pct` es `pct_fin - pct_linea`
  con ±0,0002, y `primera_*` es un entero de 0 a 159 o `-`
Test: `tests/frente_a_frente_tests.rs::test_formato_del_informe` (se omite si falta `FE_ROM`)
Estado previo: falla (el informe no existe)

Escenario: El resumen se puede reconstruir desde las filas
Caso de uso: confiar en las medias sin recalcularlas a mano | Tipo: normal
  Dado el informe del ejemplo
  Cuando se comparan sus líneas de resumen con las filas
  Entonces el número de cuadros y las dos medias coinciden con lo calculado sobre las filas
Test: `tests/frente_a_frente_tests.rs::test_resumen_coincide` (se omite si falta `FE_ROM`)
Estado previo: falla (el informe no existe)

Escenario: La comparación ejercita de verdad el estado por línea
Caso de uso: que B no sea una copia de A | Tipo: límite
  Dado el informe del ejemplo
  Cuando se busca un cuadro con más de un estado distinto
  Entonces existe al menos uno con `estados` mayor que 1 y `lineas_ajenas_al_fin` mayor que 0
Test: `tests/frente_a_frente_tests.rs::test_estado_por_linea_se_ejercita` (se omite si falta `FE_ROM`)
Estado previo: falla (el informe no existe)

### 9. Cómo lo comprobaré

- Una fila por cuadro → comando:
  `cargo run --release --features proto --example ppu_frente_a_frente -- "roms/1575 - Fire Emblem (E)(Independent).gba" secuencias/intro.txt 6280 6300 5`
  → pasa si: 5 filas, de 6280 a 6300 → no detecta: cuadros fuera del rango.
- Formato y resumen → `cargo test --features proto --test frente_a_frente_tests` → pasa si: las cuatro
  pruebas quedan verdes → no detecta: que la comparación sea justa (eso lo dice la lectura).
- Estado por línea ejercitado → la misma suite → pasa si: existe un cuadro con 2 o más estados → no
  detecta: que B se haya construido con el estado equivocado en alguna fila.
- Regresión → `cargo test` → pasa si: la suite del port sigue en verde → no detecta: el aspecto.

### 10. Terminado cuando

Se cumplen los cuatro criterios y el Resultado dice, con números, cuánto error explica el estado por
línea y cuánto queda para el resto de causas.

### 11. Parar si

- Las capturas no son el estado con el que se dibuja cada fila (primera suposición falsa): parar y
  presentar la evidencia antes de tocar `gba-core`.
- Dos intentos fallidos seguidos con el mismo criterio: parar y marcar la propuesta como `discrepada`.

---

## Resultado (lo completa el agente después de editar)

Ejecutado el 2026-10-17 con `examples/ppu_frente_a_frente` sobre la introducción completa
(`secuencias/intro.txt`, cuadros 0 a 11110 paso 250: 45 cuadros). `tests/frente_a_frente_tests.rs`:
**4 pruebas pasan** con `FE_ROM`. Mutación sobre el delta (`delta_pct` = `pct_fin`):
`test_formato_del_informe` falla. El archivo se restauró desde memoria; la suite se volvió a correr después (ver Trazabilidad).

| Medida (45 cuadros) | Valor |
| :--- | ---: |
| Error medio con el shim de hoy (fin de cuadro) | 36,59 % |
| Error medio con estado por línea | 31,70 % |
| Reducción atribuible al estado por línea | 4,89 puntos |
| Cuadros con un solo estado (9) | 0,16 % medio; 5 exactos (0,00 %) |
| Cuadros con varios estados (36) | 45,70 % → 39,59 % |
| Cuadros con mezcla real, `blend_lineas` > 0 (26) | 62,11 % → 53,90 % |
| Cuadros sin mezcla real, `blend_lineas` = 0 (19) | 1,68 % → 1,33 % |
| Reducción máxima | 60,99 puntos (cuadro 10500: 73,91 % → 12,92 %) |
| Cuadros donde el estado por línea empeora | 1 (cuadro 6000, +0,005 puntos: 2 píxeles) |

Cuatro cuadros que aíslan cada causa, con los registros leídos en `ppu_compare`:

| Cuadro | `DISPCNT` | `BLDCNT` | Fin | Por línea | Causa que queda |
| ---: | :--- | :--- | ---: | ---: | :--- |
| 10000 | `1f00` (sin ventanas) | `3c22` (sin efecto) | 0,00 % | 0,00 % | ninguna: el shim acierta píxel a píxel |
| 10500 | `1f00` | `3f28` (sin modo de efecto; 29 líneas con mezcla) | 73,91 % | 12,92 % | mezcla sin modo de efecto, probablemente sprites semitransparentes (no medido) |
| 4750 | `1f00` | `3f61` (alfa) | 97,50 % | 97,50 % | mezcla: el estado por línea no mueve la aguja |
| 6280 | `3f00` (bit 13: **WIN0**) | `3f3f` (sin efecto) | 85,02 % | 84,82 % | ventana: el software recorta a negro, el shim dibuja todo |

Predicho vs. real:

- «Una fila por cuadro»: **coincide** (5 filas, 6280 a 6300).
- «Los valores son posibles»: **coincide** (porcentajes en rango, delta exacto, primeras líneas válidas).
- «El resumen se reconstruye»: **coincide** (cuadros y ambas medias dentro de 0,001).
- «Se ejercita el estado por línea»: **coincide** (los cinco cuadros canónicos tienen 2 estados y 109
  líneas ajenas al fin; en la introducción completa hay hasta 81 estados en un cuadro).
- Suposición 1 (las capturas son el estado con el que se dibuja cada fila): **se sostiene**, porque
  en cuadros sin blend ni ventanas B queda en 0,00 a 1,33 % y nunca empeora de forma apreciable.
- «Terminado cuando el Resultado dice cuánto explica cada causa»: **coincide**, con una tercera causa
  que no estaba en el plan (ventanas) y que el propio informe destapó.

Lectura:

1. **El shim dibuja bien cuando el estado no cambia dentro del cuadro.** En los cuadros con un solo
   estado el error medio es 0,16 % y 5 de los 9 son exactos (0 píxeles de 38.400). Los otros 4 tienen
   error y este informe no los explica. Tampoco prueba si el muestreo de tiles o la paleta influyen: no
   se midieron por separado.
2. **El estado por fotograma es la primera causa cuantificable**: hasta 60,99 puntos de error en un
   solo cuadro. Donde el juego reescribe `BG0CNT`/`BG2CNT` o el scroll a mitad de cuadro, el shim pinta
   las 160 líneas con el valor de fin de cuadro.
3. **La mezcla parece la causa más extendida, pero es una hipótesis.** En 26 de 45 cuadros el software
   mezcla píxeles (`blend_lineas` > 0), y en esos cuadros el error queda en 53,90 % medio aunque cada línea tenga su
   estado. El shim no implementa mezcla, así que este informe no mide cuánto de ese error es mezcla: lo
   atribuye por correlación. En el cuadro 4750 (un solo caso) el estado por línea no cambia nada
   (97,50 % en ambos).
4. **Las ventanas son la causa que faltaba en el diagnóstico**: en el cuadro 6280 `DISPCNT` es `3f00`,
   con el bit 13 (WIN0) activo, el shim pinta la escena completa y difiere del software en el 85,02 % de los
   píxeles, error que ni la mezcla ni el estado por línea explican. Qué región recorta la ventana no se
   midió en ningún informe. El rango
   6280-6300 baja a 2,39 % cuando la escena cambia, con el mismo shim; no se leyó `DISPCNT` en cada
   cuadro del rango, solo en el 6280.
5. **Queda un residuo pequeño sin explicar**: 1,33 % medio en cuadros sin mezcla, con picos de 3,3 %
   en los que cambian 109 líneas. B ya lleva el estado por línea de VRAM, OAM y paleta, así que en
   principio descarta un cambio de VRAM dentro del cuadro. Candidatos: sprites u otro estado no capturado
   (por ejemplo `aff_ref`, ver más abajo). Sin prueba todavía.

Cambios tras prototipos (el plan cambió):

- El CSV gana dos columnas, `bldcnt` y `blend_lineas`, para poder atribuir el residuo a la mezcla sin
  abrir el shim. No sustituyen a ninguna de las pactadas: se añaden al final.
- El informe se escribe en un solo archivo (`volcados/frente-a-frente.txt`) con las líneas de resumen y
  el CSV juntos, para que las pruebas lean ambas cosas del mismo artefacto.

Archivos: `examples/ppu_frente_a_frente.rs` y `tests/frente_a_frente_tests.rs` (nuevos), `Cargo.toml`
(una entrada `[[example]]`), `volcados/frente-a-frente.txt` (generado, no versionado). El shim,
`ppu.rs`, `gba-core` y el juego no cambian.

### Trazabilidad (añadido tras revisión)

- La tabla de 45 cuadros se regeneró el 2026-10-17 con `ppu_frente_a_frente 0 11110 250` y se guardó en
  `volcados/frente-a-frente-intro.txt`. Reproduce 36,5944 % (fin) y 31,7035 % (por línea), iguales a los de la tabla.
- `volcados/frente-a-frente.txt` guarda la última ejecución (6280 a 6300). Las pruebas leen ese mismo archivo y
  esperan ese rango: si se corre otro, fallan. Pendiente: pasar el informe por parámetro o por nombre de rango.
- Pruebas: `tests/frente_a_frente_tests.rs` pasa 4 de 4 tras regenerar el informe de 6280 a 6300 (la suite corrió sobre el informe regenerado, no sobre una copia restaurada).
- Criterio de mezcla: la columna `blend_lineas` > 0 da los 26 cuadros del grupo con mezcla. `BLDCNT` distinto de 0
  ocurre en 43 cuadros, por eso ese criterio no se usa. Las medias de cada grupo se recalcularon desde el CSV.

### Suposiciones y límites de la muestra (añadido tras revisión)

- Suposición 1 (las capturas son el estado con el que se dibuja cada fila): se sostiene de forma
  indirecta. En cuadros sin mezcla ni ventanas, B queda entre 0,00 % y 1,33 %. No hay una comparación
  directa fila a fila.
- Suposición 2 (`render_frame` depende solo del estado que recibe): no verificada en este Resultado.
- Suposición 3 (A−B es cota superior de lo que el estado por línea arregla): no verificada en este
  Resultado.
- Estado de afines: el agrupado no incluye `aff_ref` ni `aff_dirty`. El diagnóstico previo encontró 0 de
  1.112 cuadros con capas afines, así que no afecta a esta muestra. Si se usa otra secuencia, hay que
  añadirlos.
- Muestra: 45 cuadros cada 250 de una sola secuencia (la introducción). No representa el juego completo;
  las conclusiones valen para esa muestra.

### Fuera de alcance (hallazgos que no se arreglaron)

- La mezcla (`BLDCNT`/`BLDALPHA`/`BLDY`) y las ventanas (`WIN0`/`WIN1`/`OBJWIN`) siguen sin existir en
  el shim: el experimento las mide, pero no las corrige. — impacto: medio
- El residuo de 1,33 % medio en cuadros sin mezcla (picos de 3,3 %) no está explicado: apunta a VRAM
  que cambia dentro del cuadro o al dibujado de sprites, y necesita su propio experimento. — impacto: bajo
- El ejemplo necesita GPU y tarda unos 5 s por cuadro cuando hay decenas de estados, así que no puede
  entrar en la suite de CI: las pruebas solo leen el informe ya generado. — impacto: bajo

---

## Revisión humana

- Que la tabla de 45 cuadros sea reproducible (ver Trazabilidad), que el criterio de mezcla sea `blend_lineas` > 0, y que las conclusiones se limiten a la muestra de la introducción: ✔ | ✘ | ?
