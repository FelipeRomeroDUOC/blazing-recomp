# Propuesta: Medir la capa debajo de los 2.523 píxeles de origen "ninguno" en el cuadro 1000

Fecha: 2026-10-17
Nivel: 2
Estado: verificada por agente

<!-- Estados: pendiente | aprobada | modificada | descartada |
     verificada por agente | revisada por humano | discrepada | cerrada -->

## Propuesta (la escribe el agente antes de editar)

### 1. Qué cambia

En `origen-del-color-x-en-el-shim-cuadro-1000` (verificada por agente) quedó medido que, de los 5.290 píxeles de aportación, 2.767 son la mezcla 50/50 del sprite semitransparente ganador con BG2, y 2.523 quedan en "origen ninguno". En esos 2.523, X no es ninguna entrada de la paleta ni el fondo vacío. Es un color mezclado, y no se sabe con qué capa se mezcla.

Esta propuesta mide, en esos 2.523 píxeles, qué capa de fondo (BG0, BG1, BG3 o el fondo vacío) produce X al mezclarse 50/50 con el sprite ganador, y si el sprite es semitransparente.

No se cambia `ppu.rs`, `gba-core`, `ppu_shim.rs`, `ppu_shim.wgsl` ni el juego. Es solo medición.

### 2. Contexto

- Contexto consultado:
  - `docs/propuestas/2026-10-17_origen-del-color-x-en-el-shim-cuadro-1000.md` (verificada por agente): A 2.767, B 0, C 0, ninguno 2.523; suma 5.290; control 0.
  - `docs/propuestas/2026-10-17_capa-debajo-del-sprite-cuadro-1000.md` (verificada por agente): la capa debajo del sprite en V1 es BG2, opaca en los 5.290.
  - `examples/shim_origen_x.rs`: ya tiene `fondo_texto`, `sprites` (con `semi`, `ind`), `mezcla_50`, `componer` y el control de composición completa. Se copian por código.
  - `src/ppu_shim.wgsl`: `fs_sprite` pone `alpha = 0.5` en sprites semitransparentes; el pipeline usa `ALPHA_BLENDING`, y las capas de fondo se dibujan con su propia pasada.
- Intención: para cada uno de los 2.523 píxeles, saber qué capa de fondo está debajo del sprite en el shim, y si el sprite es semitransparente.
- Hipótesis:
  - H50: en al menos 2.271 de los 2.523 (90 %), X es la mezcla 50/50 del sprite ganador con BG0, BG1 o BG3, con tolerancia ±1 por canal. Si se confirma, la capa de debajo es un fondo distinto de BG2.
  - H51: en al menos 2.271 de los 2.523, X es una doble mezcla: `mezcla_50(mezcla_50(sprite, BG2), BG_k)` para algún k en {0, 1, 3}, ±1. Si se confirma, el shim mezcla dos veces.
  - H52: en al menos 2.271 de los 2.523, X es la mezcla 50/50 del sprite con el fondo vacío (palette[0]), ±1. Si se confirma, el shim no tiene fondo debajo en esos píxeles.
- Restricciones y decisiones previas:
  - La PPU por software es la referencia. El shim es lo que se mide.
  - El ROM solo en `roms/`. No sobrescribir ejemplos existentes; el resultado va a un archivo nuevo.
  - Misma tolerancia y misma conversión de canal que en `origen-del-color-x`.
- Alternativas descartadas:
  - Ampliar `shim_origen_x` con las tres hipótesis → descartada: mezclaría una medición verificada con otra en curso; es más claro un ejemplo nuevo.
  - Medir solo con la PPU, sin el shim → descartada: X es el color del shim; hay que leerlo del shim.

### 3. Ejemplos

- Caso normal: un píxel del grupo "ninguno" con sprite semitransparente de color S y BG0 de color B0. Si X = mezcla_50(S, B0) ±1, cuenta en H50 como "mezcla con BG0".
- Caso límite: un píxel del grupo "ninguno" con sprite no semitransparente. Si X = S exacto no entra, porque no hay mezcla. Se cuenta en "sprite no semi" y no en H50.

