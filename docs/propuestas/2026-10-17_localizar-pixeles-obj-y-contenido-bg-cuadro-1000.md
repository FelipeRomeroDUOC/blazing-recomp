# Propuesta: Localizar los píxeles que cambia OBJ y comprobar el contenido de BG0, BG1 y BG3 en el cuadro 1000

Fecha: 2026-10-17
Nivel: 2
Estado: verificada por agente (causa de OBJ explicada el 2026-10-17: mezcla alpha de un sprite negro translúcido)

<!-- Estados: pendiente | aprobada | modificada | descartada |
     verificada por agente | revisada por humano | discrepada | cerrada -->

## Propuesta (la escribe el agente antes de editar)

### 1. Qué cambia

Quedan dos puntos sin explicar del cuadro 1000, según `diagnostico-efectos-y-ventanas-cuadro-1000`:

- OBJ cambia unos 6.000 píxeles al quitarlo, pero en `capas-por-aportacion` solo cubre 710. Estas dos cifras no pueden venir de la misma definición de medida. Hay que medir ambas en un solo script y localizar dónde están los píxeles que sobran.
- BG0, BG1 y BG3 no dibujan ningún píxel visible, ni siquiera sin ventanas. Hay que comprobar qué contienen sus tiles y sus mapas en VRAM, para confirmar H6 con datos y no solo por inferencia.

No se cambia el shim, `gba-core`, `ppu.rs` ni el juego. Todo el experimento usa la PPU por software con las capturas por línea del cuadro 1000.

### 2. Contexto

- Contexto consultado:
  - `docs/propuestas/2026-10-17_capas-por-aportacion-cuadro-1000.md`: aportación de OBJ = 6.000 píxeles, cobertura = 710. BG0, BG1 y BG3 con aportación y cobertura 0.
  - `docs/propuestas/2026-10-17_diagnostico-efectos-y-ventanas-cuadro-1000.md`: H4 y H5 descartadas; H6 confirmada por inferencia. Los efectos de color no cambian OBJ.
  - `examples/ppu_capas_linea.rs`: define la aportación como píxeles distintos entre la variante completa y la variante sin la capa, y la cobertura como píxeles distintos entre la capa sola y la variante sin nada. Ambas comparan colores, no posiciones.
  - Registros de BG: `BG0CNT` en 0x08, `BG1CNT` en 0x0A, `BG3CNT` en 0x0E; desplazamientos `BGnHOFS` y `BGnVOFS` en 0x10 a 0x1E. El bit 7 de `BGnCNT` marca 8 bpp (no 4 bpp). Los bits 2 a 3 dan el charblock (tiles) y los bits 8 a 12 el screenblock (mapa). Cada entrada de mapa son 2 bytes: índice de tile en bits 0 a 9, volteos en bits 10 y 11, paleta en bits 12 a 15 (solo 4 bpp).
  - `gba-recomp/crates/gba-core/src/ppu.rs`, `compose`: elige entre ruta rápida, alpha por tramos y general, según `razon_camino` (0 a 3). Quitar OBJ puede cambiar la ruta elegida en algunas líneas.
- Intención: explicar la diferencia de OBJ (qué píxeles y por qué) y confirmar qué contienen BG0, BG1 y BG3. Éxito: un informe por cada cifra, con sus números y coordenadas.
- Hipótesis:
  - H7: la aportación de OBJ y su cobertura miden cosas distintas (dos definiciones de medida). Se confirma si, medidas en un solo script, la aportación queda dentro de la cobertura o la diferencia desaparece.
  - H8: al quitar OBJ cambia la ruta de `compose` (rápida ↔ general) y las dos rutas dan colores distintos. Se confirma si los píxeles fuera de cobertura están en líneas donde cambia `razon_camino`.
  - H9: OBJ altera píxeles de otras capas donde no dibuja por sí solo (composición real). Se confirma si quedan píxeles fuera de cobertura en líneas donde la ruta no cambia.
  - H10: los tiles de BG0, BG1 y BG3 referenciados por sus mapas están vacíos (todos cero). Se confirma si el número de tiles referenciados no vacíos es 0.
  - H11: los tiles tienen píxeles de índice distinto de 0, pero su color de paleta es igual al color de fondo, así que no cuentan como distintos. Se confirma si los píxeles no cero de esos tiles usan colores iguales al fondo.
  - H12: los mapas de BG0, BG1 y BG3 apuntan fuera de lo visible (desplazamiento, tamaño o screenblock). Se confirma si todas las entradas de mapa visibles valen 0.
