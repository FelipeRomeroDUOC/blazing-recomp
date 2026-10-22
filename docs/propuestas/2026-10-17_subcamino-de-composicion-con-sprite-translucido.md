# Propuesta: Que la imagen no dependa de qué camino interno compone una línea con sprite translúcido

Fecha: 2026-10-17
Nivel: 2
Estado: discrepada (la medida contradijo la premisa el 2026-10-17; no hay nada que arreglar en la ruta)

<!-- Estados: pendiente | aprobada | modificada | descartada |
     verificada por agente | revisada por humano | discrepada | cerrada -->

## Propuesta (la escribe el agente antes de editar)

### 1. Qué cambia

En el cuadro 1000, una banda de 25 líneas se oscurece solo por tener un sprite translúcido en la
línea, aunque el sprite no dibuje en la mayoría de esos píxeles. Quitando el bit de OBJ de `DISPCNT`
la banda se aclara. Lo que cambia es el camino interno que compone esa línea, y los dos caminos no
dan el mismo píxel: después de este cambio, una línea se verá igual elija el camino que elija el
motor.

### 2. Contexto

- Contexto consultado:
  - `docs/propuestas/2026-10-17_localizar-pixeles-obj-y-contenido-bg-cuadro-1000.md`: aportación de
    OBJ 6.000, cobertura 710, 5.290 píxeles fuera de cobertura en las líneas 133 a 157, ruta de
    `compose` igual (1) con y sin OBJ.
  - `docs/propuestas/2026-10-17_diagnostico-efectos-y-ventanas-cuadro-1000.md`: con `BLDCNT` = 0 la
    diferencia de OBJ sube a 6.240 píxeles, no desaparece.
  - `gba-recomp/crates/gba-core/src/ppu.rs`: `render_scanline` solo llama a `render_sprites` si
    `DISPCNT` tiene el bit 12 (línea 147); `render_sprites` devuelve si la línea tiene algún píxel
    `semi` (modo 1 del sprite); `alpha_por_tramos = razon_camino == 1 && !obj_semi` (línea 1174).
    Es decir, el bit de OBJ decide entre `compose_alpha_tramos` y `compose_straight::<true>` sin
    cambiar `razon_camino`.
  - `gba-recomp/crates/gba-core/src/ppu.rs`: `componer_tramo` resuelve la pareja (capa de arriba,
    segunda capa) una vez por tramo y la aplica al tramo entero; el comentario que la precede la
    presenta como una versión optimizada del camino general, «que es la referencia de fidelidad».
  - `gba-recomp/crates/gba-core/src/ppu.rs`: ya existe un interruptor de diagnóstico
    `via_rapida_apagada()`, `#[cfg(test)]` (línea 608), que compila a `false` fuera de los tests.
- Intención: que el camino de composición sea invisible para quien juega. Éxito: los dos caminos dan
  los mismos 38.400 píxeles en el cuadro 1000, y quitar OBJ solo cambia píxeles donde OBJ dibuja o
  donde OBJ es una capa de mezcla visible.
- Restricciones y decisiones previas:
  - El arreglo se hace en el motor local `../gba-recomp` y se valida desde este repositorio a través
    del `[patch]` ya declarado en `Cargo.toml`; subir el cambio al fork y actualizar el `rev` pineado
    es un paso aparte (respuesta humana, 2026-10-17) → implica: no se toca el `rev` en esta propuesta
    → protegida por: «El port compila y pasa su suite con el motor local».
  - Decidir con medición si se conserva la vía por tramos (respuesta humana, 2026-10-17) → implica:
    medir el coste de ambos caminos antes de elegir el arreglo, con la regla de la sección 4 → protegida
    por: «El informe declara coste y decisión».
  - El shim y el juego no se tocan; `ppu_shim.rs` no implementa efectos ni sprites translúcidos
    (origen: `docs/propuestas/2026-10-17_diagnostico-efectos-y-ventanas-cuadro-1000.md`) → implica:
    la comparación es contra la PPU por software → protegida por: alcance.