### 4. Pasos

1. Crear `examples/shim_capa_ninguno.rs` (con `required-features = ["proto"]`), copiando por código de `shim_origen_x.rs`: `Gpu`, `fondo_texto`, `sprites`, `mezcla_50`, `componer`, `rgb8`, `cerca`.
2. Control previo: reconstrucción V0 y V1 sobre toda la pantalla debe dar 0, y el control de paleta del fondo debe dar 0. Si no, parar.
3. Recalcular la aportación de 5.290 y el grupo "ninguno" con la misma clasificación de `origen-x`: A primero, luego B, luego C. Debe dar 2.523. Si no, parar.
4. Para cada píxel del grupo "ninguno", obtener X del shim (estado de fin, V0).
5. Contar: sprite semitransparente o no; y, para sprites semitransparentes, cuántos cumplen H50 (con BG0, BG1 o BG3, cuál), H51 (con qué k) y H52.
6. Contar los que no cumplen ninguna hipótesis.
7. Escribir `volcados/capa-ninguno-cuadro-1000.txt` (archivo nuevo) con las líneas de la sección 8.
8. Crear `tests/capa_ninguno_tests.rs` con las pruebas de la sección 8.
9. Ejecutar con `FE_ROM` definida y registrar el resultado.

### 5. Qué más podría verse afectado

- Archivos que cambian: `examples/shim_capa_ninguno.rs` (nuevo), `tests/capa_ninguno_tests.rs` (nuevo), `Cargo.toml` (una entrada), `volcados/capa-ninguno-cuadro-1000.txt` (generado, no versionado).
- Archivos que no cambian: `ppu.rs`, `gba-core`, `ppu_shim.rs`, `ppu_shim.wgsl`, `shim_origen_x.rs`, `shim_capa_x.rs`, el juego, la APK, y todos los informes anteriores.
- Riesgo: si ninguna hipótesis se cumple, el resultado es "capa no identificada"; no se descarta nada, y habrá que mirar el orden de las pasadas del shim (opacidad y profundidad).

### 6. Suposiciones

- El control de composición y la clasificación heredados de `origen-x` siguen siendo válidos → prototipar: sí, recalculando el grupo y exigiendo 2.523 → si es falsa: parar.
- La mezcla 50/50 se hace en el espacio de valores de 8 bits, como en `origen-x` → prototipar: sí, con la misma función `mezcla_50` → si es falsa: H50 a H52 se evalúan con tolerancia mayor y se marcan como "dudosas".
- Los sprites no semitransparentes no se mezclan en el shim → prototipar: sí, contándolos aparte → si es falsa: se reporta como hallazgo, no se descarta.

### 7. Alcance

- Incluye: identificar la capa de debajo (BG0, BG1, BG3, fondo vacío o ninguna) de los 2.523 píxeles; diferenciar sprite semitransparente de no semitransparente; control; pruebas; resultado.
- No incluye: cambiar el shim; corregir el shader; otros cuadros; ventanas, mezcla de la PPU o el residuo del 1,33 %; Android.

### 8. Criterios de aceptación (no se modifican sin aprobación)

Escenario: la reconstrucción coincide con la PPU
Caso de uso: que las comparaciones sean confiables | Tipo: límite
  Dado el cuadro 1000
  Cuando se corre el ejemplo
  Entonces `control reconstruccion V0`, `control reconstruccion V1` y `control paleta fondo` valen 0
Test: `tests/capa_ninguno_tests.rs::test_reconstruccion_coincide` (se omite si no está definida `FE_ROM`)
Estado previo: falla (el informe no existe)

Escenario: el grupo "ninguno" suma lo que indica el informe anterior
Caso de uso: que el grupo analizado sea el mismo | Tipo: normal
  Dado el informe
  Cuando se lee `origen ninguno`
  Entonces vale 2.523
Test: `tests/capa_ninguno_tests.rs::test_ninguno_es_2523` (se omite si no está definida `FE_ROM`)
Estado previo: falla (el informe no existe)

