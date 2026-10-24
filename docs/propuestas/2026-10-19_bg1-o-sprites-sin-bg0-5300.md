# ¿Qué capa aparece donde BG0 no se ve? (cuadro 5300)

Fecha: 2026-10-19
Nivel: 2
Estado: verificada por agente

## Motivo

En `2026-10-19_opacidad-bg0-ppu-5300.md` hay 1843 píxeles donde BG0 no se ve. Solo 2 tienen el color de fondo de paleta. Los otros 1841 muestran otra capa: pueden ser BG1 (que tapa BG0 y no se ve en ningún otro lado) o sprites (sobre todo afines, ya que 1841 coincide con los píxeles afines distintos del shim, H330).

## Pregunta

En los píxeles donde BG0 no se ve, ¿la capa visible es BG1, OBJ (sprites), o ambos? ¿Cuántos de cada uno?

## Método

1. Copia de solo lectura de `examples/shim_afin_bg0.rs` (sin cambiar `gba-core`, el shim real ni `gba-recomp`).
2. Por línea, renderizar con la PPU cuatro variantes sobre la configuración completa:
   - completa;
   - sin BG0;
   - sin BG0 y sin OBJ (apagar bit 12 de DISPCNT);
   - sin BG0 y sin BG1 (apagar bit 9).
3. Para los píxeles sin BG0 (de la medición anterior):
   - si al apagar OBJ cambian, el sprite es visible;
   - si al apagar BG1 cambian, BG1 es visible;
   - si no cambian con ninguno, la capa es otra o es el fondo.
4. Contar cada caso. Salida: `volcados/capa-sin-bg0-5300.txt`.
5. Control: la reconstrucción completa frente al emulador debe dar 0.

## Criterios

- Si la mayoría de los 1841 píxeles depende de OBJ: los sprites tapan BG0, no BG1. Esto apoya que el problema está en sprites y no en el desplazamiento de BG1.
- Si la mayoría depende de BG1: BG1 aparece donde BG0 no está, y hay que revisar el desplazamiento de BG1 en el shim.
- Si el control no da 0: no se saca conclusión.

## Alcance excluido

- No se modifica el shim real, `gba-core` ni `gba-recomp`.
- No se hacen commits.
- No se implementa la transformación afín.

## Resultado

- Ejecución: examples/shim_afin_capa.rs (solo lectura, copia de la prueba de opacidad), job job_4466d84b3323296e, exit 0. Salida: volcados/capa-sin-bg0-5300.txt. Control: 0 diferencias (válido).
- Píxeles sin BG0 (1843):
  - solo OBJ (sprites): 1841
  - solo BG1: 0
  - OBJ y BG1: 0
  - ninguno (otra capa o fondo): 2
- Conclusión: **los píxeles donde BG0 no se ve son sprites, no BG1**. BG1 no aparece en ningún píxel del cuadro 5300. Esto confirma que el desplazamiento de BG1 no afecta la imagen aquí (ver scroll-por-linea-copia-shim-5300.md). Los 1841 píxeles coinciden con los afines distintos del shim (H330).
- Pendiente de otro tema: por qué el shim difiere de la PPU en las 12 líneas sin afines (2879 píxeles); no lo explica nada de lo medido aquí.
- gba-recomp y src/ppu_shim.* no se han modificado.
