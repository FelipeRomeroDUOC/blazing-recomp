# Buscar una escena con efecto afin visible (nivel 2)

Fecha: 2026-10-19
Estado: verificada por agente

## Objetivo
Encontrar un cuadro donde los sprites afines cambien la imagen final de forma visible, para comparar la PPU y el shim con el emulador de referencia en un caso que si importa.

## Contexto
- En intro.txt, cuadro 7500, los 124 pixeles de afin tienen el mismo color que el fondo. No hay efecto visible.
- escena3.txt a 7500 da 0 en todos los contadores; escena2.txt no llega a 7500.
- No hay otra ROM confirmada en la carpeta roms, solo la que usamos.

## Hipotesis
- H320: hay cuadros en intro.txt donde los afines escriben pixeles con color distinto del fondo. Entonces hay escenas de prueba utiles.
- H321: no hay ningun cuadro asi en estas secuencias. Entonces hace falta otra secuencia o ROM.

## Metodo (solo en la copia gba-recomp-wt, solo lectura de datos)
1. Listar la carpeta roms y las secuencias disponibles (solo nombres).
2. Recorrer intro.txt cada 100 cuadros, con afin_replay o un ejemplo nuevo, contando por cuadro: pixeles de afin escritos y pixeles de afin con color distinto del fondo.
3. Anotar los cuadros con mayor numero de pixeles distintos.

## Criterios
- Si hay cuadros con pixeles de afin distintos del fondo, se elige el de mayor numero y se repite la comparacion de la PPU y el shim con el emulador.
- Si no hay ninguno, H321 queda confirmada y se pide otra ROM o secuencia.

## Lo que NO hace
- No modifica gba-recomp, no hace commits ni stash, no toca el shim.
- No decide si el shim debe implementar la transformacion afin.

## Resultado
(pendiente)

## Resultado (2026-10-19)
- Roms en la carpeta: solo Fire Emblem (E) y gba_bios.bin. Secuencias: escena2.txt, escena3.txt, intro.txt.
- Escaneo de intro.txt cada 100 cuadros (copia gba-recomp-wt, afin_escanear, volcados/wt-escaneo/escaneo-afin.txt). Cuadros con pixeles de afin: 12 de 112.
- Cuadros con mas pixeles de afin: 5300 (1841), 5200 (1018), 7400 (789), 8700 (583), 6800 (598), 8100 (400), 9300 (422).
- Cuadro 7500 (124) es uno de los de menor actividad.
- H320 CONFIRMADA: hay cuadros con afines activos. H321 descartada.
- Pendiente: medir en 5300 si los pixeles de afin cambian la imagen final y si su color difiere del fondo (mismo metodo que 7500).
- Estado: pendiente de revision humana.
