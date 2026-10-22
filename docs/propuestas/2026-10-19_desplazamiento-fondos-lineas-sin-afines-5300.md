# Desplazamiento de fondos en las líneas sin afines (cuadro 5300)

Fecha: 2026-10-19
Nivel: 2
Estado: verificada por agente

## Motivo

En `2026-10-19_fondos-modo-lineas-sin-afines-5300.md` el modo y los fondos son iguales en las 160 líneas, y las 12 líneas sin afines tienen 239 o 240 píxeles distintos de 240 entre shim y PPU. Eso apunta a algo que cambia la línea completa. Las ventanas ya quedaron descartadas en 5300 (H351: WIN0H/V y WIN1H/V = 0).

## Pregunta

¿Cuáles son los desplazamientos de fondo (BG0HOFS a BG3VOFS, registros 0x10 a 0x1F) en cada una de las 12 líneas sin afines, y son iguales a los de las otras líneas?

## Método

1. Copia de solo lectura de `examples/shim_afin_fondos.rs` (sin cambiar `gba-core`, el shim ni `gba-recomp`).
2. Para cada línea capturada, volcar los 8 registros de desplazamiento (0x10 a 0x1F) a `volcados/desplaz-sinafin-5300.txt`.
3. Comprobar que el control de reconstrucción sigue dando 0 diferencias frente al emulador. Si no, parar.
4. Anotar qué líneas tienen desplazamientos distintos del resto y si coinciden con las 12 líneas con diferencias.

## Criterios

- Si las 12 líneas tienen desplazamientos distintos y el resto no: el shim no aplica bien el desplazamiento de fondo en esas líneas. Pista fuerte.
- Si los desplazamientos son iguales en todas las líneas: el desplazamiento no explica la diferencia. H352 queda en otras causas.
- Si el control no da 0, no se saca conclusión.

## Alcance excluido

- No se modifica el shim, `gba-core` ni `gba-recomp`.
- No se hacen commits.
- No se implementa la transformación afín.

## Resultado

- Ejecución: job job_09b52caca65f895c, ejemplo examples/shim_afin_desplaz.rs (solo lectura). Exit 0. Salida: volcados/desplaz-sinafin-5300.txt. Control: 0 diferencias (válido).
- BG0, BG2 y BG3 tienen desplazamiento 0 en todas las líneas. Solo BG1 VOFS cambia, línea a línea (0000, ffff, fffe, fffd, ... en las líneas 0 a 12, y así sigue en todo el cuadro, p. ej. línea 99 = ffce).
- El desplazamiento vertical de BG1 varía en todas las líneas, no solo en las 12 sin afines. Por eso no aísla esas 12 líneas.
- Conclusión: **no concluyente**. El desplazamiento no explica por sí solo qué líneas difieren. Hipótesis nueva (no probada): el shim podría usar un solo valor de desplazamiento por cuadro en vez de uno por línea. Requiere nueva propuesta.
- Diferencias totales del shim frente a la PPU: 33173 (igual que antes). Líneas sin afines: 2879 (igual).
- gba-recomp no se ha modificado.
