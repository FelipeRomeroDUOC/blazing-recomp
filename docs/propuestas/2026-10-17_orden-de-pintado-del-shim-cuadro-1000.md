# Propuesta: Medir si el color de debajo de los 2.523 píxeles "ninguno" es el orden de pintado del shim

Fecha: 2026-10-17
Nivel: 2
Estado: verificada por agente

<!-- Estados: pendiente | aprobada | modificada | descartada |
     verificada por agente | revisada por humano | discrepada | cerrada -->

## Propuesta (la escribe el agente antes de editar)

### 1. Qué cambia

En `capa-bajo-mezcla-ninguno-cuadro-1000` (verificada por agente) quedó medido que los 2.523 píxeles "ninguno" son todos sprites semitransparentes, y que X no sale de ninguna mezcla 50/50 con BG0, BG1, BG3, con BG2 y otra capa, ni con el fondo vacío.

Esta propuesta mide la hipótesis que queda: el shim pinta en un orden fijo y mezcla cada semitransparente con todo lo que ya está en el búfer, incluidos otros sprites. Se miden dos cosas:

- **H60:** X es la mezcla 50/50 del sprite ganador con un sprite que el shim dibuja antes que él en ese píxel.
- **H61:** X es el resultado de simular, píxel a píxel, el orden de pintado completo del shim (fondos y sprites, con mezcla 50/50 de los semitransparentes).

No se cambia `ppu.rs`, `gba-core`, `ppu_shim.rs`, `ppu_shim.wgsl` ni el juego. Es solo medición.

### 2. Contexto

- Contexto consultado:
  - `docs/propuestas/2026-10-17_capa-bajo-mezcla-ninguno-cuadro-1000.md` (verificada por agente): 2.523 "ninguno", todos semitransparentes; H50, H51 y H52 en 0.
  - `docs/propuestas/2026-10-17_origen-del-color-x-en-el-shim-cuadro-1000.md` (verificada por agente): A 2.767; B 0; C 0; control 0.
  - `src/ppu_shim.rs`, `render_frame` (leído en esta sesión): recorre `priority` de 3 a 0. En cada prioridad P pinta primero los fondos activos con prioridad P, de BG3 a BG0, y después los sprites de prioridad P. Los sprites se ordenan con `decode_oam`: prioridad mayor primero, y dentro de una prioridad, índice OAM mayor primero. Por tanto un sprite de OAM menor queda encima. La prueba de profundidad está apagada (`depth_stencil: None`), así que manda el orden de dibujo, no la profundidad.
  - `src/ppu_shim.wgsl`: `fs_sprite` pone `alpha = 0.5` en semitransparentes; el pipeline usa `ALPHA_BLENDING`; los fondos son opacos (alpha 1) salvo color 0.
  - `examples/shim_capa_ninguno.rs`: ya tiene `fondo_texto`, `sprites` (con `semi`, `ind`), `mezcla_rgb` y la clasificación del grupo "ninguno".
- Intención: para los 2.523 píxeles, saber si X sale del orden de pintado del shim (sprites que quedan debajo del ganador) y no de una capa fija.
- Hipótesis:
  - H60: en al menos 2.271 de los 2.523 (90 %), existe un sprite S_k que cubre el píxel, con prioridad y orden OAM tales que el shim lo dibuja antes que el ganador, y X = mezcla_50(S_ganador, S_k), con tolerancia ±1. Si se confirma, un solo sprite debajo explica el color.
  - H61: en al menos 2.271 de los 2.523, X coincide (±1) con la simulación completa del píxel: empezar con el fondo vacío, aplicar en orden de pintado cada fondo con píxel opaco (reemplaza), cada sprite opaco (reemplaza) y cada sprite semitransparente (mezcla_50 con lo que hay). Si se confirma, el shim es un modelo de mezcla en cadena, que la PPU no hace.
- Restricciones y decisiones previas:
  - La PPU por software es la referencia. El shim es lo que se mide.
  - El ROM solo en `roms/`. No sobrescribir ejemplos existentes; el resultado va a un archivo nuevo.
  - Misma tolerancia y misma conversión de canal que en las propuestas anteriores.
  - No se modifica el shim: la simulación es solo en el ejemplo.
- Alternativas descartadas:
  - Medir solo H60 → descartada: si ningún sprite único explica el color, no basta; H61 es la prueba completa del orden.
  - Usar la GPU para leer cada pasada por separado → descartada: requiere cambiar el shim o duplicar el pipeline; la simulación en CPU basta para la prueba.

### 3. Ejemplos

