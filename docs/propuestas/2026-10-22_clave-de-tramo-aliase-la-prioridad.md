# Propuesta: La clave de tramo no debe aliase la prioridad del sprite

Fecha: 2026-10-22
Nivel: 2
Estado: pendiente

<!-- Estados: pendiente | aprobada | modificada | descartada |
     verificada por agente | revisada por humano | discrepada | cerrada -->

## Propuesta (la escribe el agente antes de editar)

### 1. Qué cambia

Cuando la composición por tramos del alpha (el camino rápido del modo 1 de `BLDCNT`) encuentra un
sprite que cambia de prioridad justo donde un fondo cambia de opacidad, deja de pintar el sprite en
esa tira de píxeles y pinta el fondo: la imagen sale distinta de la que da el camino general, que es
la referencia.

### 2. Contexto

- Contexto consultado: `../gba-recomp/crates/gba-core/src/ppu.rs` (el parche local, con el camino
  rápido sin commitear), `../gba-recomp/crates/gba-core/src/mem.rs` (los contadores de camino),
  [`docs/explicacion/ppu-caminos-de-composicion.md`](../explicacion/ppu-caminos-de-composicion.md)
  (bloque 11, donde se describe y se midió), y la propuesta que arregló la ventana de sprites
  ([`2026-10-21_ventana-obj-camino-rapido.md`](2026-10-21_ventana-obj-camino-rapido.md)).
- Intención: que el camino rápido dé **exactamente** los mismos píxeles que el camino general, que
  es el criterio de fidelidad del port. Hoy hay una escena en la que no los da.
- Restricciones y decisiones previas (máx. 3):
  - El upstream fijado (`de4edf59`) no se modifica; el arreglo vive en el parche local
    (origen: [`2026-10-19_ppu-nativa-fase-1.md`](2026-10-19_ppu-nativa-fase-1.md) y
    `tests/datos/ppu-nativa-hashes-base.txt`) → implica: solo se toca `crates/gba-core/src/ppu.rs` del
    checkout local → protegida por: *El motor fijado no cambia de hash* | no testeable
  - El camino rápido tiene que dar los mismos píxeles que el camino general (origen:
    [`2026-10-13_alpha-real-por-tramos.md`](2026-10-13_alpha-real-por-tramos.md)) → implica: el
    arreglo se prueba comparando píxeles contra el camino general forzado, no solo contando
    llamadas → protegida por: *El camino por tramos dibuja lo mismo que el camino general*.
  - No se debilitan aserciones ni se toca código de producción para forzar el paso de un test
    (origen: `AGENTS.md` §5) → implica: el test nuevo tiene que fallar en rojo por el defecto
    concreto → protegida por: *La escena de la colisión falla en rojo antes del arreglo*.
- Alternativas descartadas (solo si las hubo):
  - Volver a composition píxel a píxel en el modo 1 → descartada porque deshace el atajo entero
    (27.865 líneas de 32.000 en la ventana medida de la escena 3) y el defecto se puede corregir en
    una función de 15 líneas.
  - Usar dos campos separados en vez de un número → descartada por ahora porque cuesta una
    comparación más por píxel; queda como alternativa si la corrección de bits no basta.
- Ante imprevistos, prioriza: la fidelidad (el camino por tramos tiene que dar los mismos píxeles
  que el general) sobre el rendimiento del camino rápido.

### 3. Ejemplos

- Caso normal: una línea con alpha real, dos fondos opacos y un sprite opaco de prioridad fija →
  antes: el camino por tramos y el general dan los mismos 240 píxeles → ahora: igual (no cambia).
- Caso límite: una línea con alpha real donde BG1 es opaca solo en x = 8..15, un sprite de
  prioridad 2 en x = 0..7 y otro de prioridad 0 en x = 8..15 → antes: el camino por tramos pinta el
  color de BG0 en x = 8..15 (`0x003F`, `0x005F`, …) y el general pinta el color del sprite
  (`0x03E0`): 8 píxeles distintos → ahora: los dos caminos coinciden en los 240 píxeles.

### 4. Pasos

1. Mover la prioridad del sprite a bits que la máscara de fondos no use, en `clave_de_tramo`.
2. Añadir el test de regresión con la escena de la colisión, que compara con el camino general
   forzado.
3. Comprobar que la suite del motor sigue en verde y que el reparto de contadores no cambia.

### 5. Qué más podría verse afectado

- `compose_alpha_tramos` (el bucle que extiende el tramo comparando claves): si la clave cambia de
  forma, cambia el número de tramos y, por tanto, el número de llamadas a `top_two_layers`. El
  contador `BUSQUEDAS_DOS_CAPAS` de los tests lo detecta.
