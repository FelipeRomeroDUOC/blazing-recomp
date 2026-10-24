# Propuesta: Identificar el color X del shim en los 5.290 píxeles de aportación del cuadro 1000

Fecha: 2026-10-17
Nivel: 2
Estado: verificada por agente

<!-- Estados: pendiente | aprobada | modificada | descartada |
     verificada por agente | revisada por humano | discrepada | cerrada -->

## Propuesta (la escribe el agente antes de editar)

### 1. Qué cambia

En `shim-fondo-cuadro-1000` quedó medido que el shim da un color propio X en los 5.290 píxeles de aportación. Con solo OBJ (V2) el shim da el fondo vacío, como la PPU. Con los fondos encendidos (V0 y V1) da X en ambos casos, así que ignora el sprite ganador en esos píxeles. Falta saber de qué capa sale X y por qué el shim no dibuja el sprite.

No se cambia `ppu.rs`, `gba-core`, `ppu_shim.rs`, `ppu_shim.wgsl` ni el juego. Es solo medición.

### 2. Contexto

- Contexto consultado:
  - `docs/propuestas/2026-10-17_shim-contra-fondo-vacio-cuadro-1000.md` (verificada por agente): X distinto de V0, V1 y V3 en los 5.290; V2 del shim igual a la PPU V2.
  - `docs/propuestas/2026-10-17_capa-ganadora-por-pixel-obj-cuadro-1000.md` (revisada por humano): sprite ganador en V0 en los 5.290; H15 (color del sprite igual al fondo vacío); control de reconstrucción en 0.
  - `examples/ppu_capa_ganadora.rs`: ya reconstruye los fondos de texto y los sprites por línea (`fondo_texto`, `sprites`, `componer`) y clasifica los píxeles. Se copia lo necesario, no se importa.
  - `examples/shim_aportacion.rs` y `shim_fondo`: ya dan el color del shim (fin de cuadro y por línea) para V0, V1 y V2.
- Intención: para cada uno de los 5.290 píxeles, saber qué fondo de texto (BG0, BG1, BG2, BG3) tiene el color X en la PPU, y qué prioridad tiene el sprite ganador frente a ese fondo.
- Hipótesis:
  - H30: en la mayoría de los 5.290 (al menos el 90 %), X coincide con el color de un fondo de texto (BG0, BG1 o BG3) en la PPU. Se confirma si el conteo es al menos 4.761. Si X no coincide con ninguno, la hipótesis se descarta.
  - H31: en los 5.290, la prioridad del sprite ganador es igual a la del fondo que muestra el shim (empate). En el empate, la PPU da el sprite y el shim da el fondo. Se confirma si todos los 5.290 tienen prioridad del sprite igual a la del fondo.
- Restricciones y decisiones previas:
  - La PPU por software es la referencia. El shim es lo que se mide.
  - El trabajo corre en el PC con Windows, con GPU. Si no hay adaptador de GPU, el ejemplo para.
  - El ROM solo en `roms/`. No sobrescribir ejemplos existentes; el resultado va a un archivo nuevo.
- Alternativas descartadas:
  - Medir X solo con la PPU, sin el shim → descartada: X es el color del shim, no de la PPU; hay que leerlo del shim.
  - Cambiar `ppu_capa_ganadora` para añadir el shim → descartada: mezclaría una medición verificada con otra en desarrollo.

### 3. Ejemplos

- Caso normal: un píxel donde el shim da X = `2A10`, la PPU da el color de BG1 = `2A10` en ese píxel, el sprite ganador tiene prioridad 1 y BG1 también tiene prioridad 1. El informe cuenta ese píxel en "X coincide BG1" y en "empate".
- Caso límite: si X no coincide con ningún fondo de texto, el píxel cuenta en "X sin fondo" y la hipótesis H30 se evalúa con ese conteo.

### 4. Pasos