- Caso normal: un píxel con sprite ganador semitransparente S1 (prioridad 1, OAM 10) y otro sprite opaco S2 (prioridad 2, OAM 5). El shim dibuja S2 antes (prioridad mayor). Simulación: fondo vacío → S2 reemplaza → S1 mezcla. Si X = mezcla_50(S1, S2) ±1, cuenta en H60 y en H61.
- Caso límite: un píxel donde el sprite ganador tiene prioridad 0 y un fondo de prioridad 0 se dibuja después de él en el orden del shim. Si el fondo es opaco, reemplaza al sprite en la simulación; si X no coincide, se cuenta como "sin hipótesis" y se reporta aparte.

### 4. Pasos

1. Crear `examples/shim_sprite_debajo.rs` (con `required-features = ["proto"]`), copiando por código de `shim_capa_ninguno.rs`: `Gpu`, `fondo_texto`, `sprites`, `mezcla_rgb`, `componer`, `rgb8`, `cerca`.
2. Cambiar `sprites` para devolver, por píxel, todos los sprites que lo cubren (no solo el ganador), con prioridad, índice OAM, semitransparencia y color. El ganador se sigue eligiendo igual.
3. Control previo: reconstrucción V0 y V1 sobre toda la pantalla debe dar 0, y el control de paleta del fondo debe dar 0. Si no, parar.
4. Recalcular la aportación de 5.290 y el grupo "ninguno" con la clasificación de `origen-x` (A, luego B, luego C). Debe dar 2.523. Si no, parar.
5. Para cada píxel del grupo "ninguno", obtener X del shim (estado de fin, V0).
6. H60: buscar un sprite cubriente que el shim dibuje antes que el ganador, con X = mezcla_50(ganador, S_k) ±1. Contar.
7. H61: simular el orden del shim sobre ese píxel (fondos y sprites en el orden de `render_frame`), con fondo vacío como base. Comparar con X ±1. Contar.
8. Contar los que no cumplen H60 ni H61, y el número de sprites que cubren cada píxel del grupo (distribución).
9. Como dato de contexto (sin parar): comparar la simulación con el shim en toda la pantalla y contar las diferencias. Es una medida de confianza en el modelo, no una condición.
10. Escribir `volcados/sprite-debajo-cuadro-1000.txt` (archivo nuevo) con las líneas de la sección 8.
11. Crear `tests/sprite_debajo_tests.rs` con las pruebas de la sección 8.
12. Ejecutar con `FE_ROM` definida y registrar el resultado.

### 5. Qué más podría verse afectado

- Archivos que cambian: `examples/shim_sprite_debajo.rs` (nuevo), `tests/sprite_debajo_tests.rs` (nuevo), `Cargo.toml` (una entrada), `volcados/sprite-debajo-cuadro-1000.txt` (generado, no versionado).
- Archivos que no cambian: `ppu.rs`, `gba-core`, `ppu_shim.rs`, `ppu_shim.wgsl`, `shim_capa_ninguno.rs`, `shim_origen_x.rs`, el juego, la APK, y todos los informes anteriores.
- Riesgo: la simulación en CPU puede diferir del GPU en el redondeo de la mezcla (f32 frente a 8 bits). El control del paso 9 lo mide, y la tolerancia ±1 lo absorbe. Si el modelo no coincide con el shim en la mayoría de la pantalla, H61 pierde valor y se marca como "dudosa".

### 6. Suposiciones

- `render_frame` es el orden de pintado vigente (leído en esta sesión) → prototipar: sí, la simulación usa ese orden tal cual → si es falsa: parar y avisar.
- El fondo vacío es `palette[0]` y es la base del búfer → prototipar: sí, el control de paleta del fondo lo confirma → si es falsa: parar.
- Los fondos opacos reemplazan y los semitransparentes solo se mezclan con sprites → prototipar: sí, se mide en el paso 9 → si es falsa: H61 se evalúa con la regla que corresponda y se reporta.

### 7. Alcance

- Incluye: H60 y H61 sobre los 2.523 píxeles; control; distribución de sprites por píxel; modelo frente al shim en toda la pantalla como dato de confianza; pruebas; resultado.
- No incluye: cambiar el shim o el shader; corregir el orden de pintado; otros cuadros; ventanas, mezcla de la PPU o el residuo del 1,33 %; Android.

### 8. Criterios de aceptación (no se modifican sin aprobación)

Escenario: la reconstrucción coincide con la PPU
Caso de uso: que las comparaciones sean confiables | Tipo: límite
  Dado el cuadro 1000
  Cuando se corre el ejemplo
  Entonces `control reconstruccion V0`, `control reconstruccion V1` y `control paleta fondo` valen 0
Test: `tests/sprite_debajo_tests.rs::test_reconstruccion_coincide` (se omite si no está definida `FE_ROM`)
Estado previo: falla (el informe no existe)