Escenario: el informe da la capa de debajo y el tipo de sprite
Caso de uso: decidir H50, H51 y H52 | Tipo: normal
  Dado el informe
  Cuando se leen sus líneas
  Entonces existen `ninguno sprite semi`, `ninguno sprite no semi`, `ninguno H50 BG0`, `ninguno H50 BG1`, `ninguno H50 BG3`, `ninguno H51`, `ninguno H52` y `ninguno sin hipotesis`
Test: `tests/capa_ninguno_tests.rs::test_capa_informe_completo` (se omite si no está definida `FE_ROM`)
Estado previo: falla (el informe no existe)

### 9. Cómo lo comprobaré

- Ejecución: `cargo run --release --features proto --example shim_capa_ninguno -- "roms\1575 - Fire Emblem (E)(Independent).gba" secuencias\intro.txt 1000` → pasa si: existe `volcados/capa-ninguno-cuadro-1000.txt` con todas las líneas de la sección 8 → no detecta: otros cuadros.
- Pruebas: con `FE_ROM` definida, `cargo test --features proto --test capa_ninguno_tests` → pasa si: las tres pruebas pasan.
- Regresión: `origen-x-cuadro-1000.txt`, `identificar-x-cuadro-1000.txt` y `shim-fondo-cuadro-1000.txt` no cambian.

### 10. Terminado cuando

Se cumplen los tres criterios de aceptación y el Resultado dice cuántos de los 2.523 píxeles son de sprite semitransparente y no semitransparente, qué capa de debajo explica X (si alguna) y si H50, H51 y H52 se confirman o se descartan.

### 11. Parar si

- El control de reconstrucción o de paleta no da 0: parar, porque la comparación no sería confiable.
- El grupo "ninguno" no es 2.523: parar y avisar, porque el grupo analizado sería otro.
- No hay adaptador de GPU: parar y avisar, sin escribir informe.
- Dos intentos fallidos seguidos con el mismo criterio: detenerse, explicar qué se probó y marcar la propuesta como `discrepada`.

---

## Resultado (lo completa el agente después de editar)

Ejecución (2026-10-17): `shim_capa_ninguno` escribió `volcados/capa-ninguno-cuadro-1000.txt`. El job terminó con código 1 por avisos de cargo; el archivo de salida confirma el resultado. Pruebas `capa_ninguno_tests`: 3 pasan, 0 fallan.

Controles: reconstrucción V0 = 0, V1 = 0; control paleta fondo = 0. Pasan. Aportación total 5.290; grupo ninguno 2.523, como se esperaba.

Conteos del grupo ninguno (2.523):
- sprite semitransparente: 2.523 (todos).
- sprite no semitransparente: 0.
- sin sprite: 0.
- H50 con BG0 / BG1 / BG3: 0 / 0 / 0.
- H51 (doble mezcla con BG2 y BG_k): 0.
- H52 (mezcla con fondo vacío): 0.
- sin hipótesis: 2.523.
- suma: 2.523.

Hipótesis:
- H50 descartada: ningún píxel es mezcla 50/50 del sprite con BG0, BG1 ni BG3.
- H51 descartada: ningún píxel es doble mezcla con BG2 y otra capa.
- H52 descartada: ningún píxel es mezcla con el fondo vacío.

Lectura:
- Los 2.523 píxeles son todos de sprite semitransparente. El color X no es explicado por ninguna mezcla con fondos de texto ni con el fondo vacío.
- Como X no es una entrada de la paleta ni una mezcla con fondos, la capa de debajo que usa el shim probablemente es otro sprite. El shim dibuja los sprites en orden con mezcla alfa del GPU, así que un sprite semitransparente se mezcla con lo que ya haya en el búfer, incluido otro sprite que no es el ganador de la PPU.
- Esto no está medido. Es la hipótesis que queda para una propuesta nueva.

---

## Revisión humana

- Que la capa de debajo de los píxeles "ninguno" se acepte como explicación del color X del shim, y que H50, H51 y H52 se confirmen o descarten: ✔ | ✘ | ?