- Alternativas descartadas:
  - Quitar el bit 12 en las variantes y comparar solo eso → descartada: no distingue «OBJ dibuja» de
    «OBJ cambia el camino», que es justo lo que hay que separar.
  - Desactivar `alpha_por_tramos` para todo el juego → descartada: cuesta rendimiento en 135 líneas
    donde hoy no hay discrepancia, y la decisión acordada es medir antes.
- Ante imprevistos, prioriza: que los dos caminos coincidan sobre conservar el camino por tramos; y
  conservar el camino por tramos sobre ahorrar una propuesta.

### 3. Ejemplos

- Caso normal: línea 140 del cuadro 1000, con un sprite translúcido en x=10 y BG2 ocupando el resto.
  Antes: 240 píxeles distintos entre «con OBJ» y «sin OBJ», 230 de ellos donde OBJ no dibuja.
  Ahora: 10 píxeles distintos (solo donde el sprite dibuja), y la banda de 25 líneas suma 710 en vez
  de 6.000.
- Caso límite: línea sin ningún sprite translúcido. Antes y ahora: idéntica byte a byte, y el camino
  elegido sigue siendo `compose_alpha_tramos`.

### 4. Pasos

1. En `gba-core`, añadir dos contadores a `GbaPerfCounters` (feature `perf-counters`, ya activa):
   `lineas_alpha_tramos` y `lineas_rectas_efecto`, incrementados donde hoy se decide el subcamino.
   No cambian ningún píxel.
2. Añadir un interruptor de diagnóstico `alpha_tramos_apagado()` / `forzar_alpha_tramos()`, con el
   mismo patrón que `via_rapida_apagada()`: `#[cfg(feature = "perf-video")]` con un atómico, y
   `false` en cualquier build de producción. Permite pedir el camino general en todas las líneas.
3. Crear `examples/ppu_subcamino.rs` (feature `proto`). Por cada línea del cuadro 1000 dibuja cuatro
   variantes con la captura por línea: OBJ tal cual, sin OBJ, OBJ con el camino general forzado y sin
   OBJ con el camino general forzado. Escribe `volcados/subcamino-cuadro-1000.txt` con una fila por
   línea (`linea L semi_con_obj semi_sin_obj cambio píxeles_distintos`) y un resumen con la aportación
   de la banda en cada combinación.
4. Medir coste en la misma corrida: por cada una de las 25 líneas, dibujar 200 veces con cada camino y
   anotar la mediana de `perf_counters.ppu_ns`. Decidir con esta regla, fijada antes de medir: si la
   mediana del camino general es **≤ 1,25×** la de la vía por tramos, el arreglo puede forzar el
   camino general en las líneas con sprite translúcido; si es mayor, el arreglo debe quedarse dentro
   de la vía por tramos.
5. Corregir en `compose_alpha_tramos` / `componer_tramo` o, si la medición lo permite, en la elección
   del camino. El arreglo no está decidido de antemano: lo decide el paso 4 más la comparación de
   píxeles del paso 3.
6. Añadir el invariante que hoy se rompe: quitar OBJ solo puede cambiar píxeles donde OBJ dibuja o
   donde OBJ es una capa de mezcla visible.
7. Escribir `tests/subcamino_tests.rs`, correr la suite y registrar el resultado.

### 5. Qué más podría verse afectado

- `gba-recomp/crates/gba-core/src/ppu.rs`: `compose`, `compose_alpha_tramos`, `componer_tramo`,
  `clave_de_tramo`, `alpha_por_tramos`. Es el único sitio donde se decide el camino.
- `gba-recomp/crates/gba-core/src/mem.rs`: el struct `GbaPerfCounters` es público y lo leen los
  ejemplos del port; añadir campos no rompe a nadie (ningún consumidor lo construye a mano).
- Cuadros 298, 303 y 500: si alguna de sus líneas usa el camino que cambia, su imagen cambia. Se
  registra en el informe y se compara contra los `.rgb` guardados en `volcados/`.
- Rendimiento: `compose_ns` de las líneas con sprite translúcido. Se mide en el paso 4.

