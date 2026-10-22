# Propuesta: Diagnosticar si los glitches del shim vienen de capas afines o de estado por línea

Fecha: 2026-10-17
Nivel: 2
Estado: verificada por agente (rango 290–310 sin señales; barrido completo 0–11110 autorizado por el humano, 2026-10-17)

<!-- Estados: pendiente | aprobada | modificada | descartada |
     verificada por agente | revisada por humano | discrepada | cerrada -->

## Propuesta (la escribe el agente antes de editar)

### 1. Qué cambia

Antes de tocar el shim, saber qué cuadros del juego necesitan cosas que el shim todavía no puede
representar: fondos afines en BG2/BG3 y efectos que cambian registros a mitad de cuadro. La salida es
una tabla por cuadro, no una imagen.

### 2. Contexto

- Contexto consultado:
  - `src/ppu_shim.rs`: `render_frame` recibe un estado por fotograma (`vram`, `oam`, `palette`, `io`);
    no hay datos por línea. `BgLayerConfig.mosaic` lee el bit 6 de `BGnCNT` (ppu_shim.rs:416), que en
    BG2 y BG3 es el bit de **afín**; `decode_oam` descarta los sprites de ventana (ppu_shim.rs:311) y
    de los afines solo suprime los flips (ppu_shim.rs:318). No hay ninguna mención a `BLDCNT`,
    `BLDALPHA` ni a `WIN0`/`WIN1`/`OBJWIN` en `ppu_shim.rs` ni en `ppu_shim.wgsl` (búsqueda con
    `grep -n "BLDCNT\|bldalpha\|WIN0\|win0" src/ppu_shim.*`).
  - `docs/propuestas/2026-10-17_subcamino-de-composicion-con-sprite-translucido.md`: en el cuadro 1000
    `BLDALPHA` cambia por línea (`EVA` de 1 a 16 entre las líneas 133 y 157).
  - `gba-recomp/crates/gba-core/src/mem.rs`: `CapturaLinea` ya guarda `io`, `palette`, `vram`, `oam`,
    `aff_ref` y `aff_dirty` por línea, y `GbaPerfCounters` ya cuenta `lineas_con_cambio`,
    `lineas_vram_cambio` y `lineas_oam_cambio`.
  - `docs/propuestas/2026-10-16_arnes-comparacion-shim-vs-software.md`: la captura única por cuadro es
    la causa probable de las bandas; ventanas y blend no existen en el shader.
- Intención: saber qué cuadros usan capas afines y qué cuadros cambian estado dentro del cuadro, para
  priorizar el shim con datos. Éxito: una tabla con una fila por cuadro y un resumen con las listas.
- Restricciones y decisiones previas:
  - El shim no se toca en esta propuesta (respuesta humana, 2026-10-17) → implica: solo se lee el
    estado con la PPU por software → protegida por: alcance.
  - La PPU por software es la referencia (decisión previa, 2026-10-16) → implica: no se compara contra
    el shim ni contra la TV → protegida por: «El control coincide con el juego».
- Alternativas descartadas:
  - Deducirlo de las capturas del shim → descartada: los glitches que vibran no se ven en una imagen
    fija, que es justo el motivo de esta propuesta.
  - Leer `BG2CNT` a mano en un solo cuadro → descartada: no dice en qué cuadros del juego aparece el
    patrón ni cuántos son.
- Ante imprevistos, prioriza: que el barrido cubra el rango pedido sobre que el informe sea bonito.

### 3. Ejemplos

- Caso normal: cuadro 500 con BG2 afín y `BG2HOFS` distinto en 3 líneas. Antes: no había forma de
  saberlo sin abrir el shim. Ahora: una fila `cuadro 500 modo 0 afines BG2 cambios BG2HOFS` y el
  cuadro 500 aparece en la lista de afines y en la de cambios por línea.
- Caso límite: rango de un solo cuadro sin capas afines ni cambios. Ahora: una fila con `afines -` y
  `cambios -`, y listas vacías en el resumen.

### 4. Pasos

1. Crear `examples/ppu_afin_y_linea.rs` (feature `proto`) con args
   `<ruta.gba> <secuencia.txt> <desde> <hasta> [paso]`.
