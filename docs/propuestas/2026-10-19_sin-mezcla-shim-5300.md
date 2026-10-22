# Prueba sin mezcla alfa en el cuadro 5300 (nivel 2)

Fecha: 2026-10-19
Estado: descartada

## Objetivo
Comprobar si la diferencia del shim frente a la PPU en las 12 lineas sin afines del cuadro 5300 se debe a la mezcla alfa (BLDCNT = 2261, BLDALPHA = 0410).

## Contexto
- Cuadro 5300: modo 0, BLDCNT 2261 (efecto alfa activo), BLDALPHA 0410, ventanas inactivas.
- Diferencias shim frente a PPU: 2879 pixeles en 12 lineas sin afines (volcados/afin-5300.txt).
- H350: la mezcla alfa explica esas diferencias.

## Metodo (en copia, sin tocar gba-recomp ni el shim)
1. Copia del estado del cuadro 5300 en la que BLDCNT pasa a 0000 (solo esa escritura en io[0x50..0x52]).
2. Medir de nuevo el shim frente a la PPU en esas 12 lineas, con el mismo metodo de shim_afin_mide.
3. Medir la PPU frente al emulador con BLDCNT = 0000, para confirmar que el control sigue siendo 0.
4. Comparar: diferencias sin mezcla frente a con mezcla.

## Hipotesis
- H360: sin mezcla, las diferencias en las 12 lineas bajan de forma clara (por ejemplo, casi a 0). Entonces H350 queda confirmada.
- H361: las diferencias se mantienen. Entonces la mezcla no es la causa y hay que revisar H352 (fondos o modo).

## Criterios
- H360 si la reduccion es mayor del 90 %.
- H361 si la reduccion es menor del 10 %.
- Un resultado intermedio deja las dos abiertas y pide mas datos.

## Lo que NO hace
- No modifica gba-recomp, no hace commits ni stash, no implementa nada en el shim.
- No cambia el juego ni el estado real: solo la copia usada en la medicion.

## Resultado
(pendiente)

## Resultado

- Ejecución: EXIT 0 (job job_84fec928ec56135f).
- Salida: volcados/afin-sinmezcla-5300.txt, byte a byte idéntica a volcados/afin-5300.txt (cmp sin diferencias).
- Líneas sin afines: 12; diferencias en ellas: 2879 (antes 2879). Reducción: 0 %.
- Diferencias totales: 33173 (igual que antes).
- Criterio: H360 si reducción > 90 %; H361 si < 10 %.
- Conclusión: **H361 confirmada**. Quitar la mezcla alfa (BLDCNT = 0) no cambia las diferencias en las líneas sin afines. La hipótesis H350 (mezcla alfa como causa) queda descartada en esta forma.
- Advertencia: el resultado idéntico byte a byte sugiere que la escritura de io[0x50..0x51] puede no llegar a la ruta de render del shim. Antes de dar H352 por cerrada, conviene comprobar que el valor de BLDCNT leído por el shim es 0 en el cuadro 5300.
- gba-recomp no se ha modificado en esta prueba.

## Resultado

- Ver 2026-10-19_comprobar-bldcnt-shim-5300.md: la prueba sin mezcla no fue válida; H361 anulada.