### 6. Suposiciones

- El bit 12 cambia el subcamino porque `obj_semi` pasa de `true` a `false` en las 25 líneas de la
  banda → prototipar: sí, es el paso 3 con los contadores nuevos → si es falso: la hipótesis cae y
  hay que buscar otra causa (por ejemplo, un sprite que sí cubre la línea entera con color casi
  igual al fondo) → no detecta: líneas donde `obj_semi` cambie por otra razón.
- Los dos caminos dan colores distintos en esas líneas y la diferencia desaparece al forzar el mismo
  camino en las dos variantes → prototipar: sí, comparación de píxeles del paso 3 → si es falso: no
  hay discrepancia de caminos y el arreglo se detiene (sección 11).
- El camino general es el correcto cuando discrepan, porque `componer_tramo` lo declara su referencia
  de fidelidad → prototipar: no → si es falso: el arreglo va en el camino general y la propuesta se
  reescribe antes de tocar código.
- Un interruptor de `perf-video` no llega al APK de producción → prototipar: no → si es falso: el
  interruptor se mueve a `#[cfg(test)]` y el ejemplo pasa a comprobarse con un test de `gba-core`.

### 7. Alcance

- Incluye: contadores de subcamino, interruptor de diagnóstico, informe del cuadro 1000 con coste y
  decisión, corrección de la discrepancia, pruebas.
- No incluye: subir el cambio al fork oficial ni actualizar el `rev` pineado; el shim; los glitches
  de otras capas; el cuadro 1000 en el televisor.

### 8. Criterios de aceptación (no se modifican sin aprobación)

Escenario: El informe muestra que el bit de OBJ cambia el subcamino en la banda
Caso de uso: separar «OBJ dibuja» de «OBJ cambia el camino» | Tipo: normal
  Dado el cuadro 1000 con sus capturas por línea
  Cuando se corre el diagnóstico de subcamino
  Entonces `volcados/subcamino-cuadro-1000.txt` declara 25 líneas con `semi_con_obj` distinto de
  `semi_sin_obj`, todas entre la 133 y la 157, y 135 líneas sin cambio
Test: `tests/subcamino_tests.rs::test_subcamino_cambia_en_la_banda` (se omite si falta `FE_ROM`)
Estado previo: falla (el informe no existe)

Escenario: Los dos caminos dan los mismos píxeles en la banda
Caso de uso: que el camino sea invisible para quien juega | Tipo: normal
  Dado el cuadro 1000
  Cuando se comparan «con OBJ» y «sin OBJ» forzando el mismo camino en las dos
  Entonces la diferencia en las líneas 133 a 157 es 0 píxeles, y sin forzar es 5.290
Test: `tests/subcamino_tests.rs::test_caminos_coinciden_en_la_banda` (se omite si falta `FE_ROM`)
Estado previo: falla (el informe no existe)

Escenario: Una línea sin sprite translúcido no cambia
Caso de uso: no pagar en las 135 líneas que hoy están bien | Tipo: límite
  Dado el cuadro 1000
  Cuando se fuerza el camino general en todas las líneas
  Entonces las líneas con `semi_con_obj` en 0 son idénticas byte a byte a la variante natural
Test: `tests/subcamino_tests.rs::test_lineas_sin_semi_intactas` (se omite si falta `FE_ROM`)
Estado previo: falla (el informe no existe)

Escenario: El informe declara coste y decisión
Caso de uso: elegir el arreglo con un número delante | Tipo: normal
  Dado el cuadro 1000
  Cuando se corre el diagnóstico de subcamino
  Entonces el informe trae la mediana de nanosegundos por línea de cada camino y una línea
  `decision: tramos` o `decision: general`, coherente con la regla de 1,25×
Test: `tests/subcamino_tests.rs::test_coste_y_decision` (se omite si falta `FE_ROM`)
Estado previo: falla (el informe no existe)

### 9. Cómo lo comprobaré

