# Shim frente al emulador en el cuadro 5300 (nivel 2)

Fecha: 2026-10-19
Estado: verificada por agente

## Objetivo
Comprobar si el shim dibuja los pixeles de los afines del cuadro 5300 de intro.txt con los mismos colores y posiciones que el emulador de referencia.

## Contexto
- Cuadro 5300: los afines de OAM 0, 1 y 2 aportan 1841 pixeles que cambian la imagen (volcados/wt-5300).
- Control de la PPU: 0 diferencias con el emulador.
- Lo que falta es saber si el shim reproduce esos pixeles. En 7500 no se podia medir porque los afines no cambiaban nada.

## Hipotesis
- H340: el shim reproduce los 1841 pixeles afines con los colores del emulador (0 diferencias en las zonas de afin). Entonces no hace falta implementar la transformacion afin en el shim.
- H341: el shim no reproduce esos pixeles, o los reproduce con colores o posiciones distintos. Entonces la transformacion afin hace falta en el shim y el dato lo justifica.

## Metodo (solo en la copia gba-recomp-wt, sin tocar gba-recomp)
1. Ejecutar shim_afin_mide (ya existe en blazing-recomp/examples) sobre intro.txt en el cuadro 5300, con la feature proto.
2. Comparar el framebuffer del shim con el del emulador en las posiciones de los 1841 pixeles afines (lista de pix-completa.txt en volcados/wt-5300).
3. Contar cuantos coinciden, cuantos difieren y en que zonas.

## Criterios
- H340 si el shim coincide en al menos el 99 % de los pixeles afines (tolerancia de 1 por canal).
- H341 si la diferencia es mayor, con el numero y la zona donde falla.

## Lo que NO hace
- No modifica gba-recomp, no hace commits ni stash, no implementa nada en el shim.
- No dice si el resultado visual es correcto mas alla del emulador de referencia.

## Resultado
(pendiente)

## Resultado (2026-10-19)
- shim_afin_mide en cuadro 5300 (volcados/afin-5300.txt).
- Control reconstruccion frente a emulador: 0 diferencias.
- Diferencias shim frente a PPU V0: 33173 de 38400 pixeles en total.
- Lineas con afines (148): 30294 diferencias.
- Lineas sin afines (12): 2879 diferencias. Por ejemplo, las lineas 0-11 tienen 240 diferencias cada una sin ningun afin.
- CONCLUSION: el test no es concluyente sobre los afines. El shim difiere de la PPU de forma generalizada en este cuadro, tambien en lineas sin afines. Esa diferencia tiene otra causa (modo, ventanas, mezcla de color, fondos) y tapa el efecto afin.
- H340 y H341: no evaluables todavia.
- Siguiente paso: separar las diferencias del shim por causa (modo de video, BLDCNT/BLDALPHA, ventanas, fondos activos) en las lineas sin afines, antes de volver a medir los afines.
- Estado: pendiente de revision humana.