- `componer_tramo`: no cambia, pero recibe la pareja de capas que ahora es correcta.
- Los contadores `lineas_alpha_tramos` y `lineas_rectas`: no cambian (la decisión de camino es la
  misma), pero el reparto por tramos dentro de la línea sí.
- `docs/explicacion/ppu-caminos-de-composicion.md`: el bloque 11 deja de describir un defecto vivo.

### 6. Suposiciones

- La clave de tramo tiene que resumir **todo** lo que `top_two_layers` usa del píxel (la máscara de
  fondos opacos y la prioridad del sprite) → prototipar: sí
  → si es falso: habría que buscar otro campo que falte en la clave, y el arreglo sería más grande
  → resultado del prototipo: confirmado con la escena del §3: dos píxeles con la misma clave
  resuelven parejas distintas (BG0, OBJ) y (OBJ, BG0), y el tramo se pinta con la del primero.
- El defecto solo aparece en el modo 1 (alpha) → prototipar: sí
  → si es falso: habría que revisar también el camino recto
  → resultado del prototipo: confirmado: `clave_de_tramo` solo se usa en `compose_alpha_tramos`.
- La escena con la colisión es representativa de lo que pasa en el juego → prototipar: no
  → no detecta: la escena es sintética; el juego puede no tener nunca dos sprites contiguos con
  prioridades distintas en la misma línea, y el defecto seguiría latente.

### 7. Alcance

- Incluye:
  - El cambio de la clave de tramo en `crates/gba-core/src/ppu.rs` (parche local).
  - El test de regresión con la escena de la colisión, en el módulo de tests del mismo fichero.
  - La actualización del bloque 11 de la página de explicación.
- No incluye:
  - Cambios en la PPU nativa del port (`src/ppu_nativa.rs` y `src/ppu_nativa.wgsl`): compone
    siempre por píxel y no tiene este defecto.
  - El arreglo del shim de GPU del port, que es otro camino de dibujo.
  - La mejora del atajo para componer la máscara de la ventana de sprites, que está anotada aparte en
    el [`BACKLOG`](../BACKLOG.md).

### 8. Criterios de aceptación (no se modifican sin aprobación)

Escenario: El camino por tramos dibuja lo mismo que el camino general
Caso de uso: una línea con alpha real donde el sprite y un fondo cambian a la vez | Tipo: regresión
  Dado que la línea de la escena de la colisión (BG1 opaca solo en x = 8..15, sprite de prioridad 2
  en x = 0..7 y sprite de prioridad 0 en x = 8..15, `BLDCNT = 0x0241`, `BLDALPHA = 0x0808`)
  Cuando se compone la línea por el camino por tramos y, en paralelo, por el camino general forzado
  Entonces los 240 píxeles de las dos líneas son idénticos, y antes del arreglo difieren los 8
  píxeles de x = 8..15
Test: `crates/gba-core/src/ppu.rs::tests::alpha_por_tramos_no_aliase_la_prioridad_del_sprite`
Estado previo: [se completa al escribir el test: falla]

Escenario: La escena de la colisión falla en rojo antes del arreglo
Caso de uso: el test nuevo, sin el arreglo | Tipo: regresión
  Dado que el motor tal y como está hoy
  Cuando se ejecuta el test de la colisión
  Entonces falla con un mensaje que nombre los píxeles distintos (8 de 240), no con un error de
  compilación
Test: `crates/gba-core/src/ppu.rs::tests::alpha_por_tramos_no_aliase_la_prioridad_del_sprite`
Estado previo: [se completa al escribir el test: falla]

Escenario: La línea con un solo sprite sigue usando un tramo por tramo de clave
Caso de uso: una línea con alpha real y un sprite opaco en medio | Tipo: normal
  Dado que la escena con dos fondos opacos y un sprite opaco de prioridad 0 en x = 20..27
  Cuando se compone la línea por el camino por tramos
  Entonces la clave de tramo cambia en el sprite, la línea se parte en tres tramos y el contador de
  búsquedas de la segunda capa vale 3
Test: `crates/gba-core/src/ppu.rs::tests::alpha_por_tramos_da_los_mismos_pixeles`
Estado previo: [se completa al escribir el test: pasa (regresión)]

Escenario: El motor fijado no cambia de hash
Caso de uso: el port compila contra el parche local, pero el upstream fijado es intocable | Tipo:
regresión
  Dado que el checkout fijado `de4edf59`
  Cuando se calculan los hashes de `crates/gba-core/src/ppu.rs` y `crates/gba-core/src/mem.rs`
  Entonces coinciden con los de `tests/datos/ppu-nativa-hashes-base.txt`
