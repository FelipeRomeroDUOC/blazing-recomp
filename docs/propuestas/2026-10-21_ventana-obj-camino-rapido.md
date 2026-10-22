# Propuesta: La ventana de sprites deja de ignorarse en el camino rápido de la PPU por software

Fecha: 2026-10-21
Nivel: 2
Estado: verificada por agente

<!-- Estados: pendiente | aprobada | modificada | descartada |
     verificada por agente | revisada por humano | discrepada | cerrada -->

## Propuesta (la escribe el agente antes de editar)

### 1. Qué cambia

El atajo que dibuja una línea entera de golpe (el «camino rápido» de la PPU por software del
motor hermano `gba-recomp`) deja de ignorar la **ventana de sprites** (la que el juego activa con
`DISPCNT` bit 15). A partir de ahora, una línea con esa ventana activada se dibuja píxel a píxel,
igual que el motor original, y la imagen de las escenas con ventana de sprites y mezcla alfa deja
de salir distinta de lo que画出 la PPU nativa del port.

### 2. Contexto

- Contexto consultado: `../gba-recomp/crates/gba-core/src/ppu.rs` (`compose`, `compose_alpha_tramos`,
  `compose_straight`, el cierre `control_at` y el módulo de pruebas), el volcado del upstream fijado
  `volcados/gba-recomp-limpio/crates/gba-core/src/ppu.rs` (rev `de4edf5`), `src/ppu_nativa.rs`
  (`mascara_control`), `src/ppu_nativa.wgsl` (máscara de control del píxel),
  `examples/ppu_camino_general.rs`, `examples/ppu_banda_nativa.rs`, `examples/ppu_tiempos.rs` (nuevo),
  `AGENTS.md`, `docs/BACKLOG.md`, `docs/propuestas/2026-10-19_ppu-nativa-fase-1.md`.
- Intención: la PPU nativa (el objetivo del port) se valida píxel a píxel contra la PPU por software
  del motor hermano. Si esa referencia dibuja mal las escenas con ventana de sprites, la comparación
  no puede validar nada y, además, la ruta de respaldo del port (la PPU por software, que se usa si
  la GPU falla) dibuja la imagen equivocada en esas escenas. El éxito es que el camino rápido y el
  camino general den los mismos píxeles en todas las líneas.
- Restricciones y decisiones previas (máx. 3):
  - La imagen que produce la PPU nativa no cambia (origen: `docs/propuestas/2026-10-19_ppu-nativa-fase-1.md`,
    estado «verificada por agente») → implica: no se toca `src/ppu_nativa.rs` ni `src/ppu_nativa.wgsl`
    → protegida por «La imagen de la PPU nativa no cambia» | testeable.
  - El motor upstream fijado (`de4edf5`) no se modifica (origen: `AGENTS.md` §4.1) → implica: el
    arreglo vive en el parche local (`../gba-recomp`), que es donde está el camino rápido → protegida
    por «El camino rápido con ventana de sprites dibuja lo mismo que el camino general» | testeable.
  - El camino rápido no puede dar un resultado distinto del camino general (origen: los tests que ya
    existen en el motor, p. ej. `alpha_real_por_tramos_dibuja_lo_mismo_que_el_camino_general`) →
    implica: la corrección se prueba con una escena que tiene la ventana de sprites activada, no solo
    con la escena sin ventana → protegida por «Una línea sin ventana de sprites sigue por la vía rápida» | testeable.
- Alternativas descartadas (solo si las hubo):
  - Corregir la PPU nativa del port → descartada porque ya es correcta: con el camino general del
    upstream forzado, la nativa da **0 píxeles distintos de 38400** en el cuadro 700 (medido con
    `examples/ppu_banda_nativa`), o sea que el error está solo en el camino rápido del motor hermano.
  - Ajustar el atajo para que aplique la máscara de la ventana de sprites (tramos por cada intervalo
    de la máscara) → descartada **en esta propuesta** porque mete cambios de estructura en el camino
    caliente (los tramos pasan de un array fijo de 5 a un número variable) y su beneficio no está
    medido; queda anotada como mejora posterior en `docs/BACKLOG.md`.
  - Comparar la PPU nativa contra el camino rápido en vez de contra el camino general → descartada
    porque validaría la imagen contra una referencia que se sabe equivocada.
- Ante imprevistos, prioriza: que la referencia vuelva a ser exacta (aunque el camino rápido pierda
  rendimiento en las escenas afectadas) > conservar el rendimiento del atajo.

### 3. Ejemplos

- Caso normal: el cuadro 700 de la secuencia `secuencias/escena3.txt` (`DISPCNT=0x9F00`,
  `WINOUT=0x313E`, `BLDCNT=0x3F40`, es decir ventana de sprites activada y mezcla alfa con segundos
  objetivos) → antes: el camino rápido y el camino general dibujan **3428 píxeles distintos** en 22
  líneas, porque el atajo usa en toda la línea el control de fuera de ventana y nunca el de dentro de
  la ventana de sprites → ahora: **0 píxeles distintos**, los dos caminos coinciden.
- Caso límite: el mismo cuadro con el bit 12 de `DISPCNT` apagado (sin sprites) → antes: 0
  diferencias → ahora: 0 diferencias (el atajo no se ha tocado donde no había nada que tocar).
- Caso límite: una línea sin ventana de sprites → antes: el atajo la dibuja por tramos → ahora: el
  atajo la sigue dibujando por tramos (no se pierde rendimiento donde no hacía falta).

### 4. Pasos

1. En `../gba-recomp/crates/gba-core/src/ppu.rs`, la decisión de camino por línea (`razon_camino`)
   comprueba la ventana de sprites **antes** que el efecto de color real, de modo que una línea con
   la ventana activada va al camino general por píxel (el del upstream, que sí aplica la máscara).
2. Test de regresión en el módulo de pruebas del motor: una escena con dos fondos opacos, mezcla alfa
   real y un sprite en modo ventana de sprites, que compara el camino rápido con el camino general.
3. Se mide el coste con `examples/ppu_tiempos.rs` (nuevo) antes y después, para dejar el número por
   escrito en el `Resultado` de esta propuesta.
4. Se revalidan los cuadros de referencia del port (5300 y 7500) y la comparación de la PPU nativa.
5. Se anotan en `docs/BACKLOG.md` el estado de la referencia por software y la mejora pendiente
   (tramos con la máscara de la ventana de sprites).

### 5. Qué más podría verse afectado

- `../gba-recomp/crates/gba-core/src/ppu.rs`: `compose` (la cadena `razon_camino` y los contadores
  `descartes_ventana` / `descartes_mezcla`, que cambian de significado en las líneas con ventana de
  sprites) y el módulo `mod tests` (un test nuevo).
- `../gba-recomp/crates/gba-core/tests/perf_counters_tests.rs`: comprueba el reparto de las líneas por
  camino; hay que ver si el reparto cambia en alguna de sus escenas (no usa la ventana de sprites).
- `examples/ppu_tiempos.rs` (nuevo) y `Cargo.toml` (su bloque `[[example]]` con
  `required-features = ["proto"]`).
- `examples/ppu_banda_nativa.rs`: el bloque que compara la nativa con el camino general forzado
  (diagnóstico, no producción).
- `docs/BACKLOG.md`: el estado de la referencia por software.
- Búsqueda hecha: `grep -rn "forzar_camino_general\|lineas_alpha_tramos\|descartes_ventana" src/ tests/
  examples/ ../gba-recomp/crates/gba-core/` para no dejar referencias colgando a la decisión de camino.

### 6. Suposiciones

- El camino general del motor es el que corresponde al hardware real (es decir, el upstream fijado
  `de4edf5` y la PPU nativa del port, que coinciden) → prototipar: sí
  → si es falsa: el arreglo estaría al revés y habría que corregir la PPU nativa en vez del motor.
  → resultado del prototipo: confirmado: en el cuadro 700 la nativa da 0 píxeles distintos de 38400
  frente a la referencia con el camino general forzado, y el upstream fijado aplica la ventana de
  sprites en `control_at`.