2. Por cada cuadro del rango: activar `capturar_linea`, correr el cuadro, apagarlo y comparar las 160
   capturas entre sí (línea 0 como referencia) para estos campos: `DISPCNT`, `BG0CNT`..`BG3CNT`,
   `BG0HOFS`..`BG3VOFS`, `BLDCNT`, `BLDALPHA`, `BLDY`, paleta, VRAM y OAM.
3. Escribir una fila por cuadro: modo de vídeo, capas activas, qué BG2/BG3 son afines (bit 6 de su
   `BGnCNT`), y la lista de campos que cambian dentro del cuadro con el número de líneas afectadas.
4. Resumen: cuadros con capas afines, cuadros con cambios por línea, y el campo que más cambia.
5. Escribir `volcados/afin-y-linea.txt` y `tests/afin_y_linea_tests.rs`.
6. No se cambia el shim, `ppu.rs`, `gba-core` ni el juego.

### 5. Qué más podría verse afectado

- `Cargo.toml`: una entrada `[[example]]` (solo se añade).
- `tests/afin_y_linea_tests.rs`: nuevo.
- Nada de `src/`: el ejemplo solo usa la API pública de `gba-core`.
- Coste: un cuadro de emulación por cada cuadro del rango, más la copia por línea que ya hace el motor
  con `capturar_linea`. Un rango de 1.000 cuadros tarda decenas de segundos, no minutos.

### 6. Suposiciones

- `CapturaLinea` conserva el estado que lee `render_scanline`, así que comparar capturas detecta lo
  que el shim no puede representar → prototipar: sí, con el control del punto 4 → si es falso: se
  añaden los campos que falten a la captura, lo que ya implica tocar `mem.rs` y pedir aprobación.
- El bit 6 de `BG2CNT`/`BG3CNT` es afín en modo 0 (documentación de la GBA: en BG2/BG3 ese bit es
  afín; en BG0/BG1 es mosaico) → prototipar: no → si es falso: el informe invertiría las etiquetas de
  «afín» y «mosaico» en esas dos capas.
- El rango de cuadros pedido es suficiente para ver los glitches de las capturas → prototipar: no →
  si es falso: se repite el barrido con otro rango, sin cambiar código.

### 7. Alcance

- Incluye: barrido por rango de cuadros, detección de capas afines, detección de cambios por línea,
  informe y pruebas.
- No incluye: cambiar el shim; renderizar capas afines; el modelo por línea; blend y ventanas;
  comparar píxeles contra el shim.

### 8. Criterios de aceptación (no se modifican sin aprobación)

Escenario: El informe cubre el rango pedido con una fila por cuadro
Caso de uso: saber qué cuadros revisar sin abrir el shim | Tipo: normal
  Dado el rango 290 a 310 con paso 1
  Cuando se corre el barrido
  Entonces `volcados/afin-y-linea.txt` tiene 21 filas `cuadro `, de la 290 a la 310, cada una con
  `modo`, `afines` y `cambios`
Test: `tests/afin_y_linea_tests.rs::test_una_fila_por_cuadro` (se omite si falta `FE_ROM`)
Estado previo: falla (el informe no existe)

Escenario: Las listas del resumen coinciden con las filas
Caso de uso: no perseguir un cuadro por una etiqueta equivocada | Tipo: normal
  Dado el informe del barrido
  Cuando se leen las listas `afines` y `con cambios por linea`
  Entonces cada cuadro listado tiene su fila con la etiqueta correspondiente, y ninguno de más
Test: `tests/afin_y_linea_tests.rs::test_resumen_coincide_con_filas` (se omite si falta `FE_ROM`)
Estado previo: falla (el informe no existe)

Escenario: El control coincide con el juego
Caso de uso: saber que el barrido no altera lo que mide | Tipo: límite
  Dado el último cuadro del rango
  Cuando se compara la variante natural dibujada fuera del juego con el framebuffer del juego
  Entonces la diferencia es 0 píxeles de 38.400
Test: `tests/afin_y_linea_tests.rs::test_control_coincide` (se omite si falta `FE_ROM`)
Estado previo: falla (el informe no existe)

Escenario: Un cuadro sin capas afines ni cambios no inventa señales
Caso de uso: que el informe no marque como sospechoso todo | Tipo: límite
  Dado un cuadro del rango con `afines -` y `cambios -`
  Cuando se lee su fila
  Entonces sus listas del resumen no lo incluyen