Test: `tests/packaging_tests.rs::test_el_motor_base_y_el_shim_no_cambian`
Estado previo: [se completa al escribir el test: pasa (regresión)]

### 9. Cómo lo comprobaré

- El camino por tramos dibuja lo mismo que el camino general → herramienta o comando:
  `cargo test -p gba-core --features perf-video alpha_por_tramos_no_aliase_la_prioridad_del_sprite`
  → pasa si: el test pasa y el mensaje de fallo (en rojo, antes del arreglo) nombra 8 píxeles
  distintos de 240 → no detecta: solo cubre la escena sintética; una escena con otra forma de
  colisión podría seguir fallando.
- La escena de la colisión falla en rojo antes del arreglo → herramienta o comando:
  `cargo test -p gba-core --features perf-video alpha_por_tramos_no_aliase_la_prioridad_del_sprite`
  (ejecutado con el código sin arreglar) → pasa si: el test falla por la diferencia de píxeles
  → no detecta: no comprueba que el arreglo sea el correcto, solo que el defecto se reproduce.
- La línea con un solo sprite sigue usando un tramo por tramo de clave → herramienta o comando:
  `cargo test -p gba-core --features perf-video alpha_por_tramos_da_los_mismos_pixeles`
  → pasa si: el contador de búsquedas vale 1 (línea sin sprite) y 3 (línea con el sprite en medio)
  → no detecta: el número de tramos depende de la escena, no del criterio general.
- El motor fijado no cambia de hash → herramienta o comando:
  `cargo test --test packaging_tests test_el_motor_base_y_el_shim_no_cambian`
  → pasa si: el test pasa → no detecta: comprueba los ficheros del checkout fijado, no los del
  parche local, que es donde vive el arreglo.

### 10. Terminado cuando

Se cumplen todos los criterios de aceptación.

### 11. Parar si

2 intentos fallidos seguidos con el mismo criterio. Detenerse, explicar qué se probó y marcar la
propuesta como "discrepada".

### Correcciones previas a presentar

La primera versión del análisis daba por hecho que la prioridad del sprite ocupaba los bits 8-15
(como decía el comentario del propio código). Al comprobarlo con la escena de la colisión se vio
que `0x100 | prioridad` deja la prioridad en los bits 0-1, que son los mismos que usa la máscara
de fondos: por eso la colisión es posible. El texto del §6 y del bloque 11 de la página de
explicación se corrigieron con esa medición.

### Cambios tras prototipos (solo si el plan cambió)

- Antes: «la prioridad del sprite va en los bits 8-15» → Ahora: «la prioridad va en `0x100 |
  prioridad`, es decir, en los bits 0-1, los mismos que la máscara de fondos» → Por qué: la escena
  de la colisión, medida sobre una copia del motor, da 8 píxeles distintos entre el camino por
  tramos y el general.

---

## Resultado (lo completa el agente después de editar)

### Predicho vs. real

- Todavía no se ha ejecutado: la propuesta está pendiente de aprobación.

### Contexto: ¿se respetó?

- Todavía no se ha ejecutado.

### Archivos modificados

- Todavía no se ha ejecutado.

### Discrepancias

- Todavía no se ha ejecutado.

### Para la próxima vez

- Una clave que resume dos cosas en un solo número debería llevar los campos en bits que no se
  solapen, o comparar los campos por separado: un «OR» de dos máscaras de bits que se pisan es un
  aliasing silencioso.

### Fuera de alcance (hallazgos que no se arreglaron)

- El defecto vive en el parche local (`../gba-recomp`), cuyo trabajo sigue sin commitear: el APK lo
  lleva por `[patch]`, así que un clon limpio compila el upstream fijado, que no tiene camino
  rápido y, por tanto, no tiene este defecto. | impacto: medio
- La PPU nativa del port (`src/ppu_nativa.rs`) compone siempre por píxel en la GPU: no sufre este
  defecto, pero tampoco se benefica del atajo por tramos, así que su coste por fotograma es el de
  la composición completa. | impacto: bajo

---

## Revisión humana

- El defecto es del camino rápido del motor local y no cambia la imagen que se ve en el móvil por
  defecto (la PPU nativa compone por píxel): ¿interesa revisarlo en el televisor antes de tocarlo, o
  basta con el test de regresión? ✔ | ✘ | ?
- Detectado solo por el humano:
