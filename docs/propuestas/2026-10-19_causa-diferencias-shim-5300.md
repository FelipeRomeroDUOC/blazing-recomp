# Causa de las diferencias shim frente a PPU en el cuadro 5300 (nivel 2)

Fecha: 2026-10-19
Estado: verificada por agente

## Objetivo
Separar las 2879 diferencias del shim en lineas sin afines del cuadro 5300 por causa, para poder medir los afines despues sin ese ruido.

## Contexto
- shim_afin_mide en 5300: 33173 diferencias shim frente a PPU; 2879 en 12 lineas sin afines (volcados/afin-5300.txt).
- En 7500 el shim coincidia con la PPU. Algo especifico de 5300 difiere.

## Hipotesis
- H350: la diferencia viene de la mezcla de color (BLDCNT/BLDALPHA o efectos de brillo). Entonces el shim no aplica efectos que la PPU si aplica.
- H351: viene de ventanas (WIN0/WIN1/WINOUT). Entonces el shim no respeta las ventanas.
- H352: viene de los fondos (modo de video o prioridades). Entonces el shim pinta un fondo distinto.

## Metodo (solo lectura de registros y medicion)
1. Leer en el cuadro 5300 los registros DISPCNT, BLDCNT, BLDALPHA, BLDY, WININ, WINOUT y WIN0/WIN1, y anotar el modo de video.
2. Para las 12 lineas sin afines, medir cuantas diferencias hay por cada causa sospechosa, activando una a una (por ejemplo, forzar sin mezcla en la PPU y ver si la diferencia baja).
3. Anotar que causa explica la mayoria de las 2879 diferencias.

## Criterios
- Si al desactivar la mezcla la diferencia baja de forma clara, H350 queda confirmada.
- Igual para ventanas (H351) y fondos (H352).
- Si ninguna causa explica las diferencias, se abre una hipotesis nueva con las lineas concretas.

## Lo que NO hace
- No modifica gba-recomp, no hace commits ni stash, no implementa nada en el shim.
- No decide si el shim debe corregirse; solo identifica la causa.

## Resultado
(pendiente)

## Resultado (2026-10-19)
- Registros en cuadro 5300 (volcados/wt-registros/registros-5300.txt):
  DISPCNT 1F00 (modo 0, BG0-3 y OBJ activos, mapeo 2D)
  BLDCNT 2261 (efecto 1 = alfa; primer objetivo BG0, OBJ, BD; segundo objetivo BG1, BD)
  BLDALPHA 0410 (EVA 16/16, EVB 4/16)
  WIN0H, WIN1H, WIN0V, WIN1V = 0000; DISPCNT sin bits de ventana (13 y 14 a 0): las ventanas no estan activas.
  WININ 2020 y WINOUT 2020: sin efecto porque las ventanas estan desactivadas.
- H350 APOYADA: hay mezcla alfa activa en este cuadro, y las lineas sin afines tienen 2879 diferencias. Es la explicacion mas probable.
- H351 DESCARTADA: las ventanas no estan activas.
- H352 SIN RESOLVER: modo 0 con cuatro fondos; no se puede descartar sin medirlo.
- Siguiente paso: desactivar BLDCNT (poner 0) en una copia del estado y volver a comparar shim y PPU en las 12 lineas sin afines. Si la diferencia baja, H350 queda confirmada.
- Estado: pendiente de revision humana.