- Perder el atajo en las líneas con ventana de sprites no hunde el rendimiento del port → prototipar: sí
  → si es falsa: habría que implementar los tramos con la máscara de la ventana de sprites.
  → resultado del prototipo: medido con `examples/ppu_tiempos.rs`: en la ventana de cuadros 600..800
  el camino general cuesta un 22.6 % más de tiempo de PPU que el rápido (123.28 ms frente a 151.10 ms
  de PPU en 200 cuadros), lo que sobre el tiempo total de cuadro son 0.14 ms de 3.27 ms (un 4.3 %);
  en las escenas sin ventana de sprites el coste es 0, porque el atajo sigue igual.
- La ruta que dibuja el juego en el móvil no usa la PPU por software → prototipar: no
  → si es falsa: el defecto se vería en pantalla y la prioridad subiría.
  → resultado del prototipo: no detectado: en `src/platform.rs` el cuadro lo dibuja la PPU nativa
  (con la marca) o el shim, los dos con la GPU; la PPU por software solo se usa si la GPU falla.

### 7. Alcance

- Incluye: la decisión de camino por línea en el motor hermano, el test de regresión del motor, la
  medición del coste antes y después, y la anotación en el BACKLOG del port.
- No incluye: la implementación de los tramos con la máscara de la ventana de sprites (queda como
  mejora posterior), la PPU nativa del port (`src/ppu_nativa.rs`, `src/ppu_nativa.wgsl`), el shim, y
  cualquier cambio en el upstream fijado `de4edf5`.
- No incluye: el empaquetado del APK (la imagen del juego no cambia) ni la revisión en el dispositivo.

### 8. Criterios de aceptación (no se modifican sin aprobación)

Escenario: El camino rápido con ventana de sprites dibuja lo mismo que el camino general
Caso de uso: la referencia por software dibuja la imagen correcta en las escenas con ventana de
sprites | Tipo: regresión
  Dado que una línea tiene dos fondos opacos, mezcla alfa real con segundos objetivos, la ventana de
  sprites activada y un sprite en modo ventana que cubre parte de la línea, con el control de dentro
  de la ventana distinto del de fuera
  Cuando se compone esa línea con el camino rápido y con el camino general (forzado)
  Entonces los dos dan los mismos 240 píxeles
Test: `cargo test -p gba-core --features perf-video` en `../gba-recomp`, test
`alpha_por_tramos_con_ventana_obj_dibuja_lo_mismo_que_el_camino_general`
Estado previo: falla (el camino rápido da 3428 píxeles distintos en el cuadro real equivalente)

Escenario: Una línea sin ventana de sprites sigue por la vía rápida
Caso de uso: el atajo sigue dando rendimiento donde no estorba | Tipo: regresión
  Dado que la misma escena de dos fondos opacos con mezcla alfa real, sin ventana de sprites
  Cuando se compone esa línea
  Entonces va por la vía por tramos (una búsqueda de la segunda capa por tramo, no 240) y dibuja los
  mismos 240 píxeles que el camino general
Test: `cargo test -p gba-core --features perf-video` en `../gba-recomp`, test
`alpha_real_por_tramos_dibuja_lo_mismo_que_el_camino_general` (ya existente, no debe romperse)
Estado previo: pasa (regresión)

Escenario: El cuadro 700 de la escena 3 coincide con el camino general
Caso de uso: la imagen de la escena con ventana de sprites sale correcta | Tipo: normal
  Dado que el cuadro 700 de `secuencias/escena3.txt` tiene la ventana de sprites activada y la
  mezcla alfa con segundos objetivos
  Cuando se comparan, cuadro a cuadro, el camino rápido y el camino general
  Entonces la diferencia es de 0 píxeles con la ventana de sprites activada y de 0 píxeles también
  con los sprites apagados (hoy: 3428 y 0)
Test: `cargo run --release --features proto --example ppu_camino_general -- <rom> secuencias/escena3.txt 700`
Estado previo: falla (D0 = 3428)

Escenario: La imagen de la PPU nativa no cambia
Caso de uso: el juego se ve igual que antes del arreglo | Tipo: regresión
  Dado que la fase 1 reproduce la imagen de la PPU de referencia píxel a píxel
  Cuando el arnés de comparación vuelve a renderizar los dos cuadros de aceptación
  Entonces las diferencias contra la referencia de fin de cuadro siguen siendo 0 de 38400 en los
  cuadros 5300 y 7500
Test: `cargo test --release --test ppu_nativa_tests test_cuadro_7500_sin_efectos_coincide`
Estado previo: pasa (regresión)

### 9. Cómo lo comprobaré

- El camino rápido con ventana de sprites dibuja lo mismo que el camino general → herramienta o
  comando: `cd ../gba-recomp && cargo test -p gba-core --features perf-video` → pasa si: el test
  nuevo pasa y la suite entera sale en verde (87 + 10 pruebas hoy) → no detecta: que la escena del
  test se parezca a la del juego; por eso está el criterio del cuadro 700.
- Una línea sin ventana de sprites sigue por la vía rápida → herramienta o comando:
  `cd ../gba-recomp && cargo test -p gba-core --features perf-video` → pasa si: el test existente
  `alpha_real_por_tramos_dibuja_lo_mismo_que_el_camino_general` sigue pasando → no detecta: que el
  atajo se haya apagado del todo (eso lo mediría el contador `lineas_alpha_tramos`).
- El cuadro 700 de la escena 3 coincide con el camino general → herramienta o comando:
  `cargo run --release --features proto --example ppu_camino_general -- <rom> secuencias/escena3.txt 700`
  → pasa si: «camino D0: 0» y «camino D1: 0» → no detecta: otras escenas con la ventana de sprites
  que no estén en la secuencia.
- La imagen de la PPU nativa no cambia → herramienta o comando:
  `cargo test --release --test ppu_nativa_tests test_cuadro_7500_sin_efectos_coincide` → pasa si: 0
  diferencias de 38400 → no detecta: los cuadros sin efectos ni ventanas, que son los que usa.
- El coste de perder el atajo en las escenas afectadas → herramienta o comando:
  `cargo run --release --features proto --example ppu_tiempos -- <rom> secuencias/escena3.txt 600 200`
  → pasa si: el informe trae las dos medidas (antes y después) → no detecta: el coste en el móvil,
  que es otra máquina.
- La suite entera del port sigue verde → herramienta o comando: `cargo test --release` → pasa si: las
  pruebas en verde y 0 rojas → no detecta: nada del motor hermano, que tiene su propia suite.

### 10. Terminado cuando

Se cumplen todos los criterios de aceptación y el coste queda medido y escrito en el `Resultado`.

### 11. Parar si

2 intentos fallidos seguidos con el mismo criterio. Detenerse, explicar qué se probó y marcar la
propuesta como «discrepada».

### Correcciones previas a presentar

- La primera idea era corregir la PPU nativa del port, porque el cuadro 700 es el primero donde se
  ve el defecto y la PPU nativa es el objetivo del port. Se descarta al medir: con el camino general
  del upstream forzado, la nativa da 0 píxeles distintos de 38400. El defecto está solo en el camino
  rápido del motor hermano, y la nativa es la que acierta.
- La primera idea de la corrección era añadir la máscara de la ventana de sprites a los tramos del
  camino rápido. Se descarta en esta propuesta (cambia la estructura del camino caliente y su
  beneficio no está medido) y se anota como mejora posterior con su número.

### Cambios tras prototipos (solo si el plan cambió)

- Antes: «la corrección hace que la línea con ventana de sprites vaya al camino general por píxel» →
  Ahora: igual, y además se mide el coste con `examples/ppu_tiempos.rs` y se anota la mejora de los
  tramos con máscara como pendiente → Por qué: la medición dio el 22.6 % de PPU en la ventana
  600..800, que es un 4.3 % del tiempo de cuadro: merece la pena medirlo, no implementarlo a ciegas.

---

## Resultado (lo completa el agente después de editar)

### Predicho vs. real

- **El camino rápido con ventana de sprites dibuja lo mismo que el camino general** → coincide
  Resultado real: el test nuevo `alpha_por_tramos_con_ventana_obj_dibuja_lo_mismo_que_el_camino_general`
  pasa, y se comprobó en rojo antes del arreglo: sin él fallaba con «con la ventana de sprites la
  línea no adopta el atajo por tramos» (3 búsquedas de la segunda capa en vez de 240). La suite del
  motor queda en verde: 88 unitarios + 10 de integración (antes 87 + 10).
  Método: el anunciado.