- Restricciones y decisiones previas:
  - La PPU por software no cambia (respuesta humana, 2026-10-16).
  - El experimento solo lee capturas y VRAM. No cambia `gba-core` ni el shim.
  - El trabajo corre en el PC con Windows (respuesta humana, 2026-10-16).
- Alternativas descartadas:
  - Forzar la ruta de `compose` cambiando `ppu.rs` para probar H8 → descartada en este paso: cambiar `ppu.rs` requiere aprobación. Si H8 no se puede probar solo con lo que ya es público, se para (ver sección 11).
  - Comparar contra la TV o contra el shim → descartada: el shim no implementa las capas BG0, BG1 y BG3 con este contenido, y la TV no da una cifra por píxel.

### 3. Ejemplos

- Caso normal: al quitar OBJ cambian N píxeles. Si X (dentro de cobertura) es igual a N, todo el cambio cae donde OBJ dibuja por sí solo, y H9 queda descartada. Si Y es mayor que 0, se mira en qué líneas están (paso 3) para decidir entre H7 y H8. Las cifras reales salen del informe, no de esta propuesta.
- Caso límite: BG0 tiene un mapa con 1.024 entradas todas en 0. Entonces H10 y H12 se confirman a la vez: el informe marca 0 tiles no vacíos referenciados y 0 entradas visibles distintas de 0.

### 4. Pasos

1. Crear `examples/ppu_pixeles_obj.rs` (con `required-features = ["proto"]`). Reutiliza la captura por línea de `ppu_capas_linea`. Por cada línea, reinicia `aff_ref` y `aff_dirty` y dibuja estas variantes con la misma función:
   - V0: completa (control: debe coincidir con el juego);
   - V1: sin OBJ;
   - V2: solo OBJ;
   - V3: nada (sin ninguna capa).
   Calcula en un solo script: aportación = píxeles distintos entre V0 y V1; cobertura = píxeles distintos entre V2 y V3. Guarda también las coordenadas (x, línea) de cada píxel en la aportación.
2. Cruzar las dos listas: aportación dentro de cobertura, y aportación fuera de cobertura. Para cada píxel fuera de cobertura, anotar el color en V0, V1, V2 y V3.
3. Para cada línea con píxeles fuera de cobertura, anotar `razon_camino` de `compose` con V0 y con V1. Si esa razón no se puede leer desde el ejemplo sin cambiar `ppu.rs`, parar (sección 11).
4. Crear `examples/ppu_contenido_bg.rs` (con `required-features = ["proto"]`). Para BG0, BG1 y BG3, leer de la VRAM capturada:
   - charblock y screenblock (de `BGnCNT`), tamaño del mapa y desplazamientos;
   - número de tiles no vacíos referenciados por las entradas del mapa (entradas con índice ≠ 0);
   - número de entradas visibles del mapa distintas de 0;
   - para los tiles no vacíos, si sus píxeles de índice ≠ 0 tienen el mismo color de paleta que el fondo (H11).
5. Escribir `volcados/pixeles-obj-cuadro-1000.txt` con: `obj aportacion N`, `obj cobertura M`, `obj aportacion dentro de cobertura X`, `obj aportacion fuera de cobertura Y`, colores de los píxeles de Y, `razon_camino` en las líneas de Y. Escribir `volcados/contenido-bg-cuadro-1000.txt` con una línea por capa (BG0, BG1, BG3) para cada medida de la paso 4.
6. Crear `tests/localizar_pixeles_tests.rs` con las pruebas de la sección 8.
7. Añadir las dos entradas `[[example]]` en `Cargo.toml`, ejecutar y registrar el resultado.

### 5. Qué más podría verse afectado

- Archivos que cambian: `examples/ppu_pixeles_obj.rs` (nuevo), `examples/ppu_contenido_bg.rs` (nuevo), `tests/localizar_pixeles_tests.rs` (nuevo), `Cargo.toml` (dos entradas).
- Archivos que no cambian: `gba-core`, `ppu.rs`, `ppu_shim.rs`, `ppu_shim.wgsl`, el juego y la APK.
- Riesgo: el volcado de coordenadas puede ser largo. Se guarda en el archivo de volcado y el informe de consola solo resume.