Test: `tests/afin_y_linea_tests.rs::test_cuadro_sin_senales` (se omite si falta `FE_ROM`)
Estado previo: falla (el informe no existe)

### 9. Cómo lo comprobaré

- Una fila por cuadro → comando:
  `cargo run --release --features proto --example ppu_afin_y_linea -- "roms/1575 - Fire Emblem (E)(Independent).gba" secuencias/intro.txt 290 310`
  → pasa si: 21 filas, de 290 a 310 → no detecta: cuadros fuera del rango.
- Resumen coincide con filas → `cargo test --features proto --test afin_y_linea_tests` → pasa si:
  las dos listas cuadran con las filas → no detecta: que el campo elegido sea el que causa el glitch.
- Control → el mismo ejemplo, línea `control` del resumen → pasa si: 0 píxeles → no detecta: errores
  que también comete la PPU por software.
- Sin señales → el mismo test → pasa si: el cuadro no aparece en ninguna lista → no detecta: un cambio
  que no esté entre los campos medidos.
- Regresión → `cargo test` → pasa si: la suite del port sigue en verde → no detecta: el aspecto en
  pantalla.

### 10. Terminado cuando

Se cumplen los cuatro criterios y el Resultado dice qué cuadros del rango usan capas afines, cuáles
cambian registros dentro del cuadro y qué campo es el más frecuente.

### 11. Parar si

- `CapturaLinea` no basta para ver los cambios (primera suposición falsa): parar y presentar qué campo
  falta antes de tocar `mem.rs`.
- El rango pedido no contiene los cuadros de las capturas: parar y pedir el rango correcto; no ampliar
  el barrido por cuenta propia.
- Dos intentos fallidos seguidos con el mismo criterio: parar y marcar la propuesta como `discrepada`.

### Correcciones previas a presentar

- La propuesta no compara contra el shim porque el shim no tiene todavía un modelo por línea: la
  comparación de píxeles se haría contra una imagen que ya se sabe incompleta. El valor está en saber
  qué cuadros exigen lo que falta.

### Cambios tras prototipos (solo si el plan cambió)

Ninguno: el plan se ejecutó tal cual.

---

## Resultado (lo completa el agente después de editar)

Ejecutado el 2026-10-17 con `examples/ppu_afin_y_linea`, rango 290–310 de `secuencias/intro.txt`.
`tests/afin_y_linea_tests.rs`: **5 pruebas pasan** con `FE_ROM` (las 4 de los criterios y una de
blindaje). Mutación sobre el informe (quitar la fila 300 y añadirla a `afines`):
`test_una_fila_por_cuadro` y `test_resumen_coincide_con_filas` fallan, y el archivo se restauró
desde memoria.

| Medida | Valor |
| :--- | ---: |
| Cuadros barridos | 21 (290 a 310) |
| Cuadros con capas afines | 0 |
| Cuadros con cambios dentro del cuadro | 0 (ni E/S, ni paleta, ni VRAM, ni OAM) |
| Campo más frecuente | ninguno |
| Control (redibujo fuera del juego vs framebuffer) | 0 píxeles de 38.400 |

Predicho vs. real:

- «Una fila por cuadro»: **coincide** (21 filas, de la 290 a la 310, en orden).
- «El resumen coincide con las filas»: **coincide** (ambas listas vacías y coherentes).
- «El control coincide con el juego»: **coincide** (0 de 38.400).
- «Un cuadro sin señales no inventa señales»: **coincide** (los 21 cumplen y ninguno aparece en las
  listas).
- Lo que se buscaba —capas afines y cambios por línea—: **no se pudo comprobar** en este rango,
  porque no hay ninguno.

Lectura:

- En 290–310 el juego no usa capas afines ni cambia registros a mitad de cuadro: los 21 cuadros dan
  `afines -` y `cambios -`, con VRAM, OAM y paleta idénticas en las 160 líneas.
- Es el resultado esperado de una pantalla estática de la introducción, y **no responde** a la
  pregunta de los glitches: las tres capturas (menú, diálogo y previsión de combate) están en otro
  punto de la partida.