- **Una línea sin ventana de sprites sigue por la vía rápida** → coincide
  Resultado real: `alpha_por_tramos_da_los_mismos_pixeles` y
  `alpha_real_por_tramos_dibuja_lo_mismo_que_el_camino_general` siguen pasando, y en la ventana de
  cuadros 600..800 del juego el atajo por tramos sigue usándose en 1.920 líneas (el 6 % del
  fotograma).
  Método: el anunciado.
- **El cuadro 700 de la escena 3 coincide con el camino general** → coincide
  Resultado real: «camino D0: 0», «camino D1: 0», «camino control: 0» y «camino lineas con
  diferencia: 0» (antes D0 = 3428 en 22 líneas). Se repitió en los cuadros 620, 660, 740 y 780 de
  la misma secuencia: D0 = 0 en los cuatro.
  Método: el anunciado.
- **La imagen de la PPU nativa no cambia** → coincide
  Resultado real: la suite del port queda en verde entera (`cargo test --release`): 176 unitarios de
  `src/` + 91 de integración de `tests/`, 0 rojos, y entre ellos
  `test_cuadro_7500_sin_efectos_coincide` y `test_el_motor_base_y_el_shim_no_cambian`, que comprueba
  que los ficheros del upstream fijado `de4edf5` siguen con los hashes de
  `tests/datos/ppu-nativa-hashes-base.txt`.
  Método: el anunciado.
- **El coste de perder el atajo en las escenas afectadas** → difiere (peor de lo estimado, pero
  dentro de lo que la propuesta ya aceptaba)
  Resultado real: medido con `examples/ppu_tiempos.rs` en la ventana de cuadros 600..800, con tres
  tomas alternadas por estado para separar el ruido del coste:

  | estado | PPU del camino rápido (mediana de 3) | composición (mediana de 3) | PPU de la ruta general forzada (mediana de 3) |
  |---|---|---|---|
  | antes (con el defecto) | 121,57 ms (121,57 / 134,12 / 118,71) | 26,38 ms | 154,23 ms (154,23 / 164,46 / 147,04) |
  | después (con el arreglo) | 144,40 ms (141,73 / 156,18 / 144,40) | 50,13 ms | 154,77 ms (165,62 / 154,77 / 147,16) |

  La ruta general no cambia de código entre las dos columnas, y se movió un 0,3 %: eso fija el ruido
  de la máquina en ±5 %, muy por debajo del +18,8 % del camino rápido. El coste viene del reparto:
  antes 27.865 líneas usaban el atajo y solo 615 iban por píxel; ahora 1.920 usan el atajo y 26.560
  van por píxel, porque 26.080 líneas de las 32.000 (el 81,5 %) tenían a la vez ventana de sprites
  y alpha real. Sobre el tiempo total de cuadro la diferencia es pequeña: el `reloj` de la pareja de
  la primera medición fue 656,25 ms antes y 658,11 ms después (+0,3 %), porque la PPU son unos 145 ms
  de esos ~657 ms.
  Método: el anunciado, con una diferencia: la primera medición (118,29 ms frente a 154,26 ms) no
  era comparable con la posterior por el ruido de la máquina, así que se repitió por triplicado y
  alternando los dos estados.

### Contexto: ¿se respetó?

- La imagen que produce la PPU nativa no cambia → respetada: no se tocó `src/ppu_nativa.rs` ni
  `src/ppu_nativa.wgsl`, y la suite del port sigue verde con 0 diferencias en los cuadros de
  aceptación.
- El motor upstream fijado (`de4edf5`) no se modifica → respetada: el arreglo vive en el parche local
  `../gba-recomp`, y el test de hashes del port sigue en verde porque comprueba los ficheros del
  checkout fijado, no los del parche local.
- El camino rápido no puede dar un resultado distinto del camino general → respetada: la corrección
  se prueba con una escena que tiene la ventana de sprites activada, no solo con la escena sin
  ventana.