- El informe muestra que el bit de OBJ cambia el subcamino en la banda → comando:
  `cargo run --release --features proto --example ppu_subcamino -- "roms/1575 - Fire Emblem (E)(Independent).gba" secuencias/intro.txt 1000`
  → pasa si: el resumen declara 25 líneas con cambio, todas en 133–157 → no detecta: líneas donde el
  cambio venga de otra causa.
- Los dos caminos dan los mismos píxeles en la banda → el mismo ejemplo, campos
  `banda forzado mismo camino` → pasa si: 0 píxeles → no detecta: diferencias fuera de la banda.
- Una línea sin sprite translúcido no cambia → el mismo ejemplo, campo
  `lineas sin semi identicas` → pasa si: 135 de 135 → no detecta: que la línea sea correcta frente al
  hardware, solo que los dos caminos coinciden.
- El informe declara coste y decisión → el mismo ejemplo, filas `coste` y `decision` → pasa si:
  ambas presentes y la decisión cumple la regla → no detecta: el coste en el televisor.
- Regression del resto del port → `cargo test` → pasa si: 131 tests de `src/`, 18 de empaquetado y
  los de diagnóstico siguen en verde → no detecta: el aspecto en pantalla.
- El port compila con el motor local → `cargo build --release` → pasa si: termina con código 0 →
  no detecta: que el cambio sea aceptable para el fork oficial.

### 10. Terminado cuando

Se cumplen los cuatro criterios de aceptación y el Resultado dice qué camino se corrigió y por qué,
con el coste medido de cada uno.

### 11. Parar si

- El bit 12 no cambia `obj_semi` en esas líneas (primera suposición falsa): parar y rehacer el
  diagnóstico antes de tocar `ppu.rs`.
- Los dos caminos ya dan los mismos píxeles: parar, la causa está en otro sitio y esta propuesta se
  marca como `discrepada`.
- La corrección exige cambiar el comportamiento de más de 5.000 píxeles fuera de esta banda: parar y
  presentar el alcance nuevo antes de aplicarla.
- Dos intentos fallidos seguidos con el mismo criterio: parar, explicar qué se probó y marcar la
  propuesta como `discrepada`.

### Correcciones previas a presentar

- `2026-10-17_localizar-pixeles-obj-y-contenido-bg-cuadro-1000.md` concluyó «H8 descartada» mirando
  solo `razon_camino`. La ruta numerada no cambia, pero el subcamino dentro de la ruta 1 sí puede
  cambiar (`alpha_por_tramos` depende de `obj_semi`, que depende del bit 12). Se corrige la
  inferencia en esa ficha: H8 queda reabierta como esta propuesta.
- Este repositorio no versiona el motor: el arreglo se aplica en `../gba-recomp` y aquí solo se
  valida. Persistirlo exige un commit en el fork y un `rev` nuevo, fuera de alcance.

### Cambios tras prototipos (solo si el plan cambió)

- Antes: dos contadores de subcamino → Ahora: tres (`lineas_alpha_tramos`, `lineas_camino_general`,
  `lineas_con_sprite_semi`) → Por qué: hacía falta saber cuántas líneas traían sprite translúcido
  antes de elegir camino, no solo cuál se eligió.
- Antes: el plan suponía que el camino cambiaba los píxeles → Ahora: la medida lo refuta y la
  propuesta se cierra como `discrepada` (ver «Discrepancia»).

---

## Resultado (lo completa el agente después de editar)

Ejecutado el 2026-10-17 con `examples/ppu_subcamino`, cuadro 1000, motor local. Control válido: la
variante natural coincide con el juego (0 píxeles de 38.400). `cargo test` del port y
`cargo test -p gba-core` en verde (los interruptores valen `false` fuera de la medición).

| Medida | Valor |
| :--- | ---: |
| Líneas con sprite translúcido (con OBJ / sin OBJ) | 28 / 0 |
| Líneas donde cambia el camino al quitar OBJ | 28 |
| Banda 132–159: aportación natural | 6.000 |
| Banda: aportación forzando el mismo camino (general) | 6.000 |
| Banda: diferencia entre caminos (natural vs general forzado) | 0 |
| Líneas sin sprite translúcido idénticas al forzar el general | 132 de 132 |
| Coste mediano de `compose_ns` por línea (tramos / general) | 500 / 1.400 |
| Decisión con la regla de 1,25× | `tramos` |