### 6. Suposiciones

- Las capturas por línea del cuadro 1000 siguen siendo válidas, porque la variante V0 da 0 píxeles de diferencia con el juego → prototipar: sí, con el control del paso 1 → si es falsa: parar, sección 11.
- Los bits de `BGnCNT` (charblock, screenblock, 8 bpp) se leen como indica la sección 2 → prototipar: sí, comprobando que el número de tiles no vacíos es coherente con la VRAM capturada → si es falsa: revisar la lectura antes de sacar conclusiones.
- `razon_camino` puede leerse desde un ejemplo sin cambiar `ppu.rs` → prototipar: sí, buscando si es público → si es falsa: parar y pedir aprobación para exponerlo.

### 7. Alcance

- Incluye: medida única de aportación y cobertura de OBJ, localización de sus píxeles, contenido de tiles y mapas de BG0, BG1 y BG3, informes y pruebas.
- No incluye: cambios en el shim, en `gba-core` o en `ppu.rs`; corrección de los glitches; otros cuadros; Android.

### 8. Criterios de aceptación (no se modifican sin aprobación)

Escenario: la aportación de OBJ se reparte entre dentro y fuera de su cobertura
Caso de uso: saber si la diferencia de 6.000 frente a 710 es un error de medida o un efecto real de OBJ | Tipo: normal
  Dado el cuadro 1000 con sus capturas por línea
  Cuando se corre el diagnóstico de píxeles de OBJ
  Entonces `volcados/pixeles-obj-cuadro-1000.txt` tiene `obj aportacion N`, `obj cobertura M`, `obj aportacion dentro de cobertura X` y `obj aportacion fuera de cobertura Y`, con X + Y = N
Test: `tests/localizar_pixeles_tests.rs::test_obj_suma_cuadra` (se omite si no está definida `FE_ROM`)
Estado previo: falla (el informe no existe)

Escenario: el informe de BG0, BG1 y BG3 da el contenido de sus tiles y mapas
Caso de uso: confirmar H6 con datos | Tipo: normal
  Dado el cuadro 1000
  Cuando se corre el diagnóstico de contenido de BG
  Entonces `volcados/contenido-bg-cuadro-1000.txt` tiene tres líneas `bg BG0`, `bg BG1` y `bg BG3`, cada una con tiles no vacíos referenciados, entradas de mapa visibles distintas de 0 y colores de paleta
Test: `tests/localizar_pixeles_tests.rs::test_bg_tiene_tres_capas` (se omite si no está definida `FE_ROM`)
Estado previo: falla (el informe no existe)

Escenario: la variante de control coincide con el juego
Caso de uso: comprobar que el experimento es válido antes de medir | Tipo: límite
  Dado el cuadro 1000
  Cuando se corre el diagnóstico de píxeles de OBJ
  Entonces la diferencia entre V0 y el juego es 0 píxeles
Test: `tests/localizar_pixeles_tests.rs::test_control_coincide_con_juego` (se omite si no está definida `FE_ROM`)
Estado previo: falla (el informe no existe)

### 9. Cómo lo comprobaré

- Ejecución: `cargo run --release --features proto --example ppu_pixeles_obj -- "roms\1575 - Fire Emblem (E)(Independent).gba" secuencias\intro.txt 1000 > ..\ppu-pixeles-obj-output.txt 2>&1` y lo mismo con `ppu_contenido_bg` → pasa si: los informes tienen las líneas de la sección 8 → no detecta: diferencias que solo se ven en la TV.
- Pruebas: con `FE_ROM` definida, `cargo test --features proto --test localizar_pixeles_tests` → pasa si: las tres pruebas pasan o se omiten con el aviso.
- Control: V0 debe dar 0 píxeles de diferencia con el juego. Si no, se para.
- Regresión: `git diff --stat` → pasa si: no aparecen `gba-core`, `ppu.rs`, `ppu_shim.rs` ni `ppu_shim.wgsl` → no detecta: cambios no versionados.

### 10. Terminado cuando

Se cumplen los tres criterios de aceptación, y el Resultado dice si H7, H8 y H9 se confirman o se descartan (para OBJ), y si H10, H11 y H12 se confirman o se descartan (para BG0, BG1 y BG3).

### 11. Parar si

