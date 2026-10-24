# ¿Por qué el shim no reacciona al desplazamiento de BG1? (cuadro 5300)

Fecha: 2026-10-19
Nivel: 2
Estado: verificada por agente

## Motivo

En `2026-10-19_scroll-por-linea-copia-shim-5300.md` cambiar BG1 VOFS de 0x000 a 0x1FF cambia 0 de 115 200 bytes de la salida del shim. El juego sí escribe valores distintos por línea. Hay dos explicaciones posibles, que no son excluyentes:

- A. BG0 (prioridad 0, siempre delante de BG1 con prioridad 1) cubre toda la pantalla, así que BG1 nunca se ve.
- B. El shim ignora el desplazamiento de BG1 en este camino de render.

## Preguntas

1. ¿BG0 es opaco (no transparente) en todos los píxeles de las 160 líneas del cuadro 5300, según la PPU? ¿Qué líneas y qué píxeles deja ver BG1?
2. ¿El cambio de desplazamiento de BG0, BG2 o BG3 sí cambia la salida del shim? Esto separa A de B.
3. Si BG1 se ve en algún píxel en la PPU, ¿esos píxeles cambian en el shim al cambiar BG1?

## Método

1. Copia de solo lectura de `examples/shim_afin_linea.rs` (sin cambiar `gba-core`, el shim real ni `gba-recomp`).
2. Pregunta 1: contar píxeles de BG0 transparentes en la PPU (`gba-core`), línea a línea. Guardar en `volcados/bg0-opacidad-5300.txt`.
3. Pregunta 2: prueba de sensibilidad por fondo: cambiar el VOFS de BG0, BG2 y BG3 (uno a la vez) y contar bytes distintos en la salida del shim.
4. Pregunta 3: cruzar la salida de la PPU y del shim en los píxeles donde BG1 se ve.
5. Control: la reconstrucción frente al emulador debe dar 0.

## Criterios

- Si BG0 es opaco en todo el cuadro y BG1 no se ve: explicación A. El desplazamiento de BG1 no importa en 5300 y el problema está en otra parte.
- Si BG0 deja ver píxeles de BG1 y el shim no cambia al tocar BG1: explicación B. Investigar el camino de render del shim para BG1.
- Si cambiar BG0, BG2 o BG3 no cambia el shim: el shim no usa ningún desplazamiento en este cuadro, y la causa está en otro lado.
- Si el control no da 0: no se saca conclusión.

## Alcance excluido

- No se modifica el shim real, `gba-core` ni `gba-recomp`.
- No se hacen commits.
- No se implementa la transformación afín.

## Resultado

- Ejecución: examples/shim_afin_bg.rs (solo lectura), job job_079db0d7e36f494b, exit 0. Salida: volcados/bg-sensibilidad-5300.txt. Control: 0 diferencias (válido).
- Sensibilidad por fondo (VOFS 0x000 a 0x1FF, bytes distintos en la salida del shim de 115200):
  - BG0: 37241
  - BG1: 0
  - BG2: 0
  - BG3: 0
- Pregunta 2 respondida: el shim reacciona al desplazamiento de BG0 pero no al de BG1, BG2 ni BG3.
- Lectura: consistente con la explicación A (BG0 tapa a los demás fondos donde se ve). No queda probado: que BG0 cambie la imagen no prueba que sea opaco en todas partes.
- No medidas: pregunta 1 (opacidad de BG0 en la PPU, línea a línea) y pregunta 3 (píxeles de BG1 visibles y su comparación). Quedan pendientes.
- gba-recomp y src/ppu_shim.* no se han modificado.