Escenario: el grupo "ninguno" suma lo que indica el informe anterior y cada píxel queda clasificado
Caso de uso: que el grupo analizado sea el mismo y que H60, H61 y "sin hipótesis" particionen el grupo | Tipo: normal
  Dado el informe
  Cuando se leen `ninguno total`, `ninguno H60`, `ninguno H61` y `ninguno sin hipotesis`
  Entonces `ninguno total` vale 2523, y la suma de H60, H61 y "sin hipótesis" es igual a `ninguno total`
Test: `tests/sprite_debajo_tests.rs::test_grupo_suma` (se omite si no está definida `FE_ROM`)
Estado previo: falla (el informe no existe)

Escenario: el informe da H60, H61 y el dato de confianza del modelo
Caso de uso: decidir si el orden de pintado explica X | Tipo: normal
  Dado el informe
  Cuando se leen sus líneas
  Entonces existen `ninguno H60`, `ninguno H61`, `ninguno sin hipotesis` y `modelo vs shim pantalla`
Test: `tests/sprite_debajo_tests.rs::test_sprite_informe_completo` (se omite si no está definida `FE_ROM`)
Estado previo: falla (el informe no existe)

### 9. Cómo lo comprobaré

- Ejecución: `cargo run --release --features proto --example shim_sprite_debajo -- "roms\1575 - Fire Emblem (E)(Independent).gba" secuencias\intro.txt 1000` → pasa si: existe `volcados/sprite-debajo-cuadro-1000.txt` con todas las líneas de la sección 8 → no detecta: otros cuadros.
- Pruebas: con `FE_ROM` definida, `cargo test --features proto --test sprite_debajo_tests` → pasa si: las tres pruebas pasan.
- Regresión: `capa-ninguno-cuadro-1000.txt`, `origen-x-cuadro-1000.txt` y `identificar-x-cuadro-1000.txt` no cambian.

### 10. Terminado cuando

Se cumplen los tres criterios de aceptación y el Resultado dice cuántos de los 2.523 píxeles explican H60 y H61, cuántos quedan sin hipótesis, y cuántas diferencias tiene el modelo frente al shim en toda la pantalla.

### 11. Parar si

- El control de reconstrucción o de paleta no da 0: parar, porque la comparación no sería confiable.
- El grupo "ninguno" no es 2.523: parar y avisar, porque el grupo analizado sería otro.
- No hay adaptador de GPU: parar y avisar, sin escribir informe.
- Dos intentos fallidos seguidos con el mismo criterio: detenerse, explicar qué se probó y marcar la propuesta como `discrepada`.

---

## Resultado (lo completa el agente después de editar)

Ejecución (2026-10-17): `shim_sprite_debajo` escribió `volcados/sprite-debajo-cuadro-1000.txt`. El job terminó con código 1 por avisos de cargo; el archivo de salida confirma el resultado. Pruebas `sprite_debajo_tests`: 3 pasan, 0 fallan.

Controles: reconstrucción V0 = 0, V1 = 0; control paleta fondo = 0; control ganador = 0 (el ganador de la lista de fragmentos coincide con el de `sprites`). Pasan.

Conteos del grupo ninguno (2.523; aportación total 5.290):
- cubre 1 sprite: 2.523. Cubre 2: 0. Cubre 3 o más: 0.
- H60: 0.
- H61: 0.
- sin hipótesis: 2.523.
- suma: 2.523.
- modelo vs shim pantalla: 21.070 de 38.400 píxeles no coinciden.

Hipótesis:
- H60 descartada, y no puede darse: en los 2.523 píxeles el sprite ganador es el único que cubre el píxel. No hay otro sprite debajo.
- H61 no concluyente: el modelo del orden de pintado no reproduce el shim en la pantalla (21.070 diferencias). Un 0 en H61 no descarta el orden de pintado, porque el modelo está mal.

Lectura:
- Como el único sprite del píxel es el ganador, el color de debajo tiene que venir de un fondo o del fondo vacío, tal como lo ve el shim.
- H50, H51 y H52 (propuesta anterior) usan los colores de fondo que reconstruye `fondo_texto`, que coincide con la PPU (control 0). Dieron 0. Eso sugiere que el shim usa colores de fondo distintos de los de la PPU en esos píxeles.
- Dato histórico (sesión anterior): el shim con solo fondos (V3) difiere de la PPU en 37.690 de 38.400 píxeles. Es coherente con lo anterior: el shim no muestra el mismo contenido de fondo que la PPU.
- Siguiente paso propuesto: medir, capa por capa, el contenido de BG0 a BG3 que muestra el shim y compararlo con `fondo_texto`. Requiere propuesta nueva.

---

## Revisión humana

- Que el orden de pintado del shim se acepte como explicación del color X en estos píxeles, y que H60 y H61 se confirmen o descarten: ✔ | ✘ | ?