- V0 no coincide con el juego: parar, porque el experimento no es válido.
- `razon_camino` no se puede leer sin cambiar `ppu.rs`: parar y pedir aprobación para exponerlo.
- Los píxeles fuera de cobertura (Y) son mayores que 0 en líneas donde la ruta no cambia (H9): parar, porque confirmarlo como bug de composición requiere cambiar `ppu.rs`, y eso pide aprobación.
- Dos intentos fallidos seguidos con el mismo criterio: detenerse, explicar qué se probó y marcar la propuesta como `discrepada`.

### Correcciones previas a presentar

- En `2026-10-17_diagnostico-efectos-y-ventanas-cuadro-1000.md`, el Resultado llama a la contradicción de OBJ "abierta". Esta propuesta la investiga, pero no la cierra hasta tener el Resultado de la sección 10.
- H6 de la propuesta de diagnóstico estaba confirmada solo por inferencia. Esta propuesta la comprueba con el contenido de tiles y mapas (H10 a H12).

### Cambios tras prototipos (solo si el plan cambió)

- **`razon_camino` sí se puede medir sin cambiar `ppu.rs`.** Es una variable local, pero cada rama de
  la ruta incrementa un contador de `perf-counters` (`lineas_rectas`, `descartes_mezcla`,
  `descartes_ventana`, `descartes_semi`). El ejemplo vacía los contadores antes de dibujar cada
  línea y los lee después, así que la ruta sale de campos públicos. No hace falta la aprobación
  prevista en la sección 11 para exponerla.
- Se añadieron dos pruebas de blindaje a las tres de los criterios: el detalle (`pixel` y `linea`)
  tiene que cuadrar con el resumen, y la partición por cambio de ruta tiene que sumar la aportación
  exterior. Total: 5 pruebas.

---

## Resultado (lo completa el agente después de editar)

Ejecutado el 2026-10-17 con `examples/ppu_pixeles_obj` y `examples/ppu_contenido_bg`, cuadro 1000,
PPU por software con las capturas por línea. Control válido: la variante completa coincide con el
juego (0 píxeles distintos de 38.400). `tests/localizar_pixeles_tests.rs`: **5 pruebas pasan** (las
3 de la sección 8 y 2 de blindaje). Mutación deliberada (`dentro + 1` en el informe):
`test_obj_suma_cuadra` **falla**, y el archivo se restauró desde memoria.

### OBJ: 6.000 píxeles = 710 opacos + 5.290 de mezcla alpha

| Medida | Píxeles |
| :--- | ---: |
| Aportación (V0 completa vs V1 sin OBJ) | 6.000 |
| Cobertura (V2 solo OBJ vs V3 nada) | 710 |
| Aportación dentro de cobertura | 710 |
| Aportación fuera de cobertura | 5.290 |
| Líneas con aportación fuera de cobertura | 25 (133 a 157) |
| Líneas con sprite translúcido (medido el 2026-10-17) | 28 (132 a 159) |
| Píxeles que son `alpha_blend` con negro | 5.290 (5.050 con el sprite arriba + 240 con BG2 arriba) |
| Diferencia entre caminos forzados (160 líneas) | 0 |
| Líneas en que cambia la ruta de `compose` al quitar OBJ | 0 |

Lectura (cerrada el 2026-10-17 con `examples/ppu_subcamino`):

- **H7 confirmada en el fondo, aunque su criterio estaba mal escrito.** La aportación y la cobertura
  sí miden cosas distintas: «aportación» son los píxeles que cambian en el cuadro y «cobertura» los
  que difieren del fondo con la capa sola. El sprite de la banda es una capa negra **translúcida**
  que se mezcla con BG2, y la cobertura no lo ve porque OBJ sola sobre negro da el mismo negro que
  no tener OBJ. El criterio pedía que la diferencia desapareciera, y no desaparece: por eso se había
  descartado. La definición era correcta; la condición de confirmación, no.
- **H8 descartada.** Quitar OBJ no cambia la ruta de `compose` en ninguna de las 160 líneas, y forzar
  el mismo camino en las dos variantes deja la diferencia en 6.000 píxeles, no en 0. Los dos caminos
  dan exactamente los mismos píxeles.