Predicho vs. real:

- «El bit 12 de `DISPCNT` cambia el subcamino»: **coincide** (28 líneas, de la 132 a la 159).
- «Forzar el mismo camino deja la diferencia en 0»: **difiere** (6.000, no 0). Es el criterio que
  refuta la propuesta.
- «Los dos caminos dan píxeles distintos»: **difiere** (0 píxeles de diferencia entre natural y
  general forzado en las 160 líneas).
- «Las líneas sin sprite translúcido no cambian»: **coincide** (132 de 132).

Qué son entonces los 5.290 píxeles, con los datos del propio informe:

- 5.050 se explican como `alpha_blend(negro, BG2)` y 240 como `alpha_blend(BG2, negro)` con la
  `BLDALPHA` de cada línea. En la banda `EVA` va de 1 a 16: es un fundido.
- Los 710 restantes son, por aritmética, los píxeles que la propuesta anterior contaba como
  cobertura (6.000 − 5.290).
- `DISPCNT` de la banda = `0x1F00` (sin WIN0, WIN1 ni OBJWIN); `BLDCNT` = `0x0F40` (alpha con
  segundos objetivos BG0-BG3).

Es decir: el sprite es una capa negra translúcida que se mezcla con BG2. Quitar OBJ cambia 6.000
píxeles porque el 88 % son mezcla alpha, y la «cobertura» del informe anterior no los ve porque OBJ
sola sobre el fondo negro da el mismo negro que no tener OBJ. Es composición real, no una
discrepancia entre caminos. No se escribieron los tests de la sección 8: la primera suposición
prototipada era falsa y la sección 11 manda parar antes de escribir tests de un plan refutado.

Archivos: `../gba-recomp/crates/gba-core/src/mem.rs` (tres contadores), `../gba-recomp/crates/gba-core/src/ppu.rs`
(contadores y dos interruptores de `perf-video`), `examples/ppu_subcamino.rs` (nuevo), `Cargo.toml`
(una entrada `[[example]]`), `volcados/subcamino-cuadro-1000.txt`. El juego no cambia: los
interruptores compilan a `false` fuera de `perf-video`.

### Fuera de alcance (hallazgos que no se arreglaron)

- La vía por tramos construye sus tramos solo con WIN0 y WIN1 e ignora la ventana de sprites
  (`obj_window`), que el camino general sí consulta en `control_at`. Con OBJWIN parcial y alpha del
  modo 1, los píxeles dentro de la máscara usarían el control de fuera. No se observó en este cuadro
  porque `DISPCNT` = `0x1F00` (sin ventanas); se detectó leyendo el código, no midiendo. — impacto: medio
- La instrumentación y el ejemplo viven en `../gba-recomp` sin commit; el motor local queda con tres
  contadores y dos interruptores nuevos que solo afectan a builds con `perf-video`. — impacto: bajo

---

## Discrepancia (solo si el estado es "discrepada")

Abierta por: agente
Punto afectado: escenario «Los dos caminos dan los mismos píxeles en la banda»
Motivo: forzar el mismo camino deja la diferencia en 6.000 píxeles, no en 0, y los dos caminos dan
exactamente los mismos píxeles en las 160 líneas (0 de diferencia). La causa real de los 5.290 es la
mezcla alpha de un sprite negro translúcido, no la elección de camino.
Clasificación: suposición falsa
Propuesta de corrección: ninguna: no hay nada que arreglar en `ppu.rs`. El hallazgo real (la métrica
de cobertura de los informes de capas) queda anotado en la propuesta
`2026-10-17_localizar-pixeles-obj-y-contenido-bg-cuadro-1000.md`.

---

## Revisión humana

- La banda de 25 líneas del cuadro 1000, comparada con la consola o con el shim en el televisor: ✔ | ✘ | ?
- Detectado solo por el humano:
