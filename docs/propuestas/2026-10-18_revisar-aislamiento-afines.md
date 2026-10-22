# Revisar el metodo de aislamiento de afines (nivel 2)

Fecha: 2026-10-18
Estado: verificada por agente

## Objetivo
Comprobar si el metodo de aislamiento de shim_afin_pixeles (oam_con) altera el resultado. Si lo altera, la medicion de H210 (0 pixeles aportados) no sirve y hay que repetirla con otro metodo.

## Contexto
- H210 se midio con oam_con: los demas afines pasan a prohibidos (attr0 0xC000) para aislar cada uno.
- Con la escena intro.txt y cuadro 7500, la copia instrumentada (volcados/gba-recomp-wt) escribe 44/39/24/5/12 pixeles sin aislar.
- Eso contradice H210. Hay que saber si el aislamiento es la causa.

## Hipotesis
- H270: el aislamiento con oam_con cambia el resultado (por ejemplo, al prohibir los demas sprites se pierde la prioridad o el orden de pintado). Entonces H210 queda invalidada por el metodo.
- H271: el aislamiento no cambia el resultado. Entonces hay que buscar otra causa de los 0 pixeles.

## Metodo
1. Leer shim_afin_pixeles.rs (solo lectura) para confirmar como se aplica oam_con.
2. En volcados/gba-recomp-wt, crear un ejemplo nuevo que, en el cuadro 7500 de intro.txt, cuente pixeles escritos de cada afin con tres configuraciones: (a) sin tocar OAM; (b) demas afines ocultos de otra forma (sin cambiar sus datos de matriz ni prioridad); (c) oam_con tal como esta.
3. Comparar los contadores de escrito_en_linea entre (a), (b) y (c).

## Criterios
- Si (a) y (c) difieren, H270 se confirma.
- Si (a) y (b) coinciden y (c) difiere, H270 se confirma y el aislamiento correcto es (b).
- Si las tres coinciden, H271 se confirma.

## Lo que NO hace
- No modifica gba-recomp, no hace commits ni stash.
- No implementa nada en el shim.
- No dice si los pixeles son correctos (eso requiere comparacion visual o con referencia).

## Resultado
(pendiente)

## Resultado (2026-10-18)
- Escena intro.txt, cuadro 7500, copia volcados/gba-recomp-wt (working tree + contadores). Ejemplo nuevo: afin_aislar (solo en la copia).
- Afines en OAM: [0,1,2,3,4] (los cinco afines son exactamente los indices 0-4).
- Escrito por afin, sin tocar / desactivar otros / prohibir otros:
  afin 0: 44 / 44 / 44
  afin 1: 39 / 39 / 39
  afin 2: 24 / 24 / 24
  afin 3: 5 / 5 / 5
  afin 4: 12 / 12 / 12
- H270 DESCARTADA: el aislamiento no cambia el resultado (con oam_con ni con desactivacion).
- H271 CONFIRMADA: el aislamiento no explica los 0 pixeles de la medicion anterior.
- Consecuencia: la medicion anterior de shim_afin_pixeles (0 pixeles) sigue sin explicacion. Su metodo de comparacion con la linea base (diferencia de framebuffer) es el siguiente sospechoso, no el oam_con. Hay que revisarlo antes de dar H210 por buena o mala.
- Limite: esto prueba que el aislamiento no altera el conteo, no que los pixeles sean correctos.
- Estado: pendiente de revision humana.