- ¿Se tocó algo que alguna restricción prohibía? no.
- Imprevistos que surgieron: dos. (1) El reparto de los contadores cambia de significado, como ya
  anticipaba el §5: una línea con la ventana de sprites se anota en `descartes_ventana` aunque
  también tenga un alpha real, y `lineas_efecto_alpha` deja de contar esas líneas. (2) El ruido de
  la máquina (±5 %) hace que una sola medición no sirva para un porcentaje de coste, así que hubo
  que medir por triplicado y alternando los estados → criterio de prioridad aplicado: sí, primero
  la exactitud de la referencia (el atajo pierde rendimiento en las escenas afectadas).
- ¿Reapareció alguna alternativa descartada? no.

### Archivos modificados

- `../gba-recomp/crates/gba-core/src/ppu.rs`: la cadena `razon_camino` (la ventana de sprites se mira
  antes que el efecto de color real, con el comentario que lo explica), el comentario del reparto de
  contadores, el test nuevo `alpha_por_tramos_con_ventana_obj_dibuja_lo_mismo_que_el_camino_general`
  con su escena `escena_alpha_con_ventana_obj`, y `PPU_CAMINO_VERSION` de `recto-2` a `recto-3`.
- `docs/propuestas/2026-10-21_ventana-obj-camino-rapido.md`: esta ficha (estado y resultado).
- `docs/guias/medir-rendimiento.md`: la marca de compilación del pipeline y el reparto de las líneas
  con ventana de sprites.
- `docs/BACKLOG.md`: el índice de propuestas y el registro de hallazgos.

¿Coincide con lo anunciado? no del todo: el §5 no mencionaba el cambio de `PPU_CAMINO_VERSION`, y
tampoco que la guía de medición hubiera de cambiar. No se tocó nada más de lo anunciado: ni
`src/ppu_nativa.rs`, ni `src/ppu_nativa.wgsl`, ni el shim, ni el upstream fijado.

### Discrepancias

- La marca de compilación del pipeline (`recto-2` → `recto-3`) no estaba en el §5 → omisión: la
  constante del motor dice que se cambia al tocar el camino caliente, y este cambio altera qué
  líneas toman cada camino, así que un registro de telemetría tiene que poder distinguir las dos
  versiones. El telón del port, que compila contra el parche local, empieza a emitir `ppu=recto-3`.
- El reparto de los contadores cambia de significado en las líneas con ventana de sprites → ya
  anticipado en el §5: la ventana es ahora la primera razón que impide el atajo, y esas líneas se
  cuentan en `descartes_ventana` en vez de `descartes_mezcla`.

### Para la próxima vez

- Cuando el ruido de la máquina sea de un ±5 %, medir por triplicado y alternando los estados antes
  de escribir cualquier porcentaje de coste en una ficha.

### Fuera de alcance (hallazgos que no se arreglaron)

- El atajo por tramos sigue sin componer la máscara de la ventana de sprites: con el arreglo esas
  líneas van píxel a píxel, pero se podría componer por tramos con la máscara y recuperar parte del
  rendimiento. Es la mejora que quedó anotada en el §2 de esta propuesta. | impacto: medio
- `lineas_efecto_alpha` ya no cuenta las líneas con alpha real que además tienen la ventana de
  sprites, así que el reparto por modo del efecto no cubre esas líneas. | impacto: bajo
- La ventana de sprites sigue sin estar soportada por hardware en el shim de GPU del port
  (`src/ppu_shim.rs`), que es un camino distinto del de la PPU por software. | impacto: bajo
- Al auditar las rutas absolutas se vio que el hallazgo del BACKLOG que accuse la ruta local
  `C:/Users/<usuario>/Downloads/...` en `docs/propuestas/2026-10-12_drain-hotspots-sin-perdida.md`
  ya no se cumple: ese archivo no contiene ninguna ruta de usuario. | impacto: bajo

---

## Discrepancia (solo si el estado es «discrepada»)

Abierta por: agente | humano
Punto afectado: [escenario o criterio]
Motivo: [qué difiere o qué falla]
Clasificación: error de juicio | de implementación | suposición falsa | información faltante | omisión | imprevisto
Propuesta de corrección: [enlace al nuevo archivo]

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

- [Escenario o punto]: ✔ | ✘ | ?  [nota de la persona, opcional]
- Detectado solo por el humano: [lo completa la persona]