1. Crear `examples/shim_capa_x.rs` (con `required-features = ["proto"]`), reutilizando por copia `fondo_texto`, `sprites` y la máscara de la aportación de `ppu_capa_ganadora.rs`.
2. Correr el juego hasta el cuadro 1000 con captura por línea.
3. Para los 5.290 píxeles, obtener X: el color del shim con estado de fin en V0 (lo mismo que `shim-fondo`).
4. Para cada píxel, calcular el color de cada fondo de texto (BG0, BG1, BG2, BG3) en la PPU, y el sprite ganador con su prioridad.
5. Contar: X coincide con BG0, BG1, BG2 o BG3 (a qué fondo); X sin fondo; y para el sprite ganador, cuántos píxeles tienen prioridad igual a la del fondo que muestra el shim.
6. Control: el color de la PPU V0 de cada píxel (que ya se sabe es el sprite) debe coincidir con la reconstrucción del sprite. Si no, parar.
7. Escribir `volcados/identificar-x-cuadro-1000.txt` (archivo nuevo).
8. Crear `tests/identificar_x_tests.rs` con las pruebas de la sección 8.
9. Ejecutar con `FE_ROM` definida y registrar el resultado.

### 5. Qué más podría verse afectado

- Archivos que cambian: `examples/shim_capa_x.rs` (nuevo), `tests/identificar_x_tests.rs` (nuevo), `Cargo.toml` (una entrada), `volcados/identificar-x-cuadro-1000.txt` (generado, no versionado).
- Archivos que no cambian: `ppu_capa_ganadora.rs`, `shim_aportacion.rs`, `shim_fondo_tests.rs`, `ppu.rs`, `gba-core`, `ppu_shim.rs`, `ppu_shim.wgsl`, el juego, la APK, y todos los informes anteriores.
- Riesgo: la reconstrucción de los fondos de texto puede diferir de la PPU; el control del paso 6 lo detecta.

### 6. Suposiciones

- La reconstrucción de fondos de texto y sprites copiada de `ppu_capa_ganadora` sigue siendo válida (control 0 en V0 y V1) → prototipar: sí, con el control del paso 6 → si es falsa: parar.
- X viene del color de un solo fondo de texto → prototipar: sí, midiendo las coincidencias con cada fondo → si es falsa: se reporta "X mezcla o sin fondo", no se descarta.

### 7. Alcance

- Incluye: identificar la capa de X (BG0, BG1, BG2, BG3 o ninguna); prioridad del sprite frente a esa capa; control; pruebas; resultado.
- No incluye: cambiar el shim; otros cuadros; ventanas, mezcla o el residuo del 1,33 %; Android.

### 8. Criterios de aceptación (no se modifican sin aprobación)

Escenario: la reconstrucción coincide con la PPU
Caso de uso: que la identificación de X sea confiable | Tipo: límite
  Dado el cuadro 1000
  Cuando se corre el ejemplo
  Entonces `control reconstruccion V0` y `control reconstruccion V1` valen 0
Test: `tests/identificar_x_tests.rs::test_reconstruccion_coincide` (se omite si no está definida `FE_ROM`)
Estado previo: falla (el informe no existe)

Escenario: X se asigna a una capa
Caso de uso: decidir H30 | Tipo: normal
  Dado el informe
  Cuando se suman los conteos de X por capa
  Entonces la suma es igual a `aportacion total`
Test: `tests/identificar_x_tests.rs::test_x_suma_total` (se omite si no está definida `FE_ROM`)
Estado previo: falla (el informe no existe)

Escenario: el informe da la prioridad del sprite frente a la capa de X
Caso de uso: decidir H31 | Tipo: normal
  Dado el informe
  Cuando se leen sus líneas
  Entonces existen `x coincide BG0`, `x coincide BG1`, `x coincide BG2`, `x coincide BG3`, `x sin fondo` y `prioridad empate`
Test: `tests/identificar_x_tests.rs::test_x_informe_completo` (se omite si no está definida `FE_ROM`)
Estado previo: falla (el informe no existe)

### 9. Cómo lo comprobaré

- Ejecución: `cargo run --release --features proto --example shim_capa_x -- "roms\1575 - Fire Emblem (E)(Independent).gba" secuencias\intro.txt 1000` → pasa si: existe `volcados/identificar-x-cuadro-1000.txt` con todas las líneas de la sección 8 → no detecta: otros cuadros.
- Pruebas: con `FE_ROM` definida, `cargo test --features proto --test identificar_x_tests` → pasa si: las tres pruebas pasan.
- Regresión: los informes `capa-ganadora-cuadro-1000.txt`, `shim-aportacion-cuadro-1000.txt` y `shim-fondo-cuadro-1000.txt` no cambian.

### 10. Terminado cuando

Se cumplen los tres criterios de aceptación y el Resultado dice de qué capa sale X, si H30 y H31 se confirman o se descartan, y qué prioridad tiene el sprite frente a esa capa.

### 11. Parar si

- El control de la reconstrucción no da 0 en V0 y V1: parar, porque la asignación de capas no sería confiable.
- No hay adaptador de GPU: parar y avisar, sin escribir informe.
- Dos intentos fallidos seguidos con el mismo criterio: detenerse, explicar qué se probó y marcar la propuesta como `discrepada`.

---

## Resultado (lo completa el agente después de editar)

Intento 1 (2026-10-17): el ejemplo compila (solo avisos) y corre, pero el control falla y se detiene por la regla de parada. No se escribió `volcados/identificar-x-cuadro-1000.txt`, así que no hay clasificación H30 ni H31.

Control: en V0, 3.583 de los 5.290 píxeles no coinciden con el color del sprite ganador (o no tienen sprite); en V1, 0 coinciden con BG2.

Causa probable: mi control de V0 comparaba el color de la PPU con el color bruto del sprite ganador. En la PPU, el color final puede venir de una mezcla (sprite semitransparente, aclarar u oscurecer por BLDCNT), que ese control no reconstruye. La reconstrucción completa de `ppu_capa_ganadora` sí incluye esa mezcla y tiene control 0. Por eso el control de este intento era demasiado simple: no es un hallazgo sobre el shim.

Próxima corrección propuesta: usar como control la reconstrucción completa de composición (la de `ppu_capa_ganadora`, ya con control 0) en lugar de comparar con el color bruto del sprite. Cambia el criterio de control, así que requiere aprobación. Este es el intento 1 de 2 permitidos.

Intento 2 (2026-10-17, con el control de composición completa aprobado): `examples/shim_capa_x.rs` corre y escribe `volcados/identificar-x-cuadro-1000.txt`. El job terminó con código 1 por avisos de cargo; el archivo de salida y las pruebas confirman el resultado.

- Control de reconstrucción: V0 = 0, V1 = 0.
- Aportación total: 5.290.
- X coincide con BG0 / BG1 / BG2 / BG3: 0 / 0 / 0 / 0.
- X sin fondo: 5.290. Suma: 5.290.
- Prioridad del sprite frente a la capa de X: empate 0; sprite mayor 0; sprite menor 0; sin sprite 0.
- Pruebas `identificar_x_tests`: 3 pasan, 0 fallan.

Lectura:
- H30 descartada: ninguno de los 5.290 píxeles tiene el color X de BG0, BG1, BG2 ni BG3. Se pedía al menos 4.761 coincidencias.
- H31 descartada: no hay ningún píxel con prioridad de sprite igual a la del fondo.
- El color X no sale de ninguno de los fondos de texto que reconstruye la PPU. La reconstrucción de fondos tiene control 0, así que la falta de coincidencia es un dato fiable.
- Queda sin explicar el origen de X en el shim. Lo más probable es paleta, banco u otro origen de color que no está medido. Esto necesita una propuesta nueva antes de tocar código.

---

## Revisión humana

- Que la capa de X y la prioridad del sprite se acepten como explicación del error del shim en estos píxeles, y que H30 y H31 se confirmen o descarten: ✔ | ✘ | ?