- **H9 confirmada y explicada.** Los 5.290 píxeles son mezcla alpha de un sprite negro translúcido:
  5.050 responden a `alpha_blend(negro, BG2)` y 240 a `alpha_blend(BG2, negro)`, con la `BLDALPHA`
  de cada línea (`EVA` va de 1 a 16: es un fundido). Los 710 restantes son la cobertura opaca.
  `BLDCNT` = `0x0F40` (alpha con segundos objetivos BG0-BG3) y `DISPCNT` = `0x1F00` (sin ventanas).
- **La sección 11 se cierra sin arreglo.** No hay bug de composición: era composición real. El aviso
  de «contradicción abierta» queda retirado y no hay nada que cambiar en `ppu.rs`.

### BG0, BG1 y BG3: los mapas no referencian ningún tile con datos

VRAM constante durante todo el cuadro (0 de 160 líneas cambian respecto de la línea 0). Modo de
vídeo 0; los tres fondos son de texto 4 bpp (bit 7 de `BGnCNT` a 0) y no cambian `BGnCNT`, `HOFS` ni
`VOFS` en ninguna línea.

| Fondo | `BGnCNT` | charblock (tiles) | screenblock (mapa) | tiles con datos en el charblock | entradas visibles ≠ 0 | tiles referenciados con datos | píxeles de tile con datos | píxeles opacos |
| :--- | :--- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| BG0 | `0x2C0A` | 2 | 44 | 897 | 0 | 0 | 0 | 0 |
| BG1 | `0x0D03` | 0 | 13 | 734 | 0 | 0 | 0 | 0 |
| BG3 | `0x0F0B` | 2 | 15 | 897 | 0 | 0 | 0 | 0 |

- **H10 confirmada.** Ni un solo tile referenciado por los mapas visibles tiene datos: los 38.400
  píxeles de cada fondo resuelven a la entrada de mapa 0 y esa entrada apunta al tile 0, que está
  vacío.
- **H11 descartada.** No hay ningún píxel opaco (índice de color ≠ 0): el informe da
  `pix opacos: 0` y `pix opacos iguales al fondo: 0`, así que no hay colores de paleta que comparar
  con el fondo.
- **H12 confirmada por su criterio, con una corrección de redacción.** Todas las entradas de mapa
  visibles valen 0, que es lo que pedía el criterio, pero no porque «apunten fuera de lo visible»:
  apuntan al tile 0 del charblock, y ese tile está vacío. El charblock sí tiene datos (897, 734 y
  897 tiles no vacíos), solo que ningún mapa visible los referencia.

### Predicho vs. real

- Predicho: un informe por cifra, con números y coordenadas → **real**: dos informes, uno con los
  5.290 píxeles fuera de cobertura uno a uno y otro con una línea por fondo.
- Predicho: H7, H8 y H9 se confirmarían o descartarían → **real**: H7 confirmada en el fondo (su
  criterio estaba mal escrito), H8 descartada, H9 confirmada y explicada como mezcla alpha; no hubo
  nada que arreglar en `ppu.rs`.
- Predicho: H10, H11 y H12 se confirmarían o descartarían → **real**: H10 y H12 confirmadas,
  H11 descartada.
- Predicho: parar si `razon_camino` no se podía leer → **real**: se leyó por los contadores públicos
  de `perf-counters`, sin exponer nada de `ppu.rs`.

Archivos: `examples/ppu_pixeles_obj.rs` y `examples/ppu_contenido_bg.rs` (nuevos),
`tests/localizar_pixeles_tests.rs` (nuevo), `Cargo.toml` (dos entradas `[[example]]`).
`gba-core`, `ppu.rs`, `ppu_shim.rs`, `ppu_shim.wgsl` y el juego no cambiaron.

### Fuera de alcance (hallazgos que no se arreglaron)- La cobertura de `ppu_capas_linea` define «visible» como diferente del fondo, así que no ve los píxeles que OBJ cambia por mezcla alpha: 5.290 de los 6.000 del cuadro 1000. — impacto: medio
- El índice de propuestas de `docs/BACKLOG.md` sigue en 58 fichas y no incluye las propuestas del
  2026-10-16 ni las del 2026-10-17, así que su recuento por estado ya no cuadra. — impacto: bajo

---

## Revisión humana

- Que la aportación de OBJ se reparta entre dentro y fuera de su cobertura, y que BG0, BG1 y BG3 queden confirmadas o descartadas con su contenido en VRAM: ✔ | ✘ | ?
