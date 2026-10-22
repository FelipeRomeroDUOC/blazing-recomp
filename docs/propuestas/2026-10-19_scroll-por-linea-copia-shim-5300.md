# Prueba del shim con desplazamiento por línea (copia, cuadro 5300)

Fecha: 2026-10-19
Nivel: 2
Estado: modificada

## Motivo

El paso A (`2026-10-19_scroll-por-cuadro-shim-5300.md`) confirmó que el shim recibe un solo `BG1 VOFS` por cuadro (0x000), mientras el juego lo cambia en cada línea (0x000 a 0x1FF). Falta medir si aplicar el desplazamiento por línea baja las diferencias shim–PPU.

## Pregunta

¿Bajan las diferencias shim–PPU en las 12 líneas sin afines (y en el total) si el shim aplica el desplazamiento de cada línea?

## Método

1. Copiar `src/ppu_shim.rs` y `src/ppu_shim.wgsl` a archivos de prueba (por ejemplo `examples/ppu_shim_linea.rs` y `examples/ppu_shim_linea.wgsl`). No se toca el shim real.
2. En la copia, en vez de un solo `scroll` por capa, usar un arreglo de 160 valores de `scroll_y` (y de `scroll_x`) tomados de `cap.io` de cada línea, pasándolo al shader como buffer o uniform de tamaño 160.
3. En el shader de la copia, usar `scroll_y[l]` para la línea `l` en vez del valor único.
4. Ejecutar el cuadro 5300 con la copia y medir: diferencias shim–PPU totales y en las 12 líneas sin afines. Guardar en `volcados/scroll-linea-5300.txt`.
5. Control: la reconstrucción frente al emulador debe dar 0.

## Criterios

- Si las diferencias en las 12 líneas bajan más del 90 %: el scroll por línea es la causa principal de esas diferencias. Pasar a una propuesta de cambio del shim real (decisión aparte).
- Si bajan entre 10 % y 90 %: causa parcial; seguir buscando.
- Si bajan menos del 10 %: el scroll no es la causa principal.
- Si el control no da 0: no se saca conclusión.

## Alcance excluido

- No se modifica `src/ppu_shim.rs`, `src/ppu_shim.wgsl`, `gba-core` ni `gba-recomp`.
- No se hacen commits.
- No se implementa la transformación afín.

## Resultado

- Desviación del método aprobado: no se copió ni se cambió el shader. Se renderizó el cuadro con el VOFS de BG1 de cada línea y se tomó solo esa fila. Es un método sin cambio de código del shim, pero distinto del planeado. Ejemplo: examples/shim_afin_linea.rs. Job: linea-log.txt y linea-exit.txt.
- Control: 0 diferencias (válido).
- Resultado: diferencias totales 33173 y líneas sin afines 2879, iguales que antes. El cambio de desplazamiento por línea no cambia nada.
- Prueba de sensibilidad (volcados/sensibilidad-5300.txt): cambiar BG1 VOFS de 0x000 a 0x1FF en el shim cambia 0 de 115200 bytes. El shim ignora ese registro en este cuadro.
- Conclusión: **H scroll por cuadro descartada como causa**. El desplazamiento de BG1 no afecta la salida del shim en 5300, así que aplicarlo por línea no puede reducir las diferencias. El paso A sigue siendo cierto (el shim recibe un solo valor), pero ese valor no tiene efecto visible aquí.
- Pendiente: entender por qué el shim no usa BG1 VOFS (posibles causas: BG0 tapa BG1 en todas las líneas, o el shim no lee ese registro en este camino). No investigado.
- gba-recomp y src/ppu_shim.* no se han modificado.