- Se cumple la condición de parada de la sección 11: el rango pedido no contiene esos cuadros. Se
  paró y se pidió el rango correcto; la respuesta humana («barre el intro completo igual», 2026-10-17)
  autorizó el barrido extra que sigue.

### Barrido completo de la introducción (0 a 11110, paso 10)

Autorizado por el humano tras la parada de la sección 11. 1.112 cuadros, un cuadro de emulación por
cuadro con captura por línea. Informe en `volcados/barrido-intro.txt` (fuera de alcance versionarlo:
son 1.112 filas generadas).

| Medida | Valor |
| :--- | ---: |
| Cuadros barridos | 1.112 |
| Cuadros con capas afines | **0** |
| Cuadros con algún cambio de E/S dentro del cuadro | **900 (81 %)** |
| `BLDALPHA` | 534 cuadros, hasta 157 líneas |
| `BLDCNT` | 528 cuadros, hasta 60 líneas |
| `BG0CNT` y `BG2CNT` | 269 cuadros cada uno, hasta 109 líneas |
| `BG2VOFS` / `BG2HOFS` | 165 / 156 cuadros, hasta 131 / 106 líneas |
| `BG0VOFS` / `BG0HOFS` | 114 / 98 cuadros, hasta 159 / 151 líneas |
| `BG1VOFS` | 26 cuadros |
| Cuadros con VRAM distinta entre líneas | 185 |
| Cuadros con OAM o paleta distintas entre líneas | 0 |

Lectura:

- **Las capas afines quedan descartadas como explicación**, al menos en la introducción: cero cuadros
  en 1.112. La hipótesis afín del shim no puede causar lo que se ve en estas pantallas.
- **El estado dentro del cuadro sí es la norma**: 900 de 1.112 cuadros escriben algún registro a
  mitad de cuadro. En 269 cuadros `BG0CNT` y `BG2CNT` se reescriben en las mismas líneas, con hasta
  109 líneas dibujadas con un `BGnCNT` distinto del de la línea 0; en 165 y 156 cuadros lo hacen
  `BG2VOFS` y `BG2HOFS` (hasta 131 y 106 líneas). Cualquiera de esos cuadros es irreproducible con un
  único estado por fotograma, que es lo que el shim aplica a las 160 líneas.
- `BLDALPHA`/`BLDCNT` cambian dentro del cuadro en la mitad de los cuadros (hasta 157 y 60 líneas
  respectivamente), coherente con los fundidos que ya aparecían en el cuadro 1000.
- OAM y paleta nunca cambian a mitad de cuadro: no son sospechosos de las bandas de sprites.

Por qué esto no convierte al shim en el culpable demostrado, y sí en el sospechoso principal: el
barrido mide lo que el juego pide, no lo que el shim dibuja. Dice que 81 % de los cuadros de la
introducción exigen un modelo que el shim no tiene; demostrar que de ahí salen los glitches necesita
el arnés de comparación por línea (`2026-10-16_arnes-comparacion-shim-vs-software.md`), que sigue
pendiente.

Archivos: `examples/ppu_afin_y_linea.rs` y `tests/afin_y_linea_tests.rs` (nuevos), `Cargo.toml` (una
entrada `[[example]]`), `volcados/afin-y-linea.txt`. El shim, `ppu.rs`, `gba-core` y el juego no
cambian.

### Fuera de alcance (hallazgos que no se arreglaron)

- `src/ppu_shim.rs` sigue tomando un único estado por fotograma: con 81 % de los cuadros de la
  introducción escribiendo E/S a mitad de cuadro, ninguna mejora del shim puede reproducirlos hasta
  que reciba estado por línea. — impacto: medio
- `BgLayerConfig.mosaic` sigue leyendo el bit 6 de `BGnCNT` como mosaico en BG2/BG3, donde ese bit es
  afín: hoy es inocuo porque el juego no usa BG2/BG3 afines en la introducción (0 de 1.112 cuadros),
  pero etiqueta mal el campo. — impacto: bajo
- El informe guarda cuántas líneas difieren de la línea 0, no cuáles: una sola escritura en mitad del
  cuadro y una tabla por línea de barrido producen el mismo número, así que el barrido no distingue
  ambos casos. — impacto: bajo

---

## Revisión humana

- Ninguno: todo se comprueba con tests de los informes.
- Detectado solo por el humano:
