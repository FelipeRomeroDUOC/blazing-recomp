# Propuesta: Qué capa queda debajo del sprite ganador en los 5.290 píxeles del cuadro 1000

Fecha: 2026-10-17
Nivel: 2
Estado: verificada por agente

<!-- Estados: pendiente | aprobada | modificada | descartada |
     verificada por agente | revisada por humano | discrepada | cerrada -->

## Propuesta (la escribe el agente antes de editar)

### 1. Qué cambia

En `capa-ganadora-por-pixel-obj-cuadro-1000` quedó confirmada H15: en los 5.290 píxeles de aportación fuera de cobertura, el sprite gana la capa de arriba en V0 y su color es igual al del fondo sin capas (V3). Falta saber qué capa aparece debajo de ese sprite cuando se quita OBJ (V1). Esta propuesta mide esa capa, pixel a pixel, sobre los mismos 5.290 píxeles.

No se cambia `ppu.rs`, `gba-core`, el shim ni el juego. Se reutiliza la reconstrucción que ya tiene control 0 en las cuatro variantes.

### 2. Contexto

- Contexto consultado:
  - `docs/propuestas/2026-10-17_capa-ganadora-por-pixel-obj-cuadro-1000.md` (Resultado del intento 2 y Revisión humana): 5.290 píxeles, todos H15, H16 = 0, H17 = 0, control 0 en V0–V3.
  - `examples/ppu_capa_ganadora.rs`: ya guarda `rec_fl1` (capa de arriba en V1) por píxel, pero solo imprime la suma de categorías, no la capa de abajo.
- Intención: saber si, al quitar OBJ, el píxel pasa al fondo de pantalla (capa 5), a un fondo de texto (capas 0 a 3) o a otra cosa, y si ese color coincide con el fondo vacío de V3.
- Hipótesis:
  - H18: en la mayoría de los píxeles, V1 muestra el fondo de pantalla (capa 5). Se confirma si al menos el 50 % de los 5.290 píxeles tiene `fl1 = 5`.
  - H19: en la mayoría, V1 muestra un fondo de texto (capas 0 a 3). Se confirma si al menos el 50 % tiene `fl1` entre 0 y 3.
  - H20: el color de V1 es igual al color de V3 en todos los píxeles de la aportación. Se confirma si el conteo de color igual a V3 es el total de 5.290.
- Restricciones y decisiones previas:
  - La PPU por software es la referencia. Esta propuesta la lee; no la cambia.
  - El trabajo corre en el PC con Windows, con el ROM solo en `roms/`.
  - Cualquier cambio en `ppu.rs` requiere otra propuesta y aprobación.
- Alternativas descartadas:
  - Escribir un ejemplo nuevo que repita la reconstrucción → descartada: duplica código y puede divergir del ya verificado.
  - Cambiar el archivo `volcados/capa-ganadora-cuadro-1000.txt` aprobado → descartada: se conserva el informe actual; los conteos nuevos van a un archivo aparte.

### 3. Ejemplos

- Caso normal: un píxel con V0 = `3062`, V1 = `0C40` y V3 = `3062`. El informe dice que en V1 la capa de arriba es el fondo 2, que ese color no es el de V3 y que el sprite ganador es el de OAM número 17. Así se ve qué sustituye al sprite cuando se quita OBJ.
- Caso límite: si `fl1` no está entre 0 y 5 en algún píxel, el ejemplo para: la categoría no existe.

### 4. Pasos

1. Extender `examples/ppu_capa_ganadora.rs` para que, además del informe actual, escriba `volcados/capa-debajo-cuadro-1000.txt`. El informe actual no cambia.
2. En ese archivo, contar para los 5.290 píxeles de aportación:
   - capa de arriba en V1: fondo de pantalla (5), fondo 0, fondo 1, fondo 2, fondo 3;
   - color de V1 igual al de V3, y distinto de V3;
   - color de V1 igual al color del sprite ganador (debe ser 0).
3. Sumar las categorías de capa y comprobar que suman 5.290.
4. Añadir a `tests/capa_ganadora_tests.rs` las pruebas de la sección 8.
5. Ejecutar con `FE_ROM` definida y registrar el resultado.

### 5. Qué más podría verse afectado

- Archivos que cambian: `examples/ppu_capa_ganadora.rs` (se añade la salida nueva), `tests/capa_ganadora_tests.rs` (tres pruebas nuevas), `volcados/capa-debajo-cuadro-1000.txt` (generado, no versionado).
- Archivos que no cambian: `gba-core`, `ppu.rs`, `ppu_shim.rs`, `ppu_shim.wgsl`, el juego, la APK y `volcados/capa-ganadora-cuadro-1000.txt`.
- Riesgo: que la salida nueva cambie el comportamiento del informe actual. El criterio de la sección 8 lo comprueba.

### 6. Suposiciones

- La reconstrucción ya verificada (control 0) sigue siendo la base → prototipar: sí, se vuelve a medir el control al correr → si es falsa: parar.
- `rec_fl1` guarda la capa de arriba en V1 correctamente → prototipar: sí, con la suma de categorías igual a 5.290 → si es falsa: parar.

### 7. Alcance

- Incluye: conteos de la capa de abajo (capa de arriba en V1 y color frente a V3) para los 5.290 píxeles; pruebas y resultado.
- No incluye: otros cuadros; cambiar la reconstrucción; cambiar la PPU, el shim o el juego; Android.

### 8. Criterios de aceptación (no se modifican sin aprobación)

Escenario: el informe actual sigue igual
Caso de uso: que la medición nueva no altere lo ya verificado | Tipo: límite
  Dado el cuadro 1000
  Cuando se corre el ejemplo con la salida nueva
  Entonces `control reconstruccion V0` a `V3` siguen en 0 y `capa categoria H15` sigue en 5290
Test: `tests/capa_ganadora_tests.rs::test_reconstruccion_coincide` y `test_informe_da_categorias` (se omiten si no está definida `FE_ROM`)
Estado previo: pasa (verificado el 2026-10-17)

Escenario: la capa de arriba en V1 se reparte en las categorías
Caso de uso: decidir entre H18 y H19 | Tipo: normal
  Dado el archivo `volcados/capa-debajo-cuadro-1000.txt`
  Cuando se suman las categorías de capa en V1
  Entonces la suma es 5290
Test: `tests/capa_ganadora_tests.rs::test_capa_debajo_suma_total` (se omite si no está definida `FE_ROM`)
Estado previo: falla (el archivo no existe)

Escenario: el color de V1 se compara con V3 y con el sprite
Caso de uso: decidir H20 y descartar que V1 use el color del sprite | Tipo: normal
  Dado el archivo `volcados/capa-debajo-cuadro-1000.txt`
  Cuando se leen las líneas de color
  Entonces existen `capa debajo color igual V3` y `capa debajo color igual sprite`, y la segunda vale 0
Test: `tests/capa_ganadora_tests.rs::test_capa_debajo_color` (se omite si no está definida `FE_ROM`)
Estado previo: falla (el archivo no existe)

### 9. Cómo lo comprobaré

- Ejecución: `cargo run --release --features proto --example ppu_capa_ganadora -- "roms\1575 - Fire Emblem (E)(Independent).gba" secuencias\intro.txt 1000` → pasa si: existen `volcados/capa-ganadora-cuadro-1000.txt` y `volcados/capa-debajo-cuadro-1000.txt` con todas las líneas de la sección 8 → no detecta: otros cuadros.
- Pruebas: con `FE_ROM` definida, `cargo test --features proto --test capa_ganadora_tests` → pasa si: las cinco pruebas pasan.
- Regresión: el informe `capa-ganadora-cuadro-1000.txt` tiene las mismas líneas que antes → pasa si: sus conteos no cambian.

### 10. Terminado cuando

Se cumplen los tres criterios de aceptación y el Resultado dice qué capa aparece debajo del sprite en los 5.290 píxeles, y si H18, H19 y H20 se confirman o se descartan.

### 11. Parar si

- El control de la reconstrucción deja de dar 0: parar, porque la capa de abajo no sería confiable.
- La salida nueva cambia el informe actual: parar y corregir antes de seguir.
- Dos intentos fallidos seguidos con el mismo criterio: detenerse, explicar qué se probó y marcar la propuesta como `discrepada`.

---

## Resultado (lo completa el agente después de editar)

Ejecutado 2026-10-17 (cuadro 1000, mismos 5.290 píxeles de aportación fuera de cobertura):

- Capa debajo del sprite en V1: fondo 2 = 5.290; fondo 0, 1, 3 y fondo de pantalla = 0. Suma = 5.290.
- Color de V1 igual al de V3 (fondo vacío): 0.
- Color de V1 igual al del sprite ganador: 0.
- Control de la reconstrucción: sigue en 0 en V0 a V3. El informe `capa-ganadora-cuadro-1000.txt` no cambió.
- Pruebas: las cinco de `capa_ganadora_tests` pasan.

Hipótesis:
- H18 (fondo de pantalla): descartada, 0 píxeles.
- H19 (fondo de texto): confirmada, y en concreto es el fondo 2 en los 5.290 píxeles.
- H20 (color de V1 igual a V3): descartada, 0 píxeles. V1 muestra un color propio del fondo 2 que no es el fondo vacío.

Lectura: al quitar OBJ, esos píxeles pasan a mostrar el fondo 2. El informe de contenido de BG (`ppu_contenido_bg`) midió BG0, BG1 y BG3, no BG2. Por eso no se sabe aún si BG2 tiene contenido visible en el cuadro 1000; hay que medirlo antes de sacar conclusiones sobre ese fondo. Este resultado no dice por qué el fondo 2 aparece solo en esos píxeles.

---

## Revisión humana

- Que la capa de abajo medida en los 5.290 píxeles se acepte como explicación de H15 y que H18, H19 y H20 queden confirmadas o descartadas con sus conteos: ✔ | ✘ | ?
